import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { compileExpression } from "../survey/expressions.ts";
import { createRuntime, evaluateSnapshot } from "../survey/engine.ts";
import { validate } from "../survey/validation.ts";
import { conditionExpression, visualCondition, variableCatalog } from "./conditions.ts";
import { createDefinition, createGroup, createQuestion, toViewer } from "./model.ts";

test("test_condition_parser_should_match_the_shared_server_corpus", () => {
  const corpus: { cases: { source: string; variables: string[] | null }[] } = JSON.parse(
    readFileSync(new URL("../../../examples/rules/conditions.json", import.meta.url), "utf8"),
  );
  for (const item of corpus.cases) {
    if (item.variables === null) assert.throws(() => compileExpression(item.source), item.source);
    else
      assert.deepEqual(
        [...compileExpression(item.source).variables].sort(),
        [...item.variables].sort(),
        item.source,
      );
  }
});
test("test_visual_conditions_should_preserve_quotes_lexical_codes_and_and_or_semantics", () => {
  for (const source of [
    "CODE == '01' && NAME != 'l\\'été'",
    "AGE >= 18 || AGE < 2",
    "is_empty(NAME)",
    "!is_empty(NAME)",
  ]) {
    const condition = visualCondition(source);
    assert.notEqual(condition, null);
    const rewritten = conditionExpression(condition!);
    const values = new Map([
      ["CODE", "01"],
      ["NAME", "l'été"],
      ["AGE", "20"],
    ]);
    const lookup = (name: string) => values.get(name) ?? null;
    assert.equal(
      compileExpression(source).evaluate(lookup),
      compileExpression(rewritten).evaluate(lookup),
    );
  }
  assert.equal(visualCondition("if(AGE > 18, 1, 0)"), null);
  assert.equal(visualCondition("AGE >= 18 && (A == 'Y' || B == 'Y')"), null);
  const lexical = visualCondition("CODE == '01'");
  assert.equal(conditionExpression(lexical!), "(CODE == '01')");
});
test("test_saved_group_and_question_conditions_should_drive_visibility_and_mandatory_validation", () => {
  let serial = 0;
  const id = () => `node-${++serial}`;
  const definition = createDefinition(id);
  const age = createQuestion(definition, id);
  age.code = "AGE";
  age.kind = "number";
  definition.groups[0]!.questions.push(age);
  const group = createGroup(id);
  definition.groups.push(group);
  group.relevance = "AGE >= 18";
  const detail = createQuestion(definition, id);
  detail.code = "DETAIL";
  detail.mandatory = "strict";
  detail.relevance = "AGE < 65";
  group.questions.push(detail);
  assert.deepEqual(
    variableCatalog(definition).map((item) => [item.name, item.kind]),
    [
      ["AGE", "number"],
      ["DETAIL", "text"],
    ],
  );
  const runtime = createRuntime(toViewer(definition));
  const shown = evaluateSnapshot(runtime, new Map([["AGE", "30"]]));
  assert.equal(shown.visible.has(detail.id), true);
  assert.equal(validate(runtime, shown).errors.has(detail.id), true);
  const hidden = evaluateSnapshot(
    runtime,
    new Map([
      ["AGE", "17"],
      ["DETAIL", "old value"],
    ]),
  );
  assert.equal(hidden.visible.has(detail.id), false);
  assert.equal(hidden.answers.has("DETAIL"), false);
  assert.equal(validate(runtime, hidden).errors.size, 0);
  assert.equal(evaluateSnapshot(runtime, new Map([["AGE", "70"]])).visible.has(detail.id), false);
});
