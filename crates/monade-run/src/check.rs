//! `monade-run check`: verifies the Monade and says, in words and numbers, what it verified.
//!
//! Every check uses the same fixed stream of pseudo-random inputs, so two runs on
//! two machines see the same inputs and must give the same numbers. Integers only.

use monade_core::address::Address;
use monade_core::oracle;
use monade_core::size::{Arena, Size};
use monade_core::space::Space;
use monade_core::specs::{Specs, SEED_KEY};
use monade_core::tally::Tally;
use monade_core::word::{self, Word};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;

use crate::store;

/// Seed of the input stream of `check` (not the seed of the mind's space).
const INPUTS_SEED: u64 = 0x4d4f_4e41_4445_c4ec;

/// State hash of the first 4096 input words: the same on every machine, forever.
const GOLDEN_INPUTS: u64 = 0xa028_f446_97b8_19db;

/// State hash of a fixed bundle of 8 blocks: the same on every machine, forever.
const GOLDEN_BUNDLE: u64 = 0xb055_ea70_f353_1b9b;

/// One number measured by a check.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Measure {
    /// What the number is.
    pub name: String,
    /// The number, as readable text.
    pub value: String,
}

/// A histogram: counts of values in bins of equal width, one series per kind.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Histogram {
    /// What is counted.
    pub title: String,
    /// Unit of the values (e.g. "bits of difference").
    pub unit: String,
    /// Lowest value of the first bin.
    pub start: u64,
    /// Width of every bin.
    pub width: u64,
    /// One series per kind of value.
    pub series: Vec<Series>,
    /// Names of the bins, when they are categories (e.g. N = 1, 2, 4 ...) rather
    /// than equal ranges of values; then the series are drawn side by side.
    #[serde(default)]
    pub labels: Vec<String>,
}

/// One series of a histogram.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Series {
    /// What the series is.
    pub name: String,
    /// Count per bin; values outside go to the first or last bin.
    pub counts: Vec<u64>,
}

/// Counts `values` in `bins` bins of `width` from `start` (outside values clamp).
fn histogram(values: &[u64], start: u64, width: u64, bins: usize) -> Vec<u64> {
    let mut counts = vec![0u64; bins];
    for v in values {
        let i = (v.saturating_sub(start) / width.max(1)) as usize;
        if let Some(c) = counts.get_mut(i.min(bins.saturating_sub(1))) {
            *c += 1;
        }
    }
    counts
}

/// The outcome of one check.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Stable identifier, e.g. `word.xor.laws`.
    pub id: String,
    /// Area: word, space, memory, size, determinism, foundations.
    pub area: String,
    /// What is claimed, in one sentence.
    pub claim: String,
    /// What was measured, in one sentence.
    pub result: String,
    /// What was expected.
    pub expected: String,
    /// Whether the claim held.
    pub passed: bool,
    /// Time taken, in microseconds.
    pub micros: u64,
    /// The numbers behind the result.
    pub measures: Vec<Measure>,
    /// Why this matters for the mind (plain words).
    #[serde(default)]
    pub why: String,
    /// How the check works.
    #[serde(default)]
    pub how: String,
    /// Distributions behind the numbers.
    #[serde(default)]
    pub charts: Vec<Histogram>,
}

/// The input stream: SplitMix64, a small, well-known generator.
pub struct Inputs(u64);

impl Inputs {
    /// The fixed stream of `check`.
    pub fn new() -> Inputs {
        Inputs(INPUTS_SEED)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn word(&mut self) -> Word {
        Word::new(self.next())
    }

    fn monade(&mut self, size: Size) -> Vec<Word> {
        (0..size.words()).map(|_| self.word()).collect()
    }

    /// A noise mask: each bit set with probability `per256 / 256` (one random
    /// byte per bit, compared with `per256`).
    fn noise(&mut self, size: Size, per256: u64) -> Vec<Word> {
        (0..size.words())
            .map(|_| {
                let mut w = 0u64;
                for chunk in 0..8 {
                    let r = self.next();
                    for i in 0..8 {
                        if (r >> (8 * i)) & 0xff < per256 {
                            w |= 1 << (8 * chunk + i);
                        }
                    }
                }
                Word::new(w)
            })
            .collect()
    }

    /// `v` with each bit flipped with probability `per256 / 256`.
    fn noisy(&mut self, v: &[Word], per256: u64) -> Vec<Word> {
        let size = Size::from_bits(64 * v.len() as u64).unwrap_or(Size::WORD);
        let mask = self.noise(size, per256);
        v.iter().zip(mask).map(|(a, m)| a.xor(m)).collect()
    }
}

/// Runs every check, in order, on the foundations in `dir`.
pub fn all(dir: &Path) -> Vec<Outcome> {
    let mut inputs = Inputs::new();
    let checks: [fn(&mut Inputs, &Path) -> Draft; 15] = [
        determinism,
        xor_laws,
        fast_equals_slow,
        random_distance,
        bundle_recall,
        tie_gives_zero,
        sizes,
        arenas,
        octal_address,
        generated_space,
        sentence,
        reinforcement,
        capacity,
        residue,
        foundations,
    ];
    checks
        .iter()
        .map(|check| {
            let start = Instant::now();
            let d = check(&mut inputs, dir);
            let micros = u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX);
            d.finish(micros)
        })
        .collect()
}

/// An outcome being built.
struct Draft {
    id: &'static str,
    area: &'static str,
    claim: String,
    result: String,
    expected: String,
    passed: bool,
    measures: Vec<Measure>,
    why: String,
    how: String,
    charts: Vec<Histogram>,
}

impl Draft {
    fn new(id: &'static str, area: &'static str, claim: &str) -> Draft {
        Draft {
            id,
            area,
            claim: claim.into(),
            result: String::new(),
            expected: String::new(),
            passed: true,
            measures: Vec::new(),
            why: String::new(),
            how: String::new(),
            charts: Vec::new(),
        }
    }

    fn explain(&mut self, why: &str, how: &str) {
        self.why = why.into();
        self.how = how.into();
    }

    fn measure(&mut self, name: &str, value: impl ToString) {
        self.measures.push(Measure {
            name: name.into(),
            value: value.to_string(),
        });
    }

    fn finish(self, micros: u64) -> Outcome {
        Outcome {
            id: self.id.into(),
            area: self.area.into(),
            claim: self.claim,
            result: self.result,
            expected: self.expected,
            passed: self.passed,
            micros,
            measures: self.measures,
            why: self.why,
            how: self.how,
            charts: self.charts,
        }
    }
}

