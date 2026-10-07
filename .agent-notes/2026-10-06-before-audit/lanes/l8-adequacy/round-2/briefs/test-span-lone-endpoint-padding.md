# Test brief: decoders report a malformed lone prefix as malformed, not truncated

Owner lane: codecs. Class G (a reachable value change no test detects), two
sites with one structure.

## Invariant

When a buffer holds only the first field of a two-field encoding, and that
field's padding has a set bit after the marker, the decoder reports the
buffer as malformed (`Decode::TrailingBits`), not as a prefix awaiting more
input (`Decode::Truncated`). The two fields are a `Span`'s lower endpoint and
a `Clock`'s party.

## Why it matters

The two errors mean different things to a caller assembling a value from a
stream: `Truncated` says "read more", `TrailingBits` says "this can never
decode". Both decoders check that the first field fits the buffer before
validating that field's padding:

- `Span::decode_bytes` (`crates/before/src/span/wire.rs:131-145`):
  `if lo_bytes > buf.len()`. The surviving mutant `>` to `>=` at
  `wire.rs:136` returns `Truncated` whenever the lower endpoint fills the
  buffer, before the padding check can reject it.
- `Party::decode_prefix` (`crates/before/src/party/io.rs:26-39`), reached
  publicly only through `Clock::decode` (`clock.rs:854`):
  `if encoded_bytes > bytes.len() as u64`. The surviving mutant `>` to `>=`
  at `io.rs:29` does the same for a lone party.

The full suite passes under both mutants (`survivors/diffs/span.md`,
`survivors/diffs/party.md`).

## The tests

Form: one unit test per decoder, each pinning one specific input (the defect
concerns one boundary: the first field exactly filling the buffer).

`crates/before/src/span/tests.rs`:

```rust
/// A lone lower endpoint with a set padding bit after its marker is
/// `TrailingBits`; the clean lone endpoint is a truncated span.
#[test]
fn lone_endpoint_with_bad_padding_is_trailing_bits() {
    let mut bytes = crate::Version::new().encode();
    assert_eq!(bytes.len(), 1);
    *bytes.last_mut().unwrap() |= 0x01;
    assert!(matches!(Span::decode(&bytes[..]), Err(crate::error::Decode::TrailingBits)));
    let clean = crate::Version::new().encode();
    assert!(matches!(Span::decode(&clean[..]), Err(crate::error::Decode::Truncated)));
}
```

`crates/before/src/clock/tests.rs`:

```rust
/// A lone party with a set padding bit after its marker is a malformed
/// clock (`TrailingBits`); the clean lone party is a truncated clock.
#[test]
fn lone_party_with_bad_padding_is_trailing_bits() {
    let mut bytes = crate::Party::seed().encode();
    assert_eq!(bytes.len(), 1);
    *bytes.last_mut().unwrap() |= 0x01;
    assert!(matches!(Clock::decode(&bytes[..]), Err(crate::error::Decode::TrailingBits)));
    let clean = crate::Party::seed().encode();
    assert!(matches!(Clock::decode(&clean[..]), Err(crate::error::Decode::Truncated)));
}
```

A property generalizing each is better if cheap: generate the first field
from `arb_oracle_version` or `arb_oracle_party`, encode it, set one of the
padding bits after the marker (skip encodings with no padding bits), and
assert `TrailingBits` for the lone buffer.

## Verified behavior (ox-east-1, probe copy)

Span (`witness/span-*.log`):

- Base: `verdict: Err(TrailingBits)`, the test passes.
- Mutant (`wire.rs:136` `>` to `>=`): `verdict: Err(Truncated)`, the test
  fails.

Clock (`witness/clock-*.log`, the bad-padding half of the test above):

- Base: `verdict: Err(TrailingBits)`, the test passes.
- Mutant (`party/io.rs:29` `>` to `>=`): `verdict: Err(Truncated)`, the test
  fails.

No fix is needed at base; these tests pin the classification.
