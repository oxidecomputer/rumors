//! Attribution of a session's allocations to the place that holds them, for
//! the session-budget meter.
//!
//! Each endpoint's ledger charges each place a session keeps in-flight state
//! separately ([`ledger`](super::ledger)). A meter that checks those charges
//! against the heap must know which place each allocation serves. The session marks the code that allocates for each place with a
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

/// A place in a session that holds in-flight state, as the charge's terms
/// divide them, or a place the charge leaves out.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Holding {
    /// The walk's questions, queued or built and waiting to be, with the
    /// children each lists: the charge's questions term, and its leaf
    /// requests at the last level.
    Questions,
    /// The walk's resolutions awaiting assembly, with their child slots, and
    /// the child listings the walk reads to answer the peer's questions,
    /// whose handles those slots keep: the charge's resolutions term.
    Resolutions,
    /// The proxy's records of questions, both those this side sent and those
    /// a taken reply creates: the charge's question-records term.
    QuestionRecords,
    /// Decoded replies parked until their stage takes them: the charge's
    /// parked-replies term.
    ParkedReplies,
    /// The decode fans that carry supplied leaves from a reply's decoder to
    /// their assembly.
    ///
    /// The charge leaves these out. Each logical stream's fan holds at most
    /// a fan of decoded leaves, and its decoder one more, whatever the
    /// budget, so together they cost a fixed amount whatever the budget and
    /// the sets' sizes, which the fixed allowance sets aside.
    DecodedLeaves,
    /// Replies this side builds and hands toward the wire, supplies
    /// included, before they are encoded: the charge's outgoing-replies
    /// term.
    OutgoingReplies,
    /// Encoding, multiplexing, demultiplexing, and decoding wire frames.
    Wire,
    /// Assembling resolved scopes into the merged tree, with the queues of
    /// fixed capacity around it: the level return queues between
    /// assemblers and the terminal leaf resolutions.
    ///
    /// The charge leaves these out. The queues are a fan deep whatever the
    /// budget, so they cost a fixed amount per level, and the in-memory
    /// backend builds each supplied subtree over its open right spine, at
    /// most one fan per path byte however many leaves the subtree holds;
    /// the fixed allowance sets both aside.
    Assembly,
    /// Admission's own bookkeeping: the units waiting for admission, the
    /// queue that orders them, and the wakers that resume them.
    ///
    /// The charge leaves it out, as a fixed amount whatever the budget and
    /// the sets' sizes, like the session's setup. A unit admitted as it
    /// asks allocates nothing, and a floor slot is a count in the ledger's
    /// own account, so only a unit that waits allocates: its waiter, the
    /// lock inside it where the standard library boxes locks, a clone of
    /// its task's waker where the executor's waker allocates one, and its
    /// place in the queue's buffer. Each walk stage waits on one unit at a
    /// time, so at most one unit per stage waits at once. Marking it apart
    /// keeps it out of whichever place's code admitted a unit or released
    /// the bytes that let one in.
    Admission,
    /// Scratch a backend fills to compute a node's hash, beyond the hash it
    /// keeps.
    ///
    /// For the in-memory backend this is the preimage a branch's hash is
    /// computed over, a record per child, which lives only within the call
    /// that computes the hash: the node keeps only the hash, inside its own
    /// allocation. A peeled view keeps no hash, so each listing or
    /// comparison that reads one computes it.
    ///
    /// The charge leaves it out, as a fixed amount whatever the budget and
    /// the sets' sizes, which the fixed allowance sets aside. Hashing is
    /// synchronous, so a thread holds the preimages of at most one hashing
    /// descent at once, and a descent nests at most one preimage per branch
    /// along a path. Marking it apart
    /// keeps it out of whichever place's code read the hash.
    Hashing,
}

impl Holding {
    /// Every place, in declaration order: how observers of ledgers and of
    /// the allocation meter index their accounts, and how the fixed
    /// allowance sums the places' working buffers.
    pub const ALL: [Self; 10] = [
        Self::Questions,
        Self::Resolutions,
        Self::QuestionRecords,
        Self::ParkedReplies,
        Self::DecodedLeaves,
        Self::OutgoingReplies,
        Self::Wire,
        Self::Assembly,
        Self::Admission,
        Self::Hashing,
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
