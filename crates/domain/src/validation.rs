use crate::{
    Definition, Group, MAX_ANSWER_CELLS, MAX_QUESTIONS, Mandatory, Question, QuestionKind,
    SCHEMA_VERSION, ValidationIssue,
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
        if definition.schema_version != SCHEMA_VERSION {
            self.issue(
                "schemaVersion",
                "Version de définition non prise en charge.",
            );
        }
        self.text("title", &definition.title, 200, true);
        self.text("welcome", &definition.welcome, 20000, false);
        self.text("end", &definition.end, 20000, false);
        if definition.languages.len() > 32 || definition.translations.len() > 32 {
            self.issue("translations", "Maximum de 32 langues.");
        }
        let mut languages = BTreeSet::new();
        for translation in definition.translations.iter().take(32) {
            if !languages.insert(&translation.language) {
                self.issue("translations", "Langue dupliquée.");
            }
            self.text("translations", &translation.language, 32, true);
            self.text(
                "translations",
                translation.title.as_deref().unwrap_or(""),
                20000,
                false,
            );
            self.text(
                "translations",
                translation.welcome.as_deref().unwrap_or(""),
                20000,
                false,
            );
            self.text(
                "translations",
                translation.end.as_deref().unwrap_or(""),
                20000,
                false,
            );
        }

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
            .map(|question| crate::rules::fields(question).len())
            .sum();
        if cells > MAX_ANSWER_CELLS {
            self.issue("groups", "Maximum de 20000 cellules de réponse.");
            return false;
        }
        true
    }

    fn group(&mut self, group: &Group, path: &str) {
        self.identifier(path, &group.id);
        self.text(&format!("{path}.relevance"), &group.relevance, 4096, false);
        self.translations(path, &group.translations);
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
        self.text(
            &format!("{path}.relevance"),
            &question.relevance,
            4096,
            false,
        );
        self.translations(path, &question.translations);
        if question.attributes.len() > 200 {
            self.issue(path, "Maximum de 200 options par question.");
        }
        for attribute in question.attributes.iter().take(200) {
            self.text(path, &attribute.name, 128, true);
            self.text(path, &attribute.value, 20000, false);
        }
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
        let fields = crate::rules::fields(question);
        if fields.len() > 2000 {
            self.issue(path, "Maximum de 2000 cellules par question.");
        }
        for field in fields {
            self.field(path, field);
        }
    }

    fn translations(&mut self, path: &str, translations: &[crate::Translation]) {
        if translations.len() > 32 {
            self.issue(path, "Maximum de 32 traductions.");
        }
        for translation in translations.iter().take(32) {
            self.text(path, &translation.language, 32, true);
            self.text(
                path,
                translation.text.as_deref().unwrap_or(""),
                20000,
                false,
            );
            self.text(
                path,
                translation.help.as_deref().unwrap_or(""),
                20000,
                false,
            );
        }
    }

    fn choices(&mut self, question: &Question, path: &str) {
        if matches!(
            question.kind,
            QuestionKind::SingleChoice | QuestionKind::Dropdown | QuestionKind::SingleChoiceComment
        ) && question.choices.is_empty()
        {
            self.issue(format!("{path}.choices"), "Prévoir au moins un choix.");
        }
        for (role, choices) in [
            ("choices", &question.choices),
            ("subquestions", &question.subquestions),
        ] {
            if choices.len() > 200 {
                self.issue(format!("{path}.{role}"), "Maximum de 200 éléments par axe.");
            }
            let mut codes = BTreeSet::new();
            for (index, choice) in choices.iter().take(200).enumerate() {
                let choice_path = format!("{path}.{role}[{index}]");
                self.identifier(&choice_path, &choice.id);
                if choice.code.is_empty()
                    || choice.code.len() > 128
                    || !codes.insert((choice.scale, &choice.code))
                {
                    self.issue(
                        format!("{choice_path}.code"),
                        "Code de choix invalide ou dupliqué.",
                    );
                }
                if choice.scale > 1 {
                    self.issue(&choice_path, "Échelle non prise en charge.");
                }
                self.text(&format!("{choice_path}.label"), &choice.label, 10000, false);
                self.text(&format!("{choice_path}.help"), &choice.help, 10000, false);
                self.text(
                    &format!("{choice_path}.relevance"),
                    &choice.relevance,
                    4096,
                    false,
                );
                self.translations(&choice_path, &choice.translations);
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
