//! Shared observer schedules and read helpers for the causal and unordered suites.

use std::collections::BTreeMap;
use std::sync::Arc;

use futures::{FutureExt, Stream, StreamExt};
use proptest::{collection::vec, prelude::*};
use rumors::{Peer, Rumors, Version};

use super::wire::{bootstrap_fork, wire_gossip};

/// A message, a temporary lack of news, or the end of an observer.
#[derive(Debug, PartialEq)]
pub enum Step {
    /// The observer yielded a message.
    Item((Version, u64)),
    /// The observer is quiet: nothing new, actors still live.
    Quiet,
    /// The observer ended: every sender is gone and the complete final
    /// state has been yielded.
    Ended,
}

/// Poll the observer exactly once without an executor.
pub fn step(obs: &mut (impl Stream<Item = (Version, Arc<u64>)> + Unpin)) -> Step {
    match obs.next().now_or_never() {
        None => Step::Quiet,
        Some(None) => Step::Ended,
        Some(Some((v, m))) => Step::Item((v, *m)),
    }
}

/// Drain the observer until it goes quiet or ends, returning the items in
/// delivery order and whether it ended.
pub fn drain(
    obs: &mut (impl Stream<Item = (Version, Arc<u64>)> + Unpin),
) -> (Vec<(Version, u64)>, bool) {
    let mut items = Vec::new();
    loop {
        match step(obs) {
            Step::Item(item) => items.push(item),
            Step::Quiet => return (items, false),
            Step::Ended => return (items, true),
        }
    }
}

/// Live messages keyed by version bytes, independent of iteration order.
pub fn live_map(rumors: &Rumors<u64>) -> BTreeMap<Vec<u8>, u64> {
    rumors
        .snapshot()
        .iter()
        .map(|(v, m)| (v.as_bytes().to_vec(), *m))
        .collect()
}

/// Maximum actions in one generated observer schedule.
const MAX_OPS: usize = 40;

/// A change or read in a two-replica network, observed at `a`.
#[derive(Debug, Clone)]
pub enum Op {
    /// Send at the observed replica.
    SendA(u64),
    /// Send at the remote replica.
    SendB(u64),
    /// Redact a live message at either replica, including gossip-learned messages.
    Redact(bool, prop::sample::Index),
    /// Converge both replicas.
    Gossip,
    /// Read a short prefix, potentially leaving a pass in progress.
    Read(usize),
    /// Read until the observer is quiet.
    Drain,
}

/// Vary writes, redactions at both peers, gossip, and partial or complete reads.
pub fn arb_ops() -> impl Strategy<Value = Vec<Op>> {
    vec(
        prop_oneof![
            3 => any::<u64>().prop_map(Op::SendA),
            3 => any::<u64>().prop_map(Op::SendB),
            3 => (any::<bool>(), any::<prop::sample::Index>())
                .prop_map(|(remote, index)| Op::Redact(remote, index)),
            2 => Just(Op::Gossip),
            3 => (1usize..4).prop_map(Op::Read),
            2 => Just(Op::Drain),
        ],
        0..=MAX_OPS,
    )
}

