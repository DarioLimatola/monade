//! Counters per bit: the first learning.
//!
//! The bit is the reading, the counter is the memory. A [`Tally`] keeps one
//! signed counter per bit of a Monade. Adding a vector votes `+w` on its 1-bits
//! and `-w` on its 0-bits; the reading is the sign of every counter, and a
//! counter at 0 (a tie) reads 0: point 0, suspension of judgement, as
//! [`word::majority`](crate::word::majority).
//!
//! What repeats grows certain; isolated noise cancels; where the voices disagree
//! the counters stay near 0, which is measurable uncertainty and the base of the
//! residue (what the majority discarded of a voice).
//!
//! Counters are one byte each (`i8`), symmetric and saturating at ±[`LIMIT`]:
//! 64 counters, one word of bits, fill one 64-byte cache line. A counter never
//! wraps around. While nothing has been clamped or decayed the tally is
//! [exact](Tally::is_exact): every counter is the sum of its votes, adding and
//! subtracting are exact inverses, and the reading equals the majority.

use crate::size::Size;
use crate::word::Word;
use core::fmt;

/// The largest absolute value of a counter. `-128` is never used, so the
/// counters are symmetric: negating a tally is always possible.
///
/// # Example
///
/// ```
/// use monade_core::tally::LIMIT;
/// assert_eq!(LIMIT, 127);
/// ```
pub const LIMIT: i8 = 127;

/// How many votes a tally holds and their total weight.
///
/// # Example
///
/// ```
/// use monade_core::size::Size;
/// use monade_core::tally::{Tally, Voices};
/// use monade_core::word::Word;
/// let mut t = Tally::new(Size::WORD);
/// t.add_weighted(&[Word::new(1)], 3).unwrap();
/// assert_eq!(t.voices(), Voices { count: 1, weight: 3 });
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Voices {
    /// Number of votes.
    pub count: u64,
    /// Sum of their weights.
    pub weight: u64,
}

/// Errors of a tally. On any error the tally is left unchanged.
///
/// # Example
///
/// ```
/// use monade_core::size::Size;
/// use monade_core::tally::{Tally, TallyError};
/// let mut t = Tally::new(Size::BLOCK);
/// assert_eq!(t.add(&[]), Err(TallyError::WrongLength { expected: 8, found: 0 }));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TallyError {
    /// A vector of the wrong number of words for this tally.
    WrongLength {
        /// Words of the tally's size.
        expected: usize,
        /// Words given.
        found: usize,
    },
    /// A weight outside 1 to 127.
    Weight(u8),
    /// Two tallies of different sizes.
    SizeMismatch {
        /// Size of this tally.
        expected: Size,
        /// Size of the other.
        found: Size,
    },
    /// Subtracting more votes, or more weight, than the tally holds.
    NoVoices,
}

impl fmt::Display for TallyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TallyError::WrongLength { expected, found } => {
                write!(f, "expected {expected} words, found {found}")
            }
            TallyError::Weight(w) => write!(f, "weight {w} is not 1 to 127"),
            TallyError::SizeMismatch { expected, found } => write!(
                f,
                "tally of {} bits vs tally of {} bits",
                expected.bits(),
                found.bits()
            ),
            TallyError::NoVoices => write!(f, "subtracting a vote the tally does not hold"),
        }
    }
}

impl std::error::Error for TallyError {}

/// One signed counter per bit of a Monade of one size.
///
/// Counters are stored apart from any bits (a structure of arrays): counter
/// `64 · i + b` belongs to bit `b` of word `i`.
///
/// # Example
///
/// ```
/// use monade_core::size::Size;
/// use monade_core::tally::Tally;
/// use monade_core::word::Word;
/// let mut t = Tally::new(Size::WORD);
/// for v in [0b110, 0b011, 0b010] {
///     t.add(&[Word::new(v)]).unwrap();
/// }
/// assert_eq!(t.read(), vec![Word::new(0b010)]); // the majority
/// assert_eq!(&t.counts()[..3], &[-1, 3, -1]);
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Tally {
    size: Size,
    counts: Vec<i8>,
    voices: Voices,
    exact: bool,
}

/// `c + delta`, clamped to ±LIMIT; the flag says whether it was clamped.
fn clamp(c: i8, delta: i16) -> (i8, bool) {
    let raw = i16::from(c) + delta;
    let limit = i16::from(LIMIT);
    let out = raw.clamp(-limit, limit);
    // `out` is within ±127, so the conversion never fails.
    (i8::try_from(out).unwrap_or(0), out != raw)
}

