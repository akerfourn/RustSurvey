use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

use rustsurvey_limesurvey::{Limits, LssDocument};

const SOURCE: &[u8] = include_bytes!("fixtures/minimal-623.lss");
static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

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
