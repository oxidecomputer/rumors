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
//!    one observer of each kind
//!    ([`UnorderedMessages`](rumors::UnorderedMessages) and
//!    [`CausalMessages`](rumors::CausalMessages)), drained concurrently
//!    with the chaos and asserting the delivery contracts inline — no
//!    message twice, no causal inversion, and full coverage of the peer's live set
//!    once the writers settle (see [`run_observers`]). This is the only
//!    place the observers' watch-coalescing path runs against genuinely
//!    parallel writers.
//! 3. **Retire**: planned retirements over possibly-faulty wires — the only
//!    phase that moves whole parties, exercising the
//!    recovered/uncertain/retired algebra under fire.
//! 4. The caller heals the survivors ([`quiesce`]) and asserts the global
//!    invariants ([`assert_party_invariants`], [`assert_converged`],
//!    [`assert_deletion_honored`], [`assert_value_oracle`]).
//!
//! # The value oracle
//!
//! Peer-vs-peer equality ([`assert_converged`]) cannot see a bug in which
//! every survivor agrees on the *wrong* set, so the engine also keeps a
//! content ledger independent of the merge machinery: every inserted value
//! is known from the plan, and every executed redaction is logged at
//! execution time by [`run_activity`] — execution time because a
//! [`Activity::Redact`] resolves its target against the peer's snapshot
//! only when it runs, so no pre-run analysis of the plan can know which
//! `(Version, value)` it removed. [`SimOutcome`] carries both sides of the
//! ledger; [`assert_deletion_honored`] and [`assert_value_oracle`] check
//! the converged fleet against it.
//!
//! # Loss accounting
//!
//! Party id-regions can leave the live universe *legitimately* when a wire
//! drops mid-hand-off: a bootstrap fork lost in flight, or a retiree's
//! [`Retire::Uncertain`] whose absorber also failed. The engine counts
//! every such *possible* loss conservatively in
//! [`SimOutcome::possible_losses`]. Disjointness must hold regardless;
//! the sharper invariants — the surviving parties fold-join back to exactly
//! [`Party::seed`], and the converged value multiset equals the ledger's
//! inserts minus redactions — are asserted whenever the count is zero
//! (which the plan generator arranges often, by disabling fault injection
//! entirely in half its plans). Zero possible losses covers message
//! content across faulted retires, not only id-regions: every retire arm
//! either leaves the retiree whole ([`Retire::Recovered`]), confirms a
//! committed absorber session — which, per [`Peer::retire`]'s contract
//! (a session reconciles content exactly as gossip would), holds the
//! union including the retiree's unique messages and redactions — or
//! increments the counter; and a faulted bootstrap risks only identity
//! space, because a newborn holds no unique content.

use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use before::Party;
use proptest::prelude::*;
use rumors::error::{
    CodecDecodeErrorKind, CodecEncodeErrorKind, RemoteError, SendError, StreamError,
};
use rumors::{Error, MirrorError, Peer, Retire, Rumors, Version};

use crate::common::fault::{self, FaultPlan};
use crate::common::oracle::{readout, readout_multiset, version_key};
use crate::common::window::{WindowAssignment, WindowChoice, arb_window_choice};
use crate::common::wire::wire_gossip_async;

/// Upper bound on the byte offset at which a cut can land.
///
/// Derived from measurement, not transcribed: the envelope session —
/// a fully-divergent pair on the generator's maximal fork lattice, each
/// endpoint holding more unique content than an entire plan can mint,
/// every party's ticks entangled into the version bounds, at the
/// sweep's widest window — moves fewer bytes per endpoint than this
/// bound, and the bound stays within twice that measurement, so
/// generated cuts reach every byte of the envelope and keep landing
/// inside real sessions. The envelope dominates plan sessions exactly
/// on value count and representatively on version shapes (the boundary
/// is stated at the pin); the two-sided pin is
/// `max_cut_spans_the_envelope_session` in `tests/disruption.rs` —
/// re-measure there before touching this number.
pub const MAX_CUT: usize = 3072;

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
    /// the entry is the fault plan for the *bootstrapping* endpoint.
    ///
    /// Even a clean entry matters: it serves the party-fork critical
    /// section while sibling handles are mid-send (see [`run_boot`]).
    pub faulty_boots: Vec<FaultPlan>,
    /// Per-peer scripts (length `n_peers`) of local sends and redactions,
    /// run concurrently with every session.
    pub scripts: Vec<Vec<Activity>>,
    /// Gossip sessions, all spawned at once.
    pub sessions: Vec<Session>,
    /// Retirements run after the chaos phase settles.
    pub retires: Vec<RetireOp>,
    /// Window choices for the whole fleet: founder `i` runs at
    /// `windows.choice(i)`, and a newcomer bootstrapped by
    /// `faulty_boots[j]` takes `windows.choice(j)`.
    ///
    /// Founders draw independently, so the chaos includes sessions
    /// between differently-configured endpoints.
    pub windows: WindowAssignment,
}

