# Partition remote-adapter-tests: The remote adapter test suites

## Partition summary

This partition is the unit specification of the reply/frame adapter in `src/tree/mirror/streaming/remote/adapter/{decode,encode,scope,error}.rs`: the lossless boundary between erased protocol replies (`Reply<E>` holding node handles, no prefix) and prefix-free wire frames (leaf runs, positional `Match` and `Query`). `tests.rs` (80 lines) holds the shared fixture vocabulary: `LeafCase`, `hash`, `leaf_run`, `unbounded`, `runtime`. `properties.rs` (1148) sweeps six laws over every concrete type-level height 0..32 under proptest through a recursive dispatch macro. `malformed.rs` (719) pins the typed rejections and a few admitting boundaries. `opening.rs` (326) covers the greeting-borne opening reply and the `early_supplies` stream. `runs.rs` (296) states the encoder's byte-budget batching contract as a proptest family plus three point witnesses. `backend_errors.rs` (213) is a per-height, per-operation injected-failure matrix. `parking.rs` (238) and `fan_occupancy.rs` (183) pin the memory-model premises: a parked reply costs handles, not subtrees, and the reader/assembler channel peaks at exactly `FAN + 1`. All 3203 lines are test code; I read every line of every file, plus the production sites each finding leans on (decode.rs, encode.rs, failing.rs, frame.rs, message.rs, queues.rs, work.rs, pump.rs, window.rs and its tests, hash.rs, local.rs, before's gamma and literal codecs).

The test design is strong. Several suites are built so that the wrong implementation yields a different typed error rather than a passing run: the eager-rejection test with two `Continue` frames, the sentinel-frame technique proving a decode consumes nothing of the following reply, the paced negative control that keeps the fan-occupancy pin from passing vacuously, the census liveness floor in the set-length test, and an injected-failure matrix that asserts the backend's operation history rather than only the error variant. Everything is deterministic: current-thread runtimes, in-process frame vectors, thread-local probes, no clocks. No residue of the V1 protocol or BLAKE3 remains in these files, and no proptest seed exists for them, so there is nothing stale to sweep. The lenses that read this partition independently found no harness bug that masks a failure, and I confirm that reading.

The dominant issues are maintenance shape and drifted prose. The same fixture searches, tail-flagging loops, node-at-height traits, dispatch macros, and the three ingress premises (`u64::MAX`, `unbounded()`, `PayloadCodec::new::<u64>(PayloadDepthLimit::default())`, spelled 36, 36, and 40 times) are repeated across the seven files; `properties.rs` states each of its six laws twice, once for `Z` and once for `S<H>`, with the bodies textually identical apart from which adapter entry they call. Two documentation-of-record numbers have expired: `parking.rs`'s module doc states megabyte figures two format changes behind its own pin constant and `message.rs`, and `runs.rs`'s `MAX_RECORD_LEN` derivation describes the retired borsh framing while nine of the ten records in the largest committed fixture exceed the "upper envelope" it claims (verified by an offline model of the framing that reproduces `before`'s committed encoding vector). The one substantive verification gap is that `read_early`, a second hand-written copy of the frame grammar, has only three of its rejection arms exercised on its own path; the mutant campaign recorded that gap with a disposition that has not landed.

## Findings

### remote-adapter-tests-1: tests.rs module map omits `backend_errors`
- Where: src/tree/mirror/streaming/remote/adapter/tests.rs:1-11 (related: src/tree/mirror/streaming/remote/adapter/tests.rs:23)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read lines 1-29; `git log -L1,11` shows f94f2056, 3a5ba643, and d6537a7b each appended their module to the map and no commit added `backend_errors`, which 07504b2e declared)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale found (completeness is the evident intent; this is the one omission)
- Owner-gated: no

The module doc maps six of the seven submodules by name and role and omits `backend_errors`, declared at line 23. A reader using the map to find where injected backend failures are covered will not find it. A module map that enumerates its members is a hand-maintained list, and this one has already rotted once.

Evidence:

    3	//! [`properties`] states the adapter's laws and sweeps the complete type-level
    4	//! height ladder. [`malformed`] pins the smaller set of wire shapes which must
    5	//! be rejected before those laws can apply. [`opening`] covers the one
    6	//! deliberately exceptional reply in the protocol. [`runs`] states the
    7	//! supply-run batching contract the byte budget imposes on the encoder.
    8	//! [`parking`] pins the memory accounting that makes a parked decoded reply
    9	//! O(fan) handles rather than a subtree. [`fan_occupancy`] pins the
    10	//! reader/assembler channel's occupancy ceiling — the supply-decode
    11	//! envelope's charge premise.
    ...
    23	mod backend_errors;

Resolution: add one sentence for `backend_errors` (its role is source-error propagation and atomicity across the backend operations the adapter reaches), or restate the map as structure rather than roster. Acceptance: every `mod` declared in tests.rs is named in its module doc.

### remote-adapter-tests-2: the three ingress premises are re-spelled at every decode and encode call site
- Where: src/tree/mirror/streaming/remote/adapter/tests.rs:35-39 (related: src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:42-55, src/tree/mirror/streaming/remote/adapter/tests/properties.rs:111-119, src/tree/mirror/streaming/remote/adapter/tests/opening.rs:153-165, src/tree/mirror/streaming/remote/adapter/tests/runs.rs:108-128)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`grep -c 'PayloadCodec::new::<u64>(PayloadDepthLimit::default())'` per file: backend_errors 1, fan_occupancy 2, malformed 14, opening 5, parking 2, properties 12, runs 4, total 40; `u64::MAX,` 36 and `unbounded(),` 36 across the partition)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: codemod residue (165b0dd3 threaded `version_bytes`, 08f2899b the ledger with the `unbounded()` helper, 4356e197 the codec inline at every site)
- Owner-gated: no

`PayloadCodec::new::<u64>(PayloadDepthLimit::default())` appears 40 times and `u64::MAX, unbounded(),` 36 times, while the only helper in hand covers one premise of three. The handful of tests that genuinely vary a premise (malformed.rs:523-571 the version bound, malformed.rs:610-683 and opening.rs:187-222 the ledger, backend_errors.rs the backend) are visually indistinguishable from the forty that do not, and the varied parameter is the thing a reader came to see.

