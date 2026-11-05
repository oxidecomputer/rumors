//! Test-only transcript of the walk's outgoing replies, payload-erased.
//!
//! Where [`super::progress`] records the walk's *internal* publications, this
//! module records every [`Reply`] each endpoint produces, reduced to its
//! reaction [`Label`]s: `Match`, `Supply` with its radix, or `Query` with its
//! listing's radices. Hashes, nodes, and versions are erased. The capture sits
//! before the remote proxy frames supplies by byte budget, so it describes the
//! walk's decisions rather than wire frames.
//!
//! Capture point: [`super::work::Work::respond`], the pump every response
//! stream funnels through. A reply lands when that pump pulls its first
//! chunk from the walk, before the counterparty can receive it, and its
//! later chunks extend it. Per-stream order therefore
//! matches the walk, and the global order is causally consistent: a reply is
//! recorded before any reply that reacts to it.

use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::tree::mirror::streaming::erased::{Reaction, Reply};

/// One reaction of a captured reply, with every payload erased.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Label {
    /// [`Reaction::Supply`]: the announced radix, the node dropped.
    Supply(u8),
    /// [`Reaction::Match`].
    Match,
    /// [`Reaction::Query`]: the listing's radices, the hashes dropped.
    Query(Vec<u8>),
}

/// One captured outgoing reply.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sent {
    /// The sending endpoint's work identity (shared with the progress trace).
    pub work: usize,
    /// The reply's children height: which logical stream it rides.
    pub height: usize,
    /// The reply's reactions, payload-erased, in reaction order.
    pub labels: Vec<Label>,
}

/// A completed session's outgoing replies, in publication order.
#[derive(Debug, Eq, PartialEq)]
pub struct Transcript(Vec<Sent>);

impl Transcript {
    /// The captured messages, in publication order.
    pub fn sent(&self) -> &[Sent] {
        &self.0
    }
}

/// One active capture: the replies begun so far, and where each stream's
/// open reply sits among them.
#[derive(Default)]
struct Capture {
    /// Replies, in the order their first chunks were published.
    sent: Vec<Sent>,
    /// The index in `sent` of each `(work, height)` stream's open reply.
    open: BTreeMap<(usize, usize), usize>,
}

// Clippy's `missing_const_for_thread_local` can reject const-block initializers
// on targets that lower `thread_local!` through fallback TLS. The allow keeps
// `-D warnings` clean on those targets.
std::thread_local! {
    /// Reply capture active on this test thread.
    #[allow(clippy::missing_const_for_thread_local)]
    static SENT: RefCell<Option<Capture>> = const { RefCell::new(None) };
}

/// Run `f` while capturing every outgoing reply it publishes.
pub fn with_transcript<R>(f: impl FnOnce() -> R) -> (R, Transcript) {
    /// Capture displaced by a nested transcript scope.
    struct Restore {
        /// The enclosing scope's capture.
        capture: Option<Capture>,
    }

    /// Reinstate the enclosing capture when the nested scope ends.
    impl Drop for Restore {
        /// Restore the enclosing transcript capture when this scope exits.
        fn drop(&mut self) {
            SENT.with(|sent| sent.replace(self.capture.take()));
        }
    }

    let previous = SENT.with(|sent| sent.replace(Some(Capture::default())));
    let restore = Restore { capture: previous };
    let result = f();
    let sent = SENT.with(|sent| sent.take().unwrap_or_default().sent);
    drop(restore);
    (result, Transcript(sent))
}

/// Record one outgoing chunk, payload-erased.
///
/// `height` is the reply's children height (which logical stream it
/// rides), threaded from the capturing pump's typed exit. A reply takes its
/// place in the transcript at its first chunk, and its later chunks extend
/// it.
pub(super) fn reply<E>(work: usize, height: usize, chunk: &Reply<E>) {
    let labels = chunk.reactions.iter().map(|reaction| match reaction {
        Reaction::Supply(radix, _) => Label::Supply(*radix),
        Reaction::Match => Label::Match,
        Reaction::Query(listing) => {
            Label::Query(listing.iter().map(|listed| listed.radix).collect())
        }
    });
    SENT.with(|sent| {
        if let Some(capture) = sent.borrow_mut().as_mut() {
            let next = capture.sent.len();
            let index = *capture.open.entry((work, height)).or_insert(next);
            if index == next {
                capture.sent.push(Sent {
                    work,
                    height,
                    labels: Vec::new(),
                });
            }
            capture.sent[index].labels.extend(labels);
            if chunk.last {
                capture.open.remove(&(work, height));
            }
        }
    });
}
