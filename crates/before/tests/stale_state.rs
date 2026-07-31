//! What deterministic ticking does when an earlier state comes back
//! into play.
//!
//! A version carries no record of who ticked it, and a party's tick is a
//! pure function of the version it is applied to — so re-running a party
//! over a duplicated version re-mints the same successors. For a
//! *version* that is valid use: a version records causal knowledge, not
//! event identity, and duplicating one to stamp divergent timelines is
//! how version-vector-style callers work. Three pins state that model.
//! For a *clock* it is the hazard the linearity rule exists to prevent:
//! restoring a clock from bytes persisted before its latest advance
//! brings a retired state of the identity back into play, and the
//! restored party overlaps its own descendant. The fourth witness pins
//! that violation. All four are built from individually documented
//! operations (`Version` is freely `Clone`; encode/decode round-trips;
//! `from_parts` pairs parts): the pins hold the crate docs' statements
//! to what actually happens.

use before::{Clock, Party, Version};

/// One party ticking two divergent clones of the same base version
/// mints equal versions.
///
/// Valid by the model: the two timelines carry equal causal knowledge,
/// so their stamps compare equal — a version is not an event identifier.
#[test]
fn same_party_ticks_on_divergent_clones_mint_equal_versions() {
    let p = Party::seed();
    let mut v1 = Version::new();
    v1.tick(&p); // one event, recorded in v1's history
    let mut v2 = v1.clone(); // Version is freely Cloneable
    v1.tick(&p); // a second event, on the v1 line
    v2.tick(&p); // a distinct event, on the v2 line

    // The two lines now carry equal knowledge and compare equal.
    assert_eq!(v1, v2);
    assert!(!v1.concurrent(&v2));
}

/// The join of two same-party-ticked clones equals either line.
///
/// Join merges causal knowledge; two lines carrying equal knowledge
/// add nothing to each other.
#[test]
fn join_of_same_party_ticked_clones_equals_either_line() {
    let p = Party::seed();
    let mut v1 = Version::new();
    v1.tick(&p);
    let mut v2 = v1.clone();
    v1.tick(&p);
    v2.tick(&p);
    let joined = v1.clone() | v2.clone();
    assert_eq!(joined, v1);
    assert_eq!(joined, v2);
}

/// A restored pre-tick backup violates linearity with zero forks
/// anywhere in the history.
///
/// A clock backed up (encode), advanced (tick = event X), then restored
/// from the backup and ticked again (event Y) hands both events the
/// *same* version — and the restored party overlaps the original, so
/// the system holds two live non-disjoint parties from one seed and
/// zero forks.
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

/// `Clock::from_parts` over an earlier version re-mints the successor
/// the party's later tick already produced.
///
/// Valid by the model: the party is the moved, latest state of the
/// identity, and the version it is paired with is knowledge, free to
/// duplicate — the pairing door is a version-duplication site.
#[test]
fn from_parts_over_an_earlier_version_re_mints_its_successor() {
    let mut c = Clock::seed();
    let earlier: Version = c.tick().clone(); // snapshot after event 1
    let x: Version = c.tick().clone(); // event X
    let (party, _current) = c.into_parts();
    let mut rebuilt = Clock::from_parts(party, earlier);
    let y: Version = rebuilt.tick().clone(); // event Y
    assert_eq!(x, y, "the rebuilt line re-mints the same successor");
}
