use borsh::{BorshDeserialize, BorshSerialize};
use proptest::collection::vec;
use proptest::prelude::*;

use crate::tree::arb::arb_root_tree;
use crate::tree::traverse::{Action, act};
use crate::tree::typed::height::Root;
use crate::tree::typed::{Node, Path};
use crate::{Key, Message, Version};

use super::{initiator, local, remote, responder};

/// Which mirror-protocol arrangement to drive: the cardinal product of
/// `{local, remote}` for the initiator side and the responder side.
///
/// In every variant, "A" is the initiator (holds tree `a`) and "B" is the
/// responder (holds tree `b`). What varies is which side's state the *test
/// thread* holds directly (`Local`) versus accesses via a wire proxy
/// (`Remote`). The variants exercise all four `(local::Exchange,
/// remote::Exchange)` pairings of arguments to `initiator` /
/// `responder` so the `Peer<P, T>` bound is checked under every concrete
/// type combination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scenario {
    /// Single-threaded, all in-memory: `initiator(local_a, local_b)`.
    LocalLocal,
    /// Test thread holds A locally; B runs on a peer thread reachable
    /// over a duplex pipe. Test thread calls `initiator(local_a,
    /// remote_b)`; peer thread calls `responder(local_b, remote_a)`.
    LocalRemote,
    /// Symmetric to [`LocalRemote`]: test thread holds B locally; A runs
    /// on a peer thread. Test thread calls `initiator(remote_a, local_b)`
    /// (driving the initiator role through a wire proxy); peer thread
    /// calls `initiator(local_a, remote_b)`.
    RemoteLocal,
    /// Both sides' state live on peer threads; test thread is a pure
    /// relay over two duplex pipes. Test thread calls
    /// `initiator(remote_a, remote_b)`; one peer thread runs
    /// `initiator(local_a, remote_relay)`, the other
    /// `responder(local_b, remote_relay)`.
    RemoteRemote,
}

/// Two `(reader, writer)` halves of a full-duplex byte channel: the
/// first half reads what the second half writes, and vice versa. Used to
/// stand up wire pairings in the scenarios that involve a peer thread.
fn duplex() -> (
    (std::io::PipeReader, std::io::PipeWriter),
    (std::io::PipeReader, std::io::PipeWriter),
) {
    let (a_to_b_r, a_to_b_w) = std::io::pipe().expect("pipe");
    let (b_to_a_r, b_to_a_w) = std::io::pipe().expect("pipe");
    ((b_to_a_r, a_to_b_w), (a_to_b_r, b_to_a_w))
}

