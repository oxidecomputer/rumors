# Simplification, modularity, vestigial code, and dissolvable scaffolding

This document collects every finding of the holistic review of `before` and `suanpan` at commit `9e5784fb4dce977cfbdfd1619886d1482b5ce764` whose primary class is simplification, modularity, vestigial, idiom, or scaffolding: 376 entries from 34 partition reports and 11 sweeps. It answers one question: what in these crates could be simpler, and what verification machinery could be dissolved or consolidated without losing a named failure class. Ids are `<partition or sweep key>-<n>`; the full record of every entry, with its lens candidates, refutation, and history passes, lives in `evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md` beside this file. The severity scale, as the finalizers applied it: *medium* names a structural cost the next change to the site will pay, or an instrument that passes for reasons unrelated to what it claims to measure; *low* names a bounded legibility or maintenance cost with a fixed-sign fix; *nit* names a mechanical edit. No entry in these classes reached *high*. Provenance: *demonstrated* means a constructed test ran; *executed* means a run settled the claim; *verified* means the claim was mechanically checked or re-derived (a grep, a read-only git query, a hand trace against the cited lines); *assessed* means the finding rests on reading. Every entry here is verified (305) or assessed (58), with three entries carrying a verified-by-trace variant; the witness pass constructed tests for findings of other classes only, so no entry in this document carries *demonstrated* provenance.

Because the five classes together exceed 120 entries, medium and low entries appear in the finalizers' full template (anchor, class and severity and confidence, provenance, the lenses that saw it, owner-gating, the argument, verbatim evidence, resolution with acceptance, and a construction where one exists), and nits appear per module in a compact table that points at the evidence file for the full record. Where I disagree with an entry or where two entries describe one site, a *Synthesis note* follows the entry; no anchor, evidence, verdict, severity, or provenance was changed. `before`'s public API is declared stable, so every entry that touches it is marked owner-gated and is a suggestion to the owner, never an assumed change; the same mark is used for entries that reopen a recorded ruling. Findings of other classes that bear on an entry are cross-referenced by id with their class in parentheses and are never reproduced here.

A note on anchors: the working tree at the time of writing sits at `7440d1a3`, two commits past `9e5784fb`; both commits touch only `.agent-notes/`, so every in-scope file is byte-identical to the briefed commit and the cited line numbers hold (verified: `git diff --stat 9e5784fb..HEAD` over `crates/before`, `crates/before-fuelscape`, `crates/suanpan`, `crates/surface-scan`, the justfile, `tools`, `.cargo`, and `.github` is empty). I spot-checked seventeen anchors of the entries featured below against the tree and each held; the remaining anchors are the finalizers', unchanged.

## Highest-value items

1. Dissolve the segments currency. Its only writer is `recurse::grow`, which is `#[cfg(test)]`, so every board, envelope, and bench reading of it is a compiled-in zero presented as a measurement, and `LADDER_TOP_SCALE`'s ×4 rests on a segment onset the board cannot observe; eight findings across four documents recommend dissolution, and `deep_tree_stack_safety` already holds the property (board-frame-1, meter-core-11).
2. Unify the four envelope harnesses in `tests/meter.rs` into one struct and one `metered`; the campaign note already dockets it, and the four-way split is where the door/kernel twin rows, the ad hoc densify read, and the five preamble copies came from (envelopes-a-4, envelopes-b-8, envelopes-a-15, envelopes-b-19).
3. Retire the line-scan extractor now that surfacecheck catches strictly more, and give the workspace one source scanner: five hand-rolled walkers with three fn-declaration rules live beside the `surface-scan` crate that was created to have one (surface-roster-9, tests-other-3, suite-economics-3, surface-roster-10).
4. Build `PackedBuilder` on `BitsBuf`, measured at the parent, and put the ten `BitsBuf`-as-stack sites on `BitStack`; the byte substrate exists twice under two invariant sets, and the stack the deep walks are documented to use is bypassed by ten of them (codec-bits-12, codec-base-text-tree-12, party-19, skyline-coding-32).
5. Consolidate the watermark kernel's copies: four undercut tails (one hand-inlined where the recorded polarity bug lived), a latent-ladder decision written three times, a duplicated first-arming preamble, and a certificate ladder whose mirror-image guard costs a line-pinned mutant exclusion re-pinned four times in eight days (skyline-watermark-18, skyline-watermark-19, skyline-watermark-12, skyline-watermark-14).
6. Collapse the Batch-era `_view` join and meet doors, then the three copies of the fold dispatch above `fold.rs` and the twice-encoded span operator table that the duality justifies (version-core-11, version-core-8, span-causally-9, span-causally-12).
7. Replace the oracle's two verbatim copies of production's balanced fold with a sequential reference compared on verdict, accumulator, and region union, and give `fold.rs` its own retention-arm witness, so the reference stays a paper transcription and the fold's discipline has one pin (oracle-laws-2, oracle-laws-26).
8. Make `CheckedCursor` the one strict skyline parser, and give the pair-difference seeding and the arity-N advance law the single home `overlay.rs` already claims for them (skyline-coding-33, skyline-sweep-place-masked-15, skyline-sweep-place-masked-7).
9. Retire the instruments whose justifying constraint expired: `benches/amplify.rs`, `emit_probe.rs` with the `bitvec` dev-dependency, `spanbands`, the cliff-fan family, and the Tier 2 compactness prose, keeping `tier2_size` as the independent sizer the length-agreement pins rest on (benches-examples-1, benches-examples-21, deps-7, fuelscape-pipeline-32, fuelscape-render-21, meter-registry-tier2-12, prose-hygiene-2).
10. Let the compiler hold the rosters now held by hand: the bench rider list, `designed()`'s and `FamilyData::build`'s envelope-only arms, and the six freeze-regime widths copied from `FREEZE_ALLOWANCE_DIGITS` with no binding test (board-frame-25, board-ops-render-2, board-families-floors-judge-6, meter-core-7).
11. In the fuzz-fit harness, delete the write-only `Ty`/`slots` liveness model, the second call path through `call_i64`, and the hand-mirrored ABI constants and digest (fuzzfit-strategies-19, fuzzfit-strategies-2, fuzzfit-bands-15, fuzzfit-strategies-14).
12. Settle whether the bench judge's leg has a failure class of its own: the validation index's bench-judge row and its fuzz-fit row both claim work that costs instructions, and the committed tripwire is a machine-word quadratic that wasmtime fuel also reads red; either commit a time-only tripwire (RED under the judge, GREEN under fuzz-fit's bands) and restate the class in the index, or retire the judge, its roster, `tests/bench_judge_roster.rs`, `tripwire.rs`, and the sidecar's `Ceiling` machinery after fuzz-fit demonstrably reads the tripwire's shape above band (tools-5; owner-gated). The empty acceptance buffers and dated rulings that this item displaces (testing-diff-gen-31, surface-roster-18, gate-legs-9, meter-adequacy-8, surface-roster-17, meter-registry-tier2-9, meter-adequacy-12) remain the dissolvable-scaffolding section's second kind and the README's decision on acceptance buffers.

## Crate-wide patterns

The 376 entries sort into eight structural themes. Each theme names the entries that instantiate it; the entries themselves carry the anchors.

**Machinery that outlived the constraint that justified it.** Four datable events left most of the vestigial findings behind. The flag day on which the skyline coding became the stored form (faf3cd0a, 2026-07-25) stranded the Tier 2 compactness prose, the cliff-fan family's threat model, and `held_at`'s runtime gate (prose-hygiene-2, meter-registry-tier2-12, skyline-coding-11). The removal of `Batch` (58a37d80d) stranded the `_view` doors, the three-strategy operator macro, and `balanced_fold`'s `view` parameter (version-core-11). surfacecheck's landing (9aa8c9ac) stranded the line-scan extractor (surface-roster-9). Gating `grow` and `descend!` under `cfg(test)` (05bd2b16d) stranded the segments currency (board-frame-1, meter-core-11). The same pattern at smaller scale: `benches/amplify.rs` predates `board.rs` by one commit (benches-examples-1); `emit_probe.rs` and `bitvec` priced a decision that landed (benches-examples-21, deps-7); the `display_growth` A/B arm and its check-cfg roster serve an experiment with no recorded verdict (skyline-coding-28, deps-10); `spanbands` recorded no result (fuelscape-pipeline-32, fuelscape-render-21); the `Ty`/`slots` model was never read (fuzzfit-strategies-19). In each case the entry names the instrument already in the tree that meets the retirement bar.

**One mechanism, several hand copies.** The largest theme by count, and the copies that cost the most are those whose agreement is held by a sentence rather than by structure: the four envelope harnesses (envelopes-a-4, envelopes-b-8), five source scanners with three fn-declaration rules (tests-other-3, suite-economics-3, surface-roster-10), five preorder decoders of the version stream under `meter/board/` with two zigzag re-implementations (board-families-floors-judge-24, board-frame-22), the 2-bit tag decode and subtree skip at six sites (party-4, skyline-fill-grow-34), the pair-difference seeding at four production sites while `OpenedPair` claims one home (skyline-sweep-place-masked-15), the arity-N advance law stated twice (skyline-sweep-place-masked-7), two strict skyline parsers (skyline-coding-33), the fold dispatch three times (version-core-8, span-causally-9), the undercut tail four times and the latent ladder three (skyline-watermark-18, skyline-watermark-19), the op applier four times (testing-diff-gen-16, testing-oracles-11), the suspended-frame discipline twice (skyline-fill-grow-27), `PackedBuilder`'s reimplementation of `BitsBuf` (codec-bits-12), and the register-or-digit-0 dispatch at five sites in suanpan (suanpan-9). Test code carries the same theme at lower stakes: replay loops, fixtures, probes, and counter helpers copied per module (suanpan-tests-6, envelopes-b-19, board-ops-render-28, skyline-watermark-23, tests-other-7, suite-economics-4, fuelscape-render-13, meter-registry-tier2-17).

**Instruments with no reader, and buffers waiting for failures.** Principle 3's question, what constructible failure does this catch that nothing else does, has no answer for the segments column; for the sixteen `meter_wide` hooks feeding a counter no committed check reads while the wide arm is engaged (rank-26); for two of three `EmitTraffic` cells (skyline-watermark-28); for `FamilySpec.denominator` and `closed_form` (meter-registry-tier2-8); for `ff_reset`, `__FS_NO_ANIM`, `Fuelscape.parse`, and six dead accessors (fuzz-guests-pins-19, fuelscape-render-26, fuelscape-pipeline-13); and for `Dyadic`'s four hand-written trait impls (testing-oracles-9). The empty acceptance buffers the doctrine forbids even when empty exist at four sites: `EXEMPTIONS` (testing-diff-gen-31), `ITEM_EXCEPTIONS` (surface-roster-18), and covcheck's `remediation` disposition with benchjudge's `red` class (gate-legs-9, meter-adequacy-8).

**Guards that recompute what committed tests already hold.** `add_at`'s exit scan and `enter_digit_engine`'s buffer scan restate clauses `ledger_invariants_hold_exhaustively` asserts after every step, at O(wide) per write in debug (suanpan-21, inventory-4); `Sub for Base`'s `debug_assert!` duplicates the backend's underflow panic and performs metered work inside dev-profile envelopes (codec-base-text-tree-7); `balanced_reduce`'s assert checks what its own closures make impossible (crate-root-22); `check_with` re-validates a pin table a committed test already pins (board-ops-render-23); the `hull.relation` `debug_assert!` is the only production reader of the field it checks (version-core-13); `held_at` exists for one debug assertion (skyline-coding-11); `validate_id` re-runs on bits the text and literal parsers built canonically (codec-base-text-tree-16, codec-bits-25, inventory-6); and `fuzz_decode` asserts what the differential target's borsh arm implies (fuzz-guests-pins-4).

**Rosters and literals the compiler could hold.** `BOARD_DECLARED_BENCH_RIDERS` (board-frame-25); `designed()`'s and `FamilyData::build`'s nineteen-variant envelope-only arms (board-ops-render-2, board-families-floors-judge-6); the freeze widths 288, 608, and 33 (meter-core-7, envelopes-a-19); `LEDGER_OPS` (suanpan-tests-9); the `dispatch!` arm list beside `COMBINE_ARITY_CAP` (fuzz-guests-pins-22); `hull_traffic`'s hand-enumerated `reset` and `snapshot` (version-core-26); `BespokeGenre::GENRES` (testing-diff-gen-4); derived liveness floors written as literals beside literal scales (envelopes-b-26); and generator minimum widths as bare clamps (board-families-floors-judge-5). Each is a place where an edit to one spelling leaves the other compiling.

**Things one module away from home.** `designed()` in the operation module (board-ops-render-2); `both_present_nodes` in the constants module (board-frame-13); the measurement seam beside the renderer (board-ops-render-14); `Div<&Party> for &Version` away from `OwnVersion` (version-core-19); the `Cow<Version>` `From` impls in `span.rs` while `causally::forms` depends on them (span-causally-6); the `Rank` and `Ranked` suites two modules from their types (version-core-27, rank-2); the serde and borsh legs in `clock/tests.rs` (clock-26); `Memo` and `PreScan` driven through `pub(super)` fields from `fill.rs` (skyline-fill-grow-12); the lease/retire pool on `MinWeb` (skyline-watermark-20); the zero-run ledger as a field plus three methods on `Accumulator` (suanpan-22); and the fold's retention-arm witness beside the laws while `fold.rs` has no tests (oracle-laws-26).

**Vocabulary and register.** The idiom findings agree that these are crate dialect to be swept once rather than partition by partition: "mint" for constructing a value, at 56 to 82 sites depending on the sweep's file set (paper-fidelity-13, recursion-9, skyline-query-23, tests-other-23, fuelscape-pipeline-2, board-families-floors-judge-15); "honest" as an undefined soundness criterion (board-families-floors-judge-15); em-dashes in `//` comments and terminal-bound messages, 374 lines in `crates/before/src` and 551 plain-comment lines across the wider file set, with rustdoc exempt (prose-hygiene-12 and the partition instances it subsumes); `core::` beside `std::` imports (span-causally-10, oracle-laws-7); and qualified paths beside existing imports in nearly every partition. Every summary that touched these recommends one owner ruling and one mechanical commit.

**Convergent findings.** Twenty-two sites were found independently by two or more partitions or sweeps; the synthesis notes on the entries name each pairing (for example the `Shl<i32> for Base` impl reached by clippy-pedantic-1, codec-base-text-tree-8, and inventory-5, or the dead `len == 64` arm reached by codec-bits-28, inventory-9, and recursion-8). Convergence is evidence that a finding is not an artifact of one reviewer's lens. The assembly kept every entry rather than merging them, because each carries its own anchors and acceptance clause; a maintainer closes each pairing with one change.

## Dissolvable scaffolding

Each entry below opens with two lines: what the instrument catches (or claims to catch), and what would replace it. The doctrine's retirement bar, that the replacement demonstrates it catches what the instrument caught before the instrument is deleted, is named inside the entry where it applies. Nothing here is dissolvable because it looks redundant; every dissolution names the committed check that already holds the property, or names the property as unheld and asks for a decision. The thirty-three entries fall into five kinds: counters and hooks with no live reader in the binary that judges them (board-frame-1, meter-core-11, rank-26, skyline-watermark-28); empty buffers and dated rulings (testing-diff-gen-31, surface-roster-18, gate-legs-9, meter-adequacy-8, surface-roster-17, meter-registry-tier2-9, meter-adequacy-12, deps-16); second implementations beside a stronger first (surface-roster-9, tests-other-3, envelopes-a-8, envelopes-b-17, fuzz-guests-pins-4, testing-oracles-25, oracle-laws-2, oracle-laws-24, deps-10, fuelscape-render-10, tools-3, tools-5, tools-20); guards that recompute committed coverage (suanpan-21, inventory-4, crate-root-22); and hand-maintained lists or seams beside a derivable source (board-frame-25, fuzzfit-bands-18, gate-legs-10, testing-oracles-17, skyline-fill-grow-32). Entries are grouped by the module that owns the instrument; the per-module sections below point back here rather than repeating them.

### Rank

Catches: Nothing: sixteen `meter_wide` hooks feed a counter no committed check reads while the wide arm is engaged (32-bit targets, or the host under the test ceiling).

Replacement: A liveness pin under `ceiling::force(TEST_CEILING_BITS)` with a derived floor; or delete the hooks and price wide-arm cost by memory and digit touches.

#### rank-26: Wide-arm limb metering has no observer: sixteen meter_wide hooks feed a counter no committed check reads while the arm is engaged
- Where: crates/before/src/version/rank/num.rs:319-324 (related: num.rs:37-48, num.rs:58-69, crates/before/src/codec/base/limb_meter.rs, .cargo/mutants.toml:80)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (`grep -rln 'ceiling::force\|arm_ceiling::force' crates/before/` returns only version/tests.rs and num/tests.rs; `grep -n 'limb_ops\|limb_meter'` on those two files returns nothing; the `limb_ops()` readers are meter.rs, tests/meter.rs, meter/tests.rs, board/*, query/tests.rs, testing/asymptotics.rs, and two integration tests, none of which forces the ceiling; the override module is `#[cfg(test)]` (num.rs:113), so integration tests cannot reach it; the wasm32 pins contain no `limb_ops` read); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (79a944ab extends the meter convention to the wide arm but pins no floor; .cargo/mutants.toml carries no num.rs entry)
- Owner-gated: no

`meter_wide` records wide-arm work at sixteen sites, justified by a liveness principle, but the wide arm engages in production only on 32-bit targets (no envelope suite runs there) and on the host only under the test ceiling, which only suites that never read the limb meter use. A wide decode recording nothing would fail no test; deleting every `meter_wide` call would fail no test. Principle 3: an instrument hook earns its place by naming a committed check that reads it; a meter needs a liveness floor or it is decoration.

Evidence:

       319	    /// alignment shift. Below the ceiling this is exactly the historical
       320	    /// spelling ([`Base::from_be_bytes`] then the metered sub-byte shift);
       321	    /// above it the limbs are assembled directly — no backend value ever
       322	    /// exists — with the materialization metered on the value's width, the
       323	    /// wide-decode convention: a meter that missed it would let a decoder
       324	    /// build arbitrarily wide values while recording nothing.

        64	fn meter_wide(limbs: u64) {
        65	    #[cfg(feature = "limb-meter")]
        66	    crate::codec::base::limb_meter::record(limbs);

Resolution: Either make the hooks live: one `#[cfg(all(test, feature = "limb-meter"))]` unit pin under `ceiling::force(TEST_CEILING_BITS)` that resets the meter, decodes a stream whose numerator crosses the ceiling, and asserts `limb_ops() >= bits.div_ceil(64)` (the floor derived from irreducible work: every limb of the value is materialized), plus the same for one `+` on the accumulate route; or remove `meter_wide` and its call sites and state in the Metering section that wide-arm cost is priced by memory and suanpan's digit-touch meter. Acceptance: a committed test fails when any `meter_wide` call is deleted, or `meter_wide` is gone and the module doc's Metering section describes what remains.

Construction: Make `meter_wide` a no-op (or delete its sixteen calls) and run `just test-all`; nothing fails today. The proposed pin: `let _g = ceiling::force(96); limb_meter::reset(); let r = Rank::decode(&bytes_of_a_200_bit_rank[..]).unwrap(); assert!(limb_meter::limb_ops() >= 200u64.div_ceil(64));`.

### Watermark and the traffic counters

Catches: `DominatedUndercut` has a committed floor; `DominatedAbove` and `Undecided` are recorded and never read, and the snapshot doc overstates what the fields sum to.

Replacement: Pin the two on a family with a derivable count, or reduce `EmitTraffic` to the one enforced cell; reword the doc to the emission-path reads.

#### skyline-watermark-28: Two of three emit-traffic counters are recorded but never read, and the snapshot doc overstates what they sum to
- Where: crates/before/src/version/skyline/web_traffic.rs:42-55 (related: watermark.rs:962-985, 1036-1043; tests/meter.rs:9250; fill/tests.rs:468, 600)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (`grep -rn -E '\.dominated_above|\.undecided|Decision::Undecided|Decision::DominatedAbove' crates/before --include='*.rs'` matches only the record sites at watermark.rs:966 and 984 and web_traffic's own dispatch; every reader reads `.dominated_undercut`; `compare_above`'s `sign_dominates_word` at 1039 records nothing); executed: no
- Seen by: claims; refutation: confirmed; history: no-rationale-found (the three-cell shape and the "fields sum to" sentence were copied from hull_traffic's four-rung idiom; 76579a3c pinned only `dominated_undercut >= k`)
- Owner-gated: no

Circular justification: a counter earns its existence by naming what reads it. The module's own liveness argument (an undriven arm is where a polarity error waits) applies to the undercut arm, which has a floor; `DominatedAbove` and `Undecided` are decoration until a family pins them. The doc sentence is also inaccurate: `compare_above`'s domination read is unclassified, so the fields sum to the emission-path reads only.

Evidence:

        42	/// A snapshot of the three decision counters, in emissions.
        43	///
        44	/// Read through `meter::emit_traffic`; the fields sum to the domination
        45	/// reads performed since the last reset.

Resolution: either (a) pin `dominated_above` on a committed family that provably routes emissions through the return-early arm (the dominated-undercut family's own sites may yield a derivable count; state the derivation in the floor doc) and fold `Undecided` into a single fallback cell, or (b) reduce `EmitTraffic` to the one enforced cell. In both cases reword 44-45 to "the emission-path domination reads". Acceptance: every field of `EmitTraffic` is read by at least one committed floor or band, or the struct has one field; the doc names the emission path.
Construction: delete the `record(Decision::DominatedAbove)` call at watermark.rs:966 and run the full suite: nothing turns red.

### suanpan

Catches: The exit invariant `ledger_invariants_hold_exhaustively` already holds, re-checked by an O(wide) scan on every digit write in every debug build.

Replacement: Keep the O(1) conjunct, delete the scan, name the ledger suite in the comment; confirm with the mutants campaign that no mutant moves from caught to survived.

#### suanpan-21: `add_at`'s exit `debug_assert!` scans the whole buffer above `top` on every digit write; the ledger suite already holds the clause
- Where: crates/suanpan/src/accumulator.rs:1391-1395 (related: 1365-1368; crates/suanpan/src/accumulator/tests/ledger.rs:51-60; Cargo.toml:180-187; .cargo/mutants.toml:33-35)
- Class / severity / confidence: scaffolding / medium / high
- Provenance: verified (read: `assert_ledger_invariants` asserts the identical two clauses at ledger.rs:51-60 after every step of every schedule and on the randomized streams; root Cargo.toml:180-187 keeps debug-assertions on for suanpan in the dev profile; `digits.len()` shrinks only through `shl`'s `mem::take` (626) and `reset` keeps the buffer (676-678)); executed: no
- Seen by: structure (0), claims (47); refutation: confirmed both; history: deliberate for its purpose but in direct tension with a recorded ruling: 9f68c475 (2026-08-07) retired `read_magnitude`'s O(skipped-prefix) debug_assert "per the recompute-and-compare ruling" and declared "The surviving debug_asserts are O(1) or once-per-epoch discipline documentation"; b93a3628 (2026-08-10) then added this O(buffer) exit scan knowingly, its message conceding "The exhaustive ledger sweep (which checks the same clause after every public step) passes unchanged". The `enter_digit_engine` scan (1293-1296) is once-per-epoch and stays within 9f68c475's ruling; that leg of the lens findings is dropped.
- Owner-gated: yes (reopens a recorded ruling; the new evidence is the earlier ruling it contradicts and the cost below)

Doctrine (Assertions and Guards): a runtime assert on a deterministic function is justified as an O(1) probe, not an O(n) re-encode, once committed coverage holds the invariant. The second conjunct walks `digits[top + 1..]` on every `add_at` in every debug build: a pooled accumulator that was once wide pays O(wide) per word-scale write for the rest of the process, in suanpan's and before's whole test suites, against an amortized-O(1) contract. Neither assert body meters, so the exactness contract is unaffected either way.

Evidence:

      1391	        debug_assert!(
      1392	            (self.top == 0 || self.digits[self.top] != 0)
      1393	                && self.digits[self.top + 1..].iter().all(|&digit| digit == 0),
      1394	            "top must rest on the highest nonzero digit at add_at exit"
      1395	        );

    ledger.rs:
        51	    assert!(
        52	        acc.digits[acc.top + 1..].iter().all(|&digit| digit == 0),
        53	        "nonzero digit above top {} after {schedule:?}",
        54	        acc.top
        55	    );
        56	    assert!(
        57	        acc.top == 0 || acc.digits[acc.top] != 0,
        58	        "top {} rests on a zero digit after {schedule:?}",
        59	        acc.top
        60	    );

Resolution: keep the O(1) conjunct (`self.top == 0 || self.digits[self.top] != 0`), delete the `digits[top + 1..]` scan, and re-state the comment at 1365-1368 to name `ledger_invariants_hold_exhaustively` (and the run-forming stream property) as the check that holds the exit invariant. Acceptance: the retiring-an-instrument discipline: run the suanpan mutants campaign under the configuration of record before and after and confirm no mutant moves from caught to survived (the dev profile's assertions are "part of the observer", mutants.toml:33-35, and the ledger suite's assertions remain in it); the ledger suite unchanged. Construction (debug build; corrected from the lens: `reset()` returns to the register at 678, so `add_small` after a reset never reaches `add_at` and a spill must precede the loop): `let mut acc = Accumulator::new(); acc.add_wide_shl(&UBig::ONE, 32 * 100_000); acc.reset(); acc.add_wide(&UBig::ONE); for _ in 0..100_000 { acc.add_small(1); }`: after the spill every `add_small` reaches `add_at` and the exit scan walks about 100,000 slack digits, roughly 10^10 element reads in debug; linear in release.

Synthesis note: inventory-4 (this document) is the sweep-side record of the same assert together with `Sub for Base`'s (codec-base-text-tree-7). The suanpan summary notes the tension with 9f68c475's recorded ruling, which is why this is owner-gated.

### Oracle and laws

Catches: The hand-back grouping and drain order of `join_all`, which both `# Errors` sections declare unspecified; everything the contract promises is already held oracle-free by `PARTY_AND_LIST` and `CLOCK_AND_LIST`.

Replacement: A sequential oracle `join_all` compared on verdict, accumulator, and region union; or, if the discipline is wanted pinned, one pin at `fold.rs` over a plain payload and the oracle copies retired.

#### oracle-laws-2: The oracle's `join_all` transcribes production's balanced fold, twice, and the differential pins a hand-back shape the contract declares unspecified while the module doc claims paper transcription
- Where: crates/before/src/oracle/clock.rs:66-108 (related: crates/before/src/oracle/party.rs:101-143, crates/before/src/fold.rs:41-81, crates/before/src/party.rs:308-372, crates/before/src/clock.rs:229-240, crates/before/src/party/tests.rs:65-170 and 182-290 and 292-310, crates/before/src/clock/tests.rs:24-139, crates/before/src/laws.rs:2330-2437 and 3282-3388, crates/before/src/testing/validation_index.rs:172-173, crates/before/src/oracle/version.rs:377-379, crates/before/src/oracle.rs:6-9)
- Class / severity / confidence: scaffolding / medium / high
- Provenance: verified (read `oracle/clock.rs:66-108`, `oracle/party.rs:101-143`, and `fold.rs:47-80` side by side: identical control flow, identical retention policy, identical `expect` strings; read both production contracts; read both differential harnesses and the known-bad variant; `git show -s ace236595` and the b3f09baa0 hunks the history pass cited); executed: no
- Seen by: scaffolding [0], adequacy [12], structure-prose [24], instrument-correctness [36]; refutation: confirmed (all four), with the caveat that the copy check is live against the conditional-drop class via committed seed `cc 5d25e701...` and is blind only to a defect mirrored into both spellings; history: deliberate-but-expired (ace236595 transcribed "the exact fixed-accumulator up-front test, binary-counter grouping, and overlap-at-merge hand-back order the production contract documents"; b9f6af2d7 retired the counter-sharing `fold_oracle` because "only one algorithm was left live on both sides"; b3f09baa0, the owner's docs pass, rewrote both contracts to "unspecified" and deleted the oracle-side `join_all` docs that stated the transcription's purpose, leaving the pin without its definition-site rationale)
- Owner-gated: yes (a recorded design decision, ace236595 and party/tests.rs:65-75; the resolution changes what the differential pins)

The oracle module doc promises "no second representation to keep in sync" and a reference that is "obviously correct"; `join_all` on both oracle types is instead a token-level copy of `fold::balanced_try_fold`, spelled a third and fourth time in `party/tests.rs` (the known-bad variant) and `fold.rs` (Principle 3: machinery outlives the constraint that justified it; the tell is that a contract-conforming reshaping of the production fold can only be kept green by editing the oracle to match, which validation_index.rs:172-173 forbids). What the element-wise differential pins beyond the laws is the hand-back grouping and order, which `party.rs:314-315` and `clock.rs:236-237` leave unspecified; everything the contract does promise (acceptance iff pairwise disjoint, accepted fold equals sequential joins, nothing dropped, region conservation) is already stated oracle-free by `PARTY_AND_LIST` and `CLOCK_AND_LIST`. The oracle's own `Version::meet_all` shows the paper-faithful shape: a plain `reduce`.

Evidence:

    oracle/clock.rs:
        75	            let mut weight = 0u32;
        76	            while stack.last().is_some_and(|(_, w)| *w == weight) {
        77	                let (mut top, _) = stack.pop().expect("the loop condition saw a top entry");
        78	                match top.join(merged.take().expect("the operand is held while merging up")) {

    fold.rs:
        54	        let mut weight = 0u32;
        55	        while stack.last().is_some_and(|(_, w)| *w == weight) {
        56	            let (top, _) = stack.pop().expect("the loop condition saw a top entry");

    oracle.rs:
         6	//! `Party` and `Version` *are* the trees; every operation is a method, so there
         7	//! is no second representation to keep in sync. Deliberately simple,
         8	//! suboptimal, and recursive: its only job is to be obviously correct, so it
         9	//! can serve as differential ground truth. It mirrors the target's **semantic**

    party.rs (the production contract):
       314	    /// handed back. In case of partial error, the set of parties which are
       315	    /// absorbed vs. handed back is unspecified.

    clock/tests.rs (what the differential compares):
        98	/// Identical outcomes: the same `Ok`/`Err` verdict — the returned version
        99	/// lowering to the oracle accumulator's — the same hand-back vector (contents
       100	/// *and* order, element-wise over `to_oracle_clock`), and accumulators (party
       101	/// and version both) lowering to the same oracle trees.

    party/tests.rs (the surviving rationale):
        71	// The recursive oracle's `join_all` (`oracle::Party`) is that discipline's
        72	// reference spelling, and these differentials pin production against it across

    oracle/version.rs (the paper-faithful shape the oracle already uses for the meet):
       377	    pub fn meet_all(iter: impl IntoIterator<Item = Version>) -> Option<Version> {
       378	        iter.into_iter().reduce(|acc, v| acc & v)
       379	    }

Resolution: Decide first whether the fold's retention and drain discipline is a pinned behavior. If not: replace both oracle `join_all` bodies with the sequential reference (test each input against the fixed accumulator; refused inputs hand back individually in feed order; `for other in inputs { if let Err(back) = self.join(other) { overlapping.push(back) } }`), and widen the `Err` arm of both `assert_join_all_matches_recursive_oracle` harnesses to the contract: same verdict, same final accumulator, and equal region union (party) or region union plus history join (clock) of the hand-backs, not element-wise order. That comparison still pins the `IdIndex` accept seam (a wrongly accepted or refused input moves the accumulator and the union) and still convicts the dropped-group variant (on `[a, b, alias(a), c, d, e]` the variant returns `Ok` and loses `c`, so verdict and union both differ); restate `join_all_differential_convicts_the_dropped_group_oracle`'s doc accordingly. If the discipline is wanted pinned: say so in both `# Errors` sections and pin it once at `fold.rs` over a plain payload type (see oracle-laws-26), then retire the oracle copies per the retirement discipline. Do not route the oracle through `crate::fold`: b9f6af2d7 retired exactly that shape. In either branch, amend oracle.rs:6-9 so it no longer claims paper transcription for `join_all`. Acceptance: no weight-stack loop remains outside `fold.rs` and the known-bad test variant; the surface roster rows for `Party::join_all` and `Clock::join_all` still cite live bindings (citecheck green); `just gate` clean.

Construction: In `party.rs:362` reverse the closing drain (`for group in groups.into_iter().rev()`), a contract-conforming change since the absorbed set is unspecified. On the committed feed `[a, b, alias(a), c, d, e]` (party/tests.rs:105-115) the stack is `[a∪b, alias∪c∪d∪e]`; production now absorbs `alias∪c∪d∪e` and hands back `a∪b`, the oracle absorbs `a∪b` and hands back `alias∪c∪d∪e`. `join_all_agrees_with_oracle_on_aliased_coalesced_group` goes red. Every `PARTY_AND_LIST` law stays green by reading: the acceptance law's `Err` arm sees `!pairwise_disjoint`, a nonempty hand-back, and `acc.covers(p)`; the conservation law sees the same region union; the best-effort and reunion laws feed families the drain order cannot affect (their aliases are rejected up front, and their fork shares are pairwise disjoint). The only edit that returns the differential to green is reversing the drain at oracle/party.rs:133 as well.

Synthesis note: oracle-laws-26 (this document) is the natural home for the fold pin this entry's second branch asks for. The oracle-laws summary's open question 1 recommends the first branch (no pinned discipline; a sequential oracle).

### Meter core

Catches: Nothing in any binary that reads it through the `meter` feature: the reading is a compiled-in zero, and the "future segments floor" slot is held open for a failure library code cannot produce.

Replacement: Dissolve the segments currency; `deep_tree_stack_safety` and the `cfg(test)` gate on `descend!` hold the property.

#### meter-core-11: The stack-segments meter has no writer in any build that reads it through the `meter` feature
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

Synthesis note: board-frame-1 (this document) carries the board-side record with the `LADDER_TOP_SCALE` derivation; the verification-gap documents carry crate-root-32, envelopes-a-2, module-graph-1, recursion-1, inventory-2, and board-ops-render-15. One change closes all eight entries.

### Registry and tier2

Catches: A malformed date shape, weakly: `"abcd-ef-gh"` and `"2026-13-99"` pass the length-10-two-dashes check.

Replacement: Drop `decided` and `REGISTRY_RATIFIED` and let git blame carry the when; or name the decision-record exemption once and share surfacecheck's typed date parse.

#### meter-registry-tier2-9: Dated rulings (`decided`, `REGISTRY_RATIFIED`) are dated rationale at declaration sites, enforced by a shape-only date test
- Where: crates/before/src/meter/registry.rs:1102-1103 (related: registry.rs:1075-1080, 1091-1096; crates/before/src/meter/registry/tests.rs:204-226; crates/before/surfacecheck/src/check.rs:259-273)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (read the fields and the test; `git show d2a9d04e` message reports the machinery as needing "a design round, not a prose sweep"; surfacecheck's `dated` closure at check.rs:262-273 checks month and day ranges); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: already known (d2a9d04e, 2026-07-31, reported it to the owner; no design round is recorded since)
- Owner-gated: yes (a documented convention shared with surfacecheck; one ruling covers both)

`Coverage::EnvelopeOnly { decided }`, `Bands::Unbanded { decided }`, and `REGISTRY_RATIFIED` carry calendar dates whose only reader is `envelope_only_rulings_are_dated`, a length-10-two-dashes check (`"abcd-ef-gh"` and `"2026-13-99"` pass) that surfacecheck's sibling check does properly. Doctrine under Principle 5: dated rationale at a declaration site is history in the tree; git blame carries the ruling date with no maintenance. The module doc treats these as decision records ("the dated ruling of record") without naming the exemption.

Evidence:

      1102	/// The date the registry's rulings were ratified as the rows of record.
      1103	const REGISTRY_RATIFIED: &str = "2026-07-29";
      1078	        /// The date of the ruling of record.
      1079	        decided: &'static str,

    registry/tests.rs:
       215	                decided.len() == 10 && decided.chars().filter(|&c| c == '-').count() == 2,

Resolution: rule once for the registry and surfacecheck. If the registry is a decision record: say so in one sentence of the module doc and lift surfacecheck's date parse into a shared helper. Otherwise: drop `decided` and `REGISTRY_RATIFIED`, keep `reason`, fold the non-empty-reason check into `band_citations_are_unique_and_nonempty`, and delete the date test. Acceptance: either the module doc names the decision-record exemption and one typed date parse is shared, or `grep -nE '20[0-9]{2}-[0-9]{2}-[0-9]{2}' crates/before/src/meter/registry*` is empty.

Synthesis note: surface-roster-17 (surfacecheck's `decided`) and meter-adequacy-12 (this document) are the same convention; rule once for both sites.

### The board: frame

Catches: Nothing, in any board binary: the counter's only writer is `recurse::grow`, which is `#[cfg(test)]`, so the column's ceiling passes vacuously and `LADDER_TOP_SCALE`'s segment-onset rationale describes an event the board cannot observe.

Replacement: Dissolve `Currency::Segments`; the no-recursion property is held by `clock::tests::deep_tree_stack_safety` and by `descend!` being `cfg(test)`.

#### board-frame-1: The segments currency reads zero by construction in every board binary; the front page, its ceiling, and the ladder-top derivation present it as a live meter
- Where: crates/before/src/meter/board.rs:47-49 (related: crates/before/src/meter/board/currency.rs:32-33, crates/before/src/meter/board/currency.rs:138-140, crates/before/src/meter/board/ceilings.rs:80-83, crates/before/src/meter/board/ceilings.rs:453-459, crates/before/src/meter/board/floors.rs:83-85, crates/before/src/meter/board/judge.rs:60-64, crates/before/src/recurse.rs:17-20, crates/before/src/recurse.rs:74-75, crates/before/src/recurse.rs:100-107, crates/before/Cargo.toml:33,44,115-117, justfile:889,893,904, crates/before/src/meter/tests.rs:399-403; outside the partition and for the envelopes reviewer: crates/before/tests/meter.rs:22-26,440-441)
- Class / severity / confidence: scaffolding / medium / high
- Provenance: verified (grep of `SEGMENTS_GROWN.fetch_add` over crates/before finds the single site recurse.rs:106 inside `grow`, which carries `#[cfg(test)]` at recurse.rs:100; `descend!` at recurse.rs:118-129 is `#[cfg(test)]`; Cargo.toml:33 `[dev-dependencies]` holds `stacker` at :44 and :115-117 declares the `amp_board` example with `required-features = ["limb-meter", "scan-meter"]`; justfile:889 and :904 run it as `cargo run --release ... --example amp_board`; judge.rs:366 compares `v > ceiling` strictly; `grep -c seg_ceiling_only()` gives 13 in floors.rs and 38 in ops.rs; `grep 'segments: Liveness::Floor'` finds no segments floor; `git log -1` on 05bd2b16d (2026-07-26, gated `grow` and `descend!` under cfg(test)), 1ddb5a483 (2026-07-31, stacker to dev-dependency), 9e36dd280 (2026-08-07, `LADDER_TOP_SCALE` with the segment-onset rationale)); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed (with justfile:893 added as a further site); history: deliberate-but-expired (05bd2b16d kept the column with "its own liveness now witnessed by a test-local guarded descent"; that witness runs only under cfg(test), where `grow` exists)
- Owner-gated: yes (removal of an instrument column and re-derivation of a ratified constant)

No code compiled into the board of record (the release `amp_board` example, or any integration or bench binary) can increment `SEGMENTS_GROWN`: its only writer is `recurse::grow`, which is `#[cfg(test)]`, and `stacker` is a dev-dependency. The column's ceiling therefore passes vacuously (Principle 2: a ceiling over a counter that cannot count), the column names no constructible failure the board can catch (Principle 3: circular justification, machinery outliving the constraint that justified it), and three sites state as present-tense fact what the binary cannot measure: board.rs:47-49 calls it "the honest stand-in for recursion-driven stack cost", recurse.rs:19-20 calls the zero "the measured fact the boards' segments column pins", and recurse.rs:74-75 says "the counter is always written". `LADDER_TOP_SCALE`'s ×4 (ceilings.rs:455-459) and the justfile's "segment-onset witness" (justfile:893) derive from segment-onset amplifiers that cannot fire there; the "~1 MiB of frames" onset figure is not derivable from recurse.rs's constants either (RED_ZONE at :51 is 256 KiB of remaining stack; STACK_GROWTH at :55 is the 1 MiB segment size). `MAX_GROWN_STACK_SEGMENTS = 1` admits one growth (judge.rs:366 is strict `>`) against a stated target of zero, with no rationale for the 1. The liveness witness in meter/tests.rs:399-403 proves the counter is alive under cfg(test); it says nothing about the board's binary.

Evidence:

        47	//! - **grown stack segments** ([`stack_segments`](crate::meter::stack_segments)),
        48	//!   the honest stand-in for recursion-driven stack cost, which bypasses any
        49	//!   heap meter;

    [recurse.rs:17-20]
        17	//! what lets it meet deep inputs safely. The segment counter below stays
        18	//! compiled for the meters: it is the deterministic stand-in for
        19	//! recursion-driven stack consumption, and its zero reading over the library
        20	//! kernels is the measured fact the boards' segments column pins.

    [recurse.rs:100-107]
       100	#[cfg(test)]
       101	#[inline]
       102	pub(crate) fn grow<R>(f: impl FnOnce() -> R) -> R {
       103	    if stacker::remaining_stack().is_some_and(|remaining| remaining >= RED_ZONE) {
       104	        f()
       105	    } else {
       106	        SEGMENTS_GROWN.fetch_add(1, Ordering::Relaxed);
       107	        stacker::grow(STACK_GROWTH, f)

    [ceilings.rs:80-83]
        80	/// Green requires at most this many grown stack segments, as an absolute count:
        81	/// the target is walks that never grow the stack, so the ceiling is flat, not
        82	/// per-byte.
        83	pub const MAX_GROWN_STACK_SEGMENTS: u64 = 1;

    [ceilings.rs:455-459]
       455	/// The base-scale sizes under-detect segment amplifiers: stacker grows a
       456	/// segment only past ~1 MiB of frames, so a recursion-frame amplifier whose
       457	/// onset sits above the base depths reads a false green there. ×4 is the
       458	/// witnessed calibration floor — the smallest sampling scale at which every
       459	/// segment-onset amplifier the suite has caught reads red — so the ladder

    [Cargo.toml:115-117; justfile:889]
       115	[[example]]
       116	name = "amp_board"
       117	required-features = ["limb-meter", "scan-meter"]
       889	    cargo run --release -p before --example amp_board --features limb-meter,scan-meter -- {{ args }}

Resolution: Owner's call between two consistent states. (a) Dissolve: remove `Currency::Segments` and the `segments` field from `ByCurrency` (the totality mechanism ripples the removal through every `Floors` literal, the judgment, and the render), delete `MAX_GROWN_STACK_SEGMENTS`, `seg_ceiling_only`, and `SEG_FLOOR_TRIP`, re-derive `LADDER_TOP_SCALE`'s doc from what the ladder still buys (the four-point trend and the heap flat-allowance regime, or state it as owner-ratified with the calibration history in the pin commit), re-word recurse.rs:17-20 and :74-75 to the true statement (the counter is written only by the test-surface guard; the depth guarantee is AGENTS.md's iterative rule plus `deep_tree_stack_safety`), and re-word justfile:893. Keep `SEGMENTS_GROWN` and `stack_segment_meter_counts_deterministically_and_resets` only if the oracle bridge's guard liveness needs the witness. (b) Keep as a structural pin: set the ceiling to 0, document at currency.rs:32 and ceilings.rs:80 that the reading is identically zero on every non-test binary because the library's only growth arm is test-only, drop the segment-onset sentence from `LADDER_TOP_SCALE`, and explain what the column would ever catch (nothing outside cfg(test), so this is the weaker state). History favors (a): 1ddb5a483's reason for the dev-dependency move is exactly that no library code recurses. Acceptance: (a) `grep -rn segments crates/before/src/meter/board` returns no currency field, `just gate` is clean, and no prose in board.rs, ceilings.rs, floors.rs, recurse.rs, or the justfile names segment onset or a measured segments column; (b) `MAX_GROWN_STACK_SEGMENTS` is 0 and the constant's doc names the cfg boundary.
Construction: Mechanically, `grep -rn 'SEGMENTS_GROWN.fetch_add' crates/before/src` yields one site under `#[cfg(test)]`, so `cargo build --release --example amp_board --features limb-meter,scan-meter` contains no writer of the atomic. Behaviourally, add an unguarded deep plain recursion (no `descend!`, which is unavailable to non-test code) to any board row's body on a deep family and run `just amp-board`: the segments column stays 0 and the cell stays green on that column, or the child overflows its stack; no input exists on which the column reads nonzero.

Synthesis note: meter-core-11 (this document) is the same column seen from the meter partition, and six findings in other documents reach the same conclusion from their own vantage: crate-root-32, envelopes-a-2, module-graph-1, recursion-1, inventory-2, and board-ops-render-15 (all verification-gap). Every review that touched the column recommends dissolution (option (a)); no review found a live writer in a `meter`-feature binary.

Catches: A declared-model cell losing its wall-clock witness, but in one direction only (rider implies declared); a new declared model on a non-designed pairing lands with the test green.

Replacement: Derive the pinned subset in `bench_cells` from `designed || declared_heap || declared_limb` on the prepared `Cell`; delete the constant and the one-way test.

#### board-frame-25: `BOARD_DECLARED_BENCH_RIDERS` is a hand-maintained cell list in a module whose doc says none exists, derivable from each cell's own declarations, pinned in one direction only, and it is the time leg the committed cadence judges
- Where: crates/before/src/meter/board/export.rs:104-121 (related: crates/before/src/meter/board/export.rs:9-11, 92-93, 144-151, crates/before/src/meter/board/tests.rs:1089-1127, crates/before/src/meter/board/ops.rs:107-187, crates/before/src/meter/board.rs:184-187, 293, crates/before/src/meter/registry.rs:581, tools/benchjudge-expected.json:2, justfile:840, 843-844, 1003)
- Class / severity / confidence: scaffolding / medium / high
- Provenance: verified (tests.rs:1103-1125 asserts only rider implies declared (`declared_heap.is_some() || declared_limb.is_some()`); export.rs:147-148 unions the list with `designed` and :151 already runs `prepare`, so the `Cell` is in hand; `designed` maps `AscendCliff` and `MirrorWide` to `Tick` only (ops.rs:125-135) while `version_min_ticks` is `Measure` and the display rows are `Version`/`Clock`, and the other `with_declared_heap` rows (ops.rs:392, 428, 1587) are `Tick` rows on `AscendCliff`, so the derived set equals today's three riders; justfile:840 `bench-judge sampling="quick" cells="pinned"` sets `BOARD_BENCH_MODE=full` only when `cells == "full"` (:843-844) and `just all` (:1003) invokes it bare, so the pinned subset is what the cadence judges); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed (severity stands at medium because pinned is the cadence of record; the export.rs:9-11 "full ... for final verdicts" sentence describes a mode no recipe runs); history: deliberate-but-expired (born as `BOARD_RED_BENCH_RIDERS` in 3eadcb107, a triage set with no cell property to derive from; 669cf3103 migrated every red to a `Cell` property and kept the list form only to hold the bench roster byte-identical, adding the one-direction pin; the constraint that required a list expired in that commit)
- Owner-gated: yes (removes a `pub` const of the meter-feature surface cited from registry.rs and the benchjudge roster's notes; whether the fold and capacity models count as declared models for rider purposes is an owner ruling)

export.rs states twice that the pinned subset is "a rule over the product, never a hand-maintained cell list" (:10-11, :92-93), yet the constant is exactly that, with a prose roster of its "current membership" and an instruction that a cure must edit it. The stated policy (:107-108) is that any declared-model cell keeps a wall-clock witness; the rule `designed(kind, group) || cell.declared_heap.is_some() || cell.declared_limb.is_some()` implements it from the `Cell` `bench_cells` already prepares. The test pins only rider implies declared, so a new declared model on a non-designed pairing lands without a time-leg witness while the test stays green (Principle 6), and the "declared model" the test knows (heap, limb) is narrower than the four ratified models board.rs:184-187 lists. Because `just all` runs the pinned subset, a missing rider has no wall-clock witness in the cadence that actually runs; the sentence at :9-11 saying full mode "times the whole product for final verdicts" describes a mode no committed recipe invokes.

Evidence:

       107	/// A cell judged green under an owner-declared counter model keeps a wall-clock
       108	/// witness even where no designed pairing times its shape.
       109	///
       110	/// Membership is by `(operation, family)` cell name, expectations live in the
       111	/// judge's roster as ever; a cell whose declared model dissolves (a cure
       112	/// landing) leaves this list in the same change. The current membership: the
       113	/// `version_min_ticks` reign-state cell and the display pair's render-merge
       114	/// cells. The tick trio's ascend-cliff cells need no rider — the tick group is
       115	/// those crosses' designed diagonal — and the tooth-tail parse cell rides its
       116	/// designed pairing likewise.
       117	pub const BOARD_DECLARED_BENCH_RIDERS: &[(&str, &str)] = &[
       118	    ("version_min_ticks", "ascend-cliff"),
       119	    ("version_display", "mirror-wide"),
       120	    ("clock_display", "mirror-wide"),
       121	];

    [export.rs:9-11]
         9	//! declared-model riders) while `BOARD_BENCH_MODE=full` times the whole product
        10	//! for final verdicts — the subset is a rule over the product, never a
        11	//! hand-maintained cell list.

    [board/tests.rs:1120-1121; justfile:840]
      1120	        assert!(
      1121	            cell.declared_heap.is_some() || cell.declared_limb.is_some(),
       840	bench-judge sampling="quick" cells="pinned":

Resolution: In `bench_cells`, keep the prepared `Cell` and include a cell under `BenchMode::Pinned` when `designed(family.kind, op.group) || cell.declared_heap.is_some() || cell.declared_limb.is_some()`; delete the constant, its re-export at board.rs:293, the registry.rs:581 link, and the one-way test (the property becomes structural); re-word the benchjudge roster's notes. Rule explicitly on the fold and capacity models (today every such cell sits on a designed pairing, so the derived subset is unchanged either way) and state the rule at `BenchMode::Pinned`. Re-word export.rs:9-11 to what the cadence judges (the pinned subset; full mode is an owner-invoked verdict) or add a full-mode leg to `just all`. If the constant must survive as public vocabulary, pin the converse instead: compute declared minus designed from the cell table and assert set equality with the roster. Acceptance: `bench_cells(scale, BenchMode::Pinned)` at the record scales yields today's cell set (diff the sidecar's denominator file); adding `.with_declared_heap(x)` to a row on a non-designed family makes that cell appear in the pinned subset with no other edit; no `BOARD_DECLARED_BENCH_RIDERS` symbol remains, or a test asserts the converse.
Construction: Add `.with_declared_heap(ASCEND_CLIFF_MIN_TICKS_HEAP_BYTES_PER_INPUT_BYTE)` to `version_min_ticks` on a second family whose `designed` arm excludes `Measure` (e.g. `Staircase`, a Tick-designed family); `bench_riders_name_declared_model_cells` passes unchanged, `bench_cells(_, BenchMode::Pinned)` omits the new cell, and `just bench-judge` never times it, contradicting export.rs:107-108.

### The surface roster

Catches: Roster totality over `pub fn`s, which surfacecheck's rustdoc-JSON walk catches strictly more of (any file, feature-gated trees, trait-declared methods, consts, macros, every trait impl), demonstrated red in check/tests.rs.

Replacement: surfacecheck alone: delete `SURFACE_SOURCES`, `extract_public_fns`, and `roster_is_total_over_the_public_fn_surface`; re-word the "two jaws" prose at four sites.

#### surface-roster-9: Two extractors of the same `pub fn` surface: the line-scan's justifying constraint dissolved when surfacecheck landed
- Where: crates/before/src/testing/surface_coverage.rs:164-167 (related: crates/before/src/testing/surface_coverage.rs:208-217, crates/before/src/testing/surface_coverage.rs:263-273, crates/before/src/testing/surface_coverage/tests.rs:70-94, crates/before/surfacecheck/src/main.rs:5-8, justfile:917-918, justfile:464, justfile:469, justfile:1000, crates/before/src/meter/board/coverage/tests.rs:18-28, crates/before/tests/doc_hidden.rs:4-8, crates/before/tests/foreign_reexport.rs:6-12)
- Class / severity / confidence: scaffolding / medium / high
- Provenance: verified (read both extractors and their consumers; `extract_public_fns` is called from the roster test and from `meter/board/coverage/tests.rs:21` only; justfile:464 and 469 put `test-all` and `surface-totality` in different gate streams, and `just ci` (1000) omits `surface-totality`; the two forks entries extract nothing: `grep -n 'pub fn'` on `src/party/forks.rs` and `src/clock/forks.rs` finds only `pub(crate) fn new`, and `git log -S'pub fn'` on both files is empty); executed: no
- Seen by: scaffolding; refutation: confirmed, with one residual (surface-roster-21); history: deliberate-but-expired (67956d1f 2026-07-26 landed the scan as the only extractor; 9aa8c9ac 2026-07-30 landed surfacecheck, whose justfile comment calls the scan "a hand-named source-file list" and itself "the other jaw of the pincer"; no commit or note states what the scan catches that rustdoc JSON does not)
- Owner-gated: yes (retiring an instrument)

`extract_public_fns` (a rustfmt-shape line scan over the seventeen-entry `SURFACE_SOURCES` list) and surfacecheck's rustdoc-JSON walk both reconcile `METHOD_SURFACE` two ways. The JSON walk catches strictly more (any file, feature-gated trees, trait-declared methods, consts, macros, every trait impl) and has run in the gate since 2026-07-30; the scan's remaining payload is feedback in `cargo nextest run -p before` on the stable toolchain, and everything else it carries is cascade: the file list, `type_overrides`, `module_prefix`, the rustfmt-shape panics (surface-roster-28, -29), two forks entries that have never extracted anything, the board coverage test's dependence on the extractor rather than the roster, and the "two jaws" prose at four sites. Principle 3: infrastructure that reimplements a capability a mature tool provides (the compiler's own account of the surface) and machinery that outlives the constraint that justified it. The doctrine's retirement bar is met: surfacecheck's `unrostered`/`orphaned` categories are the same two directions, demonstrated red in check/tests.rs:60-79.

Evidence:

       164	/// The public-API source files of record. A new public module with
       165	/// inherent methods must be added here (and the roster test's coverage
       166	/// note updated), which is itself a reviewed diff.
       167	pub(crate) const SURFACE_SOURCES: &[SourceSpec] = &[
    ...
       271	pub(crate) fn extract_public_fns() -> BTreeSet<String> {
       272	    ::surface_scan::extract_public_fns(&crate_root(), SURFACE_SOURCES)
       273	}

    justfile:
       917	# in-tree roster test scans a hand-named source-file list; this leg is
       918	# the other jaw of the pincer, with no file list to forget. The checker

Resolution: retire before's line-scan extractor: delete `SURFACE_SOURCES`, `extract_public_fns`, and `roster_is_total_over_the_public_fn_surface`; have `meter/board/coverage/tests.rs::public_surface` read `METHOD_SURFACE` op names (it is held equal to the extraction anyway); re-word the "two jaws" prose in tests/doc_hidden.rs, tests/foreign_reexport.rs, the surface_coverage.rs module doc (drop "# Tamper-evident totality"; totality is surfacecheck's), and the justfile to name one totality check. `surface-scan` stays for suanpan's claims roster. If the owner wants the stable-toolchain inner-loop check kept, the accurate alternative is to keep the scan retitled as a convenience and delete the two forks entries. Acceptance: with a `pub fn` added to any public type and no roster row, `just surface-totality` reads red naming it; with a `METHOD_SURFACE` row removed it reads red as orphaned; `cargo nextest run -p before --all-features` is green with `SURFACE_SOURCES` gone; no prose under crates/before mentions a second extractor or a pincer.

Synthesis note: surface-roster-31 (this document) applies the same retirement bar to `tools/citecheck`; surface-roster-21 (correctness) is the doc-hidden contradiction that dissolves with the scan.

Catches: A malformed `decided` date, by one of two divergent validators.

Replacement: Drop `decided` from `Exception` and the registry, deleting the `dated` closure and its fixtures; or record the convention once and host one `is_yyyy_mm_dd` in surface-scan.

#### surface-roster-17: Exception rulings carry enforced dates at the declaration site, validated by one of two divergent validators, under a name defined nowhere
- Where: crates/before/surfacecheck/src/check.rs:33-34 (related: crates/before/surfacecheck/src/check.rs:7-8, crates/before/surfacecheck/src/check.rs:259-277, crates/before/surfacecheck/src/check/tests.rs:262-326, crates/before/src/meter/registry.rs:1075-1103, crates/before/src/meter/registry/tests.rs:207-225)
- Class / severity / confidence: scaffolding / low / medium
- Provenance: verified (`grep -rn exemption crates/before --include='*.rs'` hits the phrase only in surfacecheck; the convention's referent exists: registry.rs:1079 and 1095 declare `decided`, 1103 defines `REGISTRY_RATIFIED = "2026-07-29"`, and registry/tests.rs:215 and 221 validate it with `len() == 10 && two dashes`, the shape check.rs's `misdashed` fixture (`"20-26-07xx"`) is written to defeat); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: reframed (a repo-wide owner convention in tension with Principle 5, not a local slip; the phrase is unanchored); history: already-known (d2a9d04e's message reports "the enforced dated-exception machinery (surfacecheck Exception.decided + its YYYY-MM-DD validation and tests, registry Coverage/Bands decided fields + REGISTRY_RATIFIED ...) requires field and test changes to dissolve — a design round"; no ruling recorded since)
- Owner-gated: yes (a repo-wide convention; any change should be uniform across surfacecheck and the meter registry)

Every `Exception` carries a `decided` date; `reconcile_with` validates its `YYYY-MM-DD` shape (positions, digits, month and day ranges) and a `reason.len() < 20` threshold, and two tests with four fixtures pin the validator. The meter registry keeps the same field through one named constant and validates it with a weaker check, so two validators of different strength guard one convention. The module doc attributes the discipline to "the registry exemption-list discipline", a phrase that appears in no other file. Principle 5 as written: dated rationale at a declaration site belongs in git; the `reason` is the positively stated model. This is the open item d2a9d04e reported, restated with the divergence b1403c59 later created.

Evidence:

         7	//! [`crate::census`]. Every exception is named, dated, and reasoned —
         8	//! the registry exemption-list discipline — and every exception and
    ...
        33	    /// The date of the ruling of record (YYYY-MM-DD).
        34	    pub decided: &'static str,

    registry/tests.rs:
       215	                decided.len() == 10 && decided.chars().filter(|&c| c == '-').count() == 2,

Resolution: owner's ruling, applied uniformly. Preferred: drop `decided` from `Exception` and from the registry's `EnvelopeOnly`/`Unbanded`, delete the `dated` closure and the `undated`/`misdashed`/`unmonthed` fixtures, and let `git log -L` carry the when. If dated rulings stay: record the convention once in the project instructions as a deliberate exception to Principle 5, host one `is_yyyy_mm_dd` in surface-scan used by both sites, and replace "the registry exemption-list discipline" with a pointer to that site. Either way, name the `20` (surface-roster-26). Acceptance: `grep -rn decided crates/before/surfacecheck crates/before/src/meter` is empty, or exactly one date validator exists and the convention is recorded where the phrase is defined.

Synthesis note: meter-registry-tier2-9 and meter-adequacy-12 (this document) are the registry side; rule once for both sites.

Catches: Nothing: `ITEM_EXCEPTIONS` has had no inhabitant at any commit, yet owns a parameter, a category, a render block, a `--list` disposition, a census slot, and the item halves of four tests.

Replacement: Delete the list and its supporting code; reintroduce the mechanism with its first reviewed entry.

#### surface-roster-18: `ITEM_EXCEPTIONS` is an empty per-item exception list with a full category, render block, `--list` disposition, census count, and tests behind it
- Where: crates/before/surfacecheck/src/check.rs:37-40 (related: crates/before/surfacecheck/src/check.rs:114-116, crates/before/surfacecheck/src/check.rs:185-189, crates/before/surfacecheck/src/check.rs:228-238, crates/before/surfacecheck/src/check.rs:305-310, crates/before/surfacecheck/src/main.rs:125, crates/before/surfacecheck/src/main.rs:152-153, crates/before/surfacecheck/src/check/tests.rs:129-201)
- Class / severity / confidence: scaffolding / low / medium
- Provenance: verified (read every site; the list is `&[]`); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: reframed (the "empty buffer for known failures" doctrine misapplies, since a reasoned exception is the sanctioned declared-model route; the applicable lens is Principle 3, uninhabited machinery, and the proposed roster-row substitute reaches only fns in `SURFACE_SOURCES` files); history: no-rationale-found (empty since 9aa8c9ac's first run, "zero item exceptions", and never justified)
- Owner-gated: yes (dissolving an instrument's category)

The per-item list has no inhabitant at this or any commit, yet it owns the `item_exceptions` parameter and `excepted_items` set in `reconcile_with`, the `dead_item_exceptions` category and its render block, the `excepted (item)` branch in `render_list`, a slot in the clean-sweep census line, and the item halves of four tests. Its doc says "none at this tip", a statement about the current moment rather than a rule (Principle 5). Principle 3: machinery earns its place by what it serves; this serves nothing yet. Module exceptions differ: each is an inhabited, positively stated model of a whole instrument tree, and stays.

Evidence:

        37	/// Per-item exceptions: none at this tip. An entry here is a deliberate,
        38	/// owner-reviewable ruling that one public function-like item stays off
        39	/// the roster.
        40	pub(crate) const ITEM_EXCEPTIONS: &[Exception] = &[];

Resolution: delete `ITEM_EXCEPTIONS` and its supporting code (the parameter, `excepted_items`, `Findings::dead_item_exceptions` and its render block, the `--list` branch, the census-line count, and the item halves of `exceptions_excuse_their_scope`, `dead_and_shadowing_exceptions_read_red`, `malformed_exceptions_read_red`, `committed_exceptions_are_well_formed`); re-adding the mechanism with its first inhabitant is the reviewed event. If kept, reword the doc without the temporal clause. Acceptance: `grep -rn ITEM_EXCEPTIONS crates/before/surfacecheck` is empty and `reconcile_with` has six parameters; or the doc states the rule with no "at this tip".

### The test harness: bridge, oracles, exhaustive, validation index

Catches: Event-side resolution under `event`'s per-cell bumps and `join` (the function-space legs, the only committed bound); the oracle-depth legs feed no scan.

Replacement: Trim to the function-space legs and rewrite the doc; or convert the keystone's clamp into an assert, retire the sweep, and re-point its two roster citations.

#### testing-oracles-17: `grid_cap_is_never_reached`'s oracle-depth legs feed no scan; its function-space legs are the only committed bound on event-side resolution, which its doc does not say
- Where: crates/before/src/testing/semantic_oracle/tests.rs:551-583 (related: crates/before/src/testing/semantic_oracle/tests.rs:529-549, crates/before/src/testing/semantic_oracle/tests.rs:190-194, crates/before/src/testing/semantic_oracle.rs:75-78, crates/before/src/testing/semantic_oracle.rs:317-346, crates/before/src/testing/semantic_oracle.rs:365-367, crates/before/src/testing/semantic_oracle.rs:378-383, crates/before/src/surface.rs:1222-1223, crates/before/src/testing/surface_coverage.rs:133-136)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (the keystone's grid at 190-194 derives from `se` only, so the `ev_depth`/`id_depth` maxima at 566-569 are read by no scan; `fork`'s assert at 378-383 bounds the id side at bisection only; the chain pin exercises forks only; `event` at 336 sets the ceiling to `e.res_ceiling.max(level)` with `level = id_res(i)`, so event-side resolution is bounded by an argument, not a committed check, and the sweep's `ev_res` legs at 572 are the only assertion on it; the two roster citations are surface.rs:1222-1223 and surface_coverage.rs:135); executed: no
- Seen by: scaffolding, structure-prose; refutation: reframed (retiring the test outright is refuted: a change to `event` bumping at `id_res(i) + 1` would grow `ev_res` per tick while `fork`'s assert and the chain pin stay green; only the oracle-depth legs are unconsumed); history: deliberate-but-expired (needed at 08f7ccab when `GRID_N` was a hand-picked 10 and `fork` clamped; retained at 28f6981e under a restated "distributional" rationale after the clamp became an assert)
- Owner-gated: yes (removal or reshaping of a rostered instrument; two roster citations)

The sweep maxes four quantities per case: the oracle trees' depths (566-569), which no scan consumes, and the function space's probed resolutions (570-573). The doc frames it as a distributional companion to the structural derivation and names a false two-level rate as its reason (testing-oracles-16); the true reason it is not redundant today is that the keystone's silent clamp (testing-oracles-10) plus the absence of any event-side assert leave the `ev_res` legs as the sole guard against an aliased scan. A guard should name the constructible failure it alone catches; this one's doc names a different one.

Evidence:

       566	            for c in &or {
       567	                max_d.fetch_max(ev_depth(&c.version()), AOrd::Relaxed);
       568	                max_d.fetch_max(id_depth(c.party()), AOrd::Relaxed);
       569	            }
       570	            for c in &se {
       571	                max_d.fetch_max(id_res(&c.id), AOrd::Relaxed);
       572	                max_d.fetch_max(ev_res(&c.ev), AOrd::Relaxed);
       573	            }
       578	    assert!(
       579	        observed < GRID_N,
       580	        "op-trace reached resolution {observed} ≥ GRID_N {GRID_N}; raise GRID_N so the scans \
       581	         stay fully faithful",
       582	    );

    semantic_oracle.rs:
       336	    let ceiling = e.res_ceiling.max(level);
       378	        assert!(
       379	            res < GRID_N,

Resolution: Drop the oracle-depth legs (566-569) and rewrite the doc at 529-549 to name what the sweep guards: event-side resolution under `event`'s per-cell bumps and `join`, which `fork`'s assert and the chain pin do not cover. Then the owner's call: keep the trimmed sweep, or convert the keystone's clamp into an assert (`fs_grid` or `assert!(d <= GRID_N)` at 194) so every keystone case asserts both sides, retire the sweep, and re-point the two roster citations (surface.rs:1222-1223 `GridCap { guard }` and `surface_coverage::TRIPWIRES`) to `fork_chain_raises_resolution_one_level_per_fork`, with `citecheck` green. Acceptance: the sweep reads only function-space quantities or is gone; its doc, `GRID_N`'s doc (75-78), and `fork`'s doc (365-367) name the event side or the keystone assert as the guard; both roster citations resolve.

Synthesis note: testing-oracles-10 (this document) is the keystone clamp this entry's retirement branch depends on; the testing-oracles summary's open question 5 recommends that branch (assert in the keystone, retire the sweep).

Catches: Five `crate::laws` predicates restated one for one over the exhaustive corpus, with the ruling that keeps them outside the roster stated only in a decision record.

Replacement: Drive `laws::PARTY_PAIR` and `laws::VERSION_PAIR` totally over the corpus pairs; at minimum, state ruling #75's reason at the site.

#### testing-oracles-25: The five "intrinsic symmetry laws" restate five `crate::laws` predicates one for one, and the ruling that keeps them outside the roster is stated only in the decision record
- Where: crates/before/src/testing/exhaustive/tests.rs:459-540 (related: crates/before/src/testing/exhaustive/tests.rs:47-51, crates/before/src/laws.rs:444-456, crates/before/src/laws.rs:522-526, crates/before/src/laws.rs:2183-2213, crates/before/src/testing/algebraic_laws.rs:11-12, .agent-notes/2026-07-22-before-adversarial-resource-amplification/before-adversarial-resource-amplification.md:717-729)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (read the five bodies against laws.rs: `id_is_disjoint_is_symmetric` is `disjoint_symmetric` (2184-2186); `id_join_is_commutative` is a weaker `join_commutative_outcomes` (2203-2213, which also pins the `Err` hand-back); `event_partial_cmp_is_antisymmetric` is `partial_cmp_is_dual` (524-526), not `order_antisymmetric` (508); `event_merge_is_commutative` is `merge_commutative` (448-450); `event_meet_is_commutative` is `meet_commutative` (454-456); none of the five test names is cited by surface.rs, surface_coverage.rs, or diff_ops.rs; `VERSION_PAIR` holds 32 laws and `PARTY_PAIR` 10, counted by `fn` between the `pub static` headers); executed: no
- Seen by: scaffolding, adequacy; refutation: confirmed (severity lowered: nothing is caught or missed today; adds the cost caveat); history: already-known (ruling #75: "Bespoke law tests dissolved name-for-name; kept deliberately outside: the exhaustive small-scope point laws"; the rationale lives only in the record, and the section comment at 461-465 argues oracle-independence and totality without mentioning the roster)
- Owner-gated: yes (reopens a recorded ruling with a proposal it did not consider)

Two spellings of the same law surface: a law added to laws.rs gets sampled coverage only, and a symmetry retyped here drifts independently (the join copy already omits the `Err` hand-back the law pins). The section's stated payoff, oracle-independent and total, is exactly what iterating `laws::PARTY_PAIR` and `laws::VERSION_PAIR` over the corpus pairs would deliver for every pair law rather than five. Ruling #75 declined folding the point laws into the collection; driving the collection over the exhaustive corpus is a different proposal it did not rule on. Whatever the owner decides, the reason the five stay hand-spelled belongs at the site. One naming nit rides along: the test and the module doc (47-48) call the dual property "anti-symmetric", while laws.rs reserves `order_antisymmetric` for `le(a, b) && le(b, a) => a == b` and names this property `partial_cmp_is_dual`. Cost caveat for the total drive: 32 laws over 691^2 event pairs is roughly 15M law evaluations, so the gate-resident placement needs a measurement, following the leg-split logic exhaustive.rs already applies.

Evidence:

       461	// The op symmetries are intrinsic algebraic properties of the impl, so they are
       462	// tested DIRECTLY on the impl — no oracle, and not folded into the differential
       463	// checks above. Two payoffs: a symmetry bug the oracle happened to *share* is
       464	// still caught here, and (being deterministic + total over the small-scope
       465	// corpus) the guarantee is total, not sampled.
       509	        assert_eq!(
       510	            imp[i].partial_cmp(&imp[j]),
       511	            imp[j].partial_cmp(&imp[i]).map(Ordering::reverse),

    laws.rs:
       524	    fn partial_cmp_is_dual {
       525	        a.partial_cmp(b) == b.partial_cmp(a).map(Ordering::reverse)
       526	    }
      2184	    fn disjoint_symmetric {
      2185	        a.is_disjoint(b) == b.is_disjoint(a)
      2186	    }

    decision record:
       724	  `dangerously_alias`, confined to predicate scope. Bespoke law
       725	  tests dissolved name-for-name; kept deliberately outside: the
       726	  exhaustive small-scope point laws, the population/fold laws

Resolution: At minimum, add the ruling's rationale to the section comment (why these five stay outside the roster). Preferably, replace the five with two total drivers (`for (name, law) in laws::PARTY_PAIR` over `par_for_pairs` on `impl_ids`, and the `VERSION_PAIR` twin over `impl_events`, each asserting `law(&a, &b)` with the law name in the message), measure the wall time, and place them in `exhaustive_small` or the deep tier by that measurement; rename or re-doc the antisymmetry test if it stays. Acceptance: either the section comment states the ruling's reason, or the five tests are gone, every `PARTY_PAIR`/`VERSION_PAIR` law is asserted over the full small corpus, and temporarily inverting one predicate in laws.rs fails the driver naming that law.

### The test harness: differential table, generators, snapshots, asymptotics

Catches: Nothing: `EXEMPTIONS` is an empty acceptance buffer with two validation loops and an "or exempted" arm, the shape the doctrine forbids even empty.

Replacement: Assert `emitted == included` as two sets; reintroduce a roster the day the first reviewed exemption exists.

#### testing-diff-gen-31: `EXEMPTIONS` in `fuelscape_islands.rs` is an empty acceptance buffer
- Where: crates/before/src/testing/fuelscape_islands.rs:16-22 (related: crates/before/src/testing/fuelscape_islands.rs:63-81, crates/before/src/testing/diff_ops.rs:799-801, crates/before/src/testing/diff_ops/tests.rs:122-136, crates/before-fuelscape/src/ops.rs:2173)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (before-fuelscape/src/ops.rs:2173 `pub const EXEMPTIONS` is inhabited with stated reasons, so the idiom's inhabited instance lives there; `git show b5a81583:.../fuelscape_islands.rs:18-24` shows this roster inhabited with one reviewed entry, and `git show 2efff149` emptied it and added the "Currently empty" note); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed; history: deliberate-but-expired (the roster landed inhabited as the fuelscape design note anticipated; the review round removed its one entry and kept the mechanism)
- Owner-gated: no (reintroducing the roster with its first reviewed entry is a one-diff event)

The doctrine names this shape directly: no mechanism for accepting known failures may exist, even as an empty buffer waiting to hold them. The crate applies the same rule to itself at diff_ops.rs:799-800 ("an empty genre is a dead category, dissolved rather than carried"), enforced by `every_bespoke_genre_is_inhabited`. The exemption mechanism, its two validation loops, and the "or exempted" arm exist to hold entries that do not exist, and "Currently empty" is temporal prose.

Evidence:

        16	/// Operations whose islands deliberately appear in no doc comment, each
        17	/// with its reviewed reason.
        18	///
        19	/// Currently empty: every measured operation's island reaches the
        20	/// rendered docs (the operator matrices and the conjunction cells carry
        21	/// theirs through their generating macros).
        22	const EXEMPTIONS: &[(&str, &str)] = &[];

Resolution: Delete `EXEMPTIONS`, the exemption loop (63-72), and the `exempt` set; assert `emitted == included` as two `BTreeSet<String>`s so the messages at 77-80 and 85 become the two set-difference reports. Reintroduce a roster the day the first reviewed exemption exists, with its reason, as the bespoke-genre roster does. Acceptance: the test body compares two sets; no exemption mechanism and no "Currently empty" in the file.

### The envelopes, first half

Catches: Door regressions (a clone rung allocating, an empty-operand misrouting) on the door rows; the scan column and value legs on the kernel rows; both rosters pin identical numbers on identical shapes.

Replacement: One roster of record, the door, with the scan column and value legs ported and the five kernel twins retired; or a stated door cost at every retained kernel row.

#### envelopes-a-8: Five public-door rows and five kernel rows pin identical numbers on identical shapes
- Where: crates/before/tests/meter.rs:263-264 (related: 269-270, 278-279, 1649-1662, 1876-1887, 1745-1849, 1909-2041; crates/before/src/version.rs:889-899, 1729-1733)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (compared the constants: `CMP_DENSE`/`SKYLINE_CMP_DENSE` 30_720; `CMP_CLIFF`/`SKYLINE_CMP_CLIFF` 1_330, 88, 52; `JOIN_DENSE`/`SKYLINE_JOIN_DENSE` 130_277; `JOIN_BIGROOT`/`SKYLINE_JOIN_BIGROOT` 85_060, 1_565, 939; `JOIN_CLIFF`/`SKYLINE_JOIN_CLIFF` 5_362, 308, 184; `partial_cmp` routes to `sweep::causal_cmp` (version.rs:1731) and `|` to `emit::join` (version.rs:899)); executed: no
- Seen by: scaffolding; refutation: reframed (the kernel twins are not "the scan column only": they hold the value legs the door rows lack); history: deliberate-but-expired (the kernel rows were added on 2026-07-23 to measure the skyline candidate beside the packed production coding; the flag day routed every door to the kernels; both rosters survived under the campaign's never-remove-tests constraint, which has lapsed)
- Owner-gated: yes: which roster survives is a design choice, and removal of rows follows the note's dissolution ratchet

Two rosters pin the same surface, born of a constraint (two codings) that no longer exists. Today the door rows can catch door regressions (a clone rung allocating, an empty-operand identity misrouting) but assert nothing about the result; the kernel rows carry the scan column and the verdict or byte-identity legs. Consolidation must move both the scan column and the value leg to whichever side survives.

Evidence:

       263	    pub const CMP_DENSE: Envelope = envelope(30_720, 0, 0, 0); // the iterative sweep over the Bytes-backed at-rest form (OpenedPair states the pair walk's opening move once); word-valued payloads keep the limb column at zero
       264	    pub const JOIN_DENSE: Envelope = envelope(130_277, 0, 0, 0); // the emit kernel's peak alone: the value-operator cell's lhs clone is a refcount bump, not a byte copy of the operand; word-valued payloads keep the limb column at zero

      1652	    pub const SKYLINE_CMP_DENSE: SweepEnvelope = sweep_envelope(30_720, 0, 0, 468_760, 0); // path-bit stacks and one accumulator; word-valued payloads keep the limb column at zero
      1880	    pub const SKYLINE_JOIN_DENSE: SweepEnvelope = sweep_envelope(130_277, 0, 0, 625_018, 0); // the peak is the emitted stream itself; word-valued payloads keep the limb column at zero

Resolution: Decide which side is the roster of record. If the door: give the door rows the scan column (falls out of envelopes-a-4) and the value legs (envelopes-a-9), port the kernel-only shapes (`SKYLINE_CMP_DENSE_SELF`, `SKYLINE_CMP_WIDE_TOOTH`, `SKYLINE_JOIN_ABSORB`, `SKYLINE_JOIN_WIDE_TOOTH`, `SKYLINE_MEET_*`) to `partial_cmp`, `|`, `&`, and retire the five twins with their tests. If the kernel: state at each retained door row the door cost it exists to catch. Acceptance: no two rows pin the same (shape, operation) through a door and its kernel; every retained kernel row's comment names the door cost it deliberately excludes.

### The envelopes, second half

Catches: It claims to price "the board's scatter cells" but measures a hand copy of the board's private scatter constructor, which already differs at non-power-of-two scales.

Replacement: A registry door to the board's scatter builder, called from both the board and `tests/meter.rs`.

#### envelopes-b-17: `scatter_population` re-implements the board's private scatter constructor
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

### Other suites

Catches: Rostered kernel and twin renames, hidden and foreign surface, and band names, through five divergent scanners: one misses `pub fn` and cannot see `#[ignore]`, another omits `wasm32-pins/`.

Replacement: One `tests/support/source_scan.rs` built on `surface_scan::test_fns`, included by `#[path]` from the five binaries.

#### tests-other-3: Five hand-rolled source scanners re-implement `surface_scan::test_fns` with divergent rules and universes
- Where: crates/before/tests/amp_board_smoke.rs:314-340 (related: crates/before/tests/superlinear_tripwires.rs:14-16, crates/before/tests/superlinear_tripwires.rs:72-98, crates/before/tests/superlinear_tripwires.rs:112-113, crates/before/tests/verdict_matrix.rs:1232-1287, crates/before/tests/verdict_matrix.rs:1438-1448, crates/before/tests/doc_hidden.rs:25-43, crates/before/tests/foreign_reexport.rs:62-99, crates/surface-scan/src/lib.rs:174-196, crates/before/Cargo.toml:50)
- Class / severity / confidence: scaffolding / medium / high
- Provenance: verified (read surface-scan/src/lib.rs:174-196; `surface-scan` is a dev-dependency at before/Cargo.toml:50 with consumers in surface_coverage.rs and suanpan's claims tests, none under crates/before/tests; `find crates/before/wasm32-pins -name '*.rs'` returns three files absent from the seven-entry tree list; Python replica of superlinear_tripwires.rs:80-87 returns None for `pub fn x_reads_superlinear_on_y()` and matches a non-test `fn helper_reads_superlinear_kernel()`); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed (with the reframe that the two rosters look for different markers, so the divergence is of universes: a `_reads_superlinear` fn in `benches/` or `examples/` and a `_reads_inverted` fn in `wasm32-pins/` are each invisible to their roster); history: no rationale found (fcb78e44's qualifier-aware, `target`-skipping fix landed in verdict_matrix.rs only; the seven-tree list was complete on 2026-08-13 and rotted on 2026-08-18 when eb6ba627 added wasm32-pins)
- Owner-gated: no

`band_test_names` is line-for-line `surface_scan::test_fns` plus a name filter, and four further recursive directory walkers with three different "what is a declared fn" rules live in this partition. The divergence has a live cost: superlinear_tripwires.rs's doc promises to match "the `#[test]` fns" but its scanner matches any bare `fn ` line, misses `pub fn`, and counts a non-test helper; verdict_matrix.rs's tree list claims "Every source tree the crate carries" and omits `wasm32-pins/`. Principle 3: infrastructure that reimplements a capability the workspace already owns and generates its own maintenance cascade (every fix landed in one copy).

Evidence:

       314	fn band_test_names(source: &str) -> BTreeSet<String> {
       315	    let mut names = BTreeSet::new();
       316	    let mut armed = false;
       317	    for line in source.lines() {
       318	        let t = line.trim();
       319	        if t == "#[test]" {

    (superlinear_tripwires.rs:14-16)
        14	//! decoration. This roster is the missing jaw: the committed list below
        15	//! must match the `#[test]` fns whose names carry `_reads_superlinear`,
        16	//! in both directions.

    (superlinear_tripwires.rs:80)
        80	                let Some(rest) = line.trim_start().strip_prefix("fn ") else {

    (verdict_matrix.rs:1438-1448)
      1438	    // Every source tree the crate carries: a twin declared in a side
      1439	    // tree must not escape the roster.
      1440	    for tree in [
      1441	        "src",
      1442	        "tests",
      1443	        "benches",
      1444	        "examples",
      1445	        "fuzz",
      1446	        "fuzzfit",
      1447	        "surfacecheck",
      1448	    ] {

Resolution: Add `tests/support/source_scan.rs` (the `#[path]` precedent `fuzz_seed_set.rs` sets) exposing one recursive walker over the crate root that skips any `target` directory (so the tree set is derived, not listed) and one fn-item extractor built on `surface_scan::test_fns` for attribute-gated names plus verdict_matrix's qualifier-aware `fn_name` for declaration lines; port the five callers and delete the local copies and the seven-entry list. Acceptance: `grep -n 'fn scan(' crates/before/tests` returns nothing outside `support/`; renaming any rostered kernel or twin still reads red; a `pub fn x_reads_superlinear_on_y` is rostered; `wasm32-pins/harness/tests/pins.rs` is scanned by the inverted-twin roster; superlinear_tripwires.rs's doc is true of its mechanism.

Synthesis note: suite-economics-3 and surface-roster-10 (this document) are the same scanner family from two more vantages; surface-roster-10 adds the in-crate `declared_test_names_by_file` and its richer rule. The shared walker should adopt that rule so `surface_scan::test_fns` becomes the one scanner.

### Fuzz targets, guests, and pins

Catches: Nothing `fuzz_decode_differential`'s borsh arm does not already assert on the same inputs (byte identity, re-decode and re-encode stability).

Replacement: Fold the sixteen seeds into the differential target; or keep the target with one sentence naming its unique payload (raw-door throughput, transport-feature independence).

#### fuzz-guests-pins-4: `fuzz_decode`'s assertions are implied by `fuzz_decode_differential`'s borsh arm
- Where: crates/before/fuzz/fuzz_targets/fuzz_decode.rs:14-17 (related: crates/before/fuzz/fuzz_targets/fuzz_decode_differential.rs:59-64, crates/before/fuzz/fuzz_targets/fuzz_decode_differential.rs:254-306, crates/before/tests/support/fuzz_seed_set.rs:29-35)
- Class / severity / confidence: scaffolding / low / medium
- Provenance: assessed (read `borsh_vs_raw`'s three branches against `fuzz_decode::run`); executed: no
- Seen by: scaffolding [5]; refutation: confirmed; history: no-rationale-found (778c89f50 introduced the differential target without discussing retention)
- Owner-gated: yes: removal of an instrument

For each of the six wire types, `borsh_vs_raw` runs the raw whole-slice `decode(data)` in every branch: when borsh accepts the whole slice it asserts `encode(&raw) == encode(&value) == consumed == data` (fuzz_decode's byte identity); when borsh accepts a proper prefix or rejects, it asserts the raw decode rejects, so fuzz_decode's `if let Ok` cannot fire. The re-decode and re-encode-stability checks follow from decode determinism on identical bytes. fuzz_decode therefore catches nothing the differential target does not on the same input distribution; the plausible retention reasons (executions per second on the raw doors, a coverage map without borsh, independence from the transport features) hold but are unstated. Circular justification is the tell: an instrument names what it alone catches.

Evidence:

    14	//! The roster is every public wire type: `Party`, `Version`, `Clock`, `Rank`,
    15	//! `Ranked`, and `Span`. The composite decodes (`Ranked`, `Span`) are additionally
    16	//! held to their composed counterparts — on rejection genre, not just accept — by
    17	//! the sibling `fuzz_decode_differential` target.

Resolution: Either keep the target with one sentence in its module doc naming the payload it alone provides, or dissolve it into the differential target, re-homing its 16 committed seeds (including the two non-derivable frontier witnesses) under `seeds/fuzz_decode_differential` and updating `fuzz_seed_set.rs`. Acceptance: the target's doc names its unique payload, or the target is gone with `tests/fuzz_seeds.rs` green.

Synthesis note: The fuzz-guests-pins summary's open question 4 recommends the keep-with-one-sentence branch (raw-door throughput and transport-feature independence are payloads); this entry accepts either.

### Fuzz-fit: bands

Catches: Nothing: the splice marker, the column-zero template, and the duplicated rustdoc exist to manage generated data sharing a hand-edited file.

Replacement: Calibrate emits array bodies into `include!`d data files; the constant declarations and their rustdoc stay in `bands.rs`.

#### fuzzfit-bands-18: Generated data and hand-written prose share one file, which is what the splice marker, the column-zero template, and the duplicated rustdoc exist to manage
- Where: crates/before/fuzzfit/harness/src/bin/calibrate.rs:345-358 (related: calibrate.rs:359-403, crates/before/fuzzfit/harness/src/bands.rs:312-327, bands.rs:918-926, bands.rs:977-989)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (the scaffolding lens diffed bands.rs:312-327, 918-926, 977-989 against the template at calibrate.rs:360-375, 378-386, 389-401 with the marker and rustc substitutions applied and found all three byte-identical; I re-read both copies); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: deliberate-but-expired (co-location served the in-doc dated movement ledger, which d2a9d04e retired in favour of the re-pin commit message; the marker guard 4b71d569 and the column-zero literal 27ace7a2 are patches on the shared-file design)
- Owner-gated: no

Because calibrate rewrites the tail of `bands.rs` in place, it must locate a prose marker by string, guard its absence, format a template whose continuation lines sit at column zero to satisfy the doc-summary linter, and carry the rustdoc for `PINNED_RUSTC`, `BANDS`, `SMALL_BANDS`, and `REFIT_COVERAGE` as string literals that duplicate the same prose in `bands.rs`; a doc edit in either place is lost or ineffective until the other is edited too. Principle 3: infrastructure that generates its own maintenance cascade is suspect, and the constraint that justified it has expired.

Evidence:

       345	    let bands_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bands.rs");
       346	    let current = std::fs::read_to_string(&bands_path).expect("bands.rs exists");
       347	    let marker = "/// The toolchain that pinned";
       348	    // The marker line is prose in a generated region: a rewording that
       349	    // loses it must fail here by name, never silently splice the whole
       350	    // file into the head.
       351	    let head = &current[..current
       352	        .find(marker)
       353	        .expect("bands.rs splice marker present")];
       354	    let rustc = env!("FUZZFIT_RUSTC_VERSION");
       355	    // A plain multi-line literal (continuation lines at column zero): the
       356	    // emitted `///` lines keep their paragraph structure both in the
       357	    // written file and here in the source, where the doc-summary linter
       358	    // reads them too.

Resolution: Keep the constant declarations and their rustdoc in `bands.rs` and have calibrate emit only the array bodies and the rustc literal into data files (`src/bands/pinned_bands.rs`, `pinned_small_bands.rs`, `pinned_refit_coverage.rs`, `pinned_rustc.rs`, each `include!`d as an expression, since `include!` cannot carry item docs), plus the `PIN_EVIDENCE` value from finding 2. The marker search, its expect, the column-zero literal, the template prose, and the whole-file rewrite of a hand-edited file all dissolve; calibrate's row rendering becomes one `fn` over `Band`. Acceptance: calibrate writes no file containing `///` lines; `bands.rs` is never rewritten by a tool; `just fuzzfit-calibrate` on unchanged code yields no diff; `just fuzzfit`'s fmt and clippy legs stay clean.

### Fuelscape: render

Catches: Nothing: the `overlay` field is validated, tamper-tested, and committed for a widget revision that does not exist, at a sixth of the dataset's bytes.

Replacement: Drop the field, bump `FORMAT_VERSION`, re-derive with `just fuelscape-compact`; or restate the doc in the present tense with the measured cost.

#### fuelscape-render-10: `overlay` is compacted, validated, and committed for a widget revision that does not exist, at a sixth of the dataset's bytes
- Where: crates/before-fuelscape/src/compact.rs:146-148 (related: crates/before-fuelscape/src/compact.rs:21-22, :226, :404-410; crates/before-fuelscape/src/compact/tests.rs:183; crates/before/build.rs:212-222; .agent-notes/2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md:93-96, :445-446)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (python over the 104 committed documents: overlay arrays are 216,579 of 1,334,551 bytes compact-serialized, 16.2%, about 2.1 KB per operation; grep finds no `overlay` in fuelscape.js and build.rs's island payload omits it); executed: yes (the byte share)
- Seen by: scaffolding [5], structure-prose [49]; refutation: confirmed; history: already-known (a recorded decision in the design note, whose "~1 KB/op" estimate is off by about 2x)
- Owner-gated: yes (a documented design decision in the note)

Circular justification is the tell: the field's stated purpose is to let a later revision draw family marks without a format bump or re-compaction, but re-compaction is the designed one-minute pure derivation (`just fuelscape-compact`) and a bump is a constant increment; meanwhile the field costs a validation branch, a tamper case, and 16% of the committed bytes, and its doc speaks of "later widget revisions" and "the first widget release" instead of the present fact.

Evidence:

       146	    /// The committed adversarial-family points, carried through for
       147	    /// later widget revisions (the first widget release draws none).
       148	    pub overlay: Vec<OverlayData>,

Resolution: either drop `overlay` from `WidgetOp` (with its validate branch and tamper case), bump `FORMAT_VERSION`, and re-derive with `just fuelscape-compact`; or keep it and restate the doc in the present tense with the measured cost ("the widget does not draw these; carrying them, about 2 KB per operation, lets a widget that does read the same document"). Acceptance: `du` of crates/before/fuelscape drops by roughly 215 KB and `just fuelscape-verify` is clean; or the field doc states the present fact and the measured share.

### Dependencies

Catches: A seam or bench querying a cfg missing from the check-cfg roster (loud); a recipe arm no seam queries or a roster value nothing reads is silent.

Replacement: A run-time provenance assertion in the bench (`BEFORE_ALLOC_AB_ARM`), leaving check-cfg as the sole roster kept in sync with the seams by `deny`.

#### deps-10: the alloc-A/B arm roster lives in three hand-synced places, and the check-cfg comment's "cannot drift" holds in one direction only
- Where: crates/before/Cargo.toml:97-108 (related: justfile:792-809, crates/before/benches/common/mod.rs:259-281, crates/before/src/version/skyline/query.rs:513-515 and 608, crates/before/src/version/skyline/text.rs:351-353)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (read all four sites; grep of `before_alloc_ab` in src and benches; rustc book check-cfg page fetched: "The command line `--cfg` arguments are currently NOT checked but may very well be checked in the future."); executed: no
- Verification: confirmed; history: no-rationale-found (the Cargo.toml comment acknowledges the hole and delegates it to the recipe's hand-written list)
- Owner-gated: no

`unexpected_cfgs = deny` fails a seam or bench that queries a value missing
from the check-cfg roster (roster too small is loud). A roster value no seam
queries, or a recipe arm no roster lists, is silent, because rustc does not
check command-line `--cfg` values. The recipe closes that hole with a
hand-written case list that duplicates the Cargo.toml values, which duplicate
the seams; `alloc_arms()` in the bench is a fourth spelling. An arm added to
the recipe's list without a seam passes the case check, sets a cfg nothing
queries, and saves a baseline labeled `<target>-<arm>` over shipped code
while `alloc_arms()` prints "shipped": the mislabel the comment names.

Evidence:

    crates/before/Cargo.toml
       102	# the build of any seam or bench that queries an arm missing from this
       103	# roster, so the roster cannot drift from the code. (It cannot see a
       104	# mistyped RUSTFLAGS value nothing queries; the `bench-alloc-ab` recipe
       105	# validates the arm name at invocation for exactly that hole.)
       106	unexpected_cfgs = { level = "deny", check-cfg = [
       107	    'cfg(before_alloc_ab, values("projection_growth", "projection_shrink", "display_growth"))',
       108	] }
    justfile
       808	    @case "{{ arm }}" in (shipped|projection_growth|projection_shrink|display_growth) ;; (*) echo 'bench-alloc-ab: unknown arm "{{ arm }}"' >&2; exit 2;; esac
    crates/before/benches/common/mod.rs
       268	    if cfg!(before_alloc_ab = "projection_growth") {
       269	        arms.push("projection_growth");
       270	    }
       271	    if cfg!(before_alloc_ab = "projection_shrink") {
       272	        arms.push("projection_shrink");
       273	    }
       274	    if cfg!(before_alloc_ab = "display_growth") {
       275	        arms.push("display_growth");
       276	    }

No hand-maintained enumerations: four restatements of one roster, with the
silent direction unguarded. The seams are the ground truth.

Resolution: replace the recipe's case list with a run-time provenance
assertion in the bench: the recipe exports `BEFORE_ALLOC_AB_ARM={{ arm }}`
beside RUSTFLAGS, and a small wrapper called at bench start panics when the
requested arm is not the compiled one (`alloc_arms()` stays as the label
printer). check-cfg then remains the sole roster, kept in sync with the seams
by `deny`. Acceptance: `just bench-alloc-ab presize nosuch_arm` fails before
any measurement, naming the compiled arm; adding a fourth arm requires
touching only a seam and the check-cfg roster.

Synthesis note: skyline-coding-28 (this document) retires the `display_growth` arm; if both A/B arms go, this roster dissolves with them.

### Inventory

Catches: Ledger invariants `ledger_invariants_hold_exhaustively` already asserts after every step; the `Sub for Base` assert duplicates the backend's underflow panic and performs metered work in dev-profile envelopes.

Replacement: The O(1) conjunct or nothing in `add_at` and `enter_digit_engine`; delete the `Sub` assert and re-pin any limb envelope that moves.

#### inventory-4: O(n) debug asserts recompute invariants the exhaustive ledger suite already checks, and one of them is limb-metered
- Where: crates/suanpan/src/accumulator.rs:1391-1395 (related: crates/suanpan/src/accumulator.rs:1293-1296 and 1356-1368, crates/suanpan/src/accumulator/tests/ledger.rs:31-60 and 227-239, crates/before/src/codec/base.rs:416-424 and 284-289, justfile:866-871)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (read accumulator.rs's `add_at` and `enter_digit_engine`, ledger.rs's `assert_ledger_invariants` and the exhaustive driver, base.rs's `Sub` and `Ord` impls, and the justfile's board recipe comment); executed: no
- Verification: confirmed, with one sharpening at base.rs:421; history: deliberate (the comment at 1356-1368 argues the assert guards future exits added inside the carry loop) but the exhaustive driver checks the same clauses after every step of every schedule, which is the doctrine's condition for retiring a recompute assert
- Owner-gated: no

`add_at` ends with a scan of every digit above `top`, and `enter_digit_engine`
scans the whole buffer; both restate clauses the committed exhaustive driver
`ledger_invariants_hold_exhaustively` asserts after each step (ledger.rs:34,
52, 57). They make each write O(held digits) in dev-profile builds. The
sibling site in `before`, `debug_assert!(self >= *rhs)` in `Sub for Base`,
duplicates the backend's own underflow panic and, because `Ord for Base`
records limbs (base.rs:286), it is itself metered work: `just test-all` runs
`tests/meter.rs` in the dev profile, so every pinned limb envelope over a
subtraction-heavy scenario counts the assert's compare as well as the
subtraction. The justfile already names this hazard for the board (release is
the profile of record because "debug assertions perform metered work (Base
comparisons through the limb shim...)"), but the integration envelopes are
pinned under exactly that scaffolding; removing the assert later would move
them.

Evidence:

      1391	        debug_assert!(
      1392	            (self.top == 0 || self.digits[self.top] != 0)
      1393	                && self.digits[self.top + 1..].iter().all(|&digit| digit == 0),
      1394	            "top must rest on the highest nonzero digit at add_at exit"
      1395	        );

      1293	        debug_assert!(
      1294	            self.digits.iter().all(|&digit| digit == 0),
      1295	            "a retired register leaves the digit engine idle"
      1296	        );

    ledger.rs:
        51	    assert!(
        52	        acc.digits[acc.top + 1..].iter().all(|&digit| digit == 0),
        53	        "nonzero digit above top {} after {schedule:?}",
        54	        acc.top
        55	    );
        56	    assert!(
        57	        acc.top == 0 || acc.digits[acc.top] != 0,
        58	        "top {} rests on a zero digit after {schedule:?}",
        59	        acc.top
        60	    );

    base.rs:
       419	    fn sub(self, rhs: &Base) -> Base {
       420	        meter_limbs2(&self, rhs);
       421	        debug_assert!(self >= *rhs, "Base subtraction underflow");
       422	        Base(self.0 - &rhs.0)
       423	    }

       284	impl Ord for Base {
       285	    fn cmp(&self, other: &Self) -> Ordering {
       286	        meter_limbs2(self, other);

    justfile:
       866	# The board runs at the release profile, the profile of record: debug
       867	# assertions perform metered work (Base comparisons through the limb shim,
       868	# metered probe cursors), so a dev board measures algorithm plus
       869	# verification scaffolding while release measures the production work
       870	# alone. A dev run (`cargo run -p before --example amp_board ...`) remains
       871	# a legitimate debugging view; its numbers must never be pinned anywhere.

Resolution: drop the two O(n) accumulator asserts, or reduce each to its O(1)
clause (`self.top == 0 || self.digits[self.top] != 0`) if a local probe is
wanted; drop the `Sub for Base` assert (the backend panics on underflow with
its own message). Any limb envelope in `tests/meter.rs` that moves is a
measured improvement to re-pin, attributed to the assert's removal.
Acceptance: the exhaustive ledger driver still passes; no dev-profile assert
performs a metered `Base` comparison.

Synthesis note: suanpan-21 and codec-base-text-tree-7 (this document) carry the two asserts individually with their re-pin obligations.

### Gate legs

Catches: Two rosters carry a class for accepting known failures: covcheck's `remediation` (empty) and benchjudge's `red` (holding only the schoolbook tripwire, a required red).

Replacement: Drop `remediation`; redefine `red` as the required-red tripwire set so an unexpected red has two exits, a cure or a declared model.

#### gate-legs-9: Two rosters carry a class for accepting known failures (one now empty, one holding only a demonstration)
- Where: tools/covcheck:11-15 (related: tools/covcheck:77, justfile:1006-1009, tools/benchjudge:65-69, tools/benchjudge-expected.json:12-14, crates/before/tests/bench_judge_roster.rs:57-59)
- Class / severity / confidence: scaffolding / low / medium
- Provenance: verified (read tools/covcheck:1-100; disposition tally over tools/covcheck-expected.json: 26 panic-arm, 8 unreachable, 0 remediation; read benchjudge:55-95 and benchjudge-expected.json; `git log -S'remediation' -- tools/covcheck-expected.json` shows remediation entries existed and were drained in aa7c96a0 and f77011e3 on 2026-08-12); executed: no
- Verification: confirmed; history: deliberate-but-expired (the class carried entries for one day and has been empty since; the justfile's GOAL statement still names it)
- Owner-gated: yes: the class is a design choice stated at justfile:1006-1009, and the doctrine it conflicts with is the owner's

covcheck admits a "remediation" disposition for a reachable-but-unexercised kernel line, and benchjudge describes its `red` class as where "owned reds await their cures". No covcheck entry uses "remediation" today and the only rostered red is the schoolbook tripwire, a required red that demonstrates the judge is alive rather than an accepted failure, so both are empty buffers for known failures, which the doctrine says may not exist even empty.

Evidence:

        14	invariant it rests on), and "remediation" (reachable but unexercised: a
        15	named open population gap, never a blessed exception).
        77	    dispositions = {"panic-arm", "unreachable", "remediation"}

    justfile
      1008	# panic-arm or an unreachable arm, its argument stated at the entry) or a
      1009	# named remediation item, and any NEW hole fails by name. MECHANISM:

    tools/benchjudge
        67	makes the judge enforce a *fixed* verdict
        68	map instead of all-green, so `just all` stays meaningful while owned reds
        69	await their cures. The file pins the configuration it was recorded under

    tools/benchjudge-expected.json
        12	  "red": [
        13	    "display_schoolbook/hugeleaf"
        14	  ]

Resolution: Drop "remediation" from covcheck's disposition set and from the justfile's GOAL sentence (a reachable uncovered kernel line then blocks until a directed test lands or it is curated as panic-arm or unreachable). Redefine benchjudge's `red` class as the required-red known-bad tripwires and strike the "owned reds await their cures" sentence, so an unexpected red has exactly two exits: a cure, or a sidecar-declared model. Acceptance: covcheck's self-test pins that "remediation" is refused, and benchjudge's docstring and the roster notes describe `red` only as the liveness tripwire set.

Synthesis note: meter-adequacy-8 (this document) is the benchjudge half of the same class from the adequacy sweep.

Catches: doclint sweeps in-tree build outputs; two detached workspaces carry `.cargo/config.toml` redirects whose only stated purpose is to route around it, and three do not.

Replacement: Exclude `target` in doclint's walk with a self-test fixture; dissolve or re-justify the two redirects.

#### gate-legs-10: doclint sweeps in-tree build outputs; only two of five detached workspaces redirect their target dirs to dodge it
- Where: tools/doclint:370-373 (related: tools/testdoc:19, crates/before/fuzzfit/.cargo/config.toml:1-7, crates/before-fuelscape/.cargo/config.toml:1-7, justfile:181, justfile:365-368, justfile:629-631, justfile:940-945)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (read doclint:370-373 and testdoc:19; `find crates/before/fuzz/target -name '*.rs'` lists two generated thiserror `private.rs` files; `crates/before/surfacecheck/target` and `crates/before/wasm32-pins/target` exist in-tree with no `.rs` today; `ls` shows `.cargo/config.toml` only under fuzzfit and before-fuelscape); executed: no
- Verification: confirmed; history: deliberate-and-holds for the two redirects (their comments state the doclint reason), no-rationale-found for the asymmetry
- Owner-gated: no

doclint's walk has no `target` exclusion (testdoc's does) and the gate runs it over `crates`, so generated sources under the fuzz workspace's in-tree target are linted as if committed, and surfacecheck and wasm32-pins build in-tree too. fuzzfit and fuelscape carry a `.cargo/config.toml` whose only stated purpose is to route around this; the other three detached workspaces do not, so a dependency bump that emits a long doc summary into an in-tree OUT_DIR turns doclint red on code the tree does not own.

Evidence:

       370	def rust_files(root):
       371	    if root.is_file():
       372	        return [root] if root.suffix == ".rs" else []
       373	    return sorted(root.rglob("*.rs"))

    tools/testdoc
        19	IGNORED_DIRECTORIES = {".git", "node_modules", "target"}

    crates/before/fuzzfit/.cargo/config.toml
         1	# Build into the repo root's target/ (its own subdirectory) instead of a
         2	# workspace-local target dir: the gate's doclint sweeps every .rs under
         3	# crates/, and wasmtime's build scripts generate rustdoc'd sources that
         4	# would otherwise land inside this tree and fail it. The repo root target/
         5	# is outside every gate sweep.

    find crates/before/fuzz/target -name '*.rs'
    crates/before/fuzz/target/aarch64-apple-darwin/release/build/thiserror-c36e32fcf4d5360f/out/private.rs
    crates/before/fuzz/target/debug/build/thiserror-c60931e4e8dce25d/out/private.rs

Resolution: Exclude `target`, `.git`, and `node_modules` in doclint's walk exactly as testdoc does, with a self-test fixture for the skip; then the two `.cargo/config.toml` redirects lose their stated reason and can be dissolved or re-justified at the file. Acceptance: doclint's self-test pins that a `.rs` under a `target/` directory is not visited, and no `.cargo/config.toml` cites doclint as its reason.

### Meter adequacy

Catches: A roster class documented as a queue of owned reds awaiting cures, which today holds exactly one designed known-bad kernel.

Replacement: Rename the class `tripwire`, or route the schoolbook cell through `--expect-red`; strike the queue prose.

#### meter-adequacy-8: The bench roster's `red` class is documented as a buffer for owned reds awaiting cures, a shape the roster outgrew
- Where: tools/benchjudge:65-69 (related: tools/benchjudge-expected.json:2, crates/before/tests/bench_judge_roster.rs:45-63, crates/before/tests/bench_judge_roster.rs:47, crates/before/src/meter/board/export.rs:117, crates/before/src/meter/board.rs:174-178)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (read of the docstring, roster notes, and membership pin; `git log -p --follow` on the roster shows the red set once held fifteen bigroot sweeps, the hugeleaf display pair, and `version_distance/jump-pair`, all removed by c95230c8 and ef8894ac; the docstring's phrase dates from b49424614, when those reds were real); executed: no
- Verification: confirmed; history: deliberate-but-expired (the class was built to hold standing reds while cures landed; every cure has landed and the roster's own notes and a066a8e9 now describe the survivors as declared models, yet the judge's docstring still frames the class as a queue). Also a ghost name: bench_judge_roster.rs:47 says "the board-red riders" while the constant is `BOARD_DECLARED_BENCH_RIDERS` (renamed per a066a8e9, "the rider const's new name")
- Owner-gated: no

The board side says red means untriaged and nothing else. The judge's
roster mode is the one place in the apparatus where a RED verdict is
recorded as expected, and its docstring says the mechanism exists "so `just
all` stays meaningful while owned reds await their cures". Today the class
holds exactly the schoolbook tripwire, a designed known-bad kernel, and the
membership pin is what keeps the class from becoming a queue again.

Evidence:

    tools/benchjudge
        65	Roster mode (`--roster FILE`): the committed expected-verdict roster —
        66	the board-side sibling of the gate's expected-failure test roster, same
        67	membership-by-name philosophy — makes the judge enforce a *fixed* verdict
        68	map instead of all-green, so `just all` stays meaningful while owned reds
        69	await their cures. The file pins the configuration it was recorded under

    crates/before/tests/bench_judge_roster.rs
        47	/// Every other cell — the designed diagonal, the board-red riders, and

Resolution: rename the class to what it holds (`tripwire`), or route the
schoolbook cell through the existing `--expect-red` path in its own
invocation so the roster needs no red class; re-state benchjudge:65-69 and
the roster notes so no text describes reds awaiting cures; fix "board-red
riders" at bench_judge_roster.rs:47 to the constant's name. The self-test's
laundering pins (benchjudge:857-884) hold under either name. Acceptance:
no prose in the judge or roster describes a queue of expected failures.

Synthesis note: gate-legs-9 (this document) pairs this with covcheck's empty `remediation` disposition.

### Workspace tools (tools/)

Catches: The bench judge's stated unique class, work no counter column can see, is also claimed by the validation index's fuzz-fit row, and the committed tripwire (`benches/tripwire.rs`) is a machine-word quadratic that wasmtime fuel reads red too, so nothing committed demonstrates a class only wall time sees.

Replacement: Either a time-only tripwire (work fuel prices as one instruction: bulk-memory operations, native codegen, memory hierarchy) that reads RED under the judge and GREEN under fuzz-fit, with the index restating the class; or retirement of the leg, its roster, `tests/bench_judge_roster.rs`, `tripwire.rs`, the sidecar's `Ceiling` machinery, and the two recipes, after fuzz-fit demonstrably reads the tripwire's shape above band.

#### tools-5: The bench judge's stated unique class is also priced by fuzz-fit fuel, and its committed tripwire is a fuel-visible quadratic
- Where: tools/benchjudge:5-7 (related: crates/before/src/testing/validation_index.rs:12-13, 113-116, 125-128; crates/before/benches/tripwire.rs:4-12, 42-52; crates/before/fuzzfit/harness/src/lib.rs:4-13; crates/before/fuzzfit/harness/src/ops.rs:17-19; crates/before/fuzzfit/harness/tests/enforce.rs:25, 263; justfile:467, 1003; .github/workflows/ci.yml:114-120)
- Class / severity / confidence: scaffolding / medium / medium
- Provenance: assessed (cross-read the sites named above); executed: no
- Seen by: scaffolding [2]; refutation: reframed (the two index rows do overlap, and the tripwire's kernel is pure machine-word arithmetic that fuel counts; but the premise that fuel prices every time-visible class is wrong: wasmtime charges fuel per executed wasm instruction, so bulk-memory instructions cost one unit regardless of bytes moved, and native codegen and memory-hierarchy effects are wall-time-only, assessed from wasmtime's documented fuel semantics and not verified here); history: deliberate-but-expired (the judge's "only witness" rationale, 57fe866d on 07-24, predates fuzzfit, 8cfd3c92 on 07-26, whose design note claims the judge's class explicitly; the index, written after both, re-states both rows without a separating input; no recorded decision revisits keeping the judge after fuel)
- Owner-gated: yes (restating or retiring an instrument; the leg was created by owner decision 54b68b4f)
- Cross-references: meter-adequacy-4 and tools-7 (verification) and tools-6 (verification) concern the judge's roster mode, which exit (b) retires; benches-examples-15 and tools-11 (verification) its self-test; tools-12 (documentation) its roster notes.

The index's bar for an instrument is "a failure class no row below already catches, named as a constructible input" (validation_index.rs:12-13). The judge's row names "backend multiplication below the limb shim, container bookkeeping between metered primitives"; the fuzz-fit row names "total-cost drift that escapes the metered currencies while still costing instructions". Fuel counts both. The committed live demonstration (benches/tripwire.rs) says it "catches what no deterministic meter can", but its kernel is a machine-word double loop that fuel reads quadratic too, so nothing committed demonstrates a class only wall time sees. Fuzz-fit runs in the gate (justfile:467); the judge runs only at `just all` (justfile:1003), on the one nondeterministic currency, behind a quiet-machine precondition CI cannot meet (ci.yml:114-116). Principle 3: two rows claiming one class is the circular-justification tell.

Evidence:

     5	counters and liveness floors only, so its output is byte-identical under
     6	any machine load. The implementation-agnostic witness for *time* — work in
     7	plain machine words that no counter column can see — is judged here

    (crates/before/src/testing/validation_index.rs)
   113	//! pinned by `tests/bench_judge_roster.rs`). What it alone catches:
   114	//! **work invisible to every deterministic counter** — cost in layers
   115	//! the meters do not instrument (backend multiplication below the limb
   116	//! shim, container bookkeeping between metered primitives). It is the
   ...
   127	//! chosen-family instrument above — and total-cost drift that escapes
   128	//! the metered currencies while still costing instructions. Its bands

    (crates/before/benches/tripwire.rs)
     4	//! One bench, `tripwire_unmetered_quadratic/quadratic`: a quadratic pass of
     5	//! plain machine-word arithmetic — no allocation, no recursion, no
     6	//! big-integer ops, no stream reads — so every counter column the
   ...
    12	//! judge's leg catches what no deterministic meter can. The same shape is

Resolution: owner decision, two exits. (a) Keep the leg: restate its unique class as non-instruction cost at the board's families and acceptance scale (bulk-memory operations that fuel prices as one instruction, native codegen, memory hierarchy), and commit a tripwire that demonstrates it, reading RED under the judge and GREEN under fuzz-fit's bands; if no such kernel can be built at the board's byte scales, that failure is the evidence for (b). (b) Retire the leg after committing a fuzz-fit demonstration that tripwire.rs's shape reads above-band under fuel, then remove benchjudge, its roster, tests/bench_judge_roster.rs, tripwire.rs, the sidecar's `Ceiling` machinery, and the two recipes, and let the fuzz-fit row absorb the class. Either way the index's two rows stop claiming one class, and tripwire.rs:12 stops saying "what no deterministic meter can". Acceptance: either a committed time-only tripwire fails `just bench-judge-tripwire` and is banded green by `just fuzzfit`, with validation_index.rs stating that class; or the leg is gone and `just fuzzfit` demonstrably reads the retired tripwire's shape red.
Construction: for (b), transplant `unmetered_quadratic` (tripwire.rs:42-52) into the fuzz-fit guest as a banded kernel and run `just fuzzfit`: it reads above-band, so the judge catches nothing the bands do not. For (a), build a kernel whose wasm lowering is a single `memory.copy`/`memory.fill` per step over a growing buffer (instruction count linear in steps, bytes moved quadratic) and show RED under the judge and GREEN under fuel.

Synthesis note: the excerpt's line numbers and the Where anchor were corrected at merge time: the quoted docstring lines sit at tools/benchjudge:5-7 at `9e5784fb` (the partition report numbered them 6-8); the text is verbatim.

Catches: The renderer-vocabulary contract of the wire-capture corpus, which the byte-pinned insta snapshots in `tests/gossip_snapshot.rs` already hold; the TOTAL ratio the tool prints has no committed consumer since the design note that read it moved to `.agent-notes/`.

Replacement: Move `digestshare` out of `gate-lints` and `ci` into the conveniences, keeping the tool (with tools-19's floor) for interactive runs; or commit a consumer of the TOTAL line so the leg guards a claim rather than itself.

#### tools-3: digestshare's gate role is already carried by the byte-pinned snapshots, and its ratio has no in-tree consumer
- Where: justfile:212-216 (related: tools/digestshare:2-13; justfile:403, 1000; tests/gossip_snapshot.rs)
- Class / severity / confidence: scaffolding / low / medium
- Provenance: verified (`ls design/` lists only rumors-frame-fuzz.md; grep for `digestshare` outside tools/ hits only justfile:211-220, 403, 1000; `git show -s 5f140251` describes the number as the "parent baseline" for the hash-width decision, which has since landed as SHA3-256 in 4f18c347); executed: no
- Seen by: scaffolding [7]; refutation: confirmed; history: deliberate-and-holds (an owner-gated recommendation of the CBOR wire review, implemented in fd997888 with the rationale at the recipe; the review weighed "fires only by hand" against "stays a manual aid" but did not weigh the snapshot suite's overlap, and the ratio consumer it named, design/cbor-legible-wire.md, has since moved to .agent-notes)
- Owner-gated: yes (gate policy; reopens a recorded ruling with new evidence: the consumer is gone and the decision it served has landed)

The recipe says the leg checks the renderer-vocabulary contract, never the ratio. The insta snapshot suite already pins the renderer's output byte-for-byte (root AGENTS.md hard rules), so a vocabulary move fails there first. What the leg adds is keeping a by-hand tool's regexes matching, for a number nothing committed reads. Principle 3: a thing earns its existence by what it serves outside itself.

Evidence:

   212	# vs non-digest bytes. As a gate leg it checks the renderer-vocabulary
   213	# contract, not a threshold: the tool exits nonzero when the corpus's
   214	# byte-count headers or digest annotations stop matching its patterns (the
   215	# renderer's vocabulary moved out from under the meter), never on the
   216	# measured ratio. Build-free, so it rides the lint tier.

Resolution: owner call between (a) moving `digestshare` out of `gate-lints` and `ci` into the conveniences section, keeping the tool (with tools-19's floor) for interactive runs, and (b) committing a consumer of the TOTAL line (a rustdoc figure or a pinned expectation) so the leg guards a claim rather than itself. Acceptance: either gate-lints no longer lists digestshare, or a committed artifact cites the ratio and the gate checks it.

Catches: A first doc paragraph over 220 characters on items rustdoc lists, which clippy's nursery lint `too_long_first_doc_paragraph` checks on compiled items; doclint's rule 2 (`include_str!` separation) has no external equivalent and stays.

Replacement: `-W clippy::too_long_first_doc_paragraph` on the root and detached-workspace clippy legs, confirmed to fire on doclint's summary fixtures, then rule 1 deleted; or the reason clippy's lint does not suffice recorded at `MAX_SUMMARY_CHARS`.

#### tools-20: doclint's summary-length rule reimplements clippy's `too_long_first_doc_paragraph`
- Where: tools/doclint:66-70 (related: doclint:7-25, 116-167; justfile clippy legs)
- Class / severity / confidence: scaffolding / low / medium
- Provenance: assessed (read the rule; the refutation pass reports that `clippy-driver -W help` on an installed toolchain lists `clippy::too-long-first-doc-paragraph` under the nursery group; I did not run the toolchain); executed: no
- Seen by: scaffolding [14]; refutation: confirmed; history: no-rationale-found (`grep -rn too_long_first_doc` over the tree and the agent notes is empty; neither landing nor rewrite commit names the compiler-side lint)
- Owner-gated: yes (removal of an instrument)

Rule 1 (first doc paragraph over a character budget) is what clippy's lint checks on items listed in module pages, with the crate root exempt. The in-house version differs in its 220-character budget and in reaching files the gate's clippy invocations do not compile; the fence tracking, the nbsp rule, the crate-root exemption, and the calibration constant are a maintenance cascade the compiler-side lint carries. Rule 2 (include_str! separation) has no external equivalent and stays. Principle 3: could a maintained tool produce this behavior?

Evidence:

    66	# The longest a summary may render before it stops reading as a one-liner.
    67	# Calibrated to the workspace: the median summary renders to ~180 characters
    68	# (a single sentence over two or three wrapped lines) and stays; the threshold
    69	# catches the multi-sentence paragraphs above it.
    70	MAX_SUMMARY_CHARS = 220

Resolution: owner check: enable `-W clippy::too_long_first_doc_paragraph` on the root and detached-workspace clippy legs, confirm it fires on doclint's summary fixtures, then drop rule 1 (keeping rule 2 and its self-test). If the lint's nursery grade, its compiled-cfg coverage, or its threshold is judged insufficient, record that at MAX_SUMMARY_CHARS as the reason the in-house rule exists. Acceptance: either `just clippy` fails on a 250-character first paragraph and doclint no longer measures summaries, or the constant's comment states why clippy's lint does not suffice.

### Scaffolding nits

| Id | Where | Claim | Catches; replacement | Resolution |
|---|---|---|---|---|
| skyline-fill-grow-32 | `crates/before/src/version/skyline/grow.rs:131-149` | `Cost::deepen`'s test-seam parameter is spelled three ways across its callers | A sanctioned, documented test seam (`Cost::deepen`'s `ceiling`), spelled three ways across its callers. One spelling: delete the oracle's one-argument wrapper so every caller shows the seam. | Delete the oracle's one-argument `deepen` wrapper |
| crate-root-22 | `crates/before/src/fold.rs:97-100` | `balanced_reduce`'s `debug_assert` checks a property its own closures make impossible | Nothing: the two literal closures above the assert make the asserted property impossible. Delete the assert. | Delete the assert |
| oracle-laws-24 | `crates/before/src/laws/tests.rs:9-21` | `law_names_are_unique_across_groups` restates a guarantee `laws!` gives at compile time, and its doc overstates its role | Only a hand-written group bypassing `laws!` that registers a foreign fn under another law's name; `laws!` makes registered names unique at compile time. Dissolve the test, or re-document the one door it guards. | Dissolve the test, or re-document the hand-authored-group case it guards |
| deps-16 | `crates/before/Cargo.toml:1-7` | no package include/exclude: measurement artifacts, the paper transcription, and plotting scripts would ship in the crate tarball | Nothing today; pre-release, `cargo package` would ship results, the paper transcription, and plotting scripts. An `include` (or `exclude`) list before the first release. | An `include` list before the first release |
| meter-adequacy-12 | `crates/before/src/meter/registry.rs:1073-1097` | Registry and surfacecheck rulings carry `decided` dates whose only consumers are format tests | Only a date-shape format test reads the `decided` fields. `git log -S` on the reason strings; delete the fields, `REGISTRY_RATIFIED`, and the two format tests. | Drop `decided`, `REGISTRY_RATIFIED`, and the two format tests |

## Crate root and public types

### Crate root: lib, error, iter, auto traits, build.rs, Cargo.toml

2 entries (2 low); the full record is `evidence/partitions/crate-root.md`. Related findings in other documents: crate-root-32 (verification-gap: the segments counter), crate-root-7 (verification-gap: `auto_traits` totality), crate-root-34 (correctness: serde `bytes` versus `seq`).

#### crate-root-3: serde `derive` feature enabled with no derive in the crate
- Where: crates/before/Cargo.toml:30-30 (related: Cargo.toml:68 at the workspace root)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep for `derive(.*Serialize`, `derive(.*Deserialize`, `#[serde` over crates/before/**/*.rs: no hits; every impl in serde_impls.rs is hand-written); executed: no
- Seen by: structure; refutation: confirmed; history: no-rationale-found (Phase 0 boilerplate carried through the rename; no derive ever existed)
- Owner-gated: no

The `derive` feature pulls `serde_derive` (a proc-macro crate) into every consumer's build that enables `before/serde`, and nothing in the crate uses it. `alloc` is needed (the workspace serde is `default-features = false` and the `Vec<u8>` impls live under `alloc`); `derive` guards nothing.

Evidence:

    30  serde = { workspace = true, optional = true, features = ["derive", "alloc"] }

Resolution: `features = ["alloc"]`. Acceptance: `cargo check -p before --features serde` and the serde test legs build clean.

Synthesis note: deps-8 (this document) is the same feature selection from the dependency sweep; one manifest line closes both.

#### crate-root-15: `Decode::Io` carries its `io::Error` only in the message, not as `source()`
- Where: crates/before/src/error.rs:89-91 (related: crates/before/src/borsh_impls.rs:145-150, crates/before/src/testing/snapshots.rs:276-280)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (thiserror-impl-2.0.20 selects a source field only when named `source` (prop.rs:105) or attributed `#[source]`/`#[from]` (attr.rs:82); the field here is tuple position 0 with no attribute; borsh_impls.rs:147 unwraps the cause by pattern match); executed: no
- Seen by: structure, correctness; refutation: confirmed, low and owner-gated; history: no-rationale-found (the variant switched from `ErrorKind` to `io::Error` at dc88e755b with an empty commit body; no `#[source]`/`#[from]` ever existed)
- Owner-gated: yes (`source()` moving from `None` to `Some` is an observable change on a stable public error type, though a non-breaking one)

`std::error::Error::source()` returns `None` for `Decode::Io`, so chain-walking reporters (and any `Box<dyn Error>` consumer, including serde's `D::Error::custom`) reach the underlying io error only through the Display text. The crate wants explicit construction (seven `map_err(Decode::Io)` sites), so `#[source]` rather than `#[from]` is the fit; the Display string can stay.

Evidence:

    89      /// The underlying reader failed.
    90      #[error("read error: {0}")]
    91      Io(io::Error),

Resolution: `Io(#[source] io::Error)`; keep the Display string so the snapshot at testing/snapshots.rs:276 is unchanged. Acceptance: a test asserts `std::error::Error::source(&Decode::Io(io::Error::from(io::ErrorKind::UnexpectedEof))).is_some()`.

### Clock

5 entries (2 low, 3 nit); the full record is `evidence/partitions/clock.md`. Related findings in other documents: clock-17 (correctness: `Forks::len` on 32-bit targets).

#### clock-12: `Clock::decode`'s prefix-split block is duplicated verbatim in `Span::decode`, and two sites reach two layers down instead of through a component door
- Where: crates/before/src/clock.rs:800-818 (related: crates/before/src/span/wire.rs:134-160; crates/before/src/codec/bits.rs:465-509; crates/before/src/version.rs:1116-1119; crates/before/src/clock.rs:276; crates/before/src/party.rs:352)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (both blocks read and compared: clock.rs:802-813 and span/wire.rs:136-147 are token-identical modulo `id`/`lo` and the `expect` strings, comment included; `git grep -n 'div_ceil(8)'` shows the boundary computation at exactly these two decode sites plus doc formulas and buffer internals); executed: no
- Seen by: structure (both points); refutation: confirmed (severity medium to low: seven lines at two sites; the genre rule itself lives once in `require_marker_padding`); history: no-rationale-found (4874f527 introduced the clock block explicitly as "the Span::decode pattern"; 61d00223 then had to touch both doors to reclassify the flush cut)
- Owner-gated: no

The computation that turns a prefix's end bit into a byte length, classifies a flush cut as `Decode::Truncated`, narrows to `usize` behind an `expect`, and checks the prefix's marker padding appears character for character in two decoders, five-line comment included, and git shows the last change to the truncation genre had to be applied door by door. Separately, `join_all` builds its index through `crate::party::ops::IdIndex::build(self.party.as_bits())` (the same two steps as party.rs:352) and `decode` names `crate::version::skyline::validate_prefix` for the two lines `Version::decode` also runs, so the clock module imports from two submodules it otherwise composes only through public-ish doors. The enclosing `let id_bytes = { ... }` also validates the version tail, so the binding's name understates the block.

Evidence:

       802	            // The party's padding marker rides in its final byte — which an
       803	            // input cut right after a flush id tree lacks. That cut is
       804	            // missing required data (the marker byte, and the whole version
       805	            // after it): the truncation genre, exactly as a byte-starved
       806	            // reader reports the same boundary.
       807	            let id_bytes = (id_end + 1).div_ceil(8);
       808	            if id_bytes > buf.len() as u64 {
       809	                return Err(Decode::Truncated);
       810	            }
       811	            let id_bytes =
       812	                usize::try_from(id_bytes).expect("the id prefix ends within the read buffer");
       813	            codec::require_marker_padding(&buf[..id_bytes], id_end)?;

    (span/wire.rs)
       141	            let lo_bytes = (lo_end + 1).div_ceil(8);
       142	            if lo_bytes > buf.len() as u64 {
       143	                return Err(Decode::Truncated);
       144	            }
       145	            let lo_bytes =
       146	                usize::try_from(lo_bytes).expect("the meet's prefix ends within the read buffer");
       147	            codec::require_marker_padding(&buf[..lo_bytes], lo_end)?;

       276	        let index = crate::party::ops::IdIndex::build(self.party.as_bits());
       815	            let v_end = crate::version::skyline::validate_prefix(codec::BitsView::whole(tail))?;
       816	            codec::require_marker_padding(tail, v_end)?;

Resolution: One `pub(crate)` helper in `codec` beside `require_marker_padding` (respecting 61d00223's decision that the interior guard lives at the doors, not inside the validator), e.g. `fn padded_prefix_len(buf: &[u8], end: u64) -> Result<usize, Decode>`, owning the comment, the `div_ceil(8)`, the truncation check, the narrowing (as a `<= buf.len()` guard, dissolving the `expect`), and the padding check; both decoders call it. Add `pub(crate) fn Version::validate_canonical(bytes: &[u8]) -> Result<(), Decode>` for the two lines `Version::decode` and `Clock::decode` share, and `pub(crate) fn Party::index(&self) -> IdIndex<'_>` for the two `join_all`s. Acceptance: clock.rs contains no `crate::party::ops` or `crate::version::skyline` path; `git grep -n 'div_ceil(8)' crates/before/src` shows the boundary computation only in `codec`; `just test-all` green including borsh_impls's truncation-genre suite and `decode_never_panics`; wire snapshots untouched.

#### clock-26: clock/tests.rs holds the serde and borsh legs for three types and depth proofs for other partitions' surfaces
- Where: crates/before/src/clock/tests.rs:1013-1216 (related: crates/before/src/clock/tests.rs:653-761, 566, 670, 741; crates/before/src/serde_impls/tests.rs:1-13; crates/before/src/borsh_impls/tests.rs:260-291; crates/before/AGENTS.md:37; crates/before/src/surface.rs:1221)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (serde_impls/tests.rs:5-6 points here; borsh_impls/tests.rs:269-290 `borsh_roundtrips_over_impl_history` already pins `borsh::to_vec == as_bytes` and the three-type round-trip; `const DEPTH: usize = 100_000;` declared at 566, 670, 741); executed: no
- Seen by: structure; refutation: confirmed; history: deliberate-and-holds for the serde placement (ada0db0e wrote the pointer as a co-location note) and the two extra depth proofs (reason stated inline at 661-664); accretion for the borsh overlap (d0e54d95 predates borsh_impls/tests.rs)
- Owner-gated: no

Tests live in the sibling file of what they test. The serde suite for `Party`/`Version`/`Clock` (1015-1155) and a borsh suite (1159-1216) sit here although `serde_impls/tests.rs` and `borsh_impls/tests.rs` exist; the borsh block partially duplicates `borsh_roundtrips_over_impl_history`, so the two can drift in what they assert. The rank/span/causal and text/`min_ticks` depth proofs (653-761) prove surfaces the clock module does not own, and the depth constant is declared three times. The serde pointer's stated reason ("beside the world fixture") is weak: the fixture is `testing::optrace`, which borsh_impls/tests.rs imports from its own file.

Evidence:

         1	//! Clock-level tests.
      1015	#[cfg(feature = "serde")]
      1016	proptest! {
      1159	#[cfg(feature = "borsh")]
      1160	proptest! {

    (serde_impls/tests.rs)
         5	//! The party/version/clock legs live beside the world fixture in
         6	//! `clock/tests.rs`.

    (borsh_impls/tests.rs)
       280	            prop_assert_eq!(party_bytes.as_slice(), p.as_bytes());
       281	            prop_assert_eq!(version_bytes.as_slice(), v.as_bytes());
       282	            prop_assert_eq!(&clock_bytes, &c.encode());

Resolution: Move the serde block to serde_impls/tests.rs and delete the pointer. Reconcile the borsh block against borsh_impls/tests.rs, keeping only what is not already asserted there (the two-value concatenation at 1191-1198 if no existing test covers the `Version` pair; the appended-zero-byte rejection if `non_canonical_borsh_bytes_report_invalid_data` does not). Move the two non-clock depth proofs beside their surfaces, or gather every depth-100k proof into one `testing/stack_safety.rs` and update AGENTS.md:37 and surface.rs:1221 (`GridCap { guard: "deep_tree_stack_safety" }`); hoist `DEPTH` to module scope. Acceptance: clock/tests.rs holds only clock differentials, protocol semantics, the clock depth proof, and the orbit pins; `tools/citecheck` resolves every roster citation; `just test-all --all-features` green.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| clock-7 | `crates/before/src/clock.rs:280-283` | The join-as-fallible-combiner adapter closure is spelled three times | A private `join_group` adapter per type; propose a `FnMut(&mut T, T) -> Result<(), T>` combiner shape for `fold` |
| clock-13 | `crates/before/src/clock.rs:916-923` | The manual `Debug` for `Clock` is exactly the derive's expansion | Derive `Debug`; delete the impl |
| clock-16 | `crates/before/src/clock/forks.rs:31-51` | A one-caller helper and a redundant reborrow line in `Forks::new`/`next` | Inline `clock`; delete the reborrow line |

### Party

7 entries (1 medium, 4 low, 2 nit); the full record is `evidence/partitions/party.md`. Related findings in other documents: party-27 (documentation: the deliberately raw `build_split` spine read), party-23 (claim: the fold-index fallback past 2^32 bits), party-20 (documentation: the two `IdLeafCursor`s' recorded drop), party-24 (performance), party-13 (correctness).

#### party-4: The 2-bit tag decode and the id subtree skip are hand-spelled at six sites
- Where: crates/before/src/idbits.rs:145-156 (related: crates/before/src/idbits.rs:100-109, crates/before/src/party/ops/diff.rs:312-314, crates/before/src/party/ops/diff.rs:352-372, crates/before/src/party/ops/index.rs:200-201, crates/before/src/party/ops/split.rs:47-53, crates/before/src/party/ops/split.rs:115-119, crates/before/src/party/ops/sum_split.rs:176-178, crates/before/src/version/skyline/grow.rs:290-306)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read every site; `grep -rn skip_subtree` and `grep -rn 'IdReader::at('` enumerate the callers); executed: no
- Seen by: structure, prose (the `record_bits` spelling); refutation: confirmed; history: no rationale found (the sites accreted across 32a655438, 183e3cab7, 13d8c398, a99e8c8f7)
- Owner-gated: no

The id header probe for `skip_subtree` (record 2 bits, count the two tag bits, advance 2) is written out in `IdReader::skip`, verbatim again in `diff::consume`, and a third time in `grow::id_skip`; `split::subtree_end` and `sum_split::branch_children` each construct a throwaway `IdReader::at` only to call `skip` and read `pos`. The raw tag read `(bits.bit(p), bits.bit(p + 1))` appears in `IdReader::tag`, `diff::enter`, `diff::consume`, `IdIndex::is_disjoint`, `build_split`, and `grow::id_tag`, each with its own `record_bits(2)` line or none. idbits.rs:207-209 calls `skip_subtree` "The single shared spelling of this scan", but what is shared is the generic counter; the id-specific probe is re-derived per caller, and the meter hook rides in each copy, which is how `build_split`'s spine read (party-27) came to be the one tag read that records nothing.

Evidence:

       145	    pub(crate) fn skip(&mut self) {
       146	        if let IdReader::At { bits, pos } = self {
       147	            let bits = *bits;
       148	            *pos = skip_subtree(*pos, |at| {
       149	                // One 2-bit tag scanned per skip step. Children present =
       150	                // the two tag bits; the tag is 2 bits wide.
       151	                crate::codec::scan::record_bits(2);
       152	                let children = u64::from(bits.bit(at)) + u64::from(bits.bit(at + 1));
       153	                (children, at + 2)
       154	            });
       155	        }
       156	    }

       359	        let bits = self.bits;
       360	        let scan = |at: u64| {
       361	            // One 2-bit tag scanned per skipped node. Children present = the
       362	            // two tag bits; the tag is 2 bits wide.
       363	            crate::codec::scan::record_bits(2);
       364	            let children = u64::from(bits.bit(at)) + u64::from(bits.bit(at + 1));
       365	            (children, at + 2)
       366	        };

Resolution: In `idbits`, add two `pub(crate)` free functions and route every site through them: `tag(bits, pos) -> IdNode` (the existing private `IdReader::tag`, recording its own 2 bits) and `subtree_end(bits, at) -> u64` (the id instantiation of `skip_subtree`, recording per step). `IdReader::{read, peek, skip}` call them; `diff::{enter, consume}`, `IdIndex::is_disjoint`, `split::subtree_end`, `sum_split::branch_children`, and `grow::{id_tag, id_skip}` become one-line calls. `build_split`'s spine loop is the one site whose routing changes a committed reading; take it in the same commit as party-27's re-pin or leave it raw and state the exemption there. Acceptance: `grep -n '\.bit(' crates/before/src/idbits.rs crates/before/src/party/ops/*.rs crates/before/src/version/skyline/grow.rs` shows tag-bit reads only inside the two shared helpers (plus `build.rs:257`'s debug assert if kept); every scan-meter test and board scan pin reads an identical number except where the spine newly records.

Synthesis note: skyline-fill-grow-34 (this document) is the grow.rs copy of the same tag read and skip; party-27 (documentation) records the deliberately raw `build_split` spine read whose re-pin this consolidation touches.

#### party-16: `IdBuilder` serves two clients with disjoint method sets and documents only one discipline
- Where: crates/before/src/party/ops/build.rs:4-14 (related: crates/before/src/party/ops/build.rs:137-150, crates/before/src/party/ops/build.rs:222, crates/before/src/party/ops/build.rs:262, crates/before/src/party/ops/build.rs:266, crates/before/src/party/ops/build.rs:302, crates/before/src/party/ops/sum.rs:44, crates/before/src/party/ops/sum.rs:51, crates/before/src/party/ops/sum.rs:76, crates/before/src/party/ops/sum.rs:109)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (grep over build.rs and sum.rs: `open`/`close_node` only at build.rs:222, 262, 302; `splice` only at 266; `copy_reader` at sum.rs:44, 51; `push_tag` at 76; `collapse_terminal_pair` at 109); executed: no
- Seen by: structure; refutation: confirmed; history: deliberate but expired (the type doc described every client at 1d77f4c82; da0f6a937 gave `sum` its own final-tag discipline and left the type doc unchanged)
- Owner-gated: no

The type doc describes the reserve/patch/close discipline (`open`, `close_node`, `splice`), which only `IdSkylineBuilder` uses; `sum` uses a disjoint subset (`push_tag`, `copy_reader`, `collapse_terminal_pair`) that never reserves or patches. The type carries two ways to perform the `(1, 1) → 1` collapse, the second self-described as "The truncation twin of `close_node`'s terminal-collapse arm". A reader of `sum` who opens `IdBuilder` is told about placeholders it never creates. Steelman: both are id emitters over `PackedBuilder` sharing `terminal`, `finish`, `with_capacity`, and the `Built` vocabulary, so one wrapper avoids a third type; the doc is the cheaper fix.

Evidence:

         4	/// Single-buffer builder for normalized id output.
         5	///
         6	/// A node reserves a 2-bit tag placeholder before its children are emitted;
         7	/// [`close_node`](Self::close_node) patches the tag from which children turned
         8	/// out present, collapsing `(1, 1) → 1` (both terminal) and `(0, 0) → 0` (both
         9	/// empty). The id instantiation of the crate's append-truncate discipline
        10	/// ([`PackedBuilder`] carries the shared move set): the per-node payload is
        11	/// only the tag bits, and both collapses are pure truncations.

Resolution: Either rewrite the type doc to name both disciplines and which kernel uses which (final tags at descent plus fixed-width collapse for `sum`; reserve/patch/close for the leaf-driven builder), or move `Open`, `open`, `close_node`, and the reserve into `IdSkylineBuilder`, their only client, keeping `IdBuilder` as the shared core. Acceptance: `IdBuilder`'s doc names no method that only one of its two clients uses without saying so; `sum`/`diff` differentials unchanged.

#### party-19: `compare`, `sum`, and `diff` keep their per-ancestor bit stacks on the output buffer `BitsBuf` where `BitStack` exists
- Where: crates/before/src/party/ops/compare.rs:113-115 (related: crates/before/src/party/ops/sum.rs:135-137, crates/before/src/party/ops/diff.rs:253, crates/before/src/party/ops/diff.rs:257, crates/before/src/codec/stack.rs:1-8, crates/before/src/codec/buf.rs:176-181, crates/before/src/codec/buf.rs:230-239, crates/before/src/party/ops/build.rs:181-184, crates/before/src/version/skyline/overlay.rs:474-477)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`git show --stat 56a3dfe2` lists party/ops/build.rs and the skyline modules, not compare.rs, sum.rs, or diff.rs; `BitsBuf::pop` is `get` (bounds assert plus byte index) plus `truncate` (assert, `Vec::truncate`, `mask_tail`); `BitStack::pop` is one shift with a word spill); executed: no
- Seen by: structure, correctness, claims; refutation: confirmed; history: deliberate but expired (da0f6a937 used the crate's only bit container then; 56a3dfe2 introduced `BitStack` "for every path, phase, and frame" and converted build.rs and the skyline walks; the three party sites were missed, and 83e61b4d renamed them to `BitsBuf` without moving them)
- Owner-gated: no

`Lockstep.pending`, `Frames.bits`, and the diff cursor's `path`/`pending_right` are `BitsBuf` used purely as LIFO bit stacks on the hot paths of `join`, `is_disjoint`, `covers`, and `without`, while `IdSkylineBuilder` and `overlay::IdLeafCursor` use `BitStack`, whose module doc calls it "the word-backed bit stack the deep walks keep their paths and phases on". Two spellings of "a stack of bits" in one crate, the purpose-built one documented as the walks' discipline and the ad hoc one paying an output buffer's invariants (exact bytes, zeroed dead bits) on every push and pop. Fixed sign; no meter counts these pushes.

Evidence:

       113	struct Lockstep {
       114	    /// Two presence bits per queued right child pair, innermost on top.
       115	    pending: BitsBuf,

       135	struct Frames {
       136	    bits: BitsBuf,
       137	}

Resolution: Replace the four fields with `BitStack` (same `push`/`pop` API; `len()` is `u64` as `depth()` already returns). Acceptance: `grep -n 'BitsBuf' crates/before/src/party/ops/compare.rs crates/before/src/party/ops/sum.rs` shows only the output-buffer uses (`sum`'s return type); diff.rs's cursor fields are `BitStack`; all party differentials and deep constructed tests pass; no scan or heap pin moves except the `ID_COVERS`/`ID_DISJOINT` heap readings, which may fall to zero on the divert pair (re-pin as a deliberate event; party-1's contract wording stays a bound either way).

Synthesis note: One of the ten `BitsBuf`-as-stack sites codec-base-text-tree-12 (this document) migrates together; skyline-coding-32 lists the skyline sites.

#### party-28: `split` and `sum_split` rest on a stream-suffix precondition the type does not carry: a dead `start` parameter, a redundant right-child scan asserted equal to `bits.len()`, and an unasserted twin in `branch_children`
- Where: crates/before/src/party/ops/split.rs:62-68 (related: crates/before/src/party/ops/split.rs:20-27, crates/before/src/party/ops/split.rs:78, crates/before/src/party/ops/split.rs:82, crates/before/src/party/ops/split.rs:89-93, crates/before/src/party/ops/sum_split.rs:162-165, crates/before/src/party/ops/sum_split.rs:167-185, crates/before/src/idbits.rs:92-96, crates/before/src/party.rs:234, crates/before/src/party/ops/sum_split.rs:72, crates/before/src/party/ops/sum_split.rs:75, crates/before/src/party/ops/sum_split.rs:105)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn '\.split()'` excluding tests and the oracle: party.rs:234 `self.view()`, sum_split.rs:72/75 root operands, sum_split.rs:105 `IdReader::root(...)`, so `start` is always 0 and every split subtree is the stream's tail; `subtree_end(bits, right_child)` is computed in every profile and used at 78 and 82, then debug-asserted equal to `bits.len()`; `branch_children` states the suffix fact in prose and asserts nothing; `IdReader::at` makes a mid-stream reader constructible); executed: no
- Seen by: structure ([11]), correctness ([42]), claims ([50]); refutation: confirmed and reframed (the debug asserts pin a suffix precondition, not `start == 0`); history: no rationale found (all from 32a655438 and c7c9d3e8f as written)
- Owner-gated: no

Three faces of one unstated precondition. `build_split` takes a `start` that every caller passes as 0. It skips the whole right branch child to compute `branch_end` and then debug-asserts that value equals `bits.len()`, so in release the scan is redundant work with a fixed sign (2 bits per node of the right child through `IdReader::skip`, up to half of `fork`'s tag reads on a right-heavy branch) and in debug it is a precondition check dressed as a computation. `branch_children` relies on the same fact ("the last present child runs to the stream's end") in prose with no assert. A reader positioned mid-stream by `IdReader::at` would satisfy the code's types and violate its arithmetic; the precondition belongs in the doc or the type.

Evidence:

        61	            let left_child = prefix_end + 2;
        62	            let right_child = subtree_end(bits, left_child);
        63	            let branch_end = subtree_end(bits, right_child);
        64	            debug_assert_eq!(
        65	                branch_end,
        66	                bits.len(),
        67	                "the branch subtree is the spine's tail",
        68	            );

        25	        let start = self.pos();
        26	        build_split(self.bits(), start)

       162	/// The branch node's subtree is a suffix of its stream (the spine descent above
       163	/// it consumed only unary tags), so the last present child runs to the stream's
       164	/// end and only a both-present operand pays a skip — of its left child, to find
       165	/// the boundary between the two.

Resolution: Make `split` and `sum_split` whole-stream operations in name and doc (they are today in every caller): drop `start` (`build_split(bits)` from 0), use `bits.len()` for `branch_end` and the capacity hint, keep the relation as `debug_assert_eq!(subtree_end(bits, right_child), bits.len(), ..)` so debug builds still check the root-entry precondition while release builds do no scan, add the same debug assert in `branch_children`, and state the precondition in both method docs ("the reader must be at a stream root"). If the owner prefers structural enforcement, type both entries on a root `BitsView` instead of a positioned reader. Acceptance: fork of `node(Some(&full()), Some(&leftmost(k)))` records a constant number of scan bits at any `k` instead of `4 + 2k`; the board's `party_fork` scan readings on right-heavy families move down and are re-pinned as a deliberate event; `d_fork_join_roundtrip`, `split_arbitrary`, and `sum_split_is_sum_then_split` stay green.

#### party-30: `sum_split` re-accumulates the union spine that is already present verbatim in either operand
- Where: crates/before/src/party/ops/sum_split.rs:77-78 (related: crates/before/src/party/ops/sum_split.rs:18-28, crates/before/src/party/ops/sum_split.rs:94-95, crates/before/src/party/ops/sum_split.rs:104, crates/before/src/party/ops/sum_split.rs:123-126, crates/before/src/party/ops/sum_split.rs:205-223, crates/before/src/party/ops/split.rs:72-76)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read: on a unary union node exactly one of `(al || bl, ar || br)` is set, so the other side's presence is false on both operands, and `IdNode::Internal` has at least one present child, so the present side is true on both; hence `a`'s tag equals `b`'s tag equals the union tag at every spine level, and the `spine` buffer duplicates `self.bits()[start..self.pos()]`); executed: no
- Seen by: structure; refutation: confirmed (including the caveat that line 104's `self.sum(other)` consumes `self`, so the range must be captured before it); history: no rationale found (c7c9d3e8f as written)
- Owner-gated: no

The spine loop pushes each union tag into a fresh `BitsBuf`, and `half`/`splice` copy it into both halves; the method's own spine argument (lines 18-28) proves the pushed bits equal the operand's prefix bit for bit. A buffer whose only content is a copy of bits the walk just read, plus two allocations and per-tag pushes to maintain it, is circular machinery. Splicing the operand prefix is the move `split.rs` already makes for its prefix, so the two kernels would spell the spine the same way.

Evidence:

        77	        // The union's spine tags, shared by both halves (split's prefix).
        78	        let mut spine = BitsBuf::new();

       123	            self.read();
       124	            other.read();
       125	            spine.push(left);
       126	            spine.push(right);

Resolution: Record `(self.bits(), self.pos())` before the loop and `self.pos()` after it (before the delegated `self.sum(other)` at 104 consumes `self`); `half` and `splice` take `(BitsView, Range<u64>)` and `extend_from_view` the range; delete `spine`; note in the method doc that the operand prefix is the union spine, which the spine argument already proves. Acceptance: `sum_split_is_sum_then_split`, `sum_split_collapsed_union_matches_terminal_split`, `sum_split_constructed::*`, and `sum_split_scan_never_exceeds_the_composition` pass with unchanged readings (spine pushes and `extend_from_view` are both unmetered, so `fused` does not move).

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| party-6 | `crates/before/src/party.rs:19-19` | Idiom nits: `from_frozen` bypassed by `decode`/`seed`, qualified `core::fmt`/`crate::`/`std::io` paths beside a once-used import, `then(\|\| ..)` and positional `(u64, u64)` ranges under a `type_complexity` allow | `from_frozen` at both doors; imports; `then_some`; `Range<u64>`; delete the allow |
| party-18 | `crates/before/src/party/ops/compare.rs:32-34` | `#[allow(clippy::wrong_self_convention)]` on `covers` guards nothing | Delete the attribute |

### Version core

13 entries (1 medium, 7 low, 5 nit); the full record is `evidence/partitions/version-core.md`. Related findings in other documents: version-core-5 (claim: the join/meet subadditivity lemma unpromised at the door), version-core-12 (documentation: the three-way lockstep claim).

#### version-core-11: The `_view` join/meet doors, the three-strategy operator macro, and `balanced_fold`'s `view` parameter are Batch-era machinery: every caller passes a `Version`
- Where: crates/before/src/version.rs:864-900 (related: crates/before/src/version.rs:784-791, 796-832, 902-945, 1513-1616; crates/before/src/span/algebra.rs:383-410, 430-443, 481-514, 575-633; crates/before/src/causally/conjunction.rs:41-98; crates/before/tests/meter.rs:10757-10776; crates/before/src/meter/board/coverage.rs:8-10)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (grep of `join_view|meet_view|join_refs|meet_refs|span_refs` over crates/before/{src,tests,benches,examples,fuzz,fuzzfit,wasm32-pins,surfacecheck} and src: every `_view` call passes a `Version`'s `.view()`, in version.rs at 494, 558, 660-683, 810-823, 1364, 1377, and the `binop_matrix!` cells via `r.view()`, and in span/algebra.rs via `b.lo().view()`/`b.hi().view()` where `Span::lo`/`hi` return `&Version` (span.rs:522, 542); `causally/conjunction.rs` uses only the `_refs` forms; the four ladder bodies read side by side); executed: no
- Seen by: structure; refutation: confirmed; history: deliberate-but-expired (58a37d80d removed `Batch`, whose `Batch::join_view(&mut self, incoming: &Bits)` needed a non-`Version` stream, and kept the doors as private methods; 9a6bb5fd7 then added `join_refs`/`meet_refs` as "arm-for-arm mirrors" and re-justified the pair at 784-791 as "the two forms ownership demands")
- Owner-gated: no (private surface; the roster edits it forces are named below)

`join_view`/`meet_view` take `&codec::Bits` so a foreign handle's stream could be folded in, and no such handle exists any more. Their bodies duplicate the `_refs` ladders rung for rung (the general paths are the same emitter call, 878 versus 899), aligned only by prose stated three times ("keep the two in lockstep" at 885-886, 933; "keep the three in lockstep" at 951-952, which version-core-12 shows is already false). What the in-place form buys over `*self = Self::join_refs(self, other)` is one refcount increment/decrement pair on the three trivial rungs; the stated "ownership demand" is `O(1)` refcount traffic, not stream bytes, and nothing committed pins it (the scan pins at tests/meter.rs:10757-10776 measure rung liveness, which the same rung code keeps). That duality then justifies the `own`/`clone`/`assign` `@cell` arms of `binop_matrix!` (1536-1584), the `view: fn(&mut Version, &codec::Bits)` parameter of `balanced_fold` (799), and `SpanFoldOps::{lo_view, hi_view}` (span/algebra.rs:436-438). Principle 3: machinery outlives the constraint that justified it, and a convention held in prose is not a check.

Evidence:

       864	    pub(crate) fn join_view(&mut self, incoming: &codec::Bits) {
       ...
       878	        *self = Version::from_bits(skyline::emit::join(self.0.live(), incoming.live()));
       ...
       884	    /// [`join_view`](Self::join_view) for the case where neither operand is
       885	    /// owned — the same short-circuits in the same order (keep the two in
       886	    /// lockstep), with each hand-back arm cloning the operand that is itself
       887	    /// the answer.
       ...
       899	        Version::from_bits(skyline::emit::join(a.0.live(), b.0.live()))

Resolution: make `join_view`/`meet_view` take `&Version` and be one-liners (`*self = Self::join_refs(self, other)`), or delete them and write the assignment at the call sites. Collapse `binop_matrix!` to `span_matrix!`'s shape: one arm for the four value cells using `Borrow::borrow` plus one `*Assign` arm. Drop the `view` parameter from `balanced_fold` (the `Merged × Input` arm becomes `a = refs(&a, b.borrow())`). In span/algebra.rs, `SpanFoldOps` loses `lo_view`/`hi_view` and the `_core` kernels call the `_refs` forms. Delete the three "lockstep" sentences and re-word 784-791. Rename the `join_view_*`/`meet_view_*` rows at tests/meter.rs:10757-10776 (they keep pinning the same rungs through `|=`/`&=`) and re-state coverage.rs:8-10's "the same `join_view`/`meet_view` emitters". Acceptance: `just gate` green; `empty_operands_answer_without_a_walk`'s scan pins unchanged; `grep -rn lockstep crates/before/src` returns nothing; `binop_matrix!` has one value-cell arm and one assign arm.

Synthesis note: version-core-8, span-causally-9, and span-causally-12 (this document) are the fold and operator copies the `_view` duality justifies; landing this entry first shrinks all three. version-core-12 (documentation) shows the three-way lockstep claim is already false.

#### version-core-8: `span_all` re-implements `balanced_fold`'s counter dispatch; a third copy lives in span algebra
- Where: crates/before/src/version.rs:652-698 (related: crates/before/src/version.rs:806-827, 1230-1270, 1310-1331; crates/before/src/span/algebra.rs:369-424, 430-443; crates/before/src/fold.rs:1-3, 91-104)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (all three copies read: version.rs:675-679 and 817-821 are word-for-word identical; span/algebra.rs:400-404 differs only in "every leg kernel"; both `DedupRuns::new` call sites pass a `Borrow<Version>` projection, 805 `Borrow::borrow` and 652 `FoldInput::version`, whose `Borrow` impl at 1312-1316 is the same function); executed: no
- Seen by: structure; refutation: confirmed, severity medium to low (the span-algebra copy carries a `points` fast path at 375-377 that a shared fold must accommodate, so "one copy in the crate" over-reaches); history: the two-sided accumulator is deliberate (2d3c232f0), but no commit weighs duplicating the dispatch rather than generalizing the fold; the fold-unification survey records the n-ary reduction as "already owned", the standard these copies fall short of
- Owner-gated: no

fold.rs names itself "the one home for the counter discipline, so a hardening of the fold shape reaches every fold at once", yet the input-versus-merged dispatch that sits on top of it is written three times, so a change to the lone-input or weight-0 reasoning must be made in three places. `DedupRuns<I, F>`'s projection `F` duplicates the `Borrow<Version>` bound both call sites already satisfy. This composes with version-core-11: once the `_view` forms collapse, `balanced_fold` loses its `view` parameter and generalizing it over the accumulator is a smaller step.

Evidence:

       652	        let inputs = DedupRuns::new(self.with_items(iter), FoldInput::version).map(Hull::Input);
       ...
       675	                // Unreachable through the counter's weight discipline (a
       676	                // weight-0 lone input never sits below a merged group in the
       677	                // closing drain), but the match stays total rather than
       678	                // asserting: both sides' combiners are commutative, so folding
       679	                // the raw input into the owned hull is value-identical.

Resolution: generalize `balanced_fold` over the accumulator `M` with a small ops record (`lone: fn(&Version) -> M`, `leaf: fn(&Version, &Version) -> M`, `absorb: fn(&mut M, &Version)`, `merge: fn(&mut M, M)`), instantiated with `M = Version` by `join_all`/`meet_all`/`Sum` and with the `(lo, hi)` pair by `span_all`; `Hull` dissolves into `Group<B, M>`; `DedupRuns<I>` drops `F` for `I::Item: Borrow<Version>`. Span algebra can adopt the same fold with a `points` pre-check hook. Acceptance: the weight-discipline comment appears once in version.rs; `fold_clone_collapse_is_value_invisible`, `boundary_arity_fan_folds_match_the_sequential_fold`, and the `VERSION_LIST` fold laws stay green; `DedupRuns` has no function-typed field.

Synthesis note: Composes with version-core-11; span-causally-9 (this document) is the span-side copy of the same two-sided fold and proposes the shared `Endpoints` home.

#### version-core-13: The `debug_assert!` on `hull.relation` is the only production reader of `emit::Hull.relation`, and the differential it re-runs is committed
- Where: crates/before/src/version.rs:1001-1007 (related: crates/before/src/version/skyline/emit.rs:146-160, 177-184, 265-270; crates/before/src/version/skyline/emit/tests.rs:65, 296, 464)
- Class / severity / confidence: vestigial / low / medium
- Provenance: verified (`grep -rn '\.relation\b'` over crates/before/src: production readers of `Hull.relation` are version.rs:1005 and its construction at emit.rs:267; the other hits are `fill.rs`'s `Relation` enum, a different type; emit/tests.rs pins `hulled.relation` against the oracle at three sites); executed: no
- Seen by: structure; refutation: confirmed; history: deliberate-and-holds (e4d4817f1 added field and assert together, rationale stated at both sites); the field's other stated purpose, "later comparable-pair fast paths and equality dedup" (70eb67ab1), never landed because the ladder classifies before emitting
- Owner-gated: yes (removal of an instrument)

The rationale is stated at the site, but under the doctrine's own test it does not hold: an assert must name a concrete, constructible failure the committed tests cannot catch, and "for deterministic pure functions, runtime recompute-and-compare asserts are not defense-in-depth" once differential coverage exists. `causal_cmp` and `emit::hull`'s folded relation are both deterministic functions of the two streams, each differentially pinned against the oracle (the verdict matrix; emit/tests.rs on worked, exhaustive, and proptest populations). The cost is not the assert (compiled away in release) but the surviving-directions fold inside every hull emission, whose only consumer is this debug-build assert: emit.rs:183-184 names the assert as the field's purpose, and the assert checks the field, which is the circular-justification tell.

Evidence:

      1001	        // The fused walk folds the pair relation beside its emissions (an O(1)
      1002	        // flag pair riding sign reads the walk performs anyway), so the
      1003	        // ladder's classification is cross-checked at the only door that emits.
      1004	        debug_assert!(
      1005	            hull.relation.is_none(),
      1006	            "the comparison rung admits only concurrent pairs to the emitting walk"
      1007	        );

Resolution: drop the `debug_assert!` and its comment. Then decide (skyline partition) whether `Hull.relation` and the directions fold in `emit::hull` keep a consumer; if the comparable-pair fast path is not planned, remove the field, the fold, and the three `hulled.relation` assertions in emit/tests.rs, and re-state the emit doc. If a consumer is planned, replace the assert with that consumer. Acceptance: `grep -rn 'hull.relation\|hulled.relation' crates/before/src` returns only emit-internal hits, or none; `identity_fast_paths_agree_across_buffer_identity` and the `span_is_the_pair_hull` law stay green.

Construction: to confirm redundancy rather than a defect, mutate `emit::hull` to return `relation: Some(Ordering::Equal)` unconditionally: emit/tests.rs fails on the oracle differential before any debug build of `span_refs` runs.

Synthesis note: skyline-coding-17 (this document) decides the field's fate: a shared const-generic sweep either keeps `Hull.relation` for a consumer or drops the directions fold with it.

#### version-core-24: `Ticks::limbs` re-derives an exact size and reaches through two newtypes because `suanpan::Limbs` exposes neither `ExactSizeIterator` nor a `Base` accessor
- Where: crates/before/src/version/ticks.rs:112-118 (related: crates/before/src/version/ticks.rs:125-149; crates/suanpan/src/limbs.rs:43-72; crates/before/src/codec/base.rs:26-35; crates/before/src/version/ticks/tests.rs:81-109)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read crates/suanpan/src/limbs.rs in full: `Limbs` wraps `core::slice::Chunks<'a, Word>` and implements `Iterator` and `DoubleEndedIterator` only; `Base` is `pub struct Base(pub(crate) UBig)` at base.rs:26); executed: no
- Seen by: structure; refutation: confirmed; history: the shadow counter arrived with the shape surface (46eb64f9); no note explains recomputing the count over delegating `size_hint`; suanpan's API is unpublished "until a second consumer stabilizes the API"
- Owner-gated: yes (additive trait impls on suanpan's public `Limbs`)

`core::slice::Chunks` is `ExactSizeIterator` and `FusedIterator`, so the chunk count is available for free; before's `Limbs` instead recomputes it as `bits().div_ceil(64)` through a `usize::try_from(..).expect(..)`, decrements a shadow `remaining` in `next`, and implements the two traits over the shadow. Two spellings of one quantity are held equal by a test (`limbs_respell_the_count`, per step) rather than by construction, and `&self.0 .0` bypasses the `Base` newtype at a use site.

Evidence:

       112	    pub fn limbs(&self) -> Limbs<'_> {
       113	        Limbs {
       114	            limbs: suanpan::Limbs::new(&self.0 .0),
       115	            remaining: usize::try_from(self.0.bits().div_ceil(64))
       116	                .expect("a stored count's limb count fits usize"),
       117	        }
       118	    }

Resolution: in suanpan, `impl ExactSizeIterator for Limbs<'_>` (delegating `size_hint` to `self.chunks.size_hint()`) and `impl FusedIterator for Limbs<'_>`; in before, `Limbs { limbs }` forwards `size_hint` and the `remaining` field and `expect` go; add a crate-private `Base::limbs(&self) -> suanpan::Limbs<'_>` so `Ticks::limbs` reads `self.0.limbs()`. Acceptance: `limbs_respell_the_count` stays green; `grep -n remaining crates/before/src/version/ticks.rs` is empty; no `.0 .0` in ticks.rs.

Synthesis note: api-audit-12 (this document) is the suanpan side of the same gap; one additive impl in suanpan closes both.

#### version-core-26: `hull_traffic`'s `snapshot` and `reset` enumerate the `Rung` variants by hand; `web_traffic` is a shape-for-shape copy
- Where: crates/before/src/version/hull_traffic.rs:92-102 (related: crates/before/src/version/hull_traffic.rs:67-75, 83-90; crates/before/src/version/skyline/web_traffic.rs:58-98)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read both modules: `cell()` is an exhaustive match, `snapshot` and `reset` are hand-listed; web_traffic.rs:58-98 has the identical construction over `Decision`); executed: no
- Seen by: structure; refutation: confirmed, adding that `snapshot` (84-89) shares the hand enumeration; history: hull_traffic from e4d4817f1, web_traffic from 76579a3c, whose message calls the copy "the hull_traffic idiom"; no rationale for the array literal
- Owner-gated: no

Adding a variant makes `cell()` fail to compile but leaves `reset()` compiling and silently skipping the new counter (so it never resets between scenarios) and `snapshot()` compiling without a field for it (so it goes uncounted). Principle 5's "no hand-maintained counts" applied to code: an enumeration the compiler does not check rots silently, and here the rot is a meter that stops resetting. The duplication with `web_traffic` is the crate reimplementing its own small capability twice.

Evidence:

        92	    /// Reset every rung counter to zero.
        93	    pub(crate) fn reset() {
        94	        for rung in [
        95	            super::Rung::Equal,
        96	            super::Rung::Empty,
        97	            super::Rung::Comparable,
        98	            super::Rung::Concurrent,
        99	        ] {
       100	            cell(rung).store(0, Ordering::Relaxed);
       101	        }
       102	    }

Resolution: store the cells as `static CELLS: [AtomicU64; N]` indexed by a `Rung::index()` (or `#[repr(usize)]`), so `reset` is a loop over `&CELLS` and `snapshot` reads by index; or lift a small `Tally<const N: usize>` (record/snapshot/reset over an atomic array) into `codec`/`meter` and have both classified counters use it. Apply the same to web_traffic.rs. Acceptance: no per-variant array literal in either `reset`; adding a `Rung` variant requires touching exactly the enum and the snapshot struct, with a compile error naming the second.

#### version-core-27: `version/tests.rs`'s module doc omits half the file, and the `Rank`/`Ranked` suites live two modules away from the types they test
- Where: crates/before/src/version/tests.rs:1-5 (related: crates/before/src/version/tests.rs:344-347, 753, 948, 1347, 1667, 1741, 1865, 2185, 2375; crates/before/src/version/rank.rs; crates/before/src/version/ranked.rs)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (`grep -n 'mod tests'` over rank.rs, ranked.rs, rank/*.rs: only rank/num.rs:650; `ls crates/before/src/version/rank/` holds `num` and `num.rs`, no `ranked/` directory exists; `grep -c version/tests.rs tools/covcheck-expected.json` is 0; tools/citecheck resolves `surface.rs` pins by the final `::` segment of a collected test name, so a module move costs nothing there); executed: no
- Seen by: structure ([2]), prose ([24]); refutation: both confirmed (one sub-claim softened: 283-342 are identity/absorbing laws in byte-parity form, so "lattice laws" is partly accurate); history: legacy placement (the rank type was born in version/tests.rs on 2026-06-10; the sibling-file convention entered root AGENTS.md on 2026-07-24); no decision recorded
- Owner-gated: no

The module doc lists five topics; the file's own section headers add `rank` (753), `the rank wire form` (948), `the numerator's wide arm` (1347), `the join fold` (1667), `ranked` (1741), `the composite ranked key` (1865), plus the at-rest size pin and the deep-spine grammar pin. Roughly lines 753-1665 and 1741-2114 are the `Rank` and `Ranked` unit suites, reaching their subjects as `super::Rank` and `super::rank::arm_ceiling::force`; `rank.rs` and `ranked.rs` declare no `mod tests`. The crate convention places unit tests in a sibling of the source they test, and a module doc that omits half the file is stale prose: a reviewer of `rank.rs` has no signpost to where its tests are.

Evidence:

         1	//! Version tests.
         2	//!
         3	//! The causal order and its comparison matrix, the join/meet operator matrices
         4	//! and lattice laws, grow optimality against the brute-force reference,
         5	//! `min_ticks`, and projection (`/`).

Resolution: minimum, rewrite the module doc to map every `// ───` section in the file. Better, move the rank sections to `crates/before/src/version/rank/tests.rs` (the directory exists) with `mod tests;` in rank.rs, and the `Ranked`/composite-key sections to `version/ranked/tests.rs`, taking `stream_rank`/`seeded_rank`/`rank_parts`/`stairs` with their consumers; the `pub(crate)` items they reach (`from_raw`, `raw_parts`, `numerator_is_wide`, `content_bits`, `arm_ceiling`, `BACKEND_CAPACITY_BITS`) need no visibility change. The move crosses into the rank partition's ownership; this note is the pointer. Acceptance: `rank.rs` and `ranked.rs` each have a sibling `tests.rs`, `version/tests.rs`'s module doc names exactly its remaining sections, and `just gate` is green with no new seed files (a moved proptest that had a seed replays from its new path, per `tests/seed_liveness.rs`, or the seed moves with it).

Synthesis note: rank-2 (this document) is the same relocation from the rank partition; the move is one coordinated change.

#### version-core-28: Two proptests assert the same `|=` cells on the same population
- Where: crates/before/src/version/tests.rs:163-189 (related: crates/before/src/version/tests.rs:215-231, 257-274)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (both bodies read: same generators `world_strategy`, `i`, `j` in `0..64`, same oracle expectation, same two cells `Version |= Version` and `Version |= &Version`; `grep -rn version_assign_join_matches_oracle` over crates/before and tools returns only the test file); executed: no
- Seen by: structure ([3]), prose ([22]), correctness ([47]); refutation: confirmed; history: deliberate-but-expired (at 58a37d80d^ the first test's second block was `Batch |= &Version` and the second covered the `{Version, Batch}²` matrix; 58a37d80d re-scoped both to the surviving cells, leaving them identical)
- Owner-gated: no

A test earns its place by what it catches that nothing else does; two properties over one population asserting the same cells double the run time and the reading load for no coverage. The meet side has only the matrix form, so the pair is asymmetric with its dual as well as redundant.

Evidence:

       171	    fn version_assign_join_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
       ...
       180	        let mut assign = a.clone();
       181	        assign |= b.clone();
       182	        prop_assert!(assign == expected);
       ...
       219	    fn join_assign_matrix_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
       ...
       228	        { let mut x = a.clone(); x |= b.clone(); prop_assert!(x == expected); }
       229	        { let mut x = a.clone(); x |= &b; prop_assert!(x == expected); }

Resolution: delete `version_assign_join_matches_oracle` and fold its motivation ("neither of which the by-value `|` differential reaches") into `join_assign_matrix_matches_oracle`'s doc, whose name mirrors the meet dual. Acceptance: one `|=` differential remains beside one `&=` differential; the seed file needs no change (both draw `(ops, i, j)` from `world_strategy`).

#### version-core-36: Five committed proptest seeds describe parameter shapes of properties that no longer exist in their files
- Where: crates/before/proptest-regressions/version/tests.txt:7-8 (related: crates/before/proptest-regressions/version/own/tests.txt:7-9; crates/before/src/version/own/tests.rs:127-144; tests/seed_liveness.rs:1-30)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (seed files read; grep over `proptest!` signatures in version/tests.rs finds no `scale` parameter and the only `k in` is `descending_literals_build_the_oracle_tree`'s `k in 1u64..=1u64 << 40`, a different shape from `(ops, i, j, k)`; own/tests.rs's only property is `mirror_cells_agree_on_arbitrary_triples(ow, ov, op)`; `tests/seed_liveness.rs` checks only that seed paths resolve, not that shrunk parameters match a live signature; the git attribution of each orphan to its dissolving commit is the history pass's, not re-run here); executed: no
- Seen by: structure; refutation: confirmed; history: each orphan traced (tests.txt:7 from 472d646e2, its `k`-taking properties dissolved by f3e715d0; tests.txt:8 from 782064269, its `scale` properties retired by c5b8d5ec9; own/tests.txt:7-9 from a7b6955e3, dissolved by f149d76b); no dissolving commit ruled on the seeds, and the root AGENTS.md "never strip" rule governs seeds a failure produces, not pruning orphans
- Owner-gated: yes (the doctrine forbids stripping seeds casually; this is a deliberate pruning that needs a ruling)

Proptest replays every seed in a file for every property in that file, so these still run as RNG seeds, but they no longer reproduce the failure they record and their comments name dissolved tests: a ghost reference (Principle 5) whose replay cost is paid on every run for a failure no live property can re-observe. No instrument catches this class: `seed_liveness` checks paths, not signatures.

Evidence:

         7	cc b187e79143e8821fb0e3c7c5d070a90ef9d77161138576d8c601af8c35253c26 # shrinks to ops = [Fork(0), Send(5, 1), Fork(1), Fork(0), Join(0, 3), Fork(1), Join(1, 2), Send(4, 0)], i = 0, j = 0, k = 46
         8	cc f2e1622bbf94d90894d55b51d4dda12cae2f3f7d6d0fd55126c22d635387dfa5 # shrinks to scale = 125

Resolution: for each orphaned seed, move it to the file of the property that now owns the invariant if one exists, else remove it in a commit naming the dissolved property; and consider extending `tests/seed_liveness.rs` to check each `# shrinks to` parameter list against the sibling file's live `proptest!` signatures, so the class is caught mechanically. Acceptance: every `cc … # shrinks to …` comment in the two files names only parameters of a `proptest!` signature in the sibling test file.

Construction: list each seed's `# shrinks to` parameter names and diff against the `proptest!` signatures in the sibling tests.rs; the five lines above have no match. To confirm they are inert as regressions: no live assertion can fail on them, because the property they shrank does not exist.

Synthesis note: suanpan-tests-1 (this document) is the same seed-file class in suanpan; the recommendation to extend `tests/seed_liveness.rs` to check shrink parameter lists would catch both mechanically.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| version-core-2 | `crates/before/src/version.rs:293-293` | The `M` caption is pasted five times with inconsistent wrapping | Emit the caption from the island renderer, or hoist it to one `Rank` note |
| version-core-4 | `crates/before/src/version.rs:385-386` | Rustdoc link syntax inside `//` comments is inert | Plain paths in `//` comments |
| version-core-19 | `crates/before/src/version.rs:1703-1711` | `Div<&Party> for &Version` lives in version.rs and constructs `OwnVersion` through `pub(crate)` fields | Move the `Div` impl into own.rs, or state the grouping |
| version-core-20 | `crates/before/src/version.rs:1721-1760` | `causal_cmp_impls!` has one invocation and is the fixed-body twin of `view_cmp_impls!` | One body-parameterized fan-out macro |
| version-core-31 | `crates/before/src/version/tests.rs:857-885` | The two rank-order sweeps duplicate the perturbed-pair construction verbatim | One `adversarial_pair` helper |

### Rank

10 entries (1 medium, 5 low, 4 nit); the full record is `evidence/partitions/rank.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: rank-26. Related findings in other documents: rank-32 (claim: `Ranked::encode_rank` is `rank().encode()` spelled out), rank-16 (performance: the 32-bit seam's unpinned peaks).

#### rank-22: The two-arm numerator carries a maintenance cascade a single limbs arm would not
- Where: crates/before/src/version/rank/num.rs:4-48 (related: num.rs:86-139, num.rs:402-417, crates/before/src/version/rank.rs:146-147, rank.rs:537-571, rank.rs:314-322, rank.rs:949-957, crates/before/src/codec/base.rs:97-146, crates/before/src/version/tests.rs:1347-1665, crates/before/src/version/skyline/query/integral.rs:1142-1163, crates/before/tests/meter.rs:1197-1198)
- Class / severity / confidence: simplification / medium / medium
- Provenance: assessed (read: the override module at num.rs:113-139, the `#[cfg(test)]` read inside the production routing predicate at 93-99, the dispatch invariant every constructor re-establishes at 372-400 and 409-417, the rank-only `Base` shims at base.rs:97-146 whose callers are rank.rs:807 and num.rs:287, 299, 337 only, the ~320-line wide-regime suite at version/tests.rs:1347-1665, and `Integrator::finish` reading out via `sign_magnitude` at integral.rs:1162 on an `Accumulator` that also offers `sign_limbs`); executed: no
- Seen by: structure, correctness (as an open question); refutation: confirmed as a coherent design proposal, not a defect; history: deliberate and holds (79a944ab and num.rs:4-19 record the rationale: keep the backend as the arithmetic engine wherever it can represent the value, and keep base-arm limb records byte-identical); caveat from cfa7c7ed: the board's `rank_encode` limb floor depends on base-arm limb records (a liveness floor), so any redesign must keep width records
- Owner-gated: yes (a documented design decision)

Totality past the backend's 32-bit cap is mandatory, so some mechanism must exist; the question is its shape. The two-arm form carries: a canonical-dispatch invariant every constructor re-establishes, a test-only thread-local read inside the production routing predicate, a routing predicate with dual code paths in `Add` and `checked_sub`, rank-only shims on `Base` (`msb_cmp`, `msb_windows`, `to_be_bytes`, `from_be_bytes`, `to_bytes_le`), cross-arm `PartialEq`/`Hash` metering conventions, and a wide-regime copy of the rank test suite whose only job is to drive the seam. The module doc's own justification for the base arm's shape ("meters exactly as it did when Base was the numerator's only storage") is a stabilization convention: code shaped so pinned readings do not move (Principle 3). The wide arm already implements every operation `Rank` needs on a `Vec<u64>`, and the rank fold's readout can be `sign_limbs()` instead of `sign_magnitude()`. Steelman for the current form: dashu keeps values up to two words inline (small ranks never allocate and run at dashu's speed), `Display` below the ceiling uses dashu's subquadratic radix conversion, and the existing limb envelopes stay byte-identical.

Evidence:

        14	//! this module. [`Num`] closes that gap: the [`Base`] arm keeps the
        15	//! backend as the arithmetic engine of record everywhere it can represent
        16	//! the value, and the [`Wide`] arm stores the sliver it cannot — bounded
        17	//! only by memory — implementing exactly the operation set rank arithmetic
        18	//! needs (byte assembly, bit reads, right shifts, ±1, MSB-window
        19	//! comparison, and limb streaming into the accumulator).
      ...
        42	//! stores the value. Base-arm operations delegate to [`Base`]'s already
        43	//! metered methods, so a value below the ceiling meters exactly as it did
        44	//! when [`Base`] was the numerator's only storage. Work routed through the

        93	pub(crate) fn arm_ceiling_bits() -> u64 {
        94	    #[cfg(test)]
        95	    if let Some(bits) = ceiling::override_bits() {
        96	        return bits;
        97	    }
        98	    BACKEND_CAPACITY_BITS
        99	}

Resolution: Owner decision. If pursued: `Num` becomes a limbs-only struct (`SmallVec<[u64; 2]>` if allocation-free small ranks are wanted, else `Vec<u64>`) whose methods are the current `Wide` impl plus width-scale limb records; `Add`/`checked_sub`/`Sum` all go through `accumulate` (delete `backend_alignment_fits` and both backend routes); `Display` converts to a transient `UBig` via `UBig::from_words` when the width fits the backend and falls back to the existing long division above it; the fold reads out `sign_limbs()`; delete `ceiling`, `arm_ceiling_bits`, `from_base`, `is_wide`, `numerator_is_wide`, the rank-only `Base` shims, and fold the wide-regime suites into the base suites (their generators already produce every width). Re-pin `RANK_PAIR_MISMATCH` and `RANK_SUM_MIXED` at the parent commit first (the limb column moves to the touch column). The wasm32 pins stay unchanged: they assert values. Acceptance: num.rs is the limbs implementation plus `Display`/`Hash`/`Eq`; rank.rs has one arithmetic route; `grep -rn 'arm_ceiling\|is_wide\|numerator_is_wide\|from_base\|backend_alignment_fits' crates/before/src` is empty; num/tests.rs's differential suites pass with the ceiling parameter removed; all `RANK_TRIPLE` laws, the alignment sweeps, `rank_encoding_exhaustive_small_scope`, the board's `rank_encode` limb floor, and the wasm32 rank pins pass unchanged; the two re-pinned envelopes carry the movement's attribution.

#### rank-8: The two-route dispatch is written twice, threaded by bool parameters
- Where: crates/before/src/version/rank.rs:314-322 (related: rank.rs:949-957, rank.rs:587, crates/before/src/version/rank/num.rs:426, crates/before/src/version/skyline/signed.rs:58)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (both blocks read; `accumulate(lhs, rhs, e, subtract_rhs: bool)` at 587 threads into `fold_into(&self, acc, shift, subtract: bool)` at num.rs:426; `Sign` is `pub(super)` at signed.rs:58 and not reachable here); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (both copies arrived in 79a944ab, whose message speaks of "a routing predicate" in the singular)
- Owner-gated: no

`checked_sub`'s Greater arm and `Add::add` spell the same routing, differing only in the headroom literal (0 vs 1), the backend operator (`-` vs `+`), and the bool passed to `accumulate`; the routing argument is restated in two comment blocks (299-313, 940-948). A change to the predicate's contract must land twice, and the call sites `accumulate(self, other, e, true)` / `accumulate(self, rhs, e, false)` do not say what the bool means.

Evidence:

       314	                let e = self.exp.max(other.exp);
       315	                if let (Num::Base(a), Num::Base(b)) = (&self.num, &other.num) {
       316	                    if backend_alignment_fits(a, self.exp, b, other.exp, e, 0) {
       317	                        let a = a.clone() << (e - self.exp);
       318	                        let b = b.clone() << (e - other.exp);
       319	                        return Some(Rank::from_raw(a - &b, e));
       320	                    }
       321	                }
       322	                let difference = accumulate(self, other, e, true);

       949	        let e = self.exp.max(rhs.exp);
       950	        if let (Num::Base(a), Num::Base(b)) = (&self.num, &rhs.num) {
       951	            if backend_alignment_fits(a, self.exp, b, rhs.exp, e, 1) {
       952	                let a = a.clone() << (e - self.exp);
       953	                let b = b.clone() << (e - rhs.exp);
       954	                return Rank::from_raw(a + &b, e);
       955	            }
       956	        }
       957	        accumulate(self, rhs, e, false)

Resolution: One local `enum Op { Add, Sub }` with `fn headroom(self) -> u64` and `fn backend(self, a: Base, b: &Base) -> Base`; one `fn combine(lhs: &Rank, rhs: &Rank, op: Op) -> Rank` holding the routing and its comment once; `Add::add` becomes `combine(self, rhs, Op::Add)` and the Greater arm `Some(combine(self, other, Op::Sub))`; `accumulate` and `Num::fold_into` take `Op` instead of `bool`. Acceptance: one call site of `backend_alignment_fits`; no `bool` parameter on `accumulate` or `fold_into`; the `RANK_TRIPLE` laws, `rank_wide_arm_arithmetic_matches_the_backend_oracle`, the wasm32 `pin_rank_add`/`pin_rank_checked_sub`, and the `RANK_PAIR_MISMATCH`/`RANK_SUM_MIXED` envelopes read identically (pure refactor).

#### rank-18: Four byte-source adaptors wrap decode_stream, and two decodes copy their whole input first
- Where: crates/before/src/version/rank.rs:670-682 (related: rank.rs:461-465, rank.rs:684-716, crates/before/src/version/ranked.rs:271-290, crates/before/src/borsh_impls.rs:211-220, borsh_impls.rs:240-254, crates/before/src/version.rs:33)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (grep lists the adaptors at rank.rs:675, ranked.rs:276-280, borsh_impls.rs:213-217 and 242-246, plus a test use at borsh_impls/tests.rs:501; the second name `decode_rank_stream` at version.rs:33; `read_to_end` at rank.rs:462-463 and ranked.rs:272-273); executed: no
- Seen by: structure; refutation: confirmed (with the borsh genre change flagged); history: no rationale found for the `FnMut` seam (58c85989); the `read_to_end` shape is deliberate precedent (the family's whole-input strict decode)
- Owner-gated: yes (the borsh transport's error genre on a short read would change from `Decode::Io(UnexpectedEof)` to `Decode::Truncated` unless mapped deliberately)

`decode_stream` takes `FnMut() -> Result<u8, Decode>` and every caller writes its own adaptor: an iterator closure, an index closure, and two identical `read_exact` closures. `Rank::decode` and `Ranked::decode` additionally `read_to_end` into a `Vec` before parsing, so `Rank::decode(&key[..])` copies the key to decode it. The stream is self-delimiting by design (the module doc's point at 107-113), so a `Read`-based parser is the natural spelling; the byte-source generic is machinery serving only the adaptors. Steelman: the closure form keeps `decode_stream` io-agnostic and the cost is small.

Evidence:

       673	fn decode_bytes(bytes: &[u8]) -> Result<Rank, Decode> {
       674	    let mut iter = bytes.iter();
       675	    let rank = decode_stream(|| iter.next().copied().ok_or(Decode::Truncated))?;
       676	    if iter.next().is_some() {
       677	        // `decode` handed over the whole input, so bytes past the
       678	        // self-delimited stream are non-minimal packing.
       679	        return Err(Decode::TrailingBits);
       680	    }
       681	    Ok(rank)
       682	}

    (borsh_impls.rs)
       213	        decode_rank_stream(|| {
       214	            let mut byte = [0];
       215	            reader.read_exact(&mut byte).map_err(Decode::Io)?;
       216	            Ok(byte[0])
       217	        })

Resolution: `pub(crate) fn decode_stream<R: Read>(reader: &mut R) -> Result<Rank, Decode>` with `BitSource<R>` reading one byte via `read_exact`; `Rank::decode` becomes `decode_stream` then a one-byte EOF probe (a byte read is `TrailingBits`); `Ranked::decode` becomes `decode_stream` then `Version::decode(reader)`; the two borsh impls call it directly; delete `decode_bytes` and the `decode_rank_stream` alias. The owner rules how a short read maps (`Truncated` is arguably the correct genre; `Io` preserves today's borsh behavior). Acceptance: one `decode_stream` definition and name, no closure adaptors; `rank_decoding_rejects_each_genre`, `rank_encoding_exhaustive_small_scope` (both rejection genres still fire), `ranked_decode_rejects_each_genre`, and the borsh round-trip suites pass; `Rank::decode` on a slice performs no `read_to_end` allocation.

#### rank-25: Num::msb_cmp's (Base, Base) arm is a no-op distinction and Base::msb_cmp is a one-caller wrapper
- Where: crates/before/src/version/rank/num.rs:277-292 (related: crates/before/src/codec/base.rs:97-112)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (base.rs:104-106 is exactly `msb_cmp_windows(a.msb_windows(), b.msb_windows())`; `grep -rn msb_cmp crates/before/src` shows `Base::msb_cmp`'s only caller is num.rs:287); executed: no
- Seen by: structure, claims; refutation: confirmed; history: no rationale found (79a944ab extracted the kernel from `Base::msb_cmp` and kept the wrapper without saying why)
- Owner-gated: no

The doc tells the reader two paths exist ("the historical path, metering included" versus "the same shared kernel") where all four arms are the identical expression with identical metering; a reader must open base.rs to learn there is no difference.

Evidence:

       281	    /// Same-arm base pairs take [`Base::msb_cmp`] — the historical path,
       282	    /// metering included; every other pairing streams both arms' windows
       283	    /// through the same shared kernel, so the tail rule and the per-window
       284	    /// metering are one implementation across arms.
       285	    pub(crate) fn msb_cmp(a: &Num, b: &Num) -> Ordering {
       286	        match (a, b) {
       287	            (Num::Base(x), Num::Base(y)) => Base::msb_cmp(x, y),
       288	            (Num::Base(x), Num::Wide(y)) => msb_cmp_windows(x.msb_windows(), y.msb_windows()),

    (base.rs)
       104	    pub(crate) fn msb_cmp(a: &Base, b: &Base) -> Ordering {
       105	        msb_cmp_windows(a.msb_windows(), b.msb_windows())
       106	    }

Resolution: Delete `Base::msb_cmp`; spell the four arms uniformly and rewrite the doc: "every pairing streams both arms' MSB windows through msb_cmp_windows; the arms differ only in their limb source". Optionally collapse to one arm with a small two-variant `Iterator` over the limb source so `Num::msb_windows` has one return type. Acceptance: `grep -rn 'Base::msb_cmp' crates/before/src` is empty; `msb_cmp_matches_the_aligned_oracle` and `rank_wide_arm_cmp_agrees_with_the_alignment_oracle_on_10k_pairs` pass unchanged; the limb-meter reading of `Rank::cmp` on any base pair is unchanged.

Synthesis note: codec-base-text-tree-4 (this document) is the same wrapper seen from base.rs.

#### rank-27: Two sub-limb right-shift kernels in num.rs
- Where: crates/before/src/version/rank/num.rs:355-360 (related: num.rs:569-593, num.rs:297-311, num.rs:384-395, num.rs:449-462)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (both loops read: 358 `limbs[i] = (limbs[i] >> pad) | (high << (64 - pad));` and 588 `self.limbs.get(i + 1).copied().unwrap_or(0) << (64 - bit)`); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (both in 79a944ab)
- Owner-gated: no

`materialize_be` shifts an assembled limb vector right by `pad` bits in place; `Wide::shr_limbs` shifts by `n % 64` while copying the tail. Both are `limbs[i] = (limbs[i] >> k) | (limbs[i+1] << (64 - k))` with the same top-limb handling: one cursor discipline written twice, so a fix to one (an off-by-one at the top limb, a `k == 0` guard) must be found in the other. Alongside: the limb-to-byte flatten-and-strip is spelled three times (302-307, 384-395, 453-460).

Evidence:

       355	        if pad > 0 {
       356	            for i in 0..limbs.len() {
       357	                let high = limbs.get(i + 1).copied().unwrap_or(0);
       358	                limbs[i] = (limbs[i] >> pad) | (high << (64 - pad));
       359	            }
       360	        }

       583	        for i in whole..self.limbs.len() {
       584	            let low = self.limbs[i] >> bit;
       585	            let high = if bit == 0 {
       586	                0
       587	            } else {
       588	                self.limbs.get(i + 1).copied().unwrap_or(0) << (64 - bit)
       589	            };
       590	            limbs.push(low | high);
       591	        }

Resolution: One `fn shr_in_place(limbs: &mut [u64], bits: u32)` documented for `1..64` (caller guards zero): `materialize_be` calls it when `pad > 0`; `shr_limbs` becomes `let mut tail = self.limbs[whole..].to_vec(); if bit != 0 { shr_in_place(&mut tail, bit) }; tail` and lets `from_limbs` trim the top. Optionally one `fn le_bytes_minimal(limbs: &[u64]) -> Vec<u8>` for the three flattenings. Acceptance: one occurrence of the `<< (64 - ` combine pattern in num.rs; `materialize_is_exact_and_canonical` (all pads) and `shr_matches_the_oracle_and_redispatches` (amounts 0..512) pass unchanged; the wasm32 fraction pins pass unchanged.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| rank-2 | `crates/before/src/version/rank.rs:144-147` | Rank and Ranked tests live in the parent module's test file | Move the five sections to sibling `tests.rs` files; drop the re-export |
| rank-14 | `crates/before/src/version/rank.rs:596-599` | suanpan's private digit width is hardcoded as 32 in before | Export `DIGIT_BITS` or add `reserve_bits` in suanpan |
| rank-17 | `crates/before/src/version/rank.rs:629` | Idiom nits: ilog2, a redundant rename, an expect that is not a proof, qualified paths, asymmetric twins, a re-export rename | Apply as listed; `unwrap_or(0)` or a premise-stating expect |
| rank-28 | `crates/before/src/version/rank/num.rs:372-377` | Wide's documented invariant is violated by from_limbs's transient, which is the only reason Wide::bits has a zero arm | A `limb_bits` helper decides the arm; `Wide::bits` drops its zero arm |

### Span and causally

11 entries (1 medium, 5 low, 5 nit); the full record is `evidence/partitions/span-causally.md`. Related findings in other documents: span-causally-24 and span-causally-36 (claim: the multi-hole cost contract), span-causally-26 (performance: `sweep::le` in production).

#### span-causally-9: The receiver-seeded two-sided fold is written twice, with `FoldInput`, the group enum, and the adjacent-clone dedup re-declared in each file
- Where: crates/before/src/span/algebra.rs:349-424 (related: crates/before/src/span/algebra.rs:516-565; crates/before/src/version.rs:633-698, 796-843, 1215-1331; crates/before/src/fold.rs:1-3)
- Class / severity / confidence: modularity / medium / high
- Provenance: assessed (read both fold bodies side by side, both `FoldInput` enums, `Group`/`Hull`, and `DedupRuns`); executed: no
- Seen by: structure; refutation: confirmed (one correction: fold.rs's "one home" claim is about the counter, which both folds share; the duplicated layer is the group/receiver/dedup shape above it); history: no rationale found (`DedupRuns`, `FoldInput`, `Hull` landed 2026-07-30; `fold_endpoints` with its own filter and enums one day later without reusing them)
- Owner-gated: no

`fold_endpoints` and `Version::span_all` run the same balanced two-sided fold with the same four-arm match and the same commutativity comment; `algebra.rs` re-declares `FoldInput` and a two-sided `Group` that `version.rs` already has as `FoldInput` and `Hull`, and hand-rolls the adjacent-clone dedup filter that `version.rs` names, documents, and argues as `DedupRuns`. The safety argument `ptr_eq` dedup rests on (the filter holds a clone, so a freed allocation cannot be reused at the same address and masquerade as a duplicate) is stated only in `version.rs`; `algebra.rs` relies on it silently. `Version::span_all`'s leaf combine `span_refs` is exactly `union_points`, so its body is `fold_endpoints` under `UNION_OPS` with point items. A change to the fold shape must now be made in three places.

Evidence:

       354	        // The dedup filter: one (lo, hi) buffer-identity pair of state.
       355	        let mut last: Option<(Version, Version)> = None;
       356	        let inputs = core::iter::once(FoldInput::Receiver(self))
       357	            .chain(iter.into_iter().map(FoldInput::Item))
       358	            .filter(move |input| {
       359	                let s = input.span();
       360	                let dup = last.as_ref().is_some_and(|(lo, hi)| {
       361	                    lo.view().ptr_eq(s.lo().view()) && hi.view().ptr_eq(s.hi().view())
       362	                });

       400	                    // Unreachable through the counter's weight discipline (a
       401	                    // weight-0 lone input never sits below a merged group in
       402	                    // the closing drain), but the match stays total rather than
       403	                    // asserting: every leg kernel is commutative, so folding
       404	                    // the raw input into the owned group is value-identical.

    (version.rs, the same comment)
       675	                // Unreachable through the counter's weight discipline (a
       676	                // weight-0 lone input never sits below a merged group in the
       677	                // closing drain), but the match stays total rather than
       678	                // asserting: both sides' combiners are commutative, so folding
       679	                // the raw input into the owned hull is value-identical.

Resolution: give the two-sided fold one home. One shape: a private `Endpoints` trait (`lo()`, `hi()`, `point()`) implemented for `Span<'_>` and for a `Version`-wrapping newtype (lo = hi = the version, `point()` always `Some`); a shared `FoldInput<'r, T>` and `DedupRuns` keyed on `(&Version, &Version)` (the one-sided folds pass `|v| (v, v)`); one `fold_endpoints<T: Endpoints>(receiver, items, &SpanFoldOps)`. `Version::span_all` becomes a call into it with items wrapped, keeping its stable `Borrow<Version>` item convention; `Hull` and the second `FoldInput`/dedup dissolve. Acceptance: `grep -rn 'weight-0 lone input never sits below' crates/before/src` returns one production site (or two, if `balanced_fold` keeps its one-sided copy); `enum FoldInput` and the dedup adapter are each declared once; `span_all_is_the_family_hull`, `span_union_of_points_is_span_all`, and the n-ary span laws stay green; the `span_all`/`join_all` envelopes in tests/meter.rs are re-measured at the parent and unchanged (the change deletes no work and adds none).

Synthesis note: version-core-8 and version-core-11 (this document) are the version.rs side; the three entries describe one consolidation of the fold layer above `fold.rs`.

#### span-causally-3: The clone-identity coincidence certificate is spelled inline at five production sites while `Span::is_coincident` names it
- Where: crates/before/src/span.rs:255 (related: crates/before/src/span.rs:315, 380, 459, 553-555; crates/before/src/causally/query.rs:129; crates/before/src/span/algebra.rs:361, 562; crates/before/src/version.rs:1262)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep for `is_coincident\|\.ptr_eq(` over non-test production sources enumerates exactly the sites listed); executed: no
- Seen by: structure; refutation: confirmed (adds that `is_coincident` is private to `span.rs`, so `query.rs` cannot call it today); history: no rationale found (the inline sites predate the helper; ed1b3c8b added it for the algebra only)
- Owner-gated: no

The certificate is the pivot of every fast path in the partition, and every doc comment calls it one thing ("clone identity", "the coincident span's `O(1)` certificate"), yet `place`, `dominance`, `precedence`, the receiver test in `contains`, and `Query::coverage` each write `lo.view().ptr_eq(hi.view())` rather than calling the helper that exists for it. A reader hunting for "where does coincidence dispatch?" cannot grep for it by name. Legibility, and the small-scale form of the doubled-table problem (span-causally-12).

Evidence:

       255	        if self.lo.view().ptr_eq(self.hi.view()) {

       553	    fn is_coincident(&self) -> bool {
       554	        self.lo.view().ptr_eq(self.hi.view())
       555	    }

    (causally/query.rs)
       129	        if lo.view().ptr_eq(hi.view()) {

Resolution: make `is_coincident` `pub(crate)` and call it at span.rs:255, 315, 380, 459 and query.rs:129. For the version-pair sites (algebra.rs:361, 562; version.rs:1262) add a `pub(crate) fn Version::shares_buffer(&self, other: &Version) -> bool` so `is_coincident` is `self.lo.shares_buffer(&self.hi)` and `DedupRuns` reads `prev.shares_buffer(version)`. Acceptance: `grep -rn '\.view()\.ptr_eq(' crates/before/src --include='*.rs' | grep -v tests` returns only the helper bodies.

#### span-causally-11: `intersect_points` is the swapped hull: one `span_refs` call replaces the equality compare plus two emission walks, and its comment names a rescue that cannot happen
- Where: crates/before/src/span/algebra.rs:452-463 (related: crates/before/src/span/algebra.rs:193-202, 445-450; crates/before/src/version.rs:968-1008)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (re-derived rung by rung against `span_refs` at version.rs:968-1008); executed: no
- Seen by: structure, claims; refutation: confirmed (re-derived the swap on every rung; notes no committed traffic snapshot surrounds an intersect call, so nothing moves except the unmeasured cost); history: no rationale found (only the equal-pair byte compare was deliberate)
- Owner-gated: no

Intersect's point-combine wants `(a ∨ b, a ∧ b)`; `Version::span_refs(a, b)` returns `(a ∧ b, a ∨ b)` through a five-rung ladder (equal, empty-`a`, empty-`b`, comparable, concurrent). Swapping its output is value-identical on every rung: equal gives `(a, a)`; empty `a` gives `(b, 0)`; empty `b` gives `(a, 0)`; `a < b` gives `(b, a)`; concurrent gives `(join, meet)` from one fused `emit::hull` where the current code runs `join_refs` and `meet_refs` as two full walks. The comparable rung answers with two `O(1)` clones after one sweep. This is a strict deletion of redundant work (fixed sign). Separately, the parenthetical "(or a later combine absorbs)" reads as if a crossed pair could later un-cross; under `INTERSECT_OPS` `lo` only joins upward and `hi` only meets downward, so `lo' <= hi'` would imply `lo <= lo' <= hi' <= hi`: a crossed pair stays crossed to the closing `partial_cmp` at 199-202, which is the actual soundness argument for deferring the verdict, and it is stated nowhere.

Evidence:

       452	/// Intersection's point-combine: two points share a version exactly when they
       453	/// are equal.
       454	///
       455	/// One byte compare answers the only nonempty case; an unequal pair pays the
       456	/// per-leg walks whose crossed output the operator's final validation rejects (or a
       457	/// later combine absorbs).
       458	fn intersect_points(a: &Version, b: &Version) -> (Version, Version) {
       459	    if codec::canonical_eq(a.view(), b.view()) {
       460	        return (a.clone(), a.clone());
       461	    }
       462	    (Version::join_refs(a, b), Version::meet_refs(a, b))
       463	}

Resolution: `fn intersect_points(a: &Version, b: &Version) -> (Version, Version) { let (lo, hi) = Version::span_refs(a, b); (hi, lo) }`, with the doc stating the swap and replacing the parenthetical with the monotonicity argument ("a crossed pair stays crossed under further join-`lo`/meet-`hi` legs, so the closing `partial_cmp` decides for the whole family"), cited from `intersect_all`. Note `span_refs` records `hull_traffic` rungs; no committed intersect snapshot exists, so nothing committed moves. Acceptance: a case in the pointwise laws asserting `intersect_points(a, b) == { let (l, h) = span_refs(a, b); (h, l) }` over arbitrary pairs; `nary_doors_match_sequential_folds_on_a_mixed_family` and the intersect laws stay green.

#### span-causally-12: The span operators are encoded twice: `*_core` kernels re-implement the `SpanFoldOps` table with inline fast paths, and two ordering matches restate `Span::new`
- Where: crates/before/src/span/algebra.rs:568-635 (related: crates/before/src/span/algebra.rs:199-202, 375-382, 448-514; crates/before/src/span.rs:145-148; crates/before/src/version.rs:864-945)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read both encodings; `join_refs` and `clone()+join_view` have the same three short-circuits in the same order per version.rs:884-900, so the value is unchanged); executed: no
- Seen by: structure; refutation: reframed (the duplication stands and `Span::new(lo, hi).ok()` is a loss-free replacement for both ordering matches, but the intersect "divergence" is by design: the fold defers its verdict to the closing `partial_cmp`, so `intersect_points` must return a crossed pair where the binary kernel returns `None`; keep `intersect_core`'s point fast path); severity lowered to low; history: no rationale found (both transcriptions landed together in ed1b3c8b and were reworked together in 22cdfbe1)
- Owner-gated: no

The module doc states the operator table once as four leg assignments (lines 14-20); the code states it twice: `union_core`/`join_core`/`meet_core` hand-write the coincident fast path and the per-leg kernel pair that `UNION_OPS`/`JOIN_OPS`/`MEET_OPS` with `*_points` already encode for the fold, and `fold_endpoints`'s `(Group::Input, Group::Input)` arm plus the `point()` check is already the binary combine. The ordering match at 199-202 and 599-602 is `Span::new`'s body (span.rs:145-148) restated. A reader verifying `a + b == union_all([b])` must reconcile two spellings (`clone()+*_view` in the cores, `*_refs` in the fold) whose equivalence rests on version.rs's "keep in lockstep" ladders.

Evidence:

       568	fn union_core(a: &Span<'_>, b: &Span<'_>) -> Span<'static> {
       569	    if a.is_coincident() && b.is_coincident() {
       570	        // Two points' union is their hull: one fused pair walk (Version::span's
       571	        // ladder, fast paths and traffic accounting included) where the per-leg
       572	        // folds below would walk the same operand pair twice.
       573	        return a.lo().span(b.lo());
       574	    }
       575	    let mut lo = a.lo().clone(); // O(1): a stored version's clone shares its buffer
       576	    let mut hi = a.hi().clone();
       577	    lo.meet_view(b.lo().view());
       578	    hi.join_view(b.hi().view());

    (the fold's encoding of the same table)
       448	fn union_points(a: &Version, b: &Version) -> (Version, Version) {
       449	    Version::span_refs(a, b)
       450	}

    (the restated constructor, also at 599-602)
       199	        match lo.partial_cmp(&hi) {
       200	            Some(Ordering::Less | Ordering::Equal) => Some(Span::owned(lo, hi)),
       201	            Some(Ordering::Greater) | None => None,
       202	        }

Resolution: one private `fn combine(a: &Span<'_>, b: &Span<'_>, ops: &SpanFoldOps) -> (Version, Version)` that does `match (a.point(), b.point()) { (Some(va), Some(vb)) => (ops.points)(va, vb), _ => ((ops.lo_refs)(a.lo(), b.lo()), (ops.hi_refs)(a.hi(), b.hi())) }` (lifting `Group::point` for `Input` to a `Span::point` helper). `union_core`/`join_core`/`meet_core` become `Span::owned(combine(a, b, &OPS))`; `intersect_core` keeps its point fast path and otherwise becomes `let (lo, hi) = combine(a, b, &INTERSECT_OPS); Span::new(lo, hi).ok()`; `intersect_all`'s closing match becomes `Span::new(lo, hi).ok()`; `fold_endpoints`'s `(Input, Input)` arm calls the same `combine`. Acceptance: `Version::join_refs`/`meet_refs`/`span_refs` each appear once in algebra.rs (in the `*_OPS` constants or `*_points`); the two hand-written ordering matches are gone; the span algebra laws and `nary_doors_match_sequential_folds_on_a_mixed_family` stay green.

Synthesis note: Depends on version-core-11's collapse of the `_view` forms for the `clone()+*_view` versus `*_refs` reconciliation it names.

#### span-causally-15: `OwnSpan` hand-writes the nine-way and three-way verdict tables that `place.rs` and the `verdict.rs` docs already state
- Where: crates/before/src/span/own.rs:112-131 (related: crates/before/src/span/own.rs:166-181, 217-232, 254-262; crates/before/src/span.rs:256-261; crates/before/src/version/skyline/place.rs:224-239; crates/before/src/span/verdict.rs:25-85; crates/before/src/laws.rs:1675-1693)
- Class / severity / confidence: modularity / low / medium
- Provenance: assessed (read all transcriptions); executed: no
- Seen by: structure; refutation: confirmed (adds laws.rs:1675-1693 `place_from_relations` as a test-side fourth transcription); history: no rationale found; the duplication has already cost once (4d9641bd: a mutation probe found the hand-written `Concurrent(Start)` arm unpinned and a dedicated witness had to be added)
- Owner-gated: no

The rule mapping two relations to a `Placement` lives in `own.rs` (nine arms), in `place::span`'s closing closure (the same table in another arm order), as `Span::place`'s coincident diagonal, and as prose on the `Placement` variants; `dominance`, `precedence`, and `contains` are the same two-comparison transcription of the rules the `Dominance`/`Precedence` variant docs spell. A constructor on the verdict type puts the table where its documentation already is and makes the four `OwnSpan` methods one-liners whose correctness is the constructor's, not a per-method re-derivation. The test-side copy in laws.rs is the differential oracle and may deliberately stay independent.

Evidence:

       112	    pub fn place(&self, version: &Version) -> Placement {
       113	        let (lo, hi) = (self.lo(), self.hi());
       114	        match version.partial_cmp(&lo) {
       115	            Some(Ordering::Less) => Placement::Before,
       116	            Some(Ordering::Equal) => match version.partial_cmp(&hi) {
       117	                Some(Ordering::Equal) => Placement::At(Endpoint::Both),
       118	                _ => Placement::At(Endpoint::Start),
       119	            },

Resolution: `pub(crate) fn Placement::from_relations(vs_lo: Option<Ordering>, vs_hi: Option<Ordering>) -> Placement` (and `Dominance::from_relations`, `Precedence::from_relations`) in verdict.rs, stated once beside the variant docs, with `(None, None) => Concurrent(Both)` as the total definition (what own.rs returns today at line 127); `OwnSpan::{place,dominance,precedence}` call them; `Span::place`'s coincident rung calls `Placement::from_relations(r, r)`; `place::span`'s closure calls it after its `debug_assert`. Acceptance: one nine-arm `Placement` table in production code; `own_span_place_reaches_every_concurrent_corner`, `own_span_matches_the_projected_span`, `span_place_places_every_witness`, and `coincident_span_rungs_agree_across_buffer_identity` stay green.

#### span-causally-31: `Query` is built by sixteen struct literals across four files; the one-hole forms and the `Conjoin` lifts restate constructors that exist
- Where: crates/before/src/causally/forms.rs:288-298 (related: crates/before/src/causally/forms.rs:103-148, 312-358; crates/before/src/causally/conjunction.rs:108-128; crates/before/src/causally/convert.rs:13-34; crates/before/src/causally/query.rs:63-70, 200-208)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -c 'polarity: PhantomData'` gives forms 6, conjunction 3, convert 2, query 5: sixteen production sites); executed: no
- Seen by: structure; refutation: confirmed (corrects the count from fifteen to sixteen); history: no rationale found
- Owner-gated: no

The four one-hole constructors (`Floor::or_concurrent`, `Ceiling::or_concurrent`, `Not for Ceiling`, `Not for Floor`) write the identical eight-line literal differing only in `strict`; `strictly_after`/`strictly_before` add a bound to it; `Conjoin::lift` for `Floor`/`Ceiling` duplicates the `From<Floor>`/`From<Ceiling> for Query<Neutral>` impls, which `adopt()` already lifts to any polarity. The normal-form invariant "a neutral query holds no holes", which six `unreachable!` arms in polarity.rs rest on, is today enforced only by every literal happening to agree.

Evidence:

       288	    pub fn or_concurrent(self) -> Query<'a, Down> {
       289	        Query {
       290	            floor: None,
       291	            ceiling: None,
       292	            holes: vec![Hole {
       293	                at: self.at,
       294	                strict: true,
       295	            }],
       296	            polarity: PhantomData,
       297	        }
       298	    }

Resolution: private constructors in query.rs beside `unbounded()`: `fn from_hole(hole: Hole<'a>) -> Self` and `fn bounded(floor, ceiling) -> Self`; `or_concurrent` = `Query::from_hole(Hole { at: self.at, strict: true })`, `Not` = `Query::from_hole(Hole { at: self.at, strict: false })`, the strict forms set the bound on the result, and `Conjoin::lift` for `Floor`/`Ceiling` = `Query::<Neutral>::from(self).adopt()`. Acceptance: the `PhantomData` count drops to the actual constructors (unbounded, from_hole/bounded, clone, into_owned, adopt, and); `forms_keep_their_relations`, `conjunction_normalizes`, `debug_renders_expressions`, and the conjunction laws stay green.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| span-causally-6 | `crates/before/src/span.rs:605-627` | The `Cow<Version>` `From` impls live in `span.rs` and claim a span-only purpose, but `causally::forms` depends on them | Move both impls to version.rs; reword the doc |
| span-causally-10 | `crates/before/src/span/algebra.rs:356-369` | Inline qualified paths where imports exist, and a `core::` beside `std::` imports | Import `once` and `balanced_reduce` |
| span-causally-21 | `crates/before/src/span/wire.rs:44-48` | `Span::encode` builds the composite asymmetrically and reallocates once | `[lo.as_bytes(), hi.as_bytes()].concat()` |
| span-causally-23 | `crates/before/src/span/wire.rs:156-172` | The wire decode tests `Admission::Refuted` twice: an early return, then an `unreachable!` arm for the same variant | A total match on `Admission` at the decision point |
| span-causally-32 | `crates/before/src/causally/polarity.rs:19-22` | `Hole.strict` and the `hole_demand`/`hole_name` dispatch take a bare `bool` for a two-valued domain concept | `enum Bound { Inclusive, Strict }` |

## The skyline coding

### Coding: module root, admit, build, encode and decode, emit, literal, validate, text, shape, walk

12 entries (1 medium, 6 low, 5 nit); the full record is `evidence/partitions/skyline-coding.md`. Related findings in other documents: skyline-coding-9 (claim: the re-anchor quadratic), skyline-coding-29 (claim: the render class), skyline-coding-6 (verification-gap: the fused-door planted-pair proptest), skyline-coding-3 (documentation: test inventories in kernel docs), skyline-coding-14 (documentation: the `implementation` ghost).

#### skyline-coding-33: two strict skyline parsers implement the same canonical-form obligations
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

#### skyline-coding-4: module-root instrument surface: a 21-line module for one wrapped function, dual names per entry, an inconsistently gated re-export, a comment describing one of two gated items
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

Synthesis note: inventory-12 (this document) reaches the same meter-facing skyline surface (an unnameable `Base` in `min_ticks`'s signature, bare `pub` fns in a `pub(crate)` module).

#### skyline-coding-11: `held_at` outlived the runtime gate it was introduced for
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

#### skyline-coding-12: `continue_verbatim`'s seven positional arguments are spelled at four sites under two clippy allows
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

Synthesis note: skyline-fill-grow-18 (this document) is the caller-side view of the same seven-argument signature; one struct closes both.

#### skyline-coding-17: `emit` and `hull` are two copies of the emission driver
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

Synthesis note: version-core-13 (this document) removes the only production reader of `Hull.relation`; decide the field with this consolidation.

#### skyline-coding-28: the allocation A/B arm is compiled into the production renderer for a settled experiment
- Where: crates/before/src/version/skyline/text.rs:345-354 (related: crates/before/Cargo.toml:107; crates/before/benches/presize.rs:21; crates/before/benches/common/mod.rs:268-274; crates/before/src/version/skyline/query.rs:508-515; justfile:795-809; crates/before/src/version/skyline/text.rs:391-395)
- Class / severity / confidence: vestigial / low / medium
- Provenance: verified (grep `before_alloc_ab` locates every consumer as listed); executed: no
- Seen by: structure; refutation: confirmed; history: deliberate-and-holds (b28c35ad6 landed it as a record instrument and the site states its purpose), with the observation that the owner retired the sibling stacks seams with a DECIDED entry (1300ced09: "the seam existed to price the choice, and the choice is made") and no such entry exists for `display_growth`
- Owner-gated: yes: retiring an instrument, and a recorded design decision

Principle 3: machinery outlives the constraint that justified it. The shipped arm's exactness is asserted at text.rs:391-395, the bench records rather than checks, and the apparatus spans two production kernels, a check-cfg registration, a justfile recipe, and bench support code. Steelman: a re-runnable price of exact-request versus growth across allocators may be wanted; the arm is well fenced.

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

Synthesis note: deps-10 (this document) is the roster side of the same alloc-A/B apparatus; benches-examples-12 (verification-gap) is the presize record whose closure the benches-examples summary recommends. Retiring the arm and closing the record are one change.

#### skyline-coding-32: the path and phase stacks in validate, admit, and text ride the output build buffer instead of `BitStack`
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

Synthesis note: One of the ten sites codec-base-text-tree-12 (this document) migrates together; party-19 lists the party sites.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| skyline-coding-8 | `crates/before/src/version/skyline/admit.rs:296-298` | qualified paths where the import already exists; a one-line `version_of` alias copied nine times | Import at each site; delete the `version_of` copies |
| skyline-coding-10 | `crates/before/src/version/skyline/build.rs:137-140` | `Option<bool>` tests spelled through `map`/`unwrap_or` where `== Some(..)` reads directly | `== Some(false)` and `!= Some(true)` |
| skyline-coding-21 | `crates/before/src/version/skyline/literal.rs:53-58` | `literal::node` builds a temporary buffer only to iterate it; a rustfmt-displaced trailing comment | `iter::once(false).chain(..)`; move the comment above |
| skyline-coding-27 | `crates/before/src/version/skyline/text.rs:292-296` | `StoredLeft` is `Summary` minus `root`, copied field by field | `Summary { root, body: StoredLeft }` |
| skyline-coding-36 | `crates/before/src/version/skyline/walk.rs:178-190` | `Extremum`'s reset policy keys off direction while its contract is about buffer provenance | `Extremum::min()` owns a fresh register, or carry `Provenance` as a field |

### Fill and grow

17 entries (2 medium, 8 low, 7 nit); the full record is `evidence/partitions/skyline-fill-grow.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: skyline-fill-grow-32. Related findings in other documents: skyline-fill-grow-2 (claim: heap on the distinct-minima memo families), skyline-fill-grow-23 (correctness: the `u32` link index), skyline-fill-grow-21 (documentation: "mint"), skyline-fill-grow-17 (documentation: kernel-doc test citations).

#### skyline-fill-grow-12: fill.rs drives `Memo` and `PreScan` through `pub(super)` fields; the ledger's lifetime is prose in memo.rs and mechanism in fill.rs
- Where: crates/before/src/version/skyline/fill.rs:516-532 (related: crates/before/src/version/skyline/fill.rs:317-334, 676-689; crates/before/src/version/skyline/fill/memo.rs:45-57, 69-94; crates/before/src/version/skyline/fill/prescan.rs:60, 65-103, 287-299)
- Class / severity / confidence: modularity / medium / high
- Provenance: verified (read the field visibilities: memo.rs:76 `queue`, 82 `cursor`, 85 `covered_until`, 90/93 the checksums; prescan.rs:70 `web`, 102 `suspend`; the launch sequence and `consume_site`'s hand-advanced cursor; prescan.rs:60 already imports `REL_FOLLOWER`); executed: no
- Seen by: structure (2); refutation: confirmed; history: no-rationale-found (055f2e48's split replaced a 10-field struct literal with `PreScan::new` and left the launch sequence and the consume half in fill.rs; nothing records a reason for the field exposure)
- Owner-gated: no

The fresh-scan launch (519-531) sequences `begin_scan`, `PreScan::new`, `reserve`, `web.open(1)`, `run`, `record(slot, 0)`, `follower_take`/`retire`/`close` on the scan's web, an assert on `scan.suspend`, and `memo.covered_until = end` from outside prescan.rs; `consume_site` folds the checksum, takes the link, and advances `memo.cursor` by hand; `fused_fill`'s epilogue asserts on `memo.cursor`, `memo.queue.len()`, and both checksums. memo.rs titles the rule ("# Lifetime: one create, one consume") whose consume half fill.rs implements. Modules have a single clear responsibility; the invariant the walk's cost argument rests on is stated where it is not implemented.

Evidence:

       519	                        self.memo.begin_scan();
       520	                        let scan_start = self.pos();
       521	                        let mut scan = PreScan::new(self.event, scan_start, &mut self.memo);
       522	                        let slot = scan.reserve(scan_start);
       523	                        scan.web.open(1);
       524	                        let mut reader = IdReader::at(id.bits(), id.pos());
       525	                        let end = scan.run(&mut reader);
       526	                        scan.record(slot, 0);
       527	                        let relation = scan.web.follower_take(REL_FOLLOWER);
       528	                        scan.web.retire(relation);
       529	                        scan.web.close();
       530	                        debug_assert!(scan.suspend.is_empty(), "every suspended level resolves");
       531	                        self.memo.covered_until = end;
       688	        let link = self.memo.take_link(self.memo.cursor);
       689	        self.memo.cursor += 1;
    (memo.rs)
        45	//! # Lifetime: one create, one consume

Resolution: Give `Memo` the consume half: `consume(&mut self, pos: u64) -> Option<Accumulator>` (debug-assert `cursor < queue.len()`, fold `pos` into `consumed_check`, take the link, advance), `reserve(&mut self, pos: u64) -> usize` (moved from `PreScan::reserve`), `is_covered(&self, pos) -> bool` / `cover_until(&mut self, end)`, and `drained(&self) -> bool` for the epilogue asserts. Give `PreScan` one entry, e.g. `cover(event, start, id_bits, id_pos, &mut memo) -> u64`, that owns new/reserve/open/run/record/retire/close and the `suspend.is_empty()` assert. Then every `pub(super)` field on `Memo` and `PreScan` becomes private and `record`/`reserve` become private too. Behavior-preserving. Acceptance: no `pub(super)` field on `Memo` or `PreScan`; fill.rs's left-full arm calls one `PreScan` method and `consume_site` calls one `Memo` method; `just gate` clean.

#### skyline-fill-grow-27: `Frames`/`PreFrames` and `Frame`/`PreFrame` duplicate the suspended-ancestor control-bit discipline
- Where: crates/before/src/version/skyline/fill/prescan.rs:602-695 (related: crates/before/src/version/skyline/fill/prescan.rs:590-600; crates/before/src/version/skyline/fill.rs:1129-1140, 1179-1297)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (side-by-side read: `top()` at prescan.rs:634-647 and fill.rs:1220-1233 are byte-identical apart from the enum name, as are `aux_top`, the three-bit push in `push_node`/`push_site`, `flip_to_await_right`'s debug assert plus `set_last`, and the triple pop; `PreFrame` (592-600) and `Frame` (1130-1140) are the same three variants); executed: no
- Seen by: structure (0); refutation: confirmed (severity lowered to low: the shared portion is roughly 45 lines, the payload halves legitimately differ, and nothing forces the two stacks to agree); history: no-rationale-found (both stacks were born in 05bd2b16; 055f2e48 extracted the one shared idiom it noticed, `DeltaReg`, and left the control-bit triple spelled twice; prescan.rs:602-604 itself asserts the sameness)
- Owner-gated: no

The three control `BitStack`s and their `top`/`aux_top`/`push`/`flip`/`pop` discipline are one type spelled in two files that document themselves as twins; only the value-stack payload differs (route keys plus deferred costs versus ledger slot deltas). A change to the frame encoding must be made twice, with the invariants ("`phase` unread on site frames", one aux bit per frame) documented twice. I keep medium against the refutation's low: the extraction is clean, behavior-preserving (the same bits are pushed), and it is the partition's largest duplication; "nothing forces the two to agree" is the maintenance hazard, not a mitigation.

Evidence:

       602	/// The pre-scan's suspended ancestors, held as bits: the fill walk's stack
       603	/// shape with the site payload (its ledger slot) as a pop-able word delta in
       604	/// place of route keys and costs.
       634	    fn top(&self) -> Option<PreFrame> {
       635	        let site = self.site.last()?;
       636	        Some(if site {
       637	            PreFrame::Site
       638	        } else if self
       639	            .phase
       640	            .last()
       641	            .expect("site and phase stack one bit per frame")
    (fill.rs)
      1220	    fn top(&self) -> Option<Frame> {
      1221	        let site = self.site.last()?;
      1222	        Some(if site {
      1223	            Frame::Site
      1224	        } else if self
      1225	            .phase
      1226	            .last()
      1227	            .expect("site and phase stack one bit per frame")

Resolution: Extract the control bits into one type in fill.rs, e.g. `FrameBits { site, phase, aux: BitStack }` with `len`, `top() -> Option<Frame>`, `aux_top`, `push(site: bool, aux: bool)`, `flip_to_await_right`, `pop`, and one `Frame` enum. `Frames` composes it with `values: PopStack` plus `keys: DeltaReg` and keeps the cost encode/decode; `PreFrames` composes it with `values` plus `slots`. Acceptance: one `Frame` enum and one `top()` in the fill module; `PreFrame` and the duplicated bodies gone; `just gate` clean with no envelope movement.

Synthesis note: Severity stands at medium on the finalizer's argument: the extraction is behavior-preserving and it is the partition's largest duplication. skyline-fill-grow-25 (this document) is the design-proposal half that depends on this shared frame type landing first.

#### skyline-fill-grow-8: Output-delta anchoring is a bool plus an idle accumulator beside an enum-shaped sibling, and the anchor switch is spelled twice
- Where: crates/before/src/version/skyline/fill.rs:357-364 (related: crates/before/src/version/skyline/fill.rs:858-866, 924-931, 971-984, 394-414, 912, 1024)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read); executed: no
- Seen by: structure (6); refutation: confirmed (`emit_offset` folds the offset between the bridge and materialize, so a helper returns the accumulator pre-materialize); history: no-rationale-found (`Relation` gained its tag-in-struct/storage-in-web enum and stated invariant in 055f2e48 and a736ef14; `w_anchored`/`gap` were not revisited)
- Owner-gated: yes for the enum (it moves readings); the helper is adoptable now

`w_anchored: bool` tags whether `min − prev_out` rides `OUT_FOLLOWER` or `gap` holds `h − prev_out`, with `gap` documented as idle while the tag is set. `Relation` (394-414) models the same tag-in-struct/storage-in-web shape as an enum with its invariant stated. The watermark-to-height switch (`follower_take(OUT_FOLLOWER)`, `bridge_add_gap`, `w_anchored = false`) is spelled in `emit_step` and again in `emit_offset`. Two spellings of one state shape in one struct cost the reader a second model; an idle-while-tagged accumulator is an invariant the type could carry.

Evidence:

       357	    /// `h − prev_out` while the output delta is height-anchored: every consumed
       358	    /// step folds in, and every emitted leaf re-derives it. Idle (zero) while
       359	    /// `w_anchored`.
       360	    gap: Accumulator,
       361	    /// Whether the output delta is watermark-anchored: the last emission took
       362	    /// the tracked minimum, and `min − prev_out` rides the web's
       363	    /// [`OUT_FOLLOWER`] instead of `gap`.
       364	    w_anchored: bool,
       862	            let mut out_delta = self.web.follower_take(OUT_FOLLOWER);
       863	            self.web.bridge_add_gap(&mut out_delta);
       864	            self.w_anchored = false;
       927	                let mut out_delta = self.web.follower_take(OUT_FOLLOWER);
       928	                self.web.bridge_add_gap(&mut out_delta);
       929	                fold_signed_int(&mut out_delta, offset.sign, &offset.magnitude);
       930	                self.w_anchored = false;

Resolution: Now: `fn out_delta_from_min(&mut self) -> Accumulator` wrapping the three-line switch; `emit_step` materializes it, `emit_offset` folds the offset first. Proposal for the owner: `enum OutAnchor { Height(Accumulator), Min }` mirroring `Relation`, which removes the idle-gap sentence and the `debug_assert!(!self.w_anchored, ..)` at 912 and 1024; `emit_at_min`'s `mem::replace(&mut self.gap, fresh)` at 981 becomes a variant swap. Acceptance: one switch helper; if the enum is adopted, no `w_anchored` field and no idle-gap sentence.

#### skyline-fill-grow-9: Hand-maintained `depth` counters mirror O(1) stack lengths, justified by a recount that does not exist
- Where: crates/before/src/version/skyline/fill.rs:433-440 (related: crates/before/src/version/skyline/fill.rs:536, 545, 559, 569, 603; crates/before/src/version/skyline/grow.rs:516-519, 616-625; crates/before/src/codec/stack.rs:37-45; crates/before/src/codec/buf.rs:104-107)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (read `BitStack::len`, `words.len() as u64 * 64 + u64::from(self.top_len)`, and `BitsBuf::len`, `self.live`); executed: no
- Seen by: structure (5); refutation: confirmed (fill.rs's ascend arms read `depth + 1` after the decrement at 569 but before the pops, so the replacement there is `frames.len()`); history: no-rationale-found (the "never recounts" sentence was written in a736ef14 when `len` was already the O(1) multiply-add it is today)
- Owner-gated: no

The comment's reason names a cost the code has never had: `frames.len()` is `site.len()`, a multiply-add. The `debug_assert_eq!` exists only to check the mirror, and the mirror exists only for the phantom cost: circular justification. `grow::emit` carries the same shape against `pending.len()`. A derived counter maintained by hand is one more invariant to keep across the `+= 1`/`-= 1` sites and the width paragraph it needs.

Evidence:

       433	        // Derived state: always equal to `frames.len()` (the assert below),
       434	        // carried as a word so the hot loop never recounts a bit stack.
       438	        let mut depth = 0u64;
       440	            debug_assert_eq!(depth, frames.len(), "one frame per open branch level");
    (codec/stack.rs)
        43	    pub(crate) fn len(&self) -> u64 {
        44	        self.words.len() as u64 * 64 + u64::from(self.top_len)
        45	    }
    (grow.rs)
       621	    debug_assert_eq!(
       622	        path_depth,
       623	        pending.len(),
       624	        "one pending record per path level"
       625	    );

Resolution: grow.rs is the clean case: drop `depth`, use `pending.len() + 1` at 564 and `let path_depth = pending.len();` at 616, delete the assert. fill.rs: bind `let depth = frames.len();` at the head of the descend loop and `let depth = frames.len() - 1;` after the `web.close()` in each ascend iteration (the arms' `depth + 1` is then the pre-pop `frames.len()`); delete the counter, its updates, and the asserts at 440 and 559. Acceptance: no `depth` counter or sync assert in `FillWalk::walk` or `grow::emit`; tests green; no envelope movement.

Synthesis note: grow.rs's copy of the same counter dissolves with skyline-fill-grow-34's `IdReader` threading.

#### skyline-fill-grow-13: `consume_payload` inlines `fold_block`'s body
- Where: crates/before/src/version/skyline/fill.rs:640-674
- Class / severity / confidence: simplification / low / high
- Provenance: verified (side-by-side read of 648-655 and 666-673); executed: no
- Seen by: structure (4); refutation: confirmed (`consume_payload` already builds `Signed { sign, magnitude }` at 656); history: no-rationale-found (`fold_block` landed in 77d7da0b as the block scans' re-entry fold; nothing records leaving the per-leaf copy inline)
- Owner-gated: no

Lines 648-655 are statement-for-statement the body of `fold_block` (666-673) with `sign, &magnitude` in place of `net.sign, &net.magnitude`. The doc at 662-664 already states the two are the same fold; making it structural means a new height-carried register is added in one place.

Evidence:

       648	        fold_signed_int(&mut self.height, sign, &magnitude);
       649	        self.web.fold_height(sign, &magnitude);
       650	        if !self.w_anchored {
       651	            fold_signed_int(&mut self.gap, sign, &magnitude);
       652	        }
       653	        if let Relation::Height(relation) = &mut self.relation {
       654	            fold_signed_int(relation, sign, &magnitude);
       655	        }
       662	    /// Exactly what [`consume_payload`](Self::consume_payload) would have
       663	    /// folded leaf by leaf: nothing reads the registers between a block's

Resolution: `consume_payload`: decode, then `let step = Signed { sign, magnitude }; self.fold_block(&step); step`; restate `fold_block`'s doc as the primitive (`consume_payload` decodes and folds through it). Acceptance: one fold sequence in fill.rs.

#### skyline-fill-grow-16: The collapse-then-readout idiom is spelled inline four times; the first-leaf variant rebuilds `Signed` by hand
- Where: crates/before/src/version/skyline/fill.rs:913-920 (related: crates/before/src/version/skyline/fill.rs:870-872, 935-937, 1119-1120; crates/before/src/version/skyline/fill/prescan.rs:580-581; crates/before/src/version/skyline/watermark.rs:1140-1150; crates/before/src/version/skyline/signed.rs:105-113; crates/suanpan/src/accumulator.rs:704-721, 950-968)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep for a `.sign();` immediately preceding `sign_magnitude()` under skyline/ returns fill.rs:870, 913, 935 and watermark.rs:1146; read `Signed::from_sign_magnitude`, which maps every non-`Less` ordering to `Positive`; read suanpan's `sign` (amortized O(1), collapses a cancelling prefix) and `sign_magnitude` (O(held digits), no collapse)); executed: no
- Seen by: structure (7); refutation: confirmed; history: no-rationale-found (signed.rs was created with deliberately minimal helpers; the hand-built `Signed { sign: Sign::Positive, .. }` is residue of the `Sign` enum conversion in 4ac8fd70)
- Owner-gated: no

`acc.sign(); let (s, m) = acc.sign_magnitude(); Signed::from_sign_magnitude(s, m)` appears at 870-872, 935-937, and in `MinWeb::materialize` (watermark.rs:1146-1149); at 913-920 the same readout is followed by a hand-built `Signed` where `from_sign_magnitude` already yields `Positive` for a non-`Less` sign. signed.rs presents itself as the one home of the vocabulary every walk exchanges heights in; the reason for the collapse (watermark.rs:1144-1145) is written once there and nowhere at the fill.rs sites. On the structure lens's open question: the touch-meter claim depends on the collapse; correctness does not. `sign()` compacts a cancelling prefix at amortized O(1); `sign_magnitude()` reads O(held digits) without compacting, so without the collapse a readout can touch digits the value's width does not price, breaking "materialized once, post-collapse, at the width the code itself prices" (fill.rs:56). The block-net readouts at fill.rs:1119 and prescan.rs:580 skip the collapse; their reads are funded by the block scan that just read the same digits, so no cost claim breaks, but the helper should say so.

Evidence:

       913	            self.height.sign();
       914	            let (sign, magnitude) = self.height.sign_magnitude();
       915	            debug_assert_ne!(sign, Ordering::Less, "heights are nonnegative");
       916	            let value = Signed {
       917	                sign: Sign::Positive,
       918	                magnitude: Int::from_ubig(magnitude),
       919	            }
       920	            .sum(&offset);
    (signed.rs)
       108	    pub(super) fn from_sign_magnitude(sign: Ordering, magnitude: UBig) -> Self {
       109	        Signed {
       110	            sign: Sign::from_is_negative(sign == Ordering::Less),
       111	            magnitude: Int::from_ubig(magnitude),
    (watermark.rs)
      1144	        // Collapse for an honest width before the read-out: `sign()` is
      1145	        // called for its compaction side effect, the value unread.
      1146	        let _sign = dying.sign();
      1147	        let (sign, magnitude) = dying.sign_magnitude();

Resolution: Add `Signed::read(acc: &mut Accumulator) -> Signed` beside `from_sign_magnitude` (collapse via `sign()`, then `sign_magnitude`, then `from_sign_magnitude`), carrying the width note: the collapse bounds the O(held digits) readout to the value's width plus slack, so the read is priced by the code that emits it; a block net may skip it because its scan already paid for the held digits. Use it at fill.rs:870-872, 935-937, watermark.rs:1146-1149, and make 913-920 `Signed::read(&mut self.height).sum(&offset)` (the non-negativity assert at 921 covers the sum). Acceptance: no inline `sign(); sign_magnitude(); from_sign_magnitude` triple outside signed.rs; no hand-built `Signed { sign: Sign::Positive, .. }` from a readout.

#### skyline-fill-grow-18: `continue_verbatim`'s seven positional `u64`s are hand-marshalled from two different summary structs
- Where: crates/before/src/version/skyline/fill/fuse.rs:163-198 (related: crates/before/src/version/skyline/build.rs:233-243, crates/before/src/version/skyline/fill.rs:1060-1068, crates/before/src/version/skyline/grow.rs:392-400, crates/before/src/version/skyline/grow.rs:318-331)
- Class / severity / confidence: idiom / low / medium
- Provenance: verified (read both `#[allow(clippy::too_many_arguments)]` sites and both call sites); executed: no
- Seen by: structure (9); refutation: confirmed; history: no-rationale-found (the seventh argument arrived in e7d2548f; the allow comment rationalizes the count, not the positional shape)
- Owner-gated: no (the signature lives in build.rs, another partition; both call sites are here)

`Out::continue_verbatim` forwards seven positional arguments under a `too_many_arguments` allow, filled from a `RegionSkip` at fill.rs:1060-1068 and from a `Subtree` at grow.rs:392-400, each hand-marshalling `(start, end, root_depth, first_rel_depth, last_rel_depth, last_code_len)`. Types-first: six same-typed positional integers are a swap waiting to happen, and the allow acknowledges the shape; `RegionSkip` and `Subtree` are already the same product (a block summary of one subtree for the verbatim splice).

Evidence:

       173	    #[allow(clippy::too_many_arguments)] // (src, start, end) is one logical range argument
       174	    pub(super) fn continue_verbatim(
       175	        &mut self,
       176	        src: BitsView<'_>,
       177	        start: u64,
       178	        end: u64,
       179	        root_depth: u64,
       180	        first_rel_depth: u64,
       181	        last_rel_depth: u64,
       182	        last_code_len: u64,
    (grow.rs)
       392	        out.continue_verbatim(
       393	            event.bits,
       394	            subtree.first_code.end,
       395	            subtree.end,
       396	            depth,
       397	            subtree.first_rel_depth,
       398	            subtree.last_rel_depth,
       399	            subtree.last_code.end - subtree.last_code.start,
       400	        );

Resolution: Introduce a splice-range struct in build.rs (`start`, `end`, `first_rel_depth`, `last_rel_depth`, `last_code_len`, or a `Range<u64>` plus a leaf-coordinate pair); have `RegionSkip` and `Subtree` each produce one; `continue_verbatim(src, root_depth, splice)` drops both allows. Acceptance: no `too_many_arguments` allow on either `continue_verbatim`; both call sites pass one struct.

Synthesis note: skyline-coding-12 (this document) is the signature-side view; one struct closes both.

#### skyline-fill-grow-25: The paired id-times-event walk skeleton is spelled twice (`FillWalk::walk` and `PreScan::run`)
- Where: crates/before/src/version/skyline/fill/prescan.rs:155-285 (related: crates/before/src/version/skyline/fill.rs:431-623; crates/before/src/version/skyline/fill/prescan.rs:136-154)
- Class / severity / confidence: modularity / low / medium
- Provenance: assessed (read both walks arm for arm: the `id.read()` prologue with Empty/Full/Internal, the leaf arm with two conditional skips, the left-full peek and site push, the ordinary-node push with the absent-left inline copy, and the Site/AwaitLeft/AwaitRight ascend with the right-full peek and absent-right copy recur in both; only the leaf actions and the depth-vs-level counters differ); executed: no
- Seen by: structure (11); refutation: confirmed; history: no-rationale-found (the twin relationship is deliberate and stated; no commit, note, or comment records evaluating a shared driver)
- Owner-gated: yes (a design proposal that moves readings)

The pre-scan's correctness argument is "same reads, virtual emissions" (prescan.rs:137); today that sameness is prose and a careful diff, not structure, so a new arm or a reordered peek must be mirrored by hand. The steelman: the actions differ substantially (consume plus emit plus route fold plus memo consume versus virtual emission plus record), the two counters are different notions, and a visitor trait would need on the order of eight hooks. skyline-fill-grow-27 (the shared frame type) is the adoptable-now half.

Evidence:

       136	    /// The pre-scan image of the walk's arms over the subtree at the cursor:
       137	    /// same reads, virtual emissions; returns the range end.
       138	    ///
       139	    /// The iterative twin of the fill walk: the descend phase resolves the

Resolution: After the shared frame type lands, construct a `PairedWalk` driver parameterized by a visitor (hooks: empty region, full region, leaf under id node, left-full site open/close, ordinary node left/right/absent), measure the envelopes, and keep two spelled-out walks if the hook count makes the driver less legible than the twin files. Acceptance: either one driver with two visitors and the "same reads" claim structural, or a recorded decision to keep the twins with the frame types shared.

Synthesis note: Owner-gated design proposal; skyline-fill-grow-27 (this document) is the adoptable-now half.

#### skyline-fill-grow-34: grow.rs re-spells `IdReader`'s cursor as `id_tag`/`id_skip` over a bare position
- Where: crates/before/src/version/skyline/grow.rs:290-306 (related: crates/before/src/version/skyline/grow.rs:534-607; crates/before/src/version/skyline/grow/tests.rs:116-184; crates/before/src/idbits.rs:12-13, 92-96, 114-124, 145-156; crates/before/src/version/skyline/fill.rs:471)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`id_tag` is `IdReader::read`'s `record_bits(2)` plus two bit reads returning bools; `id_skip`'s closure is `IdReader::skip`'s closure verbatim; `grep -rn 'id_tag|id_skip|EvScan'` finds uses only under grow; `IdReader::at` exists at idbits.rs:94; fill.rs:471 derives the same route key from `id.pos() - 2`); executed: no
- Seen by: structure (1); refutation: confirmed (severity low: a dozen lines whose meter accounting the envelope rows already pin equal on both halves); history: deliberate-but-expired (183e3cab needed random access to re-read child presence at a suspended frame's stored key; 78206426 fused the probe into the fill walk, leaving `emit` and the test probe purely linear; `IdReader::at` predates grow.rs by seven weeks)
- Owner-gated: no

`emit` threads a bare `id_pos: u64` with manual `+= 2` where the fill walk threads an `IdReader` and reads `id.pos()`; the id tag reading and its scan-meter accounting are spelled in idbits.rs and again here. The reason for the raw spelling expired with the fused probe. `id_tag`'s doc also attributes to canonicity ("a canonical id has no `(0, 0)` node") what the coding makes unrepresentable (idbits.rs:12-13), the same slip as skyline-fill-grow-20; deleting the function removes it.

Evidence:

       292	/// Neither present is the full `1` terminal; a canonical id has no `(0, 0)`
       293	/// node. `O(1)` random access into the packed id.
       294	fn id_tag(bits: BitsView<'_>, pos: u64) -> (bool, bool) {
       295	    codec::scan::record_bits(2);
       296	    (bits.bit(pos), bits.bit(pos + 1))
       297	}
       300	fn id_skip(bits: BitsView<'_>, pos: u64) -> u64 {
       301	    crate::idbits::skip_subtree(pos, |at| {
       302	        codec::scan::record_bits(2);
       303	        let children = u64::from(bits.bit(at)) + u64::from(bits.bit(at + 1));
       304	        (children, at + 2)
    (idbits.rs)
       148	            *pos = skip_subtree(*pos, |at| {
       151	                crate::codec::scan::record_bits(2);
       152	                let children = u64::from(bits.bit(at)) + u64::from(bits.bit(at + 1));
       153	                (children, at + 2)

Resolution: Have `emit` and grow/tests.rs's `rec` take an `IdReader`: `let key = id.pos(); match id.read() { IdNode::Full => .., IdNode::Internal { left, right } => .., IdNode::Empty => unreachable!(..) }` and `id.skip()` in place of `id_pos = id_skip(id_bits, id_pos)`; delete `id_tag` and `id_skip`. The expansion-chain loop's `current = (key, left_present, right_present)` tuple threading (577-602) collapses to reading the tag at the loop head. Acceptance: `id_tag`/`id_skip` gone; the route differential and both grids green with `FAMILY_GROW_PAIRS`/`EXHAUSTIVE_GROW_PAIRS` unchanged; scan-meter tick envelopes unchanged (both spellings record 2 bits per tag).

Synthesis note: party-4 (this document) is the crate-wide consolidation of the same tag read; this entry's `IdReader` threading is its grow.rs instance.

#### skyline-fill-grow-35: `recode` spells its zero-crossing test two ways; both grow.rs mutants exclusions are the symptom
- Where: crates/before/src/version/skyline/grow.rs:436-476 (related: .cargo/mutants.toml:15-24, 131-142; crates/before/src/codec/base.rs:31-35, 284-289, 416-423)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read the two arms; read .cargo/mutants.toml:133-142's two `recode` exclusions and its header's disposition ladder; `Base::cmp`, `eq`, and `sub` are limb-metered via `meter_limbs2` while `Base::bits()` is not); executed: no
- Seen by: structure (3); refutation: confirmed (both equivalences hold; unifying applies the width prefilter to the down-delta arm too, which lowers limb readings on that path and needs attribution when re-pinned); history: split (the width prefilter is deliberate and its rationale holds inline, 4cc9b995: the k = 1 path stays byte-identical on every meter column; the two-spellings shape and the exclusions have no recorded step-(1) refactor attempt, which the roster's header requires before any entry)
- Owner-gated: no

The two crossing arms both compute `zigzag_signed(sign.negate(), events − magnitude)` and the non-crossing arm computes `magnitude − events` with a zero-to-positive normalization; the crossing test is a plain metered `<` in one arm and a width-prefiltered `<` in the other. .cargo/mutants.toml excludes two `recode` mutants as proven equivalent, both at the boundaries this redundancy creates. The roster's standing policy is to refactor the mutated codepoint out of structural existence first; a three-way `Ordering` match has no `<`/`>` operator to mutate, so both exclusions dissolve. The width prefilter must survive in one place because `Base::cmp` is limb-metered and `Base::bits()` is not.

Evidence:

       445	        (false, Sign::Positive) if magnitude < *events => {
       458	        (true, Sign::Negative) | (false, Sign::Positive) => {
       459	            let crosses = sign.is_negative()
       460	                && (magnitude.bits() < events.bits()
       461	                    || (magnitude.bits() == events.bits()
       462	                        && events.bits() > 1
       463	                        && magnitude < *events));
    (.cargo/mutants.toml)
       137	    "grow\\.rs.*: replace > with >= in recode",
       142	    "grow\\.rs:463:38: replace < with <= in recode",

Resolution: Two top-level arms, same-sign (`magnitude + events`) and opposing-sign; the latter `match width_first_cmp(&magnitude, events) { Less => zigzag_signed(sign.negate(), events.clone() - &magnitude), Equal => zigzag_signed(Sign::Positive, Base::ZERO), Greater => zigzag_signed(sign, magnitude - events) }`, where `width_first_cmp` compares `bits()` first and falls through to `cmp` only on a width tie. Keep the `magnitude.bits() == 0` shortcut inside the `Less` arm if the measured limb touches need it. Delete both `grow\.rs` entries from .cargo/mutants.toml. Acceptance: one spelling of the crossing test; both exclusions gone and `just mutants-list` clean; tick rows re-measured at the parent and at the change, any movement recorded as an attribution.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| skyline-fill-grow-6 | `crates/before/src/version/skyline/fill.rs:264-271` | Long qualified paths where the module is already imported | `use super::grow::{self, Cost, Route}` and the sibling imports |
| skyline-fill-grow-15 | `crates/before/src/version/skyline/fill.rs:898-900` | `let _ = matched;` after `debug_assert!(matched, ..)` is dead in both build profiles | Delete the two `let _ = matched;` lines |
| skyline-fill-grow-19 | `crates/before/src/version/skyline/fill/fuse.rs:320-325` | `if left { id.skip() } if right { id.skip() }` re-spells `IdReader::skip_present_children` | Keep the `IdNode` and call `skip_present_children` |
| skyline-fill-grow-26 | `crates/before/src/version/skyline/fill/prescan.rs:324-328` | `record`'s `while head_level > level` runs at most once | `debug_assert!` plus a single `if`, or state the one-level bound |
| skyline-fill-grow-33 | `crates/before/src/version/skyline/grow.rs:270-287` | `EvScan::skip` is a `#[cfg(test)]` method in the production file with one test consumer | Move `skip` into grow/tests.rs |
| skyline-fill-grow-37 | `crates/before/src/version/skyline/grow/tests.rs:59-72` | `assert_grow` drops the ticked stream, so two tests re-tick and re-inline the oracle comparison | `assert_grow(v, p) -> Option<BitsBuf>` |

### Comparison kernels: sweep, place, masked, overlay, signed

11 entries (2 medium, 3 low, 6 nit); the full record is `evidence/partitions/skyline-sweep-place-masked.md`.

#### skyline-sweep-place-masked-7: The arity-N advance law is stated twice: `advance_set` and `shape::advance_refinement`
- Where: crates/before/src/version/skyline/overlay.rs:12-17 (related: crates/before/src/version/skyline/shape.rs:102-116, 176-199; crates/before/src/version/skyline/overlay.rs:268-286; crates/before/src/version/skyline/admit.rs:340-395)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (grep of the tie-assert message `tied boundaries close to one shared flip level` hits overlay.rs, shape.rs, and admit.rs; the two bodies read side by side); executed: no
- Seen by: structure; refutation: confirmed (with the implementation note that `advance_set(set: &mut impl CursorSet)` has an implicit `Sized` bound, so a slice impl needs `?Sized` or a thin wrapper); history: deliberate but expired (true at c6ba2208/e30d659d; shape.rs's restatement landed twelve days later in 46eb64f9 with no reason for not reusing `CursorSet`)
- Owner-gated: no

The module doc says the law lives in exactly two generic faces, but `shape::advance_refinement` (shape.rs:176-199) is a full second statement of the arity-N body: the same strict first-maximum pick with `is_none_or(|(_, max)| depth > max)`, the same tied-step loop, the same `debug_assert_eq!`. Its trait `Refine` (`depth`, `done`, `advance -> flip`) is `CursorSet`'s per-slot vocabulary under another name, and shape.rs:171-175 says its `done()` guards never decide the pick or the tie, so nothing in the body needs to differ. (`admit.rs`'s binary copy carries its own inline reason: fallible steps.) A duplicated advance discipline is the brief's named cost, and the "exactly two" claim is inaccurate as written.

Evidence:

        12	//! boundary carrying the cursor's own crossing payload. The overlay-advance law
        13	//! is stated (and debug-asserted) in exactly two generic faces — the binary
        14	//! [`advance`], which hands each crossing to the caller's fold, and the N-ary
        15	//! [`advance_set`] over a walk's whole [`CursorSet`], which folds crossings
        16	//! inside each slot's step; the boundary bookkeeping below is their shared
        17	//! correctness argument. Above them sit the two cursor instances.

    shape.rs
       191	    let flip = walks[deepest].advance();
       192	    for (slot, walk) in walks.iter_mut().enumerate() {
       193	        if slot != deepest && !walk.done() && walk.depth() >= flip {
       194	            let tied = walk.advance();
       195	            debug_assert_eq!(tied, flip, "tied boundaries close to one shared flip level");
       196	        }
       197	    }

Resolution: implement `CursorSet` for a slice of `Refine` walks (priority `0..len`, `depth(slot) = self[slot].depth()`, `step(slot) = self[slot].advance()`; give `advance_set` a `?Sized` bound or wrap the slice) and reduce `advance_refinement` to the all-done check followed by `advance_set`; or dissolve `Refine` into `CursorSet` outright. Then either make "exactly two" true by construction or drop the count and name the faces, and say that `admit.rs` restates the binary law for fallibility. Acceptance: `grep -rn 'tied boundaries close to one shared flip level' crates/before/src` hits overlay.rs and admit.rs only; the `shape` snapshots and `combine` tests are unchanged.

Synthesis note: skyline-sweep-place-masked-15 (this document) is the pair-seeding copy in the same layer; crate-root-39 (this document) notes the four public iterators' `size_hint` copies that a shared `advance_refinement` would also absorb.

#### skyline-sweep-place-masked-15: Pair-tracking state and its seeding are spelled at four production sites while `OpenedPair` claims one home
- Where: crates/before/src/version/skyline/place.rs:131-170 (related: crates/before/src/version/skyline/place/filter.rs:91-117; crates/before/src/version/skyline/overlay.rs:670-713; crates/before/src/version/skyline/admit.rs:292-298)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (all four seeding sites read: overlay.rs:703-705, place.rs:152-154, filter.rs:99-101, admit.rs:296-298; `read`/`relation` bodies in place.rs:163-170 and filter.rs:109-116 are byte-identical); executed: no
- Seen by: structure, correctness; refutation: confirmed; history: deliberate but expired (040fdfff's "one home" was true at 17:26 on 2026-07-29 and false by 19:32 when 0cbb9dc7 landed place.rs; the sibling `Directions` got its one home the next day for the same pattern)
- Owner-gated: no

`BoundSide` in place.rs carries `{diff, directions}` with `read` and `relation`; filter.rs's `Pair` is the identical pair of fields with identical bodies. The seed move (`Accumulator::new()`, fold `+a_first`, fold `-b_first`) is written out at overlay.rs:703-705, place.rs:152-154, filter.rs:99-101, and admit.rs:296-298, though overlay.rs:674-676 says the orientation "has one home". The seeding orientation is exactly the invariant the touch-meter identities rest on, so it should have one spelling.

Evidence:

       150	    fn open(bits: BitsView<'a>, probe_first: &Int) -> BoundSide<'a> {
       151	        let (cursor, first) = LeafCursor::open(bits);
       152	        let mut diff = Accumulator::new();
       153	        super::signed::fold_signed_int(&mut diff, Sign::Positive, probe_first);
       154	        super::signed::fold_signed_int(&mut diff, Sign::Negative, &first);
       ...
       162	    /// Fold this interval's sign into the surviving directions.
       163	    fn read(&mut self) {
       164	        self.directions.fold(self.diff.sign());
       165	    }
       166	
       167	    /// The relation the completed sweep decided, as the causal order.
       168	    fn relation(&self) -> Option<Ordering> {
       169	        self.directions.relation()
       170	    }

    overlay.rs
       674	/// The shared opening move of every two-skyline walk, stated once so the
       675	/// seeding's orientation — `a` positive, `b` negative, the orientation [`fold`]
       676	/// applies to every later crossing — has one home. The opening heights ride

Resolution: hoist filter's `Pair` to sit beside `Directions` in sweep.rs (or beside `OpenedPair` in overlay.rs) with `Pair::open(a_first, b_first)`, `read`, `relation`; `OpenedPair::open` seeds through the same constructor (or a shared `seed_diff`), place's `BoundSide` becomes `{cursor, pair}`, and admit's seed calls the shared function. Optionally `sweep::sweep` and `masked::Walk::run` hold a `Pair` instead of parallel `diff`/`directions` locals. Acceptance: one production site folds `Sign::Negative` into a fresh difference; the placement identity rows in tests/meter.rs are unchanged (the write sequence is identical).

Synthesis note: skyline-sweep-place-masked-7 (this document) is the arity-N restatement in the same overlay layer; both make `overlay.rs`'s "one home" claims true.

#### skyline-sweep-place-masked-12: Two structs named `IdLeafCursor` share a step body: overlay.rs and party/ops/diff.rs
- Where: crates/before/src/version/skyline/overlay.rs:583-614 (related: crates/before/src/party/ops/diff.rs:245-268, 381-432; crates/before/src/version/skyline/overlay.rs:460-485, 531-552)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'struct IdLeafCursor' crates/before/src` returns overlay.rs:471 and diff.rs:245; both `impl PlateauCursor` read); executed: no
- Seen by: structure; refutation: confirmed; history: already known (the fold-unification survey scheduled the merge as optional Phase C with a drop condition; it never ran; the rename has no record)
- Owner-gated: no

`diff.rs:245` declares a private `struct IdLeafCursor<'a>` implementing `PlateauCursor` with `Crossing = bool`; its `step` (diff.rs:414-432) is the same pop-flip loop as overlay's (same match arms, near-identical `unreachable!` text, the same `lefts`/`open_lefts -= 1`, push-true, read the right-present flag), diverging only in the settle semantics and the stack type (`BitsBuf` vs `BitStack`). `grep IdLeafCursor` yields two definitions with different fields, and a fix to the id-side flip bookkeeping lands in one.

Evidence:

       583	    fn step(&mut self) -> (u64, ()) {
       584	        loop {
       585	            match self.path.pop() {
       586	                Some(true) => {
       587	                    self.right_present.pop();
       588	                    continue;
       589	                }
       ...
       593	                Some(false) => break,
       594	                None => unreachable!(
       595	                    "the advanced cursor is never at its final region: an all-right path means the stream is consumed"
       596	                ),
       597	            }
       598	        }
       599	        self.lefts -= 1;
       600	        self.path.push(true);
       601	        let flip = self.path.len();

    diff.rs
       414	    fn step(&mut self) -> (u64, bool) {
       415	        loop {
       416	            match self.path.pop() {
       417	                Some(true) => continue, // this ancestor closed with the item
       418	                Some(false) => break,   // the flip level: its right slot is next
       419	                None => unreachable!(
       420	                    "the advanced cursor is never at its final item: an all-right path means the tiling is consumed"
       421	                ),
       422	            }
       423	        }
       424	        self.open_lefts -= 1;
       425	        self.path.push(true);
       426	        let flip = self.depth();

Resolution: at minimum rename diff.rs's cursor (for example `SpliceCursor`) so `IdLeafCursor` names one thing, and have overlay.rs's doc name it as the splicing sibling; the composition (diff's cursor over overlay's, adding the `Item`/unsettled layer) is a cross-partition proposal for the party sweep. Acceptance: `grep -rn 'struct IdLeafCursor' crates/before/src` returns one definition; party diff tests and the `party::ops` meter rows unchanged.

#### skyline-sweep-place-masked-17: `walk`'s `finish` takes `Option<Option<Ordering>>` that every arm flattens; `Some(None)` is unreachable
- Where: crates/before/src/version/skyline/place.rs:437-450 (related: crates/before/src/version/skyline/place.rs:224, 291, 295, 354, 358, 411-412, 489-492)
- Class / severity / confidence: idiom / low / high
- Provenance: assessed (read every hook and finish arm); executed: no
- Seen by: structure; refutation: confirmed (hook analysis: `span`'s `on_side` drops or breaks on a double refutation; dominance, precedence, and contains break or drop the moment their single watched direction is refuted, so a surviving side always has `le || ge`); history: deliberate but expired (e2f4e2a5 kept the arm total for non-canonical input; aac1bc04 and b0194c6a established that the hooks rule the corner out by control flow on any input; the obligation paragraph was added by 248d5539 to justify the flattening)
- Owner-gated: no

`finish` receives `Option<Option<Ordering>>` per side (`None` dropped, `Some(None)` swept to concurrent). Every finish arm calls `.flatten()` on both sides before reading, and the doc itself argues `Some(None)` is deliverable only by hooks that leave a refutation standing, which no entry point's hooks do. The nested type then needs an "Obligation on every caller" paragraph to make the flattening sound: a type wider than its reachable states, with seven flatten calls and a paragraph compensating.

Evidence:

       437	/// Obligation on every caller: a `finish` arm that reads a side through
       438	/// `flatten` merges "dropped" with "swept to concurrent", so the side's drop
       439	/// condition must agree with that arm's reading — the hook may drop a side only
       440	/// when the direction the finish arm tests is already refuted, making the
       441	/// flattened `None` and the decided relation give the same answer. Each entry
       442	/// point carries the per-verdict argument at its closures.
       ...
       449	    finish: impl FnOnce(Option<Option<Ordering>>, Option<Option<Ordering>>) -> V,

Resolution: change the signature to `finish: impl FnOnce(Option<Ordering>, Option<Ordering>) -> V`, pass `set.start.as_ref().and_then(BoundSide::relation)` (and the end twin), delete the `.flatten()` calls (the existing `debug_assert!`s work unchanged on the flattened values), and replace the obligation paragraph with one sentence: a dropped side reads `None`, the same as a swept concurrency, and hooks drop only once the direction their finish arm tests is refuted. Acceptance: `grep -c '\.flatten()' place.rs` drops to 1 (the iterator flatten at 555); `span_walks_match_the_composed_sweeps` and the witness tests pass.

#### skyline-sweep-place-masked-35: `sweep::le` and `sweep::concurrent` have no caller outside their own tests, and `le`'s opener names wiring that does not exist
- Where: crates/before/src/version/skyline/sweep.rs:151-186 (related: crates/before/src/version/skyline/sweep.rs:7, 41-44, 57; crates/before/src/version/skyline/sweep/tests.rs:8-10, 67-81; crates/before/tests/meter.rs:5045; crates/before/src/version/skyline.rs:191; crates/before/src/version.rs:26-27; crates/before/src/version.rs:236)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep over crates/before/{src,tests,benches,examples,fuzz,fuzzfit,wasm32-pins,surfacecheck}, crates/before-fuelscape, and the rumors crate: `sweep::le`/`sweep::concurrent` appear only in sweep/tests.rs; the only non-test caller of a gated entry is `meter::skyline::sweep::eq` at tests/meter.rs:5045; `git log -S'sweep::le('` and `-S'sweep::concurrent('` return nothing; skyline.rs:191 `pub mod sweep;` under version.rs:26-27 `#[cfg(any(test, feature = "meter"))] pub mod skyline;`); executed: no
- Seen by: structure, correctness; refutation: confirmed; history: deliberate and holds for the four-entry-point differential design (sweep/tests.rs:8-10 states the rationale: "Every assertion runs all four entry points, so a bookkeeping error that misreads a direction ... has four chances to separate from the oracle"); the `meter` half of the cfg is 30759af0's blanket gating with no consumer for these two, and `le`'s opener is unexamined
- Owner-gated: yes (the items sit on the `meter` feature's public surface, and deleting them reverses a recorded differential design)

`concurrent` is a one-line wrapper on `causal_cmp` (it adds no chance of separating from the oracle beyond `causal_cmp` itself); `le` is a distinct sweep whose only exerciser is the test proving it correct. Both are `pub fn` gated `#[cfg(any(test, feature = "meter"))]`, so under `--features meter` they are public instrument surface with no consumer (Principle 3). `le`'s doc calls it "the fold behind causal containment checks", but no containment check calls it: `Version::concurrent` (version.rs:236) and every containment door go through `PartialOrd` and `causal_cmp`, as the same doc's last paragraph admits.

Evidence:

       151	#[cfg(any(test, feature = "meter"))]
       152	pub fn concurrent(a: BitsView<'_>, b: BitsView<'_>) -> bool {
       153	    causal_cmp(a, b).is_none()
       154	}
       ...
       159	/// The single-direction fold behind causal containment checks: stops at the
       160	/// first elementary interval where `a`'s height exceeds `b`'s.
       ...
       169	#[cfg(any(test, feature = "meter"))]
       170	pub fn le(a: BitsView<'_>, b: BitsView<'_>) -> bool {

Resolution: at minimum narrow `le` and `concurrent` to `#[cfg(test)]` and reword `le`'s opener to name it as the test-only single-direction exit predicate the differential suite exercises. If the owner prefers, delete both, drop their assertions from `assert_verdicts`, `exhaustive_small_scope_agrees`, and `organic_histories_agree`, and restate the module doc's entry-point list for `causal_cmp` and `eq` (`eq` stays: tests/meter.rs:5045 is its caller). Acceptance: `grep -rn 'sweep::le\|sweep::concurrent' tests benches examples` is empty and the two items are `#[cfg(test)]` or gone; `le`'s doc describes what it is.

Synthesis note: span-causally-26 (performance) proposes the opposite direction, lifting `sweep::le` into production as a one-direction exit; the two entries are one owner decision, and the open question on whether the `meter`-feature `pub mod skyline` is stable API frames it.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| skyline-sweep-place-masked-2 | `crates/before/src/version/skyline/masked.rs:158-164` | `masked::Walk` carries two correlated `Option`s re-derived by `expect`, and its A/B arms are mirror copies | A `Mask` struct pairing cursor and height; factor the mirror arms |
| skyline-sweep-place-masked-18 | `crates/before/src/version/skyline/place.rs:447-448` | Verdict hooks take a `bool` that three of four callers ignore | A named two-variant enum, or adapt the three hooks to `Fn(Directions)` |
| skyline-sweep-place-masked-22 | `crates/before/src/version/skyline/place/filter.rs:98-101` | `fold_signed_int` is called by qualified path beside imports from the same module | `use super::signed::{fold_signed_int, Sign};` |
| skyline-sweep-place-masked-23 | `crates/before/src/version/skyline/place/filter.rs:140-147` | `filter::BoundSide` sits one module below `place::BoundSide` with a different shape | Rename filter's struct `DemandSide` |
| skyline-sweep-place-masked-27 | `crates/before/src/version/skyline/place/tests.rs:14-15` | Test idiom: redundant parentheses (41 sites) and an unimported qualified helper (17 sites) | Drop the parentheses; import `built_view` |
| skyline-sweep-place-masked-31 | `crates/before/src/version/skyline/signed.rs:259-276` | `gamma_code_signed_int` duplicates `gamma_code_signed`'s fused fast-path body | One `small_signed_code(sign, mag)` helper |

### Query

10 entries (5 low, 5 nit); the full record is `evidence/partitions/skyline-query.md`. Related findings in other documents: skyline-query-31 (test-quality: the adequacy kernels' "verbatim" copies), skyline-query-9 (verification-gap: the `O(M(|v|) · log |v|)` clause).

#### skyline-query-5: The freeze trigger predicate is spelled twice, in `min_ticks` and `Integrator::boundary`
- Where: crates/before/src/version/skyline/query.rs:456-458 (related: query.rs:180; query/integral.rs:81-98, 264-274, 846-850; query/tests.rs:1443, 1587, 1924)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read both predicates; `FREEZE_ALLOWANCE_DIGITS` is imported into query.rs at line 180 and used only at 456); executed: no
- Seen by: structure; refutation: confirmed; history: no-rationale-found (both spellings landed together in 24a448cf; the mutants roster records a performance-genre disposition for `Integrator::boundary`'s `>` leg only, with no twin entry for query.rs:456)
- Owner-gated: no

The integral module doc describes one freeze trigger and calls its engagement boundary "a tuning choice inside the deliberate cost allowance"; the predicate is implemented independently in `min_ticks` and `Integrator::boundary`, so a tuning can move one and not the other, and nothing pins them equal. The mutants roster already treats the two copies asymmetrically.

Evidence:

    456          if live.digit_count() > int_digits(&step.magnitude) + FREEZE_ALLOWANCE_DIGITS {
    457              ledger.freeze(&mut live);
    458          }

    (integral.rs:847)          if self.live.digit_count() > funded_digits + FREEZE_ALLOWANCE_DIGITS {

Resolution: Add `pub(super) fn freeze_due(live: &Accumulator, funded_digits: usize) -> bool` beside the constant in integral.rs; call it from `Integrator::boundary`, `min_ticks`, and the three test kernels; stop exporting `FREEZE_ALLOWANCE_DIGITS` to query.rs. Acceptance: `grep -n 'FREEZE_ALLOWANCE_DIGITS' crates/before/src/version/skyline/query.rs` is empty and the predicate expression appears once in production code.

#### skyline-query-6: `ReignWeb::leaf` relies on a two-call protocol at each call site that the callee could own
- Where: crates/before/src/version/skyline/query.rs:463-470 (related: query.rs:438-439; query/web.rs:282-296, 374-377)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read both leaf sites and `ReignWeb::leaf`; the `epoch` argument is always the ledger's current epoch and `leaf` already receives `&mut ledger`); executed: no
- Seen by: structure; refutation: confirmed; history: no-rationale-found (the pairing dates to 24a448cf and survived the c29bd2b3 rebuild without comment)
- Owner-gated: no

Both leaf sites call `ledger.leaf_ref()` immediately before `web.leaf(...)` and pass an `epoch` that is always `ledger.epoch()` while also passing `&mut ledger`. The invariant "every leaf counts one reference in the epoch it is recorded under" is held by pairing two calls at each site instead of inside the one method that has both operands.

Evidence:

    463          ledger.leaf_ref();
    464          web.leaf(
    465              leaf_sign,
    466              &leaf_offset,
    467              ledger.epoch(),
    468              &mut total,
    469              &mut ledger,
    470          );

Resolution: Change `ReignWeb::leaf` to `(&mut self, sign, offset, total, ledger)`; inside, `ledger.leaf_ref(); let epoch = ledger.epoch();` before the closures capture the ledger; make `EpochLedger::leaf_ref` private; the first-leaf call becomes `web.leaf(Sign::Positive, &Base::ZERO, &mut total, &mut ledger)`. Acceptance: `grep -n 'leaf_ref' crates/before/src/version/skyline/query.rs` is empty; `assert_single`'s min_ticks legs stay green.

#### skyline-query-15: `Integrator.one: Base` stands in for suanpan's `add_u64_shl`, which `before` never calls
- Where: crates/before/src/version/skyline/query/integral.rs:578-581 (related: query/integral.rs:771-783, 806; suanpan/src/accumulator.rs:418-424, 1194-1196, 1493; codec/base.rs:42-44, 275-277; query/tests.rs:1431, 1437, 1544, 1568, 1880, 1905)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn 'add_u64_shl\|sub_u64_shl' crates/before` returns nothing); assessed (read the dispatch: `add_magnitude_shl` maps `to_word() == Some(word)` to `add_shifted_word(word, false, shift)`, the same call `add_u64_shl` makes; `Base::to_word` is `to_u64`, which records no limb count); executed: no
- Seen by: structure, claims; refutation: confirmed; history: deliberate-but-expired (the field predates `add_u64_shl` by eight days; a736ef14 wrote its current doc after the entry point existed without weighing it)
- Owner-gated: no

The field exists so `interval` can borrow a ready `&Base` for the unit deposit. suanpan's `Accumulator::add_u64_shl(1, weight_shift)` is value- and touch-identical (same `add_shifted_word` path, unmetered `to_word`), so the field, its doc, and its initializer vanish; with `one` gone every field has a `Default` and `Integrator::new` can derive. The three test integrators copy the same field.

Evidence:

    578      /// The unit mass every interval deposits at its own scale; a constant,
    579      /// held on the struct so the per-interval deposit borrows a ready
    580      /// `&Base` instead of building one per interval.
    581      one: Base,

    806              self.segment_mass.add_magnitude_shl(&self.one, weight_shift);

Resolution: Replace line 806 with `self.segment_mass.add_u64_shl(1, weight_shift);`, delete the field and its initializer, `#[derive(Default)]` the struct (keep `new()` as `Self::default()` or drop it), and apply the same to the test kernels' `position`/`segment_mass` deposits. Acceptance: `grep -rn 'one: Base' crates/before/src/version/skyline/query*` is empty; `just test-all` green with every `skyline_rank_*`, `DISTANCE_*`, and `LAG_*` envelope row unchanged.

#### skyline-query-21: `mul_into` carries a limb-metered zero guard its sibling refuses, a shift parameter that is zero at every production call, a collected `Vec<u32>`, and a `bool` whose keep ruling lives only in history
- Where: crates/before/src/version/skyline/query/web.rs:121-135 (related: query/web.rs:93-102, 104-120, 208-214, 409-415; query/integral.rs:472-478; codec/base.rs:33-35, 248-252, 432-436; query/tests.rs:1447, 1627, 1645, 1660, 1965, 1983, 2560)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep of `mul_into(` call sites: both production callers pass `0` as `shift`; `git show aa7c96a0` and `git log -1 4ac8fd70` messages read); assessed (read `impl PartialEq for Base`, which calls `meter_limbs2`, and `Base::bits`, which is O(1) and unmetered); executed: no
- Seen by: structure, correctness, claims; refutation: confirmed; history: three verdicts: keeping `mul_into` as a distinct word-scale kernel is deliberate and stated inline (web.rs:115-120); `subtract: bool` is an explicit owner keep recorded only in 4ac8fd70 ("Deliberate keep-as-bool ... operation selector"); the factor zero guard was kept on purpose by aa7c96a0, which in the same commit dissolved `charge_digits`' guard on the ground that `Base` equality is metered width-scale work, without weighing that cost here; the `shift` parameter's caller moved to `charge_segment` in 016b91c4 and the parameter stayed
- Owner-gated: yes: the guard and the bool are recorded owner rulings

Four items on one function. (1) `if *factor == Base::ZERO` runs through `Base::eq`, which records both operands' limbs, so every reign settle and epoch settle records a factor-width compare into the limb column the min_ticks bands judge; the sibling kernel's comment at integral.rs:475-478 declines the identical guard for exactly that reason. `Base::bits() == 0` answers in O(1), unmetered. (2) Both production callers pass `shift` 0; only the test kernels pass a scale, and the doc's "a segment mass parked deep in the stream" describes those kernels, not any production charge. (3) `u32_digits` collects a `Vec<u32>` the loop then only iterates; the `Limbs` `flat_map` iterator suffices. (4) `subtract: bool` is an owner-ruled operation selector, but the ruling lives only in a commit message, so the site reads as a `Sign` that was not converted.

Evidence:

    121  pub(super) fn mul_into(
    122      total: &mut Accumulator,
    123      factor: &Base,
    124      digits: &Base,
    125      shift: u64,
    126      subtract: bool,
    127  ) {
    133      if *factor == Base::ZERO {
    134          return;
    135      }

    (integral.rs:475-478)
    475      // zero-valued factors at the sign reads that price them. A guard here
    476      // would itself be metered width-scale work: `Base` equality records its
    477      // operands' limbs, so a per-charge zero test taxes every settle by the
    478      // factor's width.
    (base.rs:248-252)
    248  impl PartialEq for Base {
    249      fn eq(&self, other: &Self) -> bool {
    250          meter_limbs2(self, other);
    251          self.0 == other.0

Resolution: `if factor.bits() == 0 { return; }` (or drop the guard: a zero factor's per-digit `*=` is word-scale and the adds are no-ops); iterate `Limbs::new(&digits.0).flat_map(..)` directly; either drop `shift` (the adequacy kernels pre-shift through a test-local wrapper) or reword the doc to say the scale serves the committed known-bad kernels while every production charge is at scale zero; add one comment line stating that `subtract` is an operation selector, not a quantity sign. Acceptance: `grep -n '\*factor == Base::ZERO' web.rs` is empty; the min_ticks limb columns drop by one factor-width record per settle (re-pin attributed to this change) with value pins unchanged; either no production call site names a shift or the doc's description matches its callers.

#### skyline-query-23: `Reign::mint` and "mint" prose for constructing a value
- Where: crates/before/src/version/skyline/query/web.rs:190 (related: query.rs:56; query/web.rs:64, 66, 176, 300, 306, 311, 330)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep of `\bmint(s|ed|ing)?\b` over the four files: nine sites; 59 word-matches across crates/before/src and crates/suanpan/src); executed: no
- Seen by: structure, prose, correctness; refutation: confirmed, severity medium -> low; history: deliberate-but-expired (the identifier landed in c29bd2b3 when "mint" was the restructure campaign's own word; the writing-style ban postdates it by about thirteen days)
- Owner-gated: no

The constructor is named `mint`, so the word propagates into four doc sentences that name the operation, and query.rs:56 says the integral submodule "mints" the height-split components. The review's vocabulary rule: never write "mint" for constructing a value. `Reign::mint` is the only identifier crate-wide; the prose part is one slice of a crate-level sweep.

Evidence:

    188  impl Reign {
    189      /// A fresh record at a leaf's value, no closes counted yet.
    190      fn mint(sign: Sign, offset: &Base, epoch: u32) -> Reign {

    (query.rs:56)  //! whose components the [`integral`] submodule mints and derives along with the
    (web.rs:64)    //! - a reign record's mint and its one death settle: the code funding

Resolution: Rename to `Reign::new` (or `Reign::at_leaf`); reword query.rs:56 "defines and derives", web.rs:64 "a reign record's creation", 66 "between creation and death", 176 "since the record was created", 306 "created or moved". Acceptance: the grep over the partition returns nothing.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| skyline-query-2 | `crates/before/src/version/skyline/query.rs:203-205` | `let scale = max_depth` aliases carry a comment justifying a conversion that no longer exists | Pass `max_depth` directly; delete both aliases |
| skyline-query-7 | `crates/before/src/version/skyline/query.rs:547` | Long qualified paths where the sibling items are imported; a one-line `version_of` wrapper | Import `gamma_code_signed_int` and `Party`; hoist the test imports; drop `version_of` |
| skyline-query-8 | `crates/before/src/version/skyline/query.rs:649-650` | `pub(crate) mod integral` is wider than any use | `mod integral;` |
| skyline-query-14 | `crates/before/src/version/skyline/query/integral.rs:501-525` | Encoding conventions in the settle kernel that a named form would make evidently right | Name the two images; an `Option` min in `combine` |
| skyline-query-18 | `crates/before/src/version/skyline/query/integral.rs:921-934` | The "read an accumulator as (Sign, Base), skipping zero" idiom is hand-spelled at seven production sites | `signed_base(acc) -> Option<(Sign, Base)>` in signed.rs |

### Watermark and the traffic counters

13 entries (3 medium, 4 low, 6 nit); the full record is `evidence/partitions/skyline-watermark.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: skyline-watermark-28. Related findings in other documents: skyline-watermark-8 (documentation: the compaction bases), skyline-watermark-21 and skyline-watermark-24 (verification-gap: the test module's reachability claims), skyline-watermark-1 and skyline-watermark-25 (documentation).

#### skyline-watermark-14: propagate's certificate ladder is cost-inert over the fold it guards, and its duplicated guard costs a line-pinned mutant exclusion
- Where: crates/before/src/version/skyline/watermark.rs:772-809 (related: watermark.rs:676-694, 747-771, 810-839; .cargo/mutants.toml:99-117; tools/covcheck-expected.json:146-157; tests/meter.rs:3301-3327; crates/suanpan/src/accumulator.rs:536-575, 718-725, 805-828, 1169-1175)
- Class / severity / confidence: simplification / medium / medium
- Provenance: assessed (read suanpan's `fold_accum`, which walks `other.digits[..=other.top]` only, so `x.sub_accum(&y)` costs O(|y|) whatever `x`'s width; `sign()` and `sign_dominates_at` share one `fold_and_collapse`; .cargo/mutants.toml:106-109 records the boundary-dominates guard's inversion as touch-identical; the cascade verified from `git show b44a8326` and the four re-pins in history); executed: no
- Seen by: structure ([2]), claims ([32]); refutation: confirmed, reframed (the seam bands stay as the width-conservation instrument; what dissolves is the certificates, the `+ 2` rule, the two `unreachable!` arms with their covcheck entries, and the line pin); history: already-known for the touch-identity (d0501cd7 kept the arm deliberately for "width conservation, arm coverage, and the closed-form value legs"); reopened here with new evidence, since no record considers the merge-direction alternative
- Owner-gated: yes: reopens d0501cd7's recorded decision to keep the boundary-dominates arm; removes two rostered panic arms and a rostered exclusion

Circular justification: a guard earns its place by naming a cost it saves. The boundary-dominates arm (790-809) does exactly the work of the comparable fold below it (one O(1) top read of `diff`, one O(|residue|) fold, retire); the residue-dominates arm (772-789) equals folding the narrower operand into the wider; and when a certificate is undecided at exactly two digits of clearance, the current code falls through to `diff.sub_accum(&residue)`, paying the wider side. `merge_into_wider`'s rule — receiver by `digit_count`, fold the narrower in, read the exact sign — reproduces all five outcomes at the same or lower touch cost with no clearance arithmetic, and dissolves the 20-line certificate comment, both `unreachable!` arms, their two covcheck roster entries, and the exclusion pinned at `790:43`, which four commits in eight days have re-measured after prose edits. If the owner keeps the ladder, the two mirror-image guards should at least be one helper, which by the roster's own disposition ladder (step 1: refactor so the mutated codepoint does not structurally exist) may retire the line pin.

Evidence:

       772	                    if residue.digit_count() >= diff.digit_count() + 2 {
       773	                        match residue.sign_dominates_at(diff.digit_count() - 1) {
       774	                            (Ordering::Greater, true) => {
       790	                    if diff.digit_count() >= residue.digit_count() + 2 {
       791	                        match diff.sign_dominates_at(residue.digit_count() - 1) {
       792	                            (Ordering::Greater, true) => {
       813	                    diff.sub_accum(&residue);
       814	                    self.retire(residue);
       815	                    match diff.sign() {

Resolution (primary): replace 772-839 with `let residue_wider = residue.digit_count() >= diff.digit_count(); let (mut wide, narrow) = if residue_wider { (residue, diff) } else { (diff, residue) }; wide.sub_accum(&narrow); self.retire(narrow);` then dispatch on `(wide.sign(), residue_wider)`: `(Greater, true)` the difference died, `on_die(payload); zeros += 1; residue = wide; continue`; `(Equal, _)` exact meet, `on_die(payload); self.retire(wide); zeros += 1; break`; `(Less, true)` the boundary survives as `diff − residue`: `wide.negate(); push Diff { compact(wide), payload }; break`; `(Greater, false)` push `Diff { compact(wide), payload }; break`; `(Less, false)` the difference died: `on_die(payload); wide.negate(); residue = wide; zeros += 1`. Rewrite 676-694 as "fold the narrower operand into the wider (`merge_into_wider`'s rule); the survivor is never read across its width", delete .cargo/mutants.toml:99-117 and the two covcheck entries, re-pin tools/mutantcheck-expected.json. Fallback: a `fn dwarfs(big: &mut Accumulator, small: &Accumulator) -> bool` written once (guard, read, one `unreachable!`), the arm becoming two `if dwarfs(...)` calls; then `cargo mutants --list` decides whether the single `>=` site still needs a pin, and a name pin replaces the line pin if so. Acceptance: `just test-all` green with the seam plunge/stop bands, the latent-ladder band, the ascend envelope, and the fill/min_ticks differentials unchanged; the seam MEASURED lines inside their bands (identical, or differing by a constant per hop); no line-and-column pin for `watermark.rs` in .cargo/mutants.toml; `just mutants-list` clean.
Construction: before changing code, delete lines 790-809 alone under a local reverted swap and run `skyline_min_ticks_seam_stop_*`: the mutants.toml rationale predicts byte-equal readings, which demonstrates that arm saves nothing. Then apply the primary replacement and capture the seam bands' MEASURED lines; equality or a per-hop constant delta inside the ×0.75/×1.25 bands settles cost-inertness.

Synthesis note: The refutation pass reframed this to keep the seam bands as the width-conservation instrument; what dissolves is the certificate ladder, the `+ 2` rule, the two `unreachable!` arms with their covcheck entries, and the line pin. The construction (delete 790-809 under a local swap and re-run the seam-stop bands) costs nothing and should precede any code change. skyline-watermark-18 and -19 (this document) are the sibling consolidations in the same kernel.

#### skyline-watermark-18: Four undercut tails, one with drop_below's follower loop hand-inlined where the polarity bug lived
- Where: crates/before/src/version/skyline/watermark.rs:969-982 (related: watermark.rs:514-519, 529-547, 919-924, 1003-1010; tests/meter.rs:9399-9411)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`git show d7293057 -- watermark.rs` shows the one-line fix `offset.sign` → `offset.sign.negate()` at the hand-written residue fold that is line 974 today; the inlined loop at 975-977 equals `drop_below` on a latent-free web because the guard at 962 makes `latent` `None` and the tag invariant then clears every tag; the lease-order difference between `undercut` (515, lease before `drop_below`) and the two `MinWeb<()>` tails (924, 1008, lease after) verified by reading, with the pool row's derivation at meter.rs:9402-9410 written for lease-first and its three assertions unable to trip under lease-after); executed: no
- Seen by: structure ([1]), prose ([21]), correctness ([27]), claims ([38]); refutation: confirmed (four duplicates merged); history: deliberate-but-expired (c29bd2b3's op-sequence preservation; fe39fca3 broke byte-identity deliberately the same day)
- Owner-gated: no

The sequence "take `gap`, negate into the residue, drive the drop outward, re-seat `gap`" is written in `undercut`, in `emit_here`, and twice in `emit_offset`; the dominated arm re-implements `drop_below`'s follower loop and derives the residue by a separate hand fold, and that hand fold is the one place the polarity was wrong (d7293057). Folding `+offset` into `gap` on the dominated arm and calling one shared tail makes the residue arithmetic a single code path and the lease order a single documented decision, which is the doctrine's "the cheapest passing artifact must be the intended one" applied to a kernel with a recorded polarity bug.

Evidence:

       969	                web_traffic::record(web_traffic::Decision::DominatedUndercut);
       970	                // gap wide-negative: v sits far below the minimum; the
       971	                // drop dwarfs the offset. Residue = m − v = −gap − offset.
       972	                let mut residue = core::mem::take(&mut self.gap);
       973	                residue.negate();
       974	                fold_signed_int(&mut residue, offset.sign.negate(), &offset.magnitude);
       975	                for follower in self.followers.iter_mut().flatten() {
       976	                    follower.sub_accum(&residue);
       977	                }
       978	                let mut gap = self.lease();
       979	                fold_signed_int(&mut gap, offset.sign.negate(), &offset.magnitude);
       980	                self.gap = gap;
       981	                self.propagate(residue, |()| ());
       982	                return;

Resolution: give `undercut` the lease-after order (`let mut residue = core::mem::take(&mut self.gap); residue.negate(); self.drop_below(residue, on_die); self.gap = self.lease();`) and state the order in its doc; replace `emit_here`'s tail (921-924) with `self.undercut(|()| ())`; in `emit_offset` make the dominated arm `fold_signed_int(&mut self.gap, offset.sign, &offset.magnitude);` and give both undercut arms the one shared tail `self.undercut(|()| ()); fold_signed_int(&mut self.gap, offset.sign.negate(), &offset.magnitude);`. The comment at 970-971 becomes "`gap` holds `v − A` with no latent, so `drop_below`'s residue is `m − v`". Acceptance: one undercut tail in the file; `a_dominated_undercut_subtracts_its_residue_from_live_followers`, `dominated_latent_annihilates_into_the_undercut_residue`, and the `fill/tests.rs` differentials green; under `--features limb-meter` the `dominated_undercut_cost` floor and ceiling hold and `seam_stop_pool_misses_stay_at_warmup_across_churn_doubling` reads equal misses (a lease moved after a retire can only lower them, never below the first arming's one).

Synthesis note: skyline-watermark-19 (this document) is the latent-ladder copy the same kernel carries; the two resolutions compose (one undercut tail, one `cmp_min`).

#### skyline-watermark-19: The latent-ladder decision is written three times
- Where: crates/before/src/version/skyline/watermark.rs:986-1002 (related: watermark.rs:452-463, 1045-1063; tests/meter.rs:3417-3427)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified by reading each copy arm by arm against the proposed helper (no latent: the plain sign; latent live: `Equal | Greater` → `Greater`, a refusal → `Greater`, a surviving dominated latent → `Less`, a collapse → a fresh sign read), including side effects (the collapse inside `decide_undercut_through_latent`, the restore on every exit in `emit_offset`); on the dominated path the helper returns `Less` without the second sign read at 462/998, so touch readings can only fall, and the latent-ladder floor's premise does not count that read; executed: no
- Seen by: structure; refutation: confirmed (acceptance corrected: no `decide_undercut_through_latent` call site outside the helper); history: deliberate-but-expired
- Owner-gated: no

Legibility and correctness surface: "does the `v − A` in `gap` lie strictly below `m`" is a four-arm ladder (sign read; if a latent lives, the domination decision; plain re-test after a possible collapse) hand-copied into `undercuts_here`, the fold path of `emit_offset` (each false exit restoring the priced fold separately), and the latent branch of `compare_above`. Three copies in a kernel whose history includes a polarity bug are three places a future ladder edit must be made in lockstep, and the restore-on-every-exit shape is where a missed restore would silently displace the web.

Evidence:

       987	        fold_signed_int(&mut self.gap, offset.sign, &offset.magnitude);
       988	        if self.gap.sign() != Ordering::Less {
       989	            // v at or above the anchor, hence at or above the minimum.
       990	            fold_signed_int(&mut self.gap, offset.sign.negate(), &offset.magnitude);
       991	            return;
       992	        }
       993	        // v < A: only a drop past the latent too is a true undercut.
       994	        if self.latent.is_some() && !self.decide_undercut_through_latent() {
       995	            fold_signed_int(&mut self.gap, offset.sign.negate(), &offset.magnitude);
       996	            return;
       997	        }
       998	        if self.gap.sign() != Ordering::Less {
       999	            // A collapse re-based the anchor to m and v is not below it.
      1000	            fold_signed_int(&mut self.gap, offset.sign.negate(), &offset.magnitude);
      1001	            return;
      1002	        }

Resolution: add a private `fn cmp_min(&mut self) -> Ordering`, documented as "the ordering of `v` against `m` for the `v − A` that `gap` currently holds; may retire the latent (a funded collapse)": read `gap.sign()`; with no latent return it; with a latent map `Equal | Greater => Greater`, and on `Less` return `Greater` if the ladder refuses, `Less` if the latent survived (dominated), else `gap.sign()`. Then `undercuts_here` is `self.cmp_min() == Ordering::Less`, `emit_offset`'s fold path is one fold, one `if self.cmp_min() != Ordering::Less { restore; return }`, one undercut tail (finding 18), and `compare_above`'s latent branch is `let sign = self.cmp_min();`. Update `undercuts_here`'s doc (the shared form is anchor-relative, not `v = h`). Acceptance: `watermark/tests.rs`, the latent-ladder differentials in `fill/tests.rs`, and `skyline_min_ticks_latent_ladder_is_flat_per_unit` pass unchanged; `decide_undercut_through_latent` has no call site outside `cmp_min`; `watermark.rs` is net smaller.

Synthesis note: skyline-watermark-18 (this document) supplies the shared undercut tail this resolution's `emit_offset` fold path calls.

#### skyline-watermark-7: Follower slots as parallel arrays; the coupling invariant lives in asserts and an expect
- Where: crates/before/src/version/skyline/watermark.rs:194-210 (related: watermark.rs:331, 364-402, 414-429, 529-547, 639-644, 1120-1138)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (read the five index loops at 367-375, 387-391, 418-426, 530-540, 639-644, the `expect` at 422, and the vacuous tag reset at 331 after every follower is asserted `None`); executed: no
- Seen by: structure; refutation: confirmed; history: no-rationale-found (the tick-cost-spec's "one-bit sigma tag" constrains the tag's cost, not its layout)
- Owner-gated: no

Types-first: "a set tag rides an active follower" is enforced by index loops, a `debug_assert!`, and an `.expect(...)` where a single `[Option<Follower>; FOLLOWER_SLOTS]` with `struct Follower { relation: Accumulator, anchor_relative: bool }` makes a tag without a follower unrepresentable, turns every loop into `for f in self.followers.iter_mut().flatten()`, and deletes the `expect`, the tag-existence asserts, and line 331. What remains to assert is only the tag-to-latent relation.

Evidence:

       194	    latent: Option<Accumulator>,
       195	    /// Per follower slot: whether the stored content is anchor-relative
       196	    /// (`f_true = f_stored − Λ`). Set only while the latent lives; a set tag
       197	    /// never outlives it.
       198	    anchor_relative: [bool; FOLLOWER_SLOTS],
       207	    /// Active followers (module doc), tracking `m − X` (anchor-relative while
       208	    /// the slot's tag is set). The fill walk installs them; the min-ticks
       209	    /// fold leaves both slots empty.
       210	    followers: [Option<Accumulator>; FOLLOWER_SLOTS],
       420	                self.followers[slot]
       421	                    .as_mut()
       422	                    .expect("a set tag rides an active follower")

Resolution: define the private `Follower` struct, change the field, delete `anchor_relative`, rewrite the five loops over the flattened iterator; `follower_set` becomes `Some(Follower { relation, anchor_relative: self.latent.is_some() })`, `follower_take` returns `.take().expect("the follower is active").relation`. Keep `park`'s and `drop_below`'s asserts relating tags to the latent. Acceptance: no `anchor_relative` array, no `expect("a set tag rides an active follower")`, no `0..self.followers.len()` loop; `watermark/tests.rs`, `fill/tests.rs`, and the fill touch envelopes unchanged (no arithmetic moves).

#### skyline-watermark-12: arm_at_height and arm_below duplicate the first-arming preamble
- Where: crates/before/src/version/skyline/watermark.rs:562-574 (related: watermark.rs:598-609, 1019-1022, 593-596)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read: the two branches differ only in what seats the new `gap`, `self.lease()` versus `below`; `emit_below_accum`'s `debug_assert!(self.pending > 0, ...)` at 1020 restates `arm_below`'s at 593-596 with a different message); executed: no
- Seen by: structure; refutation: confirmed; history: deliberate-but-expired (c29bd2b3 preserved each client's op sequence for byte-identity against twin webs deleted in the same commit)
- Owner-gated: no

Legibility: the first-arm invariants (`armed = pending`, `pending − 1` zeros, followers and latent absent) are stated twice and must agree; the module doc's "All three converge on the shared boundary bookkeeping" is true of the non-first path only.

Evidence:

       562	        if self.armed == 0 {
       563	            debug_assert!(
       564	                self.followers.iter().all(Option::is_none),
       565	                "followers attach after the first arming"
       566	            );
       567	            debug_assert!(self.latent.is_none(), "the latent dies with the web");
       568	            self.armed = pending;
       569	            let fresh = self.lease();
       570	            let old = core::mem::replace(&mut self.gap, fresh);
       571	            self.retire(old);
       572	            self.push_zeros(pending - 1);
       573	            return;
       574	        }

Resolution: extract `fn seat_first(&mut self, pending: u64, gap: Accumulator)` holding the two asserts and the seat/retire/push_zeros body, taking the already-leased or moved `gap` so the lease-before-retire order the pool row's warm-up derivation describes is preserved; call it from both entry points. Drop the duplicate assert in `emit_below_accum` or keep only the one whose message names the raise. Acceptance: one occurrence of "followers attach after the first arming"; `batch_armed_closes_consume_exactly_one_range_record` and the min_ticks differentials green.

#### skyline-watermark-27: The two watermark counters sit under different gates, and web_traffic misnames the gate of the idiom it cites
- Where: crates/before/src/version/skyline/web_traffic.rs:19-24 (related: web_traffic.rs:46, 57, 100, 109; pool_traffic.rs:20-25, 27, 49, 58; Cargo.toml:49, 74-95; src/meter.rs:3668-3699; fill/tests.rs:459, 468, 597, 600; lib.rs:438-439; codec/scan.rs:24; hull_traffic.rs:16-17)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (web_traffic's `counter`, `use`, and `EmitTraffic` are `#[cfg(feature = "meter")]`; pool_traffic's are `limb-meter`; Cargo.toml:49's self-dev-dependency enables `meter` for every bench and test build; Cargo.toml:80-83 and 92-94 state that `limb-meter` and `scan-meter` exist to keep per-primitive relaxed-atomic bumps "out of the bench builds"; codec/scan.rs:24 is `scan-meter`, so both web_traffic.rs:19-20 and hull_traffic.rs:16-17 cite an idiom under the wrong gate name; `fill/tests.rs` reads `crate::meter::emit_traffic()` at 459-468 and 597-600 with no `#[cfg]` anywhere in that file); executed: no
- Seen by: structure ([5]), claims ([41]); refutation: [5] reframed (re-gating is not consumer-free: the two fill/tests.rs readers must move under the gate; hull_traffic is a per-call counter of the same form under `meter`, so the frequency argument has an opposite precedent), [41] refuted (the gate names a frequency class, not a capability dependency); history: no-rationale-found (both the gate and the inaccurate citation were inherited by copy from hull_traffic in 76579a3c)
- Owner-gated: yes: which frequency class the `meter` feature admits is a gate-policy decision; both directions have in-tree precedent

Two counters on one struct under two gates with no stated reason, and a doc that names the wrong idiom: `web_traffic::record` fires on every latent-free word-scale priced emission in the tick kernel the bench judge times (`emit_offset` 966, 969, 984) under `meter`, which the self-dev-dependency puts in every bench build, while Cargo.toml's stated reason for `limb-meter` and `scan-meter` is to keep per-primitive bumps out of that profile; `pool_traffic::record_miss` fires only on a miss (bounded by peak demand) yet sits under `limb-meter`. Either the cost argument moves web_traffic under `limb-meter` (with the two `fill/tests.rs` readers cfg'd) or Cargo.toml states that `meter` admits decision-liveness counters (hull_traffic's per-call counter is the precedent); in both cases lines 19-20 should name the gate they actually share.

Evidence:

        19	//! The recording compiles to nothing without the `meter` feature — the
        20	//! [`codec::scan`](crate::codec) counter's idiom — and the readings are

Resolution: owner's choice of direction. (a) Gate `web_traffic`'s `counter`, `use`, and `EmitTraffic`, plus `meter::emit_traffic`/`reset_emit_traffic` and the re-export at meter.rs:81, on `limb-meter` exactly as `pool_misses` is, and put `#[cfg(feature = "limb-meter")]` on the two `fill/tests.rs` witnesses' counter asserts (or on the tests). (b) Add to Cargo.toml's `meter` comment that per-decision liveness counters (`hull_traffic`, `web_traffic`) are admitted, and state why a per-emission bump in the tick kernel is acceptable in bench builds. In either case reword web_traffic.rs:19-20 (and hull_traffic.rs:16-17, outside this partition) to name the actual gate of the cited idiom. Acceptance: one stated policy the two watermark counters both satisfy; `grep -rn "codec::scan.*idiom"` finds no citation under a gate `codec::scan` does not use; under (a), `cargo build --features meter` compiles `emit_offset` with no `web_traffic` statics and `cargo test -p before --all-features` runs `dominated_undercut_cost` and both fill witnesses green.

Synthesis note: module-graph-3 (this document) covers `hull_traffic` and `web_traffic` together and recommends `scan-meter`.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| skyline-watermark-6 | `crates/before/src/version/skyline/watermark.rs:154-168` | Strictly-positive counts held as plain u64 | `NonZeroU64` for `Word` and `ZeroRun`, if taken |
| skyline-watermark-11 | `crates/before/src/version/skyline/watermark.rs:492-500` | A redundant latent sign read, and a comment that credits it with the floor's tightness | `debug_assert_eq!` on the latent sign; re-state the comment |
| skyline-watermark-16 | `crates/before/src/version/skyline/watermark.rs:883-891` | Accumulator::default() spelled once where the crate says Accumulator::new() | `Accumulator::new()` |
| skyline-watermark-20 | `crates/before/src/version/skyline/watermark.rs:1140-1150` | materialize and the lease/retire pool are an allocator riding the watermark type | A private `Pool` type, if taken |
| skyline-watermark-23 | `crates/before/src/version/skyline/watermark/tests.rs:91-105` | The three-probe minimum read is written seven times | `assert_minimum_at` helpers |
| skyline-watermark-26 | `crates/before/src/version/skyline/pool_traffic.rs:27-60` | Four hand-rolled counter modules of one shape | A `macro_rules!` counter module, if taken |

## The codec

### Bits

7 entries (1 medium, 2 low, 4 nit); the full record is `evidence/partitions/codec-bits.md`. Related findings in other documents: codec-bits-15 (verification-gap: no model test for `PackedBuilder` and `load_be`), codec-bits-30 (test-quality: the `BitStack` differential), codec-bits-23 (correctness: the 32-bit wide-gamma width guard), codec-bits-29 (claim: `peek_flip`'s re-scan).

#### codec-bits-12: PackedBuilder reimplements BitsBuf's append-truncate substrate behind a staging register whose saving is unmeasured
- Where: crates/before/src/codec/build.rs:48-58 (related: crates/before/src/codec/build.rs:102-106, 144-167, 175-200, 208-228, 245-266, 270-282, 284-325; crates/before/src/codec/buf.rs:93-102, 142-150, 185-214, 230-239, 259-273, 346-369; crates/before/src/codec/code.rs:42-59)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (compared each primitive pair line by line; `grep -rln PackedBuilder`: the wrappers are party/ops/build.rs and version/skyline/build.rs; `from_raw_parts` has one caller, build.rs:239; `git show 83e61b4d`); executed: no
- Seen by: structure [0]; refutation: confirmed with one correction (the register is a live design choice: `BitsBuf::push_bits` pops and re-pushes the partial tail byte on every append, buf.rs:198-211, which the register avoids on sub-byte appends; 83e61b4d kept it deliberately without a measurement, so the sign is not fixed); history: deliberate-but-expired (525e7324 introduced the register against the `bitvec` buffer; 83e61b4d gave `BitsBuf` the same byte-backed representation and recorded keeping the register with no reason: "PackedBuilder keeps its staged-register internals and word-parallel moves")
- Owner-gated: no (crate-private; acceptance runs the bench judge)

`PackedBuilder` owns a `Vec<u8>` plus a sub-byte `staged`/`staged_len` register and reimplements every primitive `BitsBuf` provides: `append_bits` (245-266) is the same u128-merge/`to_be_bytes`/extend body as `push_bits` (buf.rs:185-214); `append_bytes` (270-282) is `extend_bytes` (buf.rs:259-273); `splice` (175-200) is `extend_from_view` (buf.rs:346-369) plus a meter tap; `truncate` (208-228) is `BitsBuf::truncate` (buf.rs:230-239) with a committed/staged split; `patch_bit` (144-167) is `BitsBuf::set` (buf.rs:142-150) with the same split; `read_bits`/`bit_at` (284-325) exist only because the staged bits are outside `bytes`, so the builder cannot hand out a `BitsView` of itself, which is why `extract_code`'s wide arm copies one bit at a time where `Code::from_range` is byte-parallel. Two invariant sets and two-branch readers for one discipline (Principle 3: machinery outlives the constraint that justified it; legibility).

Evidence:

        48	pub(crate) struct PackedBuilder {
        49	    /// The committed prefix: whole bytes, most-significant bit first.
        50	    bytes: Vec<u8>,
        51	    /// The trailing not-yet-committed bits, value-packed at the low end
        52	    /// (the stream's next bit is the register's most significant live
        53	    /// bit). Always fewer than eight: appends flush whole bytes
        54	    /// greedily.
        55	    staged: u64,
        56	    /// Live bits in `staged`, `0..8`.
        57	    staged_len: u32,
        58	}
    (build.rs:259-262 beside buf.rs:204-205)
       259	        let acc = (u128::from(self.staged) << len) | u128::from(value);
       260	        let rem = total % 8;
       261	        let whole = (total / 8) as usize;
       262	        let aligned = (acc << (128 - total)).to_be_bytes();
       204	        let acc = (u128::from(staged) << len) | u128::from(value);
       205	        let aligned = (acc << (128 - total)).to_be_bytes();
    (build.rs)
       102	        let mut out = BitsBuf::with_capacity(n);
       103	        for i in start..start + n {
       104	            out.push(self.bit_at(i));
       105	        }
       106	        Code::Wide(out)

Resolution: Construct `PackedBuilder { out: BitsBuf }` as the metered move set over the one build buffer: `push_bit` = record + `out.push`; `push_code` Small = record + `out.push_bits(bits, len)`, Wide = `splice`; `reserve(w)` = record + `out.push_bits(0, w)`; `patch_bit` = record + `out.set`; `splice` = record + `extend_from_view`; `truncate` = `out.truncate`; `extract_code(start)` = record + `Code::from_range(built_view(&self.out), start, self.len())`; `finish` = `self.out`. Delete `append_bits`, `append_bytes`, `read_bits`, `bit_at`, and `BitsBuf::from_raw_parts`. Measure at the parent and at the change on a quiet machine (`just bench-judge`); if the register measurably wins, invert the direction and give `BitsBuf` the register form so one implementation serves both wrappers. Acceptance: one implementation of the byte-backed append/truncate/patch discipline exists in the crate; `just test-all` green; `tests/meter.rs` envelopes and scan floors unchanged; the bench judge within band with both numbers recorded in the commit; buf.rs:41-43 becomes true.

#### codec-bits-22: gamma::load_window duplicates BitsView::load_be behind a raw-parts indirection with one caller
- Where: crates/before/src/codec/gamma.rs:153-207 (related: crates/before/src/codec/bits.rs:321-343; crates/before/src/codec/code.rs:51; crates/before/src/codec/dsi.rs:86; crates/before/src/borsh_impls.rs:119-121)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn load_be`: definition and code.rs:51 only; `window_int` and `load_window` are called only inside gamma.rs; `.body_tail()` at dsi.rs:86 and gamma.rs:154; the equivalence of `load_be(pos, proven) << (64 - proven)` to `load_window`'s zero-filled window derived from both bodies, including `load_be`'s discard of every bit past `len`, so a `Bits::live()` view's padding never leaks); executed: no
- Seen by: structure [2]; refutation: confirmed (re-derived the equivalence); history: deliberate-but-expired (78c65371 moved the window onto raw `(body, tail)` parts so the doors and the borsh reader could window without the 32-bit-capped borrowed view; 5d167a63 introduced `BitsView` with `load_be` and removed that constraint, leaving `window_int`/`load_window` in place)
- Owner-gated: no

`decode_int_window` destructures its view into `(body, tail)` solely to call `window_int`, whose only caller it is; `window_int` calls `load_window`, whose only caller it is; `load_window` gathers up to nine bytes and shifts a big-endian window exactly as `load_be` does, with clamps that exist only because it works on raw parts. Two places to get a shift-by-8 wrong, and `window_int`'s phantom-zeros prose restates `body_tail`'s contract a second time.

Evidence:

       153	pub(crate) fn decode_int_window(bits: BitsView<'_>, pos: u64) -> Option<(u64, u64)> {
       154	    let (body, tail) = bits.body_tail();
       155	    window_int(body, tail, bits.len(), pos)
       156	}
    ...
       189	    let mut buf = [0u8; 9];
       190	    let start = byte.min(body.len());
       191	    let end = byte.saturating_add(buf.len()).min(body.len());
       192	    buf[..end - start].copy_from_slice(&body[start..end]);
    (bits.rs)
       333	        let mut buf = [0u8; 9];
       334	        let end = (byte + buf.len()).min(self.bytes.len());
       335	        buf[..end - byte].copy_from_slice(&self.bytes[byte..end]);

Resolution: Fold `window_int` and `load_window` into `decode_int_window`: `let proven = bits.len().checked_sub(pos)?.min(WINDOW_BITS); if proven == 0 { return None; } let window = bits.load_be(pos, proven as u32) << (WINDOW_BITS - proven); let k = u64::from(window.leading_zeros()); let code_len = 2 * k + 1; if code_len > proven { return None; } let m = window >> (WINDOW_BITS - code_len); Some((m - 1, pos + code_len))`. `load_be`'s debug assert holds because `pos + proven <= len`; `body_tail` keeps its one remaining caller. Acceptance: gamma.rs has one window function; `gamma_window_edge`, `gamma_window_declines_conservatively`, `gamma_word_decode_matches_bit_loop`, `gamma_word_paths_match_on_arbitrary_bytes`, and the borsh differentials pass unchanged.

#### codec-bits-28: Dead `len == 64` arm in BitStack::push_bits, the sibling of the disjunct 35a09c5b swept from PackedBuilder::append_bits
- Where: crates/before/src/codec/stack.rs:61-69 (related: crates/before/src/codec/stack.rs:87-88, 213-231, 256-262; crates/before/src/codec/build.rs:245-250)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git show 35a09c5b -- crates/before/src/codec/build.rs` shows the hunk `-            len == 64 || value >> len == 0,` / `+            value >> len == 0,`; `git show d07fc20e:crates/before/src/codec/stack.rs` lines 58-69 hold the identical arm with the `len <= 63` assert already present; callers at stack.rs:221, 223, 227, 229 pass 63 or `width < 64`); executed: no
- Seen by: structure [3], prose [22], correctness [28], claims [32]; refutation: confirmed; history: no-rationale-found (landed dead in d07fc20e; 35a09c5b's sweep named the disposition for the builder and did not reach this sibling)
- Owner-gated: no

The debug assert requires `len <= 63`, so its `len == 64 ||` disjunct is unsatisfiable and the `if len == 64 { value }` arm can never execute; `pop_bits` (87-88) already has the clean form, and `PopStack::push`/`pop` carry four explicit `width == 64` splits precisely because 64-bit batched moves are not supported. A branch the preceding assert excludes makes the reader check whether the contract or the code is wrong.

Evidence:

        61	    fn push_bits(&mut self, value: u64, len: u32) {
        62	        debug_assert!(len <= 63 && (len == 64 || value >> len == 0));
        63	        let total = self.top_len + len;
        64	        if total <= 64 {
        65	            self.top = if len == 64 {
        66	                value
        67	            } else {
        68	                (self.top << len) | value
        69	            };

Resolution: Mirror 35a09c5b: `debug_assert!(len <= 63 && value >> len == 0);` and `self.top = (self.top << len) | value;`. (The larger alternative, making `push_bits`/`pop_bits` total on `1..=64` and deleting `PopStack`'s four `width == 64` splits, is an open question below.) Acceptance: no `len == 64` text remains in `push_bits`; `bit_stack_matches_a_vec_of_bools` and `pop_stack_matches_a_vec_model_across_all_widths` pass.

Synthesis note: The same dead arm is inventory-9 and recursion-8 in this document; recursion-8 adds the undocumented one-level self-call in `pop_bits`. One edit closes all three.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| codec-bits-14 | `crates/before/src/codec/build.rs:127-137` | reserve loops over 32-bit chunks for a width that is always 2 | `reserve(width: u32)` with one `append_bits` |
| codec-bits-17 | `crates/before/src/codec/dsi.rs:227-227` | Idiom nits in the word-parallel cursor and the padding judge | Use the `From<Truncated>` impl; `read_word` via `read_word_opt`; the listed one-liners |
| codec-bits-21 | `crates/before/src/codec/gamma.rs:69-101` | code_int and code_int_small share a body | `code_int` dispatches to `code_int_small` on `to_u64()` |
| codec-bits-25 | `crates/before/src/codec/literal.rs:51-65` | id_node re-parses a node it has proved normal by construction | Delete the `validate_id` call and state the induction, or validate once in `finish_id` |

### Base, text, and tree

14 entries (5 low, 9 nit); the full record is `evidence/partitions/codec-base-text-tree.md`. Related findings in other documents: codec-base-text-tree-13 (claim: the literal door's `O(n)`), codec-base-text-tree-18 (api-surprise: `Parse` precedence).

#### codec-base-text-tree-7: `Sub`'s `debug_assert!` compares through the metered `Ord` and duplicates the backend's own underflow panic; `SubAssign` clones the whole magnitude
- Where: crates/before/src/codec/base.rs:416-430 (related: crates/before/src/codec/base.rs:284-295, crates/before/tests/meter.rs:60-65, crates/before/src/meter.rs:2123, crates/before/src/meter.rs:2511, crates/before/src/oracle/version.rs:398, crates/before/src/testing/shape_rows.rs:34)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read base.rs:284-295: `PartialOrd::partial_cmp` calls `Ord::cmp`, which calls `meter_limbs2`; read dashu-int 0.5.0 `src/add_ops.rs:185-235`, every negative-result arm calls `panic_negative_ubig`, defined at `src/error.rs:19-21`; read `src/helper_macros.rs:323-338`, `impl_binop_assign_by_taking!` emits `SubAssign<&UBig> for UBig` via `mem::take`, invoked at `add_ops.rs:9`; `grep -rn '\-= &' src` on `Base` values: meter.rs:2123, :2511, oracle/version.rs:398, testing/shape_rows.rs:34, plus test files); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: deliberate but expired (the assert named an underflow the pre-dashu `u64` arm would not name; 2bfc1398 made the backend panic on every negative result in every profile)
- Owner-gated: no

`debug_assert!(self >= *rhs, …)` resolves to `PartialOrd::ge` → `Ord::cmp` → `meter_limbs2`, so under `limb-meter` in a dev build every `Base - &Base` records twice its operands' limbs and once in release; `tests/meter.rs:63-64` carries that profile dependence as a caveat, and the refutation pass reports no other `debug_assert!` comparing two `Base` values in the crate, so this assert is the caveat's one source. dashu's `UBig` subtraction already panics on a negative result in every profile (`panic_negative_ubig`), so the assert names nothing the backend misses. `sub_assign` clones `self` to reuse `Sub` although dashu provides an in-place `SubAssign<&UBig>` by `mem::take`. Doctrine: asserts get the meters' adequacy scrutiny, and a guard beside a meter must not feed it; a deterministic counter should be a function of the operation sequence alone.

Evidence:

       416	impl Sub<&Base> for Base {
       417	    type Output = Base;
       418	
       419	    fn sub(self, rhs: &Base) -> Base {
       420	        meter_limbs2(&self, rhs);
       421	        debug_assert!(self >= *rhs, "Base subtraction underflow");
       422	        Base(self.0 - &rhs.0)
       423	    }
       424	}
       425	
       426	impl SubAssign<&Base> for Base {
       427	    fn sub_assign(&mut self, rhs: &Base) {
       428	        *self = self.clone() - rhs;
       429	    }
       430	}

    (tests/meter.rs)
        63	//! (limb counts shrink under release, where `debug_assert!` comparisons
        64	//! vanish, so the dev-profile pin is the binding one), while segment counts

Resolution: Delete the `debug_assert!` and add a `# Panics` line to the impl stating that an underflowing difference panics (the backend's own check, every profile; programmer error per the crate's panic policy). Rewrite `sub_assign` as `meter_limbs2(self, rhs); self.0 -= &rhs.0;`. Measure every limb-denominated envelope whose cell subtracts at the parent commit, then tighten the committed ceilings in the same change with the movement attributed to the assert's removal; strike the profile caveat at tests/meter.rs:63-64 if no other source remains. Acceptance: under `--features limb-meter`, `limb_ops()` after one `Base - &Base` on two k-limb operands reads 2k in dev and release alike; envelopes re-pinned with the parent measurement recorded; the meter.rs header either drops the caveat or names its remaining source.

Synthesis note: inventory-4 (this document) reaches the same assert from the sweep; the limb envelopes that move must be re-pinned at the parent with attribution.

#### codec-base-text-tree-8: `Shl<i32>` and `BitOr<Base>` on `Base` exist for one test idiom; `Shl<i32>` wraps a negative amount in release
- Where: crates/before/src/codec/base.rs:448-455 (related: crates/before/src/codec/base.rs:503-510, crates/before/src/codec/tests.rs:51-61, crates/before/src/codec/tests.rs:521-546, crates/before/src/meter.rs:73, crates/before/src/version/skyline/query.rs:422)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn 'Shl<i32>\|<< 64)\|| Base::from' src tests benches examples fuzz fuzzfit wasm32-pins surfacecheck`: the only `Base` shifts by an unsuffixed literal and the only `|` on two `Base`s are codec/tests.rs:54 and :528, every other hit is on `u128`; `grep -rn '\bi32\b'` finds no other `i32` value in `before` outside a `powi` doc example, the ignored test's prose, and fuzzfit's wasm ABI; `Base::from(0u32)` and `acc += 1u32` have instrument-side callers at meter/board/defect.rs:102 and meter/board/tests.rs:266, 620-627, so `From<u32>` and `AddAssign<u32>` are not vestigial); executed: no
- Seen by: structure, correctness, claims; refutation: reframed (the `From<u32>`/`AddAssign<u32>` half is refuted by the board callers; `Shl<i32>` is not dead, it is what lets the two unsuffixed literals type-check); history: no rationale found (the idiom was written against num-bigint's operator matrix; the impls arrived with the owned enum and were carried onto dashu unreconsidered)
- Owner-gated: yes: under the `meter` feature `Base` values are reachable through `before::meter::skyline::query::min_ticks` (meter.rs:73 `pub use crate::version::skyline;`, query.rs:422 `pub fn min_ticks(bits: BitsView<'_>) -> Base`), so an operator impl on `Base` is observable from outside the crate even though the type has no public path

`n << 64` and `value << 64` in the two gamma tests fall back to `i32` for the literal, so `Shl<i32>` exists to let them compile; its `debug_assert!(rhs >= 0)` is its whole protection, and in release a negative amount casts to a `u32` near 2^32 and becomes an unbounded widening shift in the backend. `BitOr<Base>` is used only on the same two lines, where `|` means "append a limb" on a type whose contract is event counts. Types-first: a sign the type can carry should not be a runtime assertion; circular justification: an impl whose only reason to exist is a test's missing suffix.

Evidence:

       448	impl Shl<i32> for Base {
       449	    type Output = Base;
       450	
       451	    fn shl(self, rhs: i32) -> Base {
       452	        debug_assert!(rhs >= 0, "Base left shift must be non-negative");
       453	        self << rhs as u32
       454	    }
       455	}

       503	impl BitOr<Base> for Base {
       504	    type Output = Base;
       505	
       506	    fn bitor(self, rhs: Base) -> Base {
       507	        meter_limbs2(&self, &rhs);
       508	        Base(self.0 | rhs.0)
       509	    }
       510	}

    (codec/tests.rs)
        54	            n = (n << 64) | Base::from(limb);
       528	            value = (value << 64) | Base::from(limb);

Resolution: Delete both impls. At codec/tests.rs:54 and :528 write `(n << 64u32) + Base::from(limb)` (the low limb is disjoint from the shifted value, so `+` and `|` agree), or build the wide value with `UBig::from_le_bytes` as `base/tests.rs::from_limbs` does. Acceptance: `cargo check --all-features --all-targets -p before` and the detached workspaces compile without the impls; `grep -n 'Shl<i32>\|BitOr' src/codec/base.rs` is empty; `gamma_roundtrip_wide` and `gamma_word_encode_matches_bit_encode` stay green.

Construction: `Base::from(1u8) << -1i32` in a release-profile test: no assertion fires; the backend attempts a shift by 4_294_967_295 bits (a ~512 MiB allocation or a capacity panic), neither of which is a documented contract.

Synthesis note: The same impl is clippy-pedantic-1 and inventory-5 in this document; the three agree on the fix (suffix the two literals, delete the impl) and differ only on owner-gating, which turns on the meter-surface ruling.

#### codec-base-text-tree-12: `write_id` keeps its phase stack on a `BitsBuf` while the crate owns `BitStack` for exactly that role, one of ten such stacks
- Where: crates/before/src/codec/display.rs:30-32 (related: crates/before/src/codec/stack.rs:1-2, crates/before/src/codec/buf.rs:175-181, crates/before/src/party/ops/compare.rs:115, crates/before/src/party/ops/sum.rs:136, crates/before/src/party/ops/diff.rs:253-257, crates/before/src/version/skyline/admit.rs:78-81, crates/before/src/version/skyline/grow.rs:516, crates/before/src/version/skyline/text.rs:239-240, crates/before/src/version/skyline/text.rs:355, crates/before/src/version/skyline/text.rs:512)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (`grep -rn '\.pop()' src` filtered to `BitsBuf`-typed receivers, cross-checked against each struct's field declarations: display.rs `pending` (57, 61); compare.rs `pending: BitsBuf` (180, 185); sum.rs `bits: BitsBuf` (178-190); diff.rs `path`, `pending_right: BitsBuf` (416, 429); admit.rs `path`, `left_was_leaf: BitsBuf` (151, 175, 185, 214); grow.rs `pending` (681); skyline/text.rs `phase` and `pending` (280, 379, 581). `BitStack::new()` users: party/ops/build.rs:197-198, skyline/fill.rs:1206-1208 and the sites 56a3dfe2 converted); executed: no
- Seen by: structure, correctness; refutation: confirmed (and extended the roster by compare.rs); history: no rationale found (56a3dfe2 claimed "every path, phase, and frame" but its file list has neither display.rs nor skyline/text.rs; `BitsBuf::pop` was added at 83e61b4d as a like-for-like replacement of bitvec's pop)
- Owner-gated: no

`codec::stack::BitStack` documents itself as "the word-backed bit stack the deep walks keep their paths and phases on", with O(1) register push and pop; `write_id` and nine sibling stacks across `compare.rs`, `sum.rs`, `diff.rs`, `admit.rs`, `grow.rs`, and `skyline/text.rs` instead pop a `BitsBuf`, the append-truncate build buffer whose `pop` is an asserted `get`, a `truncate`, and a tail mask that re-establishes a storage invariant no LIFO needs. Two disciplines for one role (Principle 3), and `BitsBuf::pop` exists only for these sites. The roster here is wider than any lens or the refutation pass recorded, which also means the conversion commit's completeness claim is further from true than the history pass found.

Evidence:

        30	    // Per open node: a phase bit on top ([`LEFT_PHASE`]/[`RIGHT_PHASE`]); under
        31	    // a left phase, the right child's presence bit.
        32	    let mut pending = BitsBuf::new();

    (stack.rs)
         1	//! Pop-able stacks held as bits: the word-backed bit stack the deep walks keep
         2	//! their paths and phases on, and the nonnegative-integer stack built over it.

    (buf.rs)
       175	    /// Pop the newest bit.
       176	    pub(crate) fn pop(&mut self) -> Option<bool> {
       177	        let pos = self.live.checked_sub(1)?;
       178	        let bit = self.get(pos);
       179	        self.truncate(pos);
       180	        Some(bit)
       181	    }

Resolution: Make `pending` a `BitStack` in `write_id` (same `push`/`pop` calls), and migrate the nine sibling stacks in the same change (`BitStack::len()` returns `u64`, matching the depth read at skyline/text.rs:572); then delete `BitsBuf::pop` once the compiler confirms no caller remains. While there, spell the closing arm at display.rs:68 as `Some(RIGHT_PHASE)` instead of `Some(_)`, and consider hoisting `LEFT_PHASE`/`RIGHT_PHASE` beside `BitStack` as the shared phase vocabulary, since skyline/text.rs:92 redefines `LEFT_PHASE`. Acceptance: `grep -n 'fn pop' src/codec/buf.rs` is empty; `deep_id_text_roundtrip`, `parse_stacks_handle_deep_spines`, and the display and text envelopes in `tests/meter.rs` stay green, with any heap envelope that moves re-measured at the parent before re-pinning.

Synthesis note: party-19 and skyline-coding-32 (this document) list the party and skyline sites of the same ten-stack roster; this entry's resolution migrates all ten in one change so `BitsBuf::pop` can be deleted, which the codec-base-text-tree summary's open question 7 recommends.

#### codec-base-text-tree-16: `parse_id_str` re-validates bits its own parser built canonically; the test reference runs the same pass, and `validate_id`'s "single source of truth" is untrue
- Where: crates/before/src/codec/text.rs:75-84 (related: crates/before/src/codec/text.rs:161-176, crates/before/src/codec/tree.rs:104-106, crates/before/src/codec/literal.rs:52-66, crates/before/src/codec/tests.rs:1729-1743, crates/before/src/codec/tests.rs:1747-1757)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn validate_id src`: definition tree.rs:114, calls at text.rs:82, literal.rs:64, tests.rs:1741; the cannot-fail argument is assessed by reading text.rs:129-180 and literal.rs:47-66); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: deliberate but expired (Phase 7's "the string and literal paths share the one normal-form validator" was written when the text parser emitted raw bits with no canonicality check; 32a655438 made `(0, 0)` unspellable, forced the inline arms in, and kept the pass and the sentence)
- Owner-gated: no

`parse_id_tree` emits exactly the presence-tag preorder of the tree it parses (every `(` reserves a tag patched at `)` from its children's kinds, every `1` emits `00`, a `0` emits nothing) and rejects both collapsible forms before patching, so its `Ok` output is always one complete canonical tree; `parse_id_core` rejects only a `(1, 1)` node or a malformed stream, neither of which the parser can emit (a `Node` child's tag is never `00`, so the wire's terminal test coincides with `IdKind::Terminal`). Line 82 therefore cannot return `Err`; it is a recompute-and-compare guard on a deterministic pure function, an extra O(n) pass on every `Party::from_str` and `Clock::from_str`, and because the reference parser at tests.rs:1741 makes the identical call, the exhaustive and proptest differentials compare `Ok(bits)` to `Ok(bits)` and are blind to the call's presence or absence. The inline `(0, 0)` arm cannot be replaced by the validator (the form has no bit spelling; dropping it would parse `((0, 0), 1)` as `(0, 1)`), so the validator is the redundant half. The same shape recurs at literal.rs:64, whose children come only from `id_leaf`/`id_node`. Meanwhile tree.rs:105 calls `parse_id` "the single source of truth for id normal form" while the collapsible rule is implemented at three sites. Doctrine: a guard must name a constructible failure the committed tests cannot catch; recompute-and-compare on a pure function is not defense in depth once differential coverage exists.

Evidence:

        75	pub(crate) fn parse_id_str(s: &str) -> Result<BitsBuf, Parse> {
        76	    let mut cur = Cur::new(s);
        77	    let mut bits = BitsBuf::new();
        78	    parse_id_tree(&mut cur, &mut bits)?;
        79	    if cur.peek().is_some() {
        80	        return Err(Parse::Syntax); // trailing junk
        81	    }
        82	    validate_id(super::built_view(&bits))?;
        83	    Ok(bits)
        84	}

       165	                    match (left, kind) {
       166	                        (IdKind::Empty, IdKind::Empty) => return Err(Parse::NotCanonical), // (0, 0)
       167	                        (IdKind::Terminal, IdKind::Terminal) => {
       168	                            return Err(Parse::NotCanonical); // (1, 1)
       169	                        }

    (tree.rs)
       104	/// Confirm a freshly built id bit stream is exactly one canonical-normal-form
       105	/// tree. Wraps [`parse_id`] (the single source of truth for id normal form),
       106	/// mapping its outcome onto [`Parse`].

    (tests.rs)
      1741	    super::validate_id(crate::codec::built_view(&bits))?;

Resolution: Delete text.rs:82 and literal.rs:64 (behavior-preserving: both return `Ok` on every reachable input). Move the guarantee they stood in for ("text-door acceptance is contained in wire-decode acceptance") into the harness: in `assert_id_parse_matches_reference` (tests.rs:1747-1757), on `Ok(bits)` also assert `validate_id(built_view(&bits)).is_ok()`, and drop the reference's own `validate_id` call so the reference is a pure grammar transcription; add the same assertion to a literal-door test. Reword tree.rs:105 to "the wire decoders' normal-form check". If the owner prefers the runtime pass as a totality defense against a future emission bug, state that rationale at each call site instead of the "single source of truth" sentence. Acceptance: every accepted `parse_id_str` and `id_node` output is asserted canonical by a test, not by a production re-parse; `id_text_parser_matches_reference_exhaustively`, the three id-parser proptests, `id_text_parser_error_precedence_pins`, `deep_id_text_roundtrip`, and the literal tests stay green; tree.rs:105 claims no single source of truth.

Construction: Temporarily replace `validate_id(...)?` at text.rs:82 with `debug_assert!(validate_id(...).is_ok())` and run the id text parser pins and `deep_id_text_roundtrip`; nothing fires. Conversely delete line 82 outright and run the codec suite: every test passes, because the reference validates its own output. `grep -n NotCanonical src/codec/{tree,text,literal}.rs` shows the collapsible rule at three sites.

Synthesis note: codec-bits-25 and inventory-6 (this document) cover the literal-door half of the same re-validation with two other dispositions; this entry's is the most complete.

#### codec-base-text-tree-19: `parse_clock_str` pre-scans for the top-level comma with an `i64` depth counter, work the id parser on a cursor already does
- Where: crates/before/src/codec/text.rs:185-215 (related: crates/before/src/codec/tests.rs:1926-1957, crates/before/src/codec/tests.rs:1959-1976, crates/before/src/clock.rs:942-952, crates/before/src/version/skyline/text.rs:494-502, crates/before/src/version/skyline/text.rs:617-618, crates/before/src/error.rs:106-121)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read the scanner, the ignored witness, `Clock::from_str`, and the skyline parser's cursor-based body and trailing check); executed: no
- Seen by: structure, claims; refutation: confirmed (with the owner gate added); history: no rationale found (the two-pass split is Phase 7's original; d089bca0 widened the counter and added the never-panics family and the 2 GiB witness without weighing an alternative to the pre-scan)
- Owner-gated: yes: the `Parse` variant moves for one input class (`"((1, 1) 0)"`: `Syntax` today, `NotCanonical` under a cursor-based split)

The stamp split walks `inner` once counting paren depth to find the first depth-0 comma, then hands the prefix to `parse_id_str`, which parses the same bytes again. Parsing the id directly from a `Cur` over `inner` and requiring the next token to be `,` yields the same accept set and value: a well-formed id has balanced parens and every comma at depth ≥ 1, so the first depth-0 comma is the one after the id; a malformed id rejects either way. The counter generated its own maintenance cascade: an `i32` to `i64` widening, the overflow proof at 192-193, a never-panics proptest, and a ~2 GiB `#[ignore]`d test that exists only to defend the counter's width. Principle 3: infrastructure that generates its own maintenance cascade is suspect; fixed-sign deletion, denominated in id-text bytes scanned.

Evidence:

       192	    // i64 cannot overflow: depth moves by at most one per input byte, and an
       193	    // allocation holds at most `isize::MAX` (< 2⁶³) bytes.
       194	    let mut depth: i64 = 0;
       195	    let mut split = None;
       196	    for (k, &c) in inner.as_bytes().iter().enumerate() {
       197	        match c {
       198	            b'(' => depth += 1,
       199	            b')' => {
       200	                depth -= 1;
       201	                if depth < 0 {
       202	                    return Err(Parse::Syntax);
       203	                }
       204	            }
       205	            b',' if depth == 0 => {
       206	                split = Some(k);
       207	                break;
       208	            }
       209	            _ => {}
       210	        }
       211	    }
       212	    let k = split.ok_or(Parse::Syntax)?;
       213	    let id_bits = parse_id_str(&inner[..k])?;
       214	    Ok((id_bits, &inner[k + 1..]))

Resolution: Minimal shape: keep the outer-paren strip; then `let mut cur = Cur::new(inner); let mut bits = BitsBuf::new(); parse_id_tree(&mut cur, &mut bits)?; if cur.bump() != Some(b',') { return Err(Parse::Syntax); } Ok((bits, cur.rest()))` with a `Cur::rest(&self) -> &'a str` accessor (the position after an ASCII `,` is a char boundary). Deeper shape: add a cursor-taking entry to `skyline::text` (its body already runs on `Cur` and its trailing check is one `peek`) and make the stamp one cursor pass returning both bit streams. Either way delete the depth loop, its comment, and `clock_text_split_survives_two_gib_of_parens`; keep `clock_text_deep_nesting_never_panics`, which exercises the whole `Clock::from_str`. Add a point pin for the precedence corner so the change is visible. Acceptance: `grep -n 'depth: i64' src/codec/text.rs` is empty; the 2 GiB ignored test is gone; the clock text pins and the never-panics proptest stay green; the clock `FromStr` fuel band in fuzzfit is unchanged or lower, measured at the parent.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| codec-base-text-tree-2 | `crates/before/src/codec/base.rs:64-69` | The limb denomination `bits.div_ceil(64).max(1)` is spelled independently at several sites, and two `cfg` blocks exist only because `record_wide` takes a raw `UBig` | One `limbs_of_bits` in `limb_meter`; drop the two `cfg` blocks |
| codec-base-text-tree-4 | `crates/before/src/codec/base.rs:97-106` | `Base::msb_cmp` is a one-caller wrapper whose body the sibling match arms already spell inline | Delete `Base::msb_cmp`; spell the four arms uniformly; reword the doc |
| codec-base-text-tree-10 | `crates/before/src/codec/base/limb_metered.rs:15-25` | `meter_limbs1` and `meter_limbs_solo` are both one-`Base` recorders whose difference lives only in their doc comments | Rename `meter_limbs1` to `meter_limbs_scalar` |
| codec-base-text-tree-11 | `crates/before/src/codec/display.rs:16-28` | `write_id`'s `sep` parameter has one caller and one value; the separator is already a named constant elsewhere | Drop `sep`; one `SEP` constant in `codec::text` |
| codec-base-text-tree-17 | `crates/before/src/codec/text.rs:98-120` | Prose texture in the parsers: two private `IdFrame` enums, the stack discipline restated five times, fragment heads, the "X, never Y" figure, and "exactly" as intensifier | Rename the text frame; state the stack discipline once per file; recast the fragments |
| codec-base-text-tree-21 | `crates/before/src/codec/tree.rs:35-60` | `parse_id`'s `pos` parameter is always 0, and three named layers wrap one grammar body | One always-compiled `parse_id_from`; shrink the re-exports |
| codec-base-text-tree-22 | `crates/before/src/codec/tests.rs:14-17` | Long qualified paths beside existing imports, and two `DefaultHasher` helpers, in the codec test suite | Extend the imports; one hasher helper |
| codec-base-text-tree-24 | `crates/before/src/codec/tests.rs:214-222` | Em-dashes in `//` code comments across the partition | Recast each em-dash; batch with the crate-wide sweep |
| codec-base-text-tree-28 | `crates/before/src/codec/tests.rs:1794-1797` | Dead `continue` at the end of the exhaustive odometer's outer loop | Delete the trailing `continue` |

## Cross-cutting: fold, shape, recurse, serde and borsh

### Cross-cutting

6 entries (1 low, 5 nit); the full record is `evidence/partitions/crate-root.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: crate-root-22. Related findings in other documents: crate-root-32 (verification-gap: the segments counter recurse.rs describes as measured).

#### crate-root-9: borsh_impls.rs: `Ranked`'s decoder re-inlines `Rank`'s, the one-byte read is spelled three times, and full paths sit beside their imports
- Where: crates/before/src/borsh_impls.rs:242-247 (related: crates/before/src/borsh_impls.rs:94-99, crates/before/src/borsh_impls.rs:211-220, crates/before/src/borsh_impls.rs:141, crates/before/src/borsh_impls.rs:277-281, crates/before/src/borsh_impls.rs:74, crates/before/src/borsh_impls.rs:8)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (lines 242-247 read against 213-218; `BorshDeserialize` is in scope at line 14; the `read_exact` idiom at 96-97, 214-215, 243-244; full `crate::version::skyline::` paths at 141, 277, 281 while tests.rs:16 imports the same items); executed: no
- Seen by: structure, prose; refutation: confirmed; history: no-rationale-found (8519dd4789 inlined the closure instead of calling `Rank::deserialize_reader`; the stray parenthesis is from 5d167a636). The prose lens also flagged the sentence "Borsh is a transport for the one wire form, never a second format" repeated at 199, 225, 259; the history pass disputes that half, since `borsh_impls` is a private module whose doc never renders while each impl doc renders on the public type's page, and I agree, so that half is dropped.
- Owner-gated: no

The composite decoder should read as "a rank, then a version, then the cross-check"; instead its first six lines are byte-identical to `Rank::deserialize_reader`'s body. Two prose typos ride along: line 74 closes a parenthesis never opened, and line 8's "in-memory wire form" contradicts itself.

Evidence:

   242          let rank = decode_rank_stream(|| {
   243              let mut byte = [0];
   244              reader.read_exact(&mut byte).map_err(Decode::Io)?;
   245              Ok(byte[0])
   246          })
   247          .map_err(decode_error)?;
    ...
    74      /// storage form, adopted without a copy).
    ...
     8  //! borsh stream while preserving their in-memory wire form.

Resolution: Replace 242-247 with `let rank = Rank::deserialize_reader(reader)?;`; add `fn read_byte<R: Read>(reader: &mut R) -> Result<u8, Decode>` used by `read_bit` and `Rank::deserialize_reader`; import `validate_from`, `validate_dominating_from`, and `Admission` from `version::skyline` at the top; drop the `)` at 74; at 8 write "while preserving their canonical bytes". Acceptance: the differential suite, the pair matrix, and the genre pins in borsh_impls/tests.rs stay green; no `crate::version::skyline::` path remains in the file.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| crate-root-8 | `crates/before/src/borsh_impls.rs:89-91` | Em-dashes in `//` comments across the partition | Colons or semicolons at the listed sites; one dash form in the Quickstart |
| crate-root-12 | `crates/before/src/borsh_impls/tests.rs:829-831` | borsh_impls/tests.rs: qualified paths beside their imports, hand counts, a likelihood argument, and formatting slips | Use the imported names; name the prefix; drop the tallies; rewrap |
| crate-root-19 | `crates/before/src/fold.rs:53-78` | `Vec::pop_if` removes both `expect`s and the `Option` juggling in the counter loop | `Vec::pop_if` and a labeled `continue` |
| crate-root-39 | `crates/before/src/shape.rs:180-186` | Four shape iterators carry an identical `finished` flag and byte-identical `size_hint` bodies | One `open_hint` helper, or route through `advance_refinement` |

## suanpan

### suanpan

14 entries (1 medium, 6 low, 7 nit); the full record is `evidence/partitions/suanpan.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: suanpan-21. Related findings in other documents: suanpan-24 (correctness: unchecked digit-position addition on 32-bit), suanpan-40 (verification-gap: the two mutants exclusions).

#### suanpan-9: The register-or-digit-0 dispatch is spelled out at five sites
- Where: crates/suanpan/src/accumulator.rs:183-190 (related: 201-208, 219-226, 237-244, 547-550; the enter-then-deposit pairs at 617-618, 621-622, 1262-1263, 1273-1274)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep `self.add_at(0, `: 187, 205, 223, 241, 549, each behind the same `if !self.quick_add(x)` guard); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (1c1cb67d wrote the bodies inline; d07fc20e added `#[inline]` for cross-crate inlining, which a helper must carry too)
- Owner-gated: no

`add_small`, `sub_small`, `add_u64`, `sub_u64`, and `fold_accum`'s register arm each spell "register when held, else digit 0"; the four public twins differ only in sign and width and the reader diffs seven lines per entry to confirm it. A named home states the dispatch once and moves no `touch` call.

Evidence:

       183	    pub fn add_small(&mut self, delta: i64) {
       184	        if delta != 0 {
       185	            let delta = i128::from(delta);
       186	            if !self.quick_add(delta) {
       187	                self.add_at(0, delta);
       188	            }
       189	        }
       190	    }

Resolution: `#[inline] fn add_word_scale(&mut self, delta: i128) { if !self.quick_add(delta) { self.add_at(0, delta); } }` called from the four entries and `fold_accum`; optionally `fn spill_value(&mut self, value: i128, shift: u64)` for the four enter-then-deposit pairs (`spill` generalized by a shift). Acceptance: one `self.add_at(0,` site; the metered pins unchanged.

#### suanpan-12: The shift split and its `expect` are duplicated at four sites; one helper would carry the 32-bit argument and the checked landing position
- Where: crates/suanpan/src/accumulator.rs:558-560 (related: 1232-1234, 1309-1311, 1468-1470, 317-323)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep `digit positions fit a usize`: 560, 1234, 1311, 1470, each behind the same `(shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS))` split); executed: no
- Seen by: structure, prose (as an open question); refutation: confirmed; history: no rationale found (introduced once in 092b149f and replicated per path; 9f68c475 counted "the four shift-position expects" without consolidating)
- Owner-gated: no

Eight public `# Panics` sections describe one hazard; the proof for the `expect` (only a target narrower than 64 bits can fail it, from `shift = 2^37`) lives in `add_wide_shl`'s rustdoc (319-323) and at none of the four `expect`s. Every `expect` message is a one-line proof; one helper puts the proof beside the panic, and it is where suanpan-24's checked landing position belongs.

Evidence:

       558	        let (digit_shift, bit_shift) =
       559	            (shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS));
       560	        let digit_shift = usize::try_from(digit_shift).expect("digit positions fit a usize");

Resolution: `fn split_shift(shift: u64) -> (usize, u32)` whose doc carries the 32-bit-only argument and the single `expect`, called at the four sites; `bit_shift` as `u32` is the idiomatic shift-amount type. Acceptance: one `digit positions fit a usize` string remains; the `# Panics` sections need no contract change.

#### suanpan-20: `add_u64_shl` / `sub_u64_shl` have never had a caller outside suanpan's own tests
- Where: crates/suanpan/src/accumulator.rs:1193-1196 (related: 1212-1215; crates/suanpan/src/claims.rs:178-193; crates/suanpan/src/lib.rs:213; crates/before/src/version/skyline/signed.rs:203-210)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep `\.add_u64_shl(` / `\.sub_u64_shl(` across crates/, src/, tests/: hits only at crates/suanpan/src/accumulator/tests/metered.rs:122, 125, 126; `git log --all -S'.add_u64_shl(' -- crates/before src tests` is empty; e4c9b083's message says the entries are what "the Small arm folds through", but `fold_signed_int` at signed.rs:205-206 folds `Int::Small` through `add_u64`/`sub_u64`, and every shifted fold in before goes through `add_magnitude_shl`/`sub_magnitude_shl`); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (the introducing commit's stated consumer never materialized; nothing since records a decision to keep them)
- Owner-gated: yes (stable public API)

Principle 3: the only things naming these two entries are the roster rows, the table row, two `# Panics` sections, and the metered witness leg that exists because the entries exist. `sub_accum_shl` likewise has no production caller (differential.rs:478, metered.rs:330, witnesses.rs:539 only) but is the symmetric twin of `add_accum_shl` (integral.rs:800, 1160), a defensible reason to keep it.

Evidence:

      1193	    #[inline]
      1194	    pub fn add_u64_shl(&mut self, word: u64, shift: u64) {
      1195	        self.add_shifted_word(word, false, shift);
      1196	    }

    signed.rs:
       205	        (Sign::Positive, Int::Small(n)) => acc.add_u64(*n),
       206	        (Sign::Negative, Int::Small(n)) => acc.sub_u64(*n),

Resolution: owner decision: retire both (their claims rows, the table row at lib.rs:213, and the second loop of `alternating_shifted_writes_cost_the_operand_not_the_gap`), or route before's shifted word-scale folds through them so the entry has the caller its introducing commit described. Acceptance: either no `add_u64_shl`/`sub_u64_shl` remain and `claims_are_total_over_the_public_surface` passes, or a production caller in before invokes them.

#### suanpan-22: The zero-run ledger reads as a type but lives as a field plus three methods on `Accumulator`
- Where: crates/suanpan/src/accumulator.rs:1404-1440 (related: 124-154, 1353, 674, 882, 1299; crates/suanpan/src/accumulator/tests/ledger.rs:34, 68)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (grep: `zero_runs` is mutated at 674, 882, 1299 (`clear`), 1353, 1421, 1424 (`insert`), 1419, 1435 (`remove`); `crop_runs` and `consume_run_at` read no other field); executed: no
- Seen by: structure; refutation: confirmed (an adoptable-now design proposal); history: no rationale found (f7596470 designed it as a field; no note mentions a type)
- Owner-gated: no

Types-first and single responsibility: the certificate semantics, disjointness, and containment invariants (a 30-line field doc) and three methods that touch only the map sit on the accumulator with `&mut` access to digits they never touch, so a reader verifies by inspection what a newtype would make structural. Touch counts are unaffected: the ledger is bookkeeping outside the digit denomination (lib.rs:163-165).

Evidence:

      1432	    fn consume_run_at(&mut self, above: usize) -> Option<usize> {
      1433	        let (&lo, &hi) = self.zero_runs.range(..above).next_back()?;
      1434	        if hi >= above {
      1435	            self.zero_runs.remove(&lo);
      1436	            Some(lo)
      1437	        } else {
      1438	            None
      1439	        }
      1440	    }

Resolution: a private `struct ZeroRuns(BTreeMap<usize, usize>)` (same file or a sibling `accumulator/ledger.rs`) with `record(lo, hi)`, `crop(from, to)`, `consume_below(above)`, `clear()`, and `iter()` for the ledger suite; move the certificate semantics onto it, and let the `Accumulator` field doc state only how the accumulator maintains it. Acceptance: no `self.zero_runs.insert/remove/range` outside the newtype; `ledger_invariants_hold_exhaustively` asserts the same clauses.

#### suanpan-30: The roster reaches into before's test file as the sole evidence for `add_small`/`sub_small`
- Where: crates/suanpan/src/claims.rs:96-99 (related: 123-132; crates/suanpan/src/claims/tests.rs:279-283; crates/suanpan/src/accumulator/tests/metered.rs:498-520; crates/before/tests/meter.rs:6622-6627)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (read: `BANDS` resolves through `crate_root().join(file)` at tests.rs:280; `add_small`/`sub_small` cite only `(BANDS, "accum_comb_touches_flat")`; every other `Witnessed` row has an `OWN` co-witness; `u64_comb_touches_are_flat_and_exact` is the exact u64 comb one signedness away); executed: no
- Seen by: structure (8), claims (46); refutation: confirmed both; history: already-known (deliberate at landing in 9e7b7ce3 and "flagged for the owner" in 318cd654, with no recorded ruling)
- Owner-gated: yes (a flagged owner decision; also whether standalone `cargo test -p suanpan` outside this workspace layout is a goal, since tests.rs:280-282 panics on a missing file)

Dependency direction: a substrate's verification reaching upward into its consumer's test-file layout couples suanpan's gate to before's envelope names, carries a special case in prose, and leaves the crate's headline rows (the crate page's opening example is built on them) without an in-crate exact pin. An exact in-crate pin is also stronger than before's x1.25 band.

Evidence:

        96	/// The digit-touch stream bands committed in the sibling meter suite —
        97	/// the one workspace-relative path in the roster; the binding tests
        98	/// resolve it from the manifest directory.
        99	const BANDS: &str = "../before/tests/meter.rs";
       123	    Claim {
       124	        op: "Accumulator::add_small",
       125	        table_cost: Some("amortized O(1)"),
       126	        evidence: Evidence::Witnessed(&[(BANDS, "accum_comb_touches_flat")]),
       127	    },

Resolution: an `i64` twin of `u64_comb_touches_are_flat_and_exact` driving `add_small(1)`/`sub_small(1)` on the `2^k - 1` cliff at k = 4096 and 8192 with the derived exact total, cited under both rows; keep or drop the `BANDS` citations as the owner rules (their `OWN` co-witnesses stand). Acceptance: `add_small` and `sub_small` each cite an `OWN` witness that reaches them; if `BANDS` is dropped, no path outside the manifest directory remains in claims.rs.

#### suanpan-35: Hand-rolled source scanning where `syn` is the mature tool
- Where: crates/suanpan/src/claims/tests.rs:66-72 (related: 73-108, 357-361; crates/surface-scan/src/lib.rs:19-25)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read: `fn_bodies` counts every brace character (88-98) and ends a body when the count returns to zero (99-102), so a `}` inside a literal ends a body early with no signal; the convention is stated in prose (66-72) and checked nowhere; on an exempted edge the truncation lands in the silent `(false, Some(_))` arm at 361. The refutation pass reports syn 1.0.109 and 2.0.117 already in Cargo.lock); executed: no
- Seen by: structure; refutation: confirmed (the masking clause is narrow; the steelman is real); history: deliberate-and-holds (the line-scan design and its loud-panic defense are stated at surface-scan lib.rs:19-25 and tests.rs:66-72; no syn discussion anywhere)
- Owner-gated: yes (a design proposal against a stated design)

Doctrine: prefer a dependency over hand-rolling; infrastructure that rests on a convention held in prose rather than a committed check is suspect. The scanners are short and panic loudly on an unclosed body, which is why this is a proposal, not a defect.

Evidence:

        66	/// A line scan over rustfmt-normalized shape, like the shared scanners:
        67	/// a definition is a line whose trimmed form starts with `fn ` (or
        68	/// `pub fn `), and its body ends where the braces opened since the def
        69	/// line balance. Brace counting is textual — the witness files carry no
        70	/// unbalanced brace in any literal or comment (format placeholders pair)
        71	/// — and an unclosed body panics rather than silently truncating the
        72	/// reach analysis.

Resolution: owner decision: a `syn`-based visitor (`full` + `visit`) in surface-scan yielding `#[test]` fn names, `pub fn` items by impl/module context, and method-call receivers by name, replacing the brace-balance convention; or keep the scanners and add a committed check that no witness file contains an unbalanced brace inside a literal or comment. Acceptance: a witness file containing `"}"` in an assert message changes no test's verdict.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| suanpan-1 | `crates/suanpan/Cargo.toml:4-4` | Edition 2021 under a 2024 workspace root | Migrate the editions together, or state the reason at the root |
| suanpan-11 | `crates/suanpan/src/accumulator.rs:542-546` | `negative: bool` threaded through three private paths yields seven `if negative { -x } else { x }` sites | One sign multiplier per function |
| suanpan-16 | `crates/suanpan/src/accumulator.rs:812-817` | Literal `32` and `128` beside the named `DIGIT_BITS` | `DIGIT_BITS` and `u128::BITS`; state the totality premise once |
| suanpan-18 | `crates/suanpan/src/accumulator.rs:966-967` | Destructure-and-retuple in `sign_magnitude`; a redundant clamp between `sign_magnitude_shl` and `read_digits` | `read_magnitude(0)`; one clamp |
| suanpan-29 | `crates/suanpan/src/claims.rs:36-36` | `pub(crate)` on `cfg(test)` roster items read only by their child module | Drop `pub(crate)` on the nine items |
| suanpan-31 | `crates/suanpan/src/claims.rs:101-109` | The `constant()` shorthand covers four rows while six rows of the same shape are longhand | Rename to `excluded` and use it for all ten rows |
| suanpan-36 | `crates/suanpan/src/claims/tests.rs:216-217` | The table locator hardcodes `Accumulator::`, an unreachable `else` arm, and `position` without the uniqueness the doc claims | Locate by `]({op})`; plain `rsplit`; assert one header |

### suanpan tests

8 entries (3 low, 5 nit); the full record is `evidence/partitions/suanpan-tests.md`. Related findings in other documents: suanpan-tests-25 (verification-gap: `assert_no_product` without a floor), suanpan-tests-26 (test-quality: the sign-flip test's citation).

#### suanpan-tests-6: the stream-replay loop is copied at eleven sites and the run-forming arm bodies are copied across two files
- Where: crates/suanpan/src/accumulator/tests/differential.rs:467-476 (related: crates/suanpan/src/accumulator/tests/differential.rs:223-232, crates/suanpan/src/accumulator/tests/differential.rs:614-652, crates/suanpan/src/accumulator/tests/ledger.rs:241-296)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep `for op in` lists the loop at differential.rs:228, 469, 474, 496, 524, 529, 566, 591, 664, 684, 689, 804; the arm bodies at differential.rs:627-645 and ledger.rs:263-280 compared by reading); executed: no
- Seen by: structure-prose [5], api-economics [31 test half]; refutation: confirmed (with the note that the two run-forming properties differ in sign-read schedule: every step in differential.rs, only on arm 4 in ledger.rs); history: no-rationale-found (the ledger copy landed six hours after the differential one as a "complement" with no argument for two)
- Owner-gated: no

The block "fresh accumulator, zero oracle, `for op in ops { apply }`" is the `Held::Stream` arm of `build_held` and is repeated at the plain-replay sites (differential.rs:467-476, 522-531, 564-568, 662-666, 682-691, 802-806). The four run-forming match arms (`add_wide_shl`, `sub_wide_shl`, `sub_magnitude_shl`, `add_small` with oracle updates) are byte-identical between `run_forming_shift_streams_match_the_bigint_oracle` and `ledger_invariants_hold_on_run_forming_streams` apart from the ledger's added sign-read arm. The second duplication is the riskier one: a change to one file's arms silently desynchronizes the ledger sweep from the oracle sweep that shares its documented shape.

Evidence:

       467	        let mut receiver = Accumulator::new();
       468	        let mut receiver_oracle = IBig::from(0);
       469	        for op in &receiver_ops {
       470	            apply(&mut receiver, &mut receiver_oracle, op);
       471	        }
       472	        let mut operand = Accumulator::new();
       473	        let mut operand_oracle = IBig::from(0);
       474	        for op in &operand_ops {
       475	            apply(&mut operand, &mut operand_oracle, op);
       476	        }

Resolution: add `fn replay(ops: &[Op], engine: bool) -> (Accumulator, IBig)` to tests.rs (the body of `build_held`'s `Stream` arm) and use it at every plain-replay site. Lift the run-forming arm into tests.rs as `enum RunFormingOp` with `arb_run_forming_op()` and `apply_run_forming(acc, oracle, &op)`; either keep two properties drawing from that one definition, or fold the differential one into the ledger proptest with a `sign_every_step: bool` parameter so both sign-read schedules survive under the checker. Acceptance: one definition of the replay loop and one of the run-forming arms; both suites green; the per-step-sign schedule still runs.

#### suanpan-tests-9: the ledger alphabet is a u8 matched against literals, with a hand-maintained cardinality and a catch-all arm
- Where: crates/suanpan/src/accumulator/tests/ledger.rs:110-111 (related: crates/suanpan/src/accumulator/tests/ledger.rs:126-172, crates/suanpan/src/accumulator/tests/ledger.rs:184-203, crates/suanpan/src/accumulator/tests/ledger.rs:212)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (read: `const LEDGER_OPS: u8 = 11`, arms `0..=9` plus `_`, `for op in 0..LEDGER_OPS` at :191); executed: no
- Seen by: structure-prose [4], api-economics [33 LEDGER_OPS half]; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`LEDGER_OPS` counts the arms of `ledger_op`'s `match` by hand. An arm added without a bump is never drawn; a bump without an arm falls into the `_` sign-read arm, doubling that op and inflating the sweep by about (12/11)^6 for no coverage; the catch-all hides both. Types-first: an `enum LedgerOp` with an exhaustive match makes the alphabet self-describing, the compiler catches a missing arm, the cardinality comes from an `ALL` array, and the shrunk `schedule` prints op names instead of numerals.

Evidence:

       110	/// Ops in the exhaustive ledger driver's alphabet.
       111	const LEDGER_OPS: u8 = 11;
    ...
       168	        _ => {
       169	            assert_eq!(acc.sign(), oracle_sign(oracle), "sign read");
       170	        }
    ...
       191	    for op in 0..LEDGER_OPS {

Resolution: introduce `#[derive(Clone, Copy, Debug)] enum LedgerOp { AddOne, SubOne, AddMax, SubMax, AddAt96, SubAt96, AddAt224, SubAt224, SubWord32At192, AddWord32At192, SignRead }`, a `const ALL: [LedgerOp; 11]` (or derive the list), an exhaustive `match`, and `schedule: Vec<LedgerOp>`. Acceptance: `ledger_op` has no `_` arm; `LEDGER_OPS` is gone; the sweep explores the same state count (check once in a scratch run that the number of `assert_ledger_invariants` calls is unchanged).

#### suanpan-tests-15: alternating_shifted_writes repeats one loop body four times, and the word-magnitude path is the one never doubled
- Where: crates/suanpan/src/accumulator/tests/metered.rs:92-183 (related: crates/suanpan/src/accumulator/tests/metered.rs:674-717, crates/suanpan/src/claims.rs:205-232)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (three `for shift in [32_000u64, 64_000]` blocks at :94-164 and a single-shift block at :165-182; claims.rs:205-232 cites this test for `add_magnitude_shl`/`sub_magnitude_shl` "at any shift"); executed: no
- Seen by: structure-prose [6]; refutation: confirmed; history: deliberate-but-expired (the block was born as a red pin whose number encoded the shift, so one shift sufficed; the cure kept the shape)
- Owner-gated: no

The test repeats "build, reset, 1_000 × (sub, add), `assert_eq!(touches, N)`, assert value" for the wide, word-shift, and occupied-digit-0 scenarios across two shifts, then runs the `*_magnitude_shl` scenario at one shift only. The roster cites this test as the shift-axis evidence for the magnitude-shift rows, and the doubling is what makes "whatever the shift" a measurement rather than a point; the word-magnitude path is the one row without it. The same file already table-drives `magnitude_dispatch_costs_its_width_path` with `&dyn Fn` closures.

Evidence:

        94	    for shift in [32_000u64, 64_000] {
        95	        let mut acc = Accumulator::new();
        96	        acc.add_wide_shl(&one, shift);
        97	        touch_meter::reset();
        98	        for _ in 0..1_000 {
        99	            acc.sub_wide_shl(&one, shift);
       100	            acc.add_wide_shl(&one, shift);
       101	        }
    ...
       166	    let five = UBig::from(5u8);
       167	    let mut acc = Accumulator::new();
       168	    acc.add_magnitude_shl(&five, 32_000);
       169	    touch_meter::reset();
       170	    for _ in 0..1_000 {
       171	        acc.sub_magnitude_shl(&five, 32_000);
       172	        acc.add_magnitude_shl(&five, 32_000);
       173	    }

Resolution: table-drive the four scenarios over `[32_000, 64_000]` with an inline tuple of setup/sub/add closures, expected per-pair cost, and label (as the dispatch test does, with `#[allow(clippy::type_complexity)]`); expected totals stay 5_000 / 3_000 / 5_000 / 3_000. Acceptance: one loop body; the magnitude word path is asserted at both shifts; green.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| suanpan-tests-1 | `crates/suanpan/proptest-regressions/accumulator/tests/differential.txt:7-9` | differential.txt carries three seeds whose shrink records name an arm its property cannot draw | Comment the carried seeds, or prune them naming the split (owner's word) |
| suanpan-tests-5 | `crates/suanpan/src/accumulator/tests/differential.rs:415-415` | constructor and conversion idioms are spelled two ways for the same role | One spelling per role |
| suanpan-tests-19 | `crates/suanpan/src/accumulator/tests/metered.rs:599-600` | em-dashes in two assert messages and four code comments | Colon or semicolon at six sites |
| suanpan-tests-21 | `crates/suanpan/src/accumulator/tests/metered.rs:855-855` | the known-bad fold model hardcodes the decision threshold 3 where SIGN_DECIDED is importable | `partial.abs() >= SIGN_DECIDED` |
| suanpan-tests-24 | `crates/suanpan/tests/amortized_sequences.rs:8-8` | amortized_sequences.rs housekeeping: an orphaned "S1" label, qualified Ordering, f64 over exact counters, an unnamed 0.10, and a cross-crate copy of the helper | Import `Ordering`; rename `s1`; `i128` with a named bound |

## The instruments

### Oracle and laws

13 entries (1 medium, 4 low, 8 nit); the full record is `evidence/partitions/oracle-laws.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: oracle-laws-2, oracle-laws-24. Related findings in other documents: oracle-laws-19 (documentation: "door" undefined across 237 uses).

#### oracle-laws-1: The oracle Clock's one-to-one mirror claim is false: `has_seen` has no caller, the observer trio and `receive` mirror deleted or renamed production methods, and the module doc claims a single omission
- Where: crates/before/src/oracle/clock.rs:9-11 (related: crates/before/src/oracle.rs:9-12, crates/before/src/oracle/clock.rs:123-143, crates/before/src/clock/tests.rs:172-174 and 618, crates/before/src/testing/optrace.rs:93 and 140, crates/before/src/oracle/version.rs:99)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn 'has_seen\|happens_before\|concurrent_with\|\.receive('` over `crates/before`: `has_seen` has only its definition and two comment mentions; production `clock.rs` has no `has_seen`, `happens_before`, `concurrent_with`, or `receive`; `git show a8c4d395f` removes all three observers and the `ia.has_seen(&msg)` call; `git show dc88e755` renames `receive` to `recv`); executed: no
- Seen by: scaffolding [5], adequacy [15], structure-prose [23], refutation (new item 2); refutation: reframed (the `Default for oracle::Version` sub-claim is refuted: clippy's `new_without_default` under the gate's `-D warnings` requires it, and no `allow` exists); history: deliberate-but-expired (the mirror was true at b3320b385; a8c4d395f and dc88e755 changed production; b3f09baa0 reflowed the sentence without re-truing it)
- Owner-gated: no

Both mirror sentences state something the file below them contradicts (Principle 5: prose speaks in the present tense and says what is). `oracle::Clock` carries `has_seen` with no caller anywhere, `happens_before`/`concurrent_with` whose production counterparts were deleted, and `receive` where production has `recv`, while production's `ticks`, `forks`, `absorb`, `recv_all`, `absorb_all`, and `sync_all` have no oracle counterpart; `optrace::run` and `step_impl` already translate between the two spellings. The module-level sentence says the oracle omits only the byte codec, but it also omits `Span`, `Ranked`, the `causally` queries, `distance`, and `lag`. The observer differential's doc in `clock/tests.rs` names `has_seen` and `happens_before` although its body calls neither.

Evidence:

         9	/// The reference [`Clock`](crate::Clock): the paper's recursive trees,
        10	/// mirroring the optimized type's API one-to-one so the differential tests can
        11	/// drive both with the same script.

         9	//! can serve as differential ground truth. It mirrors the target's **semantic**
        10	//! surface (construction, operations, ordering, operators) and omits the one
        11	//! purely *representational* concern that carries no semantics: the byte codec
        12	//! (`encode`/`decode`).

       123	    pub fn has_seen(&self, msg: &Version) -> bool {
       124	        msg.leq(&Base::ZERO, &self.version, &Base::ZERO)
       125	    }
       140	    pub fn receive(&mut self, msg: Version) {

    clock/tests.rs (the consumer whose doc names methods its body never calls):
       172	    /// The clock observers match the oracle's: `has_seen` is `msg <= version`,
       173	    /// `happens_before` is the strict causal order, and `concurrent_with` is
       174	    /// incomparability.
       186	        prop_assert_eq!(ia.version() >= msg, oa.version() >= msg_oracle);

Resolution: Delete `has_seen` (then `Version::leq` at oracle/version.rs:99 can drop from `pub(super)` to private; its only other caller is `PartialOrd for Version`). Either rename `receive` to `recv` and replace `happens_before`/`concurrent_with` at their four call sites (oracle/tests.rs:386-387, 401; clock/tests.rs:188) with `partial_cmp`-based spellings, or keep them and rewrite oracle/clock.rs:9-11 to say the oracle mirrors the paper's operations under its own names, listing the divergences (owned-`Version` messages, `receive` for `recv`, no n-ary or absorb entries). Rewrite oracle.rs:9-12 to name what the oracle mirrors (the paper's operations) rather than claim one omission. Reword clock/tests.rs:172-174 to name the comparisons the body performs (`>=`, `<`, `concurrent`); that site belongs to the clock partition and should be carried there. `Default for oracle::Version` stays. Acceptance: `grep -rn has_seen crates/before` returns nothing; both mirror sentences are true of the code beneath them; `just gate` clean.

#### oracle-laws-17: Law predicates that `unwrap` lose the failure's name
- Where: crates/before/src/laws.rs:699-699 (related: crates/before/src/laws.rs:712-714, 725-726, 1219, 1222, 1314, 1426, 1491, 1848, and the helper at 1705; crates/before/src/testing/algebraic_laws/tests.rs:38-44; crates/before/fuzz/fuzz_targets/fuzz_laws.rs:7-8 and 86-92)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep over laws.rs for `.unwrap()\|.expect(`; ten sites inside `laws!` bodies plus one in `operand_spans`; the let-else idiom is already used at 661-664, 1041-1044, 1111-1114, 2698-2700; the two `split_last().expect(...)` at 1822 and 1870 are infallible); executed: no
- Seen by: scaffolding [3], instrument-correctness [40]; refutation: confirmed; history: no-rationale-found (the let-else idiom predates the unwraps)
- Owner-gated: no

`assert_laws!` and `drive!` emit `law violated: {name}` only when the predicate returns `false`; a panic inside the predicate reports a file and line instead, breaking the fuzz target's stated contract ("A violated law is a panic naming the law"). The same antecedent (a meet/join pair is ordered) is handled two ways in one file.

Evidence:

       699	        let definitional = hull == Span::new(&meet, &join).unwrap();

       661	        let Ok(span) = Span::new(b, b) else {
       662	            // A version is always ordered with itself.
       663	            return false;
       664	        };

    algebraic_laws/tests.rs:
        41	            prop_assert!(law($($input),+), "law violated: {}", name);

Resolution: Use the let-else `return false` idiom at 699, 713, 725, 726, 1219, 1222, 1314, 1426, 1491, 1848 (a small `fn ordered(lo, hi) -> Option<Span<'_>>` beside `within` keeps the bodies short); `operand_spans` at 1705 may keep its `expect` or return `Option`. Acceptance: `grep -n 'unwrap()\|expect(' crates/before/src/laws.rs` returns only the two infallible `split_last` sites (and optionally 1705).

#### oracle-laws-25: The law-group totality pin is a source-text scan where a compile-time tie exists, and the scan is not total over the syntax `laws!` accepts
- Where: crates/before/src/laws/tests.rs:36-61 (related: crates/before/src/laws.rs:136-166, 186-206, 192; crates/before/src/testing/diff_ops.rs:954-964 and crates/before/src/testing/diff_ops/tests.rs:169, which carry the same pattern)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified for the gap (laws/tests.rs:42 matches only lines whose trimmed text begins `pub static `, while the matcher at laws.rs:192 accepts `$(#[$group_meta:meta])* pub static`; `grep -n '^\s*#\[.*pub static' crates/before/src/laws.rs` finds no attributed invocation header today, so the hole is latent); assessed for the replacement (not compiled); executed: no
- Seen by: scaffolding [4], instrument-correctness [39]; refutation: confirmed (4's compile-time tie dissolves 39); history: deliberate-and-holds for the scan's existence (b800537d chose it because the surface-totality extractor walks function-like items only; 5c990b90 retained it explicitly; b3f09baa0, the owner, added the one-line `pub static` convention to keep it accurate), but no record weighs a compile-time reference, so this is a fresh proposal against an owner-authored mechanism
- Owner-gated: yes (replaces an instrument; the replacement must demonstrate it catches what the scan catches)

Infrastructure is suspect when it carries its own stabilization conventions (the one-line `pub static` spelling rule on the macro) and reimplements a capability the compiler provides. The reverse direction (a rostered phantom) is already a compile error because `registered_names` chains `$group.iter()`; only the unrostered-group direction needs a pin, and a name reference gives it without a file read, a parse, or a formatting rule. The scan's stated reason (surfacecheck cannot see statics) argues against relying on surfacecheck, not against a compile-time tie. The scan is also not total: `#[allow(dead_code)] pub static EXTRA: (a: &Version);` on one line compiles, registers, and escapes the prefix match.

Evidence:

        41	    for line in text.lines() {
        42	        if let Some(rest) = line.trim_start().strip_prefix("pub static ") {
        43	            let name: String = rest
        44	                .chars()
        45	                .take_while(|c| c.is_alphanumeric() || *c == '_')
        46	                .collect();

    laws.rs:
       187	    // In both the matcher and the transcriber, attributes and the declaration
       188	    // share a line: the totality pin's source scan reads any line starting `pub
       189	    // static` as a group declaration, and must see the invocations' headers
       190	    // only, never this definition.
       192	        $(#[$group_meta:meta])* pub static $group:ident: ($($param:ident: $ty:ty),+ $(,)?);

Resolution: Have `emit_registration` (already expanded from the roster, `#[cfg(test)]`) also emit `mod rostered { pub(super) use super::{VERSION_SOLO, ...}; }` from the roster, and have `laws!` append a `#[cfg(test)] fn` (or `const _`) whose body references `rostered::$group`; an unrostered group then fails `cargo check --tests` at its own declaration, the text scan and the convention comment at laws.rs:187-190 go, and `REGISTERED_GROUPS` dissolves or reduces to the alias module. `diff_ops.rs` carries the same scan pattern and would take the same tie. If the scan is kept instead, strip leading `#[...]` groups before matching (or match `pub static ` anywhere on lines without `$`) so it is total over the macro's syntax. Acceptance: laws/tests.rs has no `fs::read_to_string`; declaring `laws! { pub static PHANTOM: (a: &Version); fn x { true } }` without a roster entry fails to compile; or, in the fallback, the attributed-header case fails `every_law_group_is_registered`.

Construction: Insert `#[allow(dead_code)] pub static EXTRA: (a: &Version); fn extra_law { true }` as a `laws!` block without adding `EXTRA` to `for_each_law_group!`, run `laws::tests::every_law_group_is_registered`: it passes, while no driver, organic drive, or fuzz loop ever executes `extra_law`.

Synthesis note: testing-diff-gen-6 (this document) keeps the sibling scan in diff_ops for the ad-hoc-driven-group case; the compile-time tie proposed here catches that case as well (an unrostered group fails `cargo check --tests` at its declaration whether or not a bespoke test drives it). If the tie lands, both scans dissolve; if not, share one helper as testing-diff-gen-6 proposes.

#### oracle-laws-26: The fold's retention-arm witness lives beside the laws while `fold.rs` has no tests
- Where: crates/before/src/laws/tests.rs:63-135 (related: crates/before/src/laws/tests.rs:1-3; crates/before/src/fold.rs:18-81; crates/before/src/party/tests.rs:89-115 and 172-290; crates/before/src/clock/tests.rs:65-93; crates/before/src/laws.rs:2402-2437)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (`grep -n 'mod tests\|#\[cfg(test)\]' crates/before/src/fold.rs` returns nothing; read the three deterministic witnesses of the same arm); executed: no
- Seen by: scaffolding [6]; refutation: confirmed (also: no committed known-bad artifact fails the laws on this feed; the dropped-group variant is convicted only by the differential comparison); history: no-rationale-found (fold.rs was born without tests in b4093db21, "Pure refactor: all fold/join/meet/sync suites green"; f42ce3d4 placed this witness in laws/tests.rs without saying why)
- Owner-gated: no

A capability in the wrong layer generates per-consumer copies: the retention arm is one branch of `balanced_try_fold` (fold.rs:65-72) and needs one witness over a trivial combiner at its declaration, yet it is witnessed three times over parties and clocks, and `fold.rs`, whose module doc calls itself "the one home for the counter discipline", carries no test at all. This test also sits in a module whose doc scopes it to the collection's own invariants while it exercises production fold behavior.

Evidence:

         1	//! Guards on the law collection itself (the laws are *asserted* by the drivers
         2	//! in [`crate::testing`]'s algebraic-laws suite and by the fuzz workspace; here
         3	//! we pin the collection's own invariants).

        63	/// The conservation laws' deterministic witness: the feed order
        64	/// [a, alias(a), b, alias(b), c] holds both `join_all` conservation laws
        65	/// while its hand-back contains a *coalesced* group.

    fold.rs:
         1	//! The balanced binary-counter reduction every n-ary fold runs on: the one home
         2	//! for the counter discipline, so a hardening of the fold shape reaches every
         3	//! fold at once.

Resolution: Add `fold/tests.rs` with one witness of the retention arm over integers (a combiner that refuses a marked pair), plus the dropped-group known-bad variant from party/tests.rs:182-227 rewritten over the same combiner and held convicted there; then reduce this test to the one clause the laws module owns (the hand-back contains a coalesced group, the justification for stating conservation over unions), or move that clause beside the fold witness and delete this test; widen or honor the module doc at laws/tests.rs:1-3. This is the natural home for the pin oracle-laws-2's second branch asks for. Acceptance: `fold.rs` has `mod tests;` with the retention-arm witness and its known-bad conviction; laws/tests.rs contains only collection-level pins or its doc says otherwise.

Synthesis note: Hosts the pin oracle-laws-2's second branch would need; the two land together whichever branch is chosen.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| oracle-laws-3 | `crates/before/src/oracle/party.rs:82-82` | `unreachable!("party overlap")` is a label, not a proof, and a denormal literal reaches the arm | State the premise in the message, or make `sum` total |
| oracle-laws-5 | `crates/before/src/oracle/version.rs:12-12` | Three spellings of the grow-cost tuple with a hand-maintained "matches" comment | `pub(crate) Cost` returned from `grow_for_test`; drop `GrowCost` |
| oracle-laws-6 | `crates/before/src/oracle/version.rs:114-154` | `join_off` and `meet_off` are one recursion differing only in the leaf combiner | One `lattice_off` helper taking the leaf combiner |
| oracle-laws-7 | `crates/before/src/oracle/version.rs:211-212` | Qualified `crate::` paths where an import would do, `std`/`core` mixing, the legacy `DefaultHasher` path, and a mid-clause comment wrap | Imports; one `iter::once` spelling; `std::hash::DefaultHasher` |
| oracle-laws-8 | `crates/before/src/oracle/version.rs:449-453` | Em-dashes in `//` line comments at ten sites | Colons or ` -- `; batch with the crate-wide sweep |
| oracle-laws-11 | `crates/before/src/oracle/tests.rs:46-55` | Pool indices are drawn as `0..64` and reduced modulo the population at fourteen sites where the crate elsewhere uses `prop::sample::Index` | `prop::sample::Index` at the fourteen sites |
| oracle-laws-15 | `crates/before/src/laws.rs:248-258` | `le` is `le_by` at one type | Keep `le`; delete `le_by` |

### Meter core

4 entries (2 medium, 1 low, 1 nit); the full record is `evidence/partitions/meter-core.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: meter-core-11. Related findings in other documents: meter-core-2 (verification-gap: the roster-wide canonicality pin), meter-core-3 (documentation: `Packed`'s doc).

#### meter-core-7: Freeze-regime widths are hand-derived copies of `FREEZE_ALLOWANCE_DIGITS` with no binding test and no per-family freeze or promotion pin
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

Synthesis note: envelopes-a-19 (this document) reaches the same 288 and 608 literals from the test side and argues the oracle's independent statement should stay; the two are compatible: derive the constants in src from `FREEZE_ALLOWANCE_DIGITS` as this entry asks, and name the widths once in `skyline_flatness` with a comment naming the src constant each mirrors.

#### meter-core-6: Construction bodies are pasted across families that differ by one knob
- Where: crates/before/src/meter.rs:780-787 (related: meter.rs:948-955, 832-834 and 945-947, 842-845 and 958-961; the re-arm block loop at 1746-1752, 1834-1840, 1919-1923, 1974-1978; the memo-site sequence at 999-1006, 1161-1168, 1195-1204, 1233-1240, 1318-1325; the id-site block at 1029-1040, 1350-1361, 1108-1123, 1277-1288, 1479-1488; reveal_comb 1392-1413 vs reveal_comb_hifloor 1429-1451; seam_plunge 2726-2737 vs seam_plunge_control 2777-2788; the lean/turn spines at 1992-2004, 2499-2509, 3050-3058, 3309-3317 with `DENSE_SUFFIX_DIGIT_STRIDE` 1799 and `JUMP_PAIR_DIGIT_STRIDE` 2967)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (every listed range read side by side); executed: no
- Seen by: scaffolding, structure-prose; refutation: reframed (the `seam_stop_descent` asserts are a different set, not part of the triplication; the four lean/turn spines share a skeleton but differ in phase and turn leaf, so they are parameterizable rather than byte-identical); history: no rationale found (the paired-return convention at 57-65 concerns coupled pairs, not helper extraction; the module's own `ascend_spine`, `hole_region`, `gap_spine`, `parked_unit_spine`, `seam_stop_descent` show extraction as the practice)
- Owner-gated: no

Legibility and maintenance: a control family that must stay geometrically identical to its red twin except for one parameter is safest as the same code with the parameter passed in, which the module already does for `ascend_cliff`/`ascend_cliff_plateau`. Byte-identical copies: `collapse_hole` and `site_hole` share their per-unit event and id loops verbatim and `site_hole`'s root-site prefix is `copy_hole`'s; the four-node `for base in [&arm, &one, &settle, &one]` block appears four times; the memo-site bit sequence five times; the id-site block five times; `reveal_comb_hifloor` duplicates `reveal_comb`'s body to change one leaf; `seam_plunge_control` repeats `seam_plunge`'s three asserts. The two 33-stride constants carry the same value and near-identical derivation docs. Extraction is byte-preserving, so no committed shape, envelope, or provenance moves, and the module doc's reason for leaving paired generators as-is (63-65) does not apply.

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

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| meter-core-5 | `crates/before/src/meter.rs:728-730` | Idiom inconsistencies: `debug_assert!` guards in three helpers whose siblings `assert!`, `Base::from(0u8)` beside `Base::ZERO`, a `saturating_sub` after an `assert!`, two `Ordering` paths, thirty qualified `suanpan::UBig`s, a rustfmt-folded comment, two undocumented recursive helpers, one caller named in `bitlen`'s doc, and a repeated `Ticks` string round-trip | `assert!` in the three helpers; `Base::ZERO`; one `Ordering` import; `use suanpan::UBig` |

### Registry and tier2

6 entries (2 medium, 3 low, 1 nit); the full record is `evidence/partitions/meter-registry-tier2.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: meter-registry-tier2-9. Related findings in other documents: meter-registry-tier2-14 (documentation: tier2.rs's candidate-coding prose), meter-registry-tier2-11 (verification-gap: the hand rosters and `strum::VariantArray`), meter-registry-tier2-13 (test-quality: the tautological roster test), meter-registry-tier2-7 (verification-gap: probe builds as adequacy witnesses).

#### meter-registry-tier2-12: The cliff-fan family prices a path-sum walk no production operation performs, and its only named pin re-runs the comb stream
- Where: crates/before/src/meter/registry.rs:1653-1669 (related: registry.rs:115-117, 897-899; crates/before/src/meter.rs:342-356; crates/before/src/version/skyline.rs:11-12, 119-121; crates/before/tests/meter.rs:6629-6641; crates/before/src/version/skyline/tests.rs:555-556; crates/before/src/meter/tier2/tests.rs:531)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (read the generator doc at meter.rs:342-356, skyline.rs:11-12 and 119-121, `accum_fan_touches_flat` at tests/meter.rs:6637-6641, and every `CliffFan` use in the skyline and tier2 suites by grep); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: deliberate but expired (2b0884c0 landed the fan for entry/exit path-sum accumulation under the min-lifted coding and reused the comb stream on purpose; faf3cd0a removed every production walk that maintains path sums; the registry row, written four days later, restated the pre-flag-day hazard)
- Owner-gated: yes (removing an instrument family and a public `Shape` variant)

The fan's adversarial content is entry/exit accumulation of per-node stored bases: the generator doc says "a walk that maintains a running path sum (enter: add the stored base; leave: subtract it) crosses the `2^k` carry boundary *twice per tooth*" and "Consecutive-leaf *values* stay cliff-free". The stored coding has no per-node numbers (skyline.rs:11-12: "Internal nodes carry no numbers") and the only walk that materializes path sums is the test- and meter-only transcoder (skyline.rs:119-121). The family's threat model ended at the flag day. Its row points at "the in-crate skyline and tier2 suites' pinned envelopes", which hold it only as a corpus member, and the one test bearing its name (`accum_fan_touches_flat`) calls `comb_run` twice and never builds `Shape::CliffFan`. Principle 3: machinery outlived the constraint that justified it; Principle 5: the row and the `FamilyId` doc ("`n` sibling carry excursions funded by one stored magnitude") describe a deleted coding's hazard.

Evidence:

      1653	            FamilyId::CliffFan => FamilySpec {
      1654	                name: "cliff-fan",
      1655	                shapes: &[Shape::CliffFan],
      1656	                coverage: Coverage::EnvelopeOnly {
      1657	                    reason: "kernel-seam probe: sibling carry excursions funded by one \
      1658	                             stored magnitude, priced by the in-crate skyline and tier2 \
      1659	                             suites' pinned envelopes",

    meter.rs:
       355	/// input. Consecutive-leaf *values* stay cliff-free (`2^k ↔ 2^k + 1`): the fan
       356	/// prices entry/exit accumulation, the boundary comb prices leaf deltas.

    tests/meter.rs:
      6637	    fn accum_fan_touches_flat() {
      6638	        let small = comb_run(4_096, 50_000);
      6639	        let large = comb_run(8_192, 100_000);
      6640	        assert_flat("fan", &small, &large, envelope::COMB_MILLI_PER_DELTA);

Resolution: owner's call between two accurate states. (a) Dissolve the family: remove `FamilyId::CliffFan`, `Shape::CliffFan`, the `cliff_fan` generator, its size pin, and `accum_fan_touches_flat`; keep the shape in the agreement corpora only if it is re-justified as a coding corpus member rather than an adversary. (b) Re-justify it against a production walk that exists and add a two-point band built through `Shape::CliffFan` on that kernel, cited from the family's `Bands::Priced`, with a known-bad kernel that fails it. Either way the `EnvelopeOnly`/`Unbanded` reasons must name what exists (finding 10). Acceptance: either `grep -rn CliffFan crates/before` is empty, or a convention-named flatness test builds `Shape::CliffFan` at two scales over a public or documented-internal operation and the parity test passes.

#### meter-registry-tier2-16: The 1-Lipschitz coding pin is implied pointwise by the subadditivity pin over the same emitters and populations; its leaf clause asserts a count, not containment
- Where: crates/before/src/meter/tier2/tests.rs:27-80 (related: tier2/tests.rs:271-324, 326-373, 417-453, 526-533; crates/before/src/meter/board/cell.rs:72-75)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read both checks and every test that calls them; the implication is arithmetic: `so.total + 2 <= sa.total + sb.total` implies `so.total <= sa.total + sb.total + 4·(sa.leaves + sb.leaves)` for any leaf counts; cell.rs:72-75 cites the Lipschitz pin by name); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed with one correction (the Lipschitz adversarial grid at 313-318 runs larger operands than the subadditivity grid at 526-533, so the merge must carry them) and one new point (the leaf clause); history: deliberate but expired (the Lipschitz pin landed at 6814d77b as the board's denomination premise; the subadditivity lemma landed four hours later at eec23b79 and was ruled the lemma of record; no reason to keep the weaker clause is recorded)
- Owner-gated: no

`check_join_meet_lipschitz` asserts `so.total_bits <= sa.total_bits + sb.total_bits + 4·(sa.leaves + sb.leaves)`; `check_subadditive` asserts `so.total_bits + 2 <= sa.total_bits + sb.total_bits` under the same `EMITTERS` table (54, 450). Every operand pair passing the second passes the first, and the four `*_hold_the_lipschitz_pin` tests run the same three proptest strategies as the subadditivity tests plus a four-shape grid. The only content the Lipschitz helper adds is the leaf clause at 59-66, and that clause asserts `so.leaves < sa.leaves + sb.leaves` while its doc (44) and message (61-62) speak of boundaries "contained in the union of the inputs'": a count inequality is a consequence of containment, not containment (an output that drops one union boundary and adds one foreign boundary passes). Principle 3: a check whose every failure is already a failure of a stronger committed check catches nothing on its own; two derived constants for one lemma is the maintenance cascade the doctrine names. Two structural leftovers vanish with the merge: `EMITTERS` and the emitter fns sit under the subadditivity rule (326) yet are consumed at 54, and `operator_meet`'s doc (369-370) copies `operator_join`'s "first emitter of record" while both docs scope the emitters to "the subadditivity pins" only.

Evidence:

        37	const JOIN_MEET_BOUNDARY_SLACK_BITS: u64 = 4;
        59	        assert!(
        60	            so.leaves < sa.leaves + sb.leaves,
        61	            "{name}: {} output leaves reach the input leaf total {} + {}: \
        62	             an output boundary appeared outside the inputs' boundaries",
        67	        let ceiling =
        68	            sa.total_bits + sb.total_bits + JOIN_MEET_BOUNDARY_SLACK_BITS * (sa.leaves + sb.leaves);
       346	const JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS: u64 = 2;
       437	    assert!(
       438	        so.total_bits + JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS <= sa.total_bits + sb.total_bits,
       369	/// The public `&` operator: the subadditivity pins' first emitter of record

Resolution: move the leaf clause into `check_subadditive` and state it as what it is (a leaf-count bound; or check containment by comparing boundary positions through the oracle's dyadic intervals if containment is the claim the board's denomination rests on); rename the merged helper to state both clauses; delete `JOIN_MEET_BOUNDARY_SLACK_BITS`, its derivation, `check_join_meet_lipschitz`, and the four Lipschitz tests; carry the Lipschitz grid's larger operands (dense(512), bigroot(200, 100), hugeleaf(500), cliff_comb(64, 64)) into `adversarial_crosses_hold_subadditivity`; move the sentence "the statement the board's input denomination of the packed-output mutators rests on" onto the subadditivity constant's doc; re-point cell.rs:72-75 and this file's module doc (1-3) to the subadditivity pin; fix the two emitter docs. Acceptance: one coding-lemma helper and one constant; four fewer tests; `grep -n Lipschitz crates/before/src` finds no referent in meter or board; the leaf clause survives inside the merged check and its doc matches its assertion.
Construction: dominance needs no run. To confirm the moved leaf clause is live, weaken it to `<=` in a scratch build and observe `empty_pair_is_the_subadditivity_equality_case`'s operands (1 + 1 leaves, output 1 leaf) still pass while `so.leaves == sa.leaves + sb.leaves - 1` cases distinguish `<` from `<=`; restore.

#### meter-registry-tier2-8: `FamilySpec.denominator` and `closed_form` are prose stored as data that nothing reads; `shapes` is checked in one direction only
- Where: crates/before/src/meter/registry.rs:1022-1047 (related: registry.rs:1959-1988; crates/before/src/meter/registry/tests.rs:111-129, 159-167; crates/before/tests/amp_board_smoke.rs:370)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rnE '\.(denominator|closed_form)\b'` over crates/before and before-fuelscape: the only `.denominator` hits are `run.denominator` on the `Run` struct in tests/meter.rs:6489-6520, no `.closed_form` reader; `.spec()` outside the registry is read at shard.rs:506 and amp_board_smoke.rs:75 for `coverage` and at :364 for `bands`; `.shapes` is read only at registry/tests.rs:120; the `AXIS_BANDS` disposition is bound to `_` at amp_board_smoke.rs:370 and checked non-empty at registry/tests.rs:164-167); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed (severity lowered to low: dead weight and a rot risk, not a live defect); history: no rationale found (both fields landed at cc84df7e with no consumer)
- Owner-gated: yes (public struct fields under the `meter` feature)

The struct is documented as "the answers every instrument derives from", but no code outside registry.rs reads `denominator` or `closed_form`; no test compares the denominator strings against the board's per-cell denomination (board/cell.rs decides that mechanically) or resolves the pins `closed_form` names in prose ("the meter suite pins the size closed form"). As string fields they render in no rustdoc and are checked by nothing, so a renamed pin or a re-denominated column leaves them stale silently. `shapes` is read once, in the "cited at least once" direction, so a family row citing a shape its bundle no longer builds passes. Principle 3: a row of record earns its place by what reads it.

Evidence:

      1022	/// One family's row of record: the answers every instrument derives
      1023	/// from.
      1041	    /// The denominator of record: what this family's priced readings are
      1042	    /// charged against.
      1043	    pub denominator: &'static str,
      1044	    /// The closed-form hook, where one exists: the quantity computable two ways
      1045	    /// and the pin that compares them.
      1046	    pub closed_form: Option<&'static str>,

Resolution: owner's call between consuming and demoting. Consume: render each column's denominator from the spec in the board header so the smoke suite can pin its presence, and make `closed_form` carry a test name the smoke suite's scan resolves like band names. Demote: move both into each variant's rustdoc (where the `FamilyId` docs already carry this altitude of prose) and shrink `FamilySpec` to the fields instruments consume (`name`, `shapes`, `coverage`, `bands`); make `AXIS_BANDS: &[&str]` with each disposition as a comment above its entry. For `shapes`, have board/family.rs's bundle build record the `Shape` variants it constructs and assert equality with `spec().shapes` in the smoke suite. Acceptance: every remaining `FamilySpec` field has a reader outside registry/tests.rs, or the fields are gone and the variant docs carry the sentences; a `shapes` row gaining a variant its bundle does not build fails a test.

#### meter-registry-tier2-15: The sizer suite builds shapes through the raw generators, bypassing the registry door
- Where: crates/before/src/meter/tier2/tests.rs:15-17 (related: tier2/tests.rs:123, 166, 199, 314-317, 526-533, 557, 571; crates/before/src/meter/board/tests.rs:54, 80, 111-113, 162-163, 212-213, 353-358; crates/before/src/meter/registry.rs:12-21, 302-308)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (read the import and call sites; `grep -nE '\b(dense|bigroot|hugeleaf|cliff_comb|...)\('` in board/tests.rs shows the same pattern there); executed: no
- Seen by: structure-prose; refutation: reframed (board/tests.rs, also a child of `meter`, does the same, so the registry doc's "the module's own unit tests" is ambiguous about child suites rather than false); history: 608dea84 routed fifteen files through `Shape` and did not touch tier2/tests.rs; no recorded reason
- Owner-gated: no

Two child suites of `meter` (`tier2/tests.rs` and `board/tests.rs`) import and call the private generators directly, which visibility allows only because they are descendants of `meter`. The registry doc says `Shape::builder` is "the constructors' only caller outside the module's own unit tests" and that no "kernel unit test" can build a shape except through the registry. The sizer and board suites are sibling instruments' tests, not the generators' unit tests, so the door's universality claim is true only under a generous reading. Consistency with the registry invariant: a reader auditing who can build an unregistered shape finds two exceptions the wording does not anticipate.

Evidence:

        15	use crate::meter::{
        16	    alt_spine, bigroot, cancelling_chain, cliff_comb, cliff_fan, dense, hugeleaf, wide_tooth_comb,
        17	};

Resolution: route both suites through `Shape` (`Shape::Dense.packed1(512)`, `Shape::Bigroot.packed2(200, 100)`, `Shape::Hugeleaf.packed1(500)`, `Shape::CliffComb.packed2(64, 64)`, and so on), after which registry.rs:14-15 is exactly true; or name the child suites in the registry doc as the sanctioned exception. Acceptance: no generator name is imported into tier2/tests.rs or board/tests.rs, or the registry doc names them.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| meter-registry-tier2-17 | `crates/before/src/meter/tier2/tests.rs:48-58` | The sizer-of-a-Version idiom is copied fifteen times; `built_view` and `dense` are fully qualified beside imported siblings; the two kernel wrappers are a copy-paste pair | A `size_of(v)` helper; imports; one `kernel_emit` |

### The board: frame

8 entries (2 medium, 3 low, 3 nit); the full record is `evidence/partitions/board-frame.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: board-frame-1, board-frame-25. Related findings in other documents: board-frame-5 (documentation: the root doc's verbatim copies), board-frame-26 (claim: the validation index's touch-ceiling wording).

#### board-frame-6: Fifteen re-exported ceiling constants and the `Currency` type have no reference outside the module; ten more are public only so the module doc can link them
- Where: crates/before/src/meter/board.rs:279-292 (related: crates/before/src/meter/board/ceilings.rs:69-229, crates/before/src/meter/board/currency.rs:27-141, justfile:258, justfile:260-264)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git grep -n -w <NAME> -- '*.rs' '*.py' '*.json' justfile ':!crates/before/src/meter/board' ':!crates/before/src/meter/board.rs' ':!.agent-notes'` per name: `DEFAULT_SCALE` and `LADDER_TOP_SCALE` have code consumers (examples/amp_board.rs:18,194; benches/common/sidecar.rs:9,109,113); the eight `MAX_*`/`HEAP_FLAT_ALLOWANCE_BYTES` constants and `ByCurrency`, `Floors`, `Liveness` appear only as intra-doc links in board.rs itself; the remaining fifteen constants and `Currency` appear nowhere; the `Floors`/`Liveness` hits outside are English words in test docs; justfile:258 runs `cargo doc` under `RUSTDOCFLAGS="-D warnings"` and :260-264 names the `private_intra_doc_links` lint); executed: no
- Seen by: structure-prose; refutation: reframed (seventeen corrected to fifteen); history: no-rationale-found (public since 7d81a248a; 89cf4c8d5 re-exported them as a pure-movement invariant, not a decision)
- Owner-gated: yes (the `meter`-feature board surface is public; narrowing is an API change)

Of the 25 constants in the `pub use ceilings` block, two have code consumers outside the module, eight are public only because the module doc links them under `-D warnings`, and fifteen (`ASCEND_CLIFF_*`, `CAPACITY_MODEL_*`, `FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL`, `INDEX_PROBE_SCAN_BITS`, `MACHINE_WORD_MAGNITUDE_BITS`, `MIN_EXPONENT_DENOM_GROWTH`, `MIRROR_WIDE_*`, `SCAN_FLOOR_BITS_PER_INPUT_BYTE`, `SCAN_TOUCH_FLOOR_BITS`, `TEXT_BYTES_PER_RADIX_UNIT`, `TEXT_PIPELINE_LIMB_OPS_PER_VALUE`, `TICKS_BOARD_COUNT`) are referenced nowhere outside the module. `Currency` likewise. A `pub` item earns its visibility by naming a consumer (Principle 3); here the doc justifies the API rather than the reverse, and every `pub const` is surface a future reader must assume someone depends on.

Evidence:

       279	pub use ceilings::{
       280	    ASCEND_CLIFF_MIN_TICKS_HEAP_BYTES_PER_INPUT_BYTE, ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE,
       281	    CAPACITY_MODEL_CEILING, CAPACITY_MODEL_FLOOR, DEFAULT_SCALE,
       282	    FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL, HEAP_FLAT_ALLOWANCE_BYTES, INDEX_PROBE_SCAN_BITS,
       283	    LADDER_TOP_SCALE, MACHINE_WORD_MAGNITUDE_BITS, MAX_GROWN_STACK_SEGMENTS,
       292	pub use currency::{ByCurrency, Currency, Floors, Liveness};

Resolution: Narrow the fifteen unused constants to `pub(super)` (sibling submodules already import them via `super::ceilings::`) and drop them from the `pub use`. For the doc-linked items, either decide that the judgment constants and currency types are part of the meter API (state that at the `pub use`) or refer to them in prose by backticked path without a link and narrow them too. `ShardSpawner`, `bench_cells`, `BenchCell`, `BenchMode`, `HeapMeter`, `Summary`, and the `run*` functions stay public: they are the example's and bench's entry points. Acceptance: every name in the `pub use ceilings` block has at least one non-doc use outside the module or is absent from the block; `just docs` stays clean.

#### board-frame-22: Five hand-rolled preorder decoders of the version stream under `meter/board/`, two re-implementing the zigzag rule; three are dissolvable onto the public shape walk
- Where: crates/before/src/meter/board/defect.rs:67-86 (related: crates/before/src/meter/board/operand.rs:23-45, 57-78, 89-125, 156-191, crates/before/src/version/skyline/signed.rs:179-185, crates/before/src/shape.rs:88-98, crates/before/src/version.rs:773)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (the `while pending > 0 { ... // skyline flag: 0 internal, 1 leaf ... expect("a stored stream is canonical") }` skeleton appears at defect.rs:72-84 and operand.rs:29-43, :62-76, :95-103, :163-172; operand.rs:108-118 and :177-188 re-implement `unzigzag_base` (signed.rs:179-185, `pub(super)` there); `Version::shape` (version.rs:773) yields `Plateau { rise: Option<Rise>, depth }` with `None` for a zero delta and the first rise absolute (shape.rs:40-42, 88-98); `git log -1 46eb64f9` is 2026-08-19, after the decoders were written); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed (reframed from three copies to five); history: deliberate-but-expired (the decoders predate the public shape walk; 46eb64f9 integrated the board only at the coverage tables and never weighed the instrument's own decoders)
- Owner-gated: no

The per-leaf action each function is about sits under twelve lines of identical scaffolding, and two copies track production's zigzag convention by hand. `stored_nonzero_deltas`, `value_content_bytes`, and `stored_bases`' height pass can ride `Version::shape()` with no wire decoding (`rise.is_some()` is exactly a nonzero delta; a running sum of rises is the absolute height); the two that need stream positions or code widths (`last_leaf_flag_pos`, `mandatory_limbs_stream`) can share one private leaf iterator. Trade-off to state: floors derived through `Version::shape()` rest on a rostered public operation with differential coverage instead of a private decoder, which is the firmer footing for a floor stated in terms of what the operation must do.

Evidence:

        72	    while pending > 0 {
        73	        pending -= 1;
        74	        let flag = pos;
        75	        let internal = !bits.bit(pos); // skyline flag: 0 internal, 1 leaf
        76	        pos += 1;
        77	        if internal {
        78	            pending += 2;
        79	            continue;
        80	        }
        81	        let (_, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");

    [operand.rs:108-113, the zigzag re-implementation]
       108	                let odd = code.bit(0);
       109	                let magnitude = if odd {
       110	                    (code + 1u32) >> 1u32
       111	                } else {
       112	                    code >> 1u32
       113	                };

Resolution: In operand.rs express `stored_nonzero_deltas` as `v.shape().skip(1).filter(|p| p.rise.is_some()).count()` and the two absolute-height reconstructions as a running sum over `rise`; add one private `stored_leaves(v) -> impl Iterator<Item = (flag_pos, code, next_pos)>` for the code-width floor and defect.rs's last-leaf position; delete both zigzag re-implementations. The operand.rs functions are outside this partition's file list; the pattern is reported once here. Acceptance: exactly one `// skyline flag: 0 internal, 1 leaf` loop remains under `meter/board/`; `radix_units_match_hand_counts` and `mandatory_limbs_match_hand_counts` pass unchanged; no `>> 1u32` zigzag arithmetic remains in operand.rs.

Synthesis note: board-families-floors-judge-24 (this document) is the operand.rs half of the same duplication, with the fuller construction; land them as one change. `Version::shape()` as the source for the count walks is the footing both entries prefer.

#### board-frame-24: `BenchCell::denominator_bytes` re-implements `measure`'s denominator rule by hand and runs the body when the denominator does not need it
- Where: crates/before/src/meter/board/export.rs:69-85 (related: crates/before/src/meter/board/measure.rs:97-122, crates/before/src/meter/board/export.rs:47-51, 57-62, crates/before/src/meter/board/cell.rs:161-199)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (export.rs:73-84 and measure.rs:97-122 both `match cell.denom`, both use `content.unwrap_or(cell.input_bytes)` for `Input`, both read `(spec.output_bytes)(result)` and call `assert_honest_text` under `output_is_text`; export.rs:72 runs the body before the match and the `Input` arm at :74 never reads `result`; `body()` (:47-51) and `denominator_bytes` (:70-71) repeat the same `.expect(...)`); executed: no
- Seen by: scaffolding, structure-prose (two items); refutation: confirmed, severity medium to low (the exhaustive `match Denom` forces both sites to handle a new variant; the residual drift is identical handling); history: no-rationale-found (the copy is from 54b68b4fd; 7999b8ec2 already paid the cascade once, updating both sites)
- Owner-gated: no

export.rs:57-62 promises the bench denominator is "exactly as the board's measurement does", but the sameness is held by two parallel code paths, and a change to either arm desynchronizes the judge's time exponents from the board's counter exponents (Principle 6: the denominator is the one thing binding the two instruments). The body also runs unconditionally although the doc gives output read-back as its reason; the `Input` arm ignores it.

Evidence:

        72	        let result = (cell.body)();
        73	        match cell.denom {
        74	            Denom::Input => self.data.content_bytes.unwrap_or(cell.input_bytes),
        75	            Denom::Io(spec) => {
        76	                let output_bytes = (spec.output_bytes)(result.as_ref());
        77	                if let Some(text) = spec.text {
        78	                    if text.output_is_text {
        79	                        assert_honest_text(self.op, output_bytes, text.radix_units);
        80	                    }
        81	                }
        82	                cell.input_bytes + output_bytes
        83	            }
        84	        }

Resolution: One method on `Cell` (or `Denom`), e.g. `fn exponent_denominator(&self, op: &str, content: Option<usize>, result: impl FnOnce() -> Box<dyn Any>) -> usize`, that performs the content-or-input choice, the lazy output read-back, and the honesty assertion; `measure` derives `exp_denom_bytes` from it and `export` returns it; lift the repeated `.expect` into one private `fn cell(&self) -> Cell`. Acceptance: exactly one `match` over `Denom` computes a denominator in the board module; for a `Denom::Input` cell, `denominator_bytes` does not invoke the body (a counting body in a unit test); the bench sidecar's denominator file is byte-identical before and after at the record scales.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| board-frame-13 | `crates/before/src/meter/board/ceilings.rs:286-298` | `both_present_nodes` is an operand-content walk living in the constants module | Move `both_present_nodes` to operand.rs |
| board-frame-16 | `crates/before/src/meter/board/cell.rs:204-308` | `Cell`'s three constructors repeat the same struct literal, and its two mutually exclusive heap models are flat fields whose precedence lives in the judge | One private `with_denom` constructor; consider a `HeapModel` enum |
| board-frame-20 | `crates/before/src/meter/board/coverage/tests.rs:78-79` | Em-dashes in two `//` comments and one assert message | Decide crate-wide, then `; ` and ` -- ` |

### The board: families, floors, judge

11 entries (1 medium, 7 low, 3 nit); the full record is `evidence/partitions/board-families-floors-judge.md`. Related findings in other documents: board-families-floors-judge-1, -8, -12, -17 (documentation), board-families-floors-judge-10, -11, -21 (verification-gap: the NA declarations, the unwatched cells, the heap exponent leg).

#### board-families-floors-judge-24: operand.rs writes one preorder walk four times and the zigzag decode twice, duplicating `skyline::signed::unzigzag_base`
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

Synthesis note: board-frame-22 (this document) is the defect.rs copy and the `Version::shape()` route; one change closes both.

#### board-families-floors-judge-5: Generator minimum widths duplicated as bare literals at the family call sites
- Where: crates/before/src/meter/board/family.rs:662-673 (related: family.rs:725, 733; meter.rs:1909-1911, 2445-2447, 2612-2614)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (read the three generators' `assert!` lines against the three `.max(..)` clamps); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The DominatedUndercut, WideArming, and PlateauPuncture arms clamp their parameter with `.max(128)`, `.max(10)`, `.max(10)`, each described as "the generator's minimum width", while the minima live only as literals inside the generators' `assert!`s. A change to a generator's bound leaves the call-site clamp silently stale: too low panics under scale-down, too high stops the clamp from ever binding. Named constants over magic numbers; one home per fact.

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

#### board-families-floors-judge-6: Board membership is declared in the registry and re-derived by a 19-variant `unreachable!` arm
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

Synthesis note: board-ops-render-2 (this document) is the `designed()` copy of the same roster; the `BoardFamily` enum closes both.

#### board-families-floors-judge-7: `scatter` and `weave` hand-roll the balanced fork expansion `Party::forks` provides
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

#### board-families-floors-judge-13: floors.rs re-inlines its own helpers: five scan-floor casts, twin limb constructors, eight near-identical `Floors` literals, six zero-or-NA shapes, two rate types for one dimension
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

#### board-families-floors-judge-15: Vocabulary: "honest" as an undefined soundness criterion (29 sites, two rendered), "mint" (7), "backstop", "trued to", "door", "seams", "genre"
- Where: crates/before/src/meter/board/floors.rs:541-544 (related: floors.rs:5, 112, 136, 148, 210, 629-630; family.rs:5, 288, 293, 418, 448, 816, 1108, 1123, 1145, 1175; judge.rs, operand.rs)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep counts: `honest(ly)` 18/7/2/2/0 in floors/family/judge/operand/measure; `mint*` 7, all family.rs; `backstop` floors.rs:112, 136; `trued` 148, 210; `door` family.rs:5; `seams` 288, 293; `genre(s)` 10; crate-wide 167 `honest` and 66 `mint*`); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed (measure.rs carries no `honest` hit, only the identifier `assert_honest_text`); history: no-rationale-found (the rules entered writing-style.md on 2026-08-19, after every site; "genre" is the working vocabulary of .cargo/mutants.toml's header)
- Owner-gated: yes (crate-wide convention: a partition-local edit leaves the meter suite speaking two dialects; one owner ruling is the right unit)

"honest"/"honestly" carries a technical meaning here (a floor no conforming implementation can read below; a declaration that reflects work performed) that is defined nowhere, and two sites render in the legend (WHY_SCAN_SYNC_VERSIONS "so no party-bytes floor is honest"; NA_SEG_CEILING_ONLY "the honest floor is zero"). "mint" is used for constructing a value at seven family.rs sites, two of them `assert!` messages. "backstop", "trued to", "door", and "seams" are register transplants with no adversary, calibration, or physical seam behind them. The vocabulary rules name each; the most frequent is "honest", which deserves a definition rather than 29 repetitions.

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

#### board-families-floors-judge-16: Policy ceremony restated at every site: 50 `segments: seg_ceiling_only()` fields, five `Cell` fields copied one by one into `Sample`, twelve cfg reader wrappers
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

Synthesis note: board-frame-16 (this document) is the `Cell` constructor half of the same field-by-field cascade.

#### board-families-floors-judge-23: `judge_window`'s ceiling resolution is a match split across four statements; `Score` duplicates `Fit`; the span test is written twice
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

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| board-families-floors-judge-3 | `crates/before/src/meter/board/family.rs:547-548` | Idiom slips in family.rs and two floors.rs signatures | Bind the cross pair; rename the shadow; name the operand bytes; import `skip_subtree` |
| board-families-floors-judge-4 | `crates/before/src/meter/board/family.rs:614-615` | Em-dashes in `//` comments and in rendered legend strings | ` -- ` in seven comments; colon or semicolon in three legend strings |
| board-families-floors-judge-26 | `crates/before/src/meter/board/operand.rs:261-263` | `radix_units_party` carries a branch for a value `Party` cannot hold | Delete the empty-stream branch |

### The board: ops, render, shards, worst

9 entries (7 low, 2 nit); the full record is `evidence/partitions/board-ops-render.md`. Related findings in other documents: board-ops-render-15 (verification-gap: the segments column), board-ops-render-19 (performance: the ranking pin's sweep sharing), board-ops-render-16 (verification-gap: the shard codec's tamper table).

#### board-ops-render-2: `designed()` is a shape-axis declaration living in the operation module, hand-mirroring the registry's envelope-only roster with no test binding the two
- Where: crates/before/src/meter/board/ops.rs:99-187 (related: registry.rs:569-571; family.rs:765-786; export.rs:144-149; benches/common/sidecar.rs:131-132; justfile:466, 1003)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (grep: `designed(` has one caller, export.rs:147, inside the `BenchMode::Pinned` arm; `BenchMode::Pinned` is selected only in benches/common/sidecar.rs:131-132; the gate's board stream at justfile:466 runs acceptance and the pin only; `bench-judge` appears only in the `all` recipe at justfile:1003; tests.rs:1099 calls `bench_cells(0.02, BenchMode::Full)`, which short-circuits `designed`); executed: no
- Seen by: scaffolding [2], adequacy [24], structure-prose [33]; refutation: reframed (the registry's own checklist locates the arm in the board family module; family.rs:765-786 already enumerates the same envelope-only variants; the public `Coverage` route would expose `OpGroup`); history: no-rationale-found (designed() was written into the monolithic board.rs, dropped into ops.rs by the 89cf4c8d split, and left behind when cc84df7e moved the roster into the registry; the 19-name `unreachable!` arm is what cc84df7e added to keep the match exhaustive)
- Owner-gated: no (a move to family.rs is internal; moving the designation into the public `Coverage::Board` variant would be)

The designed pairings are "declared per shape, on the shape axis" but live in the operation module; the registry's adding-a-family checklist says the arm lives in "the board family module"; the `unreachable!` arm hand-lists nineteen envelope-only variants that family.rs:765-786 already lists for the bundle build; and nothing in `just test` exercises the arm, so a family promoted to `Coverage::Board` while still listed here panics only under `just bench-judge`. Principle 5 (no hand-maintained enumerations of a fact another table owns) and Principle 6 (every hole becomes a committed check).

Evidence:

       102	/// Declared per shape, on the shape axis, so the pinned bench subset is
       103	/// derived — a shape added to the axis must answer which groups it was
       104	/// built against (the exhaustive match), and the subset follows. The

       162	        // Envelope-only families never reach the board's product, so
       163	        // they have no designed diagonal.
       164	        FamilyId::WideToothComb
       ...
       182	        | FamilyId::LatentLadder => unreachable!(
       183	            "{kind:?} is envelope-only in the registry: the bench mirror derives its \
       184	             subset from the board roster alone"
       185	        ),

    registry.rs:
       569	/// Adding a family: the [`FamilyId::spec`] and [`FamilyId::index`] arms and —
       570	/// for a board column — the board family module's bundle-build and
       571	/// designed-diagonal match arms are compiler-forced from the variant. What the

Resolution: (1) Add a unit test in tests.rs: for every `FamilyId::ALL` variant and every `OpGroup`, `catch_unwind(|| designed(kind, group)).is_err()` equals `matches!(kind.spec().coverage, Coverage::EnvelopeOnly { .. })`, so the arm and the registry cannot disagree in either direction. (2) Move `designed` (and `OpGroup`, or a re-export) into family.rs beside the bundle-build match, where the registry doc already says it lives; the envelope-only arm then sits beside its twin. Acceptance: flipping one envelope-only variant's registry coverage to `Board { .. }` fails `just test`; `grep -n 'fn designed' ops.rs` is empty and registry.rs:569-571 reads true.

Synthesis note: board-families-floors-judge-6 (this document) is the same board-membership roster from the family module; its `BoardFamily` enum dissolves this entry's `unreachable!` arm as well, so the two are one refactor (the board-families summary's open question 3 adds the base-size constant to the same declaration).

#### board-ops-render-4: The operation table is a two-thousand-line function under a clippy allow, with its row templates repeated verbatim
- Where: crates/before/src/meter/board/ops.rs:192-194 (related: ops.rs:1840-2153 (sixteen rejection rows), 1141-1212 (four placement rows), 1094-1112, 1517-1536, 1820-1838 (three hash rows), and the eight declared-model branches of finding 1; callers shard.rs:125, 147, 415, export.rs:142, tests.rs:1114, 1241)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read; closure-to-fn-pointer coercion in a `static` initializer under the pinned 1.97.1 toolchain was not compile-checked because builds were not permitted); executed: no
- Seen by: scaffolding [14], structure-prose [29], structure-prose [30]; refutation: confirmed (severity of the template finding lowered to low: the flat table's per-row greppability is a counter-value in its own right and no contract is misstated); history: no-rationale-found (the `vec!` form and the allow are from the board's landing commit 7d81a248a)
- Owner-gated: no

`ops()` builds the whole row table as a `Vec<Op>` on every call under `#[allow(clippy::too_many_lines)]`, although `Op` is `&'static str` + `OpGroup` + a non-capturing `fn` pointer and the table is data. Inside it, the sixteen byte- and text-rejection rows share one body shape (`expect_err`, `assert!(matches!(err, ..), "the placed defect is ..., not {err:?}")`, `(err, fed)`), the four placement rows differ only in the method called, the three hash rows are identical up to the hashed type, and the declared-model attachment repeats at eight sites. Legibility (the bar for finished code is "obviously, reviewably correct"): a reader must check thirteen copies of the rejection contract to know it is stated once.

Evidence:

       192	#[allow(clippy::too_many_lines)]
       193	pub(super) fn ops() -> Vec<Op> {
       194	    vec![

      1848	                Some(Cell::new(n, floors, move || {
      1849	                    let err =
      1850	                        Version::decode(&fed[..]).expect_err("a truncated stream is rejected");
      1851	                    assert!(
      1852	                        matches!(err, Decode::Truncated),
      1853	                        "the placed defect is the cut, not {err:?}"
      1854	                    );
      1855	                    (err, fed)
      1856	                }))

Resolution: Two independent levers, either or both. (a) Make the table data: `pub(super) static OPS: &[Op] = &[ .. ]` (or split into `version_rows()`, `party_rows()`, `clock_rows()`, `rejection_rows()` at the existing section headers), removing the allow and the per-call construction. (b) Add row-builder helpers beside the table: a generic `decode_rejection(fed, floors, decode: fn(&[u8]) -> Result<R, Decode>, expected: fn(&Decode) -> bool, defect_name)` (Version, Span, Party, and Clock all decode `&[u8]` to `Result<_, Decode>`), a `placement_row(span, probe, method)`, a `hash_row(value)`, and a `declared_for(kind, cell)` step. Weigh the greppability trade the owner prefers. Acceptance: no `too_many_lines` allow in ops.rs; the `matches!(err, Decode::..)` assertion appears once per defect kind; the smoke suite's per-family cell counts and the rendered row names are unchanged.

#### board-ops-render-7: Rows reach around `party_pair()` for one side's length or bytes at thirteen sites
- Where: crates/before/src/meter/board/ops.rs:449-450 (related: ops.rs:930, 961, 1292, 1308, 1314, 1432, 1450, 1522, 1672, 2006, 2024, 2255; family.rs:1036-1039, 1061-1062)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep `parties\.as_ref()\.map` over ops.rs and family.rs returns the fourteen sites listed); executed: no
- Seen by: scaffolding [10], structure-prose [31]; refutation: confirmed (the discarded decode runs at prepare, outside measurement, so the cost is bench setup, not a reading); history: no-rationale-found (accessor and raw reach are coeval at 7d81a248a)
- Owner-gated: no

`FamilyData::party_pair()` returns both parties decoded and only their combined length, so every row that prices one party alone decodes both, discards one, and re-reads the raw slot positionally for the length or bytes; `clock()` in family.rs does the same. Legibility: a two-line idiom repeated fourteen times is an accessor waiting to exist, and the positional raw-slot reach is a second way of reading a slot the accessor already owns.

Evidence:

       449	                let (a, _, _) = f.party_pair()?;
       450	                let n = f.parties.as_ref().map(|(a, _)| a.len())?;

    family.rs:
      1036	    pub(super) fn party_pair(&self) -> Option<(Party, Party, usize)> {
      1037	        let (a, b) = self.parties.as_ref()?;
      1038	        Some((decode_party(a), decode_party(b), a.len() + b.len()))

Resolution: Add per-side accessors on `FamilyData` (`party_a(&self) -> Option<(Party, usize)>`, `party_b`, and `party_a_bytes(&self) -> Option<&[u8]>` for the rejection rows), or have `party_pair` return per-side lengths; use them at every site, including `clock()`. Acceptance: `grep -c 'parties.as_ref().map' ops.rs family.rs` is zero outside the accessors.

#### board-ops-render-14: render.rs houses the measurement seam beside the renderer, and its "one scale guard" has two verbatim copies
- Where: crates/before/src/meter/board/render.rs:164-197 (related: render.rs:1-6; shard.rs:405-408; export.rs:134-137; measure.rs:1-2; shard.rs:78)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (`grep -rn 'scale > 0.0' src/meter/board/` returns render.rs:167, shard.rs:406 (identical predicate and message), export.rs:135 (identical predicate, message "bench cells: ..."); shard.rs:78 already imports `assert_scale` and calls it in `emit_shard`); executed: no
- Seen by: scaffolding [7], scaffolding [8], structure-prose [37]; refutation: confirmed; history: the guard's three copies are coeval at c4695a459 (the uniqueness claim was false at birth); the helpers' home in render.rs expired at 289e14a4, which moved the board driver into shard.rs and left `assert_scale`, `build_pair`, `measure_cell` behind
- Owner-gated: no

The module's first sentence names two responsibilities and the code bears it out: `assert_scale`, `build_pair`, and `measure_cell` are the per-cell measurement discipline every shard child drives, wrapping `measure` from measure.rs ("The measurement engine"), while the rest of the file is the matrix. `assert_scale` is documented as "the one scale guard, shared by every entry that measures", and `merge_samples` and `bench_cells` re-inline the identical predicate (the latter with a different message). Modules have a single responsibility; a doc claim of uniqueness the code does not have is a Principle 5 failure.

Evidence:

         1	//! The measurement discipline and the printed matrix: how one grid cell is
         2	//! measured and judged, and how a whole board's judged cells render.

       164	/// The one scale guard, shared by every entry that measures.
       165	pub(super) fn assert_scale(scale: f64) {
       166	    assert!(
       167	        scale > 0.0 && scale.is_finite(),
       168	        "amp-board: scale must be a positive finite number"
       169	    );
       170	}

    shard.rs:
       405	    assert!(
       406	        scale > 0.0 && scale.is_finite(),
       407	        "amp-board: scale must be a positive finite number"
       408	    );

Resolution: Call `assert_scale` from `merge_samples` and `bench_cells`; move `assert_scale`, `build_pair`, and `measure_cell` into measure.rs beside `measure` (shard.rs already imports from both modules), leaving render.rs with `Summary`, `row`, and `render_results`, and rewrite its first sentence to name the one responsibility. Acceptance: one `scale > 0.0 && scale.is_finite()` in the board; render.rs imports nothing from `measure`, `ops`, or `family`.

#### board-ops-render-20: Literals shadow named constants: `1.0` for `DEFAULT_SCALE`, `/ 64` for `OVERLAP_FOLD_INPUT_DIVISOR`, `0.02` for the smoke scale, and the seed and empty-version packed sizes as arithmetic
- Where: crates/before/src/meter/board/worst.rs:67-67 (related: ceilings.rs:451; shard.rs:73, 575, 596-598; tests.rs:520, 1099, 1113; ops.rs:2235; family.rs:170, 1059, 1063, 1084, 1091; ops.rs:400, 436, 453, 526, 923, 957, 993, 1440, 1665, 1676, 2183, 2204; tests/amp_board_smoke.rs:30)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (ceilings.rs:451 `pub const DEFAULT_SCALE: f64 = 1.0;`; worst.rs:54 imports only `HEAP_FLAT_ALLOWANCE_BYTES` and `LADDER_TOP_SCALE`; family.rs:170 `pub(super) const OVERLAP_FOLD_INPUT_DIVISOR: usize = 64;`; ops.rs:2235 uses it by name with a `.max(MIN_SIZE_PARAM)` clamp the test at tests.rs:520 omits; `SMOKE_SCALE` lives in the separate smoke test binary); executed: no
- Seen by: scaffolding [9], scaffolding [13], adequacy [25], structure-prose [38], instrument-correctness [53]; refutation: confirmed; history: deliberate-but-expired for the `1.0` (no base-scale constant existed at 493ef543; `DEFAULT_SCALE` arrived with 9e36dd280 and the site was missed); no-rationale-found for the rest (the divisor landed two minutes before the test that spells `/ 64`, in separate commits)
- Owner-gated: no

`WORST_MAP_SCALES` spells the ladder base as `1.0` while the acceptance ladder uses `DEFAULT_SCALE` for the same point, so a change to `DEFAULT_SCALE` would leave the pin checked at a scale the ladder no longer measures. `join_all_overlap_upfront_test_reads_flat` claims parity with the `party_join_all_overlap` row but spells the divisor as `64` and omits the row's `MIN_SIZE_PARAM` clamp. Two tests use `0.02` for the smoke scale a different binary names. ops.rs and family.rs encode the seed party's and empty version's packed sizes as `n + 1`, `n + 2`, and `2` at a dozen sites, and `i as u64 % 7 + 1` at 526 carries an unnamed modulus. Named constants over magic numbers.

Evidence:

    worst.rs:
        67	pub const WORST_MAP_SCALES: [(&str, f64); 2] = [("default", 1.0), ("acceptance", LADDER_TOP_SCALE)];

    tests.rs:
       520	        let count = a_bytes.len() / 64;

    ops.rs:
       400	                    n + 1,

Resolution: `("default", DEFAULT_SCALE)` with the import, and one label pair for both renderers; `(a_bytes.len() / OVERLAP_FOLD_INPUT_DIVISOR).max(MIN_SIZE_PARAM)` in the test; a lib-side `PROBE_SCALE` constant for the `0.02` sites; `SEED_PARTY_BYTES`/`EMPTY_VERSION_BYTES` constants in family.rs (or compute `Party::seed().encode().len()` once) used by both files; name the `% 7` modulus. Acceptance: no `"default", 1.0` in worst.rs; no `/ 64` in tests.rs; no bare `n + 1`/`n + 2` denominators in ops.rs.

#### board-ops-render-23: `check_with` re-validates the pin table's structure at runtime; the committed test already does
- Where: crates/before/src/meter/board/worst.rs:597-607 (related: worst.rs:587-592; shard.rs:638-644; tests.rs:1238-1284)
- Class / severity / confidence: vestigial / low / medium
- Provenance: verified (tests.rs:1250-1260 read: per scale, `assert_eq!(pinned, ops)` and the total-count assertion imply both a known scale label and no duplicate `(scale, op)` key); executed: no
- Seen by: structure-prose [42]; refutation: confirmed; history: no-rationale-found (the loop and `worst_rankings_pin_is_well_formed` landed together in 493ef543; nothing says whether the loop exists so `just worst-cases-pin` stands alone without the test binary)
- Owner-gated: no

The runtime loop asserts every `WORST_RANKINGS` entry names a known scale label and is not a duplicate; `worst_rankings_pin_is_well_formed` pins, per scale, that the entries equal the operation table exactly and that the total is ops × scales, which excludes both. A runtime recompute over deterministic committed data is not defense in depth once committed coverage exists (Principle 3), and the `# Panics` clauses in `check_with` and `check_worst_map` then document a case the gate already excludes.

Evidence:

       597	    let mut seen = BTreeSet::new();
       598	    for (scale, op, _) in WORST_RANKINGS {
       599	        assert!(
       600	            WORST_MAP_SCALES.iter().any(|(label, _)| label == scale),
       601	            "worst-case pin: unknown scale label {scale:?} on {op}"
       602	        );
       603	        assert!(
       604	            seen.insert((*scale, *op)),
       605	            "worst-case pin: duplicate entry for {op} at the {scale} scale"
       606	        );
       607	    }

Resolution: Drop the loop and the corresponding `# Panics` clauses, or keep it with a one-line comment naming the reason it must hold without the test suite (the pin recipe running standalone). Acceptance: either the loop is gone and the two `# Panics` sections no longer mention a malformed pin, or the loop carries its standalone justification.

#### board-ops-render-28: Six probe tests hand-build `Sample`s with per-test `PROBE_NA` constants and repeated in-function imports; the radix-work formula and a trivial wrapper are duplicated
- Where: crates/before/src/meter/board/tests.rs:305-308 (related: tests.rs:22-31, 35-38, 172-174, 314-316, 325-351, 408-457, 651-682, 771-802, 874-906, 1013-1044; measure.rs:117-118)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep: full `Sample { .. }` literals at 326, 438, 657, 777, 881, 1019, each with a `PROBE_NA` at 325, 413, 655, 775, 879, 1017; in-function `use super::` blocks at 305-308, 408-411, 651-654, 771-774, 874-878, 1013-1016, of which the tests at 650, 770, and 1012 carry no cfg gate); executed: no
- Seen by: structure-prose [34]; refutation: confirmed; history: no-rationale-found (the probe tests accreted one at a time, each copying the previous one's closure and imports)
- Owner-gated: no

Each probe test defines its own `PROBE_NA` and a `sample` closure filling every `Sample` field by hand, so each tripwire's signal (two or three readings and one declaration) is buried under about twenty-five lines of defaults; the imports are repeated inside ungated test bodies although the file header explains only why the limb-meter names are gated; the `R` formula `n_io + radix_units_version + TEXT_PIPELINE_LIMB_OPS_PER_VALUE * stored_bases.len()` is copied from measure.rs at three sites; and `version_of` wraps `Packed::version` with no added meaning.

Evidence:

       305	    use super::floors::na;
       306	    use super::judge::evaluate;
       307	    use super::measure::Sample;
       308	    use super::{ByCurrency, Floors};

       325	        const PROBE_NA: &str = "probe: the limb exponent leg alone is under test";
       326	        Sample {

Resolution: One module-level `fn probe(reason: &'static str, denom: usize, readings: ByCurrency<Option<u64>>) -> Sample` with all-NA floors and defaults, and per-test one-line tweaks; hoist the shared imports to the top (keeping only the limb-meter names gated); replace `version_of(&x)` with `x.version()`; expose measure's radix-work computation as a small `pub(super) fn` or add one test-local helper. Acceptance: one `Sample {` literal in tests.rs; no `use super::` inside test bodies except cfg-gated ones; `version_of` gone.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| board-ops-render-13 | `crates/before/src/meter/board/render.rs:118-126` | render.rs idiom nits: guard-then-`expect` match arms, an inline `std::collections::` path, and an em-dash in one printed legend line | `if let` chains; import `BTreeSet`; replace the em-dash |
| board-ops-render-21 | `crates/before/src/meter/board/worst.rs:176-183` | Hand-rolled run grouping in `worst::fold` where `slice::chunk_by` expresses it | `results.chunk_by(\|a, b\| a.op == b.op)` |

### The surface roster

10 entries (1 medium, 6 low, 3 nit); the full record is `evidence/partitions/surface-roster.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: surface-roster-9, surface-roster-17, surface-roster-18. Related findings in other documents: surface-roster-7 (claim: `FAMILY_SURFACE` and the totality gate), surface-roster-21 (correctness: the doc-hidden contradiction).

#### surface-roster-5: Forty rows spell one exclusion twice; a `Copy` derive and a `law_row` helper state the disposition once per row
- Where: crates/before/src/surface.rs:929-940 (related: crates/before/src/surface.rs:26-27, crates/before/src/surface.rs:72-73, crates/before/src/surface.rs:210-217, crates/before/src/surface.rs:312-317, tools/citecheck:122-145)
- Class / severity / confidence: simplification / low / high
- Provenance: verified; executed: yes (a Python pass over surface.rs parsed 132 explicit `SurfaceRow` literals, 40 of which have `Leg::Law` on prod_tree and identical empty-pin exclusions on both fs legs; `grep -c 'pins: &\[\]'` = 143)
- Seen by: scaffolding; refutation: reframed (the count holds; the proposed acceptance was wrong because citecheck's spelling-totality guard classifies a `Leg::Law(` occurrence only as a quoted literal or a match-arm binding, so a helper body `Leg::Law(law)` would be reported and the call sites would leave its extraction); history: no-rationale-found (ebe66966 introduced helpers for its own shapes and left this one)
- Owner-gated: yes (`Copy` on `Leg` and `Exclusion` widens the meter-public API; the refactor needs a paired, deliberate extension of `tools/citecheck`'s `extract_legs`)

Forty rows (the `Span`/`OwnSpan` accessors and algebra, `Version::ranked`, the `Ranked` rows, most `FAMILY_SURFACE` span rows) share the shape "Law on prod_tree, the same empty-payload exclusion on prod_fs and tree_fs", each spelling `Leg::Excluded(Exclusion::X { pins: &[] })` twice. `Leg` and `Exclusion` hold only `&'static` data, so `Copy` is derivable, after which a `const fn law_row(op, law, fs: Exclusion)` states the fs disposition once, exactly as `codec_row`, `causally_row`, `span_row`, and `HANDBACK` already do for their shapes. Legibility: a reviewer scanning forty six-line blocks to confirm two lines are identical is doing work the type could do.

Evidence:

       929	    SurfaceRow {
       930	        op: "OwnSpan::lo",
       931	        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
       932	        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
       933	        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
       934	    },
       935	    SurfaceRow {
       936	        op: "OwnSpan::hi",
       937	        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
       938	        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
       939	        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
       940	    },

Resolution: `#[derive(Debug, Clone, Copy)]` on `Leg` and `Exclusion`; `const fn law_row(op: &'static str, law: &'static str, fs: Exclusion) -> SurfaceRow`; named consts for the recurring empty exclusions (`DEFINITIONAL`, `LINEARITY`, `NO_WIRE_FORMAT`); rewrite the forty rows as one-liners. In the same change, extend `tools/citecheck`'s `extract_legs` to classify the helper's `Leg::Law(law)` body as a binding and to extract the law literal from `law_row("op", "law", ..)` call sites, with a `--self-test` fixture for each. Acceptance: every surface_coverage test and `just citecheck` green, with citecheck's leg count unchanged (40 law citations still extracted); the file shrinks by roughly 120 lines.

#### surface-roster-10: Three `#[test]`-attribute scanners with two rule sets, in a workspace that created `surface-scan` to have one
- Where: crates/before/src/testing/surface_coverage.rs:310-365 (related: crates/before/src/testing/surface_coverage.rs:159-162, crates/surface-scan/src/lib.rs:174-196, crates/before/tests/amp_board_smoke.rs:314-340, crates/suanpan/src/claims/tests.rs:283, tools/citecheck:332-350)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (read all three scanners; before imports only `fn_name` and `SourceSpec` from surface-scan (surface_coverage.rs:162) and suanpan is `test_fns`'s only consumer; an awk over every crates/**/*.rs found no `#[test]` line followed by a comment line, so the divergence is latent); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed (the comment at 159-162 is imprecise about what is shared rather than asserting the witness scan is); history: no-rationale-found (the scanners were born separately on 2026-07-28/29 and the extraction commit's claim that surface_coverage.rs became a thin wrapper holds only for `extract_public_fns`)
- Owner-gated: no (consolidation; dropping the in-tree witness scan in favor of citecheck alone would be the owner's call)

`declared_test_names_by_file` (here), `surface_scan::test_fns` (lib.rs:174-196), and `band_test_names` (amp_board_smoke.rs:314-340) each re-implement "the names of `#[test]`-attributed fns" with divergent rules: this one matches `#[test]` by prefix and tolerates `///` and `//` lines between the attribute and the `fn`; the other two require `t == "#[test]"` and disarm on any non-attribute line. Every fix to the predicate (surface-roster-11's `#[ignore]` handling included) must land twice or thrice, and the comment at 159-162 says the extractor "and its line discipline" are the shared scanners while the witness scan stays local. Principle 3: duplicated capability generating its own maintenance cascade.

Evidence:

       331	                    let trimmed = line.trim_start();
       332	                    if trimmed.starts_with("#[test]") {
       333	                        test_pending = true;
       334	                        continue;
       335	                    }
       336	                    // Other attributes, doc comments, and comments sit
       337	                    // between `#[test]` and its `fn` without detaching it.
       338	                    if trimmed.starts_with("#[")
       339	                        || trimmed.starts_with("///")
       340	                        || trimmed.starts_with("//")
       341	                        || trimmed.is_empty()

    surface-scan/src/lib.rs:
       179	        if t == "#[test]" {
    ...
       183	        if t.starts_with("#[") || t.is_empty() {

Resolution: make `surface_scan::test_fns` the one scanner, adopting this copy's richer rule (comments between attribute and `fn` keep the arming; `fn ` after a qualifier is found via `fn_name` with a word-boundary check) and adding a per-file entry point; rewrite `declared_test_names_by_file` as a directory walk calling it per file and `band_test_names` as `test_fns(source).into_iter().filter(..)`; move the before-side fixture behaviors (doc comment between; `proptest!` property) into surface-scan's tests; reword 159-162 to what is shared. Acceptance: `grep -rn 'starts_with("#\[test\]")\|== "#\[test\]"' crates tools` matches only crates/surface-scan/src/lib.rs; before's coverage tests, suanpan's claims tests, and amp_board_smoke's parity test pass unchanged.

Synthesis note: Same scanner family as tests-other-3 and suite-economics-3 (this document); this entry names the rule the shared scanner should adopt.

#### surface-roster-24: Census rows key trait impls by private definition paths and generic parameter names, record cross-type impls under both owners, and pin the compiler-internal `StructuralPartialEq`
- Where: crates/before/surfacecheck/src/extract.rs:293-306 (related: crates/before/surfacecheck/src/extract.rs:38-43, crates/before/surfacecheck/src/extract.rs:279-281, crates/before/surfacecheck/src/census.rs:4-6, crates/before/surfacecheck/src/census.rs:37, crates/before/surfacecheck/src/census.rs:39, crates/before/surfacecheck/src/census.rs:43-45, crates/before/surfacecheck/src/census.rs:64, crates/before/surfacecheck/src/census.rs:164, crates/before/surfacecheck/src/census.rs:296, crates/before/surfacecheck/src/census.rs:332, crates/before/src/causally.rs:146, crates/before/src/causally.rs:156)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified; executed: yes (a Python pass over census.rs: 437 impl rows, 373 distinct `impl ... for ...` strings, 64 strings under two owners occupying 128 rows, 17 `StructuralPartialEq` rows; e.g. `impl core::ops::bit::BitOr<Clock> for Version` at rows 39 and 296, `impl core::convert::From<OwnSpan> for Span` at 64 and 164; `mod polarity;` is private at causally.rs:146 and re-exported at 156, yet six rows spell `before::causally::polarity::Polarity`)
- Seen by: scaffolding, structure-prose (plus the refutation pass's observation on extract.rs:38-43); refutation: confirmed (with the nuance that `StructuralPartialEq` distinguishes a derived from a hand-written `PartialEq`, which is user-observable in const patterns); history: deliberate-and-holds for one-row-per-spelling and definition-path rendering (both stated in the code and d60b7570's message); the private-path cost is weighed nowhere and `StructuralPartialEq` was never named
- Owner-gated: no (internal to surfacecheck; one re-pin of `TRAIT_IMPLS`, attributed in its commit)

`render_trait` spells the trait by its definition path from rustdoc's `paths` table, which embeds private module structure: before's own `causally::polarity` (public path `causally::Polarity`), libcore's internal layout (`core::ops::bit::`, `core::iter::traits::accum::`, `core::str::traits::`), and serde's `serde_core::` split. census.rs:4-6 promises pins move only when an impl is added, removed, or reshaped, but renaming `src/causally/polarity.rs`'s module or a dependency reorganizing internally orphans rows with zero API change, in one mechanical diff where an impl change is hardest to see. Rows also carry generic parameter names (`Add<T>`, `Cell<N>`, `Query<P>`), so a parameter rename is a pin event. Separately, 64 impls are recorded under both participating types' pages; extract.rs:38-43 defends this as "two public spellings are two rows of surface", which describes a type reachable at two paths and does not cover the mechanism that produces these rows (rustdoc lists a cross-type impl on every type that appears in it). And 17 rows pin `core::marker::StructuralPartialEq`, a perma-unstable marker that tracks `derive(PartialEq)`; it carries a small signal (derived vs hand-written) at the cost of orphaning 17 rows together if a toolchain stops emitting it. Principle 3: the tamper-evidence value of a pin list degrades in proportion to how often it moves for non-API reasons.

Evidence:

       293	/// Render a trait reference for a census row: the definition path from
       294	/// the `paths` table (unambiguous across same-named traits), plus any
       295	/// generic arguments.
       296	fn render_trait(krate: &Crate, trait_: &Path) -> String {
       297	    let mut out = krate
       298	        .paths
       299	        .get(&trait_.id)
       300	        .map(|summary| summary.path.join("::"))
       301	        .unwrap_or_else(|| trait_.path.clone());

    census.rs:
       332	    "causally::Down: impl before::causally::polarity::Polarity for Down",

Resolution: render the trait as `<crate>::<TraitName><Args>` (first and last `paths` segments), which still disambiguates same-named traits across crates; render generic parameters positionally (`_`); dedupe in `record_impl` by impl `Id` so a cross-type impl is one row (disambiguating same-named for-types only on collision), and rewrite extract.rs:38-43 to name the actual mechanism if double rows are kept; decide `StructuralPartialEq` explicitly (skip beside `is_synthetic` with a one-line reason, or keep with the derived-vs-manual rationale stated). One re-pin of `TRAIT_IMPLS`, attributed to the renderer change. Acceptance: a unit test renders a synthetic `Path` whose `paths` entry is `before::causally::polarity::Polarity` as `before::Polarity`; after re-pinning, the gate is green and renaming the private `polarity` module moves no pin.

#### surface-roster-31: citecheck re-parses the typed roster lexically with its own extraction guards, and its rejected-alternative note omits the external-binary route surfacecheck already demonstrates
- Where: tools/citecheck:59-68 (related: tools/citecheck:122-289, tools/citecheck:438-807, crates/before/surfacecheck/Cargo.toml:21, crates/before/src/lib.rs:441-454, crates/before/src/testing/surface_coverage.rs:116)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (read the header and the extraction functions at 122-289; `wc -l` = 856; surfacecheck links `before` with `meter` at Cargo.toml:21; `TRIPWIRES` is `pub(crate)` under `#[cfg(test)] mod testing` (lib.rs:453-454) and `laws` is behind its feature (lib.rs:444-445)); executed: no
- Seen by: structure-prose; refutation: reframed (viable, but a design change: `TRIPWIRES` and `DIFF_BESPOKE` must leave the cfg(test) tree first); history: deliberate-and-holds for the tools/ route (the rejected alternative is recorded and its reason still holds); the third option was not weighed although 9aa8c9ac had established the detached-binary idiom two weeks earlier
- Owner-gated: yes (moving test-tree tables under `meter`; retiring an instrument)

citecheck extracts `Leg::Bound/Law/Trans` citations, exclusion payloads, `TRIPWIRES`, `DIFF_BESPOKE`, and law and descriptor names from Rust source with Python regexes, then defends that lexical scan with extraction floors, a spelling-totality guard, a shadow guard, a fabricated-citation tripwire, and a long `--self-test`. Its rejected-alternative note considers only an in-crate `#[cfg(test)]` pin; a Rust binary in a detached workspace that links `before` with `meter`, iterates the typed roster, and reads the nextest inventory file as an input artifact keeps the layering the note wants while dissolving the regex extractors and the guards that exist only because the roster is read as text (Principle 3). The retirement bar applies: the replacement must reproduce citecheck's self-test red paths before citecheck is deleted.

Evidence:

        59	Rejected alternative, for the record: an in-crate `#[cfg(test)]` pin
        60	iterating the live `SURFACE` arrays against an inventory of collected test
        61	names. The tables are data and the crate already resolves them against its
        62	registries, but only the runner knows what it collects, and shelling out to
        63	`cargo nextest list` from inside a test binary that same runner is
        64	executing is not house style and inverts the layering; the crate cannot
        65	attest its own collection from within. The `tools/` linter route keeps the
        66	runner's inventory an input artifact, at the cost of reading the law and
        67	descriptor tables lexically — a cost the floors, the spelling totality,
        68	and the shadow guard price in.

Resolution: a second surfacecheck subcommand (or sibling binary) that iterates `METHOD_SURFACE`/`FAMILY_SURFACE` and `laws::registered_names`, reads the `cargo nextest list --message-format json` artifact the recipe hands it, and keeps the fabricated-citation tripwire and the shadow guard; move `TRIPWIRES` and `DIFF_BESPOKE` (or their name lists) under `meter`; reproduce citecheck's self-test red paths as unit tests; then retire `tools/citecheck`. If the owner keeps the tools/ route, add the omitted option to the note with the reason it was not taken. Acceptance: the justfile's `citecheck` recipe invokes the Rust checker and `tools/citecheck` is deleted; or the note records the third option.

Synthesis note: surface-roster-9 (this document) is the in-tree twin of the same move (a typed Rust checker replacing a lexical scan); the fabricated-citation tripwire and shadow guard must survive the move.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| surface-roster-8 | `crates/before/src/surface.rs:1128-1129` | Long `op` literals keep rustfmt from formatting `FAMILY_SURFACE`, leaving hand-formatted one-line rows beside vertical ones | Keep `op` to the identity; move the parentheticals to comments |
| surface-roster-19 | `crates/before/surfacecheck/src/check.rs:259-261` | Em-dashes in `//` comments at six sites | Colons or semicolons in four files |
| surface-roster-26 | `crates/before/surfacecheck/src/main.rs:44-46` | Idiom nits: a mutation inside `Option::inspect`, leading `::` on an external crate path, a fully qualified `Range`, `push_str(&format!(..))`, an order-sensitive roster comparison, and an unnamed `20` at four sites | `if let` for `--list`; imports; `writeln!`; a named `MIN_REASON_LEN` |

### The test harness: bridge, oracles, exhaustive, validation index

14 entries (1 medium, 9 low, 4 nit); the full record is `evidence/partitions/testing-oracles.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: testing-oracles-17, testing-oracles-25. Related findings in other documents: testing-oracles-3 is in this document (idiom); the summary's grid and bridge items are testing-oracles-16 and -19 (documentation).

#### testing-oracles-3: Bridge id walks recurse bare while the version walks route through `descend!`; the literal depth `0` probes on every level
- Where: crates/before/src/testing/bridge.rs:29-45 (related: crates/before/src/testing/bridge.rs:56-57, crates/before/src/testing/bridge.rs:109-132, crates/before/src/testing/bridge.rs:150-151, crates/before/src/recurse.rs:3-4, crates/before/src/recurse.rs:9-14, crates/before/src/recurse.rs:22-28, crates/before/src/recurse.rs:88-93, crates/before/src/recurse.rs:111-117, crates/before/AGENTS.md:32-36, crates/before/src/party/tests.rs:830-848)
- Class / severity / confidence: idiom / medium / high
- Provenance: verified (`grep -rn 'descend!(' crates/before/src` shows the bridge's only sites are 56, 57, 150, 151, all with literal `0`, while grow/tests.rs:172-180 and meter/tests.rs:417 thread `depth + 1`; recurse.rs:91-93 is `depth.is_multiple_of(STRIDE)`; `git show faf3cd0a -- bridge.rs` adds `descend!` around `emit_ev`/`read_ev` only; `git log -S'descend!(0, read_id'` and `-S'descend!(0, emit_id'` are empty; party/tests.rs:834 `ORACLE_SCALE_MAX = 4096` with `to_oracle_party(&Party::from_bits(...))` at 847-848 drives the unguarded `read_id` to 4096 levels); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: contradicts-hard-rule (the bridge was never guarded before faf3cd0a, which guarded only the two walks it was rewriting; `descend!(0, …)` was the entry-call idiom of 1c4c9a2d applied to recursive calls)
- Owner-gated: no

`emit_id` and `read_id` recurse on tree depth with plain calls; `emit_ev` and `read_ev` route through `descend!(0, …)`. Before's `AGENTS.md` hard-rule paragraph and `recurse.rs` both name "the oracle bridge" as the place recursion routes through the guard, so the inventory is false for half the bridge, and the guarded half is the shallower side in practice (the id walks see 4096-level spines in party/tests.rs; the version walks see at most 128 in the callers traced). Separately, `should_grow(0)` is `0.is_multiple_of(STRIDE)`, which is true, so the guarded walks probe stack headroom at every level, forfeiting the once-per-`STRIDE` amortization recurse.rs:22-28 documents and contradicting the macro's prescribed usage `descend!(depth + 1, …)`. Correctness is not exposed today (the oracle's own recursive `Drop` binds first per `oracle.rs:16-21`, and 4096 frames fit the test stack), so this is the letter of the hard rule breached on a test surface plus two inaccurate inventory statements, not a reachable overflow; the fix is trivial either way.

Evidence:

        36	        oracle::Party::Node(l, r) => {
        37	            // 2-bit presence tag, then the present children (a `0` child emits
        38	            // nothing).
        39	            out.push(!id_is_zero(l)); // bit 0 = left present
        40	            out.push(!id_is_zero(r)); // bit 1 = right present
        41	            emit_id(out, l);
        42	            emit_id(out, r);
        43	        }
        56	            descend!(0, emit_ev(out, l));
        57	            descend!(0, emit_ev(out, r));

    recurse.rs:
        11	//! surface, where the remaining depth recursion lives: the differential oracle
        12	//! bridge (`testing::bridge`), whose walks mirror the paper's recursive trees,
        91	pub(crate) fn should_grow(depth: usize) -> bool {
        92	    depth.is_multiple_of(STRIDE)
        93	}
       116	/// each recursive call site: `descend!(depth + 1, self.rec(child_args, depth +
       117	/// 1))`.

    before/AGENTS.md:
        32	  `version/skyline/fill.rs`). A walk that must recurse routes each recursive
        33	  call through `crate::recurse::descend!`, which grows the stack onto the
        34	  heap before a deep input can overflow — today those are only test
        35	  surfaces: the oracle bridge and the test-local recursive witnesses beside
        36	  it (`recurse.rs`'s module doc holds the inventory and the keep decision).

Resolution: Thread a `depth: usize` through all four walks and route every recursive call through `descend!(depth + 1, …)` as grow/tests.rs and meter/tests.rs do; if the id side is instead meant to be exempt, state its bound at `emit_id`/`read_id` (the oracle envelope plus `ORACLE_SCALE_MAX`) and correct recurse.rs:11-14 and AGENTS.md:32-36 to say which bridge walks are guarded. Acceptance: `grep -n 'descend!(0' crates/before/src/testing/bridge.rs` is empty and every recursive call in bridge.rs is inside `descend!(depth + 1, …)`; or the exemption is stated at the site and recurse.rs and AGENTS.md describe the bridge accurately.

#### testing-oracles-2: The validation index is never rendered, so its intra-doc links are unchecked and its `pub` is dead visibility
- Where: crates/before/src/testing.rs:50-50 (related: crates/before/src/lib.rs:453-454, justfile:256-270, crates/before/src/testing/validation_index.rs:1-13)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (lib.rs:453-454 reads `#[cfg(test)] mod testing;`; justfile:258 and :270 run `cargo doc ... --all-features --no-deps` and `--document-private-items` with no `--cfg test`; `grep -n 'cfg test' justfile` is empty); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found (from birth at 669cf310)
- Owner-gated: yes (the page's intended role: rendered map or source-read page)

`validation_index` is `pub mod` inside a module lib.rs gates with `#[cfg(test)]`, and no doc recipe passes `--cfg test`, so rustdoc never compiles the module and never resolves its `[`super::exhaustive`]`-style links. The page's link syntax implies a check that does not run, and the `pub` reaches no further than `pub(crate)`. Principle 3: link syntax whose only reader is a human opening the source file costs maintenance and buys no checking.

Evidence:

        50	pub mod validation_index;

    lib.rs:
       453	#[cfg(test)]
       454	mod testing;

Resolution: Owner's call between (a) rendering it: move the index out of `cfg(test)` (it holds no code) behind `#[cfg(doc)]` or a feature so `just docs-internal` checks its links, with the links pointed at renderable targets; or (b) keeping it source-read: change `pub mod` to `mod`, replace intra-doc link syntax with plain code spans, and say in the opening paragraph that it is read in source. Acceptance: either `just docs-internal` fails on a dead link in the index, or the file contains no intra-doc link syntax and its visibility matches its siblings.

#### testing-oracles-9: `Dyadic`'s hand-written `PartialEq`/`Eq`/`PartialOrd`/`Ord` are dead code, and the `Ord` doc's overflow premise names the wrong bound
- Where: crates/before/src/testing/semantic_oracle.rs:132-152 (related: crates/before/src/testing/semantic_oracle.rs:86-104, crates/before/src/testing/semantic_oracle.rs:123-129, crates/before/src/testing/semantic_oracle.rs:519-541)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn Dyadic crates/before/src crates/before/tests` outside the impl block shows only construction via `Dyadic::grid`/`Dyadic::center`, struct literals in `descend`, field reads in `cell_at`/`descend`, and `Fn(Dyadic)` parameter types; no `==`, `cmp`, `min`/`max`, sort, or ordered container over `Dyadic` anywhere); executed: no
- Seen by: adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (dead at birth in 08f7ccab; the `GRID_N` premise was true while every grid was capped at 10, and f55c8276's uncapped `fs_grid` moved the bound to 64 without revisiting the doc)
- Owner-gated: no

Nothing compares two `Dyadic` values, so the four trait impls exist only to be correct about themselves (Principle 3), and trait impls are never dead-code-warned. The `Ord` doc argues `u128` cannot overflow because exponents stay near `GRID_N`; the module's `fs_grid` deliberately does not cap at `GRID_N` and instead asserts `g < 64`, which with `center` adding one level is the operative bound (`num < 2^64` shifted by at most 64 fits `u128`). The conclusion holds; the stated reason sends an auditor to the wrong constant.

Evidence:

       143	impl Ord for Dyadic {
       144	    /// Compare `a/2^p` and `b/2^q` by cross-multiplication: `a·2^q` vs `b·2^p`
       145	    /// (exponents stay within a level of [`GRID_N`], so `u128` never
       146	    /// overflows).
       147	    fn cmp(&self, other: &Self) -> Ordering {

    fs_grid:
        86	/// Deliberately *not* capped at [`GRID_N`]: that ceiling is derived from
        98	    debug_assert!(
        99	        g < 64,

Resolution: Delete the four impls, keeping `#[derive(Clone, Copy, Debug)]`. If an ordering is wanted later, state the bound as `fs_grid`'s `g < 64` (exponent at most 64 after `center`). Acceptance: `grep -n 'impl .* for Dyadic' crates/before/src/testing/semantic_oracle.rs` is empty and the test target compiles.

#### testing-oracles-10: Test-side grid caps at `GRID_N` cannot bind, contradict `fs_grid`'s stated policy, and are guarded by constant-only asserts
- Where: crates/before/src/testing/semantic_oracle/tests.rs:40-47 (related: crates/before/src/testing/semantic_oracle/tests.rs:185-194, crates/before/src/testing/semantic_oracle/tests.rs:288, crates/before/src/testing/semantic_oracle/tests.rs:603-609, crates/before/src/testing/semantic_oracle/tests.rs:614, crates/before/src/testing/semantic_oracle.rs:62-64, crates/before/src/testing/semantic_oracle.rs:80, crates/before/src/testing/semantic_oracle.rs:86-104, crates/before/src/testing/semantic_oracle.rs:375-384)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (generators.rs:298 `ARB_DEPTH: u32 = 4` bounds `arb_oracle_*` via `prop_recursive` at 341 and 363, so every `grid_for` input is at most 5, or 7 at the `+ 3` site, against `GRID_N = 32` from optrace.rs:56 `MAX_TRACE_OPS = 30` and semantic_oracle.rs:80; `fork` asserts `res < GRID_N` before `res + 1`, so every reachable id ceiling is at most `GRID_N`, `event`'s ceiling is `e.res_ceiling.max(id_res(i))`, and the keystone's `d.min(GRID_N)` is the identity; `grid_for`'s six call sites are 230, 244, 265, 303, 422, 511); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness (the refutation added the `k < GRID_N` assert at 614 and the `GRID_N` doc sentence); refutation: confirmed; history: deliberate-but-expired (the caps matched the clamping `fork` of 08f7ccab/7487be16; 28f6981e replaced the clamp with an assert and f55c8276 stated the opposite policy for `fs_grid`, neither touching these sites; the 606-609 assert was written at 28f6981e as a guard against redefining `GRID_N` independently of the op cap)
- Owner-gated: no

`grid_for` is `fs_grid` plus `.min(GRID_N)`; the keystone caps again at line 194 under a comment that credits `grid_cap_is_never_reached`, and `fork_partitions` caps at 288. The parent module's `fs_grid` computes the same `max + 1` and refuses to cap for the stated reason that a binding cap "could only alias a scan silently". None of the caps can bind on any reachable population, so their only possible effect is the silent-alias failure the module rejects, and two helpers with opposite policies for one computation is a legibility hazard. The `GRID_N` doc at 62-64 ("this caps `g`") describes the clamp, not `fs_grid`'s assert. Beside them, `fork_chain_raises_resolution_one_level_per_fork` opens with `MAX_TRACE_OPS <= GRID_N` where `GRID_N` is defined as `MAX_TRACE_OPS as u32 + 2`, and asserts `k < GRID_N` with `k` at most 16: both are runtime checks of compile-time facts. The keystone's clamp matters for testing-oracles-17: a silent clamp is exactly what makes the sampled sweep necessary, and turning it into an assert is what would let the sweep retire.

Evidence:

        45	fn grid_for(parts: &[u32]) -> u32 {
        46	    (parts.iter().copied().max().unwrap_or(0) + 1).min(GRID_N)
        47	}
       190	        let g = se
       191	            .iter()
       192	            .map(|c| id_res(&c.id).max(ev_res(&c.ev)))
       193	            .max()
       194	            .map_or(0, |d| d.min(GRID_N));
       606	    assert!(
       607	        u32::try_from(MAX_TRACE_OPS).expect("small cap") <= GRID_N,
       608	        "the grid ceiling no longer clears the deepest in-support bisection"
       609	    );
       614	        assert!(k < GRID_N, "the chain itself must stay inside the grid");

    semantic_oracle.rs:
        80	pub(crate) const GRID_N: u32 = optrace::MAX_TRACE_OPS as u32 + 2;
        86	/// Deliberately *not* capped at [`GRID_N`]: that ceiling is derived from
        90	/// organic drivers), so a binding cap here could only alias a scan
        91	/// silently. What is asserted instead is the one hard bound the

Resolution: Delete `grid_for` and call `fs_grid` at its six sites; replace the keystone's `.map_or(0, |d| d.min(GRID_N))` with `fs_grid` or an `assert!(d <= GRID_N, …)` and reword the comment at 185-189 to say `fork`'s assertion is what keeps the probed grid inside the ceiling; use `fs_grid(&[id_depth(&p) + 1])` at 288 (see testing-oracles-16 for the rate); turn the 606-609 assert into a `const _: () = assert!(...)` (its purpose, guarding a future redefinition of `GRID_N`, survives) and delete 614; restate the `GRID_N` doc at 62-64 in terms of the assert. Acceptance: `grep -n 'min(GRID_N)\|fn grid_for' crates/before/src/testing/semantic_oracle/tests.rs` is empty; the suite is green.

Synthesis note: testing-oracles-17 (this document) is the sweep that stands in for the keystone assert this entry restores.

#### testing-oracles-11: `replay` carries a `seeds` parameter every caller fixes at 1 and re-spells the optrace steppers; `FunctionClock`'s `Err` arms are unreachable
- Where: crates/before/src/testing/semantic_oracle/tests.rs:55-63 (related: crates/before/src/testing/semantic_oracle/tests.rs:68-152, crates/before/src/testing/semantic_oracle/tests.rs:183, crates/before/src/testing/semantic_oracle/tests.rs:565, crates/before/src/testing/optrace.rs:72-120, crates/before/src/testing/optrace.rs:122-165, crates/before/src/testing/semantic_oracle.rs:697-724)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (both callers pass `1` at 183 and 565; the `Join` arm's `i2 = if j < i { i - 1 } else { i }` and the `Sync` arm's `split_at_mut` appear in replay at 108-123 and 139-146 and in optrace.rs at 98-101, 109-110, 145-147, 156-157; with one seed all live ids are disjoint, so the `if d_im` at 117 and 139 always takes the true arm; grep of diff_ops.rs shows `FunctionClock` used only by construction and field reads, so `join`/`sync`'s `Err` arms at 704 and 722 are reached by no test); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed; history: `seeds` deliberate-but-expired (needed at 08f7ccab when the sweep ran `1usize..=4` seeds; single-seed since 7487be16); the lockstep inlining deliberate-and-holds (its purpose, asserting three-way disjointness agreement before every `Join`/`Sync`, is stated at 51-58 and is something neither optrace stepper can do)
- Owner-gated: no

A parameter with one value at every site is generality with no consumer, and the doc at 55 already says so. The impl and oracle arms are a third spelling of the trace semantics that `optrace::run` and `step_impl` each spell once "so traces line up"; a divergence in index arithmetic would misalign populations and read as a false differential failure. The lockstep design itself has a stated reason (the pre-op agreement assert) and stays; what can dissolve is the duplicated dispatch. `FunctionClock::join`/`sync` return `Result` to mirror the crate's API, but with one seed and a pre-check, their failure arms are dead in every harness.

Evidence:

        55	/// Every caller passes `seeds = 1`: the invariance the keystone asserts holds
        56	/// only for a proper single-seed system (see
        57	/// [`replay_matches_across_references`]). A `Join`/`Sync` on overlapping
        58	/// parties is a no-op in all three (disjointness is invariant).
        59	fn replay(
        60	    seeds: usize,
        61	    ops: &[Op],
        62	    rng: &mut StdRng,
        63	) -> (Vec<Clock>, Vec<oracle::Clock>, Vec<FunctionClock>) {
       139	                        if d_im {
       140	                            let i2 = if j < i { i - 1 } else { i };
       141	                            let v = im.remove(j);
       142	                            assert!(im[i2].join(v).is_ok());

    optrace.rs:
       109	                        let victim = cs.remove(j);
       110	                        let i2 = if j < i { i - 1 } else { i };
       122	/// Apply one op to an impl population, mirroring [`run`] for the oracle (same index
       123	/// arithmetic, so traces line up). Used by tests that drive the impl alone.

Resolution: Drop `seeds` and start each population from one seed. Extract the oracle arm of `optrace::run` into a `step_oracle(&mut Vec<oracle::Clock>, &Op)` so `run` folds over it, write a `step_fs(&mut Vec<FunctionClock>, &Op, &mut StdRng)` beside it, and reduce `replay` to the pre-op disjointness-agreement assert followed by three step calls. Make `FunctionClock::join`/`sync` infallible operations that assert disjointness, since no caller wants the `Err`. Acceptance: `replay` contains no `match *op` arm that mutates `im` or `or` directly; the `Join`/`Sync` index arithmetic exists in optrace only (plus the fs stepper); `replay(1, …)` call sites become `replay(…)`; `replay_matches_across_references` and the sweep pass unchanged.

Synthesis note: testing-diff-gen-16 (this document) is the optrace side of the same duplicated applier; land the two together.

#### testing-oracles-20: Hand-rolled injective dedup keys where the oracle types already derive `Hash + Eq`
- Where: crates/before/src/testing/exhaustive.rs:120-125 (related: crates/before/src/testing/exhaustive.rs:156-182, crates/before/src/testing/exhaustive.rs:184-229, crates/before/src/oracle/party.rs:11, crates/before/src/oracle/version.rs:36, crates/before/src/testing/exhaustive/tests.rs:568-627)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (oracle/party.rs:11 and oracle/version.rs:36 both read `#[derive(Clone, PartialEq, Eq, Hash, Debug)]`; `id_key`/`ev_key` are used only at exhaustive.rs 128, 141, 164, 174; `seen` is never iterated, so `BTreeSet`'s ordering buys nothing); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed (severity lowered: test scaffolding with no behavioral consequence, and `corpus_counts_are_exact` already pins the dedup); history: no-rationale-found (the derives already existed at the enumerator's birth in 4eec16bb; the one stated reason, "`Party` has no `Ord`", explains `BTreeSet`'s key requirement, not why a `BTreeSet` was wanted)
- Owner-gated: no

The enumerators dedup through `BTreeSet<Vec<u8>>` keyed by 45 lines of hand-written preorder encodings carrying their own correctness argument ("this never collides", conditioned on bases staying single-byte). Dedup needs `Hash + Eq`, which both oracle types derive; a `HashSet<oracle::Party>`/`HashSet<oracle::Version>` dissolves both encoders and the argument. Prefer the mature capability over hand-rolling.

Evidence:

       120	    // `pool` holds the deduped *canonical* trees of depth `≤ d`, keyed for
       121	    // de-dup by a cheap injective preorder encoding (`Party` has no `Ord`).
       125	    let mut seen: BTreeSet<Vec<u8>> = BTreeSet::new();
       217	                out.push(0xff); // base terminator (bases are small; this never collides)

    oracle/party.rs:
        11	#[derive(Clone, PartialEq, Eq, Hash, Debug)]

Resolution: Replace `seen` with `HashSet<oracle::Party>`/`HashSet<oracle::Version>` (`seen.insert(t.clone())`), delete `id_key` and `ev_key`, and rewrite the comment to state only the every-level dedup rationale. Acceptance: `id_key`/`ev_key` are gone; `corpus_counts_are_exact` still reports `2^(2^d)` ids and 691 events with full denotation distinctness.

#### testing-oracles-23: `corpus_is_canonical`'s `> 20` non-triviality floors are superseded by the exact-count pin that names them inadequate
- Where: crates/before/src/testing/exhaustive/tests.rs:384-395 (related: crates/before/src/testing/exhaustive/tests.rs:550-552, crates/before/src/testing/exhaustive/tests.rs:568-627)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (read both tests; `corpus_counts_are_exact` pins `2^(2^d)` for every `d` in `0..=ID_SMALL_DEPTH` and the 691 literal plus denotation distinctness, and its doc at 550-552 names the floor as admitting shrinks past half the corpus; ebec26fe's message says the replacement was demonstrated red under a constructed shrink); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-but-expired (ebec26fe landed the replacement, met the retire-an-instrument bar, and left the floors in place with no stated reason)
- Owner-gated: no

Retiring an instrument requires the replacement to demonstrate it catches what the instrument caught; the replacement's own doc and commit make that demonstration, and the floors now catch nothing the exact pin misses. The `is_normal` sweep in the same test has no replacement and stays.

Evidence:

       384	    // The corpus is non-trivial (guards against an enumeration that silently produces
       385	    // nothing and makes every cross-product loop vacuous).
       386	    assert!(
       387	        ids.len() > 20,
       388	        "id corpus suspiciously small: {}",
       389	        ids.len()
       390	    );
       551	/// (`corpus_is_canonical`'s non-triviality floor admits shrinks of more
       552	/// than half the corpus). This pin closes that hole two ways:

Resolution: Delete lines 384-395 and the parenthetical at 551-552; keep `corpus_is_canonical` as the normality sweep and reword its doc accordingly. Acceptance: `corpus_is_canonical` asserts only `is_normal`; no `> 20` literal remains; a deliberate enumeration shrink (drop `P::Leaf(true)` from the seed loop) still fails `corpus_counts_are_exact`.

#### testing-oracles-27: The organic population replays the trace twice and takes versions and parties from the oracle through the bridge, although the impl replay already holds them
- Where: crates/before/src/testing/algebraic_laws/tests.rs:411-436 (related: crates/before/src/testing/algebraic_laws/tests.rs:24-35, crates/before/src/testing/algebraic_laws.rs:20-22, crates/before/src/clock.rs:625, crates/before/src/clock.rs:640, crates/before/src/party.rs:534)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (read 411-436: `run(&ops)` for the oracle, `ver`/`party` through the bridge, then `step_impl` for clocks; `Clock::party() -> &Party` at clock.rs:625, `Clock::version() -> &Version` at 640, `Party::dangerously_alias` at party.rs:534 make impl-sourced values feasible with the aliasing the clock list already uses at 435); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no-rationale-found (the shape 86dd53a7 landed with; `dangerously_alias` predates it by seven weeks)
- Owner-gated: no

The suite's claim is that the laws hold on "the value shapes real fork/tick/join/sync schedules produce"; sourcing versions and parties from the impl's own replay makes that literally about the impl's values, removes a bridge dependency from a suite whose module doc says the oracle is "only a source of bits", and halves the replay work. `Version` is `Clone` and parties alias through the documented escape hatch.

Evidence:

       411	        let cs = run(&ops);
       412	        let n = cs.len();
       413	        let picks = [i % n, j % n, k % n];
       414	        let (pa, va) = cs[picks[0]].trees();
       417	        let (ia, ib, ic) = (ver(va), ver(vb), ver(vc));
       418	        let (qa, qb, qc) = (party(pa), party(pb), party(pc));
       426	        // Clocks: replay the trace on the impl for real, reachable clocks.
       427	        let mut imp = vec![Clock::seed()];
       428	        for op in &ops {
       429	            step_impl(&mut imp, op);
       430	        }

Resolution: Replay once on the impl; derive `v` from `imp[..].version().clone()`, `p` from `imp[..].party().dangerously_alias()`, and the lists likewise; drop `run(&ops)` and the `ver`/`party` calls in this test. Acceptance: the organic test imports neither `run` nor the bridge; every law group still drives; the committed seeds in proptest-regressions/testing/algebraic_laws/tests.txt replay green.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| testing-oracles-5 | `crates/before/src/testing/bridge.rs:83-89` | Idiom residue across the partition: qualified paths beside imports, a re-spelled helper, an index before its `expect`, a `debug_assert!` in test-only code, a `Result<(), ()>` | The listed imports and spellings; `assert!` in `fs_grid` |
| testing-oracles-7 | `crates/before/src/testing/bridge.rs:102-103` | Em-dashes in `//` comments (a crate-wide pattern; four sites in this partition) | Crate-wide sweep |
| testing-oracles-13 | `crates/before/src/testing/semantic_oracle/tests.rs:218-220` | An empty "operation cross-checks" section banner survived the move of its tests to `diff_ops` | Delete the banner |
| testing-oracles-15 | `crates/before/src/testing/semantic_oracle/tests.rs:484-504` | The paper's worked-value fixture and its expected vector are spelled twice | One `paper_worked_example()` fixture |

### The test harness: differential table, generators, snapshots, asymptotics

14 entries (7 low, 7 nit); the full record is `evidence/partitions/testing-diff-gen.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: testing-diff-gen-31. Related findings in other documents: testing-diff-gen-17 (documentation: compactness.rs's pre-flag-day prose), testing-diff-gen-22 and -23 (claim and verification-gap: the asymptotics pins).

#### testing-diff-gen-2: Row tuples cross the shape_rows/diff_ops boundary unnamed and in inconsistent field order
- Where: crates/before/src/testing/diff_ops.rs:237-248 (related: crates/before/src/testing/diff_ops.rs:134-159, crates/before/src/testing/diff_ops.rs:519-533, crates/before/src/testing/shape_rows.rs:56-116)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (read the four fold signatures at shape_rows.rs:58, 73, 84-86, 103-105 and the two reordering impls); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found (the row vocabulary landed in one commit, 46eb64f9, with no discussion of its types)
- Owner-gated: no

Four row vocabularies are bare tuples with different field orders: `(Base, u64)` is height-first, `(bool, u64)` is owned-first, `(u64, Vec<Base>)` and `(u64, Base, bool)` are depth-first. The overlay `FsMatches` impl reorders fields to reuse the plateau and region comparisons, and the `clock_shape` tree spelling (521-531) is a ten-line inline transform in a table whose doc says a descriptor "states only the spellings" (285-286). Types-first: newtypes where the name carries semantic weight.

Evidence:

       237	impl FsMatches<Vec<(u64, Base, bool)>> for semantic_oracle::FunctionClock {
       238	    fn fs_matches(&self, reference: &Vec<(u64, Base, bool)>, grid: u32) -> bool {
       239	        let heights: Vec<(Base, u64)> = reference
       240	            .iter()
       241	            .map(|(depth, height, _)| (height.clone(), *depth))
       242	            .collect();
       243	        let owned: Vec<(bool, u64)> = reference
       244	            .iter()
       245	            .map(|(depth, _, owned)| (*owned, *depth))
       246	            .collect();

Resolution: In `shape_rows.rs` define `PlateauRows`, `RegionRows`, `CellRows`, `OverlayRows` as newtypes over the vectors with one field order and `heights()`/`owned()` projections on `OverlayRows`; add `oracle_overlay(&oracle::Clock) -> OverlayRows` so the `clock_shape` tree spelling becomes one call. Acceptance: no bare row tuple type in `diff_ops.rs`; each of `clock_shape_matches_the_oracle`'s three spellings is one expression.

#### testing-diff-gen-6: The registration totality pin describes the lint's failure class, not its own; its source scan imposes a layout convention on the macro and duplicates `laws/tests.rs`
- Where: crates/before/src/testing/diff_ops/tests.rs:138-179 (related: crates/before/src/testing/diff_ops.rs:297-305, crates/before/src/testing/diff_ops.rs:960-964, crates/before/src/laws/tests.rs:28-61, crates/before/src/lib.rs:453-454, justfile:127)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (both scan functions read; they differ only in prefix string and path; lib.rs:453-454 `#[cfg(test)] mod testing;` is private; justfile:127 runs `cargo clippy --workspace --all-targets --all-features -- -D warnings`); the lint's behavior on an unrostered `pub(crate) static` is assessed, not executed; executed: no
- Seen by: scaffolding, structure-prose; refutation: reframed (the pin's stated purpose is already a gate lint error by reading; what the pin alone holds is roster totality against an ad-hoc-driven group, which its doc does not say); history: no-rationale-found (the scan idiom was copied from laws/tests.rs, whose rationale concerns exported `pub static` groups the dead-code lint cannot see)
- Owner-gated: no (retiring the pin would be; the recommended step keeps it)

The pin's doc says it closes "a group static missing from the roster, which nothing would run". In a private `cfg(test)` module an unreferenced `pub(crate) static` is dead code, and the gate's clippy leg runs with `-D warnings`, so that failure class is already an error by reading. What the pin alone holds is stricter: a group that some bespoke test drives directly without rostering (referenced, so not dead) still fails the set equality. That unique catch is not in the doc. To stay blind to the macro's own definition the scan also obliges the `diff_ops!` matcher and transcriber to keep attributes and declaration on one line (diff_ops.rs:302-305), a stabilization convention that exists only to serve the instrument, and the scan body is `laws/tests.rs:36-61` with a different prefix and path.

Evidence:

       143	/// group is executed by construction and needs no per-consumer pin. The one
       144	/// door that leaves open is a group static missing from the roster, which
       145	/// nothing would run; this pin closes it against a source scan of the
       ...
       152	    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/testing/diff_ops.rs");
       153	    let text =
       154	        fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));

    diff_ops.rs:
       302	    // In both the matcher and the transcriber, the attributes and the
       303	    // declaration share a line: the registration totality pin's source scan
       304	    // reads any line starting `pub(crate) static` as a group declaration,
       305	    // and must see the invocations' headers only, never this definition.

Resolution: Keep the pin and restate its doc to name what it alone catches (a declared group driven by a bespoke test but absent from the roster; the unreferenced case is the gate's `dead_code` error). Extract one shared `testing` helper `declared_statics(path, prefix) -> BTreeSet<String>` used by both this pin and `every_law_group_is_registered`. If the owner prefers retirement, first execute the construction below so the replacement demonstrably catches what the pin caught. Acceptance: one scan body in the crate; the pin's doc names the ad-hoc-driven case; the layout comment at diff_ops.rs:302-305 either stays with the shared helper cited or goes with the scan.

Construction: In diff_ops.rs add, without touching `for_each_diff_group!`, `diff_ops! { pub(crate) static ORPHAN: (a: version); fn orphan_is_empty { prod: a.is_empty(), tree: a.is_empty() } }`; run `cargo clippy -p before --all-targets --all-features -- -D warnings`. Expected: `error: static ORPHAN is never used`. Then reference `ORPHAN` from a bespoke test body: the lint is silent and only the pin's set equality reads red. That second step is the pin's unique catch.

Synthesis note: oracle-laws-25 (this document) proposes a compile-time tie that would retire both scans; this entry's shared-helper step is the fallback if the tie is not taken.

#### testing-diff-gen-7: The organic driver draws a third version (`k`, `vc`) that no drive arm reads
- Where: crates/before/src/testing/diff_ops/tests.rs:371-382 (related: crates/before/src/testing/diff_ops/tests.rs:445-466)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -n 'v\[2\]' diff_ops/tests.rs` returns nothing; every `organic_drive!` arm at 391-426 reads only `$env.v[0]` and `$env.v[1]`; `k` at 449, `vc` at 456, stored at 461 and 464); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed; history: no-rationale-found (d1c27ce7 landed the three-slot array and the `k` pick with no `v[2]` read; no later commit added one)
- Owner-gated: no

`Organic.v` is documented as "Three versions from the trace" and the proptest draws `k in 0usize..64` to pick `vc`, but every arm indexes `v[0]` and `v[1]` only. The third pick widens the proptest case and shrink space for no coverage, and the struct doc claims a population the drivers do not exercise.

Evidence:

       372	    /// Three versions from the trace, causally related.
       373	    v: [&'a oracle::Version; 3],
       ...
       449	        k in 0usize..64,
       ...
       456	        let (_, vc) = cs[k % len].trees();

Resolution: Drop `k` and `vc`, make `v: [&'a oracle::Version; 2]`, and fix the field doc; or, if a three-version group was intended (a transitivity or `span_all` descriptor), add it so the pick is read. Acceptance: every field of `Organic` is read by at least one `organic_drive!` arm.

#### testing-diff-gen-11: `shape_version` and `shape_version_wide` duplicate the spine loop; three `unreachable!("handled above")` arms; `debug_assert!` in test-only code
- Where: crates/before/src/testing/generators.rs:127-204 (related: crates/before/src/testing/generators.rs:72-94, crates/before/src/testing/generators.rs:244-261)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (the lean match at 135-141 and 195-201 is identical; `unreachable!("handled above")` at 140, 200, 257; `debug_assert!` at 166; traced `scale == 0, at_tip == false`: `mid = 0`, the loop never runs, `leaf(0)` returns without the wide leaf); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The two version builders run the same match over `k in 1..=scale` and differ only in how a leaf's base is chosen; `bushy_version_with` (79-94) already models the fix with a `leaf_base` closure. The early-return-for-Bushy structure leaves an `unreachable!` arm in three matches. The precondition guard is a `debug_assert!` in code that compiles only under `cfg(test)`; a plain `assert!` costs the same and holds under a release-profile test run too.

Evidence:

       135	        t = match shape {
       136	            Shape::LeftSpine => V::node(0u64, t, leaf),
       137	            Shape::RightSpine => V::node(0u64, leaf, t),
       138	            Shape::Zigzag if k % 2 == 0 => V::node(0u64, t, leaf),
       139	            Shape::Zigzag => V::node(0u64, leaf, t),
       140	            Shape::Bushy => unreachable!("handled above"),
       141	        };
       ...
       166	    debug_assert!(scale >= 1, "a scale-0 shape has nowhere to put the leaf");

Resolution: `fn shape_version_with(shape, scale, leaf_base: &impl Fn(u64) -> Base)` with `shape_version` as the identity instance and `shape_version_wide` choosing the wide index up front; select the lean with one closure returned from a single match on `shape` so Bushy is handled once (the party twin at 244-261 can share it). Replace `debug_assert!` with `assert!`. Acceptance: one spine loop for versions; no `unreachable!("handled above")` remains; `scale == 0` panics for the wide builder.

#### testing-diff-gen-16: The op applier is spelled twice in `optrace.rs` and twice more elsewhere; the module doc's op list omits `Ticks` at three sites
- Where: crates/before/src/testing/optrace.rs:71-165 (related: crates/before/src/testing/optrace.rs:1-2, crates/before/src/clock/tests.rs:208-265, crates/before/src/testing/semantic_oracle/tests.rs:59-153, crates/before/src/version/tests.rs:663-672, crates/before/src/testing/compactness/tests.rs:38, crates/before/src/testing/diff_ops/tests.rs:435)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (both bodies read side by side; `grep -rn 'Op::Tick('` lists the two external copies and the tick-count mirror; optrace.rs:2 lists "fork/tick/send/sync/join" and `Ticks` exists at 26); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed, severity lowered (the `replay` copy threads a third carrier with an rng and interposes disjointness asserts, so it has a stated reason to differ); history: no-rationale-found (8f253e4e added `Ticks` to the enum and every applier without editing the list)
- Owner-gated: no

`run` and `step_impl` are the same control flow: index reduction, `Fork` push, `Send` as send-then-receive, `Sync` via `split_at_mut(hi)`, `Join` as remove-and-reindex with the `i2` adjustment. The doc asks the reader to keep them in lockstep by hand. `clock/tests.rs:208-265` (`master_differential`) is a third hand copy. The module doc's op inventory is a hand-maintained list that has already drifted; the refutation pass found the same rotted list at compactness/tests.rs:38 ("fork/tick/send/sync/join history") and diff_ops/tests.rs:435 ("fork/tick/join/sync schedules").

Evidence:

         1	//! The seed-derived op-trace generator: a proptest strategy that produces a sequence of
         2	//! fork/tick/send/sync/join steps.
       ...
       122	/// Apply one op to an impl population, mirroring [`run`] for the oracle (same index
       123	/// arithmetic, so traces line up). Used by tests that drive the impl alone.
       124	pub(crate) fn step_impl(imp: &mut Vec<Clock>, op: &Op) {

Resolution: A small `pub(crate) trait Member` (`tick`, `ticks`, `fork`, `send`/`recv`, `sync`, `join`) implemented for `oracle::Clock` (with `ticks` as the literal loop and its comment) and `Clock`; one `step<M: Member>(pop: &mut Vec<M>, op: &Op)`; `run` as the fold from `vec![M::seed()]`; `master_differential` calls `step` per population. `replay` keeps its copy with a one-line reason at the site. State the op inventory as "the variants of [`Op`]" at all three sites. Acceptance: one applier body in optrace.rs; adding an `Op` variant fails to compile in exactly one place; no prose lists the op variants.

Synthesis note: clock/tests.rs's `master_differential` is the third copy this entry names; testing-oracles-11 (this document) is the fourth (`replay`'s steppers). One `Member` trait or one `step_oracle`/`step_fs` pair serves all four.

#### testing-diff-gen-20: `decode` in compactness/tests.rs renames a transcoding call as decoding
- Where: crates/before/src/testing/compactness/tests.rs:21-24 (related: crates/before/src/testing/compactness/tests.rs:12, 76, 79, 83, crates/before/src/meter.rs:116-121)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (meter.rs:116-121 documents `Packed::version` as "transcoding the construction language ... into the skyline coding"); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: deliberate-but-expired (a working `Version::decode` until faf3cd0a replaced the body and kept the name and doc)
- Owner-gated: no

A name and doc that describe work the function no longer does: `decode` is the strict wire validator elsewhere in the crate; this wraps the transcoder. Also the `use crate::meter::registry::Shape;` at line 12 sits above the `proptest` import, outside the file's crate-import group.

Evidence:

        21	/// Decode a meter-generated packed shape into a `Version`.
        22	fn decode(packed: &meter::Packed) -> Version {
        23	    packed.version()
        24	}

Resolution: Call `.version()` at the three sites and delete the wrapper; move the `Shape` import into the crate-imports group. Acceptance: no `fn decode` in compactness/tests.rs.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| testing-diff-gen-1 | `crates/before/src/testing/diff_ops.rs:110-159` | Eleven identity `Matches`/`FsMatches` impls could be two blanket impls | Two blanket impls, or state the closed-roster intent |
| testing-diff-gen-4 | `crates/before/src/testing/diff_ops.rs:848-870` | `BespokeGenre::GENRES` and `name()` restate the variant list twice | Derive the roster from an exhaustive match, or `strum::EnumIter` |
| testing-diff-gen-8 | `crates/before/src/testing/diff_ops/tests.rs:529-553` | `check_version_pair` and `check_party_pair` are one generic function written twice | One generic `check_pair<A, B>` |
| testing-diff-gen-19 | `crates/before/src/testing/compactness.rs:137-165` | `comb` hand-emits the min-lifted stream and self-checks it; the oracle path every other generator uses gives the same bits by construction | Build the comb as an `oracle::Version` |
| testing-diff-gen-21 | `crates/before/src/testing/snapshots.rs:45-82` | Gamma code widths are pinned twice: the snapshot table and codec's `gamma_costs` | Keep the snapshot; make `gamma_costs` a closed-form proptest, or retire one |
| testing-diff-gen-25 | `crates/before/src/testing/asymptotics.rs:97-101` | Idiom nits: qualified paths where the name is imported, a `Shape` name collision, and an undocumented block helper | Imports; rename `generators::Shape` to `DeepShape`; doc `version_block` |
| testing-diff-gen-30 | `crates/before/src/testing/shape_rows.rs:43-54` | The stack-merge tiling loop is written three times in `shape_rows.rs` | One `from_rows` closer |

### The envelopes, first half

7 entries (2 medium, 3 low, 2 nit); the full record is `evidence/partitions/envelopes-a.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: envelopes-a-8. Related findings in other documents: envelopes-a-2 (verification-gap: the segments column), envelopes-a-9 (test-quality: value legs on the door rows), envelopes-a-16 (claim: the flatness slack).

#### envelopes-a-4: Four envelope structs, four const constructors, four harness bodies, and five table preambles implement one measurement procedure
- Where: crates/before/tests/meter.rs:363-408 (related: 211-243, 1120-1172, 1206-1275, 1595-1635, 1669-1732, 6746-6805, 6894-6976; preambles 245-257, 1174-1189, 1637-1647, 1865-1875, 2139-2147; 265-267; 2635-2670; 5203-5208)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`grep -n '^fn [a-z_]*metered'` returns exactly 363, 1206, 1669, 6894; the improvement-tripwire message appears at 402, 1260, 1269, 1726, 5123, 5141, 6961, 6970, 8551; read all four struct/constructor/harness triples and the five preambles); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: already-known (the campaign note's post-campaign docket, line 1751-1755: "collapse the envelope harness shapes in `tests/meter.rs` into one five-column shape with per-column floor-or-NA")
- Owner-gated: no

`Envelope`/`envelope`/`metered`, `TouchEnvelope`/`touch_envelope`/`touch_metered`, `SweepEnvelope`/`sweep_envelope`/`sweep_metered`, and `QueryEnvelope`/`query_envelope`/`query_metered` are the same reset, run, read, print, assert sequence differing only in which columns exist, with two print-composition styles (two cfg'd `eprintln!`s versus `format!` fragments) and the tripwire message copied nine times. The cascade is visible: a row can pin only the columns its struct happens to carry, so `skyline_render_records_zero_touches` exists as a separate test because `SweepEnvelope` has no touch column, the tick rows moved to `query_env` with a pointer comment at 265-267 because the four-column table "never watched" touches, the door rows lack the scan column their kernel twins carry (envelopes-a-8), `ledger_wide_arming::run` reads densified digits ad hoc, and the preamble convention is restated five times with drift (only 1186-1189 carries the re-denomination sanction). Infrastructure that generates its own maintenance cascade is suspect; this one is already docketed.

Evidence:

       363	fn metered<R>(name: &str, input_bytes: usize, env: &Envelope, f: impl FnOnce() -> R) -> R {
      1206	fn touch_metered<R>(
      1669	fn sweep_metered<R>(
      6894	fn query_metered<R>(

       265	    // The tick rows live in `query_env`: the tick walk's cost currency
       266	    // is accumulator digit touches (with scanned bits beside it), which
       267	    // this four-column table never watched.

Resolution: This is the docketed unification; the note records it as a deferred disposition, so treat this entry as a reminder with the range's evidence attached. One `Envelope` with `peak_heap` and, per optional column (limb, scan, touch, densify), an `Option<Pin { ceiling, floor }>` where `None` means not watched; one `const fn`; one `metered` that walks a static column table and composes the MEASURED line from fragments as `sweep_metered` already does; the pin convention stated once in the file doc and cited by each table in a sentence. Fold `skyline_render_records_zero_touches` into the `SKYLINE_RENDER_*` rows as a touch column pinned to 0, move the tick scenarios (465-713, 2043-2122) beside the table they use and delete 265-267. Acceptance: one harness function and one envelope type serve every table; every existing row's numbers are unchanged; the tripwire message appears once; the pin convention appears once.

Synthesis note: envelopes-b-8 (this document) is the same docketed unification seen from the file's second half; envelopes-a-15 and envelopes-b-19 are the flatness-helper and fixture copies that dissolve with it. envelopes-a-8 and envelopes-a-11 (the door/kernel twin rows) fall out of the same change once every row can carry every column.

#### envelopes-a-15: Flatness helpers (`Run`, `assert_flat`, `assert_ceilings`, the 5/4 slack) are re-implemented per module and bypassed inside their own module
- Where: crates/before/tests/meter.rs:2344-2347 (related: 2351-2356, 2927-2931, 2966-2985; 2775-2809, 2882-2910, 4273-4311; 2731-2735, 2862-2866, 4238-4241; 5016-5021, 5186, 5295-5296; 6331, 6457-6460, 6485, 8080-8083, 8087, 8420; inline 4/5 ratios at 5439, 5512, 5635, 5811, 6333, 8476, 8578, 8960, 9153)
- Class / severity / confidence: modularity / medium / high
- Provenance: verified (grep over the whole file: `const SLACK_NUM` at 2344, 6457, 8080; `fn assert_flat` at 2393, 6331, 6485, 8087, 8420 with differing signatures plus `assert_flat_step` at 5869; `struct Run` at 2351, 5016, 6285, 6468, 7822, 8202, 8376, 8681, 9222 plus `QueryRun` at 2927; `fn assert_ceilings` at 2966 and 7857; the inline `* 4 <= ... * 5` ratios listed; read the three hand-rolled ceiling loops in range); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: no-rationale-found (the three `SLACK_NUM` copies arrived with three separate rounds; the inline loops at 2775-2809 and 2882-2910 predate `assert_ceilings`)
- Owner-gated: no

Within `skyline_flatness`, `Run` and `QueryRun` differ by one field, `assert_ceilings` accepts only `QueryRun`, so the `Run`-based bands re-spell its loop three times over `(u64, u64)` tuple constants that exist only because the `[(u64, u64); 2]` shape is not shared; `eq_early_exit::Run` and `ledger_wide_arming::run`'s bare 4-tuple add two more reading shapes, and the 5/4 ratio is inlined at ten sites across the file. Today every copy agrees on 5/4, which is luck the file relies on; a reader must re-verify each inline ratio's direction.

Evidence:

      2344	    const SLACK_NUM: u64 = 5;
      2347	    const SLACK_DEN: u64 = 4;

      2966	    fn assert_ceilings(name: &str, small: &QueryRun, large: &QueryRun, ceilings: [(u64, u64); 2]) {

      2775	        for (run, (touch_ceiling, limb_ceiling), scale) in [
      2776	            (
      2777	                &over_small,
      2778	                (
      2779	                    FREEZE_BAND_OVER_TOUCH_CEILINGS.0,
      2780	                    FREEZE_BAND_OVER_LIMB_CEILINGS.0,
      2781	                ),

      5186	    fn run(w: usize) -> (u64, u64, u64, u64) {
      5295	                u128::from(large) * u128::from(small_bytes) * 4
      5296	                    <= u128::from(small) * u128::from(large_bytes) * 5,

Resolution: Hoist one `Run` (with `deltas: Option<u64>`), one `assert_flat`, one `assert_ceilings`, and the slack constants into a file-level `#[cfg(feature = "limb-meter")] mod support` used by every band module; convert the tuple ceilings (`FREEZE_BAND_OVER_*`, `RANK_JUMP_*`, `DISTANCE_JUMP_PAIR_*`) to `[(u64, u64); 2]` and route the three inline loops through `assert_ceilings`; return a `Run` from `ledger_wide_arming::run`. Acceptance: one `const SLACK_NUM`, one `fn assert_flat`, one `struct Run` in the file; no inline `* 4 <= ... * 5` ratio remains.

Synthesis note: Dissolves with envelopes-a-4 and envelopes-b-8; envelopes-b-19 lists the second half's copies of the same helpers.

#### envelopes-a-11: SKYLINE_DECODE_* rows pin a meter-only wrapper whose limb and scan columns duplicate the validate rows exactly
- Where: crates/before/tests/meter.rs:293-300 (related: 288-292, 1500-1576; crates/before/src/version/skyline.rs:167-171, 211-216, 262-274; crates/before/src/version/skyline/decode.rs:14-20; crates/before/src/version.rs:1110-1127)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`mod decode` is `#[cfg(any(test, feature = "meter"))]`; `decode_bits` callers are skyline.rs:273 and skyline/tests.rs only; `Version::decode` runs `validate_prefix` and adopts the read buffer; the five decoder rows' limb and scan columns equal the five validator rows' (0/468_758, 88/17_923, 29_509/1_000_480, 2_443/312_503, 0/468_758); `decode_bits` frees the validator's transient before allocating the copy, so the row's heap is the larger of the two, and the Dense and AltSpine decode heap pins equal their validate pins exactly at 61_440); executed: no
- Seen by: scaffolding, adequacy; refutation: confirmed, severity low; history: deliberate-but-expired (warranted when `skyline::decode` transcoded the candidate coding into the production packed Version; orphaned at the flag day; retained under the campaign's never-remove-tests constraint, which has lapsed; f6c0a6a4 re-pinned them to the copy rather than dissolving)
- Owner-gated: yes: removal of an instrument, governed by the note's dissolution ratchet

The world without these five rows loses only a heap pin on a five-line meter-only function allocating more than one copy; the production decode door is already pinned by `DECODE_DENSE`/`BIGROOT`/`HUGELEAF`/`CLIFF`, and every other column is pinned by the validate rows byte for byte. Suites exercise the public API, with internal entries as documented exceptions; here the exception pins nothing outside itself.

Evidence:

       296	    pub const SKYLINE_DECODE_DENSE: SweepEnvelope = sweep_envelope(61_440, 0, 0, 468_758, 0); // decode is validate + wrap: the wrap allocates the copy once, exactly sized
       288	    pub const SKYLINE_VALIDATE_DENSE: SweepEnvelope = sweep_envelope(61_440, 0, 0, 468_758, 0); // the open-ancestor bit stack; word-valued payloads keep the limb column at zero

    skyline.rs:
       269	/// Test- and meter-only: the production decode ([`Version::decode`]) validates
       270	/// the prefix and adopts the buffer without this wrapper.

Resolution: Dissolve the five `SKYLINE_DECODE_*` rows and their tests (1500-1576), keeping the round-trip equality as a plain unit test if `skyline::decode` stays for the tests' vocabulary; if WideTooth and AltSpine matter at the decode door, add `DECODE_WIDE_TOOTH` and `DECODE_ALT_SPINE` through `Version::decode`. Acceptance: no envelope row measures `meter::skyline::decode`; the public `DECODE_*` rows cover every shape the owner wants pinned at decode.

Synthesis note: A specific instance of envelopes-a-8's door/kernel twin pattern; retire the five rows under whichever roster envelopes-a-8's ruling keeps.

#### envelopes-a-19: Closed forms, the tick-family fixture, and the `UBig`-to-`Ticks` round trip are spelled twice, and generator widths and file-level scales appear as literals
- Where: crates/before/tests/meter.rs:3503-3506 (related: 3977-3980; 3719-3721 and 4034-4036; 729-745 and 888-904; 770-781 and 796-815; 8 `parse::<before::Ticks>` sites plus 880-885; literals: 4352 and 4418 against 188, 4398-4399 and 4463-4464 against 191, 2418-2419, 2486-2487, 2553-2554, 2625-2626, 5102-5103, 923 and 932 (`64`), 2079-2080; widths 288/289/290/608/33/34/16 at 3503-3506, 3719-3721, 3818, 4174, 4205, 4510, 4609-4618, 4798-4800, 5191; crates/before/src/meter.rs:1630, 1689, 1696, 1701, 1799, 2153, 2967)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read both copies of each form; grep confirms `MASK_DRIFT_MAGNITUDE_BITS = 512` at 188 and `MASK_DRIFT_TEETH = 1_024` at 191 beside `packed_triple(512, scale)` and `masked_cmp_run(1_024)`; the generator constants `FREEZE_POSITION_DROP_BITS = 288`, `PROMOTION_REARM_ARM_BITS = 608`, `PROMOTION_REARM_SETTLE_BITS = 288`, `PROMOTION_REARM_LEVELS_PER_BLOCK = 32`, `DENSE_SUFFIX_DIGIT_STRIDE = 33`, `LONE_FREEZE_PLATEAU_BITS = 288`, `JUMP_PAIR_DIGIT_STRIDE = 33` exist privately in src/meter.rs); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed ([13]/[52]) and reframed ([37]: the closed-form widths are legitimately spelled independently of the generator as an oracle, but deserve one named definition in the band module); history: no-rationale-found (64ed0d1c introduced `MASK_DRIFT_MAGNITUDE_BITS` and the literal 512 in the same commit; 36afe22e copied the closed forms; the seam bands landed later with named `*_ticks` helpers)
- Owner-gated: no

A closed form is the value leg every band rests on; two copies can drift apart when a generator changes and only one is updated (the seam and ladder bands already show the better shape: `seam_plunge_ticks`, `latent_ladder_ticks`). The 512/1_024/2_048 literals shadow constants the file already names. The widths 288 and 608 are the oracle's independent statement of the generator's premises, which is the point, but a reader cannot tell 608 from 288 without opening src/meter.rs; one named definition per width at the top of `skyline_flatness`, with a comment naming the generator constant it mirrors, keeps the independence and the legibility.

Evidence:

      3503	        let band = 289 + (usize::BITS - k.leading_zeros()) as usize;
      3504	        let expected = (UBig::from(2 * k as u64) << band)
      3505	            + UBig::from((k * (k - 1)) as u64) * ((UBig::ONE << 288usize) + UBig::ONE)
      3506	            + UBig::from(k as u64);

      3977	            let band = 289 + (usize::BITS - k.leading_zeros()) as usize;
      3978	            (UBig::from(2 * k as u64) << band)
      3979	                + UBig::from((k * (k - 1)) as u64) * ((UBig::ONE << 288usize) + UBig::ONE)
      3980	                + UBig::from(k as u64)

      4352	        let (comb, mask, plateau) = Shape::MaskDriftTriple.packed_triple(512, scale);
       188	const MASK_DRIFT_MAGNITUDE_BITS: usize = 512;

Resolution: Add `freeze_position_ticks(k)`, `promotion_rearm_ticks(p)`, a `tick_families()` fixture, and a `fn ticks(u: &UBig) -> before::Ticks` beside the existing closed-form helpers; reference `super::MASK_DRIFT_MAGNITUDE_BITS`, `super::MASK_DRIFT_TEETH`, `super::CLIFF_SCALE` in the band modules; name the small-run scales per band; name the word slack at 923/932; define the closed-form widths once in `skyline_flatness`; use `div_ceil(8)` for both terms at 2080. Acceptance: each closed form and the tick fixture have one definition; the decimal round trip appears once; no bare 512/1_024/2_048 where a file-level constant exists; no bare 288/608/33 in a closed form.

Synthesis note: meter-core-7 (this document) derives the same widths in src; keep this side independent as the entry argues, named once.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| envelopes-a-7 | `crates/before/tests/meter.rs:261-263` | Presentation hygiene: half-aligned rustfmt::skip tables, em-dashes in comments and messages, a whitespace run, a terse floor message | Align or drop the skip tables; colons in messages; collapse the whitespace |
| envelopes-a-13 | `crates/before/tests/meter.rs:410-424` | Mixed idioms for one purpose: `consumed` beside `black_box`, a one-line `version_of` alias, qualified `Ordering`, bare-bool selectors | `black_box` everywhere; delete `consumed` and `version_of`; import `Ordering`; an enum over the bools |

### The envelopes, second half

10 entries (1 medium, 5 low, 4 nit); the full record is `evidence/partitions/envelopes-b.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: envelopes-b-17. Related findings in other documents: envelopes-b-12 (documentation: the compaction-off demonstration's citation).

#### envelopes-b-8: Four envelope structs and four metering harnesses are column-subset copies of one another
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

Synthesis note: envelopes-a-4 (this document) is the same unification from the first half, with the campaign note's docket cited; the two open questions on column shape (`Option` columns or every column pinned) are the same question.

#### envelopes-b-5: `id_walk_scan_cost`'s flatness and floor asserts are implied by its exact-equality pins, and its doc hand-maintains derivable byte counts
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

#### envelopes-b-16: The masked-hole depth band re-pins the envelope row's touch columns as separate constants
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

#### envelopes-b-19: The two-scale flatness assertion, the counter readers, the clock-history fixture, `tick_run`, and the `UBig`-to-`Ticks` conversion are hand-copied across modules
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

Synthesis note: Dissolves with envelopes-a-4 and envelopes-b-8; envelopes-a-15 lists the first half's copies. The `From<UBig> for Ticks` half is the open question the envelopes-b summary records (recommendation: the test-local helper).

#### envelopes-b-26: Derived liveness floors are hand-computed literals beside inline scale literals
- Where: crates/before/tests/meter.rs:8817-8831 (related: 9006-9024, 9094-9104, 9282-9294; the scales at 8866, 9054, 9140, 9324; the derived-from-a-named-parameter form at 5473)
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

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| envelopes-b-11 | `crates/before/tests/meter.rs:6960-6963` | Em-dashes in assertion messages and line comments | Colons or semicolons; one site once the harnesses unify |
| envelopes-b-13 | `crates/before/tests/meter.rs:7155-7157` | Qualified paths where imports exist or would serve | File-level imports for `UBig`, `Rank`, `Ticks` |
| envelopes-b-14 | `crates/before/tests/meter.rs:7181-7185` | Small redundancies: an operand built twice, a fixture built twice, a byte assert that restates `Eq`, a string-length proxy, a one-constant module | Build each generator and fixture once; name the depths; `TryFrom<&Ticks>` for the word test |
| envelopes-b-23 | `crates/before/tests/meter.rs:8580-8580` | A dropped line continuation leaves fourteen spaces inside an assertion message | Rewrap the literal with `\` |

### Other suites

7 entries (1 medium, 4 low, 2 nit); the full record is `evidence/partitions/tests-other.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: tests-other-3. Related findings in other documents: tests-other-6 and tests-other-14 (verification-gap: the two flatness criteria without floors), tests-other-24 (verification-gap: citecheck as the collection authority), tests-other-30 (verification-gap: the leg roster).

#### tests-other-7: Duplicated meter and fixture helpers across the metered binaries, with the isolation premise stated in none of them
- Where: crates/before/tests/answer_embedded.rs:31-61 (related: crates/before/tests/fold_skeleton.rs:20-30, crates/before/tests/fold_skeleton.rs:51-59, crates/before/tests/coincident_span.rs:20-24, crates/before/tests/coincident_span.rs:31-38, crates/before/tests/coincident_span.rs:146-153, crates/before/tests/meter.rs:9479, crates/before/tests/meter.rs:354-355, crates/before/src/meter.rs:3618-3630, crates/before/src/meter/board/family.rs:836-845, crates/before/src/party.rs:251-252)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (grep: `fn counters` at answer_embedded.rs:31 and fold_skeleton.rs:20, byte-identical; `fn scanned` at coincident_span.rs:20 and four sites in tests/meter.rs; the balanced-fork loop at answer_embedded.rs:49-61, fold_skeleton.rs:51-59, family.rs:836-845; the `rounds` closure twice in coincident_span.rs); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no rationale found; the isolation premise is documented at the facade (src/meter.rs:3619-3620) and the index (validation_index.rs:97-99) but not at these reading sites (deliberate-and-holds for the premise, not for the omission)
- Owner-gated: no

`counters` is byte-identical in two files, `scanned` is defined five times, the power-of-two fork tiling is hand-rolled three times (the public `Party::forks` and `From<Party> for [Party; N]` already provide it), and `rounds` is duplicated inside one file. `counters` also bypasses the `meter` facade (`meter::touch_ops`/`reset_touch_ops`) for `suanpan::touch_meter` directly, so these suites never meet the sentence that states the process-global isolation premise, and unlike `tests/meter.rs` they append no `ISOLATION_NOTE` to a failure. The duplicates are where the liveness-floor omission (tests-other-14) crept in independently of `coincident_span.rs`'s correct pattern.

Evidence:

        31	fn counters(f: impl FnOnce()) -> (u64, u64, u64) {
        32	    meter::reset_scan_bits();
        33	    meter::reset_limb_ops();
        34	    suanpan::touch_meter::reset();
        35	    f();
        36	    (
        37	        meter::scan_bits(),
        38	        meter::limb_ops(),
        39	        suanpan::touch_meter::touches(),
        40	    )
        41	}
       ...
        49	fn fork_parties_from(seed: Party, n: usize) -> Vec<Party> {
        50	    let mut parties = vec![seed];
        51	    while parties.len() < n {

Resolution: Add `tests/support/meters.rs` with a `Counters { scan, limb, touch }` struct, `counters(f)` routed through `meter::*` and carrying the dead-meter floor and the isolation note, `scanned(f)`, and `balanced_forks(n)`; include it by `#[path]` from the four binaries (and offer it to tests/meter.rs); hoist `rounds` into `fixture()` in coincident_span.rs. Replace the tiling loops with `<[Party; 16]>::from(Party::seed())` / `seed.forks(n - 1)` only after confirming the leaf order the `step_by(2)` alternation in `wt` depends on (see the open questions). Acceptance: one definition each of `counters` and `scanned` under `tests/support/`; no `while parties.len() < n` loop in the two files; MEASURED grids unchanged against the parent, or a changed grid explained by the leaf-order difference.

Synthesis note: suite-economics-4 (this document) is the same fixture and helper duplication with the `[[test]] required-features` remedy; the suite-economics summary's open question 2 asks why the three satellites live outside `tests/meter.rs`.

#### tests-other-9: `wide_display_pair_expectations_are_split` is a strict consequence of the red-membership pin
- Where: crates/before/tests/bench_judge_roster.rs:124-129 (related: crates/before/tests/bench_judge_roster.rs:60-63, crates/before/tests/bench_judge_roster.rs:118-123)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (read: line 62 pins the red class to exactly `["display_schoolbook/hugeleaf"]`, which entails both assertions at 127-128); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-but-expired (at 7542d613 the red set held sixteen bigroot cells plus the schoolbook cell and a `boundary` class existed; c95230c8 shrank the red set to one cell, which is when this test became implied)
- Owner-gated: no

A test that cannot fail while its sibling passes names nothing it alone catches (Principle 3) and costs a reader a comparison to discover that. The rationale in its doc (the pair separates the conversion classes only while one member is required red and the other green) is worth keeping, on the pin that enforces it.

Evidence:

        61	fn roster_red_membership_is_pinned() {
        62	    assert_eq!(class(&roster(), "red"), ["display_schoolbook/hugeleaf"]);
        63	}
       ...
       124	#[test]
       125	fn wide_display_pair_expectations_are_split() {
       126	    let red = class(&roster(), "red");
       127	    assert!(red.contains(&"display_schoolbook/hugeleaf".to_string()));
       128	    assert!(!red.contains(&"version_display_wide/hugeleaf".to_string()));
       129	}

Resolution: Delete `wide_display_pair_expectations_are_split` and fold its two-sentence rationale into `roster_red_membership_is_pinned`'s doc. (The verbatim `TEXT_CEILING_CELLS` pin at 103-116 is deliberate tamper-evidence per the module doc and stays; a semantic predicate over the cell IDs would be an optional addition.) Acceptance: three tests remain, each failable by an edit that leaves the others green.

#### tests-other-23: "mint" for constructing a value, including two test names
- Where: crates/before/tests/stale_state.rs:27 (related: crates/before/tests/stale_state.rs:6, crates/before/tests/stale_state.rs:22, crates/before/tests/stale_state.rs:79, crates/before/tests/stale_state.rs:86, crates/before/tests/stale_state.rs:93, crates/before/tests/amp_board_smoke.rs:354)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`grep -n -i mint` over the thirteen files returns exactly these seven sites; the two test names are cited nowhere in src, tools, or .github; crates/before/src carries 58 further lines, so the crate-wide question is a separate sweep); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no repo rule forbids the word; the names were set in a474e189 by the agent that re-documented the witnesses
- Owner-gated: no

The vocabulary rule: never write "mint" for constructing a value. In function names the coinage also becomes a search key ("re_mints") no reader would guess, and it cuts against the file's own model (a version is knowledge, not a coin).

Evidence:

        27	fn same_party_ticks_on_divergent_clones_mint_equal_versions() {
       ...
        86	fn from_parts_over_an_earlier_version_re_mints_its_successor() {

    (amp_board_smoke.rs:354)
       354	/// band can only mint its operands through `registry::Shape`.

Resolution: Rename to `same_party_ticks_on_divergent_clones_produce_equal_versions` and `from_parts_over_an_earlier_version_reproduces_its_successor`; "produces"/"yields"/"re-derives" at lines 6, 22, 79, 93; "build its operands" at amp_board_smoke.rs:354. Acceptance: `grep -rn -i mint crates/before/tests` is empty.

#### tests-other-29: `check` is a 490-line body with a closure-as-method and a free `intern` over three `&mut` fields
- Where: crates/before/tests/verdict_matrix.rs:760-765 (related: crates/before/tests/verdict_matrix.rs:438-453, crates/before/tests/verdict_matrix.rs:720-1207, crates/before/tests/verdict_matrix.rs:1134-1138, crates/before/tests/verdict_matrix.rs:1148-1152, crates/before/tests/verdict_matrix.rs:675-692)
- Class / severity / confidence: idiom / low / medium
- Provenance: assessed (read); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no rationale found (fcb78e44 reshaped the adequacy machinery without restructuring `check` or `intern`)
- Owner-gated: no

`flag` is a closure whose first parameter is `&mut Outcome`, a method spelled as a local; `intern` takes the three fields of a pool builder separately; `check` interleaves five axes so a reader auditing one axis's legs against the `Axis` docs must scan the whole function; and the dominance/precedence census labels are inline `match`es beside the named `placement_label`/`coverage_label` fns. The cost is the one tests-other-30 names: the leg roster per axis is not visible in one place.

Evidence:

       760	    let flag = |outcome: &mut Outcome, axis: Axis, leg: &'static str, detail: String| {
       761	        *outcome.counts.entry((axis, leg)).or_insert(0) += 1;
       762	        if outcome.samples.len() < SAMPLE_CAP {
       763	            outcome.samples.push(format!("{axis:?}/{leg}: {detail}"));
       764	        }
       765	    };

Resolution: `impl Outcome { fn flag(&mut self, axis, leg, detail) }`; a `PoolBuilder { versions, stable, index }` with `fn intern(&mut self, v, ordinal) -> usize`; split `check` into per-axis helpers plus `check_span_grid` and `check_conjunction_grid`, each carrying its axis's leg names; `dominance_label`/`precedence_label` for symmetry. Do it together with tests-other-30's leg roster. Acceptance: no closure takes `&mut Outcome`; `check` is a dispatcher; the twins and the production run pass unchanged with identical leg names.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| tests-other-19 | `crates/before/tests/fuzz_seeds.rs:52` | Small idiom slips: a qualified path beside its import, a bare `unwrap` among expect-proofs, a display comparison where `is_seed` exists, a broken doc wrap | Import `BTreeMap`; an expect-proof; `is_seed()`; reflow the doc |
| tests-other-25 | `crates/before/tests/support/fuzz_seed_set.rs:129-131` | Dead `let _ = ...version();` lines that suppress nothing | Delete the dead lines |

### Benches and examples

8 entries (3 medium, 1 low, 4 nit); the full record is `evidence/partitions/benches-examples.md`. Related findings in other documents: benches-examples-12 (verification-gap: the presize A/B record), benches-examples-18 and -22 (documentation: code_study.rs and perf_probe.rs), benches-examples-25 (claim: results/benchmarks).

#### benches-examples-1: benches/amplify.rs re-lists three board cells the judged board bench already times
- Where: crates/before/benches/amplify.rs:1-24 (related: crates/before/Cargo.toml:139-141; crates/before/benches/board.rs:4-12; crates/before/src/meter/board/export.rs:3-11, 144-150; crates/before/src/meter/board/ops.rs:112-115, 197, 266, 1428-1441; crates/before/src/meter/board/family.rs:524-533, 797-803; crates/before/src/meter/registry.rs:1261-1299)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (read the board's op rows `version_decode`, `version_join`, `party_without`, the family specs for hugeleaf, bigroot, id-pair with `Coverage::Board`, `designed` at ops.rs:112-115 admitting all three pairings, and `bench_cells`'s pinned rule; `grep -rn amplify` across the tree hits only Cargo.toml:140 and the file itself; `git log --follow` dates amplify.rs to 67a5afe6 and board.rs to 8138cb9c, both 2026-07-23, amplify one commit earlier); executed: no
- Seen by: scaffolding [0], adequacy [18], structure-prose [29], instrument-correctness [54]; refutation: confirmed, with one correction to the equivalence (below); history: no rationale found (never re-justified after board.rs landed; 608dea84 maintained it without saying why it stays)
- Owner-gated: yes (removes a `[[bench]]` target)

amplify.rs hand-lists three op x shape rows at two fixed sizes each, with no denominator sidecar, no ceiling, and no consumer, while benches/board.rs derives the same three pairings (`version_decode/hugeleaf`, `version_join/bigroot`, `party_without/id-pair`) from the board's axes and times them under the judge with the scale parameter. board.rs:12 and export.rs:10-11 state that the bench cell set is "never a second list". Principle 3 (circular justification): the file's only reason to exist is that it predates board.rs by one commit. One operand differs: `bench_join_bigroot` joins bigroot with a fresh one-tick version (line 47) where the board's `version_join x bigroot` uses `version2`, the bigroot ticked once by `Party::seed()` (family.rs:799-802).

Evidence:

         1	//! Adversarial-shape benchmarks: the `before::meter` generator inputs, timed
         2	//! at small sizes so the resource-proportionality paths register as
         3	//! wall-clock numbers.
         4	//!
         5	//! No oracle comparison: these rows exist to make a superlinear regression
         6	//! on a worst-case shape visible in `cargo bench`, complementing the
         7	//! deterministic envelopes in `tests/meter.rs`.
        16	const HUGELEAF_BITS: &[usize] = &[8_192, 32_768];
        20	const BIGROOT_SIZES: &[(usize, usize)] = &[(4_096, 256), (16_384, 1_024)];
        24	const ID_SPINE_DEPTH: &[usize] = &[8_192, 32_768];
        47	        let one = Version::try_from(1u64).expect("a one-tick version is valid");

    board.rs:
        12	//! derived from the board's own axis declarations, never a second list. The deterministic record stays with the board and the

Resolution: Delete benches/amplify.rs and the `[[bench]] name = "amplify"` stanza (Cargo.toml:139-141). If the join-with-a-one-tick-version operand is wanted, add it as a board row so it gets the judge, rather than keeping a parallel list. Acceptance: `grep -rn amplify crates/before/benches crates/before/Cargo.toml` is empty; `just bench-build` and `just clippy` stay green; the pinned sidecar written by `just bench-judge` still lists `version_decode/hugeleaf`, `version_join/bigroot`, and `party_without/id-pair`.

#### benches-examples-19: Hand-synchronized copies with parity asserted in prose: code_study.rs duplicates space_consumption.rs's simulation; perf_probe.rs duplicates benches/common
- Where: crates/before/examples/code_study.rs:335-340 (related: crates/before/examples/code_study.rs:39-40, 337-426; crates/before/examples/space_consumption.rs:266-374; crates/before/examples/perf_probe.rs:18-141, 220-221; crates/before/benches/common/mod.rs:36-239; crates/before/examples/fuzz_seeds.rs:13-14; crates/before/tests/bench_judge_roster.rs:15-17)
- Class / severity / confidence: modularity / medium / high
- Provenance: verified (side-by-side read: code_study.rs:337-397 equals space_consumption.rs:269-374 body for body with docs stripped and `Scenario` flattened to `tag: u64`; perf_probe.rs:18-141 equals benches/common/mod.rs:36-239 minus the two `plan` asserts and with `take_group` inlined; the `#[path]` sharing idiom is live at fuzz_seeds.rs:13-14 and bench_judge_roster.rs:15-17); executed: no. The resolution of `pub mod sidecar;` inside a `#[path]`-included file named mod.rs was assessed against rustc's module rules, not compiled.
- Seen by: scaffolding [5], adequacy [16], structure-prose [27 part], [34], instrument-correctness [52]; refutation: confirmed; history: no rationale (8da8f815 ported the study with the copy in it; 7d101fe7 wrote the probe's copy; both postdate the idiom, bb6ea8b7)
- Owner-gated: no

"No hand-maintained restatements of enumerable facts": "same step functions, same per-run seeding" (code_study.rs:39-40) and "Same salts as benches/clock.rs" (perf_probe.rs:220-221) are claims the code can falsify without touching the prose; a change to the paper's step model or to the bench corpus silently changes what the study's REALISTIC corpus and the probe's "same corpus" mean. The `Scenario` type is lost to a bare `u64` on the way.

Evidence:

       335	// ─── the realistic simulation (space_consumption replicated, reduced) ──────
       336	
       337	fn seed_for(tag: u64, n: usize, run: u64) -> u64 {
       338	    const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;
       339	    GOLDEN ^ (tag << 56) ^ ((n as u64) << 32) ^ run
       340	}
        39	//! - REALISTIC: the `space_consumption` simulation (same step functions,
        40	//!   same per-run seeding), at the reduced parameters in the constants

    perf_probe.rs:
       220	    // Same salts as benches/clock.rs: tick uses salt 1 (1 group), join salt 3
       221	    // (2 groups).

Resolution: move `Scenario`, `seed_for`, `checkpoints`, `build_population`, `step_data`, `step_process` into `examples/support/simulation.rs` and include it from both examples with `#[path]`, so code_study's `tag: u64` becomes `Scenario`; replace perf_probe.rs:18-141 with `#[path = "../benches/common/mod.rs"] mod common;` and `use common::{SEED, plan, impl_clocks, hole_pair, oracle_clocks}` (name the salts as constants in common if the correspondence with benches/clock.rs matters, and use them on both sides). The code_study half is moot if benches-examples-18 dissolves the example. Acceptance: one definition of each simulation and corpus function in the tree; `just check` green; the two parity comments disappear or read as mechanical ("shares `common::plan`").

#### benches-examples-21: emit_probe.rs exercises no code in the tree, is the sole reason `bitvec` is a dev-dependency, and its one assert names a path that does not exist
- Where: crates/before/examples/emit_probe.rs:50-52 (related: crates/before/examples/emit_probe.rs:1-11, 32-48, 71; crates/before/Cargo.toml:33-36; Cargo.toml:22; .agent-notes/2026-08-04-perf-probe/README.md:15-17; .agent-notes/2026-08-04-perf-probe/probe-report.md:63-69)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (the file imports only `bitvec` and std; `grep -rln bitvec --include='*.rs' --include='*.toml'` hits the workspace Cargo.toml, crates/before/Cargo.toml, and this example; `git log --follow` shows 7d101fe7 (added 2026-08-04) then 83e61b4d (2026-08-18, "bitvec leaves the production dependency graph (it stays a dev-dependency as emit_probe's external baseline)"); the only `spill` in the file is the local at line 75; the numbers it produced are at probe-report.md:66-69); executed: no
- Seen by: scaffolding [1 part], adequacy [17], structure-prose [28], instrument-correctness [53 part]; refutation: confirmed; history: deliberate-but-expired (the keep at 83e61b4d and the note's "can still be run" name no consumer of the comparison going forward)
- Owner-gated: yes (a recorded keep decision)

Principle 3: machinery outlives the constraint that justified it. The probe compares `bitvec` against a standalone `WordWriter` "reproduced standalone so the comparison needs no crate internals"; neither arm is `before`, so it cannot observe the shipped `PackedBuilder`/`BitsBuf` regress; its timing loop hand-rolls what criterion provides; the decision it priced landed, and its record has a home. Line 71's message names a "spill path" this file never had.

Evidence:

        50	/// A minimal word-buffered MSB-first bit writer: the staging discipline
        51	/// the crate's own `PackedBuilder` ships, reproduced standalone so the
        52	/// comparison needs no crate internals.
        71	        debug_assert!(len <= 63, "codes wider than 63 bits take the spill path");

    Cargo.toml (crates/before):
        34	# The emit_probe example's external comparison baseline: the production
        35	# build buffer is the crate-owned BitsBuf, and nothing shipped links bitvec.
        36	bitvec = { workspace = true }

Resolution: delete the example and the `bitvec` dev-dependency (and the workspace entry, which nothing else uses); the note is the record. If a standing primitive-cost comparison is wanted, write it as a criterion `[[bench]]` that times the crate's actual `BitsBuf`/`PackedBuilder` against `bitvec`, so a regression is a criterion-tracked number, and fix the assert message to the guard it enforces (`(1u64 << spill) - 1` overflows at `len == 64`). Acceptance: `grep -rn bitvec crates/ Cargo.toml` returns nothing (or the probe times `before`'s builder); `just check` and `just bench-build` clean; the perf-probe README updated to say the probe retired and where its numbers live.

Synthesis note: deps-7 (this document) is the dependency-side record of the same probe; both recommend retiring emit_probe.rs and `bitvec`. perf_probe.rs's fate is benches-examples-22 (documentation) and the benches-examples summary's open question 2.

#### benches-examples-4: benches/common spells the universe build and group fold five times, carries dead `map_err` adapters, and has `rng` copied into four targets
- Where: crates/before/benches/common/mod.rs:104-239 (related: crates/before/benches/common/mod.rs:169, 204, 232; crates/before/benches/party.rs:13-17; crates/before/benches/version.rs:15-17; crates/before/benches/clock.rs:12-14; crates/before/benches/presize.rs:53-55; crates/before/examples/perf_probe.rs:71, 104, 135, 181, 341; crates/before/src/clock.rs:218, 916)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (side-by-side read of `impl_parties`, `oracle_parties`, `impl_clocks`, `hole_pair`, `oracle_clocks`: the same fork-by-schedule loop and, in four of them, the same take_group/reduce/join fold; `impl Debug for Clock` at src/clock.rs:916 is unconditional and `Clock::join` returns `Result<&Version, Clock>` at :218, so `.expect` compiles without the adapter, and the Party fold at mod.rs:116 already calls `expect` directly; `fn rng(salt)` appears in four bench targets, documented in one); executed: no
- Seen by: scaffolding [12], structure-prose [32], [33]; refutation: confirmed; history: no rationale (the original 6e69b427 shape; the adapter was dead at origin)
- Owner-gated: no

Legibility: the module's central promise (line 124, "same plan, structurally identical trees") is a property of one code path only if there is one code path; five transcriptions must be diffed by eye. The `.map_err(|_| ())` before `expect` says nothing, because `Clock: Debug`; the party fold beside it shows the plain form.

Evidence:

       104	pub fn impl_parties(plan: &Plan, groups: u8) -> Vec<Party> {
       105	    let mut universe = vec![Party::seed()];
       106	    for &i in &plan.schedule {
       107	        let child = universe[i].fork();
       108	        universe.push(child);
       109	    }

       167	                .reduce(|mut acc, c| {
       168	                    acc.join(c)
       169	                        .map_err(|_| ())
       170	                        .expect("universe members are pairwise disjoint");

Resolution: two private generics, `universe<T>(seed: T, schedule: &[usize], fork: impl FnMut(&mut T) -> T) -> Vec<T>` and `fold_groups<T>(slots: Vec<T>, label: &[u8], groups: u8, join: impl FnMut(&mut T, T)) -> Vec<T>`; express the five builders through them (`hole_pair` reuses `universe` and a full fold); delete every `.map_err(|_| ())` (also in perf_probe.rs); move `rng` with party.rs's doc comment into common as `pub fn rng(salt: u64) -> StdRng`. Acceptance: one fork loop and one group fold in the module; `grep -rn 'map_err(|_| ())' crates/before/benches crates/before/examples` returns nothing; one `fn rng` under crates/before/benches; bench IDs, seeds, salts, and inputs unchanged.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| benches-examples-2 | `crates/before/benches/board.rs:12-12` | Mechanical slips in board.rs and presize.rs: a 126-column doc line, a split import group, a forward-looking clause | Re-wrap the doc line; group the imports; reword the forward-looking clause |
| benches-examples-9 | `crates/before/benches/common/sidecar.rs:159-187` | The denominator sidecar hand-rolls JSON with serde_json already a dev-dependency | Derive `Serialize` with `preserve_order`, or leave the writer as is |
| benches-examples-16 | `crates/before/benches/tripwire.rs:46-48` | Knuth's MMIX multiplier inlined as an unnamed literal in two instrument files | Name `LCG_MULTIPLIER` once; use it at all sites |
| benches-examples-20 | `crates/before/examples/code_study.rs:513-525` | code_study.rs dispatches on a string label and keeps parallel per-code arrays | Carry the denominator in the tuple; one `Vec<Option<u128>>`; moot if the example dissolves |

### Fuzz targets, guests, and pins

10 entries (6 low, 4 nit); the full record is `evidence/partitions/fuzz-guests-pins.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: fuzz-guests-pins-4. Related findings in other documents: fuzz-guests-pins-14 (verification-gap: the flat heap cap), fuzz-guests-pins-33 (claim) and fuzz-guests-pins-35 (verification-gap): the memory-terminal pins.

#### fuzz-guests-pins-12: `drive_groups!` duplicates the in-tree `organic_drive!` selection macro arm for arm
- Where: crates/before/fuzz/fuzz_targets/fuzz_laws.rs:122-180 (related: crates/before/src/testing/algebraic_laws/tests.rs:336-394, crates/before/src/laws.rs:109-134)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (read both macros against the roster: eighteen arms each over the same signatures, differing in field name `k` versus `c` and pool indices `p[0]` versus `p[1]`/`p[2]`, `v[0]` versus `v[1]` for `(clock, version)`); executed: no
- Seen by: structure-prose [54]; refutation: confirmed; history: no-rationale-found (86dd53a71 landed both without explaining two selection macros)
- Owner-gated: yes: adds an exported macro to the `laws` instrument feature

The signature-to-inputs selection is a property of the roster, not of either driver, yet both must be extended in lockstep whenever `for_each_law_group!` gains a signature, and the compile-time totality each claims is enforced twice. The two copies already differ in pool indices; nothing appears to depend on the difference.

Evidence:

    122	macro_rules! drive_groups {
    123	    (args: ($env:expr); $(($group:ident, $driver:ident, $shape:tt)),* $(,)?) => {
    124	        $( drive_groups!(@one $env, $group, $shape); )*
    125	    };
    126	    (@one $env:expr, $group:ident, (version)) => {
    127	        drive!(before::laws::$group, $env.v[0]);
    128	    };

Resolution: Move the eighteen selection arms into `laws.rs` beside `for_each_law_group!` as a macro taking the environment expression and the assertion macro (`assert!` here, `assert_laws!` in the tests), and have both consumers invoke it; fix the pool-index choice once. Acceptance: one spelling of the eighteen arms in the tree; a signature added to `for_each_law_group!` breaks the build in exactly one place.

#### fuzz-guests-pins-18: The fuzz-fit guest hand-expands its register-file accessors, split borrows, and verdict tables, and the copies already drift
- Where: crates/before/fuzzfit/guest/src/lib.rs:123-245 (related: crates/before/fuzzfit/guest/src/lib.rs:369-378, crates/before/fuzzfit/guest/src/lib.rs:440-450, crates/before/fuzzfit/guest/src/lib.rs:478-524, crates/before/fuzzfit/guest/src/lib.rs:582-587, crates/before/fuzzfit/guest/src/lib.rs:685-706, crates/before/fuzzfit/guest/src/lib.rs:934-960, crates/before/fuzzfit/guest/src/lib.rs:1238-1281, crates/before/fuzzfit/guest/src/lib.rs:1288-1291, crates/before/fuzzfit/guest/src/lib.rs:1305-1308, crates/before/fuzzfit/guest/src/lib.rs:1497-1507, crates/before/fuzzfit/guest/src/lib.rs:1758-1768, crates/before/fuzzfit/harness/src/ops.rs:640-652, crates/before/fuzzfit/harness/src/ops.rs:910-920)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep: `split_at_mut` four times; `Some(Some(Val::C(c)))` at 234 and 242 in the helpers and re-inlined at 371, 442, 1289, 1306, while `with_c` is called once, at 776; `Placement::After => 8` twice; the FNV offset and prime twice each at 686/701 and 689/706; the sign mask four times at 623, 691, 738, 1086; `hit as i32` at 1560 and 1830 against `i32::from` elsewhere); executed: no
- Seen by: scaffolding [14]; adequacy [35]; structure-prose [40],[41]; refutation: confirmed, reframed to legibility and maintenance (a divergent verdict copy fails loudly against the harness's native `expect`, ops.rs:644-649, so no masked differential); history: no-rationale-found (`with_c` arrived with the shape walk in 46eb64f9 and the four older clock kernels were never migrated)
- Owner-gated: no

Four `take_*` and eight `with_*` accessors are one pattern per slot type; the two-index `split_at_mut` borrow is copied four times; the `Option<Ordering>`, `Ordering`, `Placement`, `Dominance`, and `Precedence` return-code tables are each spelled two or three times; `decimal_digest` is `ShapeDigest` over a string's bytes with the same two literal constants (and the harness holds a third copy it must keep in step); and `ff_clock_encode`, `ff_clock_display`, `ff_clock_own_version`, and `ff_clock_version` re-inline the match `with_c` performs. Every new kernel is written by copying a neighbor, and the `with_c` bypasses show the copies already disagree on which spelling is current. Legibility of a 2000-line instrument.

Evidence:

    369	pub extern "C" fn ff_clock_encode(src: u32) -> i32 {
    370	    REGS.with_borrow(|regs| match regs.get(src as usize) {
    371	        Some(Some(Val::C(c))) => {
    ...
    685	fn decimal_digest(text: &str) -> i64 {
    686	    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    ...
    700	    fn new() -> Self {
    701	        ShapeDigest(0xcbf2_9ce4_8422_2325)

Resolution: A `Slot` trait (`from_val`, `as_ref`, `as_mut`) implemented for the six slot types gives one generic `take::<T>`, `with::<T>`, `with_mut::<T>`, `take_range::<T>`, and `with_two_mut::<A, B>`; name the return-code tables as functions (`ordering_code`, `placement_code`, `dominance_code`, `precedence_code`) with the encoding in their doc; delete `decimal_digest` in favor of `ShapeDigest` fed the text's bytes, and name `FNV_OFFSET`, `FNV_PRIME`, `NONNEGATIVE_MASK`; route the four clock kernels through `with_c`. Acceptance: `split_at_mut` appears once; `Some(Some(Val::C(c)))` appears only in the helpers; each verdict encoding is spelled once; `cbf2_9ce4` appears once in the guest; `just fuzzfit` bands unchanged, since nothing measured changes.

Synthesis note: fuzzfit-strategies-14 (this document) is the harness side of the `decimal_digest` and return-code duplication.

#### fuzz-guests-pins-19: `ff_reset` has no caller
- Where: crates/before/fuzzfit/guest/src/lib.rs:261-267 (related: crates/before/fuzzfit/harness/src/wasm.rs:1-11)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn ff_reset crates/` hits only the definition; wasm.rs:4-6 states a fresh `Guest` per program as the determinism model); executed: no
- Seen by: scaffolding [15]; refutation: confirmed; history: no-rationale-found (born with the guest in 8cfd3c929, never named by any consumer in any revision)
- Owner-gated: no

The harness's determinism model is a fresh guest per program, which makes an in-place reset redundant; an export nothing drives exists only for itself and suggests a reuse mode the harness deliberately does not have.

Evidence:

    261	/// Clear the register file and the staging buffer.
    262	#[no_mangle]
    263	pub extern "C" fn ff_reset() -> i32 {
    264	    REGS.with_borrow_mut(Vec::clear);
    265	    STAGE.with_borrow_mut(Vec::clear);
    266	    OK
    267	}

Resolution: Delete it. Acceptance: `grep -rn ff_reset crates` returns nothing.

#### fuzz-guests-pins-31: The wasm32 guest's failure codes are bare negative literals, and the harness says the guest's docs key them
- Where: crates/before/wasm32-pins/harness/src/lib.rs:29-30 (related: crates/before/wasm32-pins/guest/src/lib.rs:66-116, crates/before/wasm32-pins/guest/src/lib.rs:189-210, crates/before/wasm32-pins/guest/src/lib.rs:648-660, crates/before/fuzzfit/guest/src/lib.rs:98-105)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (no guest doc enumerates a code; 54 `return -N` sites by grep; the same code means different checks per export, so -2 is "bytes differ" at 112-113 and "r <= ZERO" at 196-197; the fuzz-fit guest names `OK`, `ERR_REG`, `ERR_OP`, `ERR_CODEC`); executed: no
- Seen by: scaffolding [12]; refutation: confirmed; history: no-rationale-found (false from the leg's first commit)
- Owner-gated: no

A failing pin reports `Value(-3)` and the reader must open the guest source to learn which check failed; the harness doc's claim is false as written. Named constants over magic numbers.

Evidence:

    29	    /// The export returned: nonnegative is its observation, negative names
    30	    /// the first failed in-guest check (the guest's doc comments key them).

Resolution: Introduce named codes in the guest (`DECODE_REJECTED`, `BYTES_DIFFER`, `LENGTH_UNADDRESSABLE`, and so on) shared across exports, and either re-export them for the pins to assert on or correct the harness doc to say the guest's code names key them. Acceptance: `grep -E 'return -[0-9]+' crates/before/wasm32-pins/guest/src/lib.rs` is empty; the harness doc sentence is true.

#### fuzz-guests-pins-34: `BUILD_CAP_BYTES` and the `*_build_cap` test-name family name a cap two commits removed; three pins spell its coordinate as literals
- Where: crates/before/wasm32-pins/harness/tests/pins.rs:14-25 (related: crates/before/wasm32-pins/harness/tests/pins.rs:44-79, the `*_build_cap` tests at pins.rs:45, 59, 74, 312, 328, 339, 388, 404, 415, 440, 451, 462, 485, 497, 507, 545, 562, 574, 591, 609, crates/before/Cargo.toml:33-36)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git log -S'BUILD_CAP_BYTES'` names 5d167a63 and 83e61b4d; `git show 83e61b4d^:.../pins.rs` lines 14-23 read "one byte under the 32-bit bit-vector length encoding's cap" and "the emitters' build buffer is the one surface still bounded by this encoding"; 83e61b4d, "replace the bitvec build buffer with the crate-owned BitsBuf", rewrote that doc to the hypothetical while keeping the identifier and the test names; `git blame` puts the literals at 47-48, 61-62, 76-77 in eb6ba627, before the constant existed; `grep -c 'fn .*_build_cap'` is 20; crates/before/Cargo.toml:34-35 confirms nothing shipped links bitvec); executed: no
- Seen by: scaffolding [9]; adequacy [34]; structure-prose [37],[57]; instrument-correctness [70]; refutation: confirmed, severity low (a rename with byte-identical assertions; the prose was already re-denominated); history: deliberate-but-expired (83e61b4d)
- Owner-gated: no

No cap exists (the doc's own second paragraph says "Every surface is exact across it"), yet the constant's name and twenty test names say `build_cap`, and the doc speaks in the present tense of "the cap a 32-bit bit-vector length encoding imposes". A reader of `version_decode_at_build_cap` looks for a build cap and finds none, and the constant is named for the last size below the coordinate, so the `*_at_build_cap` tests run at `BUILD_CAP_BYTES + 1`. Separately, the version-decode straddle spells `67_108_863`, `67_108_864`, `67_108_865` and `8 * 67_108_86x - 8` as literals while every later straddle uses the constant. Rider: five asserts (356, 427, 474, 523, 534) carry a trailing `Outcome::Value(0),)` the others do not. Prose speaks in the present tense; named constants over magic numbers.

Evidence:

    14	/// The largest buffer byte count whose whole-buffer bit count stays below
    15	/// 2^29 bits — `usize::MAX >> 3`, the cap a 32-bit bit-vector length
    16	/// encoding imposes — so 67108863 bytes.
    ...
    25	const BUILD_CAP_BYTES: u64 = 67_108_863;
    45	fn version_decode_below_build_cap() {
    46	    assert_eq!(
    47	        call1("pin_version_decode", 67_108_863),
    48	        Outcome::Value(8 * 67_108_863 - 8),

Resolution: Rename the constant for the coordinate it is (for example `STRADDLE_COORDINATE_BYTES` for 67_108_864, so `at` reads as the coordinate itself and `below`/`past` as `- 1`/`+ 1`), rename the test family `*_below_straddle`/`*_at_straddle`/`*_past_straddle`, restate lines 14-16 as a present-tense definition (the byte count at which a buffer's bit count reaches 2^29, where a `usize`-denominated 32-bit bit count would bind), use the constant at 47-48, 61-62, 76-77 with a `fn live_bits(n: u64) -> i64 { 8 * n - 8 }` helper, and drop the five trailing commas. Acceptance: `grep -rn -i 'build_cap\|build cap' crates/before/wasm32-pins` returns nothing; `grep -c '67_108_86' pins.rs` is 1; the pins' assertions are byte-identical before and after.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| fuzz-guests-pins-5 | `crates/before/fuzz/fuzz_targets/fuzz_decode.rs:31-86` | The six-type wire roster is spelled three times across the decode targets | One `round_trip!` macro; one roster list for both differentials |
| fuzz-guests-pins-13 | `crates/before/fuzz/fuzz_targets/fuzz_parse.rs:53-63` | `fuzz_parse` compares the `Clock` round-trip by `encode()` while its siblings compare by `==` | `assert_eq!(again, clock, ..)`, or a comment saying why not |
| fuzz-guests-pins-22 | `crates/before/fuzzfit/guest/src/lib.rs:791-831` | `COMBINE_ARITY_CAP` and the `dispatch!` arm list are hand-parallel | Derive the arm list from the cap, or tie them with a `const _` assert |
| fuzz-guests-pins-28 | `crates/before/wasm32-pins/guest/src/lib.rs:102-106` | Repeated prologue and epilogue fragments in the wasm32 guest; a slice-dispatch `unreachable!` in the harness | `addressable` and `rank_observations` helpers; a generic `call<P: WasmParams>` |

### Fuzz-fit: bands

8 entries (3 low, 5 nit); the full record is `evidence/partitions/fuzzfit-bands.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: fuzzfit-bands-18. Related findings in other documents: fuzzfit-bands-2 (documentation: pin-time evidence prose), fuzzfit-bands-5 and fuzzfit-bands-19 (verification-gap: the per-key slack and the determinism premise).

#### fuzzfit-bands-11: Four shared computations are each spelled twice: bucket medians and their thresholds, the deterministic stream loop, and the `Fit`-to-`Band` transcription
- Where: crates/before/fuzzfit/harness/src/curve.rs:100-106 (related: crates/before/fuzzfit/harness/src/fit.rs:86-100, fit.rs:138-147, curve.rs:51-59, crates/before/fuzzfit/harness/src/bin/diag.rs:33-53, crates/before/fuzzfit/harness/src/drive.rs:115-131, crates/before/fuzzfit/harness/src/fit/tests.rs:10-24, crates/before/fuzzfit/harness/src/bin/calibrate.rs:45-59, calibrate.rs:139-155, calibrate.rs:186-202, crates/before/fuzzfit/harness/src/bands.rs:133-159)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read both median expressions side by side; `MIN_BUCKETS`/`MIN_DECADES` declared at fit.rs:88,100 private and curve.rs:52,59 public with equal values; diag.rs:33-46 and drive.rs:119-128 differ only in the family filter and the accumulation target; the two `writeln!` templates differ only in the hard-coded `rejected: false`); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed (nuance: diag's own loop skips `build`/`run_program` for filtered-out families); history: no rationale found (3d11ecc3 unified `BUCKETS_PER_DECADE` only; the rest are copies)
- Owner-gated: no

`curve.rs` re-implements `fit.rs`'s half-decade bucketing and per-bucket median statistic and re-declares `MIN_BUCKETS` and `MIN_DECADES`; `diag.rs` re-implements `drive::for_each_deterministic_program`, so its "same corpus as `calibrate`" holds by copy, not by construction; `Fit` and `Band` share eight fields transcribed by two identical `band_of` helpers and two byte-identical source templates in `calibrate`. fit.rs:92-96 already makes the argument for one exported constant ("the diagnostics must bucket exactly the way the fit does, or their medians stop describing the fit's inputs"); the same argument applies to the median itself and to the thresholds. Legibility and one source of truth.

Evidence:

       100	    let median = |pts: &mut Vec<(f64, f64)>| {
       101	        pts.sort_by(|a, b| a.1.total_cmp(&b.1));
       102	        let y = pts[pts.len() / 2].1;
       103	        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
       104	        let x = pts[pts.len() / 2].0;
       105	        (x, y)
       106	    };

    the same statistic in fit.rs:

       140	        .map(|pts| {
       141	            pts.sort_by(|a, b| a.1.total_cmp(&b.1));
       142	            let my = pts[pts.len() / 2].1;
       143	            pts.sort_by(|a, b| a.0.total_cmp(&b.0));
       144	            let mx = pts[pts.len() / 2].0;
       145	            (mx, my)
       146	        })

    the twin `band_of` helpers (calibrate.rs:45-47, fit/tests.rs:10-12):

        45	/// A [`Band`] transcribing a fresh [`Fit`] (what the pin will say).
        46	fn band_of(kernel: &'static str, rejected: bool, f: &Fit) -> Band {

        10	/// A band transcribing `f` exactly (the agreeing pin).
        11	fn band_of(f: &Fit) -> Band {

Resolution: One `pub fn bucket_medians(samples) -> Vec<(f64, f64)>` (and a bucket-key helper) in `fit.rs`, consumed by `curve.rs` after its thin-bucket filter and by `diag.rs` for the key; `MIN_BUCKETS` and `MIN_DECADES` declared once (`fit.rs`, public) and imported by `curve.rs`. Expose the deterministic stream as an iterator or add a family predicate to `for_each_deterministic_program` so `diag` consumes it (keeping its skip of filtered-out families). Compose `Band { kernel, rejected, fit: Fit }` (or a `Band::of(kernel, rejected, Fit)` constructor), delete both `band_of`s, and factor the two `writeln!` bodies into one `fn band_source(&Band) -> String`. Acceptance: one `total_cmp` median expression and one `BUCKETS_PER_DECADE).floor()` expression in the crate; `grep -rn 'TestRunner::deterministic' harness/src` returns only drive.rs; `grep -c 'slope: {:.6}' calibrate.rs` is 1; `just fuzzfit-calibrate` on unchanged code produces byte-identical data modulo any deliberate layout change.

#### fuzzfit-bands-15: `Guest::call_i64` and `Op::returns_i64` form a second call path the untyped `call` already covers
- Where: crates/before/fuzzfit/harness/src/wasm.rs:237-252 (related: wasm.rs:206-210, crates/before/fuzzfit/harness/src/drive.rs:47-51, crates/before/fuzzfit/harness/src/ops.rs:251-254, crates/before-fuelscape/src/ops.rs:377, 390, 559, 693, 884, 1179)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (wasmtime-47.0.4 `src/runtime/func.rs:1151-1157`: `call_impl_check_args` bails only when `ty.results().len() != results.len()`; `func.rs:1200-1202` writes each result slot by the function's declared type, `Val::from_raw(&mut *store, *val, ty)`; grep of before-fuelscape/src/ops.rs for the six `call_i64` sites); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: no rationale found (both paths born in 8cfd3c92; "Typed convenience" is the only stated reason)
- Owner-gated: no

wasmtime's untyped `Func::call` requires only that the results slice length equal the function's result count and writes each result by the declared type, so `Guest::call` with `results = [Val::I32(0)]` already returns i64 kernels correctly through its `Val::I64(v) => v` arm. `call_i64`, `Op::returns_i64`, and the branch in `drive.rs` are a parallel path for the same kernels; `call_i64` also indexes `args[0]` unconditionally. Legibility: one call path per kernel keeps the driver obviously correct, and the `returns_i64` predicate is a per-variant fact the driver keeps in sync with the guest ABI by hand. Fuel is consumed by guest instructions only, so the host-side call path cannot move a reading.

Evidence:

       237	    /// Typed convenience for i64-returning kernels (`ff_version_min_ticks`).
       238	    pub fn call_i64(&mut self, name: &str, args: &[u32]) -> Measured {
       239	        let func: TypedFunc<u32, i64> = self
       240	            .instance
       241	            .get_typed_func(&mut self.store, name)
       242	            .unwrap_or_else(|e| panic!("guest kernel {name}: {e}"));
       243	        self.store.set_fuel(FUEL_TANK).expect("fuel is enabled");
       244	        let ret = func
       245	            .call(&mut self.store, args[0])

    while `call` already reads:

       206	        let ret = match results[0] {
       207	            Val::I32(v) => v as i64,
       208	            Val::I64(v) => v,
       209	            ref other => panic!("guest kernel {name} returned unexpected type {other:?}"),
       210	        };

Resolution: Delete `call_i64` and `Op::returns_i64`; make drive.rs:47-51 a single `guest.call(op.kernel(), &args)`; switch the six fuelscape call sites to `call`. Acceptance: both detached workspaces build; `just fuzzfit` and `just fuelscape-test` pass with the bands untouched, and `just fuzzfit-calibrate` on the changed code produces no diff (fuel cannot move, and the empty diff is the check).

Synthesis note: fuzzfit-strategies-2 (this document) is the drive.rs side of the same second call path; this entry adds the six fuelscape call sites. One change closes both.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| fuzzfit-bands-14 | `crates/before/fuzzfit/harness/src/wasm.rs:131-135` | Em-dashes in `//` comments and assert strings; past-tense incident narration at two declaration sites | `--` or colons at eight sites; reword two past-tense comments positively |
| fuzzfit-bands-22 | `crates/before/fuzzfit/harness/tests/enforce.rs:56-66` | The band key is a bare `(&str, bool)` tuple with the `" [err]"` rendering repeated eleven times | A `BandKey` struct with `Display`; one `violation` fn |
| fuzzfit-bands-23 | `crates/before/fuzzfit/harness/tests/enforce.rs:158-164` | Two dead guards: `bands_are_pinned` is subsumed by the roster parity test, and the small-band `BelowFloor` arm is unreachable | Delete `bands_are_pinned`; make `BelowFloor` unreachable or unrepresentable |
| fuzzfit-bands-25 | `crates/before/fuzzfit/harness/tests/enforce.rs:369-371` | Idiom nits: qualified paths beside imports, magic numbers, an expect message that asserts rather than names | Imports; `10f64.powf(1.0 / BUCKETS_PER_DECADE)`; named `NOP_CEILING` and `PROGRESS_EVERY` |
| fuzzfit-bands-28 | `crates/before/fuzzfit/harness/tests/sanity.rs:1-3` | The pure `judge_against` tripwire lives in an integration file titled "Generator sanity"; `bands` has no sibling tests | Move the pure tripwire to `src/bands/tests.rs`; retitle sanity.rs |

### Fuzz-fit: strategies

9 entries (1 medium, 7 low, 1 nit); the full record is `evidence/partitions/fuzzfit-strategies.md`. Related findings in other documents: fuzzfit-strategies-7 (verification-gap: 56 measured kernels with no `Op`).

#### fuzzfit-strategies-19: The builder's `Ty`/`slots` liveness model is write-only
- Where: crates/before/fuzzfit/harness/src/strategies.rs:358-368 (related: crates/before/fuzzfit/harness/src/strategies.rs:337-347, crates/before/fuzzfit/harness/src/strategies.rs:382-385, crates/before/fuzzfit/harness/src/strategies.rs:471, crates/before/fuzzfit/harness/src/strategies.rs:532, crates/before/fuzzfit/harness/src/strategies.rs:547-548, crates/before/fuzzfit/harness/src/strategies.rs:569, crates/before/fuzzfit/harness/src/strategies.rs:611-614, crates/before/fuzzfit/harness/src/strategies.rs:635, crates/before/fuzzfit/harness/src/strategies.rs:645, crates/before/fuzzfit/harness/src/strategies.rs:682, crates/before/fuzzfit/harness/src/strategies.rs:848, crates/before/fuzzfit/harness/src/strategies.rs:857, crates/before/fuzzfit/harness/src/strategies.rs:865, crates/before/fuzzfit/harness/src/strategies.rs:1042, crates/before/fuzzfit/harness/src/strategies.rs:1122, crates/before/fuzzfit/harness/src/strategies.rs:1841, crates/before/fuzzfit/harness/src/strategies.rs:1936, crates/before/fuzzfit/harness/tests/sanity.rs:41-43)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (`grep -n slots` lists 24 sites: the declaration, the constructor, `push` at 383, `.len()` at 384, 594, 678, and 17 `= Ty::Dead` assignments; a grep for any other use of a `Ty` value outside `alloc(Ty::...)` (32 sites) matches nothing; every access to the field contains the token `slots`, so the list is exhaustive); executed: yes: the two greps settle that no code path reads a slot's type
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed, severity medium to low (no instrument is weakened; the mirror plus `sanity::programs_are_well_formed` own liveness); history: no-rationale-found (no version of strategies.rs in its eleven commits ever read a slot's type; 319f9c53 centralized "slot liveness" bookkeeping as if something depended on it)
- Owner-gated: no

`B::slots: Vec<Ty>` is written at every emitter and read only through `.len()` to allocate the next register; no expression matches on `Ty::V | P | C | R` or consults `slots[i]` before emitting an op, so the builder does not model liveness, and the struct doc, the `Ty::Dead` doc, and sanity.rs's test doc ("the builder's liveness model matches real consumption") all describe a check the mirror alone performs (Principle 3: infrastructure earns its place by naming what it catches; a write-only type record catches nothing, costs an enum plus seventeen bookkeeping lines a maintainer must keep for no reader, and its documentation invites trusting a check that does not exist). I hold this at medium against the refutation's low because the cost is spread across every emitter in a 2000-line generator and the false claim sits at the site a maintainer adding a family reads first; the dissolution is mechanical and the corpus byte-identical.

Evidence:

       358	/// The type-tracked program builder: emits ops, models liveness the way the
       359	/// mirror will, and enforces the budget unconditionally (an out-of-budget
       360	/// request is skipped, so any parameter draw stays within [`BUDGET`]).
       361	struct B {
       362	    ops: Vec<Op>,
       363	    slots: Vec<Ty>,

       344	    /// Consumed, or of uncertain liveness after a possibly-rejecting op;
       345	    /// never used again either way.
       346	    Dead,

    sanity.rs:41	    /// Every generated program is well-formed: the native mirror executes
    sanity.rs:42	    /// it end to end without a register-file violation, i.e. the builder's
    sanity.rs:43	    /// liveness model matches real consumption (linearity by construction).

Resolution: replace `slots: Vec<Ty>` with a `next_reg: Reg` counter, delete `enum Ty`, make `alloc()` take no argument, remove every `self.slots[..] = Ty::Dead` line (keeping the informative comments such as `// may underflow` where they explain why a destination is not pooled), and rewrite the struct doc to "emits ops and enforces the budget unconditionally; the mirror owns well-formedness (`programs_are_well_formed`)"; fix sanity.rs:41-43 to state what the test checks. (The alternative, making the model live with a slot-type assertion in each emitter, samples no space `programs_are_well_formed` cannot, so dissolution is the doctrinal answer.) Acceptance: `grep -n 'Ty\b\|slots' strategies.rs` returns only the counter; `generation_is_deterministic` and `programs_are_well_formed` stay green with the deterministic corpus byte-identical (register numbering unchanged); no doc names a builder liveness model.

#### fuzzfit-strategies-2: `Op::returns_i64` and the `call_i64` branch are a second call path `Guest::call` already covers
- Where: crates/before/fuzzfit/harness/src/drive.rs:47-51 (related: crates/before/fuzzfit/harness/src/ops.rs:251-254, crates/before/fuzzfit/harness/src/wasm.rs:194-252, crates/before-fuelscape/src/ops.rs:404)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read `Guest::call`'s result match at wasm.rs:206-209, `call_i64`'s `args[0]` at wasm.rs:245, and fuelscape's untyped call of the two-argument i64 kernel `ff_shape_combine` at ops.rs:404; the guest declares `ff_shape_combine(src: u32, n: u32) -> i64` at guest/src/lib.rs:797); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no-rationale-found (both paths are coeval originals; `returns_i64` was added with the one-word doc "Typed convenience")
- Owner-gated: no

`Guest::call` already returns i64 results (it matches `Val::I64`, and fuelscape drives an i64-returning two-argument kernel through it), so `returns_i64` and the branch exist only to route one kernel through `TypedFunc<u32, i64>`, which reads `args[0]` and would drop further arguments for any future multi-argument i64 op (Principle 3: two paths for one capability, the narrower one carrying a latent arity trap).

Evidence:

        47	        let measured = if op.returns_i64() {
        48	            guest.call_i64(op.kernel(), &args)
        49	        } else {
        50	            guest.call(op.kernel(), &args)
        51	        };

    wasm.rs:206	        let ret = match results[0] {
    wasm.rs:207	            Val::I32(v) => v as i64,
    wasm.rs:208	            Val::I64(v) => v,

    wasm.rs:245	            .call(&mut self.store, args[0])

Resolution: delete `Op::returns_i64` (ops.rs:251-254) and the branch; always `guest.call`. Retiring `wasm::Guest::call_i64` itself belongs to the wasm.rs partition (fuelscape calls it at six sites). Acceptance: drive.rs has one call site and the `min_ticks` differential (`expect == decimal_digest`) still passes in `just fuzzfit`.

Synthesis note: fuzzfit-bands-15 (this document) retires `Guest::call_i64` itself and lists the fuelscape callers.

#### fuzzfit-strategies-4: The driver's snapshot table restates `Op::kernel` through `u8` tags and an `unreachable!`
- Where: crates/before/fuzzfit/harness/src/drive.rs:83-89 (related: crates/before/fuzzfit/harness/src/ops.rs:407-423, crates/before/fuzzfit/harness/tests/sanity.rs:55-75)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (compared the four strings against `Op::kernel` at ops.rs:167, 181, 192, 199; read the identical tag match in sanity.rs:55-75); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found (7fb3b5ce original)
- Owner-gated: no

`Mirror::live_regs` returns `(Reg, u8)` with `b'v'|b'p'|b'c'|b'r'` tags, and the driver maps each tag to a kernel string that `Op::kernel` already owns (the four snapshot kernels are exactly `Op::VersionEncode`, `Op::PartyEncode`, `Op::ClockEncode`, `Op::RankDisplay`); sanity.rs repeats the same match with the same `unreachable!` (types-first: the compiler, not a comment, should exclude the fifth tag, and kernel names belong to the vocabulary module, so a second spelling here is a rename hazard the differential catches only at runtime).

Evidence:

        83	        let kernel = match tag {
        84	            b'v' => "ff_version_encode",
        85	            b'p' => "ff_party_encode",
        86	            b'c' => "ff_clock_encode",
        87	            b'r' => "ff_rank_display",
        88	            _ => unreachable!("mirror tags are v/p/c/r"),
        89	        };

    ops.rs:407	    /// Registers currently live, tagged `'v' | 'p' | 'c' | 'r'`.
    ops.rs:408	    pub fn live_regs(&self) -> Vec<(Reg, u8)> {

Resolution: in ops.rs add `pub enum Kind { Version, Party, Clock, Rank }` with `fn snapshot_op(self, reg: Reg) -> Op` (the three `*Encode` ops and `RankDisplay`); `live_regs() -> Vec<(Reg, Kind)>`; the driver becomes `let op = kind.snapshot_op(reg); guest.call(op.kernel(), &op.args())`; sanity.rs matches on `Kind`. Acceptance: no `ff_` string literal in drive.rs; `grep -n unreachable! drive.rs tests/sanity.rs` is empty; `just fuzzfit` green.

#### fuzzfit-strategies-12: `Mirror::step` duplicates whole arms that differ by one method call
- Where: crates/before/fuzzfit/harness/src/ops.rs:697-736 (related: crates/before/fuzzfit/harness/src/ops.rs:458-477, crates/before/fuzzfit/harness/src/ops.rs:603-627, crates/before/fuzzfit/harness/src/ops.rs:668-688, crates/before/fuzzfit/harness/src/ops.rs:487-503, crates/before/fuzzfit/harness/src/ops.rs:790-806, crates/before/fuzzfit/harness/src/ops.rs:578, crates/before/fuzzfit/harness/src/ops.rs:744, crates/before/fuzzfit/harness/src/ops.rs:760, crates/before/fuzzfit/harness/src/ops.rs:841, crates/before/fuzzfit/harness/src/ops.rs:855)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read each arm pair); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`VersionJoinAll`/`VersionMeetAll` are twenty identical lines each except `join_all`/`meet_all`; `ClockTick`/`ClockSend` differ only in `tick()`/`send()`; `VersionJoin`/`VersionMeet` and `VersionDistance`/`VersionLag` likewise; `ClockJoin`/`PartyJoin` share the take-join-restore shape; and `(self.stage.len() as u64) * 8` appears five times (legibility: a denomination or linearity fix must be applied in two or three places, and a reviewer diffs arms by eye to confirm they still agree).

Evidence:

       697	            Op::VersionJoinAll { dst, src, n } => {
       698	                let mut denom = 0u64;
       699	                let mut operands = Vec::with_capacity(n as usize);

       717	            Op::VersionMeetAll { dst, src, n } => {
       718	                let mut denom = 0u64;
       719	                let mut operands = Vec::with_capacity(n as usize);

Resolution: extract small helpers on `Mirror` (`take_versions(src, n) -> Result<(u64, Vec<Version>), Malformed>` for the folds; `with_clock(c, f)` for tick/send/fork; a `version_pair_bits(a, b)`; `stage_bits()`), keeping one arm per `Op` so the exhaustive match still documents the ABI. Acceptance: `Mirror::step` roughly halves with no arm losing its comment; `just fuzzfit` green; the deterministic stream unchanged.

#### fuzzfit-strategies-13: Two `as u64` casts on `encoded_bits()` survived the u64 denomination migration
- Where: crates/before/fuzzfit/harness/src/ops.rs:764-764 (related: crates/before/fuzzfit/harness/src/ops.rs:858, crates/before/src/version.rs:1150-1153, crates/before/src/party.rs:596-600, justfile:594-597, .github/workflows/ci.yml:117-120)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`encoded_bits` returns `u64` at version.rs:1151, party.rs:597, clock.rs:852; `git show 05d87e1b -- ops.rs` removes 20 `encoded_bits() as u64` casts and leaves these two; no commit has touched ops.rs since); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed; history: deliberate-but-expired (width-changing usize-to-u64 conversions until 05d87e1b, 2026-08-19)
- Owner-gated: no

The two casts convert `u64` to `u64`; the other same-shaped casts in the file (`(len as u64) * 8`) change width, so these two read as if `encoded_bits` were narrower (Principle 5: code states what IS). Whether clippy's `unnecessary_cast` reads red on the `just fuzzfit` lint leg (`cargo clippy --all-targets -- -D warnings`, justfile:596) is unsettled: the lint ordinarily fires on a same-type cast of a non-literal expression, but 4e64a4fb (2026-08-31) reports `just gate clean (398s, all legs)` with these casts present and the gate's wasm stream runs that leg (justfile:467); CI does not run it (ci.yml:117-120). I was not permitted to run clippy; see the open questions.

Evidence:

       764	                let denom = text_bits + version.encoded_bits() as u64;

       858	                let denom = text_bits + party.encoded_bits() as u64;

    version.rs:1150	    #[cfg(any(test, feature = "meter"))]
    version.rs:1151	    pub fn encoded_bits(&self) -> u64 {

Resolution: drop both casts and run the fuzzfit recipe's clippy line. Acceptance: `grep -n 'encoded_bits() as u64' ops.rs` is empty and `cd crates/before/fuzzfit && cargo clippy --all-targets -- -D warnings` is clean.

#### fuzzfit-strategies-14: `decimal_digest` and the ABI return codes are duplicated by hand between guest and harness
- Where: crates/before/fuzzfit/harness/src/ops.rs:910-920 (related: crates/before/fuzzfit/guest/src/lib.rs:680-692, crates/before/fuzzfit/guest/src/lib.rs:98-105, crates/before/fuzzfit/harness/src/ops.rs:322-325, crates/before/fuzzfit/harness/src/ops.rs:644-649, crates/before/fuzzfit/guest/src/lib.rs:582-587)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (read both digest bodies, byte-identical; `ERR_OP = -2` at ops.rs:325 and guest:103; the cmp codes 0/1/2/3 at ops.rs:645-648 and guest:583-586); executed: no
- Seen by: scaffolding; refutation: confirmed (the guest is a cdylib depending only on `before`, so a `#[path]`-shared module is feasible); history: no-rationale-found (68b60f06 added both bodies in one commit with "computed identically on both sides" as the only binding)
- Owner-gated: no

The digest, `ERR_OP`/`OK`, and the comparison encodings are transcribed by hand on both sides, bound only by prose ("computed identically here") and by the runtime differential on the first `min_ticks` step; the hole is defended, but by a convention plus a late runtime failure rather than one definition (prefer a dependency over hand-rolled parity).

Evidence:

       910	/// A nonnegative FNV-1a digest of a decimal rendering: the `i64`
       911	/// channel's encoding for unbounded counts, mirrored in the guest's
       912	/// `ff_version_min_ticks`.
       913	fn decimal_digest(text: &str) -> i64 {

    guest lib.rs:680	/// A nonnegative FNV-1a digest of a decimal rendering.
    guest lib.rs:685	fn decimal_digest(text: &str) -> i64 {

Resolution: factor the return codes, the digest, and ideally the kernel-name strings into one shared module both crates include (`#[path]` from a sibling `abi.rs`, or a dependency-free `fuzzfit-abi` crate); drop the mirrored-by-hand prose. Acceptance: one definition of `decimal_digest` and `ERR_OP` in the workspace; the `min_ticks` differential still passes.

Synthesis note: fuzz-guests-pins-18 (this document) names the guest-side copies of the same digest and return codes; a shared `abi.rs` closes both.

#### fuzzfit-strategies-21: Guards that can never fire in the gate: three in `fork_balanced`, `any_program`'s non-empty filter, and `B::push`'s release-compiled-out `debug_assert`
- Where: crates/before/fuzzfit/harness/src/strategies.rs:439-465 (related: crates/before/fuzzfit/harness/src/strategies.rs:391-394, crates/before/fuzzfit/harness/src/strategies.rs:1965-1971, crates/before/fuzzfit/harness/src/strategies.rs:1754-1760, crates/before/fuzzfit/Cargo.toml:23-25, justfile:575-577, justfile:597, crates/before/fuzzfit/harness/tests/sanity.rs:16-39)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (executed: a Python replay of `fork_balanced` as written against a stripped variant (first disjunct only; no `next.len() == pool.len()` return; no trailing break; `truncate` kept), over `n` in 0..300 and every fork-budget cutoff 0..39 plus unbounded, at `<scratchpad>/before/final:fuzzfit-strategies/fork_balanced_replay.py`: output "output diffs: 0 | disjunct decisive: 0 | eqlen taken: 0"; the structure-prose lens and the refutation pass each ran an independent replay with the same result; the filter and the assert were settled by reading: every `construct` arm opens with `b.clock_seed()`, which succeeds when `max_ops >= 1`, `Independent` clamps to at least two universes, the release profile leaves debug assertions off, and `just fuzzfit` runs `cargo nextest run --cargo-profile release`); executed: yes: the replay above
- Seen by: structure-prose, scaffolding, adequacy; refutation: confirmed and reframed (`pool.truncate(n)` at 463 is live for `n = 0`, which turns `[src]` into `[]`; no roster draw passes 0, but `build` is public, so keep it); history: no-rationale-found (all 7fb3b5ce originals; the suite has run under the release profile since the first recipe)
- Owner-gated: no

In `fork_balanced`, the disjunct `pool.len() * 2 <= n as usize` (445) is never decisive (when it holds, `next.len() <= 2·pool.len() − 1 < n`, so the first disjunct already holds), the `next.len() == pool.len()` return (455-457) is unreachable (the first element of every pass forks or returns on budget exhaustion), and the trailing `if pool.len() as u32 >= n { break; }` (459-461) restates the `while` condition; the interleaved parent/child order is semantic (the comb families alternate on index parity) and the stripped loop preserves it. `any_program`'s `prop_filter("non-empty program")` never rejects. `B::push`'s `debug_assert!` is compiled out in every gate execution and the invariant it guards is pinned in release by `programs_respect_the_budget` (legibility: conditions that look necessary and are not force every reader to re-derive the invariant; Principle 3: a guard must sample a space the committed tests cannot).

Evidence:

       445	                if (next.len() as u32) < n || pool.len() * 2 <= n as usize {

       455	            if next.len() == pool.len() {
       456	                return pool;
       457	            }

       459	            if pool.len() as u32 >= n {
       460	                break;
       461	            }

       392	        debug_assert!(self.room(), "callers check room() before emitting");

      1970	        .prop_filter("non-empty program", |p| !p.is_empty())

Resolution: reduce `fork_balanced` to one interleaving pass per doubling with the single `next.len() < n` guard, keeping `truncate` (or give `n = 0` an explicit early return) and stating the doubling invariant in the doc comment; delete the `prop_filter`; delete the `debug_assert` or promote it to `assert!` if the owner wants the emission site named on a breach. Acceptance: `generation_is_deterministic` and `programs_respect_the_budget` green with the deterministic corpus byte-identical (`for_each_deterministic_program` yields the same programs; the refit staleness check confirms since the stream is the same); `grep -n prop_filter strategies.rs` empty.

#### fuzzfit-strategies-22: `construct` is an 840-line match with a partial domain and five repeated epilogues
- Where: crates/before/fuzzfit/harness/src/strategies.rs:885-887 (related: crates/before/fuzzfit/harness/src/strategies.rs:932-941, crates/before/fuzzfit/harness/src/strategies.rs:1070-1082, crates/before/fuzzfit/harness/src/strategies.rs:1142-1155, crates/before/fuzzfit/harness/src/strategies.rs:1168-1183, crates/before/fuzzfit/harness/src/strategies.rs:1195-1207, crates/before/fuzzfit/harness/src/strategies.rs:1226-1238, crates/before/fuzzfit/harness/src/strategies.rs:1249-1261, crates/before/fuzzfit/harness/src/strategies.rs:1504-1717, crates/before/fuzzfit/harness/src/strategies.rs:1718-1720, crates/before/fuzzfit/harness/src/strategies.rs:1754-1762)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (read the five epilogues and the two join chains; the `unreachable!` at 1719 guards a variant `build` expands at 1757 before dispatch); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`construct` spans 885-1723 and the `Escalation` arm alone is 214 lines; five spine families end with the same epilogue (push seed; `version_of(cur)` into versions, sometimes with a rank; if `cur != seed`, `split_parts(cur)` into parties and versions, sometimes with a cross tick), `RevealComb` and `PureComb` repeat the adjacent join chain verbatim, and the match carries an `unreachable!` because `Family` conflates the coupled families with `Independent`, which `build` expands and `reduced_family` never returns (modules and functions have a single clear responsibility; the repeated epilogues are where a family-specific pooling mistake would hide, and the `unreachable!` is a type admitting a value the function refuses).

Evidence:

       885	fn construct(b: &mut B, family: &Family) -> Pools {
       886	    let mut pools = Pools::default();
       887	    match *family {

      1718	        Family::Independent { .. } => {
      1719	            unreachable!("Independent is expanded by build(), not construct()")
      1720	        }

Resolution: one function per family with `construct` reduced to dispatch; a shared `spine_epilogue(b, pools, seed, cur, rank: bool, cross_tick: bool)` and `join_chain(b, shares) -> Option<Reg>`; consider a `Coupled` sub-enum returned by `reduced_family` and taken by `construct`, with `Family::Independent { .. }` and `Family::Coupled(Coupled)` at the top level so the `unreachable!` dissolves; each family function then has a rustdoc home for the construction comments now attached to match arms. Acceptance: `construct` or its replacement fits on a screen; `grep -c unreachable! strategies.rs` is 0; `generation_is_deterministic` and the enforce staleness cross-check confirm the emitted programs are unchanged.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| fuzzfit-strategies-9 | `crates/before/fuzzfit/harness/src/ops.rs:273-276` | Idiom nits across `ops.rs` and `strategies.rs` | Apply the listed renames and named selectors; `Builder` for `B` |

### Fuelscape: pipeline

12 entries (1 medium, 7 low, 4 nit); the full record is `evidence/partitions/fuelscape-pipeline.md`. Related findings in other documents: fuelscape-pipeline-1 (claim: the overlay contract), fuelscape-pipeline-15 (documentation: the `CENSUS` literal).

#### fuelscape-pipeline-30: Two exemption rosters over one surface: 51 rows excused twice with independently worded reasons
- Where: crates/before-fuelscape/src/ops.rs:2173-2229 (related: crates/before-fuelscape/src/ops.rs:2160-2172, 2229-2509; crates/before/src/meter/board/coverage.rs:271-300; crates/before/src/surface.rs:181-194; crates/before-fuelscape/src/ops/tests.rs:18-54)
- Class / severity / confidence: modularity / medium / high
- Provenance: verified; executed: yes: a Python extraction of the first string of each tuple gives `EXEMPTIONS` 72 rows, `BOARD_NOT_APPLICABLE` 79, `BOARD_PRICED` 91; 51 rows are excused by both tables, 21 are atlas-exempt but board-priced, 28 board-excused but atlas-panelled
- Seen by: scaffolding [1]; refutation: confirmed; history: no-rationale-found: the two tables landed the same day from two instruments' commits (eb6b35f42, 669cf310), each describing only its own tiling; neither the validation index nor the coverage module's doc records a decision to hold the classification twice
- Owner-gated: yes: the fix reshapes `SurfaceRow` in `before` (crates/before/src/surface.rs:183-194), the roster of record shared by the board and the atlas

Fifty-one `before::surface` rows carry two separately maintained prose reasons for one fact, that an O(1) accessor or constructor has no size axis, and every new such item costs a surface row plus two exemption lines in two crates. Principle 3: a duplicated roster generating its own maintenance cascade (two reason tables drifting in wording for one mechanism). The 21 rows the atlas exempts but the board prices, and the 28 the board excuses but the atlas panels, are instrument-specific decisions and belong where they are; the 51 shared rows are a property of the surface row itself.

Evidence:

    (ops.rs)
      2196	    ("Party::as_bytes", "O(1) borrow of the stored packed bytes"),

    (crates/before/src/meter/board/coverage.rs)
       300	    ("Party::as_bytes", "a borrow of the stored canonical bytes"),

Resolution: Give `SurfaceRow` a size-axis classification (e.g. `axis: SizeAxis`, with `SizeAxis::None(&'static str)` carrying the one reviewed reason) and have both tilings treat `SizeAxis::None` rows as excused automatically; each instrument's table then shrinks to its own decisions (atlas: delegating wrappers priced at another panel; board: rows it excuses but the atlas panels). Acceptance: `EXEMPTIONS` and `BOARD_NOT_APPLICABLE` name no row whose `SurfaceRow` carries `SizeAxis::None`; both tiling tests pass; adding an O(1) accessor requires exactly one reason, on its surface row.

#### fuelscape-pipeline-2: Vocabulary and register sweep: "mint" for constructing values, "honest" and "real" for properties, em-dashes in line comments
- Where: crates/before-fuelscape/src/lib.rs:19 (related: plan.rs:19, 168; ops.rs:105, 185, 800, 2448, 2507 ("mint"); ops.rs:2168, sample/tests.rs:99 ("honest"); count.rs:32, enumerate.rs:7, sample.rs:26, count/tests.rs:114, 129, families.rs:41, sample/tests.rs:73, 88, 226, 247 ("real" for the shipping decoders); sample.rs:298, 438, ops.rs:856, 1745, 2088 (em-dashes in `//` comments))
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep over the fourteen partition files for `mint`, `\bhonest\b`, `\breal\b`, and `^\s*//[^/!].*—`); executed: no
- Seen by: scaffolding [11], adequacy [21], structure-prose [32], [38]; refutation: confirmed; history: the rules live in the owner's writing-style doctrine (`~/.claude/writing-style.md`:149-152, 170-175, 326-330), tracked from 2026-08-19, after most of these sites were written (2026-07-28..31), so this is sweep work rather than an authoring-time breach
- Owner-gated: no

The owner's vocabulary rule bans "mint" for constructing a value outright and asks that "real"/"honest" be replaced by the property that holds; the register rule reserves em-dashes for rendered prose and gives `//` comments the spaced double hyphen. Eight "mint" sites, two "honest", about ten "real" where count/tests.rs:172 already has the precise term ("the shipping decoder"), and five em-dashes in line comments. Severity low rather than nit because these are stated rules, not taste, and the sweep is one batch.

Evidence:

    (lib.rs)
        18	//! pipeline smoke test, nothing else: no fuel threshold, percentile gate,
        19	//! or band is ever minted from atlas data — the envelope suite and the

    (ops.rs)
      2448	        "O(1) hole mint over the atom's bound: no comparison, no walk",
      2168	/// forms have no honest size axis to plot, and delegating wrappers are

    (sample/tests.rs)
        99	/// `2(k - 1)`, so six standard deviations above the mean rejects honest

    (sample.rs)
       298	                // right subtree, its remainder the leaf mantissa — but the

Resolution: "derived" (lib.rs:19); "split in the guest" / "guest-split" (plan.rs:19, 168; ops.rs:105, 800); "builds" (ops.rs:185); "O(1) hole construction" (ops.rs:2448); "produce" (ops.rs:2507); "no size axis" (ops.rs:2168); "rejects a uniform draw" (sample/tests.rs:99); "the shipping decoder(s)" for "real" and rename `real` to `decoded` in sample/tests.rs:73, 88; swap the five line-comment em-dashes for ` -- ` or a colon. Acceptance: `grep -n -i -w 'mint\|minted\|mints\|honest' crates/before-fuelscape/src` returns nothing; `grep -n -E '^\s*//[^/!].*—'` over the partition returns nothing.

#### fuelscape-pipeline-4: draw_inputs copies the member-draw closure seven times; the two slice arms differ by one expression
- Where: crates/before-fuelscape/src/plan.rs:274-311 (related: crates/before-fuelscape/src/plan.rs:213-243, 312-352)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read); executed: no
- Seen by: scaffolding [8], structure-prose [26]; refutation: confirmed; history: the arms accreted by copying across three commits (eb6b35f42/c459b1d0, 98c63b26, 46eb64f9) with no discussion of shape
- Owner-gated: no

The version draw with `expect("every byte size down to 1 has canonical versions")` and the `rejected` accumulation appears at 226-231, 283-288, 302-307, and 330-335; the party draw at 234-238, 324-328, and 346-350; the `VersionSlice` and `VersionSliceCapped` arms are identical except for `size.min(cap as usize)`. The draw-stream order (arity, then split, then members) is the fact the determinism test rests on, and it is easiest to audit stated once. Legibility standard: finished code is obviously correct.

Evidence:

       293	        Inputs::VersionSliceCapped(cap) => {
       294	            // As `VersionSlice`, the arity draw capped at the roster
       295	            // row's declared bound.
       296	            let arity = draw_arity(size.min(cap as usize), rng);
       297	            let sizes = split_budget(size, arity, rng);
       298	            let mut rejected = 0;
       299	            let inputs = sizes
       300	                .into_iter()
       301	                .map(|n| {
       302	                    let draw = samplers
       303	                        .version
       304	                        .sample_bytes(n, rng)
       305	                        .expect("every byte size down to 1 has canonical versions");
       306	                    rejected += draw.rejected;
       307	                    draw.bytes
       308	                })
       309	                .collect();
       310	            (inputs, arity, rejected)
       311	        }

Resolution: Two helpers, `draw_version(samplers, n, rng, &mut rejected) -> Vec<u8>` and `draw_party(samplers, n, rng) -> Vec<u8>`, owning the two `expect` proofs; `draw_packed` and every arm call them. Match `Inputs::VersionSlice | Inputs::VersionSliceCapped(_)` in one arm with `let cap = match op.inputs { Inputs::VersionSliceCapped(c) => size.min(c as usize), _ => size }`, or give `VersionSlice` an `Option<u32>` cap. Acceptance: `run_op_is_deterministic_and_ordered` and the smoke test pass unchanged (the draw order is preserved); each `expect` string appears once in plan.rs.

#### fuelscape-pipeline-11: split_sum's hand-written parallel/sequential branch, its reference twin, and the 2048-bit pin dissolve into rayon's with_min_len
- Where: crates/before-fuelscape/src/count.rs:96-115 (related: crates/before-fuelscape/src/count.rs:73-84, 150-156, 224-230; crates/before-fuelscape/src/count/tests.rs:75-111)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (read rayon 1.12.0 in the cargo registry: `IndexedParallelIterator` for `RangeInclusive` is implemented only at u8/u16/i8/i16/char (range_inclusive.rs:223-226) while `Range<usize>` is indexed (range.rs:286); the enumeration pins at 24 bits reach at most 20 splits, below `PAR_SPLIT_THRESHOLD = 1536`, so the pin is today the only committed check that enters the parallel arm); executed: no
- Seen by: scaffolding [3]; refutation: reframed (the pin is not circular today; the one-liner needs a `Range<usize>`; "dated" is inaccurate: the threshold doc is machine-specific but undated); history: deliberate-and-holds: the reference build and pin were introduced on purpose with the rationale inline and in the commit (969cf3ae1)
- Owner-gated: no

`split_sum` branches by hand on a threshold between a rayon reduction and a sequential fold; `build_sequential` on both tables exists so `parallel_build_matches_sequential_reference` can compare the two transcriptions, four 2048-entry big-integer builds per test run. What the pin checks (order-independence of exact-integer addition across rayon's reduction tree) is a property of rayon and num-bigint, and rayon's `with_min_len` is exactly the "sequential below a length" rule written here by hand. Principle 3: infrastructure that reimplements a mature tool's capability, plus the maintenance it generates (a threshold constant, a reference twin, a pin with its own liveness guard). The pin is the only current reach into the parallel arm, so the dissolution and the rewrite land together, not the pin's deletion alone. The threshold's inline measurement table (count.rs:76-83) is what doctrine asks of a tuning constant and is not part of this finding.

Evidence:

        96	fn split_sum(subtree: &[BigUint], splits: RangeInclusive<usize>, pair_sum: usize) -> BigUint {
        97	    if splits.end() - splits.start() + 1 < PAR_SPLIT_THRESHOLD {
        98	        split_sum_sequential(subtree, splits, pair_sum)
        99	    } else {
       100	        splits
       101	            .into_par_iter()
       102	            .map(|a| &subtree[a] * &subtree[pair_sum - a])
       103	            .reduce(BigUint::zero, |x, y| x + y)
       104	    }
       105	}

Resolution: One chain over `*splits.start()..*splits.end() + 1` (a `Range<usize>`, which rayon indexes): `.into_par_iter().with_min_len(PAR_SPLIT_THRESHOLD).map(|a| &subtree[a] * &subtree[pair_sum - a]).sum::<BigUint>()`. Delete `split_sum_sequential`, both `build_sequential`, the `sum:` parameter of `build_with`, and `parallel_build_matches_sequential_reference`; restate the threshold doc as the `with_min_len` floor (rayon's splitter stops splitting when a half would fall below it, so the sequential regime is roughly twice the constant; the doc already calls the exact cut low-stakes). Acceptance: count.rs holds one convolution expression; count/tests.rs retains the enumeration and decoder pins; `just fuelscape-test` passes and its wall time drops by the four 2048-entry builds.

#### fuelscape-pipeline-12: VersionCounts and PartyCounts, and the two samplers, duplicate every method but the recurrence
- Where: crates/before-fuelscape/src/count.rs:137-156 (related: crates/before-fuelscape/src/count.rs:204-278; crates/before-fuelscape/src/sample.rs:184-207, 380-400; crates/before-fuelscape/src/plan.rs:129-158)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read: `build`, `build_with_progress`, `build_sequential`, the `build_with` signature, `subtree`, `whole`, `max_bits` are written twice with identical bodies; only the two `build_with` loop bodies differ); executed: no
- Seen by: structure-prose [27]; refutation: confirmed, with the caveat that the two named types keep a `VersionSampler` from being handed `PartyCounts`; history: both impl blocks date from the crate's first commit (3d6ab1d49); the later `build_with` refactor deduplicated within each grammar only
- Owner-gated: no

The grammar recurrences are the payload and deserve to sit side by side; the sixty lines of identical scaffolding around them are what a reader skims past to find them, and a change to the progress protocol or the convolution strategy touches two (with the samplers, six) sites. Legibility standard.

Evidence:

       137	    pub fn build(max_bits: usize) -> VersionCounts {
       138	        Self::build_with(max_bits, split_sum, |_, _| {})
       139	    }
       ...
       143	    pub fn build_with_progress(
       144	        max_bits: usize,
       145	        progress: impl FnMut(usize, usize),
       146	    ) -> VersionCounts {
       147	        Self::build_with(max_bits, split_sum, progress)
       148	    }
       ...
       154	    pub fn build_sequential(max_bits: usize) -> VersionCounts {
       155	        Self::build_with(max_bits, split_sum_sequential, |_, _| {})
       156	    }

Resolution: One `Counts<G: Grammar>` with the shared methods, a `Grammar` trait with one method (`fn entry(j: usize, subtree: &[BigUint]) -> BigUint`) and two unit types holding the recurrences; `VersionCounts = Counts<VersionGrammar>` and `PartyCounts = Counts<PartyGrammar>` keep the typed distinction the refutation notes. Behavior-preserving. Acceptance: count/tests.rs passes with constructor names unchanged; both recurrences read in one screen.

#### fuelscape-pipeline-13: Dead accessors: counts() on both samplers, max_bits() on both tables, BitSink::len and is_empty
- Where: crates/before-fuelscape/src/count.rs:191-194 (related: crates/before-fuelscape/src/count.rs:274-277; crates/before-fuelscape/src/sample.rs:62-70, 204-207, 397-400)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn '\.counts()' crates/before-fuelscape/src` and `'\.max_bits()'` return nothing; the only `.len()` in sample.rs is `self.bytes.len()` at line 53; `BitSink` is used outside sample.rs only at enumerate.rs:157 through `default`/`push`/`into_bytes`; the spanbands binary uses `VersionSampler::new` and `sample_bytes` only); executed: no
- Seen by: structure-prose [25]; refutation: confirmed; history: no-rationale-found: the accessors and the doc naming "plan-time feasibility checks" arrived together (3d6ab1d49) and have had no caller at any commit since
- Owner-gated: no

Principle 3: an accessor earns its place by naming a caller outside itself; `counts()`'s docstring names a use ("plan-time feasibility checks") that does not exist, which is a ghost reference in the forward direction, and `is_empty` exists only to satisfy clippy's `len_without_is_empty` for a `len` nobody reads.

Evidence:

    (count.rs)
       191	    /// The largest subtree size the table covers.
       192	    pub fn max_bits(&self) -> usize {
       193	        self.subtree.len() - 1
       194	    }

    (sample.rs)
       204	    /// The count table (for tests and plan-time feasibility checks).
       205	    pub fn counts(&self) -> &VersionCounts {
       206	        &self.counts
       207	    }

Resolution: Delete the six methods. If a plan-time feasibility check is intended (a plan whose span exceeds the table), write it and keep `counts()` with that caller. Acceptance: `cargo check` in crates/before-fuelscape is clean with the methods removed.

#### fuelscape-pipeline-24: slice_overlays dispatches on operation-name strings with a runtime panic as its only totality check
- Where: crates/before-fuelscape/src/families.rs:507-521 (related: crates/before-fuelscape/src/families.rs:98-113, 385; crates/before-fuelscape/src/render/tests.rs:32-43)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (read: nine name literals and an `other => panic!` arm; `overlay_inputs` panics on an unmapped signature at 385; the smoke test iterates every `ROSTER` row once at `max_bytes: 8`, which is what closes the hole today); executed: no
- Seen by: structure-prose [31]; refutation: confirmed (structure, not a gap); history: no-rationale-found: the string match arrived with the fold panels (eb6b35f42) and 46eb64f9 extended it
- Owner-gated: no

Types-first: which committed families mark which slice panel is roster knowledge held outside the roster, keyed by a string a rename does not update, and its totality is a runtime panic the smoke test happens to reach. A `SliceFamilies { Stagger, MeetShade, Both }` value on the `Inputs::VersionSlice`/`VersionSliceCapped` variants (or an `overlay` field on `OpSpec`) makes the assignment exhaustive at compile time and puts the per-row rationale beside the row it describes.

Evidence:

       507	fn slice_overlays(name: &str, max_bytes: usize) -> Vec<FamilyInput> {
       508	    match name {
       509	        "version_join_all" | "span_join_all" => stagger_ramps(max_bytes),
       ...
       513	        "shape_combine" => stagger_ramps(max_bytes),
       514	        "version_meet_all" | "span_meet_all" => meet_shade_ramp(max_bytes),
       ...
       520	        other => panic!("no committed slice families for {other}"),

Resolution: Move the slice-family choice into the roster (`Inputs::VersionSlice(SliceFamilies)`, `VersionSliceCapped(u32, SliceFamilies)`), delete the name match, and keep the signature-keyed table for fixed-arity rows (a function of the signature). Keep the `other => panic!` at 385 only with a comment naming the smoke test as its check. Acceptance: no `match name` on string literals remains in families.rs; adding a slice row without choosing its families is a compile error.

#### fuelscape-pipeline-32: spanbands is a one-off investigation binary that re-implements the plan's split rule and hand-rolls its CLI
- Where: crates/before-fuelscape/src/bin/spanbands.rs:108-121 (related: crates/before-fuelscape/src/bin/spanbands.rs:65-83; crates/before-fuelscape/src/plan.rs:189-206, 213-243; crates/before-fuelscape/Cargo.toml:18-21, 36-40)
- Class / severity / confidence: vestigial / low / medium
- Provenance: verified (`git log -- crates/before-fuelscape/src/bin/spanbands.rs` shows one commit, 5b2ae58c, 2026-07-31; `grep -rln spanbands .agent-notes justfile` finds nothing; only `bin/fuelscape.rs` uses clap; spanbands.rs:114 inlines `rng.gen_range(1..size)` while `split_budget` and `draw_packed` are private); executed: no
- Seen by: scaffolding [10]; refutation: confirmed (at HEAD the equivalence holds exactly, since `split_budget(total, 2)` is one `gen_range(1..total)`; the hazard is future drift; whether the banding question is settled is owner knowledge); history: no-rationale-found: the commit states the question it instruments; no note, results file, or later commit records what the CSV showed
- Owner-gated: no

This anchor lies outside the fourteen listed partition files; it is included because the binary duplicates the partition's draw rule. It claims to re-draw the atlas's two-operand input space but inlines the binary split instead of calling `plan`'s, so a change to `split_budget` breaks the equivalence silently (Principle 3, duplicated generator); it parses flags by hand while Cargo.toml:36-40 says clap serves "the runner binaries"; and the classify-first-versus-emit-always question it was built to answer has no recorded verdict.

Evidence:

       111	            let mut rng = cell_rng(seed, "spanbands", size, index);
       112	            // The binary split rule of the atlas's two-operand rows: one
       113	            // uniform cut in `1..size`.
       114	            let split_a = rng.gen_range(1..size);
       115	            let split_b = size - split_a;

Resolution: If the banding question is settled, record the verdict in an agent note and delete the binary (and the `scan-meter`/`limb-meter` features Cargo.toml:18-21 pulls for it, if nothing else reads them). If it is still wanted, make it a `--native-counters` mode of the main runner that draws through `plan` (a `pub(crate) draw_pair`, or `draw_packed` made `pub(crate)`) and uses the shared clap `Args`. Acceptance: either the binary is gone with its conclusion recorded, or it draws through `plan` and a change to `split_budget` changes its pairs.

Synthesis note: fuelscape-render-21 (this document) is the same binary from the render partition, adding the clap rationale and the manifest footprint; one decision closes both.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| fuelscape-pipeline-16 | `crates/before-fuelscape/src/enumerate.rs:120` | enumerate is a pub module with only test callers, and party_subtrees returns a documented tuple beside a named struct | `#[cfg(test)] pub mod enumerate;`; a `PartyMember` struct |
| fuelscape-pipeline-18 | `crates/before-fuelscape/src/sample.rs:289-290` | The zero-leaf exclusion is spelled through version_leaf_count at a corner argument; the two samplers differ in assert strength; a dead .max(1) | One `excluded_two_bit_leaf` helper; one assert strength; drop `.max(1)` |
| fuelscape-pipeline-20 | `crates/before-fuelscape/src/sample/tests.rs:40-61` | Two exhaustive byte-string sweeps of the decoders, in two modules, with two bound constants | One parallel `accepted_byte_strings(len)` helper; one bound constant |
| fuelscape-pipeline-22 | `crates/before-fuelscape/src/families.rs:58-60` | ramp's 1 << 20 guard contradicts its comment and names what it catches nowhere | `const MAX_RAMP_KNOB` with the failure it catches |

### Fuelscape: render

11 entries (1 medium, 5 low, 5 nit); the full record is `evidence/partitions/fuelscape-render.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: fuelscape-render-10. Related findings in other documents: fuelscape-render-15 (feature-gap: dump accretion), fuelscape-render-27 and fuelscape-render-23 (correctness: `typesetDocMath`'s reach and the smoothing).

#### fuelscape-render-18: A panel pool of width 1: scoped threads, an atomic cursor, and two mutexes wrap a sequential loop
- Where: crates/before-fuelscape/src/bin/fuelscape.rs:69-75 (related: crates/before-fuelscape/src/bin/fuelscape.rs:37-41, :206-217, :239-293)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`git show 7744a717` introduces `const CONCURRENT_PANELS: usize = 4;` with a stated purpose; `git show 6d84c8fa`, fourteen minutes later, is a one-line diff to `1` whose whole message is "Fuelscape only samples one panel at a time."); executed: no
- Seen by: scaffolding [3], adequacy [29], structure-prose [33], instrument-correctness [58]; refutation: confirmed; history: no rationale found (no note or later commit says why 1)
- Owner-gated: no (a dev-tool binary; git keeps the pool)

Machinery outlives the constraint that justified it, and legibility matters almost as much as correctness. With the width at 1 the loop over `selected` is sequential in roster order, yet the reader must still verify a `Mutex<Option<DumpWriter>>`, a `Mutex<Vec<Option<_>>>` of result slots, an `AtomicUsize` cursor, per-panel `insert_before` sub-bars, and four `expect` proofs about panicked holders and filled slots; the module doc (37-41) and the loop comment (206-211) describe completion-order printing and overlapping panel wall times that a width of 1 makes impossible, so the prose is not in the present tense about what the code does.

Evidence:

        69	/// Panels sampled concurrently.
        70	///
        71	/// Each panel's own samples already fan out on the shared rayon pool,
        72	/// so this bounds only how many panels' serial phases (overlay points,
        73	/// render, dump append) overlap — and how many sub-bars the progress
        74	/// display carries at once.
        75	const CONCURRENT_PANELS: usize = 1;
       ...
        39	//! plain lines). Completion lines print in completion order — panel
        40	//! wall times overlap — while the gallery and the dump index keep
        41	//! roster order, and the measurements themselves stay order-free.

Resolution: replace the pool with `for (i, op) in selected.iter().enumerate()` holding `writer` and `rendered` directly (no mutexes, no slots, no cursor), delete the constant, and rewrite lines 37-41 and 206-211 for sequential panels whose samples fan out on rayon; or, if 1 is a measured tuning outcome the owner wants to keep as a parameter, record the measured reason at the constant. Acceptance: either `std::thread::scope`, `AtomicUsize`, and both `Mutex`es are gone from `main` with the smoke and dump pins unchanged and the gallery in roster order, or the constant's doc names why 1 and the module doc no longer describes overlap that cannot occur.

#### fuelscape-render-12: `compact.rs` and `dump.rs` carry one reader and one writer twice
- Where: crates/before-fuelscape/src/compact.rs:320-375 (related: crates/before-fuelscape/src/compact.rs:92-120, :283-307; crates/before-fuelscape/src/dump.rs:72-103, :152-165, :186-247)
- Class / severity / confidence: modularity / low / high
- Provenance: assessed (read both readers side by side; the string "run parameters differ from the index's" appears at compact.rs:359 and dump.rs:222); executed: no
- Seen by: structure-prose [34]; refutation: confirmed; history: mirroring is deliberate at the format level (design note §2:63-66, "following the dump module's idiom exactly"); code sharing is undiscussed, and 6f63edb7 edited both readers in lockstep
- Owner-gated: no

The two `read` functions are the same body modulo constants and the final per-document check (index-path resolution, `parse`, `check_banner`, empty-ops rejection, per-op parse, banner, `RunParams` uniformity, name agreement), and the `IndexDoc`/`OpDoc` pairs and writers are likewise parallel; a strictness rule added to one (rejecting duplicate op names in the index, say) must be mirrored by hand in the other, and the version-bump history shows they already evolve together. `compact.rs` already imports `dump::{parse, check_banner, malformed, write_atomic}`, so the import path for a shared generic exists.

Evidence:

       356	        if RunParams::from(&doc.meta) != index.meta {
       357	            return Err(dump::malformed(
       358	                &op_path,
       359	                "run parameters differ from the index's",
       360	            ));
       361	        }

    dump.rs:
       219	        if RunParams::from(&doc.meta) != index.meta {
       220	            return Err(malformed(
       221	                &op_path,
       222	                "run parameters differ from the index's",
       223	            ));
       224	        }

Resolution: extract one generic dataset layer in `dump.rs` (or a sibling module): a `read` parameterized by the banner constants and the payload type with a per-document `validate` callback, and the matching writer; `dump` passes the grid check, `compact` passes `validate`; both keep their own constants and payload types. Acceptance: one reader body in the crate; `compact::read` and `dump::read` are thin calls; every existing compact and dump test passes unchanged.

#### fuelscape-render-13: Test fixtures duplicated verbatim, and hand-rolled temp dirs that leak on failure
- Where: crates/before-fuelscape/src/compact/tests.rs:9-50 (related: crates/before-fuelscape/src/dump/tests.rs:11-52; crates/before-fuelscape/src/render/tests.rs:28-29, :64, :104-107, :124; crates/before-fuelscape/src/compact/tests.rs:119, :185, :213, :272-274; crates/before-fuelscape/src/dump/tests.rs:129, :160, :199; crates/before-fuelscape/Cargo.toml:82-83)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (compared both `synthetic_atlas` bodies line by line: identical; `temp_dir` differs only in the "before-compact-"/"before-fuelscape-" prefix; every site cleans up only via a trailing `remove_dir_all`); executed: no
- Seen by: scaffolding [13], adequacy [32], structure-prose [36]; refutation: confirmed; history: no rationale (compact/tests.rs copied dump/tests.rs's fixture; `tempfile` is not a dev-dependency)
- Owner-gated: no

Two copies of one fixture drift silently (a change to the roster glyphs the fixture exercises must land twice), and prefer a dependency over hand-rolling: `tempfile::TempDir` cleans up on drop, including on a failing assertion, where the trailing `remove_dir_all` leaves every failed test's directory behind in the system temp dir.

Evidence:

        45	/// A per-test temporary directory, cleaned up by the caller.
        46	fn temp_dir(name: &str) -> std::path::PathBuf {
        47	    let dir = std::env::temp_dir().join(format!("before-compact-{name}-{}", std::process::id()));
        48	    std::fs::create_dir_all(&dir).expect("temp output dir");
        49	    dir
        50	}

Resolution: move `synthetic_atlas` (and a JSON `tamper` helper, see finding 14) into one `#[cfg(test)]` module both suites import; add `tempfile` as a dev-dependency and replace the four temp-dir idioms with `TempDir::new()`, dropping the trailing `remove_dir_all` calls. Acceptance: one `synthetic_atlas` definition; no `std::env::temp_dir()` in the crate's tests; a deliberately failing assertion leaves nothing behind.

#### fuelscape-render-21: `spanbands` is referenced by nothing, records no result, hand-rolls its CLI against the manifest's clap rationale, and alone justifies two `before` features and the `suanpan` dependency
- Where: crates/before-fuelscape/src/bin/spanbands.rs:1-6 (related: crates/before-fuelscape/src/bin/spanbands.rs:20-23, :65-83; crates/before-fuelscape/Cargo.toml:16-26, :36-40)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep for "spanbands" across *.rs, *.md, *.toml, *.yml, *.py, *.typ, and the justfile, excluding target and .claude/worktrees, hits only Cargo.toml:19 and the binary; `git log -- crates/before-fuelscape/src/bin/spanbands.rs` shows one commit, 5b2ae58c, whose message states the question; no `.agent-notes` entry records an answer); executed: no
- Seen by: scaffolding [4], adequacy [21], structure-prose [41], [42], instrument-correctness [59]; refutation: confirmed (and verified that `scan-meter`/`limb-meter` and `suanpan/touch-meter` serve only this binary); history: no rationale found; the clap port (c6d8106a, "The runner's flag parsing moves to clap") postdates the binary and wrote the manifest's "runner binaries" rationale without touching it
- Owner-gated: yes (retiring an instrument)

An instrument earns its place by naming what it serves outside itself: a one-off analysis binary with no recipe, test, CI leg, note, or recorded outcome is scaffolding, and its dependency footprint (Cargo.toml:16-26) exists only for it. If it stays, it should meet the crate's own standard: it parses flags by hand with `panic!`/`expect` and keeps a hand-maintained flag list in its module doc while Cargo.toml:36-40 justifies clap for "the runner binaries" precisely against hand-rolled asserts.

Evidence:

         1	//! The span-band discriminator: per-pair native measurements of the
         2	//! pair operations over the `version_span` input space, as raw CSV.
         3	//!
         4	//! The atlas's `version_span` heatmap shows banded conditional work
         5	//! distributions, but a heatmap cannot say *which coordinate of the
         6	//! pair* separates the bands — it plots work against total size alone.
       ...
        81	            other => panic!("unknown flag {other} (see the module doc for the flag list)"),

    Cargo.toml:
        36	# Argument parsing for the runner binaries: per-flag help and defaults
        37	# beside their declarations, and the mode exclusions (a replay or a
        38	# compaction takes no measuring flags) enforced declaratively instead of
        39	# by hand-rolled asserts.

Resolution: owner's call: (a) record the discriminator's result (which pair coordinate separates the `version_span` bands, and what it decided about the classify-first versus emit-always trade) in an agent note or the span kernel's docs, and delete the binary together with the `scan-meter`/`limb-meter` features and the `suanpan` dependency in this manifest; or (b) keep it, port the flags to a clap `Args` struct, add a `just spanbands` recipe with the question stated, and give it a smoke test. Acceptance: either the binary and its three manifest lines are gone and a note holds its result, or `just --list` names it, `--help` derives the flag docs, and `--sizes 64,abc` exits with a clap error rather than a panic.

Synthesis note: fuelscape-pipeline-32 (this document) records the duplicated split rule; one decision closes both.

#### fuelscape-render-26: `__FS_NO_ANIM` and the `Fuelscape.parse` export have no consumer
- Where: crates/before/docs/fuelscape.js:1105 (related: crates/before/docs/fuelscape.js:1533-1535; tools/fuelscape-claims:25-39; .agent-notes/2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md:316-317, :449)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep for `__FS_NO_ANIM` over rs/js/md/py/justfile, excluding target and the derived header, hits only fuelscape.js:1105 and the design note; tools/fuelscape-claims calls only `Fuelscape.accepts`); executed: no
- Seen by: structure-prose [40]; refutation: confirmed; history: deliberate-but-expired (the tool called `parse` at 61f05692 and 2efff149 switched it to `accepts`; the note keeps the hook as a "test hook" while ruling screenshot pinning a non-goal, and nothing ever set it)
- Owner-gated: no

A hook earns its place by naming the test that flips it, and an export by its caller; the test this hook anticipated was declared a non-goal, and the tool that used `parse` now uses `accepts`.

Evidence:

      1105	    const noAnim = typeof window !== "undefined" && window.__FS_NO_ANIM;
       ...
      1533	const Fuelscape = {
      1534	  parse: parseBound,
      1535	  accepts: acceptBound,

Resolution: delete the `noAnim` branch (keeping the `prefers-reduced-motion` path) and the `parse` export; or land the DOM or node test that uses them and cite it at the hook. Acceptance: grep for `__FS_NO_ANIM` across the tree returns nothing or returns a test; `Fuelscape`'s exports are exactly what tools/fuelscape-claims and the page use.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| fuelscape-render-2 | `crates/before-fuelscape/src/render.rs:137-138` | nit: long qualified paths at use sites, and a `use` block split by the allocator static | Import at file top; one contiguous `use` block |
| fuelscape-render-4 | `crates/before-fuelscape/src/render.rs:226-239` | nit: dead defaults and clamps in `aggregate` and the reference curves, palette hex repeated in the gallery CSS, and consequential drops with no comment | Drop the dead defaults and clamps; derive gallery colors from the constants; comment the drops |
| fuelscape-render-20 | `crates/before-fuelscape/src/bin/fuelscape.rs:180-187` | nit: stringly-typed table callback with a catch-all arm; allocator rationale duplicated with the manifest | A `Table` enum for the callback; one allocator rationale |
| fuelscape-render-24 | `crates/before/docs/fuelscape.js:568-569` | nit: the pointer-to-viewBox conversion, the quantile-step handler, and the guide-tip scan are each written twice | `svgPoint(e)`; route the slider through `quantKey`; derive `tip` from `vals` |
| fuelscape-render-28 | `crates/before/docs/fuelscape.css:54` | nit: one sans-serif font stack spelled five times where the stylesheet already uses a token | Add `--fs-sans` beside `--fs-mono` |

### Workspace tools (tools/)

#### tools-13: citecheck carries three string-literal-aware scanners with three escape conventions
- Where: tools/citecheck:88-108 (related: citecheck:205-235, 289-329)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (line 101 uses a lookbehind on the previous character, line 222 skips one character on a backslash, line 310 skips two; only `macro_block_fn_names` handles char literals and block comments); executed: no
- Seen by: structure-prose [41]; refutation: confirmed; history: no-rationale-found (two commits wrote the three lexers; 11363f89 fixed a crash in one without unifying them)
- Owner-gated: no
- Cross-references: surface-roster-31 (this document) asks for citecheck to become a typed Rust checker, which would retire all three lexers at once.

Three lexers means three places a Rust lexical corner (a `'"'` char literal, an escaped quote) is handled differently, and three places to test. Legibility and one definition per capability.

Evidence:

   101	            if c == '"' and (i == 0 or line[i - 1] != "\\"):
   102	                in_string = not in_string
   103	            elif not in_string and c == "/" and line[i : i + 2] == "//":
   222	            if c == "\\":
   223	                i += 1
   310	                    i += 2 if text[i] == "\\" else 1

Resolution: one `code_spans(text)` generator that skips string and char literals and both comment forms and yields code text with offsets; `strip_line_comments`, `top_level_entries`, and the `macro_block_fn_names` brace walk consume it. Acceptance: one lexer function; the existing fixtures (the quoted-brace descriptor at 481-486 included) still pass.

Synthesis note: two excerpt line numbers were corrected at merge time: the quoted `if c == "\\":` and `i += 1` lines sit at tools/citecheck:222-223 at `9e5784fb` (the partition report numbered them 221-222); the text is verbatim, and the argument's "line 222" names the backslash test, whose skip is the line below it.

#### tools-18: covcheck-expected.json carries an empty `place/filter.rs` branch key, the residue of a cured remediation entry
- Where: tools/covcheck-expected.json:202 (related: tools/covcheck:116, 170-173)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git log -S'place/filter.rs' -- tools/covcheck-expected.json` lists only 8a9583c5, where the key held a `remediation` entry anchored `let (flip, step) = self.probe.step();`; the JSON tally shows it is the only empty list in the file); executed: no
- Seen by: scaffolding [12], structure-prose [45]; refutation: confirmed; history: deliberate-but-expired (the entry was removed by aa7c96a0 and the empty list left behind)
- Owner-gated: no
- Cross-references: tools-16 (verification) proposes the scope-wide presence rule that replaces this row's only effect; gate-legs-9 (this document) and tools-14 (verification) hold the `remediation` disposition the row's entry once carried.

Through `resolve()` (line 116 records empty entry lists) and `check_branches()` (170-173), the key asserts only that the file appears in the branch lcov, which no other file gets and which no `why` states; a reader cannot distinguish it from intent.

Evidence:

   202	  "crates/before/src/version/skyline/place/filter.rs": [],

Resolution: delete the key; if file presence is the intended pin, it belongs in covcheck as the scope-wide rule of tools-16, not in one empty row. Acceptance: the branch map has no empty lists and `just coverage-kernel-branch` is unaffected.

#### tools-32: workflowlint spells the interpreter roster twice
- Where: tools/workflowlint:86-94 (related: workflowlint:146-161, 164-174)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (`INTERPRETERS` and the alternation inside `SUBST_FETCH` repeat the same nine names plus the python form; the pipeline check at 170 and the substitution check at 174 consult different spellings); executed: no
- Seen by: structure-prose [38]; refutation: confirmed; history: no-rationale-found (both spellings born in facc7550)
- Owner-gated: no

Adding an interpreter to one and not the other makes the two checks disagree: add `pwsh` to `INTERPRETERS` and `curl ... | pwsh` is caught while `pwsh <(curl ...)` is not. Named constants over restated literals.

Evidence:

    86	INTERPRETERS = {"sh", "bash", "dash", "zsh", "fish", "ksh", "node", "perl", "ruby"}
    87	PYTHON = re.compile(r"python[0-9.]*$")
    90	SUBST_FETCH = re.compile(
    91	    r"(?:(?:/[\w./-]+/)?(?:sh|bash|dash|zsh|fish|ksh|node|perl|ruby"
    92	    r"|python[0-9.]*)|\$SHELL|\$\{SHELL\})\b[^|;&]*"

Resolution: build the alternation from the set (`"|".join(map(re.escape, sorted(INTERPRETERS)))` joined with the python form) and compile `SUBST_FETCH` from it. Acceptance: each interpreter name appears once in the file; a self-test case adds a name to the set and shows both the pipeline and substitution forms red.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| tools-10 | `tools/benchjudge:244-247` | `checked_denominators` runs twice per judged cell (the call inside `fit_exponent` can never raise), and mutantcheck computes each pattern's listed and suppressed counts twice | Drop the call at benchjudge:246 and say in `fit_exponent`'s docstring that the caller validated the pair; a `counts(rx, raw, filtered)` helper in mutantcheck |
| tools-21 | `tools/doclint:279-282` | Two overlapping fixture suites (`summary_cases` 3-tuples and the named `cases` 4-tuples) drive `long_summaries`, pinning the crate-root exemption twice | Fold `summary_cases` into `cases` with names, one loop |
| tools-24 | `tools/manifestlint:140-143` | One self-test expectation is a bare tuple, and the `isinstance` branch at 149-150 exists only to normalize it | Write the expectation as a one-element list; delete the branch |
| tools-31 | `tools/testdoc:111-113` | House style varies per tool: self-test success announced by five and silent in three; `read_text`/`write_text` without an encoding at four sites; three defs without docstrings; argparse in four tools and hand-parsed argv in six | One convention per axis: `<tool>: self-test ok`, `encoding="utf-8"` on every call, a docstring per def, one argv style |

## Workspace sweeps

### Module graph

5 entries (3 low, 2 nit); the full record is `evidence/sweeps/module-graph.md`. Related findings in other documents: module-graph-1 (verification-gap: the segments counter), module-graph-5 and module-graph-11 (documentation: the `implementation` essay and the validation index).

#### module-graph-3: The `meter` feature compiles two hot-path atomic counters into bench builds, against the manifest's stated policy
- Where: crates/before/Cargo.toml:60-68 (related: crates/before/Cargo.toml:49, crates/before/Cargo.toml:79-84, crates/before/src/version/hull_traffic.rs:16-21, crates/before/src/version/hull_traffic.rs:45-58, crates/before/src/version/hull_traffic.rs:105-118, crates/before/src/version/skyline/web_traffic.rs:19-24, crates/before/src/version/skyline/web_traffic.rs:46-57, crates/before/src/version/skyline/web_traffic.rs:100-113, crates/before/src/version.rs:968-999, crates/before/src/version/skyline/watermark.rs:962-984, crates/before/src/version/skyline/pool_traffic.rs:27-60, crates/before/src/meter.rs:75-81, crates/before/src/meter.rs:3646-3675, crates/before/src/meter/tests.rs:578-616, crates/before/src/version/skyline/fill/tests.rs:459-468, crates/before/src/version/skyline/fill/tests.rs:597-600, crates/before/tests/meter.rs:9208, crates/before/tests/meter.rs:9244-9250, crates/before/benches/board.rs:81-84, Cargo.toml:117-122, src/tree/tests.rs:1414-1416)
- Class / severity / confidence: modularity / low / high
- Provenance: assessed (read every gate and call site of both counters, the self-dev-dependency, the bench imports, and the introducing commits e4d4817f1 and 76579a3c); executed: no
- Verification: confirmed, with one refinement on the idiom claim: `hull_traffic.rs:16-17` and `web_traffic.rs:19-20` call their gate "the `codec::scan` counter's idiom", but `codec::scan` is gated under `scan-meter`, a counter feature kept out of bench builds; the shim shape matches, the gate does not; history: no-rationale-found: e4d4817f1 introduced the rung counters "under the meter feature" without addressing the bench-build policy, and 76579a3c followed "the hull_traffic idiom".
- Owner-gated: yes: `SpanTraffic`, `EmitTraffic`, `span_traffic`, `emit_traffic`, and their resets are `pub` under `meter`.

The `meter` feature is documented as exposure (generators, counter readers, the surface roster,
`encoded_bits`), and the `limb-meter` comment states the policy: a counter that adds a relaxed
atomic bump to a production path lives behind an additive counter feature and stays out of the
bench builds. `hull_traffic::record` (one bump per pair-hull call in `Version::span_refs`) and
`web_traffic::record` (one bump per priced-offset domination read in `MinWeb::emit_offset`) are
gated on `feature = "meter"`, which the self-dev-dependency lights for every bench and test build.
The board and amplify benches therefore time a span ladder and an emit path carrying one more
atomic per call than the shipped library.

Evidence:

        60	# Exposes the adversarial input generators and deterministic resource meters
        61	# (`src/meter.rs`) as `before::meter`, so the metering test binaries can build
        62	# worst-case inputs and read the counters they envelope, and the public
    --- Cargo.toml:80-83 (the policy, stated at limb-meter) ---
        80	# sees them. Additive and off by default: it adds one relaxed atomic bump per
        81	# `Base` operation, so it stays out of the bench builds (deliberately not in the
        82	# self-dev-dependency above) and is lit by `--all-features` runs and the
        83	# metering envelopes that assert it.
    --- hull_traffic.rs:112-118 ---
       112	#[inline(always)]
       113	pub(crate) fn record(rung: Rung) {
       114	    #[cfg(feature = "meter")]
       115	    counter::record(rung);
       116	    #[cfg(not(feature = "meter"))]
       117	    let _ = rung;
       118	}
    --- version.rs:970-971 ---
       970	        if codec::canonical_eq(&a.0, &b.0) {
       971	            hull_traffic::record(Rung::Equal);
    --- watermark.rs:966 ---
       966	                    web_traffic::record(web_traffic::Decision::DominatedAbove);

Resolution: Either move both counters' `mod counter`, their `record` bodies, and the `meter::*_traffic`
readers under a counter feature (`scan-meter`, whose contract already reads "one relaxed atomic
bump per primitive, out of the bench builds", or a new `traffic-meter = ["meter"]`), mirroring how
`pool_traffic` rides `limb-meter`; or amend the `meter` feature comment to name the two bumps it
adds to production paths. Costs to name for the first option: the unit-test readers at
`meter/tests.rs:578-616` and `fill/tests.rs:459-468, 597-600` run under `cfg(test)` with no
feature gate, so the counter gate becomes `any(test, feature = "scan-meter")` or those tests gain a
cfg; the integration reader at `tests/meter.rs:9244-9250` already sits under `limb-meter` (line
9208); the rumors consumer reads `span_traffic` under its own `meter = ["before/limb-meter",
"before/scan-meter"]` (Cargo.toml:122; src/tree/tests.rs:1414), so it keeps compiling under either
counter feature (the dependence sweep should confirm). Acceptance: the `meter` feature comment and
the set of production-path bumps it compiles agree.

Synthesis note: skyline-watermark-27 (this document) is the same gate question for `web_traffic` alone; the module-graph summary's open question 2 recommends moving both counters under `scan-meter`.

#### module-graph-4: Limb-meter taps are spelled three ways and their module path five ways
- Where: crates/before/src/codec/base.rs:10-14 (related: crates/before/src/codec/base.rs:92-93, crates/before/src/codec/base.rs:142-143, crates/before/src/codec/base.rs:167-168, crates/before/src/codec/gamma.rs:259-262, crates/before/src/codec/dsi.rs:282-285, crates/before/src/version/rank/num.rs:58-69, crates/before/src/version/skyline/query/integral.rs:366-376, crates/before/src/version/skyline/query/integral.rs:387-393, crates/before/src/version/skyline/query/integral.rs:414-420, crates/before/src/codec/base/limb_metered.rs:1-54, crates/before/src/codec/base/limb_meter.rs:29-69, crates/before/src/codec/scan.rs:24-59, crates/before/src/codec.rs:39-40, crates/before/src/version/skyline/pool_traffic.rs:20-21, crates/before/Cargo.toml:115-117)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (grep of every `limb_meter::` and `limb_metered::` reference outside tests; each site read); executed: no
- Verification: confirmed, and one cost added: the same counter is reached as `limb_meter::` (base.rs), `super::base::limb_meter::` (gamma.rs:262), `super::limb_meter::` (dsi.rs:285, through the gated re-export at codec.rs:39-40), `crate::codec::limb_meter::` (integral.rs:370-372, 390, 417), and `crate::codec::base::limb_meter::` (rank/num.rs:66); history: no-rationale-found for the split beyond `limb_metered.rs` needing `Base` from `super` while the counter does not.
- Owner-gated: no

The crate's counter idiom is one ungated `#[inline(always)]` shim over a cfg-gated body, called
unconditionally (`codec::scan`, `hull_traffic`, `web_traffic`, `pool_traffic`, suanpan's `touch`).
The limb meter is tapped through `limb_metered.rs` shims, through `#[cfg(feature = "limb-meter")]`
attributes placed on statements inside kernel bodies (five sites), and through four private
site-local shims that re-spell the idiom. A cfg attribute inside a kernel body makes the featured
and unfeatured token streams differ at that site, so the "compiles to nothing" argument is re-made
per site instead of once at the shim, and nothing mechanical checks it. The `base.rs:10` comment
"Test-only metering" is also inaccurate: the release `amp_board` example requires the feature.

Evidence:

        10	// Test-only metering for big-arithmetic operations:
        11	#[cfg(feature = "limb-meter")]
        12	pub(crate) mod limb_meter;
        13	pub(crate) mod limb_metered;
        14	use limb_metered::*;
    --- base.rs:167-168 ---
       167	                #[cfg(feature = "limb-meter")]
       168	                limb_meter::record(2);
    --- gamma.rs:261-262 ---
       261	    #[cfg(feature = "limb-meter")]
       262	    super::base::limb_meter::record_wide(&m);
    --- rank/num.rs:63-69 ---
        63	#[inline(always)]
        64	fn meter_wide(limbs: u64) {
        65	    #[cfg(feature = "limb-meter")]
        66	    crate::codec::base::limb_meter::record(limbs);
        67	    #[cfg(not(feature = "limb-meter"))]
        68	    let _ = limbs;
        69	}
    --- pool_traffic.rs:20-21 ---
        20	//! The recording compiles to nothing without the `limb-meter` feature —
        21	//! its siblings' idiom (`codec::limb_meter`, `suanpan::touch_meter`) —

Resolution: Fold `limb_metered.rs` into `limb_meter.rs` in the `codec::scan` shape: an inner
`#[cfg(feature = "limb-meter")] mod counter` holding the statics, readers, and resets; ungated shims
`record(u64)`, `record_wide(&UBig)`, `record_densified(u64)`, and the existing `meter_limbs*`; make
`pub(crate) mod limb_meter` unconditional and the codec.rs:39-40 re-export ungated; replace the five
inline `#[cfg]` taps with plain calls; reduce the four site-local shims to direct calls; spell the
path one way (`crate::codec::limb_meter`); re-word base.rs:10 to name the feature. Acceptance: no
`#[cfg(feature = "limb-meter")]` attribute appears outside `limb_meter.rs` and `meter.rs`; behaviour
and every envelope reading unchanged.

Synthesis note: codec-base-text-tree-2 (this document) is the `limbs_of_bits` half of the same limb-meter consolidation.

#### module-graph-9: meter.rs holds a private generator table and the public counter read surface in one file
- Where: crates/before/src/meter.rs:18-19 (related: crates/before/src/meter.rs:67-122, crates/before/src/meter.rs:3543-3723, crates/before/src/meter/registry.rs:12-15, crates/before/src/meter/board/ops.rs:192-194)
- Class / severity / confidence: modularity / low / medium
- Provenance: assessed (item skeleton of meter.rs by grep: 3726 lines, 103 `fn`s, the sixteen public counter readers all at 3552-3721; 69 `super::` references in registry.rs; ops.rs's `ops()` at 193 with `#[allow(clippy::too_many_lines)]` above it); executed: no
- Verification: confirmed as a source-navigation cost only: the rendered `before::meter` rustdoc shows the public items alone, so the doc reader is unaffected; the file reader scrolls 3.4k lines of private constructors to reach the read API every metering binary imports; history: no-rationale-found.
- Owner-gated: no

The module has two responsibilities: about eighty private shape constructors reachable only through
`registry::Shape`'s exhaustive match, and the deterministic-counter read surface plus `Packed`. The
registry's "private to the meter module" invariant survives unchanged if the constructors move to a
child module with `pub(super)` visibility.

Evidence:

        18	//! The generators themselves are private: every instrument mints its shapes
        19	//! through the family registry ([`registry`]), whose roster is the single
    --- registry.rs:12-15 ---
        12	//! - **Construction.** The raw shape constructors are private to the
        13	//!   [`meter`](crate::meter) module, and [`Shape`] is the one public door:
        14	//!   its constructor table (`Shape::builder`, the exhaustive match) is
        15	//!   the constructors' only caller outside the module's own unit tests.
    --- board/ops.rs:192-193 ---
       192	#[allow(clippy::too_many_lines)]
       193	pub(super) fn ops() -> Vec<Op> {

Resolution: Move the constructors, `Packed::from_bits`, and the `ev_*`/`pow2*` helpers into
`meter/shapes.rs` as `pub(super)` items; leave meter.rs with its module doc, submodule declarations,
`Packed`, the counter readers, and the re-exports; rewrite registry.rs's `super::x` to
`super::shapes::x` (mechanical). Nit in the same spirit: `ops()` spans lines 193-2275 under a
`too_many_lines` allow; concatenated per-`OpGroup` functions would keep the table declarative while
making a row findable. Acceptance: `before::meter`'s public items and the `compile_fail,E0603`
doctest at registry.rs:23-27 are unchanged; file motion only.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| module-graph-10 | `crates/before/src/codec/display.rs:1-3` | The production module cycles are facade/engine pairs; one (codec::display -> idbits) is the substrate reaching up | Consider moving `write_id` beside `idbits`; leave the five facade pairs |
| module-graph-12 | `crates/before/src/laws.rs:107-109` | Redundant cfg on the exported law-roster macro inside an already-gated module | Drop the redundant cfg attribute |

### Dependencies

7 entries (4 low, 3 nit); the full record is `evidence/sweeps/deps.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: deps-10, deps-16. Related findings in other documents: deps-2 and deps-3 (verification-gap: dev-only duplicates in the deny leg; build.rs's rerun list), deps-5 (correctness).

#### deps-4: build.rs is a docs-only formatter every consumer compiles; a design question against a recorded decision
- Where: crates/before/build.rs:1-21 (related: crates/before/Cargo.toml:16-21, justfile:680-715, crates/before/src/testing/fuelscape_islands.rs:29, .agent-notes/2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md:46-54 and 171-174)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read build.rs, Cargo.toml, the justfile recipes, the design note; `du -sh crates/before/fuelscape` = 1.5M, 105 files; grep counts 155 `fuelscapes/` include sites in src); executed: no
- Verification: reframed: the sweep filed this as a medium-severity dissolution candidate; the design note records the per-consumer cost as weighed and accepted, so this is an owner-gated design question, not a defect; history: deliberate-and-holds (the note's §4 accepts the cost at "~320 KB ... negligible"; the dataset is now 1.3 MB committed JSON, still small)
- Owner-gated: yes (the direction of a documentation pipeline the owner designed)

The script renders 105 committed JSON files into rustdoc `<details>` islands
included at 155 `#[doc = include_str!(concat!(env!("OUT_DIR"), ...))]` sites
and themes one SVG. Every consumer of `before` (rumors, before-viz, the four
detached workspaces, any external user) compiles a serde_json
build-dependency and parses the dataset per clean build, and a malformed
committed dataset is a compile failure of every downstream crate. The
repository already uses the alternative idiom for the header and the README
figure (a committed derived file held fresh by a check), and the script has
accreted connections that serve its own placement: the rerun list (deps-3), a
source-tree write behind an env opt-in, and a dotted `.open.html` name to
stay outside the totality scan's charset.

Evidence:

    crates/before/Cargo.toml
        16	# build.rs is a pure formatter: it renders the committed fuelscape
        17	# widget datasets (fuelscape/) into the doc islands the # Complexity
        18	# sections include from $OUT_DIR. serde_json only reads the committed
        19	# JSON; nothing is measured or computed at build time.
        20	[build-dependencies]
        21	serde_json = { workspace = true }
    crates/before/build.rs
        78	        // include (the dot keeps it outside the totality scan's
        79	        // island-name charset — the op's own doc site still owes the
        80	        // closed island).
        ...
       187	    if std::env::var_os("BEFORE_REGEN_DOC_FIGURE").is_some() {
       188	        std::fs::write(path, &want).expect("the README figure is writable");

    .agent-notes/2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md
       171	- Build-deps: `serde_json` only. The script runs for every consumer build
       172	  (including the detached fuzz/fuzzfit workspaces and wasm targets — build
       173	  scripts execute on the host, so this is safe); it is a file transform
       174	  over ~320 KB and stays negligible.

The decision is recorded and the cost it accepted is still small; what has
changed since is the dataset size (4x), the two freshness and message
defects found in the script by this sweep (deps-3, deps-5), and the
carve-outs above, which are the "maintenance cascade" signal of Principle 3.
The note does not record the committed-islands alternative as considered.

Resolution: a design proposal for the owner, costs named. Have the compactor
(crates/before-fuelscape, which owns the format) emit the islands, the
`.open` variant, and the two themed SVGs as committed derived files under
crates/before/docs/, included by path from the doc sites; hold them fresh by
`fuelscape-verify`'s existing diff plus an in-crate test that re-derives the
header and README figure from CARGO_MANIFEST_DIR; point the totality test
(fuelscape_islands.rs:29, which reads `$OUT_DIR/fuelscapes/index`) at the
committed directory listing. Then build.rs and the serde_json
build-dependency dissolve, `just doc-figure` becomes a compactor mode, and a
dataset defect fails a test instead of every consumer's compile. Cost:
roughly 1.5 to 3 MB more committed derived HTML (the islands embed the JSON
payload), or the standalone JSON goes if tools/fuelscape-claims learns to read
the islands. Acceptance: if taken, `crates/before` has no build.rs and no
`[build-dependencies]`, `just docs` renders every island, and
`fuelscape-verify` fails on a hand-edited island. If declined, the deps-3 and
deps-5 fixes stand on their own and this entry closes as a recorded
decision.

#### deps-7: the bitvec dev-dependency and two one-off probe examples outlive the investigation that justified them
- Where: crates/before/Cargo.toml:33-36 (related: crates/before/examples/emit_probe.rs:1-16 and 50-52, crates/before/examples/perf_probe.rs:1-9, .agent-notes/2026-08-04-perf-probe/README.md:15-17)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep for emit_probe/perf_probe across justfile, tools/, .github/, and both AGENTS.md files: the only reference is the Cargo.toml comment; grep of `before::` in emit_probe.rs: none; `git log` on both files and on the BitsBuf landing); executed: no
- Verification: confirmed, with one sharpening: emit_probe.rs uses nothing from `before` at all (it reproduces a word writer standalone "so the comparison needs no crate internals"), so it is a self-contained microbenchmark of hand-rolled writers against bitvec living in before's examples; history: already-known (the agent note states where the files remain; it records no reason to keep them)
- Owner-gated: yes (whether a profiling harness stays at hand is the owner's call; the emit_probe half, whose question BitsBuf answered, is not)

`bitvec` is a dev-dependency only so emit_probe.rs can time an external
baseline. Both probes date from the 2026-08-04 perf investigation
(7d101fe7), perf_probe calls itself "One-off", neither appears in any recipe,
and the decision they informed landed on 2026-08-18 (83e61b4d, "replace the
bitvec build buffer with the crate-owned BitsBuf"). Cost: bitvec and its four
transitive crates compile into every `cargo test -p before`, `clippy
--all-targets`, and `check --all-targets`, and 527 lines of probe code are
linted, doclinted, and rustdoc-built on every gate for no committed verdict.

Evidence:

    crates/before/Cargo.toml
        34	# The emit_probe example's external comparison baseline: the production
        35	# build buffer is the crate-owned BitsBuf, and nothing shipped links bitvec.
        36	bitvec = { workspace = true }
    crates/before/examples/emit_probe.rs
        50	/// A minimal word-buffered MSB-first bit writer: the staging discipline
        51	/// the crate's own `PackedBuilder` ships, reproduced standalone so the
        52	/// comparison needs no crate internals.
    crates/before/examples/perf_probe.rs
         1	//! One-off profiling harness for the perf-probe investigation.
    .agent-notes/2026-08-04-perf-probe/README.md
        15	The instruments it names — `perf_probe.rs` (op loops) and `emit_probe.rs`
        16	(output-primitive microbenchmarks) — are cargo examples and remain at
        17	`crates/before/examples/`, where they can still be run.

Principle 3: machinery outlives the constraint that justified it. The
criterion benches against `before::oracle` are the retained instrument for
the question perf_probe asked.

Resolution: retire examples/emit_probe.rs and the `bitvec` dev-dependency
(its question is closed and it exercises no crate code); decide separately
whether perf_probe.rs stays as a profiling harness, and if it goes, re-aim
the agent note's sentence at git history (the note is exempt from the ghost
rule; the Cargo.toml comment is not). Acceptance: `bitvec` absent from
crates/before/Cargo.toml and the root lock; `just gate` clean.

Synthesis note: benches-examples-21 (this document) carries the probe's own defects (no crate code exercised; a ghost path in its one assert).

#### deps-8: before's `serde` feature selects `derive`, which nothing in the crate uses
- Where: crates/before/Cargo.toml:30-30 (related: Cargo.toml:68, crates/before/src/serde_impls.rs:14-15 and 28)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep for `derive(...Serialize|Deserialize)` and `serde_derive` over crates/before/{src,tests,benches,examples}: nothing; read serde_impls.rs:1-40); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes (the selection is part of the published feature surface, though no consumer in this workspace observes a difference)

Every serde impl in before is hand-written and imports only the traits; the
Deserialize impls go through `<Vec<u8>>::deserialize`, which serde's `alloc`
feature supplies. The workspace table takes serde with default features off,
so before's selection is the whole selection, and `derive` pulls serde_derive,
syn, quote, and proc-macro2 into the graph of any external consumer that
enables `before/serde` without otherwise deriving. rumors and before-viz
already select `derive` themselves, so the cost is external-consumer only.

Evidence:

    crates/before/Cargo.toml
        30	serde = { workspace = true, optional = true, features = ["derive", "alloc"] }
    Cargo.toml
        68	serde = { version = "1", default-features = false }
    crates/before/src/serde_impls.rs
        14	use serde::de::Error as _;
        15	use serde::{Deserialize, Deserializer, Serialize, Serializer};
        ...
        28	        let bytes = <Vec<u8>>::deserialize(d)?;

Resolution: `serde = { workspace = true, optional = true, features = ["alloc"] }`.
Acceptance: `just gate` clean; `cargo tree -p before --features serde -e
normal` shows no serde_derive.

Synthesis note: crate-root-3 (this document) is the same line.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| deps-13 | `crates/before/src/meter.rs:1660-1663` | before names one type two ways: `dashu_int::UBig` in codec, `suanpan::UBig` in meter and the query tests | `use dashu_int::UBig;` in meter.rs and query/tests.rs |
| deps-14 | `crates/before-fuelscape/Cargo.toml:42-45` | before-fuelscape carries a second bignum (num-bigint + num-traits) beside the dashu-int already in its graph | Port count.rs and sample.rs to dashu-int's `rand` feature; hold the atlas byte-identical |

### Inventory

7 entries (1 low, 6 nit); the full record is `evidence/sweeps/inventory.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: inventory-4. Related findings in other documents: inventory-2 (verification-gap: the segments column).

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| inventory-5 | `crates/before/src/codec/base.rs:448-455` | `Shl<i32> for Base` exists only to accept unsuffixed literal shifts in two tests, and says nothing about it | Suffix the two literals and delete the impl, or comment it and hard-assert |
| inventory-6 | `crates/before/src/codec/literal.rs:52-66` | `id_node` re-validates the whole subtree at every tuple level | Validate once in the tuple `TryFrom` impl |
| inventory-8 | `crates/before/src/clock.rs:257` | `#[allow(clippy::result_large_err)]` on `Clock::join_all` carries no local justification | Put the rationale at line 257 |
| inventory-9 | `crates/before/src/codec/stack.rs:61-71` | Dead `len == 64` arm under a `len <= 63` assert in `BitStack::push_bits` | Two asserts and `(top << len) \| value` |
| inventory-11 | `crates/before/src/version/rank.rs:838-843` | Capacity hint spelled as an `expect` where the crate elsewhere degrades to zero | `unwrap_or(0)` with the hint sentence |
| inventory-12 | `crates/before/src/version/skyline/query.rs:422` | Visibility: `pub fn` inside `pub(crate)` modules, and an unnameable `Base` in a feature-public signature | Re-export `Base` (or return `Ticks`); narrow four `pub` fns to `pub(crate)` |

### Clippy pedantic

3 entries (1 low, 2 nit); the full record is `evidence/sweeps/clippy-pedantic.md`.

#### clippy-pedantic-1: `impl Shl<i32> for Base` serves two unsuffixed test literals and turns a negative amount into a `u32::MAX`-scale shift in release
- Where: crates/before/src/codec/base.rs:448-455 (related: crates/before/src/codec/tests.rs:54, crates/before/src/codec/tests.rs:528, crates/before/src/codec/base.rs:480-488, crates/before/src/testing/snapshots.rs:72, crates/before/src/lib.rs:417, crates/before/src/codec.rs:41)
- Class / severity / confidence: vestigial / low / high
- Provenance: assessed (read the impl and its siblings; grepped every `Shl`/`Shr` impl and every `<< <literal>` in before's src, tests, benches, examples; `git blame` and `git log -S` on the impl); executed: no
- Verification: confirmed; history: no-rationale-found (the impl is unchanged since the crate's first commit as `itc`, 94902e86, 2026-06-01; no note, comment, or roster entry mentions it)
- Owner-gated: no (`Base` is crate-private: `mod codec;` at lib.rs:417, `pub use base::Base` only inside codec.rs)

The impl exists so an unsuffixed literal shift on a `Base` compiles through the
`i32` fallback; the only such sites in the crate are two test lines shifting by
`64`. Its guard is a `debug_assert!` and its body is `rhs as u32`, so in a
release build a negative amount becomes a shift of up to 4,294,967,295 bits
handed to the backend rather than a named failure. A production impl carried
for two test spellings fails the "name what it serves outside itself" test
(Principle 3), and a debug-only guard in front of a lossy sign cast fails the
panic doctrine's "every guard is a one-line proof in every profile".

Evidence:

       448	impl Shl<i32> for Base {
       449	    type Output = Base;
       450	
       451	    fn shl(self, rhs: i32) -> Base {
       452	        debug_assert!(rhs >= 0, "Base left shift must be non-negative");
       453	        self << rhs as u32
       454	    }
       455	}

    crates/before/src/codec/tests.rs
        54	            n = (n << 64) | Base::from(limb);
       528	            value = (value << 64) | Base::from(limb);

    crates/before/src/testing/snapshots.rs (the suffixed idiom already in use)
        72	    let big = Base::from(1u8) << 64u32; // 2^64

    crates/before/src/codec/base.rs (the house form for the side that cannot be total)
       485	        let rhs = usize::try_from(rhs)
       486	            .expect("a left shift this wide exceeds the backend's representable width");

Resolution: suffix the two literals (`n << 64u32`, `value << 64u32`) to match
snapshots.rs:72, then delete the `Shl<i32>` impl. If unsuffixed literals should
keep compiling, replace the body with
`self << u32::try_from(rhs).expect("Base left shift amount is non-negative")`
so the failure is named in every profile. Acceptance: `grep -rn "impl Shl<i32>"
crates/before/src` is empty and `just clippy` is clean; or, under the second
option, `Base::from(1u8) << -1i32` panics at the named `expect` in a
`--release` test.
Construction: delete the impl and run `cargo check -p before --all-targets`;
exactly codec/tests.rs:54 and :528 fail with `no implementation for Base <<
i32`, which confirms the caller set.

Synthesis note: Three reviews reached this impl (clippy-pedantic-1, codec-base-text-tree-8, inventory-5, all in this document). They disagree only on owner-gating: codec-base-text-tree-8 holds that `Base` values are reachable under the `meter` feature (`meter::skyline::query::min_ticks` returns one), so the open ruling on whether the meter surface is stable API decides.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| clippy-pedantic-7 | `crates/before/src/codec/base.rs:46-50` | A bundle of pedantic hits that are pure spelling improvements with no behavior change | One mechanical commit of the listed spellings (`is_ok_and`, `ilog2`, `&self`, elided lifetimes, digit separators) |
| clippy-pedantic-8 | `crates/before/src/meter/board/floors.rs:377-380` | Scan floors route an integer byte count through `f64` to multiply by a constant that is `1.0`, while the sibling floors are integers | Make the scan-floor constant `u64 = 1` with `saturating_mul`, as the tick floor does (owner-gated: the constant is `pub`) |

### API audit

1 entries (1 low); the full record is `evidence/sweeps/api-audit.md`.

#### api-audit-12: suanpan::Limbs withholds the exact size Chunks already knows, so before's Ticks::limbs re-derives it by hand
- Where: crates/suanpan/src/limbs.rs:60-72 (related: crates/suanpan/src/limbs.rs:43-45, crates/before/src/version/ticks.rs:112-149, crates/suanpan/src/claims.rs:86)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read limbs.rs and ticks.rs in full; read `dashu-int-0.5.0/src/ubig.rs:90-94` and `src/repr.rs:226-238`: `as_words` returns the normalized word slice, empty for zero, so `chunks.len()` equals `bits().div_ceil(64)` on both word widths); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes: additive trait impls on suanpan's public iterator, and the claims roster row names the trait set

`suanpan::Limbs` wraps `core::slice::Chunks` (which is `ExactSizeIterator +
DoubleEndedIterator + FusedIterator`) yet implements only `Iterator` and
`DoubleEndedIterator` with the default `size_hint` of `(0, None)`. before's
`Limbs` therefore carries a `remaining` counter computed from
`bits().div_ceil(64)` with a `-= 1` on every step and an `expect` on the
`usize` conversion, all to state a number the wrapped iterator already holds.

Evidence:

        60	impl Iterator for Limbs<'_> {
        61	    type Item = u64;
        62	
        63	    fn next(&mut self) -> Option<u64> {
        64	        self.chunks.next().map(pack_limb)
        65	    }
        66	}

       112	    pub fn limbs(&self) -> Limbs<'_> {
       113	        Limbs {
       114	            limbs: suanpan::Limbs::new(&self.0 .0),
       115	            remaining: usize::try_from(self.0.bits().div_ceil(64))
       116	                .expect("a stored count's limb count fits usize"),
       117	        }
       118	    }

Resolution: in suanpan, add `size_hint` delegating to `self.chunks.size_hint()`, `impl ExactSizeIterator for Limbs<'_> {}`, `impl FusedIterator for Limbs<'_> {}`, and update the `FAMILY_SURFACE` row "Limbs iteration (Iterator / DoubleEndedIterator)"; then let before's `Limbs` delegate and drop `remaining`. Acceptance: `suanpan::Limbs::new(&x).len()` equals the yielded count under proptest; before's `Limbs` has no `remaining` field.

Synthesis note: version-core-24 (this document) is the before side; the same additive impl closes both.

### Gate legs

2 entries (2 low); the full record is `evidence/sweeps/gate-legs.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: gate-legs-9, gate-legs-10. Related findings in other documents: gate-legs-1 (correctness: `masked_cmp_hole_envelope` under llvm-cov), gate-legs-3 (verification-gap: wasm32-pins absent from CI).

### Meter adequacy

2 entries (1 low, 1 nit); the full record is `evidence/sweeps/meter-adequacy.md`. Scaffolding entries for this module, reproduced in the scaffolding section above: meter-adequacy-8, meter-adequacy-12.

### Paper fidelity

1 entries (1 nit); the full record is `evidence/sweeps/paper-fidelity.md`.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| paper-fidelity-13 | `crates/before/src/lib.rs:49` | "mint" used for constructing values across the crate | One pass replacing "mint" at the 56 sites |

### Prose hygiene

2 entries (1 medium, 1 low); the full record is `evidence/sweeps/prose-hygiene.md`.

#### prose-hygiene-2: Tier 2 compactness apparatus describes a representation decision that has been taken
- Where: crates/before/src/meter/tier2.rs:1-19 (related: crates/before/src/meter/tier2.rs:23-32; crates/before/src/testing/compactness.rs:1-20, 31-44, 59-67, 78-84, 115-125; crates/before/src/meter/tier2/tests.rs:223-236, 705-709; crates/before/src/meter.rs:307-309; crates/before/src/version/skyline.rs:1-2, 26-28)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (read every listed range; `git log` on tier2.rs and compactness.rs; read `.agent-notes/2026-07-23-before-skyline-encoding` §12); executed: no
- Verification: confirmed; history: deliberate-but-expired (the instruments landed 2026-07-23 in 3e1a1631 and f2d0011b to inform "the open decision" the skyline-encoding note's §12 names; faf3cd0a closed it on 2026-07-25; the 2026-07-31 excision touched compactness.rs only to remove two calendar dates and left every "today")
- Owner-gated: yes: whether the ratio check survives as a stored-versus-per-node size bound is the owner's call; the prose fix is unconditional

`tier2.rs`, `testing/compactness.rs` and their tests call the Tier 2 coding a
candidate whose adoption a decision "turns on" and call the per-node coding
"today's"; `skyline.rs` states that the stored form is the Tier 2 coding, and
`check_sample`'s own comment concedes it. "Today" now denotes a coding that
survives only as the oracle-lowered `packed_bits_of` form.

Evidence:

         5	//! large a canonical [`Version`](crate::Version) would be if re-encoded as its
         6	//! preorder topology (one flag bit per node, exactly as today) plus its leaf
        10	//! compactness ratio between this size and today's encoded size is the evidence
        11	//! the representation decision turns on, so the walk here is written for

    compactness.rs:
         3	//! The Tier 2 coding stores preorder topology plus delta-coded absolute leaf
         4	//! values ([`crate::meter::tier2`]); the claim its adoption turns on is that
         5	//! its coded size never exceeds ~2x today's size plus O(1) bits per node.
        78	    // The decision-era "current" coding is the min-lifted packed preorder
        79	    // stream (one gamma-coded base per node), re-derived through the
        80	    // oracle lowering; the stored coding is Tier 2 itself.

    skyline.rs:
        26	//! This coding is the stored and wire form of a [`Version`]:

Resolution: owner call on the instrument: dissolve `meter::tier2` and
`testing::compactness` (the claim is settled and the stored coding is the
measured one), or re-denominate them as a stored-coding-versus-per-node-coding
size bound with the per-node coding named for what it is
(`testing::bridge::packed_bits_of`) and every "today"/"decision" phrase
removed. Either way rewrite tier2.rs:1-19 and 23-32, compactness.rs:1-20,
31-44, 59-67 and 115-125, tier2/tests.rs:223-236 and 705-709, and
meter.rs:307-309 in the present tense over what is. Acceptance: `grep -rn -i
"today\|decision" crates/before/src/meter/tier2* crates/before/src/testing/compactness*`
returns nothing, and the module docs state which two codings are compared
and why the comparison is kept.

Synthesis note: The dissolution option is narrower than the entry states. The meter-registry-tier2 partition refuted dissolving the sizer half: `tier2_size` is the independent second implementation that the length-agreement pins at skyline/tests.rs:514-523 and the kernel emission pins in tier2/tests.rs rest on. The testing-diff-gen partition records that dissolving the compactness suite reopens the 2026-07-24 defended keeps. Scope the dissolution to `testing::compactness`'s envelope and to the prose; keep `tier2_size`. The prose half is meter-registry-tier2-14 and testing-diff-gen-17 (both documentation).

#### prose-hygiene-12: Em-dashes in // and # comments and in message string literals
- Where: crates/before/tests/meter.rs:251-252 (related: plain `//` lines per file: tests/meter.rs 107, skyline/fill.rs 23, skyline/fill/tests.rs 19, version/tests.rs 18, board/ops.rs 17, codec/tests.rs 17; string-literal lines: before-fuelscape/src/ops.rs 31, tests/meter.rs 23, board/coverage.rs 10, surface.rs 9, registry.rs 6, suanpan claims/tests.rs 4; non-Rust: justfile 57, docs/fuelscape-header.html 24, docs/fuelscape.js 22, .cargo/mutants.toml 20, ci.yml 9, scripts 7, Cargo.toml comments 13)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (partitioned every U+2014 hit over the in-scope file set by line prefix: 4140 total, 3255 rustdoc, 551 plain `//`, 104 Rust code lines, 230 non-Rust of which 58 are the two cargo-rdme-derived READMEs); executed: no
- Verification: confirmed (the sweep's 553/106/~170 reproduce within a few lines); history: no-rationale-found
- Owner-gated: no

The owner's doctrine puts spaced double-hyphens in code comments and colons
or semicolons in messages that reach a terminal, with true em-dashes
reserved for rendered prose. Rustdoc and the derived READMEs are exempt;
the remaining 655 Rust lines and 172 non-Rust lines are not.

Evidence:

       251	// mechanism that prices the row; the measurements of record — and every
       252	// re-pin's movement and attribution — live in the pin commits (`git log

    board/coverage.rs:
       181	        "Version ^ Version (BitXor, owned and borrowed — the pair hull)",

    board/coverage/tests.rs:
        78	            "{op}: priced by board rows AND excused in BOARD_NOT_APPLICABLE — \

Resolution: mechanical sweep: in `//` and `#` comments replace ` — ` with
`: `, `; ` or ` -- ` by sentence sense; in string literals that reach a
terminal replace with colons. A `tools/` linter leg rejecting U+2014 outside
`///`, `//!` and Markdown would keep it closed. Acceptance: the partition
reports zero plain-comment, code-line and non-Markdown hits.

### Recursion

2 entries (2 nit); the full record is `evidence/sweeps/recursion.md`. Related findings in other documents: recursion-1 (verification-gap: the segments currency), recursion-2 (verification-gap: the bridge guard policy).

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| recursion-8 | `crates/before/src/codec/stack.rs:58-102` | BitStack::push_bits carries a dead len == 64 arm; pop_bits's one-level recursion is undocumented | Drop the dead arm; note or inline `pop_bits`'s one-level self-call |
| recursion-9 | `crates/before/src/meter.rs:18-19` | "mint" for constructing values and coining terms is a crate-wide idiom | Sweep the 57 sites with the vocabulary pass |

### Suite economics

3 entries (2 low, 1 nit); the full record is `evidence/sweeps/suite-economics.md`.

#### suite-economics-3: Five hand-rolled source scanners in before/tests beside the surface-scan crate
- Where: crates/before/tests/amp_board_smoke.rs:314-340 (related: crates/surface-scan/src/lib.rs:168-196; crates/before/tests/doc_hidden.rs:25-43; crates/before/tests/foreign_reexport.rs:62-99; crates/before/tests/superlinear_tripwires.rs:14-16 and 72-98; crates/before/tests/verdict_matrix.rs:1232-1287; crates/before/Cargo.toml:50; crates/surface-scan/Cargo.toml:5)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (read all five scanners and `test_fns`; `grep -rn surface_scan crates/before` finds only src/testing/surface_coverage.rs:162 and 272, so no tests/ binary uses the dev-dependency declared at Cargo.toml:50); executed: no
- Verification: confirmed; history: no-rationale-found (surface-scan was cut down from a shared crate in 0a5bdaebd with `extract_public_fns` and `test_fns` as its scope; the roster pins predate it (f10f5b56f, 8409fe552, 9daec56ec) and were not migrated; nothing records a reason)
- Owner-gated: no

`band_test_names` is `surface_scan::test_fns` line for line with a name filter added, and four roster pins each carry a private recursive directory walk with its own fn-name discipline: superlinear_tripwires matches any trimmed `fn ` line (not attribute-gated, misses `pub fn`), verdict_matrix strips visibility and qualifiers but is also not attribute-gated, doc_hidden counts substring occurrences, foreign_reexport filters lines. surface-scan's own manifest calls it "the workspace's shared test machinery", so the circular-justification tell applies: the shared tool exists so rosters do not each re-derive the scan, yet four before rosters do, and three disciplines for one job means three blind spots a reviewer reasons about separately. The doc at superlinear_tripwires.rs:15 promises `#[test]` fns while the scanner at line 80 does not check the attribute.

Evidence:

       314	fn band_test_names(source: &str) -> BTreeSet<String> {
       315	    let mut names = BTreeSet::new();
       316	    let mut armed = false;
       317	    for line in source.lines() {
       318	        let t = line.trim();
       319	        if t == "#[test]" {
       320	            armed = true;
       321	            continue;
       322	        }

    (crates/surface-scan/src/lib.rs)
       174	pub fn test_fns(source: &str) -> BTreeSet<String> {
       175	    let mut names = BTreeSet::new();
       176	    let mut armed = false;
       177	    for line in source.lines() {
       178	        let t = line.trim();
       179	        if t == "#[test]" {
       180	            armed = true;
       181	            continue;
       182	        }

    (crates/before/tests/superlinear_tripwires.rs)
        15	//! must match the `#[test]` fns whose names carry `_reads_superlinear`,
        80	                let Some(rest) = line.trim_start().strip_prefix("fn ") else {

Resolution: add one directory walker to surface-scan (a `walk_rs_sources(root)` yielding `(PathBuf, String)` per `.rs` file) and compose each pin from it plus `test_fns` or a line filter; `band_test_names` becomes `test_fns(source)` filtered on the band convention. Acceptance: the four pins keep their rosters and pass unchanged; superlinear_tripwires.rs:15 is accurate because the scan is attribute-gated; no `fn scan(` remains under crates/before/tests.

Synthesis note: Same scanners as tests-other-3 and surface-roster-10 (this document).

#### suite-economics-4: Satellite meter binaries duplicate meter.rs fixtures and helpers, and the copied fixture has already drifted
- Where: crates/before/tests/coincident_span.rs:20-50 (related: crates/before/tests/meter.rs:10556-10587 and 769-781; crates/before/tests/answer_embedded.rs:31-41, 114, and 193; crates/before/tests/fold_skeleton.rs:20-30 and 40-42; crates/before/tests/meter.rs:10; crates/before/Cargo.toml:49 and 110-125; crates/before/tests/coincident_span.rs:13; crates/before/tests/answer_embedded.rs:25; crates/before/tests/fold_skeleton.rs:14)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (compared by reading; binary inventory from `wc -l` and the sweep's per_binary.txt: nine of thirteen before integration binaries hold at most six tests and under 0.1 s of summed work); executed: no
- Verification: confirmed and sharpened: the sweep called the fixture a verbatim copy, but the bodies are identical while the return tuples differ in order (`(v, w, redecoded)` at coincident_span.rs:49 against `(v, redecoded, w)` at meter.rs:10586), so the copies have already diverged; history: no-rationale-found (47b03e89 and 4398dcd4f state each witness's purpose and mutant, not why it lives outside tests/meter.rs)
- Owner-gated: no

coincident_span.rs copies `identity_fast_paths::fixture` (same six forked clocks, same 24-round schedule, same re-decode) and its `scanned` helper; answer_embedded.rs and fold_skeleton.rs carry identical `counters` helpers whose reset trio is `ticks_counters`'s body. Each satellite also picks its own flatness tolerance (x1.10 and a tenth of the top cell in answer_embedded.rs:193 and :114; x1.35 in fold_skeleton.rs:42) beside the suite's stated x1.25 (meter.rs:10; fold_skeleton.rs:40-41 cites it and widens). The three are cfg-gated at the file top, so `just test` (default features: the self-dev-dependency at Cargo.toml:49 enables `oracle` and `meter` only) builds and links three empty harnesses; the manifest declares `required-features` for the examples (115-117, 123-125) and has no `[[test]]` section. A fixture that exists twice drifts twice (it already has), and a reader of the flatness bands meets three tolerances derived at three sites.

Evidence:

        28	fn fixture() -> (Version, Version, Version) {
        29	    let mut main = Clock::seed();
        30	    let mut others: Vec<Clock> = (0..6).map(|_| main.fork()).collect();
        49	    (v, w, redecoded)

    (crates/before/tests/meter.rs)
     10565	    fn fixture() -> (Version, Version, Version) {
     10566	        let mut main = Clock::seed();
     10567	        let mut others: Vec<Clock> = (0..6).map(|_| main.fork()).collect();
     10586	        (v, redecoded, w)

    (crates/before/tests/fold_skeleton.rs)
        40	/// The per-byte flatness band across a doubling: the meter suite's
        41	/// ×1.25 flatness convention with margin for amortization wobble.
        42	const GROWTH_BOUND: f64 = 1.35;

Resolution: move coincident_span's three tests into meter.rs's `identity_fast_paths` (one `fixture`, one `scanned`); place answer_embedded and fold_skeleton beside `answer_embedded_product` in meter.rs with one counters helper and each band's tolerance derived at its site; for any feature-gated binary that stays separate, declare `[[test]] required-features` as the examples already do, so the unfeatured build skips it instead of linking an empty harness. Folding the four tiny roster binaries into one is a taste call whose link-time benefit is assessed, not measured. Acceptance: one `fn fixture()` and one counters helper across tests/; `cargo nextest list --workspace` under default features lists no zero-test before binary.

Synthesis note: tests-other-7 (this document) carries the `tests/support/meters.rs` shape and the isolation-note omission.

#### Nits

| Id | Where | Claim | Resolution |
|---|---|---|---|
| suite-economics-7 | `crates/surface-scan/src/tests.rs:14-25` | surface-scan test fixtures are written under the shared temp dir and never removed | `tempfile::tempdir()` from `fixture` |

## Positives

Each positive below is a partition or sweep review's, restated once where several reviews reported it; I re-read the cited site myself only where marked (re-read). They are the designs the simplification findings should preserve, and in several cases the model an entry asks a sibling to follow.

- Compile-to-nothing counter shims, uniform at `codec::scan`, `hull_traffic`, `web_traffic`, `pool_traffic`, and suanpan's `touch`: an ungated `#[inline(always)]` function over a cfg-gated `mod counter`, called unconditionally, so the kernels carry no feature forks (module-graph sweep; skyline-query and suanpan partitions). module-graph-4 asks the limb meter to adopt the same shape.
- `ByCurrency<T>` has no `Default` and no `..` path, and `each` destructures exhaustively, so a new currency is a compile error at every declaration, judgment, and render site (board-frame, board-families-floors-judge). This is the totality mechanism board-frame-1's dissolution rides on.
- `laws!` makes law registration structural and every consumer expands from `for_each_law_group!` (verified at four sites by the oracle-laws review); `diff_ops!` makes registration and execution the same act; the registry's exhaustive `Shape::builder` match ties every generator to one variant, with the `compile_fail,E0603` doctest as the committed demonstration that the generators are private (meter-core, meter-registry-tier2, testing-diff-gen).
- One roster, several consumers, no copy: surfacecheck, the in-tree coverage suite, before-fuelscape, and the board tiling all bind `before::surface::METHOD_SURFACE` under `meter`; the typed `Exclusion` vocabulary makes each exclusion a defended family, and `every_exclusion_family_is_inhabited` applies the scaffolding audit to the enum itself (surface-roster).
- `tests/support/fuzz_seed_set.rs` is one derivation included by `#[path]` from both the writer example and the checker test, so the seed corpus cannot drift from its check (fuzz-guests-pins, tests-other, module-graph); this is the precedent tests-other-3 and tests-other-7 extend to a shared scanner and a shared meter helper.
- `pair_fold`'s `orientation: impl Fn(Ordering) -> i8` is one merge walk serving three measures, each closure a three-line transcription of one row of the sigma table; one `Integrator` serves rank, distance, lag, and rank_cmp; `web.rs` reuses `MinWeb<P>` generically rather than re-implementing the anchored-minimum web (skyline-query).
- The watermark's payload seam is a zero-cost generic: `FnOnce` payload construction only on the arms that store, an `FnMut` death hook at exactly the difference's death, and a total `Close<P>`; the fill client at `P = ()` pays nothing (skyline-watermark). `pool_traffic.rs` passes the circular-justification test in writing by naming the property no other meter can see.
- `polarity.rs` concentrates the whole `Down`/`Up` difference in one sealed table of six one-line methods, with `Neutral`'s `unreachable!` arms carrying a proof checkable inside the module; `algebra.rs` states the four span operators as one leg table with each totality argument written once (span-causally).
- `emit.rs` makes join and meet differ in exactly one pick function; `sweep::sweep` carries each question as an exit predicate so three questions share one loop, and `Directions`, `order_exit`, and `eq_exit` are reused by masked and emit rather than re-spelled (skyline-coding, skyline-sweep-place-masked). skyline-coding-17 asks `emit` and `hull` to finish the job.
- Types over asserts in the fill kernel: the changed flag as an output mode (`Out`), `Cost` deriving `Ord` with its field order spelling the rule, layout claims bound to constants under the compiler, and `Step`'s absent variant argued from the types (skyline-fill-grow).
- `lockstep_holds` parametrizes the two region predicates by the single settling node with the algebra shared; `sum` collapses `(1, 1)` by fixed-width truncation so no position stack exists; `IdIndex`'s module doc names its trade and the instruments that price it (party).
- `DedupRuns` holds a clone rather than an address, with the reason (a freed buffer reused at the same address cannot masquerade as a duplicate); both n-ary folds keep the `(Input, Merged)` arm total with the commutativity argument stated (version-core; re-read at version.rs:1227-1229 by the partition).
- `fold.rs` is one home for the counter discipline with the quadratic left-fold failure named beside it; `MAX_TRACE_OPS` is derived into `GRID_N` rather than transcribed; `rng.rs` is one home for seeded randomness with the portability argument stated as mechanism (crate-root, testing-oracles, testing-diff-gen).
- suanpan's four single-responsibility modules form a DAG, the crate root re-exports and declares nothing of its own, every coined term is anchored to an identifier, the metering seam compiles to nothing, and the claims roster is total in both directions with witnesses bound by reach (suanpan).
- Bench IDs are the board's own cell names derived from the axis declarations, so a red board cell names the bench that times it with no second list; `benches/common` builds every input from one `Plan`; `amp_board.rs` owns only what the library cannot (benches-examples, board-frame). benches-examples-1 is the one exception the review found.
- `build.rs` is a pure formatter that recomputes nothing and pairs each derived artifact with a freshness assert naming the regenerating recipe; `Cargo.toml` closes two quiet-failure paths mechanically (`unexpected_cfgs = deny` and `required-features` on `amp_board`); `tools/manifestlint` holds every member manifest to the workspace table (crate-root, deps).
- The fuelscape pipeline's adequacy triangle (table versus enumeration, grammar versus the shipping decoders as set equality, decoder census, chi-square, proptest round trips), `select.rs` closing the silent-empty-run hole per filter, `dump::read` recomputing every grid and refusing disagreement, and `tools/fuelscape-claims` parsing claims with the widget's own exported grammar (fuelscape-pipeline, fuelscape-render).
- The fuzz-fit harness ties `REGS_RESERVE` to both budgets by a `const` assert, prices the ceiling and floor widths separately with the margins' measured edges named, and ships a synthetic tripwire per judgment leg; the differential is total, not sampled (fuzzfit-bands, fuzzfit-strategies).
- `gate-streams` carries a liveness floor on its own verdict; every `tools/` checker leads with `--self-test`; CI invokes justfile recipes rather than re-listing their steps; the `features` recipe checks each cfg-gated surface alone; four of five detached workspaces carry their own fmt and clippy leg (gate-legs, module-graph).
- Zero `unwrap()` in shipped code, zero `TODO`/`FIXME` markers, and no library function recursing on input depth: the recursion sweep's call-graph scan and its reading of every candidate agree, and every explicit stack states its per-level cost at its declaration (inventory, prose-hygiene, recursion; the `#[cfg(test)]` on `recurse::grow` at recurse.rs:100 re-read).
- Default clippy passes `-D warnings` with 28 single-site `#[allow]`s, each but one naming what it accepts and why; every production narrowing cast the pedantic sweep read is bounded in the same function by a named constant or an early return (clippy-pedantic, inventory).
- The dated-notes excision (d2a9d04e) collapsed two accreted history ledgers into standing prose and reported the `decided` machinery it could not dissolve as an owner question in its own message; the envelope table's comment keeps every measurement's history in pin commits (prose-hygiene, module-graph).

## Open questions for Finch

Deduplicated across the summaries; each item names the entries it settles and carries a recommendation.

1. The segments currency (board-frame-1, meter-core-11; crate-root-32, envelopes-a-2, module-graph-1, recursion-1, inventory-2, board-ops-render-15 in other documents): dissolve it, or keep it as a documented structural-zero pin at ceiling 0? Every review recommends dissolution; 1ddb5a483's reason for the dev-dependency move is that no library code recurses, and `deep_tree_stack_safety` is the instrument. Recommendation: dissolve, re-deriving `LADDER_TOP_SCALE`'s doc from what the ladder still provides.
2. Is the feature-gated `meter` surface (`pub mod meter`, `pub mod skyline` under `meter`, the counter readers, the board constants, `Base` in `min_ticks`'s signature) held to the stable-API rule? The answer decides owner-gating for board-frame-6, board-families-floors-judge-16, skyline-sweep-place-masked-35, skyline-coding-4, inventory-12, codec-base-text-tree-8, clippy-pedantic-1, and clippy-pedantic-8. Recommendation: treat it as instrument surface, record the ruling where the surface check reads it, and narrow the fifteen unreferenced board constants to `pub(super)` now.
3. Envelope harness unification (envelopes-a-4, envelopes-b-8): `Option`-typed columns (rows keep their pin sets, no re-measure) or every column pinned on every row (wider coverage, a one-time re-measure of the three-column tables)? Recommendation: `Option` columns first, widened row by row under the tightening rule. Which side is the roster of record for the door/kernel twin rows (envelopes-a-8, envelopes-a-11)? Recommendation: the door, with the scan column and value legs ported.
4. The `_view` doors (version-core-11): was the saved refcount clone/drop pair ever measured to matter? Nothing pins refcount traffic. Recommendation: treat the collapse as free and land it, taking a reading before rather than after if one is wanted.
5. `join_all`'s hand-back grouping and drain order (oracle-laws-2, oracle-laws-26): a pinned behavior or unspecified, as both `# Errors` sections say? Recommendation: unspecified; make the oracle sequential, compare verdict, accumulator, and region union, and let the laws carry the contract. If pinned, say so in the contracts and pin it once at `fold.rs`.
6. The two-arm rank numerator (rank-22): a coherent design proposal, not a defect. Recommendation: pursue after rank-32 (claim) and rank-16 (performance) land, since both shrink the seam's surface first; re-pin `RANK_PAIR_MISMATCH` and `RANK_SUM_MIXED` at the parent commit before the change.
7. `PackedBuilder` on `BitsBuf` (codec-bits-12): is the staging register's saving measurable on the bench corpus? Recommendation: construct and measure at parent and change on a quiet machine; take the consolidation unless a cell leaves its band, and if the register wins, move the register form into `BitsBuf` so one implementation remains. Should the `BitStack` migration (codec-base-text-tree-12, party-19, skyline-coding-32) cover all ten sites in one change so `BitsBuf::pop` can be deleted? Recommendation: yes.
8. Retire the line-scan extractor (surface-roster-9), or keep it as a stated stable-toolchain inner-loop convenience? Retirement moves the only totality feedback to the gate's `surface` stream and the CI `instruments` job. Recommendation: retire; the JSON walk catches strictly more and the retirement bar is met. Should `tools/citecheck` follow the same route into a typed Rust checker (surface-roster-31)? Recommendation: yes, once the fabricated-citation tripwire and shadow guard are reproduced as unit tests.
9. The `decided` dates (surface-roster-17, meter-registry-tier2-9, meter-adequacy-12): an intended embedded decision-record schema, or dated rationale to dissolve? d2a9d04e put this question to you. Recommendation: dissolve; `git log -S` on the reason strings recovers every date. If kept, one shared typed date parse and the exemption stated once in the project instructions.
10. The presize and `display_growth` allocation A/B arms (skyline-coding-28, deps-10; benches-examples-12 in the verification-gap document): was the record ever run to a verdict? Recommendation: close it the way the stacks leg was closed at cd171c29 (record the verdict in a note and the closing commit, dissolve seams, roster, recipe, and `alloc_arms`).
11. perf_probe.rs and emit_probe.rs (benches-examples-21, deps-7; benches-examples-22 in the documentation document): criterion 0.5.1 ships `--profile-time`, and every perf_probe loop has a criterion twin. Recommendation: retire emit_probe.rs and `bitvec` now; retire perf_probe.rs and name `just bench <target> <filter> -- --profile-time 10` as the profiler entry.
12. `spanbands` (fuelscape-pipeline-32, fuelscape-render-21): was the classify-first versus emit-always question answered on the span branch? Recommendation: record the answer in an agent note and delete the binary with its two `before` features and the `suanpan` dependency.
13. Cliff-fan (meter-registry-tier2-12): dissolve the family, or re-justify it against a production walk that exists? Recommendation: dissolve; keep the shape only as a coding-corpus member if the agreement corpora want it.
14. The Tier 2 compactness apparatus (prose-hygiene-2; meter-registry-tier2-14 and testing-diff-gen-17 in the documentation document): dissolve `testing::compactness`'s envelope, or re-denominate it as a stored-coding-versus-per-node-coding size bound? The keep is a recorded 2026-07-24 ruling and `tier2_size` stays either way. Recommendation: re-denominate the prose now; dissolve the envelope only if no consumer of the ratio outside the module can be named.
15. `BOARD_DECLARED_BENCH_RIDERS` (board-frame-25): does the "every declared-model cell keeps a wall-clock witness" rule apply to all four ratified models or to the heap/limb pair the test checks? Recommendation: derive the pinned subset from `declared_heap`/`declared_limb`, state at `BenchMode::Pinned` that the fold and capacity models sit on designed pairings by construction, and pin that. Should `just all` run the full bench mode, or should export.rs say the pinned subset is the verdict of record? Recommendation: the latter.
16. `designed()`'s destination and the board sub-roster (board-ops-render-2, board-families-floors-judge-6): family.rs beside the bundle-build match, or a field on `Coverage::Board`? Recommendation: family.rs, as part of a `BoardFamily` enum that also carries the base-size constant, so the registry row is the single declaration a new column requires.
17. The `meter` feature's scope (module-graph-3, skyline-watermark-27): should `hull_traffic` and `web_traffic` move under a counter feature so bench builds carry no relaxed-atomic bumps, or does `meter` admit per-decision liveness counters with the cost stated? Recommendation: move both under `scan-meter`; rumors' own `meter` feature already enables it.
18. `FREEZE_ALLOWANCE_DIGITS` as `pub(crate)` (meter-core-7): so meter.rs derives its six constants from it, with the test side naming the widths once (envelopes-a-19). Recommendation: yes.
19. `Packed` as one type or two (meter-core-3 in the documentation document, touched by meter-core-6's byte-identity oracle): split so `version()` exists only for event shapes? Recommendation: split, under the meter-surface ruling of item 2.
20. `Shl<i32> for Base` (clippy-pedantic-1, codec-base-text-tree-8, inventory-5): keep the impl with a total guard, or suffix the two test literals and delete it? Recommendation: delete; the smaller tree.
21. The `validate_id` re-passes (codec-base-text-tree-16, codec-bits-25, inventory-6): delete both calls and move the guarantee into the test harness, or keep one runtime pass as a totality defense with its rationale at the call site? Recommendation: delete both; the reference parser then becomes a pure grammar transcription.
22. `sweep::le` (skyline-sweep-place-masked-35 here; span-causally-26 in the performance document): narrow to `#[cfg(test)]`, or lift into production as the one-direction exit the fused walks advertise? One decision; recommendation: narrow now, and treat the lift as a separate performance proposal with its own pin.
23. `fuzz_decode` (fuzz-guests-pins-4): retained for raw-door throughput and transport-feature independence? Recommendation: keep it and say so in one sentence of its module doc.
24. `add_at`'s exit `debug_assert!` (suanpan-21, inventory-4) reopens 9f68c475's ruling with the assert's O(wide) cost as the new evidence. Recommendation: keep the O(1) conjunct, delete the scan, and confirm with the mutants campaign that no mutant moves from caught to survived. Is the exact-touch contract (lib.rs:283-286) meant to freeze constant-factor improvements? Recommendation: restate it as "deterministic and pinned; a count change is a versioned change named in its commit".
25. suanpan's standalone testability (suanpan-30; the envelopes-b summary's `accum_streams` question): should the accumulator witnesses in `tests/meter.rs` move into `crates/suanpan/tests/` so `claims.rs` stops citing `../before/tests/meter.rs`? Recommendation: move them and point `BANDS` at the in-crate path; add the in-crate `add_small`/`sub_small` pin either way.
26. The fold-index fallback past 2^32 bits (party-23 in the claim document; party-4 and party-28 touch the same kernels): widen the table and dissolve `build_unindexed` and its differentials, or carry the size clause? Recommendation: widen; the fallback arm exists only to be held by tests that would then go away.
27. Test relocation (version-core-27, rank-2, clock-26): move the `Rank`/`Ranked` suites to sibling `tests.rs` files and the serde legs to `serde_impls/tests.rs`, gathering the depth-100k proofs in one `testing/stack_safety.rs`? Recommendation: yes, as one coordinated change with `citecheck` green.
28. `[[test]] required-features` for the three cfg-gated whole binaries (coincident_span, answer_embedded, fold_skeleton; suite-economics-4, tests-other-7): under `just test` they compile to empty green binaries. Recommendation: fold them into `tests/meter.rs` beside `identity_fast_paths` and `answer_embedded_product`; for any that stays separate, declare `required-features` as the examples do.
29. Crate-wide vocabulary and register (paper-fidelity-13, recursion-9, prose-hygiene-12, board-families-floors-judge-15, and the partition instances): "mint" (56 to 82 sites), "honest" (146 to 234 lines), "door" (208 to 279 lines with no defining site), "seam" (276), and em-dashes in `//` comments (374 lines in `crates/before/src`). Recommendation: one owner ruling per word and one mechanical commit; define "door" once or replace it with the plain noun (the style guide's own table lists it); a `tools/` linter leg rejecting U+2014 outside `///`, `//!`, and Markdown keeps the dash rule closed.
30. `core::` versus `std::` (span-causally-10, oracle-laws-7): 32 files import `core::cmp::Ordering`, 15 `std::cmp::Ordering`; the crate is not `no_std`. Recommendation: `std` crate-wide in a mechanical sweep.
31. Edition 2021 under a 2024 workspace root (suanpan-1): migrate the five 2021 crates together, or state the reason at the root? Recommendation: migrate together with `cargo fix --edition`, one commit.
32. The fuzzfit clippy leg at HEAD (fuzzfit-strategies-13): the two same-type casts at ops.rs:764 and 858 would ordinarily trip `unnecessary_cast` under `-D warnings`, yet 4e64a4fb reports a clean gate with them present. One `just fuzzfit` run settles whether the wasm gate stream has been running; if red, it is a process finding, not a lint nit.
33. Packaging (deps-16): add an explicit `include` list before the first release so `cargo package` ships neither `results/benchmarks`, `reference/`, nor `scripts/`. Recommendation: yes, at release time; if build.rs's direction changes (deps-4), the list shrinks to `src`, `docs`, and `README`.

## Counts

By severity (all classes in this document):

| Severity | Entries |
|---|---|
| high | 0 |
| medium | 36 |
| low | 177 |
| nit | 163 |
| total | 376 |

By class:

| Class | Entries |
|---|---|
| simplification | 137 |
| idiom | 111 |
| vestigial | 51 |
| modularity | 44 |
| scaffolding | 33 |

By module:

| Module | Medium | Low | Nit | Total |
|---|---|---|---|---|
| Crate root: lib, error, iter, auto traits, build.rs, Cargo.toml | 0 | 2 | 0 | 2 |
| Clock | 0 | 2 | 3 | 5 |
| Party | 1 | 4 | 2 | 7 |
| Version core | 1 | 7 | 5 | 13 |
| Rank | 1 | 5 | 4 | 10 |
| Span and causally | 1 | 5 | 5 | 11 |
| Coding: module root, admit, build, encode and decode, emit, literal, validate, text, shape, walk | 1 | 6 | 5 | 12 |
| Fill and grow | 2 | 8 | 7 | 17 |
| Comparison kernels: sweep, place, masked, overlay, signed | 2 | 3 | 6 | 11 |
| Query | 0 | 5 | 5 | 10 |
| Watermark and the traffic counters | 3 | 4 | 6 | 13 |
| Bits | 1 | 2 | 4 | 7 |
| Base, text, and tree | 0 | 5 | 9 | 14 |
| Cross-cutting | 0 | 1 | 5 | 6 |
| suanpan | 1 | 6 | 7 | 14 |
| suanpan tests | 0 | 3 | 5 | 8 |
| Oracle and laws | 1 | 4 | 8 | 13 |
| Meter core | 2 | 1 | 1 | 4 |
| Registry and tier2 | 2 | 3 | 1 | 6 |
| The board: frame | 2 | 3 | 3 | 8 |
| The board: families, floors, judge | 1 | 7 | 3 | 11 |
| The board: ops, render, shards, worst | 0 | 7 | 2 | 9 |
| The surface roster | 1 | 6 | 3 | 10 |
| The test harness: bridge, oracles, exhaustive, validation index | 1 | 9 | 4 | 14 |
| The test harness: differential table, generators, snapshots, asymptotics | 0 | 7 | 7 | 14 |
| The envelopes, first half | 2 | 3 | 2 | 7 |
| The envelopes, second half | 1 | 5 | 4 | 10 |
| Other suites | 1 | 4 | 2 | 7 |
| Benches and examples | 3 | 1 | 4 | 8 |
| Fuzz targets, guests, and pins | 0 | 6 | 4 | 10 |
| Fuzz-fit: bands | 0 | 3 | 5 | 8 |
| Fuzz-fit: strategies | 1 | 7 | 1 | 9 |
| Fuelscape: pipeline | 1 | 7 | 4 | 12 |
| Fuelscape: render | 1 | 5 | 5 | 11 |
| Workspace tools (tools/) | 1 | 5 | 4 | 10 |
| Module graph | 0 | 3 | 2 | 5 |
| Dependencies | 0 | 4 | 3 | 7 |
| Inventory | 0 | 1 | 6 | 7 |
| Clippy pedantic | 0 | 1 | 2 | 3 |
| API audit | 0 | 1 | 0 | 1 |
| Gate legs | 0 | 2 | 0 | 2 |
| Meter adequacy | 0 | 1 | 1 | 2 |
| Paper fidelity | 0 | 0 | 1 | 1 |
| Prose hygiene | 1 | 1 | 0 | 2 |
| Recursion | 0 | 0 | 2 | 2 |
| Suite economics | 0 | 2 | 1 | 3 |

Provenance: 308 entries verified (one of them, tools-31, also executed a self-test run), 65 assessed, 3 verified by trace or gap-check; none demonstrated. Owner-gated entries: 66.
