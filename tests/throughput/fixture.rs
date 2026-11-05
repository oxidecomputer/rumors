//! Deterministic replica pairs, and the runner that reconciles them over a
//! [`Link`].
//!
//! Every pair forks one network seeded from a fixed ChaCha stream, so its
//! leaves, and with them the tree's shape, are the same on every run and
//! every machine. Every message is a 98-character string, which encodes in
//! CBOR as a two-byte head and its bytes: 100 bytes, the sizing guide's
//! reference message.

use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use rumors::{Peer, Rumors};
use rumors_testkit::bench::link::{Link, LinkConfig, Report, Side, drive};
use rumors_testkit::common::{
    window::WindowChoice,
    wire::{block_on, bootstrap_fork_with_window_async},
};

/// The ChaCha seed of every fixture's network.
const NETWORK_SEED: u64 = 7;

/// The messages each side of a pair holds that the other lacks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Shape {
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
            a: messages,
            b: messages,
        }
    }
}

/// The fixture message numbered `index` on `side`: the side's letter and
/// the index, zero-padded to 98 characters.
fn message(side: Side, index: u64) -> String {
    let letter = match side {
        Side::A => 'a',
        Side::B => 'b',
    };
    format!("{letter}{index:0>97}")
}

/// Builds a pair of the given shape, both peers on a memory budget of
/// `budget` bytes.
pub fn pair(shape: Shape, budget: usize) -> (Rumors<String>, Rumors<String>) {
    block_on(async {
        let mut rng = ChaChaRng::seed_from_u64(NETWORK_SEED);
        let root = Peer::<String>::seed_rng(&mut rng).into_rumors();
        let a = bootstrap_fork_with_window_async(&root, WindowChoice::Budget(budget)).await;
        let b = bootstrap_fork_with_window_async(&root, WindowChoice::Budget(budget)).await;
        a.send_all((0..shape.a).map(|index| message(Side::A, index)))
            .expect("fixture messages encode");
        b.send_all((0..shape.b).map(|index| message(Side::B, index)))
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

/// One completed session: what the link carried.
#[derive(Clone, Debug)]
pub struct Session {
    /// The session that ran.
    pub case: Case,
    /// What the link carried, and when.
    pub report: Report,
}

impl Session {
    /// The session's duration, in byte-times.
    pub fn duration(&self) -> u64 {
        self.report.duration()
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
    let (a_out, b_out, report) = drive(
        &link,
        a.gossip_once(&mut a_end.read, &mut a_end.write),
        b.gossip_once(&mut b_end.read, &mut b_end.write),
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
    Session { case, report }
}
