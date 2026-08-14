use super::{listing, select};
use crate::ops::{Inputs, OpSpec, Operand, ROSTER};

/// A roster row for selection tests: only the name participates in
/// selection, so everything else is inert.
fn spec(name: &'static str) -> OpSpec {
    OpSpec {
        name,
        inputs: Inputs::Packed(&[Operand::Version]),
        covers: &["unused"],
        size_measure: "unused",
        variant: "",
        contract: "unused",
        claim: "unused",
        measure: |_, _, _| unreachable!("selection never measures"),
    }
}

/// The names a selection picked, for comparison against expectations.
fn names(selected: &[&OpSpec]) -> Vec<&'static str> {
    selected.iter().map(|op| op.name).collect()
}

/// No filters selects the whole roster, in roster order: a bare
/// invocation of the runner stays a full survey.
#[test]
fn no_filters_selects_the_whole_roster() {
    let selected = select(ROSTER, &[]).expect("the empty filter list always selects");
    let roster_names: Vec<&str> = ROSTER.iter().map(|op| op.name).collect();
    assert_eq!(names(&selected), roster_names);
}

/// The `--list` output is exactly the selection's names, one per line,
/// in roster order — so listing the unfiltered selection is listing the
/// roster.
#[test]
fn listing_is_the_selection_one_name_per_line() {
    let selected = select(ROSTER, &[]).expect("the empty filter list always selects");
    let expected: String = ROSTER.iter().map(|op| format!("{}\n", op.name)).collect();
    assert_eq!(listing(&selected), expected);
}

/// A filter selects every operation whose name contains it as a
/// substring, preserving roster order.
#[test]
fn a_filter_selects_by_substring_in_roster_order() {
    let roster = [spec("alpha_join"), spec("alpha_meet"), spec("beta_join")];
    let selected = select(&roster, &["join".to_string()]).expect("two rows match");
    assert_eq!(names(&selected), ["alpha_join", "beta_join"]);
}

/// Multiple filters select the union of their matches, each row at most
/// once, still in roster order — not once per matching filter, and not
/// in filter order.
#[test]
fn filters_union_without_duplicates() {
    let roster = [spec("alpha_join"), spec("alpha_meet"), spec("beta_join")];
    let filters = ["join".to_string(), "alpha".to_string()];
    let selected = select(&roster, &filters).expect("every filter matches");
    assert_eq!(names(&selected), ["alpha_join", "alpha_meet", "beta_join"]);
}

/// A filter matching nothing is an error, and the error names the
/// offending filter and every available operation: a typo can never
/// produce a silent empty run, and the message hands the caller the
/// roster to pick from.
#[test]
fn a_zero_match_filter_errors_naming_the_roster() {
    let Err(err) = select(ROSTER, &["definitely_not_an_op".to_string()]) else {
        panic!("no roster name contains the filter, so selection must error");
    };
    let message = err.to_string();
    assert!(message.contains("\"definitely_not_an_op\""));
    for op in ROSTER {
        assert!(
            message.contains(op.name),
            "the error must name every available operation; {} is missing",
            op.name
        );
    }
}

/// The zero-match check is per filter: one misspelled filter errors
/// even when another filter matches, naming exactly the misspelled one.
#[test]
fn one_bad_filter_errors_even_beside_a_good_one() {
    let roster = [spec("alpha_join"), spec("beta_join")];
    let filters = ["join".to_string(), "gamma".to_string()];
    let Err(err) = select(&roster, &filters) else {
        panic!("the second filter matches nothing, so selection must error");
    };
    let message = err.to_string();
    assert!(message.contains("\"gamma\""));
    assert!(
        !message.contains("\"join\""),
        "the matching filter must not be reported as unmatched"
    );
}
