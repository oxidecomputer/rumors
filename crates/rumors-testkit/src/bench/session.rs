//! Deterministic replica pairs, and the runner that reconciles them over a
//! [`Link`] and reads what each side's ledger admitted.
//!
//! Every pair forks one network seeded from a fixed ChaCha stream, so its
//! leaves, and with them the tree's shape, are the same on every run and
//! every machine. Every message is a string sized so that its CBOR
//! encoding, head and characters, takes the shape's
//! [`message_bytes`](crate::bench::session::Shape::message_bytes): 100
//! bytes unless the shape names another size.
//!
//! A session's ledgers report every change through the process's ledger
//! hook ([`observe_ledgers`]), and every unit they admit through its
//! admission hook ([`observe_admissions`]). From those reports a
//! [`LedgerReading`] keeps, beside its totals, three records of how the
//! session spent its budget over time: the budget-counted bytes at every
//! moment ([`LedgerReading::profile`]), every wait for admission
//! ([`LedgerReading::waits`]), and what each place held for each depth
//! ([`LedgerReading::holders`]). The link model polls both peers
//! on the thread that drives the session, so the runner keeps each
//! thread's readings apart and attributes them to the session that thread
//! runs, and reads only while one of its sessions runs: a session a thread
//! runs some other way reports through the same hooks, and the runner
//! ignores it. The runner installs both hooks, so a process that runs
//! sessions through it installs no other.
//!
//! [`Link`]: crate::bench::link::Link
//! [`LedgerReading`]: crate::bench::session::LedgerReading
//! [`LedgerReading::profile`]: crate::bench::session::LedgerReading::profile
//! [`LedgerReading::waits`]: crate::bench::session::LedgerReading::waits
//! [`LedgerReading::holders`]: crate::bench::session::LedgerReading::holders
//! [`observe_ledgers`]: rumors::testing::observe_ledgers
//! [`observe_admissions`]: rumors::testing::observe_admissions

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::ops::Range;
use std::pin::pin;
use std::thread;

use rumors::Rumors;
use rumors::testing::{
    Admission, ChargeUnit, Holding, LedgerChange, PLACES, SharePeak, floor_allowance,
    observe_admissions, observe_ledgers,
};

use crate::bench::link::{Link, LinkConfig, Report, SEGMENT_BYTES, Side, drive};
use crate::common::seed::NetworkSeed;
use crate::common::{
    budget::BudgetChoice,
    wire::{block_on, bootstrap_fork_with_budget_async},
};

/// The seed of every fixture's network.
const NETWORK_SEED: NetworkSeed = NetworkSeed::new(7);

/// A LAN: 1 Gb/s with a 1 ms round trip, a 125 kB bandwidth-delay product.
pub const LAN_BDP: u64 = 125_000;

/// A metropolitan link: 1 Gb/s with a 10 ms round trip, a 1.25 MB
/// bandwidth-delay product.
pub const METRO_BDP: u64 = 1_250_000;

/// A long-haul link: 1 Gb/s with a 100 ms round trip, a 12.5 MB
/// bandwidth-delay product.
pub const LONG_HAUL_BDP: u64 = 12_500_000;

/// The scopes a session can admit units about, the root's depth 0 through
/// the leaves' 32.
pub const DEPTHS: usize = 33;

/// The encoded size of every fixture message unless a shape names
/// another: 100 bytes.
pub const MESSAGE_BYTES: usize = 100;

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

/// The messages a pair shares, those each side inserted that the other
/// has never seen, those each side redacted that the other still holds,
/// and the bytes each message encodes in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Shape {
    /// Messages both sides hold.
    pub shared: u64,
    /// Messages only [`Side::A`] holds, which [`Side::B`] has never seen.
    pub a: u64,
    /// Messages only [`Side::B`] holds, which [`Side::A`] has never seen.
    pub b: u64,
    /// Messages both sides received, which [`Side::A`] has since redacted
    /// and [`Side::B`] still holds: its version covers each, and its tree
    /// holds none.
    pub redacted_a: u64,
    /// Messages both sides received, which [`Side::B`] has since redacted
    /// and [`Side::A`] still holds.
    pub redacted_b: u64,
    /// The bytes each message's CBOR encoding takes, head included.
    pub message_bytes: usize,
    /// The draw of the pair's tree: messages the network sends and redacts
    /// before any other, so that every later message's version, and with
    /// it its leaf's path, differs from the draws before it.
    ///
    /// A leaf's path is its version's hash keyed by the network's secret,
    /// and every fixture's network is seeded from `NETWORK_SEED`, so the
    /// counts alone fix a pair's tree; the draw changes the tree and
    /// nothing else, since a redaction leaves no trace in either tree.
    pub draw: u64,
}

