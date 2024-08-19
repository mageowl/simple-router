use std::{
    ffi::OsStr,
    fs,
    io::{BufRead, BufReader, Write},
    iter,
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use mime_guess::MimeGuess;
use notify_debouncer_mini::{notify::RecursiveMode, DebounceEventResult};

use crate::{
    build::{self, Verbosity},
    config::Config,
};

pub fn start(port: u16, hostname: String, config: Config) {
    println!("\x1b[35m[BUILD]\x1b[0m Buildng website...");
    let time_start = Instant::now();

    if let Err(err) = build::build(Verbosity::Low, config.clone()) {
        println!("\x1b[31m[BUILD FAILED]\x1b[31m {err}")
    } else {
        println!(
            "\x1b[35m[BUILD]\x1b[0m Website built in {:.2}s.",
            time_start.elapsed().as_secs_f32()
        );
    }

    println!("\x1b[36m[SERVER]\x1b[0m Starting web server...");

    let directory: PathBuf = config.out.path.clone().into();
    let not_found: PathBuf = config.js.not_found.clone().into();
    thread::spawn(move || listen(port, hostname, &directory, &not_found));

    let cfg = config.clone();
    let mut skip_next = false;
    let mut debouncer = notify_debouncer_mini::new_debouncer(Duration::from_secs(1), move |res| {
        if !skip_next {
            handle_file_update(cfg.clone(), res);
        }
        skip_next = !skip_next;
    })
    .unwrap();

    debouncer
        .watcher()
        .watch(Path::new(&config.source.path), RecursiveMode::Recursive)
        .unwrap();

    loop {}
}

fn listen(port: u16, hostname: String, directory: &Path, not_found: &Path) {
    let listener = TcpListener::bind((hostname, port)).unwrap();

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let response = handle_connection(&mut stream, directory, not_found);

        stream.write_all(&response).unwrap();
    }
}

fn handle_connection(stream: &mut TcpStream, directory: &Path, not_found: &Path) -> Vec<u8> {
    let buf_reader = BufReader::new(stream);
    let request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    let req_header: Vec<&str> = request[0].split(" ").collect();
    let [method, path, ..] = &req_header[..] else {
        return b"HTTP/1.1 500 BAD REQUEST".to_vec();
    };

    match *method {
        "GET" => {
            let path_buf = Path::new(path.strip_prefix("/").unwrap_or(path));
            let mut file = directory.join(path_buf);

            if file.is_dir() {
                file.push("index.html")
            }

            if !file.exists() {
                println!(
                    "\x1b[31m[404]\x1b[0m Not found: ./{}",
                    file.to_str().unwrap()
                );
                file = directory.join(not_found);
                if !file.exists() {
                    return format!(
                        "HTTP/1.1 404 NOT FOUND\r\n\r\nCannot {method} {path}",
                        path = path.to_string(),
                    )
                    .as_bytes()
                    .to_vec();
                }
            } else if file.extension() == Some(OsStr::new("html"))
                || file.extension() == Some(OsStr::new("json"))
            {
                println!("\x1b[32m[GET]\x1b[0m ./{}", file.to_str().unwrap());
            }

            let status = "HTTP/1.1 200 OK";
            let mime_type = MimeGuess::from_path(&file)
                .first()
                .map_or(String::new(), |mime| mime.essence_str().to_owned());
            let mut contents = fs::read(file).unwrap();
            let length = contents.len();

            let mut response = format!(
                "{status}\r\nContent-Length: {length}\r\nContent-Type: {mime_type}\r\n\r\n"
            )
            .as_bytes()
            .to_vec();
            response.append(&mut contents);
            response
        }
        _ => {
            println!("\x1b[31m[{method}]\x1b[0m {path}");
            format!("HTTP/1.1 404 NOT FOUND\r\n\r\nCannot {method} {path}")
                .as_bytes()
                .to_vec()
        }
    }
}

fn handle_file_update(config: Config, res: DebounceEventResult) {
    match res {
        Ok(events) => {
            if events.iter().any(|ev| {
                !config
                    .source
                    .exclude
                    .iter()
                    .chain(iter::once(&config.out.path))
                    .any(|d| ev.path.starts_with(String::from("./") + d))
            }) {
                println!("\x1b[35m[BUILD]\x1b[0m Changes detected, building...");
                let time_start = Instant::now();

                let result = build::build(Verbosity::Low, config.clone());

                match result {
                    Ok(_) => println!(
                        "\x1b[35m[BUILD]\x1b[0m Website built in {:.2}s.",
                        time_start.elapsed().as_secs_f32()
                    ),
                    Err(err) => println!("\x1b[31m[BUILD FAILED]\x1b[31m {err}"),
                }
            }
        }
        Err(e) => {
            println!("\x1b[35m[BUILD]\x1b[0m Error watching files: {e}");
        }
    }
}
