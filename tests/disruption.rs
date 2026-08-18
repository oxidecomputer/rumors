//! Party linearity and disjointness under arbitrary disruption.
//!
//! Two simulations, one engine of invariants (`common::sim`):
//!
//! - **Intra-process**: a fleet of peers on one multi-thread runtime,
//!   every gossip session, bootstrap, send, and redact spawned at once,
//!   over in-memory wires that may be severed at arbitrary byte offsets.
//! - **Inter-process**: peers split across genuinely separate OS
//!   processes — the test binary re-executes itself as each child — over a
//!   real TCP link (one socket per stream; see `common::tcp`) with the same
//!   fault injection on the child side, children retiring home at the end
//!   so the id-space can be audited.
//!
//! Both assert the same global properties, stated on each test below. Task
//! and process interleavings are nondeterministic, so a counterexample may
//! not replay byte-for-byte; the invariants quantify over *all*
//! interleavings, so any failure is a genuine one.

mod common;

use std::collections::BTreeSet;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use proptest::prelude::*;
use rumors::{Peer, Retire, Rumors};
use tokio::net::{TcpListener, TcpStream};

use crate::common::fault::{self, FaultPlan};
use crate::common::sim::{
    Activity, MAX_PLAN_PEERS, MAX_PLAN_SCRIPT_OPS, MAX_PLAN_SEED_MESSAGES, Plan, Redaction,
    RetireOp, Session, Transfer, arb_fault, arb_plan, assert_converged, assert_deletion_honored,
    assert_honest_error, assert_honest_gossip, assert_party_invariants, assert_value_oracle,
    is_honest_error, lost_custody, probe_disjointness, quiesce, run_plan, survivor_readouts,
};
use crate::common::tcp;
use crate::common::window::{WindowAssignment, WindowChoice};
use crate::common::wire::bootstrap_fork_async;

/// A fresh multi-thread runtime per simulation, so tasks interleave with
/// real parallelism rather than cooperative scheduling alone.
fn mt_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build multi-thread runtime")
}

// ---- intra-process ----------------------------------------------------------

proptest! {
    /// Under arbitrary concurrent gossip over wires cut at arbitrary byte
    /// offsets, the global party invariants hold:
    ///
    /// 1. every session failure is an injected I/O fault, never
    ///    `PartyOverlap` or a protocol violation;
    /// 2. at every probed instant the live parties are pairwise disjoint;
    /// 3. after a clean heal, all survivors converge to identical content;
    /// 4. when no hand-off was lost in flight, the surviving parties
    ///    fold-join back to exactly `Party::seed()` — the id-space is
    ///    conserved with no duplication and no leak;
    /// 5. no retained redaction's key is live at any survivor (deletion
    ///    honoring against the execution-time redaction log; every
    ///    redaction is retained whenever `possible_losses` is zero);
    /// 6. when no hand-off was lost in flight, the converged value
    ///    multiset equals the plan's inserts minus the logged redactions.
    ///    The gate is sound because `possible_losses == 0` covers message
    ///    content across faulted retires: every retire arm either leaves
    ///    the retiree whole, confirms a committed absorber session (which
    ///    per `Peer::retire`'s contract reconciles content exactly as
    ///    gossip would), or increments the counter, and a faulted
    ///    bootstrap risks only identity space.
    ///
    /// Peer-vs-peer equality (3) alone cannot catch every survivor
    /// agreeing on the *wrong* set; (5) and (6) check the fleet against a
    /// ledger independent of the merge machinery.
    ///
    /// The chaos: overlapping sessions through
    /// cloned [`Rumors`] handles, concurrent sends and redactions,
    /// bootstraps served mid-chaos against the same shared state, and
    /// retirements.
    #[test]
    fn disrupted_concurrent_gossip_upholds_party_invariants(plan in arb_plan()) {
        mt_runtime().block_on(check_plan(plan));
    }

    /// The floor-everywhere baseline leg: the same invariants as
    /// `disrupted_concurrent_gossip_upholds_party_invariants`, with
    /// every window pinned at the serialization floor on every
    /// iteration.
    ///
    /// The capacity-one orderings the deadlock-freedom argument
    /// certifies are deterministically exercised in this engine, not
    /// merely with the probability the swept leg happens to draw.
    #[test]
    fn disrupted_concurrent_gossip_upholds_party_invariants_at_floor(
        plan in arb_plan().prop_map(|mut plan| {
            plan.windows = WindowAssignment::floor();
            plan
        }),
    ) {
        mt_runtime().block_on(check_plan(plan));
    }
}