impl Shape {
    /// Two equal, disjoint sets of `messages` each: every message is
    /// disputed in both directions.
    pub const fn symmetric(messages: u64) -> Self {
        Self {
            shared: 0,
            a: messages,
            b: messages,
            redacted_a: 0,
            redacted_b: 0,
            message_bytes: MESSAGE_BYTES,
            draw: 0,
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
            redacted_a: 0,
            redacted_b: 0,
            message_bytes: MESSAGE_BYTES,
            draw: 0,
        }
    }

    /// `messages` on [`Side::A`] and none on [`Side::B`]: a catch-up, in
    /// which every message crosses one way.
    pub const fn catch_up(messages: u64) -> Self {
        Self {
            shared: 0,
            a: messages,
            b: 0,
            redacted_a: 0,
            redacted_b: 0,
            message_bytes: MESSAGE_BYTES,
            draw: 0,
        }
    }

    /// This shape with `a` more messages both sides received that
    /// [`Side::A`] has redacted, and `b` that [`Side::B`] has.
    pub const fn with_redactions(self, a: u64, b: u64) -> Self {
        Self {
            redacted_a: a,
            redacted_b: b,
            ..self
        }
    }

    /// This shape's draw number `draw`: another tree for the same counts
    /// ([`Shape::draw`]).
    pub const fn with_draw(self, draw: u64) -> Self {
        Self { draw, ..self }
    }

    /// This shape with every message encoding in `bytes` bytes.
    pub const fn with_message_bytes(self, bytes: usize) -> Self {
        Self {
            message_bytes: bytes,
            ..self
        }
    }
}

/// The fixture message numbered `index` held by `holder`: the holder's
/// letter and the index, zero-padded so the string's CBOR encoding, head
/// and characters, takes `bytes` bytes.
///
/// # Panics
///
/// If no string of `bytes` encoded bytes holds the letter and the index:
/// a fixture whose messages are too small to number.
fn message(holder: char, index: u64, bytes: usize) -> String {
    let width = text_chars(bytes)
        .and_then(|chars| chars.checked_sub(1))
        .unwrap_or_else(|| panic!("no CBOR text string encodes in {bytes} bytes"));
    let digits = index.to_string();
    let zeros = width.checked_sub(digits.len()).unwrap_or_else(|| {
        panic!("a message of {bytes} encoded bytes is too small to number message {index}")
    });
    // Built rather than formatted: a formatting width is at most 65,535.
    let mut message = String::with_capacity(width + 1);
    message.push(holder);
    message.extend(std::iter::repeat_n('0', zeros));
    message.push_str(&digits);
    message
}

