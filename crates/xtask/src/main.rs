//! `cargo all`: every check of the Monade, in order, stopping at the first failure.
//!
//! Steps: formatting; clippy with warnings as errors; no floating point in
//! `monade-core`; tests in debug; tests in release. The same command runs on
//! Windows, on Linux and in CI.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const CARGO_STEPS: &[(&str, &[&str])] = &[
    ("format", &["fmt", "--all", "--", "--check"]),
    (
        "clippy",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    ),
    ("tests (debug)", &["test", "--workspace", "--all-features"]),
    (
        "tests (release)",
        &["test", "--workspace", "--all-features", "--release"],
    ),
];

fn main() -> ExitCode {
    match run() {
        Ok(()) => {
            println!("all checks passed");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("FAILED: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    for (i, (name, args)) in CARGO_STEPS.iter().enumerate() {
        if i == 2 {
            println!("== no floating point in monade-core");
            no_float(&root.join("crates/monade-core/src"))?;
        }
        println!("== {name}");
        let ok = Command::new(&cargo)
            .args(*args)
            .current_dir(&root)
            .status()
            .map_err(|e| format!("{name}: {e}"))?
            .success();
        if !ok {
            return Err((*name).to_string());
        }
    }
    Ok(())
}

/// Fails if any `.rs` file under `dir` names a floating-point type, even in a comment.
fn no_float(dir: &Path) -> Result<(), String> {
    let mut found = Vec::new();
    for file in rust_files(dir).map_err(|e| e.to_string())? {
        let text = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
        for (n, line) in text.lines().enumerate() {
            if has_float(line) {
                found.push(format!("{}:{}: {}", file.display(), n + 1, line.trim()));
            }
        }
    }
    if found.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "floating point in monade-core:\n{}",
            found.join("\n")
        ))
    }
}

fn rust_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            out.extend(rust_files(&path)?);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// `true` if the line contains the whole token `f32` or `f64`.
fn has_float(line: &str) -> bool {
    let b = line.as_bytes();
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    (0..b.len().saturating_sub(2)).any(|i| {
        b[i] == b'f'
            && matches!(&b[i + 1..i + 3], b"32" | b"64")
            && (i == 0 || !word(b[i - 1]))
            && b.get(i + 3).is_none_or(|c| !word(*c))
    })
}

#[cfg(test)]
mod tests {
    use super::has_float;

    #[test]
    fn finds_float_tokens_only() {
        let float = ["f", "64"].concat();
        assert!(has_float(&format!("let x: {float} = 1;")));
        assert!(has_float(&format!("1.0_{float}").replace("_", " ")));
        assert!(!has_float("let u: u64 = 0xf64;"));
        assert!(!has_float("buf32 offset64"));
    }
}
