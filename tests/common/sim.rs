//! Plan-driven disruption simulation: a fleet of peers gossiping
//! *concurrently* over fault-injected channels, with the global party
//! invariants probed throughout.
//!
//! Where the `schedule` machinery executes one gossip session at a time,
//! this engine spawns everything at once onto a multi-thread runtime:
//! overlapping sessions between arbitrary pairs (including several sessions
//! involving the same peer simultaneously, through cloned [`Rumors`]
//! handles), concurrent local sends and redactions, bootstraps served over
//! lossy wires, and a prober that re-checks global party disjointness while
//! the chaos is in flight. Task interleaving is genuinely nondeterministic;
//! the invariants asserted here must hold under *every* interleaving, so a
//! failure is always a true failure even if it does not replay
//! byte-for-byte.
//!
//! # Phases
//!
//! 1. **Fleet**: one seed and its clean bootstrap forks.
//! 2. **Chaos**: every session, every activity script, and every extra
//!    bootstrap attempt runs concurrently; channel cuts land at arbitrary
//!    byte offsets via [`FaultPlan`]s. Serving a bootstrap mid-chaos puts
//!    the snapshot-and-fork critical section under concurrent sends from
//!    sibling handles (see [`run_boot`]); a failed attempt may orphan the
//!    served fork's id-region — counted, see below. Each peer also carries
//!    one observer of each kind ([`Messages`](rumors::Messages) and
//!    [`CausalMessages`](rumors::CausalMessages)), drained concurrently
//!    with the chaos and asserting the delivery contracts inline — no key
//!    twice, no causal inversion, and full coverage of the peer's live set
//!    once the writers settle (see [`run_observers`]). This is the only
//!    place the observers' watch-coalescing path runs against genuinely
//!    parallel writers.
//! 3. **Retire**: planned retirements over possibly-faulty wires — the only
//!    phase that moves whole parties, exercising the
//!    recovered/uncertain/retired algebra under fire.
//! 4. The caller heals the survivors ([`quiesce`]) and asserts the global
//!    invariants ([`assert_party_invariants`], [`assert_converged`]).
//!
//! # Loss accounting
//!
//! Party id-regions can leave the live universe *legitimately* when a wire
//! drops mid-hand-off: a bootstrap fork lost in flight, or a retiree's
//! [`Retire::Uncertain`] whose absorber also failed. The engine counts
//! every such *possible* loss conservatively in
//! [`SimOutcome::possible_losses`]. Disjointness must hold regardless;
//! the sharper invariant — the surviving parties fold-join back to exactly
//! [`Party::seed`] — is asserted whenever the count is zero (which the
//! plan generator arranges often, by disabling fault injection entirely in
//! half its plans).

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use before::Party;
use proptest::prelude::*;
use rumors::{Error, Key, Peer, Retire, Rumors, Version};
use tokio::io::duplex;

use crate::common::fault::{self, FaultPlan};
use crate::common::oracle::readout;
use crate::common::wire::wire_gossip_async;

/// In-memory channel capacity, matching `wire.rs`.
const DUPLEX_BUF: usize = 8 * 1024;

/// Upper bound on the byte offset at which a cut can land. Sessions in
/// these plans are small, so this comfortably spans everything from the
/// first preamble byte to deep inside the reconciliation descent.
const MAX_CUT: usize = 2048;

/// Headroom on the heal loop, as in `peer::quiesce`.
const MAX_QUIESCE_ROUNDS_PER_PEER: usize = 16;

/// One concurrently-executed simulation plan. See the module docs for how
/// the pieces are scheduled.
#[derive(Debug, Clone)]
pub struct Plan {
    /// Fleet size built by clean bootstraps before anything else runs.
    pub n_peers: usize,
    /// Messages inserted at the seed before any fork.
    pub seed_messages: Vec<u64>,
    /// Extra joiners bootstrapped *during* the chaos phase, one per entry;
    /// the entry is the fault plan for the *bootstrapping* endpoint. Even
    /// a clean entry matters: it serves the party-fork critical section
    /// while sibling handles are mid-send (see [`run_boot`]).
    pub faulty_boots: Vec<FaultPlan>,
    /// Per-peer scripts (length `n_peers`) of local sends and redactions,
    /// run concurrently with every session.
    pub scripts: Vec<Vec<Activity>>,
    /// Gossip sessions, all spawned at once.
    pub sessions: Vec<Session>,
    /// Retirements run after the chaos phase settles.
    pub retires: Vec<RetireOp>,
}

