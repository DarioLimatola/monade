//! The generated space: basic vectors computed from their identifier and the seed.
//!
//! The basic vectors of the Monade (an axis, a role, a symbol, a word of a
//! language) are not stored: they are computed when needed from an identifier,
//! with a hash keyed by the seed. The same seed gives the same vectors on every
//! machine and in every run; nothing costs memory until it is used. A different
//! seed gives an entirely different space: a new mind.
//!
//! **Hash, version 1:** BLAKE3 in keyed mode. The key is derived from the seed with
//! the context `"monade space v1"`. The input is a domain byte (0 = numeric id,
//! 1 = name) followed by the id (8 bytes, little-endian) or the name (UTF-8). The
//! output is BLAKE3's extendable output, read in 64-byte blocks: block `k` of every
//! generated Monade is bytes `64k .. 64k + 64` of the output, so each 512-bit block
//! can be computed alone (random access, lazy, parallel), and a smaller size is
//! the beginning of a larger one (the same idea at a coarser resolution). Bytes
//! become words little-endian.
//!
//! This is Logos' `GEN`. Locked: changing anything here would be a new mind.

use crate::address::{Address, AddressError};
use crate::size::Size;
use crate::specs::{Specs, HASH_VERSION, HASH_VERSION_KEY, SEED_KEY};
use crate::word::{Block, Word};
use core::fmt;

/// Context of the key derivation of the space, version 1. Never change it.
pub const CONTEXT_V1: &str = "monade space v1";

const DOMAIN_ID: u8 = 0;
const DOMAIN_NAME: u8 = 1;

/// Errors of the generated space.
///
/// # Example
///
/// ```
/// use monade_core::space::{Space, SpaceError};
/// assert_eq!(Space::parse_seed("dog"), Err(SpaceError::BadSeed("dog".into())));
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SpaceError {
    /// The seed in the specifications is not a hexadecimal `u64` (`0x…`).
    BadSeed(String),
    /// The specifications ask for a hash version this code does not know.
    UnknownVersion(String),
    /// A specification needed by the space is missing.
    Missing(&'static str),
    /// The address of a part is not valid for the requested size.
    Address(AddressError),
}

impl fmt::Display for SpaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpaceError::BadSeed(s) => write!(f, "seed `{s}` is not 0x followed by hex digits"),
            SpaceError::UnknownVersion(v) => write!(f, "hash version `{v}` is unknown (only 1)"),
            SpaceError::Missing(k) => write!(f, "specification `{k}` is missing"),
            SpaceError::Address(e) => write!(f, "address: {e}"),
        }
    }
}

impl std::error::Error for SpaceError {}

impl From<AddressError> for SpaceError {
    fn from(e: AddressError) -> Self {
        SpaceError::Address(e)
    }
}

/// The generated space of one mind: a seed and the key derived from it.
///
/// # Example
///
/// ```
/// use monade_core::space::Space;
/// use monade_core::size::Size;
/// use monade_core::word::hamming;
/// let space = Space::genesis();
/// let dog = space.named("it.cane", Size::BLOCK);
/// let cat = space.named("it.gatto", Size::BLOCK);
/// assert_eq!(dog, space.named("it.cane", Size::BLOCK)); // always the same
/// let d = hamming(&dog, &cat).unwrap();
/// assert!(d > 190 && d < 322); // two unrelated ideas: about half the bits differ
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Space {
    seed: u64,
    key: [u8; 32],
}

impl Space {
    /// The space of a seed (hash version 1).
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::space::Space;
    /// assert_ne!(Space::new(1), Space::new(2));
    /// assert_eq!(Space::new(7).seed(), 7);
    /// ```
    pub fn new(seed: u64) -> Space {
        Space {
            seed,
            key: blake3::derive_key(CONTEXT_V1, &seed.to_le_bytes()),
        }
    }

    /// The space of the genesis seed of the specifications.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::space::Space;
    /// use monade_core::specs::SEED;
    /// assert_eq!(Space::genesis().seed(), SEED);
    /// ```
    pub fn genesis() -> Space {
        Space::new(crate::specs::SEED)
    }

    /// The space described by the specifications (seed and hash version).
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::space::Space;
    /// use monade_core::specs::Specs;
    /// assert_eq!(Space::from_specs(&Specs::genesis()), Ok(Space::genesis()));
    /// ```
    pub fn from_specs(specs: &Specs) -> Result<Space, SpaceError> {
        let version = &specs
            .get(HASH_VERSION_KEY)
            .ok_or(SpaceError::Missing(HASH_VERSION_KEY))?
            .value;
        if version != &HASH_VERSION.to_string() {
            return Err(SpaceError::UnknownVersion(version.clone()));
        }
        let seed = &specs
            .get(SEED_KEY)
            .ok_or(SpaceError::Missing(SEED_KEY))?
            .value;
        Ok(Space::new(Space::parse_seed(seed)?))
    }