/// The characters of the ASCII text string whose CBOR encoding takes
/// `bytes` bytes, if one does.
///
/// A text string's head is one byte for fewer than 24 characters, two for
/// fewer than 256, three for fewer than 65,536, and five below 2^32; no
/// string encodes in 25 bytes, since 24 characters need a two-byte head.
fn text_chars(bytes: usize) -> Option<usize> {
    [
        (1, 0..24),
        (2, 24..256),
        (3, 256..65_536),
        (5, 65_536..1 << 32),
    ]
    .into_iter()
    .find_map(|(head, chars)| bytes.checked_sub(head).filter(|n| chars.contains(n)))
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
/// The public setter raises this to
/// [`MIN_SYNC_MEMORY_BUDGET`](rumors::MIN_SYNC_MEMORY_BUDGET) when `bytes`
/// is below the largest grant. A suite that states its budgets by what the
/// ledgers admit, such as fractions of `Λ` or of the no-cost point, and
/// must reach below that, sets the ledger budget directly instead
/// ([`BudgetChoice::Ledger`]).
pub fn admitting(bytes: usize) -> usize {
    floor_allowance().saturating_add(bytes)
}

/// Builds a pair of the given shape, both peers on a memory budget of
/// `budget` bytes, set through the public setter ([`pair_at`]).
pub fn pair(shape: Shape, budget: usize) -> (Rumors<String>, Rumors<String>) {
    pair_at(shape, BudgetChoice::Bytes(budget))
}

/// Builds a pair of the given shape, both peers on the budget `budget`
/// chooses.
///
/// The shared messages, and those either side goes on to redact, are sent
/// before either side joins, so both receive them while joining; each side
/// then redacts its own, numbered after the shared ones.
pub fn pair_at(shape: Shape, budget: BudgetChoice) -> (Rumors<String>, Rumors<String>) {
    block_on(async {
        let root = NETWORK_SEED.peer::<String>().into_rumors();
        if shape.draw > 0 {
            root.send_all((0..shape.draw).map(|index| message('d', index, shape.message_bytes)))
                .expect("fixture messages encode");
            let drawn: Vec<_> = root
                .snapshot()
                .iter()
                .map(|(version, _)| version.clone())
                .collect();
            root.redact_all(&drawn);
        }
        let received = shape.shared + shape.redacted_a + shape.redacted_b;
        root.send_all((0..received).map(|index| message('s', index, shape.message_bytes)))
            .expect("fixture messages encode");
        let a = bootstrap_fork_with_budget_async(&root, budget).await;
        let b = bootstrap_fork_with_budget_async(&root, budget).await;
        let redacted_by_a = shape.shared..shape.shared + shape.redacted_a;
        let redacted_by_b = redacted_by_a.end..received;
        redact(&a, redacted_by_b.clone(), redacted_by_a.clone(), shape);
        redact(&b, redacted_by_a, redacted_by_b, shape);
        a.send_all((0..shape.a).map(|index| message(letter(Side::A), index, shape.message_bytes)))
            .expect("fixture messages encode");
        b.send_all((0..shape.b).map(|index| message(letter(Side::B), index, shape.message_bytes)))
            .expect("fixture messages encode");
        (a, b)
    })
}

/// Has `replica` redact the shared messages numbered in `redacted`, after
/// checking that it holds those numbered in `kept`, which its peer
/// redacts.
///
/// # Panics
///
/// If the replica does not hold every message in both ranges, each once.
fn redact(replica: &Rumors<String>, kept: Range<u64>, redacted: Range<u64>, shape: Shape) {
    let snapshot = replica.snapshot();
    let numbered = |range: Range<u64>| -> BTreeSet<String> {
        range
            .map(|index| message('s', index, shape.message_bytes))
            .collect()
    };
    let (kept, redacted) = (numbered(kept), numbered(redacted));
    let versions: Vec<_> = snapshot
        .iter()
        .filter(|(_, message)| redacted.contains(message.as_str()))
        .map(|(version, _)| version.clone())
        .collect();
    assert_eq!(
        versions.len(),
        redacted.len(),
        "each message to redact is held once"
    );
    let held = snapshot
        .iter()
        .filter(|(_, message)| kept.contains(message.as_str()))
        .count();
    assert_eq!(
        held,
        kept.len(),
        "each message the peer redacts is held once"
    );
    replica.redact_all(&versions);
}

/// One session to run: the pair's shape, both peers' budget, and the link.
#[derive(Clone, Copy, Debug)]
pub struct Case {
    /// The pair's shape.
    pub shape: Shape,
    /// Both peers' budget: a memory budget through the public setter, or
    /// a ledger budget through the test path, which reaches below the
    /// public minimum.
    pub budget: BudgetChoice,
    /// The link between them.
    pub link: LinkConfig,
}

/// The grants one ledger made to one kind of unit at one depth.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Grants {
    /// Units granted.
    pub units: u64,
    /// Bytes granted, over those units.
    pub bytes: u64,
    /// Units that waited for admission.
    pub waited: u64,
    /// Units admitted through their depth's floor slot.
    pub floor: u64,
    /// Bytes the grants returned unused once their units were built.
    pub returned: u64,
}

impl Grants {
    /// Counts one admission, or one grant's return of what its unit did
    /// not use.
    fn add(&mut self, admission: &Admission) {
        if admission.leftover {
            self.returned += admission.bytes as u64;
            return;
        }
        self.units += 1;
        self.bytes += admission.bytes as u64;
        self.waited += u64::from(admission.waited);
        self.floor += u64::from(admission.floor);
    }

    /// Bytes the units kept out of their grants: what they built.
    pub fn kept(&self) -> u64 {
        self.bytes - self.returned
    }
}

/// One admission, dated by the link's clock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DatedAdmission {
    /// The link time of the admission, in byte-times.
    pub at: u64,
    /// The depth of the scope the unit concerns.
    pub depth: usize,
    /// Whether the unit is a question rather than an answer's own items.
    pub question: bool,
    /// Whether it took its depth's floor slot rather than budget.
    pub floor: bool,
    /// Whether it waited for admission.
    pub waited: bool,
    /// Its grant's bytes.
    pub bytes: usize,
}

/// One unit's wait for admission, dated by the link's clock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Wait {
    /// When the ledger queued the unit and could not admit it.
    pub queued: u64,
    /// When the unit's task took its grant.
    pub admitted: u64,
    /// Whether the unit is a question rather than an answer's own items.
    pub question: bool,
    /// The depth of the scope the unit concerns.
    pub depth: usize,
    /// Its grant's bytes.
    pub bytes: usize,
    /// Whether it was admitted through its depth's floor slot.
    pub floor: bool,
}

/// One step of a ledger's profile: from `at` until the next step's time,
/// the ledger held `bytes` outside its floor slots.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Step {
    /// The link time the step starts, in byte-times.
    pub at: u64,
    /// The bytes held outside floor slots from then on.
    pub bytes: usize,
}

