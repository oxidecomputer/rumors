//! Test-only transcript of the walk's outgoing replies, payload-erased.
//!
//! Where [`super::progress`] records the walk's *internal* publications, this
//! module records every [`Reply`] each endpoint produces, reduced to its
//! reaction [`Label`]s: `Match`, `Supply` with its radix, or `Query` with its
//! listing's radices. Hashes, nodes, and versions are erased. The capture sits
//! before the remote proxy frames supplies by byte budget, so it describes the
//! walk's decisions rather than link-level frames.
//!
//! Capture point: [`super::work::Work::respond`], the pump every response
//! stream funnels through. Entries land when that pump pulls a reply from the
//! walk, before the counterparty can receive it. Per-stream order therefore
//! matches the walk, and the global order is causally consistent: a reply is
//! recorded before any reply that reacts to it.

use std::cell::RefCell;

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

// Clippy's `missing_const_for_thread_local` can reject const-block initializers
// on targets that lower `thread_local!` through fallback TLS. The allow keeps
// `-D warnings` clean on those targets.
std::thread_local! {
    /// Reply capture active on this test thread.
    #[allow(clippy::missing_const_for_thread_local)]
    static SENT: RefCell<Option<Vec<Sent>>> = const { RefCell::new(None) };
}

/// Run `f` while capturing every outgoing reply it publishes.
pub fn with_transcript<R>(f: impl FnOnce() -> R) -> (R, Transcript) {
    /// Capture displaced by a nested transcript scope.
    struct Restore {
        /// Replies captured by the enclosing scope.
        sent: Option<Vec<Sent>>,
    }

    /// Reinstate the enclosing capture when the nested scope ends.
    impl Drop for Restore {
        /// Restore the enclosing transcript capture when this scope exits.
        fn drop(&mut self) {
            SENT.with(|sent| sent.replace(self.sent.take()));
        }
    }

    let previous = SENT.with(|sent| sent.replace(Some(Vec::new())));
    let restore = Restore { sent: previous };
    let result = f();
    let sent = SENT.with(|sent| sent.take().unwrap_or_default());
    drop(restore);
    (result, Transcript(sent))
}

/// Record one outgoing reply, payload-erased; `height` is the reply's
/// children height (which logical stream it rides), threaded from the
/// capturing pump's typed exit.
pub(super) fn reply<E>(work: usize, height: usize, reply: &Reply<E>) {
    let labels = reply
        .reactions
        .iter()
        .map(|reaction| match reaction {
            Reaction::Supply(radix, _) => Label::Supply(*radix),
            Reaction::Match => Label::Match,
            Reaction::Query(listing) => {
                Label::Query(listing.iter().map(|(radix, _)| *radix).collect())
            }
        })
        .collect();
    SENT.with(|sent| {
        if let Some(sent) = sent.borrow_mut().as_mut() {
            sent.push(Sent {
                work,
                height,
                labels,
            });
        }
    });
}
