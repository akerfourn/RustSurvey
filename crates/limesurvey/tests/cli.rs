use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

use rustsurvey_limesurvey::{Limits, LssDocument};

const SOURCE: &[u8] = include_bytes!("fixtures/minimal-623.lss");
static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
fn test_group_settings_command_should_preserve_unknown_context_and_write_private_reports_only() {
    let directory = TestDirectory::new();
    let input = directory.file("input.lss");
    let output = directory.file("settings.json");
    fs::write(&input, SOURCE).unwrap();
    let result = cli(&[Path::new("group-settings"), &input, &output]);
    assert!(result.status.success(), "{:?}", result.stderr);
    assert!(result.stdout.is_empty() && result.stderr.is_empty());
    let bytes = fs::read(&output).unwrap();
    let report: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(report["complete"], false);
    assert!(
        report["settings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|setting| setting["name"] == "showwelcome" && setting["value"].is_null())
    );
    assert!(
        !cli(&[Path::new("group-settings"), &input, &output])
            .status
            .success()
    );
    assert_eq!(fs::read(&output).unwrap(), bytes);
    let context = directory.file("context.json");
    fs::write(
        &context,
        br#"{"schemaVersion":1,"private":"synthetic-private-context"}"#,
    )
    .unwrap();
    let rejected = directory.file("rejected.json");
    let result = cli(&[Path::new("group-settings"), &input, &rejected, &context]);
    assert!(!result.status.success());
    assert!(!rejected.exists());
    assert!(!String::from_utf8_lossy(&result.stderr).contains("synthetic-private-context"));
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "rustsurvey-lss-test-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated test directory");
        Self(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove isolated test directory");
    }
}

fn cli(arguments: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rustsurvey-limesurvey"))
        .args(arguments)
        .output()
        .expect("run local CLI")
}

#[test]
fn test_response_commands_should_preserve_source_values_without_overwriting_or_logging_them() {
    let directory = TestDirectory::new();
    let input = directory.file("responses.json");
    let xml = directory.file("responses.lsr");
    let decoded = directory.file("decoded.json");
    let vv = directory.file("responses.vv");
    let aliases = directory.file("aliases.json");
    let table = serde_json::json!({"languages":["fr"],"columns":["701X1X1","701X1X2","701X1X3"],"rows":[{"701X1X1":"  O'Neil `value`\r\n{question_not_shown}  ","701X1X2":"","701X1X3":null}]});
    fs::write(&input, serde_json::to_vec(&table).unwrap()).unwrap();
    fs::write(&aliases, br#"{"701X1X1":"Q1"}"#).unwrap();
    let result = cli(&[Path::new("export-responses"), &input, &xml]);
    assert!(result.status.success());
    assert!(result.stdout.is_empty() && result.stderr.is_empty());
    assert!(
        cli(&[Path::new("import-responses"), &xml, &decoded])
            .status
            .success()
    );
    let actual: serde_json::Value = serde_json::from_slice(&fs::read(decoded).unwrap()).unwrap();
    assert_eq!(actual, table);
    assert!(
        cli(&[Path::new("export-vv"), &input, &aliases, &vv])
            .status
            .success()
    );
    let exported = fs::read(&vv).unwrap();
    assert!(
        !cli(&[Path::new("export-vv"), &input, &aliases, &vv])
            .status
            .success()
    );
    assert_eq!(fs::read(vv).unwrap(), exported);
}

#[test]
fn test_inspect_should_emit_only_report_when_given_a_synthetic_survey() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/minimal-623.lss");

    let output = cli(&[Path::new("inspect"), &fixture]);

    assert!(output.status.success(), "{:?}", output.stderr);
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert_eq!(report["source_schema"], 623);
    assert_eq!(report["errors"], 0);
    assert_eq!(report["execution"], "not_implemented");
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("synthetic()")
    );
}

