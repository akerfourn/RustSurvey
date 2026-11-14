use rustsurvey_domain::{
    Definition, QuestionKind,
    cells::{self, Coordinate, Role},
};
use std::collections::BTreeSet;

fn definition() -> Definition {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../examples/rules/collection-forms.json"
    ))
    .unwrap();
    serde_json::from_value(corpus["definition"].clone()).unwrap()
}

#[test]
fn test_cells_should_match_existing_runtime_aliases_for_every_native_form() {
    for question in definition()
        .groups
        .iter()
        .flat_map(|group| &group.questions)
    {
        let cells = cells::for_question(question).unwrap();
        let addresses: BTreeSet<_> = cells.iter().map(|cell| &cell.address).collect();
        assert_eq!(
            addresses.len(),
            cells.len(),
            "cell addresses must be unique"
        );
        assert_eq!(
            cells
                .into_iter()
                .map(|cell| cell.runtime_key)
                .collect::<Vec<_>>(),
            rustsurvey_domain::rules::fields(question),
            "existing campaigns must keep their aliases"
        );
    }
}

#[test]
fn test_cells_should_keep_identity_after_codes_are_renamed_and_rows_reordered() {
    let mut definition = definition();
    let question = definition.groups[0]
        .questions
        .iter_mut()
        .find(|item| item.kind == QuestionKind::TextMatrix)
        .unwrap();
    let before = cells::for_question(question).unwrap();
    question.code = "RENAMED".into();
    question.subquestions[0].code = "NEWROW".into();
    question.subquestions.reverse();
    let after = cells::for_question(question).unwrap();
    let identities = |cells: &[cells::Cell]| {
        cells
            .iter()
            .map(|cell| cell.address.clone())
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(identities(&before), identities(&after));
    assert_ne!(before[0].runtime_key, after[0].runtime_key);
}

#[test]
fn test_cells_should_distinguish_comment_other_and_dual_scale_coordinates() {
    let definition = definition();
    let questions = &definition.groups[0].questions;
    let comments = cells::for_question(
        questions
            .iter()
            .find(|item| item.kind == QuestionKind::MultipleChoiceComment)
            .unwrap(),
    )
    .unwrap();
    assert!(
        comments
            .iter()
            .any(|item| item.address.role == Role::Comment)
    );
    assert!(comments.iter().any(|item| item.address.role == Role::Other));
    assert!(
        comments
            .iter()
            .any(|item| item.address.role == Role::OtherComment)
    );
    let dual = cells::for_question(
        questions
            .iter()
            .find(|item| item.kind == QuestionKind::DualScale)
            .unwrap(),
    )
    .unwrap();
    assert!(dual.iter().any(|item| matches!(
        item.address.coordinate,
        Coordinate::Subquestion { scale: Some(0), .. }
    )));
    assert!(dual.iter().any(|item| matches!(
        item.address.coordinate,
        Coordinate::Subquestion { scale: Some(1), .. }
    )));
}
