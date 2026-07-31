//! Corruption-aware transactions over a raw [`Kv`] store.
//!
//! The [`Kv`] contract fixes each transaction closure's error type to the
//! store's own, which leaves no channel for the one failure the backend
//! itself detects mid-transaction: a stored row that fails to decode
//! ([`Corruption`](super::Corruption)). This module widens the channel.
//! [`read`] and [`write()`] run closures whose transaction views carry
//! [`KvError<E>`](KvError) as their error type, so decode doors deep in
//! the schema and custody layers refuse with plain `?`, and the refusal
//! surfaces to the caller as [`KvError::Corrupt`].
//!
//! A refusal must also *apply nothing*: corruption can be discovered
//! after a closure has already issued mutations (a reclamation step
//! decodes records between its deletes), and committing that prefix
//! would trade a detected corruption for a silent invariant breach. The
//! write view therefore buffers every mutation and flushes to the
//! underlying transaction only when the closure completes `Ok` — a
//! closure that refuses leaves the store byte-for-byte as it found it.
//! Reads through the view see the buffer first (read-your-writes), and
//! the cursor merges buffered entries with the store's in key order, so
//! a closure observes exactly the semantics of writing directly.
//!
//! The buffering trades one in-memory copy of a transaction's write set
//! for the refusal guarantee. Backend transactions are deliberately
//! bounded (budgeted release flushes, budgeted reclamation, one node
//! install), so the copy is small change against the store I/O the same
//! transaction performs.

use std::collections::BTreeMap;
use std::ops::Bound;

use super::error::KvError;
use super::kv::{Kv, ReadTxn, Table, WriteTxn};

/// Runs one read-only transaction whose view reports through
/// [`KvError`]: the store's failures as [`KvError::Store`], the
/// closure's own refusals as [`KvError::Corrupt`].
pub(crate) async fn read<K, R, F>(kv: &K, mut f: F) -> Result<R, KvError<K::Error>>
where
    K: Kv,
    R: Send + 'static,
    F: FnMut(&mut CheckedRead<'_, K::Error>) -> Result<R, KvError<K::Error>> + Send + 'static,
{
    kv.read(move |txn| {
        // A read transaction applies nothing, so a refusal needs no
        // undo: it rides the success channel as data and is re-split
        // below.
        match f(&mut CheckedRead { inner: txn }) {
            Ok(value) => Ok(Ok(value)),
            Err(KvError::Store(error)) => Err(error),
            Err(KvError::Corrupt(corruption)) => Ok(Err(corruption)),
        }
    })
    .await
    .map_err(KvError::Store)?
    .map_err(KvError::from)
}

/// Runs one write transaction whose view reports through [`KvError`]
/// and whose refusals apply nothing (see the [module docs](self)).
///
/// The closure's `Ok` flushes the buffered write set into the
/// underlying transaction and commits; [`KvError::Store`] aborts the
/// transaction through the store's own mechanism; [`KvError::Corrupt`]
/// flushes nothing, so the transaction commits empty and the store is
/// untouched.
pub(crate) async fn write<K, R, F>(kv: &K, mut f: F) -> Result<R, KvError<K::Error>>
where
    K: Kv,
    R: Send + 'static,
    F: FnMut(&mut CheckedWrite<'_, K::Error>) -> Result<R, KvError<K::Error>> + Send + 'static,
{
    kv.write(move |txn| {
        let mut checked = CheckedWrite {
            inner: txn,
            buffer: BTreeMap::new(),
        };
        match f(&mut checked) {
            Ok(value) => {
                checked.flush()?;
                Ok(Ok(value))
            }
            Err(KvError::Store(error)) => Err(error),
            Err(KvError::Corrupt(corruption)) => Ok(Err(corruption)),
        }
    })
    .await
    .map_err(KvError::Store)?
    .map_err(KvError::from)
}

/// A read view whose error channel is [`KvError<E>`](KvError): pure
/// error widening, no buffering (reads have nothing to undo).
pub(crate) struct CheckedRead<'a, E> {
    inner: &'a mut (dyn ReadTxn<Error = E> + 'a),
}

