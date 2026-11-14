import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { readFile, mkdir, writeFile } from "node:fs/promises";
import { arch, availableParallelism, cpus, platform, release, totalmem } from "node:os";
import { dirname, join, resolve } from "node:path";
import { setTimeout } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { processMetrics } from "./performance/metrics.mjs";
import { measure } from "./performance/measure.mjs";
import { DisposableServer } from "./performance/server.mjs";
import {
  campaign,
  command,
  identifier,
  participant,
  save,
  verify,
  workload,
} from "./performance/workloads.mjs";
import { cargoEnvironment } from "./cargo-environment.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const execute = promisify(execFile);
const options = { operations: 1000, sessions: 1000, repeats: 3, concurrency: [1, 2, 8], queue: 8 };
for (let index = 2; index < process.argv.length; index += 2) {
  const key = process.argv[index]?.replace(/^--/, "");
  const value = process.argv[index + 1];
  if (!Object.hasOwn(options, key) || value === undefined)
    throw new Error(
      "Usage: npm run bench -- [--operations 1..1000] [--sessions 0..10000] [--repeats 1..10] [--concurrency 1,2,8] [--queue 0..32]",
    );
  options[key] = key === "concurrency" ? value.split(",").map(Number) : Number(value);
}
for (const [key, maximum] of [
  ["operations", 1000],
  ["sessions", 10000],
  ["repeats", 10],
  ["queue", 32],
]) {
  const minimum = key === "sessions" || key === "queue" ? 0 : 1;
  if (!Number.isInteger(options[key]) || options[key] < minimum || options[key] > maximum)
    throw new Error(`Invalid benchmark option: ${key}`);
}
if (
  options.concurrency.length === 0 ||
  options.concurrency.length > 4 ||
  options.concurrency.some((value) => !Number.isInteger(value) || value < 1 || value > 32) ||
  new Set(options.concurrency).size !== options.concurrency.length
)
  throw new Error("Concurrency must contain 1 to 4 distinct integers between 1 and 32.");
const binary = join(
  resolve(root, process.env.CARGO_TARGET_DIR ?? "target"),
  "release",
  process.platform === "win32" ? "rustsurvey-server.exe" : "rustsurvey-server",
);
const environment = cargoEnvironment(process.env);
console.log("Building the optimized RustSurvey server with the locked dependencies.");
await execute("cargo", ["build", "-p", "rustsurvey-server", "--release", "--locked"], {
  cwd: root,
  env: environment,
  timeout: 600000,
});
const [{ stdout: rustc }, { stdout: head }, { stdout: status }, binaryBytes] = await Promise.all([
  execute("rustc", ["--version"], { env: environment, timeout: 5000 }),
  execute("git", ["rev-parse", "HEAD"], { cwd: root, timeout: 5000 }),
  execute("git", ["status", "--porcelain"], { cwd: root, timeout: 5000 }),
  readFile(binary),
]);
const report = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  subject: "RustSurvey local HTTP release baseline; no LimeSurvey comparison",
  source: {
    head: head.trim(),
    dirty: status.trim() !== "",
    binarySha256: createHash("sha256").update(binaryBytes).digest("hex"),
    rustc: rustc.trim(),
  },
  host: {
    platform: platform(),
    release: release(),
    arch: arch(),
    cpu: cpus()[0]?.model ?? "unknown",
    logicalProcessors: cpus().length,
    availableParallelism: availableParallelism(),
    totalMemoryBytes: totalmem(),
    node: process.version,
  },
  configuration: {
    ...options,
    database: "disposable file SQLite, driver defaults",
    objectStore: "file",
    memorySamplingIntervalMs: 100,
    warmupSavesPerWorker: 10,
    retries: 0,
    sameHostClient: true,
  },
  scenarios: [],
};
let active = null;
let interrupted = false;
function interrupt() {
  interrupted = true;
  active?.interrupt();
}
process.on("SIGINT", interrupt);
process.on("SIGTERM", interrupt);
let operationSerial = 1_000_000;

