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
//! admission hook ([`observe_admissions`]). The link model polls both peers
//! on the thread that drives the session, so the runner keeps each
//! thread's readings apart and attributes them to the session that thread
//! runs, and reads only while one of its sessions runs: a session a thread
//! runs some other way reports through the same hooks, and the runner
//! ignores it. The runner installs both hooks, so a process that runs
//! sessions through it installs no other.
//!
//! [`Link`]: crate::bench::link::Link
//! [`observe_ledgers`]: rumors::testing::observe_ledgers
//! [`observe_admissions`]: rumors::testing::observe_admissions

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::thread;

use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use rumors::testing::{
    Admission, LedgerChange, PLACES, floor_allowance, observe_admissions, observe_ledgers,
};
use rumors::{Peer, Rumors};

use crate::bench::link::{Link, LinkConfig, Report, SEGMENT_BYTES, Side, drive};
use crate::common::{
    budget::BudgetChoice,
    wire::{block_on, bootstrap_fork_with_budget_async},
};

/// The ChaCha seed of every fixture's network.
const NETWORK_SEED: u64 = 7;

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

/// The deepest scope whose admissions a reading dates one by one
/// ([`LedgerReading::dated`]): depth 1, the top-level subtrees, whose
/// questions the root's answer asks and whose answers open them.
pub const DATED_DEPTH: usize = 1;

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

/// The messages a pair shares, those each side holds that the other
/// lacks, and the bytes each message encodes in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Shape {
    /// Messages both sides hold.
    pub shared: u64,
    /// Messages only [`Side::A`] holds.
    pub a: u64,
    /// Messages only [`Side::B`] holds.
    pub b: u64,
    /// The bytes each message's CBOR encoding takes, head included.
    pub message_bytes: usize,
}

impl Shape {
    /// Two equal, disjoint sets of `messages` each: every message is
    /// disputed in both directions.
    pub const fn symmetric(messages: u64) -> Self {
        Self {
            shared: 0,
            a: messages,
            b: messages,
            message_bytes: MESSAGE_BYTES,
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
            message_bytes: MESSAGE_BYTES,
        }
    }

    /// `messages` on [`Side::A`] and none on [`Side::B`]: a catch-up, in
    /// which every message crosses one way.
    pub const fn catch_up(messages: u64) -> Self {
        Self {
            shared: 0,
            a: messages,
            b: 0,
            message_bytes: MESSAGE_BYTES,
        }
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
        root.send_all((0..shape.shared).map(|index| message('s', index, shape.message_bytes)))
            .expect("fixture messages encode");
        let a = bootstrap_fork_with_budget_async(&root, BudgetChoice::Bytes(budget)).await;
        let b = bootstrap_fork_with_budget_async(&root, BudgetChoice::Bytes(budget)).await;
        a.send_all((0..shape.a).map(|index| message(letter(Side::A), index, shape.message_bytes)))
            .expect("fixture messages encode");
        b.send_all((0..shape.b).map(|index| message(letter(Side::B), index, shape.message_bytes)))
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

/// One admission at depth [`DATED_DEPTH`] or shallower, dated by the
/// link's clock.
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

/// What one side's ledger held and admitted over a session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LedgerReading {
    /// The most bytes it held outside its floor slots: the part its budget
    /// counts.
    pub budget_peak: usize,
    /// The most bytes it held in all.
    pub held_peak: usize,
    /// The most bytes its floor slots held at once, what they carried for
    /// charges admitted against the budget included.
    pub floor_peak: usize,
    /// The sum over floor slots of the largest grant each admitted: the
    /// `Σ F` its bound adds to its budget.
    pub floor_bound: usize,
    /// Units that waited for admission, as the ledger counts them.
    pub waited: u64,
    /// The bytes it held outside its floor slots, integrated over the
    /// link's clock: byte-times times bytes.
    pub budget_area: u128,
    /// The bytes it held for each place, floor slots included, integrated
    /// over the link's clock, in
    /// [`Holding::ALL`](rumors::testing::Holding::ALL) order: which places
    /// held the budget, and for how long.
    pub place_area: [u128; PLACES],
    /// The bytes charges built out of floor grants held, integrated over
    /// the link's clock.
    pub floor_area: u128,
    /// The bytes its floor slots carried for charges admitted against the
    /// budget, integrated over the link's clock: with `budget_area` and
    /// `floor_area`, what its places held.
    pub carried_area: u128,
    /// Its grants to questions, by the depth of the scope each asks about.
    pub questions: [Grants; DEPTHS],
    /// Its grants to answers' own items, by the depth of the scope each
    /// answers.
    pub answers: [Grants; DEPTHS],
    /// Its admissions at depth [`DATED_DEPTH`] or shallower, in order:
    /// when each top-level subtree's units started, which shows whether
    /// the root's answer opens its subtrees at once or one after another.
    pub dated: Vec<DatedAdmission>,
}

impl Default for LedgerReading {
    /// Returns a reading of a ledger that held and admitted nothing.
    fn default() -> Self {
        Self {
            budget_peak: 0,
            held_peak: 0,
            floor_peak: 0,
            floor_bound: 0,
            waited: 0,
            budget_area: 0,
            place_area: [0; PLACES],
            floor_area: 0,
            carried_area: 0,
            questions: [Grants::default(); DEPTHS],
            answers: [Grants::default(); DEPTHS],
            dated: Vec::new(),
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
}

impl Readings {
    /// The reading of the ledger identified as `ledger`, begun if this is
    /// its first report.
    fn ledger(&mut self, ledger: usize) -> &mut Reading {
        let order = self.ledgers.len();
        self.ledgers.entry(ledger).or_insert_with(|| Reading {
            order,
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
        let Reading {
            reading,
            last,
            held,
            places,
            floor,
            carried,
            ..
        } = readings.ledger(change.ledger);
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
        *places = change.places;
        *floor = change.floor;
        *carried = change.carried;
        *last = (normal, now);
        reading.budget_peak = reading.budget_peak.max(normal);
        reading.held_peak = reading.held_peak.max(change.held);
        reading.floor_peak = reading.floor_peak.max(change.floor + change.carried);
        reading.floor_bound = reading.floor_bound.max(change.floor_bound);
        reading.waited = reading.waited.max(change.waited);
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
        let reading = &mut readings.ledger(admission.ledger).reading;
        if admission.depth <= DATED_DEPTH && !admission.leftover {
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
/// the count the ledger itself keeps: two counts of one quantity must
/// agree.
fn admitted() -> Admitted {
    READINGS.with(|readings| {
        let mut readings = readings.borrow_mut();
        readings.link = None;
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
    let (a, b) = pair(case.shape, case.budget);
    let link = Link::new(case.link);
    let (mut a_end, mut b_end) = (link.end(Side::A), link.end(Side::B));
    start_reading(&link);
    let mut outcome = None;
    let observed = within(&mut || {
        let (a_out, b_out, report) = drive(
            &link,
            a.gossip_once(&mut a_end.read, &mut a_end.write),
            b.gossip_once(&mut b_end.read, &mut b_end.write),
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
