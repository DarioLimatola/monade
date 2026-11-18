//! The run folders: every `check` leaves a readable trace in `runs/`.
//!
//! One folder per run, `runs/YYYYMMDD-HHMMSS/`, with two files:
//! - `run.json`: when, where (machine), which version, how many checks passed;
//! - `results.jsonl`: one line per check (JSON Lines), in order.
//!
//! Files are only ever added, never changed.

use crate::check::Outcome;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Default folder of the runs.
pub const RUNS_DIR: &str = "runs";
const META_FILE: &str = "run.json";
const RESULTS_FILE: &str = "results.jsonl";

/// What a run says about itself.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Meta {
    /// Name of the run folder.
    pub name: String,
    /// Version of the Monade.
    pub version: String,
    /// Start, UTC, `YYYY-MM-DDTHH:MM:SSZ`.
    pub started: String,
    /// Operating system.
    pub os: String,
    /// Processor architecture.
    pub arch: String,
    /// Processor features that matter to the Monade.
    pub cpu: Vec<String>,
    /// Threads available.
    pub threads: u64,
    /// Checks passed.
    pub passed: u64,
    /// Checks failed.
    pub failed: u64,
    /// Total time, microseconds.
    pub micros: u64,
}

/// A run read back from disk.
#[derive(Clone, Debug)]
pub struct Run {
    /// What the run says about itself.
    pub meta: Meta,
    /// Its outcomes, in order.
    pub outcomes: Vec<Outcome>,
}

/// Errors of the run folders.
#[derive(Debug)]
pub enum RunError {
    /// A file cannot be read or written.
    Io(PathBuf, io::Error),
    /// A file is not valid JSON of the expected shape.
    Json(PathBuf, serde_json::Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::Io(p, e) => write!(f, "{}: {e}", p.display()),
            RunError::Json(p, e) => write!(f, "{}: {e}", p.display()),
        }
    }
}

/// The machine this runs on.
pub fn machine() -> (String, String, Vec<String>, u64) {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get() as u64);
    (
        std::env::consts::OS.to_string(),
        std::env::consts::ARCH.to_string(),
        cpu_features(),
        threads,
    )
}

#[cfg(target_arch = "x86_64")]
fn cpu_features() -> Vec<String> {
    let mut f = Vec::new();
    if std::is_x86_feature_detected!("popcnt") {
        f.push("popcnt".to_string());
    }
    if std::is_x86_feature_detected!("avx2") {
        f.push("avx2".to_string());
    }
    if std::is_x86_feature_detected!("avx512f") {
        f.push("avx512f".to_string());
    }
    f
}

#[cfg(not(target_arch = "x86_64"))]
fn cpu_features() -> Vec<String> {
    Vec::new()
}

/// Writes a new run folder under `root`; returns its path.
pub fn save(root: &Path, meta: &mut Meta, outcomes: &[Outcome]) -> Result<PathBuf, RunError> {
    let io = |p: &Path| {
        let p = p.to_path_buf();
        move |e| RunError::Io(p.clone(), e)
    };
    fs::create_dir_all(root).map_err(io(root))?;
    let mut dir = root.join(&meta.name);
    let mut n = 1;
    while dir.exists() {
        n += 1;
        dir = root.join(format!("{}-{n}", meta.name));
    }
    meta.name = dir
        .file_name()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    fs::create_dir(&dir).map_err(io(&dir))?;
    let mut lines = String::new();
    for o in outcomes {
        let line = serde_json::to_string(o).map_err(|e| RunError::Json(dir.clone(), e))?;
        lines.push_str(&line);
        lines.push('\n');
    }
    let results = dir.join(RESULTS_FILE);
    fs::write(&results, lines).map_err(io(&results))?;
    let meta_path = dir.join(META_FILE);
    let text =
        serde_json::to_string_pretty(meta).map_err(|e| RunError::Json(meta_path.clone(), e))?;
    fs::write(&meta_path, text + "\n").map_err(io(&meta_path))?;
    Ok(dir)
}

/// Reads every run under `root`, oldest first. A missing folder means no runs.
pub fn load_all(root: &Path) -> Result<Vec<Run>, RunError> {
    let entries = match fs::read_dir(root) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(RunError::Io(root.to_path_buf(), e)),
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.join(META_FILE).is_file())
        .collect();
    dirs.sort();
    dirs.iter().map(|d| load(d)).collect()
}

fn load(dir: &Path) -> Result<Run, RunError> {
    let read = |p: PathBuf| fs::read_to_string(&p).map_err(|e| RunError::Io(p, e));
    let meta_path = dir.join(META_FILE);
    let meta: Meta = serde_json::from_str(&read(meta_path.clone())?)
        .map_err(|e| RunError::Json(meta_path, e))?;
    let results = dir.join(RESULTS_FILE);
    let mut outcomes = Vec::new();
    for line in read(results.clone())?
        .lines()
        .filter(|l| !l.trim().is_empty())
    {
        outcomes.push(serde_json::from_str(line).map_err(|e| RunError::Json(results.clone(), e))?);
    }
    Ok(Run { meta, outcomes })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::Measure;

    fn outcome(id: &str, passed: bool) -> Outcome {
        Outcome {
            id: id.into(),
            area: "word".into(),
            claim: "c".into(),
            result: "r \"quoted\"\nnew line".into(),
            expected: "e".into(),
            passed,
            micros: 7,
            measures: vec![Measure {
                name: "n".into(),
                value: "1".into(),
            }],
        }
    }

    #[test]
    fn save_then_load_round_trip() {
        let root = std::env::temp_dir().join(format!("monade-runs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let (os, arch, cpu, threads) = machine();
        let mut meta = Meta {
            name: "20261118-180000".into(),
            version: "0.2.0".into(),
            started: "2026-11-18T18:00:00Z".into(),
            os,
            arch,
            cpu,
            threads,
            passed: 1,
            failed: 1,
            micros: 14,
        };
        let outs = [outcome("a", true), outcome("b", false)];
        save(&root, &mut meta, &outs).unwrap();
        let mut second = meta.clone();
        second.name = "20261118-180000".into();
        save(&root, &mut second, &outs).unwrap();
        assert_eq!(
            second.name, "20261118-180000-2",
            "a run never overwrites another"
        );
        let runs = load_all(&root).unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].meta, meta);
        assert_eq!(runs[0].outcomes, outs);
        let _ = fs::remove_dir_all(&root);
        assert!(load_all(&root).unwrap().is_empty());
    }
}
