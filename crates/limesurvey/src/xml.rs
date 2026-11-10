use std::{collections::BTreeSet, ops::Range};

use quick_xml::{
    NsReader, XmlVersion,
    events::{BytesDecl, BytesStart, Event},
    name::{NamespaceResolver, ResolveResult},
};

use crate::{Error, Limits};

pub(crate) struct Element {
    pub name: String,
    pub unqualified: bool,
    pub children: Vec<usize>,
    pub text: String,
    pub span: Range<usize>,
    pub content: Option<Range<usize>>,
}

pub(crate) struct Document {
    pub source: String,
    pub elements: Vec<Element>,
    pub root: usize,
}

struct Parser {
    elements: Vec<Element>,
    stack: Vec<usize>,
    root: Option<usize>,
    seen_declaration: bool,
    declaration_offset: u64,
    limits: Limits,
}

impl Document {
    pub fn parse(input: &[u8], limits: Limits) -> Result<Self, Error> {
        validate_limits(limits)?;
        if input.len() > limits.max_bytes {
            return Err(Error::LimitExceeded {
                resource: "input bytes",
                limit: limits.max_bytes,
            });
        }
        let source = std::str::from_utf8(input).map_err(|_| Error::InvalidUtf8)?;
        validate_characters(source)?;
        let mut parser = Parser {
            elements: Vec::new(),
            stack: Vec::new(),
            root: None,
            seen_declaration: false,
            declaration_offset: 0,
            limits,
        };
        parser.read(source)?;
        let root = parser.root.ok_or(Error::NotSurveyDocument)?;
        Ok(Self {
            source: source.to_owned(),
            elements: parser.elements,
            root,
        })
    }

    pub fn children_named<'a>(
        &'a self,
        parent: usize,
        name: &'a str,
    ) -> impl Iterator<Item = usize> + 'a {
        self.elements[parent]
            .children
            .iter()
            .copied()
            .filter(move |&id| {
                let element = &self.elements[id];
                element.unqualified && element.name == name
            })
    }

    pub fn field(&self, parent: usize, name: &str) -> Option<&str> {
        let mut matches = self.children_named(parent, name);
        let id = matches.next()?;
        if matches.next().is_some() || !self.elements[id].children.is_empty() {
            return None;
        }
        Some(&self.elements[id].text)
    }
}

impl Parser {
    fn read(&mut self, source: &str) -> Result<(), Error> {
        // quick-xml removes the BOM without including it in buffer positions.
        // Keep offsets relative to the original bytes used for surgical edits.
        let content = source.strip_prefix('\u{feff}').unwrap_or(source);
        let origin = (source.len() - content.len()) as u64;
        self.declaration_offset = origin;
        let mut reader = NsReader::from_str(content);
        reader.config_mut().check_comments = true;
        loop {
            let start = origin + reader.buffer_position();
            let (namespace, event) = reader
                .read_resolved_event()
                .map_err(|_| Error::InvalidXml { offset: start })?;
            let unqualified = match namespace {
                ResolveResult::Unbound => true,
                ResolveResult::Bound(_) => false,
                ResolveResult::Unknown(_) => return Err(Error::InvalidXml { offset: start }),
            };
            let end = origin + reader.buffer_position();
            if let Event::Start(tag) | Event::Empty(tag) = &event {
                validate_attributes(tag, self.limits.max_attributes, start)?;
                validate_namespaces(tag, reader.resolver(), start)?;
            }
            if matches!(event, Event::Eof) {
                if !self.stack.is_empty() {
                    return Err(Error::InvalidXml { offset: start });
                }
                return Ok(());
            }
            self.event(event, unqualified, start, end)?;
        }
    }

    fn event(
        &mut self,
        event: Event<'_>,
        unqualified: bool,
        start: u64,
        end: u64,
    ) -> Result<(), Error> {
        match event {
            Event::Start(tag) => self.open(&tag, unqualified, start, end, false),
            Event::Empty(tag) => self.open(&tag, unqualified, start, end, true),
            Event::End(_) => self.close(start, end),
            Event::Text(text) => {
                if text.contains("]]>") {
                    return Err(Error::InvalidXml { offset: start });
                }
                self.append(&text.xml10_content(), true, start)
            }
            Event::CData(text) => self.append(&text.xml10_content(), false, start),
            Event::GeneralRef(reference) => {
                let character = match reference
                    .resolve_char_ref()
                    .map_err(|_| Error::InvalidXml { offset: start })?
                {
                    Some(character) => character,
                    None => match reference.as_ref() {
                        "amp" => '&',
                        "lt" => '<',
                        "gt" => '>',
                        "apos" => '\'',
                        "quot" => '"',
                        _ => return Err(Error::InvalidXml { offset: start }),
                    },
                };
                self.append(character.encode_utf8(&mut [0; 4]), false, start)
            }
            Event::Decl(declaration) => self.declaration(&declaration, start),
            Event::DocType(_) => Err(Error::DtdForbidden),
            Event::PI(_) => Err(Error::ProcessingInstructionForbidden),
            Event::Comment(_) | Event::Eof => Ok(()),
        }
    }

    fn open(
        &mut self,
        tag: &BytesStart<'_>,
        unqualified: bool,
        start: u64,
        end: u64,
        empty: bool,
    ) -> Result<(), Error> {
        if self.stack.len() >= self.limits.max_depth {
            return Err(Error::LimitExceeded {
                resource: "element depth",
                limit: self.limits.max_depth,
            });
        }
        if self.elements.len() >= self.limits.max_elements {
            return Err(Error::LimitExceeded {
                resource: "element count",
                limit: self.limits.max_elements,
            });
        }
        if self.stack.is_empty() && self.root.is_some() {
            return Err(Error::InvalidXml { offset: start });
        }
        validate_name(tag.name().as_ref(), start)?;
        let id = self.elements.len();
        self.elements.push(Element {
            name: tag.name().as_ref().to_owned(),
            unqualified,
            children: Vec::new(),
            text: String::new(),
            span: start as usize..end as usize,
            content: if empty {
                None
            } else {
                Some(end as usize..end as usize)
            },
        });
        if let Some(&parent) = self.stack.last() {
            self.elements[parent].children.push(id);
        } else {
            self.root = Some(id);
        }
        if !empty {
            self.stack.push(id);
        }
        Ok(())
    }

