//! Runs `before`'s focused cost probes inside a wasm guest, reading Wasmtime
//! fuel around one call of each probe's measured operation.
//!
//! The harness (the `focused-fuel-harness` crate) instantiates this module
//! afresh for every measurement and calls [`measure`] with a
//! [`Probe`] number and two parameters. The guest builds the probe's operands,
//! opens a measured window, runs the operation, closes the window, and then
//! checks the operation's answer, so neither preparation nor the check enters
//! the reading. [`reading`] returns the fuel the window consumed.
//!
//! The global allocator is the fuel board's window allocator: each window runs
//! on a fresh, primed heap, so a reading does not depend on what preparation
//! left in the heap. That module and the panic record are the board guest's
//! own source files, included by path rather than through a shared crate:
//! moving them into a crate would change the board guest's compiled code, and
//! with it the fuel its ceilings were calibrated against. Their prose speaks
//! of the board's cells; here the one measured window is one probe's
//! operation.

mod early_exit;
mod identity;
#[path = "../../../board-fuel/guest/src/panic_record.rs"]
mod panic_record;
#[path = "../../../board-fuel/guest/src/window.rs"]
mod window;

use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};

use focused_fuel_protocol::{Probe, MEASURED, UNKNOWN_PROBE};

#[global_allocator]
static ALLOCATOR: window::WindowAllocator = window::WindowAllocator;

// A wasm import can only be declared in an `unsafe extern` block, and fuel
// can only be read through an import. The block declares one function, which
// takes no arguments, touches no guest memory, and returns an integer, so it
// is declared `safe` and no call site is `unsafe`. It lives in this tool-side
// guest alone, which no production crate links; the guest's only other
// `unsafe` code is its global allocator, in `window`.
#[link(wasm_import_module = "focused_fuel")]
unsafe extern "C" {
    /// Returns the fuel this instance has consumed so far, read by the host
    /// from the store.
    ///
    /// Safe to call: the host function takes no arguments, touches no guest
    /// memory, and returns a plain integer.
    safe fn fuel_used() -> u64;
}

/// The fuel the last measured window consumed.
static READING: AtomicU64 = AtomicU64::new(0);

/// Measures probe `number` with parameters `a` and `b`, keeping the reading
/// for [`reading`].
///
/// Returns [`MEASURED`], or [`UNKNOWN_PROBE`] for a number that names no
/// probe. A wrong answer, or parameters the probe cannot use, panics instead,
/// which traps with the message in the panic record.
#[no_mangle]
pub extern "C" fn measure(number: u32, a: u64, b: u64) -> u32 {
    panic_record::install();
    let Ok(probe) = Probe::try_from(number) else {
        return UNKNOWN_PROBE;
    };
    READING.store(run(probe, a, b), Ordering::Relaxed);
    MEASURED
}

/// Returns the fuel the last measured window consumed.
#[no_mangle]
pub extern "C" fn reading() -> u64 {
    READING.load(Ordering::Relaxed)
}

/// Dispatches `probe` to the function that builds its operands and measures
/// it.
fn run(probe: Probe, a: u64, b: u64) -> u64 {
    match probe {
        Probe::EqRefutedAtFirstInterval => early_exit::eq_refuted_at_first_interval(only(a, b)),
        Probe::CloneCompare => identity::clone_compare(only(a, b)),
        Probe::CloneJoin => identity::clone_join(only(a, b)),
        Probe::CloneMeet => identity::clone_meet(only(a, b)),
        Probe::CloneSpan => identity::clone_span(only(a, b)),
        Probe::CloneJoinAll => identity::clone_join_all(only(a, b)),
        Probe::CloneMeetAll => identity::clone_meet_all(only(a, b)),
        Probe::CloneSpanAll => identity::clone_span_all(only(a, b)),
        Probe::CloneDistance => identity::clone_distance(only(a, b)),
        Probe::CloneLag => identity::clone_lag(only(a, b)),
        Probe::RedecodedCompare => identity::redecoded_compare(only(a, b)),
        Probe::RedecodedJoin => identity::redecoded_join(only(a, b)),
        Probe::RedecodedMeet => identity::redecoded_meet(only(a, b)),
        Probe::RedecodedSpan => identity::redecoded_span(only(a, b)),
        Probe::RedecodedDistance => identity::redecoded_distance(only(a, b)),
        Probe::RedecodedLag => identity::redecoded_lag(only(a, b)),
        Probe::EmptyJoinLeft => identity::empty_join_left(only(a, b)),
        Probe::EmptyJoinRight => identity::empty_join_right(only(a, b)),
        Probe::EmptyMeetLeft => identity::empty_meet_left(only(a, b)),
        Probe::EmptyMeetRight => identity::empty_meet_right(only(a, b)),
        Probe::EmptySpanLeft => identity::empty_span_left(only(a, b)),
        Probe::EmptySpanRight => identity::empty_span_right(only(a, b)),
        Probe::EmptyJoinAssignLeft => identity::empty_join_assign_left(only(a, b)),
        Probe::EmptyJoinAssignRight => identity::empty_join_assign_right(only(a, b)),
        Probe::EmptyMeetAssignLeft => identity::empty_meet_assign_left(only(a, b)),
        Probe::EmptyMeetAssignRight => identity::empty_meet_assign_right(only(a, b)),
        Probe::EmptyJoinAllRight => identity::empty_join_all_right(only(a, b)),
        Probe::EmptyJoinAllLeft => identity::empty_join_all_left(only(a, b)),
        Probe::EmptyMeetAllRight => identity::empty_meet_all_right(only(a, b)),
        Probe::EmptyMeetAllLeft => identity::empty_meet_all_left(only(a, b)),
    }
}

/// Returns parameter `a` as a size, for a probe that uses `a` alone.
///
/// # Panics
///
/// Panics if `b` is not zero, or if `a` does not fit wasm32's `usize`.
fn only(a: u64, b: u64) -> usize {
    assert_eq!(b, 0, "this probe takes one parameter, so `b` must be zero");
    size(a)
}

/// Returns a parameter as a size.
///
/// # Panics
///
/// Panics if the parameter does not fit wasm32's `usize`.
fn size(parameter: u64) -> usize {
    usize::try_from(parameter).expect("a probe's size parameter fits wasm32's usize")
}

/// Runs `body` in a measured window and returns the fuel it consumed, then
/// passes its result to `check`, outside the window.
///
/// Operands are built before this is called, and the result is checked and
/// dropped after the window closes, so the reading is the operation's own
/// instructions, plus a fixed cost for the window's two fuel reads and the
/// call of `body`.
fn measured<R>(body: impl FnOnce() -> R, check: impl FnOnce(R)) -> u64 {
    let origin = enter_window();
    let result = black_box(body());
    let used = exit_window()
        .checked_sub(origin)
        .expect("an instance's consumed fuel never decreases");
    check(result);
    used
}

/// Opens a measured window, then returns this instance's consumed fuel.
///
/// The read is the hook's last act, and `window::enter` runs in a separate
/// function, so no instruction of the hook runs after the read.
fn enter_window() -> u64 {
    open_window();
    fuel_used()
}

/// Returns this instance's consumed fuel, then closes the measured window.
///
/// The read is the hook's first act, and `window::exit` runs in a separate
/// function, so no instruction of the hook runs before the read.
fn exit_window() -> u64 {
    let used = fuel_used();
    close_window();
    used
}

/// Opens the measured window.
#[inline(never)]
fn open_window() {
    window::enter();
}

/// Closes the measured window.
#[inline(never)]
fn close_window() {
    window::exit();
}
