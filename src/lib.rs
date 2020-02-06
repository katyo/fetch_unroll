/*!
Simple functions intended to use in __Rust__ `build.rs` scripts for tasks which related to fetching from _HTTP_ and unrolling `.tar.gz` archives with precompiled binaries and etc.

```
use fetch_unroll::fetch_unroll;

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
fetch_unroll(pack_url, dest_dir).unwrap();
```
 */

use std::{
    path::Path,
    io::{Error as IoError, Read},
    error::{Error as StdError},
    result::{Result as StdResult},
    fmt::{Display, Formatter, Result as FmtResult},
};

use ureq::{Error as HttpError, get as http_get};
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
    Http(String),

    /// Generic IO error
    Io(IoError),
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
        }
    }
}

impl From<&HttpError> for Error {
    fn from(error: &HttpError) -> Self {
        use self::HttpError::*;

        match error {
            BadUrl(url) => Error::Http(format!("Invalid url: {}", url)),
            UnknownScheme(scheme) => Error::Http(format!("Unsupported scheme: {}", scheme)),
            DnsFailed(dns) => Error::Http(format!("Unresolved domain name: {}", dns)),
            ConnectionFailed(error) => Error::Http(format!("Reset connection: {}", error)),
            TooManyRedirects => Error::Http(format!("Infinite redirect loop")),
            BadStatusRead => Error::Http(format!("Unable to read status")),
            BadStatus => Error::Http(format!("Invalid status")),
            BadHeader => Error::Http(format!("Unable to read headers")),
            Io(error) => Error::Http(format!("Network error: {}", error)),
        }
    }
}

impl From<IoError> for Error {
    fn from(error: IoError) -> Self {
        Error::Io(error)
    }
}

/// Fetch archive from HTTP(S) server and unroll to local directory
pub fn fetch_unroll<U: AsRef<str>, D: AsRef<Path>>(href: U, path: D) -> Status {
    unroll(fetch(href)?, path)
}

/// Fetch contents from HTTP(S) server and return a reader on success
pub fn fetch<U: AsRef<str>>(href: U) -> Result<impl Read> {
    let response = http_get(href.as_ref()).call();

    if let Some(error) = response.synthetic_error() {
        return Err(Error::from(error));
    }

    Ok(response.into_reader())
}

/// Unroll packed data
///
/// *NOTE*: Currently supported __.tar.gz__ archives only.
pub fn unroll<S: Read, D: AsRef<Path>>(pack: S, path: D) -> Status {
    let unpacker = Decoder::new(pack).map_err(Error::from)?;
    let mut extractor = Archive::new(unpacker);

    extractor.unpack(path).map_err(Error::from)
}

#[cfg(test)]
mod test {
    use super::fetch_unroll;

    #[test]
    fn github_archive() {
        let src_url = format!(
            "{base}/{user}/{repo}/archive/{ver}.tar.gz",
            base = "https://github.com",
            user = "katyo",
            repo = "fluidlite",
            ver = "1.2.0",
        );

        let dst_dir = "target/test_archive";

        // Creating destination directory
        std::fs::create_dir_all(dst_dir).unwrap();

        // Fetching and unrolling archive
        fetch_unroll(src_url, dst_dir).unwrap();

        //std::fs::remove_dir_all(dst_dir).unwrap();
    }
}
