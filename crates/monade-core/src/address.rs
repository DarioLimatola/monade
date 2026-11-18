//! The octal address: one rule to move across levels, tensors and bits.
//!
//! An address is a **root** (which arena, i.e. which tensor) and a **path of
//! base-8 digits**, one per level, each choosing one of the 8 parts of the level
//! above. Down = append a digit; up = drop the last one; another tensor = another
//! root. The same rule at every scale: from a 2 MB Monade, 8 digits reach one bit.
//!
//! **Levels.** A Monade of `8^e` bits has exponent `e` (`e = Size::level() + 2`:
//! a word has `e = 2`, 2 MB has `e = 8`). The part at depth `d` has `8^(e - d)`
//! bits: level 0 is one bit, level 1 one byte, level 2 one word.
//!
//! **Order.** Digit 0 is the topmost split. The part starts at bit
//! `Σ digitᵢ · 8^(e - 1 - i)` of the Monade (bit `b` is bit `b % 64` of word
//! `b / 64`, least significant first).
//!
//! **Morton.** Each digit is an octant: bit 0 → x, bit 1 → y, bit 2 → z. The
//! chain of octants is the Morton order, so a Monade of `8^e` bits is a cube of
//! side `2^e` with one point per bit (2 MB = 256³), and no separate Morton code
//! is needed.
//!
//! **Packed form, 56 bits** (the 7 operand bytes of a Logos instruction):
//! bits 0–23 digits (digit `i` at bits `21 - 3i ..= 23 - 3i`, the top digit
//! highest, unused digits 0), bits 24–27 depth, bits 28–55 root, bits 56–63 zero.
//! The top size is not packed: it belongs to the root's arena.

use crate::size::{Arena, Size};
use crate::word::{Ref, Word};
use core::fmt;

/// Largest root + 1: roots take 28 bits in the packed form.
pub const ROOTS: u32 = 1 << 28;

/// The deepest path: 8 digits, from 2 MB down to one bit.
pub const MAX_DEPTH: u8 = 8;

/// Errors of addresses. Never a panic.
///
/// # Example
///
/// ```
/// use monade_core::address::{Address, AddressError};
/// use monade_core::size::Size;
/// let a = Address::root(0, Size::WORD).unwrap();
/// assert_eq!(a.down(8), Err(AddressError::NotADigit(8)));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AddressError {
    /// A digit must be 0 to 7.
    NotADigit(u8),
    /// The path would go below one bit.
    BelowOneBit,
    /// The root does not fit in 28 bits.
    RootTooLarge(u32),
    /// The offset or point is outside the Monade.
    Outside,
    /// The offset or point is not the start of a part at that depth.
    NotAligned,
    /// The packed word is not a valid address for this size.
    BadPacked(u64),
    /// The part is smaller than a word (arenas hand out whole words).
    SmallerThanAWord,
    /// The address belongs to a Monade of another size.
    WrongSize,
    /// The arena has no Monade at this reference.
    NoSuchMonade,
}

impl fmt::Display for AddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddressError::NotADigit(d) => write!(f, "digit {d} is not 0 to 7"),
            AddressError::BelowOneBit => write!(f, "the path would go below one bit"),
            AddressError::RootTooLarge(r) => write!(f, "root {r} does not fit in 28 bits"),
            AddressError::Outside => write!(f, "outside the Monade"),
            AddressError::NotAligned => write!(f, "not the start of a part at that depth"),
            AddressError::BadPacked(p) => write!(f, "{p:#018x} is not a packed address"),
            AddressError::SmallerThanAWord => write!(f, "the part is smaller than a word"),
            AddressError::WrongSize => write!(f, "the address is for a Monade of another size"),
            AddressError::NoSuchMonade => write!(f, "no Monade at this reference"),
        }
    }
}

impl std::error::Error for AddressError {}

