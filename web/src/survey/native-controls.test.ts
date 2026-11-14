import assert from "node:assert/strict";
import { test } from "node:test";
import { createRuntime, evaluateSnapshot, runtimeIssues } from "./engine.ts";
import { matrixOptions } from "./matrix-options.ts";
import { makeQuestion, makeStudy, makeSubquestion } from "./test-fixtures.ts";
import { validate } from "./validation.ts";
import type { SurveyQuestion, SurveyStudy } from "./types.ts";

function settings(values: Record<string, string>): SurveyQuestion["attributes"] {
  return Object.entries(values).map(([name, value]) => ({ name, value, language: null }));
}
function source(type: string, values: Record<string, string> = {}): SurveyStudy {
  const study = makeStudy([
    makeQuestion("1", "GRID", { sourceType: type, attributes: settings(values) }),
  ]);
  study.subquestions = [makeSubquestion("11", "1", "A"), makeSubquestion("12", "1", "B")];
  study.answers = [
    { id: "21", questionId: "1", code: "01", order: 0, scale: 0, labels: [] },
    { id: "22", questionId: "1", code: "02", order: 1, scale: 0, labels: [] },
  ];
  return study;
}
function inspect(study: SurveyStudy, values: [string, string][] = []) {
  const runtime = createRuntime(study);
  const snapshot = evaluateSnapshot(runtime, new Map(values));
  return {
    runtime,
    snapshot,
    validation: validate(runtime, snapshot),
    issues: runtimeIssues(runtime, snapshot),
  };
}

test("test_fixed_arrays_should_validate_native_codes_when_each_domain_is_used", () => {
  for (const [type, valid, invalid] of [
    ["A", "5", "6"],
    ["B", "10", "11"],
    ["C", "U", "I"],
    ["E", "I", "Y"],
  ]) {
    const study = source(type ?? "");
    study.questions[0]!.mandatory = "strict";
    const result = inspect(study, [
      ["GRID_A", valid ?? ""],
      ["GRID_B", valid ?? ""],
    ]);
    assert.deepEqual(result.issues, []);
    assert.equal(result.validation.errors.size, 0);
    assert.match(
      inspect(study, [["GRID_A", invalid ?? ""]])
        .validation.errors.get("1")
        ?.join() ?? "",
      /Choix de réponse inconnu/,
    );
  }
});

test("test_dual_arrays_should_bind_independent_scales_when_codes_overlap", () => {
  const study = source("1");
  study.answers.push({ id: "23", questionId: "1", code: "other", order: 0, scale: 1, labels: [] });
  study.questions[0]!.mandatory = "strict";
  const result = inspect(study, [
    ["GRID_A_0", "01"],
    ["GRID_A_1", "other"],
    ["GRID_B_0", "02"],
    ["GRID_B_1", "other"],
  ]);
  assert.deepEqual(result.issues, []);
  assert.equal(result.validation.errors.size, 0);
  assert.deepEqual(result.runtime.questions.get("1")?.fields, [
    "GRID_A_0",
    "GRID_A_1",
    "GRID_B_0",
    "GRID_B_1",
  ]);
  assert.match(
    inspect(study, [["GRID_A_1", "01"]])
      .validation.errors.get("1")
      ?.join() ?? "",
    /Choix de réponse inconnu/,
  );
});

test("test_matrices_should_validate_numeric_steps_and_text_when_two_axes_are_present", () => {
  const study = source(":", {
    multiflexible_min: "0",
    multiflexible_max: "1",
    multiflexible_step: "0.5",
  });
  study.subquestions.push(makeSubquestion("13", "1", "C", 1));
  const valid = inspect(study, [
    ["GRID_A_C", "0"],
    ["GRID_B_C", "0.5"],
  ]);
  assert.deepEqual(valid.issues, []);
  assert.equal(valid.validation.errors.size, 0);
  assert.deepEqual(matrixOptions(study.questions[0]!, 2), ["0", "0.5", "1"]);
  assert.match(
    inspect(study, [["GRID_A_C", "0.3"]])
      .validation.errors.get("1")
      ?.join() ?? "",
    /pas de la matrice/,
  );
  study.questions[0]!.sourceType = ";";
  assert.equal(inspect(study, [["GRID_A_C", "texte libre"]]).validation.errors.size, 0);
});

test("test_matrices_should_block_invalid_or_excessive_option_ranges_before_rendering", () => {
  for (const attributes of [
    { multiflexible_step: "0" },
    { multiflexible_min: "10", multiflexible_max: "1" },
    { multiflexible_max: "10000000" },
    { multiflexible_min: "NaN" },
  ]) {
    const study = source(":", attributes);
    study.subquestions.push(makeSubquestion("13", "1", "C", 1));
    assert.ok(inspect(study).issues.length > 0);
  }
});

