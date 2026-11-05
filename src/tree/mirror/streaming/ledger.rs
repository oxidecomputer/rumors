//! The session ledger: what one endpoint's session holds, and the room it
//! sets aside for the replies it has invited, in bytes.
//!
//! Each endpoint keeps one [`Ledger`], which its walk charges for the items
//! it hands its proxy as well as its own. A [`Charge`] is one entry in it:
//! a number of bytes for one place a session keeps state in ([`Holding`]).
//! A charge travels with the item it pays for, and dropping the item drops
//! the charge, which returns its bytes to the ledger. So every charge is
//! released exactly where its item is dropped, by construction, and a
//! charge that outlives its session would show as bytes the ledger still
//! holds when the session ends.
//!
//! # What the ledger holds
//!
//! Every allocation a session makes falls into one of three classes:
//!
//! - **Built items**, whose size is known when the session builds them:
//!   the replies it builds, the questions it queues with the children each
//!   lists, the resolutions awaiting assembly, the records of the questions
//!   it sent. Each is charged as it is built.
//! - **Invited items**, which the peer's replies to this side's questions
//!   become: a parked reply, the records its take derives, and the parent
//!   resolution the stage builds from it. Each question is charged, when
//!   it is asked, a *reservation*: the most those items can hold, from the
//!   bound the peer's grandchild count of the node asked about sets
//!   ([`ReplyBound`]). Each part of the reservation returns as soon as it
//!   is known to be unused. The room the reply's chunks park in travels
//!   with the question's record to the decoder, which charges each chunk
//!   its own part as it decodes, returned once the chunk's stage has used
//!   it, and returns the rest as soon as the reply shows it cannot use it:
//!   whatever exceeds what the reply can still deliver. The records part
//!   returns when the stage has finished the reply, and the parent
//!   resolution's part moves into that resolution and returns at assembly.
//! - **Fixed state**, which every session sets up whatever it compares:
//!   wire buffering, decode fans, assembly's queues and spines, and tasks.
//!   The ledger charges none of it; it is a constant beside the ledger.
//!
//! The ledger's account therefore covers the session's transient heap,
//! less the fixed state and the scratch each stage fills for the one scope
//! it works on, at every moment. The session-budget suite holds it to
//! that.
//!
//! # What the ledger decides
//!
//! The walk builds its work in units, each admitted by the ledger against
//! the endpoint's budget with a grant of the most its items can hold
//! ([`Ledger::admit`]); every item is built out of its unit's grant
//! ([`Charge::take`]). The [`admission`] module states the rule, the
//! bound it keeps, and why every unit is admitted.

use std::mem::size_of;
#[cfg(any(test, feature = "test-internals"))]
use std::sync::PoisonError;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::FAN;
use super::holding::Holding;
use super::message::Grandchildren;
use crate::counter;

#[cfg(any(test, feature = "test-internals"))]
pub use self::admission::{Admission, AdmissionHook, observe_admissions};
pub(crate) use self::admission::{Admit, Unit};

/// The default memory budget for each synchronization session: 512 MiB.
///
/// See [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget) for
/// what the budget covers and [the sizing guide](crate::sizing) for choosing
/// one.
pub const DEFAULT_SYNC_MEMORY_BUDGET: usize = 512 * 1024 * 1024;

