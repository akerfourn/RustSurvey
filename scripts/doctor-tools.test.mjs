import assert from "node:assert/strict";
import { test } from "node:test";
import { probeTools } from "./doctor-tools.mjs";

test("test_windows_doctor_should_run_npm_through_cmd_and_find_msvc_outside_the_inherited_path", () => {
  const calls = [];
  const compiler = "C:\\Visual Studio\\VC\\Tools\\MSVC\\14.44\\bin\\Hostx64\\x64\\cl.exe";
  const checks = probeTools({
    platform: "win32",
    arch: "x64",
    environment: { "ProgramFiles(x86)": "C:\\Programs", ComSpec: "C:\\Windows\\cmd.exe" },
    run(command, args) {
      calls.push({ command, args });
      if (command === "cl.exe") return { status: null, error: new Error("not on PATH") };
      if (command.endsWith("vswhere.exe")) return { status: 0, stdout: `${compiler}\r\n` };
      if (command === compiler)
        return { status: 0, stderr: "MSVC test version\r\n", stdout: "usage" };
      return { status: 0, stdout: "test version" };
    },
  });
  assert.ok(
    calls.some(
      (call) => call.command === "C:\\Windows\\cmd.exe" && call.args.at(-1) === "npm.cmd --version",
    ),
  );
  assert.ok(!calls.some((call) => call.command === "npm.cmd" || call.command === "cc"));
  assert.equal(checks.find((check) => check.label === "Compilateur C").output, "MSVC test version");
});

test("test_doctor_should_report_missing_required_tools_without_treating_locator_success_as_compiler_success", () => {
  const checks = probeTools({
    platform: "win32",
    arch: "x64",
    environment: { "ProgramFiles(x86)": "C:\\Programs" },
    run(command) {
      if (command.endsWith("vswhere.exe")) return { status: 0, stdout: "" };
      return { status: null, error: new Error("missing tool") };
    },
  });
  const compiler = checks.find((check) => check.label === "Compilateur C");
  assert.equal(compiler.output, null);
  assert.equal(compiler.required, true);
  assert.equal(checks.find((check) => check.label === "Docker").required, false);
});

test("test_posix_doctor_should_keep_native_executables_and_compiler_checks", () => {
  const calls = [];
  const checks = probeTools({
    platform: "linux",
    arch: "x64",
    environment: {},
    run(command, args) {
      calls.push({ command, args });
      return { status: 0, stdout: "version\nmore details" };
    },
  });
  assert.ok(calls.some((call) => call.command === "npm" && call.args[0] === "--version"));
  assert.ok(calls.some((call) => call.command === "cc"));
  assert.ok(checks.every((check) => check.output === "version"));
});
