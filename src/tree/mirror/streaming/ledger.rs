//! The session ledger: what one endpoint's session holds, and the room it
//! sets aside for the replies it has invited, in bytes.
//!
//! Each endpoint keeps one [`Ledger`], shared by its walk and its proxy. A
//! [`Charge`] is one entry in it: a number of bytes for one place a session
//! keeps state in ([`Holding`]). A charge travels with the item it pays
//! for, and dropping the item drops the charge, which returns its bytes to
//! the ledger. So every charge is released exactly where its item is
//! dropped, by construction, and a charge that outlives its session would
//! show as bytes the ledger still holds when the session ends.
//!
//! # What the ledger holds
//!
//! Every allocation a session makes falls into one of three classes:
//!
//! - **Built items**, whose size is known when the session builds them:
//!   the replies it builds, the questions it queues with the children each
//!   lists, the resolutions awaiting assembly, the records of the questions
//!   it sent, and the node copies its backend makes to explode a node
//!   ([`ErasedNode::explosion_bytes`](super::ErasedNode::explosion_bytes)).
//!   Each is charged as it is built.
//! - **Invited items**, which the peer's replies to this side's questions
//!   become: a parked reply, the records its take derives, and the parent
//!   resolution the stage builds from it. Each question is charged, when
//!   it is asked, a *reservation*: the most those items can hold, from the
//!   bound the peer's grandchild count of the node asked about sets
//!   ([`ReplyBound`]). The reservation
//!   travels with the question and is released in two parts: the parent
//!   resolution's part moves into that resolution and returns at assembly,
//!   and the rest returns when the stage has finished the reply.
//! - **Fixed state**, which every session sets up whatever it compares:
//!   wire buffering, decode fans, assembly's queues and spines, and tasks.
//!   The ledger charges none of it; it is a constant beside the ledger.
//!
//! The ledger's account therefore covers the session's transient heap,
//! less the fixed state and the scratch each stage fills for the one scope
//! it works on, at every moment, with one exception: a request reply's
//! charge prices the deletion filter's work by the top node's explosion
//! only, so a filter that rebuilds deeper nodes holds more than it is
//! charged.
//!
//! The ledger only accounts here; the session's window still decides how
//! much is in flight.

use std::mem::size_of;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::holding::Holding;
use super::message::Grandchildren;
use super::window::FAN;
#[cfg(any(test, feature = "test-internals"))]
use std::sync::OnceLock;

/// The most a reply can hold of the two things that grow with it: its
/// reactions, and the listing entries its queries carry.
///
/// A reply about a node holds one reaction per child either side holds:
/// at most the asker's listed children and the replier's children, which
/// are no more than the replier's grandchild count of the node, and never
/// more than a fan. Its queries list the replier's children of children it
/// shares with the asker, which together are at most that same count. A
/// reply to an empty question supplies the replier's children and lists
/// nothing; a reply about a leaf's parent lists nothing, since a question
/// about a leaf is empty; a reply about a leaf supplies one leaf.
/// The [`Default`] bound allows nothing: the bound of values built outside
/// a session, such as protocol values a test builds directly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ReplyBound {
    /// The most reactions the reply can hold.
    pub(crate) reactions: usize,
    /// The most listing entries its queries can carry together.
    pub(crate) listing: usize,
}

impl ReplyBound {
    /// The bound on the reply to a question about a node at typed height
    /// `height` that lists `listed` of the asker's children, whose
    /// replier's grandchild count of the node is `grandchildren`.
    pub(crate) fn of(height: usize, listed: usize, grandchildren: Grandchildren) -> Self {
        let grandchildren = grandchildren.get();
        if height == 1 {
            // A leaf's parent: its children are leaves, so its questions
            // are empty and its listings carry nothing.
            Self {
                reactions: (listed + grandchildren).min(FAN),
                listing: 0,
            }
        } else if listed == 0 {
            Self {
                reactions: grandchildren.min(FAN),
                listing: 0,
            }
        } else {
            Self {
                reactions: (listed + grandchildren).min(FAN),
                listing: grandchildren,
            }
        }
    }

    /// The bound on the reply to a leaf request: one supply.
    pub(crate) const LEAF: Self = Self {
        reactions: 1,
        listing: 0,
    };
}

/// Bytes one item of type `T` takes while it waits in a session channel.
///
/// Every channel boxes each queued item in a node beside one link pointer,
/// so the node is the item, padded to a word, and a word.
pub(crate) const fn queued<T>() -> usize {
    size_of::<Option<T>>().next_multiple_of(size_of::<usize>()) + size_of::<usize>()
}

