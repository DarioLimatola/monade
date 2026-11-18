//! Point 0 as object: an append-only log of foundations. Empty for now.
//!
//! The mechanism exists; the content is decided last, by the Demiurge.
//! There is no function to delete or modify an entry: a change is a new
//! entry that amends an old one. Identifiers are never reused. The engine
//! receives the log as `&Origin`, read only; only `monade-run` appends.

use crate::record::{self, RecordError};
use core::fmt;

/// Whether a foundation is in force.
///
/// # Example
///
/// ```
/// use monade_core::origin::State;
/// assert_eq!(State::parse("suspended"), Some(State::Suspended));
/// assert_eq!(State::Active.as_str(), "active");
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    /// In force.
    Active,
    /// Kept in the log, not in force.
    Suspended,
}

impl State {
    /// The word written in the log.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::origin::State;
    /// assert_eq!(State::Suspended.as_str(), "suspended");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            State::Active => "active",
            State::Suspended => "suspended",
        }
    }

    /// Reads the word written in the log.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::origin::State;
    /// assert_eq!(State::parse("active"), Some(State::Active));
    /// assert_eq!(State::parse("Active"), None);
    /// ```
    pub fn parse(s: &str) -> Option<State> {
        match s {
            "active" => Some(State::Active),
            "suspended" => Some(State::Suspended),
            _ => None,
        }
    }
}

/// What the Demiurge writes to add a foundation; the log assigns id and version.
///
/// # Example
///
/// ```
/// use monade_core::origin::{Draft, Origin, State};
/// let mut o = Origin::new();
/// let d = Draft {
///     name: "legibility".into(),
///     text: "Every change is logged.".into(),
///     date: "2026-11-18".into(),
///     motive: "example".into(),
///     state: State::Active,
///     amends: None,
/// };
/// assert_eq!(o.append(d), Ok(1));
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Draft {
    /// Short name.
    pub name: String,
    /// Readable text.
    pub text: String,
    /// Date, `YYYY-MM-DD`.
    pub date: String,
    /// Why it is written.
    pub motive: String,
    /// Active or suspended.
    pub state: State,
    /// The id of the foundation this one amends, if any.
    pub amends: Option<u64>,
}

/// One entry of point 0, as stored.
///
/// # Example
///
/// ```
/// use monade_core::origin::{Draft, Origin, State};
/// let mut o = Origin::new();
/// let d = Draft { name: "n".into(), text: "t".into(), date: "2026-11-18".into(),
///                 motive: "m".into(), state: State::Active, amends: None };
/// o.append(d).unwrap();
/// let f = &o.entries()[0];
/// assert_eq!((f.id, f.version), (1, 1));
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Foundation {
    /// Identifier, from 1, never reused.
    pub id: u64,
    /// Short name.
    pub name: String,
    /// Readable text.
    pub text: String,
    /// 1, or the amended foundation's version plus 1.
    pub version: u64,
    /// Date, `YYYY-MM-DD`.
    pub date: String,
    /// Why it was written.
    pub motive: String,
    /// Active or suspended.
    pub state: State,
    /// The id of the foundation this one amends, if any.
    pub amends: Option<u64>,
}

/// Errors of point 0.
///
/// # Example
///
/// ```
/// use monade_core::origin::{Origin, OriginError};
/// assert!(matches!(Origin::from_text("log: specs\nformat: 1\n"), Err(OriginError::Record(_))));
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum OriginError {
    /// The text cannot be read.
    Record(RecordError),
    /// Name, text or motive is empty.
    Empty(&'static str),
    /// The date is not `YYYY-MM-DD`.
    BadDate,
    /// The amended id does not exist.
    UnknownAmends(u64),
    /// The amended foundation was already amended (chains stay linear).
    AlreadyAmended(u64),
    /// A stored entry disagrees with what appending it would give.
    Inconsistent(u64),
}

impl fmt::Display for OriginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OriginError::Record(e) => write!(f, "{e}"),
            OriginError::Empty(field) => write!(f, "{field} is empty"),
            OriginError::BadDate => write!(f, "date is not YYYY-MM-DD"),
            OriginError::UnknownAmends(id) => write!(f, "foundation {id} does not exist"),
            OriginError::AlreadyAmended(id) => write!(f, "foundation {id} was already amended"),
            OriginError::Inconsistent(id) => write!(f, "entry {id} is inconsistent with the log"),
        }
    }
}

