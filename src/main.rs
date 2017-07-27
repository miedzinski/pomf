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

mod error;

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

use clap::{AppSettings, Arg, SubCommand};
use clipboard::{ClipboardContext, ClipboardProvider};
use inotify::{Inotify, watch_mask};
use hyper::Url;
use hyper::client::Request;
use hyper::method::Method;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use multipart::client::Multipart;

use error::{Error, Result};

static UPLOAD_URL: &str = "https://cocaine.ninja/upload.php";

#[derive(Debug, Deserialize)]
struct File {
    hash: String,
    name: String,
    url: String,
    size: usize,
}

#[derive(Debug, Deserialize)]
struct Response {
    success: bool,
    files: Vec<File>,
}

fn xdg_user_dir(dir: &str) -> Result<PathBuf> {
    let output = process::Command::new("xdg-user-dir")
        .arg(dir)
        .output()
        .map_err(|e| Error::Xdg(e))?;
    Ok(PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
}

fn fail(msg: &str) -> ! {
    let _ = io::stderr().write_all(msg.as_bytes());
    process::exit(1);
}

struct Uploader {
    upload_url: Url,
    connector: HttpsConnector<NativeTlsClient>,
}

impl Uploader {
    fn new(upload_url: &str) -> Result<Uploader> {
        let tls = NativeTlsClient::new()?;
        let upload_url: Url = upload_url.parse()?;
        Ok(Uploader {
            connector: HttpsConnector::new(tls),
            upload_url,
        })
    }

    fn upload<P: AsRef<Path>>(&self, path: P) -> Result<Url> {
        let request =
            Request::with_connector(Method::Post, self.upload_url.clone(), &self.connector)?;
        let mut multipart = Multipart::from_request(request)?;
        multipart.write_file("files[]", path)?;
        let response: Response = serde_json::from_reader(multipart.send()?)?;
        if response.success && response.files.len() > 0 {
            Ok(response.files[0].url.parse()?)
        } else {
            Err(Error::ServerError)
        }
    }
}

struct Watcher {
    uploader: Uploader,
    clipboard: ClipboardContext,
    dir: PathBuf,
    watch: Inotify,
}

impl Watcher {
    fn new(uploader: Uploader, dir: PathBuf) -> Result<Watcher> {
        if !dir.is_dir() {
            return Err(Error::NotADirectory(dir));
        }
        let clipboard: ClipboardContext = ClipboardProvider::new()
            .map_err(|e| Error::Clipboard(e))?;
        let mut watch = Inotify::init().map_err(|e| Error::Watch(e))?;
        watch.add_watch(&dir, watch_mask::CLOSE_WRITE).map_err(|e| Error::Watch(e))?;
        Ok(Watcher { uploader, clipboard, dir, watch })
    }

    fn watch(mut self) {
        let mut buffer = [0u8; 4096];
        loop {
            let events = self.watch.read_events_blocking(&mut buffer).unwrap();
            for event in events {
                let path = self.dir.join(event.name);
                match self.uploader.upload(path) {
                    Ok(url) => {
                        let url = url.to_string();
                        println!("{}", url);
                        let _ = self.clipboard.set_contents(url);
                    }
                    Err(err) => {
                        eprintln!("failed to upload: {:?}", err);
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
    let uploader = match Uploader::new(upload_url) {
        Ok(pomf) => pomf,
        Err(err) => {
            fail(&format!("failed to initialize client: {:?}", err));
        }
    };

    match matches.subcommand_name() {
        Some("upload") => {
            let matches = matches.subcommand_matches("upload").unwrap();
            let path = matches.value_of("FILE").unwrap();
            match uploader.upload(path) {
                Ok(url) => println!("{}", url),
                Err(err) => fail(&format!("failed to upload: {:?}", err)),
            }
        }
        Some("watch") => {
            let matches = matches.subcommand_matches("watch").unwrap();
            let path = match matches.value_of("DIR") {
                Some("XDG_PICTURES_DIR") |
                None => match xdg_user_dir(&"PICTURES") {
                    Ok(path) => path,
                    Err(err) => fail(&format!("failed to get XDG_PICTURES_DIR: {:?}", err)),
                },
                Some(path) => PathBuf::from(path),
            };
            let watcher = match Watcher::new(uploader, path) {
                Ok(watcher) => watcher,
                Err(err) => {
                    fail(&format!("failed to initialize watch: {:?}", err));
                }
            };
            watcher.watch();
        }
        _ => (),
    }
}
