use super::layout::{Layout, NativeQuestion};
use crate::{Error, xml_write::Xml};
use rustsurvey_domain::{
    cells::{Coordinate, Role},
    defaults::DefaultValue,
};
use std::collections::{BTreeMap, BTreeSet};

struct NativeDefault<'a> {
    id: usize,
    question_id: u64,
    subquestion_id: u64,
    special_type: &'static str,
    values: Vec<&'a DefaultValue>,
}

pub(super) fn write(xml: &mut Xml, layout: &Layout<'_>) -> Result<(), Error> {
    let rows = rows(layout)?;
    if rows.is_empty() {
        return Ok(());
    }
    xml.table(
        "defaultvalues",
        &["dvid", "qid", "sqid", "scale_id", "specialtype"],
    )?;
    for row in &rows {
        xml.row(&[
            &row.id.to_string(),
            &row.question_id.to_string(),
            &row.subquestion_id.to_string(),
            "0",
            row.special_type,
        ])?;
    }
    xml.end_table("defaultvalues")?;
    xml.table(
        "defaultvalue_l10ns",
        &["id", "dvid", "language", "defaultvalue"],
    )?;
    let mut translation_id = 1;
    for row in rows {
        for value in row.values {
            xml.row(&[
                &translation_id.to_string(),
                &row.id.to_string(),
                &value.language,
                &value.value,
            ])?;
            translation_id += 1;
        }
    }
    xml.end_table("defaultvalue_l10ns")
}

fn rows<'a>(layout: &Layout<'a>) -> Result<Vec<NativeDefault<'a>>, Error> {
    let languages: BTreeSet<_> = layout.languages.iter().copied().collect();
    let mut rows = Vec::new();
    for question in &layout.questions {
        let Some(defaults) = &question.source.defaults else {
            continue;
        };
        let mut targets = BTreeMap::new();
        for value in &defaults.values {
            if languages.contains(value.language.as_str()) {
                targets
                    .entry((&value.coordinate, value.role))
                    .or_insert_with(Vec::new)
                    .push(value);
            }
        }
        for ((coordinate, role), mut values) in targets {
            values.sort_by(|left, right| left.language.cmp(&right.language));
            let (subquestion_id, special_type) = target(question, coordinate, role)?;
            rows.push(NativeDefault {
                id: rows.len() + 1,
                question_id: question.id,
                subquestion_id,
                special_type,
                values,
            });
        }
    }
    Ok(rows)
}

fn target(
    question: &NativeQuestion<'_>,
    coordinate: &Coordinate,
    role: Role,
) -> Result<(u64, &'static str), Error> {
    match (coordinate, role) {
        (Coordinate::Primary, Role::Answer) => Ok((0, "")),
        (Coordinate::Primary, Role::Other) => Ok((0, "other")),
        (Coordinate::Subquestion { id, scale: None }, Role::Answer) => question
            .subquestions
            .iter()
            .find(|row| row.source.id == *id)
            .map(|row| (row.id, ""))
            .ok_or(Error::InvalidStructure),
        _ => Err(Error::InvalidStructure),
    }
}