/// An octal address: a root and a path of base-8 digits.
///
/// # Example
///
/// ```
/// use monade_core::address::Address;
/// use monade_core::size::Size;
/// // In a 512-bit block: digit 3, then digit 5 → bits 3·64 + 5·8 = 232 .. 240.
/// let a = Address::root(7, Size::BLOCK).unwrap().down(3).unwrap().down(5).unwrap();
/// assert_eq!((a.bit_offset(), a.bit_len()), (232, 8));
/// assert_eq!(a.up().unwrap().bit_offset(), 192);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Address {
    root: u32,
    top: Size,
    depth: u8,
    /// Digits packed as in the packed form: digit `i` at bits `21 - 3i ..= 23 - 3i`.
    digits: u32,
}

/// Exponent of a size: a Monade of `8^e` bits.
fn exponent(top: Size) -> u8 {
    top.level() + 2
}

/// `8^k` as bits (k ≤ 8).
fn pow8(k: u8) -> u64 {
    1u64 << (3 * u32::from(k))
}

impl Address {
    /// The whole Monade at `root`, of size `top`.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::Address;
    /// use monade_core::size::Size;
    /// let a = Address::root(1, Size::THOUGHT).unwrap();
    /// assert_eq!((a.depth(), a.level(), a.bit_len()), (0, 8, 1 << 24));
    /// assert!(Address::root(1 << 28, Size::WORD).is_err());
    /// ```
    pub fn root(root: u32, top: Size) -> Result<Address, AddressError> {
        if root >= ROOTS {
            return Err(AddressError::RootTooLarge(root));
        }
        Ok(Address {
            root,
            top,
            depth: 0,
            digits: 0,
        })
    }

    /// One level down: the part `digit` (0 to 7) of this one.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::{Address, AddressError};
    /// use monade_core::size::Size;
    /// let w = Address::root(0, Size::WORD).unwrap();
    /// let bit = w.down(7).unwrap().down(7).unwrap();
    /// assert_eq!((bit.bit_offset(), bit.bit_len()), (63, 1));
    /// assert_eq!(bit.down(0), Err(AddressError::BelowOneBit));
    /// ```
    pub fn down(self, digit: u8) -> Result<Address, AddressError> {
        if digit >= 8 {
            return Err(AddressError::NotADigit(digit));
        }
        if self.depth >= exponent(self.top) {
            return Err(AddressError::BelowOneBit);
        }
        let shift = 21 - 3 * u32::from(self.depth);
        Ok(Address {
            depth: self.depth + 1,
            digits: self.digits | (u32::from(digit) << shift),
            ..self
        })
    }

    /// One level up; `None` at the root.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::Address;
    /// use monade_core::size::Size;
    /// let r = Address::root(0, Size::BLOCK).unwrap();
    /// assert_eq!(r.down(4).unwrap().up(), Some(r));
    /// assert_eq!(r.up(), None);
    /// ```
    pub fn up(self) -> Option<Address> {
        let depth = self.depth.checked_sub(1)?;
        let shift = 21 - 3 * u32::from(depth);
        Some(Address {
            depth,
            digits: self.digits & !(7 << shift),
            ..self
        })
    }

    /// Number of digits.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::Address;
    /// use monade_core::size::Size;
    /// assert_eq!(Address::root(0, Size::WORD).unwrap().down(1).unwrap().depth(), 1);
    /// ```
    pub fn depth(self) -> u8 {
        self.depth
    }

    /// Level of the addressed part: it has `8^level` bits (0 = one bit).
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::Address;
    /// use monade_core::size::Size;
    /// let a = Address::root(0, Size::BLOCK).unwrap();
    /// assert_eq!(a.level(), 3);
    /// assert_eq!(a.down(0).unwrap().level(), 2); // a word
    /// ```
    pub fn level(self) -> u8 {
        exponent(self.top) - self.depth
    }

    /// Digit `i` (0 = topmost), if the path has it.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::Address;
    /// use monade_core::size::Size;
    /// let a = Address::root(0, Size::BLOCK).unwrap().down(6).unwrap();
    /// assert_eq!((a.digit(0), a.digit(1)), (Some(6), None));
    /// ```
    pub fn digit(self, i: u8) -> Option<u8> {
        if i >= self.depth {
            return None;
        }
        Some(((self.digits >> (21 - 3 * u32::from(i))) & 7) as u8)
    }

