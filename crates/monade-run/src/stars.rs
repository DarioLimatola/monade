//! A first world: the stars of a catalog, held in Monadi.
//!
//! A 2 MB Monade is a cube of 256 x 256 x 256 cells, one bit per cell (the octal
//! address: 8 base-8 digits from the whole cube down to one cell). Each star lights
//! the bit of its cell, so one Monade is a map of the sky around the Sun at one
//! scale. Four scales (half-widths 16, 64, 256, 1024 parsecs), each 4 times the one
//! before, are four Monadi. The other dimensions (seen with the naked eye, colour)
//! are further Monadi with the same addresses: layers of the same map.
//!
//! Integers only: positions are read from the catalog's decimal text as
//! thousandths of a parsec, never as floating point.

use crate::check::grouped;
use monade_core::address::Address;
use monade_core::size::Size;
use monade_core::word::{self, Word};
use std::fmt::{self, Write as _};
use std::path::{Path, PathBuf};

/// Half-width of each scale's cube, in parsecs (the Sun at the centre).
pub const HALF_WIDTHS_PC: [i64; 4] = [16, 64, 256, 1024];

/// The layers of every scale: one Monade each, same addresses.
pub const LAYERS: [&str; 7] = [
    "every star",
    "seen with the naked eye (magnitude 6 or brighter)",
    "blue (colour index below 0)",
    "white (0 to 0.5)",
    "yellow (0.5 to 1)",
    "orange (1 to 1.5)",
    "red (1.5 and above)",
];

const SIDE: i64 = 256;

/// One star: position in thousandths of a parsec, magnitude and colour index in thousandths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Star {
    /// x, y, z (equatorial, the Sun at 0), thousandths of a parsec.
    pub pos: [i64; 3],
    /// Apparent magnitude, thousandths (smaller is brighter).
    pub mag: Option<i64>,
    /// Colour index B-V, thousandths (smaller is bluer).
    pub ci: Option<i64>,
    /// Proper name, if any.
    pub name: String,
}

/// Errors of the stars world.
#[derive(Debug)]
pub enum StarsError {
    /// The catalog cannot be read.
    Read(PathBuf, String),
    /// A column the world needs is missing.
    Column(&'static str),
}

impl fmt::Display for StarsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StarsError::Read(p, e) => write!(f, "{}: {e}", p.display()),
            StarsError::Column(c) => write!(f, "the catalog has no column `{c}`"),
        }
    }
}

/// A decimal number as thousandths, without floating point: "-1.23456" → -1234.
pub fn parse_milli(s: &str) -> Option<i64> {
    let s = s.trim();
    let (neg, digits) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let (int, frac) = digits.split_once('.').unwrap_or((digits, ""));
    if int.is_empty() && frac.is_empty() {
        return None;
    }
    if !int.bytes().chain(frac.bytes()).all(|b| b.is_ascii_digit()) {
        return None;
    }
    let whole: i64 = if int.is_empty() { 0 } else { int.parse().ok()? };
    let mut milli = 0i64;
    for (i, b) in frac.bytes().take(3).enumerate() {
        milli += i64::from(b - b'0') * [100, 10, 1][i];
    }
    let v = whole.checked_mul(1000)?.checked_add(milli)?;
    Some(if neg { -v } else { v })
}

