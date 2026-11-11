import assert from "node:assert/strict";
import { test } from "node:test";
import { compileExpression } from "./expressions.ts";
function evaluate(source: string) {
  return compileExpression(source).evaluate(() => "");
}

test("test_expressions_should_accept_native_two_argument_if_and_round_precision", () => {
  assert.equal(evaluate("if(true, 'ok')"), "ok");
  assert.equal(evaluate("if(false, 1/0)"), "");
  assert.equal(evaluate("round(12.345,2)"), 12.35);
  assert.equal(evaluate("round(-12.345,2)"), -12.35);
  assert.equal(evaluate("round(123,-1)"), 120);
});
test("test_expressions_should_compare_strings_and_textual_operator_aliases", () => {
  assert.equal(evaluate("'b' gt 'a' and 2 le 3 and '01' ne '1'"), true);
  assert.equal(evaluate("min('z', 'a')"), "a");
  assert.equal(evaluate("max(2, 10)"), 10);
});
test("test_expressions_should_handle_unicode_text_and_empty_list_members", () => {
  assert.equal(evaluate("join(strtoupper('été'), '-', substr('😀été',1,2))"), "ÉTÉ-ét");
  assert.equal(evaluate("list('', 'A', '', 'B')"), "A, B");
  assert.equal(evaluate("implode('|', 'A', 'B')"), "A|B");
  assert.equal(evaluate("str_replace('x','ok','x x')"), "ok ok");
  assert.equal(evaluate("strrev('abc')"), "cba");
  assert.throws(() => evaluate("strrev('a😀b')"), /ASCII/);
  assert.equal(evaluate("trim('  hello  ')"), "hello");
});
test("test_expressions_should_distinguish_missing_substrings_from_zero_offsets", () => {
  assert.equal(evaluate("strpos('abc', 'a')"), 0);
  assert.equal(evaluate("strpos('abc', 'x')"), false);
  assert.equal(evaluate("stripos('Abc', 'a')"), 0);
  assert.equal(evaluate("strcasecmp('ABC','abc')"), 0);
});
test("test_expressions_should_support_math_and_reject_non_finite_nested_results", () => {
  assert.equal(evaluate("sqrt(pow(3,2) + pow(4,2))"), 5);
  assert.equal(evaluate("ceil(1.1) + floor(2.9)"), 4);
  assert.ok(Math.abs(Number(evaluate("sin(pi()/2)")) - 1) < 1e-12);
  assert.throws(() => evaluate("is_empty(sqrt(-1))"), /non fini/);
  assert.throws(() => evaluate("str_repeat('x',100000000)"), /trop grande/);
  assert.throws(() => evaluate("round(1,100)"), /Précision/);
});
test("test_expressions_should_check_types_and_uniqueness_without_executing_external_code", () => {
  assert.equal(evaluate("is_numeric('0')"), true);
  assert.equal(evaluate("is_numeric('bad')"), false);
  assert.equal(evaluate("unique('A','','B')"), true);
  assert.equal(evaluate("unique('A','A')"), false);
  assert.throws(() => evaluate("fetch('url')"), /non prise en charge/);
});

test("test_expression_results_should_be_bounded_before_string_replacement_allocation", () => {
  assert.throws(
    () => evaluate("str_replace('x',str_repeat('a',65536),str_repeat('x',65536))"),
    /trop long/,
  );
});

test("test_expressions_should_aggregate_conditionally_and_search_using_unicode_positions", () => {
  assert.equal(evaluate("countif('Y','Y','N','Y')"), 2);
  assert.equal(evaluate("countifop('>=',2,1,2,3)"), 2);
  assert.equal(evaluate("sumifop('>',1,1,2,3)"), 5);
  assert.equal(evaluate("strstr('abc','b')"), "bc");
  assert.equal(evaluate("strstr('abc','b',true)"), "a");
  assert.equal(evaluate("strpos('😀été', 'é', 2)"), 3);
});
