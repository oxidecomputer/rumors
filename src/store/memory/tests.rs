//! The reference store's own behavior: history, faults, and the
//! transaction mechanics the conformance suite assumes it can trust.

use super::*;

const TABLE: Table = Table("test");
const OTHER: Table = Table("other");

/// A committed write is visible to later reads; an absent key reads as
/// `None`.
#[pollster::test]
async fn committed_writes_are_visible() {
    let store = Memory::new();
    store.write(|txn| txn.put(TABLE, b"k", b"v")).await.unwrap();
    let value = store.read(|txn| txn.get(TABLE, b"k")).await.unwrap();
    assert_eq!(value.as_deref(), Some(b"v".as_slice()));
    let absent = store.read(|txn| txn.get(TABLE, b"absent")).await.unwrap();
    assert_eq!(absent, None);
}

/// A closure `Err` aborts the transaction: nothing it put is visible
/// afterwards.
#[pollster::test]
async fn closure_error_applies_nothing() {
    let store = Memory::new();
    let result: Result<(), _> = store
        .write(|txn| {
            txn.put(TABLE, b"k", b"v")?;
            Err(MemoryError::Aborted)
        })
        .await;
    assert_eq!(result, Err(MemoryError::Aborted));
    let value = store.read(|txn| txn.get(TABLE, b"k")).await.unwrap();
    assert_eq!(value, None);
}

/// Tables are namespaces: the same key in two tables holds two values,
/// and a cursor never crosses from one table into another.
#[pollster::test]
async fn tables_are_disjoint() {
    let store = Memory::new();
    store
        .write(|txn| {
            txn.put(TABLE, b"k", b"a")?;
            txn.put(OTHER, b"k", b"b")
        })
        .await
        .unwrap();
    let (a, b) = store
        .read(|txn| Ok((txn.get(TABLE, b"k")?, txn.get(OTHER, b"k")?)))
        .await
        .unwrap();
    assert_eq!(a.as_deref(), Some(b"a".as_slice()));
    assert_eq!(b.as_deref(), Some(b"b".as_slice()));
    let walked = store
        .read(|txn| {
            let mut keys = Vec::new();
            let mut cursor = None;
            while let Some((key, _)) = txn.next_after(TABLE, cursor.as_deref())? {
                keys.push(key.clone());
                cursor = Some(key);
            }
            Ok(keys)
        })
        .await
        .unwrap();
    assert_eq!(walked, vec![b"k".to_vec()]);
}

/// The empty key participates in cursor walks: `next_after(None)` starts
/// from the very beginning, including a zero-length key.
#[pollster::test]
async fn cursor_reaches_the_empty_key() {
    let store = Memory::new();
    store
        .write(|txn| {
            txn.put(TABLE, b"", b"empty")?;
            txn.put(TABLE, b"k", b"v")
        })
        .await
        .unwrap();
    let first = store.read(|txn| txn.next_after(TABLE, None)).await.unwrap();
    assert_eq!(first, Some((Vec::new(), b"empty".to_vec())));
}

/// The history records the empty store plus one state per committed
/// write — aborted transactions leave no entry — and `reopen_at`
/// resurrects exactly the chosen prefix.
#[pollster::test]
async fn history_records_every_committed_prefix() {
    let store = Memory::recording();
    store.write(|txn| txn.put(TABLE, b"a", b"1")).await.unwrap();
    let aborted: Result<(), _> = store.write(|_| Err(MemoryError::Aborted)).await;
    assert!(aborted.is_err());
    store.write(|txn| txn.put(TABLE, b"b", b"2")).await.unwrap();

    assert_eq!(store.history_len(), 3);
    let at_empty = store.reopen_at(0);
    assert_eq!(
        at_empty.read(|txn| txn.get(TABLE, b"a")).await.unwrap(),
        None
    );
    let at_first = store.reopen_at(1);
    assert_eq!(
        at_first
            .read(|txn| txn.get(TABLE, b"a"))
            .await
            .unwrap()
            .as_deref(),
        Some(b"1".as_slice())
    );
    assert_eq!(
        at_first.read(|txn| txn.get(TABLE, b"b")).await.unwrap(),
        None
    );
}

/// An injected abort fails the scheduled write with nothing applied; the
/// store works normally afterwards.
#[pollster::test]
async fn injected_abort_applies_nothing() {
    let store = Memory::new();
    store.inject_abort(0);
    let result: Result<(), _> = store.write(|txn| txn.put(TABLE, b"k", b"v")).await;
    assert_eq!(result, Err(MemoryError::Injected));
    assert_eq!(store.read(|txn| txn.get(TABLE, b"k")).await.unwrap(), None);
    store.write(|txn| txn.put(TABLE, b"k", b"v")).await.unwrap();
}

