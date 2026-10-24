//! Runs the amplification board's shard child inside a wasm guest, with
//! Wasmtime fuel as its fourth currency.
//!
//! The harness instantiates this module once per board cell and calls
//! [`shard`] with a one-cell deal (`board::max_useful_shards` shards), so
//! every cell runs in a fresh instance and no state an earlier cell left,
//! such as a lazily initialized static, reaches its reading. The guest
//! supplies the board one meter, fuel, read through the host import
//! `board_fuel.fuel_used`. `before` is built without the scan and touch
//! probes, so every instruction fuel counts belongs to the measured code, the
//! board's own bookkeeping around it, or the allocator.
//!
//! The global allocator, [`window::WindowAllocator`], gives each measured
//! window a fresh, primed heap through the meter's `enter` and `exit` hooks,
//! so the allocator's share of a reading is its work for the body's own
//! allocations, whatever the cell's preparation left in the heap.
//!
//! A cell that panics ends in a trap; [`panic_record`] keeps its message for
//! the host.

mod panic_record;
mod window;

use std::hint::black_box;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use before::testing::meter::board::{self, FuelMeter, Meters, Sampling};

#[global_allocator]
static ALLOCATOR: window::WindowAllocator = window::WindowAllocator;

// A wasm import can only be declared in an `unsafe extern` block, and fuel
// can only be read through an import. The block declares one function, which
// takes no arguments, touches no guest memory, and returns an integer, so it
// is declared `safe` and no call site is `unsafe`. It lives in this tool-side
// guest alone, which no production crate links; the guest's only other
// `unsafe` code is its global allocator, in `window`.
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

/// Opens a measured window: the board's [`FuelMeter`] `enter` hook.
///
/// A nonzero [`set_spacer`] size first allocates a spacer of that many bytes
/// from the ordinary heap, held until the window closes.
fn enter_window() {
    let bytes = SPACER_BYTES.load(Ordering::Relaxed);
    if bytes != 0 {
        let bytes = usize::try_from(bytes).expect("a u32 fits in wasm32's usize");
        *spacer() = black_box(vec![0; bytes]);
    }
    window::enter();
}

/// Closes the measured window, then frees the spacer, if one is held: the
/// board's [`FuelMeter`] `exit` hook.
fn exit_window() {
    window::exit();
    *spacer() = Vec::new();
}

/// The size in bytes of the spacer each measured window holds, or zero for
/// none; set by [`set_spacer`].
static SPACER_BYTES: AtomicU32 = AtomicU32::new(0);

/// The spacer the open window holds, or an empty vector.
static SPACER: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// Locks the spacer.
///
/// A panic aborts this guest without unwinding, so no guard is dropped
/// mid-panic and the lock is never poisoned; recovering the inner buffer
/// keeps the hooks total regardless.
fn spacer() -> MutexGuard<'static, Vec<u8>> {
    SPACER.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Makes every later measured window hold a spacer of `bytes` bytes,
/// allocated from the ordinary heap just before the window opens and freed
/// just after it closes; zero holds none, as in an ordinary run.
///
/// A spacer changes the heap state the cell's preparation leaves, and nothing
/// the body does, so with the window allocator every reading must equal the
/// reading without one. The harness's window test checks that.
#[no_mangle]
pub extern "C" fn set_spacer(bytes: u32) {
    SPACER_BYTES.store(bytes, Ordering::Relaxed);
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

/// Measures shard `index` of `count` at the sampling whose words, as
/// [`Sampling::to_words`] writes them, are `sampling_tag` and
/// `sampling_word`, keeping the shard wire form for [`output_ptr`] and
/// [`output_len`].
///
/// Returns 0, or 1 for words that name no sampling. A board panic traps
/// instead, with its message in the panic record.
#[no_mangle]
pub extern "C" fn shard(sampling_tag: u32, sampling_word: u64, index: u32, count: u32) -> u32 {
    panic_record::install();
    let Some(sampling) = Sampling::from_words(sampling_tag, sampling_word) else {
        return 1;
    };
    let meters = Meters {
        heap: None,
        fuel: Some(FuelMeter {
            used: read_fuel,
            enter: enter_window,
            exit: exit_window,
        }),
    };
    let mut emitted = Vec::new();
    board::emit_shard(
        sampling,
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
