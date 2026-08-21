# Codec cluster: 23 survivors under `src/tree/mirror/streaming/remote/codec/`

Analysis at branch tip 62447263; every file in this cluster is byte-identical
between the run base 02560b1f and the tip, so the listed line numbers hold.
All claims below marked "verified" come from reading the tip's code and test
files; killing-test designs are proposals, not measurements.

## Orientation

The codec is the streaming wire's byte layer. `signal.rs` packs (stream,
semantic state) into one dense code byte; `frame.rs` holds the semantic
frame vocabulary, the `LeafRun` record container, and `ListingBuilder` — the
one structural gate every child listing from the wire passes; `encode.rs`
renders frames; `decode.rs` and `decode/async_io.rs` are the sync oracle and
the async reader, differentially held together by the suites' `decode_both`;
`greeting.rs` is the V2 greeting's deterministic map spelling; `budget.rs`
prices supply-run batching and enforces it at ingress.

Three shared causes explain every survivor in this cluster (each verified
against the specific tests that should have killed the mutant and did not):

1. **Self-denominated constant oracles.** Tests that assert a derived
   constant's consequences *in terms of the constant itself*
   (`over_ceiling_budgets_saturate_to_the_framing_ceiling` measures
   saturation against `MAX_RUN_BUDGET_BYTES`, admission against
   `SUPPLY_FRAME_OVERHEAD`) move with the mutation and pass vacuously. The
   house style already has the antidote — `default_budget_matches_its_derivation`
   pins `1_830_400` by literal — it just doesn't cover these two constants.
2. **Half-misdial blindness.** Every wire guard of the shape
   `head.major != M || head.value != V` has two single-clause discriminators:
   *right major, wrong value* and *wrong major, right value*. The existing
   malformed-input tests drive only wrong-both shapes (a bare byte string
   where a record belongs; renamed greeting keys), which both clauses reject,
   so the `||`→`&&` legs all survive. Eight survivors are exactly this.
3. **Coincidental value-equivalence in const arithmetic.** `2 + 2 == 2 * 2`
   and `head_len` collapsing nearby values make four mutants compile to the
   identical value: unkillable by any test, dissolvable by refactor.

**Headline recommendation** — one general instrument kills most of family 2
and hardens the whole wire surface: a **decode-canonicity oracle**. The wire
promises one spelling per value; state it as a property: *any input the
decoder accepts re-encodes byte-identically to that input* —
`decode(bytes) = Ok(v)  ⇒  encode(v) == bytes` — driven by a generator that
starts from encoder output and applies head-level misdials (each guarded
position × {wrong-major/right-value, right-major/wrong-value, wrong-both})
plus arbitrary single-byte edits. Run it per surface: greeting item, frame
(via `decode_both`, keeping the sync/async differential), supply run,
record. Every `&&` leg below that silently *accepts* a non-canonical
spelling is killed by the re-encode comparison; every leg that misclassifies
gets killed by the per-position taxonomy asserts of the same family.

---

## budget.rs — the run-budget constants (3 mutants)

### budget.rs:88:5 `+ → *` in `SUPPLY_FRAME_OVERHEAD`

