import { setTimeout } from "node:timers/promises";
import { percentile } from "./measure.mjs";

/** Uniform slots, deterministically permuted so arrivals and saves do not stay synchronized. */
export function phaseOffsets(count, intervalMs) {
  if (!Number.isInteger(count) || count < 1 || !Number.isFinite(intervalMs) || intervalMs <= 0)
    throw new Error("Invalid participant schedule.");
  const order = Array.from({ length: count }, (_, index) => index);
  order.sort((left, right) => hash(left) - hash(right) || left - right);
  const offsets = Array(count);
  for (let rank = 0; rank < count; rank += 1)
    offsets[order[rank]] = ((rank + 0.5) * intervalMs) / count;
  return offsets;
}
function hash(value) {
  return Math.imul(value + 1, 2654435761) >>> 0;
}

export async function waitUntil(start, offsetMs, signal) {
  const remaining = start + offsetMs - performance.now();
  if (remaining > 0) await setTimeout(remaining, undefined, { signal });
  if (signal.aborted) throw new Error("Participant benchmark interrupted.");
  return Math.max(0, performance.now() - start - offsetMs);
}

export function trafficSummary(attempts, durationMs) {
  const successful = attempts.filter((attempt) => attempt.status >= 200 && attempt.status < 300);
  const rejected = attempts.filter((attempt) => attempt.status < 200 || attempt.status >= 300);
  const statuses = {};
  for (const attempt of attempts) {
    const key = `${attempt.status}:${attempt.code}`;
    statuses[key] = (statuses[key] ?? 0) + 1;
  }
  const latencies = (items) =>
    Object.fromEntries(
      [0.5, 0.95, 0.99].map((fraction) => [
        `p${fraction * 100}`,
        percentile(
          items.map((item) => item.durationMs),
          fraction,
        ),
      ]),
    );
  return {
    attempts: attempts.length,
    successful: successful.length,
    rejected: rejected.length,
    statuses,
    successesPerSecond: (successful.length * 1000) / durationMs,
    successfulLatencyMs: latencies(successful),
    rejectedLatencyMs: latencies(rejected),
    receivedBytes: attempts.reduce((sum, item) => sum + item.receivedBytes, 0),
    sentBytes: attempts.reduce((sum, item) => sum + item.sentBytes, 0),
    scheduleLagMs: Object.fromEntries(
      [0.5, 0.95, 0.99].map((fraction) => [
        `p${fraction * 100}`,
        percentile(
          attempts.filter((item) => item.scheduleLagMs !== null).map((item) => item.scheduleLagMs),
          fraction,
        ),
      ]),
    ),
  };
}

export async function observe(metrics, users, action) {
  const before = await metrics.sample();
  const samples = [before];
  let pending = null;
  let samplingFailure = null;
  let active = 0;
  let peakActive = 0;
  let peakConnections = 0;
  const attempts = [];
  const timer = setInterval(() => {
    peakConnections = Math.max(
      peakConnections,
      users.reduce((sum, user) => sum + user.connections, 0),
    );
    if (pending !== null || samplingFailure !== null) return;
    pending = metrics
      .sample()
      .then((sample) => samples.push(sample))
      .catch((error) => {
        samplingFailure = error;
      })
      .finally(() => {
        pending = null;
      });
  }, 100);
  async function request(user, kind, path, options, scheduleLagMs = null) {
    active += 1;
    peakActive = Math.max(peakActive, active);
    const start = performance.now();
    let result;
    try {
      result = await user.request(path, options);
      let code = "ok";
      if (result.status >= 400) {
        try {
          code = JSON.parse(result.body.toString("utf8")).error?.code ?? "http_error";
        } catch {
          code = "http_error";
        }
      }
      attempts.push({
        kind,
        status: result.status,
        code,
        durationMs: performance.now() - start,
        scheduleLagMs,
        receivedBytes: result.bytes,
        sentBytes: result.sentBytes,
      });
    } catch {
      attempts.push({
        kind,
        status: 0,
        code: "network_error",
        durationMs: performance.now() - start,
        scheduleLagMs,
        receivedBytes: 0,
        sentBytes: 0,
      });
      result = { status: 0 };
    } finally {
      active -= 1;
    }
    return result;
  }
  const start = performance.now();
  try {
    await action(request, start);
  } finally {
    clearInterval(timer);
    if (pending !== null) await pending;
  }
  const durationMs = performance.now() - start;
  const after = await metrics.sample();
  if (samplingFailure !== null) throw samplingFailure;
  samples.push(after);
  peakConnections = Math.max(
    peakConnections,
    users.reduce((sum, user) => sum + user.connections, 0),
  );
  const resources = {
    durationMs,
    cpuMs: after.cpuMs - before.cpuMs,
    cpuPercentOfOneCore: ((after.cpuMs - before.cpuMs) * 100) / durationMs,
    residentBeforeBytes: before.residentBytes,
    residentAfterBytes: after.residentBytes,
    sampledPeakResidentBytes: Math.max(...samples.map((sample) => sample.residentBytes)),
    lifetimePeakResidentBytes: after.peakResidentBytes,
    privateCommittedAfterBytes: after.privateCommittedBytes,
    memorySamples: samples.length,
    peakHttpInFlight: peakActive,
    peakClientConnections: peakConnections,
    clientConnectionsAtEnd: users.reduce((sum, user) => sum + user.connections, 0),
  };
  const kinds = [...new Set(attempts.map((item) => item.kind))];
  return {
    resources,
    traffic: trafficSummary(attempts, durationMs),
    byKind: Object.fromEntries(
      kinds.map((kind) => [
        kind,
        trafficSummary(
          attempts.filter((item) => item.kind === kind),
          durationMs,
        ),
      ]),
    ),
  };
}
