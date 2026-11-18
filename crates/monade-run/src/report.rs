//! `monade-run report`: one self-contained HTML page from the run folders.
//!
//! No server, no installation, no network: open the file in a browser.
//! Visual rules of the designer (structure v5, section 15): mostly grey, colour only
//! where it carries meaning (Okabe-Ito: blue = passed, vermilion = failed), numbers
//! always printed, dark theme by default and light when the system asks for it.

use crate::check::{grouped, Histogram, Outcome};
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
.card{border-top:1px solid var(--line);padding:16px 0 24px;max-width:960px}
.card h3{font-size:14px;font-weight:600;margin:0 0 8px}
.label{color:var(--mute);display:inline-block;min-width:96px;vertical-align:top}
.row{margin:4px 0}
.text{display:inline-block;max-width:820px}
.intro{max-width:820px}
svg text{fill:var(--mute);font:11px system-ui,'Segoe UI',sans-serif}
svg .axis{stroke:var(--line)}
dl{max-width:820px} dt{font-weight:600;margin-top:8px} dd{margin:0 0 0 16px;color:var(--mute)}
";

/// What the Monade is today, in plain words (updated with each step).
const TODAY: &str = "Today the Monade is an algebra and a memory container: \
vectors of bits from 64 bits to 2 MB, the operations that bind (XOR), compare \
(Hamming distance) and superpose (majority) them, arenas that store them, an octal \
address that reaches any part down to one bit, and the foundations (point 0, still \
empty, and the specifications with the locked seed). It does not learn yet: \
learning starts with the counters (step 5), the ternary unit (step 7) and the \
learning cycle (step 10). These checks prove that the ground it will stand on is exact.";

/// Short definitions of the words used in the checks.
const GLOSSARY: &[(&str, &str)] = &[
    ("Bit vector", "A row of 0s and 1s: 64 bits (a word), 512 (a block), 4096 (a symbol), up to 2 MB (a thought). Every idea of the Monade is one."),
    ("XOR (binding)", "Bit by bit: 1 where the two vectors differ. Ties a role to a filler; applied twice it unties."),
    ("Hamming distance", "How many bits differ between two vectors. Half the bits = strangers; few = similar."),
    ("Majority (bundle)", "Bit by bit vote among several vectors; a tie gives 0. Holds several voices in one vector."),
    ("Decoy", "A random vector used as a comparison: a member must be closer than every decoy."),
    ("Standard deviation (sigma)", "The usual spread of a measure around its mean; 6 sigma away is practically impossible by chance."),
    ("Oracle", "Slow, obvious reference code that reads one bit at a time; the fast code must always agree with it."),
    ("State hash", "A 64-bit fingerprint of a state: equal fingerprints mean equal states."),
    ("Octal address, Morton order", "A path of base-8 digits; each digit is an octant of a cube, so the path is also a point in space."),
    ("Seed", "The number from which the whole generated space is computed; locked: a new seed is a new mind."),
    ("Point 0", "The origin: the empty coordinate and the log of foundations, written only by the author."),
];

