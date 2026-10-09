//! Records the message of the panic that ends a check, for the host to read
//! after the trap.
//!
//! On `wasm32-unknown-unknown` both a panic and a failed allocation abort the
//! guest, and both aborts reach the same `unreachable` trap, so the trap alone
//! cannot say which happened. Only a panic runs the panic hook, so a recorded
//! message always identifies a panic.
//!
//! The converse is weaker: a trap with nothing recorded means only that no
//! panic reached the hook. Usually the guest ran out of memory, and that can
//! happen while a panic is under way. Std formats every panic message that
//! has runtime arguments into a heap-allocated `String` before it calls the
//! hook. Every `expect` message is built that way, even when the caller
//! passes a literal, and so is `Result::unwrap`'s. Such a panic raised after
//! memory is exhausted aborts while formatting and records nothing. A panic
//! whose message is a constant with no runtime arguments, such as
//! `Option::unwrap`'s, needs no allocation and is recorded even then.
//!
//! The hook itself copies the message into static storage without
//! allocating, so recording adds no allocation of its own.

use std::panic::{self, PanicHookInfo};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// The most message bytes the record keeps.
///
/// Every message the pins expect is far shorter. A longer message keeps its
/// longest prefix that ends on a character boundary.
const CAPACITY: usize = 1024;

/// The message of the panic that ended this instance, if one did.
struct PanicRecord {
    /// Whether the panic hook has run.
    panicked: bool,
    /// The number of bytes of `message` that hold the recorded text.
    len: usize,
    /// The message as UTF-8, truncated to [`CAPACITY`] bytes.
    message: [u8; CAPACITY],
}

/// The record that the panic hook writes and the exports below read.
static RECORD: Mutex<PanicRecord> = Mutex::new(PanicRecord {
    panicked: false,
    len: 0,
    message: [0; CAPACITY],
});

/// Installs the hook that records the next panic's message.
pub fn install() {
    // A function item is zero-sized, so boxing it allocates nothing.
    panic::set_hook(Box::new(record));
}

/// Copies a panic's message into the record without allocating.
fn record(info: &PanicHookInfo<'_>) {
    // std's default hook names a payload that is not a string the same way.
    let message = info.payload_as_str().unwrap_or("Box<dyn Any>");
    let mut len = message.len().min(CAPACITY);
    // Index zero is always a character boundary, so this stops at or above it.
    while !message.is_char_boundary(len) {
        len -= 1;
    }
    let mut record = lock();
    record.message[..len].copy_from_slice(&message.as_bytes()[..len]);
    record.len = len;
    record.panicked = true;
}

/// Locks the record.
///
/// A panic aborts this guest without unwinding, so no guard is dropped
/// mid-panic and the lock is never poisoned; recovering the inner record
/// keeps the exports total regardless.
fn lock() -> MutexGuard<'static, PanicRecord> {
    RECORD.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Returns 1 if a panic has been recorded, and 0 otherwise.
#[no_mangle]
pub extern "C" fn panic_recorded() -> u32 {
    u32::from(lock().panicked)
}

/// Returns the linear-memory address of the recorded message's first byte.
///
/// The record is static, so the address stays valid after the trap.
#[no_mangle]
pub extern "C" fn panic_message_ptr() -> u32 {
    u32::try_from(lock().message.as_ptr().addr()).expect("a wasm32 address fits in u32")
}

/// Returns the recorded message's length in bytes.
#[no_mangle]
pub extern "C" fn panic_message_len() -> u32 {
    u32::try_from(lock().len).expect("the record holds at most `CAPACITY` bytes")
}