/// Reads the stars of an HYG-like CSV (columns x, y, z, mag, ci, proper).
pub fn read_catalog(path: &Path) -> Result<Vec<Star>, StarsError> {
    let err = |e: String| StarsError::Read(path.to_path_buf(), e);
    let mut r = csv::Reader::from_path(path).map_err(|e| err(e.to_string()))?;
    let headers = r.headers().map_err(|e| err(e.to_string()))?.clone();
    let col = |name: &'static str| {
        headers
            .iter()
            .position(|h| h == name)
            .ok_or(StarsError::Column(name))
    };
    let (x, y, z) = (col("x")?, col("y")?, col("z")?);
    let (mag, ci, proper) = (col("mag")?, col("ci")?, col("proper")?);
    let mut out = Vec::new();
    for rec in r.records() {
        let rec = rec.map_err(|e| err(e.to_string()))?;
        let get = |i: usize| rec.get(i).unwrap_or("");
        let (Some(px), Some(py), Some(pz)) = (
            parse_milli(get(x)),
            parse_milli(get(y)),
            parse_milli(get(z)),
        ) else {
            continue;
        };
        out.push(Star {
            pos: [px, py, pz],
            mag: parse_milli(get(mag)),
            ci: parse_milli(get(ci)),
            name: get(proper).to_string(),
        });
    }
    Ok(out)
}

/// The cell of a position in a cube of half-width `half` (thousandths of a parsec).
pub fn cell(pos: [i64; 3], half: i64) -> Option<(u32, u32, u32)> {
    let c = |p: i64| {
        let shifted = p.checked_add(half)?;
        if shifted < 0 {
            return None;
        }
        let i = shifted.checked_mul(SIDE)? / (2 * half);
        u32::try_from(i).ok().filter(|v| i64::from(*v) < SIDE)
    };
    Some((c(pos[0])?, c(pos[1])?, c(pos[2])?))
}

/// The layers a star belongs to (index into `LAYERS`).
fn layers_of(s: &Star) -> Vec<usize> {
    let mut l = vec![0];
    if s.mag.is_some_and(|m| m <= 6000) {
        l.push(1);
    }
    if let Some(ci) = s.ci {
        l.push(match ci {
            i64::MIN..0 => 2,
            0..500 => 3,
            500..1000 => 4,
            1000..1500 => 5,
            _ => 6,
        });
    }
    l
}

/// One scale: seven Monadi of 2 MB (the layers) and what was measured building them.
pub struct Scale {
    /// Half-width of the cube, parsecs.
    pub half_pc: i64,
    /// One Monade (2 MB) per layer.
    pub layers: Vec<Vec<Word>>,
    /// Stars inside the cube.
    pub inside: u64,
    /// Stars that fell in a cell already lit.
    pub shared: u64,
    /// Sum of the distances between each star and the centre of its cell, thousandths of a parsec.
    pub error_sum: u64,
    /// Largest such distance.
    pub error_max: u64,
    /// Named bright stars (magnitude below 1.5): name and cell.
    pub names: Vec<(String, (u32, u32, u32))>,
}