/// One local operation in a peer's concurrent activity script.
#[derive(Debug, Clone, Copy)]
pub enum Activity {
    /// Insert this value.
    Send(u64),
    /// Redact the message at this index (modulo the live count) of the
    /// peer's own snapshot at execution time — a message the application
    /// could have observed; a no-op while the peer holds nothing.
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

/// One executed redaction, logged by [`run_activity`] at the moment it
/// resolved its target, and deduplicated per [`Version`] (two peers racing
/// to redact the same message are one redaction of it).
#[derive(Debug, Clone)]
pub struct Redaction {
    /// The version of the message actually redacted.
    pub version: Version,
    /// The value that message carried, read from the redactor's snapshot
    /// in the same pass that selected it.
    pub value: u64,
    /// Whether the redaction is guaranteed present in the surviving
    /// fleet's causal history.
    ///
    /// Retention means some logging founder's final content reached the
    /// survivors through an unbroken chain of committed transfers
    /// ([`lost_custody`]): it survived to the heal phase itself, or
    /// every hop of its retire chain committed.
    ///
    /// A redaction whose every logger's custody chain was broken by a
    /// loss arm may honestly never reach the survivors, so only
    /// retained redactions are subject to [`assert_deletion_honored`];
    /// with [`SimOutcome::possible_losses`] zero, every redaction is
    /// retained.
    pub retained: bool,
}

/// How one executed retirement moved content custody between fleet slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transfer {
    /// The retiree recovered whole: custody never moved.
    Recovered,
    /// The absorber committed its session: everything the retiree carried
    /// now rides in the absorber.
    ///
    /// Sound by [`Peer::retire`]'s contract — a retirement session
    /// reconciles content exactly as gossip would, *then* the peer
    /// absorbs the identity — so an absorber whose session committed has
    /// completed the content reconciliation and holds the full union; no
    /// commit-before-transfer ordering exists.
    Committed,
    /// The absorber's session failed: everything the retiree carried may
    /// be gone with it.
    Lost,
}

/// Which founders' final content may have failed to reach the surviving
/// fleet, given the executed retire sequence.
///
/// Custody is transitive: a founder's content is retained only through
/// an unbroken chain of committed transfers.
///
/// A founder that retired into a committed absorber rides in that
/// absorber — and is lost with it if the absorber is later dropped in a
/// loss arm. Each slot starts carrying its own founder; a committed
/// transfer moves the retiree's whole cargo into the absorber, a lost
/// transfer forfeits it, and a recovery moves nothing.
pub fn lost_custody(n_founders: usize, transfers: &[(usize, usize, Transfer)]) -> BTreeSet<usize> {
    // carriers[slot] = the founders whose final content currently rides
    // in that slot's peer.
    let mut carriers: Vec<BTreeSet<usize>> = (0..n_founders).map(|i| BTreeSet::from([i])).collect();
    let mut lost = BTreeSet::new();
    for &(retiree, absorber, transfer) in transfers {
        match transfer {
            Transfer::Recovered => {}
            Transfer::Committed => {
                let cargo = std::mem::take(&mut carriers[retiree]);
                carriers[absorber].extend(cargo);
            }
            Transfer::Lost => {
                lost.extend(std::mem::take(&mut carriers[retiree]));
            }
        }
    }
    lost
}

