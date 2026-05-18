//! Deterministic simulation test suite for the `rumors` public API.
//!
//! See `/Users/oxide/.claude/plans/i-d-like-to-make-golden-ritchie.md` for the
//! design. Each `group_*` module pins down one invariant group from the plan;
//! `oracle`, `peer`, and `schedule` are shared helpers.

mod oracle;
mod peer;
mod schedule;

mod group_a;
mod group_b;
mod group_c;
mod group_d;
mod group_e;
mod group_f;
mod wire;