/// A number with thousands separated by spaces: 10 000.
pub fn grouped(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// Tenths as text: 2563 → "256.3".
fn tenths(t: u64) -> String {
    format!("{}.{}", t / 10, t % 10)
}

/// Integer square root (floor).
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

fn determinism(inputs: &mut Inputs, _: &Path) -> Draft {
    let mut d = Draft::new(
        "determinism.golden",
        "determinism",
        "The same inputs give the same state hash on every machine, forever.",
    );
    d.explain(
        "The mind must be reproducible: the same experience must give the same mind on any computer, so that every result can be checked by someone else and every change can be traced. If two machines disagreed here, no later test could be trusted.",
        "A fixed stream of 4096 pseudo-random words and a bundle of 8 blocks are reduced to a 64-bit fingerprint (FNV-1a state hash). The two fingerprints are compared with values written in the code when the check was created; they must never change.",
    );
    let mut fresh = Inputs::new();
    let stream: Vec<Word> = (0..4096).map(|_| fresh.word()).collect();
    let h1 = oracle::state_hash(&stream);
    let blocks: Vec<Vec<Word>> = (0..8).map(|_| fresh.monade(Size::BLOCK)).collect();
    let refs: Vec<&[Word]> = blocks.iter().map(Vec::as_slice).collect();
    let mut bundle = vec![Word::ZERO; Size::BLOCK.words()];
    let ok = word::majority(&refs, &mut bundle).is_ok();
    let h2 = oracle::state_hash(&bundle);
    d.passed = ok && h1 == GOLDEN_INPUTS && h2 == GOLDEN_BUNDLE;
    d.result = format!("inputs hash {h1:#018x}, bundle hash {h2:#018x}");
    d.expected = format!("{GOLDEN_INPUTS:#018x} and {GOLDEN_BUNDLE:#018x}");
    d.measure("inputs state hash", format!("{h1:#018x}"));
    d.measure("bundle state hash", format!("{h2:#018x}"));
    let _ = inputs;
    d
}

fn xor_laws(inputs: &mut Inputs, _: &Path) -> Draft {
    let mut d = Draft::new(
        "word.xor.laws",
        "word",
        "XOR is undone by itself, has 0 as neutral, commutes, and a XOR a gives point 0.",
    );
    d.explain(
        "XOR is binding: it ties a role to a filler (subject = dog) and unties it again. These four laws are what makes binding reversible, order-free and anchored to point 0.",
        "10 000 pairs of random 64-bit words; each law is tested on every pair and the pairs where it holds are counted.",
    );
    let n = 10_000u64;
    let (mut inverse, mut neutral, mut commutes, mut to_zero) = (0u64, 0u64, 0u64, 0u64);
    for _ in 0..n {
        let (a, b) = (inputs.word(), inputs.word());
        inverse += u64::from(a.xor(b).xor(b) == a);
        neutral += u64::from(a.xor(Word::ZERO) == a);
        commutes += u64::from(a.xor(b) == b.xor(a));
        to_zero += u64::from(a.xor(a) == Word::ZERO);
    }
    d.passed = [inverse, neutral, commutes, to_zero]
        .iter()
        .all(|c| *c == n);
    d.result = format!(
        "{} random pairs: (a^b)^b = a in {}, a^0 = a in {}, a^b = b^a in {}, a^a = 0 in {}",
        grouped(n),
        grouped(inverse),
        grouped(neutral),
        grouped(commutes),
        grouped(to_zero)
    );
    d.expected = format!("all {}", grouped(n));
    d.measure("pairs", n);
    d.measure("inverse holds", inverse);
    d.measure("neutral holds", neutral);
    d.measure("commutes", commutes);
    d.measure("self gives zero", to_zero);
    d
}

fn fast_equals_slow(inputs: &mut Inputs, _: &Path) -> Draft {
    let mut d = Draft::new(
        "word.fast_equals_slow",
        "word",
        "Every fast operation gives the same answer as its slow, bit-by-bit reference.",
    );
    d.explain(
        "Fast code is clever and can hide mistakes; slow code is obvious. Every fast kernel of the Monade has a slow twin (the oracle) that reads one bit at a time, and the two must always agree.",
        "2 000 random word pairs through XOR, AND, popcount and Hamming distance, and 200 bundles of 3 to 8 blocks through the majority: fast and slow answers are compared and every difference is counted.",
    );
    let n = 2_000u64;
    let mut wrong = 0u64;
    for _ in 0..n {
        let (a, b) = (inputs.word(), inputs.word());
        wrong += u64::from(a.xor(b) != oracle::xor_word(a, b));
        wrong += u64::from(a.and(b) != oracle::and_word(a, b));
        wrong += u64::from(a.popcount() != oracle::popcount_word(a));
        wrong += u64::from(a.hamming(b) != oracle::hamming_word(a, b));
    }
    let trials = 200u64;
    let mut bundles = 0u64;
    for t in 0..trials {
        let k = 3 + (t % 6) as usize;
        let vs: Vec<Vec<Word>> = (0..k).map(|_| inputs.monade(Size::BLOCK)).collect();
        let refs: Vec<&[Word]> = vs.iter().map(Vec::as_slice).collect();
        let mut fast = vec![Word::ZERO; Size::BLOCK.words()];
        if word::majority(&refs, &mut fast).is_err() || fast != oracle::majority(&refs) {
            wrong += 1;
        }
        bundles += 1;
    }
    d.passed = wrong == 0;
    d.result = format!(
        "{} word pairs x 4 operations and {} bundles of 3 to 8 blocks: {} differences",
        grouped(n),
        grouped(bundles),
        wrong
    );
    d.expected = "0 differences".into();
    d.measure("word pairs", n);
    d.measure("bundles", bundles);
    d.measure("differences", wrong);
    d
}

fn random_distance(inputs: &mut Inputs, _: &Path) -> Draft {
    let mut d = Draft::new(
        "space.random_distance",
        "space",
        "Random vectors are almost orthogonal: they differ on about half their bits, never much less.",
    );
    d.explain(
        "The whole memory relies on this: two unrelated ideas look like strangers (about half their bits differ), so a small distance always means a real resemblance and never a coincidence. The larger the vector, the narrower the spread.",
        "1 000 pairs of random vectors of 512 and of 4096 bits; for each pair the number of differing bits (Hamming distance) is counted. The mean must sit at half the bits and no pair may come closer than half minus 6 standard deviations.",
    );
    let pairs = 1_000u64;
    let mut parts = Vec::new();
    for size in [Size::BLOCK, Size::SYMBOL] {
        let bits = size.bits();
        let (mut sum, mut min, mut max) = (0u64, u64::MAX, 0u64);
        let mut values = Vec::new();
        for _ in 0..pairs {
            let (a, b) = (inputs.monade(size), inputs.monade(size));
            let h = word::hamming(&a, &b).unwrap_or(0);
            values.push(h);
            sum += h;
            min = min.min(h);
            max = max.max(h);
        }
        // Mean within 4 standard errors of bits/2: (sum - pairs*bits/2)^2 <= 16 * pairs * bits/4.
        let centre = pairs * bits / 2;
        let dev = sum.abs_diff(centre);
        let mean_ok = dev * dev <= 4 * pairs * bits;
        // No pair closer than bits/2 - 6 sigma, sigma = sqrt(bits)/2.
        let floor = bits / 2 - 3 * isqrt(bits);
        let floor_ok = min > floor;
        d.passed &= mean_ok && floor_ok;
        let mean_tenths = sum * 10 / pairs;
        let width = isqrt(bits) / 4;
        let start = bits / 2 - 20 * width;
        d.charts.push(Histogram {
            title: format!(
                "Distance between two random {bits}-bit vectors ({} pairs)",
                grouped(pairs)
            ),
            unit: "bits of difference".into(),
            start,
            width,
            series: vec![Series {
                name: "random pairs".into(),
                counts: histogram(&values, start, width, 40),
            }],
            labels: Vec::new(),
        });
        parts.push(format!(
            "{bits} bits: mean {} (expected {}), closest {min}, farthest {max}",
            tenths(mean_tenths),
            bits / 2
        ));
        d.measure(&format!("{bits} bits mean distance"), tenths(mean_tenths));
        d.measure(&format!("{bits} bits closest pair"), min);
        d.measure(&format!("{bits} bits farthest pair"), max);
        d.measure(&format!("{bits} bits floor (half - 6 sigma)"), floor);
    }
    d.result = format!("{} pairs per size; {}", grouped(pairs), parts.join("; "));
    d.expected =
        "mean = half the bits (within 4 standard errors); closest pair above half - 6 sigma".into();
    d
}

fn bundle_recall(inputs: &mut Inputs, _: &Path) -> Draft {
    let mut d = Draft::new(
        "memory.bundle_recall",
        "memory",
        "A bundle (majority) of k blocks stays close to each of them: each is found again among 100 random decoys.",
    );
    d.explain(
        "This is how the blackboard holds several voices in one block: their majority stays recognisably close to each voice. It is memory by superposition, without any training. 8 voices in 512 bits is the size decided for the blackboard.",
        "In each of 200 trials, k random 512-bit blocks are bundled by bit-wise majority. Each member must be closer to the bundle than all of 100 random decoys. The charts show the two distributions of distances: members on the left, decoys on the right; the gap between them is the margin of safety.",
    );
    let trials = 200u64;
    let mut parts = Vec::new();
    for (k, threshold_per_mille) in [(3usize, 1000u64), (8, 990)] {
        let (mut found, mut tried) = (0u64, 0u64);
        let (mut member_sum, mut decoy_sum, mut decoys) = (0u64, 0u64, 0u64);
        let (mut member_values, mut decoy_values) = (Vec::new(), Vec::new());
        for _ in 0..trials {
            let members: Vec<Vec<Word>> = (0..k).map(|_| inputs.monade(Size::BLOCK)).collect();
            let refs: Vec<&[Word]> = members.iter().map(Vec::as_slice).collect();
            let mut bundle = vec![Word::ZERO; Size::BLOCK.words()];
            if word::majority(&refs, &mut bundle).is_err() {
                d.passed = false;
                continue;
            }
            let mut nearest_decoy = u64::MAX;
            for _ in 0..100 {
                let decoy = inputs.monade(Size::BLOCK);
                let h = word::hamming(&bundle, &decoy).unwrap_or(0);
                decoy_values.push(h);
                decoy_sum += h;
                decoys += 1;
                nearest_decoy = nearest_decoy.min(h);
            }
            for m in &members {
                let h = word::hamming(&bundle, m).unwrap_or(u64::MAX);
                member_values.push(h);
                member_sum += h;
                tried += 1;
                found += u64::from(h < nearest_decoy);
            }
        }
        d.charts.push(Histogram {
            title: format!("Bundle of {k} voices: distance to its members and to random decoys"),
            unit: "bits of difference (of 512)".into(),
            start: 96,
            width: 4,
            series: vec![
                Series {
                    name: format!("members ({})", grouped(member_values.len() as u64)),
                    counts: histogram(&member_values, 96, 4, 52),
                },
                Series {
                    name: format!("decoys ({})", grouped(decoy_values.len() as u64)),
                    counts: histogram(&decoy_values, 96, 4, 52),
                },
            ],
            labels: Vec::new(),
        });
        let per_mille = found * 1000 / tried.max(1);
        d.passed &= per_mille >= threshold_per_mille;
        let member_mean = member_sum * 10 / tried.max(1);
        let decoy_mean = decoy_sum * 10 / decoys.max(1);
        parts.push(format!(
            "{k} voices: found {}/{} ({}%), distance to members {} vs decoys {}",
            grouped(found),
            grouped(tried),
            tenths(per_mille),
            tenths(member_mean),
            tenths(decoy_mean)
        ));
        d.measure(&format!("{k} voices found"), format!("{found}/{tried}"));
        d.measure(
            &format!("{k} voices mean distance to members"),
            tenths(member_mean),
        );
        d.measure(
            &format!("{k} voices mean distance to decoys"),
            tenths(decoy_mean),
        );
    }
    d.result = format!(
        "512-bit blocks, {} trials each; {}",
        grouped(trials),
        parts.join("; ")
    );
    d.expected = "3 voices: 100% found (distance about 128); 8 voices: at least 99% found (distance about 186); decoys about 256".into();
    d
}

fn tie_gives_zero(inputs: &mut Inputs, _: &Path) -> Draft {
    let mut d = Draft::new(
        "memory.tie_gives_zero",
        "memory",
        "When the votes are tied, the result is 0: back to point 0, suspension of judgement.",
    );
    d.explain(
        "A tie must not be decided by chance: the Monade suspends judgement and returns to point 0. This keeps the vote deterministic and gives the minority (the residue) a defined place.",
        "1 000 random blocks vote against their exact opposite (every bit tied 1 to 1); the result must be all zeros every time.",
    );
    let n = 1_000u64;
    let mut zero = 0u64;
    for _ in 0..n {
        let a = inputs.monade(Size::BLOCK);
        let not_a: Vec<Word> = a.iter().map(|w| Word::new(!w.bits())).collect();
        let mut out = vec![Word::new(1); Size::BLOCK.words()];
        let ok = word::majority(&[&a, &not_a], &mut out).is_ok();
        zero += u64::from(ok && word::popcount(&out) == 0);
    }
    d.passed = zero == n;
    d.result = format!(
        "{} blocks voted against their opposite: result 0 in {}",
        grouped(n),
        grouped(zero)
    );
    d.expected = format!("all {}", grouped(n));
    d.measure("tied votes", n);
    d.measure("result zero", zero);
    d
}

fn sizes(_: &mut Inputs, _: &Path) -> Draft {
    let mut d = Draft::new(
        "size.ladder",
        "size",
        "There are exactly 7 sizes, from 64 bits to 2 MB, each 8 times the one below; any other size is refused.",
    );
    d.explain(
        "One recursive rule of size at every scale: a Monade is 8 of the size below. Refusing every other size keeps every operation the same at every level.",
        "Each of the 7 sizes is compared with its recursive definition in the oracle and converted to bits and back; 6 wrong sizes and level 7 must be refused.",
    );
    let mut ladder = Vec::new();
    let mut ok = 0u64;
    for level in 0..=6u8 {
        let Ok(s) = Size::new(level) else {
            d.passed = false;
            continue;
        };
        let good = s.words() == oracle::size_words(s)
            && s.bits() == 1u64 << (3 * (u64::from(level) + 2))
            && Size::from_bits(s.bits()).ok() == Some(s)
            && oracle::size_from_bits(s.bits()) == Some(s);
        ok += u64::from(good);
        ladder.push(match s.bytes() {
            b if b >= 1 << 20 => format!("{} MB", b >> 20),
            b if b >= 1 << 10 => format!("{} KB", b >> 10),
            b => format!("{b} B"),
        });
    }
    let forbidden = [0u64, 1, 100, 128, 1 << 25, 1 << 27];
    let refused = forbidden
        .iter()
        .filter(|b| Size::from_bits(**b).is_err())
        .count() as u64
        + u64::from(Size::new(7).is_err());
    d.passed &= ok == 7 && refused == forbidden.len() as u64 + 1;
    d.result = format!(
        "sizes {}: {ok}/7 match the definition; {refused}/{} forbidden sizes refused",
        ladder.join(" > "),
        forbidden.len() + 1
    );
    d.expected = "7/7 and all forbidden sizes refused".into();
    d.measure("sizes matching", ok);
    d.measure("forbidden refused", refused);
    d
}

fn arenas(inputs: &mut Inputs, _: &Path) -> Draft {
    let mut d = Draft::new(
        "size.arena",
        "size",
        "An arena gives back exactly what was put in it, and refuses a Monade of the wrong size.",
    );
    d.explain(
        "Arenas are the memory of the mind: Monadi of one size side by side, reached by a reference. Nothing may be altered or mixed up on the way in or out.",
        "1 000 blocks, 100 symbols and 2 thoughts of 2 MB are stored and read back; each must be identical, and a Monade one word too long must be refused.",
    );
    let mut parts = Vec::new();
    for (size, count) in [
        (Size::BLOCK, 1000usize),
        (Size::SYMBOL, 100),
        (Size::THOUGHT, 2),
    ] {
        let mut arena = Arena::new(size);
        let stored: Vec<Vec<Word>> = (0..count).map(|_| inputs.monade(size)).collect();
        let refs: Vec<_> = stored.iter().filter_map(|m| arena.push(m).ok()).collect();
        let same = refs
            .iter()
            .zip(&stored)
            .filter(|(r, m)| arena.get(**r) == Some(m.as_slice()))
            .count();
        let wrong = vec![Word::ZERO; size.words() + 1];
        let refused = arena.push(&wrong).is_err();
        d.passed &= same == count && refused;
        parts.push(format!(
            "{} x {} bits: {} identical, wrong size {}",
            grouped(count as u64),
            grouped(size.bits()),
            grouped(same as u64),
            if refused { "refused" } else { "ACCEPTED" }
        ));
        d.measure(
            &format!("{} bits identical", size.bits()),
            format!("{same}/{count}"),
        );
    }
    d.result = parts.join("; ");
    d.expected = "all identical, wrong size refused".into();
    d
}

fn octal_address(inputs: &mut Inputs, _: &Path) -> Draft {
    let mut d = Draft::new(
        "address.octal",
        "address",
        "One octal address reaches any part of a Monade, down to one bit; the path is the Morton order of a cube with one point per bit.",
    );
    d.explain(
        "One addressing rule for experts, Monadi and bits: each base-8 digit chooses one of 8 parts, i.e. an octant of a cube. Moving down, up or to another tensor is just adding, removing or changing digits, and the same path is a position in space for the visual map.",
        "Every bit of a block (8x8x8) and of a symbol (16x16x16) is turned into a point: all points must be distinct. 10 000 random addresses go through every conversion and back, and are compared with slow references.",
    );
    let mut cubes = Vec::new();
    for top in [Size::BLOCK, Size::SYMBOL] {
        let e = top.level() + 2;
        let side = 1usize << e;
        let mut seen = vec![false; side * side * side];
        let mut distinct = 0u64;
        for offset in 0..top.bits() {
            if let Ok(a) = Address::from_bit_offset(0, top, offset, e) {
                let (x, y, z) = a.point();
                let i = (z as usize * side + y as usize) * side + x as usize;
                if let Some(s) = seen.get_mut(i) {
                    distinct += u64::from(!*s);
                    *s = true;
                }
            }
        }
        d.passed &= distinct == top.bits();
        cubes.push(format!(
            "{} bits = {} distinct points of a {side}x{side}x{side} cube",
            grouped(top.bits()),
            grouped(distinct)
        ));
        d.measure(&format!("{} bits distinct points", top.bits()), distinct);
    }
    let n = 10_000u64;
    let (mut round, mut slow) = (0u64, 0u64);
    for _ in 0..n {
        let r = inputs.next();
        let top = Size::new((r % 7) as u8).unwrap_or(Size::WORD);
        let e = top.level() + 2;
        let depth = ((r >> 8) % (u64::from(e) + 1)) as u8;
        let Ok(mut a) = Address::root(((r >> 16) % (1 << 28)) as u32, top) else {
            continue;
        };
        let mut digits = Vec::new();
        for i in 0..depth {
            let dg = ((r >> (40 + 3 * u32::from(i))) & 7) as u8;
            digits.push(dg);
            a = a.down(dg).unwrap_or(a);
        }
        let ok = a.depth() == depth
            && Address::from_bit_offset(a.root_id(), top, a.bit_offset(), depth) == Ok(a)
            && Address::from_point(a.root_id(), top, a.point(), depth) == Ok(a)
            && Address::unpack(a.pack(), top) == Ok(a)
            && a.up()
                .map_or(depth == 0, |p| p.down(digits[digits.len() - 1]) == Ok(a));
        round += u64::from(ok);
        slow += u64::from(
            a.bit_offset() == oracle::address_offset(e, &digits)
                && a.point() == oracle::morton_point(a.bit_offset(), e),
        );
    }
    let mut bit = Address::root(0, Size::THOUGHT);
    for _ in 0..8 {
        bit = bit.and_then(|b| b.down(7));
    }
    let one_bit = bit.is_ok_and(|b| b.bit_len() == 1 && b.point() == (255, 255, 255));
    d.passed &= round == n && slow == n && one_bit;
    d.result = format!(
        "{}; {} random addresses: round trips (down/up, offset, point, packed) in {}, fast = slow in {}; 8 digits from 2 MB reach one bit at (255, 255, 255): {}",
        cubes.join("; "),
        grouped(n),
        grouped(round),
        grouped(slow),
        if one_bit { "yes" } else { "NO" }
    );
    d.expected =
        "every bit a distinct point; all round trips; fast = slow; one bit at depth 8".into();
    d.measure("random addresses", n);
    d.measure("round trips", round);
    d.measure("fast = slow", slow);
    d
}

/// The space of the mind in `dir` (genesis if the specifications cannot be read).
fn space_of(dir: &Path) -> Space {
    store::load_specs(dir)
        .ok()
        .and_then(|s| Space::from_specs(&s).ok())
        .unwrap_or_else(Space::genesis)
}

/// Fingerprint of blocks of ids 0..64 of the genesis space (as in `monade-core`).
const GOLDEN_SPACE: u64 = 0x1125_fbd0_ff9f_9379;

fn generated_space(inputs: &mut Inputs, dir: &Path) -> Draft {
    let mut d = Draft::new(
        "space.generated",
        "space",
        "Basic vectors are computed, not stored: the same id gives the same vector on every machine, different ids are strangers, and any block can be computed alone.",
    );
    d.explain(
        "Every axis, role and word of the mind needs its own random vector. Generating them from the seed means they cost no memory until used, are identical everywhere, and form one space per mind: another seed would be another mind.",
        "BLAKE3 keyed by the seed (context \"monade space v1\") turns an id into a stream of bits. The check compares a fingerprint of 64 generated blocks with the value fixed when version 1 was written, measures the distance between 1 000 pairs of generated vectors, checks that any block of a 2 MB vector computed alone equals the same block of the whole vector, and times the generation.",
    );
    let space = space_of(dir);
    let ids: Vec<Word> = (0..64)
        .flat_map(|id| space.generate(id, Size::BLOCK))
        .collect();
    let fingerprint = oracle::state_hash(&ids);
    let golden_ok = space != Space::genesis() || fingerprint == GOLDEN_SPACE;
    let pairs = 1_000u64;
    let mut values = Vec::new();
    for _ in 0..pairs {
        let (a, b) = (inputs.next(), inputs.next());
        if a == b {
            continue;
        }
        let h = word::hamming(
            &space.generate(a, Size::BLOCK),
            &space.generate(b, Size::BLOCK),
        )
        .unwrap_or(0);
        values.push(h);
    }
    let sum: u64 = values.iter().sum();
    let n = values.len() as u64;
    let min = values.iter().copied().min().unwrap_or(0);
    let mean_ok = {
        let dev = sum.abs_diff(n * 256);
        dev * dev <= 4 * n * 512
    };
    let width = 5;
    let start = 256 - 20 * width;
    d.charts.push(Histogram {
        title: format!(
            "Distance between the generated vectors of two different ids ({} pairs, 512 bits)",
            grouped(n)
        ),
        unit: "bits of difference".into(),
        start,
        width,
        series: vec![Series {
            name: "generated pairs".into(),
            counts: histogram(&values, start, width, 40),
        }],
        labels: Vec::new(),
    });
    let thoughts = 8u64;
    let mut same = 0u64;
    let start_time = Instant::now();
    for _ in 0..thoughts {
        let id = inputs.next();
        let whole = space.generate(id, Size::THOUGHT);
        let k = inputs.next() % 32_768;
        let block = space.block(id, k);
        let at = (k * 8) as usize;
        same += u64::from(whole.get(at..at + 8) == Some(&block.0[..]));
    }
    let micros = u64::try_from(start_time.elapsed().as_micros())
        .unwrap_or(u64::MAX)
        .max(1);
    let mb_per_s = thoughts * 2 * 1_000_000 / micros;
    d.passed = golden_ok && mean_ok && min > 190 && same == thoughts;
    d.result = format!(
        "fingerprint {fingerprint:#018x} ({}); {} pairs of ids: mean distance {} of 512, closest {min}; any block of a 2 MB vector computed alone: {same}/{thoughts} identical; speed about {} MB/s",
        if golden_ok { "as fixed" } else { "CHANGED" },
        grouped(n),
        tenths(sum * 10 / n.max(1)),
        grouped(mb_per_s)
    );
    d.expected = format!(
        "fingerprint {GOLDEN_SPACE:#018x}; mean 256; closest above 190; all blocks identical"
    );
    d.measure("fingerprint", format!("{fingerprint:#018x}"));
    d.measure("seed", format!("{:#018x}", space.seed()));
    d.measure("mean distance (512 bits)", tenths(sum * 10 / n.max(1)));
    d.measure("closest pair", min);
    d.measure("blocks identical", format!("{same}/{thoughts}"));
    d.measure("generation speed (MB/s)", mb_per_s);
    d
}

fn sentence(_: &mut Inputs, dir: &Path) -> Draft {
    let mut d = Draft::new(
        "space.sentence",
        "space",
        "A sentence is one 512-bit block: \"the dog sleeps\" bound role by role and bundled; asking a role gives back its word.",
    );
    d.explain(
        "This is composition: several ideas, each tied to its role, held in one vector and taken apart again by asking. It is the first time the Monade represents something built from generated vectors; nothing is learned, it is pure algebra.",
        "Words and roles are generated from their names. The sentence is the majority of (subject XOR dog), (verb XOR sleep) and (time XOR present). Asking a role means XOR of the sentence with the role, then finding the nearest word of a vocabulary of 8. A second sentence, \"the cat ran\", must give different answers.",
    );
    let space = space_of(dir);
    let v = |n: &str| space.named(n, Size::BLOCK);
    let vocabulary = [
        "it.cane",
        "it.gatto",
        "it.dormire",
        "it.correre",
        "tempo.presente",
        "tempo.passato",
        "it.casa",
        "it.mangiare",
    ];
    let roles = ["ruolo.soggetto", "ruolo.verbo", "ruolo.tempo"];
    let sentences: [[&str; 3]; 2] = [
        ["it.cane", "it.dormire", "tempo.presente"],
        ["it.gatto", "it.correre", "tempo.passato"],
    ];
    let mut right = 0u64;
    let mut lines = Vec::new();
    let mut worst_margin = u64::MAX;
    for words in sentences {
        let bound: Vec<Vec<Word>> = roles
            .iter()
            .zip(words)
            .map(|(r, w)| v(r).iter().zip(v(w)).map(|(a, b)| a.xor(b)).collect())
            .collect();
        let refs: Vec<&[Word]> = bound.iter().map(Vec::as_slice).collect();
        let mut s = vec![Word::ZERO; Size::BLOCK.words()];
        if word::majority(&refs, &mut s).is_err() {
            d.passed = false;
            continue;
        }
        let mut answers = Vec::new();
        for (role, expected) in roles.iter().zip(words) {
            let query: Vec<Word> = s.iter().zip(v(role)).map(|(a, b)| a.xor(b)).collect();
            let mut ranked: Vec<(u64, &str)> = vocabulary
                .iter()
                .map(|w| (word::hamming(&query, &v(w)).unwrap_or(u64::MAX), *w))
                .collect();
            ranked.sort();
            let (best, name) = ranked[0];
            let next = ranked[1].0;
            worst_margin = worst_margin.min(next.saturating_sub(best));
            right += u64::from(name == expected);
            answers.push(format!(
                "{}? {name} ({best}, next {next})",
                role.trim_start_matches("ruolo.")
            ));
            d.measure(
                &format!("{} / {}: answer", words[0], role),
                format!("{name} at {best}, next {next}"),
            );
        }
        lines.push(format!("[{}] {}", words.join(" "), answers.join(", ")));
    }
    d.passed &= right == 6;
    d.result = format!(
        "{}; right answers {right}/6; smallest margin {worst_margin} bits",
        lines.join("; ")
    );
    d.expected = "6/6 right; each answer near 128 bits, every other word near 256".into();
    d.measure("right answers", format!("{right}/6"));
    d.measure("smallest margin (bits)", worst_margin);
    d
}

/// Noise of `memory.reinforcement`: each bit flipped with probability 77/256 (30.1%).
const FLIP_30: u64 = 77;

/// Pre-registered for `memory.reinforcement` (written before the first run, from
/// the exact binomial; co-signed by the statistician): N noisy copies of a 512-bit
/// pattern, each bit flipped with probability 77/256. For each N: the expected
/// mean distance between the reading and the pattern, and the tolerance, both in
/// tenths of a bit. A tie reads 0 and is wrong half the time, so N = 2 is no
/// better than N = 1. Tolerance = 4 standard errors over 32 trials, at least 1 bit.
const REINFORCEMENT: [(usize, u64, u64); 7] = [
    (1, 1540, 73),
    (2, 1540, 73),
    (4, 1111, 66),
    (8, 651, 53),
    (16, 260, 35),
    (32, 50, 16),
    (64, 2, 10),
];

/// A tally of `votes` (all the same size).
fn tally_of(size: Size, votes: &[Vec<Word>]) -> Tally {
    let mut t = Tally::new(size);
    for v in votes {
        let _ = t.add(v);
    }
    t
}

fn reinforcement(inputs: &mut Inputs, _: &Path) -> Draft {
    let mut d = Draft::new(
        "memory.reinforcement",
        "memory",
        "What repeats is reinforced: a pattern seen N times, each time with 30% of its bits flipped at random, is recovered better as N grows; random voices with no pattern are never recovered.",
    );
    d.explain(
        "This is the first learning of the Monade. Each bit has a counter: a vote adds 1 where the bit is 1 and takes 1 where it is 0. The bit is the reading, the counter is the memory. Noise points every way and cancels; the pattern always points the same way and grows. Nothing is stored but the counters, and no example is kept.",
        "For each N (1, 2, 4 ... 64), in 32 trials, a random 512-bit pattern is shown N times, each copy with every bit flipped with probability 30%; the copies are counted in a tally and the reading (the sign of each counter) is compared with the pattern. The expected distances were computed beforehand from the binomial law and written in the code. Control: N random voices with no common pattern, read the same way, must stay as far from the pattern as a stranger (above half the bits minus 6 sigma).",
    );
    let trials = 32u64;
    let floor = 256 - 3 * isqrt(512);
    let (mut means, mut controls, mut labels, mut parts) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut previous: Option<(u64, u64)> = None;
    for (n, expected, tolerance) in REINFORCEMENT {
        let (mut sum, mut control_sum, mut control_min) = (0u64, 0u64, u64::MAX);
        for _ in 0..trials {
            let pattern = inputs.monade(Size::BLOCK);
            let copies: Vec<Vec<Word>> = (0..n).map(|_| inputs.noisy(&pattern, FLIP_30)).collect();
            let read = tally_of(Size::BLOCK, &copies).read();
            sum += word::hamming(&read, &pattern).unwrap_or(512);
            let strangers: Vec<Vec<Word>> = (0..n).map(|_| inputs.monade(Size::BLOCK)).collect();
            let c = word::hamming(&tally_of(Size::BLOCK, &strangers).read(), &pattern).unwrap_or(0);
            control_sum += c;
            control_min = control_min.min(c);
        }
        let mean = sum * 10 / trials;
        let within = mean.abs_diff(expected) <= tolerance;
        // Never worse than half as many copies (within the tolerance).
        let monotone = previous.is_none_or(|(m, tol)| mean <= m + tol.max(tolerance));
        let control_ok = control_min > floor;
        d.passed &= within && monotone && control_ok;
        previous = Some((mean, tolerance));
        means.push((mean + 5) / 10);
        controls.push(control_sum / trials);
        labels.push(format!("N={n}"));
        parts.push(format!("N={n}: {}", tenths(mean)));
        d.measure(
            &format!("N = {n}: mean distance (expected)"),
            format!(
                "{} ({} ± {})",
                tenths(mean),
                tenths(expected),
                tenths(tolerance)
            ),
        );
        d.measure(
            &format!("N = {n}: control, mean and closest"),
            format!("{} and {control_min}", tenths(control_sum * 10 / trials)),
        );
    }
    d.charts.push(Histogram {
        title: "Distance between the reading and the pattern, by number of noisy copies (512 bits; lower is better)".into(),
        unit: "bits of difference (mean of 32 trials)".into(),
        start: 0,
        width: 1,
        series: vec![
            Series {
                name: "pattern seen N times".into(),
                counts: means,
            },
            Series {
                name: "control: N random voices".into(),
                counts: controls,
            },
        ],
        labels,
    });
    d.result = format!(
        "512 bits, 30% noise, {} trials each; mean distance {}; control always above {floor}",
        trials,
        parts.join(", ")
    );
    d.expected = "each mean within its pre-registered tolerance (154 at N=1 and N=2, 111 at 4, 65 at 8, 26 at 16, 5 at 32, 0.2 at 64); never worse with more copies; control above half minus 6 sigma".into();
    d
}

/// Numbers of voices of `memory.capacity`.
const CAPACITY_VOICES: [usize; 10] = [1, 2, 3, 4, 6, 8, 12, 16, 24, 32];

fn capacity(inputs: &mut Inputs, _: &Path) -> Draft {
    let mut d = Draft::new(
        "memory.capacity",
        "memory",
        "A tally holds a limited number of voices, growing with its size: 8 voices in 512 bits stay recognisable (about 6 sigma above chance); in 64 bits they do not (about 2).",
    );
    d.explain(
        "The blackboard of a meeting is one 512-bit block per level holding up to 8 voices. This check measures how many voices a tally can hold and still give each one back, at 64, 512 and 4096 bits: the capacity curve of test 19. It tells where the blackboard stands, and that 64 bits are too few.",
        "For each size and each number of voices k, in 40 trials, k random voices are counted in a tally; each voice must be closer to the reading than all of 100 random decoys. Separation = (half the bits - mean distance of the voices) / (sigma of a decoy). Thresholds written before the first run, from the exact binomial: 512 bits with 8 voices at least 99% and 5 sigma (expected 100%, 6.2); 4096 bits with 32 voices at least 99% (expected 9 sigma). Control that must fail: 64 bits with 8 voices at most 60% and 3 sigma (expected about 35% and 2.2).",
    );
    let trials = 40u64;
    let mut series = Vec::new();
    let mut parts = Vec::new();
    for size in [Size::WORD, Size::BLOCK, Size::SYMBOL] {
        let bits = size.bits();
        let root = isqrt(100 * bits);
        let mut counts = Vec::new();
        let mut line = Vec::new();
        for k in CAPACITY_VOICES {
            let (mut found, mut tried, mut sum) = (0u64, 0u64, 0u64);
            for _ in 0..trials {
                let voices: Vec<Vec<Word>> = (0..k).map(|_| inputs.monade(size)).collect();
                let read = tally_of(size, &voices).read();
                let nearest = (0..100)
                    .map(|_| word::hamming(&read, &inputs.monade(size)).unwrap_or(0))
                    .min()
                    .unwrap_or(0);
                for v in &voices {
                    let h = word::hamming(&read, v).unwrap_or(bits);
                    sum += h;
                    tried += 1;
                    found += u64::from(h < nearest);
                }
            }
            let per_mille = found * 1000 / tried.max(1);
            // Separation in tenths of sigma: 20 (n/2 - mean) / sqrt(n).
            let sep = (bits * tried / 2).saturating_sub(sum) * 200 / (tried * root).max(1);
            counts.push((per_mille + 5) / 10);
            line.push(format!("{k}: {}% ({})", tenths(per_mille), tenths(sep)));
            match (bits, k) {
                (512, 8) => {
                    d.passed &= per_mille >= 990 && sep >= 50;
                    parts.insert(
                        0,
                        format!(
                            "8 voices in 512 bits: {}% found, {} sigma",
                            tenths(per_mille),
                            tenths(sep)
                        ),
                    );
                }
                (4096, 32) => {
                    d.passed &= per_mille >= 990;
                    parts.push(format!(
                        "32 voices in 4096 bits: {}% found, {} sigma",
                        tenths(per_mille),
                        tenths(sep)
                    ));
                }
                (64, 8) => {
                    d.passed &= per_mille <= 600 && sep <= 30;
                    parts.push(format!(
                        "control, 8 voices in 64 bits: {}% found, {} sigma (must fail)",
                        tenths(per_mille),
                        tenths(sep)
                    ));
                }
                _ => {}
            }
        }
        series.push(Series {
            name: format!("{bits} bits"),
            counts,
        });
        d.measure(
            &format!("{bits} bits: voices: found (sigma)"),
            line.join(", "),
        );
    }
    // Speed of counting at 2 MB (a number for the electronic engineer; no threshold).
    let thought = inputs.monade(Size::THOUGHT);
    let mut t = Tally::new(Size::THOUGHT);
    let start = Instant::now();
    for _ in 0..4 {
        let _ = t.add(&thought);
    }
    let micros = u64::try_from(start.elapsed().as_micros())
        .unwrap_or(u64::MAX)
        .max(1);
    let mb_per_s = 4 * 2 * 1_000_000 / micros;
    d.measure("counting speed at 2 MB (MB of votes per second)", mb_per_s);
    d.charts.push(Histogram {
        title: format!(
            "Voices recovered from one tally, by number of voices ({} trials, 100 decoys each)",
            trials
        ),
        unit: "% of voices found again".into(),
        start: 0,
        width: 1,
        series,
        labels: CAPACITY_VOICES.iter().map(|k| format!("k={k}")).collect(),
    });
    d.result = format!(
        "{}; counting speed about {} MB/s",
        parts.join("; "),
        grouped(mb_per_s)
    );
    d.expected = "512 bits, 8 voices: at least 99% and 5 sigma; 4096 bits, 32 voices: at least 99%; 64 bits, 8 voices: at most 60% and 3 sigma".into();
    d
}

/// Noise of `memory.residue`: each bit flipped with probability 26/256 (10.2%).
const FLIP_10: u64 = 26;

/// Certainty below which a bit is uncertain in `memory.residue`.
const UNCERTAIN_BELOW: u8 = 3;

fn residue(inputs: &mut Inputs, _: &Path) -> Draft {
    let mut d = Draft::new(
        "memory.residue",
        "memory",
        "With 5 voices agreeing and 3 saying something else, the reading follows the 5, the residue of the 3 is large, and the uncertain counters mark where the two groups disagree.",
    );
    d.explain(
        "The residue is what the majority discarded of a voice: the base form of Mutation, and the way the minority is heard instead of erased. The counters say how sure each bit is: near 0 where the voices disagree. So one vote gives three things: the decision, who was outvoted and by how much, and where the doubt is.",
        "In 32 trials, 5 voices are copies of a pattern A and 3 of a pattern B (each copy with 10% of its bits flipped); all 8 are counted in one 512-bit tally. Measured: the distance of the reading from A; the residue (voice XOR reading) of the 5 and of the 3; the uncertain mask (counters with absolute value below 3) against the bits where A and B differ. Thresholds written before the first run, from a simulation of this model: residue of the 3 at least 180 bits (expected 209), of the 5 at most 100 (73); reading within 80 bits of A (50); mask recall at least 75% (82%) and precision at least 90% (96%). Control that must fail: 8 voices all from A: uncertain bits at most 40 (20) and residue of its last 3 at most 80 (52), i.e. no minority.",
    );
    let trials = 32u64;
    let (mut read_a, mut maj, mut min, mut uncertain, mut hits, mut differ) =
        (0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let (mut ctl_uncertain, mut ctl_residue) = (0u64, 0u64);
    let (mut certainty, mut ctl_certainty) = (vec![0u64; 9], vec![0u64; 9]);
    let res = |t: &Tally, v: &[Word]| t.residue(v).map_or(512, |r| word::popcount(&r));
    for _ in 0..trials {
        let a = inputs.monade(Size::BLOCK);
        let b = inputs.monade(Size::BLOCK);
        let mut voices: Vec<Vec<Word>> = (0..5).map(|_| inputs.noisy(&a, FLIP_10)).collect();
        voices.extend((0..3).map(|_| inputs.noisy(&b, FLIP_10)));
        let t = tally_of(Size::BLOCK, &voices);
        read_a += word::hamming(&t.read(), &a).unwrap_or(512);
        maj += voices[..5].iter().map(|v| res(&t, v)).sum::<u64>();
        min += voices[5..].iter().map(|v| res(&t, v)).sum::<u64>();
        let mask = t.uncertain(UNCERTAIN_BELOW);
        let diff: Vec<Word> = a.iter().zip(&b).map(|(x, y)| x.xor(*y)).collect();
        let both: Vec<Word> = mask.iter().zip(&diff).map(|(m, x)| m.and(*x)).collect();
        uncertain += word::popcount(&mask);
        hits += word::popcount(&both);
        differ += word::popcount(&diff);
        for (slot, n) in certainty.iter_mut().zip(t.certainty_histogram()) {
            *slot += n;
        }
        let agreeing: Vec<Vec<Word>> = (0..8).map(|_| inputs.noisy(&a, FLIP_10)).collect();
        let c = tally_of(Size::BLOCK, &agreeing);
        ctl_uncertain += word::popcount(&c.uncertain(UNCERTAIN_BELOW));
        ctl_residue += agreeing[5..].iter().map(|v| res(&c, v)).sum::<u64>();
        for (slot, n) in ctl_certainty.iter_mut().zip(c.certainty_histogram()) {
            *slot += n;
        }
    }
    let read_a = read_a * 10 / trials;
    let maj = maj * 10 / (5 * trials);
    let min = min * 10 / (3 * trials);
    let recall = hits * 1000 / differ.max(1);
    let precision = hits * 1000 / uncertain.max(1);
    let chance = differ * 1000 / (512 * trials);
    let ctl_uncertain = ctl_uncertain * 10 / trials;
    let ctl_residue = ctl_residue * 10 / (3 * trials);
    d.passed = min >= 1800
        && maj <= 1000
        && read_a <= 800
        && recall >= 750
        && precision >= 900
        && ctl_uncertain <= 400
        && ctl_residue <= 800;
    d.measure("distance of the reading from A", tenths(read_a));
    d.measure("residue of the 5 (majority), mean", tenths(maj));
    d.measure("residue of the 3 (minority), mean", tenths(min));
    d.measure(
        "uncertain bits (|counter| < 3), mean",
        tenths(uncertain * 10 / trials),
    );
    d.measure(
        "mask recall: disagreements found",
        format!("{}%", tenths(recall)),
    );
    d.measure(
        "mask precision: marks that are disagreements",
        format!("{}%", tenths(precision)),
    );
    d.measure(
        "precision by chance (share of bits where A and B differ)",
        format!("{}%", tenths(chance)),
    );
    d.measure("control: uncertain bits, mean", tenths(ctl_uncertain));
    d.measure("control: residue of the last 3, mean", tenths(ctl_residue));
    let scale = |v: Vec<u64>| v.iter().map(|n| n / trials).collect::<Vec<u64>>();
    d.charts.push(Histogram {
        title:
            "Certainty of the counters (|counter|) in a 512-bit tally of 8 voices, mean per trial"
                .into(),
        unit: "bits with each certainty (0 = tie, 8 = all 8 voices agree)".into(),
        start: 0,
        width: 1,
        series: vec![
            Series {
                name: "5 agree, 3 differ".into(),
                counts: scale(certainty),
            },
            Series {
                name: "control: 8 agree".into(),
                counts: scale(ctl_certainty),
            },
        ],
        labels: (0..=8).map(|c| c.to_string()).collect(),
    });
    d.result = format!(
        "{trials} trials of 5 + 3 voices in 512 bits: reading {} bits from A; residue of the 3 = {} bits vs the 5 = {}; uncertain mask: recall {}%, precision {}% (chance {}%); control of 8 agreeing: {} uncertain bits, residue of the last 3 = {}",
        tenths(read_a),
        tenths(min),
        tenths(maj),
        tenths(recall),
        tenths(precision),
        tenths(chance),
        tenths(ctl_uncertain),
        tenths(ctl_residue)
    );
    d.expected = "residue of the 3 at least 180, of the 5 at most 100; reading within 80 of A; recall at least 75%, precision at least 90%; control: at most 40 uncertain bits, residue of the last 3 at most 80".into();
    d
}

fn foundations(_: &mut Inputs, dir: &Path) -> Draft {
    let mut d = Draft::new(
        "foundations.read",
        "foundations",
        "Point 0 and the specifications are read in canonical form; the seed of the space is locked.",
    );
    d.explain(
        "Point 0 and the specifications are the foundations: only the author writes them, nothing can rewrite the past, and the seed that generates the space can never change (a new seed is a new mind).",
        "The two files are read from the foundations folder (a missing file means a new mind) and checked for canonical form; the seed must equal the genesis seed and be locked.",
    );
    let origin = store::load_origin(dir);
    let specs = store::load_specs(dir);
    match (origin, specs) {
        (Ok(o), Ok(s)) => {
            let seed = s.get(SEED_KEY).map(|x| x.value.clone()).unwrap_or_default();
            let genesis = Specs::genesis()
                .get(SEED_KEY)
                .map(|x| x.value.clone())
                .unwrap_or_default();
            d.passed = seed == genesis && Specs::is_locked(SEED_KEY);
            let entries = o.entries().len();
            d.result = format!(
                "folder {}: point 0 has {entries} entries{}; {} specification entries; seed {seed} locked",
                dir.display(),
                if entries == 0 { " (empty, decided last)" } else { "" },
                s.history().len()
            );
            d.measure("point 0 entries", entries);
            d.measure("specification entries", s.history().len());
            d.measure("seed", seed);
        }
        (o, s) => {
            d.passed = false;
            let err = |e: Option<String>| e.unwrap_or_default();
            d.result = format!(
                "cannot read: {} {}",
                err(o.err().map(|e| e.to_string())),
                err(s.err().map(|e| e.to_string()))
            );
        }
    }
    d.expected = "both files readable; seed equal to the genesis seed and locked".into();
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_check_passes_on_a_new_mind() {
        let dir = std::env::temp_dir().join(format!("monade-check-{}", std::process::id()));
        let out = all(&dir);
        assert_eq!(out.len(), 15);
        for o in &out {
            assert!(o.passed, "{}: {}", o.id, o.result);
        }
        assert!(!dir.exists(), "check never writes foundations");
    }

    #[test]
    fn histogram_clamps() {
        assert_eq!(histogram(&[0, 5, 9, 10, 99], 4, 2, 3), vec![2, 0, 3]);
    }

    #[test]
    fn helpers() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(10_000), "10 000");
        assert_eq!(grouped(1_234_567), "1 234 567");
        assert_eq!(tenths(2563), "256.3");
        assert_eq!(isqrt(512), 22);
        assert_eq!(isqrt(4096), 64);
    }
}
