<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P4 lane: one mechanism, one home

## Goal

Each mechanism the review found spelled several times has one spelling:
the stored-code walk and height decode in the board's operand module,
the suspended-ancestor frame bits in the fill walks, the pair-tracking
state and its seed in the comparison layer, the arity-N advance law,
the strict skyline parser, the watermark's undercut tail and latent
ladder, the receiver-seeded two-sided fold, the id tag read and subtree
skip, and the fuzz framing constants. Every extraction is
behavior-preserving: the same bits are pushed, the same touches
recorded, the same verdicts returned, and the envelopes re-measured at
the parent are unchanged unless the entry names a movement. Rulings 72
(two entries), 74 (one), 75, 76, and 77 fix each shape; this brief
carries them, plus the simplification-class lows and nits of P4 as a
roster pending approval.

## Ground rules

These apply to every lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `10cdd255` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `10cdd255`, fast-forward; if it has diverged, stop and report. Never call
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
- **Prose (ruling 105).** Every paragraph you touch passes the three tests in `PROSE.md` (altitude, concision, legibility); the reviewer applies its checks; the diff is net shorter in prose unless your report says what the additions buy.
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

## Ordering

These are code changes in kernel files. Run after `p2-cures` has landed
for any file it touches (`fill.rs`, `masked.rs`, `overlay.rs`, `build.rs`,
`ranked.rs`, the `causally` module), rebasing otherwise; the fill
`FrameBits` extraction (skyline-fill-grow-27) precedes ruling 2's
memo-heap representation change only if this lane lands first, and the
coordinator says which. tests-other-22 and tests-other-18 depend on
`p1-fuzz` (regenerated seeds) and edit the fuzz targets; land them last.
party-4's `build_split` re-pin lands with party-27's attribution in one
commit. This lane owns `operand.rs`, `defect.rs`, `fill.rs` and its
prescan sibling, `place.rs`, `filter.rs`, `overlay.rs`, `admit.rs`,
`shape.rs`, `validate.rs`, `watermark.rs`, `algebra.rs`, `idbits.rs`, the
party ops, and `fuzz/framing.rs`; `p4-ghosts` leaves their prose alone.

## Hazards and stops

- An extraction that changes any committed touch, scan, limb, or heap
  reading is a stop unless the entry names the movement (party-27's
  spine; skyline-coding-33's possible heap change, which is measured at
  the parent and attributed).
- `CheckedCursor` becoming the one parser must keep the reject corpus,
  the admission genres, the fuzz-seed span rejects, and the borsh span
  tests green unchanged; any verdict change is a stop.
- The n-ary span laws and the `span_all`/`join_all` envelopes hold
  byte-identical after span-causally-9.

## Members

### board-families-floors-judge-24 (medium, simplification): ruling 72

operand.rs writes one preorder walk four times and the zigzag decode twice, duplicating `skyline::signed::unzigzag_base`

Resolution: One private `fn stored_leaf_codes(v: &Version) -> impl Iterator<Item = Base>` (or a `Node::Internal | Node::Leaf(Base)` iterator when the topology is needed) in operand.rs, with `stored_nonzero_deltas`, `mandatory_limbs_stream`, and `stored_bases` pass 1 as folds over it; one `fn stored_heights(v)` applying `unzigzag_base` (widened to `pub(crate)`) so `value_content_bytes` is a sum over it and `stored_bases` collects it; route defect.rs's `last_leaf_flag_pos` and testing/bridge.rs (another partition) through the same pieces. For the count walks the public `Version::shape()` iterator is an alternative source (`v.shape().skip(1).filter(|p| p.rise.is_some()).count()` is `stored_nonzero_deltas`); the raw code width `mandatory_limbs_stream` needs is the one quantity that argues for keeping a code-level iterator. Acceptance: `grep -c 'pending += 2' operand.rs` is 1 (or 0 with a skyline-side iterator); `grep -c '>> 1u32' operand.rs` is 0; `limb_floor_derivations_split_on_plateaus_and_coincide_on_a_leaf`, `radix_units_match_hand_counts`, `mandatory_limbs_match_hand_counts`, and the smoke board pass unchanged.

