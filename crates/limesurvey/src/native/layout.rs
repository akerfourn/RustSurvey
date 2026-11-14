use crate::SourceId;
use rustsurvey_domain::navigation::NavigationSettings;
use rustsurvey_domain::{Choice, Definition, Group, Question, QuestionKind};
use std::collections::BTreeMap;

pub(super) struct NativeGroup<'a> {
    pub id: u64,
    pub order: usize,
    pub source: &'a Group,
}
pub(super) struct NativeQuestion<'a> {
    pub id: u64,
    pub group_id: u64,
    pub order: usize,
    pub source: &'a Question,
    pub subquestions: Vec<NativeChoice<'a>>,
    pub answers: Vec<NativeChoice<'a>>,
}
pub(super) struct NativeChoice<'a> {
    pub id: u64,
    pub order: usize,
    pub source: &'a Choice,
}
pub(super) struct Layout<'a> {
    pub definition: &'a Definition,
    pub survey_id: u64,
    pub languages: Vec<&'a str>,
    pub groups: Vec<NativeGroup<'a>>,
    pub questions: Vec<NativeQuestion<'a>>,
    pub navigation: NavigationSettings,
}

impl<'a> Layout<'a> {
    pub fn new(definition: &'a Definition, survey_id: SourceId) -> Self {
        let mut languages = vec![definition.language.as_str()];
        for language in &definition.languages {
            if language != &definition.language {
                languages.push(language);
            }
        }
        let mut groups = Vec::new();
        let mut questions = Vec::new();
        let mut next_question = 1;
        let mut next_answer = 1;
        for (order, group) in definition.groups.iter().enumerate() {
            let group_id = order as u64 + 1;
            groups.push(NativeGroup {
                id: group_id,
                order,
                source: group,
            });
            for (order, question) in group.questions.iter().enumerate() {
                let id = next_question;
                next_question += 1;
                let subquestions = choices(&question.subquestions, &mut next_question);
                let answers = choices(&question.choices, &mut next_answer);
                questions.push(NativeQuestion {
                    id,
                    group_id,
                    order,
                    source: question,
                    subquestions,
                    answers,
                });
            }
        }
        Self {
            definition,
            survey_id: survey_id.0,
            languages,
            groups,
            questions,
            navigation: NavigationSettings::default(),
        }
    }
}

fn choices<'a>(source: &'a [Choice], next_id: &mut u64) -> Vec<NativeChoice<'a>> {
    let mut orders = BTreeMap::new();
    source
        .iter()
        .map(|source| {
            let order = orders.entry(source.scale).or_insert(0);
            let item = NativeChoice {
                id: *next_id,
                order: *order,
                source,
            };
            *order += 1;
            *next_id += 1;
            item
        })
        .collect()
}

pub(crate) fn question_type(kind: QuestionKind) -> Option<&'static str> {
    use QuestionKind::*;
    Some(match kind {
        ShortText => "S",
        LongText => "T",
        HugeText => "U",
        Number => "N",
        Date => "D",
        SingleChoice => "L",
        Dropdown => "!",
        RatingFive => "5",
        YesNo => "Y",
        Gender => "G",
        MultipleChoice => "M",
        MultipleChoiceComment => "P",
        SingleChoiceComment => "O",
        MultipleText => "Q",
        MultipleNumbers => "K",
        ArrayFive => "A",
        ArrayTen => "B",
        ArrayYesNo => "C",
        ArrayIncrease => "E",
        ArrayChoice => "F",
        ArrayColumn => "H",
        DualScale => "1",
        NumberMatrix => ":",
        TextMatrix => ";",
        Ranking => "R",
        Language => "I",
        Equation => "*",
        Information => "X",
        FileUpload => "|",
        Unsupported => return None,
    })
}

pub(crate) fn base_theme(kind: QuestionKind) -> &'static str {
    use QuestionKind::*;
    match kind {
        ShortText => "shortfreetext",
        LongText => "longfreetext",
        HugeText => "hugefreetext",
        Number => "numerical",
        Date => "date",
        SingleChoice => "listradio",
        Dropdown => "list_dropdown",
        RatingFive => "5pointchoice",
        YesNo => "yesno",
        Gender => "gender",
        MultipleChoice => "multiplechoice",
        MultipleChoiceComment => "multiplechoice_with_comments",
        SingleChoiceComment => "list_with_comment",
        MultipleText => "multipleshorttext",
        MultipleNumbers => "multiplenumeric",
        ArrayFive => "arrays/5point",
        ArrayTen => "arrays/10point",
        ArrayYesNo => "arrays/yesnouncertain",
        ArrayIncrease => "arrays/increasesamedecrease",
        ArrayChoice => "arrays/array",
        ArrayColumn => "arrays/column",
        DualScale => "arrays/dualscale",
        NumberMatrix => "arrays/multiflexi",
        TextMatrix => "arrays/texts",
        Ranking => "ranking",
        Language => "language",
        Equation => "equation",
        Information => "boilerplate",
        FileUpload => "file_upload",
        Unsupported => "",
    }
}