/// Drive the mirror protocol through the high-level [`super::initiator`]
/// / [`super::responder`] drivers under the chosen [`Scenario`], and
/// return the reconciled tree (which must be equal on both sides if the
/// protocol converged).
fn mirror_via<P, T>(
    a: Option<Node<P, T, Root>>,
    b: Option<Node<P, T, Root>>,
    scenario: Scenario,
) -> Option<Node<P, T, Root>>
where
    P: Clone
        + Ord
        + AsRef<[u8]>
        + std::fmt::Debug
        + BorshSerialize
        + BorshDeserialize
        + Send
        + Sync
        + 'static,
    T: PartialEq + std::fmt::Debug + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    fn x<P: Ord, T>(_v: &Version<P>, _k: Key, _m: &Message<T>) {}

    let version_a = a.as_ref().map(|n| n.version().clone()).unwrap_or_default();
    let version_b = b.as_ref().map(|n| n.version().clone()).unwrap_or_default();

    match scenario {
        Scenario::LocalLocal => {
            let local_a = local::Exchange::start(a, &version_b, x, x);
            let local_b = local::Exchange::start(b, &version_a, x, x);
            initiator(local_a, local_b).expect("local-local mirror")
        }

        Scenario::LocalRemote => {
            let ((a_r, a_w), (b_r, b_w)) = duplex();
            // Move owned versions into each closure so neither side
            // borrows across thread boundaries.
            let version_a_for_peer = version_a.clone();
            std::thread::scope(|s| {
                let peer = s.spawn(move || {
                    let local_b = local::Exchange::start(b, &version_a_for_peer, x, x);
                    let remote_a = remote::Exchange::start(b_r, b_w);
                    responder(local_b, remote_a).expect("peer responder")
                });
                let local_a = local::Exchange::start(a, &version_b, x, x);
                let remote_b = remote::Exchange::start(a_r, a_w);
                let out = initiator(local_a, remote_b).expect("test initiator");
                let peer_out = peer.join().expect("peer thread joined");
                assert_eq!(out, peer_out, "local-remote endpoints should converge");
                out
            })
        }

        Scenario::RemoteLocal => {
            let ((a_r, a_w), (b_r, b_w)) = duplex();
            let version_b_for_peer = version_b.clone();
            std::thread::scope(|s| {
                let peer = s.spawn(move || {
                    let local_a = local::Exchange::start(a, &version_b_for_peer, x, x);
                    let remote_b = remote::Exchange::start(a_r, a_w);
                    initiator(local_a, remote_b).expect("peer initiator")
                });
                let local_b = local::Exchange::start(b, &version_a, x, x);
                let remote_a = remote::Exchange::start(b_r, b_w);
                // Test thread plays the responder role on `local_b`;
                // the initiator-side is the wire proxy `remote_a`. The
                // matching driver here is `responder`, whose first arg
                // is the responder side.
                responder(local_b, remote_a).expect("test responder");
                peer.join().expect("peer thread joined")
            })
        }

        Scenario::RemoteRemote => {
            // Three threads: peer A holds tree `a` and drives the
            // initiator role; peer B holds tree `b` and drives the
            // responder role; the test thread is a pure relay between
            // the two via two duplex pipes.
            let ((a_relay_r, a_relay_w), (a_r, a_w)) = duplex();
            let ((b_relay_r, b_relay_w), (b_r, b_w)) = duplex();
            let version_a_for_b = version_a.clone();
            let version_b_for_a = version_b.clone();
            std::thread::scope(|s| {
                let peer_a = s.spawn(move || {
                    let local_a = local::Exchange::start(a, &version_b_for_a, x, x);
                    let remote_relay = remote::Exchange::start(a_r, a_w);
                    initiator(local_a, remote_relay).expect("peer A initiator")
                });
                let peer_b = s.spawn(move || {
                    let local_b = local::Exchange::start(b, &version_a_for_b, x, x);
                    let remote_relay = remote::Exchange::start(b_r, b_w);
                    responder(local_b, remote_relay).expect("peer B responder")
                });
                let remote_a = remote::Exchange::<P, T, _, _, Root>::start(a_relay_r, a_relay_w);
                let remote_b = remote::Exchange::<P, T, _, _, Root>::start(b_relay_r, b_relay_w);
                // Test thread relays: it acts as the responder toward
                // peer A (reading initiator messages off `remote_a`,
                // forwarding through `remote_b`) and as the initiator
                // toward peer B. `initiator(remote_a, remote_b)` is
                // exactly that.
                initiator(remote_a, remote_b).expect("relay");
                let out_a = peer_a.join().expect("peer A joined");
                let out_b = peer_b.join().expect("peer B joined");
                assert_eq!(out_a, out_b, "relay endpoints should converge");
                out_a
            })
        }
    }
}

