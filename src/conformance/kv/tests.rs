//! The suite passes the honest reference store and has teeth: a store
//! that tears aborted transactions, disorders its cursor, or loses
//! concurrent increments is caught by name.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use super::*;
use crate::store::{Memory, MemoryError, ReadTxn, WriteTxn};

/// The reference store conforms.
#[pollster::test]
async fn memory_conforms() {
    check(async || Memory::new(), || MemoryError::Aborted).await;
}

/// Which clause the lying store violates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lie {
    /// Apply a transaction's effects directly to shared state, so a
    /// closure `Err` leaves everything it already put.
    Torn,
    /// Skip every other entry in cursor walks.
    SkippingCursor,
    /// Drop one increment in sixteen: apply blind last-writer-wins from a
    /// stale read under concurrency.
    LostUpdate,
}

/// The lying store's rows: every table's entries, in one flat map.
type Rows = BTreeMap<(Table, Vec<u8>), Vec<u8>>;

/// A deliberately non-conforming store: the negative control proving the
/// suite catches what it claims to. Each instance tells exactly one lie;
/// everything else is an honest direct-apply map.
#[derive(Debug, Clone)]
struct Lying {
    lie: Lie,
    state: Arc<Mutex<Rows>>,
    writes: Arc<Mutex<u64>>,
}

impl Lying {
    fn new(lie: Lie) -> Self {
        Self {
            lie,
            state: Arc::new(Mutex::new(BTreeMap::new())),
            writes: Arc::new(Mutex::new(0)),
        }
    }
}

/// The lying store's transaction view.
///
/// `Torn` applies effects straight to the shared map (no working copy);
/// the others stage a working copy like an honest store.
struct LyingTxn {
    lie: Lie,
    /// The working copy (`None` for `Torn`, which mutates shared state).
    staged: Option<Rows>,
    shared: Arc<Mutex<Rows>>,
}

impl LyingTxn {
    fn with<R>(&mut self, f: impl FnOnce(&mut Rows) -> R) -> R {
        match self.staged.as_mut() {
            Some(staged) => f(staged),
            None => f(&mut self.shared.lock().unwrap()),
        }
    }
}

impl ReadTxn for LyingTxn {
    type Error = MemoryError;

    fn get(&mut self, table: Table, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self.with(|state| state.get(&(table, key.to_vec())).cloned()))
    }

    fn next_after(
        &mut self,
        table: Table,
        after: Option<&[u8]>,
    ) -> Result<Option<(Vec<u8>, Vec<u8>)>, Self::Error> {
        let skip = usize::from(self.lie == Lie::SkippingCursor && after.is_some());
        Ok(self.with(|state| {
            state
                .iter()
                .filter(|((entry_table, key), _)| {
                    *entry_table == table && after.is_none_or(|after| key.as_slice() > after)
                })
                .nth(skip)
                .map(|((_, key), value)| (key.clone(), value.clone()))
        }))
    }
}

impl WriteTxn for LyingTxn {
    fn put(&mut self, table: Table, key: &[u8], value: &[u8]) -> Result<(), Self::Error> {
        self.with(|state| state.insert((table, key.to_vec()), value.to_vec()));
        Ok(())
    }

    fn delete(&mut self, table: Table, key: &[u8]) -> Result<(), Self::Error> {
        self.with(|state| state.remove(&(table, key.to_vec())));
        Ok(())
    }
}

impl Kv for Lying {
    type Error = MemoryError;

    async fn read<R, F>(&self, mut f: F) -> Result<R, Self::Error>
    where
        R: Send + 'static,
        F: FnMut(&mut dyn ReadTxn<Error = Self::Error>) -> Result<R, Self::Error> + Send + 'static,
    {
        f(&mut LyingTxn {
            lie: self.lie,
            staged: Some(self.state.lock().unwrap().clone()),
            shared: self.state.clone(),
        })
    }

    async fn write<R, F>(&self, mut f: F) -> Result<R, Self::Error>
    where
        R: Send + 'static,
        F: FnMut(&mut dyn WriteTxn<Error = Self::Error>) -> Result<R, Self::Error> + Send + 'static,
    {
        // A `LostUpdate` liar reads its base state, then lets another
        // writer land in between: yielding here gives a concurrent
        // sibling the window a real race would find.
        let staged = (self.lie != Lie::Torn).then(|| self.state.lock().unwrap().clone());
        if self.lie == Lie::LostUpdate {
            tokio::task::yield_now().await;
        }
        let mut txn = LyingTxn {
            lie: self.lie,
            staged,
            shared: self.state.clone(),
        };
        let result = f(&mut txn)?;
        if let Some(staged) = txn.staged {
            let drop_this = {
                let mut writes = self.writes.lock().unwrap();
                *writes += 1;
                self.lie == Lie::LostUpdate && writes.is_multiple_of(16)
            };
            if !drop_this {
                *self.state.lock().unwrap() = staged;
            }
        }
        Ok(result)
    }
}

/// A store that leaks an aborted transaction's effects fails the
/// atomicity clause by name.
#[pollster::test]
#[should_panic(expected = "an aborted transaction applies nothing")]
async fn torn_store_is_caught() {
    check(async || Lying::new(Lie::Torn), || MemoryError::Aborted).await;
}

/// A store whose cursor skips entries fails the cursor clause by name.
#[pollster::test]
#[should_panic(expected = "the cursor visits every key")]
async fn skipping_cursor_is_caught() {
    check(
        async || Lying::new(Lie::SkippingCursor),
        || MemoryError::Aborted,
    )
    .await;
}

/// A store that loses updates under concurrency fails the serialization
/// clause by name. Runs under tokio so concurrent writers genuinely
/// interleave at the liar's yield point.
#[tokio::test]
#[should_panic(expected = "concurrent write transactions serialize")]
async fn lost_updates_are_caught() {
    check(
        async || Lying::new(Lie::LostUpdate),
        || MemoryError::Aborted,
    )
    .await;
}
