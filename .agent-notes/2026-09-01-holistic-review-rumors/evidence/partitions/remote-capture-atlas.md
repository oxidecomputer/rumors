# Partition remote-capture-atlas: The remote module root, the wire capture renderer, the codec test suite and its error atlas

## Partition summary

The partition is the root of the wire-bound proxy and the test instruments that sit beside its codec. `remote.rs` (94 lines) declares the proxy's submodules, carries a long module doc restating the frame grammar, and forwards a shelf of test-gated names from `codec` to `remote`; `remote/error.rs` (19 lines) is the flat alias layer that gives the adapter and codec error types unambiguous names (`ReplyDecodeError`, `CodecDecodeError`) for `crate::error` to re-export. `codec/capture.rs` (731 lines, compiled only under `test` or `test-internals`) is the CBOR reflection renderer: it parses each observed hook item with the codec's own canonical head grammar into a `Node` tree and renders it as annotated diagnostic notation, and its module doc argues that this rendering is injective on wire bytes and therefore a byte pin without a hexdump. The three remaining files are test code: `capture/tests.rs` (354 lines) pins the renderer's commitments, `codec/tests.rs` (739 lines) holds the frame round-trip properties, the 340-placement atlas snapshot, the bounded exhaustive corpus manifest, and the transport read-plan meter, and `codec/tests/error_atlas.rs` (747 lines) witnesses every frame-stream error variant with wildcard-free `describe_*` matches as the compile-time tripwire. Total read: 2,684 lines, of which 1,840 are test code and 731 are test-gated infrastructure; production code proper is 113 lines.

The quality is high. The renderer's depth budget is one counter spanning structural descent and embedded byte-string re-parses; I checked the stated invariant (render depth never exceeds parse depth) at every `render_*` call site and it holds, and three committed tests drive it through both the control-item path and the harness-reachable frame path. The error atlas closes coverage from both ends and names its own blind spot. The read-plan meter holds a formula, the reader, and pinned reference numbers to each other. Every panic in the renderer is a capture-harness contract whose message says why it cannot be the peer's fault.

The dominant issue is one real hole in the renderer's stated contract: a map key that is a container or a protocol-named tag renders as a literal `…`, so two wire byte strings differing only inside such a key render identically, contradicting the injectivity claim the module doc rests the snapshot discipline on. The prior review assessed injectivity sound by hand and took no action; the hole is where that hand analysis stopped, and it argues for the generative injectivity test the suite lacks. The second substantive issue is a ghost in `remote.rs`'s module doc describing a query listing spelling the wire has not had since the deterministic-CBOR rewrite, which is the cost of that doc restating its children's contracts. Everything else is small: a test-only wrapper around a constant already reachable by name, one type carrying two names in one namespace, a frame-to-signal projection written three times, a test-side copy of the signal roster, several testdocs that overstate their bodies, and a handful of idiom and prose nits.

## Findings

### remote-capture-atlas-1: The root module doc restates its submodules' contracts, literal tallies included
- Where: src/tree/mirror/streaming/remote.rs:12-41 (related: src/tree/mirror/streaming/remote.rs:56-67, src/tree/mirror/streaming/remote.rs:8, src/tree/mirror/streaming/remote/codec.rs:18-56, src/tree/mirror/streaming/remote/streams.rs:3, src/tree/mirror/streaming/remote/codec/signal/tests.rs:7)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (side-by-side read of remote.rs against codec.rs:18-56 and streams.rs:1-10; `grep -rnE '\b(162|163|340)\b' src` returns remote.rs:18, codec.rs:25-26, codec/tests.rs:246, and the enforced home signal/tests.rs:7; `git show --stat 3327a92b` shows the last opener change edited both remote.rs and codec.rs)
- Seen by: structure, prose, correctness, perfapi; refutation: confirmed; history: no rationale found (the doc was deliberately reshaped in c5f1210a and maintained in lockstep, but nothing records why the root should carry the grammar)
- Owner-gated: no

After the one-paragraph role map (lines 3-6), the root doc re-describes the frame grammar with its counts (17 streams, ten states, 162/163 placements), the reply/stream end separation, the query and run spellings, scope retention, and stream binding, each of which the owning submodule states in nearly the same words. The literal tallies therefore live in two prose homes beside their enforced homes (`Stream::COUNT`, `Signal::STATE_COUNT`, `VALID_PLACEMENTS`), and the copy drifts independently: finding 2 is the drift this duplication has already cost. Principle: documentation altitude (a module root orients and points; it does not restate a child's contract) and no hand-maintained counts.

Evidence:

    12	//! [`codec`] defines the common frame grammar: a frame opens with its
    13	//! stream's index (one of 17) and its signal's state code (one of ten:
    ...
    17	//! reserved. The phase schedule narrows that product: the initiator
    18	//! admits 162 placements and the responder 163, rejecting the rest

    codec.rs:
    18	//! A frame opens with two unsigned ints: the index of the logical stream
    19	//! it rides (one of 17) and its signal's state code (one of ten frame
    ...
    25	//! select a phase-specific subset of states: the initiator admits 162
    26	//! placements and the responder 163, rejecting the rest before their

    streams.rs:
    3	//! This layer binds the protocol's 17-per-direction logical streams onto a

Resolution: Trim remote.rs to the role map (3-6), the transport sentence (8-10, citing `Stream::COUNT` by name rather than "17"), and the cross-cutting story only this level can tell (the opening question riding the greeting and the early-supply stream, 43-54), with intra-doc links to `codec`, `adapter`, and `streams` for their own contracts. Where a count survives anywhere in prose (codec.rs:25-26, streams.rs:3), name the enforced place (`VALID_PLACEMENTS`, `Stream::COUNT`) rather than the literal. Acceptance: `grep -nE '162|163|one of 17|one of ten' src/tree/mirror/streaming/remote.rs` is empty; no sentence in remote.rs's module doc duplicates one in codec.rs, adapter.rs, or streams.rs.

### remote-capture-atlas-2: The module doc describes a query-listing spelling the wire no longer has
- Where: src/tree/mirror/streaming/remote.rs:30-31 (related: src/tree/mirror/streaming/remote/codec.rs:37-41, src/tree/mirror/streaming/remote/codec/frame.rs:517-518, src/tree/mirror/streaming/remote/codec/encode.rs:112-115)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`write_listing` at frame.rs:517-518 writes `cbor::write_head(out, MAJOR_MAP, children.len() as u64)`, the count carried directly in the map head; `git blame -L30,31` dates the lines to e41e7069 and c5f1210a; `git show c5f1210a:src/tree/mirror/streaming/remote/codec/encode.rs` carries `QUERY_COUNT_BIAS` and a `count: [u8; 1]` body field at lines 17, 54, 71; `git show 4dd2053c -- remote.rs` touched only the `mod codec` visibility and one re-export line)
- Seen by: prose, correctness; refutation: confirmed; history: deliberate but expired (accurate for the byte-coded wire; orphaned by 4dd2053c, which rewrote codec.rs's doc and left this paragraph)
- Owner-gated: no

The doc of record for the proxy says a nonempty query's body is a "one-byte count-minus-one". The encoder writes a `{radix: hash}` CBOR map whose head carries the count directly (one byte below 24, two or three bytes above), and codec.rs:37-38 documents it that way. A reader implementing or debugging against this paragraph goes looking for a count byte that is not on the wire. This breaches the AGENTS.md hard rule that nothing in the codebase refers to code that no longer exists.

Evidence:

    30	//! An empty query occupies its signal alone; a nonempty query's one-byte
    31	//! count-minus-one admits every fan from 1 through 256.

    frame.rs:
    517	pub(crate) fn write_listing(out: &mut Vec<u8>, children: &[(u8, Hash)]) {
    518	    cbor::write_head(out, MAJOR_MAP, children.len() as u64);

Resolution: Delete the sentence and let codec.rs own the grammar (finding 1), or restate it: "An empty query occupies its signal alone; a nonempty query's body is a `{radix: hash}` map of one to 256 children, its map head carrying the count." While there, lines 33-41 omit the record's tag-63 wrapper and the version atom's own tag that codec.rs:44-46 states; acceptable at this altitude if deliberate, but decide. Acceptance: `grep -rn count-minus-one src` is empty and any remaining sentence matches `write_listing`.

### remote-capture-atlas-3: Five identical `cfg` attributes gate what two `use` statements would carry
- Where: src/tree/mirror/streaming/remote.rs:75-84 (related: src/tree/mirror/streaming/remote.rs:70, src/tests.rs:281, src/testing.rs:14-16)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read; `grep -rn 'remote::codec::' src tests` outside `remote/` returns only src/tests.rs:281)
- Seen by: structure, perfapi; refutation: reframed (the forwarding layer is a defensible layering choice, not circular justification; the mechanical part is the cfg merge); history: deliberate but expired (the hop was load-bearing when `codec` was private; 4dd2053c made it `pub(crate)` for src/tests.rs and left the forwarding)
- Owner-gated: no

The shelf is written as five separate `#[cfg(any(test, feature = "test-internals"))]` statements, two `pub use` and three `pub(crate) use`, where one attribute per visibility carries the same names; the accretion is visible in blame. Separately, `codec` is `pub(crate)` (line 70) and src/tests.rs:281 reaches `remote::codec::greeting::encode_greeting` directly, so the forwarding shelf is not the only door; whether it should be is a layering question recorded under open questions.

Evidence:

    75	#[cfg(any(test, feature = "test-internals"))]
    76	pub use codec::{FrameShape, PreparedFrame};
    77	#[cfg(any(test, feature = "test-internals"))]
    78	pub use codec::{HookCapture, HookStream, LinkCapture};
    79	#[cfg(any(test, feature = "test-internals"))]
    80	pub(crate) use codec::{assert_items_account_for, render_hook_capture, stream_label};
    81	#[cfg(any(test, feature = "test-internals"))]
    82	pub(crate) use codec::{decode_frame_discarded, lone_record_run, supply_frame_head};
    83	#[cfg(any(test, feature = "test-internals"))]
    84	pub(crate) use codec::{prepare_frame, write_prepared_frame};

Resolution: Collapse to one `pub use codec::{...}` and one `pub(crate) use codec::{...}` under one attribute each. Acceptance: remote.rs has two `cfg(any(test, feature = "test-internals"))` use statements.

### remote-capture-atlas-4: `codec_stream_count` wraps a constant already reachable as `remote::Stream::COUNT`
- Where: src/tree/mirror/streaming/remote.rs:87-91 (related: src/link/tests.rs:17, src/tree/mirror/streaming/remote/error.rs:15, src/tree/mirror/streaming/remote/codec/signal.rs:32)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn codec_stream_count src tests` returns the definition and one caller; `Stream` is in remote/error.rs:15's re-export list and remote.rs:85 is `pub use error::*`; signal.rs:32 is `pub const COUNT: u8 = 17;`)
- Seen by: structure, perfapi; refutation: confirmed; history: no rationale found (redundant from birth: b3b877d9 introduced the wrapper in the same commit that exported `Stream` at this level)
- Owner-gated: no

The `#[cfg(test)]` function exists for one caller to read `codec::Stream::COUNT`, but `Stream` is exported through the `error::*` glob at the same path the caller already spells, so `remote::Stream::COUNT` resolves directly. Principle: circular justification (the function's only reason to exist is to name a thing already named); it also removes one `cfg(test)` item from a production file.

Evidence:

    87	/// The codec's logical stream count, for cross-layer constant assertions.
    88	#[cfg(test)]
    89	pub(crate) fn codec_stream_count() -> u8 {
    90	    codec::Stream::COUNT
    91	}

    link/tests.rs:
    17	        usize::from(crate::tree::mirror::streaming::remote::codec_stream_count()),

Resolution: Delete lines 87-91; in src/link/tests.rs:17 write `usize::from(crate::tree::mirror::streaming::remote::Stream::COUNT)`. Acceptance: `grep -rn codec_stream_count src` is empty; `stream_count_matches_the_codec` compiles and passes.

### remote-capture-atlas-5: The proxy error carries two names in the `remote` namespace, and tests rename toward the one that already exists
- Where: src/tree/mirror/streaming/remote.rs:93 (related: src/tree/mirror/streaming/remote/error.rs:17, src/tree/mirror/streaming/remote.rs:85, src/peer/gossip.rs:1416, src/peer/gossip.rs:1423, src/tree/mirror/streaming/remote/proxy/tests.rs:32, src/tree/mirror/streaming/remote/proxy/tests/{harness,malformed,declarations,failures}.rs)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep for `Error as RemoteError` returns five proxy test files plus the alias site; grep for `streaming_remote::Error\b` returns gossip.rs:1416 and 1423; src/error.rs:48 fixes the public name as `RemoteError`)
- Seen by: structure, correctness, perfapi; refutation: reframed (the sub-claim that the glob also doubles the codec and adapter `DecodeError`s is wrong: `codec` is `pub(crate) mod` and `adapter` is private, neither glob-exported); history: no rationale found (both spellings arrived in one WIP commit; 474ddcc0 settled the public flattening on `RemoteError`)
- Owner-gated: no