/// One holder of a ledger's bytes: one place's charges for units of one
/// kind at one depth, through floor slots or against the budget.
///
/// Every charge belongs to one holder at a time, and moves between holders
/// only when the session moves its bytes from one place to another, so the
/// holders partition what the ledger holds: those not through floor slots
/// add up, at every moment, to the budget-counted bytes and the bytes floor
/// slots carry for charges admitted against the budget
/// ([`LedgerReading::carries`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Holder {
    /// The place.
    pub place: Holding,
    /// The unit kind and depth, if admission granted the charges.
    pub unit: Option<ChargeUnit>,
    /// Whether the charges were built out of floor grants, which the
    /// budget does not count.
    pub through_floor: bool,
}

/// What one holder held over a session.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Held {
    /// The bytes it held, integrated over the link's clock.
    pub area: u128,
    /// The bytes it held while at least one unit waited for its ledger's
    /// admission, integrated over the link's clock: what kept the budget
    /// full while admission held work back.
    pub area_waiting: u128,
    /// The bytes that entered it, charged to it or moved into it from
    /// another place: with `area`, the mean time a byte stayed
    /// (`area / inflow`, by Little's law).
    pub inflow: u64,
}

/// The invitation share check that weighed a ledger's share peak, dated by
/// the link's clock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DatedCheck {
    /// The link time of the first ledger change that reported the check,
    /// in byte-times.
    ///
    /// A check that admits its question is reported by its grant's own
    /// charge, at the moment it was made, so in a session with no budget,
    /// where every check admits, this is the check's own time. A check that
    /// refuses its question is reported by the ledger's next change.
    pub at: u64,
    /// The check's terms.
    pub check: SharePeak,
}

/// A freed floor slot beginning to carry the bytes its kind and depth held
/// against the budget, dated by the link's clock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Carry {
    /// The link time it began, in byte-times.
    pub at: u64,
    /// The slot's kind and depth.
    pub unit: ChargeUnit,
    /// The bytes it carried.
    pub bytes: usize,
}

/// What one side's ledger held and admitted over a session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LedgerReading {
    /// The side whose session first reported the ledger, if it reported
    /// while the runner polled one side's session: every ledger [`run`]
    /// reads.
    pub side: Option<Side>,
    /// The most bytes it held outside its floor slots: the part its budget
    /// counts.
    pub budget_peak: usize,
    /// The link time it first held `budget_peak`, in byte-times.
    pub budget_peak_at: u64,
    /// What each holder outside floor slots held at that moment, every
    /// holder that held anything, in holder order: they add up to
    /// `budget_peak` and `budget_peak_carried`.
    pub budget_peak_holders: Vec<(Holder, usize)>,
    /// The bytes its floor slots carried for charges admitted against the
    /// budget at that moment.
    pub budget_peak_carried: usize,
    /// The most bytes it held in all.
    pub held_peak: usize,
    /// The most bytes its floor slots held at once, what they carried for
    /// charges admitted against the budget included.
    pub floor_peak: usize,
    /// The sum over floor slots of the largest grant each admitted: the
    /// `Σ F` its bound adds to its budget.
    pub floor_bound: usize,
    /// The largest sum any of its invitation share checks weighed: the
    /// share peak.
    pub share_peak: usize,
    /// The check that weighed the share peak, if it made any.
    pub share_peak_check: Option<DatedCheck>,
    /// Units that waited for admission, as the ledger counts them.
    pub waited: u64,
    /// The bytes it held outside its floor slots, integrated over the
    /// link's clock: byte-times times bytes.
    pub budget_area: u128,
    /// The bytes it charged over the session: the sum of every rise in
    /// what it holds.
    pub charged: u64,
    /// The bytes its items needed beyond their grants, over the session.
    pub short: u64,
    /// The bytes it held for each place, floor slots included, integrated
    /// over the link's clock, in
    /// [`Holding::ALL`](rumors::testing::Holding::ALL) order: which places
    /// held the budget, and for how long.
    pub place_area: [u128; PLACES],
    /// The bytes charges built out of floor grants held, integrated over
    /// the link's clock.
    pub floor_area: u128,
    /// The bytes its floor slots carried for charges admitted against the
    /// budget, integrated over the link's clock: with `budget_area`, what
    /// the holders outside floor slots held.
    pub carried_area: u128,
    /// Each time a freed floor slot began to carry bytes of its kind and
    /// depth held against the budget, in order.
    pub carries: Vec<Carry>,
    /// Its grants to questions, by the depth of the scope each asks about.
    pub questions: [Grants; DEPTHS],
    /// Its grants to answers' own items, by the depth of the scope each
    /// answers.
    pub answers: [Grants; DEPTHS],
    /// Its admissions, in order: when each unit started, which shows, for
    /// one, whether the root's answer opens its subtrees at once or one
    /// after another.
    pub dated: Vec<DatedAdmission>,
    /// The bytes it held outside its floor slots over the session, as
    /// steps in time order, one per link time at which they changed.
    ///
    /// The value a step records is the one in force once every change at
    /// its time is made, which is all an integral over time can see. Before
    /// the first step it held nothing.
    pub profile: Vec<Step>,
    /// Every unit that waited for admission, in the order each was
    /// admitted.
    pub waits: Vec<Wait>,
    /// The link time during which at least one unit waited for its
    /// admission, in byte-times.
    pub waiting_time: u64,
    /// What each holder of its bytes held over the session.
    pub holders: BTreeMap<Holder, Held>,
}

