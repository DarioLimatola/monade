//! A first world: the stars of a catalog, held as octal addresses.
//!
//! **Sparse, exact.** Every star is one Morton code of 63 bits: an octal address of
//! 21 digits in a cube of 16 777 parsecs around the Sun, down to cells of 0.008 pc
//! (a star is at most 0.007 pc from the centre of its cell: exact for this catalog).
//! Only the stars are stored, sorted by code, not the empty space: 8 bytes a star
//! (about 1 MB for 120 000 stars), against 2 MB per Monade per scale before.
//!
//! **Dense, on demand.** A region is a prefix of the address, so its stars are one
//! contiguous run of the sorted codes. Any region can be turned into a Monade of
//! 2 MB (256 × 256 × 256 cells, one bit per cell) by taking the 8 digits that follow
//! its prefix ([`Sky::dense`]): the dense form is for the mind to compare and combine
//! regions (XOR, majority, distance), the sparse form is for keeping them.
//!
//! Integers only: positions are read from the catalog's decimal text as thousandths
//! of a parsec, never as floating point.

use crate::check::grouped;
use monade_core::size::Size;
use monade_core::word::Word;
use std::fmt::{self, Write as _};
use std::ops::Range;
use std::path::{Path, PathBuf};

/// Digits of the address of a star (21 octal digits = 63 bits).
pub const DEPTH: u32 = 21;
/// Side of the finest cell, thousandths of a parsec.
pub const CELL_MILLI: i64 = 8;
/// Half-width of the whole cube, thousandths of a parsec (2^20 cells × 8).
pub const HALF_MILLI: i64 = CELL_MILLI << (DEPTH - 1);
/// Distances the catalog marks as unknown (HYG writes 100 000 pc) are left out.
pub const UNKNOWN_DISTANCE_MILLI: i64 = 99_000_000;

/// One star: position in thousandths of a parsec, magnitudes and colour in thousandths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Star {
    /// x, y, z (equatorial, the Sun at 0), thousandths of a parsec.
    pub pos: [i64; 3],
    /// Apparent magnitude, thousandths (smaller is brighter).
    pub mag: Option<i64>,
    /// Absolute magnitude (as seen from 10 pc), thousandths.
    pub absmag: Option<i64>,
    /// Colour index B-V, thousandths (smaller is bluer).
    pub ci: Option<i64>,
    /// Proper name, if any.
    pub name: String,
    /// Hipparcos number, if any: the same star in two catalogs.
    pub hip: Option<u32>,
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

/// Reads the stars of an HYG-like CSV, plain or gzip-compressed (`.gz`): columns x, y,
/// z, mag, ci, proper; absmag and hip if present. HYG and the HYGLike subset of AT-HYG
/// (Gaia DR3 distances) have the same columns. Stars at the catalog's "unknown
/// distance" are left out.
pub fn read_catalog(path: &Path) -> Result<Vec<Star>, StarsError> {
    let err = |e: String| StarsError::Read(path.to_path_buf(), e);
    let file = std::fs::File::open(path).map_err(|e| err(e.to_string()))?;
    let input: Box<dyn std::io::Read> = if path.extension().is_some_and(|e| e == "gz") {
        Box::new(flate2::read::GzDecoder::new(std::io::BufReader::new(file)))
    } else {
        Box::new(std::io::BufReader::new(file))
    };
    let mut r = csv::Reader::from_reader(input);
    let headers = r.headers().map_err(|e| err(e.to_string()))?.clone();
    let find = |name: &str| headers.iter().position(|h| h == name);
    let col = |name: &'static str| find(name).ok_or(StarsError::Column(name));
    let (x, y, z) = (col("x")?, col("y")?, col("z")?);
    let (mag, ci, proper) = (col("mag")?, col("ci")?, col("proper")?);
    let absmag = find("absmag");
    let hip = find("hip");
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
        if px.abs().max(py.abs()).max(pz.abs()) >= UNKNOWN_DISTANCE_MILLI / 2 {
            continue;
        }
        out.push(Star {
            pos: [px, py, pz],
            mag: parse_milli(get(mag)),
            absmag: absmag.and_then(|i| parse_milli(get(i))),
            ci: parse_milli(get(ci)),
            name: get(proper).to_string(),
            hip: hip.and_then(|i| get(i).trim().parse().ok()),
        });
    }
    Ok(out)
}

/// The cell of a position along one axis (0 .. 2^21), or None outside the cube.
fn axis(p: i64) -> Option<u64> {
    let shifted = p.checked_add(HALF_MILLI)?;
    if shifted < 0 {
        return None;
    }
    let i = shifted / CELL_MILLI;
    u64::try_from(i).ok().filter(|v| *v < 1 << DEPTH)
}

