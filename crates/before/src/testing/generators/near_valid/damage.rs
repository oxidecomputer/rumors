//! The damage done to an encoding at each stage of spelling it: the live
//! bits, the seal, and the bytes.

use proptest::option;
use proptest::prelude::*;
use proptest::sample::Index;

use super::Expect;
use crate::testing::oracles::codec::{pack, seal};

/// Inverts the bit at `position`, counted from the first byte's high bit.
pub(super) fn flip_bit(bytes: &mut [u8], position: usize) {
    bytes[position / 8] ^= 0x80 >> (position % 8);
}

/// Sets the bits of the last byte from bit `start` on wherever `pattern`'s
/// bits, read from its high end, are set, and returns whether it set any.
///
/// `start` counts from the first byte's high bit and lies in the last byte,
/// so the bits it sets are padding after a stream's last meaningful bit.
pub(super) fn dirty_padding(bytes: &mut [u8], start: usize, pattern: u8) -> bool {
    let end = bytes.len() * 8;
    let mut dirtied = false;
    for (offset, position) in (start..end).enumerate() {
        if pattern & (0x80 >> offset) != 0 {
            bytes[position / 8] |= 0x80 >> (position % 8);
            dirtied = true;
        }
    }
    dirtied
}

/// An edit to a stream's live bits, before sealing.
#[derive(Clone, Debug)]
pub(super) enum BitEdit {
    /// Inverts one bit.
    Flip(Index),
    /// Keeps a strict prefix, which ends inside the tree.
    Cut(Index),
    /// Inserts a bit at any position, the end included.
    Insert(Index, bool),
    /// Removes one bit.
    Delete(Index),
}

/// Bit edits, cuts twice as often as each other kind: a cut is the only
/// edit that always leaves the tree incomplete.
pub(super) fn arb_bit_edit() -> impl Strategy<Value = BitEdit> {
    prop_oneof![
        1 => any::<Index>().prop_map(BitEdit::Flip),
        2 => any::<Index>().prop_map(BitEdit::Cut),
        1 => (any::<Index>(), any::<bool>()).prop_map(|(at, bit)| BitEdit::Insert(at, bit)),
        1 => any::<Index>().prop_map(BitEdit::Delete),
    ]
}

impl BitEdit {
    /// Applies the edit to a stream's live bits, which are never empty.
    pub(super) fn apply(&self, mut bits: Vec<bool>) -> Vec<bool> {
        let len = bits.len();
        match self {
            BitEdit::Flip(at) => bits[at.index(len)] ^= true,
            BitEdit::Cut(at) => bits.truncate(at.index(len)),
            BitEdit::Insert(at, bit) => bits.insert(at.index(len + 1), *bit),
            BitEdit::Delete(at) => {
                bits.remove(at.index(len));
            }
        }
        bits
    }
}

/// A defect in sealing a version or party stream.
#[derive(Clone, Debug)]
enum SealEdit {
    /// Packs the live bits with no marker bit.
    NoMarker,
    /// Fills the padding after the marker with this byte's bits, high first.
    JunkAfterMarker(u8),
    /// Appends a zero byte after the sealed stream.
    ExtraZeroByte,
    /// Appends this byte after the sealed stream.
    ExtraByte(u8),
    /// Drops the sealed stream's last byte.
    DropLastByte,
}

/// Each sealing defect equally often, with any junk or extra byte.
fn arb_seal_edit() -> impl Strategy<Value = SealEdit> {
    prop_oneof![
        Just(SealEdit::NoMarker),
        any::<u8>().prop_map(SealEdit::JunkAfterMarker),
        Just(SealEdit::ExtraZeroByte),
        any::<u8>().prop_map(SealEdit::ExtraByte),
        Just(SealEdit::DropLastByte),
    ]
}

impl SealEdit {
    /// Seals `bits` with this defect.
    fn seal(&self, bits: &[bool]) -> Vec<u8> {
        match self {
            SealEdit::NoMarker => pack(bits),
            SealEdit::JunkAfterMarker(junk) => {
                let mut sealed = seal(bits);
                dirty_padding(&mut sealed, bits.len() + 1, *junk);
                sealed
            }
            SealEdit::ExtraZeroByte => [seal(bits), vec![0]].concat(),
            SealEdit::ExtraByte(byte) => [seal(bits), vec![*byte]].concat(),
            SealEdit::DropLastByte => {
                let mut sealed = seal(bits);
                sealed.pop();
                sealed
            }
        }
    }
}