impl Default for LedgerReading {
    /// Returns a reading of a ledger that held and admitted nothing.
    fn default() -> Self {
        Self {
            side: None,
            budget_peak: 0,
            budget_peak_at: 0,
            budget_peak_holders: Vec::new(),
            budget_peak_carried: 0,
            held_peak: 0,
            floor_peak: 0,
            floor_bound: 0,
            share_peak: 0,
            share_peak_check: None,
            waited: 0,
            budget_area: 0,
            charged: 0,
            short: 0,
            place_area: [0; PLACES],
            floor_area: 0,
            carried_area: 0,
            carries: Vec::new(),
            questions: [Grants::default(); DEPTHS],
            answers: [Grants::default(); DEPTHS],
            dated: Vec::new(),
            profile: Vec::new(),
            waits: Vec::new(),
            waiting_time: 0,
            holders: BTreeMap::new(),
        }
    }
}

impl LedgerReading {
    /// Its question grants over every depth.
    pub fn all_questions(&self) -> Grants {
        sum(&self.questions)
    }

    /// Its answer grants over every depth.
    pub fn all_answers(&self) -> Grants {
        sum(&self.answers)
    }
}

/// The grants of every depth in `grants`, added.
fn sum(grants: &[Grants]) -> Grants {
    grants
        .iter()
        .fold(Grants::default(), |total, depth| Grants {
            units: total.units + depth.units,
            bytes: total.bytes + depth.bytes,
            waited: total.waited + depth.waited,
            floor: total.floor + depth.floor,
            returned: total.returned + depth.returned,
        })
}

/// What one session's ledgers admitted.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Admitted {
    /// Each ledger that reported, in the order of its first report.
    pub ledgers: Vec<LedgerReading>,
    /// The most bytes the ledgers held together at once, floor slots
    /// included.
    pub joint_held_peak: usize,
}

impl Admitted {
    /// The most bytes any one ledger held outside its floor slots: the part
    /// its budget counts.
    pub fn budget_peak(&self) -> usize {
        self.ledgers
            .iter()
            .map(|ledger| ledger.budget_peak)
            .max()
            .unwrap_or(0)
    }

    /// The largest sum any ledger's invitation share checks weighed: the
    /// share peak ([`LedgerReading::share_peak`]).
    pub fn share_peak(&self) -> usize {
        self.ledgers
            .iter()
            .map(|ledger| ledger.share_peak)
            .max()
            .unwrap_or(0)
    }

    /// The session's no-cost point, read from an unlimited session: the
    /// least budget at which it runs as the unlimited session does.
    ///
    /// That is the larger of `Λ`, every ledger's peak outside its floor
    /// slots, and the [`share_peak`](Self::share_peak).
    ///
    /// Below `Λ` some admission the unlimited session made would not fit,
    /// and below the share peak some question it asked would not fit beside
    /// its depth's claims, so some unit waits. At or above both, every check
    /// the unlimited session made passes, by induction along its schedule.
    pub fn no_cost_budget(&self) -> usize {
        self.budget_peak().max(self.share_peak())
    }

    /// Units that waited for admission, over every ledger.
    pub fn waited(&self) -> u64 {
        self.ledgers.iter().map(|ledger| ledger.waited).sum()
    }
}

/// One ledger's reading while its session runs: the reading so far, and
/// what integrating its budget-counted bytes over time needs.
#[derive(Default)]
struct Reading {
    /// The order of the ledger's first report among this thread's ledgers.
    order: usize,
    /// The reading so far.
    reading: LedgerReading,
    /// The bytes held outside floor slots after the last change, and the
    /// link time of that change.
    last: (usize, u64),
    /// The bytes held in all after the last change.
    held: usize,
    /// The bytes held for each place after the last change, in
    /// [`Holding::ALL`](rumors::testing::Holding::ALL) order.
    places: [usize; PLACES],
    /// The bytes charges built out of floor grants held after the last
    /// change.
    floor: usize,
    /// The bytes floor slots carried after the last change.
    carried: usize,
    /// Each holder's bytes now, the link time they last changed, and the
    /// waiting clock's reading then.
    holders: BTreeMap<Holder, (usize, u64, u64)>,
    /// The units waiting for admission, by order, with the time each began.
    pending: BTreeMap<u64, (u64, Admission)>,
    /// The link time at which `pending` last changed.
    pending_since: u64,
}