/// Morton code of a cell: bit 3i is bit i of x, 3i + 1 of y, 3i + 2 of z (the
/// convention of the octal address: each digit is an octant, bit 0 → x).
pub fn interleave(x: u64, y: u64, z: u64) -> u64 {
    let mut code = 0;
    for i in 0..DEPTH {
        code |= (x >> i & 1) << (3 * i) | (y >> i & 1) << (3 * i + 1) | (z >> i & 1) << (3 * i + 2);
    }
    code
}

/// The cell of a Morton code (the inverse of [`interleave`]).
pub fn deinterleave(code: u64) -> (u64, u64, u64) {
    let (mut x, mut y, mut z) = (0, 0, 0);
    for i in 0..DEPTH {
        x |= (code >> (3 * i) & 1) << i;
        y |= (code >> (3 * i + 1) & 1) << i;
        z |= (code >> (3 * i + 2) & 1) << i;
    }
    (x, y, z)
}

/// The code of a position, or None outside the cube.
pub fn code_of(pos: [i64; 3]) -> Option<u64> {
    Some(interleave(axis(pos[0])?, axis(pos[1])?, axis(pos[2])?))
}

/// The centre of the cell of a code, thousandths of a parsec.
pub fn centre(code: u64) -> [i64; 3] {
    let (x, y, z) = deinterleave(code);
    let c = |v: u64| v as i64 * CELL_MILLI - HALF_MILLI + CELL_MILLI / 2;
    [c(x), c(y), c(z)]
}

/// What the page needs of a star beyond its place, in 32 bits: colour index in
/// hundredths from -0.5 (9 bits, 511 = unknown), apparent magnitude in quarters from
/// -2 (7 bits, 127 = unknown), absolute magnitude in eighths from -10 (8 bits,
/// 255 = unknown), named (1 bit).
pub fn info(s: &Star) -> u32 {
    let class = |v: Option<i64>, from: i64, step: i64, max: u32| {
        v.map_or(max, |v| {
            ((v - from).max(0) / step).min(i64::from(max) - 1) as u32
        })
    };
    class(s.ci, -500, 10, 511)
        | class(s.mag, -2000, 250, 127) << 9
        | class(s.absmag, -10_000, 125, 255) << 16
        | u32::from(!s.name.is_empty()) << 24
}

/// The sky: every star as a code, sorted, with its info.
pub struct Sky {
    /// Morton codes, sorted.
    pub codes: Vec<u64>,
    /// Info of each star ([`info`]), same order.
    pub info: Vec<u32>,
    /// Names of the named stars: (index into codes, name).
    pub names: Vec<(usize, String)>,
    /// Stars read but outside the cube.
    pub outside: u64,
    /// Stars sharing their cell with another one.
    pub shared: u64,
    /// Largest distance from a star to the centre of its cell, thousandths of a parsec.
    pub error_max: u64,
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

impl Sky {
    /// Builds the sky from the stars.
    pub fn build(stars: &[Star]) -> Sky {
        let mut rows: Vec<(u64, u32, &str)> = Vec::with_capacity(stars.len());
        let (mut outside, mut error_max) = (0, 0);
        for s in stars {
            let Some(code) = code_of(s.pos) else {
                outside += 1;
                continue;
            };
            let c = centre(code);
            let e = isqrt(
                (0..3)
                    .map(|i| (s.pos[i] - c[i]).unsigned_abs().pow(2))
                    .sum(),
            );
            error_max = error_max.max(e);
            rows.push((code, info(s), s.name.as_str()));
        }
        rows.sort_unstable_by_key(|r| r.0);
        let shared = rows.windows(2).filter(|w| w[0].0 == w[1].0).count() as u64;
        Sky {
            names: rows
                .iter()
                .enumerate()
                .filter(|(_, r)| !r.2.is_empty())
                .map(|(i, r)| (i, r.2.to_string()))
                .collect(),
            codes: rows.iter().map(|r| r.0).collect(),
            info: rows.iter().map(|r| r.1).collect(),
            outside,
            shared,
            error_max,
        }
    }

    /// The stars of a region: the cube of the address `prefix` with `depth` digits
    /// (0 = the whole sky). One contiguous run of the sorted codes.
    pub fn region(&self, prefix: u64, depth: u32) -> Range<usize> {
        let shift = 3 * (DEPTH - depth.min(DEPTH));
        let lo = prefix << shift;
        let hi = (prefix + 1) << shift;
        self.codes.partition_point(|c| *c < lo)..self.codes.partition_point(|c| *c < hi)
    }

