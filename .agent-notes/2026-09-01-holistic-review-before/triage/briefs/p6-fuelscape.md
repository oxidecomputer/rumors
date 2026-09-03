<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P6 lane: fuelscape (before-fuelscape, docs/, build.rs's island job)

## Goal

The fuelscape pipeline's and renderer's entries, landed per their Resolutions inside an approved roster, under ruling 89 (decision 66: the grid pin's mechanism, the typesetting gate, the accretion writer, the overlay derived from the board, the size-axis classification; `build.rs` stays) and ruling 90 (the probe trace keeps its smooth and aligns the rest to it). Any change to a rendered `before` panel beyond those two ruled ones is a stop.

## Rulings on this lane's mediums

Every medium in this lane is ruled (rulings 93 to 103): fuelscape-render-18, fuelscape-render-19. The decisions stand beside each entry under Members.

## Roster summary

6 ruled (4 medium, 2 low); 2 medium ruled (93 to 103); 34 roster members approved (ruling 104) (16 low, 18 nit).

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
- **Prose (ruling 105).** Every paragraph you touch passes the three tests in
  `PROSE.md` (altitude, concision, legibility); the reviewer applies its
  checks; the diff is net shorter in prose unless your report says what the
  additions buy.
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

Every P6 lane runs after the P1 to P5 and P7 lanes that touch its files have landed on main, or rebases onto them before its final gate run; the coordinator names the base SHA at launch. Lows and nits inside this lane's approved roster are swept without a question to Finch; every high and medium has an individual ruling. A change that would alter a rendered `before` doc panel is a stop (ruling 89). Follows `p1-fuzz` (fuelscape-pipeline-23's perturbed twins), `p2-surface` (the guest-exported arity cap, the surface row shape), and `p7-api` (ruling 85's `rank_display` re-pin). The fuelscape survey that regenerates the committed datasets after the overlay change is hours long and is the coordinator's run: report the code change and the local verification, leave the regeneration to the coordinator. Owns `crates/before-fuelscape/**`, `crates/before/docs/**`, `crates/before/build.rs`.

## Members

### fuelscape-pipeline-30 (medium, simplification): ruling 89

Two exemption rosters over one surface: 51 rows excused twice with independently worded reasons

- Owner-gated: yes: the fix reshapes `SurfaceRow` in `before` (crates/before/src/surface.rs:183-194), the roster of record shared by the board and the atlas

Resolution: Give `SurfaceRow` a size-axis classification (e.g. `axis: SizeAxis`, with `SizeAxis::None(&'static str)` carrying the one reviewed reason) and have both tilings treat `SizeAxis::None` rows as excused automatically; each instrument's table then shrinks to its own decisions (atlas: delegating wrappers priced at another panel; board: rows it excuses but the atlas panels). Acceptance: `EXEMPTIONS` and `BOARD_NOT_APPLICABLE` name no row whose `SurfaceRow` carries `SizeAxis::None`; both tiling tests pass; adding an O(1) accessor requires exactly one reason, on its surface row.

Ruled (89, decision 66): `SurfaceRow` gains a size-axis classification (`SizeAxis::None(&'static str)` with one reviewed reason) that both tilings honor automatically; the two exemption tables shrink to instrument-specific decisions. This edits `crates/before/src/surface.rs`, shared with `p2-surface`; coordinate.

### fuelscape-render-15 (medium, api): ruling 89

The dump format accretes, but the only writer truncates the index, and the one accretion on record was a hand merge

- Owner-gated: no

Resolution: add `DumpWriter::open(dir, meta)` that loads an existing index with `read`-grade strictness (gz-aware via `parse`), refuses a `RunParams` mismatch and a duplicate op name, and appends; make `new` refuse a directory already holding `atlas.json` or `atlas.json.gz`; have `parse` refuse when both the plain file and its `.gz` sibling exist; wire the open path as `--append-to <dump>` (or make `--dump` open-or-create) and document the accretion recipe (measure alone, gzip, append, `fuelscape-compact`) beside the full re-measure at justfile:680-691. Acceptance: a dump test appends a synthetic op to an existing dump and `read` returns the union in index order with the original documents byte-unchanged; a params-mismatch test and a plain-beside-gz test are refused naming the file; `DumpWriter::new` on a directory holding an index errors. Construction: in crates/before-fuelscape, `cargo run --bin fuelscape -- version_tick --dump --samples 2 --max-bytes 4 --out dump`, then read `dump/atlas.json`: `ops` is `["version_tick"]` while 104 `.json.gz` documents sit unindexed, and `dump::read("dump")` now sees one op.

Ruled (89, decision 66): `DumpWriter::open` with the params-mismatch, duplicate-op, and plain-beside-gz refusals; `new` refuses an existing index; `parse` refuses both files present; the accretion recipe documented beside the full re-measure.

### fuelscape-render-27 (medium, correctness): ruling 89

The typesetting pass rewrites code spans on every workspace crate's rustdoc pages, while the justfile says the script activates only on `.fuelscape` elements

- Owner-gated: no

Resolution: gate the pass on before's own pages: the script already reads `meta[name="rustdoc-vars"]` at line 711, and rustdoc stamps `data-current-crate`, so `hydrate` can run `typesetDocMath` only when `vars.dataset.currentCrate === "before"`. If the owner wants the typesetting workspace-wide instead, correct justfile:253 and the Cargo.toml comment at 9-12 to say so and confirm the other crates' authors accept the face change. Acceptance: after `just docs`, rumors' `Link` doc ("Hand the completed half to `f`.", src/link.rs:190) renders `<code>f</code>` intact while before's `# Complexity` sections are still typeset; the justfile comment matches the behavior either way. Construction: after `just docs`, open the rendered page for `src/link.rs`'s completed-half method: the `f` code span is an `<i>f</i>` inside `<span class="fs-math">`. Or in node with a stub DOM containing `<div class="docblock"><code>k</code></div>`, call `Fuelscape.hydrate()`: the `<code>` is gone.

Ruled (89, decision 66): gate `typesetDocMath` on `data-current-crate === "before"`; other crates' rendered pages stop being rewritten, before's rendering is unchanged (verify by diffing before's rendered panels before and after; a change is a stop).

### fuelscape-render-7 (medium, verification): ruling 89

The cross-platform grid pin's stated sensitivity mechanism is not how it is sensitive, and no known-bad demonstration exists

- Owner-gated: no

Resolution: restate the mechanism (the serialized f64 domain edges, chiefly `y_hi`, carry the platform; bin flips do not); make the fixture carry more sensitive values (a non-power-of-two smallest fuel and smallest size, so `y_lo`, `x_lo`, `x_hi` are all live `log2` evaluations) and state that the cross-host check of record is the illumos gate run plus `fuelscape-verify` on CI; add the known-bad demonstration where feasible: a test-only shadow of `aggregate` using `f64::log2`, asserting its hash differs from the committed one on at least one documented host, or an explicit statement that no single host can witness the swap. Acceptance: the docstring's mechanism matches the arithmetic; the fixture has no exact-power-of-two extremes; either a committed test shows the `f64::log2` variant hashes differently somewhere, or the docstring says plainly that sensitivity is established by the two-architecture gate run and `fuelscape-verify`, not by this host alone. Construction: change `lg` to `f64::log2(v.max(1) as f64)` and run `aggregate_bins_identically_on_every_platform` on the development host; if it passes there, the docstring's sensitivity claim is refuted on that host and only the illumos run and `fuelscape-verify` stand between a libm regression and the committed dump.

Ruled (89, decision 66): restate the mechanism (the serialized f64 domain edges carry the platform), make the fixture's extremes non-powers-of-two so three edges are live, and either commit the `f64::log2` shadow that hashes differently or state plainly that the two-architecture gate run plus `fuelscape-verify` is the check of record.

### fuelscape-render-17 (low, api): ruling 89

The stored `HeatGrid` has no regrid path, so any change to the aggregation constants makes the committed dump unreadable

- Owner-gated: yes (persisting the grid is a documented design decision)

Resolution: add a `--regrid <dump>` mode (or a `DumpWriter` entry) that rewrites each op document's grid from its samples, so an aggregation change is a recipe beside `fuelscape-compact` rather than a hand migration; or move the grid out of the dump into a derived sidecar the way `compact.rs` already derives the widget form; in either case narrow the module doc's styling promise to what it excludes. Acceptance: changing `FUEL_BINS` and running `--regrid dump` then `--render-from dump` succeeds without hand-editing a committed document, and the dump tests still pin raw-sample round-trip and replay byte-identity. Construction: change `FUEL_BINS` from 56 to 64 and run `cargo run --bin fuelscape -- --render-from dump --out target/x` in crates/before-fuelscape: every op document is refused with "stored grid does not match" and nothing in the tool repairs it.

Ruled (89, decision 66): tool the regrid path when next needed; land only the doc sentence that names the accretion writer as the tooled path now, unless a regrid is needed in this lane.

### fuelscape-render-23 (low, correctness): ruling 90

The probe trace draws a five-column smooth of the quantile while its label, handle, readout, and guide anchor use the raw value

- Owner-gated: yes (a presentation choice)

Resolution: draw the raw quantiles (the natural choice at 4096 samples per column), or keep the smoothing and anchor guides and the handle to `tv.sm`, disclosing it in the y-label or probe tooltip ("median, smoothed across neighboring columns"). Acceptance: with an interior column locked, the slider handle, the active guide's crossing, and the drawn trace coincide at that column, and the label names what is drawn. Construction: open any island (`version_tick`), select the `n` hypothesis, click the size-64 column to lock it, and compare the handle's y (raw median) with the trace's y at the same x (smoothed); they differ by the five-point residual, largest where the median's slope changes between columns.

Ruled (90). Finch's words on rendering: any change to before's rendered docs is ruled deliberately, and this one was: the probe trace keeps its five-column smooth, and the label, drag handle, readout, and guide anchor move onto the smoothed value so every element agrees with the drawn line. The Resolution's raw-quantile alternative is struck.

Ledger note: smooth kept; handle, readout, guide aligned to it

## Mediums ruled 93 to 103

Each medium below now carries its ruling and any amendment beside its quoted Resolution; land per the ruling.

### fuelscape-render-18 (medium, simplification): ruling 95

A panel pool of width 1: scoped threads, an atomic cursor, and two mutexes wrap a sequential loop

- Owner-gated: no (a dev-tool binary; git keeps the pool)

Resolution: replace the pool with `for (i, op) in selected.iter().enumerate()` holding `writer` and `rendered` directly (no mutexes, no slots, no cursor), delete the constant, and rewrite lines 37-41 and 206-211 for sequential panels whose samples fan out on rayon; or, if 1 is a measured tuning outcome the owner wants to keep as a parameter, record the measured reason at the constant. Acceptance: either `std::thread::scope`, `AtomicUsize`, and both `Mutex`es are gone from `main` with the smoke and dump pins unchanged and the gallery in roster order, or the constant's doc names why 1 and the module doc no longer describes overlap that cannot occur.

Ruled (95): Replace the width-1 pool with a plain sequential loop holding the writer and results directly; delete the constant; rewrite the two comments. The keep-the-pool alternative is struck. No rendering change. See ../rulings.md.

### fuelscape-render-19 (medium, verification): ruling 95

Measurement provenance is stamped unchecked and accepted as any string

- Owner-gated: no

Resolution: (1) in the `fuelscape` recipe, refuse to run a `--dump` survey when `git diff --quiet HEAD -- crates/before crates/suanpan` fails (or stamp `git describe --always --dirty --abbrev=40` so a dirty tree is visible); (2) make `--dump` refuse to run without `FUELSCAPE_TIP` instead of writing `untracked` (ad-hoc renders may keep the fallback); (3) require the commit to be exactly 40 lowercase hex characters in `compact::validate` and in `build.rs`'s loop, so neither `untracked` nor `<sha>-dirty` can reach the committed dataset. Acceptance: a `--dump` run without `FUELSCAPE_TIP` exits nonzero naming the variable; a tamper line in compact/tests.rs setting `doc["meta"]["commit"] = "untracked"` is rejected naming the check; the same edit to a committed `crates/before/fuelscape/<op>.json` fails `cargo build -p before` naming the file. Construction: `FUZZFIT_GUEST_WASM=... cargo run --bin fuelscape -- --dump --max-bytes 2 --samples 1 --out <tmp> version_tick` without `FUELSCAPE_TIP`, then `--compact-from <tmp> --out <tmp2>`: the written `version_tick.json` carries `"commit":"untracked"` and passes build.rs's checks verbatim. Or edit one committed document's `meta.commit` to `"untracked"` and run `cargo build -p before`: it succeeds.

Ruled (95): Bind the stamp: the survey refuses a dirty tree under `crates/before` or `crates/suanpan` (or stamps `--dirty` visibly); `--dump` refuses without the tip variable; compact's validator and `build.rs` require exactly forty lowercase hex characters. See ../rulings.md.

## Roster members approved (ruling 104)

Lows and nits approved as this lane's roster by ruling 104. Land each per its quoted Resolution and Acceptance, swept with the ruled members; report rather than choose if a Resolution conflicts with a ruling or offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89).

### fuelscape-pipeline-11 (low, simplification): roster: approved (ruling 104)

split_sum's hand-written parallel/sequential branch, its reference twin, and the 2048-bit pin dissolve into rayon's with_min_len

- Owner-gated: no

Resolution: One chain over `*splits.start()..*splits.end() + 1` (a `Range<usize>`, which rayon indexes): `.into_par_iter().with_min_len(PAR_SPLIT_THRESHOLD).map(|a| &subtree[a] * &subtree[pair_sum - a]).sum::<BigUint>()`. Delete `split_sum_sequential`, both `build_sequential`, the `sum:` parameter of `build_with`, and `parallel_build_matches_sequential_reference`; restate the threshold doc as the `with_min_len` floor (rayon's splitter stops splitting when a half would fall below it, so the sequential regime is roughly twice the constant; the doc already calls the exact cut low-stakes). Acceptance: count.rs holds one convolution expression; count/tests.rs retains the enumeration and decoder pins; `just fuelscape-test` passes and its wall time drops by the four 2048-entry builds.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-12 (low, simplification): roster: approved (ruling 104)

VersionCounts and PartyCounts, and the two samplers, duplicate every method but the recurrence

- Owner-gated: no

Resolution: One `Counts<G: Grammar>` with the shared methods, a `Grammar` trait with one method (`fn entry(j: usize, subtree: &[BigUint]) -> BigUint`) and two unit types holding the recurrences; `VersionCounts = Counts<VersionGrammar>` and `PartyCounts = Counts<PartyGrammar>` keep the typed distinction the refutation notes. Behavior-preserving. Acceptance: count/tests.rs passes with constructor names unchanged; both recurrences read in one screen.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-15 (low, documentation): roster: approved (ruling 104)

The census literals' claimed independent derivation is not in the tree

- Owner-gated: no

Resolution: Re-state the doc against what the tree holds: the literals are the committed canonical-stream counts per bit length; the enumeration must reproduce them; the decoder census below re-derives the same numbers from the shipping parser, and this pin alone survives a coordinated change to both, so a canonical-form change edits these integers deliberately. Drop the reference to the out-of-tree program (or commit it as a test if its independence is wanted live). Acceptance: the doc comment names no derivation the tree does not contain and states the tamper-evidence role; the array is unchanged.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-24 (low, simplification): roster: approved (ruling 104)

slice_overlays dispatches on operation-name strings with a runtime panic as its only totality check

- Owner-gated: no

Resolution: Move the slice-family choice into the roster (`Inputs::VersionSlice(SliceFamilies)`, `VersionSliceCapped(u32, SliceFamilies)`), delete the name match, and keep the signature-keyed table for fixed-arity rows (a function of the signature). Keep the `other => panic!` at 385 only with a comment naming the smoke test as its check. Acceptance: no `match name` on string literals remains in families.rs; adding a slice row without choosing its families is a compile error.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-29 (low, documentation): roster: approved (ruling 104)

Constants restated as literals in stamped size-measure strings and overlay labels

- Owner-gated: no

Resolution: Build the stamped strings from the constants (`const_format::formatcp!`, a dependency in keeping with the crate's preference for libraries over hand-rolling) or assemble them at compaction time from `OpSpec` fields; label the party-fold overlays with `format!("... (k={PARTY_FOLD_OVERLAY_SHARES})")`. Failing that, one unit test in ops/tests.rs asserting each such string contains its constant formatted. Acceptance: changing `COMBINE_ARITY_CAP`, `FORKS_SHARES`, `TICKS_COUNT`, or `PARTY_FOLD_OVERLAY_SHARES` either needs no prose edit or fails a test.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-4 (low, simplification): roster: approved (ruling 104)

draw_inputs copies the member-draw closure seven times; the two slice arms differ by one expression

- Owner-gated: no

Resolution: Two helpers, `draw_version(samplers, n, rng, &mut rejected) -> Vec<u8>` and `draw_party(samplers, n, rng) -> Vec<u8>`, owning the two `expect` proofs; `draw_packed` and every arm call them. Match `Inputs::VersionSlice | Inputs::VersionSliceCapped(_)` in one arm with `let cap = match op.inputs { Inputs::VersionSliceCapped(c) => size.min(c as usize), _ => size }`, or give `VersionSlice` an `Option<u32>` cap. Acceptance: `run_op_is_deterministic_and_ordered` and the smoke test pass unchanged (the draw order is preserved); each `expect` string appears once in plan.rs.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-5 (low, verification): roster: approved (ruling 104)

The determinism test says its op list walks every input space; VersionSliceCapped has no replay pin

- Owner-gated: no

Resolution: Add `"shape_combine"` to the list and name the capped draw in the doc; better, derive the list from `ROSTER` by picking the first row per `Inputs` variant through a `match` with no wildcard arm, so a new variant is a compile error and the doc can say "one row per `Inputs` variant". Acceptance: every `Inputs` variant has a row in the replay; adding a variant without one fails to compile (derived) or the doc no longer says "every".

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-7 (low, verification): roster: approved (ruling 104)

The chi-square statistic is written three times; plan/tests.rs hardcodes the threshold sample/tests.rs derives

- Owner-gated: no

Resolution: A `#[cfg(test)]` helper module (`src/testing.rs`) with `chi_square(observed: &[u64], expected: f64) -> f64`, `chi_square_threshold(categories) -> f64`, and `assert_uniform(observed, label)`; the two sampler pins, the arity pin, and the split pin proposed in fuelscape-pipeline-9 call it. Acceptance: the literal `32.0` is gone from plan/tests.rs and one chi-square implementation exists in the crate.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-8 (low, verification): roster: approved (ruling 104)

fold_rows_expose_the_arity_axis_in_fuel asserts a sign with no margin, on a tuple sort that biases toward passing

- Owner-gated: no

Resolution: Sort by arity alone with a stable sort (`sort_by_key(|&(arity, _)| arity)`) and require a relationship with margin that costs no fuel threshold: a Spearman rank correlation between arity and fuel across the column at or above a stated bound, or the mean of the top-arity third exceeding the bottom-arity third by a factor the fold's `log k` model predicts. Commit the known-bad demonstration beside it. Acceptance: passing a constant arity to `op.measure` at plan.rs:394 while leaving `CellSample.arity` as drawn fails the test deterministically at the committed seed for both fold rows. Construction: At plan.rs:394 replace `(op.measure)(&mut guest, &inputs, arity)` with `(op.measure)(&mut guest, &inputs, 4)` for the `PartyShares` row (for `clock_join_all`, whose measure ignores the argument, fix the drawn clock count instead). Fuel is then independent of the recorded arity; sorting by `(arity, fuel)` and splitting at the median yields two means whose order depends only on which party bytes landed in which half at seed 0x5eed.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-9 (low, verification): roster: approved (ruling 104)

split_budget is pinned for reachability where the module doc claims exact uniformity, and the suite's own argument says reachability cannot pin uniformity

- Owner-gated: no

Resolution: Replace the reachability test with a one-sided chi-square over the ten compositions of (6, 3) at 2000 draws (expected 200 each; threshold `chi_square_threshold(10)` = 9 + 6 * sqrt(18), about 34.5, the sampler pins' idiom), which subsumes reachability (an unreached composition alone contributes 200). Acceptance: green on the current `split_budget`, red on either known-bad split below, at the committed seed. Construction: Known-bad A (stick-breaking): first part `gen_range(1..=total - parts + 1)`, recurse on the remainder; for (6, 3), P((4,1,1)) = 1/4 and P((1,1,4)) = 1/16 against the uniform 1/10; every composition is reachable, sum and positivity hold, and the chi-square over 2000 draws reads in the hundreds. Known-bad B (collision nudge): draw `parts - 1` cuts in `1..total` with replacement, sort, and move each duplicate to the next unused position; for (6, 3) the compositions with adjacent cuts carry 3/25 each and the rest 2/25, all reachable, statistic about 84.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-render-12 (low, simplification): roster: approved (ruling 104)

`compact.rs` and `dump.rs` carry one reader and one writer twice

- Owner-gated: no

Resolution: extract one generic dataset layer in `dump.rs` (or a sibling module): a `read` parameterized by the banner constants and the payload type with a per-document `validate` callback, and the matching writer; `dump` passes the grid check, `compact` passes `validate`; both keep their own constants and payload types. Acceptance: one reader body in the crate; `compact::read` and `dump::read` are thin calls; every existing compact and dump test passes unchanged.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-render-14 (low, verification): roster: approved (ruling 104)

The tamper test's doc undercounts its cases, and several enumerated rejections in both readers have no known-bad demonstration

- Owner-gated: no

Resolution: rewrite the doc as the family ("each structural rejection the module doc enumerates that a single-field edit can reach is alive") and let the `tamper` calls be the list; add the missing cases (`doc["op"]["extra"] = 1` expecting "unknown field"; `doc["op"]["op_name"] = "other"` expecting "index claims"; `doc["op"]["cols"] = []` expecting "one histogram per size column"; `doc["op"]["sizes"] = []` expecting "empty"; `doc["op"]["cols"][0]["c"] = []` expecting "empty"; through the index file, `ops: []`); lift the closure into dump/tests.rs for the dump reader's banner, run-parameter, name, empty-samples, and unknown-field checks. Acceptance: every `malformed(...)`/`reject(...)` site in `dump::read` and `compact::validate`/`read` is reached by a tamper case whose assertion names it; commenting out any one rejection branch turns a test red; neither test doc enumerates by hand. Construction: comment out dump.rs:225-233 (the op-name check) and run the fuelscape suite: nothing fails today; the proposed `doc["op"]["op_name"] = "other"` case would.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-render-29 (low, documentation): roster: approved (ruling 104)

`build.rs`'s module doc describes one of its two jobs and denies a duplication the script deliberately carries

- Owner-gated: no (the leaf-crate alternative would be)

Resolution: open the doc with both responsibilities and add a second inputs/outputs paragraph naming the three figure paths (or split the figure job into `mod figure;` with its own doc); reword the sentence to "holds no binning or statistical constant; the format banners and version are the one deliberate duplication, spelled on both sides of the package boundary"; hoist `const FORMAT_VERSION: u64 = 3;` and the two banner strings so the check and its message read one constant; and either add build.rs's overlay-positivity check or state at 280-282 which of the compactor's checks are deliberately not repeated. Acceptance: the module doc names every file build.rs reads and writes; no bare `3`/"v3" in `check_banner`; the doc names the duplicated identifiers.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-render-30 (low, verification): roster: approved (ruling 104)

`build.rs` reads two figure files it does not declare in `rerun-if-changed`, so the README-figure freshness check is dormant under incremental builds

- Owner-gated: no

Resolution: add `println!("cargo:rerun-if-changed=results/space_consumption/itc_space_consumption.svg");` and `println!("cargo:rerun-if-changed=docs/itc_space_consumption_readme.svg");` beside the existing four (the second may sit next to the env-changed line in `check_readme_figure_fresh` for locality). Acceptance: after a warm `cargo build -p before`, append a byte to `docs/itc_space_consumption_readme.svg` and build again: the script reruns (visible with `-vv`) and fails with "is stale relative to results/space_consumption". Construction: warm build of `before`; edit a color in `results/space_consumption/itc_space_consumption.svg` (in a scratch copy of the repo); `cargo build -p before -vv` shows `Fresh before` with no build-script run and no staleness panic; `cargo clean -p before` then fails with the stale-figure message.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-render-5 (low, documentation): roster: approved (ruling 104)

Unanchored design-system vocabulary in the palette constants

- Owner-gated: no

Resolution: state the checkable property in plain terms ("orange, chosen to stay distinguishable from the ramp's blue under common color-vision deficiencies"; "a 6% inset leaves a visible gap between adjacent occupied bins so cells stay countable"), or drop the qualifiers. Acceptance: the three phrases are gone and each color constant's doc names a checkable property or none.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### module-graph-7 (low, documentation): roster: approved (ruling 104)

before-fuelscape depends on suanpan directly on a claim that `before` exposes no touch reader; `before::meter::touch_ops` exists

- Owner-gated: no

Resolution: In spanbands.rs read `meter::reset_touch_ops()` and `meter::touch_ops()`, then drop the `suanpan` dependency and its comment (the detached workspace's lockfile updates with it); if the owner prefers the direct read, correct the comment to the actual reason. Acceptance: fuelscape's manifest has no `suanpan` line, or its comment states a true reason.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-10 (nit, documentation): roster: approved (ruling 104)

The count module says every count is pinned two independent ways; the version table reaches the decoder only through the enumeration

Where: `crates/before-fuelscape/src/count.rs:31-35`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "Every count is pinned against the grammar enumeration; the party table is also pinned directly against `Party::decode`'s accept census ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-16 (nit, simplification): roster: approved (ruling 104)

enumerate is a pub module with only test callers, and party_subtrees returns a documented tuple beside a named struct

Where: `crates/before-fuelscape/src/enumerate.rs:120`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `#[cfg(test)] pub mod enumerate;`; a `PartyMember` struct

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-17 (nit, documentation): roster: approved (ruling 104)

Two small doc inaccuracies: "zero pad" for the marker padding; EXHAUSTIVE_BYTES credits the wrong companion pin

Where: `crates/before-fuelscape/src/sample.rs:178-179`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "before the marker padding" at sample.rs:178 and 376; at sample/tests.rs:14-16 name count/tests.rs's decoder census (23 bits) as the companion that ca ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-18 (nit, simplification): roster: approved (ruling 104)

The zero-leaf exclusion is spelled through version_leaf_count at a corner argument; the two samplers differ in assert strength; a dead .max(1)

Where: `crates/before-fuelscape/src/sample.rs:289-290`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One `excluded_two_bit_leaf` helper; one assert strength; drop `.max(1)`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-20 (nit, simplification): roster: approved (ruling 104)

Two exhaustive byte-string sweeps of the decoders, in two modules, with two bound constants

Where: `crates/before-fuelscape/src/sample/tests.rs:40-61`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One parallel `accepted_byte_strings(len)` helper; one bound constant

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-22 (nit, simplification): roster: approved (ruling 104)

ramp's 1 << 20 guard contradicts its comment and names what it catches nowhere

Where: `crates/before-fuelscape/src/families.rs:58-60`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `const MAX_RAMP_KNOB` with the failure it catches

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-25 (nit, documentation): roster: approved (ruling 104)

"adding an operation is one OpSpec entry" names one step of the chain a new row requires

Where: `crates/before-fuelscape/src/ops.rs:4-5`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Replace the clause with a short "adding a row" list naming the kernel, the overlay arm (or its signature match), the re-measure and compact re-pin ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-26 (nit, documentation): roster: approved (ruling 104)

"constant dispatch overhead, identical for every sample" is one register move per operand on the fold rows

Where: `crates/before-fuelscape/src/ops.rs:9-12`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "plus the guest's dispatch overhead: constant for the fixed-signature rows, one register move per operand for the folds"

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-31 (nit, documentation): roster: approved (ruling 104)

Exemption reasons cite panels in unchecked prose; the Ticks entry names "min_ticks", a panel that is not a roster name

Where: `crates/before-fuelscape/src/ops.rs:2354-2358`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Write `version_min_ticks` at 2357. If the panel references are worth enforcing, structure the exemptions as `Exemption::NoSizeAxis(&str)` and `Exempti ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-pipeline-6 (nit, verification): roster: approved (ruling 104)

`arity_draw_reaches_every_count` is subsumed by the chi-square pin, and the known-bad draw it cites (statistic 723.2) exists only in prose.

Where: `crates/before-fuelscape/src/plan/tests.rs:153-178`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Delete the reachability test; build the biased draw in the uniformity test and assert its statistic exceeds the threshold.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-render-11 (nit, documentation): roster: approved (ruling 104)

nit: `expect` messages that name a hope, not the proof

Where: `crates/before-fuelscape/src/compact.rs:208-212`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "hi >= k0: max and min of the same nonempty list" and "k >= k0 by construction"

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-render-16 (nit, documentation): roster: approved (ruling 104)

nit: `write_atomic`'s crash claim outruns its mechanism

Where: `crates/before-fuelscape/src/dump.rs:168-175`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): narrow the doc to "a dying process" (matching dump.rs:16-18), or add `File::create` + `write_all` + `sync_all` before the rename (or use `tempfile::Na ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-render-2 (nit, simplification): roster: approved (ruling 104)

nit: long qualified paths at use sites, and a `use` block split by the allocator static

Where: `crates/before-fuelscape/src/render.rs:137-138`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Import at file top; one contiguous `use` block

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-render-20 (nit, simplification): roster: approved (ruling 104)

nit: stringly-typed table callback with a catch-all arm; allocator rationale duplicated with the manifest

Where: `crates/before-fuelscape/src/bin/fuelscape.rs:180-187`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): A `Table` enum for the callback; one allocator rationale

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-render-24 (nit, simplification): roster: approved (ruling 104)

nit: the pointer-to-viewBox conversion, the quantile-step handler, and the guide-tip scan are each written twice

Where: `crates/before/docs/fuelscape.js:568-569`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `svgPoint(e)`; route the slider through `quantKey`; derive `tip` from `vals`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-render-28 (nit, simplification): roster: approved (ruling 104)

nit: one sans-serif font stack spelled five times where the stylesheet already uses a token

Where: `crates/before/docs/fuelscape.css:54`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Add `--fs-sans` beside `--fs-mono`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-render-32 (nit, documentation): roster: approved (ruling 104)

nit: `fuelscape-claims` describes an acceptance rule the widget does not implement

Where: `tools/fuelscape-claims:36-38`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): replace both passages with the rule by name: "the rule is `Fuelscape.accepts`: every measured size must give >= 1 after the smallest constant argument ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuelscape-render-4 (nit, simplification): roster: approved (ruling 104)

nit: dead defaults and clamps in `aggregate` and the reference curves, palette hex repeated in the gallery CSS, and consequential drops with no comment

Where: `crates/before-fuelscape/src/render.rs:226-239`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Drop the dead defaults and clamps; derive gallery colors from the constants; comment the drops

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).


## Coordinator handoffs (recorded during the P2 surface lane's review)

- Compaction holds each dump's recorded `size_measure` to the roster
  row, so rewording the measures' vocabulary (ruling 110) required
  re-stamping every committed dump; the cheapest artifact that passes
  `fuelscape-verify` is therefore a re-stamped dump, and the mechanism
  cannot tell a vocabulary re-stamp from a re-stamp hiding a plan
  change (the coordinator verified this change's dumps moved only in
  that string, by decompressing each). The dump should carry the
  sampling plan's identity separately from its prose (the `Inputs`
  variant, the operand list, and the declared constants, serialized),
  so compaction pins the plan mechanically and the prose can be
  reworded without touching artifacts of record.
