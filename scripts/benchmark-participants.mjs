import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { arch, cpus, platform, release, totalmem } from "node:os";
import { dirname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { setTimeout } from "node:timers/promises";
import { processMetrics } from "./performance/metrics.mjs";
import { observe, phaseOffsets, waitUntil } from "./performance/paced-measure.mjs";
import { DisposableServer } from "./performance/server.mjs";
import { VirtualUser } from "./performance/virtual-user.mjs";
import { campaign, identifier, workload } from "./performance/workloads.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const options = {
  package: "",
  clients: [100, 500, 1000],
  "arrival-seconds": 30,
  "think-seconds": 30,
  cycles: 2,
  "idle-seconds": 5,
  burst: 1000,
};
for (let index = 2; index < process.argv.length; index += 2) {
  const name = process.argv[index].replace(/^--/, "");
  const value = process.argv[index + 1];
  if (!Object.hasOwn(options, name) || value === undefined)
    throw new Error(
      "Use npm run bench:participants -- --package PATH [--clients 100,500,1000] [--arrival-seconds 30] [--think-seconds 30] [--cycles 2] [--idle-seconds 5] [--burst 1000]",
    );
  if (name === "package") options[name] = value;
  else if (name === "clients") options[name] = value.split(",").map(Number);
  else options[name] = Number(value);
}
if (
  options.package === "" ||
  options.clients.length < 1 ||
  options.clients.length > 4 ||
  options.clients.some((count) => !Number.isInteger(count) || count < 1 || count > 5000) ||
  new Set(options.clients).size !== options.clients.length
)
  throw new Error("Select a pilot package and one to four client counts in 1..5000.");
for (const [name, min, max] of [
  ["arrival-seconds", 1, 60],
  ["think-seconds", 1, 60],
  ["cycles", 1, 4],
  ["idle-seconds", 1, 30],
  ["burst", 0, 5000],
]) {
  if (!Number.isInteger(options[name]) || options[name] < min || options[name] > max)
    throw new Error(`Invalid participant benchmark option: ${name}`);
}
const directory = resolve(root, options.package);
const manifestBytes = await readFile(join(directory, "manifest.json"));
const manifest = JSON.parse(manifestBytes);
assert.equal(manifest.platform, platform());
assert.equal(manifest.architecture, arch());
assert.equal(manifest.verification.packagedHttpWorkflow, "passed");
for (const [name, expected] of Object.entries(manifest.files)) {
  const path = resolve(directory, name);
  assert.ok(path.startsWith(directory + sep));
  assert.equal(
    createHash("sha256")
      .update(await readFile(path))
      .digest("hex"),
    expected,
    name,
  );
}
const executable = platform() === "win32" ? "rustsurvey-server.exe" : "rustsurvey-server";
const binary = join(directory, executable);
const report = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  subject: "Paced virtual participants on the delivered pilot; no LimeSurvey comparison",
  source: {
    ...manifest.source,
    binarySha256: manifest.files[executable],
    packageManifestSha256: createHash("sha256").update(manifestBytes).digest("hex"),
  },
  host: {
    platform: platform(),
    release: release(),
    arch: arch(),
    cpu: cpus()[0]?.model ?? "unknown",
    logicalProcessors: cpus().length,
    totalMemoryBytes: totalmem(),
    node: process.version,
  },
  configuration: {
    ...options,
    package: undefined,
    database: "disposable SQLite file, default driver settings",
    objectStore: "file",
    http: "one independent persistent HTTP/1.1 connection per virtual user",
    https: false,
    browser: false,
    sameHostClient: true,
    retries: 0,
    queue: 8,
    memorySamplingIntervalMs: 100,
    workload: "28 native question types, all on one page",
    coldAssetsPerUser: true,
  },
  scenarios: [],
};
const abort = new AbortController();
let current = null;
let liveUsers = [];
for (const signal of ["SIGINT", "SIGTERM"])
  process.on(signal, () => {
    abort.abort();
    for (const user of liveUsers) user.close();
    current?.interrupt();
  });
const fixture = await workload("forms");
let serial = 100000;
const output = join(
  root,
  "target/performance",
  `${report.generatedAt.replace(/[:.]/g, "-")}-participants.json`,
);
await mkdir(dirname(output), { recursive: true });
report.state = "running";
await writeFile(output, `${JSON.stringify(report, null, 2)}\n`, { flag: "wx" });
async function checkpoint() {
  await writeFile(output, `${JSON.stringify(report, null, 2)}\n`);
}