test("test_comments_should_remain_optional_and_leave_answer_counts_unchanged", () => {
  const single = source("O");
  single.subquestions = [];
  single.questions[0]!.mandatory = "strict";
  assert.equal(inspect(single, [["GRID", "01"]]).validation.errors.size, 0);
  const multiple = source("P", { min_answers: "2" });
  const result = inspect(multiple, [
    ["GRID_A", "Y"],
    ["GRID_Acomment", "Précision"],
  ]);
  assert.deepEqual(result.issues, []);
  assert.match(result.validation.errors.get("1")?.join() ?? "", /au moins 2/);
  assert.equal(result.snapshot.answers.get("GRID_Acomment"), "Précision");
  const cleared = evaluateSnapshot(
    result.runtime,
    new Map([...result.snapshot.answers, ["GRID_A", ""]]),
  );
  assert.equal(cleared.answers.has("GRID_Acomment"), false);
});

test("test_other_should_require_detail_and_clear_it_when_a_standard_option_is_selected", () => {
  const study = source("L", { other_numbers_only: "1" });
  study.subquestions = [];
  study.questions[0]!.other = true;
  const missing = inspect(study, [["GRID", "-oth-"]]);
  assert.deepEqual(missing.issues, []);
  assert.match(missing.validation.errors.get("1")?.join() ?? "", /Précisez/);
  const filled = inspect(study, [
    ["GRID", "-oth-"],
    ["GRID_other", "0"],
  ]);
  assert.equal(filled.validation.errors.size, 0);
  const cleared = evaluateSnapshot(
    filled.runtime,
    new Map([...filled.snapshot.answers, ["GRID", "01"]]),
  );
  assert.equal(cleared.answers.has("GRID_other"), false);
});

test("test_ranking_should_reject_duplicates_gaps_and_unknown_codes_when_order_is_entered", () => {
  const study = source("R");
  study.subquestions = [];
  assert.equal(
    inspect(study, [
      ["GRID_1", "02"],
      ["GRID_2", "01"],
    ]).validation.errors.size,
    0,
  );
  for (const values of [
    [
      ["GRID_1", "01"],
      ["GRID_2", "01"],
    ],
    [["GRID_2", "02"]],
    [["GRID_1", "bad"]],
  ]) {
    assert.ok(inspect(study, values as [string, string][]).validation.errors.size > 0);
  }
});

test("test_language_should_initialize_from_base_and_reject_unlisted_languages", () => {
  const study = source("I");
  study.questions[0]!.mandatory = "strict";
  study.subquestions = [];
  study.languages = ["fr", "en"];
  assert.equal(inspect(study).snapshot.answers.get("GRID"), "fr");
  assert.equal(inspect(study, [["GRID", "en"]]).validation.errors.size, 0);
  assert.match(
    inspect(study, [["GRID", "xx"]])
      .validation.errors.get("1")
      ?.join() ?? "",
    /Choix de réponse inconnu/,
  );
});

test("test_equations_should_recalculate_hidden_results_before_dependent_relevance", () => {
  const study = makeStudy([
    makeQuestion("3", "DETAIL", { relevance: "TOTAL >= 4" }),
    makeQuestion("2", "TOTAL", {
      sourceType: "*",
      attributes: settings({ equation: "sum(NUMBER, 2)", hidden: "1" }),
    }),
    makeQuestion("1", "NUMBER", { sourceType: "N" }),
  ]);
  const initial = inspect(study, [
    ["NUMBER", "2"],
    ["TOTAL", "999"],
    ["DETAIL", "draft"],
  ]);
  assert.deepEqual(initial.issues, []);
  assert.equal(initial.snapshot.visible.has("2"), false);
  assert.equal(initial.snapshot.answers.get("TOTAL"), "4");
  assert.equal(initial.snapshot.visible.has("3"), true);
  const changed = evaluateSnapshot(
    initial.runtime,
    new Map([...initial.snapshot.answers, ["NUMBER", "0"]]),
  );
  assert.equal(changed.answers.get("TOTAL"), "2");
  assert.equal(changed.answers.has("DETAIL"), false);
});

test("test_equations_should_diagnose_cycles_and_runtime_failures_instead_of_retaining_forged_results", () => {
  const cycle = makeStudy([
    makeQuestion("1", "VALUE", {
      sourceType: "*",
      attributes: settings({ equation: "VALUE + 1" }),
    }),
  ]);
  assert.match(inspect(cycle).issues.join(), /Cycle/);
  const failure = makeStudy([
    makeQuestion("1", "VALUE", { sourceType: "*", attributes: settings({ equation: "1 / 0" }) }),
  ]);
  const result = inspect(failure, [["VALUE", "forged"]]);
  assert.equal(result.snapshot.answers.has("VALUE"), false);
  assert.match(result.issues.join(), /Division par zéro/);
});

