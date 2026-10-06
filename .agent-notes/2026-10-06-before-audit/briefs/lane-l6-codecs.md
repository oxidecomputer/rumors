<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the lane section for the codec auditor; the model for the other seven lane sections; under review by Finch before launch. -->

# Lane L6: codecs and serialization adapters

## Scope

This lane's theme is *the boundary between values and bytes*: everything that
turns a value into bytes or text and back. Canonical encoding is a strong
promise:

- each value has exactly one byte spelling, so equal bytes mean equal values
- decoders are strict, accepting exactly the canonical forms
- encodings are self-delimiting, so they compose inside larger streams
- the adapters (serde, borsh, text) preserve all of this through their own
  buffering and error paths

Some things to consider, as starting points rather than a checklist. They
are prompts for your own exploration: pursue whatever else the code
suggests, and expect the most valuable questions to be ones not listed
here.

- Does every decoder accept exactly the canonical encodings, and reject
  everything else with the documented error and precedence, however the input
  arrives (chunked, interrupted, or from a failing reader)?
- Does every encoding path (`encode`, `encode_to`, `as_bytes`, serde, borsh)
  produce exactly the canonical bytes?
- Is the bit-level machinery beneath (gamma codes, windows, bit stacks,
  validators, splices) correct at its boundaries, including on 32-bit
  targets and at extreme sizes?
- Do decoders keep their resource promises on hostile input?

The identifiers below are a starting map, not a boundary. Follow the theme
wherever the code takes it, including into code this list does not name.

- **Byte codecs:** `encode`, `encode_to`, `decode`, `as_bytes`, and
  `encoded_bits` on `Party`, `Version`, and `Clock`; `encode`, `encode_to`,
  and `decode` on `Rank`, `Ranked`, and `Span`; and the byte mechanics of
  `encode_rank` on `Version` and `Ranked`.
- **Text forms:** the hexadecimal `Display`/`FromStr` of `Party` and
  `Version`.
- **Adapters:** serde through postcard (binary), CBOR via ciborium
  (self-describing binary), and JSON (human-readable); borsh, including
  `Count`'s limb sequence and the one-byte-at-a-time stream reader.
- **Implementation:** `src/bits/`, `src/version/io/`, `src/party/io/`,
  `src/span/wire.rs`, `src/text.rs`, `src/serde_impls.rs`,
  `src/borsh_impls.rs`, `src/error.rs`.
- **32-bit behavior** of all of the above, currently instrumented by the
  `wasm32-pins` workspace.

Neighboring lanes explore adjacent themes. When your work crosses into one,
establish what you found and report it, rather than continuing the
investigation there. L2 explores Version algebra, L4 the ordering semantics of
rank encodings and `Rank`'s text, L5 Span semantics, and L7 `suanpan`.

## Contracts to read first

- The crate page's "Replicating clocks between processes" section and the
  canonical-bytes promise in its quickstart: one byte spelling per value, so
  equal bytes mean equal values.
- Every `encode*`/`decode` rustdoc, including `# Errors` and `# Complexity`.
- `error::Decode`: its four variants and the precedence the docs state between
  `Truncated` and `TrailingBits`.
- The module docs of `bits`, `version::io`, `party::io`, `span::wire`,
  `serde_impls`, and `borsh_impls`. They state invariants such as "decoding
  validates the complete stream before adopting its bytes" and "the encodings
  are self-delimiting".
- The crate page's claim that transient memory stays a small constant multiple
  of the input size, which applies to every decoder.

## Prior coverage to map

- **Tests:** `src/version/io/tests.rs`, `src/party/io/writer/tests.rs`,
  `src/bits/tests.rs`, `src/bits/reader/tests.rs`,
  `src/bits/stack/*/tests.rs`, `src/serde_impls/tests.rs`,
  `src/borsh_impls/tests.rs`, `src/text/tests.rs`, `tests/fuzz_seeds.rs`,
  `tests/representation_space.rs`, and the board's codec families.
- **The 32-bit checks in `crates/before/wasm32-pins/guest/src/checks.rs`.**
- **The September review's partition reports**, as a map of examined ground
  (line numbers are stale):
  `.agent-notes/2026-09-01-holistic-review-before/evidence/partitions/`
  `codec-bits.md`, `codec-base-text-tree.md`, and `skyline-coding.md`; and
  section 03 of
  `.agent-notes/2026-09-16-before-triage-plan/checklist.md`.

## Closed fixes to re-attack

- The span decode's rejection of a collapsible pair in the middle of the
  stream, which once had no witness through `Span::decode`.
- The reconciliation of marker padding, truncation, trailing input, and error
  precedence across the id, version, rank, clock, span, serde, and borsh
  entries.
- Serde deserializing exactly the data model it serializes.
- Serde and borsh exercised through their real buffering and ownership paths
  on wide-leaf and deep-topology inputs.
- `Rank::decode` reading from `Read` without retaining a copy of its input.

## Candidate leads (evaluate, don't confirm)

Each lead is an unverified hypothesis. Dispute it with evidence where it is
wrong.

1. **Error precedence across entry points.** An input that is simultaneously
   truncated and non-canonical may be classified differently by the raw,
   borsh, and serde entries. Check whether any in-tree property compares
   rejection classes across entry points.
2. **Readers that misbehave without failing.** A `Read` that returns short
   reads, `ErrorKind::Interrupted`, or an error on call *k*. `read_exact`
   retries `Interrupted`, but a hand-written loop (for example in the borsh
   stream reader) may not. The decoders' results should not depend on how the
   input is chunked.
3. **Writers that fail midway.** What does each `encode_to` promise when its
   writer fails partway through? And do the serde and borsh serializers
   surface the same failure?
4. **One-constraint violations.** For each canonical-form condition, take a
   valid encoding of a rich value and violate exactly that condition at every
   reachable position. Every public decoder must reject it, with the
   documented error class.
5. **Byte-count arithmetic at the extremes.** Expressions such as
   `(end + 1).div_ceil(8)` in `Party::decode_prefix`, and every `u64` to
   `usize` conversion, at the edges of the integer types, especially on
   32-bit targets.
6. **Agreement between text and serde forms.** The human-readable serde forms
   should agree with `FromStr` and `Display` for every value, and both should
   reject the same malformed inputs.
