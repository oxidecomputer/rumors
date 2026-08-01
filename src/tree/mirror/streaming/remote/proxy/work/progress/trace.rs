use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};

use crate::tree::typed::height::{Height as _, UnderRoot};

/// One progress-critical proxy publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    WireReply { questions: usize },
    LocalQuestion,
    DecodedReply { scopes: usize },
    NextScope,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Event {
    work: usize,
    height: usize,
    kind: Kind,
}

/// A completed positive session's proxy-ordering trace.
#[derive(Debug)]
pub struct Trace(Vec<Event>);

impl Trace {
    /// Assert wire-before-question and reply-before-scope ordering.
    pub fn assert_valid(&self) {
        let mut questions = BTreeMap::<(usize, usize), usize>::new();
        let mut scopes = BTreeMap::<(usize, usize), usize>::new();
        for (index, event) in self.0.iter().enumerate() {
            match event.kind {
                Kind::WireReply { questions: count } => {
                    assert_drained(&questions, event, index, "questions");
                    questions.insert((event.work, event.height), count);
                }
                Kind::LocalQuestion => consume(&mut questions, event, index, "wire reply"),
                Kind::DecodedReply { scopes: count } => {
                    assert_drained(&scopes, event, index, "scopes");
                    scopes.insert((event.work, event.height), count);
                }
                Kind::NextScope => consume(&mut scopes, event, index, "decoded reply"),
            }
        }
        assert!(
            questions.values().all(|remaining| *remaining == 0),
            "some wire replies did not publish every question: {questions:?}",
        );
        assert!(
            scopes.values().all(|remaining| *remaining == 0),
            "some decoded replies did not publish every scope: {scopes:?}",
        );
    }

    /// Assert context-registration causality: no decoded reply overtakes
    /// the flushed local question whose scope interprets it.
    ///
    /// Decoding pairs replies with scopes positionally off a FIFO whose
    /// entries are registered at encode time, after the question's complete
    /// wire reply flushed. So at every trace prefix, per endpoint, the
    /// decode count at a height is bounded by the flushed-question count at
    /// the height it consumes scopes from:
    ///
    /// - a reply decoded at height `h` consumed a question at `h + 1` (the
    ///   scope it was asked under);
    /// - leaf-height decodes drain both the last internal stage's height-1
    ///   scopes and the terminal height-0 leaf questions, so their bound is
    ///   the sum;
    /// - the single greeting-seeded opening (recorded at the under-root
    ///   height, the one reply with no wire frame at all) is scoped by the
    ///   greeting itself.
    ///
    /// A violation means a reply was interpreted by a scope its own
    /// question had not yet made publishable: the receive-side complement
    /// of the wire-before-question ordering
    /// [`assert_valid`](Self::assert_valid) pins on the send side.
    pub fn assert_registration_causality(&self) {
        let mut questions = BTreeMap::<(usize, usize), usize>::new();
        let mut decoded = BTreeMap::<(usize, usize), usize>::new();
        for (index, event) in self.0.iter().enumerate() {
            match event.kind {
                Kind::LocalQuestion => {
                    *questions.entry((event.work, event.height)).or_insert(0) += 1;
                }
                Kind::DecodedReply { .. } => {
                    let count = decoded.entry((event.work, event.height)).or_insert(0);
                    *count += 1;
                    let flushed =
                        |height: usize| questions.get(&(event.work, height)).copied().unwrap_or(0);
                    let available = if event.height == UnderRoot::HEIGHT {
                        1
                    } else if event.height == 0 {
                        flushed(0) + flushed(1)
                    } else {
                        flushed(event.height + 1)
                    };
                    assert!(
                        *count <= available,
                        "event {event:?} at trace index {index}: decoded reply {count} \
                         arrived before the question that scopes it was flushed \
                         ({available} available)",
                    );
                }
                Kind::WireReply { .. } | Kind::NextScope => {}
            }
        }
    }
}

fn assert_drained(
    ledger: &BTreeMap<(usize, usize), usize>,
    event: &Event,
    index: usize,
    items: &str,
) {
    let remaining = ledger
        .get(&(event.work, event.height))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        remaining, 0,
        "event {event:?} at trace index {index} overtook {remaining} prior {items}",
    );
}

fn consume(
    ledger: &mut BTreeMap<(usize, usize), usize>,
    event: &Event,
    index: usize,
    prerequisite: &str,
) {
    let remaining = ledger
        .get_mut(&(event.work, event.height))
        .unwrap_or_else(|| {
            panic!("event {event:?} at trace index {index} preceded its {prerequisite}")
        });
    assert!(
        *remaining > 0,
        "event {event:?} at trace index {index} exceeded its {prerequisite}'s count",
    );
    *remaining -= 1;
}

// clippy's `missing_const_for_thread_local` misreads `thread_local!`'s
// fallback-TLS lowering (illumos among the gate's targets) and denies
// initializers that already sit in `const` blocks; the allow keeps
// `-D warnings` honest on every platform the gate runs.
std::thread_local! {
    #[allow(clippy::missing_const_for_thread_local)]
    static EVENTS: RefCell<Option<Vec<Event>>> = const { RefCell::new(None) };
    #[allow(clippy::missing_const_for_thread_local)]
    static NEXT_WORK: Cell<usize> = const { Cell::new(0) };
}

/// Run `f` while tracing every proxy publication it creates.
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

    let events = EVENTS.with(|events| events.replace(Some(Vec::new())));
    let next_work = NEXT_WORK.with(|next| next.replace(0));
    let restore = Restore { events, next_work };
    let result = f();
    let events = EVENTS.with(|events| events.take().unwrap_or_default());
    drop(restore);
    (result, Trace(events))
}

pub fn new_work() -> usize {
    NEXT_WORK.with(|next| {
        let work = next.get();
        next.set(work + 1);
        work
    })
}

pub fn record(work: usize, kind: Kind, height: usize) {
    EVENTS.with(|events| {
        if let Some(events) = events.borrow_mut().as_mut() {
            events.push(Event { work, height, kind });
        }
    });
}

#[cfg(test)]
mod tests;
