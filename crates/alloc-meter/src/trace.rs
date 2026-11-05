//! Allocation lifetimes over a traced region of one thread's work.
//!
//! A [`Stats`](crate::Stats) reading counts bytes, but it cannot say which
//! of the bytes a region allocated are still live when the region ends. A
//! *trace* records every allocator request the calling thread makes while
//! it runs, in order, with each block's address and size, so that a later
//! analysis can follow every block from its allocation to its free:
//!
//! ```
//! let (trace, kept) = alloc_meter::trace(16, || {
//!     drop(Vec::<u8>::with_capacity(32));
//!     Vec::<u8>::with_capacity(8)
//! });
//! let lifetimes = trace.lifetimes().expect("sixteen events hold this region");
//! let unlabelled = lifetimes.label(alloc_meter::Label::NONE);
//! assert_eq!(unlabelled.transient_peak, 32);
//! assert_eq!(unlabelled.surviving_bytes, 8);
//! drop(kept);
//! ```
//!
//! # Classifying a block
//!
//! [`Trace::lifetimes`] sorts every block the region allocated into one of
//! two classes:
//!
//! - A *transient* block was allocated and freed inside the region.
//! - A *surviving* block was allocated inside the region and was still live
//!   when it ended.
//!
//! A block allocated before the region began is neither, even if the region
//! frees it: its free matches no allocation in the trace, and the analysis
//! ignores it. So does a block allocated inside [`untraced`].
//! [`Trace::survivors`] lists the surviving blocks themselves, so a caller can
//! check what owns them.
//!
//! The analysis reports, per [`Label`], the highest number of transient
//! bytes live at once over the region, the *transient peak*. Time is the
//! order of allocator requests. A reallocation counts as allocating the new
//! block and then freeing the old one, so the peak covers both blocks at
//! once, as [`Stats::peak_bytes`](crate::Stats::peak_bytes) does.
//!
//! # Labels
//!
//! [`labelled`] runs a closure with a label, and every block allocated
//! inside it carries that label. A block's bytes count toward the label it
//! was allocated under, whichever label is current when it is freed, so a
//! caller interleaving several independent computations on one thread can
//! attribute each block to the computation that allocated it.
//!
//! A label's transient peak is the highest its own bytes reach, so the peaks
//! of two labels need not occur together and their sum can exceed the peak
//! of both labels' bytes together. [`Trace::lifetimes_grouped`] classifies
//! the trace as though each label were replaced by the group a function
//! maps it to, so a caller who encodes several facts in one label (which
//! computation allocated a block, and what for) reads the peak of each group
//! as well as of each label.
//!
//! # Limits
//!
//! - A trace records into a buffer of the capacity [`trace`] is given,
//!   allocated before the region begins, because the allocator must not
//!   allocate while it records. A request that finds the buffer full is
//!   counted in [`Trace::dropped`] instead, and [`Trace::lifetimes`] then
//!   refuses to classify, since a lost free would turn a transient block
//!   into a surviving one.
//! - A trace sees only the calling thread, like the ledger, and for the same
//!   reasons (see [the crate docs](crate#why-each-thread-keeps-its-own-ledger)).
//!   A block the region allocates and another thread frees looks like a
//!   surviving block, unless the region later receives a new block at the
//!   same address; the analysis then ends the first block's lifetime there,
//!   and counts it in [`Lifetimes::reused_while_live`].

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[cfg(test)]
mod tests;

/// The attribution a block carries: the label current when it was
/// allocated (see [the module docs](self#labels)).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Label(pub u32);

impl Label {
    /// The label of a block allocated outside any [`labelled`] call.
    pub const NONE: Self = Self(0);
}

/// What one recorded allocator request did to one block.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventKind {
    /// A block became live, by a fresh allocation or as the new block of a
    /// reallocation.
    Allocated,
    /// A block was freed by a deallocation, or by a reallocation inside
    /// [`untraced`], whose new block the trace omits.
    Freed,
    /// The old block of a traced reallocation was freed. The event that
    /// follows it is always the new block's [`Allocated`](Self::Allocated),
    /// and the transient peak counts the two blocks live together.
    Resized,
}

