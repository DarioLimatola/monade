//! The readable text format shared by point 0 and the specifications.
//!
//! A file is a comment line, then records separated by one empty line.
//! A record is a fixed sequence of `key: value` lines. Values are escaped
//! (`\\`, `\n`, `\r`) so every record line stays one line.
//!
//! ```text
//! # Monade · point 0
//!
//! log: origin
//! format: 1
//!
//! id: 1
//! name: …
//! ```

use core::fmt;

/// One record: its values in the order of its keys.
pub type Record = Vec<(String, String)>;

/// Errors of the record format.
///
/// # Example
///
/// ```
/// use monade_core::record::{read, RecordError};
/// assert!(matches!(read("no colon here"), Err(RecordError::Syntax { line: 1, .. })));
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RecordError {
    /// A line that cannot be read.
    Syntax {
        /// Line number, from 1.
        line: usize,
        /// What is wrong.
        reason: &'static str,
    },
    /// A record whose keys are not the expected ones, in order.
    Shape {
        /// Record number, from 0 (the header).
        record: usize,
        /// The keys expected, in order.
        expected: &'static [&'static str],
    },
    /// The header is not `log: <kind>`, `format: 1`.
    Header {
        /// The kind of log expected.
        expected: &'static str,
    },
    /// A value that cannot be read as its type.
    Value {
        /// Record number, from 0 (the header).
        record: usize,
        /// The key whose value is wrong.
        key: &'static str,
    },
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecordError::Syntax { line, reason } => write!(f, "line {line}: {reason}"),
            RecordError::Shape { record, expected } => {
                write!(f, "record {record}: expected keys `{}`", expected.join(" "))
            }
            RecordError::Header { expected } => {
                write!(f, "not a `{expected}` log (format 1)")
            }
            RecordError::Value { record, key } => {
                write!(f, "record {record}: bad value for `{key}`")
            }
        }
    }
}

impl std::error::Error for RecordError {}

/// Writes a comment line and the records, in the canonical form.
///
/// # Example
///
/// ```
/// use monade_core::record::{read, write};
/// let r = vec![("name".to_string(), "two\nlines".to_string())];
/// let text = write("title", &[r.clone()]);
/// assert_eq!(text, "# title\n\nname: two\\nlines\n");
/// assert_eq!(read(&text), Ok(vec![r]));
/// ```
pub fn write(title: &str, records: &[Record]) -> String {
    let mut out = format!("# {}\n", escape(title));
    for record in records {
        out.push('\n');
        for (key, value) in record {
            out.push_str(key);
            out.push(':');
            if !value.is_empty() {
                out.push(' ');
                out.push_str(&escape(value));
            }
            out.push('\n');
        }
    }
    out
}

/// Reads records; comment lines (`#`) are skipped. Never panics.
///
/// # Example
///
/// ```
/// use monade_core::record::read;
/// let recs = read("# t\n\na: 1\nb:\n\nc: x\n").unwrap();
/// assert_eq!(recs.len(), 2);
/// assert_eq!(recs[0][1], ("b".to_string(), String::new()));
/// ```
pub fn read(text: &str) -> Result<Vec<Record>, RecordError> {
    let mut records = Vec::new();
    let mut current = Record::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if line.starts_with('#') {
            continue;
        }
        if line.is_empty() {
            if !current.is_empty() {
                records.push(core::mem::take(&mut current));
            }
            continue;
        }
        let Some((key, rest)) = line.split_once(':') else {
            return Err(RecordError::Syntax {
                line: n,
                reason: "expected `key: value`",
            });
        };
        if !is_key(key) {
            return Err(RecordError::Syntax {
                line: n,
                reason: "a key is lowercase letters, digits, `_` or `.`",
            });
        }
        let raw = rest.strip_prefix(' ').unwrap_or(rest);
        let value = unescape(raw).ok_or(RecordError::Syntax {
            line: n,
            reason: "bad escape: use \\\\, \\n or \\r",
        })?;
        current.push((key.to_string(), value));
    }
    if !current.is_empty() {
        records.push(current);
    }
    Ok(records)
}

/// Writes a log: the header record (`log: <kind>`, `format: 1`), then the entries.
///
/// # Example
///
/// ```
/// use monade_core::record::{read_log, write_log};
/// let text = write_log("t", "specs", &[]);
/// assert_eq!(text, "# t\n\nlog: specs\nformat: 1\n");
/// assert_eq!(read_log(&text, "specs"), Ok(vec![]));
/// ```
pub fn write_log(title: &str, kind: &str, entries: &[Record]) -> String {
    let header = vec![
        ("log".to_string(), kind.to_string()),
        ("format".to_string(), "1".to_string()),
    ];
    let mut records = vec![header];
    records.extend_from_slice(entries);
    write(title, &records)
}