Verified values: `head_len(2)=1`, `WireSignal::MAX_ENCODED_LEN=2`,
`head_len(TAG_CBOR_SEQUENCE=63)=2`, `head_len(u32::MAX)=5`, so the constant
is 10. The mutated leg associates as `((1+2)*2)+5 = 11` — a real value
change, not an equivalent. It survives because every test touching the
boundary (`admission_charges_the_frame_envelope`,
`over_ceiling_budgets_saturate_to_the_framing_ceiling`, and the decode
suite's `frame_wire_size` helper) is denominated in the constant itself,
and because encoder flush and decoder ingress share the constant, a
self-consistent session never observes the shift — only the absolute
boundary against the greeting-negotiated budget number moves by one byte.

**Reachable in principle:** yes — a peer running unmutated code negotiates a
budget and observes the mutated side flushing one byte early / rejecting one
byte tight. Severity: interop boundary drift, not corruption.

**Disposition — refactor (rung 1), with the derivation kept as a test.**
The 6ed8982c precedent (`PAYLOAD_CHUNK_LEN`, the bookmark `MAJOR_*`
constants) applies exactly: make `SUPPLY_FRAME_OVERHEAD` the literal `10`,
and move the head-sum derivation into a committed test asserting the literal
equals the recomputed sum (mutants are not generated in test code, and the
test keeps the derivation from drifting when a head width changes). This
kills the mutant structurally; a literal-pin test alone (rung 4,
`assert_eq!(SUPPLY_FRAME_OVERHEAD, 10)`) also works if the owner prefers the
arithmetic to stay in the code.

### budget.rs:102:59 `- → +` and `- → /` in `MAX_RUN_BUDGET_BYTES`

`u32::MAX as usize - SUPPLY_FRAME_OVERHEAD`. Verified survival cause: the
saturation test asserts `from_bytes(usize::MAX).bytes() == MAX_RUN_BUDGET_BYTES`
and derives its probe body from the same constant — both sides move with the
mutation. Under `+` the ceiling is `u32::MAX + 10`: budgets above the cap
stop saturating, and a peer configured near `usize::MAX` can buffer a >4 GiB
run that then deterministically fails at the run head — precisely the
failure mode the constant's own doc comment exists to prevent. Under `/`
(`u32::MAX / 10`) every budget above ~429 MB silently clamps: safe but a
real behavior change to the public `target_message_size` knob.

**Reachable in principle:** yes, through the public knob at extreme values;
no test drives budgets between ~429 MB and the cap.

**Disposition — killing property (rung 4), two legs, no 4 GiB allocations
needed (the arithmetic is observable through `bytes()` and `covers` without
materializing buffers):**

- *Ceiling identity:* `MAX_RUN_BUDGET_BYTES + SUPPLY_FRAME_OVERHEAD ==
  u32::MAX as usize` — the stated invariant (a saturated whole-frame budget
  exhausts exactly the wire's run-cap range). Kills both legs
  (`u32::MAX+20 ≠ u32::MAX`; `u32::MAX/10 + 10 ≠ u32::MAX`).
- *Identity below the ceiling:* `from_bytes(4_000_000_000).bytes() ==
  4_000_000_000` (a literal chosen inside the gap the `/` leg closes) — keeps
  saturation from ever binding early. Generalizes to a property over
  arbitrary `bytes`: `from_bytes(b).bytes() == b` whenever
  `b + SUPPLY_FRAME_OVERHEAD ≤ u32::MAX as usize`, and the covers/admits
  monotonicity that already exists stays as is.

---

## signal.rs — coincidental const equivalents (4 mutants)

### signal.rs:217:53 `+ → *` in `Signal::QUERY_STATE`

`QUERY_EMPTY_STATE + Flow::STATE_COUNT` is `2 + 2`; the mutation yields
`2 * 2`. **Identical compiled value — no test can ever kill this** (verified:
the neighboring state constants' same-shaped mutants, where the operands
differ, were all caught by `encoding_is_bijective`; only the `2,2` site
survived).

### signal.rs:287:66 `* → +`, 287:82 `- → +`, 287:82 `- → /` in `WireSignal::MAX_ENCODED_LEN`

`head_len(STATE_COUNT * Stream::COUNT - 1)` is `head_len(169) = 2`; the
mutated arguments are 26, 171, and 170 — all in `head_len`'s `24..=0xff`
band, all yielding 2. **All three are identical compiled values.**

**Reachable in principle:** no — the discriminating input class is empty at
the current constants; these are true equivalents.

**Disposition — refactor (rung 1), not roster.** The ladder runs refactor
first, and the 6ed8982c literal-constant precedent fits: make the dense
state constants (`QUERY_EMPTY_STATE`, `QUERY_STATE`, `SUPPLY_STATE`,
`REPLY_END_STATE`, `STREAM_END_STATE`) and `MAX_ENCODED_LEN` plain literals.
The existing `encoding_is_bijective` and `wire_byte_layout_snapshot` already
pin the layout table exhaustively, and a small committed test can keep the
derivations honest (`assert_eq!(MAX_ENCODED_LEN, head_len((STATE_COUNT *
Stream::COUNT - 1) as u64))` — in test code, out of mutation reach). If the
owner prefers keeping the arithmetic in code, the fallback is a line-pinned
roster pair with the coincidence proofs (`2+2 = 2*2`; `head_len` constant on
`{26, 169..=171}`), but roster is rung 3 and refactor is available.

---

## encode.rs:104:62 `+ → *` in `FrameEncoding::to_vec` (1 mutant)

Verified: the sum computes `Vec::with_capacity`'s argument only; the bytes
appended afterward are identical under any capacity. Work-only, every leg —
an allocation-sizing hint on a path that materializes a frame for an
attached observer's one-item view.

**Disposition — refactor (rung 1).** Build the buffer by slice
concatenation (`[head, body_parts...].concat()` or equivalent), which
computes capacity internally and dissolves the hand-maintained length
arithmetic. Cheaper than a roster entry and removes the whole mutable
surface. (A roster entry "performance genre, capacity hint only" is sound
if the explicit-capacity style is preferred.)

---

## frame.rs — the record and listing gates (7 mutants)

### frame.rs:108:9 `LeafRun::eq → true`

`eq` compares the runs' exact bytes. No committed test ever compares two
*unequal* frames or runs, so `true` survives. This one has a meta-hazard
worth naming: `decode_both` and every round-trip test compare decoded
frames with `assert_eq!` — under this mutant those oracles pass vacuously
on the supply arm, so its survival weakens *other* tests' ability to catch
future decode bugs.

**Disposition — killing property (rung 4).** The contract is byte-extensional
equality; state it as a family over generated record lists: for arbitrary
runs `a, b` built by `push`, `(a == b) == (a.as_bytes() == b.as_bytes())`,
with the distinct-lists-are-unequal direction doing the killing (the message
suite's `distinct_payloads_are_unequal` is the house pattern). Kills
`eq → true` and the symmetric `eq → false` in the same stroke.

### frame.rs:116:9 `LeafRun::fmt (Debug) → Ok(Default::default())`

Diagnostics-only gap; the 6ed8982c batch already established the treatment
for exactly this genre (Message, Attachment, Peer Debug impls read their
content). **Disposition — killing test (rung 4):** assert the rendering
names the record count and encoded length for a small constructed run,
beside the eq family above.

### frame.rs:286:32 `|| → &&` in `record_head`

The guard `head.major != MAJOR_TAG || head.value != TAG_CBOR_SEQUENCE`
rejects a non-record front. Verified survival cause:
`malformed_record_heads_are_typed` drives only a wrong-both shape (a bare
byte string, major BSTR value 1, which even the `&&` form rejects). The two
discriminators go untested: a *wrong-valued tag* front (e.g. tag 24 wrapping
a byte string — under `&&` it passes as the embedded-sequence tag, and
`from_encoded` then **accepts a non-canonical run**), and a *non-tag head
carrying value 63* (e.g. uint 63).

**Reachable in principle:** yes — any conformance-buggy peer's run bytes;
under the mutant such runs are silently accepted, so the conformance bug
detector is blinded. Bug-shaped in the conformance register.

**Disposition — killing family (rung 4): the record-front misdial matrix.**
For each front in {tag with wrong value, non-tag with value 63, wrong-both},
`from_encoded` rejects with `NotARecord`. Subsumed by the decode-canonicity
oracle (an accepted tag-24 run re-encodes with tag 63 and differs). The same
family should sweep every `major/value` guard in this file rather than this
one site.

### frame.rs:348:21 (match guard → true) and 348:45 (`&& → ||`) in `parse_record`

The version-atom tag gate. Verified survival cause:
`supplied_record_errors_are_typed`'s "untagged version" case produces
`DecodeLeafError::Version(InvalidData)` under the original *and* under the
mutants (ciborium fails downstream with the same variant and kind), so the
assert cannot separate them. The discriminating inputs are records fronted
by a **wrong-valued tag wrapping an otherwise-valid version atom** — the
original rejects; the mutants consume the bogus tag head and then
successfully parse the version, **accepting the record** — and (for the `||`
leg) a non-tag head whose value equals `VERSION_TAG`.

**Reachable in principle:** yes (malformed record content from a
conformance-buggy peer is exactly what `records()` exists to type).
Bug-shaped in the conformance register: silent acceptance of a non-canonical
record.

**Disposition — killing family (rung 4):** record-content misdial matrix —
generate arbitrary valid `(version, message)` records, then re-front the
content with each misdial shape (tag ≠ `VERSION_TAG` over the same atom;
uint head valued `VERSION_TAG`; no tag) and assert the iterator yields the
version defect. The canonicity oracle covers the acceptance direction.

### frame.rs:466:9 (`value_head → Ok(())`) and 466:37 (`|| → &&`) in `ListingBuilder::value_head`

The listing value gate: a byte string of exactly one digest. Verified: the
module doc names `ListingBuilder` as *the one gate every wire listing
passes* (query frames, the greeting's root fan, all three readers), so this
single gap is wire-wide. Existing listing tests cover order violations,
truncated digest bytes, and widened spellings — never a wrong value head.
Under `Ok(())` or under `&&` with a half-misdial (a byte string of the wrong
declared length; a non-byte-string head declaring 32), the parsers proceed
to consume 32 bytes regardless (`parse_listing_map` splits
`MERKLE_HASH_LEN` unconditionally), desynchronizing the item framing —
crafted inputs are then *accepted* with hashes read from misaligned bytes.

**Reachable in principle:** yes, same conformance register; the misaligned
acceptance makes it the sharpest gap in this file.

**Disposition — killing family (rung 4):** listing misdial matrix at the
gate, driven through all three public entries (async query decode, sync
oracle, `parse_listing_map` for the greeting) or — better — through the
canonicity oracle, which kills every acceptance: corrupt one value head per
generated listing across {wrong major / right length, right major / wrong
length across head widths, wrong both} and assert `ListingIssue::Shape`.

---

## decode.rs / decode/async_io.rs — supply-body gates (3 mutants)

### decode.rs:289:37 `|| → &&` in `run_head`

The supply body's opening gate (shared by both decoders — verified both call
it). Under `&&`, a supply body opening with a *uint 63* head (or a
wrong-valued tag) passes as the embedded-sequence tag; the following byte
string head then parses normally and the frame **decodes successfully** —
a non-canonical frame accepted with no error at all. Existing tests
(`frame_shape_is_enforced`, `malformed_run_structure_is_typed`) never
misdial these two heads.

**Disposition — killing family (rung 4):** supply-opening misdial matrix
through `decode_both` (within budget): for each two-head opening not exactly
(tag 63)(byte string), assert `Malformed(SupplyLength)` with the two
half-misdial shapes present. The canonicity oracle also kills the
acceptance leg (re-encode restores tag 63).

### async_io.rs:193:20 `< → <=` in `AsyncFrameDecoder::supply`

The over-budget short-length rejection: `len < RECORD_TAG_LEN + 1` (= 3).
The boundary case `len == 3` is the *smallest legal lone record* — tag (2
bytes) + zero-length byte-string head (1 byte) — which
`a_zero_length_record_is_structurally_valid` proves acceptable, but only
within budget. Verified survival cause: `overbatched_corners_classify_exactly`
sweeps declared lengths `0..3` (rejections only) and the lone-record
property uses real records (≥ ~10 bytes), so the `len == 3` accept side of
the boundary is never driven over budget. The mutant misrejects exactly that
frame as `OverbatchedRun` under budgets below 13 bytes.

**Reachable in principle:** yes but doubly marginal (needs a sub-13-byte
negotiated budget and a peer shipping an empty-content record, itself a
content-level defect). Boundary-exactness gap, diagnostics register.

**Disposition — killing family (rung 4):** generalize
`oversized_lone_record_still_decodes`'s generator from real records to raw
lone records of content length `0..=k` (the zero-budget corner test already
has the harness): over budget, accept iff `lone_record_spans` holds. The
`len == 3` acceptance kills the leg; the family also re-pins the corner
sweep's rejections.

### async_io.rs:221:41 `|| → &&` in `AsyncFrameDecoder::record_prefix`

Same half-misdial blindness inside the over-budget path: under `&&` a
non-record first head (uint 63; tag with wrong value) is treated as a
record's tag, the classification shifts from `OverbatchedRun` to whatever
`from_encoded` or the span check reports downstream. Diagnostics register
(the frame still fails, mislabeled).

**Disposition — killing family (rung 4):** the over-budget arm of the same
misdial matrix: zero budget, first-head misdials, assert `OverbatchedRun`
specifically (the corner test's existing shapes all open with a proper tag;
add the two half-misdials). Kills this leg alongside decode.rs:289's.

---

## greeting.rs — deterministic-map shape guards (5 mutants)

All five are `|| → &&` on guards of the same two-clause shape; all survive
by half-misdial blindness (verified: `greeting_key_roster_is_exact`
permutes/renames keys — wrong-both; `widened_value_spelling_is_rejected`
covers spelling width, not major/value misdials). Each has a genuine
acceptance-divergence input, so these are conformance-register bug-shaped,
not just taxonomy drift:

- **135:38 (map head):** under `&&`, a map head declaring the *wrong entry
  count* over the correct 7-entry body is accepted (the roster loop ignores
  the declared count — verified: it iterates `KEYS`, not `head.value`), and
  a *non-map* head declaring 7 (e.g. an array head, whose body bytes are
  identical under CBOR's map-as-pairs layout) is accepted. Two non-canonical
  spellings admitted.
- **148:37 (key head):** a text head *lying about its length* over the
  correct key bytes is accepted (`split` uses `key.len()`, not the declared
  value — verified), because the byte cursor never desyncs; only the
  declared length is false.
- **172:44 (version-atom tag):** a wrong-valued tag (or a non-tag head
  valued `VERSION_TAG`) fronting a valid version atom is accepted.
- **194:45 (protocol-magic head):** a text head lying about the magic's
  length over the correct `"rumors"` bytes is accepted (same `split`
  structure as the key guard).
- **276:32 (`read_greeting`'s embedded-item tag):** a non-tag head valued 24
  or a wrong-valued tag opening the greeting item is accepted at the
  transport gate.

**Disposition — killing family (rung 4), one instrument:** the
**greeting canonicity oracle** — for arbitrary greetings, any accepted byte
string re-encodes to itself (`parse_greeting(b) = Ok(g) ⇒
encode_greeting-map(g) == b`, plus the `read_greeting` wrapper level for
276:32), with the generator applying the per-position misdial matrix and
arbitrary single-byte edits to encoder output. Every acceptance divergence
above re-encodes differently and dies. The existing `greetings_round_trip`
covers only the encode→decode direction; this is its decode→encode dual, and
it kills all five legs at once plus the whole class for future keys.

---

## Tally

| Rung | Count | Mutants |
|---|---|---|
| 1 — refactor out of existence | 6 | signal.rs ×4 (literal state/width constants, derivations moved to tests), encode.rs to_vec (concat), budget.rs:88:5 (literal `SUPPLY_FRAME_OVERHEAD` + derivation test) |
| 2 — assert at site | 0 | — |
| 3 — roster exclusion | 0 | (fallback only, if the signal/budget refactors are declined: the four signal.rs legs are provably value-identical) |
| 4 — killing property family | 17 | budget.rs:102:59 ×2 (ceiling identity + identity-below-ceiling), frame.rs ×7 (eq/Debug contract pair; record-front, record-content, and listing-value misdial matrices), decode.rs:289 + async_io.rs ×2 (supply misdial matrix, lone-record boundary family), greeting.rs ×5 (greeting canonicity oracle) |

Three instruments cover all seventeen rung-4 kills: the **decode-canonicity
oracle** (decode∘encode identity on the accepted set, per wire surface), the
**head-misdial matrix generator** feeding it (per guarded position:
right-major/wrong-value, wrong-major/right-value, wrong-both), and the
**lone-record boundary family** (content lengths from zero, over and under
budget). The first two generalize past these mutants: every future
`major/value` guard the wire grows is born covered.

None of the 23 is exploitable under the model of record (authenticated
honest peers); the bug-shaped ones all sit in the conformance-detection
register, where the current suites verifiably accept non-canonical
spellings under the mutations — the detector the violation machinery
promises is what the killing families restore.
