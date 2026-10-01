//! Deterministic liveness checks for the coverage-guided targets.

use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use crate::input::Target;
use crate::under_heap_cap;

/// Serializes tests that reset the process-wide peak measurement.
static HEAP_METER: Mutex<()> = Mutex::new(());

/// Acquire the heap meter even when the deliberate panic test poisoned it.
fn heap_meter() -> MutexGuard<'static, ()> {
    HEAP_METER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Locate the committed seed corpus in the detached fuzz workspace.
fn seeds_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("seeds")
}

/// Every committed seed executes the exact target body that libFuzzer calls.
///
/// This makes framing and oracle regressions gate failures; the slower fuzz
/// sweep remains responsible only for discovering new inputs.
#[test]
fn committed_seeds_execute_their_target_oracles() {
    let _meter = heap_meter();
    for target in Target::ALL {
        let directory = seeds_root().join(target.name());
        let mut paths = fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("cannot list {}: {error}", directory.display()))
            .map(|entry| entry.expect("cannot read a seed-directory entry").path())
            .collect::<Vec<_>>();
        paths.sort();
        assert!(
            !paths.is_empty(),
            "{} has no committed seeds",
            target.name()
        );
        for path in paths {
            let bytes = fs::read(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            under_heap_cap(|| target.run(&bytes));
        }
    }
}

/// The allocation guard rejects a survivable spike above its configured cap.
#[test]
#[should_panic(expected = "transient heap grew")]
fn heap_cap_detects_excess_growth() {
    let _meter = heap_meter();
    super::under_heap_cap_at(64 * 1024, || {
        let bytes = vec![0_u8; 128 * 1024];
        std::hint::black_box(bytes);
    });
}
