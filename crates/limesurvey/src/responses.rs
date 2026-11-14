//! Schema-623 response XML. An omitted row field is NULL; an absent column stays absent.
use crate::{Error, Limits, TARGET_SCHEMA, xml::Document, xml_write::Xml};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type ResponseRow = BTreeMap<String, Option<String>>;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResponseTable {
    pub languages: Vec<String>,
    pub columns: Vec<String>,
    pub rows: Vec<ResponseRow>,
}

fn child(xml: &Document, parent: usize, name: &str) -> Result<usize, Error> {
    let mut children = xml.children_named(parent, name);
    let value = children.next().ok_or(Error::InvalidStructure)?;
    if children.next().is_some() {
        return Err(Error::InvalidStructure);
    }
    Ok(value)
}

fn only_children(xml: &Document, parent: usize, allowed: &[&str]) -> Result<(), Error> {
    if !xml.elements[parent].text.trim().is_empty()
        || xml.elements[parent].children.iter().any(|id| {
            let element = &xml.elements[*id];
            !element.unqualified || !allowed.contains(&element.name.as_str())
        })
    {
        return Err(Error::InvalidStructure);
    }
    Ok(())
}

fn element_name(column: &str) -> Result<String, Error> {
    if column.is_empty()
        || column.len() > 256
        || !column
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_#".contains(&byte))
    {
        return Err(Error::InvalidMetadata("response column"));
    }
    let mut name = column.replace('#', "-");
    if column.as_bytes()[0].is_ascii_digit() || column.starts_with('_') {
        name.insert(0, '_');
    }
    Ok(name)
}

impl ResponseTable {
    /// Read a bounded LSR without guessing types or interpreting response values.
    pub fn parse(input: &[u8], limits: Limits) -> Result<Self, Error> {
        let xml = Document::parse(input, limits)?;
        if xml.elements.iter().any(|element| element.has_attributes) {
            return Err(Error::InvalidStructure);
        }
        let root = &xml.elements[xml.root];
        if root.name != "document" || !root.unqualified {
            return Err(Error::InvalidStructure);
        }
        only_children(
            &xml,
            xml.root,
            &["LimeSurveyDocType", "DBVersion", "languages", "responses"],
        )?;
        if xml.field(xml.root, "LimeSurveyDocType") != Some("Responses") {
            return Err(Error::InvalidMetadata("response document type"));
        }
        let schema = xml
            .field(xml.root, "DBVersion")
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or(Error::InvalidMetadata("response schema"))?;
        if schema != TARGET_SCHEMA {
            return Err(Error::UnsupportedSchema(schema));
        }
        let language_section = child(&xml, xml.root, "languages")?;
        only_children(&xml, language_section, &["language"])?;
        let mut languages = Vec::new();
        for id in xml.children_named(language_section, "language") {
            let item = &xml.elements[id];
            if !item.children.is_empty() || item.text.is_empty() || languages.contains(&item.text) {
                return Err(Error::InvalidMetadata("response language"));
            }
            languages.push(item.text.clone());
        }
        if languages.is_empty() || languages.len() > 32 {
            return Err(Error::InvalidMetadata("response languages"));
        }
        if xml.children_named(xml.root, "responses").next().is_none() {
            return Ok(Self {
                languages,
                columns: Vec::new(),
                rows: Vec::new(),
            });
        }
        let table = child(&xml, xml.root, "responses")?;
        only_children(&xml, table, &["fields", "rows"])?;
        let fields = child(&xml, table, "fields")?;
        only_children(&xml, fields, &["fieldname"])?;
        let mut columns = Vec::new();
        let mut names = BTreeMap::new();
        for id in xml.children_named(fields, "fieldname") {
            let item = &xml.elements[id];
            if !item.children.is_empty()
                || names
                    .insert(element_name(&item.text)?, item.text.clone())
                    .is_some()
            {
                return Err(Error::InvalidStructure);
            }
            columns.push(item.text.clone());
        }
        if columns.is_empty() {
            return Err(Error::InvalidStructure);
        }
        let row_section = child(&xml, table, "rows")?;
        only_children(&xml, row_section, &["row"])?;
        let mut rows = Vec::new();
        for id in xml.children_named(row_section, "row") {
            if rows.len().saturating_add(1).saturating_mul(columns.len()) > limits.max_elements {
                return Err(Error::LimitExceeded {
                    resource: "response cells",
                    limit: limits.max_elements,
                });
            }
            let mut row: ResponseRow = columns
                .iter()
                .map(|column| (column.clone(), None))
                .collect();
            let mut seen = BTreeSet::new();
            if !xml.elements[id].text.trim().is_empty() {
                return Err(Error::InvalidStructure);
            }
            for value_id in &xml.elements[id].children {
                let value = &xml.elements[*value_id];
                let column = names.get(&value.name).ok_or(Error::InvalidStructure)?;
                if !value.unqualified || !value.children.is_empty() || !seen.insert(column) {
                    return Err(Error::InvalidStructure);
                }
                row.insert(column.clone(), Some(value.text.clone()));
            }
            rows.push(row);
        }
        Ok(Self {
            languages,
            columns,
            rows,
        })
    }

    /// Write only supplied columns; missing row values become NULL, never an empty string.
    pub fn to_xml(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        let mut xml = Xml::compact(limits)?;
        xml.start("document")?;
        xml.field("LimeSurveyDocType", "Responses")?;
        xml.field("DBVersion", &TARGET_SCHEMA.to_string())?;
        xml.start("languages")?;
        for language in &self.languages {
            xml.field("language", language)?;
        }
        xml.end("languages")?;
        if self.columns.is_empty() && self.rows.is_empty() {
            xml.end("document")?;
            let output = xml.finish();
            Self::parse(&output, limits)?;
            return Ok(output);
        }
        xml.start("responses")?;
        xml.start("fields")?;
        let mut names = BTreeMap::new();
        for column in &self.columns {
            if names.insert(column, element_name(column)?).is_some() {
                return Err(Error::InvalidStructure);
            }
            xml.field("fieldname", column)?;
        }
        xml.end("fields")?;
        xml.start("rows")?;
        for row in &self.rows {
            if row.keys().any(|column| !names.contains_key(column)) {
                return Err(Error::InvalidStructure);
            }
            xml.start("row")?;
            for column in &self.columns {
                if let Some(Some(value)) = row.get(column) {
                    xml.field(&names[column], value)?;
                }
            }
            xml.end("row")?;
        }
        xml.end("rows")?;
        xml.end("responses")?;
        xml.end("document")?;
        let output = xml.finish();
        Self::parse(&output, limits)?;
        Ok(output)
    }
}
