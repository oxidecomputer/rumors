//! Conformance checks for caller-built [`Kv`] stores.
//!
//! The persistence layer's crash-consistency argument rests on the
//! [storage contract](crate::store::kv): atomic serializable write
//! transactions, snapshot reads, ordered cursors, and all-or-nothing
//! visibility for interrupted commits. This crate validates its own
//! reference store with these checks; a deployment that wraps its own
//! store should validate it the same way.
//!
//! # Using the suite
//!
//! Provide a factory that builds a *fresh, empty* store per call, plus a
//! way to construct a sample error (the abort probe needs one the suite
//! cannot mint for an arbitrary error type), then run [`check`]:
//!
//! ```
//! # tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(async {
//! rumors::conformance::kv::check(
//!     async || rumors::Memory::new(),
//!     || rumors::store::MemoryError::Aborted,
//! )
//! .await;
//! # });
//! ```
//!
//! Every check panics with the violated clause on failure, so the suite
//! drops into any test harness. Checks run on the caller's executor; a
//! store whose transactions genuinely suspend should run under its
//! runtime, with a timeout around the whole suite.
//!
//! # What the suite cannot see
//!
//! A black box bounds what any probe can establish; a pass leaves each of
//! these the implementation's own documented obligation:
//!
//! - **Prefix consistency after a crash.** No probe can crash the host.
//!   That the surviving state is always the state after some prefix of
//!   the committed transaction sequence — the clause the persistent
//!   backend's recovery is a theorem over — is entirely the store's
//!   obligation (its journal, WAL, or shadow paging).
//! - **Durability policy.** Whether an acknowledged commit survives a
//!   crash, and what [`sync`](Kv::sync) flushes, is unprobeable from
//!   inside a healthy process; the store documents it.
//! - **Interruption mid-commit.** The dropped-write probe abandons
//!   transaction futures at every poll boundary it can reach, but a
//!   store that offloads its commit may have states no in-process poll
//!   schedule exposes; "committed-or-not, never partial" beyond the
//!   probed schedules is the implementation's obligation.
//! - **Serializability under contention shapes** the concurrent probe
//!   does not produce (it drives one hot key from several tasks; a
//!   store that misbehaves only under disjoint-key write skew passes
//!   anyway).

use std::pin::pin;
use std::task::{Context, Poll};

use futures::future::join_all;

use crate::store::kv::{Kv, Table};

/// Tables the probes use; a conforming store treats every table alike, so
/// two are enough to check namespacing.
const TABLE_A: Table = Table("rumors-conformance:a");
const TABLE_B: Table = Table("rumors-conformance:b");

/// Concurrent writers in the serialization probe.
const WRITERS: usize = 8;

/// Increments each writer applies in the serialization probe.
const INCREMENTS: u64 = 16;

/// Keys the dropped-write probe writes per transaction; all or none must
/// survive an abandonment.
const DROP_KEYS: u64 = 8;

/// Runs every check against fresh stores from `factory`.
///
/// `error` constructs a sample error for the abort probe; the suite never
/// inspects it, only returns it from a closure and requires the
/// transaction to have applied nothing.
///
/// # Panics
///
/// On the first violated contract clause, naming it.
pub async fn check<K, E, F>(mut factory: F, error: E)
where
    K: Kv,
    E: Fn() -> K::Error + Send + 'static,
    F: AsyncFnMut() -> K,
{
    visibility(factory().await).await;
    read_your_writes(factory().await).await;
    abort_applies_nothing(factory().await, error).await;
    cursor_walks_in_order(factory().await).await;
    writes_serialize(factory().await).await;
    dropped_writes_are_all_or_nothing(factory().await).await;
    sync_resolves(factory().await).await;
}

/// Committed values are visible to later reads, absent keys read `None`,
/// deletes remove, and tables are disjoint namespaces.
async fn visibility<K: Kv>(store: K) {
    store
        .write(|txn| {
            txn.put(TABLE_A, b"k", b"a")?;
            txn.put(TABLE_B, b"k", b"b")
        })
        .await
        .expect("a healthy store must commit a plain write");
    let (a, b, absent) = store
        .read(|txn| {
            Ok((
                txn.get(TABLE_A, b"k")?,
                txn.get(TABLE_B, b"k")?,
                txn.get(TABLE_A, b"absent")?,
            ))
        })
        .await
        .expect("a healthy store must serve a plain read");
    assert_eq!(
        a.as_deref(),
        Some(b"a".as_slice()),
        "clause: a committed put is visible to a later read"
    );
    assert_eq!(
        b.as_deref(),
        Some(b"b".as_slice()),
        "clause: tables are disjoint namespaces (the other table's value leaked)"
    );
    assert_eq!(absent, None, "clause: an absent key reads as None");

    store
        .write(|txn| txn.delete(TABLE_A, b"k"))
        .await
        .expect("a healthy store must commit a delete");
    let gone = store
        .read(|txn| txn.get(TABLE_A, b"k"))
        .await
        .expect("a healthy store must serve a plain read");
    assert_eq!(gone, None, "clause: a committed delete removes the key");
}

/// A transaction's own mutations are visible to its later reads and
/// cursor calls, before any commit.
async fn read_your_writes<K: Kv>(store: K) {
    store
        .write(|txn| {
            txn.put(TABLE_A, b"k", b"v")?;
            let own = txn.get(TABLE_A, b"k")?;
            assert_eq!(
                own.as_deref(),
                Some(b"v".as_slice()),
                "clause: a transaction reads its own puts"
            );
            let walked = txn.next_after(TABLE_A, None)?;
            assert_eq!(
                walked.map(|(key, _)| key),
                Some(b"k".to_vec()),
                "clause: a transaction's cursor sees its own puts"
            );
            txn.delete(TABLE_A, b"k")?;
            assert_eq!(
                txn.get(TABLE_A, b"k")?,
                None,
                "clause: a transaction reads its own deletes"
            );
            Ok(())
        })
        .await
        .expect("a healthy store must commit the read-your-writes probe");
}