/// What a [`run_plan`] execution leaves behind for the caller's assertions.
pub struct SimOutcome {
    /// Every peer still alive: the fleet minus retired/consumed members.
    pub peers: Vec<Rumors<u64>>,
    /// Conservative count of hand-offs in which an id-region *may* have
    /// been lost in flight; see the module docs. Zero enables the sharp
    /// seed-reconstitution and value-multiset checks.
    pub possible_losses: usize,
    /// Every value the plan inserted anywhere: the seed's pre-fork batch
    /// plus every [`Activity::Send`] executed by a founder's script.
    ///
    /// Inserts are local operations and always execute, so this side of
    /// the ledger is known from the plan alone.
    pub inserted: Vec<u64>,
    /// The execution-time redaction log; see [`Redaction`].
    pub redactions: Vec<Redaction>,
}

// ---- strategies ------------------------------------------------------------

/// Most peers a plan's founding fleet can hold. Every bound here is
/// public so dominance arguments (the envelope session pin in
/// `tests/disruption.rs`) derive from the generator instead of
/// transcribing it.
pub const MAX_PLAN_PEERS: usize = 5;

/// Most messages a plan can insert at the seed before forking.
pub const MAX_PLAN_SEED_MESSAGES: usize = 7;

/// Most operations one founder's activity script can carry.
pub const MAX_PLAN_SCRIPT_OPS: usize = 7;

/// Strategy for one endpoint's fault plan.
///
/// With `faults` disabled it is always clean, so a whole plan generated
/// under `false` is loss-free by construction; enabled, each direction
/// independently stays clean or cuts at an arbitrary offset.
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

/// A whole plan.
///
/// The leading `bool` decides fault injection for the entire plan: half of
/// all generated plans are loss-free by construction, so the sharp
/// seed-reconstitution invariant is exercised as often as the disruption
/// paths.
pub fn arb_plan() -> impl Strategy<Value = Plan> {
    // Draw order is part of the seed-compatibility surface: committed
    // regression seeds regenerate each field from a stable prefix of the
    // RNG stream, so `windows` draws last.
    (any::<bool>(), 2usize..=MAX_PLAN_PEERS).prop_flat_map(|(faults, n)| {
        (
            prop::collection::vec(any::<u64>(), 0..=MAX_PLAN_SEED_MESSAGES),
            prop::collection::vec(arb_fault(faults), 0..=3),
            prop::collection::vec(
                prop::collection::vec(arb_activity(), 0..=MAX_PLAN_SCRIPT_OPS),
                n,
            ),
            prop::collection::vec(arb_session(n, faults), 1..16),
            prop::collection::vec(arb_retire(n, faults), 0..=2),
            prop::collection::vec(arb_window_choice(), n),
        )
            .prop_map(
                move |(seed_messages, faulty_boots, scripts, sessions, retires, windows)| Plan {
                    n_peers: n,
                    seed_messages,
                    faulty_boots,
                    scripts,
                    sessions,
                    retires,
                    windows: WindowAssignment::new(windows),
                },
            )
    })
}

// ---- honesty of failures ---------------------------------------------------

/// Assert `e` is an injected I/O fault that *truncated* a frame: the only
/// error an honest, single-universe simulation can surface.
///
/// Anything else — [`Error::PartyOverlap`] above all, network/protocol
/// mismatches, or a frame that arrived whole but failed to parse — is an
/// invariant violation, not a disruption, and fails the test on the spot.
///
/// A wire cut stops the byte stream mid-frame, so a faulted read surfaces as an
/// I/O error whose kind is `UnexpectedEof` (or a write/broken-pipe variant) —
/// never a complete-but-malformed frame. A decode failure (`InvalidData`) is
/// therefore a protocol/codec bug, not a fault: it is exactly how a
/// non-canonical [`Party`] on the wire once slipped through, so reject it
/// alongside the non-I/O variants.
pub fn assert_honest_error(e: &Error) {
    assert!(
        is_honest_error(e),
        "an honest, single-universe simulation must only surface injected I/O \
         faults that truncate a frame (a cut never corrupts one); got: {e:?}"
    );
}

/// Whether an error is exactly one of the disruption harness's wire cuts.
pub fn is_honest_error(error: &Error) -> bool {
    match error {
        Error::Io(error) => honest_io(error),
        // A cut that lands on the closing epilogue exchange is post-commit
        // but still an honest severed wire; a non-marker byte there
        // (`InvalidData`) stays dishonest, as everywhere.
        Error::Epilogue(error) => honest_io(error),
        Error::Mirror(MirrorError::Server(error)) => honest_remote(error),
        _ => false,
    }
}

