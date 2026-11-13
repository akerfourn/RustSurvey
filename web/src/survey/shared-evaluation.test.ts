import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { compileExpression, type Scalar } from "./expressions.ts";

interface EvaluationCase {
  name: string;
  expression: string;
  values?: Record<string, Scalar>;
  expected?: Scalar;
  error?: string;
}

const cases = JSON.parse(
  readFileSync(new URL("../../../examples/rules/evaluation.json", import.meta.url), "utf8"),
) as EvaluationCase[];

for (const fixture of cases) {
  test(`shared expression: ${fixture.name}`, () => {
    function evaluate(): Scalar {
      return compileExpression(fixture.expression).evaluate((name) => fixture.values?.[name] ?? "");
    }
    if (fixture.error !== undefined) {
      assert.throws(
        evaluate,
        (error: unknown) => error instanceof Error && error.message.includes(fixture.error!),
      );
      return;
    }
    const actual = evaluate();
    if (typeof actual === "number" && typeof fixture.expected === "number") {
      const tolerance = 1e-12 * Math.max(1, Math.abs(fixture.expected));
      assert.ok(
        Math.abs(actual - fixture.expected) <= tolerance,
        `${actual} != ${fixture.expected}`,
      );
      return;
    }
    assert.equal(actual, fixture.expected);
  });
}