/// One recorded allocator request's effect on one block.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Event {
    /// What happened to the block.
    pub kind: EventKind,
    /// The label current when the request was made.
    pub label: Label,
    /// The block's address.
    pub address: usize,
    /// The block's size in bytes, as its layout gives it.
    pub size: usize,
}

/// The calling thread's tracing state, read on every allocator request.
///
/// A `Copy` value in a `Cell` with a const initializer needs no destructor,
/// so reaching it never allocates, as [`crate::LEDGER`]'s comment explains.
#[derive(Clone, Copy, Debug)]
struct Mode {
    /// Whether a [`trace`] is running on this thread.
    tracing: bool,
    /// How many [`untraced`] calls enclose the current request; allocations
    /// are omitted while it is nonzero.
    untraced: u32,
    /// The label current for new blocks.
    label: Label,
}

impl Mode {
    /// The state every thread starts in.
    const IDLE: Self = Self {
        tracing: false,
        untraced: 0,
        label: Label::NONE,
    };
}

/// A trace's preallocated event buffer, with the count of events it had no
/// room for.
#[derive(Debug)]
struct Buffer {
    /// Recorded events, never grown while tracing.
    events: Vec<Event>,
    /// Events lost because `events` was full.
    dropped: usize,
}

// See the comment above `LEDGER` for the clippy allow.
thread_local! {
    /// The calling thread's tracing state.
    #[allow(clippy::missing_const_for_thread_local)]
    static MODE: Cell<Mode> = const { Cell::new(Mode::IDLE) };

    /// The calling thread's event buffer.
    ///
    /// Unlike [`MODE`], the buffer owns a vector and so has a destructor,
    /// whose registration on first access may allocate. The allocator
    /// reaches the buffer only while [`MODE`] says a trace is running, and
    /// [`trace`] touches the buffer before it sets that flag, so that first
    /// access, and any allocation it makes, happens outside the allocator.
    #[allow(clippy::missing_const_for_thread_local)]
    static BUFFER: RefCell<Buffer> = const {
        RefCell::new(Buffer { events: Vec::new(), dropped: 0 })
    };
}

/// Records one event when a trace is running on this thread.
///
/// Called from inside the allocator, so it must neither allocate nor panic.
/// Every access is fallible rather than panicking, and the push happens
/// only while the buffer has spare capacity, so it never grows the vector.
fn record(kind: EventKind, address: usize, size: usize) {
    let Ok(mode) = MODE.try_with(Cell::get) else {
        return;
    };
    if !mode.tracing || (kind == EventKind::Allocated && mode.untraced > 0) {
        return;
    }
    let event = Event {
        kind,
        label: mode.label,
        address,
        size,
    };
    let _ = BUFFER.try_with(|buffer| {
        if let Ok(mut buffer) = buffer.try_borrow_mut() {
            if buffer.events.len() < buffer.events.capacity() {
                buffer.events.push(event);
            } else {
                buffer.dropped = buffer.dropped.saturating_add(1);
            }
        }
    });
}

/// Whether allocations on this thread are currently omitted from its trace.
fn allocations_untraced() -> bool {
    MODE.try_with(|mode| mode.get().untraced > 0)
        .unwrap_or(true)
}

/// Records a successful fresh allocation.
pub(crate) fn allocated(address: usize, size: usize) {
    record(EventKind::Allocated, address, size);
}

/// Records a deallocation.
pub(crate) fn freed(address: usize, size: usize) {
    record(EventKind::Freed, address, size);
}

