use super::{
    EvaluationBudget, Scalar,
    syntax::Node,
    value::{compare, equal},
};
use std::cmp::Ordering;

pub(super) fn evaluate(
    node: &Node,
    lookup: &impl Fn(&str) -> Scalar,
    budget: &mut EvaluationBudget,
) -> Result<Scalar, &'static str> {
    budget.charge(1)?;
    let value = match node {
        Node::Literal(value) => value.clone(),
        Node::Variable(name) => {
            let value = lookup(name);
            if let Scalar::String(text) = &value {
                budget.charge(text.len())?;
            }
            value
        }
        Node::Unary(operator, child) => {
            let value = evaluate(child, lookup, budget)?;
            match operator.as_str() {
                "!" | "not" => Scalar::Boolean(!value.truth()),
                "-" => Scalar::Number(-value.numeric()?),
                _ => Scalar::Number(value.numeric()?),
            }
        }
        Node::Binary(operator, left, right) => {
            let left = evaluate(left, lookup, budget)?;
            match operator.as_str() {
                "and" | "&&" => {
                    Scalar::Boolean(left.truth() && evaluate(right, lookup, budget)?.truth())
                }
                "or" | "||" => {
                    Scalar::Boolean(left.truth() || evaluate(right, lookup, budget)?.truth())
                }
                _ => binary(operator, &left, &evaluate(right, lookup, budget)?)?,
            }
        }
        Node::Call(name, arguments) if name == "if" => {
            let condition = evaluate(&arguments[0], lookup, budget)?;
            if condition.truth() {
                evaluate(&arguments[1], lookup, budget)?
            } else if let Some(no) = arguments.get(2) {
                evaluate(no, lookup, budget)?
            } else {
                Scalar::String(String::new())
            }
        }
        Node::Call(name, arguments) => {
            let mut values = Vec::with_capacity(arguments.len());
            for argument in arguments {
                let value = evaluate(argument, lookup, budget)?;
                if let Scalar::String(text) = &value {
                    budget.charge(text.len())?;
                }
                values.push(value);
            }
            super::functions::evaluate(name, &values, budget)?
        }
    };
    value.checked()
}

fn binary(operator: &str, left: &Scalar, right: &Scalar) -> Result<Scalar, &'static str> {
    let relation = match operator {
        "==" | "eq" => Some(equal(left, right)),
        "!=" | "ne" => Some(!equal(left, right)),
        "<" | "lt" => Some(compare(left, right) == Ordering::Less),
        "<=" | "le" => Some(compare(left, right) != Ordering::Greater),
        ">" | "gt" => Some(compare(left, right) == Ordering::Greater),
        ">=" | "ge" => Some(compare(left, right) != Ordering::Less),
        _ => None,
    };
    if let Some(value) = relation {
        return Ok(Scalar::Boolean(value));
    }
    let left = left.numeric()?;
    let right = right.numeric()?;
    let value = match operator {
        "+" => left + right,
        "-" => left - right,
        "*" => left * right,
        "/" | "%" if right == 0.0 => return Err("Division par zéro dans une règle."),
        "/" => left / right,
        _ => left % right,
    };
    Scalar::Number(value).checked()
}
