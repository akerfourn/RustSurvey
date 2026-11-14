use rustsurvey_domain::{
    Attribute, Definition,
    collection::{Answers, FORMS_PROFILE, PROFILE, Questionnaire},
};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Baseline {
    answers: Answers,
    expected: Answers,
}
#[derive(Deserialize)]
struct Corpus {
    definition: Definition,
    baseline: Baseline,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    name: String,
    base: Option<bool>,
    submit: bool,
    #[serde(default)]
    answers: Answers,
    #[serde(default)]
    expected: Answers,
    #[serde(default)]
    remove_expected: Vec<String>,
    #[serde(default)]
    attributes: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    subquestion_relevance: BTreeMap<String, BTreeMap<String, String>>,
    error: Option<String>,
}

fn corpus() -> Corpus {
    serde_json::from_str(include_str!(
        "../../../examples/rules/collection-forms.json"
    ))
    .unwrap()
}

#[test]
fn test_equation_should_allow_translated_labels_when_a_shared_attribute_defines_the_calculation() {
    let mut corpus = corpus();
    corpus.definition.upgrade().unwrap();
    corpus.definition.languages = vec!["fr".into(), "en".into()];
    let calculation = corpus.definition.groups[0]
        .questions
        .iter_mut()
        .find(|question| question.code == "CALC")
        .unwrap();
    calculation
        .translations
        .push(rustsurvey_domain::Translation {
            language: "en".into(),
            text: Some("Calculated result".into()),
            help: None,
        });
    let expression = calculation
        .attributes
        .iter()
        .find(|attribute| attribute.name == "equation")
        .unwrap()
        .value
        .clone();
    let snapshot = Questionnaire::compile_forms(&corpus.definition)
        .unwrap()
        .evaluate(&corpus.baseline.answers, true)
        .unwrap();
    assert_eq!(
        snapshot.answers.get("CALC"),
        corpus.baseline.expected.get("CALC")
    );

    let calculation = corpus.definition.groups[0]
        .questions
        .iter_mut()
        .find(|question| question.code == "CALC")
        .unwrap();
    calculation
        .attributes
        .retain(|attribute| attribute.name != "equation");
    calculation.title = format!("{{{expression}}}");
    let issues = Questionnaire::compile_forms(&corpus.definition)
        .err()
        .unwrap();
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("équations localisées")),
        "localized question text still needs qualification when it supplies the expression"
    );
}

#[test]
fn test_forms_profile_should_match_the_shared_28_type_behavior_corpus() {
    let corpus = corpus();
    for case in corpus.cases {
        let mut definition = corpus.definition.clone();
        for question in definition
            .groups
            .iter_mut()
            .flat_map(|group| &mut group.questions)
        {
            if let Some(settings) = case.attributes.get(&question.code) {
                question
                    .attributes
                    .retain(|attribute| !settings.contains_key(&attribute.name));
                for (name, value) in settings {
                    question.attributes.push(Attribute {
                        name: name.clone(),
                        value: value.clone(),
                        language: None,
                    });
                }
            }
            if let Some(conditions) = case.subquestion_relevance.get(&question.code) {
                for row in &mut question.subquestions {
                    if let Some(condition) = conditions.get(&row.code) {
                        row.relevance = condition.clone();
                    }
                }
            }
        }
        let mut answers = Answers::new();
        let mut expected = Answers::new();
        if case.base != Some(false) {
            answers = corpus.baseline.answers.clone();
            expected = corpus.baseline.expected.clone();
        }
        answers.extend(case.answers);
        expected.extend(case.expected);
        for key in case.remove_expected {
            expected.remove(&key);
        }
        let runtime = Questionnaire::compile_forms(&definition)
            .unwrap_or_else(|issues| panic!("{}: {issues:?}", case.name));
        let result = runtime.evaluate(&answers, case.submit);
        if let Some(error) = case.error {
            let issues = result
                .err()
                .unwrap_or_else(|| panic!("{}: expected rejection", case.name));
            assert!(
                issues.iter().any(|issue| issue.message.contains(&error)),
                "{}: {issues:?}",
                case.name
            );
        } else {
            let snapshot = result.unwrap_or_else(|issues| panic!("{}: {issues:?}", case.name));
            assert_eq!(snapshot.answers, expected, "{}", case.name);
        }
    }
}

#[test]
fn test_profile_dispatch_should_preserve_the_legacy_campaign_contract_and_reject_unknown_versions()
{
    let definition = corpus().definition;
    assert!(Questionnaire::compile_profile(&definition, PROFILE).is_err());
    assert!(Questionnaire::compile_profile(&definition, FORMS_PROFILE).is_ok());
    let issues = Questionnaire::compile_profile(&definition, "future-profile")
        .err()
        .unwrap();
    assert_eq!(issues[0].message, "Profil de collecte inconnu.");
}

#[test]
fn test_forms_profile_should_refuse_unbounded_matrix_options_before_accepting_answers() {
    let mut definition = corpus().definition;
    let question = definition.groups[0]
        .questions
        .iter_mut()
        .find(|question| question.code == "NUM")
        .unwrap();
    question.attributes.push(Attribute {
        name: "multiflexible_max".into(),
        value: "1e10".into(),
        language: None,
    });
    let issues = Questionnaire::compile_forms(&definition).err().unwrap();
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("2000 choix"))
    );
}

#[test]
fn test_slider_should_refuse_a_finite_step_that_overflows_its_progression() {
    let mut definition = corpus().definition;
    let question = definition.groups[0]
        .questions
        .iter_mut()
        .find(|question| question.code == "K")
        .unwrap();
    question
        .attributes
        .iter_mut()
        .find(|attribute| attribute.name == "slider_accuracy")
        .unwrap()
        .value = "1e-308".into();
    let issues = Questionnaire::compile_forms(&definition)
        .err()
        .expect("infinite slider progression");
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("Intervalle invalide"))
    );
}

#[test]
fn test_filters_should_match_selection_cells_without_interpreting_comment_fields_as_choices() {
    let corpus = corpus();
    let mut definition = corpus.definition;
    let question = definition.groups[0]
        .questions
        .iter_mut()
        .find(|question| question.code == "F")
        .unwrap();
    question.subquestions[1].code = "A1comment".into();
    question.attributes.push(Attribute {
        name: "array_filter".into(),
        value: "P".into(),
        language: None,
    });
    let mut answers = corpus.baseline.answers;
    answers.remove("F_A2");
    answers.insert("F_A1comment".into(), "02".into());
    let runtime = Questionnaire::compile_forms(&definition).unwrap();
    let snapshot = runtime.evaluate(&answers, true).unwrap();
    assert_eq!(
        snapshot.answers.get("F_A1comment").map(String::as_str),
        Some("02")
    );
}

#[test]
fn test_scalar_question_codes_should_not_be_confused_with_other_or_comment_cell_suffixes() {
    let corpus = corpus();
    for (original, code) in [("L", "CHOICE_other"), ("O", "CHOICE_comment")] {
        let mut definition = corpus.definition.clone();
        let mut question = definition.groups[0]
            .questions
            .iter()
            .find(|question| question.code == original)
            .unwrap()
            .clone();
        question.code = code.into();
        question.other = original == "L";
        definition.groups[0].questions = vec![question];
        let runtime = Questionnaire::compile_forms(&definition).unwrap();
        let answers = Answers::from([(code.into(), "01".into())]);
        let snapshot = runtime.evaluate(&answers, true).unwrap();
        assert_eq!(snapshot.answers, answers);
    }
}
