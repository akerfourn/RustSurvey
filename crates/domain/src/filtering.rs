use crate::QuestionKind;

pub fn supports_source(kind: QuestionKind) -> bool {
    use QuestionKind::*;
    matches!(
        kind,
        MultipleChoice
            | MultipleChoiceComment
            | MultipleText
            | MultipleNumbers
            | ArrayFive
            | ArrayTen
            | ArrayYesNo
            | ArrayIncrease
            | ArrayChoice
            | ArrayColumn
            | DualScale
            | NumberMatrix
            | TextMatrix
            | Ranking
    )
}
