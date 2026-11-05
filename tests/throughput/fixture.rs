//! Deterministic replica pairs, and the runner that reconciles them over a
//! [`Link`] and reads what each side's ledger admitted.
//!
//! Every pair forks one network seeded from a fixed ChaCha stream, so its
//! leaves, and with them the tree's shape, are the same on every run and
//! every machine. Every message is a 98-character string, which encodes in
//! CBOR as a two-byte head and its bytes, 100 bytes in all.
//!
//! A session's ledgers report every change through the process's ledger
//! hook ([`observe_ledgers`]). The link model polls both peers on the
//! thread that drives the session, so the runner keeps each thread's
//! readings apart and attributes them to the session that thread runs.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::thread;

use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use rumors::testing::{LedgerChange, floor_allowance, observe_ledgers};
use rumors::{Peer, Rumors};
use rumors_testkit::bench::link::{Link, LinkConfig, Report, SEGMENT_BYTES, Side, drive};
use rumors_testkit::common::{
    budget::BudgetChoice,
    wire::{block_on, bootstrap_fork_with_budget_async},
};

/// The ChaCha seed of every fixture's network.
const NETWORK_SEED: u64 = 7;

/// A LAN: 1 Gb/s with a 1 ms round trip, a 125 kB bandwidth-delay product.
pub const LAN_BDP: u64 = 125_000;

/// A long-haul link: 1 Gb/s with a 100 ms round trip, a 12.5 MB
/// bandwidth-delay product.
pub const LONG_HAUL_BDP: u64 = 12_500_000;

/// The model's timing resolution between two sessions of one pair on a
/// link with bandwidth: one segment's serialization per one-way hop of the
/// critical path, which `latency_only` measures.
///
/// A segment arrives whole, so each serialized hop of the critical path can
/// shift by up to one segment's serialization between two sessions that
/// differ only in how their bytes are grouped.
pub fn resolution(latency_only: &Session) -> u64 {
    latency_only.hops() * SEGMENT_BYTES
}

/// The messages a pair shares, and those each side holds that the other
/// lacks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Shape {
    /// Messages both sides hold.
    pub shared: u64,
    /// Messages only [`Side::A`] holds.
    pub a: u64,
    /// Messages only [`Side::B`] holds.
    pub b: u64,
}

impl Shape {
    /// Two equal, disjoint sets of `messages` each: every message is
    /// disputed in both directions.
    pub const fn symmetric(messages: u64) -> Self {
        Self {
            shared: 0,
            a: messages,
            b: messages,
        }
    }

    /// Two sets of `messages` each, of which `shared_percent` percent are
    /// held by both and the rest by one side alone.
    pub const fn overlapping(messages: u64, shared_percent: u64) -> Self {
        let shared = messages * shared_percent / 100;
        Self {
            shared,
            a: messages - shared,
            b: messages - shared,
        }
    }

    /// `messages` on [`Side::A`] and none on [`Side::B`]: a catch-up, in
    /// which every message crosses one way.
    pub const fn catch_up(messages: u64) -> Self {
        Self {
            shared: 0,
            a: messages,
            b: 0,
        }
    }
}

/// The fixture message numbered `index` held by `holder`: the holder's
/// letter and the index, zero-padded to 98 characters.
fn message(holder: char, index: u64) -> String {
    format!("{holder}{index:0>97}")
}

/// The letter of the messages only `side` holds.
fn letter(side: Side) -> char {
    match side {
        Side::A => 'a',
        Side::B => 'b',
    }
}

/// The memory budget whose sessions' ledgers admit work against `bytes`:
/// `bytes` beside the floor allowance every budget sets aside
/// ([`floor_allowance`]).
///
/// A suite that states its budgets by what the ledgers admit, such as
/// fractions of `Λ`, configures peers with this.
pub fn admitting(bytes: usize) -> usize {
    floor_allowance().saturating_add(bytes)
}

/// Builds a pair of the given shape, both peers on a memory budget of
/// `budget` bytes.
///
/// The shared messages are sent before either side joins, so both receive
/// them while joining.
pub fn pair(shape: Shape, budget: usize) -> (Rumors<String>, Rumors<String>) {
    block_on(async {
        let mut rng = ChaChaRng::seed_from_u64(NETWORK_SEED);
        let root = Peer::<String>::seed_rng(&mut rng).into_rumors();
        root.send_all((0..shape.shared).map(|index| message('s', index)))
            .expect("fixture messages encode");
        let a = bootstrap_fork_with_budget_async(&root, BudgetChoice::Bytes(budget)).await;
        let b = bootstrap_fork_with_budget_async(&root, BudgetChoice::Bytes(budget)).await;
        a.send_all((0..shape.a).map(|index| message(letter(Side::A), index)))
            .expect("fixture messages encode");
        b.send_all((0..shape.b).map(|index| message(letter(Side::B), index)))
            .expect("fixture messages encode");
        (a, b)
    })
}

