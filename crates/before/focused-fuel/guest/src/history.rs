//! The history fixture: snapshots of one clock's multi-party history, whose
//! causal order is known by construction.
//!
//! [`Snapshot`]'s documentation states the construction and the order.
//! Every round is a tick and a received send from one of the forked clocks
//! in turn, so each snapshot's stream has multi-party structure, and the raise of
//! `2^80` before the second phase gives the later snapshots wide codes.

use before::{Clock, Count, Version};
use focused_fuel_protocol::{unpack, Snapshot};

/// The rounds of each phase per sender, so every sender's send is received
/// several times and each phase grows every sender's region.
const ROUNDS_PER_SENDER: usize = 4;

/// The width in bits of the raise before the second phase, beyond every
/// machine word so the later streams exercise wide gamma codes.
const WIDE_RAISE_BITS: usize = 80;

/// The population records the snapshot after every round whose index within
/// its phase is a multiple of this.
const POPULATION_STRIDE: usize = 7;

/// The fixture's snapshots, and a population of intermediate snapshots for
/// the n-ary folds.
pub struct History {
    /// The snapshot after the first phase.
    start: Version,
    /// The snapshot after the wide raise and the second phase.
    middle: Version,
    /// The snapshot after the third phase.
    end: Version,
    /// The fork taken at `start`, ticked once per round.
    divergent: Version,
    /// The join of `end` and `divergent`.
    top: Version,
    /// Every seventh snapshot of the first two phases.
    population: Vec<Version>,
}

impl History {
    /// Builds the history of `senders` forked clocks, with four rounds per
    /// sender in each phase.
    ///
    /// The version tree holds one region per sender, so the snapshots'
    /// streams grow in proportion to `senders`; more rounds alone would only
    /// raise heights.
    ///
    /// # Panics
    ///
    /// Panics if the snapshots do not stand in the documented order, which
    /// would make every probe's expected answer wrong.
    pub fn new(senders: usize) -> History {
        let rounds = ROUNDS_PER_SENDER * senders;
        let mut main = Clock::seed();
        let mut senders: Vec<Clock> = (0..senders).map(|_| main.fork()).collect();
        let mut population = Vec::new();
        phase(&mut main, &mut senders, rounds, Some(&mut population));
        let start = main.version().clone();
        let mut diverged = main.fork();
        main.ticks(power_of_two(WIDE_RAISE_BITS));
        phase(&mut main, &mut senders, rounds, Some(&mut population));
        let middle = main.version().clone();
        phase(&mut main, &mut senders, rounds, None);
        let end = main.version().clone();
        for _ in 0..rounds {
            diverged.tick();
        }
        let divergent = diverged.version().clone();
        let top = &end | &divergent;
        assert!(
            start < middle && middle < end,
            "the snapshot chain is strict"
        );
        assert!(
            start <= divergent,
            "the divergent line extends the fork point"
        );
        assert!(
            middle.concurrent(&divergent) && end.concurrent(&divergent),
            "the lines diverge"
        );
        History {
            start,
            middle,
            end,
            divergent,
            top,
            population,
        }
    }

    /// Returns the snapshot `snapshot` names.
    pub fn get(&self, snapshot: Snapshot) -> &Version {
        match snapshot {
            Snapshot::Start => &self.start,
            Snapshot::Middle => &self.middle,
            Snapshot::End => &self.end,
            Snapshot::Divergent => &self.divergent,
            Snapshot::Top => &self.top,
        }
    }

    /// Returns the `N` snapshots that probe parameter `packed` names.
    ///
    /// # Panics
    ///
    /// Panics if `packed` does not name exactly `N` snapshots.
    pub fn snapshots<const N: usize>(&self, packed: u64) -> [Version; N] {
        let names = unpack::<N>(packed)
            .unwrap_or_else(|| panic!("probe parameter {packed:#x} does not pack {N} snapshots"));
        names.map(|name| self.get(name).clone())
    }

    /// Returns the start snapshot and the population.
    pub fn population(&self) -> (&Version, &[Version]) {
        (&self.start, &self.population)
    }
}

/// Runs one phase of `rounds` rounds on `main`, each a tick and a received
/// send from the next of `senders`, recording every seventh snapshot in
/// `population` if one is given.
fn phase(
    main: &mut Clock,
    senders: &mut [Clock],
    rounds: usize,
    mut population: Option<&mut Vec<Version>>,
) {
    for round in 0..rounds {
        main.tick();
        let message = senders[round % senders.len()].send().clone();
        main.recv(&message);
        if let Some(population) = population.as_mut() {
            if round % POPULATION_STRIDE == 0 {
                population.push(main.version().clone());
            }
        }
    }
}

/// Builds `2^exponent` as a tick count, by doubling.
fn power_of_two(exponent: usize) -> Count {
    let mut count = Count::from(1u8);
    for _ in 0..exponent {
        count += count.clone();
    }
    count
}

/// Returns a byte-equal copy of `version` decoded from its encoding, so it
/// holds a buffer of its own.
pub fn redecoded(version: &Version) -> Version {
    Version::decode(&version.encode()[..]).expect("a stored version's encoding decodes")
}
