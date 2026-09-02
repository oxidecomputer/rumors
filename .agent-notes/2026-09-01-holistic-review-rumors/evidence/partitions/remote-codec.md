# Partition remote-codec: The remote wire codec: budgets, decode (sync and async), encode, errors, frames, greeting, signals

## Partition summary

The remote codec is the frame grammar every logical stream of the streaming mirror speaks. A frame is one CBOR array item, `[stream, state]` or `[stream, state, body]`: the opener's two unsigned ints name the logical stream and the signal's state code, validated against a per-speaker phase schedule (`signal.rs`); a body is either a canonical `{radix: hash}` listing map (queries) or a tag-63 byte string holding a run of leaf records (supplies), the records kept encoded and decoded lazily (`frame.rs`). `budget.rs` derives the default run budget in closed form and defines the one whole-frame boundary (`covers`, with `admits` as `covers` of the grown body) that the encoder's flush rule and both decoders' ingress gate share. `encode.rs` renders the fixed heads on the stack and `encode/async_io.rs` writes the pieces straight to the transport; `decode/async_io.rs` is the production reader, which reads exactly (never a byte of the next frame), batches the opener and the listing entries into bulk reads, and holds supply frames to the negotiated budget from the first record's heads before buffering the body. `decode.rs` holds the shared validators plus a `#[cfg(test)]` synchronous decoder that the tests hold the async reader to through `decode_both`. `greeting.rs` spells the tag-24 greeting item, and `error.rs` is the typed taxonomy that reaches users as `rumors::error::{CodecDecodeErrorKind, ...}`.

I read all sixteen partition files with line numbers, 5072 lines in total, of which the six `tests.rs` siblings are test code (`budget/tests.rs` 91, `decode/tests.rs` 1108, `encode/tests.rs` 236, `frame/tests.rs` 182, `greeting/tests.rs` 213, `signal/tests.rs` 168: 1998 lines). Production code is `codec.rs`, `budget.rs`, `decode.rs`, `decode/async_io.rs`, `encode.rs`, `encode/async_io.rs`, `error.rs`, `frame.rs`, `greeting.rs`, and `signal.rs`, though about 330 of those lines are `#[cfg(test)]` or `test-internals` scaffolding. To settle points I also read `src/tree/mirror/framing.rs`, `src/tree/mirror/streaming/remote/error.rs`, the export list in `src/error.rs`, parts of `cbor.rs`, `observe.rs`, `peer.rs`, `link.rs`, `height.rs`, `error_atlas.rs`, the review packet under `.agent-notes/2026-08-20-cbor-wire-review/`, and the pinned-version sources of `ciborium` 0.2.2, `bytes` 1.11.1, `tokio` 1.52.3, and the 1.97.1 standard library.

The code is in good shape. The invariants that matter are each enforced in one place and pinned by a committed test: one ingress gate for both listing surfaces (`ListingBuilder`), one budget boundary for encoder and decoders, closed-form wire constants each pinned against an actual encode, a lazy run whose per-frame memory bound is one run's bytes, an over-budget gate that decides legality before buffering (with the truncated-after-heads case in the proptest that distinguishes an early decision from buffer-then-check), and a stated ingress spelling boundary protected by three tests that say so in their docs. Every `expect` and `unreachable!` in production code carries a one-line proof that is true of the surrounding code, both async entry points document cancel safety concretely, and the error strings read as plain English.

The dominant issues are three. First, today's three commits (the two-item opener, the bulk reads, the stack-rendered heads) left small residues: a public type leak (`Signal` through `InvalidSignalPlacement::signal`), four spellings of the opener length, hand-maintained counts in the module doc, and two latent correctness defects in the bulk-read paths that the sync-oracle differential cannot see because no committed test drives a failing `AsyncRead` or a listing head defect through the async reader. Second, the error taxonomy is uneven: the frame decoder flattens typed head and listing defects into static strings while every sibling taxonomy keeps them typed, a public `GreetingError::Order` variant is unreachable through any public path, and most variants of the public enums carry no doc. Third, one contract breach: the over-budget lone-record read resumes into a `Vec` whose slack capacity lets `read_buf` take bytes past the declared run, breaching the exactness guarantee two docs state; a conforming encoder cannot produce the triggering record, so it is latent, but the clause is real and the clamp is cheap. The rest is simplification and prose: a greeting parser whose roster-loop shape forces eight panic sites, hand-written derives left from the erased type parameter, a stale public variant doc, and a handful of register tells.

## Findings

### remote-codec-1: Hand-maintained counts and rosters restate constants and tests
- Where: src/tree/mirror/streaming/remote/codec.rs:18-27 (related: src/tree/mirror/streaming/remote/codec/signal/tests.rs:9-10 and :146; src/tree/mirror/streaming/remote/codec/budget/tests.rs:3-4; src/tree/mirror/streaming/remote/codec/frame.rs:424-426; src/tree/mirror/streaming/remote/codec/decode/tests.rs:798-806; outside the partition, src/tree/mirror/streaming/remote.rs:13 and :18)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep `162\b|163\b` over src hits only remote.rs:18, codec.rs:25-26, and `VALID_PLACEMENTS` at signal/tests.rs:7; every cited line read)
- Seen by: structure (15), prose (18, 31); refutation: confirmed; history: contradicts CLAUDE.md Principle 5 doctrine; the module-doc counts were written by 3327a92b and are currently accurate
- Owner-gated: no

The module doc states the stream count, the state count, and the two per-speaker placement totals as literals; two testdocs restate the state and stream counts and one the radix fan; `ListingBuilder`'s doc enumerates its three callers; and the doc of `supply_truncation_at_chunk_boundaries_is_typed` re-lists the cut offsets that `chunk_boundary_cuts` centralizes "so the boundary roster cannot drift". Each is a copy of a fact the code can change without touching the prose, and each already has a mechanically enforced home: `Stream::COUNT`, `Signal::STATE_COUNT`, `FAN`, `VALID_PLACEMENTS` asserted by `placements_match_the_phase_schedule_exhaustively`, and `chunk_boundary_cuts` itself.

Evidence:

    18	//! A frame opens with two unsigned ints: the index of the logical stream
    19	//! it rides (one of 17) and its signal's state code (one of ten frame
    20	//! states — four reaction forms, each continuing or ending its reply,
    ...
    25	//! select a phase-specific subset of states: the initiator admits 162
    26	//! placements and the responder 163, rejecting the rest before their
    27	//! frame body is read.

    signal/tests.rs:9	/// The state roster is a bijection between the ten signals and the state
    signal/tests.rs:10	/// codes 0 through 9; every other code is reserved.
    signal/tests.rs:146	/// Both elected speakers map their 17 stream indices bijectively to schedule heights.
    frame.rs:425	/// greeting's root-fan listing — and whichever reader drives it (the
    frame.rs:426	/// async decoder, the sync oracle, or the slice parser). The map's
    decode/tests.rs:801	/// The seeded offsets are one byte short of, exactly on, and one byte
    decode/tests.rs:802	/// past each payload chunk boundary, plus the zero-byte, one-byte, and
    decode/tests.rs:803	/// one-short-of-total cuts. The chunked body read preserves the typed

Resolution: In `codec.rs:18-27`, drop "(one of 17)" and "one of ten" (the structure is already stated) and replace "the initiator admits 162 placements and the responder 163" with "each speaker admits a phase-specific subset, pinned by `placements_match_the_phase_schedule_exhaustively` and `invalid_placement_snapshot`". In `signal/tests.rs:9-10` and `:146` cite `Signal::STATE_COUNT` and `Stream::COUNT`; in `budget/tests.rs:3` write "`FAN` full-fan query frames"; in `frame.rs:424-426` say "whichever reader drives it" without the roster; in `decode/tests.rs:798-806` say "every cut in `chunk_boundary_cuts`'s roster" and repeat none of the offsets (the first sentence, "cuts at every seeded offset all classify", also wants a rewrite). Apply the same edit at `remote.rs:13` and `:18` (outside this partition). Acceptance: `grep -n "162\|163" src/tree/mirror/streaming/remote` hits only `VALID_PLACEMENTS`; no literal 17, ten, or 256 remains in codec prose outside the enforcing constant or test; the truncation testdoc names the helper and none of its offsets.

### remote-codec-2: "trust boundary" and "honest encoder" in an honest-peer model, plus smaller register tells
- Where: src/tree/mirror/streaming/remote/codec.rs:58-61 (related: src/tree/mirror/streaming/remote/codec/encode.rs:82; src/tree/mirror/streaming/remote/codec/frame.rs:515; src/tree/mirror/streaming/remote/codec/decode/async_io.rs:434-436; codec.rs:29; src/tree/mirror/streaming/remote/codec/budget/tests.rs:74 and :89; src/tree/mirror/streaming/remote/codec/decode/tests.rs:804)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep "trust boundary|honest encoder" over src returns exactly the four codec sites; the smaller tells located by reading)
- Seen by: prose (29); refutation: confirmed; history: contradicts the AGENTS.md hard rule (a59dc786); codec.rs:60 predates the rule, the other three sites postdate it, and ed0f1775's own message uses the calibrated phrase "conformance-bug detector"
- Owner-gated: no

Three docs call decoding "the trust boundary" (and the encoder "not a trust boundary") and one comment reasons about "an honest encoder", in a crate whose model of record is authenticated honest peers and whose fail-fast machinery is a conformance-bug detector, not a security boundary. `budget.rs:28` already has the calibrated wording ("a conformance-buggy peer"). Smaller tells: "orthogonal" for independent, "real bound" and "genuinely binds" as moralizers, "at every seam" as a promoted metaphor.

Evidence:

    58	//! Encoding trusts the protocol and adapter to produce phase-correct,
    59	//! canonically ordered frames; it performs no redundant semantic validation.
    60	//! Decoding is the trust boundary and validates every peer-controlled signal,

    encode.rs:82	/// The encoder is not a trust boundary: phase placement, query ordering, and
    frame.rs:515	/// The encoder is not a trust boundary — callers guarantee canonical
    async_io.rs:434	            // The frame outsizes the budget the peer's encoder flushes
    async_io.rs:435	            // within, so the one shape an honest encoder can still have
    budget/tests.rs:74	/// maximum is refused, so the saturated budget is a real bound, not a
    budget/tests.rs:89	    // Negative control: the ceiling genuinely binds.

Resolution: "Decoding is where conformance is checked" / "The encoder performs no conformance checks" / "a conforming encoder"; "independent" for "orthogonal"; drop "real" and "genuinely"; "at every chunk boundary" for "at every seam". Acceptance: `grep -rn "trust boundary\|honest encoder" src/tree/mirror/streaming/remote/codec` returns nothing; the other sites read in plain terms.

### remote-codec-3: Wire quantities spelled several times: the opener length, the supply head, the record item length, the stream count, the radix fan
- Where: src/tree/mirror/streaming/remote/codec/budget.rs:86-89 (related: budget.rs:53-55; src/tree/mirror/streaming/remote/codec/encode.rs:24-31; src/tree/mirror/streaming/remote/codec/decode/async_io.rs:37-39; src/tree/mirror/streaming/remote/codec/frame.rs:155-160, :179-181, :325-328, :19; src/tree/mirror/streaming/remote/codec/frame/tests.rs:8-10; src/tree/mirror/streaming/remote/codec/signal.rs:31-32; outside the partition, src/link.rs:161-169 and src/tree/mirror/streaming/window.rs:132)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read; the stream-count derivation checked against `height.rs:128-130`, where `Root` is `S` applied 32 times to `Z`, so `32 / 2 + 1 = 17`)
- Seen by: structure (6, 7, 11), perfapi (52); refutation: confirmed all four; history: the opener spellings accreted across 3327a92b and 6f3792ab with no rationale; the `record_len`/`push` pair is deliberate with its rationale inline (frame.rs:148-151) and pinned by f2b74a97; `COUNT = 17` is from 2203c104 with no rationale; f94f2056's message treats `FAN` and `MAX_QUERY_CHILDREN` as distinct quantities that share the radix
- Owner-gated: no (the `FAN`/`MAX_QUERY_CHILDREN` unification is taste; ask first)

One quantity, several spellings. `cbor::head_len(3) + WireSignal::ENCODED_LEN` is the opener's length and appears as `FRAME_HEAD_LEN` (encode.rs:26), as `OPENER_LEN` (async_io.rs:39), and inline twice in budget.rs (:53-54 and :86-87); `head_len(TAG_CBOR_SEQUENCE) + head_len(u32::MAX as u64)` is `SUPPLY_HEAD_LEN` (encode.rs:31) and is spelled out again inside `SUPPLY_FRAME_OVERHEAD`. `RECORD_TAG_LEN + head_len(body) + body` is computed in `record_len`, again in `push`, and a third time in u64 in `lone_record_spans`. `Stream::COUNT` is the literal 17 six lines below the two constants it follows from, with the derivation living only in `link.rs` prose; `MAX_QUERY_CHILDREN = 256` and `window::FAN = 256` name the same radix fan and `budget.rs` imports both. A reader cannot see that the encoder's stack buffer, the reader's opener buffer, and the budget envelope are one quantity without expanding each by hand.

