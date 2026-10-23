//! Runs the amplification board's shard child inside a wasm guest, with
//! Wasmtime fuel as its fourth currency.
//!
//! The harness instantiates this module once per board cell and calls
//! [`shard`] with a one-cell deal (`board::max_useful_shards` shards), so
//! every cell runs in a fresh instance and its fuel reading cannot depend on
//! what earlier cells left in the allocator. The guest supplies the board
//! one meter, fuel, read through the host import `board_fuel.fuel_used`. It
//! installs no counting allocator, and `before` is built without the scan and
//! touch probes, so every instruction fuel counts belongs to the measured
//! code, the board's own bookkeeping around it, or the allocator.
//!
//! A cell that panics ends in a trap; [`panic_record`] keeps its message for
//! the host.

mod panic_record;

use std::sync::{Mutex, MutexGuard, PoisonError};

use before::testing::meter::board::{self, FuelMeter, Meters};

// The fuel board's one `unsafe` item: a wasm import can only be declared in
// an `unsafe extern` block, and fuel can only be read through an import. The
// block declares one function, which takes no arguments, touches no guest
// memory, and returns an integer, so it is declared `safe` and no call site
// is `unsafe`. It lives in this tool-side guest alone, which no production
// crate links.
#[link(wasm_import_module = "board_fuel")]
unsafe extern "C" {
    /// Returns the fuel this instance has consumed so far, read by the host
    /// from the store.
    ///
    /// Safe to call: the host function takes no arguments, touches no guest
    /// memory, and returns a plain integer.
    safe fn fuel_used() -> u64;
}

/// Reads this instance's consumed fuel; the board's [`FuelMeter`] reader.
fn read_fuel() -> u64 {
    fuel_used()
}

/// The shard wire form [`shard`] emitted, kept for the host to copy out.
static OUTPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// Locks the output.
///
/// A panic aborts this guest without unwinding, so no guard is dropped
/// mid-panic and the lock is never poisoned; recovering the inner buffer
/// keeps the exports total regardless.
fn output() -> MutexGuard<'static, Vec<u8>> {
    OUTPUT.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Measures shard `index` of `count` at the scale whose IEEE-754 bits are
/// `scale_bits`, keeping the shard wire form for [`output_ptr`] and
/// [`output_len`].
///
/// Returns 0. A board panic traps instead, with its message in the panic
/// record.
#[no_mangle]
pub extern "C" fn shard(scale_bits: u64, index: u32, count: u32) -> u32 {
    panic_record::install();
    let meters = Meters {
        heap: None,
        fuel: Some(FuelMeter { used: read_fuel }),
    };
    let mut emitted = Vec::new();
    board::emit_shard(
        f64::from_bits(scale_bits),
        usize::try_from(index).expect("a u32 fits in wasm32's usize"),
        usize::try_from(count).expect("a u32 fits in wasm32's usize"),
        &meters,
        &mut emitted,
    )
    .expect("writing to a Vec cannot fail");
    *output() = emitted;
    0
}

/// Returns the linear-memory address of the output's first byte.
#[no_mangle]
pub extern "C" fn output_ptr() -> u32 {
    u32::try_from(output().as_ptr().addr()).expect("a wasm32 address fits in u32")
}

/// Returns the output's length in bytes.
#[no_mangle]
pub extern "C" fn output_len() -> u32 {
    u32::try_from(output().len()).expect("a wasm32 length fits in u32")
}
