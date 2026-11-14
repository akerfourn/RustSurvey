import assert from "node:assert/strict";
import { test } from "node:test";
import { decodeSample, linuxSample } from "./metrics.mjs";
import { summarize } from "./measure.mjs";

test("test_linux_metrics_should_read_cpu_ticks_and_resident_memory_with_parentheses_in_the_command", () => {
  const fields = Array(24).fill("0");
  fields[0] = "R";
  fields[11] = "150";
  fields[12] = "50";
  const stat = `123 (server (test) worker) ${fields.join(" ")}`;
  assert.deepEqual(linuxSample(stat, "VmRSS:\t21 kB\nVmHWM:\t42 kB\n", 100), {
    cpuMs: 2000,
    residentBytes: 21 * 1024,
    peakResidentBytes: 42 * 1024,
    privateCommittedBytes: null,
  });
  assert.throws(() => linuxSample(stat, "", 100), /Invalid process metric/);
  assert.throws(() => linuxSample(stat, "", 0), /Invalid CPU clock ticks/);
});

test("test_metrics_should_refuse_missing_or_nonfinite_measurements_instead_of_reporting_zero", () => {
  const valid = {
    cpuMs: 150,
    residentBytes: 1024,
    peakResidentBytes: 2048,
    privateCommittedBytes: 4096,
  };
  assert.deepEqual(decodeSample(valid), valid);
  assert.throws(() => decodeSample({ ...valid, cpuMs: NaN }), /Invalid process metric: cpuMs/);
  assert.throws(
    () => decodeSample({ ...valid, residentBytes: -1 }),
    /Invalid process metric: residentBytes/,
  );
});

test("test_measurement_summary_should_separate_rejected_work_from_successful_latency_and_throughput", () => {
  const before = {
    cpuMs: 100,
    residentBytes: 10,
    peakResidentBytes: 10,
    privateCommittedBytes: null,
  };
  const after = {
    cpuMs: 140,
    residentBytes: 15,
    peakResidentBytes: 30,
    privateCommittedBytes: null,
  };
  const result = summarize(
    [
      { status: 200, code: "ok", durationMs: 10 },
      { status: 503, code: "write_busy", durationMs: 1 },
      { status: 200, code: "ok", durationMs: 20 },
      { status: 200, code: "ok", durationMs: 30 },
    ],
    200,
    before,
    after,
    [before, { ...after, residentBytes: 25 }, after],
  );
  assert.equal(result.successful, 3);
  assert.deepEqual(result.statuses, { "200:ok": 3, "503:write_busy": 1 });
  assert.equal(result.successesPerSecond, 15);
  assert.equal(result.cpuPercentOfOneCore, 20);
  assert.equal(result.cpuMsPerSuccess, 40 / 3);
  assert.deepEqual(result.successfulLatencyMs, { p50: 20, p95: 30, p99: 30 });
  assert.equal(result.sampledPeakResidentBytes, 25);
  assert.equal(result.lifetimePeakResidentBytes, 30);
  assert.equal(result.rejectedLatencyMs.p95, 1);
});
