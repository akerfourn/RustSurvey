pub(crate) mod bindings;
mod defaults;
pub(crate) mod layout;
mod tables;
mod validate;

use crate::{Error, Limits, LssDocument, SourceId};
use rustsurvey_domain::navigation::NavigationSettings;
use rustsurvey_domain::{Definition, ValidationIssue};
use std::fmt;

pub struct NativeExport {
    pub xml: Vec<u8>,
    pub bindings: Vec<FieldBinding>,
}

pub use bindings::FieldBinding;

pub fn native_bindings(
    definition: &Definition,
    survey_id: SourceId,
) -> Result<Vec<FieldBinding>, NativeExportError> {
    validate::definition(definition, survey_id)?;
    bindings::build(&layout::Layout::new(definition, survey_id))
}

#[derive(Debug)]
pub enum NativeExportError {
    Unsupported(Vec<ValidationIssue>),
    Format(Error),
}

impl fmt::Display for NativeExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(_) => formatter.write_str(
                "native definition cannot be exported without changing unsupported data",
            ),
            Self::Format(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for NativeExportError {}
impl From<Error> for NativeExportError {
    fn from(error: Error) -> Self {
        Self::Format(error)
    }
}

/// Create a schema-623 structure; original imported LSS documents use surgical edits instead.
/// The survey ID is an exchange alias; LimeSurvey may reassign it when importing.
pub fn export_native(
    definition: &Definition,
    survey_id: SourceId,
    limits: Limits,
) -> Result<NativeExport, NativeExportError> {
    export_native_with_navigation(definition, survey_id, NavigationSettings::default(), limits)
}

/// Export the presentation frozen by a native campaign, independently of its answer cells.
pub fn export_native_with_navigation(
    definition: &Definition,
    survey_id: SourceId,
    navigation: NavigationSettings,
    limits: Limits,
) -> Result<NativeExport, NativeExportError> {
    validate::definition(definition, survey_id)?;
    let mut layout = layout::Layout::new(definition, survey_id);
    layout.navigation = navigation;
    let mut xml = crate::xml_write::Xml::new(limits)?;
    xml.start("document")?;
    xml.field("LimeSurveyDocType", "Survey")?;
    xml.field("DBVersion", &crate::TARGET_SCHEMA.to_string())?;
    xml.start("languages")?;
    for language in &layout.languages {
        xml.field("language", language)?;
    }
    xml.end("languages")?;
    tables::write(&mut xml, &layout)?;
    xml.end("document")?;
    let bytes = xml.finish();
    let checked = LssDocument::parse(&bytes, limits)?;
    if checked.report().has_errors() {
        return Err(Error::InvalidStructure.into());
    }
    Ok(NativeExport {
        xml: bytes,
        bindings: bindings::build(&layout)?,
    })
}