/// Records a successful reallocation of the block at `old_address` to a new
/// block at `new_address`, which may be the same address.
///
/// The old block's free comes first, so that an analysis matching frees to
/// allocations by address pairs it with the old block even when the system
/// resizes in place. Inside [`untraced`] the new block is omitted, and the
/// old block's free is recorded as a plain [`EventKind::Freed`], so that
/// every [`EventKind::Resized`] is followed by its new block.
pub(crate) fn reallocated(
    old_address: usize,
    old_size: usize,
    new_address: usize,
    new_size: usize,
) {
    if allocations_untraced() {
        record(EventKind::Freed, old_address, old_size);
    } else {
        record(EventKind::Resized, old_address, old_size);
        record(EventKind::Allocated, new_address, new_size);
    }
}

/// Restores the tracing state a scoped call changed, when the call ends
/// normally or by unwinding.
struct Restore(Mode);

impl Drop for Restore {
    /// Puts back the state the scoped call found.
    fn drop(&mut self) {
        MODE.set(self.0);
    }
}

/// The events recorded over one [`trace`] region.
#[derive(Debug)]
pub struct Trace {
    /// Every recorded event, in request order.
    events: Vec<Event>,
    /// Events lost to a full buffer.
    dropped: usize,
}

impl Trace {
    /// Returns the recorded events, in the order the requests were made.
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Returns how many events the trace lost because its buffer was full.
    pub fn dropped(&self) -> usize {
        self.dropped
    }

    /// Classifies every block the region allocated as transient or
    /// surviving, and finds each label's transient peak (see
    /// [the module docs](self#classifying-a-block)).
    ///
    /// # Errors
    ///
    /// Returns [`Incomplete`] when the trace dropped events, since any
    /// classification of an incomplete trace could be wrong.
    pub fn lifetimes(&self) -> Result<Lifetimes, Incomplete> {
        self.lifetimes_grouped(|label| label)
    }

    /// Classifies every block as [`lifetimes`](Self::lifetimes) does, with
    /// each event's label replaced by `group(label)` (see
    /// [the module docs](self#labels)).
    ///
    /// Each group's transient peak is the highest number of transient bytes
    /// live at once across every label in the group, which can be less than
    /// the sum of those labels' own peaks.
    ///
    /// # Errors
    ///
    /// Returns [`Incomplete`] when the trace dropped events, as
    /// [`lifetimes`](Self::lifetimes) does.
    pub fn lifetimes_grouped(
        &self,
        group: impl Fn(Label) -> Label,
    ) -> Result<Lifetimes, Incomplete> {
        let events = self.complete()?;
        Ok(Lifetimes::of_grouped(events, &resolve(events, group)))
    }

    /// Returns the allocation event of every surviving block, in request
    /// order: the blocks the region allocated that were live when it ended.
    ///
    /// A caller can check what owns the survivors by tracing the work that
    /// should free them and matching its frees against these blocks.
    ///
    /// # Errors
    ///
    /// Returns [`Incomplete`] when the trace dropped events, as
    /// [`lifetimes`](Self::lifetimes) does.
    pub fn survivors(&self) -> Result<Vec<Event>, Incomplete> {
        let events = self.complete()?;
        let (transient, _) = transience(events);
        Ok(events
            .iter()
            .zip(transient)
            .filter(|(event, transient)| event.kind == EventKind::Allocated && !transient)
            .map(|(event, _)| *event)
            .collect())
    }

