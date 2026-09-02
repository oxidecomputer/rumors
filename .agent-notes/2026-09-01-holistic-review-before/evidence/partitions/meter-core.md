# Partition meter-core: The meter module: adversarial generators and deterministic resource meters, with its tests

## Partition summary

`crates/before/src/meter.rs` (3726 lines) is the crate's instrument library. It holds about seventy private generators that build adversarial inputs in closed form (the dense spine, the boundary combs, the memo probes, the freeze and promotion families, the seam and ladder shapes, the multi-operand populations), the `Packed` output type with its `version()` door into the stored skyline coding via `skyline::encode_bits`, and the public counter readers (`stack_segments`, `limb_ops`, `densified_digits`, `touch_ops`, `span_traffic`, `emit_traffic`, `pool_misses`, `scan_bits`) that the envelope suite, the amplification board, and the rumors crate's metering tests read. `crates/before/src/meter/tests.rs` (1648 lines; the partition's only test file) pins a subset of the generators (closed-form length, strict wire round-trip, and a semantic leg such as `min_ticks`, an exact rank, or a text spelling) and witnesses the counters' liveness, reset, and determinism. Every generator is reached from exactly one arm of the registry's exhaustive `Shape::builder` match (registry.rs:309-379), which is the compile-time tie that keeps the roster and the generators in step.

The quality of what is pinned is high. Each generator states its parameter contract under `# Panics` and enforces it with an `assert!` whose message reads as the proof; the one non-obvious frontier (the ascend-cliff code band) is proptested from both sides. The pinned families carry a semantic leg independent of the construction: `min_ticks` stored-base sums, the harmonic telescoping rank `1 - rank(H(d)) = 1/2^d` at depth 4096, `plateau_puncture`'s answer-embedded rank `(2xy + 1)/2^(66d)` beside a pin of its stored size, and readable text spellings for the re-arm, gap-spine, parked-spine, and tooth-tail families. The counter witnesses floor on irreducible work (a strict decode reads every live bit; a join writes at least its output's bits) and every process-global comparison carries `ISOLATION_NOTE`. `meet_shade` documents why its shades are built in distinct buffers so the flatness band measures the byte compare and not the folds' clone-identity collapse, which is the worst-passing-artifact question asked and closed at the generator.

The dominant issues are three verification and documentation gaps that share one cause: the module's prose was written for a world where the construction language was the stored coding, and the flag-day change to skyline storage was never carried through here. First, the module doc says every generator is pinned by this module's tests and round-trips through `Version::decode`; twenty registry-dispatched generators have no pin at all, four of their stated exact sizes are wrong, and six memo-family event shapes reach every consumer through the unvalidating `Packed::version()` with no strict decode anywhere. Second, the `Packed` doc says `bytes` is what `decode` accepts, which is false for event shapes, and the `cliff_comb` and `wide_tooth_comb` funding arguments describe the stored coding as a hypothetical. Third, the stack-segments meter has no writer in any binary that reads it under the `meter` feature, so the envelope column and the board currency it feeds are compiled-in zeros. Beneath those sit two regime-claim problems: six constants copy the query fold's freeze allowance by hand with no binding test, and the `wide_arming`/`hoisted_window` guard admits widths at which the promotion their docs describe cannot fire. The remainder is duplication, a handful of doc-versus-code arithmetic slips, and prose nits.

Total lines read: 5374 in the partition, plus the anchors cited under each finding in recurse.rs, encode.rs, version.rs, query.rs, query/integral.rs, query/web.rs, board/family.rs, board/measure.rs, board/currency.rs, board/judge.rs, board/floors.rs, board/ceilings.rs, registry.rs, tests/meter.rs, Cargo.toml, gamma.rs, signed.rs, skyline.rs, ticks.rs, and the rumors crate's `src/tree/typed/untyped/tests.rs`. No cargo, just, or build command was run; every numeric claim below is a hand derivation from the code as read.

## Findings

### meter-core-1: Module-doc and comment prose: unanchored coinage, duplicated sentence, history in prose, "mint", em-dashes in `//` comments
- Where: crates/before/src/meter.rs:18-27 (related: meter.rs:3-4 and 12-13; meter.rs:1220-1221, 1383, 2673, 2687, 3614; meter/tests.rs:411, 441-443; `//` em-dashes at meter.rs:1241 and meter/tests.rs:369, 442, 1011, 1085, 1208, 1210; registry.rs:572-583)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep of each phrase over both partition files and registry.rs; sites read); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed (with the "names nothing" part corrected); history: no rationale found; the coinage was minted in fe0aa9ef3 pointing at a list that now lives in the `FamilyId` doc
- Owner-gated: no

The vocabulary rule wants every coined term anchored to an identifier or defined once, "mint" never used for construction, and history kept out of declaration-site prose. Line 26's "the luck-proof touch list" points at a real list (registry.rs:572, "What the compiler cannot force, in the order it is otherwise found by luck") but under a name that appears nowhere else, so a reader cannot grep their way to it; "mints"/"minted"/"mint" appear at 18, 1383, 2673; "earns a column" at 24; lines 3-4 restate the feature-gating sentence at 12-13; "the refuted live-anchored followers' tombstone" (1220-1221), "honestly refuses" (2687), "no longer sees" (3614), tests.rs:411 "so its size is honest", and tests.rs:441-443 "the descent that once grew the stack — and must now read zero" narrate history or moralize code; seven `//` comments carry true em-dashes where the owner's doctrine wants spaced double hyphens.

Evidence:

        18	//! The generators themselves are private: every instrument mints its shapes
        24	//! additionally earns a column on the amplification board ([`board`]) only when
        26	//! criterion, each family's coverage answer, and the luck-proof touch list sit
        27	//! on the registry's [`FamilyId`](crate::meter::registry::FamilyId)).

         3	//! Public under the `meter` feature so the metering test binaries can drive
         4	//! them.
        12	//! against. Public under the `meter` feature so the metering test binaries (and
        13	//! benches) can reach it; never part of a production build. (The proptest

      1220	/// folds all `d` per drop (quadratic), the refuted live-anchored followers'
      1221	/// tombstone. Normal form: leaf pairs `(0, i + 1)`, the run's unit-step

      3614	/// counter no longer sees on narrow values: a word-scale fold rides the

    tests.rs
       441	    // The conversion ratchet: the fill walk pairs a deep spine on BOTH
       442	    // sides — exactly the descent that once grew the stack — and must now
       443	    // read zero, its depth on explicit heap stacks the heap meter prices.

Resolution: Point line 26 at the registry's own phrasing ("the roster entries the compiler cannot force, listed on `FamilyId`") or give that paragraph a heading and cite it; delete lines 3-4; "mints" to "builds", "minted" to "created", "earns" to "gets"; restate 1220-1221 as the positive mechanism ("one live record per open level folds all `d` per drop, quadratic"); "honestly refuses" to "refuses"; "no longer sees" to "does not see"; tests.rs:411 to the mechanism ("a stack-resident payload so each frame has nonzero size"); tests.rs:441-443 to "the fill walk pairs a deep spine on both sides and must read zero: its depth lives on explicit heap stacks"; swap the seven `//` em-dashes for colons or double hyphens. Acceptance: `grep -nE 'luck-proof|\bmint|honest|tombstone|refuted|no longer|once grew|earns' crates/before/src/meter.rs crates/before/src/meter/tests.rs` is empty; `grep -nE '^[^/]*//[^/!].*—'` on both files is empty; the module doc states the feature gate once.

