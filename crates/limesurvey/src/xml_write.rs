use crate::{Error, Limits};

pub(crate) struct Xml {
    output: String,
    limits: Limits,
    elements: usize,
    columns: &'static [&'static str],
    newlines: bool,
}

impl Xml {
    pub fn new(limits: Limits) -> Result<Self, Error> {
        if limits.max_bytes == 0 || limits.max_elements == 0 {
            return Err(Error::InvalidLimits);
        }
        let mut xml = Self {
            output: String::new(),
            limits,
            elements: 0,
            columns: &[],
            newlines: true,
        };
        xml.append("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")?;
        Ok(xml)
    }

    pub fn compact(limits: Limits) -> Result<Self, Error> {
        let mut xml = Self::new(limits)?;
        xml.newlines = false;
        Ok(xml)
    }

    fn append(&mut self, source: &str) -> Result<(), Error> {
        if source.len() > self.limits.max_bytes.saturating_sub(self.output.len()) {
            return Err(Error::LimitExceeded {
                resource: "XML bytes",
                limit: self.limits.max_bytes,
            });
        }
        self.output.push_str(source);
        Ok(())
    }

    pub fn start(&mut self, name: &str) -> Result<(), Error> {
        self.open(name, ">")
    }

    fn open(&mut self, name: &str, ending: &str) -> Result<(), Error> {
        self.elements += 1;
        if self.elements > self.limits.max_elements {
            return Err(Error::LimitExceeded {
                resource: "XML elements",
                limit: self.limits.max_elements,
            });
        }
        self.append("<")?;
        self.append(name)?;
        self.append(ending)
    }

    pub fn end(&mut self, name: &str) -> Result<(), Error> {
        self.append("</")?;
        self.append(name)?;
        self.append(">")?;
        if self.newlines {
            self.append("\n")?;
        }
        Ok(())
    }

    pub fn field(&mut self, name: &str, value: &str) -> Result<(), Error> {
        crate::xml::validate_characters(value)?;
        if value.is_empty() {
            self.open(name, "/>")?;
            if self.newlines {
                self.append("\n")?;
            }
            return Ok(());
        }
        self.start(name)?;
        let mut begin = 0;
        for (offset, character) in value.char_indices() {
            let escaped = match character {
                '&' => "&amp;",
                '<' => "&lt;",
                '>' => "&gt;",
                '\r' => "&#13;",
                _ => continue,
            };
            self.append(&value[begin..offset])?;
            self.append(escaped)?;
            begin = offset + character.len_utf8();
        }
        self.append(&value[begin..])?;
        self.end(name)
    }

    pub fn table(&mut self, name: &str, columns: &'static [&'static str]) -> Result<(), Error> {
        self.start(name)?;
        self.start("fields")?;
        for column in columns {
            self.field("fieldname", column)?;
        }
        self.end("fields")?;
        self.start("rows")?;
        self.columns = columns;
        Ok(())
    }

    pub fn row(&mut self, values: &[&str]) -> Result<(), Error> {
        if values.len() != self.columns.len() {
            return Err(Error::InvalidStructure);
        }
        self.start("row")?;
        for (column, value) in self.columns.iter().zip(values) {
            self.field(column, value)?;
        }
        self.end("row")
    }

    pub fn end_table(&mut self, name: &str) -> Result<(), Error> {
        self.end("rows")?;
        self.columns = &[];
        self.end(name)
    }

    pub fn finish(self) -> Vec<u8> {
        self.output.into_bytes()
    }
}