    /// A region as one dense Monade of 2 MB: 256 × 256 × 256 cells, one bit per cell,
    /// the bit offset being the 8 digits after the prefix. `depth` at most 13.
    pub fn dense(&self, prefix: u64, depth: u32) -> Vec<Word> {
        let depth = depth.min(DEPTH - 8);
        let mut out = vec![Word::ZERO; Size::THOUGHT.words()];
        let shift = 3 * (DEPTH - depth - 8);
        for &code in &self.codes[self.region(prefix, depth)] {
            let offset = (code >> shift) & ((1 << 24) - 1);
            let (w, b) = ((offset / 64) as usize, offset % 64);
            out[w] = Word::new(out[w].bits() | 1 << b);
        }
        out
    }

    /// The 8 regions of `depth` digits (at least 1) that meet at the Sun. The Sun is the
    /// centre of the whole cube, and the centre of a cube is the corner of its eight
    /// parts: around the Sun there are always eight regions, never one.
    pub fn around_sun(depth: u32) -> [u64; 8] {
        let depth = depth.clamp(1, DEPTH);
        let s = DEPTH - depth;
        let sun = 1u64 << (DEPTH - 1); // the cell of the Sun on each axis
        let mut out = [0; 8];
        for (o, p) in out.iter_mut().enumerate() {
            let a = |bit: usize| (sun >> s) - u64::from(o >> bit & 1 == 0);
            *p = interleave(a(0), a(1), a(2));
        }
        out
    }
}

/// The agreement of two catalogs (two voices) on the stars they share, matched by
/// Hipparcos number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agreement {
    /// Stars found in both.
    pub matched: u64,
    /// Median distance between the two places of a star, thousandths of a parsec.
    pub median: u64,
    /// 90th percentile of that distance.
    pub p90: u64,
    /// Stars whose distances from the Sun differ by more than 10 %, per mille.
    pub far_permille: u64,
}

/// Compares the places of the stars that `a` and `b` share.
pub fn agree(a: &[Star], b: &[Star]) -> Agreement {
    let other: std::collections::HashMap<u32, [i64; 3]> =
        b.iter().filter_map(|s| Some((s.hip?, s.pos))).collect();
    let mut gaps = Vec::new();
    let mut far = 0u64;
    for s in a {
        let Some(p) = s.hip.and_then(|h| other.get(&h)) else {
            continue;
        };
        let sq = |v: [i64; 3]| v.iter().map(|x| x.unsigned_abs().pow(2)).sum::<u64>();
        gaps.push(isqrt(sq([
            s.pos[0] - p[0],
            s.pos[1] - p[1],
            s.pos[2] - p[2],
        ])));
        let (ra, rb) = (isqrt(sq(s.pos)), isqrt(sq(*p)));
        if ra.abs_diff(rb) * 10 > rb {
            far += 1;
        }
    }
    gaps.sort_unstable();
    let at = |q: usize| {
        gaps.get(gaps.len().saturating_sub(1) * q / 100)
            .copied()
            .unwrap_or(0)
    };
    Agreement {
        matched: gaps.len() as u64,
        median: at(50),
        p90: at(90),
        far_permille: if gaps.is_empty() {
            0
        } else {
            far * 1000 / gaps.len() as u64
        },
    }
}

/// The first bytes of a sky file.
const MAGIC: &[u8; 8] = b"MONSKY01";

