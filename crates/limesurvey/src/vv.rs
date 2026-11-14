use crate::{Error, Limits, ResponseRow, ResponseTable};
use std::collections::{BTreeMap, BTreeSet};

pub struct VvEncoder {
    columns: Vec<String>,
    header: Vec<u8>,
    limits: Limits,
}

impl VvEncoder {
    /// Prepare the headers once and encode each response with a separate bounded buffer.
    pub fn new(
        columns: Vec<String>,
        aliases: &BTreeMap<String, String>,
        limits: Limits,
    ) -> Result<Self, Error> {
        let header = ResponseTable {
            languages: Vec::new(),
            columns: columns.clone(),
            rows: Vec::new(),
        }
        .to_vv(aliases, limits)?;
        Ok(Self {
            columns,
            header,
            limits,
        })
    }

    pub fn header(&self) -> &[u8] {
        &self.header
    }

    pub fn row(&self, row: &ResponseRow) -> Result<Vec<u8>, Error> {
        let known: BTreeSet<_> = self.columns.iter().collect();
        let mut output = String::new();
        encode_row(&mut output, &self.columns, &known, row, self.limits)?;
        Ok(output.into_bytes())
    }
}

impl ResponseTable {
    /// Export VV with explicit question-code aliases, independent of reassigned SGQA IDs.
    pub fn to_vv(
        &self,
        aliases: &BTreeMap<String, String>,
        limits: Limits,
    ) -> Result<Vec<u8>, Error> {
        if self.columns.is_empty()
            || self.columns.len() > limits.max_elements
            || self.rows.len().saturating_mul(self.columns.len()) > limits.max_elements
        {
            return Err(Error::InvalidStructure);
        }
        let mut names = BTreeSet::new();
        let mut header = Vec::new();
        for column in &self.columns {
            let alias = aliases.get(column).unwrap_or(column);
            if alias.is_empty()
                || alias.len() > 256
                || !alias
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_#".contains(&byte))
                || !names.insert(alias)
            {
                return Err(Error::InvalidMetadata("VV response alias"));
            }
            header.push(alias.as_str());
        }
        if aliases.keys().any(|column| !self.columns.contains(column)) {
            return Err(Error::InvalidStructure);
        }
        let columns: BTreeSet<_> = self.columns.iter().collect();
        let mut output = String::new();
        // LimeSurvey ignores the first, human-readable header and maps the second.
        for _ in 0..2 {
            for (index, alias) in header.iter().enumerate() {
                if index > 0 {
                    append(&mut output, "\t", limits)?;
                }
                append(&mut output, alias, limits)?;
            }
            append(&mut output, "\n", limits)?;
        }
        for row in &self.rows {
            encode_row(&mut output, &self.columns, &columns, row, limits)?;
        }
        Ok(output.into_bytes())
    }
}

fn encode_row(
    output: &mut String,
    columns: &[String],
    known: &BTreeSet<&String>,
    row: &ResponseRow,
    limits: Limits,
) -> Result<(), Error> {
    if row.keys().any(|column| !known.contains(column)) {
        return Err(Error::InvalidStructure);
    }
    for (index, column) in columns.iter().enumerate() {
        if index > 0 {
            append(output, "\t", limits)?;
        }
        if let Some(Some(value)) = row.get(column) {
            encode(output, value, limits)?;
        } else {
            append(output, "{question_not_shown}", limits)?;
        }
    }
    append(output, "\n", limits)
}

fn encode(output: &mut String, value: &str, limits: Limits) -> Result<(), Error> {
    for character in value.chars() {
        let mut bytes = [0; 4];
        let encoded = match character {
            '{' => "{lbrace}",
            '\n' => "{newline}",
            '\r' => "{cr}",
            '\t' => "{tab}",
            '"' => "{quote}",
            _ => character.encode_utf8(&mut bytes),
        };
        append(output, encoded, limits)?;
    }
    Ok(())
}

fn append(output: &mut String, value: &str, limits: Limits) -> Result<(), Error> {
    if value.len() > limits.max_bytes.saturating_sub(output.len()) {
        return Err(Error::LimitExceeded {
            resource: "VV bytes",
            limit: limits.max_bytes,
        });
    }
    output.push_str(value);
    Ok(())
}
