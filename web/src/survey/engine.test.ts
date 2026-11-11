import assert from "node:assert/strict";
import { test } from "node:test";
import { createRuntime, evaluateSnapshot, runtimeIssues } from "./engine.ts";
import { pipeText } from "./text.ts";
import type { SurveyQuestion, SurveyStudy } from "./types.ts";
import { validate } from "./validation.ts";

function question(
  id: string,
  code: string,
  overrides: Partial<SurveyQuestion> = {},
): SurveyQuestion {
  return {
    id,
    groupId: "10",
    code,
    sourceType: "S",
    family: "text",
    mandatory: "no",
    order: 0,
    relevance: "1",
    randomizationGroup: "",
    validation: "",
    other: false,
    labels: [],
    attributes: [],
    ...overrides,
  };
}

function study(questions: SurveyQuestion[]): SurveyStudy {
  return {
    surveyId: "1",
    baseLanguage: "fr",
    languages: ["fr"],
    titles: [],
    showWelcome: false,
    messages: [],
    groups: [
      { id: "10", order: 0, relevance: "1", randomizationGroup: "", labels: [] },
      { id: "20", order: 1, relevance: "STACK == 'rust'", randomizationGroup: "", labels: [] },
    ],
    questions,
    subquestions: [],
    answers: [],
  };
}

test("engine should hide dependent groups and erase their answers when the controlling answer changes", () => {
  const runtime = createRuntime(
    study([
      question("1", "STACK"),
      question("2", "DETAIL", { groupId: "20", mandatory: "strict" }),
    ]),
  );
  const initial = evaluateSnapshot(
    runtime,
    new Map([
      ["STACK", "rust"],
      ["DETAIL", "private"],
    ]),
  );
  const updated = evaluateSnapshot(runtime, new Map([...initial.answers, ["STACK", "typescript"]]));
  assert.equal(initial.visible.has("2"), true);
  assert.equal(updated.visible.has("2"), false);
  assert.equal(updated.answers.has("DETAIL"), false);
  assert.deepEqual(
    updated.groups.map((group) => group.id),
    ["10"],
  );
  assert.equal(validate(runtime, updated).errors.size, 0);
});

test("engine should cascade hidden answers in dependency order even when source order is reversed", () => {
  const runtime = createRuntime(
    study([
      question("3", "C", { relevance: "B == 'show'" }),
      question("2", "B", { relevance: "A == 'show'" }),
      question("1", "A"),
    ]),
  );
  const snapshot = evaluateSnapshot(
    runtime,
    new Map([
      ["A", "hide"],
      ["B", "show"],
      ["C", "old"],
    ]),
  );
  assert.deepEqual([...snapshot.answers], [["A", "hide"]]);
  assert.deepEqual([...snapshot.visible], ["1"]);
});

test("engine should use subquestion codes for checkbox relevance and validate a required multiple choice", () => {
  const source = study([
    question("1", "STACK"),
    question("2", "CHOICES", { sourceType: "M", mandatory: "strict" }),
    question("3", "DETAIL", { relevance: "CHOICES_other == 'Y'", mandatory: "strict" }),
  ]);
  source.subquestions = [
    {
      id: "21",
      parentId: "2",
      groupId: "10",
      code: "other",
      scale: 0,
      order: 0,
      relevance: "1",
      labels: [],
    },
  ];
  const runtime = createRuntime(source);
  const empty = evaluateSnapshot(runtime, new Map());
  const checked = evaluateSnapshot(runtime, new Map([["CHOICES_other", "Y"]]));
  assert.equal(empty.visible.has("3"), false);
  assert.equal(validate(runtime, empty).errors.has("2"), true);
  assert.equal(checked.visible.has("3"), true);
  assert.equal(validate(runtime, checked).errors.has("2"), false);
  assert.equal(validate(runtime, checked).errors.has("3"), true);
});

test("validation should accept zero as an answer and enforce integer, minimum and maximum constraints", () => {
  const numeric = question("1", "NUMBER", {
    sourceType: "N",
    mandatory: "strict",
    attributes: [
      { name: "num_value_int_only", value: "1", language: null },
      { name: "min_num_value_n", value: "0", language: null },
      { name: "max_num_value_n", value: "2", language: null },
    ],
  });
  const runtime = createRuntime(study([numeric]));
  assert.equal(
    validate(runtime, evaluateSnapshot(runtime, new Map([["NUMBER", "0"]]))).errors.size,
    0,
  );
  assert.match(
    validate(runtime, evaluateSnapshot(runtime, new Map([["NUMBER", "3"]])))
      .errors.get("1")
      ?.join() ?? "",
    /maximale est 2/,
  );
  assert.match(
    validate(runtime, evaluateSnapshot(runtime, new Map([["NUMBER", "1.5"]])))
      .errors.get("1")
      ?.join() ?? "",
    /entier/,
  );
  assert.match(
    validate(runtime, evaluateSnapshot(runtime, new Map([["NUMBER", "-1"]])))
      .errors.get("1")
      ?.join() ?? "",
    /minimale est 0/,
  );
});

test("validation should apply a zero upper bound rather than discard it as an inactive attribute", () => {
  const runtime = createRuntime(
    study([
      question("1", "N", {
        sourceType: "N",
        attributes: [{ name: "max_num_value_n", value: "0", language: null }],
      }),
    ]),
  );
  assert.match(
    validate(runtime, evaluateSnapshot(runtime, new Map([["N", "1"]])))
      .errors.get("1")
      ?.join() ?? "",
    /maximale est 0/,
  );
});

