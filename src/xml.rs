use std::{
    collections::HashMap,
    fmt::Debug,
    fs::File,
    io::{self, BufReader, BufWriter, Write},
    path::Path,
};

use xml::{
    attribute::{Attribute, OwnedAttribute},
    name::OwnedName,
    namespace::Namespace,
    reader::{self, XmlEvent},
    writer::{self, XmlEvent as WriteEvent},
    EmitterConfig, EventReader, EventWriter, ParserConfig,
};

struct MutBuf<'a>(&'a mut Vec<u8>);

impl Write for MutBuf<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        todo!()
    }
}

pub enum TemplateError {
    Io(io::Error),
    Parse(reader::Error),
    Write(writer::Error),
    MissingProp(String),
    MalformedProp(String),
}

impl From<io::Error> for TemplateError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<reader::Error> for TemplateError {
    fn from(value: reader::Error) -> Self {
        Self::Parse(value)
    }
}

impl From<writer::Error> for TemplateError {
    fn from(value: writer::Error) -> Self {
        Self::Write(value)
    }
}

impl Debug for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => Debug::fmt(&err, f),
            Self::Write(err) => Debug::fmt(&err, f),
            Self::Parse(err) => Debug::fmt(&err, f),
            Self::MissingProp(name) => write!(f, "Missing property {}.", name),
            Self::MalformedProp(name) => write!(
                f,
                "Property '{}' is non-alphanumeric or reserved. (accepted: A-z 0-9 _; must not start with __).",
                name
            ),
        }
    }
}

#[derive(Clone)]
enum TemplateEvent {
    Xml(XmlEvent),
    LibraryInsert,
    StartPlaceholder {
        prop: String,
        name: OwnedName,
        attributes: Vec<OwnedAttribute>,
        namespace: Namespace,
    },
}

pub struct Template {
    events: Vec<TemplateEvent>,
    parser_config: ParserConfig,
    library_path: String,
}

impl Template {
    const PROPS_SPECIAL: [&'static str; 2] = ["__path", "__filename"];

    pub fn parse_from_file(
        path: &Path,
        parser_config: ParserConfig,
        library_path: String,
    ) -> Result<Self, TemplateError> {
        let file = File::open(path)?;
        let file = BufReader::new(file);

        let parser = EventReader::new_with_config(file, parser_config.clone());
        let mut events = Vec::new();
        for event in parser {
            match event? {
                XmlEvent::StartElement {
                    name,
                    attributes,
                    namespace,
                } => {
                    let mut placeholder = String::new();
                    for OwnedAttribute {
                        name: attr_name,
                        value,
                    } in &attributes
                    {
                        if attr_name.to_string() == "sr-prop" {
                            placeholder = value.to_string();
                            break;
                        }
                    }

                    if placeholder == "" {
                        events.push(TemplateEvent::Xml(XmlEvent::StartElement {
                            name,
                            attributes,
                            namespace,
                        }))
                    } else if !placeholder.chars().all(|c| c.is_alphanumeric() || c == '_')
                        || (placeholder.starts_with("__")
                            && !Self::PROPS_SPECIAL.contains(&placeholder.as_str()))
                    {
                        return Err(TemplateError::MalformedProp(placeholder));
                    } else {
                        events.push(TemplateEvent::StartPlaceholder {
                            prop: placeholder,
                            name,
                            attributes,
                            namespace,
                        });
                    }
                }
                XmlEvent::EndElement { name } => {
                    if name.to_string() == "head" {
                        events.push(TemplateEvent::LibraryInsert);
                    }
                    events.push(TemplateEvent::Xml(XmlEvent::EndElement { name }))
                }
                e => events.push(TemplateEvent::Xml(e)),
            }
        }

        Ok(Self {
            events,
            parser_config,
            library_path,
        })
    }

    fn writer_config() -> EmitterConfig {
        EmitterConfig {
            normalize_empty_elements: false,
            write_document_declaration: false,
            ..Default::default()
        }
    }

    pub fn write_to_file(
        &self,
        source: BufReader<File>,
        out: BufWriter<File>,
        out_json: BufWriter<File>,
        mut props_map: HashMap<String, Vec<XmlEvent>>,
    ) -> Result<(), TemplateError> {
        let parser = EventReader::new_with_config(source, self.parser_config.clone());
        let mut current_prop = None;
        let mut current_events = Vec::new();

        for event in parser {
            if current_prop.is_none() {
                match event? {
                    XmlEvent::StartElement { name, .. } => {
                        current_prop = Some(name.to_string());
                    }
                    _ => (),
                }
            } else {
                match event? {
                    XmlEvent::EndElement { name } => {
                        if current_prop
                            .as_ref()
                            .is_some_and(|p| name.to_string() == *p)
                        {
                            props_map.insert(current_prop.take().unwrap(), current_events);
                            current_events = Vec::new();
                        } else {
                            current_events.push(XmlEvent::EndElement { name })
                        }
                    }
                    event => current_events.push(event),
                }
            }
        }

        let mut writer = EventWriter::new_with_config(out, Self::writer_config());
        let mut json_map = HashMap::new();

        for event in self.events.clone() {
            match event {
                TemplateEvent::Xml(xml_event) => {
                    let writer_event = xml_event.as_writer_event();
                    match writer_event {
                        Some(WriteEvent::StartDocument { .. }) => (),
                        Some(writer_event) => writer.write(writer_event)?,
                        None => (),
                    }
                }
                TemplateEvent::LibraryInsert => {
                    writer.write::<WriteEvent<'_>>(
                        WriteEvent::start_element("script")
                            .attr("src", &self.library_path)
                            .into(),
                    )?;
                    writer
                        .write::<WriteEvent<'_>>(WriteEvent::end_element().name("script").into())?;
                }
                TemplateEvent::StartPlaceholder {
                    prop,
                    name,
                    attributes,
                    namespace,
                } => {
                    let xml_event = WriteEvent::StartElement {
                        name: name.borrow(),
                        attributes: attributes
                            .iter()
                            .map(|a| {
                                if a.name.to_string() == "sr-prop" {
                                    Attribute {
                                        name: "data-sr-prop".into(),
                                        value: &a.value,
                                    }
                                } else {
                                    a.borrow()
                                }
                            })
                            .collect(),
                        namespace: namespace.borrow(),
                    };
                    writer.write(xml_event)?;

                    let mut json_buf = Vec::new();
                    let mut json_writer = EventWriter::new_with_config(
                        MutBuf(&mut json_buf),
                        EmitterConfig {
                            perform_indent: false,
                            ..Self::writer_config()
                        },
                    );

                    for event in props_map
                        .get(&prop)
                        .ok_or(TemplateError::MissingProp(prop.clone()))?
                    {
                        let writer_event = event.as_writer_event();
                        if let Some(writer_event) = writer_event {
                            writer.write(writer_event.clone())?;
                            json_writer.write(writer_event)?;
                        }
                    }

                    json_map.insert(prop, String::from_utf8(json_buf).unwrap());
                }
            }
        }

        serde_json::to_writer(out_json, &json_map).unwrap();

        Ok(())
    }
}
