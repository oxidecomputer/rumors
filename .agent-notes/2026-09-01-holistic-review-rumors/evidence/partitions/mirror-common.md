# Partition mirror-common: Mirror protocol shared layers: CBOR, framing, handshake, party, the streaming module root, protocol phases, messages, driver, erasure, tasks, stats

## Partition summary

This partition is the session envelope beneath the streaming mirror and the plumbing the mirror's two implementors share. The envelope is four small modules under `src/tree/mirror/`: `cbor` (the canonical CBOR head grammar, a writer and two readers held together by round-trip proptests), `framing` (exact-read payload buffering, the memory policy every declared-length body read funnels through), `handshake` (the fixed 30-byte preamble and its cancel-safe receiver), and `party` (the trailing identity hand-off). The plumbing lives under `src/tree/mirror/streaming/`: `protocol` and `protocol/peer` spell the type-level phase schedule and the 15/14-round `Peer`/`Client`/`Server` chains; `message` is the wire vocabulary and the greeting; `driver` expands the schedule into a body and routes the first error; `erased` is the boundary between the height-typed schedule and the height-erased walk; `tasks` holds the completion helpers; `stats` is the per-session counters that surface publicly as `SessionStats`; and `streaming.rs` elects roles and runs the descent. `mirror.rs` is the module root with the `contained` predicate and the two-sided `Error<C, S>`.

I read all 18 files with line numbers, 3579 lines in total, of which 920 are test code (`cbor/tests.rs`, `framing/tests.rs`, `handshake/tests.rs`, `party/tests.rs`, `mirror/tests.rs`) plus the 36-line inline test block at the bottom of `driver.rs`. I also read the files the findings depend on outside the partition (the codec's `record_prefix` and `lone_record_spans`, the gossip call sites of `reconcile`, `error.rs`, `channel.rs`, `remote/streams.rs`, `.cargo/mutants.toml`, the CBOR-wire review packet and the V1-retirement note) and the pinned library sources behind the one correctness finding (tokio 1.52.3 `io/util/read_buf.rs`, bytes 1.11.1 `BufMut for Vec<u8>`, std 1.97.1 `raw_vec`).

The code is in good shape. Every wire-facing parser is total over its input; every `expect` and `unreachable!` carries a one-line proof or is unreachable by type; the ingress suites are the right instruments (an exhaustive intent-byte sweep, a cut at every preamble prefix, a field-by-field oracle proptest, a chunked-versus-whole-read differential, a canonicality oracle on accepted hand-off bodies). The dominant issues are small and of two kinds. First, one real contract defect: `framing::resume_payload` promises `read_payload`'s exactness for any caller buffer, but `read_buf` fills all spare capacity, so a prefix buffer with capacity beyond `len` over-reads the transport; the single production caller escapes by one byte through std's minimum `Vec` capacity. Second, residue of three recent changes that the retiring commits did not sweep: the V1 retirement left a two-item list with one bullet in public `SessionStats` docs, "we selected" in a Display string, a `[u8; 6]` that used to be `LEGACY_MAGIC.len()`, and `LengthOverflow` in a module that no longer writes a length header; the move of `mirror_connected` to `driver.rs` left a comment naming the old file; and the re-denomination of `PayloadDepthLimit` to recursion steps missed the greeting field. The one owner-gated item reopens a recorded ruling (R2, keep-and-document on the two defensive `PreambleDefect` variants) on new evidence: since the V1 retirement, `Staged::buf` is a `[u8; V2_PREAMBLE_LEN]`, so the typed signature that dissolves both variants is now free. The rest is legibility and idiom, batched as nits.

## Findings

### mirror-common-1: Public `MirrorError` docs speak at maintainer altitude
- Where: src/tree/mirror.rs:44-49 (related: src/tree/mirror.rs:66-75, src/error.rs:52-53, src/error.rs:283-289, src/tree/mirror/streaming.rs:211-215)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read)
- Seen by: prose; refutation: confirmed; history: no rationale found (the coherence argument is deliberate and recorded in 9c8800a6e; its placement on public rustdoc was never ruled)
- Owner-gated: no

`mirror::Error<C, S>` reaches users as `MirrorError` (error.rs:53) and `Error::Mirror` (error.rs:289). Its variant docs say "The protocol participant supplied in the client position failed", but a library user supplies no participants and is not told that in `MirrorError` the client position is always the local replica's walk and the server position the wire proxy (the alias fixes both). The `From<C>` impl's rustdoc is entirely maintainer material (coherence overlap, frame-relative instantiation, the party boundary), rendered on a public item. Documentation altitude: public rustdoc names only what the API reaches; the why-only-one-`From` argument belongs to the maintainer.