impl Tally {
    /// An empty tally: all counters 0, no voices. It reads as point 0.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// use monade_core::tally::Tally;
    /// use monade_core::word::Word;
    /// let t = Tally::new(Size::BLOCK);
    /// assert_eq!(t.counts().len(), 512);
    /// assert_eq!(t.read(), vec![Word::ZERO; 8]);
    /// ```
    pub fn new(size: Size) -> Tally {
        Tally {
            size,
            counts: vec![0; size.words() * 64],
            voices: Voices::default(),
            exact: true,
        }
    }

    /// The size of the Monade this tally counts.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// use monade_core::tally::Tally;
    /// assert_eq!(Tally::new(Size::SYMBOL).size(), Size::SYMBOL);
    /// ```
    pub fn size(&self) -> Size {
        self.size
    }

    /// The counters, one per bit (read-only): the memory itself.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// use monade_core::tally::Tally;
    /// use monade_core::word::Word;
    /// let mut t = Tally::new(Size::WORD);
    /// t.add(&[Word::new(1)]).unwrap();
    /// assert_eq!(t.counts()[0], 1);
    /// assert_eq!(t.counts()[1], -1);
    /// ```
    pub fn counts(&self) -> &[i8] {
        &self.counts
    }

    /// How many votes, and their total weight.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// use monade_core::tally::Tally;
    /// use monade_core::word::Word;
    /// let mut t = Tally::new(Size::WORD);
    /// t.add(&[Word::ZERO]).unwrap();
    /// t.add_weighted(&[Word::ZERO], 4).unwrap();
    /// assert_eq!((t.voices().count, t.voices().weight), (2, 5));
    /// ```
    pub fn voices(&self) -> Voices {
        self.voices
    }

    /// True while every counter is the exact sum of its votes: nothing was ever
    /// clamped at ±127 and nothing was decayed. Then `add` and `subtract` are
    /// exact inverses and the reading equals the majority of the votes.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// use monade_core::tally::Tally;
    /// use monade_core::word::Word;
    /// let mut t = Tally::new(Size::WORD);
    /// t.add_weighted(&[Word::ZERO], 127).unwrap();
    /// assert!(t.is_exact());
    /// t.add(&[Word::ZERO]).unwrap(); // -128 is beyond the limit: clamped
    /// assert!(!t.is_exact());
    /// assert_eq!(t.counts()[0], -127);
    /// ```
    pub fn is_exact(&self) -> bool {
        self.exact
    }

    fn check_len(&self, v: &[Word]) -> Result<(), TallyError> {
        let expected = self.size.words();
        if v.len() == expected {
            Ok(())
        } else {
            Err(TallyError::WrongLength {
                expected,
                found: v.len(),
            })
        }
    }

    fn check_weight(w: u8) -> Result<i16, TallyError> {
        if (1..=LIMIT.unsigned_abs()).contains(&w) {
            Ok(i16::from(w))
        } else {
            Err(TallyError::Weight(w))
        }
    }

    /// Adds `sign · w` on the 1-bits of `v` and `-sign · w` on its 0-bits.
    fn vote(&mut self, v: &[Word], delta: i16) {
        let mut clamped = false;
        for (word, counts) in v.iter().zip(self.counts.chunks_exact_mut(64)) {
            let bits = word.bits();
            for (b, c) in counts.iter_mut().enumerate() {
                let d = if (bits >> b) & 1 == 1 { delta } else { -delta };
                let (out, cl) = clamp(*c, d);
                *c = out;
                clamped |= cl;
            }
        }
        self.exact &= !clamped;
    }

    /// One vote of weight 1: +1 on the 1-bits of `v`, -1 on its 0-bits.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// use monade_core::tally::Tally;
    /// use monade_core::word::Word;
    /// let mut t = Tally::new(Size::WORD);
    /// t.add(&[Word::new(0b1)]).unwrap();
    /// t.add(&[Word::new(0b1)]).unwrap();
    /// assert_eq!(t.counts()[0], 2);
    /// assert_eq!(t.counts()[1], -2);
    /// ```
    pub fn add(&mut self, v: &[Word]) -> Result<(), TallyError> {
        self.add_weighted(v, 1)
    }