    /// Returns the peak transient level of each group of labels, and of
    /// every group together, within each segment of the trace that `starts`
    /// marks.
    ///
    /// An event's *level* for a group is the bytes of the group's transient
    /// blocks live during it, counted as the classification counts them: a
    /// block a reallocation moves stays live through the allocation of its
    /// new block, and a block whose address is reused stays live through the
    /// reusing allocation (see [`Lifetimes::reused_while_live`]). `group`
    /// maps each label to the index of its group, below `groups`, or to
    /// `None` to leave the label's blocks out.
    ///
    /// The segments partition the events in order: the first runs from the
    /// trace's start to `starts[0]`, segment `k` from `starts[k - 1]` to
    /// `starts[k]`, and the last from the last start to the trace's end, so
    /// there is one more segment than starts. A segment holding no event,
    /// such as one between equal starts or one beginning at or beyond the
    /// trace's end, peaks at zero. A caller holding the trace's [`position`]
    /// at moments of its own, such as the changes of an account it keeps
    /// beside the trace, passes those positions as the starts. The account
    /// then holds one value through each segment, and each group's peak
    /// within the segment is the most its level reached against that value.
    ///
    /// # Panics
    ///
    /// Panics if `starts` decreases anywhere, or if `group` maps a label to
    /// an index of `groups` or above.
    ///
    /// # Errors
    ///
    /// Returns [`Incomplete`] when the trace dropped events, as
    /// [`lifetimes`](Self::lifetimes) does.
    pub fn segment_peaks(
        &self,
        starts: &[usize],
        groups: usize,
        group: impl Fn(Label) -> Option<usize>,
    ) -> Result<SegmentPeaks, Incomplete> {
        let events = self.complete()?;
        let resolved = resolve(events, |label| {
            group(label).inspect(|&index| {
                assert!(
                    index < groups,
                    "{label:?} maps to group {index}, beyond the {groups} groups"
                );
            })
        });
        Ok(SegmentPeaks::of(events, starts, groups, &resolved))
    }

    /// Returns the events when the trace recorded every request, or the
    /// refusal to classify an incomplete one.
    fn complete(&self) -> Result<&[Event], Incomplete> {
        if self.dropped > 0 {
            return Err(Incomplete {
                recorded: self.events.len(),
                dropped: self.dropped,
            });
        }
        Ok(&self.events)
    }
}

/// The refusal to classify a trace that lost events to a full buffer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Incomplete {
    /// Events the trace recorded, which is its buffer's capacity.
    pub recorded: usize,
    /// Events it had no room for: a trace given `recorded + dropped`
    /// capacity would have held them all.
    pub dropped: usize,
}

/// The blocks one label's allocations left, classified by lifetime.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LabelLifetimes {
    /// The highest number of transient bytes the label held live at once.
    pub transient_peak: usize,
    /// Blocks allocated and freed inside the region.
    pub transient_blocks: usize,
    /// Total bytes of those blocks, over the whole region.
    pub transient_bytes: usize,
    /// Blocks allocated inside the region and live when it ended.
    pub surviving_blocks: usize,
    /// Total bytes of those blocks.
    pub surviving_bytes: usize,
}

/// Every label's classified blocks over one trace.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Lifetimes {
    /// Each label that allocated at least one block, with its blocks.
    labels: BTreeMap<Label, LabelLifetimes>,
    /// Allocations whose address was already held by a live traced block,
    /// which must have been freed where the trace could not see.
    pub reused_while_live: usize,
}

impl Lifetimes {
    /// Returns the classified blocks of `label`, all zero if it allocated
    /// nothing.
    pub fn label(&self, label: Label) -> LabelLifetimes {
        self.labels.get(&label).copied().unwrap_or_default()
    }

    /// Classifies the blocks of an event sequence in request order.
    #[cfg(test)]
    fn of(events: &[Event]) -> Self {
        Self::of_grouped(events, &resolve(events, |label| label))
    }

    /// Classifies the blocks of an event sequence in request order, with
    /// each event's label replaced by its group in `groups`, which
    /// [`resolve`] built from the same events.
    ///
    /// The first pass finds which blocks are transient ([`transience`]). The
    /// second replays the events, pairing them the same way, and holds each
    /// transient block's bytes live under its allocation's group from its
    /// allocation to the event that ends it. A [`Resized`](EventKind::Resized)
    /// free takes effect only after the next event, the new block's
    /// allocation.
    fn of_grouped(events: &[Event], groups: &Resolved<Label>) -> Self {
        let group = |label| groups.of(label);
        let (transient, reused_while_live) = transience(events);
        let mut lifetimes = Self {
            reused_while_live,
            ..Self::default()
        };
        let mut pairing = Pairing::default();
        let mut held: BTreeMap<Label, usize> = BTreeMap::new();
        let mut deferred: Option<&Event> = None;
        for (index, event) in events.iter().enumerate() {
            if event.kind == EventKind::Allocated {
                let label = group(event.label);
                let entry = lifetimes.labels.entry(label).or_default();
                if transient[index] {
                    entry.transient_blocks += 1;
                    entry.transient_bytes += event.size;
                    let level = held.entry(label).or_default();
                    *level += event.size;
                    entry.transient_peak = entry.transient_peak.max(*level);
                } else {
                    entry.surviving_blocks += 1;
                    entry.surviving_bytes += event.size;
                }
            }
            // A resize's old block stays live through the event after it,
            // the new block's allocation, so the peak above counted both.
            if let Some(block) = deferred.take() {
                release(&mut held, group(block.label), block.size);
            }
            if let Some(allocation) = pairing.ends(index, event) {
                let block = &events[allocation];
                if event.kind == EventKind::Resized {
                    deferred = Some(block);
                } else {
                    release(&mut held, group(block.label), block.size);
                }
            }
        }
        lifetimes
    }
}

