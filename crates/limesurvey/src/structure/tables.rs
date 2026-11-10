use super::fields::issue;
use crate::{DiagnosticCode, Report, xml::Document};
use std::collections::BTreeSet;

pub(super) fn rows(xml: &Document, section: &str) -> Vec<usize> {
    let Some(table) = xml.children_named(xml.root, section).next() else {
        return Vec::new();
    };
    let Some(rows) = xml.children_named(table, "rows").next() else {
        return Vec::new();
    };
    xml.children_named(rows, "row").collect()
}

pub(super) fn validate_table(xml: &Document, section: &str, report: &mut Report) {
    let tables: Vec<usize> = xml.children_named(xml.root, section).collect();
    if tables.len() > 1 {
        issue(
            report,
            DiagnosticCode::InvalidTable,
            section,
            "duplicate table section",
        );
    }
    let Some(&table) = tables.first() else {
        return;
    };
    if xml.elements[table].children.is_empty() {
        return;
    }
    let fields: Vec<usize> = xml.children_named(table, "fields").collect();
    let row_sets: Vec<usize> = xml.children_named(table, "rows").collect();
    if fields.len() != 1 || row_sets.len() != 1 {
        issue(
            report,
            DiagnosticCode::InvalidTable,
            section,
            "expected one fields element and one rows element",
        );
        return;
    }
    let mut declared = BTreeSet::new();
    for &id in &xml.elements[fields[0]].children {
        let field = &xml.elements[id];
        if !field.unqualified
            || field.name != "fieldname"
            || !field.children.is_empty()
            || field.text.is_empty()
            || !declared.insert(field.text.as_str())
        {
            issue(
                report,
                DiagnosticCode::InvalidTable,
                section,
                "invalid or duplicate field declaration",
            );
        }
    }
    for (index, &id) in xml.elements[row_sets[0]].children.iter().enumerate() {
        validate_row(
            xml,
            id,
            &declared,
            &format!("{section}.rows[{index}]"),
            report,
        );
    }
}

fn validate_row(
    xml: &Document,
    id: usize,
    declared: &BTreeSet<&str>,
    path: &str,
    report: &mut Report,
) {
    let row = &xml.elements[id];
    if !row.unqualified || row.name != "row" || !row.text.trim().is_empty() {
        issue(
            report,
            DiagnosticCode::InvalidTable,
            path,
            "expected a row of scalar fields",
        );
        return;
    }
    let mut names = BTreeSet::new();
    for &id in &row.children {
        let field = &xml.elements[id];
        if !field.unqualified
            || !declared.contains(field.name.as_str())
            || !names.insert(&field.name)
            || !field.children.is_empty()
        {
            issue(
                report,
                DiagnosticCode::InvalidField,
                path,
                "row contains an undeclared, duplicate, namespaced or non-scalar field",
            );
        }
    }
}