async function scenario(name) {
  if (interrupted) throw new Error("Benchmark interrupted.");
  const fixture = await workload(name);
  const server = new DisposableServer(binary, options.queue);
  active = server;
  let metrics;
  try {
    await server.start();
    metrics = await processMetrics(server.child);
    await server.login();
    const run = await campaign(server, fixture);
    const result = {
      name,
      profile: run.profile,
      questions: fixture.definition.groups.reduce((sum, group) => sum + group.questions.length, 0),
      definitionSha256: fixture.definitionSha256,
      sessionGrowth: [],
      writes: [],
    };
    report.scenarios.push(result);
    result.sessionGrowth.push({ additionalOpenParticipants: 0, metrics: await metrics.sample() });
    let opened = 0;
    const milestones = [...new Set([Math.min(100, options.sessions), options.sessions])].filter(
      (count) => count > 0,
    );
    for (const milestone of milestones) {
      const start = opened;
      const phase = await measure(
        metrics,
        milestone - start,
        Math.min(2, milestone - start),
        async (_worker, index) => {
          await participant(server, run, 100_000 + start + index);
          return { status: 201, code: "created" };
        },
      );
      opened = milestone;
      await setTimeout(250);
      result.sessionGrowth.push({
        additionalOpenParticipants: opened,
        phase,
        metrics: await metrics.sample(),
      });
      console.log(
        `${name}: ${opened} open participants, RSS ${(result.sessionGrowth.at(-1).metrics.residentBytes / 1048576).toFixed(2)} MiB`,
      );
    }
    const checkpoints = [];
    let participantSerial = 1;
    for (let repetition = 1; repetition <= options.repeats; repetition += 1) {
      for (const concurrency of options.concurrency) {
        const workers = [];
        for (let index = 0; index < concurrency; index += 1) {
          const started = await participant(server, run, participantSerial);
          participantSerial += 1;
          let answers = { ...fixture.answers };
          if (name === "defaults") answers = { ...answers, ...started.answers };
          const worker = { id: started.id, version: 0, answers };
          for (let warmup = 0; warmup < 10; warmup += 1) {
            const saved = await save(server, worker, command(worker, fixture, operationSerial++));
            assert.equal(saved.status, 200);
          }
          const canonical = await verify(server, worker, fixture);
          worker.answers = canonical.answers;
          workers.push(worker);
        }
        const phase = await measure(metrics, options.operations, concurrency, async (index) =>
          save(server, workers[index], command(workers[index], fixture, operationSerial++)),
        );
        for (const worker of workers) await verify(server, worker, fixture);
        result.writes.push({ repetition, concurrency, ...phase });
        checkpoints.push(workers[0]);
        console.log(
          `${name}: repetition ${repetition}, concurrency ${concurrency}, ${phase.successful}/${phase.attempts} saved, ${phase.successesPerSecond.toFixed(1)} saves/s, p95 ${phase.successfulLatencyMs.p95?.toFixed(2)} ms, CPU ${phase.cpuMsPerSuccess?.toFixed(3)} ms/save`,
        );
      }
    }
    await metrics.close();
    metrics = null;
    await server.stop();
    await server.start();
    for (const worker of checkpoints) await verify(server, worker, fixture);
    if (opened > 0) {
      const last = await server.json(
        `/api/responses/${identifier(100_000 + opened - 1)}`,
        undefined,
        { participant: true },
      );
      assert.equal(last.version, 0);
    }
    result.verifiedAfterProcessRestart = true;
  } finally {
    await metrics?.close();
    await server.dispose();
    active = null;
  }
}

for (const name of ["small", "forms", "defaults"]) await scenario(name);
report.completedAt = new Date().toISOString();
const outputDirectory = join(root, "target/performance");
await mkdir(outputDirectory, { recursive: true });
const output = join(outputDirectory, `${report.generatedAt.replace(/[:.]/g, "-")}-rustsurvey.json`);
await writeFile(output, `${JSON.stringify(report, null, 2)}\n`, { flag: "wx" });
console.log(`Measured report: ${output}`);
