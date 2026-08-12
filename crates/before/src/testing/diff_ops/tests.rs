//! The descriptor table's own guards: the tiling against the coverage
//! roster, the genre vocabulary's hygiene, and the registration totality
//! pin.
//!
//! The descriptors are *asserted* by the drivers beside them; here we pin
//! the collection's own invariants.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use proptest::prelude::*;

use super::{registered_names, BespokeGenre, DiffOp, DIFF_BESPOKE, REGISTERED_GROUPS};
use crate::oracle;
use crate::surface::{Leg, FAMILY_SURFACE, METHOD_SURFACE};

/// Assert every descriptor in a slice, naming the violated one on failure.
///
/// The vehicle every driver asserts through: the drivers differ only in
/// which population they feed it.
macro_rules! assert_diff_ops {
    ($group:expr, $($input:expr),+) => {
        for (name, check) in $group {
            prop_assert!(check($($input),+), "descriptor violated: {}", name);
        }
    };
}

/// Every test name a `Leg::Bound` disposition cites, across both rosters.
///
/// `Bound` is the differential leg — the one this table exists to derive —
/// so it is the leg the tiling quantifies over. `Law`, `Trans`, and
/// `Excluded` dispositions are held by their own vocabularies in the
/// coverage suite.
fn bound_citations() -> BTreeSet<&'static str> {
    METHOD_SURFACE
        .iter()
        .chain(FAMILY_SURFACE)
        .flat_map(|row| {
            [&row.prod_tree, &row.prod_fs, &row.tree_fs]
                .into_iter()
                .filter_map(|leg| match leg {
                    Leg::Bound(test) => Some(*test),
                    _ => None,
                })
        })
        .collect()
}

/// Every `Bound` citation in the coverage roster is derived from the
/// descriptor table or bespoke under a declared genre, never both, never
/// neither.
///
/// The seam this pin defends is a drift, not an error: a pointwise pure
/// operation added as one more hand-written body, because that was the
/// shorter path on the day. Making bespoke a *rostered status* rather than
/// the default turns that choice into a named diff — the new citation fails
/// here until someone writes down which genre excuses it — and holds the
/// reverse direction too, so a bespoke entry outliving the body it names is
/// a phantom rather than silent slack. Both tables name only citations the
/// roster actually makes, so a renamed differential orphans the entry that
/// leaned on it.
#[test]
fn diff_ops_tile_the_bound_citations() {
    let cited = bound_citations();
    let derived: BTreeSet<&str> = registered_names().into_iter().collect();
    assert_eq!(
        derived.len(),
        registered_names().len(),
        "duplicate descriptor names: a failure must name exactly one descriptor"
    );

    let mut bespoke: BTreeMap<&str, BespokeGenre> = BTreeMap::new();
    for (name, genre) in DIFF_BESPOKE {
        assert!(
            cited.contains(*name),
            "DIFF_BESPOKE names {name:?}, which no roster row cites as a \
             Bound differential: remove or rename the entry"
        );
        assert!(
            !derived.contains(*name),
            "{name}: derived from the descriptor table AND rostered as \
             bespoke — the tiling sides must stay disjoint; remove one"
        );
        assert!(
            bespoke.insert(*name, *genre).is_none(),
            "{name} appears twice in DIFF_BESPOKE"
        );
    }

    let unclassified: Vec<&str> = cited
        .iter()
        .copied()
        .filter(|name| !derived.contains(name) && !bespoke.contains_key(name))
        .collect();
    assert!(
        unclassified.is_empty(),
        "Bound citations neither derived from the descriptor table nor \
         rostered in DIFF_BESPOKE with a genre (migrate them into a \
         descriptor, or declare the genre that excuses them): {unclassified:?}"
    );

    // The reverse leg on the derived side: a descriptor no row cites is a
    // check nothing in the roster claims, which the coverage suite would
    // never notice going missing.
    let orphans: Vec<&str> = derived
        .iter()
        .copied()
        .filter(|name| !cited.contains(name))
        .collect();
    assert!(
        orphans.is_empty(),
        "registered descriptors cited by no roster row (cite each from the \
         row it binds, or retire the descriptor): {orphans:?}"
    );
}

/// Every bespoke genre is inhabited: an empty genre is a dead category,
/// dissolved rather than carried in the vocabulary.
#[test]
fn every_bespoke_genre_is_inhabited() {
    let mut census: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, genre) in DIFF_BESPOKE {
        *census.entry(genre.name()).or_default() += 1;
    }
    for genre in BespokeGenre::GENRES {
        assert!(
            census.get(genre).copied().unwrap_or(0) > 0,
            "bespoke genre {genre} is uninhabited: dissolve it or inhabit it"
        );
    }
}

