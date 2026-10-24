//! Causal filtering and span placement: the fused walks decode each stream
//! once and keep the early exits of the compositions they replace.
//!
//! Every claim is stated against the pair comparison on the same operands,
//! over the history fixture (`focused_fuel_protocol::Snapshot` states its
//! construction and order) at three sizes.

use focused_fuel_harness::{assert_at_most, assert_less, assert_saving_grows, measure, Probe};
use focused_fuel_protocol::{pack, Snapshot};

use Snapshot::{Divergent, End, Middle, Start, Top};

/// Senders of the history fixture, each size twice the last.
const SENDERS: [u64; 4] = [6, 12, 24, 48];

/// Measures `probe` over the history of `senders` senders on `snapshots`.
fn history<const N: usize>(probe: Probe, senders: u64, snapshots: [Snapshot; N]) -> u64 {
    measure(probe, senders, pack(snapshots))
}

/// Measures `x.partial_cmp(y)` over the history of `senders` senders.
fn compare(senders: u64, x: Snapshot, y: Snapshot) -> u64 {
    history(Probe::HistoryCompare, senders, [x, y])
}

/// A query over a single stored bound answers at its first refuted
/// direction.
///
/// A refuted ceiling and a satisfied hole cost less than the pair
/// comparison, which must refute both directions or confirm one to
/// exhaustion, and a concurrent ceiling costs no more than it.
///
/// Forms: strict saving (F-lt) of the refuted ceiling and the satisfied hole
/// under the comparison of the same pair, and derived bound (F-bound) of the
/// concurrent ceiling by the comparison's reading.
#[test]
fn query_single_bound_exits_at_the_first_refuted_direction() {
    for senders in SENDERS {
        let raw = compare(senders, End, Middle);
        assert_less(
            &format!("refuted ceiling vs the sweep at {senders} senders"),
            history(Probe::HistoryBefore, senders, [Middle, End]),
            raw,
        );
        assert_less(
            &format!("satisfied hole vs the sweep at {senders} senders"),
            history(Probe::HistorySince, senders, [Middle, End]),
            raw,
        );
        assert_at_most(
            &format!("concurrent ceiling vs the sweep at {senders} senders"),
            history(Probe::HistoryBefore, senders, [Divergent, Middle]),
            compare(senders, Middle, Divergent),
        );
    }
}

/// The composition's early exit on a probe concurrent with the ceiling
/// survives the fusion: the refuted containment answers at the deciding
/// interval, so the fused walk costs less than the two comparisons.
///
/// Form: strict saving (F-lt) of the fused walk under the composition.
#[test]
fn query_concurrent_ceiling_exit_survives_the_fusion() {
    for senders in SENDERS {
        assert_less(
            &format!("concurrent with the ceiling at {senders} senders"),
            history(
                Probe::HistorySinceBefore,
                senders,
                [Start, Divergent, Middle],
            ),
            compare(senders, Middle, Start) + compare(senders, Middle, Divergent),
        );
    }
}

/// One fused span placement decodes each stream once, and its coarsenings
/// never cost more than it.
///
/// On a full sweep, `place` costs less than the two comparisons with the
/// endpoints, by a saving that grows with the streams. On a contained probe,
/// `dominance` drops the end stream at its refuted domination and
/// `precedence` the start stream, so both cost less than `place`, while
/// `contains`, whose two directions confirm only at exhaustion, costs no
/// more. On a probe dominating the whole span `dominance` costs no more than
/// `place`, and on one preceding it `precedence` costs no more.
///
/// Forms: strict saving (F-lt) for `place` under the composition, with a
/// growing saving, and for the coarsenings that drop a stream; derived bound
/// (F-bound) by `place`'s reading for each coarsening that refutes nothing.
#[test]
fn span_place_decodes_each_stream_once() {
    let rows: Vec<(u64, u64, u64)> = SENDERS
        .iter()
        .map(|&senders| {
            let place = history(Probe::SpanPlace, senders, [Start, End, Middle]);
            let composed = compare(senders, Middle, Start) + compare(senders, Middle, End);
            (senders, place, composed)
        })
        .collect();
    assert_saving_grows("fused placement vs two comparisons", &rows);
    for &(senders, place, _) in &rows {
        assert_less(
            &format!("dominance vs placement at {senders} senders"),
            history(Probe::SpanDominance, senders, [Start, End, Middle]),
            place,
        );
        assert_less(
            &format!("precedence vs placement at {senders} senders"),
            history(Probe::SpanPrecedence, senders, [Start, End, Middle]),
            place,
        );
    }
    for (lo, hi, probe, coarsening) in [
        (Start, End, Middle, Probe::SpanContains),
        (Start, Middle, End, Probe::SpanDominance),
        (Middle, End, Start, Probe::SpanPrecedence),
    ] {
        for senders in SENDERS {
            assert_at_most(
                &format!("{coarsening:?} of {probe:?} in [{lo:?}, {hi:?}] at {senders} senders"),
                history(coarsening, senders, [lo, hi, probe]),
                history(Probe::SpanPlace, senders, [lo, hi, probe]),
            );
        }
    }
}