/// Bytes a vector's buffer holds: its capacity, in elements.
pub(crate) fn buffer<T>(items: &Vec<T>) -> usize {
    items.capacity() * size_of::<T>()
}

/// The places a ledger keeps an account for, in [`Holding::ALL`] order.
const PLACES: usize = 9;

/// One endpoint's ledger: a handle onto the account its walk and proxy
/// share.
#[derive(Clone, Default)]
pub(crate) struct Ledger {
    /// The shared account.
    account: Arc<Account>,
}

/// The bytes one ledger holds, in all and per place, and the most it has
/// held at once.
#[derive(Default)]
struct Account {
    /// Bytes every live charge holds together.
    held: AtomicUsize,
    /// The most `held` has been.
    peak: AtomicUsize,
    /// Bytes held per place, indexed by [`place_index`].
    places: [AtomicUsize; PLACES],
}

/// A place's index among the ledger's accounts.
fn place_index(place: Holding) -> usize {
    match place {
        Holding::Questions => 0,
        Holding::Resolutions => 1,
        Holding::QuestionRecords => 2,
        Holding::ParkedReplies => 3,
        Holding::DecodedLeaves => 4,
        Holding::OutgoingReplies => 5,
        Holding::Wire => 6,
        Holding::Assembly => 7,
        Holding::ExplodedNodes => 8,
    }
}

impl Ledger {
    /// Begin an empty ledger.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Charge `bytes` for `place`, returning the charge, which returns them
    /// when it is dropped.
    pub(crate) fn charge(&self, place: Holding, bytes: usize) -> Charge {
        self.account.add(place, bytes);
        Charge {
            account: Some(self.account.clone()),
            place,
            bytes,
        }
    }

    /// Bytes every live charge holds together.
    #[cfg(test)]
    pub(crate) fn held(&self) -> usize {
        self.account.held.load(Ordering::Relaxed)
    }

    /// The most the ledger has held at once.
    #[cfg(test)]
    pub(crate) fn peak(&self) -> usize {
        self.account.peak.load(Ordering::Relaxed)
    }

    /// Bytes the live charges for `place` hold together.
    #[cfg(test)]
    pub(crate) fn held_for(&self, place: Holding) -> usize {
        self.account.places[place_index(place)].load(Ordering::Relaxed)
    }
}

impl Account {
    /// Add `bytes` to the account of `place`.
    fn add(&self, place: Holding, bytes: usize) {
        let held = self.held.fetch_add(bytes, Ordering::Relaxed) + bytes;
        self.peak.fetch_max(held, Ordering::Relaxed);
        self.places[place_index(place)].fetch_add(bytes, Ordering::Relaxed);
        observe(self, place, held);
    }

    /// Return `bytes` from the account of `place`.
    fn sub(&self, place: Holding, bytes: usize) {
        let held = self.held.fetch_sub(bytes, Ordering::Relaxed) - bytes;
        self.places[place_index(place)].fetch_sub(bytes, Ordering::Relaxed);
        observe(self, place, held);
    }
}

/// Bytes charged for one place, returned to their ledger when dropped.
///
/// A charge with no ledger holds nothing: the [`Default`] charge, which
/// items carry where no ledger accounts for them, such as in tests that
/// build protocol values directly.
pub(crate) struct Charge {
    /// The account the bytes are charged to, if any.
    account: Option<Arc<Account>>,
    /// The place the bytes are for.
    place: Holding,
    /// The bytes charged.
    bytes: usize,
}

impl Charge {
    /// The bytes this charge holds.
    #[cfg(test)]
    pub(crate) fn bytes(&self) -> usize {
        self.bytes
    }

    /// Move `bytes` of this charge, at most all of it, into a charge of its
    /// own for `place`, so the two parts return separately.
    pub(crate) fn split(&mut self, place: Holding, bytes: usize) -> Charge {
        let bytes = bytes.min(self.bytes);
        let Some(account) = &self.account else {
            return Charge::default();
        };
        account.sub(self.place, bytes);
        account.add(place, bytes);
        self.bytes -= bytes;
        Charge {
            account: Some(account.clone()),
            place,
            bytes,
        }
    }

    /// Fold `other` into this charge, which must be for the same ledger
    /// and place, or hold nothing.
    pub(crate) fn absorb(&mut self, mut other: Charge) {
        if other.account.is_none() {
            return;
        }
        if self.account.is_none() {
            *self = other;
            return;
        }
        debug_assert!(
            self.place == other.place
                && Arc::ptr_eq(
                    self.account.as_ref().expect("checked above"),
                    other.account.as_ref().expect("checked above"),
                ),
            "a charge absorbs only charges of its own ledger and place",
        );
        self.bytes += other.bytes;
        other.account = None;
    }
}

