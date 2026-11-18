//! The word, the block and the bit operations.
//!
//! A [`Word`] is 64 bits with three views: bits (`u64`), bytes (`[u8; 8]`),
//! and reference ([`Ref`], an index into an arena). A [`Block`] is 8 words
//! aligned to one 64-byte cache line.
//!
//! The slice functions ([`xor_assign`], [`and_assign`], [`popcount`],
//! [`hamming`], [`majority`]) work on any number of words, so the same
//! operations serve every size of the Monade, from one word to 2 MB.

use core::fmt;

/// A 64-bit word: the atom of the Monade.
///
/// # Example
///
/// ```
/// use monade_core::word::Word;
/// let w = Word::new(0b1011);
/// assert_eq!(w.bits(), 11);
/// assert_eq!(w.popcount(), 3);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
#[repr(transparent)]
pub struct Word(u64);

impl Word {
    /// The zero word: point 0 at word scale, the neutral element of XOR.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::Word;
    /// let v = Word::new(42);
    /// assert_eq!(v.xor(Word::ZERO), v);
    /// ```
    pub const ZERO: Word = Word(0);

    /// Makes a word from its 64-bit view.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::Word;
    /// assert_eq!(Word::new(7).bits(), 7);
    /// ```
    pub const fn new(bits: u64) -> Word {
        Word(bits)
    }

    /// The 64-bit view.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::Word;
    /// assert_eq!(Word::new(u64::MAX).bits(), u64::MAX);
    /// ```
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// The 8-byte view, little-endian on every machine.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::Word;
    /// assert_eq!(Word::new(1).to_bytes(), [1, 0, 0, 0, 0, 0, 0, 0]);
    /// ```
    pub const fn to_bytes(self) -> [u8; 8] {
        self.0.to_le_bytes()
    }

    /// Makes a word from its 8-byte view (little-endian).
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::Word;
    /// let w = Word::new(0x0102_0304_0506_0708);
    /// assert_eq!(Word::from_bytes(w.to_bytes()), w);
    /// ```
    pub const fn from_bytes(bytes: [u8; 8]) -> Word {
        Word(u64::from_le_bytes(bytes))
    }

    /// The reference view: `None` if the word does not fit a 32-bit index.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::{Ref, Word};
    /// assert_eq!(Word::new(5).to_ref(), Some(Ref::new(5)));
    /// assert_eq!(Word::new(u64::MAX).to_ref(), None);
    /// ```
    pub fn to_ref(self) -> Option<Ref> {
        u32::try_from(self.0).ok().map(Ref)
    }

    /// Makes a word from a reference.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::{Ref, Word};
    /// assert_eq!(Word::from_ref(Ref::new(9)).bits(), 9);
    /// ```
    pub const fn from_ref(r: Ref) -> Word {
        Word(r.0 as u64)
    }

    /// Bit-wise XOR: binding.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::Word;
    /// assert_eq!(Word::new(0b1100).xor(Word::new(0b1010)), Word::new(0b0110));
    /// ```
    pub const fn xor(self, other: Word) -> Word {
        Word(self.0 ^ other.0)
    }

    /// Bit-wise AND: overlap.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::Word;
    /// assert_eq!(Word::new(0b1100).and(Word::new(0b1010)), Word::new(0b1000));
    /// ```
    pub const fn and(self, other: Word) -> Word {
        Word(self.0 & other.0)
    }

    /// Number of bits set.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::Word;
    /// assert_eq!(Word::new(0xFF).popcount(), 8);
    /// ```
    pub const fn popcount(self) -> u64 {
        self.0.count_ones() as u64
    }

    /// Hamming distance: the number of differing bits, `popcount(a ⊕ b)`.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::Word;
    /// assert_eq!(Word::new(0b1100).hamming(Word::new(0b1010)), 2);
    /// ```
    pub const fn hamming(self, other: Word) -> u64 {
        self.xor(other).popcount()
    }
}

