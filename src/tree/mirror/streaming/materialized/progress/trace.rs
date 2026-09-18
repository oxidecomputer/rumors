//! Test-only trace storage and validation for the materialized walk.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

/// The kind of one observable publication in a work graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Kind {
    /// An outgoing wire action for a scope, recorded before its internal twin.
    Wire,
    /// The initiator's single root query.
    InitialQuery,
    /// An answerer-side scope resolution, with its `Pending` slot count.
    Resolution {
        /// How many of the scope's children await lower stages.
        pending: usize,
    },
    /// One dependent work item (a child query) issued below a resolution.
    DependentWork,
    /// A whole-subtree provision resolved in place (the supplier's side of a
    /// one-sided request).
    Ready,
    /// An asker-side parent summary, with its `Pending` slot count.
    ParentResolution {
        /// How many of the scope's children await lower stages.
        pending: usize,
    },
}

/// One recorded publication: which endpoint, at which scope, of what kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    /// The endpoint-local work identity ([`new_work`]).
    pub work: usize,
    /// The scope's byte prefix (empty for the root).
    pub scope: Vec<u8>,
    /// The publication kind.
    pub kind: Kind,
}

/// The ordering trace from a completed session.
#[derive(Debug, Eq, PartialEq)]
pub struct Trace(pub(super) Vec<Event>);

/// Validate a completed publication trace.
impl Trace {
    /// The recorded publications, in publication order.
    pub fn events(&self) -> &[Event] {
        &self.0
    }

    /// Check the publication order that keeps the walk live and correctly paired.
    ///
    /// - **wire before internal publication**: every internal publication
    ///   consumes a prior wire action for its scope;
    /// - **resolution before dependent work**: dependent work follows its
    ///   scope's resolution, exactly `pending` items per resolution;
    /// - **lower resolutions before parent**: a parent resolution follows
    ///   the lower resolutions it counts;
    /// - **sibling contiguity**: a resolution may not arrive while an
    ///   already-resolved sibling still owes dependent work;
    /// - **wire contiguity**: a wire may not depart while an earlier
    ///   disputed sibling is unresolved or any resolved sibling still owes
    ///   dependent work;
    /// - **radix order**: each event kind leaves a parent scope in strictly
    ///   increasing radix order;
    /// - **parent placement**: a parent resolution is its scope's last
    ///   publication — it may not depart while any wire of its scope is
    ///   unsent, any disputed child's resolution is unsent, or any
    ///   resolved child's dependent-work quota is unfilled.
    ///
    /// Sibling and wire contiguity prevent a stage from filling one bounded
    /// edge while withholding the work that drains another. The Lean theorem
    /// `Control.jam_not_deadlockFree` gives a deadlocking schedule when wire
    /// contiguity is omitted. Radix order preserves positional pairing because
    /// protocol items do not carry their scope keys. Parent-last placement is
    /// the remaining schedule premise used by `Sched.deadlock_free`.
    pub fn assert_valid(&self) {
        self.assert_valid_with_wire_contiguity(true);
        self.assert_parent_last();
    }

    /// Check every invariant except wire contiguity.
    ///
    /// This keeps sibling contiguity independently falsifiable even though
    /// wire contiguity subsumes it on a complete, valid trace.
    pub(super) fn assert_valid_without_wire_contiguity(&self) {
        self.assert_valid_with_wire_contiguity(false);
    }