    fn close(&mut self, start: u64, end: u64) -> Result<(), Error> {
        let id = self
            .stack
            .pop()
            .ok_or(Error::InvalidXml { offset: start })?;
        let element = &mut self.elements[id];
        element.span.end = end as usize;
        if let Some(content) = &mut element.content {
            content.end = start as usize;
        }
        Ok(())
    }

    fn append(&mut self, text: &str, allow_outer_space: bool, offset: u64) -> Result<(), Error> {
        validate_characters(text)?;
        if let Some(&id) = self.stack.last() {
            self.elements[id].text.push_str(text);
            return Ok(());
        }
        if allow_outer_space
            && text
                .chars()
                .all(|ch| matches!(ch, ' ' | '\t' | '\n' | '\r'))
        {
            return Ok(());
        }
        Err(Error::InvalidXml { offset })
    }

    fn declaration(&mut self, declaration: &BytesDecl<'_>, offset: u64) -> Result<(), Error> {
        if self.seen_declaration || self.root.is_some() || offset != self.declaration_offset {
            return Err(Error::InvalidXml { offset });
        }
        let version = declaration
            .version()
            .map_err(|_| Error::InvalidXml { offset })?;
        if version != "1.0" {
            return Err(Error::UnsupportedEncoding);
        }
        if let Some(encoding) = declaration.encoding() {
            let encoding = encoding.map_err(|_| Error::InvalidXml { offset })?;
            if !encoding.eq_ignore_ascii_case("utf-8") {
                return Err(Error::UnsupportedEncoding);
            }
        }
        let tag = BytesStart::from_content(declaration.as_ref(), 3);
        validate_attributes(&tag, self.limits.max_attributes, offset)?;
        self.seen_declaration = true;
        Ok(())
    }
}

fn validate_limits(limits: Limits) -> Result<(), Error> {
    if [
        limits.max_bytes,
        limits.max_depth,
        limits.max_elements,
        limits.max_attributes,
        limits.max_diagnostics,
    ]
    .contains(&0)
    {
        return Err(Error::InvalidLimits);
    }
    Ok(())
}

pub(crate) fn validate_characters(text: &str) -> Result<(), Error> {
    if text.chars().all(|ch| matches!(ch,
        '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}'
    )) { return Ok(()); }
    Err(Error::InvalidXmlCharacter)
}

fn validate_attributes(tag: &BytesStart<'_>, limit: usize, offset: u64) -> Result<(), Error> {
    for (index, attribute) in tag.attributes().enumerate() {
        if index >= limit {
            return Err(Error::LimitExceeded {
                resource: "attributes per element",
                limit,
            });
        }
        let attribute = attribute.map_err(|_| Error::InvalidXml { offset })?;
        validate_name(attribute.key.as_ref(), offset)?;
        if attribute.value.contains('<') {
            return Err(Error::InvalidXml { offset });
        }
        let value = attribute
            .normalized_value(XmlVersion::Explicit1_0)
            .map_err(|_| Error::InvalidXml { offset })?;
        validate_characters(&value)?;
    }
    Ok(())
}

fn validate_namespaces(
    tag: &BytesStart<'_>,
    resolver: &NamespaceResolver,
    offset: u64,
) -> Result<(), Error> {
    let mut expanded_names = BTreeSet::new();
    for attribute in tag.attributes() {
        let attribute = attribute.map_err(|_| Error::InvalidXml { offset })?;
        let name = attribute.key.as_ref();
        if name == "xmlns" || name.starts_with("xmlns:") {
            continue;
        }
        let (namespace, local) = resolver.resolve_attribute(attribute.key);
        let namespace = match namespace {
            ResolveResult::Unbound => "",
            ResolveResult::Bound(namespace) => namespace.into_inner(),
            ResolveResult::Unknown(_) => return Err(Error::InvalidXml { offset }),
        };
        if !expanded_names.insert((local.into_inner(), namespace)) {
            return Err(Error::InvalidXml { offset });
        }
    }
    Ok(())
}

fn validate_name(name: &str, offset: u64) -> Result<(), Error> {
    let mut parts = name.split(':');
    for part in parts.by_ref().take(2) {
        let mut characters = part.chars();
        if !characters.next().is_some_and(name_start) || !characters.all(name_character) {
            return Err(Error::InvalidXml { offset });
        }
    }
    if parts.next().is_some() {
        return Err(Error::InvalidXml { offset });
    }
    Ok(())
}

fn name_start(ch: char) -> bool {
    matches!(ch, 'A'..='Z' | '_' | 'a'..='z' | '\u{C0}'..='\u{D6}' | '\u{D8}'..='\u{F6}'
        | '\u{F8}'..='\u{2FF}' | '\u{370}'..='\u{37D}' | '\u{37F}'..='\u{1FFF}'
        | '\u{200C}'..='\u{200D}' | '\u{2070}'..='\u{218F}' | '\u{2C00}'..='\u{2FEF}'
        | '\u{3001}'..='\u{D7FF}' | '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFFD}'
        | '\u{10000}'..='\u{EFFFF}')
}

fn name_character(ch: char) -> bool {
    name_start(ch)
        || matches!(ch, '-' | '.' | '0'..='9' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}')
}
