# builder-peakalloc-single-thread NOTES

Slot /Users/oxide/src/rumors-slot-41, branch proposal/peakalloc-single-thread, base 8b28bbd81 (verified, clean, ddfabe4c ancestor).
Brief: survey-proposals.md section 41; survey 1.6; l8 coordinator-leads item 1.

## Established
- libtest 1.97.1 lib.rs: concurrent path inserts into running_tests/timeout_queue after spawn; concurrency==1 path calls rx.recv() right after spawn.
- std 1.97.1 mpmc list.rs recv -> Context::with (TLS Arc::new on first use) + receivers.register (selectors.push into empty Vec). Both persist. So single-thread path ALSO allocates on main after spawn (source reading).
- nextest 0.9.140 supports run-extra-args per override; honors cargo [env].
- landing check calls cargo nextest directly (not via just) and stable `cargo test --doc` (edition 2021).
- rumors tests/support/allocation.rs already per-thread (not exposed).

## In flight
- Temporary probe crates/before/tests/zz_libtest_race_probe.rs (UNCOMMITTED, delete after). Delays main-thread allocs 2ms; test asserts no main alloc in window. Run default vs RUST_TEST_THREADS=1.

## Next
- If single-thread probe fails: brief premise false -> stop and report, no commit.

## Result (final)
- Probe ran on box: concurrent path main_allocs_in_window=4 sizes=[608,48,24,96] live_delta=924; RUST_TEST_THREADS=1 main_allocs_in_window=2 sizes=[24,96] live_delta=168. Both FAIL. Logs: probe-concurrent.log, probe-single.log. Probe source kept here.
- Observed failure 120 B = 24 + 96: the receive-side allocations the single-thread path keeps.
- Probe deleted from slot; git status/diff empty; HEAD 8b28bbd81. No commits. Reported to coordinator as blocked (premise false).
