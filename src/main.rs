use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter},
    path::{Path, PathBuf},
};

use config::Config;
use xml::Template;

mod config;
mod xml;

fn main() {
    let config_file = fs::read_to_string("simple-router.toml")
        .expect("No config file found at ./simple-router.toml");
    let config: Config = toml::from_str(&config_file).expect("Failed to parse config file.");

    print!("Creating out directory at {} ", config.out.path);
    if fs::metadata(&config.out.path).is_ok_and(|m| m.is_dir()) {
        fs::remove_dir_all(&config.out.path).unwrap();
    }
    fs::create_dir(&config.out.path).unwrap();
    println!("Done!");

    println!("Finding pages in {}", config.source.path);
    let pages = get_pages(&Path::new(&config.source.path)).unwrap();
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
    for page in pages {
        if page.to_str() == Some(&config.source.template) {
            continue;
        }

        let source = BufReader::new(File::open(page.clone()).unwrap());

        let mut path_out = PathBuf::from(config.out.path.clone());
        path_out.push(page.strip_prefix(&config.source.path).unwrap());

        println!("  {}", path_out.as_os_str().to_str().unwrap());
        let out = BufWriter::new(File::create(path_out).unwrap());
        template.write_to_file(source, out).unwrap();
    }
    println!("Done!");
}

fn get_pages(dir_path: &Path) -> io::Result<Vec<PathBuf>> {
    let mut pages = Vec::new();

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            pages.append(&mut get_pages(&path)?);
        } else if path.extension().is_some_and(|ext| ext == "html") {
            println!("  {}", path.to_str().unwrap_or("???"));
            pages.push(path);
        }
    }

    Ok(pages)
}
