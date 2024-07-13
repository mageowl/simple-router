use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    time::Instant,
};

use ::xml::reader::XmlEvent;
use config::Config;
use xml::Template;

mod config;
mod xml;

fn main() {
    let time_start = Instant::now();

    let config = fs::read_to_string("simple-router.toml")
        .expect("No config file found at ./simple-router.toml");
    let config: Config = toml::from_str(&config).expect("Failed to parse config file.");

    println!("Creating output directory at {}", config.out.path);
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
            .map(|p| PathBuf::from(p))
            .collect(),
    )
    .unwrap();
    println!("Done!");

    print!("Parsing template at {} ", config.source.template);
    let template = Template::parse_from_file(
        &Path::new(&config.source.template),
        config.xml.into(),
        config.out.lib_file.clone(),
    )
    .unwrap();
    println!("Done!");

    println!("Generating static site in {} ", config.out.path);
    for (page, page_out) in pages {
        if page.to_str() == Some(&config.source.template) {
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
        props.insert(
            "__filename".to_string(),
            vec![XmlEvent::Characters(String::from(
                page_out
                    .strip_prefix(&config.out.path)
                    .unwrap()
                    .to_str()
                    .unwrap(),
            ))],
        );

        let source = BufReader::new(File::open(page.clone()).unwrap());

        let out_json = BufWriter::new(File::create(page_out.with_extension("page.json")).unwrap());

        println!("  {}", page_out.as_os_str().to_str().unwrap());
        let out = BufWriter::new(File::create(page_out).unwrap());

        template
            .write_to_file(source, out, out_json, props)
            .unwrap();
    }
    println!("Done!");

    let mut library_path = PathBuf::from(&config.out.path);
    library_path.push(&config.out.lib_file);
    print!(
        "Adding library file at {} ",
        library_path.to_str().unwrap_or("")
    );
    File::create(library_path)
        .unwrap()
        .write_all(include_bytes!("simple_router.js"))
        .unwrap();
    println!("Done!");

    println!(
        "Generation finished in {:.1}s.",
        time_start.elapsed().as_secs_f64()
    )
}

// From StackOverflow: https://stackoverflow.com/a/65192210
fn copy_dir(
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
    exclude: &Vec<PathBuf>,
) -> io::Result<Vec<(PathBuf, PathBuf)>> {
    fs::create_dir_all(&dst)?;
    let mut pages = Vec::new();

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;

        if exclude.contains(&entry.path()) {
            continue;
        }

        if ty.is_dir() {
            pages.append(&mut copy_dir(
                entry.path(),
                dst.as_ref().join(entry.file_name()),
                exclude,
            )?);
        } else if !entry
            .path()
            .extension()
            .is_some_and(|ext| ext.to_str().unwrap() == "html")
        {
            println!(
                "  {}",
                dst.as_ref()
                    .join(entry.file_name())
                    .to_str()
                    .unwrap_or("???")
            );
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            pages.push((entry.path(), dst.as_ref().join(entry.file_name())));
        }
    }

    Ok(pages)
}