/// The span walk's concurrency exit fires on a probe concurrent with both
/// endpoints, so it costs less than the two comparisons with the endpoints.
///
/// The walk returns at the second deciding interval, so the saving grows with
/// the streams.
///
/// Form: strict saving (F-lt) of `place` under the composition, with a
/// growing saving.
#[test]
fn span_concurrent_exit_survives_the_fusion() {
    let rows: Vec<(u64, u64, u64)> = SENDERS
        .iter()
        .map(|&senders| {
            let place = history(Probe::SpanPlace, senders, [Middle, End, Divergent]);
            let composed = compare(senders, Divergent, Middle) + compare(senders, Divergent, End);
            (senders, place, composed)
        })
        .collect();
    assert_saving_grows("place of a probe concurrent with both endpoints", &rows);
}

/// The dominance query stops when the probe fails to dominate the start,
/// whether the start is concurrent with the probe or above it.
///
/// A concurrent start refutes `lo <= probe` at its first opposing interval,
/// so the walk returns before full resolution and under the two-check shape
/// it replaces: the start's comparison with the probe, then the end's
/// containment in the probe's past. A dominating start is refuted at the
/// first excess interval, under even the first check, which confirms only at
/// exhaustion. Form: strict saving (F-lt) in each case.
#[test]
fn dominance_bails_at_the_refuted_start() {
    for senders in SENDERS {
        let fused = history(Probe::SpanDominance, senders, [Divergent, Top, Middle]);
        let first = compare(senders, Divergent, Middle);
        let second = history(Probe::HistoryBeforeBound, senders, [Middle, Top]);
        assert_less(
            &format!("concurrent start: dominance vs placement at {senders} senders"),
            fused,
            history(Probe::SpanPlace, senders, [Divergent, Top, Middle]),
        );
        assert_less(
            &format!("concurrent start: dominance vs two checks at {senders} senders"),
            fused,
            first + second,
        );

        let fused = history(Probe::SpanDominance, senders, [Middle, End, Start]);
        let first = compare(senders, Middle, Start);
        let second = history(Probe::HistoryBeforeBound, senders, [Start, End]);
        assert_less(
            &format!("dominating start: dominance vs the first check at {senders} senders"),
            fused,
            first,
        );
        assert_less(
            &format!("dominating start: dominance vs two checks at {senders} senders"),
            fused,
            first + second,
        );
    }
}

/// The precedence query stops when the probe fails to precede the end,
/// whether the end is concurrent with the probe or below it: the dominance
/// bail, mirrored.
///
/// The two-check shape it replaces is the end's comparison with the probe,
/// then the start's containment in the probe's future. Form: strict saving
/// (F-lt) in each case.
#[test]
fn precedence_bails_at_the_refuted_end() {
    for senders in SENDERS {
        let fused = history(Probe::SpanPrecedence, senders, [Start, Divergent, Middle]);
        let first = compare(senders, Divergent, Middle);
        let second = history(Probe::HistoryAfterBound, senders, [Middle, Start]);
        assert_less(
            &format!("concurrent end: precedence vs placement at {senders} senders"),
            fused,
            history(Probe::SpanPlace, senders, [Start, Divergent, Middle]),
        );
        assert_less(
            &format!("concurrent end: precedence vs two checks at {senders} senders"),
            fused,
            first + second,
        );

        let fused = history(Probe::SpanPrecedence, senders, [Start, Middle, End]);
        let first = compare(senders, Middle, End);
        let second = history(Probe::HistoryAfterBound, senders, [End, Start]);
        assert_less(
            &format!("preceded end: precedence vs the first check at {senders} senders"),
            fused,
            first,
        );
        assert_less(
            &format!("preceded end: precedence vs two checks at {senders} senders"),
            fused,
            first + second,
        );
    }
}

/// The membership query stops at the first refuted direction, on either
/// side, before the placement walk, which confirms its `After` or `Before`
/// verdict only at exhaustion.
///
/// Form: strict saving (F-lt) of `contains` under `place` for a probe above
/// the end and for one below the start.
#[test]
fn contains_bails_at_either_refuted_side() {
    for senders in SENDERS {
        for (lo, hi, probe) in [(Start, Middle, End), (Middle, End, Start)] {
            assert_less(
                &format!("contains vs place of {probe:?} in [{lo:?}, {hi:?}] at {senders} senders"),
                history(Probe::SpanContains, senders, [lo, hi, probe]),
                history(Probe::SpanPlace, senders, [lo, hi, probe]),
            );
        }
    }
}
