import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { decodeDefinition } from "../authoring/definition-codec.ts";
import { toViewer } from "../authoring/model.ts";
import { createRuntime, evaluateSnapshot, runtimeIssues } from "./engine.ts";
import { validate } from "./validation.ts";

interface Case {
  name: string;
  base?: boolean;
  submit: boolean;
  answers?: Record<string, string>;
  expected?: Record<string, string>;
  removeExpected?: string[];
  attributes?: Record<string, Record<string, string>>;
  subquestionRelevance?: Record<string, Record<string, string>>;
  error?: string;
}
interface Corpus {
  definition: unknown;
  baseline: { answers: Record<string, string>; expected: Record<string, string> };
  cases: Case[];
}
const corpus = JSON.parse(
  readFileSync(new URL("../../../examples/rules/collection-forms.json", import.meta.url), "utf8"),
) as Corpus;

for (const fixture of corpus.cases) {
  test(`shared forms: ${fixture.name}`, () => {
    const definition = decodeDefinition(structuredClone(corpus.definition));
    for (const question of definition.groups.flatMap((group) => group.questions)) {
      const settings = fixture.attributes?.[question.code];
      if (settings !== undefined) {
        question.attributes = question.attributes.filter((item) => !(item.name in settings));
        for (const [name, value] of Object.entries(settings))
          question.attributes.push({ name, value, language: null });
      }
      const conditions = fixture.subquestionRelevance?.[question.code];
      for (const row of question.subquestions)
        if (conditions?.[row.code] !== undefined) row.relevance = conditions[row.code]!;
    }
    let answers = { ...corpus.baseline.answers, ...fixture.answers };
    let expected = { ...corpus.baseline.expected, ...fixture.expected };
    if (fixture.base === false) {
      answers = { ...fixture.answers };
      expected = { ...fixture.expected };
    }
    for (const key of fixture.removeExpected ?? []) delete expected[key];
    const runtime = createRuntime(toViewer(definition, null));
    const snapshot = evaluateSnapshot(runtime, new Map(Object.entries(answers)));
    const validation = validate(runtime, snapshot);
    assert.deepEqual(runtimeIssues(runtime, snapshot), []);
    if (fixture.error !== undefined) {
      const errors = [...validation.errors.values()].flat();
      assert.ok(
        errors.some((message) => message.includes(fixture.error!)),
        errors.join("; "),
      );
    } else {
      if (fixture.submit) assert.deepEqual([...validation.errors], []);
      assert.deepEqual(Object.fromEntries(snapshot.answers), expected);
    }
  });
}

test("test_slider_should_refuse_a_finite_step_that_overflows_its_progression", () => {
  const definition = decodeDefinition(structuredClone(corpus.definition));
  const question = definition.groups[0]!.questions.find((question) => question.code === "K")!;
  question.attributes.find((attribute) => attribute.name === "slider_accuracy")!.value = "1e-308";
  const runtime = createRuntime(toViewer(definition, null));
  const snapshot = evaluateSnapshot(runtime, new Map());
  assert.ok(
    runtimeIssues(runtime, snapshot).some((message) =>
      message.includes("Intervalle de curseur invalide"),
    ),
  );
});

test("test_filters_should_match_selection_cells_without_interpreting_comment_fields_as_choices", () => {
  const definition = decodeDefinition(structuredClone(corpus.definition));
  const question = definition.groups[0]!.questions.find((question) => question.code === "F")!;
  question.subquestions[1]!.code = "A1comment";
  question.attributes.push({ name: "array_filter", value: "P", language: null });
  const answers = new Map(Object.entries(corpus.baseline.answers));
  answers.delete("F_A2");
  answers.set("F_A1comment", "02");
  const runtime = createRuntime(toViewer(definition, null));
  const snapshot = evaluateSnapshot(runtime, answers);
  assert.deepEqual(runtimeIssues(runtime, snapshot), []);
  assert.equal(snapshot.answers.get("F_A1comment"), "02");
});