test("test_subquestion_relevance_should_clear_cells_before_followup_conditions_are_evaluated", () => {
  const study = source("A");
  study.questions[0]!.mandatory = "strict";
  study.questions.push(
    makeQuestion("2", "GATE"),
    makeQuestion("3", "DETAIL", { relevance: "GRID_A == '5'" }),
  );
  study.subquestions[0]!.relevance = "GATE == 'show'";
  const shown = inspect(study, [
    ["GATE", "show"],
    ["GRID_A", "5"],
    ["GRID_B", "1"],
    ["DETAIL", "draft"],
  ]);
  assert.deepEqual(shown.issues, []);
  const hidden = evaluateSnapshot(
    shown.runtime,
    new Map([...shown.snapshot.answers, ["GATE", "hide"]]),
  );
  assert.equal(hidden.answers.has("GRID_A"), false);
  assert.equal(hidden.answers.has("DETAIL"), false);
  assert.equal(hidden.visibleSubquestions.has("11"), false);
  assert.equal(validate(shown.runtime, hidden).errors.size, 0);
});

test("test_filters_should_chain_multiple_choice_selection_and_clear_excluded_rows", () => {
  const study = source("F", { array_filter: "SELECTED", array_filter_exclude: "EXCLUDED" });
  study.questions.push(
    makeQuestion("2", "SELECTED", { sourceType: "M" }),
    makeQuestion("3", "EXCLUDED", { sourceType: "M" }),
  );
  for (const parent of ["2", "3"])
    study.subquestions.push(
      makeSubquestion(`${parent}1`, parent, "A"),
      makeSubquestion(`${parent}2`, parent, "B"),
    );
  const result = inspect(study, [
    ["SELECTED_A", "Y"],
    ["SELECTED_B", "Y"],
    ["EXCLUDED_B", "Y"],
    ["GRID_A", "01"],
    ["GRID_B", "02"],
  ]);
  assert.deepEqual(result.issues, []);
  assert.equal(result.snapshot.answers.get("GRID_A"), "01");
  assert.equal(result.snapshot.answers.has("GRID_B"), false);
  assert.equal(result.snapshot.visibleSubquestions.has("12"), false);
});

test("test_filter_style_should_keep_disabled_rows_visible_without_requiring_or_preserving_their_values", () => {
  const study = source("F", { array_filter: "SELECTED", array_filter_style: "1" });
  study.questions[0]!.mandatory = "strict";
  study.questions.push(makeQuestion("2", "SELECTED", { sourceType: "M" }));
  study.subquestions.push(makeSubquestion("21", "2", "A"), makeSubquestion("22", "2", "B"));
  const result = inspect(study, [
    ["SELECTED_A", "Y"],
    ["GRID_A", "01"],
    ["GRID_B", "02"],
  ]);
  assert.equal(result.snapshot.visibleSubquestions.has("12"), true);
  assert.equal(result.snapshot.activeFields.has("GRID_B"), false);
  assert.equal(result.snapshot.answers.has("GRID_B"), false);
  assert.equal(result.validation.errors.size, 0);
});

test("test_sum_constraints_should_validate_numeric_cells_instead_of_comments", () => {
  const study = source("K", { equals_num_value: "10", min_num_value: "0", max_num_value: "10" });
  assert.equal(
    inspect(study, [
      ["GRID_A", "4"],
      ["GRID_B", "6"],
    ]).validation.errors.size,
    0,
  );
  assert.match(
    inspect(study, [
      ["GRID_A", "4"],
      ["GRID_B", "5"],
    ])
      .validation.errors.get("1")
      ?.join() ?? "",
    /somme/,
  );
});

test("test_sliders_should_validate_literal_bounds_steps_and_zero_values", () => {
  const study = source("K", {
    slider_layout: "1",
    slider_min: "0",
    slider_max: "10",
    slider_accuracy: "2",
    slider_reset: "1",
  });
  assert.deepEqual(inspect(study).issues, []);
  assert.equal(inspect(study, [["GRID_A", "0"]]).validation.errors.size, 0);
  assert.match(
    inspect(study, [["GRID_A", "3"]])
      .validation.errors.get("1")
      ?.join() ?? "",
    /pas du curseur/,
  );
});

test("test_unknown_zero_attributes_should_be_diagnosed_instead_of_assumed_disabled", () => {
  const study = source("S", { unknown_rule: "0" });
  study.subquestions = [];
  assert.match(inspect(study).issues.join(), /unknown_rule/);
});

test("test_themes_should_block_unimplemented_functional_variants_instead_of_assuming_core_rendering", () => {
  const study = source("S");
  study.subquestions = [];
  study.questions[0]!.theme = "browserdetect";
  assert.match(inspect(study).issues.join(), /Thème de question/);
});