Line 93 exports `proxy::Error` as `remote::Error`; remote/error.rs:17 exports the same type as `RemoteError`, which the glob at line 85 also places in `remote`. Five test modules then import `remote::{Error as RemoteError, ...}`, hand-renaming to a name the module already provides, while gossip.rs uses the bare `Error`. One type, one name per namespace; the renames are evidence the doubling makes readers re-derive a name that is already there.

Evidence:

    85	pub use error::*;
    ...
    93	pub use proxy::Error;

    error.rs:
    17	pub use super::proxy::Error as RemoteError;

Resolution: Delete `pub use proxy::Error;` at line 93 (`RemoteError` remains via the glob and is the name `crate::error` re-exports), change gossip.rs:1416 and 1423 to `streaming_remote::RemoteError`, and drop the `Error as RemoteError` renames in the five proxy test files. The `tree` module is crate-private, so this is not a public API change. Acceptance: `grep -rn 'Error as RemoteError' src` returns only remote/error.rs:17; `grep -rnE 'streaming_remote::Error\b' src` is empty.

### remote-capture-atlas-6: Three renderer doc sentences misstate the hook contract, the panic boundary, and the error type
- Where: src/tree/mirror/streaming/remote/codec/capture.rs:4-5 (related: src/tree/mirror/streaming/remote/codec/capture.rs:42-47, src/tree/mirror/streaming/remote/codec/capture.rs:244-247, src/tree/mirror/streaming/remote/codec/capture.rs:277-287, src/tree/mirror/streaming/remote/codec/capture.rs:347-349, src/observe.rs:8-10)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read capture.rs:1-47, 242-299, 343-394 and observe.rs:1-15)
- Seen by: prose; refutation: confirmed; history: no rationale found (all three sentences date from the renderer's first commit)
- Owner-gated: no

(a) "one CBOR item per line of the hook's contract" does not parse; the hook contract (observe.rs:9) is that each invocation carries exactly one whole CBOR item. (b) Lines 42-46 say an item that is not one canonical CBOR item "where the wire grammar requires one" is a panic, but a frame body the wire grammar also constrains (a listing, a run) falls back at lines 282-285 rather than panicking; `render_frame`'s own doc (244-247) states the split correctly, so the module doc overreaches. (c) Lines 347-348 say "Any violation is a typed reason" while `parse_node` returns `Result<Node, String>`. Private docs serve the maintainer and must be accurate against today's code.

Evidence:

    4	//! this module is the form that pin takes: each observed item — one
    5	//! CBOR item per line of the hook's contract — renders as a fully

    347	/// shortest-form arithmetic. Any violation is a typed reason for the
    348	/// caller's explicit fallback.
    349	fn parse_node(input: &mut &[u8], depth: usize) -> Result<Node, String> {

Resolution: (a) "each observed item (exactly one CBOR item, per the hook's contract)". (b) Restrict the panic sentence to control items, frame openers, and labels and add that frame bodies fall back, or point at `render_frame`. (c) "a stated reason", or make the reason an enum if a typed one is wanted. Acceptance: the three sentences read true against `render_item`, `render_frame`, `parse_node`, and observe.rs:9.

### remote-capture-atlas-7: "the walk" in the renderer collides with the crate's anchored term for the in-process participant
- Where: src/tree/mirror/streaming/remote/codec/capture.rs:22 (related: src/tree/mirror/streaming/remote/codec/capture.rs:334, src/tree/mirror/streaming/remote/codec/capture.rs:447, src/tree/mirror/streaming/remote/codec/capture.rs:615, src/tree/mirror/streaming/remote/codec/capture.rs:722-723, src/tree/mirror/streaming/remote/codec/capture/tests.rs:6, src/tree/mirror/streaming/remote/codec/capture/tests.rs:91, src/tree/mirror/streaming/remote/codec/capture/tests.rs:260, src/tree/mirror/streaming.rs:5)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -rn 'the walk' src/tree/mirror/streaming/remote src/tree/mirror/streaming.rs`: streaming.rs:5 defines the term; proxy/work/pump.rs:8,132, queues.rs:15, encode.rs:57, adapter/decode.rs:338, and two proxy test docs use it in that sense; the seven renderer sites above use it for the parse traversal)
- Seen by: prose; refutation: confirmed; history: no rationale found (streaming.rs anchored the term in c5f1210a; the renderer adopted the phrase four weeks later)
- Owner-gated: no

streaming.rs:5 defines *the walk* as the in-process protocol participant, and the rest of the `remote` subtree uses it in that sense. capture.rs and its tests use "the walk" for the renderer's own traversal, so a reader of the `remote` tree meets two referents for one anchored term of art. "The walk" is also on the review's list of unanchored metaphors promoted to jargon.

Evidence:

    22	//! byte streams cannot render identically. Wherever the walk cannot

    streaming.rs:
    5	//! roles recur through every layer below: *the walk* is the in-process

Resolution: Say "the renderer" or "the traversal" at the eight renderer sites. Acceptance: `grep -n 'the walk\|walk cannot\|walk never\|walk.s' capture.rs capture/tests.rs` is empty; the streaming sense remains the only one under `remote/`.

### remote-capture-atlas-8: `stream_label` returns a nested tuple, and the capture structs derive nothing
- Where: src/tree/mirror/streaming/remote/codec/capture.rs:117-122 (related: src/tree/mirror/streaming/remote/codec/capture.rs:75, src/tree/mirror/streaming/remote/codec/capture.rs:86, src/tree/mirror/streaming/remote/codec/capture.rs:98, src/testing.rs:29-33, tests/common/gossip_snapshot.rs:354, tests/observe.rs:186, tests/observe.rs:217, src/tree/mirror/streaming/remote/codec/capture/tests.rs:233)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: verified (grep of every `stream_label(` call site: four positional destructurings; capture.rs:66-109 read: no `#[derive]` on `LinkCapture`, `HookCapture`, or `HookStream`)
- Seen by: structure, perfapi; refutation: confirmed; history: no rationale found
- Owner-gated: no

`((u8, u8), usize)` is `(epoch, index)` plus the label's byte length; the re-export doc at testing.rs:29-30 has to spell out the positions, and every caller writes `let ((epoch, index), label_len) = stream_label(...)`. The three `pub` capture structs exposed through `rumors::testing` derive nothing, not even `Debug`, so an assertion cannot print them. Types-first: named fields over positional tuples, especially across the test-internals boundary.

Evidence:

    117	pub fn stream_label(bytes: &[u8]) -> ((u8, u8), usize) {
    ...
    121	    ((epoch, index), bytes.len() - rest.len())

    86	pub struct HookCapture {

Resolution: Return a `StreamLabel { epoch: u8, index: u8, len: usize }`; update testing.rs:29-33, the three integration-test call sites, and capture/tests.rs:233. Add `#[derive(Debug)]` (at least) to the three capture structs. Acceptance: no `((` destructuring of `stream_label` remains; the three structs print under `{:?}`.

### remote-capture-atlas-9: The `Role -> Speaker` inverse lives apart from `Speaker::role`
- Where: src/tree/mirror/streaming/remote/codec/capture.rs:214-220 (related: src/tree/mirror/streaming/remote/codec/signal.rs:128-134, src/tree/mirror/streaming/remote/codec/capture.rs:195)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -rn 'Role::Initiator => Speaker\|From<Role>' src` hits only capture.rs:217; signal.rs:129 defines `pub fn role(self) -> Role`)
- Seen by: structure; refutation: reframed (a private inherent method is a pure move; `impl From<Role> for Speaker` is a public trait impl on a publicly re-exported type and would be owner-gated); history: no rationale found
- Owner-gated: no (with the private-method resolution)

`Speaker::role` (signal.rs:129) maps `Speaker -> Role`; the renderer defines the inverse as a file-local free function. Two halves of one bijection in two files drift independently; beside each other they read as one statement.

Evidence:

    214	/// The elected role, in the codec's speaker vocabulary.
    215	fn speaker(role: Role) -> Speaker {
    216	    match role {
    217	        Role::Initiator => Speaker::Initiator,
    218	        Role::Responder => Speaker::Responder,
    219	    }
    220	}

Resolution: Add a `pub(super) fn from_role(role: Role) -> Speaker` beside `Speaker::role` in signal.rs, call it at capture.rs:195, delete lines 214-220. (`impl From<Role> for Speaker` would be the idiomatic public form but adds to the public surface; owner's call.) Acceptance: the two match arms of the bijection sit in one file.

### remote-capture-atlas-10: `MAX_DEPTH` states no derivation and no relation to the payload depth limit
- Where: src/tree/mirror/streaming/remote/codec/capture.rs:334-341 (related: src/message.rs:59, src/tree/mirror/streaming/remote/codec/capture.rs:280, src/tree/mirror/streaming/remote/codec/capture.rs:490, src/tree/mirror/streaming/remote/codec/capture.rs:567, src/tree/mirror/streaming/remote/codec/capture.rs:647, src/tree/mirror/streaming/remote/codec/capture.rs:685, src/tree/mirror/streaming/remote/codec/capture/tests.rs:3-5)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (traced depth through `render_frame` (parse at 0) -> `render_node` Tag arm (`depth + 1`) -> `render_tag` -> `render_embedded_as` (re-parse at 1) -> record Tag (`depth + 1`) -> `render_embedded_as` (re-parse at 2): a supply record's payload root parses at depth 2, so `depth >= MAX_DEPTH` at line 350 trips once the payload nests about 62 levels; message.rs:59 admits 256)
- Seen by: prose, correctness; refutation: confirmed; history: no rationale found (the constant arrived at 64 with no derivation; the prior review named both numbers without relating them)
- Owner-gated: no

The constant's doc explains that the budget is shared, not why 64. A payload legal under `DEFAULT_PAYLOAD_DEPTH_LIMIT` (256) but nested past about 62 levels renders as the explicit hex fallback, which keeps the pin injective but forfeits the field-localization commitment (capture/tests.rs:3-5) for that payload. That is the next question a maintainer asks, and the declaration does not answer it. Named constants over magic numbers: the name is there, the derivation is not.

Evidence:

    334	/// Nesting past this bound falls back to exact hex: the walk never
    335	/// recurses on unbounded input-controlled depth.
    ...
    341	const MAX_DEPTH: usize = 64;

    message.rs:
    59	pub const DEFAULT_PAYLOAD_DEPTH_LIMIT: PayloadDepthLimit = PayloadDepthLimit(256);

Resolution: Add one sentence at the declaration: the bound is a legibility bound chosen below `DEFAULT_PAYLOAD_DEPTH_LIMIT`; a fixture nested that deep pins as the explicit hex fallback rather than a value tree; embedded-byte-string chains (which the payload limit does not bound) are why the bound must exist at all. Deriving the bound from the payload limit instead would deepen `render_node`'s recursion to 256-plus frames, so documenting is the safer option. Acceptance: the doc states why 64 and names the relationship to the payload depth limit.

### remote-capture-atlas-11: Bare major numbers `7` and `1` beside named `MAJOR_*` constants
- Where: src/tree/mirror/streaming/remote/codec/capture.rs:356-362 (related: src/tree/mirror/streaming/remote/codec/capture.rs:392, src/tree/mirror/cbor.rs:33-48, src/tree/mirror/cbor.rs:23-27)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (cbor.rs:33-48 defines `MAJOR_UINT`, `MAJOR_BSTR`, `MAJOR_TEXT`, `MAJOR_ARRAY`, `MAJOR_MAP`, `MAJOR_TAG` only)
- Seen by: structure, correctness; refutation: confirmed (cbor.rs itself spells the mask as `& 0x1f` and the major as `>> 5`, so only the two major literals are the inconsistency); history: no rationale found (cbor.rs's module doc scopes its constants to what the production codecs read, which argues for local constants here rather than widening cbor.rs)
- Owner-gated: no

`parse_node` tests `initial >> 5 == 7` and matches `1 =>` for the negative-int major, while every other arm is a named constant; the `unreachable!` proof at line 392 reads more evidently with the arms it summarizes named.

Evidence:

    356	    if initial >> 5 == 7 {
    ...
    362	        1 => Ok(Node::Nint(head.value)),
    ...
    392	        _ => unreachable!("majors 0 through 6 handled; 7 split off above"),

Resolution: Define `const MAJOR_NINT: u8 = 1;` and `const MAJOR_SIMPLE: u8 = 7;` locally in capture.rs (the renderer is their only reader, and cbor.rs deliberately owns only the majors the production codecs read) and use them at 356 and 362. Acceptance: no bare major literal in `parse_node`.

### remote-capture-atlas-12: The `"listing"` key rule applies at every depth, so application payloads acquire listing annotations
- Where: src/tree/mirror/streaming/remote/codec/capture.rs:464-469 (related: src/tree/mirror/streaming/remote/codec/capture.rs:222-240, src/tree/mirror/streaming/remote/codec/capture.rs:272-276)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read the map arm; the rule is keyed on the text alone, and a supply record's payload renders through the same arm per the trace in finding 10)
- Seen by: structure (as an open question), refutation pass (raised as new); refutation: confirmed by the refutation pass's reading; history: not examined (the scope-A mutants note, Family 4, records the guard's mutant as annotation placement)
- Owner-gated: no

A payload map `{"listing": {1: h'...'}}` renders with `/ listing: 1 child(ren) /`, hex radix keys, and `/ digest /` comments, possibly with a `NON-CANONICAL ORDER` verdict, on application data the naming layer was not designed for. Injectivity is unaffected (annotations add rather than drop), so this is a misannotation, not a defect; but the protocol's `listing` key occurs in exactly one place (the greeting item), and the rule could be scoped to it.

Evidence:

    464	                // The one context-sensitive key: a map value under the
    465	                // text key "listing" is a `{radix => digest}` listing.
    466	                let value_naming = match key {
    467	                    Node::Text(text) if text == "listing" => Naming::Listing,
    468	                    _ => Naming::Plain,
    469	                };

Resolution: Add a `Naming::Greeting` context that `render_direction` passes for the control item `control_item_name` identifies as the greeting, and fire the `"listing"` rule only under it; or state at the comment that the rule is deliberately context-free and payload maps carrying that key are annotated as listings. Acceptance: a payload map with a `"listing"` text key renders without listing annotations, or the comment states the choice.

### remote-capture-atlas-13: Container and named-tag map keys render as a bare `…`, breaking the renderer's injectivity contract
- Where: src/tree/mirror/streaming/remote/codec/capture.rs:470 (related: src/tree/mirror/streaming/remote/codec/capture.rs:523, src/tree/mirror/streaming/remote/codec/capture.rs:20-22, src/tree/mirror/streaming/remote/codec/capture.rs:46-47, src/tree/mirror/streaming/remote/codec/capture.rs:705-716, src/tree/mirror/streaming/remote/codec/capture/tests.rs:7-8)
- Class / severity / confidence: correctness / high / high
- Provenance: assessed (traced by reading: `scalar` returns `None` for `Node::Array(_) | Node::Map(_)` at line 705 and for the six protocol-named tags at 708-716; both call sites substitute `"…"`; the frame path reaches the generic map arm for a supply record's payload per the trace in finding 10. Verified mechanically that no committed snapshot contains `…`: `grep -rl '…'` over tests/snapshots, both codec snapshot directories, and src/bookmark returns nothing, so the fix moves no pin)
- Seen by: structure, prose, correctness; refutation: confirmed (all three lenses traced independently; the refutation pass adds that a single container-keyed listing entry renders `… =>` with no annotation at all, since the order check is pairwise); history: no rationale found (the elision was written in ef6569c4 with no comment; REVIEW.md item 12 recorded "Injectivity: assessed sound, modulo B1. No action beyond B1" from hand construction, so this overturns a recorded assessment rather than reopening a ruling)
- Owner-gated: no

When a map key is an array, a map, a protocol-named tag, or a generic tag over a container, `scalar(key)` is `None` and the entry renders its key as `…`, discarding the key's content; the same elision sits in `render_listing` at line 523. Two wire byte strings that differ only inside such a key render identically, which is exactly what the module doc says cannot happen, on the input class (application payload) the doc promises "only ever fall[s] back explicitly". Application payloads are arbitrary serde CBOR, so any type with tuple or struct keys reaches this path through the public `send` API. Principle: correct for all inputs, and the cheapest artifact that passes the snapshot must be the intended bytes: a payload-key change in a fixture would pass the snapshot unseen. The renderer is test-only infrastructure, and no committed fixture carries a container key today, so the hole is latent; the contract clause and the masking potential are what set the severity.

Evidence:

    470	                let key = scalar(key).unwrap_or_else(|| "…".into());

    523	            other => scalar(other).unwrap_or_else(|| "…".into()),

    20	//! the determinism contract a complete value tree has exactly one
    21	//! encoding, so the rendering is injective on wire bytes: two different
    22	//! byte streams cannot render identically. Wherever the walk cannot

    46	//! harness, not the peer, is broken. Application payload bytes are the
    47	//! application's own CBOR and only ever fall back explicitly.

    705	        Node::Array(_) | Node::Map(_) => return None,

Resolution: Never elide. When `scalar(key)` is `None`, render the entry in block form: a `key =>` line preceded by `render_node(key, Naming::Plain, deeper, depth + 1, out)` (or a `/ key /`-annotated block), then the value block or inline value. Apply the same at line 523 (a listing key that is not a uint is already off-grammar; render it fully and let a key-shape verdict carry the diagnosis, see finding 14). `Node` does not retain its byte span, so block-form keys are the local fix; routing container-keyed maps through `fallback` with exact bytes is the alternative if span retention is added. Then tighten the module doc's fallback list to match. Acceptance: the construction below fails before the fix and passes after; `grep -n '"…"' capture.rs` is empty; the existing snapshots are byte-identical (`just test-all`).

Construction: In capture/tests.rs, using the existing `run_lines` helper, build two `LeafRun`s each holding one record: `run.push(&Version::new(), &Message::new(BTreeMap::from([((1_u8, 2_u8), 7_u8)])))` and the same with `((3_u8, 4_u8), 7_u8)`. `Message::new` serializes through `ciborium::ser::into_writer` (message.rs:285-288), and serde serializes a tuple key as a two-element CBOR array, so each payload is a map with an array key; every integer is a one-byte head, so both encodings have equal length and every rendered header byte count matches. Assert `run_lines(&a) != run_lines(&b)`. Today both render the entry as `… => 7` and the assertion fails. A second case for the named-tag branch (lines 708-716): keys `Tag(VERSION_TAG, Bytes([0xe0]))` versus `Tag(VERSION_TAG, Bytes([0x40]))` built through `cbor::write_head` and rendered via `render_item`, which also collapse to `… => ...`.

### remote-capture-atlas-14: Dead disjunct in the listing order check, and an order verdict for a key-shape violation
- Where: src/tree/mirror/streaming/remote/codec/capture.rs:502-513 (related: src/tree/mirror/streaming/remote/codec/capture.rs:520-524)
- Class / severity / confidence: simplification / nit / high
- Provenance: assessed (`[T]::windows(2)` over fewer than two entries yields nothing and `Iterator::all` over nothing is `true`, so the disjunct never changes the result)
- Seen by: structure, correctness, perfapi; refutation: confirmed; history: already known (the scope-A mutants note records this clause as "rung 1, delete the clause" and notes the neighbouring `<` at line 505 is live; not yet landed)
- Owner-gated: no

The `|| entries.len() < 2` clause is structurally redundant, and a reader stops to look for the case it covers. The `_ => false` arm also makes a non-uint key pair produce `NON-CANONICAL ORDER`, a key-shape violation labelled as an order violation; and for a single non-uint entry no verdict fires at all (the pairwise check sees no pair), which combines with finding 13 into a `… =>` line with no annotation.

Evidence:

    502	    let ascending = entries
    503	        .windows(2)
    504	        .all(|pair| match (&pair[0].0, &pair[1].0) {
    505	            (Node::Uint(a), Node::Uint(b)) => a < b,
    506	            _ => false,
    507	        })
    508	        || entries.len() < 2;

Resolution: Delete the disjunct. Split the verdict: `NON-CANONICAL KEY` when any key is not a uint (checked per entry, so a lone entry is covered), `NON-CANONICAL ORDER` when uint keys are not strictly ascending. Acceptance: behavior-preserving on the committed fixtures (`descending_listing_renders_an_order_verdict` and `listing_renders_children_with_digest_annotations` pass; wire snapshots unchanged); a single non-uint-keyed listing entry renders a verdict.

### remote-capture-atlas-15: `render_tag`'s scalar guard computes `scalar()` twice and re-proves itself with an `expect`
- Where: src/tree/mirror/streaming/remote/codec/capture.rs:600-609
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read)
- Seen by: structure, perfapi; refutation: confirmed; history: already known (the scope-A mutants note records the `589 -> true` mutant panicking on exactly this `expect`; the note's own disposition ladder prefers refactor, which it did not take)
- Owner-gated: no

The penultimate arm guards on `scalar(content).is_some()`, then calls `scalar(content)` again and unwraps with `expect("checked by the guard")`; the final `_` arm is the block form. A single arm matching on the `Option` says what guard-plus-expect says, without a panic site whose only proof is the line above it, and dissolves the mutant leg outright.

Evidence:

    600	        (_, scalar_content) if scalar(scalar_content).is_some() => {
    601	            let text = scalar(scalar_content).expect("checked by the guard");
    602	            writeln!(out, "{indent}{number}({text})").unwrap();
    603	        }
    604	        _ => {

Resolution: Merge the two arms into one `_ => match scalar(content) { Some(text) => inline, None => block }`. Acceptance: `capture.rs` contains no `expect("checked by the guard")`; the wire snapshots are byte-identical.

### remote-capture-atlas-16: `render_embedded` is a one-production-caller wrapper over `render_embedded_as`
- Where: src/tree/mirror/streaming/remote/codec/capture.rs:616-625 (related: src/tree/mirror/streaming/remote/codec/capture.rs:570, src/tree/mirror/streaming/remote/codec/capture.rs:627-628, src/tree/mirror/streaming/remote/codec/capture/tests.rs:25, src/tree/mirror/streaming/remote/codec/capture/tests.rs:99, src/tree/mirror/streaming/remote/codec/capture/tests.rs:125)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (`grep -rn 'render_embedded(' src`: one production caller at line 570, three in capture/tests.rs)
- Seen by: structure; refutation: confirmed; history: no rationale found (both functions were written together in ef6569c4)
- Owner-gated: no

The wrapper fixes `inner = Naming::Plain` and forwards; the `_as` suffix and the split doc comment ("[`render_embedded`], with the naming context") are the tell that one function with a parameter suffices.

Evidence:

    616	fn render_embedded(
    ...
    624	    render_embedded_as(number, name, Naming::Plain, bytes, indent, depth, out);
    625	}

    570	            render_embedded(number, "embedded item", bytes, indent, depth, out);

Resolution: Rename `render_embedded_as` to `render_embedded`, pass `Naming::Plain` at line 570 and the three test call sites, delete the wrapper, merge the two doc comments. Acceptance: one `fn render_embedded` in capture.rs.

### remote-capture-atlas-17: The renderer's injectivity claim has no committed test; every fallback is pinned by a point example
- Where: src/tree/mirror/streaming/remote/codec/capture/tests.rs:3-10 (related: src/tree/mirror/streaming/remote/codec/capture.rs:12-30, tests/wire_legibility.rs:1-17)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (capture/tests.rs read in full: no `proptest!` block; tests/wire_legibility.rs is a proptest over `ciborium::Value` parseability, not rendering distinctness)
- Seen by: correctness; refutation: confirmed; history: already known and reopened (REVIEW.md item 12 recorded "No action beyond B1", a decision not to add a test; the scope-A mutants note independently records that "the renderer is total over CBOR, while everything that exercises it speaks only the protocol's dialect" and proposes generative families, still open; finding 13 is the new evidence that reopens item 12)
- Owner-gated: yes (reopens a recorded ruling; the choice of form, inverse parser or leaf mutation, decides how much of the rendering grammar becomes load-bearing)

The suite's three commitments are localization, explicit fallback for named bad inputs, and totality. The property the module doc rests the snapshot discipline on, that distinct wire bytes render distinctly, is sampled nowhere: each fallback test hand-builds one bad input and checks for the `!!` line. A generative test over canonical CBOR items would have found the `…` elision (finding 13) and will find the next hole (a scalar spelling that collides, a tag arm that drops content). Principle: every criterion needs a committed demonstration that a known-bad mechanism fails it; a property stated in prose and never sampled is decoration, and "injective on all canonical CBOR" is a family, so it belongs in a proptest.

Evidence:

    3	//! Three commitments: the rendered value tree localizes the semantic
    4	//! field a snapshot re-accept moved to exactly one rendered line
    5	//! carrying the exact value (its surrounding vocabulary is the wire
    6	//! snapshots' to pin), bytes the walk cannot vouch for render as
    7	//! an explicit failure above their exact hex (never as a silently pretty
    8	//! tree, never as silent omission), and the totality witness

Resolution: Add a proptest with a `Node`-shaped strategy (bounded depth and size; uints, nints, bytes, text, arrays, maps with arbitrary keys, tags including the protocol's named tags and 24/63 over byte strings), encoded through `cbor::write_head`. Either (a) write a small inverse parser from the rendering's grammar back to bytes and assert round-trip, which pins injectivity outright, or (b) the cheaper mutation form: generate an item, mutate exactly one scalar leaf anywhere (map keys included), and `prop_assert_ne!` the two renderings through `render_item`; cover `Naming::Listing` and the `"listing"` key context so `render_listing` is exercised. Acceptance: the proptest is committed with a doc comment stating the injectivity invariant; it fails on the current tree and passes once finding 13 is fixed; any shrunk seed rides along in `proptest-regressions`.

Construction: `fn arb_node(depth: u32) -> impl Strategy<Value = Node>` via `prop_recursive`, with a test-local `fn encode(node: &Node, out: &mut Vec<u8>)` mirroring the renderer's grammar (one `write_head` per major; tag 24/63 content as a byte string of a nested encoding). For form (b): pick a random leaf path, replace the leaf with a different scalar, render both, and assert inequality. Form (a) also catches the float-width and trailing-byte survivors the mutants note lists as injectivity-voiding.

### remote-capture-atlas-18: A doubled menace adverb and a moralizer in two testdocs
- Where: src/tree/mirror/streaming/remote/codec/capture/tests.rs:7-8 (related: src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:133-134)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep for `silently` and `genuine` over the partition)
- Seen by: prose; refutation: confirmed; history: no rationale found
- Owner-gated: no

"never as a silently pretty tree, never as silent omission" doubles a menace adverb without naming the mechanism; "genuinely absent" is a moralizer where "absent" says it. Both are on the review's default-dialect list.

Evidence:

    7	//! an explicit failure above their exact hex (never as a silently pretty
    8	//! tree, never as silent omission), and the totality witness

    error_atlas.rs:
    133	/// Every inventoried error variant has an atlas witness, and every
    134	/// exemption is genuinely absent (a witnessed exemption is stale).

Resolution: "never as a value tree presented as whole, never omitted"; drop "genuinely". Acceptance: the two adverbs are gone.

### remote-capture-atlas-19: Three testdocs claim more than their bodies check
- Where: src/tree/mirror/streaming/remote/codec/capture/tests.rs:41-42 (related: src/tree/mirror/streaming/remote/codec/capture/tests.rs:238-258, src/tree/mirror/streaming/remote/codec/capture/tests.rs:260-272, src/tree/mirror/streaming/remote/codec/capture.rs:572-587, src/tree/mirror/streaming/remote/codec/capture.rs:238)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (each body read against its doc; capture.rs:572-587 read for the rendered form)
- Seen by: prose; refutation: confirmed; history: no rationale found (the "version-addressed hash" phrase was inaccurate from birth: written ten minutes after the commit that renamed the annotation to "causal version, event tree")
- Owner-gated: no

(1) `supply_reflection_localizes_the_field_that_moved` says "the record's version-addressed hash moves on the same line"; the moved line is `VERSION_TAG(h'<version bytes>') / causal version, event tree: ... /` (capture.rs:578-586): no hash is rendered, the hex is the version's own bytes. (2) `control_items_are_named_by_shape` claims "an unknown shape is a broken capture, not a renderable one" but never exercises the panic at capture.rs:238. (3) `nesting_past_the_depth_bound_falls_back` claims the nesting "falls back explicitly" while the body asserts only that `parse_node` returns an error containing "deeper than"; the fallback rendering is asserted only by the two `deep_*` tests. Every test's doc comment states its invariant in English and must be accurate; an inaccurate testdoc is a bug in the test.

Evidence:

    41	/// Containment, not equality: the record's version-addressed hash
    42	/// moves on the same line. The annotation's surrounding vocabulary is

    238	/// Control items are named by their shape; an unknown shape is a broken
    239	/// capture, not a renderable one.

    260	/// Nesting past the walk's depth bound falls back explicitly instead of
    261	/// recursing without bound on input-controlled depth.

Resolution: (1) "the version's canonical bytes (as hex) and its event-tree rendering move on the same line". (2) Add a `#[should_panic(expected = "no known shape")]` companion feeding a bare uint item, or drop the clause. (3) Retitle to what it checks (`parse_node` rejects nesting past `MAX_DEPTH` with the depth reason), or drive it through `render_item` and `assert_depth_fallback`. Acceptance: each testdoc names only what its body asserts.

### remote-capture-atlas-20: Depth-bound tests overshoot the boundary; the exact edge is not pinned
- Where: src/tree/mirror/streaming/remote/codec/capture/tests.rs:263-268 (related: src/tree/mirror/streaming/remote/codec/capture/tests.rs:325, src/tree/mirror/streaming/remote/codec/capture/tests.rs:340, src/tree/mirror/streaming/remote/codec/capture.rs:350)
- Class / severity / confidence: test-quality / nit / high
- Provenance: assessed (the test writes `0..=MAX_DEPTH` array heads, 65 arrays; the `deep_*` tests use `10 * MAX_DEPTH`; with `depth >= MAX_DEPTH` at capture.rs:350, 63 nested arrays around a uint parse and 64 do not)
- Seen by: correctness; refutation: confirmed; history: already known (the scope-A mutants note, Family 3, proposes exactly this boundary-exact family; open)
- Owner-gated: no

No committed test pins either side of the edge, so a drift from `>=` to `>` at capture.rs:350 passes. A bound whose tests only sample far past it cannot tell the bound from the bound plus or minus one.

Evidence:

    263	fn nesting_past_the_depth_bound_falls_back() {
    264	    let mut bytes = Vec::new();
    265	    for _ in 0..=MAX_DEPTH {
    266	        cbor::write_head(&mut bytes, MAJOR_ARRAY, 1);
    267	    }

Resolution: Add the boundary pair: `MAX_DEPTH - 1` nested arrays around a uint parses; `MAX_DEPTH` nested arrays fails with the depth reason; state in the doc which count is the last accepted. Acceptance: both cases committed; changing `>=` to `>` at capture.rs:350 fails one of them.

### remote-capture-atlas-21: `FrameShape`'s doc says one- or two-element array; the decoder requires two or three
- Where: src/tree/mirror/streaming/remote/codec/error.rs:131-133 (related: src/tree/mirror/streaming/remote/codec/decode.rs:217-231, src/tree/mirror/streaming/remote/codec/decode/tests.rs:180-181)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (decode.rs:225 `if !(2..=3).contains(&head.value) {` against error.rs:131; decode/tests.rs:180-181's testdoc already states the correct arity)
- Seen by: correctness; refutation: confirmed; history: deliberate but expired (accurate under the single-int opener; 3327a92b moved the opener to two items and the range to `2..=3` and missed this variant doc)
- Owner-gated: no

Adjacent to the partition (the atlas pins this error): the variant doc names an arity the decoder rejects. Prose contradicts code.

Evidence:

    131	    /// The frame item is not a one- or two-element CBOR array.
    132	    #[error("frame is not a CBOR reaction array: {detail}")]
    133	    FrameShape { detail: &'static str },

    decode.rs:
    225	    if !(2..=3).contains(&head.value) {

Resolution: "The frame item is not a two- or three-element CBOR array." Acceptance: the variant doc agrees with `frame_arity`. Dedupe against the codec-core partition if it reports the same line.

### remote-capture-atlas-22: `record_slices`'s visibility rationale names a capture-renderer use that does not exist
- Where: src/tree/mirror/streaming/remote/codec/frame.rs:246-248 (related: src/tree/mirror/streaming/remote/codec/frame.rs:232, src/tree/mirror/streaming/remote/codec/frame.rs:240, src/tree/mirror/streaming/remote/codec/capture.rs:561-567, src/tree/mirror/streaming/remote/codec/capture.rs:634-688)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn record_slices src` returns frame.rs:232, 240, 248 only; capture.rs parses run bytes through `parse_node` in `render_embedded_as`)
- Seen by: correctness; refutation: confirmed; history: deliberate but expired (true when written in 74ebba4d; ef6569c4 rewrote the renderer to walk run bytes generically and dropped every `record_slices` call)
- Owner-gated: no

Adjacent to the partition (the rationale is about this partition's module): the `pub(super)` is justified by a caller in capture.rs that no longer exists; the only users are `record_count` and `records` in the same file. No ghost references: a rationale naming a phantom caller is dated reasoning at a declaration site.

Evidence:

    246	    /// `pub(super)` for the capture renderer, which decodes each
    247	    /// record's version structurally without knowing the leaf type.
    248	    pub(super) fn record_slices(&self) -> RecordSlices<'_> {

Resolution: Make `record_slices` private and delete the rationale (and narrow `RecordSlices`'s visibility if nothing else needs it). Acceptance: `grep -rn record_slices src` shows only frame.rs; no prose names a nonexistent caller.

### remote-capture-atlas-23: Test-side `SIGNALS`, `SIGNAL_COUNT`, a bare `2`, and "All 340 placements" restate what the codec owns
- Where: src/tree/mirror/streaming/remote/codec/tests.rs:42-63 (related: src/tree/mirror/streaming/remote/codec/tests.rs:246, src/tree/mirror/streaming/remote/codec/tests.rs:331-340, src/tree/mirror/streaming/remote/codec/tests.rs:347, src/tree/mirror/streaming/remote/codec/tests.rs:477-482, src/tree/mirror/streaming/remote/codec/signal.rs:232-246, src/tree/mirror/streaming/remote/codec/signal/tests.rs:12-20)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (signal.rs:232 `pub const STATE_COUNT`, :235 `const STATES` (private) lists the same ten signals in the same order; signal/tests.rs:14-17 proves `STATES[i].state() == i`, so the `position` lookup at 479-482 equals `usize::from(signal.state())`)
- Seen by: structure, prose, correctness, perfapi; refutation: confirmed (noting `STATES` must widen to `pub(super)` since `codec::tests` is a sibling of `signal`, not a child); history: deliberate but expired (the copy had a mechanical reason at birth, both constants were private to signal.rs; 3327a92b made `STATE_COUNT` public and did not sweep the copy)
- Owner-gated: no

`SIGNAL_COUNT = 10` hand-maintains `Signal::STATE_COUNT`; `SIGNALS` is a byte-for-byte copy of the private `Signal::STATES`; the corpus bucketing recovers a signal's index by linear search over the copy where `signal.state()` is that index; `accepted = [0; 2]` uses a bare 2 beside the named `SPEAKER_COUNT`; and the atlas testdoc pins "All 340 placements" where the body derives the set from `SPEAKER_COUNT * Stream::COUNT * SIGNAL_COUNT`. A new `Signal` variant fails compilation at `representative_frame` (331-340, exhaustive) but not at `SIGNALS`, so the atlas and manifest would enumerate a stale roster without noticing. No hand-maintained counts: a number that matters lives in a mechanically enforced place the code cites by name.

Evidence:

    43	const SPEAKER_COUNT: usize = 2;
    ...
    46	const SIGNAL_COUNT: usize = 10;
    ...
    52	const SIGNALS: [Signal; SIGNAL_COUNT] = [
    53	    Signal::Match(Flow::Continue),

    246	/// All 340 placements pin either their canonical frame bytes or typed rejection.

    347	    let mut accepted = [0; 2];

    479	    let signal_index = SIGNALS
    480	        .iter()
    481	        .position(|candidate| *candidate == signal)
    482	        .expect("every frame maps to a semantic signal state");

    signal.rs:
    232	    pub const STATE_COUNT: u8 = Flow::STATE_COUNT * Self::REACTION_COUNT + Self::END_COUNT;
    235	    const STATES: [Signal; Self::STATE_COUNT as usize] = [

Resolution: `const SIGNAL_COUNT: usize = Signal::STATE_COUNT as usize;`; widen `Signal::STATES` to `pub(super)` and iterate it in place of `SIGNALS`; replace the `position` lookup with `usize::from(signal.state())`; write `[0; SPEAKER_COUNT]`; reword line 246 to "Every (speaker, stream, signal) placement pins either its canonical frame bytes or its typed rejection." Acceptance: codec/tests.rs contains no literal signal roster and no literal `10` or `340`; both snapshot files are byte-identical; a new `Signal` variant fails compilation in codec/tests.rs without touching a literal.

### remote-capture-atlas-24: `arb_query` cannot generate the full 256-child fan its range names
- Where: src/tree/mirror/streaming/remote/codec/tests.rs:89-92 (related: src/tree/mirror/streaming/remote/codec/tests.rs:98-118, src/tree/mirror/streaming/remote/codec/tests.rs:705-718, src/tree/mirror/streaming/remote/codec/decode/tests.rs:635-654)
- Class / severity / confidence: verification-gap / nit / high
- Provenance: verified (proptest 1.11.0 `collection.rs:516-528`: `btree_map` is `Filter::new(Map::new(vec((key, value), size), VecToBTreeMap), "BTreeMap minimum size", MinSize(size.start()))`; the only rejection is below the minimum, so duplicate `u8` keys collapse the realized size)
- Seen by: correctness; refutation: reframed to nit (the boundary is decoded by the async reader at 705-718, encoded at every count in encode/tests.rs, and rejected at 257 in both decoders; only the `#[cfg(test)]` sync `decode` oracle never sees a 256-fan); history: no rationale found
- Owner-gated: no

A request for 256 entries drawn from `any::<u8>()` yields about 162 distinct radixes, and fans above roughly 190 have negligible probability, so the three proptests on `arb_frame` never see the widest listing (whose map head is the only three-byte head in the query grammar). The stated bound is nominal. White-box worst-case construction: the input family that maximizes a listing's work is the full fan, and a strategy that names a boundary it cannot reach is coverage in appearance.

Evidence:

    89	fn arb_query() -> impl Strategy<Value = Vec<(u8, Hash)>> {
    90	    btree_map(any::<u8>(), arb_hash(), 0..=MAX_QUERY_CHILDREN)
    91	        .prop_map(|children: BTreeMap<_, _>| children.into_iter().collect())
    92	}

Resolution: Generate the fan as a subset of the radix space so every size in `0..=256` has positive probability: `proptest::sample::subsequence((0..=u8::MAX).collect::<Vec<u8>>(), 0..=MAX_QUERY_CHILDREN)` zipped with hashes, or a `vec(any::<bool>(), 256)` mask over `(radix, arb_hash())`. Acceptance: the strategy reaches 256 by inspection; the existing proptests pass; one committed case round-trips a 256-child query through the sync `decode`.

### remote-capture-atlas-25: Speaker drawn from a `bool` three times where an `arb_speaker()` strategy would do
- Where: src/tree/mirror/streaming/remote/codec/tests.rs:128-132 (related: src/tree/mirror/streaming/remote/codec/tests.rs:156-160, src/tree/mirror/streaming/remote/codec/tests.rs:670-674, src/tree/mirror/streaming/remote/codec/tests.rs:81-96)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (read; the identical five-line `if` at all three sites)
- Seen by: structure; refutation: confirmed; history: no rationale found
- Owner-gated: no

Three proptests take `initiator in any::<bool>()` and expand it identically into a `Speaker`; the file already defines `arb_stream`, `arb_hash`, and `arb_flow` in the `prop_oneof!`/`Just` style, so `arb_speaker` is the missing sibling.

Evidence:

    128	        let speaker = if initiator {
    129	            Speaker::Initiator
    130	        } else {
    131	            Speaker::Responder
    132	        };

Resolution: `fn arb_speaker() -> impl Strategy<Value = Speaker> { prop_oneof![Just(Speaker::Initiator), Just(Speaker::Responder)] }` and `speaker in arb_speaker()` at the three sites. Acceptance: no `initiator in any::<bool>()` remains.

### remote-capture-atlas-26: Fully qualified paths where the import already exists
- Where: src/tree/mirror/streaming/remote/codec/tests.rs:265-301 (related: src/tree/mirror/streaming/remote/codec/tests.rs:22, src/tree/mirror/streaming/remote/codec/capture.rs:236, src/tree/mirror/streaming/remote/codec/capture.rs:572, src/tree/mirror/streaming/remote/codec/capture.rs:588, src/tree/mirror/streaming/remote/codec/capture.rs:591, src/tree/mirror/streaming/remote/codec/capture.rs:712-714, src/tree/mirror/streaming/remote/codec/capture/tests.rs:190, src/tree/mirror/streaming/remote/codec/capture/tests.rs:251, src/tree/mirror/streaming/remote/codec/capture/tests.rs:255)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (codec/tests.rs:22 imports `cbor`, used as `cbor::head_len` at 567 and 596; capture.rs:53-56 imports `MAJOR_TEXT` and the constants named, with no `use crate::tags`; grep for `crate::tags::` and `cbor::MAJOR_TEXT` in the cited files)
- Seen by: structure, correctness, perfapi; refutation: reframed (the `cbor::TAG_SELF_DESCRIBED` sites are short module-qualified paths through the imported `self`, not long paths; only inconsistent with their imported siblings); history: deliberate but expired for codec/tests.rs (the `cbor` import arrived in 0521b207, twelve days after the qualified sites, and did not sweep them); no rationale for the capture.rs sites
- Owner-gated: no

The atlas snapshot body spells `crate::tree::mirror::cbor::write_head` and `crate::tree::mirror::cbor::MAJOR_UINT`/`MAJOR_ARRAY` six times beside an existing `cbor` import; capture.rs writes `crate::tags::PARTY_TAG`/`VERSION_TAG`/`CLOCK_TAG` seven times with no `use crate::tags`; capture/tests.rs:190 and 251 repeat the pattern and line 255 writes `cbor::MAJOR_TEXT` though `MAJOR_TEXT` arrives via `use super::*`. Imports over long qualified paths except where the qualification informs; here it carries nothing the neighbouring imported names do not.

Evidence:

    22	        mirror::{cbor, framing::PAYLOAD_CHUNK_LEN},
    ...
    265	                        crate::tree::mirror::cbor::write_head(
    266	                            &mut expected_opener,
    267	                            crate::tree::mirror::cbor::MAJOR_UINT,

    capture.rs:
    236	        (MAJOR_TAG, crate::tags::PARTY_TAG) => "party hand-off",
    572	        (crate::tags::VERSION_TAG, Node::Bytes(bytes)) => {

Resolution: In codec/tests.rs:265-301 use `cbor::write_head`/`cbor::MAJOR_UINT`/`cbor::MAJOR_ARRAY`. In capture.rs add `use crate::tags::{CLOCK_TAG, PARTY_TAG, VERSION_TAG};` (and optionally `TAG_SELF_DESCRIBED` to the `cbor` import list for consistency). In capture/tests.rs write `VERSION_TAG`, `PARTY_TAG`, `MAJOR_TEXT`. Acceptance: `grep -n 'crate::tree::mirror::cbor::' codec/tests.rs` and `grep -n 'crate::tags::' capture.rs capture/tests.rs` are empty.

### remote-capture-atlas-27: `write_hex` hand-rolls `hex::encode`, which the same file already uses
- Where: src/tree/mirror/streaming/remote/codec/tests.rs:325-329 (related: src/tree/mirror/streaming/remote/codec/tests.rs:282, src/tree/mirror/streaming/remote/codec/tests.rs:415)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (line 415 calls `hex::encode`; `hex` is a regular dependency in Cargo.toml)
- Seen by: structure; refutation: confirmed; history: no rationale found (`hex` was already a dependency when `write_hex` was written)
- Owner-gated: no

Two spellings of one operation in one file; prefer the dependency over hand-rolling. `hex::encode` produces the identical lowercase two-digit string, so the snapshot does not move.

Evidence:

    325	fn write_hex(out: &mut impl Write, bytes: &[u8]) {
    326	    for byte in bytes {
    327	        write!(out, "{byte:02x}").unwrap();
    328	    }
    329	}

Resolution: Replace `write_hex(&mut atlas, &encoded);` at line 282 with `atlas.push_str(&hex::encode(&encoded));` and delete `write_hex`. Acceptance: `canonical_frame_atlas_snapshot` passes without `cargo insta review`.

### remote-capture-atlas-28: The bounded corpus test runs about 2.2 million codec round-trips at opt-level 0 with a fresh allocation per case, unmeasured
- Where: src/tree/mirror/streaming/remote/codec/tests.rs:487-497 (related: src/tree/mirror/streaming/remote/codec/tests.rs:34, src/tree/mirror/streaming/remote/codec/tests.rs:375, src/tree/mirror/streaming/remote/codec/tests.rs:389, Cargo.toml:174-187, .config/nextest.toml:25-26)
- Class / severity / confidence: performance / low / medium
- Provenance: assessed (read; `EXHAUSTIVE_FRAME_CASES = 1_118_600` is asserted at 389 and I recomputed it: per stream, 2 flows x (Match + Supply + C(256,0) + C(256,1) + C(256,2) queries) + 2 ends = 65,800; x17 = 1,118,600; `check_both` runs each case for both speakers; Cargo.toml's dev profile raises opt-level only for `before` and `suanpan`; nextest's slow period is 60 s. No wall time is recorded anywhere I searched and I did not run it)
- Seen by: perfapi; refutation: uncertain (facts verified; cost unmeasured); history: no rationale found
- Owner-gated: no

Per case the loop allocates a fresh `Vec::new()`, encodes, decodes through `decode_exact` (which allocates the frame), and feeds SHA3, with `children.to_vec()` per query case; `rumors` and `sha3` compile at opt-level 0 in the unit-test binary. The denominator (2.2 million cases per run) is large enough that this single test plausibly sits on the suite's critical path, but no committed number says so. Instruments before cures: measure, then land the fixed-sign items.

Evidence:

    490	                let mut encoded = Vec::new();
    491	                encode(speaker, &frame, &mut encoded).unwrap();
    492	                accepted[direction] += 1;
    493	                assert_eq!(
    494	                    decode_exact(speaker, RunBudget::default(), &encoded).unwrap(),
    495	                    frame
    496	                );
    497	                bucket.accept(&encoded);

Resolution: Measure first (`just test bounded_corpus_manifest_snapshot`, wall time under the dev profile, recorded). If material, land the two fixed-sign items: reuse one encode buffer across the loop (`encoded.clear()`), and `[profile.dev.package.sha3] opt-level = 2` following the manifest's own precedent; re-measure. Whether the 1.1-million-case enumeration earns its remaining cost (given `frame_round_trips` samples the family and the atlas pins the 340 placements) is the owner's call and would move the manifest snapshot. Acceptance: a before/after wall time is recorded; the manifest snapshot is byte-identical after the fixed-sign changes.

### remote-capture-atlas-29: The frame-to-signal projection is written three times: inline in the encoder and twice verbatim in tests
- Where: src/tree/mirror/streaming/remote/codec/tests.rs:504-514 (related: src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:543-553, src/tree/mirror/streaming/remote/codec/encode.rs:107-124, src/tree/mirror/streaming/remote/codec/tests.rs:28)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'fn frame_signal' src` returns the two test sites, whose bodies are byte-identical by reading; encode.rs:107-124 computes the same five-arm projection inline in `FrameEncoding::new`; `mod error_atlas;` at codec/tests.rs:28 makes `use super::frame_signal` the zero-cost minimum)
- Seen by: structure, correctness, perfapi; refutation: confirmed (neither test copy acts as a differential oracle: both only filter placements and pick buckets); history: no rationale found (both copies date from d88d84e5; maintained in lockstep through every roster change)
- Owner-gated: no

A change to the signal roster must be made in three places, two of them test code where a stale copy narrows what the corpus enumerates without failing to compile. A `Frame::signal(&self) -> Signal` method would be the one home and would let `FrameEncoding::new` read as signal-then-body.

Evidence:

    504	fn frame_signal(frame: &Frame) -> Signal {
    505	    match frame {
    506	        Frame::Reaction(Reaction::Match, flow) => Signal::Match(*flow),
    507	        Frame::Reaction(Reaction::Query(children), flow) if children.is_empty() => {
    508	            Signal::QueryEmpty(*flow)
    509	        }
    510	        Frame::Reaction(Reaction::Query(_), flow) => Signal::Query(*flow),
    511	        Frame::Reaction(Reaction::Supply(_), flow) => Signal::Supply(*flow),
    512	        Frame::End(end) => Signal::End(*end),
    513	    }
    514	}

    encode.rs:
    107	        let (signal, body) = match frame {
    108	            Frame::Reaction(Reaction::Match, flow) => (Signal::Match(*flow), BodyEncoding::Empty),

Resolution: Add `pub(super) fn signal(&self) -> Signal` on `Frame` in frame.rs; have `FrameEncoding::new` compute `let signal = frame.signal();` and match only on the body; delete both test copies and call `frame.signal()`. Minimal alternative: delete error_atlas.rs:543-553 and `use super::frame_signal;`. Acceptance: exactly one `Frame -> Signal` match exists in the tree; codec and atlas snapshots are byte-identical.

### remote-capture-atlas-30: Import groups in the atlas are interleaved: crate, std, tokio, super, crate, serde
- Where: src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:25-47 (related: src/tree/mirror/streaming/remote/codec/tests.rs:27-28)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read; rustfmt sorts within a group but never merges groups)
- Seen by: structure; refutation: confirmed; history: no rationale found (accretion across five commits; the `use serde::Serialize;` directly above the first doc comment came from a mechanized sweep)
- Owner-gated: no

The file opens with a `crate::` import, then `std`, then `tokio`, then `super::super`, then two more `crate::` lines, then `use serde::Serialize;` with no blank line before the `WITNESS_MARKERS` doc comment. codec/tests.rs:27-28 has the same `use serde::Serialize;` / `mod error_atlas;` adjacency.

Evidence:

    25	use crate::message::{PayloadCodec, PayloadDepthLimit};
    26	use std::{
    ...
    34	use tokio::io::AsyncWrite;
    ...
    43	use crate::tree::mirror::cbor::{self, MAJOR_BSTR, MAJOR_TAG, TAG_CBOR_SEQUENCE};
    44	use crate::{Version, message::Message, tree::typed::Hash};
    45	
    46	use serde::Serialize;
    47	/// One rendered marker per error variant the atlas must witness.

Resolution: Regroup as std, external (`serde`, `tokio`), `crate`, `super`; put a blank line before the first item's doc comment; same at codec/tests.rs:27-28. Acceptance: each file's import block reads as contiguous groups.

### remote-capture-atlas-31: The admitted `FramePart` marker hole is closable with a wildcard-free `describe_part`
- Where: src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:76-86 (related: src/tree/mirror/streaming/remote/codec/error.rs:40-53, src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:14-18)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (error.rs:41-53: `FramePart` has five variants and derives `Debug`; the markers are `part={variant:?}` spellings)
- Seen by: correctness; refutation: confirmed; history: no rationale found (3939d36d gave every other error enum a wildcard-free describe function and left `FramePart` to a comment; 151135c5 reworded the comment for accuracy but kept the by-hand mechanism)
- Owner-gated: no

The rest of the file's design is that every enum gets a wildcard-free `describe_*` so the compiler is the tripwire; `FramePart` is the one enum left to a convention held in a comment. Every hole found becomes a committed check, never a convention held in memory.

Evidence:

    79	    // never fails at the signal separately from the frame head). FramePart
    80	    // has no exhaustive match here, so a new component's marker must be
    81	    // added by hand alongside its witnesses.
    82	    "part=FrameHead",
    83	    "part=Signal",

Resolution: Add `fn describe_part(part: FramePart) -> &'static str` with a wildcard-free match returning each variant's `Debug` spelling; derive the five `part=` markers from it (or assert in `atlas_covers_every_error_variant` that `part=<describe_part(v)>` appears for every variant). Then the comment can say the compiler enforces the roster. Acceptance: adding a `FramePart` variant without a witness fails compilation or the coverage test; the by-hand sentence is removed.

### remote-capture-atlas-32: The one variant-backed exemption is not tied to its variant, so the confessed exemption-rot hole stays open
- Where: src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:92-98 (related: src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:20-23, src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:89-91, src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:146-152, src/tree/mirror/framing.rs:57-65, src/tree/mirror/streaming/remote/codec/error.rs:64-76)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (framing.rs:59-65: `LengthOverflow { pub len: usize, pub source: std::num::TryFromIntError }`, both fields public; codec/error.rs:65-76: `SupplyTooLarge(#[from] LengthOverflow)`; the coverage test asserts only `!atlas.contains(marker)` at 146-152)
- Seen by: structure, prose; refutation: reframed (the `Greeting`/`Listing` entries name no variant of any inventoried enum, but 151135c5 added them deliberately as promotion tripwires with the reason stated inline and at module doc lines 6-12, so deleting them reopens a recorded decision without new evidence; the constructive half survives); history: deliberate and holds for the tripwires; the exemption-rot hole was a recorded choice (33bd7d56 punch list) to document rather than close, and nothing rules against closing it
- Owner-gated: no

The module doc admits (lines 20-23) that an exemption outlives its variant silently because the check asserts absence, which a deleted variant satisfies trivially, so pruning must happen by hand. For the one exemption that names a real variant, `SupplyTooLarge`, the hole is closable: both `LengthOverflow` fields are public, so the test can construct the variant, describe it, and assert the description carries the exempt marker; deleting the variant then fails compilation and renaming it fails the marker match. The `EXEMPT_MARKERS` summary at line 89 ("Variants deliberately absent") also reads loosely against the two tripwire entries, which name handshake-layer types rather than variants of the inventoried enums.

Evidence:

    92	const EXEMPT_MARKERS: &[(&str, &str)] = &[
    93	    (
    94	        "kind: SupplyTooLarge(",
    95	        "requires a run body past the wire's run byte cap: a >4 GiB in-memory \
    96	         run is resource exhaustion by construction; the cap itself is pinned \
    97	         at its exact boundary in frame/tests.rs",
    98	    ),

    20	//! or the witnesses; the comments at each match carry that obligation. An
    21	//! exemption also outlives its variant silently — the check asserts absence,
    22	//! which a deleted variant satisfies trivially — so pruning a variant must
    23	//! prune its `EXEMPT_MARKERS` entry by hand.

Resolution: In `atlas_covers_every_error_variant`, build `EncodeErrorKind::SupplyTooLarge(LengthOverflow { len: usize::MAX, source: u32::try_from(u64::MAX).unwrap_err() })`, run `describe_encode_kind` on it, and assert the output starts with the marker's suffix (`SupplyTooLarge(`). Rescope lines 20-23 to the two type-level tripwires (which have no variant to prune) or delete the sentence. Reword line 89 to "Markers deliberately absent from the atlas". Acceptance: every variant-backed `EXEMPT_MARKERS` entry is tied to a constructed instance; the module doc no longer describes a by-hand pruning obligation for variants.

### remote-capture-atlas-33: The atlas pins one of `FrameShape`'s two `detail` strings
- Where: src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:305-308 (related: src/tree/mirror/streaming/remote/codec/decode.rs:219-231, src/tree/mirror/streaming/remote/codec/decode/tests.rs:182-192)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (`grep -rn 'not two or three' src tests` returns only decode.rs:227; the atlas's `FrameShape` witness is the single `[0x00]` case)
- Seen by: correctness; refutation: reframed (the claim that no test exercises the arity-out-of-range branch is refuted: `frame_shape_is_enforced` at decode/tests.rs:182-192 feeds `[0x81]` and `[0x84]` and asserts `FrameShape { .. }`; what is true is that the second `detail` string is pinned nowhere); history: no rationale found (3327a92b changed the range and the detail text and re-accepted the atlas without adding the case)
- Owner-gated: no

`detail` is part of the typed error's observable payload, and the atlas exists to be the stable witness roster for exactly these strings; the `frame array is not two or three items` spelling can drift without any pin moving.

Evidence:

    307	        let error = decode_exact(speaker, RunBudget::default(), &[0x00]).unwrap_err();
    308	        record_decode(atlas, &format!("{speaker:?}/frame/not-an-array"), &error);

Resolution: Add `frame/arity-one` from `[0x81, stream]` and `frame/arity-four` from `[0x84, stream, state, 0, 0]` beside `frame/not-an-array`, each recorded as `FrameShape(frame array is not two or three items)`; re-accept the atlas snapshot naming the added witnesses. Acceptance: `codec_error_atlas_snapshot` gains the two entries.

### remote-capture-atlas-34: Synchronous fail-after IO doubles are re-declared per codec test file
- Where: src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:674-747 (related: src/tree/mirror/streaming/remote/codec/encode/tests.rs:148-158, src/tree/mirror/streaming/remote/codec/decode/tests.rs:774-780)
- Class / severity / confidence: simplification / nit / medium
- Provenance: verified by the structure lens (grep for `impl std::io::Write for` / `impl std::io::Read for` under codec/: exactly the four impls); I read the atlas pair and the refutation pass read the two siblings
- Seen by: structure; refutation: confirmed (nit and owner taste: local doubles read locally); history: no rationale found
- Owner-gated: no

`FailAfterWriter`/`FailAfterReader` (fail after N delivered bytes) generalize `encode/tests.rs`'s `FailingWriter` and `decode/tests.rs`'s `FailingReader` (N = 0). One pair visible from `codec::tests` would serve all three; the steelman for locality is fair, which is why this is a nit.

Evidence:

    684	impl std::io::Write for FailAfterWriter {
    685	    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
    686	        if self.remaining == 0 {
    687	            return Err(io::ErrorKind::Other.into());
    688	        }

Resolution: Move the pair into codec/tests.rs as `pub(super)` items and have the two siblings use `FailAfterWriter::new(0)` / `FailAfterReader::new(bytes, 0)`. Acceptance: one `impl std::io::Write` and one `impl std::io::Read` test double under `codec/`.

### remote-capture-atlas-35: The rule deciding which exported error enums are `#[non_exhaustive]` lives only in git history
- Where: src/tree/mirror/streaming/remote/error.rs:11-16 (related: src/error.rs:44-50, src/tree/mirror/streaming/remote/codec/error.rs:12, src/tree/mirror/streaming/remote/codec/error.rs:42, src/tree/mirror/streaming/remote/codec/error.rs:65, src/tree/mirror/streaming/remote/codec/error.rs:104, src/tree/mirror/streaming/remote/codec/frame.rs:377, src/tree/mirror/streaming/remote/codec/signal.rs:139, src/tree/mirror/streaming/remote/codec/signal.rs:388, src/tree/mirror/streaming/remote/adapter/error.rs:7, src/tree/mirror/streaming/remote/adapter/error.rs:30, src/tree/mirror/streaming/remote/adapter/error.rs:44, src/tree/mirror/streaming/remote/streams.rs:102, src/tree/mirror/streaming/remote/streams.rs:239, src/tree/mirror/cbor.rs:150)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (grep of `pub enum` with preceding attributes in each definition file: `#[non_exhaustive]` is present on `DecodeErrorKind`, `ListingIssue`, adapter `DecodeError<E>`, streams `StreamError`, `AcceptError`, `GreetingError`, proxy `Error<E>`; absent on `Origin`, `FramePart`, `EncodeErrorKind`, `DecodeLeafError`, `LeafRunError`, `StreamClass`, `DecodeSignalError`, `ScopeError`, `OpeningError`, adapter `EncodeError<E>`, `ReplyFrameError`, `SendError`, `HeadError`; all are re-exported through remote/error.rs into `crate::error`)
- Seen by: perfapi; refutation: confirmed; history: deliberate and holds, rationale unstated in code (1e458d69: "the contract-outcome and wire-grammar enums stay exhaustive deliberately"; REVIEW.md post-rebase ruling R3: `HeadError` "stays exhaustive (no `#[non_exhaustive]`)")
- Owner-gated: yes (any change to the attribute set is a public API decision; the recorded rule is the owner's)

The taxonomy this module assembles is offered by `crate::error` "for matching and bug reports", and the split between exhaustive and non-exhaustive enums follows a recorded rule (taxonomies that grow with enforcement are open; contract-outcome and wire-grammar enums stay closed) that no enum states at its definition, so to a reader of the code the omissions look like accidents and a future maintainer adding a variant to a closed enum has no signal that it is a semver decision. Two enums also bear a second look against the rule's own criterion: `DecodeSignalError`'s variant set changed in 3327a92b (`Reserved` became `Stream` and `State`), and `LeafRunError` is decode-side structural validation, so both arguably "grow as enforcement grows".

Evidence:

    11	pub use super::codec::{
    12	    DecodeError as CodecDecodeError, DecodeErrorKind as CodecDecodeErrorKind, DecodeLeafError,
    13	    DecodeSignalError, EncodeError as CodecEncodeError, EncodeErrorKind as CodecEncodeErrorKind,
    14	    FramePart, GreetingError, HeadError, InvalidSignalPlacement, LeafRunError, ListingIssue,
    15	    Origin, QueryOrderError, Speaker, Stream, StreamClass,
    16	};

Resolution: State the rule once where the taxonomy is assembled (this module's doc or `crate::error`'s) and mark each deliberately closed enum with a one-line comment at its definition ("closed by design: a wire-grammar vocabulary; a new variant is a protocol change"). Separately decide `DecodeSignalError` and `LeafRunError` against the recorded criterion. Acceptance: every `pub enum` reachable from `crate::error` either carries `#[non_exhaustive]` or a closed-by-design comment; the rule is stated in one place the enums can cite.

## Positives

- capture.rs: the depth budget is one counter spanning structural descent and embedded byte-string re-parses (`MAX_DEPTH` at 341; `render_embedded_as` re-parses at the consumed depth, 647), the invariant is stated once at `render_node` (447-455), and I checked it at every `render_*` call site (281, 295, 459, 476, 486, 490, 539, 567, 570, 597, 607, 647, 685): it holds. Three committed deep-input tests drive it, including `deep_payload_through_the_frame_path_falls_back` (capture/tests.rs:338-354), which shows that a payload the depth limit admits can still demand 640 unfold levels. This is the no-input-controlled-recursion rule done exactly right.
- error_atlas.rs: coverage is enforced from both ends. Wildcard-free `describe_*` matches (488-497, 568-576, 592-658) make a new variant a compile error until described; `atlas_covers_every_error_variant` (136-153) makes a described-but-unwitnessed variant a test failure, with failure messages naming the table to edit; and the module doc (18-23) names the one hole neither half can see instead of claiming totality.
- codec/tests.rs: the read-plan meter holds three things to each other: a formula over the frame's wire shape (`read_plan`, 620-638, documented in English that matches the code line for line), the reader's actual transport reads (`decode_counting`, 643-656), and pinned numbers at reference shapes (`read_plan_at_reference_shapes`, 696-739), so neither the formula nor the reader can drift alone.
- codec/tests.rs: the exhaustive corpus constants are asserted, and the arithmetic checks by hand: 2 flows x (2 + 1 + 256 + 32,640) + 2 = 65,800 frames per stream, x17 = 1,118,600.
- capture/tests.rs: `supply_reflection_localizes_the_field_that_moved` (46-89) tests the renderer's purpose (a one-field change moves exactly one rendered line carrying the exact value) rather than its output text, and the fixture rationale at 68-71 (a non-flat event tree so containment cannot match vacuously) is exactly the "why" a future reader needs.
- capture.rs: every capture-integrity panic names why it is the harness's fault, not the peer's (module doc 42-47, `label_item` 127-132, `render_frame` 252-267), and the one `expect` inside the parser (`parse_major_seven`, 399) carries a one-line proof pointing at the caller's peek. `parse_major_seven` (398-432) gets the RFC 8949 corners right: one-byte simples below 32 rejected as non-canonical, float widths 25..=27 with exact bits, reserved 28..=30 and indefinite 31 both refused.
- remote/error.rs is a clean, flat alias layer: nineteen lines, no logic, one renaming convention (`Codec*`, `Reply*`) that resolves same-named adapter and codec types without leaking module paths, consumed by name from src/error.rs:44-50.
- The totality witness is enforced, not merely stated: tests/common/gossip_snapshot.rs:350 and 363 call `assert_items_account_for` per directed stream before anything is rendered, so the rendering-as-byte-pin license is actually held.

## Open questions for Finch

- Injectivity pin form (finding 17): an inverse parser from the rendering back to bytes pins injectivity outright but makes the rendering grammar load-bearing; a leaf-mutation property is cheaper and weaker. Recommendation: the inverse parser; it also catches the float-width and trailing-byte survivors the mutants note lists, and the grammar is already stable enough to pin.
- Layering of the test-internals shelf (finding 3): `codec` is `pub(crate)` and src/tests.rs:281 bypasses `remote` to reach `codec::greeting::encode_greeting`. Either make `codec` private and add a `remote`-level re-export for the greeting encoder, or drop the shelf and let `testing.rs` import from `remote::codec`. Recommendation: keep the shelf, make `codec` private, and route src/tests.rs through `remote`; one door is easier to read from `tests/common`.
- `MAX_DEPTH` (finding 10): keep 64 and document, or derive from `DEFAULT_PAYLOAD_DEPTH_LIMIT` plus the frame's structural overhead so every admissible payload renders as a tree. Recommendation: keep 64 and document; deriving deepens `render_node`'s recursion to 256-plus frames for a legibility gain no fixture needs.
- `LinkCapture`'s home: the crate never reads it (its only in-crate appearances are the definition and three re-export hops; `tests/common/gossip_snapshot.rs` constructs it and re-aliases it as `CapturedLink`). It stays crate-side because design/rumors-frame-fuzz.md section 4 plans to consume it from a separate fuzz crate through `rumors::testing`. If that plan proceeds, the placement is right; if it is dropped, move the type to `tests/common`. Recommendation: leave it until the fuzz target lands or is abandoned, then revisit.
- Em-dashes in `//` comments: two sites in this partition (error_atlas.rs:77, 588), but 116 across `src/`, so the crate's de facto convention is the em-dash in line comments and AGENTS.md rules on nothing here; the spaced double-hyphen rule is your global doctrine. Recommendation: decide once, crate-wide, and if double-hyphens win, sweep mechanically rather than per partition.
- `#[non_exhaustive]` on `DecodeSignalError` and `LeafRunError` (finding 35): both arguably meet 1e458d69's "grows as enforcement grows" criterion. Recommendation: mark both; they are decode-side validation taxonomies, and a downstream wildcard arm is the correct response to a new one.
- `render_frame` (capture.rs:252) reads the frame array's head and asserts its major but never holds `head.value` (the arity) against the number of body items it parses at 277-287, and the module doc's list of capture-integrity panics (42-46) does not mention arity. Recommendation: add the assertion; the decoder's `FrameArity` check on the real path means a mismatch is a harness bug, which is exactly what the renderer panics on.
- Wall time of `bounded_corpus_manifest_snapshot` (finding 28): unmeasured. Recommendation: time it once and record the number; if it is under a few seconds, close the finding.
- The `Greeting`/`Listing` exemption entries (error_atlas.rs:99-111) are deliberate promotion tripwires per 151135c5 and cannot fire on today's code. Recommendation: keep them (the cost is two entries and the module doc explains them) and fix the line-89 summary as finding 32 proposes.
- Two out-of-partition pointers the lenses raised that I did not verify: the perfapi lens reports that `DEFAULT_TARGET_MESSAGE_SIZE`'s export site (remote.rs:92) is imported by materialized.rs, inverting the layer order streaming.rs:10-24 states; and the prose lens reports that AGENTS.md's renderer-vocabulary re-accept witness speaks of "hexdump line sequence" while the renderer emits a value tree. Both belong to other partitions or to AGENTS.md; forwarded, unverified.

## Dropped

- `LinkCapture` is defined in the crate but consumed only by `tests/common` [6]: deliberate and holding per design/rumors-frame-fuzz.md section 4 (a fuzz crate needs it through `rumors::testing`); moved to open questions.
- Two atlas exemptions assert the absence of markers no described variant can render [7], [22]: the `Greeting`/`Listing` entries were added deliberately in 151135c5 as promotion tripwires with the reason stated inline and at module doc lines 6-12; deleting them reopens a recorded decision without new evidence. The constructive half (tie `SupplyTooLarge` to a constructed instance; fix the line-89 summary) survives as finding 32.
- The re-export layer is circular justification [5]: reframed by the refutation pass; a curated shelf is the same pattern codec.rs:80-84 uses for `capture`, and the layering choice is taste. The mechanical cfg merge survives as finding 3; the layering question is under open questions.
- The glob doubles the codec and adapter `DecodeError`s too (sub-claim of [4]): refuted; `codec` is `pub(crate) mod` and `adapter` is private, neither glob-exported, so only `proxy::Error` is doubled (finding 5).
- The arity-out-of-range branch has no witness anywhere [34]: refuted; `frame_shape_is_enforced` (decode/tests.rs:182-192) feeds `[0x81]` and `[0x84]` and asserts `FrameShape`. The unpinned `detail` string survives as finding 33.
- codec/tests.rs lacks a module doc (half of [40]): below the bar; no rule requires test-module docs (the gate's `testdoc` checks functions), and signal/tests.rs lacks one too.
- Em-dashes in two `//` comments (half of [29]): crate-wide convention question (116 sites), not a partition nit; moved to open questions. The two adverbs survive as finding 18.
- `cbor::TAG_SELF_DESCRIBED` left module-qualified (part of [9]): reframed by the refutation pass as a short qualified path through the imported `self`, a consistency nit folded into finding 26's optional step.
- Duplicates merged: [21], [30] into finding 13; [47] into 29; [46] into 4; [43], [52] into 5; [23], the count halves of [24] and [40] into 1; [32] into 2; [48] and the tag half of [41] into 26; the major half of [41] into 11; [50] into 15; [49] into 14; [36] into 10; [17] and the roster half of [24] into 23; [51] into 8.
- Refutation-pass new item: a single container-keyed listing entry renders `… =>` with no annotation: folded into findings 13 and 14.
- Refutation-pass new item: streams.rs:3 also hand-states "17-per-direction": folded into finding 1 as a related site.