async function completeAll(tasks) {
  const settled = await Promise.allSettled(tasks);
  const failed = settled.find((result) => result.status === "rejected");
  if (failed !== undefined) throw failed.reason;
}
function accepted(result, expected) {
  if (result.status === expected) return true;
  if (result.status !== 0 && result.status !== 503)
    throw new Error(`Unexpected participant status ${result.status}.`);
  return false;
}
async function scenario(count, burst) {
  const server = new DisposableServer(binary, 8, { assetsDirectory: join(directory, "web") });
  current = server;
  let metrics;
  const users = [];
  liveUsers = users;
  try {
    await server.start();
    metrics = await processMetrics(server.child);
    await server.login();
    const run = await campaign(server, fixture);
    const index = await (await server.request("/")).text();
    const assets = [
      ...new Set(
        [...index.matchAll(/(?:src|href)="(\/assets\/[^"?]+)"/g)].map((match) => match[1]),
      ),
    ];
    assert.ok(assets.some((path) => path.endsWith(".js")));
    for (let i = 0; i < count; i += 1) {
      const id = identifier(serial++);
      const token = createHash("sha256").update(`synthetic-participant:${id}`).digest("hex");
      users.push(new VirtualUser(server.base, id, token, fixture.answers));
    }
    const item = {
      kind: burst ? "burst" : "paced",
      requestedParticipants: count,
      profile: run.profile,
      definitionSha256: fixture.definitionSha256,
      phases: {},
    };
    report.scenarios.push(item);
    console.log(
      `${item.kind}: ${count} participants, downloading the client and opening responses.`,
    );
    item.phases.arrival = await observe(metrics, users, async (request, start) => {
      await completeAll(
        users.map(async (user, number) => {
          const lag = await waitUntil(
            start,
            burst ? 0 : (number * options["arrival-seconds"] * 1000) / count,
            abort.signal,
          );
          for (const path of ["/", ...assets, `/api/runs/${run.id}`]) {
            const kind = path.startsWith("/api/") ? "definition" : "asset";
            const loaded = await request(
              user,
              kind,
              path,
              { signal: abort.signal },
              path === "/" ? lag : null,
            );
            if (!accepted(loaded, 200)) return;
          }
          const response = await request(user, "start", `/api/runs/${run.id}/responses`, {
            method: "POST",
            authorized: true,
            signal: abort.signal,
            body: { responseId: user.id, language: "fr" },
          });
          if (accepted(response, 201)) {
            assert.equal(JSON.parse(response.body.toString("utf8")).id, user.id);
            user.entered = true;
          }
        }),
      );
    });
    const entered = users.filter((user) => user.entered);
    item.enteredParticipants = entered.length;
    await checkpoint();
    console.log(
      `${item.kind}: ${entered.length}/${count} entered; ${item.phases.arrival.traffic.rejected} rejected requests.`,
    );
    item.phases.idle = await observe(metrics, users, async () => {
      await setTimeout(options["idle-seconds"] * 1000, undefined, { signal: abort.signal });
    });
    if (!burst && entered.length > 0) {
      const interval = options["think-seconds"] * 1000;
      const offsets = phaseOffsets(entered.length, interval);
      item.offeredWritesPerSecond = entered.length / options["think-seconds"];
      console.log(
        `paced: ${entered.length} users thinking; ${item.offeredWritesPerSecond.toFixed(2)} planned writes/s for ${options.cycles * options["think-seconds"]} seconds.`,
      );
      item.phases.activity = await observe(metrics, users, async (request, start) => {
        await completeAll(
          entered.map(async (user, number) => {
            for (let cycle = 0; cycle < options.cycles; cycle += 1) {
              const lag = await waitUntil(start, offsets[number] + cycle * interval, abort.signal);
              if (user.pending === null)
                user.pending = {
                  expectedVersion: user.version,
                  operationId: identifier(serial++),
                  answers: { ...user.answers, U: `paced-${cycle}` },
                  checkpoint: { language: "fr", groupId: null, pageId: "all" },
                  submit: cycle === options.cycles - 1,
                };
              const command = user.pending;
              const response = await request(
                user,
                command.submit ? "submit" : "save",
                `/api/responses/${user.id}`,
                { method: "PUT", authorized: true, body: command, signal: abort.signal },
                lag,
              );
              if (!accepted(response, 200)) continue;
              const result = JSON.parse(response.body.toString("utf8"));
              assert.equal(result.id, user.id);
              assert.equal(result.version, command.expectedVersion + 1);
              assert.equal(result.status, command.submit ? "submitted" : "draft");
              user.version = result.version;
              user.status = result.status;
              user.answers = command.answers;
              user.pending = null;
            }
          }),
        );
        await waitUntil(start, options.cycles * interval, abort.signal);
      });
      console.log(
        `paced: ${item.phases.activity.traffic.successful}/${item.phases.activity.traffic.attempts} writes accepted, ${item.phases.activity.traffic.successfulLatencyMs.p95?.toFixed(2)} ms p95.`,
      );
    }
    item.submittedParticipants = entered.filter((user) => user.status === "submitted").length;
    await checkpoint();
    // Verify every acknowledged row through the admin API, without persisting any answer or token in the report.
    let next = 0;
    async function verifyRows() {
      while (next < entered.length) {
        const user = entered[next++];
        const stored = await server.json(
          `/api/admin/runs/${run.id}/responses/${user.id}/response.json`,
        );
        assert.equal(stored.id, user.id);
        assert.equal(stored.version, user.version);
        assert.equal(stored.status, user.status);
        if (user.version > 0) {
          assert.equal(stored.answers.U, user.answers.U);
          assert.equal(stored.answers.CALC, fixture.computedValue);
        }
      }
    }
    await completeAll(Array.from({ length: Math.min(8, entered.length) }, verifyRows));
    item.rowsVerified = entered.length;
    for (const user of users) user.close();
    await metrics.close();
    metrics = null;
    await server.stop();
    await server.start();
    next = 0;
    await completeAll(Array.from({ length: Math.min(8, entered.length) }, verifyRows));
    item.rowsVerifiedAfterRestart = entered.length;
    await checkpoint();
  } finally {
    for (const user of users) user.close();
    await metrics?.close();
    await server.dispose();
    current = null;
    liveUsers = [];
  }
}
try {
  for (const count of options.clients) await scenario(count, false);
  if (options.burst > 0) await scenario(options.burst, true);
  report.state = "complete";
} catch (error) {
  report.state = "failed";
  throw error;
} finally {
  report.completedAt = new Date().toISOString();
  await checkpoint();
}
console.log(`Participant report: ${output}`);
