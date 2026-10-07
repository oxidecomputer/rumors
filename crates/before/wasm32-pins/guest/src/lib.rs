//! Executes `before` and `suanpan` with a genuinely 32-bit `usize`.
//!
//! Ordinary host tests cannot reach pointer-width boundaries on a 64-bit
//! machine. This guest synthesizes the large operands inside wasm and applies
//! the public operations there. Only a typed check number and two `u64`
//! parameters cross the host boundary, so the property being tested is never
//! accidentally performed by the 64-bit harness.
//!
//! Checks return a typed failure without trapping. Product checks therefore
//! trap only when the exercised operation panics, as `suanpan` documents for
//! an unaddressable digit landing, or when the guest exhausts its memory.
//! Both end in the same wasm trap, so the guest records a panic's message for
//! the host to read afterwards; the `panic_record` module states when a panic
//! goes unrecorded. Harness checks trap deliberately in each way, so the
//! native driver proves it tells a pass, a panic, and an abort without a
//! panic apart.

mod checks;
mod panic_record;
mod synthesis;

use wasm32_pins_protocol::{Check, Failure, PASS};

/// Runs one typed boundary check.
#[no_mangle]
pub extern "C" fn check(number: u32, a: u64, b: u64) -> i32 {
    panic_record::install();
    let check = match Check::try_from(number) {
        Ok(check) => check,
        Err(()) => return Failure::UnknownCheck.code(),
    };
    match checks::run(check, a, b) {
        Ok(()) => PASS,
        Err(failure) => failure.code(),
    }
}
