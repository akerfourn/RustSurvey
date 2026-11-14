import assert from "node:assert/strict";
import { test } from "node:test";
import { decodeReport } from "../api/inspection.ts";
import { sourceBehaviorIssues } from "./source-behavior.ts";

function report(status?: "complete" | "partial"): unknown {
  const value = {
    sourceSchema: 623,
    targetSchema: 623,
    errors: 0,
    warnings: 1,
    omittedDiagnostics: 0,
    checkedSections: [],
    diagnostics: [],
    sections: [{ name: "defaultvalues", rows: 1 }],
  };
  if (status === undefined) return value;
  return {
    ...value,
    defaultProjection: {
      profile: "source-defaults-v1",
      status,
      sourceValues: 2,
      projectedValues: 2,
      issueCount: 0,
      issues: [],
    },
  };
}

test("test_imported_preview_should_allow_projected_defaults_and_keep_legacy_and_partial_imports_blocked", () => {
  assert.deepEqual(sourceBehaviorIssues(decodeReport(report("complete"))), []);
  for (const status of [undefined, "partial"] as const) {
    assert.match(sourceBehaviorIssues(decodeReport(report(status)))[0] ?? "", /defaultvalues/);
  }
  const completed = decodeReport(report("complete"));
  completed.sections.push({ name: "quota", rows: 1 });
  assert.match(sourceBehaviorIssues(completed)[0] ?? "", /quota/);
});

test("test_source_defaults_should_reject_unknown_projection_profiles_and_surface_target_diagnostics", () => {
  const partial = decodeReport(report("partial"));
  partial.defaultProjection!.issues.push({
    path: "source.questions.1.defaults",
    message: "Identité source absente.",
  });
  assert.ok(
    sourceBehaviorIssues(partial).some((issue) => issue.includes("Identité source absente")),
  );
  assert.throws(
    () =>
      decodeReport({
        ...partial,
        defaultProjection: { ...partial.defaultProjection, profile: "future" },
      }),
    /Profil de projection/,
  );
});