impl Reading {
    /// The waiting clock at `now`: the link time so far during which at
    /// least one unit waited, which grows at one byte-time per byte-time
    /// while any does and stands still otherwise.
    fn waiting_clock(&self, now: u64) -> u64 {
        let current = if self.pending.is_empty() {
            0
        } else {
            now - self.pending_since
        };
        self.reading.waiting_time + current
    }

    /// Bring the waiting clock up to `now`, before `pending` changes.
    fn settle_waiting(&mut self, now: u64) {
        self.reading.waiting_time = self.waiting_clock(now);
        self.pending_since = now;
    }

    /// Integrate `holder`'s bytes up to `now`, then add `delta` to them.
    fn hold(&mut self, holder: Holder, now: u64, delta: isize) {
        let clock = self.waiting_clock(now);
        let (bytes, since, clock_then) = self.holders.entry(holder).or_insert((0, now, clock));
        let held = self.reading.holders.entry(holder).or_default();
        held.area += *bytes as u128 * u128::from(now - *since);
        held.area_waiting += *bytes as u128 * u128::from(clock - *clock_then);
        if let Ok(entered) = u64::try_from(delta) {
            held.inflow += entered;
        }
        *bytes = bytes
            .checked_add_signed(delta)
            .expect("a holder releases no more than it holds");
        *since = now;
        *clock_then = clock;
    }
}

/// One thread's readings: the link whose clock dates them, and each ledger
/// by its identity.
#[derive(Default)]
struct Readings {
    /// The link the thread's session runs over.
    link: Option<Link>,
    /// Each ledger's reading, by its identity.
    ledgers: BTreeMap<usize, Reading>,
    /// The bytes every ledger holds together.
    joint_held: usize,
    /// The most `joint_held` has been.
    joint_held_peak: usize,
    /// Whether a root answer has returned what its grant left over, which
    /// it does once it has admitted its last question.
    root_answered: bool,
    /// The side whose session the runner is polling, if it is polling one.
    polling: Option<Side>,
}

impl Readings {
    /// The reading of the ledger identified as `ledger`, begun if this is
    /// its first report, as the ledger of the side being polled.
    fn ledger(&mut self, ledger: usize) -> &mut Reading {
        let order = self.ledgers.len();
        let side = self.polling;
        self.ledgers.entry(ledger).or_insert_with(|| Reading {
            order,
            reading: LedgerReading {
                side,
                ..LedgerReading::default()
            },
            ..Reading::default()
        })
    }
}

// Clippy's `missing_const_for_thread_local` can reject const-block
// initializers on targets that lower `thread_local!` through fallback TLS.
// The allow keeps `-D warnings` clean on those targets.
thread_local! {
    /// The readings of the session this thread runs.
    #[allow(clippy::missing_const_for_thread_local)]
    static READINGS: RefCell<Readings> = RefCell::new(Readings::default());
}

/// Records one ledger change against the reporting thread's session, if
/// the runner is running one on this thread.
///
/// A session this thread runs some other way, outside [`run`], reports
/// through the same hook; its changes are not the runner's to read, and a
/// stale reading of a ledger whose address it reuses would date them
/// against the wrong clock.
fn record_change(change: LedgerChange) {
    READINGS.with(|readings| {
        let mut readings = readings.borrow_mut();
        let Some(now) = readings.link.as_ref().map(Link::now) else {
            return;
        };
        let ledger = readings.ledger(change.ledger);
        let moved =
            isize::try_from(change.bytes).expect("a change moves fewer than isize::MAX bytes");
        for (place, delta) in [(change.from, -moved), (change.to, moved)] {
            if let Some(place) = place {
                let holder = Holder {
                    place,
                    unit: change.unit,
                    through_floor: change.through_floor,
                };
                ledger.hold(holder, now, delta);
            }
        }
        let Reading {
            reading,
            last,
            held,
            places,
            floor,
            carried,
            holders,
            ..
        } = ledger;
        let before_held = std::mem::replace(held, change.held);
        let normal = change.held - change.floor - change.carried;
        let (before, since) = *last;
        let elapsed = u128::from(now - since);
        reading.budget_area += before as u128 * elapsed;
        for (area, bytes) in reading.place_area.iter_mut().zip(*places) {
            *area += bytes as u128 * elapsed;
        }
        reading.floor_area += *floor as u128 * elapsed;
        reading.carried_area += *carried as u128 * elapsed;
        if change.carried > *carried
            && let Some(unit) = change.unit
        {
            reading.carries.push(Carry {
                at: now,
                unit,
                bytes: change.carried - *carried,
            });
        }
        *places = change.places;
        *floor = change.floor;
        *carried = change.carried;
        *last = (normal, now);
        match reading.profile.last_mut() {
            Some(step) if step.at == now => step.bytes = normal,
            Some(step) if step.bytes == normal => {}
            _ => reading.profile.push(Step {
                at: now,
                bytes: normal,
            }),
        }
        if normal > reading.budget_peak {
            reading.budget_peak = normal;
            reading.budget_peak_at = now;
            reading.budget_peak_carried = change.carried;
            reading.budget_peak_holders = holders
                .iter()
                .filter(|(holder, (bytes, ..))| !holder.through_floor && *bytes > 0)
                .map(|(holder, (bytes, ..))| (*holder, *bytes))
                .collect();
        }
        reading.held_peak = reading.held_peak.max(change.held);
        reading.floor_peak = reading.floor_peak.max(change.floor + change.carried);
        reading.floor_bound = reading.floor_bound.max(change.floor_bound);
        if change.share_peak.sum > reading.share_peak {
            reading.share_peak = change.share_peak.sum;
            reading.share_peak_check = Some(DatedCheck {
                at: now,
                check: change.share_peak,
            });
        }
        reading.waited = reading.waited.max(change.waited);
        // The runner observes on one thread, so changes arrive in the order
        // they happened and each rise is exactly one charge.
        reading.charged += change.held.saturating_sub(before_held) as u64;
        reading.short = change.short;
        readings.joint_held = readings.joint_held - before_held + change.held;
        readings.joint_held_peak = readings.joint_held_peak.max(readings.joint_held);
    });
}