impl std::error::Error for OriginError {}

impl From<RecordError> for OriginError {
    fn from(e: RecordError) -> Self {
        OriginError::Record(e)
    }
}

const TITLE: &str = "Monade · point 0 · append-only · written only by monade-run";
const KEYS: &[&str; 8] = &[
    "id", "name", "text", "version", "date", "motive", "state", "amends",
];

/// Point 0: the append-only log of foundations.
///
/// # Example
///
/// ```
/// use monade_core::origin::Origin;
/// let o = Origin::new();
/// assert!(o.entries().is_empty()); // empty for now, by decision
/// assert_eq!(Origin::from_text(&o.to_text()), Ok(o));
/// ```
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Origin {
    entries: Vec<Foundation>,
}

impl Origin {
    /// The empty log.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::origin::Origin;
    /// assert_eq!(Origin::new().entries().len(), 0);
    /// ```
    pub fn new() -> Origin {
        Origin::default()
    }

    /// All entries, oldest first. Read only.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::origin::Origin;
    /// assert!(Origin::new().entries().is_empty());
    /// ```
    pub fn entries(&self) -> &[Foundation] {
        &self.entries
    }

    /// The foundations in force: the last of each chain, if active.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::origin::{Draft, Origin, State};
    /// let mut o = Origin::new();
    /// let d = Draft { name: "n".into(), text: "t".into(), date: "2026-11-18".into(),
    ///                 motive: "m".into(), state: State::Active, amends: None };
    /// o.append(d.clone()).unwrap();
    /// o.append(Draft { state: State::Suspended, amends: Some(1), ..d }).unwrap();
    /// assert_eq!(o.current().count(), 0);
    /// ```
    pub fn current(&self) -> impl Iterator<Item = &Foundation> {
        self.entries.iter().filter(|f| {
            f.state == State::Active && !self.entries.iter().any(|g| g.amends == Some(f.id))
        })
    }

    /// Appends a foundation; returns its new id. Nothing else can change the log.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::origin::{Draft, Origin, OriginError, State};
    /// let mut o = Origin::new();
    /// let d = Draft { name: "n".into(), text: "t".into(), date: "2026-11-18".into(),
    ///                 motive: "m".into(), state: State::Active, amends: Some(7) };
    /// assert_eq!(o.append(d), Err(OriginError::UnknownAmends(7)));
    /// ```
    pub fn append(&mut self, draft: Draft) -> Result<u64, OriginError> {
        for (field, value) in [
            ("name", &draft.name),
            ("text", &draft.text),
            ("motive", &draft.motive),
        ] {
            if value.is_empty() {
                return Err(OriginError::Empty(field));
            }
        }
        if !record::is_date(&draft.date) {
            return Err(OriginError::BadDate);
        }
        let version = match draft.amends {
            None => 1,
            Some(target) => {
                let old = self
                    .entries
                    .iter()
                    .find(|f| f.id == target)
                    .ok_or(OriginError::UnknownAmends(target))?;
                if self.entries.iter().any(|f| f.amends == Some(target)) {
                    return Err(OriginError::AlreadyAmended(target));
                }
                old.version + 1
            }
        };
        let id = self.entries.len() as u64 + 1;
        self.entries.push(Foundation {
            id,
            name: draft.name,
            text: draft.text,
            version,
            date: draft.date,
            motive: draft.motive,
            state: draft.state,
            amends: draft.amends,
        });
        Ok(id)
    }

    /// The canonical text of the log.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::origin::Origin;
    /// assert!(Origin::new().to_text().contains("log: origin"));
    /// ```
    pub fn to_text(&self) -> String {
        let mut records = Vec::new();
        for f in &self.entries {
            let amends = f.amends.map_or(String::new(), |a| a.to_string());
            let values = [
                f.id.to_string(),
                f.name.clone(),
                f.text.clone(),
                f.version.to_string(),
                f.date.clone(),
                f.motive.clone(),
                f.state.as_str().to_string(),
                amends,
            ];
            records.push(record::entry(KEYS, values));
        }
        record::write_log(TITLE, "origin", &records)
    }

