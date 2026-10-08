# Simplification brief: state `Rank`'s normalization invariant as the code keeps it

Kind: self-contained, behavior-preserving prose correction (maintainer-facing
docs only; no public API, format, or test change).

## Current text (base 58285ca5)

- `crates/before/src/rank.rs:10-11`, module doc, section "Canonical
  representation":

  > A rank is stored as `num / 2^exp`. The numerator is odd unless the value is
  > zero, whose exponent is also zero.

- `crates/before/src/rank.rs:130-131`, doc on the private field `Rank::num`:

  > The numerator. Normalized: odd, or zero with `exp` zero, so each
  > value has exactly one representation.

## Why it is wrong (verified by reading the code; no run needed)

`Rank::from_raw` (`rank.rs:479-493`) strips `min(trailing_zeros(num), exp)`
factors of two. When `exp` is zero nothing is stripped, so every even integer
keeps an even numerator: the integer 2 is stored as `(num, exp) = (2, 0)`, the
integer 4 as `(4, 0)`. `FromStr` (`"10"` parses to `num = 2, exp = 0`),
`decode_stream` (an integral-only stream yields `num = integral, exp = 0`), and
`Version::rank` (`Rank::from_raw(numerator, max_depth)`) all produce such
values. The code's own statements of the invariant are the correct ones:

- `rank.rs:29` (module doc): "Because a normalized fractional numerator is odd".
- `rank.rs:562-563` (encoder comment): "an odd numerator whenever exp > 0".
- `rank.rs:715-718` (decoder `debug_assert!`): `exp == 0 || num.bit(0)`.
- `rank.rs:1180-1181` (Display comment): "A normalized non-integral rank has a
  one in bit zero".

The two sentences above contradict these and the code. A maintainer who trusted
them could write code that assumes an odd numerator for every nonzero rank
(for example a comparison tail rule, or a trailing-zero shortcut), which would
be wrong on even integers.

## Proposed text

Module doc, replacing the second sentence:

> A rank is stored as `num / 2^exp` with no factor of two shared by numerator
> and denominator: a positive exponent implies an odd numerator, and zero is
> stored with exponent zero. An integral rank therefore has exponent zero and
> may have an even numerator.

Field doc on `num`:

> The numerator. Normalized: odd whenever `exp` is positive, and zero only with
> `exp` zero, so each value has exactly one representation.

## Why the result is more obviously correct

The invariant is then stated once, positively, in the same form the encoder,
decoder, and Display rely on, and it matches `from_raw`'s `tz.min(exp)`
directly.

## Coverage

No behavior changes. The invariant itself is already checked:
`rank_cross_path_normalization` (law registry) and the decoder's
`debug_assert!`. Rendered-doc check: `just internal-docs` (private items).
