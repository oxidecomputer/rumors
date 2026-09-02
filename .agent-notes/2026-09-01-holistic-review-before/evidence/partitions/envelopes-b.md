# Partition envelopes-b: The resource-envelope pins, second half (tests/meter.rs lines 5306-10808)

## Partition summary

This range is the second half of before's resource-envelope test binary. It holds sixteen feature-gated band modules (`hoisted_window`, `parse_wide_arming`, `answer_embedded_product`, `settle_flatness`, `id_walk_scan_cost`, `accum_streams`, `fold_stagger`, `fold_alias`, `meet_fold`, `memo_resolution_cost`, `width_circulation_cost`, `dominated_undercut_cost`, `pool_recycle`, `placement`, `span`, `span_codec`, `identity_fast_paths`), the `fork_env` and `query_env` row tables with their `sweep_metered`/`query_metered` harness, and the per-row envelope tests for the skyline query kernels, the version-pair queries, the masked comparisons, the cheap-clone cell, and the join folds. Every file in the partition is test code: I read lines 5306-10808 in full with line numbers (5,503 lines) plus the file header (1-120), the `Envelope` harness (200-430), and the `TouchEnvelope` and `SweepEnvelope` harnesses (1110-1280, 1590-1735) for context.

The measurement design is strong and consistent. Each band's `run` resets every counter, takes the heap baseline after the resets, runs only the operation under test, and reads the counters before any formatting allocation; operands, closed forms, and reference folds are built outside the window. Nearly every cost pin rides beside a semantic leg on the same run (a `min_ticks` closed form, the exact rank product `2·x·y + 1`, text-literal expected trees, oracle equality). The derived liveness floors carry their derivations inline and the arithmetic checks (2·999 + 32 = 2,030; 4·1,999 + 64 = 8,060; 1,999 + 64 = 2,063; 1,024·(48 + 2) = 51,200; 2·40 = 80; the id-walk exact pins 500,004 and 1,000,004 equal 2·(2d + 2) at both depths). The `placement`, `span`, `span_codec`, and `identity_fast_paths` sections state their costs as relational identities against compositions on the same operands, each with a nonzero liveness read and a walking control, so nothing there can rot. The `meet_fold` band commits and rosters its known-bad kernel; `pool_recycle` and `dominated_undercut_cost` price properties no other meter can see. Every identifier the range's prose cites resolves in the tree.

The dominant issues are structural and historical rather than measurement defects. Structurally, the file carries four column-subset copies of one envelope struct and harness, and the ×1.25 two-scale ratio, the counter readers, the clock-history fixture, the `tick_run` helper, and the `UBig`-to-`Ticks` conversion are each hand-copied across sibling modules. Historically, the red-first pins of late July were flipped to green guards without renaming: three tests still carry names asserting the refuted reading, eighteen docs open with a `GREEN PIN:` label whose `RED PIN` counterpart no longer exists, and thirteen pins in the range are named outside the convention the band roster scans, so they bind to no roster while the registry describes three of them inaccurately. Two instrument gaps survive review: the stagger and scatter fold bands' known-bad mechanism is never metered (unlike `meet_fold`'s), and `memo_resolution_cost` pins class signatures with no absolute ceiling on six of seven tests. One test's doc describes a limb leg its body no longer has and narrates the removal, which breaches the root AGENTS.md no-ghost-references hard rule; it is the range's one high-severity finding and is a documentation fix.

I ran nothing. Every arithmetic claim below is hand-checked; measured readings quoted below come from the refutation pass's run log (`scratchpad/before/refute-envelopes-b/run1.log`, 24 tests passing), which I read as an artifact. Provenance is stated per finding.

## Findings

### envelopes-b-1: The hoisted-window densify band admits ×1.25 growth its own doc says must be zero
- Where: crates/before/tests/meter.rs:5511-5517 (related: 5453-5459, 5480-5481, 7539-7543, 9449-9453)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read; the refutation pass's run1.log reads `small=84dg/4603B large=84dg/8443B`, so the two readings are equal today); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no rationale found (93741a48 records "84 dg -> 84 dg across the tail doubling (judged absolute: the tail funds no span)" and then applied the ×1.25 convention)
- Owner-gated: no

The constant's doc and the test's doc state the densify column "must not grow across it at all", but the assertion admits a 25% band. An equality pin is available (the readings are equal) and the file already uses one for depth-independence claims in `masked_cmp_hole_depth_band` (7539-7543) and `seam_stop_pool_misses_stay_at_warmup_across_churn_doubling` (9449-9453). The assertion is weaker than the stated invariant.

Evidence:

      5453	    /// Judged absolute, never per byte: the tail doubling adds no window
      5454	    /// density and no settle width, so the densified spans — and this
      5455	    /// column with them — must not grow across it at all. An image sized
      5456	    /// by a cluster's absolute digit position grows with the tail knob —
      5457	    /// roughly ×2 across the doubling — while a span-priced column does not
      5458	    /// move at all.
    ...
      5511	        assert!(
      5512	            large_densify * 4 <= small_densify * 5,
      5513	            "rank's densify column grew more than x1.25 absolute across the \
      5514	             hoisted-window tail doubling ({small_densify} -> {large_densify}): \

Resolution: replace the ×1.25 band with `assert_eq!(small_densify, large_densify, ...)`, keeping the floor and the two absolute ceilings; if the readings are ever not equal, the doc must say why a bounded difference is admissible and pin that difference. Acceptance: the band asserts equality of the two densify readings and passes on the committed readings.
Construction: model a densification that sizes each image `span + position/2048` digits: at tail 10,240 the position term adds about 5 digits and at 20,480 about 10 over the ~84-digit span-priced base (ratio about 1.06), which passes the ×1.25 band and every ceiling while violating the stated invariant; an equality pin fails it.

### envelopes-b-2: The wide-arming and plateau-puncture bands restate the generators' structural strides as literals
- Where: crates/before/tests/meter.rs:5581-5584 (related: 5717; crates/before/src/meter.rs:1799, 2412)
- Class / severity / confidence: test-quality / nit / medium
- Provenance: verified (grep: `DENSE_SUFFIX_DIGIT_STRIDE: usize = 33` is a private const at src/meter.rs:1799; the 66 stride is inline at src/meter.rs:2412); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no rationale found
- Owner-gated: no

`parse_wide_arming::run` derives its liveness floor from "33s leaves" and `answer_embedded_product::run` formats the expected rank exponent as `66 * s`; both are the generators' structural constants, one private and one inline in `plateau_puncture_factors`. A generator stride change changes what the test floor and format assert with nothing to announce it (named constants over magic numbers; a floor's premise should bind to the mechanism that produces it).

Evidence:

      5581	        // The gap spine alone holds 33s leaves (the whole family
      5582	        // 33s + 5), and each leaf's delta extraction reads at least
      5583	        // one accumulator digit.
      5584	        let leaf_floor = 33 * s as u64;
    ...
      5717	            format!("{}/2^{}", ((&x * &y) << 1usize) + 1u8, 66 * s),

Resolution: expose the strides (or a `gap_spine_leaves(d)` helper and the puncture exponent) on the `meter` instrument surface and use them here. Acceptance: `33` and `66` appear in the test only through named items defined beside the generators.

### envelopes-b-3: Coined labels and register transplants: `GREEN PIN`, `mandate`, `mint`, two senses of `genre`, moralized and economic vocabulary
- Where: crates/before/tests/meter.rs:5685-5689 (related: 8917, 9106, 9531, 9572, 9652, 9699, 9725, 9828, 9874, 9955, 10034, 10138, 10204, 10223, 10261, 10389, 10436, 10487 (`GREEN PIN`); 8735, 9203 (`mint`); 6832; 6404; 5751, 9380; 6689, 6733; 5536, 5675, 8161, 8262; the file doc at 35-38)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep over 5306-10808: `GREEN PIN` 18, `RED PIN` 0 in the whole file and no definition anywhere under crates/before; `genre` 22; `honest` 11; `mint` at 8735 and 9203; `git show 03f78744c:crates/before/tests/meter.rs` carries `RED PIN:` at 3474 and 3528 beside `GREEN PIN:` at 3589, and `99d302083`'s tree keeps only the GREEN label); executed: no
- Seen by: structure-prose, scaffolding; refutation: confirmed (and added the two `mint` sites); history: deliberate-but-expired for `GREEN PIN` (one half of a RED/GREEN pair whose RED half every cure removed); no rationale for the rest
- Owner-gated: no

`GREEN PIN:` prefixes eighteen doc first sentences and is defined nowhere; the contrast that gave it meaning (the `RED PIN` labels on committed-failing pins) is gone, and later modules reused the label for relational identities with no red counterpart. `genre` is used for cost-mechanism classes ("the settle's densified-image span genre") while the file doc defines it only for the two lower-bound kinds. "mint"/"minted" is used for producing a value at 8735 and 9203. `mandate`/`mandatory` (5688, 5764), `honest` (11 uses), "not a vibe" (6832), "is the point" (6404), "kills"/"killed" (5751, 9380), "load-bearing" (6689, 6733), and "never decoration" (four uses) perform significance rather than state mechanism. The vocabulary rule: every coined term is anchored to an identifier or defined once by contrast; a doc's first sentence stands alone in a listing.

Evidence:

      5685	    /// Carries the `min_ticks` closed form (`s · x + 1` over the
      5686	    /// committed factors) as the generator's semantic leg, the
      5687	    /// exact-rank leg (the answer is the product `2·x·y + 1` — the
      5688	    /// `Ω(M(|v|))` mandate's witness), and the
      5689	    /// one-touch-per-operand-byte liveness floor.
    ...
      8735	    /// consume-minted width-b boundary difference parks in the latent
    ...
      9531	    /// GREEN PIN: on a full sweep (no demand settles before
      9532	    /// exhaustion), the fused membership walk scans exactly the

Resolution: delete the `GREEN PIN:` prefixes (each sentence already states the invariant); replace `genre` outside the file doc's floor/tripwire contrast with `class` or define the second sense once; `mandate` to `lower bound`; `consume-minted` to `consume-time`, `mints` to `produces`; drop "not a vibe", "is the point", "kills"; "honest improvement" to "improvement" where the dead-meter contrast is already stated. Acceptance: `grep -c 'GREEN PIN' crates/before/tests/meter.rs` reads 0; `grep -n 'mint' crates/before/tests/meter.rs` is empty; `genre` appears only in its defined sense.