    /// One vote of integer weight `w` (1 to 127): `+w` on the 1-bits, `-w` on
    /// the 0-bits. Counters saturate at ±127 and never wrap around.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// use monade_core::tally::{Tally, TallyError};
    /// use monade_core::word::Word;
    /// let mut t = Tally::new(Size::WORD);
    /// t.add_weighted(&[Word::new(1)], 5).unwrap();
    /// assert_eq!(t.counts()[0], 5);
    /// assert_eq!(t.add_weighted(&[Word::new(1)], 0), Err(TallyError::Weight(0)));
    /// assert_eq!(t.add_weighted(&[Word::new(1)], 200), Err(TallyError::Weight(200)));
    /// ```
    pub fn add_weighted(&mut self, v: &[Word], w: u8) -> Result<(), TallyError> {
        self.check_len(v)?;
        let delta = Tally::check_weight(w)?;
        self.vote(v, delta);
        self.voices.count = self.voices.count.saturating_add(1);
        self.voices.weight = self.voices.weight.saturating_add(u64::from(w));
        Ok(())
    }

    /// Removes one vote of weight 1: the exact inverse of [`add`](Tally::add)
    /// while the tally [is exact](Tally::is_exact).
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// use monade_core::tally::Tally;
    /// use monade_core::word::Word;
    /// let mut t = Tally::new(Size::WORD);
    /// t.add(&[Word::new(9)]).unwrap();
    /// let before = t.clone();
    /// t.add(&[Word::new(5)]).unwrap();
    /// t.subtract(&[Word::new(5)]).unwrap();
    /// assert_eq!(t, before);
    /// ```
    pub fn subtract(&mut self, v: &[Word]) -> Result<(), TallyError> {
        self.subtract_weighted(v, 1)
    }

    /// Removes one vote of weight `w`. Refused (`NoVoices`) if the tally does
    /// not hold at least one vote and `w` of weight.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// use monade_core::tally::{Tally, TallyError};
    /// use monade_core::word::Word;
    /// let mut t = Tally::new(Size::WORD);
    /// assert_eq!(t.subtract(&[Word::ZERO]), Err(TallyError::NoVoices));
    /// t.add_weighted(&[Word::ZERO], 3).unwrap();
    /// t.subtract_weighted(&[Word::ZERO], 3).unwrap();
    /// assert_eq!(t, Tally::new(Size::WORD));
    /// ```
    pub fn subtract_weighted(&mut self, v: &[Word], w: u8) -> Result<(), TallyError> {
        self.check_len(v)?;
        let delta = Tally::check_weight(w)?;
        if self.voices.count == 0 || self.voices.weight < u64::from(w) {
            return Err(TallyError::NoVoices);
        }
        self.vote(v, -delta);
        self.voices.count -= 1;
        self.voices.weight -= u64::from(w);
        Ok(())
    }

    /// The reading: bit set where the counter is positive; 0 where it is
    /// negative or tied (point 0).
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// use monade_core::tally::Tally;
    /// use monade_core::word::Word;
    /// let mut t = Tally::new(Size::WORD);
    /// t.add(&[Word::new(0b11)]).unwrap();
    /// t.add(&[Word::new(0b01)]).unwrap();
    /// assert_eq!(t.read(), vec![Word::new(0b01)]); // bit 1 is tied: 0
    /// ```
    pub fn read(&self) -> Vec<Word> {
        self.mask(|c| c > 0)
    }

    /// Bits where `|counter| < threshold`: where the voices disagree. A
    /// threshold of 1 marks exact ties; 0 marks nothing.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// use monade_core::tally::Tally;
    /// use monade_core::word::Word;
    /// let mut t = Tally::new(Size::WORD);
    /// t.add(&[Word::new(0b11)]).unwrap();
    /// t.add(&[Word::new(0b01)]).unwrap();
    /// assert_eq!(t.uncertain(1), vec![Word::new(0b10)]);
    /// assert_eq!(t.uncertain(0), vec![Word::ZERO]);
    /// ```
    pub fn uncertain(&self, threshold: u8) -> Vec<Word> {
        self.mask(|c| c.unsigned_abs() < threshold)
    }

    fn mask(&self, f: impl Fn(i8) -> bool) -> Vec<Word> {
        self.counts
            .chunks_exact(64)
            .map(|counts| {
                let mut w = 0u64;
                for (b, c) in counts.iter().enumerate() {
                    if f(*c) {
                        w |= 1 << b;
                    }
                }
                Word::new(w)
            })
            .collect()
    }

