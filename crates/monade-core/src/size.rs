//! The sizes of the Monade and the arenas that hold them.
//!
//! A size is defined recursively: the word, or 8 of the size below. Seven
//! sizes, from 64 bits (8²) to 2 MB (8⁸ bits). Any other length is an error.
//! An [`Arena`] holds Monades of one size, side by side; a [`Ref`] is an
//! index into it. `Ref::ORIGIN` is point 0: the zero Monade, never data.

use crate::word::{Ref, Word};
use core::fmt;

/// A size of the Monade: `8^level` words, level 0 (word) to 6 (2 MB).
///
/// # Example
///
/// ```
/// use monade_core::size::Size;
/// assert_eq!(Size::BLOCK.words(), 8);
/// assert_eq!(Size::THOUGHT.bytes(), 2 * 1024 * 1024);
/// assert_eq!(Size::from_bits(4096), Ok(Size::SYMBOL));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Size(u8);

/// Errors of sizes and arenas.
///
/// # Example
///
/// ```
/// use monade_core::size::{Size, SizeError};
/// assert_eq!(Size::from_bits(100), Err(SizeError::NotASize(100)));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SizeError {
    /// Not 64 · 8^k bits with k from 0 to 6.
    NotASize(u64),
    /// Not a level from 0 to 6.
    NotALevel(u8),
    /// A Monade of the wrong length for this arena.
    WrongLength {
        /// Words of the arena's size.
        expected: usize,
        /// Words given.
        found: usize,
    },
    /// The zero Monade: point 0 holds no data.
    Zero,
    /// The arena has used every 32-bit reference.
    Full,
}

impl fmt::Display for SizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SizeError::NotASize(bits) => {
                write!(f, "{bits} bits is not a size (64 to 2 MB in powers of 8)")
            }
            SizeError::NotALevel(l) => write!(f, "level {l} is not 0 to 6"),
            SizeError::WrongLength { expected, found } => {
                write!(f, "expected {expected} words, found {found}")
            }
            SizeError::Zero => write!(f, "the zero Monade is point 0 and holds no data"),
            SizeError::Full => write!(f, "arena full"),
        }
    }
}

impl std::error::Error for SizeError {}

impl Size {
    /// 64 bits: measures, references, counters.
    pub const WORD: Size = Size(0);
    /// 512 bits: one cache line, for comparison.
    pub const BLOCK: Size = Size(1);
    /// 4096 bits: composition and large memories.
    pub const SYMBOL: Size = Size(2);
    /// 2 MB: the largest Monade, "a thought".
    pub const THOUGHT: Size = Size(6);

    /// The size of a level, 0 to 6.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// assert_eq!(Size::new(1), Ok(Size::BLOCK));
    /// assert!(Size::new(7).is_err());
    /// ```
    pub fn new(level: u8) -> Result<Size, SizeError> {
        if level <= Size::THOUGHT.0 {
            Ok(Size(level))
        } else {
            Err(SizeError::NotALevel(level))
        }
    }

    /// The size with this many bits.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// assert_eq!(Size::from_bits(64), Ok(Size::WORD));
    /// assert!(Size::from_bits(128).is_err());
    /// ```
    pub fn from_bits(bits: u64) -> Result<Size, SizeError> {
        let z = bits.trailing_zeros();
        if bits.is_power_of_two() && z.is_multiple_of(3) && (6..=24).contains(&z) {
            Ok(Size(((z - 6) / 3) as u8))
        } else {
            Err(SizeError::NotASize(bits))
        }
    }

    /// Level: 0 for the word, 6 for 2 MB.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// assert_eq!(Size::SYMBOL.level(), 2);
    /// ```
    pub const fn level(self) -> u8 {
        self.0
    }

    /// Words: `8^level`.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// assert_eq!(Size::SYMBOL.words(), 64);
    /// ```
    pub const fn words(self) -> usize {
        1 << (3 * self.0 as usize)
    }

    /// Bits: `64 · 8^level`.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// assert_eq!(Size::BLOCK.bits(), 512);
    /// ```
    pub const fn bits(self) -> u64 {
        64 * self.words() as u64
    }

    /// Bytes: `8 · 8^level`.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// assert_eq!(Size::BLOCK.bytes(), 64);
    /// ```
    pub const fn bytes(self) -> usize {
        8 * self.words()
    }

    /// The size below (one eighth), or `None` for the word.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// assert_eq!(Size::BLOCK.smaller(), Some(Size::WORD));
    /// assert_eq!(Size::WORD.smaller(), None);
    /// ```
    pub fn smaller(self) -> Option<Size> {
        self.0.checked_sub(1).map(Size)
    }

    /// The size above (8 of this one), or `None` above 2 MB.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::Size;
    /// assert_eq!(Size::WORD.larger(), Some(Size::BLOCK));
    /// assert_eq!(Size::THOUGHT.larger(), None);
    /// ```
    pub fn larger(self) -> Option<Size> {
        Size::new(self.0 + 1).ok()
    }
}

/// Monades of one size, side by side; references are indices.
///
/// Reference 0 is point 0: the zero Monade, present from the start.
///
/// # Example
///
/// ```
/// use monade_core::size::{Arena, Size};
/// use monade_core::word::{Ref, Word};
/// let mut a = Arena::new(Size::WORD);
/// let r = a.push(&[Word::new(7)]).unwrap();
/// assert_eq!(a.get(r), Some(&[Word::new(7)][..]));
/// assert_eq!(a.get(Ref::ORIGIN), Some(&[Word::ZERO][..]));
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Arena {
    size: Size,
    words: Vec<Word>,
}