/// Run one plan through the full invariant battery: execute, heal,
/// then convergence, party, deletion-honoring, and value-ledger checks.
async fn check_plan(plan: Plan) {
    let outcome = run_plan(plan).await;
    quiesce(&outcome.peers).await;
    let readouts = survivor_readouts(&outcome.peers);
    assert_converged(&outcome.peers, &readouts);
    assert_party_invariants(&outcome.peers, outcome.possible_losses);
    assert_deletion_honored(&readouts, &outcome.redactions);
    assert_value_oracle(
        &readouts,
        outcome.possible_losses,
        &outcome.inserted,
        &outcome.redactions,
    );
}

// ---- value-oracle adequacy tripwires -----------------------------------------

/// Whether `f` panics, with the unwind caught so the test can assert on it.
fn panics(f: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_err()
}

/// A small, fault-free plan for the tripwires.
///
/// Deterministically loss-free by construction (no fault is ever
/// injected), with enough content that a live key always exists to
/// corrupt the ledger around.
fn tripwire_plan() -> Plan {
    Plan {
        n_peers: 2,
        seed_messages: vec![10, 20, 30],
        faulty_boots: vec![],
        scripts: vec![
            vec![Activity::Send(40), Activity::Redact(0)],
            vec![Activity::Send(50)],
        ],
        sessions: vec![Session {
            a: 0,
            b: 1,
            fault_a: FaultPlan::NONE,
            fault_b: FaultPlan::NONE,
        }],
        retires: vec![],
        // One floor and one default endpoint: the corruption checks run
        // against an asymmetric-window session, the sweep's general case.
        windows: WindowAssignment::new(vec![WindowChoice::Floor, WindowChoice::Default]),
    }
}

/// Adequacy tripwire for the value oracle: the checks must catch the two
/// known-bad mechanisms they exist for before their green is trusted.
///
/// A *suppressed redaction* — the application called `redact()` (so the
/// ledger holds it) but the mechanism left the leaf live — must fail both
/// the deletion-honoring check and the multiset check; it is simulated by
/// appending a ledger entry for a key that is genuinely live in the
/// converged fleet. A *dropped insert* — a value the plan sent but the
/// network silently lost — must fail the multiset check; it is simulated
/// by appending a never-sent value to the insert ledger. The uncorrupted
/// ledger must pass both checks in the same run, so this test also pins
/// that the real assertions are green on an honest execution.
#[test]
fn value_oracle_tripwires_catch_known_bad_mechanisms() {
    mt_runtime().block_on(async {
        let outcome = run_plan(tripwire_plan()).await;
        assert_eq!(
            outcome.possible_losses, 0,
            "a fault-free plan must be loss-free by construction"
        );
        quiesce(&outcome.peers).await;
        let readouts = survivor_readouts(&outcome.peers);
        assert_converged(&outcome.peers, &readouts);

        // Green on the honest run: both checks pass the uncorrupted ledger.
        assert_deletion_honored(&readouts, &outcome.redactions);
        assert_value_oracle(&readouts, 0, &outcome.inserted, &outcome.redactions);

        // Known-bad mechanism 1: a suppressed redaction. Its ledger entry
        // names a key still live in the converged fleet.
        let (&key, &value) = readouts[0]
            .iter()
            .next()
            .expect("the tripwire plan leaves live content");
        let mut suppressed = outcome.redactions.clone();
        suppressed.push(Redaction {
            key,
            value,
            retained: true,
        });
        assert!(
            panics(|| assert_deletion_honored(&readouts, &suppressed)),
            "the deletion-honoring check must catch a suppressed redaction"
        );
        assert!(
            panics(|| assert_value_oracle(&readouts, 0, &outcome.inserted, &suppressed)),
            "the multiset check must catch a suppressed redaction"
        );

        // Known-bad mechanism 2: a dropped insert. The ledger holds a value
        // the converged fleet never received.
        let mut dropped = outcome.inserted.clone();
        dropped.push(0xDEAD_BEEF);
        assert!(
            panics(|| assert_value_oracle(&readouts, 0, &dropped, &outcome.redactions)),
            "the multiset check must catch a dropped insert"
        );
    });
}

// ---- custody regressions -----------------------------------------------------

