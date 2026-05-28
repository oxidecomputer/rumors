//! Wire-equivalence test for the *synchronous* gossip path:
//! `sync::Local::gossip` over `std::io::pipe`s must agree with
//! bidirectional `Local::process`.

mod common;

use proptest::prelude::*;
use rumors::sync::ignore;

use crate::common::action::{arb_local_actions, build_local};
use crate::common::oracle::readout;
use crate::common::sync_wire::sync_wire_gossip;

proptest! {
    /// `sync::Local::gossip` over `std::io::pipe`s yields the same live
    /// content as bidirectional `Local::process`. Exercised with the
    /// shared `Insert`/`Redact` action shape so redactions cross the
    /// wire too (not just inserts).
    #[test]
    fn sync_gossip_matches_local_process(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let a0 = build_local("alice", &a_actions);
        let b0 = build_local("bob", &b_actions);

        let mut a_proc = a0.clone();
        let mut b_proc = b0.clone();
        let a_snap = a_proc.clone();
        let b_snap = b_proc.clone();
        a_proc.process(b_snap, ignore);
        b_proc.process(a_snap, ignore);

        let (a_wire, b_wire) = sync_wire_gossip(a0, b0);

        prop_assert_eq!(readout(&a_proc), readout(&a_wire));
        prop_assert_eq!(readout(&b_proc), readout(&b_wire));
        prop_assert_eq!(readout(&a_wire), readout(&b_wire));
    }
}
