//! `monade-run report`: one self-contained HTML page from the run folders.
//!
//! No server, no installation, no network: open the file in a browser.
//! Visual rules of the designer (structure v5, section 15): mostly grey, colour only
//! where it carries meaning (Okabe-Ito: blue = passed, vermilion = failed), numbers
//! always printed, dark theme by default and light when the system asks for it.

use crate::check::grouped;
use crate::runs::Run;
use std::fmt::Write as _;

/// How many runs the piano roll shows, newest on the right.
const COLUMNS: usize = 32;

/// Escapes text for HTML.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// Milliseconds with one decimal, from microseconds.
fn ms(micros: u64) -> String {
    format!("{}.{} ms", micros / 1000, (micros % 1000) / 100)
}

const STYLE: &str = "
:root{--bg:#121212;--fg:#e6e6e6;--mute:#8a8a8a;--line:#2a2a2a;--pass:#0072B2;--fail:#D55E00;--none:#2e2e2e}
@media (prefers-color-scheme: light){:root{--bg:#fafafa;--fg:#1a1a1a;--mute:#6b6b6b;--line:#dddddd;--none:#e8e8e8}}
*{box-sizing:border-box}
body{margin:0;padding:32px 40px;background:var(--bg);color:var(--fg);font:13px/1.6 system-ui,'Segoe UI',Helvetica,Arial,sans-serif;font-variant-numeric:tabular-nums}
h1{font-size:16px;font-weight:600;margin:0 0 8px;letter-spacing:.04em}
h2{font-size:13px;font-weight:600;margin:32px 0 8px;color:var(--mute);text-transform:uppercase;letter-spacing:.08em}
.meta{color:var(--mute)}
.big{font-size:16px}
.pass{color:var(--pass)} .fail{color:var(--fail)}
table{border-collapse:collapse}
td,th{padding:4px 8px;border-bottom:1px solid var(--line);vertical-align:top;text-align:left}
th{font-weight:600;color:var(--mute)}
.roll td{padding:0 2px;border:none}
.roll td.id{padding-right:16px;white-space:nowrap}
.cell{display:block;width:16px;height:16px;border-radius:2px;background:var(--none)}
.cell.p{background:var(--pass)} .cell.f{background:var(--fail)}
.mono{font-family:ui-monospace,Consolas,monospace}
details summary{cursor:pointer;color:var(--mute)}
.measures td{border:none;padding:0 16px 0 0;color:var(--mute)}
";

/// The whole page.
pub fn html(runs: &[Run], generated: &str) -> String {
    let mut h = String::new();
    let _ = write!(
        h,
        "<!DOCTYPE html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>Monade · report</title><style>{STYLE}</style></head><body>"
    );
    let _ = write!(
        h,
        "<h1>MONADE · REPORT</h1><div class=\"meta\">generated {} · {} runs</div>",
        esc(generated),
        runs.len()
    );
    let Some(last) = runs.last() else {
        h.push_str("<p>No runs yet. Run <span class=\"mono\">cargo run -p monade-run -- check</span> first.</p></body></html>\n");
        return h;
    };
    let m = &last.meta;
    let state = if m.failed == 0 {
        format!("<span class=\"pass\">all {} checks passed</span>", m.passed)
    } else {
        format!(
            "<span class=\"fail\">{} of {} checks failed</span>",
            m.failed,
            m.passed + m.failed
        )
    };
    let _ = write!(
        h,
        "<h2>Latest run</h2><div class=\"big\">{state}</div>\
         <div class=\"meta\">{} · v{} · {} {} · {} · {} threads · {}</div>",
        esc(&m.started),
        esc(&m.version),
        esc(&m.os),
        esc(&m.arch),
        esc(&m.cpu.join(" ")),
        m.threads,
        ms(m.micros)
    );

    roll(&mut h, runs);
    details(&mut h, last);

    h.push_str(
        "<h2>How to read this</h2><p class=\"meta\">Each row is a check, each column a run \
         (newest on the right). Blue = the claim held, vermilion = it did not, grey = not run. \
         Hover a cell for its result. Below, the latest run in full: what is claimed, what was \
         measured, what was expected. Regenerate with \
         <span class=\"mono\">cargo run -p monade-run -- report</span>.</p></body></html>\n",
    );
    h
}

/// The piano roll: rows = checks, columns = runs.
fn roll(h: &mut String, runs: &[Run]) {
    let shown = &runs[runs.len().saturating_sub(COLUMNS)..];
    let mut ids: Vec<&str> = Vec::new();
    for r in shown.iter().rev() {
        for o in &r.outcomes {
            if !ids.contains(&o.id.as_str()) {
                ids.push(&o.id);
            }
        }
    }
    let _ = write!(
        h,
        "<h2>History · {} runs</h2><table class=\"roll\">",
        shown.len()
    );
    for id in ids {
        let _ = write!(h, "<tr><td class=\"id mono\">{}</td>", esc(id));
        for r in shown {
            match r.outcomes.iter().find(|o| o.id == id) {
                Some(o) => {
                    let _ = write!(
                        h,
                        "<td><span class=\"cell {}\" title=\"{} · {}\"></span></td>",
                        if o.passed { "p" } else { "f" },
                        esc(&r.meta.started),
                        esc(&o.result)
                    );
                }
                None => h.push_str("<td><span class=\"cell\" title=\"not run\"></span></td>"),
            }
        }
        h.push_str("</tr>");
    }
    h.push_str("</table>");
}

/// The latest run in full.
fn details(h: &mut String, run: &Run) {
    h.push_str(
        "<h2>Latest run in full</h2><table><tr><th></th><th>check</th><th>claim</th>\
         <th>measured</th><th>expected</th><th>time</th></tr>",
    );
    for o in &run.outcomes {
        let mark = if o.passed {
            "<span class=\"pass\">&#10003;</span>"
        } else {
            "<span class=\"fail\">&#10007;</span>"
        };
        let mut measures =
            String::from("<details><summary>numbers</summary><table class=\"measures\">");
        for m in &o.measures {
            let _ = write!(
                measures,
                "<tr><td>{}</td><td class=\"mono\">{}</td></tr>",
                esc(&m.name),
                esc(&m.value)
            );
        }
        measures.push_str("</table></details>");
        let _ = write!(
            h,
            "<tr><td>{mark}</td><td class=\"mono\">{}</td><td>{}</td><td>{}{measures}</td>\
             <td class=\"meta\">{}</td><td class=\"meta\">{}</td></tr>",
            esc(&o.id),
            esc(&o.claim),
            esc(&o.result),
            esc(&o.expected),
            ms(o.micros)
        );
    }
    let total: u64 = run.outcomes.iter().map(|o| o.micros).sum();
    let _ = write!(
        h,
        "</table><div class=\"meta\">{} checks · {} µs in total</div>",
        run.outcomes.len(),
        grouped(total)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{Measure, Outcome};
    use crate::runs::Meta;

    fn run(name: &str, passed: bool) -> Run {
        Run {
            meta: Meta {
                name: name.into(),
                version: "0.2.0".into(),
                started: format!("{name}Z"),
                os: "windows".into(),
                arch: "x86_64".into(),
                cpu: vec!["avx2".into()],
                threads: 16,
                passed: u64::from(passed),
                failed: u64::from(!passed),
                micros: 1500,
            },
            outcomes: vec![Outcome {
                id: "word.xor.laws".into(),
                area: "word".into(),
                claim: "a < b & \"c\"".into(),
                result: "ok".into(),
                expected: "all".into(),
                passed,
                micros: 1500,
                measures: vec![Measure {
                    name: "pairs".into(),
                    value: "10000".into(),
                }],
            }],
        }
    }

    #[test]
    fn empty_page_says_what_to_do() {
        let p = html(&[], "now");
        assert!(p.contains("No runs yet"));
        assert!(p.ends_with("</html>\n"));
    }

    #[test]
    fn page_shows_history_and_escapes() {
        let p = html(&[run("a", true), run("b", false)], "now");
        assert!(p.contains("1 of 1 checks failed"));
        assert!(p.contains("class=\"cell p\""));
        assert!(p.contains("class=\"cell f\""));
        assert!(p.contains("a &lt; b &amp; &quot;c&quot;"));
        assert!(!p.contains("a < b"));
        assert_eq!(ms(1500), "1.5 ms");
    }
}