    /// The root: which arena, i.e. which tensor.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::Address;
    /// use monade_core::size::Size;
    /// assert_eq!(Address::root(42, Size::WORD).unwrap().root_id(), 42);
    /// ```
    pub fn root_id(self) -> u32 {
        self.root
    }

    /// Size of the Monade at the root.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::Address;
    /// use monade_core::size::Size;
    /// assert_eq!(Address::root(0, Size::SYMBOL).unwrap().top(), Size::SYMBOL);
    /// ```
    pub fn top(self) -> Size {
        self.top
    }

    /// First bit of the part inside the Monade at the root.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::Address;
    /// use monade_core::size::Size;
    /// let a = Address::root(0, Size::WORD).unwrap().down(2).unwrap();
    /// assert_eq!(a.bit_offset(), 16);
    /// ```
    pub fn bit_offset(self) -> u64 {
        let e = exponent(self.top);
        (0..self.depth)
            .map(|i| u64::from(self.digit(i).unwrap_or(0)) * pow8(e - 1 - i))
            .sum()
    }

    /// Length of the part, in bits: `8^level`.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::Address;
    /// use monade_core::size::Size;
    /// assert_eq!(Address::root(0, Size::BLOCK).unwrap().down(0).unwrap().bit_len(), 64);
    /// ```
    pub fn bit_len(self) -> u64 {
        pow8(self.level())
    }

    /// The address of the part of depth `depth` that starts at bit `offset`.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::{Address, AddressError};
    /// use monade_core::size::Size;
    /// let a = Address::from_bit_offset(0, Size::BLOCK, 232, 2).unwrap();
    /// assert_eq!((a.digit(0), a.digit(1)), (Some(3), Some(5)));
    /// assert_eq!(Address::from_bit_offset(0, Size::BLOCK, 233, 2), Err(AddressError::NotAligned));
    /// assert_eq!(Address::from_bit_offset(0, Size::BLOCK, 512, 3), Err(AddressError::Outside));
    /// ```
    pub fn from_bit_offset(
        root: u32,
        top: Size,
        offset: u64,
        depth: u8,
    ) -> Result<Address, AddressError> {
        let e = exponent(top);
        if depth > e {
            return Err(AddressError::BelowOneBit);
        }
        if offset >= pow8(e) {
            return Err(AddressError::Outside);
        }
        if !offset.is_multiple_of(pow8(e - depth)) {
            return Err(AddressError::NotAligned);
        }
        let mut a = Address::root(root, top)?;
        for i in 0..depth {
            a = a.down(((offset / pow8(e - 1 - i)) % 8) as u8)?;
        }
        Ok(a)
    }

    /// The corner of the part in the cube of side `2^e` (one point per bit).
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::Address;
    /// use monade_core::size::Size;
    /// // Octant 7 of a word (a cube of side 4) is the corner (2, 2, 2).
    /// let a = Address::root(0, Size::WORD).unwrap().down(7).unwrap();
    /// assert_eq!(a.point(), (2, 2, 2));
    /// ```
    pub fn point(self) -> (u32, u32, u32) {
        let (mut x, mut y, mut z) = (0u32, 0u32, 0u32);
        for i in 0..self.depth {
            let d = u32::from(self.digit(i).unwrap_or(0));
            x = (x << 1) | (d & 1);
            y = (y << 1) | ((d >> 1) & 1);
            z = (z << 1) | ((d >> 2) & 1);
        }
        let rest = u32::from(self.level());
        (x << rest, y << rest, z << rest)
    }