/// One session to run: the pair's shape, both peers' budget, and the link.
#[derive(Clone, Copy, Debug)]
pub struct Case {
    /// The pair's shape.
    pub shape: Shape,
    /// Both peers' memory budget, in bytes.
    pub budget: usize,
    /// The link between them.
    pub link: LinkConfig,
}

/// What one session's ledgers admitted, over both sides.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Admitted {
    /// The ledgers that reported a change.
    pub ledgers: usize,
    /// The most bytes any one ledger held outside its floor slots: the
    /// part its budget counts.
    pub budget_peak: usize,
    /// Units that waited for admission, over both ledgers.
    pub waited: u64,
}

// Clippy's `missing_const_for_thread_local` can reject const-block
// initializers on targets that lower `thread_local!` through fallback TLS.
// The allow keeps `-D warnings` clean on those targets.
thread_local! {
    /// Each ledger this thread's session reports, by its identity: the
    /// most it held outside its floor slots, and the units that waited.
    #[allow(clippy::missing_const_for_thread_local)]
    static LEDGERS: RefCell<BTreeMap<usize, (usize, u64)>> =
        const { RefCell::new(BTreeMap::new()) };
}

/// Records one ledger change against the reporting thread's session.
fn record(change: LedgerChange) {
    LEDGERS.with(|ledgers| {
        let mut ledgers = ledgers.borrow_mut();
        let (peak, waited) = ledgers.entry(change.ledger).or_default();
        *peak = (*peak).max(change.held - change.floor);
        *waited = (*waited).max(change.waited);
    });
}

/// Clears this thread's readings, so they cover only what follows.
fn start_reading() {
    assert!(
        observe_ledgers(record),
        "the throughput suite installs the only ledger hook in its process",
    );
    LEDGERS.with(|ledgers| ledgers.borrow_mut().clear());
}

/// Returns what the ledgers this thread observed since [`start_reading`]
/// admitted.
fn admitted() -> Admitted {
    LEDGERS.with(|ledgers| {
        let ledgers = ledgers.borrow();
        Admitted {
            ledgers: ledgers.len(),
            budget_peak: ledgers.values().map(|&(peak, _)| peak).max().unwrap_or(0),
            waited: ledgers.values().map(|&(_, waited)| waited).sum(),
        }
    })
}

/// One completed session: what each side reported, what its ledgers
/// admitted, and what the link carried.
#[derive(Clone, Debug)]
pub struct Session {
    /// The session that ran.
    pub case: Case,
    /// What the session's ledgers admitted.
    pub admitted: Admitted,
    /// What the link carried, and when.
    pub report: Report,
}

impl Session {
    /// The session's duration, in byte-times.
    pub fn duration(&self) -> u64 {
        self.report.duration()
    }

    /// The bytes the busier direction carried: the session's duration on a
    /// link with no delay and a writer that never pauses.
    pub fn busier_direction_bytes(&self) -> u64 {
        Side::BOTH
            .map(|side| self.report.sent_by(side).bytes())
            .into_iter()
            .max()
            .expect("a link has two directions")
    }

    /// The session's duration in one-way delays.
    ///
    /// # Panics
    ///
    /// If the duration is not a whole number of delays, which on a
    /// latency-only link would mean some event happened between arrivals.
    pub fn hops(&self) -> u64 {
        let delay = self.case.link.one_way_delay();
        assert_eq!(
            self.duration() % delay,
            0,
            "a latency-only session ends on the delay lattice: {} byte-times at delay {delay}",
            self.duration(),
        );
        self.duration() / delay
    }
}

/// Builds the pair `case` names and reconciles it over its link.
///
/// # Panics
///
/// If either side's session fails, or either side gains other than every
/// message its peer holds that it lacks.
pub fn run(case: Case) -> Session {
    let (a, b) = pair(case.shape, case.budget);
    let link = Link::new(case.link);
    let (mut a_end, mut b_end) = (link.end(Side::A), link.end(Side::B));
    start_reading();
    let (a_out, b_out, report) = drive(
        &link,
        a.gossip_once(&mut a_end.read, &mut a_end.write),
        b.gossip_once(&mut b_end.read, &mut b_end.write),
    );
    let admitted = admitted();
    assert_eq!(
        admitted.ledgers, 2,
        "each side's session reports through one ledger of its own",
    );
    let stats = [
        a_out.expect("A's session completes").0.stats,
        b_out.expect("B's session completes").0.stats,
    ];
    assert_eq!(
        stats[0].messages_gained, case.shape.b,
        "A gains B's messages"
    );
    assert_eq!(
        stats[1].messages_gained, case.shape.a,
        "B gains A's messages"
    );
    Session {
        case,
        admitted,
        report,
    }
}

/// Runs every case, each on a thread of its own, returning the sessions in
/// the cases' order.
///
/// Each session runs on one thread against a link of its own, so running
/// them side by side changes no session's report.
pub fn run_all<const N: usize>(cases: [Case; N]) -> [Session; N] {
    thread::scope(|scope| {
        cases
            .map(|case| scope.spawn(move || run(case)))
            .map(|handle| {
                handle
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
    })
}
