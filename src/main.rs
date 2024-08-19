use std::{fs, time::Instant};

use build::Verbosity;
use clap::{crate_name, crate_version, value_parser, Arg, Command};
use config::Config;

mod build;
mod config;
mod server;
mod xml;

fn main() {
    let mut cmd = Command::new(crate_name!())
        .subcommand(
            Command::new("build").about("Build static site.").arg(
                Arg::new("verbosity")
                    .long("verbosity")
                    .short('v')
                    .value_parser(["silent", "low", "high"])
                    .default_value("high"),
            ),
        )
        .subcommand(
            Command::new("dev")
                .about("Launch live web server.")
                .arg(
                    Arg::new("port")
                        .long("port")
                        .value_parser(value_parser!(u16))
                        .default_value("3000"),
                )
                .arg(Arg::new("host").long("host").default_value("localhost")),
        )
        .version(crate_version!());
    let matches = cmd.get_matches_mut();

    let config = match get_config() {
        Ok(c) => c,
        Err(msg) => {
            println!("\x1b[31mError reading config\x1b[0m: {msg}");
            return;
        }
    };

    match matches.subcommand() {
        Some(("build", subcmd)) => {
            let verbosity: Verbosity = subcmd
                .get_one::<String>("verbosity")
                .unwrap()
                .as_str()
                .try_into()
                .expect("Verbosity level must be silent, low, or high.");

            if verbosity >= Verbosity::Low {
                println!("\x1b[35mBuilding static site...\x1b[0m");
            }
            let time_start = Instant::now();

            let result = build::build(verbosity, config);

            if let Err(err) = result {
                println!("\n\x1b[31mErrors while building static site\x1b[0m: {err}.");
            } else {
                if verbosity >= Verbosity::Low {
                    println!(
                        "\x1b[32mWebsite built in {:.2}s.\x1b[0m",
                        time_start.elapsed().as_secs_f64()
                    )
                }
            }
        }
        Some(("dev", subcmd)) => {
            server::start(
                *subcmd.get_one("port").unwrap(),
                subcmd.get_one::<String>("host").unwrap().clone(),
                config,
            );
        }
        None => cmd.print_help().unwrap(),
        _ => (),
    }
}

fn get_config() -> Result<Config, String> {
    let file = fs::read_to_string("simple-router.toml")
        .map_err(|_| String::from("No file found at ./simple-router.toml."))?;
    let config: Config = toml::from_str(&file).map_err(|e| e.message().to_owned())?;

    if config.library_version != crate_version!() {
        Err(format!(
            "Incorrect config version. Using version {crate}, but config is on {conf}.",
            crate = crate_version!(),
            conf = config.library_version
        ))
    } else {
        Ok(config)
    }
}
