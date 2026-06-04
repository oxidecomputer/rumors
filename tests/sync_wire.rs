//! Wire-equivalence test for the *synchronous* gossip path:
//! `sync::Known::gossip` over `std::io::pipe`s must agree with
//! bidirectional `Known::learn`.

mod common;

use proptest::prelude::*;
use rumors::sync::Known;

use crate::common::action::{arb_local_actions, build_local};
use crate::common::oracle::readout;
use crate::common::sync_wire::sync_wire_gossip;

proptest! {
    /// `sync::Known::gossip` over `std::io::pipe`s yields the same live
    /// content as bidirectional `Known::learn`. Exercised with the
    /// shared `Insert`/`Redact` action shape so redactions cross the
    /// wire too (not just inserts).
    #[test]
    fn sync_gossip_matches_local_process(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        // One universe seed; alice and bob start as disjoint forks.
        let mut seed = Known::<u64>::seed();
        let mut a0 = build_local(seed.fork(), &a_actions);
        let mut b0 = build_local(seed.fork(), &b_actions);

        // Local-`learn` path runs on forks, leaving the originals for the wire
        // path. Each `learn` consumes a fresh fork of the counterpart.
        let mut a_proc = a0.fork();
        let mut b_proc = b0.fork();
        let a_snap = a_proc.fork();
        let b_snap = b_proc.fork();
        a_proc.join(b_snap).unwrap();
        b_proc.join(a_snap).unwrap();

        let (a_wire, b_wire) = sync_wire_gossip(a0, b0);

        prop_assert_eq!(readout(&a_proc), readout(&a_wire));
        prop_assert_eq!(readout(&b_proc), readout(&b_wire));
        prop_assert_eq!(readout(&a_wire), readout(&b_wire));
    }
}
