<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P6 lane: the skyline coding (coding, fill and grow, comparison kernels, query, watermark)

## Goal

The skyline kernels' per-module entries: each lands per its stated Resolution inside an approved roster, under rulings 43, 52, 64, and 88; the mediums await their individual rulings.

## Rulings on this lane's mediums

Every medium in this lane is ruled (rulings 93 to 103): skyline-coding-23, skyline-coding-6, skyline-fill-grow-12, skyline-fill-grow-24, skyline-sweep-place-masked-19, skyline-watermark-21. The decisions stand beside each entry under Members.

## Roster summary

5 ruled (2 medium, 3 low); 6 medium ruled (93 to 103); 76 roster members approved (ruling 104) (26 low, 50 nit).

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

Every P6 lane runs after the P1 to P5 and P7 lanes that touch its files have landed on main, or rebases onto them before its final gate run; the coordinator names the base SHA at launch. Lows and nits inside this lane's approved roster are swept without a question to Finch; every high and medium has an individual ruling. A change that would alter a rendered `before` doc panel is a stop (ruling 89). Follows `p2-rows`, `p2-cures` (skyline-coding-9's cure, the memo-heap representation), `p2-widths` (the link index), `p4-structure` (CheckedCursor, FrameBits, Pair, advance_set, the watermark tail and cmp_min), `p4-ghosts`, and `p8-performance` (the batch). skyline-watermark-14 (ruling 91) lands after skyline-watermark-18 and -19. Owns `src/version/skyline/**`.

## Members

### skyline-query-31 (medium, verification): ruling 90

The adequacy kernels hand-copy the shipped driver loops, `finish`, and the settle reduction under "verbatim" claims that have drifted

- Owner-gated: no

Resolution: Test-local refactor, no production change (the precedent 982bd260 set for `mass_split`): one small trait (`open`, `interval`, `jump`, `boundary`, `live(&mut)`, `finish`) implemented by a thin wrapper over the shipped `Integrator` and by each known-bad integrator; one `fold_single` and one `fold_pair` driver in the test module; one `settle_with(integ, merge)` reducer shared by the per-digit and schoolbook kernels; one shared `finish_with(integ, close)`. The kernels then differ from the shipped code only in the component they refute, and "verbatim" becomes true by construction where it is still claimed. If the owner prefers to keep the copies independent, strike "verbatim" from all ten docs and re-sync the copies to the current shipped bodies (drop the zero guards and the `is_empty` return, adopt the unconditional `jump`, match `.expect`, add the opening-sign step). Acceptance: no doc in tests.rs says "verbatim" about a body that is not a shared function; `grep -n 'final_window_magnitude != \|segment_magnitude != \|unwrap_or(1)' tests.rs` is empty; every `_reads_superlinear` test keeps its rostered name and still reads red at its floor and value-exact against the shipped fold under `just test-all`.

Ruled (90): test-local refactor: one integrator trait implemented by a wrapper over the shipped type and by each known-bad, one `fold_single` and `fold_pair`, one settle reducer, one finish; the kernels differ from the shipped code only in the component they refute. The strike-and-resync alternative is struck.

### skyline-watermark-14 (medium, simplification): ruling 91

propagate's certificate ladder is cost-inert over the fold it guards, and its duplicated guard costs a line-pinned mutant exclusion

- Owner-gated: yes: reopens d0501cd7's recorded decision to keep the boundary-dominates arm; removes two rostered panic arms and a rostered exclusion

Resolution (primary): replace 772-839 with `let residue_wider = residue.digit_count() >= diff.digit_count(); let (mut wide, narrow) = if residue_wider { (residue, diff) } else { (diff, residue) }; wide.sub_accum(&narrow); self.retire(narrow);` then dispatch on `(wide.sign(), residue_wider)`: `(Greater, true)` the difference died, `on_die(payload); zeros += 1; residue = wide; continue`; `(Equal, _)` exact meet, `on_die(payload); self.retire(wide); zeros += 1; break`; `(Less, true)` the boundary survives as `diff − residue`: `wide.negate(); push Diff { compact(wide), payload }; break`; `(Greater, false)` push `Diff { compact(wide), payload }; break`; `(Less, false)` the difference died: `on_die(payload); wide.negate(); residue = wide; zeros += 1`. Rewrite 676-694 as "fold the narrower operand into the wider (`merge_into_wider`'s rule); the survivor is never read across its width", delete .cargo/mutants.toml:99-117 and the two covcheck entries, re-pin tools/mutantcheck-expected.json. Fallback: a `fn dwarfs(big: &mut Accumulator, small: &Accumulator) -> bool` written once (guard, read, one `unreachable!`), the arm becoming two `if dwarfs(...)` calls; then `cargo mutants --list` decides whether the single `>=` site still needs a pin, and a name pin replaces the line pin if so. Acceptance: `just test-all` green with the seam plunge/stop bands, the latent-ladder band, the ascend envelope, and the fill/min_ticks differentials unchanged; the seam MEASURED lines inside their bands (identical, or differing by a constant per hop); no line-and-column pin for `watermark.rs` in .cargo/mutants.toml; `just mutants-list` clean. Construction: before changing code, delete lines 790-809 alone under a local reverted swap and run `skyline_min_ticks_seam_stop_*`: the mutants.toml rationale predicts byte-equal readings, which demonstrates that arm saves nothing. Then apply the primary replacement and capture the seam bands' MEASURED lines; equality or a per-hop constant delta inside the ×0.75/×1.25 bands settles cost-inertness.