proptest! {

    /// Mirroring a node with itself is a no-op: the two replicas have
    /// identical content and versions, so the reconciled tree is unchanged.
    #[test]
    fn idempotent(a in arb_root_tree("a", 0..=8)) {
        for scenario in [
            Scenario::LocalLocal,
            Scenario::LocalRemote,
            Scenario::RemoteLocal,
            Scenario::RemoteRemote,
        ] {
            prop_assert_eq!(mirror_via(a.clone(), a.clone(), scenario), a.clone());
        }
    }

    /// The reconciled tree is the same regardless of which replica
    /// initiates and which responds.
    #[test]
    fn commutative(
        a in arb_root_tree("a", 0..=8),
        b in arb_root_tree("b", 0..=8),
    ) {
        for scenario in [
            Scenario::LocalLocal,
            Scenario::LocalRemote,
            Scenario::RemoteLocal,
            Scenario::RemoteRemote,
        ] {
            prop_assert_eq!(
                mirror_via(a.clone(), b.clone(), scenario),
                mirror_via(b.clone(), a.clone(), scenario),
            );
        }
    }

    /// Re-mirroring the result with a peer already synced with is a no-op:
    /// the result already contains everything the peer had.
    #[test]
    fn absorptive(
        a in arb_root_tree("a", 0..=8),
        b in arb_root_tree("b", 0..=8),
    ) {
        for scenario in [
            Scenario::LocalLocal,
            Scenario::LocalRemote,
            Scenario::RemoteLocal,
            Scenario::RemoteRemote,
        ] {
            let ab = mirror_via(a.clone(), b.clone(), scenario);
            prop_assert_eq!(mirror_via(ab.clone(), b.clone(), scenario), ab);
        }
    }

    /// Three-way mirror is order-independent: syncing (a,b) then c
    /// produces the same tree as syncing a then (b,c).
    #[test]
    fn associative(
        a in arb_root_tree("a", 0..=4),
        b in arb_root_tree("b", 0..=4),
        c in arb_root_tree("c", 0..=4),
    ) {
        for scenario in [
            Scenario::LocalLocal,
            Scenario::LocalRemote,
            Scenario::RemoteLocal,
            Scenario::RemoteRemote,
        ] {
            let ab_c = mirror_via(
                mirror_via(a.clone(), b.clone(), scenario),
                c.clone(),
                scenario,
            );
            let a_bc = mirror_via(
                a.clone(),
                mirror_via(b.clone(), c.clone(), scenario),
                scenario,
            );
            prop_assert_eq!(ab_c, a_bc);
        }
    }

    /// Mirror is equivalent to replaying both sides' full action
    /// histories — inserts and forgets — through a single `act` call.
    #[test]
    fn equivalent_to_cross_react(
        entries_a in vec((any::<[u8; 32]>(), any::<bool>()), 0..=8),
        entries_b in vec((any::<[u8; 32]>(), any::<bool>()), 0..=8),
    ) {
        let make_actions = |party: &str, entries: &[([u8; 32], bool)]| -> Vec<_> {
            let n = entries.len();
            let mut actions: Vec<_> = entries.iter().enumerate().map(|(i, (bytes, _))| {
                let path = Path::from(*bytes);
                let version = Version::from((party.to_string(), i as u64 + 1));
                (path, version, Action::Insert(Message::new(())))
            }).collect();
            for (j, (bytes, forget)) in entries.iter().enumerate() {
                if *forget {
                    let path = Path::from(*bytes);
                    let version = Version::from((party.to_string(), (n + j + 1) as u64));
                    actions.push((path, version, Action::Forget));
                }
            }
            actions
        };

        let actions_a = make_actions("a", &entries_a);
        let actions_b = make_actions("b", &entries_b);

        let node_a = act(None, actions_a.clone());
        let node_b = act(None, actions_b.clone());

        let mut all_actions = actions_a;
        all_actions.extend(actions_b);
        let expected = act(None, all_actions);

        for scenario in [
            Scenario::LocalLocal,
            Scenario::LocalRemote,
            Scenario::RemoteLocal,
            Scenario::RemoteRemote,
        ] {
            let mirrored = mirror_via(node_a.clone(), node_b.clone(), scenario);
            prop_assert_eq!(mirrored, expected.clone());
        }
    }
}
