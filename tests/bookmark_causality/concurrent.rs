//! Generated fleet operations run while join sessions hold reserved forks.

use futures::{FutureExt, future::LocalBoxFuture};
use tokio::io::{BufReader, DuplexStream};

use super::*;
use rumors_testkit::common::seed::network_seed;

/// A join paused after the provider reserves its donation.
struct Attempt {
    /// The newcomer owns its connection end until joining completes or is
    /// cancelled.
    joining: LocalBoxFuture<
        'static,
        rumors::Join<Msg, BufReader<DuplexStream>, DuplexStream, FlakyInMemoryBookmark>,
    >,
    /// The newcomer's durable store, selected with its bookmark when the join
    /// starts.
    store: DurableStore,
    /// The newcomer's bookmark fail schedule, shared with its bookmark.
    faults: Arc<Mutex<FaultFeed>>,
    /// The provider owns its connection end and the pending donation guard.
    serving: LocalBoxFuture<'static, Result<rumors::Gossiped, Error<FlakyInMemoryBookmark>>>,
    /// The reserved region, used only to check that live peers cannot reclaim
    /// it.
    reserved: Party,
}

/// Drive only the boundaries needed to interleave ordinary fleet operations.
impl Attempt {
    /// Exchange the preamble and stop before the newcomer continues
    /// reconciliation.
    ///
    /// The newcomer's bookmark is selected here, as the join starts, and
    /// labeled `label` for diagnostics.
    fn reserve(server: &Peer<Msg, FlakyInMemoryBookmark>, label: usize) -> Self {
        let before = server.dangerously_alias_party();
        let serving = server.clone();
        let store = DurableStore::default();
        let faults = Arc::new(Mutex::new(FaultFeed::new(Vec::new(), Vec::new())));
        let bookmark = FlakyInMemoryBookmark::new(store.clone(), faults.clone(), label);
        let (near, far) = rumors::testing::memory_connection(MEMORY_CAPACITY);
        let mut joining = async move {
            Peer::<Msg>::builder()
                .sync_memory_budget(0)
                .bookmark(bookmark)
                .join(near.read, near.write)
                .await
        }
        .boxed_local();
        let mut serving = async move {
            serving
                .gossip_once(far.read, far.write)
                .await
                .map(|(gossiped, ..)| gossiped)
        }
        .boxed_local();
        block_on(async {
            assert!(futures::poll!(joining.as_mut()).is_pending());
            assert!(futures::poll!(serving.as_mut()).is_pending());
        });
        let after = server.dangerously_alias_party();
        assert_ne!(before, after, "the join must actually reserve a fork");
        let reserved = before.without(&after).expect("a nonempty donation");
        Self {
            joining,
            store,
            faults,
            serving,
            reserved,
        }
    }

    /// Complete the session and register any joined peer for subsequent checks.
    fn finish(self, world: &mut World, server: usize) {
        let may_fail = world.bookmark_may_fail(server);
        let (joined, served) = block_on(async { futures::join!(self.joining, self.serving) });
        if let Err(error) = served {
            assert!(may_fail, "a reliable pending join must complete: {error:?}");
            assert_not_codec_bug("interleaved join provider", &error);
        }
        match joined {
            rumors::Join::Connected {
                peer,
                connection: rumors::Connection::Reusable { .. },
            } => {
                let label = world.n();
                world.nodes.push(Node {
                    network: peer.network(),
                    state: NodeState::Live(Box::new(peer)),
                    store: self.store,
                    faults: self.faults,
                    label,
                    pending: Vec::new(),
                    pending_redactions: Vec::new(),
                });
                world.secure(label);
            }
            rumors::Join::Failed { error, .. } => {
                assert!(may_fail, "a reliable newcomer must join: {error:?}");
                assert_not_codec_bug("interleaved join newcomer", &error);
            }
            other => panic!("unexpected join outcome: {other:?}"),
        }
        world.secure(server);
        world.assert_ownership();
    }
}

/// Apply a generated operation, ending drivers before an exclusive lifecycle
/// change.
fn apply(world: &mut World, pending: &mut Vec<Attempt>, step: Step) {
    match step {
        Step::Send(i) => world.send(i),
        Step::Redact(i, k) => world.redact(i, k),
        Step::Gossip(i, j, a, b) => world.gossip(i, j, a, b),
        Step::Crash(i) => {
            if i == 2 {
                pending.clear();
            }
            if world.nodes[i].is_live() && world.other_live_in_network(i) {
                world.crash(i);
            }
        }
        Step::Retire(i, j) => {
            // Retirement requires exclusive ownership: stop the provider's
            // drivers first. Other peers may retire into it while forks wait.
            if i == 2 {
                pending.clear();
            }
            world.retire(i, j);
        }
    }
    world.assert_ownership();
    for attempt in pending {
        for node in &world.nodes {
            if let Some(peer) = node.live() {
                assert!(
                    attempt
                        .reserved
                        .is_disjoint(&peer.dangerously_alias_party()),
                    "a live peer reclaimed a fork still held by a join"
                );
            }
        }
    }
}

proptest! {
    /// Concurrent checkpoints, absorption, storage failures, and cancellation
    /// cannot make reserved forks reusable or let a restart outrun durable writes.
    #[test]
    fn interleaved_joins_preserve_recovery(
        finish in proptest::collection::vec(any::<bool>(), 1..4),
        steps in proptest::collection::vec(arb_step(4, true), 0..18),
        write_faults in arb_fault_bits(true),
        network in network_seed(),
    ) {
        let mut world = World::single_network(4, network);
        world.gossip(2, 0, FaultPlan::NONE, FaultPlan::NONE);
        let first_label = world.n();
        let mut pending: Vec<_> = (0..finish.len())
            .map(|attempt| Attempt::reserve(world.nodes[2].live().unwrap(), first_label + attempt))
            .collect();
        // Keep node 0 behind while node 1 witnesses a write from the remaining
        // identity. This prefix guarantees the schedule exercises a checkpoint
        // with overlapping claims, even when the generated suffix is empty.
        world.send(2);
        world.gossip(2, 1, FaultPlan::NONE, FaultPlan::NONE);
        *world.nodes[2].faults.lock().unwrap() = FaultFeed::new(Vec::new(), write_faults);
        for step in steps {
            apply(&mut world, &mut pending, step);
        }
        for (attempt, finish) in pending.into_iter().zip(finish) {
            if finish { attempt.finish(&mut world, 2); } else { drop(attempt); }
            world.assert_ownership();
        }

        // Force recovery before the final heal can hide an unsafe intermediate
        // identity by supplying its missing history. Faults end for this phase.
        for node in &world.nodes { node.faults.lock().unwrap().disable(); }
        world.revive(0);
        world.crash(2);
        assert!(world.join_into(2, 0));
        world.gossip(2, 0, FaultPlan::NONE, FaultPlan::NONE);
        world.assert_ownership();
        world.send(2);
        world.heal();
        world.assert_ownership();
        world.assert_healed();
        world.settle();
        world.assert_reclaimed();
    }
}