/// A reference: a 32-bit index into an arena. `Ref::ORIGIN` (0) is point 0.
///
/// # Example
///
/// ```
/// use monade_core::word::Ref;
/// assert_eq!(Ref::new(3).index(), 3);
/// assert_eq!(Ref::ORIGIN.index(), 0);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Ref(u32);

impl Ref {
    /// Reference 0: the origin, point 0 as a coordinate.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::Ref;
    /// assert_eq!(Ref::ORIGIN, Ref::new(0));
    /// ```
    pub const ORIGIN: Ref = Ref(0);

    /// Makes a reference from its index.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::Ref;
    /// assert_eq!(Ref::new(1).index(), 1);
    /// ```
    pub const fn new(index: u32) -> Ref {
        Ref(index)
    }

    /// The index, for slicing.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::Ref;
    /// let v = [10, 20, 30];
    /// assert_eq!(v[Ref::new(2).index()], 30);
    /// ```
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Eight words aligned to 64 bytes: one cache line, 512 bits.
///
/// # Example
///
/// ```
/// use monade_core::word::{Block, Word};
/// let mut b = Block::ZERO;
/// b.0[3] = Word::new(1);
/// assert_eq!(b.popcount(), 1);
/// assert_eq!(core::mem::align_of::<Block>(), 64);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
#[repr(C, align(64))]
pub struct Block(pub [Word; 8]);

impl Block {
    /// The zero block.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::Block;
    /// assert_eq!(Block::ZERO.popcount(), 0);
    /// ```
    pub const ZERO: Block = Block([Word::ZERO; 8]);

    /// Word-wise XOR of two blocks.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::{Block, Word};
    /// let a = Block([Word::new(3); 8]);
    /// assert_eq!(a.xor(&a), Block::ZERO);
    /// ```
    pub fn xor(&self, other: &Block) -> Block {
        let mut out = *self;
        for (o, b) in out.0.iter_mut().zip(other.0) {
            *o = o.xor(b);
        }
        out
    }

    /// Word-wise AND of two blocks.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::{Block, Word};
    /// let a = Block([Word::new(3); 8]);
    /// assert_eq!(a.and(&Block::ZERO), Block::ZERO);
    /// ```
    pub fn and(&self, other: &Block) -> Block {
        let mut out = *self;
        for (o, b) in out.0.iter_mut().zip(other.0) {
            *o = o.and(b);
        }
        out
    }

    /// Number of bits set in the block.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::{Block, Word};
    /// assert_eq!(Block([Word::new(1); 8]).popcount(), 8);
    /// ```
    pub fn popcount(&self) -> u64 {
        popcount(&self.0)
    }

    /// Hamming distance between two blocks.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::word::{Block, Word};
    /// let a = Block([Word::new(1); 8]);
    /// assert_eq!(a.hamming(&Block::ZERO), 8);
    /// ```
    pub fn hamming(&self, other: &Block) -> u64 {
        self.xor(other).popcount()
    }
}

/// Errors of the slice operations.
///
/// # Example
///
/// ```
/// use monade_core::word::{hamming, Word, WordError};
/// let e = hamming(&[Word::ZERO], &[]);
/// assert_eq!(e, Err(WordError::LengthMismatch { expected: 1, found: 0 }));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WordError {
    /// Two operands have different numbers of words.
    LengthMismatch {
        /// Words in the first operand (or the output).
        expected: usize,
        /// Words in the other operand.
        found: usize,
    },
    /// Majority of nothing.
    NoInputs,
}

impl fmt::Display for WordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WordError::LengthMismatch { expected, found } => {
                write!(f, "length mismatch: {expected} words vs {found}")
            }
            WordError::NoInputs => write!(f, "majority of no inputs"),
        }
    }
}

impl std::error::Error for WordError {}

fn same_len(a: usize, b: usize) -> Result<(), WordError> {
    if a == b {
        Ok(())
    } else {
        Err(WordError::LengthMismatch {
            expected: a,
            found: b,
        })
    }
}