Ruled (91; reopens and retires d0501cd7's keep): first the construction (delete the boundary-dominates arm under a local swap and re-run the seam-stop bands), then replace the certificate ladder with `merge_into_wider`'s rule and the five-way dispatch on `(sign, residue_wider)`; delete the two `unreachable!` arms and their covcheck entries (the mutant exclusion is already gone with the roster, ruling 18). Composes with skyline-watermark-18 and -19 (`p4-structure`); run after that lane.

### skyline-fill-grow-25 (low, simplification): ruling 92

The paired id-times-event walk skeleton is spelled twice (`FillWalk::walk` and `PreScan::run`)

- Owner-gated: yes (a design proposal that moves readings)

Resolution: After the shared frame type lands, construct a `PairedWalk` driver parameterized by a visitor (hooks: empty region, full region, leaf under id node, left-full site open/close, ordinary node left/right/absent), measure the envelopes, and keep two spelled-out walks if the hook count makes the driver less legible than the twin files. Acceptance: either one driver with two visitors and the "same reads" claim structural, or a recorded decision to keep the twins with the frame types shared.

Approved roster (92, decision 72): lands per the quoted Resolution inside this lane. A design proposal that moves readings: measured at the parent, every moved pin tightened or, if a reading rises, a stop. Depends on skyline-fill-grow-27's shared frame type (`p4-structure`).

### skyline-fill-grow-8 (low, simplification): ruling 92

Output-delta anchoring is a bool plus an idle accumulator beside an enum-shaped sibling, and the anchor switch is spelled twice

- Owner-gated: yes for the enum (it moves readings); the helper is adoptable now

Resolution: Now: `fn out_delta_from_min(&mut self) -> Accumulator` wrapping the three-line switch; `emit_step` materializes it, `emit_offset` folds the offset first. Proposal for the owner: `enum OutAnchor { Height(Accumulator), Min }` mirroring `Relation`, which removes the idle-gap sentence and the `debug_assert!(!self.w_anchored, ..)` at 912 and 1024; `emit_at_min`'s `mem::replace(&mut self.gap, fresh)` at 981 becomes a variant swap. Acceptance: one switch helper; if the enum is adopted, no `w_anchored` field and no idle-gap sentence.

Approved roster (92, decision 72): lands per the quoted Resolution inside this lane. A design proposal that moves readings: measured at the parent, every moved pin tightened or, if a reading rises, a stop.

### skyline-query-21 (low, simplification): ruling 92

`mul_into` carries a limb-metered zero guard its sibling refuses, a shift parameter that is zero at every production call, a collected `Vec<u32>`, and a `bool` whose keep ruling lives only in history

- Owner-gated: yes: the guard and the bool are recorded owner rulings

Resolution: `if factor.bits() == 0 { return; }` (or drop the guard: a zero factor's per-digit `*=` is word-scale and the adds are no-ops); iterate `Limbs::new(&digits.0).flat_map(..)` directly; either drop `shift` (the adequacy kernels pre-shift through a test-local wrapper) or reword the doc to say the scale serves the committed known-bad kernels while every production charge is at scale zero; add one comment line stating that `subtract` is an operation selector, not a quantity sign. Acceptance: `grep -n '\*factor == Base::ZERO' web.rs` is empty; the min_ticks limb columns drop by one factor-width record per settle (re-pin attributed to this change) with value pins unchanged; either no production call site names a shift or the doc's description matches its callers.

Approved roster (92, decision 72): lands per the quoted Resolution inside this lane.

## Mediums ruled 93 to 103

Each medium below now carries its ruling and any amendment beside its quoted Resolution; land per the ruling.

### skyline-coding-23 (medium, verification): ruling 101

two test-local depth recursions sit outside recurse.rs's inventory and bypass `descend!`

- Owner-gated: no

Resolution: route both recursions through `descend!` as testing/bridge.rs does, or add both to recurse.rs's inventory with the depth bound stated at each site (grid_version already states `O(log)`; inverted_flag_stream should state it is bounded by the generator depths it is fed). Acceptance: recurse.rs's inventory and `grep -rn 'fn walk(\|fn build(' crates/before/src` agree on the set of test-local recursions, and each site either uses `descend!` or states its bound.

Ruled (101): Route both test-local recursions through `descend!`; the inventory-with-stated-bounds alternative is struck. See ../rulings.md.

### skyline-coding-6 (medium, verification): ruling 102

the admission walk's mid-stream collapsible-pair rejection has no committed witness through Span::decode

- Owner-gated: no

Resolution: add to src/span/tests.rs, beside the existing witness, a deterministic composite whose join carries a pair closing mid-stream (the bytes below; note they carry a proper padding marker, which the existing `0b0111_1000` witness does not, passing only because `NotCanonical` fires before `require_marker_padding`), and a proptest mirroring `planted_collapsible_pairs_are_rejected_at_every_leaf` through `Span::decode`: take a canonical `lo <= hi`, split one leaf of `hi` into an equal-sibling zero-delta pair at a proptest-chosen preorder position (heights unchanged, so only canonicality can reject), re-pad, assert `Err(Decode::NotCanonical)`; run the composite through the borsh span leg too. Acceptance: `Span::decode(&[0xE0, 0x3E, 0xE0]) == Err(Decode::NotCanonical)` is asserted and the proptest is committed; replacing `self.last_delta_zero` with `false` at admit.rs:173 turns both red while every existing witness stays green. Construction: composite `[0xE0, 0x3E, 0xE0]`: `lo` = `11` (the empty version) with marker at bit 2; `hi` bits `0 0 1 1 1 1 1 011` then marker at bit 10 (root internal, left internal, leaf gamma(0), leaf zigzag(0), leaf zigzag(+1)). Trace: `hi` opens at depth 2, `D = 0`. First `advance` (`lo` depth 0 < 2): `hi.step()` flips at level 2, pushes `left_was_leaf = true`, reads the second leaf's code 0, sets `last_delta_zero = true`. Second `advance`: `hi.step()` pops `true` and `close_ancestor(is_leaf = true, zero_delta = true)` sees `left_was_leaf = true`, so `Err(NotCanonical)`. With that arm's `zero_delta` forced `false`: the walk flips at the root, reads +1, `D = -1` (Less, `equal = false`), both cursors done, `finish()` pops the single right branch with `left_was_leaf = false` and returns `Dominates`; `require_marker_padding(tail, 10)` passes; `Span::decode` accepts a span whose `hi` is the non-canonical `[0x3E, 0xE0]`, a second live spelling of the constant-0-then-1 function under `Eq`.

Ruled (102): Commit the mid-stream composite through `Span::decode` and the borsh leg, and drive the planted-pair proptest through the admission entry; lands with or before ruling 75's parser unification (`p4-structure`). See ../rulings.md.

### skyline-fill-grow-12 (medium, simplification): ruling 102

fill.rs drives `Memo` and `PreScan` through `pub(super)` fields; the ledger's lifetime is prose in memo.rs and mechanism in fill.rs

- Owner-gated: no

Resolution: Give `Memo` the consume half: `consume(&mut self, pos: u64) -> Option<Accumulator>` (debug-assert `cursor < queue.len()`, fold `pos` into `consumed_check`, take the link, advance), `reserve(&mut self, pos: u64) -> usize` (moved from `PreScan::reserve`), `is_covered(&self, pos) -> bool` / `cover_until(&mut self, end)`, and `drained(&self) -> bool` for the epilogue asserts. Give `PreScan` one entry, e.g. `cover(event, start, id_bits, id_pos, &mut memo) -> u64`, that owns new/reserve/open/run/record/retire/close and the `suspend.is_empty()` assert. Then every `pub(super)` field on `Memo` and `PreScan` becomes private and `record`/`reserve` become private too. Behavior-preserving. Acceptance: no `pub(super)` field on `Memo` or `PreScan`; fill.rs's left-full arm calls one `PreScan` method and `consume_site` calls one `Memo` method; `just gate` clean.

Ruled (102): `Memo` gains `consume`, `reserve`, `is_covered`/`cover_until`, and `drained`; `PreScan` gains one `cover` entry; every `pub(super)` field becomes private. Behavior-preserving; ruling 2's memo-heap cure (`p2-cures`) builds on this shape, so coordinate the order with that lane. See ../rulings.md.

### skyline-fill-grow-24 (medium, documentation): ruling 102

The Counter widths section says the fill walk's `depth` "stays `usize`"; fill.rs declares it `u64`, and the crate argues one width three ways

- Owner-gated: no

Resolution: Rewrite prescan.rs:32-47 to cite the convention by name and drop the contrast: the site-nesting counters are `u64` like every depth on the walk surface (codec/stack.rs's depth denomination), and the one width contract that differs in kind is the ledger's `u32` link index. Have fill.rs:435-437 and grow.rs:517-518 cite the same convention rather than re-deriving it. Acceptance: no sentence in prescan.rs names `usize` for the fill walk's depth; the width rationale for walk depths appears once (codec/stack.rs) and is cited elsewhere.

Ruled (102): One width rationale for walk depths at `codec/stack.rs`, cited from prescan.rs, fill.rs, and grow.rs; the false `usize` contrast deleted (the ledger's link index widens under ruling 33). See ../rulings.md.

### skyline-sweep-place-masked-19 (medium, verification): ruling 102

The placement write-sequence identity is claimed pinned by touch readings, but the placement rows read no touch meter

- Owner-gated: no

Resolution: add `touch_ops` identity legs beside the scan and limb legs in the placement module (for the single exhaustion-confirmed bound, `touches(contains) == touches(partial_cmp)`; for `span_place_scans_each_stream_once`, a relational touch identity against the composed sweeps), on a fixture whose deltas cross a carry boundary (the CliffComb family) so step order moves the reading; then point the three docs at the touch legs by name. Acceptance: with the new legs, reordering `Cursors::priority` to `[START, END, PROBE]` (verdicts, scan, and limb rows unchanged) turns the touch legs red on the carry-boundary fixture; restoring the order turns them green.

Ruled (102): Add touch-identity legs beside the scan and limb legs on a carry-boundary fixture (the cliff-comb family) and point the three docs at them by name. See ../rulings.md.

### skyline-watermark-21 (medium, verification): ruling 103

The test module's unreachability claim is false for min_ticks, which reaches drop_below's latent annihilation with no directed public-API witness

- Owner-gated: no

Resolution: add a directed min_ticks witness beside the query differentials (a `Version` built through the oracle's normalizing constructor, asserted against `oracle::Version::min_ticks` and the closed form), and re-state the module doc: the annihilation is reachable from `Version::min_ticks` (a right leaf emitted under a range whose left child parked) and unreachable from the two fill-side walks because every fill-side child opens its own range, so a post-park emission always arms; only `emit_offset`'s post-collapse restore is `MinWeb<()>`-only. Optionally add a decision tap on `decide_undercut_through_latent`'s three exits (the `web_traffic` idiom) so the min_ticks populations' coverage of each arm is a readable floor. Acceptance: a committed test under `Version::min_ticks` whose input parks a word-scale latent and then emits a drop that dominates it with an outer boundary still stacked, asserting the exact closed form, which fails when `residue.sub_accum(&latent)` at watermark.rs:543 is deleted; the module doc no longer claims the arm is unreachable from packed streams. Construction: leaf heights in stream order `w = 0` (depth 1), `a = 2^34` (depth 3), `b = 2^34 + 2` (depth 4), `c = 2^34 + 1` (depth 4), `z = 1` (depth 2); the tree `R(w, O(X(a, I(b, c)), z))` is normal (no equal sibling leaves). Under `min_ticks`: `w` arms `R` at 0; `a` opens `O`, `X` and arms them 2^34 above (`Word(2^34)`, then a zero run); `b` opens `I` and arms it 2 above; `c` undercuts `I` to 2^34 + 1 (boundary 1, anchor `A = 2^34 + 1`); at `z` the step closes `I` (parks `Λ = 1`) and `X` (zero run), leaving `R`, `O` armed with diffs `[Word(2^34)]`; `z` emits with no pending range: `gap = 1 − A = −2^34`, the latent cannot dominate a two-digit gap, `gap.sign_dominates_at(0)` certifies (2^34 ≥ 3·2^32), `undercuts_here` returns true with the latent live, `undercut` → `drop_below` annihilates (residue 2^34 − 1), which stops at `Word(2^34)` and leaves `O`'s boundary at 1. Expected `min_ticks = (3·2^34 + 4) − (0 + 1 + 2^34 + 2^34 + 1) = 2^34 + 2`; with the annihilation skipped the residue 2^34 meets the outer boundary exactly, settles `R`'s record early, and the answer reads 2^34 + 1.

Ruled (103): Add the directed `min_ticks` witness beside the query differentials, asserted against the oracle and the closed form; restate the module doc's reachability sentence. See ../rulings.md.

## Roster members approved (ruling 104)

Lows and nits approved as this lane's roster by ruling 104. Land each per its quoted Resolution and Acceptance, swept with the ruled members; report rather than choose if a Resolution conflicts with a ruling or offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89).

### recursion-3 (low, documentation): roster: approved (ruling 104)

Test comment credits production folds to descend!; the fat-stack thread is missing from the oracle envelope's bound list

- Owner-gated: no

Resolution: Reword the comment: the production folds are iterative on explicit stacks (integral.rs's settle), so the headroom is for the recursive oracle and bridge witnesses only. Add the fat-stack witness thread to oracle.rs:25-27's list of bound kinds, or cap this leg's tree so the default test stack suffices and drop the thread. Acceptance: the comment names the iterative folds, and oracle.rs's list covers every mechanism an oracle-facing harness uses.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-coding-12 (low, simplification): roster: approved (ruling 104)

`continue_verbatim`'s seven positional arguments are spelled at four sites under two clippy allows

- Owner-gated: no

Resolution: introduce a small struct (a `Continuation { range: Range<u64>, root_depth, first_rel_depth, last_rel_depth, last_code_len }`, or a pre-sliced view plus the four coordinates as a named struct); delete both clippy allows; build it from `Subtree` in grow.rs and from `RegionSkip` in fill.rs; return it from the test helper. Acceptance: `just gate` clean with clippy and without the allows; tick/fill/grow differentials unchanged.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-coding-13 (low, verification): roster: approved (ruling 104)

builder testdocs misstate their bodies (leaf count; an unasserted cost claim)

- Owner-gated: no

Resolution: lines 69-71 and 375: "three equal plateaus, two at depth 2 and one at depth 1 (the tiling of `((7, 7), 7)`), collapse pairwise to a single depth-0 leaf". Lines 109-111: drop "while the wide held code is written exactly once", or make it true by asserting on the scan meter (the write count is what `record_bits` records). Acceptance: each testdoc describes exactly the leaf sequence its body feeds and claims only what its assertions check.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-coding-17 (low, simplification): roster: approved (ruling 104)

`emit` and `hull` are two copies of the emission driver

- Owner-gated: no

Resolution: one const-generic sweep, `fn sweep<const N: usize>(a, b, picks: [fn(Ordering, Side) -> Side; N]) -> ([BitsBuf; N], Directions)`, with join/meet taking output 0 and hull taking both; construct each `Emission` with its real opening side instead of a placeholder. Measure join/meet on the bench judge before landing (touch meters are unaffected: `Directions` is two bools). Acceptance: emit/tests.rs differentials and the `span_is_the_pair_hull` law pass; `skyline_join_*`/`skyline_meet_*` envelopes hold; the bench judge shows no join/meet regression.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-fill-grow-13 (low, simplification): roster: approved (ruling 104)

`consume_payload` inlines `fold_block`'s body

- Owner-gated: no

Resolution: `consume_payload`: decode, then `let step = Signed { sign, magnitude }; self.fold_block(&step); step`; restate `fold_block`'s doc as the primitive (`consume_payload` decodes and folds through it). Acceptance: one fold sequence in fill.rs.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-fill-grow-16 (low, simplification): roster: approved (ruling 104)

The collapse-then-readout idiom is spelled inline four times; the first-leaf variant rebuilds `Signed` by hand

- Owner-gated: no

Resolution: Add `Signed::read(acc: &mut Accumulator) -> Signed` beside `from_sign_magnitude` (collapse via `sign()`, then `sign_magnitude`, then `from_sign_magnitude`), carrying the width note: the collapse bounds the O(held digits) readout to the value's width plus slack, so the read is priced by the code that emits it; a block net may skip it because its scan already paid for the held digits. Use it at fill.rs:870-872, 935-937, watermark.rs:1146-1149, and make 913-920 `Signed::read(&mut self.height).sum(&offset)` (the non-negativity assert at 921 covers the sum). Acceptance: no inline `sign(); sign_magnitude(); from_sign_magnitude` triple outside signed.rs; no hand-built `Signed { sign: Sign::Positive, .. }` from a readout.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-fill-grow-18 (low, simplification): roster: approved (ruling 104)

`continue_verbatim`'s seven positional `u64`s are hand-marshalled from two different summary structs

- Owner-gated: no (the signature lives in build.rs, another partition; both call sites are here)

Resolution: Introduce a splice-range struct in build.rs (`start`, `end`, `first_rel_depth`, `last_rel_depth`, `last_code_len`, or a `Range<u64>` plus a leaf-coordinate pair); have `RegionSkip` and `Subtree` each produce one; `continue_verbatim(src, root_depth, splice)` drops both allows. Acceptance: no `too_many_arguments` allow on either `continue_verbatim`; both call sites pass one struct.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-fill-grow-30 (low, verification): roster: approved (ruling 104)

No deep witness drives a late divergence after a long matched prefix (`Out::materialize`'s replay at scale)

- Owner-gated: no

Resolution: Add a closed-form deep case to `deep_spines_tick_and_flag_identically` through `assert_deep_changed` at `d = 4096`: event `"(0, 0, ".repeat(d) + "5" + ")".repeat(d)` (a right spine of `d` nodes, every left leaf 0, tip leaf 5); id `"(0, ".repeat(d - 1) + "(1, 0)" + ")".repeat(d - 1)` (the tip `(1, 0)` sits over the deepest `(0, 0, 5)` node); expected `"(0, 0, ".repeat(d - 1) + "5" + ")".repeat(d - 1)`. The walk matches `d − 1` pass-through zero leaves, the tip's left-full raise lifts 0 to 5 through the absent-right arm, the equal pair collapses, and the divergence replays a `d − 1`-plateau prefix. Optionally register the shape as a `tick_late_divergence` envelope row (envelopes partition) pinning scan bits at about 2× the event's. Acceptance: the deep test asserts the closed form, canonicality, the re-walk flag clear, and entry agreement at `d = 4096`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-fill-grow-31 (low, verification): roster: approved (ruling 104)

The orbit test's doc states a tighter log term than its body asserts

- Owner-gated: no

Resolution: Decide the intended claim (open question below). If the tighter bound: `let logk = u64::from(u32::BITS - (k + 1).leading_zeros()) - u64::from((k + 1).is_power_of_two());` here and at 1352 and 1369, then run the three orbit tests. If the looser: write the doc as the code computes it, `4·bitlen(k + 1)` ("four bits per bit of `k + 1`"). Acceptance: the doc formula and the `logk` expression agree for `k + 1` a power of two.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-fill-grow-35 (low, simplification): roster: approved (ruling 104)

`recode` spells its zero-crossing test two ways; both grow.rs mutants exclusions are the symptom

- Owner-gated: no

Resolution: Two top-level arms, same-sign (`magnitude + events`) and opposing-sign; the latter `match width_first_cmp(&magnitude, events) { Less => zigzag_signed(sign.negate(), events.clone() - &magnitude), Equal => zigzag_signed(Sign::Positive, Base::ZERO), Greater => zigzag_signed(sign, magnitude - events) }`, where `width_first_cmp` compares `bits()` first and falls through to `cmp` only on a width tie. Keep the `magnitude.bits() == 0` shortcut inside the `Less` arm if the measured limb touches need it. Delete both `grow\.rs` entries from .cargo/mutants.toml. Acceptance: one spelling of the crossing test; both exclusions gone and `just mutants-list` clean; tick rows re-measured at the parent and at the change, any movement recorded as an attribution.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-fill-grow-38 (low, verification): roster: approved (ruling 104)

Test scaffolding is triplicated and grow's pools are a hand-copied strict subset of fill's

- Owner-gated: no

Resolution: Move `left_spike`, `version_of`, `party_of`, and the family pools into `crate::testing` (a `pools` module beside `generators`); have grow/tests.rs draw its pools from the same source, either the full roster or a named subset with the exclusion stated at the site; re-derive `FAMILY_GROW_PAIRS` in the same commit. tests/meter.rs keeps its `left_spike` unless `testing` is exposed under the `meter` feature; note that at the site. Acceptance: one definition each of `left_spike`, `version_of`, `party_of` under `src/`; grow's pools defined in terms of a shared roster; `family_pairs_grow_identically` green with its pin re-derived and its doc still stating the count is exact.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-fill-grow-9 (low, simplification): roster: approved (ruling 104)

Hand-maintained `depth` counters mirror O(1) stack lengths, justified by a recount that does not exist

- Owner-gated: no

Resolution: grow.rs is the clean case: drop `depth`, use `pending.len() + 1` at 564 and `let path_depth = pending.len();` at 616, delete the assert. fill.rs: bind `let depth = frames.len();` at the head of the descend loop and `let depth = frames.len() - 1;` after the `web.close()` in each ascend iteration (the arms' `depth + 1` is then the pre-pop `frames.len()`); delete the counter, its updates, and the asserts at 440 and 559. Acceptance: no `depth` counter or sync assert in `FillWalk::walk` or `grow::emit`; tests green; no envelope movement.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-query-10 (low, documentation): roster: approved (ruling 104)

`mass_split`'s doc overstates the right half's bound in the clamped case

- Owner-gated: no

Resolution: Amend both sentences: "the right half is at most half the node's mass unless the clamp made it a single leaf, in which case it is that leaf and the recursion ends there". Acceptance: the doc's statement holds on masses `[1, 1, 100]`. Construction: `mass_split(&[0, 1, 2, 102], 0, 3)`: `target = (0 + 102).div_ceil(2) = 51`; `prefix[1..3] = [1, 2]`, `partition_point(p < 51) = 2`; `lo + 1 + 2 = 3`, clamped by `min(hi - 1 = 2)` to `2`; the right half `[2, 3)` has mass 100 > 51.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-query-15 (low, simplification): roster: approved (ruling 104)

`Integrator.one: Base` stands in for suanpan's `add_u64_shl`, which `before` never calls

- Owner-gated: no

Resolution: Replace line 806 with `self.segment_mass.add_u64_shl(1, weight_shift);`, delete the field and its initializer, `#[derive(Default)]` the struct (keep `new()` as `Self::default()` or drop it), and apply the same to the test kernels' `position`/`segment_mass` deposits. Acceptance: `grep -rn 'one: Base' crates/before/src/version/skyline/query*` is empty; `just test-all` green with every `skyline_rank_*`, `DISTANCE_*`, and `LAG_*` envelope row unchanged.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-query-25 (low, verification): roster: approved (ruling 104)

The promoting-pool rationale cites an `arb_base` ceiling the generator no longer has

- Owner-gated: no

Resolution: Restate the rationale against generators.rs as it is: freezes are in-support for arbitrary trees; what the pool uniquely supplies is promotion (a parked component at least 10 digits wide against a wide-spelled narrow drift, or an 18-digit parked sum) and the multi-arming trains, which the arbitrary sweep reaches rarely if at all. Delete both 2^128 figures (237-242 and 644-645). If the exclusivity sentence ("these shapes are the only ones that arm it") is to stay, back it with a promotion tap over `family_pool` and the arbitrary sweep; otherwise drop it. Acceptance: `grep -n '2^128\|128-bit' tests.rs` returns nothing and the stated premise matches the widest arm in generators.rs:322-332. Construction: Not a runtime construction; the falsity is textual. For the freeze claim: `spine_of(&[Base::from(1u8), (Base::from(1u8) << 500) + Base::from(1u8), (Base::from(1u8) << 500) + Base::from(3u8)])` folds deltas of about 2^500 then +2 and trips the trigger (16 live digits against 1 funded + 8), a shape `arb_oracle_version` generates whenever a wide-arm node base sits over two small leaves.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-query-27 (low, verification): roster: approved (ruling 104)

`zero_drift_freezes_keep_the_totals_exact` claims a tripped trigger and an empty freeze that nothing in the test observes

- Owner-gated: no

Resolution: Either (a) add a `#[cfg(test)]` tap on the zero-drift arm of `Integrator::freeze` and on the discard arm of `EpochLedger::freeze` and floor both here (at least one per fold per case), so the gate itself sees the regime; or (b) keep the coverage legs as the instrument and say so in the doc: "value-exact on the zero-drift schedule; that the arm is driven is pinned by the CI coverage legs (tools/covcheck-expected.json lists no entry for either arm), not by this test". Acceptance: under (a), deleting the `if drift == UBig::ZERO` arm at integral.rs:860-866 or the `if drift != UBig::ZERO` guard at web.rs:388 turns this test red; under (b), the doc and body agree. Construction: Change the strategy to `p in 1u32..=2` (or raise `FREEZE_ALLOWANCE_DIGITS` to 16): the live spelling never exceeds the allowance, no trigger fires, and the test still passes; independently, delete both zero arms and the test still passes, because parking a zero is value-identical to not parking it.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-query-29 (low, documentation): roster: approved (ruling 104)

Test comment attributes production stack safety to `crate::recurse::descend!`, which production code does not use

- Owner-gated: no

Resolution: "the production folds are iterative (the crate's recursion rule: depth lives on explicit stacks), so the headroom is for the witnesses, not the code under test." Acceptance: the comment names no `descend!` and the grep stays empty.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-query-5 (low, simplification): roster: approved (ruling 104)

The freeze trigger predicate is spelled twice, in `min_ticks` and `Integrator::boundary`

- Owner-gated: no

Resolution: Add `pub(super) fn freeze_due(live: &Accumulator, funded_digits: usize) -> bool` beside the constant in integral.rs; call it from `Integrator::boundary`, `min_ticks`, and the three test kernels; stop exporting `FREEZE_ALLOWANCE_DIGITS` to query.rs. Acceptance: `grep -n 'FREEZE_ALLOWANCE_DIGITS' crates/before/src/version/skyline/query.rs` is empty and the predicate expression appears once in production code.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-query-6 (low, simplification): roster: approved (ruling 104)

`ReignWeb::leaf` relies on a two-call protocol at each call site that the callee could own

- Owner-gated: no

Resolution: Change `ReignWeb::leaf` to `(&mut self, sign, offset, total, ledger)`; inside, `ledger.leaf_ref(); let epoch = ledger.epoch();` before the closures capture the ledger; make `EpochLedger::leaf_ref` private; the first-leaf call becomes `web.leaf(Sign::Positive, &Base::ZERO, &mut total, &mut ledger)`. Acceptance: `grep -n 'leaf_ref' crates/before/src/version/skyline/query.rs` is empty; `assert_single`'s min_ticks legs stay green.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-sweep-place-masked-12 (low, simplification): roster: approved (ruling 104)

Two structs named `IdLeafCursor` share a step body: overlay.rs and party/ops/diff.rs

- Owner-gated: no

Resolution: at minimum rename diff.rs's cursor (for example `SpliceCursor`) so `IdLeafCursor` names one thing, and have overlay.rs's doc name it as the splicing sibling; the composition (diff's cursor over overlay's, adding the `Item`/unsettled layer) is a cross-partition proposal for the party sweep. Acceptance: `grep -rn 'struct IdLeafCursor' crates/before/src` returns one definition; party diff tests and the `party::ops` meter rows unchanged.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-sweep-place-masked-17 (low, simplification): roster: approved (ruling 104)

`walk`'s `finish` takes `Option<Option<Ordering>>` that every arm flattens; `Some(None)` is unreachable

- Owner-gated: no

Resolution: change the signature to `finish: impl FnOnce(Option<Ordering>, Option<Ordering>) -> V`, pass `set.start.as_ref().and_then(BoundSide::relation)` (and the end twin), delete the `.flatten()` calls (the existing `debug_assert!`s work unchanged on the flattened values), and replace the obligation paragraph with one sentence: a dropped side reads `None`, the same as a swept concurrency, and hooks drop only once the direction their finish arm tests is refuted. Acceptance: `grep -c '\.flatten()' place.rs` drops to 1 (the iterator flatten at 555); `span_walks_match_the_composed_sweeps` and the witness tests pass.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-sweep-place-masked-25 (low, documentation): roster: approved (ruling 104)

`coverage`'s `finish` doc states a discipline its hole emptiness arms do not keep

- Owner-gated: no

Resolution: rewrite the paragraph to say that the hole emptiness arms read their emptiness endpoint whether or not it is settled, relying on the permanence of refutation (a settled emptiness endpoint has its emptying direction refuted, so `false` is the decided answer), and that the `!live` guards on the two required arms exist because those arms' refutations already live in `full_possible`. Acceptance: the doc and the six emptiness arms agree on the argument each uses; `filter_coverage_matches_the_composed_sweeps` and `filter_coverage_organic_witnesses` stay green.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-watermark-12 (low, simplification): roster: approved (ruling 104)

arm_at_height and arm_below duplicate the first-arming preamble

- Owner-gated: no

Resolution: extract `fn seat_first(&mut self, pending: u64, gap: Accumulator)` holding the two asserts and the seat/retire/push_zeros body, taking the already-leased or moved `gap` so the lease-before-retire order the pool row's warm-up derivation describes is preserved; call it from both entry points. Drop the duplicate assert in `emit_below_accum` or keep only the one whose message names the raise. Acceptance: one occurrence of "followers attach after the first arming"; `batch_armed_closes_consume_exactly_one_range_record` and the min_ticks differentials green.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-watermark-13 (low, documentation): roster: approved (ruling 104)

push_boundary's doc says only the pushed-above arm constructs the payload; the undercut arm does too

- Owner-gated: no

Resolution: "Two arms construct the payload: the pushed-above arm stacks it beside the new difference, and an arming undercut hands it straight to `on_die` before the residue drives outward; an exact meet touches no payload at all — the reigning state continues." Acceptance: the `push_boundary` and `arm_at_height` docs agree and name the same two arms as lines 655 and 664.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-watermark-3 (low, verification): roster: approved (ruling 104)

The pool-recycle claim is stated for every client but pinned only through min_ticks

- Owner-gated: no

Resolution: add a tick-path pool-miss row beside `pool_recycle` on a committed follower-churn family (`width_circulation_cost`'s or `memo_resolution_cost`'s shapes): reset misses, tick, assert `small >= 1`, `small == large` across the doubling, and `large <= WARMUP` with the warm-up derived from the walk's peak outstanding leases. Then cite both rows at 86, or scope the sentence to the min-ticks client. Acceptance: a `limb-meter` row whose MEASURED line reads equal misses at both scales on a tick family, and which turns red when one `self.web.retire(...)` in fill.rs becomes `drop(...)` under a local, reverted swap. Construction: under a local reverted swap replace `self.web.retire(relation)` at fill.rs:808 with `drop(relation)`; run the existing suite: every envelope and differential stays green (peak heap falls; touches identical). Add the proposed row and observe misses proportional to `k`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-watermark-7 (low, simplification): roster: approved (ruling 104)

Follower slots as parallel arrays; the coupling invariant lives in asserts and an expect

- Owner-gated: no

Resolution: define the private `Follower` struct, change the field, delete `anchor_relative`, rewrite the five loops over the flattened iterator; `follower_set` becomes `Some(Follower { relation, anchor_relative: self.latent.is_some() })`, `follower_take` returns `.take().expect("the follower is active").relation`. Keep `park`'s and `drop_below`'s asserts relating tags to the latent. Acceptance: no `anchor_relative` array, no `expect("a set tag rides an active follower")`, no `0..self.followers.len()` loop; `watermark/tests.rs`, `fill/tests.rs`, and the fill touch envelopes unchanged (no arithmetic moves).

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-coding-10 (nit, simplification): roster: approved (ruling 104)

`Option<bool>` tests spelled through `map`/`unwrap_or` where `== Some(..)` reads directly

Where: `crates/before/src/version/skyline/build.rs:137-140`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `== Some(false)` and `!= Some(true)`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-coding-21 (nit, simplification): roster: approved (ruling 104)

`literal::node` builds a temporary buffer only to iterate it; a rustfmt-displaced trailing comment

Where: `crates/before/src/version/skyline/literal.rs:53-58`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `iter::once(false).chain(..)`; move the comment above

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-coding-25 (nit, documentation): roster: approved (ruling 104)

text.rs module doc misplaces the second pin and misstates the kernels' visibility; a testdoc possessive

Where: `crates/before/src/version/skyline/text.rs:30-35`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "pinned twice: by the wide-arming flatness band in `tests/meter.rs` ..., and by the committed schoolbook kernel in this module's tests (under `limb-me ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-coding-27 (nit, simplification): roster: approved (ruling 104)

`StoredLeft` is `Summary` minus `root`, copied field by field

Where: `crates/before/src/version/skyline/text.rs:292-296`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `Summary { root, body: StoredLeft }`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-coding-3 (nit, documentation): roster: approved (ruling 104)

two inventories of one test suite, already drifting

Where: `crates/before/src/version/skyline.rs:123-145`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): keep the inventory in the tests module doc (where a new test is added) and reduce the kernel module's section to the invariants the tests protect plus ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-coding-36 (nit, simplification): roster: approved (ruling 104)

`Extremum`'s reset policy keys off direction while its contract is about buffer provenance

Where: `crates/before/src/version/skyline/walk.rs:178-190`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `Extremum::min()` owns a fresh register, or carry `Provenance` as a field

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-coding-8 (nit, simplification): roster: approved (ruling 104)

qualified paths where the import already exists; a one-line `version_of` alias copied nine times

Where: `crates/before/src/version/skyline/admit.rs:296-298`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Import at each site; delete the `version_of` copies

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-fill-grow-15 (nit, simplification): roster: approved (ruling 104)

`let _ = matched;` after `debug_assert!(matched, ..)` is dead in both build profiles

Where: `crates/before/src/version/skyline/fill.rs:898-900`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Delete the two `let _ = matched;` lines

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-fill-grow-19 (nit, simplification): roster: approved (ruling 104)

`if left { id.skip() } if right { id.skip() }` re-spells `IdReader::skip_present_children`

Where: `crates/before/src/version/skyline/fill/fuse.rs:320-325`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Keep the `IdNode` and call `skip_present_children`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-fill-grow-26 (nit, simplification): roster: approved (ruling 104)

`record`'s `while head_level > level` runs at most once

Where: `crates/before/src/version/skyline/fill/prescan.rs:324-328`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `debug_assert!` plus a single `if`, or state the one-level bound

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-fill-grow-28 (nit, documentation): roster: approved (ruling 104)

`pop_site`'s comment gives the wrong reason the slot cast is lossless

Where: `crates/before/src/version/skyline/fill/prescan.rs:691-693`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "A slot is a `queue` index pushed as `u64` at `push_site`, so the round trip to `usize` is exact." At memo.rs:137: `expect("nonzero link count fits u3 ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-fill-grow-29 (nit, documentation): roster: approved (ruling 104)

Two line-wrap typos split compound modifiers

Where: `crates/before/src/version/skyline/fill/tests.rs:1005-1005`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "nested-full-sibling id"; "pending-sibling path bits"

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-fill-grow-33 (nit, simplification): roster: approved (ruling 104)

`EvScan::skip` is a `#[cfg(test)]` method in the production file with one test consumer

Where: `crates/before/src/version/skyline/grow.rs:270-287`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Move `skip` into grow/tests.rs

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-fill-grow-37 (nit, simplification): roster: approved (ruling 104)

`assert_grow` drops the ticked stream, so two tests re-tick and re-inline the oracle comparison

Where: `crates/before/src/version/skyline/grow/tests.rs:59-72`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `assert_grow(v, p) -> Option<BitsBuf>`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-fill-grow-6 (nit, simplification): roster: approved (ruling 104)

Long qualified paths where the module is already imported

Where: `crates/before/src/version/skyline/fill.rs:264-271`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `use super::grow::{self, Cost, Route}` and the sibling imports

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-query-11 (nit, documentation): roster: approved (ruling 104)

`clusters`' "ascending" precondition must be strict; the gap subtraction underflows on a repeated index

Where: `crates/before/src/version/skyline/query/integral.rs:343`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Say "strictly ascending" at 324-325, 441, and 603, and extend `charge_digits`' debug_assert loop to check each index exceeds its predecessor

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-query-14 (nit, simplification): roster: approved (ruling 104)

Encoding conventions in the settle kernel that a named form would make evidently right

Where: `crates/before/src/version/skyline/query/integral.rs:501-525`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Name the two images; an `Option` min in `combine`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-query-17 (nit, verification): roster: approved (ruling 104)

The hard assert in `Integrator::jump` ("a monotone orientation's change term is a debit") has no committed demonstration that it fires; every shipped closure is monotone.

Where: `crates/before/src/version/skyline/query/integral.rs:831-838`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): A `#[should_panic]` test driving `pair_fold` with an anti-monotone closure on `ConcurrentPair`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-query-18 (nit, simplification): roster: approved (ruling 104)

The "read an accumulator as (Sign, Base), skipping zero" idiom is hand-spelled at seven production sites

Where: `crates/before/src/version/skyline/query/integral.rs:921-934`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `signed_base(acc) -> Option<(Sign, Base)>` in signed.rs

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-query-19 (nit, documentation): roster: approved (ruling 104)

"sound" used loosely for "correct only when" and "holds"

Where: `crates/before/src/version/skyline/query/integral.rs:980-981`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): integral.rs:980 "Correct only immediately after ..."; tests.rs:1418 and 1524 "the identity ... holds"

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-query-2 (nit, simplification): roster: approved (ruling 104)

`let scale = max_depth` aliases carry a comment justifying a conversion that no longer exists

Where: `crates/before/src/version/skyline/query.rs:203-205`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Pass `max_depth` directly; delete both aliases

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-query-20 (nit, documentation): roster: approved (ruling 104)

Comment says the emptiness check is not re-taken here, directly above a debug_assert that re-takes it

Where: `crates/before/src/version/skyline/query/integral.rs:1044-1051`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "The caller gates the call on a non-empty ledger; this restates the precondition in debug builds." Acceptance: comment and code agree on whether the c ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-query-26 (nit, documentation): roster: approved (ruling 104)

A fullwidth left parenthesis (U+FF08) in the `zero_drift_heights` doc

Where: `crates/before/src/version/skyline/query/tests.rs:341`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Replace `（` with `(`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-query-7 (nit, simplification): roster: approved (ruling 104)

Long qualified paths where the sibling items are imported; a one-line `version_of` wrapper

Where: `crates/before/src/version/skyline/query.rs:547`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Import `gamma_code_signed_int` and `Party`; hoist the test imports; drop `version_of`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-query-8 (nit, simplification): roster: approved (ruling 104)

`pub(crate) mod integral` is wider than any use

Where: `crates/before/src/version/skyline/query.rs:649-650`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `mod integral;`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-sweep-place-masked-1 (nit, documentation): roster: approved (ruling 104)

Rustdoc link syntax inside a `//` comment, and a ragged module-doc wrap

Where: `crates/before/src/version/skyline/masked.rs:110-111`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): write `// (Directions::relation's map).` and rewrap lines 38-46 to the module's width

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-sweep-place-masked-11 (nit, verification): roster: approved (ruling 104)

`IdLeafCursor::open`'s empty-stream arm is reachable through none of its callers (every mask is a `Party`, never empty) and has no test.

Where: `crates/before/src/version/skyline/overlay.rs:488-514`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One unit test opening the cursor on the empty view, or drop the guard and state that masks are never empty.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-sweep-place-masked-18 (nit, simplification): roster: approved (ruling 104)

Verdict hooks take a `bool` that three of four callers ignore

Where: `crates/before/src/version/skyline/place.rs:447-448`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): A named two-variant enum, or adapt the three hooks to `Fn(Directions)`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-sweep-place-masked-2 (nit, simplification): roster: approved (ruling 104)

`masked::Walk` carries two correlated `Option`s re-derived by `expect`, and its A/B arms are mirror copies

Where: `crates/before/src/version/skyline/masked.rs:158-164`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): A `Mask` struct pairing cursor and height; factor the mirror arms

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-sweep-place-masked-22 (nit, simplification): roster: approved (ruling 104)

`fold_signed_int` is called by qualified path beside imports from the same module

Where: `crates/before/src/version/skyline/place/filter.rs:98-101`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `use super::signed::{fold_signed_int, Sign};`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-sweep-place-masked-23 (nit, simplification): roster: approved (ruling 104)

`filter::BoundSide` sits one module below `place::BoundSide` with a different shape

Where: `crates/before/src/version/skyline/place/filter.rs:140-147`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Rename filter's struct `DemandSide`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-sweep-place-masked-24 (nit, documentation): roster: approved (ruling 104)

Read-order prose names a write-sequence effect that independent accumulators cannot have

Where: `crates/before/src/version/skyline/place/filter.rs:152-154`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): at filter.rs:152-154, :318-319, and place.rs:459-463, replace "fixes the accumulator write sequence" / "so every question's accumulator traffic is ide ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-sweep-place-masked-26 (nit, verification): roster: approved (ruling 104)

place/tests.rs has no module doc naming its oracle (the composed pair sweeps), and two tests share an indistinguishable first sentence.

Where: `crates/before/src/version/skyline/place/tests.rs:1-1`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Add the module doc; reword line 294.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-sweep-place-masked-27 (nit, simplification): roster: approved (ruling 104)

Test idiom: redundant parentheses (41 sites) and an unimported qualified helper (17 sites)

Where: `crates/before/src/version/skyline/place/tests.rs:14-15`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Drop the parentheses; import `built_view`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-sweep-place-masked-28 (nit, verification): roster: approved (ruling 104)

`dominance` is the one placement entry point without an organic-witness test; the module doc presents `precedence` as its mirror.

Where: `crates/before/src/version/skyline/place/tests.rs:218-224`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Add `dominance_walk_verdicts_organic_witnesses` mirroring the precedence test.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-sweep-place-masked-30 (nit, documentation): roster: approved (ruling 104)

signed.rs says "the tests pin the bijection", but the bijection test lives in skyline/tests.rs

Where: `crates/before/src/version/skyline/signed.rs:12-14`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): move `zigzag_is_a_bijection_without_negative_zero` (skyline/tests.rs:300-331) into signed/tests.rs, or ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-sweep-place-masked-31 (nit, simplification): roster: approved (ruling 104)