/// Each distinct label of an event sequence, sorted, with the value a
/// caller's function maps it to.
///
/// The analyses that take a caller's function over labels call it once per
/// distinct label here and replay the events through [`Resolved::of`]. The
/// replay is then not generic over the function, so it is compiled with this
/// crate rather than with each caller. A profile's `opt-level` for this
/// crate therefore optimizes every replay, even for a caller built without
/// optimization, as a test binary in the dev profile is.
#[derive(Debug)]
struct Resolved<T>(Vec<(Label, T)>);

impl<T: Copy> Resolved<T> {
    /// The value `label` maps to.
    ///
    /// # Panics
    ///
    /// Panics if `label` is not one of the resolved events' labels.
    fn of(&self, label: Label) -> T {
        let index = self
            .0
            .binary_search_by_key(&label, |&(resolved, _)| resolved)
            .expect("every label of the events is resolved");
        self.0[index].1
    }
}

/// Maps each distinct label of `events` through `f`, calling it once per
/// label.
fn resolve<T>(events: &[Event], f: impl Fn(Label) -> T) -> Resolved<T> {
    Resolved(
        distinct_labels(events)
            .into_iter()
            .map(|label| (label, f(label)))
            .collect(),
    )
}

/// Returns the distinct labels of `events`, sorted.
fn distinct_labels(events: &[Event]) -> Vec<Label> {
    let mut labels = BTreeSet::new();
    let mut last = None;
    for event in events {
        // Consecutive events usually share a label, so a repeat skips the set.
        if last != Some(event.label) {
            labels.insert(event.label);
            last = Some(event.label);
        }
    }
    labels.into_iter().collect()
}

/// The peak transient level of each group of labels, and of every group
/// together, within each segment of a trace, as [`Trace::segment_peaks`]
/// computes them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SegmentPeaks {
    /// The number of groups.
    groups: usize,
    /// Each segment's peaks in order, one row per segment: each group's
    /// peak, then the peak of every group together.
    rows: Vec<usize>,
}

impl SegmentPeaks {
    /// Returns the number of segments: one more than the starts that
    /// marked them.
    pub fn len(&self) -> usize {
        self.rows.len() / (self.groups + 1)
    }

    /// Returns whether there are no segments, which never holds: even no
    /// starts mark one segment, the whole trace.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Returns the peak level of each group within `segment`, by group
    /// index.
    ///
    /// # Panics
    ///
    /// Panics if `segment` is not below [`len`](Self::len).
    pub fn groups(&self, segment: usize) -> &[usize] {
        &self.row(segment)[..self.groups]
    }

    /// Returns the peak level of every group together within `segment`,
    /// which can be less than the sum of the groups' own peaks there.
    ///
    /// # Panics
    ///
    /// Panics if `segment` is not below [`len`](Self::len).
    pub fn all(&self, segment: usize) -> usize {
        self.row(segment)[self.groups]
    }

    /// Returns `segment`'s row.
    fn row(&self, segment: usize) -> &[usize] {
        let width = self.groups + 1;
        &self.rows[segment * width..][..width]
    }