/// `a ⊕= b`, word by word.
///
/// # Example
///
/// ```
/// use monade_core::word::{xor_assign, Word};
/// let mut a = [Word::new(0b11), Word::new(1)];
/// xor_assign(&mut a, &[Word::new(0b01), Word::new(1)]).unwrap();
/// assert_eq!(a, [Word::new(0b10), Word::ZERO]);
/// ```
pub fn xor_assign(a: &mut [Word], b: &[Word]) -> Result<(), WordError> {
    same_len(a.len(), b.len())?;
    for (x, y) in a.iter_mut().zip(b) {
        *x = x.xor(*y);
    }
    Ok(())
}

/// `a &= b`, word by word.
///
/// # Example
///
/// ```
/// use monade_core::word::{and_assign, Word};
/// let mut a = [Word::new(0b11)];
/// and_assign(&mut a, &[Word::new(0b01)]).unwrap();
/// assert_eq!(a, [Word::new(0b01)]);
/// ```
pub fn and_assign(a: &mut [Word], b: &[Word]) -> Result<(), WordError> {
    same_len(a.len(), b.len())?;
    for (x, y) in a.iter_mut().zip(b) {
        *x = x.and(*y);
    }
    Ok(())
}

/// Number of bits set in a run of words.
///
/// # Example
///
/// ```
/// use monade_core::word::{popcount, Word};
/// assert_eq!(popcount(&[Word::new(1), Word::new(3)]), 3);
/// ```
pub fn popcount(v: &[Word]) -> u64 {
    v.iter().map(|w| w.popcount()).sum()
}

/// Hamming distance between two runs of words of equal length.
///
/// # Example
///
/// ```
/// use monade_core::word::{hamming, Word};
/// assert_eq!(hamming(&[Word::new(1), Word::ZERO], &[Word::ZERO, Word::new(1)]), Ok(2));
/// ```
pub fn hamming(a: &[Word], b: &[Word]) -> Result<u64, WordError> {
    same_len(a.len(), b.len())?;
    Ok(a.iter().zip(b).map(|(x, y)| x.hamming(*y)).sum())
}

