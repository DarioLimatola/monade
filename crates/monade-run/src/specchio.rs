//! Specchio, the interface of the Monade (Leibniz: every monad is a living mirror
//! of the universe).
//!
//! A small web page served **on this computer only** (127.0.0.1): nothing is
//! reachable from the network. Seven rooms; the first ones show what exists today
//! (the checks, the stars, the foundations), the others say which step brings them.
//! Every page is built by [`respond`] (and every command by [`command`]), so they
//! can be tested without a server; [`serve`] only moves bytes.
//!
//! Commands are a fixed list (check, doctor, show, stars, crawl, library): nothing
//! else can be run.
//! They are accepted only as POST with the header `X-Monade: 1`, which a page of
//! another site cannot send to this server (the browser blocks it), so no web page
//! can start a command behind the author's back.

use crate::report::{esc, STYLE};
use crate::runs;
use std::io::Read as _;
use std::path::PathBuf;

/// Default port.
pub const PORT: u16 = 8088;

/// Where Specchio finds what it shows.
pub struct Ctx {
    /// Folder of point 0 and the specifications.
    pub dir: PathBuf,
    /// Folder of the runs of `check`.
    pub runs: PathBuf,
    /// The stars map written by `monade-run stars`.
    pub stars: PathBuf,
}

/// One answer: status, type, body.
#[derive(Debug, PartialEq, Eq)]
pub struct Reply {
    /// HTTP status code.
    pub status: u16,
    /// Content type.
    pub content_type: &'static str,
    /// The body.
    pub body: String,
}

/// The rooms: path, name, what it holds, and the step that brings it (empty if it
/// exists already).
const ROOMS: [(&str, &str, &str, &str); 7] = [
    (
        "mind",
        "Mind",
        "the checks, what was verified, the certainty of the mind",
        "",
    ),
    (
        "commands",
        "Commands",
        "the everyday commands, one click each",
        "",
    ),
    (
        "worlds",
        "Worlds",
        "the worlds the mind lives in; today the stars",
        "",
    ),
    (
        "lessons",
        "Lessons",
        "dataset cards, lessons, learning curves",
        "step 6b (the gate and the lessons)",
    ),
    (
        "blackboard",
        "Blackboard",
        "voices, consensus and residue: an 8 × 8 grid",
        "step 6 (the blackboard)",
    ),
    (
        "map",
        "Map",
        "the inside of the mind in 3D: Monadi, words, bindings, sentences and their references; any point drawn as a cube of bits",
        "",
    ),
    (
        "foundations",
        "Foundations",
        "point 0 and the specifications, read-only",
        "",
    ),
];

const HTML: &str = "text/html; charset=utf-8";

fn reply(status: u16, content_type: &'static str, body: String) -> Reply {
    Reply {
        status,
        content_type,
        body,
    }
}

/// A plain page in the style of the report.
fn page(title: &str, body: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>{}</title><style>{STYLE}</style></head><body>{body}</body></html>\n",
        esc(title)
    )
}

/// The frame: header, the rooms on the left, the chosen room on the right.
fn index(version: &str) -> String {
    let mut nav = String::new();
    for (i, (path, name, what, _)) in ROOMS.iter().enumerate() {
        nav.push_str(&format!(
            "<a href=\"#{path}\" data-room=\"{path}\" title=\"{}\"{}>{}</a>",
            esc(what),
            if i == 0 { " class=\"on\"" } else { "" },
            esc(name)
        ));
    }
    let body = format!(
        "<style>\
         body{{padding:0;display:grid;grid-template-rows:auto 1fr;height:100vh}}\
         header{{padding:12px 24px;border-bottom:1px solid var(--line)}}\
         main{{display:grid;grid-template-columns:160px 1fr;min-height:0}}\
         nav{{display:flex;flex-direction:column;padding:16px 0;border-right:1px solid var(--line)}}\
         nav a{{color:var(--mute);text-decoration:none;padding:6px 24px}}\
         nav a.on{{color:var(--fg);border-left:2px solid var(--pass);padding-left:22px}}\
         iframe{{border:0;width:100%;height:100%;background:var(--bg)}}\
         </style>\
         <header><h1>SPECCHIO · MONADE v{}</h1><div class=\"meta\" id=\"state\">…</div></header>\
         <main><nav>{nav}</nav><iframe id=\"room\" src=\"/mind\" title=\"room\"></iframe></main>\
         <script>\
         const f=document.getElementById('room');\
         function go(){{const r=(location.hash||'#mind').slice(1);f.src='/'+r;\
         document.querySelectorAll('nav a').forEach(a=>a.className=a.dataset.room===r?'on':'');}}\
         window.addEventListener('hashchange',go);go();\
         fetch('/api/state').then(r=>r.json()).then(s=>{{document.getElementById('state').textContent=\
         s.runs+' runs · last: '+(s.last?s.last.started+' · '+s.last.passed+' passed, '+s.last.failed+' failed':'none yet (run check)');}});\
         </script>",
        esc(version)
    );
    page("Specchio · Monade", &body)
}

