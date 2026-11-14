#[path = "support/filters.rs"]
mod fixture;
use rustsurvey_domain::{
    collection::{Answers, Questionnaire},
    rules,
};
use serde_json::{Value, json};

#[test]
fn test_filters_should_match_the_shared_corpus_with_typed_sources_other_and_ancestors() {
    for case in fixture::cases() {
        let definition = fixture::definition(&case);
        let answers: Answers =
            serde_json::from_value(fixture::answers(&case, &definition)).unwrap();
        let runtime = Questionnaire::compile_forms(&definition)
            .unwrap_or_else(|issues| panic!("{}: {issues:?}", case["name"]));
        let snapshot = runtime
            .evaluate(&answers, false)
            .unwrap_or_else(|issues| panic!("{}: {issues:?}", case["name"]));
        let target: Answers = snapshot
            .answers
            .iter()
            .filter(|(key, _)| key.starts_with("TARGET"))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        assert_eq!(
            serde_json::to_value(target).unwrap(),
            case["expected"],
            "{}",
            case["name"]
        );
        if let Some(expected) = case.get("expectedSource") {
            let source: Answers = snapshot
                .answers
                .iter()
                .filter(|(key, _)| key.starts_with("SRC_"))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            assert_eq!(
                serde_json::to_value(source).unwrap(),
                *expected,
                "{}",
                case["name"]
            );
        }
        if let Some(expected) = case.get("expectedMiddle") {
            let middle: Answers = snapshot
                .answers
                .iter()
                .filter(|(key, _)| key.starts_with("MID_"))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            assert_eq!(
                serde_json::to_value(middle).unwrap(),
                *expected,
                "{}",
                case["name"]
            );
        }
    }
}

#[test]
fn test_filter_validation_should_reject_cycles_and_scalar_sources() {
    let mut definition = fixture::definition(&json!({"kind":"multiple_text","answers":{}}));
    definition.groups[0].questions[0].attributes =
        definition.groups[0].questions[1].attributes.clone();
    definition.groups[0].questions[0].attributes[0].value = "TARGET".into();
    let issues = rules::validate(&definition, None);
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.to_lowercase().contains("cycle")),
        "{issues:?}"
    );
    let mut scalar = fixture::definition(&json!({"kind":"single_choice","answers":{}}));
    let issues = rules::validate(&scalar, None);
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("lignes multiples ou un classement"))
    );
    scalar.groups[0].questions[1].attributes.clear();
    assert_eq!(
        serde_json::to_value(rules::validate(&scalar, None)).unwrap(),
        Value::Array(vec![])
    );
}

#[test]
fn test_filter_validation_should_reject_a_renamed_or_incompatible_source_even_when_the_filter_text_is_unchanged()
 {
    let original = fixture::definition(&json!({"kind":"multiple_text","answers":{}}));
    let mut changed = original.clone();
    changed.groups[0].questions[0] =
        fixture::definition(&json!({"kind":"single_choice","answers":{}})).groups[0].questions[0]
            .clone();
    assert!(
        rules::validate(&changed, Some(&original))
            .iter()
            .any(|issue| issue.message.contains("Le filtre SRC"))
    );
    let mut renamed = original.clone();
    renamed.groups[0].questions[0].code = "RENAMED".into();
    assert!(
        rules::validate(&renamed, Some(&original))
            .iter()
            .any(|issue| issue.message.contains("Le filtre SRC"))
    );
    assert!(
        rules::validate(&changed, Some(&changed)).is_empty(),
        "unchanged source settings must remain preservable"
    );
}

#[test]
fn test_filtered_ranking_should_require_only_available_positions() {
    for answers in [json!({"SRC_A1":"Y"}), json!({})] {
        let case = json!({"kind":"multiple_choice","targetKind":"ranking","answers":answers,"targetAnswers":{"TARGET_1":"A1"}});
        let mut definition = fixture::definition(&case);
        definition.groups[0].questions.last_mut().unwrap().mandatory =
            rustsurvey_domain::Mandatory::Strict;
        let input: Answers = serde_json::from_value(fixture::answers(&case, &definition)).unwrap();
        let result = Questionnaire::compile_forms(&definition)
            .unwrap()
            .evaluate(&input, true);
        assert!(result.is_ok(), "{answers}: {:?}", result.err());
    }
}

#[test]
fn test_filtered_ranking_should_reject_unknown_duplicate_and_gapped_answers() {
    for (target, message) in [
        (json!({"TARGET_1":"unknown"}), "Choix de réponse inconnu"),
        (
            json!({"TARGET_1":"A1","TARGET_2":"A1"}),
            "une seule position",
        ),
        (json!({"TARGET_2":"A1"}), "position vide"),
    ] {
        let case = json!({"kind":"multiple_choice","targetKind":"ranking","answers":{"SRC_A1":"Y"},"targetAnswers":target});
        let definition = fixture::definition(&case);
        let input: Answers = serde_json::from_value(fixture::answers(&case, &definition)).unwrap();
        let result = Questionnaire::compile_forms(&definition)
            .unwrap()
            .evaluate(&input, true);
        let issues = result.err().expect("invalid ranking must be rejected");
        assert!(
            issues.iter().any(|issue| issue.message.contains(message)),
            "{issues:?}"
        );
    }
}

#[test]
fn test_radio_filter_should_recheck_mandatory_after_clearing_a_selection() {
    let case = json!({"kind":"multiple_choice","targetKind":"single_choice","answers":{"SRC_A1":"Y"},"targetAnswers":{"TARGET":"A2"}});
    let mut definition = fixture::definition(&case);
    definition.groups[0].questions.last_mut().unwrap().mandatory =
        rustsurvey_domain::Mandatory::Strict;
    let input: Answers = serde_json::from_value(fixture::answers(&case, &definition)).unwrap();
    let issues = Questionnaire::compile_forms(&definition)
        .unwrap()
        .evaluate(&input, true)
        .err()
        .unwrap();
    assert!(
        issues
            .iter()
            .any(|issue| issue.message == "Cette question est obligatoire.")
    );
}