/// Bit-wise majority of `inputs` written into `out`.
///
/// A bit is set when more than half of the inputs have it set; a tie gives 0.
///
/// # Example
///
/// ```
/// use monade_core::word::{majority, Word};
/// let (a, b, c) = ([Word::new(0b110)], [Word::new(0b011)], [Word::new(0b010)]);
/// let mut out = [Word::ZERO];
/// majority(&[&a, &b, &c], &mut out).unwrap();
/// assert_eq!(out, [Word::new(0b010)]);
/// ```
pub fn majority(inputs: &[&[Word]], out: &mut [Word]) -> Result<(), WordError> {
    if inputs.is_empty() {
        return Err(WordError::NoInputs);
    }
    for v in inputs {
        same_len(out.len(), v.len())?;
    }
    let n = inputs.len() as u64;
    for (i, o) in out.iter_mut().enumerate() {
        let mut counts = [0u64; 64];
        for v in inputs {
            let mut w = v[i].bits();
            while w != 0 {
                counts[w.trailing_zeros() as usize] += 1;
                w &= w - 1;
            }
        }
        let mut m = 0u64;
        for (bit, c) in counts.iter().enumerate() {
            if 2 * c > n {
                m |= 1 << bit;
            }
        }
        *o = Word::new(m);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle;
    use proptest::prelude::*;

    fn word() -> impl Strategy<Value = Word> {
        any::<u64>().prop_map(Word::new)
    }

    fn words(n: usize) -> impl Strategy<Value = Vec<Word>> {
        proptest::collection::vec(word(), n)
    }

    #[test]
    fn block_is_one_cache_line() {
        assert_eq!(core::mem::size_of::<Block>(), 64);
        assert_eq!(core::mem::align_of::<Block>(), 64);
    }

    #[test]
    fn errors_are_values() {
        assert_eq!(majority(&[], &mut []), Err(WordError::NoInputs));
        let mut a = [Word::ZERO];
        assert!(xor_assign(&mut a, &[]).is_err());
        assert!(and_assign(&mut a, &[]).is_err());
    }

    proptest! {
        #[test]
        fn zero_is_neutral(v in word()) {
            prop_assert_eq!(v.xor(Word::ZERO), v);
        }

        #[test]
        fn xor_is_its_own_inverse(a in word(), b in word()) {
            prop_assert_eq!(a.xor(b).xor(b), a);
        }

        #[test]
        fn xor_commutes(a in word(), b in word()) {
            prop_assert_eq!(a.xor(b), b.xor(a));
        }

        #[test]
        fn self_binding_is_point_zero(a in word()) {
            prop_assert_eq!(a.xor(a), Word::ZERO);
        }

        #[test]
        fn hamming_is_a_metric(a in word(), b in word(), c in word()) {
            prop_assert_eq!(a.hamming(b), b.hamming(a));
            prop_assert_eq!(a.hamming(a), 0);
            prop_assert!(a.hamming(c) <= a.hamming(b) + b.hamming(c));
            prop_assert_eq!(a.hamming(b), a.xor(b).popcount());
        }

        #[test]
        fn views_round_trip(a in word(), r in any::<u32>()) {
            prop_assert_eq!(Word::from_bytes(a.to_bytes()), a);
            prop_assert_eq!(Word::from_ref(Ref::new(r)).to_ref(), Some(Ref::new(r)));
        }

        #[test]
        fn fast_equals_slow_on_words(a in word(), b in word()) {
            prop_assert_eq!(a.xor(b), oracle::xor_word(a, b));
            prop_assert_eq!(a.and(b), oracle::and_word(a, b));
            prop_assert_eq!(a.popcount(), oracle::popcount_word(a));
            prop_assert_eq!(a.hamming(b), oracle::hamming_word(a, b));
        }

        #[test]
        fn fast_equals_slow_on_slices(
            (a, b) in (0usize..20).prop_flat_map(|n| (words(n), words(n)))
        ) {
            prop_assert_eq!(hamming(&a, &b), Ok(oracle::hamming(&a, &b)));
            prop_assert_eq!(popcount(&a), oracle::popcount(&a));
            let mut x = a.clone();
            xor_assign(&mut x, &b).unwrap();
            prop_assert_eq!(&x, &oracle::xor(&a, &b));
            let mut y = a.clone();
            and_assign(&mut y, &b).unwrap();
            prop_assert_eq!(&y, &oracle::and(&a, &b));
        }

        #[test]
        fn fast_equals_slow_on_blocks(a in words(8), b in words(8)) {
            let (ba, bb) = (Block(a.clone().try_into().unwrap()), Block(b.clone().try_into().unwrap()));
            prop_assert_eq!(ba.hamming(&bb), oracle::hamming(&a, &b));
            prop_assert_eq!(ba.xor(&bb).0.to_vec(), oracle::xor(&a, &b));
            prop_assert_eq!(ba.and(&bb).0.to_vec(), oracle::and(&a, &b));
        }

        #[test]
        fn majority_fast_equals_slow(
            vs in (1usize..9, 0usize..6)
                .prop_flat_map(|(k, n)| proptest::collection::vec(words(n), k))
        ) {
            let refs: Vec<&[Word]> = vs.iter().map(|v| v.as_slice()).collect();
            let mut out = vec![Word::ZERO; vs[0].len()];
            majority(&refs, &mut out).unwrap();
            prop_assert_eq!(out, oracle::majority(&refs));
        }

        #[test]
        fn majority_of_one_is_identity(v in words(4)) {
            let mut out = vec![Word::ZERO; 4];
            majority(&[&v], &mut out).unwrap();
            prop_assert_eq!(out, v);
        }
    }
}
