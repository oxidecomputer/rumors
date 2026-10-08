# Test brief: decoders are total on wasm32 past 2^30 buffered bytes

Two cases, one per demonstrated site: `Rank::decode`'s group buffer, and
borsh's stream buffer. They share a root cause and belong on one branch.

Defect record: `defect-rank-decode-wasm32-growth.md` (same directory).
Base: `main` at `58285ca5`.

## Invariant

On a 32-bit target, `Rank::decode` returns, without panicking, for a
canonical rank stream of `2^30 + 1` fraction groups; when it returns `Ok`,
the value is exactly `2^-(8·(2^30+1))`.

## Form

A unit test pinning one boundary coordinate, run in the `wasm32-pins`
executor (`crates/before/wasm32-pins/`), because only a 32-bit `usize`
reaches the failure. It belongs beside
`rank_decode_accepts_the_first_exponent_past_usize` in
`crates/before/wasm32-pins/harness/tests/pins.rs`, with its guest side in
`guest/src/checks.rs` and `guest/src/synthesis.rs` and a new `Check`
variant in `protocol/src/lib.rs`.

## Generator construction (guest side)

Synthesize the input lazily, as a `std::io::Read` with no backing buffer, so
the decoder's own buffers are the only input-proportional allocations (an
in-memory input of 1.13 GiB plus the decoder's 1 GiB buffer and image would
crowd wasm32's 4 GiB and confuse the result with allocator exhaustion).

The canonical stream for `2^-(8g)`, MSB first:

- bit 0: `0` (the integral header for integral part zero);
- for each group `k` in `0..g`: bit `1 + 9k` is `1` (continuation), the next
  eight bits are the group's digits, all `0` except the very last digit of the
  last group, at bit `9g`, which is `1`;
- bit `9g + 1`: `0` (the close bit), then zero padding to the byte boundary.

The total length is `ceil((9g + 2) / 8)` bytes. The existing
`synthesis::fraction` builds the same periodic body with a nine-byte tile;
byte `i` of the body is `(TILE[(i-1) % 9] << 7) | (TILE[i % 9] >> 1)` with
`TILE = [0x80, 0x40, ..., 0x01, 0]`, cut at the close bit, plus the final
digit. One working implementation is `UnitFractionReader` on
`explore/l6-codecs` (`crates/before/wasm32-pins/guest/src/synthesis.rs`,
commit `679cc2e5`), which you may adapt. Validate it as that branch does: for
small `g`, the decode must equal `Rank::decode` of
`synthesis::unit_fraction(8 * g)`.

## Assertion

Guest check `RankDecodeGroups(g)`:
- `Ok(rank)`: return `PASS` after checking `rank` against the expected value
  without materializing a 1 GiB encoding. One cheap exact witness:
  `&rank + &rank` must equal the rank decoded from a lazy stream for
  `2^-(8g - 1)`. That stream is the same shape with the final set digit one
  position earlier, which is also canonical. Return `WrongValue` otherwise.
- `Err(_)`: return `Failure::DecodeRejected`.

Host test, documented as stating the invariant above:

```rust
assert!(
    matches!(
        run(Check::RankDecodeGroups, (1 << 30) + 1, 0),
        Outcome::Passed | Outcome::Failed(Failure::DecodeRejected)
    ),
    "Rank::decode panicked on a canonical rank past 2^30 groups"
);
```

Whether `Failed(DecodeRejected)` stays acceptable depends on the owner's
ruling on question Q4 (succeed for representable values, or return a typed
"too large for this target" error). If Q4 rules "succeed", tighten the
assertion to `Outcome::Passed`.

Add a control at exactly `2^30` groups asserting `Outcome::Passed`, so the pair
pins the boundary from both sides.

## Failure output on the base commit

```
assertion failed: ... Rank::decode panicked on a canonical rank past 2^30 groups
```

with `run` returning `Outcome::Trapped(Trap::UnreachableCodeReached)` after
about two minutes of guest execution on ox-east-1. The `2^30` control passes
on base.

## Second case: borsh `Version` decode past 2^30 bytes

**Invariant.** On a 32-bit target, `Version::deserialize_reader` returns,
without panicking, for a canonical version stream of `2^30 + 5` bytes; when it
returns `Ok`, the version's `encoded_bits()` equals the stream's live length.

**Generator.** A lazy `Read`, with no backing buffer, of the canonical
one-leaf version of height `2^k`: leaf flag `1` at bit 0, `k` zeros, then the
mantissa `1 0^(k-1) 1` (bits `k+1` and `2k+1` set), then the marker at bit
`2k+2`, then zero padding. That is `ceil((2k + 3) / 8)` bytes. Take
`k = 2^32 + 16` for `2^30 + 5` bytes, and the control `k = 2^32 - 2` for
exactly `2^30` bytes. Reference implementation: `WideLeafReader` on
`explore/l6-codecs` (commit `3854b4c1`). The guest must enable `before`'s
`borsh` feature and depend on `borsh` directly; lock `borsh` 1.6.1 and
`cfg_aliases` 0.2.1, the main workspace's versions (commit `431011b3`).

**Assertion.** `run(Check::VersionBorshWideLeaf, (1 << 32) + 16, 0)` must be
`Outcome::Passed`, or `Outcome::Failed(Failure::DecodeRejected)` if Q4 rules
for a typed error. The control at `(1 << 32) - 2` must be `Outcome::Passed`.

**Base output.** `Trapped(UnreachableCodeReached)` after about 140 s; the
control passes after about 150 s.

**Optional third assertion, informational.** `Version::decode` of the same
lazy stream returns `Decode::Io` of kind `OutOfMemory` on base (about 16 s).
Pin it only if Q4 rules that this is the intended error. Otherwise it should
move with the repair.

## Cost

About 1.13 GiB streamed per case and 2 to 2.5 GiB of guest memory at the
`2^30` control; about two minutes each on ox-east-1 under load. The existing
`RankDecode` pin already streams about 600 MB three times. Keep both cases in
separate `#[test]`s so nextest runs them in parallel.