### meter-core-2: Twenty registry-dispatched generators have no size or canonicality pin; four exact closed forms are wrong; six event shapes never meet a strict decode
- Where: crates/before/src/meter.rs:29-32 (related: meter/tests.rs:1-2, 10-19; meter.rs:641-642, 659, 669-670, 688, 973-975, 1091-1096, 1131-1133, 1157, 1258-1263; registry.rs:311-378; version.rs:1192-1201; encode.rs:19-23, 54-58; board/family.rs:765-786, 1232-1234; tests/meter.rs:411-413; fill/tests.rs:47-49, 138-149; query/tests.rs:36; tests/verdict_matrix.rs:329-353)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep -cw of each name in meter/tests.rs returns 0, the lone `staircase` hit at 731 being prose; registry dispatch, `Version::from_bits`, `encode_bits`, `version_of` at every consumer, and the board's envelope-only arm read; closed forms re-derived by hand from each emission sequence and gamma.rs:28's width rule); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed (scope reframed: only the six memo-family event shapes meet no strict decode anywhere; the board re-decodes WideTail, Staircase, RevealComb, RevealCombHifloor, PureComb, AscendCliff); history: no rationale found (the claim was true at 6014124ad, whose message records the pin catching an off-by-one in `bigroot`; the 07-25/26 generator wave landed without pins)
- Owner-gated: no

