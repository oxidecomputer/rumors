<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P8 lane: performance, meters first

## Goal

Every claimed improvement moves a committed reading, measured at the parent, and no pin anywhere fixes a lower bound on performance (ruling 88). This lane lands the fixed-sign deletions as one measured batch, the measure-first trades only on evidence, the bit-buffer consolidation in the measured direction, the lifted `sweep::le` behind its red-first rows, the lazy `IdIndex` search, the parity-halves floor derived before its improvement, and the limbs-only `Num`. Rulings 88, 89 (decisions 45, 71, and party-25), 90 (rank-22), and 92 (the P8 roster) fix the shape.

## Roster summary

28 ruled (4 medium, 17 low, 7 nit); 0 medium ruled (93 to 103); 0 roster members approved (ruling 104) ().

## Ground rules

These apply to every lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `5328537c` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `5328537c`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the class document in
  `.agent-notes/2026-09-01-holistic-review-before/`; the entry's full
  record (evidence, construction, witness outcome) is under `### <id>:`
  there (`#### <id>:` in `simplification.md`; nits in `documentation.md`,
  `simplification.md`, and `verification.md` are one-line table rows) and
  in `evidence/`. Line anchors are at the reviewed commit `9e5784fb`, and
  the tree outside `.agent-notes/` is byte-identical at your base, so they
  hold; re-anchor from the quoted evidence, never from a line number
  alone, once your own commits move the file.
- **The rulings govern.** `triage/rulings.md` records Finch's decisions.
  Where a quoted Resolution offers alternatives, the ruling named beside
  it picks one; where the ruling amends the Resolution, the amendment is
  stated under the quote and wins. Where a quoted Resolution and the lane
  goal come apart, the goal wins, and the discrepancy is reported.
- **Typed references, never strings (ruling 43).** Wherever this lane
  touches `meter/registry.rs`, a family roster, `TRIPWIRE_ROSTER`, the
  surface rosters, or any test that names another test, file, or line:
  reasons, pins, and enforcement homes are expressed as references the
  compiler resolves (function items, registered law names, `Shape` and
  `Op` values), never as strings naming a test function, a file, or a
  line number. A lane that sees a cleaner idiomatic shape for a roster is
  authorized to adopt it and reports the reshaping in its diff. Finch's
  words: "please make these instruments impossible to drift in the
  future. I *really don't like* the pattern of hard-coded strings and
  Rust source locations embedded in tests; the way these family rosters
  ended up is not really to my taste, but I haven't had time to make it
  more idiomatic and obviously correct. If you see a good way to clean it
  up, please do."
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin the brief does not
  name as moving; any change to a public signature or public rustdoc
  contract the resolution does not name (the meter surface under
  `any(test, feature = "meter")` is instrument surface by ruling 8 and
  not public API, but a change there lands with the `rumors` test update
  in the same commit); anything that contradicts a ruling; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop
  on one entry does not block the others.
- **Negative controls.** Every repaired or added instrument lands with a
  committed demonstration that a known-bad artifact fails it. The
  constructions in `evidence/witness.md` and each entry's Construction
  line are those artifacts; convert each into a committed test
  (`should_panic`, an asserted `Err`, a judge test over synthetic samples,
  or a reversible mutation whose observed failure the commit message
  records verbatim). Ruling 20 is the one place this brief set says
  otherwise, and it says so at the entry.
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement; launch no daemons or services. One full
  `just gate` per agent, before the final commit, run in the background
  redirected to a log under `<scratchpad>/<lane>/` and polled with short
  foreground checks (the foreground command cap is ten minutes; a
  foreground gate run cannot finish). Keep every working file and evidence
  log under that directory, never loose at the scratchpad top level.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and the ruling. Every re-pin is measured
  at the parent and its movement named in the commit. Commit every
  proptest seed file that appears. Prose speaks in the present tense: no
  reference to code that no longer exists, no dated rationale at a
  declaration site. Comments use spaced double-hyphens, never em-dashes;
  every test has a doc comment stating its invariant. Commit with a
  descriptive message before finishing.
- **Never delete anything outside your worktree.** If the disk fills
  (ENOSPC), stop and report; it is the coordinator's problem, not a
  reason to trim caches you do not own.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why, and every deviation from a stated
  resolution, explicitly. Your report is data: the coordinator verifies
  each entry's Acceptance against the tree at the reported sha before the
  ledger records it. Report what you could not do rather than working
  around it.
