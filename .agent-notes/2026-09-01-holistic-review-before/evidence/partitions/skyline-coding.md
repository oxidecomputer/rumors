# Partition skyline-coding: The skyline coding: module root, admit, build, encode/decode, emit, literal, validate, text, shape, walk, and the skyline tests

## Partition summary

The skyline coding is the stored and wire form of a `Version`: preorder topology flags interleaved with delta-coded absolute leaf heights, canonical by three conditions the module root states (minimal topology, natural heights, exactness). The partition holds the coding's kernels: `validate.rs` (the strict one-pass validator on two bits per open ancestor plus one cliff-free `Accumulator`), `admit.rs` (the span wire form's fused parse-and-dominance walk, `CheckedCursor` beside a `LeafCursor`), `build.rs` (the collapsing output builder every emitter drives: absorb, re-anchor, cascade, the held leaf), `emit.rs` (join, meet, and the fused hull over one merge sweep), `text.rs` (the paper notation rendered from and parsed into streams without materializing heights), `literal.rs` (the `TryFrom` composers), `shape.rs` (the refinement walks under the public step-function iterators), `walk.rs` (the single-stream leaf-walk driver and its block scans), plus the test-only `decode.rs` and `encode.rs` transcoders. Four test files (`build/tests.rs`, `emit/tests.rs`, `text/tests.rs`, `tests.rs`) hold the differential, exhaustive, mutation, and reject suites. I read all fifteen files in full at 9e5784fb, 5092 lines, and the callees and consumers each finding cites (the codec builder, the span wire decoder, the borsh cursor, the meter shapes, the board ceilings, the fuelscape roster).

The code is in good shape at the altitude that matters. Every kernel is short and single-purpose, every deep walk keeps its transient on bit stacks (nothing in the partition recurses on input depth), and the load-bearing disciplines are argued at the code and pinned by tests a reader can re-derive by hand: the absorb face of the builder, the reset-not-subtract move in the parser (held red by a committed known-bad twin), the height-subsumption argument in the admission walk (pinned relationally, with no constant to rot), and the reject corpus whose accepted mutants are re-derived through the oracle bridge, the one comparison a lax validator cannot pass. Every test carries an accurate doc comment and lives in a sibling file.

Two cost arguments are wrong, and both name an input the committed instruments never run. The builder's amortization prices each cascade copy against its own deletion but never prices the flush that re-writes a re-anchored wide code one level up, so `join` of a flat wide leaf against a left spine of right-child pairs does Θ(depth × width) work on Θ(depth + width) input, against the door's published `O(|self| + |other|)`; every envelope row and board cell drives the absorb face only. The render merge re-adds a wide `span` once per spine level, a quadratic leading term on `WideTail`, while the fuelscape contract says "superlinear, subquadratic" with no argument. Beside those: the admission walk's mid-stream collapsible-pair arm is a second implementation of the validator's check with no gate-tier witness through `Span::decode`; the two strict parsers duplicate their obligations; a testdoc cites a design essay the crate retired; two test-local recursions sit outside the recursion inventory the hard rule points at. The remainder is leftovers of the tree's evolution (a predicate that outlived its runtime gate, stacks left on the output buffer by a mechanical migration, dual entry names) and prose nits.

## Findings

### skyline-coding-1: "currency" collides with the board's defined term; "arm" carries three senses; "door" is undefined
- Where: crates/before/src/version/skyline.rs:55-55 (related: crates/before/src/version/skyline/walk.rs:203, crates/before/src/version/skyline/shape.rs:110, crates/before/src/version/skyline/walk.rs:118-119 and 168, crates/before/src/version/skyline/emit.rs:175, crates/before/src/version/skyline/validate.rs:55, crates/before/src/version/skyline/signed.rs:1, crates/before/src/codec/stack.rs:5 and 192, crates/before/src/meter/board/currency.rs:5)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (grep: "currency" at signed.rs:1 and 191, skyline.rs:55, walk.rs:203, stack.rs:5 and 192, board/currency.rs:1-9; "door" 214 hits in src with no defining sentence; "arm" sites by reading); executed: no
- Seen by: prose; refutation: reframed (the sign-magnitude sense is anchored by signed.rs's module doc, so the defect is the collision); history: no rationale (the second sense was coined after board/currency.rs anchored the first)
- Owner-gated: no

The vocabulary rule wants each coined term anchored once; "currency" is defined by board/currency.rs as one deterministic meter, then reused here for the sign-magnitude type and in codec/stack.rs for the denomination of a transient. "arm" means the `Extremum`'s first-leaf arming, shape.rs's setting of a pending rise, and a match arm. "door" (entry point) appears 214 times crate-wide and is defined nowhere; the style guide lists it as a default-dialect term whose plain form is strictly clearer.

Evidence:

    skyline.rs
    55	//! share), and `signed` (the sign-magnitude currency: the zigzag maps, the

    walk.rs
    203	/// the way. Both arrive as signed magnitudes in the walks' exchange currency,

    shape.rs
    110	    /// Advance past the current plateau, arming any pending rise;

Resolution: skyline.rs:55 and walk.rs:203: "the sign-magnitude type" / "as `Signed` values". shape.rs:110: "setting the pending rise". walk.rs: keep "arms" only if the module defines it by contrast, else "the first leaf initializes the register and is never folded". "door" is a crate-root ruling (open question below); here "the public entry" and "a byte-level entry's whole-buffer view" read as well. Acceptance: "currency" appears in the partition only for the board's metering sense; each remaining "arm" has one sense per module.

### skyline-coding-2: transcoder cost sentence overstates on Bigroot
- Where: crates/before/src/version/skyline.rs:119-121 (related: crates/before/src/version/skyline/encode.rs:14-15 and 42-45, crates/before/src/meter.rs:174-179)
- Class / severity / confidence: claim / low / high
- Provenance: assessed (read); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale (205a361da wrote the sentence while re-denominating transcoder prose)
- Owner-gated: no

`encode_bits` pushes `value.clone()` for every internal node onto a `Vec<Base>` of inherited path sums; on `Bigroot(b, d)` each of the `d` spine nodes clones a `b`-bit `Base`, so time and peak stack are Θ(d·b) against Θ(2b + 4d) packed input bits, the genre the Bigroot family exists to expose. encode.rs's own doc states the accurate bound; the module root claims a linearity the code does not have (statement faithfulness). Test- and meter-only code, so a fixture cost, not a shipped one.

Evidence:

    skyline.rs
    119	//! flight. The construction-language transcoder (`encode_bits`, test- and
    120	//! meter-only) is the one walk that materializes path sums, priced by the
    121	//! packed stream it reads.

    encode.rs
    42	        let value = &offset + &base;
    43	        if internal {
    44	            offsets.push(value.clone());
    45	            offsets.push(value);

    meter.rs
    177	/// 1 · 0^b` (`2b + 1` bits). Puts a `b`-bit magnitude on every root-to-node
    178	/// path sum while keeping paths long — the shape that makes owned per-frame
    179	/// path sums quadratic in the input.

Resolution: reword skyline.rs:119-121 to encode.rs:14-15's bound ("transient state is one `Base` per open subtree, bounded by the packed input's depth and magnitudes"), or make the transcoder push the node's base rather than the running sum so the stack holds Θ(input) bits and the sentence becomes true. Acceptance: the two docs state the same bound; if the delta-stack rewrite lands, the length-agreement and round-trip tests in skyline/tests.rs stay green.
Construction: under `limb-meter`, transcode `Shape::Bigroot.packed2(b, d)` for (b, d) = (2048, 2048) and (4096, 4096) and read `meter::limb_ops()` per input bit; the per-bit cost roughly doubles where a stream-priced walk would stay flat.

### skyline-coding-3: two inventories of one test suite, already drifting
- Where: crates/before/src/version/skyline.rs:123-145 (related: crates/before/src/version/skyline/tests.rs:1-16, 157-185, 333-441; crates/before/src/version/skyline/emit.rs:59-70; crates/before/src/version/skyline/emit/tests.rs:1-16)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read: the topology-flag bijection pins at tests.rs:333-441 and the planted-pair proptest at tests.rs:157-185 are absent from skyline.rs's list); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale (the two pins landed by 3e5b95df2 and 0a534fe02 without touching the root inventory)
- Owner-gated: yes: which inventory is canonical is a documentation-structure ruling

Principle 5: enumerations of module contents the code can change without touching the prose rot silently; two inventories of one suite are one too many, and the divergence is already visible.

Evidence:

    skyline.rs
    123	//! # Testing
    124	//!
    125	//! - **Length agreement**: the encoder's output length equals
    126	//!   [`crate::meter::tier2::tier2_size`] bit for bit, proptested over every
    127	//!   adversarial generator family, arbitrary trees, and organic histories.

Resolution: keep the inventory in the tests module doc (where a new test is added) and reduce the kernel module's section to the invariants the tests protect plus a pointer, as `validation_index.rs` does at crate scale. Acceptance: one inventory per suite; the kernel module doc names invariants, not tests.

### skyline-coding-4: module-root instrument surface: a 21-line module for one wrapped function, dual names per entry, an inconsistently gated re-export, a comment describing one of two gated items
- Where: crates/before/src/version/skyline.rs:152-160 (related: crates/before/src/version/skyline.rs:170-176, 210-217, 262-274; crates/before/src/version/skyline/decode.rs:1-21; crates/before/src/version/skyline/emit/tests.rs:31; crates/before/src/version/skyline/text/tests.rs:31; crates/before/src/version/skyline/tests.rs:34)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep `decode_bits`: definition, the re-export at skyline.rs:212, the wrapper at 273, and tests only; grep `skyline::BitsView` across crates/before and crates/before-fuelscape: no uses; 13 `#[cfg(any(test, feature = "meter"))]` attributes in skyline.rs); executed: no
- Seen by: structure; refutation: confirmed (count corrected from nine to 13); history: deliberate-but-expired (decode.rs held a 116-line reconstruction decoder at f848fe155; the C2 flag day faf3cd0a made decode validate-then-adopt and the module outlived its content)
- Owner-gated: no

`decode.rs` exists to hold `decode_bits`, whose only non-test caller is the one-line `skyline::decode` wrapper; `validate`/`validate_bits` and `decode`/`decode_bits` are two names per entry with no difference, and the partition's own tests use both spellings; `pub use crate::codec::BitsView` rides ungated while its siblings are gated on meter and the comment above says only "the frozen form rides the meter gate" though `BitsBuf` (the build form) is gated identically. Circular justification: a module whose only justification is the wrapper that names it.

Evidence:

    skyline.rs
    152	// The storage forms, re-exported so the resource-envelope suite can name the
    153	// streams this module's entry points exchange. The frozen form rides the meter
    154	// gate: only the suite (and the public docs the meter feature exposes) name it
    155	// through this module.
    156	#[cfg(any(test, feature = "meter"))]
    157	pub use crate::codec::Bits;
    158	#[cfg(any(test, feature = "meter"))]
    159	pub use crate::codec::BitsBuf;
    160	pub use crate::codec::BitsView;

    271	#[cfg(any(test, feature = "meter"))]
    272	pub fn decode(bits: BitsView<'_>) -> Result<Version, Decode> {
    273	    decode_bits(bits)
    274	}

Resolution: inline `decode_bits`'s six lines into `skyline::decode` and delete decode.rs, its `mod` gate, and its re-export; pick one name per entry and update the tests; gate the `BitsView` re-export like its siblings (or gate none and say why) and make the comment describe both; consider one gated inline module for the meter-facing surface so the cfg is stated once. Acceptance: `just gate` clean under default, test, and meter features; tests/meter.rs's `meter::skyline::{encode, validate, decode, view}` calls compile unchanged.

### skyline-coding-5: em-dashes in 24 line comments and one assert message
- Where: crates/before/src/version/skyline.rs:239-239 (related: admit.rs:301; build.rs:134, 136, 330, 331; emit.rs:75, 217, 296; emit/tests.rs:143, 144; literal.rs:55; text.rs:237, 337, 347, 348, 504, 510, 543, 548, 556; text/tests.rs:356 and the assert string at 551; tests.rs:126, 192; all under crates/before/src/version/skyline/)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep `//.*—` over the fifteen files, doc comments excluded, reproduces exactly the 24 sites; grep `—` on non-comment lines finds text/tests.rs:551); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale (the style guide states the rule for exactly this register)
- Owner-gated: no

