# Machinery brief: drive `Accumulator::normalize` with stored digits at their bound

Owner lane: suanpan (L7). Found by the adequacy lane's mutation campaign and
branch coverage.

## The failure class it catches

`Accumulator::normalize` (`crates/suanpan/src/accumulator.rs:240`, kernel
`crates/suanpan/src/accumulator/digits/normalize.rs:35-88`) has two width
changes beyond its main carry pass:

- **growth by one digit** (lines 66-76): the carry out of the top position
  is stored as a new digit, `(polarity * remainder) as i64` at line 74;
- **shrinking** (lines 81-84): `highest` is lowered past digits the pass
  zeroed, which the public doc promises ("Cancellation can make `Q`
  arbitrarily smaller than `W`", accumulator.rs:229-231).

Neither is checked by any test:

- Coverage (coverage/suanpan__accumulator__digits__normalize.txt): lines
  82-84 never execute; the loop condition's `digits[highest] == 0` outcome is
  never true.
- Mutation (survivors/suanpan__accumulator__digits__normalize.txt), all
  surviving the full suanpan suite under `PROPTEST_RNG_SEED=8008`:
  `normalize.rs:74:46 replace * with /`, `81:23 replace > with ==`, `< `,
  `>=`, `83:21 replace -= with +=`, `/=`.

The surface model (`accumulator/tests/surface.rs:474-489`) does run
`Operation::Normalize` and asserts the exact resulting width, so the
assertions exist; its operation generator just never builds stored digits at
the representation's bound beneath a small top digit.

## Witnesses (verified on ox-east-1, explore/l8-adequacy, `l8/l8_normalize_probe.rs`)

Both are built through public operations only. Negative deposits stay lazy
in the stored digits, so repeated subtraction at one position reaches the
bound `-(2B - 1)` with `B = 2^32`; positive deposits carry eagerly and do not.

1. **Shrink.**
   `a.add_shifted_limbs(64, [3]); for s in [0, 32] { a.sub_shifted_limbs(s, [u32::MAX as u64]); a.sub_shifted_limbs(s, [u32::MAX as u64]); a.sub_shifted_limbs(s, [1]); }`
   gives stored digits `[-8589934591, -8589934591, 3]` (that is
   `[-(2B-1), -(2B-1), 3]`, value `B^2 - B + 1`). `normalize()` enters the
   shrinking loop (verified by a flag in the probe copy) and leaves
   `digits [1, 4294967295]`, `highest_nonzero 1`, value
   `18446744069414584321`.
2. **Growth with top carry 2.**
   `for s in [32, 0] { a.sub_shifted_limbs(s, [u32::MAX as u64]); a.sub_shifted_limbs(s, [u32::MAX as u64]); a.sub_shifted_limbs(s, [1]); }`
   gives stored digits `[-8589934591, -8589934591]`. `normalize()` grows to
   `[-4294967295, 0, -2]`, value `-36893488151714070527`. Under the surviving
   mutant at line 74 (`*` replaced by `/`) the same input yields value
   `-4294967295`: a wrong value the full suite does not detect.

The cancelling compaction inside `cmp_zero`
(`digits/sign.rs:57-101`, threshold `COMPARISON_DECIDED = 3`) runs first and
absorbs most top cancellation, which is why ordinary cancelling sequences
never reach the shrinking loop: it needs a top digit of exactly 3 above
digits at the negative bound.

## Proposed machinery

Extend the surface model's operand generator rather than adding a parallel
test: add an operation (or a state-construction prefix) that deposits
`-(B - 1)` repeatedly at one shifted position, so lazily accumulated digits
reach `-(2B - 1)`, mixed with a small positive top digit placed by a shifted
add. The existing `Operation::Normalize` assertions (exact width, at most one
digit of growth) and the oracle comparison then decide. Two point tests
pinning the witnesses above are reasonable as named regression cases.

## Calibration

The extended generator must fail under each listed mutant; the line-74 one
already demonstrably changes the value for witness 2. A builder should check
the shrinking-loop mutants (`-=` to `+=` makes `highest` climb past the
buffer, `> ` to `>=` lets `highest` underflow when the low digit is zero) by
reversible string swap, each failing.
