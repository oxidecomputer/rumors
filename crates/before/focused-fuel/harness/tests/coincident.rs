//! The coincident-span rungs: a span whose endpoints are clones of one
//! version collapses its queries, and coincident endpoints in distinct
//! buffers do not.

use focused_fuel_harness::{assert_differ, assert_equal_difference, assert_less, measure, Probe};
use focused_fuel_protocol::{pack, Snapshot};

use Snapshot::{Divergent, End, Middle, Start};

/// Senders of the history fixture, each size twice the last.
const SENDERS: [u64; 4] = [6, 12, 24, 48];

/// `place` on a clone-coincident span collapses to one pairwise comparison,
/// while coincident endpoints in distinct buffers take the fused
/// three-stream walk.
///
/// Forms: equal difference (F-diff) of the coincident `place` over the
/// comparison of the probe with the endpoint, over the history's size; and
/// strict saving (F-lt) of that comparison under the distinct-buffer
/// `place`.
#[test]
fn coincident_place_collapses_to_the_pair_sweep() {
    let pair = pack([Middle, Divergent]);
    assert_equal_difference(
        (Probe::CoincidentPlace, pair),
        (Probe::HistoryCompare, pack([Divergent, Middle])),
        &SENDERS,
    );
    for senders in SENDERS {
        assert_less(
            &format!("collapsed vs walked placement at {senders} senders"),
            measure(Probe::HistoryCompare, senders, pack([Divergent, Middle])),
            measure(Probe::DistinctPlace, senders, pair),
        );
    }
}

/// `dominance` on a clone-coincident span collapses to one containment,
/// while coincident endpoints in distinct buffers take the fused walk.
///
/// The fused walk's dominance exit can cost less than the collapsed
/// containment on a refuting probe, so the walking leg asserts only that
/// the distinct-buffer reading differs from the coincident one: had the
/// clone rung fired across buffers, the two would be the same path and read
/// the same. Forms: equal difference (F-diff) of the coincident `dominance`
/// over `before(probe).contains(endpoint)`, over the history's size; and
/// different paths (F-ne) of the coincident and distinct-buffer readings.
#[test]
fn coincident_dominance_collapses_to_one_containment() {
    let pair = pack([Middle, Divergent]);
    assert_equal_difference(
        (Probe::CoincidentDominance, pair),
        (Probe::HistoryBeforeBound, pack([Divergent, Middle])),
        &SENDERS,
    );
    for senders in SENDERS {
        assert_differ(
            &format!("coincident vs distinct-buffer dominance at {senders} senders"),
            measure(Probe::CoincidentDominance, senders, pair),
            measure(Probe::DistinctDominance, senders, pair),
        );
    }
}

/// Every version-shaped argument to `contains` takes the fused membership
/// walk, while a coincident span in distinct buffers takes the general
/// endpoint comparisons, which decode the probe once per endpoint.
///
/// Forms: equal difference (F-diff) of the owned-version and point-span
/// arguments over the borrowed version, over the history's size; and strict
/// saving (F-lt) of the borrowed version under the distinct-buffer span, on
/// a contained probe, where both confirm only at exhaustion.
#[test]
fn coincident_argument_collapses_to_the_membership_walk() {
    let contained = pack([Start, End, Middle]);
    for shape in [Probe::SpanContainsOwned, Probe::SpanContainsPoint] {
        assert_equal_difference(
            (shape, contained),
            (Probe::SpanContains, contained),
            &SENDERS,
        );
    }
    for senders in SENDERS {
        assert_less(
            &format!("membership walk vs endpoint comparisons at {senders} senders"),
            measure(Probe::SpanContains, senders, contained),
            measure(Probe::SpanContainsDistinctPoint, senders, contained),
        );
    }
}