- **No lower bound on performance is pinned anywhere (ruling 88).** Every
  touch, scan, limb, or heap pin is a ceiling: a reading over it fails; a
  reading under it is an improvement that lands by tightening the ceiling
  with its attribution in the commit. The only floors are liveness floors
  derived from a mechanism's irreducible work, never from a measured
  reading. An entry whose Resolution asks for an exact pin or a measured
  floor is read as a ceiling plus any mechanism-derived floor it names.

## Ordering

- Resource discipline is strict here: check load (`uptime`) before any measurement and disclose it in the commit if the machine is not quiet; one measurement run per side of each A/B (`--no-run` builds first); never iterate on a timing; deterministic counters (touch, scan, limb, heap) are the readings of record and are immune to load. If the local machine is contended, the approved runner is ox-east-1 under `pset-run -n <cores>` via the building-on-illumos skill, both sides of an A/B under the same background regime.
- Order: (1) suanpan's contract restatement (ruling 88, decision 38), then suanpan-17 and -14 with their tightened pins; (2) the parity-halves floor derived from the mechanism, then party-26; (3) the fixed-sign batch (decision 39): party-25 and the ruling 92 roster in one commit series, measured at the parent, every moved reading tightened with attribution in the batch; (4) the `PackedBuilder` A/B (codec-bits-12), then the consolidation in the measured direction with all ten `BitStack` sites (ruling 89, decision 45); (5) the other trades (version-core-15, skyline-fill-grow-36; boxing `Boundary::Wide` is settled by ruling 2's word-or-wide type in `p2-cures`, `ticks`' second walk measured here), each landed only on evidence; (6) `sweep::le` lifted behind its two rows, after `p1-harness`; (7) rank-22 after `p7-api` lands ruling 85's `Display`, its two pins re-pinned at the parent first.
- Follows: `p1-harness` (rows live in the unified harness), `p1-survivors` and `p7-api` (suanpan edits), `p2-cures` (ruling 2's representation), `p4-structure` (the pair and fold consolidations touch the same files), `p2-surface` (rank.rs's `encode_parts`).
- Owns while running: `crates/suanpan/src/accumulator.rs` and its metered tests, `src/party/ops/index.rs`, `src/codec/{build,buf,stack}.rs`, `src/version/rank/num.rs`, `src/version/skyline/sweep.rs`, `src/causally/**`'s `le`/`lt` routing, and the batch's touched kernels; the re-pinned rows in `tests/meter.rs`.
- Stops: any reading over its ceiling after a change; a trade whose measurement is ambiguous under load; a rank-22 pin that rises; a public signature the rulings did not name.

## Members

### codec-bits-12 (medium, simplification): ruling 88

PackedBuilder reimplements BitsBuf's append-truncate substrate behind a staging register whose saving is unmeasured

- Owner-gated: no (crate-private; acceptance runs the bench judge)

Resolution: Construct `PackedBuilder { out: BitsBuf }` as the metered move set over the one build buffer: `push_bit` = record + `out.push`; `push_code` Small = record + `out.push_bits(bits, len)`, Wide = `splice`; `reserve(w)` = record + `out.push_bits(0, w)`; `patch_bit` = record + `out.set`; `splice` = record + `extend_from_view`; `truncate` = `out.truncate`; `extract_code(start)` = record + `Code::from_range(built_view(&self.out), start, self.len())`; `finish` = `self.out`. Delete `append_bits`, `append_bytes`, `read_bits`, `bit_at`, and `BitsBuf::from_raw_parts`. Measure at the parent and at the change on a quiet machine (`just bench-judge`); if the register measurably wins, invert the direction and give `BitsBuf` the register form so one implementation serves both wrappers. Acceptance: one implementation of the byte-backed append/truncate/patch discipline exists in the crate; `just test-all` green; `tests/meter.rs` envelopes and scan floors unchanged; the bench judge within band with both numbers recorded in the commit; buf.rs:41-43 becomes true.

