import assert from "node:assert/strict";
import { afterEach, test } from "node:test";
import { listSourceContexts, uploadSourceContext } from "./source-context-api.ts";
import { checkCollection } from "./collection-check-api.ts";

const originalFetch = globalThis.fetch;
const STUDY = "00000000-0000-4000-8000-000000000001";
const CONTEXT = "00000000-0000-4000-8000-000000000002";
function metadata() {
  return {
    id: CONTEXT,
    studyId: STUDY,
    revision: 2,
    sourceSha256: "a".repeat(64),
    sha256: "b".repeat(64),
    size: 2,
    profile: "limesurvey-group-settings-v1",
    complete: false,
    createdAt: 1800000000,
  };
}
afterEach(() => {
  globalThis.fetch = originalFetch;
});

test("source context upload should send exact file bytes with CSRF and private request options", async () => {
  const file = new File(["{}"], "source.json", { type: "application/json" });
  globalThis.fetch = async (url, options) => {
    assert.equal(url, `/api/admin/studies/${STUDY}/revisions/2/settings-contexts`);
    assert.equal(options?.method, "POST");
    assert.equal(options?.body, file);
    assert.equal(options?.credentials, "same-origin");
    assert.equal(options?.cache, "no-store");
    assert.equal(new Headers(options?.headers).get("X-CSRF-Token"), "csrf");
    return Response.json(metadata());
  };
  assert.equal((await uploadSourceContext(STUDY, 2, file, "csrf")).id, CONTEXT);
});

test("source context upload should reject empty or oversized files before contacting the server", async () => {
  globalThis.fetch = async () => {
    assert.fail("invalid files must not be uploaded");
  };
  for (const file of [new File([], "empty.json"), new File([" ".repeat(65537)], "large.json")])
    await assert.rejects(uploadSourceContext(STUDY, 2, file, "csrf"), /64 Kio/);
});

test("source context lists should reject a foreign revision and duplicate snapshots", async () => {
  for (const items of [[{ ...metadata(), revision: 1 }], [metadata(), metadata()]]) {
    globalThis.fetch = async () => Response.json({ items });
    await assert.rejects(listSourceContexts(STUDY, 2), /autre révision|dupliqués/);
  }
});

test("collection check should qualify only the explicitly requested source context", async () => {
  globalThis.fetch = async (url) => {
    assert.equal(
      url,
      `/api/admin/studies/${STUDY}/revisions/2/collection-check?contextId=${CONTEXT}`,
    );
    return Response.json({
      schemaVersion: 1,
      revision: 2,
      sourceContext: null,
      execution: "blocked",
      continuity: "unavailable",
      projection: {},
    });
  };
  await assert.rejects(checkCollection(STUDY, 2, undefined, CONTEXT), /autre contexte/);
});
