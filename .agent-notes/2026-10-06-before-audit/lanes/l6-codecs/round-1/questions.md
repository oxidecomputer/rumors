# L6 questions for the owner

Each question is answerable without the audit conversation. Evidence was
produced on ox-east-1 at `58285ca5` (explore branch `explore/l6-codecs`,
probe module `crates/before/src/testing/l6_probes.rs`, test
`probe_json_composites_accept_arrays` and `probe_cbor_text_and_array_as_bytes`).

## Q1. Should human-readable serde reject the positional-array form of `Clock`, `Span`, and `Ranked`?

**Observed (verified).** `serde_json` accepts a JSON array in place of the
named record that `Serialize` emits:

```
PROBE json clock array ["20","e0"] -> Ok([32, 224])      // == Clock::seed()
PROBE json span array ["a8","b8"] -> Ok([168, 184])
PROBE json ranked array ["a8"] -> Ok([128, 168])
```

**Mechanism.** `ClockOwned`, `SpanOwned`, and `RankedOwned`
(`crates/before/src/serde_impls.rs:36`, `:54`, `:72`) use `#[derive(Deserialize)]`.
A derived struct deserializer implements `visit_seq` as well as `visit_map`,
so every self-describing format that hands it a sequence gets positional
decoding. `deny_unknown_fields` does not affect this.

**Why it matters.** The September triage closed "Make serde deserialize the
same data model it serializes" (`.agent-notes/2026-09-16-before-triage-plan/checklist.md`,
section 03). The serializer emits only the named record
(`serde_impls.rs:23-77`), so the deserializer currently accepts a strictly
larger data model. No value is mis-decoded: an array still goes through
`FromStr` on each component and `Span::new`'s ordering check, so canonical
strictness of the *values* is intact. Only the record framing is lenient.

**Options.**
1. Accept the leniency, and say so in the serde module doc (no behavior change).
2. Reject sequences: replace the three derives with small hand-written
   visitors that implement only `visit_map` (behavior change for inputs no
   serializer of this crate produces; no wire change for anything it emits).

**Recommendation.** Option 2 if the closed fix's goal is the contract; option 1
otherwise. Either way, the decision belongs in the serde module doc.

## Q2. Should binary `Count` deserialization reject a CBOR byte string?

**Observed (verified).**

```
PROBE cbor bytes as count [42, 01, 02] -> Ok("36893488147419103233")
```

The CBOR byte string `h'0102'` decodes as the `Count` whose limbs are
`[1, 2]` (`2·2^64 + 1`). `Serialize` emits a CBOR array of `u64`
(`[1, 2]` is `0x82 0x01 0x02`), so this is a second CBOR spelling for every
count whose limbs are all below 256.

**Mechanism.** `Count`'s binary `Deserialize` calls `deserialize_seq`
(`serde_impls.rs:276`). ciborium answers `deserialize_seq` on a byte string by
visiting its bytes as a sequence, and `next_element::<u64>()` accepts each
`u8`. This is the same bytes-for-sequence bridging the owner already accepted
in the other direction under decision 13 (`serde_bytes` for the byte-encoded
types), so it may well be acceptable.

**Options.** Accept and document; or reject non-`u64` elements with a
visitor that deserializes each element through a `u64`-only seed (ciborium
would still bridge, so this needs a check on the sequence's source type, which
serde does not expose). I found no clean way to reject it without a custom
visitor per format, so my recommendation is to accept it and document that
binary formats may bridge byte strings and sequences in both directions.

**Related, for completeness (no question).** Binary `Party`/`Version`/... accept
a CBOR array of integers in place of a byte string (`cbor array [81, 18, 20] ->
Ok([32])`), which decision 13 ruled in by choosing `serde_bytes`. CBOR text
strings are *rejected* (`invalid type: string, expected byte buffer`), so
`serde_bytes::ByteBuf`'s `visit_str` is never reached through ciborium.

## Q3. Should `Decode::TrailingBits`'s definition cover the rank decoder's all-zero final fraction group?

**Observed (verified by reading; no test needed).** `Decode::TrailingBits` is
defined as "A complete value was followed by bits or bytes that are not part of
its canonical encoding" (`crates/before/src/error.rs:112-119`). `Rank::decode`
also returns it for "an all-zero final fraction group" (documented at
`rank.rs:338`, produced at `rank.rs:682`), which lies *inside* the stream, before the
close bit, and is a non-minimal spelling rather than trailing input. The rank
module doc calls the whole class "not the minimal packing of its content".

**Options.** Widen the variant doc to "bits or bytes that are not part of the
value's minimal packing, inside or after it"; or reclassify the zero group as
`NotCanonical` (a behavior change for one rejection class; no wire change).
**Recommendation:** widen the doc; reclassifying would move a pinned error
class and buys nothing a caller can act on.

## Q4. What should a 32-bit decoder do with a canonical value too large to buffer?

**Context (verified).** `Rank::decode` panics on wasm32 for a canonical rank of
`2^30 + 1` fraction groups (about 1.13 GiB), because an internal `Vec<u8>`
cannot double past `isize::MAX` (defect record
`defect-rank-decode-wasm32-growth.md`). The same input decodes on 64-bit
targets. Several other decoders share the mechanism (fix note).

**The question.** After the panic is removed, which behavior is the contract?

1. **Succeed whenever the target can represent the value.** The demonstrated
   input is `2^-(8·(2^30+1))`, stored as `num = 1`; a decoder that counts
   leading zero groups and assembles the numerator directly decodes it in
   `O(1)` memory. Values whose representation itself exceeds the address
   space still need option 2.
2. **Return a typed error** for "too large for this target". `Decode` is
   `#[non_exhaustive]`, so a new variant (for example `Decode::TooLarge`) is
   source-compatible, but it is a public API addition. Reusing
   `Decode::Io(OutOfMemory)` matches what `std`'s `read_to_end` already
   returns at the same size for the reader-based decoders, but misstates `Io`'s
   documented meaning ("the underlying reader failed").

**Recommendation.** Both: option 1 for the rank's zero groups (it is cheap and
removes an avoidable 1 GiB buffer), and option 2 as the backstop at every
input-proportional growth, with a new variant rather than overloading `Io`.
The test brief accepts either until you rule.

## Owner's ruling on Q1

Be lenient. The human-readable deserializers of `Clock`, `Span`, and
`Ranked` accept both the named record their serializer writes and the
fields in order, as `#[derive(Deserialize)]` provides. Some human-readable
formats write structs positionally by their own choice: `csv` without
headers, and `rmp-serde` in its human-readable tuple mode. A serde visitor
cannot tell which format called it, so rejecting JSON's positional array
would also reject those formats' round trips. The leniency is intended,
and a committed test states it positively.

## Owner's ruling on Q2

Accept and document. Binary formats may bridge byte strings and sequences in
both directions, so a CBOR byte string decodes as `Count` limbs. The serde
module doc states this.

