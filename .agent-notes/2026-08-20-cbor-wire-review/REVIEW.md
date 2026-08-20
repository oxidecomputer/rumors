# Adversarial review: the CBOR-legible wire stack (rumors#35)

Reviewed: `review/cbor-wire` @ `a5c4ca1a9b7f836f3b47d2fad4ae5d1a1fc27834`
(base `15bd905e`, 11 commits + 1 merge), plus `w2/tracing-adapter` @
`526cbf87`. Method: the charter's four rounds — full read of the codec
stack, the instruments, the seams, and constructed assumption checks —
file by file over every non-snapshot change. Every claim below is marked
**verified** (I ran or constructed it) or **assessed** (read only).
Constructed witnesses were added as temporary scratch tests, run, and
removed; the worktree is clean, `REVIEW.md` excepted (it sat at the
review SHA through the review and is re-anchored at `d05a6d03` since
the rebase note below).

Each finding carries a **Resolution** block written to be executable
without this review's context: file, mechanism, and acceptance criteria.
Items that change public API or gate policy are marked **owner-gated**
and must go to Finch before landing; everything else is executable
directly in the implementing worktree. Line numbers cite `a5c4ca1a`
for files the post-review rebase left untouched (byte-identical at
`d05a6d03`) and are restated at `d05a6d03` anchors where it moved them;
see the rebase note below.

Verification I ran: the three dispute-wire calibration cells (with
measured values), 605 tests across the lib and the affected integration
suites (observe, wire_legibility, handshake, all three snapshot suites,
target_message_size, decode_alloc) — all green; `tools/digestshare`
(figures match the design doc exactly); a per-commit
`cargo check --all-targets --all-features` bisection sweep over all 12
commits; the gate's `clippy-default` rumors leg; `cargo check` of the
adapter crate at the branch tip; three constructed witnesses (below);
and a one-off orphan-snapshot sweep.

**Rebase note.** After this review froze, main merged PR #37 (internal
height- and value-erasure), and the branch was rebased onto that merge
(`991f4663`) as `d05a6d03` — same 12-commit topology; the tip is
gate-clean per the rebasing agent's report (told, not re-verified
here), and a fresh per-commit check sweep over the rebased range is
recorded at seed 2. Every referent in this packet was re-checked
against the rebased tree. Files the erasure left untouched carry over
exactly (capture.rs, handshake.rs, observe.rs, error.rs, cbor.rs,
link.rs, the bookmark tree, tests/observe.rs, tests/dispute_wire.rs);
the erasure-moved referents are restated in place at their `d05a6d03`
anchors — B1's call-site sweep, B2a's witness (`Message` and `LeafRun`
are no longer generic; `records` takes the peer-minted deserializer),
and the depth-limit spec's step 3, which the erasure *simplified*:
both dialects' payload ingress now funnel through one minted
`PayloadDeserializer` (`message.rs`). The rebasing agent's judgment
calls were reviewed: dropping the leaf-level trailing-bytes variant in
favor of the deserializer's exactly-one-value `InvalidData` is
consistent with `from_slice`'s documented contract (if R5's
typed-error extension is ruled in, this class is a candidate to ride
along); the observe-hook re-threading through the erased
`Reconciliation` funnel is assessed from instrument evidence, not
line-read — the wire-byte differential, the capture-complementarity
suites, and tests/observe.rs are unchanged and green at the tip, and
those are the instruments a threading mistake would trip. **Dispatch
sequencing**: the rebased commits are unsigned (the signing agent was
locked during the rebase); re-sign and force-push *first* — the
re-sign rewrites every SHA — and only then branch fix work off the
settled tip. Commit SHAs named in this packet's dispositions
(`186037d9`, `0db57d00`, `5367a82e`, and the rest) are the *reviewed*
history, preserved at the local ref `backup/cbor-wire-pre-37`; the
findings themselves are restated against the rebased files, so an
executor never needs those commits.

**Post-rebase rulings (Finch, review follow-up).** Every owner-gated
question this packet posed is now ruled, each per this review's
recommendation; executors treat these as decided:

- **B2**: enforcement alternative *declined* — the prose rescope plus
  boundary-pinning witnesses is the resolution of record.
- **R2**: keep-and-document; do *not* delete `NetworkTruncated`.
- **R3**: re-export `HeadError`; it stays exhaustive (no
  `#[non_exhaustive]`).
- **R5**: the typed-error ruling *extends* to the identity hand-off
  and the greeting — implement the sized-M shape in the resolution.
- **`SessionKind`**: add `#[non_exhaustive]` now, pre-release.
- **R6 step 4**: add the bare-lib clippy gate leg (one justfile line).
- **Seed 13**: the pairing test (`tests/snapshot_liveness.rs`), not
  the cargo-insta runner.
- **digestshare**: gate-wire it (justfile recipe in the lint tier).
- **Depth-limit feature**: the minted-codec shape is approved (ruling
  recorded at step 5 of the implementation spec).

**Rebase audit trail.** The rebasing agent disclosed ten finer-grained
judgment calls; each is dispositioned here. Items marked *seed* go
into the adversarial review rounds as dispute-don't-confirm seeds:

1. Assertion-strength choices in decode/tests.rs
   (`a_zero_length_record_is_structurally_valid`,
   `supplied_record_errors_are_typed`) — *seed*: re-derive the error
   paths from `parse_record` (frame.rs:326–369) independently; the
   agent's reasoning is verified only by the tests passing.
2. Testdoc accuracy on stitched test bodies (decode/tests.rs and
   error_atlas.rs especially) — *seed*: the gate's testdoc leg checks
   that doc comments exist, not that they are still true; re-read each
   stitched test's stated invariant against its merged body.
3. Payload-type anchoring: erased `Message::new(0)` would infer `i32`
   where `LeafRun::<u64>` used to anchor `u64`; every such literal was
   made explicit (`0u64`) rather than relying on the encodings
   coinciding — recorded; the change is strictly toward explicitness.
4. `pushed_runs_validate_and_iterate` (frame/tests.rs) reads payloads
   back through `records(Message::deserializer::<u64>())` +
   `.arc::<u64>()` — rebase-authored code, neither parent's — *seed*:
   review as new code (`arc` is the checked-downcast panic path,
   acceptable in a test).
5. capture.rs adopted wholesale from `0db57d00` — **discharged by
   verification**: the file at `d05a6d03` is byte-identical to
   `0db57d00`'s *and* to `a5c4ca1a`'s (the file this review read), and
   the post-rebase sweep compiles it under
   `--all-targets --all-features`. B1's anchors are exact.
6. Turbofish removals in `remote/` were regex substitutions, not
   eyeball edits — recorded; the compiler and clippy backstop, and
   reviewers eyeball the sites they visit anyway.
7. Observe-threading placement in gossip.rs (`observe.begin` after the
   deserializer mint, before the preamble; `bootstrap_v2` gained a
   trailing `observe` parameter; V1 paths deliberately get none) —
   *seed*: derive the required placement independently from the hook
   contract (begin-before-first-byte; election-before-data; the V1
   exclusion) and judge the code against the derivation, not the
   agent's argument.
8. The keep-both merge heuristic once produced syntactically-plausible
   garbage (duplicated `connected` return lines; compiler-caught) —
   *seed as a class*: hunt for merge duplicates that DO compile —
   duplicated match arms, repeated doc paragraphs, double writes.
