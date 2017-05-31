#[macro_use]
extern crate clap;
extern crate clipboard;
extern crate inotify;
extern crate hyper;
extern crate hyper_native_tls;
extern crate multipart;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::{AppSettings, Arg, SubCommand};
use clipboard::{ClipboardContext, ClipboardProvider};
use inotify::{Inotify, watch_mask};
use hyper::Url;
use hyper::client::Request;
use hyper::method::Method;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use multipart::client::Multipart;

static UPLOAD_URL: &str = "https://cocaine.ninja/upload.php";

#[derive(Debug, Deserialize)]
struct PomfFile {
    hash: String,
    name: String,
    url: String,
    size: usize,
}

#[derive(Debug, Deserialize)]
struct PomfResponse {
    success: bool,
    files: Vec<PomfFile>,
}

fn xdg_user_dir(dir: &str) -> PathBuf {
    let output = Command::new("xdg-user-dir")
        .arg(dir)
        .output()
        .expect(&format!("Couldn't get XDG_{}_DIR", dir));
    PathBuf::from(String::from_utf8_lossy(&output.stdout).trim())
}

struct Pomf {
    upload_url: Url,
    connector: HttpsConnector<NativeTlsClient>,
}

impl Pomf {
    fn new<S: AsRef<str>>(upload_url: S) -> Pomf {
        let tls = NativeTlsClient::new().expect("Failed to initialize HTTPS client");
        let upload_url: Url = upload_url
            .as_ref()
            .parse()
            .expect("Failed to parse upload url");
        Pomf {
            connector: HttpsConnector::new(tls),
            upload_url,
        }
    }

    fn upload<P: AsRef<Path>>(&self, path: P) -> Result<Url, Box<Error>> {
        let request =
            Request::with_connector(Method::Post, self.upload_url.clone(), &self.connector)?;
        let mut multipart = Multipart::from_request(request)?;
        multipart.write_file("files[]", path)?;
        let response: PomfResponse = serde_json::from_reader(multipart.send()?)?;
        response.files[0].url.parse().map_err(Into::into)
    }

    fn watch<P: AsRef<Path>>(&self, dir: P) {
        let dir = dir.as_ref();
        if !dir.is_dir() {
            panic!("Not a directory: {:?}", dir);
        }
        let mut clipboard: ClipboardContext =
            ClipboardProvider::new().expect("Failed to initialize clipboard provider");
        let mut inotify = Inotify::init().expect("Failed to initialize inotify");
        inotify
            .add_watch(dir, watch_mask::CLOSE_WRITE)
            .expect("Failed to add inotify watch");
        let mut buffer = [0u8; 4096];
        loop {
            let events = inotify.read_events_blocking(&mut buffer).unwrap();
            for event in events {
                let path = dir.join(event.name);
                println!("uploading {:?}...", path);
                match self.upload(path) {
                    Ok(url) => {
                        let url = url.to_string();
                        println!("{}", url);
                        let _ = clipboard.set_contents(url);
                    }
                    Err(err) => {
                        println!("error: {}", err);
                    }
                }
            }
        }
    }
}

fn main() {
    let matches =
        app_from_crate!()
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .subcommand(SubCommand::with_name("upload").arg(Arg::from_usage("[FILE]")
                                                                .help("Upload FILE")
                                                                .required(true)))
            .subcommand(SubCommand::with_name("watch")
                            .arg(Arg::from_usage("[DIR] 'Set up watch on DIR'")
                                     .default_value("XDG_PICTURES_DIR")))
            .arg(Arg::from_usage("--upload-url=[URL] 'Upload URL'").default_value(UPLOAD_URL))
            .get_matches();

    let upload_url = matches.value_of("upload-url").unwrap_or(UPLOAD_URL);
    let pomf = Pomf::new(upload_url);

    match matches.subcommand_name() {
        Some("upload") => {
            let matches = matches.subcommand_matches("upload").unwrap();
            let path = matches.value_of("FILE").unwrap();
            let url = pomf.upload(path).unwrap();
            println!("{}", url);
        }
        Some("watch") => {
            let matches = matches.subcommand_matches("watch").unwrap();
            let path = match matches.value_of("DIR") {
                Some("XDG_PICTURES_DIR") |
                None => xdg_user_dir(&"PICTURES"),
                Some(path) => PathBuf::from(path),
            };
            pomf.watch(path);
        }
        _ => (),
    }
}
