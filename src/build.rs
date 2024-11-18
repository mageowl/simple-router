use std::{
    collections::HashMap,
    fmt::Display,
    fs::{self, File},
    io::{self, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    process::Command,
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
    if let Some(cmd) = config.scripts.prebuild {
        if verbosity == Verbosity::High {
            println!("Running pre-build script... ");
        }
        let status = Command::new("sh").args(["-c", &cmd]).status()?;
        if status.success() {
            if verbosity == Verbosity::High {
                println!("Done!");
            }
        } else {
            return Err(BuildError::Other {
                msg: format!("Pre build script failed with exit code {status}"),
                source: None,
            });
        }
    }

    if verbosity == Verbosity::High {
        println!("Creating output directory at {}", config.out.path);
    }
    if let Ok(metadata) = fs::metadata(&config.out.path) {
        if metadata.is_dir() {
            fs::remove_dir_all(&config.out.path)?;
        } else {
            return Err(BuildError::Other {
                msg: format!(
                    "File exists at {path}, blocking output directory.",
                    path = config.out.path
                ),
                source: None,
            });
        }
    }

    if PathBuf::from(&config.source.static_path) == PathBuf::from(&config.source.pages_path) {
        return Err(BuildError::Other {
            msg: String::from("static_path cannot be the same as pages_path."),
            source: None,
        });
    }

    for (file, out) in scan_dir(
        &config.source.static_path,
        &config.out.path,
        &config
            .source
            .exclude
            .iter()
            .map(|s| s.as_str())
            .chain(
                [
                    &config.out.path,
                    &config.source.pages_path,
                    &config.source.template,
                    "simple-router.toml",
                ]
                .into_iter(),
            )
            .collect(),
        verbosity,
    )? {
        fs::copy(file, out)?;
    }

    if verbosity == Verbosity::High {
        println!("Done!");
    }

    let template_path = PathBuf::from(&config.source.template);
    if verbosity == Verbosity::High {
        print!("Parsing template at {} ", template_path.to_string_lossy());
    }
    let template = Template::parse_from_file(
        &template_path,
        config.xml.into(),
        config.out.lib_file.clone(),
    )
    .map_err(|err| {
        BuildError::from(err).with_source(template_path.to_string_lossy().to_string())
    })?;
    if verbosity == Verbosity::High {
        println!("Done!");
    }

    if verbosity == Verbosity::High {
        println!("Generating static site in {} ", config.out.path);
    }
    for (page, page_out) in scan_dir(
        &config.source.pages_path,
        &config.out.path,
        &config
            .source
            .exclude
            .iter()
            .map(|s| s.as_str())
            .chain(
                [
                    &config.out.path,
                    &config.source.static_path,
                    &config.source.template,
                    "simple-router.toml",
                ]
                .into_iter(),
            )
            .collect(),
        verbosity,
    )? {
        let mut props = HashMap::new();
        props.insert(
            "__path".to_string(),
            vec![XmlEvent::Characters(String::from(
                page_out
                    .strip_prefix(&config.out.path)
                    .unwrap_or(&page_out)
                    .with_extension("")
                    .to_string_lossy(),
            ))],
        );

        let source = BufReader::new(File::open(page.clone())?);

        let out_json = BufWriter::new(File::create(page_out.with_extension("page.json"))?);

        if verbosity == Verbosity::High {
            println!("  {}", page.to_string_lossy());
        }

        let is_404 = page_out.ends_with(Path::new(&config.js.not_found));

        let out = BufWriter::new(File::create(page_out)?);

        template
            .write_to_file(source, out, out_json, props, is_404)
            .map_err(|err| BuildError::from(err).with_source(page.to_string_lossy().to_string()))?;
    }
    if verbosity == Verbosity::High {
        println!("Done!");
    }

    let mut library_path = PathBuf::from(&config.out.path);
    library_path.push(&config.out.lib_file);
    if verbosity == Verbosity::High {
        print!("Adding library file at {} ", library_path.to_string_lossy());
    }

    let mut library = config.js.get_code().as_bytes().to_vec();
    library.extend_from_slice(include_bytes!("simple_router.js"));

    File::create(library_path)?.write_all(&library)?;
    if verbosity == Verbosity::High {
        println!("Done!");
    }

    if let Some(cmd) = config.scripts.postbuild {
        if verbosity == Verbosity::High {
            println!("Running post-build script... ");
        }

        let status = Command::new("sh").args(["-c", &cmd]).status()?;
        if status.success() {
            if verbosity == Verbosity::High {
                println!("Done!");
            }
        } else {
            return Err(BuildError::Other {
                msg: format!("Post-build script failed with exit code {status}"),
                source: None,
            });
        }
    }

    Ok(())
}

// From StackOverflow: https://stackoverflow.com/a/65192210 + modifications
fn scan_dir(
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
    exclude: &Vec<&str>,
    verbosity: Verbosity,
) -> io::Result<Vec<(PathBuf, PathBuf)>> {
    let mut entries = Vec::new();
    fs::create_dir_all(dst.as_ref())?;

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
            entries.append(&mut scan_dir(
                entry.path(),
                dst.as_ref().join(entry.file_name()),
                exclude,
                verbosity,
            )?);
        } else {
            if verbosity == Verbosity::High {
                println!(
                    "  {}",
                    dst.as_ref().join(entry.file_name()).to_string_lossy()
                );
            }
            entries.push((entry.path(), dst.as_ref().join(entry.file_name())));
        }
    }

    Ok(entries)
}
