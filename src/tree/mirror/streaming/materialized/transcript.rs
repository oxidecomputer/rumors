//! Test-only transcript of the session's outgoing wire messages, payload-erased.
//!
//! Where [`super::progress`] records the walk's *internal* publications, this
//! module records what actually crosses the wire: every [`Reply`] each
//! endpoint sends, reduced to its reaction [`Label`]s — `Match`, `Supply`
//! with its announced radix, `Query` with its listing's radices. Hashes,
//! nodes, and versions are erased. This is the observable the formal model's
//! payload-independence premise quantifies over (MODEL.md §1: "the count and
//! order of channel operations depend only on each child's merge-join arm,
//! never on payloads"), promoted to a proptest bridge by the mux
//! adjudication (bridge B5; `formal/AUDIT-NOTES.md` A5): the announced
//! dispute skeleton must be reconstructible from this transcript alone, and
//! the session's channel-op trace must be a function of that skeleton only.
//!
//! Capture point: [`super::work::Work::respond`], the pump every response
//! stream funnels through. Entries land in publication order — the moment
//! the pump pulls a reply from its walk, before the counterparty can have
//! seen it — so per-stream order is exactly the wire order, and the global
//! order is causally consistent: a reply is always recorded before any reply
//! that reacts to it.

use std::cell::RefCell;

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        message::{Reaction, Reply},
    },
    typed::height::{Height, Z},
};

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

/// One captured wire message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sent {
    /// The sending endpoint's work identity (shared with the progress trace).
    pub work: usize,
    /// The reply's children height: which logical stream it rides.
    pub height: usize,
    /// The reply's reactions, payload-erased, in reaction order.
    pub labels: Vec<Label>,
}

/// A completed session's outgoing-message transcript, in publication order.
#[derive(Debug, Eq, PartialEq)]
pub struct Transcript(Vec<Sent>);

impl Transcript {
    /// The captured messages, in publication order.
    pub fn sent(&self) -> &[Sent] {
        &self.0
    }
}

std::thread_local! {
    static SENT: RefCell<Option<Vec<Sent>>> = const { RefCell::new(None) };
}

/// Run `f` while capturing every wire message it publishes.
pub fn with_transcript<R>(f: impl FnOnce() -> R) -> (R, Transcript) {
    struct Restore {
        sent: Option<Vec<Sent>>,
    }

    impl Drop for Restore {
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

/// Record one outgoing reply, payload-erased.
pub(super) fn reply<B, T, H>(work: usize, reply: &Reply<B, T, H>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    let labels = reply
        .replies
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
                height: H::HEIGHT,
                labels,
            });
        }
    });
}
