//! Convergence test for the *asynchronous* gossip path:
//! `rumors::Known::gossip` driven concurrently with `tokio::join!` over a
//! `tokio::io::duplex` pipe must converge both peers on the union of their
//! pre-session live content. Mirrors `sync_wire.rs`, which exercises the
//! synchronous `sync::Known::gossip` path over `std::io::pipe`s.
//!
//! (The old in-process `Known::join` is gone — wire gossip *is* the merge —
//! so the oracle is the abstract union of the two pre-session readouts:
//! sound because the peers tick disjoint parties, never share keys, and
//! only ever redact keys they themselves minted before the session.)
//!
//! Both tests share the `Insert`/`Redact` action shape, so redactions cross
//! the wire too (not just inserts), and run against both a primitive (`u64`)
//! and a non-primitive (`String`) value type to cover the borsh round-trip.

mod common;

use proptest::prelude::*;
use rumors::Known;

use crate::common::action::{arb_local_actions, arb_string_actions, build_local_async};
use crate::common::oracle::readout;
use crate::common::wire::{bootstrap_fork, wire_gossip};

proptest! {
    /// Driving two async `Known`s through `Known::gossip` over a
    /// `tokio::io::duplex` pipe converges both on the union of the two
    /// pre-session readouts — content already redacted on one side never
    /// reaches the other, and both sides end byte-identical (`hash`) and
    /// causally equal (`latest`).
    #[test]
    fn async_gossip_converges_on_the_union(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let mut seed = Known::<u64>::seed();
        let mut a = build_local_async(bootstrap_fork(&mut seed), &a_actions);
        let mut b = build_local_async(bootstrap_fork(&mut seed), &b_actions);

        let mut expected = readout(&a.snapshot());
        expected.extend(readout(&b.snapshot()));

        wire_gossip(&mut a, &mut b);

        prop_assert_eq!(readout(&a.snapshot()), expected.clone());
        prop_assert_eq!(readout(&b.snapshot()), expected);
        prop_assert_eq!(a.hash(), b.hash());
        prop_assert_eq!(a.latest(), b.latest());
    }

    /// String-T variant of [`async_gossip_converges_on_the_union`]: same
    /// invariant for `T = String`, exercising the borsh round-trip for a
    /// non-primitive value type over the concurrent wire.
    #[test]
    fn async_gossip_converges_on_the_union_string(
        a_actions in arb_string_actions(),
        b_actions in arb_string_actions(),
    ) {
        let mut seed = Known::<String>::seed();
        let mut a = build_local_async(bootstrap_fork(&mut seed), &a_actions);
        let mut b = build_local_async(bootstrap_fork(&mut seed), &b_actions);

        let mut expected = readout(&a.snapshot());
        expected.extend(readout(&b.snapshot()));

        wire_gossip(&mut a, &mut b);

        prop_assert_eq!(readout(&a.snapshot()), expected.clone());
        prop_assert_eq!(readout(&b.snapshot()), expected);
        prop_assert_eq!(a.hash(), b.hash());
        prop_assert_eq!(a.latest(), b.latest());
    }
}
