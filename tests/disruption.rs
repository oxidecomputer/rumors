//! Party linearity and disjointness under arbitrary disruption.
//!
//! One simulation (`common::sim`): a fleet of peers on one multi-thread
//! runtime, every gossip session, bootstrap, send, and redact spawned at
//! once, over in-memory wires that may be severed at arbitrary byte
//! offsets. The global properties are stated on each test below. Task
//! interleavings are nondeterministic, so a counterexample may not replay
//! byte-for-byte; the invariants quantify over *all* interleavings, so any
//! failure is a genuine one.

mod common;

use std::collections::BTreeSet;

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;
use rumors::testing::run_to_quiescence;
use rumors::{Peer, Rumors};

use crate::common::fault::{self, FaultPlan, Vanish};
use crate::common::sim::{
    Activity, MAX_PLAN_PEERS, MAX_PLAN_SCRIPT_OPS, MAX_PLAN_SEED_MESSAGES, MAX_SHRINK_TIME,
    MAX_VANISH_OFFSET, MAX_VANISH_STREAM, Plan, Redaction, RetireOp, Session, Transfer, arb_plan,
    assert_converged, assert_deletion_honored, assert_party_invariants, assert_survivor,
    assert_value_oracle, lost_custody, quiesce, run_plan, survivor_readouts,
};
use crate::common::window::{WindowAssignment, WindowChoice};
use crate::common::wire::bootstrap_fork;

/// A fresh multi-thread runtime per simulation, so tasks interleave with
/// real parallelism rather than cooperative scheduling alone.
fn mt_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build multi-thread runtime")
}

// ---- the property -----------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        max_shrink_time: MAX_SHRINK_TIME,
        ..ProptestConfig::default()
    })]

    /// Under arbitrary concurrent gossip over wires cut at arbitrary byte
    /// offsets, with peers vanishing mid-stream, the global party
    /// invariants hold:
    ///
    /// 1. every session failure is an injected I/O fault or the honest
    ///    severance a vanished peer leaves behind, never `PartyOverlap`
    ///    or a protocol violation;
    /// 2. at every probed instant the live parties are pairwise disjoint;
    /// 3. after a clean heal, all survivors converge to identical content;
    /// 4. when no hand-off was lost in flight, the surviving parties
    ///    fold-join back to exactly `Party::seed()` -- the id-space is
    ///    conserved with no duplication and no leak;
    /// 5. no retained redaction's message is live at any survivor (deletion
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
    /// bootstraps served mid-chaos against the same shared state,
    /// retirements, and endpoints that vanish mid-protocol (their session
    /// dropped with its link, promised streams never opened). Every surviving
    /// session must finish before the deadline, including after a vanish.
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

/// The vanish dimension is live in the generated plan population: across
/// a deterministic sample of plans run through the engine, some endpoint
/// reaches its vanish point.
///
/// A drawn vanish past the end of every stream its endpoint opens never
/// trips, so the floor counts vanishes that fired, not plans that carry
/// one.
#[test]
fn vanishes_fire_in_the_generated_population() {
    let mut runner = TestRunner::deterministic();
    let strategy = arb_plan();
    let runtime = mt_runtime();
    let mut fired = 0usize;
    for _ in 0..32 {
        let plan = strategy
            .new_tree(&mut runner)
            .expect("plan strategy always generates")
            .current();
        fired += runtime.block_on(run_plan(plan)).vanished;
    }
    assert!(
        fired > 0,
        "no endpoint of a sampled plan reached its vanish point: the vanish \
         dimension has silently left the population"
    );
}

/// Run one session under the closed-world poller in which `a`, holding
/// the only new content, vanishes at `point`, and `b` survives with
/// nothing to send: `b`'s outcome, or the poller's name for `b` parking.
fn survive_a_vanish(
    point: Vanish,
) -> Result<Result<rumors::Gossiped, rumors::Error>, rumors::testing::Quiescence> {
    let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    a.send_all(0..8).unwrap();
    let b = bootstrap_fork(&a);
    a.send_all(8..16).unwrap();
    let (a_link, b_link) = rumors::link::memory();
    let vanishing = FaultPlan {
        vanish: Some(point),
        ..FaultPlan::NONE
    };
    run_to_quiescence(async {
        let (driven, survivor) = futures::join!(
            fault::drive(a_link, vanishing, async move |link| a.gossip(link).await),
            async {
                let mut link = fault::faulty(b_link, FaultPlan::NONE);
                b.gossip(&mut link).await
            },
        );
        assert!(
            driven.outcome.is_none(),
            "the vanishing peer never reached its point: {:?}",
            driven.outcome
        );
        survivor
    })
}

/// A peer whose counterparty vanishes mid-stream ends its session with
/// an honest error: the vanish fires, and the survivor reads end-of-stream
/// inside a frame.
#[test]
fn survivor_notices_a_peer_vanished_mid_stream() {
    match survive_a_vanish(Vanish::OnStream {
        index: 0,
        offset: 8,
    }) {
        Ok(survivor) => assert_survivor(&survivor),
        Err(quiescence) => {
            panic!("the survivor parked after its peer vanished mid-stream: {quiescence:?}")
        }
    }
}

/// Control EOF ends a wait for the departed peer's first data stream.
#[test]
fn survivor_notices_a_peer_vanished_before_its_first_stream() {
    let survivor = survive_a_vanish(Vanish::AtFirstConnect)
        .expect("the survivor must not park on an owed stream after control EOF");
    assert_survivor(&survivor);
}