fn isqrt(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Builds the four scales from the stars.
pub fn build(stars: &[Star]) -> Vec<Scale> {
    HALF_WIDTHS_PC
        .iter()
        .enumerate()
        .map(|(root, half_pc)| {
            let half = half_pc * 1000;
            let mut layers = vec![vec![Word::ZERO; Size::THOUGHT.words()]; LAYERS.len()];
            let mut sc = Scale {
                half_pc: *half_pc,
                layers: Vec::new(),
                inside: 0,
                shared: 0,
                error_sum: 0,
                error_max: 0,
                names: Vec::new(),
            };
            for s in stars {
                let Some(c) = cell(s.pos, half) else { continue };
                let Ok(a) = Address::from_point(root as u32, Size::THOUGHT, c, 8) else {
                    continue;
                };
                let offset = a.bit_offset();
                let (w, b) = ((offset / 64) as usize, offset % 64);
                sc.inside += 1;
                if layers[0][w].bits() >> b & 1 == 1 {
                    sc.shared += 1;
                }
                for l in layers_of(s) {
                    layers[l][w] = Word::new(layers[l][w].bits() | 1 << b);
                }
                // Distance from the star to the centre of its cell.
                let centre = |i: u32| -half + (2 * i64::from(i) + 1) * half / SIDE;
                let d = [
                    s.pos[0] - centre(c.0),
                    s.pos[1] - centre(c.1),
                    s.pos[2] - centre(c.2),
                ];
                let e = isqrt(d.iter().map(|v| v.unsigned_abs().pow(2)).sum());
                sc.error_sum += e;
                sc.error_max = sc.error_max.max(e);
                if !s.name.is_empty() && s.mag.is_some_and(|m| m < 1500) {
                    sc.names.push((s.name.clone(), c));
                }
            }
            sc.layers = layers;
            sc
        })
        .collect()
}

/// The occupied cells of a scale as points, read back from the Monade itself
/// (bit offset → octal address → point), with one flag bit per further layer.
pub fn points(sc: &Scale, root: u32) -> Vec<[u8; 4]> {
    let mut out = Vec::new();
    for (w, word) in sc.layers[0].iter().enumerate() {
        let mut bits = word.bits();
        while bits != 0 {
            let b = u64::from(bits.trailing_zeros());
            bits &= bits - 1;
            let offset = w as u64 * 64 + b;
            let Ok(a) = Address::from_bit_offset(root, Size::THOUGHT, offset, 8) else {
                continue;
            };
            let (x, y, z) = a.point();
            let mut flags = 0u8;
            for (l, layer) in sc.layers.iter().enumerate().skip(1) {
                if layer[w].bits() >> b & 1 == 1 {
                    flags |= 1 << (l - 1);
                }
            }
            out.push([x as u8, y as u8, z as u8, flags]);
        }
    }
    out
}

/// Occupied cells in each of the 8 octants (the first digit of the address).
pub fn octants(sc: &Scale, root: u32) -> [u64; 8] {
    let mut out = [0u64; 8];
    for (d, o) in out.iter_mut().enumerate() {
        if let Ok(a) = Address::root(root, Size::THOUGHT).and_then(|r| r.down(d as u8)) {
            let from = (a.bit_offset() / 64) as usize;
            let len = (a.bit_len() / 64) as usize;
            *o = word::popcount(&sc.layers[0][from..from + len]);
        }
    }
    out
}

/// Thousandths of a parsec as text: 1234 → "1.234 pc".
fn pc(milli: u64) -> String {
    format!("{}.{:03} pc", milli / 1000, milli % 1000)
}

/// A summary for the terminal.
pub fn summary(stars: &[Star], scales: &[Scale]) -> String {
    let mut out = format!(
        "MONADE · STARS  {} stars read\n\n",
        grouped(stars.len() as u64)
    );
    for (root, sc) in scales.iter().enumerate() {
        let occupied = word::popcount(&sc.layers[0]);
        let cell = (2 * sc.half_pc * 1000 / SIDE) as u64;
        let _ = writeln!(
            out,
            "scale {root}: cube of {} pc, cell {}: {} stars inside, {} cells lit, {} sharing a cell; mean error {}, max {}; naked eye {}",
            2 * sc.half_pc,
            pc(cell),
            grouped(sc.inside),
            grouped(occupied),
            grouped(sc.shared),
            pc(sc.error_sum / sc.inside.max(1)),
            pc(sc.error_max),
            grouped(word::popcount(&sc.layers[1]))
        );
    }
    out
}

/// Standard base64 (for embedding the points in the page).
fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= c.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The map: one self-contained HTML page with a rotating 3D view of every scale.
pub fn html(stars: &[Star], scales: &[Scale], source: &str) -> String {
    let mut data = String::from("[");
    let mut table = String::new();
    for (root, sc) in scales.iter().enumerate() {
        let pts = points(sc, root as u32);
        let flat: Vec<u8> = pts.iter().flatten().copied().collect();
        let names: Vec<String> = sc
            .names
            .iter()
            .map(|(n, (x, y, z))| {
                format!(
                    "[{},{x},{y},{z}]",
                    serde_json::to_string(n).unwrap_or_default()
                )
            })
            .collect();
        let cell = (2 * sc.half_pc * 1000 / SIDE) as u64;
        let _ = write!(
            data,
            "{{\"half\":{},\"cell\":\"{}\",\"n\":{},\"pts\":\"{}\",\"names\":[{}]}},",
            sc.half_pc,
            pc(cell),
            pts.len(),
            base64(&flat),
            names.join(",")
        );
        let oct = octants(sc, root as u32);
        let _ = write!(
            table,
            "<tr><td>{root}</td><td>{} pc</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td class=\"mono\">{}</td></tr>",
            2 * sc.half_pc,
            pc(cell),
            grouped(sc.inside),
            grouped(pts.len() as u64),
            grouped(sc.shared),
            pc(sc.error_sum / sc.inside.max(1)),
            grouped(word::popcount(&sc.layers[1])),
            oct.iter().map(|o| grouped(*o)).collect::<Vec<_>>().join(" · ")
        );
    }
    data.push(']');
    PAGE.replace("{{DATA}}", &data)
        .replace("{{TABLE}}", &table)
        .replace("{{STARS}}", &grouped(stars.len() as u64))
        .replace("{{SOURCE}}", &esc(source))
}

const PAGE: &str = r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Monade · stars</title><style>
:root{--bg:#0b0b0d;--fg:#e6e6e6;--mute:#8a8a8a;--line:#26262a}
*{box-sizing:border-box}
body{margin:0;padding:24px 32px;background:var(--bg);color:var(--fg);font:13px/1.6 system-ui,'Segoe UI',Helvetica,Arial,sans-serif;font-variant-numeric:tabular-nums}
h1{font-size:16px;font-weight:600;margin:0 0 4px;letter-spacing:.04em}
h2{font-size:13px;font-weight:600;margin:24px 0 8px;color:var(--mute);text-transform:uppercase;letter-spacing:.08em}
.meta{color:var(--mute)} .mono{font-family:ui-monospace,Consolas,monospace}
table{border-collapse:collapse} td,th{padding:3px 12px 3px 0;border-bottom:1px solid var(--line);text-align:left;white-space:nowrap}
th{color:var(--mute);font-weight:600}
.bar{display:flex;gap:16px;align-items:center;flex-wrap:wrap;margin:8px 0}
select,button{background:#18181c;color:var(--fg);border:1px solid var(--line);border-radius:4px;padding:3px 8px;font:inherit}
canvas{display:block;background:#000;border:1px solid var(--line);border-radius:4px;cursor:grab;max-width:100%}
.intro{max-width:860px}
.key span{display:inline-block;width:10px;height:10px;border-radius:5px;margin:0 4px 0 10px}
</style></head><body>
<h1>MONADE · STARS</h1>
<div class="meta">{{STARS}} stars read · source: {{SOURCE}}</div>
<p class="intro">Each scale is one Monade of 2 MB: a cube of 256 x 256 x 256 cells around the Sun, one bit per cell. A star lights the bit of its cell; the octal address of the cell (8 base-8 digits) is its place in the cube. The points drawn here are read back from the Monade itself, not from the catalog. The colours and the naked-eye filter are further Monadi with the same addresses: layers of the same map.</p>
<div class="bar">
<label>Scale <select id="scale"></select></label>
<label>Show <select id="layer"><option value="0">every star</option><option value="1">naked eye only</option></select></label>
<label><input type="checkbox" id="spin" checked> rotate</label>
<label><input type="checkbox" id="labels" checked> names</label>
<button id="reset">reset view</button>
<span class="meta" id="info"></span>
</div>
<canvas id="c" width="1000" height="680"></canvas>
<div class="meta key">drag to turn · wheel to zoom ·
<span style="background:#9bb0ff"></span>blue<span style="background:#f8f7ff"></span>white<span style="background:#fff4ea"></span>yellow<span style="background:#ffd2a1"></span>orange<span style="background:#ffb56b"></span>red<span style="background:#777"></span>no colour<span style="background:#ffd700"></span>the Sun</div>
<h2>The four Monadi</h2>
<table><tr><th>scale</th><th>cube</th><th>cell</th><th>stars inside</th><th>cells lit</th><th>sharing a cell</th><th>mean error</th><th>naked eye</th><th>lit cells per octant (first digit 0–7)</th></tr>{{TABLE}}</table>
<p class="meta intro">Star data: <a href="https://github.com/astronexus/HYG-Database" style="color:inherit">HYG Database</a> by David Nash (astronexus.com), licensed under <a href="https://creativecommons.org/licenses/by-sa/4.0/" style="color:inherit">CC BY-SA 4.0</a>. Changed here: positions quantised to cells of a 256³ cube, magnitude and colour reduced to layers of one bit. This map, as adapted material, is shared under the same licence.</p>
<p class="meta intro">Mean error: average distance between a star and the centre of its cell, the price of one bit per cell. Sharing a cell: stars that fell where another star already was (one bit cannot count them; counters will, in step 5). Octants: the 8 parts of the cube, the first digit of every address.</p>
<script>
const S={{DATA}};
const COL=[[155,176,255],[248,247,255],[255,244,234],[255,210,161],[255,181,107]];
const c=document.getElementById('c'),g=c.getContext('2d'),W=c.width,H=c.height;
const sel=document.getElementById('scale'),lay=document.getElementById('layer'),info=document.getElementById('info');
S.forEach((s,i)=>{const o=document.createElement('option');o.value=i;o.textContent=(2*s.half)+' pc (cell '+s.cell+')';sel.appendChild(o);});
sel.value=1;
function dec(b){const s=atob(b),a=new Uint8Array(s.length);for(let i=0;i<s.length;i++)a[i]=s.charCodeAt(i);return a;}
S.forEach(s=>s.a=dec(s.pts));
let yaw=0.6,pitch=0.35,zoom=1,drag=null;
function draw(){
 const s=S[+sel.value],a=s.a,only=+lay.value===1;
 const img=g.createImageData(W,H),d=img.data;
 const cy=Math.cos(yaw),sy=Math.sin(yaw),cp=Math.cos(pitch),sp=Math.sin(pitch);
 const f=zoom*H*1.45,D=640,sz=s.n<5000?2:1;let shown=0;
 function proj(x,y,z){x-=127.5;y-=127.5;z-=127.5;
  const x1=cy*x-sy*y,y1=sy*x+cy*y;const y2=cp*y1-sp*z,z2=sp*y1+cp*z;
  const w=D+y2;if(w<20)return null;return [W/2+f*x1/w,H/2-f*z2/w,w];}
 for(let i=0;i<a.length;i+=4){
  const fl=a[i+3];if(only&&!(fl&1))continue;
  const p=proj(a[i],a[i+1],a[i+2]);if(!p)continue;
  const px=p[0]|0,py=p[1]|0;if(px<0||py<0||px>=W||py>=H)continue;
  let k=-1;for(let b=1;b<6;b++)if(fl&(1<<b)){k=b-1;break;}
  const col=k<0?[119,119,119]:COL[k];const br=(fl&1)?1:0.55;
  for(let dy=0;dy<sz;dy++)for(let dx=0;dx<sz;dx++){if(px+dx>=W||py+dy>=H)continue;
   const o=((py+dy)*W+px+dx)*4;d[o]=Math.min(255,d[o]+col[0]*br);d[o+1]=Math.min(255,d[o+1]+col[1]*br);d[o+2]=Math.min(255,d[o+2]+col[2]*br);d[o+3]=255;}
  shown++;}
 for(let i=3;i<d.length;i+=4)if(!d[i])d[i]=255;
 g.putImageData(img,0,0);
 const sun=proj(128,128,128);
 if(sun){g.strokeStyle='#ffd700';g.beginPath();g.arc(sun[0],sun[1],5,0,7);g.stroke();}
 g.strokeStyle='rgba(255,255,255,0.12)';g.beginPath();
 const E=[[0,0,0],[256,0,0],[256,256,0],[0,256,0]];
 for(const zz of [0,256]){E.forEach((e,j)=>{const p=proj(e[0],e[1],zz),q=proj(E[(j+1)%4][0],E[(j+1)%4][1],zz);if(p&&q){g.moveTo(p[0],p[1]);g.lineTo(q[0],q[1]);}});}
 E.forEach(e=>{const p=proj(e[0],e[1],0),q=proj(e[0],e[1],256);if(p&&q){g.moveTo(p[0],p[1]);g.lineTo(q[0],q[1]);}});
 g.stroke();
 if(document.getElementById('labels').checked){g.fillStyle='#bbb';g.font='11px system-ui,sans-serif';
  s.names.forEach(n=>{const p=proj(n[1],n[2],n[3]);if(p)g.fillText(n[0],p[0]+6,p[1]-4);});}
 info.textContent=shown.toLocaleString()+' cells drawn · cube '+(2*s.half)+' pc · cell '+s.cell;
}
function tick(){if(document.getElementById('spin').checked&&!drag){yaw+=0.004;draw();}requestAnimationFrame(tick);}
c.addEventListener('mousedown',e=>{drag=[e.clientX,e.clientY];c.style.cursor='grabbing';});
addEventListener('mouseup',()=>{drag=null;c.style.cursor='grab';});
addEventListener('mousemove',e=>{if(!drag)return;yaw+=(e.clientX-drag[0])*0.006;pitch=Math.max(-1.5,Math.min(1.5,pitch+(e.clientY-drag[1])*0.006));drag=[e.clientX,e.clientY];draw();});
c.addEventListener('wheel',e=>{e.preventDefault();zoom*=e.deltaY<0?1.1:1/1.1;draw();},{passive:false});
document.getElementById('reset').onclick=()=>{yaw=0.6;pitch=0.35;zoom=1;draw();};
[sel,lay,document.getElementById('labels')].forEach(x=>x.onchange=draw);
draw();tick();
</script></body></html>
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimals_without_floating_point() {
        assert_eq!(parse_milli("219.740502"), Some(219_740));
        assert_eq!(parse_milli("-1.5"), Some(-1500));
        assert_eq!(parse_milli("0.000005"), Some(0));
        assert_eq!(parse_milli("12"), Some(12_000));
        assert_eq!(parse_milli(""), None);
        assert_eq!(parse_milli("1e5"), None);
    }

    #[test]
    fn cells() {
        let half = 16_000;
        assert_eq!(cell([0, 0, 0], half), Some((128, 128, 128)));
        assert_eq!(cell([-16_000, -16_000, -16_000], half), Some((0, 0, 0)));
        assert_eq!(cell([15_999, 0, 0], half), Some((255, 128, 128)));
        assert_eq!(cell([16_000, 0, 0], half), None);
        assert_eq!(cell([-16_001, 0, 0], half), None);
    }

    #[test]
    fn build_and_read_back() {
        let star = |x, mag, ci, name: &str| Star {
            pos: [x, 0, 0],
            mag: Some(mag),
            ci: Some(ci),
            name: name.into(),
        };
        let stars = [
            star(0, 1000, 600, "Sun-like"),
            star(1, 9000, -100, ""),
            star(5_000, 3000, 1600, ""),
            star(2_000_000, 0, 0, "far"),
        ];
        let scales = build(&stars);
        let s0 = &scales[0];
        assert_eq!(s0.inside, 3);
        assert_eq!(s0.shared, 1, "the first two share the centre cell");
        let pts = points(s0, 0);
        assert_eq!(pts.len(), 2);
        assert!(pts.iter().any(|p| p[..3] == [128, 128, 128]));
        assert_eq!(octants(s0, 0).iter().sum::<u64>(), 2);
        assert_eq!(scales[3].inside, 3, "2 000 pc is outside every cube");
        let page = html(&stars, &scales, "test");
        assert!(page.contains("<canvas") && page.contains("Sun-like"));
        assert!(summary(&stars, &scales).contains("4 stars read"));
    }

    #[test]
    fn base64_known_values() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(base64(b""), "");
    }
}
