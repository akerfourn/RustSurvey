import assert from "node:assert/strict";
import { test } from "node:test";
import { createRuntime, evaluateSnapshot, runtimeIssues } from "../survey/engine.ts";
import { validate } from "../survey/validation.ts";
import {
  QUESTION_KINDS,
  changeKind,
  createDefinition,
  createQuestion,
  moveItem,
  toViewer,
  type Definition,
} from "./model.ts";

function ids(): () => string {
  let serial = 0;
  return () => `node-${++serial}`;
}

function completeDefinition(): Definition {
  const makeId = ids();
  const definition = createDefinition(makeId);
  const group = definition.groups[0]!;
  for (const kind of QUESTION_KINDS.keys()) {
    const question = changeKind(createQuestion(definition, makeId), kind, makeId);
    if (kind !== "information") question.mandatory = "strict";
    group.questions.push(question);
  }
  return definition;
}

test("test_new_questions_should_find_a_free_code_across_groups_without_mutating_the_definition", () => {
  const makeId = ids();
  const definition = createDefinition(makeId);
  definition.groups[0]!.questions.push(createQuestion(definition, makeId));
  const before = structuredClone(definition);
  const question = createQuestion(definition, makeId);
  assert.equal(question.code, "Q2");
  assert.notEqual(question.id, definition.groups[0]!.questions[0]!.id);
  assert.deepEqual(definition, before);
});

test("test_kind_changes_should_preserve_choice_identities_and_clear_inapplicable_settings", () => {
  const makeId = ids();
  const definition = createDefinition(makeId);
  const question = createQuestion(definition, makeId);
  const single = changeKind(question, "single_choice", makeId);
  single.choices[0]!.code = "01";
  single.mandatory = "strict";
  const multiple = changeKind(single, "multiple_choice", makeId);
  assert.deepEqual(multiple.subquestions, single.choices);
  assert.equal(multiple.id, question.id);
  const information = changeKind(multiple, "information", makeId);
  assert.equal(information.mandatory, "no");
  assert.deepEqual(information.choices, []);
  assert.equal(single.choices[0]!.code, "01");
});

test("test_reordering_should_preserve_node_identity_and_leave_the_previous_order_unchanged", () => {
  const definition = completeDefinition();
  const original = definition.groups[0]!.questions;
  const reordered = moveItem(original, 1, -1);
  assert.equal(reordered[0], original[1]);
  assert.equal(reordered[1], original[0]);
  assert.equal(original[0]!.code, "Q1");
  assert.deepEqual(moveItem(original, 0, -1), original);
});

test("test_native_definition_should_execute_all_eight_editor_types_in_the_existing_viewer", () => {
  const definition = completeDefinition();
  definition.welcome = "Bienvenue";
  const study = toViewer(definition, "saved-study");
  const runtime = createRuntime(study);
  assert.deepEqual(runtimeIssues(runtime, evaluateSnapshot(runtime, new Map())), []);
  assert.equal(study.surveyId, "saved-study");
  assert.deepEqual(
    study.questions.map((question) => question.sourceType),
    ["S", "T", "N", "D", "L", "M", "Y", "X"],
  );
  assert.equal(validate(runtime, evaluateSnapshot(runtime, new Map())).errors.size, 7);
  const answers = new Map([
    ["Q1", "Projet RustSurvey"],
    ["Q2", "Texte long"],
    ["Q3", "0"],
    ["Q4", "2026-11-11"],
    ["Q5", "A1"],
    ["Q6_A1", "Y"],
    ["Q7", "N"],
  ]);
  assert.equal(validate(runtime, evaluateSnapshot(runtime, answers)).errors.size, 0);
});

test("test_viewer_projection_should_preserve_lexical_codes_and_group_order", () => {
  const definition = completeDefinition();
  const group = definition.groups[0]!;
  group.questions[4]!.choices[0]!.code = "01";
  group.questions = moveItem(group.questions, 7, -1);
  const study = toViewer(definition);
  assert.equal(study.answers[0]!.code, "01");
  assert.equal(study.questions[6]!.sourceType, "X");
  assert.equal(study.questions[7]!.order, 7);
  assert.equal(study.questions[7]!.id, group.questions[7]!.id);
});
