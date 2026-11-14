//! Reversible cell projection. Historical answers are never evaluated or revalidated here.
use crate::{ResponseRow, SourceBindings, SurveySource, source_bindings};
use rustsurvey_domain::{
    Definition, QuestionKind, ValidationIssue,
    collection::{Answers, MAX_ANSWER_BYTES},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

const TECHNICAL: &[&str] = &[
    "id",
    "submitdate",
    "lastpage",
    "startlanguage",
    "seed",
    "token",
    "startdate",
    "datestamp",
    "ipaddr",
    "refurl",
];

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreservedSourceValue {
    pub native_value: String,
    pub source_value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedResponse {
    pub answers: Answers,
    pub metadata: ResponseRow,
    pub original_values: BTreeMap<String, PreservedSourceValue>,
    pub issue_count: usize,
    pub issues: Vec<ValidationIssue>,
}

pub struct ResponseProjector {
    mapping: SourceBindings,
    columns: BTreeSet<String>,
    mapped_columns: BTreeSet<String>,
    date_keys: BTreeSet<String>,
    unknown_columns: BTreeSet<String>,
}

fn issue(path: impl Into<String>, message: &'static str) -> ValidationIssue {
    ValidationIssue {
        path: path.into(),
        message: message.into(),
    }
}

impl ResponseProjector {
    pub fn compile(
        definition: &Definition,
        source: &SurveySource,
        columns: &[String],
    ) -> Result<Self, Vec<ValidationIssue>> {
        let mapping = source_bindings(definition, source)
            .map_err(|_| vec![issue("mapping", "Correspondances source indisponibles.")])?;
        if mapping.issue_count > 0 {
            return Err(mapping.issues);
        }
        let declared: BTreeSet<_> = columns.iter().cloned().collect();
        if declared.len() != columns.len() {
            return Err(vec![issue(
                "columns",
                "Une colonne source est déclarée plusieurs fois.",
            )]);
        }
        let mapped_columns: BTreeSet<_> = mapping
            .fields
            .iter()
            .map(|field| field.column.clone())
            .collect();
        let missing: Vec<_> = mapped_columns
            .difference(&declared)
            .take(100)
            .map(|column| {
                issue(
                    format!("columns.{column}"),
                    "Colonne source attendue absente ; aucune valeur n'est inventée.",
                )
            })
            .collect();
        if !missing.is_empty() {
            return Err(missing);
        }
        let date_questions: BTreeSet<_> = definition
            .groups
            .iter()
            .flat_map(|group| &group.questions)
            .filter(|question| question.kind == QuestionKind::Date)
            .map(|question| &question.id)
            .collect();
        let date_keys = mapping
            .fields
            .iter()
            .filter(|field| date_questions.contains(&field.address.question_id))
            .map(|field| field.runtime_key.clone())
            .collect();
        let unknown_columns = declared
            .iter()
            .filter(|column| {
                !mapped_columns.contains(*column)
                    && !TECHNICAL.contains(&column.as_str())
                    && !mapping.ignored_columns.contains(column)
            })
            .cloned()
            .collect();
        Ok(Self {
            mapping,
            columns: declared,
            mapped_columns,
            date_keys,
            unknown_columns,
        })
    }

    pub fn bindings(&self) -> &SourceBindings {
        &self.mapping
    }

    pub fn project(&self, row: &ResponseRow) -> Result<ProjectedResponse, Vec<ValidationIssue>> {
        if row.keys().any(|column| !self.columns.contains(column)) {
            return Err(vec![issue(
                "row",
                "Une valeur source utilise une colonne non déclarée.",
            )]);
        }
        let mut projected = ProjectedResponse {
            answers: Answers::new(),
            metadata: ResponseRow::new(),
            original_values: BTreeMap::new(),
            issue_count: 0,
            issues: Vec::new(),
        };
        for column in self.columns.difference(&self.mapped_columns) {
            projected
                .metadata
                .insert(column.clone(), row.get(column).cloned().unwrap_or(None));
            if self.unknown_columns.contains(column) {
                projected.add_issue(issue(format!("columns.{column}"), "Colonne conservée sans interprétation native ; son effet doit être qualifié avant la collecte."));
            }
        }
        let mut bytes = 0_usize;
        for field in &self.mapping.fields {
            let Some(Some(value)) = row.get(&field.column) else {
                continue;
            };
            let mut native = value.as_str();
            if self.date_keys.contains(&field.runtime_key) && !value.is_empty() {
                if value.is_ascii()
                    && value.len() == 19
                    && &value[10..] == " 00:00:00"
                    && date_shape(&value[..10])
                {
                    native = &value[..10];
                    projected.original_values.insert(
                        field.column.clone(),
                        PreservedSourceValue {
                            native_value: native.into(),
                            source_value: value.clone(),
                        },
                    );
                } else if !date_shape(value) {
                    projected.add_issue(issue(format!("columns.{}", field.column), "La date ou l'heure source dépasse la représentation calendaire native ; sa valeur reste conservée."));
                }
            }
            bytes = bytes
                .saturating_add(field.runtime_key.len())
                .saturating_add(native.len());
            if bytes > MAX_ANSWER_BYTES || native.encode_utf16().count() > 65_536 {
                return Err(vec![issue(
                    "answers",
                    "Les valeurs source dépassent les limites du document de réponses natif.",
                )]);
            }
            projected
                .answers
                .insert(field.runtime_key.clone(), native.into());
        }
        Ok(projected)
    }

    /// Restore lexical values while their projected value is unchanged. NULL stays distinct from empty.
    pub fn restore(
        &self,
        projected: &ProjectedResponse,
    ) -> Result<ResponseRow, Vec<ValidationIssue>> {
        let keys: BTreeSet<_> = self
            .mapping
            .fields
            .iter()
            .map(|field| &field.runtime_key)
            .collect();
        if projected.answers.keys().any(|key| !keys.contains(key))
            || projected.metadata.keys().any(|column| {
                !self.columns.contains(column) || self.mapped_columns.contains(column)
            })
            || projected
                .original_values
                .keys()
                .any(|column| !self.mapped_columns.contains(column))
        {
            return Err(vec![issue(
                "projection",
                "Coordonnée de réponse ou de métadonnée inconnue.",
            )]);
        }
        let mut row = projected.metadata.clone();
        for field in &self.mapping.fields {
            let mut value = projected.answers.get(&field.runtime_key).cloned();
            if let Some(original) = projected.original_values.get(&field.column) {
                if !self.date_keys.contains(&field.runtime_key)
                    || original.source_value.len() != 19
                    || !original.source_value.is_ascii()
                    || &original.source_value[10..] != " 00:00:00"
                    || original.native_value != original.source_value[..10]
                    || !date_shape(&original.native_value)
                {
                    return Err(vec![issue(
                        "originalValues",
                        "Valeur source de conversion incohérente.",
                    )]);
                }
                if value.as_ref() == Some(&original.native_value) {
                    value = Some(original.source_value.clone());
                }
            }
            row.insert(field.column.clone(), value);
        }
        for column in &self.columns {
            row.entry(column.clone()).or_insert(None);
        }
        Ok(row)
    }
}

impl ProjectedResponse {
    fn add_issue(&mut self, issue: ValidationIssue) {
        self.issue_count += 1;
        if self.issues.len() < 100 {
            self.issues.push(issue);
        }
    }
}

fn date_shape(value: &str) -> bool {
    value.len() == 10
        && value.is_ascii()
        && value.bytes().enumerate().all(|(index, byte)| {
            if index == 4 || index == 7 {
                byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        })
}