Evidence:

    44	    /// The protocol participant supplied in the client position failed.
    45	    #[error("mirror client failed")]
    46	    Client(#[source] C),
    47	    /// The protocol participant supplied in the server position failed.
    48	    #[error("mirror server failed")]
    49	    Server(#[source] S),
    ...
    68	/// Only the first position can have this impl: its second-position mirror
    69	/// would overlap with it when `C = S`, and coherence permits one. This
    70	/// asymmetry shapes how the streaming driver uses the sum — each party runs
    71	/// its session at the *frame-relative* instantiation with its own error
    72	/// first, so `?` lifts either party's backend errors through this one impl,

Resolution: Variant docs: "The client-position participant failed; in a gossip session ([`MirrorError`](crate::MirrorError)) this is the local replica's walk." and "... the wire proxy for the peer." Move the coherence paragraph to a `//` comment above the impl and leave its rustdoc at "Lift a client-position error into the sum." Acceptance: the public docs on `Error<C, S>` and its `From` impl name only what a `MirrorError` holder can observe; the coherence argument survives as a maintainer comment.

### mirror-common-2: Hand-derived counts and caller rosters written as literals beside the constants that derive them
- Where: src/tree/mirror/cbor.rs:59-62 (related: src/tree/mirror/cbor.rs:66-68, src/tree/mirror/streaming/erased.rs:196, src/tree/mirror/streaming/message.rs:14-17, src/tree/mirror/streaming/message.rs:57, src/tree/mirror/handshake.rs:135-137, src/tree/mirror/handshake.rs:155, src/tree/mirror/handshake.rs:221, src/tree/mirror/handshake.rs:235, src/tree/mirror/handshake/tests.rs:285, src/tree/mirror/streaming/remote/adapter/tests/parking.rs:59)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read every cited site; confirmed `DISPUTED_REPLY_TRANSIENT_CEILING` at parking.rs:59 is the pin behind message.rs's figures)
- Seen by: prose; refutation: reframed (handshake.rs:16's "30 bytes" is the dialect's specification and stays); history: no rationale found, except that the handshake sites are R2's prescribed wording (39290c4af)
- Owner-gated: no

Derived arithmetic appears as literals where a named constant exists: "17 of the fixed item's 30 bytes" (handshake.rs:135-137, :155, :221, :235, tests.rs:285) derives from `V2_PREAMBLE_LEN` and `NETWORK_LEN`; "a 33-arm match" (erased.rs:196) is one arm per height `0..=32`; the ≈1.8 MB / ≈3.5 MB / ~4.3 KB figures (message.rs:14-17, :57) are pinned by `DISPUTED_REPLY_TRANSIENT_CEILING` but do not say so; and cbor.rs:59-62 and :66-68 enumerate the consumers of two constants to justify a cfg gate. No hand-maintained counts: a number that matters lives in a mechanically enforced place that prose cites by name; caller rosters rot the moment a caller is added.

Evidence:

    59	/// The production writers and validators spell the tag through its
    60	/// rendered head, [`SELF_DESCRIBED_HEAD`]; the number itself is consumed
    61	/// only by the test-gated capture renderer and by the pin test holding
    62	/// the rendered spelling to it, so it carries the same gate.

Resolution: Cite the constants by name ("the fixed item's `V2_PREAMBLE_LEN` bytes"), write "one arm per height", replace the cbor.rs roster with the property it justifies ("only test-gated code consumes the number; production spells it through `SELF_DESCRIBED_HEAD`"), and name the pin at message.rs ("pinned by `DISPUTED_REPLY_TRANSIENT_CEILING`"). The handshake sites vanish with mirror-common-10 if that is taken; otherwise respell them via the constants, which preserves R2's substance. Acceptance: no literal `30`, `17`, or `33` in prose where a named constant exists; no caller enumeration in a doc comment.

### mirror-common-3: `read_head` spells its width as `1 + (a - b - 1)`, and the info-to-width table is written twice with the second copy's error arms unreached by any test
- Where: src/tree/mirror/cbor.rs:191-194 (related: src/tree/mirror/cbor.rs:170-190, src/tree/mirror/cbor.rs:270-280, src/tree/mirror/cbor/tests.rs:46-59, src/tree/mirror/cbor/tests.rs:91-105, src/tree/mirror/streaming/remote/codec/decode/async_io.rs:302)
- Class / severity / confidence: simplification / low / high
- Provenance: demonstrated (second witness pass: with `extension_len`'s two error arms swapped, the 135 existing tests selected from the mutated function's callers all passed and only an added witness test failed, `initial byte 0x1c: async reader classified Indefinite, slice reader says Reserved`; the whole suite was not run; before the pass, verified by grep: the only assertions on `HeadError::Reserved` and `HeadError::Indefinite` are cbor/tests.rs:49-52, which call the slice `read_head`; `extension_len` is reached only from `read_head_async`, `read_head_io`, and the codec's `partial_head`)
- Seen by: structure, correctness; refutation: confirmed, plus a new verification-gap note; history: no rationale found (birth forms from 4dd2053c9)
- Owner-gated: no

Line 191 computes the consumed width as `1 + (input.len() - rest.len() - 1)`, which is `input.len() - rest.len()`; a self-cancelling pair makes a reader stop to check an identity. The additional-information-to-width mapping (24 → 1, 25 → 2, 26 → 4, 27 → 8, 28..=30 reserved, 31 indefinite) is written once in `read_head`'s match and again in `extension_len`. `async_heads_match_the_slice_reader` ties the two copies together only on canonical heads; no committed test feeds an initial byte with info 28..=31 to `read_head_async`, `read_head_io`, or the codec, so swapping `extension_len`'s two error arms would survive every suite. Legibility (finished code reads as evidently right) and adequacy (a criterion needs a committed demonstration that the bad mechanism fails it).

Evidence:

    191	    let width = 1 + (input.len() - rest.len() - 1);
    192	    if width != head_len(value) {
    193	        return Err(HeadError::NotShortest);
    194	    }
    ...
    277	        28..=30 => Err(HeadReadError::Malformed(HeadError::Reserved)),
    278	        _ => Err(HeadReadError::Malformed(HeadError::Indefinite)),

Resolution: Write `let width = input.len() - rest.len();`. Either restructure `read_head` to call `extension_len(initial)` and decode the argument by the returned width (one classification table), or add one row to `async_heads_match_the_slice_reader` (or a sibling point test) feeding `[(major << 5) | info]` for `info in 28..=31` to `read_head_async` and asserting the same `HeadError` the slice reader returns. Factoring the shared twelve lines of `read_head_async`/`read_head_io` into `assemble_head(initial, argument)` is optional. Acceptance: the four head proptests pass unchanged; a deliberate swap of the two arms at cbor.rs:277-278 fails a committed test.
Construction (run in the second witness pass): swap the two arms at cbor.rs:277-278 and run the suites that reach `extension_len`: every existing test passes at this commit (135 of 135 in the pass's selection; the whole suite was not run), because cbor/tests.rs:46-59 exercises the slice reader's own copy of the table (:188-189).

Witness: the second witness pass ran this construction (`witness/results.md`, `## mirror-common-3`). The two error arms of `extension_len` at cbor.rs:277-278 were swapped, a witness test feeding `[(major << 5) | info]` for `info in 28..=31` to `read_head_async` was appended to cbor/tests.rs, and every unit module that reaches `extension_len` (the `cbor`, every `remote::codec` submodule, `remote::streams`, `party`, and `proxy::tests::malformed`) plus the `gossip_snapshot` and `decode_alloc` binaries were run: 136 tests. Decisive output:

        Starting 136 tests across 60 binaries (703 tests skipped)
            FAIL [   0.017s] ( 11/136) rumors tree::mirror::cbor::tests::zz_witness_async_reader_classifies_reserved_and_indefinite
        assertion `left == right` failed: initial byte 0x1c: async reader classified Indefinite, slice reader says Reserved
         Summary [   8.722s] 136 tests run: 135 passed, 1 failed, 703 skipped

With the arms swapped, all 135 existing tests in the selection passed and only the added witness failed; the whole suite was not run (by instruction), but the selection covers every caller of the mutated function found by grep (src/tree/mirror/party.rs:151, src/tree/mirror/streaming/remote/streams.rs:761, src/tree/mirror/streaming/remote/codec/greeting.rs:240 and :254, src/tree/mirror/streaming/remote/codec/decode/async_io.rs:302 and :503, and the test-only src/tree/mirror/streaming/remote/codec/decode.rs:87 and :199). The `1 + (input.len() - rest.len() - 1)` reading at cbor.rs:191 is assessed by reading only. The edits were restored afterwards.

### mirror-common-4: Cancel-safety hazards stated inline instead of under the crate's `# Cancel safety` section
- Where: src/tree/mirror/cbor.rs:219-224 (related: src/tree/mirror/handshake.rs:244, src/tree/mirror/handshake.rs:264-282, src/link.rs:252, src/tree/mirror/streaming/remote/codec/decode/async_io.rs:108)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: eight sites in the crate use a `# Cancel safety` section; cbor.rs:223-224 states the hazard as a trailing sentence; handshake.rs:244 and :264 state cancel safety in prose without a section)
- Seen by: prose; refutation: reframed (`Staged` does state its cancel safety, at :244 and :264; only the section form and `fill`'s return-arm inventory are missing); history: no rationale found (the section convention predates cbor.rs's inline sentence)
- Owner-gated: no

`read_head_async` ends its summary paragraph with "Not cancel safe: a dropped future may have consumed part of the head." The crate spells this hazard as a `# Cancel safety` section elsewhere, so a reader scanning for the section misses it here. `Staged::fill`, whose reason to exist is cancel safety, states it only in prose and does not document its three outcomes (`Filled`; `Closed` only when no byte has arrived; `Truncated` on a close mid-frame). Hazards get uniform named sections.

Evidence:

    221	/// A clean end-of-stream *before the first byte* returns `Ok(None)`; an
    222	/// end-of-stream inside the head is an
    223	/// [`UnexpectedEof`](std::io::ErrorKind::UnexpectedEof) I/O error. Not
    224	/// cancel safe: a dropped future may have consumed part of the head.
    ...
    264	    /// Continue receiving the fixed frame without losing cancelled progress.
    265	    pub(crate) async fn fill<R>(&mut self, reader: &mut R) -> Result<Fill, Error>

Resolution: cbor.rs: move the sentence under `# Cancel safety`. handshake.rs:264: add `# Cancel safety` ("Cancel safe: bytes received before a drop stay in `self`, and the next call resumes from them.") and state the return arms. Acceptance: every async reader in the partition that is not cancel safe, and `Staged::fill`, carries a `# Cancel safety` section.

### mirror-common-5: The cbor proptests exclude major 7 without saying why, and build a tokio runtime per iteration where `pollster` serves
- Where: src/tree/mirror/cbor/tests.rs:10-10 (related: src/tree/mirror/cbor/tests.rs:27, src/tree/mirror/cbor/tests.rs:47, src/tree/mirror/cbor/tests.rs:65, src/tree/mirror/cbor/tests.rs:92-113, Cargo.toml:59)
- Class / severity / confidence: test-quality / nit / medium
- Provenance: verified (grep: no head writer in the crate passes major 7, every call uses a `MAJOR_*` constant; `pollster` is a workspace dev-dependency; the sibling suites use `pollster::block_on`)
- Seen by: prose; refutation: reframed (the testdocs are accurate: "Every head a writer emits" is true because no writer emits major 7; the exclusion is defensible but unstated); history: no rationale found
- Owner-gated: no

Four proptests draw `major in 0u8..7` while the exhaustive rejection loop sweeps `0u8..8`. Major 7's argument is a simple value or float, not a shortest-form integer, so excluding it from the round-trip family is a reasonable choice, but nothing at the site says so, and a reader comparing the two ranges has to reconstruct the reason. Separately, `async_heads_match_the_slice_reader` builds a current-thread tokio runtime inside the proptest closure on every case and again for the empty-stream check, while the reader is a slice needing no reactor and the sibling suites (`framing`, `handshake`, `party`) use `pollster::block_on`. A deliberate exclusion needs its one-line reason at the site; idiom consistency across sibling suites is legibility.

Evidence:

    10	    proptest!(|(major in 0u8..7, value: u64, trailing: Vec<u8>)| {
    ...
    47	    for major in 0u8..8 {
    ...
    95	        let head = tokio::runtime::Builder::new_current_thread()
    96	            .build()
    97	            .expect("runtime builds")

Resolution: Name the range once (`const WRITER_MAJORS: Range<u8> = 0..7;` with a comment "major 7 carries simple values and floats, which the head grammar never spells as a length") and use it in the four proptests; replace the two runtime builders with `pollster::block_on`. Acceptance: cbor/tests.rs contains no `tokio::runtime::Builder`; the major range is defined once with its reason.

### mirror-common-6: `LengthOverflow` is a codec-side error housed in the reader-side framing module
- Where: src/tree/mirror/framing.rs:55-65 (related: src/tree/mirror/streaming/remote/codec/frame.rs:306-313, src/tree/mirror/streaming/remote/codec/error.rs:75, src/tree/mirror/streaming/remote/adapter/error.rs:53, src/tree/mirror/streaming/remote/error.rs:19, src/error.rs:47)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (grep: the only constructor is `checked_run_len` at frame.rs:311; nothing in framing.rs names a `u32` or a header; the public path is `rumors::error::LengthOverflow` via remote/error.rs:19 and error.rs:47)
- Seen by: structure, perfapi; refutation: confirmed; history: deliberate but expired (framing owned `length_header` and `LENGTH_HEADER_LEN` until 368da2a5 deleted them; the V1-retirement note kept the type for survival, not placement)
- Owner-gated: no

`framing` documents itself as "the memory policy beneath every variable-length body read"; both its functions take `len: usize` and nothing in the file converts to `u32` or writes a header. `LengthOverflow` describes a length "which cannot be represented by a `u32` wire length header" and is produced only by the encoder's `checked_run_len`. A reader of `framing` meets a type none of its code can produce; the encoder imports its own error from a module about reading. Modules have one responsibility.

Evidence:

    55	/// A payload length which cannot be represented by a `u32` wire length
    56	/// header.
    57	#[derive(Debug, thiserror::Error)]
    58	#[error("payload length {len} exceeds the u32 framing limit")]
    59	pub struct LengthOverflow {

Resolution: Move `LengthOverflow` beside `checked_run_len` (or into `remote/codec/error.rs`); repoint the `pub use` at remote/error.rs:19 so `rumors::error::LengthOverflow` is unchanged. Acceptance: framing.rs defines only `PAYLOAD_CHUNK_LEN`, `chunk_boundary_cuts`, `read_payload`, `resume_payload`; the public path still resolves.

### mirror-common-7: The framing readers demand a `Sized` reader, forcing a double reborrow and a fully qualified path in `party.rs`
- Where: src/tree/mirror/framing.rs:77-80 (related: src/tree/mirror/framing.rs:95-99, src/tree/mirror/party.rs:116, src/tree/mirror/cbor.rs:225, src/tree/mirror/handshake.rs:267)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep of every caller; tokio 1.52.3 `io/util/read_buf.rs:11-14` takes `R: AsyncRead + Unpin + ?Sized`, and `resume_payload`'s body calls only `read.read_buf`)
- Seen by: structure, perfapi; refutation: confirmed; history: no rationale found
- Owner-gated: no

`read_payload` and `resume_payload` bound `R: AsyncRead + Unpin` (implicitly `Sized`), unlike their neighbours `cbor::read_head_async` and `Staged::fill`, which take `?Sized`. The party hand-off is generic over `R: ?Sized`, so its call has to reborrow `&mut &mut *reader` and, alone among the file's uses, spells the full crate path. Bounds no wider than needed; imports over long qualified paths.

Evidence:

    77	pub(crate) async fn read_payload<R: AsyncRead + Unpin>(
    78	    read: &mut R,
    79	    len: usize,
    80	) -> std::io::Result<Vec<u8>> {

    party.rs:
    116	    let bytes = crate::tree::mirror::framing::read_payload(&mut &mut *reader, len)

Resolution: Add `?Sized` to both bounds; `use crate::tree::mirror::framing::read_payload;` in party.rs and call `read_payload(reader, len)`. Acceptance: party.rs:116 reads `read_payload(reader, len)`; the other callers (greeting.rs:273, decode/async_io.rs:430 and :460, testing.rs:111) are unchanged.

### mirror-common-8: `resume_payload`'s exactness contract is false for admitted inputs: spare capacity beyond `len` over-reads the transport, and a prefix longer than `len` returns `Ok` over-long
- Where: src/tree/mirror/framing.rs:84-110 (related: src/tree/mirror/framing.rs:11-16, src/tree/mirror/framing.rs:74, src/tree/mirror/streaming/remote/codec/decode/async_io.rs:449-462, src/tree/mirror/streaming/remote/codec/decode/async_io.rs:473-487, src/tree/mirror/streaming/remote/codec/frame.rs:155-170, src/tree/mirror/streaming/remote/codec/frame.rs:324-329, src/tree/mirror/streaming/remote/codec/budget.rs:104-112, src/tree/mirror/framing/tests.rs:66-104)
- Class / severity / confidence: correctness / medium / high
- Provenance: assessed (read; the mechanism confirmed in the pinned library sources: tokio 1.52.3 `read_buf.rs:47-67` hands `poll_read` the whole `chunk_mut`, and bytes 1.11.1 `buf_mut.rs:1623-1636` returns `capacity - len` bytes for a `Vec<u8>`; std 1.97.1 `raw_vec/mod.rs:158-166` gives `Vec<u8>` a minimum non-zero capacity of 8; the construction below was not run)
- Seen by: prose, correctness, perfapi; refutation: confirmed (with the same library reading, against tokio 1.53.1 and bytes 1.12.1; the lockfile pins 1.52.3 and 1.11.1, whose code is identical at these lines); history: not already known (REVIEW.md:1158-1162 dismissed `reserve_exact` overshoot inside the loop, never a caller-supplied buffer's capacity; ed0f1775e states no precondition)
- Owner-gated: no

The doc at :92-94 says growth, exactness, and error behavior are `read_payload`'s, and `read_payload` (:74) promises "Never consumes a byte beyond `len`". The loop reserves only when `payload.len() == payload.capacity()` and otherwise calls `read.read_buf(&mut payload)`, which offers the reader every spare byte of capacity, not `len - payload.len()`. A caller-supplied prefix whose capacity exceeds `len` therefore consumes bytes belonging to the next frame, which the module doc (:11-16) makes the property that keeps a session boundary a stream position. The doc also says the prefix's bytes "count toward `len`" but not that they must not exceed it; a prefix longer than `len` skips the loop and returns `Ok` with an over-long buffer. The one production caller (`record_prefix` → `resume_payload`, async_io.rs:473-487 and :460) is safe by a one-byte margin it does not state: `Vec::new()` plus two head writes yields length 3 with capacity 8 (std's minimum), so over-read needs `len < 8`, while the smallest lone record is `RECORD_TAG_LEN` (2) + a one-byte length head + at least 6 bytes of content (a 3-byte version tag head, a 1-byte length head, at least one version byte since the empty version pads to one byte, at least one payload byte), so `len >= 9`; and `lone_record_spans` (frame.rs:324-329) establishes the prefix is at most `len` before the call. Correct for all inputs: a documented guarantee must hold for every input the signature admits or state its precondition; "our one caller happens to pass a full buffer" is the "we are the only writer" premise the doctrine warns about, and here it rests on std's allocation policy and constants in two other modules.

Evidence:

    84	/// Continue an exact `len`-byte payload read into `payload`, whose
    85	/// existing bytes — a prefix the caller already consumed from the same
    86	/// source — count toward `len`.
    ...
    92	/// where a read-then-splice would briefly hold the payload twice. Growth,
    93	/// exactness, and error behavior are [`read_payload`]'s (it is this
    94	/// function from an empty buffer).
    ...
    100	    while payload.len() < len {
    101	        if payload.len() == payload.capacity() {
    102	            let target = (payload.capacity() * 2).max(PAYLOAD_CHUNK_LEN).min(len);
    103	            payload.reserve_exact(target - payload.len());
    104	        }
    105	        if read.read_buf(&mut payload).await? == 0 {
    106	            return Err(std::io::ErrorKind::UnexpectedEof.into());
    107	        }
    108	    }
    109	    Ok(payload)

Resolution: Make the claim true by construction: bound each read by the bytes still owed, `use bytes::BufMut;` (already a dependency, Cargo.toml:126) and `read.read_buf(&mut (&mut payload).limit(len - payload.len())).await?` (`Limit<&mut Vec<u8>>` is `BufMut`). Keep the growth policy. State the remaining precondition (`payload.len() <= len`) in the doc and check it with an O(1) `debug_assert!` or return `InvalidData` (the sanctioned place for a runtime assert: a cheap spot check at a contract boundary). In framing/tests.rs, add a differential proptest for `resume_payload`: for `prefix_len <= len` and arbitrary prefix capacity, `resume_payload(rest, prefix, len)` equals `read_payload(prefix ++ rest, len)` byte for byte, truncation classification included, with trailing bytes left unread; the suite currently exercises `read_payload` only (:87, :115, :135). Acceptance: a committed test passes `resume_payload` a 3-byte prefix in a `Vec::with_capacity(64)`, `len = 9`, and 16 trailing transport bytes, and asserts the returned payload is exactly 9 bytes and the cursor stops at the trailing bytes; it fails at this commit and passes after the change; `chunked_read_matches_whole_read_reference` still passes.
Construction: in `src/tree/mirror/framing/tests.rs`: `let len = 9; let mut transcript = pattern(len, 5); transcript.extend_from_slice(b"next-frame-bytes"); let mut prefix = Vec::with_capacity(64); prefix.extend_from_slice(&transcript[..3]); let mut cursor: &[u8] = &transcript[3..]; let decoded = pollster::block_on(resume_payload(&mut cursor, prefix, len)).unwrap(); assert_eq!(decoded, &transcript[..len]); assert_eq!(cursor, b"next-frame-bytes");`. At this commit `payload.len()` (3) differs from its capacity (64), so no reserve runs, `chunk_mut` offers 61 bytes, and the slice reader copies all 22 remaining bytes in one `read_buf`: `decoded` has 25 bytes and `cursor` is empty. For the second clause: `resume_payload(&mut cursor, vec![0u8; 12], 9)` returns `Ok` with 12 bytes.

### mirror-common-9: "Handshake" names both the preamble module and the greeting exchange, against `message.rs`'s own definitions
- Where: src/tree/mirror/handshake.rs:1-8 (related: src/tree/mirror/streaming/message.rs:34-36, src/tree/mirror/streaming.rs:30, src/tree/mirror/streaming.rs:35-36, src/tree/mirror/streaming.rs:155, src/tree/mirror/streaming.rs:165, src/error.rs:21, src/error.rs:195)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the cited lines; grep for `mid-handshake` in error.rs finds :21 and :195, both about the preamble truncation)
- Seen by: prose; refutation: confirmed; history: no rationale found (the definition arrived in c5f1210a3 while editing this module's own doc and leaving line 1)
- Owner-gated: no

`message.rs` defines three terms as three things: the preamble is the fixed transport bytes, the handshake is the act of exchanging greetings, a `Greeting` is the message. The module that implements the preamble is named `handshake` and opens "The transport handshake opening every mirror session"; `streaming.rs`'s module doc uses both senses in one paragraph (`[`handshake`]` the greeting-exchange function at :30, `[`super::handshake`]` the preamble module at :36); and `handshake()` binds the received `Greeting` to `our_handshake` under a summary that says "Exchange versions". A term defined in the codebase must be used as defined everywhere else in it, or the definition is a trap.

Evidence:

    1	//! The transport handshake opening every mirror session.

    message.rs:
    34	/// Three terms, three things: the *preamble* is the fixed transport bytes
    35	/// that precede any message, the *handshake* is the act of exchanging
    36	/// greetings, and a `Greeting` is the message each side contributes to it.

Resolution: Adopt `message.rs`'s definitions: rename `pub(crate) mod handshake` to `preamble` (no public path moves; `PreambleDefect` is re-exported from error.rs by path) and open it "The fixed preamble opening every mirror session."; rewrite streaming.rs:35-36 as "first exchanges the fixed [`super::preamble`]"; rename `our_handshake` to `our_greeting` and make :155 "Exchange greetings and return both connected protocol states." In error.rs (another partition), "mid-handshake" → "mid-preamble" at :21 and :195. Acceptance: `grep -rn -i handshake src/tree/mirror` returns only the greeting-exchange sense.

### mirror-common-10: `Preamble::decode` takes `&[u8]` where every caller holds `[u8; V2_PREAMBLE_LEN]`, and pays for it with two public `PreambleDefect` variants that guard programmer error
- Where: src/tree/mirror/handshake.rs:111-159 (related: src/tree/mirror/handshake.rs:99-108, src/tree/mirror/handshake.rs:219-241, src/tree/mirror/handshake.rs:246, src/tree/mirror/handshake.rs:285-288, src/tree/mirror/handshake/tests.rs:283-290, src/error.rs:39, .cargo/mutants.toml:15-24, .agent-notes/2026-08-20-cbor-wire-review/REVIEW.md:72)
- Class / severity / confidence: simplification / medium / high
- Provenance: assessed (read: the width arithmetic by hand against `read_head`'s shortest-form check at cbor.rs:191-194 and `V2_PREAMBLE_LEN = 11 + 1 + 17 + 1` at :53; the call sites verified by grep: `Staged::validate` at :287 passes `&self.buf: [u8; V2_PREAMBLE_LEN]`, the two proptests pass full-width inputs; no `.cargo/mutants.toml` entry covers handshake.rs)
- Seen by: structure, prose, correctness, perfapi; refutation: confirmed (adopting [39]'s mechanism: keep `read_head` for the version so a wide canonical version head still yields `VersionMismatch { remote_version }`); history: already known, reopens R2 ("keep-and-document; do not delete `NetworkTruncated`", REVIEW.md:72, executed in 39290c4af) on one new fact: R2's premise was a `buf: [u8; PREAMBLE_MAX]` sliced per dialect, and 368da2a5 made it `[u8; V2_PREAMBLE_LEN]`
- Owner-gated: yes: removes two variants of the public `#[non_exhaustive] PreambleDefect` and reopens a recorded ruling

`decode` accepts an arbitrary slice although its only production caller passes the fixed array, so it re-derives the fixed width at runtime: two `// Defensive:` checks (:135-140, :148-153) are the sole constructors of `PreambleDefect::NetworkTruncated` and `PreambleDefect::TrailingBytes`, whose own docs say they are "Defensively reachable only" and guard "the decoder's width arithmetic against layout drift, not any input the current dialect admits"; the test file carries a standing exemption for them (tests.rs:283-290); and the slice indexings at :112 and :114 panic on any input shorter than the width the type could carry. After a canonical one-byte version head (`value == 2`) and a canonical one-byte network head (`value == 16`), exactly 17 of the 30 bytes remain, so neither check can fire. Layout drift is programmer error, and `prefix_matches_the_writers` plus `encode`'s `debug_assert_eq!` already catch it. A guard must name a constructible failure the committed tests cannot catch; never document what the types prevent; and the project's own mutants policy (mutants.toml:15-24) orders this disposition: "(1) refactor, so the mutated codepoint does not structurally exist (... a stronger type, a dissolved dead arm)". Both checks are value-equivalent mutant sites (`<` → `<=` at :138; `!input.is_empty()` → `false` at :151) with no roster entry. Since R2 was ruled, the typed signature became free, which is why the question is worth putting again.

Evidence:

    111	    fn decode(bytes: &[u8]) -> Result<Self, Error> {
    112	        if bytes[..V2_PREFIX.len()] != V2_PREFIX {
    ...
    135	        // Defensive: a validated version and network head leave 17 of the
    136	        // fixed item's 30 bytes here, so the 16 network bytes always fit;
    137	        // the bound keeps `split_at` in range under any layout drift.
    138	        if input.len() < NETWORK_LEN {
    139	            return Err(malformed(PreambleDefect::NetworkTruncated));
    140	        }
    ...
    148	        // Defensive: the one-byte intent item consumes the fixed item's
    149	        // last byte, so nothing can trail; the check guards any caller
    150	        // handing the decoder non-fixed input.
    151	        if !input.is_empty() {
    152	            return Err(malformed(PreambleDefect::TrailingBytes));
    153	        }
    ...
    246	    buf: [u8; V2_PREAMBLE_LEN],

Resolution: Take `bytes: &[u8; V2_PREAMBLE_LEN]` and split it with `split_first_chunk::<{ V2_PREFIX.len() }>()`, which types the remainder as `&[u8; 19]`. Keep the version diagnosis as it is (`cbor::read_head` over the remainder, `VersionMismatch` carrying `remote_version` for any canonical value other than 2, `Malformed(Version)` for a widened or non-uint head): once `value == 2` passes, canonicality means the head was exactly the remainder's first byte, so destructure the remainder as `[_version, network_head, network @ .., intent]` and both `network: &[u8; NETWORK_LEN]` and `intent: &u8` are type facts. Validate `network_head` and `intent` each as a one-byte head via `cbor::read_head(&mut &[*b][..])` with the same filters as today; a widened head on a one-byte slice reports `Truncated` and lands in the same `Malformed(Network)`/`Malformed(Intent)` class the tests pin (`intent_byte_space_is_exhaustive` classifies `0x18..` as `Malformed(Intent)`). Delete `NetworkTruncated`, `TrailingBytes`, both `// Defensive:` comments, the `expect("network width")`, the intent-width `expect` (a one-byte uint head's value is at most 23, so `intent.value as u8` with that sentence, or `Intent::from_byte(*intent & 0x1f)` after the major check), and the tests.rs exemption block. Dually, `encode(self) -> [u8; V2_PREAMBLE_LEN]` states the width in its type and drops the per-session `Vec` and its `debug_assert_eq!` (this half is not owner-gated). Acceptance: `PreambleDefect` has exactly `Version`, `Network`, `Intent`, each with a construction test; `decode` contains no `Defensive` comment and no `expect`; `intent_byte_space_is_exhaustive`, `widened_version_spelling_is_the_version_defect`, `version_mismatch_is_diagnosed_before_intent`, `arbitrary_preamble_decodes_by_the_oracle`, and `arbitrary_bytes_never_panic` pass with their classifications unchanged; a peer sending a canonical two-byte version head (`0x18 0x18`) still yields `VersionMismatch { remote_version: 24 }`.

### mirror-common-11: Two widths restated as literals beside the constants that name them
- Where: src/tree/mirror/handshake.rs:176-178 (related: src/tree/mirror/handshake.rs:31-32, src/tree/mirror/handshake.rs:37, src/tree/mirror/handshake.rs:114, src/error.rs:73, src/network.rs:25, src/network.rs:40, src/network.rs:55, src/network.rs:64, src/network.rs:80, src/network.rs:85)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read handshake.rs:37, :114, :178 and error.rs:73; grep of network.rs shows seven literal `16`s and no width constant)
- Seen by: structure, perfapi; refutation: confirmed; history: deliberate but expired (the `6` was `LEGACY_MAGIC.len()` via `MAGIC_LEN`; 368da2a5 renamed the constant to `MISMATCH_PREVIEW_LEN` and updated the slice at :114 but not the two array types)
- Owner-gated: no

`MISMATCH_PREVIEW_LEN` names the preview width and the slice at :114 uses it, but the array type in both the crate-private and the public `MagicMismatch` variant is spelled `[u8; 6]`, so the constant and the type can drift. `const NETWORK_LEN: usize = 16;` restates `Network`'s width, which `network.rs` itself spells as the literal `16` at its struct and six further sites with no shared constant. Named constants over magic numbers: a width declared once cannot drift from its uses.

Evidence:

    176	    /// The peer is not speaking the rumors protocol.
    177	    #[error("peer is not a rumors stream (leading bytes: {remote_magic:x?})")]
    178	    MagicMismatch { remote_magic: [u8; 6] },
    ...
    31	/// Canonical width of one network identifier.
    32	const NETWORK_LEN: usize = 16;

Resolution: Make `MISMATCH_PREVIEW_LEN` `pub(crate)` and write `[u8; MISMATCH_PREVIEW_LEN]` at handshake.rs:178 and error.rs:73. Add `Network::LEN` (or use `size_of::<Network>()`) in network.rs (another partition) and replace `NETWORK_LEN` and network.rs's literals with it. Acceptance: `grep -n '\[u8; 6\]' src/tree/mirror/handshake.rs src/error.rs` is empty; the network width has one definition.

### mirror-common-12: `VersionMismatch` says "we selected" a protocol nothing selects any more
- Where: src/tree/mirror/handshake.rs:180-184 (related: src/tree/mirror/handshake.rs:204, src/error.rs:14, src/error.rs:76, src/error.rs:179, src/error.rs:201, src/protocol.rs:1, .agent-notes/2026-09-01-v1-retirement/README.md:153-160)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: no `fn protocol(` exists in src; `Protocol` has the single variant `V2 = 2`; handshake.rs:127 hard-codes `local_protocol: Protocol::V2`; the wording survives at the seven listed sites)
- Seen by: structure, prose; refutation: confirmed; history: deliberate but expired (accurate under 83edcd944's two-variant, builder-selectable `Protocol`; 368da2a5 removed the builders and swept five other "selected" lines from handshake.rs, leaving :180 and :204; the retirement note rules that `local_protocol` stays as wire vocabulary)
- Owner-gated: no

With the `.protocol()` builders gone and `Protocol` down to one variant, no code path selects a protocol. The Display text "we selected {local_protocol:?}" (mirrored at error.rs:76, the string users see) and "the selected dialect" (handshake.rs:204, error.rs:179, :201) describe a removed affordance, and error.rs:14's remedy column ("select the same [`Protocol`] at both ends") invites a repair that no longer exists when the only remedy is aligning crate versions. No ghost references: prose, Display text included, may not describe code that no longer exists.

Evidence:

    180	    #[error("peer speaks rumors protocol version {remote_version}, we selected {local_protocol:?}")]
    181	    VersionMismatch {
    182	        local_protocol: Protocol,
    183	        remote_version: u64,
    184	    },
    ...
    204	/// peer opened as a rumors stream of the selected dialect, but one

Resolution: "peer speaks rumors protocol version {remote_version}; this build speaks {local_protocol:?}" here and, byte-identical, at error.rs:76; handshake.rs:204 "opened as a rumors stream of this build's dialect"; sweep error.rs:14, :179, :201 and protocol.rs:1 ("Selectable") in their own partitions. Dropping `local_protocol` from the public variant is the further step the retirement note already declined. Acceptance: `grep -rn -i 'we selected\|selected dialect\|selectable' src` is empty; the two `VersionMismatch` strings are identical; tests/handshake.rs still passes.

### mirror-common-13: Two small inaccuracies: `Staged::is_empty` is documented by its caller's question, and the streaming module doc calls the tree-less proxy "backed by trees"
- Where: src/tree/mirror/handshake.rs:259-262 (related: src/tree/mirror/streaming.rs:32-33, src/tree/mirror/streaming/protocol.rs:5-8, src/tree/mirror/streaming/remote/proxy/start.rs:168)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read; protocol.rs:5 says "A party that owns no tree — the [`remote`] proxy")
- Seen by: prose; refutation: reframed (the "tag" sub-claim about party/tests.rs:172-173 is dropped: `before` itself calls the party codec's presence bits tags); history: no rationale found ("backed by trees" was accurate and deliberately exclusive at 5402b31b6; cbfe1aff3 added the tree-less proxy to the same sentence)
- Owner-gated: no

`is_empty`'s doc states what its caller does with the answer rather than the predicate. streaming.rs:32-33 says implementors "backed by trees" start with either `materialized::Handshaking::start` or `remote::Handshaking::start`, but protocol.rs:5 says the remote proxy owns no tree. A doc's first sentence stands alone and must be accurate against the code.

Evidence:

    259	    /// Whether an idle-boundary hang-up can still be a clean goodbye.
    260	    pub(crate) fn is_empty(&self) -> bool {

    streaming.rs:
    32	//! Implementors backed by trees start with either
    33	//! [`materialized::Handshaking::start`] or [`remote::Handshaking::start`].

Resolution: `is_empty`: "No preamble byte has arrived yet: a hang-up here is a clean goodbye, not a truncation." streaming.rs:32-33: "The walk starts with [`materialized::Handshaking::start`]; the proxy with [`remote::Handshaking::start`]." Acceptance: both sentences read correctly against `Staged::fill` and protocol.rs:5.

### mirror-common-14: The preamble totality proptest almost never passes the magic check, so it demonstrates one arm of the parser it claims is total
- Where: src/tree/mirror/handshake/tests.rs:257-263 (related: src/tree/mirror/handshake.rs:111-119, src/tree/mirror/handshake/tests.rs:210-255, src/tree/mirror/handshake/tests.rs:296-334)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read: the strategy draws all 30 bytes from `any::<[u8; V2_PREAMBLE_LEN]>()`, so the 11-byte prefix matches `V2_PREFIX` with probability 2^-88)
- Seen by: correctness; refutation: confirmed; history: no rationale found (strategy unchanged since 4dd2053c9; the CBOR-wire review's clean bill covers diagnostic order, not this strategy's reach)
- Owner-gated: no

`arbitrary_bytes_never_panic` returns `MagicMismatch` at :112-117 on effectively every case, so the version, network, and intent arms are never reached under this strategy. Those arms are exercised by `arbitrary_preamble_decodes_by_the_oracle`, but only with one-byte version and intent heads (`0..=0x17`) and the fixed network head `0x50`; canonical wide version heads (`0x19 0x01 0x00`), wide or widened network heads, and multi-byte intent heads are outside every strategy in the file (the two wide-version point tests at :296-334 are the only exceptions). The testdoc claims "the parser is total over its fixed-width input"; a generator that never leaves the rejection arm would also pass a parser that panicked on any valid prefix. Property tests sample the family the claim is about.

Evidence:

    257	    /// Arbitrary bytes in the preamble's place decode to a typed error or
    258	    /// a valid preamble, never a panic: the parser is total over its
    259	    /// fixed-width input.
    260	    #[test]
    261	    fn arbitrary_bytes_never_panic(bytes in any::<[u8; V2_PREAMBLE_LEN]>()) {
    262	        let _ = Preamble::decode(&bytes);
    263	    }

Resolution: Supplement the strategy with a prefix-valid arm, weighted heavily: `prop_oneof![1 => any::<[u8; V2_PREAMBLE_LEN]>(), 8 => any::<[u8; 19]>().prop_map(|tail| { let mut b = [0u8; V2_PREAMBLE_LEN]; b[..11].copy_from_slice(&V2_PREFIX); b[11..].copy_from_slice(&tail); b })]`, and optionally a third arm that also fixes the version byte at `Protocol::V2 as u8` so the network and intent arms are reached on most cases. Acceptance: a temporary `unreachable!()` inserted after the magic check (handshake.rs:119) is hit by the suite; the committed strategy reaches every `Err` arm of `decode` (checked once by counting arm hits).
Construction: insert `unreachable!("reached past the magic check")` at handshake.rs:119 and run `arbitrary_bytes_never_panic` alone: it passes all 256 default cases at this commit.

### mirror-common-15: `HandOffDefect::Undecodable` admits `Decode::Io`, a state `decode_party` never produces
- Where: src/tree/mirror/party.rs:43-51 (related: src/tree/mirror/party.rs:125-138, src/error.rs:40, crates/before/src/error.rs:68-92)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: verified (read `decode_party` at party.rs:131-138, which routes `Decode::Io` to `Error::Io`; `before::error::Decode` has `Truncated`, `TrailingBits`, `NotCanonical`, `Io`; `HandOffDefect` is public via error.rs:40)
- Seen by: perfapi; refutation: confirmed; history: deliberate and holds in shape (R5 ruled the typed `Undecodable(before::error::Decode)` form; the `Io` pass-through is stated at `decode_party`, :127-130, not at the variant a public reader opens)
- Owner-gated: no (the one-sentence doc fix; the fuller three-variant reshaping would reopen R5 and is owner-gated)

The public variant wraps `before::error::Decode` whole, but only the non-`Io` variants ever reach it: the body is decoded from a slice and a reader failure surfaces as `Error::Io`. A user matching `HandOffDefect::Undecodable(Decode::Io(_))` writes a dead arm, and the variant doc explains the truncation case but not this one. Types-first: a public payload wider than the states produced is a contract the caller cannot read from the type; where the type stays, the doc must say what the type does not.

Evidence:

    43	    /// The byte string's content is not one canonical party encoding.
    44	    ///
    45	    /// The body arrived whole — exactly the length its head declared —
    46	    /// so this is never a transport cut: the content itself is wrong.
    ...
    51	    Undecodable(before::error::Decode),
    ...
    132	    Party::decode(bytes).map_err(|defect| match defect {
    133	        before::error::Decode::Io(e) => Error::Io(e),

Resolution: Add one sentence to the variant doc: "`Decode::Io` never appears here: the body is decoded from a slice, and a reader failure surfaces as [`Error::Io`]." The fuller alternative (a crate-owned `Truncated`/`TrailingBits`/`NotCanonical` enum with `From<Decode>` for the non-`Io` arms) is an owner decision. Acceptance: the variant doc names the unreachable arm, or the type excludes it.

### mirror-common-16: `Handshaken` keeps a cloned `Greeting` to read two scalars, and the `(len, version)` election key is spelled three ways
- Where: src/tree/mirror/streaming.rs:88-92 (related: src/tree/mirror/streaming.rs:101-104, src/tree/mirror/streaming.rs:117-131, src/tree/mirror/streaming.rs:165-180, src/tree/mirror/streaming.rs:184-191, src/tree/mirror/streaming/message.rs:141-146, src/peer/gossip.rs:1156, src/peer/gossip.rs:1160, src/peer/gossip.rs:1221)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep: every `.peer()` reader outside the file reads `.version` only; `reconcile` reads `peer.version` and `peer.set_len`; both `complete_connect` implementations take `Greeting` by value, materialized.rs:544 and remote/proxy/start.rs:168)
- Seen by: structure, perfapi; refutation: confirmed; history: no rationale found (the clone dates from 0aa29ed94, the commit that made the greeting carry the root listing; the two length scalars were added by 872aaaa19)
- Owner-gated: no

`handshake` clones the peer's `Greeting` (its `listing` is up to a full root fan, plus a `Version`) to hand one copy to `complete_connect` and retain the other in `Handshaken.peer`, from which only `version` and `set_len` are ever read. `descend` then takes `local_version, local_len, remote_version, remote_len` as four adjacent scalars that `reconcile` unpacks and re-pairs in a different order for `message::initiates(len, version, peer_len, peer_version)`: one concept, the role-election key, spelled three ways, with same-typed adjacent parameters the compiler cannot keep in order. `peer()` destructures `self` to return one field. Clone where a move would do (sign fixed, per session, negligible bytes); types-first legibility for the election key.

Evidence:

    88	    our_version: Version,
    89	    /// Our advertised live message count: our half of the role election's
    90	    /// primary key ([`message::initiates`]).
    91	    our_len: u64,
    92	    peer: message::Greeting,
    ...
    101	    pub(crate) fn peer(&self) -> &message::Greeting {
    102	        let Handshaken { peer, .. } = self;
    103	        peer
    104	    }
    ...
    169	    let client = client
    170	        .complete_connect(peer.clone())

Resolution: In `handshake`, take `peer_version` and `peer_len` before `complete_connect(peer)` and store them in place of `peer: Greeting`, symmetric with `our_version`/`our_len`; rename `peer()` to `peer_version()` (three gossip.rs readers). Optionally name the pair (`ElectionKey { set_len: u64, version: Version }`) so `descend(local, remote, ours, theirs)` and `initiates(ours, theirs)` speak one vocabulary. Acceptance: no `peer.clone()` in streaming.rs; `Handshaken` holds no `Greeting`; gossip.rs compiles against `peer_version()`; the streaming suites pass unchanged.

### mirror-common-17: The descent future is boxed twice: `reconcile` returns a `BoxFuture` and every caller `Box::pin`s it again
- Where: src/tree/mirror/streaming.rs:110-116 (related: src/peer/gossip.rs:1108-1126, src/peer/gossip.rs:1164, src/peer/gossip.rs:1223, src/peer/gossip/tests.rs:218, src/lib.rs:300-302, tests/future_size.rs:1-15)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read the three call sites, each `let descent: BoxFuture<'_, _> = Box::pin(handshaken.reconcile());`; `#![deny(clippy::large_futures)]` at lib.rs:302)
- Seen by: structure, perfapi; refutation: confirmed; history: deliberate but expired (before 83edcd944 the callers' box carried the comment "Boxed: the descent state machine is a large future, and the codec buffers inflate it past the crate-wide `large_futures` ceiling"; 83edcd944 moved the box into `reconcile` and rewrote every caller with a second box, deleting the comment; neither box has a rationale today)
- Owner-gated: no

`Handshaken::reconcile` already returns `Pin<Box<dyn Future>>`; all three callers wrap it in another `Box::pin`, producing `Pin<Box<Pin<Box<dyn Future>>>>` coerced back to `BoxFuture`: one redundant allocation per session, one extra vtable hop per poll of the descent, and an annotation that reads as if it were doing the erasure. Legibility: a reader asking why the future is boxed twice finds no answer at either site; the codegen-boundary rationale lives on the outer `Reconciliation::reconcile` shell (gossip.rs:1118-1126), which owns its own box.

Evidence:

    110	    pub(crate) fn reconcile<'a>(
    111	        self,
    112	    ) -> BoxFuture<'a, Result<(C::Output, S::Output), Error<C::Error, S::Error>>>
    113	    where
    114	        Self: 'a,
    115	    {
    116	        Box::pin(async move {

    gossip.rs:
    1164	            let descent: BoxFuture<'_, _> = Box::pin(handshaken.reconcile());

Resolution: Keep the inner box (it is what satisfies `clippy::large_futures` at the await sites) and delete the callers' re-boxing: `let (root, (read, write)) = handshaken.reconcile().await.map_err(streaming_error)?;` at gossip.rs:1164-1165, :1223-1224, and gossip/tests.rs:218-219. State the reason at `reconcile`'s doc ("Boxed: the descent state machine is a large future; the box keeps every await of it under the crate's `large_futures` ceiling."). Acceptance: exactly one `Box::pin` stands between `reconcile`'s body and each `.await`; `grep -rn 'Box::pin(handshaken.reconcile())'` is empty; gossip, bootstrap, and `tests/future_size.rs` pass unchanged.

### mirror-common-18: Redundant `std::future::Future` imports under edition 2024
- Where: src/tree/mirror/streaming/driver.rs:3-3 (related: src/tree/mirror/streaming/tasks.rs:3, src/tree/mirror/streaming/protocol.rs:62, Cargo.toml:86)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (Cargo.toml:86 `edition = "2024"`, whose prelude exports `Future`; protocol.rs:62 writes `impl Future` with no import)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (the lens's inference that the files predate an edition bump is wrong: the crate has been edition 2024 since its first Cargo.toml; the import is redundant from birth)
- Owner-gated: no

Two files in one module family import `Future` explicitly while a third uses it from the prelude. Harmless, and a small inconsistency a reader notices.

Evidence:

    3	use std::{future::Future, pin::pin};

Resolution: `use std::pin::pin;` in driver.rs:3 and tasks.rs:3 (six more such imports exist outside the partition: link.rs:155, adversarial.rs:4, instrumented.rs:3, faulting.rs:3, adapter/encode.rs:1, routed.rs:168). Acceptance: both files compile without the import.

### mirror-common-19: Two unrelated types named `ErrorRoute`, both "one-slot first-error routes", in sibling modules
- Where: src/tree/mirror/streaming/driver.rs:19-23 (related: src/tree/mirror/streaming/driver.rs:34-44, src/tree/mirror/streaming/remote/streams.rs:497-520, src/tree/mirror/streaming/remote/streams.rs:528)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (grep `struct ErrorRoute`: driver.rs:20 and streams.rs:498; read both)
- Seen by: structure; refutation: confirmed (the streams one carries a `supply_failure` slot and `supply_failed`, so the unification is not trivial reuse; the rename is the load-bearing part); history: no rationale found
- Owner-gated: no

`driver::ErrorRoute<E, S>` (a `try_send` reporter with a `wrap: fn(E) -> S`) and `remote::streams::ErrorRoute` (a `try_send` reporter over `StreamError` with a deposited supply failure) share a name, a one-capacity `mpsc` channel, and a `report` method with the same "first wins, the rest are cascade" semantics, but are distinct types with distinct receivers (`FirstError` versus `FirstStreamError`). A reader following `ErrorRoute` from the driver lands in `streams.rs` and vice versa. One name, one thing.

Evidence:

    19	/// One endpoint's typed route into the shared session error.
    20	pub struct ErrorRoute<E, S> {
    21	    send: mpsc::Sender<S>,
    22	    wrap: fn(E) -> S,
    23	}

    streams.rs:
    496	/// The reporting half of the session's one-slot first-error route.
    498	pub struct ErrorRoute {

Resolution: Minimum: rename the streams one `StreamErrorRoute`, matching its `FirstStreamError` twin. Optional: build the streams reporter on `driver::ErrorRoute<StreamError, StreamError>` (identity wrap) and keep only the `supply_failure` slot local, so the first-wins semantics are stated once. Acceptance: `grep -rn 'struct ErrorRoute' src` returns one definition, or two with distinct names.

### mirror-common-20: Visibility wider than use: `pub` items with in-file-only callers
- Where: src/tree/mirror/streaming/driver.rs:65-72 (related: src/tree/mirror/streaming/driver.rs:19-23, src/tree/mirror/streaming/driver.rs:47, src/tree/mirror/streaming.rs:48, src/tree/mirror/streaming.rs:184, src/tree/mirror/cbor.rs:142-145, src/lib.rs:322)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep: `ErrorRoute`, `FirstError`, `race_session`, `error_routes` have no reader outside driver.rs; `descend(` is called only at streaming.rs:124; `mod driver` and `mod tree` are private; no `unreachable_pub` lint is configured anywhere in the tree)
- Seen by: structure; refutation: confirmed; history: mixed (driver.rs's items had no outside callers even at birth; `descend`'s `pub(crate)` was load-bearing until 368da2a5 deleted `alternating.rs`, its other caller)
- Owner-gated: no

`ErrorRoute`, `FirstError`, `race_session`, and `error_routes` are `pub` inside the private `driver` module and used only there; `descend` is `pub(crate)` with one caller in its own file; `cbor::Head` is `pub` with undocumented `pub` fields beside `pub(crate)` siblings in a `pub(crate)` module. Because `tree` is private, nothing leaks; the cost is signal: a reader infers external callers that do not exist.

Evidence:

    65	/// The receiving side of a session's first-error route.
    66	pub struct FirstError<E>(mpsc::Receiver<E>);
    67	
    68	/// Race a session against response errors, preserving their causal priority.
    69	pub async fn race_session<O, E>(

Resolution: Make the four driver items private and `descend` private; `Head` `pub(crate)` with one-line field docs; consider `#![warn(unreachable_pub)]` for the crate so the compiler keeps this honest. Acceptance: the listed items are narrowed, or `unreachable_pub` is clean for these files.

### mirror-common-21: The `mirror!` party tuples' reason lives only in commit history
- Where: src/tree/mirror/streaming/driver.rs:103-114 (related: src/tree/mirror/streaming/driver.rs:142-148, src/tree/mirror/streaming/driver.rs:161-171)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (git: aa22c2a2b and 1e9ea0372 record the rationale; `git grep 'mirror!' aa22c2a2b -- src/tree/mirror/alternating.rs` is empty, and the three call sites at aa22c2a2b were segments of one schedule; `git show aa22c2a2b^:src/tree/mirror/streaming.rs` shows the pre-macro driver was already the straight-line `seq!` form)
- Seen by: structure (as "V1-era generality with one caller, inline it"); refutation: confirmed as a legibility proposal; history: deliberate and holds (the macro's rationale is one-line-per-phase legibility, recorded in aa22c2a2b and 1e9ea0372; V1 never used it)
- Owner-gated: no

The macro's doc says what each step does but not why every party ident carries a `(state, route)` tuple and why a produced stream rides in the producer's own tuple: macro hygiene bars an ambient `msgs` local from crossing invocations (and `seq!` pastes one expansion per iteration), so only the caller's idents can carry the pending stream from one step to the next. A reader of `@one` reverses this from the rules. The structure lens's proposal to inline the expansion reverts to the form the driver had before aa22c2a2b, which the owner replaced deliberately; without new evidence that stays a recorded choice, and the actionable residue is stating the rationale at the site.

Evidence:

    103	/// Expand the type-level phase schedule into the connected driver's body.
    104	///
    105	/// Each step diverts one response stream, advances its counterparty, and
    106	/// retains the producer's next state. The terminal joins both sides after
    107	/// mapping their distinct errors into the session error type.
    108	macro_rules! mirror {
    109	    (@one $a:ident >> $b:ident.$m:ident) => {
    110	        let ((msgs, state), route) = $a;
    111	        let msgs = divert(msgs, route.clone());
    112	        let $a = (state, route);
    113	        let $b = ($b.0.$m(msgs), $b.1);
    114	    };

Resolution: Add to the macro doc: "Each party ident is bound to a `(state, route)` pair, and a produced stream rides in the producer's own pair until its consumer's step: macro hygiene keeps a `msgs` local from crossing invocations, so only the caller's identifiers can carry the pending stream from one step (and one `seq!` iteration) to the next." Acceptance: the tuple encoding is explained where it is defined.

### mirror-common-22: `driver.rs` carries an inline test module against the sibling-file convention
- Where: src/tree/mirror/streaming/driver.rs:203-238 (related: AGENTS.md:72-74, src/testing.rs:397, src/testing/transport.rs:802, src/tree/mirror/streaming/testing/failing.rs:268, src/tree/mirror/streaming/backend/local/adversarial.rs:128)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep `^\s*mod tests {` in src: five inline blocks; driver.rs is the only production module among them, the other four are test scaffolding; every other module in this partition uses `mod tests;`)
- Seen by: structure, prose, correctness, perfapi; refutation: confirmed; history: no rationale found (the block, cbfe1aff3 on 2026-07-15, predates the AGENTS.md rule, dfd19c447 on 2026-07-24; never swept)
- Owner-gated: no

The two `race_session` tests live in an inline `#[cfg(test)] mod tests { ... }` at the bottom of the production file. AGENTS.md, "Writing tests": "Unit and protocol tests live in a sibling file: `mod tests;` in the source, `tests.rs` next to it", for brevity of reading the implementation.

Evidence:

    203	#[cfg(test)]
    204	mod tests {
    205	    use std::convert::identity;
    206	    use std::task::Poll;

Resolution: Move lines 205-237 to `src/tree/mirror/streaming/driver/tests.rs` and replace the block with `#[cfg(test)] mod tests;`. Whoever owns the four scaffolding files can decide whether test scaffolding is exempt. Acceptance: `grep -n 'mod tests {' src/tree/mirror/streaming/driver.rs` is empty; both tests run from the sibling file.

### mirror-common-23: `erased.rs` module doc compares today's code to a prior design state ("exactly as before")
- Where: src/tree/mirror/streaming/erased.rs:29-30 (related: AGENTS.md:128-130)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read)
- Seen by: prose; refutation: confirmed; history: contradicts the hard rule (the clause compares to the pre-erasure design of d1b7f00e; AGENTS.md's first hard rule forbids prose referring to code that no longer exists)
- Owner-gated: no

"is a compile error, exactly as before" compares the present design to one that no longer exists. The sentence is complete and true without the clause. Prose speaks in the present tense; provenance lives in git.

Evidence:

    29	//! Outside the walk, pairing a height-5 payload with a height-6 consumer
    30	//! is a compile error, exactly as before: the schedule's typestates and

Resolution: Delete ", exactly as before". Acceptance: `grep -n 'as before' src/tree/mirror/streaming/erased.rs` is empty.

### mirror-common-24: `ReplyResultStream` stores a fn pointer that its own type parameters already determine
- Where: src/tree/mirror/streaming/erased.rs:116-139 (related: src/tree/mirror/streaming/erased.rs:168-188)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read: the only constructor at :183-186 sets `assume` to `|item| item.map(assume_reply::<B, H>)`; `B` and `H` are the struct's parameters)
- Seen by: structure, perfapi; refutation: confirmed; history: no rationale found
- Owner-gated: no

The `assume` field can hold exactly one value and is called per item in `poll_next`; `assume_reply::<B, H>` is nameable directly inside the `impl<B, H, Err> Stream` block. A field with one possible value is `PhantomData` in disguise, and the indirection invites the reader to look for a second function.

Evidence:

    121	    inner: ReceiverStreamOf<Result<Reply<B::Erased>, Err>>,
    122	    assume: fn(Result<Reply<B::Erased>, Err>) -> Result<message::Reply<B, H>, Err>,
    ...
    185	            assume: |item| item.map(assume_reply::<B, H>),

Resolution: Replace the field with `_height: PhantomData<fn() -> H>` and write `.map(|item| item.map(|r| r.map(assume_reply::<B, H>)))` in `poll_next`; `reply_channel` no longer passes a closure. Acceptance: `ReplyResultStream` has no fn-pointer field; the streaming suites pass unchanged.

### mirror-common-25: The channel module's test/production stream swap is re-implemented in two consumers
- Where: src/tree/mirror/streaming/erased.rs:141-159 (related: src/tree/mirror/streaming/erased.rs:46-47, src/tree/mirror/streaming/materialized/common.rs:47-65, src/tree/mirror/streaming/channel.rs:75-90, src/tree/mirror/streaming/channel/instrumented.rs:176, src/tree/mirror/streaming/remote/adapter/decode.rs:84)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (read channel.rs:75-90, the cfg split of `Receiver`/`Sender`/`channel`; instrumented.rs:176 `impl<T> Stream for Receiver<T>`; common.rs:47-65 `ok_channel_with` and `OkReceiverStream` re-derive the same split; the only other `ReceiverStream` use, adapter/decode.rs, wraps a locally created tokio channel)
- Seen by: structure; refutation: confirmed; history: no rationale found
- Owner-gated: no

`channel.rs` owns the cfg split between tokio's `Receiver` (production, not a `Stream`) and the instrumented `Receiver` (test, a `Stream`), but never exports a stream-typed receiver, so `erased.rs` defines a cfg'd `ReceiverStreamOf` alias plus a cfg'd `receiver_stream` function, and `materialized/common.rs` defines the same split again as `OkReceiverStream` plus `ok_channel_with`. Two consumers re-derive a fact that belongs to the module that created it, and the two derivations already differ in name and shape.

Evidence:

    143	#[cfg(test)]
    144	type ReceiverStreamOf<E> = Receiver<E>;
    ...
    147	#[cfg(not(test))]
    148	type ReceiverStreamOf<E> = ReceiverStream<E>;
    149	
    150	fn receiver_stream<E: Send>(receiver: Receiver<E>) -> ReceiverStreamOf<E> {
    151	    #[cfg(test)]
    152	    {
    153	        receiver
    154	    }
    155	    #[cfg(not(test))]
    156	    {
    157	        ReceiverStream::new(receiver)
    158	    }
    159	}

Resolution: In `channel.rs`, export one stream-typed receiver under both cfgs (`pub type ReceiverStream<T> = tokio_stream::wrappers::ReceiverStream<T>;` / `= instrumented::Receiver<T>;`) and a `stream_channel(role, capacity) -> (Sender<T>, ReceiverStream<T>)`. Then `erased.rs` drops `ReceiverStreamOf`, `receiver_stream`, and its `tokio_stream` import, and `common.rs` reduces `OkReceiverStream` to one alias over `channel::ReceiverStream<T>` and `ok_channel_with` to `(tx, rx.map(Ok))`. Acceptance: no `cfg(test)` on a channel type outside `channel.rs`; `tokio_stream::wrappers::ReceiverStream` is named only in `channel.rs` and `adapter/decode.rs`.

### mirror-common-26: "The pinned pairwise lemmas" cites `before`'s subadditivity proptests without naming them
- Where: src/tree/mirror/streaming/message.rs:80-82 (related: src/tree/mirror/streaming/window.rs:328-329, src/tree/mirror/streaming/window.rs:352, tests/window_census.rs:146, crates/before/src/version/tests.rs:121, crates/before/src/version/tests.rs:148, crates/before/src/version/tests.rs:491, crates/before/src/version/tests.rs:515)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep: the artifacts exist as `join_encoding_is_subadditive`, `meet_encoding_is_subadditive`, and their `_arbitrary` twins in crates/before/src/version/tests.rs, plus `check_join_meet_subadditive` in meter/tier2/tests.rs; none of the four citing sites names them)
- Seen by: prose (as "an artifact nothing in the tree names"); refutation: reframed (the premise was wrong: the lemmas exist; only the citation is unanchored); history: deliberate and holds (206c288ae coined the phrase for exactly these tests)
- Owner-gated: no

`Greeting::max_version_bytes` rests its bound on a join or meet encoding never exceeding its inputs' combined encodings, "the pinned pairwise lemmas". The pins exist, in another crate, under names a reader could open; the phrase gives none of them. Cite artifacts by stable name.

Evidence:

    80	    /// materializes (covered by that side's aggregate alone) or a
    81	    /// join/meet of the two sides' contributions, whose encoding never
    82	    /// exceeds its inputs' combined (the pinned pairwise lemmas), so the

Resolution: "(`before`'s `join_encoding_is_subadditive` and `meet_encoding_is_subadditive` proptests)" here and at window.rs:328-329, :352, and tests/window_census.rs:146. Acceptance: each site names a test a reader can open.

### mirror-common-27: `Greeting::payload_depth_limit` is documented "in scopes"; the type's unit is decode recursion steps, and "scope" is a different crate term
- Where: src/tree/mirror/streaming/message.rs:104-106 (related: src/message.rs:61-62, src/message.rs:81-83, src/tree/mirror/streaming/stats.rs:45-47)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read: src/message.rs:61-62 "counted in the CBOR decode engine's recursion steps", :81-83 `get` "in decode recursion steps"; stats.rs:45-47 defines scope publicly as one prefix of the tree)
- Seen by: prose; refutation: confirmed; history: deliberate but expired (71de90c1 documented both sites "in scopes"; 6e4b6eea re-denominated src/message.rs to recursion steps and did not touch this file)
- Owner-gated: no

The wire field's stated unit disagrees with the unit of the value it carries, and the word it uses means the subtree one question names everywhere else in the mirror. One term, one referent; a unit stated at the wire field must match the type's.

Evidence:

    104	    /// The sender's configured payload nesting-depth limit
    105	    /// ([`Peer::payload_depth_limit`](crate::Peer::payload_depth_limit)),
    106	    /// in scopes.

Resolution: "in decode recursion steps". Acceptance: the field doc's unit matches `PayloadDepthLimit::get`'s.

### mirror-common-28: `Reply`'s field of `Reaction`s is named `replies`
- Where: src/tree/mirror/streaming/message.rs:158-162 (related: src/tree/mirror/streaming/erased.rs:63-65)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep `\.replies\b` in src: 33 sites; `tree` is a private module, so the rename is crate-internal)
- Seen by: structure; refutation: confirmed; history: no rationale found (7126e489e introduced `Reply` with `Reaction` as the vocabulary and named the field `replies` without comment)
- Owner-gated: no

A `Reply` whose parts are `replies`, each a `Reaction`, misreads at every one of its 33 use sites; the field doc already says "The reactions to a single previous query." Names match the type they hold, especially in the module that defines the protocol's vocabulary.

Evidence:

    158	/// The sole stream message.
    159	pub struct Reply<B: Backend<Node<Z>: Leaf>, H: Height> {
    160	    /// The reactions to a single previous query.
    161	    pub replies: Vec<Reaction<B, H>>,
    162	}

Resolution: Rename the field to `reactions` in `message::Reply` and `erased::Reply` (mechanical). Acceptance: `grep -rn '\.replies\b' src` is empty.

### mirror-common-29: `protocol.rs` re-states the module-wide `type_complexity` allow it already inherits
- Where: src/tree/mirror/streaming/protocol.rs:10-10 (related: src/tree/mirror/streaming.rs:42-43, src/tree/mirror/streaming/materialized.rs:388, src/tree/mirror/streaming/materialized/work/levels.rs:67, src/tree/mirror/streaming/materialized/work/answer.rs:31, src/tree/mirror/streaming/materialized/work/resolver.rs:61, src/tree/mirror/streaming/remote/adapter/encode.rs:57)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (grep: streaming.rs:43 carries `#![allow(clippy::type_complexity)]` at the `streaming` module level and protocol.rs:10 repeats it; the inheritance of lint levels into child modules is assessed from the language's lint scoping, not compiled)
- Seen by: structure; refutation: confirmed, generalized (every item-level `#[allow(clippy::type_complexity)]` under `streaming/**` is a no-op for the same reason: fifteen sites, all outside this partition); history: deliberate but expired (protocol.rs's allow came with bdfdf2529; da4234baf later allowed the lint module-wide "instead of appeased alias by alias", making the older one redundant)
- Owner-gated: no

Lint levels are lexically scoped over the module tree, so the identical inner attribute in the child module does nothing. Vestigial attributes invite the reader to look for a reason that does not exist. The same redundancy holds for the item-level allows under `streaming/**`; those document where the complexity actually is, so the owner's choice is which layer to keep.

Evidence:

    10	#![allow(clippy::type_complexity)]

    streaming.rs:
    42	// Where we're going, we need to write some Complex Types.
    43	#![allow(clippy::type_complexity)]

Resolution: Delete protocol.rs:10. For the item-level allows in sibling partitions, pick one layer: keep the module-wide allow and delete the item-level ones, or delete the module-wide allow and keep the item-level ones as the record of where the complexity lives. Acceptance: `just clippy` clean with one layer of allows.

### mirror-common-30: The phase schedule's spine is half undocumented
- Where: src/tree/mirror/streaming/protocol.rs:24-31 (related: src/tree/mirror/streaming/protocol.rs:56, src/tree/mirror/streaming/protocol.rs:65, src/tree/mirror/streaming/protocol.rs:77, src/tree/mirror/streaming/protocol.rs:123, src/tree/mirror/streaming/protocol.rs:163, src/tree/mirror/streaming/protocol/peer.rs:60, src/tree/mirror/streaming/protocol/peer.rs:78, src/tree/mirror/streaming/protocol/peer.rs:92, src/tree/mirror/streaming.rs:80, src/tree/mirror/streaming.rs:101, src/tree/mirror/streaming/message.rs:63)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read every cited line; `grep -rn missing_docs src Cargo.toml .cargo justfile` is empty, and tools/testdoc checks test functions only)
- Seen by: prose; refutation: confirmed; history: no rationale found (`pub trait Protocol` was undocumented at birth, b6625f5fa, and stayed so)
- Owner-gated: no

The traits a maintainer must read to follow the type-level schedule carry no doc comment: `Protocol`, `Connect`, `Accept`, `CompleteConnect`, `Responder`, `Reply`, and the macro-generated `Peer`/`Server`/`Client`; `Handshaken` and its `peer()` likewise; and `Greeting::version` is the one undocumented field of an otherwise fully documented struct, the field whose equality ends a session before the descent. Private docs serve the maintainer: a phase trait's contract (which height it sits at, what it consumes, what it yields, which phase follows) is what the reader cannot recover from bounds alone.

Evidence:

    24	pub trait Protocol: Send {
    25	    type Height: Height;
    26	    // `Send + 'static` because these traits' outgoing streams carry it, both
    27	    // bare and inside an `OutputError`, and the driver moves it into its
    28	    // error slot.
    29	    type Error: Send + 'static;
    30	    type Output: Send;
    31	}

Resolution: One sentence apiece stating height, input, output, and successor, e.g. `Connect`: "The client's opening phase at the root: produce our greeting and the state that awaits the peer's."; `Reply`: "One descent step at `Self::Height`: consume the peer's replies at this height and yield ours one height down."; `Peer`/`Client`/`Server`: "The full chain from a connected state to a terminal, spelled out so the compiler holds `mirror_connected`'s schedule to it."; `Handshaken`: "Both participants past the greeting exchange, plus the election keys."; `Greeting::version`: "The sender's set version; equal versions end the session at the greeting." Acceptance: every `pub` or `pub(crate)` trait, struct, and field in protocol.rs, protocol/peer.rs, streaming.rs, and message.rs has a doc comment whose first sentence stands alone (a review check: `missing_docs` ignores private items).

### mirror-common-31: The typestate trait is named `Protocol`, colliding with the public `Protocol` enum one directory up
- Where: src/tree/mirror/streaming/protocol.rs:24-24 (related: src/tree/mirror/streaming.rs:75, src/protocol.rs:15-19, src/tree/mirror/handshake.rs:26)
- Class / severity / confidence: idiom / nit / medium
- Provenance: verified (read: streaming.rs:75 glob-imports the trait via `use protocol::*;`, handshake.rs:26 imports the `crate::Protocol` enum; no file names both, so there is no compile-time collision)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (the trait came first, b6625f5fa; the enum ten days later with src/protocol.rs; the V1-retirement note rules the enum stays public, so a rename targets the trait)
- Owner-gated: no

`streaming::protocol::Protocol` (a phase's typestate: `Height`, `Error`, `Output`) shares its name with `crate::Protocol` (the wire dialect the preamble and public errors carry), so a reader of the mirror module sees `Protocol` meaning two things one directory apart. The trait names a phase, not a dialect.

Evidence:

    24	pub trait Protocol: Send {

Resolution: Rename the trait (`Phase` reads naturally against `Reply`, `CompleteEqual`, and the rest) and give it a one-line doc (mirror-common-30). Acceptance: `grep -rn 'trait Protocol' src` is empty; the enum is the only `Protocol`.

### mirror-common-32: The "rustc explodes" boxing rationale is pasted three times instead of stated once at `BoxResponses`
- Where: src/tree/mirror/streaming/protocol.rs:116-118 (related: src/tree/mirror/streaming/protocol.rs:53-54, src/tree/mirror/streaming/protocol.rs:135-137, src/tree/mirror/streaming/protocol.rs:174-176)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (grep: three copies; `BoxResponses` at :53-54 is documented only as "A boxed [`Responses`] stream.")
- Seen by: structure; refutation: confirmed; history: no rationale found for the triplication (the boxing itself is recorded in b6625f5fa)
- Owner-gated: no

The same two-line comment sits above three `BoxResponses<..>` return positions while the alias it constrains says nothing about why it exists. State a constraint once, where the thing it constrains is defined.

Evidence:

    116	        // IMPORTANT: This must be boxed because otherwise `rustc` explodes on
    117	        // an exponentially-sized type!
    118	        BoxResponses<B, UnderRoot, Self::Error>,

Resolution: Move the rationale into `BoxResponses`'s doc ("Boxed because an unboxed `impl Responses` here nests through every phase and rustc's type grows exponentially") and delete the three copies. Acceptance: `grep -c 'IMPORTANT: This must be boxed' src/tree/mirror/streaming/protocol.rs` is 0.

### mirror-common-33: The `define_peer!` comment locates `mirror_connected` in the wrong file and states as a manual obligation what the compiler enforces
- Where: src/tree/mirror/streaming/protocol/peer.rs:108-111 (related: src/tree/mirror/streaming/driver.rs:152-172, src/tree/mirror/streaming.rs:74, src/tree/mirror/streaming/protocol/peer.rs:60-67)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: `pub(super) async fn mirror_connected` is defined at driver.rs:152 only; streaming.rs:74 imports it; the compile-time coupling is assessed by reading the nested `Next` bounds against the driver's 15 loop `reply`s plus one)
- Seen by: structure, prose, correctness, perfapi; refutation: reframed (the round arithmetic is consistent as written: 15 `_` yield 16 nested `Reply` bounds matching the driver's fifteen loop rounds plus the final `i.reply`; only the location and the manual-coupling framing are wrong); history: deliberate but expired (8037ad045 wrote the comment while the function lived at streaming.rs:83; cbfe1aff3 moved it to driver.rs and left the comment)
- Owner-gated: no

The function lives in driver.rs, and a driver whose round count disagrees with the chains fails to type-check (after the wrong number of `reply` calls the state is still a `Reply` phase, which does not implement `CompleteInitiator`, or a `Reply` bound is missing for the extra call), so the coupling is compiler-held, not hand-held. A comment naming the wrong file sends the maintainer to the wrong place, and it undersells the guarantee the types give.

Evidence:

    108	// One `_` per exchange round: the initiator descends heights 31 → 1 in
    109	// fifteen rounds of two heights each, the responder 30 → 2 in fourteen.
    110	// `mirror_connected` in streaming.rs drives this same schedule; the counts
    111	// must move together.

Resolution: "`driver::mirror_connected` drives this same schedule; a count that disagrees with its loop fails to compile against the chains' terminal bounds, which is what holds the two in step (`seq!` and macro repetition cannot share a named constant)." The larger redesign (height-indexed chain traits deriving the depth from `Root`, dissolving the `_` roster) is a design proposal nobody has compiled; see the open questions. Acceptance: the comment names driver.rs and states the compile-time coupling.

### mirror-common-34: "Seam" is an undefined house metaphor used for three different boundaries, including in public `SessionStats` docs
- Where: src/tree/mirror/streaming/stats.rs:29-29 (related: src/tree/mirror/streaming/stats.rs:6, src/tree/mirror/streaming/stats.rs:104, src/tree/mirror/streaming/stats.rs:122, src/tree/mirror/streaming/erased.rs:1, src/tree/mirror/streaming/erased.rs:12, src/tree/mirror/streaming.rs:19, src/tree/mirror/framing.rs:42, src/tree/mirror/framing/tests.rs:18)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: nine sites in the partition, 48 crate-wide, none defining the word)
- Seen by: prose; refutation: reframed (one metaphor, a junction between layers, applied to three different junctions; the doctrinal point stands and three of the sites are public rustdoc); history: no rationale found (house dialect from 487e17ea3 and d1b7f00e)
- Owner-gated: no

In stats.rs the word means the code location where a count is taken; in erased.rs and streaming.rs the typed/erased boundary; in framing.rs a chunk boundary in a byte stream. A library user reading `SessionStats` meets "the seam named in its field docs" with no anchor. Metaphors only where they can be rewritten as mechanism without loss; each site here can be.

Evidence:

    29	/// Every count is taken locally, at the seam named in its field docs, while

Resolution: stats.rs: "at the point named in its field docs" / "taken exactly at that boundary, between the codec and the transport stream" / "Counted at the same codec boundary as"; erased.rs and streaming.rs: "the height-erased boundary"; framing.rs:42 "at the chunk boundaries"; framing/tests.rs:18 "every partial-read boundary". Acceptance: `grep -rn -i seam` over the nine files is empty.

### mirror-common-35: Public `SessionStats` doc announces "Two deliberate boundaries" and lists one
- Where: src/tree/mirror/streaming/stats.rs:33-38 (related: src/tree/mirror/streaming/stats.rs:143-145)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (git: `git show 487e17ea3:src/tree/mirror/streaming/stats.rs` shows the second bullet, "[`Protocol::V1`](crate::Protocol::V1) sessions report zero in every field."; 368da2a5 removes exactly those lines and leaves the heading)
- Seen by: structure, prose, correctness, perfapi; refutation: confirmed; history: deliberate but expired (the count was correct when written; the V1 retirement deleted the bullet and not the count)
- Owner-gated: no

The public rustdoc on `SessionStats` (re-exported at the crate root) opens a two-item list and delivers one bullet. Prose speaks in the present tense and carries no hand-maintained counts; this one rotted under the code and lands on the library user.

Evidence:

    33	/// Two deliberate boundaries:
    34	///
    35	/// - **No duration field.** The caller owns the clock: wrap the `gossip`
    36	///   call (or the `gossip_when` stream's polls) in whatever timing
    37	///   instrument the application already uses. A duration measured inside
    38	///   the crate would bake in one notion of time and satisfy nobody's.

Resolution: Fold the bullet into a sentence ("There is deliberately no duration field: the caller owns the clock, ..."), dropping the enumerated form; or, if a second boundary is real today (`window_granted` at :143-145 already says it "is a summary, not the whole vector"), state it as the second bullet. Do not restore the V1 bullet. Acceptance: the heading's count matches the list beneath it, or there is no count.

### mirror-common-36: Dialect tells: moralized and loose words (genuine, sound, honestly, real) and first-person narration in `Reaction`'s docs
- Where: src/tree/mirror/streaming/stats.rs:42-43 (related: src/tree/mirror/streaming/message.rs:59-60, src/tree/mirror/streaming/message.rs:171-189, src/tree/mirror/party/tests.rs:186, src/tree/mirror/party/tests.rs:247, src/tree/mirror/framing/tests.rs:9-10)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read every cited line)
- Seen by: prose; refutation: confirmed; history: no rationale found
- Owner-gated: no

Public: `disputed_scopes` "resolved as genuine disputes", where the next clause already defines a dispute and "genuine" adds judgment, not meaning. Maintainer: "there is nothing sound to gate the bytes on" (message.rs:59-60, "sound" for "available"), "honestly sized frame" (party/tests.rs:186, :247), "real content" (framing/tests.rs:10), and `Reaction`'s variant docs narrate in "we provide it", "we indicate such", "we recur" while `Greeting` speaks of the sender and the counterparty. Plain English, established terms only, one voice per module.

Evidence:

    42	    /// Scopes this side resolved as genuine disputes: questions it answered
    43	    /// where both replicas held the subtree and their contents differed.

    message.rs:
    59	    /// session. Divergence is not knowable at greeting time, so there is
    60	    /// nothing sound to gate the bytes on.
    ...
    171	    /// Having inferred that the counterparty lacks this node through its
    172	    /// absence in the counterparty's listing of hashes, we provide it, at
    173	    /// this radix.

Resolution: "resolved as disputes:"; "nothing to gate the bytes on"; "a frame whose header matches its body"; "content rather than zero fill"; recast `Reaction`'s docs in the third person ("the sender supplies it at this radix"; "both sides hold the node with equal hashes"; "the sender recurs"). Acceptance: the cited words are gone from the cited lines; `Reaction`'s variant docs use `Greeting`'s voice.

### mirror-common-37: `tasks::complete` is documented as a race; its body is a fail-fast join
- Where: src/tree/mirror/streaming/tasks.rs:19-37
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read: after either `select!` branch resolves `Ok`, the body awaits the other to completion, `finish.await` at :29 and `tasks.await?` at :33, so `Ok` requires both and the first `Err` short-circuits, which is `future::try_join`'s contract)
- Seen by: prose; refutation: confirmed; history: no rationale found (doc and body are their birth forms from cbfe1aff3; the mismatch is original)
- Owner-gated: no

"Race" tells the caller the loser is dropped; the body never drops a branch that has not finished. A doc's first sentence stands alone in a listing and must be accurate; race and join are opposite contracts for the caller's cancellation expectations.

Evidence:

    19	/// Race registered work against its terminal operation, failing on either.
    20	pub async fn complete<O, E>(
    ...
    31	        output = &mut finish => {
    32	            let output = output?;
    33	            tasks.await?;
    34	            Ok(output)
    35	        }

Resolution: Either rewrite the summary ("Run registered work alongside its terminal operation: both must complete, and the first error cancels whatever remains.") or replace the body with `future::try_join(try_run_all(tasks), finish).await.map(|((), output)| output)`, which has the same semantics (unbiased `select!` imposes no ordering) and deletes the hand-rolled select. Acceptance: the summary names the semantics the body has; if the body changes, the streaming suites stay green.

## Positives

- `cbor.rs`'s module doc argues the rejected alternative as mechanism (a general CBOR reader has no head-level incremental API and buffers past item boundaries), scopes the module to the head grammar alone, and names the round-trip tests as what holds writers and readers together. The tests do that: an inverse-plus-width-plus-untouched-trailing round trip, every wider spelling of every value rejected, a cut at every prefix leaving the input unconsumed, the `[0xd9, 0xd9, 0xf7]` literal pinned to `write_tag`, and an async-versus-slice differential. `HeadBytes` renders heads on the stack, and `tests/encode_alloc.rs` prices that at zero allocations.
- `framing.rs` states its memory policy as a contract (grow only as bytes arrive, doubling from one granule, clamped to `len`, exact consumption), turns it into a session-boundary argument (exact reads make a session boundary a stream position), and proves the growth policy differentially against a whole-read reference across arbitrary read schedules and cut points, with `chunk_boundary_cuts` shared with the codec suite so the boundary roster cannot drift. `tests/decode_alloc.rs` prices the policy with an allocator meter.
- The handshake tests are exemplary: `intent_byte_space_is_exhaustive` sweeps all 256 bytes and classifies each; `every_truncation_boundary_is_typed` cuts at every prefix and checks the reported counts; `arbitrary_preamble_decodes_by_the_oracle` is a field-by-field oracle proptest; `prefix_matches_the_writers` pins the flat `V2_PREFIX` literal to the head writers; `fragmented_exchange_is_symmetric` drives the exchange over a one-byte duplex. `Preamble::decode` diagnoses in a useful order, so a dialect skew reports as `VersionMismatch` carrying the remote's number, never as garbled fields.
- `party.rs` draws the `Error::HandOffTruncated` (the stream stopped) versus `HandOffDefect::Undecodable(Decode::Truncated)` (the body arrived whole, its content is short) distinction carefully in code, docs, and tests; `bytes_after_the_frame_stay_untouched` proves the exact-read contract the epilogue depends on; the body proptest checks that every accepted body re-encodes byte for byte (a canonicality oracle, not a no-panic check); and `decode_party` keeps the `Decode::Io` arm total, with the reason stated.
- `mirror.rs` names `contained` for exactly the partial-order pitfall (`!(a <= b)` is not "strictly above") and its test covers all three regimes, incomparable included.
- `driver.rs`: the manual `Clone` for `ErrorRoute<E, S>` is the right idiom (a derive would demand `E: Clone, S: Clone` the fields do not need); `race_session`'s biased select plus `try_recv` fallback preserves causal priority, and both behaviors have committed tests; `divert` parks after reporting so a producer failure can never masquerade as a completed phase.
- `erased.rs` derives the type-level height from the prefix's byte length so the coordinate and its witness cannot drift apart, keeps `at_height!`'s out-of-range arm unreachable by type (`ErasedPrefix` is an `ArrayVec<[u8; 32]>`), and its module doc says plainly what the types stop proving inside the walk and which suites catch it instead.
- `message.rs`: every `Greeting` field doc says why the field rides the greeting, the listing's cost is stated with its trade, and `initiates` carries a `# Panics` section stating the precondition that `descend`'s equality guard (streaming.rs:197) discharges. `protocol.rs`'s `Initiator` doc explains why the protocol never sends a root hash, and `Responder::Next`'s comment says why its bound is left loose.
- `stats.rs`'s public field docs follow one shape (the mechanism, where the count is taken, when it is zero, and for `disputed_scopes` why the two ends disagree); the `Relaxed` ordering is justified by the single post-completion read; and `tests/session_stats.rs` pins `bytes_sent`/`bytes_received` against an independent transport-level tally, so a misplaced counter would disagree with an oracle.
- `tasks.rs` is a model small module: five items, each used across the walk and the proxy, each with a one-line doc stating its invariant (`cancelled` parks so an error cannot be followed by a successful EOF).
- The two "Defensive-variant exemption" comments (handshake/tests.rs:283-290, party/tests.rs:51-57) say plainly what is untested and why; whatever mirror-common-10 decides, that habit is the right one.

## Open questions for Finch

- mirror-common-10 reopens your R2 ruling (keep-and-document on `NetworkTruncated` and `TrailingBytes`). The new fact since R2: `Staged::buf` is now `[u8; V2_PREAMBLE_LEN]`, so the typed `decode` signature that makes both arms structurally nonexistent is free, and the mutants policy you wrote orders that disposition. Recommendation: take it; the two variants are public surface no peer can produce, and their only test artifact is a comment explaining why they cannot be tested. If you keep them, the `encode -> [u8; V2_PREAMBLE_LEN]` half and the constant-name respelling (mirror-common-2) are still worth doing.
- `define_peer!` (peer.rs:12-106) exists to spell a 16-deep and a 15-deep nested bound whose depth is a function of `Root`'s height. The structure lens sketched height-indexed chain traits in the style `ReplyHeight` already uses (`trait InitiatorChain<B, H: Height>` with a base impl at `S<Z>` ending in `CompleteInitiator` and a step impl at `S<S<H>>`, dually for the responder), which would derive the chain length from the height types and delete the `_` roster. Nobody compiled it; the risk to measure is rustc's handling of a 16-deep associated-type recursion through impl selection versus the macro's fully expanded form. Recommendation: worth one afternoon as an experiment on a branch, abandon on compile-time or diagnostic-quality evidence, never on anticipated complexity.
- `race_session`'s `Ok` arm (driver.rs:79) returns the output without consulting `first_error`, while the `Err` arm prefers a same-poll routed error. The correctness lens could not construct an (`Ok`, routed) case, and the refutation pass agrees it is unreachable today because `divert` parks after reporting, so a routed error never lets its consumer see EOF. Nothing states that invariant at the arm. Recommendation: a one-line comment at the arm; no test, since the state is unreachable.
- `HandOffDefect::UnaddressableLength` (party.rs:32-37) is reachable only on 32-bit targets and carries a no-test exemption (party/tests.rs:51-57). `wasm32-unknown-unknown` is in `rust-toolchain.toml`. Is a 32-bit run of the party ingress suite planned, or is the exemption the intended permanent state? Recommendation: leave it as is unless the wasm target grows a test leg; the exemption says what it needs to.
- `SessionStats` counts bytes but not frames (stats.rs:99-132). An operator tuning `target_message_size` wants mean frame size against the target, which bytes alone cannot give; a `frames_sent`/`frames_received` pair would be counted at the codec's frame boundary (`CountedWrite`/`CountedRead` see writes, not frames, so it needs a hook one layer up) and is non-breaking under `#[non_exhaustive]`. Nothing in the current docs is wrong. Recommendation: only if you have the tuning question yourself; otherwise no.
- A gitignored build directory sits inside the source tree: `src/tree/mirror/streaming/target/doctest-nightly/` (dated Aug 17). The `doctest` recipe uses a relative `--target-dir target/doctest-nightly`, so something ran with that cwd. Harmless to the gate; worth deleting and noting which invocation produced it.

## Dropped

- "The `mirror!` schedule macro is V1-era generality with one remaining caller; inline it" (structure [2]): premise refuted by git (`alternating.rs` never used `mirror!`; the three call sites at aa22c2a2b were segments of one schedule), and the proposed straight-line `seq!` form is the driver's pre-aa22c2a2b shape, which the owner replaced deliberately for one-line-per-phase legibility; no new evidence. Converted to mirror-common-21 (state the hygiene rationale at the site).
- "One `_` per exchange round does not literally hold" (prose [24], sub-claim): refuted; with a round as one two-height descent, 15 `_` are the 15 descents between the initiator's 16 `Reply` nodes and 14 the responder's, matching driver.rs:164-168. The location error survives in mirror-common-33.
- "`Staged::fill` has neither a section nor a statement of cancel safety" (prose [27], second half): refuted; handshake.rs:244 and :264 both state it. The section form and return-arm inventory survive in mirror-common-4.
- "The pinned pairwise lemmas cite an artifact nothing in the tree names" (prose [29]): premise refuted; the lemmas are `before`'s `join_encoding_is_subadditive`/`meet_encoding_is_subadditive` proptests. Reframed to mirror-common-26 (cite by name), severity nit.
- "The cbor proptests' testdocs are inaccurate for excluding major 7" (prose [35], first half): refuted; no writer emits major 7, so "Every head a writer emits" is accurate. The unstated exclusion and the runtime builders survive in mirror-common-5.
- "'Tag' for an ITC bit in party/tests.rs:172-173" (prose [37], third clause): dropped; `before` itself calls the party codec's presence bits tags (crates/before/src/party.rs:122).
- "driver.rs and tasks.rs predate the edition bump" (perfapi [54], inference): wrong; the crate has been edition 2024 since 6b70ee9a3. The redundant import itself survives as mirror-common-18.
- "SessionStats counts bytes but not frames" (perfapi [59]): not a defect against the code; converted to an open question.
- "`payload.capacity() * 2` can wrap on 32-bit targets" (prose lens open question): dropped; a `Vec`'s capacity is at most `isize::MAX`, so `2 * capacity <= usize::MAX - 1` on every target and the multiplication cannot overflow.
- "Add a `race_session` test for a session that completes `Ok` in the same poll a route reported" (correctness [43], suggestion): not a gap; `divert` parks after `route.report`, so the state is unreachable. Kept as an open question about a comment.
- "`mirror::Error<C, S>` derives `Clone` but `MirrorError` is not `Clone`" (perfapi open question): below the bar; an inert derive on a generic type is harmless and costs nothing.
- Duplicates merged: [19], [41], [46] into mirror-common-35; [26], [39], [45] into mirror-common-10; [47] into mirror-common-17; [58] into mirror-common-6; [25], [43], [49] into mirror-common-22; [22] into mirror-common-12; [24], [42], [50] into mirror-common-33; [44] into mirror-common-3; [51] into mirror-common-7; [53] into mirror-common-11; [56] into mirror-common-24; [20], [38], [52] into mirror-common-8; [16] and [48] into mirror-common-16; [57] kept separate from [23] as mirror-common-31 because the fixes differ (rename versus document).
- Out of partition, noted for their owners: the item-level `type_complexity` allows under `streaming/**` (mirror-common-29's related sites); "mid-handshake" at error.rs:21 and :195 (mirror-common-9); the `selected` vocabulary at error.rs:14, :76, :179, :201 and protocol.rs:1 (mirror-common-12); the literal `16`s in network.rs (mirror-common-11); the parking.rs module doc's ≈1.1 MB / ≈2.2 MB figures, which disagree with message.rs:14-17's ≈1.8 MB / ≈3.5 MB while message.rs agrees with the pin.
