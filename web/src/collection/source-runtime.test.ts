import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { decodeRuntimeFacts, readRuntimeFacts } from "./source-runtime.ts";

const facts: unknown = JSON.parse(
  readFileSync(new URL("../../../examples/rules/import-execution.json", import.meta.url), "utf8"),
).facts;

test("test_runtime_facts_should_read_the_versioned_source_document_without_inferring_missing_options", async () => {
  const parsed = decodeRuntimeFacts(facts);
  assert.deepEqual(
    await readRuntimeFacts(new File([JSON.stringify(facts)], "runtime.json")),
    parsed,
  );
  assert.equal(parsed.surveyId, "701");
  assert.equal(parsed.timeZone, "UTC");
  assert.equal(parsed.timeAdjust, "0");
  assert.throws(() => decodeRuntimeFacts({ ...parsed, showNoAnswer: undefined }), /Indicateur/);
  assert.throws(() => decodeRuntimeFacts({ ...parsed, timeZone: undefined }), /horaire/);
  assert.throws(() => decodeRuntimeFacts({ ...parsed, timeAdjust: undefined }), /horaire/);
});

test("test_runtime_facts_should_reject_unknown_fields_noncanonical_identities_and_invalid_counts", () => {
  const parsed = decodeRuntimeFacts(facts);
  assert.throws(() => decodeRuntimeFacts({ ...parsed, password: "private-value" }), /Format/);
  for (const surveyId of ["0", "0701", "18446744073709551616"])
    assert.throws(() => decodeRuntimeFacts({ ...parsed, surveyId }), /Identité/);
  for (const activePlugins of [-1, 0.5, 4294967296])
    assert.throws(() => decodeRuntimeFacts({ ...parsed, activePlugins }), /extensions/);
});

test("test_runtime_facts_should_bound_files_and_keep_unreadable_contents_out_of_errors", async () => {
  await assert.rejects(readRuntimeFacts(new File([" ".repeat(65537)], "large.json")), /64 Kio/);
  await assert.rejects(
    readRuntimeFacts(new File(['{"secret":"private-value'], "invalid.json")),
    (cause: unknown) => {
      assert.ok(cause instanceof Error);
      assert.match(cause.message, /JSON lisible/);
      assert.equal(cause.message.includes("private-value"), false);
      return true;
    },
  );
});
