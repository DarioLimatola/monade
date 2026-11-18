//! The specifications: a readable, versioned log, mutable with history.
//!
//! A change appends a new version of a key; old values stay in the history.
//! One entry is locked: the seed and version of the hash of the generated
//! space. Changing it would mean a new mind, so no function can.
//! Reading a log ([`Specs::from_text`]) is also its check: it must start
//! with the genesis entries and replay through [`Specs::set`].

use crate::record::{self, RecordError};
use core::fmt;

/// Key of the locked seed of the generated space.
pub const SEED_KEY: &str = "space.seed";

/// Key of the locked version of the hash of the generated space.
pub const HASH_VERSION_KEY: &str = "space.hash_version";

/// The seed of the generated space: ASCII `MONADE` followed by `0x0001`.
///
/// Fixed for this mind. A different seed gives different basic vectors and
/// makes everything learned meaningless: it is a new mind.
///
/// # Example
///
/// ```
/// use monade_core::specs::SEED;
/// assert_eq!(&SEED.to_be_bytes()[..6], b"MONADE");
/// ```
pub const SEED: u64 = 0x4D4F_4E41_4445_0001;

/// The version of the hash function of the generated space.
pub const HASH_VERSION: u64 = 1;

/// The date of the genesis entries.
pub const GENESIS_DATE: &str = "2026-11-18";

/// One version of one specification.
///
/// # Example
///
/// ```
/// use monade_core::specs::{Specs, SEED_KEY};
/// let s = Specs::genesis();
/// assert_eq!(s.get(SEED_KEY).unwrap().version, 1);
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Spec {
    /// Key: lowercase letters, digits, `_`, `.`.
    pub key: String,
    /// Value, as readable text.
    pub value: String,
    /// 1 for the first value of the key, then +1 per change.
    pub version: u64,
    /// Date, `YYYY-MM-DD`.
    pub date: String,
    /// Why it was set.
    pub motive: String,
}

/// Errors of the specifications.
///
/// # Example
///
/// ```
/// use monade_core::specs::{Specs, SpecError, SEED_KEY};
/// let mut s = Specs::genesis();
/// let e = s.set(SEED_KEY, "0x1", "2026-11-18", "try");
/// assert_eq!(e, Err(SpecError::Locked));
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SpecError {
    /// The text cannot be read.
    Record(RecordError),
    /// The key is locked (the seed): a new seed is a new mind.
    Locked,
    /// The key is not lowercase letters, digits, `_`, `.`.
    BadKey,
    /// The motive is empty.
    NoMotive,
    /// The date is not `YYYY-MM-DD`.
    BadDate,
    /// The log does not start with the genesis entries of this mind.
    NotThisMind,
    /// A stored version disagrees with the history.
    Inconsistent(usize),
}

impl fmt::Display for SpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpecError::Record(e) => write!(f, "{e}"),
            SpecError::Locked => write!(f, "locked: a new seed is a new mind"),
            SpecError::BadKey => write!(f, "a key is lowercase letters, digits, `_` or `.`"),
            SpecError::NoMotive => write!(f, "motive is empty"),
            SpecError::BadDate => write!(f, "date is not YYYY-MM-DD"),
            SpecError::NotThisMind => write!(f, "the locked seed differs: another mind"),
            SpecError::Inconsistent(i) => write!(f, "entry {i} is inconsistent with the history"),
        }
    }
}

impl std::error::Error for SpecError {}

impl From<RecordError> for SpecError {
    fn from(e: RecordError) -> Self {
        SpecError::Record(e)
    }
}

const TITLE: &str = "Monade · specifications · versioned · written only by monade-run";
const KEYS: &[&str; 5] = &["key", "value", "version", "date", "motive"];

/// The specifications: the full history of every key.
///
/// # Example
///
/// ```
/// use monade_core::specs::Specs;
/// let mut s = Specs::genesis();
/// s.set("vigilance", "0.625", "2026-11-18", "council default").unwrap();
/// assert_eq!(s.get("vigilance").unwrap().value, "0.625");
/// assert_eq!(Specs::from_text(&s.to_text()), Ok(s));
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Specs {
    history: Vec<Spec>,
}

impl Specs {
    /// The specifications of a new mind: only the locked seed and hash version.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::specs::{Specs, HASH_VERSION_KEY};
    /// assert_eq!(Specs::genesis().get(HASH_VERSION_KEY).unwrap().value, "1");
    /// ```
    pub fn genesis() -> Specs {
        let spec = |key: &str, value: String, motive: &str| Spec {
            key: key.into(),
            value,
            version: 1,
            date: GENESIS_DATE.into(),
            motive: motive.into(),
        };
        Specs {
            history: vec![
                spec(
                    SEED_KEY,
                    format!("{SEED:#018x}"),
                    "locked: the seed of the generated space; a new seed is a new mind",
                ),
                spec(
                    HASH_VERSION_KEY,
                    HASH_VERSION.to_string(),
                    "locked: the version of the hash of the generated space",
                ),
            ],
        }
    }

    /// The full history, oldest first.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::specs::Specs;
    /// assert_eq!(Specs::genesis().history().len(), 2);
    /// ```
    pub fn history(&self) -> &[Spec] {
        &self.history
    }

