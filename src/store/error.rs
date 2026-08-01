//! The persistent backend's failure taxonomy: a store that *failed*
//! versus a store that *lied*.
//!
//! Every fallible operation of the persistent backend reports through
//! [`KvError`], which keeps the two failure genres apart because a
//! deployment handles them oppositely:
//!
//! - [`KvError::Store`] wraps the [`Kv`](super::Kv) implementation's own
//!   error: the store could not serve the operation. Whatever the store's
//!   docs say about the failure (transient contention, a full disk, a
//!   closed handle) governs; retrying against healthy storage is
//!   sensible, and nothing about the stored replica is in doubt.
//! - [`KvError::Corrupt`] reports that the store's contents are not
//!   what this crate wrote: a record failed to decode, a row key has an
//!   impossible shape, a loaded value violates an invariant every write
//!   upholds, or a row this crate wrote and never deleted is absent.
//!   Retrying cannot help — the same bytes will come back — and the
//!   replica's stored state is no longer trustworthy. The
//!   [`Corruption`] payload names the table and key so the deployment
//!   can investigate; recovering the *content* needs no backup ritual,
//!   because any other replica of the set can restore it through
//!   ordinary gossip (see the crate docs' storage section).

use super::kv::Table;

/// Evidence that a store's contents diverge from what the persistent
/// backend wrote.
///
/// Corruption is environmental — bit rot, a torn write the store's own
/// integrity layer missed, an operator's misdirected script — so the
/// backend treats it as an observable, handleable condition: the
/// operation that read the bytes refuses (applying nothing), and this
/// value rides the error path to the caller. What to do about it is the
/// deployment's decision, not this crate's.
///
/// Two corruptions compare equal when they name the same row and
/// diagnosis, which is what lets tests pin the exact refusal a
/// constructed bad row produces.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("corrupt {what} at {table}[{key}]: the store's contents are not what this crate wrote", table = .table.0, key = hex(.key))]
pub struct Corruption {
    table: Table,
    key: Box<[u8]>,
    what: &'static str,
}

/// The key rendered as lowercase hex, for the `Display` message.
fn hex(key: &[u8]) -> String {
    key.iter().map(|byte| format!("{byte:02x}")).collect()
}

impl Corruption {
    /// Records that the row at `key` in `table` failed to load as, or
    /// cannot lawfully hold, a `what` (a short noun phrase naming the
    /// record kind or the violated shape).
    pub(crate) fn new(table: Table, key: &[u8], what: &'static str) -> Self {
        Self {
            table,
            key: key.into(),
            what,
        }
    }

    /// The table holding the corrupt row.
    pub fn table(&self) -> Table {
        self.table
    }

    /// The corrupt row's key (for a malformed key, the key bytes
    /// themselves).
    pub fn key(&self) -> &[u8] {
        &self.key
    }
}

/// A persistent-backend failure: the store failed, or the store lied.
///
/// This is the error type of every [`KvBackend`](super::KvBackend)
/// operation, so it is the `E` that public error enums carry for a
/// persistent peer (for example
/// [`Error::Storage`](crate::error::Error::Storage) wraps it). The
/// two arms differ in handling:
/// [`Store`](Self::Store) is the store's own failure and worth
/// retrying per the store's docs, while [`Corrupt`](Self::Corrupt) is
/// evidence the stored bytes changed underneath the backend and will
/// not decode any better on retry.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KvError<E> {
    /// The [`Kv`](super::Kv) store failed to serve an operation; the
    /// wrapped error is the store's own.
    #[error(transparent)]
    Store(E),

    /// The store's contents diverge from what the backend wrote; the
    /// payload names the row.
    #[error(transparent)]
    Corrupt(#[from] Corruption),
}
