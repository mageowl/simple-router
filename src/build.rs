use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, BufReader, BufWriter, Write},
    iter,
    path::{Path, PathBuf},
};

use crate::config::Config;
use crate::xml::Template;
use xml::reader::XmlEvent;

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

pub fn build(verbosity: Verbosity, config: Config) {
    if verbosity == Verbosity::High {
        println!("Creating output directory at {}", config.out.path);
    }
    if fs::metadata(&config.out.path).is_ok_and(|m| m.is_dir()) {
        fs::remove_dir_all(&config.out.path).unwrap();
    }
    let pages = copy_dir(
        &config.source.path,
        &config.out.path,
        &config
            .source
            .exclude
            .iter()
            .chain(iter::once(&config.out.path))
            .collect(),
        verbosity,
    )
    .unwrap();
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
    .unwrap();
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
                    .unwrap()
                    .with_extension("")
                    .to_str()
                    .unwrap(),
            ))],
        );

        let source = BufReader::new(File::open(page.clone()).unwrap());

        let out_json = BufWriter::new(File::create(page_out.with_extension("page.json")).unwrap());

        if verbosity == Verbosity::High {
            println!("  {}", page.as_os_str().to_str().unwrap());
        }

        let out = BufWriter::new(File::create(page_out).unwrap());

        template
            .write_to_file(source, out, out_json, props)
            .unwrap();
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
    File::create(library_path)
        .unwrap()
        .write_all(include_bytes!("simple_router.js"))
        .unwrap();
    if verbosity == Verbosity::High {
        println!("Done!");
    }
}

// From StackOverflow: https://stackoverflow.com/a/65192210
fn copy_dir(
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
    exclude: &Vec<&String>,
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
            .is_some_and(|ext| ext.to_str().unwrap() == "html")
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