/// One local operation in a peer's concurrent activity script.
#[derive(Debug, Clone, Copy)]
pub enum Activity {
    /// Insert this value.
    Send(u64),
    /// Redact the key at this index (modulo the live count) of the peer's
    /// own snapshot at execution time — a key the application could have
    /// observed; a no-op while the peer holds nothing.
    Redact(usize),
}

/// One gossip session between fleet peers `a != b`, each endpoint with its
/// own fault plan.
#[derive(Debug, Clone, Copy)]
pub struct Session {
    pub a: usize,
    pub b: usize,
    pub fault_a: FaultPlan,
    pub fault_b: FaultPlan,
}

/// One planned retirement: `retiree` retires into `absorber` (distinct
/// fleet indices), the retiree's endpoint faulted by `fault`.
#[derive(Debug, Clone, Copy)]
pub struct RetireOp {
    pub retiree: usize,
    pub absorber: usize,
    pub fault: FaultPlan,
}

/// What a [`run_plan`] execution leaves behind for the caller's assertions.
pub struct SimOutcome {
    /// Every peer still alive: the fleet minus retired/consumed members.
    pub peers: Vec<Rumors<u64>>,
    /// Conservative count of hand-offs in which an id-region *may* have
    /// been lost in flight; see the module docs. Zero enables the sharp
    /// seed-reconstitution check.
    pub possible_losses: usize,
}

// ---- strategies ------------------------------------------------------------

/// Strategy for one endpoint's fault plan. With `faults` disabled it is
/// always clean, so a whole plan generated under `false` is loss-free by
/// construction; enabled, each direction independently stays clean or cuts
/// at an arbitrary offset.
pub fn arb_fault(faults: bool) -> BoxedStrategy<FaultPlan> {
    if !faults {
        return Just(FaultPlan::NONE).boxed();
    }
    let cut = prop_oneof![2 => Just(None), 3 => (0..MAX_CUT).prop_map(Some)];
    (cut.clone(), cut)
        .prop_map(|(write_cut, read_cut)| FaultPlan {
            write_cut,
            read_cut,
        })
        .boxed()
}

fn arb_activity() -> impl Strategy<Value = Activity> {
    prop_oneof![
        any::<u64>().prop_map(Activity::Send),
        (0usize..64).prop_map(Activity::Redact),
    ]
}

/// `a` and `b` are kept distinct by construction (offset in `1..n`), so the
/// shrinker can never collapse a session onto a single peer.
fn arb_session(n: usize, faults: bool) -> impl Strategy<Value = Session> {
    (0..n, 1..n, arb_fault(faults), arb_fault(faults)).prop_map(
        move |(a, off, fault_a, fault_b)| Session {
            a,
            b: (a + off) % n,
            fault_a,
            fault_b,
        },
    )
}

fn arb_retire(n: usize, faults: bool) -> impl Strategy<Value = RetireOp> {
    (0..n, 1..n, arb_fault(faults)).prop_map(move |(retiree, off, fault)| RetireOp {
        retiree,
        absorber: (retiree + off) % n,
        fault,
    })
}

/// A whole plan. The leading `bool` decides fault injection for the entire
/// plan: half of all generated plans are loss-free by construction, so the
/// sharp seed-reconstitution invariant is exercised as often as the
/// disruption paths.
pub fn arb_plan() -> impl Strategy<Value = Plan> {
    (any::<bool>(), 2usize..=5).prop_flat_map(|(faults, n)| {
        (
            prop::collection::vec(any::<u64>(), 0..8),
            prop::collection::vec(arb_fault(faults), 0..=3),
            prop::collection::vec(prop::collection::vec(arb_activity(), 0..8), n),
            prop::collection::vec(arb_session(n, faults), 1..16),
            prop::collection::vec(arb_retire(n, faults), 0..=2),
        )
            .prop_map(
                move |(seed_messages, faulty_boots, scripts, sessions, retires)| Plan {
                    n_peers: n,
                    seed_messages,
                    faulty_boots,
                    scripts,
                    sessions,
                    retires,
                },
            )
    })
}

