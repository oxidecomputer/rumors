//! Stale-state hazards: what deterministic ticking does when an earlier
//! state of an identity comes back into play.
//!
//! A version carries no record of who ticked it, and a party's tick is a
//! pure function of the version it is applied to — so any path that
//! re-runs an identity over a state it already advanced past re-mints
//! spent versions, and distinct real-world events compare as causally
//! identical. These tests pin the concrete corruption shapes the
//! crate-level safety rules exist to prevent, each built from
//! individually documented operations (`Version` is freely `Clone`;
//! encode/decode round-trips; `from_parts` pairs parts): the behavior is
//! inherent to interval tree clocks, and the pins hold the crate docs'
//! warnings to what actually happens.

use before::{Clock, Party, Version};

/// One party ticking two divergent clones of the same base version
/// yields equal versions: two events the caller meant as distinct
/// concurrent occurrences compare as causally identical, with no
/// `Party` or `Clock` ever duplicated.
#[test]
fn same_party_divergent_clones_conflate() {
    let p = Party::seed();
    let mut v1 = Version::new();
    v1.tick(&p); // event A, recorded in v1's history
    let mut v2 = v1.clone(); // Version is freely Cloneable
    v1.tick(&p); // event B, on the v1 line
    v2.tick(&p); // event C, on the v2 line: a different event

    // B and C are distinct events on divergent histories, yet the two
    // histories now compare equal: causal order that never happened.
    assert_eq!(v1, v2);
    assert!(!v1.concurrent(&v2));
}

/// The dual corruption: joining the two conflated histories loses an
/// event. The join of two lines carrying three distinct events is
/// indistinguishable from either single line, so no reconciliation can
/// recover the third event.
#[test]
fn conflated_histories_join_loses_an_event() {
    let p = Party::seed();
    let mut v1 = Version::new();
    v1.tick(&p);
    let mut v2 = v1.clone();
    v1.tick(&p);
    v2.tick(&p);
    let joined = v1.clone() | v2.clone();
    // The join of two lines carrying three distinct events equals
    // either single line: one event has vanished from causal history.
    assert_eq!(joined, v1);
    assert_eq!(joined, v2);
}

/// A clock backed up (encode), advanced (tick = event X), then restored
/// from the backup and ticked again (event Y): the two distinct events
/// receive the *same* version, with no fork anywhere in the history —
/// and the restored party overlaps the original, so the system holds
/// two live non-disjoint parties from one seed and zero forks.
#[test]
fn restored_pre_tick_clock_conflates_without_any_fork() {
    let mut c = Clock::seed();
    c.tick();
    let backup = c.encode();
    let x: Version = c.tick().clone(); // event X, after the backup
    let mut restored = Clock::decode(&backup[..]).unwrap();
    let y: Version = restored.tick().clone(); // event Y, a distinct event
    assert_eq!(x, y, "distinct events X and Y received the same version");
    assert_eq!(c.version(), restored.version());
    // Two live overlapping parties, from a one-seed, zero-fork history.
    assert!(!c.party().is_disjoint(restored.party()));
}

/// `Clock::from_parts` pairs a party with a version that does not carry
/// the party's full tick history: the rebuilt clock re-mints an
/// already-spent version for a new event. No fork, no decode — the
/// pairing door alone reproduces the conflation.
#[test]
fn from_parts_stale_version_conflates() {
    let mut c = Clock::seed();
    let stale: Version = c.tick().clone(); // snapshot after event 1
    let x: Version = c.tick().clone(); // event X
    let (party, _current) = c.into_parts();
    let mut rebuilt = Clock::from_parts(party, stale);
    let y: Version = rebuilt.tick().clone(); // event Y
    assert_eq!(x, y, "distinct events X and Y received the same version");
}
