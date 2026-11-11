import assert from "node:assert/strict";
import { test } from "node:test";
import { displayColumns } from "./attributes.ts";
import { createRuntime, evaluateSnapshot, runtimeIssues } from "./engine.ts";
import { makeQuestion, makeStudy } from "./test-fixtures.ts";
import { validate } from "./validation.ts";

test("test_attributes_should_preserve_numeric_validation_when_input_size_is_present", () => {
  const source = makeStudy([
    makeQuestion("1", "N", {
      sourceType: "N",
      attributes: [
        { name: "input_size", value: "20", language: null },
        { name: "min_num_value_n", value: "1", language: null },
        { name: "max_num_value_n", value: "3", language: null },
      ],
    }),
  ]);
  const runtime = createRuntime(source);
  assert.deepEqual(runtimeIssues(runtime, evaluateSnapshot(runtime, new Map())), []);
  assert.equal(validate(runtime, evaluateSnapshot(runtime, new Map([["N", "2"]]))).errors.size, 0);
  assert.match(
    validate(runtime, evaluateSnapshot(runtime, new Map([["N", "0"]])))
      .errors.get("1")
      ?.join() ?? "",
    /minimale est 1/,
  );
  assert.match(
    validate(runtime, evaluateSnapshot(runtime, new Map([["N", "4"]])))
      .errors.get("1")
      ?.join() ?? "",
    /maximale est 3/,
  );
});

test("test_attributes_should_allow_administration_and_inactive_timer_options_when_no_duration_is_set", () => {
  const runtime = createRuntime(
    makeStudy([
      makeQuestion("1", "INFO", {
        sourceType: "X",
        attributes: [
          { name: "save_as_default", value: "Y", language: null },
          { name: "clear_default", value: "1", language: null },
          { name: "time_limit", value: "", language: null },
          { name: "time_limit_action", value: "1", language: null },
          { name: "time_limit_warning_message", value: "Message", language: "fr" },
        ],
      }),
    ]),
  );
  assert.deepEqual(runtimeIssues(runtime, evaluateSnapshot(runtime, new Map())), []);
});

test("test_attributes_should_block_active_timers_and_unknown_execution_options", () => {
  const runtime = createRuntime(
    makeStudy([
      makeQuestion("1", "TIMED", {
        attributes: [
          { name: "time_limit", value: "30", language: null },
          { name: "time_limit_action", value: "1", language: null },
        ],
      }),
      makeQuestion("2", "UNKNOWN", {
        attributes: [{ name: "time_limit_custom_behavior", value: "1", language: null }],
      }),
    ]),
  );
  assert.match(runtime.questions.get("1")?.issues.join() ?? "", /Minuterie active/);
  assert.match(runtime.questions.get("2")?.issues.join() ?? "", /time_limit_custom_behavior/);
});

test("test_attributes_should_use_maximum_chars_when_a_unicode_text_limit_is_set", () => {
  const runtime = createRuntime(
    makeStudy([
      makeQuestion("1", "TEXT", {
        attributes: [{ name: "maximum_chars", value: "1", language: null }],
      }),
    ]),
  );
  assert.deepEqual(runtime.questions.get("1")?.issues, []);
  assert.equal(
    validate(runtime, evaluateSnapshot(runtime, new Map([["TEXT", "😀"]]))).errors.size,
    0,
  );
  assert.match(
    validate(runtime, evaluateSnapshot(runtime, new Map([["TEXT", "😀a"]])))
      .errors.get("1")
      ?.join() ?? "",
    /limitée à 1 caractères/,
  );
  const unlimited = createRuntime(
    makeStudy([
      makeQuestion("1", "TEXT", {
        attributes: [{ name: "maximum_chars", value: "0", language: null }],
      }),
    ]),
  );
  assert.equal(
    validate(unlimited, evaluateSnapshot(unlimited, new Map([["TEXT", "anything"]]))).errors.size,
    0,
  );
});

test("test_attributes_should_bound_columns_when_layout_values_are_untrusted", () => {
  for (const [value, expected] of [
    ["2", 2],
    ["3", 3],
    ["0", 1],
    ["-1", 1],
    ["1000000", 12],
    ["2; color:red", 1],
  ] as const) {
    assert.equal(
      displayColumns(
        makeQuestion("1", "Q", {
          attributes: [{ name: "display_columns", value, language: null }],
        }),
      ),
      expected,
    );
  }
});