impl Sky {
    /// Writes the sky (the sparse Monade): MAGIC, count, codes, info (little-endian),
    /// then one line per name: index, tab, name.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let mut out = Vec::with_capacity(16 + 12 * self.codes.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&(self.codes.len() as u64).to_le_bytes());
        self.codes
            .iter()
            .for_each(|c| out.extend_from_slice(&c.to_le_bytes()));
        self.info
            .iter()
            .for_each(|i| out.extend_from_slice(&i.to_le_bytes()));
        for (i, n) in &self.names {
            out.extend_from_slice(format!("{i}\t{}\n", n.replace(['\t', '\n'], " ")).as_bytes());
        }
        std::fs::write(path, out)
    }

    /// Reads a sky written by [`Sky::save`].
    pub fn load(path: &Path) -> std::io::Result<Sky> {
        let bad = || std::io::Error::new(std::io::ErrorKind::InvalidData, "not a sky file");
        let b = std::fs::read(path)?;
        if b.len() < 16 || &b[..8] != MAGIC {
            return Err(bad());
        }
        let n = u64::from_le_bytes(b[8..16].try_into().map_err(|_| bad())?) as usize;
        let end = n
            .checked_mul(12)
            .and_then(|x| x.checked_add(16))
            .ok_or_else(bad)?;
        if b.len() < end {
            return Err(bad());
        }
        let codes = b[16..16 + 8 * n]
            .chunks_exact(8)
            .map(|c| u64::from_le_bytes(c.try_into().unwrap_or([0; 8])))
            .collect();
        let info = b[16 + 8 * n..end]
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap_or([0; 4])))
            .collect();
        let names = String::from_utf8_lossy(&b[end..])
            .lines()
            .filter_map(|l| {
                let (i, n) = l.split_once('\t')?;
                Some((i.parse().ok()?, n.to_string()))
            })
            .collect();
        Ok(Sky {
            codes,
            info,
            names,
            outside: 0,
            shared: 0,
            error_max: 0,
        })
    }

    /// The 8 children of a region: for each octant, the number of stars.
    pub fn children(&self, prefix: u64, depth: u32) -> [u64; 8] {
        let mut out = [0; 8];
        if depth < DEPTH {
            for (o, n) in out.iter_mut().enumerate() {
                *n = self.region(prefix << 3 | o as u64, depth + 1).len() as u64;
            }
        }
        out
    }
}

/// Thousandths of a parsec as text: 1234 → "1.234 pc".
pub(crate) fn pc(milli: u64) -> String {
    format!("{}.{:03} pc", milli / 1000, milli % 1000)
}

/// Side of a region of `depth` digits, thousandths of a parsec.
pub fn side_milli(depth: u32) -> u64 {
    (CELL_MILLI as u64) << (DEPTH - depth)
}