### envelopes-b-4: The settle-flatness module comment's level ratio (×1.17) is wrong at the committed probe counts, and `assert_flat_step`'s doc names one band while its limb row uses another
- Where: crates/before/tests/meter.rs:5830-5836 (related: 5866-5868, 5883-5905, 6023-6031, 6083-6087, 7759, 8184, 7833-7852)
- Class / severity / confidence: claim / low / high
- Provenance: verified (arithmetic by hand; `git blame` puts 5830-5835 in 016b91c4a, whose tree already ran `train_run(4/8/16, ...)` at its lines 4497-4501; src/meter.rs:2465-2466 fixes one arming per block, so the arming count is `n`; the run1.log limb readings 6806/7652 -> 16873/14500 give a per-byte ratio of ×1.31 at 4->8, which is why the limb row needs ×1.5); executed: no
- Seen by: adequacy, instrument-correctness; refutation: confirmed; history: no rationale found for the number (wrong at birth with the same probe sizes); deliberate-but-expired for the `assert_flat_step` doc (e4c9b083e widened the limb band to ×1.5 and updated the in-body comment at 5883-5888 but not the fn doc at 5866-5868)
- Owner-gated: no

The comment's own formula `log2(2n)/log2(n)` gives 1.5 at n = 4 and 1.33 at n = 8; under the `(2n).log2()` level model the fold bands use (7759, 8184) the per-doubling ratios are 1.33 (4->8) and 1.25 (8->16). The value 1.17 (7/6) corresponds to n = 64, which no probe runs. The band holds because, as the sentence's second clause says, the settle does not dominate, not because ×1.25 covers the model's admissible growth; a re-pinner reasoning from the comment would call a ×1.3 touch reading at 4->8 a regression the model admits. Separately, `assert_flat_step`'s doc says "per currency, flatness (×1.25 per byte)" while its limb row is banded at 2/3 (×1.5). A band's slack must be justified by a correct argument.

Evidence:

      5830	// The tree rewrites a window's digits once per level and the mass
      5831	// balance keeps levels logarithmic in the arming count, so the
      5832	// settle's metered traffic per byte can grow only by the level ratio
      5833	// across an arming-count doubling — ×log₂(2n)/log₂(n), at most ×1.17
      5834	// from the probes' smallest count — and only if the settle dominated
      5835	// the fold's linear work, which it does not: the ×1.25 flatness
      5836	// convention covers the model's whole admissible growth here. The
    ...
      5866	    /// Assert one probe's reading against its absolute pinned ceilings
      5867	    /// and, per currency, flatness (×1.25 per byte) across the
      5868	    /// doubling, and report the readings.
    ...
      5903	                2u128,
      5904	                3u128,

Resolution: either normalize the settle probes by the level count as `fold_stagger::assert_model_flat` does, so ×1.25 judges the model's constant, or correct the comment to the ratios at the committed counts (×1.5 at 4->8 and ×1.33 at 8->16 under the comment's formula; ×1.33 and ×1.25 under the level model) and rest the touch band explicitly on the measured non-dominance of the settle. Make `assert_flat_step`'s doc name both bands (touches ×1.25, limb ops ×1.5) and why they differ. Acceptance: the comment's number equals its formula at the smallest committed n; the fn doc matches its two numerator/denominator pairs.
Construction: compute log2(8)/log2(4) = 1.5 and log2(16)/log2(8) = 1.333 against the `train_run(4, ...)`, `(8, ...)`, `(16, ...)` calls at 6083-6087; neither is 1.17 and neither is at or under 1.25.

### envelopes-b-5: `id_walk_scan_cost`'s flatness and floor asserts are implied by its exact-equality pins, and its doc hand-maintains derivable byte counts
- Where: crates/before/tests/meter.rs:6361-6367 (related: 6290-6304, 6322-6326, 6331-6342, 6385-6391)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (arithmetic: 1,000,004 · 62,502 · 4 <= 500,004 · 125,002 · 5 and 500,004 >= 62,502 are fixed facts of the constants; bytes per operand = ceil((2d + 3)/8) from `right_spine_tags`/`pack_bits`, so 62,502 and 125,002 follow from `ID_DEPTH`; run1.log confirms bytes=62502/125002 and scan_bits=500004/1000004); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: no rationale found (2f6df4347 replaced the ×1.25 ceilings with equalities and left the floor and flatness in place without comment)
- Owner-gated: no

Given the two-sided equality (whose argument at 6297-6300 is correct), the per-byte ratio and the one-bit-per-byte floor are arithmetic facts about the constants and cannot fire independently: if the equality fails the test fails anyway, and if it holds the other two pass. A guard must name a failure the other committed checks cannot catch. The module doc also states "62,502 packed bytes" and "125,002", which follow from `ID_DEPTH` and rot if the depth moves.