mod admission;
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
    pub(crate) const fn of(height: usize, listed: usize, grandchildren: Grandchildren) -> Self {
        let grandchildren = grandchildren.get();
        if height == 1 {
            // A leaf's parent: its children are leaves, so its questions
            // are empty and its listings carry nothing.
            Self {
                reactions: min(listed + grandchildren, FAN),
                listing: 0,
            }
        } else if listed == 0 {
            Self {
                reactions: min(grandchildren, FAN),
                listing: 0,
            }
        } else {
            Self {
                reactions: min(listed + grandchildren, FAN),
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

/// Bytes one item of type `T` takes in an [`Arc`]'s allocation: the item
/// beside its two reference counts, padded to the item's alignment.
pub(crate) const fn shared<T>() -> usize {
    (2 * size_of::<usize>()).next_multiple_of(align_of::<T>()) + size_of::<T>()
}

/// Bytes a vector's buffer holds: its capacity, in elements.
pub(crate) fn buffer<T>(items: &Vec<T>) -> usize {
    items.capacity() * size_of::<T>()
}

/// The smaller of `a` and `b`, as [`Ord::min`] computes it, for the prices
/// constants compute: constants cannot call `Ord`'s methods.
pub(crate) const fn min(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}

/// The larger of `a` and `b`, as [`Ord::max`] computes it, for the prices
/// constants compute.
pub(crate) const fn max(a: usize, b: usize) -> usize {
    if a < b { b } else { a }
}

/// The places a ledger keeps an account for, in [`Holding::ALL`] order.
#[cfg(any(test, feature = "test-internals"))]
pub const PLACES: usize = Holding::ALL.len();

/// One endpoint's ledger: a handle onto the account its walk and proxy
/// share.
///
/// The [`Default`] ledger has an unlimited budget.
#[derive(Clone, Default)]
pub(crate) struct Ledger {
    /// The shared account.
    account: Arc<Account>,
}

/// The bytes one ledger holds and the admission state against its budget.
///
/// In builds that observe ledgers, it counts the bytes held in all,
/// through floor slots, carried by them, and per place, and the most held
/// at once. The bytes the budget counts, what each kind and depth of unit
/// holds against the budget, what floor slots carry, and the invitation
/// share's claims and pace live in the admission queue, under its lock
/// ([`admission`]).
///
/// Of the bytes held, admission reads only what the budget counts and what
/// each kind and depth holds against it, so builds that observe nothing
/// keep only those, in the queue: every charge, split, and release would
/// otherwise update counters nothing reads.
struct Account {
    /// Bytes every live charge holds together.
    #[cfg(any(test, feature = "test-internals"))]
    held: AtomicUsize,
    /// The most `held` has been.
    #[cfg(any(test, feature = "test-internals"))]
    peak: AtomicUsize,
    /// Bytes held per place, indexed by [`place_index`].
    #[cfg(any(test, feature = "test-internals"))]
    places: [AtomicUsize; PLACES],
    /// Bytes held by charges built out of floor grants, which the budget
    /// does not count.
    #[cfg(any(test, feature = "test-internals"))]
    floor: AtomicUsize,
    /// Bytes floor slots carry for charges built out of grants admitted
    /// against the budget, which the budget then no longer counts
    /// ([`admission`], "Carrying").
    ///
    /// With `floor`, it makes the ledger's floor part. Admission changes it
    /// only under its queue's lock, where each slot's share is kept.
    #[cfg(any(test, feature = "test-internals"))]
    carried: AtomicUsize,
    /// Bytes answers have kept out of their grants, by the depth of the
    /// scope each answers.
    ///
    /// Every answer's grant counts as it is made, less what it returns
    /// unused. The invitation share's pace reads it as each reply finishes
    /// ([`Ledger::processed`]).
    kept: [AtomicU64; admission::DEPTHS],
    /// Bytes items needed beyond the grant they were built out of: zero
    /// whenever every grant bounds its unit, as admission requires.
    short: AtomicU64,
    /// Units that waited for admission, over the ledger's life.
    waited: AtomicU64,
    /// How many live charges share each floor slot, indexed by
    /// [`Slot::index`](admission::Slot::index): a slot is taken exactly
    /// while its count is above zero.
    ///
    /// A count moves to or from zero only under the admission queue's lock
    /// ([`admission::Slot`]).
    sharing: [AtomicUsize; admission::SLOTS],
    /// The share peak, with the terms of the check that weighed it: the
    /// largest sum any invitation share check has weighed, so the least
    /// budget at which every check made so far would pass ([`admission`]).
    ///
    /// Read from a session with no budget, it and the ledger's peak outside
    /// its floor slots give the session's no-cost point. Every check is made
    /// under the admission queue's lock, which orders the updates.
    #[cfg(any(test, feature = "test-internals"))]
    share_peak: Mutex<SharePeak>,
    /// The largest grant each floor slot has admitted or carried, by kind
    /// and depth: each slot's `F`.
    #[cfg(any(test, feature = "test-internals"))]
    slot_peaks: [[AtomicUsize; admission::DEPTHS]; 2],
    /// The most the ledger may hold outside its floor slots: for a
    /// session, its memory budget less the floor allowance ([`admission`]).
    budget: usize,
    /// The units waiting for admission.
    admission: Mutex<admission::Queue>,
}

impl Default for Account {
    /// Returns an empty account with an unlimited budget.
    fn default() -> Self {
        Self::new(usize::MAX)
    }
}

/// A place's index among the ledger's accounts.
#[cfg(any(test, feature = "test-internals"))]
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
        Holding::Admission => 8,
        Holding::Hashing => 9,
    }
}

impl Ledger {
    /// Begin an empty ledger whose units are admitted against `budget`.
    pub(crate) fn new(budget: usize) -> Self {
        Self {
            account: Arc::new(Account::new(budget)),
        }
    }

    /// Wait until a unit of `unit` kind concerning a scope `depth` bytes
    /// deep, whose items take at most `bytes`, is admitted, and return its
    /// grant ([`admission`]).
    ///
    /// # Panics
    ///
    /// Panics if `depth` exceeds 32, the depth of a leaf: a programmer
    /// error, since every scope is a prefix of a 32-byte path.
    ///
    /// Panics if `bytes` exceeds `u32::MAX`: a programmer error, since no
    /// unit's grant exceeds the walk's largest, which a constant assertion
    /// beside the walk's construction keeps within `u32::MAX`
    /// ([`largest_grant`](super::materialized::largest_grant)).
    pub(crate) fn admit(&self, unit: Unit, depth: usize, bytes: usize) -> Admit {
        let bytes = u32::try_from(bytes)
            .expect("a grant is at most the largest grant, which fits a charge's four bytes");
        Admit::new(self.account.clone(), unit, depth, bytes, None)
    }

    /// Wait until a question is admitted under the invitation share, and
    /// return its grant ([`admission`]).
    ///
    /// The question concerns a scope `depth` bytes deep, its items take at
    /// most `bytes`, and its reply is predicted to need `need` bytes
    /// processed.
    ///
    /// # Panics
    ///
    /// As [`admit`](Self::admit).
    pub(crate) fn invite(&self, depth: usize, bytes: usize, need: usize) -> Admit {
        let bytes = u32::try_from(bytes)
            .expect("a grant is at most the largest grant, which fits a charge's four bytes");
        Admit::new(
            self.account.clone(),
            Unit::Question,
            depth,
            bytes,
            Some(need),
        )
    }

    /// Note that this side's stage has finished processing a reply to a
    /// question about a scope `depth` bytes deep, whose need was `need`, and
    /// which took its depth's floor slot if `floor` ([`admission`]).
    ///
    /// A question the invitation share admitted against the budget claimed
    /// its need as it was admitted, and withdraws it here; one on its floor
    /// slot claimed nothing. The need must be the one the question was
    /// admitted with, which the stage recomputes from the bound the
    /// question's query carries.
    ///
    /// The finish also moves the depth's pace. One stage processes the
    /// replies at each depth, one at a time, and only that processing builds
    /// answers one level down, so the bytes those answers have kept as a
    /// reply finishes are exactly what the finished replies built, and the
    /// pace compares them with the finished replies' needs. The claims
    /// shrink and the pace may move either way, so the waiting units are
    /// reconsidered.
    ///
    /// The claims and the pace change together under the admission queue's
    /// lock, where every invitation share check reads them, so no check
    /// sees a finish half applied ([`admission::processed`]).
    pub(crate) fn processed(&self, depth: usize, need: usize, floor: bool) {
        admission::processed(&self.account, depth, need, floor);
    }

    /// Units that have waited for admission, over the ledger's life: those
    /// [`Admit`] did not admit as it queued them.
    pub(crate) fn waited(&self) -> u64 {
        self.account.waited.load(Ordering::Relaxed)
    }

    /// Charge `bytes` for `place` outside admission, returning the charge,
    /// which returns them when it is dropped.
    #[cfg(test)]
    pub(crate) fn charge(&self, place: Holding, bytes: usize) -> Charge {
        let bytes = u32::try_from(bytes).expect("a test charges fewer than 4 GiB");
        admission::charge(&self.account, place, bytes)
    }

    /// Bytes every live charge holds together.
    #[cfg(test)]
    pub(crate) fn held(&self) -> usize {
        self.account.held.load(Ordering::Relaxed)
    }

    /// Bytes items have needed beyond the grants and rooms they were built
    /// out of, over the ledger's life ([`Charge::take`]).
    #[cfg(test)]
    pub(crate) fn short(&self) -> u64 {
        self.account.short.load(Ordering::Relaxed)
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

    /// Bytes the live charges built out of floor grants hold together, not
    /// counting what floor slots carry for others.
    #[cfg(test)]
    pub(crate) fn floor(&self) -> usize {
        self.account.floor.load(Ordering::Relaxed)
    }

    /// The invitation share's pace at `depth` ([`admission`]).
    #[cfg(test)]
    fn pace(&self, depth: usize) -> admission::Pace {
        admission::pace(&self.account, depth)
    }

    /// Bytes held by charges built out of grants of `unit` kind admitted
    /// against the budget, concerning scopes at `depth`, the bytes their
    /// slot carries for them included.
    #[cfg(test)]
    pub(crate) fn budgeted(&self, unit: Unit, depth: usize) -> usize {
        admission::budgeted(&self.account, unit, depth)
    }

    /// Bytes the floor slot of `unit` kind at `depth` carries for charges
    /// admitted against the budget ([`admission`], "Carrying").
    #[cfg(test)]
    pub(crate) fn carried(&self, unit: Unit, depth: usize) -> usize {
        admission::carried(&self.account, unit, depth)
    }

    /// The largest grant the ledger has admitted to a unit of `unit` kind
    /// at `depth`, through its floor slot or against the budget.
    #[cfg(test)]
    pub(crate) fn largest(&self, unit: Unit, depth: usize) -> usize {
        admission::largest(&self.account, unit, depth)
    }

    /// The outstanding claims of the questions about scopes at `depth`.
    #[cfg(test)]
    fn claimed(&self, depth: usize) -> usize {
        admission::claimed(&self.account, depth)
    }

    /// The largest sum any invitation share check has weighed.
    #[cfg(test)]
    fn share_peak(&self) -> usize {
        self.account.share_peak().sum
    }

    /// Bytes held outside floor slots: what the budget counts.
    #[cfg(test)]
    fn normal(&self) -> usize {
        admission::normal(&self.account)
    }
}

impl Account {
    /// Begin an empty account whose units are admitted against `budget`.
    fn new(budget: usize) -> Self {
        Self {
            #[cfg(any(test, feature = "test-internals"))]
            held: AtomicUsize::new(0),
            #[cfg(any(test, feature = "test-internals"))]
            peak: AtomicUsize::new(0),
            #[cfg(any(test, feature = "test-internals"))]
            places: Default::default(),
            #[cfg(any(test, feature = "test-internals"))]
            floor: AtomicUsize::new(0),
            #[cfg(any(test, feature = "test-internals"))]
            carried: AtomicUsize::new(0),
            kept: std::array::from_fn(|_| AtomicU64::new(0)),
            short: AtomicU64::new(0),
            waited: AtomicU64::new(0),
            sharing: std::array::from_fn(|_| AtomicUsize::new(0)),
            #[cfg(any(test, feature = "test-internals"))]
            share_peak: Mutex::default(),
            #[cfg(any(test, feature = "test-internals"))]
            slot_peaks: std::array::from_fn(|_| std::array::from_fn(|_| AtomicUsize::new(0))),
            budget,
            admission: Mutex::default(),
        }
    }

    /// Add `bytes` to the account of `place`, and to the floor part if
    /// `floor`, for a charge of `unit` ([`Charge::unit`]).
    ///
    /// The caller holds the admission queue's lock: `slots`, which only the
    /// lock reaches, counts what each kind and depth holds against the
    /// budget.
    fn add(
        &self,
        slots: &mut admission::Slots,
        place: Holding,
        bytes: usize,
        floor: bool,
        unit: Option<(Unit, usize)>,
    ) {
        if !floor {
            slots.normal += bytes;
            if let Some((unit, depth)) = unit {
                *slots.budgeted(unit, depth) += bytes;
            }
        }
        self.tally((None, Some(place)), bytes, floor, unit);
    }

    /// Return `bytes` from the account of `place`, and from the floor part
    /// if `floor`, for a charge of `unit` ([`Charge::unit`]), `carried` of
    /// them to what its kind and depth's floor slot carries.
    ///
    /// The caller holds the admission queue's lock, as `slots` proves
    /// ([`admission::release`]). A charge outside the floor that has a unit
    /// returns its bytes first to what its slot carries, at most all of
    /// them. The budget stopped counting those `carried` bytes when the slot
    /// took them, so only the rest leave its count, in one change. `carried`
    /// is zero for every other charge.
    fn sub(
        &self,
        slots: &mut admission::Slots,
        place: Holding,
        bytes: usize,
        floor: bool,
        unit: Option<(Unit, usize)>,
        carried: usize,
    ) {
        if !floor {
            slots.normal -= bytes - carried;
            if let Some((unit, depth)) = unit {
                *slots.budgeted(unit, depth) -= bytes;
            }
        }
        #[cfg(any(test, feature = "test-internals"))]
        self.carried.fetch_sub(carried, Ordering::Relaxed);
        self.tally((Some(place), None), bytes, floor, unit);
    }

    /// Count `bytes` of the charges of `unit` kind at `depth` admitted
    /// against the budget as carried by their floor slot, which admission
    /// has taken for them ([`admission`], "Carrying").
    ///
    /// The caller holds the admission queue's lock, as `slots` proves, and
    /// keeps the slot's share of the carried bytes, which are at most what
    /// the kind and depth hold against the budget.
    fn carry(&self, slots: &mut admission::Slots, bytes: usize, unit: Unit, depth: usize) {
        slots.normal -= bytes;
        #[cfg(any(test, feature = "test-internals"))]
        {
            self.carried.fetch_add(bytes, Ordering::Relaxed);
            self.slot_peaks[unit.row()][depth].fetch_max(bytes, Ordering::Relaxed);
        }
        self.tally((None, None), bytes, false, Some((unit, depth)));
    }

    /// Count one change in the totals only observation reads, and report it
    /// to the observation hook ([`observe`]).
    ///
    /// The change moves `bytes` of a charge of `unit`, through a floor slot
    /// if `floor`, out of the first place of `(from, to)` and into the
    /// second. A change with only `to` builds a charge, one with only `from` drops
    /// one, one with both moves bytes between places, and one with neither
    /// moves no charge's bytes, as a slot starting to carry does.
    #[cfg(any(test, feature = "test-internals"))]
    fn tally(
        &self,
        (from, to): (Option<Holding>, Option<Holding>),
        bytes: usize,
        floor: bool,
        unit: Option<(Unit, usize)>,
    ) {
        let held = match (from, to) {
            (None, Some(_)) => {
                if floor {
                    self.floor.fetch_add(bytes, Ordering::Relaxed);
                }
                let held = self.held.fetch_add(bytes, Ordering::Relaxed) + bytes;
                self.peak.fetch_max(held, Ordering::Relaxed);
                held
            }
            (Some(_), None) => {
                if floor {
                    self.floor.fetch_sub(bytes, Ordering::Relaxed);
                }
                self.held.fetch_sub(bytes, Ordering::Relaxed) - bytes
            }
            _ => self.held.load(Ordering::Relaxed),
        };
        if let Some(from) = from {
            self.places[place_index(from)].fetch_sub(bytes, Ordering::Relaxed);
        }
        if let Some(to) = to {
            self.places[place_index(to)].fetch_add(bytes, Ordering::Relaxed);
        }
        observe(self, (from, to), bytes, floor, unit, held);
    }

    /// Count one change in the totals only observation reads: nothing in
    /// builds that observe nothing.
    #[cfg(not(any(test, feature = "test-internals")))]
    #[inline(always)]
    fn tally(
        &self,
        _: (Option<Holding>, Option<Holding>),
        _: usize,
        _: bool,
        _: Option<(Unit, usize)>,
    ) {
    }

    /// The bytes units of `unit` kind concerning scopes at `depth` have
    /// kept out of their grants, if the pace reads them: answers' only.
    fn kept(&self, unit: Unit, depth: usize) -> Option<&AtomicU64> {
        match unit {
            Unit::Answer => Some(&self.kept[depth]),
            Unit::Question => None,
        }
    }

    /// Keep `check` as the share peak if it weighed more than every check
    /// before it.
    #[cfg(any(test, feature = "test-internals"))]
    fn weigh(&self, check: SharePeak) {
        let mut peak = self
            .share_peak
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if check.sum > peak.sum {
            *peak = check;
        }
    }

    /// The share peak and the terms of the check that weighed it.
    #[cfg(any(test, feature = "test-internals"))]
    fn share_peak(&self) -> SharePeak {
        *self
            .share_peak
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// Bytes charged for one place, returned to their ledger when dropped.
///
/// A charge with no ledger holds nothing: the [`Default`] charge, which
/// items carry where no ledger accounts for them, such as in tests that
/// build protocol values directly.
///
/// Every item carries its charges and is priced at its in-memory size,
/// charges included, so a charge is kept to 16 bytes on 64-bit targets: its
/// account's pointer, its bytes in four, its place, its floor slot, named in
/// one byte by kind and depth, and the kind and depth of its unit in the
/// two bytes of padding left.
pub(crate) struct Charge {
    /// The account the bytes are charged to, if any.
    account: Option<Arc<Account>>,
    /// The bytes charged.
    ///
    /// Every charge is a unit's grant or a part of one, and no grant
    /// exceeds the walk's largest, which a constant assertion keeps within
    /// `u32::MAX` ([`largest_grant`](super::materialized::largest_grant)).
    /// The ledger's totals of what it holds, which sum many live charges,
    /// are words; its cumulative counters, which grow over its life, are
    /// `u64`s.
    bytes: u32,
    /// The place the bytes are for.
    place: Holding,
    /// The floor slot the charge's unit took, if it was admitted through
    /// one; the charge's bytes are then the ledger's floor part, and the
    /// charge counts among the slot's sharers until it drops.
    slot: Option<admission::Slot>,
    /// The unit this charge was built out of, if admission granted it,
    /// and whether it is that unit's grant itself, whose drop returns what
    /// its unit did not use.
    ///
    /// This field must not grow a charge, since a charge prices every item
    /// that carries it: it fits in the padding beside `place` and `slot`,
    /// which the assertion below the type keeps.
    origin: Option<Origin>,
}

/// The unit a charge was built out of.
///
/// The ledger counts the bytes questions hold at each depth from it, which
/// the invitation share reads ([`admission`]), and the bytes each depth's
/// answers keep out of their grants, which the share's pace reads. In
/// builds that observe ledgers, the observation hook also reports every
/// change with it, so a test can tell which depths hold the budget, and the
/// admission hook hears what a grant returns unused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Origin {
    /// What the charge is to its unit.
    role: Role,
    /// The depth of the scope the unit concerns, at most 32.
    depth: u8,
}

/// What a charge is to the unit it was built out of, by the unit's kind:
/// one enumeration rather than a kind and a flag, so that an origin fits
/// two bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// An answer's grant.
    AnswerGrant,
    /// A question's grant.
    QuestionGrant,
    /// A part split off an answer's grant: an item built out of it, or a
    /// part on its way back to the ledger.
    AnswerPart,
    /// A part split off a question's grant.
    QuestionPart,
}

impl Origin {
    /// The origin of the grant of a unit of `unit` kind at `depth`.
    fn grant(unit: Unit, depth: usize) -> Self {
        let role = match unit {
            Unit::Answer => Role::AnswerGrant,
            Unit::Question => Role::QuestionGrant,
        };
        let depth = u8::try_from(depth).expect("a scope sits at most 32 bytes deep");
        Self { role, depth }
    }

    /// The origin of a part split off a charge of this origin.
    fn part(self) -> Self {
        let role = match self.role {
            Role::AnswerGrant | Role::AnswerPart => Role::AnswerPart,
            Role::QuestionGrant | Role::QuestionPart => Role::QuestionPart,
        };
        Self { role, ..self }
    }

    /// The unit's kind.
    fn unit(self) -> Unit {
        match self.role {
            Role::AnswerGrant | Role::AnswerPart => Unit::Answer,
            Role::QuestionGrant | Role::QuestionPart => Unit::Question,
        }
    }

    /// The unit's kind and depth, if the charge is the unit's grant.
    fn grant_of(self) -> Option<(Unit, usize)> {
        matches!(self.role, Role::AnswerGrant | Role::QuestionGrant)
            .then(|| (self.unit(), usize::from(self.depth)))
    }
}

/// A charge takes 16 bytes on 64-bit targets, where the session-budget pins
/// are measured, its unit's tag included: so the tag changes no price the
/// ledger charges.
#[cfg(target_pointer_width = "64")]
const _: () = assert!(
    size_of::<Charge>() == 16,
    "a charge prices every item that carries it, and must stay 16 bytes",
);

/// A charge's bytes widen losslessly into the ledger's word-sized totals.
const _: () = assert!(size_of::<u32>() <= size_of::<usize>());

/// A charge's `bytes` as the ledger's totals count them.
fn widen(bytes: u32) -> usize {
    // Lossless by the assertion above.
    bytes as usize
}

impl Charge {
    /// The bytes this charge holds.
    pub(crate) fn bytes(&self) -> usize {
        widen(self.bytes)
    }

    /// Whether this charge was built out of a floor grant.
    pub(crate) fn is_floor(&self) -> bool {
        self.slot.is_some()
    }

    /// Charge `bytes` for `place` on `account`, as a grant through `slot`
    /// if one is given, and as the grant of a unit of `unit` kind at its
    /// depth if one is given.
    ///
    /// A charge through `slot` must already be counted among the slot's
    /// sharers. The caller holds the admission queue's lock, as `slots`
    /// proves ([`Account::add`]).
    fn granted(
        account: &Arc<Account>,
        slots: &mut admission::Slots,
        place: Holding,
        bytes: u32,
        slot: Option<admission::Slot>,
        unit: Option<(Unit, usize)>,
    ) -> Charge {
        account.add(slots, place, widen(bytes), slot.is_some(), unit);
        if let Some(kept) = unit.and_then(|(unit, depth)| account.kept(unit, depth)) {
            kept.fetch_add(u64::from(bytes), Ordering::Relaxed);
        }
        Charge {
            account: Some(account.clone()),
            place,
            bytes,
            slot,
            origin: unit.map(|(unit, depth)| Origin::grant(unit, depth)),
        }
    }

    /// The kind and depth of the unit this charge was built out of, if
    /// admission granted it.
    fn unit(&self) -> Option<(Unit, usize)> {
        self.origin
            .map(|origin| (origin.unit(), usize::from(origin.depth)))
    }

    /// Move `bytes` of this charge, at most all of it, into a charge of its
    /// own for `place`, so the two parts return separately.
    ///
    /// The part shares this charge's floor slot, if any.
    pub(crate) fn split(&mut self, place: Holding, bytes: usize) -> Charge {
        let bytes = u32::try_from(bytes).map_or(self.bytes, |bytes| bytes.min(self.bytes));
        let Some(account) = &self.account else {
            return Charge::default();
        };
        if let Some(slot) = self.slot {
            slot.share(account);
        }
        account.tally(
            (Some(self.place), Some(place)),
            widen(bytes),
            self.slot.is_some(),
            self.unit(),
        );
        self.bytes -= bytes;
        Charge {
            account: Some(account.clone()),
            place,
            bytes,
            slot: self.slot,
            origin: self.origin.map(Origin::part),
        }
    }

    /// Return all of this charge but `bytes` to its ledger now.
    ///
    /// A grant reports the part it returns to the admission hook as unused
    /// before its unit is built ([`Admission::partial`]), and the rest as
    /// usual when it drops.
    ///
    /// The returned part is split off and dropped, so a floor grant keeps
    /// its slot: the part leaves the slot's sharers as it drops, while this
    /// charge still shares it. The drop reconsiders the waiters, so the
    /// returned bytes may admit a waiting unit at once.
    pub(crate) fn keep(&mut self, bytes: usize) {
        let returned = self.bytes().saturating_sub(bytes);
        if let (Some(account), Some((unit, depth))) =
            (&self.account, self.origin.and_then(Origin::grant_of))
        {
            if let Some(kept) = account.kept(unit, depth) {
                kept.fetch_sub(counter::widen(returned), Ordering::Relaxed);
            }
            #[cfg(any(test, feature = "test-internals"))]
            admission::settled(account, unit, depth, returned, self.slot.is_some(), true);
        }
        drop(self.split(self.place, returned));
    }

    /// Build an item's charge of `bytes` for `place` out of this grant.
    ///
    /// A grant bounds what its unit builds, so it always has the bytes; if
    /// it falls short, the item is still charged in full, through the
    /// grant's slot if any, and the shortfall is counted for the
    /// session-budget suite to report. The shortfall joins the ledger under
    /// the admission queue's lock, as every byte admitted against the budget
    /// does ([`admission::shortfall`]). A grant on no ledger builds charges
    /// on none.
    ///
    /// # Panics
    ///
    /// Panics if `bytes` exceeds `u32::MAX`: a programmer error, since
    /// every item is priced within the grant of its unit, and no grant
    /// exceeds the walk's largest, which fits a charge's four bytes.
    pub(crate) fn take(&mut self, place: Holding, bytes: usize) -> Charge {
        let mut taken = self.split(place, bytes);
        let Some(account) = &self.account else {
            return taken;
        };
        let short = bytes - taken.bytes();
        if short > 0 {
            let whole = u32::try_from(bytes).expect(
                "an item is priced within its unit's grant, which fits a charge's four bytes",
            );
            account
                .short
                .fetch_add(counter::widen(short), Ordering::Relaxed);
            admission::shortfall(account, place, short, taken.slot.is_some(), taken.unit());
            taken.bytes = whole;
        }
        taken
    }

    /// Fold `other` into this charge, which must be for the same ledger,
    /// place, and floor slot, or hold nothing.
    ///
    /// # Panics
    ///
    /// Panics if the two hold more than `u32::MAX` together: a programmer
    /// error, since a charge absorbs only other parts of its own unit's
    /// grant, which together hold no more than the grant.
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
        debug_assert!(
            self.slot == other.slot,
            "a charge absorbs only charges of its own floor slot, or of none as it is",
        );
        self.bytes = self
            .bytes
            .checked_add(other.bytes)
            .expect("parts of one grant hold no more than the grant, which fits four bytes");
        other.account = None;
        if let Some(slot) = other.slot {
            // The folded charge shares the slot no longer.
            admission::fold(self.account.as_ref().expect("checked above"), slot);
        }
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
            slot: None,
            origin: None,
        }
    }
}

