use super::super::AppError;
use rustsurvey_domain::{Definition, ValidationIssue};
use rustsurvey_limesurvey::{
    FieldBinding, Limits, LssDocument, ResponseRow, ResponseTable, source_bindings,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Schema {
    pub schema_version: u32,
    pub source_survey_id: String,
    pub languages: Vec<String>,
    pub columns: Vec<String>,
    pub bindings: Vec<FieldBinding>,
    pub ignored_columns: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Report {
    conservation: &'static str,
    mapping: &'static str,
    execution: &'static str,
    continuity: &'static str,
    row_count: usize,
    submitted: usize,
    drafts: usize,
    unknown_status: usize,
    mapped_cells: usize,
    unknown_column_count: usize,
    missing_column_count: usize,
    duplicate_ids: usize,
    invalid_ids: usize,
    issue_count: usize,
    issues: Vec<ValidationIssue>,
}

pub(super) struct Row {
    pub ordinal: i64,
    pub source_id: Option<String>,
    pub status: &'static str,
    pub sha256: String,
    pub document: String,
}

pub(super) struct Plan {
    pub schema: String,
    pub report: String,
    pub rows: Vec<Row>,
}

fn invalid(message: &'static str) -> AppError {
    AppError::new(
        axum::http::StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_response_import",
        message,
    )
}

impl Report {
    fn issue(&mut self, path: String, message: &'static str) {
        self.issue_count += 1;
        if self.issues.len() < 128 {
            self.issues.push(ValidationIssue {
                path,
                message: message.into(),
            });
        }
    }
}

pub(super) fn prepare(
    definition: Definition,
    original: String,
    input: &[u8],
) -> Result<Plan, AppError> {
    let source = LssDocument::parse(original.as_bytes(), Limits::default())
        .map_err(|_| AppError::internal())?;
    let mapping = source_bindings(
        &definition,
        source.source_model().ok_or_else(AppError::internal)?,
    )
    .map_err(|_| AppError::internal())?;
    let source_languages = source
        .source_model()
        .ok_or_else(AppError::internal)?
        .languages
        .clone();
    drop(source);
    drop(original);
    drop(definition);
    let table = ResponseTable::parse(input, Limits::default()).map_err(|_| {
        invalid("Fichier LSR illisible, schéma non pris en charge ou limites dépassées.")
    })?;
    for column in &table.columns {
        if let Some((survey, rest)) = column.split_once('X')
            && !survey.is_empty()
            && survey.bytes().all(|byte| byte.is_ascii_digit())
            && rest.contains('X')
            && survey != mapping.survey_id
        {
            return Err(invalid(
                "Une colonne de réponse appartient à une autre étude que le LSS archivé.",
            ));
        }
    }
    let declared: BTreeSet<_> = table.columns.iter().cloned().collect();
    let mapped: BTreeSet<_> = mapping
        .fields
        .iter()
        .map(|field| field.column.clone())
        .collect();
    let mut report = Report {
        conservation: "available",
        mapping: "complete",
        execution: "unverified",
        continuity: "unavailable",
        row_count: table.rows.len(),
        submitted: 0,
        drafts: 0,
        unknown_status: 0,
        mapped_cells: mapping.fields.len(),
        unknown_column_count: 0,
        missing_column_count: 0,
        duplicate_ids: 0,
        invalid_ids: 0,
        issue_count: 0,
        issues: Vec::new(),
    };
    if table
        .languages
        .iter()
        .any(|language| !source_languages.contains(language))
    {
        report.issue(
            "languages".into(),
            "Une langue du fichier n'appartient pas au LSS archivé ; elle reste conservée.",
        );
    }
    report.issue_count += mapping.issue_count;
    for issue in mapping.issues {
        if report.issues.len() < 128 {
            report.issues.push(issue);
        }
    }
    for column in &table.columns {
        if !mapped.contains(column)
            && !TECHNICAL.contains(&column.as_str())
            && !mapping.ignored_columns.contains(column)
        {
            report.unknown_column_count += 1;
            report.issue(
                format!("columns.{column}"),
                "Colonne conservée sans correspondance de cellule connue.",
            );
        }
    }
    for column in mapped.difference(&declared) {
        if table.rows.is_empty() {
            break;
        }
        report.missing_column_count += 1;
        report.issue(
            format!("columns.{column}"),
            "Colonne attendue absente ; aucune valeur n'est inventée.",
        );
    }
    let schema = Schema {
        schema_version: 1,
        source_survey_id: mapping.survey_id,
        languages: table.languages,
        columns: table.columns,
        bindings: mapping.fields,
        ignored_columns: mapping.ignored_columns,
    };
    let mut ids = BTreeSet::new();
    let mut rows = Vec::new();
    for (ordinal, values) in table.rows.into_iter().enumerate() {
        rows.push(prepare_row(ordinal, values, &mut ids, &mut report)?);
    }
    if report.issue_count > 0 {
        report.mapping = "partial";
    }
    let schema = serde_json::to_string(&schema).map_err(|_| AppError::internal())?;
    if schema.len() > 16 * 1024 * 1024 {
        return Err(invalid("Le schéma de correspondance dépasse 16 Mio."));
    }
    Ok(Plan {
        schema,
        report: serde_json::to_string(&report).map_err(|_| AppError::internal())?,
        rows,
    })
}

fn prepare_row(
    ordinal: usize,
    values: ResponseRow,
    ids: &mut BTreeSet<String>,
    report: &mut Report,
) -> Result<Row, AppError> {
    let source_id = values
        .get("id")
        .and_then(Option::as_deref)
        .filter(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|id| id.parse::<u64>().ok())
        .filter(|id| *id > 0)
        .map(|id| id.to_string());
    if let Some(id) = &source_id {
        if !ids.insert(id.clone()) {
            report.duplicate_ids += 1;
            report.issue(
                format!("rows[{ordinal}].id"),
                "Identifiant source présent plusieurs fois ; toutes les lignes restent conservées.",
            );
        }
    } else {
        report.invalid_ids += 1;
        report.issue(
            format!("rows[{ordinal}].id"),
            "Identifiant source absent ou invalide ; aucune identité n'est inventée.",
        );
    }
    let status = match values.get("submitdate") {
        Some(Some(_)) => {
            report.submitted += 1;
            "submitted"
        }
        Some(None) => {
            report.drafts += 1;
            "draft"
        }
        None => {
            report.unknown_status += 1;
            report.issue(
                format!("rows[{ordinal}].submitdate"),
                "Colonne de soumission absente ; état historique inconnu.",
            );
            "unknown"
        }
    };
    let sparse: BTreeMap<_, _> = values
        .into_iter()
        .filter_map(|(key, value)| value.map(|value| (key, value)))
        .collect();
    let document = serde_json::to_string(&sparse).map_err(|_| AppError::internal())?;
    if document.len() > 16 * 1024 * 1024 {
        return Err(invalid("Une ligne source dépasse 16 Mio après encodage."));
    }
    let sha256 = super::super::runtime::hex(&Sha256::digest(document.as_bytes()));
    Ok(Row {
        ordinal: ordinal as i64,
        source_id,
        status,
        sha256,
        document,
    })
}