/// Whether an I/O source is one of the fault harness's severed-wire outcomes.
///
/// `ConnectionRefused` belongs here because of how a dead peer manifests to
/// a stream *open*: on a per-connection transport (the inter-process TCP
/// link), a peer that died mid-session takes its stream listener with it,
/// and the next `connect` is refused rather than reset.
fn honest_io(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionRefused
            | std::io::ErrorKind::UnexpectedEof
    )
}

/// Recognize only typed V2 surfaces directly caused by a severed transport.
fn honest_remote(error: &RemoteError<Infallible>) -> bool {
    match error {
        RemoteError::HandshakeRead(source) | RemoteError::HandshakeWrite(source) => {
            honest_io(source)
        }
        // A stream truncated mid-frame, or a stream supply that died: both
        // are the transport dying somewhere the protocol did not choose.
        RemoteError::Stream(StreamError::Truncated { .. }) => true,
        RemoteError::Stream(StreamError::SupplyClosed { source, .. }) => {
            source.as_ref().is_none_or(honest_io)
        }
        RemoteError::Stream(StreamError::Decode(error)) => match &error.kind {
            CodecDecodeErrorKind::Read { source, .. }
            | CodecDecodeErrorKind::Truncated { source, .. } => honest_io(source),
            _ => false,
        },
        RemoteError::Send(SendError::Connect { source, .. } | SendError::Label { source, .. }) => {
            honest_io(source)
        }
        RemoteError::Send(SendError::Frame(error)) => match &error.kind {
            CodecEncodeErrorKind::Write { source, .. } | CodecEncodeErrorKind::Flush(source) => {
                honest_io(source)
            }
            _ => false,
        },
        _ => false,
    }
}

/// [`assert_honest_error`] over a session outcome.
pub fn assert_honest_gossip(out: &Result<rumors::Gossiped, Error>) {
    if let Err(e) = out {
        assert_honest_error(e);
    }
}

// ---- the engine ------------------------------------------------------------