    /// Check that each parent summary follows all work needed to assemble it.
    ///
    /// This is the parent-last premise of `Sched.deadlock_free`: every wire,
    /// disputed-child resolution, and dependent work item owned by a scope
    /// must precede that scope's parent resolution.
    pub(super) fn assert_parent_last(&self) {
        // The complete trace identifies disputed children and each scope's
        // last wire; neither is known when the parent event is recorded.
        let mut last_wire = BTreeMap::<(usize, Vec<u8>), usize>::new();
        let mut last_resolution = BTreeMap::<(usize, Vec<u8>), usize>::new();
        for (index, event) in self.0.iter().enumerate() {
            if let Some(scope_parent) = parent(&event.scope) {
                let key = (event.work, scope_parent);
                match event.kind {
                    Kind::Wire => {
                        last_wire.insert(key, index);
                    }
                    Kind::Resolution { .. } => {
                        last_resolution.insert(key, index);
                    }
                    _ => {}
                }
            }
        }

        let mut owed = BTreeMap::<(usize, Vec<u8>), usize>::new();
        for (index, event) in self.0.iter().enumerate() {
            match event.kind {
                Kind::Resolution { pending } => {
                    owed.insert((event.work, event.scope.clone()), pending);
                }
                Kind::DependentWork => {
                    if let Some(scope_parent) = parent(&event.scope)
                        && let Some(remaining) = owed.get_mut(&(event.work, scope_parent))
                    {
                        *remaining = remaining.saturating_sub(1);
                    }
                }
                Kind::ParentResolution { .. } => {
                    let key = (event.work, event.scope.clone());
                    if let Some(wire_at) = last_wire.get(&key)
                        && *wire_at > index
                    {
                        panic!(
                            "parent resolution {event:?} at trace index {index} departed before its scope's wire at index {wire_at}: the parent summary is the scope's last publication"
                        );
                    }
                    if let Some(resolved_at) = last_resolution.get(&key)
                        && *resolved_at > index
                    {
                        panic!(
                            "parent resolution {event:?} at trace index {index} departed before a disputed child's resolution at index {resolved_at}"
                        );
                    }
                    let owing = owed.iter().find(|((work, child), remaining)| {
                        *work == event.work
                            && **remaining > 0
                            && parent(child) == Some(event.scope.clone())
                    });
                    if let Some(((_, child), remaining)) = owing {
                        panic!(
                            "parent resolution {event:?} at trace index {index} departed while child {child:?} still owes {remaining} dependent work items"
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// Check the trace, optionally omitting the stronger wire-level check.
    fn assert_valid_with_wire_contiguity(&self, check_wire_contiguity: bool) {
        // The completed trace reveals whether an earlier sibling eventually
        // resolves and was therefore disputed.
        let mut resolutions_at = BTreeMap::<(usize, Vec<u8>), usize>::new();
        for (index, event) in self.0.iter().enumerate() {
            if let Kind::Resolution { .. } = event.kind {
                resolutions_at.insert((event.work, event.scope.clone()), index);
            }
        }

        let mut wires = BTreeMap::<(usize, Vec<u8>), usize>::new();
        let mut dependent = BTreeMap::<(usize, Vec<u8>), usize>::new();
        let mut lower = BTreeMap::<(usize, Vec<u8>), usize>::new();
        let mut wire_order = BTreeMap::<(usize, Vec<u8>), u8>::new();
        let mut resolution_order = BTreeMap::<(usize, Vec<u8>), u8>::new();
        let mut dependent_order = BTreeMap::<(usize, Vec<u8>), u8>::new();
        let mut parent_order = BTreeMap::<(usize, Vec<u8>), u8>::new();

        for (index, event) in self.0.iter().enumerate() {
            match event.kind {
                Kind::Wire => in_radix_order(&mut wire_order, event, index),
                Kind::Resolution { .. } => in_radix_order(&mut resolution_order, event, index),
                Kind::DependentWork => in_radix_order(&mut dependent_order, event, index),
                Kind::ParentResolution { .. } => in_radix_order(&mut parent_order, event, index),
                Kind::InitialQuery | Kind::Ready => {}
            }

            let key = (event.work, event.scope.clone());
            match event.kind {
                Kind::Wire => {
                    if check_wire_contiguity && let Some(scope_parent) = parent(&event.scope) {
                        let owing = dependent.iter().find(|((work, sibling), remaining)| {
                            *work == event.work
                                && **remaining > 0
                                && *sibling != event.scope
                                && parent(sibling) == Some(scope_parent.clone())
                        });
                        if let Some(((_, sibling), remaining)) = owing {
                            panic!(
                                "wire {event:?} at trace index {index} departed while resolved sibling {sibling:?} still owes {remaining} dependent work items"
                            );
                        }
                        let unresolved =
                            resolutions_at
                                .iter()
                                .find(|((work, sibling), resolved_at)| {
                                    *work == event.work
                                        && *sibling != event.scope
                                        && parent(sibling) == Some(scope_parent.clone())
                                        && sibling.last() < event.scope.last()
                                        && **resolved_at > index
                                });
                        if let Some(((_, sibling), _)) = unresolved {
                            panic!(
                                "wire {event:?} at trace index {index} preceded disputed sibling {sibling:?}'s resolution"
                            );
                        }
                    }
                    *wires.entry(key).or_default() += 1
                }
                Kind::InitialQuery | Kind::Ready | Kind::Resolution { .. } => {
                    let available = wires.entry(key.clone()).or_default();
                    assert!(
                        *available > 0,
                        "internal publication {event:?} at trace index {index} preceded its wire action"
                    );
                    *available -= 1;

                    if let Kind::Resolution { pending } = event.kind {
                        if let Some(scope_parent) = parent(&event.scope) {
                            let owing = dependent.iter().find(|((work, sibling), remaining)| {
                                *work == event.work
                                    && **remaining > 0
                                    && *sibling != event.scope
                                    && parent(sibling) == Some(scope_parent.clone())
                            });
                            if let Some(((_, sibling), remaining)) = owing {
                                panic!(
                                    "resolution {event:?} at trace index {index} arrived while resolved sibling {sibling:?} still owes {remaining} dependent work items"
                                );
                            }
                        }
                        dependent.insert(key.clone(), pending);
                        if let Some(parent) = parent(&event.scope) {
                            *lower.entry((event.work, parent)).or_default() += 1;
                        }
                    }
                }
                Kind::DependentWork => {
                    let parent = parent(&event.scope).expect("dependent work is below a scope");
                    let key = (event.work, parent);
                    let available = dependent.get_mut(&key).unwrap_or_else(|| {
                        panic!(
                            "dependent work {event:?} at trace index {index} preceded its resolution"
                        )
                    });
                    assert!(
                        *available > 0,
                        "too much dependent work for its resolution at trace index {index}: {event:?}"
                    );
                    *available -= 1;
                }
                Kind::ParentResolution { pending } => {
                    let available = lower.entry(key).or_default();
                    assert!(
                        *available >= pending,
                        "parent resolution {event:?} at trace index {index} preceded its {pending} lower resolutions"
                    );
                    *available -= pending;
                }
            }
        }

        assert!(
            dependent.values().all(|remaining| *remaining == 0),
            "some resolutions did not publish all dependent work: {dependent:?}"
        );
        assert!(
            wires.values().all(|remaining| *remaining == 0),
            "some wire actions had no corresponding internal publication: {wires:?}"
        );
    }
}

// Clippy's `missing_const_for_thread_local` can reject const-block initializers
// on targets that lower `thread_local!` through fallback TLS. The allow keeps
// `-D warnings` clean on those targets.
std::thread_local! {
    /// Publications captured by the current trace scope.
    #[allow(clippy::missing_const_for_thread_local)]
    static EVENTS: RefCell<Option<Vec<Event>>> = const { RefCell::new(None) };
    /// Next endpoint identity within the current trace scope.
    #[allow(clippy::missing_const_for_thread_local)]
    static NEXT_WORK: Cell<usize> = const { Cell::new(0) };
}

/// Run `f` while tracing every materialized publication it creates.
pub fn with_trace<R>(f: impl FnOnce() -> R) -> (R, Trace) {
    /// Restores an enclosing trace when this scope ends or unwinds.
    struct Restore {
        /// Publications captured by the enclosing scope.
        events: Option<Vec<Event>>,
        /// The enclosing scope's next endpoint identity.
        next_work: usize,
    }

    /// Restores the thread-local recorder.
    impl Drop for Restore {
        /// Reinstate the enclosing trace state.
        fn drop(&mut self) {
            EVENTS.with(|events| events.replace(self.events.take()));
            NEXT_WORK.with(|next| next.set(self.next_work));
        }
    }

    let previous = EVENTS.with(|events| events.replace(Some(Vec::new())));
    let previous_next = NEXT_WORK.with(|next| next.replace(0));
    let restore = Restore {
        events: previous,
        next_work: previous_next,
    };
    let result = f();
    let events = EVENTS.with(|events| events.take().unwrap_or_default());
    drop(restore);
    (result, Trace(events))
}

/// Allocate one endpoint-local work identity.
pub(super) fn new_work() -> usize {
    NEXT_WORK.with(|next| {
        let work = next.get();
        next.set(work + 1);
        work
    })
}

/// Append a publication when a trace is active on this thread.
pub(super) fn record(work: usize, scope: &[u8], kind: Kind) {
    EVENTS.with(|events| {
        if let Some(events) = events.borrow_mut().as_mut() {
            events.push(Event {
                work,
                scope: scope.to_vec(),
                kind,
            });
        }
    });
}

/// Return the parent prefix of a non-root scope.
fn parent(scope: &[u8]) -> Option<Vec<u8>> {
    scope.split_last().map(|(_, parent)| parent.to_vec())
}

/// Panic unless the event's scope strictly exceeds, in final radix, every
/// same-kind event already seen under the same parent.
///
/// Root-scoped events have no parent and no radix; they are exempt.
fn in_radix_order(ledger: &mut BTreeMap<(usize, Vec<u8>), u8>, event: &Event, index: usize) {
    let Some((radix, parent)) = event.scope.split_last() else {
        return;
    };
    let key = (event.work, parent.to_vec());
    if let Some(previous) = ledger.get(&key) {
        assert!(
            radix > previous,
            "event {event:?} at trace index {index} violates radix order: an event of its kind already left this scope at radix {previous:#04x}"
        );
    }
    ledger.insert(key, *radix);
}
