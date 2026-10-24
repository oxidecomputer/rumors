//! The n-ary folds' duplicate filter: a run of copies sharing one buffer
//! folds like one copy.

use focused_fuel_harness::{assert_equal_difference, Probe};

/// The duplicate fixture's two widths in shares, a fourfold step apart.
const SHARES: [u64; 2] = [16, 64];

/// Every fold entry point folds a run of shared-buffer copies of one input
/// as it folds that input once, plus a cost per copy that does not depend on
/// the copy's size.
///
/// Each fold reads `x` and then `k` clones of `d`, a version concurrent with
/// `x`. With the filter, the balanced counter sees `x` and one `d` at every
/// `k`; without it, every further counter level runs the kernel again, at a
/// cost that grows with the versions. Form: equal difference (F-diff) of the
/// fold over `k` copies, for `k` of 2, 3, and 8, over the fold over one copy,
/// across the fixture's two widths.
#[test]
fn duplicate_runs_fold_like_one_copy() {
    for probe in [
        Probe::DuplicateVersionJoinAll,
        Probe::DuplicateVersionMeetAll,
        Probe::DuplicateVersionSpanAll,
        Probe::DuplicateSumBorrowed,
        Probe::DuplicateSumOwned,
        Probe::DuplicateCollectOwned,
        Probe::DuplicateCollectBorrowed,
        Probe::DuplicateSpanUnionAll,
        Probe::DuplicateSpanIntersectAll,
        Probe::DuplicateSpanJoinAll,
        Probe::DuplicateSpanMeetAll,
        Probe::DuplicatePointJoinAll,
        Probe::DuplicateOptionSpanSumBorrowedSpans,
        Probe::DuplicateOptionSpanSumOwnedSpans,
        Probe::DuplicateOptionSpanSumBorrowedVersions,
        Probe::DuplicateOptionSpanSumOwnedVersions,
        Probe::DuplicateOptionSpanCollectBorrowedSpans,
        Probe::DuplicateOptionSpanCollectOwnedSpans,
        Probe::DuplicateOptionSpanCollectBorrowedVersions,
        Probe::DuplicateOptionSpanCollectOwnedVersions,
        Probe::DuplicateOptionSpanProductBorrowedSpans,
        Probe::DuplicateOptionSpanProductOwnedSpans,
    ] {
        for copies in [2, 3, 8] {
            assert_equal_difference((probe, copies), (probe, 1), &SHARES);
        }
    }
}