/// Custody of a founder's final content follows the retire sequence
/// transitively.
///
/// The concrete reviewed counterexample: founder 1 redacts, retires into
/// founder 2 (committed), then 2 retires toward 0 and that transfer is
/// lost. Founder 1's cargo rode in 2 and is gone with it, so both must be
/// reported lost — deriving loss from each logger's own retire outcome
/// alone would leave 1 retained and blame the protocol on an honest run
/// (its redaction can no longer reach the survivors). Recoveries move
/// custody in neither direction.
#[test]
fn custody_chain_loss_is_transitive() {
    let lost = lost_custody(3, &[(1, 2, Transfer::Committed), (2, 0, Transfer::Lost)]);
    assert_eq!(
        lost,
        BTreeSet::from([1, 2]),
        "founder 1's cargo rode in founder 2's lost transfer"
    );

    let lost = lost_custody(3, &[(1, 2, Transfer::Recovered), (2, 0, Transfer::Lost)]);
    assert_eq!(
        lost,
        BTreeSet::from([2]),
        "a recovered retiree keeps its own cargo; only the lost transfer forfeits"
    );
}

/// An unbroken chain of committed transfers retains custody end to end:
/// nothing is reported lost, so every logger's redactions stay subject
/// to the unconditional deletion-honoring check.
///
/// The transitive weakening in [`lost_custody`] must never eat honest
/// coverage.
#[test]
fn custody_committed_chain_retains() {
    let lost = lost_custody(
        3,
        &[(1, 2, Transfer::Committed), (2, 0, Transfer::Committed)],
    );
    assert!(
        lost.is_empty(),
        "a fully committed chain loses nothing: {lost:?}"
    );
}

/// End-to-end deterministic run of a committed retire chain over clean
/// wires.
///
/// Founder 1 redacts a seed message, retires into 2, which retires into
/// 0 — both transfers commit, the run is loss-free, the redaction rides
/// the chain into the survivor, and both ledger checks hold. The
/// corruption half then re-proves the checks' liveness in the presence
/// of retires: a fabricated retained redaction of a live key must fail
/// deletion honoring and the multiset equality.
#[test]
fn value_oracle_survives_committed_retire_chain() {
    mt_runtime().block_on(async {
        let plan = Plan {
            n_peers: 3,
            seed_messages: vec![10, 20, 30],
            faulty_boots: vec![],
            scripts: vec![vec![], vec![Activity::Redact(0)], vec![Activity::Send(40)]],
            sessions: vec![Session {
                a: 0,
                b: 1,
                fault_a: FaultPlan::NONE,
                fault_b: FaultPlan::NONE,
            }],
            retires: vec![
                RetireOp {
                    retiree: 1,
                    absorber: 2,
                    fault: FaultPlan::NONE,
                },
                RetireOp {
                    retiree: 2,
                    absorber: 0,
                    fault: FaultPlan::NONE,
                },
            ],
            windows: WindowAssignment::new(vec![
                WindowChoice::Floor,
                WindowChoice::Default,
                WindowChoice::Floor,
            ]),
        };
        let outcome = run_plan(plan).await;
        assert_eq!(
            outcome.possible_losses, 0,
            "clean wires commit every transfer"
        );
        assert!(
            !outcome.redactions.is_empty(),
            "founder 1 holds the seed messages, so its redact always executes"
        );
        assert!(
            outcome.redactions.iter().all(|r| r.retained),
            "a fully committed chain retains every redaction"
        );
        quiesce(&outcome.peers).await;
        let readouts = survivor_readouts(&outcome.peers);
        assert_converged(&outcome.peers, &readouts);
        assert_deletion_honored(&readouts, &outcome.redactions);
        assert_value_oracle(&readouts, 0, &outcome.inserted, &outcome.redactions);

        // Liveness after the custody weakening: fabricating a retained
        // redaction of a live key must still fire both checks.
        let (&key, &value) = readouts[0]
            .iter()
            .next()
            .expect("live content survives the chain");
        let mut corrupted = outcome.redactions.clone();
        corrupted.push(Redaction {
            key,
            value,
            retained: true,
        });
        assert!(
            panics(|| assert_deletion_honored(&readouts, &corrupted)),
            "deletion honoring must still fire through a retire chain"
        );
        assert!(
            panics(|| assert_value_oracle(&readouts, 0, &outcome.inserted, &corrupted)),
            "the multiset check must still fire through a retire chain"
        );
    });
}

// ---- MAX_CUT derivation pin --------------------------------------------------

/// Distinct values per side of the envelope session: one more than the
/// most content an entire plan can mint anywhere.
///
/// Derived from the generator's own bounds so the dominance premise
/// cannot drift from the strategy.
const ENVELOPE_VALUES_PER_SIDE: u64 =
    (MAX_PLAN_SEED_MESSAGES + MAX_PLAN_PEERS * MAX_PLAN_SCRIPT_OPS + 1) as u64;