Owner doctrine (Code Organization): prefer colons or semicolons over em-dashes in log messages and comments; the style guide adds that the spaced double-hyphen is the dash of code comments. Rendered rustdoc is exempt and was not counted. literal.rs:55's em-dash sits in a trailing comment rustfmt has aligned to column 22 (see skyline-coding-21).

Evidence:

    skyline.rs
    239	    // Live bits only — no padding: consumers walk these as a stream.

    text/tests.rs
    551	             the adequacy witness went green — the kernel no longer demonstrates \

Resolution: replace each with a colon, semicolon, or restructured sentence; the assert message becomes "the adequacy witness went green: the kernel no longer demonstrates ...". Acceptance: `grep -n '//.*—' <partition files> | grep -v '//[/!]'` returns nothing and no assert/expect/panic string in the partition contains an em-dash.

### skyline-coding-6: the admission walk's mid-stream collapsible-pair rejection has no committed witness through Span::decode
- Where: crates/before/src/version/skyline/admit.rs:174-180 (related: crates/before/src/version/skyline/admit.rs:168-173 and 210-219; crates/before/src/span/tests.rs:333-353; crates/before/src/borsh_impls/tests.rs:1364-1389; crates/before/src/version/skyline/tests.rs:157-185; crates/before/tests/fuzz_seeds.rs:173-221; crates/before/src/span/wire.rs:150-158)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of span/tests.rs, borsh_impls/tests.rs, and tests/fuzz_seeds.rs for the fused door's non-canonical witnesses; the byte trace below derived by hand against admit.rs:116-219 and span/wire.rs:134-158); executed: no
- Seen by: correctness; refutation: confirmed (independent trace agrees); history: no rationale (the borsh witness names itself the "close-out arm" tripwire; 9cc00ccac unified `close_ancestor` without recording the step-arm gap)
- Owner-gated: no

`CheckedCursor` is a second implementation of the validator's minimal-topology check, reachable from wire bytes through `Span::decode` and the borsh span leg. The arm that fires when a collapsible pair's parent closes inside `step()` (leaves still to come) is exercised at gate tier by nothing: the two hand-built collapsible joins (span/tests.rs:338, borsh_impls/tests.rs:1375) are the root-level pair `0b0111_1000`, which closes in `finish()`; the planted-pair proptest drives `validate_bits` only; `span_wire_decode_matches_bitwise_reference` compares two cursors under the same fused walk; the replayed fuzz seeds' `NotCanonical` cases are `span_crossed` and `span_negative_join`; the fused-vs-composed differential is the off-gate fuzz target. Hard rule: decode strictly rejects non-canonical input because `Eq`/`Hash` rest on byte equality, so a gap here is an equality bug; doctrine: a criterion needs a committed demonstration that a known-bad mechanism fails it, and a walk that checks pairs only at `finish()` passes today's gate.

