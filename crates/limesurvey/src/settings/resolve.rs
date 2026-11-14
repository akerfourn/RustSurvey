use super::{
    GroupSettingsReport, ResolvedSetting, SettingOrigin, SettingsContext, specification::SETTINGS,
};
use crate::xml::Document;
use rustsurvey_domain::ValidationIssue;

pub(super) fn resolve(
    xml: &Document,
    row: usize,
    context: Option<&SettingsContext>,
) -> GroupSettingsReport {
    let mut result = GroupSettingsReport {
        schema_version: 1,
        complete: true,
        settings: Vec::new(),
        issues: Vec::new(),
    };
    for specification in SETTINGS {
        let name = specification.name;
        let (value, origin) = inherited(
            xml.field(row, name),
            name,
            context,
            xml.field(row, "gsid").and_then(|id| id.parse().ok()),
        );
        let parsed = value.and_then(|value| specification.parse(value));
        let status = if parsed.is_some() {
            "resolved"
        } else if value.is_none() || value.is_some_and(inherits) {
            "unresolved"
        } else {
            "invalid"
        };
        if parsed.is_none() {
            result.complete = false;
            result.issues.push(ValidationIssue {
                path: format!("source.surveys.{name}"),
                message: if status == "invalid" {
                    "Valeur source non admise pour ce réglage."
                } else {
                    "La valeur héritée requiert le contexte des groupes et réglages globaux source."
                }
                .into(),
            });
        }
        result.settings.push(ResolvedSetting {
            name,
            value: parsed,
            origin,
            status,
        });
    }
    result
}

fn inherited<'a>(
    survey_value: Option<&'a str>,
    name: &str,
    context: Option<&'a SettingsContext>,
    source_group: Option<u64>,
) -> (Option<&'a str>, Option<SettingOrigin>) {
    let mut value = survey_value;
    let mut origin = survey_value.map(|_| SettingOrigin {
        layer: "survey",
        group_id: None,
    });
    let Some(context) = context else {
        if source_group.is_none_or(|id| id == 0) || value.is_none_or(inherits) {
            return (None, None);
        }
        return (value, origin);
    };
    // The upstream initial call for group zero replaces even explicit survey values.
    if context.survey_group_id == "0" {
        return (
            context.global.get(name).and_then(Option::as_deref),
            Some(SettingOrigin {
                layer: "global",
                group_id: Some("0".into()),
            }),
        );
    }
    let mut current = Some(context.survey_group_id.as_str());
    while let Some(id) = current.filter(|id| *id != "0") {
        let Some(group) = context.groups.iter().find(|group| group.id == id) else {
            return (None, None);
        };
        if value.is_none_or(inherits) {
            value = group.values.get(name).and_then(Option::as_deref);
            origin = Some(SettingOrigin {
                layer: "group",
                group_id: Some(id.into()),
            });
        }
        current = group.parent_id.as_deref();
    }
    if value.is_none_or(inherits) {
        value = context.global.get(name).and_then(Option::as_deref);
        origin = Some(SettingOrigin {
            layer: "global",
            group_id: Some("0".into()),
        });
    }
    (value, origin)
}

pub(crate) fn inherits(value: &str) -> bool {
    if ["", "0", "I", "inherit"].contains(&value) {
        return true;
    }
    // PHP's loose numeric comparison also recognizes -1.0 and numeric whitespace.
    value
        .trim()
        .parse::<f64>()
        .is_ok_and(|number| number == -1.0)
}