/// Byte extent of the envelope session, per endpoint.
///
/// The construction dominates a plan's *value count* exactly — each
/// endpoint holds more unique content than an entire plan can mint —
/// and exercises the version shapes plans produce at the generator's
/// bounds: the fleet sits on a `MAX_PLAN_PEERS`-party fork lattice,
/// every party contributes a send tick and a redaction tick, and two
/// star rounds entangle every party's ticks into both endpoints'
/// version bounds before the measured, fully-divergent session runs at
/// the sweep's widest window. Byte extent is not *proven* maximal over
/// version shapes (plan versions vary in dimensions no single
/// construction dominates); the two-sided band in
/// [`max_cut_spans_the_envelope_session`] is what keeps the constant
/// tracking reality. Metered with the same counters the fault cuts
/// spend, so the result is directly comparable to cut offsets.
async fn envelope_session_bytes() -> usize {
    let seed = WindowChoice::Default
        .apply(Peer::<u64>::seed())
        .into_rumors();
    let mut fleet = vec![seed];
    for _ in 1..MAX_PLAN_PEERS {
        fleet.push(
            common::wire::bootstrap_fork_with_window_async(&fleet[0], WindowChoice::Default).await,
        );
    }
    // One send tick and one redaction tick per party: each peer marks
    // and immediately redacts its own marker (its snapshot holds only
    // the marker — nothing has gossiped yet), leaving every party's
    // ticks in its version bounds without leaving shared live content
    // that would blunt the divergence.
    for (i, peer) in fleet.iter().enumerate() {
        peer.send(2_000_000 + i as u64);
        let (marker, _, _) = peer
            .snapshot()
            .iter()
            .next()
            .expect("the peer holds exactly its own marker");
        peer.redact(marker);
    }
    // Two star rounds spread every party's ticks into every peer's
    // bounds (the first collects at the hub, the second redistributes).
    for _ in 0..2 {
        for i in 1..fleet.len() {
            common::wire::wire_gossip_async(&fleet[0], &fleet[i]).await;
        }
    }
    let (a, b) = (&fleet[1], &fleet[2]);
    common::wire::diverge(a, b, ENVELOPE_VALUES_PER_SIDE);
    let (link_a, link_b) = rumors::link::memory();
    let (mut link_a, meter_a) = fault::metered(link_a);
    let (mut link_b, meter_b) = fault::metered(link_b);
    let (out_a, out_b) = tokio::join!(a.gossip(&mut link_a), b.gossip(&mut link_b));
    out_a.expect("envelope session A");
    out_b.expect("envelope session B");
    meter_a.written().max(meter_b.written())
}

/// Pins `MAX_CUT` to the envelope session's measured byte extent, from
/// both sides.
///
/// Every byte of the envelope session is a reachable cut offset
/// (`measured <= MAX_CUT`), and the cut range is not vacuously wide
/// (`MAX_CUT <= 2 * measured`), so generated cuts keep landing inside
/// real sessions rather than past their end. The envelope's dominance
/// premise (exact on value count, representative on version shapes) is
/// stated at [`envelope_session_bytes`].
#[test]
fn max_cut_spans_the_envelope_session() {
    let measured = mt_runtime().block_on(envelope_session_bytes());
    println!("envelope session bytes per endpoint: {measured}");
    assert!(
        measured <= crate::common::sim::MAX_CUT,
        "the envelope session moves {measured} bytes per endpoint, beyond \
         MAX_CUT ({}): deep-session cut offsets are unreachable",
        crate::common::sim::MAX_CUT,
    );
    assert!(
        crate::common::sim::MAX_CUT <= 2 * measured,
        "MAX_CUT ({}) is more than twice the envelope session's {measured} \
         bytes: most generated cuts would land past the end of every \
         session and never fire",
        crate::common::sim::MAX_CUT,
    );
}

// ---- inter-process ----------------------------------------------------------

/// Environment protocol between the parent test and its child processes.
/// The presence of `CHILD_ADDR` is what turns the re-executed test binary
/// into a child peer.
const CHILD_ADDR: &str = "RUMORS_SIM_CHILD_ADDR";
const CHILD_INDEX: &str = "RUMORS_SIM_CHILD_INDEX";
const CHILD_SENDS: &str = "RUMORS_SIM_CHILD_SENDS";
const CHILD_BOOT: &str = "RUMORS_SIM_CHILD_BOOT";
const CHILD_SESSIONS: &str = "RUMORS_SIM_CHILD_SESSIONS";
const CHILD_RETIRE: &str = "RUMORS_SIM_CHILD_RETIRE";

/// Child exit codes: the loss-accounting back-channel. Anything else
/// (including a panic's 101) fails the parent test.
const EXIT_CLEAN: i32 = 0;
/// Retired cleanly, but an earlier faulty bootstrap attempt failed: the
/// fork served for it may be orphaned (possible loss).
const EXIT_BOOT_LOSS: i32 = 2;
/// The final retirement ended [`Retire::Uncertain`]: the party may be in
/// limbo (possible loss).
const EXIT_UNCERTAIN: i32 = 3;
/// A state the protocol promises is unreachable for this topology.
const EXIT_ANOMALY: i32 = 4;

