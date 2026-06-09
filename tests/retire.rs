//! Integration tests for `rumors::Known::retire` (and its synchronous twin
//! `sync::Known::retire`): a peer hands its ITC party back to a peer that
//! causally dominates it, so its id-region is reclaimed rather than leaked.
//!
//! Retirement never moves content — it rides entirely on the connect-phase
//! greeting — so the assertions here are about *outcomes* (`Ok(None)` retired,
//! `Ok(Some(self))` declined) and the invariants the absorbing peer must
//! preserve (its tree and version are untouched). Party integrity on the
//! decline path is shown behaviorally: the declined retiree remains a live
//! `Known` whose content still `join`s (a network-guarded content merge) into
//! the peer it tried to retire into.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use proptest::prelude::*;
use rumors::{Key, Known, Version};

use crate::common::action::{LocalAction, arb_local_actions, build_local, build_local_async};
use crate::common::sync_wire::{sync_bootstrap_fork, sync_wire_gossip};
use crate::common::wire::{block_on, bootstrap_fork, wire_gossip};

/// Capacity for the in-memory duplex pipe. Retire moves no content, so a small
/// buffer would do; this matches the other wire tests' headroom.
const DUPLEX_BUF: usize = 64 * 1024;

// ---- builders ------------------------------------------------------------

/// Build an async `Known<u64>` by inserting `vals` into a disjoint originator
/// (a genuine bootstrap fork: its own party region, ready to originate).
fn async_known(peer: Known<u64>, vals: &[u64]) -> Known<u64> {
    let actions: Vec<LocalAction<u64>> = vals.iter().map(|&v| LocalAction::Insert(v)).collect();
    block_on(build_local_async(peer, &actions))
}

/// Build a synchronous `sync::Known<u64>` by inserting `vals` into a disjoint
/// originator (a genuine bootstrap fork: its own party region).
fn sync_known(peer: rumors::sync::Known<u64>, vals: &[u64]) -> rumors::sync::Known<u64> {
    let actions: Vec<LocalAction<u64>> = vals.iter().map(|&v| LocalAction::Insert(v)).collect();
    build_local(peer, &actions)
}

// ---- wire harnesses ------------------------------------------------------

/// Drive `retiree.retire` against `peer.gossip` concurrently over a duplex
/// pipe, returning the retiree's outcome and the (possibly absorbing) peer.
fn retire_into_gossip(retiree: Known<u64>, peer: Known<u64>) -> (Option<Known<u64>>, Known<u64>) {
    block_on(async move {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        let (retire_out, gossip_out) = tokio::join!(
            retiree.retire(&mut a_r, &mut a_w),
            peer.gossip(&mut b_r, &mut b_w),
        );
        (
            retire_out.expect("retiree"),
            gossip_out.expect("gossiping peer"),
        )
    })
}

/// Adapt "count this message" into the higher-ranked async callback shape
/// `gossip_then` expects (the explicit return type pins the HRTB lifetime, the
/// same trick `common::action::record_key` uses).
fn count_into(
    sink: Arc<AtomicUsize>,
) -> impl FnMut(Key, &Version, &Arc<u64>) -> std::future::Ready<()> {
    move |_k: Key, _v: &Version, _m: &Arc<u64>| {
        sink.fetch_add(1, Ordering::Relaxed);
        std::future::ready(())
    }
}

/// Like [`retire_into_gossip`], but the gossiping peer counts the messages it
/// is asked to deliver. Returns that count alongside the two outcomes.
fn retire_into_counting_gossip(
    retiree: Known<u64>,
    peer: Known<u64>,
) -> (Option<Known<u64>>, Known<u64>, usize) {
    let count = Arc::new(AtomicUsize::new(0));
    let sink = Arc::clone(&count);
    let (retire_out, gossip_out) = block_on(async move {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        tokio::join!(
            retiree.retire(&mut a_r, &mut a_w),
            peer.gossip_then(&mut b_r, &mut b_w, count_into(sink)),
        )
    });
    (
        retire_out.expect("retiree"),
        gossip_out.expect("gossiping peer"),
        count.load(Ordering::Relaxed),
    )
}

/// Drive `a.retire` against `b.retire` concurrently: a mutual retirement.
fn retire_into_retire(a: Known<u64>, b: Known<u64>) -> (Option<Known<u64>>, Option<Known<u64>>) {
    block_on(async move {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        let (a_out, b_out) =
            tokio::join!(a.retire(&mut a_r, &mut a_w), b.retire(&mut b_r, &mut b_w));
        (a_out.expect("retiree A"), b_out.expect("retiree B"))
    })
}

