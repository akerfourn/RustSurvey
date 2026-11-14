import assert from "node:assert/strict";
import { createServer } from "node:http";
import { test } from "node:test";
import { phaseOffsets, trafficSummary } from "./paced-measure.mjs";
import { VirtualUser } from "./virtual-user.mjs";

test("test_participant_schedule_should_spread_each_user_over_the_interval_without_synchronizing_to_arrival_order", () => {
  for (const count of [10, 137]) {
    const offsets = phaseOffsets(count, 30000);
    assert.deepEqual(
      [...offsets].sort((left, right) => left - right),
      Array.from({ length: count }, (_, index) => ((index + 0.5) * 30000) / count),
    );
    assert.equal(new Set(offsets).size, count);
    assert.notDeepEqual(
      offsets,
      [...offsets].sort((left, right) => left - right),
    );
  }
  assert.throws(() => phaseOffsets(0, 30000), /Invalid participant schedule/);
});

test("test_participant_traffic_should_count_network_errors_and_keep_schedule_lag_separate_from_response_latency", () => {
  const base = { receivedBytes: 10, sentBytes: 2, scheduleLagMs: null };
  const result = trafficSummary(
    [
      { ...base, status: 200, code: "ok", durationMs: 20, scheduleLagMs: 100 },
      { ...base, status: 503, code: "write_busy", durationMs: 1 },
      { ...base, status: 0, code: "network_error", durationMs: 15000 },
    ],
    30000,
  );
  assert.equal(result.successful, 1);
  assert.equal(result.rejected, 2);
  assert.equal(result.statuses["0:network_error"], 1);
  assert.equal(result.successfulLatencyMs.p95, 20);
  assert.equal(result.scheduleLagMs.p95, 100);
  assert.equal(result.receivedBytes, 30);
});

test("test_virtual_users_should_reuse_their_own_connections_and_bound_received_bodies", async () => {
  const peers = [];
  const server = createServer((request, response) => {
    peers.push(request.socket.remotePort);
    if (request.url === "/large") {
      response.end("x".repeat(100));
      return;
    }
    response.end("été");
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const base = `http://127.0.0.1:${server.address().port}`;
  const first = new VirtualUser(base, "first", "synthetic-token", {});
  const second = new VirtualUser(base, "second", "synthetic-token", {});
  try {
    const result = await first.request("/");
    assert.equal(result.body.toString("utf8"), "été");
    assert.equal(result.bytes, Buffer.byteLength("été"));
    await first.request("/");
    await second.request("/");
    assert.equal(peers[0], peers[1], "requests of one participant must reuse the connection");
    assert.notEqual(peers[0], peers[2], "participants must not share the same connection pool");
    assert.equal(first.connections, 1);
    assert.equal(second.connections, 1);
    await assert.rejects(first.request("/large", { limit: 20 }), /size limit/);
  } finally {
    first.close();
    second.close();
    await new Promise((resolve) => server.close(resolve));
  }
});