/// A room that does not exist yet.
fn coming(name: &str, what: &str, step: &str) -> String {
    page(
        name,
        &format!(
            "<h2>{}</h2><p class=\"intro\">{}.</p><p class=\"meta\">Not built yet: it comes with {}.</p>",
            esc(name),
            esc(what),
            esc(step)
        ),
    )
}

/// The state of the mind as JSON: version, number of runs, the last run.
fn state(ctx: &Ctx) -> String {
    let all = runs::load_all(&ctx.runs).unwrap_or_default();
    let last = all.last().map_or("null".to_string(), |r| {
        format!(
            "{{\"started\":{},\"passed\":{},\"failed\":{}}}",
            serde_json::Value::String(r.meta.started.clone()),
            r.meta.passed,
            r.meta.failed
        )
    });
    format!(
        "{{\"version\":\"{}\",\"runs\":{},\"last\":{last}}}",
        env!("CARGO_PKG_VERSION"),
        all.len()
    )
}

/// The answer to a request for `path`; `now` is the time shown in the report.
pub fn respond(path: &str, ctx: &Ctx, now: &str) -> Reply {
    let full = path;
    let path = path.split(['?', '#']).next().unwrap_or("/");
    match path {
        "/map" => reply(200, HTML, crate::map::page()),
        "/tree" => reply(200, HTML, crate::map::tree_page()),
        "/api/tree" => match crate::stars::Sky::load(&ctx.stars.with_extension("sky")) {
            Ok(sky) => {
                let a = crate::map::query(full, "a").unwrap_or_default();
                reply(200, "application/json", crate::map::tree(&sky, &a).to_string())
            }
            Err(_) => reply(200, "application/json", "{\"missing\":true}".into()),
        },
        "/api/map" => {
            let text = crate::map::query(full, "s").unwrap_or_else(|| crate::map::DEFAULT.to_string());
            let space = crate::store::load_specs(&ctx.dir)
                .ok()
                .and_then(|s| monade_core::space::Space::from_specs(&s).ok())
                .unwrap_or_else(monade_core::space::Space::genesis);
            reply(200, "application/json", crate::map::graph(&space, &text).to_string())
        }
        "/" | "/index.html" => reply(200, HTML, index(env!("CARGO_PKG_VERSION"))),
        "/api/state" => reply(200, "application/json", state(ctx)),
        "/mind" => match runs::load_all(&ctx.runs) {
            Ok(all) => reply(200, HTML, crate::report::html(&all, now)),
            Err(e) => reply(
                200,
                HTML,
                page("Mind", &format!("<h2>Mind</h2><p>{}</p>", esc(&e.to_string()))),
            ),
        },
        "/worlds" => match std::fs::read_to_string(&ctx.stars) {
            Ok(map) => reply(200, HTML, map),
            Err(_) => reply(
                200,
                HTML,
                page(
                    "Worlds",
                    "<h2>Worlds</h2><p class=\"intro\">No world yet. The stars: \
                     <span class=\"mono\">cargo run --release -p monade-run -- stars --catalog hyg.csv</span>, \
                     then reload this room.</p>",
                ),
            ),
        },
        "/commands" => reply(200, HTML, commands_page()),
        "/foundations" => {
            let text = crate::show_foundations(&ctx.dir).unwrap_or_else(|e| e.to_string());
            reply(
                200,
                HTML,
                page(
                    "Foundations",
                    &format!(
                        "<h2>Foundations</h2><p class=\"meta\">Read-only: only the author writes them, with monade-run.</p><pre class=\"mono\">{}</pre>",
                        esc(&text)
                    ),
                ),
            )
        }
        other => {
            let name = other.trim_start_matches('/');
            match ROOMS.iter().find(|(p, ..)| *p == name) {
                Some((_, title, what, step)) => reply(200, HTML, coming(title, what, step)),
                None => reply(404, HTML, page("Not found", "<h2>Not found</h2>")),
            }
        }
    }
}

/// The commands that can be run from Specchio: path, button, what it does.
const COMMANDS: [(&str, &str, &str); 6] = [
    (
        "check",
        "Verify",
        "runs every check of the Monade and saves the run (then see Mind)",
    ),
    (
        "doctor",
        "Doctor",
        "what the Python tools see: Python, data folder, free disk, GPU",
    ),
    ("show", "Foundations", "point 0 and the specifications"),
    (
        "crawl",
        "Crawl",
        "the crawler follows the links between authors in the background, for at most 30 minutes",
    ),
    (
        "library",
        "Library",
        "what the crawler has gathered: counts of authors, works, refusals, lessons queued",
    ),
    (
        "stars",
        "Stars",
        "builds the stars map from a catalog file (path below), then see Worlds",
    ),
];