    /// How many counters have each certainty `|counter|`, from 0 to 127.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// use monade_core::tally::Tally;
    /// use monade_core::word::Word;
    /// let mut t = Tally::new(Size::WORD);
    /// t.add(&[Word::new(0b11)]).unwrap();
    /// t.add(&[Word::new(0b01)]).unwrap();
    /// let h = t.certainty_histogram();
    /// assert_eq!((h[0], h[2]), (1, 63));
    /// ```
    pub fn certainty_histogram(&self) -> [u64; 128] {
        let mut h = [0u64; 128];
        for c in &self.counts {
            if let Some(slot) = h.get_mut(usize::from(c.unsigned_abs())) {
                *slot += 1;
            }
        }
        h
    }

    /// The residue of a voice: `voice ⊕ reading`, the bits of that voice the
    /// majority discarded (Mutation's base form).
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// use monade_core::tally::Tally;
    /// use monade_core::word::Word;
    /// let mut t = Tally::new(Size::WORD);
    /// for v in [0b110, 0b011, 0b010] {
    ///     t.add(&[Word::new(v)]).unwrap();
    /// }
    /// assert_eq!(t.residue(&[Word::new(0b110)]), Ok(vec![Word::new(0b100)]));
    /// ```
    pub fn residue(&self, voice: &[Word]) -> Result<Vec<Word>, TallyError> {
        self.check_len(voice)?;
        Ok(voice
            .iter()
            .zip(self.read())
            .map(|(v, r)| v.xor(r))
            .collect())
    }

    /// Adds the counters and voices of `other` into this tally (saturating).
    /// Merging is commutative, and, while nothing saturates, merging two halves
    /// equals counting all the votes in one tally: two cores can count apart.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// use monade_core::tally::Tally;
    /// use monade_core::word::Word;
    /// let (mut a, mut b, mut all) = (Tally::new(Size::WORD), Tally::new(Size::WORD), Tally::new(Size::WORD));
    /// a.add(&[Word::new(3)]).unwrap();
    /// b.add(&[Word::new(5)]).unwrap();
    /// all.add(&[Word::new(5)]).unwrap();
    /// all.add(&[Word::new(3)]).unwrap();
    /// a.merge(&b).unwrap();
    /// assert_eq!(a, all);
    /// ```
    pub fn merge(&mut self, other: &Tally) -> Result<(), TallyError> {
        if other.size != self.size {
            return Err(TallyError::SizeMismatch {
                expected: self.size,
                found: other.size,
            });
        }
        let mut clamped = false;
        for (c, o) in self.counts.iter_mut().zip(&other.counts) {
            let (out, cl) = clamp(*c, i16::from(*o));
            *c = out;
            clamped |= cl;
        }
        self.exact = self.exact && other.exact && !clamped;
        self.voices.count = self.voices.count.saturating_add(other.voices.count);
        self.voices.weight = self.voices.weight.saturating_add(other.voices.weight);
        Ok(())
    }

