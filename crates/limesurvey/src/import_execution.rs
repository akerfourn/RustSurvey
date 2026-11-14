//! Qualified execution of imported forms, separate from transfer publication and credentials.
mod fields;
mod options;

use crate::{GroupSettingsReport, LssDocument, SettingsContext, source_bindings};
use rustsurvey_domain::{
    Definition, ValidationIssue,
    collection::{DEFAULTS_PROFILE, PAGES_PROFILE, Questionnaire},
    navigation::NavigationSettings,
    participation::ParticipationPolicy,
};
use serde::{Deserialize, Serialize};

pub const IMPORT_EXECUTION_PROFILE: &str = "limesurvey-import-v1";

/// Facts absent from a structure export must be supplied from the same source snapshot.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceRuntimeFacts {
    pub schema_version: u32,
    pub survey_id: String,
    pub has_participants_table: bool,
    pub delete_non_values: bool,
    pub show_no_answer: bool,
    pub effective_theme: String,
    pub active_plugins: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_adjust: Option<String>,
}

pub struct ExecutableImport {
    pub definition: Definition,
    pub navigation: NavigationSettings,
    pub participation_policy: ParticipationPolicy,
    pub native_profile: &'static str,
    pub source_profile: &'static str,
}

pub(crate) fn issue(path: impl Into<String>, message: &str) -> ValidationIssue {
    ValidationIssue {
        path: path.into(),
        message: message.into(),
    }
}

impl LssDocument {
    /// Build an executable snapshot only after all source fields and effective options qualify.
    /// This does not publish responses or establish any participant's right to resume.
    pub fn compile_import(
        &self,
        definition: &Definition,
        context: &SettingsContext,
        facts: &SourceRuntimeFacts,
    ) -> Result<ExecutableImport, Vec<ValidationIssue>> {
        let gaps = crate::execution::inspect_profile(
            &self.xml,
            &self.report,
            128,
            fields::projected,
            false,
        );
        let mut issues = gaps.issues;
        let settings = self.group_settings(Some(context)).map_err(|_| {
            vec![issue(
                "source.context",
                "Contexte source contradictoire ou invalide.",
            )]
        })?;
        issues.extend(settings.issues.iter().cloned());
        let model = self
            .source_model()
            .ok_or_else(|| vec![issue("source", "Modèle source indisponible.")])?;
        if facts.schema_version != 1
            || model.survey_id.map(|id| id.0.to_string()).as_deref() != Some(&facts.survey_id)
        {
            issues.push(issue(
                "source.runtimeFacts",
                "Version ou identité des faits source invalide.",
            ));
        }
        if !facts.delete_non_values
            || !facts.show_no_answer
            || facts.active_plugins != 0
            || facts.effective_theme != "vanilla"
        {
            issues.push(issue("source.runtimeFacts", "Le profil exige l'effacement des valeurs inactives, Sans réponse, le thème vanilla et aucun plugin actif."));
        }
        fields::validate(&self.xml, facts, &mut issues);
        if !facts
            .time_zone
            .as_deref()
            .is_some_and(|zone| ["UTC", "Etc/UTC", "Etc/GMT", "GMT"].contains(&zone))
            || !facts.time_adjust.as_deref().is_some_and(zero_adjustment)
        {
            issues.push(issue("source.runtimeFacts.clock", "Le profil exige un contexte horaire UTC explicite sans décalage, pour conserver la signification des dates exportées."));
        }
        if issues.len() >= 128 {
            issues.truncate(128);
            return Err(issues);
        }
        let mapping = source_bindings(definition, model).map_err(|_| {
            vec![issue(
                "source.mapping",
                "Correspondances source indisponibles.",
            )]
        })?;
        issues.extend(mapping.issues);
        let configuration = options::resolve(&settings, facts, &mut issues);
        let mut definition = definition.clone();
        check_defaults(model, &mut definition, &mut issues);
        normalize_themes(&mut definition, &mut issues);
        if let Some((show_welcome, _, _)) = &configuration {
            definition.show_welcome = Some(*show_welcome);
        }
        if !issues.is_empty() {
            issues.truncate(128);
            return Err(issues);
        }
        let (_, navigation, participation_policy) = configuration
            .ok_or_else(|| vec![issue("source.settings", "Réglages effectifs incomplets.")])?;
        let has_defaults = definition
            .groups
            .iter()
            .flat_map(|group| &group.questions)
            .any(|question| {
                question
                    .defaults
                    .as_ref()
                    .is_some_and(|defaults| !defaults.values.is_empty())
            });
        let native_profile = if has_defaults {
            DEFAULTS_PROFILE
        } else {
            PAGES_PROFILE
        };
        Questionnaire::compile_profile(&definition, native_profile)?;
        Ok(ExecutableImport {
            definition,
            navigation,
            participation_policy,
            native_profile,
            source_profile: IMPORT_EXECUTION_PROFILE,
        })
    }
}

fn check_defaults(
    model: &crate::SurveySource,
    definition: &mut Definition,
    issues: &mut Vec<ValidationIssue>,
) {
    for question in definition
        .groups
        .iter_mut()
        .flat_map(|group| &mut group.questions)
    {
        let original = model.questions.iter().find(|source| {
            question
                .source
                .as_ref()
                .is_some_and(|identity| identity.id == source.id.0.to_string())
        });
        let Some(original) = original else {
            continue;
        };
        match crate::project_question_defaults(model, original, question) {
            Ok(expected) if expected == question.defaults => {}
            Ok(_) => issues.push(issue(
                &question.id,
                "Les défauts de la révision diffèrent de leur projection source.",
            )),
            Err(errors) => issues.extend(errors),
        }
        if issues.len() > 128 {
            issues.truncate(128);
            return;
        }
    }
}

fn normalize_themes(definition: &mut Definition, issues: &mut Vec<ValidationIssue>) {
    for question in definition
        .groups
        .iter_mut()
        .flat_map(|group| &mut group.questions)
    {
        if issues.len() >= 128 {
            return;
        }
        let base = crate::native::layout::base_theme(question.kind);
        if !question.theme.is_empty() && question.theme != base {
            issues.push(issue(
                &question.id,
                "Le thème de question ne correspond pas au rendu de base qualifié.",
            ));
        } else {
            question.theme = "core".into();
        }
        if question.kind == rustsurvey_domain::QuestionKind::ArrayColumn
            && question.attributes.iter().any(|attribute| {
                ["array_filter", "array_filter_exclude"].contains(&attribute.name.as_str())
                    && !attribute.value.is_empty()
            })
        {
            issues.push(issue(
                &question.id,
                "La référence n'exécute pas les filtres cibles du tableau H.",
            ));
        }
    }
}

pub(crate) fn flag(settings: &GroupSettingsReport, name: &str) -> Option<bool> {
    match settings
        .settings
        .iter()
        .find(|item| item.name == name)?
        .value
        .as_ref()?
    {
        crate::SettingValue::Flag(value) => Some(*value),
        _ => None,
    }
}

fn zero_adjustment(value: &str) -> bool {
    let value = value.trim();
    let value = value
        .strip_prefix('+')
        .or_else(|| value.strip_prefix('-'))
        .unwrap_or(value);
    value.len() <= 64
        && value.bytes().any(|byte| byte == b'0')
        && value.bytes().all(|byte| byte == b'0' || byte == b'.')
        && value.bytes().filter(|byte| *byte == b'.').count() <= 1
}
