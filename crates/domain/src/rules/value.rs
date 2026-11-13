use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub const MAX_STRING_UNITS: usize = 65_536;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum Scalar {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
}

/// Shared by all expressions in one questionnaire evaluation.
pub struct EvaluationBudget {
    remaining: usize,
}

impl Default for EvaluationBudget {
    fn default() -> Self {
        Self::new(1_000_000)
    }
}

impl EvaluationBudget {
    pub fn new(units: usize) -> Self {
        Self { remaining: units }
    }

    pub(crate) fn charge(&mut self, units: usize) -> Result<(), &'static str> {
        self.remaining = self
            .remaining
            .checked_sub(units)
            .ok_or("Budget d’évaluation dépassé.")?;
        Ok(())
    }
}

impl Scalar {
    pub fn truth(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Boolean(value) => *value,
            Self::Number(value) => value.is_finite() && *value != 0.0,
            Self::String(value) => !value.is_empty() && value != "0",
        }
    }

    pub fn numeric(&self) -> Result<f64, &'static str> {
        match self {
            Self::Null => Ok(0.0),
            Self::Boolean(value) => Ok(u8::from(*value).into()),
            Self::Number(value) => finite(*value),
            Self::String(value) if value.is_empty() => Ok(0.0),
            Self::String(value) => {
                let text = value.trim_matches(super::syntax::is_whitespace);
                if text.is_empty() || text.bytes().any(|byte| !b"0123456789+-.eE".contains(&byte)) {
                    return Err("Une règle attend un nombre.");
                }
                finite(
                    text.parse::<f64>()
                        .map_err(|_| "Une règle attend un nombre.")?,
                )
            }
        }
    }

    pub fn text(&self) -> String {
        match self {
            Self::Null | Self::Boolean(false) => String::new(),
            Self::Boolean(true) => "1".to_owned(),
            Self::String(value) => value.clone(),
            Self::Number(value) if *value == 0.0 => "0".to_owned(),
            Self::Number(value) if value.abs() >= 1e21 || value.abs() < 1e-6 => {
                let scientific = format!("{value:e}");
                let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
                if exponent.starts_with('-') {
                    scientific
                } else {
                    format!("{mantissa}e+{exponent}")
                }
            }
            Self::Number(value) => value.to_string(),
        }
    }

    pub(super) fn checked(self) -> Result<Self, &'static str> {
        match &self {
            Self::Number(value) => {
                finite(*value)?;
            }
            Self::String(value) if value.encode_utf16().count() > MAX_STRING_UNITS => {
                return Err("Texte calculé trop long.");
            }
            _ => {}
        }
        Ok(self)
    }
}

pub(super) fn finite(value: f64) -> Result<f64, &'static str> {
    if !value.is_finite() {
        return Err("Résultat numérique non fini.");
    }
    Ok(value)
}

pub(super) fn compare(left: &Scalar, right: &Scalar) -> Ordering {
    if let (Ok(left), Ok(right)) = (left.numeric(), right.numeric()) {
        return left.partial_cmp(&right).unwrap_or(Ordering::Equal);
    }
    left.text().encode_utf16().cmp(right.text().encode_utf16())
}

pub(super) fn equal(left: &Scalar, right: &Scalar) -> bool {
    if matches!(left, Scalar::Boolean(_)) || matches!(right, Scalar::Boolean(_)) {
        return left.truth() == right.truth();
    }
    if left == right {
        return true;
    }
    let numeric_text = match (left, right) {
        (Scalar::Number(_), Scalar::String(value)) | (Scalar::String(value), Scalar::Number(_)) => {
            !value.trim_matches(super::syntax::is_whitespace).is_empty()
        }
        _ => false,
    };
    numeric_text && matches!((left.numeric(), right.numeric()), (Ok(a), Ok(b)) if a == b)
}
