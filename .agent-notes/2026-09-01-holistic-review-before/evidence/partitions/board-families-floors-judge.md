# Partition board-families-floors-judge: The board families, floors, judge, measure, operands

## Partition summary

The five files are the amplification board's judgment core, compiled only under the `meter` feature and reached by no production path. `family.rs` (1275 lines) is the shape axis: thirty base-size constants with their derivations, the `FamilyData` operand bundle, a `build` arm per board family, a uniform post-pass that derives the slots a shape does not natively fill, and the two mount adapters that lift one id shape into a disjoint or an overlapping party pair. `floors.rs` (793 lines) is the liveness vocabulary: the rendered `WHY_`/`NA_` derivation strings and the constructors that turn operand quantities into per-currency `Floors`. `judge.rs` (437 lines) fits the exponent trend, resolves each currency's ceiling, and pronounces the per-cell verdict; `measure.rs` (209 lines) brackets one cell body with every counter and settles the denominators from the actual result; `operand.rs` (297 lines) holds the iterative walks over the packed skyline stream that the floors and denominators are stated in. I read all 3011 lines with line numbers, and the neighbor sites every finding rests on (board.rs, ceilings.rs, currency.rs, cell.rs, render.rs, shard.rs, ops.rs call sites, board/tests.rs, registry.rs, the codec scan and build recorders, validate.rs, signed.rs, suanpan's accumulator and touch meter, party.rs, idbits.rs, split.rs, clock.rs, version.rs, forks.rs, tools/benchjudge, tests/superlinear_tripwires.rs, and the 500d4d09 commit). None of the five files is a test file; the sibling `board/tests.rs` was read in part for the adequacy demonstrations.

The apparatus is structurally sound and in several places exemplary. Floors are derived from operands at prepare, outside measurement, and fork on the verdict a cell will produce rather than on the family; `touch_pair_fold` is a model minimum-work derivation (max not sum, zero deltas excluded with the mechanism stated, the family a naive count would ban named at the site); `measure` resets every counter immediately before the body and reads it immediately after, keeping the result alive until the heap peak is read; the exponent estimator is a genuine log-log least-squares slope; and the committed known-bad artifacts (the meter-bypassing walk, the chunked schoolbook converter, the lump ladder versus the quadratic ladder, the fold model's fat constant, the stale capacity model) go through `evaluate` itself. The `ByCurrency` totality mechanism means a new currency is a compile error at every declaration and judgment site. Under the scaffolding lens nothing is dissolvable outright: every instrument names a failure class outside itself and no external tool produces these numbers.

The dominant issues are three. First, one genuine hole in the verdict of record: the heap exponent leg fits the raw readings while the constant leg subtracts the flat allowance, so a Theta(n^2) heap term of allowance-scale magnitude reads green across the whole acceptance ladder, weaker than the bare debugging view on the same artifact (finding 21). Second, a pattern of not-applicable declarations whose rendered reasons the code contradicts (the decode rows' touch, the seed party's scan, `clock_fork`'s heap), which leaves `clock_fork` on every version-only family watched by no floor at all and outside the module's own exposure disclosure (findings 10, 11). Third, prose drift of the kind the owner's own 2026-08-11 excision ruling addressed but did not finish in `family.rs`: probe-build readings quoted as present-tense fact, one mislabeled "committed", and a slot roster that has outgrown the code (findings 1, 2). The remainder is legibility work a maintainer would schedule: the same preorder walk and zigzag decode written out four and two times over in `operand.rs`, repeated `Floors` literals and scan-floor casts in `floors.rs`, an undefined "deterministic-liveness" class contradicted by the module's first sentence, and a vocabulary of "honest", "mint", and "today" that the writing-style rules now name.

## Findings

### board-families-floors-judge-1: Base-size docs quote probe-build readings the owner's excision ruling classifies as excised; one is labeled "committed" against its source, two have no committed kernel
- Where: crates/before/src/meter/board/family.rs:27-38 (related: family.rs:236-240, 250-254, 272-276, 288-292, 306-317; ceilings.rs:56-62; tests/meter.rs:4551, 5220, 5596; tests/superlinear_tripwires.rs:27-68)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git show 500d4d09 -- family.rs` touched only the FREEZE_POS, PROMO_REARM, and RevealComb passages; `git blame` puts every cited range at b3f09baa09, 2026-08-06, five days before the sweep; the tripwire roster and tests/meter.rs lines read); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed (partial sweep, not a policy exemption); history: deliberate-but-expired
- Owner-gated: no (the ruling exists; this applies it)

Commit 500d4d09 ("excise measured snapshots from meter prose") rules that probe-build readings are excised and known-bad separations restated as complexity classes, and names board/family.rs among the swept surfaces; six constant docs survived it: HUGELEAF (29-35: "~1× to ~4×", "e 1.41"), WEIGHT_COMB (238: ×1.93), FREEZE_PARADE (252: ×1.91), DENSE_SUFFIX (274: ×1.96), WIDE_ARMING (290-291: ~×1.9, ×2.00, plus the copied literals 500 and 256), PLATEAU_PUNCTURE (311-317: ×1.879, ×1.579, ×1.555, ×1.91, ×1.65). The weight-comb line calls its source "the band ceiling doc's committed probe-build measurement" where tests/meter.rs:4551 says "a local probe build", and the tripwire roster has no weight-comb or freeze-parade kernel, so those two readings have no committed instrument behind them. Principle 5 (dated measurement reports are not exempt) and Principle 8 (a number you were handed is a hypothesis).

Evidence:

        29	/// Sized so the level doubling stays inside one backend decimal-conversion
        30	/// regime: the backend's divide-and-conquer parser switches algorithm between
        31	/// 16,000 and 20,000 value bits (its parse transient steps from ~1× to ~4× the
        32	/// value bytes there, by measurement), and a probe pair straddling that switch
        33	/// reads the step as a heap exponent — a 16,000-bit base fits e 1.41 on the

       237	/// of the `skyline_flatness` weight-comb band's small run: with certificate
       238	/// consumption disabled, rank reads ×1.93 per-byte growth across this regime's
       239	/// doubling (the band ceiling doc's committed probe-build measurement), so the

    tests/meter.rs:
      4551	    /// consumption disabled (a local probe build whose scans step

    ceilings.rs:
        56	// Several ceilings below argue their calibration from the worst honest reader
        57	// at the release profile of record. The readings themselves live in the pin
        58	// commits (`git log -S` the constant), never in this prose: a quoted reading
        59	// would keep asserting itself as present-tense fact while headroom absorbed

Resolution: Apply the 500d4d09 treatment to the six docs: keep the design argument (which band's small run the base matches, why the pair straddles the regime, the mod-32 remainder alignment), restate each known-bad separation as a class ("the known-bad settle reads a quadratic's ~×2 per byte per doubling"), cite the committed `_reads_superlinear` kernel by name where one exists, and for weight comb and freeze parade either write "a local probe build" as the band doc does or commit the kernel. Replace the literals 500 and 256 at 289-290 with the two `WIDE_ARMING_SMALL` names. For HUGELEAF, state the backend-regime boundary as the constraint (both probes on one side of the parser's algorithm switch) without the measured exponents. Acceptance: `grep -nE '×[0-9]\.[0-9]{2,3}|e 1\.41|~1× to ~4×' crates/before/src/meter/board/family.rs` returns nothing; "committed" is not applied to a probe-build measurement; every cited kernel name matches a row in tests/superlinear_tripwires.rs.

### board-families-floors-judge-2: The `version2` slot doc enumerates two pair shapes; four build arms fill the slot
- Where: crates/before/src/meter/board/family.rs:413-415 (related: family.rs:679, 717, 751, 761)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the four `data.version2 = Some(..)` arms; `git log -S'jump-pair, concurrent-pair'` traces the phrase to d88be7d9, 2026-07-27, before the tooth-tail and dense-suffix arms landed); executed: no
- Seen by: adequacy; refutation: confirmed; history: deliberate-but-expired
- Owner-gated: no

The slot doc names "the pair shapes (jump-pair, concurrent-pair)" as the only arms that fill `version2` themselves; the `DenseSuffix` arm (717) and the `ToothTail` arm (761) also set it. A hand-maintained enumeration of a fact the code changes without touching the prose (Principle 5), already rotted.

Evidence:

       413	    /// Derived uniformly by the post-pass — except on the pair shapes
       414	    /// (jump-pair, concurrent-pair), whose build arms fill it with the pairing
       415	    /// the shape was constructed around and the post-pass leaves in place.

       717	                data.version2 = Some(Shape::DenseSuffixMate.packed2(p, p).version().encode());

       761	                data.version2 = Some(b.version().encode());

Resolution: State the structure, not the roster: "except where a build arm fills it with the pairing the shape was constructed around; the post-pass leaves such a pairing in place". Acceptance: the doc names no shapes.

### board-families-floors-judge-3: Idiom slips in family.rs and two floors.rs signatures
- Where: crates/before/src/meter/board/family.rs:547-548 (related: family.rs:518, 1054-1064, 1084, 1091, 1199, 1262-1275; floors.rs:751-756, 561-568; ops.rs:969, 1007)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read every site; ops.rs:969 and :1007 read for the `&(..).partial_cmp(..)` call shape; party.rs:122-124 and the empty version's 2-bit stream confirm the `+ 1`/`+ 2` values are correct today); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

family.rs:547 assigns `data.cross` and immediately reads it back through `.as_ref().expect("just set")`; :518 shadows the `scale: f64` parameter with a `usize` of the same name inside a function whose `size` closure still captures the original; :1059, :1063, :1084, :1091 add `+ 1`/`+ 2` for the seed party's and empty versions' packed bytes as bare literals; :1061-1062 decodes both parties to keep one and then re-reads `self.parties` for the byte length; :1199 spells `crate::idbits::skip_subtree` inline where the file imports everything else at the top; :1267-1270 clones out of an owned `FamilyData` about to drop. In floors.rs, `masked_cmp_floors` takes `verdict: &Option<Ordering>` (a `Copy` value) so both callers borrow a temporary, and `sync_floors` re-encodes both versions to obtain a byte count every sibling takes as `packed_bytes`. Named constants over magic numbers; imports over long qualified paths; none affects behavior.

Evidence:

       547	                let (v, p) = data.cross.as_ref().expect("just set");
       548	                data.content_bytes = Some(value_content_bytes(&decode_version(v)) + p.len());

      1061	        let (a, _, _) = self.party_pair()?;
      1062	        let n = self.parties.as_ref().map(|(a, _)| a.len())?;
      1063	        Some((Clock::from_parts(a, Version::new()), n + 1))

    floors.rs:
       751	pub(super) fn masked_cmp_floors(
       752	    verdict: &Option<Ordering>,

Resolution: Bind the cross pair to a local before assigning; rename the shadow as the neighbouring arms do (`let s = size(CLIFF_BASE_SCALE)`); derive the operand bytes from the operands built (`Party::seed().encode().len()`, `Version::new().encode().len()`) or name them; in `clock()` read `self.parties.as_ref()?` once and decode only the party used; `use crate::idbits::skip_subtree;`; move out of `data` in `study_family_versions`; take `verdict: Option<Ordering>` by value; give `sync_floors` the bytes. Acceptance: no `expect("just set")`; no bare `n + 1`/`n + 2` in family.rs; `masked_cmp_floors` callers pass `partial_cmp(..)` without `&`.

### board-families-floors-judge-4: Em-dashes in `//` comments and in rendered legend strings
- Where: crates/before/src/meter/board/family.rs:614-615 (related: family.rs:742, 898, 912, 914; judge.rs:312; floors.rs:161, 253, 543)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -nP '^\s*//(?![/!]).*—'` over the partition: seven hits; an awk pass over the `WHY_`/`NA_` constant bodies in floors.rs: three hits; `git blame` puts family.rs:614-615 at 500d4d09, 2026-08-11); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found (the colons rule was already captured on 2026-08-10; only 614-615 postdate it)
- Owner-gated: no

Non-doc `//` comments carry em-dashes at seven sites, and three legend strings that render in the board's terminal output carry them too (WHY_LIMB_STREAM, WHY_TOUCH_FOLD_MERGES, WHY_SCAN_SYNC_VERSIONS). The owner's rule is spaced double-hyphens in code comments and colons or semicolons in terminal output. The pattern is crate-wide (374 such `//` comments in crates/before/src), so this partition's sites are a sample; the two at 614-615 were written after the rule's first capture.

Evidence:

       614	                // input — output ~x4 per joint input doubling by construction
       615	                // — the same output domination the comb-scatter cross

    floors.rs:
       161	     own codes, not the decoded tree's values — a plateau of equal wide leaves stores its \

Resolution: ` -- ` in the seven `//` comments; a colon or semicolon in the three strings. Acceptance: `grep -nP '^\s*//(?![/!]).*—' crates/before/src/meter/board/*.rs` is empty; `grep -n '—' floors.rs` matches only `///` and `//!` lines.

### board-families-floors-judge-5: Generator minimum widths duplicated as bare literals at the family call sites
- Where: crates/before/src/meter/board/family.rs:662-673 (related: family.rs:725, 733; meter.rs:1909-1911, 2445-2447, 2612-2614)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (read the three generators' `assert!` lines against the three `.max(..)` clamps); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The DominatedUndercut, WideArming, and PlateauPuncture arms clamp their knob with `.max(128)`, `.max(10)`, `.max(10)`, each described as "the generator's minimum width", while the minima live only as literals inside the generators' `assert!`s. A change to a generator's bound leaves the call-site clamp silently stale: too low panics under scale-down, too high stops the clamp from ever binding. Named constants over magic numbers; one home per fact.

Evidence:

       662	            FamilyId::DominatedUndercut => {
       663	                // One knob drives the site count and the wide width (the
       664	                // band's DU(s, s) diagonal), floored at the generator's
       665	                // minimum width; the floor binds only under extreme
       666	                // scale-down (the base constant's rustdoc).
       667	                let s = size(DOMINATED_UNDERCUT_BASE).max(128);

    meter.rs:
      2612	    assert!(
      2613	        b >= 128,
      2614	        "the wide width must decide the word-bound domination read"

Resolution: Have each generator export its minimum as a named `pub(crate) const` used by both its `assert!` and the family arm's clamp, or move the clamp into the `Shape` constructor so the board never needs to know it. Acceptance: no bare `.max(<literal>)` remains in `FamilyData::build`.

### board-families-floors-judge-6: Board membership is declared in the registry and re-derived by a 19-variant `unreachable!` arm
- Where: crates/before/src/meter/board/family.rs:765-786 (related: registry.rs:569-586, 1232-1236; shard.rs:505-508; tests/amp_board_smoke.rs:72-81)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (`grep -c 'coverage: Coverage::Board'` = 33 and `'coverage: Coverage::EnvelopeOnly'` = 19 in registry.rs; the arm lists 19 variants; shard.rs and the smoke test each carry a `let Coverage::Board { cells } = .. else { unreachable!(..) }`); executed: no
- Seen by: scaffolding, instrument-correctness; refutation: confirmed; history: no-rationale-found (registry.rs:569-571 states the intent that a board column's build arm be compiler-forced; nothing opposes a typed sub-roster)
- Owner-gated: yes (a cross-partition design change touching the registry)

`FamilyId::board()` filters the roster on the registry's `Coverage::Board` answer, yet `FamilyData::build` must separately list every `Coverage::EnvelopeOnly` variant in an `unreachable!` arm, and two more sites re-derive the same partition. The rosters agree today but nothing ties them: a variant flipped to Board without a build arm panics in the smoke test; a variant given a build arm and left EnvelopeOnly is dead code no check flags. The registry doc already lists what "the compiler cannot force" when a family is added; this arm is part of that cascade (Principle 3).

Evidence:

       782	            | FamilyId::PropagateSeam
       783	            | FamilyId::LatentLadder => unreachable!(
       784	                "{kind:?} is envelope-only in the registry: it has no operand bundle, \
       785	                 and the board sweeps FamilyId::board() alone"
       786	            ),

    registry.rs:
      1232	    pub fn board() -> impl Iterator<Item = FamilyId> {
      1233	        FamilyId::ALL
      1234	            .into_iter()
      1235	            .filter(|f| matches!(f.spec().coverage, Coverage::Board { .. }))
      1236	    }

Resolution: Carry the board sub-roster as its own type: `Coverage::Board { family: BoardFamily, cells }` with a `BoardFamily` enum, `FamilyId::board()` yielding `BoardFamily`, and `FamilyData::build(kind: BoardFamily, ..)` matching exhaustively; the shard and smoke-test destructurings dissolve with it. If the owner takes it, the base-size constant can ride the same declaration (see open questions). Acceptance: no `unreachable!` in the board module or its tests mentions `FamilyId::board()` or "envelope-only"; adding a Board variant fails to compile until it has a build arm.

### board-families-floors-judge-7: `scatter` and `weave` hand-roll the balanced fork expansion `Party::forks` provides
- Where: crates/before/src/meter/board/family.rs:836-845 (related: family.rs:887-896; meter.rs:3123-3132; party/forks.rs:1-10, 105-110)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (`grep -rnE 'while (parties|clocks)\.len\(\) <'` across src, tests, benches, examples: 12 sites; forks.rs read for the residual-first preorder); executed: no
- Seen by: structure-prose; refutation: confirmed (with the correction that forks.rs's "footgun" is repeated fork on one party, which this loop does not do); history: no-rationale-found (`Party::forks` predates both loops)
- Owner-gated: no

The doubling loop is written twice in this file and again in meter.rs and four test files, while `Party::forks(k)` yields the same preorder leaves of a minimal-depth tree with the residual as the first preorder leaf. Prefer the crate's own mechanism over a hand-rolled copy (Principle 3); a shared helper also lets `scatter`'s order-dependence (evens before odds) rest on one documented preorder. At non-power-of-two `n`, `scatter` truncates a complete tree (846-848) whereas `forks` gives a minimal-depth partition; `forks(next_power_of_two - 1)` then `truncate` reproduces today's shape. Confidence medium: I read forks.rs's residual and preorder documentation but did not trace `Split` step by step.

Evidence:

       836	        let mut parties = vec![Party::seed()];
       837	        while parties.len() < n {
       838	            let mut next = Vec::with_capacity(parties.len() * 2);
       839	            for mut p in parties {
       840	                let q = p.fork();
       841	                next.push(p);
       842	                next.push(q);
       843	            }
       844	            parties = next;
       845	        }

Resolution: One `pub(crate) fn balanced_leaves(n: usize) -> Vec<Party>` in `meter` (or `[Party::seed()].into_iter().chain(first.forks(n as u64 - 1))` directly), used by family.rs, meter.rs, and the tests. Acceptance: one balanced-expansion implementation in the instrument code; `scatter` and `weave` produce byte-identical bundles at power-of-two `n` (compare `study_family_versions(1.0)` before and after).

### board-families-floors-judge-8: "deterministic-liveness" floors are undefined, spelled with "today" in six rendered strings, and contradicted by the module's opening sentence
- Where: crates/before/src/meter/board/floors.rs:5-8 (related: floors.rs:41-42, 64-69, 171-183, 216-221, 230-266, 287-293; board.rs:88-91, 102-104; currency.rs:118-119, 138-140; operand.rs:17)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -rn 'deterministic-liveness' crates/before/src` finds uses in floors.rs, currency.rs:119, operand.rs:17 and no definition; `grep -nw today floors.rs` gives 175, 182, 220, 233, 265, 291, all inside `WHY_` strings; board.rs:88-91 read); executed: no
- Seen by: scaffolding, structure-prose (and the "today" half of the scaffolding vocabulary sweep); refutation: confirmed; history: deliberate-and-holds (both floor classes are intended and the class's purpose is stated at board.rs:102-104 and floors.rs:64-69; only the definition and the opening sentence are missing)
- Owner-gated: no (a reconciliation of two in-code statements, not a design change)

The module opens by stating every floor is derived from what the operation must do, "never from how it does it" (board.rs:88-91 says the same), yet nine floors are pinned to the shipped kernel's mechanism under the tag "deterministic-liveness", a coinage defined nowhere, and six of their rendered legend strings carry the relative date "today". The distinction between a floor no conforming implementation can undercut and a floor pinned to the current kernel is the first thing a maintainer reading a floor trip needs, and the file's first sentence tells them the second kind does not exist. Principle 5 (dated rationale at a declaration site) and the vocabulary rule (a coined term is anchored to an identifier or defined once by contrast).

Evidence:

         5	//! A floor states the least a watching counter can honestly read, derived from
         6	//! what the operation must do, never from how it does it; the board module
         7	//! doc's Liveness floors section carries the criterion and what a trip means.

        41	//! - **Touch** floors are deterministic-liveness declarations, like the
        42	//!   fork rows' heap floor, at three derivations. The single-operand

       173	pub(super) const WHY_LIMB_RANK_ENCODE: &str =
       174	    "deterministic-liveness: the encoder extracts and biases the integral part through \
       175	     one arithmetic pass over the numerator today, one op per 64 numerator bits; a \
       176	     pure bit-walk emission (riding the bias as a carry) would lower this floor \
       177	     deliberately";

Resolution: Define the two kinds once, by contrast, at the top of the module doc (a contract floor, derived from mandatory work and sound for every conforming implementation; a kernel-pinned floor, derived from the shipped kernel's mechanism and lowered deliberately by a re-representation, committed so state migrating off the meter trips red), and fix the opening sentence here and board.rs:88-91 to admit both. Delete "today" from the six `WHY_` strings and currency.rs:139: the trailing "would lower this floor deliberately" clause already carries the mutability. Consider anchoring the kind to an identifier (a `FloorKind` field on `Liveness::Floor`, or one shared prefix constant) so the legend's tag is not free prose. Acceptance: `grep -nw today crates/before/src/meter/board` returns nothing; the module doc defines both kinds before using either; board.rs's liveness section no longer says "never from how it does it" without qualification.

### board-families-floors-judge-9: Three floors.rs sentences disagree with the code or number they describe
- Where: crates/before/src/meter/board/floors.rs:23-26 (related: floors.rs:127-129, 531-537, 712, 728; ceilings.rs:137-139; ops.rs:1329-1333, 1616-1620)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read the seven `NA_SCAN_*` constants against lines 23-26; ceilings.rs:137-139; `membership_floors` at 719-728); executed: no
- Seen by: adequacy, instrument-correctness (the `w ≤ v` slip), refutation (the scan-NA enumeration, raised as new); refutation: confirmed; history: no-rationale-found
- Owner-gated: no

(a) Lines 23-26 say scan not-applicable "is reserved for" wholesale byte moves or compares and operands with no packed stream, yet the file defines four further scan-NA reasons (NA_SCAN_SEED_PARTY, NA_SCAN_SEED_PROJECTION, NA_SCAN_RANK_BYTES, NA_SCAN_TEXT_REJECTION): a hand-maintained enumeration that drifted. (b) WHY_SCAN_TOUCH and the SCAN_TOUCH_FLOOR_BITS doc say the early exit "still reads the operands' root codes" (plural), but the constant is 2 bits, one operand's root code; on the pair rows the derivation yields 4. The same constant is exact on the single-operand fork rows, so the fix is the pair rows' wording (or a pair constant), not a blind change to 4. (c) `membership_floors`'s doc writes "`w ≤ v`, strictly" where the code tests `Some(Ordering::Greater)`, i.e. `w < v` (equality is the arm above). Principle 5: prose states what IS.

Evidence:

        23	//!   Not-applicable is reserved for operations
        24	//!   whose contract is a wholesale byte move or compare (encode, hash,
        25	//!   same-form equality) or whose operands have no packed stream at all
        26	//!   (the rank pair).

       127	/// Scan floor: early exit is legitimate, but the root codes are still read.
       128	const WHY_SCAN_TOUCH: &str =
       129	    "may answer at the first divergence: still reads the operands' root codes";

       712	/// - **The query refuses `w`** (`w ≤ v`, strictly). Refusal certifies

       728	    if v.partial_cmp(w) == Some(Ordering::Greater) {

Resolution: (a) State the structure ("not-applicable where the contract forces no metered stream work; each `NA_SCAN_` constant carries its reason") or list all seven. (b) "at least one operand's root code", or a 4-bit pair constant. (c) Write `w < v`. Acceptance: the three sentences agree with the constants and code they describe.

### board-families-floors-judge-10: Not-applicable declarations whose rendered reasons the code contradicts, leaving `clock_fork` unwatched on every version-only family
- Where: crates/before/src/meter/board/floors.rs:60-64 (related: floors.rs:143-145, 207-221, 289-296, 302-319; ops.rs:1306-1340, 1608-1626; family.rs:1058-1059; version/skyline/validate.rs:80-110; version/skyline/signed.rs:203-210; suanpan/src/accumulator.rs:219-226, 1253-1266, 1369-1373; suanpan/src/touch_meter.rs:9-12; party.rs:121-130, 233-237; party/ops/split.rs:20-27; idbits.rs:84-90, 132-140; clock.rs:153-157)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read every call path named below; no cargo run); executed: no
- Seen by: adequacy, structure-prose, instrument-correctness; refutation: confirmed (and extended: WHY_TOUCH_WIDE_STREAM carries the same false clause); history: no-rationale-found (the decode-row NA predates the quick register; NA_SCAN_SEED_PARTY was false at introduction; the two fork rows have carried different genres for one mechanism since 1a5e57e5)
- Owner-gated: no

Three floor sites declare not-applicable on the strength of a mechanism claim the code contradicts, so a column (or a whole cell) goes unwatched where the row's own deterministic-liveness genre yields a positive floor. (a) The decode rows' touch: the module doc, NA_TOUCH_LAZY_BATCH, and the parenthetical inside the rendered WHY_TOUCH_WIDE_STREAM say word-scale deltas "batch in the accumulator's lazy zone and force no digit touches"; but `validate_from` folds every leaf after the first through `fold_signed_int`, which routes word-scale values to `add_u64`/`sub_u64`; those skip only `delta == 0` and otherwise call `quick_add` (`touch(1)`) or `add_at` (`touch(1)` per digit), and touch_meter.rs documents the register absorb as exactly one touch. Every nonzero word-scale delta therefore records at least one touch, and the floor the tick and query rows already commit (`touch_delta_fold(stored_nonzero_deltas(v))`) binds on `version_decode`/`clock_decode` too; on every all-narrow family they declare NA instead. (b) The seed party's scan: NA_SCAN_SEED_PARTY says "its packed form is empty", but `Party::seed()` is the 2-bit terminal tag `00` in one static byte, and `Party::fork` -> `view().split()` -> `IdReader::split` begins with `self.peek()`, which for a nonempty view records 2 scan bits; `scan_touch()` (2 bits) is met exactly. The empty stream is the anonymous id, never a `Party`. (c) `clock_fork`'s heap: NA_HEAP_FORK_SHARES says "no other allocation is semantically forced", but `Clock::fork` calls `self.party.fork()`, the same party-half materialization `party_fork` floors as WHY_HEAP_FORK_HALF; one mechanism carries two genres across two rows. Consequence of (b)+(c): `clock_fork` on every version-only family (where `FamilyData::clock` pairs the seed party, family.rs:1058-1059) is NA on all five columns and is absent from the exposure disclosure at floors.rs:99-106 (finding 11). The floors module's own rule (lines 5-7) is that a floor states the least a watching counter can read and an NA is sound only when no floor can bind; instruments before cures.

Evidence:

        60	//!   under the same premise). The validator batches word-scale deltas in the accumulator's
        61	//!   lazy zone, so the decode rows floor only what it must fold digit by
        62	//!   digit: one touch per 64 bits of every stored code wider than the
        63	//!   machine-word bound (the stream-derived
        64	//!   convention the tick rows' limb floor uses). Either floor is what a

       295	const NA_TOUCH_LAZY_BATCH: &str = "every stored code fits the machine-word bound: word-scale \
       296	     deltas batch in the accumulator's lazy zone and force no digit touches";

       144	pub(super) const NA_SCAN_SEED_PARTY: &str =
       145	    "the forked party is the seed: its packed form is empty";

       213	pub(super) const NA_HEAP_FORK_SHARES: &str = "the forked child's version hand-over is a \
       214	     refcount bump on the shared stored buffer, never a byte copy, and no other allocation \
       215	     is semantically forced";

    validate.rs:
       100	        if seen_leaf {
       101	            zero_delta = code.is_zero();
       102	            let (sign, magnitude) = unzigzag(code);
       103	            fold_signed_int(&mut height, sign, &magnitude);

    signed.rs:
       205	        (Sign::Positive, Int::Small(n)) => acc.add_u64(*n),
       206	        (Sign::Negative, Int::Small(n)) => acc.sub_u64(*n),

    suanpan accumulator.rs:
       219	    pub fn add_u64(&mut self, delta: u64) {
       220	        if delta != 0 {
       221	            let delta = i128::from(delta);
       222	            if !self.quick_add(delta) {
       223	                self.add_at(0, delta);
      1253	    fn quick_add(&mut self, delta: i128) -> bool {
      1254	        let Some(held) = self.quick else {
      1255	            return false;
      1256	        };
      1257	        touch(1);

    touch_meter.rs:
         9	//! unit every cost on the crate page is denominated in. The quick
        10	//! register meters too, though it holds no digits: a delta, sign query,
        11	//! negation, or shift the register absorbs counts exactly one touch,

    party.rs:
       122	        // The seed id is exactly the 2-bit terminal tag `00` (the whole
       123	        // interval, owned), marker-padded to the one static byte
       124	        // `0b0010_0000`: construction allocates nothing, and every seed

    split.rs:
        22	        if let IdNode::Empty = self.peek() {

    idbits.rs:
       135	            IdReader::At { bits, pos } => {
       136	                crate::codec::scan::record_bits(2); // one 2-bit tag scanned

    clock.rs:
       153	    pub fn fork(&mut self) -> Clock {
       154	        let child_party = self.party.fork();

    ops.rs:
      1612	                let floors = Floors {
      1613	                    heap: na(NA_HEAP_FORK_SHARES),
      1614	                    limb: na(NA_LIMB_NOT_FORCED),
      1615	                    segments: seg_ceiling_only(),
      1616	                    scan: if clock.party().is_seed() {
      1617	                        na(NA_SCAN_SEED_PARTY)
      1618	                    } else {
      1619	                        scan_touch()
      1620	                    },
      1621	                    touch: na(NA_TOUCH_NOT_FORCED),

Resolution: (a) Floor the decode rows' touch at the max of `touch_delta_fold(stored_nonzero_deltas(v))` and the wide-stream limb count, and reword lines 60-64, NA_TOUCH_LAZY_BATCH, and the clause inside WHY_TOUCH_WIDE_STREAM to the meter's actual accounting (a zero delta is skipped; a nonzero word-scale delta costs one register or digit touch). (b) Replace NA_SCAN_SEED_PARTY with `scan_touch()` at ops.rs:1329-1333 and 1616-1620 and delete the constant. (c) Give `clock_fork` the party half's WHY_HEAP_FORK_HALF floor as `party_fork` does (the child's packed bytes, probed at prepare), or move both rows to one genre with the reason stated. Acceptance: with the counter features, a probe that decodes `dense(1_000)`'s bytes with the touch counter reset reads `touches() >= stored_nonzero_deltas(&v)`, and a `Sample` with `touch: Some(0)` under the new floor reads TOUCH_FLOOR_TRIP through `evaluate`; `Party::seed().fork()` with the scan counter reset reads `scan_bits() >= 2`; the `clock_fork` cells on version-only families render at least one `flr[..]` entry; `just amp-board-acceptance` stays green (any new red is a genuine finding to triage).
Construction: (a) In board/tests.rs under `limb-meter`: `let v = version_of(&dense(1_000)); suanpan::touch_meter::reset(); let _ = Version::decode(&v.encode()); assert!(suanpan::touch_meter::touches() >= stored_nonzero_deltas(&v));` passes today by the reading above, while `touch_wide_stream(&v)` returns `NotApplicable`: a validator moved onto an unmetered `i128` height would read 0 touches and stay green on every all-narrow family. (b) Under `scan-meter`: `crate::meter::reset_scan_bits(); let mut p = Party::seed(); let _ = p.fork(); assert!(crate::meter::scan_bits() >= 2);` while the cell declares "its packed form is empty". (c) Reset the peak allocator, `Clock::from_parts(Party::seed(), v).fork()`, observe peak >= the child party's packed byte: the allocation `party_fork` floors and `clock_fork` declares unforced.

### board-families-floors-judge-11: The "four cells watched by neither leg" disclosure is a hand count with no pin on either side, already stale, and its 10 µs restates a benchjudge constant
- Where: crates/before/src/meter/board/floors.rs:99-106 (related: floors.rs:92-97, 107-112, 431-439; board.rs:99-100; ops.rs:1904, 1924, 2065, 2085, 2142, 1608-1626; tools/benchjudge:146-160; tools/benchjudge-expected.json)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read the five text-rejection rows passing all-NA through `text_rejection_floors`; tools/benchjudge:146-160; no `NotApplicable` match in board tests outside render/shard; the hash and eq rows' all-NA status is as the disclosure itself states, assessed; the sub-microsecond claim for `clock_fork` on a seed party is inferred, not timed); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed (with the note that the disclosure's criterion is two-part, all-NA and sub-floor, and neither half is pinned); history: no-rationale-found (the count grew by hand: "three cells to four" in b35e3f00)
- Owner-gated: no

The disclosure names four cells as watched by neither the deterministic floors nor the time leg. Nothing enumerates the all-NA cells (the five text-rejection rows are all-NA too, disclosed separately at 92-97 with the time leg claimed to carry them, and finding 10 shows `clock_fork` on every version-only family is all-NA and sub-microsecond), and nothing pins the bench judge's sub-floor set (tools/benchjudge-expected.json pins only `red`). "10 µs" is `MIN_JUDGED_MEDIAN_NANOS = CRITERION_TIMER_ERROR_NANOS * RESOLUTION_DOMINANCE_FACTOR` restated as a literal. Principle 6 (every hole found becomes a committed check, never a convention held in memory) and the no-hand-maintained-counts rule: an exposure roster kept in prose grows silently while the prose keeps saying four.

Evidence:

        99	//! Four cells are watched by neither leg, an exposure accepted here so it is
       100	//! stated rather than silent: `version_hash`, `party_hash`, `clock_hash`, and
       101	//! `version_eq` on the benign family. Hashing folds the stored canonical bytes
       105	//! benign operands are small enough (a few hundred packed bytes across both
       106	//! scales) that the body never reaches the bench judge's 10 µs judgment floor.

    board.rs:
        99	//! constructors that commit them, along with the disclosure of the four cells
       100	//! no deterministic leg watches.

    tools/benchjudge:
       157	# The judgment floor: cells whose larger-scale median sits below this are
       158	# enumerated but never judged. Derived, not calibrated:
       159	# CRITERION_TIMER_ERROR_NANOS x RESOLUTION_DOMINANCE_FACTOR = 10 us.
       160	MIN_JUDGED_MEDIAN_NANOS = CRITERION_TIMER_ERROR_NANOS * RESOLUTION_DOMINANCE_FACTOR

Resolution: Add a test beside the board tests that builds every board bundle, collects every cell whose `Floors` are all `NotApplicable`, and asserts the set equals a committed roster (the four named, the five text-rejection rows, and the `clock_fork` cells finding 10 leaves all-NA until it is fixed, or the smaller set once it is); have the disclosure here and at board.rs:99-100 state the class and cite that roster and `MIN_JUDGED_MEDIAN_NANOS` by name instead of "four" and "10 µs". If the wall-time half matters, have tools/benchjudge emit its sub-floor cell set and pin it in tests/bench_judge_roster.rs. Acceptance: a committed test fails when a new op declares NA on every floored currency without joining the roster; floors.rs:99-112 and board.rs:99-100 carry no cell count and no duration literal.
Construction: Add a row to ops.rs whose `prepare` returns `Floors` with `na(..)` on heap, limb, scan, and touch, or observe that `clock_fork` on the dense family already does: nothing in the gate changes and floors.rs still reads "Four cells".

### board-families-floors-judge-12: The same two derivations are restated up to seven times across floors.rs and operand.rs
- Where: crates/before/src/meter/board/floors.rs:159-162 (related: floors.rs:27-40, 49-60, 237-245, 302-308, 458-478, 494-504, 579-584, 773-781; operand.rs:47-56, 127-137)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read); executed: no
- Seen by: structure-prose (and the judge.rs half of the same lens finding, handled in finding 23); refutation: confirmed; history: partially deliberate (50a017d0 restated the touch derivations in the module doc because private intra-doc links have no path in the public build; the constructor-level repeats have no rationale)
- Owner-gated: no

The stream-codes-not-tree-values limb argument ("a plateau of equal wide leaves stores its width once") is written at floors.rs:27-40, 159-162, 302-308, 579-584, 773-781 and operand.rs:47-56, 127-137; the pair-fold max-not-sum argument at floors.rs:49-60, 237-245, 458-478, 494-504. Doc altitude: every sentence competes with the contract the reader came for, and a derivation with seven homes drifts (one site says "provably need not", another "legitimately"). The module-doc copy is deliberate (a link-check constraint); the constructor and operand.rs copies are not.

Evidence:

       159	const WHY_LIMB_STREAM: &str = "every payload code of the stored stream wider than the \
       160	     machine-word bound must be decoded limb by limb: one op per 64 code bits (the stream's \
       161	     own codes, not the decoded tree's values — a plateau of equal wide leaves stores its \
       162	     width once)";

        31	//!   stored payload code wider than [`MACHINE_WORD_MAGNITUDE_BITS`](super::ceilings::MACHINE_WORD_MAGNITUDE_BITS) — a
        32	//!   plateau of equal wide leaves stores its width once and steps by
        33	//!   unit deltas after, and a conforming walk provably need not

Resolution: Give each derivation one home (the constructor whose `why` string it justifies: `limb_stream`, `touch_pair_fold`) and have the sibling constructors and operand.rs point at it by name; keep the module doc's prose statement (the link constraint) and the rendered strings to the one-line mechanism. Acceptance: `grep -c 'stores its width once' crates/before/src/meter/board/*.rs` is at most 2 (the module doc and the constructor).

### board-families-floors-judge-13: floors.rs re-inlines its own helpers: five scan-floor casts, twin limb constructors, eight near-identical `Floors` literals, six zero-or-NA shapes, two rate types for one dimension
- Where: crates/before/src/meter/board/floors.rs:372-383 (related: floors.rs:388-399, 413-424, 523-529, 560-577, 585-612, 614-620, 644-651, 668-689, 718-739, 751-768, 782-793; ceilings.rs:135, 145; operand.rs:124, 296; ops.rs:1318-1326)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read every site; `(.. as f64 * SCAN_FLOOR_BITS_PER_INPUT_BYTE) as u64` at 378, 394, 419, 526, 566; `Liveness::NotApplicable { reason: NA_LIMB_NARROW }` spelled out at 587-589 and 603-605 where `na()` exists at 623); executed: no
- Seen by: scaffolding, adequacy (the zero-minimum `Floor`), structure-prose, instrument-correctness (the rate types); refutation: confirmed; history: no-rationale-found (7bd84b49 copied the literals; the f64 rate has been 1.0 since its origin)
- Owner-gated: no (making the `pub` rate constant a `u64` is the one owner-call inside it)

The expression `(bytes as f64 * SCAN_FLOOR_BITS_PER_INPUT_BYTE) as u64` appears five times beside a `scan_examines` that fixes the `why`; `limb_stream` and `limb_wide` are identical except for their `why` string and spell out the variant `na` wraps; eight `Floors` literals share `heap: na(NA_HEAP_IN_PLACE), limb: na(NA_LIMB_NOT_FORCED), segments: seg_ceiling_only()`, with the concurrent arm of `comparison_floors` byte-identical to the else arm of `masked_cmp_floors` and the two equal-pair arms identical; the zero-then-NA shape is hand-written six times while `scan_examines`, `heap_materializes`, and the rejection constructors construct `Floor { min: 0 }` on a zero argument (never reached today, a uniformity point); the scan rate is an `f64` (`SCAN_FLOOR_BITS_PER_INPUT_BYTE: f64 = 1.0`) while the tick rate is a `u64` used with `saturating_mul`, and operand.rs:124 narrows with `as usize` where :296 uses `usize::try_from`. Legibility ("obviously, reviewably correct"): the reader diffs eight literals to see that only scan and touch vary, and a convention change touches every copy. While consolidating, pick one placement convention for the message constants (lines 124-300 are one mixed block; from 401 each constant sits beside its constructor, with `touch_wide_stream` wedged into the block at 309-319).

Evidence:

       372	pub(super) fn rejection_floors(fed_bytes: usize, why: &'static str) -> Floors {
       373	    Floors {
       374	        heap: na(NA_HEAP_REJECTION),
       375	        limb: na(NA_LIMB_REJECTION),
       376	        segments: seg_ceiling_only(),
       377	        scan: Liveness::Floor {
       378	            min: (fed_bytes as f64 * SCAN_FLOOR_BITS_PER_INPUT_BYTE) as u64,
       379	            why,
       380	        },

       585	pub(super) fn limb_stream(mandatory_limbs: u64) -> Liveness {
       586	    if mandatory_limbs == 0 {
       587	        Liveness::NotApplicable {
       588	            reason: NA_LIMB_NARROW,
       589	        }

    ceilings.rs:
       135	pub const SCAN_FLOOR_BITS_PER_INPUT_BYTE: f64 = 1.0;
       145	pub(super) const TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE: u64 = 8;

Resolution: `fn scan_floor(bytes: usize, why: &'static str) -> Liveness` routing `scan_examines` and the three rejection constructors and `sync_floors`; `fn floor_or_na(min: u64, why, na_reason) -> Liveness` for the six zero-or-NA sites, with `limb_stream`/`limb_wide` collapsing into it; `fn in_place(scan: Liveness, touch: Liveness) -> Floors`, `fn equal_pair() -> Floors`, and `fn witness(touch_na: &'static str) -> Floors` for the literals; make `SCAN_FLOOR_BITS_PER_INPUT_BYTE` a `u64` multiplied with `saturating_mul` as the tick floor does (owner's call: the constant is `pub`); `usize::try_from` at operand.rs:124. Acceptance: one occurrence of `SCAN_FLOOR_BITS_PER_INPUT_BYTE` in an expression; `grep -c 'Liveness::NotApplicable {' floors.rs` is 1 (inside `na`); no site constructs `Liveness::Floor` with a possibly-zero `min` directly; the smoke board's rendered legend is byte-identical before and after.

### board-families-floors-judge-14: `touch_pair_fold`'s doc overclaims that a dead touch meter trips on every committed pair family
- Where: crates/before/src/meter/board/floors.rs:474-478 (related: floors.rs:479-492, 668-679; family.rs:510-516, 797-803; meter.rs:204-210; operand.rs:39-42)
- Class / severity / confidence: claim / low / high
- Provenance: assessed (read: hugeleaf is one leaf flag plus one gamma code; the post-pass ticks it at the seed, leaving one leaf; `stored_nonzero_deltas` counts only payloads after the first, so both operands read 0 and the constructor returns `na(NA_TOUCH_NO_DELTAS)`, which `comparison_floors` carries on a comparable pair); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no-rationale-found (false or undefined from be6dd6a0 on; "pair family" has a narrower module-local sense at family.rs:413-415 under which it might hold, but nothing pins either reading)
- Owner-gated: no

The sentence is the liveness argument for the touch column: because the floor is positive wherever either operand stores a nonzero delta, "a dead touch meter still trips it on every committed pair family". The hugeleaf column is a committed board family whose two operands store no deltas at all, so the constructor returns NA there and a dead touch meter is not tripped on its comparison, join, or meet cells. The conditional half is correct; the universal is a hand-maintained roster claim no test pins.

Evidence:

       474	/// sign-read traffic as mandatory and ban the efficiency. The floor stays
       475	/// strictly positive wherever either operand stores a nonzero delta, so a dead
       476	/// touch meter still trips it on every committed pair family. Equal operands
       477	/// are answered by canonical byte identity before any sweep runs (`a ∨ a = a`,

Resolution: Drop the universal clause and state the conditional ("positive wherever either operand stores a nonzero delta; a delta-free pair such as the single-leaf hugeleaf column declares NA"), or, if every pair family is meant to carry a live touch floor, pin it with a test over `FamilyId::board()`'s version pairs and give hugeleaf a counterpart that stores a delta. Acceptance: the doc asserts no property of every committed family, or a committed test iterating the board's version pairs asserts `touch_pair_fold` is `Liveness::Floor` on each and passes.
Construction: Build the hugeleaf bundle (`FamilyData::build(FamilyId::Hugeleaf, 1.0, 0)`), decode `version` and `version2`, and assert `matches!(touch_pair_fold(&v, &w), Liveness::NotApplicable { .. })`: it holds today, contradicting the sentence.

### board-families-floors-judge-15: Vocabulary: "honest" as an undefined soundness criterion (29 sites, two rendered), "mint" (7), "backstop", "trued to", "door", "seams", "genre"
- Where: crates/before/src/meter/board/floors.rs:541-544 (related: floors.rs:5, 112, 136, 148, 210, 629-630; family.rs:5, 288, 293, 418, 448, 816, 1108, 1123, 1145, 1175; judge.rs, operand.rs)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep counts: `honest(ly)` 18/7/2/2/0 in floors/family/judge/operand/measure; `mint*` 7, all family.rs; `backstop` floors.rs:112, 136; `trued` 148, 210; `door` family.rs:5; `seams` 288, 293; `genre(s)` 10; crate-wide 167 `honest` and 66 `mint*`); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed (measure.rs carries no `honest` hit, only the identifier `assert_honest_text`); history: no-rationale-found (the rules entered writing-style.md on 2026-08-19, after every site; "genre" is the working vocabulary of .cargo/mutants.toml's header)
- Owner-gated: yes (crate-wide convention: a partition-local edit leaves the meter suite speaking two dialects; one owner ruling is the right unit)

"honest"/"honestly" carries a technical meaning here (a floor no conforming implementation can read below; a declaration that reflects real work) that is defined nowhere, and two sites render in the legend (WHY_SCAN_SYNC_VERSIONS "so no party-bytes floor is honest"; NA_SEG_CEILING_ONLY "the honest floor is zero"). "mint" is used for constructing a value at seven family.rs sites, two of them `assert!` messages. "backstop", "trued to", "door", and "seams" are register transplants with no adversary, calibration, or physical seam behind them. The vocabulary rules name each; the load-bearing one is "honest", which deserves a definition rather than 29 repetitions.

Evidence:

       541	const WHY_SCAN_SYNC_VERSIONS: &str = "the reconciliation's version join must read both \
       542	     version streams in full to emit their union; the party leg guarantees only its root \
       543	     tags — the fused sum-split splices a subtree owned by one side alone without \
       544	     scanning its nodes, so no party-bytes floor is honest";

       629	const NA_SEG_CEILING_ONLY: &str = "ceiling-only by policy: the target is walks that never grow \
       630	     the stack, so the honest floor is zero and a zero floor asserts nothing";

    family.rs:
      1121	    assert!(
      1122	        decode_party(&a).is_disjoint(&decode_party(&b)),
      1123	        "the disjoint-mount adapter must mint a disjoint pair"
      1124	    );

Resolution: Define the criterion once in the floors module doc ("a floor is sound when every conforming implementation reads at least it; a not-applicable declaration is sound when the contract forces no metered work") and use "sound"/"forced"/"mandatory" or the mechanism at the sites, the two rendered strings first; "mint" -> "build"/"derive"; "backstop" -> "the one leg that bounds"; "trued to" -> "matches"; "door" -> "constructor"; "seams" -> "the two kernels"; "genre" is established in-repo and may stay by ruling. Acceptance: no `honest` inside a rendered `WHY_`/`NA_` string; `grep -nwiE 'mint|mints|minted' family.rs` is empty; the owner's ruling on the crate-wide sweep is recorded.

### board-families-floors-judge-16: Policy ceremony restated at every site: 50 `segments: seg_ceiling_only()` fields, five `Cell` fields copied one by one into `Sample`, twelve cfg reader wrappers
- Where: crates/before/src/meter/board/floors.rs:632-636 (related: ops.rs (38 sites); measure.rs:124-142, 145-209; cell.rs:108-159; currency.rs:13-25, 133-141; meter.rs:3574-3583, 3714-3723; board/tests.rs:438-456, 776-801)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (`grep -c 'segments: seg_ceiling_only()'` = 38 in ops.rs and 12 in floors.rs; measure.rs:129-134 and 145-209 read; meter.rs's readers exist only under their features); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed (count corrected to 50); history: deliberate-and-holds for the per-cell segments declaration (96c5af54: an explicit declaration instead of a missing field), so this is a refinement preserving the totality tie, not a correction
- Owner-gated: yes (touches currency.rs's stated totality argument; Option-returning readers touch the `meter` feature's public surface, which crates/before/AGENTS.md declares stable)

Every `Floors` literal restates `segments: seg_ceiling_only()`, a policy constant that is not per-cell information; a constructor `Floors::new(heap, limb, scan, touch)` applying the policy keeps the compile-time tie (its arity changes when a currency joins the axis). `measure()` moves `floors`, `fold_arity`, `fold_search_bits`, `declared_heap`, `declared_limb` from `Cell` into `Sample` field by field, so every new declaration touches `Cell`, `Sample`, `measure`, and the four `Sample` literals in tests.rs; a `Declared` struct owned by `Cell` and moved whole dissolves the cascade. The limb/touch/scan reset-and-read pairs are twelve one-line `#[cfg]` functions because meter.rs's readers exist only under their features; Option-returning readers (or no-op resets) in meter.rs would dissolve them, if the reader shape is not part of the stable surface. Principle 3: a convention restated fifty times is maintenance cascade, not information.

Evidence:

       632	/// The segments currency's declaration: ceiling-only by policy, on every
       633	/// cell.
       634	pub(super) fn seg_ceiling_only() -> Liveness {
       635	    na(NA_SEG_CEILING_ONLY)
       636	}

    measure.rs:
       129	        floors: cell.floors,
       130	        fold_arity: cell.fold_arity,
       131	        fold_search_bits: cell.fold_search_bits,
       132	        heap_model,
       133	        declared_heap: cell.declared_heap,
       134	        declared_limb: cell.declared_limb,

       145	/// Reset the limb counter when the `limb-meter` feature carries one.
       146	#[cfg(feature = "limb-meter")]
       147	fn reset_limb() {
       148	    meter::reset_limb_ops();
       149	}
       150	
       151	/// Without the `limb-meter` feature there is no counter to reset.
       152	#[cfg(not(feature = "limb-meter"))]
       153	fn reset_limb() {}

Resolution: (a) A `Floors` constructor taking the four derived declarations and applying the segments policy, used at every site (the struct literal stays available for the all-NA probe tests). (b) `pub(super) struct Declared { floors, fold_arity, fold_search_bits, declared_heap, declared_limb }` owned by `Cell` and moved into `Sample` as one field. (c) If the `meter` reader shape is not stable surface, `limb_ops() -> Option<u64>` / `scan_bits() -> Option<u64>` and no-op resets under `cfg(not(feature))` so measure.rs reads them unconditionally; otherwise collect the wrappers into one `const OPTIONAL_METERS: [Counter; 3]`. Acceptance: `grep -rc 'segments: seg_ceiling_only()' crates/before/src/meter/board` is near zero; adding a sixth currency still fails to compile at every construction site; measure.rs has at most one cfg pair per optional counter.

### board-families-floors-judge-17: The tick rows' 8-bits-per-byte scan floor is derived from a single examination that records fewer bits than the floor, and nothing states why these rows alone floor at the full rate
- Where: crates/before/src/meter/board/floors.rs:782-793 (related: floors.rs:226-229, 773-775; ceilings.rs:56-62, 129-145; board.rs:106-113; codec/scan.rs:9-17; codec/build.rs:83, 95, 113, 128, 145, 180; version.rs:1129-1131)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (scan.rs's recording rules and build.rs's six `record_bits` sites read; the size law at version.rs:1129-1131; `git show -s 500d4d09` lists "the tick walk's 2-5x floor margin" among kept calibrations; 3b2d00c9's message and bd75d025's rewording per the history pass); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: reframed (the derivation wording is imprecise by at most 16 bits per pair; the shipped fill walk clears the floor because the builder records emitted writes; the 2-5x aside is a deliberately kept calibration, so that part is refuted); history: deliberate-and-holds for both rates, with no stated reason why the tick rows differ
- Owner-gated: yes (the eighth-versus-full choice is measurement policy)

WHY_SCAN_TICK_WALK derives 8 bits per input byte as "the walk's irreducible single examination", but a single examination through the metered cursors records the live bits (`encoded_bits()`), which are strictly fewer than 8 times `encode().len()` for every operand (the marker bit and padding: 1 to 8 bits short per operand). The floor holds because the codec builder also records every emitted bit, which the derivation does not name; a metered read-only pass would sit under it. Separately, every other full-examination row floors at `SCAN_FLOOR_BITS_PER_INPUT_BYTE = 1`, defended at board.rs:106-113 as the vacuity-detection eighth, and neither constant's doc says why the tick rows alone carry the full rate (my reading: because the fill walk emits output whose writes are recorded, roughly doubling a read-only walk's count, so the full rate is only sound where emission is metered). The 2-5x margin at 773-775 is a kept calibration by the owner's ruling, but ceilings.rs:56-62 states the no-readings convention without the kept-calibration class the ruling names, so a reader cannot tell the reading is sanctioned.

Evidence:

       227	const WHY_SCAN_TICK_WALK: &str = "the paired fill walk examines every topology bit and payload \
       228	     code of both operands at least once: 8 bits per input byte, the walk's irreducible single \
       229	     examination";

       787	        scan: Liveness::Floor {
       788	            min: (packed_bytes as u64).saturating_mul(TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE),
       789	            why: WHY_SCAN_TICK_WALK,
       790	        },

    ceilings.rs:
       132	/// One bit per byte is an eighth of the stored bits: far below any honest full
       133	/// walk (measured ~8 bits per byte across the board), and far above a counter
       134	/// that has stopped watching (which reads ~0).
       135	pub const SCAN_FLOOR_BITS_PER_INPUT_BYTE: f64 = 1.0;

    version.rs:
      1129	    /// The exact length in bits of [`encode`](Self::encode) before its
      1130	    /// padding — the marker bit and zero-pad to the byte boundary, so
      1131	    /// `encode().len()` is `(encoded_bits() + 1).div_ceil(8)`.

Resolution: Either floor the tick rows at the exact single examination (`version.encoded_bits() + party.encoded_bits()`, still eight times the universal floor) and restate WHY_SCAN_TICK_WALK in live bits, or keep 8 per byte and name the recorded emission in the derivation. State at `SCAN_FLOOR_BITS_PER_INPUT_BYTE` (or `TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE`) why the tick rows alone take the full rate. Add the kept-calibration class to the ceilings.rs:56-62 convention header so the 2-5x margin reads as sanctioned. Acceptance: the WHY string derives the number it states; the two scan-floor constants' docs explain their relation; the convention header names what readings may remain.
Construction: For any cross bundle, `8 * (v.encode().len() + p.encode().len()) - (v.encoded_bits() + p.encoded_bits())` is at least 2; a metered read-only pass (a `DsiCursor` skipping each code once over `v` plus an `IdReader` over `p`, no builder) records exactly the live-bit sum and reads below the committed floor.

### board-families-floors-judge-18: `trend`'s docstring claims a lumpy counter "errs red, never green"; a counter dark at the larger point reads green
- Where: crates/before/src/meter/board/judge.rs:33-36 (related: judge.rs:37-54)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (my transcription of `trend` run in python: `trend([(100,5),(200,0)]) = -2.322`, `trend([(100,0),(200,5)]) = +2.322`; no cargo run); executed: no (a python transcription, not the Rust function)
- Seen by: adequacy; refutation: confirmed; history: no-rationale-found (added by 248d5539 as a doc clarification with no analysis of the direction)
- Owner-gated: no

The clamp `max(m, 1)` steepens the fit only when the zero is at the smaller denominator; a counter reading nonzero at the small size and zero at the large size yields a negative slope (green on the exponent leg) and a zero constant (green), so only a declared floor catches it. The following clause already hands vacuously quiet counters to the floors, which limits the exposure to NA-declared columns, but the universal "never green" is false as stated (statement faithfulness).

Evidence:

        33	/// variance) score 0. A sparse, lumpy counter therefore errs red, never
        34	/// green: its clamped zeros steepen the fit toward the exponent ceiling (a
        35	/// conservative false red to triage), and a vacuously quiet counter is the
        36	/// liveness floors' business, not the trend's.

Resolution: State the direction-dependence: a zero at a smaller point steepens the fit (a conservative false red); a zero at a larger point flattens it and reads green, which is why every judged column carries a liveness declaration. Optionally pin `trend(&[(100, 5), (200, 0)]) < 0.0` in a one-line judge test. Acceptance: the docstring states the direction-dependence.
Construction: `assert!(trend(&[(100, 5), (200, 0)]) < 0.0)`; through `evaluate` with all-NA floors and `limb: Some(60)` then `Some(0)` over denominators 100 -> 200, `red` is empty.

### board-families-floors-judge-19: Only the scan floor has a committed known-dead demonstration; limb, touch, and heap floor trips are asserted in prose alone
- Where: crates/before/src/meter/board/judge.rs:65-73 (related: judge.rs:77-82, 373-388; board/tests.rs:405-484; floors.rs:441-521)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rn '_FLOOR_TRIP'` outside judge.rs: tests.rs:409 and :480 only, both SCAN_FLOOR_TRIP); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no-rationale-found (the touch and limb floors landed after the scan bypass tripwire with no trip demonstration)
- Owner-gated: no

`bypassing_walk_is_green_under_ceilings_alone_and_red_under_floors` demonstrates that a body routing its work around the meters reads green under ceilings and red on SCAN_FLOOR_TRIP; no committed test exercises LIMB_FLOOR_TRIP, TOUCH_FLOOR_TRIP, or HEAP_FLOOR_TRIP through `evaluate`, although the touch and limb floors carry the more intricate derivations. `below_floor` is currency-agnostic, so the mechanism generalizes; the gap is that the per-currency derivations feeding those floors are exercised only by hand-count tests, never end to end as a trip. Every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

        65	/// The limb column's floor-trip message.
        66	pub(super) const LIMB_FLOOR_TRIP: &str =
        67	    "limb floor: counter reads below floor: the meter is not watching this work";
        68	/// The scan column's floor-trip message.
        69	pub(super) const SCAN_FLOOR_TRIP: &str =
        70	    "scan floor: counter reads below floor: the meter is not watching this work";
        71	/// The touch column's floor-trip message.
        72	pub(super) const TOUCH_FLOOR_TRIP: &str =
        73	    "touch floor: counter reads below floor: the meter is not watching this work";

Resolution: Extend the probe pattern at tests.rs:430-457 with one `Sample` pair per remaining floored currency: `touch: Some(0)` under `touch_pair_fold(v, w)` on a dense pair -> `red == [TOUCH_FLOOR_TRIP]`; `limb: Some(0)` under `limb_stream(mandatory_limbs_stream(&hugeleaf(256)))` -> `[LIMB_FLOOR_TRIP]`; `heap: Some(0)` under `heap_materializes(n)` -> `[HEAP_FLOOR_TRIP]`. Acceptance: each of the four live `*_FLOOR_TRIP` constants is asserted by name in a committed test that feeds a zero reading against a floor the floors.rs constructors derived.
Construction: Reuse the tests.rs:430-457 `sample` closure with `touch: Some(0)` and `floors: walk_floors(n, touch_pair_fold(&v, &w))` where `v = version_of(&dense(1_000))` and `w` is `v` ticked at the seed; `evaluate` on two such samples must give `red == vec![TOUCH_FLOOR_TRIP]`. Repeat with `limb: Some(0)` and `floors.limb = limb_stream(mandatory_limbs_stream(&hugeleaf(256)))` (4 limbs per tests.rs:55), expecting LIMB_FLOOR_TRIP.

### board-families-floors-judge-20: Exponent legs can go unjudged with nothing pinning which cells may
- Where: crates/before/src/meter/board/judge.rs:120-124 (related: judge.rs:149-152, 363; render.rs:74-82; ceilings.rs:218-229; shard.rs:497-520; board/tests.rs:683-707, 1059-1062; tests/amp_board_smoke.rs:229-302)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -rn exp_judged` across src, tests, examples: judge.rs, render.rs:78, and the one negative assertion at tests.rs:1060; shard.rs:505-520 counts cells against `Coverage::Board` only; tools/benchjudge-expected.json pins only `red`); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no-rationale-found (letting an unjudged leg ride constants and floors is deliberate and stated at ceilings.rs:225-228; pinning the unjudged set was never considered)
- Owner-gated: no

An exponent leg whose denominator pair fails `MIN_EXPONENT_DENOM_GROWTH` is silently unjudged (green) and renders ` -.--`; no test, roster, or merge refusal pins the set of cells whose exponent legs are expected to be unjudged, so a family whose generator stops scaling (or a denominator wiring slip) converts every exponent ceiling in that column into a vacuous pass while the acceptance board stays green. A per-byte constant at a non-scaling size is not a scaling judgment. This is the exponent-ceiling analogue of a ceiling over a dead counter: the doctrine pairs every ceiling with a liveness guard and a committed known-bad artifact, and here the known-bad artifact (a cell whose denominator stopped doubling) reads green.

Evidence:

       120	    let spans = |points: &[(usize, u64)]| -> bool {
       121	        let first = points.first().map_or(0, |&(n, _)| n);
       122	        let last = points.last().map_or(0, |&(n, _)| n);
       123	        last as f64 >= first as f64 * MIN_EXPONENT_DENOM_GROWTH
       124	    };

       149	    Fit {
       150	        exp: Some(trend(&points)),
       151	        judged: spans(&points),
       152	    }

       363	        if s.exp_judged && s.exp.is_some_and(|e| e > *ceilings.get(c)) {

    render.rs:
        78	            Some(e) if s.exp_judged => format!("{e:5.2}"),
        79	            Some(_) => " -.--".to_string(),

Resolution: Commit a tamper-evident roster of the cells whose exponent legs are expected unjudged (today: the rank rows on the benign family, heap legs inside the flat allowance, capacity-model cells), in the style of the worst-rankings roster, and have `run_acceptance` (or the shard merge) refuse any compiled currency whose `Fit.judged` is false on a cell outside it; add the known-bad artifact beside `merge_refuses_a_silently_shrunk_grid_for_every_family`: a capture with one cell's second-sample `exp_denom` overwritten to equal the first's must be refused. Acceptance: the tampered-capture test fails against today's code (the board renders GREEN with ` -.--` on the tampered cell) and passes once the roster check lands; the smoke and acceptance boards match the committed roster exactly.
Construction: In tests/amp_board_smoke.rs, reuse the capture-and-edit loop at 229-302: split one cell line on tabs, set the second sample's `exp_denom` field (the second field of the second sample block, per `emit_sample`'s order at shard.rs:205-241) equal to the first sample's, keep the end count, and call `board::run` on the tampered capture. Today it succeeds and the rendered row shows ` -.--` on that cell's limb/scan/touch exponents with a GREEN verdict.

### board-families-floors-judge-21: The heap exponent is fitted on allowance-inclusive readings, so a quadratic heap term of allowance-scale magnitude passes acceptance
- Where: crates/before/src/meter/board/judge.rs:132-147 (related: judge.rs:19-29, 100-108, 256-259; board/tests.rs:708-753; ceilings.rs:69, 73, 78; board.rs:214-222; lib.rs's space guarantees)
- Class / severity / confidence: verification-gap / high / high
- Provenance: verified (I transcribed `trend` (37-54) and the heap branch of `fit_currency` (125-147) into python and ran the construction: points (4096, 9011), (8192, 11468), (16384, 21299), (32768, 60620) all clear `HEAP_FLAT_ALLOWANCE_BYTES = 8192` and span x8; raw four-point slope 0.914 (< 1.15, green); top window alone 1.509 (red under `evaluate`); residual `(m - 8192)` slope 2.000; constants 0.20, 0.40, 0.80, 1.60 B/B against a 16.0 ceiling; the existing `over_allowance` probe reads 6.03 under a residual fit and `straddling` keeps one cleared point. No cargo command run); executed: no (a python transcription of the Rust estimator, not the Rust code)
- Seen by: adequacy; refutation: confirmed (independent re-transcription reproduced every number); history: no-rationale-found (the exclusion filter was reasoned twice, f68802c3 and 1a5e57e5, purely against false red; fitting the residual was never considered; 9e36dd28's "still reads red" was asserted, not demonstrated against a quadratic of sub-allowance magnitude)
- Owner-gated: no

The heap exponent leg fits `trend` over the raw readings (points that merely clear `HEAP_FLAT_ALLOWANCE_BYTES`), while the constant leg judges `m2 - allowance`; the un-subtracted flat term deflates the log-log slope, so a peak heap of `A + n^2/20480` bytes over the acceptance ladder 4096..32768 reads a four-point trend of 0.914 and constants of 0.2-1.6 B/B, green in both windows of `evaluate_acceptance`, even though the top window's own two-point fit reads 1.51 and the residual fit reads exactly 2.0. The verdict of record is weaker than the bare debugging view on this artifact, and the docstring's promise that "a genuine super-linearity bends every point and still reads red" is falsified by construction. The crate docs make space claims hard guarantees, and the board is the one instrument pinning heap order across the op x family product; the worst artifact that passes is a Theta(n^2) heap term whose magnitude at the board's KiB-scale sizes sits within a few multiples of the 8 KiB allowance (Principles 2 and 6).

Evidence:

       132	        let cleared: Vec<(usize, u64)> = points
       133	            .iter()
       134	            .copied()
       135	            .filter(|&(_, m)| m > HEAP_FLAT_ALLOWANCE_BYTES as u64)
       136	            .collect();
       137	        return if cleared.len() >= 2 && spans(&cleared) {
       138	            Fit {
       139	                exp: Some(trend(&cleared)),
       140	                judged: true,
       141	            }

       256	        let per_unit = match c {
       257	            Currency::Heap => {
       258	                m2.saturating_sub(HEAP_FLAT_ALLOWANCE_BYTES as u64) as f64 / s2.denom_bytes as f64
       259	            }

        25	/// single generator lump at one point cannot define the estimate, while a
        26	/// genuine super-linearity bends every point and still reads red. Densifying

Resolution: Fit the heap trend over the same quantity the constant leg judges, `m.saturating_sub(HEAP_FLAT_ALLOWANCE_BYTES as u64).max(1)` on the cleared points, keeping the `cleared.len() >= 2 && spans` guards, and add a materiality guard so a residual of a few bytes above the allowance cannot manufacture an exponent (for example, each cleared residual at least a stated fraction of the allowance, derived and documented beside `HEAP_FLAT_ALLOWANCE_BYTES`). Commit the construction below as a tripwire beside `exponent_guards_skip_noise_and_keep_real_amplifiers_red`. Under the residual fit the existing `over_allowance` probe stays red and `straddling` stays unjudged, so the committed pins survive. Acceptance: a committed judge test builds four `Sample`s with `heap: Some(8192 + n*n/20480)` at n = 4096, 8192, 16384, 32768 (other columns NA or zero) and asserts `evaluate_acceptance` returns "heap exponent" in `red` for both windows; the existing judge tests stay green; `just amp-board-acceptance` stays green on the real board (any new red is a genuine finding to triage).
Construction: In board/tests.rs, reuse the all-NA `Sample` helper shape from `acceptance_trend_absorbs_lumps_and_keeps_amplifiers_red` (776-801) with `readings.heap = Some((HEAP_FLAT_ALLOWANCE_BYTES + n*n/20480) as u64)`, `limb: None`, and `exp_denom_bytes = denom_bytes = n`; call `evaluate_acceptance("heap_probe", "quadratic-under-allowance", (sample(4096), sample(8192)), (sample(16384), sample(32768)))` and assert both windows' `red` is empty today (the demonstration) versus containing "heap exponent" after the fix. For contrast, `evaluate("heap_probe", "top-window", sample(16384), sample(32768))` reads "heap exponent" red today: the debugging view catches what the verdict of record does not.

### board-families-floors-judge-22: Constants are judged at each window's larger size only; board.rs says "per size across the ladder"
- Where: crates/before/src/meter/board/judge.rs:256-263 (related: judge.rs:315-337, 381-387, 414-418; board.rs:69-71, 222-224)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read: `per_unit` is computed from `m2`/`s2` alone and the declared heap/limb ceilings apply to that one value at 342 and 352, while floors (381-387) and the capacity band (325-336) check both samples); executed: no
- Seen by: adequacy; refutation: confirmed; history: deliberate-and-holds (board.rs:69-71 states "a per-denominator-byte constant at the larger scale"; 9e36dd28's "per size" meant per window; the reason for excluding the smaller size is stated nowhere)
- Owner-gated: no for the doc fix; yes if both sizes are to be judged

`judge_window` computes `per_unit` from `s2` alone, so across the acceptance ladder the constant legs and the declared heap/limb ceilings are judged at two of the four sizes; the board module doc states that "constants, declared-model bands, and liveness floors stay judged per size across the ladder", which is true for bands and floors and false for constants. Prose states what IS; judging at the larger size only may be deliberate (fixed overhead inflates per-byte constants at the smaller size), in which case the doc should say so.

Evidence:

       256	        let per_unit = match c {
       257	            Currency::Heap => {
       258	                m2.saturating_sub(HEAP_FLAT_ALLOWANCE_BYTES as u64) as f64 / s2.denom_bytes as f64
       259	            }
       260	            Currency::Segments => m2 as f64,
       261	            Currency::Limb => m2 as f64 / s2.limb_denom as f64,
       262	            Currency::Scan | Currency::Touch => m2 as f64 / s2.denom_bytes as f64,
       263	        };

    board.rs:
       222	//! super-linearity bends every point and still reads red; constants,
       223	//! declared-model bands, and liveness floors stay judged per size across the
       224	//! ladder. A bare single-scale run fits its own window's two points and

Resolution: Correct board.rs:222-224 to "constants at each window's larger size; bands and floors at every size" and state the reason the smaller size is excluded; or (owner's call) judge `per_unit` at both samples of each window. Acceptance: board.rs and judge.rs agree on which sizes carry the constant legs.
Construction: Through `evaluate`, `sample(n, limb = 10*128*n)` (over the 128/B ceiling) paired with `sample(2n, limb = 2n)` reads no "limb constant" red today, since only `s2` is judged.

### board-families-floors-judge-23: `judge_window`'s ceiling resolution is a match split across four statements; `Score` duplicates `Fit`; the span test is written twice
- Where: crates/before/src/meter/board/judge.rs:280-362 (related: judge.rs:19-29, 56-73, 84-93, 120-124, 172-174, 200-206, 316-336, 373-380; board.rs:212-228)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: no-rationale-found for the code shape; the policy paragraph on `trend` and its "owner-ratified" tag are deliberate (9e36dd28, kept through d2a9d04e's dated-notes excision), so that part is not a finding
- Owner-gated: no

`let (mut ceiling, exp_label, const_label) = match c { .. }` (280-310) is followed by three `if c == Currency::X { .. ceiling = .. }` overrides (341-345, 351-355, 357-362) that belong in the arms; `Score { exp, exp_judged, per_unit }` restates `Fit { exp, judged }` field by field instead of embedding it; the denominator-span test `last as f64 >= first as f64 * MIN_EXPONENT_DENOM_GROWTH` is written as a closure at 120-124 and inline at 173-174; `banded(s, X).is_some_and(|over| over)` is `== Some(true)`, with `banded`'s direction chosen by `edge > 1.0` rather than two named closures; the five floor-trip constants plus the `match c` that selects them could be one `fn floor_trip(c: Currency) -> &'static str`. This is the verdict kernel; the bar is "obviously, reviewably correct", and resolving each currency's constant ceiling in one arm lets a reader see the whole rule for a currency in one place.

Evidence:

       280	        let (mut ceiling, exp_label, const_label) = match c {
       281	            Currency::Heap => (

       341	        if c == Currency::Heap {
       342	            if let Some(declared) = s2.declared_heap {
       343	                ceiling = declared;
       344	            }
       345	        }

       351	        if c == Currency::Limb {
       352	            if let Some((_, per_radix_unit)) = s2.declared_limb {
       353	                ceiling = per_radix_unit;
       354	            }
       355	        }

Resolution: Fold the overrides into the match arms (the capacity-model `continue` can precede the match); `Score { fit: Fit, per_unit: Option<f64> }`; hoist `fn denominators_span(first: usize, last: usize) -> bool` to module level; `== Some(true)` and an `over_ceiling`/`under_floor` pair in place of `banded`; `fn floor_trip(c: Currency) -> &'static str` (tests.rs:409 imports `SCAN_FLOOR_TRIP`; the test can call the fn). Trim `trend`'s doc to the estimator's mechanics with a pointer to board.rs's exponent-policy section only if the owner wants the policy stated once. Acceptance: `judge_window` reads each currency's ceiling from a single match arm; one span helper; `acceptance_trend_absorbs_lumps_and_keeps_amplifiers_red`, `exponent_guards_skip_noise_and_keep_real_amplifiers_red`, `declared_capacity_model_bands_the_projection_peak`, and the smoke board pass unchanged.

### board-families-floors-judge-24: operand.rs writes one preorder walk four times and the zigzag decode twice, duplicating `skyline::signed::unzigzag_base`
- Where: crates/before/src/meter/board/operand.rs:29-37 (related: operand.rs:62-70, 95-103, 108-119, 163-172, 177-188; board/defect.rs:67-86; version/skyline/signed.rs:174-185; testing/bridge.rs:158-170; shape.rs:80-98; version.rs:773)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`grep -rn 'pending += 2'`: operand.rs:34, 67, 100, 169 and defect.rs:78; `grep -rn '(code + 1u32) >> 1u32'`: operand.rs:110, 179, bridge.rs:164, and the one production home signed.rs:181, which is `pub(super)`); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (the local decodes date from 6814d77b, 2026-07-23, when a crate zigzag decoder already existed; `unzigzag_base` arrived without operand.rs being routed through it; `Version::shape()` postdates every walk)
- Owner-gated: no

The `pending`/`internal`/`decode_int` preorder loop over stored leaf codes is copied verbatim in `stored_nonzero_deltas`, `mandatory_limbs_stream`, `value_content_bytes`, and `stored_bases` pass 1, with a fifth copy in defect.rs; the odd-is-negative delta decode is written out twice here and a third time in testing/bridge.rs, while `skyline::signed::unzigzag_base` is the crate's one decoder. A reviewer checking that the floors and denominators are derived from the right quantity must verify four loops and two decodes instead of one of each; a change to the leaf flag polarity or the sign convention touches every copy with no compile-time tie. Legibility matters almost as much as correctness; one home per mechanism (Principle 3).

Evidence:

        29	    while pending > 0 {
        30	        pending -= 1;
        31	        let internal = !bits.bit(pos); // skyline flag: 0 internal, 1 leaf
        32	        pos += 1;
        33	        if internal {
        34	            pending += 2;
        35	            continue;
        36	        }
        37	        let (payload, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");

       108	                let odd = code.bit(0);
       109	                let magnitude = if odd {
       110	                    (code + 1u32) >> 1u32
       111	                } else {
       112	                    code >> 1u32
       113	                };

    signed.rs:
       179	pub(super) fn unzigzag_base(code: Base) -> (Sign, Base) {
       180	    if code.bit(0) {
       181	        (Sign::Negative, (code + 1u32) >> 1u32)
       182	    } else {
       183	        (Sign::Positive, code >> 1u32)
       184	    }
       185	}

Resolution: One private `fn stored_leaf_codes(v: &Version) -> impl Iterator<Item = Base>` (or a `Node::Internal | Node::Leaf(Base)` iterator when the topology is needed) in operand.rs, with `stored_nonzero_deltas`, `mandatory_limbs_stream`, and `stored_bases` pass 1 as folds over it; one `fn stored_heights(v)` applying `unzigzag_base` (widened to `pub(crate)`) so `value_content_bytes` is a sum over it and `stored_bases` collects it; route defect.rs's `last_leaf_flag_pos` and testing/bridge.rs (another partition) through the same pieces. For the count walks the public `Version::shape()` iterator is an alternative source (`v.shape().skip(1).filter(|p| p.rise.is_some()).count()` is `stored_nonzero_deltas`); the raw code width `mandatory_limbs_stream` needs is the one quantity that argues for keeping a code-level iterator. Acceptance: `grep -c 'pending += 2' operand.rs` is 1 (or 0 with a skyline-side iterator); `grep -c '>> 1u32' operand.rs` is 0; `limb_floor_derivations_split_on_plateaus_and_coincide_on_a_leaf`, `radix_units_match_hand_counts`, `mandatory_limbs_match_hand_counts`, and the smoke board pass unchanged.

### board-families-floors-judge-25: The touch floor's sole basis and the flat-denominator axis have no differential pin against the public shape iterator
- Where: crates/before/src/meter/board/operand.rs:89-125 (related: operand.rs:23-45; shape.rs:80-98, 161-178; version.rs:773; board/tests.rs:52-102, 379-383, 556-620)
- Class / severity / confidence: test-quality / low / medium
- Provenance: verified (`grep -n 'stored_nonzero_deltas\|value_content_bytes' board/tests.rs`: an import at 27 and two growth-ratio uses at 562, 618; the hand-count tests cover `mandatory_limbs_*` and `radix_units_*` only; shape.rs:89-95 read for the `rise: None` semantics, assessed not executed); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no-rationale-found (`Version::shape()` landed 2026-08-19, after every walk; nothing since has considered the pin)
- Owner-gated: no

`stored_nonzero_deltas` is the sole basis of every touch floor and `value_content_bytes` is the flat-denominator exponent axis; neither has a hand-count pin nor a check against the production reader, while `Version::shape()` yields exactly the stored rises (one `Plateau` per stored leaf, `rise: None` iff the stored delta is zero, the first rise the absolute root height), giving the identity `stored_nonzero_deltas(v) == v.shape().skip(1).filter(|p| p.rise.is_some()).count()` for free. Any quantity computable two ways gets a committed test comparing them; a flag-polarity or zigzag drift in one walk would today move every touch floor silently.

Evidence:

        89	pub(super) fn value_content_bytes(v: &Version) -> usize {
        90	    let bits = v.as_bits();
        91	    let mut pos = 0u64;
        92	    let mut pending = 1usize;

    shape.rs:
        89	    /// The height change entering this plateau; `None` continues level.
        90	    ///
        91	    /// The first plateau's rise is its absolute height (`None` if the
        92	    /// shape starts at 0): the walk begins at height 0 on the interval's
        93	    /// left edge. `None` occurs mid-stream too: two equal-height
        94	    /// plateaus separated by a subtree boundary are a real shape.

Resolution: A proptest over the crate's version generators (and a sweep over `study_family_versions`) asserting `stored_nonzero_deltas(v) == v.shape().skip(1).filter(|p| p.rise.is_some()).count()` and that `value_content_bytes(v)` equals the byte-rounded sum over `shape()` of `bits(height).max(1)` with heights accumulated from the rises; optionally implement `stored_nonzero_deltas` through `shape()` and dissolve one decoder (finding 24). Acceptance: the committed differential test passes; flipping the `!bits.bit(pos)` polarity or the odd/even zigzag arm in either walk fails it.
Construction: proptest over `crate::testing::generators`' `Version` strategy: compute both counts and assert equality; the identity follows from shape.rs:89-95.

### board-families-floors-judge-26: `radix_units_party` carries a branch for a value `Party` cannot hold
- Where: crates/before/src/meter/board/operand.rs:261-263 (related: party.rs:1-6, 245-249, 634-654, 791-799; board/tests.rs:98-99)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (party.rs:643-654: the anonymous id is "never a publicly constructible value"; `Party::decode` rejects an exhausted stream and `finish_id` rejects the empty id; `decode_party` is the only producer of the operand); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no-rationale-found (dead from its introduction in 6814d77b, when decode already rejected the empty id)
- Owner-gated: no

The empty-bits branch documents and handles the anonymous id, a `pub(crate)`-transient value that is never a `Party`; every operand reaching this walk is a decoded board party, and the seed takes the loop path and renders "1" as tests.rs:98-99 pins. Never document what the types prevent; a comment must be true of a reachable state.

Evidence:

       261	    if bits.is_empty() {
       262	        return 1; // the empty id renders one `0` token
       263	    }

    party.rs:
       646	    /// Internal and transient only (i.e. for use in `mem::swap`) and *never* a
       647	    /// publicly constructible value (a `Party` is a nonzero share).

Resolution: Delete the branch. Acceptance: `radix_units_party` has no empty-stream branch; `radix_units_match_hand_counts` still passes.

### board-families-floors-judge-27: `version_output_bytes` and the sibling ops.rs sites omit the marker bit and under-report the packed size by one byte on byte-aligned streams
- Where: crates/before/src/meter/board/operand.rs:292-297 (related: ops.rs:1313-1326, 1491-1492, 1504-1511, 1794-1795, 1807-1814; version.rs:1129-1131, 1174-1180; party.rs:576-578; clock.rs:852-858; codec/bits.rs:70-81)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (the size law at version.rs:1129-1131 and party.rs:576-578; the marker-then-zero-pad rule at bits.rs:74-77; `Clock::encoded_bits` at clock.rs:856-857 already rounds the party component to whole bytes, so a clock is at most one byte short; ops.rs:1316 floor-divides); executed: no
- Seen by: instrument-correctness; refutation: reframed (a clock's `div_ceil` form is at most one byte short, not two); history: deliberate-but-expired (correct until d800957e's marker bit on 2026-08-06 changed the size law without touching operand.rs or ops.rs)
- Owner-gated: no

The function documents itself as the packed byte size of a measured version but computes `encoded_bits().div_ceil(8)`; the crate's own contract is `encode().len() == (encoded_bits() + 1).div_ceil(8)`, so the result is one byte short whenever `encoded_bits() % 8 == 0`. The same stale law sits at ops.rs:1491, 1510, 1794, 1813 (the party and clock parse rows' I/O denominators and heap floors), and ops.rs:1316 floor-divides (`/ 8`): on a child half with fewer than 8 live bits `child_bytes` is 0 and the fork row's heap floor silently becomes NA_HEAP_IN_PLACE (1319-1320), and on every other child the floor is up to seven bits lenient. Correct for all inputs: the function's doc is false on one stream in eight; the direction is conservative on the constants and lenient on the heap floors, so it masks nothing today, but it is a plain harness bug with a one-token fix that disagrees with every input-side `encode().len()` on the same board.

Evidence:

       292	/// The packed byte size of a version produced by a measured body.
       293	pub(super) fn version_output_bytes(v: &Version) -> usize {
       294	    // The measured value's stored buffer is allocated on this host, so its
       295	    // byte count fits `usize`.
       296	    usize::try_from(v.encoded_bits().div_ceil(8)).expect("an allocated buffer's byte count")
       297	}

    version.rs:
      1129	    /// The exact length in bits of [`encode`](Self::encode) before its
      1130	    /// padding — the marker bit and zero-pad to the byte boundary, so
      1131	    /// `encode().len()` is `(encoded_bits() + 1).div_ceil(8)`.

    ops.rs:
      1316	                    probe.fork().encoded_bits() / 8

Resolution: `v.as_bytes().len()` (O(1), public) and delete the `try_from` dance; at the ops.rs sites use `as_bytes().len()` / `encode().len()` for party and clock (1316 included, which also removes the NA collapse on tiny children). Acceptance: a committed test sweeping `study_family_versions(DEFAULT_SCALE)` plus one version with `encoded_bits() % 8 == 0` asserts `version_output_bytes(&v) == v.encode().len()` and passes.
Construction: Tick a fresh `Version` with `Party::seed()` until `v.encoded_bits() % 8 == 0` (or pick one such stream from the family corpus), then `assert_eq!(version_output_bytes(&v), v.encode().len())`: the left side is one less today.

## Positives

- Floors are derived from the operands at prepare, never from readings, and fork on the verdict the cell will produce: `comparison_floors`, `membership_floors`, and `masked_cmp_floors` (floors.rs:668-689, 718-739, 751-768) realize "one universal premise, no per-family carve-outs" in code rather than prose, and `membership_floors` gets the one-directional derivation right (refusal is the full-certification direction; admission needs one witness), agreeing with `since()`'s documented semantics.
- `touch_pair_fold` (floors.rs:458-492) is a model liveness floor: per boundary not per element, a max rather than a sum with the reason stated, zero deltas excluded as legitimate less-work inputs with the mechanism (an accumulator add of zero is a no-op), and the family a naive count would have banned (tooth-tail) named at the derivation.
- `touch_fold_first_merges`'s arrival-adjacent-pairs premise is exact against the implementation: `Version::join_all` seeds the receiver first, the binary-counter fold merges arrival-adjacent inputs at weight 0, and the dedup collapses on pointer identity only, so `chunks(2)` over the prepared list is the true first level.
- `measure` (measure.rs:84-95) resets every counter immediately before the body and reads it immediately after with no metered work between, keeps the result alive until the heap peak is read, settles the I/O denominator from the actual result (`(spec.output_bytes)(result.as_ref())`), and asserts text-output honesty at the measurement site; `HeapMeter` as caller-supplied fn pointers is the right shape for per-binary allocator state the library cannot own, and its doc says exactly why.
- `judge::trend` is a genuine log-log least-squares slope over every measured point with the clamp's direction documented, and the committed adequacy demonstrations go through `evaluate` itself: the meter-bypassing walk (green under ceilings, red under the scan floor), the chunked schoolbook converter (under kappa, red on the n_io exponent), the lump ladder versus the quadratic ladder through `evaluate_acceptance`, the fold model's fat constant and quadratic, and the capacity band red on both the regressed and the improved side.
- `amp_board` carries `required-features = ["limb-meter", "scan-meter"]` (Cargo.toml:115-117), so a dark counter column cannot reach the verdict of record; the `None`-reading exemption in `judge_window` is unreachable there.
- `ByCurrency`'s field-per-currency totality with exhaustive `each()` destructuring means a new meter is a compile error at every declaration, judgment, and render site; judge.rs's loops run over the axis itself, not a hand list.
- The mod-32 remainder-alignment derivations on the base constants (family.rs:127-139, 148-157, 205-215, 224-230, 241-244, 277-283, 294-296) are careful measurement design: they explain why a legitimate amortized-O(1) constant would otherwise read as growth across the ladder and choose the base to hold it fixed, as mechanism rather than observed number.
- `MIN_SIZE_PARAM`'s doc (family.rs:380-390) says precisely what the floor preserves (positivity) and what it does not (relations between knobs) and names the two repair conventions; `overlap_mounted_pair` (1128-1178) states plainly that its outputs are semantically void by design and why the cost claim still needs them.
- The stream-derived versus tree-derived limb floor split (operand.rs:47-56, 127-137) is argued from the coding (a plateau stores its width once) and pinned by a test that constructs the separating shape.
- The exposure disclosure at floors.rs:99-112 states the unwatched cells and bounds the exposure by mechanism instead of leaving it silent; finding 11 asks for it to be pinned, but the instinct to disclose is exactly right.
- The scan meter records builder writes as well as reads (codec/build.rs), so the tick rows' 8-bits-per-byte floor is in fact cleared by the shipped walk with wide margin; finding 17 is about the derivation's wording, not the floor's soundness.

## Open questions for Finch

1. The flat-denominator content axis (`Sample.exp_denom_bytes`, `FamilyData.content_bytes`, `value_content_bytes`, the `content` argument to `measure`, two tripwire tests, and cell.rs's derivation) exists for one family, comb-scatter, because its packed bytes grow ~1.2x per doubling at fixed tooth magnitude. Could the shape be rescaled so packed bytes track the knob (as Cliff scales k with n) while keeping the output-domination ratio the projection rows need, retiring the second denominator? Recommendation: keep it for now, but steelman the dissolution in cell.rs's derivation so the next reader sees it was weighed. Related: the content denominator is applied to version-only rows' exponents on comb-scatter too (family.rs:548 adds `p.len()`), harmless while both halves double per level, worth a sentence.
2. Kernel-pinned ("deterministic-liveness") floors pin the current metered representation and trip on an improvement; the envelope suite (tests/meter.rs) is the instrument whose pins are explicitly implementation-pinned. Should those floors live there, leaving the board's floors purely contract-derived, or is the board's rendered legend the disclosure surface you want? Recommendation: keep them on the board (the legend is where a trip is read) and define the class per finding 8.
3. The registry doc (registry.rs:569-583) lists the base-size constant and its derivation doc among the things "found by luck" when a family is added. Should `Coverage::Board` carry the base size (or a `BoardBase` struct) so the registry row is the single declaration a new column requires? Recommendation: yes, as part of finding 6's `BoardFamily` refactor, moving family.rs's thirty constants beside their coverage answers.
4. The disjoint-mount and overlap-mount adapters assert the property they construct on every bundle build (family.rs:1121-1124, 1150-1154, 1173-1176). The constructions are deterministic pure functions of the shape bytes and the smoke test builds every board family. Recommendation: keep them; they run once per family per scale at prepare, outside measurement, each message is a one-line proof, and the cost is negligible; convert to unit tests only if build time becomes a budget item.
5. Which board families present `version2 < version`, so the membership row's refusing (full-certification) direction is priced at all? For every non-pair shape `version2` is the seed tick of `version`, which `since(v).contains(w)` admits with one witness; only the pair shapes can exercise refusal. Recommendation: a one-line test asserting at least one board family's membership cell carries the full-examination floor, so the arm is not dead on the board.
6. The hugeleaf ladder (32k..256k value bits) and the envelope suite's hugeleaf pin (125k bits) both sit above the backend parser's algorithm switch at 16k-20k bits that HUGELEAF_BASE_MAGNITUDE_BITS documents. Is the sub-switch transient's magnitude pinned anywhere at a fixed size, so a regression from ~x4 to ~x40 there would be seen? Values under 16k bits are the common case. Recommendation: one fixed-size envelope pin below the switch.
7. Under a residual-based heap fit (finding 21), what materiality guard do you want: a fixed fraction of `HEAP_FLAT_ALLOWANCE_BYTES`, or a per-denominator-byte minimum tied to `MAX_HEAP_BYTES_PER_INPUT_BYTE`? The choice sets how small a super-linear heap term the board can resolve at KiB inputs. Recommendation: a fraction of the allowance (say a quarter), stated beside it.
8. Is the `meter` feature's counter-reader surface (`meter::limb_ops`, `scan_bits`, the resets) covered by the public-API stability rule? That decides whether Option-returning readers (finding 16(c)) are an internal refactor or an owner-gated API change.
9. Vocabulary (finding 15): 167 "honest", 66 "mint", 22 "today", and 374 em-dash `//` comments across crates/before/src are the dialect of the period, predating the 2026-08-19 writing-style rules. Recommendation: one crate-wide ruling and a single sweep commit rather than per-partition edits, with "genre" allowed to stand as established in-repo vocabulary (.cargo/mutants.toml uses it).
10. Weight comb and freeze parade: is the intent that their flatness bands eventually get a committed `_reads_superlinear` kernel, or is the band's adequacy deliberately resting on a local probe build? family.rs currently describes the latter as "committed" (finding 1).
11. Relayed for the board-frame/tests reviewer (outside this partition): board/tests.rs:520 writes `a_bytes.len() / 64` where `OVERLAP_FOLD_INPUT_DIVISOR` (family.rs:170) is the constant of record, and tests.rs:505 cites "the design doc's §3 entry" from code.

## Dropped

- [0] operand.rs walk/zigzag duplication (scaffolding): duplicate of board-families-floors-judge-24 (the structure-prose statement is the more complete one).
- [1] Base-size docs quote readings (scaffolding): merged into board-families-floors-judge-1 with [23]; owner-gating removed because the ruling exists.
- [4] Default-dialect vocabulary (scaffolding): the "today" half merged into board-families-floors-judge-8; the rest into board-families-floors-judge-15 (measure.rs's "3 honest" was a miscount; it has none).
- [5] floors.rs helper re-inlining (scaffolding): merged into board-families-floors-judge-13 with [25], [21], [44].
- [6] Module doc's universal rule contradicted (scaffolding): subsumed by board-families-floors-judge-8.
- [7] Hand-maintained "Four cells" and 10 µs (scaffolding): merged into board-families-floors-judge-11 with [27] and the count half of [43].
- [11] Sample field copying and cfg shims (scaffolding): merged into board-families-floors-judge-16 with [36].
- [12] judge.rs policy prose duplicated, spans twice, Fit/Score (scaffolding): the mechanical parts merged into board-families-floors-judge-23; the policy paragraph on `trend` and its "owner-ratified" tag are deliberate per 9e36dd28 and d2a9d04e (history), so the prose-removal half is dropped.
- [13] family.rs qualified path and double decode (scaffolding): subsumed by board-families-floors-judge-3.
- [19] Measured 2-5x reading in tick_walk_floors prose (adequacy): refuted; 500d4d09 explicitly keeps "the tick walk's 2-5x floor margin" among order-of-magnitude calibrations; converted into board-families-floors-judge-17's note that the ceilings.rs convention header should name the kept-calibration class.
- [20] SCAN_TOUCH_FLOOR_BITS half its derivation (adequacy): merged into board-families-floors-judge-9.
- [21] Zero-minimum Floor constructible (adequacy): merged into board-families-floors-judge-13 (the `floor_or_na` constructor makes the state unrepresentable).
- [28] NA_SCAN_SEED_PARTY says the seed is empty (structure-prose): merged into board-families-floors-judge-10(b).
- [29] "mint" seven times (structure-prose): merged into board-families-floors-judge-15.
- [30] Vocabulary sweep (structure-prose): merged into board-families-floors-judge-15.
- [31] Derivations restated (structure-prose): the floors.rs/operand.rs half is board-families-floors-judge-12; the judge.rs half is dropped for the same reason as [12].
- [32] judge_window match split (structure-prose): merged into board-families-floors-judge-23.
- [35] Constant placement convention (structure-prose): folded into board-families-floors-judge-13's resolution as a layout step while consolidating.
- [39] Tick-walk scan floor derivation (instrument-correctness): reframed by the refutation pass and merged with [10] into board-families-floors-judge-17.
- [40] NA_SCAN_SEED_PARTY and the empty branch (instrument-correctness): the NA half merged into board-families-floors-judge-10(b); the empty branch is board-families-floors-judge-26.
- [41] Decode rows' touch NA (instrument-correctness): duplicate of board-families-floors-judge-10(a).
- [43] "Four cells" and `w ≤ v, strictly` (instrument-correctness): the count merged into board-families-floors-judge-11; the inequality slip into board-families-floors-judge-9(c).
- [44] Count arithmetic idioms (instrument-correctness): merged into board-families-floors-judge-13.
- [45] &Option<Ordering> and qualified path (instrument-correctness): subsumed by board-families-floors-judge-3.
- Structure-prose open question "does Party::fork read the seed's tag through a metered primitive": answered yes by reading (idbits.rs:132-139 via split.rs:22), so it became evidence for board-families-floors-judge-10(b) rather than an open question.
