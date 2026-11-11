import assert from "node:assert/strict";
import { test } from "node:test";
import { createRuntime, evaluateSnapshot, runtimeIssues } from "./engine.ts";
import { MAX_QUESTION_CELLS, MAX_SURVEY_CELLS } from "./cells.ts";
import {
  makeCheckboxMatrix,
  makeChoiceArray,
  makeQuestion,
  makeStudy,
  makeSubquestion,
} from "./test-fixtures.ts";
import { validate } from "./validation.ts";

test("test_arrays_should_bind_each_subquestion_when_type_is_F_or_H", () => {
  for (const type of ["F", "H"] as const) {
    const runtime = createRuntime(makeChoiceArray(type));
    const array = runtime.questions.get("2");
    assert.deepEqual(array?.fields, ["GRID_before", "GRID_after"]);
    assert.deepEqual(
      array?.cells.map((cell) => [cell.kind, cell.rowId, cell.columnId]),
      [
        ["choice", "21", null],
        ["choice", "22", null],
      ],
    );
    assert.equal(
      runtime.fields.has("GRID"),
      false,
      "an array has cells rather than a scalar answer",
    );
    assert.deepEqual(runtimeIssues(runtime, evaluateSnapshot(runtime, new Map())), []);
  }
});

test("test_arrays_should_drive_conditions_and_clear_dependents_when_an_answer_changes", () => {
  const runtime = createRuntime(makeChoiceArray("H"));
  const visible = evaluateSnapshot(
    runtime,
    new Map([
      ["GATE", "Y"],
      ["GRID_before", "01"],
      ["GRID_after", "02"],
      ["DETAIL", "go"],
      ["FOLLOWUP", "draft"],
    ]),
  );
  assert.equal(visible.visible.has("3"), true);
  assert.equal(visible.visible.has("4"), true);
  const changed = evaluateSnapshot(runtime, new Map([...visible.answers, ["GRID_after", "01"]]));
  assert.equal(changed.visible.has("3"), false);
  assert.equal(changed.answers.has("DETAIL"), false);
  assert.equal(changed.answers.has("FOLLOWUP"), false);
  const hidden = evaluateSnapshot(runtime, new Map([...visible.answers, ["GATE", "N"]]));
  assert.deepEqual([...hidden.answers], [["GATE", "N"]]);
});

test("test_arrays_should_preserve_codes_and_validate_every_cell_when_answers_are_supplied", () => {
  const study = makeChoiceArray("F");
  const runtime = createRuntime(study);
  const empty = evaluateSnapshot(runtime, new Map([["GATE", "Y"]]));
  assert.equal(validate(runtime, empty).errors.size, 0, "optional cells can remain blank");
  const valid = evaluateSnapshot(
    runtime,
    new Map([
      ["GATE", "Y"],
      ["GRID_before", "01"],
      ["GRID_after", "02"],
    ]),
  );
  assert.equal(valid.answers.get("GRID_before"), "01");
  assert.equal(validate(runtime, valid).errors.size, 0);
  const unknown = evaluateSnapshot(
    runtime,
    new Map([
      ["GATE", "Y"],
      ["GRID_before", "1"],
    ]),
  );
  assert.match(
    validate(runtime, unknown).errors.get("2")?.join() ?? "",
    /Choix de réponse inconnu/,
  );
});

test("test_arrays_should_require_all_cells_when_question_is_strict_mandatory", () => {
  const study = makeChoiceArray("H");
  const array = study.questions.find((question) => question.id === "2");
  assert.ok(array);
  array.mandatory = "strict";
  const runtime = createRuntime(study);
  assert.match(
    validate(
      runtime,
      evaluateSnapshot(
        runtime,
        new Map([
          ["GATE", "Y"],
          ["GRID_before", "01"],
        ]),
      ),
    )
      .errors.get("2")
      ?.join() ?? "",
    /obligatoire/,
  );
  assert.equal(
    validate(
      runtime,
      evaluateSnapshot(
        runtime,
        new Map([
          ["GATE", "Y"],
          ["GRID_before", "01"],
          ["GRID_after", "02"],
        ]),
      ),
    ).errors.size,
    0,
  );
});

