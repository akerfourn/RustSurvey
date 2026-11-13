use super::{EvaluationBudget, Scalar};
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub(super) enum Node {
    Literal(Scalar),
    Variable(String),
    Unary(String, Box<Node>),
    Binary(String, Box<Node>, Box<Node>),
    Call(String, Vec<Node>),
}

#[derive(Clone, Debug)]
pub struct Expression {
    root: Node,
    pub variables: BTreeSet<String>,
}

impl Expression {
    pub fn compile(source: &str) -> Result<Self, &'static str> {
        let mut parser = Parser {
            tokens: tokenize(source.trim_matches(is_whitespace))?,
            position: 0,
            variables: BTreeSet::new(),
        };
        let root = parser.binary(0, 0)?;
        if !matches!(parser.peek(), Token::End) {
            return Err("Fin d’expression inattendue.");
        }
        Ok(Self {
            root,
            variables: parser.variables,
        })
    }

    pub fn evaluate(&self, lookup: impl Fn(&str) -> Scalar) -> Result<Scalar, &'static str> {
        self.evaluate_with_budget(&lookup, &mut EvaluationBudget::default())
    }

    pub fn evaluate_with_budget(
        &self,
        lookup: &impl Fn(&str) -> Scalar,
        budget: &mut EvaluationBudget,
    ) -> Result<Scalar, &'static str> {
        super::evaluate::evaluate(&self.root, lookup, budget)
    }
}

#[derive(Clone, Debug)]
enum Token {
    Number(f64),
    String(String),
    Name(String),
    Operator(String),
    Symbol(char),
    End,
}

