use super::{EvaluationBudget, Scalar, value::compare};
use std::{cmp::Ordering, collections::BTreeSet};

pub(super) fn arity(name: &str) -> Option<(usize, usize)> {
    match name {
        "abs" | "acos" | "asin" | "atan" | "ceil" | "cos" | "exp" | "floor" | "sin" | "sqrt"
        | "tan" | "is_empty" | "is_null" | "is_string" | "is_numeric" | "strlen" | "strtolower"
        | "strtoupper" | "trim" | "ltrim" | "rtrim" | "strrev" => Some((1, 1)),
        "atan2" | "pow" | "strcmp" | "strcasecmp" | "str_repeat" => Some((2, 2)),
        "pi" => Some((0, 0)),
        "log" | "round" => Some((1, 2)),
        "if" | "substr" | "strpos" | "stripos" | "strstr" => Some((2, 3)),
        "str_replace" => Some((3, 3)),
        "count" => Some((0, 512)),
        "sum" | "min" | "max" | "join" | "implode" | "list" | "unique" => Some((1, 512)),
        "countif" => Some((2, 512)),
        "countifop" | "sumifop" => Some((3, 512)),
        _ => None,
    }
}

pub(super) fn empty(value: &Scalar) -> bool {
    matches!(value, Scalar::Null) || matches!(value, Scalar::String(text) if text.is_empty())
}

pub(super) fn evaluate(
    name: &str,
    values: &[Scalar],
    budget: &mut EvaluationBudget,
) -> Result<Scalar, &'static str> {
    let first = values.first().unwrap_or(&Scalar::Null);
    let boolean = match name {
        "is_empty" => Some(empty(first)),
        "is_null" => Some(matches!(first, Scalar::Null)),
        "is_string" => Some(matches!(first, Scalar::String(_))),
        "is_numeric" => {
            Some(!empty(first) && !matches!(first, Scalar::Boolean(_)) && first.numeric().is_ok())
        }
        "unique" => {
            let mut seen = BTreeSet::new();
            Some(
                values
                    .iter()
                    .filter(|value| !empty(value))
                    .all(|value| seen.insert(value.text())),
            )
        }
        _ => None,
    };
    if let Some(value) = boolean {
        return Ok(Scalar::Boolean(value));
    }
    match name {
        "count" => {
            return Ok(Scalar::Number(
                values.iter().filter(|value| !empty(value)).count() as f64,
            ));
        }
        "countif" | "countifop" | "sumifop" => return aggregate(name, values),
        "sum" => return sum(values.iter()),
        "min" | "max" => {
            let mut best = first;
            for value in &values[1..] {
                let order = compare(value, best);
                if (name == "min" && order == Ordering::Less)
                    || (name == "max" && order == Ordering::Greater)
                {
                    best = value;
                }
            }
            return Ok(best.clone());
        }
        _ => {}
    }
    if let Some(value) = math(name, values)? {
        return Scalar::Number(value).checked();
    }
    super::text_functions::evaluate(name, values, budget)
}

fn sum<'a>(values: impl Iterator<Item = &'a Scalar>) -> Result<Scalar, &'static str> {
    let mut total = 0.0;
    for value in values {
        total += value.numeric()?;
    }
    Scalar::Number(total).checked()
}

fn matches(operator: &str, left: &Scalar, right: &Scalar) -> Result<bool, &'static str> {
    let order = compare(left, right);
    match operator {
        "==" | "eq" => Ok(order == Ordering::Equal),
        "!=" | "ne" => Ok(order != Ordering::Equal),
        "<" | "lt" => Ok(order == Ordering::Less),
        "<=" | "le" => Ok(order != Ordering::Greater),
        ">" | "gt" => Ok(order == Ordering::Greater),
        ">=" | "ge" => Ok(order != Ordering::Less),
        _ => Err("Comparateur de fonction non pris en charge."),
    }
}

fn aggregate(name: &str, values: &[Scalar]) -> Result<Scalar, &'static str> {
    let (operator, target, candidates) = if name == "countif" {
        ("==".to_owned(), &values[0], &values[1..])
    } else {
        (values[0].text(), &values[1], &values[2..])
    };
    let mut matching = Vec::new();
    for value in candidates {
        if name != "countif" && empty(value) {
            continue;
        }
        if matches(&operator, value, target)? {
            matching.push(value);
        }
    }
    if name == "sumifop" {
        return sum(matching.into_iter());
    }
    Ok(Scalar::Number(matching.len() as f64))
}

fn math(name: &str, values: &[Scalar]) -> Result<Option<f64>, &'static str> {
    if name == "pi" {
        return Ok(Some(std::f64::consts::PI));
    }
    if ![
        "abs", "acos", "asin", "atan", "ceil", "cos", "exp", "floor", "sin", "sqrt", "tan",
        "atan2", "pow", "log", "round",
    ]
    .contains(&name)
    {
        return Ok(None);
    }
    let value = values[0].numeric()?;
    let result = match name {
        "abs" => value.abs(),
        "acos" => value.acos(),
        "asin" => value.asin(),
        "atan" => value.atan(),
        "ceil" => value.ceil(),
        "cos" => value.cos(),
        "exp" => value.exp(),
        "floor" => value.floor(),
        "sin" => value.sin(),
        "sqrt" => value.sqrt(),
        "tan" => value.tan(),
        "atan2" => value.atan2(values[1].numeric()?),
        "pow" => value.powf(values[1].numeric()?),
        "log" if values.len() == 2 => value.ln() / values[1].numeric()?.ln(),
        "log" => value.ln(),
        "round" => {
            let precision = values.get(1).unwrap_or(&Scalar::Number(0.0)).numeric()?;
            if precision.fract() != 0.0 || precision.abs() > 15.0 {
                return Err("Précision d’arrondi limitée à 15 chiffres.");
            }
            let scale = 10_f64.powf(precision);
            value.signum() * (value.abs() * scale + f64::EPSILON).round() / scale
        }
        _ => return Err("Fonction non prise en charge."),
    };
    Ok(Some(result))
}