test("test_matrix_should_create_two_axis_cells_when_checkbox_layout_is_enabled", () => {
  const runtime = createRuntime(makeCheckboxMatrix());
  assert.deepEqual(runtime.questions.get("2")?.fields, [
    "MATRIX_R1_C1",
    "MATRIX_R1_C2",
    "MATRIX_R10_C1",
    "MATRIX_R10_C2",
  ]);
  assert.deepEqual(runtime.questions.get("2")?.cells[3], {
    key: "MATRIX_R10_C2",
    kind: "checkbox",
    role: "answer",
    scale: 0,
    rowId: "22",
    columnId: "24",
  });
  assert.deepEqual(runtimeIssues(runtime, evaluateSnapshot(runtime, new Map())), []);
});

test("test_matrix_should_distinguish_empty_checked_and_unchecked_when_values_change", () => {
  const runtime = createRuntime(makeCheckboxMatrix());
  const empty = evaluateSnapshot(runtime, new Map([["GATE", "Y"]]));
  assert.equal(empty.answers.has("MATRIX_R1_C1"), false);
  assert.equal(empty.visible.has("3"), false);
  const checked = evaluateSnapshot(
    runtime,
    new Map([...empty.answers, ["MATRIX_R1_C1", "1"], ["MATRIX_R1_C2", "0"], ["DETAIL", "draft"]]),
  );
  assert.equal(checked.visible.has("3"), true);
  assert.equal(checked.answers.get("MATRIX_R1_C2"), "0");
  const unchecked = evaluateSnapshot(runtime, new Map([...checked.answers, ["MATRIX_R1_C1", "0"]]));
  assert.equal(unchecked.answers.get("MATRIX_R1_C1"), "0");
  assert.equal(unchecked.answers.has("MATRIX_R10_C1"), false);
  assert.equal(unchecked.answers.has("DETAIL"), false);
  const hidden = evaluateSnapshot(runtime, new Map([...unchecked.answers, ["GATE", "N"]]));
  assert.deepEqual([...hidden.answers], [["GATE", "N"]]);
});

test("test_matrix_should_allow_blank_optional_cells_and_reject_invalid_boolean_codes", () => {
  const runtime = createRuntime(makeCheckboxMatrix());
  assert.equal(
    validate(runtime, evaluateSnapshot(runtime, new Map([["GATE", "Y"]]))).errors.size,
    0,
  );
  const invalid = evaluateSnapshot(
    runtime,
    new Map([
      ["GATE", "Y"],
      ["MATRIX_R1_C1", "Y"],
    ]),
  );
  assert.match(
    validate(runtime, invalid).errors.get("2")?.join() ?? "",
    /Choix de réponse inconnu/,
  );
});

test("test_matrix_should_require_one_checked_cell_per_row_when_mandatory_is_strict", () => {
  const study = makeCheckboxMatrix();
  const matrix = study.questions.find((question) => question.id === "2");
  assert.ok(matrix);
  matrix.mandatory = "strict";
  const runtime = createRuntime(study);
  const missingRow = evaluateSnapshot(
    runtime,
    new Map([
      ["GATE", "Y"],
      ["MATRIX_R1_C1", "1"],
      ["MATRIX_R10_C1", "0"],
      ["MATRIX_R10_C2", "0"],
    ]),
  );
  assert.match(validate(runtime, missingRow).errors.get("2")?.join() ?? "", /obligatoire/);
  const complete = evaluateSnapshot(
    runtime,
    new Map([...missingRow.answers, ["MATRIX_R10_C2", "1"]]),
  );
  assert.equal(validate(runtime, complete).errors.size, 0);
});

test("test_matrix_should_count_checked_cells_when_min_answers_is_set", () => {
  const study = makeCheckboxMatrix();
  study.questions[1]?.attributes.push({ name: "min_answers", value: "1", language: null });
  const runtime = createRuntime(study);
  const allZero = evaluateSnapshot(
    runtime,
    new Map([
      ["GATE", "Y"],
      ["MATRIX_R1_C1", "0"],
      ["MATRIX_R1_C2", "0"],
    ]),
  );
  assert.match(validate(runtime, allZero).errors.get("2")?.join() ?? "", /au moins 1/);
});

