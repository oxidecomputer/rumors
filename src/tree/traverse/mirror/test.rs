use std::cell::OnceCell;
use std::future::Future;

use borsh::{BorshDeserialize, BorshSerialize};
use proptest::collection::vec;
use proptest::prelude::*;
use tokio::runtime::Runtime;

use crate::tree::arb::arb_tree_root;
use crate::tree::traverse::{Action, act};
use crate::tree::typed::Path;
use crate::{message::Message, tree::key::Key, version::Version};

use super::{local, mirror, remote};

thread_local! {
    /// One current-thread tokio runtime per test thread, initialized lazily on
    /// first use. Cargo's test harness gives each test its own thread, and
    /// proptest cases within a test run sequentially on that thread, so a
    /// single runtime per thread serves every case without contention.
    static RT: OnceCell<Runtime> = const { OnceCell::new() };
}

/// Drive an async future to completion on the per-thread runtime. Used to
/// bridge proptest's synchronous body with the now-async `mirror` driver.
fn block_on<F: Future>(fut: F) -> F::Output {
    RT.with(|cell| {
        cell.get_or_init(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build tokio current-thread runtime")
        })
        .block_on(fut)
    })
}

/// Which mirror-protocol arrangement to drive: the cardinal product of
/// `{local, remote}` for the initiator side and the responder side.
///
/// In every variant, "A" is the client (holds tree `a`) and "B" is the
/// server (holds tree `b`). What varies is which side's state the *test
/// thread* holds directly (`Local`) versus accesses via a wire proxy
/// (`Remote`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scenario {
    /// Single-threaded, all in-memory: `initiator(local_a, local_b)`.
    LocalLocal,
    /// Test thread holds A locally; B runs on a peer thread reachable
    /// over a duplex pipe.
    LocalRemote,
}

/// In-memory full-duplex byte channel capacity, in bytes. The mirror
/// protocol strictly alternates messages within a session, so the receiver
/// is always reading by the time the sender writes; a small buffer is
/// sufficient and exercises backpressure naturally.
const DUPLEX_BUF: usize = 8 * 1024;

/// Drive the mirror protocol through the high-level [`super::mirror`]
/// driver under the chosen [`Scenario`], and return the reconciled tree
/// (which must be equal on both sides if the protocol converged).
fn mirror_via<P, T>(
    a: crate::tree::Root<P, T>,
    b: crate::tree::Root<P, T>,
    scenario: Scenario,
) -> crate::tree::Root<P, T>
where
    P: Clone + Ord + AsRef<[u8]> + std::fmt::Debug + BorshSerialize + BorshDeserialize,
    T: PartialEq + std::fmt::Debug + BorshSerialize + BorshDeserialize,
{
    fn x<P: Ord, T>(_v: &Version<P>, _k: Key, _m: &Message<T>) {}

    block_on(async move {
        match scenario {
            Scenario::LocalLocal => {
                let local_a = local::Exchange::start(a, x, x);
                let local_b = local::Exchange::start(b, x, x);
                match mirror(local_a, local_b).await {
                    Err(e) => match e {},
                    Ok(result) => result.1,
                }
            }

            Scenario::LocalRemote => {
                // Two ends of a full-duplex in-memory pipe: anything the
                // client side writes is readable by the server side and
                // vice versa. Splitting each end gives the four halves the
                // remote proxies expect.
                let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
                let (a_r, a_w) = tokio::io::split(a_side);
                let (b_r, b_w) = tokio::io::split(b_side);

                let local_a = local::Exchange::start(a, x, x);
                let remote_b = remote::Exchange::start(a_r, a_w);
                let client = mirror(local_a, remote_b);

                let local_b = local::Exchange::start(b, x, x);
                let remote_a = remote::Exchange::start(b_r, b_w);
                let server = mirror(local_b, remote_a);

                // Both sides poll on the same current-thread task; no
                // spawning, no Send bound, no thread handoff.
                let (client_result, server_result) = tokio::join!(client, server);
                let out = client_result.expect("test client").0;
                let peer_out = server_result.expect("peer server").0;
                assert_eq!(out, peer_out, "local-remote endpoints should converge");
                out
            }
        }
    })
}

const SCENARIOS: [Scenario; 2] = [Scenario::LocalLocal, Scenario::LocalRemote];

proptest! {

    /// Mirroring a node with itself is a no-op: the two replicas have
    /// identical content and versions, so the reconciled tree is unchanged.
    #[test]
    fn idempotent(a in arb_tree_root("a", 0..=8)) {
        for scenario in SCENARIOS {
            prop_assert_eq!(mirror_via(a.clone(), a.clone(), scenario), a.clone());
        }
    }

    /// The reconciled tree is the same regardless of which replica
    /// initiates and which responds.
    #[test]
    fn commutative(
        a in arb_tree_root("a", 0..=8),
        b in arb_tree_root("b", 0..=8),
    ) {
        for scenario in SCENARIOS {
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
        a in arb_tree_root("a", 0..=8),
        b in arb_tree_root("b", 0..=8),
    ) {
        for scenario in SCENARIOS {
            let ab = mirror_via(a.clone(), b.clone(), scenario);
            prop_assert_eq!(mirror_via(ab.clone(), b.clone(), scenario), ab);
        }
    }

    /// Three-way mirror is order-independent: syncing (a,b) then c
    /// produces the same tree as syncing a then (b,c).
    #[test]
    fn associative(
        a in arb_tree_root("a", 0..=4),
        b in arb_tree_root("b", 0..=4),
        c in arb_tree_root("c", 0..=4),
    ) {
        for scenario in SCENARIOS {
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

        // The wrapper version must be a causal upper bound on every action
        // we apply — `Tree::react` maintains the same invariant by `|=`-ing
        // each action's version into the tree's version vector.
        let wrap = |actions: &[(Path, Version<String>, Action<()>)]| crate::tree::Root {
            version: actions
                .iter()
                .fold(Version::default(), |acc, (_, v, _)| acc | v.clone()),
            root: act(None, actions.to_vec()),
        };

        let tree_a = wrap(&actions_a);
        let tree_b = wrap(&actions_b);

        let mut all_actions = actions_a;
        all_actions.extend(actions_b);
        let expected = wrap(&all_actions);

        for scenario in SCENARIOS {
            let mirrored = mirror_via(tree_a.clone(), tree_b.clone(), scenario);
            prop_assert_eq!(mirrored, expected.clone());
        }
    }
}