/// An injected commit-then-error fault reports failure for a write that
/// nevertheless committed in full: the caller's `Err` does not certify
/// that nothing happened.
#[pollster::test]
async fn injected_commit_then_error_commits_anyway() {
    let store = Memory::new();
    store.inject_commit_then_error(0);
    let result: Result<(), _> = store.write(|txn| txn.put(TABLE, b"k", b"v")).await;
    assert_eq!(result, Err(MemoryError::Injected));
    assert_eq!(
        store
            .read(|txn| txn.get(TABLE, b"k"))
            .await
            .unwrap()
            .as_deref(),
        Some(b"v".as_slice())
    );
}

/// The durability ledger: local commits advance nothing, a completed
/// sync counts itself and covers exactly the committed history at that
/// moment, and `reopen_at(synced_prefix())` resurrects that state.
#[pollster::test]
async fn sync_ledger_marks_the_covered_prefix() {
    let store = Memory::recording();
    store.write(|txn| txn.put(TABLE, b"a", b"1")).await.unwrap();
    assert_eq!(store.sync_count(), 0, "commits alone never sync");
    assert_eq!(store.synced_prefix(), 0);

    store.sync().await.unwrap();
    assert_eq!(store.sync_count(), 1);
    assert_eq!(store.synced_prefix(), 1, "the sync covers the commit");

    store.write(|txn| txn.put(TABLE, b"b", b"2")).await.unwrap();
    let crashed = store.reopen_at(store.synced_prefix());
    assert_eq!(
        crashed
            .read(|txn| txn.get(TABLE, b"a"))
            .await
            .unwrap()
            .as_deref(),
        Some(b"1".as_slice()),
        "the covered commit survives the worst legal crash"
    );
    assert_eq!(
        crashed.read(|txn| txn.get(TABLE, b"b")).await.unwrap(),
        None,
        "the uncovered commit is the one a write-behind policy may lose"
    );
}

/// An injected sync failure fails the barrier without counting or
/// covering anything, one-shot: the next sync succeeds.
#[pollster::test]
async fn injected_sync_error_is_one_shot() {
    let store = Memory::recording();
    store.write(|txn| txn.put(TABLE, b"a", b"1")).await.unwrap();
    store.inject_sync_error();
    assert_eq!(store.sync().await, Err(MemoryError::Injected));
    assert_eq!(store.sync_count(), 0, "a failed sync counts nothing");
    assert_eq!(store.synced_prefix(), 0, "a failed sync covers nothing");
    store.sync().await.unwrap();
    assert_eq!(store.sync_count(), 1);
}

/// The re-execution schedule runs every closure twice with the first
/// execution's effects discarded.
///
/// The double run is observable through a deliberately leaked side
/// effect while the committed state reflects exactly one application:
/// the adequacy witness that a closure leaking effects outside its
/// transaction argument diverges under a retrying store.
#[pollster::test]
async fn retrying_reruns_closures_and_discards_the_first() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let store = Memory::new().retrying();
    let executions = Arc::new(AtomicUsize::new(0));

    let writes = executions.clone();
    store
        .write(move |txn| {
            // The leak: an effect outside the transaction argument.
            writes.fetch_add(1, Ordering::Relaxed);
            txn.put(TABLE, b"k", b"v")
        })
        .await
        .unwrap();
    assert_eq!(
        executions.load(Ordering::Relaxed),
        2,
        "the write closure ran on the re-execution schedule"
    );
    assert_eq!(
        store
            .read(|txn| txn.get(TABLE, b"k"))
            .await
            .unwrap()
            .as_deref(),
        Some(b"v".as_slice()),
        "the committed state reflects one application"
    );

    let reads = executions.clone();
    executions.store(0, Ordering::Relaxed);
    store
        .read(move |txn| {
            reads.fetch_add(1, Ordering::Relaxed);
            txn.get(TABLE, b"k")
        })
        .await
        .unwrap();
    assert_eq!(
        executions.load(Ordering::Relaxed),
        2,
        "the read closure ran on the re-execution schedule"
    );

    // The control: a plain store runs each closure once, so the counter
    // above is measuring the schedule, not some ambient double-call.
    let plain = Memory::new();
    let control = executions.clone();
    executions.store(0, Ordering::Relaxed);
    plain
        .write(move |txn| {
            control.fetch_add(1, Ordering::Relaxed);
            txn.put(TABLE, b"k", b"v")
        })
        .await
        .unwrap();
    assert_eq!(executions.load(Ordering::Relaxed), 1);
}

/// A closure error on the schedule's first execution aborts with
/// nothing applied and no second run, exactly as a conflict-retrying
/// store aborts on a closure error.
#[pollster::test]
async fn retrying_closure_error_aborts_once() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let store = Memory::new().retrying();
    let executions = Arc::new(AtomicUsize::new(0));
    let counted = executions.clone();
    let result: Result<(), _> = store
        .write(move |txn| {
            counted.fetch_add(1, Ordering::Relaxed);
            txn.put(TABLE, b"k", b"v")?;
            Err(MemoryError::Aborted)
        })
        .await;
    assert_eq!(result, Err(MemoryError::Aborted));
    assert_eq!(
        executions.load(Ordering::Relaxed),
        1,
        "a failing closure is not re-executed"
    );
    assert_eq!(store.read(|txn| txn.get(TABLE, b"k")).await.unwrap(), None);
}