// ---- honesty of failures ---------------------------------------------------

/// Every error an honest, single-universe simulation can surface is an
/// injected I/O fault that *truncated* a frame. Anything else —
/// [`Error::PartyOverlap`] above all, network/protocol mismatches, or a frame
/// that arrived whole but failed to parse — is an invariant violation, not a
/// disruption, and fails the test on the spot.
///
/// A wire cut stops the byte stream mid-frame, so a faulted read surfaces as an
/// I/O error whose kind is `UnexpectedEof` (or a write/broken-pipe variant) —
/// never a complete-but-malformed frame. A decode failure (`InvalidData`) is
/// therefore a protocol/codec bug, not a fault: it is exactly how a
/// non-canonical [`Party`] on the wire once slipped through, so reject it
/// alongside the non-I/O variants.
pub fn assert_honest_error(e: &Error) {
    let honest = matches!(e, Error::Io(io) if io.kind() != std::io::ErrorKind::InvalidData);
    assert!(
        honest,
        "an honest, single-universe simulation must only surface injected I/O \
         faults that truncate a frame (a cut never corrupts one); got: {e:?}"
    );
}

/// [`assert_honest_error`] over a session outcome.
pub fn assert_honest_gossip(out: &Result<(), Error>) {
    if let Err(e) = out {
        assert_honest_error(e);
    }
}

// ---- the engine ------------------------------------------------------------

/// Run one gossip session between two handles over a fault-injected
/// in-memory wire. Each side's halves are owned by its own task, so the
/// failing side's drop surfaces as EOF to its counterparty instead of
/// wedging the session.
async fn run_session(a: Rumors<u64>, b: Rumors<u64>, fault_a: FaultPlan, fault_b: FaultPlan) {
    let (side_a, side_b) = duplex(DUPLEX_BUF);
    let task_a = tokio::spawn(async move {
        let (mut r, mut w) = fault::faulty(side_a, fault_a);
        a.gossip(&mut r, &mut w).await
    });
    let task_b = tokio::spawn(async move {
        let (mut r, mut w) = fault::faulty(side_b, fault_b);
        b.gossip(&mut r, &mut w).await
    });
    assert_honest_gossip(&task_a.await.expect("session task A"));
    assert_honest_gossip(&task_b.await.expect("session task B"));
}

/// Serve one bootstrap from `server` mid-chaos, the joiner's endpoint
/// faulted by `fault`. Returns the newcomer, or `None` for a possible
/// in-flight loss of the donated fork.
///
/// This is the most delicate intra-set race the engine exercises: serving
/// a bootstrap must snapshot the tree and speculatively fork the party in
/// one critical section, while sibling `Rumors` clones of the same set
/// concurrently send, redact, and gossip. A torn snapshot/fork would hand
/// the newcomer a version exceeding what its party slice justifies —
/// surfacing downstream as a disjointness or convergence failure.
///
/// The serving side stays clean; the joiner's fault plan covers both
/// observable directions of a duplex (its read cut models the server's
/// frames dying in flight). A joiner that fails may or may not have cost
/// the server its donated fork, so it conservatively counts as a possible
/// loss either way.
async fn run_boot(server: Rumors<u64>, fault: FaultPlan) -> Option<Peer<u64>> {
    let (boot_side, serve_side) = duplex(DUPLEX_BUF);
    let serve = tokio::spawn(async move {
        let (mut r, mut w) = fault::faulty(serve_side, FaultPlan::NONE);
        server.gossip(&mut r, &mut w).await
    });
    let boot = tokio::spawn(async move {
        let (mut r, mut w) = fault::faulty(boot_side, fault);
        Peer::<u64>::bootstrap(&mut r, &mut w).await
    });
    assert_honest_gossip(&serve.await.expect("bootstrap serve task"));
    match boot.await.expect("bootstrap join task") {
        Ok(Some(newcomer)) => Some(newcomer),
        Ok(None) => unreachable!("the serving peer is never itself bootstrapping"),
        Err(e) => {
            assert_honest_error(&e);
            None
        }
    }
}

