use super::{Event, Kind, Trace};

/// Build one trace event for the test endpoint.
fn event(scope: &[u8], kind: Kind) -> Event {
    Event {
        work: 0,
        scope: scope.to_vec(),
        kind,
    }
}

/// The complete wire-to-parent event chain satisfies every publication dependency.
#[test]
fn accepts_wire_resolution_work_parent_order() {
    Trace(vec![
        event(&[1], Kind::Wire),
        event(&[1], Kind::Resolution { pending: 1 }),
        event(&[1, 2], Kind::DependentWork),
        event(&[], Kind::ParentResolution { pending: 1 }),
    ])
    .assert_valid();
}

/// Internal readiness cannot be published before its corresponding wire action.
#[test]
#[should_panic(expected = "preceded its wire action")]
fn rejects_internal_publication_before_wire() {
    Trace(vec![event(&[1], Kind::Ready)]).assert_valid();
}

/// Work below a scope cannot begin until that scope's resolution makes it meaningful.
#[test]
#[should_panic(expected = "preceded its resolution")]
fn rejects_dependent_work_before_resolution() {
    Trace(vec![event(&[1, 2], Kind::DependentWork)]).assert_valid();
}

/// A parent cannot resolve while any lower resolution it counts remains outstanding.
#[test]
#[should_panic(expected = "preceded its 1 lower resolutions")]
fn rejects_parent_before_lower_resolution() {
    Trace(vec![event(&[], Kind::ParentResolution { pending: 1 })]).assert_valid();
}

/// A scope cannot resolve while an already-resolved sibling still owes work.
///
/// Without sibling contiguity, publishing every resolution before any query
/// satisfies the other checks and deadlocks the cap-1 child-resolution queue.
#[test]
#[should_panic(expected = "still owes 1 dependent work items")]
fn rejects_sibling_resolution_before_dependent_work() {
    Trace(vec![
        event(&[1], Kind::Wire),
        event(&[1], Kind::Resolution { pending: 1 }),
        event(&[2], Kind::Wire),
        event(&[2], Kind::Resolution { pending: 0 }),
    ])
    .assert_valid_without_wire_contiguity();
}

/// A wire may not depart while a resolved sibling still owes dependent work.
///
/// A wire stream that runs ahead of its siblings' dependent work satisfies the
/// other checks and deadlocks a three-walk wait cycle at uneven fan. The
/// kernel-checked witness is the Lean control theorem
/// `Control.jam_not_deadlockFree`.
#[test]
#[should_panic(expected = "departed while resolved sibling")]
fn rejects_wire_while_sibling_owes_dependent_work() {
    Trace(vec![
        event(&[1], Kind::Wire),
        event(&[1], Kind::Resolution { pending: 1 }),
        event(&[2], Kind::Wire),
    ])
    .assert_valid();
}

/// A wire may not overtake an earlier disputed sibling's resolution.
///
/// This covers the case where the later sibling's wire departs before the
/// earlier sibling resolves, so at wire time the earlier sibling owes
/// nothing yet — only the completed trace reveals it was disputed. Without
/// this arm the owes-work check alone would pass the deadlocking order.
#[test]
#[should_panic(expected = "preceded disputed sibling")]
fn rejects_wire_before_earlier_sibling_resolution() {
    Trace(vec![
        event(&[1], Kind::Wire),
        event(&[2], Kind::Wire),
        event(&[1], Kind::Resolution { pending: 0 }),
        event(&[2], Kind::Resolution { pending: 0 }),
    ])
    .assert_valid();
}

/// A parent resolution may not depart while a wire of its scope is unsent.
///
/// The parent summary is the scope's last publication. This trace otherwise
/// satisfies wire contiguity and radix order, so it isolates that rule through
/// the normal validator.
#[test]
#[should_panic(expected = "the parent summary is the scope's last publication")]
fn rejects_parent_before_trailing_wire() {
    Trace(vec![
        event(&[1], Kind::Wire),
        event(&[1], Kind::Resolution { pending: 0 }),
        event(&[], Kind::ParentResolution { pending: 1 }),
        event(&[2], Kind::Wire),
        event(&[2], Kind::Resolution { pending: 0 }),
    ])
    .assert_valid();
}

/// A parent resolution may not depart while a disputed child is unresolved.
///
/// The second arm of parent placement: only the completed trace reveals
/// the late child was disputed, so the check must read hindsight, exactly
/// like wire contiguity. Exercised directly: the same trace also trips
/// wire contiguity's earlier-sibling arm inside `assert_valid`.
#[test]
#[should_panic(expected = "departed before a disputed child's resolution")]
fn rejects_parent_before_disputed_child_resolution() {
    Trace(vec![
        event(&[1], Kind::Wire),
        event(&[2], Kind::Wire),
        event(&[1], Kind::Resolution { pending: 0 }),
        event(&[], Kind::ParentResolution { pending: 1 }),
        event(&[2], Kind::Resolution { pending: 0 }),
    ])
    .assert_parent_last();
}

/// A parent resolution may not depart while a child owes dependent work.
///
/// The third arm of parent placement: the child resolved, but its
/// dependent-work quota is unfilled at parent departure. Exercised
/// directly: the trailing dependent-work item also trips sibling
/// contiguity ordering inside `assert_valid`.
#[test]
#[should_panic(expected = "still owes 1 dependent work items")]
fn rejects_parent_while_child_owes_dependent_work() {
    Trace(vec![
        event(&[1], Kind::Wire),
        event(&[1], Kind::Resolution { pending: 1 }),
        event(&[], Kind::ParentResolution { pending: 1 }),
        event(&[1, 2], Kind::DependentWork),
    ])
    .assert_parent_last();
}

/// Publications for one scope's children must leave in radix order.
///
/// Positional pairing is the protocol's only correlation mechanism: no
/// message or return carries a key, so a consumer knows the k-th item's
/// scope only because producers never reorder within a channel.
#[test]
#[should_panic(expected = "violates radix order")]
fn rejects_out_of_order_sibling_wires() {
    Trace(vec![event(&[2], Kind::Wire), event(&[1], Kind::Wire)]).assert_valid();
}

/// Radix order applies per event kind, including dependent work slots.
#[test]
#[should_panic(expected = "violates radix order")]
fn rejects_out_of_order_dependent_work() {
    Trace(vec![
        event(&[1], Kind::Wire),
        event(&[1], Kind::Resolution { pending: 2 }),
        event(&[1, 2], Kind::DependentWork),
        event(&[1, 1], Kind::DependentWork),
    ])
    .assert_valid();
}