Evidence:

      6294	    /// the metered primitives — [`SCAN_EXACT_BITS_SMALL`] on 62,502
      6295	    /// packed bytes at the half depth, [`SCAN_EXACT_BITS_LARGE`] on
      6296	    /// 125,002 at the full depth, identical for the covers and disjoint
    ...
      6361	        assert_flat("covers", &small, &large);
      6362	        assert_eq!(
      6363	            (small.bits, large.bits),
      6364	            (SCAN_EXACT_BITS_SMALL, SCAN_EXACT_BITS_LARGE),

Resolution: keep the equality and the MEASURED line; delete `assert_flat` and the `bits >= bytes` floor from this module (or state the linearity of the two constants as a `const _: () = assert!(...)` with the byte counts derived from `ID_DEPTH`); replace the literal byte counts in the doc with the derivation. Acceptance: one runtime assertion per depth pair (the exact equality) and no literal byte count in the module's prose.

### envelopes-b-6: The fork row's docs contradict each other on whether a cost record exists, and name no instrument for the split's spine walk
- Where: crates/before/tests/meter.rs:6395-6419 (related: 6410; crates/before/fuzzfit/harness/src/bands.rs:557; crates/before/src/meter/board/ops.rs:1304; crates/before/src/party/ops/split.rs:7)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git blame` resolves 6395 and 6416-6419 to fc862595d9; grep finds `kernel: "ff_party_fork"` at fuzzfit bands.rs:557 and the `party_fork` board cell at ops.rs:1304; split.rs:7 claims `O(|self|)`); executed: no
- Seen by: structure-prose, adequacy (and scaffolding's finding 9, whose circularity objection is answered by the inline rationale); refutation: confirmed; history: no rationale found (both sentences date from the same commit)
- Owner-gated: no

The section header calls `fork_env::ID_FORK` "the split kernel's committed cost record" while the test doc twenty lines later says fork has "no committed cost record"; both were written together, so the second was true only until the first landed. The row's scan column pins the split's absence from the metered primitives by stated design (6400-6405, fc862595d9), which is fine, but nothing in the row says which instrument does price the `O(|self|)` spine walk: it is the fuzz-fit fuel band `ff_party_fork` and the board's `party_fork` cell, neither named here, so the deliberate hole reads as an accidental one. Prose speaks in the present tense, and a row that looks like the instrument should point at the instrument.

Evidence:

      6395	// ─── fork envelope (the split kernel's committed cost record) ───────────────
    ...
      6416	/// Fork is the one id operation with no committed cost record: its halves
      6417	/// materialize (the heap column prices them), its spine walk is iterative
      6418	/// (zero segments), and its writes are raw (the scan pin above). The
      6419	/// rejoin closes the semantic leg: fork then join is the identity.

Resolution: restate 6416-6419 positively ("Fork's cost has no counter of its own here: the halves materialize (heap), the spine walk is iterative (zero segments), the writes are raw (the scan pin above); the walk's linearity is priced by the fuzz-fit `ff_party_fork` band and the board's `party_fork` cell"). Acceptance: the header and the test doc agree, and the row names the instrument that prices the walk.

### envelopes-b-7: `accum_fan_touches_flat` is byte-identical to the comb test; the fan stream its doc prices is never constructed
- Where: crates/before/tests/meter.rs:6629-6641 (related: 6622-6627, 6706-6713; crates/before/src/meter.rs:342-385; crates/suanpan/src/claims.rs:126-131)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (read both bodies: they differ only in the name string; `git log -S'fn fan_run' -- crates/before/tests/meter.rs` is empty and the test entered in 2b0884c0a in this form; run1.log prints identical MEASURED lines for both, `denominator=100000 touches=200000` and `denominator=200000 touches=400000`; suanpan's claims roster cites `accum_comb_touches_flat` and never the fan); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: the shared ceiling is recorded intent (2b0884c0a: "the fan's entry/exit stream is the same arithmetic and shares the ceiling"), but no rationale exists for a second `#[test]` with an identical body
- Owner-gated: no

The doc says the test prices "The unpaid-crossing fan's entry/exit stream", but the body calls `comb_run(4_096, 50_000)` and `comb_run(8_192, 100_000)` exactly as `accum_comb_touches_flat` does. The closing clause "per delta the two streams are the same arithmetic, and the pinned ceiling says so" is asserted by nothing: the test never consults the `cliff_fan` generator (src/meter.rs:342-385, which describes an enter/leave path-sum stream crossing `2^k` twice per tooth). A test's doc must state the invariant it checks, and this test cannot fail independently of its sibling.

Evidence:

      6629	    /// The unpaid-crossing fan's entry/exit stream — the root magnitude
      6630	    /// paid once, then `±1` path-sum crossings per tooth — stays under the
      6631	    /// same pinned per-delta ceiling, flat across the doubling.
    ...
      6636	    #[test]
      6637	    fn accum_fan_touches_flat() {
      6638	        let small = comb_run(4_096, 50_000);
      6639	        let large = comb_run(8_192, 100_000);
      6640	        assert_flat("fan", &small, &large, envelope::COMB_MILLI_PER_DELTA);
      6641	    }

Resolution: either write a `fan_run(k, n)` that drives the accumulator with the fan's own stream (`add_wide(2^k - 1)` once, then per tooth the enter `+1`/sign/leave `-1`/sign crossings of the `2^k` boundary from the root magnitude, its own denominator), keeping the shared ceiling as the measured claim that the streams cost the same; or delete `accum_fan_touches_flat` and move the one-sentence remark at 6711-6713 into the comb test's doc as the reason no fan row exists. Acceptance: no two `#[test]` fns in `accum_streams` share a body; if a fan row remains, its MEASURED line reports a fan-specific stream and the test fails when its per-tooth deltas are made to widen with k.
Construction: break the fan generator's accumulator stream (make `cliff_fan` emit a sequence on which the accumulator is quadratic): `accum_fan_touches_flat` stays green because it never runs that stream.

### envelopes-b-8: Four envelope structs and four metering harnesses are column-subset copies of one another
- Where: crates/before/tests/meter.rs:6746-6805 (related: 211-243, 363-408, 1120-1172, 1206-1275, 1595-1635, 1669-1732, 6894-6976; crates/before/src/meter/board/currency.rs:61-72)
- Class / severity / confidence: modularity / medium / high
- Provenance: verified (read all four structs and harnesses; the tripwire message is copied verbatim at 402, 1260, 1269, 1726, 6961, 6970 and again in band bodies at 7534, 8551, 8788, 8979; `ByCurrency<T>` at currency.rs:61-72 is the five-currency container); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: already known (9de99184c: "the census's B6 unification direction: per-surface column gaps are how the four-harness split keeps manufacturing blind spots"; no `.agent-notes` entry carries it, and the direction was never executed)
- Owner-gated: no

`QueryEnvelope` is `Envelope` plus scan and touch columns and a touch floor; `TouchEnvelope` and `SweepEnvelope` are two other subsets. Each has a `const fn` builder whose only job is the underscore dance for cfg'd-out parameters, and each has its own harness repeating the reset/measure/print/assert block with one or two columns added. Any message or policy change must be applied in four places (the em-dash tripwire message already exists in ten copies), and the doc's own phrase "`sweep_metered`'s harness plus the accumulator touch column" states the subset relation. Infrastructure that generates its own maintenance cascade is the circular-justification tell.

Evidence:

      6782	const fn query_envelope(
      6783	    peak_heap: usize,
      6784	    segments: u64,
      6785	    _limb_ops: u64,
      6786	    _scan_bits: u64,
      6787	    _touches: u64,
      6788	    _limb_floor: u64,
      6789	    _touch_floor: u64,
      6790	) -> QueryEnvelope {
    ...
      6892	/// [`sweep_metered`]'s harness plus the accumulator touch column; prints
      6893	/// the measured numbers so re-pinning never requires editing the harness.

Resolution: collapse to one envelope struct carrying every column (heap, segments, limb ops, scan bits, touches, limb floor, touch floor; `before::meter::board::ByCurrency<Bound>` is the ready-made shape) and one `metered` that stores all columns unconditionally (they are constants) and cfg-gates only the reads and asserts. Two shapes are possible and the owner should pick: `Option<u64>` for unpinned columns (rows keep their current pin sets; the harness prints but does not assert `None`), or every column pinned on every row (wider coverage, but a one-time re-measure of the three-column tables). Acceptance: one struct and one `metered` in the file; the row tables unchanged in shape; `grep -c 'tripwire (measured x0.75)' crates/before/tests/meter.rs` drops from ten to one harness copy plus the band bodies; readings byte-identical before and after.

### envelopes-b-9: The "expansion rows" header sits above the hole and masked rows it does not describe
- Where: crates/before/tests/meter.rs:6851-6854 (related: 6861-6862)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read 6851-6862); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: deliberate-but-expired (the header preceded the expansion rows when written; aafc9bf6f and later commits inserted the hole and masked rows between them)
- Owner-gated: no

The comment introducing "The expansion rows" is followed by `TICK_OWNERSHIP_HOLE` and six more hole and masked rows; `TICK_EXPAND_SPINE` and `TICK_EXPAND_CROSS` begin at 6861. A section header that labels the wrong rows misleads a reader re-pinning by section.

Evidence:

      6851	    // The expansion rows: grow-branch deep
      6852	    // ticks measuring the whole public tick — walk, route fold, and
      6853	    // splice — in one fused pass.
      6854	    pub const TICK_OWNERSHIP_HOLE: QueryEnvelope = query_envelope(3_647, 0, 0, 37_585, 7_563, 0, 4_537); // the ownership-gated block scan: unowned staircase runs fold as one net-and-minimum summary each; the touch ceiling sits below the leaf-by-leaf mechanism's reading, so the skip must engage for the pin to hold, and the scan column holds every skipped bit still read

Resolution: move the comment to precede `TICK_EXPAND_SPINE` at 6861 and give the hole/masked rows a one-line header of their own. Acceptance: each section comment in `query_env` is followed by the rows it names.

### envelopes-b-10: The process-per-test premise has no mechanical guard
- Where: crates/before/tests/meter.rs:6900-6909 (related: 354-355, 5372-5374, 7481, 9479-9483; justfile:113-114; src/conformance/backend/tests.rs:31-37 for a precedent)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (grep finds no `NEXTEST_EXECUTION_MODE`, `AtomicBool`, or `Mutex` guard in the file or the tree; `.config/nextest.toml` sets only slow-timeouts; justfile `test-all` runs `cargo nextest run --workspace --all-features`, so the gate is sound); executed: no
- Seen by: instrument-correctness; refutation: reframed (an in-flight guard catches only two overlapping metered windows; unmetered concurrent work still bleeds heap and counters into a metered body, so the complete guard is an environment check at the cost of forbidding a single-threaded `cargo test`); history: the disposition is documentary (0d1ea4905: "the binary's module doc states that requirement"), and the rumors conformance backend uses a static per-test lock that makes plain `cargo test` sound
- Owner-gated: yes (a documented design decision; the guard shape is a trade-off on how the suite may be run)

Every harness and band `run` resets process-global relaxed atomics and reads them after the body, relying on nextest's process-per-test model documented at 16-21. Nothing checks the premise at runtime: under `cargo test --test meter --all-features` another test's `reset` landing inside a body zeroes a counter mid-flight and a ceiling passes on a partial count; `ISOLATION_NOTE` is appended only to failures, so a false pass carries no note. Every hole becomes a committed check, never a convention held in memory.

Evidence:

      6900	    meter::reset_stack_segments();
      6901	    #[cfg(feature = "limb-meter")]
      6902	    meter::reset_limb_ops();
      6903	    #[cfg(feature = "limb-meter")]
      6904	    suanpan::touch_meter::reset();
      6905	    #[cfg(feature = "scan-meter")]
      6906	    meter::reset_scan_bits();
      6907	    HEAP.reset_peak_usage();
      6908	    let baseline = HEAP.current_usage();
      6909	    let r = f();

Resolution: the owner's choice between (a) one shared entry check that `std::env::var_os("NEXTEST_EXECUTION_MODE")` reads `process-per-test`, panicking with `ISOLATION_NOTE` otherwise (complete, but forbids a single-threaded `cargo test` that is in fact isolation-safe), and (b) the status quo with the premise stated in the file doc. If (a), the check belongs in one helper called by the four harnesses and every band `run` that resets counters directly (5372-5374, 5574, 7481, 8398, 9418), or in `src/meter.rs`'s reset functions. Acceptance: running the binary under a shared-process parallel runner fails deterministically with the isolation message at the first scenario; under nextest all tests pass unchanged.
Construction: two tests in one process, A in `query_metered` and B in `hoisted_window::run`; B's `touch_meter::reset()` at 5372 executes while A's `f()` is mid-walk, so A's `touches` at 6915 reads only the post-reset tail and `touches <= env.touches` passes vacuously; if the reset lands in the final quarter of A's work, `touches >= env.touch_floor` also passes.

### envelopes-b-11: Em-dashes in assertion messages and line comments
- Where: crates/before/tests/meter.rs:6960-6963 (related: 6969-6972, 7533-7536, 8551-8552, 8962-8963, 9155-9156, 9446, 9451, 10530; the row tables' trailing comments at 6837, 6839, 6860; roughly 68 `//` lines in the range)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read the cited sites; the structure-prose lens's awk count of 68 `//` lines and 12 string-literal lines is theirs); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no rationale found (no project rule addresses dashes; the preference is the owner's global doctrine)
- Owner-gated: no

The owner's doctrine prefers colons or semicolons over em-dashes in log messages and code comments (terminal compatibility, sentence flow). Rustdoc is rendered prose where em-dashes are permitted sparingly; the 143 `///` lines in range with em-dashes are not a rule breach.

Evidence:

      6958	    assert!(
      6959	        limb_ops >= env.limb_floor,
      6960	        "{name}: limb counter reads {limb_ops}, below the {} improvement \
      6961	         tripwire (measured x0.75): attribute the drop — an honest \
      6962	         improvement re-pins the band; a dead meter is the bypass this \
      6963	         column exists to catch",

Resolution: replace em-dashes in `//` comments and in assert/eprintln strings with colons or semicolons; the harness message copy (finding envelopes-b-8) makes this a one-site change once the harnesses are unified. Acceptance: `awk 'NR>=5306 && NR<=10808 && !/^[[:space:]]*\/\/\// && /—/' crates/before/tests/meter.rs` returns no lines.

### envelopes-b-12: Past-tense and provenance narration at three sites: a bracketed uncommitted demonstration, a `//` provenance block under a `///` doc, and "the old" check
- Where: crates/before/tests/meter.rs:7137-7140 (related: 7551-7554, 9881-9887; .cargo/mutants.toml:70-76)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git log -S'demonstrated under the live'` -> 500d4d094; `git show 4874f527b9:crates/before/tests/meter.rs` shows the `//` block at 7551-7554 was a dated measurement ledger ("536 -> 352 (2026-07-30 ...)") that 500d4d094 re-worded in place without changing the comment form; 9aec9aa2 (2026-08-18) recorded the compacting delete-field mutant CAUGHT by this row and .cargo/mutants.toml:70-76 states it; e2f4e2a5e dissolved the range walk the "old" checks at 9881-9885 refer to); executed: no
- Seen by: adequacy, structure-prose, scaffolding; refutation: confirmed for the bracket and the split comment, reframed for the compaction-off demonstration (the separator exists as a documented cargo-mutants disposition outside the gate); history: deliberate-but-expired at all three sites
- Owner-gated: no

The ascend row's doc narrates a one-off hand experiment in brackets ("demonstrated under the live swap, same harness") when a committed present-tense fact is available: the mutants campaign of record found `MinWeb::compacting`'s delete-field mutant killed by exactly this row (.cargo/mutants.toml:70-76). `JOIN_EQUAL_OPERANDS_PEAK` carries a `///` doc followed by a `//` block continuing the same rationale, the residue of a provenance ledger whose readings were excised. `dominance_bails_at_the_refuted_start` compares against "the old *first check alone*" and "the old floor-first check", a composition the test constructs live (9906-9912), so "old" narrates the dissolved range walk rather than naming what is measured. Prose speaks in the present tense; provenance lives in git.

Evidence:

      7137	/// accumulator hop. With compaction deleted the same body reads over
      7138	/// both the heap and touch ceilings \[demonstrated under the live
      7139	/// swap, same harness\], so this row is the measured basis
      7140	/// `MinWeb::compacting` cites.
    ...
      7548	/// Peak-heap ceiling for the equal-operands join fold: the fold's own
      7549	/// machinery (the counter's group vec, the dedup adapter's held clone),
      7550	/// none of it proportional to the operands.
      7551	// The equality rung's hand-back is an O(1) refcount bump, so the fold's
      7552	// peak is its size-independent machinery alone — the flatness leg below
      7553	// is the proof. Ceiling 1.25x the measurement of record (the reading
      7554	// lives in the pin commit).
    ...
      9881	    /// replaces; against the old *first check alone* the earlier bail
    ...
      9884	    /// (`probe < lo`, comparable) is where the bail changes class: the
      9885	    /// old floor-first check could confirm `Greater` only at

Resolution: at 7137-7140 replace the bracket with the present-tense fact ("the delete-field mutant of `MinWeb::compacting` reads over both ceilings under the campaign of record, .cargo/mutants.toml"); merge 7551-7554 into the `///` block; replace "the old first check alone" and "the old floor-first check" with "the two-check composition's first check" (the shape built at 9906-9912). Acceptance: no bracketed history, no `//` continuation of a `///` doc, and no "the old" in the range.

### envelopes-b-13: Qualified paths where imports exist or would serve
- Where: crates/before/tests/meter.rs:7155-7157 (related: 7181, 7620-7621, 7644, 7792-7794, 10659, 10704-10723; the sixteen `parse::<before::Ticks>` sites)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read the sites; `use before::{meter, Party, Version};` at line 71 is in scope for the top-level fns at 7181 and 7620-7644; `identity_fast_paths` imports no `Ordering`, so 10659 is a missing import rather than a bypassed one); executed: no
- Seen by: structure-prose; refutation: confirmed (with the 10659 correction); history: no rationale found
- Owner-gated: no

`dashu_int::UBig::` is spelled three times in one expression; `before::Party` is written at top level where `Party` is imported; `before::Rank::ZERO` five times at 10704-10723; `core::cmp::Ordering::Equal` at 10659. Imports over long qualified paths except where the qualification informs; none of these does.

Evidence:

      7155	    let expected = dashu_int::UBig::from(k as u64)
      7156	        * (dashu_int::UBig::ONE << ASCEND_STACK_MAGNITUDE_BITS)
      7157	        + dashu_int::UBig::from((k * (k + 1) / 2) as u64);
    ...
      7181	    let party = before::Party::decode(&Shape::ScatteredId.packed1(CLIFF_SCALE / 2).bytes[..])

Resolution: add `use dashu_int::UBig;` and `use before::{Rank, Ticks};` at file level (and `Party`/`Ordering` in the modules that lack them) and use the short names; use `party_of` at 7181 (see envelopes-b-14). Acceptance: `grep -c 'dashu_int::UBig::\|before::Rank::\|before::Party::\|core::cmp::Ordering' crates/before/tests/meter.rs` reads 0 outside `use` lines.

### envelopes-b-14: Small redundancies: an operand built twice, a fixture built twice, a byte assert that restates `Eq`, a string-length proxy, a one-constant module
- Where: crates/before/tests/meter.rs:7181-7185 (related: 7114-7117, 7580-7586, 10747, 10801, 6406-6411)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (read the sites; `Version`'s `Eq` is byte equality per crates/before/AGENTS.md's hard rule, so 7581 restates 7580); executed: no
- Seen by: adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no rationale found (each site traces to a single introducing commit)
- Owner-gated: no

`skyline_project_comb_scatter_envelope` builds `Shape::ScatteredId.packed1(CLIFF_SCALE / 2)` twice (7181 for the party, 7185 for the byte count); `empty_operands_answer_without_a_walk` destructures `fixture()` as `(v, _, _)` at 10747 and calls `fixture()` again at 10801 for a `w` the first call discarded; `join_all_equal_operands_is_clone_cheap` asserts `out.as_bytes() == a.as_bytes()` right after `out == a` and hand-states "(depth 1,000)" beside `run(1000)`; `skyline_min_ticks_cliff_envelope` uses `r.to_string().len() > 20` as the "exceeds a machine word" proxy; `fork_env` is a `#[rustfmt::skip]` module holding one constant. All outside metered windows; legibility only.

Evidence:

      7181	    let party = before::Party::decode(&Shape::ScatteredId.packed1(CLIFF_SCALE / 2).bytes[..])
      7182	        .expect("scattered id is strict normal form");
      7183	    let enc = skyline_of(&p);
      7184	    let io_bytes_in =
      7185	        enc.as_raw_slice().len() + Shape::ScatteredId.packed1(CLIFF_SCALE / 2).bytes.len();
    ...
      7580	        assert_eq!(out, a, "a ∨ a = a: the fold's verdict is the operand");
      7581	        assert_eq!(out.as_bytes(), a.as_bytes());
    ...
      7114	    assert!(
      7115	        r.to_string().len() > 20,
      7116	        "the comb's floor exceeds any machine word: the wide arm is live"
      7117	    );

Resolution: bind the scattered id once and reuse its bytes (via `party_of`); bind `let (v, _, w) = fixture();` at 10747 and use `(&v | &w)` for the walking control; drop 7581 and name the two depths as constants; assert the cliff floor exceeds a word by `u64::try_from(&r).is_err()` (the `TryFrom<&Ticks> for u64` at src/version/ticks.rs:185) instead of a digit count; make `ID_FORK` a doc-commented `const` beside the test. Acceptance: each generator and fixture is built once per test; the listed sites read as described.

### envelopes-b-15: Three pair rows discard their result without a value leg
- Where: crates/before/tests/meter.rs:7259-7267 (related: 7284-7292, 7334-7342, 7238-7242, 7318-7324, 5984-5988, 422)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read the three bodies and their siblings; `consumed` exists at 422; `pair_run` at 5984-5988 already uses the halves-sum leg); executed: no
- Seen by: scaffolding, instrument-correctness; refutation: confirmed; history: no rationale found (d88be7d938 introduced the lag rows in this shape while giving the distance rows their value legs)
- Owner-gated: no

`version_lag_jump_pair_envelope`, `version_rank_concurrent_envelope`, and `version_lag_concurrent_envelope` return the `(r, ...)` tuple from `query_metered` and drop it, asserting nothing about `r`, while their sibling rows anchor the value (distance equals the two lags' sum at 7238-7242; distance equals rank 2 at 7318-7324). The file's convention, which the lenses list as a positive, is a semantic leg beside every cost pin so a ceiling cannot pass on a wrong answer; these three are the exceptions, and the lag rows have a cheap exact leg (`lag(a, b) + lag(b, a) == distance(a, b)`).

Evidence:

      7259	    query_metered(
      7260	        "version_lag_jump_pair",
      7261	        input_bytes,
      7262	        &query_env::LAG_JUMP_PAIR,
      7263	        move || {
      7264	            let r = a.lag(&b);
      7265	            (r, a, b)
      7266	        },
      7267	    );

Resolution: add the halves-sum leg to both lag rows and a rank closed form (or at least `consumed(r)`) to the concurrent rank row; borrowing closures (`|| a.lag(&b)`) remove the tuple-return shape copied from the distance row. Acceptance: every `query_metered` call in the range either asserts on its returned value or wraps it in `consumed`.

### envelopes-b-16: The masked-hole depth band re-pins the envelope row's touch columns as separate constants
- Where: crates/before/tests/meter.rs:7498-7504 (related: 6860, 7476-7489, 7520-7544)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`MASKED_CMP_HOLE`'s `touches`/`touch_floor` columns at 6860 are 18 and 10, equal to the two constants; run1.log reads `lo=14 hi=14`, and 14 × 1.25 rounds up to 18, 14 × 0.75 rounds down to 10; both landed in a6385d5f; `QueryEnvelope` and the band are top-level items, so the row's private fields are readable from the band); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: no rationale found
- Owner-gated: no

The depth band's flat ceiling and tripwire equal the row's touch columns, and the band's hi point is the same body at the same depth the envelope test measures (`masked_hole_touches` duplicates `masked_cmp_hole_envelope`'s body). One measurement is pinned in two places and a re-pin must move four numbers; the band's unique payload is `assert_eq!(lo, hi)`. Any quantity stated twice drifts.

