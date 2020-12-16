/*!
Simple functions intended to use in __Rust__ `build.rs` scripts for tasks which related to fetching from _HTTP_ and unrolling `.tar.gz` archives with precompiled binaries and etc.

```
use fetch_unroll::Fetch;

let pack_url = format!(
    concat!("{base}/{user}/{repo}/releases/download/",
            "{package}-{version}/{package}_{target}_{profile}.tar.gz"),
    base = "https://github.com",
    user = "katyo",
    repo = "aubio-rs",
    package = "libaubio",
    version = "0.5.0-alpha",
    target = "armv7-linux-androideabi",
    profile = "debug",
);

let dest_dir = "target/test_download";

// Fetching and unrolling archive
Fetch::from(pack_url)
    .unroll().strip_components(1).to(dest_dir)
    .unwrap();
```
 */

#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    //clippy::cargo,
)]

use std::{
    path::{Path, PathBuf},
    fs::{File, remove_file, create_dir_all, remove_dir_all},
    io::{Error as IoError, Read, Cursor, copy},
    error::{Error as StdError},
    result::{Result as StdResult},
    fmt::{Display, Formatter, Result as FmtResult},
};

use ureq::{Error as HttpError, get as http_get};
use libflate::gzip::{Decoder as GzipDecoder};
use tar::{Archive as TarArchive, EntryType as TarEntryType};

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
    #[must_use]
    fn from(error: &HttpError) -> Self {
        use self::HttpError::*;

        Self::Http(match error {
            BadUrl(url) => format!("Invalid url: {}", url),
            UnknownScheme(scheme) => format!("Unsupported scheme: {}", scheme),
            DnsFailed(dns) => format!("Unresolved domain name: {}", dns),
            ConnectionFailed(error) => format!("Reset connection: {}", error),
            TooManyRedirects => "Infinite redirect loop".to_string(),
            BadStatus => "Invalid status".to_string(),
            BadHeader => "Unable to read headers".to_string(),
            Io(error) => format!("Network error: {}", error),
            BadProxy => "Bad proxy".to_string(),
            ProxyConnect => "Proxy connection error".to_string(),
            BadProxyCreds => "Bad proxy creds".to_string(),
            InvalidProxyCreds => "Invalid proxy creds".to_string(),
        })
    }
}

impl From<IoError> for Error {
    #[must_use]
    fn from(error: IoError) -> Self {
        Self::Io(error)
    }
}

/// Fetch archive from HTTP(S) server and unroll to local directory
#[deprecated]
#[allow(deprecated)]
pub fn fetch_unroll<U: AsRef<str>, D: AsRef<Path>>(href: U, path: D) -> Status {
    unroll(fetch(href)?, path)
}

/// Fetch contents from HTTP(S) server and return a reader on success
#[deprecated]
pub fn fetch<U: AsRef<str>>(href: U) -> Result<impl Read> {
    http_fetch(href.as_ref())
}

/// Unroll packed data
///
/// *NOTE*: Currently supported __.tar.gz__ archives only.
#[deprecated]
pub fn unroll<S: Read, D: AsRef<Path>>(pack: S, path: D) -> Status {
    let unpacker = GzipDecoder::new(pack)?;
    let mut extractor = TarArchive::new(unpacker);

    extractor.unpack(path)?;
    Ok(())
}

/// HTTP(S) fetcher
pub struct Fetch<R> {
    source: Result<R>,
}

#[allow(clippy::use_self)]
impl Fetch<()>
{
    /// Fetch data from url
    pub fn from<U>(url: U) -> Fetch<impl Read>
    where
        U: AsRef<str>,
    {
        Fetch { source: http_fetch(url.as_ref()) }
    }
}

fn http_fetch(url: &str) -> Result<impl Read> {
    let response = http_get(url).call();

    if response.status() == 403 {
        return Err(Error::Http(format!("Denied: {}", url)));
    }

    if response.status() == 404 {
        return Err(Error::Http(format!("Not found: {}", url)));
    }

    if let Some(error) = response.synthetic_error() {
        return Err(Error::from(error));
    }

    Ok(response.into_reader())
}

impl<R> Fetch<R>
where
    R: Read
{
    /// Write fetched data to file
    pub fn save(self) -> Save<impl Read> {
        Save::from(self.source)
    }

    /// Unroll fetched archive
    pub fn unroll(self) -> Unroll<impl Read> {
        Unroll::from(self.source)
    }
}

/// File writer
pub struct Save<R> {
    source: Result<R>,
    options: SaveOptions,
}

struct SaveOptions {
    create_dest_path: bool,
    force_overwrite: bool,
    fix_invalid_dest: bool,
    cleanup_on_error: bool,
}

