//! Group F — sanity (panic-freedom and clone independence).

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::Local;

use crate::oracle::readout_multiset;
use crate::peer::quiesce;
use crate::schedule::{arb_schedule, execute_and_quiesce};

proptest! {
    /// F1: arbitrary schedules complete without panicking and produce
    /// a finite converged state. The implicit safety net for every
    /// other invariant in the suite — if this fails, the others
    /// cannot run.
    #[test]
    fn f1_no_panics(schedule in arb_schedule(2..=8, 50)) {
        let _ = execute_and_quiesce(&schedule);
    }

    /// F2: cloning is non-destructive — a clone that ingests new
    /// values, when recombined with the original via `+`, gives a
    /// content multiset equal to the original directly ingesting
    /// those values. (This is the documented use case: clones drive
    /// remote gossip in parallel; mutation happens on one side and
    /// recombines.)
    #[test]
    fn f2_clone_independence(
        original_values in vec(any::<u64>(), 0..=8),
        helper_values in vec(any::<u64>(), 0..=8),
    ) {
        let mut base: Local<u64> = Local::for_party("alice");
        base.message(original_values.clone(), |_, _, _| {});

        // Helper is a clone that ingests new values; original stays untouched.
        let mut helper = base.clone();
        helper.message(helper_values.clone(), |_, _, _| {});

        // Recombine: helper's new content flows back into base.
        let recombined = base.clone() + helper;

        // Compare against the original Local also ingesting the helper's values.
        let mut direct = base;
        direct.message(helper_values, |_, _, _| {});

        prop_assert_eq!(
            readout_multiset(&recombined),
            readout_multiset(&direct),
        );
    }

    /// F2b: quiesce on a degenerate input (zero or one peers) returns
    /// trivially and doesn't panic.
    #[test]
    fn f2_quiesce_degenerate(_anything in any::<u8>()) {
        let mut zero: Vec<crate::peer::Peer<u64>> = Vec::new();
        quiesce(&mut zero);
        let mut one = vec![crate::peer::Peer::<u64>::new("alone")];
        quiesce(&mut one);
        prop_assert!(true);
    }
}
