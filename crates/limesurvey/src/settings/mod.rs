//! Resolution of the selected SurveysGroupsettings options, without local defaults.
mod resolve;
mod specification;
pub(crate) use resolve::inherits as inherits_value;

use crate::{LssDocument, xml::Document};
use rustsurvey_domain::ValidationIssue;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_SETTINGS_CONTEXT_BYTES: usize = 64 * 1024;
pub const GROUP_SETTINGS_PROFILE: &str = "limesurvey-group-settings-v1";

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsContext {
    pub schema_version: u32,
    pub survey_id: String,
    pub survey_group_id: String,
    pub groups: Vec<SettingsGroup>,
    #[serde(deserialize_with = "unique_values")]
    pub global: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsGroup {
    pub id: String,
    pub parent_id: Option<String>,
    #[serde(deserialize_with = "unique_values")]
    pub values: BTreeMap<String, Option<String>>,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SettingValue {
    Flag(bool),
    Integer(u32),
    Code(&'static str),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedSetting {
    pub name: &'static str,
    pub value: Option<SettingValue>,
    pub origin: Option<SettingOrigin>,
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingOrigin {
    pub layer: &'static str,
    pub group_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupSettingsReport {
    pub schema_version: u32,
    pub complete: bool,
    pub settings: Vec<ResolvedSetting>,
    pub issues: Vec<ValidationIssue>,
}

fn unique_values<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, Option<String>>, D::Error> {
    struct Values;
    impl<'de> serde::de::Visitor<'de> for Values {
        type Value = BTreeMap<String, Option<String>>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("unique setting names with string or null values")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut values = BTreeMap::new();
            while let Some((name, value)) = map.next_entry::<String, Option<String>>()? {
                if values.insert(name, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate setting name"));
                }
            }
            Ok(values)
        }
    }
    deserializer.deserialize_map(Values)
}

impl SettingsContext {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1
            || !identifier(&self.survey_id)
            || !identifier(&self.survey_group_id)
            || self.survey_id == "0"
            || self.groups.len() > 64
        {
            return Err("invalid settings context version, identity or group limit");
        }
        validate_values(&self.global)?;
        let mut ids = BTreeSet::new();
        for group in &self.groups {
            if !identifier(&group.id)
                || group.id == "0"
                || !ids.insert(&group.id)
                || group.parent_id.as_deref().is_some_and(|id| !identifier(id))
            {
                return Err("invalid or duplicate settings group identity");
            }
            validate_values(&group.values)?;
        }
        // Validate every supplied branch, including groups not reached by this survey.
        for group in &self.groups {
            let mut seen = BTreeSet::new();
            let mut current = Some(group.id.as_str());
            while let Some(id) = current.filter(|id| *id != "0") {
                if !seen.insert(id) {
                    return Err("cyclic settings group hierarchy");
                }
                let found = self
                    .groups
                    .iter()
                    .find(|item| item.id == id)
                    .ok_or("missing settings group ancestor")?;
                current = found.parent_id.as_deref();
            }
        }
        if self.survey_group_id != "0" && !ids.contains(&self.survey_group_id) {
            return Err("missing survey settings group");
        }
        Ok(())
    }
}

fn validate_values(values: &BTreeMap<String, Option<String>>) -> Result<(), &'static str> {
    for (name, value) in values {
        if !specification::SETTINGS.iter().any(|spec| spec.name == name)
            || value.as_ref().is_some_and(|value| value.len() > 64)
        {
            return Err("unknown setting or setting value exceeds 64 bytes");
        }
    }
    Ok(())
}

fn identifier(value: &str) -> bool {
    value.parse::<u64>().is_ok_and(|id| id.to_string() == value)
}

pub(super) fn inspect(
    document: &LssDocument,
    context: Option<&SettingsContext>,
) -> Result<GroupSettingsReport, &'static str> {
    if document.report().source_schema != crate::TARGET_SCHEMA || document.report().has_errors() {
        return Err("settings resolution requires a valid schema-623 survey");
    }
    let xml = &document.xml;
    let row = survey_row(xml).ok_or("missing survey settings row")?;
    if let Some(context) = context {
        context.validate()?;
        if document
            .source_model()
            .and_then(|source| source.survey_id)
            .map(|id| id.0.to_string())
            .as_deref()
            != Some(context.survey_id.as_str())
        {
            return Err("settings context belongs to another survey");
        }
        if let Some(group) = xml.field(row, "gsid")
            && group.parse::<u64>().ok() != context.survey_group_id.parse::<u64>().ok()
        {
            return Err("settings context belongs to another survey group");
        }
    }
    Ok(resolve::resolve(xml, row, context))
}

fn survey_row(xml: &Document) -> Option<usize> {
    let table = xml.children_named(xml.root, "surveys").next()?;
    let rows = xml.children_named(table, "rows").next()?;
    xml.children_named(rows, "row").next()
}