/// The script of the commands page: one POST per button, the answer shown below.
const COMMANDS_SCRIPT: &str = "<script>
const out = document.getElementById('out');
const all = () => document.querySelectorAll('button');
document.querySelectorAll('button[data-cmd]').forEach(b => b.onclick = async () => {
  const c = b.dataset.cmd;
  out.textContent = b.textContent + ': running…';
  all().forEach(x => x.disabled = true);
  try {
    const r = await fetch('/run/' + c, {
      method: 'POST',
      headers: { 'X-Monade': '1' },
      body: c === 'stars' ? document.getElementById('catalog').value : ''
    });
    out.textContent = await r.text();
  } catch (e) {
    out.textContent = 'error: ' + e;
  }
  all().forEach(x => x.disabled = false);
});
</script>";

/// The page of the commands: one button each, the output below.
fn commands_page() -> String {
    let mut buttons = String::new();
    for (path, label, what) in COMMANDS {
        buttons.push_str(&format!(
            "<div class=\"row\"><button data-cmd=\"{path}\">{}</button> <span class=\"meta\">{}</span></div>",
            esc(label),
            esc(what)
        ));
    }
    page(
        "Commands",
        &format!(
            "<style>button{{background:var(--bg);color:var(--fg);border:1px solid var(--line);\
             border-radius:4px;padding:3px 10px;min-width:104px;font:inherit;cursor:pointer}}\
             button:hover{{border-color:var(--pass)}} button:disabled{{color:var(--mute);cursor:wait}}</style>\
             <h2>Commands</h2>{buttons}\
             <div class=\"row\"><span class=\"label\">Catalog</span><input id=\"catalog\" size=\"60\" \
             placeholder=\"C:\\monade-data\\stars\\hygdata_v41.csv\" \
             style=\"background:var(--bg);color:var(--fg);border:1px solid var(--line);padding:3px 6px\"></div>\
             <h2>Output</h2><pre id=\"out\" class=\"mono\">Press a button.</pre>{COMMANDS_SCRIPT}"
        ),
    )
}

/// Runs one of the fixed commands; `trusted` is true only for a POST that carries
/// the header `X-Monade: 1`; `body` is the catalog path for `stars`.
pub fn command(path: &str, body: &str, trusted: bool, ctx: &Ctx) -> Reply {
    const TEXT: &str = "text/plain; charset=utf-8";
    if !trusted {
        return reply(
            403,
            TEXT,
            "commands are accepted only from Specchio itself\n".into(),
        );
    }
    let dir = ctx.dir.display().to_string();
    let runs = ctx.runs.display().to_string();
    let stars = ctx.stars.display().to_string();
    let args: Vec<String> = match path.trim_start_matches("/run/") {
        "check" => vec!["check".into(), "--dir".into(), dir, "--runs".into(), runs],
        "doctor" | "library" => {
            let tool: Vec<String> = if path.ends_with("doctor") {
                vec!["doctor".into()]
            } else {
                vec!["crawl".into(), "status".into()]
            };
            return match crate::tools_output(&tool) {
                Ok(text) => reply(200, TEXT, text),
                Err(e) => reply(200, TEXT, format!("error: {e}\n")),
            };
        }
        "crawl" => {
            return match crawl_in_background() {
                Ok(()) => reply(
                    200,
                    TEXT,
                    "crawl started in the background (at most 30 minutes).\n\
                     Press Library to see what it has gathered; the full log is crawl.log\n\
                     in the folder biblioteca of the data folder. Close Specchio freely: the crawl goes on.\n"
                        .into(),
                ),
                Err(e) => reply(200, TEXT, format!("error: {e}\n")),
            };
        }
        "show" => vec!["show".into(), "--dir".into(), dir],
        "stars" => {
            let catalog = body.trim().trim_matches('"');
            if catalog.is_empty() {
                return reply(
                    200,
                    TEXT,
                    "write the path of the catalog file first\n".into(),
                );
            }
            vec![
                "stars".into(),
                "--catalog".into(),
                catalog.into(),
                "--out".into(),
                stars,
            ]
        }
        _ => return reply(404, TEXT, "no such command\n".into()),
    };
    match crate::run(&args, &crate::now()) {
        Ok(text) => reply(200, TEXT, text),
        Err(e) => reply(200, TEXT, format!("error: {e}\n")),
    }
}

