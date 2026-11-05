//! Property-test support shared by this crate's unit tests and the test kit:
//! reproducible network seeds, and case counts scaled to each property's
//! cost.
//!
//! # Network seeds
//!
//! A network created with `Peer::seed()` draws its secret from the operating
//! system's random source, so anything that depends on the secret differs on
//! every run, and a failure that depends on it cannot be replayed. Tests and
//! fixtures therefore create networks only through a [`NetworkSeed`], which
//! derives the secret from a ChaCha stream whose output for a given seed is
//! fixed across platforms and `rand` releases.
//!
//! A test takes its seed in one of two ways, by what it asserts:
//!
//! - A claim that holds for every secret (convergence, causality, redaction,
//!   party conservation, deadlines) draws the seed from [`network_seed`] as a
//!   proptest input. A failing case prints its `NetworkSeed`, and proptest
//!   persists a regression seed that replays the same secret.
//! - A check of exact numbers (wire snapshots, byte counts, budget peaks,
//!   call counts) holds only for one secret, so it names one fixed seed as a
//!   constant built with [`NetworkSeed::new`].
//!
//! Appending `seed in network_seed()` as a proptest's *last* argument leaves
//! the inputs drawn before it unchanged, so the test's existing regression
//! seeds still replay the inputs they recorded.
//!
//! # Scaled case counts
//!
//! A property whose cases each cost many ordinary cases (a gossip session, a
//! fleet, a runtime with timers) runs a fraction of the configured count
//! ([`divided_cases`]). The default count then keeps it inside nextest's time
//! limit in a debug build, and a run that sets `PROPTEST_CASES` scales it in
//! proportion. This mirrors the scheme of `divided_cases` in `before`'s
//! `testing::property` module.
//!
//! `proptest!` cannot express this. Every form of the macro passes its
//! configuration through proptest's `contextualize_config`, which re-reads the
//! `PROPTEST_*` environment variables, so a set `PROPTEST_CASES` replaces
//! whatever case count the configuration carried. [`run`] starts instead from
//! the default configuration, which has already read the environment, and
//! divides its count afterwards. [`divided_proptest!`] wraps [`run`] in
//! `proptest!`'s syntax.
//!
//! The divisors below name cost classes shared across suites. A suite whose
//! cases cost something else names its own divisor beside the test.

use std::fmt;

use proptest::prelude::*;
use proptest::test_runner::{Config, TestCaseError, TestRunner};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::Peer;

pub use crate::divided_proptest;

/// The seed of one test network's secret.
///
/// Create each network once from its seed, and let other members join it by
/// bootstrapping. Two peers created from one seed share the network's
/// identity *and* its whole party, so letting them gossip breaks the linearity
/// of parties. A test that needs two independent networks draws two seeds.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NetworkSeed(u64);

impl NetworkSeed {
    /// Name a fixed seed, for a test whose assertions hold for one secret.
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The random stream this seed names.
    ///
    /// [`peer`](Self::peer) draws the secret from the start of this stream. A
    /// test that also draws its workload from the stream after the secret
    /// passes it to `Peer::seed_rng` itself.
    pub fn rng(self) -> ChaChaRng {
        ChaChaRng::seed_from_u64(self.0)
    }

    /// Create the first peer of the network this seed names.
    pub fn peer<T>(self) -> Peer<T>
    where
        T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
    {
        Peer::seed_rng(&mut self.rng())
    }
}

/// Prints the seed in hexadecimal, ready to paste into [`NetworkSeed::new`].
impl fmt::Debug for NetworkSeed {
    /// Write `NetworkSeed(0x…)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NetworkSeed({:#018x})", self.0)
    }
}

/// Every network seed, uniformly.
///
/// The strategy does not shrink toward a smaller seed: no seed is simpler
/// than another, so a shrunk failure keeps the seed it failed under.
pub fn network_seed() -> impl Strategy<Value = NetworkSeed> + Clone {
    any::<u64>().no_shrink().prop_map(NetworkSeed)
}

