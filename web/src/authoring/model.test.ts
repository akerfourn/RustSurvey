import assert from "node:assert/strict";
import { test } from "node:test";
import { createRuntime, evaluateSnapshot, runtimeIssues } from "../survey/engine.ts";
import { validate } from "../survey/validation.ts";
import { setSetting, settingValue } from "./question-settings.ts";
import { changeBaseLanguage, parseLanguages, translateTarget } from "./translations.ts";
import {
  QUESTION_KINDS,
  changeKind,
  createDefinition,
  createQuestion,
  moveItem,
  toViewer,
  type Definition,
  type QuestionKind,
} from "./model.ts";

const SIMPLE_KINDS: readonly QuestionKind[] = [
  "short_text",
  "long_text",
  "number",
  "date",
  "single_choice",
  "multiple_choice",
  "yes_no",
  "information",
];

test("imported welcome inheritance should remain unknown while native defaults stay local", () => {
  const definition = createDefinition(ids());
  definition.welcome = "Synthetic welcome";
  definition.showWelcome = null;
  assert.equal(toViewer(definition).showWelcome, true);
  assert.equal(toViewer(definition, "imported", true).showWelcome, null);
  definition.showWelcome = false;
  assert.equal(toViewer(definition, "imported", true).showWelcome, false);
});

function ids(): () => string {
  let serial = 0;
  return () => `node-${++serial}`;
}

function completeDefinition(
  kinds: readonly QuestionKind[] = [...QUESTION_KINDS.keys()],
): Definition {
  const makeId = ids();
  const definition = createDefinition(makeId);
  const group = definition.groups[0]!;
  for (const kind of kinds) {
    const question = changeKind(createQuestion(definition, makeId), kind, makeId);
    if (kind !== "information" && kind !== "equation") question.mandatory = "strict";
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
  const definition = completeDefinition(SIMPLE_KINDS);
  const original = definition.groups[0]!.questions;
  const reordered = moveItem(original, 1, -1);
  assert.equal(reordered[0], original[1]);
  assert.equal(reordered[1], original[0]);
  assert.equal(original[0]!.code, "Q1");
  assert.deepEqual(moveItem(original, 0, -1), original);
});

test("test_native_definition_should_execute_all_eight_editor_types_in_the_existing_viewer", () => {
  const definition = completeDefinition(SIMPLE_KINDS);
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
  const definition = completeDefinition(SIMPLE_KINDS);
  const group = definition.groups[0]!;
  group.questions[4]!.choices[0]!.code = "01";
  group.questions = moveItem(group.questions, 7, -1);
  const study = toViewer(definition);
  assert.equal(study.answers[0]!.code, "01");
  assert.equal(study.questions[6]!.sourceType, "X");
  assert.equal(study.questions[7]!.order, 7);
  assert.equal(study.questions[7]!.id, group.questions[7]!.id);
});

test("test_editor_should_create_all_28_types_with_valid_axes_and_executable_defaults", () => {
  const definition = completeDefinition();
  const runtime = createRuntime(toViewer(definition));
  assert.equal(QUESTION_KINDS.size, 28);
  assert.deepEqual(runtimeIssues(runtime, evaluateSnapshot(runtime, new Map())), []);
  const allIds: string[] = [];
  for (const question of definition.groups[0]!.questions) {
    allIds.push(
      question.id,
      ...question.choices.map((item) => item.id),
      ...question.subquestions.map((item) => item.id),
    );
  }
  assert.equal(
    new Set(allIds).size,
    allIds.length,
    "all response axes need distinct stable identities",
  );
  assert.equal(evaluateSnapshot(runtime, new Map()).answers.get("Q28"), "0");
});

test("test_type_conversion_should_not_reuse_row_ids_as_answer_ids_in_two_dimensional_questions", () => {
  const makeId = ids();
  const multiple = changeKind(
    createQuestion(createDefinition(makeId), makeId),
    "multiple_choice",
    makeId,
  );
  const table = changeKind(multiple, "dual_scale", makeId);
  assert.deepEqual(table.subquestions, multiple.subquestions);
  const nodeIds = [...table.choices, ...table.subquestions].map((item) => item.id);
  assert.equal(nodeIds.length, new Set(nodeIds).size);
  assert.equal(table.choices.filter((item) => item.scale === 1).length, 2);
});

