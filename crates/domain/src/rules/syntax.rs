use std::collections::BTreeSet;

#[derive(Clone, Debug)]
enum Token {
    Number,
    String,
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
                    index += 1;
                }
            }
            if !closed {
                return Err("Chaîne d’expression non terminée.");
            }
            tokens.push(Token::String);
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
            if !number.parse::<f64>().is_ok_and(f64::is_finite) {
                return Err("Nombre non fini dans l’expression.");
            }
            tokens.push(Token::Number);
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
    fn binary(&mut self, minimum: u8, depth: usize) -> Result<(), &'static str> {
        if depth > 32 {
            return Err("Expression trop imbriquée.");
        }
        self.atom(depth + 1)?;
        while let Token::Operator(operator) = self.peek() {
            let Some(level) = precedence(operator) else {
                break;
            };
            if level < minimum {
                break;
            }
            self.take();
            self.binary(level + 1, depth + 1)?;
        }
        Ok(())
    }
    fn atom(&mut self, depth: usize) -> Result<(), &'static str> {
        if depth > 32 {
            return Err("Expression trop imbriquée.");
        }
        match self.take() {
            Token::Number | Token::String => Ok(()),
            Token::Operator(operator) if ["!", "not", "-", "+"].contains(&operator.as_str()) => {
                self.atom(depth + 1)
            }
            Token::Symbol('(') => {
                self.binary(0, depth + 1)?;
                if !matches!(self.take(), Token::Symbol(')')) {
                    return Err("Parenthèse fermante manquante.");
                }
                Ok(())
            }
            Token::Name(name) => {
                let lower = name.to_ascii_lowercase();
                if ["true", "false", "null"].contains(&lower.as_str()) {
                    return Ok(());
                }
                if matches!(self.peek(), Token::Symbol('(')) {
                    return self.call(&lower, depth + 1);
                }
                self.variables.insert(name);
                Ok(())
            }
            _ => Err("Valeur attendue dans l’expression."),
        }
    }
    fn call(&mut self, name: &str, depth: usize) -> Result<(), &'static str> {
        let (minimum, maximum) = arity(name).ok_or("Fonction non prise en charge.")?;
        self.take();
        let mut count = 0;
        if !matches!(self.peek(), Token::Symbol(')')) {
            loop {
                self.binary(0, depth + 1)?;
                count += 1;
                if !matches!(self.peek(), Token::Symbol(',')) {
                    break;
                }
                self.take();
            }
        }
        if !matches!(self.take(), Token::Symbol(')')) {
            return Err("Parenthèse de fonction manquante.");
        }
        if count < minimum || count > maximum {
            return Err("Nombre d’arguments incorrect.");
        }
        Ok(())
    }
}
fn arity(name: &str) -> Option<(usize, usize)> {
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

pub fn references(source: &str) -> Result<BTreeSet<String>, &'static str> {
    let source = source.trim_matches(is_whitespace);
    let mut parser = Parser {
        tokens: tokenize(source)?,
        position: 0,
        variables: BTreeSet::new(),
    };
    parser.binary(0, 0)?;
    if !matches!(parser.peek(), Token::End) {
        return Err("Fin d’expression inattendue.");
    }
    Ok(parser.variables)
}
