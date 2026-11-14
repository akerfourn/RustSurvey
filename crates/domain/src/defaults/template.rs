use crate::rules::{EvaluationBudget, Expression, Scalar};
use std::collections::BTreeSet;

enum Part {
    Literal(String),
    Expression(Expression),
}

pub struct Template {
    parts: Vec<Part>,
    pub variables: BTreeSet<String>,
}

impl Template {
    pub fn compile(source: &str) -> Result<Self, &'static str> {
        if source.encode_utf16().count() > 4096 {
            return Err("Valeur par défaut limitée à 4096 unités UTF-16.");
        }
        let mut parts = Vec::new();
        let mut variables = BTreeSet::new();
        let mut remaining = source;
        let mut expressions = 0;
        while let Some(open) = remaining.find('{') {
            if open > 0 {
                parts.push(Part::Literal(remaining[..open].into()));
            }
            remaining = &remaining[open + 1..];
            let close = expression_end(remaining)?;
            let expression = Expression::compile(&remaining[..close])?;
            variables.extend(expression.variables.iter().cloned());
            parts.push(Part::Expression(expression));
            expressions += 1;
            if expressions > 100 {
                return Err("Maximum de 100 expressions par défaut.");
            }
            remaining = &remaining[close + 1..];
        }
        parts.push(Part::Literal(remaining.into()));
        Ok(Self { parts, variables })
    }

    pub fn literal(&self) -> Option<&str> {
        if let [Part::Literal(value)] = self.parts.as_slice() {
            return Some(value);
        }
        None
    }

    pub fn render(
        &self,
        lookup: &impl Fn(&str) -> Scalar,
        budget: &mut EvaluationBudget,
    ) -> Result<String, &'static str> {
        let mut output = String::new();
        let mut units = 0;
        for part in &self.parts {
            let value = match part {
                Part::Literal(value) => value.clone(),
                Part::Expression(expression) => {
                    expression.evaluate_with_budget(lookup, budget)?.text()
                }
            };
            let size = value.encode_utf16().count();
            budget.charge(size.max(1))?;
            units += size;
            if units > crate::rules::MAX_STRING_UNITS {
                return Err("Résultat de valeur par défaut trop long.");
            }
            output.push_str(&value);
        }
        Ok(output)
    }
}

fn expression_end(source: &str) -> Result<usize, &'static str> {
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in source.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if let Some(delimiter) = quote {
            if character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '}' => return Ok(index),
            '{' => return Err("Accolades imbriquées non prises en charge dans un défaut."),
            _ => {}
        }
    }
    Err("Expression de valeur par défaut non fermée.")
}