/// Run one peer's activity script, yielding between operations so it
/// interleaves with every in-flight session.
async fn run_activity(handle: Rumors<u64>, script: Vec<Activity>) {
    for op in script {
        match op {
            Activity::Send(value) => {
                handle.send(value);
            }
            Activity::Redact(index) => {
                let keys: Vec<Key> = handle.snapshot().iter().map(|(k, _, _)| k).collect();
                if !keys.is_empty() {
                    handle.redact(keys[index % keys.len()]);
                }
            }
        }
        tokio::task::yield_now().await;
    }
}

/// Drain one peer's observers — one of each kind — concurrently with the
/// chaos, asserting the delivery contracts on every step:
///
/// - **Exactly-once**: neither observer ever yields a key twice.
/// - **Causal order** (the causal observer): no delivery is ever a causal
///   predecessor of an earlier one.
/// - **Coverage**: once `done` (the writers have settled), a final drain
///   leaves every key live in the peer's snapshot observed by both.
///
/// The interesting part is not the assertions but where they run: under
/// genuinely parallel sends, redactions, and gossip sessions on sibling
/// handles, this is the only exercise of the observers' watch-coalescing
/// path (`send_if_modified` racing `borrow_and_update`) outside
/// single-threaded tests.
async fn run_observers(handle: Rumors<u64>, done: Arc<AtomicBool>) {
    use futures::FutureExt;

    let mut plain = handle.unordered_messages();
    let mut causal = handle.causal_messages();
    let mut plain_seen: BTreeSet<Key> = BTreeSet::new();
    let mut causal_seen: BTreeSet<Key> = BTreeSet::new();
    let mut causal_delivered: Vec<Version> = Vec::new();

    loop {
        // Settle *before* draining: after the writers finish, one more full
        // drain below sees their complete effect, so the coverage check
        // races nothing.
        let finished = done.load(Ordering::Acquire);

        while let Some(Some((key, version, _))) = plain.borrow_next().now_or_never() {
            let _ = version;
            assert!(
                plain_seen.insert(key),
                "Messages delivered key {key:?} twice"
            );
        }
        while let Some(Some((key, version, _))) = causal.borrow_next().now_or_never() {
            assert!(
                causal_seen.insert(key),
                "CausalMessages delivered key {key:?} twice"
            );
            // `Version` is a partial order: `!(version < earlier)` also
            // admits concurrent pairs, which `version >= earlier` would
            // reject.
            #[allow(clippy::neg_cmp_op_on_partial_ord)]
            for earlier in &causal_delivered {
                assert!(
                    !(version < earlier),
                    "causal inversion: {version:?} delivered after {earlier:?}, \
                     which causally depends on it"
                );
            }
            causal_delivered.push(version.clone());
        }

        if finished {
            break;
        }
        tokio::task::yield_now().await;
    }

    // The writers have settled and both observers are quiet: everything
    // live in the set was live at each observer's final pass.
    for (key, _, _) in handle.snapshot().iter() {
        assert!(
            plain_seen.contains(&key),
            "Messages never delivered live key {key:?}"
        );
        assert!(
            causal_seen.contains(&key),
            "CausalMessages never delivered live key {key:?}"
        );
    }
}

/// Concurrently re-assert global pairwise party disjointness until `done`.
///
/// Sound mid-flight: a region is removed from its holder's shared state
/// *before* it rides the wire and joined into the recipient *after* it
/// arrives, so no interleaving of these per-peer aliases can witness one
/// region twice unless linearity is actually broken.
pub async fn probe_disjointness(handles: Vec<Rumors<u64>>, done: Arc<AtomicBool>) {
    loop {
        let finished = done.load(Ordering::Acquire);
        let parties: Vec<(usize, Party)> = handles
            .iter()
            .enumerate()
            .filter_map(|(i, h)| h.dangerously_alias_party().map(|p| (i, p)))
            .collect();
        for (n, (i, pi)) in parties.iter().enumerate() {
            for (j, pj) in parties.iter().skip(n + 1) {
                assert!(
                    pi.is_disjoint(pj),
                    "live parties must stay pairwise disjoint at every instant: \
                     peers {i} and {j} overlap ({pi:?} vs {pj:?})"
                );
            }
        }
        if finished {
            return;
        }
        tokio::task::yield_now().await;
    }
}

