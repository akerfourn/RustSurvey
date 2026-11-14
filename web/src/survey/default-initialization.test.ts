import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { decodeDefinition } from "../authoring/definition-codec.ts";
import { toViewer, type Definition, type Question } from "../authoring/model.ts";
import { createRuntime, evaluateSnapshot, runtimeIssues } from "./engine.ts";
import { enterPage } from "./initialization.ts";
import { compileDefaultTemplate, renderDefaultTemplate } from "./default-template.ts";
import { validate } from "./validation.ts";

interface Step {
  language: string;
  state: "missing" | "null" | "empty" | "keep" | "existing";
  value?: string;
  seed?: Record<string, string>;
  expected: string | null;
  expectedWrite?: { present: boolean; value: string | null };
}
interface Case {
  name: string;
  question: string;
  row?: string;
  target?: string;
  special?: string;
  relevance?: string;
  filter?: string;
  sameDefault?: boolean;
  nativeSupported?: boolean;
  nativeExecutable?: boolean;
  seed?: Record<string, string>;
  values: Record<string, string>;
  steps: Step[];
}
const corpus = JSON.parse(
  readFileSync(new URL("../../../examples/rules/defaults-reference.json", import.meta.url), "utf8"),
) as { cases: Case[] };
const source: unknown = JSON.parse(
  readFileSync(new URL("../../../examples/rules/collection-forms.json", import.meta.url), "utf8"),
).definition;

function definitionFor(fixture: Case): Definition {
  const definition = decodeDefinition(source);
  definition.languages = ["fr", "en"];
  for (const group of definition.groups) {
    for (const question of group.questions) {
      question.relevance = "1";
      for (const row of question.subquestions) row.relevance = "1";
    }
  }
  const question = definition.groups[0]!.questions.find(
    (question) => question.code === fixture.question,
  )!;
  question.relevance = fixture.relevance ?? "1";
  if (fixture.filter !== undefined)
    question.attributes.push({ name: "array_filter", value: fixture.filter, language: null });
  if (fixture.special === "other") question.other = true;
  const row = question.subquestions.find((row) => row.code === fixture.row && row.scale === 0);
  question.defaults = {
    sameForAllLanguages: fixture.sameDefault ?? false,
    values: Object.entries(fixture.values).map(([language, value]) => ({
      coordinate:
        row === undefined ? { kind: "primary" } : { kind: "subquestion", id: row.id, scale: null },
      role: fixture.special === "other" ? "other" : "answer",
      language,
      value,
    })),
  };
  return definition;
}

for (const fixture of corpus.cases) {
  test(`test_default_initialization_should_match_reference: ${fixture.name}`, () => {
    const runtime = createRuntime(toViewer(definitionFor(fixture), null));
    if (fixture.nativeSupported === false) {
      assert.ok([...runtime.questions.values()].some((question) => question.issues.length > 0));
      return;
    }
    const scope = new Set(runtime.questions.keys());
    let answers = new Map(Object.entries(fixture.seed ?? {}));
    const target = fixture.target ?? fixture.question;
    for (const step of fixture.steps) {
      for (const [key, value] of Object.entries(step.seed ?? {})) answers.set(key, value);
      if (step.state === "missing" || step.state === "null") answers.delete(target);
      else if (step.state === "empty") answers.set(target, "");
      else if (step.state === "existing") answers.set(target, step.value!);
      const snapshot = evaluateSnapshot(runtime, answers, {
        language: step.language,
        initialize: scope,
      });
      assert.deepEqual(runtimeIssues(runtime, snapshot), []);
      if (fixture.nativeExecutable === false) {
        assert.ok(
          validate(runtime, snapshot).errors.size > 0,
          "the invalid source value must not pass native validation",
        );
        return;
      }
      const expected = step.expectedWrite?.value ?? step.expected;
      // An explicit NULL write takes precedence over a non-null engine session value.
      let persisted = expected;
      if (step.expectedWrite !== undefined) persisted = step.expectedWrite.value;
      assert.equal(snapshot.answers.get(target) ?? null, persisted);
      answers = snapshot.answers;
    }
  });
}

function selected(codes: string[]): Definition {
  const definition = definitionFor(corpus.cases[0]!);
  for (const group of definition.groups) {
    group.questions = group.questions.filter((question) => codes.includes(question.code));
    for (const question of group.questions) question.defaults = null;
  }
  return definition;
}
function setDefault(question: Question, value: string): void {
  question.defaults = {
    sameForAllLanguages: true,
    values: [{ coordinate: { kind: "primary" }, role: "answer", language: "fr", value }],
  };
}

