import assert from "node:assert/strict";
import { test } from "node:test";
import { compileExpression, type Scalar } from "./expressions.ts";

function evaluate(source: string, values: Record<string, Scalar> = {}): Scalar {
  return compileExpression(source).evaluate((name) => values[name] ?? "");
}

test("expressions should respect arithmetic precedence and conditions", () => {
  assert.equal(
    evaluate("(TEAM >= 2 and STACK == 'rust') || TEAM + 2 * 3 == 7", {
      TEAM: "1",
      STACK: "typescript",
    }),
    true,
  );
  assert.equal(evaluate("STACK == 'rust' and TEAM < 3", { TEAM: "2", STACK: "rust" }), true);
  assert.equal(evaluate("not (STACK != 'rust')", { STACK: "rust" }), true);
});

test("expressions should short circuit inactive branches", () => {
  assert.equal(evaluate("if(true, 'ok', 1 / 0)"), "ok");
  assert.equal(evaluate("false && 1 / 0"), false);
  assert.equal(evaluate("true || 1 / 0"), true);
});

test("expressions should handle empty answers, numeric zero and supported functions", () => {
  assert.equal(evaluate("is_empty(ANSWER)"), true);
  assert.equal(evaluate("is_empty(ANSWER)", { ANSWER: "0" }), false);
  assert.equal(evaluate("sum(TEAM, 2, 3)", { TEAM: "0" }), 5);
  assert.equal(evaluate("count('', '0', 'Y')"), 2);
  assert.equal(evaluate("round(-1.5)"), -2);
  assert.equal(evaluate("strlen('été')"), 3);
  assert.equal(evaluate("strlen('été 😀')"), 5);
});

test("expressions should preserve dependencies including NAOK names", () => {
  assert.deepEqual(
    [...compileExpression("A.NAOK == 1 && B_other == 'Y'").variables],
    ["A.NAOK", "B_other"],
  );
});

test("expressions should reject executable code, unknown functions and properties", () => {
  assert.throws(
    () => compileExpression("fetch('https://example.invalid')"),
    /Fonction non prise en charge/,
  );
  assert.throws(() => compileExpression("A; alert(1)"), /Syntaxe/);
  assert.throws(() => compileExpression("A = 1"), /Syntaxe/);
  assert.throws(() => compileExpression("'unfinished"), /non terminée/);
  assert.throws(() => compileExpression("if(1, 2)"), /arguments/);
});

test("expressions should enforce complexity and numeric limits", () => {
  assert.throws(() => compileExpression("1".repeat(4097)), /trop longue/);
  assert.throws(() => compileExpression("(".repeat(40) + "1" + ")".repeat(40)), /imbriquée/);
  assert.throws(() => evaluate("1 / 0"), /Division par zéro/);
  assert.throws(() => evaluate("1e308 * 1e308"), /non fini/);
  assert.throws(() => evaluate("1e308 * 1e308 > 2"), /non fini/);
});
