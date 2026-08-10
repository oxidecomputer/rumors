//! The coverage roster's enforcement: totality against the extracted
//! public surface, liveness of every cited binding test, and the
//! committed-seed tripwire.

use std::collections::BTreeSet;
use std::fs;

use super::{
    cited_test_names, crate_root, declared_test_names, declared_test_names_by_file,
    extract_public_fns, FAMILY_SURFACE, METHOD_SURFACE, TRIPWIRES,
};

/// The deliberate same-named test pairs, by declaring files.
///
/// Parallel same-shaped suites in sibling modules: the party/version codec
/// pins, the clock/party fold differentials, the clock/oracle worked
/// examples, the skyline query/sweep corpus sweeps.
///
/// Citations resolve by bare name, so a duplicated name is satisfiable by
/// either declaration — deleting one copy would leave every citation green
/// while half the coverage vanished. This roster makes each duplication a
/// reviewed decision: [`duplicate_test_names_are_rostered`] holds it equal,
/// both directions, to the scan of the tree.
const DUPLICATE_TEST_NAMES: &[(&str, &[&str])] = &[
    (
        "as_bytes_matches_encode",
        &["src/party/tests.rs", "src/version/tests.rs"],
    ),
    (
        "byte_equality_matches_bit_equality",
        &["src/party/tests.rs", "src/version/tests.rs"],
    ),
    (
        "decode_encode_arbitrary",
        &["src/party/tests.rs", "src/version/tests.rs"],
    ),
    (
        "exhaustive_small_scope_agrees",
        &[
            "src/version/skyline/query/tests.rs",
            "src/version/skyline/sweep/tests.rs",
        ],
    ),
    (
        "heterogeneous_joins",
        &["src/clock/tests.rs", "src/oracle/tests.rs"],
    ),
    (
        "join_all_agrees_with_oracle_on_aliased_coalesced_group",
        &["src/clock/tests.rs", "src/party/tests.rs"],
    ),
    (
        "join_all_matches_the_recursive_oracle",
        &["src/clock/tests.rs", "src/party/tests.rs"],
    ),
    (
        "organic_histories_agree",
        &[
            "src/version/skyline/query/tests.rs",
            "src/version/skyline/sweep/tests.rs",
        ],
    ),
    ("sync", &["src/clock/tests.rs", "src/oracle/tests.rs"]),
    (
        "worked_example",
        &["src/clock/tests.rs", "src/oracle/tests.rs"],
    ),
];

/// The roster is total over the public inherent-`pub fn` surface, both
/// directions.
///
/// Every extracted operation has exactly one named row, and every method
/// row names an operation that still exists — so a new public op forces
/// a reviewed row, and a removed op orphans one.
#[test]
fn roster_is_total_over_the_public_fn_surface() {
    let extracted = extract_public_fns();
    let rostered: BTreeSet<String> = METHOD_SURFACE.iter().map(|row| row.op.to_owned()).collect();
    assert_eq!(
        rostered.len(),
        METHOD_SURFACE.len(),
        "duplicate roster rows: every op gets exactly one row"
    );
    let missing: Vec<&String> = extracted.difference(&rostered).collect();
    let orphaned: Vec<&String> = rostered.difference(&extracted).collect();
    assert!(
        missing.is_empty() && orphaned.is_empty(),
        "the coverage roster and the public surface disagree.\n\
         public ops without a roster row (add one, naming each leg's \
         disposition): {missing:?}\n\
         roster rows without a public op (remove or rename): {orphaned:?}"
    );
}

/// Every test name the roster or a tripwire cites resolves to an
/// executable binding.
///
/// The bindings: a `#[test]`-attributed item under `src/`, or a law
/// name registered in [`crate::laws`]'s tables (the entries the
/// algebraic-laws drivers run). A renamed or deleted binding test fails
/// here by name, so a disposition can never silently point at nothing,
/// and a same-named helper or kernel can never stand in for the test a
/// row claims.
#[test]
fn every_cited_binding_test_exists() {
    let mut declared = declared_test_names();
    declared.extend(
        crate::laws::registered_names()
            .into_iter()
            .map(str::to_owned),
    );
    let dead: Vec<&str> = cited_test_names()
        .into_iter()
        .filter(|name| !declared.contains(*name))
        .collect();
    assert!(
        dead.is_empty(),
        "roster/tripwire citations resolve to no `#[test]` item or \
         registered law: {dead:?}"
    );
}

