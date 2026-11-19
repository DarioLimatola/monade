//! `monade-run`: the command line of the Monade.
//!
//! The only writer of point 0 and of the specifications; it also verifies the
//! Monade (`check`) and shows the results (`report`). Errors are values: every
//! command returns its text or an error, and `main` prints it.

#![forbid(unsafe_code)]

mod check;
mod report;
mod runs;
mod specchio;
mod stars;
mod store;

use monade_core::origin::{Draft, State};
use monade_core::specs::Specs;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};
use store::StoreError;

const USAGE: &str = "\
monade-run — the only writer of point 0 and of the specifications

USAGE
  monade-run show
  monade-run check  [--runs R]
  monade-run report [--runs R] [--out F]
  monade-run stars  --catalog C [--out F]
  monade-run specchio [--port P] [--runs R] [--open no]
  monade-run tools ARGS...
  monade-run append-foundation --name N --text T --motive M [--state active|suspended] [--amends ID]
  monade-run set-spec KEY VALUE --motive M

  show     point 0 and the specifications
  check    verifies the Monade, says what it verified, saves the run in R
  report   writes one HTML page (F) from every run in R; open it in a browser
  stars    puts the stars of a catalog (HYG CSV) into four 2 MB Monadi and writes
           a 3D map (F, default stars.html)
  specchio the interface: serves it on this computer only (127.0.0.1, port P,
           default 8088) and opens the browser; Ctrl+C stops it
  tools    runs the Python tools with uv: `tools doctor`, `tools card new ...`
           (the arguments go to `monade-tools` unchanged; needs uv)

OPTIONS
  --dir D    folder of origin.txt and specs.txt (default: foundations)
  --runs R   folder of the runs (default: runs)
  --out F    the HTML page (default: report.html)
";

/// Errors of the command line.
#[derive(Debug)]
enum CliError {
    /// Wrong arguments; the text says which.
    Usage(String),
    /// The files refused the change.
    Store(StoreError),
    /// A run folder cannot be read or written.
    Runs(runs::RunError),
    /// Some checks failed; the text is the full output.
    ChecksFailed(String),
    /// An external tool (uv, the Python tools) could not run or failed.
    Tool(String),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Usage(m) => write!(f, "{m}\n\n{USAGE}"),
            CliError::Store(e) => write!(f, "{e}"),
            CliError::Runs(e) => write!(f, "runs: {e}"),
            CliError::ChecksFailed(out) => write!(f, "some checks failed\n\n{out}"),
            CliError::Tool(m) => write!(f, "{m}"),
        }
    }
}

impl From<StoreError> for CliError {
    fn from(e: StoreError) -> Self {
        CliError::Store(e)
    }
}

impl From<runs::RunError> for CliError {
    fn from(e: runs::RunError) -> Self {
        CliError::Runs(e)
    }
}

impl From<monade_core::origin::OriginError> for CliError {
    fn from(e: monade_core::origin::OriginError) -> Self {
        CliError::Store(e.into())
    }
}