/// Records one admission against the reporting thread's session, if the
/// runner is running one on this thread, as [`record_change`] does.
fn record_admission(admission: Admission) {
    READINGS.with(|readings| {
        let mut readings = readings.borrow_mut();
        let Some(now) = readings.link.as_ref().map(Link::now) else {
            return;
        };
        // A part returned before the answer was built is not its end.
        if admission.depth == 0 && !admission.question && admission.leftover && !admission.partial {
            readings.root_answered = true;
        }
        let ledger = readings.ledger(admission.ledger);
        if admission.waiting {
            ledger.settle_waiting(now);
            let order = admission
                .order
                .expect("a unit that waits was queued, so its report names its order");
            ledger.pending.insert(order, (now, admission));
            return;
        }
        if admission.waited {
            ledger.settle_waiting(now);
            if let Some((queued, _)) = admission
                .order
                .and_then(|order| ledger.pending.remove(&order))
            {
                ledger.reading.waits.push(Wait {
                    queued,
                    admitted: now,
                    question: admission.question,
                    depth: admission.depth,
                    bytes: admission.bytes,
                    floor: admission.floor,
                });
            }
        }
        let reading = &mut ledger.reading;
        if !admission.leftover {
            reading.dated.push(DatedAdmission {
                at: now,
                depth: admission.depth,
                question: admission.question,
                floor: admission.floor,
                waited: admission.waited,
                bytes: admission.bytes,
            });
        }
        let kind = if admission.question {
            &mut reading.questions
        } else {
            &mut reading.answers
        };
        kind[admission.depth].add(&admission);
    });
}

/// Clears this thread's readings, so they cover only the session that
/// runs over `link` next.
fn start_reading(link: &Link) {
    assert!(
        observe_ledgers(record_change),
        "the session runner installs the only ledger hook in its process",
    );
    assert!(
        observe_admissions(record_admission),
        "the session runner installs the only admission hook in its process",
    );
    READINGS.with(|readings| {
        *readings.borrow_mut() = Readings {
            link: Some(link.clone()),
            ..Readings::default()
        };
    });
}

/// Returns what the ledgers this thread observed since [`start_reading`]
/// admitted, and stops dating changes by the session's link.
///
/// # Panics
///
/// If a ledger's admissions that waited, counted one by one, differ from
/// the count the ledger itself keeps, or if the bytes its admissions
/// granted, with what its items needed beyond their grants, differ from
/// the rises in what it holds: two computations of one quantity must
/// agree.
fn admitted() -> Admitted {
    READINGS.with(|readings| {
        let mut readings = readings.borrow_mut();
        let end = readings.link.take().map_or(0, |link| link.now());
        for ledger in readings.ledgers.values_mut() {
            // Every holder's area runs to the session's end, and the
            // waiting clock with it.
            let holders: Vec<Holder> = ledger.holders.keys().copied().collect();
            for holder in holders {
                ledger.hold(holder, end, 0);
            }
            ledger.settle_waiting(end);
        }
        let mut ledgers: Vec<_> = readings.ledgers.values().collect();
        ledgers.sort_by_key(|reading| reading.order);
        let ledgers: Vec<_> = ledgers
            .into_iter()
            .map(|reading| reading.reading.clone())
            .collect();
        for ledger in &ledgers {
            let waited = ledger.all_questions().waited + ledger.all_answers().waited;
            assert_eq!(
                waited, ledger.waited,
                "the admissions that waited must number the waits the ledger counts",
            );
            // A ledger's held bytes rise only when admission grants a unit
            // or an item needs more than its grant holds, so the grants,
            // counted one by one, and the shortfall account for every rise.
            let granted = ledger.all_questions().bytes + ledger.all_answers().bytes;
            assert_eq!(
                granted + ledger.short,
                ledger.charged,
                "the bytes admission granted, with any shortfall, must be the rises in what the \
                 ledger holds",
            );
        }
        Admitted {
            ledgers,
            joint_held_peak: readings.joint_held_peak,
        }
    })
}

