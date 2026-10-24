//! Rank encodings and their format's own defects.

use num_bigint::BigUint;
use proptest::prelude::*;

use super::damage::{arb_bit_edit, arb_byte_edit, dirty_padding, BitEdit, ByteEdit};
use super::{with_unstructured, Expect, NearValid, Regime};
use crate::rank::DECODE_CHUNK_BYTES;
use crate::testing::oracles::codec::{pack, rank_bits};

/// A normalized rank.
///
/// The integral part is below 4, any `u64`, or `2^k` or `2^k - 1` for `k`
/// from 60 to 399. The fraction draws up to 199 digits, or 200 to 2,999 one
/// time in five, and loses its trailing zeros.
fn arb_rank_value() -> impl Strategy<Value = (BigUint, u64)> {
    (
        prop_oneof![
            4 => (0u64..4).prop_map(BigUint::from),
            2 => any::<u64>().prop_map(BigUint::from),
            1 => (60u32..400).prop_map(|k| (BigUint::from(1u8) << k) - 1u32),
            1 => (60u32..400).prop_map(|k| BigUint::from(1u8) << k),
        ],
        prop_oneof![
            4 => prop::collection::vec(any::<bool>(), 0..200),
            1 => prop::collection::vec(any::<bool>(), 200..3000),
        ],
    )
        .prop_map(|(integral, mut digits)| {
            while digits.last() == Some(&false) {
                digits.pop();
            }
            let exp = digits.len() as u64;
            let num = digits.into_iter().fold(integral, |num, digit| {
                (num << 1u32) | BigUint::from(u8::from(digit))
            });
            (num, exp)
        })
}

/// The damage done to a rank encoding.
#[derive(Clone, Debug)]
enum RankDamage {
    /// Prefixes a run of this many ones, at least 64 where the format allows
    /// at most 63, and its terminating zero.
    TooWideHeader(u8),
    /// Replaces the closing bit with one more group of eight zero digits,
    /// then closes.
    ZeroFinalGroup,
    /// Sets the last byte's padding bits where this nonzero byte's high bits
    /// are set, or appends the byte when the stream ends on a byte boundary.
    ///
    /// A byte whose high bits are all zero sets nothing and leaves the
    /// encoding canonical.
    DirtyPadding(u8),
    /// Edits the stream's bits before packing.
    Bits(BitEdit),
    /// Edits the packed bytes.
    Bytes(ByteEdit),
}

/// Near-valid rank encodings: the canonical encoding four times in nine, and
/// otherwise one kind of damage.
///
/// Regimes: [`Regime::LongRank`], [`Regime::TooWideRankHeader`],
/// [`Regime::ZeroFinalGroup`], and [`Regime::DirtyRankPadding`].
pub(crate) fn arb_rank_encoding() -> impl Strategy<Value = NearValid> {
    let damage = prop_oneof![
        4 => Just(None),
        1 => (64u8..=69).prop_map(|ones| Some(RankDamage::TooWideHeader(ones))),
        1 => Just(Some(RankDamage::ZeroFinalGroup)),
        1 => (1u8..=255).prop_map(|byte| Some(RankDamage::DirtyPadding(byte))),
        1 => arb_bit_edit().prop_map(|edit| Some(RankDamage::Bits(edit))),
        1 => arb_byte_edit().prop_map(|edit| Some(RankDamage::Bytes(edit))),
    ];
    with_unstructured(
        (arb_rank_value(), damage).prop_map(|((num, exp), damage)| damaged_rank(&num, exp, damage)),
    )
}

/// A rank encoding under `damage`, with its regimes.
fn damaged_rank(num: &BigUint, exp: u64, damage: Option<RankDamage>) -> NearValid {
    let bits = rank_bits(num, exp);
    let (bytes, expect, regime) = match damage {
        None => {
            let bytes = pack(&bits);
            let long = bytes.len() > DECODE_CHUNK_BYTES;
            (bytes, Expect::Canonical, long.then_some(Regime::LongRank))
        }
        Some(RankDamage::TooWideHeader(ones)) => {
            let mut widened = vec![true; usize::from(ones)];
            widened.push(false);
            widened.extend(bits);
            (
                pack(&widened),
                Expect::Defective,
                Some(Regime::TooWideRankHeader),
            )
        }
        Some(RankDamage::ZeroFinalGroup) => {
            let mut padded = bits;
            padded.pop();
            padded.push(true);
            padded.extend([false; 8]);
            padded.push(false);
            (
                pack(&padded),
                Expect::Defective,
                Some(Regime::ZeroFinalGroup),
            )
        }
        Some(RankDamage::DirtyPadding(byte)) => {
            let mut bytes = pack(&bits);
            let dirtied = if bits.len().is_multiple_of(8) {
                bytes.push(byte);
                true
            } else {
                dirty_padding(&mut bytes, bits.len(), byte)
            };
            if dirtied {
                (bytes, Expect::Defective, Some(Regime::DirtyRankPadding))
            } else {
                (bytes, Expect::Canonical, None)
            }
        }
        Some(RankDamage::Bits(edit)) => (pack(&edit.apply(bits)), Expect::Open, None),
        Some(RankDamage::Bytes(edit)) => (edit.apply(pack(&bits)), Expect::Open, None),
    };
    NearValid {
        bytes,
        expect,
        regimes: regime.into_iter().collect(),
        split: None,
    }
}
