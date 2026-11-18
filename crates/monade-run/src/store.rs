//! Reading and writing the two foundation files, with their guards.
//!
//! - A missing file means the start: empty point 0, genesis specifications.
//! - A file must be in canonical form: a hand edit is refused.
//! - A new text must begin with the old one: rows are only ever added.

use monade_core::origin::{Origin, OriginError};
use monade_core::specs::{SpecError, Specs};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// File name of point 0 inside the foundations folder.
pub const ORIGIN_FILE: &str = "origin.txt";
/// File name of the specifications inside the foundations folder.
pub const SPECS_FILE: &str = "specs.txt";

/// Everything that can go wrong with the files.
#[derive(Debug)]
pub enum StoreError {
    /// The file cannot be read or written.
    Io(PathBuf, io::Error),
    /// Point 0 cannot be read or the change is refused.
    Origin(OriginError),
    /// The specifications cannot be read or the change is refused.
    Specs(SpecError),
    /// The file is not in canonical form (edited by hand?).
    NotCanonical(PathBuf),
    /// The new text does not begin with the old one.
    NotAppendOnly(PathBuf),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Io(p, e) => write!(f, "{}: {e}", p.display()),
            StoreError::Origin(e) => write!(f, "point 0: {e}"),
            StoreError::Specs(e) => write!(f, "specifications: {e}"),
            StoreError::NotCanonical(p) => {
                write!(
                    f,
                    "{}: not in canonical form (edited by hand?)",
                    p.display()
                )
            }
            StoreError::NotAppendOnly(p) => {
                write!(f, "{}: refused, existing rows would change", p.display())
            }
        }
    }
}

impl From<OriginError> for StoreError {
    fn from(e: OriginError) -> Self {
        StoreError::Origin(e)
    }
}

impl From<SpecError> for StoreError {
    fn from(e: SpecError) -> Self {
        StoreError::Specs(e)
    }
}

/// The text of a file, `None` if it does not exist; CRLF read as LF.
fn read_text(path: &Path) -> Result<Option<String>, StoreError> {
    match fs::read_to_string(path) {
        Ok(t) => Ok(Some(t.replace("\r\n", "\n"))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(StoreError::Io(path.to_path_buf(), e)),
    }
}

/// Reads a log with `parse`, or `start` if the file is missing; checks canonical form.
fn load<T, E>(
    path: &Path,
    start: T,
    parse: impl Fn(&str) -> Result<T, E>,
    print: impl Fn(&T) -> String,
) -> Result<T, StoreError>
where
    StoreError: From<E>,
{
    let Some(text) = read_text(path)? else {
        return Ok(start);
    };
    let value = parse(&text)?;
    if print(&value) != text {
        return Err(StoreError::NotCanonical(path.to_path_buf()));
    }
    Ok(value)
}

/// Writes `new`, only if it begins with the current text of the file.
fn save(path: &Path, new: &str) -> Result<(), StoreError> {
    let old = read_text(path)?;
    if let Some(old) = old {
        if !new.starts_with(&old) {
            return Err(StoreError::NotAppendOnly(path.to_path_buf()));
        }
    }
    let io = |e| StoreError::Io(path.to_path_buf(), e);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(io)?;
    }
    let tmp = path.with_extension("txt.tmp");
    fs::write(&tmp, new).map_err(io)?;
    fs::rename(&tmp, path).map_err(io)
}

/// Reads point 0 from `dir`.
pub fn load_origin(dir: &Path) -> Result<Origin, StoreError> {
    load(
        &dir.join(ORIGIN_FILE),
        Origin::new(),
        Origin::from_text,
        Origin::to_text,
    )
}

/// Reads and checks the specifications from `dir`.
pub fn load_specs(dir: &Path) -> Result<Specs, StoreError> {
    load(
        &dir.join(SPECS_FILE),
        Specs::genesis(),
        Specs::from_text,
        Specs::to_text,
    )
}

/// Writes point 0 to `dir`.
pub fn save_origin(dir: &Path, origin: &Origin) -> Result<(), StoreError> {
    save(&dir.join(ORIGIN_FILE), &origin.to_text())
}

/// Writes the specifications to `dir`.
pub fn save_specs(dir: &Path, specs: &Specs) -> Result<(), StoreError> {
    save(&dir.join(SPECS_FILE), &specs.to_text())
}
