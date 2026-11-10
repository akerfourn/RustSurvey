use super::tables::rows;
use crate::{
    Error, Limits,
    xml::{self, Document},
};

pub(crate) fn replace_title(
    xml: &Document,
    language: &str,
    title: &str,
    limits: Limits,
) -> Result<String, Error> {
    if title.len() > limits.max_bytes {
        return Err(Error::LimitExceeded {
            resource: "output bytes",
            limit: limits.max_bytes,
        });
    }
    xml::validate_characters(title)?;
    let mut matching = rows(xml, "surveys_languagesettings")
        .into_iter()
        .filter(|&row| xml.field(row, "surveyls_language") == Some(language));
    let row = matching.next().ok_or(Error::TitleNotFound)?;
    if matching.next().is_some() {
        return Err(Error::TitleNotFound);
    }
    let mut titles = xml.children_named(row, "surveyls_title");
    let id = titles.next().ok_or(Error::TitleNotFound)?;
    if titles.next().is_some() {
        return Err(Error::TitleNotFound);
    }
    if xml.field(row, "surveyls_title") == Some(title) {
        return Ok(xml.source.clone());
    }
    let element = &xml.elements[id];
    let (span, markup_bytes) = match &element.content {
        Some(content) => (content.clone(), 0),
        None => (
            element.span.clone(),
            element.span.len() - 2 + "></surveyls_title>".len(),
        ),
    };
    let mut size = xml.source.len() - span.len() + markup_bytes;
    if size > limits.max_bytes {
        return Err(Error::LimitExceeded {
            resource: "output bytes",
            limit: limits.max_bytes,
        });
    }
    for character in title.chars() {
        let bytes = match character {
            '&' | '\r' => 5,
            '<' | '>' => 4,
            '"' | '\'' => 6,
            _ => character.len_utf8(),
        };
        size = size.checked_add(bytes).ok_or(Error::LimitExceeded {
            resource: "output bytes",
            limit: limits.max_bytes,
        })?;
        if size > limits.max_bytes {
            return Err(Error::LimitExceeded {
                resource: "output bytes",
                limit: limits.max_bytes,
            });
        }
    }
    let escaped = quick_xml::escape::escape(title).replace('\r', "&#13;");
    let replacement = match &element.content {
        Some(_) => escaped,
        None => format!(
            "{}>{escaped}</surveyls_title>",
            &xml.source[element.span.start..element.span.end - 2]
        ),
    };
    let mut updated = String::with_capacity(size);
    updated.push_str(&xml.source[..span.start]);
    updated.push_str(&replacement);
    updated.push_str(&xml.source[span.end..]);
    Ok(updated)
}
