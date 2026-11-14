import { performance } from "node:perf_hooks";

export function percentile(values, fraction) {
  if (values.length === 0) return null;
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.max(0, Math.ceil(sorted.length * fraction) - 1)];
}

export function summarize(attempts, durationMs, before, after, samples) {
  const successful = attempts.filter((attempt) => attempt.status >= 200 && attempt.status < 300);
  const statuses = {};
  for (const attempt of attempts) {
    const key = `${attempt.status}:${attempt.code}`;
    statuses[key] = (statuses[key] ?? 0) + 1;
  }
  const cpuMs = after.cpuMs - before.cpuMs;
  if (cpuMs < 0 || durationMs <= 0 || attempts.length === 0)
    throw new Error("Invalid measurement interval.");
  return {
    attempts: attempts.length,
    successful: successful.length,
    statuses,
    durationMs,
    cpuMs,
    cpuMsPerAttempt: cpuMs / attempts.length,
    cpuMsPerSuccess: successful.length === 0 ? null : cpuMs / successful.length,
    successesPerSecond: (successful.length * 1000) / durationMs,
    cpuPercentOfOneCore: (cpuMs * 100) / durationMs,
    successfulLatencyMs: {
      p50: percentile(
        successful.map((attempt) => attempt.durationMs),
        0.5,
      ),
      p95: percentile(
        successful.map((attempt) => attempt.durationMs),
        0.95,
      ),
      p99: percentile(
        successful.map((attempt) => attempt.durationMs),
        0.99,
      ),
    },
    rejectedLatencyMs: {
      p95: percentile(
        attempts.filter((attempt) => attempt.status >= 300).map((attempt) => attempt.durationMs),
        0.95,
      ),
    },
    residentBeforeBytes: before.residentBytes,
    residentAfterBytes: after.residentBytes,
    sampledPeakResidentBytes: Math.max(...samples.map((sample) => sample.residentBytes)),
    lifetimePeakResidentBytes: after.peakResidentBytes,
    privateCommittedAfterBytes: after.privateCommittedBytes,
    memorySamples: samples.length,
  };
}

export async function measure(metrics, count, concurrency, operation) {
  const before = await metrics.sample();
  const samples = [before];
  let pending = null;
  let failure = null;
  const timer = setInterval(() => {
    if (pending !== null || failure !== null) return;
    pending = metrics
      .sample()
      .then((sample) => samples.push(sample))
      .catch((error) => {
        failure = error;
      })
      .finally(() => {
        pending = null;
      });
  }, 100);
  const attempts = [];
  let next = 0;
  let workerFailure = null;
  const started = performance.now();
  try {
    const workers = Array.from({ length: concurrency }, async (_, worker) => {
      try {
        while (next < count && workerFailure === null) {
          if (failure !== null) throw failure;
          if (performance.now() - started > 60_000)
            throw new Error("Benchmark phase exceeded 60 seconds.");
          const index = next;
          next += 1;
          const start = performance.now();
          const result = await operation(worker, index);
          attempts.push({ ...result, durationMs: performance.now() - start });
        }
      } catch (error) {
        workerFailure = error;
        throw error;
      }
    });
    const settled = await Promise.allSettled(workers);
    const failed = settled.find((result) => result.status === "rejected");
    if (failed !== undefined) throw failed.reason;
  } finally {
    clearInterval(timer);
    if (pending !== null) await pending;
  }
  const durationMs = performance.now() - started;
  const after = await metrics.sample();
  if (failure !== null) throw failure;
  samples.push(after);
  return summarize(attempts, durationMs, before, after, samples);
}