/// Every `pub(crate) static` descriptor group in `diff_ops.rs` is carried
/// by the roster (`for_each_diff_group!`) — no group can compile and never
/// execute.
///
/// Every consumer derives from the roster by macro expansion, so a rostered
/// group is executed by construction and needs no per-consumer pin. The one
/// door that leaves open is a group static missing from the roster, which
/// nothing would run; this pin closes it against a source scan of the
/// declarations in the module (its only `pub(crate) static`s are descriptor
/// groups). The known-bad groups deliberately live in this test file, out
/// of the scan's reach, since registering them would drive them as if they
/// were real.
#[test]
fn every_descriptor_group_is_registered() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/testing/diff_ops.rs");
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let mut declared = BTreeSet::new();
    for line in text.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("pub(crate) static ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            assert!(
                !name.is_empty(),
                "unnamed pub(crate) static in diff_ops.rs: {line}"
            );
            declared.insert(name);
        }
    }
    let registered: BTreeSet<String> = REGISTERED_GROUPS
        .iter()
        .map(|group| (*group).to_string())
        .collect();
    assert_eq!(
        declared, registered,
        "the descriptor-group statics in diff_ops.rs and the \
         for_each_diff_group! roster must be the same set: an unrostered \
         group never executes, and a rostered phantom names nothing"
    );
}

// ───────────────────── the known-bad descriptors, held convicted ─────────────────────
//
// A table centralizes each operation's oracle spelling: one descriptor is
// the only transcription every population sees, where a body per population
// was an independent transcription each. That trade is only payable if a
// wrong transcription cannot pass, so the wrong ones are committed here and
// held convicted. These groups are deliberately absent from the roster —
// registering them would drive them as if they were real — and the
// registration totality pin scans only the table's own file, so their
// `pub(crate) static`s do not reach it.

diff_ops! {
    /// The mis-transcribed version-pair descriptor: the oracle leg spells
    /// the join where the production leg spells the meet.
    ///
    /// The likeliest transcription slip is a dual operation, since the two
    /// sides read alike and differ only in one operator.
    pub(crate) static KNOWN_BAD_VERSION_PAIR: (a: version, b: version);

    /// `&` on production against `|` on the oracle.
    fn meet_transcribed_as_join {
        prod: a.clone() & b.clone(),
        tree: a.clone() | b.clone(),
    }
}

diff_ops! {
    /// The mis-transcribed id-pair descriptor: the oracle leg takes the
    /// region difference in the opposite operand order.
    ///
    /// The other likely slip is an operand swap on an asymmetric
    /// operation, which no amount of type checking catches.
    pub(crate) static KNOWN_BAD_PARTY_PAIR: (a: party, b: party);

    /// `a \ b` on production against `b \ a` on the oracle.
    fn without_transcribed_with_swapped_operands {
        prod: a.without(&b),
        tree: b.without(&a),
    }
}

/// Run a version-pair group through the drivers' assertion vehicle.
// The group's element type is the signature it carries; naming it would
// mint a synonym per signature to appease the lint.
#[allow(clippy::type_complexity)]
fn check_version_pair(
    group: &[DiffOp<fn(&oracle::Version, &oracle::Version) -> bool>],
    a: &oracle::Version,
    b: &oracle::Version,
) -> Result<(), TestCaseError> {
    assert_diff_ops!(group, a, b);
    Ok(())
}

/// Run an id-pair group through the drivers' assertion vehicle.
// The group's element type is the signature it carries; naming it would
// mint a synonym per signature to appease the lint.
#[allow(clippy::type_complexity)]
fn check_party_pair(
    group: &[DiffOp<fn(&oracle::Party, &oracle::Party) -> bool>],
    a: &oracle::Party,
    b: &oracle::Party,
) -> Result<(), TestCaseError> {
    assert_diff_ops!(group, a, b);
    Ok(())
}

/// The assertion the drivers run convicts a mis-transcribed descriptor, and
/// passes it exactly where the mis-transcription makes no difference.
///
/// Two directions, and the second is what makes the first mean anything. A
/// comparison that had gone blind — a `Matches` implementation that always
/// agrees, a bridge that erases the result — would let the wrong descriptor
/// through, which the conviction witnesses catch. A comparison stuck at
/// "disagree" would convict everything, including correct descriptors,
/// which the agreement witnesses catch. Each known-bad descriptor is
/// therefore committed with an input pair where its two spellings genuinely
/// differ and one where they coincide, so the vehicle is shown to
/// discriminate rather than merely to fail.
#[test]
fn the_drivers_convict_a_mis_transcribed_descriptor() {
    use crate::oracle::Version as V;

    // The join and the meet of an ordered pair differ, so the swapped
    // operator changes the answer.
    assert!(
        check_version_pair(KNOWN_BAD_VERSION_PAIR, &V::leaf(1u64), &V::leaf(2u64)).is_err(),
        "the meet-transcribed-as-join descriptor must be convicted where \
         the join and the meet disagree"
    );
    // On a coincident pair they agree, so nothing is there to convict.
    assert!(
        check_version_pair(KNOWN_BAD_VERSION_PAIR, &V::leaf(3u64), &V::leaf(3u64)).is_ok(),
        "the same descriptor must pass where the join and the meet coincide: \
         a comparison that convicts everything convicts nothing"
    );

    // Two disjoint halves: each survives the other's removal, and the two
    // remainders are different regions, so the operand swap changes the
    // answer.
    let mut keep = oracle::Party::seed();
    let give = keep.fork();
    assert!(
        check_party_pair(KNOWN_BAD_PARTY_PAIR, &keep, &give).is_err(),
        "the operand-swapped difference descriptor must be convicted where \
         the two orders yield different regions"
    );
    // Against itself the difference is empty in either order.
    assert!(
        check_party_pair(KNOWN_BAD_PARTY_PAIR, &keep, &keep).is_ok(),
        "the same descriptor must pass where both operand orders yield the \
         empty region"
    );
}