impl From<monade_core::specs::SpecError> for CliError {
    fn from(e: monade_core::specs::SpecError) -> Self {
        CliError::Store(e.into())
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args, &now()) {
        Ok(out) => {
            print!("{out}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Positional arguments and `--flag value` pairs.
struct Args {
    positional: Vec<String>,
    flags: Vec<(String, String)>,
}

impl Args {
    fn parse(args: &[String]) -> Result<Args, CliError> {
        let mut out = Args {
            positional: Vec::new(),
            flags: Vec::new(),
        };
        let mut it = args.iter();
        while let Some(a) = it.next() {
            match a.strip_prefix("--") {
                Some(name) => {
                    let value = it
                        .next()
                        .ok_or_else(|| CliError::Usage(format!("--{name} needs a value")))?;
                    out.flags.push((name.to_string(), value.clone()));
                }
                None => out.positional.push(a.clone()),
            }
        }
        Ok(out)
    }

    /// The value of a flag, if given.
    fn flag(&self, name: &str) -> Option<&str> {
        self.flags
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// The value of a flag that must be given.
    fn required(&self, name: &str) -> Result<&str, CliError> {
        self.flag(name)
            .ok_or_else(|| CliError::Usage(format!("missing --{name}")))
    }

    /// Error unless every flag is allowed and given once, with `positional` arguments.
    fn only(&self, allowed: &[&str], positional: usize) -> Result<(), CliError> {
        for (i, (n, _)) in self.flags.iter().enumerate() {
            if !allowed.contains(&n.as_str()) && n != "dir" {
                return Err(CliError::Usage(format!("unknown option --{n}")));
            }
            if self.flags[..i].iter().any(|(m, _)| m == n) {
                return Err(CliError::Usage(format!("--{n} given twice")));
            }
        }
        if self.positional.len() != positional {
            return Err(CliError::Usage(format!(
                "expected {positional} argument(s) after the command"
            )));
        }
        Ok(())
    }

    fn dir(&self) -> PathBuf {
        PathBuf::from(self.flag("dir").unwrap_or("foundations"))
    }

    fn runs(&self) -> PathBuf {
        PathBuf::from(self.flag("runs").unwrap_or(runs::RUNS_DIR))
    }
}

/// Runs one command; `now` is the UTC time, `YYYY-MM-DDTHH:MM:SSZ`.
fn run(args: &[String], now: &str) -> Result<String, CliError> {
    let today = &now[..10.min(now.len())];
    let Some((command, rest)) = args.split_first() else {
        return Ok(USAGE.to_string());
    };
    if command == "tools" {
        return tools_command(rest);
    }
    let a = Args::parse(rest)?;
    match command.as_str() {
        "show" => {
            a.only(&[], 0)?;
            show(&a)
        }
        "append-foundation" => {
            a.only(&["name", "text", "motive", "state", "amends"], 0)?;
            append_foundation(&a, today)
        }
        "set-spec" => {
            a.only(&["motive"], 2)?;
            set_spec(&a, today)
        }
        "check" => {
            a.only(&["runs"], 0)?;
            check_command(&a, now)
        }
        "report" => {
            a.only(&["runs", "out"], 0)?;
            report_command(&a, now)
        }
        "stars" => {
            a.only(&["catalog", "out"], 0)?;
            stars_command(&a)
        }
        "specchio" => {
            a.only(&["port", "runs", "open"], 0)?;
            specchio_command(&a)
        }
        "help" | "--help" | "-h" => Ok(USAGE.to_string()),
        other => Err(CliError::Usage(format!("unknown command `{other}`"))),
    }
}

fn show(a: &Args) -> Result<String, CliError> {
    show_foundations(&a.dir())
}

/// Point 0 and the specifications in `dir`, as readable text.
fn show_foundations(dir: &std::path::Path) -> Result<String, CliError> {
    let origin = store::load_origin(dir)?;
    let specs = store::load_specs(dir)?;
    let mut out = format!("Monade · foundations in {}\n\n", dir.display());
    let n = origin.entries().len();
    if n == 0 {
        out.push_str("POINT 0  empty (decided last)\n");
    } else {
        let _ = writeln!(out, "POINT 0  {n} entries");
    }
    for f in origin.entries() {
        let amends = f.amends.map_or(String::new(), |id| format!(" amends {id}"));
        let _ = writeln!(
            out,
            "  [{}] {} · v{} · {} · {}{}\n      {}\n      motive: {}",
            f.id,
            f.name,
            f.version,
            f.state.as_str(),
            f.date,
            amends,
            f.text,
            f.motive
        );
    }
    let _ = writeln!(
        out,
        "\nSPECIFICATIONS  checked · {} entries in history",
        specs.history().len()
    );
    let mut seen: Vec<&str> = Vec::new();
    for s in specs.history() {
        if seen.contains(&s.key.as_str()) {
            continue;
        }
        seen.push(&s.key);
        if let Some(cur) = specs.get(&s.key) {
            let lock = if Specs::is_locked(&cur.key) {
                " · locked"
            } else {
                ""
            };
            let _ = writeln!(
                out,
                "  {} = {}  (v{} · {}{})",
                cur.key, cur.value, cur.version, cur.date, lock
            );
        }
    }
    Ok(out)
}

fn append_foundation(a: &Args, today: &str) -> Result<String, CliError> {
    let dir = a.dir();
    let mut origin = store::load_origin(&dir)?;
    let state = match a.flag("state") {
        None => State::Active,
        Some(s) => State::parse(s)
            .ok_or_else(|| CliError::Usage("--state is active or suspended".into()))?,
    };
    let amends = match a.flag("amends") {
        None => None,
        Some(s) => Some(
            s.parse()
                .map_err(|_| CliError::Usage("--amends is a foundation id".into()))?,
        ),
    };
    let id = origin.append(Draft {
        name: a.required("name")?.into(),
        text: a.required("text")?.into(),
        date: today.into(),
        motive: a.required("motive")?.into(),
        state,
        amends,
    })?;
    store::save_origin(&dir, &origin)?;
    Ok(format!("point 0: foundation {id} appended\n"))
}

fn set_spec(a: &Args, today: &str) -> Result<String, CliError> {
    let dir = a.dir();
    let mut specs = store::load_specs(&dir)?;
    let (key, value) = (&a.positional[0], &a.positional[1]);
    let version = specs.set(key, value, today, a.required("motive")?)?.version;
    store::save_specs(&dir, &specs)?;
    Ok(format!("specifications: {key} = {value} (v{version})\n"))
}

fn check_command(a: &Args, now: &str) -> Result<String, CliError> {
    let outcomes = check::all(&a.dir());
    let (os, arch, cpu, threads) = runs::machine();
    let passed = outcomes.iter().filter(|o| o.passed).count() as u64;
    let failed = outcomes.len() as u64 - passed;
    let micros = outcomes.iter().map(|o| o.micros).sum();
    let name: String = now
        .chars()
        .filter(char::is_ascii_digit)
        .enumerate()
        .flat_map(|(i, c)| if i == 8 { vec!['-', c] } else { vec![c] })
        .collect();
    let mut meta = runs::Meta {
        name,
        version: env!("CARGO_PKG_VERSION").into(),
        started: now.into(),
        os,
        arch,
        cpu,
        threads,
        passed,
        failed,
        micros,
    };
    let dir = runs::save(&a.runs(), &mut meta, &outcomes)?;
    let mut out = format!(
        "MONADE CHECK  v{}  {}  {} {}  {}  {} threads\n\n",
        meta.version,
        now.replace('T', " ").replace('Z', " UTC"),
        meta.os,
        meta.arch,
        meta.cpu.join(" "),
        meta.threads
    );
    for o in &outcomes {
        let _ = writeln!(
            out,
            "{}  {:<24} {}",
            if o.passed { "ok  " } else { "FAIL" },
            o.id,
            o.result
        );
        if !o.passed {
            let _ = writeln!(out, "      {:<24} expected: {}", "", o.expected);
        }
    }
    let _ = writeln!(
        out,
        "\n{passed} of {} passed in {} ms · saved in {}\nsee the report: cargo run -p monade-run -- report",
        outcomes.len(),
        micros / 1000,
        dir.display()
    );
    if failed == 0 {
        Ok(out)
    } else {
        Err(CliError::ChecksFailed(out))
    }
}

fn report_command(a: &Args, now: &str) -> Result<String, CliError> {
    let all = runs::load_all(&a.runs())?;
    let page = report::html(&all, &now.replace('T', " ").replace('Z', " UTC"));
    let out = PathBuf::from(a.flag("out").unwrap_or("report.html"));
    std::fs::write(&out, page).map_err(|e| CliError::Store(StoreError::Io(out.clone(), e)))?;
    Ok(format!(
        "report of {} runs written to {}\nopen it in a browser (double click)\n",
        all.len(),
        out.display()
    ))
}

fn stars_command(a: &Args) -> Result<String, CliError> {
    let catalog = PathBuf::from(a.required("catalog")?);
    let list = stars::read_catalog(&catalog).map_err(|e| CliError::Usage(e.to_string()))?;
    let scales = stars::build(&list);
    let source = catalog
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let page = stars::html(
        &list,
        &scales,
        &format!("{source} (HYG Database, astronexus.com, CC BY-SA 4.0)"),
    );
    let out = PathBuf::from(a.flag("out").unwrap_or("stars.html"));
    std::fs::write(&out, page).map_err(|e| CliError::Store(StoreError::Io(out.clone(), e)))?;
    Ok(format!(
        "{}\nmap written to {}\nopen it in a browser (double click)\n",
        stars::summary(&list, &scales),
        out.display()
    ))
}

fn specchio_command(a: &Args) -> Result<String, CliError> {
    let port: u16 = match a.flag("port") {
        None => specchio::PORT,
        Some(p) => p
            .parse()
            .map_err(|_| CliError::Usage(format!("--port {p} is not a port number")))?,
    };
    let open = match a.flag("open") {
        None | Some("yes") => true,
        Some("no") => false,
        Some(o) => return Err(CliError::Usage(format!("--open {o}: yes or no"))),
    };
    let ctx = specchio::Ctx {
        dir: a.dir(),
        runs: a.runs(),
        stars: PathBuf::from("stars.html"),
    };
    specchio::serve(&ctx, port, open).map_err(CliError::Usage)?;
    Ok(String::new())
}

/// Runs `uv run --project tools monade-tools ARGS` and returns what it printed.
fn tools_command(args: &[String]) -> Result<String, CliError> {
    let out = std::process::Command::new("uv")
        .args(["run", "--quiet", "--project", "tools", "monade-tools"])
        .args(args)
        .output()
        .map_err(|e| {
            CliError::Tool(format!(
                "cannot start uv ({e}).\nInstall it: powershell -ExecutionPolicy ByPass -c \"irm https://astral.sh/uv/install.ps1 | iex\"\nthen open a new terminal in the repository folder."
            ))
        })?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    if out.status.success() {
        Ok(text)
    } else {
        Err(CliError::Tool(format!(
            "the tools ended with {}\n{text}",
            out.status
        )))
    }
}

/// The current time (UTC), `YYYY-MM-DDTHH:MM:SSZ`.
fn now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    let t = secs % 86_400;
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        t / 3600,
        (t / 60) % 60,
        t % 60
    )
}

/// Year, month, day of a count of days since 1970-01-01 (H. Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &[&str], dir: &std::path::Path) -> Vec<String> {
        let mut v: Vec<String> = s.iter().map(|x| x.to_string()).collect();
        v.push("--dir".into());
        v.push(dir.display().to_string());
        v
    }

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("monade-run-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(11_017), (2000, 3, 1));
        assert_eq!(civil_from_days(20_775), (2026, 11, 18));
        assert!(monade_core::record::is_date(&now()[..10]));
        assert_eq!(now().len(), 20);
    }

    #[test]
    fn show_on_a_new_mind() {
        let d = temp("show");
        let out = run(&args(&["show"], &d), "2026-11-18").unwrap();
        assert!(out.contains("POINT 0  empty"));
        assert!(out.contains("space.seed = 0x4d4f4e4144450001"));
        assert!(!d.exists(), "show never writes");
    }

    #[test]
    fn append_and_set_then_show() {
        let d = temp("write");
        let day = "2026-11-18";
        run(
            &args(
                &[
                    "append-foundation",
                    "--name",
                    "n",
                    "--text",
                    "t",
                    "--motive",
                    "m",
                ],
                &d,
            ),
            day,
        )
        .unwrap();
        run(
            &args(&["set-spec", "vigilance", "0.625", "--motive", "m"], &d),
            day,
        )
        .unwrap();
        let out = run(&args(&["show"], &d), day).unwrap();
        assert!(out.contains("[1] n · v1 · active"));
        assert!(out.contains("vigilance = 0.625"));
        let locked = run(
            &args(&["set-spec", "space.seed", "1", "--motive", "m"], &d),
            day,
        );
        assert!(locked.is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn hand_edits_are_refused() {
        let d = temp("edit");
        let day = "2026-11-18";
        run(&args(&["set-spec", "a", "1", "--motive", "m"], &d), day).unwrap();
        let p = d.join(store::SPECS_FILE);
        let text = std::fs::read_to_string(&p).unwrap();
        std::fs::write(&p, format!("{text}# a note added by hand\n")).unwrap();
        assert!(run(&args(&["show"], &d), day).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn check_then_report() {
        let d = temp("check");
        let r = temp("check-runs");
        let html = temp("check-report.html");
        let mut a = args(&["check", "--runs"], &d);
        a.insert(2, r.display().to_string());
        let out = run(&a, "2026-11-18T18:10:05Z").unwrap();
        assert!(out.contains("16 of 16 passed"), "{out}");
        assert!(out.contains("word.xor.laws"));
        assert!(r.join("20261118-181005").join("results.jsonl").is_file());
        let rep = run(
            &[
                "report".into(),
                "--runs".into(),
                r.display().to_string(),
                "--out".into(),
                html.display().to_string(),
            ],
            "2026-11-18T18:11:00Z",
        )
        .unwrap();
        assert!(rep.contains("report of 1 runs"));
        let page = std::fs::read_to_string(&html).unwrap();
        assert!(page.contains("all 16 checks passed"));
        let _ = std::fs::remove_dir_all(&r);
        let _ = std::fs::remove_file(&html);
    }

    #[test]
    fn usage_errors_are_values() {
        let d = temp("usage");
        assert!(run(&args(&["nope"], &d), "2026-11-18").is_err());
        assert!(run(&args(&["show", "--x", "1"], &d), "2026-11-18").is_err());
        assert!(run(&args(&["set-spec", "only-key"], &d), "2026-11-18").is_err());
        assert!(run(&args(&["append-foundation"], &d), "2026-11-18").is_err());
    }
}
