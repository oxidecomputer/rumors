//! Wire-equivalence test for the *asynchronous* gossip path:
//! `rumors::Known::gossip` driven concurrently with `tokio::join!` over a
//! `tokio::io::duplex` pipe must agree with the in-process bidirectional
//! merge (`Known::join`). Mirrors `sync_wire.rs`, which exercises the
//! synchronous `sync::Known::gossip` path over `std::io::pipe`s.
//!
//! Both tests share the `Insert`/`Redact` action shape, so redactions cross
//! the wire too (not just inserts), and run against both a primitive (`u64`)
//! and a non-primitive (`String`) value type to cover the borsh round-trip.

mod common;

use proptest::prelude::*;
use rumors::Known;

use crate::common::action::{arb_local_actions, arb_string_actions, build_local_async};
use crate::common::wire::{block_on, bootstrap_fork, wire_gossip};

/// Bidirectional in-process merge via `Known::join`: the local oracle the wire
/// path must reproduce. `join` merges content network-guarded, so any two peers
/// in the same universe merge; every caller passes same-universe peers, so the
/// joins never fail.
fn local_merge<T>(a: &mut Known<T>, b: &mut Known<T>)
where
    T: Send + Sync,
{
    let a_snapshot = a.rumors();
    let b_snapshot = b.rumors();
    a.join(b_snapshot)
        .unwrap_or_else(|_| unreachable!("same-universe operands"));
    b.join(a_snapshot)
        .unwrap_or_else(|_| unreachable!("same-universe operands"));
}

proptest! {
    /// Bidirectional `Known::join` produces the same final live content as
    /// driving the same two async `Known`s through `Known::gossip` over a
    /// `tokio::io::duplex` pipe — proving the concurrent wire protocol is
    /// faithful to the in-process merge. `Known::eq` compares live content
    /// (the tree) independent of party, so a direct equality is meaningful.
    #[test]
    fn async_gossip_matches_local_merge(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let seed = Known::<u64>::seed();
        let a0 = block_on(build_local_async(bootstrap_fork(&seed), &a_actions));
        let b0 = block_on(build_local_async(bootstrap_fork(&seed), &b_actions));

        // Genuine party-disjoint copies of each side's content for the oracle;
        // a0/b0 themselves go on to the wire.
        let mut a_proc = bootstrap_fork(&a0);
        let mut b_proc = bootstrap_fork(&b0);
        local_merge(&mut a_proc, &mut b_proc);

        let (a_wire, b_wire) = wire_gossip(a0, b0);

        prop_assert_eq!(&a_proc, &a_wire);
        prop_assert_eq!(&b_proc, &b_wire);
        prop_assert_eq!(&a_wire, &b_wire);
    }

    /// String-T variant of [`async_gossip_matches_local_merge`]: same
    /// invariant for `T = String`, exercising the borsh round-trip for a
    /// non-primitive value type over the concurrent wire.
    #[test]
    fn async_gossip_matches_local_merge_string(
        a_actions in arb_string_actions(),
        b_actions in arb_string_actions(),
    ) {
        let seed = Known::<String>::seed();
        let a0 = block_on(build_local_async(bootstrap_fork(&seed), &a_actions));
        let b0 = block_on(build_local_async(bootstrap_fork(&seed), &b_actions));

        // Genuine party-disjoint copies of each side's content for the oracle;
        // a0/b0 themselves go on to the wire.
        let mut a_proc = bootstrap_fork(&a0);
        let mut b_proc = bootstrap_fork(&b0);
        local_merge(&mut a_proc, &mut b_proc);

        let (a_wire, b_wire) = wire_gossip(a0, b0);

        prop_assert_eq!(&a_proc, &a_wire);
        prop_assert_eq!(&b_proc, &b_wire);
        prop_assert_eq!(&a_wire, &b_wire);
    }
}
