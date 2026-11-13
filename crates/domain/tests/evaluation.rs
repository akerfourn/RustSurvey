use rustsurvey_domain::rules::{EvaluationBudget, Expression, Scalar};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Case {
    name: String,
    expression: String,
    #[serde(default)]
    values: BTreeMap<String, Scalar>,
    expected: Option<Scalar>,
    error: Option<String>,
}

#[test]
fn test_evaluation_should_match_the_shared_client_server_corpus() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../../../examples/rules/evaluation.json")).unwrap();
    for case in cases {
        let actual = Expression::compile(&case.expression).and_then(|expression| {
            expression.evaluate(|name| {
                case.values
                    .get(name)
                    .cloned()
                    .unwrap_or(Scalar::String(String::new()))
            })
        });
        if let Some(error) = case.error {
            assert!(
                actual.unwrap_err().contains(&error),
                "{}: incorrect error",
                case.name
            );
            continue;
        }
        let actual = actual.unwrap_or_else(|error| panic!("{}: {error}", case.name));
        let expected = case.expected.unwrap();
        if let (Scalar::Number(actual), Scalar::Number(expected)) = (&actual, &expected) {
            let tolerance = 1e-12 * expected.abs().max(1.0);
            assert!(
                (actual - expected).abs() <= tolerance,
                "{}: {actual} != {expected}",
                case.name
            );
        } else {
            assert_eq!(actual, expected, "{}", case.name);
        }
    }
}

#[test]
fn test_evaluation_should_share_a_bounded_budget_across_expressions() {
    let expression = Expression::compile("join(VALUE, VALUE)").unwrap();
    let lookup = |_: &str| Scalar::String("abcd".to_owned());
    let mut budget = EvaluationBudget::new(30);
    assert_eq!(
        expression.evaluate_with_budget(&lookup, &mut budget),
        Ok(Scalar::String("abcdabcd".to_owned()))
    );
    assert_eq!(
        expression.evaluate_with_budget(&lookup, &mut budget),
        Err("Budget d’évaluation dépassé.")
    );
}
