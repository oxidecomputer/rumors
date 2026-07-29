//! The in-memory reference [`Kv`] store; all user-facing documentation
//! lives on [`Memory`] itself (the module is private, the type
//! re-exported).

use std::collections::BTreeMap;
use std::ops::Bound;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use imbl::OrdMap;

use super::kv::{Kv, ReadTxn, Table, WriteTxn};

/// One committed state: every table's entries, structurally shared.
type State = OrdMap<(Table, Bytes), Bytes>;

/// The error a [`Memory`] store can report.
///
/// A fault-free `Memory` never fails; every variant is injected
/// instrumentation or a caller's own abort.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MemoryError {
    /// An injected fault fired (see [`Memory::inject_abort`] and
    /// [`Memory::inject_commit_then_error`]).
    #[error("injected storage fault")]
    Injected,
    /// A transaction closure aborted deliberately.
    #[error("transaction aborted by its closure")]
    Aborted,
}

/// What an injected write fault does when it fires.
#[derive(Debug, Clone, Copy)]
enum WriteFault {
    /// Abort the transaction: nothing applies, the caller sees `Err`.
    Abort,
    /// Commit the transaction in full, then report `Err` anyway: the
    /// committed-or-not ambiguity, resolved to "committed".
    CommitThenError,
}

#[derive(Debug, Default)]
struct Shared {
    committed: State,
    /// Every committed state, index 0 the empty store, when recording.
    history: Option<Vec<State>>,
    /// Scheduled faults keyed by the 0-based index (counted from now) of
    /// the write transaction they fire on.
    faults: BTreeMap<u64, WriteFault>,
    /// Write transactions attempted so far (the fault clock).
    writes: u64,
}

/// The in-memory reference [`Kv`] store.
///
/// The store the crate validates its own persistence layer against, and
/// the right store for tests, simulations, and conformance runs. Cloning
/// shares the store, per the [`Kv`] handle contract.
///
/// Beyond the plain contract it offers two pieces of instrumentation no
/// production store can:
///
/// - **A committed-state history.** Every committed transaction's
///   resulting state is retained (structural sharing makes each O(1)),
///   and [`reopen_at`](Memory::reopen_at) turns any committed prefix into
///   a fresh store — exactly the [`Kv`] contract's prefix-consistency
///   crash model, made enumerable. Crash tests iterate every prefix
///   instead of sampling a few.
/// - **Fault injection.** A scheduled write can abort with an error
///   ([`inject_abort`](Memory::inject_abort)) or commit and *then* report
///   failure ([`inject_commit_then_error`](Memory::inject_commit_then_error)
///   — the committed-or-not ambiguity of a dropped or offloaded commit,
///   made deterministic).
///
/// Both faces exist for test harnesses; neither perturbs the store's
/// conformance when unused.
#[derive(Debug, Clone, Default)]
pub struct Memory {
    shared: Arc<Mutex<Shared>>,
}

impl Memory {
    /// A fresh, empty store that keeps no history.
    pub fn new() -> Self {
        Self::default()
    }

    /// A fresh, empty store recording its committed-state history.
    pub fn recording() -> Self {
        Self {
            shared: Arc::new(Mutex::new(Shared {
                history: Some(vec![State::default()]),
                ..Shared::default()
            })),
        }
    }

    /// How many committed states the history holds: the initial empty
    /// state plus one per committed write. Zero when not recording.
    pub fn history_len(&self) -> usize {
        self.shared
            .lock()
            .expect("Memory lock poisoned")
            .history
            .as_ref()
            .map(Vec::len)
            .unwrap_or_default()
    }

    /// A fresh, recording store whose committed state is the history entry
    /// at `prefix` — the store as a crash surviving exactly that prefix of
    /// committed transactions would reopen it.
    ///
    /// # Panics
    ///
    /// If the store is not recording or `prefix` is out of range.
    pub fn reopen_at(&self, prefix: usize) -> Self {
        let shared = self.shared.lock().expect("Memory lock poisoned");
        let history = shared
            .history
            .as_ref()
            .expect("reopen_at requires a recording Memory");
        Self {
            shared: Arc::new(Mutex::new(Shared {
                committed: history[prefix].clone(),
                history: Some(vec![history[prefix].clone()]),
                ..Shared::default()
            })),
        }
    }