test("validation should distinguish soft mandatory warnings from blocking errors", () => {
  const runtime = createRuntime(study([question("1", "OPTIONAL", { mandatory: "soft" })]));
  const result = validate(runtime, evaluateSnapshot(runtime, new Map()));
  assert.equal(result.errors.size, 0);
  assert.match(result.warnings.get("1")?.join() ?? "", /recommandé/);
});

test("validation should reject an unknown choice and an invalid calendar date", () => {
  const runtime = createRuntime(
    study([
      question("1", "YESNO", { sourceType: "Y" }),
      question("2", "DATE", { sourceType: "D" }),
    ]),
  );
  const result = validate(
    runtime,
    evaluateSnapshot(
      runtime,
      new Map([
        ["YESNO", "injected"],
        ["DATE", "2025-02-30"],
      ]),
    ),
  );
  assert.match(result.errors.get("1")?.join() ?? "", /Choix de réponse inconnu/);
  assert.match(result.errors.get("2")?.join() ?? "", /date valide/);
});

test("engine should report cycles and unknown variables rather than execute ambiguous rules", () => {
  const runtime = createRuntime(
    study([
      question("1", "A", { relevance: "B == 1" }),
      question("2", "B", { relevance: "A == 1" }),
      question("3", "C", { relevance: "TOKEN == 'secret'" }),
    ]),
  );
  const snapshot = evaluateSnapshot(
    runtime,
    new Map([
      ["A", "1"],
      ["C", "old"],
    ]),
  );
  assert.match(runtimeIssues(runtime, snapshot).join(), /Cycle de pertinence/);
  assert.match(runtimeIssues(runtime, snapshot).join(), /Variable non prise en charge : TOKEN/);
  assert.equal(snapshot.answers.size, 0);
});

test("engine should diagnose unsupported types, attributes and subquestion rules", () => {
  const source = study([
    question("1", "FILE", { sourceType: "|" }),
    question("2", "M", {
      sourceType: "M",
      attributes: [{ name: "random_order", value: "1", language: null }],
    }),
  ]);
  source.subquestions = [
    {
      id: "3",
      parentId: "2",
      groupId: "10",
      code: "one",
      scale: 0,
      order: 0,
      relevance: "FILE == 'yes'",
      labels: [],
    },
  ];
  const runtime = createRuntime(source);
  assert.match(runtimeIssues(runtime, evaluateSnapshot(runtime, new Map())).join(), /Type \|/);
  assert.match(runtime.questions.get("2")?.issues.join() ?? "", /random_order/);
  assert.ok(runtime.questions.get("2")?.subquestionRules.size);
});

test("engine should safely address prototype-like question codes", () => {
  const runtime = createRuntime(
    study([
      question("1", "__proto__"),
      question("2", "FOLLOW", { relevance: "__proto__ == 'yes'" }),
    ]),
  );
  const snapshot = evaluateSnapshot(runtime, new Map([["__proto__", "yes"]]));
  assert.equal(snapshot.visible.has("2"), true);
  assert.equal(snapshot.answers.get("__proto__"), "yes");
});

test("engine should flag source randomization rather than silently use a fixed order", () => {
  const runtime = createRuntime(study([question("1", "RANDOM", { randomizationGroup: "pool" })]));
  assert.match(
    runtimeIssues(runtime, evaluateSnapshot(runtime, new Map())).join(),
    /Ordre aléatoire/,
  );
});

test("text substitution should resolve answers without executing unsupported expressions", () => {
  const result = pipeText(
    "Équipe {TEAM} : {if(TEAM > 1, 'plusieurs', 'une')} / {javascript(1)}",
    new Map([["TEAM", "2"]]),
    new Map([["TEAM", "1"]]),
  );
  assert.equal(result.text, "Équipe 2 : plusieurs / {javascript(1)}");
  assert.match(result.issues.join(), /Fonction non prise en charge/);
});

test("text diagnostics should inspect expressions before an unvisited page is rendered", async () => {
  const { textIssues } = await import("./text.ts");
  const fields = new Map([["KNOWN", "1"]]);
  assert.match(textIssues("{MISSING}", fields).join(), /Variable de texte/);
  assert.match(textIssues("{unsupported(KNOWN)}", fields).join(), /Fonction/);
  assert.deepEqual(textIssues("{if(KNOWN, 'ok')}", fields), []);
});

test("text evaluation failures should block displayed expressions without evaluating hidden pages", async () => {
  const { textExecutionIssues } = await import("./capabilities.ts");
  const { compileExpression } = await import("./expressions.ts");
  const runtime = createRuntime(
    study([question("1", "A"), question("2", "B", { relevance: "A == 'show'" })]),
  );
  const expression = {
    ownerId: "2",
    rowId: null,
    groupId: null,
    expressions: [compileExpression("1/0")],
  };
  assert.deepEqual(
    textExecutionIssues([expression], evaluateSnapshot(runtime, new Map([["A", "hide"]]))),
    [],
  );
  assert.match(
    textExecutionIssues([expression], evaluateSnapshot(runtime, new Map([["A", "show"]]))).join(),
    /Division par zéro/,
  );
});