Evidence:

    86	pub const SUPPLY_FRAME_OVERHEAD: usize = cbor::head_len(3)
    87	    + WireSignal::ENCODED_LEN
    88	    + cbor::head_len(cbor::TAG_CBOR_SEQUENCE)
    89	    + cbor::head_len(u32::MAX as u64);

    encode.rs:26	const FRAME_HEAD_LEN: usize = cbor::head_len(3) + WireSignal::ENCODED_LEN;
    async_io.rs:39	const OPENER_LEN: usize = cbor::head_len(3) + WireSignal::ENCODED_LEN;
    frame.rs:157	        RECORD_TAG_LEN
    frame.rs:158	            .saturating_add(cbor::head_len(body as u64))
    frame.rs:159	            .saturating_add(body)
    frame.rs:179	        let item = RECORD_TAG_LEN
    frame.rs:180	            .saturating_add(cbor::head_len(body as u64))
    frame.rs:181	            .saturating_add(body);
    signal.rs:32	    pub const COUNT: u8 = 17;

Resolution: Define the opener length once (beside `WireSignal::ENCODED_LEN` in signal.rs, or in frame.rs) and the supply head length once (beside `RECORD_TAG_LEN`), then `SUPPLY_FRAME_OVERHEAD = OPENER_LEN + SUPPLY_HEAD_LEN`, `FULL_FAN_QUERY_FRAME_LEN` starts from `OPENER_LEN`, and `Heads<OPENER_LEN>` / `Heads<SUPPLY_HEAD_LEN>` use the shared names. Add `const fn record_item_len(content: usize) -> usize` (saturating) and have `record_len` and `push` call it; `lone_record_spans` compares through a u64 twin or a `usize::try_from`. The pin `record_len_matches_an_actual_push` stays meaningful because it compares the closed form against bytes actually written, not against a second arithmetic. Write `pub const COUNT: u8 = (STREAMED_HEIGHT_COUNT / STREAM_HEIGHT_STRIDE + 1) as u8;` with a `const _: () = assert!(...)` that it fits, keeping `link::STREAM_COUNT` as the transport's own literal and the existing cross-layer pin. Ask whether `MAX_QUERY_CHILDREN` should be defined as `FAN` (or both from one radix constant); if they are meant as distinct quantities, say so at one of the two declarations. Acceptance: exactly one definition each of the opener length, the supply head length, and the record item sum in the codec; no bare `17` in signal.rs; `default_budget_matches_its_derivation` (pinned 1_830_400), `full_fan_frame_len_matches_an_actual_encode`, `record_len_matches_an_actual_push`, and `stream_count_matches_the_codec` pass unchanged.

### remote-codec-4: "pre-batching" names a wire state that no longer exists
- Where: src/tree/mirror/streaming/remote/codec/budget.rs:108-110 (related: none)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: the partition's only occurrence; `git log -S"pre-batching"` on the file names only f94f2056, the commit that introduced batching)
- Seen by: prose (19); refutation: confirmed; history: contradicts the AGENTS.md hard rule; the phrase predates the rule (a59dc786) and no sweep caught it
- Owner-gated: no

`RunBudget`'s doc describes a zero budget by comparison to the wire before batching landed. The present-tense fact is one leaf per frame; the comparison is history the tree should not carry.

Evidence:

    108	/// [`admits`](Self::admits). Any value, including zero, is safe: the
    109	/// minimum-one-record rule keeps every leaf shippable, degrading a zero
    110	/// budget to the pre-batching one-leaf-per-frame wire traffic, and the

Resolution: "degrading a zero budget to one leaf per frame, and the". Acceptance: `grep -rn pre-batching src` returns nothing.

### remote-codec-5: The effective run budget is not observable: saturation is silent, there is no getter, and no session stat records the negotiated minimum
- Where: src/tree/mirror/streaming/remote/codec/budget.rs:128-132 (related: budget.rs:102; src/peer.rs:537-551; src/tree/mirror/streaming/stats.rs:106)
- Class / severity / confidence: feature-gap / low / high
- Provenance: verified (grep: `MAX_RUN_BUDGET_BYTES` is re-exported nowhere; `Peer`'s `&self` methods are `network`, `warm_caches`, `dangerously_alias_party` at peer.rs:304, :714, :728; `stats.rs` mentions the budget only in doc links)
- Seen by: perfapi (47); refutation: confirmed; history: the knob shipped in f94f2056 with no read-back; no note discusses a getter or stat
- Owner-gated: yes (public API addition)

`Peer::target_message_size(bytes)` stores `RunBudget::from_bytes(bytes)`, which clamps to `MAX_RUN_BUDGET_BYTES`; that constant is `pub` inside a private module and its value is described only in prose at peer.rs:540-543. A user tuning batching across a mixed fleet can neither read back the value a peer runs at nor learn which end's setting won the session minimum when frames come out smaller than configured.

Evidence:

    128	    pub fn from_bytes(bytes: usize) -> Self {
    129	        Self {
    130	            bytes: bytes.min(MAX_RUN_BUDGET_BYTES),
    131	        }
    132	    }

Resolution: Re-export `MAX_RUN_BUDGET_BYTES` beside `DEFAULT_TARGET_MESSAGE_SIZE` and cite it by name in `Peer::target_message_size`'s doc; add a `Peer` getter returning the saturated value as the greeting advertises it; consider a `SessionStats` field for the negotiated minimum. Acceptance: a test sets a target above the cap and reads back `MAX_RUN_BUDGET_BYTES` through the public getter; a two-peer session with unequal targets reports the minimum in both sides' stats if the field lands.

### remote-codec-6: The second assertion in `default_budget_matches_its_derivation` cannot fail
- Where: src/tree/mirror/streaming/remote/codec/budget/tests.rs:7-14 (related: src/tree/mirror/streaming/remote/codec/budget.rs:165-169 and :128-132; budget/tests.rs:59-64)
- Class / severity / confidence: test-quality / nit / high
- Provenance: assessed (read)
- Seen by: prose (24); refutation: confirmed; history: deliberate-but-expired (f94f2056 asserted `.bytes() == DEFAULT_TARGET_MESSAGE_SIZE`; 8e1ed47a rewrote it when `bytes()` was removed as unused; bdf74d45 re-added `bytes()`, and the later saturation cap gave the original form real content)
- Owner-gated: no

`RunBudget::default()` is defined as `Self::from_bytes(DEFAULT_TARGET_MESSAGE_SIZE)`, so asserting it equals `RunBudget::from_bytes(DEFAULT_TARGET_MESSAGE_SIZE)` passes for any `from_bytes`. Asserting `.bytes() == DEFAULT_TARGET_MESSAGE_SIZE` would pin that saturation does not clip the default, which `default_budget_fits_the_framing_header` only approximates through `u32::try_from`.

Evidence:

    10	    assert_eq!(
    11	        RunBudget::default(),
    12	        RunBudget::from_bytes(DEFAULT_TARGET_MESSAGE_SIZE)
    13	    );

    budget.rs:166	    fn default() -> Self {
    budget.rs:167	        Self::from_bytes(DEFAULT_TARGET_MESSAGE_SIZE)

Resolution: Replace with `assert_eq!(RunBudget::default().bytes(), DEFAULT_TARGET_MESSAGE_SIZE);` and consider folding `default_budget_fits_the_framing_header` into it. Acceptance: the test fails if saturation ever clips the default.

### remote-codec-7: Test-only code is interleaved through the production codec files
- Where: src/tree/mirror/streaming/remote/codec/decode.rs:3-61 (related: decode.rs:63-215; src/tree/mirror/streaming/remote/codec/encode.rs:3-4, :17-22, :33-44, :159-180; src/tree/mirror/streaming/remote/codec.rs:105-240)
- Class / severity / confidence: modularity / low / medium
- Provenance: assessed (read)
- Seen by: structure (8); refutation: confirmed; history: the `#[cfg(test)]` gate on `FrameDecoder` was added by a WIP commit (83edcd94) when the async reader took over, never as a placement decision; the meter shims arrived with 90512e88 without saying why they live in codec.rs
- Owner-gated: no

`decode.rs` is half test oracle: the imports at 3-4, 12-15, 21-22, the module's headline `decode` and `decode_exact`, and the whole `FrameDecoder` (about 190 of 378 lines) are `#[cfg(test)]`, while the production reader lives in the child `async_io.rs`. `encode.rs` carries the `#[cfg(test)]` sync `encode`, `FrameEncoding::write`, and `write`; `codec.rs` spends lines 105-240 on `test-internals` meter scaffolding. A reader of the production path skips cfg gates to find it, and the parent module's headline function never ships. The project convention keeps tests in sibling files "for brevity of reading the implementation"; oracles and meter scaffolding are test support and deserve the same separation. The crate already has the shape: `capture` is a cfg-gated sibling module (codec.rs:66-67).

Evidence:

    54	/// Frame reader that adds protocol context as soon as the signal reveals it.
    55	#[cfg(test)]
    56	struct FrameDecoder<'a, R> {
    57	    speaker: Speaker,
    58	    /// The session's run budget, gating supply-body buffering.
    59	    budget: RunBudget,
    60	    read: &'a mut R,
    61	}

Resolution: Move the sync oracle into `#[cfg(test)] mod oracle;` under `decode/` (its re-exports are consumed by several sibling test modules, so `decode/tests.rs` is the wrong home), keeping the shared validators (`frame_arity`, `opener_item`, `check_arity`, `query_listing`, `run_head`, `head_error`, `listing_issue`, `decode_signal`) in `decode.rs`; give the struct a doc stating its role as the differential's independent implementation. Move the sync `encode` and `FrameEncoding::write` into a cfg-gated `encode/oracle.rs`; move the meter scaffolding into `#[cfg(any(test, feature = "test-internals"))] mod meters;` beside `capture`, re-exporting from `codec.rs` as today. Acceptance: `decode.rs`, `encode.rs`, and `codec.rs` contain no `#[cfg(test)]` items other than `mod` and `pub use` lines; the gate passes with the same test set.

### remote-codec-8: Decode fragments duplicated between the async reader and the sync oracle: the over-budget gate, `record_prefix`, the EOF classification, and a bare `RECORD_TAG_LEN + 1`
- Where: src/tree/mirror/streaming/remote/codec/decode.rs:147-163 (related: src/tree/mirror/streaming/remote/codec/decode/async_io.rs:444-457; decode.rs:183-196 and async_io.rs:473-486; decode.rs:207-213, :338-344, async_io.rs:204-212, :543-551)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read; each pair compared line by line)
- Seen by: structure (9); refutation: confirmed; history: the mirroring is a recorded choice (ed0f1775: "the shared lone_record_spans predicate keeps the two decoders' boundary identical"), but nothing says why only that predicate was shared; the stated purpose is served better by sharing more
- Owner-gated: no

The over-budget gate (the `OverbatchedRun` closure, the `len < RECORD_TAG_LEN + 1` short-body check, the `lone_record_spans` test) is repeated verbatim in both decoders; `record_prefix` is repeated differing only by `.await`; the mapping "UnexpectedEof becomes Truncated, anything else Read" is spelled four times; and the `+ 1` (the smallest byte-string head) is an unnamed number at both sites. The oracle's documented independent contribution (decode.rs:169-172) is the whole-body read shape; none of these fragments is about read shape, so sharing them costs no oracle independence.