/// One completed session: what its ledgers admitted and what the link
/// carried.
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
    run_within(case, |session| session()).0
}

/// `session`, as `side`'s session: while the runner polls it, every ledger
/// that first reports is read as that side's ([`LedgerReading::side`]).
async fn sided<F: Future>(side: Side, session: F) -> F::Output {
    let mut session = pin!(session);
    std::future::poll_fn(|cx| {
        READINGS.with(|readings| readings.borrow_mut().polling = Some(side));
        let poll = session.as_mut().poll(cx);
        READINGS.with(|readings| readings.borrow_mut().polling = None);
        poll
    })
    .await
}

/// Builds the pair `case` names and reconciles it over its link, running
/// the session itself, and nothing of the pair's construction, inside
/// `within`, which returns what it observed.
///
/// `within` must call the closure it is given exactly once, on the calling
/// thread: the session's readings are this thread's.
///
/// # Panics
///
/// As [`run`], and if `within` does not run the session.
pub fn run_within<T>(case: Case, within: impl FnOnce(&mut dyn FnMut()) -> T) -> (Session, T) {
    let (a, b) = pair_at(case.shape, case.budget);
    let link = Link::new(case.link);
    let (mut a_end, mut b_end) = (link.end(Side::A), link.end(Side::B));
    start_reading(&link);
    let mut outcome = None;
    let observed = within(&mut || {
        let (a_out, b_out, report) = drive(
            &link,
            sided(Side::A, a.gossip_once(&mut a_end.read, &mut a_end.write)),
            sided(Side::B, b.gossip_once(&mut b_end.read, &mut b_end.write)),
        );
        let gained = [
            a_out
                .expect("A's session completes")
                .0
                .stats
                .messages_gained,
            b_out
                .expect("B's session completes")
                .0
                .stats
                .messages_gained,
        ];
        outcome = Some((gained, report));
    });
    let (gained, report) = outcome.expect("`within` runs the session");
    let admitted = admitted();
    assert_eq!(
        admitted.ledgers.len(),
        2,
        "each side's session reports through one ledger of its own",
    );
    assert_eq!(gained[0], case.shape.b, "A gains B's messages");
    assert_eq!(gained[1], case.shape.a, "B gains A's messages");
    (
        Session {
            case,
            admitted,
            report,
        },
        observed,
    )
}

/// The grants of the depth-1 questions the responder's root answer asks,
/// in the order it asks them, when the pair `shape` names reconciles at an
/// unlimited budget over a LAN.
///
/// Their total is the budget a session needs to open every top-level
/// subtree at once. They are all admitted in the session's first round
/// trip, so the session stops as soon as the root answer has admitted its
/// last question, which it shows by returning what its grant left over:
/// the count costs the pair's construction and one round trip, not a
/// whole session, and reaches replica sizes a whole session cannot.
pub fn root_question_grants(shape: Shape) -> Vec<usize> {
    let (a, b) = pair(shape, usize::MAX);
    let link = Link::new(LinkConfig::with_bdp(LAN_BDP));
    let (mut a_end, mut b_end) = (link.end(Side::A), link.end(Side::B));
    start_reading(&link);
    let answered = || READINGS.with(|readings| readings.borrow().root_answered);
    let until_answered = |session| async move {
        let session = std::pin::pin!(session);
        let stop = std::pin::pin!(std::future::poll_fn(|_| {
            if answered() {
                std::task::Poll::Ready(())
            } else {
                std::task::Poll::Pending
            }
        }));
        drop(futures::future::select(session, stop).await);
    };
    drive(
        &link,
        until_answered(a.gossip_once(&mut a_end.read, &mut a_end.write)),
        until_answered(b.gossip_once(&mut b_end.read, &mut b_end.write)),
    );
    admitted()
        .ledgers
        .iter()
        .map(|ledger| {
            ledger
                .dated
                .iter()
                .filter(|admission| admission.question && admission.depth == 1)
                .map(|admission| admission.bytes)
                .collect::<Vec<_>>()
        })
        .max_by_key(Vec::len)
        .unwrap_or_default()
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

#[cfg(test)]
mod tests;
