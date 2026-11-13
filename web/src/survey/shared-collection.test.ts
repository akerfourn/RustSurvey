import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { decodeDefinition } from "../authoring/definition-codec.ts";
import { toViewer } from "../authoring/model.ts";
import { createRuntime, evaluateSnapshot } from "./engine.ts";
import { validate } from "./validation.ts";

interface CollectionCase {
  name: string;
  answers: Record<string, string>;
  submit: boolean;
  expected?: Record<string, string>;
  groups?: string[];
  relevant?: string[];
  warnings?: string[];
  error?: string;
}
interface Corpus {
  definition: unknown;
  cases: CollectionCase[];
}
const corpus = JSON.parse(
  readFileSync(new URL("../../../examples/rules/collection.json", import.meta.url), "utf8"),
) as Corpus;
const runtime = createRuntime(toViewer(decodeDefinition(corpus.definition), null));

for (const fixture of corpus.cases) {
  test(`shared collection: ${fixture.name}`, () => {
    assert.deepEqual(runtime.issues, []);
    const snapshot = evaluateSnapshot(runtime, new Map(Object.entries(fixture.answers)));
    const validation = validate(runtime, snapshot);
    if (fixture.error !== undefined) {
      const messages = [...snapshot.issues, ...[...validation.errors.values()].flat()];
      assert.ok(
        messages.some((message) => message.includes(fixture.error!)),
        messages.join("; "),
      );
      return;
    }
    assert.deepEqual(snapshot.issues, []);
    if (fixture.submit) assert.deepEqual([...validation.errors], []);
    assert.deepEqual(Object.fromEntries(snapshot.answers), fixture.expected);
    assert.deepEqual(
      snapshot.groups.map((group) => group.id),
      fixture.groups,
    );
    assert.deepEqual([...snapshot.relevant].sort(), fixture.relevant);
    if (fixture.submit) assert.deepEqual([...validation.warnings.keys()].sort(), fixture.warnings);
  });
}