    /// Reads a seed written as `0x` and hexadecimal digits.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::space::Space;
    /// assert_eq!(Space::parse_seed("0x4d4f4e4144450001"), Ok(0x4d4f_4e41_4445_0001));
    /// ```
    pub fn parse_seed(text: &str) -> Result<u64, SpaceError> {
        let bad = || SpaceError::BadSeed(text.to_string());
        let hex = text.strip_prefix("0x").ok_or_else(bad)?;
        if hex.is_empty() || hex.len() > 16 {
            return Err(bad());
        }
        u64::from_str_radix(hex, 16).map_err(|_| bad())
    }

    /// The seed of this space.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::space::Space;
    /// assert_eq!(Space::new(3).seed(), 3);
    /// ```
    pub fn seed(&self) -> u64 {
        self.seed
    }

    fn reader(&self, domain: u8, payload: &[u8]) -> blake3::OutputReader {
        let mut h = blake3::Hasher::new_keyed(&self.key);
        h.update(&[domain]);
        h.update(payload);
        h.finalize_xof()
    }

    /// Block `k` (512 bits) of the vector of `id`, computed alone.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::space::Space;
    /// use monade_core::size::Size;
    /// let s = Space::genesis();
    /// let v = s.generate(9, Size::SYMBOL);
    /// assert_eq!(s.block(9, 5).0, v[40..48]); // block 5 = words 40..48
    /// ```
    pub fn block(&self, id: u64, k: u64) -> Block {
        let mut r = self.reader(DOMAIN_ID, &id.to_le_bytes());
        r.set_position(64 * k);
        read_block(&mut r)
    }

    /// The vector of `id`, of the given size.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::space::Space;
    /// use monade_core::size::Size;
    /// let s = Space::genesis();
    /// let block = s.generate(1, Size::BLOCK);
    /// let word = s.generate(1, Size::WORD);
    /// assert_eq!(word[0], block[0]); // a smaller size is the beginning of a larger one
    /// assert_ne!(s.generate(2, Size::BLOCK), block);
    /// ```
    pub fn generate(&self, id: u64, size: Size) -> Vec<Word> {
        let r = self.reader(DOMAIN_ID, &id.to_le_bytes());
        read_words(r, size.words())
    }

    /// The vector of a name (e.g. `"it.cane"`), of the given size.
    ///
    /// Names and numeric ids live in separate domains: a name never collides with an id.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::space::Space;
    /// use monade_core::size::Size;
    /// let s = Space::genesis();
    /// assert_eq!(s.named("role.subject", Size::WORD).len(), 1);
    /// assert_ne!(s.named("a", Size::BLOCK), s.named("b", Size::BLOCK));
    /// ```
    pub fn named(&self, name: &str, size: Size) -> Vec<Word> {
        let r = self.reader(DOMAIN_NAME, name.as_bytes());
        read_words(r, size.words())
    }

    /// Only the part of the vector of `id` that `address` points to (a word or larger),
    /// computing only the blocks it needs.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::Address;
    /// use monade_core::space::Space;
    /// use monade_core::size::Size;
    /// let s = Space::genesis();
    /// // The last 32 KB of a 2 MB vector, without generating the other 1.97 MB.
    /// let a = Address::root(0, Size::THOUGHT).unwrap().down(7).unwrap().down(7).unwrap();
    /// let part = s.part(4, &a).unwrap();
    /// assert_eq!(part.len(), 4096);
    /// assert_eq!(part[..8], s.block(4, 32_768 - 512).0);
    /// ```
    pub fn part(&self, id: u64, address: &Address) -> Result<Vec<Word>, SpaceError> {
        if address.level() < 2 {
            return Err(AddressError::SmallerThanAWord.into());
        }
        let first_word = address.bit_offset() / 64;
        let words = usize::try_from(address.bit_len() / 64).map_err(|_| AddressError::Outside)?;
        let mut r = self.reader(DOMAIN_ID, &id.to_le_bytes());
        r.set_position(8 * first_word);
        Ok(read_words(r, words))
    }
}

