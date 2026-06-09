//! Wire-equivalence test for the *synchronous* gossip path:
//! `sync::Known::gossip` over `std::io::pipe`s must agree with
//! bidirectional `Known::learn`.

mod common;

use proptest::prelude::*;
use rumors::sync::Known;

use crate::common::action::{arb_local_actions, build_local};
use crate::common::oracle::readout;
use crate::common::sync_wire::{sync_bootstrap_fork, sync_wire_gossip};

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
        // One universe seed; alice and bob start as genuine party-disjoint
        // originators so they can each `message`/`redact` independently.
        let seed = Known::<u64>::seed();
        let a0 = build_local(sync_bootstrap_fork(&seed), &a_actions);
        let b0 = build_local(sync_bootstrap_fork(&seed), &b_actions);

        // Genuine party-disjoint copies of each side's content for the oracle;
        // a0/b0 themselves go on to the wire. `join` merges content from a
        // `rumors` snapshot of the counterpart.
        let mut a_proc = sync_bootstrap_fork(&a0);
        let mut b_proc = sync_bootstrap_fork(&b0);
        let a_snap = a_proc.rumors();
        let b_snap = b_proc.rumors();
        a_proc.join(b_snap).unwrap();
        b_proc.join(a_snap).unwrap();

        let (a_wire, b_wire) = sync_wire_gossip(a0, b0);

        prop_assert_eq!(readout(&a_proc), readout(&a_wire));
        prop_assert_eq!(readout(&b_proc), readout(&b_wire));
        prop_assert_eq!(readout(&a_wire), readout(&b_wire));
    }
}
