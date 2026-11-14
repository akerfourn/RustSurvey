import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { decodeDefinition } from "../authoring/definition-codec.ts";
import { toViewer } from "../authoring/model.ts";
import { createRuntime, evaluateSnapshot } from "./engine.ts";
import {
  createPages,
  pageCheckpoint,
  type NavigationSettings,
  type SurveyPage,
} from "./navigation.ts";
import { validate } from "./validation.ts";

const source: unknown = JSON.parse(
  readFileSync(new URL("../../../examples/rules/collection.json", import.meta.url), "utf8"),
).definition;
const corpus = JSON.parse(
  readFileSync(new URL("../../../examples/rules/navigation.json", import.meta.url), "utf8"),
) as {
  cases: {
    name: string;
    format: NavigationSettings["format"];
    answers: Record<string, string>;
    pages: SurveyPage[];
    hideAll?: boolean;
    reverseGroups?: boolean;
  }[];
};

for (const fixture of corpus.cases) {
  test(`shared navigation: ${fixture.name}`, () => {
    const definition = decodeDefinition(structuredClone(source));
    if (fixture.hideAll === true) for (const group of definition.groups) group.relevance = "0";
    if (fixture.reverseGroups === true) definition.groups.reverse();
    const runtime = createRuntime(toViewer(definition, null));
    const snapshot = evaluateSnapshot(runtime, new Map(Object.entries(fixture.answers)));
    assert.deepEqual(snapshot.issues, []);
    for (const showProgress of [true, false]) {
      const settings = { format: fixture.format, showProgress };
      const pages = createPages(runtime, snapshot, settings);
      assert.deepEqual(pages, fixture.pages);
      for (const page of pages)
        assert.deepEqual(pageCheckpoint(page, "en", settings), {
          language: "en",
          groupId: page.groupId,
          pageId: page.id,
        });
    }
  });
}

test("test_question_navigation_should_validate_only_the_current_question_before_final_validation", () => {
  const runtime = createRuntime(toViewer(decodeDefinition(source), null));
  const snapshot = evaluateSnapshot(runtime, new Map([["GATE", "Y"]]));
  const current = validate(runtime, snapshot, undefined, new Set(["gate"]));
  assert.deepEqual([...current.errors], []);
  assert.deepEqual([...current.warnings], []);
  assert.ok(validate(runtime, snapshot).errors.has("age"));
});

test("test_legacy_cursors_should_keep_their_original_shape_and_empty_preview_behavior", () => {
  const definition = decodeDefinition(source);
  for (const group of definition.groups) group.relevance = "0";
  const runtime = createRuntime(toViewer(definition, null));
  const snapshot = evaluateSnapshot(runtime, new Map());
  assert.deepEqual(createPages(runtime, snapshot, null), []);
  assert.deepEqual(
    pageCheckpoint({ id: "g:main", groupId: "main", questionIds: ["gate"] }, "fr", null),
    { language: "fr", groupId: "main" },
  );
});
