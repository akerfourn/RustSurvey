import { execFile, spawn } from "node:child_process";
import { readFile } from "node:fs/promises";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execute = promisify(execFile);

export function decodeSample(value) {
  const keys = ["cpuMs", "residentBytes", "peakResidentBytes", "privateCommittedBytes"];
  for (const key of keys) {
    if (key === "privateCommittedBytes" && value[key] === null) continue;
    if (!Number.isFinite(value[key]) || value[key] < 0)
      throw new Error(`Invalid process metric: ${key}`);
  }
  return Object.fromEntries(keys.map((key) => [key, value[key]]));
}

export function linuxSample(stat, status, clockTicks) {
  if (!Number.isSafeInteger(clockTicks) || clockTicks <= 0)
    throw new Error("Invalid CPU clock ticks.");
  const end = stat.lastIndexOf(")");
  if (end < 0) throw new Error("Invalid /proc process status.");
  const fields = stat
    .slice(end + 1)
    .trim()
    .split(/\s+/);
  const cpuTicks = Number(fields[11]) + Number(fields[12]);
  const memory = new Map();
  for (const line of status.split("\n")) {
    const match = /^(VmRSS|VmHWM):\s+(\d+) kB$/.exec(line.trim());
    if (match !== null) memory.set(match[1], Number(match[2]) * 1024);
  }
  return decodeSample({
    cpuMs: (cpuTicks * 1000) / clockTicks,
    residentBytes: memory.get("VmRSS"),
    peakResidentBytes: memory.get("VmHWM"),
    privateCommittedBytes: null,
  });
}

export async function processMetrics(child) {
  if (process.platform === "linux") {
    const { stdout } = await execute("getconf", ["CLK_TCK"], { timeout: 2000 });
    const clockTicks = Number(stdout.trim());
    return {
      async sample() {
        if (child.exitCode !== null || child.signalCode !== null)
          throw new Error("The measured process has exited.");
        const [stat, status] = await Promise.all([
          readFile(`/proc/${child.pid}/stat`, "utf8"),
          readFile(`/proc/${child.pid}/status`, "utf8"),
        ]);
        return linuxSample(stat, status, clockTicks);
      },
      async close() {},
    };
  }
  if (process.platform !== "win32")
    throw new Error("Process metrics currently support Windows and Linux.");
  const helper = spawn(
    "powershell.exe",
    [
      "-NoLogo",
      "-NoProfile",
      "-NonInteractive",
      "-File",
      fileURLToPath(new URL("process-metrics.ps1", import.meta.url)),
      "-TargetProcessId",
      String(child.pid),
    ],
    {
      windowsHide: true,
      stdio: ["pipe", "pipe", "pipe"],
    },
  );
  helper.stderr.resume();
  const lines = createInterface({ input: helper.stdout });
  let pending = null;
  let failure = null;
  function fail(error) {
    failure = error;
    pending?.reject(error);
    pending = null;
  }
  helper.on("error", () => fail(new Error("Cannot start the process metrics helper.")));
  helper.stdin.on("error", () => fail(new Error("The process metrics pipe closed.")));
  helper.on("exit", () => fail(new Error("The process metrics helper exited.")));
  lines.on("line", (line) => {
    if (pending === null) return;
    const request = pending;
    pending = null;
    try {
      request.resolve(decodeSample(JSON.parse(line)));
    } catch {
      request.reject(new Error("Invalid process metrics response."));
    }
  });
  let queue = Promise.resolve();
  return {
    sample() {
      const result = queue.then(
        () =>
          new Promise((resolve, reject) => {
            if (failure !== null) return reject(failure);
            if (child.exitCode !== null || child.signalCode !== null)
              return reject(new Error("The measured process has exited."));
            const timer = setTimeout(() => {
              pending = null;
              fail(new Error("Process metrics timed out."));
              reject(failure);
            }, 3000);
            pending = {
              resolve: (value) => {
                clearTimeout(timer);
                resolve(value);
              },
              reject: (error) => {
                clearTimeout(timer);
                reject(error);
              },
            };
            helper.stdin.write("sample\n");
          }),
      );
      queue = result.catch(() => {});
      return result;
    },
    async close() {
      await queue;
      helper.stdin.end("stop\n");
      if (helper.exitCode !== null || helper.signalCode !== null) return;
      await new Promise((resolve) => {
        const timer = setTimeout(() => helper.kill(), 2000);
        helper.once("exit", () => {
          clearTimeout(timer);
          resolve();
        });
      });
    },
  };
}