impl Default for SaveOptions {
    fn default() -> Self {
        Self {
            create_dest_path: true,
            force_overwrite: true,
            fix_invalid_dest: true,
            cleanup_on_error: true,
        }
    }
}

impl<R> From<Result<R>> for Save<R> {
    fn from(source: Result<R>) -> Self {
        Self {
            source,
            options: SaveOptions::default(),
        }
    }
}

impl<R> Save<R> {
    /// Create destination directory when it doesn't exists
    ///
    /// Default: `true`
    pub const fn create_dest_path(mut self, flag: bool) -> Self {
        self.options.create_dest_path = flag;
        self
    }

    /// Overwrite existing file
    ///
    /// Default: `true`
    pub const fn force_overwrite(mut self, flag: bool) -> Self {
        self.options.force_overwrite = flag;
        self
    }

    /// Try to fix destination path when it is not a valid
    ///
    /// For example, when destination already exists
    /// and it is a directory, it will be removed
    ///
    /// Default: `true`
    pub const fn fix_invalid_dest(mut self, flag: bool) -> Self {
        self.options.fix_invalid_dest = flag;
        self
    }

    /// Cleanup already written data when errors occurs
    ///
    /// Default: `true`
    pub const fn cleanup_on_error(mut self, flag: bool) -> Self {
        self.options.cleanup_on_error = flag;
        self
    }
}

impl<R> Save<R> {
    /// Save file to specified path
    pub fn to<D>(self, path: D) -> Status
    where
        R: Read,
        D: AsRef<Path>,
    {
        let Self { source, options } = self;

        let mut source = source?;

        let path = path.as_ref();

        if path.is_file() {
            if options.force_overwrite {
                remove_file(path)?;
            } else {
                return Ok(());
            }
        } else if path.is_dir() {
            if options.fix_invalid_dest {
                remove_dir_all(path)?;
            }
        } else { // not exists
            if options.create_dest_path {
                if let Some(path) = path.parent() {
                    create_dir_all(path)?;
                }
            }
        }

        copy(&mut source, &mut File::create(path)?)
            .map(|_| ())
            .or_else(|error| {
                if options.cleanup_on_error && path.is_file() {
                    remove_file(path)?;
                }
                Err(error)
            })?;

        Ok(())
    }
}

/// Archive unroller
///
/// *NOTE*: Currently supported __.tar.gz__ archives only.
pub struct Unroll<R> {
    source: Result<R>,
    options: UnrollOptions,
}

struct UnrollOptions {
    create_dest_path: bool,
    cleanup_dest_dir: bool,
    fix_invalid_dest: bool,
    cleanup_on_error: bool,
    strip_components: usize,
    strip_when_alone: bool,
}

impl Default for UnrollOptions {
    fn default() -> Self {
        Self {
            create_dest_path: true,
            cleanup_dest_dir: true,
            fix_invalid_dest: true,
            cleanup_on_error: true,
            strip_components: 0,
            strip_when_alone: false,
        }
    }
}

impl<R> From<Result<R>> for Unroll<R> {
    fn from(source: Result<R>) -> Self {
        Self {
            source,
            options: UnrollOptions::default(),
        }
    }
}

impl<R> Unroll<R> {
    /// Create destination directory when it doesn't exists
    ///
    /// Default: `true`
    pub const fn create_dest_path(mut self, flag: bool) -> Self {
        self.options.create_dest_path = flag;
        self
    }

    /// Cleanup destination directory before extraction
    ///
    /// Default: `true`
    pub const fn cleanup_dest_dir(mut self, flag: bool) -> Self {
        self.options.cleanup_dest_dir = flag;
        self
    }

    /// Try to fix destination path when it is not a valid
    ///
    /// For example, when destination already exists
    /// and it is not a directory, it will be removed
    ///
    /// Default: `true`
    pub const fn fix_invalid_dest(mut self, flag: bool) -> Self {
        self.options.fix_invalid_dest = flag;
        self
    }

    /// Cleanup already extracted data when errors occurs
    ///
    /// Default: `true`
    pub const fn cleanup_on_error(mut self, flag: bool) -> Self {
        self.options.cleanup_on_error = flag;
        self
    }

    /// Strip the number of leading components from file names on extraction
    ///
    /// Default: `0`
    pub const fn strip_components(mut self, num_of_components: usize) -> Self {
        self.options.strip_components = num_of_components;
        self
    }

    /// Strip the leading components only when it's alone
    ///
    /// Default: `false`
    pub const fn strip_when_alone(mut self, flag: bool) -> Self {
        self.options.strip_when_alone = flag;
        self
    }
}