/// An edit to an encoding's bytes.
#[derive(Clone, Debug)]
pub(super) enum ByteEdit {
    /// Inverts one bit of one byte.
    Flip(Index),
    /// Keeps a strict prefix of the bytes.
    Cut(Index),
    /// Appends one to three bytes.
    Append(Vec<u8>),
}

/// Each byte edit equally often, at any position.
pub(super) fn arb_byte_edit() -> impl Strategy<Value = ByteEdit> {
    prop_oneof![
        any::<Index>().prop_map(ByteEdit::Flip),
        any::<Index>().prop_map(ByteEdit::Cut),
        prop::collection::vec(any::<u8>(), 1..=3).prop_map(ByteEdit::Append),
    ]
}

/// A byte edit one time in four, for composites whose components carry
/// their own damage.
pub(super) fn arb_composite_edit() -> impl Strategy<Value = Option<ByteEdit>> {
    prop_oneof![3 => Just(None), 1 => arb_byte_edit().prop_map(Some)]
}

impl ByteEdit {
    /// Applies the edit; flipping or cutting empty bytes leaves them empty.
    pub(super) fn apply(&self, mut bytes: Vec<u8>) -> Vec<u8> {
        match self {
            ByteEdit::Flip(at) if !bytes.is_empty() => {
                let position = at.index(bytes.len() * 8);
                flip_bit(&mut bytes, position);
            }
            ByteEdit::Cut(at) if !bytes.is_empty() => bytes.truncate(at.index(bytes.len())),
            ByteEdit::Append(extra) => bytes.extend_from_slice(extra),
            ByteEdit::Flip(_) | ByteEdit::Cut(_) => {}
        }
        bytes
    }
}

/// The damage done to one tree's encoding, at each stage of spelling it.
#[derive(Clone, Debug)]
pub(super) struct Damage<P> {
    /// A non-canonical spelling planted in the tree.
    pub(super) plant: Option<P>,
    /// An edit to the live bits.
    pub(super) bits: Option<BitEdit>,
    /// A sealing defect.
    seal: Option<SealEdit>,
    /// An edit to the sealed bytes.
    bytes: Option<ByteEdit>,
}

impl<P> Damage<P> {
    /// No damage: the canonical encoding.
    pub(super) fn none() -> Self {
        Damage {
            plant: None,
            bits: None,
            seal: None,
            bytes: None,
        }
    }

    /// What construction settles: a tree with nothing done to it is
    /// canonical, and one with only a plant is non-canonical. Any edit after
    /// planting may or may not leave a canonical encoding.
    pub(super) fn expect(&self) -> Expect {
        let edited = self.bits.is_some() || self.seal.is_some() || self.bytes.is_some();
        match (&self.plant, edited) {
            (None, false) => Expect::Canonical,
            (Some(_), false) => Expect::Defective,
            (_, true) => Expect::Open,
        }
    }

    /// Spells a tree's live bits under the edits.
    pub(super) fn spell(&self, live: Vec<bool>) -> Vec<u8> {
        let live = match &self.bits {
            Some(edit) => edit.apply(live),
            None => live,
        };
        let sealed = match &self.seal {
            Some(edit) => edit.seal(&live),
            None => seal(&live),
        };
        match &self.bytes {
            Some(edit) => edit.apply(sealed),
            None => sealed,
        }
    }
}

/// Damage drawn one kind at a time, most often: out of ten draws, none
/// three times, a plant alone twice, a bit edit alone twice, a sealing defect
/// or a byte edit alone once each, and every stage independently once.
///
/// Damage of one kind keeps the verdict settled for most inputs; the
/// independent arm reaches inputs with several defects at once.
pub(super) fn arb_damage<S>(plant: fn() -> S) -> impl Strategy<Value = Damage<S::Value>>
where
    S: Strategy + 'static,
    S::Value: Clone,
{
    prop_oneof![
        3 => Just(Damage::none()),
        2 => plant().prop_map(|plant| Damage { plant: Some(plant), ..Damage::none() }),
        2 => arb_bit_edit().prop_map(|edit| Damage { bits: Some(edit), ..Damage::none() }),
        1 => arb_seal_edit().prop_map(|edit| Damage { seal: Some(edit), ..Damage::none() }),
        1 => arb_byte_edit().prop_map(|edit| Damage { bytes: Some(edit), ..Damage::none() }),
        1 => (
            option::of(plant()),
            option::of(arb_bit_edit()),
            option::of(arb_seal_edit()),
            option::of(arb_byte_edit()),
        )
            .prop_map(|(plant, bits, seal, bytes)| Damage { plant, bits, seal, bytes }),
    ]
}
