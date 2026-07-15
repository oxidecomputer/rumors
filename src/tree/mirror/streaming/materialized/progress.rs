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
    /// Five checks: every internal publication consumes a prior wire
    /// action for its scope (wire before internal publication); dependent
    /// work follows its scope's resolution, exactly `pending` items per
    /// resolution (resolution before dependent work); a parent resolution
    /// follows the lower resolutions it counts; a resolution may not
    /// arrive while an already-resolved sibling still owes dependent work
    /// (sibling contiguity); and each event kind leaves a parent scope in
    /// strictly increasing radix order (radix order). Sibling contiguity
    /// is what makes one slot sufficient for the child-resolution queues:
    /// without it, a walk that published all its resolutions before any
    /// of their queries would satisfy the other checks and deadlock.
    /// Radix order is what positional pairing rests on: no message or
    /// return carries a key, so a consumer's only way to know which scope
    /// the k-th item describes is that producers never reorder within a
    /// channel.
    pub fn assert_valid(&self) {
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
                Kind::Wire => *wires.entry(key).or_default() += 1,
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