/// Drive `retiree.retire` against a fresh `bootstrap`: neither can serve the
/// other. Returns the retiree's outcome and the bootstrapper's.
fn retire_into_bootstrap(retiree: Known<u64>) -> (Option<Known<u64>>, Option<Known<u64>>) {
    block_on(async move {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        let (retire_out, boot_out) = tokio::join!(
            retiree.retire(&mut a_r, &mut a_w),
            Known::<u64>::bootstrap(&mut b_r, &mut b_w),
        );
        (
            retire_out.expect("retiree"),
            boot_out.expect("bootstrapper"),
        )
    })
}

/// Synchronous counterpart of [`retire_into_gossip`]: one peer per OS thread,
/// connected by a pair of `std::io::pipe`s, exactly as `sync_wire_gossip` does.
fn sync_retire_into_gossip(
    retiree: rumors::sync::Known<u64>,
    peer: rumors::sync::Known<u64>,
) -> (Option<rumors::sync::Known<u64>>, rumors::sync::Known<u64>) {
    let (mut a_to_b_r, mut a_to_b_w) = std::io::pipe().expect("pipe a→b");
    let (mut b_to_a_r, mut b_to_a_w) = std::io::pipe().expect("pipe b→a");

    let peer_thread = std::thread::spawn(move || {
        peer.gossip(&mut a_to_b_r, &mut b_to_a_w)
            .expect("sync gossiping peer")
    });
    let retire_out = retiree
        .retire(&mut b_to_a_r, &mut a_to_b_w)
        .expect("sync retiree");
    let peer_out = peer_thread.join().expect("join peer thread");
    (retire_out, peer_out)
}

// ---- async behavioral tests ---------------------------------------------

/// Retiring into a peer that has gossiped to convergence (equal versions, so it
/// reflexively dominates) succeeds: the retiree drops itself (`Ok(None)`) and
/// the absorbing peer's tree and version are untouched (no content crosses).
#[test]
fn retire_into_converged_peer_succeeds() {
    let seed = Known::<u64>::seed();
    let a = async_known(bootstrap_fork(&seed), &[1, 2]);
    let b = async_known(seed, &[3, 4]);

    let (a, b) = wire_gossip(a, b);
    let hash = b.hash();
    let version = b.latest().clone();

    let (retired, b) = retire_into_gossip(a, b);
    assert!(retired.is_none(), "a dominating peer absorbs the retiree");
    assert_eq!(b.hash(), hash, "absorbing a retiree moves no content");
    assert_eq!(
        b.latest(),
        &version,
        "absorbing a retiree is a version no-op"
    );
}

/// Equal versions satisfy the `<=` domination precondition reflexively: a
/// fresh, empty bootstrap fork can retire into the peer it forked from with
/// no prior gossip.
#[test]
fn empty_equal_version_retire_succeeds() {
    let seed = Known::<u64>::seed();
    let a = bootstrap_fork(&seed);
    let b = seed;

    let (retired, _b) = retire_into_gossip(a, b);
    assert!(
        retired.is_none(),
        "equal versions dominate reflexively, so retire commits"
    );
}

/// A retiree whose peer does *not* dominate it (the two diverged concurrently)
/// is declined and handed back intact. Both parties survive live and disjoint,
/// proven by a subsequent `join` succeeding.
#[test]
fn divergent_peers_decline() {
    let seed = Known::<u64>::seed();
    let a = async_known(bootstrap_fork(&seed), &[1]);
    let b = async_known(seed, &[2]);

    let (retired, b) = retire_into_gossip(a, b);
    let mut a = retired.expect("a non-dominating peer declines the retiree, returning self");
    assert!(
        a.join(b.rumors()).is_ok(),
        "a declined retire leaves both parties live and disjoint"
    );
}

/// Two peers that both try to retire into each other both decline: each sees
/// the other's retire-intent in the greeting and refuses to absorb a peer that
/// is itself leaving. Both are handed back intact.
#[test]
fn mutual_retire_declines() {
    let seed = Known::<u64>::seed();
    let a = async_known(bootstrap_fork(&seed), &[1, 2]);
    let b = async_known(seed, &[3, 4]);

    let (ra, rb) = retire_into_retire(a, b);
    let mut a = ra.expect("mutual retire declines A");
    let b = rb.expect("mutual retire declines B");
    assert!(
        a.join(b.rumors()).is_ok(),
        "a mutually-declined retire leaves both parties live and disjoint"
    );
}