/// Reads a log of the given kind; returns its entries, header removed.
///
/// # Example
///
/// ```
/// use monade_core::record::{read_log, RecordError};
/// assert!(read_log("log: origin\nformat: 1\n", "origin").is_ok());
/// assert_eq!(read_log("", "origin"), Err(RecordError::Header { expected: "origin" }));
/// ```
pub fn read_log(text: &str, kind: &'static str) -> Result<Vec<Record>, RecordError> {
    let mut records = read(text)?;
    let good = records.first().is_some_and(|h| {
        h.len() == 2 && h[0] == ("log".into(), kind.into()) && h[1] == ("format".into(), "1".into())
    });
    if !good {
        return Err(RecordError::Header { expected: kind });
    }
    records.remove(0);
    Ok(records)
}

/// Builds a record from keys and values, in order.
///
/// # Example
///
/// ```
/// use monade_core::record::entry;
/// let r = entry(&["a", "b"], ["1".to_string(), "2".to_string()]);
/// assert_eq!(r[1], ("b".to_string(), "2".to_string()));
/// ```
pub fn entry<const N: usize>(keys: &[&str; N], values: [String; N]) -> Record {
    keys.iter().map(|k| k.to_string()).zip(values).collect()
}

/// The values of a record, if its keys are exactly `keys`, in order.
///
/// # Example
///
/// ```
/// use monade_core::record::{read, values};
/// let recs = read("a: 1\nb: 2\n").unwrap();
/// assert_eq!(values(&recs[0], 0, &["a", "b"]).unwrap(), vec!["1", "2"]);
/// assert!(values(&recs[0], 0, &["b", "a"]).is_err());
/// ```
pub fn values<'a>(
    record: &'a Record,
    index: usize,
    keys: &'static [&'static str],
) -> Result<Vec<&'a str>, RecordError> {
    let same = record.len() == keys.len() && record.iter().zip(keys).all(|((k, _), e)| k == e);
    if !same {
        return Err(RecordError::Shape {
            record: index,
            expected: keys,
        });
    }
    Ok(record.iter().map(|(_, v)| v.as_str()).collect())
}

/// Reads a decimal `u64` value, or `Value` error for `key`.
///
/// # Example
///
/// ```
/// use monade_core::record::number;
/// assert_eq!(number("42", 1, "id"), Ok(42));
/// assert!(number("+42", 1, "id").is_err());
/// ```
pub fn number(value: &str, record: usize, key: &'static str) -> Result<u64, RecordError> {
    let digits = !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit());
    let canonical = value == "0" || !value.starts_with('0');
    match value.parse::<u64>() {
        Ok(n) if digits && canonical => Ok(n),
        _ => Err(RecordError::Value { record, key }),
    }
}

/// `true` for a date written `YYYY-MM-DD` with a plausible month and day.
///
/// # Example
///
/// ```
/// use monade_core::record::is_date;
/// assert!(is_date("2026-11-18"));
/// assert!(!is_date("2026-13-01"));
/// assert!(!is_date("18/11/2026"));
/// ```
pub fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    let shape = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if !shape {
        return false;
    }
    let two = |i: usize| u32::from(b[i] - b'0') * 10 + u32::from(b[i + 1] - b'0');
    (1..=12).contains(&two(5)) && (1..=31).contains(&two(8))
}

/// `true` for a key: non-empty, lowercase ASCII letters, digits, `_` or `.`.
///
/// # Example
///
/// ```
/// use monade_core::record::is_key;
/// assert!(is_key("space.seed"));
/// assert!(!is_key("Space Seed"));
/// ```
pub fn is_key(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'.')
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

fn unescape(s: &str) -> Option<String> {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next()? {
            '\\' => out.push('\\'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            _ => return None,
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn record() -> impl Strategy<Value = Record> {
        proptest::collection::vec(("[a-z][a-z0-9_.]{0,8}", any::<String>()), 1..5)
    }

    proptest! {
        #[test]
        fn read_after_write_is_identity(recs in proptest::collection::vec(record(), 0..5)) {
            prop_assert_eq!(read(&write("t", &recs)), Ok(recs));
        }

        #[test]
        fn escape_round_trips(s in any::<String>()) {
            prop_assert_eq!(unescape(&escape(&s)), Some(s));
        }

        #[test]
        fn read_never_panics(s in any::<String>()) {
            let _ = read(&s);
        }
    }

    #[test]
    fn crlf_is_read_like_lf() {
        assert_eq!(read("a: 1\r\nb: 2\r\n"), read("a: 1\nb: 2\n"));
    }
}