pub(super) fn is_whitespace(character: char) -> bool {
    matches!(
        character,
        '\t' | '\n' | '\u{000b}' | '\u{000c}' | '\r' | ' ' | '\u{00a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

fn precedence(operator: &str) -> Option<u8> {
    match operator {
        "or" | "||" => Some(1),
        "and" | "&&" => Some(2),
        "eq" | "ne" | "lt" | "le" | "gt" | "ge" | "==" | "!=" | "<" | "<=" | ">" | ">=" => Some(3),
        "+" | "-" => Some(4),
        "*" | "/" | "%" => Some(5),
        _ => None,
    }
}

fn tokenize(source: &str) -> Result<Vec<Token>, &'static str> {
    if source.encode_utf16().count() > 4096 {
        return Err("Expression trop longue.");
    }
    let chars: Vec<_> = source.chars().collect();
    let mut index = 0;
    let mut tokens = Vec::new();
    while index < chars.len() {
        let character = chars[index];
        if is_whitespace(character) {
            index += 1;
            continue;
        }
        if character == '\'' || character == '"' {
            index += 1;
            let mut closed = false;
            let mut value = String::new();
            while index < chars.len() {
                let next = chars[index];
                index += 1;
                if next == character {
                    closed = true;
                    break;
                }
                if next == '\\' {
                    if index >= chars.len()
                        || !['n', 'r', 't', '\\', '\'', '"'].contains(&chars[index])
                    {
                        return Err("Échappement de chaîne non pris en charge.");
                    }
                    value.push(match chars[index] {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        escaped => escaped,
                    });
                    index += 1;
                } else {
                    value.push(next);
                }
            }
            if !closed {
                return Err("Chaîne d’expression non terminée.");
            }
            tokens.push(Token::String(value));
        } else if character.is_ascii_digit()
            || (character == '.' && chars.get(index + 1).is_some_and(char::is_ascii_digit))
        {
            let start = index;
            while chars.get(index).is_some_and(char::is_ascii_digit) {
                index += 1;
            }
            if chars.get(index) == Some(&'.') {
                index += 1;
                while chars.get(index).is_some_and(char::is_ascii_digit) {
                    index += 1;
                }
            }
            if chars.get(index).is_some_and(|c| *c == 'e' || *c == 'E') {
                index += 1;
                if chars.get(index).is_some_and(|c| *c == '+' || *c == '-') {
                    index += 1;
                }
                let exponent = index;
                while chars.get(index).is_some_and(char::is_ascii_digit) {
                    index += 1;
                }
                if exponent == index {
                    return Err("Exposant numérique invalide.");
                }
            }
            let number: String = chars[start..index].iter().collect();
            let value = number
                .parse::<f64>()
                .map_err(|_| "Nombre non fini dans l’expression.")?;
            if !value.is_finite() {
                return Err("Nombre non fini dans l’expression.");
            }
            tokens.push(Token::Number(value));
        } else if character.is_ascii_alphabetic() || character == '_' {
            let start = index;
            index += 1;
            while index < chars.len() {
                if chars[index].is_ascii_alphanumeric() || chars[index] == '_' {
                    index += 1;
                } else if chars[index] == '.'
                    && chars
                        .get(index + 1)
                        .is_some_and(|c| c.is_ascii_alphabetic() || *c == '_')
                {
                    index += 2;
                } else {
                    break;
                }
            }
            let name: String = chars[start..index].iter().collect();
            let lower = name.to_ascii_lowercase();
            if ["and", "or", "not", "eq", "ne", "lt", "le", "gt", "ge"].contains(&lower.as_str()) {
                tokens.push(Token::Operator(lower));
            } else {
                tokens.push(Token::Name(name));
            }
        } else if ['(', ')', ','].contains(&character) {
            tokens.push(Token::Symbol(character));
            index += 1;
        } else {
            let pair: String = chars[index..chars.len().min(index + 2)].iter().collect();
            if ["==", "!=", "<=", ">=", "&&", "||"].contains(&pair.as_str()) {
                tokens.push(Token::Operator(pair));
                index += 2;
            } else if ['+', '-', '*', '/', '%', '<', '>', '!'].contains(&character) {
                tokens.push(Token::Operator(character.to_string()));
                index += 1;
            } else {
                return Err("Syntaxe d’expression non prise en charge.");
            }
        }
        if tokens.len() > 512 {
            return Err("Expression trop complexe.");
        }
    }
    tokens.push(Token::End);
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
    variables: BTreeSet<String>,
}
impl Parser {
    fn take(&mut self) -> Token {
        let token = self
            .tokens
            .get(self.position)
            .cloned()
            .unwrap_or(Token::End);
        self.position += 1;
        token
    }
    fn peek(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::End)
    }
    fn binary(&mut self, minimum: u8, depth: usize) -> Result<Node, &'static str> {
        if depth > 32 {
            return Err("Expression trop imbriquée.");
        }
        let mut left = self.atom(depth + 1)?;
        while let Token::Operator(operator) = self.peek() {
            let Some(level) = precedence(operator) else {
                break;
            };
            if level < minimum {
                break;
            }
            let operator = operator.clone();
            self.take();
            let right = self.binary(level + 1, depth + 1)?;
            left = Node::Binary(operator, Box::new(left), Box::new(right));
        }
        Ok(left)
    }
    fn atom(&mut self, depth: usize) -> Result<Node, &'static str> {
        if depth > 32 {
            return Err("Expression trop imbriquée.");
        }
        match self.take() {
            Token::Number(value) => Ok(Node::Literal(Scalar::Number(value))),
            Token::String(value) => Ok(Node::Literal(Scalar::String(value))),
            Token::Operator(operator) if ["!", "not", "-", "+"].contains(&operator.as_str()) => {
                Ok(Node::Unary(operator, Box::new(self.atom(depth + 1)?)))
            }
            Token::Symbol('(') => {
                let node = self.binary(0, depth + 1)?;
                if !matches!(self.take(), Token::Symbol(')')) {
                    return Err("Parenthèse fermante manquante.");
                }
                Ok(node)
            }
            Token::Name(name) => {
                let lower = name.to_ascii_lowercase();
                if ["true", "false", "null"].contains(&lower.as_str()) {
                    let value = match lower.as_str() {
                        "true" => Scalar::Boolean(true),
                        "false" => Scalar::Boolean(false),
                        _ => Scalar::Null,
                    };
                    return Ok(Node::Literal(value));
                }
                if matches!(self.peek(), Token::Symbol('(')) {
                    return self.call(&lower, depth + 1);
                }
                self.variables.insert(name.clone());
                Ok(Node::Variable(name))
            }
            _ => Err("Valeur attendue dans l’expression."),
        }
    }
    fn call(&mut self, name: &str, depth: usize) -> Result<Node, &'static str> {
        let (minimum, maximum) =
            super::functions::arity(name).ok_or("Fonction non prise en charge.")?;
        self.take();
        let mut arguments = Vec::new();
        if !matches!(self.peek(), Token::Symbol(')')) {
            loop {
                arguments.push(self.binary(0, depth + 1)?);
                if !matches!(self.peek(), Token::Symbol(',')) {
                    break;
                }
                self.take();
            }
        }
        if !matches!(self.take(), Token::Symbol(')')) {
            return Err("Parenthèse de fonction manquante.");
        }
        if arguments.len() < minimum || arguments.len() > maximum {
            return Err("Nombre d’arguments incorrect.");
        }
        Ok(Node::Call(name.to_owned(), arguments))
    }
}

pub fn references(source: &str) -> Result<BTreeSet<String>, &'static str> {
    Ok(Expression::compile(source)?.variables)
}
