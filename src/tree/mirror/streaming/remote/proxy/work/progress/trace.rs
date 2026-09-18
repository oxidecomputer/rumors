use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};

use crate::tree::typed::height::{Height as _, Root, UnderRoot};

/// The overlapping portion of the session that owns a publication.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Stage {
    /// The root-to-leaf walk.
    Walk,
    /// The final bidirectional leaf exchange.
    Terminal,
}

/// One progress-critical proxy publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    /// A complete outgoing reply has flushed its question frames.
    WireReply { questions: usize },
    /// One flushed question is being registered for decoding.
    LocalQuestion,
    /// An incoming answer is being published before its derived scopes.
    DecodedReply { scopes: usize },
    /// One derived scope is being published for the next exchange.
    NextScope,
}

/// A publication labelled by its proxy and the question it belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Event {
    /// The proxy endpoint that published this event.
    work: usize,
    /// The independently progressing part of that endpoint.
    stage: Stage,
    /// The question's height, shared by its answer and any derived scopes.
    height: usize,
    /// Which publication occurred.
    kind: Kind,
}

/// A completed positive session's proxy-ordering trace.
#[derive(Debug)]
pub struct Trace(Vec<Event>);

/// Check ordering and the question registrations needed to decode replies.
impl Trace {
    /// Assert wire-before-question and reply-before-scope ordering.
    pub fn assert_valid(&self) {
        let mut questions = BTreeMap::<(usize, Stage, usize), usize>::new();
        let mut scopes = BTreeMap::<(usize, Stage, usize), usize>::new();
        for (index, event) in self.0.iter().enumerate() {
            match event.kind {
                Kind::WireReply { questions: count } => {
                    assert_drained(&questions, event, index, "questions");
                    questions.insert((event.work, event.stage, event.height), count);
                }
                Kind::LocalQuestion => consume(&mut questions, event, index, "wire reply"),
                Kind::DecodedReply { scopes: count } => {
                    assert_drained(&scopes, event, index, "scopes");
                    scopes.insert((event.work, event.stage, event.height), count);
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

    /// Reject vacuous traces: a divergent session records one opening from
    /// the greeting, one opening question, and at least one outgoing reply.
    pub fn assert_covers_divergent_session(&self) {
        let openings = self
            .0
            .iter()
            .filter(|event| {
                event.height == Root::HEIGHT && matches!(event.kind, Kind::DecodedReply { .. })
            })
            .count();
        assert_eq!(
            openings, 1,
            "a divergent session records exactly one greeting-seeded opening reply, found {openings}",
        );
        let questions = self
            .0
            .iter()
            .filter(|event| {
                event.height == UnderRoot::HEIGHT && matches!(event.kind, Kind::LocalQuestion)
            })
            .count();
        assert_eq!(
            questions, 1,
            "a divergent session records exactly one opening question, found {questions}",
        );
        assert!(
            self.0
                .iter()
                .any(|event| matches!(event.kind, Kind::WireReply { .. })),
            "a divergent session records at least one wire reply, found none",
        );
    }

    /// Every decoded answer needs a previously flushed question on the same
    /// endpoint. Questions and answers pair in FIFO order at each height.
    /// The root opening is the sole exception: its scope comes from the greeting.
    pub fn assert_registration_causality(&self) {
        let mut questions = BTreeMap::<(usize, Stage, usize), usize>::new();
        let mut decoded = BTreeMap::<(usize, Stage, usize), usize>::new();
        for (index, event) in self.0.iter().enumerate() {
            match event.kind {
                Kind::LocalQuestion => {
                    *questions
                        .entry((event.work, event.stage, event.height))
                        .or_insert(0) += 1;
                }
                Kind::DecodedReply { .. } => {
                    let count = decoded
                        .entry((event.work, event.stage, event.height))
                        .or_insert(0);
                    *count += 1;
                    let available = if event.height == Root::HEIGHT {
                        1
                    } else {
                        questions
                            .get(&(event.work, event.stage, event.height))
                            .copied()
                            .unwrap_or(0)
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

/// A reply cannot overtake the previous reply’s dependent publications.
fn assert_drained(
    ledger: &BTreeMap<(usize, Stage, usize), usize>,
    event: &Event,
    index: usize,
    items: &str,
) {
    let remaining = ledger
        .get(&(event.work, event.stage, event.height))
        .copied()
        .unwrap_or(0);
    assert_eq!(
        remaining, 0,
        "event {event:?} at trace index {index} overtook {remaining} prior {items}",
    );
}

/// Charge one dependent publication to the reply that made it available.
fn consume(
    ledger: &mut BTreeMap<(usize, Stage, usize), usize>,
    event: &Event,
    index: usize,
    prerequisite: &str,
) {
    let remaining = ledger
        .get_mut(&(event.work, event.stage, event.height))
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
    /// Publications recorded by the current trace scope.
    #[allow(clippy::missing_const_for_thread_local)]
    static EVENTS: RefCell<Option<Vec<Event>>> = const { RefCell::new(None) };
    /// Next endpoint identifier within the current trace.
    #[allow(clippy::missing_const_for_thread_local)]
    static NEXT_WORK: Cell<usize> = const { Cell::new(0) };
}

/// Run `f` while tracing every proxy publication it creates.
pub fn with_trace<R>(f: impl FnOnce() -> R) -> (R, Trace) {
    /// Restore an outer trace even when the inner test panics.
    struct Restore {
        events: Option<Vec<Event>>,
        next_work: usize,
    }

    /// Restore the saved thread-local recorder and endpoint numbering.
    impl Drop for Restore {
        /// Reinstate the enclosing trace.
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

/// Allocate an endpoint identifier within the current trace.
pub fn new_work() -> usize {
    NEXT_WORK.with(|next| {
        let work = next.get();
        next.set(work + 1);
        work
    })
}

/// Append a publication when a trace is active on this thread.
pub fn record(work: usize, kind: Kind, height: usize) {
    record_stage(work, Stage::Walk, kind, height);
}

/// Append a publication for the given session stage when tracing is active.
pub fn record_stage(work: usize, stage: Stage, kind: Kind, height: usize) {
    EVENTS.with(|events| {
        if let Some(events) = events.borrow_mut().as_mut() {
            events.push(Event {
                work,
                stage,
                height,
                kind,
            });
        }
    });
}

#[cfg(test)]
mod tests;