impl<R> Unroll<R> {
    /// Extract contents to specified directory
    pub fn to<D>(self, path: D) -> Status
    where
        R: Read,
        D: AsRef<Path>,
    {
        let Self { source, options } = self;

        let source = source?;

        let path = path.as_ref();
        let mut dest_already_exists = false;

        if path.is_dir() {
            dest_already_exists = true;

            if options.cleanup_dest_dir {
                remove_dir_entries(path)?;
            }
        } else if path.is_file() {
            //dest_already_exists = true;

            if options.fix_invalid_dest {
                remove_file(path)?;

                if options.create_dest_path {
                    create_dir_all(path)?;
                }
            }
        } else { // not exists
            if options.create_dest_path {
                create_dir_all(path)?;
            }
        }

        unroll_archive_to(source, &options, path)
            .or_else(|error| {
                if options.cleanup_on_error && path.is_dir() {
                    if dest_already_exists {
                        remove_dir_entries(path)?;
                    } else {
                        remove_dir_all(path)?;
                    }
                }
                Err(error)
            })
    }
}

fn unroll_archive_to<R>(source: R, options: &UnrollOptions, destin: &Path) -> Status
where
    R: Read,
{
    let mut decoder = GzipDecoder::new(source)?;

    if options.strip_components < 1 {
        let mut archive = TarArchive::new(decoder);
        archive.unpack(destin)?;
        Ok(())
    } else {
        let mut decoded_data = Vec::new();
        decoder.read_to_end(&mut decoded_data)?;

        let strip_components = if options.strip_when_alone {
            let mut archive = TarArchive::new(Cursor::new(&decoded_data));
            options.strip_components.min(count_common_components(&mut archive)?)
        } else {
            options.strip_components
        };

        let mut archive = TarArchive::new(Cursor::new(decoded_data));
        let entries = archive.entries()?;

        for entry in entries {
            use self::TarEntryType::*;

            let mut entry = entry?;
            let type_ = entry.header().entry_type();

            {
                let entry_path = entry.path()?;

                match type_ {
                    Directory => {
                        let stripped_path = entry_path.iter().skip(strip_components).collect::<PathBuf>();
                        if stripped_path.iter().count() < 1 {
                            continue;
                        }
                        let dest_path = destin.join(stripped_path);

                        //create_dir_all(dest_path);
                        entry.unpack(dest_path)?;
                    },
                    Regular => {
                        let strip_components = strip_components.min(entry_path.iter().count() - 1);
                        let stripped_path = entry_path.iter().skip(strip_components).collect::<PathBuf>();
                        let dest_path = destin.join(stripped_path);

                        entry.unpack(dest_path)?;
                    },
                    _ => println!("other: {:?}", entry_path),
                }
            }
        }

        Ok(())
    }
}

fn count_common_components<R>(archive: &mut TarArchive<R>) -> StdResult<usize, IoError>
where
    R: Read,
{
    let mut common_ancestor = None;

    for entry in archive.entries()? {
        use self::TarEntryType::*;

        let entry = entry?;
        let entry_path = entry.path()?;

        match entry.header().entry_type() {
            Directory | Regular => {
                if common_ancestor.is_none() {
                    common_ancestor = Some(entry_path.to_path_buf());
                } else {
                    let common_ancestor = common_ancestor.as_mut().unwrap();

                    *common_ancestor = common_ancestor.iter().zip(entry_path.iter())
                        .take_while(|(common_component, entry_component)| {
                            common_component == entry_component
                        }).map(|(common_component, _)| common_component).collect();
                }
            },
            _ => (),
        }
    }

    Ok(if let Some(path) = common_ancestor {
        path.iter().count()
    } else {
        0
    })
}

fn remove_dir_entries(path: &Path) -> StdResult<(), IoError> {
    for entry in path.read_dir()? {
        let path = entry?.path();
        if path.is_file() {
            remove_file(path)?;
        } else {
            remove_dir_all(path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

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
        #[allow(deprecated)]
        fetch_unroll(src_url, dst_dir).unwrap();

        //std::fs::remove_dir_all(dst_dir).unwrap();
    }

    #[test]
    fn github_archive_new() {
        let src_url = format!(
            "{base}/{user}/{repo}/archive/{ver}.tar.gz",
            base = "https://github.com",
            user = "katyo",
            repo = "fluidlite",
            ver = "1.2.0",
        );

        let dst_dir = "target/test_archive_new";

        // Fetching and unrolling archive (new way)
        Fetch::from(src_url).unroll()
            .strip_components(1)
            .strip_when_alone(true)
            .to(dst_dir)
            .unwrap();

        //std::fs::remove_dir_all(dst_dir).unwrap();
    }
}
