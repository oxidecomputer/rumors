//! Test-only trace of the walk's progress-critical publications.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        materialized::{Query, Resolution, Resolve},
    },
    typed::{
        Prefix,
        height::{Height, S, Z},
    },
};

/// The kind of one observable publication in a work graph.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Kind {
    Wire,
    InitialQuery,
    Resolution { pending: usize },
    DependentWork,
    Ready,
    ParentResolution { pending: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Event {
    work: usize,
    scope: Vec<u8>,
    kind: Kind,
}

/// A completed positive session's ordering trace.
#[derive(Debug)]
pub struct Trace(Vec<Event>);

impl Trace {
    /// Check the publication-order invariants for every traced scope.
    ///
    /// Seven checks: every internal publication consumes a prior wire
    /// action for its scope (wire before internal publication); dependent
    /// work follows its scope's resolution, exactly `pending` items per
    /// resolution (resolution before dependent work); a parent resolution
    /// follows the lower resolutions it counts; a resolution may not
    /// arrive while an already-resolved sibling still owes dependent work
    /// (sibling contiguity); a wire may not depart while an earlier
    /// disputed sibling is unresolved or any resolved sibling still owes
    /// dependent work (wire contiguity); each event kind leaves a
    /// parent scope in strictly increasing radix order (radix order);
    /// and a parent resolution is its scope's last publication (parent
    /// placement): it may not depart while any wire of its scope is
    /// unsent, any disputed child's resolution is unsent, or any resolved
    /// child's dependent-work quota is unfilled.
    /// Sibling contiguity is what makes one slot sufficient for the
    /// child-resolution queues: without it, a walk that published all its
    /// resolutions before any of their queries would satisfy the other
    /// checks and deadlock. Wire contiguity is its wire-stream twin
    /// (finding #6): without it, a wire stream that runs ahead of an
    /// earlier sibling's resolution or queries satisfies the other checks
    /// and deadlocks a three-walk wait cycle at uneven fan — the
    /// kernel-checked witness is
    /// `formal/lean/StreamingMirror/Controls.lean`. On wire-disciplined
    /// traces wire contiguity subsumes sibling contiguity; both stay, as
    /// independent statements of intent. Radix order is what positional
    /// pairing rests on: no message or return carries a key, so a
    /// consumer's only way to know which scope the k-th item describes is
    /// that producers never reorder within a channel. Parent placement
    /// (finding #7) is the `d6` ordering ledger of the formal model
    /// (`formal/PROGRESS.md` §8): the local invariant under which the
    /// `AxMode.impl` deadlock-freedom theorem holds, mirrored here so the
    /// encoder's traces pin exactly the discipline the proof consumes.
    pub fn assert_valid(&self) {
        self.assert_valid_with_wire_contiguity(true);
        self.assert_parent_last();
    }

    /// Check every invariant except wire contiguity.
    ///
    /// This test-only entry point keeps the sibling-contiguity check
    /// independently falsifiable even though wire contiguity subsumes it on a
    /// complete, valid trace.
    #[cfg(test)]
    fn assert_valid_without_wire_contiguity(&self) {
        self.assert_valid_with_wire_contiguity(false);
    }

    /// The parent-placement check (finding #7): a parent resolution is
    /// its scope's last publication.
    ///
    /// This is the `d6` (epilogue-placement) ordering ledger of the
    /// formal model, mirrored verbatim (`formal/PROGRESS.md` §8): a
    /// parent summary that departs while any wire of its scope is
    /// unsent, or while any disputed child's resolution is unsent or its
    /// dependent-work quota not fully issued, is a violation. This is
    /// the discipline the encoder actually follows (the scope epilogue's
    /// "Launch every `Pending` slot's work before publishing its
    /// enclosing parent resolution" placement in levels.rs), and the
    /// local invariant the `AxMode.impl` deadlock-freedom theorem
    /// consumes. Its deliberate opposite, the weave's parent-early
    /// discipline, is documented by [`Self::assert_parent_early`]; the
    /// design trade between the two corners is
    /// `design/parent-placement.md`.
    fn assert_parent_last(&self) {
        // Like wire contiguity, the check needs the completed trace: a
        // scope's wire and resolution sets are known only in hindsight
        // (a child is disputed iff it ever resolves).
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

    /// The parent-EARLY discipline (the formal model's `d5` ledger, the
    /// weave's placement) — deliberately NOT wired into `assert_valid`:
    /// the encoder does not and should not satisfy it.
    ///
    /// D5 as minted: once the resolution of a scope's last disputed child
    /// has been emitted, any further wire or query of that scope before
    /// the parent summary is a violation; a scope with no disputed
    /// children must emit its parent before any wire or query. The
    /// encoder deliberately violates this order (`yield_resolve_query!`
    /// publishes each child's queries immediately after its resolution,
    /// and the parent resolution departs only in the scope epilogue),
    /// trading the d5 corner's any-capacity deadlock freedom for maximal
    /// descent/assembly pipelining under the assembler capacity floor —
    /// the adjudicated design decision recorded in
    /// `design/parent-placement.md`, with the capacity-universal theorem
    /// for this discipline kept as `Sched.deadlock_free_d5` in the
    /// formal model. Retained as the design-space record; the pin
    /// documenting that the encoder's order rejects it is
    /// `real_encoder_order_violates_parent_early_discipline`.
    #[cfg(test)]
    fn assert_parent_early(&self) {
        // Like wire contiguity, the check needs the completed trace to
        // know each scope's disputed-children set: disputed iff it ever
        // resolves.
        let mut disputed = BTreeMap::<(usize, Vec<u8>), usize>::new();
        for event in &self.0 {
            if let Kind::Resolution { .. } = event.kind
                && let Some(scope_parent) = parent(&event.scope)
            {
                *disputed.entry((event.work, scope_parent)).or_default() += 1;
            }
        }

        let mut resolved = BTreeMap::<(usize, Vec<u8>), usize>::new();
        let mut parent_done = BTreeMap::<(usize, Vec<u8>), bool>::new();
        for (index, event) in self.0.iter().enumerate() {
            // The scope whose walk owns this event, per the model mapping:
            // a wire at child c belongs to scope parent(c); a dependent
            // query at grandchild g belongs to scope parent(parent(g)).
            // Root-owned events (no such scope) are the openers' domain,
            // outside d5.
            let owner = match event.kind {
                Kind::Wire => parent(&event.scope),
                Kind::DependentWork => parent(&event.scope).as_deref().and_then(parent),
                _ => None,
            };
            if let Some(scope) = owner {
                let key = (event.work, scope);
                let all_resolved = resolved.get(&key).copied().unwrap_or(0)
                    == disputed.get(&key).copied().unwrap_or(0);
                if all_resolved && !parent_done.get(&key).copied().unwrap_or(false) {
                    panic!(
                        "event {event:?} at trace index {index} departed after scope {:?}'s final resolution with the parent summary unsent",
                        key.1,
                    );
                }
            }
            match event.kind {
                Kind::Resolution { .. } => {
                    if let Some(scope_parent) = parent(&event.scope) {
                        *resolved.entry((event.work, scope_parent)).or_default() += 1;
                    }
                }
                Kind::ParentResolution { .. } => {
                    parent_done.insert((event.work, event.scope.clone()), true);
                }
                _ => {}
            }
        }
    }

    /// Check the trace, optionally omitting the stronger wire-level check.
    fn assert_valid_with_wire_contiguity(&self, check_wire_contiguity: bool) {
        // Wire contiguity's "unresolved earlier sibling" arm needs to know
        // which scopes are disputed at all, which only the completed trace
        // can say: a scope is disputed iff it ever resolves.
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

std::thread_local! {
    static EVENTS: RefCell<Option<Vec<Event>>> = const { RefCell::new(None) };
    static NEXT_WORK: Cell<usize> = const { Cell::new(0) };
}

/// Run `f` while tracing every materialized publication it creates.
pub fn with_trace<R>(f: impl FnOnce() -> R) -> (R, Trace) {
    struct Restore {
        events: Option<Vec<Event>>,
        next_work: usize,
    }

    impl Drop for Restore {
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

pub(super) fn wire<H: Height>(work: usize, scope: Prefix<H>) {
    record(work, scope, Kind::Wire);
}

pub(super) fn initial_query<B, T, H>(work: usize, query: &Query<B, T, H>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    record(work, query.prefix, Kind::InitialQuery);
}

pub(super) fn resolution<B, T, H>(work: usize, resolution: &Resolution<B, T, H>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    record(
        work,
        resolution.prefix,
        Kind::Resolution {
            pending: pending(&resolution.resolved),
        },
    );
}

pub(super) trait Scoped {
    fn scope(&self) -> &[u8];
}

impl<H: Height> Scoped for Prefix<H> {
    fn scope(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<B, T, H> Scoped for Query<B, T, H>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    fn scope(&self) -> &[u8] {
        self.prefix.as_bytes()
    }
}

pub(super) fn dependent(work: usize, item: &impl Scoped) {
    record_bytes(work, item.scope(), Kind::DependentWork);
}

pub(super) fn ready<H: Height>(work: usize, scope: Prefix<H>) {
    record(work, scope, Kind::Ready);
}

pub(super) fn parent_resolution<B, T, H>(work: usize, resolution: &Resolution<B, T, H>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    record(
        work,
        resolution.prefix,
        Kind::ParentResolution {
            pending: pending(&resolution.resolved),
        },
    );
}

fn pending<B, T, H>(resolved: &[(u8, Resolve<B, T, H>)]) -> usize
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    resolved
        .iter()
        .filter(|(_, slot)| matches!(slot, Resolve::Pending))
        .count()
}

fn record<H: Height>(work: usize, scope: Prefix<H>, kind: Kind) {
    record_bytes(work, scope.as_bytes(), kind);
}

fn record_bytes(work: usize, scope: &[u8], kind: Kind) {
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

#[cfg(test)]
mod tests;