    /// Reads a log by replaying every entry through [`Origin::append`].
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::origin::Origin;
    /// assert!(Origin::from_text("log: origin\nformat: 1\n").is_ok());
    /// assert!(Origin::from_text("").is_err());
    /// ```
    pub fn from_text(text: &str) -> Result<Origin, OriginError> {
        let records = record::read_log(text, "origin")?;
        let mut origin = Origin::new();
        for (i, rec) in records.iter().enumerate() {
            let n = i + 1;
            let v = record::values(rec, n, KEYS)?;
            let id = record::number(v[0], n, "id")?;
            let version = record::number(v[3], n, "version")?;
            let state = State::parse(v[6]).ok_or(RecordError::Value {
                record: n,
                key: "state",
            })?;
            let amends = match v[7] {
                "" => None,
                a => Some(record::number(a, n, "amends")?),
            };
            let draft = Draft {
                name: v[1].into(),
                text: v[2].into(),
                date: v[4].into(),
                motive: v[5].into(),
                state,
                amends,
            };
            let got = origin.append(draft)?;
            if got != id || origin.entries[i].version != version {
                return Err(OriginError::Inconsistent(id));
            }
        }
        Ok(origin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn draft(amends: Option<u64>) -> impl Strategy<Value = Draft> {
        (
            ".{1,12}",
            ".{1,40}",
            ".{1,12}",
            any::<bool>(),
            (2000u32..2100, 1u32..13, 1u32..29),
        )
            .prop_map(move |(name, text, motive, active, (y, m, d))| Draft {
                name,
                text,
                date: format!("{y:04}-{m:02}-{d:02}"),
                motive,
                state: if active {
                    State::Active
                } else {
                    State::Suspended
                },
                amends,
            })
    }

    fn log() -> impl Strategy<Value = Origin> {
        proptest::collection::vec((draft(None), any::<bool>()), 0..8).prop_map(|drafts| {
            let mut o = Origin::new();
            for (mut d, amend) in drafts {
                // Amend the newest head when asked and possible.
                let head = o.current().last().map(|f| f.id);
                d.amends = if amend { head } else { None };
                let _ = o.append(d);
            }
            o
        })
    }

    #[test]
    fn empty_for_now() {
        let o = Origin::new();
        assert_eq!(o.to_text(), Origin::new().to_text());
        assert_eq!(Origin::from_text(&o.to_text()), Ok(o));
    }

    proptest! {
        #[test]
        fn read_after_write_is_identity(o in log()) {
            prop_assert_eq!(Origin::from_text(&o.to_text()), Ok(o.clone()));
        }

        #[test]
        fn write_after_read_is_identity(o in log()) {
            let text = o.to_text();
            prop_assert_eq!(Origin::from_text(&text).unwrap().to_text(), text);
        }

        #[test]
        fn existing_rows_never_change(o in log(), d in draft(None)) {
            let before = o.to_text();
            let mut after = o.clone();
            after.append(d).unwrap();
            prop_assert!(after.to_text().starts_with(&before));
            prop_assert_eq!(&after.entries()[..o.entries().len()], o.entries());
        }

        #[test]
        fn ids_are_never_reused(o in log()) {
            for (i, f) in o.entries().iter().enumerate() {
                prop_assert_eq!(f.id, i as u64 + 1);
            }
        }

        #[test]
        fn reader_never_panics(s in any::<String>()) {
            let _ = Origin::from_text(&s);
        }
    }

    #[test]
    fn amendments_are_linear() {
        let mut o = Origin::new();
        let d = Draft {
            name: "n".into(),
            text: "t".into(),
            date: "2026-11-18".into(),
            motive: "m".into(),
            state: State::Active,
            amends: None,
        };
        o.append(d.clone()).unwrap();
        let amend = Draft {
            amends: Some(1),
            ..d.clone()
        };
        assert_eq!(o.append(amend.clone()), Ok(2));
        assert_eq!(o.entries()[1].version, 2);
        assert_eq!(o.append(amend), Err(OriginError::AlreadyAmended(1)));
        assert_eq!(
            o.append(Draft {
                date: "2026-11-1".into(),
                ..d
            }),
            Err(OriginError::BadDate)
        );
    }
}
