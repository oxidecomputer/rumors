use super::{listing, select};
use crate::ops::{Compensation, Inputs, OpSpec, Operand, ROSTER};

/// Construct an operation whose name is the only meaningful field.
fn spec(name: &'static str) -> OpSpec {
    OpSpec {
        name,
        inputs: Inputs::Operands(&[Operand::Version]),
        covers: &["unused"],
        size_measure: "unused",
        variant: "",
        contract: "unused",
        compensation: Compensation::None,
        measure: |_, _, _| unreachable!("selection never measures"),
    }
}

/// Collect selected operation names for direct comparison.
fn names(selected: &[&OpSpec]) -> Vec<&'static str> {
    selected.iter().map(|op| op.name).collect()
}

/// An empty filter list selects every operation in declaration order.
#[test]
fn no_filters_selects_the_whole_roster() {
    let selected = select(ROSTER, &[]).expect("the empty filter list always selects");
    let roster_names: Vec<&str> = ROSTER.iter().map(|op| op.name).collect();
    assert_eq!(names(&selected), roster_names);
}

/// Listing writes each selected operation name on its own line.
#[test]
fn listing_is_the_selection_one_name_per_line() {
    let selected = select(ROSTER, &[]).expect("the empty filter list always selects");
    let expected: String = ROSTER.iter().map(|op| format!("{}\n", op.name)).collect();
    assert_eq!(listing(&selected), expected);
}

/// A filter selects matching substrings without changing declaration order.
#[test]
fn a_filter_selects_by_substring_in_roster_order() {
    let roster = [spec("alpha_join"), spec("alpha_meet"), spec("beta_join")];
    let selected = select(&roster, &["join".to_string()]).expect("two rows match");
    assert_eq!(names(&selected), ["alpha_join", "beta_join"]);
}

/// Multiple filters form an ordered union without duplicate operations.
#[test]
fn filters_union_without_duplicates() {
    let roster = [spec("alpha_join"), spec("alpha_meet"), spec("beta_join")];
    let filters = ["join".to_string(), "alpha".to_string()];
    let selected = select(&roster, &filters).expect("every filter matches");
    assert_eq!(names(&selected), ["alpha_join", "alpha_meet", "beta_join"]);
}

/// An unmatched filter reports itself and every available operation.
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

/// One unmatched filter fails the selection even when another filter matches.
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
