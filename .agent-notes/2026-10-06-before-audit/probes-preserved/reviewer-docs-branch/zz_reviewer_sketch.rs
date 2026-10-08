//! Reviewer sketches for docs/audit-corrections; temporary, never committed.
//! One seed per test: no two universes interact.

use before::{Clock, Version};

/// `from_parts` over a version that lacks one of the party's events, with a
/// receive between that event and the party's last stamp: the rebuilt
/// clock's tick is no stamp the party issued, yet the party's last stamp
/// already contains it.
#[test]
fn from_parts_after_a_receive_yields_an_unissued_dominated_version() {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let m = bob.tick().clone();
    let s1 = alice.tick().clone();
    let s2 = alice.recv(&m).clone();
    let (party, latest) = alice.into_parts();
    assert_eq!(latest, s2);
    let mut rebuilt = Clock::from_parts(party, m.clone());
    let y = rebuilt.tick().clone();
    eprintln!("m={m:?}\ns1={s1:?}\ns2={s2:?}\ny={y:?}\ns1|m={:?}", &s1 | &m);
    assert_ne!(y, s1, "y is not alice's first stamp");
    assert_ne!(y, s2, "y is not alice's second stamp");
    assert!(y < s2, "alice's issued stamp s2 already contains the new event y");
}

/// The same with the stale version taken from the clock's own history: the
/// rebuilt clock is a retired state, yet its tick is still no issued stamp.
#[test]
fn from_parts_over_a_held_version_before_a_receive_yields_an_unissued_stamp() {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let m = bob.tick().clone();
    let s1 = alice.tick().clone();
    let s2 = alice.recv(&m).clone();
    let (party, _latest) = alice.into_parts();
    let mut rebuilt = Clock::from_parts(party, s1.clone());
    let y = rebuilt.tick().clone();
    eprintln!("s1={s1:?}\ns2={s2:?}\ny={y:?}");
    assert_ne!(y, s1);
    assert_ne!(y, s2);
    assert!(y < s2);
}

/// The off-grid example in `two_party_grid`'s doc is inside the grid's
/// interval and off the grid, and the grid is closed under join and meet.
#[test]
fn two_party_grid_off_grid_example_and_closure() {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let a1 = alice.tick().clone();
    let a2 = alice.tick().clone();
    let b1 = bob.tick().clone();
    let b2 = bob.tick().clone();
    let heights_a = [None, Some(&a1), Some(&a2)];
    let heights_b = [None, Some(&b1), Some(&b2)];
    let grid: Vec<Version> = heights_a
        .iter()
        .flat_map(|a| heights_b.iter().map(move |b| (a, b)))
        .map(|(a, b)| match (a, b) {
            (None, None) => Version::new(),
            (Some(a), None) => (*a).clone(),
            (None, Some(b)) => (*b).clone(),
            (Some(a), Some(b)) => *a | *b,
        })
        .collect();
    let top = &a2 | &b2;

    // Two of alice's events over one half of her identity, one over the other.
    let (mut left, _) = alice.into_parts();
    let right = left.fork();
    let mut v = Version::new();
    right.tick(&mut v);
    left.ticks(&mut v, 2u8);
    eprintln!("a1={a1:?}\na2={a2:?}\nv={v:?}");
    assert!(Version::new() <= v && v <= top, "inside the interval");
    assert!(a1 < v && v < a2, "strictly between alice's uniform heights one and two");
    assert!(!grid.contains(&v), "off the grid");
    assert_eq!(Version::decode(&v.encode()[..]).unwrap(), v, "canonical");

    for x in &grid {
        for y in &grid {
            assert!(grid.contains(&(x | y)), "closed under join");
            assert!(grid.contains(&(x & y)), "closed under meet");
        }
    }
}