    /// Schedules the write transaction `nth` from now (0 = the next) to
    /// abort with [`MemoryError::Injected`], applying nothing.
    pub fn inject_abort(&self, nth: u64) {
        let mut shared = self.shared.lock().expect("Memory lock poisoned");
        let at = shared.writes + nth;
        shared.faults.insert(at, WriteFault::Abort);
    }

    /// Schedules the write transaction `nth` from now (0 = the next) to
    /// commit in full and then report [`MemoryError::Injected`] anyway.
    ///
    /// This is the dropped-commit ambiguity resolved to "committed", for
    /// tests that must exercise an acknowledgment the caller never saw.
    pub fn inject_commit_then_error(&self, nth: u64) {
        let mut shared = self.shared.lock().expect("Memory lock poisoned");
        let at = shared.writes + nth;
        shared.faults.insert(at, WriteFault::CommitThenError);
    }
}

/// A transaction view: a working copy of the committed state.
///
/// Reads see the snapshot plus this transaction's own mutations; the
/// working copy replaces the committed state only if the closure and the
/// fault schedule both let the commit through.
#[derive(Debug)]
pub struct MemoryTxn {
    state: State,
}

impl ReadTxn for MemoryTxn {
    type Error = MemoryError;

    fn get(&mut self, table: Table, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self
            .state
            .get(&(table, Bytes::copy_from_slice(key)))
            .map(|value| value.to_vec()))
    }

    fn next_after(
        &mut self,
        table: Table,
        after: Option<&[u8]>,
    ) -> Result<Option<(Vec<u8>, Vec<u8>)>, Self::Error> {
        let start = match after {
            None => Bound::Included((table, Bytes::new())),
            Some(key) => Bound::Excluded((table, Bytes::copy_from_slice(key))),
        };
        Ok(self
            .state
            .range((start, Bound::Unbounded))
            .next()
            .filter(|((entry_table, _), _)| *entry_table == table)
            .map(|((_, key), value)| (key.to_vec(), value.to_vec())))
    }
}

impl WriteTxn for MemoryTxn {
    fn put(&mut self, table: Table, key: &[u8], value: &[u8]) -> Result<(), Self::Error> {
        self.state.insert(
            (table, Bytes::copy_from_slice(key)),
            Bytes::copy_from_slice(value),
        );
        Ok(())
    }

    fn delete(&mut self, table: Table, key: &[u8]) -> Result<(), Self::Error> {
        self.state.remove(&(table, Bytes::copy_from_slice(key)));
        Ok(())
    }
}

impl Kv for Memory {
    type Error = MemoryError;

    async fn read<R, F>(&self, mut f: F) -> Result<R, Self::Error>
    where
        R: Send + 'static,
        F: FnMut(&mut dyn ReadTxn<Error = Self::Error>) -> Result<R, Self::Error> + Send + 'static,
    {
        let snapshot = self
            .shared
            .lock()
            .expect("Memory lock poisoned")
            .committed
            .clone();
        f(&mut MemoryTxn { state: snapshot })
    }

    async fn write<R, F>(&self, mut f: F) -> Result<R, Self::Error>
    where
        R: Send + 'static,
        F: FnMut(&mut dyn WriteTxn<Error = Self::Error>) -> Result<R, Self::Error> + Send + 'static,
    {
        // The whole transaction runs under the store lock: writes are
        // trivially serializable, and the closure never suspends (it is
        // synchronous by the trait's design).
        let mut shared = self.shared.lock().expect("Memory lock poisoned");
        let at = shared.writes;
        let fault = shared.faults.remove(&at);
        shared.writes += 1;

        if let Some(WriteFault::Abort) = fault {
            return Err(MemoryError::Injected);
        }

        let mut txn = MemoryTxn {
            state: shared.committed.clone(),
        };
        let result = f(&mut txn)?;

        shared.committed = txn.state;
        let committed = shared.committed.clone();
        if let Some(history) = shared.history.as_mut() {
            history.push(committed);
        }

        match fault {
            Some(WriteFault::CommitThenError) => Err(MemoryError::Injected),
            _ => Ok(result),
        }
    }
}

#[cfg(test)]
mod tests;
