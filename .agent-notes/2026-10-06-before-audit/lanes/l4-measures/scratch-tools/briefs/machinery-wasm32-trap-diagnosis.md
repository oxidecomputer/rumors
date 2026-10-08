# Machinery brief: tell a panic from an allocation abort in `wasm32-pins`

## Failure class it catches (constructible)

`wasm32_pins_harness::run` reports every guest trap as
`Outcome::Trapped(Trap::UnreachableCodeReached)`. On `wasm32-unknown-unknown`
both a panic (panic = abort) and an allocation failure
(`handle_alloc_error` aborts) end in that same trap, so the harness cannot
tell them apart. A pin that asserts a trap therefore passes for the wrong
reason when the code under test aborts on memory instead of panicking with its
documented rejection.

The committed pin `suanpan_rejects_unaddressable_digit_landings`
(`crates/before/wasm32-pins/harness/tests/pins.rs`) asserts exactly
`Outcome::Trapped(Trap::UnreachableCodeReached)` for suanpan's documented
digit-landing panic. A wrong implementation that drops the landing check and
instead requests a huge buffer near `isize::MAX` (rather than wrapping)
would abort with the identical outcome, so the pin would stay green. That
consequence is inferred from the measured outcome identity below; I did not
build the wrong implementation. Measured evidence
that the outcomes coincide (base `58285ca5` plus prototype): an allocation
abort reached by replaying `Sum`'s accumulator calls reports
`Trapped(UnreachableCodeReached) pages=42014 panic=""`, while suanpan's
documented panic reports `Trapped(UnreachableCodeReached) pages=18
panic="panicked at .../suanpan/src/accumulator.rs:417:10:\na nonzero
contribution needs an addressable digit position"`.

## Which instrument it extends, and why no existing one can

It extends `wasm32-pins` itself; no native instrument can observe wasm32
traps. The protocol (`wasm32-pins/protocol`) and `Outcome` stay as they are;
the change adds a recorded panic message beside the trap.

## Construction (prototype on `explore/l4-measures` at `2c82c7ba`)

- Guest (`wasm32-pins/guest/src/lib.rs`): at entry to `check`, install a
  panic hook (`std::panic::set_hook`) that stores `info.to_string()` into a
  `static Mutex<Vec<u8>>`, and export `panic_message_len() -> u32` and
  `panic_message_byte(i: u32) -> u32`. No unsafe code is needed. The
  allocation-failure path does not call the panic hook, so an empty message
  after a trap means a non-panic abort.
- Harness (`wasm32-pins/harness/src/lib.rs`): after a trap, call those
  exports on the same instance (wasmtime permits calls after a trap) and
  return the message, plus the memory size in pages from
  `instance.get_memory(&mut store, "memory")`. The prototype is
  `run_diagnosed`; a mergeable version should fold the message into
  `Outcome::Trapped` (for example `Trapped { trap, panic: Option<String> }`)
  so every trap-expecting pin must state which panic it expects.
- Pins: `suanpan_rejects_unaddressable_digit_landings` asserts the panic
  message contains suanpan's documented text ("a nonzero contribution needs an
  addressable digit position"); `harness_outcomes_are_live` asserts the
  deliberate trap carries "deliberate harness trap" and adds one deliberate
  allocation abort (for example `Vec::<u8>::with_capacity(isize::MAX as usize
  - 1)` in a new harness-only check, or a `Vec` growth past the guest's
  memory) that must report a trap with no panic message, proving the two are
  distinguished.

## Calibration evidence

On ox-east-1, release guest and harness (`wasm-3a.log`, `wasm-3b.log`):

- `HarnessTrap`: `panic="panicked at guest/src/checks.rs:20:31:\ndeliberate
  harness trap"`.
- `SuanpanLanding` case 1: the documented suanpan message above.
- Accumulator replay of `Sum`'s deep-first calls (prototype case 11):
  `Trapped(UnreachableCodeReached) pages=42014 panic=""`; the same replay
  after an exact `reserve_digits` (case 12): `Passed pages=42014`. The empty
  message identifies the abort as allocation failure, and the reservation
  removing it confirms the cause.

## Index entry

Add a `validation_index.rs` row for the `wasm32-pins` workspace (currently
absent from the index), naming its failure class (pointer-width boundary
behavior, including the distinction this brief adds).

## Lane note

This instrument serves suanpan's pins (L7) and the adequacy lane (L8) as much
as L4; I built and calibrated it while investigating `Sum` on wasm32.