/// The citation haystack admits only executable tests: a helper `fn`
/// must never satisfy a binding-test citation.
///
/// Two directions. Negative: named non-test helpers — declared `fn`s the
/// old bare-name scan accepted — are absent from
/// [`declared_test_names`], so a roster row whose cited differential test
/// is deleted goes red even while a same-named helper survives. Positive
/// (the scan's own liveness): a known `#[test]` item and a known
/// proptest-block property both resolve, so the seal cannot green by
/// scanning nothing.
#[test]
fn citation_haystack_admits_only_attributed_tests() {
    let declared = declared_test_names();
    for helper in ["declared_test_names", "parse_impl_self_type", "fn_name"] {
        assert!(
            !declared.contains(helper),
            "{helper} is a helper fn, not a test, and must not be able to \
             satisfy a binding-test citation"
        );
    }
    for test in [
        // This suite's own plain `#[test]`.
        "every_cited_binding_test_exists",
        // A `proptest!`-block property the roster cites.
        "join_all_matches_the_recursive_oracle",
    ] {
        assert!(
            declared.contains(test),
            "{test} is an attributed test and must resolve in the haystack"
        );
    }
    // The law leg: registered names come from the tables the drivers run,
    // and the tables' local helper fns are not registered.
    let laws = crate::laws::registered_names();
    assert!(
        laws.contains(&"forks_matches_from_array"),
        "a roster-cited law must be registered in its table"
    );
    assert!(
        !laws.contains(&"le") && !laws.contains(&"hash_of"),
        "laws.rs helper fns must not be able to satisfy a citation"
    );
}

/// Every payload name an exclusion carries resolves to an executable
/// binding, and each family's structural obligation holds.
///
/// The exclusion legs are the roster's largest genre, and their payloads
/// carry the production-side pins each exclusion rests on.
/// [`every_cited_binding_test_exists`] never sees them — `Leg::cited`
/// returns `None` for exclusions — so a pin renamed or deleted while
/// cited only in a payload would rot silently. Every `pins` element and
/// `license` must be a `#[test]`-attributed item under `src/` or a
/// registered law name; a `GridCap` guard must be an executable test
/// (a premise guard must run, not merely resolve); a `NotAPaperObject`
/// binding site must be a roster row, test, or law.
#[test]
fn exclusion_payload_citations_resolve() {
    use crate::surface::Exclusion;
    let mut resolvable = declared_test_names();
    resolvable.extend(
        crate::laws::registered_names()
            .into_iter()
            .map(str::to_owned),
    );
    let tests_only = declared_test_names();
    let rows: BTreeSet<&str> = METHOD_SURFACE
        .iter()
        .chain(FAMILY_SURFACE)
        .map(|row| row.op)
        .collect();
    let mut dead: Vec<String> = Vec::new();
    for row in METHOD_SURFACE.iter().chain(FAMILY_SURFACE) {
        for leg in [&row.prod_tree, &row.prod_fs, &row.tree_fs] {
            let Some(family) = leg.exclusion() else {
                continue;
            };
            let mut names: Vec<&str> = Vec::new();
            match family {
                Exclusion::NoWireFormatInReferences { pins }
                | Exclusion::DefinitionalCombinator { pins }
                | Exclusion::NAryNotInReferences { pins }
                | Exclusion::LinearityMechanics { pins } => names.extend(*pins),
                Exclusion::NotAPaperObject { bound_at, pins } => {
                    names.extend(*pins);
                    if !rows.contains(bound_at) {
                        names.push(bound_at);
                    }
                }
                Exclusion::GridCap { guard } => {
                    if !tests_only.contains(*guard) {
                        dead.push(format!(
                            "{}: GridCap guard {guard} is no executable #[test]",
                            row.op
                        ));
                    }
                }
                Exclusion::RepresentationMechanics { license } => names.push(license),
            }
            for name in names {
                if !resolvable.contains(name) {
                    dead.push(format!("{}: {name}", row.op));
                }
            }
        }
    }
    assert!(
        dead.is_empty(),
        "exclusion payloads cite names that resolve to no `#[test]` item, \
         registered law, or (for bound_at) roster row: {dead:?}"
    );
}

