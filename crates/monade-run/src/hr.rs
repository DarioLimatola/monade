//! The first learning test on the stars: the Hertzsprung–Russell relation.
//!
//! Can the mind learn, by counting only, how bright a star truly is from its colour?
//! For each of 64 colour bins (B-V in steps of 0.05 from -0.5) a tally of 64 bits
//! counts the absolute magnitudes of the training stars, written as a thermometer
//! (level L = the L lowest bits at 1, levels of half a magnitude from -10). The
//! majority of a thermometer code is its median, so reading the tally of a colour
//! gives the typical brightness of that colour: the main sequence, learned.
//!
//! Stars with an even place in the catalog teach, those with an odd place are asked.
//! The teachers come in a shuffled order (a fixed permutation): a catalog is sorted
//! (HYG ends with thousands of faint red dwarfs), and a tally of 8 bits, once
//! saturated, follows its most recent votes. The first run, in catalog order, showed
//! it: the tally of all the stars read 42 (the red dwarfs at the end) where the median
//! was 23.
//! The error is compared with a baseline that ignores colour (one tally for every
//! star), and with a control that learns from shuffled colours, which must not learn.
//!
//! Thresholds written before the first run (2026-11-19), as the rules ask:
//! learned error at most 60 % of the baseline; control error at least 90 % of it.
//! Integers only.

use crate::stars::Star;
use monade_core::size::Size;
use monade_core::tally::Tally;
use monade_core::word::{self, Word};

/// Colour bins and brightness levels.
pub const BINS: usize = 64;
/// Learned error at most this share of the baseline (per mille). Pre-registered.
pub const PASS_PERMILLE: u64 = 600;
/// Control error at least this share of the baseline (per mille). Pre-registered.
pub const CONTROL_PERMILLE: u64 = 900;

/// What the test measured. Errors are mean absolute errors, thousandths of a magnitude.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hr {
    /// Stars that taught, stars that were asked.
    pub train: u64,
    /// Stars that were asked.
    pub test: u64,
    /// Error of the tallies per colour.
    pub learned: u64,
    /// Error of one tally for every star (colour ignored).
    pub baseline: u64,
    /// Error of tallies per colour learned from shuffled colours.
    pub control: u64,
    /// Error of the exact median per colour (a sort, not a tally): what this way of
    /// learning could reach with counters that never saturate. A reference, not a test.
    pub exact: u64,
    /// Colour bins with at least one voice.
    pub bins_used: u64,
    /// Colour bins whose counters never clamped.
    pub bins_exact: u64,
    /// The learned brightness level of each colour bin (None: no voice).
    pub curve: Vec<Option<u64>>,
}

impl Hr {
    /// Learned error within the pre-registered share of the baseline.
    pub fn learned_passes(&self) -> bool {
        self.learned * 1000 <= PASS_PERMILLE * self.baseline
    }
    /// The control did not learn (as it must).
    pub fn control_holds(&self) -> bool {
        self.control * 1000 >= CONTROL_PERMILLE * self.baseline
    }
}

fn level(absmag: i64) -> u64 {
    ((absmag + 10_000) / 500).clamp(0, 63) as u64
}

fn bin(ci: i64) -> usize {
    ((ci + 500) / 50).clamp(0, BINS as i64 - 1) as usize
}

