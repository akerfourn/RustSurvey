use rustsurvey_domain::{
    Definition, collection::Questionnaire, participation::ParticipationPolicy,
};
use rustsurvey_limesurvey::{Limits, LssDocument, SettingsContext, SourceRuntimeFacts};
use serde_json::Value;

#[path = "support/import_execution.rs"]
mod fixtures;
use fixtures::{attach, fixture};

#[test]
fn test_import_execution_should_compile_a_frozen_native_snapshot_and_leave_the_source_unchanged() {
    let (definition, xml, context, facts) = fixture();
    let document = LssDocument::parse(xml.as_bytes(), Limits::default()).unwrap();
    let plan = document
        .compile_import(&definition, &context, &facts)
        .unwrap();
    assert_eq!(plan.source_profile, "limesurvey-import-v1");
    assert_eq!(plan.native_profile, "native-pages-v1");
    assert_eq!(plan.participation_policy, ParticipationPolicy::Open);
    assert!(
        plan.definition
            .groups
            .iter()
            .flat_map(|group| &group.questions)
            .all(|question| question.theme == "core")
    );
    assert_eq!(document.original_xml(), xml.as_bytes());
    assert_ne!(definition.groups[0].questions[0].theme, "core");
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../examples/rules/collection-forms.json"
    ))
    .unwrap();
    let answers = serde_json::from_value(corpus["baseline"]["answers"].clone()).unwrap();
    let result = Questionnaire::compile_profile(&plan.definition, plan.native_profile)
        .unwrap()
        .evaluate(&answers, true)
        .unwrap();
    assert_eq!(
        serde_json::to_value(&result.answers).unwrap(),
        corpus["baseline"]["expected"]
    );
}

#[test]
fn test_import_execution_should_require_effective_source_facts_instead_of_local_defaults() {
    let (definition, xml, mut context, mut facts) = fixture();
    let document = LssDocument::parse(xml.as_bytes(), Limits::default()).unwrap();
    context.global.remove("shownoanswer");
    let issues = document
        .compile_import(&definition, &context, &facts)
        .err()
        .unwrap();
    assert!(
        issues
            .iter()
            .any(|issue| issue.path.ends_with("shownoanswer"))
    );
    context
        .global
        .insert("shownoanswer".into(), Some("Y".into()));
    for changed in ["delete", "noanswer", "plugins", "identity", "theme"] {
        facts.delete_non_values = changed != "delete";
        facts.show_no_answer = changed != "noanswer";
        facts.active_plugins = u32::from(changed == "plugins");
        facts.survey_id = if changed == "identity" {
            "702".into()
        } else {
            "701".into()
        };
        facts.effective_theme = if changed == "theme" {
            "custom".into()
        } else {
            "vanilla".into()
        };
        assert!(
            document
                .compile_import(&definition, &context, &facts)
                .is_err(),
            "{changed}"
        );
    }
}

#[test]
fn test_import_execution_should_refuse_active_source_features_and_unknown_fields_without_disclosing_values()
 {
    let (definition, xml, context, facts) = fixture();
    for changed in [
        xml.replace("<allowprev>Y</allowprev>", "<allowprev>N</allowprev>"),
        xml.replace("<datestamp>Y</datestamp>", "<datestamp>N</datestamp>"),
        xml.replacen(
            "<gsid>2</gsid>",
            "<gsid>2</gsid><startdate>private-value</startdate>",
            1,
        ),
        xml.replacen(
            "<gsid>2</gsid>",
            "<gsid>2</gsid><unknown_feature>private-value</unknown_feature>",
            1,
        ),
        xml.replace("<usecaptcha>N</usecaptcha>", "<usecaptcha>Y</usecaptcha>"),
    ] {
        let document = LssDocument::parse(changed.as_bytes(), Limits::default()).unwrap();
        let issues = document
            .compile_import(&definition, &context, &facts)
            .err()
            .unwrap();
        assert!(
            !serde_json::to_string(&issues)
                .unwrap()
                .contains("private-value")
        );
    }
}

