//! Shared deterministic poll delays for test instruments.

#[cfg(test)]
use std::cell::RefCell;
use std::task::Context;

/// Largest number of self-waking suspensions assigned to one poll.
///
/// Two exercises both single and repeated suspension while bounding the work
/// generated schedules can add at each boundary.
pub(crate) const MAX_SCHEDULED_DELAY: u8 = 2;

/// The independently configurable schedules used by streaming tests.
#[derive(Clone, Copy)]
#[cfg(test)]
enum ScheduleKind {
    /// Delays at in-memory backend operations.
    Backend,
    /// Delays at bounded-channel operations.
    Channel,
}

/// Delays consumed in order by one class of poll boundary.
#[cfg(test)]
struct Schedule {
    /// Pending-count choices for successive polls.
    delays: Vec<u8>,
    /// The next choice to consume.
    step: usize,
}

// Clippy can reject const-block TLS initializers on targets that use fallback
// thread-local storage, so these targeted allows keep the cross-platform gate
// consistent.
#[cfg(test)]
std::thread_local! {
    /// Schedule active at backend poll boundaries on this test thread.
    #[allow(clippy::missing_const_for_thread_local)]
    static BACKEND: RefCell<Option<Schedule>> = const { RefCell::new(None) };
    /// Schedule active at channel poll boundaries on this test thread.
    #[allow(clippy::missing_const_for_thread_local)]
    static CHANNEL: RefCell<Option<Schedule>> = const { RefCell::new(None) };
}

/// Access to one thread-local poll schedule.
#[cfg(test)]
#[derive(Clone, Copy)]
pub(crate) struct ScheduleCell(ScheduleKind);

/// The schedule applied at in-memory backend operations.
#[cfg(test)]
pub(crate) const BACKEND_SCHEDULE: ScheduleCell = ScheduleCell(ScheduleKind::Backend);

/// The schedule applied at bounded-channel operations.
#[cfg(test)]
pub(crate) const CHANNEL_SCHEDULE: ScheduleCell = ScheduleCell(ScheduleKind::Channel);

#[cfg(test)]
impl ScheduleCell {
    /// Borrow this schedule's thread-local storage.
    fn access<R>(self, f: impl FnOnce(&RefCell<Option<Schedule>>) -> R) -> R {
        match self.0 {
            ScheduleKind::Backend => BACKEND.with(f),
            ScheduleKind::Channel => CHANNEL.with(f),
        }
    }

    /// Run `f` with `delays` active, restoring any enclosing schedule afterward.
    pub(crate) fn with<R>(self, delays: Vec<u8>, f: impl FnOnce() -> R) -> R {
        /// Restore an enclosing schedule after return or panic.
        struct Restore {
            /// Schedule cell to restore.
            cell: ScheduleCell,
            /// State displaced by the nested schedule.
            previous: Option<Schedule>,
        }

        /// Restore the displaced schedule.
        impl Drop for Restore {
            /// Reinstate the schedule saved on entry.
            fn drop(&mut self) {
                self.cell.access(|cell| cell.replace(self.previous.take()));
            }
        }

        let previous = self.access(|cell| cell.replace(Some(Schedule { delays, step: 0 })));
        let _restore = Restore {
            cell: self,
            previous,
        };
        f()
    }

    /// Consume the next bounded delay, or return zero outside a schedule.
    pub(crate) fn next(self) -> u8 {
        self.access(|cell| {
            let mut schedule = cell.borrow_mut();
            let Some(current) = schedule.as_mut() else {
                return 0;
            };
            let delay = current.delays.get(current.step).copied().unwrap_or(0);
            current.step += 1;
            delay.min(MAX_SCHEDULED_DELAY)
        })
    }
}

/// Pending polls remaining before a wrapped operation may be polled.
#[derive(Default)]
pub(crate) struct Countdown(Option<u8>);

impl Countdown {
    /// Spend one scheduled delay, self-waking when the caller must suspend.
    pub(crate) fn suspend(&mut self, next: impl FnOnce() -> u8, cx: &Context<'_>) -> bool {
        if self.0.is_none() {
            self.0 = Some(next());
        }
        let Some(remaining) = self.0.as_mut() else {
            return false;
        };
        if *remaining == 0 {
            return false;
        }
        *remaining -= 1;
        cx.waker().wake_by_ref();
        true
    }

    /// Begin a fresh delay before the wrapped operation's next poll.
    pub(crate) fn clear(&mut self) {
        self.0 = None;
    }

    /// Report whether this operation has begun its scheduled delay.
    pub(crate) fn is_active(&self) -> bool {
        self.0.is_some()
    }
}