Evidence:

    35	/// A set-length allowance no fixture here can exhaust, for tests whose
    36	/// subject is not the ingress supply charge.
    37	fn unbounded() -> SupplyLedger {
    38	    SupplyLedger::new(u64::MAX)
    39	}

    42	    let error = runtime().block_on(async {
    43	        let mut frames = stream::iter(frames);
    44	        decode_leaf_reply(
    45	            Local,
    46	            u64::MAX,
    47	            unbounded(),
    48	            Scope::new(parent.erase(), &[(0, hash(0))]),
    49	            &mut frames,
    50	            PayloadCodec::new::<u64>(PayloadDepthLimit::default()),
    51	        )

Resolution: add to tests.rs a `fn codec() -> PayloadCodec` and thin wrappers that fix `Local, u64::MAX, unbounded(), codec()` for the common decode, leaf-decode, and early-supply shapes, leaving the raw six-argument calls only where a premise is varied so those sites stand out. Acceptance: the codec constructor appears in the helper and at the sites that vary it, nowhere else; the tests varying the version bound, ledger, or backend are the only ones spelling those arguments.

### remote-adapter-tests-3: `LeafCase` doc claims distinct cases produce distinct versions, but the fold drops `value`'s top byte
- Where: src/tree/mirror/streaming/remote/adapter/tests.rs:65-73
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read; `wrapping_shl(8)` on `u64` wraps only the shift amount, so it is `value << 8` and `(v, t)` collides with `(v + 2^56, t)`)
- Seen by: structure-prose, blind-spots; refutation: confirmed (and noted the `wrapping_` prefix misleads); history: no rationale found (961f63c6 introduced the fold; injectivity was never discussed)
- Owner-gated: no

The doc states a universal property that the fold does not have, and `wrapping_shl` suggests a wrapping fold of the value where there is none. No current test relies on injectivity (`foreign_parent` compares prefixes), so nothing breaks; the standard is that a helper's stated property holds for all inputs or is qualified.

Evidence:

    65	    /// A deterministic test leaf: the version scalar folds `value` and
    66	    /// `ticks` together so distinct cases produce distinct versions — the
    67	    /// axis paths derive from — while `value` also picks the payload.
    68	    fn new(value: u64, ticks: u8) -> Self {
    69	        Self {
    70	            value,
    71	            version: Version::try_from(value.wrapping_shl(8) | u64::from(ticks))
    72	                .expect("every u64 scalar is a valid linear version"),

Resolution: write `value << 8` and qualify the doc ("distinct in the low 56 bits of `value` or in `ticks`"), or use an injective fold if distinctness is ever relied upon. Acceptance: the doc's claim is true of the fold as written.

### remote-adapter-tests-4: backend_errors.rs claims every reachable backend operation, but `Leaf::leaf` failure is not injectable
- Where: src/tree/mirror/streaming/remote/adapter/tests/backend_errors.rs:1-1 (related: src/tree/mirror/streaming/remote/adapter/tests/backend_errors.rs:203-205, src/tree/mirror/streaming/testing/failing.rs:26-33, src/tree/mirror/streaming/testing/failing.rs:185-195, src/tree/mirror/streaming/remote/adapter/decode.rs:168-170, src/tree/mirror/streaming/remote/adapter/decode.rs:375-377)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read failing.rs: `Operation` has only `Children` and `Parent`; `FailingNode::leaf` maps `Failure::Inner` and never injects; decode.rs maps `Leaf::leaf` errors to `DecodeError::Backend` at two sites)
- Seen by: structure-prose, blind-spots; refutation: confirmed; history: deliberate but expired (true at 07504b2e; 8aeed2dd made `Leaf::leaf` fallible and kept `Failing` out of construction without narrowing this doc)
- Owner-gated: no

The adapter reaches a third fallible backend operation on the decode side, leaf construction, where payload custody passes to the backend; `Failing` cannot fail there, so the atomicity claims (no partial reply, no later call, sentinel untouched) at that moment are unexercised. The test's own doc at 203-205 correctly says "explode/assemble"; the module doc claims more than the instrument can inject.

Evidence:

    1	//! Source-error propagation across every backend operation reachable by the adapter.

    185	    // Custody passes straight through: fault injection targets the
    186	    // traversal operations, not construction.

Resolution: either add an `Operation::Leaf` injection point to `Failing` (in-crate test infrastructure) and a decode row that fails at the k-th record of a multi-record run, asserting the typed error, the history, the untouched sentinel, and via the census that no node from the failing record took custody; or narrow line 1 to "every backend traversal operation" and, at failing.rs:185, state why construction is exempt (the comment currently gives no reason beyond itself). Acceptance: a committed test drives `decode_reply` to `DecodeError::Backend(Failure::Injected(Operation::Leaf))`, or the module doc no longer claims every reachable operation.

### remote-adapter-tests-5: import layout is codemod residue across the suite
- Where: src/tree/mirror/streaming/remote/adapter/tests/backend_errors.rs:3-26 (related: src/tree/mirror/streaming/remote/adapter/tests/opening.rs:12-37, src/tree/mirror/streaming/remote/adapter/tests/opening.rs:288, src/tree/mirror/streaming/remote/adapter/tests/fan_occupancy.rs:80, src/tree/mirror/streaming/remote/adapter/tests/fan_occupancy.rs:133, src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:611-612, src/tree/mirror/streaming/remote/adapter/tests/opening.rs:188)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (the first `use` line of all seven files is `use crate::message::{PayloadCodec, PayloadDepthLimit};`; `<Local as Backend>::` appears at 23 sites: opening 10, malformed 5, properties 4, parking 2, runs 2)
- Seen by: structure-prose, api-economics; refutation: confirmed (with the correction that child modules need their own `SupplyLedger` import, so the fix is a file-top import); history: codemod residue (4356e197 inserted the codec import at the head of each import block; d8bef16b inserted `fn erased` above opening.rs's pre-existing `use super::` block)
- Owner-gated: no

Every file opens with the codec import ahead of `std` and external crates, with the remaining `crate::` imports split across two or three later groups; opening.rs defines `fn erased` between two `use` blocks; fan_occupancy.rs calls `super::runtime()` qualified while importing `unbounded`; opening.rs:288 spells `super::super::ScopeError::UnpositionedMatch` inline; malformed.rs:611 and opening.rs:188 do function-local `use ... SupplyLedger`. `<Local as Backend>::erase(..)` and `::assume::<H>(..)` are written in UFCS where `Local::erase(..)` resolves with `Backend` in scope (only the associated type `<Local as Backend>::Erased` needs the qualified form).

Evidence:

    3	use crate::message::{PayloadCodec, PayloadDepthLimit};
    4	use std::convert::Infallible;
    5	
    6	use futures::{StreamExt, stream};
    7	
    8	use crate::tree::{
    ...
    19	use super::{
    20	    super::{DecodeError, EncodeError, Scope, decode_reply, encode_reply},
    21	    LeafCase, hash, leaf_run, runtime, unbounded,
    22	};
    23	use crate::tree::mirror::streaming::{
    24	    convert::Convert,
    25	    remote::codec::{End, Flow, Frame, Reaction as WireReaction, RunBudget},
    26	};

Resolution: merge each file's `crate::` imports into one tree in std / external / crate / super order; move opening.rs's `erased` below the imports; import `runtime`, `ScopeError`, and `SupplyLedger` at file top; prefer `Local::erase(..)` and `Local::assume::<H>(..)` where the trait is imported. Acceptance: each file has one `use crate::{...}` tree; no item precedes a `use`; no `super::super::` path or function-local `use` where a top-level import serves.

### remote-adapter-tests-6: the injected-failure matrix's second ordering is dead for the committed fixture, and its atomicity assertion is fixture-specific
- Where: src/tree/mirror/streaming/remote/adapter/tests/backend_errors.rs:61-107 (related: src/tree/mirror/streaming/remote/adapter/tests/backend_errors.rs:208, src/tree/mirror/streaming/remote/adapter/encode.rs:158-245)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (re-ran the refutation pass's offline model: the skyline leaf literal for `(0xfeed_face << 8) | 7` hashed with untagged SHA3-256, as `PathHash::of` does at hash.rs:257-258, gives path e1891ba5…932e with no 0xff byte at indices 0..=30; the model reproduces `before`'s committed `0xE0` vector for the empty version at span/tests.rs:471)
- Seen by: blind-spots (the over-strong assertion); refutation: raised the dead arm as new and confirmed the assertion reading; history: no rationale found (fixture and arms from 07504b2e, no message body)
- Owner-gated: no

The test selects the reaction order by whether the supply radix is 255, but for `LeafCase::new(0xfeed_face, 7)` no byte of the path at any of the 31 exercised heights is 0xff, so the `[Query, Supply]` arm never executes and the atomicity assertion is only ever exercised with the failing supply first. Separately, `yielded.is_empty()` is stronger than the contract for replies of three or more reactions (the encoder holds only the last frame pending, encode.rs:158, so a frame before the failing reaction is legitimately yielded); the two-reaction fixture cannot distinguish the doc's claim ("no frame after the failure") from "no frame at all". The branch's presence suggests coverage it does not deliver.

Evidence:

    63	        let positional_radix = if supply_radix < u8::MAX {
    64	            supply_radix + 1
    65	        } else {
    66	            supply_radix - 1
    67	        };
    ...
    73	            let replies = if supply_radix < u8::MAX {
    74	                vec![
    75	                    Reaction::Supply(supply_radix, supply),
    76	                    Reaction::Query(Vec::new()),
    77	                ]
    78	            } else {
    79	                vec![
    80	                    Reaction::Query(Vec::new()),
    81	                    Reaction::Supply(supply_radix, supply),
    82	                ]
    83	            };
    ...
    102	            assert!(
    103	                yielded.is_empty(),
    104	                "height {} failure {fail_after} published a frame or question",
    105	                Self::HEIGHT,
    106	            );

Resolution: drive both orderings explicitly (a row per ordering, or a second fixture value whose path carries a 0xff byte at some height) rather than selecting by the fixture's radix; add a three-reaction row (for example `[Match, Match, Supply(failing)]` under a two-child listing) asserting that exactly the frames before the pending one were yielded with `Flow::Continue`, the pending frame was withheld, and the stream ended; or reword the doc to "withholds the pending frame". Acceptance: both orderings execute for every height (a counter or a per-ordering row makes that visible); the new row fails if `render` yields its pending frame after a source error.

### remote-adapter-tests-7: bare `assert!(matches!(..))` rejections lose the received error on failure
- Where: src/tree/mirror/streaming/remote/adapter/tests/backend_errors.rs:164-176 (related: src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:56, 79, 113-116, 133-136, 164-167, 184-187, 303, 718; src/tree/mirror/streaming/remote/adapter/tests/opening.rs:264, 286-289)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (read every listed site; the same files use the let-else diagnostic form at malformed.rs:429-431, 471-473, 510-512, 562-568 and a `{error:?}` message at opening.rs:218-221 and malformed.rs:666-669)
- Seen by: structure-prose; refutation: confirmed; history: generational drift (bare forms from 00b29d32/07504b2e, diagnostic forms from later commits)
- Owner-gated: no

A failing typed-rejection check prints only the stringified pattern, not the error received, so the maintainer's first step is to add the diagnostic the test should have carried. The idiom is inconsistent within the same files.

Evidence:

    164	fn assert_encode_failure(error: EncodeError<Failure<Infallible>>, expected: Operation) {
    165	    assert!(matches!(
    166	        error,
    167	        EncodeError::Backend(Failure::Injected(actual)) if actual == expected
    168	    ));
    169	}

Resolution: adopt the let-else-with-diagnostic form, or `assert!(matches!(..), "unexpected error: {error:?}")`, at the listed sites. Acceptance: every rejection assertion in the partition reports the received error on failure.

### remote-adapter-tests-8: dialect tells in test prose
- Where: src/tree/mirror/streaming/remote/adapter/tests/fan_occupancy.rs:12-16 (related: src/tree/mirror/streaming/remote/adapter/tests/fan_occupancy.rs:105-108, 149; src/tree/mirror/streaming/remote/adapter/tests/parking.rs:3, 58, 85; src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:607)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -nE '\breal\b|silently|provably|RAM-sound'` over the partition returns exactly the listed sites plus malformed.rs:440, where the mechanism is stated in the same clause)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: the phrases are carried verbatim from the commit messages of the day (d6537a7b, 08f2899b)
- Owner-gated: no

"real" as a quality marker ("the priced regime is real", "supplies real groups", "The real root fan"), "silently" without the mechanism (fan_occupancy.rs:107, parking.rs:58), "provably" where the test asserts (malformed.rs:607), and "RAM-sound" (parking.rs:3) are the default-dialect residue the writing-style rules name: describe the property that holds, and pair "silently" with the mechanism.

Evidence:

    12	//! timing anywhere: an eager frame source *reaches* the `FAN + 1`
    13	//! ceiling on each path (the priced regime is real, so the pins cannot
    14	//! pass vacuously) and never exceeds it, and a paced source is the
    15	//! negative control proving the probe reports the regime rather than a
    16	//! constant.

    105	/// `SUPPLY_DECODE_ENVELOPE_BYTES` prices is real. Not exceeding it is
    106	/// the charge premise itself, so a widened channel or a new buffer
    107	/// stage on this path fails here instead of silently underpricing every
    108	/// session budget.

Resolution: rewrite to the property ("the ceiling is reached, so the pin is not vacuous"; "fails here rather than underpricing every session budget with no test going red"; "the fixture's root fan"; "so custody stops at the charge"; "memory-bounded"). Acceptance: none of "silently", "real", "provably", "RAM-sound" remain in the partition's prose except where the mechanism is stated in the same clause.

### remote-adapter-tests-9: `FAN + 1` is spelled independently at five sites and only one pair is mechanically bound
- Where: src/tree/mirror/streaming/remote/adapter/tests/fan_occupancy.rs:113-115 (related: src/tree/mirror/streaming/remote/adapter/tests/fan_occupancy.rs:152-154, src/tree/mirror/streaming/window.rs:191-192, src/tree/mirror/streaming/window.rs:397-398, src/tree/mirror/streaming/window/tests.rs:240-241)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (grep of `FAN + 1` and `FAN as u128 + 1` over src/)
- Seen by: blind-spots; refutation: reframed (window/tests.rs:239-247 does bind the constant to a recomputed flat term, so a constant-only factor change fails there; what remains textual is the binding between this occupancy pin, the constant, and the solve's own spelling); history: no rationale found (`SUPPLY_DECODE_ENVELOPE_BYTES` is gated `#[cfg(any(test, feature = "test-internals"))]`, so a shared factor must live outside that gate or be gated the same way)
- Owner-gated: no

The pins prove the code's occupancy is `FAN + 1`; they do not connect that figure to the constant they say they underwrite. A named records-per-stream constant used by the envelope, the solve, the window test, and these pins would make the agreement mechanical rather than textual.

Evidence:

    113	    assert_eq!(
    114	        peak,
    115	        FAN + 1,

    191	pub(crate) const SUPPLY_DECODE_ENVELOPE_BYTES: usize =
    192	    STREAM_COUNT * (FAN + 1) * (std::mem::size_of::<typed::Node<Z>>() + FAN_SLOT_BYTES);

Resolution: introduce one `pub(crate)` records-per-stream constant in window.rs (outside the test gate), use it in `SUPPLY_DECODE_ENVELOPE_BYTES`, in the solve at window.rs:397-399, in window/tests.rs:240-242, and assert against it in both eager pins here. Acceptance: `FAN + 1` as an arithmetic expression appears once in the crate.

### remote-adapter-tests-10: fixture and helper machinery is duplicated across the partition
- Where: src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:321-335 (related: src/tree/mirror/streaming/remote/adapter/tests/runs.rs:53-84; src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:445-448, 576-582; src/tree/mirror/streaming/remote/adapter/tests/properties.rs:965-975; src/tree/mirror/streaming/remote/adapter/tests/opening.rs:123-124, 137-151, 190-191; src/tree/mirror/streaming/remote/adapter/tests/fan_occupancy.rs:43-75; src/tree/mirror/streaming/remote/adapter/tests/runs.rs:87-104; src/tree/mirror/streaming/remote/adapter/tests/properties.rs:727-739, 863-872; src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:584-595; src/tree/mirror/streaming/remote/adapter/tests/properties.rs:61-62, 98-101, 363-366; src/tree/mirror/streaming/remote/adapter/tests/backend_errors.rs:28-48, 178-201; src/tree/mirror/streaming/remote/adapter/tests/properties.rs:1003-1030; src/tree/mirror/streaming/remote/adapter/tests/properties.rs:834-840; src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:369-377; src/tree/mirror/streaming/remote/adapter/tests/runs.rs:284-294)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read every cited site; `grep -c 'a test record fits the run framing'` finds the expect string defined at tests.rs:46 and again at fan_occupancy.rs:65)
- Seen by: structure-prose, api-economics; refutation: confirmed (with the correction that the "ascending leaves" family spans three different fixture shapes, so the shared helper needs a parameter rather than being a pure hoist); history: pure accretion (each file's helpers landed with the commit that created the file; `OpeningNode::node` at opening.rs:43-56 builds a fixed `Version::new()`/`Message::new(())` leaf and is a different shape from the two `LeafCase`-derived node traits)
- Owner-gated: no

The same machinery is written several times with small variations: (a) `under_root_pair()` is `colliding_leaves(2)` with a tuple return consumed as `.0/.1/.2` at malformed.rs:343, 347, 369, 409-410, 432-433, 694, 699; (b) "scan versions until a path predicate holds" appears as runs.rs `separated_leaves`, malformed.rs:445-448, and properties.rs `foreign_parent`; (c) "n ascending leaves" appears as malformed.rs `ascending_leaves`, fan_occupancy.rs `leaves` (raw versions), and inline at opening.rs:123-124 and 190-191 (the last duplicated within its own file), each choosing `ticks` differently (`value as u8 % 4`, `0`, `1`); (d) fan_occupancy.rs:62-66 re-implements `leaf_run` with the identical expect string; (e) "last frame gets `Flow::End`" tail-flagging is spelled six times; (f) `AdapterHeight::node` and `BackendHeight::node` are the same one-leaf-subtree builder; (g) the `dispatch_height!`/`at_height!` recursive macro pair is defined twice here (and a third time in `materialized/work/tests/violations.rs`) where a `seq_macro::seq!` over a range would express each in six lines; (h) the `<Local as Backend>::leaves(Local, prefix, assume::<H>(node.clone()))` rebuild appears three times. Each duplicate is a place a fixture change must be repeated, and the tuple-indexed variant is less legible than the `LeafCase` it wraps.

Evidence:

    321	fn under_root_pair() -> [(Version, Message, Path); 2] {
    322	    let mut by_radix: BTreeMap<u8, Vec<(Version, Message, Path)>> = BTreeMap::new();
    323	    for value in 0..u64::MAX {
    324	        let leaf = LeafCase::new(value, value as u8 % 4);
    325	        let path = leaf.path();
    326	        let bytes: [u8; 32] = path.into();
    327	        let group = by_radix.entry(bytes[0]).or_default();
    328	        group.push((leaf.version, leaf.message, path));
    329	        if group.len() == 2 {

    53	fn colliding_leaves(count: usize) -> Vec<LeafCase> {
    54	    let mut by_radix: BTreeMap<u8, Vec<LeafCase>> = BTreeMap::new();
    55	    for value in 0..u64::MAX {
    56	        let leaf = LeafCase::new(value, value as u8 % 4);

Resolution: hoist into tests.rs: `colliding_leaves(count)`, a `leaf_outside::<H>(anchor)` search, an `ascending_leaves(count, ticks)` builder, a `reply_frames(Vec<WireReaction>) -> Vec<Frame>` that owns tail-flagging, one `NodeAt: Height { fn node(&LeafCase) -> Node<Self> }` trait for both `LeafCase`-derived ladders, one `seq!`-based dispatch macro parameterized by range (the production `at_height!` in erased.rs spans 0..=32 and cannot be reused directly because `AdapterHeight` is not implemented at H32 and `FailureHeight` not at Z or H32), and a `leaves_of::<H>(node, prefix)` rebuild. Replace `under_root_pair` with `colliding_leaves(2)` and index fields by name; have fan_occupancy's `frames` call `leaf_run`. Acceptance: each helper has one definition in the partition; malformed.rs contains no `.0/.1/.2` access on leaf fixtures; the expect string appears once; `grep -rn 'macro_rules! dispatch_height' src` is empty; the partition's line count drops with no test removed.

### remote-adapter-tests-11: a positive round-trip law lives in malformed.rs and duplicates a weaker test in runs.rs
- Where: src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:337-396 (related: src/tree/mirror/streaming/remote/adapter/tests/runs.rs:257-296, src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:1, src/tree/mirror/streaming/remote/adapter/tests/runs.rs:152-217)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (read both bodies; `git log -S` places the malformed test at 00b29d32 and the runs.rs test at f94f2056, which also rewrote the malformed test's closing assertion to the batched form)
- Seen by: structure-prose, api-economics (proposing opposite deletions); refutation: confirmed (the malformed test is the stronger one: frame byte-equality at :395 implies runs.rs's `frames.len() == 1` at :264; both point tests are already implied by the runs.rs proptest apart from the default-budget instantiation); history: no rationale found (f94f2056 kept both without saying why)
- Owner-gated: no

`a_multi_leaf_run_is_one_supplied_subtree` decodes two unbatched frames to one node, rebuilds two leaves, and asserts the re-encoded frame list equals one batched run; runs.rs `a_batched_run_round_trips_the_reply` does the first two steps for four leaves and stops. Neither is a malformed-wire case, contradicting malformed.rs's module doc, and the same law is maintained in two files.

Evidence:

    1	//! Focused malformed-wire cases which are not naturally height-parametric.

    337	/// Consecutive leaves in one version-derived run assemble as one node and reexplode exactly.
    338	#[test]
    339	fn a_multi_leaf_run_is_one_supplied_subtree() {
    340	    let leaves = under_root_pair();

    257	/// The re-encoded reply of a multi-leaf reaction under the default budget
    258	/// is one batched frame whose decode reproduces the protocol reply
    259	/// exactly: the round trip is lossless through the batched form.
    260	#[test]
    261	fn a_batched_run_round_trips_the_reply() {

Resolution: keep one multi-leaf round-trip witness in runs.rs (the batching contract's module) carrying the stronger assertion (re-encoded frames equal one `leaf_run` of all leaves with `Flow::End`), delete the other, and retire `under_root_pair` in favor of the hoisted `colliding_leaves`. Either the version-bound and set-length tests' admitting halves move beside the laws they witness, or malformed.rs's module doc widens to "ingress validation: rejections and their admitting boundaries". Acceptance: one multi-leaf round-trip test in the partition, in runs.rs, asserting the canonical batched frame bytes; malformed.rs's module doc matches its roster.

### remote-adapter-tests-12: the set-length test claims failure at the first over-record but pins only residency below 128
- Where: src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:597-683 (related: src/tree/mirror/streaming/remote/adapter/tests/opening.rs:178-199, src/tree/mirror/streaming/remote/adapter/decode.rs:367-382, src/testing.rs:51-52)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read; the refutation pass confirmed by reading that every `OverdrawnSupply` site in src/ and tests/ matches the variant only)
- Seen by: structure-prose; refutation: confirmed; history: no rationale found (08f2899b added the `small < SMALL` clause as a loose bound; the slope-then-equality design is deliberate and recorded, the exactness clause is not)
- Owner-gated: no

The testdoc says the reply "fails typed at its first over-declaration record" and "custody provably stops at the charge", but the body pins the error variant, residency equality across a doubled overrun, and `small < SMALL` (128). A decoder that admitted a hundred records before charging would pass. With an allowance of one, residency at rejection is O(1), so a tight bound is derivable. Relatedly, opening.rs:178-180 claims the rejection lands "while the one opening reply is still open", but its fixture is one `Flow::End` frame with no trailing sentinel, so openness is unobserved (backend_errors.rs:124-159 shows the sentinel technique). The test also reads the process-global `census` directly (line 612) rather than through `testing::node_census`, whose doc states the premise the equality rests on ("tests that assert on them must own the process"); that premise is not restated here.

Evidence:

    597	/// A reply streaming past the declared `set_len` fails typed at its first
    598	/// over-declaration record, under node residency independent of the
    599	/// overrun; a declaration exactly covering the stream admits it whole.
    ...
    678	    assert!(
    679	        small < SMALL as usize,
    680	        "custody stops at the charge: {small} resident handles against a \
    681	         {SMALL}-leaf stream",
    682	    );

Resolution: measure the residency at rejection once and pin it (exact, or `<= 1 + <assembly's open-parent slot>` with the derivation in a comment) so a charge delayed by k records fails; for the opening test, append a sentinel frame and assert it is unconsumed, or drop the "still open" clause; restate the process-per-test premise at the census read. Acceptance: a hand-mutation that moves `ledger.charge(1)` after `Leaf::leaf` in decode.rs, or delays it by several records, fails the test; every clause of the testdoc has an assertion behind it.
Construction: temporarily reorder decode.rs:372-377 so the charge follows the `Leaf::leaf` await and the send; run `a_reply_past_the_declared_set_len_fails_at_its_first_over_record` and observe it still passes (residency grows by at most one). Then delay the charge by four records via a counter and observe it still passes because 4 < 128.

### remote-adapter-tests-13: the `SupplyOrder` pin ignores both fields, and only one field shape is reachable
- Where: src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:718-718 (related: src/tree/mirror/streaming/remote/adapter/decode.rs:516-536)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (read decode.rs `observe`: the `LeafOrder` check at 516-527 runs before the `SupplyOrder` check at 531; once leaf paths strictly ascend under one parent, the byte at `parent_len` is non-decreasing, so `previous >= radix` fires only with `previous == radix`, the resumed-run shape this test constructs)
- Seen by: blind-spots; refutation: confirmed (and raised the production half: the `>` half of `previous >= radix` at decode.rs:531 is a dead branch a `>=` to `==` mutant would prove equivalent)
- Owner-gated: no

`matches!(error, DecodeError::SupplyOrder { .. })` pins the variant without its fields. Destructuring and asserting `previous == radix == <first path byte of leaves[0]>` would document the precedence the decoder actually has. The production observation (the variant's doc says "reused or preceded" and "preceded" appears unreachable) belongs to the decode.rs reviewer; see the open questions.

Evidence:

    718	    assert!(matches!(error, DecodeError::SupplyOrder { .. }));

    531	            if let Some(previous) = self.previous_radix.filter(|previous| *previous >= radix) {
    532	                return Err(DecodeError::SupplyOrder { previous, radix });

Resolution: destructure `SupplyOrder { previous, radix }` and assert both equal the shared leading byte. Acceptance: the assertion reads the fields.

### remote-adapter-tests-14: `early_supplies`' documented incremental yield is never observed
- Where: src/tree/mirror/streaming/remote/adapter/tests/opening.rs:153-165 (related: src/tree/mirror/streaming/remote/adapter/tests/opening.rs:229-241, src/tree/mirror/streaming/remote/adapter/tests/fan_occupancy.rs:136-146, src/tree/mirror/streaming/remote/adapter/decode.rs:57-61, src/tree/mirror/streaming/remote/proxy/work/pump.rs:493-506)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read; every happy-path test of `early_supplies` in the partition collects the whole stream before asserting)
- Seen by: blind-spots; refutation: confirmed (no other test in the crate observes the schedule; the consequence of regression is pipelining latency, not deadlock or memory); history: the contract is deliberate and stated in code (decode.rs:57-61, adapter.rs:33-35, 55d76d5c's message), the tests were written collecting from the start
- Owner-gated: no

decode.rs states the distinguishing contract of `early_supplies`: "this stream yields each assembled node as soon as its group completes". The three happy-path tests `try_collect` before asserting, so an implementation that buffered every group until the reply end passes all of them. The property is what the responder's root merge-join pipelines on, and it is the function's stated reason to exist apart from `decode_reply`.

Evidence:

    153	    let decoded: Vec<(u8, _)> = runtime()
    154	        .block_on(
    155	            early_supplies::<Local, _>(
    156	                Local,
    157	                u64::MAX,
    158	                unbounded(),
    159	                Prefix::new().erase(),
    160	                stream::iter(frames),
    161	                PayloadCodec::new::<u64>(PayloadDepthLimit::default()),
    162	            )
    163	            .try_collect(),
    164	        )
    165	        .expect("a canonical opening-supply reply decodes");

Resolution: feed frames through a `tokio::sync::mpsc` channel on the current-thread runtime: send the frames completing group A plus the first record of group B, poll the stream once, and assert A's `(radix, node)` has been yielded before any further frame is sent; then send the rest and assert B and the end. The fan_probe's determinism argument (single thread, FIFO channel) applies unchanged. Acceptance: a committed test fails when `early_supplies` is replaced by a variant that collects all groups before yielding.
Construction: in opening.rs, replace `stream::iter(frames)` with the receiving half of an `mpsc` channel; after sending group A's frames and one record of group B, `poll_next` the stream once (a `futures::poll!` on the pinned stream) and assert `Some(Ok((radix_a, _)))`; a buffered-until-end implementation returns `Pending`.

### remote-adapter-tests-15: `read_early` duplicates the frame grammar but only three of its rejection arms are pinned on the early path
- Where: src/tree/mirror/streaming/remote/adapter/tests/opening.rs:245-290 (related: src/tree/mirror/streaming/remote/adapter/decode.rs:136-200, src/tree/mirror/streaming/remote/adapter/decode.rs:306-392, src/tree/mirror/streaming/remote/adapter/decode.rs:352-355, .agent-notes/2026-08-21-cbor-wire-mutants-scopeA/03-proxy-adapter.md:63-90)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -n 'BareEndAfterReaction\|UnexpectedStreamEnd\|TruncatedReply\|UnpositionedQuery'` over opening.rs and fan_occupancy.rs is empty; read both loops side by side: `read_reply` carries the empty-run `debug_assert!` at 352-355 and `read_early`'s record loop at 155-178 does not)
- Seen by: blind-spots, api-economics; refutation: confirmed (medium stands: the recorded disposition is the missing check); history: already known as the `!any` mutant survivor with a grammar-recognizer differential family as its disposition, marked a design and not landed; the `debug_assert!` asymmetry is not covered by that note (8e1ed47a added the assert to `read_reply`; `read_early` was written five days later without it)
- Owner-gated: no

`early_supplies` reads frames through `read_early`, a second hand-written copy of the loop in `read_reply`. On the early path the suites pin `OverdrawnSupply`, `ExtraOpeningReply`, and `UnpositionedMatch` only. Each of `TruncatedReply` (decode.rs:151-152), `UnpositionedQuery` (185-187), `BareEndAfterReaction` (188-189, via the `any` flag), `UnexpectedStreamEnd` (190), `OversizedVersion`, `LeafOrder`, `SupplyOrder`, and `Record` has a distinct arm there and is tested only through `decode_reply`/`decode_leaf_reply`. When a grammar is implemented twice, a pin on one copy says nothing about the other; a fix applied to one loop (reordering the `any` check, dropping a `?`) passes the suite. The doctrine is that every hole found becomes a committed check, never a note.

Evidence:

    245	/// The opening-supply stream carries exactly one reply: frames after its
    246	/// end are rejected, not absorbed into a phantom second reply.
    247	#[test]
    248	fn second_opening_supply_reply_is_rejected() {
    249	    let frames: Vec<Frame> = vec![Frame::End(End::Reply), Frame::End(End::Reply)];

    188	            Frame::End(End::Reply) if !any => Flow::End,
    189	            Frame::End(End::Reply) => return Err(DecodeError::BareEndAfterReaction),
    190	            Frame::End(End::Stream) => return Err(DecodeError::UnexpectedStreamEnd),

Resolution: land the recognizer-differential family the note designs: generate short frame words over {Supply(Continue), Supply(End), End(Reply), End(Stream), Match, Query}, define the accepted language once (`Supply(Continue)* Supply(End)` or one bare `End(Reply)`; nothing after), and assert `early_supplies` accepts exactly it and rejects each other word with the variant the recognizer predicts; the existing point tests become witnesses of the family or are dropped. Alternatively make the malformed cases entry-point-parametric (one table run through both `decode_reply` over `Scope::opening(&[])` and `early_supplies` over the root prefix), or factor the shared record handling into one function both loops call. Add the empty-run `debug_assert!` to `read_early` or state at the site why the asymmetry is intended. Acceptance: a test in opening.rs fails when decode.rs:188's `if !any` is replaced by `true`; each of `BareEndAfterReaction`, `UnexpectedStreamEnd`, `TruncatedReply`, `UnpositionedQuery` is asserted at least once against `early_supplies`.
Construction: `early_supplies` over `[Frame::Reaction(WireReaction::Supply(one record), Flow::Continue), Frame::End(End::Reply)]` must return `Err(DecodeError::BareEndAfterReaction)`; with the `!any` guard replaced by `true` it returns `Ok` with one node, which no committed test observes.

### remote-adapter-tests-16: parking.rs misattributes the one-slot response relay to `proxy/work/queues.rs`
- Where: src/tree/mirror/streaming/remote/adapter/tests/parking.rs:5-7 (related: src/tree/mirror/streaming/remote/proxy/work/queues.rs:8-10, src/tree/mirror/streaming/remote/proxy/work.rs:147-154, src/tree/mirror/streaming/remote/proxy/work/pump.rs:19-21)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (queues.rs:8-10 says the decoded-response relay "is created by the response pump itself: see `Work::respond`"; work.rs:151-154 constructs it with capacity 1)
- Seen by: structure-prose; refutation: confirmed (the cited file does explain the edge's capacity rationale, so the citation is misleading rather than wrong); history: deliberate but expired (correct at 3a5ba643 when queues.rs held `responses()`; d8bef16b moved the constructor into `Work::respond` and updated queues.rs's header but not this pointer)
- Owner-gated: no

The module doc locates the one-slot relay in a file whose own header says it is constructed elsewhere.

Evidence:

    5	//! never a materialized subtree. The one-slot response relay
    6	//! (`proxy/work/queues.rs`) bounds decoded replies to one in flight per
    7	//! stage, and any wire backlog behind that slot sits in the transport's

    8	//! its runtime [`QueueRole`] label. (The third proxy edge, the one-slot
    9	//! decoded-response relay, is created by the response pump itself: see
    10	//! `Work::respond`.)

Resolution: cite `Work::respond` (proxy/work.rs). Acceptance: the cited location constructs the relay.

### remote-adapter-tests-17: parking.rs module doc states megabyte figures that contradict its own pin and `message.rs`
- Where: src/tree/mirror/streaming/remote/adapter/tests/parking.rs:13-20 (related: src/tree/mirror/streaming/remote/adapter/tests/parking.rs:51-59, src/tree/mirror/streaming/message.rs:14-17)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git log -L13,20` on parking.rs shows only 3a5ba643 and c20b9cf4 touched the figure lines; `git show 2d1e6ea5 -- parking.rs` and `git show 4dd2053c -- parking.rs` each change only the constant line, 2_300_000 to 3_380_000 to 3_570_000, while message.rs:15-16 was re-derived at both to ≈ 1.8 MB / ≈ 3.5 MB; `size_of::<(u8, Hash)>()` with `MERKLE_HASH_LEN = 24` is 25, so the decoded half alone is 65,536 × 25 = 1,638,400 bytes, above the prose's 1.1 MB)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no rationale found; the owner ruling recorded in 2d1e6ea5's message ("Prose figures that were hand-synced to measurements are excised rather than re-synced where they are not load-bearing ... cite the pinned constant or recomputation instead") prescribes the shape of the fix
- Owner-gated: no

The module doc says the disputed-reply skeleton is "≈ 1.1 MB encoded, ≈ 2.2 MB while the encoded and decoded forms coexist"; the pin beside it is 3,570,000 with about 3% headroom, and `message.rs`, which this doc says states the same figure, reads ≈ 1.8 MB / ≈ 3.5 MB. The constant's own comment names preventing exactly this staleness as its purpose; the figure went stale twice anyway because only the constant is enforced. Three sites disagree and a re-baseliner cannot tell which is authoritative.

Evidence:

    13	//! node, and a maximally disputed reply retains a pure skeleton of at most
    14	//! fan² `(radix, hash)` entries: ≈ 1.1 MB encoded, ≈ 2.2 MB while the
    15	//! encoded and decoded forms coexist mid-decode. That coexistence
    16	//! transient is the figure the session's memory model charges per parked
    17	//! reply (`streaming/message.rs`), and it is pinned here as a sum of the

    55	/// Chosen tight so growth in either half — a wider hash, a larger fan,
    56	/// heavier framing — fails the pin and forces the module doc's charged
    57	/// figure (and `streaming/message.rs`, which states it) to be
    58	/// re-derived rather than silently going stale.
    59	const DISPUTED_REPLY_TRANSIENT_CEILING: usize = 3_570_000;

    15	//! reactions × a 256-entry listing ≈ fan² hashes ≈ 1.8 MB encoded
    16	//! (≈ 3.5 MB while an encoded and a decoded copy coexist), transient, at

Resolution: apply the standing ruling: excise the megabyte figures from the module doc and cite the mechanism and the enforced constant by name ("the encoded reply plus the decoded fan² skeleton, pinned by `DISPUTED_REPLY_TRANSIENT_CEILING`"); have message.rs name the same constant so there is one number of record, or give the crate one visible constant both cite. Acceptance: `grep -n MB` over parking.rs returns no figure disagreeing with the pin; the module doc, the constant doc, and message.rs:14-17 agree or defer to the named constant.

### remote-adapter-tests-18: a runtime size assertion is called "compiler-checked", em-dashes sit in `//` comments, and the pin's readout is unexplained
- Where: src/tree/mirror/streaming/remote/adapter/tests/parking.rs:66-74 (related: src/tree/mirror/streaming/remote/adapter/tests/parking.rs:136, 198-201; src/tree/mirror/streaming/backend/local.rs:108-110)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -nE '^\s*//[^/!].*—'` over the partition returns exactly parking.rs:68 and :136; local.rs:110 is the compile-time form `const _: () = assert!(..)` of the same claim)
- Seen by: structure-prose; refutation: reframed (the `eprintln!` at 198-201 is the readout both re-pin commits quote, c20b9cf4's "1,114,624 B" and 4dd2053c's "measured 3,468,800", which a passing run exposes only through this print, so the fix is a comment naming that purpose, not deletion; `Stream::new(11)` is justified by the comment directly above it and `Stream::COUNT = 17` keeps every label a one-byte CBOR head); history: "compiler-checked" and the em-dashes are from 3a5ba643 and were inaccurate from the start
- Owner-gated: no

The comment says the handle claim is compiler-checked; the check beneath it is a runtime `assert_eq!` on `mem::size_of`, and the compile-time form already exists at local.rs:110. Two `//` comments use em-dashes where the register rule is spaced double hyphens. The `eprintln!` serves the re-baselining procedure and says nothing about that at the site.

Evidence:

    66	    // The handle claim, compiler-checked: a backend node reference is one
    67	    // shared pointer (an `Arc` bump to clone), so parking a reply costs
    68	    // `replies.len()` pointers plus the skeleton — the subtree's bytes live
    69	    // in backend custody behind the handle.
    70	    assert_eq!(
    71	        mem::size_of::<typed::Node<UnderRoot>>(),
    72	        mem::size_of::<usize>(),
    73	        "a parked supply must be a shared handle, not an owned subtree",
    74	    );

Resolution: either make the size check `const _: () = assert!(..)` (or cite the one at local.rs:110) so "compiler-checked" is true, or reword to "pinned"; replace the two em-dashes with ` -- `; add one line above the `eprintln!` stating that it is the measurement the pin constant is re-derived from on a format change. Acceptance: the grep for em-dashes in `//` comments over the partition is empty; the size assertion's comment matches its mechanism; the readout's purpose is stated at the site.

### remote-adapter-tests-19: `ErasedUnit` and `ErasedU64` alias the same type; the per-payload distinction is gone
- Where: src/tree/mirror/streaming/remote/adapter/tests/properties.rs:21-23 (related: src/tree/mirror/streaming/remote/adapter/tests/properties.rs:142, 186, 407, 454, 684, 763, 808, 827, 886, 920; src/tree/mirror/streaming/backend/local.rs:116)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git log -L21,23`: d8bef16b introduced `<Local as Backend<()>>::Erased` and `<Local as Backend<u64>>::Erased`, distinct under the then payload-generic `Backend<T>`; 48bc31df rewrote both right-hand sides to `<Local as Backend>::Erased` and kept the names and the "per payload" sentence)
- Seen by: structure-prose, api-economics; refutation: reframed (the structure-prose seed's premise that the distinction "has never existed in the type system" is wrong; it existed until the payload erasure); history: deliberate but expired
- Owner-gated: no

Both aliases expand to `typed::untyped::Node`. The names and the comment describe a distinction payload erasure removed, and the helper signatures below still choose one or the other (`Reply<ErasedUnit>` at 142, 407, 886, 920; `Reply<ErasedU64>` at 684, 763, 808, 827), suggesting a typing constraint that no longer exists. A reader must check the definitions to learn the two are interchangeable.

Evidence:

    21	/// The in-memory backend's erased node representations, per payload.
    22	type ErasedUnit = <Local as Backend>::Erased;
    23	type ErasedU64 = <Local as Backend>::Erased;

Resolution: collapse to one alias (`type Erased = <Local as Backend>::Erased;`) or use the path directly, and drop the "per payload" sentence. Acceptance: no `ErasedUnit`/`ErasedU64` identifiers remain in properties.rs.

### remote-adapter-tests-20: properties.rs states each of its six laws twice, once for `Z` and once for `S<H>`
- Where: src/tree/mirror/streaming/remote/adapter/tests/properties.rs:98-658 (related: src/tree/mirror/streaming/remote/adapter/decode.rs:203-259, src/tree/mirror/streaming/remote/adapter/encode.rs:80-142, src/tree/mirror/streaming/remote/adapter.rs:22-26)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (side-by-side read of 103-133 vs 368-398, 135-175 vs 400-445, 177-251 vs 447-527, 253-313 vs 529-610, 315-333 vs 612-630, 335-354 vs 632-657)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale found (both impls from 00b29d32/07504b2e, no message bodies; the production split's rationale is stated at adapter.rs:22-26, the test-side duplication's is not; backend_errors.rs implements `FailureHeight` for `S<H>` only because leaf height has nothing to inject into, so it never faced this choice)
- Owner-gated: no

The differences between the `Z` impl (98-355) and the `S<H>` impl (357-658) are exactly: `decode_leaf_reply`/`encode_leaf_reply` vs `decode_reply`/`encode_reply`; `Scope::leaf(prefix)` vs `Scope::new(prefix, &nested)`; `leaf_case.nested.clear()` at 185 (leaf queries carry no listing); the literal `"height 0"` labels (`Z::HEIGHT` is 0, so `"height {}", Self::HEIGHT` serves both); and `Prefix::<S<Z>>` vs `Prefix::<S<S<H>>>`, both spellable as `Prefix::<S<Self>>`. Everything else is textually identical. A law stated twice can drift into two laws, and the reviewer must diff 560 lines to confirm the leaf case is the general case plus one exception.

Evidence:

    103	    fn supplied_leaf_is_lossless(
    104	        leaf: &LeafCase,
    105	        runtime: &tokio::runtime::Runtime,
    106	    ) -> TestCaseResult {
    107	        let (parent, radix) = Prefix::<Z>::containing(&leaf.path()).pop();
    108	        let scope = Scope::new(parent.erase(), &[]);
    109	        let frame = supplied_frame(leaf, Flow::End);
    110	        let mut frames = stream::iter([frame.clone()]);
    111	        let decoded = runtime
    112	            .block_on(decode_leaf_reply(

    368	    fn supplied_leaf_is_lossless(
    369	        leaf: &LeafCase,
    370	        runtime: &tokio::runtime::Runtime,
    371	    ) -> TestCaseResult {
    372	        let (parent, radix) = Prefix::<S<H>>::containing(&leaf.path()).pop();
    373	        let scope = Scope::new(parent.erase(), &[]);
    374	        let frame = supplied_frame(leaf, Flow::End);
    375	        let mut frames = stream::iter([frame.clone()]);
    376	        let decoded = runtime
    377	            .block_on(decode_reply::<Local, _>(

Resolution: move the varying operations onto `AdapterHeight`: `decode(..)` and `encode(..)` selecting the entry, `question(parent, radix, nested) -> Scope` (`Z`: `Scope::leaf`; `S<H>`: `Scope::new`), and `nested(case) -> &[(u8, Hash)]` (empty at `Z`). Implement those once per impl and write each of the six laws once as a free generic function over `H: AdapterHeight` (or default trait methods), labelling with `"height {}", H::HEIGHT` throughout. Acceptance: each law body appears once in properties.rs; the `Z` and `S<H>` impls contain only node construction and entry/question selection; the six proptests and their doc comments are unchanged; the file is roughly half its current length.

### remote-adapter-tests-21: two of the six height sweeps are strict special cases of two others
- Where: src/tree/mirror/streaming/remote/adapter/tests/properties.rs:1033-1062 (related: src/tree/mirror/streaming/remote/adapter/tests/properties.rs:1064-1119, 154-173, 419-443)
- Class / severity / confidence: performance / nit / medium
- Provenance: assessed (read; the runtime share is not measured, and no timing run was made)
- Seen by: api-economics; refutation: confirmed (the `btree_set(.., 0..=8)` generator produces the empty case at a useful rate, so the degenerate sweeps add mostly naming value); history: the specific sweeps came first (00b29d32) and the general ones were added later (07504b2e) without retiring them; 24b187a8 added the sentinel to `matches` and `mixed` and not to `positioned`
- Owner-gated: no

`supplied_leaf_is_lossless_at_every_height` is `mixed_reactions_are_lossless_at_every_height` with `radixes` empty; `matches_and_boundaries_are_lossless_at_every_height` is `positional_reactions_are_lossless_at_every_height` with `queries == 0`, except that only the former checks the trailing sentinel. Each sweep is 256 cases × 32 heights of encode plus decode with a fresh channel and boxed async stream. The degenerate shapes do have naming value as seeds, so this is a judgment call.

Evidence:

    1033	    /// For every reply height, assembling one supplied wire leaf and then
    1034	    /// exploding the resulting backend node reproduces the exact frame.
    1035	    #[test]
    1036	    fn supplied_leaf_is_lossless_at_every_height(
    1037	        value in any::<u64>(),
    1038	        ticks in any::<u8>(),
    1039	    ) {

Resolution: fold the sentinel check into `positioned_reactions` and drop the two special-case sweeps, or keep them at a documented reduced case count since their generators vary only the leaf. Acceptance: four sweeps at full case count, or six with the two degenerate ones at a documented reduced count; the file's runtime measured once before and after on a quiet machine.

### remote-adapter-tests-22: `MAX_RECORD_LEN`'s derivation describes the retired framing, its stated envelope is false for the committed fixture, and nothing enforces it
- Where: src/tree/mirror/streaming/remote/adapter/tests/runs.rs:42-49 (related: src/tree/mirror/streaming/remote/adapter/tests/runs.rs:53-67, 162-165; src/tree/mirror/streaming/remote/codec/frame.rs:33, 36, 155-169; src/tree/mirror/cbor.rs:80-88; crates/before/src/codec/gamma.rs:25-45; crates/before/src/version/skyline/literal.rs:18-23)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (re-ran the refutation pass's offline model of the framing from the read code: `RECORD_TAG_LEN = head_len(63) = 2`, body head 1, `VERSION_TAG_LEN = head_len(0xD256) = 3`, version head 1, skyline leaf literal as topology bit plus Elias-gamma plus the `1 0*` marker, ciborium shortest-form uint payload, untagged SHA3-256 path; the model reproduces `before`'s committed `0xE0` vector for the empty version. `colliding_leaves(10)` selects values 201, 328, 415, 422, 429, 511, 839, 1076, 1077, 1106 with record lengths [14, 15, 15, 15, 15, 15, 15, 15, 15, 15], sum 149; `colliding_leaves(4)` peaks at 14. `git log -L42,49` shows the doc lines from f94f2056 (2026-07-18) untouched since; borsh was retired at f2b74a97 (2026-08-18) and the record re-spelled again at 4dd2053c, neither touching runs.rs)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed and aligned to medium; history: deliberate but expired (exact for the framing f94f2056 introduced)
- Owner-gated: no

The comment derives 14 as "a 4-byte header, a version of at most two bytes, and the fixed 8-byte payload"; under the current codec the per-record framing is 7 bytes, a leaf version of the fixture's scalars (around 2^15..2^18) is 5 bytes, and the payload is 1-3 CBOR bytes. Nine of the ten records in the largest committed case are 15 bytes, so the constant's own contract ("Upper envelope on one encoded `u64` leaf record here") is breached by the fixture. `MAX_CASE_BUDGET = 11 × 14 + SUPPLY_FRAME_OVERHEAD` exceeds the count-10 run (149 + overhead) by five budget values, so the doc's promised coverage of "run-swallowing budgets" survives at the maximum case on a five-byte accident that nothing committed protects. The laws hold at every budget, so this is silent coverage narrowing, not a false pass; the ghost derivation is the same defect the repo's hard rule on prose names.

Evidence:

    42	/// Upper envelope on one encoded `u64` leaf record here: a 4-byte header,
    43	/// a version of at most two bytes, and the fixed 8-byte payload.
    44	const MAX_RECORD_LEN: usize = 14;
    45	
    46	/// Exclusive bound on generated byte budgets: past every record and past a
    47	/// whole case's run with its frame envelope, so the sweep covers zero,
    48	/// sub-record, mid-run, and run-swallowing budgets.
    49	const MAX_CASE_BUDGET: usize = (MAX_CASE_LEAVES + 1) * MAX_RECORD_LEN + SUPPLY_FRAME_OVERHEAD;

    155	    pub fn record_len(version: &Version, message: &Message) -> usize {
    156	        let body = Self::record_body_len(version, message);
    157	        RECORD_TAG_LEN
    158	            .saturating_add(cbor::head_len(body as u64))
    159	            .saturating_add(body)

Resolution: derive the sweep's upper bound from the fixture rather than prose: compute it as `SUPPLY_FRAME_OVERHEAD + colliding_leaves(MAX_CASE_LEAVES).iter().map(|l| LeafRun::record_len(&l.version, &l.message)).sum::<usize>() + 1` (via `prop_flat_map` on `count`, or a `LazyLock`), or keep the constant and `prop_assert!(LeafRun::record_len(&leaf.version, &leaf.message) <= MAX_RECORD_LEN)` for every generated leaf as the envelope's liveness floor. Rewrite the doc in present-tense codec terms (record tag and body head, version tag and head, canonical version bytes, CBOR payload) or cite `LeafRun::record_len`. Add one witness that the maximum generated budget yields a single frame for the largest case. Acceptance: a committed assertion ties the sweep's upper bound to the actual record lengths of the generated leaves (lowering `MAX_RECORD_LEN` to 13 fails the property); the constant's doc names no borsh-era widths; a test shows the ten-leaf case encodes as one frame at the top budget.

## Positives

- `leaf_query_matrix_is_exhaustive` (malformed.rs:190-283) enumerates the 2×2×2 cells and asserts `checked == 8`, so the doc's "all eight" is a count the test enforces rather than maintains by hand; a shrunk loop domain fails it.
- `an_unpositioned_match_is_rejected_in_both_directions` (malformed.rs:82-137) is built so that a late, whole-reply rejection would surface as `TruncatedReply` while an eager one yields `UnpositionedMatch`, both frames being `Flow::Continue`; the error type alone distinguishes the property under test.
- The sentinel-frame technique (properties.rs:154-173, 289-311, 419-443; backend_errors.rs:124-159) proves a decode consumes nothing of the following reply, including after an injected failure; this is the property the pump's positional pairing rests on.
- fan_occupancy.rs pairs each `FAN + 1` equality pin with a paced negative control (161-183) and a non-emptiness check, on a thread-local probe whose current-thread FIFO premise is written where the probe lives (decode.rs:550-560); the pins cannot pass vacuously and no timing is involved.
- backend_errors.rs asserts the `Failing` history equals exactly the operations up to and including the injected one, at every height and every fail-after index in both directions, so post-failure work is observable and forbidden rather than argued.
- runs.rs's proptest (152-217) states the greedy law completely: every non-final run is full because the successor's first record would not have fit, no run is empty, only a lone record overflows, and records round-trip in order.
- parking.rs's supply test uses an independent oracle (Merkle hash and leaf count of each assembled node against the source fan) over a 512-leaf tree, and its disputed-reply pin sums a measured half and a derived half with a deliberately tight ceiling, which has in fact been re-derived at both format changes.
- The `dispatch_height!` ladder under proptest gives full type-level coverage of the 32 heights without 32 hand-written tests, and `foreign_supply_is_rejected_at_every_scopable_height` documents the one principled exclusion (height 31) in both the impl and the testdoc.
- tests.rs keeps the fixture vocabulary small: `Version::try_from(u64)` is infallible, so the `expect` message at line 72 is a true one-line proof.
- No residue of the V1 protocol or BLAKE3 was found anywhere in the partition, and no proptest seed exists for these suites.

## Open questions for Finch

- Bundle the three ingress premises in production? `decode_reply`, `decode_leaf_reply`, and `early_supplies` (decode.rs:68-75, 203-210, 231-238) each take `version_bytes: u64`, `ledger: SupplyLedger`, and `codec: PayloadCodec` positionally, and `proxy/work.rs:54-65` holds the same three as fields and re-passes them at each call. A crate-internal struct for "what the peer's greeting declared" would shrink the signatures, remove a `u64`-first ordering hazard, and let the tests' helper (finding 2) be one `Ingress::unbounded()`. The module is private (`mod adapter;`), so this is a design change rather than a public-API change. Recommendation: yes, alongside the test-side helper.
- Retire the leaf/non-leaf entry split? The `*_leaf_reply` entries differ from the general ones only in the question closure (decode.rs:243-258, encode.rs:80-142), and production selects them from statically leaf-typed code (pump.rs). The split forces the properties.rs duplication (finding 20); `decode()` computes `scope.parent().height() - 1` at decode.rs:277 unguarded, so handing `decode_reply` a leaf-parent scope underflows. Recommendation: keep the split as a mirror of the type-level phase schedule (the rationale at adapter.rs:22-26 is sound) and add a `debug_assert!(scope.parent().height() > 1)` in the non-leaf entries so the misuse fails loudly; fix the duplication test-side.
- Add a leaf-construction injection point to `Failing`? The adapter is the only place `Leaf::leaf` is fallible and awaited on the ingress path (finding 4). Recommendation: yes; it is the custody-transfer moment and `Failing` is in-crate test infrastructure, so the cost is small.
- For the decode.rs reviewer: `SupplyOrder`'s comparison `previous >= radix` at decode.rs:531 can only fire with `previous == radix` (the `LeafOrder` check at 516-527 runs first and strict path ascent under one parent makes the radix non-decreasing), so the `>` half is a dead branch and the variant doc's "preceded" appears unreachable. Recommendation: compare with `==` and state the invariant inline, or make the impossibility an assert, per the mutants ladder.
- For the window reviewer: `supply_decode_envelope_matches_the_charge` (window/tests.rs:239-247) recomputes the flat term with its own `(FAN as u128 + 1)` rather than reading it off `from_budget` (window.rs:397-399), so the solve's factor can drift from the constant without failing it; the shared constant in finding 9 would close this too.
- Does the fan-occupancy equality pin (as opposed to the ceiling) rest on tokio's cooperative-budget behavior? The blind-spots lens read `join` as polling the reader first, with a budget-exhausted assembler unable to drain in the same poll, so the peak is exactly `FAN + 1` under every schedule the current runtime produces. Recommendation: add one sentence at the probe stating which scheduling premise the equality rests on, so a future runtime change reads as a scheduling change, not a code regression.

## Dropped

- `Encoded::write_with` has no test (blind-spots [15]): refuted. Its body is `write(frame).await?; Ok(question)`; an `Err` carries no `Q`, so "release the question on a failed write" is unrepresentable; dropping the write's `Result` is an `unused_must_use` warning under `-D warnings`; proxy/tests/failures.rs injects write faults full-stack and requires the typed transport error, and `context_registration_is_causal` pins the ordering over conforming runs. A direct unit test of a two-statement function adds a point check over covered ground.
- `assert_eq!(checked, 8)` is tautological (blind-spots [21a]): below the bar. The counter guards the loop domains (a shrunk `[false, true]` fails it), and the doc's "all eight" is thereby a mechanically enforced count, which is the shape the doctrine asks for.
- Delete the `eprintln!` in parking.rs (structure-prose [12], one item): refuted. It is the readout both re-pin commits quote; folded into finding 18 as "state its purpose at the site".
- `Stream::new(11)` needs a justifying comment (structure-prose [12], one item): refuted. The comment directly above it (parking.rs:184-185) justifies it, and `Stream::COUNT = 17` keeps every label a one-byte CBOR head.
- A miscounted underscore silently drops a height from the dispatch macros (api-economics [27]): refuted. `for height in 0..32` requests every height and the fall-through arm panics; the duplication itself is folded into finding 10 with the `seq!` shape as its resolution.
- Bundle the ingress premises in production (api-economics [25], production half): out of partition; recorded as an owner-gated open question.
- Retire the `*_leaf_reply` entries or debug_assert the height (api-economics [26], production half): out of partition; recorded as an owner-gated open question.
- Leaf-fixture searches re-derived per file (api-economics [29]): duplicate of finding 10.
- parking.rs figures stale (blind-spots [14], api-economics [23]): duplicates of finding 17.
- `MAX_RECORD_LEN` borsh-era (structure-prose [5], blind-spots [19]): duplicates of finding 22.
- properties.rs impl duplication (api-economics [26], test half): duplicate of finding 20.
- backend_errors doc overclaim (blind-spots [18]) and tests.rs map omission (api-economics [31]): duplicates of findings 4 and 1.
- Positive law in malformed.rs (api-economics [30]): duplicate of finding 11 (the two seeds proposed opposite deletions; the finding names the trade-off).
- Import layout (api-economics [32]): duplicate of finding 5.
- `read_early` rejections untested (blind-spots [16]): duplicate of finding 15.
- Erased aliases (structure-prose [7]): duplicate of finding 19, with its incorrect "never existed" premise replaced by the verified history.
- `LeafCase` fold (blind-spots [21d]): duplicate of finding 3.
- `SupplyOrder`'s dead `>` half in decode.rs (refutation, new 1): production, out of partition; recorded as an open question for the decode.rs reviewer, with the test-side half in finding 13.
- `supply_decode_envelope_matches_the_charge` recomputes the factor by hand (refutation, new 3): out of partition (window); recorded as an open question for the window reviewer.
