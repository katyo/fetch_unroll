/*!
Simple functions intended to use in __Rust__ `build.rs` scripts for tasks which related to fetching from _HTTP_ and unrolling `.tar.gz` archives with precompiled binaries and etc.

```
use fetch_unroll::{
    Config,
    fetch_unroll,
};

let pack_url = format!(
    "{base}/{user}/{repo}/releases/download/{ver}/{pkg}_{prof}.tar.gz",
    base = "https://github.com",
    user = "katyo",
    repo = "oboe-rs",
    pkg = "liboboe-ext",
    ver = "0.1.0",
    prof = "release",
);

let dest_dir = "target/test_download";

// Creating destination directory
std::fs::create_dir_all(dest_dir).unwrap();

// Fetching and unrolling archive
fetch_unroll(pack_url, dest_dir, Config::default()).unwrap();
```
 */

use std::{
    path::Path,
    io::{Error as IoError, Cursor, Read},
    error::{Error as StdError},
    result::{Result as StdResult},
    fmt::{Display, Formatter, Result as FmtResult},
};

use http_req::{
    request::{get as http_get},
    error::{Error as HttpError},
};
use libflate::gzip::Decoder;
use tar::Archive;

/// Result type
pub type Result<T> = StdResult<T, Error>;

/// Status type
///
/// The result without payload
pub type Status = Result<()>;

/// Error type
#[derive(Debug)]
pub enum Error {
    /// Generic HTTP error
    Http(HttpError),

    /// Generic IO error
    Io(IoError),

    /// Redirect error
    Redirect(String),

    /// Invalid response status
    Status(&'static str),
}

impl StdError for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        use self::Error::*;
        match self {
            Http(error) => {
                "Http error: ".fmt(f)?;
                error.fmt(f)
            },
            Io(error) => {
                "IO error: ".fmt(f)?;
                error.fmt(f)
            },
            Status(error) => {
                "Invalid status: ".fmt(f)?;
                error.fmt(f)
            },
            Redirect(href) => {
                "Redirect loop: \"".fmt(f)?;
                href.fmt(f)?;
                "\"".fmt(f)
            },
        }
    }
}

impl From<HttpError> for Error {
    fn from(error: HttpError) -> Self {
        Error::Http(error)
    }
}

impl From<IoError> for Error {
    fn from(error: IoError) -> Self {
        Error::Io(error)
    }
}

impl From<&'static str> for Error {
    fn from(error: &'static str) -> Self {
        Error::Status(error)
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Error::Redirect(error)
    }
}

/// Configuration options
pub struct Config {
    /// The maximum number of redirects
    pub redirect_limit: usize,
}

/// Default limit for redirect
const DEFAULT_REDIRECT_LIMIT: usize = 20;

impl Default for Config {
    fn default() -> Self {
        Self {
            redirect_limit: DEFAULT_REDIRECT_LIMIT,
        }
    }
}

impl AsRef<Config> for Config {
    fn as_ref(&self) -> &Self {
        self
    }
}

/// Fetch archive from url and unroll to directory
pub fn fetch_unroll<U: AsRef<str>, D: AsRef<Path>, C: AsRef<Config>>(href: U, path: D, conf: C) -> Status {
    unroll(fetch(href, conf)?, path)
}

/// Fetch url with limited redirect
pub fn fetch<U: AsRef<str>, C: AsRef<Config>>(href: U, conf: C) -> Result<Cursor<Vec<u8>>> {
    let mut href = String::from(href.as_ref());
    let mut limit = conf.as_ref().redirect_limit;
    loop {
        return match fetch_raw(href) {
            Ok(body) => Ok(body),
            Err(Error::Redirect(location)) => {
                limit -= 1;
                if limit > 0 {
                    href = location;
                    continue;
                } else {
                    Err(Error::Redirect(location))
                }
            },
            Err(error) => Err(error),
        };
    }
}

/// Fetch url without redirects
fn fetch_raw<U: AsRef<str>>(href: U) -> Result<Cursor<Vec<u8>>> {
    let mut body = Vec::new();
    let response = http_get(href, &mut body).map_err(Error::from)?;

    let status_code = response.status_code();

    if status_code.is_redirect() {
        if let Some(href) = response.headers().get("Location") {
            return Err(Error::from(href.clone()));
        }
    }

    if status_code.is_success() {
        Ok(Cursor::new(body))
    } else {
        Err(Error::from(status_code.reason().unwrap_or("Wrong")))
    }
}

/// Unroll packed data (.tar.gz)
pub fn unroll<S: Read, D: AsRef<Path>>(pack: S, path: D) -> Status {
    let unpacker = Decoder::new(pack).map_err(Error::from)?;
    let mut extractor = Archive::new(unpacker);

    extractor.unpack(path).map_err(Error::from)
}