test("test_numeric_matrix_inputs_should_remain_unbounded_when_no_explicit_limits_are_defined", () => {
  const study = source(":", { input_boxes: "1" });
  study.subquestions.push(makeSubquestion("13", "1", "C", 1));
  const result = inspect(study, [
    ["GRID_A_C", "-50.25"],
    ["GRID_B_C", "200"],
  ]);
  assert.deepEqual(result.issues, []);
  assert.equal(result.validation.errors.size, 0);
});

test("test_core_theme_names_should_be_accepted_when_they_match_the_native_question_type", () => {
  const study = source("L");
  study.subquestions = [];
  study.questions[0]!.theme = "listradio";
  assert.deepEqual(inspect(study).issues, []);
  study.questions[0]!.theme = "shortfreetext";
  assert.match(inspect(study).issues.join(), /Thème de question/);
});

test("test_matrix_axis_relevance_should_clear_only_inactive_columns_and_keep_mandatory_rows_valid", () => {
  const study = source(";");
  study.questions[0]!.mandatory = "strict";
  study.questions.push(makeQuestion("2", "GATE"));
  const column = makeSubquestion("13", "1", "C", 1);
  column.relevance = "GATE == 'show'";
  study.subquestions.push(column, makeSubquestion("14", "1", "D", 1));
  const result = inspect(study, [
    ["GATE", "hide"],
    ["GRID_A_C", "stale"],
    ["GRID_A_D", "one"],
    ["GRID_B_D", "two"],
  ]);
  assert.deepEqual(result.issues, []);
  assert.equal(result.snapshot.answers.has("GRID_A_C"), false);
  assert.equal(result.snapshot.visibleSubquestions.has("13"), false);
  assert.equal(result.validation.errors.size, 0);
});

test("test_dual_minimum_should_count_fully_answered_rows_instead_of_individual_scales", () => {
  const study = source("1", { min_answers: "1" });
  study.answers.push({ id: "23", questionId: "1", code: "high", order: 0, scale: 1, labels: [] });
  assert.match(
    inspect(study, [
      ["GRID_A_0", "01"],
      ["GRID_B_0", "01"],
    ])
      .validation.errors.get("1")
      ?.join() ?? "",
    /au moins 1/,
  );
  assert.equal(
    inspect(study, [
      ["GRID_A_0", "01"],
      ["GRID_A_1", "high"],
    ]).validation.errors.size,
    0,
  );
});

test("test_ranking_should_bound_the_product_of_positions_and_options_before_rendering", () => {
  const study = source("R");
  study.subquestions = [];
  study.answers = Array.from({ length: 50 }, (_, index) => ({
    id: String(index),
    questionId: "1",
    code: String(index),
    order: index,
    scale: 0,
    labels: [],
  }));
  assert.match(inspect(study).issues.join(), /Classement limité/);
});

test("test_filter_sources_should_be_diagnosed_when_they_are_unknown_or_create_cycles", () => {
  const study = source("M", { array_filter: "MISSING" });
  assert.match(
    inspect(study).issues.join(),
    /doit désigner une question à lignes multiples ou un classement/,
  );
  study.questions[0]!.attributes = settings({ array_filter: "GRID" });
  assert.match(inspect(study).issues.join(), /Cycle/);
});

test("test_ranking_should_allow_partial_mandatory_answers_when_count_constraints_are_defined", () => {
  const study = source("R", { max_answers: "1" });
  study.questions[0]!.mandatory = "strict";
  study.subquestions = [];
  assert.equal(inspect(study, [["GRID_1", "01"]]).validation.errors.size, 0);
  assert.match(inspect(study).validation.errors.get("1")?.join() ?? "", /obligatoire/);
  assert.match(
    inspect(study, [
      ["GRID_1", "01"],
      ["GRID_2", "02"],
    ])
      .validation.errors.get("1")
      ?.join() ?? "",
    /au plus 1/,
  );
  study.questions[0]!.attributes = settings({ max_subquestions: "1" });
  assert.deepEqual(inspect(study).runtime.questions.get("1")?.fields, ["GRID_1"]);
  assert.equal(inspect(study, [["GRID_1", "01"]]).validation.errors.size, 0);
});

test("test_numeric_bounds_should_use_slider_fallbacks_only_when_explicit_numeric_constraints_are_absent", () => {
  const study = source("K", { slider_min: "0", slider_max: "10" });
  assert.match(
    inspect(study, [["GRID_A", "-1"]])
      .validation.errors.get("1")
      ?.join() ?? "",
    /bornes numériques/,
  );
  study.questions[0]!.attributes.push({ name: "min_num_value_n", value: "-5", language: null });
  assert.equal(inspect(study, [["GRID_A", "-1"]]).validation.errors.size, 0);
});