Ruled (72): one stored-code walk and one height decode; route `defect.rs` (board-frame-22) and `testing/bridge.rs` through them.

### board-frame-22 (low, simplification): ruling 72

Five hand-rolled preorder decoders of the version stream under `meter/board/`, two re-implementing the zigzag rule; three are dissolvable onto the public shape walk

Resolution: In operand.rs express `stored_nonzero_deltas` as `v.shape().skip(1).filter(|p| p.rise.is_some()).count()` and the two absolute-height reconstructions as a running sum over `rise`; add one private `stored_leaves(v) -> impl Iterator<Item = (flag_pos, code, next_pos)>` for the code-width floor and defect.rs's last-leaf position; delete both zigzag re-implementations. The operand.rs functions are outside this partition's file list; the pattern is reported once here. Acceptance: exactly one `// skyline flag: 0 internal, 1 leaf` loop remains under `meter/board/`; `radix_units_match_hand_counts` and `mandatory_limbs_match_hand_counts` pass unchanged; no `>> 1u32` zigzag arithmetic remains in operand.rs.

Ruled (72): lands with board-families-floors-judge-24.

Ledger note: lands with board-families-floors-judge-24

### party-4 (medium, simplification): ruling 74

The 2-bit tag decode and the id subtree skip are hand-spelled at six sites