Evidence:

    admit.rs
    172	        let mut is_leaf = true;
    173	        let mut zero_delta = self.last_delta_zero;
    174	        loop {
    175	            match self.path.pop() {
    176	                Some(true) => {
    177	                    // This ancestor closes: the completed subtree was its right
    178	                    // child.
    179	                    self.close_ancestor(&mut is_leaf, &mut zero_delta)?;
    180	                }

    span/tests.rs
    338	    let collapsible: Vec<u8> = vec![0b0111_1000];

    borsh_impls/tests.rs
    1370	/// forbids. It rides beside
    1371	/// [`span_wire_decode_matches_bitwise_reference`] as the deterministic
    1372	/// tripwire for the close-out arm's genre.

Resolution: add to src/span/tests.rs, beside the existing witness, a deterministic composite whose join carries a pair closing mid-stream (the bytes below; note they carry a proper padding marker, which the existing `0b0111_1000` witness does not, passing only because `NotCanonical` fires before `require_marker_padding`), and a proptest mirroring `planted_collapsible_pairs_are_rejected_at_every_leaf` through `Span::decode`: take a canonical `lo <= hi`, split one leaf of `hi` into an equal-sibling zero-delta pair at a proptest-chosen preorder position (heights unchanged, so only canonicality can reject), re-pad, assert `Err(Decode::NotCanonical)`; run the composite through the borsh span leg too. Acceptance: `Span::decode(&[0xE0, 0x3E, 0xE0]) == Err(Decode::NotCanonical)` is asserted and the proptest is committed; replacing `self.last_delta_zero` with `false` at admit.rs:173 turns both red while every existing witness stays green.
Construction: composite `[0xE0, 0x3E, 0xE0]`: `lo` = `11` (the empty version) with marker at bit 2; `hi` bits `0 0 1 1 1 1 1 011` then marker at bit 10 (root internal, left internal, leaf gamma(0), leaf zigzag(0), leaf zigzag(+1)). Trace: `hi` opens at depth 2, `D = 0`. First `advance` (`lo` depth 0 < 2): `hi.step()` flips at level 2, pushes `left_was_leaf = true`, reads the second leaf's code 0, sets `last_delta_zero = true`. Second `advance`: `hi.step()` pops `true` and `close_ancestor(is_leaf = true, zero_delta = true)` sees `left_was_leaf = true`, so `Err(NotCanonical)`. With that arm's `zero_delta` forced `false`: the walk flips at the root, reads +1, `D = -1` (Less, `equal = false`), both cursors done, `finish()` pops the single right branch with `left_was_leaf = false` and returns `Dominates`; `require_marker_padding(tail, 10)` passes; `Span::decode` accepts a span whose `hi` is the non-canonical `[0x3E, 0xE0]`, a second live spelling of the constant-0-then-1 function under `Eq`.

### skyline-coding-7: "mints" for constructing an error value
- Where: crates/before/src/version/skyline/admit.rs:256-257 (related: none in the partition; 65 `mint` uses crate-wide mix the coinage and construction senses)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep `\bmint` over the fifteen files: skyline.rs:7 and admit.rs:256 only); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-and-holds for skyline.rs:7 (the coinage sense, under the owner's dissolve/link/mint vocabulary ruling in a736ef14a), so only this site remains
- Owner-gated: no

The style guide bans "mint" for constructing a value in code, prose, or comments; this site constructs a `Decode::NotCanonical`.

Evidence:

    admit.rs
    256	/// padding, the wire-side cursor its final byte's dead bits) and mints
    257	/// [`Decode::NotCanonical`] from a [`Refuted`](Admission::Refuted) verdict only

Resolution: "and returns [`Decode::NotCanonical`] for a [`Refuted`] verdict only after they pass". Acceptance: the construction sense of "mint" appears nowhere under skyline/.

### skyline-coding-8: qualified paths where the import already exists; a one-line `version_of` alias copied nine times
- Where: crates/before/src/version/skyline/admit.rs:296-298 (related: crates/before/src/version/skyline/decode.rs:18-19; crates/before/src/version/skyline/text/tests.rs:130 and 241; crates/before/src/version/skyline/emit/tests.rs:430-433; `crate::codec::built_view(` 42/19/5/3 times in emit/tests.rs, tests.rs, text/tests.rs, build/tests.rs; `fn version_of` at tests.rs:37, emit/tests.rs:37, text/tests.rs:37, fill/tests.rs:47, grow/tests.rs:44, sweep/tests.rs:29, query/tests.rs:36, meter/board/tests.rs:36, tests/meter.rs:411)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep counts as listed; `built_view` is `pub(crate)` and skyline.rs:228 imports it plainly); executed: no
- Seen by: structure, prose; refutation: confirmed; history: no rationale (the qualified spellings arrived mechanically with the BitsView/BitsBuf migrations 5d167a63 and 83e61b4d; `version_of` had a real body until faf3cd0a introduced `Packed::version()`)
- Owner-gated: no

Doctrine: imports over long qualified paths except where the qualification informs. admit.rs imports from `super::signed` at line 57 yet spells `super::signed::fold_signed_int`; decode.rs imports `BitsView` yet spells `crate::codec::BitsBuf::with_capacity`; text/tests.rs imports `validate_bits` yet spells `super::super::validate_bits` at line 130; and the repetition of `crate::codec::built_view(` is what makes emit/tests.rs:430-433 unbreakable 250-column `prop_assert_eq!`s. The `version_of` alias adds nothing over `p.version()`.

Evidence:

    admit.rs
    296	    let mut diff = Accumulator::new();
    297	    super::signed::fold_signed_int(&mut diff, Sign::Positive, &lo_first);
    298	    super::signed::fold_signed_int(&mut diff, Sign::Negative, &hi_first);

    decode.rs
    18	    let mut copy = crate::codec::BitsBuf::with_capacity(bits.len() + 1);
    19	    crate::codec::extend_from_view(&mut copy, bits, 0, bits.len());

    emit/tests.rs
    36	/// Decode a meter-generated packed shape as a [`Version`].
    37	fn version_of(p: &Packed) -> Version {
    38	    p.version()
    39	}

Resolution: import `fold_signed_int`, `BitsBuf`, `extend_from_view`, and `built_view` at each site (or bind a local `fn v(b: &BitsBuf) -> BitsView<'_>` for the law tests); replace `version_of(p)` with `p.version()` and delete the in-crate copies (tests/meter.rs's copy is an integration test and may keep a local helper). Acceptance: `just fmt` and `just clippy` clean; `grep -c 'crate::codec::built_view(' <partition test files>` is 0; `grep -rn 'fn version_of' crates/before/src` returns nothing.

### skyline-coding-9: the re-anchor cascade re-copies a wide left-sibling code once per level: join is Θ(depth × width), not linear
- Where: crates/before/src/version/skyline/build.rs:29-34 (related: crates/before/src/version/skyline/build.rs:152-155 and 318-340; crates/before/src/codec/build.rs:93-107 and 110-122; crates/before/src/version/skyline/emit.rs:46-57; crates/before/src/version.rs:889-900; crates/before-fuelscape/src/ops.rs:562-578; crates/before/src/lib.rs:350-358; crates/before/tests/meter.rs:1881 and 1929-1949; crates/before/src/version/skyline/emit/tests.rs:242-261; crates/before/src/version/skyline/build/tests.rs:138-150)
- Class / severity / confidence: claim / high / high
- Provenance: assessed (read; three independent hand traces of `SkylineBuilder::leaf`/`cascade` on the construction below agree: the claims lens, the refutation pass, and mine); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale (build.rs, the 2026-07-23 design note, and the exposition all price the copy against the deletion and none prices the re-flush)
- Owner-gated: no

The module doc prices each cascade copy against "that deletion", but after a re-anchor the extracted code becomes the held leaf, the next leaf's flush writes it again (build.rs:154-155), and the next cascade extracts it again one level up (build.rs:333-335). On `join` of a flat wide leaf against a left spine whose right child at every level is a two-leaf pair, the emitted leaf sequence is `(d, gamma(W))` then zero deltas at depths `d+1, d+1, d, d, ..., 2, 2`; each level flushes W bits (`push_code` splices a `Code::Wide`), extracts W bits (`extract_code` copies bit by bit past `SMALL_CODE_BITS`), and truncates W+2, so the builder does Θ(d·W) work on Θ(d + W) input bits and a (1 + W)-bit output. Contract breached: crates/before-fuelscape/src/ops.rs:571 publishes `O(|self| + |other|)` for `version_join`, and lib.rs:351-353 makes every asymptotic claim a hard guarantee for all input shapes. The public path reaches the kernel: `join_refs` (version.rs:889-900) short-circuits only equality and the empty operand. Uninstrumented: `SKYLINE_JOIN_ABSORB` (tests/meter.rs:1881, 1936-1949) joins the flat leaf against `Dense`, a left spine, so it exercises the absorb face where the held code never moves; the board pairs each shape with its once-ticked twin; `reanchor_cascade_climbs_chained_levels` is a pure right spine where the code moves once; no row or cell mentions re-anchor.

Evidence:

    build.rs
    29	//! Cascading is the loop over re-anchor: a merged leaf may in turn be a
    30	//! zero-delta right sibling one level up. Each cascade step deletes at least
    31	//! three stream bits and copies only a code already priced by that deletion, so
    32	//! emission stays amortized O(1) per output bit; the wide code a deep uniform
    33	//! region telescopes onto is *held*, never re-copied (the absorb repair moves
    34	//! no code bits at all, whatever the held width).

    153	        let flushed_len = held.len();
    154	        self.out.push_bit(true);
    155	        self.out.push_code(&held);

    333	            let code_len = self.lens.pop();
    334	            let code = self.out.extract_code(self.out.len() - code_len);
    335	            self.out.truncate(self.out.len() - code_len - 2);

    codec/build.rs
    102	        let mut out = BitsBuf::with_capacity(n);
    103	        for i in start..start + n {
    104	            out.push(self.bit_at(i));
    105	        }
    106	        Code::Wide(out)

    before-fuelscape/src/ops.rs
    571	        contract: "`O(|self| + |other|)`",

Resolution: decide the class first. The strict cure is structural: emit topology flags and payload codes into two streams and interleave once in `finish()`; every repair then becomes an O(1) truncation of the topology stream plus, for absorb, one 1-bit truncation of the payload stream, and the held-leaf discipline, `lens`, and `extract_code` dissolve (`continue_verbatim` would de-interleave its source range by walking it, still linear). If instead the class is accepted, restate build.rs:29-34 and emit.rs:46-57 as Θ(output + Σ over re-anchors of the re-anchored code width) and change the `version_join`/`version_meet`/`version_span` island contracts to match; the crate docs' hard-guarantee sentence forbids leaving both as they stand. Either way, land the instrument in the same change: a two-scale flatness pin on the construction below (scan bits, and peak heap), plus a committed known-bad demonstrator if the builder is rewritten. Acceptance: a committed test under `scan-meter` builds `b(d)` = the text `(0, T_{d-1}, (0, 0, 1))` iterated from `T_0 = 0` and `a = Shape::Hugeleaf.packed1(10·d)`, measures `meter::scan_bits()` around `&a | &b` at `d` and `2d`, and asserts the per-input-bit scan cost stays within ×1.25 (it reads about ×2 per doubling today by the trace); the same pair added to `assert_emits` still matches the oracle byte for byte; the island contracts and the build.rs/emit.rs cost prose agree with the code.
Construction: `fn spine_of_pairs(d: usize) -> Version { let mut t = String::from("0"); for _ in 0..d { t = format!("(0, {t}, (0, 0, 1))"); } t.parse().unwrap() }` (canonical: every node has a zero-base child, no equal sibling leaves; preorder heights 0, then 0,1 repeated). `fn join_scan(d: usize) -> (u64, u64) { let a = Shape::Hugeleaf.packed1(10 * d).version(); let b = spine_of_pairs(d); let bits = (a.encode().len() + b.encode().len()) as u64 * 8; meter::reset_scan_bits(); std::hint::black_box(&a | &b); (meter::scan_bits(), bits) }`. Expected under the doc's claim: scan/bits flat from `d` to `2d`. Expected from the trace: about `2·d·(20d)` scan bits against about `30d` input bits, so the per-bit ratio doubles per doubling.

Disposition (owner ruling 1, 2026-09-02, `triage/rulings.md`): the asymptotic claims are absolute, so this is a defect to cure, not a class to accept; the instrument (the two-scale scan pin above, as an envelope row and a board family) lands first so the breach reads as a failure, then the fix; the candidate cure is to hold the re-anchored code in place at the stream's tail instead of extracting and re-splicing it, with the two-stream builder as the fallback (unverified). The ruling also asks for an audit of what else the join, meet, span, rank, masked-comparison, and coverage instruments miss by driving only the benign face.

### skyline-coding-10: `Option<bool>` tests spelled through `map`/`unwrap_or` where `== Some(..)` reads directly
- Where: crates/before/src/version/skyline/build.rs:137-140 (related: crates/before/src/version/skyline/build.rs:323-326; crates/before/src/version/skyline/fill.rs:1261; crates/before/src/version/skyline/fill/prescan.rs:673)
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read; `BitStack::last` returns `Option<bool>`, codec/stack.rs:168); executed: no
- Seen by: structure; refutation: confirmed (the crate already spells it `== Some(false)` at fill.rs:1261 and prescan.rs:673); history: no rationale (the closure form arrived with the BitStack conversion 56a3dfe25)
- Owner-gated: no

Legibility on a correctness-critical predicate: the reader evaluates a closure and a default to recover "is `Some(false)`".

Evidence:

    build.rs
    137	        if depth == self.path.len()
    138	            && self.path.last().map(|bit| !bit).unwrap_or(false)
    139	            && code.len() == ZERO_DELTA_CODE_BITS
    140	        {

    323	            if held.len() != ZERO_DELTA_CODE_BITS
    324	                || !self.path.last().unwrap_or(false)
    325	                || !self.left_leaf.last().unwrap_or(false)
    326	            {

Resolution: `self.path.last() == Some(false)` at 138; `self.path.last() != Some(true) || self.left_leaf.last() != Some(true)` at 324-325 (behavior-preserving: both forms treat `None` and `Some(false)` alike). Acceptance: build/tests.rs and the emit differentials unchanged.

### skyline-coding-11: `held_at` outlived the runtime gate it was introduced for
- Where: crates/before/src/version/skyline/build.rs:192-200 (related: crates/before/src/version/skyline/build.rs:248-253; crates/before/src/version/skyline/build/tests.rs:329-387; .cargo/mutants.toml:15-19)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep `held_at` across src, tests, benches, examples: only build.rs:198, build.rs:249, and build/tests.rs:339-387); executed: no
- Seen by: structure; refutation: confirmed; history: deliberate-but-expired (77d7da0b introduced it as a runtime gate in fill.rs; e7d2548f removed that gate as dead and kept the predicate for the debug_assert; d8ade9f8 then added the two tests to kill mutants of it)
- Owner-gated: no

The accessor's one production use is the `debug_assert!` inside `continue_verbatim`; two dedicated tests pin a one-line predicate only a debug assertion reads. Principle 3: machinery outlives the constraint that justified it, and tests whose purpose is to kill mutants of a debug-only predicate are the disposition .cargo/mutants.toml's ladder puts last ("refactor, so the mutated codepoint does not structurally exist").

Evidence:

    build.rs
    198	    pub(super) fn held_at(&self, depth: u64) -> bool {
    199	        self.held.is_some() && self.path.len() == depth
    200	    }

    248	        debug_assert!(
    249	            self.held_at(root_depth + first_rel_depth),

Resolution: inline the conjunction into the `debug_assert!` (`self.held.is_some() && self.path.len() == root_depth + first_rel_depth`), delete `held_at` and the two `held_at_*` tests, and let the tick/fill/grow differentials carry the precondition. Acceptance: `just gate` clean; no reference to `held_at` remains; the grow/fill differential suites are unchanged.

### skyline-coding-12: `continue_verbatim`'s seven positional arguments are spelled at four sites under two clippy allows
- Where: crates/before/src/version/skyline/build.rs:233-243 (related: crates/before/src/version/skyline/fill/fuse.rs:173-193; crates/before/src/version/skyline/grow.rs:319-331 and 392-400; crates/before/src/version/skyline/fill.rs:1060-1068; crates/before/src/version/skyline/walk.rs:207-222; crates/before/src/version/skyline/build/tests.rs:180-220)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep `continue_verbatim`: the definition, the fuse.rs wrapper, and call sites grow.rs:392 and fill.rs:1060); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale (the `(src, start, end)` triple is deliberate per 5d167a63 and its allow comment; the four depth/length coordinates being positional has none)
- Owner-gated: no

Types-first doctrine: six adjacent `u64`s of distinct meaning are swap-prone and self-documenting only at the definition; the allow comment concedes the range is one argument; the call sites unpack values from structs that already exist (`grow.rs`'s `Subtree`, `walk.rs`'s `RegionSkip`), and the test helper returns them as an anonymous 4-tuple.

Evidence:

    build.rs
    233	    #[allow(clippy::too_many_arguments)] // (src, start, end) is one logical range argument
    234	    pub(super) fn continue_verbatim(
    235	        &mut self,
    236	        src: BitsView<'_>,
    237	        start: u64,
    238	        end: u64,
    239	        root_depth: u64,
    240	        first_rel_depth: u64,
    241	        last_rel_depth: u64,
    242	        last_code_len: u64,
    243	    ) {

Resolution: introduce a small struct (a `Continuation { range: Range<u64>, root_depth, first_rel_depth, last_rel_depth, last_code_len }`, or a pre-sliced view plus the four coordinates as a named struct); delete both clippy allows; build it from `Subtree` in grow.rs and from `RegionSkip` in fill.rs; return it from the test helper. Acceptance: `just gate` clean with clippy and without the allows; tick/fill/grow differentials unchanged.

### skyline-coding-13: builder testdocs misstate their bodies (leaf count; an unasserted cost claim)
- Where: crates/before/src/version/skyline/build/tests.rs:69-71 (related: crates/before/src/version/skyline/build/tests.rs:74-78, 109-120, 375-383)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read: both bodies feed three leaves at depths 2, 2, 1; the third body asserts only stream equality); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale (the mismatch is from birth at 147b55f6c)
- Owner-gated: no

AGENTS.md: every test's doc comment states the behavior it protects and review holds it to the standard; an incorrect testdoc is a bug in the test. Two docs say "four equal leaves at depth 2" over bodies feeding three leaves (the tiling of `((7, 7), 7)`), and a third says the wide code "is written exactly once" while the body instruments no writes.

Evidence:

    build/tests.rs
    69	/// The absorb cascade climbs: four equal leaves at depth 2 collapse pairwise
    70	/// all the way to a single depth-0 leaf, one parent-flag truncation per level,
    71	/// with the held code never moving.

    74	    let stream = built(vec![
    75	        (2, gamma(7)),
    76	        (2, delta(Sign::Positive, 0)),
    77	        (1, delta(Sign::Positive, 0)),
    78	    ]);

    109	/// Deep uniformity around a wide code stays a single leaf: a depth-8
    110	/// left spine of equal plateaus collapses level by level while the wide
    111	/// held code is written exactly once.

    375	    // Cascade: four equal depth-2 leaves collapse pairwise to depth 0.

Resolution: lines 69-71 and 375: "three equal plateaus, two at depth 2 and one at depth 1 (the tiling of `((7, 7), 7)`), collapse pairwise to a single depth-0 leaf". Lines 109-111: drop "while the wide held code is written exactly once", or make it true by asserting on the scan meter (the write count is what `record_bits` records). Acceptance: each testdoc describes exactly the leaf sequence its body feeds and claims only what its assertions check.

### skyline-coding-14: ghost reference to the retired `implementation` essay in a test doc
- Where: crates/before/src/version/skyline/build/tests.rs:394-396 (related: crates/before/AGENTS.md:6; crates/before/examples/code_study.rs:6; both outside this partition)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (grep for `mod implementation`, `before::implementation`, and the phrase across crates/before finds only the three citing sites); executed: no
- Seen by: prose, correctness, claims; refutation: confirmed (three lenses, one finding); history: contradicts-hard-rule (67970b75 added the module; 22cdfbe1 retired it and left the citations standing)
- Owner-gated: no

crates/before/AGENTS.md's hard rule and Principle 5: nothing in the codebase refers to code that no longer exists; provenance lives in git. The citation sits in the doc of the pin whose purpose is to name what it guards. Severity medium rather than high because the blast radius is one parenthetical and the fix is mechanical; the weightier occurrence is the guidepost at AGENTS.md:6, outside this partition.

Evidence:

    build/tests.rs
    394	/// independently; a different integer code (the `implementation` essay
    395	/// contemplates ζ₂, whose zero costs two bits) would silently turn every
    396	/// collapse check into a no-op — this pin turns that into a red test.

Resolution: restate the counterfactual in terms of what is: "a different integer code whose zero costs two bits (ζ₂, for instance) would turn every collapse check into a no-op"; excise the two out-of-partition sites in the same pass. Acceptance: `grep -rn 'implementation\` essay\|before::implementation' crates/before` returns nothing; the testdoc still names the two-bit-zero counterfactual.

### skyline-coding-15: `hull` reads `sign_magnitude` twice per switch boundary; both emissions switch together
- Where: crates/before/src/version/skyline/emit.rs:251-262 (related: crates/before/src/version/skyline/emit.rs:96-112, 166-172, 333-352, 356-370; crates/suanpan/src/accumulator.rs:950-968)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read: `sign_magnitude` is `O(|self|)` digit touches plus a same-order allocation per its doc); executed: no
- Seen by: claims; refutation: confirmed (with the precision that after an opening tie only one emission moves at the first departure; every later switch moves both); history: no rationale (70eb67ab1's accounting of what hull doubles omits it)
- Owner-gated: no

Fixed-sign optimization (strict deletion of redundant work): `follow_max` and `follow_min` are mirrors, so once seated on opposite sides both change side at the same boundary, and `delta_code` → `switch_delta` → `diff.sign_magnitude()` runs twice for one `(sign, magnitude)` pair on the per-boundary hot path; the hull doc's "only the per-interval side selections and the two output builders are doubled" undercounts it.

Evidence:

    emit.rs
    251	        for emission in &mut outputs {
    252	            let new_side = (emission.pick)(sign, emission.side);
    253	            let code = delta_code(
    254	                &diff,
    255	                emission.side,
    256	                new_side,
    257	                step_a.as_ref(),
    258	                step_b.as_ref(),
    259	            );
    260	            emission.side = new_side;
    261	            emission.out.leaf(depth, code);
    262	        }

    357	    let (diff_sign, magnitude) = diff.sign_magnitude();

Resolution: in hull's loop compute the magnitude lazily once per boundary (only when either emission's `new_side` differs) and pass it to both `switch_delta` calls; keep `emit`'s single-output path unchanged. Acceptance: the hull differential in emit/tests.rs stays byte-identical; the `version_span` board cells' touch readings on switch-heavy pairs fall and are re-pinned under attribution; no other column rises.

### skyline-coding-16: "costing one reallocation" is unargued; a doubling buffer can reallocate more than once
- Where: crates/before/src/version/skyline/emit.rs:295-297 (related: crates/before/src/codec/build.rs:60-72)
- Class / severity / confidence: claim / nit / medium
- Provenance: assessed (read: `PackedBuilder::with_capacity` is `Vec::with_capacity(capacity / 8 + 1)` with std's doubling growth); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale (c8ae28c70 rewrote the sentence to state its epistemic status; "one" was phrasing, not a derived bound)
- Owner-gated: no

Statement faithfulness in cost prose: an overrun past twice the estimate costs two reallocations; the heap envelopes pin the measured peak, so nothing is at risk beyond the sentence.

Evidence:

    emit.rs
    295	    // is bounded by the boundary's input codes only up to a constant, so a
    296	    // pathological switch-heavy pair could outgrow it — costing one
    297	    // reallocation, never correctness.

Resolution: "a bounded number of reallocations, never correctness", or derive the output bound (each elementary interval's code is at most the wider input code at that boundary plus a constant) and size the capacity to it. Acceptance: the comment states only what is argued.

### skyline-coding-17: `emit` and `hull` are two copies of the emission driver
- Where: crates/before/src/version/skyline/emit.rs:313-319 (related: crates/before/src/version/skyline/emit.rs:190-271, 219-243, 232-234, 298-302)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read); executed: no
- Seen by: structure; refutation: confirmed (the sign of the proposed const-generic sweep is not fixed: hull takes a `fn` pointer and a `Directions` fold join and meet would then pay); history: no rationale (the Wave 6 dedup pass worked on this file and left both drivers)
- Owner-gated: no

Both functions open the pair, seed the first output leaf from the winning side's absolute height, and run the same `advance_diff`/pick/`delta_code`/`leaf` loop; the "sticky-tie seed `Side::A` is arbitrary" comment is duplicated verbatim, and `hull` seats `side: Side::A` placeholders only to re-seat them in a loop. The switch algebra is already shared, so only the driver remains doubled.

Evidence:

    emit.rs
    313	    while !(cursor_a.done() && cursor_b.done()) {
    314	        let (step_a, step_b) = advance_diff(&mut cursor_a, &mut cursor_b, &mut diff);
    315	        let new_side = pick(diff.sign(), side);
    316	        let code = delta_code(&diff, side, new_side, step_a.as_ref(), step_b.as_ref());
    317	        side = new_side;
    318	        out.leaf(cursor_a.depth().max(cursor_b.depth()), code);
    319	    }

    216	    // height; the capacity estimate is `emit`'s, per builder. The `side`
    217	    // initializers are placeholders — the loop below seats each output's
    218	    // real opening side.

Resolution: one const-generic sweep, `fn sweep<const N: usize>(a, b, picks: [fn(Ordering, Side) -> Side; N]) -> ([BitsBuf; N], Directions)`, with join/meet taking output 0 and hull taking both; construct each `Emission` with its real opening side instead of a placeholder. Measure join/meet on the bench judge before landing (touch meters are unaffected: `Directions` is two bools). Acceptance: emit/tests.rs differentials and the `span_is_the_pair_hull` law pass; `skyline_join_*`/`skyline_meet_*` envelopes hold; the bench judge shows no join/meet regression.

### skyline-coding-18: same-side wide steps are decoded then re-gamma-coded instead of spliced from the source
- Where: crates/before/src/version/skyline/emit.rs:344-348 (related: crates/before/src/codec/code.rs:42-59; crates/before/src/version/skyline/build.rs:268-269; crates/before/tests/meter.rs:1884)
- Class / severity / confidence: performance / low / medium
- Provenance: assessed (read); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale (147b55f6c describes the intent as "same-side deltas pass through" while the implementation re-encodes)
- Owner-gated: no

On the same-side path the output code equals the followed side's own input code bit for bit, yet `delta_code` rebuilds it with `gamma_code_signed_int` from the decoded `Int` (a fresh `BitsBuf` built bit by bit for `Int::Wide`) where `Code::from_range` would copy the source range; the fill/grow path already takes the copy route through `continue_verbatim`. The decode is needed for the accumulator fold; the re-encode is redundant. Fixed sign for `Wide` codes, workload-dependent for word-scale ones, so measure first.

Evidence:

    emit.rs
    344	    if new_side == side {
    345	        return match step {
    346	            Some(step) => gamma_code_signed_int(step.sign, &step.magnitude),
    347	            None => gamma_code_signed_int(Sign::Positive, &Int::ZERO),
    348	        };

Resolution: have `LeafCursor::step` also report the payload code's bit range and let `delta_code` return `Code::from_range(src, start, end)` on the same-side path when the step is `Some`; keep the re-encode only for switches. Measure at the parent on `SKYLINE_JOIN_WIDE_TOOTH` and `SKYLINE_JOIN_DENSE` before adopting. Acceptance: the emit differential is unchanged; `SKYLINE_JOIN_WIDE_TOOTH` limb ops fall measurably with the row re-pinned under attribution, and `SKYLINE_JOIN_DENSE` does not rise.

### skyline-coding-19: `expect` message names the wrong artifact
- Where: crates/before/src/version/skyline/encode.rs:36-36 (related: crates/before/src/version/skyline/encode.rs:1-3 and 54-58)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (accurate at f848fe155 when the input was a stored `Version`; faf3cd0a made the input a generator-built packed stream and 205a361da's re-denomination missed this message)
- Owner-gated: no

Every `expect` message is a one-line proof; the transcoder's input is the packed construction-language stream, not a `Version`'s stored stream, so this one proves the wrong premise; line 57's sibling message is correct.

Evidence:

    encode.rs
    36	        let (base, next) = codec::decode_int(bits, pos).expect("canonical Version parses cleanly");

Resolution: "a canonical packed preorder stream parses cleanly". Acceptance: the message names the packed construction-language stream.

### skyline-coding-20: the literal composer rescans both children per level; the `O(m)` constant is depth-proportional and unstated
- Where: crates/before/src/version/skyline/literal.rs:32-34 (related: crates/before/src/version/skyline/literal.rs:82-119; crates/before/src/version.rs:1486-1488 and 1497-1511)
- Class / severity / confidence: claim / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: correctness, claims; refutation: reframed (the nesting depth is a property of the tuple type, so per instantiation `O(m)` holds; the constant grows with the literal's static nesting); history: no rationale (the claim was written as prose and transcribed into the retired checker as `Bound::Linear`; the fuelscape roster files `TryFrom literals` under `version_display`'s superlinear contract)
- Owner-gated: no (the doc-precision fix); a one-pass composer that reshapes the `TryFrom` bounds would be

`node()` scans both already-built child streams in full, materializing a `Vec<Base>` of every leaf height, at every composition level; the public `O(m)` holds because a tuple literal's depth is fixed by its type, but the constant is that depth, the `# Complexity` section does not say so, and the per-node `Vec<Base>` is the materialization the render and parse kernels deliberately avoid.

Evidence:

    literal.rs
    32	pub(crate) fn node(base: u64, left: BitsView<'_>, right: BitsView<'_>) -> Result<BitsBuf, Parse> {
    33	    let (left_topology, left_heights) = scan(left);
    34	    let (right_topology, right_heights) = scan(right);

    version.rs
    1486	/// # Complexity
    1487	///
    1488	/// `O(m)`, with `m` the built version's size in bytes.

Resolution: state the constant in the `# Complexity` section ("each nesting level of the literal rescans its children, so the constant is the literal's static depth"), or build the literal in one pass by lowering the tuple to an iterative walk that feeds `SkylineBuilder` directly (the parse kernel already does this for text), which also retires `scan()`'s `Vec<Base>`. Acceptance: the `TryFrom<(u64, T, S)>` complexity text matches a stated derivation; if the one-pass builder lands, the literal doctests and the text/tests.rs corpus stay green.

### skyline-coding-21: `literal::node` builds a temporary buffer only to iterate it; a rustfmt-displaced trailing comment
- Where: crates/before/src/version/skyline/literal.rs:53-58 (related: crates/before/src/version/skyline/literal.rs:59-63)
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read; `BitsBuf::iter` yields `bool`s, so a chained iterator is a drop-in); executed: no
- Seen by: structure, prose; refutation: confirmed; history: no rationale (the shape dates from faf3cd0a; the Wave 6 naming sweep renamed the locals without restructuring)
- Owner-gated: no

A needless allocation in a production entry (the public `TryFrom` impls route here), and a trailing comment whose continuation rustfmt aligned to column 22 so it reads as a fragment narrating the next lines.

Evidence:

    literal.rs
    53	    let mut bits = BitsBuf::new();
    54	    bits.push(false); // topology: this node
    55	                      // …then the left subtree's topology, then the right's — but the
    56	                      // payloads interleave, so re-emit.
    57	    bits.extend_from_buf(&left_topology);
    58	    bits.extend_from_buf(&right_topology);

Resolution: `let flags = core::iter::once(false).chain(left_topology.iter()).chain(right_topology.iter());` and move the comment onto its own lines above ("Topology: this node, then the left subtree's, then the right's; the payloads interleave, so re-emit below."). Acceptance: the `TryFrom` doctests and the version literal tests are unchanged.

### skyline-coding-22: the overlay-advance law has a third generic statement; overlay.rs's "exactly two generic faces" is a stale hand-maintained count
- Where: crates/before/src/version/skyline/shape.rs:176-198 (related: crates/before/src/version/skyline/shape.rs:17-18; crates/before/src/version/skyline/overlay.rs:12-17 and 268-286; crates/before/src/version/skyline/admit.rs:340-347; crates/before/src/shape.rs:284-287 and 357-359)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for the tie-assert string: admit.rs:370 and 382, overlay.rs:176, 190, and 283, shape.rs:195; both shape call sites read); executed: no
- Seen by: structure; refutation: reframed (the count correction is the solid part; folding `advance_refinement` into `CursorSet` would need an all-done guard at both callers because `advance_set` steps the deepest slot unconditionally); history: no rationale (e30d659de made the count true; 46eb64f97 added shape.rs's statement without saying why `advance_set` was not used)
- Owner-gated: no

Principle 5: a hand-maintained count that the tree has outgrown. `advance_refinement<W: Refine>` is the N-ary law with done-ness folded in, `admit::advance` is the documented fallible restatement, and overlay.rs still says the law is stated in exactly two generic faces; each statement carries its own copy of the tie assert.

Evidence:

    shape.rs
    17	//! walk whose depth the flip level reaches. [`advance_refinement`] states
    18	//! that law once over [`Refine`], for any arity and either walk kind;

    176	pub(crate) fn advance_refinement<W: Refine>(walks: &mut [W]) -> bool {

    191	    let flip = walks[deepest].advance();
    192	    for (slot, walk) in walks.iter_mut().enumerate() {
    193	        if slot != deepest && !walk.done() && walk.depth() >= flip {
    194	            let tied = walk.advance();
    195	            debug_assert_eq!(tied, flip, "tied boundaries close to one shared flip level");

    overlay.rs
    12	//! boundary carrying the cursor's own crossing payload. The overlay-advance law
    13	//! is stated (and debug-asserted) in exactly two generic faces — the binary
    14	//! [`advance`], which hands each crossing to the caller's fold, and the N-ary
    15	//! [`advance_set`] over a walk's whole [`CursorSet`], which folds crossings

Resolution: required: correct overlay.rs:12-17 to name the statements that exist and why each does (two overlay faces; shape's `Refine` face, which folds exhaustion in; admit's fallible restatement), or drop the count and state the structure. Optional: implement `CursorSet` for `[W; N]` over `Refine` (priority `0..N`, depth `0` when done, step = `advance`) plus an all-done check at src/shape.rs:284 and :359, and delete `advance_refinement`. Acceptance: `just gate` clean; the public shape iterators' snapshot and differential tests pass; overlay.rs's count matches `grep -rn 'tied boundaries close to one shared flip level' crates/before/src`.

### skyline-coding-23: two test-local depth recursions sit outside recurse.rs's inventory and bypass `descend!`
- Where: crates/before/src/version/skyline/tests.rs:349-366 (related: crates/before/src/version/skyline/emit/tests.rs:510-519; crates/before/src/version/skyline/tests.rs:419-429; crates/before/src/recurse.rs:9-14; crates/before/AGENTS.md:32-36)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (grep `descend!` outside recurse.rs: testing/bridge.rs, grow/tests.rs, query/tests.rs, meter/tests.rs only; recurse.rs:9-14 read; tests.rs:421 feeds `Dense.packed1(1_000)` to `assert_flag_bijection`, so `walk` recurses 1000 deep today); executed: no
- Seen by: correctness; refutation: confirmed; history: contradicts-hard-rule (1ddb5a483 wrote the inventory naming two witnesses while both recursions already existed)
- Owner-gated: no

crates/before/AGENTS.md:32-36: a walk that must recurse routes each call through `crate::recurse::descend!`, and recurse.rs's module doc "holds the inventory". `inverted_flag_stream::walk` recurses on oracle-tree depth over the generator families (depth 1000 for `Dense.packed1(1_000)`) and `grid_version::build` over the grid's log depth; neither routes through `descend!` and neither is in the inventory, so the stated inventory is false and a deeper family fed to `assert_flag_bijection` would overflow before the rule's guard sees it. Medium because it breaches a stated hard rule; the depths reached today are bounded by the generators.

Evidence:

    tests.rs
    349	    fn walk(t: &oracle::Version, offset: &Base, prev: &mut Option<Base>, out: &mut BitsBuf) {

    359	            oracle::Version::Node(n, l, r) => {
    360	                out.push(true); // internal flag, inverted spelling
    361	                let offset = offset + n;
    362	                walk(l, &offset, prev, out);
    363	                walk(r, &offset, prev, out);

    emit/tests.rs
    511	    fn build(values: &[Base]) -> crate::oracle::Version {
    512	        match values {
    513	            [v] => crate::oracle::Version::leaf(v.clone()),
    514	            _ => {
    515	                let (l, r) = values.split_at(values.len() / 2);
    516	                crate::oracle::Version::node(0u64, build(l), build(r))

    recurse.rs
    11	//! surface, where the remaining depth recursion lives: the differential oracle
    12	//! bridge (`testing::bridge`), whose walks mirror the paper's recursive trees,
    13	//! plus the test-local recursive witnesses beside it (the grow suite's
    14	//! reference cost probe, the meter suite's segment-liveness dive).

Resolution: route both recursions through `descend!` as testing/bridge.rs does, or add both to recurse.rs's inventory with the depth bound stated at each site (grid_version already states `O(log)`; inverted_flag_stream should state it is bounded by the generator depths it is fed). Acceptance: recurse.rs's inventory and `grep -rn 'fn walk(\|fn build(' crates/before/src` agree on the set of test-local recursions, and each site either uses `descend!` or states its bound.

### skyline-coding-24: hand-maintained "depth-2" in two testdocs duplicates `EV_SMALL_DEPTH`
- Where: crates/before/src/version/skyline/tests.rs:477-478 (related: crates/before/src/version/skyline/tests.rs:570; crates/before/src/testing/exhaustive.rs:83; crates/before/src/version/skyline/emit/tests.rs:263-264)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (exhaustive.rs:83 `pub(crate) const EV_SMALL_DEPTH: usize = 2;`; the bodies at tests.rs:482 and 575 iterate `all_normal_events(EV_SMALL_DEPTH)`; emit/tests.rs:263-264 already says "to the small-scope depth"); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale
- Owner-gated: no

Principle 5: a literal that restates a constant the code can change without touching the prose.

Evidence:

    tests.rs
    477	/// Exhaustive small scope: flipping any single bit of any depth-2 normal form's
    478	/// encoding either rejects or round-trips to a different canonical value — no

    570	/// Exhaustive small scope: every normal-form tree to depth 2 round-trips,

Resolution: "of any normal form to the small-scope depth" (477) and "every normal-form tree to the small-scope depth" (570). Acceptance: no literal depth number in the two testdocs.

### skyline-coding-25: text.rs module doc misplaces the second pin and misstates the kernels' visibility; a testdoc possessive
- Where: crates/before/src/version/skyline/text.rs:30-35 (related: crates/before/src/version/skyline/text.rs:55-57; crates/before/src/version/skyline/text/tests.rs:492-493 and 498-499; crates/before/src/version.rs:26-29)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (text.rs:230 `pub fn render`, text.rs:494 `pub fn parse` in `pub mod text`; the schoolbook kernel lives at text/tests.rs:314 with its contrast under `#[cfg(feature = "limb-meter")] mod schoolbook_contrast` at 498-499, not in tests/meter.rs; version.rs:26-29 makes `skyline` `pub` under test/meter and `pub(crate)` otherwise); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired ("module-private" was true at e83ef6008; faf3cd0a routed Display/FromStr through the `pub fn`s and kept the word; 248d55397 wrote "pinned twice in tests/meter.rs" naming a pin that was already in text/tests.rs)
- Owner-gated: no

Prose must be accurate against today's code: a reader following the pointer to tests/meter.rs for the schoolbook kernel will not find it there, and "module-private" contradicts the `pub fn` items and their version.rs callers.

Evidence:

    text.rs
    30	//!   exact-top discipline is what keeps the walk linear, and it is
    31	//!   pinned twice in `tests/meter.rs`: the wide-arming flatness band
    32	//!   (per-byte touches flat across a size doubling on the wide-swing
    33	//!   family), and the committed schoolbook kernel beside this module's
    34	//!   tests — a known-bad twin that re-zeroes by compensating

    55	//! The kernels are module-private to the skyline codec (test- and
    56	//! meter-visible) and *are* the production text path: `Display` routes to

    text/tests.rs
    492	/// the text seam) while the shipped reset discipline stays flat per byte. Both
    493	/// leg's runs are value-pinned to the stored stream, so the contrast is an

Resolution: "pinned twice: by the wide-arming flatness band in `tests/meter.rs` ..., and by the committed schoolbook kernel in this module's tests (under `limb-meter`) ..."; lines 55-56: "The kernels are crate-private (public under test and meter) and *are* the production text path"; text/tests.rs:493 "Both legs' runs". Acceptance: each pointer names the file its pin lives in; the visibility wording matches the `pub` items and their callers.

### skyline-coding-26: dated incident narration and a meter-denominated constant at declaration sites in text.rs
- Where: crates/before/src/version/skyline/text.rs:147-161 (related: none)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale (742a02fc1 wrote both while curing the display heap reds; no commit chooses to keep incident narration at the declaration)
- Owner-gated: no

Principle 5: dated rationale at a declaration site is a ghost reference in disguise; state the invariant positively and leave when-and-why to git. Principle 3: a constant whose only stated justification is fitting a meter's allowance is the calibration-constant tell; if the board's allowance moves, this prose rots silently.

Evidence:

    text.rs
    147	/// How many parked entries one [`ParkedStack`] chunk holds: small enough that a
    148	/// shallow walk's single chunk sits inside the board's flat heap allowance,
    149	/// large enough that the chunk spine stays negligible.
    150	const PARKED_CHUNK: usize = 64;

    158	/// a doubling `Vec`, never holds an old and a new buffer at once during growth:
    159	/// that realloc coexistence spike is exactly what pushed the deep left-full
    160	/// shapes over the board's heap ceiling, and a chunk never moves once
    161	/// allocated.

Resolution: lines 158-161: "never holds an old and a new buffer at once during growth, so a deep left-full shape's peak transient is one chunk of slack, and a chunk never moves once allocated" (naming the board row that pins it is fine). Lines 147-150: derive 64 from a domain quantity (entry size against the per-level transient target) or state plainly that it is a tuning value whose live pin is the named board row. Acceptance: neither doc uses past-tense incident language; `PARKED_CHUNK`'s doc names a derivation or the committed pin that moves if the value changes.

### skyline-coding-27: `StoredLeft` is `Summary` minus `root`, copied field by field
- Where: crates/before/src/version/skyline/text.rs:292-296 (related: crates/before/src/version/skyline/text.rs:118-145 and 443-482)
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale (742a02fc1 documents why the root is dropped, not why the payload is copied rather than composed)
- Owner-gated: no

Two structs with identical bodies and a manual projection between them; composition expresses the relationship (a `Summary` is a root beside a `StoredLeft`) in the types and removes the copy.

Evidence:

    text.rs
    292	                    lefts.push(StoredLeft {
    293	                        drop: summary.drop,
    294	                        span: summary.span,
    295	                        incoming: summary.incoming,
    296	                    });

Resolution: `struct Summary { root: usize, body: StoredLeft }` (or rename `StoredLeft` to the payload it is); park `summary.body` and read `left.drop`/`span`/`incoming` through it in `merge`. Acceptance: `just gate` clean; text differentials unchanged.

### skyline-coding-28: the allocation A/B arm is compiled into the production renderer for a settled experiment
- Where: crates/before/src/version/skyline/text.rs:345-354 (related: crates/before/Cargo.toml:107; crates/before/benches/presize.rs:21; crates/before/benches/common/mod.rs:268-274; crates/before/src/version/skyline/query.rs:508-515; justfile:795-809; crates/before/src/version/skyline/text.rs:391-395)
- Class / severity / confidence: vestigial / low / medium
- Provenance: verified (grep `before_alloc_ab` locates every consumer as listed); executed: no
- Seen by: structure; refutation: confirmed; history: deliberate-and-holds (b28c35ad6 landed it as a record instrument and the site states its purpose), with the observation that the owner retired the sibling stacks seams with a DECIDED entry (1300ced09: "the seam existed to price the choice, and the choice is made") and no such entry exists for `display_growth`
- Owner-gated: yes: retiring an instrument, and a recorded design decision

Principle 3: machinery outlives the constraint that justified it. The shipped arm's exactness is asserted at text.rs:391-395, the bench records rather than checks, and the apparatus spans two production kernels, a check-cfg registration, a justfile recipe, and bench plumbing. Steelman: a re-runnable price of exact-request versus growth across allocators may be wanted; the arm is well fenced.

Evidence:

    text.rs
    345	    // Allocation-strategy seam: the shipped arm requests the exact size, one
    346	    // allocation, never grown (the assert below pins the exactness). The
    347	    // `before_alloc_ab` cfg — `RUSTFLAGS`-only, never a cargo feature, so no
    348	    // dependent build can select it — compiles in the growth-from-empty arm so
    349	    // the allocation benchmark can price the exact request against doubling
    350	    // growth on this site; shipped builds always take the exact arm.
    351	    #[cfg(not(before_alloc_ab = "display_growth"))]
    352	    let mut out = String::with_capacity(exact);
    353	    #[cfg(before_alloc_ab = "display_growth")]
    354	    let mut out = String::new();

Resolution: owner decision. Retire the `display_growth` arm (delete text.rs:351-354's cfg pair, the value from Cargo.toml:107's check-cfg list, the arm in presize.rs and benches/common/mod.rs, and the justfile case) with a DECIDED entry on 1300ced09's template, keeping the exact-size assert as the pin; or keep it and record at the check-cfg registration that the arms are a standing re-runnable record. Acceptance: if retired, `just gate` and `just ci` clean and `bench-alloc-ab`'s arm list matches the remaining cfg values; if kept, the rationale is stated once at the registration.

### skyline-coding-29: the render merge re-adds a wide `span` once per level; the class is published as "subquadratic"/"n log n" without an argument
- Where: crates/before/src/version/skyline/text.rs:443-482 (related: crates/before/src/version/skyline/text.rs:10-20, 211-225, 337-340; crates/before/src/version/skyline/signed.rs:282-291; crates/before/src/meter.rs:641-667; crates/before-fuelscape/src/ops.rs:324-337; crates/before/src/meter/board/ceilings.rs:409-430; crates/before/src/testing/asymptotics.rs:42-70)
- Class / severity / confidence: claim / medium / high
- Provenance: assessed (read: `signed_sum`'s same-sign arm is `&x + y`, allocating a `Base` of the wider operand's width; `wide_tail` is a right spine of zero leaves over one `2^b − 1` tail; the fuelscape `version_display` overlay families exclude WideTail; `grep -rn 'grows faster'` finds the sentence asymptotics.rs:45-46 quotes only in asymptotics.rs itself); executed: no
- Seen by: claims; refutation: confirmed (plus a second superlinear term: `entries.sort_unstable()` at text.rs:340 is Θ(k log k) in printed nonzero bases, documented as a mechanism at text.rs:219 and absent from every class statement); history: already-known (the superlinearity is a declared, owner-ratified model with `render_merge_superlinearity_is_alive` as its liveness floor); the class words "subquadratic" (2ee971421) and "n log n" (the fuelscape roster) are what is new
- Owner-gated: yes: reopens a ratified model

`merge` computes `span = entry_step + right.span` with `signed_sum`, whose same-sign arm allocates and adds at the wider width; on `WideTail(s, s)` every one of the `s` spine levels carries `span = +W` with `drop` and `lift` zero, so each merge does one `s`-bit add: Θ(s · ⌈s/64⌉) limb ops on `6s` input bits and Θ(s) output bytes, a quadratic leading term with constant 1/64. Every asymptotic claim needs an argument, a matching implementation, and an instrument that fails when false. (a) "superlinear, subquadratic time" (ops.rs:331) has no argument anywhere; ceilings.rs:416-418 points back at the `# Complexity` sections that carry the same string. (b) `a·n² + b·n` is not `o(n²)`. (c) The instruments pin weaker facts: the liveness floor proves the class exists, and `MIRROR_WIDE_RENDER_LIMB_EXPONENT_CEILING = 1.96` caps a fitted exponent at ladder scale, where a quadratic term beside a linear companion fits under 1.96, so the sentence "a genuinely quadratic conversion (~2.0) still reads red" is a snapshot, not a model. text.rs's module doc argues only the space pricing and never states the time class of its own kernel, and asymptotics.rs:45-46 cites a rustdoc sentence that no longer exists.

Evidence:

    text.rs
    457	    // The node's span: its last leaf is the right child's last.
    458	    let span = signed_sum(
    459	        entry_step.0,
    460	        entry_step.1.clone(),
    461	        right.span.0,
    462	        &right.span.1,
    463	    );

    signed.rs
    282	pub(super) fn signed_sum(x_sign: Sign, x: Base, y_sign: Sign, y: &Base) -> (Sign, Base) {
    283	    if x_sign == y_sign {
    284	        return (x_sign, &x + y);
    285	    }

    before-fuelscape/src/ops.rs
    331	        contract: "superlinear, subquadratic time; `O(|self|)` space",
    332	        claim: "n log n",

    ceilings.rs
    423	/// 0.15; the fitted exponents live in the pin commit), so a genuinely
    424	/// quadratic conversion (~2.0) still reads red. The model's under-side is not banded here: the class's

Resolution: owner ruling on a ratified model; two consistent options. Cure: carry `span` (and `drop`) up the spine instead of re-summing them at each level (fold the small `entry_step` into the moved child summary in place; an `Accumulator` per flowing summary keeps the fold amortized O(1) across carry cliffs, and only a printed base pays a magnitude read), measure at the parent on `SKYLINE_RENDER_*` and the mirror-wide cells, then retire the declared model and the liveness pin together as ceilings.rs:425-429 prescribes. Accept: replace "superlinear, subquadratic"/"n log n" with the derived class Θ(|self| + depth × max interior summary width + k log k) in ops.rs and the island, state both mechanisms in text.rs's render doc, delete the "genuinely quadratic still reads red" sentence, re-point asymptotics.rs:45-46 at a sentence that exists, and add the closed-form witness below so the instrument pins the order rather than a scale-bound fit. Acceptance: either the mirror-wide cells and `render_merge_superlinearity_is_alive` both read linear after the cure (the pin flips red and is retired in the same commit), or the `version_display`/`version_fromstr` contracts, the text.rs render doc, and the ceilings.rs prose all state the same derived class and a committed check `render_limb_ops(s) >= s * s.div_ceil(64)` holds at three doublings.
Construction: under `limb-meter`, for `s in [1024, 2048, 4096]` assert `render_limb_ops(s) as usize >= s * s.div_ceil(64)` (each of the `s` spine levels' span sum costs at least `⌈s/64⌉` limb ops); as an order witness, the doubling ratios `render_limb_ops(2s) / render_limb_ops(s)` increase with `s` and exceed 3.5 by `s = 4096`, where an `n log n` mechanism would hold near `2·(1 + 1/log2 s) ≈ 2.2`.

### skyline-coding-30: the reset-versus-compensating-subtraction argument is restated at full length three times
- Where: crates/before/src/version/skyline/text.rs:540-549 (related: crates/before/src/version/skyline/text.rs:21-37, 94-104, 552-557)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale (33ba5a34a wrote the module-doc bullet and the body comment in one commit; no commit chose one home)
- Owner-gated: no

Comments state what the code cannot show; a ten-line comment duplicating the module doc doubles the maintenance surface for one argument and buries the branch-local fact (which of `reset()` and `Accumulator::new()` runs, and why the `digit_count` threshold).

Evidence:

    text.rs
    542	        // below readies the accumulator for the next one. The re-zeroing is a
    543	        // reset — not a compensating subtraction of the extracted magnitude —
    544	        // which is what keeps the walk linear: it settles the digit buffer's
    545	        // top back to zero, so the next extraction pays the span written since
    546	        // *this* leaf, never a stale wide spelling left cancelling above it (a
    547	        // value-zero subtraction can leave the top parked at the widest swing,
    548	        // and every later leaf would re-walk those dead digits — the
    549	        // exact-`top` genre the wide-arming pins hold).

Resolution: keep the module-doc statement; at 540-549 leave only "A reset, not a compensating subtraction: the module doc's exact-top argument"; at 552-557 likewise "Past `PATH_SUM_KEEP_DIGITS` the buffer is dropped rather than reset (its doc)". Acceptance: text.rs states the exact-top argument once at full length; body comments point to it in one line each.

### skyline-coding-31: `validate_from`'s error list omits the `Decode::Io` arm the borsh cursor surfaces
- Where: crates/before/src/version/skyline/validate.rs:65-68 (related: crates/before/src/version/skyline/admit.rs:271-272; crates/before/src/borsh_impls.rs:88-97 and 141; crates/before/src/error.rs:91; crates/before/src/version/skyline.rs:218-220)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (borsh_impls.rs:88-92 `impl<R: Read> BitCursor for ReaderCursor` with `type Error = Decode`, :97 `.map_err(Decode::Io)?`, :141 calls `validate_from(&mut cursor)`; error.rs:91 `Io(io::Error)`); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (complete when only the slice cursor existed; faf3cd0a introduced `ReaderCursor` and routed the borsh leg here; admit.rs, written later, lists the arm from birth)
- Owner-gated: no

A maintainer-facing contract must state every return arm; the function is generic over `C: BitCursor` with `Decode: From<C::Error>`, the borsh `ReaderCursor` returns `Decode::Io` on a failed read, and admit.rs documents that arm for the identical bound.

Evidence:

    validate.rs
    65	/// Returns with the cursor just past the tree. Errors: running out of bits
    66	/// mid-tree or mid-code is [`Decode::Truncated`]; a collapsible sibling pair
    67	/// (an internal node's two leaf children with a zero right delta) or a delta
    68	/// driving the running leaf height negative is [`Decode::NotCanonical`].

    admit.rs
    271	/// - [`Decode::Io`]: the cursor's own reads fail (the wire-side
    272	///   cursor's genre; a slice cursor reports truncation instead).

Resolution: add the arm in admit.rs's words and, optionally, convert the inline "Errors:" sentence to a `# Errors` list matching admit.rs:261-272. Acceptance: `validate_from`'s doc names `Truncated`, `NotCanonical`, and `Io` with the slice-cursor caveat.

### skyline-coding-32: the path and phase stacks in validate, admit, and text ride the output build buffer instead of `BitStack`
- Where: crates/before/src/version/skyline/validate.rs:73-76 (related: crates/before/src/version/skyline/admit.rs:74-93; crates/before/src/version/skyline/text.rs:240, 355, 512; crates/before/src/codec/stack.rs:1-8; crates/before/src/codec/buf.rs:176-181)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`git show --stat 56a3dfe25` lists build/fill/fuse/query/sweep/walk and not these three files; 83e61b4d moved their stacks onto `BitsBuf`; codec/stack.rs's `pub(crate)` surface is new/len/push/trailing_ones/pop/set_last/last/all_set, so validate's per-ancestor pair becomes two pushes, not one `push_bits`; text.rs:239's `topology` is iterated at 357 and stays a `BitsBuf`); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale (the BitStack commit never touched these files; the later bitvec-to-BitsBuf migration was mechanical)
- Owner-gated: no

Two spellings of one concept across sibling kernels: `validate_from`'s `open`, `CheckedCursor`'s `path` and `left_was_leaf`, and text.rs's `phase`/`pending` stacks are `BitsBuf` (the byte-vector build form whose `pop` is `get` + `truncate`), while `LeafCursor`, `LeafWalk`, and `SkylineBuilder` keep the identical root-to-leaf path on `BitStack`, whose module doc names it as the stack "the deep walks keep their paths and phases on"; validate.rs's comment even says "A packed bit stack" over the buffer type. A reader auditing transient bounds learns two cost models; admit's separate `open_lefts` bookkeeping re-derives what `BitStack` could answer.

Evidence:

    validate.rs
    73	    // Two bits per open ancestor, pushed [left-complete, left-was-leaf] and
    74	    // popped in reverse order below. A packed bit stack, so depth costs bits,
    75	    // not frames.
    76	    let mut open: BitsBuf = BitsBuf::new();

    admit.rs
    78	    path: BitsBuf,
    79	    /// Per open ancestor: whether its completed left child was a leaf (a
    80	    /// placeholder `false` until that child completes).
    81	    left_was_leaf: BitsBuf,

    codec/stack.rs
    1	//! Pop-able stacks held as bits: the word-backed bit stack the deep walks keep
    2	//! their paths and phases on, and the nonnegative-integer stack built over it.

Resolution: switch the six stacks (validate.rs:76; admit.rs:78 and 81; text.rs:240, 355, 512) to `BitStack`; then stack.rs's claim is true of the tree. If the byte-granular `BitsBuf` is deliberately chosen for these walks' heap envelopes, say so at stack.rs's doc instead. Acceptance: `just gate` clean; the validate/decode/parse/render heap columns in tests/meter.rs hold within their ceilings, or move by a measured, attributed delta re-pinned from the parent.

### skyline-coding-33: two strict skyline parsers implement the same canonical-form obligations
- Where: crates/before/src/version/skyline/validate.rs:112-139 (related: crates/before/src/version/skyline/admit.rs:13-28, 36-38, 59-220; crates/before/src/version/skyline/tests.rs:169-183; crates/before/src/span/tests.rs:334-351 and 454-487; crates/before/tests/meter.rs:1427-1573)
- Class / severity / confidence: modularity / medium / medium
- Provenance: verified (grep confirms `validate_bits` is the only validator the planted-pair proptest calls and that `validate_dominating_from` is reached from span/wire.rs:153 and borsh_impls.rs; the obligation-by-obligation comparison is by reading); executed: no
- Seen by: structure; refutation: confirmed (with the caveat that the two transients differ, so unification moves the validate heap column pinned by the `DECODE_*` rows and needs a parent-commit re-pin); history: no rationale (admit.rs's "restated rather than reused" justifies restating the advance law on fallibility grounds, never the parse body; the Wave 6 dedup pass touched this pair and added cross-citations without recording a reason for two bodies)
- Owner-gated: no

`validate_from` (the `Version::decode` path) and `admit::CheckedCursor` (the `Span::decode` and borsh span path) each implement the unary descent, the per-ancestor left-was-leaf bookkeeping, the collapsible-pair test at every close, and the exactness bookkeeping, as separate bodies over different stack layouts (one interleaved two-bits-per-ancestor buffer versus two parallel buffers plus an `open_lefts` counter). The only obligation `CheckedCursor` lacks is the height accumulator. The hard rule makes a validator gap an equality bug, so two validators means a gap closed in one can persist in the other; skyline-coding-6 is that asymmetry made concrete. Principle 3 and the legibility bar: one strict parser reads as evidently right; two invite drift.

Evidence:

    validate.rs
    131	            // The completed subtree was the right child: the ancestor
    132	            // closes. Two leaf children with a zero right delta are the
    133	            // collapsible pair minimal topology prohibits.
    134	            if left_was_leaf && is_leaf && leaf_zero_delta {
    135	                return Err(Decode::NotCanonical);
    136	            }

    admit.rs
    153	        if left_was_leaf && *is_leaf && *zero_delta {
    154	            return Err(Decode::NotCanonical); // a collapsible sibling pair
    155	        }

    36	//! overlay-advance law is restated at this cursor pair — restated rather than
    37	//! reused, because the generic law is infallible and the checked side's
    38	//! crossings are `Result`s — with the same step and fold order as

Resolution: make `CheckedCursor` the one strict parsing cursor (move it to validate.rs or a shared sibling; admit imports it) and express `validate_from` as: open a `CheckedCursor`, fold the first payload positively into a height `Accumulator`, loop `while !done { step()?; fold; if the step's sign is Negative and height.sign() is Less, return NotCanonical }`, then `finish()`; the touch sequence (fold, then a sign read only after a negative delta) is unchanged. Alternatively adopt `validate_from`'s interleaved single-stack layout inside `CheckedCursor` first, so the admission walk's transient shrinks by one buffer. Either way, extend the planted-pair proptest to drive the admission entry too (skyline-coding-6). Acceptance: `just gate` clean; the tests.rs reject corpus, the span/tests.rs admission genres, the tests/fuzz_seeds.rs Span rejects, and the borsh span tests pass unchanged; the `DECODE_*`/`SKYLINE_DECODE_*` envelopes and the span scan/touch legs in tests/meter.rs hold, or any heap movement from the layout change is measured at the parent and re-pinned with attribution.

### skyline-coding-34: walk.rs's module doc enumerates a client roster that has rotted
- Where: crates/before/src/version/skyline/walk.rs:4-6 (related: crates/before/src/version/skyline/grow.rs:349; crates/before/src/version/skyline/query.rs:639)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep `LeafWalk::new`: grow.rs:349, query.rs:639, fill.rs:1019 and 1092, fill/prescan.rs:475, 511, 556, fill/fuse.rs:219); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale (the roster was written by 248d5539 a week after e1905a3f0 put `LeafWalk` under grow.rs and query.rs)
- Owner-gated: no

Principle 5: an enumeration of callers rots silently when a new client arrives; the tick splice's subtree locator and `max_depth` also drive `LeafWalk`.

Evidence:

    walk.rs
    4	//! Its clients are the scanning walks that visit one skyline subtree's leaves
    5	//! through a caller-owned cursor — the fill walk's and its pre-scan's block
    6	//! scans and sibling walks.

Resolution: keep the structural clause and drop the enumeration, or make it explicitly non-exhaustive. Acceptance: the doc names no client list, or every listed client matches `grep -rn LeafWalk::new crates/before/src`.

### skyline-coding-35: a `# Panics` paragraph copied five times in walk.rs while claiming to be "stated once there"
- Where: crates/before/src/version/skyline/walk.rs:74-79 (related: crates/before/src/version/skyline/walk.rs:238-243, 284-289, 317-322, 345-350; crates/before/src/version/skyline/emit.rs:321-323; crates/before/src/version/skyline/literal.rs:75-76; crates/before/src/version/skyline/text.rs:626-627)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read: the six-line paragraph is identical at the five sites; the `Version::from_bits` sentence appears at the three emitter sites); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-and-holds (a736ef14a adopted the uniform per-function paragraph pointing at `causal_cmp`'s canonical statement), so this is a consolidation proposal, not a contradiction
- Owner-gated: no

Every sentence competes with the contract the reader came for; five copies of one paragraph on `pub(super)` items bury the per-function facts (what `pending` means, why `first: false`) that are those docs' actual content. A smaller triplet of the same kind: "is `Version::from_bits`'s job, the single gate a stream passes through when it becomes a stored value" at emit.rs:321-323, literal.rs:75-76, and text.rs:626-627.

Evidence:

    walk.rs
    74	    /// # Panics
    75	    ///
    76	    /// The stream must be canonical. The violations this walk structurally
    77	    /// notices — truncation, malformation — panic; the rest walk silently
    78	    /// with an unspecified result (the contract of
    79	    /// [`causal_cmp`](super::sweep::causal_cmp), stated once there).

Resolution: state it once in walk.rs's module doc and reduce each function's section to `# Panics` plus one line ("Canonical input required; see the module doc"), keeping the uniform section header; consider the same for the `from_bits` triplet (one statement at the builder's `finish()` doc, cited from the three emitters). Acceptance: walk.rs contains the canonical-input contract once in prose; each function keeps a one-line pointer.

### skyline-coding-36: `Extremum`'s reset policy keys off direction while its contract is about buffer provenance
- Where: crates/before/src/version/skyline/walk.rs:178-190 (related: crates/before/src/version/skyline/walk.rs:124-131, 144-166, 358; crates/before/src/version/skyline/fill.rs:1091; crates/before/src/version/skyline/fill/prescan.rs:555)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep `Extremum::max`/`Extremum::min`: `max` over `self.web.lease()` at fill.rs:1091 and prescan.rs:555 only; `min` over `Accumulator::new()` at walk.rs:358 only); executed: no
- Seen by: structure; refutation: confirmed; history: deliberate-and-holds (a736ef14a wrote the two policies and the provenance contract, stated at walk.rs:124-131 and on both constructors), so this is a structural-tightening suggestion, not an expired decision
- Owner-gated: no

The match dispatches on direction while the doc explains the arms by provenance and warns in prose ("Hand each constructor the provenance its doc names, or the stated costs invert"); in the tree the axes are perfectly correlated, so `min`'s register parameter can only ever be a fresh `Accumulator::new()`, and a parameter with one possible value is an invitation to pass the wrong one. Making the coupling a construction fact removes the prose warning.

Evidence:

    walk.rs
    184	        if self.register.sign() == overtaken {
    185	            match self.direction {
    186	                Direction::Max => self.register.reset(),
    187	                Direction::Min => self.register = Accumulator::new(),
    188	            }
    189	        }

    130	/// digit a wide swing left. Hand each constructor the provenance its doc names,
    131	/// or the stated costs invert.

Resolution: have `Extremum::min()` take no register (it always owns a fresh `Accumulator::new()`), or carry the policy as its own field (`Provenance::{Pooled, Owned}`) and match on it in `fold_armed`; delete the "Hand each constructor..." sentence once the code enforces it. Acceptance: `just gate` clean; fill/prescan differentials and the pool-traffic/touch envelopes unchanged.

## Positives

- build.rs's absorb face is a genuinely elegant amortized-O(1) design (truncate one parent flag per level around a held code that never moves), argued at the code and pinned by hand-derivable bit-level tests per collapse genre; `ZERO_DELTA_CODE_BITS`'s doc names the one other 1-bit code and argues why it can never fire a collapse, and `zero_delta_has_the_lone_shortest_code` pins the cross-module coupling as a red test rather than a convention.
- text/tests.rs's `parse_schoolbook` is a committed known-bad kernel doing double duty: the independent second seat of the twin-parity differentials, and the demonstration that the wide-arming flatness criterion is non-vacuous (asserted red at ≥×1.5 per byte while the shipped kernel stays ≤×1.25). This is "every criterion needs a committed demonstration that a known-bad mechanism fails it", done properly.
- admit.rs states the height-subsumption argument once, cites it by name at every later mention, warns callers that `Refuted` doubles as a rejection surface, and its fused-parse claim is pinned relationally (fused == standalone-lo-parse + comparison on scan bits and touches), so there is no constant to rot. The advance restatement matches `overlay::advance` arm for arm, and its done-cursor argument holds.
- validate.rs is exactly its module doc's contract: two bits per open ancestor, one accumulator whose sign is read only after a subtracting fold, no panic reachable from bytes; tests.rs covers every reject genre with its exact variant, sweeps truncation at every cut point, and `assert_mutation_never_aliases` explains precisely why only an oracle re-derivation can convict a lax validator (decode adopts bytes verbatim).
- emit.rs makes join and meet differ in exactly one pick function, shares the switch algebra, and is witnessed three independent ways (oracle bytes, a three-cursor pointwise walk, the lattice laws on emitted streams) over families, the exhaustive small scope, arbitrary pairs, organic histories, and the 29..=34-bit grid straddling the fused-gamma guard.
- The gate runs the internal doc build (`just gate` → `gate-streams` → `docs-internal`, justfile:400-470), so emit.rs's `#![allow(rustdoc::private_intra_doc_links)]` is backed by a check that holds the private links against rot, as its comment claims; I verified this rather than taking the comment's word.
- Small comments that state what the code cannot show: decode.rs:16-17 (why one spare bit of capacity), validate.rs:96-98 (why the first leaf's `zero_delta` stays false), emit.rs:292-301 (why the capacity is an estimate and what bounds the miss), text.rs:404-412 (why `node_index` may abort), walk.rs:124-131 (the two reset policies and the cost inversion that follows from violating the provenance contract).
- Every test in the partition carries a doc comment stating its invariant, the comments read accurately against their bodies (with the three exceptions in skyline-coding-13), and tests live in sibling files throughout.

## Open questions for Finch

1. Re-anchor quadratic (skyline-coding-9): cure with the two-stream builder, or accept the class and restate the join/meet/span contracts? Recommendation: cure. `O(|self| + |other|)` for join is the crate's headline guarantee and lib.rs makes every such claim a hard promise; the two-stream design also dissolves the held-leaf discipline, `lens`, and `extract_code`. Land the two-scale scan-bit pin on `join(Hugeleaf, spine_of_pairs(d))` first, red, then the cure.
2. Render class (skyline-coding-29): cure the per-level wide-span re-fold, or restate the class? Recommendation: restate now (delete "subquadratic"/"n log n", state Θ(|self| + depth × max interior summary width + k log k), replace the fitted-exponent ceiling with the closed-form witness) and schedule the cure; the model is ratified and the instruments are committed, so the doc-and-instrument change is the cheap consistent step.
3. Validator unification (skyline-coding-33) and the fused-door planted-pair proptest (skyline-coding-6): unify on `CheckedCursor` and extend the family to both doors, accepting a heap-column re-pin? Recommendation: yes; land the proptest first (it is red-capable today against a weakened step arm and green against the shipped code), then unify.
4. The `display_growth` allocation arm (skyline-coding-28): retire with a DECIDED entry on 1300ced09's template, or record it as a standing re-runnable record at the check-cfg registration? Recommendation: retire; the question it answers is settled and asserted.
5. Test-suite inventories (skyline-coding-3): keep the inventory in the tests module doc and reduce the kernel doc to invariants plus a pointer? Recommendation: yes, matching `validation_index.rs`.
6. Vocabulary at crate root (skyline-coding-1): should "door" be defined once by contrast or replaced by "entry point" crate-wide, and should "min-lifted" (anchored only at literal.rs:29-31) be defined once at the crate root or the oracle module? Recommendation: replace "door" (the style guide's own translation table lists it), and anchor "min-lifted" once at the crate root since the meter and bridge modules use it too.
7. Cross-partition items surfaced here, for their owners: the `implementation` ghost at crates/before/AGENTS.md:6 and examples/code_study.rs:6; asymptotics.rs:45-46 quotes a rustdoc sentence ("summary-merge cost that grows faster than the operand") that grep finds nowhere else in the crate; overlay.rs:12-17's "exactly two generic faces"; span/tests.rs:338's collapsible witness carries no padding marker after its 5-bit tree and passes only because `NotCanonical` fires before `require_marker_padding`; the fuelscape roster's `version_display` contract strings; `tests/meter.rs:411`'s `version_of` copy.

## Dropped

- The cited `meter::tier2` plain-sweep pin does not exist ([35]): refuted. The pin exists at crates/before/src/meter/tier2/tests.rs:239 (`cliff_comb_plain_delta_sweep_is_quadratic_in_tier2_wire_bits`, cfg `limb-meter`, added 7c3677192 before either citation was written); the lens's regex could not match `plain_delta_sweep` and its tier2 grep read tier2.rs, not tier2/tests.rs. Both citations resolve. The residual (the demonstrator drives a standalone `Base` over the comb's deltas rather than the validator's code path) is a strengthening, not a gap, and below the bar.
- Production assert on the render's exact sizing ([41]): deliberate and documented (e83ef6008; text.rs:221 and 345-350), an O(1) probe of the kind the doctrine's Assertions and Guards clause explicitly admits, and the only exact pin of `exact` under the `display_growth` arm; whether a sizing slip should panic `Display` in release is a severity judgment for the owner, not a finding.
- Ghost `implementation` essay ([30], [42]): duplicates of skyline-coding-14.
- Nested literal construction re-scans every subtree per level ([37]): duplicate of skyline-coding-20.
- Test files spell `crate::codec::built_view` 69 times and triplicate `version_of` ([26]): duplicate of skyline-coding-8.
- "mint" at skyline.rs:7 (half of [15]): the coinage sense, permitted under the owner's dissolve/link/mint ruling (a736ef14a: "skyline.rs carries a plain gloss naming overlay as the minting site"); only admit.rs:256 survives as skyline-coding-7.
- Whether `just docs-internal` is in the gate (prose lens open question): resolved by reading justfile:400-470; it is, so emit.rs's allow comment is accurate and no finding follows.
