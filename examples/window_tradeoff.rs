//! Regenerate the sizing guide's trade-off table with `just window-tradeoff`.
//!
//! The table uses the window calculation without running sessions. The window
//! tests compare the committed table with this output to detect stale values.

/// Print the generated Markdown table to standard output.
fn main() {
    print!("{}", rumors::testing::window_tradeoff_table());
}