/// Wall-clock bound on each child process.
const CHILD_DEADLINE: Duration = Duration::from_secs(60);

/// The value of child `index`'s `s`-th send: distinct per child and per
/// send, so the parent can assert that a cleanly-retired child's content
/// all made it home.
fn child_value(index: usize, s: usize) -> u64 {
    (index as u64 + 1) * 1_000_000 + s as u64
}

fn encode_cut(cut: Option<usize>) -> String {
    cut.map_or_else(|| "-".to_owned(), |n| n.to_string())
}

fn decode_cut(s: &str) -> Option<usize> {
    (s != "-").then(|| s.parse().expect("malformed cut budget"))
}

fn encode_fault(fault: &FaultPlan) -> String {
    format!(
        "{}:{}",
        encode_cut(fault.write_cut),
        encode_cut(fault.read_cut)
    )
}

fn decode_fault(s: &str) -> FaultPlan {
    let (write, read) = s.split_once(':').expect("malformed fault plan");
    FaultPlan {
        write_cut: decode_cut(write),
        read_cut: decode_cut(read),
    }
}

/// One child process's script: how many sends, the fault plan for an
/// initial (deliberately lossy) bootstrap attempt, per-session fault
/// plans, and the fault plan for its final retirement.
#[derive(Debug, Clone)]
struct ChildPlan {
    n_sends: usize,
    boot: FaultPlan,
    sessions: Vec<FaultPlan>,
    retire: FaultPlan,
}

#[derive(Debug, Clone)]
struct ProcPlan {
    n_parent_peers: usize,
    seed_messages: Vec<u64>,
    children: Vec<ChildPlan>,
}

fn arb_child_plan(faults: bool) -> impl Strategy<Value = ChildPlan> {
    (
        0usize..6,
        arb_fault(faults),
        prop::collection::vec(arb_fault(faults), 1..4),
        arb_fault(faults),
    )
        .prop_map(|(n_sends, boot, sessions, retire)| ChildPlan {
            n_sends,
            boot,
            sessions,
            retire,
        })
}

/// As in `arb_plan`, the leading `bool` turns fault injection off for half
/// of all plans, so the sharp seed-reconstitution check runs often.
fn arb_proc_plan() -> impl Strategy<Value = ProcPlan> {
    any::<bool>().prop_flat_map(|faults| {
        (
            1usize..=2,
            prop::collection::vec(any::<u64>(), 0..4),
            prop::collection::vec(arb_child_plan(faults), 1..=3),
        )
            .prop_map(|(n_parent_peers, seed_messages, children)| ProcPlan {
                n_parent_peers,
                seed_messages,
                children,
            })
    })
}

/// Kill (and reap) a child process if the parent unwinds before it exits.
struct KillOnDrop(std::process::Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(8))]

    /// The same four invariants as the intra-process simulation, with the
    /// fleet split across OS processes gossiping over real TCP sockets
    /// severed at arbitrary byte offsets.
    ///
    /// Child processes bootstrap from
    /// the parent, gossip concurrently with it (and with its own
    /// in-process sessions), then retire home. Cleanly-retired children
    /// must additionally leave every one of their sends in the parent's
    /// converged content, and a loss-free run must fold the parent's
    /// surviving parties back to exactly `Party::seed()`.
    #[test]
    fn inter_process_disruption_upholds_party_invariants(plan in arb_proc_plan()) {
        mt_runtime().block_on(run_proc_plan(plan));
    }
}

// ---- re-minted inter-process counterexamples ---------------------------------
//
// Historical shrunk counterexamples, preserved as explicit constructions:
// their committed seeds regenerate through the fault strategy's cut range,
// so a range change re-maps the offsets and the seed no longer replays the
// case it pinned. Each test runs the exact plan its seed's shrink recorded,
// under the same invariants as the proptest above. The seed files stay
// committed; these constructions carry the counterexamples themselves.

/// Shorthand for one endpoint's fault plan in a re-minted construction.
fn fp(write_cut: Option<usize>, read_cut: Option<usize>) -> FaultPlan {
    FaultPlan {
        write_cut,
        read_cut,
    }
}

