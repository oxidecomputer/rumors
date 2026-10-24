//! Checks, on representative cells, that changing the heap state a cell's
//! preparation leaves behind moves none of the cell's fuel readings.
//!
//! The guest gives each measured window a fresh heap (its `window` module).
//! These tests run real board cells in the guest that the
//! `amp-board-fuel-build` recipe builds, named by `BOARD_FUEL_GUEST_WASM`;
//! the `amp-board-fuel` recipe runs them before it judges the board.

use before::testing::meter::board::{self, Sampling, DEFAULT_SCALE};
use board_fuel_harness::FuelBoard;

/// The environment variable naming the compiled guest module.
const GUEST_ENV: &str = "BOARD_FUEL_GUEST_WASM";

/// The cells checked, as (operation, family), each chosen for the allocator
/// calls its body makes.
const CELLS: [(&str, &str); 6] = [
    // Clones a rank: allocations only, into an empty window heap.
    ("rank_clone", "memo-oscillating"),
    // Folds many spans: a long sequence of allocations and frees.
    ("span_join_all", "scatter"),
    // Decodes a version, then drops its input bytes inside the window: a
    // deferred free of a block allocated before the window.
    ("version_decode", "hugeleaf"),
    // Deserializes a clock, likewise dropping its input bytes in the window.
    ("clock_borsh_deserialize", "hoisted-window"),
    // Joins into a version in place: hundreds of reallocations of blocks
    // the window heap allocated.
    ("version_join_assign", "jump-pair"),
    // Deserializes a span from a byte buffer that serde hands to the
    // decoder, which reallocates that buffer, a block allocated before the
    // window: a move into the window heap with a copy of its contents.
    ("span_serde_deserialize", "alt-spine"),
];

/// The spacer sizes in bytes: one small enough to fit in memory the ordinary
/// heap already holds, and one far larger than a wasm page, which may make
/// the ordinary heap grow linear memory and so move each window's base.
const SPACERS: [u32; 2] = [24, 16 << 20];

/// Reads the compiled guest that `BOARD_FUEL_GUEST_WASM` names.
fn guest() -> Vec<u8> {
    let path = std::env::var_os(GUEST_ENV).unwrap_or_else(|| {
        panic!("{GUEST_ENV} must name the compiled guest; run `just amp-board-fuel`")
    });
    std::fs::read(&path).unwrap_or_else(|error| panic!("reading {path:?}: {error}"))
}

/// For each of [`CELLS`] at the default scale, a spacer of every size in
/// [`SPACERS`], held across each measured window, leaves the cell's capture
/// byte-identical to the capture without one.
///
/// A spacer changes only the heap state that the cell's preparation leaves
/// for the body, so equal captures mean that on these cells, under these
/// spacers, no reading moved, both samples' fuel included. Each plain capture
/// must contain its cell's line, so an empty shard cannot pass by matching
/// another empty shard; [`spacer_reaches_the_guest`] shows that the spacer
/// is allocated at all.
#[test]
fn spacer_moves_no_reading() {
    let fuel_board = FuelBoard::new(&guest(), 1).expect("the guest compiles");
    for (op, family) in CELLS {
        let index = board::cell_shard(op, family)
            .unwrap_or_else(|| panic!("{op} x {family} is a board cell"));
        let plain = fuel_board
            .spaced_shard(Sampling::Scaled(DEFAULT_SCALE), index, 0)
            .unwrap_or_else(|error| panic!("{op} x {family}: {error}"));
        let line = format!("\ncell\t{op}\t{family}\t");
        assert!(
            plain
                .windows(line.len())
                .any(|window| window == line.as_bytes()),
            "the shard for {op} x {family} measured that cell"
        );
        for spacer in SPACERS {
            let spaced = fuel_board
                .spaced_shard(Sampling::Scaled(DEFAULT_SCALE), index, spacer)
                .unwrap_or_else(|error| panic!("{op} x {family}, spacer {spacer}: {error}"));
            assert!(
                spaced == plain,
                "{op} x {family}: a {spacer}-byte spacer moved the capture\n\
                 without: {}\n   with: {}",
                String::from_utf8_lossy(&plain),
                String::from_utf8_lossy(&spaced)
            );
        }
    }
}

/// A spacer reaches the guest's allocation path: one of `u32::MAX` bytes,
/// more than any wasm32 allocation can hold, traps the shard with `std`'s
/// capacity-overflow panic.
///
/// This is the liveness check for [`spacer_moves_no_reading`]: its equal
/// captures are evidence only because the spacer is allocated before each
/// window opens.
#[test]
fn spacer_reaches_the_guest() {
    let fuel_board = FuelBoard::new(&guest(), 1).expect("the guest compiles");
    let (op, family) = CELLS[0];
    let index =
        board::cell_shard(op, family).unwrap_or_else(|| panic!("{op} x {family} is a board cell"));
    let error = fuel_board
        .spaced_shard(Sampling::Scaled(DEFAULT_SCALE), index, u32::MAX)
        .expect_err("a spacer of u32::MAX bytes cannot be allocated")
        .to_string();
    assert!(error.contains("capacity overflow"), "{error}");
}
