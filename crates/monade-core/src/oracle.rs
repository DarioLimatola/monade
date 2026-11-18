//! The oracle: slow, obvious reference implementations, for tests only.
//!
//! Every fast kernel is checked against its twin here on random inputs.
//! Compiled only in tests or with the `oracle` feature; the engine never uses it.

use crate::size::Size;
use crate::word::Word;

/// The 64 bits of a word, least significant first.
pub fn to_bools(w: Word) -> [bool; 64] {
    let mut out = [false; 64];
    for (i, b) in out.iter_mut().enumerate() {
        *b = (w.bits() >> i) & 1 == 1;
    }
    out
}

/// The word with the given 64 bits, least significant first.
pub fn from_bools(bits: [bool; 64]) -> Word {
    let mut v = 0u64;
    for (i, b) in bits.iter().enumerate() {
        if *b {
            v |= 1 << i;
        }
    }
    Word::new(v)
}

fn zip_bits(a: Word, b: Word, f: impl Fn(bool, bool) -> bool) -> Word {
    let (x, y) = (to_bools(a), to_bools(b));
    let mut out = [false; 64];
    for i in 0..64 {
        out[i] = f(x[i], y[i]);
    }
    from_bools(out)
}

/// XOR, bit by bit.
pub fn xor_word(a: Word, b: Word) -> Word {
    zip_bits(a, b, |x, y| x != y)
}

/// AND, bit by bit.
pub fn and_word(a: Word, b: Word) -> Word {
    zip_bits(a, b, |x, y| x && y)
}

/// Popcount, bit by bit.
pub fn popcount_word(a: Word) -> u64 {
    to_bools(a).iter().filter(|b| **b).count() as u64
}

/// Hamming distance, bit by bit.
pub fn hamming_word(a: Word, b: Word) -> u64 {
    let (x, y) = (to_bools(a), to_bools(b));
    (0..64).filter(|&i| x[i] != y[i]).count() as u64
}

/// XOR of two runs of words (assumes equal length).
pub fn xor(a: &[Word], b: &[Word]) -> Vec<Word> {
    a.iter().zip(b).map(|(x, y)| xor_word(*x, *y)).collect()
}

/// AND of two runs of words (assumes equal length).
pub fn and(a: &[Word], b: &[Word]) -> Vec<Word> {
    a.iter().zip(b).map(|(x, y)| and_word(*x, *y)).collect()
}

/// Popcount of a run of words.
pub fn popcount(a: &[Word]) -> u64 {
    a.iter().map(|w| popcount_word(*w)).sum()
}

/// Hamming distance of two runs of words (assumes equal length).
pub fn hamming(a: &[Word], b: &[Word]) -> u64 {
    a.iter().zip(b).map(|(x, y)| hamming_word(*x, *y)).sum()
}

/// Bit-wise majority: set when more than half of the inputs are set; tie gives 0.
pub fn majority(inputs: &[&[Word]]) -> Vec<Word> {
    let n = inputs.len();
    let len = inputs.first().map_or(0, |v| v.len());
    (0..len)
        .map(|i| {
            let mut bits = [false; 64];
            for (b, bit) in bits.iter_mut().enumerate() {
                let ones = inputs.iter().filter(|v| to_bools(v[i])[b]).count();
                *bit = 2 * ones > n;
            }
            from_bools(bits)
        })
        .collect()
}

/// Words in a size, by the definition: one word, or 8 of the size below.
pub fn size_words(s: Size) -> usize {
    match s.smaller() {
        None => 1,
        Some(below) => 8 * size_words(below),
    }
}

/// The size with this many bits, by walking up from the word.
pub fn size_from_bits(bits: u64) -> Option<Size> {
    let mut s = Size::WORD;
    loop {
        if s.bits() == bits {
            return Some(s);
        }
        s = s.larger()?;
    }
}

/// Hash of a state: FNV-1a over the little-endian bytes of every word.
///
/// For checking that two runs reached the same state; not cryptographic.
pub fn state_hash(words: &[Word]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for w in words {
        for byte in w.to_bytes() {
            h ^= u64::from(byte);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_hash_is_stable() {
        // Known FNV-1a value of eight zero bytes; this number must never change.
        assert_eq!(state_hash(&[Word::ZERO]), 0xa8c7_f832_281a_39c5);
        assert_eq!(state_hash(&[]), 0xcbf2_9ce4_8422_2325);
        assert_ne!(state_hash(&[Word::new(1)]), state_hash(&[Word::ZERO]));
    }

    #[test]
    fn bools_round_trip() {
        let w = Word::new(0xDEAD_BEEF_0123_4567);
        assert_eq!(from_bools(to_bools(w)), w);
    }
}