/// A closure `Err` aborts with nothing applied.
async fn abort_applies_nothing<K: Kv>(store: K, error: impl Fn() -> K::Error + Send + 'static) {
    let aborted: Result<(), _> = store
        .write(move |txn| {
            txn.put(TABLE_A, b"torn-1", b"v")?;
            txn.put(TABLE_A, b"torn-2", b"v")?;
            Err(error())
        })
        .await;
    assert!(
        aborted.is_err(),
        "clause: a closure Err surfaces as the write's Err"
    );
    let survivors = store
        .read(|txn| Ok((txn.get(TABLE_A, b"torn-1")?, txn.get(TABLE_A, b"torn-2")?)))
        .await
        .expect("a healthy store must serve a plain read");
    assert_eq!(
        survivors,
        (None, None),
        "clause: an aborted transaction applies nothing (partial effects leaked)"
    );
}

/// The cursor visits every key exactly once, in ascending byte order,
/// terminates, includes the empty key, and never crosses tables.
async fn cursor_walks_in_order<K: Kv>(store: K) {
    // Deliberately inserted out of order, with the empty key and a
    // prefix-nested pair.
    let keys: &[&[u8]] = &[b"b", b"", b"aa", b"a", b"ab", b"z"];
    store
        .write(move |txn| {
            for key in keys {
                txn.put(TABLE_A, key, b"v")?;
            }
            txn.put(TABLE_B, b"intruder", b"v")
        })
        .await
        .expect("a healthy store must commit the cursor fixture");
    let walked = store
        .read(|txn| {
            let mut walked = Vec::new();
            let mut cursor = None;
            while let Some((key, _)) = txn.next_after(TABLE_A, cursor.as_deref())? {
                walked.push(key.clone());
                cursor = Some(key);
            }
            Ok(walked)
        })
        .await
        .expect("a healthy store must serve the cursor walk");
    let mut expected: Vec<Vec<u8>> = keys.iter().map(|key| key.to_vec()).collect();
    expected.sort();
    assert_eq!(
        walked, expected,
        "clause: the cursor visits every key of the table exactly once, in ascending byte order, and no other table's"
    );
}

/// Concurrent read-modify-write transactions serialize: no increment is
/// lost.
async fn writes_serialize<K: Kv>(store: K) {
    let writers = (0..WRITERS).map(|_| {
        let store = store.clone();
        async move {
            for _ in 0..INCREMENTS {
                store
                    .write(|txn| {
                        let current = txn
                            .get(TABLE_A, b"counter")?
                            .map(|value| {
                                u64::from_be_bytes(
                                    value.as_slice().try_into().expect("counter width"),
                                )
                            })
                            .unwrap_or(0);
                        txn.put(TABLE_A, b"counter", &(current + 1).to_be_bytes())
                    })
                    .await
                    .expect("a healthy store must commit an increment");
            }
        }
    });
    join_all(writers).await;
    let total = store
        .read(|txn| txn.get(TABLE_A, b"counter"))
        .await
        .expect("a healthy store must serve a plain read")
        .map(|value| u64::from_be_bytes(value.as_slice().try_into().expect("counter width")))
        .unwrap_or(0);
    assert_eq!(
        total,
        WRITERS as u64 * INCREMENTS,
        "clause: concurrent write transactions serialize (increments were lost)"
    );
}

/// Poll boundaries the dropped-write probe explores before concluding
/// the store's fixture write never resolves (a liveness failure in its
/// own right).
const DROP_PATIENCE: usize = 64;

/// A write future abandoned at any poll boundary commits everything or
/// nothing — never a prefix of its puts.
async fn dropped_writes_are_all_or_nothing<K: Kv>(store: K) {
    let mut completed = false;
    for polls in 0..DROP_PATIENCE {
        let round = polls; // keys per round, so rounds never alias
        // The future lives only inside this block: `pin!` pins into a
        // hidden local, so the block's end — not a `drop` of the pinned
        // reference — is what genuinely abandons it before the check.
        {
            let owner = store.clone();
            let mut future = pin!(async move {
                owner
                    .write(move |txn| {
                        for key in 0..DROP_KEYS {
                            txn.put(TABLE_A, &[round as u8, key as u8], b"v")?;
                        }
                        Ok(())
                    })
                    .await
            });
            let waker = std::task::Waker::noop();
            let mut context = Context::from_waker(waker);
            for _ in 0..polls {
                if let Poll::Ready(result) = future.as_mut().poll(&mut context) {
                    result.expect("a healthy store must commit the dropped-write fixture");
                    completed = true;
                    break;
                }
            }
        }

        let present = store
            .read(move |txn| {
                let mut present = 0u64;
                for key in 0..DROP_KEYS {
                    if txn.get(TABLE_A, &[round as u8, key as u8])?.is_some() {
                        present += 1;
                    }
                }
                Ok(present)
            })
            .await
            .expect("a healthy store must serve a plain read");
        assert!(
            present == 0 || present == DROP_KEYS,
            "clause: an abandoned write commits all or nothing (a partial prefix of its puts survived)"
        );
        if completed {
            return;
        }
    }
    panic!(
        "clause: a plain write must resolve (the fixture never completed within {DROP_PATIENCE} polls)"
    );
}

/// The durability barrier resolves on a healthy store.
async fn sync_resolves<K: Kv>(store: K) {
    store
        .sync()
        .await
        .expect("clause: sync resolves Ok on a healthy store");
}

#[cfg(test)]
mod tests;
