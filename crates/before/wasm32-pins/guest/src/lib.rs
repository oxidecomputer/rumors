//! Executes `before` and `suanpan` with a genuinely 32-bit `usize`.
//!
//! Ordinary host tests cannot reach pointer-width boundaries on a 64-bit
//! machine. This guest synthesizes the large operands inside wasm and applies
//! the public operations there. Only a typed check number and two `u64`
//! parameters cross the host boundary, so the property being tested is never
//! accidentally performed by the 64-bit harness.
//!
//! Checks return a typed failure without trapping. Product checks therefore
//! trap only when the exercised operation panics, including `suanpan`'s
//! documented rejection of an unaddressable digit landing. One explicit
//! harness check traps deliberately so the native driver proves it preserves
//! that distinction.

mod checks;
mod synthesis;

use wasm32_pins_protocol::{Check, Failure, PASS};

/// Runs one typed boundary check.
#[no_mangle]
pub extern "C" fn check(number: u32, a: u64, b: u64) -> i32 {
    let check = match Check::try_from(number) {
        Ok(check) => check,
        Err(()) => return Failure::UnknownCheck.code(),
    };
    match checks::run(check, a, b) {
        Ok(()) => PASS,
        Err(failure) => failure.code(),
    }
}
