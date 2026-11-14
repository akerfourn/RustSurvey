import assert from "node:assert/strict";
import { afterEach, test } from "node:test";
import { checkCollection } from "./collection-check-api.ts";
import { decodeGroupSettings } from "./group-settings.ts";

const originalFetch = globalThis.fetch;

test("group settings should reject contradictory resolution or provenance", () => {
  const setting = {
    name: "showwelcome",
    value: true,
    status: "resolved",
    origin: { layer: "survey", groupId: null },
  };
  for (const patch of [
    { value: null },
    { status: "unresolved" },
    { value: -1 },
    { origin: { layer: "unknown", groupId: null } },
  ]) {
    assert.throws(
      () =>
        decodeGroupSettings({
          schemaVersion: 1,
          complete: true,
          settings: [{ ...setting, ...patch }],
        }),
      /invalide/,
    );
  }
  assert.throws(
    () => decodeGroupSettings({ schemaVersion: 1, complete: true, settings: [setting, setting] }),
    /invalide/,
  );
});
afterEach(() => {
  globalThis.fetch = originalFetch;
});

function report(): Record<string, unknown> {
  return {
    schemaVersion: 1,
    revision: 2,
    sourceContext: null,
    execution: "blocked",
    continuity: "unavailable",
    runtime: "supported",
    mapping: "complete",
    mappedCells: 52,
    groupSettings: {
      schemaVersion: 1,
      complete: false,
      settings: [
        { name: "showwelcome", status: "unresolved", value: null, origin: null },
        {
          name: "usecookie",
          status: "resolved",
          value: false,
          origin: { layer: "survey", groupId: null },
        },
      ],
    },
    projection: { unprojectedSections: 1, unprojectedColumns: 4, missingSettings: 8 },
    issueCount: 1,
    issues: [{ path: "execution.reference", message: "Validation nécessaire." }],
  };
}

test("collection check should keep native support separate from imported execution", async () => {
  const controller = new AbortController();
  globalThis.fetch = async (url, options) => {
    assert.equal(url, "/api/admin/studies/study/revisions/2/collection-check");
    assert.equal(options?.credentials, "same-origin");
    assert.equal(options?.cache, "no-store");
    assert.equal(options?.signal, controller.signal);
    return Response.json(report());
  };
  const result = await checkCollection("study", 2, controller.signal);
  assert.equal(result.runtime, "supported");
  assert.equal(result.mappedCells, 52);
  assert.equal(result.missingSettings, 8);
  assert.equal(result.groupSettings[0]?.value, null);
  assert.equal(result.groupSettings[1]?.value, false);
});

test("collection check should reject a stale revision or an unrecognized qualification", async () => {
  for (const patch of [
    { revision: 1 },
    { schemaVersion: 2 },
    { execution: "available" },
    { issueCount: 0 },
    { runtime: "unknown" },
  ]) {
    globalThis.fetch = async () => Response.json({ ...report(), ...patch });
    await assert.rejects(checkCollection("study", 2), /invalide/);
  }
});
