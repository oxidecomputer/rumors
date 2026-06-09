//! Property tests for [`CausalList`].
//!
//! Real [`rumors::Version`]s can only be minted by a live `Known`, so these
//! tests drive the list with a synthetic vector clock — the same partially
//! ordered shape — generated from random causal histories (per-party chains
//! with random cross-party merges) and delivered in random arrival orders.

use std::cmp::Ordering;

use proptest::prelude::*;

use super::*;

/// A classic vector clock over a fixed set of parties: the minimal stand-in
/// with `Version`'s ordering structure (comparable along causal chains,
/// incomparable across concurrent forks).
#[derive(Clone, Debug, PartialEq)]
struct VClock(Vec<u64>);

impl PartialOrd for VClock {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let (mut le, mut ge) = (true, true);
        for (a, b) in self.0.iter().zip(&other.0) {
            le &= a <= b;
            ge &= a >= b;
        }
        match (le, ge) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (false, false) => None,
        }
    }
}

const PARTIES: usize = 4;

/// One step of a causal history: `party` ticks its clock, after optionally
/// merging the latest clock of `merge_from`.
#[derive(Clone, Debug)]
struct Step {
    party: usize,
    merge_from: Option<usize>,
}

/// Replay a history into one clock per event: event `i` is causally after
/// every earlier event of the same party, and after everything `merge_from`
/// had seen; events on parties that never exchanged are concurrent.
fn clocks(steps: &[Step]) -> Vec<VClock> {
    let mut latest: Vec<VClock> = (0..PARTIES).map(|_| VClock(vec![0; PARTIES])).collect();
    let mut out = Vec::with_capacity(steps.len());
    for step in steps {
        let mut clock = latest[step.party].clone();
        if let Some(from) = step.merge_from {
            let other = latest[from].clone();
            for (c, o) in clock.0.iter_mut().zip(&other.0) {
                *c = (*c).max(*o);
            }
        }
        clock.0[step.party] += 1;
        latest[step.party] = clock.clone();
        out.push(clock);
    }
    out
}

fn steps() -> impl Strategy<Value = Vec<Step>> {
    prop::collection::vec(
        (0..PARTIES, prop::option::of(0..PARTIES))
            .prop_map(|(party, merge_from)| Step { party, merge_from }),
        1..40,
    )
}

/// A history plus a random arrival order: `(key, clock)` pairs shuffled.
fn arrivals() -> impl Strategy<Value = Vec<(u32, VClock)>> {
    steps()
        .prop_map(|s| {
            clocks(&s)
                .into_iter()
                .enumerate()
                .map(|(i, c)| (i as u32, c))
                .collect::<Vec<_>>()
        })
        .prop_shuffle()
}

fn is_linear_extension(list: &CausalList<u32, VClock>) -> bool {
    let slots: Vec<_> = list.iter().collect();
    slots.iter().enumerate().all(|(j, later)| {
        slots[..j]
            .iter()
            .all(|earlier| later.version.partial_cmp(&earlier.version) != Some(Ordering::Less))
    })
}

proptest! {
    /// Linear extension: whatever order messages arrive in, the displayed
    /// sequence never shows a message before one it causally depends on.
    #[test]
    fn linear_extension(arrivals in arrivals()) {
        let mut list = CausalList::new();
        for (key, clock) in arrivals {
            list.insert(key, clock);
        }
        prop_assert!(is_linear_extension(&list));
    }

    /// Placement: each insert lands strictly after every present entry
    /// causally before it and strictly before every present entry causally
    /// after it (concurrent entries may fall on either side), and the
    /// returned index is the key's actual position.
    #[test]
    fn placement(arrivals in arrivals()) {
        let mut list = CausalList::new();
        for (key, clock) in arrivals {
            let index = list.insert(key, clock.clone()).expect("keys are unique");
            let slots: Vec<_> = list.iter().collect();
            prop_assert_eq!(slots[index].key, key);
            for (i, slot) in slots.iter().enumerate() {
                match slot.version.partial_cmp(&clock) {
                    Some(Ordering::Less) => prop_assert!(i < index),
                    Some(Ordering::Greater) => prop_assert!(i > index),
                    _ => {}
                }
            }
        }
    }

    /// Stability: an insert never reorders the entries already present.
    #[test]
    fn stability(arrivals in arrivals()) {
        let mut list = CausalList::new();
        for (key, clock) in arrivals {
            let before: Vec<u32> = list.iter().map(|s| s.key).collect();
            list.insert(key, clock);
            let after: Vec<u32> = list.iter().filter(|s| s.key != key).map(|s| s.key).collect();
            prop_assert_eq!(before, after);
        }
    }

    /// Idempotence: re-inserting a present key returns `None` and leaves the
    /// sequence untouched, whatever version the duplicate carries.
    #[test]
    fn idempotence(arrivals in arrivals()) {
        let mut list = CausalList::new();
        for (key, clock) in arrivals.clone() {
            list.insert(key, clock);
        }
        let before: Vec<u32> = list.iter().map(|s| s.key).collect();
        for (key, clock) in arrivals {
            prop_assert_eq!(list.insert(key, clock), None);
        }
        let after: Vec<u32> = list.iter().map(|s| s.key).collect();
        prop_assert_eq!(before, after);
    }

    /// Removal: removing any entry and re-inserting it yields a sequence
    /// that is still a linear extension and holds the same key set.
    #[test]
    fn remove_reinsert(arrivals in arrivals(), pick in any::<prop::sample::Index>()) {
        let mut list = CausalList::new();
        for (key, clock) in arrivals.clone() {
            list.insert(key, clock);
        }
        let (key, clock) = arrivals[pick.index(arrivals.len())].clone();
        let removed = list.remove(&key);
        prop_assert!(removed.is_some());
        prop_assert!(!list.contains(&key));
        prop_assert!(is_linear_extension(&list));
        list.insert(key, clock);
        prop_assert!(list.contains(&key));
        prop_assert_eq!(list.len(), arrivals.len());
        prop_assert!(is_linear_extension(&list));
    }
}

/// A causally old message arriving late lands mid-list (index < previous
/// length), which is exactly the UI's "inserted out of order" highlight
/// signal.
#[test]
fn late_arrival_lands_mid_list() {
    let a = VClock(vec![1, 0, 0, 0]);
    let b = VClock(vec![2, 0, 0, 0]); // a < b: same party, later
    let mut list = CausalList::new();
    assert_eq!(list.insert(1u32, b), Some(0));
    // `a` causally precedes the already-displayed `b`: it must land before it.
    assert_eq!(list.insert(0u32, a), Some(0));
    let order: Vec<u32> = list.iter().map(|s| s.key).collect();
    assert_eq!(order, vec![0, 1]);
}