/// Starts `monade-tools crawl run --minutes 30` as a separate process and returns at
/// once; the crawl writes its own log. A second crawl refuses to start (lock file).
fn crawl_in_background() -> Result<(), String> {
    std::process::Command::new("uv")
        .args([
            "run",
            "--quiet",
            "--project",
            "tools",
            "monade-tools",
            "crawl",
            "run",
            "--minutes",
            "30",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("cannot start uv ({e}); see the Doctor"))
}

/// Serves Specchio on 127.0.0.1:`port` until the process is stopped (Ctrl+C);
/// opens the default browser if `open`.
pub fn serve(ctx: &Ctx, port: u16, open: bool) -> Result<(), String> {
    let addr = format!("127.0.0.1:{port}");
    let server = tiny_http::Server::http(&addr).map_err(|e| {
        format!("cannot listen on {addr}: {e} (another program on this port? try --port 8089)")
    })?;
    let url = format!("http://{addr}/");
    println!("Specchio: {url}  (only on this computer; Ctrl+C to stop)");
    if open {
        open_browser(&url);
    }
    for mut request in server.incoming_requests() {
        let now = crate::now().replace('T', " ").replace('Z', " UTC");
        let url = request.url().to_string();
        let r = if url.starts_with("/run/") {
            let post = *request.method() == tiny_http::Method::Post;
            let marked = request
                .headers()
                .iter()
                .any(|h| h.field.equiv("X-Monade") && h.value.as_str() == "1");
            let mut body = String::new();
            let _ = request.as_reader().take(4096).read_to_string(&mut body);
            command(&url, &body, post && marked, ctx)
        } else {
            respond(&url, ctx, &now)
        };
        let header = tiny_http::Header::from_bytes(&b"Content-Type"[..], r.content_type.as_bytes())
            .map_err(|()| "bad header".to_string())?;
        let response = tiny_http::Response::from_string(r.body)
            .with_status_code(r.status)
            .with_header(header);
        if let Err(e) = request.respond(response) {
            eprintln!("specchio: {e}");
        }
    }
    Ok(())
}

/// Asks the system to open `url` in the default browser; failures are only reported.
fn open_browser(url: &str) {
    let result = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .status()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).status()
    } else {
        std::process::Command::new("xdg-open").arg(url).status()
    };
    if let Err(e) = result {
        eprintln!("open the browser at {url} ({e})");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(name: &str) -> Ctx {
        let base =
            std::env::temp_dir().join(format!("monade-specchio-{name}-{}", std::process::id()));
        Ctx {
            dir: base.join("foundations"),
            runs: base.join("runs"),
            stars: base.join("stars.html"),
        }
    }

    #[test]
    fn every_room_answers() {
        let c = ctx("rooms");
        let home = respond("/", &c, "now");
        assert_eq!(home.status, 200);
        assert!(home.body.contains("SPECCHIO"));
        for (path, name, ..) in ROOMS {
            let r = respond(&format!("/{path}"), &c, "now");
            assert_eq!(r.status, 200, "{path}");
            assert!(r.body.contains("</html>"), "{path}");
            assert!(home.body.contains(name));
        }
        assert!(respond("/lessons", &c, "now").body.contains("step 6b"));
        assert!(respond("/foundations", &c, "now").body.contains("POINT 0"));
        assert_eq!(respond("/nothing", &c, "now").status, 404);
        assert_eq!(respond("/mind?x=1", &c, "now").status, 200);
        let m = respond("/api/map?s=il+cane+dorme", &c, "now");
        let v: serde_json::Value = serde_json::from_str(&m.body).unwrap();
        assert_eq!(v["sentences"], 1);
        assert!(respond("/map", &c, "now").body.contains("il cane dorme"));
    }

    #[test]
    fn commands_need_the_mark_and_are_a_fixed_list() {
        let c = ctx("commands");
        assert_eq!(command("/run/show", "", false, &c).status, 403);
        let shown = command("/run/show", "", true, &c);
        assert_eq!(shown.status, 200);
        assert!(shown.body.contains("POINT 0"));
        assert_eq!(command("/run/rm", "", true, &c).status, 404);
        assert!(command("/run/stars", "  ", true, &c).body.contains("path"));
        assert!(command("/run/stars", "no-such-file.csv", true, &c)
            .body
            .starts_with("error"));
        assert!(respond("/commands", &c, "now")
            .body
            .contains("data-cmd=\"check\""));
    }

    #[test]
    fn state_is_json() {
        let r = respond("/api/state", &ctx("state"), "now");
        let v: serde_json::Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!(v["runs"], 0);
        assert!(v["last"].is_null());
        assert_eq!(v["version"], env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn text_is_escaped() {
        assert!(coming("<x>", "a & b", "s").contains("&lt;x&gt;"));
    }
}