test("test_page_entry_should_defer_future_defaults_and_preserve_clearing", () => {
  const definition = selected(["S", "T"]);
  const [first, second] = definition.groups[0]!.questions;
  setDefault(first!, "start");
  setDefault(second!, "value {S}");
  const runtime = createRuntime(toViewer(definition, null));
  const settings = { format: "question", showProgress: true } as const;
  const initial = enterPage(runtime, new Map(), settings, "fr");
  assert.equal(initial.snapshot.answers.get("S"), "start");
  assert.equal(initial.snapshot.answers.has("T"), false);
  const changed = new Map(initial.snapshot.answers);
  changed.set("S", "manual");
  const next = enterPage(runtime, changed, settings, "fr", initial.page?.id, `q:${second!.id}`);
  assert.equal(next.snapshot.answers.get("T"), "value manual");
  next.snapshot.answers.set("T", "");
  const resumed = enterPage(
    runtime,
    next.snapshot.answers,
    settings,
    "en",
    next.page?.id,
    next.page?.id,
  );
  assert.equal(resumed.snapshot.answers.get("T"), "");
});

test("test_page_entry_should_keep_supplied_filtered_values_when_a_source_default_activates_them", () => {
  const definition = selected(["M", "Q"]);
  const [source, target] = definition.groups[0]!.questions;
  source!.defaults = {
    sameForAllLanguages: true,
    values: [
      {
        coordinate: { kind: "subquestion", id: source!.subquestions[0]!.id, scale: null },
        role: "answer",
        language: "fr",
        value: "Y",
      },
    ],
  };
  target!.attributes.push({ name: "array_filter", value: "M", language: null });
  const runtime = createRuntime(toViewer(definition, null));
  const result = enterPage(
    runtime,
    new Map([
      ["Q_A1", "saved detail"],
      ["Q_A2", "inactive"],
    ]),
    null,
    "fr",
  );
  assert.equal(result.snapshot.answers.get("Q_A1"), "saved detail");
  assert.equal(result.snapshot.answers.has("Q_A2"), false);
});

test("test_default_templates_should_keep_quoted_braces_and_bound_output", () => {
  const template = compileDefaultTemplate("avant {if(1, '}', '{')} après");
  assert.equal(
    renderDefaultTemplate(template, () => null, { remaining: 1_000_000 }),
    "avant } après",
  );
  assert.throws(() => compileDefaultTemplate("{sum(1, 2)"), /non fermée/);
  assert.throws(
    () =>
      renderDefaultTemplate(compileDefaultTemplate("{S}"), () => "x".repeat(65_537), {
        remaining: 1_000_000,
      }),
    /trop long/,
  );
});

test("test_defaults_should_diagnose_unverified_inactive_references_and_cycles", () => {
  const definition = selected(["S", "T"]);
  const [first, second] = definition.groups[0]!.questions;
  setDefault(first!, "{S}");
  const runtime = createRuntime(toViewer(definition, null));
  const entry = enterPage(runtime, new Map(), null, "fr");
  assert.equal(entry.snapshot.answers.get("S"), "");
  second!.relevance = "S.NAOK == ''";
  const inactive = createRuntime(toViewer(definition, null));
  assert.ok(
    runtimeIssues(inactive, evaluateSnapshot(inactive, new Map())).some((issue) =>
      issue.includes("contexte de valeurs inactives"),
    ),
  );
  second!.relevance = "1";
  setDefault(first!, "{T}");
  setDefault(second!, "{S}");
  const cyclic = createRuntime(toViewer(definition, null));
  assert.ok(
    runtimeIssues(cyclic, evaluateSnapshot(cyclic, new Map())).some((issue) =>
      issue.includes("Cycle"),
    ),
  );
});

test("test_default_initialization_should_keep_an_explicit_exclusive_selection", () => {
  const definition = selected(["M"]);
  const question = definition.groups[0]!.questions[0]!;
  question.attributes.push({ name: "exclude_all_others", value: "A1;A2", language: null });
  question.defaults = {
    sameForAllLanguages: true,
    values: [
      {
        coordinate: { kind: "subquestion", id: question.subquestions[0]!.id, scale: null },
        role: "answer",
        language: "fr",
        value: "Y",
      },
    ],
  };
  const runtime = createRuntime(toViewer(definition, null));
  const answers = new Map([["M_A2", "Y"]]);
  const entry = enterPage(runtime, answers, null, "fr");
  assert.deepEqual(entry.snapshot.answers, answers);
  assert.deepEqual(runtimeIssues(runtime, entry.snapshot), []);
  const conflicting = enterPage(
    runtime,
    new Map([
      ["M_A1", "Y"],
      ["M_A2", "Y"],
    ]),
    null,
    "fr",
  );
  assert.ok(
    conflicting.snapshot.issues.some((issue) => issue.includes("Plusieurs lignes exclusives")),
  );
});