test("test_editor_settings_should_drive_calculations_and_sum_validation_in_the_shared_viewer", () => {
  const makeId = ids();
  const definition = createDefinition(makeId);
  let budget = changeKind(createQuestion(definition, makeId), "multiple_numbers", makeId);
  budget = setSetting(budget, "equals_num_value", "10");
  budget = setSetting(budget, "min_num_value_n", "0");
  budget = setSetting(budget, "slider_layout", "1");
  definition.groups[0]!.questions.push(budget);
  let total = changeKind(createQuestion(definition, makeId), "equation", makeId);
  total = setSetting(total, "equation", "sum(Q1_A1, Q1_A2)");
  definition.groups[0]!.questions.push(total);
  const runtime = createRuntime(toViewer(definition));
  const snapshot = evaluateSnapshot(
    runtime,
    new Map([
      ["Q1_A1", "3"],
      ["Q1_A2", "7"],
    ]),
  );
  assert.deepEqual(runtimeIssues(runtime, snapshot), []);
  assert.equal(snapshot.answers.get("Q2"), "10");
  assert.equal(validate(runtime, snapshot).errors.size, 0);
  const wrongSum = evaluateSnapshot(
    runtime,
    new Map([
      ["Q1_A1", "3"],
      ["Q1_A2", "8"],
    ]),
  );
  assert.match(validate(runtime, wrongSum).errors.get(budget.id)!.join(), /somme/);
});

test("test_editor_filters_should_hide_and_clear_rows_that_are_not_selected_in_the_source", () => {
  const makeId = ids();
  const definition = createDefinition(makeId);
  const multiple = changeKind(createQuestion(definition, makeId), "multiple_choice", makeId);
  definition.groups[0]!.questions.push(multiple);
  let followUp = changeKind(createQuestion(definition, makeId), "multiple_text", makeId);
  followUp = setSetting(followUp, "array_filter", "Q1");
  definition.groups[0]!.questions.push(followUp);
  const runtime = createRuntime(toViewer(definition));
  const snapshot = evaluateSnapshot(
    runtime,
    new Map([
      ["Q1_A1", "Y"],
      ["Q2_A1", "Visible"],
      ["Q2_A2", "Must be cleared"],
    ]),
  );
  assert.deepEqual(runtimeIssues(runtime, snapshot), []);
  assert.equal(snapshot.answers.get("Q2_A1"), "Visible");
  assert.equal(snapshot.activeFields.has("Q2_A2"), false);
  assert.equal(snapshot.answers.has("Q2_A2"), false);
});

test("test_setting_edits_should_preserve_localized_and_unknown_attributes_without_mutating_the_question", () => {
  const definition = createDefinition(ids());
  const question = createQuestion(definition, () => "q1");
  question.attributes = [
    { name: "prefix", value: "ancien", language: "" },
    { name: "prefix", value: "English", language: "en" },
    { name: "future_setting", value: "kept", language: null },
  ];
  const updated = setSetting(question, "prefix", "nouveau");
  assert.equal(settingValue(updated, "prefix"), "nouveau");
  assert.equal(settingValue(question, "prefix"), "ancien");
  assert.equal(updated.attributes.find((item) => item.language === "en")!.value, "English");
  assert.equal(updated.attributes.find((item) => item.name === "future_setting")!.value, "kept");
});

test("test_base_language_changes_should_preserve_text_translations_choice_codes_and_identities", () => {
  const makeId = ids();
  let definition = createDefinition(makeId);
  const question = changeKind(createQuestion(definition, makeId), "single_choice", makeId);
  question.choices[0]!.code = "01";
  definition.groups[0]!.questions.push(question);
  definition.languages = parseLanguages("fr, en; en", "fr");
  definition.translations.push({
    language: "en",
    title: "English study",
    welcome: "Welcome",
    end: "Thank you",
  });
  definition = translateTarget(definition, question.id, "en", "English question", "English help");
  definition = translateTarget(definition, question.choices[0]!.id, "en", "English choice", "");
  const before = structuredClone(definition);
  const english = changeBaseLanguage(definition, "en");
  assert.equal(english.title, "English study");
  assert.equal(english.groups[0]!.questions[0]!.title, "English question");
  assert.equal(english.groups[0]!.questions[0]!.choices[0]!.label, "English choice");
  assert.equal(english.groups[0]!.questions[0]!.choices[0]!.code, "01");
  assert.equal(english.groups[0]!.questions[0]!.choices[0]!.id, question.choices[0]!.id);
  const restored = changeBaseLanguage(english, "fr");
  assert.equal(restored.title, definition.title);
  assert.equal(restored.groups[0]!.questions[0]!.title, question.title);
  assert.equal(
    toViewer(restored).questions[0]!.labels.find((label) => label.language === "en")!.text,
    "English question",
  );
  assert.deepEqual(definition, before);
  assert.throws(() => parseLanguages("en, ../../tmp", "fr"), /codes de langue/);
});
