use rustsurvey_domain::{
    Definition,
    collection::{Answers, Questionnaire},
};
use serde::Deserialize;

#[test]
fn test_native_profile_should_execute_each_of_its_seventeen_question_types() {
    use rustsurvey_domain::QuestionKind::*;
    let kinds = [
        ShortText,
        LongText,
        HugeText,
        Number,
        Date,
        SingleChoice,
        Dropdown,
        RatingFive,
        YesNo,
        Gender,
        MultipleChoice,
        MultipleText,
        MultipleNumbers,
        SingleChoiceComment,
        Information,
        Equation,
        Language,
    ];
    for kind in kinds {
        let mut definition = corpus().definition;
        definition.groups.truncate(1);
        let question = &mut definition.groups[0].questions[0];
        question.kind = kind;
        question.mandatory = rustsurvey_domain::Mandatory::No;
        let (key, value) = match kind {
            Number => ("GATE", "0"),
            Date => ("GATE", "2024-02-29"),
            RatingFive => ("GATE", "3"),
            YesNo => ("GATE", "Y"),
            Gender => ("GATE", "F"),
            Language => ("GATE", "en"),
            MultipleChoice | MultipleText | MultipleNumbers => {
                question.subquestions = vec![rustsurvey_domain::Choice {
                    id: "row".to_owned(),
                    code: "A".to_owned(),
                    label: "A".to_owned(),
                    help: String::new(),
                    scale: 0,
                    relevance: "1".to_owned(),
                    translations: Vec::new(),
                    source: None,
                }];
                let value = match kind {
                    MultipleChoice => "Y",
                    MultipleNumbers => "0",
                    _ => "été",
                };
                ("GATE_A", value)
            }
            SingleChoice | Dropdown | SingleChoiceComment => {
                question.choices = vec![rustsurvey_domain::Choice {
                    id: "option".to_owned(),
                    code: "01".to_owned(),
                    label: "Premier".to_owned(),
                    help: String::new(),
                    scale: 0,
                    relevance: "1".to_owned(),
                    translations: Vec::new(),
                    source: None,
                }];
                ("GATE", "01")
            }
            Equation => {
                question.attributes = vec![rustsurvey_domain::Attribute {
                    name: "equation".to_owned(),
                    value: "2 + 2".to_owned(),
                    language: None,
                }];
                ("GATE", "4")
            }
            _ => ("GATE", "été"),
        };
        let answers = if kind == Information {
            Answers::new()
        } else {
            Answers::from([(key.to_owned(), value.to_owned())])
        };
        let runtime = Questionnaire::compile(&definition)
            .unwrap_or_else(|issues| panic!("{kind:?}: {issues:?}"));
        let snapshot = runtime
            .evaluate(&answers, true)
            .unwrap_or_else(|issues| panic!("{kind:?}: {issues:?}"));
        assert_eq!(snapshot.answers, answers, "{kind:?}");
    }
}

#[test]
fn test_dates_should_reject_signed_components_and_invalid_leap_days() {
    let mut definition = corpus().definition;
    definition.groups.truncate(1);
    definition.groups[0].questions[0].kind = rustsurvey_domain::QuestionKind::Date;
    let runtime = Questionnaire::compile(&definition).unwrap();
    for value in [
        "2023-02-29",
        "2024-02-30",
        "2024-+1-01",
        "+024-01-01",
        "2024-01-+1",
    ] {
        let result = runtime.evaluate(
            &Answers::from([("GATE".to_owned(), value.to_owned())]),
            false,
        );
        let issues = match result {
            Err(issues) => issues,
            Ok(_) => panic!("accepted {value}"),
        };
        assert_eq!(issues[0].message, "Une date valide est attendue.");
    }
}

#[derive(Deserialize)]
struct Corpus {
    definition: Definition,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    name: String,
    answers: Answers,
    submit: bool,
    expected: Option<Answers>,
    #[serde(default)]
    groups: Vec<String>,
    #[serde(default)]
    relevant: Vec<String>,
    #[serde(default)]
    warnings: Vec<String>,
    error: Option<String>,
}

fn corpus() -> Corpus {
    serde_json::from_str(include_str!("../../../examples/rules/collection.json")).unwrap()
}

#[test]
fn test_collection_should_match_client_visibility_clearing_calculations_and_validation() {
    let corpus = corpus();
    let runtime = Questionnaire::compile(&corpus.definition).unwrap();
    for case in corpus.cases {
        let result = runtime.evaluate(&case.answers, case.submit);
        if let Some(error) = case.error {
            let issues = match result {
                Err(issues) => issues,
                Ok(_) => panic!("{}: expected rejection", case.name),
            };
            assert!(
                issues.iter().any(|issue| issue.message.contains(&error)),
                "{}: {issues:?}",
                case.name
            );
            continue;
        }
        let snapshot = result.unwrap_or_else(|issues| panic!("{}: {issues:?}", case.name));
        assert_eq!(
            snapshot.answers,
            case.expected.unwrap(),
            "{}: answers",
            case.name
        );
        assert_eq!(snapshot.groups, case.groups, "{}: groups", case.name);
        assert_eq!(
            snapshot.relevant.into_iter().collect::<Vec<_>>(),
            case.relevant,
            "{}: relevance",
            case.name
        );
        assert_eq!(
            snapshot
                .warnings
                .iter()
                .map(|issue| issue.path.clone())
                .collect::<Vec<_>>(),
            case.warnings,
            "{}: warnings",
            case.name
        );
    }
}

#[test]
fn test_profile_should_reject_unsupported_execution_features_before_opening_collection() {
    for (name, value) in [
        ("time_limit", "5"),
        ("array_filter", "GATE"),
        ("unknown_option", "0"),
    ] {
        let mut definition = corpus().definition;
        definition.groups[0].questions[0]
            .attributes
            .push(rustsurvey_domain::Attribute {
                name: name.to_owned(),
                value: value.to_owned(),
                language: None,
            });
        let issues = match Questionnaire::compile(&definition) {
            Err(issues) => issues,
            Ok(_) => panic!("accepted {name}"),
        };
        assert!(!issues.is_empty(), "{name} must be diagnosed");
    }
    let mut definition = corpus().definition;
    definition.groups[0].questions[0].kind = rustsurvey_domain::QuestionKind::FileUpload;
    assert!(Questionnaire::compile(&definition).is_err());
}

#[test]
fn test_collection_should_reject_unknown_fields_and_limit_answer_allocations() {
    let runtime = Questionnaire::compile(&corpus().definition).unwrap();
    for (key, value, message) in [
        ("UNKNOWN", "x".to_owned(), "inconnue"),
        ("GATE", "x".repeat(65_537), "volumineuses"),
    ] {
        let result = runtime.evaluate(&Answers::from([(key.to_owned(), value)]), false);
        let issues = match result {
            Err(issues) => issues,
            Ok(_) => panic!("accepted invalid answers"),
        };
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains(message));
    }
}