9. Mid-branch bisectability was unverified by the rebase (tree-level
   hop proof only; intermediate commits never individually compiled;
   `fa6cbc26` committed from git's staged auto-resolution) —
   **partially discharged**: this review's post-rebase sweep
   check-verifies all 12 commits; full-gate verification remains
   tip-only, matching the lane's original endpoint-gating disclosure.
   `fa6cbc26`'s diff is small enough to eyeball in review.
10. Dead-import removals (`DeserializeOwned`/`PhantomData`/`std::io`
    in streams.rs/start.rs) were grep-justified — recorded; clippy's
    clean pass corroborates.

---

## Chase list: the functional axis

Every finding below, re-sorted by what is actually at stake — so the
set of "bugs to chase" is exact. The tiering in the findings sections
is by review severity (claims violated, discipline breached); this list
is by functional exposure.

- **Functional defects in shipped code introduced by this stack: none
  found.** Across all four rounds, nothing surfaced where a production
  peer, replica, bookmark, or observer reaches wrong state, wrong
  bytes, a panic, or divergence. The shipped-code items in this packet
  are diagnostic-fidelity and API-shape concerns only. (One *pre-
  existing* functional edge — an undocumented payload nesting-depth
  limit, asymmetric between encode and decode — was surfaced by B1's
  user-chosen-`T` analysis and is dispositioned in the out-of-range
  observation at the end of this document; it exists at the base
  commit too and is not this stack's.)
- **Demonstrated failure, test instrumentation only: B1.** A real,
  constructed crash (SIGABRT), but in the `test-internals` renderer; no
  committed input reaches it, and the trigger is a payload a test
  author would have to choose. What makes it chase-worthy is the crash
  demonstration plus the false in-tree bound claim plus the hard rule's
  standing demand for the property. This is the entire "bug" chase
  list, strictly construed.
- **Instrument that can silently mask a real regression: R1.** The
  minimal calibration cell's ±2 band is the one place found where a
  genuine (small-record wire cost) regression passes green. The
  standing gaps in the residuals section (no orphan-snapshot guard;
  digestshare liveness unwired) are the same species but only fail to
  *alert*, they never assert falsely.
- **Loud-when-bitten latency: R4's test hardcode.** At epoch ≥ 24 it
  panics with a missing-handler message — it cannot pass wrongly — and
  no current suite reaches that epoch. Annoyance deferred, not a mask.
- **Prose-contradicts-code / discipline-only, no functional exposure:
  B2** (reclassified; constructions real, stakes documentary),
  **R2** (all three arms unreachable by arithmetic; the fabricated
  diagnostic can never be emitted), **R4's doc phrase, R6, seed 3's
  `declared` nit, S1, S2, S4.**
- **API-shape and ruling questions, owner's call: R3** (unnameable
  `HeadError` — ergonomics and rustdoc integrity, nothing functional),
  **R5, `SessionKind`'s openness, R6's gate-leg proposal.**

## Findings, by severity

### Bugs

**B1. The capture renderer recurses without bound on input-controlled
nesting: a crafted (or merely unlucky) application payload overflows the
stack.** — **verified by construction.**

`MAX_DEPTH = 64` bounds one `parse_node` tree
([capture.rs:327](file:///Users/oxide/src/rumors-review/src/tree/mirror/streaming/remote/codec/capture.rs)),
but `render_embedded_as` re-parses each embedded byte string (tag 24/63)
**at depth 0** (capture.rs:594) and unfolds it through
`render_node → render_tag → render_embedded_as` — one Rust stack frame
chain per embedding level, with no budget that survives the byte-string
boundary. Failure scenario, constructed: a chain of
`24(bstr(24(bstr(…))))` 200,000 levels deep (≈1.4 MB of bytes, buildable
outside-in in one pass) fed to `render_item` → **SIGABRT, "has
overflowed its stack"** (reproduced under nextest). A supply record's
payload is the application's own CBOR, so any snapshot/test corpus whose
payload nests embedded-CBOR tags reaches this through the ordinary
harness path (`render_hook_capture → render_frame → supply run → record
items`).

This contradicts three in-tree claims: the hard rule ("no traversal
recurses on input-controlled depth"), the module doc's depth-bound
fallback claim (capture.rs:24–26), and the committed test
`nesting_past_the_depth_bound_falls_back`, which exercises only
`parse_node` — exactly the path that *is* bounded — leaving the render
recursion untested.

Functional-axis statement, to be exact about what is at stake: the
surface is `test`/`test-internals`-gated (never in a shipped artifact),
no committed corpus nests deeply enough to trigger it (the suites'
payloads are flat byte strings and `u64`s), and the trigger is a payload
a test author would have to write — so no shipped functionality and no
current suite is exposed. When it does fire it fires loudly (an abort,
never a wrong render accepted). What keeps it in the bug tier rather
than the prose tier: unlike B2, the violated claim comes with a
demonstrated behavioral failure — a harness that dies on legal input —
and the hard rule demands the bounded-traversal property with a
committed stress test, which does not exist.

Can a user-chosen payload type `T` reach this class in *production*?
**No** — verified along the whole chain:

- Every production parse of payload structure delegates to
  `ciborium::de::from_reader` (all call sites swept, restated at
  `d05a6d03`: the peer-minted `PayloadDeserializer` — the one funnel
  both dialects' wire ingress passes through — plus the typed
  rehydrators and V1's outer byte-string unwrap,
  `message.rs:113/156/178/220`; the version atoms, `tree/wire.rs:205`
  and `frame.rs:361`; `bookmark/format.rs:464`), and `from_reader`
  constructs its
  deserializer with a hard recursion cap of 256 scopes covering array,
  map, and tag descent alike (verified in ciborium-0.2.2 source:
  `recurse: 256`, `Error::RecursionLimitExceeded` at zero). Peer bytes
  nested past the cap yield a typed decode error and a clean session
  abort, never unbounded recursion — whatever `T` is, including
  `ciborium::Value` itself.
- No shipped code hand-walks CBOR structure recursively: the only
  self-recursive walkers in the tree are this renderer (test-gated) and
  the wire-legibility test binary. Tag-24/63 embedded byte strings are
  never *unfolded* in production — they parse as one flat
  `Tag(Bytes)` node — so B1's reset-across-the-boundary mechanism has
  no production analogue.
- The tracing adapter (the production-facing consumer) parses through
  the same capped `from_reader` and renders under its own shrinking
  budgets; worst-case stack is a few hundred frames by construction.
- The encode side recurses only over the user's *own in-memory value*
  via their `Serialize` impl — recursion that value's construction and
  `Drop` already entail; serde's ownership, not this crate's.
- The one inheritance path: a user who opts into the unstable,
  doc-hidden `test-internals` feature and drives `render_hook_capture`
  on their own captures inherits B1 as stated.

This analysis surfaced one adjacent *functional* edge that is out of
this review's range (it predates the stack — payloads were already
CBOR before it): the ciborium decode cap is asymmetric with encode. See
the out-of-range observation at the end of this document. Contrast: the
tracing adapter got this right — its `UNFOLD_BUDGET` decrements across
embedded boundaries (rumors-tracing `render.rs`), and `ciborium`'s own
recursion limit (256, verified in ciborium-0.2.2 source) bounds its
parse.

> **Resolution** (directly executable, in
> `src/tree/mirror/streaming/remote/codec/capture.rs`):
>
> 1. Thread one combined depth counter through the whole render walk, so
>    structural descent and embedded re-parses draw on a single
>    `MAX_DEPTH` budget:
>    - Add a `depth: usize` parameter to `render_node`, `render_tag`,
>      `render_listing`, `render_embedded`, and `render_embedded_as`.
>    - Entry points pass `0`: the `render_item` call (capture.rs:283–290)
>      and `render_frame`'s body loop (capture.rs:269–278). These already
>      call `parse_node(&mut rest, 0)`; keep that.
>    - `render_node` passes `depth + 1` wherever it recurses (array
>      items, map values, tag content, and the nested-value fallthroughs
>      in `render_listing`); `render_tag` passes its `depth` through
>      unchanged (it adds no structural level of its own).
>    - `render_embedded_as` replaces both of its `parse_node(&mut rest, 0)`
>      calls with `parse_node(&mut rest, depth)`, and renders the parsed
>      items at `depth + 1`.
>    - `render_node` additionally checks `depth >= MAX_DEPTH` at entry
>      and, when exceeded, emits the existing hex fallback
>      (`fallback(...)`) — this guards the path where a tree parsed just
>      under the bound is rendered from a deeper starting point. Note the
>      fallback needs the node's exact bytes; the cheapest correct shape
>      is to check the budget *before* parsing (i.e. rely on
>      `parse_node(depth)` erroring and the caller's existing fallback),
>      which the changes above already achieve — the `render_node` entry
>      check is then only needed if any call site renders a node at a
>      depth greater than the depth it was parsed at; with the wiring
>      above no such site remains, so the invariant to keep is: **every
>      `render_*` call site passes a depth ≤ the depth its node was
>      parsed at**. State that invariant in a comment on `render_node`.
>    - The combined bound also caps the per-level `format!("{indent}  ")`
>      growth (≤ MAX_DEPTH levels × 2 spaces), so no separate fix is
>      needed there.
> 2. Update the module doc (capture.rs:22–27): the depth-bound sentence
>    must state that the bound spans embedded-byte-string unfolds, not
>    only one parsed tree.
> 3. Commit two tests beside `nesting_past_the_depth_bound_falls_back`
>    (capture/tests.rs):
>    - *Deep embedded chain falls back, never overflows*: build a
>      tag-24 chain ≥ 10 × MAX_DEPTH levels deep, outside-in (compute
>      each level's length first: `len[0] = 1` for a `0x00` innermost
>      uint; `len[i+1] = 2 + cbor::head_len(len[i] as u64) + len[i]`;
>      then write tag-24 head + bstr head from outermost to innermost,
>      ending with `0x00`). Call `render_item`; assert it returns and
>      the output contains the depth-fallback marker text and an
>      `h'…'` hex line (the injectivity fallback).
>    - Same shape through the frame path: wrap the chain as a supply
>      record payload and drive `render_frame`; assert no panic/overflow
>      and a fallback line. (This pins the harness-reachable path, not
>      just the helper.)
> 4. Acceptance: both new tests pass under nextest; the previously
>    constructed 200k-level witness (same construction, larger N) no
>    longer aborts; the full capture/tests suite and the three snapshot
>    suites re-run byte-identical (`cargo insta` must report **no**
>    snapshot changes — this fix must not move any accepted render,
>    because no committed corpus nests past the bound).

### Contract–prose mismatches (constructions verified; no functional impact)

**B2. The "deterministic encoding enforced on ingress" claim is false at
two layers where decoding delegates to ciborium: non-shortest-form (and
indefinite-length) spellings are accepted.** — **constructions verified,
both; the functional analysis is assessed from the code.**
*Reclassified from the bug tier: no crate functionality rests on
ingress spelling enforcement (analysis below), so what survives is a
prose-vs-code mismatch — the tree claims an ingress guarantee it does
not perform — plus an inconsistent, unstated enforcement boundary.*

- *B2a — record version atom*: `parse_record`
  ([frame.rs:326–369](file:///Users/oxide/src/rumors-review/src/tree/mirror/streaming/remote/codec/frame.rs))
  reads the version-atom *tag* through the canonical head grammar, then
  hands the version byte string to `ciborium::de::from_reader` (and the
  payload to the peer-minted deserializer, itself a plain
  `from_reader`), both of which accept widened and indefinite-length
  heads. Constructed (at `a5c4ca1a`; the parse structure is unchanged
  at `d05a6d03`): a record whose version bstr head is spelled
  `0x59 0x00 len` instead of the canonical short head decodes `Ok`
  through `LeafRun::from_encoded` + `records(...)`. This contradicts
  the codec module doc ("decoding rejects any other spelling",
  codec.rs:8) and `LeafRun`'s own doc ("a records iterator therefore
  never fails structurally, only on a record's canonical content",
  frame.rs:76–78).
- *B2b — bookmark payload*: `unframe` enforces shortest-form heads on
  the frame, but `walk`
  ([format.rs:462](file:///Users/oxide/src/rumors-review/src/bookmark/format.rs))
  parses the payload with `ciborium::Value`. Constructed:
  `decode(&frame(&[0xbf, 0xff]))` — the record map spelled as an
  *indefinite-length* map, a spelling this codec never writes, with a
  correct hash over those bytes — decodes `Ok` to the empty record. This
  contradicts format.rs:39–42 ("the decoder rejects any other
  spelling").

Why no functionality rests on this — what each half of the contract
actually carries:

- **Encoder-side determinism (intact) is what licenses the byte-pinning
  snapshot discipline**: equal semantics ⇒ equal bytes. Ingress plays no
  part in that, and the renderer's injectivity argument is likewise
  self-contained (its own walk falls back to exact hex on non-canonical
  heads).
- **Replica correctness is spelling-independent.** No wire framing byte
  is stored or hashed: payload bytes propagate verbatim from the
  originator to every replica (the decode adapter retains the exact
  slice; re-supply writes it back unchanged), so message identity sees
  one byte sequence per message regardless of the framing around it.
  Version and party atoms are decoded to values and re-encoded
  canonically from the value on every hop — `Version::decode` enforces
  before's bit-level coding, which *is* semantics-bearing and *is*
  enforced — so a widened CBOR head around an atom does not survive one
  hop, and cannot perturb paths, hashes, or convergence.
- **What ingress structurally must reject, it still rejects.**
  Indefinite-length and reserved heads are unparseable by the
  exact-read/O(1)-skip machinery and die wherever the protocol
  hand-parses; listing order is enforced as the protocol walk's own
  invariant. Both predate and stand apart from the CBOR determinism
  contract, and both are intact.
- **What remains for ingress spelling enforcement is conformance
  diagnosis only** — catching a hypothetical third-party encoder that
  emits widened spellings — and per the model of record that machinery
  is a bug detector, never a boundary. There is also a
  circular-justification argument against building it: this crate's own
  encoder cannot produce the spellings such a check would catch, so
  against the only implementation that exists the detector never fires.

The residue whichever way one leans: the enforcement boundary is
currently *inconsistent* — frames, signals, listings, record framing,
greeting, preamble, and the bookmark frame all reject non-shortest
spellings; the record's version-atom head and the bookmark payload do
not — and no prose states where the boundary sits. The tree must say
what IS.

> **Resolution** (primary; directly executable; docs plus acceptance
> pins, zero behavior change):
>
> 1. Re-scope the three claim sites to state the actual contract —
>    determinism is the *encoder's* promise; ingress promises structure
>    and definite lengths, with spelling additionally judged only where
>    the codec hand-parses:
>    - `src/tree/mirror/streaming/remote/codec.rs:6–10`: replace the
>      "and decoding rejects any other spelling" clause with words to
>      the effect of: "The wire is *emitted* as deterministic-encoding
>      CBOR — shortest-form heads, definite lengths, one spelling per
>      value — which is what keeps the byte-pinning snapshot discipline
>      meaningful. Ingress validates structure everywhere; every head
>      the codec hand-parses additionally rejects indefinite lengths
>      and non-shortest spellings, while a record's version atom and
>      application payload are decoded by a general CBOR reader that
>      judges neither — the atom's *content* canonicality is enforced
>      by its own strict decoder." **(Amended after the round-1 fix
>      review: this resolution's first wording claimed "definite
>      lengths everywhere", which the general-reader positions refute
>      by construction — an indefinite-length payload map and an
>      indefinite-length version-atom byte string both decode Ok. The
>      fix round's F1 corrects the transcribed sentence and pins both
>      indefinite witnesses. Any roster of hand-parsed positions must
>      be complete — preamble and greeting included — or omitted.)**
>    - `frame.rs` `LeafRun`/`records` doc (:73–78): replace "only on a
>      record's canonical content" with "only on a record's content —
>      a version atom whose content bytes fail the strict `Version`
>      decoder (its CBOR byte-string head is read by a general CBOR
>      parser and not re-judged for shortest form), or an application
>      payload that does not decode".
>    - `src/bookmark/format.rs:39–42`: replace "and the decoder rejects
>      any other spelling" with "the frame's own heads are rejected in
>      any other spelling; the embedded payload is decoded by a general
>      CBOR reader, so its one-spelling property is the encoder's
>      (equal records produce equal files), not an ingress check".
> 2. Pin the boundary so it cannot drift silently in either direction —
>    commit this review's witnesses as *acceptance* tests whose doc
>    comments cite the scoped contract:
>    - In `src/tree/mirror/streaming/remote/codec/decode/tests.rs`,
>      beside `supplied_record_errors_are_typed`, using its
>      `raw_record` helper: build record content = version tag head
>      (`cbor::write_head(MAJOR_TAG, crate::tags::VERSION_TAG)`), then
>      the widened byte-string head `0x59 0x00 <len>` followed by the
>      canonical atom bytes of `Version::new()` (obtain them by
>      ciborium-serializing `Version::new()` and stripping its first
>      byte, whose low bits are the length), then a ciborium-serialized
>      `0u64` payload. Assert `LeafRun::from_encoded(raw_record(
>      &content)).unwrap().records(Message::deserializer::<u64>())
>      .next().unwrap()` is `Ok` (the minted deserializer is how every
>      test in that file drives `records` at `d05a6d03`). Doc
>      comment: "Pins the stated ingress boundary: the version atom's
>      CBOR head is not spelling-judged (the atom's content is, by
>      `Version::decode`); flipping this to rejection is a deliberate
>      contract change, not drift."
>    - In `src/bookmark/format/tests.rs`:
>      `decode(&frame(&[0xbf, 0xff]))` is `Ok` and empty — same doc
>      pattern ("the payload's spelling is not ingress-judged; the
>      hash binds bytes, the frame binds shape").
> 3. Acceptance: all suites green; **zero** snapshot movement;
>    `grep -rn "any other spelling" src/` returns nothing.
>
> **Alternative (owner-gated): enforce a uniform ingress boundary
> instead.** The case for it is conformance diagnosis with precedent —
> the signal-redundancy ruling paid recurring wire bytes to keep the
> `Mislabeled` detector — and the case against is stated above (the
> detector would never fire against any existing implementation). If
> Finch rules for enforcement, record the ruling in
> `design/cbor-legible-wire.md`'s decision record and implement:
>
> - *B2a, in `frame.rs` `parse_record`*: replace the ciborium parse of
>   the version with the hand-parse the greeting already uses
>   (greeting.rs:152–174 is the model): after the version-tag check,
>   `cbor::read_head` again; require `MAJOR_BSTR` (else
>   `DecodeLeafError::Version(InvalidData)`); `HeadError::Truncated` →
>   the existing `UnexpectedEof` version error, other `HeadError`s →
>   `InvalidData` with the head error's Display; `usize::try_from` the
>   length, split the atom off `input` (shortfall → `UnexpectedEof`),
>   decode via `Version::decode` with the error mapping `decode_party`
>   uses (party.rs:114–126). The payload keeps its minted-deserializer
>   parse. The
>   step-2 witnesses above then invert into rejection assertions
>   (`Err(DecodeLeafError::Version)`, `InvalidData`), plus a canonical
>   control case asserting `Ok` against over-tightening.
> - *B2b, in `src/bookmark/format.rs`*: re-encode-and-compare at the
>   trust boundary: extract `encode`'s map-building +
>   `ciborium::ser::into_writer` body into a private
>   `fn record_payload(record) -> Vec<u8>` (one spelling authority,
>   called by `encode` too); in `decode`, after `walk` succeeds,
>   compare `record_payload(&record)` to the payload slice; mismatch →
>   new `RecordDefect::NonCanonical` variant (additive; the enum is
>   `#[non_exhaustive]`), docstring "the payload decodes, but not from
>   the one spelling this codec writes". This closes widened heads,
>   indefinite lengths, and key order in one total check. Witnesses
>   invert to rejection; add a widened-map-head case (re-spell
>   `sample_record()`'s payload head `0xa1` as `0xb8 0x01`, re-frame).
> - Acceptance either way: zero snapshot movement (encoders untouched);
>   `wire_legibility`, `observe`, and the corruption/truncation sweeps
>   green.

### Risks

**R1 (seed 4). The minimal-record calibration cell sits exactly at the
±2 tolerance edge, and the band's stated rationale is wrong.** —
**verified** (ran the cells: minimal implied **50** vs expected **52**;
mid 107/107 and design 215/215 exact).
`TOLERANCE_BYTES`'s doc
([dispute_wire.rs:99–101](file:///Users/oxide/src/rumors-review/tests/dispute_wire.rs))
says the counts are deterministic and the slack "only absorbs
integer-division adjacency"; window.rs:215–218 correctly documents the
2 B residual as *systematic* (denser batching at small records). The two
rationales contradict, and the operational effect is real: a genuine
+1..+4 B per-message regression confined to small records moves the
implied value to 51..54 and stays inside the band — invisible — while
the design cell (payload-dominated) stays green. The affine-law claim
("three collinear points") is also not literally true: the minimal point
is 2 B off the line.

> **Resolution** (directly executable, in `tests/dispute_wire.rs` and
> `src/tree/mirror/streaming/window.rs`):
>
> 1. In dispute_wire.rs, split the band into what it actually absorbs:
>    - `const TOLERANCE_BYTES: usize = 1;` with its doc reduced to the
>      integer-division-adjacency sentence only.
>    - New `const MINIMAL_CELL_RESIDUAL: usize = 2;` documented as: "the
>      minimal cell reads this many bytes *under* the intercept —
>      small records batch more densely, so their share of per-frame
>      framing is smaller (the mechanism is stated at
>      `DISPUTE_OVERHEAD_BYTES`); a measured value moving off
>      `intercept + payload − MINIMAL_CELL_RESIDUAL` by more than the
>      division tolerance is a real framing change, in either
>      direction."
>    - `minimal_records_pin_the_fixed_overhead`: `expected =
>      fixed_overhead_bytes() + U64_ENCODED_BYTES -
>      MINIMAL_CELL_RESIDUAL` (= 50), tolerance ±1.
>    - The mid and design cells keep their current `expected` values,
>      tolerance now ±1 (they sit exact today; verify by running —
>      expected prints are `107` and `215`).
>    - Reword the module doc's "three collinear points" linearity
>      sentence: the law is affine over the interior and design points,
>      with a stated, pinned −2 B residual at the minimal end.
> 2. In window.rs:215–218, point the parenthetical at the per-cell
>    constant instead of "the cells' tolerance band absorbs" (e.g.
>    "pinned as the minimal cell's own residual constant in
>    `tests/dispute_wire.rs`").
> 3. Acceptance: `cargo nextest run --test dispute_wire` green;
>    mutation check by hand: temporarily add 1 to the minimal cell's
>    measured value (e.g. `implied + 1` in the assertion) and confirm
>    the test now fails — then revert. All three cells' printed
>    `implied` values unchanged (50, 107, 215).

**R2 (seed 8, first clause). Three defensive arms in `decode_v2` are
unreachable, none documented as such — the charter's disclosure
("documented without overclaiming") is not true of the tree.** —
**verified by arithmetic + grep.** *(Correction to this review's first
issue: I initially reported two dead arms; writing the resolution
surfaced a third.)*
`decode_v2` ([handshake.rs:219–271](file:///Users/oxide/src/rumors-review/src/tree/mirror/handshake.rs))
always receives exactly 30 bytes (`Staged::validate` slices `want`,
which is `V2_PREAMBLE_LEN`). Passing the version check forces a one-byte
version head (`0x02`; wider spellings of 2 are `NotShortest` →
`PreambleDefect::Version`, other values → `VersionMismatch` before the
network parse); passing the network filter forces the one-byte head
`0x50`. So consumption before the intent item is always 13 + 16 = 29
bytes, leaving exactly one:
- `input.len() < NETWORK_LEN` (line 254, `NetworkTruncated`) can never
  fire;
- the intent item is one byte, so `intent.value ≤ 23` and
  `u8::try_from` (line 267) can never fail — and if it ever did, it
  fabricates `IntentInvalid { byte: 0xff }`, a byte the peer never sent;
- after a one-byte intent, `input` is always empty, so the
  `TrailingBytes` arm (line 264) can never fire either.
Grep finds no documentation of any of the three.

> **Resolution** (mostly directly executable; the variant-removal
> alternative is **owner-gated**):
>
> 1. Intent width arm (handshake.rs:267–268): per the crate's
>    disposition ladder this is a truly-unreachable branch that must
>    structurally exist → make it assert. Replace the
>    `.map_err(|_| Error::IntentInvalid { byte: u8::MAX })` with
>    `.expect("the 30-byte preamble leaves exactly one byte for the
>    intent item, whose one-byte head's value is at most 23")`, keeping
>    `.and_then(Intent::from_byte)` → adjust to
>    `Intent::from_byte(u8::try_from(intent.value).expect(...))?`. This
>    removes the diagnostic fabrication outright.
> 2. `NetworkTruncated` and `TrailingBytes`: keep the arms (the slice
>    bound is load-bearing for `split_at`, and the trailing check guards
>    any future caller with non-fixed input), and document the
>    defensive status in both places:
>    - On each variant's rustdoc
>      (handshake.rs:333–335 and :341–343): one sentence of the form
>      "Defensively reachable only: the fixed 30-byte V2 preamble with a
>      validated version and network head always leaves exactly
>      16 bytes / no trailing byte; this variant guards the width
>      arithmetic against future layout drift, not any input the current
>      dialect admits." (State it positively and undated, per the
>      no-ghost-references rule.)
>    - A short comment at each construction site restating the same
>      derivation in one line.
> 3. Add the missing reachable-defect constructions (they exist for
>    `Intent` only) as unit tests in
>    `src/tree/mirror/handshake/tests.rs`, using the existing `staged`
>    helper pattern:
>    - `PreambleDefect::Version`: a 30-byte frame with byte 11 (the
>      version item) spelled `0x38` (a major-1 head) — assert
>      `Err(Error::Malformed { defect: PreambleDefect::Version })`; and
>      a second case with bytes 11–12 = `0x18 0x02` (widened spelling of
>      2) asserting the same (this is the determinism witness at this
>      layer).
>    - `PreambleDefect::Network`: byte 12 = `0x51` (bstr of length 17) —
>      assert the `Network` defect.
>    - For the two defensive variants, add an explicit exemption note in
>      the test module (the error-atlas `EXEMPT_MARKERS` pattern is the
>      house style): a comment block naming both variants and the width
>      derivation, so an auditor sees the absence of constructions is
>      deliberate.
> 4. **Owner-gated alternative**: delete `NetworkTruncated` (and fold
>    its check into an `expect`), shrinking the just-ruled public
>    `PreambleDefect` enum. Do not take this path without Finch's
>    ruling; the enum is `#[non_exhaustive]` and was owner-shaped in
>    `5367a82e`.
> 5. Acceptance: handshake tests green including the new constructions;
>    `intent_byte_space_is_exhaustive` and
>    `arbitrary_preamble_decodes_by_the_oracle` unchanged and green;
>    `cargo clippy --workspace --all-targets --all-features -- -D
>    warnings` clean.

**R3 (seed 7). `LeafRunError::Head`'s `source` field is publicly
unnameable.** — **verified.**
`LeafRunError` is re-exported at
[error.rs:41](file:///Users/oxide/src/rumors-review/src/error.rs);
its `Head { source: HeadError }` payload type is `pub` only inside the
crate-private `tree::mirror::cbor` module and re-exported nowhere. A
user can bind `source` and use it via `std::error::Error`, but cannot
write its type, match its variants (`Truncated`/`Indefinite`/`Reserved`/
`NotShortest` — precisely the taxonomy the determinism contract makes
interesting), or follow the rustdoc link (it renders dead).

> **Resolution** (directly executable; additive API, so note it in the
> PR description for Finch's review pass):
>
> 1. Re-export the type along the same path its carrier travels: in
>    `src/tree/mirror/streaming/remote/codec.rs`'s public export block
>    (the `pub use error::{...}` / `pub use frame::{...}` cluster at
>    :84–96), add `pub use crate::tree::mirror::cbor::HeadError;` — then
>    it rides the existing
>    `pub use crate::tree::mirror::streaming::remote::{...}` list in
>    `src/error.rs:38–44`; add `HeadError` to that list. (Route through
>    the codec module rather than exporting `cbor` itself: the head
>    grammar is codec vocabulary; the `cbor` module stays private.)
> 2. Check `HeadError`'s rustdoc reads as public API (it already
>    documents each variant); add `#[non_exhaustive]` **only if** Finch
>    wants room for future head-grammar defect classes — default: leave
>    it exhaustive, since RFC 8949's head grammar is closed
>    (**owner-gated** either way, one line).
> 3. Acceptance: a doc build (`just gate`'s docs leg, or
>    `cargo doc -p rumors --no-deps`) shows `LeafRunError::Head`'s
>    `source` type as a live link; `rumors::error::HeadError` is
>    nameable from an integration test (add one line to any existing
>    tests/ suite that imports it and matches `HeadError::NotShortest`
>    to pin the reachability).

**R4. The stream-open label is documented as "two-byte" but is 2–4
bytes, and `tests/observe.rs` hardcodes the two-byte shape.** —
**verified by reading.**
The label is two CBOR uint items (streams.rs:59–69); an epoch ≥ 24
encodes in two bytes, so a link past its 24th session writes a 3-byte
label. [observe.rs:101](file:///Users/oxide/src/rumors-review/src/observe.rs)
("the two-byte stream-open label") is wrong for that case — the design
doc's own table says "+0–1 bytes per stream", so the doc knows better.
`tests/observe.rs:189/196/220/226` index `blob[1]` and slice `blob[2..]`
— correct for the epochs these tests reach, wrong at epoch ≥ 24. The
failure mode when it bites is loud, not masking: `blob[1]` would read
the epoch's value byte (≥ 24, outside the 0..17 stream-index range), the
handler lookup fails, and the test panics with a missing-handler
message — it cannot pass wrongly. No production code depends on the
two-byte assumption (the snapshot harness parses labels through
`stream_label`).

> **Resolution** (directly executable):
>
> 1. `src/observe.rs:101`: replace "the two-byte stream-open label" with
>    "the stream-open label (two leading unsigned-int items)". Check
>    the same phrase does not recur elsewhere
>    (`grep -rn "two-byte" src/ crates/rumors-tracing/` and fix any
>    other label reference the same way; the phrase at framing.rs, if
>    any, refers to the V1 length header and is unrelated).
> 2. `tests/observe.rs`: import `stream_label` from
>    `rumors::testing` (the file already imports from
>    `crate::common::gossip_snapshot`, which re-exports the testing
>    door; either path works). In `assert_mirrors` (:184–199) and
>    `assert_received_mirrors_remote` (:219–229) replace the
>    `blob[1]` / `&blob[2..]` pairs with:
>    `let ((_, index), label_len) = stream_label(blob);` and
>    `&blob[label_len..]`. Drop the now-redundant `blob.len() >= 2`
>    assertion (stream_label panics with a named reason on a malformed
>    label, which is the harness contract).
> 3. Acceptance: `cargo nextest run --test observe` green;
>    `grep -n "blob\[" tests/observe.rs` returns nothing.

**R5 (seed 8, second clause). The io::Error collapse one layer below
the handshake: assessed, and I recommend extending the ruling.** —
**owner-gated (public error surface).**
The disclosed sites hold up as disclosed:
[party.rs:141](file:///Users/oxide/src/rumors-review/src/tree/mirror/party.rs)
and [greeting.rs:305](file:///Users/oxide/src/rumors-review/src/tree/mirror/streaming/remote/codec/greeting.rs)
map malformed-CBOR heads to `io::Error(InvalidData)` (party's whole
malformed surface rides `Error::Io`; the greeting's rides the mirror's
`HandshakeDecode`). The current state is *self-consistent* — the
`Error::Io` table row explicitly documents "a wire framing fault outside
the streaming mirror (counterparty bug: report it)" — but it is not
consistent with the ruling's principle: an application cannot
programmatically distinguish "transport died" from "counterparty sent a
malformed identity hand-off" without string-sniffing `ErrorKind`, and
that is the same diagnosis the owner ruled worth typing at the preamble.
The hand-off especially is high-stakes (identity in flight; the
retire/bootstrap recovery guidance differs between the two causes).

> **Resolution** (two steps; step 1 is the executable one):
>
> 1. Put the decision to Finch, framed as: "the 2026-08-19 error-taxonomy
>    ruling typed the preamble's failures; the identity hand-off and the
>    greeting still collapse malformed-peer-bytes into
>    `io::Error(InvalidData)`. Extend the ruling one layer, or record a
>    scoping decision?" Record the outcome in
>    `design/cbor-legible-wire.md`'s decision record either way.
> 2. If extended, the implementation shape (sized M, no wire change):
>    - New public `HandOffDefect` enum (mirroring `PreambleDefect`'s
>      style): `NotPartyTagged`, `NotAByteString`, `UnaddressableLength`,
>      `HeadMalformed(HeadError)` (nameable once R3 lands),
>      `Undecodable` (carrying the `before::error::Decode` rendered or
>      typed). New `Error::HandOffMalformed { defect }` and
>      `Error::HandOffTruncated { }` variants (the promised-hand-off
>      close; today `UnexpectedEof` under `Io`). Wire them in
>      `party.rs::receive_v2`/`read_head`/`decode_party`; the V1 path
>      keeps `Error::Io` (frozen dialect).
>    - Greeting: replace `ReadGreetingError::Decode(io::Error)`'s
>      payload with the existing typed `GreetingError` (it already
>      enumerates every failure; today it is flattened to a string at
>      greeting.rs:277–281), and surface it through the mirror's
>      `HandshakeDecode` variant as a typed source instead of
>      `io::Error`.
>    - Update the `Error` doc table rows; update
>      `tests/common/sim.rs::is_honest_error` (malformed stays
>      dishonest; a typed hand-off *truncation* joins
>      `PreambleTruncated` as honest-cut); sweep
>      `src/tree/mirror/party/tests.rs` (its `io_error` helper and the
>      typed-error assertions move to the new variants).
>    - Acceptance: party and greeting test suites re-pin every failure
>      to its typed variant; the disruption suite green (a new
>      committed seed, if one shakes out, is committed per policy);
>      `just gate` clean.

**R6. `TAG_SELF_DESCRIBED` is dead code in the default-features lib, and
55799 is spelled three independent ways.** — **verified** (`cargo check
-p rumors` warns "constant `TAG_SELF_DESCRIBED` is never used"; the
gate's clippy legs never observe that configuration — `--tests` keeps
the capture module alive — so the gate is green while a bare check of
the shipped lib warns).
The production preamble spells 55799 as the `V2_PREFIX` byte literal
(handshake.rs:78), the bookmark as its own `SELF_DESCRIBED` byte literal
(format.rs:72), and the constant is used only by the test-gated
renderer. Even the pin test `prefix_matches_the_writers` writes the
literal `55799`, not the constant.

> **Resolution** (directly executable except the gate-leg question):
>
> 1. Gate the constant to its users: in
>    `src/tree/mirror/cbor.rs:45–47`, add
>    `#[cfg(any(test, feature = "test-internals"))]` on
>    `TAG_SELF_DESCRIBED` (matching the capture module's own gate at
>    codec.rs:60–61).
> 2. Tie all three spellings to one authority by committed tests:
>    - `src/tree/mirror/handshake/tests.rs::prefix_matches_the_writers`:
>      replace the literal `55799` with `cbor::TAG_SELF_DESCRIBED`.
>    - `src/bookmark/format/tests.rs`: add a test asserting
>      `SELF_DESCRIBED` equals the rendering of
>      `cbor::write_tag(&mut v, cbor::TAG_SELF_DESCRIBED)` (3 bytes),
>      doc: "the bookmark's opening literal is the self-described tag's
>      one canonical spelling; the shared constant is the authority."
>      (`SELF_DESCRIBED` is private to `format.rs`; the test module is
>      its child and sees it.)
> 3. Acceptance: `cargo check -p rumors` (no flags, default features)
>    emits **zero** warnings; `just gate` clean; the two pin tests
>    green.
> 4. **Owner-gated** follow-up worth proposing while there: a
>    `clippy-default` leg that lints the bare lib
>    (`cargo clippy -p rumors --lib -- -D warnings`, without `--tests`),
>    so dead code in the shipped configuration cannot recur unseen. One
>    justfile line; Finch's call because it lengthens the gate.

### Behavioral notes (bytes unchanged, behavior worth knowing)

**N1 (seed 6). Query listings and frame heads now read per-item against
the transport.** — **assessed.**
The old wire's length-framed bodies arrived in one bulk read; the V2
decoder reads each head byte-by-byte (`read_head_async`: a 1-byte read
plus the extension) and each digest in a 32-byte `read_exact`. A
full-fan query is on the order of a thousand small reads on an
unbuffered transport. The greeting was deliberately embedded (tag 24) to
avoid exactly this ("no incremental map walk happens against the
transport", greeting.rs:6–8) — data-stream queries walk incrementally.
The documented mitigation stands (framing.rs:19–24: wrap raw sockets in
`BufReader`; caller-owned buffering is session-safe).

> **Resolution** (optional, docs-only, directly executable): the
> BufReader guidance currently lives in a crate-private module doc
> (framing.rs). Surface one sentence where a `Link` builder will see it:
> in `src/link.rs`'s rustdoc (the transport-contract section), add
> "Reads are exact and item-granular; on an unbuffered transport, wrap
> the read half in `tokio::io::BufReader` — caller-owned buffering
> outlives a session and is safe across session boundaries." Acceptance:
> `just readme` re-derived if the crate-level docs changed (they should
> not); docs leg green. If left undone, no invariant is at risk.

---

## Seed dispositions (charter's 14, in order)

1. **Red docs-only gate pair** — verified: `d6262c03` touches only
   `///`/`//!`/comment lines (mechanically checked); nothing non-docs
   rode in. *No action.*
2. **Bisection** — verified: all 12 commits in `15bd905e..a5c4ca1a` pass
   `cargo check --all-targets --all-features`. The adapter branch's
   disclosed broken merge (`7403bd72`) stands as disclosed; its tip
   builds. *Rebase addendum*: the rebase to `d05a6d03` replayed these
   commits and the rebasing agent gated only the tip plus its three
   conflict stops, so the verdict was re-established rather than
   carried over — all 12 rebased commits in `991f4663..d05a6d03` pass
   the same check sweep (**verified**, post-rebase). *No action.*
3. **`SUPPLY_FRAME_OVERHEAD` envelope** — verified against the encoder:
   the envelope charges heads at their widest (10 B; actual 6–10), the
   encoder's `admits` and the decoder's `covers` share the single
   `covers` boundary so they cannot drift, `record_len` is pinned
   against an actual `push`, and the flush loop
   (adapter/encode.rs:225–239) checks before every post-first push.
   Under-batching is bounded at 4 B/frame and documented. One nit: the
   `OverbatchedRun.declared` field reports the *charged* envelope,
   overstating the actual frame by up to 4 B while reading as a wire
   fact.
   > **Resolution** (directly executable, docs-only): on
   > `DecodeErrorKind::OverbatchedRun`
   > (codec/error.rs:148–151), add a field doc on `declared`: "the
   > frame's charged wire size — its run body plus the
   > `SUPPLY_FRAME_OVERHEAD` envelope at its widest — which may exceed
   > the actual frame by the envelope's head slack (at most 4 bytes)";
   > and adjust the `#[error]` text from "occupies {declared} wire
   > bytes" to "charges {declared} wire bytes". Acceptance: error-atlas
   > suite green (its markers match on the prefix "kind:
   > OverbatchedRun(declared=", which does not change).
4. **Minimal-cell ±2 band** — broken as suspected; finding **R1**.
5. **Mutants roster** — verified untouched: every exclusion predates the
   stack and names suanpan/before skyline code; nothing excludes the new
   codec/bookmark/hook code, so no in-diff green-washing. Whether the
   new code's mutants die is unmeasured — see the residual-risks
   resolution (mutants campaign) below.
6. **Per-entry query decode** — finding **N1**.
7. **`LeafRunError::Head` visibility** — finding **R3**.
8. **Handshake taxonomy** — every `PreambleDefect` variant has exactly
   one construction site in code, and the disruption honesty classifier
   admits the typed truncation correctly (sim.rs). But three of the
   decoder's defensive arms are unreachable and undocumented — finding
   **R2** (which also carries the missing-construction resolutions).
   Sibling collapses — finding **R5**.
9. **"Uncontended by construction" mutexes** — verified by reading every
   `control_sent`/`control_received` site: per direction the writers are
   protocol-sequential phases owning the half exclusively (preamble →
   greeting → hand-off → epilogue), and the only concurrency (preamble
   write ∥ read; greeting send ∥ receive; epilogue send ∥ receive) pairs
   *different* mutexes. The locks are leaf locks, held only across the
   synchronous `message()` call, no await under lock. *No action.*
10. **Renderer panic/failure split** — principled and documented at the
    module boundary (capture.rs:40–45). Assessed clean — except that one
    fallback trigger (the depth bound) does not hold on the
    embedded-unfold path: finding **B1**.
11. **No session ordinal** — verified: `SessionInfo` carries none, the
    library holds no numbering state, the hook docs teach the
    observer-side atomic pattern, and the adapter implements it
    correctly (peer-level `AtomicU64` for session ordinals, per-session
    shared `Arc<AtomicU64>` for message ordinals, ordinal advanced even
    when the subscriber is disabled). Two nits, resolved under
    Simplifications S2 (the `bootstrap.rs:170` prose vestige) and
    Missing tests T2 (concurrent-session numbering untested).
12. **Extractor re-accept** — verified structurally: `0db57d00` touches
    no wire-producing file (renderer, its tests, harness, test surface,
    and exports only), so wire bytes could not move in it; the totality
    oracle (`assert_items_account_for`, applied per stream to control
    and every data stream, both endpoints, with a count check that no
    hook stream lacks transport bytes and vice versa) re-attests
    items==wire on every run, and the received directions are separately
    held to the peer's sent capture in tests/observe.rs. Attacks on the
    oracle's coverage came up empty except B1's renderer robustness and
    the injectivity analysis: I could not construct two byte streams
    rendering identically — canonical-head enforcement in `parse_node`,
    exact-width float/simple preservation, escaped text, full hex for
    byte strings, and explicit-hex fallbacks close the paths I tried
    (arity is recoverable from body presence; embedded contents show
    byte counts). Injectivity: assessed sound, modulo B1. *No action
    beyond B1.*
13. **Orphaned snapshots** — verified: a name-resolution sweep over all
    24 `tests/snapshots/*.snap` plus the src-side insta snapshots finds
    every one referenced by its generating test. The corpus still has no
    *standing* defense.
    > **Resolution** (directly executable): add
    > `tests/snapshot_liveness.rs`, modeled on
    > `tests/seed_liveness.rs` (same doc-comment style: the invariant,
    > the resolution rules transcribed, the provenance note):
    > - Walk `tests/snapshots/*.snap`; for each, split the stem on the
    >   first `"__"` into `(suite, name)`; require `tests/<suite>.rs`
    >   to exist and to contain either `fn <name>(` or the quoted
    >   string `"<name>"` (explicitly named snapshots — the bookmark
    >   pins use this form).
    > - Walk `src/**/snapshots/*.snap`; for each
    >   `rumors__<modpath>__tests__<name>.snap`, map `<modpath>` to
    >   `src/<modpath with __ as />/tests.rs` and apply the same
    >   containment rule.
    > - Fail with the orphan's path and the file searched. Skip
    >   nothing; `.snap.new` files (pending insta output) fail loudly
    >   as "unaccepted snapshot committed".
    > - It is an ordinary integration test, so `just test-all` (and
    >   therefore the gate) runs it with no justfile change.
    > - Acceptance: green on the current tree; delete-resistant
    >   demonstration: temporarily add a stray
    >   `tests/snapshots/gossip_snapshot__nonexistent.snap` and confirm
    >   it fails naming that file — then remove it.
    > Rejected alternative, for the record:
    > `cargo insta test --unreferenced=reject` is the mature-tool path
    > but couples the gate to the cargo-insta runner and a full-suite
    > invocation; the pairing test matches the tree's existing
    > seed-liveness pattern and runs in the same harness. If Finch
    > prefers the tool, wire `cargo insta test --unreferenced=reject`
    > as a distinct justfile recipe in the gate's test tier instead —
    > **owner-gated** because it is a gate/tooling change.
14. **Known pin gaps** — confirmed as disclosed, with one sharpening and
    one refutation: the session corpus contains no nonempty Query frame
    (only `QueryEmpty`; nonempty listings appear only inside greetings),
    *but* the codec-level `canonical_frame_atlas_snapshot` byte-pins a
    one-child Query frame at every placement, so the gap is corpus-level
    composition, not spelling. digestshare is confirmed absent from the
    justfile. The depth-64 bound does **not** hold on every walk path —
    refuted by construction (**B1**).
    > **Resolution, query fixture** (directly executable): add one
    > gossip-snapshot test (in `tests/gossip_snapshot.rs`, alongside
    > `deep_trie_divergence`) whose corpus provokes a nonempty Query
    > frame. Construction guidance: a nonempty Query needs a disputed
    > interior node whose reply *lists children* rather than shipping
    > or pruning them — two peers sharing a common prefix under which
    > **both** sides hold children (≥ 2 shared radixes) with differing
    > content on at least one, so neither side's subtree is absent (an
    > absent side yields supplies; identical subtrees yield matches;
    > an empty listing yields `QueryEmpty`). Iterate the corpus until
    > the accepted snapshot's render contains a signal line matching
    > `/ Query(` **and** a body line matching
    > `{ / listing: <n> child(ren) /` with n ≥ 1 — assert exactly that
    > in the test body *before* the `insta::assert_snapshot!`, so the
    > fixture cannot silently degrade back to `QueryEmpty` under a
    > future corpus change (the liveness floor for this pin). Accept
    > the new snapshot; the re-accepting commit names the new fixture
    > (it adds a snapshot; it re-accepts nothing existing).
    > **Resolution, digestshare liveness**: see Residual risks below
    > (**owner-gated** gate change).

---

## Round-4 assumption checks not covered above

- **O(1) record and run skip under tag 63** — assessed sound:
  `record_head` reads exactly two heads and returns the content length;
  `RecordSlices::next` jumps by length; nested containers inside a
  record's payload are never walked during skip or structural
  validation (`from_encoded` chains lengths only). *No action.*
- **Charge-before-custody** — verified by reading both supply paths
  (adapter/decode.rs:168–182 and 372–388): `ledger.charge(1)` lands
  after structural parse, before `Leaf::leaf` takes payload custody,
  per record, while the reply is open. The overbatch gate rejects
  before buffering (both decoders, mirrored logic, shared
  `lone_record_spans` boundary), with committed corner classification.
  *No action.*
- **Budget memory argument** — verified by reading: runs stay encoded on
  both sides; the decoder yields records one at a time into a bounded
  channel; the only whole-run decoded materialization is the test-gated
  renderer. *No action.*
- **Deadlock freedom** — assessed: `src/link.rs` is byte-untouched in
  the range; the hook adds no protocol dependency (handlers are
  synchronous, per-stream, invoked outside any protocol lock; control
  observers are leaf mutexes; a blocking handler stalls only its own
  directed stream, as documented). *No action.*
- **Greeting key order** — verified: the `KEYS` roster is exactly CBOR
  deterministic (length-first bytewise) order, and `parse_greeting`
  demands the exact roster in that order. *No action.*
- **Preamble fixed width** — verified: 11 + 1 + 17 + 1 = 30, pinned to
  the writers by `prefix_matches_the_writers`. *No action.*
- **Merge seam (`9aff980b`)** — verified: empty combined diff (no manual
  conflict resolution); disjoint file sets except `lib.rs`; the merged
  tree's cross-references compile and pass. *No action.*
- **V1 frozen** — verified: `crates/before`, `src/link.rs`, all 28
  alternating snapshots and the V1 codec untouched; the single V1-side
  test change is a mechanical adaptation to the shared handshake's new
  signature, not a behavior change. The charter's "all V1 tests
  untouched" is inexact on that one file; the wire claim ("zero byte
  movement") holds. *No action.*
- **Wire × hook byte identity** — verified by the committed differential
  (`observation_never_changes_the_wire`) plus reading the capture path
  (received items are transport bytes via `CaptureRead`; the one
  re-encoding — the over-budget lone-record prefix — is byte-identical
  because heads are canonical-enforced; sent items are the exact buffers
  just flushed). *No action.*

## Public API deltas — judgment

- `rumors::observe` — well-shaped: three levels mirror the session
  machinery, bytes-only contract keeps it rumors-blind, `SessionInfo`/
  `StreamInfo`/`StreamId` are `#[non_exhaustive]`, decline-at-any-level
  is cheap, and the rustdoc is genuinely at the crate's standard. Two
  nits:
  - `SessionKind` is *not* `#[non_exhaustive]` although it enumerates
    lifecycle operations the crate could plausibly grow; the crate's
    recent practice (`1e458d69`) marks open diagnostic taxonomies
    non-exhaustive.
    > **Resolution** (**owner-gated**, one attribute): ask Finch whether
    > `SessionKind` is an open taxonomy. If yes, add
    > `#[non_exhaustive]` to it (src/observe.rs:207) *now, pre-release*
    > (adding it later is the breaking change), and confirm no in-tree
    > exhaustive `match` on it exists outside the crate boundary
    > (grep shows none today; the tracing adapter formats it with
    > `?kind` and never matches). If no, record one sentence at the
    > enum ("closed by design: a session is entered by exactly these
    > operations") so the asymmetry against the crate's convention
    > reads as deliberate.
  - `Observer::session`'s "called once per session" reads as
    unconditional while V1 sessions never call it (the module doc states
    the exclusion).
    > **Resolution** (directly executable): in the method doc
    > (src/observe.rs:130–134), append "— for sessions of an observable
    > dialect; see the module docs' `Protocol::V1` exclusion."
- `Peer::observe` / `Bootstrap::observe` / `Bootstrap` losing `Copy` for
  `Clone` — right call, honestly documented, retry affordance
  preserved. The bootstrap-ordinal prose vestige is resolved under S2.
- `rumors::tags` — minimal and correctly scoped; provisional-numbers
  caveat and serde-placement rule stated at the module. The
  triple-spelling of 55799 is resolved under R6.
- `BOOKMARK_MAGIC` removed, `BOOKMARK_FORMAT_VERSION` u16→u64 (=4),
  `FormatError` restructured with typed `FrameDefect`/`RecordDefect` —
  coherent, meaning-named, `#[non_exhaustive]` where open; docs
  excellent. `Error::VersionMismatch.remote_version` u16→u64 matches
  the V2 preamble's uint field. *No action.*
- `PreambleDefect` + `PreambleMalformed`/`PreambleTruncated` — good
  shape and docs; dead arms resolved under R2.
- `PROTOCOL_MAGIC` behind `protocol-v1` — correct, and the V2 endpoint's
  internal legacy diagnosis is tested both whole and truncated. *No
  action.*
- `rumors-tracing` — clean minimal surface, deliberately not `Clone`,
  correct explicit span parenting, bounded rendering with all four
  budgets, honest "when not to use it" section. Nit: the crate doc's "a
  disabled target costs the enabled-check alone" omits the (documented-
  in-code, deliberate) relaxed `fetch_add` per message.
  > **Resolution** (directly executable, on `w2/tracing-adapter`): in
  > `crates/rumors-tracing/src/lib.rs`, amend the cost sentence to "a
  > disabled target costs the enabled check plus one relaxed atomic
  > increment (the ordinal must advance even unobserved; see
  > `StreamAdapter::message`)". Then re-derive the README
  > (`just readme`; the crate is wired into `tools/readme` per the
  > branch's diff). Acceptance: `readme-check` green.
- Testing surface (`render_hook_capture`, `HookCapture`, `HookStream`,
  `assert_items_account_for`, `stream_label`; `render_v2_capture`
  removed) — appropriate for a doc-hidden test-internals door. *No
  action.*

## Simplification candidates

Adoptable-now (behavior-preserving; each is its own one-commit fix):

- **S1.** [gossip/tests.rs:130](file:///Users/oxide/src/rumors-review/src/peer/gossip/tests.rs)
  test doc says the epilogue read "consumes exactly one byte"; the
  marker is two bytes (the test body is correct). Test-doc correctness
  is a stated crate invariant.
  > **Resolution**: change the doc comment's first line to "Reading the
  > marker consumes exactly the marker's bytes, leaving later bytes
  > untouched." (Byte-count-free, so it cannot rot again.) No code
  > change.
- **S2.** [bootstrap.rs:170](file:///Users/oxide/src/rumors-review/src/peer/bootstrap.rs)
  ordinal vestige (seed 11): "its session ordinals counting from the
  join (session `0`)" speaks as if a defined numbering exists; by owner
  ruling the hook carries none.
  > **Resolution**: replace the clause with "the joined peer then keeps
  > the handler exactly as [`Peer::observe`] would attach it; an
  > observer that numbers sessions will count the join as the first
  > session it sees." Grep `session 0`/`ordinal` across `src/` to
  > confirm no other vestige (the observe.rs doc-example's `ordinal`
  > is the observer-side pattern and stays).
- **S4.** [greeting.rs:213](file:///Users/oxide/src/rumors-review/src/tree/mirror/streaming/remote/codec/greeting.rs)
  `uint(input, _key)`: the parameter is unused.
  > **Resolution**: use it for diagnostics — change the signature to
  > `fn uint(input: &mut &[u8], detail: &'static str)` and have the
  > three callers pass `"set_len is not an unsigned int"`,
  > `"max_version_bytes is not an unsigned int"`,
  > `"target_message_size is not an unsigned int"`, returned as the
  > `Shape` detail (replacing the current shared string). Acceptance:
  > greeting and handshake suites green; no test asserts the old shared
  > string (grep `"greeting size entry"` — only the source).

Design-proposal (moves readings or API; each **owner-gated**):

- **S3.** `bookmark/format.rs`'s `push_head`/`Reader::head`
  re-implements `mirror/cbor.rs`'s `write_head`/`read_head` (plus the
  third 55799 spelling, R6). Both are property-tested, but one
  canonical-head implementation is the crate's own stated ideal, and
  drift between them is the class B2 shows the prose already lost track
  of.
  > **Resolution sketch** (for the round that takes it): replace
  > `push_head` with `cbor::write_head` (byte-identical output — pin by
  > leaving `frame_empty`/`frame_non_trivial` untouched), and rebuild
  > `Reader::head` on `cbor::read_head` with a thin adapter mapping
  > `HeadError::Truncated` → `FormatError::Truncated { len }` and every
  > other `HeadError` → `NotABookmark { defect }` (the caller's
  > defect), preserving the exact-position `Truncated.len` semantics
  > (the reader's `at` bookkeeping stays). Acceptance: both bookmark
  > snapshot pins byte-identical; the corruption, truncation, and
  > version-spelling suites green with unchanged variant assertions.
- R3's `#[non_exhaustive]` question, R5's typed hand-off/greeting
  errors, and `SessionKind`'s openness — specified at their findings.

## Where I found no issues

The canonical head grammar itself (cbor.rs: shortest-form check, widened
/indefinite/reserved/truncation rejection, async/sync agreement — the
proptests are the right ones and I could not construct a hole); signal
grammar and phase validation (the 340-placement atlas with exact bytes
is excellent); frame arity/shape enforcement; the two decoders' mirrored
over-budget logic; encoder flush algebra (seed 3); ledger
charge-before-custody; the greeting's exact-roster parse; preamble
diagnostic ordering (magic → version → semantics, with the cross-dialect
diagnosis tested in both directions and at the truncation boundary); the
bookmark frame's integrity totality (every byte hash-covered or
exact-compared; the corruption and truncation sweeps are
value-independent, so their proof generalizes past the `^0xff` flip);
the error atlas's two-ended coverage enforcement; the hook's threading
(begin-before-first-byte, election-before-data, V1 exclusion, control
handlers minted ahead of the preamble); the internal-capture suite (both
directions, election complementarity, one-item checks); the
wire-legibility property (genuinely rumors-blind walker); seed-liveness
auto-covering the new proptest seed files; the docs-only repair commit;
the merge seam; the re-derived pins (each read from its instrument, no
transcribed constants — the probe's old hardcoded `28` was correctly
dissolved into `dispute_overhead_bytes()`); digestshare's figures and
its renderer contract; and the design document's cost table against the
shipped constants (the +3 B/child, 35→43, +3.9%/+14%, 1.8 MB reply, and
30-byte preamble figures all reconcile with code or measurement).

## Considered and dismissed

Candidates examined during the review and dismissed, recorded so the
packet is self-contained (previously these dispositions lived only in
review conversation):

- **The greeting's declared length is uncapped (u64)** where the old
  wire's framing capped at u32: dismissed — `read_payload`'s memory
  tracks receipt, never the declaration, so a large declaration costs
  only what the transport actually delivers; consistent with the stated
  memory policy and with V1's behavior at its own bound.
- **`resume_payload`/`reserve_exact` over-allocation edge**: in
  principle `Vec::reserve_exact` may over-reserve, letting `read_buf`
  read past the payload boundary; dismissed — capacity is clamped to
  `len` on every growth step, the global allocator honors exact
  requests in practice, and the code predates this stack.
- **`chunk_boundary_cuts(0)` underflows**: dismissed — a `cfg(test)`
  helper with no zero-total caller; the underflow is unreachable from
  any committed test.

## Residual risks and test gaps

- **No mutants evidence over the new code** (seed 5's flip side): the
  roster is clean, but no campaign result over the codec/bookmark/hook
  rewrite is in evidence. B2 is a useful calibration point for suite
  blindness even after its reclassification: nothing noticed that two
  decode sites delegate spelling judgment to a tolerant parser — in
  contract once B2's prose rescope lands, but the same blindness would
  hide an *unintended* delegation or a lost structural check.
  > **Resolution** (directly executable, resource-heavy): run
  > `cargo mutants --file 'src/tree/mirror/**' --file 'src/bookmark/**'
  > --file 'src/observe.rs'` from the repo root — the campaign
  > configuration of record (nextest, `--all-features`, dev profile) is
  > already in `.cargo/mutants.toml` and applies to the plain
  > invocation. Prefer running it on the big remote box (ox-east-1, via
  > the `building-on-illumos` sync flow) rather than a laptop; it is a
  > long, parallel run. Triage every survivor by the roster header's
  > disposition ladder (refactor → assert → exclude-with-rationale).
  > Sequencing (owner-ruled in review follow-up): the campaign runs
  > LAST — after the review-fix round reaches quiescence and after the
  > depth-limit/batch feature lands — so the single authoritative run
  > measures the fully settled code; the mutated files include exactly
  > the surfaces those rounds move. Deliverable: either zero survivors,
  > or each survivor dispositioned per the ladder in the same change
  > that records it.
- **Missing taxonomy constructions**: `PreambleDefect::Version`/
  `Network` (resolved under R2), the greeting's `NotShortest`
  rejection, and the bookmark's `Integrity`/`PayloadTag`/
  `PayloadByteString` defects (covered positionally by the corruption
  sweep but never asserted as their typed variants).
  > **Resolution** (directly executable):
  > - Greeting: in
  >   `src/tree/mirror/streaming/remote/codec/greeting/tests.rs`, add a
  >   test that takes a canonical `encode_greeting(&sample(vec![]))`,
  >   locates the `set_len` value head inside the embedded map (use the
  >   existing `find` helper on the key bytes `b"set_len"`; the value
  >   head follows the 8-byte key region: 1 head byte + 7 text bytes),
  >   re-spells that one-byte uint as the widened `0x18 <v>` form
  >   (splicing one byte in, and fixing the embedded byte-string length
  >   head and outer item accordingly — or simpler: build the malformed
  >   *map* directly by copying `greeting_map`'s output and splicing,
  >   then call `parse_greeting` on the map bytes, which needs no outer
  >   fix-up), and asserts
  >   `Err(GreetingError::Head(HeadError::NotShortest))`.
  > - Bookmark: in `src/bookmark/format/tests.rs`, three targeted
  >   flips on `frame(b"payload")` asserting typed defects:
  >   byte at the integrity head's offset (the `0x58` of
  >   `INTEGRITY_HEAD`; compute the offset as
  >   `SELF_DESCRIBED.len() + 1 + <version item len>` — 3 + 1 + 1 = 5
  >   today — rather than hardcoding) →
  >   `NotABookmark { defect: FrameDefect::Integrity }`;
  >   the payload tag byte (`0xd8`, at integrity offset + 2 + 32) →
  >   `PayloadTag`; the payload byte-string head re-spelled widened
  >   (`0x58 0x07` for the 7-byte payload, with the hash recomputed
  >   over the re-spelled covered region so only the spelling check can
  >   reject — the `non_canonical_version_spelling_is_rejected` test is
  >   the template) → `PayloadByteString`.
  > - Acceptance: each new test names its variant in a `matches!`; all
  >   suites green.
- **B1's class needs a standing test** — specified in B1's resolution
  (the two deep-nesting tests). Optionally mirror an
  unfold-budget-exhaustion test in the adapter's `render/tests.rs`
  (nest tag-24 five levels — one past `UNFOLD_BUDGET` — and assert the
  innermost renders as a raw `h'…'` byte string, not unfolded): the
  adapter passes today; the test pins that it stays true.
- **Orphan-snapshot recurrence** — specified at seed 13.
- **Query-frame session fixture** — specified at seed 14.
- **Adapter concurrency**: session numbering under genuinely concurrent
  sessions is argued from the atomic, tested only sequentially.
  > **Resolution** (directly executable, on `w2/tracing-adapter`): in
  > `crates/rumors-tracing/tests/adapter.rs`, add a test that clones one
  > observed peer's `Rumors` handle, creates two in-memory link pairs
  > and two counterparty peers, and drives both gossip sessions inside
  > one `tokio::join!` (current-thread runtime is fine; the sessions
  > interleave at await points, which is the property under test —
  > `session()` re-entrancy). Assert: exactly two `session` spans, with
  > `ordinal` fields `{0, 1}` as a *set* (order between concurrent
  > sessions is unspecified), and each session's message ordinals dense
  > from 0 (reuse the existing density assertion). Acceptance: test
  > green repeatedly (`--no-capture -j1` and default).
- **Label-width latency** — resolved under R4.
- **digestshare liveness** fires only when the tool is run by hand.
  > **Resolution** (**owner-gated**, gate change): add a justfile recipe
  > `digestshare:` running `./tools/digestshare` (its exit code already
  > carries the liveness verdict; a comment above the recipe should say
  > it checks the renderer-vocabulary contract, not a threshold) and
  > list it in the gate's lint tier next to the other `tools/` linters.
  > If Finch rules digestshare stays a manual measurement aid, record
  > that in the tool's docstring ("not gate-wired by decision: the
  > liveness guard protects only interactive runs") so the gap reads as
  > chosen rather than missed.

## Out-of-range observation (predates the stack; owner-ruled)

Surfaced while answering "can a user-chosen `T` crash production?"
(B1's functional-axis analysis), and disposed here explicitly rather
than mentioned in passing. It is **not** part of this stack's findings:
payloads were CBOR before the review range (the version-keying
migration), so the edge exists at the base commit too. **Assessed from
source** (ciborium ser/de internals plus the crate's call sites), not
constructed.

**Payload nesting depth has an undocumented, asymmetric functional
limit.** `ciborium`'s serializer has no recursion cap, so `send`
accepts a payload value of any nesting depth (recursion there is over
the user's own in-memory value); `ciborium::de::from_reader` caps
decode at 256 scopes. Consequence: a payload nested deeper than 256
container/tag scopes is accepted and stored locally, but **every
transfer of that leaf to any peer fails** — the receiver's record
decode returns `RecursionLimitExceeded` (typed, as
`DecodeLeafError::Message`), the session aborts cleanly, and the retry
fails the same way for as long as the divergence persists: a
deterministic gossip wedge on a locally-legal input. Nothing in the
crate's payload-facing documentation states a depth limit.

> **Owner disposition (Finch, ruled in review follow-up)**: the
> recursion limit becomes a configurable setup value on the `Peer`,
> enforced **symmetrically** (send-side admission and decode-side
> ingress judge the same bound), defaulting symmetrically to a
> reasonable number. Refined in the same follow-up: the limit is
> **exchanged in the greeting and must match exactly**; a mismatch in
> either direction is an unconditional, typed abort at the handshake.
> Rationale of record: negotiating down is unsound — a peer whose
> session limit dropped below its own configured limit may already
> hold messages deeper than the negotiated bound, content it is then
> not allowed to gossip — so any negotiation scheme merely relocates
> the failure to mid-session, conditional on which leaves actually
> differ. Parameter equality trades that (sometimes-crashy with some
> peers on some messages) for a deterministic fail-fast on mixed
> configurations at every pairing. Structurally, the limit is a
> property of the *shared set* — every replica must be able to hold
> and forward all content — so it is Network-like (pairwise equality,
> transitively fleet-wide agreement), not
> `target_message_size`-like (a per-session resource trade where the
> minimum is safe). The knob's rustdoc carries the contract inline;
> this packet records the ruling.
>
> **Implementation spec** (executable without further context; every
> formerly open sub-decision is now owner-ruled in place — steps 5 and
> 6 carry those rulings):
>
> 1. **The knob.** A builder-style setter in the mold of
>    `Peer::target_message_size`: `#[must_use]` on `Peer<T, B>`, plus
>    `Bootstrap<T>` and the `BookmarkedBootstrap` passthrough (the join
>    session decodes supplied records before a `Peer` exists — same
>    pattern as `run_budget`/`observe`). The value follows the peer
>    through `into_rumors`, cloning, reunion, bookmarking, and
>    retirement, like every other setup value. Store beside
>    `run_budget` in the config fields.
> 2. **The default.** Recommend `DEFAULT_PAYLOAD_DEPTH_LIMIT = 256` —
>    exactly the decode bound today's code already enforces implicitly
>    (ciborium's `from_reader` default), so a fleet upgrading together
>    sees no acceptance change on existing content; the only new
>    rejections are send-side (landing on the author of an over-deep
>    value) and the handshake mismatch (landing on mixed
>    configurations). Wire interop with pre-change code is governed by
>    the greeting format change in step 6, not by this constant. State
>    that rationale in the constant's doc. A named constant, exported
>    beside `DEFAULT_TARGET_MESSAGE_SIZE`.
> 3. **Decode side.** The value-erasure landed by PR #37 concentrated
>    wire payload ingress — both dialects — into one parse: the
>    peer-minted `PayloadDeserializer`
>    (`Message::deserializer::<T>`, `message.rs:148–166`), reached via
>    `Message::from_wire` from V2's `parse_record` and from V1's
>    `Message::from_reader` (whose own outer parse unwraps a flat byte
>    string). Replace that one inner `ciborium::de::from_reader` with
>    `ciborium::de::from_reader_with_recursion_limit(input, limit)`.
>    The minted fn is a plain function pointer and cannot capture a
>    runtime value, so the limit rides as data, in the **ruled
>    minted-codec shape** (the ruling and its rationale are recorded
>    at step 5): a small `Copy` struct pairing the minted serializer
>    and deserializer fn pointers with the `PayloadDepthLimit` field,
>    minted at `Peer` construction where the bare `deserializer` field
>    is minted today (`peer.rs:225`) and threaded wherever that field
>    travels. This preserves the erasure's property that sessions stay
>    non-generic. Parses that
>    stay at the library default, each structurally flat: V1's outer
>    byte-string unwrap (`message.rs:220`), the version atoms
>    (`tree/wire.rs:205`, `frame.rs:361`), and the bookmark payload
>    walk. Note in the commit message that the V1 freeze is byte-level
>    and this moves no byte; only the local acceptance bound becomes
>    configurable, symmetrically with V2.
>    Deliberately *not* threaded from peer config:
>    `Message::from_slice`/`from_bytes` (public constructors over
>    caller-supplied bytes — the caller's trust domain; they take the
>    limit as an explicit parameter instead, per step 5) and the
>    bookmark payload walk (crate-authored, structurally flat).
> 4. **Send side (the symmetric half).** Enforce at serialization time
>    — concretely inside `Message::try_new`, step 5's constructor:
>    after ciborium-serializing — the crate's own output, so
>    definite-length and canonical — run an O(n) *iterative* depth
>    scan over the
>    produced bytes using the crate's head grammar (`cbor::read_head`
>    with an explicit stack of remaining-child counts: an array head
>    pushes its count, a map head pushes 2× its count, a tag pushes
>    one; depth is the stack's high-water mark; bail as soon as it
>    exceeds the limit). Do **not** transcribe ciborium's scope
>    accounting into prose or constants — pin the symmetry with a
>    differential proptest instead: generate values nested to depths
>    around the limit (arrays, maps, and tags mixed), and assert the
>    send-side scanner and
>    `from_reader_with_recursion_limit::<ciborium::Value>` agree on
>    accept/reject at limit and limit ± 1. That test is the instrument
>    that keeps "symmetric" true against either side drifting.
> 5. **Send fallibility and the batch lifecycle (ruled, refined across
>    follow-ups).** Three rulings compose here: `send` eagerly creates
>    the `Message` at invocation, returning a typed error on a depth
>    violation; a failed batch commits nothing; and the batch is
>    reshaped into a **closure scope**, so batch state cannot exist
>    across an await point (by language rule — a synchronous closure
>    body cannot await) and commit becomes explicit code that runs iff
>    the closure returns `Ok`. `Batch` currently has no consumers, so
>    the reshape is contained to the crate's own tutorial, doctests,
>    and tests. Facts verified in-tree that this leans on:
>    `Batch::send` already serializes eagerly (`batch.rs`:
>    "Serialization runs here, not at commit"), so batching's
>    efficiency gain — one tree traversal, one commit, one wakeup — is
>    untouched; and building a batch holds no lock, so running a user
>    closure while building is sound. Implementation:
>    - **Thread the limit into `Message` creation itself** (without
>      this, every creation site silently reverts to the library
>      default): give `Message` a fallible, limit-taking constructor —
>      `Message::try_new<T>(message: T, limit) -> Result<Message,
>      PayloadDepthError>` (`Message` is type-erased; the constructor
>      is generic exactly as `Message::new` is) — which serializes
>      (`to_vec`), runs the
>      step-4 depth scan over the produced bytes, and errors past the
>      limit. The error carries the configured limit and says the
>      value exceeded it (the scanner may bail at limit + 1; it need
>      not report the true depth).
>    - **Failure-class split, stated at the constructor**: a
>      `Serialize`-impl failure keeps `Message`'s existing documented
>      panic contract (serializability is the caller's obligation —
>      programmer error, unchanged); a depth violation is the typed
>      error (data-driven — the value's shape can carry end-user
>      data).
>    - **The closure-scoped API (ruled)**: `Rumors::batch` becomes
>      `fn batch<R, E>(&self, f: F) -> Result<R, E>
>      where F: for<'s> FnOnce(&'s mut Batch<'_, T>) -> Result<R, E>`.
>      The scope handle keeps the `Batch` name and carries the private
>      fields (the `&watch::Sender<Inner<T>>`, the action list, the
>      depth limit from peer config). `E` is fully generic and
>      unbounded: a closure `?`s `Batch::send`'s `PayloadDepthError`
>      into its own error type (or returns it directly), and returning
>      any `Err` deliberately cancels the batch — an explicit abort
>      affordance the RAII design never had. Scope methods lose their
>      chaining returns (statement sequencing inside the closure
>      replaces fluent chaining):
>      `fn send(&mut self, T) -> Result<(), PayloadDepthError>`
>      minting via `Message::try_new` with the carried limit, and
>      `fn redact(&mut self, &Version)`. Single-action sugar stays on
>      `Rumors`: `fn send(&self, T) -> Result<(), PayloadDepthError>`
>      and infallible `fn redact(&self, &Version)`, each committing
>      immediately.
>    - **Commit-on-`Ok`; the lifecycle table collapses**:
>      `Rumors::batch` runs the closure and performs the
>      `send_if_modified` commit only on `Ok`. The scope type's `Drop`
>      impl is **deleted** (and with it the `thread::panicking()`
>      guard and the empty-list check). Each previously documented row
>      falls out: a send error commits nothing (the ruled
>      cancel-on-error, now structural); a user `Err` commits nothing;
>      a panic unwinds past the commit call, committing nothing; and
>      the async-cancellation prefix-commit hazard becomes
>      *unrepresentable* — a cancellation lands between polls, and the
>      closure runs inside one poll. Delete
>      `a_cancelled_batch_commits_its_prefix`
>      (`tests/single_peer.rs`) together with the hazard it pins, and
>      rewrite the `Batch` docs' lifecycle prose as what IS — the
>      batch commits iff the closure returns `Ok`, all-or-nothing — so
>      the "performance optimization, not an atomicity guarantee"
>      section inverts into a stated guarantee, with no ghost
>      references to the drop-driven semantics (provenance lives in
>      git).
>    - **Leak-proofing, all static** (the enforcement that makes the
>      no-await fiat real; every item is load-bearing): the
>      higher-ranked `for<'s>` bound with `R` and `E` quantified
>      outside it, so nothing borrowing through the handle can be
>      returned (futures included) and no outer variable can stash the
>      `&'s mut` (`'s` unifies with no outer lifetime); no `Clone`, no
>      `Default`, no public constructor on the scope type (no owned
>      escape, no `mem::swap` donor); fields private, public methods
>      exactly `send`/`redact`. On variance: in the signature above,
>      `'s` rides only on the `&'s mut` handle, whose inherent
>      invariance in its pointee plus the HRTB already close the
>      variance tricks — no marker is needed, and this simpler shape
>      is preferred. Only if the implementation instead threads the
>      scope lifetime *into* the type (a
>      `Batch<'s, 'env, T>` received as `&'s mut Batch<'s, 'env, T>`,
>      the literal `std::thread::scope` shape) does it also need that
>      pattern's invariance marker (`PhantomData<&'s mut &'s ()>`).
>      Pin the two principal leak vectors as `compile_fail` doctests
>      on `Rumors::batch` (stash into an outer `Option`; return the
>      handle) — doctests, so no new dev-dependency.
>    - **Re-entrancy**: the closure may call `rumors.send(...)` or
>      `rumors.batch(...)` on the same handle — building holds no
>      lock, and the outer commit runs only after the closure returns
>      — so nesting is sound and produces separate commits,
>      inner-before-outer: one doc sentence, one test.
>    - **Public rehydration constructors**: `Message::from_slice` and
>      `Message::from_bytes` gain the limit as an explicit parameter
>      (they have no peer context), passed to
>      `from_reader_with_recursion_limit`; pre-release, change the
>      signatures rather than minting `_with_limit` variants. This is
>      the trap the threading rule closes: an application on a raised
>      fleet limit must be able to rehydrate its own stored deep
>      messages, which the implicit default would refuse. The depth
>      failure surfaces through their existing `io::Result` as
>      `InvalidData` (ciborium's `RecursionLimitExceeded`), documented.
>      `Message::from_wire` (the wire path's ingress constructor) is
>      where step 3's limit-carrying deserializer lands; no separate
>      change. One admission sweep the erasure makes necessary: check
>      whether any *other* public `Message` constructor (`new`,
>      `from_arc`) can reach a peer's set — if one can, it takes the
>      same limit-checked path, else state the admission invariant
>      (only `Rumors::send`/`Batch::send`/wire ingress insert) where
>      the constructors are documented.
>    - **Shape suggestion**: mint a `PayloadDepthLimit` newtype
>      (newtypes over bare `usize` in public signatures, per house
>      style) carrying the default via `Default` and used uniformly by
>      the `Peer`/`Bootstrap` knob, `Batch`, `Message` constructors,
>      and the greeting codec.
>    - **The minted codec (ruled — proposed by Finch in review
>      follow-up, endorsed with one refinement, approved)**: push all
>      serde bounds to `Peer` construction by minting a payload
>      *serializer* there too, beside the deserializer, both carrying
>      the configured depth, and using them pervasively. The
>      refinement: a plain fn pointer cannot capture a runtime value,
>      so the concrete shape is a minted codec — a small `Copy` struct
>      pairing the two fn pointers with the `PayloadDepthLimit` field
>      — threaded wherever the deserializer travels today. What it
>      buys: the limit is unmissable (every `Message` creation and
>      every ingress parse in the peer's orbit goes through the one
>      codec value, closing the threading trap structurally rather
>      than by sweep); `T: Serialize` bounds drop from
>      `Rumors::send`/`Batch::send` (bounds concentrate at
>      construction, finishing for `Serialize` what the erasure did
>      for `DeserializeOwned`); and the greeting reads the limit off
>      the codec sessions already carry. Cost, accepted in the ruling:
>      `Peer` construction demands `T: Serialize` even for a peer that
>      never sends — symmetric with construction already demanding
>      `DeserializeOwned` to mint the deserializer (forwarding needs
>      neither bound, since gossip re-supplies cached bytes).
>      Accordingly `Message::try_new`'s body is the minted
>      serializer's target, and every "thread the limit" instruction
>      in steps 3–6 reads as "thread the codec".
>    - **Caller sweep**: the tutorial module, doctests, and every
>      in-tree `.send(`/`.batch(` use migrate to the closure form or
>      the single-action sugar, gaining `?`/`expect` as appropriate
>      (`Batch` has no consumers outside the tree, so the sweep ends
>      at the crate boundary).
>    - **Tests**: commit-on-`Ok` — a closure batching sends and a
>      redact commits once, atomically (observers see one wakeup); a
>      depth-violating `send` inside the closure, `?`-propagated,
>      commits **nothing**, earlier-queued actions included (tree
>      unchanged, no wakeup — the cancel-on-error pin); a user `Err`
>      return commits nothing; a panicking closure commits nothing
>      (`catch_unwind` in the test); the re-entrancy ordering test;
>      the two `compile_fail` leak doctests; and `from_slice` at a
>      raised limit rehydrates a deep message that the default-limit
>      call rejects (both directions asserted).
> 6. **Parameter equality at the handshake (ruled — no longer a
>    sub-decision).** The greeting carries the sender's configured
>    limit, and a session proceeds only if the two values are equal;
>    a mismatch in either direction is a typed, unconditional abort
>    after the greetings are exchanged and before anything else — in
>    particular before the equal-versions early return, so mixed
>    configurations surface even on converged, no-op sessions.
>    - *Wire*: a new entry in the greeting map. Suggested key:
>      `"payload_depth_limit"`; recompute the deterministic key order
>      (length-first, then bytewise — at 19 characters it ties
>      `"target_message_size"` on length and sorts before it on
>      content) and update both the `KEYS` roster in
>      `codec/greeting.rs` and `parse_greeting`'s exact-roster check
>      (which becomes a seven-entry map). This is a deliberate,
>      owner-ruled pre-release wire format change: re-accept the V2
>      snapshot corpus in the implementing commit, naming this change;
>      re-run `tools/digestshare` and update the corpus figures in
>      `design/cbor-legible-wire.md`; run the `tests/dispute_wire.rs`
>      cells (the few-byte greeting growth amortizes to well under
>      their bands at 8,192 divergent messages — verify, don't
>      assume); add the greeting-table row and a decision-record entry
>      for this ruling to `design/cbor-legible-wire.md`.
>    - *Check placement*: in `proxy/start.rs`, in both
>      `complete_connect` and `accept`, immediately after both
>      greetings are in hand and before `connected()` runs its
>      equal-versions resolution. Both sides detect symmetrically,
>      like `NetworkMismatch`.
>    - *Error*: a new typed public variant per the taxonomy ruling
>      (errors name what they diagnose), e.g.
>      `Error::PayloadDepthMismatch { local, remote }`, documented in
>      the `Error` table ("fix the configuration: the limit is a
>      fleet-wide parameter; align it and reconnect").
>    - *V1*: the frozen greeting cannot carry the parameter, so V1
>      sessions keep decode-side-only enforcement; the knob's docs
>      state that content-conditional failure remains possible on the
>      legacy dialect.
>    - *Achieved invariant*, worth stating in the knob's rustdoc: with
>      send-side admission (step 4) plus handshake equality, no V2
>      session between conforming peers can fail on payload depth at
>      all — over-deep values are rejected at their author at the
>      moment of choice, and mismatched fleets are rejected at hello.
>      Changing the limit is therefore a fleet-coordinated
>      configuration event, like changing the selected [`Protocol`] —
>      document it in that register, not as a tuning knob.
> 7. **Tests to commit** (beyond the differential in step 4): a
>    boundary pin — a payload at exactly the default depth round-trips
>    peer-to-peer over an in-memory link; one level deeper is rejected
>    at send with the typed error; a decode-side ingress test feeding
>    a hand-crafted over-deep record through the codec test helpers (a
>    nonconforming sender must still die typed at ingress, since
>    send-side enforcement only binds this crate's own API); and the
>    handshake-equality pair — two peers with different limits abort
>    with the typed mismatch on both sides and open no data stream
>    (assert via the capture harness or the observation hook), plus an
>    equal-raised-limits control that gossips clean.
> 8. **Docs.** The knob's rustdoc states: what counts as a nesting
>    scope (by reference to the differential test as the accounting's
>    pin, not a prose transcription), the three enforcement points and
>    the achieved invariant from step 6 (send rejects at the author;
>    handshake rejects mismatched fleets; ingress rejects
>    nonconforming implementations), the default and its rationale,
>    and the fleet-coordination framing for changing the value. Sweep
>    the rest of the rustdoc for drop-commit language about batches
>    (crate docs, `Rumors` method docs, the tutorial) and restate it
>    in the commit-on-`Ok` form. If crate-level docs change, re-derive
>    READMEs (`just readme`).
> 9. **Sequencing against the review-fix round.** Every resolution in
>    this packet's findings sections moves zero wire bytes (B1 and B2
>    assert zero snapshot movement in their acceptance criteria), so
>    the review fixes and this feature can land in either order — but
>    do not interleave them: this feature's greeting change is the
>    sole snapshot re-accept in flight, and per the hard rules the
>    re-accepting commit must contain exactly that deliberate change
>    and name it. If R1's per-cell re-pinning lands *after* this
>    feature, measure its cells at the then-parent before pinning
>    (attribution discipline: never fold this feature's greeting
>    growth, however sub-band, into R1's recorded numbers).
> 10. **Record of decisions.** This package (the knob, symmetric
>     enforcement, greeting equality, and the closure-scoped batch)
>     has outgrown a review packet; give it a small design document
>     beside the wire doc whose decision record transcribes the
>     rulings currently held only here: the depth limit as a property
>     of the shared set (hence pairwise equality, not negotiation —
>     with the negotiate-down unsoundness argument); eager `Message`
>     creation with fallible `send`; a failed batch commits nothing;
>     the closure scope as the no-await mechanism, recording the
>     rejected fiats (`!Send` binds only futures that must be `Send`
>     and misstates the type; `#[must_not_suspend]` is unstable on the
>     pinned toolchain; clippy's `await-holding-invalid-types` binds
>     only in-repo runs) and the commit-on-`Ok` atomicity inversion;
>     the 256 default's rationale; and the V1 carve-out. Per the house
>     rules, the design doc cites code, code cites nothing back, and
>     the knob's rustdoc carries every invariant inline; this packet
>     remains as review provenance only.