#[test]
fn test_import_execution_should_map_cookie_and_linked_invitation_policies_explicitly() {
    let (definition, xml, context, mut facts) = fixture();
    let cookie_xml = xml.replace("<usecookie>N</usecookie>", "<usecookie>Y</usecookie>");
    let cookie = LssDocument::parse(cookie_xml.as_bytes(), Limits::default()).unwrap();
    assert_eq!(
        cookie
            .compile_import(&definition, &context, &facts)
            .unwrap()
            .participation_policy,
        ParticipationPolicy::BrowserOnce
    );
    let linked = xml
        .replace("<anonymized>Y</anonymized>", "<anonymized>N</anonymized>")
        .replace(
            "<tokenanswerspersistence>N</tokenanswerspersistence>",
            "<tokenanswerspersistence>Y</tokenanswerspersistence>",
        )
        .replace(
            "<alloweditaftercompletion>N</alloweditaftercompletion>",
            "<alloweditaftercompletion>Y</alloweditaftercompletion>",
        );
    facts.has_participants_table = true;
    let document = LssDocument::parse(linked.as_bytes(), Limits::default()).unwrap();
    assert_eq!(
        document
            .compile_import(&definition, &context, &facts)
            .unwrap()
            .participation_policy,
        ParticipationPolicy::Invitation {
            persistence: true,
            allow_edit_after_submit: true
        }
    );
}

#[test]
fn test_import_execution_should_require_a_known_utc_clock_without_inventing_source_offsets() {
    let (definition, xml, context, mut facts) = fixture();
    let document = LssDocument::parse(xml.as_bytes(), Limits::default()).unwrap();
    for (zone, adjustment) in [
        (None, Some("0")),
        (Some("UTC"), None),
        (Some("Europe/Paris"), Some("0")),
        (Some("UTC"), Some("1")),
        (Some("UTC"), Some("1e-1000")),
    ] {
        facts.time_zone = zone.map(str::to_owned);
        facts.time_adjust = adjustment.map(str::to_owned);
        let issues = document
            .compile_import(&definition, &context, &facts)
            .err()
            .unwrap();
        assert!(
            issues
                .iter()
                .any(|issue| issue.path == "source.runtimeFacts.clock")
        );
    }
    facts.time_zone = Some("UTC".into());
    facts.time_adjust = Some("-0.00".into());
    assert!(
        document
            .compile_import(&definition, &context, &facts)
            .is_ok()
    );
}

#[test]
#[ignore = "requires artifacts from the isolated LimeSurvey 6.5.14 reference harness"]
fn test_import_execution_reference_should_compile_an_actual_limesurvey_export() {
    use std::{fs::File, io::Read, path::PathBuf};
    let directory = PathBuf::from(
        std::env::var_os("RUSTSURVEY_EXECUTION_REFERENCE_DIR")
            .expect("isolated reference directory"),
    );
    assert!(
        directory.is_absolute()
            && directory
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("rustsurvey-limesurvey-reference-")
    );
    fn read(path: PathBuf) -> Vec<u8> {
        let mut bytes = Vec::new();
        File::open(path)
            .unwrap()
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .unwrap();
        assert!(bytes.len() <= 8 * 1024 * 1024);
        bytes
    }
    let bytes = read(directory.join("output/execution.lss"));
    let document = LssDocument::parse(&bytes, Limits::default()).unwrap();
    assert!(!document.report().has_errors(), "{:?}", document.report());
    let mut definition: Definition =
        serde_json::from_slice(&read(directory.join("definition.json"))).unwrap();
    attach(&mut definition, &document);
    let context: SettingsContext =
        serde_json::from_slice(&read(directory.join("output/execution-context.json"))).unwrap();
    let facts: SourceRuntimeFacts =
        serde_json::from_slice(&read(directory.join("output/execution-facts.json"))).unwrap();
    let plan = document
        .compile_import(&definition, &context, &facts)
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(plan.participation_policy, ParticipationPolicy::Open);
    assert_eq!(plan.native_profile, "native-pages-v1");
}
