#[path = "support/defaults.rs"]
mod fixture;

use rustsurvey_domain::{
    Attribute, Definition, Question,
    cells::{Coordinate, Role},
    collection::{Answers, PageCursor, Questionnaire},
    defaults::{DefaultValue, QuestionDefaults},
    navigation::{NavigationSettings, PageFormat},
};
use serde_json::Value;

#[test]
fn test_default_initialization_should_match_persisted_values_of_reference_cases() {
    for case in fixture::cases() {
        let definition = fixture::definition(&case);
        let compiled = Questionnaire::compile_defaults(&definition);
        if case["nativeSupported"] == false || case["nativeExecutable"] == false {
            assert!(compiled.is_err(), "{}", case["name"]);
            continue;
        }
        let runtime = compiled.unwrap_or_else(|issues| panic!("{}: {issues:?}", case["name"]));
        let questions = definition
            .groups
            .iter()
            .flat_map(|group| &group.questions)
            .map(|question| question.id.clone())
            .collect();
        let target = case["target"]
            .as_str()
            .unwrap_or(case["question"].as_str().unwrap());
        let mut answers = Answers::new();
        seed(&mut answers, &case["seed"]);
        for step in case["steps"].as_array().unwrap() {
            seed(&mut answers, &step["seed"]);
            match step["state"].as_str().unwrap() {
                "missing" | "null" => {
                    answers.remove(target);
                }
                "empty" => {
                    answers.insert(target.into(), "".into());
                }
                "existing" => {
                    answers.insert(target.into(), step["value"].as_str().unwrap().into());
                }
                "keep" => {}
                state => panic!("unknown state {state}"),
            }
            let snapshot = runtime
                .evaluate_initialized(
                    &answers,
                    false,
                    step["language"].as_str().unwrap(),
                    &questions,
                )
                .unwrap_or_else(|issues| panic!("{}: {issues:?}", case["name"]));
            let expected = step
                .get("expectedWrite")
                .map(|write| &write["value"])
                .unwrap_or(&step["expected"]);
            assert_eq!(
                snapshot.answers.get(target).map(String::as_str),
                expected.as_str(),
                "{}: {step}",
                case["name"]
            );
            answers = snapshot.answers;
        }
    }
}

#[test]
fn test_page_entry_should_defer_future_defaults_and_keep_explicit_clearing() {
    let mut definition = selected(&["S", "T"]);
    set_default(question(&mut definition, "S"), Coordinate::Primary, "start");
    set_default(
        question(&mut definition, "T"),
        Coordinate::Primary,
        "value {S}",
    );
    let runtime = Questionnaire::compile_defaults(&definition).unwrap();
    let settings = question_pages();
    let (initial, first) = runtime
        .enter_page(&Answers::new(), settings, "fr", None, None, false)
        .unwrap();
    assert_eq!(initial.answers.get("S").unwrap(), "start");
    assert!(
        !initial.answers.contains_key("T"),
        "a future page must not consume its default early"
    );
    let first = cursor(&first);
    let target = PageCursor {
        page_id: Some(format!("q:{}", definition.groups[0].questions[1].id)),
        group_id: Some(definition.groups[0].id.clone()),
    };
    let mut changed = initial.answers;
    changed.insert("S".into(), "manual".into());
    let (second, _) = runtime
        .enter_page(&changed, settings, "fr", Some(&first), Some(&target), false)
        .unwrap();
    assert_eq!(second.answers.get("T").unwrap(), "value manual");
    let mut cleared = second.answers;
    cleared.insert("T".into(), "".into());
    let (resumed, _) = runtime
        .enter_page(
            &cleared,
            settings,
            "en",
            Some(&target),
            Some(&target),
            false,
        )
        .unwrap();
    assert_eq!(resumed.answers.get("T").map(String::as_str), Some(""));
}

#[test]
fn test_page_entry_should_preserve_existing_filtered_values_when_a_source_default_activates_them() {
    let mut definition = selected(&["M", "Q"]);
    let row = question(&mut definition, "M").subquestions[0].id.clone();
    set_default(
        question(&mut definition, "M"),
        Coordinate::Subquestion {
            id: row,
            scale: None,
        },
        "Y",
    );
    question(&mut definition, "Q").attributes.push(Attribute {
        name: "array_filter".into(),
        value: "M".into(),
        language: None,
    });
    let runtime = Questionnaire::compile_defaults(&definition).unwrap();
    let supplied = Answers::from([
        ("Q_A1".into(), "saved detail".into()),
        ("Q_A2".into(), "inactive".into()),
    ]);
    let (snapshot, _) = runtime
        .enter_page(
            &supplied,
            NavigationSettings::default(),
            "fr",
            None,
            None,
            false,
        )
        .unwrap();
    assert_eq!(
        snapshot.answers.get("Q_A1").map(String::as_str),
        Some("saved detail")
    );
    assert!(!snapshot.answers.contains_key("Q_A2"));
}

#[test]
fn test_page_entry_should_apply_hidden_preceding_defaults_before_choosing_the_first_page() {
    let mut definition = selected(&["S", "T", "N"]);
    let source = question(&mut definition, "S");
    source.attributes.push(Attribute {
        name: "hidden".into(),
        value: "1".into(),
        language: None,
    });
    set_default(source, Coordinate::Primary, "show");
    let target = question(&mut definition, "T");
    target.relevance = "S == 'show'".into();
    set_default(target, Coordinate::Primary, "visible");
    set_default(question(&mut definition, "N"), Coordinate::Primary, "2");
    let runtime = Questionnaire::compile_defaults(&definition).unwrap();
    let (snapshot, page) = runtime
        .enter_page(&Answers::new(), question_pages(), "fr", None, None, false)
        .unwrap();
    assert_eq!(
        page.question_ids,
        [definition.groups[0].questions[1].id.clone()]
    );
    assert_eq!(snapshot.answers.get("T").unwrap(), "visible");
    assert!(!snapshot.answers.contains_key("N"));
}

