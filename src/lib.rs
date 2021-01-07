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
    error::Error as StdError,
    fmt::{Display, Formatter, Result as FmtResult},
    fs::{create_dir_all, remove_dir_all, remove_file, File},
    io::{copy, Cursor, Error as IoError, Read},
    path::{Path, PathBuf},
    result::Result as StdResult,
};

use libflate::gzip::Decoder as GzipDecoder;
use tar::{Archive as TarArchive, EntryType as TarEntryType};
use ureq::{get as http_get, Error as HttpError};

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
        match self {
            Self::Http(error) => {
                "Http error: ".fmt(f)?;
                error.fmt(f)
            }
            Self::Io(error) => {
                "IO error: ".fmt(f)?;
                error.fmt(f)
            }
        }
    }
}

impl From<&HttpError> for Error {
    #[must_use]
    fn from(error: &HttpError) -> Self {
        // Map the error to our error type.
        Self::Http(match error {
            HttpError::Status(code, _) => {
                format!("Invalid status: {}", code)
            }
            HttpError::Transport(transport) => {
                format!("Transport error: {}", transport)
            }
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
type Flag = u8;

const CREATE_DEST_PATH: Flag = 1 << 0;
const FORCE_OVERWRITE: Flag = 1 << 1;
const FIX_INVALID_DEST: Flag = 1 << 2;
const CLEANUP_ON_ERROR: Flag = 1 << 3;
const CLEANUP_DEST_DIR: Flag = 1 << 4;
const STRIP_WHEN_ALONE: Flag = 1 << 5;

const DEFAULT_SAVE_FLAGS: Flag =
    CREATE_DEST_PATH | FORCE_OVERWRITE | FIX_INVALID_DEST | CLEANUP_ON_ERROR;
const DEFAULT_UNROLL_FLAGS: Flag =
    CREATE_DEST_PATH | FIX_INVALID_DEST | CLEANUP_ON_ERROR | CLEANUP_DEST_DIR;

macro_rules! flag {
    // Get flag
    ($($var:ident).* [$key:ident]) => {
        ($($var).* & $key) == $key
    };

    // Set flag
    ($($var:ident).* [$key:ident] = $val:expr) => {
        if $val {
            $($var).* |= $key;
        } else {
            $($var).* &= !$key;
        }
    };
}

/// HTTP(S) fetcher
pub struct Fetch<R> {
    source: Result<R>,
}

#[allow(clippy::use_self)]
impl Fetch<()> {
    /// Fetch data from url
    pub fn from<U>(url: U) -> Fetch<impl Read>
    where
        U: AsRef<str>,
    {
        Fetch {
            source: http_fetch(url.as_ref()),
        }
    }
}

fn http_fetch(url: &str) -> Result<impl Read> {
    match http_get(url).call() {
        Ok(response) => Ok(response.into_reader()),
        Err(error) => {
            // Map the error to our error type.
            Err(Error::from(&error))
        }
    }
}

impl<R> Fetch<R>
where
    R: Read,
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
    flags: Flag,
}

impl Default for SaveOptions {
    fn default() -> Self {
        Self {
            flags: DEFAULT_SAVE_FLAGS,
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
        flag! { self.options.flags[CREATE_DEST_PATH] = flag }
        self
    }

    /// Overwrite existing file
    ///
    /// Default: `true`
    pub const fn force_overwrite(mut self, flag: bool) -> Self {
        flag! { self.options.flags[FORCE_OVERWRITE] = flag }
        self
    }

    /// Try to fix destination path when it is not a valid
    ///
    /// For example, when destination already exists
    /// and it is a directory, it will be removed
    ///
    /// Default: `true`
    pub const fn fix_invalid_dest(mut self, flag: bool) -> Self {
        flag! { self.options.flags[FIX_INVALID_DEST] = flag }
        self
    }

    /// Cleanup already written data when errors occurs
    ///
    /// Default: `true`
    pub const fn cleanup_on_error(mut self, flag: bool) -> Self {
        flag! { self.options.flags[CLEANUP_ON_ERROR] = flag }
        self
    }
}

impl<R> Save<R> {
    /// Save file to specified path
    ///
    /// # Errors
    /// - Destination directory does not exists when `create_dest_path` is not set
    /// - File already exist at destination directory when `force_overwrite` is not set
    /// - Destination path is not a file when `fix_invalid_dest` is not set
    pub fn to<D>(self, path: D) -> Status
    where
        R: Read,
        D: AsRef<Path>,
    {
        let Self { source, options } = self;

        let mut source = source?;

        let path = path.as_ref();

        if path.is_file() {
            if flag!(options.flags[FORCE_OVERWRITE]) {
                remove_file(path)?;
            } else {
                return Ok(());
            }
        } else if path.is_dir() {
            if flag!(options.flags[FIX_INVALID_DEST]) {
                remove_dir_all(path)?;
            }
        } else {
            // not exists
            if flag!(options.flags[CREATE_DEST_PATH]) {
                if let Some(path) = path.parent() {
                    create_dir_all(path)?;
                }
            }
        }

        copy(&mut source, &mut File::create(path)?)
            .map(|_| ())
            .or_else(|error| {
                if flag!(options.flags[CLEANUP_ON_ERROR]) && path.is_file() {
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
    strip_components: usize,
    flags: Flag,
}

impl Default for UnrollOptions {
    fn default() -> Self {
        Self {
            strip_components: 0,
            flags: DEFAULT_UNROLL_FLAGS,
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
        flag! { self.options.flags[CREATE_DEST_PATH] = flag }
        self
    }

    /// Cleanup destination directory before extraction
    ///
    /// Default: `true`
    pub const fn cleanup_dest_dir(mut self, flag: bool) -> Self {
        flag! { self.options.flags[CLEANUP_DEST_DIR] = flag }
        self
    }

    /// Try to fix destination path when it is not a valid
    ///
    /// For example, when destination already exists
    /// and it is not a directory, it will be removed
    ///
    /// Default: `true`
    pub const fn fix_invalid_dest(mut self, flag: bool) -> Self {
        flag! { self.options.flags[FIX_INVALID_DEST] = flag }
        self
    }

    /// Cleanup already extracted data when errors occurs
    ///
    /// Default: `true`
    pub const fn cleanup_on_error(mut self, flag: bool) -> Self {
        flag! { self.options.flags[CLEANUP_ON_ERROR] = flag }
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
        flag! { self.options.flags[STRIP_WHEN_ALONE] = flag }
        self
    }
}

impl<R> Unroll<R> {
    /// Extract contents to specified directory
    ///
    /// # Errors
    /// - Destination directory does not exists when `create_dest_path` is not set
    /// - Destination directory is not empty when `cleanup_dest_dir` is not set
    /// - Destination path is not a directory when `fix_invalid_dest` is not set
    /// - Required number of path components cannot be stripped  when `strip_when_alone` is not set
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

            if flag!(options.flags[CLEANUP_DEST_DIR]) {
                remove_dir_entries(path)?;
            }
        } else if path.is_file() {
            //dest_already_exists = true;

            if flag!(options.flags[FIX_INVALID_DEST]) {
                remove_file(path)?;

                if flag!(options.flags[CREATE_DEST_PATH]) {
                    create_dir_all(path)?;
                }
            }
        } else {
            // not exists
            if flag!(options.flags[CREATE_DEST_PATH]) {
                create_dir_all(path)?;
            }
        }

        unroll_archive_to(source, &options, path).or_else(|error| {
            if flag!(options.flags[CLEANUP_ON_ERROR]) && path.is_dir() {
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

        let strip_components = if flag!(options.flags[STRIP_WHEN_ALONE]) {
            let mut archive = TarArchive::new(Cursor::new(&decoded_data));
            options
                .strip_components
                .min(count_common_components(&mut archive)?)
        } else {
            options.strip_components
        };

        let mut archive = TarArchive::new(Cursor::new(decoded_data));
        let entries = archive.entries()?;

        for entry in entries {
            let mut entry = entry?;
            let type_ = entry.header().entry_type();

            {
                let entry_path = entry.path()?;

                match type_ {
                    TarEntryType::Directory => {
                        let stripped_path = entry_path
                            .iter()
                            .skip(strip_components)
                            .collect::<PathBuf>();
                        if stripped_path.iter().count() < 1 {
                            continue;
                        }
                        let dest_path = destin.join(stripped_path);

                        //create_dir_all(dest_path);
                        entry.unpack(dest_path)?;
                    }
                    TarEntryType::Regular => {
                        let strip_components = strip_components.min(entry_path.iter().count() - 1);
                        let stripped_path = entry_path
                            .iter()
                            .skip(strip_components)
                            .collect::<PathBuf>();
                        let dest_path = destin.join(stripped_path);

                        entry.unpack(dest_path)?;
                    }
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
        let entry = entry?;
        let entry_path = entry.path()?;

        match entry.header().entry_type() {
            TarEntryType::Directory | TarEntryType::Regular => {
                if common_ancestor.is_none() {
                    common_ancestor = Some(entry_path.to_path_buf());
                } else {
                    let common_ancestor = common_ancestor.as_mut().unwrap();

                    *common_ancestor = common_ancestor
                        .iter()
                        .zip(entry_path.iter())
                        .take_while(|(common_component, entry_component)| {
                            common_component == entry_component
                        })
                        .map(|(common_component, _)| common_component)
                        .collect();
                }
            }
            _ => (),
        }
    }

    Ok(common_ancestor.map_or(0, |path| path.iter().count()))
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
        Fetch::from(src_url)
            .unroll()
            .strip_components(1)
            .strip_when_alone(true)
            .to(dst_dir)
            .unwrap();

        //std::fs::remove_dir_all(dst_dir).unwrap();
    }
}