/// A histogram as an inline SVG: bars per bin, one colour per series.
fn chart(h: &Histogram) -> String {
    const W: u64 = 640;
    const H: u64 = 120;
    let bins = h.series.iter().map(|s| s.counts.len()).max().unwrap_or(0) as u64;
    let top = h
        .series
        .iter()
        .flat_map(|s| s.counts.iter().copied())
        .max()
        .unwrap_or(0)
        .max(1);
    if bins == 0 {
        return String::new();
    }
    let bw = W / bins;
    let colours = ["#0072B2", "#E69F00"];
    let mut svg = format!(
        "<div class=\"meta\">{}</div><svg width=\"{}\" height=\"{}\" role=\"img\" aria-label=\"{}\">",
        esc(&h.title),
        W + 8,
        H + 40,
        esc(&h.title)
    );
    for (si, s) in h.series.iter().enumerate() {
        let colour = colours[si % colours.len()];
        for (i, c) in s.counts.iter().enumerate() {
            if *c == 0 {
                continue;
            }
            let bh = (c * H / top).max(1);
            let _ = write!(
                svg,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{bh}\" fill=\"{colour}\" fill-opacity=\"0.75\"><title>{}: {}–{}: {c}</title></rect>",
                i as u64 * bw,
                H - bh,
                bw.saturating_sub(1).max(1),
                esc(&s.name),
                h.start + i as u64 * h.width,
                h.start + (i as u64 + 1) * h.width - 1
            );
        }
    }
    let end = h.start + bins * h.width;
    let mid = h.start + bins / 2 * h.width;
    let _ = write!(
        svg,
        "<line class=\"axis\" x1=\"0\" y1=\"{H}\" x2=\"{W}\" y2=\"{H}\"/>\
         <text x=\"0\" y=\"{}\">{}</text><text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{mid}</text>\
         <text x=\"{W}\" y=\"{}\" text-anchor=\"end\">{end}</text>\
         <text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
        H + 14,
        h.start,
        W / 2,
        H + 14,
        H + 14,
        W / 2,
        H + 32,
        esc(&h.unit)
    );
    let mut x = 0;
    for (si, s) in h.series.iter().enumerate() {
        let colour = colours[si % colours.len()];
        let _ = write!(
            svg,
            "<rect x=\"{x}\" y=\"{}\" width=\"10\" height=\"10\" fill=\"{colour}\"/><text x=\"{}\" y=\"{}\">{}</text>",
            H + 22,
            x + 14,
            H + 31,
            esc(&s.name)
        );
        x += 14 + 8 * s.name.len() as u64;
    }
    svg.push_str("</svg>");
    svg
}

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

    let _ = write!(
        h,
        "<h2>What the Monade is today</h2><p class=\"intro\">{}</p>",
        esc(TODAY)
    );
    roll(&mut h, runs);
    details(&mut h, last);
    h.push_str("<h2>Words used here</h2><dl>");
    for (term, meaning) in GLOSSARY {
        let _ = write!(h, "<dt>{}</dt><dd>{}</dd>", esc(term), esc(meaning));
    }
    h.push_str("</dl>");

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

/// The latest run in full: one card per check.
fn details(h: &mut String, run: &Run) {
    h.push_str("<h2>Latest run in full</h2>");
    for o in &run.outcomes {
        card(h, o);
    }
    let total: u64 = run.outcomes.iter().map(|o| o.micros).sum();
    let _ = write!(
        h,
        "<div class=\"meta\">{} checks · {} µs in total</div>",
        run.outcomes.len(),
        grouped(total)
    );
}

/// One check: claim, why it matters, how it works, what was measured, numbers, charts.
fn card(h: &mut String, o: &Outcome) {
    let mark = if o.passed {
        "<span class=\"pass\">&#10003; passed</span>"
    } else {
        "<span class=\"fail\">&#10007; failed</span>"
    };
    let _ = write!(
        h,
        "<div class=\"card\"><h3><span class=\"mono\">{}</span> · {mark} <span class=\"meta\">· {} · {}</span></h3>",
        esc(&o.id),
        esc(&o.area),
        ms(o.micros)
    );
    let row = |h: &mut String, label: &str, text: &str| {
        if !text.is_empty() {
            let _ = write!(
                h,
                "<div class=\"row\"><span class=\"label\">{label}</span><span class=\"text\">{}</span></div>",
                esc(text)
            );
        }
    };
    row(h, "Claim", &o.claim);
    row(h, "Why it matters", &o.why);
    row(h, "How", &o.how);
    row(h, "Measured", &o.result);
    row(h, "Expected", &o.expected);
    if !o.measures.is_empty() {
        h.push_str("<div class=\"row\"><span class=\"label\">Numbers</span><table class=\"measures text\">");
        for m in &o.measures {
            let _ = write!(
                h,
                "<tr><td>{}</td><td class=\"mono\">{}</td></tr>",
                esc(&m.name),
                esc(&m.value)
            );
        }
        h.push_str("</table></div>");
    }
    for c in &o.charts {
        h.push_str(&chart(c));
    }
    h.push_str("</div>");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{Measure, Series};
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
                why: "w".into(),
                how: "h".into(),
                charts: vec![Histogram {
                    title: "t".into(),
                    unit: "u".into(),
                    start: 10,
                    width: 2,
                    series: vec![Series {
                        name: "s".into(),
                        counts: vec![1, 0, 3],
                    }],
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
        assert!(p.contains("Why it matters"));
        assert!(p.contains("<svg"));
        assert!(p.contains("What the Monade is today"));
        assert!(p.contains("class=\"cell p\""));
        assert!(p.contains("class=\"cell f\""));
        assert!(p.contains("a &lt; b &amp; &quot;c&quot;"));
        assert!(!p.contains("a < b"));
        assert_eq!(ms(1500), "1.5 ms");
    }
}
