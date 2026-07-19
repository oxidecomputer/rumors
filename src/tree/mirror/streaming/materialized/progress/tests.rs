use super::{Event, Kind, Trace};

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

/// Internal readiness before the corresponding wire action violates write-before-publish.
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
/// The wire-stream twin of sibling contiguity (finding #6): a wire stream
/// that runs ahead of its siblings' dependent work satisfies the other
/// checks and deadlocks a three-walk wait cycle at uneven fan. The
/// kernel-checked witness is `formal/lean/StreamingMirror/Controls.lean`.
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
/// The other arm of wire contiguity (finding #6): the deadlock witness
/// sends wire B2 *before* res B1, so at wire time the earlier sibling owes
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

/// The encoder's publication order rejects the parent-early (d5) discipline.
///
/// This trace is the encoder's own order (the same trace
/// `accepts_wire_resolution_work_parent_order` accepts): the sole disputed
/// child's dependent work departs after the final resolution and before the
/// parent summary, exactly what the weave's d5 placement forbids. Pinned as
/// the design-space record (finding #7, adjudicated: the encoder keeps the
/// epilogue placement and the `d6`/`assert_parent_last` check instead —
/// `design/parent-placement.md`): if this test starts failing because the
/// panic disappears, the encoder's order changed corners — re-audit against
/// the design doc before accepting.
#[test]
#[should_panic(expected = "with the parent summary unsent")]
fn real_encoder_order_violates_parent_early_discipline() {
    Trace(vec![
        event(&[1], Kind::Wire),
        event(&[1], Kind::Resolution { pending: 1 }),
        event(&[1, 2], Kind::DependentWork),
        event(&[], Kind::ParentResolution { pending: 1 }),
    ])
    .assert_parent_early();
}

/// A parent resolution may not depart while a wire of its scope is unsent.
///
/// The first arm of parent placement (finding #7, the `d6` ledger): the
/// parent summary is the scope's last publication. This trace passes every
/// older check (the trailing wire is contiguity-clean and radix-ordered),
/// so the arm adds real coverage; it goes through `assert_valid` to prove
/// the check is wired in.
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