/// A summary for the terminal.
pub fn summary(stars: &[Star], sky: &Sky) -> String {
    let mut out = format!(
        "MONADE · STARS  {} stars read, {} kept as codes ({} bytes), {} outside the cube, {} sharing a cell; \
         cube {}, cell {}, largest error {}\n\nDense Monadi of 2 MB near the Sun (any region can be one):\n",
        grouped(stars.len() as u64),
        grouped(sky.codes.len() as u64),
        grouped(8 * sky.codes.len() as u64),
        grouped(sky.outside),
        grouped(sky.shared),
        pc(side_milli(0)),
        pc(CELL_MILLI as u64),
        pc(sky.error_max),
    );
    for depth in [5, 7, 9, 11, 13] {
        let (mut n, mut lit) = (0, 0);
        for p in Sky::around_sun(depth) {
            n += sky.region(p, depth).len() as u64;
            lit += monade_core::word::popcount(&sky.dense(p, depth));
        }
        let _ = writeln!(
            out,
            "  depth {depth:>2}: 8 regions of {} meeting at the Sun (a cube of {}), cell {}: {} stars, {} cells lit",
            pc(side_milli(depth)),
            pc(2 * side_milli(depth)),
            pc(side_milli(depth + 8)),
            grouped(n),
            grouped(lit),
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

/// The map: one self-contained HTML page (3D view at any radius, and the
/// Hertzsprung–Russell diagram), the stars decoded from their codes.
pub fn html(stars: &[Star], sky: &Sky, source: &str) -> String {
    let codes: Vec<u8> = sky.codes.iter().flat_map(|c| c.to_le_bytes()).collect();
    let info: Vec<u8> = sky.info.iter().flat_map(|i| i.to_le_bytes()).collect();
    let names: Vec<String> = sky
        .names
        .iter()
        .map(|(i, n)| format!("[{i},{}]", serde_json::to_string(n).unwrap_or_default()))
        .collect();
    PAGE.replace("{{CODES}}", &base64(&codes))
        .replace("{{INFO}}", &base64(&info))
        .replace("{{NAMES}}", &format!("[{}]", names.join(",")))
        .replace("{{STARS}}", &grouped(stars.len() as u64))
        .replace("{{KEPT}}", &grouped(sky.codes.len() as u64))
        .replace("{{BYTES}}", &grouped(8 * sky.codes.len() as u64))
        .replace("{{OUTSIDE}}", &grouped(sky.outside))
        .replace("{{ERROR}}", &pc(sky.error_max))
        .replace("{{SOURCE}}", &esc(source))
}

const PAGE: &str = include_str!("stars.html");

#[cfg(test)]
mod tests {
    use super::*;
    use monade_core::address::Address;

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
    fn codes_are_octal_addresses() {
        // The low 24 bits of a code are the address of a cell in a 2 MB Monade.
        for (x, y, z) in [(0, 0, 0), (255, 0, 7), (13, 200, 99), (255, 255, 255)] {
            let a = Address::from_point(0, Size::THOUGHT, (x, y, z), 8).unwrap();
            assert_eq!(
                interleave(u64::from(x), u64::from(y), u64::from(z)),
                a.bit_offset()
            );
        }
        for code in [
            0,
            1,
            0o777,
            0x7fff_ffff_ffff_ffff,
            0x1234_5678_9abc_def0 >> 1,
        ] {
            let (x, y, z) = deinterleave(code);
            assert_eq!(interleave(x, y, z), code);
        }
    }

    #[test]
    fn positions_are_exact() {
        assert_eq!(
            code_of([0, 0, 0]).map(deinterleave),
            Some((1 << 20, 1 << 20, 1 << 20))
        );
        assert_eq!(code_of([HALF_MILLI, 0, 0]), None);
        assert_eq!(
            code_of([-HALF_MILLI, 0, 0]).map(|c| deinterleave(c).0),
            Some(0)
        );
        let p = [1_302_000, -2_047, 7_999_999];
        let c = centre(code_of(p).unwrap());
        assert!((0..3).all(|i| (p[i] - c[i]).abs() <= CELL_MILLI / 2));
    }

    #[test]
    fn regions_and_dense_monadi() {
        let star = |pos, mag, ci, name: &str| Star {
            pos,
            mag: Some(mag),
            absmag: None,
            ci: Some(ci),
            name: name.into(),
            hip: None,
        };
        let stars = [
            star([1_000, 1_000, 1_000], 1000, 600, "a"),
            star([1_001, 1_001, 1_001], 9000, -100, ""),
            star([-5_000, 2_000, -300], 3000, 1600, ""),
            star([4_000_000, 0, 0], 0, 0, "far"),
            star([90_000_000, 0, 0], 0, 0, "outside"),
        ];
        let sky = Sky::build(&stars);
        assert_eq!((sky.codes.len(), sky.outside, sky.shared), (4, 1, 1));
        assert!(sky.codes.windows(2).all(|w| w[0] <= w[1]));
        assert!(sky.error_max <= 7);
        assert_eq!(sky.region(0, 0), 0..4);
        // The 8 regions meeting at the Sun hold every star within half their side.
        for depth in [1, 5, 9, 13] {
            let n: usize = Sky::around_sun(depth)
                .iter()
                .map(|p| sky.region(*p, depth).len())
                .sum();
            let half = side_milli(depth) as i64;
            let inside = stars
                .iter()
                .filter(|s| s.pos.iter().all(|v| -half <= *v && *v < half))
                .count();
            assert_eq!(n, inside, "depth {depth}");
        }
        // A dense Monade lights one cell per distinct star cell of its region.
        let lit: u64 = Sky::around_sun(5)
            .iter()
            .map(|p| monade_core::word::popcount(&sky.dense(*p, 5)))
            .sum();
        assert_eq!(lit, 2, "the two close stars share a cell at this depth");
        assert_eq!(info(&stars[0]) & 511, 110); // B-V 0.6 → 110 hundredths from -0.5
        let page = html(&stars, &sky, "test");
        assert!(page.contains("<canvas") && page.contains("\"a\""));
        assert!(summary(&stars, &sky).contains("5 stars read"));
    }

    #[test]
    fn two_voices_and_the_sky_file() {
        let star = |x: i64, hip| Star {
            pos: [x, 0, 0],
            mag: None,
            absmag: None,
            ci: None,
            name: "n".into(),
            hip,
        };
        let a = [star(10_000, Some(1)), star(20_000, Some(2)), star(5, None)];
        let b = [
            star(10_500, Some(1)),
            star(30_000, Some(2)),
            star(7, Some(9)),
        ];
        let g = agree(&a, &b);
        assert_eq!((g.matched, g.median, g.p90), (2, 500, 500));
        assert_eq!(g.far_permille, 500, "star 2: 20 against 30 pc");
        let sky = Sky::build(&a);
        let p = std::env::temp_dir().join(format!("monade-sky-{}.sky", std::process::id()));
        sky.save(&p).unwrap();
        let back = Sky::load(&p).unwrap();
        assert_eq!(
            (back.codes, back.info, back.names),
            (sky.codes.clone(), sky.info.clone(), sky.names.clone())
        );
        std::fs::write(&p, b"nonsense").unwrap();
        assert!(Sky::load(&p).is_err());
        let _ = std::fs::remove_file(&p);
        assert_eq!(sky.children(0, 0).iter().sum::<u64>(), 3);
    }

    #[test]
    fn base64_known_values() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(base64(b""), "");
    }
}
