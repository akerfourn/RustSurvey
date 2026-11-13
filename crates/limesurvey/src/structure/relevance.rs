use super::tables::rows;
use crate::{
    Error, Limits, RelevanceEdit, RelevanceTarget,
    xml::{self, Document},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

struct Replacement {
    span: Range<usize>,
    value: String,
}

pub(crate) fn replace(
    xml: &Document,
    edits: &[RelevanceEdit],
    limits: Limits,
) -> Result<String, Error> {
    if edits.len() > 4096 {
        return Err(Error::LimitExceeded {
            resource: "relevance edits",
            limit: 4096,
        });
    }
    if edits
        .iter()
        .try_fold(0_usize, |size, edit| {
            size.checked_add(edit.expression.len())
        })
        .is_none_or(|size| size > limits.max_bytes)
    {
        return Err(Error::LimitExceeded {
            resource: "relevance bytes",
            limit: limits.max_bytes,
        });
    }
    let mut targets = BTreeMap::new();
    for (target, section, key, field) in [
        (RelevanceTarget::Group, "groups", "gid", "grelevance"),
        (RelevanceTarget::Question, "questions", "qid", "relevance"),
    ] {
        for row in rows(xml, section) {
            if let Some(id) = xml
                .field(row, key)
                .and_then(|value| value.parse::<u64>().ok())
            {
                targets.insert((target, id), (row, section, field));
            }
        }
    }
    let mut seen = BTreeSet::new();
    let mut declarations = BTreeSet::new();
    let mut replacements = Vec::new();
    for edit in edits {
        if !seen.insert((edit.target, edit.id.0)) {
            return Err(Error::InvalidRelevanceEdit);
        }
        xml::validate_characters(&edit.expression)?;
        let &(row, section, field) = targets
            .get(&(edit.target, edit.id.0))
            .ok_or(Error::InvalidRelevanceEdit)?;
        if xml.field(row, field).unwrap_or("1") == edit.expression {
            continue;
        }
        let escaped = quick_xml::escape::escape(&edit.expression).replace('\r', "&#13;");
        if let Some(element) = xml.children_named(row, field).next() {
            replacements.push(replace_content(xml, element, &escaped)?);
        } else {
            let position = xml.elements[row]
                .content
                .as_ref()
                .ok_or(Error::InvalidRelevanceEdit)?
                .end;
            replacements.push(Replacement {
                span: position..position,
                value: format!("<{field}>{escaped}</{field}>"),
            });
            declarations.insert((section, field));
        }
    }
    if replacements.is_empty() {
        return Ok(xml.source.clone());
    }
    // Legacy conditions can regenerate relevance on import. Never export contradictory rule systems.
    if xml.children_named(xml.root, "conditions").any(|table| {
        xml.children_named(table, "rows")
            .any(|rows| xml.children_named(rows, "row").next().is_some())
    }) {
        return Err(Error::LegacyConditionsPreventEdit);
    }
    for (section, name) in declarations {
        let table = xml
            .children_named(xml.root, section)
            .next()
            .ok_or(Error::InvalidRelevanceEdit)?;
        let fields = xml
            .children_named(table, "fields")
            .next()
            .ok_or(Error::InvalidRelevanceEdit)?;
        if xml
            .children_named(fields, "fieldname")
            .any(|id| xml.elements[id].text == name)
        {
            continue;
        }
        replacements.push(append_content(
            xml,
            fields,
            &format!("<fieldname>{name}</fieldname>"),
        )?);
    }
    replacements.sort_by_key(|edit| edit.span.start);
    let mut removed = 0;
    let mut added = 0;
    let mut end = 0;
    for edit in &replacements {
        if edit.span.start < end {
            return Err(Error::InvalidRelevanceEdit);
        }
        end = edit.span.end;
        removed += edit.span.len();
        added += edit.value.len();
    }
    let size = xml
        .source
        .len()
        .checked_sub(removed)
        .and_then(|size| size.checked_add(added))
        .filter(|size| *size <= limits.max_bytes)
        .ok_or(Error::LimitExceeded {
            resource: "output bytes",
            limit: limits.max_bytes,
        })?;
    let mut updated = String::with_capacity(size);
    let mut cursor = 0;
    for edit in replacements {
        updated.push_str(&xml.source[cursor..edit.span.start]);
        updated.push_str(&edit.value);
        cursor = edit.span.end;
    }
    updated.push_str(&xml.source[cursor..]);
    Ok(updated)
}

fn replace_content(xml: &Document, id: usize, value: &str) -> Result<Replacement, Error> {
    let element = &xml.elements[id];
    if !element.children.is_empty() {
        return Err(Error::InvalidRelevanceEdit);
    }
    if let Some(content) = &element.content {
        return Ok(Replacement {
            span: content.clone(),
            value: value.to_owned(),
        });
    }
    Ok(Replacement {
        span: element.span.clone(),
        value: format!(
            "{}>{value}</{}>",
            &xml.source[element.span.start..element.span.end - 2],
            element.name
        ),
    })
}

fn append_content(xml: &Document, id: usize, value: &str) -> Result<Replacement, Error> {
    if let Some(content) = &xml.elements[id].content {
        return Ok(Replacement {
            span: content.end..content.end,
            value: value.to_owned(),
        });
    }
    replace_content(xml, id, value)
}
