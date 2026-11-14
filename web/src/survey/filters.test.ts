import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { decodeDefinition } from "../authoring/definition-codec.ts";
import { toViewer, type Question, type QuestionKind } from "../authoring/model.ts";
import { createRuntime, evaluateSnapshot, runtimeIssues } from "./engine.ts";
import { validate } from "./validation.ts";
import { decodeInspection } from "../api/inspection.ts";

interface FilterCase {
  name: string;
  kind: QuestionKind;
  answers: Record<string, string>;
  expected: Record<string, string>;
  expectedSource?: Record<string, string>;
  other?: boolean;
  sourceRows?: string[];
  sourceHiddenRows?: string[];
  sourceAttributes?: Record<string, string>;
  middleRows?: string[];
  middleKind?: QuestionKind;
  expectedMiddle?: Record<string, string>;
  ancestorMode?: "exclude";
  mode?: "exclude";
  targetKind?: QuestionKind;
  targetRows?: string[];
  targetChoices?: string[];
  targetAnswers?: Record<string, string>;
  expectedChoices?: Record<string, boolean>;
  targetOther?: boolean;
  visibleOther?: boolean;
  style?: string;
}
const base = decodeDefinition(
  JSON.parse(
    readFileSync(new URL("../../../examples/rules/collection-forms.json", import.meta.url), "utf8"),
  ).definition,
);
const corpus = JSON.parse(
  readFileSync(new URL("../../../examples/rules/filters.json", import.meta.url), "utf8"),
) as { cases: FilterCase[] };

function question(kind: QuestionKind, code: string): Question {
  const question = structuredClone(
    base.groups[0]!.questions.find((question) => question.kind === kind)!,
  );
  question.id = code;
  question.code = code;
  question.mandatory = "no";
  question.relevance = "1";
  question.other = false;
  question.attributes = [];
  for (const axis of ["subquestions", "choices"] as const)
    for (const [index, item] of question[axis].entries()) {
      item.id = `${code}-${axis}-${index}`;
      item.relevance = "1";
    }
  if (["ranking", "single_choice"].includes(kind))
    for (const [index, choice] of question.choices.entries()) choice.code = `A${index + 1}`;
  return question;
}

function rows(question: Question, codes: string[] | undefined): void {
  if (codes === undefined) return;
  const prototype = question.subquestions[0]!;
  question.subquestions = codes.map((code, index) => ({
    ...structuredClone(prototype),
    id: `${question.code}-row-${index}`,
    code,
  }));
}

function fixture(caseData: FilterCase) {
  const definition = structuredClone(base);
  const source = question(caseData.kind, "SRC");
  rows(source, caseData.sourceRows);
  source.other = caseData.other === true;
  source.attributes = Object.entries(caseData.sourceAttributes ?? {}).map(([name, value]) => ({
    name,
    value,
    language: null,
  }));
  for (const row of source.subquestions)
    if (caseData.sourceHiddenRows?.includes(row.code) === true) row.relevance = "0";
  const questions = [source];
  let filterSource = "SRC";
  if (caseData.middleRows !== undefined || caseData.middleKind !== undefined) {
    const middle = question(caseData.middleKind ?? "multiple_text", "MID");
    rows(middle, caseData.middleRows);
    let name = "array_filter";
    if (caseData.ancestorMode === "exclude") name = "array_filter_exclude";
    middle.attributes = [{ name, value: "SRC", language: null }];
    questions.push(middle);
    filterSource = "MID";
  }
  const target = question(caseData.targetKind ?? "array_choice", "TARGET");
  rows(target, caseData.targetRows);
  if (caseData.targetChoices !== undefined) {
    const prototype = target.choices[0]!;
    target.choices = caseData.targetChoices.map((code, index) => ({
      ...structuredClone(prototype),
      id: `TARGET-choice-${index}`,
      code,
    }));
  }
  target.other = caseData.targetOther === true;
  let name = "array_filter";
  if (caseData.mode === "exclude") name = "array_filter_exclude";
  target.attributes = [
    { name, value: filterSource, language: null },
    { name: "array_filter_style", value: caseData.style ?? "0", language: null },
  ];
  questions.push(target);
  definition.groups[0]!.questions = questions;
  const answers = new Map(Object.entries(caseData.answers));
  const comments = target.kind === "multiple_choice_comment";
  for (const row of target.subquestions) {
    let value = "01";
    if (comments) value = "Y";
    answers.set(`TARGET_${row.code}`, value);
    if (comments) answers.set(`TARGET_${row.code}comment`, "note");
  }
  if (target.other) {
    answers.set("TARGET_other", "custom");
    if (comments) answers.set("TARGET_othercomment", "note");
  }
  if (caseData.targetAnswers !== undefined) {
    for (const key of [...answers.keys()]) if (key.startsWith("TARGET")) answers.delete(key);
    for (const [key, value] of Object.entries(caseData.targetAnswers)) answers.set(key, value);
  }
  return { definition, answers };
}