    /// Computes each segment's peaks over `events` by replaying them once,
    /// with each label's group, among `groups`, looked up in `resolved`.
    ///
    /// The replay pairs events with the blocks they end as the
    /// classification does, and keeps each group's level and their total.
    /// A level rises only at the allocation of an included transient block,
    /// so a segment's peaks are its levels at its first event, raised at
    /// each such allocation within it.
    fn of(
        events: &[Event],
        starts: &[usize],
        groups: usize,
        resolved: &Resolved<Option<usize>>,
    ) -> Self {
        assert!(starts.is_sorted(), "segment starts must not decrease");
        let (transient, _) = transience(events);
        let width = groups + 1;
        let total = groups;
        let mut rows = vec![0; (starts.len() + 1) * width];
        // Each group's level, then their total.
        let mut levels = vec![0usize; width];
        // Each live included transient block's address, with its size and
        // group.
        let mut live: HashMap<usize, (usize, usize)> = HashMap::new();
        // Blocks that have ended but stay counted until the allocation
        // completing their event, as group and size.
        let mut ending: Vec<(usize, usize)> = Vec::new();
        let (mut segment, mut opened, mut fresh) = (0, 0, true);
        for (index, (event, transient)) in events.iter().zip(transient).enumerate() {
            while opened < starts.len() && starts[opened] <= index {
                segment += 1;
                opened += 1;
                fresh = true;
            }
            let mut risen = None;
            match event.kind {
                EventKind::Allocated => {
                    let group = if transient {
                        resolved.of(event.label)
                    } else {
                        None
                    };
                    let previous = match group {
                        Some(group) => {
                            levels[group] += event.size;
                            levels[total] += event.size;
                            risen = Some(group);
                            live.insert(event.address, (event.size, group))
                        }
                        None => live.remove(&event.address),
                    };
                    if let Some((size, group)) = previous {
                        ending.push((group, size));
                    }
                }
                EventKind::Resized => {
                    if let Some((size, group)) = live.remove(&event.address) {
                        ending.push((group, size));
                    }
                }
                EventKind::Freed => {
                    if let Some((size, group)) = live.remove(&event.address) {
                        levels[group] -= size;
                        levels[total] -= size;
                    }
                }
            }
            let row = &mut rows[segment * width..][..width];
            if fresh {
                row.copy_from_slice(&levels);
                fresh = false;
            } else if let Some(group) = risen {
                row[group] = row[group].max(levels[group]);
                row[total] = row[total].max(levels[total]);
            }
            if event.kind == EventKind::Allocated {
                for (group, size) in ending.drain(..) {
                    levels[group] -= size;
                    levels[total] -= size;
                }
            }
        }
        Self { groups, rows }
    }
}

/// Matches events to the traced blocks whose lifetimes they end, replaying
/// them in request order.
///
/// A free ends the most recent traced allocation at its address that no
/// earlier event has ended. A free with no such allocation belongs to a
/// block from before the region or from inside [`untraced`], and ends
/// nothing. An allocation at an address a live traced block still holds
/// ends that block: it was freed where the trace could not see, and the new
/// allocation is the latest its lifetime can have ended.
#[derive(Debug, Default)]
struct Pairing {
    /// Each live traced block's address, with its allocation's event index.
    live: HashMap<usize, usize>,
}

impl Pairing {
    /// Returns the event index of the allocation whose block the event at
    /// `index` ends, if any. Must be called for every event in order.
    fn ends(&mut self, index: usize, event: &Event) -> Option<usize> {
        match event.kind {
            EventKind::Allocated => self.live.insert(event.address, index),
            EventKind::Freed | EventKind::Resized => self.live.remove(&event.address),
        }
    }
}

/// Returns, for each event, whether it allocates a transient block, with the
/// count of allocations that ended a live block by reusing its address.
fn transience(events: &[Event]) -> (Vec<bool>, usize) {
    let mut transient = vec![false; events.len()];
    let mut reused = 0;
    let mut pairing = Pairing::default();
    for (index, event) in events.iter().enumerate() {
        if let Some(allocation) = pairing.ends(index, event) {
            transient[allocation] = true;
            reused += usize::from(event.kind == EventKind::Allocated);
        }
    }
    (transient, reused)
}

