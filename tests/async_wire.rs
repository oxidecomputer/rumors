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
use crate::common::wire::{block_on, wire_gossip};

/// Bidirectional in-process merge via `Known::join`: the local oracle the
/// wire path must reproduce. Operands must be disjoint (forked from a shared
/// seed), which every caller guarantees, so the joins never fail.
fn local_merge<T>(a: &mut Known<T>, b: &mut Known<T>)
where
    T: Send + Sync,
{
    let a_snapshot = a.fork();
    let b_snapshot = b.fork();
    a.join(b_snapshot)
        .unwrap_or_else(|_| unreachable!("disjoint operands"));
    b.join(a_snapshot)
        .unwrap_or_else(|_| unreachable!("disjoint operands"));
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
        let mut seed = Known::<u64>::seed();
        let mut a0 = block_on(build_local_async(seed.fork(), &a_actions));
        let mut b0 = block_on(build_local_async(seed.fork(), &b_actions));

        let mut a_proc = a0.fork();
        let mut b_proc = b0.fork();
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
        let mut seed = Known::<String>::seed();
        let mut a0 = block_on(build_local_async(seed.fork(), &a_actions));
        let mut b0 = block_on(build_local_async(seed.fork(), &b_actions));

        let mut a_proc = a0.fork();
        let mut b_proc = b0.fork();
        local_merge(&mut a_proc, &mut b_proc);

        let (a_wire, b_wire) = wire_gossip(a0, b0);

        prop_assert_eq!(&a_proc, &a_wire);
        prop_assert_eq!(&b_proc, &b_wire);
        prop_assert_eq!(&a_wire, &b_wire);
    }
}