`gamma_code_signed_int` duplicates `gamma_code_signed`'s fused fast-path body

Where: `crates/before/src/version/skyline/signed.rs:259-276`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One `small_signed_code(sign, mag)` helper

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-sweep-place-masked-6 (nit, documentation): roster: approved (ruling 104)

`unreachable!` messages that are slot counts, not proofs

Where: `crates/before/src/version/skyline/masked.rs:384-384`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): at all four sites, "slot indices come only from `priority`, which names the constants above"

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-sweep-place-masked-8 (nit, documentation): roster: approved (ruling 104)

Colon-fronted fragments open body paragraphs

Where: `crates/before/src/version/skyline/overlay.rs:71-71`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): rewrite as sentences ("The bounds are derived rather than measured: ..."; "A *plateau* is one maximal constant run ..." ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-watermark-11 (nit, simplification): roster: approved (ruling 104)

A redundant latent sign read, and a comment that credits it with the floor's tightness

Where: `crates/before/src/version/skyline/watermark.rs:492-500`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `debug_assert_eq!` on the latent sign; re-state the comment

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-watermark-15 (nit, documentation): roster: approved (ruling 104)

compact's 'anything wider can never fit' overstates suanpan's collapse bound

Where: `crates/before/src/version/skyline/watermark.rs:850-853`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "The width test reads the digit count alone: a difference spelled in more than two digits after its sign read is at least `2^64 − 2^32` (the dominatio ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-watermark-16 (nit, simplification): roster: approved (ruling 104)

Accumulator::default() spelled once where the crate says Accumulator::new()

Where: `crates/before/src/version/skyline/watermark.rs:883-891`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `Accumulator::new()`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-watermark-17 (nit, verification): roster: approved (ruling 104)

`emit_here`/`emit_offset` accept an emission outside any open range and heal silently at the next arming, where their siblings debug-assert their `armed`/`pending` preconditions.

Where: `crates/before/src/version/skyline/watermark.rs:947-958`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): A `debug_assert!` that `pending > 0` or `armed > 0` holds, at the top of both.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-watermark-2 (nit, documentation): roster: approved (ruling 104)