/// A retiree gets nothing from a peer that is itself bootstrapping (it has no
/// state to give and cannot dominate), and the bootstrapper gets nothing from a
/// peer that is retiring (it will not serve). Both bail after the greeting with
/// no deadlock.
#[test]
fn retire_into_bootstrapper_declines() {
    let seed = Known::<u64>::seed();
    let retiree = async_known(bootstrap_fork(&seed), &[1, 2]);

    let (retired, bootstrapped) = retire_into_bootstrap(retiree);
    assert!(
        retired.is_some(),
        "a retiree declines against a peer that cannot serve"
    );
    assert!(
        bootstrapped.is_none(),
        "a bootstrapper gets nothing from a retiring peer"
    );
}

/// Ordinary `gossip` transparently absorbs a retiree: the gossiping peer ends
/// with `Ok(self)`, delivers *zero* messages (no content moves when it already
/// dominates), and its tree and version are unchanged.
#[test]
fn gossip_absorbs_retiree_without_callbacks() {
    let seed = Known::<u64>::seed();
    let a = async_known(bootstrap_fork(&seed), &[1, 2]);
    let b = async_known(seed, &[3, 4]);

    let (a, b) = wire_gossip(a, b);
    let hash = b.hash();
    let version = b.latest().clone();

    let (retired, b, callbacks) = retire_into_counting_gossip(a, b);
    assert!(retired.is_none(), "the gossiping peer absorbs the retiree");
    assert_eq!(callbacks, 0, "absorbing a retiree delivers no messages");
    assert_eq!(b.hash(), hash, "absorbing a retiree moves no content");
    assert_eq!(
        b.latest(),
        &version,
        "absorbing a retiree is a version no-op"
    );
}

// ---- synchronous parity tests -------------------------------------------

/// The synchronous `retire` surface behaves like the async one: retiring into a
/// converged (dominating) peer succeeds and leaves that peer's tree and version
/// untouched.
#[test]
fn sync_retire_into_converged_peer_succeeds() {
    let seed = rumors::sync::Known::<u64>::seed();
    let a = sync_known(sync_bootstrap_fork(&seed), &[1, 2]);
    let b = sync_known(seed, &[3]);

    let (a, b) = sync_wire_gossip(a, b);
    let hash = b.hash();
    let version = b.latest().clone();

    let (retired, b) = sync_retire_into_gossip(a, b);
    assert!(retired.is_none(), "a dominating peer absorbs the retiree");
    assert_eq!(b.hash(), hash, "absorbing a retiree moves no content");
    assert_eq!(
        b.latest(),
        &version,
        "absorbing a retiree is a version no-op"
    );
}

/// Synchronous decline parity: a non-dominating peer declines the retiree over
/// the blocking wire, and both parties survive live and disjoint.
#[test]
fn sync_divergent_peers_decline() {
    let seed = rumors::sync::Known::<u64>::seed();
    let a = sync_known(sync_bootstrap_fork(&seed), &[1]);
    let b = sync_known(seed, &[2]);

    let (retired, b) = sync_retire_into_gossip(a, b);
    let mut a = retired.expect("a non-dominating peer declines the retiree");
    assert!(
        a.join(b.rumors()).is_ok(),
        "a declined retire leaves both parties live and disjoint"
    );
}

// ---- wire-equivalence property test -------------------------------------

proptest! {
    /// Retiring A into B over the wire (after gossiping to convergence) leaves B
    /// with the same live content (`hash`) and causal version (`latest`) as
    /// merging A into B with an in-process `join` — the local oracle for
    /// reunion. Two independently-seeded universes built from identical action
    /// sequences are compared; `hash`/`latest` are network-independent, so the
    /// distinct `Network` ids do not perturb the comparison.
    #[test]
    fn retire_matches_local_join(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        // Wire path: converge, then retire A into B.
        let (wire_hash, wire_version) = {
            let seed = Known::<u64>::seed();
            let a = block_on(build_local_async(bootstrap_fork(&seed), &a_actions));
            let b = block_on(build_local_async(seed, &b_actions));
            let (a, b) = wire_gossip(a, b);
            let (retired, b) = retire_into_gossip(a, b);
            prop_assert!(retired.is_none(), "a converged peer dominates, so retire commits");
            (b.hash(), b.latest().clone())
        };

        // Oracle: an in-process content-merge (`join`) of an identically-built
        // universe; `join` is network-guarded and merges content only.
        let (join_hash, join_version) = {
            let seed = Known::<u64>::seed();
            let a = block_on(build_local_async(bootstrap_fork(&seed), &a_actions));
            let mut b = block_on(build_local_async(seed, &b_actions));
            b.join(a.rumors()).expect("same-universe peers join");
            (b.hash(), b.latest().clone())
        };

        prop_assert_eq!(
            wire_hash, join_hash,
            "retire-over-wire leaves the same live content as a local join"
        );
        prop_assert_eq!(
            wire_version, join_version,
            "retire-over-wire leaves the same causal version as a local join"
        );
    }
}