The module doc states that every generator's output round-trips through strict decode and that its exact bit length is a closed formula pinned by this module's tests. Neither holds for `nested_full_id`, `nested_left_full_id`, `wide_tail`, `staircase`, `memo_chain`, `memo_chain_id`, `memo_comb`, `memo_comb_id`, `memo_fanout`, `memo_oscillating`, `memo_churn`, `memo_churn_id`, `descending_raises`, `descending_raises_id`, `reveal_comb`, `reveal_comb_hifloor`, `reveal_comb_id`, `pure_comb`, `pure_comb_id`, or `ascend_cliff_id`: none is imported or called in tests.rs. Re-deriving each emission with `gamma(n)` costing `2*floor(log2(n+1)) + 1` bits: `wide_tail` emits `4d + 2b + 2` (doc `4d + 2b + 3`; the doc's own layout line sums to `4d + 2b + 2`), `memo_chain(k, false)` emits `10k + 6` (doc `13k + 9`), `memo_comb_id` emits `14d + 8` (doc `14d + 12`; its layout line sums to `14d + 8`), `memo_churn_id` emits `10d + 4` (doc `14d + 6`; its layout line sums to `10d + 4`), `staircase` is exactly `6d + 2` (doc `~5d`; the `5d + 8` capacity hint under-allocates from `d = 7`), and `memo_fanout` is `k(4b + 6) + 6` (doc `~(13k + 2kb + 9)`, half the leading term; the capacity hint `13k + 4b + 9` has no `kb` term at all). Every consumer lifts event shapes through `Packed::version()`, which is `Version::from_bits(encode_bits(..))`: `from_bits` adopts without validation ("Callers guarantee canonical skyline form") and `encode_bits` asserts only clean parsing and full consumption, never minimal topology. The six memo-family event shapes are envelope-only (family.rs:770-775 is an `unreachable!` arm), so they never pass the board's `decode_version` either; they are measured in tests/meter.rs, fill/tests.rs, query/tests.rs, and tests/verdict_matrix.rs with no check that the stream is one `Version::decode` would accept. This breaches Principle 6 (the cheapest artifact satisfying the proxy must be the intended one) and Principle 5 (the doc claim is false as written).

Evidence:

        29	//! Every generator output is strict normal form: it round-trips through
        30	//! [`Party::decode`](crate::Party::decode)/[`Version::decode`](crate::Version::decode)
        31	//! and re-encodes byte-identically, and its exact bit length is a closed
        32	//! formula in the parameters (pinned by this module's tests). Normal form is

       641	/// A right-leaning spine of zero leaves with one `2^b − 1` tail leaf: depth
       642	/// `d`, `4d + 2b + 3` bits.

       973	/// The memo-chain event `Q(k, distinct)`: a right-leaning spine of `k`
       974	/// single-leaf left-full sites, `~(14k + 9)` bits distinct (γ(j) codes), `13k +
       975	/// 9` shared.

      1091	/// The memo-comb id over [`memo_comb`]: a covering `(1, ·)` site per level
      1092	/// interleaved with the single-leaf sites' `(1, (1, 0))`, `14d + 12` bits.

      1258	/// The memo-churn id over [`memo_churn`]: a covering `(1, ·)` root, per level
      1259	/// the site's `(1, (1, 0))` under a carrier whose right arm continues, and an
      1260	/// absent id over the descending run, `14d + 6` bits.

    version.rs
      1196	    /// Callers guarantee canonical skyline form; the freeze seals the
      1199	    pub(crate) fn from_bits(bits: codec::BitsBuf) -> Self {
      1200	        Version(codec::Bits::freeze(bits))

Resolution: Add `check_version`/`check_party` pins for all twenty at two sizes, correcting the six closed forms above (spell the gamma-length sums out as `hole_region_bits` already does; `staircase` is exactly `6d + 2`) and fixing the capacity hints; then add one roster-wide pin that walks every `Shape` variant through the matching `check_*` so a new constructor cannot enter `Shape::builder` unpinned. Consider additionally routing `Packed::version()` through `Version::decode` of the transcoded bytes under `cfg(any(test, feature = "meter"))`, since construction happens before counters are reset and the only cost is one validation pass per shape. Acceptance: every arm of `Shape::builder` names a generator that appears in a meter/tests.rs pin asserting its `bits` against a closed form and round-tripping the shape; the construction below fails a committed test; the module doc's sentence at 29-32 is true of every variant.

Construction: Change `memo_chain`'s range leaf at meter.rs:1005 from `if distinct { j as u64 } else { 1 }` to emit `0` in the shared case, so every site is the equal sibling pair `(0, 0)`. `cargo nextest run -p before --lib meter::tests` stays green (nothing calls `memo_chain`), and the `memo_chain_shared_*` rows in tests/meter.rs and the fill differential at fill/tests.rs:140 build and measure the stream, which `Version::decode` would reject. Apply the same mutation to `dense` (make the bottom pair `(0, 0)`) and `dense_decodes_canonically_at_predicted_length` fails inside `check_version`, showing the pin is what catches it. For the closed forms, `assert_eq!(wide_tail(1, 1).bits, 4 + 2 + 3)` fails with 8; `assert_eq!(memo_chain(1, false).bits, 13 + 9)` fails with 16; `assert_eq!(memo_comb_id(1).bits, 14 + 12)` fails with 22; `assert_eq!(memo_churn_id(1).bits, 14 + 6)` fails with 14; `assert_eq!(staircase(7).bits, 5 * 7)` fails with 44.

### meter-core-3: `Packed` documents one coding but carries two; event-shape `bytes` are the construction language, which `Version::decode` cannot accept
- Where: crates/before/src/meter.rs:85-96 (related: meter.rs:6-9, 29-31, 116-121, 2286-2288; encode.rs:9-17, 38-41; meter/tests.rs:30-37, 1207-1219; tests/meter.rs:328-346, 433-434, 446, 459, 472, 1067-1070, 1075, 1089, 8397)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (encode.rs read: the transcoder inverts the flag and re-codes leaves as absolute-then-zigzag deltas; `check_version` decodes `v.encode()`, never `p.bytes`; construction-versus-stored sizes derived by hand: dense(1000) 4004 vs 3006 bits, cliff_comb(1000, 1000) 2,010,002 vs 14,002 bits); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate but expired (written in 6014124ad when the construction language was the stored coding; faf3cd0a made the skyline the stored coding and added `Packed::version`; d800957e re-edited the sentence without correcting it; the 205a361da ghost sweep did not touch meter.rs)
- Owner-gated: no for the doc; yes for the type split below (the meter surface is public under a feature)

The `Packed` doc's first sentence, and lines 87-88, say `bytes` "is what `decode` accepts and `encode` reproduces". For every event-shape generator the bytes are the construction language: flag `1` = internal (the skyline flags `0` internal), one `gamma(base)` per node, which `encode_bits` transcodes rather than decodes. Only id shapes' bytes decode directly. The same type carries both kinds with no discrimination, so `Shape::IdSpine.packed_flagged(5, false).version()` compiles and feeds an id wire to the event transcoder. `bits` is the construction stream's length, which differs asymptotically from the stored size (the module's own `plateau_puncture` doc at 2286-2288 and the pin at tests.rs:1207-1219 say so). Downstream, tests/meter.rs denominates the skyline `cmp_*`/`join_*`/`tick_dense` rows by `p.bytes.len()` while the kernel reads `version_of(&p)`, so the printed `MEASURED` input for `cmp_cliff` is about 143 times the operand the sweep read, and the `heap_meter_floor_on_decode_dense` floor asserts `peak >= p.bytes.len()` on the premise that "the decoded version owns a copy of the packed bits", whereas the version owns 3006 stored bits against 4004 construction bits and the floor passes only because `encode_bits` allocates `with_capacity(bits.len())` (encode.rs:24). Those downstream sites belong to the envelope partition; the false contract that leads there is this type's doc. This breaches the documentation-accuracy rule (a first sentence that is false for half the values the type holds) and, downstream, the doctrine that a liveness floor derives from irreducible work.

Evidence:

        85	/// A generator's output: canonical packed bytes plus the exact bit length.
        86	///
        87	/// `bytes` is what `decode` accepts and `encode` reproduces
        88	/// (marker-padded to a byte boundary); `bits` is the live bit length
        89	/// before that padding, so tests can pin the closed-form size of each
        90	/// shape.

    encode.rs
        38	        // The construction language flags `1` internal; the skyline stream
        39	        // flags `0` internal (`1` leaf), so the flag inverts at this transcode
        40	        // boundary.
        41	        out.push(!internal);

    tests/meter.rs
       330	/// The decoded version owns a copy of the packed bits, so the one big
       341	        peak >= p.bytes.len(),

Resolution: Restate the `Packed` doc as it is: for id shapes `bytes` is the crate codec and decodes directly; for event shapes `bytes` is the construction language and `version()` is the door to the stored form, whose wire then round-trips; `bits` is the construction stream's live length (the denominator the size pins use), not the stored size. Say in the module doc (6-9, 29-31) which coding each closed form denominates. Owner-gated option: split `Packed` into an id type and an event type (or an enum) so `version()` exists only for event shapes and the registry's `Builder` arms carry the distinction. Hand the envelope partition the two consequences: denominate skyline rows by `version_of(&p).encode().len()` as the `decode_*` rows and `tick_run` already do, and derive the decode-dense floor from the stored size. Acceptance: no sentence in meter.rs claims `Version::decode` accepts an event generator's `bytes`; the `Packed` doc names the two codings and which one each field measures; if split, `Shape::IdSpine.packed_flagged(..).version()` does not compile.

Construction: `assert!(Version::decode(&dense(3).bytes[..]).is_ok())` fails (the stream's first bit is `1`, which the skyline reads as a leaf flag over a topology that does not parse as one leaf); `assert_eq!(dense(1000).bits as u64, dense(1000).version().encoded_bits())` fails with 4004 against 3006.

### meter-core-4: The `cliff_comb` and `wide_tooth_comb` funding arguments describe the construction coding as "this coding" and the stored delta coding as a hypothetical
- Where: crates/before/src/meter.rs:220-224 (related: meter.rs:307-309; skyline.rs:102-110, 114; tests/meter.rs:7-9, 1067-1070)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (skyline.rs:102-114 read; the two passages read; stored-versus-construction per-crossing widths derived by hand: 3 bits per crossing stored, `2k + 1` per tooth constructed); executed: no
- Seen by: instrument-correctness; refutation: confirmed (git log -S dates the phrase to 7c3677192, 2026-07-23, before the storage flip); history: deliberate but expired (accurate when written; faf3cd0a decided the question; the 205a361da ghost sweep did not cover meter.rs or tests/meter.rs)
- Owner-gated: no

The comb's doc says that in "this coding" each tooth stores its own `gamma(2^k - 1)` so operations stay linear per input bit, and that "a delta coding of the same tree stores 3-bit ±1 codes per crossing instead, which is what makes this the separating family for the leaf-delta representation question". The stored skyline is that delta coding: skyline.rs:102-106 says the comb's payload stream is 3-bit codes on a `2^k` carry boundary so a plain running height pays `Theta(W^2)`, and skyline.rs:114 says "The stored form *is* this coding". On the operand the kernels read, the comb is the load-bearing adversary the balanced accumulator exists for, not a funded control, and the representation question is decided. `wide_tooth_comb` makes the same claim with a temporal marker ("under today's coding", 308). Prose must speak in the present tense and a load-bearing rationale must describe the coding in use; a maintainer reading this learns the opposite of what the kernel doc states.

Evidence:

       220	/// crossing. In this coding each tooth stores its own `gamma(2^k − 1)` — `2k +
       221	/// 1` bits — so every crossing is paid for by a comparably-wide input code and
       222	/// operations stay linear per input bit; a delta coding of the same tree stores
       223	/// 3-bit `±1` codes per crossing instead, which is what makes this the
       224	/// separating family for the leaf-delta representation question.

       307	/// normalized region pays O(delta limbs). Each tooth stores `gamma(2^k − 2^w)`
       308	/// — `2k − 1` bits — so under today's coding every crossing is paid for by a
       309	/// comparably-wide input code.

    skyline.rs
       102	//! The accumulator choice is load-bearing, not an optimization: on the boundary
       103	//! comb (`meter::cliff_comb`) the payload stream is 3-bit `±1` codes sitting
       104	//! exactly on a `2^k` carry boundary, so a plain big-integer running height
       114	//! The stored form *is* this coding, so neither byte-level entry point

Resolution: Rewrite the two rationales in terms of the stored skyline: each crossing is a 3-bit (comb) or about `2w + 3`-bit (wide tooth) delta code while the carry spans `k` (or `k - w`) bits, and the balanced signed-digit accumulator is what keeps validation, sweep, and emit linear per stored bit; state that the construction spells the tooth magnitude per tooth only as a building convenience and is not the operand's size. Delete "the leaf-delta representation question" and "under today's coding". Hand tests/meter.rs:7-9 and 1067-1070 to the envelope partition for the same re-denomination. Acceptance: `grep -n "representation question\|under today's coding" crates/before/src/meter.rs` is empty; both docs name the stored per-crossing code width and the accumulator as the mechanism.

### meter-core-5: Idiom inconsistencies: `debug_assert!` guards in three helpers whose siblings `assert!`, `Base::from(0u8)` beside `Base::ZERO`, a `saturating_sub` after an `assert!`, two `Ordering` paths, thirty qualified `suanpan::UBig`s, a rustfmt-folded comment, two undocumented recursive helpers, one caller named in `bitlen`'s doc, and a repeated `Ticks` string round-trip
- Where: crates/before/src/meter.rs:728-730 (related: meter.rs:149, 506, 662, 690, 1245, 1200-1203, 1677-1681, 2067-2077, 2130-2141, 3199-3202, 3453, 3483; meter/tests.rs:183-186 and the fourteen sibling sites through 1483, 290, 930-933, 1361, 1545, 681, 718, 788, 1492)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep counts: `Base::from(0u8)` 4 sites, `suanpan::UBig` 29 occurrences in meter.rs; every listed line read; ticks.rs:155-197 read for the `From` impls); executed: no
- Seen by: structure-prose (two findings), scaffolding (bitlen doc); refutation: confirmed; history: no rationale found for the idiom items; the `Ticks` string door is deliberate and holds (no `From<UBig>` exists, keeping the suanpan type off the stable API), so only a test-local helper remains
- Owner-gated: no

Legibility and the imports-over-qualified-paths rule. The one item with a cost beyond taste: `hole_region` (729-730), `sparse_cliff_comb` (3453), and `scattered_id_offset` (3483) guard with `debug_assert!` while `parked_unit_spine` (2022), `ascend_spine` (1603-1611), and `seam_stop_descent` (2867-2874) use `assert!`; the helpers' own guards are dead in release and their callers' `# Panics` contracts hold only because the callers assert first, which the helper docs do not say. The rest are pattern breaks in a file whose value is uniform construction code, and tests.rs spells the `UBig -> String -> Ticks` door fifteen times where one local helper would do (the integer-valued sites at 290, 930-933, 1361, 1545 can use `Ticks::from`).

Evidence:

       728	fn hole_region(bits: &mut BitsBuf, lead: usize, m: usize) {
       729	    debug_assert!(lead >= 2, "a hole region's block routing needs depth 2+");
       730	    debug_assert!(m >= 1, "a hole region needs at least one descending step");

       149	        codec::encode_int(bits, &Base::from(0u8)); // gamma(0) = "1"
       506	    for level in (0..d.saturating_sub(1)).rev() {

      1200	        ev_leaf(&mut bits, 0); // its collapsed left leaf
      1201	                               // the range: minima oscillating wide/narrow, funded by the
      1202	                               // input codes that store them

      1677	/// The bit length of `k` (`k >= 1`): the freeze-position band's
      1678	/// headroom exponent.

    tests.rs
       183	        let ticks: crate::Ticks = expected
       184	            .to_string()
       185	            .parse()
       186	            .expect("the closed form renders as a count");

Resolution: Make the three helpers `assert!` like their siblings (or state in their docs that the caller owns the contract); `Base::ZERO` at 149, 662, 690, 1245; `d - 1` at 506 (the `assert!(d >= 1)` at 493 makes the saturation dead); one `use core::cmp::Ordering;` in tests.rs; `use suanpan::UBig;` at the top of meter.rs; move the 1200-1203 comment above the call; give the nested `block` fns at 2067 and 2130 the one-line log-depth note `stagger_comb` carries at 3199-3202; make `bitlen`'s doc "the bit length of `k >= 1`" (its callers include `freeze_parade`, `arming_train`, and the tests); add `fn ticks(n: &UBig) -> Ticks` beside `check_version`. Acceptance: `grep -c 'Base::from(0u8)\|suanpan::UBig' crates/before/src/meter.rs` is 0; `just fmt` and `just clippy` are clean; the test bodies read as one assertion each.

### meter-core-6: Construction bodies are pasted across families that differ by one knob
- Where: crates/before/src/meter.rs:780-787 (related: meter.rs:948-955, 832-834 and 945-947, 842-845 and 958-961; the re-arm block loop at 1746-1752, 1834-1840, 1919-1923, 1974-1978; the memo-site sequence at 999-1006, 1161-1168, 1195-1204, 1233-1240, 1318-1325; the id-site block at 1029-1040, 1350-1361, 1108-1123, 1277-1288, 1479-1488; reveal_comb 1392-1413 vs reveal_comb_hifloor 1429-1451; seam_plunge 2726-2737 vs seam_plunge_control 2777-2788; the lean/turn spines at 1992-2004, 2499-2509, 3050-3058, 3309-3317 with `DENSE_SUFFIX_DIGIT_STRIDE` 1799 and `JUMP_PAIR_DIGIT_STRIDE` 2967)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (every listed range read side by side); executed: no
- Seen by: scaffolding, structure-prose; refutation: reframed (the `seam_stop_descent` asserts are a different set, not part of the triplication; the four lean/turn spines share a skeleton but differ in phase and turn leaf, so they are parameterizable rather than byte-identical); history: no rationale found (the paired-return convention at 57-65 concerns coupled pairs, not helper extraction; the module's own `ascend_spine`, `hole_region`, `gap_spine`, `parked_unit_spine`, `seam_stop_descent` show extraction as the practice)
- Owner-gated: no

Legibility and maintenance: a control family that must stay geometrically identical to its red twin except for one knob is safest as the same code with the knob passed in, which the module already does for `ascend_cliff`/`ascend_cliff_plateau`. Byte-identical copies: `collapse_hole` and `site_hole` share their per-unit event and id loops verbatim and `site_hole`'s root-site prefix is `copy_hole`'s; the four-node `for base in [&arm, &one, &settle, &one]` block appears four times; the memo-site bit sequence five times; the id-site block five times; `reveal_comb_hifloor` duplicates `reveal_comb`'s body to change one leaf; `seam_plunge_control` repeats `seam_plunge`'s three asserts. The two 33-stride constants carry the same value and near-identical derivation docs. Extraction is byte-preserving, so no committed shape, envelope, or provenance moves, and the module doc's reason for leaving paired generators as-is (63-65) does not apply.

Evidence:

       780	    for i in 0..k {
       781	        ev.push(true); // spine node
       782	        codec::encode_int(&mut ev, &Base::ZERO);
       783	        ev.push(true); // the unit's site node
       784	        codec::encode_int(&mut ev, &Base::ZERO);
       785	        hole_region(&mut ev, 2 + (i % 2), m); // the collapse range
       786	        ev_leaf(&mut ev, 0); // the site's absent-side sibling leaf
       787	    }

      1746	    for _ in 0..p {
      1747	        for base in [&arm, &one, &settle, &one] {
      1748	            bits.push(true); // block node: 0-leaf left, spine right
      1749	            codec::encode_int(&mut bits, base);
      1750	            ev_leaf(&mut bits, 0);
      1751	        }
      1752	    }

Resolution: Extract `rearm_block(bits, arm: &Base)` for the four block loops; `memo_site(bits, left: &Base, right: &Base)` and `memo_site_id(bits)`; `hole_units(ev, id, k, m)` plus one root-site helper for the hole pairs; give `reveal_comb` a `floor: &Base` parameter with two thin wrappers like `ascend_spine`; hoist the seam-plunge asserts into one shared check; give `gap_spine` a turn-leaf and phase parameter and route `arming_train`, `jump_pair_operand`, and `puncture_product` through it, collapsing the two stride constants into one. Land the pins from meter-core-2 first so the refactor has a byte-identity oracle for every family it touches. Acceptance: each listed bit pattern has one definition site; every existing and new `check_version`/`check_party` pin passes unchanged; tests/meter.rs needs no re-pin.

### meter-core-7: Freeze-regime widths are hand-derived copies of `FREEZE_ALLOWANCE_DIGITS` with no binding test and no per-family freeze or promotion pin
- Where: crates/before/src/meter.rs:1624-1630 (related: meter.rs:1683-1696, 1792-1799, 2146-2153, 2181-2182, 2435-2436, 2480-2489, 2960-2967; integral.rs:274, 846-849, 880-881; query/tests.rs:366-395, 441-452; meter/tests.rs literal sites such as 181, 211, 225, 265, 900, 964, 997, 1298, 1388, 1437)
- Class / severity / confidence: modularity / medium / high
- Provenance: verified (integral.rs:274 read: `pub(super) const FREEZE_ALLOWANCE_DIGITS: usize = 8`, invisible from `crate::meter`; grep: meter.rs has 26 lines with `288`, 9 with `608`, 8 with `33`; meter/tests.rs 11, 5, 7; tests/meter.rs 3, 0, 4; no reference to `FREEZE_ALLOWANCE_DIGITS` outside the query module; `FREEZE_HITS` is `cfg(test)` at integral.rs:284 and read only in query/tests.rs); executed: no
- Seen by: scaffolding; refutation: confirmed (literal counts corrected); history: no rationale found (the constants landed across five commits in three days, each restating the derivation in prose)
- Owner-gated: no

Circular justification is the tell: six constants (`FREEZE_POSITION_DROP_BITS`, `PROMOTION_REARM_SETTLE_BITS`, `LONE_FREEZE_PLATEAU_BITS` all 288; `PROMOTION_REARM_ARM_BITS` 608; `DENSE_SUFFIX_DIGIT_STRIDE` and `JUMP_PAIR_DIGIT_STRIDE` both 33) and dozens of test literals stand in for one number the query fold owns. `288 = 32 * (8 + 1)` is the least power-of-two width with ten base-2^32 digits, the least that trips `live.digit_count() > 1 + 8` at a unit code (integral.rs:847); `608 = 32 * 19` gives twenty digits, clearing the ten-digit settle drift by more than eight (integral.rs:880). Nothing asserts either derivation against `int_digits`/`base_digits`, and the regime claims the docs make ("every block fires one freeze" at 1629, "Exactly one freeze fires ... and no promotion ever does" at 2181-2182, "with no promotion ever firing" at 2435-2436) are pinned by no test: query/tests.rs's `FREEZE_HITS` floors cover the parked-cancellation shapes and the promoting pool in aggregate, and no promotion counter exists at all. The length and `min_ticks` pins are regime-independent, so they stay green when the regime changes underneath the families.

Evidence:

      1627	/// `2^288` is a ten-base-2^32-digit value, so a block's drift exceeds the
      1628	/// following unit code's one digit by more than the query folds' eight-digit
      1629	/// freeze allowance, and every block fires one freeze.
      1630	const FREEZE_POSITION_DROP_BITS: usize = 288;

      1696	const PROMOTION_REARM_SETTLE_BITS: usize = 288;
      2153	const LONE_FREEZE_PLATEAU_BITS: usize = 288;
      1799	const DENSE_SUFFIX_DIGIT_STRIDE: usize = 33;
      2967	const JUMP_PAIR_DIGIT_STRIDE: usize = 33;

    integral.rs
       274	pub(super) const FREEZE_ALLOWANCE_DIGITS: usize = 8;
       847	        if self.live.digit_count() > funded_digits + FREEZE_ALLOWANCE_DIGITS {
       880	        if self.parked.digit_count() > base_digits(&drift) + FREEZE_ALLOWANCE_DIGITS {

Resolution: Widen `FREEZE_ALLOWANCE_DIGITS` to `pub(crate)`; in meter.rs define one `FREEZE_TRIP_BITS = 32 * (FREEZE_ALLOWANCE_DIGITS + 1)`, one `PROMOTION_TRIP_BITS = 32 * (FREEZE_ALLOWANCE_DIGITS + 1 + FREEZE_ALLOWANCE_DIGITS + 1)` (twenty digits), and one `DIGIT_ISOLATION_STRIDE = 33` with its derivation, and make the three 288s, the 608, and both 33s those names; have the tests reference the constants; add a unit test asserting the derivation predicates against `int_digits`/`base_digits`; add a `cfg(test)` promotion tap beside `FREEZE_HITS` and per-family pins for the "exactly one freeze", "one freeze per block", and "never promotes" claims. Acceptance: `grep -nE '\b(288|608|33)\b' crates/before/src/meter.rs crates/before/src/meter/tests.rs` shows only the constant definitions; the construction below fails a named test.

Construction: Set `FREEZE_ALLOWANCE_DIGITS` to 10. A `2^288` drift (ten digits) no longer exceeds `1 + 10`, so `lone_freeze` and `freeze_position` fire no freeze, and `promotion_rearm`'s `2^608` arming (twenty digits) no longer clears `10 + 10`, so no promotion fires. Every test in meter/tests.rs still passes; the pool-level `FREEZE_HITS` floor at query/tests.rs:449-452 still passes because `arming_train`'s `2^(32 * 19)` keeps freezing; only the tests/meter.rs flatness bands move, without naming the cause.

### meter-core-8: The `wide_arming` and `hoisted_window` guard admits widths at which the promotion their docs describe cannot fire, and the committed hoisted-window band runs at one
- Where: crates/before/src/meter.rs:1904-1912 (related: meter.rs:1885-1889, 1931-1933, 1952-1963, 1683-1689, 2480-2489; query.rs:201-225; integral.rs:846-849, 858-887; suanpan accumulator.rs:942-947; meter/tests.rs:960-961, 993-994; tests/meter.rs:5306-5332, 5186-5220)
- Class / severity / confidence: claim / medium / medium
- Provenance: assessed (hand trace of `query::rank`'s fold through `Integrator::boundary` and `freeze` for `wide_arming(w, d)`: the first freeze parks `2^(32w)`, `w + 1` digits; the second freeze's drift is `2^288 + 1`, ten digits; integral.rs:880 promotes only when `w + 1 > 10 + 8`, so `w >= 18`); executed: no
- Seen by: refutation pass (raised as new); refutation: raised with the same trace; history: not examined
- Owner-gated: no for the guard and docs; the `hoisted_window` band's width is the envelope partition's pin

The generator doc says the block "climbs `2^288` (whose unit's freeze finds the parked component over-wide and promotes it — the one ledger arming)", and the `# Panics` rationale says `w >= 10` is "the parked component must clear the settling drift's ten digits by more than the freeze allowance". That rationale describes `w + 1 > 18`, that is `w >= 18`; the number 10 is instead the width at which the arming's own unit code trips the freeze (`w + 1 > 9`). By the trace, for `10 <= w <= 17` the sweep freezes twice and never promotes, so the mechanism the family exists to price is not realized at parameters the guard admits. `hoisted_window` inherits the guard and the claim (1931-1933, 1954-1955), and the committed `hoisted_window` band runs at `HOISTED_WINDOW_WIDTH = 12` (tests/meter.rs:5332), where by this trace no promotion fires; the `ledger_wide_arming` band runs at `w = 500` and `1000`, inside the promoting range. The sibling constants agree with the rule: `PROMOTION_REARM_ARM_BITS` is twenty digits (1685-1688, "more than the ... allowance above the settling drop's ten") and `arming_train` guards `w >= 19` with the same rationale (2480-2489). No promotion counter exists to settle this in a run, and the meter/tests.rs pins at `(10, 1)` and `(12, 5)` check length and `min_ticks` only, both regime-independent. A claim in a generator's doc is a statement of record for every band denominated on it; this one is contradicted by the promotion rule as read.

Evidence:

      1885	/// One promotion whose parked mass is as wide as the input, owing its debt
      1886	/// across a trailing mass as dense as the input. Exactly `134d + 64w + 600`
      1887	/// bits. The one block climbs `2^(32w)` (parked at its unit), climbs `2^288`
      1888	/// (whose unit's freeze finds the parked component over-wide and promotes it —
      1889	/// the one ledger arming), and the sweep then consumes the `Θ(d)`-dense

      1906	/// Panics if `w < 10` (the parked component must clear the settling drift's ten
      1907	/// digits by more than the freeze allowance) or `d == 0`.
      1908	fn wide_arming(w: usize, d: usize) -> Packed {
      1909	    assert!(
      1910	        w >= 10,
      1911	        "the wide arming must out-span the settling drift plus the allowance"
      1912	    );

    integral.rs
       880	        if self.parked.digit_count() > base_digits(&drift) + FREEZE_ALLOWANCE_DIGITS {
       881	            self.promote();

    tests/meter.rs
      5332	    const HOISTED_WINDOW_WIDTH: usize = 12;

Resolution: Add a `cfg(test)` promotion tap beside `FREEZE_HITS` (integral.rs:284) and a meter/tests.rs pin that `wide_arming(w, d).version().rank()` promotes exactly once for `w >= 18` and zero times at `w = 17`; then either raise both guards to the promotion threshold, derived from `FREEZE_ALLOWANCE_DIGITS` once meter-core-7 makes it reachable, and hand the envelope partition a re-parameterized `hoisted_window` band at a promoting width, or re-state both docs and the band prose to the freeze-only mechanism the current widths realize. Acceptance: the promotion pin exists and passes; the guard's number and its rationale describe the same threshold; the `hoisted_window` band's width sits on the documented side of it.

Construction: With the tap in place, run `wide_arming(12, 5).version().rank()` under `cargo nextest run -p before --lib` and read the tap: zero promotions, two freezes. Run `wide_arming(18, 5)`: one promotion. Without the tap, the same fact is visible by instrumenting `Integrator::promote` with an `eprintln!` for one local run.

### meter-core-9: `factor_digit` and `dense_factor` docs differ from their code in the mixing input and a forced bit
- Where: crates/before/src/meter.rs:2325-2334 (related: meter.rs:2348-2350, 2360-2362)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (lines read; the `| 2` is what keeps digit 0 nonzero when the stream digit is 1 after `& !1`); executed: no
- Seen by: instrument-correctness, scaffolding, structure-prose; refutation: confirmed; history: no rationale found (both mismatches born together in 013334f2)
- Owner-gated: no

These are the committed content streams the plateau-puncture incompressibility argument rests on, and both docs must let a reader re-derive the digits exactly. `factor_digit`'s doc says "the SplitMix64 finalizer over `seed ⊕ i`" while the code mixes `seed ^ i.wrapping_mul(0x9E37_79B9_7F4A_7C15)`; `dense_factor`'s doc lists three forced bits while the code also forces bit 1 of digit 0 set, which is what keeps the digit nonzero and even together, and the doc's "every base-2^32 digit nonzero" rests on it unstated.

Evidence:

      2325	/// Digit `i` of the deterministic pseudorandom content stream `seed`: the
      2326	/// SplitMix64 finalizer over `seed ⊕ i`, truncated to one base-2^32 digit, zero
      2334	    let mut z = seed ^ i.wrapping_mul(0x9E37_79B9_7F4A_7C15);

      2348	/// The top bit is forced set (exact width), the top digit's bit 30 forced clear
      2349	/// (never an all-ones digit), and bit 0 forced clear (so `+ 1` never carries
      2350	/// past digit 0 — the gamma code of the value stays at its closed-form width).
      2361	            digit = (digit & !1) | 2;

Resolution: State the mix as the finalizer over `seed ⊕ (i · φ)` naming the golden-ratio constant, and add the fourth forcing to `dense_factor`'s list: "bit 0 forced clear and bit 1 forced set, so digit 0 is even and nonzero". Acceptance: both docs match the code line for line.

### meter-core-10: `arming_train`'s band is documented as `32w + ⌈log₂ n⌉ + 2` but computed as `32w + bitlen(n) + 2`, and the test re-spells the code instead of the doc
- Where: crates/before/src/meter.rs:2470-2472 (related: meter.rs:2490, 1679-1681; meter/tests.rs:1244-1245, 1253-1259)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (lines read; `bitlen(n) = floor(log2 n) + 1` exceeds `ceil(log2 n)` by one at every power of two: `n = 1` gives 1 vs 0, `n = 2` gives 2 vs 1, `n = 4` gives 3 vs 2, and the test table includes `n = 1, 2, 4`); executed: no
- Seen by: adequacy; refutation: confirmed; history: no rationale found (all three spellings born together in e695d5cf)
- Owner-gated: no

Prose states what is: a reader deriving the closed form from the doc gets a different bit count than the generator emits at `n ∈ {1, 2, 4}`. The test passes only because line 1259 recomputes the band as `(usize::BITS - n.leading_zeros()) as usize`, an inline copy of the `bitlen` it already imports (tests.rs:11), so the pin mirrors the implementation rather than the documented derivation and cannot catch the divergence it exists to guard.

Evidence:

      2470	/// every dense window to its right. All wide leaves live in one gamma band
      2471	/// (`band = 32w + ⌈log₂ n⌉ + 2` headroom bits over the swings and kickers), so
      2472	/// the packed size is the closed form `n(g(2·band + 132) + 8·band + 16) + 2`
      2490	    let band = 32 * w + bitlen(n) + 2;

    tests.rs
      1259	        let band = 32 * w + (usize::BITS - n.leading_zeros()) as usize + 2;

Resolution: State the band as `32w + bitlen(n) + 2` (equivalently `32w + ⌊log₂ n⌋ + 3`) in both docs, and have the test call `bitlen(n)` or, better, assert the doc's own spelling as an independent expression. Acceptance: the doc formula evaluated by hand at `n = 1, 2, 4` equals the generator's `band`; tests.rs:1259 no longer re-spells `bitlen`.

### meter-core-11: The stack-segments meter has no writer in any build that reads it through the `meter` feature
- Where: crates/before/src/meter.rs:3543-3559 (related: recurse.rs:17-20, 30, 68-86, 100-109, 118-129; Cargo.toml:33, 44; tests/meter.rs:22-26, 273-279, 363-391 and the sibling harnesses at 1212-1221, 1675-1684, 6900-6911; board/measure.rs:84-92; board/currency.rs:138-140; board/judge.rs:60-64; board/floors.rs:83-85; board/ceilings.rs:80-83, 455-459; meter/tests.rs:392-454; clock/tests.rs:565-566)
- Class / severity / confidence: scaffolding / medium / high
- Provenance: verified (cfg attributes read: `SEGMENTS_GROWN.fetch_add` appears only inside `#[cfg(test)] fn grow` at recurse.rs:100-109; `descend!` is `#[cfg(test)]`; the counter and its readers are `cfg(any(test, feature = "meter"))`; `stacker` sits under `[dev-dependencies]` at Cargo.toml:33, 44; `grep -c 'envelope('` over tests/meter.rs is 172 and `grep -nE 'envelope\(\s*[0-9_]+\s*,\s*[1-9]'` is empty; board sites read); executed: no
- Seen by: scaffolding, instrument-correctness; refutation: confirmed; history: deliberate but expired (the keep is recorded at recurse.rs:17-20 and in the design note's "Defended keeps (2026-07-24 scaffolding sweep) — adjudicated once, not relitigated"; 05bd2b16d on 2026-07-26 gated the only writer behind `cfg(test)`, and 1ddb5a483 on 2026-07-31 made `stacker` a dev-dependency, so the adjudication predates the premise's removal)
- Owner-gated: yes: it relitigates a recorded keep, dissolves an instrument, and touches the board's currency axis and gate policy

The reader's doc presents a live measurement ("this reads the counter bumped at the one place a segment is created"). That place is `recurse::grow`, compiled only under `cfg(test)`, and it calls `stacker`, a dev-dependency. Every consumer outside the library's own unit tests links the library without `cfg(test)`: the four `metered` harness variants in tests/meter.rs, the board's `measure.rs`, and the benches. In those binaries the value read is a compiled-in zero: all 172 envelope pins carry `segments = 0`, the board judges the column under `MAX_GROWN_STACK_SEGMENTS = 1` with a floor declared "NA on every cell today" and a floor-trip message kept "so a future segments floor binds without a code change". Instruments before cures: a ceiling over a counter that cannot count passes vacuously, and here it cannot count by construction rather than by regression; the "future segments floor" slot is a mechanism held open for a failure class no library edit can produce (library code cannot call a `cfg(test)` item). recurse.rs:17-20 says the zero reading "is the measured fact the boards' segments column pins", which is the circular justification. The property the column stands for, no library walk recursing on depth, is already held by `deep_tree_stack_safety` (clock/tests.rs:565-566, depth 100 000) and by `descend!` being `cfg(test)`. The unit test `stack_segment_meter_counts_deterministically_and_resets` proves liveness only in the `cfg(test)` library compilation none of the consumers run in, and its doc at tests.rs:399-403 claims to keep the board's column from being "a dead counter instead of a measured fact", which it cannot do across compilations.

Evidence:

      3546	/// The deterministic stand-in for recursion-driven stack consumption: the
      3547	/// segments the stack guard allocates never pass through the global allocator,
      3548	/// so no heap meter can see them; this reads the counter bumped at the one
      3549	/// place a segment is created. Process-global — meaningful per scenario only

    recurse.rs
       100	#[cfg(test)]
       101	#[inline]
       102	pub(crate) fn grow<R>(f: impl FnOnce() -> R) -> R {
       103	    if stacker::remaining_stack().is_some_and(|remaining| remaining >= RED_ZONE) {
       104	        f()
       105	    } else {
       106	        SEGMENTS_GROWN.fetch_add(1, Ordering::Relaxed);
       107	        stacker::grow(STACK_GROWTH, f)

    Cargo.toml
        33	[dev-dependencies]
        44	stacker = { workspace = true }

    board/judge.rs
        60	/// The segments column's floor-trip message (unreachable while segments is
        61	/// ceiling-only by policy; the judgment loop still carries it so a future
        62	/// segments floor binds without a code change).

Resolution: Owner decision, presented as a relitigation justified by the changed premise. Recommended: dissolve the segments currency: delete `stack_segments`/`reset_stack_segments`, `Currency::Segments`, the `Envelope.segments` column and its assert, `MAX_GROWN_STACK_SEGMENTS`, and `SEG_FLOOR_TRIP`; gate `SEGMENTS_GROWN` and its readers to `cfg(test)` beside `grow` (keeping the meter/tests.rs liveness witness as the guard's own test) or delete them together with that test; re-state tests/meter.rs:1-2 and 22-26, board.rs:47, recurse.rs:17-20, and ceilings.rs:455-459 ("every segment-onset amplifier the suite has caught") against what is. If the owner wants a live recursion signal instead, note that none is reachable from library code by construction; the depth-100k test is the instrument. Acceptance: `grep -rn 'stack_segments\|Currency::Segments\|MAX_GROWN_STACK_SEGMENTS\|SEG_FLOOR_TRIP' crates/before` is empty, `just gate` is clean, the rendered board has no segments column, and `deep_tree_stack_safety` still passes; or, if kept, every doc naming the segments reading in a `meter`-feature binary states that it is structurally zero there.

Construction: In any library kernel compiled with `--features meter`, add an unguarded recursion of depth 200 000 (a `descend!`-guarded one does not compile outside `cfg(test)`). `cargo nextest run -p before --features meter --test meter` still reads `segments=0` on every scenario (stacker is never linked), while `clock::tests::deep_tree_stack_safety` overflows. In neither direction does the column move.

### meter-core-12: Two test docs describe checks their bodies do not make
- Where: crates/before/src/meter/tests.rs:27-29 (related: meter/tests.rs:1009-1020; encode.rs:19-23, 54-58)
- Class / severity / confidence: test-quality / nit / high
- Provenance: assessed (helper and transcoder read; the hoisted-window comment and its assertion read); executed: no
- Seen by: instrument-correctness, adequacy, structure-prose; refutation: confirmed; history: the `check_version` claim was written in faf3cd0a, the same commit that removed the strict decode of the construction stream it describes
- Owner-gated: no

A test helper's doc is the statement of record for what its callers pin. `check_version` claims "canonicality of both codings", but it pins `p.bits`, transcodes through `encode_bits` (which asserts only clean parsing and full consumption), and round-trips the stored wire; no validator for the construction language exists, so its min-lifted normal form is never checked and is unobservable after transcoding. The comment at tests.rs:1009-1014 opens with "rank agrees with the wide-arming shape it deepens at every tail scale", then pivots to comparing the family across its own tail doubling, and the assertion is only `hoisted.rank() >= deeper.rank()`.

Evidence:

        27	/// Check an event-shape output: the construction stream has exactly the
        28	/// closed-form `bits`, and it lifts through the transcoding bridge into a
        29	/// version that survives a wire round-trip (canonicality of both codings).

      1009	    // The tail leaves the fold's answer untouched too: rank agrees with the
      1010	    // wide-arming shape it deepens at every tail scale (the tail's leaves ride
      1017	    assert!(
      1018	        hoisted.rank().checked_sub(&deeper.rank()).is_some(),

Resolution: Say what `check_version` checks: the closed-form construction length and the canonicality of the transcoded skyline (decode round-trip, byte-identical re-encode), noting that the construction language's own normal form is unvalidated and constrained only indirectly by the length pin (or add the validator and keep the claim). Rewrite 1009-1014 to state only what is asserted: a deeper tail's rank is at most the shallower's because the tail sliver's area halves. Acceptance: each doc names exactly the properties its body asserts.

### meter-core-13: The pointwise-dominance premise of two rank bands is witnessed by rank order and by the rank fold itself, not by the comparison sweep
- Where: crates/before/src/meter/tests.rs:297-307 (related: meter/tests.rs:940-950, 1490-1494; version.rs:309-311, 397-405, 428-436, 1730-1732)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (dispatch read: `Version::rank` and `Version::lag` route to `skyline::query::rank`/`lag`, both `Integrator` folds; `partial_cmp` routes to `skyline::sweep::causal_cmp`, a separate kernel; the `lag` contract at version.rs:402-403 makes `lag == ZERO` a correct pointwise witness); executed: no
- Seen by: adequacy; refutation: confirmed; history: no rationale found
- Owner-gated: no

Adequacy: a premise check that shares its kernel with the instrument it underwrites can be wrong in lockstep with it. The `promotion_rearm_mate` and `dense_suffix_mate` pins document that the wide operand dominates its mate pointwise and witness it with `a.rank().checked_sub(&b.rank()).is_some()` (rank order, strictly weaker than dominance, under a message that claims dominance) plus `a.lag(&b) == Rank::ZERO` (sufficient, but computed by the same query fold whose bands rest on the premise). The `tooth_tail` pin does it the independent way, `partial_cmp == Some(Less)` through the comparison sweep.

Evidence:

       297	    let a = promotion_rearm(3).version();
       298	    let b = promotion_rearm_mate(3).version();
       299	    assert!(
       300	        a.rank().checked_sub(&b.rank()).is_some(),
       301	        "the re-arm spine dominates its mate"
       302	    );
       303	    assert_eq!(
       304	        a.lag(&b),
       305	        crate::Rank::ZERO,
       306	        "the dominating side lags by nothing"
       307	    );

Resolution: Replace the `checked_sub` assertion in both tests with `assert_eq!(a.partial_cmp(&b), Some(Ordering::Greater), ..)`; keep `lag == ZERO` if the rank identity is wanted as a second leg, named as such. Acceptance: both tests witness dominance through `partial_cmp`; no message claims dominance over a rank-order check.

### meter-core-14: The seam-plunge control's wire-prefix check contradicts its comment and carries an underived 200-bit slack
- Where: crates/before/src/meter/tests.rs:1083-1104 (related: meter/tests.rs:1060-1067; meter.rs:2753-2767; signed.rs:129-137; gamma.rs:25-28)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (comment and code read; the control's final stored code derived by hand: leaf flag plus `gamma(zigzag(+5·2^64))`, `zigzag(+k) = 2k = 5·2^65`, `m = 5·2^65 + 1` has bit length 68, so `2·68 − 1 = 135` bits plus the flag is 136; with at most 7 padding bits and one partial divergence byte the derived bound is about 151 bits, leaving about 49 of the 200 unexplained); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no rationale found (comment and code born together in d0501cd7)
- Owner-gated: no

Comments state what the code cannot show; here the comment states what the code does not do. The test doc says the control's wire differs from `seam_plunge`'s "only in the final delta code" and the comment says the identity is "asserted structurally instead of byte-wise here, by the leaf heights"; the code counts equal leading bytes of the two wires and accepts any divergence within 200 bits of the control's end, a byte-wise check weaker than the doc's claim with a magic bound. The seam band's reading of the pair's difference as the plunge's own propagation rests on this leg (1064-1067).

Evidence:

      1083	        // The wire-prefix identity: the pair's stored streams agree byte for
      1084	        // byte up to the control's full length less its final code and
      1085	        // padding — asserted structurally instead of byte-wise here, by the
      1086	        // leaf heights: both trees walk the identical ascent.
      1094	        let shared = control_wire
      1095	            .iter()
      1096	            .zip(plunge_wire.iter())
      1097	            .take_while(|(c, p)| c == p)
      1098	            .count();
      1099	        assert!(
      1100	            shared * 8 + 200 >= control_wire.len() * 8,

Resolution: Either check the identity exactly (bit-level prefix equality up to the control stream's live length less its final code, via `BitsView`), or keep the byte-prefix check with a named, derived slack (`SEAM_CONTROL_FINAL_CODE_BITS = 1 + 2 * 68 - 1` plus a stated padding-and-divergence allowance) and rewrite the doc and comment to describe the check performed. Acceptance: the assertion's bound is a named constant with its derivation beside it; the test doc, the comment, and the code describe the same check; the test passes at the three `(k, r)` points.

## Positives

- The registry's exhaustive `Shape::builder` match (registry.rs:301-380) is a genuine compile-time tie: every generator is reached from exactly one variant, and the module's helpers (`ev_leaf`, `ev_spine`, `hole_region`, `gap_spine`, `parked_unit_spine`, `ascend_spine`, `seam_stop_descent`, `jump_pair_operand`) are called only by generators. No dead generators.
- Every generator states its parameter contract under `# Panics` and enforces it with an `assert!` whose message reads as the proof (for example meter.rs:1605-1611 derives the ascend band bound inline), and the one non-obvious frontier is proptested from both sides: `ascend_cliff_band_guard_admits_exactly_the_documented_frontier` (tests.rs:1627-1648) builds at `k = 2^b − 2` and panics at `2^b − 1` through both callers.
- Where a family is pinned, the pin carries a semantic leg independent of the construction and through the public API: `min_ticks` stored-base sums for a dozen families; the harmonic telescoping witness `1 − rank(H(d)) = 1/2^d` that keeps the pin exact at depth 4096 (tests.rs:328-337); `plateau_puncture`'s exact rank `(2xy + 1)/2^(66d)` through the public fold beside a pin of its stored size `128w + 198d + 2` so the floor's denominator cannot drift (1207-1240); full-walk `Less` verdicts for the correlated triples and quadruples, fused and materialized alike (671-728, 778-796); and text spellings that pin the bit-level layout against the tree the doc reasons about (206-250, 896-919, 1358-1383, 1433-1465, 1476-1527).
- The counter witnesses are the right genre: `scan_meter_counts_deterministically_and_resets` floors at `first >= bits` (a strict decode must read every live bit) and the join leg at the output's own bits; `span_traffic_classifies_each_rung` reads the full four-cell snapshot per witness in both operand orders so a miswired arm moves its own cell rather than a total; every process-global comparison appends `ISOLATION_NOTE` (tests.rs:21-25).
- Every counter behind the readers is documented as a relaxed process-global atomic with its isolation requirement stated once per reader, and every reader's doc names the failure class only it can see (3543-3723).
- `meet_shade` (3345-3350) names the clone-identity hazard and builds each shade in a distinct buffer so the flatness band measures the byte compare and not the folds' collapse: the worst-passing-artifact question asked and closed at the generator.
- `factor_digit`/`dense_factor` (2325-2369) state why the content stream is hand-rolled: byte-for-byte reproducibility of committed factors, structureless under the settle's own balanced compaction, which is the domain payload that justifies not reaching for a PRNG dependency.
- The generators' only recursion (`weight_comb`, `freeze_parade`, `stagger_comb`, `stagger_id`) is bounded by the log of a power-of-two parameter and says so at the site (3199-3202); `encode_bits` is iterative over a heap stack.
- Every `usize`/`u32` narrowing goes through `try_from(..).expect(..)` with a one-line reason (`Packed::from_bits`, `pow2`, `plateau_puncture_factors`, `seam_stop_descent`), and the size-formula subtractions (`202n − 4`, `1546k − 2`, `m(4L + 6) − 2`) are guarded by the constructors' knob asserts.

## Open questions for Finch

1. Segments currency (meter-core-11): dissolve it, or keep it with every doc re-stated to say the reading is structurally zero under the `meter` feature? Recommendation: dissolve. The 2026-07-24 keep was adjudicated two days before 05bd2b16d gated the only writer behind `cfg(test)`, so the recorded rationale ("its zero reading over the library kernels is the measured fact") no longer describes a measurement; `deep_tree_stack_safety` and the `cfg(test)` gate on `descend!` already hold the property.
2. Where should the roster-wide canonicality pin live (meter-core-2): a table-driven test in meter/tests.rs over every `Shape`, or `Packed::version()` routed through `Version::decode` under `cfg(any(test, feature = "meter"))`, or both? Recommendation: both. The pin makes the closed forms explicit and catches size drift; the structural door means no consumer can ever measure a non-canonical stream, at the cost of one validation pass per shape before the counters are reset.
3. `Packed` as one type or two (meter-core-3): keep one type with an accurate doc, or split into id and event types so `version()` exists only for event shapes? Recommendation: split, since the meter surface is feature-gated instrument API rather than the library's production surface, but this is your call under the stable-API rule.
4. `wide_arming`/`hoisted_window` (meter-core-8): raise the guard to the promotion threshold and re-parameterize the `hoisted_window` band at a promoting width, or re-state the docs to the freeze-only mechanism the current widths realize? Recommendation: land the promotion tap and pin first (it settles the trace in a run), then raise the guards, deriving the threshold from `FREEZE_ALLOWANCE_DIGITS`; the band re-pin is a consequence for the envelope partition.
5. Should `FREEZE_ALLOWANCE_DIGITS` become `pub(crate)` so meter.rs derives its six constants from it (meter-core-7)? Recommendation: yes; the alternative is six hand-copied numbers and a test that pins the derivation from outside the query module.
6. Cross-partition pointer, not a question for this partition: skyline.rs:125-127 says the tier2 length agreement is "proptested over every adversarial generator family", while tier2/tests.rs imports eight meter generators. I did not read the tier2 test bodies; the tier2 or skyline partition should check whether that sentence is true as written.

## Dropped

- `touch_ops`/`reset_touch_ops` have no caller (adequacy [8]): refuted. The rumors crate calls both at `src/tree/typed/untyped/tests.rs:676` and `:682` under its `meter` feature, which is the audience the reader's doc names; the readers were added for that consumer. The residual observation that before's own suites call `suanpan::touch_meter` directly (121 sites) is a consistency asymmetry below the bar.
- "The luck-proof touch list" names nothing (instrument-correctness [26] and the same claim in [6], [12], [18]): corrected. The list exists at registry.rs:572-583; what remains is an unanchored coinage, merged into meter-core-1.
- "genre" as an unanchored coinage ([18]): below the bar. It reads as ordinary English for a cost class and tests/meter.rs:37-43 defines the two floor genres by contrast.
- The gap-spine lean/turn loops as byte-identical copies ([16] item b): reframed. The four spines share a skeleton but differ in phase and turn leaf; kept as a parameterization item inside meter-core-6.
- `seam_stop_descent` as part of the seam-assert triplication ([4]): wrong. Its asserts (`k >= 1`, `k <= 1 << 11`) are a different set; only `seam_plunge_control` repeats `seam_plunge`'s three.
- Envelope pin count of 62 ([22]): undercounts; the harness has 172 `envelope(` sites, none with a nonzero segments argument. Merged into meter-core-11.
- The `Ticks` string round-trip as a missing conversion ([19]): the string door is the deliberate API (`Ticks` has `From` for unsigned machine integers only, keeping the suanpan type off the stable surface); reduced to a test-local helper inside meter-core-5.
- Duplicates merged: [22] into meter-core-11; [7], [15], [25] into meter-core-2; [14], [23] into meter-core-3; [16] into meter-core-6; [11], [17], [28] into meter-core-14; [12], [13], [18], [21] items 1 and 2, [26] into meter-core-1 and meter-core-9; [21] item 3 and [27] into meter-core-12; [6] items e and f into meter-core-5 and meter-core-9.
