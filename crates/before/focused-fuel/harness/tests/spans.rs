//! Fused span construction: one sweep feeds both endpoints of a hull.
//!
//! Two regimes: the pair regime (binary `span`, and `span_all`'s leaf
//! combines) shares its operands between the meet and join legs, so the
//! pair's streams are decoded once where the composed emitters decode them
//! twice; the interior regime (`span_all`'s combines over merged hulls) reads
//! a different operand pair per leg, so it consolidates without a shared
//! walk. The fused span decode likewise parses its second component once.

use focused_fuel_harness::{
    assert_at_most, assert_equal_difference, assert_less, assert_saving_grows, measure, Probe,
};
use focused_fuel_protocol::{pack, Snapshot};

use Snapshot::{Divergent, Middle, Start};

/// Senders of the history fixture, each size twice the last.
const SENDERS: [u64; 4] = [6, 12, 24, 48];

/// A comparable pair's hull is the pair handed back at the cost of one
/// comparison (`span_is_the_pair_hull`), with no emission.
///
/// Form: equal difference (F-diff) of `span` over the comparison of the same
/// pair, over the history's size.
#[test]
fn comparable_span_costs_one_comparison() {
    let pair = pack([Start, Middle]);
    assert_equal_difference(
        (Probe::HistorySpan, pair),
        (Probe::HistoryCompare, pair),
        &SENDERS,
    );
}

/// A concurrent pair's hull decodes the pair once and folds each crossing
/// into one shared running difference.
///
/// So it costs less than the meet, the join, and the classifying comparison
/// composed, by a saving that grows with the streams and is at least half of
/// one decode of each operand.
///
/// The composed emitters decode each operand once more than the fused hull
/// and keep a second running difference. One decode of an operand is priced
/// as half the comparison of that operand with its byte-equal re-decode,
/// which walks it twice; the price also carries half that comparison's
/// running-difference work. The floor is half that price for each operand:
/// the hull saves at least half a decode of each. Fuel does not separate the
/// saved decode from the saved accumulator, so this bound does not judge
/// either half alone. Forms: strict saving (F-lt) of `span` under the
/// composition, with a growing saving, and derived bound (F-bound) of a
/// quarter of the two operands' re-decode comparisons under the saving.
#[test]
fn concurrent_span_shares_the_pair_walk_and_its_folds() {
    let pair = pack([Middle, Divergent]);
    let rows: Vec<(u64, u64, u64)> = SENDERS
        .iter()
        .map(|&senders| {
            let fused = measure(Probe::HistorySpan, senders, pair);
            let composed = measure(Probe::HistoryMeet, senders, pair)
                + measure(Probe::HistoryJoin, senders, pair)
                + measure(Probe::HistoryCompare, senders, pair);
            (senders, fused, composed)
        })
        .collect();
    assert_saving_grows("fused hull vs meet, join, and comparison", &rows);
    for &(senders, fused, composed) in &rows {
        let decodes = measure(Probe::HistoryRedecodedCompare, senders, pack([Middle]))
            + measure(Probe::HistoryRedecodedCompare, senders, pack([Divergent]));
        assert_at_most(
            &format!(
                "half of one decode of each operand vs the hull's saving at {senders} senders"
            ),
            decodes / 4,
            composed - fused,
        );
    }
}

/// `span_all`'s leaf combines use the fused pair walk: at one item the n-ary
/// hull costs the binary span plus a fixed amount.
///
/// Form: equal difference (F-diff) of `span_all` over `span` on the same
/// pair, over the history's size.
#[test]
fn span_all_leaf_combine_is_the_fused_pair_walk() {
    let pair = pack([Start, Middle]);
    assert_equal_difference(
        (Probe::HistorySpanAll, pair),
        (Probe::HistorySpan, pair),
        &SENDERS,
    );
}

/// The n-ary hull costs less than the composed meet and join folds, and more
/// than half of them: it fuses the leaf level alone, while each interior
/// combine reads a different operand pair per leg.
///
/// Form: strict saving (F-lt) of `span_all` under the composition, and of
/// the composition under twice `span_all`, over the identical population.
#[test]
fn span_all_fuses_the_leaf_level() {
    for senders in SENDERS {
        let fused = measure(Probe::PopulationSpanAll, senders, 0);
        let composed = measure(Probe::PopulationMeetAll, senders, 0)
            + measure(Probe::PopulationJoinAll, senders, 0);
        assert_less(
            &format!("span_all vs the composed folds at {senders} senders"),
            fused,
            composed,
        );
        assert_less(
            &format!("the composed folds vs twice span_all at {senders} senders"),
            composed,
            2 * fused,
        );
    }
}

/// Fused span decoding parses the second component inside its dominance
/// check, omitting that component's standalone parse and validation-height
/// accumulator.
///
/// So the fused decode costs less than decoding both components and then
/// comparing them, by a saving that grows with the streams and is at least
/// half the second component's standalone decode. The composition pays that
/// whole decode, and a fused decode that kept it would save nothing; the
/// floor sits midway, derived from the mechanism rather than from any
/// reading. Forms: strict saving (F-lt) of the fused decode under the
/// composition, with a growing saving, and derived bound (F-bound) of half
/// the second component's decode under the saving.
#[test]
fn span_decode_parses_the_second_component_once() {
    let span = pack([Start, Middle]);
    let mut rows = Vec::new();
    for senders in SENDERS {
        let fused = measure(Probe::SpanDecode, senders, span);
        let upper = measure(Probe::SpanDecodeUpper, senders, span);
        let composed = measure(Probe::SpanDecodeLower, senders, span)
            + upper
            + measure(Probe::HistoryCompare, senders, span);
        assert_at_most(
            &format!("half the second decode vs the saving at {senders} senders"),
            upper / 2,
            composed - fused,
        );
        rows.push((senders, fused, composed));
    }
    assert_saving_grows("fused decode vs two decodes and a comparison", &rows);
}
