use std::{
    collections::HashMap,
    fmt::Display,
    fs::{self, File},
    io::{self, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use crate::xml::Template;
use crate::{config::Config, xml::TemplateError};
use xml::{
    reader::{self, XmlEvent},
    writer,
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Verbosity {
    Silent,
    Low,
    High,
}

impl TryFrom<&str> for Verbosity {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "silent" => Ok(Self::Silent),
            "low" => Ok(Self::Low),
            "high" => Ok(Self::High),
            _ => Err(()),
        }
    }
}

pub enum BuildError {
    Io(io::Error),
    Parse {
        err: reader::Error,
        source: Option<String>,
    },
    Write {
        err: writer::Error,
        source: Option<String>,
    },
    Other {
        msg: String,
        source: Option<String>,
    },
}

impl BuildError {
    fn with_source(self, source: String) -> Self {
        match self {
            Self::Io(_) => self,
            Self::Parse { err, .. } => Self::Parse {
                err,
                source: Some(source),
            },
            Self::Write { err, .. } => Self::Write {
                err,
                source: Some(source),
            },
            Self::Other { msg, .. } => Self::Other {
                msg,
                source: Some(source),
            },
        }
    }
}

impl From<io::Error> for BuildError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<TemplateError> for BuildError {
    fn from(value: TemplateError) -> Self {
        match value {
            TemplateError::Io(v) => Self::Io(v),
            TemplateError::Parse(err) => Self::Parse { err, source: None },
            TemplateError::Write(err) => Self::Write { err, source: None },
            TemplateError::MissingProp(name) => Self::Other { msg: format!("Missing property {name}."), source: None },
            TemplateError::MalformedProp(name) => Self::Other { msg: format!("Property '{name}' is non-alphanumeric or reserved.\n  (accepted: A-z 0-9 _; must not start with __)."), source: None },
        }
    }
}

impl Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(v) => v.fmt(f),
            Self::Parse { err, source } => write!(
                f,
                "{err} in {}",
                if let Some(source) = source {
                    source
                } else {
                    ""
                }
            ),
            Self::Write { err, source } => write!(
                f,
                "{err} in {}",
                if let Some(source) = source {
                    source
                } else {
                    ""
                }
            ),
            Self::Other { msg, source } => write!(
                f,
                "{msg} in {}",
                if let Some(source) = source {
                    source
                } else {
                    ""
                }
            ),
        }
    }
}

pub fn build(verbosity: Verbosity, config: Config) -> Result<(), BuildError> {
    if verbosity == Verbosity::High {
        println!("Creating output directory at {}", config.out.path);
    }
    if fs::metadata(&config.out.path).is_ok_and(|m| m.is_dir()) {
        fs::remove_dir_all(&config.out.path)?;
    }
    let pages = copy_dir(
        &config.source.path,
        &config.out.path,
        &config
            .source
            .exclude
            .iter()
            .map(|s| s.as_str())
            .chain([&config.out.path, "simple-router.toml"].into_iter())
            .collect(),
        verbosity,
    )?;
    if verbosity == Verbosity::High {
        println!("Done!");
    }

    let template_path =
        Into::<PathBuf>::into(config.source.path.clone()).join(&config.source.template);
    if verbosity == Verbosity::High {
        print!(
            "Parsing template at {} ",
            template_path.to_str().unwrap_or("")
        );
    }
    let template = Template::parse_from_file(
        &template_path,
        config.xml.into(),
        config.out.lib_file.clone(),
    )
    .map_err(|err| {
        BuildError::from(err).with_source(template_path.to_str().unwrap_or("").to_string())
    })?;
    if verbosity == Verbosity::High {
        println!("Done!");
    }

    if verbosity == Verbosity::High {
        println!("Generating static site in {} ", config.out.path);
    }
    for (page, page_out) in pages {
        if page == template_path {
            continue;
        }

        let mut props = HashMap::new();
        props.insert(
            "__path".to_string(),
            vec![XmlEvent::Characters(String::from(
                page_out
                    .strip_prefix(&config.out.path)
                    .unwrap_or(&page_out)
                    .with_extension("")
                    .to_str()
                    .unwrap_or(""),
            ))],
        );

        let source = BufReader::new(File::open(page.clone())?);

        let out_json = BufWriter::new(File::create(page_out.with_extension("page.json"))?);

        if verbosity == Verbosity::High {
            println!("  {}", page.as_os_str().to_str().unwrap_or(""));
        }

        let is_404 = page_out.ends_with(Path::new(&config.js.not_found));

        let out = BufWriter::new(File::create(page_out)?);

        template
            .write_to_file(source, out, out_json, props, is_404)
            .map_err(|err| {
                BuildError::from(err).with_source(page.to_str().unwrap_or("").to_string())
            })?;
    }
    if verbosity == Verbosity::High {
        println!("Done!");
    }

    let mut library_path = PathBuf::from(&config.out.path);
    library_path.push(&config.out.lib_file);
    if verbosity == Verbosity::High {
        print!(
            "Adding library file at {} ",
            library_path.to_str().unwrap_or("")
        );
    }

    let library = config.js.get_code() + include_str!("simple_router.js");

    File::create(library_path)?.write_all(library.as_bytes())?;
    if verbosity == Verbosity::High {
        println!("Done!");
    }

    Ok(())
}

// From StackOverflow: https://stackoverflow.com/a/65192210 + modifications
fn copy_dir(
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
    exclude: &Vec<&str>,
    verbosity: Verbosity,
) -> io::Result<Vec<(PathBuf, PathBuf)>> {
    fs::create_dir_all(&dst)?;
    let mut pages = Vec::new();

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;

        if exclude
            .iter()
            .any(|d| entry.path().starts_with(String::from("./") + d))
        {
            continue;
        }

        if ty.is_dir() {
            pages.append(&mut copy_dir(
                entry.path(),
                dst.as_ref().join(entry.file_name()),
                exclude,
                verbosity,
            )?);
        } else if !entry
            .path()
            .extension()
            .is_some_and(|ext| ext.to_str().unwrap_or("") == "html")
        {
            if verbosity == Verbosity::High {
                println!(
                    "  {}",
                    dst.as_ref()
                        .join(entry.file_name())
                        .to_str()
                        .unwrap_or("???")
                );
            }
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            pages.push((entry.path(), dst.as_ref().join(entry.file_name())));
        }
    }

    Ok(pages)
}