fn thermometer(l: u64) -> [Word; 1] {
    [Word::new(if l == 0 { 0 } else { u64::MAX >> (64 - l) })]
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

fn read_level(t: &Tally) -> u64 {
    word::popcount(&t.read())
}

/// Runs the test on the stars that have a colour and an absolute magnitude.
/// None if there are fewer than 2 such stars.
pub fn learn(stars: &[Star]) -> Option<Hr> {
    let pairs: Vec<(usize, u64)> = stars
        .iter()
        .filter_map(|s| Some((bin(s.ci?), level(s.absmag?))))
        .collect();
    let train: Vec<(usize, u64)> = pairs.iter().step_by(2).copied().collect();
    let test: Vec<(usize, u64)> = pairs.iter().skip(1).step_by(2).copied().collect();
    if train.is_empty() || test.is_empty() {
        return None;
    }
    let new = || Tally::new(Size::WORD);
    let mut by_colour: Vec<Tally> = (0..BINS).map(|_| new()).collect();
    let mut shuffled: Vec<Tally> = (0..BINS).map(|_| new()).collect();
    let mut all = new();
    let n = train.len();
    let step = (7919..).find(|p| gcd(*p, n) == 1).unwrap_or(1);
    let train: Vec<(usize, u64)> = (0..n).map(|k| train[k * step % n]).collect();
    for (i, (b, l)) in train.iter().enumerate() {
        let v = thermometer(*l);
        // Same lengths by construction: adding cannot fail.
        let _ = by_colour[*b].add(&v);
        let _ = all.add(&v);
        // The control: the brightness of this star, the colour of another one.
        let other = train[(i * 7919 + 13) % n].0;
        let _ = shuffled[other].add(&v);
    }
    let predict = |t: &[Tally], b: usize| {
        if t[b].voices().count == 0 {
            read_level(&all)
        } else {
            read_level(&t[b])
        }
    };
    let error = |f: &dyn Fn(usize) -> u64| {
        test.iter()
            .map(|(b, l)| f(*b).abs_diff(*l) * 500)
            .sum::<u64>()
            / test.len() as u64
    };
    let base = read_level(&all);
    let mut per_bin: Vec<Vec<u64>> = vec![Vec::new(); BINS];
    for (b, l) in &train {
        per_bin[*b].push(*l);
    }
    let mut all_levels: Vec<u64> = train.iter().map(|p| p.1).collect();
    all_levels.sort_unstable();
    let global_median = all_levels[all_levels.len() / 2];
    let medians: Vec<u64> = per_bin
        .iter_mut()
        .map(|v| {
            v.sort_unstable();
            v.get(v.len() / 2).copied().unwrap_or(global_median)
        })
        .collect();
    Some(Hr {
        train: n as u64,
        test: test.len() as u64,
        learned: error(&|b| predict(&by_colour, b)),
        baseline: error(&|_| base),
        control: error(&|b| predict(&shuffled, b)),
        exact: error(&|b| medians[b]),
        bins_used: by_colour.iter().filter(|t| t.voices().count > 0).count() as u64,
        bins_exact: by_colour
            .iter()
            .filter(|t| t.voices().count > 0 && t.is_exact())
            .count() as u64,
        curve: by_colour
            .iter()
            .map(|t| (t.voices().count > 0).then(|| read_level(t)))
            .collect(),
    })
}

/// Thousandths of a magnitude as text.
fn mag(m: u64) -> String {
    format!("{}.{:02} mag", m / 1000, m % 1000 / 10)
}

/// The result in words, for the terminal.
pub fn summary(name: &str, hr: &Hr) -> String {
    format!(
        "HR relation learned from {name}: {} stars taught, {} asked; error {} (baseline without colour {}, \
         control with shuffled colours {}; exact median per colour, a reference: {}); learned {} 60% of the baseline: {}; control {} 90%: {}; \
         {} colour bins used, {} exact\n",
        crate::check::grouped(hr.train),
        crate::check::grouped(hr.test),
        mag(hr.learned),
        mag(hr.baseline),
        mag(hr.control),
        mag(hr.exact),
        if hr.learned_passes() { "within" } else { "NOT within" },
        if hr.learned_passes() { "PASS" } else { "FAIL" },
        if hr.control_holds() { "at least" } else { "below" },
        if hr.control_holds() { "PASS (it did not learn, as it must)" } else { "FAIL (the control learned: the test is unsound)" },
        hr.bins_used,
        hr.bins_exact,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn star(ci: i64, absmag: i64) -> Star {
        Star {
            pos: [0, 0, 0],
            mag: None,
            absmag: Some(absmag),
            ci: Some(ci),
            name: String::new(),
            hip: None,
        }
    }

    #[test]
    fn learns_a_relation_and_the_control_does_not() {
        // A clean main sequence: absolute magnitude = 1 + 6 × colour, with scatter.
        let stars: Vec<Star> = (0..4000)
            .map(|i| {
                let ci = (i * 37 % 2000) as i64 - 200;
                let scatter = ((i * 7919 % 1000) as i64 - 500) * 2;
                star(ci, 1000 + 6 * ci + scatter)
            })
            .collect();
        let hr = learn(&stars).unwrap();
        assert_eq!((hr.train, hr.test), (2000, 2000));
        assert!(hr.learned_passes(), "{hr:?}");
        assert!(hr.control_holds(), "{hr:?}");
        assert!(hr.learned < hr.baseline / 2);
    }

    #[test]
    fn no_relation_no_learning() {
        // Brightness independent of colour: the learned error is not better.
        let stars: Vec<Star> = (0..4000)
            .map(|i| {
                star(
                    (i * 37 % 2000) as i64 - 200,
                    (i * 7919 % 10_000) as i64 - 2000,
                )
            })
            .collect();
        let hr = learn(&stars).unwrap();
        assert!(!hr.learned_passes(), "{hr:?}");
    }

    #[test]
    fn thermometer_majority_is_the_median() {
        let mut t = Tally::new(Size::WORD);
        for l in [3, 10, 11, 12, 40] {
            t.add(&thermometer(l)).unwrap();
        }
        assert_eq!(read_level(&t), 11);
        assert!(learn(&[star(0, 0)]).is_none());
    }
}