Evidence:

      7498	#[cfg(feature = "limb-meter")]
      7499	const MASK_HOLE_TOUCH_CEILING: u64 = 18;
    ...
      7503	#[cfg(feature = "limb-meter")]
      7504	const MASK_HOLE_TOUCH_FLOOR: u64 = 10;
    ...
      6860	    pub const MASKED_CMP_HOLE: QueryEnvelope = query_envelope(480, 0, 0, 7_535, 18, 0, 10); // the block skip consumes the spine's unowned continuation whole: the touch reading is a function of the mask depth alone; a per-boundary walk reads ~one touch per spine boundary, orders over the ceiling — the depth band beside this row holds the reading flat across a spine-depth doubling

Resolution: have `masked_cmp_hole_depth_band` read `query_env::MASKED_CMP_HOLE.touches` and `.touch_floor` and delete the two constants; or reduce the band to `lo == hi` plus the row's ceiling applied to `lo`. Acceptance: `grep -c 'MASK_HOLE_TOUCH_' crates/before/tests/meter.rs` reads 0; a change to the row's touch column alone re-pins the band.

### envelopes-b-17: `scatter_population` re-implements the board's private scatter constructor
- Where: crates/before/tests/meter.rs:7618-7649 (related: crates/before/src/meter/board/family.rs:835-872; crates/before/src/meter/registry.rs:1031-1033, 1336-1347, 511, 524)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (read both constructors; `family.rs::scatter(n)` is private and truncates to `n` at 846-848, the test copy does not; the registry's `Scatter` spec has `shapes: &[]`; `Shape::MeetShade.versions` (511) and `Shape::StaggerPopulation.population` (524) are the exposed pattern the other fold bands use); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no rationale found (the registry deliberately keeps organic populations off `Shape`, which explains the missing door, but no commit justifies the second copy)
- Owner-gated: no

The join-fold rows build the scatter population by hand as "the board's `scatter` family recipe" while the board holds the constructor privately. Scatter is the one fold population in the range with two generators; the two recipes already differ (the board's truncates at non-power-of-two scales), and the rows' claim to price "the board's scatter cells" rests on a copy.

Evidence:

      7618	/// Build the scatter fold population: balanced-forked parties, one tick
      7619	/// each, evens before odds (the board's `scatter` family recipe).
      7620	fn scatter_population() -> (Vec<Version>, Vec<before::Party>) {

Resolution: expose the board's scatter builder through the registry as the stagger and shade populations are exposed (a `Shape::Scatter.population(n)` door or a `FamilyId::Scatter` accessor on the `meter` instrument surface) and call it from `fold_version_scatter_envelope` and `fold_party_scatter_envelope`; delete `scatter_population`. Acceptance: one scatter constructor under crates/before, reachable from both the board and tests/meter.rs.

### envelopes-b-18: The stagger and scatter fold bands' known-bad mechanism is computed in the harness but never metered, and its demonstration lives in a commit message
- Where: crates/before/tests/meter.rs:7720-7729 (related: 7750, 7661, 7605-7611, 8313-8343; crates/before/tests/superlinear_tripwires.rs:27-68)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`version_fold_run` builds `sequential` at 7750 before `touch_meter::reset()` at 7753; `TRIPWIRE_ROSTER`'s only tests/meter.rs row is `sequential_meet_reduce_reads_superlinear_on_shade`; the board's `declared_fold_model_admits_the_log_factor_and_rejects_quadratic` (board/tests.rs:873-931) feeds a hand-written exponent `sample(n2, k2, 634_600)`, not a live left fold); executed: no
- Seen by: scaffolding, adequacy; refutation: confirmed; history: no rationale found (the join folds were cured in 4f12b8218 before any roster existed; be6dd6a0d committed a red kernel only for `meet_all`, a live defect at the time; 500d4d094 pointed the prose at the pin commit; no ruling exempts the join folds)
- Owner-gated: no

The module comment names the refuted mechanism (the sequential left fold) and defers its readings to the pin commit; the join-fold section at 7605-7611 makes the same claim for the scatter rows. Unlike `meet_fold`, which commits and rosters `sequential_meet_reduce_reads_superlinear_on_shade`, no committed test demonstrates that a sequential `|` fold fails the stagger or scatter bands, so nothing notices if the family stops separating the two mechanisms. Every criterion needs a committed demonstration that a known-bad mechanism fails it; a demonstration in a commit message is not re-run.

Evidence:

      7720	// The known-bad mechanism these bands separate from: the sequential
      7721	// left version fold (`fold(Version::new(), |acc, v| acc | v)`) re-walks
      7722	// its never-coalescing accumulator per input, and the sequential party
      7723	// fold (one `join` per input) re-walks its accumulated region the same
      7724	// way — the growing-accumulator genre the balanced reduction exists to
      7725	// foreclose, quadratic in arity where the reduction is log-linear, so
      7726	// its readings sit several times over these bands with the gap widening
      7727	// with arity (the demonstration readings live in the pin commit); the

Resolution: add `sequential_join_reduce_reads_superlinear_on_stagger` mirroring 8313-8343 (`population.into_iter().reduce(|acc, v| acc | v)` over the version half of `Shape::StaggerPopulation.population(n, m)` on the arity axis at two scales, asserting model-normalized per-byte growth at or above a class-separating floor such as ×1.49), roster it in `TRIPWIRE_ROSTER`, and excise the parenthetical at 7727; do the same for the party left fold if its reading separates. Acceptance: a `_reads_superlinear_on_stagger` kernel in tests/meter.rs, rostered, red on the sequential fold and failing if `join_all` is swapped in; 7727 no longer points at a commit message.
Construction: copy `sequential_meet_reduce_reads_superlinear_on_shade` with `&` -> `|` and the shade population -> the stagger population at `(n, n)` and `(2n, n)`; the module comment predicts about ×2 per-byte growth per arity doubling, so a ≥ ×1.49 assertion should hold; if it does not, the module comment's adequacy claim is the finding.

### envelopes-b-19: The two-scale flatness assertion, the counter readers, the clock-history fixture, `tick_run`, and the `UBig`-to-`Ticks` conversion are hand-copied across modules
- Where: crates/before/tests/meter.rs:7833-7852 (related: 8212-8231; ratio spellings at 5439, 5635, 5811, 6333, 6511, 8476, 8578, 8960, 9153 (`* 4`/`* 5`), 9340 (`100`/`125`), 8879 (`100`/`115`), 7848 and 8227 (`* 1.25`), `SLACK_NUM`/`SLACK_DEN` at 6457-6460 and 8080-8083; `struct Run` at 6285, 6468, 7822, 8202, 8376, 8681, 9222; `fn scanned` at 9479, 10089, 10348, 10556; `fn fixture` at 9492, 10101, 10360, 10565; `fn tick_run` at 8394, 8701, 9238; `touches`/`limbs` closures at 9706, 10293, 10451, 10502, 10696; the 2^80 decimal literal at 9510, 10123, 10377; `parse::<before::Ticks>` at 5367, 5566, 5700, 5961, 9425, 9511, 10124, 10378 (sixteen in the file); `Party::decode(` on a `Packed` at 7370, 7406, 7408, 7448, 7479, 7794, 8396, 8703, 9242 with `party_of` at 416; the ten per-row bodies at 6982-7075, 7081-7123, 7366-7470)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read both `assert_model_flat` bodies, identical modulo the message prefix; grep counts as listed; `Ticks: From<u128>` exists at src/version/ticks.rs:164 and no `From<UBig>` does); executed: no
- Seen by: scaffolding, structure-prose (three findings), instrument-correctness; refutation: confirmed (downgraded to low: helper-level duplication with no correctness effect); history: no rationale found (the modules' differing cfg gates are the only structural variation, and nothing states them as the reason)
- Owner-gated: no (the `From<UBig> for Ticks` half is an API addition and stays with the owner; see the open questions)

`meet_fold::assert_model_flat` is byte-identical to `fold_stagger::assert_model_flat` modulo the message prefix. The ×1.25 two-scale ratio, the file's one slack policy, is spelled five ways across fifteen sites; seven module-private `Run` structs carry overlapping fields; four identical `scanned` helpers and five `touches`/`limbs` closures read the counters; four `fixture()` builders construct the same six-fork, 24-round, 2^80-plateau history (e4c9b083e added the identical plateau to three of them in one commit); three `tick_run`s (two with the identical `input / 8` floor and a near-identical doc); the 2^80 count is a 25-digit string three times although `Ticks: From<u128>` exists; every closed-form leg converts `UBig` via `.to_string().parse::<before::Ticks>()`; nine `Party::decode` sites bypass `party_of`; ten per-row envelope tests repeat the pack/version/encode/query_metered/assert sequence with only the shape, row, and name varying; and the two-scale ceiling constants take four shapes (`[(u64, u64); 2]`, `(u64, u64)`, `[u64; 2]`, `[(u64, u64, u64); 3]`). Named constants over magic numbers and one helper over hand-rolling: each copy is a place a later change can be missed.

Evidence:

      7833	    fn assert_model_flat(name: &str, small: &Run, large: &Run, counter: fn(&Run) -> u64) {
    ...
      7847	        assert!(
      7848	            m2 * d1 <= m1 * d2 * 1.25,
    ...
      8212	    fn assert_model_flat(name: &str, small: &Run, large: &Run, counter: fn(&Run) -> u64) {
    ...
      8226	        assert!(
      8227	            m2 * d1 <= m1 * d2 * 1.25,
    ...
      9479	    fn scanned(f: impl FnOnce()) -> u64 {
      9480	        meter::reset_scan_bits();
      9481	        f();
      9482	        meter::scan_bits()
      9483	    }

Resolution: hoist to file level one `Slack { num, den }` (or named `Ratio` constants: `FLAT = 5/4`, `LIMB_LEVEL = 3/2`, `WIDTH = 23/20`, `GROWTH = 5/2`), one `assert_flat_per_unit(name, small: (counter, unit), large, slack)` that also prints the MEASURED line (the fold modules pass `unit = bytes × levels`), one `Reading` struct, one `scanned`/`limbed`/`touched` trio, one `History` builder returning the snapshots each section needs, one `tick_run(ev, id) -> Run` with the shared floor, `fn redecoded(v: &Version) -> Version`, `fn ticks(n: &UBig) -> Ticks`, `Ticks::from(1u128 << 80)` for the constant, `party_of` at the decode sites, and `rank_kernel_envelope(name, shape, row)`/`masked_triple_envelope` row helpers so each `#[test]` is a one-line call under its doc. Acceptance: one definition each of `assert_model_flat`, `scanned`, `fixture`, `tick_run`; `grep -c 'fn scanned' crates/before/tests/meter.rs` reads 1; `grep -c 1208925819614629174706176 crates/before/tests/meter.rs` reads 0; `grep -c 'parse::<before::Ticks>' crates/before/tests/meter.rs` reads at most 1; readings and verdicts unchanged.

### envelopes-b-20: Touch liveness floors whose stated premises do not reach the asserted constant
- Where: crates/before/tests/meter.rs:8384-8410 (related: 8690-8717, 5380-5385, 5720-5725, 5857-5862, 5989-5994, 8192-8198, 8661-8668)
- Class / severity / confidence: claim / low / medium
- Provenance: verified for the `input / 8` composition (arithmetic: one touch per 64-bit limb is one touch per 8 payload bytes, and a payload of at least an eighth of the input yields `touches >= input / 64`, not `input / 8`; fe39fca35 states the same two premises for `input/8` and records margins only against readings, "plateau 1,438 vs 206 (x7.0)"); assessed for the per-byte rank floors (no derivation appears in their docs at 5349-5352, 5685-5689, 5847-5848; the adequacy lens's estimate that PP(500, 500)'s irreducible touches sit around a third of its byte count is a re-derivation from src/meter.rs:2380-2415 and is unverified); executed: no
- Seen by: adequacy; refutation: confirmed; history: the `input/8` floor is a deliberate premise-first re-derivation (fe39fca35) whose purpose holds, but the arithmetic critique survives; the `touches >= bytes` floors have no derivation anywhere in history
- Owner-gated: no

The memo and width-circulation `tick_run` docs derive a one-touch-per-eight-bytes floor from two premises whose composition gives one per sixty-four; the floor holds today because readings sit far above it (run1.log: memo_oscillating 996,964 touches against 260,503/8 = 32,562). The module's own header at 8661-8668 distinguishes derived floors from measured tripwires, and a floor whose derivation does not reach its number is an observed-value floor in the liveness genre's clothing. The rank and pair probes' `touches >= bytes` floors carry no derivation at all; if the adequacy lens's estimate is right, an improvement that touches each digit of the plateau once would trip a floor labelled "liveness". A floor asserts the minimum possible work from one universal premise; a legitimate input below it is a finding about the premise.

Evidence:

      8384	    /// Enforces a one-touch-per-eight-input-bytes liveness floor before
      8385	    /// returning, derived from the walk's irreducible work: every
      8386	    /// consumed code's magnitude folds into the height accumulator at
      8387	    /// least once — one digit touch per 64-bit limb of the operand,
      8388	    /// zero limbs included — and in every family here the folded
      8389	    /// payload (the circulated memo minima and the per-leaf delta
      8390	    /// codes) is at least an eighth of the packed input. A reading
    ...
      8404	        assert!(
      8405	            run.touches >= run.input / 8,

Resolution: for the `/8` floors, restate the premise so it yields the constant (for example: every consumed code costs at least one touch, a code spans at most eight input bytes per touch it costs, and the payload is the whole input) or lower the constant to what the stated premises support (`input / 64`); for the per-byte rank floors, derive them per family from the code structure (leaf count plus the wide codes' digit counts) or relabel them as measured-basis tripwires. Acceptance: each floor's doc reproduces its constant from its premises; a hand computation of PP(500, 500)'s irreducible touches is at or above its asserted floor.
Construction: not a runtime failure today. Demonstration for the composition: `(input / 8) / 8 = input / 64`. For the rank floor, a settle that delegates the whole wide × dense product to the backend and touches each of x's ~500 digits once plus ~500 leaf folds reads roughly 1,000-2,000 touches on a ~4.6 KB PP(500, 500) operand and trips `touches >= bytes` at 5720 while being strictly cheaper.

### envelopes-b-21: `memo_resolution_cost::assert_flat` pins a raw ×2.5 class signature only: six of seven tests have no absolute ceiling, and the doubling the ratio divides by is unchecked
- Where: crates/before/tests/meter.rs:8413-8432 (related: 8443-8453, 8462-8485, 8497-8504, 8564-8586, 8597-8607, 8619-8629; the pattern to follow at 8506-8555; the same raw form at 8772-8778 and 9066-9073; the file header at 7-11; crates/before/src/meter/board/family.rs:770-786)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read all seven tests: only `memo_fanout_wide_cost_is_site_count_independent` asserts an absolute ceiling (8541) and tripwire (8548); family.rs:770-786 lists the memo families and DescendingRaises as envelope-only, so no board cell prices them; run1.log's large readings 88,006 / 57,018 / 89,620 / 65,628 / 56,021 / 996,964 are pinned nowhere; the memo inputs grow ×2.13-2.15 across the "doubling" (6,497 -> 13,985 B; 3,442 -> 7,374; 6,100 -> 12,989; 5,107 -> 10,996) while reveal and ascend grow exactly ×2.0); executed: no
- Seen by: adequacy, scaffolding; refutation: confirmed for the missing ceilings, downgraded the unchecked denominator to nit (today's input ratios make the raw band as tight as the per-byte convention); history: the ratio form is the red-first assert's shape carried through the cure (9de99184c's `>= x3.5` flipped to `<= x2.5` in 5f6af2462); the design note records the state, and only `memo_fanout` was given an absolute ceiling for its k-independence claim; no commit or note rules against absolute ceilings on the other six, and the file header's "every scenario here pins the current measured cost" does not hold for this module
- Owner-gated: no

A ratio alone pins the class, never the constant: a change that reads every ledger link twice (still linear) keeps every ratio at ×2.0 and passes all six tests, against the header's promise that "A regression fails loudly now; each improvement tightens a committed number". Every other band module in the range pairs its ratio with an absolute measured ×1.25 ceiling, and `memo_fanout` already does so within this module. Separately, the ratio is raw (`large.touches * 2 <= small.touches * 5`) with the input doubling asserted only in prose ("the input doubles"), while sibling checks in the same module normalize per byte; the band's meaning rests on a generator property nothing checks.

Evidence:

      8413	    /// Assert the linear signature: touches grow by at most ×2.5
      8414	    /// across a size doubling.
      8415	    ///
      8416	    /// A linear resolution reads ×2.0 (the input doubles); a
      8417	    /// resolution that re-reads links once per crossing reads ~×4. A
    ...
      8425	        assert!(
      8426	            u128::from(large.touches) * 2 <= u128::from(small.touches) * 5,
      8427	            "{name}: touch growth across the doubling exceeds x2.5 \
      8428	             ({} -> {}): a ledger link is being read more than once",

Resolution: give each of the six ratio-only tests an absolute touch ceiling (measured ×1.25, rounded up) and a ×0.75 tripwire at its larger run, following `MEMO_FANOUT_TOUCH_CEILING`/`MEMO_FANOUT_TOUCH_TRIPWIRE`; change `assert_flat` (and the inline reveal/ascend growth checks) to the per-byte form `large.touches * small.input * 4 <= small.touches * large.input * 5`, or assert the input ratio it assumes. Acceptance: each test in the module asserts an absolute ceiling at its larger run; every growth band in `memo_resolution_cost` and `width_circulation_cost` divides by `input` or pins the input ratio; a constructed ×2 inflation of the resolution's per-link touches trips at least one ceiling while leaving the ratios green.
Construction: in the frame ledger's site resolution, fold each ledger link into the raise decision twice. `memo_chain_distinct`'s touches roughly double at both scales (ratio about ×2.0, under ×2.5); the `input / 8` floor is trivially satisfied; the per-byte controls are unchanged. All six pass; only `memo_fanout`'s absolute ceiling (73,402) can trip. For the denominator: alter `Shape::MemoChain` so the large run's packed input grows ×1.6 while touches grow ×2.4; the raw band passes (2.4 ≤ 2.5) although per-byte touches rose ×1.5.

### envelopes-b-22: Thirteen cost pins bind to no roster, and the registry excuses their families in prose that misdescribes three of them
- Where: crates/before/tests/meter.rs:8443-8453 (related: 8497, 8531, 8564, 8597, 8619, 8742, 8848, 9043, 9440, 6354, 6378, 7566; crates/before/tests/amp_board_smoke.rs:314-340; crates/before/tests/superlinear_tripwires.rs:87; crates/before/src/meter/registry.rs:1091-1096, 1250-1252, 1463-1466, 1723-1725, 1739-1742, 1756-1758, 1772-1774, 1789-1791; .cargo/mutants.toml:65-69)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`band_test_names` at amp_board_smoke.rs:331 matches only names containing `_is_flat_per_unit` or ending in `_band`; superlinear_tripwires.rs:87 matches only `_reads_superlinear`; the refutation pass's repo-wide grep finds none of the thirteen names outside tests/meter.rs; registry.rs:1723-1725, 1772-1774, and 1789-1791 call the MemoComb, MemoChurn, and DescendingRaises pins "absolute pins outside the band convention" while those tests (8497, 8597, 8619) assert only the ×2.5 ratio at 8426; MemoFanout's reason (1739-1742) is accurate and MemoOscillating's (1756-1758) does not say "absolute"; RevealComb and PureComb are `Priced` by first-half bands (1436, 1454) so the three width-circulation pins here are supplementary; `.cargo/mutants.toml:65-69` cites "the seam-stop pool row in tests/meter.rs" by prose only); executed: no
- Seen by: scaffolding; refutation: reframed (the claim-bearing unrostered pins are the six memo/descending pins cited by registry prose and the seam-stop pool row cited by mutants.toml; three, not four, `Unbanded` reasons misdescribe ratio bands); history: deliberate-but-expired (the `Unbanded` rulings were ratified in cc84df7e on 2026-07-29 after 5f6af2462 (07-25) had flipped the memo pins to two-point ratio bands, so their descriptions were stale at ratification; the Dense ruling "no committed two-point flatness claim" predates `join_all_equal_operands_is_clone_cheap` (4874f527b9, 07-30), a two-point Dense claim)
- Owner-gated: no

`memo_chain_distinct_resolution_reads_linear`, `memo_comb_resolution_reads_linear`, `memo_fanout_wide_cost_is_site_count_independent`, `memo_oscillating_links_are_input_funded`, `memo_churn_undercuts_fold_one_follower`, `descending_raises_stay_linear_under_min_movement`, `reveal_comb_close_reveal_cycle_reads_width_quadratic`, `pure_comb_width_cycle_reads_width_scaled`, `ascend_cliff_undercut_cascade_reads_residue_width`, `seam_stop_pool_misses_stay_at_warmup_across_churn_doubling`, `id_covers_scan_cost_is_pinned_and_flat`, `id_disjoint_scan_cost_is_pinned_and_flat`, and `join_all_equal_operands_is_clone_cheap` are two-scale bands or liveness pins that no roster names, so any of them can be deleted with every gate leg green. `tests/superlinear_tripwires.rs` states the principle ("an unrostered kernel could be deleted with every gate leg green"); the band parity survivor's totality claim holds only for names spelled to its convention; and the registry's `Bands::Unbanded` ("Why no two-point band exists for this family") is functioning as an escape hatch for a naming choice, with three reasons calling ratio bands "absolute pins".

Evidence:

      8442	    #[test]
      8443	    fn memo_chain_distinct_resolution_reads_linear() {
    ...
      8425	        assert!(
      8426	            u128::from(large.touches) * 2 <= u128::from(small.touches) * 5,

    registry.rs:
      1723	                bands: Bands::Unbanded {
      1724	                    reason: "priced by the memo-resolution gate pins in tests/meter.rs, \
      1725	                             absolute pins outside the band convention",

    amp_board_smoke.rs:
       331	                    if name.contains("_is_flat_per_unit") || name.ends_with("_band") {

Resolution: rename the thirteen to the convention (`..._is_flat_per_unit` for the ratio bands, `..._band` for the equality and liveness pins) and replace each `Bands::Unbanded { reason: "priced by ... tests/meter.rs" }` with `Bands::Priced(&[...])` naming them; alternatively extend `band_test_names` to the `_reads_linear`/`_touches_flat`/`_is_pinned_and_flat` genres. Either way correct the three registry reasons that call ratio bands "absolute pins", and revisit the Dense ruling against the clone-cheap pin. Acceptance: `band_tests_and_registry_citations_stay_paired` fails when any one of the thirteen is deleted or renamed; `grep -c 'absolute pins outside the band convention' crates/before/src/meter/registry.rs` reads 0.
Construction: delete `fn memo_comb_resolution_reads_linear` (8496-8504) and run `just gate`: no leg fails, because neither `band_tests_and_registry_citations_stay_paired` nor `superlinear_tripwires_match_the_committed_roster` scans that name.

### envelopes-b-23: A dropped line continuation leaves fourteen spaces inside an assertion message
- Where: crates/before/tests/meter.rs:8580-8580
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`sed -n 8580p | od -c` shows `a c r o s s` followed by fourteen space bytes before `t h e`; `git blame` -> 1196faccc0, whose line 3347 already carried the run, so it was born this way); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no rationale found
- Owner-gated: no

The rendered failure text contains a run of spaces mid-sentence; every sibling message in the module wraps with `\`.

Evidence:

      8580	            "memo_oscillating: per-byte touch cost grew more than x1.25 across              the size doubling: {}/{}B -> {}/{}B",

Resolution: rewrap the literal with `\` as 8478-8479 does. Acceptance: `sed -n 8580p crates/before/tests/meter.rs | grep -c '  the size'` reads 0.

### envelopes-b-24: The width-circulation header restates the file doc's two-genre paragraph
- Where: crates/before/tests/meter.rs:8661-8668 (related: 35-53)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read both paragraphs); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no rationale found for the module-level restatement (the per-constant "live in the pin commits" pointers, fifty in the file, were the deliberate replacement 500d4d094 made for excised snapshots and are not counted here)
- Owner-gated: no

The paragraph distinguishing liveness floors from improvement tripwires appears in the file doc (35-53) and again, in different words, at the head of `width_circulation_cost`; duplicated policy prose drifts, and every sentence competes with the contract the reader came for.

Evidence:

      8661	// Two lower-bound genres guard these pins, named apart because their
      8662	// trips mean opposite things. A liveness FLOOR is derived from the
      8663	// mechanism's irreducible work, never from a measured basis: an honest
      8664	// improvement approaches it but can never cross it, so a trip means

Resolution: delete 8661-8668 and let the module's constants cite the file doc's genres by name. Acceptance: the two-genre paragraph appears once, in the file doc.

### envelopes-b-25: Three green pins carry the names of the red readings they were born asserting
- Where: crates/before/tests/meter.rs:8741-8742 (related: 8848, 9043; the `_reads_linear` names at 8443 and 8497; crates/before/tests/superlinear_tripwires.rs:104-107)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (`git log -S`: `reads_width_quadratic` entered in 03f78744c ("reads RED in a committed reading", pinned `>= x3.5`), `ascend_cliff_undercut_cascade_reads_residue_width` in cac71c8ac; the flips 99d302083 and cdb831fa8 rewrote the assertions to `<= x2.5` and removed the `RED PIN` labels but kept the fn names; run1.log reads reveal_comb 30,848 -> 61,696 on 2,758 -> 5,514 B and ascend_cliff 7,432 -> 14,848 on 1,901 -> 3,800 B, exactly linear); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-but-expired (the memo pins set the precedent the fix wants: 9de99184c named them `_reads_quadratic` and 5f6af2462 renamed them `_reads_linear` at their flip)
- Owner-gated: no

The docs say "gap-funded flat" and "dying-digit-funded flat", the bodies assert `large.touches * 2 <= small.touches * 5` (linear), and the names say the cycle "reads width quadratic", "reads residue width", "reads width scaled". A test's name is the first line of its doc and the string a red run prints; these three state the negation of the invariant, and the roster convention reserves `_reads_` for committed-failing kernels (`..._reads_superlinear_on_...`, superlinear_tripwires.rs:104-107), which the two `_reads_linear` memo names also sit against. The names are also why these pins escape the band roster (envelopes-b-22).

Evidence:

      8727	    /// The reveal comb's close-reveal cycle is gap-funded flat —
      8728	    /// touches grow by at most ×2.5 across the joint (k, b) doubling
    ...
      8741	    #[test]
      8742	    fn reveal_comb_close_reveal_cycle_reads_width_quadratic() {
    ...
      8772	        assert!(
      8773	            u128::from(large.touches) * 2 <= u128::from(small.touches) * 5,

Resolution: rename to the invariant in the band convention (`reveal_comb_close_reveal_cycle_is_flat_per_unit`, `pure_comb_width_cycle_is_flat_per_unit`, `ascend_cliff_undercut_cascade_is_flat_per_unit`, and `memo_chain_distinct_resolution_is_flat_per_unit`/`memo_comb_resolution_is_flat_per_unit` if envelopes-b-21's per-byte form lands), keeping `_reads_superlinear_on_...` exclusively for red kernels, and cite them from their families' `Bands::Priced`. Acceptance: `grep -n 'fn .*_reads_' crates/before/tests/meter.rs` returns only `sequential_meet_reduce_reads_superlinear_on_shade`.

### envelopes-b-26: Derived liveness floors are hand-computed literals beside inline scale literals
- Where: crates/before/tests/meter.rs:8817-8831 (related: 9006-9024, 9094-9104, 9282-9294; the scales at 8866, 9054, 9140, 9324; the derived-from-a-named-knob form at 5473)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (arithmetic by hand: 2·999 + 2,048/64 = 2,030; 4·1,999 + 4,096/64 = 8,060; 1,999 + 64 = 2,063; 1,024·(3·1,024/64 + 2) = 51,200; the scales are bare literals in the bodies); executed: no
- Seen by: structure-prose, instrument-correctness; refutation: confirmed; history: no rationale found (fe39fca35 wrote the derivations in prose beside literals)
- Owner-gated: no

Each floor's doc states a formula over `(k, b)` while the scales live as literals in the test body and the floor is a literal, not an expression over them, so a scale edit leaves the floor silently stale: too low and the liveness margin the column exists for shrinks; too high and a false trip invites a hand-edited number. `HOISTED_WINDOW_DENSIFY_FLOOR: u64 = 2 * HOISTED_WINDOW_GAPS as u64` (5473) shows the intended form. Named constants over magic numbers; the derivation should be code the compiler evaluates.

Evidence:

      8826	    /// The wide plateau's own code folds into the running height once,
      8827	    /// at one touch per 64-bit limb. At (k, b) = (1,000, 2,048):
      8828	    /// 2·(k − 1) + b/64 = 1,998 + 32. A design that honestly does less
      8829	    /// is a floor-premise finding — re-derive the premise before
      8830	    /// trusting the trip.
      8831	    const PURE_COMB_TOUCH_FLOOR: u64 = 2_030;
    ...
      8866	            Shape::PureComb.packed2(1_000, 2_048),

Resolution: name each family's large-run scales (`PURE_COMB_SITES`, `PURE_COMB_WIDTH_BITS`, and so on), use them in the bodies, and write each floor as a `const` expression over them (`2 * (K - 1) + B / 64`, `4 * (K - 1) + B / 64`, `(K - 1) + B / 64`, `K * (3 * B / 64 + 2)`) with the premise in the doc and the numbers gone. Acceptance: every touch liveness floor in `width_circulation_cost` and `dominated_undercut_cost` is a const expression over named scale constants the body uses; the "At (k, b) = ..." numeric restatements are gone.
Construction: change `tick_run(..., packed2(2_000, 4_096), ...)` at 9054 to `(2_000, 8_192)`: the derivation says the floor should read 4·1,999 + 128 = 8,124; the literal stays 8,060; the test still passes with the floor 64 touches under irreducible work and nothing announces it.

### envelopes-b-27: `span_shares_the_crossing_folds` documents a limb leg the body does not have and narrates its removal
- Where: crates/before/tests/meter.rs:10261-10292 (related: 10156-10202, 10293-10327)
- Class / severity / confidence: documentation / high / high
- Provenance: verified (read the doc and body; `git log -S'No limb leg'` -> e4c9b083e, whose message says "the span crossing-fold pin retires its limb undercut" and whose diff adds the "No limb leg ... once rode" comment without touching the doc paragraph at 10265-10269); executed: no
- Seen by: adequacy, structure-prose; refutation: confirmed (downgraded to low as a doc-only fix); history: contradicts the root AGENTS.md hard rule ("Nothing in the codebase refers to code that no longer exists")
- Owner-gated: no

The doc promises "the two meter faces of the fusion, one leg each" and devotes a paragraph to a limb leg with an "unfused hull" witness; the body has only the touch leg and says so in a comment that explains what the removed leg "once rode". This is a test doc that does not state the invariant the test asserts (a claim contradicted) and a ghost reference to a retired assertion, the root AGENTS.md hard rule. The severity follows the rule breached; the fix is a doc edit.

Evidence:

     10261	    /// GREEN PIN: the fused hull decodes the pair once at arithmetic
     10262	    /// width, and folds each crossing into ONE shared running
     10263	    /// difference — the two meter faces of the fusion, one leg each.
     10264	    ///
     10265	    /// The limb leg pins the decode sharing at arithmetic width: each
     10266	    /// wide-gamma decode records one value-width limb count, and the
     10267	    /// composed emitters decode every operand twice. Its witness is an
     10268	    /// unfused hull that decodes per emission; it is blind to the
     10269	    /// accumulator, whose folds record no limb ops.
    ...
     10286	        // No limb leg: word-scale crossings never enter the limb
     10287	        // denomination, so the arithmetic-width undercut that once rode
     10288	        // the composed emissions' duplicated zigzag work has no margin
     10289	        // left to read. Decode sharing is pinned structurally by the

Resolution: rewrite the doc to the touch leg alone (the fused hull folds each crossing into one shared difference; a two-accumulator spelling reads the composed folds back; decode sharing is pinned by the scan identity in `span_fuses_the_pair_walk`); delete the "once rode" sentence, stating only the present fact that word-scale crossings enter no limb denomination, so the pin has no limb leg. Acceptance: the doc names exactly the legs the body asserts; `grep -n 'once rode\|limb leg pins' crates/before/tests/meter.rs` is empty.

### envelopes-b-28: Unchecked counter subtraction in `span_shares_the_crossing_folds`
- Where: crates/before/tests/meter.rs:10320-10326
- Class / severity / confidence: correctness / nit / high
- Provenance: verified (read; the suite runs under the dev/test profile with overflow checks, which .cargo/mutants.toml:33-35 relies on); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no rationale found (1bb610d19a introduced the form)
- Owner-gated: no

`fused - cmp` on `u64` readings rests on the premise that the span ladder runs the classifying comparison first, so `fused >= cmp`. If a ladder change dropped that comparison, the test would die with "attempt to subtract with overflow" instead of this assertion's message. The relation `fused - cmp < met + joined` is `fused < met + joined + cmp` with no subtraction. A harness assertion should fail with its own message under every input it can meet.

Evidence:

     10320	        assert!(
     10321	            fused - cmp < met + joined,
     10322	            "the fused hull's own folds must undercut the composed \
     10323	             emissions' two accumulators ({} vs {} composed touches)",
     10324	            fused - cmp,
     10325	            met + joined
     10326	        );

Resolution: rewrite as `fused < met + joined + cmp` and print all four readings (or assert `fused >= cmp` first with its own message). Acceptance: no bare `-` between counter readings in the range.
Construction: any hypothetical `span` fast path that skips the classifying `partial_cmp` yields `fused` below `cmp`'s early-exiting prefix; the current line panics on overflow before reaching the assertion.

### envelopes-b-29: `span_decode_shares_the_second_payload_decode`'s doc says the gap is "exactly" one check per leaf delta; the test asserts only `>=`
- Where: crates/before/tests/meter.rs:10436-10446 (related: 10474-10478, 10402-10434, 10500-10538)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read; the sibling scan and touch pins use `assert_eq!` at 10422 and 10526); executed: no
- Seen by: adequacy; refutation: confirmed; history: deliberate-but-expired (7b4fd6a1b9 measured the gap as "exactly two limb ops per stepped second-component leaf" and wrote it into the doc; e4c9b083e moved word-scale work out of the limb denomination and left the sentence)
- Owner-gated: no

The doc states an exact identity for the limb gap, but the assertions are a one-sided floor and a strict undercut, so "exactly" is unenforced prose, and after e4c9b083e the gap described (one word-scale zero-equality per leaf delta) is likely zero.

Evidence:

     10439	    /// The floor is the fusion's own pieces; the gap above it is
     10440	    /// exactly the topology-minimality check the fusion keeps (one
     10441	    /// word-scale zero-equality per second-component leaf *delta* —
    ...
     10474	        assert!(
     10475	            fused >= decode_lo + cmp,

Resolution: either assert the gap (count the second component's leaf deltas through the public shape iterators and `assert_eq!(fused, decode_lo + cmp + deltas)`) or reword to "at least the fusion's own pieces; the remainder is the topology-minimality check". Acceptance: "exactly" in this doc is backed by an equality assertion or removed.

### envelopes-b-30: `distinct_buffers_keep_the_walked_paths_covered`'s doc lists operations the body does not exercise
- Where: crates/before/tests/meter.rs:10649-10651 (related: 10668-10672, 10683-10727)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read; the `byte_rung` cells are join, meet, span; distance and lag over re-decoded operands are pinned in `metric_fast_paths_skip_the_fold`); executed: no
- Seen by: structure-prose; refutation: confirmed; history: inaccurate from birth (1bb610d19a wrote the sentence and the metric test together)
- Owner-gated: no

The doc says the byte-compare rung "answers join/meet/span/distance/lag with no bit-stream walk", but the body measures join, meet, and span only.

Evidence:

     10649	    /// not fire across buffers), while the byte-compare rung answers
     10650	    /// join/meet/span/distance/lag with no bit-stream walk — and every
     10651	    /// verdict equals the clone-operand fast path's.

Resolution: drop "/distance/lag" or point at `metric_fast_paths_skip_the_fold` as the test that pins them. Acceptance: the doc names only the three operations the body measures.

## Positives

- The relational identity pins in `placement` (9471-10071), `span` (10084-10328), `span_codec` (10343-10539), and `identity_fast_paths` (10551-10808) are the strongest instruments in the range: each states the fused walk's cost as an exact identity against the composition on the same operands (`fused + cmp_ss / 2 == cmp_sv + cmp_se`; `fused + decode_a + decode_b == met + joined + cmp_ab`; `fused == decode_lo + cmp`), prices a probe decode as half a buffer-distinct self-comparison, and pairs every zero-cost claim with a `> 0` liveness read and a walking control, so there is no constant to re-pin and a dead meter cannot green a section.
- The derived liveness floors carry their derivations inline and the arithmetic checks at every site: `PURE_COMB_TOUCH_FLOOR` 2·999 + 32 = 2,030 (8831), `ASCEND_CLIFF_TOUCH_FLOOR` 4·1,999 + 64 = 8,060 (9024), `PLATEAU_TOUCH_FLOOR` 1,999 + 64 = 2,063 (9104), `DOMINATED_UNDERCUT_TOUCH_FLOOR` 1,024·(48 + 2) = 51,200 (9294), `HOISTED_WINDOW_DENSIFY_FLOOR` 2·40 = 80 (5473), `SEAM_STOP_POOL_WARMUP` = 2 from peak simultaneous demand (9411). Each names its premise and says what a trip means, and the two lower-bound genres are named apart with every assertion message saying which one tripped.
- `id_walk_scan_cost` pins the covers and disjoint walks with two-sided exact equality (500,004 and 1,000,004 = 2·(2d + 2) at both depths) and explains at 6297-6300 why a ceiling plus a slack floor cannot see a uniform tap undercount.
- `dominated_undercut_cost` (9179-9365) reads the decision counter (`emit_traffic().dominated_undercut >= k`) beside the touch band, with the module comment stating exactly why no differential and no cost band alone can prove the arm still fires.
- `pool_recycle` (9367-9461) names the failure class no other meter can see (a dead recycle leaves heap, limb, and touch byte-identical), derives its ceiling of 2 from peak simultaneous demand, asserts equality across the churn doubling, and `.cargo/mutants.toml:65-69` relies on it to kill the retire/lease deletion mutants; the two files agree.
- `masked_cmp_hole_depth_band` asserts `lo == hi`, an exact depth-independence claim no per-boundary mechanism can satisfy, and `join_all_equal_operands_is_clone_cheap` asserts byte-identical peak heap across a 4× operand growth: the strongest available forms of their O(1) claims.
- `fold_stagger` and `meet_fold` normalize every counter by the declared model's level count before the ×1.25 band, so they enforce the model's constant rather than a flat reading the documented log factor forbids, and hold the arity and size axes independently under absolute ceilings at every point.
- `meet_fold` commits its known-bad kernel beside the band, rosters it in `tests/superlinear_tripwires.rs`, and places its ×1.49 floor midway between the linear and quadratic signatures so only a class change crosses it.
- Delta placement is exact throughout: every harness and band `run` resets all counters, takes the heap baseline after the resets, runs only the operation, and reads every counter before any `format!`/`eprintln!`; operands, closed forms, and reference folds are built outside the window.
- Every cost pin but the three in envelopes-b-15 rides beside a semantic leg on the same run (closed-form `min_ticks`, the exact rank product, text-literal expected trees, oracle equality), so a ceiling cannot pass on a wrong value.
- Every identifier the range's prose cites (`ledger_wide_arming`, `mul_bound_embedding_is_alive`, `arbitrary_factors_embed_their_product_in_exact_rank`, `densify_tap_prices_the_cluster_span`, `span_is_the_pair_hull`, the `before::laws` names, the `*_log_factor_is_alive` pins, the counter accessors) resolves in the tree; I found no ghost identifiers.
- The zero-heap pins on the concurrent rows (`RANK_CONCURRENT`, `DISTANCE_CONCURRENT`, `LAG_CONCURRENT` at 6833, 6877, 6878) price the benign regime's constant so an adversarial-path cure cannot hide a charge to common inputs.

## Open questions for Finch

1. Should `accum_streams` (6448-6721) move into `crates/suanpan/tests/`? The module imports only `dashu_int::UBig` and `suanpan::{touch_meter, Accumulator}`, suanpan has the `touch-meter` feature and the dashu-int dependency, and suanpan's claims roster binds the witnesses by a cross-crate relative path (`claims.rs:99 const BANDS: &str = "../before/tests/meter.rs"`), so suanpan cannot verify its own cost table without before's tree. The placement was chosen explicitly (9e7b7ce33, "beside the consumer"), so I converted it from a finding to a question. Recommendation: move it and point `BANDS` at the in-crate path; the only tie to before is the shared ×1.25 comment convention.
2. Is a rostered live kernel owed for the compaction-off separator? The known-bad demonstration for `SKYLINE_MIN_TICKS_ASCEND` exists as a cargo-mutants disposition (.cargo/mutants.toml:70-76, campaign 9aec9aa2) and the gate runs only `mutants-list`, never the campaign (justfile:314-326). Recommendation: cite the disposition in the present tense now (envelopes-b-12); add a `_reads_superlinear` kernel through a documented internal entry in `query/web.rs` only if you want gate-time enforcement, since the seam is the cost.
3. `From<suanpan::UBig> for Ticks`: the sixteen `.to_string().parse::<before::Ticks>()` sites exist because `Ticks` converts from the primitives and `FromStr` only (src/version/ticks.rs:152-213), and before does not re-export `UBig`, so the impl would put suanpan's type in before's public API. Recommendation: the test-local `fn ticks(&UBig) -> Ticks` from envelopes-b-19 now; the API impl only if you want it.
4. The ×1.25 band carries two meanings in the range: `settle_flatness` (5830-5836) and `fold_alias`'s depth axis (8129-8134) let the model's log factor consume part of the slack, while `fold_stagger` and `meet_fold` divide the log factor out and hold a true flatness. Recommendation: normalize every model-bearing band by its model so ×1.25 means one thing (envelopes-b-4's first resolution).
5. Harness unification shape (envelopes-b-8): `Option`-typed columns (rows keep their pin sets, no re-measure) or every column pinned on every row (wider coverage, a one-time re-measure of the three-column tables)? Recommendation: `Option` columns first; widen row by row afterwards under the tightening rule.
6. Process-isolation guard (envelopes-b-10): an environment check forbids a single-threaded `cargo test` that is in fact isolation-safe; the status quo leaves a false pass with no note. Recommendation: the environment check, since `test-all` is nextest anyway and the rumors conformance backend already made the analogous choice with a static lock.
7. Cross-partition (meter-core): `arming_train`'s doc states `band = 32w + ⌈log₂ n⌉ + 2` (src/meter.rs:2471) while the code uses `bitlen(n)` (2490), which for the powers of two the probes use is `log₂ n + 1`; the test mirror at 5940 follows the code. The generator doc is off by one, not the test.
8. Cross-partition (skyline-watermark): `MinWeb::compacting`'s rustdoc (src/version/skyline/watermark.rs:253-254) quotes "×1.41 that row's pinned peak heap and ×2.0 its pinned touches" from the uncommitted hand swap, the measured-snapshot genre 500d4d094 swept from meter surfaces.
9. `SKYLINE_MIN_TICKS_ASCEND` pins 553,660 B peak heap (6839) on an operand of roughly 1.3 KB, an auxiliary-space amplification of hundreds of times; the file header (7-11) owns "today's implementation is far from that", while the crate docs promise auxiliary space at most a small constant multiple of the input. Which document is authoritative, and should the row's comment state the amplification it pins so the contradiction is visible at the row? (I did not verify the operand size; the adequacy lens reports it.)
10. The registry's `closed_form: Option<&'static str>` holds each family's closed form as prose while the executable forms live in this file's `run` bodies (for example `d + 2^(32w) + 2^288 + 3` at 5359-5362), and nothing compares them. Recommendation: leave as is unless you want the slot to become an executable hook the bands call.

## Dropped

- Fork scan column pins the meter's blindness (scaffolding [9]): deliberate and documented inline at 6400-6405 (fc862595d9's "the number moving is the point" ratchet); the useful residue, that the row names no instrument for the walk, lives in envelopes-b-6.
- suanpan's accumulator witnesses live in before's envelope binary ([10]): recorded intent (9e7b7ce33; claims.rs:96-99); converted to open question 1.
- Fan == comb duplicates ([19], [32], [48]): merged into envelopes-b-7.
- Red-mechanism names duplicates ([24], [38], [52]): merged into envelopes-b-25; [52]'s sub-claim about `party_fold_alias_rejection_depth_is_flat_per_unit` (8136) is deliberate-and-holds (the band convention name is what binds it to the registry's `AXIS_BANDS` at registry.rs:1985, and its doc at 8130-8134 states the model bound explicitly).
- Line 8580 duplicates ([27], [44], [53]): merged into envelopes-b-23.
- Masked-hole re-pin duplicates ([36], [58]): merged into envelopes-b-16.
- id-scan tautology duplicates ([40], [60]): merged into envelopes-b-5.
- Three rows without a value leg duplicate ([54]): merged into envelopes-b-15.
- ×1.17 arithmetic duplicate ([50]): merged into envelopes-b-4, which also carries its `assert_flat_step` doc half.
- span limb-leg doc duplicate ([33]): merged into envelopes-b-27.
- Double fixture / operand duplicates ([28], [59]) and the small-redundancies list ([46]): merged into envelopes-b-14; the tuple-return closures are addressed under envelopes-b-15.
- Literal floors duplicate ([51]): merged into envelopes-b-26.
- Harness quadruplication duplicate ([29]): merged into envelopes-b-8.
- Helper duplication ([30], [31]), the UBig-to-Ticks round trip ([14]), and the per-row bodies ([37]): merged into envelopes-b-19.
- Join-fold known-bad duplicate ([18]): merged with [11] into envelopes-b-18.
- Compaction-off demonstration ([34]) and the bracketed aside ([26]): merged into envelopes-b-12 as documentation, with the rostered-kernel question moved to open question 2.
- Raw ×2.5 growth ceilings ([5]): merged into envelopes-b-21 as its second point.
- `GREEN PIN` ([13]): merged into envelopes-b-3.
- `JOIN_EQUAL_OPERANDS_PEAK` split comment ([15]) and the fork docs contradiction ([39]): merged into envelopes-b-12 and envelopes-b-6 respectively.
- Fork row's unnamed pricing instrument ([23]): merged into envelopes-b-6.
- The adequacy lens's PP(500, 500) irreducible-work estimate: kept only as a lower-confidence related note inside envelopes-b-20, not as its own claim (unverified).
- watermark.rs:253-254 snapshot prose and the `arming_train` doc off-by-one: out of partition; routed to open questions 7 and 8.