impl Arena {
    /// An arena holding only point 0.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::{Arena, Size};
    /// assert_eq!(Arena::new(Size::BLOCK).len(), 1);
    /// ```
    pub fn new(size: Size) -> Arena {
        Arena {
            size,
            words: vec![Word::ZERO; size.words()],
        }
    }

    /// The size of its Monades.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::{Arena, Size};
    /// assert_eq!(Arena::new(Size::SYMBOL).size(), Size::SYMBOL);
    /// ```
    pub fn size(&self) -> Size {
        self.size
    }

    /// Number of Monades, point 0 included.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::{Arena, Size};
    /// assert_eq!(Arena::new(Size::WORD).len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.words.len() / self.size.words()
    }

    /// Always `false`: point 0 is always there.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::{Arena, Size};
    /// assert!(!Arena::new(Size::WORD).is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// Stores a Monade; returns its reference.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::{Arena, Size, SizeError};
    /// use monade_core::word::Word;
    /// let mut a = Arena::new(Size::WORD);
    /// assert_eq!(a.push(&[Word::ZERO]), Err(SizeError::Zero));
    /// assert!(a.push(&[Word::new(1), Word::new(2)]).is_err());
    /// ```
    pub fn push(&mut self, monade: &[Word]) -> Result<Ref, SizeError> {
        let expected = self.size.words();
        if monade.len() != expected {
            return Err(SizeError::WrongLength {
                expected,
                found: monade.len(),
            });
        }
        if monade.iter().all(|w| *w == Word::ZERO) {
            return Err(SizeError::Zero);
        }
        let index = u32::try_from(self.len()).map_err(|_| SizeError::Full)?;
        self.words.extend_from_slice(monade);
        Ok(Ref::new(index))
    }

    /// The Monade at a reference, or `None` outside the arena.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::size::{Arena, Size};
    /// use monade_core::word::Ref;
    /// assert_eq!(Arena::new(Size::WORD).get(Ref::new(1)), None);
    /// ```
    pub fn get(&self, r: Ref) -> Option<&[Word]> {
        let n = self.size.words();
        let start = r.index().checked_mul(n)?;
        self.words.get(start..start.checked_add(n)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle;
    use proptest::prelude::*;

    #[test]
    fn seven_sizes_from_64_bits_to_2_mb() {
        let bits: Vec<u64> = (0..=6).map(|l| Size::new(l).unwrap().bits()).collect();
        assert_eq!(bits, [64, 512, 4096, 32768, 262_144, 2_097_152, 16_777_216]);
        assert_eq!(Size::THOUGHT.bytes(), 2 << 20);
    }

    #[test]
    fn fast_equals_slow_for_every_size() {
        for l in 0..=6 {
            let s = Size::new(l).unwrap();
            assert_eq!(s.words(), oracle::size_words(s));
            assert_eq!(Size::from_bits(s.bits()), Ok(s));
            assert_eq!(oracle::size_from_bits(s.bits()), Some(s));
        }
    }

    #[test]
    fn a_thought_fits_an_arena() {
        let mut a = Arena::new(Size::THOUGHT);
        let mut m = vec![Word::ZERO; Size::THOUGHT.words()];
        m[0] = Word::new(1);
        let r = a.push(&m).unwrap();
        assert_eq!(a.get(r), Some(&m[..]));
    }

    proptest! {
        #[test]
        fn from_bits_fast_equals_slow(bits in prop_oneof![
            any::<u64>(),
            (0u32..64).prop_map(|z| 1u64 << z),
        ]) {
            prop_assert_eq!(Size::from_bits(bits).ok(), oracle::size_from_bits(bits));
        }

        #[test]
        fn every_other_length_is_an_error(bits in any::<u64>()) {
            match Size::from_bits(bits) {
                Ok(s) => prop_assert_eq!(s.bits(), bits),
                Err(e) => prop_assert_eq!(e, SizeError::NotASize(bits)),
            }
        }

        #[test]
        fn arena_fast_equals_slow((level, pushes) in (0u8..3).prop_flat_map(|level| {
            let n = 1usize << (3 * level);
            let monade = prop_oneof![
                3 => proptest::collection::vec(any::<u64>(), n),
                1 => proptest::collection::vec(any::<u64>(), 0..n + 2),
                1 => Just(vec![0u64; n]),
            ];
            (Just(level), proptest::collection::vec(monade, 0..20))
        })) {
            // The slow arena: one Vec per Monade, point 0 first.
            let size = Size::new(level).unwrap();
            let mut fast = Arena::new(size);
            let mut slow: Vec<Vec<Word>> = vec![vec![Word::ZERO; size.words()]];
            for p in pushes {
                let m: Vec<Word> = p.into_iter().map(Word::new).collect();
                let ok = m.len() == size.words() && m.iter().any(|w| *w != Word::ZERO);
                let r = fast.push(&m);
                prop_assert_eq!(r.is_ok(), ok);
                if ok {
                    slow.push(m);
                }
            }
            prop_assert_eq!(fast.len(), slow.len());
            for (i, m) in slow.iter().enumerate() {
                prop_assert_eq!(fast.get(Ref::new(i as u32)), Some(&m[..]));
            }
            prop_assert_eq!(fast.get(Ref::new(slow.len() as u32)), None);
            let flat: Vec<Word> = slow.concat();
            prop_assert_eq!(oracle::state_hash(&fast.words), oracle::state_hash(&flat));
        }
    }
}
