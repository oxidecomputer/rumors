//! The cost argument for updates of one sign, measured in bitset levels.
//!
//! The crate drops the limb row's logarithmic term for a stretch of one
//! sign that begins where `G`'s window begins. Its argument: no bitset word
//! empties during such a stretch, so the levels its insertions climb exceed
//! the insertions only by the words they fill, and those words number at
//! most one per 63 positions below the stretch's highest top, plus word
//! zero at each level once. The bitset's test-only counters measure both
//! sides of that inequality.
//!
//! The argument also says why a clear cannot begin such a stretch, by a
//! cycle that pays a level per base-64 digit of the width each round. That
//! cycle is measured here too, as the non-guarantee it documents.

use num_bigint::{BigInt as IBig, BigUint as UBig};
use proptest::prelude::*;

use super::{assert_value, from_limbs, Accumulator};

/// Read the bitset work counters of an accumulator's written set; all zero
/// while the set holds no bitset.
fn written_work(acc: &Accumulator) -> ClimbReading {
    let counts = acc.stored().digits.written_climb_counts();
    ClimbReading {
        insertions: counts.insertions,
        climbed: counts.inserted_levels_climbed,
        cleared: counts.cleared_levels_examined,
        levels: counts.levels,
    }
}

/// One reading of the bitset counters, named for the inequality checked.
#[derive(Clone, Copy, Debug)]
struct ClimbReading {
    /// Calls of the bitset's insertion.
    insertions: usize,
    /// Levels those insertions climbed, each counting its stopping level.
    climbed: usize,
    /// Levels the bitset's clears examined.
    cleared: usize,
    /// Levels the bitset holds.
    levels: usize,
}

proptest! {
    /// After a `<<=`, a stretch of one-sign limb updates at any shifts climbs
    /// at most one bitset level per insertion, plus one per 63 positions
    /// below its highest top, plus one per level, and clears nothing.
    ///
    /// The `<<=` moves the value into new digits with a new written set, so
    /// the stretch begins where the crate's argument says one may. The
    /// updates land anywhere in the first 12,288 bits: in the gap below the
    /// shifted value, inside it, and above its top, so they convert the set
    /// to a bitset, insert into it below the run, and move runs into it. Every word an insertion fills covers a held
    /// position at or below the top, and none empties again, which is the
    /// bound: the levels climbed less the insertions are at most `P / 63`
    /// plus word zero at each level. No update clears the set.
    #[test]
    fn a_one_sign_stretch_after_a_shift_climbs_once_per_insertion_and_fill(
        start in proptest::collection::vec(any::<u64>(), 1..=4),
        shift in 1u64..=8_192,
        updates in proptest::collection::vec(
            (proptest::collection::vec(any::<u64>(), 1..=2), 0u64..=12_288),
            1..=64,
        ),
        negative: bool,
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        let apply = |acc: &mut Accumulator, oracle: &mut IBig, limbs: &[u64], at: u64| {
            let scaled = IBig::from(from_limbs(limbs)) << usize::try_from(at).unwrap();
            if negative {
                acc.sub_shifted_limbs(at, limbs.iter().copied());
                *oracle -= scaled;
            } else {
                acc.add_shifted_limbs(at, limbs.iter().copied());
                *oracle += scaled;
            }
        };
        apply(&mut acc, &mut oracle, &start, 0);
        acc <<= shift;
        oracle <<= usize::try_from(shift).unwrap();
        let before = written_work(&acc);
        for (limbs, at) in &updates {
            apply(&mut acc, &mut oracle, limbs, *at);
        }
        let after = written_work(&acc);
        let top = acc.stored_digit_count() - 1;
        let insertions = after.insertions - before.insertions;
        let climbed = after.climbed - before.climbed;
        prop_assert_eq!(
            after.cleared, before.cleared,
            "a stretch of one sign clears nothing"
        );
        prop_assert!(
            climbed <= insertions + top / 63 + after.levels,
            "{} levels climbed by {} insertions below top {} in {} levels: the fills exceed \
             one per 63 positions plus word zero at each level",
            climbed,
            insertions,
            top,
            after.levels
        );
        assert_value(&acc, &oracle);
    }
}

/// Each round of a clear, a re-raise with no growth, and one limb at `64^m`
/// below it climbs `m + 1` bitset levels for the limb and `m + 1` levels
/// for the next clear, at every `m` drawn.
///
/// This is the cycle the crate's cost argument gives for why a clear cannot
/// begin a stretch of one sign. The clear leaves `G`'s window running and
/// the bitset's storage covering the width reached, but empties its words.
/// The limb at position `q + 2` re-raises the top with a `G` of zero, and
/// its jump finds position zero held: one level. The limb at `q = 64^m`
/// lands in the gap below, fills one word at each level from 0 to `m - 1`,
/// and stops at word zero of level `m`. The clear that follows zeroes the
/// held positions `q` and `q + 2` and examines `m + 1` levels, emptying those
/// words again. The first round differs, while the storage grows, so
/// the exact counts are asserted from the second round on.
#[test]
fn clear_cycle_climbs_a_level_per_base_64_digit_each_round() {
    for m in 1..=4u32 {
        let q = 64u64.pow(m);
        let mut acc = Accumulator::new();
        for round in 0..4 {
            let start = written_work(&acc);
            acc.add_shifted_limbs(32 * (q + 2), [1]);
            let raised = written_work(&acc);
            acc.add_shifted_limbs(32 * q, [1]);
            let filled = written_work(&acc);
            assert_value(
                &acc,
                &IBig::from(
                    (UBig::from(1u8) << usize::try_from(32 * (q + 2)).unwrap())
                        + (UBig::from(1u8) << usize::try_from(32 * q).unwrap()),
                ),
            );
            acc.clear();
            let cleared = written_work(&acc);
            if round == 0 {
                continue;
            }
            assert_eq!(
                raised.climbed - start.climbed,
                1,
                "m = {m}, round {round}: the re-raise's jump climbs one level"
            );
            assert_eq!(
                filled.climbed - raised.climbed,
                m as usize + 1,
                "m = {m}, round {round}: the limb at 64^{m} climbs m + 1 levels"
            );
            assert_eq!(
                cleared.cleared - filled.cleared,
                m as usize + 1,
                "m = {m}, round {round}: the clear examines m + 1 levels"
            );
        }
    }
}