    /// Forgetting (sleep): moves every counter `k` towards 0, never across it.
    /// A counter at 0 stays 0; no sign ever changes. Returns how many counters
    /// reached 0 now, i.e. how many bits returned to point 0.
    ///
    /// The voices are kept: they say what was heard, not what remains.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// use monade_core::tally::Tally;
    /// use monade_core::word::Word;
    /// let mut t = Tally::new(Size::WORD);
    /// t.add_weighted(&[Word::new(1)], 3).unwrap();
    /// assert_eq!(t.decay(2), 0);
    /// assert_eq!((t.counts()[0], t.counts()[1]), (1, -1));
    /// assert_eq!(t.decay(5), 64);
    /// assert_eq!(t.counts()[0], 0);
    /// ```
    pub fn decay(&mut self, k: u8) -> u64 {
        if k == 0 {
            return 0;
        }
        let k = i16::from(k);
        let mut erased = 0u64;
        for c in &mut self.counts {
            if *c == 0 {
                continue;
            }
            let v = i16::from(*c);
            let out = if v > 0 {
                (v - k).max(0)
            } else {
                (v + k).min(0)
            };
            // `out` lies between 0 and the old value, so it fits.
            *c = i8::try_from(out).unwrap_or(0);
            erased += u64::from(out == 0);
            self.exact = false;
        }
        erased
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle;
    use crate::word;
    use proptest::prelude::*;

    /// Sizes small enough for property tests; 2 MB has its own test below.
    fn size() -> impl Strategy<Value = Size> {
        prop_oneof![Just(Size::WORD), Just(Size::BLOCK), Just(Size::SYMBOL)]
    }

    fn monade(s: Size) -> impl Strategy<Value = Vec<Word>> {
        proptest::collection::vec(any::<u64>().prop_map(Word::new), s.words())
    }

    /// A size and `k` votes of that size, `k` in `lo..hi`.
    fn votes(lo: usize, hi: usize) -> impl Strategy<Value = (Size, Vec<Vec<Word>>)> {
        size().prop_flat_map(move |s| (Just(s), proptest::collection::vec(monade(s), lo..hi)))
    }

    fn tally_of(s: Size, vs: &[Vec<Word>]) -> Tally {
        let mut t = Tally::new(s);
        for v in vs {
            t.add(v).unwrap();
        }
        t
    }

    #[test]
    fn errors_are_values_and_leave_the_tally_unchanged() {
        let mut t = Tally::new(Size::BLOCK);
        t.add(&[Word::new(7); 8]).unwrap();
        let before = t.clone();
        let short = [Word::ZERO; 7];
        assert!(t.add(&short).is_err());
        assert!(t.subtract(&short).is_err());
        assert!(t.residue(&short).is_err());
        assert!(t.add_weighted(&[Word::ZERO; 8], 0).is_err());
        assert!(t.add_weighted(&[Word::ZERO; 8], 128).is_err());
        assert!(t.subtract_weighted(&[Word::ZERO; 8], 2).is_err());
        assert!(t.merge(&Tally::new(Size::WORD)).is_err());
        assert_eq!(t, before);
        assert_eq!(
            Tally::new(Size::WORD).merge(&Tally::new(Size::BLOCK)),
            Err(TallyError::SizeMismatch {
                expected: Size::WORD,
                found: Size::BLOCK
            })
        );
    }

    #[test]
    fn saturation_never_wraps() {
        let v = vec![Word::new(0xF0F0_F0F0_F0F0_F0F0); 8];
        let mut t = Tally::new(Size::BLOCK);
        for _ in 0..300 {
            t.add_weighted(&v, LIMIT.unsigned_abs()).unwrap();
        }
        assert!(t.counts().iter().all(|c| c.unsigned_abs() == 127));
        assert_eq!(t.read(), v);
        assert!(!t.is_exact());
        assert_eq!(t.voices().count, 300);
    }

    #[test]
    fn a_thought_of_2_mb_reads_its_majority() {
        let s = Size::THOUGHT;
        let vs: Vec<Vec<Word>> = (1..=3u64)
            .map(|k| {
                (0..s.words() as u64)
                    .map(|i| Word::new((i ^ k).wrapping_mul(0x9e37_79b9_7f4a_7c15)))
                    .collect()
            })
            .collect();
        let t = tally_of(s, &vs);
        let refs: Vec<&[Word]> = vs.iter().map(Vec::as_slice).collect();
        let mut m = vec![Word::ZERO; s.words()];
        word::majority(&refs, &mut m).unwrap();
        assert_eq!(t.read(), m);
        assert_eq!(t.counts().len(), 16_777_216);
    }

    proptest! {
        /// Invariant 1: the reading of plain votes is the majority (ties → 0).
        #[test]
        fn reading_equals_majority((s, vs) in votes(1, 12)) {
            let t = tally_of(s, &vs);
            let refs: Vec<&[Word]> = vs.iter().map(Vec::as_slice).collect();
            let mut m = vec![Word::ZERO; s.words()];
            word::majority(&refs, &mut m).unwrap();
            prop_assert_eq!(t.read(), m);
            prop_assert!(t.is_exact());
        }

        /// Invariant 2: order-free; merging two halves = all votes in one tally.
        #[test]
        fn order_free_and_merge((s, vs) in votes(0, 12), cut in 0usize..12, turn in 0usize..12) {
            let all = tally_of(s, &vs);
            let mut rotated = vs.clone();
            rotated.reverse();
            if !rotated.is_empty() {
                let n = rotated.len();
                rotated.rotate_left(turn % n);
            }
            prop_assert_eq!(&tally_of(s, &rotated), &all);
            let cut = cut.min(vs.len());
            let mut a = tally_of(s, &vs[..cut]);
            let b = tally_of(s, &vs[cut..]);
            let mut b2 = b.clone();
            a.merge(&b).unwrap();
            prop_assert_eq!(&a, &all);
            b2.merge(&tally_of(s, &vs[..cut])).unwrap();
            prop_assert_eq!(&b2, &all);
        }

        /// Merge is commutative even when it saturates; the empty tally is neutral.
        #[test]
        fn merge_is_commutative((s, vs) in votes(2, 6), w in 1u8..=127) {
            let mut a = Tally::new(s);
            let mut b = Tally::new(s);
            for (i, v) in vs.iter().enumerate() {
                if i % 2 == 0 { a.add_weighted(v, w).unwrap() } else { b.add_weighted(v, w).unwrap() }
            }
            let mut ab = a.clone();
            ab.merge(&b).unwrap();
            let mut ba = b.clone();
            ba.merge(&a).unwrap();
            prop_assert_eq!(ab.counts(), ba.counts());
            let mut a0 = a.clone();
            a0.merge(&Tally::new(s)).unwrap();
            prop_assert_eq!(a0, a);
        }

        /// Invariant 3: add then subtract gives the original, while exact.
        #[test]
        fn subtract_inverts_add((s, vs) in votes(1, 8), w in 1u8..=60) {
            let base = tally_of(s, &vs[1..]);
            let mut t = base.clone();
            t.add_weighted(&vs[0], w).unwrap();
            prop_assert!(t.is_exact());
            t.subtract_weighted(&vs[0], w).unwrap();
            prop_assert_eq!(t, base);
        }

        /// Invariant 4: decay moves every counter k towards 0, never across it.
        #[test]
        fn decay_never_flips_a_sign((s, vs) in votes(0, 10), w in 1u8..=127, k in 0u8..=255) {
            let mut t = Tally::new(s);
            for v in &vs {
                t.add_weighted(v, w).unwrap();
            }
            let before = t.counts().to_vec();
            let reached = t.decay(k);
            let mut zeros = 0u64;
            for (b, a) in before.iter().zip(t.counts()) {
                prop_assert!(a.signum() == b.signum() || *a == 0);
                prop_assert_eq!(a.unsigned_abs(), b.unsigned_abs().saturating_sub(k));
                zeros += u64::from(*b != 0 && *a == 0);
            }
            prop_assert_eq!(reached, zeros);
        }

        /// Invariants 5 and 6: fast = slow, with weights, subtraction and
        /// saturation, against the bit-by-bit oracle.
        #[test]
        fn fast_equals_slow(
            (s, vs) in votes(1, 10),
            ws in proptest::collection::vec(1u8..=127, 10),
            signs in proptest::collection::vec(any::<bool>(), 10),
        ) {
            let mut t = Tally::new(s);
            let mut steps: Vec<(&[Word], i64)> = Vec::new();
            for (i, v) in vs.iter().enumerate() {
                let w = ws[i];
                if signs[i] && t.voices().count > 0 && t.voices().weight >= u64::from(w) {
                    t.subtract_weighted(v, w).unwrap();
                    steps.push((v, -i64::from(w)));
                } else {
                    t.add_weighted(v, w).unwrap();
                    steps.push((v, i64::from(w)));
                }
            }
            let slow = oracle::tally(s.words(), &steps);
            let fast: Vec<i64> = t.counts().iter().map(|c| i64::from(*c)).collect();
            prop_assert_eq!(&fast, &slow);
            prop_assert_eq!(t.read(), oracle::tally_read(&slow));
            prop_assert!(fast.iter().all(|c| c.abs() <= 127));
        }

        /// Residue = voice ⊕ reading; uncertain = |counter| < threshold.
        #[test]
        fn residue_and_uncertain((s, vs) in votes(1, 9), th in 0u8..10) {
            let t = tally_of(s, &vs);
            let read = t.read();
            for v in &vs {
                prop_assert_eq!(t.residue(v).unwrap(), oracle::xor(v, &read));
            }
            let counts: Vec<i64> = t.counts().iter().map(|c| i64::from(*c)).collect();
            prop_assert_eq!(t.uncertain(th), oracle::tally_uncertain(&counts, i64::from(th)));
            let h = t.certainty_histogram();
            prop_assert_eq!(h.iter().sum::<u64>(), s.bits());
        }

        /// Invariant 7: any input of the wrong length is an error, never a panic.
        #[test]
        fn wrong_lengths_are_errors(s in size(), n in 0usize..80) {
            let v = vec![Word::ZERO; n];
            let mut t = Tally::new(s);
            let ok = n == s.words();
            prop_assert_eq!(t.add(&v).is_ok(), ok);
            prop_assert_eq!(t.residue(&v).is_ok(), ok);
        }
    }
}
