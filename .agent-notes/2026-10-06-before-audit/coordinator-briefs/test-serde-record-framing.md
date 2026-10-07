<!-- CAVEAT LECTOR: a test brief written by the coordinator (Claude Opus 5.5) from lane L6's question Q1, after Finch ruled to reject positional arrays. -->

# Test brief: human-readable serde accepts only the record form it emits

## The ruling and the defect

The owner ruled that the human-readable deserializers of `Clock`, `Span`,
and `Ranked` accept only the named-record form they serialize. Binary formats
keep the sequence form they depend on. The ruling is in
`lanes/l6-codecs/round-1/questions.md`, at the end.

Today the three intermediate records `ClockOwned`, `SpanOwned`, and
`RankedOwned` (`crates/before/src/serde_impls.rs:33-77`) use
`#[derive(Deserialize)]`. A derived visitor accepts a sequence as well as a
map, so `serde_json` decodes `["20","e0"]` as `Clock::seed()`. The lane L6
auditor verified this with the probe `probe_json_composites_accept_arrays` on
`explore/l6-codecs`.

These `*Owned` records serve only the human-readable path; the binary path
decodes canonical bytes. Confirm that by reading `serde_impls.rs` before
relying on it.

## Invariant

For every `Clock`, `Span`, and `Ranked`, deserializing JSON that holds the
positional array of the fields `Serialize` emits as a record fails. The record
form still round-trips.

## Form

A property over the crate's existing arbitrary generators for each of the
three types, in `crates/before/src/serde_impls/tests.rs`, beside the existing
serde tests. Use a shared helper, not three copies.

## Construction

For each generated value `v`:

1. Serialize it with `serde_json::to_value` and confirm it is an object. That
   premise check keeps the test from passing vacuously.
2. Build the positional form, a JSON array of the object's field values in
   declaration order:
   - `[party, version]` for `Clock`
   - `[lo, hi]` for `Span`
   - `[version]` for `Ranked`
3. Deserialize both forms.

## Assertions

- The record form deserializes to a value equal to `v`.
- The positional form returns `Err`.

## Expected failure on the base commit

The positional form deserializes `Ok` to a value equal to `v`, for every
generated value. The first failing case shrinks to the smallest value, for
example `Clock::seed()` from `["20","e0"]`.