    /// The current version of a key.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::specs::Specs;
    /// assert!(Specs::genesis().get("missing").is_none());
    /// ```
    pub fn get(&self, key: &str) -> Option<&Spec> {
        self.history.iter().rev().find(|s| s.key == key)
    }

    /// `true` for a locked key.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::specs::{Specs, SEED_KEY};
    /// assert!(Specs::is_locked(SEED_KEY));
    /// assert!(!Specs::is_locked("vigilance"));
    /// ```
    pub fn is_locked(key: &str) -> bool {
        key == SEED_KEY || key == HASH_VERSION_KEY
    }

    /// Sets a key to a new value; the old value stays in the history.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::specs::Specs;
    /// let mut s = Specs::genesis();
    /// s.set("vigilance", "0.6", "2026-11-18", "first").unwrap();
    /// s.set("vigilance", "0.625", "2026-11-19", "second").unwrap();
    /// assert_eq!(s.get("vigilance").unwrap().version, 2);
    /// assert_eq!(s.history().len(), 4);
    /// ```
    pub fn set(
        &mut self,
        key: &str,
        value: &str,
        date: &str,
        motive: &str,
    ) -> Result<&Spec, SpecError> {
        if Specs::is_locked(key) {
            return Err(SpecError::Locked);
        }
        if !record::is_key(key) {
            return Err(SpecError::BadKey);
        }
        if motive.is_empty() {
            return Err(SpecError::NoMotive);
        }
        if !record::is_date(date) {
            return Err(SpecError::BadDate);
        }
        let version = self.get(key).map_or(1, |s| s.version + 1);
        self.history.push(Spec {
            key: key.into(),
            value: value.into(),
            version,
            date: date.into(),
            motive: motive.into(),
        });
        Ok(&self.history[self.history.len() - 1])
    }

    /// The canonical text of the history.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::specs::Specs;
    /// assert!(Specs::genesis().to_text().contains("key: space.seed"));
    /// ```
    pub fn to_text(&self) -> String {
        let records: Vec<_> = self
            .history
            .iter()
            .map(|s| {
                record::entry(
                    KEYS,
                    [
                        s.key.clone(),
                        s.value.clone(),
                        s.version.to_string(),
                        s.date.clone(),
                        s.motive.clone(),
                    ],
                )
            })
            .collect();
        record::write_log(TITLE, "specs", &records)
    }

    /// Reads and checks a history: genesis first, then every change replayed.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::specs::{Specs, SpecError};
    /// let other = Specs::genesis().to_text().replace("0x4d4f", "0x0000");
    /// assert_eq!(Specs::from_text(&other), Err(SpecError::NotThisMind));
    /// ```
    pub fn from_text(text: &str) -> Result<Specs, SpecError> {
        let records = record::read_log(text, "specs")?;
        let mut stored = Vec::with_capacity(records.len());
        for (i, rec) in records.iter().enumerate() {
            let v = record::values(rec, i + 1, KEYS)?;
            stored.push(Spec {
                key: v[0].into(),
                value: v[1].into(),
                version: record::number(v[2], i + 1, "version")?,
                date: v[3].into(),
                motive: v[4].into(),
            });
        }
        let mut specs = Specs::genesis();
        let n = specs.history.len();
        if stored.get(..n) != Some(&specs.history[..]) {
            return Err(SpecError::NotThisMind);
        }
        for (i, s) in stored.iter().enumerate().skip(n) {
            let got = specs.set(&s.key, &s.value, &s.date, &s.motive)?;
            if got != s {
                return Err(SpecError::Inconsistent(i + 1));
            }
        }
        Ok(specs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn specs() -> impl Strategy<Value = Specs> {
        proptest::collection::vec(("[a-c]{1,2}", ".{0,20}", ".{1,20}"), 0..10).prop_map(|sets| {
            let mut s = Specs::genesis();
            for (k, v, m) in sets {
                s.set(&k, &v, "2026-11-18", &m).unwrap();
            }
            s
        })
    }

    #[test]
    fn the_seed_is_locked() {
        let mut s = Specs::genesis();
        assert_eq!(
            s.set(SEED_KEY, "1", GENESIS_DATE, "m"),
            Err(SpecError::Locked)
        );
        assert_eq!(
            s.set(HASH_VERSION_KEY, "2", GENESIS_DATE, "m"),
            Err(SpecError::Locked)
        );
        assert_eq!(
            s.get(SEED_KEY).unwrap().value,
            "0x4d4f4e4144450001",
            "the seed of this mind never changes"
        );
    }

    proptest! {
        #[test]
        fn read_after_write_is_identity(s in specs()) {
            prop_assert_eq!(Specs::from_text(&s.to_text()), Ok(s.clone()));
        }

        #[test]
        fn write_after_read_is_identity(s in specs()) {
            let text = s.to_text();
            prop_assert_eq!(Specs::from_text(&text).unwrap().to_text(), text);
        }

        #[test]
        fn history_rows_never_change(s in specs(), v in ".{0,10}") {
            let before = s.to_text();
            let mut after = s.clone();
            after.set("x", &v, "2026-11-19", "m").unwrap();
            prop_assert!(after.to_text().starts_with(&before));
        }

        #[test]
        fn reader_never_panics(t in any::<String>()) {
            let _ = Specs::from_text(&t);
        }
    }
}