/// Reads one 64-byte block from the output.
fn read_block(r: &mut blake3::OutputReader) -> Block {
    let mut bytes = [0u8; 64];
    r.fill(&mut bytes);
    let mut words = [Word::ZERO; 8];
    for (w, chunk) in words.iter_mut().zip(bytes.as_chunks::<8>().0) {
        *w = Word::from_bytes(*chunk);
    }
    Block(words)
}

/// Reads `n` words from the output, little-endian.
fn read_words(mut r: blake3::OutputReader, n: usize) -> Vec<Word> {
    let mut bytes = vec![0u8; 8 * n];
    r.fill(&mut bytes);
    bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|c| Word::from_bytes(*c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle;
    use crate::word::hamming;
    use proptest::prelude::*;

    #[test]
    fn blake3_known_answer() {
        // Official BLAKE3 hash of the empty input.
        let h = blake3::hash(b"");
        let hex: String = h.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex,
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }

    #[test]
    fn golden_vectors_never_change() {
        // Fingerprints of the genesis space; if one changes, it is another mind.
        let s = Space::genesis();
        let ids: Vec<Word> = (0..64).flat_map(|id| s.generate(id, Size::BLOCK)).collect();
        assert_eq!(oracle::state_hash(&ids), GOLDEN_IDS);
        let named = s.named("it.cane", Size::BLOCK);
        assert_eq!(oracle::state_hash(&named), GOLDEN_NAME);
    }

    /// State hash of blocks 0..64 of the genesis space; fixed when version 1 was written.
    const GOLDEN_IDS: u64 = 0x1125fbd0ff9f9379;
    /// State hash of the block of "it.cane" in the genesis space.
    const GOLDEN_NAME: u64 = 0x93cda09673f69172;

    #[test]
    fn seed_and_version_come_from_the_specs() {
        let mut specs = Specs::genesis();
        assert_eq!(Space::from_specs(&specs), Ok(Space::genesis()));
        specs
            .set("other", "1", "2026-11-18", "a test")
            .expect("unlocked key");
        assert_eq!(Space::from_specs(&specs), Ok(Space::genesis()));
        assert!(Space::parse_seed("0x").is_err());
        assert!(Space::parse_seed("0x1ffffffffffffffff").is_err());
        assert!(Space::parse_seed("12").is_err());
    }

    proptest! {
        #[test]
        fn blocks_equal_sequential_reading(id in any::<u64>(), level in 0u8..=3) {
            let s = Space::genesis();
            let size = Size::new(level).unwrap();
            let v = s.generate(id, size);
            for (k, chunk) in v.chunks(8).enumerate() {
                let b = s.block(id, k as u64);
                prop_assert_eq!(chunk, &b.0[..chunk.len()]);
            }
            prop_assert_eq!(oracle::generate(&s, id, size.words()), v);
        }

        #[test]
        fn parts_equal_slices(id in any::<u64>(), digits in prop::collection::vec(0u8..8, 0..=4)) {
            let s = Space::genesis();
            let top = Size::SYMBOL; // 8^4 bits: depth up to 2 gives a word or more
            let mut a = Address::root(0, top).unwrap();
            for d in digits.iter().take(2) {
                a = a.down(*d).unwrap();
            }
            let whole = s.generate(id, top);
            let from = (a.bit_offset() / 64) as usize;
            let len = (a.bit_len() / 64) as usize;
            prop_assert_eq!(s.part(id, &a).unwrap(), whole[from..from + len].to_vec());
        }

        #[test]
        fn different_ids_are_strangers(a in any::<u64>(), b in any::<u64>()) {
            prop_assume!(a != b);
            let s = Space::genesis();
            let d = hamming(&s.generate(a, Size::SYMBOL), &s.generate(b, Size::SYMBOL)).unwrap();
            // 4096 bits: mean 2048, sigma 32; 8 sigma either way.
            prop_assert!((1792..=2304).contains(&d), "distance {}", d);
        }
    }

    #[test]
    fn another_seed_is_another_mind() {
        let (a, b) = (Space::new(1), Space::new(2));
        let d = hamming(&a.generate(0, Size::SYMBOL), &b.generate(0, Size::SYMBOL)).unwrap();
        assert!((1792..=2304).contains(&d));
        assert_ne!(a.named("x", Size::WORD), b.named("x", Size::WORD));
    }

    #[test]
    fn small_parts_are_refused() {
        let a = Address::root(0, Size::BLOCK)
            .unwrap()
            .down(0)
            .unwrap()
            .down(0)
            .unwrap();
        assert_eq!(
            Space::genesis().part(0, &a),
            Err(SpaceError::Address(AddressError::SmallerThanAWord))
        );
    }
}