/// Run one gossip session between two handles over a fault-injected
/// in-memory wire.
///
/// Each side's halves are owned by its own task, so the failing side's
/// drop surfaces as EOF to its counterparty instead of wedging the session.
async fn run_session(a: Rumors<u64>, b: Rumors<u64>, fault_a: FaultPlan, fault_b: FaultPlan) {
    let (link_a, link_b) = rumors::link::memory();
    let task_a = tokio::spawn(async move {
        let mut link = fault::faulty(link_a, fault_a);
        a.gossip(&mut link).await
    });
    let task_b = tokio::spawn(async move {
        let mut link = fault::faulty(link_b, fault_b);
        b.gossip(&mut link).await
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
async fn run_boot(
    server: Rumors<u64>,
    fault: FaultPlan,
    window: WindowChoice,
) -> Option<Peer<u64>> {
    let (boot_side, serve_side) = rumors::link::memory();
    let serve = tokio::spawn(async move {
        let mut link = fault::faulty(serve_side, FaultPlan::NONE);
        server.gossip(&mut link).await
    });
    let boot = tokio::spawn(async move {
        let mut link = fault::faulty(boot_side, fault);
        Peer::<u64>::bootstrap().join(&mut link).await
    });
    assert_honest_gossip(&serve.await.expect("bootstrap serve task"));
    match boot.await.expect("bootstrap join task") {
        Ok(Some(newcomer)) => Some(window.apply(newcomer)),
        Ok(None) => unreachable!("the serving peer is never itself bootstrapping"),
        Err(e) => {
            assert_honest_error(&e);
            None
        }
    }
}

/// Run one peer's activity script, yielding between operations so it
/// interleaves with every in-flight session.
///
/// Returns the `(Version, value)` of every redaction actually executed:
/// the target resolves against the peer's snapshot only here, so this
/// execution-time log is the one ground truth of what the plan redacted
/// (the value oracle's deletion side; see the module docs).
///
/// A logged redaction may race a sibling's redaction of the same message;
/// either way the message ends redacted network-wide, so the log stays
/// sound and [`run_plan`] deduplicates by version.
async fn run_activity(handle: Rumors<u64>, script: Vec<Activity>) -> Vec<(Version, u64)> {
    let mut redacted = Vec::new();
    for op in script {
        match op {
            Activity::Send(value) => {
                handle.send(value);
            }
            Activity::Redact(index) => {
                let live: Vec<(Version, u64)> = handle
                    .snapshot()
                    .iter()
                    .map(|(v, m)| (v.clone(), *m))
                    .collect();
                if !live.is_empty() {
                    let (version, value) = live[index % live.len()].clone();
                    handle.redact(&version);
                    redacted.push((version, value));
                }
            }
        }
        tokio::task::yield_now().await;
    }
    redacted
}

/// Drain one peer's observers — one of each kind — concurrently with the
/// chaos, asserting the delivery contracts on every step:
///
/// - **Exactly-once**: neither observer ever yields a message twice.
/// - **Causal order** (the causal observer): no delivery is ever a causal
///   predecessor of an earlier one.
/// - **Coverage**: once `done` (the writers have settled), a final drain
///   leaves every message live in the peer's snapshot observed by both.
///
/// The interesting part is not the assertions but where they run: under
/// genuinely parallel sends, redactions, and gossip sessions on sibling
/// handles, this is the only exercise of the observers' watch-coalescing
/// path (`send_if_modified` racing `borrow_and_update`) outside
/// single-threaded tests.
async fn run_observers(handle: Rumors<u64>, done: Arc<AtomicBool>) {
    use futures::{FutureExt, StreamExt};

    let mut plain = handle.unordered_messages();
    let mut causal = handle.causal_messages();
    let mut plain_seen: BTreeSet<Vec<u8>> = BTreeSet::new();
    let mut causal_seen: BTreeSet<Vec<u8>> = BTreeSet::new();
    let mut causal_delivered: Vec<Version> = Vec::new();

    loop {
        // Settle *before* draining: after the writers finish, one more full
        // drain below sees their complete effect, so the coverage check
        // races nothing.
        let finished = done.load(Ordering::Acquire);

        while let Some(Some((version, _))) = plain.next().now_or_never() {
            assert!(
                plain_seen.insert(version_key(&version)),
                "Messages delivered version {version:?} twice"
            );
        }
        while let Some(Some((version, _))) = causal.next().now_or_never() {
            assert!(
                causal_seen.insert(version_key(&version)),
                "CausalMessages delivered version {version:?} twice"
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
    for (version, _) in handle.snapshot().iter() {
        assert!(
            plain_seen.contains(version.as_bytes()),
            "Messages never delivered live version {version:?}"
        );
        assert!(
            causal_seen.contains(version.as_bytes()),
            "CausalMessages never delivered live version {version:?}"
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

    // The insert side of the value ledger, known from the plan alone:
    // sends are local operations and always execute.
    let inserted: Vec<u64> = plan
        .seed_messages
        .iter()
        .copied()
        .chain(plan.scripts.iter().flatten().filter_map(|op| match op {
            Activity::Send(value) => Some(*value),
            Activity::Redact(_) => None,
        }))
        .collect();

    // Phase 1: fleet. The seed's content predates every fork. Each
    // founder runs at its planned window choice, so the chaos phase mixes
    // floor, budgeted, and default windows across concurrent sessions.
    let seed = plan
        .windows
        .choice(0)
        .apply(Peer::<u64>::seed())
        .into_rumors();
    {
        let mut batch = seed.batch();
        for &v in &plan.seed_messages {
            batch.send(v);
        }
    }
    let mut fleet: Vec<Rumors<u64>> = vec![seed];
    for i in 1..plan.n_peers {
        let child = crate::common::wire::bootstrap_fork_with_window_async(
            &fleet[0],
            plan.windows.choice(i),
        )
        .await;
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

    let mut activity_tasks = Vec::new();
    for (handle, script) in casts.iter().zip(&plan.scripts) {
        activity_tasks.push(tokio::spawn(run_activity(handle.clone(), script.clone())));
    }
    let mut session_tasks = Vec::new();
    for s in &plan.sessions {
        session_tasks.push(tokio::spawn(run_session(
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
        .map(|(i, fault)| {
            tokio::spawn(run_boot(
                casts[i % casts.len()].clone(),
                *fault,
                plan.windows.choice(i),
            ))
        })
        .collect();
    // Per-founder execution-time redaction logs, indexed like `casts`.
    let mut redaction_logs: Vec<Vec<(Version, u64)>> = Vec::with_capacity(activity_tasks.len());
    for task in activity_tasks {
        redaction_logs.push(task.await.expect("activity task"));
    }
    for task in session_tasks {
        task.await.expect("session task");
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

    // How each executed retirement moved content custody, in execution
    // order; folded into the lost-founder set by [`lost_custody`] once
    // the sequence is complete (see [`Redaction::retained`]).
    let mut transfers: Vec<(usize, usize, Transfer)> = Vec::new();

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
        let (retiree_side, absorber_side) = rumors::link::memory();
        let fault = op.fault;
        let (outcome, absorbed) = tokio::join!(
            async move {
                let mut link = fault::faulty(retiree_side, fault);
                retiree.retire(&mut link).await
            },
            async {
                let mut link = fault::faulty(absorber_side, FaultPlan::NONE);
                absorber.gossip(&mut link).await
            },
        );
        assert_honest_gossip(&absorbed);
        match outcome {
            // The retiree believes its party was delivered; if the absorber
            // failed too, delivery is unconfirmed on both sides and the
            // region may be in limbo.
            Retire::Retired => {
                let transfer = if absorbed.is_err() {
                    possible_losses += 1;
                    Transfer::Lost
                } else {
                    Transfer::Committed
                };
                transfers.push((op.retiree, op.absorber, transfer));
            }
            // The party never crossed the wire: the retiree survives whole.
            Retire::Recovered { peer, error } => {
                assert_honest_error(&error);
                slots[op.retiree] = Some(peer);
                transfers.push((op.retiree, op.absorber, Transfer::Recovered));
            }
            // In flight when the wire died. If the absorber committed
            // cleanly it holds the party (no loss); otherwise it is gone.
            Retire::Uncertain { error } => {
                assert_honest_error(&error);
                let transfer = if absorbed.is_err() {
                    possible_losses += 1;
                    Transfer::Lost
                } else {
                    Transfer::Committed
                };
                transfers.push((op.retiree, op.absorber, transfer));
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

    // Deduplicate the redaction ledger by version: racing redactors of one
    // message are one redaction of it, retained if *any* logger's final
    // content reached the surviving fleet. A version names exactly one
    // message, so colliding logs always agree on the value.
    let lost_founders = lost_custody(plan.n_peers, &transfers);
    let mut by_version: BTreeMap<Vec<u8>, Redaction> = BTreeMap::new();
    for (founder, log) in redaction_logs.iter().enumerate() {
        let retained = !lost_founders.contains(&founder);
        for (version, value) in log {
            let entry = by_version.entry(version_key(version)).or_insert(Redaction {
                version: version.clone(),
                value: *value,
                retained: false,
            });
            assert_eq!(
                entry.value, *value,
                "one version logged with two values: a version names exactly \
                 one message"
            );
            entry.retained |= retained;
        }
    }

    SimOutcome {
        peers: slots.into_iter().flatten().map(Peer::into_rumors).collect(),
        possible_losses,
        inserted,
        redactions: by_version.into_values().collect(),
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
    let fingerprint = |p: &Rumors<u64>| {
        let snapshot = p.snapshot();
        (snapshot.hash(), snapshot.latest().clone())
    };
    let max_rounds = MAX_QUIESCE_ROUNDS_PER_PEER * n;
    for _ in 0..max_rounds {
        // Identical fingerprints are the fixed point itself: peers with
        // equal content and version exchange nothing, so no confirming
        // mesh round is owed.
        let first: ([u8; rumors::MERKLE_HASH_LEN], Version) = fingerprint(&peers[0]);
        if peers[1..].iter().all(|p| fingerprint(p) == first) {
            return;
        }
        for i in 0..n {
            for j in (i + 1)..n {
                wire_gossip_async(&peers[i], &peers[j]).await;
            }
        }
    }
    panic!("heal phase did not converge within {max_rounds} rounds for {n} peers");
}

/// One readout per survivor, in fleet order: the identity → value lens
/// (keyed by [`version_key`]) every converged-fleet assertion consumes.
///
/// Compute once after the heal and thread through [`assert_converged`],
/// [`assert_deletion_honored`], and [`assert_value_oracle`].
pub fn survivor_readouts(peers: &[Rumors<u64>]) -> Vec<BTreeMap<Vec<u8>, u64>> {
    peers.iter().map(|p| readout(&p.snapshot())).collect()
}

/// After healing, every survivor holds identical live content: equal
/// identity → value readouts, equal observable hashes, equal causal
/// versions.
///
/// `readouts` is the fleet's [`survivor_readouts`], indexed like `peers`.
pub fn assert_converged(peers: &[Rumors<u64>], readouts: &[BTreeMap<Vec<u8>, u64>]) {
    assert_eq!(peers.len(), readouts.len(), "one readout per survivor");
    let Some(first) = peers.first() else { return };
    let snapshot = first.snapshot();
    let expected = (&readouts[0], snapshot.hash(), snapshot.latest().clone());
    for (i, peer) in peers.iter().enumerate().skip(1) {
        let snapshot = peer.snapshot();
        let actual = (&readouts[i], snapshot.hash(), snapshot.latest().clone());
        assert_eq!(
            actual, expected,
            "peer {i} diverged from peer 0 after the heal phase"
        );
    }
}

/// Asserts deletion honoring against the execution-time redaction log: no
/// retained redaction's message is live at any survivor.
///
/// Unconditional over every retained redaction — and every redaction is
/// retained when [`SimOutcome::possible_losses`] is zero. A non-retained
/// redaction (its every logger dropped in a retire loss arm) may honestly
/// never have reached the survivors, so it is exempt: asserting it would
/// fail runs in which the protocol did nothing wrong.
pub fn assert_deletion_honored(readouts: &[BTreeMap<Vec<u8>, u64>], redactions: &[Redaction]) {
    for redaction in redactions.iter().filter(|r| r.retained) {
        for (i, live) in readouts.iter().enumerate() {
            assert!(
                !live.contains_key(redaction.version.as_bytes()),
                "deletion honoring violated: version {:?} (value {}) was \
                 redacted during the run, the redaction is retained in the \
                 surviving fleet's history, and yet the message is live at \
                 survivor {i}",
                redaction.version,
                redaction.value,
            );
        }
    }
}

/// Asserts the converged value multiset equals the ledger: every survivor's
/// live values are exactly the plan's inserts minus one instance per
/// redacted message.
///
/// Gated on `possible_losses == 0` (a nonzero count returns without
/// checking): zero possible losses covers message content across faulted
/// retires, not only id-regions — every retire arm either leaves the
/// retiree whole, confirms a committed absorber session (which, per
/// [`Peer::retire`]'s contract, reconciles content exactly as gossip
/// would and so holds the union including the retiree's unique messages),
/// or increments the counter; and a faulted bootstrap risks only identity
/// space, because a newborn holds no unique content. Under that premise a
/// loss-free run conserves every insert and propagates every redaction,
/// so the multiset equality is exact.
pub fn assert_value_oracle(
    readouts: &[BTreeMap<Vec<u8>, u64>],
    possible_losses: usize,
    inserted: &[u64],
    redactions: &[Redaction],
) {
    if possible_losses != 0 {
        return;
    }
    let mut expected: BTreeMap<u64, usize> = BTreeMap::new();
    for &value in inserted {
        *expected.entry(value).or_insert(0) += 1;
    }
    for redaction in redactions {
        // Loss-free runs retain every redaction; each removes exactly one
        // instance of its message's value. A redacted value absent from the
        // insert ledger is a harness accounting bug, not a protocol bug.
        match expected.get_mut(&redaction.value) {
            Some(count) if *count > 0 => *count -= 1,
            _ => panic!(
                "value-ledger accounting bug: redacted version {:?} carried \
                 value {}, which the insert ledger does not hold",
                redaction.version, redaction.value,
            ),
        }
    }
    expected.retain(|_, count| *count > 0);
    for (i, live) in readouts.iter().enumerate() {
        let mut actual: BTreeMap<u64, usize> = BTreeMap::new();
        for value in live.values() {
            *actual.entry(*value).or_insert(0) += 1;
        }
        assert_eq!(
            actual, expected,
            "silent divergence from the value ledger: survivor {i}'s \
             converged multiset differs from inserts minus redactions"
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