#[test]
fn test_defaults_should_follow_dependency_order_and_reject_cycles_or_unknown_variables() {
    let mut definition = selected(&["S", "T"]);
    set_default(
        question(&mut definition, "S"),
        Coordinate::Primary,
        "source",
    );
    set_default(question(&mut definition, "T"), Coordinate::Primary, "{S}");
    definition.groups[0].questions.reverse();
    let runtime = Questionnaire::compile_defaults(&definition).unwrap();
    let (snapshot, _) = runtime
        .enter_page(
            &Answers::new(),
            NavigationSettings::default(),
            "fr",
            None,
            None,
            false,
        )
        .unwrap();
    assert_eq!(snapshot.answers.get("T").unwrap(), "source");
    set_default(question(&mut definition, "S"), Coordinate::Primary, "{T}");
    let issues = Questionnaire::compile_defaults(&definition).err().unwrap();
    assert!(issues.iter().any(|issue| issue.message.contains("Cycle")));
    set_default(
        question(&mut definition, "S"),
        Coordinate::Primary,
        "{UNKNOWN}",
    );
    let issues = Questionnaire::compile_defaults(&definition).err().unwrap();
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("Variable inconnue"))
    );
}

#[test]
fn test_default_templates_should_preserve_quoted_braces_and_limit_generated_values() {
    use rustsurvey_domain::{
        defaults::template::Template,
        rules::{EvaluationBudget, Scalar},
    };
    let template = Template::compile("avant {if(1, '}', '{')} après").unwrap();
    assert_eq!(
        template
            .render(&|_| Scalar::Null, &mut EvaluationBudget::default())
            .unwrap(),
        "avant } après"
    );
    assert!(Template::compile("{sum(1, 2)").is_err());
    let template = Template::compile("{S}").unwrap();
    let error = template
        .render(
            &|_| Scalar::String("x".repeat(65_537)),
            &mut EvaluationBudget::default(),
        )
        .unwrap_err();
    assert!(error.contains("trop long"));
}

#[test]
fn test_defaults_should_require_page_context_and_diagnose_unverified_inactive_references() {
    let mut definition = selected(&["S", "T"]);
    set_default(question(&mut definition, "S"), Coordinate::Primary, "{S}");
    let runtime = Questionnaire::compile_defaults(&definition).unwrap();
    assert!(
        runtime.evaluate(&Answers::new(), false).err().unwrap()[0]
            .message
            .contains("entrée de page")
    );
    let (snapshot, _) = runtime
        .enter_page(
            &Answers::new(),
            NavigationSettings::default(),
            "fr",
            None,
            None,
            false,
        )
        .unwrap();
    assert_eq!(snapshot.answers.get("S").map(String::as_str), Some(""));
    question(&mut definition, "T").relevance = "S.NAOK == ''".into();
    let issues = Questionnaire::compile_defaults(&definition).err().unwrap();
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("contexte de valeurs inactives"))
    );
}

#[test]
fn test_default_initialization_should_keep_an_explicit_exclusive_selection() {
    let mut definition = selected(&["M"]);
    let source = question(&mut definition, "M");
    source.attributes.push(Attribute {
        name: "exclude_all_others".into(),
        value: "A1;A2".into(),
        language: None,
    });
    set_default(
        source,
        Coordinate::Subquestion {
            id: source.subquestions[0].id.clone(),
            scale: None,
        },
        "Y",
    );
    let runtime = Questionnaire::compile_defaults(&definition).unwrap();
    let answers = Answers::from([("M_A2".into(), "Y".into())]);
    let (snapshot, _) = runtime
        .enter_page(
            &answers,
            NavigationSettings::default(),
            "fr",
            None,
            None,
            false,
        )
        .unwrap();
    assert_eq!(snapshot.answers, answers);
    let conflicting = Answers::from([("M_A1".into(), "Y".into()), ("M_A2".into(), "Y".into())]);
    assert!(
        runtime
            .enter_page(
                &conflicting,
                NavigationSettings::default(),
                "fr",
                None,
                None,
                false
            )
            .is_err()
    );
}

fn seed(answers: &mut Answers, source: &Value) {
    if let Some(values) = source.as_object() {
        for (key, value) in values {
            answers.insert(key.clone(), value.as_str().unwrap().into());
        }
    }
}

fn selected(codes: &[&str]) -> Definition {
    let mut definition = fixture::definition(&fixture::cases()[0]);
    for group in &mut definition.groups {
        group
            .questions
            .retain(|question| codes.contains(&question.code.as_str()));
        for question in &mut group.questions {
            question.defaults = None;
        }
    }
    definition
}

fn question<'a>(definition: &'a mut Definition, code: &str) -> &'a mut Question {
    definition
        .groups
        .iter_mut()
        .flat_map(|group| &mut group.questions)
        .find(|question| question.code == code)
        .unwrap()
}

fn set_default(question: &mut Question, coordinate: Coordinate, value: &str) {
    question.defaults = Some(QuestionDefaults {
        same_for_all_languages: true,
        values: vec![DefaultValue {
            coordinate,
            role: Role::Answer,
            language: "fr".into(),
            value: value.into(),
        }],
    });
}

fn question_pages() -> NavigationSettings {
    NavigationSettings {
        format: PageFormat::Question,
        show_progress: true,
    }
}

fn cursor(page: &rustsurvey_domain::navigation::Page) -> PageCursor {
    PageCursor {
        page_id: Some(page.id.clone()),
        group_id: page.group_id.clone(),
    }
}