/// Execute `plan`: build the fleet, run the chaos and retire phases, and
/// return the survivors plus the loss accounting. Panics on any invariant
/// violation observable mid-run (dishonest errors, transient overlap).
pub async fn run_plan(plan: Plan) -> SimOutcome {
    let mut possible_losses = 0usize;

    // Phase 1: fleet. The seed's content predates every fork.
    let seed = Peer::<u64>::seed().into_rumors();
    {
        let mut batch = seed.batch();
        for &v in &plan.seed_messages {
            batch.send(v);
        }
    }
    let mut fleet: Vec<Rumors<u64>> = vec![seed];
    for _ in 1..plan.n_peers {
        let child = crate::common::wire::bootstrap_fork_async(&fleet[0]).await;
        fleet.push(child);
    }

    // Phase 2: chaos. Everything at once: every session, every activity
    // script, every bootstrap, and the disjointness prober, interleaving
    // freely.
    let casts = fleet;
    let done = Arc::new(AtomicBool::new(false));
    let prober = tokio::spawn(probe_disjointness(casts.clone(), Arc::clone(&done)));
    let observers: Vec<_> = casts
        .iter()
        .map(|handle| tokio::spawn(run_observers(handle.clone(), Arc::clone(&done))))
        .collect();

    let mut tasks = Vec::new();
    for (handle, script) in casts.iter().zip(&plan.scripts) {
        tasks.push(tokio::spawn(run_activity(handle.clone(), script.clone())));
    }
    for s in &plan.sessions {
        tasks.push(tokio::spawn(run_session(
            casts[s.a].clone(),
            casts[s.b].clone(),
            s.fault_a,
            s.fault_b,
        )));
    }
    let boot_tasks: Vec<_> = plan
        .faulty_boots
        .iter()
        .enumerate()
        .map(|(i, fault)| tokio::spawn(run_boot(casts[i % casts.len()].clone(), *fault)))
        .collect();
    for task in tasks {
        task.await.expect("chaos task");
    }
    let mut newcomers = Vec::new();
    for task in boot_tasks {
        match task.await.expect("bootstrap task") {
            Some(newcomer) => newcomers.push(newcomer),
            None => possible_losses += 1,
        }
    }
    done.store(true, Ordering::Release);
    prober.await.expect("prober task");
    for observer in observers {
        observer.await.expect("observer task");
    }

    // Phase 3: reunite, then run the planned retirements. Retirement
    // requires the unique `Peer` (the Peer/Rumors XOR), so parties
    // move only now — over wires that may still drop mid-hand-off.
    let mut slots: Vec<Option<Peer<u64>>> = Vec::with_capacity(casts.len());
    for cast in casts {
        slots.push(Some(
            cast.try_into_peer()
                .await
                .expect("every chaos task dropped its handles"),
        ));
    }
    // Newcomers born mid-chaos hold live id-regions, so they face the same
    // heal and party audit as the founding fleet. Appended after the
    // founders, they sit above every index a `RetireOp` can name.
    slots.extend(newcomers.into_iter().map(Some));

    for op in &plan.retires {
        // A slot emptied by an earlier retirement skips the op.
        let Some(retiree) = slots[op.retiree].take() else {
            continue;
        };
        let Some(absorber) = slots[op.absorber].take() else {
            slots[op.retiree] = Some(retiree);
            continue;
        };
        // The absorber's side of a retirement is plain gossip, which lives
        // on `Rumors`; it converts back the moment the session ends.
        let absorber = absorber.into_rumors();
        let (retiree_side, absorber_side) = duplex(DUPLEX_BUF);
        let fault = op.fault;
        let (outcome, absorbed) = tokio::join!(
            async move {
                let (mut r, mut w) = fault::faulty(retiree_side, fault);
                retiree.retire(&mut r, &mut w).await
            },
            async {
                let (mut r, mut w) = fault::faulty(absorber_side, FaultPlan::NONE);
                absorber.gossip(&mut r, &mut w).await
            },
        );
        assert_honest_gossip(&absorbed);
        match outcome {
            // The retiree believes its party was delivered; if the absorber
            // failed too, delivery is unconfirmed on both sides and the
            // region may be in limbo.
            Retire::Retired => {
                if absorbed.is_err() {
                    possible_losses += 1;
                }
            }
            // The party never crossed the wire: the retiree survives whole.
            Retire::Recovered { peer, error } => {
                assert_honest_error(&error);
                slots[op.retiree] = Some(peer);
            }
            // In flight when the wire died. If the absorber committed
            // cleanly it holds the party (no loss); otherwise it is gone.
            Retire::Uncertain { error } => {
                assert_honest_error(&error);
                if absorbed.is_err() {
                    possible_losses += 1;
                }
            }
            Retire::Declined { .. } => {
                unreachable!("the absorber runs plain gossip and never declines")
            }
        }
        slots[op.absorber] = Some(
            absorber
                .try_into_peer()
                .await
                .expect("the absorber's sole handle reclaims the Peer"),
        );
    }

    SimOutcome {
        peers: slots.into_iter().flatten().map(Peer::into_rumors).collect(),
        possible_losses,
    }
}

