//! The checked write view is a faithful transaction (read-your-writes,
//! ordered cursor merge) whose corruption refusals apply nothing.

use proptest::prelude::*;

use super::*;
use crate::store::error::Corruption;
use crate::store::{Memory, MemoryError};

/// The probe table every test here writes.
const TABLE: Table = Table("rumors-checked:probe");

/// A second table, proving the buffer keys per table.
const OTHER: Table = Table("rumors-checked:other");

/// A refusing closure leaves the store byte-for-byte untouched, no
/// matter how many mutations it issued first: the refusals-apply-nothing
/// guarantee the corruption policy rests on.
#[pollster::test]
async fn refusal_applies_nothing() {
    let store = Memory::recording();
    store
        .write(|txn| txn.put(TABLE, b"kept", b"before"))
        .await
        .expect("seed");

    let refused: Result<(), KvError<MemoryError>> = write(&store, |txn| {
        txn.put(TABLE, b"kept", b"clobbered")?;
        txn.put(TABLE, b"fresh", b"value")?;
        txn.delete(TABLE, b"kept")?;
        Err(Corruption::new(TABLE, b"kept", "probe").into())
    })
    .await;
    assert!(matches!(refused, Err(KvError::Corrupt(_))));

    let (kept, fresh) = store
        .read(|txn| Ok((txn.get(TABLE, b"kept")?, txn.get(TABLE, b"fresh")?)))
        .await
        .expect("read back");
    assert_eq!(kept.as_deref(), Some(b"before".as_slice()));
    assert_eq!(fresh, None, "no buffered mutation leaked");
}

/// A store failure inside the closure aborts through the store's own
/// mechanism and surfaces as [`KvError::Store`]; a completing closure
/// flushes its whole write set.
#[pollster::test]
async fn store_errors_abort_and_success_flushes() {
    let store = Memory::new();
    store.inject_abort(0);
    let aborted: Result<(), KvError<MemoryError>> =
        write(&store, |txn| txn.put(TABLE, b"k", b"v")).await;
    assert!(matches!(
        aborted,
        Err(KvError::Store(MemoryError::Injected))
    ));

    write(&store, |txn| {
        txn.put(TABLE, b"k", b"v")?;
        txn.put(OTHER, b"k", b"w")?;
        txn.delete(TABLE, b"absent")?;
        Ok(())
    })
    .await
    .expect("the fault window is over");
    let (a, b) = store
        .read(|txn| Ok((txn.get(TABLE, b"k")?, txn.get(OTHER, b"k")?)))
        .await
        .expect("read back");
    assert_eq!(a.as_deref(), Some(b"v".as_slice()));
    assert_eq!(b.as_deref(), Some(b"w".as_slice()));
}

/// One scripted mutation against the checked view's probe state.
#[derive(Debug, Clone)]
enum Step {
    Put(u8, u8),
    Delete(u8),
}

fn steps() -> impl Strategy<Value = Vec<Step>> {
    proptest::collection::vec(
        prop_oneof![
            3 => (any::<u8>(), any::<u8>()).prop_map(|(k, v)| Step::Put(k, v)),
            1 => any::<u8>().prop_map(Step::Delete),
        ],
        0..24,
    )
}

proptest! {
    /// The checked write view is observationally identical to writing
    /// directly.
    ///
    /// Over any base state and mutation script, every `get` and every
    /// `next_after` walk mid-transaction agrees with a raw transaction
    /// running the same script, and the committed state afterwards is
    /// identical too. This is the read-your-writes and cursor-merge
    /// contract the custody closures rely on.
    #[test]
    fn checked_view_matches_direct_writes(
        base in proptest::collection::btree_map(any::<u8>(), any::<u8>(), 0..16),
        script in steps(),
    ) {
        pollster::block_on(async move {
            // Two stores from the same base state: one written through
            // the checked view, one directly.
            let checked_store = Memory::new();
            let direct_store = Memory::new();
            for store in [&checked_store, &direct_store] {
                let base = base.clone();
                store
                    .write(move |txn| {
                        for (key, value) in &base {
                            txn.put(TABLE, &[*key], &[*value])?;
                        }
                        Ok(())
                    })
                    .await
                    .expect("seed");
            }

            // Run the script through the checked view, snapshotting the
            // mid-transaction observations it can make.
            let observed = {
                let script = script.clone();
                write(&checked_store, move |txn| {
                    for step in &script {
                        match step {
                            Step::Put(key, value) => txn.put(TABLE, &[*key], &[*value])?,
                            Step::Delete(key) => txn.delete(TABLE, &[*key])?,
                        }
                    }
                    observe(txn)
                })
                .await
                .expect("the checked script commits")
            };
            // The oracle: the same script direct, same observations.
            let expected = direct_store
                .write(move |txn| {
                    for step in &script {
                        match step {
                            Step::Put(key, value) => txn.put(TABLE, &[*key], &[*value])?,
                            Step::Delete(key) => txn.delete(TABLE, &[*key])?,
                        }
                    }
                    observe(txn)
                })
                .await
                .expect("the direct script commits");
            prop_assert_eq!(&observed, &expected);

            // The committed states agree entry for entry.
            let walk = |store: &Memory| {
                let store = store.clone();
                async move {
                    store
                        .read(|txn| {
                            let mut entries = Vec::new();
                            let mut cursor = None;
                            while let Some((key, value)) = txn.next_after(TABLE, cursor.as_deref())? {
                                entries.push((key.clone(), value));
                                cursor = Some(key);
                            }
                            Ok(entries)
                        })
                        .await
                        .expect("committed walk")
                }
            };
            prop_assert_eq!(walk(&checked_store).await, walk(&direct_store).await);
            Ok(())
        })?;
    }
}

/// Every observation a transaction view can make: all point reads plus
/// the full cursor walk, generic so the checked view and the raw
/// transaction produce comparable values.
// The inline tuple is this probe's whole vocabulary; a minted alias
// would carry no meaning of its own.
#[allow(clippy::type_complexity)]
fn observe<T: ReadTxn + ?Sized>(
    txn: &mut T,
) -> Result<(Vec<Option<Vec<u8>>>, Vec<(Vec<u8>, Vec<u8>)>), T::Error> {
    let mut gets = Vec::new();
    for key in 0..=u8::MAX {
        gets.push(txn.get(TABLE, &[key])?);
    }
    let mut walked = Vec::new();
    let mut cursor = None;
    while let Some((key, value)) = txn.next_after(TABLE, cursor.as_deref())? {
        walked.push((key.clone(), value));
        cursor = Some(key);
    }
    Ok((gets, walked))
}
