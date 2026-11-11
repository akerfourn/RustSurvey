use crate::{
    Definition, Group, MAX_ANSWER_CELLS, MAX_QUESTIONS, Mandatory, Question, QuestionKind,
    ValidationIssue,
};
use std::collections::BTreeSet;

#[derive(Default)]
struct Validator {
    issues: Vec<ValidationIssue>,
    ids: BTreeSet<String>,
    codes: BTreeSet<String>,
    fields: BTreeSet<String>,
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn valid_question_code(value: &str) -> bool {
    value.len() <= 64
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

impl Validator {
    fn issue(&mut self, path: impl Into<String>, message: &str) {
        if self.issues.len() >= 100 {
            return;
        }
        self.issues.push(ValidationIssue {
            path: path.into(),
            message: message.to_owned(),
        });
    }

    fn text(&mut self, path: &str, value: &str, maximum: usize, required: bool) {
        if required && value.trim().is_empty() {
            self.issue(path, "Une valeur est requise.");
        }
        if value.chars().count() > maximum {
            self.issue(path, "Texte trop long.");
        }
        if value.contains('\0') {
            self.issue(path, "Le caractère nul n’est pas autorisé.");
        }
    }

    fn identifier(&mut self, path: &str, value: &str) {
        if !valid_identifier(value) || !self.ids.insert(value.to_owned()) {
            self.issue(format!("{path}.id"), "Identifiant invalide ou dupliqué.");
        }
    }

    fn field(&mut self, path: &str, value: String) {
        if !self.fields.insert(value) {
            self.issue(path, "Nom de cellule ambigu.");
        }
    }

    fn metadata(&mut self, definition: &Definition) {
        if definition.schema_version != 1 {
            self.issue(
                "schemaVersion",
                "Version de définition non prise en charge.",
            );
        }
        self.text("title", &definition.title, 200, true);
        self.text("welcome", &definition.welcome, 20000, false);
        self.text("end", &definition.end, 20000, false);
        if definition.language.is_empty()
            || definition.language.len() > 32
            || !definition
                .language
                .bytes()
                .all(|byte| byte.is_ascii_alphabetic() || byte == b'-')
        {
            self.issue("language", "Code de langue invalide.");
        }
    }

    fn bounded_structure(&mut self, definition: &Definition) -> bool {
        if definition.groups.len() > 50 {
            self.issue("groups", "Maximum de 50 groupes.");
            return false;
        }
        let count: usize = definition
            .groups
            .iter()
            .map(|group| group.questions.len())
            .sum();
        if count > MAX_QUESTIONS {
            self.issue("groups", "Maximum de 500 questions.");
            return false;
        }
        let cells: usize = definition
            .groups
            .iter()
            .flat_map(|group| &group.questions)
            .map(|question| match question.kind {
                QuestionKind::MultipleChoice => question.choices.len(),
                QuestionKind::Information => 0,
                _ => 1,
            })
            .sum();
        if cells > MAX_ANSWER_CELLS {
            self.issue("groups", "Maximum de 20000 cellules de réponse.");
            return false;
        }
        true
    }

    fn group(&mut self, group: &Group, path: &str) {
        self.identifier(path, &group.id);
        self.text(&format!("{path}.title"), &group.title, 500, true);
        self.text(
            &format!("{path}.description"),
            &group.description,
            20000,
            false,
        );
        for (index, question) in group.questions.iter().enumerate() {
            self.question(question, &format!("{path}.questions[{index}]"));
        }
    }

    fn question(&mut self, question: &Question, path: &str) {
        self.identifier(path, &question.id);
        if !valid_question_code(&question.code) || !self.codes.insert(question.code.clone()) {
            self.issue(format!("{path}.code"), "Code invalide ou dupliqué.");
        }
        self.text(&format!("{path}.title"), &question.title, 10000, true);
        self.text(&format!("{path}.help"), &question.help, 10000, false);
        if question.kind == QuestionKind::Information && question.mandatory != Mandatory::No {
            self.issue(
                format!("{path}.mandatory"),
                "Une information sans réponse ne peut pas être obligatoire.",
            );
        }
        self.choices(question, path);
        if !matches!(
            question.kind,
            QuestionKind::MultipleChoice | QuestionKind::Information
        ) {
            self.field(path, question.code.clone());
        }
    }

    fn choices(&mut self, question: &Question, path: &str) {
        let accepts_choices = matches!(
            question.kind,
            QuestionKind::SingleChoice | QuestionKind::MultipleChoice
        );
        if accepts_choices && !(2..=200).contains(&question.choices.len()) {
            self.issue(format!("{path}.choices"), "Prévoir entre 2 et 200 choix.");
        }
        if !accepts_choices && !question.choices.is_empty() {
            self.issue(
                format!("{path}.choices"),
                "Ce type n’accepte pas de choix personnalisés.",
            );
        }
        let mut codes = BTreeSet::new();
        for (index, choice) in question.choices.iter().take(200).enumerate() {
            let choice_path = format!("{path}.choices[{index}]");
            self.identifier(&choice_path, &choice.id);
            if choice.code.is_empty()
                || choice.code.len() > 32
                || !choice.code.bytes().all(|byte| byte.is_ascii_alphanumeric())
                || !codes.insert(&choice.code)
            {
                self.issue(
                    format!("{choice_path}.code"),
                    "Code de choix invalide ou dupliqué.",
                );
            }
            self.text(&format!("{choice_path}.label"), &choice.label, 2000, true);
            if question.kind == QuestionKind::MultipleChoice {
                self.field(path, format!("{}_{}", question.code, choice.code));
            }
        }
    }
}

pub(super) fn validate(definition: &Definition) -> Vec<ValidationIssue> {
    let mut validator = Validator::default();
    validator.metadata(definition);
    if validator.bounded_structure(definition) {
        for (index, group) in definition.groups.iter().enumerate() {
            validator.group(group, &format!("groups[{index}]"));
        }
    }
    validator.issues
}
