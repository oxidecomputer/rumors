//! Allocation lifetimes over a traced region of one thread's work.
//!
//! A [`Stats`](crate::Stats) reading counts bytes, but it cannot say which
//! of the bytes a region allocated are still live when the region ends. A
//! *trace* records every allocator request the calling thread makes while
//! it runs, in order, with each block's address and size, so that a later
//! analysis can follow every block from its allocation to its free.
//!
//! # Labels
//!
//! [`labelled`] runs a closure with a label, and every block allocated
//! inside it carries that label. A block's bytes count toward the label it
//! was allocated under, whichever label is current when it is freed, so a
//! caller interleaving several independent computations on one thread can
//! attribute each block to the computation that allocated it.
//!
//! # Limits
//!
//! - A trace records into a buffer of the capacity [`trace`] is given,
//!   allocated before the region begins, because the allocator must not
//!   allocate while it records. A request that finds the buffer full is
//!   counted in [`Trace::dropped`] instead.
//! - A trace sees only the calling thread, like the ledger, and for the same
//!   reasons (see [the crate docs](crate#why-each-thread-keeps-its-own-ledger)).

use std::cell::{Cell, RefCell};

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
