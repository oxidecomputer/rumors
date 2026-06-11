//! Convergence test for the *synchronous* gossip path:
//! `sync::Known::gossip` over `std::io::pipe`s must converge both peers on
//! the union of their pre-session live content, and must agree with the
//! asynchronous path: the two surfaces drive the same protocol.

mod common;

use proptest::prelude::*;
use rumors::sync::Known;

use crate::common::action::{arb_local_actions, build_local};
use crate::common::oracle::readout;
use crate::common::sync_wire::{sync_bootstrap_fork, sync_wire_gossip};

proptest! {
    /// `sync::Known::gossip` over `std::io::pipe`s converges both peers on
    /// the union of the two pre-session readouts (sound because the peers
    /// tick disjoint parties, never share keys, and only redact keys they
    /// themselves minted before the session). Exercised with the shared
    /// `Insert`/`Redact` action shape so redactions cross the wire too.
    #[test]
    fn sync_gossip_converges_on_the_union(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        // One universe seed; alice and bob start as genuine party-disjoint
        // originators so they can each `send`/`redact` independently.
        let mut seed = Known::<u64>::seed();
        let mut a = build_local(sync_bootstrap_fork(&mut seed), &a_actions);
        let mut b = build_local(sync_bootstrap_fork(&mut seed), &b_actions);

        let mut expected = readout(&a.snapshot());
        expected.extend(readout(&b.snapshot()));

        sync_wire_gossip(&mut a, &mut b);

        prop_assert_eq!(readout(&a.snapshot()), expected.clone());
        prop_assert_eq!(readout(&b.snapshot()), expected);
        prop_assert_eq!(a.hash(), b.hash());
        prop_assert_eq!(a.latest(), b.latest());
    }
}