// ---- healing and the global assertions --------------------------------------

/// Drive the survivors to a full-mesh fixed point over clean wires, as
/// `peer::quiesce` does for the schedule tests.
pub async fn quiesce(peers: &[Rumors<u64>]) {
    let n = peers.len();
    if n < 2 {
        return;
    }
    let max_rounds = MAX_QUIESCE_ROUNDS_PER_PEER * n;
    for _ in 0..max_rounds {
        let before: Vec<([u8; rumors::MERKLE_HASH_LEN], Version)> = peers
            .iter()
            .map(|p| {
                let snapshot = p.snapshot();
                (snapshot.hash(), snapshot.latest().clone())
            })
            .collect();
        for i in 0..n {
            for j in (i + 1)..n {
                wire_gossip_async(&peers[i], &peers[j]).await;
            }
        }
        let changed = peers.iter().zip(&before).any(|(p, (hash, latest))| {
            let snapshot = p.snapshot();
            snapshot.hash() != *hash || snapshot.latest() != latest
        });
        if !changed {
            return;
        }
    }
    panic!("heal phase did not converge within {max_rounds} rounds for {n} peers");
}

/// After healing, every survivor holds identical live content: equal
/// `Key → value` readouts, equal observable hashes, equal causal versions.
pub fn assert_converged(peers: &[Rumors<u64>]) {
    let Some(first) = peers.first() else { return };
    let snapshot = first.snapshot();
    let expected = (
        readout(&snapshot),
        snapshot.hash(),
        snapshot.latest().clone(),
    );
    for (i, peer) in peers.iter().enumerate().skip(1) {
        let snapshot = peer.snapshot();
        let actual = (
            readout(&snapshot),
            snapshot.hash(),
            snapshot.latest().clone(),
        );
        assert_eq!(
            actual, expected,
            "peer {i} diverged from peer 0 after the heal phase"
        );
    }
}

/// The global party invariants over the surviving fleet:
///
/// 1. **Disjointness, always**: every pair of live parties is disjoint.
/// 2. **Linearity, sharply, when nothing was lost**: with zero possible
///    in-flight losses, fold-joining every live party reconstitutes
///    exactly [`Party::seed`] — every id-region is held by exactly one
///    live peer, and none leaked.
pub fn assert_party_invariants(peers: &[Rumors<u64>], possible_losses: usize) {
    let parties: Vec<Party> = peers
        .iter()
        .map(|k| {
            k.dangerously_alias_party()
                .expect("a live peer holds its party")
        })
        .collect();

    for (n, pi) in parties.iter().enumerate() {
        for (m, pj) in parties.iter().enumerate().skip(n + 1) {
            assert!(
                pi.is_disjoint(pj),
                "surviving peers {n} and {m} hold overlapping parties \
                 ({pi:?} vs {pj:?})"
            );
        }
    }

    if possible_losses == 0 {
        let mut parties = parties.into_iter();
        let mut whole = parties
            .next()
            .expect("at least one peer survives every plan");
        for party in parties {
            whole
                .join(party)
                .expect("pairwise-disjoint parties always join");
        }
        assert_eq!(
            whole,
            Party::seed(),
            "a loss-free run must reconstitute the seed's whole id-space"
        );
    }
}