impl<E> ReadTxn for CheckedRead<'_, E> {
    type Error = KvError<E>;

    fn get(&mut self, table: Table, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        self.inner.get(table, key).map_err(KvError::Store)
    }

    fn next_after(
        &mut self,
        table: Table,
        after: Option<&[u8]>,
    ) -> Result<Option<(Vec<u8>, Vec<u8>)>, Self::Error> {
        self.inner.next_after(table, after).map_err(KvError::Store)
    }
}

/// A write view whose error channel is [`KvError<E>`](KvError) and
/// whose mutations buffer until [`flush`](Self::flush): the mechanism
/// behind [`write()`]'s refusals-apply-nothing guarantee.
pub(crate) struct CheckedWrite<'a, E> {
    inner: &'a mut (dyn WriteTxn<Error = E> + 'a),
    /// The pending write set: `Some` a put, `None` a delete tombstone.
    /// Reads consult this before the store, so the view is
    /// read-your-writes exactly as the raw transaction is.
    buffer: BTreeMap<Table, BTreeMap<Vec<u8>, Option<Vec<u8>>>>,
}

impl<E> CheckedWrite<'_, E> {
    /// Applies the buffered write set to the underlying transaction, in
    /// key order. Only [`write()`] calls this, and only on closure `Ok`.
    fn flush(&mut self) -> Result<(), E> {
        for (table, entries) in std::mem::take(&mut self.buffer) {
            for (key, slot) in entries {
                match slot {
                    Some(value) => self.inner.put(table, &key, &value)?,
                    None => self.inner.delete(table, &key)?,
                }
            }
        }
        Ok(())
    }
}

impl<E> ReadTxn for CheckedWrite<'_, E> {
    type Error = KvError<E>;

    fn get(&mut self, table: Table, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        if let Some(slot) = self.buffer.get(&table).and_then(|entries| entries.get(key)) {
            return Ok(slot.clone());
        }
        self.inner.get(table, key).map_err(KvError::Store)
    }

    fn next_after(
        &mut self,
        table: Table,
        after: Option<&[u8]>,
    ) -> Result<Option<(Vec<u8>, Vec<u8>)>, Self::Error> {
        // Merge the store's cursor with the buffered write set: the
        // smaller next key wins, a buffered entry shadows the store's at
        // the same key, and a tombstone advances the cursor past the
        // key it deletes.
        let mut cursor: Option<Vec<u8>> = after.map(<[u8]>::to_vec);
        loop {
            let stored = self
                .inner
                .next_after(table, cursor.as_deref())
                .map_err(KvError::Store)?;
            let buffered = self.buffer.get(&table).and_then(|entries| {
                let from = match cursor.as_deref() {
                    None => entries.range::<[u8], _>(..),
                    Some(cursor) => {
                        entries.range::<[u8], _>((Bound::Excluded(cursor), Bound::Unbounded))
                    }
                };
                from.map(|(key, slot)| (key.clone(), slot.clone())).next()
            });
            match (stored, buffered) {
                (None, None) => return Ok(None),
                // No buffered entry past the cursor, so nothing shadows
                // the store's next key.
                (Some(found), None) => return Ok(Some(found)),
                (stored, Some((key, slot))) => {
                    if let Some((stored_key, stored_value)) = stored
                        && stored_key < key
                    {
                        return Ok(Some((stored_key, stored_value)));
                    }
                    match slot {
                        Some(value) => return Ok(Some((key, value))),
                        // A tombstone: skip the deleted key (shadowing
                        // the store's entry when the keys coincide) and
                        // look again past it.
                        None => cursor = Some(key),
                    }
                }
            }
        }
    }
}

impl<E> WriteTxn for CheckedWrite<'_, E> {
    fn put(&mut self, table: Table, key: &[u8], value: &[u8]) -> Result<(), Self::Error> {
        self.buffer
            .entry(table)
            .or_default()
            .insert(key.to_vec(), Some(value.to_vec()));
        Ok(())
    }

    fn delete(&mut self, table: Table, key: &[u8]) -> Result<(), Self::Error> {
        self.buffer
            .entry(table)
            .or_default()
            .insert(key.to_vec(), None);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
