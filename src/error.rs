use hyper::error::{Error as HyperError, ParseError};
use hyper_native_tls::native_tls;

use std::error::Error as StdError;
use std::io;
use std::path::PathBuf;
use std::result::Result as StdResult;

#[derive(Debug)]
pub enum Error {
    Http(HyperError),
    UrlParse(ParseError),
    Tls(native_tls::Error),
    Json(::serde_json::Error),
    Xdg(io::Error),
    NotADirectory(PathBuf),
    Clipboard(Box<StdError>),
    Watch(io::Error),
    ServerError,
}

impl From<HyperError> for Error {
    fn from(err: HyperError) -> Error {
        Error::Http(err)
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Error {
        Error::UrlParse(err)
    }
}

impl From<native_tls::Error> for Error {
    fn from(err: native_tls::Error) -> Error {
        Error::Tls(err)
    }
}

impl From<::serde_json::Error> for Error {
    fn from(err: ::serde_json::Error) -> Error {
        Error::Json(err)
    }
}

pub type Result<T> = StdResult<T, Error>;