    /// The address of the part of depth `depth` whose corner is `point`.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::{Address, AddressError};
    /// use monade_core::size::Size;
    /// let a = Address::from_point(0, Size::WORD, (2, 2, 2), 1).unwrap();
    /// assert_eq!(a.digit(0), Some(7));
    /// assert_eq!(Address::from_point(0, Size::WORD, (1, 0, 0), 1), Err(AddressError::NotAligned));
    /// assert_eq!(Address::from_point(0, Size::WORD, (4, 0, 0), 2), Err(AddressError::Outside));
    /// ```
    pub fn from_point(
        root: u32,
        top: Size,
        point: (u32, u32, u32),
        depth: u8,
    ) -> Result<Address, AddressError> {
        let e = exponent(top);
        if depth > e {
            return Err(AddressError::BelowOneBit);
        }
        let (x, y, z) = point;
        let side = 1u32 << e;
        if x >= side || y >= side || z >= side {
            return Err(AddressError::Outside);
        }
        let low = (1u32 << (e - depth)) - 1;
        if (x | y | z) & low != 0 {
            return Err(AddressError::NotAligned);
        }
        let mut a = Address::root(root, top)?;
        for i in 0..depth {
            let b = u32::from(e - 1 - i);
            let d = ((x >> b) & 1) | (((y >> b) & 1) << 1) | (((z >> b) & 1) << 2);
            a = a.down(d as u8)?;
        }
        Ok(a)
    }

    /// The packed form: 56 bits (digits, depth, root); the top 8 bits are 0.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::Address;
    /// use monade_core::size::Size;
    /// let a = Address::root(1, Size::WORD).unwrap().down(5).unwrap();
    /// assert_eq!(a.pack(), (1 << 28) | (1 << 24) | (5 << 21));
    /// assert_eq!(Address::unpack(a.pack(), Size::WORD), Ok(a));
    /// ```
    pub fn pack(self) -> u64 {
        (u64::from(self.root) << 28) | (u64::from(self.depth) << 24) | u64::from(self.digits)
    }

    /// Reads a packed address for a Monade of size `top`.
    ///
    /// Refused: top 8 bits not 0, depth below one bit, digits beyond the depth not 0.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::{Address, AddressError};
    /// use monade_core::size::Size;
    /// assert!(Address::unpack(u64::MAX, Size::THOUGHT).is_err());
    /// assert_eq!(Address::unpack(3 << 24, Size::WORD), Err(AddressError::BadPacked(3 << 24)));
    /// ```
    pub fn unpack(packed: u64, top: Size) -> Result<Address, AddressError> {
        let bad = AddressError::BadPacked(packed);
        if packed >> 56 != 0 {
            return Err(bad);
        }
        let digits = (packed & 0xff_ffff) as u32;
        let depth = ((packed >> 24) & 0xf) as u8;
        let root = (packed >> 28) as u32;
        if depth > exponent(top) {
            return Err(bad);
        }
        let used = 24 - 3 * u32::from(depth);
        if used < 24 && digits & ((1 << used) - 1) != 0 {
            return Err(bad);
        }
        Ok(Address {
            root,
            top,
            depth,
            digits,
        })
    }

    /// The packed form as a word (its reference view stays for arena indices).
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::Address;
    /// use monade_core::size::Size;
    /// let a = Address::root(3, Size::BLOCK).unwrap();
    /// assert_eq!(Address::from_word(a.to_word(), Size::BLOCK), Ok(a));
    /// ```
    pub fn to_word(self) -> Word {
        Word::new(self.pack())
    }

    /// Reads an address from a word, for a Monade of size `top`.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::Address;
    /// use monade_core::size::Size;
    /// use monade_core::word::Word;
    /// assert!(Address::from_word(Word::new(u64::MAX), Size::BLOCK).is_err());
    /// ```
    pub fn from_word(w: Word, top: Size) -> Result<Address, AddressError> {
        Address::unpack(w.bits(), top)
    }
}