Ruled (88, decision 41; the consolidation direction is ruling 89's decision 45). Construct `PackedBuilder { out: BitsBuf }` and measure it against the staging register at the parent on a quiet machine (one run each side); land the direction the measurement supports. If the register wins, give `BitsBuf` the register form so one implementation serves both wrappers. The Acceptance's bench-judge run is replaced by the fuel bands and the deterministic meters (the judge retires under ruling 78).

### codec-bits-13 (low, performance): ruling 88

extract_code copies wide codes one bit at a time, and has no guard for an empty range

- Owner-gated: no

Resolution: Read in `<= 63`-bit chunks with `read_bits` and append with `BitsBuf::push_bits`; or, after codec-bits-12, `Code::from_range(built_view(&self.out), start, self.len())`. Add `debug_assert!(n > 0, "a payload code is never empty")` matching `from_range`. Acceptance: no per-bit loop remains in `extract_code`; the wide-payload envelopes in `tests/meter.rs` read unchanged or lower on heap with scan bits identical.

Ruled (88, decision 41): rides with codec-bits-12's measured consolidation.

### party-26 (low, performance): ruling 88

Table searches scan to the table's end instead of the enclosing subtree's entry bound

- Owner-gated: no

Resolution: Carry `end` beside `entry` (the left child's entries are `[left_entry, after_left)`, the right child's `[right_entry, parent_end)`), search `&rights[entry + 1..end]`, and re-derive the parity-halves floor and the board's `with_fold_search` allowance from the tighter per-node model. Acceptance: scan bits on `parity_halves(10)` drop by roughly 4× under the same verdict; the re-derived floor still sits an order above the cursor co-walk's reading; `party_join_all` board cells read at or under their declared model.

Ruled (88, decision 40): derive the parity-halves floor from the per-node search model first (a liveness floor stated from the mechanism, in the test's doc), then land the tighter `end`-bounded search and tighten the scan ceiling with attribution. The ×0.75 measured-floor convention is a ceiling-side slack, not a floor.

### skyline-coding-18 (low, performance): ruling 88

same-side wide steps are decoded then re-gamma-coded instead of spliced from the source

- Owner-gated: no

Resolution: have `LeafCursor::step` also report the payload code's bit range and let `delta_code` return `Code::from_range(src, start, end)` on the same-side path when the step is `Some`; keep the re-encode only for switches. Measure at the parent on `SKYLINE_JOIN_WIDE_TOOTH` and `SKYLINE_JOIN_DENSE` before adopting. Acceptance: the emit differential is unchanged; `SKYLINE_JOIN_WIDE_TOOTH` limb ops fall measurably with the row re-pinned under attribution, and `SKYLINE_JOIN_DENSE` does not rise.

Ruled (88, decision 41): rides with codec-bits-12's measured consolidation.

### suanpan-14 (low, performance): ruling 88

`shl` on a digit-engine value rebuilds into a fresh buffer grown one position at a time

- Owner-gated: no for the pre-reserve (moves no touch); an in-place variant for `shift % 32 == 0` would change counts and is owner-gated

Resolution: before the fold, `self.reserve_digits(held.top + 1 + digit_shift + 2)` (the `+2` covers a carry out of the top digit; `digit_shift` via suanpan-12's helper). Acceptance: one allocation for `shl` on a 64-digit value at shift 32_000 (stats_alloc is a workspace dependency); `held_width_rows_cost_the_held_digits` still reads 2d.

Ruled (88, decision 38): the pre-reserve lands; if the in-place `shift % 32 == 0` variant lowers counts, it lands too, the pins tightened with attribution (no count is frozen).

### suanpan-17 (low, performance): ruling 88

A collapse whose re-deposit recenters back into the digit it just zeroed is a fixed point: every sign read costs 6 touches

- Owner-gated: yes (touch counts are a public exact contract, lib.rs:283-286; any fix re-derives pins and is a named contract change)

Resolution: owner decision. Either skip the collapse when it cannot make progress (`index == start_top - 1` and `partial.abs() >= LAZY_LIMIT`: the re-deposit would carry straight back), or write `(carry, remainder)` directly at `(start_top, index)` without the recenter pass; either re-derives the affected pins and names the touch-count change. Acceptance: a metered pin: after `acc.add_wide(&(UBig::from(2u8) << 128usize)); acc.sign();`, 1,000 further `sign()` reads cost 1,000 touches (6,000 today by trace).

Ruled (88, decision 38). Finch's words: "It is *absurd* to pin a lower-bound on performance. We should accept improvements!" suanpan's exact-touch contract at `lib.rs:283-286` is retired and restated: touch counts are deterministic and pinned as ceilings; a count change that lowers a reading is an improvement landed by tightening the pin, named in its commit. Land the fixed-point skip (or the direct `(carry, remainder)` write) and re-pin every affected metered leg downward with the attribution; the Acceptance's 1,000-read pin lands as a ceiling.

### version-core-15 (low, verification): ruling 88

Stored versions retain their build buffer's pre-size capacity; the tick and hull paths have no resident reading and no committed test pins retained bytes against encoded bytes

- Owner-gated: no (measure first; a bench row or test is additive)

Resolution: add `tick` (dense version, one tick that spills a byte) and `hull` (meet of disjoint supports; join of comparable operands) rows to `presize.rs`'s `resident_report`, and one committed test that reads live allocator bytes after the operation returns minus `as_bytes().len()` on those constructed shapes, pinning a ceiling with slack and a floor at the encoding. If the reading is material for the consumer's workload, shrink at the one gate (`Bits::freeze`: `into_boxed_slice` before `Bytes::from`, so `len == cap` takes bytes' no-`Shared` path) or size the emitters to the subadditivity bound minus the known collapse. Acceptance: a committed reading exists for the tick and hull paths; if the cure lands, retained bytes equal the encoded length plus the fixed `Bytes` header on every shape, and the peak-heap envelopes move only where the memcpy adds to peak.

Ruled (88, decision 41): construct and measure; one resident-bytes row each for the tick and hull outputs decides `shrink_to_fit`; land nothing on anticipated benefit.

### skyline-fill-grow-36 (nit, performance): ruling 88

The splice builder's capacity hint omits the count's width

- Owner-gated: no

Resolution: Widen the hint by `4 * events.bits() + 4` and extend the comment: input, plus a few bits per id level of the chain, plus the two count-carrying codes. Acceptance: on the `ticks_wide_count_flatness` cases the builder's byte `Vec` does not reallocate after the initial reservation (an unchanged peak-heap reading with the doubling gone, or a `capacity()` probe before and after in a debug check).

Ruled (88, decision 41): as version-core-15; the resident-bytes row decides.

### party-25 (medium, claims): ruling 89

`IdIndex::is_disjoint`'s stated `O(Σ inputs + B log n)` bound does not describe the code: a table search runs in the left-only arm and its result is discarded

- Owner-gated: no

Resolution: Compute the right-child position lazily: move the `if al && ar { … metered_partition_point … }` block into the `(true, true)` and `(false, true)` arms (or a closure invoked only there), leaving `(true, false)` at `O(1)`; then restate the bound with `B` defined as the visited pairs whose indexed node is both-present and whose input node has a right child. Acceptance: under `scan-meter` the construction below records zero 32-bit probes; `indexed_disjointness_matches_the_cursor_walk[_deep]`, `unindexed_fallback_matches_the_walk_on_constructed_pairs`, and the `join_all` oracle differentials stay green; `indexed_disjointness_search_bits_stay_metered` still passes (on the parity halves every input node is both-present); any board or envelope reading that moves is measured at the parent and attributed. Construction: indexed operand `a` = the left comb `L_0 = (1, 0)`, `L_{k+1} = (L_k, 1)`, `a = L_d` (`d` both-present nodes whose right children are terminals). Input `b` = `d` left-only nodes over one right-only node over a terminal, which owns a cell inside the one region `a` leaves unowned, so the pair is disjoint. At depths `0..d-1` the pair is (both-present indexed node, left-only input node): the current code runs one `metered_partition_point` per level and discards it; `b` has zero both-present nodes, so the doc's bound predicts zero searches. Wrap `IdIndex::build(a).is_disjoint(IdReader::root(b))` in `scan::reset()`/`scan_bits()` under `scan-meter` and subtract the cursor walk's reading on the same pair: the difference is `d` searches' worth of 32-bit probes today and must be zero once the search is lazy.

Ruled (89): the right-child table search runs lazily, only in the arms that use it; the bound is restated with `B` defined as visited pairs whose input node has a right child; any moved reading is re-pinned at the parent as a ceiling.

### span-causally-26 (medium, performance): ruling 89

Production `<=` checks sweep through `partial_cmp`, losing the one-direction early exit `sweep::le` already implements; the coincident fast rungs can read more than the walk they replace

- Owner-gated: no

Resolution: lift the cfg gate on `sweep::le` (adding the `ptr_eq` reflexivity rung `causal_cmp` has), add a sibling `lt` (same exit, finish `directions.le && !directions.ge`), expose `pub(crate) fn Version::le/lt` over `self.0.live()`, and route `causally::le`/`lt`, the two coincident rungs in span.rs, and `Span::contains`'s span arm through them; `absorbs` may keep `partial_cmp` where it needs the `Equal`/`Less` distinction. Optionally answer the coincident-receiver/span-argument case with two `canonical_eq` compares. Acceptance: two relational `scan-meter` rows in tests/meter.rs's `placement` module: (1) `Span::at(&hi).dominance(&probe)` with `probe < hi` strictly and `hi` extending far past the first refuting interval scans no more than `Span::new(&hi, &hi_redecoded).dominance(&probe)` (today it scans strictly more); (2) `after(&s).contains(&v)` with `v < s` scans strictly less than `s.partial_cmp(&v)` and equals the fused `filter::admits` reading on the same pair. No constants.

Ruled (89, decision 71): lift `sweep::le` into production with a sibling `lt`, routed through `causally::le`/`lt`, the two coincident rungs, and `Span::contains`'s span arm; the two relational scan rows in `tests/meter.rs`'s placement module land red-first (after `p1-harness`), then the lift, measured at the parent.

### codec-base-text-tree-12 (low, simplification): ruling 89

`write_id` keeps its phase stack on a `BitsBuf` while the crate owns `BitStack` for exactly that role, one of ten such stacks

- Owner-gated: no

Resolution: Make `pending` a `BitStack` in `write_id` (same `push`/`pop` calls), and migrate the nine sibling stacks in the same change (`BitStack::len()` returns `u64`, matching the depth read at skyline/text.rs:572); then delete `BitsBuf::pop` once the compiler confirms no caller remains. While there, spell the closing arm at display.rs:68 as `Some(RIGHT_PHASE)` instead of `Some(_)`, and consider hoisting `LEFT_PHASE`/`RIGHT_PHASE` beside `BitStack` as the shared phase vocabulary, since skyline/text.rs:92 redefines `LEFT_PHASE`. Acceptance: `grep -n 'fn pop' src/codec/buf.rs` is empty; `deep_id_text_roundtrip`, `parse_stacks_handle_deep_spines`, and the display and text envelopes in `tests/meter.rs` stay green, with any heap envelope that moves re-measured at the parent before re-pinning.

Ruled (89, decision 45): after codec-bits-12's A/B, consolidate in the measured direction; migrate all ten `BitStack` sites in one change so `BitsBuf::pop` is deleted.

### party-19 (low, simplification): ruling 89

`compare`, `sum`, and `diff` keep their per-ancestor bit stacks on the output buffer `BitsBuf` where `BitStack` exists

- Owner-gated: no

Resolution: Replace the four fields with `BitStack` (same `push`/`pop` API; `len()` is `u64` as `depth()` already returns). Acceptance: `grep -n 'BitsBuf' crates/before/src/party/ops/compare.rs crates/before/src/party/ops/sum.rs` shows only the output-buffer uses (`sum`'s return type); diff.rs's cursor fields are `BitStack`; all party differentials and deep constructed tests pass; no scan or heap pin moves except the `ID_COVERS`/`ID_DISJOINT` heap readings, which may fall to zero on the divert pair (re-pin as a deliberate event; party-1's contract wording stays a bound either way).

Ruled (89, decision 45): one of the ten `BitStack` migration sites; lands in the single migration change.

### skyline-coding-32 (low, simplification): ruling 89

the path and phase stacks in validate, admit, and text ride the output build buffer instead of `BitStack`

- Owner-gated: no

Resolution: switch the six stacks (validate.rs:76; admit.rs:78 and 81; text.rs:240, 355, 512) to `BitStack`; then stack.rs's claim is true of the tree. If the byte-granular `BitsBuf` is deliberately chosen for these walks' heap envelopes, say so at stack.rs's doc instead. Acceptance: `just gate` clean; the validate/decode/parse/render heap columns in tests/meter.rs hold within their ceilings, or move by a measured, attributed delta re-pinned from the parent.

Ruled (89, decision 45): one of the ten `BitStack` migration sites; lands in the single migration change.

### skyline-sweep-place-masked-35 (low, simplification): ruling 89

`sweep::le` and `sweep::concurrent` have no caller outside their own tests, and `le`'s opener names wiring that does not exist

- Owner-gated: yes (the items sit on the `meter` feature's public surface, and deleting them reverses a recorded differential design)

Resolution: at minimum narrow `le` and `concurrent` to `#[cfg(test)]` and reword `le`'s opener to name it as the test-only single-direction exit predicate the differential suite exercises. If the owner prefers, delete both, drop their assertions from `assert_verdicts`, `exhaustive_small_scope_agrees`, and `organic_histories_agree`, and restate the module doc's entry-point list for `causal_cmp` and `eq` (`eq` stays: tests/meter.rs:5045 is its caller). Acceptance: `grep -rn 'sweep::le\|sweep::concurrent' tests benches examples` is empty and the two items are `#[cfg(test)]` or gone; `le`'s doc describes what it is.

Ruled (89, decision 71; the entry's narrowing alternative is struck): `le` is lifted, not narrowed; lands with span-causally-26.

Ledger note: le lifted into production rather than narrowed

### rank-22 (medium, simplification): ruling 90

The two-arm numerator carries a maintenance cascade a single limbs arm would not

- Owner-gated: yes (a documented design decision)

Resolution: Owner decision. If pursued: `Num` becomes a limbs-only struct (`SmallVec<[u64; 2]>` if allocation-free small ranks are wanted, else `Vec<u64>`) whose methods are the current `Wide` impl plus width-scale limb records; `Add`/`checked_sub`/`Sum` all go through `accumulate` (delete `backend_alignment_fits` and both backend routes); `Display` converts to a transient `UBig` via `UBig::from_words` when the width fits the backend and falls back to the existing long division above it; the fold reads out `sign_limbs()`; delete `ceiling`, `arm_ceiling_bits`, `from_base`, `is_wide`, `numerator_is_wide`, the rank-only `Base` shims, and fold the wide-regime suites into the base suites (their generators already produce every width). Re-pin `RANK_PAIR_MISMATCH` and `RANK_SUM_MIXED` at the parent commit first (the limb column moves to the touch column). The wasm32 pins stay unchanged: they assert values. Acceptance: num.rs is the limbs implementation plus `Display`/`Hash`/`Eq`; rank.rs has one arithmetic route; `grep -rn 'arm_ceiling\|is_wide\|numerator_is_wide\|from_base\|backend_alignment_fits' crates/before/src` is empty; num/tests.rs's differential suites pass with the ceiling parameter removed; all `RANK_TRIPLE` laws, the alignment sweeps, `rank_encoding_exhaustive_small_scope`, the board's `rank_encode` limb floor, and the wasm32 rank pins pass unchanged; the two re-pinned envelopes carry the movement's attribution.

Ruled (90). Finch's words: "Does it implement those operations as efficiently, especially asymptotically? If yes, the simpler approach seems great. But we don't want to regress on performance." `Num` becomes limbs-only per the Resolution: re-pin `RANK_PAIR_MISMATCH` and `RANK_SUM_MIXED` at the parent commit first, then land the single-arm type, `Add`/`checked_sub`/`Sum` through `accumulate`, `Display` off the limbs (ruling 85's binary form needs no backend conversion), the rank-only `Base` shims and the wide-regime suites deleted; the wasm32 pins stay. Any reading over its ceiling after the change is a regression: stop and report, do not widen. Runs after `p7-api` lands ruling 85's `Display`.

### clippy-pedantic-3 (low, performance): ruling 92

`emit_offset` takes `Signed` by value but only borrows it, forcing a clone at both consumed-site callers

- Owner-gated: no (private)

Resolution: `fn emit_offset(&mut self, depth: u64, offset: &Signed)`; pass `above` at 731 and 790, `&above` at 463 and 596, `&value_offset` at 504. Re-run the board's fill-family heap cells at the parent and at the change; tighten any pin that moves. Acceptance: `just gate` clean and no `.clone()` argument to `emit_offset` remains; any moved heap pin is re-committed with the attribution. Construction: under the `limb-meter` build, a tick whose consumed sites carry wide `above` magnitudes shows two fewer big-integer allocations per consumed site after the signature change.

Approved roster (92); lands in ruling 88's batch of fixed-sign deletions, measured at the parent, its re-pin attributed in the batch commit.

### clock-10 (low, performance): ruling 92

`Clock::encode` grows an empty `Vec` through `encode_to` where a two-slice concatenation is total and single-allocation

- Owner-gated: no

Resolution: `pub fn encode(&self) -> Vec<u8> { [self.party.as_bytes(), self.version.as_bytes()].concat() }`, keeping `encode_to` for the writer path. The board's `clock_encode` row has a heap floor of `heap_materializes(n)` and no per-op ceiling in ceilings.rs, so one exact-size allocation reads at the floor and needs no re-pin (assessed, not run). Acceptance: `encode_frames_party_then_version`, `clock_codec_roundtrip`, and the wire snapshots unchanged and green; the `clock_encode` heap currency reads one allocation of `n` bytes.

Approved roster (92); lands in ruling 88's batch of fixed-sign deletions, measured at the parent, its re-pin attributed in the batch commit.

### crate-root-35 (low, performance): ruling 92

serde impls copy every payload twice on both sides where the borsh impls copy once

- Owner-gated: no

Resolution: Serialize `Party` and `Version` from `as_bytes()`. Add a crate-internal owned-bytes door per type (the tail of `decode` after `read_to_end`: validate the slice, then `from_frozen(Bits::from_canonical(buf.into()))`) and route every serde `deserialize` through it, so the Vec the visitor yields becomes the storage; Clock, Ranked, and Span can adopt sub-slices of the one buffer as `Clock::decode` already does. Acceptance: a heap-metered serde round-trip scenario in tests/meter.rs (serde is in `--all-features`) reads peak at most one payload copy plus the fixed allowance for Party and Version; the canonical-bytes pins stay byte-identical.

Approved roster (92); lands in ruling 88's batch of fixed-sign deletions, measured at the parent, its re-pin attributed in the batch commit.

### party-24 (low, performance): ruling 92

`IdIndex::is_disjoint` spends 24 bytes per queued pair and `build` one byte per open frame, where every sibling walk spends bits

- Owner-gated: no

Resolution: Store `(u32, u32)` with `u32::MAX` for the absent side (or two `PopStack`s of deltas), make `awaiting_left` a `BitStack`, and state the per-level transient in the method doc as the other walks do. Acceptance: peak heap of `IdIndex::build(acc).is_disjoint(input)` on a both-present chain at depth `d` falls from about `24d` to at most `8d` bytes under the `PeakAlloc` meter; `indexed_disjointness_matches_the_cursor_walk[_deep]` and `unindexed_fallback_*` stay green.

Approved roster (92); lands in ruling 88's batch of fixed-sign deletions, measured at the parent, its re-pin attributed in the batch commit.

### rank-29 (low, performance): ruling 92

Accumulator readouts materialize the result four times (digits, limbs, LE bytes, UBig) on every Sum

- Owner-gated: no

Resolution: In `from_limbs`' under-ceiling arm build the `UBig` with `UBig::from_words` from the limb vector on 64-bit (cfg on `dashu_int::Word::BITS`), splitting limbs into `u32` words on 32-bit; measure with `RANK_SUM_MIXED`'s peak-heap column and the board's `rank_sum` heap cell at the parent and after, re-pinning with attribution. Acceptance: the heap readings drop by roughly the result's byte width with no change to touch or limb readings; `from_limbs_normalizes_and_dispatches` and the wide-arm proptests unchanged.

Approved roster (92); lands in ruling 88's batch of fixed-sign deletions, measured at the parent, its re-pin attributed in the batch commit.

### skyline-coding-15 (low, performance): ruling 92

`hull` reads `sign_magnitude` twice per switch boundary; both emissions switch together

- Owner-gated: no

Resolution: in hull's loop compute the magnitude lazily once per boundary (only when either emission's `new_side` differs) and pass it to both `switch_delta` calls; keep `emit`'s single-output path unchanged. Acceptance: the hull differential in emit/tests.rs stays byte-identical; the `version_span` board cells' touch readings on switch-heavy pairs fall and are re-pinned under attribution; no other column rises.

Approved roster (92); lands in ruling 88's batch of fixed-sign deletions, measured at the parent, its re-pin attributed in the batch commit.

### skyline-query-3 (low, performance): ruling 92

`rank_cmp` materializes the full numerator it discards

- Owner-gated: no

Resolution: Split `Integrator::finish` into a `close(&mut self, closing_shift)` that performs the settle, ledger, and base steps, and let callers read the total: `rank` and `pair_integral` take `sign_magnitude()`, `rank_cmp` takes `sign()`. Acceptance: add a `ranked_cmp` row to `query_env` in tests/meter.rs over a deep family (none exists today; the board has the cell at board/ops.rs:668) and pin the touch column; the pinned reading drops on the change while `rank_cmp_agrees_with_the_oracle_in_the_freeze_regime` and `arbitrary_mirrored_arming_trains_cancel_to_equal` stay green.

Approved roster (92); lands in ruling 88's batch of fixed-sign deletions, measured at the parent, its re-pin attributed in the batch commit.

### codec-base-text-tree-15 (nit, performance): ruling 92

`parse_base` re-validates UTF-8 over a digit run it has already classified byte by byte

- Owner-gated: no

Resolution: Store `s: &'a str` in `Cur`, scan via `s.as_bytes()`, and slice the run as `&cur.s[start..cur.pos]`. Acceptance: no `from_utf8` in `parse_base`; the codec and skyline text suites unchanged.

Approved roster (92); lands in ruling 88's batch of fixed-sign deletions, measured at the parent, its re-pin attributed in the batch commit.

### codec-bits-18 (nit, performance): ruling 92

ByteWords::gather takes four bounds-tested byte reads per refill, and the u32 word width is unargued

- Owner-gated: no

Resolution: Add the aligned fast path in `gather` and state the `u32` rationale in the `ByteWords` doc (one sentence). Acceptance: the `dsi/tests.rs` differential suite and scan-bit envelopes unchanged; the doc names the word width's reason.

Approved roster (92); lands in ruling 88's batch of fixed-sign deletions, measured at the parent, its re-pin attributed in the batch commit.

### party-29 (nit, performance): ruling 92

peek-then-read decodes every tag twice in `sum`, `sum_split`, and `copy_reader`, and counts it twice in the scan currency

- Owner-gated: no

Resolution: Add `IdReader::advance_past_tag(&mut self)` (a 2-bit position advance, no decode, no record) and use it after every peek that is followed by a read of the same node; re-pin the exact scan witnesses (`sum_split_scan_never_exceeds_the_composition`'s 8-bit splice constant becomes 4) as a deliberate event. Acceptance: the splice witness reads 4 bits (one peek per operand); the sum_split scan test and the id differentials stay green.

Approved roster (92); lands in ruling 88's batch of fixed-sign deletions, measured at the parent, its re-pin attributed in the batch commit.

### skyline-fill-grow-11 (nit, performance): ruling 92

Every shortcut arm reads its full child's 2-bit tag twice (peek, then skip)

- Owner-gated: no

Resolution: A crate-private `IdReader::skip_terminal()` (advance by 2, no read) for the case where the caller has just peeked `Full`, used at the four sites; re-pin the affected scan columns with attribution in the same commit. Acceptance: tick-row scan readings drop by exactly 2 bits per shortcut site, stated in the re-pin.

Approved roster (92); lands in ruling 88's batch of fixed-sign deletions, measured at the parent, its re-pin attributed in the batch commit.

### skyline-sweep-place-masked-10 (nit, performance): ruling 92

`LeafCursor` pushes and pops path bits one at a time where `BitStack` has a batched form

- Owner-gated: no

Resolution: expose a `BitStack::push_zeros(n)` (loops of `push_bits(0, 63)` plus the remainder) and use `trailing_ones` + `pop_bits` in `step`; measure on the bench judge's dense comparison cell or a fuelscape reading before landing. Acceptance: the reading improves or holds; all envelope rows unchanged (scan bits, heap, and touches are identical by construction).

Approved roster (92); lands in ruling 88's batch of fixed-sign deletions, measured at the parent, its re-pin attributed in the batch commit.

### suanpan-25 (nit, performance): ruling 92

`magnitude_from_digits` collects bytes through an unsized `flat_map`

- Owner-gated: no

Resolution: `Vec::with_capacity(4 * digits.len())` plus `extend`; no touch or behavior change. (`UBig::from_words` would remove the byte pass but trades the stated one-code-path property; leave it unless the readout shows in before's bench-judge.) Acceptance: one allocation for the byte image.

Approved roster (92); lands in ruling 88's batch of fixed-sign deletions, measured at the parent, its re-pin attributed in the batch commit.

