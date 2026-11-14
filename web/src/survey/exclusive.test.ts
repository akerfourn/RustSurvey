import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { decodeDefinition } from "../authoring/definition-codec.ts";
import { toViewer } from "../authoring/model.ts";
import { createRuntime, evaluateSnapshot } from "./engine.ts";
import { changeExclusiveAnswers, parseExclusive } from "./exclusive.ts";

interface Case {
  name: string;
  type: string;
  exclusive: string;
  auto?: boolean;
  other?: boolean;
  hideExclusive?: boolean;
  hideOthers?: boolean;
  withFilter?: boolean;
  answers: Record<string, string>;
  expected?: Record<string, string>;
  error?: string;
}
const corpus = JSON.parse(
  readFileSync(new URL("../../../examples/rules/exclusive.json", import.meta.url), "utf8"),
) as {
  manual: { type: string; value: string }[];
  cases: Case[];
};
const source: unknown = JSON.parse(
  readFileSync(new URL("../../../examples/rules/collection-forms.json", import.meta.url), "utf8"),
).definition;

test("test_exclusive_parser_should_reject_duplicate_attributes_without_choosing_a_value", () => {
  const runtime = runtimeFor({ name: "duplicate", type: "M", exclusive: "A2", answers: {} });
  const question = [...runtime.questions.values()][0]!;
  question.source.attributes.push({ name: "exclude_all_others", value: "A1", language: null });
  assert.throws(
    () => parseExclusive(question.source, question.subquestions),
    /déclarée plusieurs fois/,
  );
});

function runtimeFor(fixture: Case) {
  const definition = decodeDefinition(source);
  const question = definition.groups[0]!.questions.find(
    (question) => question.code === fixture.type,
  )!;
  question.mandatory = "no";
  question.relevance = "1";
  question.other = fixture.other === true;
  question.attributes = [
    { name: "exclude_all_others", value: fixture.exclusive, language: null },
    {
      name: "exclude_all_others_auto",
      value: String(Number(fixture.auto === true)),
      language: null,
    },
  ];
  for (const row of question.subquestions) {
    row.relevance = "1";
    if (
      (fixture.hideExclusive === true && row.code === "A2") ||
      (fixture.hideOthers === true && row.code === "A1")
    )
      row.relevance = "0";
  }
  const selected = [question];
  if (fixture.withFilter === true) {
    const filter = definition.groups[0]!.questions.find((question) => question.code === "F")!;
    filter.mandatory = "no";
    filter.relevance = "1";
    filter.attributes = [{ name: "array_filter", value: "M", language: null }];
    for (const row of filter.subquestions) row.relevance = "1";
    selected.push(filter);
  }
  definition.groups[0]!.questions = selected;
  return createRuntime(toViewer(definition, null));
}

const cases = [...corpus.cases];
for (const { type, value } of corpus.manual)
  cases.push({
    name: `manual ${type}`,
    type,
    exclusive: "A2",
    answers: { [`${type}_A1`]: value, [`${type}_A2`]: value },
    expected: { [`${type}_A2`]: value },
  });

for (const fixture of cases) {
  test(`shared exclusive: ${fixture.name}`, () => {
    const runtime = runtimeFor(fixture);
    assert.deepEqual(runtime.issues, []);
    const snapshot = evaluateSnapshot(runtime, new Map(Object.entries(fixture.answers)));
    if (fixture.error !== undefined) {
      assert.ok(
        snapshot.issues.some((issue) => issue.includes(fixture.error!)),
        snapshot.issues.join("; "),
      );
    } else {
      assert.deepEqual(snapshot.issues, []);
      assert.deepEqual(Object.fromEntries(snapshot.answers), fixture.expected);
      if (fixture.name.startsWith("manual ")) {
        assert.equal(snapshot.activeFields.has(`${fixture.type}_A1`), false);
        assert.equal(snapshot.visibleSubquestions.size, 2);
      }
    }
  });
}

test("test_exclusive_changes_should_use_the_last_clicked_exclusive_option_and_never_restore_cleared_answers", () => {
  const runtime = runtimeFor({ name: "click", type: "M", exclusive: "A1;A2", answers: {} });
  const initial = new Map([["M_A1", "Y"]]);
  const changed = changeExclusiveAnswers(runtime, initial, [["M_A2", "Y"]]);
  assert.deepEqual(Object.fromEntries(initial), { M_A1: "Y" });
  const selected = evaluateSnapshot(runtime, changed);
  assert.deepEqual(selected.issues, []);
  assert.deepEqual(Object.fromEntries(selected.answers), { M_A2: "Y" });
  const cleared = evaluateSnapshot(
    runtime,
    changeExclusiveAnswers(runtime, selected.answers, [["M_A2", ""]]),
  );
  assert.deepEqual(Object.fromEntries(cleared.answers), { M_A2: "" });
  assert.equal(cleared.activeFields.has("M_A1"), true);
});
