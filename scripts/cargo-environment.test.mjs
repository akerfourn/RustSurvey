import assert from "node:assert/strict";
import { test } from "node:test";
import { cargoEnvironment } from "./cargo-environment.mjs";

test("test_windows_environment_should_expose_rustup_when_the_app_path_omits_cargo", () => {
  const environment = { Path: "C:\\Windows\\System32", KEEP: "unchanged" };
  const result = cargoEnvironment(environment, {
    platform: "win32",
    homeDirectory: "C:\\Users\\Developer",
  });
  assert.deepEqual(result, {
    KEEP: "unchanged",
    PATH: "C:\\Windows\\System32;C:\\Users\\Developer\\.cargo\\bin",
  });
  assert.deepEqual(environment, { Path: "C:\\Windows\\System32", KEEP: "unchanged" });
});

test("test_windows_environment_should_keep_npm_path_priority_without_duplicate_path_keys", () => {
  const result = cargoEnvironment(
    { Path: "old-app-path", PATH: "npm-tools;chosen-rust" },
    { platform: "win32", homeDirectory: "C:\\Users\\Developer" },
  );
  assert.equal(result.PATH, "npm-tools;chosen-rust;C:\\Users\\Developer\\.cargo\\bin");
  assert.equal(Object.keys(result).filter((key) => key.toLowerCase() === "path").length, 1);
});

test("test_environment_should_find_a_custom_cargo_home_before_the_default_installation", () => {
  const result = cargoEnvironment(
    { PATH: "/usr/bin", CARGO_HOME: "/tools/rust" },
    { platform: "linux", homeDirectory: "/home/developer" },
  );
  assert.equal(result.PATH, "/usr/bin:/tools/rust/bin:/home/developer/.cargo/bin");
  assert.equal(result.CARGO_HOME, "/tools/rust");
});

test("test_environment_should_handle_an_absent_path_without_searching_the_current_directory", () => {
  const result = cargoEnvironment({}, { platform: "linux", homeDirectory: "/home/developer" });
  assert.equal(result.PATH, "/home/developer/.cargo/bin");
});