/// Re-minted counterexample: a single clean child whose final retirement
/// wire dies at the very first written byte, forcing the
/// recovered-then-retry path against a live parent.
#[test]
fn remint_child_retire_cut_at_first_byte() {
    mt_runtime().block_on(run_proc_plan(ProcPlan {
        n_parent_peers: 1,
        seed_messages: vec![],
        children: vec![ChildPlan {
            n_sends: 0,
            boot: FaultPlan::NONE,
            sessions: vec![FaultPlan::NONE],
            retire: fp(Some(0), None),
        }],
    }));
}

/// Re-minted counterexample: one child whose deliberately lossy first
/// bootstrap dies mid-transfer and whose gossip sessions are cut in both
/// directions, retiring cleanly afterward.
#[test]
fn remint_child_faulted_boot_and_sessions() {
    mt_runtime().block_on(run_proc_plan(ProcPlan {
        n_parent_peers: 1,
        seed_messages: vec![8910283091],
        children: vec![ChildPlan {
            n_sends: 2,
            boot: fp(Some(1198), None),
            sessions: vec![fp(Some(124), None), fp(Some(1308), Some(935))],
            retire: FaultPlan::NONE,
        }],
    }));
}

/// Re-minted counterexample: three children with cuts across every phase
/// (bootstrap, sessions, retirement), overlapping at the parent.
#[test]
fn remint_three_children_cut_across_phases() {
    mt_runtime().block_on(run_proc_plan(ProcPlan {
        n_parent_peers: 1,
        seed_messages: vec![16893878652516216069, 17088246115921829969],
        children: vec![
            ChildPlan {
                n_sends: 1,
                boot: fp(None, Some(595)),
                sessions: vec![FaultPlan::NONE],
                retire: fp(None, Some(1243)),
            },
            ChildPlan {
                n_sends: 2,
                boot: fp(Some(1733), Some(1980)),
                sessions: vec![fp(Some(151), Some(348))],
                retire: fp(Some(1390), None),
            },
            ChildPlan {
                n_sends: 2,
                boot: fp(Some(637), Some(259)),
                sessions: vec![
                    FaultPlan::NONE,
                    fp(Some(1206), Some(1750)),
                    fp(Some(954), Some(25)),
                ],
                retire: fp(Some(98), Some(1600)),
            },
        ],
    }));
}

/// Re-minted counterexample: three children whose cuts sit near the deep
/// end of the fault range the case was found under, severing sessions and
/// retirements late in their byte streams.
#[test]
fn remint_three_children_deep_cuts() {
    mt_runtime().block_on(run_proc_plan(ProcPlan {
        n_parent_peers: 1,
        seed_messages: vec![597761422003064892],
        children: vec![
            ChildPlan {
                n_sends: 4,
                boot: fp(None, Some(670)),
                sessions: vec![fp(Some(559), None), fp(Some(2047), None)],
                retire: fp(Some(1947), Some(1274)),
            },
            ChildPlan {
                n_sends: 0,
                boot: fp(None, Some(1943)),
                sessions: vec![FaultPlan::NONE, FaultPlan::NONE, fp(Some(1489), None)],
                retire: fp(Some(695), None),
            },
            ChildPlan {
                n_sends: 5,
                boot: FaultPlan::NONE,
                sessions: vec![fp(None, Some(1511)), FaultPlan::NONE, fp(None, Some(28))],
                retire: fp(Some(1124), None),
            },
        ],
    }));
}