/// Removes a transient block's `size` bytes from the live level of `label`,
/// the group of the label it was allocated under.
fn release(held: &mut BTreeMap<Label, usize>, label: Label, size: usize) {
    let level = held
        .get_mut(&label)
        .expect("a transient block is counted live under its group from its allocation");
    *level -= size;
}

/// Runs `f` and returns the trace of every allocator request the calling
/// thread made while it ran, with `f`'s result.
///
/// `capacity` is the number of events the trace can hold, reserved before
/// `f` runs; each allocation or free is one event and each reallocation two.
/// On a system that overcommits memory, a reservation is address space,
/// committed only as events fill it; a system that does not, such as
/// illumos, reserves backing store for the whole capacity at once, so size
/// the capacity near what `f` needs. The trace counts events it has no room
/// for (see [`Trace::dropped`]).
///
/// The trace is independent of the ledger: requests inside it reach
/// [`measure`](crate::measure) and [`reading`](crate::reading) as usual.
///
/// # Panics
///
/// Panics if a trace is already running on this thread; traces do not nest.
/// A panic in `f` propagates, after tracing stops.
pub fn trace<T>(capacity: usize, f: impl FnOnce() -> T) -> (Trace, T) {
    let mode = MODE.get();
    assert!(!mode.tracing, "a trace is already running on this thread");
    let events = Vec::with_capacity(capacity);
    // Installing the buffer is the first access that registers its
    // destructor; tracing is still off, so nothing it allocates is traced.
    let _ = BUFFER.replace(Buffer { events, dropped: 0 });
    let value = {
        let _restore = Restore(mode);
        MODE.set(Mode {
            tracing: true,
            ..mode
        });
        f()
    };
    let buffer = BUFFER.replace(Buffer {
        events: Vec::new(),
        dropped: 0,
    });
    (
        Trace {
            events: buffer.events,
            dropped: buffer.dropped,
        },
        value,
    )
}

/// Returns the index the next event of the trace running on this thread
/// will take, its events recorded and dropped so far, or `None` when no
/// trace is running.
///
/// Code inside a traced region can note the position at moments of its
/// own, and line them up afterwards with [`Trace::segment_peaks`]. It
/// neither allocates nor panics, so the allocator's own bookkeeping is
/// undisturbed.
pub fn position() -> Option<usize> {
    let mode = MODE.try_with(Cell::get).ok()?;
    if !mode.tracing {
        return None;
    }
    BUFFER
        .try_with(|buffer| {
            buffer
                .try_borrow()
                .ok()
                .map(|buffer| buffer.events.len() + buffer.dropped)
        })
        .ok()
        .flatten()
}

/// Runs `f` with `label` current, so every block allocated inside it,
/// outside any nested `labelled` call, carries `label`.
///
/// Labels nest: the innermost call's label applies. Outside a [`trace`]
/// the label is recorded nowhere, and the call only runs `f`.
pub fn labelled<T>(label: Label, f: impl FnOnce() -> T) -> T {
    let mode = MODE.get();
    let _restore = Restore(mode);
    MODE.set(Mode { label, ..mode });
    f()
}

/// Runs `f` with its allocations omitted from any running [`trace`].
///
/// Frees inside `f` are still recorded, so a traced block that `f` frees
/// still ends its lifetime there. A block allocated inside `f` is invisible
/// to the analysis wherever it is later freed, because its free matches no
/// traced allocation. The omission suits buffers that a measurement's
/// harness owns, such as a simulated link's, which belong to neither the
/// measured work nor its result.
pub fn untraced<T>(f: impl FnOnce() -> T) -> T {
    let mode = MODE.get();
    let _restore = Restore(mode);
    MODE.set(Mode {
        untraced: mode.untraced.saturating_add(1),
        ..mode
    });
    f()
}
