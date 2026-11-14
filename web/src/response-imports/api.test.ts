import assert from "node:assert/strict";
import { test } from "node:test";
import { complete, decodeDetails, upload } from "./api.ts";

const batch = {
  id: "00000000-0000-4000-8000-000000000001",
  revision: 1,
  state: "loading",
  rowCount: 520,
  loadedRows: 500,
  sha256: "a".repeat(64),
  createdAt: 100,
  report: {
    conservation: "available",
    mapping: "complete",
    execution: "unverified",
    continuity: "unavailable",
    submitted: 10,
    drafts: 510,
    unknownStatus: 0,
    mappedCells: 2,
    unknownColumnCount: 0,
    missingColumnCount: 0,
    duplicateIds: 0,
    invalidIds: 0,
    issueCount: 0,
    issues: [],
  },
};

test("test_response_import_should_validate_progress_and_keep_unverified_execution_distinct", () => {
  assert.equal(decodeDetails(batch).report.execution, "unverified");
  assert.throws(() => decodeDetails({ ...batch, loadedRows: 521 }), /Progression/);
  assert.throws(() => decodeDetails({ ...batch, state: "prepared" }), /Progression/);
  assert.throws(() => decodeDetails({ ...batch, state: "published" }), /État/);
});

test("test_response_import_should_resume_by_batch_without_uploading_the_file_again", async () => {
  const original = globalThis.fetch;
  const calls: { url: string; options: RequestInit }[] = [];
  globalThis.fetch = async (input, options = {}) => {
    calls.push({ url: String(input), options });
    return Response.json({ ...batch, state: "prepared", loadedRows: 520 });
  };
  try {
    const progress: number[] = [];
    const result = await complete(
      decodeDetails(batch),
      "synthetic-csrf",
      new AbortController().signal,
      (next) => progress.push(next.loadedRows),
    );
    assert.equal(result.state, "prepared");
    assert.deepEqual(progress, [500, 520]);
    assert.equal(calls.length, 1);
    const call = calls[0];
    assert.ok(call !== undefined);
    assert.equal(call.url, `/api/admin/response-imports/${batch.id}/continue`);
    assert.equal(call.options.body, "{}");
    assert.equal(new Headers(call.options.headers).get("X-CSRF-Token"), "synthetic-csrf");
  } finally {
    globalThis.fetch = original;
  }
});

test("test_response_import_should_stop_when_resume_does_not_advance_or_changes_identity", async () => {
  const original = globalThis.fetch;
  try {
    globalThis.fetch = async () => Response.json(batch);
    await assert.rejects(
      complete(decodeDetails(batch), "synthetic-csrf", new AbortController().signal, () => {}),
      /pas progressé/,
    );
    globalThis.fetch = async () =>
      Response.json({ ...batch, id: "different", state: "prepared", loadedRows: 520 });
    await assert.rejects(
      complete(decodeDetails(batch), "synthetic-csrf", new AbortController().signal, () => {}),
      /identité/,
    );
  } finally {
    globalThis.fetch = original;
  }
});

test("test_response_import_should_refuse_empty_files_before_sending_them", async () => {
  const original = globalThis.fetch;
  let calls = 0;
  globalThis.fetch = async () => {
    calls += 1;
    throw new Error("unexpected request");
  };
  try {
    await assert.rejects(
      upload("study", 1, new File([], "empty.lsr"), "synthetic-csrf", new AbortController().signal),
      /8 Mio/,
    );
    assert.equal(calls, 0);
  } finally {
    globalThis.fetch = original;
  }
});
