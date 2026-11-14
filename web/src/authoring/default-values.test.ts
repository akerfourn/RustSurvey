import assert from "node:assert/strict";
import { test } from "node:test";
import { createDefinition, createQuestion, changeKind, type Question } from "./model.ts";
import { decodeDefinition } from "./definition-codec.ts";
import { defaultTargets, setDefaultValue, pruneQuestionDefaults } from "./default-values.ts";
import { decodeQuestionDefaults } from "../survey/defaults.ts";
import { createRuntime, evaluateSnapshot, runtimeIssues } from "../survey/engine.ts";
import { toViewer } from "./model.ts";

function fixture() {
  let counter = 0;
  function identifier(): string {
    counter += 1;
    return `id-${counter}`;
  }
  const definition = createDefinition(identifier);
  const question = changeKind(createQuestion(definition, identifier), "multiple_text", identifier);
  definition.groups[0]!.questions.push(question);
  definition.languages = ["fr", "en"];
  return { definition, question, identifier };
}

test("test_defaults_should_distinguish_empty_and_absent_values_and_keep_other_languages", () => {
  const { question } = fixture();
  const target = defaultTargets(question)[0]!;
  const french = setDefaultValue(question, target, "fr", "");
  const bilingual = setDefaultValue(french, target, "en", "hello");
  assert.equal(question.defaults, null);
  assert.deepEqual(
    bilingual.defaults!.values.map((value) => [value.language, value.value]),
    [
      ["fr", ""],
      ["en", "hello"],
    ],
  );
  const removed = setDefaultValue(bilingual, target, "fr", null);
  assert.deepEqual(
    removed.defaults!.values.map((value) => value.language),
    ["en"],
  );
  assert.equal(bilingual.defaults!.values.length, 2);
});

test("test_default_targets_should_survive_code_changes_and_ordering", () => {
  const { question } = fixture();
  const original = defaultTargets(question)[0]!;
  const configured = setDefaultValue(question, original, "fr", "001");
  const renamed: Question = {
    ...configured,
    code: "RENAMED",
    subquestions: [...configured.subquestions]
      .reverse()
      .map((row) => ({ ...row, code: `NEW${row.code}` })),
  };
  assert.ok(defaultTargets(renamed).some((target) => target.key === original.key));
  assert.deepEqual(pruneQuestionDefaults(renamed, configured).defaults, configured.defaults);
});

test("test_defaults_should_remove_only_cells_deleted_by_an_explicit_edit", () => {
  const { question, identifier } = fixture();
  const targets = defaultTargets(question);
  let configured = setDefaultValue(question, targets[0]!, "fr", "first");
  configured = setDefaultValue(configured, targets[1]!, "fr", "second");
  const removed = pruneQuestionDefaults(
    { ...configured, subquestions: configured.subquestions.slice(1) },
    configured,
  );
  assert.equal(removed.defaults!.values.length, 1);
  assert.equal(removed.defaults!.values[0]!.value, "second");
  assert.equal(changeKind(configured, "single_choice", identifier).defaults!.values.length, 0);
  const invalid = structuredClone(configured);
  invalid.defaults!.values[0]!.coordinate = {
    kind: "subquestion",
    id: "already-missing",
    scale: null,
  };
  assert.equal(
    pruneQuestionDefaults({ ...invalid, help: "edited" }, invalid).defaults!.values.length,
    2,
    "an unrelated edit must not hide a pre-existing invalid reference",
  );
});

test("test_default_codec_should_upgrade_v3_without_mutation_and_require_v4_for_new_fields", () => {
  const { definition, question } = fixture();
  const legacy = { ...definition, schemaVersion: 3 };
  const before = structuredClone(legacy);
  assert.equal(decodeDefinition(legacy).schemaVersion, 4);
  assert.deepEqual(legacy, before);
  definition.groups[0]!.questions[0] = setDefaultValue(
    question,
    defaultTargets(question)[0]!,
    "fr",
    "",
  );
  assert.deepEqual(decodeDefinition(definition), definition);
  assert.throws(() => decodeDefinition({ ...definition, schemaVersion: 3 }), /v4/);
});

test("test_default_codec_should_reject_unknown_coordinates_roles_and_oversized_values", () => {
  const base = {
    sameForAllLanguages: false,
    values: [{ coordinate: { kind: "primary" }, role: "answer", language: "fr", value: "" }],
  };
  assert.throws(() => decodeQuestionDefaults({ ...base, extra: true }), /inconnu/);
  for (const entry of [
    { ...base.values[0], coordinate: { kind: "matrix" } },
    { ...base.values[0], role: "calculation" },
    { ...base.values[0], value: "x".repeat(4097) },
  ])
    assert.throws(() => decodeQuestionDefaults({ ...base, values: [entry] }), /défaut|Coordonnée/);
});

test("test_configured_defaults_should_wait_for_explicit_page_entry_in_the_viewer", () => {
  const { definition, question } = fixture();
  definition.groups[0]!.questions[0] = setDefaultValue(
    question,
    defaultTargets(question)[0]!,
    "fr",
    "initial",
  );
  const runtime = createRuntime(toViewer(definition));
  const snapshot = evaluateSnapshot(runtime, new Map());
  assert.deepEqual(runtimeIssues(runtime, snapshot), []);
  assert.equal(snapshot.answers.size, 0);
  const initialized = evaluateSnapshot(runtime, new Map(), {
    language: "fr",
    initialize: new Set(runtime.questions.keys()),
  });
  assert.equal(
    initialized.answers.get(`${question.code}_${question.subquestions[0]!.code}`),
    "initial",
  );
});