/// Run the same network schedule through either public message observer.
///
/// Check termination, sent content, duplicate-free delivery, and coverage of
/// the final live set. Return delivery order for the causal suite's extra check.
pub fn interleave<S>(
    ops: &[Op],
    subscribe: impl FnOnce(&Rumors<u64>) -> S,
) -> Result<Vec<(Version, u64)>, TestCaseError>
where
    S: Stream<Item = (Version, Arc<u64>)> + Unpin,
{
    let a = Peer::<u64>::seed().sync_window_floor().into_rumors();
    let sibling = a.clone();
    let b = bootstrap_fork(&a);
    let mut obs = subscribe(&a);
    let mut published = BTreeMap::new();
    let mut delivered = Vec::new();

    for (i, op) in ops.iter().enumerate() {
        match op {
            Op::SendA(value) | Op::SendB(value) => {
                let sender = match op {
                    Op::SendB(_) => &b,
                    _ if i % 2 == 0 => &a,
                    _ => &sibling,
                };
                let version = sender.send(*value).unwrap();
                published.insert(version.as_bytes().to_vec(), *value);
            }
            Op::Redact(remote, index) => {
                let peer = if *remote { &b } else { &a };
                let snapshot = peer.snapshot();
                if !snapshot.is_empty() {
                    let (version, _) = snapshot.iter().nth(index.index(snapshot.len())).unwrap();
                    peer.redact(version);
                }
            }
            Op::Gossip => wire_gossip(&a, &b),
            Op::Read(count) => {
                for _ in 0..*count {
                    match step(&mut obs) {
                        Step::Item(item) => delivered.push(item),
                        Step::Quiet => break,
                        Step::Ended => prop_assert!(false, "live handles keep the observer open"),
                    }
                }
            }
            Op::Drain => {
                let (items, ended) = drain(&mut obs);
                prop_assert!(!ended, "live handles keep the observer open");
                delivered.extend(items);
            }
        }
    }

    // B remains alive but holds no handle to A's replica. Closing A must
    // drain its final snapshot, including any pass already in progress.
    let final_live = live_map(&a);
    drop(sibling);
    drop(a);
    let (items, ended) = drain(&mut obs);
    prop_assert!(ended);
    delivered.extend(items);

    let mut seen = BTreeMap::new();
    for (version, value) in &delivered {
        let key = version.as_bytes().to_vec();
        prop_assert_eq!(
            published.get(&key),
            Some(value),
            "delivery preserves sent content"
        );
        prop_assert!(
            seen.insert(key, *value).is_none(),
            "version {version:?} delivered twice"
        );
    }
    for (key, value) in &final_live {
        prop_assert_eq!(
            seen.get(key),
            Some(value),
            "a final live message was never delivered"
        );
    }
    Ok(delivered)
}

/// A pass retains its captured messages across local or gossiped redactions.
/// Messages redacted before a later pass starts must stay absent.
pub fn redact_during_pass<S>(
    values: Vec<u64>,
    taken: prop::sample::Index,
    redactions: Vec<prop::sample::Index>,
    remote: bool,
    subscribe: impl FnOnce(&Rumors<u64>) -> S,
) -> Result<(), TestCaseError>
where
    S: Stream<Item = (Version, Arc<u64>)> + Unpin,
{
    let a = Peer::<u64>::seed().sync_window_floor().into_rumors();
    let b = bootstrap_fork(&a);
    b.send_all(values).unwrap();
    wire_gossip(&a, &b);
    let expected = live_map(&a);
    let mut obs = subscribe(&a);
    let mut delivered = Vec::new();
    // Leave at least one captured message unconsumed. This is the state a
    // drain-only schedule cannot reach while redactions are being applied.
    for _ in 0..=taken.index(expected.len() - 1) {
        match step(&mut obs) {
            Step::Item(item) => delivered.push(item),
            other => prop_assert!(false, "the captured pass still has messages: {other:?}"),
        }
    }
    let pending: Vec<_> = a
        .snapshot()
        .iter()
        .map(|(version, _)| version.clone())
        .filter(|version| !delivered.iter().any(|(seen, _)| seen == version))
        .collect();
    let redactor = if remote { &b } else { &a };
    for index in &redactions {
        redactor.redact(&pending[index.index(pending.len())]);
    }
    wire_gossip(&a, &b);
    let live = live_map(&a);
    for index in &redactions {
        prop_assert!(!live.contains_key(pending[index.index(pending.len())].as_bytes()));
    }
    let (rest, ended) = drain(&mut obs);
    prop_assert!(!ended);
    delivered.extend(rest);
    prop_assert_eq!(delivered.len(), expected.len());
    let actual: BTreeMap<_, _> = delivered
        .into_iter()
        .map(|(version, value)| (version.as_bytes().to_vec(), value))
        .collect();
    prop_assert_eq!(
        actual,
        expected,
        "redactions do not invalidate a captured pass"
    );

    let version = a.send(0).unwrap();
    a.redact(&version);
    prop_assert_eq!(
        step(&mut obs),
        Step::Quiet,
        "redacted before capture: no delivery"
    );
    Ok(())
}
