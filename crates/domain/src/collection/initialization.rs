use super::{Answers, Questionnaire, Snapshot, attribute, snapshot};
use crate::{
    ValidationIssue,
    navigation::{NavigationSettings, Page, PageFormat},
    rules::EvaluationBudget,
};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default)]
pub struct PageCursor {
    pub page_id: Option<String>,
    pub group_id: Option<String>,
}

impl Questionnaire {
    pub fn enter_page(
        &self,
        answers: &Answers,
        settings: NavigationSettings,
        language: &str,
        previous: Option<&PageCursor>,
        requested: Option<&PageCursor>,
        submit: bool,
    ) -> Result<(Snapshot, Page), Vec<ValidationIssue>> {
        self.check_initialization(language)?;
        let mut budget = EvaluationBudget::default();
        let mut working = answers.clone();
        let mut scope = BTreeSet::new();
        let baseline =
            snapshot::evaluate_context(self, &working, false, language, None, false, &mut budget)?;
        let mut relevant = baseline.relevant;
        if let Some(previous) = previous {
            let page = resolve(settings, &self.definition, &relevant, Some(previous))?;
            scope.extend(self.page_questions(settings, &page));
            scope.extend(self.hidden_prefix(settings, &page));
            let snapshot = self.probe(&mut working, language, &scope, &mut budget)?;
            relevant = snapshot.relevant;
        }
        let page = resolve(settings, &self.definition, &relevant, requested)?;
        scope.extend(self.hidden_prefix(settings, &page));
        let snapshot = self.probe(&mut working, language, &scope, &mut budget)?;
        let page = resolve(settings, &self.definition, &snapshot.relevant, requested)?;
        scope.extend(self.page_questions(settings, &page));
        if submit {
            scope.extend(
                self.questions
                    .iter()
                    .map(|question| question.source.id.clone()),
            );
        }
        let snapshot = snapshot::evaluate_context(
            self,
            &working,
            submit,
            language,
            Some(&scope),
            true,
            &mut budget,
        )?;
        let cursor = PageCursor {
            page_id: Some(page.id),
            group_id: page.group_id,
        };
        let page = resolve(
            settings,
            &self.definition,
            &snapshot.relevant,
            Some(&cursor),
        )?;
        Ok((snapshot, page))
    }

    fn probe(
        &self,
        working: &mut Answers,
        language: &str,
        scope: &BTreeSet<String>,
        budget: &mut EvaluationBudget,
    ) -> Result<Snapshot, Vec<ValidationIssue>> {
        let snapshot =
            snapshot::evaluate_context(self, working, false, language, Some(scope), false, budget)?;
        for key in &snapshot.initialized {
            if let Some(value) = snapshot.answers.get(key) {
                working.insert(key.clone(), value.clone());
            }
        }
        Ok(snapshot)
    }

    fn page_questions(&self, settings: NavigationSettings, page: &Page) -> BTreeSet<String> {
        match settings.format {
            PageFormat::All => self
                .questions
                .iter()
                .map(|question| question.source.id.clone())
                .collect(),
            PageFormat::Group => self
                .questions
                .iter()
                .filter(|question| Some(&question.group_id) == page.group_id.as_ref())
                .map(|question| question.source.id.clone())
                .collect(),
            PageFormat::Question => page.question_ids.iter().cloned().collect(),
        }
    }

    fn hidden_prefix(&self, settings: NavigationSettings, page: &Page) -> BTreeSet<String> {
        let questions: Vec<_> = self
            .definition
            .groups
            .iter()
            .flat_map(|group| &group.questions)
            .collect();
        let last = match settings.format {
            PageFormat::All => None,
            PageFormat::Group => self
                .definition
                .groups
                .iter()
                .find(|group| Some(&group.id) == page.group_id.as_ref())
                .and_then(|group| group.questions.last())
                .map(|question| question.id.as_str()),
            PageFormat::Question => page.question_ids.last().map(String::as_str),
        };
        let mut hidden = BTreeSet::new();
        for question in questions {
            if attribute(question, "hidden") == "1" {
                hidden.insert(question.id.clone());
            }
            if last == Some(question.id.as_str()) {
                break;
            }
        }
        hidden
    }
}

fn resolve(
    settings: NavigationSettings,
    definition: &crate::Definition,
    relevant: &BTreeSet<String>,
    cursor: Option<&PageCursor>,
) -> Result<Page, Vec<ValidationIssue>> {
    settings
        .resolve_page(
            definition,
            relevant,
            cursor.and_then(|cursor| cursor.page_id.as_deref()),
            cursor.and_then(|cursor| cursor.group_id.as_deref()),
        )
        .map_err(|issue| vec![issue])
}