for (const caseData of corpus.cases) {
  test(`shared filters: ${caseData.name}`, () => {
    const { definition, answers } = fixture(caseData);
    const runtime = createRuntime(toViewer(definition, null));
    const snapshot = evaluateSnapshot(runtime, answers);
    assert.deepEqual(runtimeIssues(runtime, snapshot), []);
    const target = Object.fromEntries(
      [...snapshot.answers].filter(([key]) => key.startsWith("TARGET")),
    );
    assert.deepEqual(target, caseData.expected);
    if (caseData.expectedChoices !== undefined) {
      const allowed = snapshot.activeChoices.get("TARGET");
      assert.notEqual(allowed, undefined);
      const actual = Object.fromEntries(
        Object.keys(caseData.expectedChoices).map((code) => {
          let value = code;
          if (code === "other") value = "-oth-";
          return [code, allowed!.has(value)];
        }),
      );
      assert.deepEqual(actual, caseData.expectedChoices);
    }
    if (caseData.expectedSource !== undefined)
      assert.deepEqual(
        Object.fromEntries([...snapshot.answers].filter(([key]) => key.startsWith("SRC_"))),
        caseData.expectedSource,
      );
    if (caseData.visibleOther !== undefined)
      assert.equal(snapshot.visibleOther.has("TARGET"), caseData.visibleOther);
    if (caseData.expectedMiddle !== undefined)
      assert.deepEqual(
        Object.fromEntries([...snapshot.answers].filter(([key]) => key.startsWith("MID_"))),
        caseData.expectedMiddle,
      );
  });
}

test("test_filter_validation_should_reject_cycles_and_scalar_sources", () => {
  const { definition } = fixture({
    name: "cycle",
    kind: "multiple_text",
    answers: {},
    expected: {},
  });
  definition.groups[0]!.questions[0]!.attributes.push({
    name: "array_filter",
    value: "TARGET",
    language: null,
  });
  const cyclic = createRuntime(toViewer(definition, null));
  assert.ok(
    runtimeIssues(cyclic, evaluateSnapshot(cyclic, new Map())).some((issue) =>
      issue.toLowerCase().includes("cycle"),
    ),
  );
  const { definition: scalar } = fixture({
    name: "scalar",
    kind: "single_choice",
    answers: {},
    expected: {},
  });
  const runtime = createRuntime(toViewer(scalar, null));
  assert.ok(
    runtimeIssues(runtime, evaluateSnapshot(runtime, new Map())).some((issue) =>
      issue.includes("lignes multiples ou un classement"),
    ),
  );
});

test("test_filtered_ranking_should_require_only_available_positions", () => {
  for (const source of [{ SRC_A1: "Y" }, {}]) {
    const { definition, answers } = fixture({
      name: "mandatory ranking",
      kind: "multiple_choice",
      targetKind: "ranking",
      answers: source,
      targetAnswers: { TARGET_1: "A1" },
      expected: {},
    });
    definition.groups[0]!.questions.at(-1)!.mandatory = "strict";
    const runtime = createRuntime(toViewer(definition));
    const snapshot = evaluateSnapshot(runtime, answers);
    assert.deepEqual(runtimeIssues(runtime, snapshot), []);
    assert.equal(validate(runtime, snapshot).errors.size, 0);
  }
});

test("test_filtered_ranking_should_reject_unknown_duplicate_and_gapped_answers", () => {
  const cases: [Record<string, string>, RegExp][] = [
    [{ TARGET_1: "unknown" }, /Choix de réponse inconnu/],
    [{ TARGET_1: "A1", TARGET_2: "A1" }, /une seule position/],
    [{ TARGET_2: "A1" }, /position vide/],
  ];
  for (const [targetAnswers, error] of cases) {
    const { definition, answers } = fixture({
      name: "invalid ranking",
      kind: "multiple_choice",
      targetKind: "ranking",
      answers: { SRC_A1: "Y" },
      targetAnswers,
      expected: {},
    });
    const runtime = createRuntime(toViewer(definition));
    assert.match(runtimeIssues(runtime, evaluateSnapshot(runtime, answers)).join("\n"), error);
  }
});

test("test_radio_filter_should_recheck_mandatory_after_clearing_a_selection", () => {
  const { definition, answers } = fixture({
    name: "mandatory list",
    kind: "multiple_choice",
    targetKind: "single_choice",
    answers: { SRC_A1: "Y" },
    targetAnswers: { TARGET: "A2" },
    expected: {},
  });
  definition.groups[0]!.questions.at(-1)!.mandatory = "strict";
  const runtime = createRuntime(toViewer(definition));
  const snapshot = evaluateSnapshot(runtime, answers);
  assert.deepEqual(validate(runtime, snapshot).errors.get("TARGET"), [
    "Cette question est obligatoire.",
  ]);
});

test("test_imported_column_filter_should_report_the_reference_gap_without_applying_native_behavior", () => {
  const { definition, answers } = fixture({
    name: "column filter",
    kind: "multiple_choice",
    targetKind: "array_column",
    answers: { SRC_A1: "Y" },
    expected: {},
  });
  const native = createRuntime(toViewer(definition));
  assert.deepEqual(runtimeIssues(native, evaluateSnapshot(native, answers)), []);
  const study = toViewer(definition, null, true);
  const decoded = decodeInspection({
    metadata: { surveyId: null, baseLanguage: "fr", languages: ["fr"], titles: [] },
    study,
    report: {
      sourceSchema: 623,
      targetSchema: 623,
      errors: 0,
      warnings: 0,
      omittedDiagnostics: 0,
      checkedSections: [],
      sections: [],
      diagnostics: [],
    },
  }).study!;
  for (const imported of [study, decoded]) {
    const runtime = createRuntime(imported);
    const snapshot = evaluateSnapshot(runtime, answers);
    assert.ok(
      runtimeIssues(runtime, snapshot).some((issue) => issue.includes("n’applique pas ce filtre")),
    );
    assert.equal(snapshot.relevant.has("TARGET"), false);
  }
});
