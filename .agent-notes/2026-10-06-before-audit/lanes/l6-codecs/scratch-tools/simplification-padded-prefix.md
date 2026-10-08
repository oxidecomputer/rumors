# Simplification brief: one padded-prefix step for `Party` and `Version`

Kind: self-contained, behavior-preserving, no public API or format change.
Base: `main` at `58285ca5`.

## Current code

Two decoders find a byte-aligned value at the head of a larger buffer with the
same seven-step sequence: validate the tree from the whole buffer, compute the
padded byte length `(end + 1).div_ceil(8)`, reject a missing padding byte as
`Truncated`, narrow to `usize`, then validate the marker and padding inside
that prefix.

- `crates/before/src/party/io.rs:26-39`, `Party::decode_prefix` (used by
  `Clock::decode_bytes`, `clock.rs:853-857`).
- `crates/before/src/span/wire.rs:134-142`, the lower endpoint inside
  `Span::decode_bytes`, spelled inline.

`Clock` decoding and `Span` decoding are the same composition (a byte-aligned
prefix, then a second value on the rest), but only the party half has a named
step; the version half repeats the arithmetic inline, with its own `expect`
message.

## Proposed structure

1. In `crates/before/src/bits/storage.rs`, beside `validate_padding`, add one
   helper that owns the arithmetic and the two rejections:

   ```rust
   /// The byte length of the marker-padded value whose live bits end at
   /// `end`, after validating its marker and padding.
   ///
   /// # Errors
   ///
   /// [`Decode::Truncated`] if `bytes` ends before the padding byte the value
   /// owes; [`Decode::TrailingBits`] if the marker or padding is malformed.
   pub(crate) fn padded_len(bytes: &[u8], end: u64) -> Result<usize, Decode>
   ```

   Its body is exactly the current `party/io.rs:28-34`.
2. `Party::decode_prefix` becomes: validate, `padded_len`, slice.
3. Add `Version::decode_prefix(bytes: &Bytes) -> Result<(Self, usize), Decode>`
   in `crates/before/src/version/io.rs`, the mirror of `Party::decode_prefix`.
4. `Span::decode_bytes` opens with
   `let (lo, lo_bytes) = Version::decode_prefix(&buf)?;` and keeps everything
   from `let hi_bytes = ...` unchanged.

Optional, same branch if the reviewer agrees: `Party::decode_bytes` and
`Version::decode_bytes` can become `decode_prefix` plus "the prefix is the
whole buffer, else `TrailingBits`". I checked the class mapping by hand: a
padded length past the buffer is `Truncated` in both spellings; a padded
length short of the buffer is `TrailingBits` in both (whole-buffer
`validate_padding` sees a remainder above eight bits; the prefix spelling sees
either malformed padding or bytes after it). Keep this step only if the diff
stays small.

## Why the result is more obviously correct

The boundary arithmetic, the one place a `u64`/`usize` mix-up or an
off-by-one in the marker bit would live, exists once and is named. `Clock`
and `Span` decoding read as the same composition, `T::decode_prefix` followed
by a second decode, which is what their module docs already say they are.

## Coverage

- Error classes and acceptance: `version/io/tests.rs` (truncation sweeps,
  trailing bits), `borsh_impls/tests.rs` (`flush_cut_*_is_truncated_by_both_decoders`,
  which pin the flush-byte `Truncated` boundary for clocks and spans), and
  `tests/fuzz_seeds.rs`.
- The 32-bit boundary: `wasm32-pins` `version_decode` exercises the
  `u64` byte-count arithmetic through `Version::decode_bytes`; after the
  optional step it would exercise `padded_len` directly.
- Counters: the helper performs the same calls in the same order, so the
  board's `party_decode`, `clock_decode`, and `span_decode` scan and heap cells
  must not move; run `just gate` and confirm the board leg reproduces the
  baseline exactly.
- My explore-branch harness (`crates/before/src/testing/l6_spec.rs` on
  `explore/l6-codecs`) detects a one-bit error in this arithmetic: calibration
  mutation M9 (`(end + 1)` changed to `end`) fails its clock leg immediately.