Evidence:

    147	        if !self.budget.covers(len) {
    148	            let budget = self.budget;
    149	            let overbatched = move || DecodeErrorKind::OverbatchedRun {
    150	                declared: super::budget::SUPPLY_FRAME_OVERHEAD.saturating_add(len),
    151	                budget: budget.bytes(),
    152	            };
    153	            // A body too short to hold a record's heads cannot be a lone
    154	            // record: rejected on the declared length alone.
    155	            if len < super::frame::RECORD_TAG_LEN + 1 {
    156	                return Err(overbatched());
    157	            }

    async_io.rs:445	            let overbatched = move || DecodeErrorKind::OverbatchedRun {
    async_io.rs:446	                declared: super::super::budget::SUPPLY_FRAME_OVERHEAD.saturating_add(len),
    async_io.rs:449	            if len < super::super::frame::RECORD_TAG_LEN + 1 {

Resolution: In `budget.rs` add `pub(super) fn overbatched(self, body: usize) -> DecodeErrorKind` (or a free function in decode.rs), and in `frame.rs` a `pub(super) const MIN_RECORD_HEADS_LEN: usize = RECORD_TAG_LEN + 1` with a doc naming it as the tag head plus a one-byte byte-string head; have both decoders call them. Hoist `classify` to `decode.rs` as `pub(super)` and route `head_error`'s `Io` arm and the sync `read_exact` through it (`Arrived::short` can call it with a fresh `UnexpectedEof`). Acceptance: `OverbatchedRun { .. }` is constructed in exactly one place; `RECORD_TAG_LEN + 1` appears nowhere as a bare expression; `ErrorKind::UnexpectedEof => DecodeErrorKind::Truncated` appears once; the `decode_both` suites pass.

### remote-codec-9: The frame decoder flattens typed head and listing defects into static strings; every sibling taxonomy keeps them typed
- Where: src/tree/mirror/streaming/remote/codec/decode.rs:316-360 (related: src/tree/mirror/streaming/remote/codec/error.rs:137-143; src/tree/mirror/streaming/remote/codec/frame.rs:381-387 and :404-419; src/tree/mirror/streaming/remote/codec/greeting.rs:102-110; src/tree/mirror/streaming/remote/codec/decode/async_io.rs:531-538; src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:105-110)
- Class / severity / confidence: simplification / medium / high
- Provenance: assessed (read)
- Seen by: structure (1), perfapi (open question); refutation: confirmed; history: `listing_issue` and `head_detail` arrived whole in 4dd2053c with no rationale for the shape; the atlas exemption states the collapse as a fact; R5 typed only the greeting side
- Owner-gated: yes (`DecodeErrorKind` reaches users as `rumors::error::CodecDecodeErrorKind`)

`listing_issue` and `head_detail` exist only to convert `ListingIssue` and `HeadError` into `DecodeErrorKind::Malformed { detail: &'static str }`, discarding which `HeadError` fired (`ListingIssue::Head(_)` becomes the fixed string "listing head is not canonical") and re-spelling `ListingIssue::Truncated`'s own message. `LeafRunError::Head { source: HeadError }`, `GreetingError::Head(HeadError)`, and `GreetingError::Listing(ListingIssue)` all keep the defect typed, so the two listing ingress surfaces (query frame and greeting) type the same failure differently, and the error atlas carries an exemption explaining the collapse. Types-first: a typed source carries strictly more than a string re-spelling of its `Display`, and the two mapping functions are machinery whose only job is to lose information.

Evidence:

    317	pub(super) fn listing_issue(issue: ListingIssue) -> DecodeErrorKind {
    318	    match issue {
    319	        ListingIssue::Order(order) => DecodeErrorKind::QueryOutOfOrder(order),
    320	        ListingIssue::Head(_) => DecodeErrorKind::Malformed {
    321	            part: FramePart::QueryChildren,
    322	            detail: "listing head is not canonical",
    323	        },
    ...
    353	fn head_detail(error: cbor::HeadError) -> &'static str {
    354	    match error {
    355	        cbor::HeadError::Truncated => "truncated head",

    error_atlas.rs:106	        "kind: Listing",
    error_atlas.rs:107	        "ListingIssue never surfaces from the frame decoders: they collapse \

Resolution: Add typed variants to `DecodeErrorKind`: `Head { part: FramePart, #[source] source: HeadError }` (dissolving `head_detail`) and `InvalidListing(#[from] ListingIssue)` (dissolving `listing_issue`; `QueryOutOfOrder` then duplicates `InvalidListing(ListingIssue::Order(_))` and can retire, or stay as the flat form if matchers rely on it). Keep `Malformed { part, detail }` for the shape-level cases ("frame item is not an array", "query body is not a listing map"). Re-accept the error-atlas snapshot deliberately and delete its `"kind: Listing"` exemption. This also dissolves remote-codec-10. Acceptance: `listing_issue` and `head_detail` no longer exist; a widened listing key head decodes to an error carrying `HeadError::NotShortest` as a typed source in both the frame and greeting paths; the atlas covers the new variants without an exemption; `tests/common/sim.rs` and the proxy tests that match `CodecDecodeErrorKind` compile.

### remote-codec-10: The async and sync decoders spell a non-canonical listing entry head differently, and no differential test drives listing defects through the async reader
- Where: src/tree/mirror/streaming/remote/codec/decode.rs:320-323 (related: src/tree/mirror/streaming/remote/codec/decode/async_io.rs:531-538; decode.rs:198-202 and :336-360; src/tree/mirror/streaming/remote/codec/decode/tests.rs:565-629 and :844-850)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read; both paths traced to their detail strings, and `kind_signature` read to confirm `Malformed` is compared through `{:?}`)
- Seen by: correctness (34); refutation: confirmed and reframed (kind agrees, detail differs); history: introduced today by 18527932's `partial_head`, whose message claims the sync-oracle differential "hold[s] as [it was]"; true only because no `decode_both` test constructs a listing head defect
- Owner-gated: no

Inside a listing, the async reader classifies a widened, indefinite, or reserved entry head through `partial_head` into `listing_issue(ListingIssue::Head(e))`, which emits `detail: "listing head is not canonical"`; the sync oracle reads the same head through `self.head(FramePart::QueryChildren)`, then `head_error` and `head_detail`, emitting "head not in shortest form" (or the indefinite/reserved text). Both are `Malformed { part: QueryChildren }`, so the kind agrees, but `decode_both` compares the full `Debug` and would panic on this input, which no committed test constructs: `unordered_query_is_rejected` and `empty_query_listing_is_rejected` run through `decode_exact` alone. The `FrameRead` doc promises a defect "is classified exactly as a reader fetching one head at a time would classify it"; the oracle is that reader, and the differential exists to hold the two together. An oracle that would disagree, with no test asking it, is the blind spot.

Evidence:

    320	        ListingIssue::Head(_) => DecodeErrorKind::Malformed {
    321	            part: FramePart::QueryChildren,
    322	            detail: "listing head is not canonical",
    323	        },

    async_io.rs:535	        Err(error) => Err(listing_issue(super::super::frame::ListingIssue::Head(
    decode.rs:358	        cbor::HeadError::NotShortest => "head not in shortest form",
    decode/tests.rs:578	        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();

Resolution: Make `listing_issue(ListingIssue::Head(head))` use `head_detail(head)` so the two paths agree byte for byte (remote-codec-9 subsumes this by typing the source). Then route `unordered_query_is_rejected` and `empty_query_listing_is_rejected` through `decode_both`, and add a proptest over listing-head spellings (widened key `[0x18, r]` for r < 24, widened value head `[0x59, 0x00, 0x18]`, indefinite `0x5f`, reserved `0x1c`) through `decode_both`. Acceptance: before the one-line fix, the widened-key case panics inside `decode_both` ("the two decoders classify the failure differently"); after it, every listing-defect test passes through `decode_both`, and the atlas snapshot for `query/listing-key` is unchanged (a `Shape` defect, untouched).

Construction: Query frame on stream 5: opener `[0x83, 0x05, Signal::Query(Flow::Continue).state()]` (state 4), map head `0xa1`, key `[0x18, 0x05]` (radix 5 in widened form), value head `[0x58, 0x18]`, 24 digest bytes. `decode_both(speaker, RunBudget::default(), &bytes)`: the sync path yields `Malformed { part: QueryChildren, detail: "head not in shortest form" }`, the async path `Malformed { part: QueryChildren, detail: "listing head is not canonical" }`; `kind_signature` formats both with `{:?}` and the assertion at decode/tests.rs:871-875 fails.

### remote-codec-11: The opener bulk read drops a transport error that arrives after a partial fill
- Where: src/tree/mirror/streaming/remote/codec/decode/async_io.rs:152-161 (related: async_io.rs:249-267, :50-63, :293-311)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read; the control flow traced from `fill` through `Pending`/`Exact::head`)
- Seen by: correctness (33); refutation: confirmed; history: today's 18527932; the inline comment covers only the clean-close case, and the atlas's `Read` witnesses drive only the sync oracle, so the claim was never exercised on the async path
- Owner-gated: no

`Exact::fill` returns `Arrived { filled, failure: Some(e) }` when the transport errors mid-fill, but `read_frame` consults `failure` only when `filled == 0`. With one or two opener bytes in hand and a failure recorded, the error object is discarded and `Exact::head` re-reads the transport for the missing head; the outcome then depends on what the transport does after an error: a sticky error reproduces `Read { part: Signal }` with the second error object, an EOF misclassifies as `Truncated { missing: Signal }`, and a transport that resumes delivering decodes the frame as if nothing failed. The struct doc promises classification "exactly as a reader fetching one head at a time would classify it", which would report the first error at the signal item.

Evidence:

    151	    let mut opener = [0u8; OPENER_LEN];
    152	    let arrived = exact.fill(&mut opener).await;
    153	    if arrived.filled == 0 {
    154	        return match arrived.failure {
    155	            None => Ok(None),
    156	            Some(source) => Err(direction(DecodeErrorKind::Read {
    157	                part: FramePart::FrameHead,
    158	                source,
    159	            })),
    160	        };
    161	    }

    255	                Err(source) => {
    256	                    return Arrived {
    257	                        filled,
    258	                        failure: Some(source),
    259	                    };

Resolution: Carry `arrived.failure` into the wire-order judgment: parse the bytes in hand and, at the first head that needs more bytes, return `Read { part, source: failure }` instead of re-reading (for example, give `Exact` a pending-failure slot that `fill_exact` surfaces before touching the transport). Add the async failing-reader fixture of remote-codec-15 with a non-sticky shape (`[0x82]`, then `Err(Other)`, then EOF). Acceptance: the non-sticky fixture yields `Read { part: Signal, source: Other }` in both decoders (today the async reader yields `Truncated { missing: Signal }`), and the sticky-error and full-delivery cases are unchanged.

Construction: An `AsyncRead` that serves `[0x82]` on the first poll, `Err(io::ErrorKind::Other)` on the second, and `Ok(())` with nothing filled thereafter. Drive `FrameRead::new(Speaker::Initiator, RunBudget::default(), reader).frame()`. Trace: `fill` returns `{ filled: 1, failure: Some(Other) }`; the `filled == 0` gate is not taken; the frame head parses from the byte in hand (arity 2); the stream `Pending` takes an empty `rest`, so `Exact::head` calls `fill_exact(1 byte)`, which reads EOF and returns `short(Signal)` with `failure: None`, so `Truncated { missing: Signal }`. Expected by the one-head-at-a-time contract and by the sync oracle over a `FailAfterReader::new(bytes, 1)`: `Read { part: Signal }`.

### remote-codec-12: Listing bulk reads zero-fill their scratch before reading, unlike the body reader's policy
- Where: src/tree/mirror/streaming/remote/codec/decode/async_io.rs:270-276 (related: src/tree/mirror/framing.rs:69-71 and :100-107)
- Class / severity / confidence: performance / nit / high
- Provenance: assessed (read)
- Seen by: perfapi (49); refutation: confirmed (bound 256 × 27 = 6912 bytes per query frame); history: today's 18527932; nothing chooses `resize` over `read_buf`
- Owner-gated: no

`fill_vec` grows the retained listing scratch with `resize(start + want, 0)` and then reads into it, so a query frame memsets up to about 7 KiB before its bytes arrive, while the same codec's body reader (`resume_payload`) reads into spare capacity and documents "no zero fill". Two body readers in one codec use two policies. Denominator: per query frame; sign: fixed (a strict deletion), magnitude small, so this is consistency more than speed.

Evidence:

    270	    async fn fill_vec(&mut self, scratch: &mut Vec<u8>, want: usize) -> Arrived {
    271	        let start = scratch.len();
    272	        scratch.resize(start + want, 0);
    273	        let arrived = self.fill(&mut scratch[start..]).await;
    274	        scratch.truncate(start + arrived.filled);
    275	        arrived
    276	    }

Resolution: `scratch.reserve(want)` and loop on `read.read_buf(scratch)` until `scratch.len() - start` reaches `want`, EOF, or error, mirroring `resume_payload`; or factor one bounded-read helper both callers use. Acceptance: `fill_vec` contains no `resize`; the listing proptests and `truncated_bodies_are_rejected` stay green.

### remote-codec-13: Long qualified paths at use sites where the file already imports the module
- Where: src/tree/mirror/streaming/remote/codec/decode/async_io.rs:446-455 (related: async_io.rs:9-18 and :535; src/tree/mirror/streaming/remote/codec/decode.rs:130, :150, :155, :161, :277, :290; src/tree/mirror/streaming/remote/codec/encode.rs:118; src/tree/mirror/streaming/remote/codec/frame.rs:36, :188, :348, :340, :344, :357-358; src/tree/mirror/streaming/remote/codec/greeting.rs:71, :160, :244-245, :258-259)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (each cited line read; grep: `std::io::Error::new(std::io::ErrorKind` occurs exactly twice in src, both at frame.rs:340 and :344, against six `io::Error::new(io::ErrorKind` sites elsewhere)
- Seen by: structure (13); refutation: confirmed; history: the `super::super::` spellings arrived with ed0f1775 and 18527932 beside existing `use super::super::{...}` blocks; the codec is the outlier against the tree's own `io::Error` convention
- Owner-gated: no

Several items are spelled by full path in expression position although the file already has a `use super::super::{...}` block: `super::super::budget::SUPPLY_FRAME_OVERHEAD`, `super::super::frame::RECORD_TAG_LEN`, `super::super::frame::lone_record_spans`, `super::super::frame::ListingIssue::Head`; `super::frame::ListingBuilder` and friends in decode.rs; `super::frame::checked_run_len` in encode.rs; `crate::tags::VERSION_TAG` five times; and `std::io::Error::new(std::io::ErrorKind::...)` with `io` unimported. None of these qualifications disambiguates or informs; `io::Error` is the sanctioned conventional pair, and this form is not it.

Evidence:

    446	                declared: super::super::budget::SUPPLY_FRAME_OVERHEAD.saturating_add(len),
    ...
    449	            if len < super::super::frame::RECORD_TAG_LEN + 1 {
    ...
    455	            if !super::super::frame::lone_record_spans(len, record) {

    frame.rs:340	            e => std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),

Resolution: Extend the existing `use` blocks (`use super::super::{budget::SUPPLY_FRAME_OVERHEAD, frame::{RECORD_TAG_LEN, lone_record_spans}, ...}`; `use crate::tags::VERSION_TAG;`; `use std::io;` then `io::Error::new(io::ErrorKind::..)`). Acceptance: no `super::super::` or `super::frame::`/`super::budget::` in expression position in the codec; `crate::tags::VERSION_TAG` appears only in `use` lines; `just fmt` and `just clippy` clean.

### remote-codec-14: The over-budget lone-record body read can consume bytes of the next frame
- Where: src/tree/mirror/streaming/remote/codec/decode/async_io.rs:458-462 (related: async_io.rs:473-486; src/tree/mirror/framing.rs:84-110; src/tree/mirror/streaming/remote/streams.rs:485-492; src/tree/mirror/streaming/remote/codec/decode/tests.rs:1001-1059)
- Class / severity / confidence: correctness / medium / high
- Provenance: assessed (read; the capacity arithmetic traced through the pinned toolchain's `alloc/raw_vec/mod.rs:158-166` (`min_non_zero_cap(1) == 8`), `bytes` 1.11.1 `BufMut for Vec<u8>::chunk_mut` at buf_mut.rs:1623-1628 (exposes the whole spare capacity), and `tokio` 1.52.3 `AsyncRead for &[u8]` at async_read.rs:102 (`min(self.len(), buf.remaining())`); not constructed or run)
- Seen by: correctness (32); refutation: confirmed, severity argued down to low on consequence; history: the review packet's "considered and dismissed" entry on `resume_payload` rests on capacity being "clamped to `len` on every growth step", which covers growth inside the loop and not the capacity the caller's `prefix` already carries, so the finding is not already dismissed
- Owner-gated: no

The over-budget supply path builds `prefix` from `Vec::new()` with two `cbor::write_head` calls (each an `extend_from_slice`), which std grows to the one-byte-element minimum capacity of 8, and hands it to `resume_payload`, whose loop reserves only when `len() == capacity()` and otherwise calls `read_buf` into the whole spare capacity. When the declared run `len` is below that capacity (a lone record of 1 to 4 content bytes: `len` 4 to 7 with a 3-byte prefix), the first `read_buf` may take up to `capacity - len` bytes belonging to the next frame. The frame is then judged with foreign trailing bytes (`LeafRun::from_encoded` sees a second, bogus record and fails `NotARecord`) and the next frame's bytes are gone. Two docs promise the opposite: `read_payload`'s "Never consumes a byte beyond `len`", which `resume_payload` inherits, and `FrameRead`'s "a valid frame is consumed exactly and no byte of the next frame is touched"; `streams.rs` hands the transport half back through `into_inner` on the strength of that exactness. A conforming encoder cannot produce such a record (its smallest is at least 8 bytes: a 2-byte record tag, a 1-byte length, the 3-byte version tag, a 1-byte version head, and at least one payload byte), and the triggering record fails at `records()` regardless, so today the breach shows only as a misclassification (`InvalidRun(NotARecord)` where the sync oracle says `Ok`) on a frame that fails anyway. I keep it at medium because the clause breached is the exactness contract the frame-boundary hand-off rests on, the callee's documentation promises it without stating a capacity precondition, and the clamp is cheap and protects every future caller.

Evidence:

    458	            // Legal lone record: resume the body read behind the heads
    459	            // already consumed, in the same single buffer.
    460	            resume_payload(&mut *self.exact.read, prefix, len)
    461	                .await
    462	                .map_err(|source| classify(FramePart::SupplyRun, source))?

    474	        let mut prefix = Vec::new();

    framing.rs:100	    while payload.len() < len {
    framing.rs:101	        if payload.len() == payload.capacity() {
    framing.rs:102	            let target = (payload.capacity() * 2).max(PAYLOAD_CHUNK_LEN).min(len);
    framing.rs:103	            payload.reserve_exact(target - payload.len());
    framing.rs:104	        }
    framing.rs:105	        if read.read_buf(&mut payload).await? == 0 {

Resolution: Clamp the read in `resume_payload` so no iteration can fill past `len`: read through `(&mut payload).limit(len - payload.len())` (`BufMut::limit` caps `chunk_mut`), or shrink so `capacity() <= len` before the loop; the callee is the right home because its documentation already promises exactness without a capacity precondition, and `record_prefix` should not have to know std's minimum capacity. Then add to `overbatched_corners_classify_exactly` (or a sibling) a case decoding, through `decode_both`, a zero-budget frame carrying `raw_record(&[0x00])` followed by a second frame's bytes, asserting the first decode is `Ok` and, through `FrameRead`, that the second frame then decodes cleanly; also feed the same bytes through a chunked reader so the resume path runs under partial delivery. Acceptance: the new test fails at HEAD (the async reader returns `InvalidRun(NotARecord { remaining: 3, .. })` while the sync oracle returns `Ok`, so `decode_both` panics on disagreement) and passes after the clamp; `supply_full_delivery_costs_at_most_payload_plus_chunk` and `overbatched_supply_rejects_without_buffering_its_body` in `tests/decode_alloc.rs` stay green.

Construction: Bytes: opener `[0x83, 0x09, 0x07]` (stream 9, `Signal::Supply(Flow::End)`), then `[0xd8, 0x3f, 0x44]` (tag 63, byte string of 4), then the run body `[0xd8, 0x3f, 0x41, 0x00]` (one record of 1 content byte), then a trailing frame `[0x82, 0x09, 0x09]` (`End::Stream` on stream 9). Budget `RunBudget::from_bytes(0)`. Async: `covers(4)` is false (envelope 10 + 4 > 0); `len` 4 is not below 3; `record_prefix` yields `prefix = [d8 3f 41]` (len 3, capacity 8) and record content 1; `lone_record_spans(4, 1)` is `2 + 1 + 1 == 4`, true; `resume_payload` sees `3 < 4` and `3 != 8`, so `read_buf` offers a 5-byte chunk and the slice reader fills it with all 4 remaining bytes `[00 82 09 09]`; the payload is 7 bytes; `from_encoded` parses record one, then reads `0x82` (major 4) where a tag belongs and returns `NotARecord { remaining: 3, .. }`. Sync: `read_exact` takes exactly 1 byte and returns `Ok` with a one-record run. `decode_both` hits its `(Err, Ok)` arm and panics; alternatively call `FrameRead::frame` twice and observe the second call return `Ok(None)` (EOF) instead of `Some((stream 9, Frame::End(End::Stream)))`.

### remote-codec-15: The async reader's transport-failure classification has no per-part witness against the oracle
- Where: src/tree/mirror/streaming/remote/codec/decode/tests.rs:774-780 (related: src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:229-263 and :719-747; src/tree/mirror/streaming/remote/codec/tests.rs:543-548; src/tree/mirror/streaming/remote/codec/decode/async_io.rs:201-213)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read: `FailingReader` and `FailAfterReader` implement `std::io::Read`; the codec's only `AsyncRead` fixture, `CountingRead`, never fails)
- Seen by: correctness (35); refutation: reframed (session-level fault suites `AdversarialRead` and `Cut` do drive read errors through live sessions and match `CodecDecodeErrorKind::Read`; what is missing is the codec-level per-part comparison); history: the atlas's `Read` witnesses were built on the sync oracle when both readers shared a per-head read shape, and 18527932 changed the async shape without revisiting them
- Owner-gated: no

Every `DecodeErrorKind::Read` witness in the codec suites and in the error atlas drives the sync oracle through a `std::io::Read` fixture; no `AsyncRead` fixture ever fails, so `FrameRead`'s `Read` arms (`Arrived::short` with a failure, `classify` on a non-EOF error, the opener path) are exercised at the codec level only by success and EOF. The session-level suites confirm that a failing transport surfaces as some `Read`, not that it surfaces at the same part the oracle names; remote-codec-11 lives entirely in this unexamined region. I keep it at low rather than nit because it is the blind spot that let a real defect through.

Evidence:

    774	struct FailingReader;
    775	
    776	impl std::io::Read for FailingReader {
    777	    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
    778	        Err(std::io::ErrorKind::Other.into())
    779	    }
    780	}

    error_atlas.rs:735	impl std::io::Read for FailAfterReader {

Resolution: Add a `FailAfterAsyncReader` mirroring `FailAfterReader` (serve `remaining` bytes, then `Err(Other)`, then optionally EOF) and a `decode_both`-style helper taking a reader factory, so the sync `FailAfterReader` and its async twin are compared at the offsets `error_atlas::decode_errors` already uses (0, 1, 3, 4 for a query; 3 and 6 for a supply); record the async witnesses in the atlas beside the sync ones. Acceptance: each frame part has an async `Read(part=...)` witness in the atlas, and the classification equals the sync oracle's at every offset for a sticky failing reader; the non-sticky shape from remote-codec-11 is included.

Construction: `struct FailAfterAsyncReader { bytes: Vec<u8>, remaining: usize, then_eof: bool }` implementing `AsyncRead`: serve `min(remaining, buf.remaining())` bytes while `remaining > 0`, then `Poll::Ready(Err(Other))` once, then EOF if `then_eof`. Drive `FrameRead::frame` and compare `kind_signature` against `decode(speaker, budget, &mut FailAfterReader::new(bytes, remaining))` for each offset.

### remote-codec-16: The lone-record acceptance boundary at exactly `RECORD_TAG_LEN + 1` is tested only from the rejecting side
- Where: src/tree/mirror/streaming/remote/codec/decode/tests.rs:1006-1007 (related: src/tree/mirror/streaming/remote/codec/decode/async_io.rs:449; src/tree/mirror/streaming/remote/codec/decode.rs:155; decode/tests.rs:373-401)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read; `RECORD_TAG_LEN` is `head_len(63) == 2`, so the sweep is `0..3` and the gate is `len < 3`; the only zero-content-record test runs at the default budget on the `read_payload` path)
- Seen by: correctness (39); refutation: confirmed (`.cargo/mutants.toml` excludes nothing in rumors, so a `<=` mutant would be a reported survivor by reading, not run); history: ed0f1775 introduced the guard and the rejecting-side sweep together
- Owner-gated: no

The over-budget gate rejects declared bodies shorter than `RECORD_TAG_LEN + 1` on the length alone; the corner test sweeps `0..3` and asserts rejection, but no test presents a 3-byte body (the zero-content record `[d8 3f 40]`) over budget and asserts acceptance. A mutant loosening `<` to `<=` at either decoder survives. Off-by-one at a boundary is judged from both sides; a check tested only on its rejecting side cannot tell `<` from `<=`.

Evidence:

    1006	        // Declared bodies too short for a record's heads, none delivered.
    1007	        for declared in 0..RECORD_TAG_LEN + 1 {

    async_io.rs:449	            if len < super::super::frame::RECORD_TAG_LEN + 1 {

Resolution: Extend `overbatched_corners_classify_exactly` with `supply(stream, Flow::End, &raw_record(&[]))` under the zero budget, asserting `decode_both` yields `Ok` with a one-record run (the same run `a_zero_length_record_is_structurally_valid` accepts within budget). Acceptance: the new case passes at HEAD and fails when the comparison is loosened to `<=` in either decoder.

Construction: Bytes: opener `[0x83, 0x09, 0x07]`, then `[0xd8, 0x3f, 0x43]`, then body `[0xd8, 0x3f, 0x40]`. Under `RunBudget::from_bytes(0)`: `len` 3 is not below 3; `record_prefix` yields a 3-byte prefix with content 0; `lone_record_spans(3, 0)` is `2 + 1 + 0 == 3`; `resume_payload` returns the prefix untouched (`3 < 3` is false); `from_encoded` accepts one zero-content record. With `<=`, `overbatched()` fires instead.

### remote-codec-17: Three testdocs state invariants their bodies do not check
- Where: src/tree/mirror/streaming/remote/codec/decode/tests.rs:1061-1070 (related: decode/tests.rs:1092-1095; src/tree/mirror/streaming/remote/codec/greeting/tests.rs:52-80; src/tree/mirror/streaming/remote/codec/frame/tests.rs:27-37 and :71-74; src/tree/mirror/streaming/remote/codec.rs:47-50)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read: each doc compared against its body)
- Seen by: prose (21, 22, 23); refutation: confirmed all three; history: the "at ingress" wording transcribes the depth-limit spec's loose use of the word (REVIEW.md step 7, landed by 4356e197); the greeting overclaim survived 574cc760's removal of the magic-key case; the regime overclaim is original to f2b74a97
- Owner-gated: no

AGENTS.md holds every testdoc to accuracy; an inaccurate one is a bug in the test. Three here overclaim. `an_over_deep_supplied_payload_dies_typed_at_ingress` says the failure is "at wire ingress", but the body accepts the run at `LeafRun::from_encoded` and takes the error from `run.records(codec).next()`, the deferred record decode; the module's own vocabulary (codec.rs:47-50) separates exactly these two stages and the test's inline comment says "at the record iterator". `greeting_key_roster_is_exact` promises rejection of "a missing or out-of-order key, or trailing bytes" but constructs a renamed key and trailing bytes only; a dropped key (the count branch at greeting.rs:123-127) and a swapped pair are untested. `record_len_matches_an_actual_push` opens with "at every CBOR byte-string head width a version can occupy", then concedes the chain lengths land in the first two regimes and asserts only `>= 2`.

Evidence:

    1061	/// A hand-crafted record whose payload nests one scope past the peer's
    1062	/// depth limit dies typed at wire ingress, while the same shape at
    1063	/// exactly the limit decodes clean, pinning the boundary.
    ...
    1092	    // One scope past the limit: typed rejection at the record iterator.
    1093	    let over = record_with_payload(&deep_payload(limit.get() as usize + 1));
    1094	    let run = LeafRun::from_encoded(raw_record(&over)).unwrap();
    1095	    let error = run.records(codec).next().unwrap().unwrap_err();

    greeting/tests.rs:52	/// The greeting's map admits exactly one spelling: a missing or
    greeting/tests.rs:53	/// out-of-order key, or trailing bytes, are each rejected — one
    greeting/tests.rs:67	    wrong_key[at] = b'x';
    frame/tests.rs:27	/// `record_len` prices exactly what `push` writes, at every CBOR
    frame/tests.rs:28	/// byte-string head width a version can occupy.
    frame/tests.rs:72	        checked_regimes.len() >= 2,

Resolution: Rename the depth test to `..._fails_typed_at_the_record_iterator` and restate its first sentence ("fails typed at the record iterator, run structure having already passed ingress"). In the greeting test either construct the two named cases (splice the `set_len` entry out and decrement the map head from `0xa6` to `0xa5`, expecting `Shape("greeting is not a map of one entry per roster key")`; swap the `listing` and `set_len` entries, expecting the roster `Shape`) or restate the doc to "a renamed key or trailing bytes". In the record-length test either write "at the one- and two-byte head widths" or extend the sweep until `head_len` reaches 3 and assert `== 3`. Acceptance: each doc's stated rejections and widths correspond to constructed inputs asserted in the body; no testdoc in the partition places a `DecodeLeafError` at ingress.

Construction (greeting case): build the canonical map via `greeting_map(&sample(Vec::new()))`, locate the `set_len` text head and the following uint, remove those bytes, decrement the map head byte, and assert `parse_greeting` returns `GreetingError::Shape(_)`; the current test passes unchanged whether or not the count branch exists.

### remote-codec-18: `#[non_exhaustive]` follows a rule recorded only in a commit message
- Where: src/tree/mirror/streaming/remote/codec/error.rs:64-65 (related: error.rs:11-12, :41-42, :103-104, :112-114; src/tree/mirror/streaming/remote/codec/frame.rs:376-377 and :404-406; src/tree/mirror/streaming/remote/codec/signal.rs:387-388; src/tree/mirror/streaming/remote/codec/greeting.rs:99-101)
- Class / severity / confidence: api-surprise / low / high
- Provenance: assessed (read: `DecodeErrorKind`, `ListingIssue`, `GreetingError` carry the attribute; `EncodeErrorKind`, `FramePart`, `DecodeLeafError`, `Origin`, `LeafRunError`, `DecodeSignalError`, `StreamClass` do not)
- Seen by: perfapi (45), prose (open question); refutation: confirmed; history: a rule exists in 1e458d69 ("Six enums whose variant sets grow as enforcement grows ... the contract-outcome and wire-grammar enums stay exhaustive deliberately"), applied to `GreetingError`/`ListingIssue` by 0e2e85c6 and to `HeadError` by ruling R3; it is stated nowhere in the tree
- Owner-gated: yes (a semver policy on public enums)

Of the error enums this partition exports through `rumors::error`, three are `#[non_exhaustive]` and seven are not, and the tree states no rule. History has one: taxonomies that grow with enforcement are open; wire-grammar and contract-outcome enums stay closed. Under it `FramePart` and `DecodeSignalError` are wire-grammar and closed by design, and `EncodeErrorKind`, `DecodeLeafError`, and `LeafRunError` are the cases the rule was never explicitly applied to. Undocumented deliberate choice: state the rule at one site and rule on the three.

Evidence:

    64	#[derive(Debug, thiserror::Error)]
    65	pub enum EncodeErrorKind {

    112	#[derive(Debug, thiserror::Error)]
    113	#[non_exhaustive]
    114	pub enum DecodeErrorKind {

Resolution: State the rule once, in `src/error.rs`'s module doc or beside the first open enum ("taxonomies that grow as enforcement grows are `#[non_exhaustive]`; wire-grammar and contract-outcome enums are closed"), and decide `EncodeErrorKind`, `DecodeLeafError`, and `LeafRunError` under it. Acceptance: the rule is readable from the tree; each public error enum in the codec is consistent with it.

### remote-codec-19: Public error docs: uneven variant coverage, undocumented public constructors, and two docs naming internals
- Where: src/tree/mirror/streaming/remote/codec/error.rs:97-102 (related: error.rs:19-26, :40-53, :63-76, :111-159, :150-154; src/tree/mirror/streaming/remote/codec/signal.rs:112-117, :137-154, :396-397; src/tree/mirror/streaming/remote/codec.rs:75-76; src/tree/mirror/streaming/remote/error.rs:11-16)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified for the two internals (read: `LeafRun` is absent from the export list at remote/error.rs:11-16, and `SUPPLY_FRAME_OVERHEAD` is re-exported only under `#[cfg(test)]` at codec.rs:75-76); the coverage tally assessed by reading error.rs and signal.rs
- Seen by: prose (25, 26); refutation: confirmed both; history: the `declared` field doc is a verbatim transcription of REVIEW.md seed 3's prescribed wording, whose purpose survives the rewording; `Origin::direction`/`stream` have been public and undocumented since e41e7069; no `missing_docs` lint has ever been enabled
- Owner-gated: yes for narrowing the public constructors; no for the prose

Within `DecodeErrorKind`, three variants carry docs and seven do not; `EncodeErrorKind`, `FramePart`, and `Speaker` variants have none; `Origin::direction` and `Origin::stream` are undocumented `pub fn`s; `StreamClass` documents one variant of five; `DecodeSignalError::Placement` is undocumented. `src/error.rs:3-6` tells users this taxonomy is "for matching and bug reports", so the distinction a matcher needs (`Read`: the transport failed; `Truncated`: a clean close mid-frame; `Malformed`: present but not canonical) belongs at the variants. Two docs also explain themselves in terms of names the API does not reach: `DecodeLeafError` via `LeafRun::records` and "the incoming adapter", and `OverbatchedRun::declared` via `SUPPLY_FRAME_OVERHEAD`.

Evidence:

    97	/// A decode or canonicality failure in a supplied leaf record.
    98	///
    99	/// Produced by the run's record iterator (`LeafRun::records`), which the
    100	/// incoming adapter drives record by record; run *structure* is instead
    ...
    115	    #[error("could not read the frame's {part}")]
    116	    Read {
    ...
    150	        /// The frame's charged wire size — its run body plus the
    151	        /// `SUPPLY_FRAME_OVERHEAD` envelope at its widest — which may

    19	impl Origin {
    20	    pub fn direction(speaker: Speaker) -> Self {

Resolution: One line per variant stating what separates it from its neighbors (`Read`: "The transport failed while the frame's `part` was being read."; `Truncated`: "The transport closed cleanly before the frame's `missing` component arrived."; `TrailingBytes`: "Bytes followed the frame where the input was required to end."; likewise for `EncodeErrorKind`, `FramePart`, `Speaker`, `StreamClass`, `DecodeSignalError::Placement`). Rewrite error.rs:99-100 as "Produced when a supplied record is decoded, after the run's structure has already been validated at the wire ([`LeafRunError`])." and error.rs:150-153 as "The frame's charged wire size: its run body plus a fixed frame-head envelope, which may exceed the frame's actual size by a few bytes of head slack." Document or narrow to `pub(crate)` the `Origin` constructors. Acceptance: every public variant and public method in error.rs and signal.rs has a doc comment; no public doc in error.rs names a type, function, or constant that `rumors::error` does not export.

### remote-codec-20: The public `FrameShape` variant doc states the wrong array arity
- Where: src/tree/mirror/streaming/remote/codec/error.rs:131-133 (related: src/tree/mirror/streaming/remote/codec/decode.rs:217-231; src/tree/mirror/streaming/remote/codec.rs:3-5)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git log -S"one- or two-element"` names only 4dd2053c, where decode.rs:227 read `if !(1..=2).contains(&head.value)`; `git log -S"(2..=3).contains"` names 3327a92b, whose stat touches no codec/error.rs)
- Seen by: structure (5), prose (17), correctness (40), perfapi (43); refutation: confirmed; history: deliberate-but-expired (the two-item opener widened the decoder without touching this line)
- Owner-gated: no

The doc says a frame is "a one- or two-element CBOR array"; every frame now opens with two items plus an optional body, `frame_arity` enforces `(2..=3)`, and the module doc describes `[stream, state]` and `[stream, state, body]`. The variant is reachable as `rumors::error::CodecDecodeErrorKind::FrameShape`, so a user reads a contract the code contradicts. The Display text "frame is not a CBOR reaction array" also misnames bare `End` frames, which are not reactions.

Evidence:

    131	    /// The frame item is not a one- or two-element CBOR array.
    132	    #[error("frame is not a CBOR reaction array: {detail}")]
    133	    FrameShape { detail: &'static str },

    decode.rs:225	    if !(2..=3).contains(&head.value) {
    decode.rs:227	            detail: "frame array is not two or three items",

Resolution: "The frame item is not a two- or three-item CBOR array (the opener's stream and state, then a body when the state takes one)."; consider "frame is not a CBOR array of two or three items" for the Display text (a wire-neutral message change, but the atlas snapshot pins it, so re-accept deliberately). Acceptance: the variant doc and Display agree with `frame_arity`'s accepted range and detail strings; no prose in the partition says "one- or two".

### remote-codec-21: Visibility wider than reach, `thiserror::Error` derived for `Display` alone, and a positional three-field variant
- Where: src/tree/mirror/streaming/remote/codec/frame.rs:18-29 (related: frame.rs:490, :517, :527; src/tree/mirror/streaming/remote/codec/signal.rs:9-15 and :138-139; src/tree/mirror/streaming/remote/codec/error.rs:41-42; src/tree/mirror/streaming/remote/codec/decode/async_io.rs:338-340, :398, :521-525; src/tree/mirror/streaming/remote/codec.rs:98-99)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep: `HASH_HEAD_LEN` has no user outside frame.rs:28; `LEAF_HEIGHT`, `HIGHEST_STREAM_HEIGHT`, `STREAMED_HEIGHT_COUNT` appear only in signal.rs and signal/tests.rs; `listing_len`, `MAX_QUERY_CHILDREN`, `listing_entry_len` appear only within the codec; `parse_listing_map`/`write_listing` reach proxy/tests/harness.rs through the re-export at codec.rs:99)
- Seen by: structure (14); refutation: reframed (the `pub(super)` suggestion for `parse_listing_map`/`write_listing` is impossible: a `pub(crate) use` re-export cannot widen an item's own visibility, E0364, so those two must stay `pub(crate)`); history: no rationale for any of the visibilities or the `Error` derives
- Owner-gated: no

`HASH_HEAD_LEN` is `pub` with one user in the same file; the three height constants are `pub` but used only within signal.rs and its tests; `MAX_QUERY_CHILDREN`, `listing_entry_len`, and `listing_len` are `pub`/`pub(crate)` with `pub(super)` reach. `StreamClass` and `FramePart` derive `thiserror::Error` although neither is an error; they need only `Display` for interpolation, and `FramePart` is a public type that implements `std::error::Error` by accident of the shortcut. `Entry::Complete(u8, [u8; MERKLE_HASH_LEN], usize)` carries radix, digest, and consumed width positionally, and its consumer destructures three unlabeled fields. Since `codec` is `pub(crate)` and `tree` private, none of these leaks outside the crate; the point is that visibility should state reach.

Evidence:

    18	/// Largest query fan a listing map can carry: one child per radix value.
    19	pub const MAX_QUERY_CHILDREN: usize = 256;
    20	
    21	/// Bytes of the byte-string head ahead of one listed Merkle hash.
    22	pub const HASH_HEAD_LEN: usize = cbor::head_len(MERKLE_HASH_LEN as u64);

    signal.rs:138	#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
    signal.rs:139	pub enum StreamClass {
    async_io.rs:340	    Complete(u8, [u8; MERKLE_HASH_LEN], usize),

Resolution: Tighten `HASH_HEAD_LEN`, the height constants, `listing_len`, `MAX_QUERY_CHILDREN`, and `listing_entry_len` to their reach; leave `parse_listing_map` and `write_listing` at `pub(crate)`. Replace the `thiserror::Error` derives on `StreamClass` and `FramePart` with a hand-written `Display` (or keep `Error` on `FramePart` only if it is meant as a public error type, and say so). Make `Entry::Complete { radix, digest, width }`. Acceptance: no `pub` item in the codec is unused outside its own module; `FramePart` implements `Error` only if intended; `parse_entry`'s match arm names its fields.

### remote-codec-22: `LeafRun`'s hand-written `Default`, `Clone`, `PartialEq`, and `Eq` are residue of the erased type parameter
- Where: src/tree/mirror/streaming/remote/codec/frame.rs:92-112 (related: frame.rs:114-121)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git show 48bc31df -- src/tree/mirror/streaming/remote/codec/frame.rs` shows `-pub struct LeafRun<T>`, `-    marker: PhantomData<fn() -> T>,`, and `-impl<T> Clone for LeafRun<T>` rewritten as `+impl Clone for LeafRun`, likewise for `Default`, `PartialEq`, `Eq`, with the bodies unchanged)
- Seen by: structure (4), perfapi (44); refutation: confirmed; history: deliberate-but-expired (the impls existed to avoid `T: Clone`/`T: PartialEq` bounds; 48bc31df erased `T` and the marker but only stripped the `<T>`)
- Owner-gated: no

`LeafRun` has one field, `bytes: Vec<u8>`, and four hand-written impls that are byte-for-byte what `#[derive(Default, Clone, PartialEq, Eq)]` produces. Machinery outliving the constraint that justified it: twenty lines a reader must verify are equivalent to derives. `Debug` is legitimately hand-written (it renders counts, not bytes).

Evidence:

    92	impl Default for LeafRun {
    93	    fn default() -> Self {
    94	        Self::new()
    95	    }
    96	}
    97	
    98	impl Clone for LeafRun {
    99	    fn clone(&self) -> Self {
    100	        Self {
    101	            bytes: self.bytes.clone(),
    102	        }
    103	    }
    104	}
    105	
    106	impl PartialEq for LeafRun {
    107	    fn eq(&self, other: &Self) -> bool {
    108	        self.bytes == other.bytes
    109	    }
    110	}
    111	
    112	impl Eq for LeafRun {}

Resolution: `#[derive(Default, Clone, PartialEq, Eq)]` on `LeafRun`, keeping the custom `Debug`. Acceptance: `frame.rs` has no `impl Default/Clone/PartialEq/Eq for LeafRun`; the codec tests pass.

### remote-codec-23: `checked_run_len`'s doc calls the encoder-side check redundant; it is the only bound on the accumulated body
- Where: src/tree/mirror/streaming/remote/codec/frame.rs:300-306 (related: frame.rs:177-182; src/tree/mirror/streaming/remote/codec/encode.rs:117-122; src/tree/mirror/streaming/remote/codec/budget.rs:147-149; src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:92-98)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read: `push` calls `checked_run_len(item)` on the single record item; `FrameEncoding::new` calls it on `run.encoded_len()`, the accumulated body; `LeafRun` itself never consults `RunBudget::admits`)
- Seen by: prose (28); refutation: reframed (the call at encode.rs:118 is also the `usize` to `u64` conversion the byte-string head needs, typed as the cap check, so it is not standalone guard machinery; the doc sentence is the fix); history: the doc is 4dd2053c prose and the only rationale on record; the atlas exempts `SupplyTooLarge` as "resource exhaustion by construction", consistent with a programmer-error path, not with "belt to that suspender"
- Owner-gated: no for the doc; the typed-error-versus-assert question is the owner's (see open questions)

The doc says a run the cap rejects "was necessarily a single record" that `push` already rejected, and calls the encoder-side call "the belt to that suspender", whose mechanism reading is "a redundant second check". It is not redundant: `push` bounds each record item, never the cumulative body, so a caller that pushes past the cap without consulting `RunBudget::admits` is caught here and nowhere else. A guard earns its place by naming the constructible failure it catches; this doc names the wrong justification and omits the right one.

Evidence:

    300	/// Check a run body length against the wire's run byte cap.
    301	///
    302	/// The encoder's boundary: a run the cap rejects was necessarily a single
    303	/// record (the budget saturates below the cap, so a multi-record run never
    304	/// grows here), and [`LeafRun::push`] already rejected any such record —
    305	/// this check is the belt to that suspender, priced identically.

    179	        let item = RECORD_TAG_LEN
    ...
    182	        checked_run_len(item)?;
    encode.rs:118	                let len = super::frame::checked_run_len(run.encoded_len())?;

Resolution: Restate: "The run byte cap as a checked conversion to the u64 the run head carries. `push` bounds each record item; the encoder applies this to the accumulated body, which only a caller pushing past the cap without consulting `RunBudget::admits` can exceed (the budget saturates below the cap). Reaching it there is a programmer error in the accumulator, surfaced typed as `EncodeErrorKind::SupplyTooLarge`." Acceptance: the doc names the constructible failure; no metaphor remains.

### remote-codec-24: Every supplied record's version atom is decoded through the general CBOR reader: a 4 KiB stack zero and a `Vec` allocation per leaf, and an unjudged head spelling
- Where: src/tree/mirror/streaming/remote/codec/frame.rs:364-365 (related: frame.rs:73-84 and :337-345; src/tree/mirror/streaming/remote/codec.rs:10-16; src/tree/mirror/streaming/remote/codec/greeting.rs:158-178; src/tree/mirror/streaming/remote/codec/decode/tests.rs:467-534; crates/before/src/serde_impls.rs:39-43)
- Class / severity / confidence: performance / medium / high
- Provenance: assessed (read at the locked versions: ciborium-0.2.2 `src/de/mod.rs:825-831` is `pub fn from_reader(...) { let mut scratch = [0; 4096]; from_reader_with_buffer(reader, &mut scratch) }`; `crates/before/src/serde_impls.rs:41-42` is `let bytes = <Vec<u8>>::deserialize(d)?; Version::decode(&bytes[..])`; not measured)
- Seen by: perfapi (41); refutation: confirmed; history: already-known and reopens a ruling: REVIEW.md B2a proposed exactly this resolution and Finch ruled "B2: enforcement alternative declined — the prose rescope plus boundary-pinning witnesses is the resolution of record"; 60a6d191 and 18fbc92f committed the two pins, each saying flipping to rejection is "a deliberate contract change, not drift"; the ruling rested on the detector never firing against any existing encoder, and the performance argument was not before Finch
- Owner-gated: yes (wire acceptance changes; reopens B2)

`parse_record` hand-parses the version-atom tag, then hands the byte string to `ciborium::de::from_reader`, which zeroes a 4 KiB stack scratch per call, after which `before`'s `Deserialize for Version` allocates a `Vec<u8>` for the atom bytes before `Version::decode`. Both costs are strictly redundant with the head primitive the crate already uses for the same shape in the greeting (`read_head`, require `MAJOR_BSTR`, `split`, `Version::decode`), and `de_error` exists only to convert the reader's error. The same choice is what leaves the atom's byte-string head unjudged for shortest form and definiteness, documented at frame.rs:77-84 and codec.rs:12-16 and pinned by two tests, so the wire has one canonicality rule for the greeting's atom and a weaker one for a record's. Denominator: per supplied record; sign: fixed (the scratch zero, the intermediate `Vec`, the serde dispatch, and `de_error` are deleted outright). This reopens B2 on grounds the ruling did not weigh, and the finding says so.

Evidence:

    364	    let version: Version =
    365	        ciborium::de::from_reader(&mut input).map_err(|e| DecodeLeafError::Version(de_error(e)))?;

    greeting.rs:165	                let head = cbor::read_head(&mut input).map_err(GreetingError::Head)?;
    greeting.rs:166	                if head.major != MAJOR_BSTR {
    greeting.rs:178	                version = Some(Version::decode(atom).map_err(GreetingError::Version)?);

Resolution: In `parse_record`, after the version-tag check, `cbor::read_head(&mut input)`, require `MAJOR_BSTR`, split `head.value` bytes, and `Version::decode(atom)`; surface the decoder's error as `DecodeLeafError::Version` (mapping into the existing `io::Error`, or changing the variant to carry `before::error::Decode` as `GreetingError::Version` does). Re-state `widened_version_atom_head_is_not_spelling_judged` and `indefinite_version_atom_head_is_not_spelling_judged` as rejections, and re-denominate the exception sentences at frame.rs:77-84 and codec.rs:12-16 to name only the application payload as the general-reader position. Name the B2 ruling and the wire-acceptance change in the commit. Acceptance: `parse_record` contains no `ciborium::de` call; the two former pins assert rejection with the typed head error; the gossip and codec snapshots are byte-identical (the encoder already emits shortest form); an allocator meter over `records()` on a run of N records shows no per-record allocation beyond the `Version`, `Arc`, and `Bytes` it constructs.

### remote-codec-25: The per-record payload copy is the custody hand-off; the site does not say so
- Where: src/tree/mirror/streaming/remote/codec/frame.rs:370-371 (related: frame.rs:366-369; src/tree/mirror/streaming/remote/codec/budget.rs:19-24; src/message.rs:433-438)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read: `Message::from_wire` takes `Bytes`; the custody statement lives at budget.rs:22-23)
- Seen by: perfapi (51); refutation: confirmed; history: the intent is recorded in budget.rs and the review packet, not at the site
- Owner-gated: no

`parse_record` copies each payload out of the run with `Bytes::copy_from_slice`. The obvious optimization is to hold the run as `Bytes` and hand each `Message` a `slice_ref`; it is wrong here, because every surviving message would then pin the whole run allocation (up to the negotiated budget, about 1.8 MB by default) after its siblings are redacted, and per-leaf pricing would no longer describe residency. The copy is what makes budget.rs's "each record passing custody of its payload to the storage backend as it is read" true, but the site's comment speaks only to the payload parse. Comments state what the code cannot show.

Evidence:

    370	    let message = Message::from_wire(bytes::Bytes::copy_from_slice(input), codec)
    371	        .map_err(DecodeLeafError::Message)?;

Resolution: One sentence at the call: the copy severs the message from the run buffer so a retained leaf never pins a whole run and per-leaf pricing stays exact. Acceptance: the sentence is present; no code change.

### remote-codec-26: `KEYS`' deterministic order is asserted by hand, and the greeting round-trip is three fixtures, not a family
- Where: src/tree/mirror/streaming/remote/codec/greeting.rs:35-44 (related: src/tree/mirror/streaming/remote/codec/greeting/tests.rs:5-50)
- Class / severity / confidence: verification-gap / nit / high
- Provenance: assessed (read; I checked the order by hand: text heads 0x67, 0x67, 0x67, 0x71, 0x73, 0x73, then bytewise `l < s < v` and `p < t`, which is correct; `greeting/tests.rs` contains no reference to `KEYS`)
- Seen by: correctness (38); refutation: confirmed; history: REVIEW.md's round-4 check recorded a by-hand verification and "No action"; 71de90c1 re-derived the order by hand again when adding a key
- Owner-gated: no

The module doc states the keys "ride in CBOR deterministic order", and `KEYS` is that order by hand. Nothing computes the order a second way, and `greetings_round_trip` covers three listing fixtures with one fixed version and fixed integers (7, 4096, 300, 1 << 20), so the wider head regimes of the integer fields are exercised only indirectly through the gossip snapshots. Any quantity computable two ways gets a committed test; a claim about a family is stated as a proptest so the shrunk counterexample rides along as a seed. This test is also what lets remote-codec-27 replace the runtime roster walk.

Evidence:

    37	const KEYS: [&str; 6] = [
    38	    "listing",
    39	    "set_len",
    40	    "version",
    41	    "max_version_bytes",
    42	    "payload_depth_limit",
    43	    "target_message_size",
    44	];

Resolution: Add a unit test asserting `KEYS` is strictly ascending under `(head_len(len), bytes)` (or by encoding each key with `write_head(MAJOR_TEXT, ..)` and comparing the byte vectors), and a `proptest!` round-trip over `arb_version()`, `btree_set(any::<u8>(), 0..=256)` listings, and `any::<u64>()` for the four integer fields. Acceptance: both tests committed and green; reordering `KEYS` or widening a field's head fails one of them.

### remote-codec-27: Greeting encode and parse dispatch on key strings inside a roster loop, forcing six `Option`s, six `expect`s, and two `unreachable!`s
- Where: src/tree/mirror/streaming/remote/codec/greeting.rs:60-88 (related: greeting.rs:128-133, :150-211; src/tree/mirror/streaming/remote/codec/greeting/tests.rs:56)
- Class / severity / confidence: simplification / medium / high
- Provenance: assessed (read)
- Seen by: structure (2), correctness (37), perfapi (53); refutation: confirmed; history: the roster-as-data shape is the original 4dd2053c design and its purpose (one spelling of the roster and its order shared by writer and parser) is stated inline and holds; nothing argues for the loop-plus-string-match as the mechanism, so this is a mechanism swap under the stated goal, not a reopen
- Owner-gated: no (the wire is untouched)

Both `greeting_map` and `parse_greeting` iterate `KEYS` and then `match key { "listing" => ..., _ => unreachable!(...) }`. The loop forces the parser to accumulate six `Option`s and unwrap each with `expect("the roster visits ...")`, and gives both functions an `unreachable!` arm; all eight panic sites exist only because a fixed six-step sequence is expressed as data-then-string-dispatch. Each site is programmer-error-only today and its message argues it, but the doctrine prefers removing the need for a proof over supplying one, and finished code should be obviously correct, not correct by an argument about a roster. Written straight-line in roster order, the same code has no `Option`, no `expect`, no `unreachable!`, and no stringly dispatch.

Evidence:

    63	    for key in KEYS {
    64	        cbor::write_head(&mut map, MAJOR_TEXT, key.len() as u64);
    65	        map.extend_from_slice(key.as_bytes());
    66	        match key {
    67	            "listing" => write_listing(&mut map, &greeting.listing),
    ...
    84	            _ => unreachable!("the key roster is exhaustive"),

    128	    let mut version = None;
    129	    let mut set_len = None;
    ...
    204	    Ok(Greeting {
    205	        version: version.expect("the roster visits version"),
    206	        set_len: set_len.expect("the roster visits set_len"),

Resolution: Factor `fn key(input: &mut &[u8], name: &'static str) -> Result<(), GreetingError>` for the text-key check and read the six entries in wire order as straight-line code (`key(&mut input, "listing")?; let listing = parse_listing_map(&mut input)...?; key(&mut input, "set_len")?; let set_len = uint(&mut input, ...)?; ...; Ok(Greeting { .. })`), mirroring `greeting_map` as six explicit writes. Keep `KEYS` as the single spelling of the roster and its order for the sortedness test of remote-codec-26 (and for the map count), so the roster's single-source role survives as a test rather than a runtime loop. Acceptance: `greeting.rs` contains no `unreachable!`, no `expect`, and no `match key`; `greetings_round_trip` and `greeting_key_roster_is_exact` pass unchanged; the greeting bytes in `tests/gossip_snapshot.rs` are byte-identical.

### remote-codec-28: `GreetingError::Order` is a public variant no public path produces
- Where: src/tree/mirror/streaming/remote/codec/greeting.rs:108-113 (related: greeting.rs:150-156 and :276-279; src/tree/mirror/streaming/remote/proxy/start.rs:317-321; src/tree/mirror/streaming/remote/proxy/error.rs:22-30; src/tree/mirror/streaming/remote/codec/greeting/tests.rs:177-195)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (grep over src and tests: `GreetingError::Order` is constructed only at greeting.rs:153 and consumed only by the stripping match at :277 and the test at greeting/tests.rs:190; every downstream consumer matches `Error::HandshakeListing`)
- Seen by: structure (3), correctness (36); refutation: confirmed; history: deliberate-but-expired (at 4dd2053c `GreetingError` was `pub(crate)` and `Order` a private routing variant; 0e2e85c6 made the enum `pub` and `#[non_exhaustive]` and re-exported it at `rumors::error` without revisiting the variant)
- Owner-gated: yes (a variant removal on a public enum)

`ListingIssue` already carries `Order(QueryOrderError)`. `parse_greeting` lifts that variant out into a flat `GreetingError::Order`, and `read_greeting` immediately strips it back out into `ReadGreetingError::Listing`, which `start.rs` routes to `Error::HandshakeListing`. Through every public path the `Order` variant never occurs and `GreetingError::Listing(ListingIssue::Order(_))` is never constructed, so a consumer matching on `Order` writes a dead arm. Circular justification: the variant exists to be unwrapped by the next layer, which exists only because the first wrap was done.

Evidence:

    108	    /// The listing map violated a structural rule.
    109	    #[error("greeting listing is malformed: {0}")]
    110	    Listing(ListingIssue),
    111	    /// The listing's keys were not in canonical strictly ascending order.
    112	    #[error(transparent)]
    113	    Order(QueryOrderError),

    152	                listing = Some(parse_listing_map(&mut input).map_err(|issue| match issue {
    153	                    ListingIssue::Order(order) => GreetingError::Order(order),
    276	    parse_greeting(&bytes).map_err(|e| match e {
    277	        GreetingError::Order(order) => ReadGreetingError::Listing(order),

Resolution: Delete `GreetingError::Order`; let `parse_greeting` return `GreetingError::Listing(issue)` unmapped; in `read_greeting` route `GreetingError::Listing(ListingIssue::Order(order))` to `ReadGreetingError::Listing(order)` and everything else to `ReadGreetingError::Decode`. Update `greeting_listing_order_is_enforced` to the nested form. (If the owner would rather collapse `HandshakeListing` into `HandshakeDecode(GreetingError::Listing(..))`, the fix is different; the finding assumes the separate surfacing stays.) Acceptance: `GreetingError` has no variant that no public path produces; `parse_greeting` has no `map_err` on the listing parse; `proxy/start/tests.rs` still observes `Error::HandshakeListing` for descending and repeated radixes.

### remote-codec-29: `read_greeting`'s doc describes an error arm the signature does not have
- Where: src/tree/mirror/streaming/remote/codec/greeting.rs:229-234 (related: greeting.rs:235 and :282-294)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read); the history pass verified with `git show 4dd2053c` that the phrase and the `Result<Greeting, ReadGreetingError>` signature landed together
- Seen by: structure (16), prose (20); refutation: confirmed; history: opaque from the start; 0e2e85c6 rewrote the following clause and left the phrase
- Owner-gated: no

The doc says transport failures "pass through as `Err(Ok-side io)`", a phrase describing no arm of `Result<Greeting, ReadGreetingError>`; the reader must scroll to the enum to learn the three arms are `Io`, `Decode`, and `Listing`. The sentence that should name them misdirects instead.

Evidence:

    229	/// Read one complete greeting item from the control stream.
    230	///
    231	/// Transport failures pass through as `Err(Ok-side io)`; a malformed or
    232	/// non-canonical greeting is a typed [`GreetingError`], except a
    233	/// non-canonical listing order, surfaced separately so the handshake can
    234	/// report it as the codec's own violation class.

Resolution: "Transport failures surface as [`ReadGreetingError::Io`]; a malformed or non-canonical greeting as [`ReadGreetingError::Decode`], except a non-canonical listing order, which [`ReadGreetingError::Listing`] carries separately so the handshake can report it as the codec's own violation class." Adjust if remote-codec-28 lands. Acceptance: the doc names each variant of `ReadGreetingError` by intra-doc link and contains no `Ok-side` phrase.

### remote-codec-30: Public methods on re-exported error types return types no user can name; `Display` routes through `Debug`
- Where: src/tree/mirror/streaming/remote/codec/signal.rs:41 (related: signal.rs:105-110, :361, :375; src/tree/mirror/streaming/remote/codec/error.rs:29-38; src/tree/mirror/streaming/remote/codec.rs:101-103; src/tree/mirror/streaming/remote/error.rs:11-18; src/error.rs:44-50; src/tree/mirror/streaming/remote/streams.rs:266)
- Class / severity / confidence: api-surprise / medium / high
- Provenance: verified (grep over src for `pub use` naming `Signal`, `InvalidSignalState`, or `StreamError` finds only remote/error.rs:18, which re-exports `streams::StreamError`; remote.rs:70 is `pub(crate) mod codec` and lib.rs:322 is `mod tree`, so the `pub use` chain at remote/error.rs:11-16 and src/error.rs:44-50 is the only public reach; no `unnameable_types`, `missing_docs`, `private_interfaces`, or `[lints]` in Cargo.toml, lib.rs, or the justfile)
- Seen by: structure (0), perfapi (42), prose (open question); refutation: confirmed; history: `signal::StreamError` and the `pub` on `Stream::new` date from e41e7069; the `Signal` leak is today's 3327a92b; ruling R3 (re-export `HeadError`) is precedent for the re-export option, not an obstacle
- Owner-gated: yes (public API surface)

`Stream` and `InvalidSignalPlacement` reach users as `rumors::error::{Stream, InvalidSignalPlacement}`, but `Stream::new` returns `Result<Self, StreamError>` where `StreamError` is `signal::StreamError`, and `InvalidSignalPlacement::signal()` returns `Signal`; neither type, nor `InvalidSignalState` behind `Signal::from_state`, appears in any `pub use`. A user calling `Stream::new(17)` gets an error they can match only through `Debug`, and `signal()` yields a value they cannot store in a named binding. The codec's `StreamError` also shares its name with the public `streams::StreamError` re-exported from the same module, so the one type a user can name is not the one `Stream::new` returns; a single-variant enum named like a sibling public type wants to be a struct with a distinct name. The same internal enums leak into public text: `InvalidSignalPlacement`'s message is `"signal {signal:?} on stream {} is invalid for {class}"` and `Origin`'s is `"{speaker:?} direction"`, so renaming a variant rewrites a user-visible message. Nothing catches the class mechanically.

Evidence:

    41	    pub fn new(index: u8) -> Result<Self, StreamError> {

    105	/// A programmatic stream index outside the wire's logical streams.
    106	#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
    107	pub enum StreamError {
    108	    #[error("wire stream index {index} is outside the valid range")]
    109	    Invalid { index: u8 },
    110	}

    361	#[error("signal {signal:?} on stream {} is invalid for {class}", stream.index())]
    375	    pub fn signal(self) -> Signal {
    error.rs:32	            Origin::Direction(speaker) => write!(f, "{speaker:?} direction"),

Resolution: Decide the intended surface. Either (a) the error taxonomy is diagnostic only: narrow `Stream::new`, `Stream::at_height`, `Stream::height`, `Speaker::other`, `Speaker::role`, and `InvalidSignalPlacement::signal` to `pub(crate)` (every in-crate caller of `Stream::new` uses `.expect` or `.ok()`), rename `signal::StreamError` to a struct such as `InvalidStreamIndex { index: u8 }`, and give `Origin` and `InvalidSignalPlacement` a `Display` that does not route through `Debug`; or (b) `Signal`, `Flow`, `End`, and the index error are public vocabulary: re-export them from `remote/error.rs` and `src/error.rs`, rename the index error so it does not collide with `streams::StreamError`, and give `Signal` a `Display`. Either way, enable `unnameable_types` in the crate lints so the gate holds the line. Acceptance: every `pub fn` reachable from `rumors::error` has a signature whose every type is nameable from outside the crate; no two public types in `rumors::error` share a simple name; the `Display` impls of `Origin` and `InvalidSignalPlacement` contain no `{:?}`; the lint reports nothing.

### remote-codec-31: `at_height` reuses a phase remainder as the index offset without the derivation; `STATE_STRIDE` is used once
- Where: src/tree/mirror/streaming/remote/codec/signal.rs:67-70 (related: signal.rs:20-21, :203-204, :228, :235-258; src/tree/mirror/streaming/remote/codec/signal/tests.rs:12-26)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read; derivation checked: for the initiator `height = 32 - 2i`, so `distance = 2i - 1` and `i = (distance + 1) / 2 = quotient + 1`, where the `+ 1` equals `INITIATOR_HEIGHT_PHASE` only because `2 * phase / stride == phase` at stride 2)
- Seen by: structure (10); refutation: reframed (the two-way state roster, arithmetic `state()` versus the `STATES` table, is doctrine-compliant as written: a quantity computed two ways with a committed test comparing them); history: both definitions and both constants date from 2203c104 with no rationale
- Owner-gated: no

`at_height` adds `INITIATOR_HEIGHT_PHASE` (a division remainder) to the quotient as the offset that skips the shared first stream; the value coincides but the code does not show why. `STATE_STRIDE` (`= 1`) is used once, at line 228, to mean "the next code". Both are constants named for one meaning and used for another, which undercuts the point of naming them. The two-way state roster itself is fine: the `STATES` table is what the snapshot pins, and `state_roster_is_bijective` reconciles it with the arithmetic.

Evidence:

    67	        let index = match (speaker, div_rem) {
    68	            (Speaker::Initiator, (quotient, INITIATOR_HEIGHT_PHASE)) => {
    69	                quotient + INITIATOR_HEIGHT_PHASE
    70	            }

    203	    /// Distance between adjacent state codes.
    204	    const STATE_STRIDE: u8 = 1;
    228	    const STREAM_END_STATE: u8 = Self::REPLY_END_STATE + Self::STATE_STRIDE;

Resolution: In `at_height`, write the offset as a named `SHARED_OPENING_STREAMS: usize = 1` (or `usize::from(Self::FIRST) + 1`) with a one-line derivation comment, or state the derivation beside the arm. Retire `STATE_STRIDE` in favor of `+ 1` with the comment "the bare stream end follows the bare reply end". Acceptance: no constant in signal.rs is used for a meaning other than its doc states; `stream_height_mappings_are_bijective` and `state_roster_snapshot` pass unchanged.

### remote-codec-32: `Speaker` and `observe::Role` name the same elected role; the reason is recorded only in a commit message
- Where: src/tree/mirror/streaming/remote/codec/signal.rs:112-135 (related: src/observe.rs:8 and :202-208; src/tree/mirror/streaming/remote/streams.rs:218 and :454)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (observe.rs:8 read: "rumors-blind: no protocol type appears in its signature"; `Speaker::role` has two callers, both in streams.rs)
- Seen by: structure (12); refutation: reframed (a rationale exists in 40b1e96a: the hook is "rumors-blind", so `observe` owns its own `Role` and the codec bridges into it; the dependency direction, codec importing the public hook vocabulary, is the intended one); history: deliberate-and-holds
- Owner-gated: yes if unified; no for the sentence

Two public enums (`rumors::error::Speaker`, `rumors::observe::Role`) name the elected role, and `Speaker::role()` exists to convert one to the other. The separation is deliberate (the observation hook must not name protocol types), but neither declaration says so, so the next reader re-derives the question. Undocumented deliberate choice: state the rationale at `Speaker`.

Evidence:

    112	/// The elected protocol role speaking in one transport direction.
    113	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
    114	pub enum Speaker {
    115	    Initiator,
    116	    Responder,
    117	}
    ...
    128	    /// This role in the observation hook's public vocabulary.
    129	    pub fn role(self) -> Role {

Resolution: One sentence at `Speaker`: "The observation hook's [`Role`] names the same election in a vocabulary kept free of protocol types; [`role`](Self::role) bridges to it." Or unify the two if the owner no longer wants the separation. Acceptance: the `Speaker` doc states why a second type exists, or one type remains.

### remote-codec-33: Two arms of the phase schedule carry no rule
- Where: src/tree/mirror/streaming/remote/codec/signal.rs:326-341 (related: signal.rs:137-154)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read)
- Seen by: prose (27); refutation: confirmed; history: the arms and the `VALID_PLACEMENTS` pin date from 3dac1b85 (no body); the structural reason exists only in `formal/MODEL.md` (`LeafRequests`, `TerminalLeafResolutions`), which AGENTS.md forbids citing from code, so it must be restated inline
- Owner-gated: no

`validate` explains why the initiator's opening stream admits only supplies and ends, but says nothing at the `LeafParentReplies` arm (why a leaf-parent reply never carries a nonempty `Query`) or the `TerminalLeafReplies` arm (why the responder's last stream admits only a reply-ending `Supply`). The snapshot pins the table; nothing in code states the rule it encodes, and the `StreamClass` variant docs are labels without rules. A table with one arm explained and two not invites the next editor to widen an arm without knowing what it protects; the schedule is wire format.

Evidence:

    329	            // One supplies-only reply (empty when pruning left nothing),
    330	            // then the stream end: the opening carries answers the
    331	            // responder is about to ask for, never questions of its own.
    332	            StreamClass::OpeningSupplies => {
    333	                matches!(self.signal, Signal::Supply(_) | Signal::End(_))
    334	            }
    335	            StreamClass::OpeningReply => true,
    336	            StreamClass::InteriorReplies => true,
    337	            StreamClass::LeafParentReplies => !matches!(self.signal, Signal::Query(_)),
    338	            StreamClass::TerminalLeafReplies => {
    339	                matches!(self.signal, Signal::Supply(Flow::End) | Signal::End(_))
    340	            }

Resolution: One comment per restricted arm, in the owner's words; my reading of the schedule: for `LeafParentReplies`, "A leaf-parent's children are leaves, which are supplied whole, never listed: a nonempty query has nothing to name."; for `TerminalLeafReplies`, "The responder's final stream answers only the initiator's empty leaf-parent queries, each with exactly one reply-ending supply." Say the same in one line at the two `StreamClass` variants. Acceptance: each arm of `validate` whose admitted set is a proper subset carries a comment stating its rule.

## Positives

- `Heads<const N: usize>` (encode.rs:46-78) renders the fixed heads on the stack with a capacity derived from the grammar's maxima, so body-free frames, the frames a session writes most often, allocate nothing; the claim is enforced by the committed allocator meter in `tests/encode_alloc.rs`, not asserted in prose.
- `ListingBuilder` (frame.rs:421-486) is one ingress gate for both listing surfaces: the async reader, the sync oracle, and the slice parser (hence the greeting) all drive the same `key`/`value_head`/`entry` discipline, so the deterministic-key-order rule and the canonical-child-order rule are enforced once as one rule, and `ListingBuilder::new` rejects an oversized map head before any entry is read.
- The budget algebra is closed under one boundary: `admits(body, record)` is `covers(body + record)` (budget.rs:147-162), so the encoder's flush rule and the decoder's ingress gate cannot drift; every wire constant is derived from the head grammar, and each closed form is pinned against an actual encode (`full_fan_frame_len_matches_an_actual_encode`, `record_len_matches_an_actual_push` sweeping head-width regimes, `admission_charges_the_frame_envelope`).
- `LeafRun` stays encoded on both sides of the wire (frame.rs:58-71): the encoder borrows it and the decoder validates framing once and yields records lazily, so the per-frame bound is the run's bytes; `a_zero_length_record_is_structurally_valid` pins the laziness rather than relying on it.
- The over-budget ingress gate (async_io.rs:429-463) decides legality from the first record's heads alone before buffering the body, and `multi_record_frames_are_held_to_the_run_budget` includes the truncated-after-heads case that distinguishes an early decision from a buffer-then-check implementation; `overbatched_supply_rejects_without_buffering_its_body` prices the premise under an allocation ceiling.
- `FrameRead`'s `# Reads` section (async_io.rs:50-63) states the bulk-read invariant in one paragraph, and the code visibly honors it through `Pending` and `Exact::head`; both async entry points carry accurate `# Cancel safety` sections (async_io.rs:108-115, encode/async_io.rs:54-60) naming the hazard and the two safe disciplines.
- The module doc's ingress-boundary statement (codec.rs:10-16) is protected by three contract tests (decode/tests.rs:467-563), each ending "Flipping this to rejection is a deliberate contract change, not drift", so prose and tests say the same thing and the tests would catch the prose going stale.
- `decode_both` (decode/tests.rs:855-881) is a tidy differential harness: two decoders, identical classification required, I/O text differences deliberately elided by `kind_signature`, and the testdocs say "in both decoders" where it applies.
- Every `expect` and `unreachable!` in production code carries a one-line proof that is true of the surrounding code (frame.rs:265, :268; async_io.rs:376, :523; codec.rs:116, :206, :212, :220, :239), and every peer-declared count or length is bounded before it drives a loop or an allocation.
- The `OpeningSupplies` arm of `validate` (signal.rs:329-331) is a model for branch commentary: the rule, its exception, and the reason in three lines.
- No error, assert, or log string in the partition contains an em-dash or a colon-fronted fragment; the Display strings read as plain English sentences.

## Open questions for Finch

- Is the error taxonomy diagnostic-only, or are `Signal`, `Flow`, `End`, and the stream-index error public vocabulary (remote-codec-30)? Recommendation: diagnostic-only: narrow the accessors to `pub(crate)`, rename the index error to a struct, give `Origin` and `InvalidSignalPlacement` a `Display` that avoids `Debug`, and enable `unnameable_types` so the class cannot recur.
- Should `DecodeErrorKind` carry typed `HeadError` and `ListingIssue` sources (remote-codec-9)? Recommendation: yes; it dissolves `listing_issue`, `head_detail`, the atlas exemption, and remote-codec-10 in one change, and matches the greeting's typed surface.
- Reopen B2 (remote-codec-24)? The ruling declined enforcement because the detector would never fire against an existing encoder; the per-record allocation and 4 KiB memset were not before you. Recommendation: enforce (hand-parse the atom as the greeting does), name the ruling and the two flipped pins in the commit.
- Where should the exactness clamp live (remote-codec-14)? Recommendation: in `resume_payload` (framing.rs), since its doc already promises exactness without a capacity precondition, so every present and future caller inherits it.
- Keep the sync oracle? Its independent coverage is narrower than its lines suggest (opener assembly and the whole-body read), but remote-codec-10 is exactly the class only an independent implementation catches, and retiring an instrument requires the replacement to demonstrate coverage first. Recommendation: keep it, give the struct a doc stating its role, and dissolve the shared fragments (remote-codec-8).
- `checked_run_len` at the encoder (remote-codec-23): a typed `EncodeErrorKind::SupplyTooLarge` that only a programmer error in the accumulator can reach, exempted from the atlas as unconstructible. Typed error or assertion? Recommendation: keep it typed (it is also the length conversion the run head needs) and let the doc name what it catches.
- Em-dashes in `//` comments: 116 sites across src, 5 in this partition, against 2 sites using ` -- `. CLAUDE.md prefers the double-hyphen. Recommendation: one crate-wide sweep or an amendment to the rule; fixing the five here alone would make the codec the outlier.
- `FAN` and `MAX_QUERY_CHILDREN` (remote-codec-3): one radix fan spelled as two constants that `budget.rs` multiplies together. Same quantity, or deliberately distinct (a fan of reactions versus a fan of listed children)? Recommendation: define one from the other and say why at the declaration.
- The `#[non_exhaustive]` rule (remote-codec-18): state it in the tree, and rule on `EncodeErrorKind`, `DecodeLeafError`, and `LeafRunError`. Recommendation: open (they grow with enforcement).
- A public getter for the effective run budget and a stat for the negotiated minimum (remote-codec-5)? Recommendation: add the getter and the `MAX_RUN_BUDGET_BYTES` re-export now; defer the stat until a user asks.
- `read_greeting` reads a peer-declared byte-string length through `read_payload` with no cap (greeting.rs:273). Memory tracks receipt, and under the honest-peer model the greeting is bounded by the peer's own version size, which is legitimately unbounded. Recommendation: no change.
- The two unaddressable-length `Shape` details (greeting.rs:171-172, :268-271) are exempt from construction tests only by a free comment at greeting/tests.rs:197-203; the atlas's `EXEMPT_MARKERS` covers `GreetingError` wholesale. Recommendation: acceptable as is (no 32-bit test host exists; the gate's wasm32 target builds the fuzz-fit guest, not tests), or move the two markers into `EXEMPT_MARKERS` if you want every exemption mechanical.

## Dropped

- Supply frames cost three transport writes plus a flush (perfapi 48): refuted; the piecewise writes exist for `FramePart` attribution, the sign is workload-dependent, and the reader's own resolution was to leave it unless a profile shows otherwise.
- Run buffers grow by amortized doubling and are never reused (perfapi 50): refuted; std's standard policy, both alternatives retain memory the budget doc says is released, and the resolution was measure-first with no measurement.
- Em-dashes in `//` comments (prose 30): the premise that the codebase's comment dash is the double-hyphen is false (116 em-dash sites against 2); a crate-wide style decision, moved to open questions.
- Dissolve the sync decoder oracle (perfapi 46): a design proposal with a concrete counterexample against it (remote-codec-10 is a defect only an independent implementation surfaces) and a recorded retention rationale; moved to open questions.
- Greeting truncation split across `Head(Truncated)`, `Shape("... is truncated")`, and `Listing(Truncated)` (refutation's new observation 3): below the bar; the greeting arrives whole through a declared length, so "cut short" inside it is a shape defect of the item, not a transport truncation a matcher would want to group.
- Narrow `parse_listing_map` and `write_listing` to `pub(super)` (part of structure 14): impossible; the `pub(crate) use` re-export at codec.rs:99 requires `pub(crate)` on the definitions (E0364). The remaining visibility items survive in remote-codec-21.
- The two-definition state roster (structure 10, first half): doctrine-compliant as written (a quantity computed two ways with a committed test comparing them; the table is what the snapshot pins); the residue survives as remote-codec-31.
- perfapi 42: duplicate of structure 0, merged into remote-codec-30 with its `Display`-through-`Debug` observation.
- correctness 37 and perfapi 53: duplicates of structure 2, merged into remote-codec-27.
- correctness 36: duplicate of structure 3, merged into remote-codec-28.
- perfapi 44: duplicate of structure 4, merged into remote-codec-22.
- prose 17, correctness 40, perfapi 43: duplicates of structure 5, merged into remote-codec-20.
- perfapi 52: the stream-count half of structure 11, merged into remote-codec-3 with structure 6 and 7.
- prose 18 and 31: merged into remote-codec-1 with structure 15.
- prose 20: duplicate of structure 16, merged into remote-codec-29.
- prose 21 and 23: merged with prose 22 into remote-codec-17.
- prose 25: merged with prose 26 into remote-codec-19.
- structure 12: reframed to a missing rationale sentence (remote-codec-32); the separation is deliberate and its reason recorded in 40b1e96a and observe.rs:8.