test("test_matrix_should_warn_without_blocking_when_a_soft_mandatory_row_is_empty", () => {
  const study = makeCheckboxMatrix();
  const matrix = study.questions.find((question) => question.id === "2");
  assert.ok(matrix);
  matrix.mandatory = "soft";
  const runtime = createRuntime(study);
  const snapshot = evaluateSnapshot(
    runtime,
    new Map([
      ["GATE", "Y"],
      ["MATRIX_R1_C1", "1"],
    ]),
  );
  const result = validate(runtime, snapshot);
  assert.equal(result.errors.size, 0);
  assert.match(result.warnings.get("2")?.join() ?? "", /recommandé/);
});

test("test_arrays_should_bound_rendered_choices_when_a_table_would_have_too_many_inputs", () => {
  const study = makeChoiceArray("F");
  study.subquestions = Array.from({ length: MAX_QUESTION_CELLS }, (_, index) =>
    makeSubquestion(String(index + 100), "2", `R${index}`),
  );
  const runtime = createRuntime(study);
  assert.match(runtime.questions.get("2")?.issues.join() ?? "", /limité à 2000 choix affichés/);
});

test("test_matrix_should_accept_numeric_mode_and_diagnose_missing_axes_and_ambiguous_codes", () => {
  const missingAxis = makeCheckboxMatrix();
  missingAxis.subquestions = missingAxis.subquestions.filter((item) => item.scale === 0);
  assert.match(
    createRuntime(missingAxis).questions.get("2")?.issues.join() ?? "",
    /axe de lignes et un axe de colonnes/,
  );
  const numeric = makeCheckboxMatrix();
  numeric.questions[1]?.attributes.splice(0);
  assert.deepEqual(createRuntime(numeric).questions.get("2")?.issues, []);
  const ambiguous = makeCheckboxMatrix();
  ambiguous.subquestions = [
    makeSubquestion("21", "2", "a_b"),
    makeSubquestion("22", "2", "a"),
    makeSubquestion("23", "2", "c", 1),
    makeSubquestion("24", "2", "b_c", 1),
  ];
  assert.match(createRuntime(ambiguous).issues.join(), /Cellule ambiguë/);
});

test("test_matrix_should_bound_cell_expansion_when_axes_are_large", () => {
  const study = makeCheckboxMatrix();
  study.subquestions = Array.from({ length: MAX_QUESTION_CELLS + 1 }, (_, index) =>
    makeSubquestion(String(index + 100), "2", `R${index}`),
  );
  study.subquestions.push(makeSubquestion("99", "2", "C", 1));
  const matrix = createRuntime(study).questions.get("2");
  assert.equal(matrix?.cells.length, 0);
  assert.match(matrix?.issues.join() ?? "", /limitée à 2000 cellules/);
});

test("test_runtime_should_bound_total_cells_when_many_matrices_are_imported", () => {
  const questions = Array.from({ length: MAX_SURVEY_CELLS / 1000 + 1 }, (_, index) =>
    makeQuestion(String(index + 1), `M${index}`, {
      sourceType: ":",
      attributes: [{ name: "multiflexible_checkbox", value: "1", language: null }],
    }),
  );
  const study = makeStudy(questions);
  for (const question of questions) {
    for (let index = 0; index < 50; index += 1)
      study.subquestions.push(
        makeSubquestion(String(Number(question.id) * 1000 + index), question.id, `R${index}`),
      );
    for (let index = 0; index < 20; index += 1)
      study.subquestions.push(
        makeSubquestion(
          String(Number(question.id) * 1000 + 100 + index),
          question.id,
          `C${index}`,
          1,
        ),
      );
  }
  const runtime = createRuntime(study);
  assert.equal(runtime.questions.size, 0);
  assert.match(runtime.issues.join(), /limité à 20000 cellules/);
});