/// Every exclusion family is inhabited: an empty family is a dead
/// category, dissolvable rather than carried in the vocabulary.
#[test]
fn every_exclusion_family_is_inhabited() {
    use std::collections::BTreeMap;
    let mut census: BTreeMap<&str, usize> = BTreeMap::new();
    for row in METHOD_SURFACE.iter().chain(FAMILY_SURFACE) {
        for leg in [&row.prod_tree, &row.prod_fs, &row.tree_fs] {
            if let Some(family) = leg.exclusion() {
                *census.entry(family.family()).or_default() += 1;
            }
        }
    }
    for family in crate::surface::Exclusion::FAMILIES {
        assert!(
            census.get(family).copied().unwrap_or(0) > 0,
            "exclusion family {family} is uninhabited: dissolve it or \
             inhabit it"
        );
    }
}

/// Every test name declared in more than one file is rostered in
/// [`DUPLICATE_TEST_NAMES`] with exactly its declaring files.
///
/// Both directions: a new same-named test fails here until the
/// duplication is reviewed and rostered, and a rostered duplicate losing
/// a copy fails here instead of leaving its citations satisfiable by the
/// survivor.
///
/// This is the duplicate-name half of citation integrity: the bare-name
/// citation check cannot tell which file satisfies which roster row, so
/// every collision must be a committed, named decision.
#[test]
fn duplicate_test_names_are_rostered() {
    let scanned: Vec<(String, Vec<String>)> = declared_test_names_by_file()
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .map(|(name, files)| (name, files.into_iter().collect()))
        .collect();
    let rostered: Vec<(String, Vec<String>)> = DUPLICATE_TEST_NAMES
        .iter()
        .map(|(name, files)| {
            (
                (*name).to_owned(),
                files.iter().map(|f| (*f).to_owned()).collect(),
            )
        })
        .collect();
    assert_eq!(
        scanned, rostered,
        "the same-named tests in the tree and the DUPLICATE_TEST_NAMES \
         roster must agree exactly: an unrostered duplicate makes bare-name \
         citations ambiguous, and a rostered phantom names a deleted copy"
    );
}

/// No registered law name is also a `#[test]` name: a citation must resolve
/// to exactly one binding kind.
///
/// The haystack the citation checks search is the union of the declared
/// tests and the law tables; a name living in both is doubly satisfiable,
/// so deleting either binding leaves every citation green while the
/// coverage it named silently halves.
#[test]
fn law_names_never_shadow_test_names() {
    let declared = declared_test_names();
    let shadowed: Vec<&str> = crate::laws::registered_names()
        .into_iter()
        .filter(|name| declared.contains(*name))
        .collect();
    assert!(
        shadowed.is_empty(),
        "law names doubling as #[test] names make citations ambiguous: \
         {shadowed:?}"
    );
}

/// The family roster's rows are unique by op description (totality over
/// the operator/trait surface is by review of the file; this pins the
/// table's internal hygiene).
#[test]
fn family_rows_are_unique() {
    let ops: BTreeSet<&str> = FAMILY_SURFACE.iter().map(|row| row.op).collect();
    assert_eq!(ops.len(), FAMILY_SURFACE.len(), "duplicate family rows");
}

/// Every tripwire leg label is nonempty and unique — the tripwire list
/// stays a legible per-leg inventory, not a grab bag.
#[test]
fn tripwires_are_labeled() {
    let labels: BTreeSet<&str> = TRIPWIRES.iter().map(|(label, _)| *label).collect();
    assert_eq!(labels.len(), TRIPWIRES.len(), "duplicate tripwire labels");
    assert!(
        TRIPWIRES.iter().all(|(label, _)| !label.is_empty()),
        "empty tripwire label"
    );
}

/// The prod↔tree adequacy seeds stay committed: the two fold-mutation
/// witnesses replay through the `join_all` differentials from these
/// files on every run, and this pin makes stripping them a red diff.
#[test]
fn d1_seeds_stay_committed() {
    for (file, seed) in [
        ("proptest-regressions/party/tests.txt", "cc e1aea6c3"),
        ("proptest-regressions/clock/tests.txt", "cc efc8c717"),
    ] {
        let path = crate_root().join(file);
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        assert!(
            text.contains(seed),
            "{file} no longer carries the committed fold seed {seed}"
        );
    }
}