#[test]
fn test_export_should_preserve_existing_files_when_output_already_exists() {
    let directory = TestDirectory::new();
    let input = directory.file("input.lss");
    let output = directory.file("output.lss");
    fs::write(&input, SOURCE).unwrap();
    fs::write(&output, b"existing output").unwrap();

    let result = cli(&[Path::new("export-original"), &input, &output]);

    assert!(!result.status.success());
    assert_eq!(fs::read(&output).unwrap(), b"existing output");
    assert_eq!(fs::read(&input).unwrap(), SOURCE);
}

#[test]
fn test_export_should_preserve_all_bytes_when_destination_is_new() {
    let directory = TestDirectory::new();
    let input = directory.file("input.lss");
    let output = directory.file("output.lss");
    fs::write(&input, SOURCE).unwrap();

    let result = cli(&[Path::new("export-original"), &input, &output]);

    assert!(result.status.success(), "{:?}", result.stderr);
    assert_eq!(fs::read(&output).unwrap(), SOURCE);
}

#[test]
fn test_native_export_cli_should_generate_schema_623_without_overwriting_files() {
    let directory = TestDirectory::new();
    let input = directory.file("native.json");
    let output = directory.file("native.lss");
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../examples/rules/collection-forms.json"
    ))
    .unwrap();
    fs::write(&input, serde_json::to_vec(&corpus["definition"]).unwrap()).unwrap();
    let arguments = [
        Path::new("export-native"),
        &input,
        &output,
        Path::new("701"),
    ];
    let result = cli(&arguments);
    assert!(result.status.success(), "{:?}", result.stderr);
    let bytes = fs::read(&output).unwrap();
    let document = LssDocument::parse(&bytes, Limits::default()).unwrap();
    assert_eq!(document.source_model().unwrap().questions.len(), 28);
    assert!(!cli(&arguments).status.success());
    assert_eq!(fs::read(&output).unwrap(), bytes);
}

#[test]
fn test_native_export_cli_should_validate_optional_navigation_before_creating_the_output() {
    let directory = TestDirectory::new();
    let input = directory.file("native.json");
    let output = directory.file("native.lss");
    let navigation = directory.file("navigation.json");
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../examples/rules/collection-forms.json"
    ))
    .unwrap();
    fs::write(&input, serde_json::to_vec(&corpus["definition"]).unwrap()).unwrap();
    let arguments = [
        Path::new("export-native"),
        &input,
        &output,
        Path::new("701"),
        &navigation,
    ];
    fs::write(&navigation, br#"{"format":"question"}"#).unwrap();
    let invalid = cli(&arguments);
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("invalid navigation settings JSON"));
    assert!(!output.exists());
    fs::write(
        &navigation,
        br#"{"format":"question","showProgress":false}"#,
    )
    .unwrap();
    assert!(cli(&arguments).status.success());
    let document = LssDocument::parse(&fs::read(&output).unwrap(), Limits::default()).unwrap();
    assert_eq!(document.source_model().unwrap().questions.len(), 28);
}

#[test]
fn test_set_title_should_write_localized_change_when_output_is_new() {
    let directory = TestDirectory::new();
    let input = directory.file("input.lss");
    let output = directory.file("output.lss");
    fs::write(&input, SOURCE).unwrap();

    let result = cli(&[
        Path::new("set-title"),
        &input,
        &output,
        Path::new("fr"),
        Path::new("New <title>"),
    ]);

    assert!(result.status.success(), "{:?}", result.stderr);
    let xml = fs::read(&output).unwrap();
    let document = LssDocument::parse(&xml, Limits::default()).unwrap();
    assert_eq!(
        document.source_model().unwrap().titles[0].value.as_deref(),
        Some("New <title>")
    );
    assert_eq!(fs::read(&input).unwrap(), SOURCE);
}

#[test]
fn test_inspect_should_exit_with_status_two_when_references_are_invalid() {
    let directory = TestDirectory::new();
    let input = directory.file("invalid.lss");
    let source =
        std::str::from_utf8(SOURCE)
            .unwrap()
            .replacen("<gid>20</gid>", "<gid>999</gid>", 1);
    fs::write(&input, source).unwrap();

    let result = cli(&[Path::new("inspect"), &input]);

    assert_eq!(result.status.code(), Some(2));
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(report["errors"].as_u64().unwrap() > 0);
}