/// The charge with no ledger, which holds nothing.
impl Default for Charge {
    /// Returns a charge of no bytes on no ledger.
    fn default() -> Self {
        Self {
            account: None,
            place: Holding::Questions,
            bytes: 0,
        }
    }
}

/// Return the charge's bytes to its ledger.
impl Drop for Charge {
    /// Subtracts the charge from its place and from the ledger's total.
    fn drop(&mut self) {
        if let Some(account) = self.account.take() {
            account.sub(self.place, self.bytes);
        }
    }
}

/// Show a charge's size and place, not its ledger.
impl std::fmt::Debug for Charge {
    /// Formats the charge's place and bytes.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Charge")
            .field("place", &self.place)
            .field("bytes", &self.bytes)
            .finish()
    }
}

/// One change to a ledger, as the process's observation hook sees it.
#[cfg(any(test, feature = "test-internals"))]
#[derive(Clone, Copy, Debug)]
pub struct LedgerChange {
    /// The ledger that changed, identified for as long as it lives.
    pub ledger: usize,
    /// The place whose account changed.
    pub place: Holding,
    /// Bytes the ledger holds in all after the change.
    pub held: usize,
}

/// The process's ledger observation hook, called after every change to any
/// ledger.
#[cfg(any(test, feature = "test-internals"))]
pub type LedgerHook = fn(LedgerChange);

/// The hook every ledger in the process reports through, once installed.
#[cfg(any(test, feature = "test-internals"))]
static HOOK: OnceLock<LedgerHook> = OnceLock::new();

/// Install `hook` as the process's ledger observation hook, returning
/// whether it is now the installed one.
///
/// The first installation wins for the life of the process, as for the
/// allocation attribution hook.
#[cfg(any(test, feature = "test-internals"))]
pub fn observe_ledgers(hook: LedgerHook) -> bool {
    std::ptr::fn_addr_eq(*HOOK.get_or_init(|| hook), hook)
}

/// Report one change to `account` to the installed hook.
#[cfg(any(test, feature = "test-internals"))]
fn observe(account: &Account, place: Holding, held: usize) {
    if let Some(hook) = HOOK.get() {
        hook(LedgerChange {
            ledger: std::ptr::from_ref(account) as usize,
            place,
            held,
        });
    }
}

/// Report one change to `account`: nothing outside test builds.
#[cfg(not(any(test, feature = "test-internals")))]
#[inline(always)]
fn observe(_: &Account, _: Holding, _: usize) {}

/// One finished reply against the bound its question reserved for, as the
/// process's fit hook sees it.
#[cfg(any(test, feature = "test-internals"))]
#[derive(Clone, Copy, Debug)]
pub struct ReplyFit {
    /// The reactions the reply held.
    pub reactions: usize,
    /// The reactions its reservation allowed.
    pub reaction_bound: usize,
    /// The listing entries its queries carried.
    pub listing: usize,
    /// The listing entries its reservation allowed.
    pub listing_bound: usize,
}

/// The process's fit hook, called as each stage finishes a reply.
#[cfg(any(test, feature = "test-internals"))]
pub type FitHook = fn(ReplyFit);

/// The fit hook every session in the process reports through, once
/// installed.
#[cfg(any(test, feature = "test-internals"))]
static FIT_HOOK: OnceLock<FitHook> = OnceLock::new();

/// Install `hook` as the process's fit hook, returning whether it is now
/// the installed one; the first installation wins.
#[cfg(any(test, feature = "test-internals"))]
pub fn observe_fits(hook: FitHook) -> bool {
    std::ptr::fn_addr_eq(*FIT_HOOK.get_or_init(|| hook), hook)
}

/// Report a finished reply that held `held` against the `bound` its
/// question reserved for.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) fn fitted(bound: ReplyBound, held: ReplyBound) {
    if let Some(hook) = FIT_HOOK.get() {
        hook(ReplyFit {
            reactions: held.reactions,
            reaction_bound: bound.reactions,
            listing: held.listing,
            listing_bound: bound.listing,
        });
    }
}

/// Report a finished reply: nothing outside test builds.
#[cfg(not(any(test, feature = "test-internals")))]
#[inline(always)]
pub(crate) fn fitted(_: ReplyBound, _: ReplyBound) {}

#[cfg(test)]
mod tests;