/// A retiree that vanishes mid-retirement is a recorded loss: its party
/// left with it, so the run counts one possible loss, its slot stays
/// empty, and the relaxed party check holds over the survivor.
///
/// The retiree holds content the absorber lacks, so its retirement opens
/// a stream and the mid-stream vanish fires.
#[test]
fn vanished_retiree_is_a_recorded_loss() {
    mt_runtime().block_on(async {
        let plan = Plan {
            n_peers: 2,
            seed_messages: vec![10],
            faulty_boots: vec![],
            scripts: vec![vec![], vec![Activity::Send(20), Activity::Send(30)]],
            sessions: vec![],
            retires: vec![RetireOp {
                retiree: 1,
                absorber: 0,
                fault: FaultPlan {
                    vanish: Some(Vanish::OnStream {
                        index: 0,
                        offset: 0,
                    }),
                    ..FaultPlan::NONE
                },
            }],
            windows: WindowAssignment::floor(),
        };
        let outcome = run_plan(plan).await;
        quiesce(&outcome.peers).await;
        let readouts = survivor_readouts(&outcome.peers);
        assert_converged(&outcome.peers, &readouts);
        // The party check first: an accounting that missed the vanish
        // would run it sharply and fail on the party that left.
        assert_party_invariants(&outcome.peers, outcome.possible_losses);
        assert_eq!(
            outcome.possible_losses, 1,
            "a vanished retiree is exactly one possible loss"
        );
        assert_eq!(
            outcome.peers.len(),
            1,
            "the vanished retiree's slot is empty"
        );
    });
}

/// Pins the vanish draw's two bounds to the envelope session's measured
/// stream shape, from both sides.
///
/// Every data stream an envelope endpoint opens is a reachable vanish
/// ordinal (`streams <= MAX_VANISH_STREAM`) and the ordinal range is not
/// vacuously wide (`MAX_VANISH_STREAM <= 2 * streams`); every byte of the
/// widest stream is a reachable offset (`widest <= MAX_VANISH_OFFSET`)
/// and the offset range is not vacuously wide
/// (`MAX_VANISH_OFFSET <= 2 * widest`), so generated vanishes keep landing
/// on streams that exist, inside them.
#[test]
fn vanish_draw_spans_the_envelope_session() {
    let extent = mt_runtime().block_on(envelope_session_bytes());
    println!(
        "envelope session per endpoint: {} data streams, widest {} bytes",
        extent.streams, extent.widest_stream
    );
    assert!(
        extent.streams <= MAX_VANISH_STREAM,
        "the envelope endpoint opens {} data streams, beyond MAX_VANISH_STREAM \
         ({MAX_VANISH_STREAM}): later streams are unreachable vanish points",
        extent.streams
    );
    assert!(
        MAX_VANISH_STREAM <= 2 * extent.streams,
        "MAX_VANISH_STREAM ({MAX_VANISH_STREAM}) is more than twice the envelope \
         endpoint's {} data streams: most generated vanishes would name a stream \
         that never opens",
        extent.streams
    );
    assert!(
        extent.widest_stream <= MAX_VANISH_OFFSET,
        "the envelope's widest data stream carries {} bytes, beyond \
         MAX_VANISH_OFFSET ({MAX_VANISH_OFFSET}): deep offsets are unreachable",
        extent.widest_stream
    );
    assert!(
        MAX_VANISH_OFFSET <= 2 * extent.widest_stream,
        "MAX_VANISH_OFFSET ({MAX_VANISH_OFFSET}) is more than twice the envelope's \
         widest stream ({} bytes): most generated vanishes would land past the \
         end of every stream",
        extent.widest_stream
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
/// injected), with enough content that a live message always exists to
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
/// appending a ledger entry for a message that is genuinely live in the
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
        // names a message still live in the converged fleet. (Readout keys
        // are canonical version bytes, so the version decodes back out.)
        let (key, &value) = readouts[0]
            .iter()
            .next()
            .expect("the tripwire plan leaves live content");
        let mut suppressed = outcome.redactions.clone();
        suppressed.push(Redaction {
            version: rumors::Version::decode(key.as_slice())
                .expect("readout keys are canonical version bytes"),
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
        // redaction of a live message must still fire both checks.
        let (key, &value) = readouts[0]
            .iter()
            .next()
            .expect("live content survives the chain");
        let mut corrupted = outcome.redactions.clone();
        corrupted.push(Redaction {
            version: rumors::Version::decode(key.as_slice())
                .expect("readout keys are canonical version bytes"),
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
/// most content an entire plan can create anywhere.
///
/// Derived from the generator's own bounds so the dominance premise
/// cannot drift from the strategy.
const ENVELOPE_VALUES_PER_SIDE: u64 =
    (MAX_PLAN_SEED_MESSAGES + MAX_PLAN_PEERS * MAX_PLAN_SCRIPT_OPS + 1) as u64;

/// Byte extent of the envelope session, per endpoint.
///
/// The construction dominates a plan's *value count* exactly — each
/// endpoint holds more unique content than an entire plan can create —
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
async fn envelope_session_bytes() -> EnvelopeExtent {
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
        peer.send(2_000_000 + i as u64).unwrap();
        let marker = {
            let snapshot = peer.snapshot();
            let (marker, _) = snapshot
                .iter()
                .next()
                .expect("the peer holds exactly its own marker");
            marker.clone()
        };
        peer.redact(&marker);
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
    EnvelopeExtent {
        bytes: meter_a.written().max(meter_b.written()),
        streams: meter_a.streams_opened().max(meter_b.streams_opened()),
        widest_stream: meter_a.widest_stream().max(meter_b.widest_stream()),
    }
}

/// The envelope session's extent per endpoint, each the wider endpoint's:
/// total bytes written, data streams opened, and bytes on the widest data
/// stream.
struct EnvelopeExtent {
    bytes: usize,
    streams: usize,
    widest_stream: usize,
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
    let measured = mt_runtime().block_on(envelope_session_bytes()).bytes;
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
