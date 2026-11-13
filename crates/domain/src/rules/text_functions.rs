use super::{
    EvaluationBudget, Scalar, functions::empty, syntax::is_whitespace, value::MAX_STRING_UNITS,
};
use std::cmp::Ordering;

pub(super) fn evaluate(
    name: &str,
    values: &[Scalar],
    budget: &mut EvaluationBudget,
) -> Result<Scalar, &'static str> {
    if ["join", "implode", "list"].contains(&name) {
        return joined(name, values, budget);
    }
    let text = values[0].text();
    let value = match name {
        "strlen" => return Ok(Scalar::Number(text.chars().count() as f64)),
        "strtolower" => text.to_lowercase(),
        "strtoupper" => text.to_uppercase(),
        "trim" => text.trim_matches(is_whitespace).to_owned(),
        "ltrim" => text.trim_start_matches(is_whitespace).to_owned(),
        "rtrim" => text.trim_end_matches(is_whitespace).to_owned(),
        "strrev" => {
            if !text.is_ascii() {
                return Err("strrev est limité aux caractères ASCII dans cet aperçu.");
            }
            text.chars().rev().collect()
        }
        "substr" => substring(&text, values)?,
        "strcmp" | "strcasecmp" => {
            let mut left = text;
            let mut right = values[1].text();
            if name == "strcasecmp" {
                left = left.to_lowercase();
                right = right.to_lowercase();
            }
            let order = left.encode_utf16().cmp(right.encode_utf16());
            let result = match order {
                Ordering::Less => -1.0,
                Ordering::Equal => 0.0,
                Ordering::Greater => 1.0,
            };
            return Ok(Scalar::Number(result));
        }
        "strpos" | "stripos" => return position(name, text, values),
        "strstr" => {
            let Some(position) = text.find(&values[1].text()) else {
                return Ok(Scalar::Boolean(false));
            };
            if values.get(2).is_some_and(Scalar::truth) {
                text[..position].to_owned()
            } else {
                text[position..].to_owned()
            }
        }
        "str_repeat" => {
            let count = values[1].numeric()?;
            if count.fract() != 0.0 || !(0.0..=MAX_STRING_UNITS as f64).contains(&count) {
                return Err("Répétition de texte trop grande.");
            }
            let count = count as usize;
            if text.encode_utf16().count().saturating_mul(count) > MAX_STRING_UNITS {
                return Err("Répétition de texte trop grande.");
            }
            budget.charge(text.len().saturating_mul(count))?;
            text.repeat(count)
        }
        "str_replace" => {
            let replacement = values[1].text();
            let source = values[2].text();
            if text.is_empty() {
                source
            } else {
                let count = source.matches(&text).count();
                let units = source.encode_utf16().count() - count * text.encode_utf16().count();
                if units.saturating_add(count.saturating_mul(replacement.encode_utf16().count()))
                    > MAX_STRING_UNITS
                {
                    return Err("Texte calculé trop long.");
                }
                budget.charge(
                    source
                        .len()
                        .saturating_add(count.saturating_mul(replacement.len())),
                )?;
                source.replace(&text, &replacement)
            }
        }
        _ => return Err("Fonction non prise en charge."),
    };
    Scalar::String(value).checked()
}

fn substring(text: &str, values: &[Scalar]) -> Result<String, &'static str> {
    let characters: Vec<_> = text.chars().collect();
    let offset = values[1].numeric()?.trunc();
    let start = if offset < 0.0 {
        (characters.len() as f64 + offset).max(0.0)
    } else {
        offset
    };
    let start = start.min(characters.len() as f64) as usize;
    let end = if let Some(length) = values.get(2) {
        let length = length.numeric()?.trunc();
        let end = if length < 0.0 {
            characters.len() as f64 + length
        } else {
            start as f64 + length
        };
        end.clamp(start as f64, characters.len() as f64) as usize
    } else {
        characters.len()
    };
    Ok(characters[start..end].iter().collect())
}

fn position(name: &str, mut text: String, values: &[Scalar]) -> Result<Scalar, &'static str> {
    let mut needle = values[1].text();
    if name == "stripos" {
        text = text.to_lowercase();
        needle = needle.to_lowercase();
    }
    let length = text.chars().count();
    let mut offset = values
        .get(2)
        .unwrap_or(&Scalar::Number(0.0))
        .numeric()?
        .trunc();
    if offset < 0.0 {
        offset += length as f64;
    }
    if offset < 0.0 || offset > length as f64 {
        return Ok(Scalar::Boolean(false));
    }
    let start = text
        .char_indices()
        .nth(offset as usize)
        .map_or(text.len(), |(index, _)| index);
    let Some(position) = text[start..].find(&needle) else {
        return Ok(Scalar::Boolean(false));
    };
    Ok(Scalar::Number(
        text[..start + position].chars().count() as f64
    ))
}

fn joined(
    name: &str,
    values: &[Scalar],
    budget: &mut EvaluationBudget,
) -> Result<Scalar, &'static str> {
    let mut separator = String::new();
    let mut items = values;
    if name == "implode" {
        separator = values[0].text();
        items = &values[1..];
    } else if name == "list" {
        separator = ", ".to_owned();
    }
    let strings: Vec<_> = items
        .iter()
        .filter(|value| name != "list" || !empty(value))
        .map(Scalar::text)
        .collect();
    let separators = strings.len().saturating_sub(1);
    let units = strings
        .iter()
        .map(|text| text.encode_utf16().count())
        .sum::<usize>()
        + separator.encode_utf16().count() * separators;
    if units > MAX_STRING_UNITS {
        return Err("Texte calculé trop long.");
    }
    budget.charge(strings.iter().map(String::len).sum::<usize>() + separator.len() * separators)?;
    Ok(Scalar::String(strings.join(&separator)))
}
