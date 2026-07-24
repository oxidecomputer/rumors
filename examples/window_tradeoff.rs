//! Emit the sync-budget trade-off table included in the rustdoc.
//!
//! Pure arithmetic from the closed form documented at
//! `Peer::sync_memory_budget` — no sessions, no timing — so the output
//! is deterministic. Regenerate with `just window-tradeoff`; the output
//! lands in `src/tree/mirror/streaming/window/tradeoff.md`, and the
//! window suite byte-compares the committed file against the same
//! rendering, so drift fails the gate.

fn main() {
    print!("{}", rumors::testing::window_tradeoff_table());
}
