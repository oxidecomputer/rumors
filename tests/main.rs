//! Centralizes proptest regression seeds for the integration suites.
//!
//! Proptest looks upward from a test source for the nearest `lib.rs` or
//! `main.rs`. This deliberately empty binary supplies that anchor, placing a
//! seed for `tests/<suite>.rs` at `proptest-regressions/<suite>.txt`.
//!
//! `tests/seed_liveness.rs` verifies that every committed seed occupies the
//! path proptest will read.