The emission bullet of the cost discipline is one sixteen-line sentence with rewrap residue

Where: `crates/before/src/version/skyline/watermark.rs:55-70`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): split into three sentences (the amortized sign read against the anchor; the O(1) latent decision by top-index domination with comparable scales foldin ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-watermark-20 (nit, simplification): roster: approved (ruling 104)

materialize and the lease/retire pool are an allocator riding the watermark type

Where: `crates/before/src/version/skyline/watermark.rs:1140-1150`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): A private `Pool` type, if taken

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-watermark-22 (nit, verification): roster: approved (ruling 104)

The `wide()` helper doc says a word-range wide spelling probes certification; `to_word` dispatches it to the word path, so it probes nothing there.

Where: `crates/before/src/version/skyline/watermark/tests.rs:46-53`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Restate: within the word range the spelling is a no-op; past it `add_wide` spills.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-watermark-26 (nit, simplification): roster: approved (ruling 104)

Four hand-rolled counter modules of one shape

Where: `crates/before/src/version/skyline/pool_traffic.rs:27-60`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): A `macro_rules!` counter module, if taken

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-watermark-5 (nit, documentation): roster: approved (ruling 104)

Two maintainer docs misattribute which operand a fold reads or a reader mutates

Where: `crates/before/src/version/skyline/watermark.rs:113-115`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): 115 and 586 "the offset `gap_old − below` costs one fold of `below`'s width, which the caller priced and which survives as the new `gap`" ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-watermark-6 (nit, simplification): roster: approved (ruling 104)

Strictly-positive counts held as plain u64

Where: `crates/before/src/version/skyline/watermark.rs:154-168`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `NonZeroU64` for `Word` and `ZeroRun`, if taken

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### skyline-watermark-9 (nit, documentation): roster: approved (ruling 104)

fold_height's armed guard has its why only in .cargo/mutants.toml

Where: `crates/before/src/version/skyline/watermark.rs:282-290`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): add one clause to the doc: "An unarmed web skips the fold: its `gap` is replaced wholesale at the first arming ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