async fn run_proc_plan(plan: ProcPlan) {
    // Parent fleet: the seed and its clean forks, as shared-state handles
    // so inbound sessions can overlap arbitrarily.
    let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
    {
        let mut batch = seed.batch();
        for &v in &plan.seed_messages {
            batch.send(v);
        }
    }
    let mut casts: Vec<Rumors<u64>> = vec![seed];
    for _ in 1..plan.n_parent_peers {
        let fork = bootstrap_fork_async(&casts[0]).await;
        casts.push(fork);
    }

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind simulation listener");
    let addr = listener.local_addr().expect("listener address");

    // Serve every inbound connection with plain gossip, which transparently
    // handles a bootstrapping or retiring counterparty. Errors are expected
    // (the children sever wires); *dishonest* errors are recorded for the
    // final assertion rather than panicking inside a detached task, and
    // every error conservatively counts as a possible in-flight loss (a
    // dying session may have been a bootstrap holding a donated fork).
    let serve_errors = Arc::new(AtomicUsize::new(0));
    let dishonest = Arc::new(Mutex::new(Vec::<String>::new()));
    let accept = {
        let casts = casts.clone();
        let serve_errors = Arc::clone(&serve_errors);
        let dishonest = Arc::clone(&dishonest);
        tokio::spawn(async move {
            let mut sessions = tokio::task::JoinSet::new();
            let mut next = 0usize;
            loop {
                let Ok((socket, _)) = listener.accept().await else {
                    break;
                };
                let handle = casts[next % casts.len()].clone();
                next += 1;
                let serve_errors = Arc::clone(&serve_errors);
                let dishonest = Arc::clone(&dishonest);
                sessions.spawn(async move {
                    // A child that dies before the listener-port swap is the
                    // same honest severance as one that dies mid-session.
                    let mut link = match tcp::link(socket).await {
                        Ok(link) => link,
                        Err(_) => {
                            serve_errors.fetch_add(1, Ordering::Relaxed);
                            return;
                        }
                    };
                    if let Err(e) = handle.gossip(&mut link).await {
                        serve_errors.fetch_add(1, Ordering::Relaxed);
                        if !is_honest_error(&e) {
                            dishonest
                                .lock()
                                .expect("dishonest log")
                                .push(format!("{e:?}"));
                        }
                    }
                });
            }
        })
    };

    // Probe parent-side party disjointness while the children hammer it.
    let done = Arc::new(AtomicBool::new(false));
    let prober = tokio::spawn(probe_disjointness(casts.clone(), Arc::clone(&done)));

    // Spawn the children: this same test binary, re-executed straight into
    // `sim_child` with its script in the environment.
    let exe = std::env::current_exe().expect("current test binary");
    let mut children = Vec::new();
    for (index, child) in plan.children.iter().enumerate() {
        let sessions: Vec<String> = child.sessions.iter().map(encode_fault).collect();
        let process = std::process::Command::new(&exe)
            .args(["--exact", "sim_child", "--ignored"])
            .env(CHILD_ADDR, addr.to_string())
            .env(CHILD_INDEX, index.to_string())
            .env(CHILD_SENDS, child.n_sends.to_string())
            .env(CHILD_BOOT, encode_fault(&child.boot))
            .env(CHILD_SESSIONS, sessions.join(","))
            .env(CHILD_RETIRE, encode_fault(&child.retire))
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn child process");
        children.push(KillOnDrop(process));
    }

    // Reap the children, folding their exit codes into the loss accounting.
    let deadline = tokio::time::Instant::now() + CHILD_DEADLINE;
    let mut possible_losses = 0usize;
    let mut clean_children = vec![false; plan.children.len()];
    for (index, child) in children.iter_mut().enumerate() {
        let status = loop {
            if let Some(status) = child.0.try_wait().expect("poll child") {
                break status;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "child {index} did not finish within {CHILD_DEADLINE:?}"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        };
        match status.code() {
            Some(EXIT_CLEAN) => clean_children[index] = true,
            Some(EXIT_BOOT_LOSS) | Some(EXIT_UNCERTAIN) => possible_losses += 1,
            other => panic!(
                "child {index} exited abnormally ({other:?}): an invariant \
                 violation or panic in the child process"
            ),
        }
    }
    possible_losses += serve_errors.load(Ordering::Relaxed);

    // Wind down: stop the prober and the accept loop (dropping its
    // `JoinSet` aborts any straggling serve task), then reclaim the
    // parent's `Peer`s — `try_into_peer` resolves once every serving clone
    // is gone, so this is the synchronization point proving quiescence.
    // The heal phase below runs on the data plane, so each reclaimed
    // `Peer` converts straight back out.
    done.store(true, Ordering::Release);
    prober.await.expect("prober task");
    accept.abort();
    let _ = accept.await;
    let mut survivors = Vec::new();
    for cast in casts {
        survivors.push(
            cast.try_into_peer()
                .await
                .expect("all serving clones dropped")
                .into_rumors(),
        );
    }

    assert!(
        dishonest.lock().expect("dishonest log").is_empty(),
        "serving sessions surfaced non-fault errors: {:?}",
        dishonest.lock().expect("dishonest log")
    );

    quiesce(&survivors).await;
    let readouts = survivor_readouts(&survivors);
    assert_converged(&survivors, &readouts);

    // Every cleanly-retired child's sends must have survived into the
    // parent's converged content: its final retirement reconciled before
    // the party hand-off, so nothing it published may be lost.
    let live: BTreeSet<u64> = readouts[0].values().copied().collect();
    for (index, child) in plan.children.iter().enumerate() {
        if clean_children[index] {
            for s in 0..child.n_sends {
                assert!(
                    live.contains(&child_value(index, s)),
                    "send {s} of cleanly-retired child {index} was lost"
                );
            }
        }
    }

    assert_party_invariants(&survivors, possible_losses);
}

// ---- the child process ------------------------------------------------------

/// Child-process entry point for `inter_process_disruption_*`: **not a
/// test**.
///
/// The parent re-executes this binary with `--exact sim_child
/// --ignored` and the script in the environment; without `CHILD_ADDR` set
/// (e.g. under `--run-ignored`) it is a no-op. Outcomes travel back as
/// exit codes (`EXIT_*`); invariant violations panic, which the parent
/// sees as an abnormal exit.
#[test]
#[ignore = "child-process entry point for the inter-process simulation, not a test"]
fn sim_child() {
    let Ok(addr) = std::env::var(CHILD_ADDR) else {
        return;
    };
    let code = mt_runtime().block_on(child_main(addr));
    if code != EXIT_CLEAN {
        std::process::exit(code);
    }
}

async fn child_main(addr: String) -> i32 {
    let env = |name: &str| std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set"));
    let index: usize = env(CHILD_INDEX).parse().expect("child index");
    let n_sends: usize = env(CHILD_SENDS).parse().expect("send count");
    let boot = decode_fault(&env(CHILD_BOOT));
    let sessions: Vec<FaultPlan> = {
        let raw = env(CHILD_SESSIONS);
        raw.split(',')
            .filter(|s| !s.is_empty())
            .map(decode_fault)
            .collect()
    };
    let retire_fault = decode_fault(&env(CHILD_RETIRE));

    // Join the universe. A faulty plan first attempts a bootstrap over a
    // cut wire: on failure the fork served for it may be orphaned, which
    // the parent must hear about (the exit code), and the child joins for
    // real over a clean connection.
    let mut had_boot_loss = false;
    let mut known: Option<Peer<u64>> = None;
    if !boot.is_clean() {
        let socket = TcpStream::connect(&addr)
            .await
            .expect("connect for faulty bootstrap");
        let link = tcp::link(socket).await.expect("swap listener ports");
        let mut link = fault::faulty_link(link, boot);
        match Peer::<u64>::bootstrap().join(&mut link).await {
            Ok(Some(k)) => known = Some(k.sync_window_floor()),
            Ok(None) => panic!("the parent never bootstraps"),
            Err(e) => {
                assert_honest_error(&e);
                had_boot_loss = true;
            }
        }
    }
    let known = match known {
        Some(k) => k,
        None => {
            let socket = TcpStream::connect(&addr)
                .await
                .expect("connect for bootstrap");
            let mut link = tcp::link(socket).await.expect("swap listener ports");
            Peer::<u64>::bootstrap()
                .join(&mut link)
                .await
                .expect("clean bootstrap")
                .expect("the parent serves every bootstrap")
                .sync_window_floor()
        }
    };

    // Chaos: local sends concurrent with possibly-severed gossip sessions
    // back to the parent.
    let cast = known.into_rumors();
    let sender = {
        let handle = cast.clone();
        tokio::spawn(async move {
            for s in 0..n_sends {
                handle.send(child_value(index, s));
                tokio::task::yield_now().await;
            }
        })
    };
    for fault in sessions {
        let socket = TcpStream::connect(&addr)
            .await
            .expect("connect for session");
        let link = tcp::link(socket).await.expect("swap listener ports");
        let mut link = fault::faulty_link(link, fault);
        let handle = cast.clone();
        assert_honest_gossip(&handle.gossip(&mut link).await);
    }
    sender.await.expect("sender task");

    // One clean session so everything this child published is home even
    // before the retirement reconciles.
    {
        let socket = TcpStream::connect(&addr)
            .await
            .expect("connect for final gossip");
        let mut link = tcp::link(socket).await.expect("swap listener ports");
        cast.gossip(&mut link).await.expect("clean final gossip");
    }

    // Retire home, possibly through a cut wire; a recovered retiree gets
    // one clean retry. Outcomes map to the exit-code protocol.
    let mut known = cast
        .try_into_peer()
        .await
        .expect("sender finished; sole handle");
    let mut fault = retire_fault;
    for _attempt in 0..2 {
        let socket = TcpStream::connect(&addr).await.expect("connect for retire");
        let link = tcp::link(socket).await.expect("swap listener ports");
        let mut link = fault::faulty_link(link, fault);
        match known.retire(&mut link).await {
            Retire::Retired => {
                return if had_boot_loss {
                    EXIT_BOOT_LOSS
                } else {
                    EXIT_CLEAN
                };
            }
            Retire::Recovered {
                peer: recovered,
                error,
            } => {
                assert_honest_error(&error);
                known = recovered;
                fault = FaultPlan::NONE;
            }
            Retire::Uncertain { error } => {
                assert_honest_error(&error);
                return EXIT_UNCERTAIN;
            }
            Retire::Declined { .. } => return EXIT_ANOMALY,
        }
    }
    // A clean retry can only end `Retired`; reaching here is an anomaly.
    EXIT_ANOMALY
}
