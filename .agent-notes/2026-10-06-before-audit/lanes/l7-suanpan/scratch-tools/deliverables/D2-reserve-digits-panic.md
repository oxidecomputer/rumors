# D2: `reserve_digits` panics on an oversized request, and its rustdoc names no panic

Severity: low (an undocumented panic on a public method; reachable only through an extreme argument).
Status: verified on ox-east-1 (native, 64-bit), explore branch at `df000229`.

## Contract clause breached

`crates/suanpan/src/accumulator.rs:183-198`:

> Reserve capacity for at least `digits` base-2^32 digit positions.
>
> This allocation hint does not change the value. [...]
>
> # Complexity
>
> O(1) time and space if the existing allocation suffices. Otherwise this
> performs at most one allocation [...]

The method has no `# Panics` section. The owner's rule is that a panic must be documented unless only programmer error can trigger it, and this method calls itself a hint. The audit's contract model also requires every operation to be total and panic-free over its inputs.

## Minimal reproduction

The explore test `l7_probe_reserve_digits_extreme` (`crates/suanpan/src/accumulator/tests/explore_l7.rs`, module `probes`) calls `Accumulator::new().reserve_digits(usize::MAX)` inside `catch_unwind` and prints the outcome.

```
/Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh /Users/oxide/src/rumors-audit-l7-suanpan 'unset CARGO_TARGET_DIR; cargo nextest run --locked -p suanpan --all-features --no-capture -E "test(l7_probe_reserve_digits_extreme)"'
```

Output, verbatim:

```
thread 'accumulator::tests::explore_l7::probes::l7_probe_reserve_digits_extreme' (2) panicked at /rustc/8bab26f4f68e0e26f0bb7960be334d5b520ea452/library/alloc/src/raw_vec/mod.rs:28:5:
PROBE reserve_digits(usize::MAX): panicked: capacity overflow
```

## Failure family

`Digits::reserve` (`crates/suanpan/src/accumulator/digits.rs:105-108`) forwards to `Vec::<i64>::reserve_exact(count - len)`. Two regimes, inferred from `Vec`'s documented behavior and confirmed for the first:

- `count * 8 > isize::MAX` bytes (on 64-bit, `count > 2^60 - 1`; on 32-bit, `count > 2^28 - 1`): panic "capacity overflow". Verified at `usize::MAX`.
- A byte size within `isize::MAX` that the allocator cannot supply: `handle_alloc_error`, which aborts the process; this cannot be caught. Not run (it would abort the test binary); inferred from `Vec::reserve_exact`'s contract.

Requests at or below the existing length do nothing (`saturating_sub`).

## Root cause

The hint forwards an infallible `Vec` reservation, so `Vec`'s capacity-overflow panic and allocation-failure abort pass straight through to a method documented as a value-neutral hint.

## Fix preserves API and formats

Yes.

## Fix note

Two candidate repairs; I recommend the first because the method is a hint:

1. Make the hint best-effort: `self.digits.try_reserve_exact(count.saturating_sub(self.digits.len())).ok();` (or ignore the `Err` explicitly), and state in the rustdoc that a reservation the allocator cannot satisfy is ignored. This removes both the panic and the abort, and a later write that genuinely needs the space still fails at that write, where the crate already documents its limits.
2. Keep the behavior and add `# Panics` (capacity overflow) plus a sentence about allocation failure, mirroring `Vec::reserve_exact`.

No other suanpan entry point reserves on the caller's behalf. `before::Rank::accumulate` (`crates/before/src/rank.rs:513-517`) calls `reserve_digits` with a width derived from its operands, which stays proportional to input size, so it is not a practical trigger.

## Test brief (for a demonstrator)

- Invariant: `reserve_digits` is a value-neutral hint for every `usize` request; a request too large to honor leaves the accumulator usable with its value unchanged.
- Form: a unit test over the two named overflowing requests (one specific boundary regime; the allocator-failure regime cannot be exercised without aborting the process, so it stays out of the committed test).
- Where: `crates/suanpan/src/accumulator/tests/witnesses.rs`, beside `reserve_digits_is_value_neutral`.
- Construction: for each request in `[usize::MAX, (isize::MAX as usize) / 8 + 1]` (the first digit count whose byte size exceeds `isize::MAX`), and for both representations (a fresh accumulator holding `7`, and one forced into digits by `add_shifted_limbs(3_200, [1])` plus `+= 7`), call `reserve_digits(request)`, then check the value with the existing `assert_value` helper and that a later `+= 1_i64` still reads back exactly.
- Assertion: the call returns normally, and `assert_value(&acc, &oracle)` holds before and after the follow-up update.
- Expected failure on the base commit: a panic with message `capacity overflow` from `alloc/src/raw_vec`, at the first `reserve_digits` call.
- If the owner chooses the documentation repair instead (fix note option 2), the test becomes a `#[should_panic(expected = "capacity overflow")]` witness of the documented panic, and the rustdoc gains `# Panics`.