Resolution: In `idbits`, add two `pub(crate)` free functions and route every site through them: `tag(bits, pos) -> IdNode` (the existing private `IdReader::tag`, recording its own 2 bits) and `subtree_end(bits, at) -> u64` (the id instantiation of `skip_subtree`, recording per step). `IdReader::{read, peek, skip}` call them; `diff::{enter, consume}`, `IdIndex::is_disjoint`, `split::subtree_end`, `sum_split::branch_children`, and `grow::{id_tag, id_skip}` become one-line calls. `build_split`'s spine loop is the one site whose routing changes a committed reading; take it in the same commit as party-27's re-pin or leave it raw and state the exemption there. Acceptance: `grep -n '\.bit(' crates/before/src/idbits.rs crates/before/src/party/ops/*.rs crates/before/src/version/skyline/grow.rs` shows tag-bit reads only inside the two shared helpers (plus `build.rs:257`'s debug assert if kept); every scan-meter test and board scan pin reads an identical number except where the spine newly records.

Ruled (74): two shared `pub(crate)` helpers in `idbits`; `build_split`'s spine is the one routing that changes a committed reading, so it lands with party-27's re-pin measured at the parent and attributed in the same commit.

### skyline-coding-33 (medium, simplification): ruling 75

two strict skyline parsers implement the same canonical-form obligations

Resolution: make `CheckedCursor` the one strict parsing cursor (move it to validate.rs or a shared sibling; admit imports it) and express `validate_from` as: open a `CheckedCursor`, fold the first payload positively into a height `Accumulator`, loop `while !done { step()?; fold; if the step's sign is Negative and height.sign() is Less, return NotCanonical }`, then `finish()`; the touch sequence (fold, then a sign read only after a negative delta) is unchanged. Alternatively adopt `validate_from`'s interleaved single-stack layout inside `CheckedCursor` first, so the admission walk's transient shrinks by one buffer. Either way, extend the planted-pair proptest to drive the admission entry too (skyline-coding-6). Acceptance: `just gate` clean; the tests.rs reject corpus, the span/tests.rs admission genres, the tests/fuzz_seeds.rs Span rejects, and the borsh span tests pass unchanged; the `DECODE_*`/`SKYLINE_DECODE_*` envelopes and the span scan/touch legs in tests/meter.rs hold, or any heap movement from the layout change is measured at the parent and re-pinned with attribution.

Ruled (75): `CheckedCursor` becomes the one strict parser; extend the planted-pair proptest to the admission entry. Any heap movement from the layout change is measured at the parent and re-pinned with attribution; a movement in any other envelope is a stop.

### skyline-fill-grow-27 (medium, simplification): ruling 75

`Frames`/`PreFrames` and `Frame`/`PreFrame` duplicate the suspended-ancestor control-bit discipline

Resolution: Extract the control bits into one type in fill.rs, e.g. `FrameBits { site, phase, aux: BitStack }` with `len`, `top() -> Option<Frame>`, `aux_top`, `push(site: bool, aux: bool)`, `flip_to_await_right`, `pop`, and one `Frame` enum. `Frames` composes it with `values: PopStack` plus `keys: DeltaReg` and keeps the cost encode/decode; `PreFrames` composes it with `values` plus `slots`. Acceptance: one `Frame` enum and one `top()` in the fill module; `PreFrame` and the duplicated bodies gone; `just gate` clean with no envelope movement.

Ruled (75): as stated; any re-pin a layout change causes is measured at the parent and attributed in the commit.

### skyline-sweep-place-masked-15 (medium, simplification): ruling 75

Pair-tracking state and its seeding are spelled at four production sites while `OpenedPair` claims one home

Resolution: hoist filter's `Pair` to sit beside `Directions` in sweep.rs (or beside `OpenedPair` in overlay.rs) with `Pair::open(a_first, b_first)`, `read`, `relation`; `OpenedPair::open` seeds through the same constructor (or a shared `seed_diff`), place's `BoundSide` becomes `{cursor, pair}`, and admit's seed calls the shared function. Optionally `sweep::sweep` and `masked::Walk::run` hold a `Pair` instead of parallel `diff`/`directions` locals. Acceptance: one production site folds `Sign::Negative` into a fresh difference; the placement identity rows in tests/meter.rs are unchanged (the write sequence is identical).

Ruled (75): as stated; any re-pin a layout change causes is measured at the parent and attributed in the commit.

### skyline-sweep-place-masked-7 (medium, simplification): ruling 75

The arity-N advance law is stated twice: `advance_set` and `shape::advance_refinement`

Resolution: implement `CursorSet` for a slice of `Refine` walks (priority `0..len`, `depth(slot) = self[slot].depth()`, `step(slot) = self[slot].advance()`; give `advance_set` a `?Sized` bound or wrap the slice) and reduce `advance_refinement` to the all-done check followed by `advance_set`; or dissolve `Refine` into `CursorSet` outright. Then either make "exactly two" true by construction or drop the count and name the faces, and say that `admit.rs` restates the binary law for fallibility. Acceptance: `grep -rn 'tied boundaries close to one shared flip level' crates/before/src` hits overlay.rs and admit.rs only; the `shape` snapshots and `combine` tests are unchanged.

Ruled (75): as stated; any re-pin a layout change causes is measured at the parent and attributed in the commit.

### skyline-sweep-place-masked-33 (medium, documentation): ruling 76

sweep.rs names `Version`'s `PartialOrd` as the verdict oracle, but that is the sweep itself

Resolution: re-denominate both sentences against what is: the recursive oracle (`oracle::Version`, the paper transcription, reached through `testing::bridge::to_oracle_version`) is the verdict witness, over the exhaustive small scope, the generator families, and the organic histories, as sweep/tests.rs:1-10 already says. Drop "stored-form comparison" from sweep.rs. Acceptance: `grep -n 'stored-form comparison' sweep.rs` returns nothing; the Testing section names the oracle sweep/tests.rs calls.

Ruled (76): as stated.

### skyline-watermark-18 (medium, simplification): ruling 76

Four undercut tails, one with drop_below's follower loop hand-inlined where the polarity bug lived

Resolution: give `undercut` the lease-after order (`let mut residue = core::mem::take(&mut self.gap); residue.negate(); self.drop_below(residue, on_die); self.gap = self.lease();`) and state the order in its doc; replace `emit_here`'s tail (921-924) with `self.undercut(|()| ())`; in `emit_offset` make the dominated arm `fold_signed_int(&mut self.gap, offset.sign, &offset.magnitude);` and give both undercut arms the one shared tail `self.undercut(|()| ()); fold_signed_int(&mut self.gap, offset.sign.negate(), &offset.magnitude);`. The comment at 970-971 becomes "`gap` holds `v − A` with no latent, so `drop_below`'s residue is `m − v`". Acceptance: one undercut tail in the file; `a_dominated_undercut_subtracts_its_residue_from_live_followers`, `dominated_latent_annihilates_into_the_undercut_residue`, and the `fill/tests.rs` differentials green; under `--features limb-meter` the `dominated_undercut_cost` floor and ceiling hold and `seam_stop_pool_misses_stay_at_warmup_across_churn_doubling` reads equal misses (a lease moved after a retire can only lower them, never below the first arming's one).

Ruled (76): as stated.

### skyline-watermark-19 (medium, simplification): ruling 76

The latent-ladder decision is written three times

Resolution: add a private `fn cmp_min(&mut self) -> Ordering`, documented as "the ordering of `v` against `m` for the `v − A` that `gap` currently holds; may retire the latent (a funded collapse)": read `gap.sign()`; with no latent return it; with a latent map `Equal | Greater => Greater`, and on `Less` return `Greater` if the ladder refuses, `Less` if the latent survived (dominated), else `gap.sign()`. Then `undercuts_here` is `self.cmp_min() == Ordering::Less`, `emit_offset`'s fold path is one fold, one `if self.cmp_min() != Ordering::Less { restore; return }`, one undercut tail (finding 18), and `compare_above`'s latent branch is `let sign = self.cmp_min();`. Update `undercuts_here`'s doc (the shared form is anchor-relative, not `v = h`). Acceptance: `watermark/tests.rs`, the latent-ladder differentials in `fill/tests.rs`, and `skyline_min_ticks_latent_ladder_is_flat_per_unit` pass unchanged; `decide_undercut_through_latent` has no call site outside `cmp_min`; `watermark.rs` is net smaller.

Ruled (76): as stated.

### span-causally-9 (medium, simplification): ruling 76

The receiver-seeded two-sided fold is written twice, with `FoldInput`, the group enum, and the adjacent-clone dedup re-declared in each file

Resolution: give the two-sided fold one home. One shape: a private `Endpoints` trait (`lo()`, `hi()`, `point()`) implemented for `Span<'_>` and for a `Version`-wrapping newtype (lo = hi = the version, `point()` always `Some`); a shared `FoldInput<'r, T>` and `DedupRuns` keyed on `(&Version, &Version)` (the one-sided folds pass `|v| (v, v)`); one `fold_endpoints<T: Endpoints>(receiver, items, &SpanFoldOps)`. `Version::span_all` becomes a call into it with items wrapped, keeping its stable `Borrow<Version>` item convention; `Hull` and the second `FoldInput`/dedup dissolve. Acceptance: `grep -rn 'weight-0 lone input never sits below' crates/before/src` returns one production site (or two, if `balanced_fold` keeps its one-sided copy); `enum FoldInput` and the dedup adapter are each declared once; `span_all_is_the_family_hull`, `span_union_of_points_is_span_all`, and the n-ary span laws stay green; the `span_all`/`join_all` envelopes in tests/meter.rs are re-measured at the parent and unchanged (the change deletes no work and adds none).

Ruled (76): one fold home; the `span_all` and `join_all` envelopes are re-measured at the parent and must be unchanged.

### surface-roster-6 (medium, verification): ruling 77

Five writer-sink rows carry no resolvable name on any leg, and the only evidence behind `Span::encode_to` cannot see endpoint order

Resolution: extend `encode_to_matches_encode` (codec/tests.rs:812) to `Rank`, `Ranked` (both doors), `Span` with distinct endpoints, and `Version::encode_rank_to`, and cite it from the five rows (through `encode_to_row`, or a sibling helper for the rank doors). Then add a roster-level floor in surface_coverage/tests.rs: every row carries at least one resolvable name across its legs and payloads (a citation, a `pins` element, a `license`, a `guard`, or a `bound_at`), so a zero-binding row reads red by construction. Acceptance: the construction below turns the extended proptest red; the new floor fails on the current roster (five rows) and passes once the citations are added. Construction: edit crates/before/src/span/wire.rs:72-73 to `self.hi.encode_to(writer)?; self.lo.encode_to(writer)`. Run `just test-all` and the doctest leg: every roster and coverage test, the doctest at wire.rs:62-70 (lo == hi), and the serde and borsh suites stay green.

Ruled (77): `dup` of gate-legs-8 (the gate lane, ruling 50); its exposure clause is refuted by the witness. Nothing to do here; listed so the lane knows the roster half is owned elsewhere.

Ledger note: carried by gate-legs-8 (ruling 50); exposure clause refuted

### tests-other-18 (medium, verification): ruling 77

The `fuzz_decode_ops` seeds are held byte-identical to the derivation but never re-parsed under the target's framing

Resolution: Add `decode_ops_seeds_decode_per_framing`: for each `fuzz_decode_ops` seed, split flavour and length exactly as the target's `run` does, assert the value bytes decode as a `Clock` and re-encode identically, assert flavour-0 script bytes are each below the op-table span and together cover it (the claimed full lap), and assert flavour-1's tail decodes as a `Version` with the relation the derivation claims (after tests-other-26 fixes that relation). Share the span constant with the target per tests-other-22. Also list the ops and laws contract tests in the module doc (lines 7-10 omit both). Acceptance: changing the target's length prefix to two bytes, or dropping an index from the seed's op script, turns `fuzz_seeds` red naming the seed; HEAD (after the seed regeneration) is green. Construction: Edit fuzz_decode_ops.rs so `drive_clock` dispatches on `op % 7`. The committed `clock_then_ops` seed's trailing byte 7 now aliases `tick`; the seed still matches its derivation byte-for-byte and the directory census is unchanged, so `committed_seeds_match_the_live_derivation` and `seed_directories_hold_exactly_the_set_of_record` stay green and no test observes that the seed no longer drives the op it was written for.

Ruled (77): add the per-framing decode test; depends on tests-other-22's shared framing and on tests-other-26's regenerated seeds (`p1-fuzz`).

### tests-other-22 (medium, verification): ruling 77

The `fuzz_laws` framing constants are transcribed across the workspace boundary and bound only by comments

Resolution: Move the framing (`ARITY_SPAN`, the three pool sizes, `chunk`, `picks`, and the ops target's op-table span and flavour mask) into one file, e.g. `crates/before/fuzz/framing.rs`, included by `#[path]` from the fuzz targets, `fuzz_seed_set.rs`, and `fuzz_seeds.rs` (the same mechanism that already shares the derivation between the example and the test); `laws_chunk`/`laws_script` become thin callers. Acceptance: `grep -rn 'ARITY_SPAN: usize = ' crates/before` returns one line; changing it there fails `laws_seeds_decode_per_framing_and_stay_wide` (the arity-17 seed goes out of band) instead of passing. Construction: Edit fuzz_laws.rs to `const ARITY_SPAN: usize = 16;`. `fuzz_seeds` stays green (its own 18 still admits 17), while under the target the `laws_wide_gamma` seed's version script (arity byte 17) folds to arity 1 and its party script (16) to arity 0, so the seed no longer represents the second-octave crossings the corpus docs claim, with no gate leg red.

Ruled (77): one `fuzz/framing.rs` included by `#[path]` from the targets, the seed writer, and the checker. The fuzz targets are the fuzz lane's files; land after `p1-fuzz` (which regenerates the seeds under ruling 50) and rebase onto it.

## Roster members approved (ruling 104)

Lows and nits approved as this lane's roster by ruling 104. Land each per its quoted Resolution and Acceptance, swept with the ruled members; report rather than choose if a Resolution conflicts with a ruling or offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89).

### board-families-floors-judge-5 (low, simplification): roster: approved (ruling 104)

Generator minimum widths duplicated as bare literals at the family call sites

Resolution: Have each generator export its minimum as a named `pub(crate) const` used by both its `assert!` and the family arm's clamp, or move the clamp into the `Shape` constructor so the board never needs to know it. Acceptance: no bare `.max(<literal>)` remains in `FamilyData::build`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-ops-render-23 (low, simplification): roster: approved (ruling 104)

`check_with` re-validates the pin table's structure at runtime; the committed test already does

Resolution: Drop the loop and the corresponding `# Panics` clauses, or keep it with a one-line comment naming the reason it must hold without the test suite (the pin recipe running standalone). Acceptance: either the loop is gone and the two `# Panics` sections no longer mention a malformed pin, or the loop carries its standalone justification.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-ops-render-28 (low, simplification): roster: approved (ruling 104)

Six probe tests hand-build `Sample`s with per-test `PROBE_NA` constants and repeated in-function imports; the radix-work formula and a trivial wrapper are duplicated

Resolution: One module-level `fn probe(reason: &'static str, denom: usize, readings: ByCurrency<Option<u64>>) -> Sample` with all-NA floors and defaults, and per-test one-line tweaks; hoist the shared imports to the top (keeping only the limb-meter names gated); replace `version_of(&x)` with `x.version()`; expose measure's radix-work computation as a small `pub(super) fn` or add one test-local helper. Acceptance: one `Sample {` literal in tests.rs; no `use super::` inside test bodies except cfg-gated ones; `version_of` gone.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### codec-bits-28 (low, simplification): roster: approved (ruling 104)

Dead `len == 64` arm in BitStack::push_bits, the sibling of the disjunct 35a09c5b swept from PackedBuilder::append_bits

Resolution: Mirror 35a09c5b: `debug_assert!(len <= 63 && value >> len == 0);` and `self.top = (self.top << len) | value;`. (The larger alternative, making `push_bits`/`pop_bits` total on `1..=64` and deleting `PopStack`'s four `width == 64` splits, is an open question below.) Acceptance: no `len == 64` text remains in `push_bits`; `bit_stack_matches_a_vec_of_bools` and `pop_stack_matches_a_vec_model_across_all_widths` pass.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### envelopes-b-26 (low, simplification): roster: approved (ruling 104)

Derived liveness floors are hand-computed literals beside inline scale literals

Resolution: name each family's large-run scales (`PURE_COMB_SITES`, `PURE_COMB_WIDTH_BITS`, and so on), use them in the bodies, and write each floor as a `const` expression over them (`2 * (K - 1) + B / 64`, `4 * (K - 1) + B / 64`, `(K - 1) + B / 64`, `K * (3 * B / 64 + 2)`) with the premise in the doc and the numbers gone. Acceptance: every touch liveness floor in `width_circulation_cost` and `dominated_undercut_cost` is a const expression over named scale constants the body uses; the "At (k, b) = ..." numeric restatements are gone. Construction: change `tick_run(..., packed2(2_000, 4_096), ...)` at 9054 to `(2_000, 8_192)`: the derivation says the floor should read 4·1,999 + 128 = 8,124; the literal stays 8,060; the test still passes with the floor 64 touches under irreducible work and nothing announces it.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuelscape-render-13 (low, simplification): roster: approved (ruling 104)

Test fixtures duplicated verbatim, and hand-rolled temp dirs that leak on failure

Resolution: move `synthetic_atlas` (and a JSON `tamper` helper, see finding 14) into one `#[cfg(test)]` module both suites import; add `tempfile` as a dev-dependency and replace the four temp-dir idioms with `TempDir::new()`, dropping the trailing `remove_dir_all` calls. Acceptance: one `synthetic_atlas` definition; no `std::env::temp_dir()` in the crate's tests; a deliberately failing assertion leaves nothing behind.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-coding-11 (low, simplification): roster: approved (ruling 104)

`held_at` outlived the runtime gate it was introduced for

Resolution: inline the conjunction into the `debug_assert!` (`self.held.is_some() && self.path.len() == root_depth + first_rel_depth`), delete `held_at` and the two `held_at_*` tests, and let the tick/fill/grow differentials carry the precondition. Acceptance: `just gate` clean; no reference to `held_at` remains; the grow/fill differential suites are unchanged.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-fill-grow-34 (low, simplification): roster: approved (ruling 104)

grow.rs re-spells `IdReader`'s cursor as `id_tag`/`id_skip` over a bare position

Resolution: Have `emit` and grow/tests.rs's `rec` take an `IdReader`: `let key = id.pos(); match id.read() { IdNode::Full => .., IdNode::Internal { left, right } => .., IdNode::Empty => unreachable!(..) }` and `id.skip()` in place of `id_pos = id_skip(id_bits, id_pos)`; delete `id_tag` and `id_skip`. The expansion-chain loop's `current = (key, left_present, right_present)` tuple threading (577-602) collapses to reading the tag at the loop head. Acceptance: `id_tag`/`id_skip` gone; the route differential and both grids green with `FAMILY_GROW_PAIRS`/`EXHAUSTIVE_GROW_PAIRS` unchanged; scan-meter tick envelopes unchanged (both spellings record 2 bits per tag).

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### suanpan-9 (low, simplification): roster: approved (ruling 104)

The register-or-digit-0 dispatch is spelled out at five sites

Resolution: `#[inline] fn add_word_scale(&mut self, delta: i128) { if !self.quick_add(delta) { self.add_at(0, delta); } }` called from the four entries and `fold_accum`; optionally `fn spill_value(&mut self, value: i128, shift: u64)` for the four enter-then-deposit pairs (`spill` generalized by a shift). Acceptance: one `self.add_at(0,` site; the metered pins unchanged.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### suanpan-tests-6 (low, simplification): roster: approved (ruling 104)

the stream-replay loop is copied at eleven sites and the run-forming arm bodies are copied across two files

Resolution: add `fn replay(ops: &[Op], engine: bool) -> (Accumulator, IBig)` to tests.rs (the body of `build_held`'s `Stream` arm) and use it at every plain-replay site. Lift the run-forming arm into tests.rs as `enum RunFormingOp` with `arb_run_forming_op()` and `apply_run_forming(acc, oracle, &op)`; either keep two properties drawing from that one definition, or fold the differential one into the ledger proptest with a `sign_every_step: bool` parameter so both sign-read schedules survive under the checker. Acceptance: one definition of the replay loop and one of the run-forming arms; both suites green; the per-step-sign schedule still runs.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### suanpan-tests-9 (low, simplification): roster: approved (ruling 104)

the ledger alphabet is a u8 matched against literals, with a hand-maintained cardinality and a catch-all arm

Resolution: introduce `#[derive(Clone, Copy, Debug)] enum LedgerOp { AddOne, SubOne, AddMax, SubMax, AddAt96, SubAt96, AddAt224, SubAt224, SubWord32At192, AddWord32At192, SignRead }`, a `const ALL: [LedgerOp; 11]` (or derive the list), an exhaustive `match`, and `schedule: Vec<LedgerOp>`. Acceptance: `ledger_op` has no `_` arm; `LEDGER_OPS` is gone; the sweep explores the same state count (check once in a scratch run that the number of `assert_ledger_invariants` calls is unchanged).

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### testing-diff-gen-16 (low, simplification): roster: approved (ruling 104)

The op applier is spelled twice in `optrace.rs` and twice more elsewhere; the module doc's op list omits `Ticks` at three sites

Resolution: A small `pub(crate) trait Member` (`tick`, `ticks`, `fork`, `send`/`recv`, `sync`, `join`) implemented for `oracle::Clock` (with `ticks` as the literal loop and its comment) and `Clock`; one `step<M: Member>(pop: &mut Vec<M>, op: &Op)`; `run` as the fold from `vec![M::seed()]`; `master_differential` calls `step` per population. `replay` keeps its copy with a one-line reason at the site. State the op inventory as "the variants of [`Op`]" at all three sites. Acceptance: one applier body in optrace.rs; adding an `Op` variant fails to compile in exactly one place; no prose lists the op variants.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### testing-oracles-11 (low, simplification): roster: approved (ruling 104)

`replay` carries a `seeds` parameter every caller fixes at 1 and re-spells the optrace steppers; `FunctionClock`'s `Err` arms are unreachable

Resolution: Drop `seeds` and start each population from one seed. Extract the oracle arm of `optrace::run` into a `step_oracle(&mut Vec<oracle::Clock>, &Op)` so `run` folds over it, write a `step_fs(&mut Vec<FunctionClock>, &Op, &mut StdRng)` beside it, and reduce `replay` to the pre-op disjointness-agreement assert followed by three step calls. Make `FunctionClock::join`/`sync` infallible operations that assert disjointness, since no caller wants the `Err`. Acceptance: `replay` contains no `match *op` arm that mutates `im` or `or` directly; the `Join`/`Sync` index arithmetic exists in optrace only (plus the fs stepper); `replay(1, …)` call sites become `replay(…)`; `replay_matches_across_references` and the sweep pass unchanged.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### version-core-26 (low, simplification): roster: approved (ruling 104)

`hull_traffic`'s `snapshot` and `reset` enumerate the `Rung` variants by hand; `web_traffic` is a shape-for-shape copy

Resolution: store the cells as `static CELLS: [AtomicU64; N]` indexed by a `Rung::index()` (or `#[repr(usize)]`), so `reset` is a loop over `&CELLS` and `snapshot` reads by index; or lift a small `Tally<const N: usize>` (record/snapshot/reset over an atomic array) into `codec`/`meter` and have both classified counters use it. Apply the same to web_traffic.rs. Acceptance: no per-variant array literal in either `reset`; adding a `Rung` variant requires touching exactly the enum and the snapshot struct, with a compile error naming the second.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### version-core-8 (low, simplification): roster: approved (ruling 104)

`span_all` re-implements `balanced_fold`'s counter dispatch; a third copy lives in span algebra

Resolution: generalize `balanced_fold` over the accumulator `M` with a small ops record (`lone: fn(&Version) -> M`, `leaf: fn(&Version, &Version) -> M`, `absorb: fn(&mut M, &Version)`, `merge: fn(&mut M, M)`), instantiated with `M = Version` by `join_all`/`meet_all`/`Sum` and with the `(lo, hi)` pair by `span_all`; `Hull` dissolves into `Group<B, M>`; `DedupRuns<I>` drops `F` for `I::Item: Borrow<Version>`. Span algebra can adopt the same fold with a `points` pre-check hook. Acceptance: the weight-discipline comment appears once in version.rs; `fold_clone_collapse_is_value_invisible`, `boundary_arity_fan_folds_match_the_sequential_fold`, and the `VERSION_LIST` fold laws stay green; `DedupRuns` has no function-typed field.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzz-guests-pins-22 (nit, simplification): roster: approved (ruling 104)

`COMBINE_ARITY_CAP` and the `dispatch!` arm list are hand-parallel

Where: `crates/before/fuzzfit/guest/src/lib.rs:791-831`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Derive the arm list from the cap, or tie them with a `const _` assert

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### inventory-9 (nit, simplification): roster: approved (ruling 104)

Dead `len == 64` arm under a `len <= 63` assert in `BitStack::push_bits`

Where: `crates/before/src/codec/stack.rs:61-71`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Two asserts and `(top << len) \

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### meter-registry-tier2-17 (nit, simplification): roster: approved (ruling 104)

The sizer-of-a-Version idiom is copied fifteen times; `built_view` and `dense` are fully qualified beside imported siblings; the two kernel wrappers are a copy-paste pair

Where: `crates/before/src/meter/tier2/tests.rs:48-58`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): A `size_of(v)` helper; imports; one `kernel_emit`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### recursion-8 (nit, simplification): roster: approved (ruling 104)

BitStack::push_bits carries a dead len == 64 arm; pop_bits's one-level recursion is undocumented

Where: `crates/before/src/codec/stack.rs:58-102`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Drop the dead arm; note or inline `pop_bits`'s one-level self-call

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-watermark-23 (nit, simplification): roster: approved (ruling 104)

The three-probe minimum read is written seven times

Where: `crates/before/src/version/skyline/watermark/tests.rs:91-105`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `assert_minimum_at` helpers

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### testing-diff-gen-4 (nit, simplification): roster: approved (ruling 104)

`BespokeGenre::GENRES` and `name()` restate the variant list twice

Where: `crates/before/src/testing/diff_ops.rs:848-870`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Derive the roster from an exhaustive match, or `strum::EnumIter`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

