//! Attribution of a session's allocations to the place that holds them, for
//! the session-budget meter.
//!
//! A session keeps in-flight state in several places: the walk's questions
//! and resolutions, the proxy's question records and parked replies, the
//! replies on their way to the wire, and more. A meter that checks a
//! session's heap place by place must know which place each allocation
//! serves. The session marks the code that allocates for each place with a
//! [`Holding`]: synchronous work through [`held_by`], and a task or a stream
//! that allocates for one place throughout through [`holding`] and
//! [`holding_stream`]. A test installs one hook for its whole process with
//! [`attribute_allocations`], and the session calls it around each marked
//! region, inside which the test labels the allocator's records; the
//! innermost mark applies.
//!
//! An allocation counts toward the place whose code made it, wherever the
//! value later moves. The marks therefore sit where each place's contents
//! are built: a question's child listing where the walk explodes the node it
//! asks about, a parked reply where the decoder assembles it. Moving a value
//! between places allocates nothing but a channel's per-message node, which
//! the sending place's mark covers.
//!
//! Without the `test-internals` feature, every mark is the bare expression
//! or future it wraps, and nothing here exists at run time.

#[cfg(any(test, feature = "test-internals"))]
use std::{future::poll_fn, pin::pin, sync::OnceLock};

use futures::{Future, Stream};

/// A place in a session that holds in-flight state.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Holding {
    /// The walk's questions, queued or built and waiting to be, with the
    /// children each lists, and the leaf requests at the last level.
    Questions,
    /// The walk's resolutions awaiting assembly, with their child slots, and
    /// the child listings the walk reads to answer the peer's questions,
    /// whose handles those slots keep.
    Resolutions,
    /// The proxy's records of questions, both those this side sent and those
    /// a taken reply creates.
    QuestionRecords,
    /// Decoded replies parked until their stage takes them.
    ParkedReplies,
    /// The decode fans that carry supplied leaves from a reply's decoder to
    /// their assembly.
    DecodedLeaves,
    /// Replies this side builds and hands toward the wire, supplies
    /// included, before they are encoded.
    OutgoingReplies,
    /// Encoding, multiplexing, demultiplexing, and decoding wire frames.
    Wire,
    /// Assembling resolved scopes into the merged tree, with the queues of
    /// fixed capacity around it: the level return queues between
    /// assemblers and the terminal leaf resolutions.
    ///
    /// The queues are a fan deep whatever the window, so they cost a fixed
    /// amount per level, and the in-memory backend builds each supplied
    /// subtree over its open right spine, at most one fan per path byte
    /// however many leaves the subtree holds.
    Assembly,
    /// Nodes a backend builds to explode a node into its children, beyond
    /// the handles it yields.
    ///
    /// For the in-memory backend these are the copies of shared compressed
    /// nodes that peeling one byte off a path makes.
    ExplodedNodes,
}

impl Holding {
    /// Every place, in declaration order.
    #[cfg(any(test, feature = "test-internals"))]
    pub const ALL: [Self; 9] = [
        Self::Questions,
        Self::Resolutions,
        Self::QuestionRecords,
        Self::ParkedReplies,
        Self::DecodedLeaves,
        Self::OutgoingReplies,
        Self::Wire,
        Self::Assembly,
        Self::ExplodedNodes,
    ];
}

/// The process's attribution hook: called with the place a region serves
/// and the region itself, which it must run exactly once.
#[cfg(any(test, feature = "test-internals"))]
pub type Hook = fn(Holding, &mut dyn FnMut());

/// The hook every session in the process attributes through, once installed.
#[cfg(any(test, feature = "test-internals"))]
static HOOK: OnceLock<Hook> = OnceLock::new();

/// Install `hook` as the process's attribution hook, returning whether it is
/// now the installed one.
///
/// The first installation wins for the life of the process; a later call
/// with the same function succeeds, and one with another function fails and
/// changes nothing.
#[cfg(any(test, feature = "test-internals"))]
pub fn attribute_allocations(hook: Hook) -> bool {
    std::ptr::fn_addr_eq(*HOOK.get_or_init(|| hook), hook)
}

/// The process's report hook: called with a place and the bytes a backend
/// reported the place's next work will allocate.
#[cfg(any(test, feature = "test-internals"))]
pub type ReportHook = fn(Holding, usize);

/// The report hook every session in the process reports through, once
/// installed.
#[cfg(any(test, feature = "test-internals"))]
static REPORT_HOOK: OnceLock<ReportHook> = OnceLock::new();

/// Install `hook` as the process's report hook, returning whether it is now
/// the installed one.
///
/// A session reports, before each node it explodes, the bytes its backend
/// says the explosion allocates, as work for [`Holding::ExplodedNodes`]; a
/// meter compares the sum with what that place's marked regions allocated.
/// The first installation wins for the life of the process, as for
/// [`attribute_allocations`].
#[cfg(any(test, feature = "test-internals"))]
pub fn report_allocations(hook: ReportHook) -> bool {
    std::ptr::fn_addr_eq(*REPORT_HOOK.get_or_init(|| hook), hook)
}

/// Report that `holding`'s next work allocates `bytes`.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) fn reported(holding: Holding, bytes: usize) {
    if let Some(hook) = REPORT_HOOK.get() {
        hook(holding, bytes);
    }
}

/// Report that `holding`'s next work allocates `bytes`.
#[cfg(not(any(test, feature = "test-internals")))]
#[inline(always)]
pub(crate) fn reported(_: Holding, _: usize) {}

/// Run `region` as work for `holding`.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) fn held_by<T>(holding: Holding, region: impl FnOnce() -> T) -> T {
    let Some(hook) = HOOK.get() else {
        return region();
    };
    let mut region = Some(region);
    let mut output = None;
    hook(holding, &mut || {
        if let Some(region) = region.take() {
            output = Some(region());
        }
    });
    output.expect("the attribution hook runs its region exactly once")
}

/// Run `region` as work for `holding`.
#[cfg(not(any(test, feature = "test-internals")))]
#[inline(always)]
pub(crate) fn held_by<T>(_: Holding, region: impl FnOnce() -> T) -> T {
    region()
}

/// Poll `future` as work for `holding`, every poll.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) async fn holding<F: Future>(holding: Holding, future: F) -> F::Output {
    let mut future = pin!(future);
    poll_fn(|cx| held_by(holding, || future.as_mut().poll(cx))).await
}

/// Poll `future` as work for `holding`, every poll.
#[cfg(not(any(test, feature = "test-internals")))]
#[inline(always)]
pub(crate) fn holding<F: Future>(_: Holding, future: F) -> F {
    future
}

/// Poll `stream` as work for `holding`, every poll.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) fn holding_stream<S: Stream>(
    holding: Holding,
    stream: S,
) -> impl Stream<Item = S::Item> {
    async_stream::stream! {
        let mut stream = pin!(stream);
        while let Some(item) =
            poll_fn(|cx| held_by(holding, || stream.as_mut().poll_next(cx))).await
        {
            yield item;
        }
    }
}

/// Poll `stream` as work for `holding`, every poll.
#[cfg(not(any(test, feature = "test-internals")))]
#[inline(always)]
pub(crate) fn holding_stream<S: Stream>(_: Holding, stream: S) -> S {
    stream
}