/// Pairs of distinct network seeds, for a test about two independent networks.
///
/// The second seed is the first plus an offset in `1..=u64::MAX`, wrapping, so
/// the two differ by construction rather than by rejecting equal draws.
pub fn distinct_network_seeds() -> impl Strategy<Value = (NetworkSeed, NetworkSeed)> + Clone {
    (any::<u64>(), any::<u64>())
        .no_shrink()
        .prop_map(|(first, offset)| {
            let second = first.wrapping_add(offset.max(1));
            (NetworkSeed(first), NetworkSeed(second))
        })
}

/// The divisor for a case that runs only local operations on one replica:
/// sends, redactions, snapshots, and observers, with no gossip session.
pub const LOCAL_DIVISOR: u32 = 4;

/// The divisor for a case that runs a few gossip sessions over in-memory
/// connections, or one bootstrap or retirement.
pub const SESSION_DIVISOR: u32 = 16;

/// The divisor for a case that builds a fleet, runs many sessions, or drives
/// sessions on a runtime with timers or sockets.
pub const FLEET_DIVISOR: u32 = 64;

/// The case count for a property whose cases each cost about `divisor`
/// ordinary cases: `configured` divided by `divisor`, rounded up.
///
/// # Panics
///
/// Panics if `divisor` is zero.
pub fn divided_cases(configured: u32, divisor: u32) -> u32 {
    configured.div_ceil(divisor)
}

/// Runs `test` on [`divided_cases`] of the configured case count, drawing
/// each case from `strategy`.
///
/// The run is the one `proptest!` performs, but for the case count:
/// persisted failures replay first, and a new failure shrinks and persists.
/// `test_name` and `source_file` play their `proptest!` roles. Pass the
/// test's `module_path!()`-qualified name and its own `file!()`, which
/// locates the persisted failures, as [`divided_proptest!`] does.
///
/// # Panics
///
/// Panics with proptest's report when a case fails, and if `divisor` is
/// zero.
pub fn run<S: Strategy>(
    divisor: u32,
    test_name: &'static str,
    source_file: &'static str,
    strategy: S,
    test: impl Fn(S::Value) -> Result<(), TestCaseError>,
) {
    let mut config = Config::default();
    config.cases = divided_cases(config.cases, divisor);
    config.test_name = Some(test_name);
    config.source_file = Some(source_file);
    let mut runner = TestRunner::new(config);
    if let Err(failure) = runner.run(&strategy, test) {
        panic!("{failure}\n{runner}");
    }
}

/// Declares property tests that run
/// [`divided_cases`](crate::testing::property::divided_cases) of the
/// configured case count, in `proptest!`'s syntax with `#![divisor(...)]` in
/// place of `#![proptest_config(...)]`.
///
/// A test draws its arguments as one tuple strategy, which consumes the
/// random stream exactly as `proptest!`'s argument list does, so seeds
/// persisted by `proptest!` replay the same inputs. A test body asserts by
/// panicking (`assert!`, `assert_eq!`), and proptest reports the panic as the
/// case's failure.
///
/// The macro is exported from the crate root, as every `macro_rules!` macro
/// used across crates must be, and re-exported from
/// [`testing::property`](crate::testing::property), the path to name it by.
#[doc(hidden)]
#[macro_export]
macro_rules! divided_proptest {
    (
        #![divisor($divisor:expr)]
        $(
            $(#[$meta:meta])*
            fn $name:ident($($parm:pat in $strategy:expr),+ $(,)?) $body:block
        )+
    ) => {
        $(
            $(#[$meta])*
            fn $name() {
                $crate::testing::property::run(
                    $divisor,
                    ::core::concat!(::core::module_path!(), "::", ::core::stringify!($name)),
                    ::core::file!(),
                    ($($strategy,)+),
                    |($($parm,)+)| {
                        let (): () = $body;
                        ::core::result::Result::Ok(())
                    },
                );
            }
        )+
    };
}