/// Return the charge's bytes to its ledger.
impl Drop for Charge {
    /// Subtracts the charge from its place and from the ledger's total.
    ///
    /// Releasing bytes may admit a waiting unit; a charge's floor slot, if
    /// this was the last charge sharing it, frees after the bytes return.
    /// The waiters are reconsidered after both, so a unit waiting for the
    /// bytes or for the slot is admitted here if nothing else admits it
    /// first.
    ///
    /// All three happen in one critical section of the admission queue's
    /// lock, where a charge admitted against the budget returns first
    /// whatever bytes its slot carries for its kind and depth
    /// ([`admission::release`]).
    fn drop(&mut self) {
        let Some(account) = self.account.take() else {
            return;
        };
        let bytes = widen(self.bytes);
        if let Some((unit, depth)) = self.origin.and_then(Origin::grant_of) {
            if let Some(kept) = account.kept(unit, depth) {
                kept.fetch_sub(counter::widen(bytes), Ordering::Relaxed);
            }
            #[cfg(any(test, feature = "test-internals"))]
            admission::settled(&account, unit, depth, bytes, self.slot.is_some(), false);
        }
        admission::release(&account, self.place, bytes, self.slot, self.unit());
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
///
/// Every change to what the ledger holds in all, through its floor slots,
/// or carried by them is made, and reported, under the ledger's admission
/// queue lock, as is every invitation share check, so a report's `held`,
/// `floor`, `carried`, `floor_bound` and `share_peak` agree with one
/// another. A split moves bytes between places outside that
/// lock, so its report's `places` can include another thread's change made
/// as it reports. `short` and `waited` are counted outside the
/// lock: when the tasks that change the ledger run on several threads, a
/// report can include a shortfall whose bytes it does not yet show as held,
/// or miss a wait another thread has just counted. Both are exact when
/// every task that changes the ledger runs on one thread.
#[cfg(any(test, feature = "test-internals"))]
#[derive(Clone, Copy, Debug)]
pub struct LedgerChange {
    /// The ledger that changed, identified for as long as it lives.
    pub ledger: usize,
    /// The place the change took bytes from, if any: a release or a move
    /// between places.
    pub from: Option<Holding>,
    /// The place the change put bytes into, if any: a charge or a move
    /// between places.
    pub to: Option<Holding>,
    /// The bytes the change moved.
    pub bytes: usize,
    /// Whether they are bytes of a charge built out of a floor grant, which
    /// the budget does not count.
    pub through_floor: bool,
    /// The unit whose charge changed, if admission granted it: every charge
    /// a session makes, since each is a grant or a part of one.
    pub unit: Option<ChargeUnit>,
    /// Bytes the ledger holds in all after the change.
    pub held: usize,
    /// Bytes the ledger holds for each place after the change, in
    /// [`Holding::ALL`] order.
    pub places: [usize; PLACES],
    /// Bytes of `held` that charges built out of floor grants hold.
    pub floor: usize,
    /// Bytes of `held` that floor slots carry for charges admitted against
    /// the budget, which the budget does not count either: with `floor`,
    /// the ledger's floor part.
    ///
    /// A change that only moves bytes onto a slot names neither place, and
    /// the unit whose bytes the slot carries.
    pub carried: usize,
    /// Bytes items have needed beyond their grants, over the ledger's life.
    pub short: u64,
    /// Units that have waited for admission, over the ledger's life.
    pub waited: u64,
    /// The sum over floor slots of the largest grant each has admitted or
    /// carried: the `Σ F` the ledger's bound adds to its budget.
    pub floor_bound: usize,
    /// The largest sum any invitation share check has weighed so far, the
    /// least budget at which every check made so far would pass, with the
    /// terms of the check that weighed it.
    pub share_peak: SharePeak,
}

/// One invitation share check, as [`LedgerChange::share_peak`] reports the
/// one that weighed the largest sum: the question it decided and the
/// ledger's state at that moment.
///
/// The check admits a question about a scope at `depth` only while
/// `invited + grant + α · (claimed + need) ≤ B`, where `α` is the depth's
/// pace, `pace_actual / pace_predicted`, and the scaled term is rounded
/// down; `sum` is that left side. The [`Default`] check weighed nothing:
/// the report of a ledger that has made no check.
#[cfg(any(test, feature = "test-internals"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SharePeak {
    /// The sum the check weighed.
    pub sum: usize,
    /// The depth of the scope the question asks about: the root's 0
    /// through the leaves' 32.
    pub depth: usize,
    /// The bytes that depth's questions held against the budget: the room
    /// its invitations held, `I_d`.
    pub invited: usize,
    /// The question's grant, `c`.
    pub grant: usize,
    /// The depth's outstanding claims before the question's own: `N_d`,
    /// the needs of its questions admitted against the budget whose replies
    /// the stage had not finished.
    pub claimed: usize,
    /// The question's predicted need, `n`.
    pub need: usize,
    /// The bytes the depth's pace counted as kept: its numerator.
    pub pace_actual: u64,
    /// The needs the depth's pace counted as predicted: its denominator,
    /// never zero.
    pub pace_predicted: u64,
    /// The bytes the ledger held outside its floor slots as it made the
    /// check, `L − F`.
    pub normal: usize,
}

/// The unit a charge was built out of, as [`LedgerChange::unit`] reports
/// it.
#[cfg(any(test, feature = "test-internals"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChargeUnit {
    /// Whether the unit is a question; otherwise it is an answer's own
    /// items.
    pub question: bool,
    /// The depth of the scope the unit concerns: the root's 0 through the
    /// leaves' 32.
    pub depth: usize,
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

/// Report one change to `account` to the installed hook: `bytes` of a
/// charge of `unit`, through a floor slot if `floor`, moved out of the
/// first place of `(from, to)` and into the second, after which the ledger
/// holds `held`.
#[cfg(any(test, feature = "test-internals"))]
fn observe(
    account: &Account,
    (from, to): (Option<Holding>, Option<Holding>),
    bytes: usize,
    floor: bool,
    unit: Option<(Unit, usize)>,
    held: usize,
) {
    if let Some(hook) = HOOK.get() {
        hook(LedgerChange {
            ledger: std::ptr::from_ref(account) as usize,
            from,
            to,
            bytes,
            through_floor: floor,
            unit: unit.map(|(unit, depth)| ChargeUnit {
                question: unit == Unit::Question,
                depth,
            }),
            held,
            places: Holding::ALL
                .map(|place| account.places[place_index(place)].load(Ordering::Relaxed)),
            floor: account.floor.load(Ordering::Relaxed),
            carried: account.carried.load(Ordering::Relaxed),
            short: account.short.load(Ordering::Relaxed),
            waited: account.waited.load(Ordering::Relaxed),
            floor_bound: account
                .slot_peaks
                .iter()
                .flatten()
                .map(|peak| peak.load(Ordering::Relaxed))
                .sum(),
            share_peak: account.share_peak(),
        });
    }
}

/// One finished reply against the bound its question reserved for, as the
/// process's fit hook sees it.
#[cfg(any(test, feature = "test-internals"))]
#[derive(Clone, Copy, Debug)]
pub struct ReplyFit {
    /// The reaction slots the reply took: one per reaction, and one per
    /// child its match covers, which takes no reaction but a slot of the
    /// bound.
    pub reactions: usize,
    /// The reaction slots its reservation allowed.
    pub reaction_bound: usize,
    /// The listing entries its queries carried.
    pub listing: usize,
    /// The listing entries its reservation allowed.
    pub listing_bound: usize,
    /// The bytes the records the take derived from its questions held.
    pub records: usize,
    /// The bytes its reservation's records part held for them.
    pub records_bound: usize,
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
/// question reserved for, and whose questions derived records of `records`
/// bytes against a records part of `records_bound` bytes.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) fn fitted(bound: ReplyBound, held: ReplyBound, records_bound: usize, records: usize) {
    if let Some(hook) = FIT_HOOK.get() {
        hook(ReplyFit {
            reactions: held.reactions,
            reaction_bound: bound.reactions,
            listing: held.listing,
            listing_bound: bound.listing,
            records,
            records_bound,
        });
    }
}

/// Report a finished reply: nothing outside test builds.
#[cfg(not(any(test, feature = "test-internals")))]
#[inline(always)]
pub(crate) fn fitted(_: ReplyBound, _: ReplyBound, _: usize, _: usize) {}

#[cfg(test)]
mod tests;