impl Arena {
    /// The words of the part addressed by `a` in the Monade at `r`.
    ///
    /// Parts of a word or larger only; a byte or a bit is an error for now.
    ///
    /// # Example
    ///
    /// ```
    /// use monade_core::address::{Address, AddressError};
    /// use monade_core::size::{Arena, Size};
    /// use monade_core::word::Word;
    /// let mut arena = Arena::new(Size::BLOCK);
    /// let block: Vec<Word> = (1..=8).map(Word::new).collect();
    /// let r = arena.push(&block).unwrap();
    /// let third = Address::root(0, Size::BLOCK).unwrap().down(2).unwrap();
    /// assert_eq!(arena.part(r, &third), Ok(&[Word::new(3)][..]));
    /// let byte = third.down(0).unwrap();
    /// assert_eq!(arena.part(r, &byte), Err(AddressError::SmallerThanAWord));
    /// ```
    pub fn part(&self, r: Ref, a: &Address) -> Result<&[Word], AddressError> {
        if a.top() != self.size() {
            return Err(AddressError::WrongSize);
        }
        if a.level() < 2 {
            return Err(AddressError::SmallerThanAWord);
        }
        let monade = self.get(r).ok_or(AddressError::NoSuchMonade)?;
        let start = usize::try_from(a.bit_offset() / 64).map_err(|_| AddressError::Outside)?;
        let len = usize::try_from(a.bit_len() / 64).map_err(|_| AddressError::Outside)?;
        monade.get(start..start + len).ok_or(AddressError::Outside)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle;
    use proptest::prelude::*;

    fn any_size() -> impl Strategy<Value = Size> {
        (0u8..=6).prop_map(|l| Size::new(l).unwrap())
    }

    /// A random address: root, size, and a path of random length.
    fn any_address() -> impl Strategy<Value = Address> {
        (
            0u32..ROOTS,
            any_size(),
            prop::collection::vec(0u8..8, 0..=8),
        )
            .prop_map(|(root, top, digits)| {
                let mut a = Address::root(root, top).unwrap();
                for d in digits.into_iter().take(usize::from(exponent(top))) {
                    a = a.down(d).unwrap();
                }
                a
            })
    }

    fn digits(a: Address) -> Vec<u8> {
        (0..a.depth()).filter_map(|i| a.digit(i)).collect()
    }

    proptest! {
        #[test]
        fn down_up_round_trip(a in any_address(), d in 0u8..8) {
            if let Ok(child) = a.down(d) {
                prop_assert_eq!(child.up(), Some(a));
                prop_assert_eq!(child.digit(a.depth()), Some(d));
            } else {
                prop_assert_eq!(a.level(), 0);
            }
            if let Some(parent) = a.up() {
                let last = a.digit(a.depth() - 1).unwrap();
                prop_assert_eq!(parent.down(last), Ok(a));
            }
        }

        #[test]
        fn offset_point_pack_round_trips(a in any_address()) {
            let (r, t, d) = (a.root_id(), a.top(), a.depth());
            prop_assert_eq!(Address::from_bit_offset(r, t, a.bit_offset(), d), Ok(a));
            prop_assert_eq!(Address::from_point(r, t, a.point(), d), Ok(a));
            prop_assert_eq!(Address::unpack(a.pack(), t), Ok(a));
            prop_assert_eq!(a.pack() >> 56, 0);
        }

        #[test]
        fn fast_equals_slow(a in any_address()) {
            let e = exponent(a.top());
            prop_assert_eq!(a.bit_offset(), oracle::address_offset(e, &digits(a)));
            prop_assert_eq!(a.point(), oracle::morton_point(a.bit_offset(), e));
        }

        #[test]
        fn child_lies_inside_parent(a in any_address(), d in 0u8..8) {
            if let Ok(c) = a.down(d) {
                prop_assert!(c.bit_offset() >= a.bit_offset());
                prop_assert!(c.bit_offset() + c.bit_len() <= a.bit_offset() + a.bit_len());
            }
        }

        #[test]
        fn lexicographic_order_is_offset_order(a in any_address(), b in any_address()) {
            let b = {
                // The same root, size and depth as `a`, other digits.
                let mut x = Address::root(a.root_id(), a.top()).unwrap();
                for i in 0..a.depth() {
                    x = x.down(b.digit(i % b.depth().max(1)).unwrap_or(i % 8)).unwrap();
                }
                x
            };
            prop_assert_eq!(digits(a).cmp(&digits(b)), a.bit_offset().cmp(&b.bit_offset()));
        }

        #[test]
        fn never_panics(packed in any::<u64>(), top in any_size(), d in any::<u8>(), off in any::<u64>(),
                        x in any::<u32>(), y in any::<u32>(), z in any::<u32>(), root in any::<u32>()) {
            let _ = Address::unpack(packed, top);
            let _ = Address::from_bit_offset(root, top, off, d);
            let _ = Address::from_point(root, top, (x, y, z), d);
            if let Ok(a) = Address::root(root, top) {
                let _ = a.down(d);
            }
        }
    }

    /// Every bit of a Monade of this size, as a point: each point hit once.
    fn morton_is_a_bijection(top: Size, step: u64) {
        let e = exponent(top);
        let side = 1usize << e;
        let mut seen = vec![false; side * side * side];
        let mut count = 0usize;
        let mut offset = 0;
        while offset < pow8(e) {
            let a = Address::from_bit_offset(0, top, offset, e).unwrap();
            let (x, y, z) = a.point();
            let i = (z as usize * side + y as usize) * side + x as usize;
            assert!(!seen[i], "point hit twice at offset {offset}");
            seen[i] = true;
            count += 1;
            offset += step;
        }
        assert_eq!(count as u64, pow8(e).div_ceil(step));
    }

    #[test]
    fn morton_bijection_block_and_symbol() {
        morton_is_a_bijection(Size::WORD, 1);
        morton_is_a_bijection(Size::BLOCK, 1);
        morton_is_a_bijection(Size::SYMBOL, 1);
    }

    #[test]
    fn morton_injective_on_a_sample_of_2_mb() {
        // 2^24 points; every 4099th one (an odd step visits every residue).
        morton_is_a_bijection(Size::THOUGHT, 4099);
    }

    #[test]
    fn depth_8_from_2_mb_is_one_bit() {
        let mut a = Address::root(0, Size::THOUGHT).unwrap();
        for _ in 0..MAX_DEPTH {
            a = a.down(7).unwrap();
        }
        assert_eq!(
            (a.level(), a.bit_len(), a.bit_offset()),
            (0, 1, (1 << 24) - 1)
        );
        assert_eq!(a.point(), (255, 255, 255));
        assert_eq!(a.pack() & 0xff_ffff, 0xff_ffff, "24 bits of digits");
        assert_eq!(a.down(0), Err(AddressError::BelowOneBit));
    }

    #[test]
    fn unpack_refusals() {
        let ok = Address::root(5, Size::BLOCK)
            .unwrap()
            .down(1)
            .unwrap()
            .pack();
        assert!(Address::unpack(ok, Size::BLOCK).is_ok());
        assert!(Address::unpack(ok | (1 << 56), Size::BLOCK).is_err());
        assert!(
            Address::unpack(ok | 1, Size::BLOCK).is_err(),
            "digit beyond depth"
        );
        assert!(
            Address::unpack(4 << 24, Size::BLOCK).is_err(),
            "below one bit"
        );
        assert!(Address::unpack(8 << 24, Size::THOUGHT).is_ok());
    }

    #[test]
    fn parts_of_an_arena() {
        let mut arena = Arena::new(Size::SYMBOL);
        let m: Vec<Word> = (1..=64).map(Word::new).collect();
        let r = arena.push(&m).unwrap();
        let a = Address::root(0, Size::SYMBOL).unwrap().down(5).unwrap();
        assert_eq!(arena.part(r, &a).unwrap(), &m[40..48]);
        let w = a.down(3).unwrap();
        assert_eq!(arena.part(r, &w).unwrap(), &m[43..44]);
        let other = Address::root(0, Size::BLOCK).unwrap();
        assert_eq!(arena.part(r, &other), Err(AddressError::WrongSize));
        assert_eq!(arena.part(Ref::new(9), &a), Err(AddressError::NoSuchMonade));
    }
}
