<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P5 lane: the bench judge and the expired instruments

## Goal

Every instrument whose justifying constraint has expired is gone, each
with its replacement shown first: the bench judge (wall time, `just all`
only, behind a quiet-machine precondition CI cannot meet) retires in
favor of fuzz-fit's fuel bands, with `benches/tripwire.rs`'s quadratic
committed as an above-band fuzz-fit case before deletion; the A/B arms
whose question is settled, the probes criterion's `--profile-time`
replaces, `spanbands`, the cliff-fan family, and the `overlay` field
retire with their records closed; `tools/digestshare` is deleted because
the byte-pinned snapshots already hold what it checked. `fuzz_decode`
stays. Rulings 78 (decision 77) and 79 (decisions 49 and 78) fix the
shape; this brief carries them, plus four pending lows of the same kind
(dead or unconsumed instrument code).

## The retirement discipline

Every dissolution in this lane follows the doctrine's rule for retiring
an instrument: the replacement demonstrates it catches what the
instrument caught before the instrument is deleted. Concretely, each
retirement is one commit series: first the replacement (a unit test,
a fuzz-fit case, a clippy lint confirmed on a fixture, a surfacecheck
reconciliation) landed and shown firing on the artifact the old
instrument existed to catch, with the firing quoted in the commit
message; then the deletion, in a commit that names the replacement.
A retirement whose replacement cannot be shown to fire does not land:
it is reported as a finding about the replacement and the entry stays
open. Deletions leave no trace in the code (no "formerly", no retired
names in prose); the commit message carries the provenance.

## Ground rules

These apply to every lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `33779b10` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `33779b10`, fast-forward; if it has diverged, stop and report. Never call
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

- Judge retirement, in order: (1) commit the tripwire's kernel shape as
  a fuzz-fit case that reads above band (the one fuzz-fit calibration
  run this lane may make; if `p1-fuzz` is mid-flight on `bands.rs`, wait
  for it); (2) delete `tools/benchjudge`, its expected-value roster,
  `tests/bench_judge_roster.rs`, the sidecar's judged classes, and `just
  all`'s call, restating the `just all` recipe comment; (3) restate any
  prose naming the judge. Ruling 26's load stamp is superseded and is
  not built; `p1-gate` carries no item for it.
- Decision 49's retirements are independent commits; the `bitvec`
  dev-dependency goes with `emit_probe.rs` and `perf_probe.rs`;
  `deny.toml` and the lockfiles follow (`p1-gate` edits the lockfile
  audit; rebase).
- digestshare's deletion removes a `gate-lints` and `ci` leg;
  `workflowlint`'s roster and `tools/mutantcheck`-style expected files
  are updated in the same commit, and `p1-gate`'s `ci`-from-`gate-lints`
  derivation (ruling 50, gate-legs-4) is rebased onto.
- This lane owns `tools/benchjudge*`, `tools/digestshare*`,
  `benches/{amplify,emit_probe,perf_probe,tripwire}.rs` and the bench
  sidecars, `tests/bench_judge_roster.rs`, the A/B arm sites in
  `src/version/skyline/build.rs` and `codec`, `fuelscape/spanbands`,
  the cliff-fan family in `registry.rs` and `board/family.rs`, and the
  render's `overlay` field. `p1-board` and `p1-suites` also edit
  `registry.rs`; rebase.

## Hazards and stops

- A tripwire shape that does not read above band under fuzz-fit is a
  stop: the judge stays until Finch rules again, and the entry reports
  the reading.
- Deleting the cliff-fan family moves board and envelope rows; each
  removal is named in its commit with the rows it deletes, and no other
  row's reading may move.
- `just all` and `just ci` must stay green at every commit; a recipe
  that references a deleted tool is a gate failure, not a follow-up.

## Members

### tools-5 (medium, simplification): ruling 78

The bench judge's stated unique class is also priced by fuzz-fit fuel, and its committed tripwire is a fuel-visible quadratic

Resolution: owner decision, two exits. (a) Keep the leg: restate its unique class as non-instruction cost at the board's families and acceptance scale (bulk-memory operations that fuel prices as one instruction, native codegen, memory hierarchy), and commit a tripwire that demonstrates it, reading RED under the judge and GREEN under fuzz-fit's bands; if no such kernel can be built at the board's byte scales, that failure is the evidence for (b). (b) Retire the leg after committing a fuzz-fit demonstration that tripwire.rs's shape reads above-band under fuel, then remove benchjudge, its roster, tests/bench_judge_roster.rs, tripwire.rs, the sidecar's `Ceiling` machinery, and the two recipes, and let the fuzz-fit row absorb the class. Either way the index's two rows stop claiming one class, and tripwire.rs:12 stops saying "what no deterministic meter can". Acceptance: either a committed time-only tripwire fails `just bench-judge-tripwire` and is banded green by `just fuzzfit`, with validation_index.rs stating that class; or the leg is gone and `just fuzzfit` demonstrably reads the retired tripwire's shape red. Construction: for (b), transplant `unmetered_quadratic` (tripwire.rs:42-52) into the fuzz-fit guest as a banded kernel and run `just fuzzfit`: it reads above-band, so the judge catches nothing the bands do not. For (a), build a kernel whose wasm lowering is a single `memory.copy`/`memory.fill` per step over a growing buffer (instruction count linear in steps, bytes moved quadratic) and show RED under the judge and GREEN under fuel.

Ruled (78, decision 77): retire. The one construction this entry asks for that survives is the demonstration in the other direction: commit `benches/tripwire.rs`'s kernel shape as a fuzz-fit above-band case before deleting the judge, so the retirement shows its replacement catching what the judge claimed.

Ledger note: bench judge retired

### tools-6 (medium, verification): ruling 78

The roster's `red` class is documented as a buffer for owned reds awaiting cures, and the mechanism accepts any cell

Resolution: bind the class to bench code the way `Ceiling` already is: the sidecar declares a tripwire flag at the cell's definition (only `display_schoolbook/hugeleaf` and tripwire.rs's cell qualify), `load_cells` cross-checks the two sidecars agree, and `roster_violations` requires roster `red` to equal the sidecar's tripwire set exactly, so a library cell can never be rostered and an owned regression must become a cure or a declared model. Rename the key (`tripwire`) so an awaiting-cure entry needs a schema change that `roster_schema_carries_expectations_only` and `load_roster` both refuse. Reword benchjudge:65-80, the JSON notes, and bench_judge_roster.rs:7 and 57-59 (drop "owned red"). Acceptance: self-test cases: a roster naming a non-tripwire cell is an input error (exit 2); a tripwire cell missing from the roster is an input error; the existing laundering pins still hold; the three descriptions agree. Construction: add `"version_rank/harmonic"` to `red` in tools/benchjudge-expected.json and to the expected array at bench_judge_roster.rs:62. If that cell ever reads RED, `just bench-judge` exits 0 and the gate's test suite is green: a regression accepted by two one-line edits, with no cure and no declared model.

Ruled (78, decision 77; decision 48 half in ruling 79): moot with the judge deleted; the `red` class goes with the tool. Nothing of the Resolution's binding is built.

Ledger note: bench judge retired

### tools-7 (medium, verification): ruling 78

Unrostered cells drift GREEN and SKIP freely, so a board cell whose bench body goes dark passes the judge

Resolution: add a judged-set expectation: cells whose hi median sat well above the floor at pinning (a 10x margin, so noise cannot cross it) must read GREEN or RED, never SKIP, and such a cell reading SKIP is a violation. Declare it where the ceiling class is declared (the sidecar, a per-cell `judged` flag asserted against a pinned set), so the roster still carries expectations only and `roster_schema_carries_expectations_only` stays as is. Acceptance: a self-test case in which a judged-declared cell with a sub-floor pair returns exit 1 in roster mode; the construction below returns 1 instead of 0. Construction: at board.rs:169 replace `|body| black_box(body())` with `|body| black_box(&body)` and run `just bench-judge`: every board cell reads SKIP (hi median below 10 us) or a flat GREEN, `display_schoolbook/hugeleaf` still reads RED, `roster_violations` finds nothing, exit 0.

Ruled (78, decision 77): moot; the judge retires with its roster test.

Ledger note: bench judge retired

### meter-adequacy-4 (low, verification): ruling 78

The bench judge's judged set is unpinned: any unrostered cell may leave judgment as SKIP with no diff

Resolution: pin the expected sub-floor set without pinning a noisy threshold: add a `may_skip` expectation class to the roster (membership pinned in `tests/bench_judge_roster.rs`) listing the cells accurately under the floor; a listed cell may read GREEN or SKIP (so noise at the floor never flips a verdict), an unlisted cell reading SKIP is a violation, and RED stays a violation everywhere. Replace the prose count at floors.rs:99 with a reference to that list. Acceptance: the roster carries the class, the schema pin at bench_judge_roster.rs:82 admits it, and the self-test asserts an unlisted SKIP exits 1. Construction: lower `BENIGN_BASE_CLOCKS` (family.rs:359) until a benign cell's hi median falls under 10 µs; `just bench-judge` renders it SKIP and exits 0 with the roster satisfied.

Ruled (78, decision 77): moot; the judge retires. This entry closes with the deletion commit and needs no pin of the judged set.

Ledger note: bench judge retired

### tools-12 (nit, documentation): ruling 78

The roster's `notes` duplicate measured exponents held at sidecar.rs:73-78 and enumerate six of `TEXT_CEILING_CELLS`' seven text cells by hand

Where: `tools/benchjudge-expected.json:2`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Trim `notes` to the roster's contract, point at `TEXT_CEILING_CELLS`, keep each measurement at one site

Ruled (78, decision 77): moot; the judge retires.

Ledger note: bench judge retired

### benches-examples-1 (medium, simplification): ruling 79

benches/amplify.rs re-lists three board cells the judged board bench already times

Resolution: Delete benches/amplify.rs and the `[[bench]] name = "amplify"` stanza (Cargo.toml:139-141). If the join-with-a-one-tick-version operand is wanted, add it as a board row so it gets the judge, rather than keeping a parallel list. Acceptance: `grep -rn amplify crates/before/benches crates/before/Cargo.toml` is empty; `just bench-build` and `just clippy` stay green; the pinned sidecar written by `just bench-judge` still lists `version_decode/hugeleaf`, `version_join/bigroot`, and `party_without/id-pair`.

Ruled (79, decision 49): retire. Each retirement is one commit naming what settled the instrument's question; the A/B records are closed as cd171c29 closed the stacks leg. `fuzz_decode` alone stays, with one sentence at the target naming its unique payload.

### benches-examples-12 (medium, verification): ruling 79

The presize A/B record has arms compiled into production source, no recorded verdict, and an unpinned deterministic column

Resolution: close the leg the way the stacks leg was closed: take the record per the recipe's protocol (one `shipped` run plus one run per arm), write the verdict into a note and the closing commit, then dissolve the three cfg seams, the check-cfg roster, `bench-alloc-ab`, and `alloc_arms`. Keep only what the other benches lack: the `projection_outgrow` family (unique to this file) can move to benches/version.rs's hole group; `presize/display` and `presize/parse` duplicate the board's `version_display` and `version_parse_*` rows. If end-state resident bytes matter, pin them as a deterministic test in tests/meter.rs (frozen capacity minus length per site, with a floor) rather than printing them. Acceptance: `grep -rn before_alloc_ab crates/before Cargo.toml justfile` is empty; the closing commit names the measured arm ratios; if a resident-bytes test lands, it fails when query.rs:514 is changed to `let capacity = 0;` and passes on the shipped arm. Construction: `RUSTFLAGS='--cfg before_alloc_ab="projection_growth"' cargo bench -p before --bench presize -- --sample-size 10 --measurement-time 1`: the `presize-resident site=projection` lines move relative to the shipped build, and no committed test names the moved quantity; `just test-all` under the same RUSTFLAGS exercises transient peak-heap pins, which say nothing about end-state residency.

Ruled (79, decision 49): retire. Each retirement is one commit naming what settled the instrument's question; the A/B records are closed as cd171c29 closed the stacks leg. `fuzz_decode` alone stays, with one sentence at the target naming its unique payload.

### benches-examples-21 (medium, simplification): ruling 79

emit_probe.rs exercises no code in the tree, is the sole reason `bitvec` is a dev-dependency, and its one assert names a path that does not exist

Resolution: delete the example and the `bitvec` dev-dependency (and the workspace entry, which nothing else uses); the note is the record. If a standing primitive-cost comparison is wanted, write it as a criterion `[[bench]]` that times the crate's actual `BitsBuf`/`PackedBuilder` against `bitvec`, so a regression is a criterion-tracked number, and fix the assert message to the guard it enforces (`(1u64 << spill) - 1` overflows at `len == 64`). Acceptance: `grep -rn bitvec crates/ Cargo.toml` returns nothing (or the probe times `before`'s builder); `just check` and `just bench-build` clean; the perf-probe README updated to say the probe retired and where its numbers live.

Ruled (79, decision 49): retire. Each retirement is one commit naming what settled the instrument's question; the A/B records are closed as cd171c29 closed the stacks leg. `fuzz_decode` alone stays, with one sentence at the target naming its unique payload.

### meter-registry-tier2-12 (medium, simplification): ruling 79

The cliff-fan family prices a path-sum walk no production operation performs, and its only named pin re-runs the comb stream

Resolution: owner's call between two accurate states. (a) Dissolve the family: remove `FamilyId::CliffFan`, `Shape::CliffFan`, the `cliff_fan` generator, its size pin, and `accum_fan_touches_flat`; keep the shape in the agreement corpora only if it is re-justified as a coding corpus member rather than an adversary. (b) Re-justify it against a production walk that exists and add a two-point band built through `Shape::CliffFan` on that kernel, cited from the family's `Bands::Priced`, with a known-bad kernel that fails it. Either way the `EnvelopeOnly`/`Unbanded` reasons must name what exists (finding 10). Acceptance: either `grep -rn CliffFan crates/before` is empty, or a convention-named flatness test builds `Shape::CliffFan` at two scales over a public or documented-internal operation and the parity test passes.

Ruled (79, decision 49): retire. Each retirement is one commit naming what settled the instrument's question; the A/B records are closed as cd171c29 closed the stacks leg. `fuzz_decode` alone stays, with one sentence at the target naming its unique payload.

### tools-19 (medium, verification): ruling 79

digestshare passes a vacant, missing, or partially drifted corpus, keeps a dead V1 branch, and tracebacks on a headerless file

Resolution: refuse a nonexistent directory (exit 2, named); make the floor unconditional (zero files, zero wire bytes, or zero digests exits 1 naming the directory); delete the V1 handling and its prose so every `.snap` is a capture; require `total > 0` per file, failing by name (digests per file stay corpus-wide, since a capture with no listing legitimately has none); turn a file without two `---` into a named problem. Acceptance: `./tools/digestshare <empty dir>` and `<missing dir>` exit nonzero; rewriting one committed capture's `control item N (B bytes)` lines to another spelling fails naming that file; a headerless file is reported, not a traceback; `grep -c V1 tools/digestshare` is 0; the committed corpus still prints its table and TOTAL line.

Ruled (79, decision 78): the tool is deleted; the floor repair is moot.

Ledger note: tool deleted

### benches-examples-22 (low, documentation): ruling 79

perf_probe.rs frames itself as a one-off for a past investigation and documents a feature flag that is always on

Resolution: rewrite the module doc as what the file is ("Profiler harness: one `#[inline(never)]` loop per hot operation over the bench corpus, so a sampling profiler attributes cycles per operation; the wall-time record is the criterion suite"); drop the cfg gates and the `--features oracle` note. Acceptance: no "one-off" or "investigation" framing in the file; `cargo check -p before --example perf_probe` with no feature flags builds and the oracle loops are unconditional.

Ruled (79, decision 49): retire. Each retirement is one commit naming what settled the instrument's question; the A/B records are closed as cd171c29 closed the stacks leg. `fuzz_decode` alone stays, with one sentence at the target naming its unique payload.

### deps-10 (low, simplification): ruling 79

the alloc-A/B arm roster lives in three hand-synced places, and the check-cfg comment's "cannot drift" holds in one direction only

Resolution: replace the recipe's case list with a run-time provenance assertion in the bench: the recipe exports `BEFORE_ALLOC_AB_ARM={{ arm }}` beside RUSTFLAGS, and a small wrapper called at bench start panics when the requested arm is not the compiled one (`alloc_arms()` stays as the label printer). check-cfg then remains the sole roster, kept in sync with the seams by `deny`. Acceptance: `just bench-alloc-ab presize nosuch_arm` fails before any measurement, naming the compiled arm; adding a fourth arm requires touching only a seam and the check-cfg roster.

Ruled (79, decision 49): retire. Each retirement is one commit naming what settled the instrument's question; the A/B records are closed as cd171c29 closed the stacks leg. `fuzz_decode` alone stays, with one sentence at the target naming its unique payload.

### deps-7 (low, simplification): ruling 79

the bitvec dev-dependency and two one-off probe examples outlive the investigation that justified them

Resolution: retire examples/emit_probe.rs and the `bitvec` dev-dependency (its question is closed and it exercises no crate code); decide separately whether perf_probe.rs stays as a profiling harness, and if it goes, re-aim the agent note's sentence at git history (the note is exempt from the ghost rule; the Cargo.toml comment is not). Acceptance: `bitvec` absent from crates/before/Cargo.toml and the root lock; `just gate` clean.

Ruled (79, decision 49): retire. Each retirement is one commit naming what settled the instrument's question; the A/B records are closed as cd171c29 closed the stacks leg. `fuzz_decode` alone stays, with one sentence at the target naming its unique payload.

### fuelscape-pipeline-32 (low, simplification): ruling 79

spanbands is a one-off investigation binary that re-implements the plan's split rule and hand-rolls its CLI

Resolution: If the banding question is settled, record the verdict in an agent note and delete the binary (and the `scan-meter`/`limb-meter` features Cargo.toml:18-21 pulls for it, if nothing else reads them). If it is still wanted, make it a `--native-counters` mode of the main runner that draws through `plan` (a `pub(crate) draw_pair`, or `draw_packed` made `pub(crate)`) and uses the shared clap `Args`. Acceptance: either the binary is gone with its conclusion recorded, or it draws through `plan` and a change to `split_budget` changes its pairs.

Ruled (79, decision 49): retire. Each retirement is one commit naming what settled the instrument's question; the A/B records are closed as cd171c29 closed the stacks leg. `fuzz_decode` alone stays, with one sentence at the target naming its unique payload.

### fuelscape-render-10 (low, simplification): ruling 79

`overlay` is compacted, validated, and committed for a widget revision that does not exist, at a sixth of the dataset's bytes

Resolution: either drop `overlay` from `WidgetOp` (with its validate branch and tamper case), bump `FORMAT_VERSION`, and re-derive with `just fuelscape-compact`; or keep it and restate the doc in the present tense with the measured cost ("the widget does not draw these; carrying them, about 2 KB per operation, lets a widget that does read the same document"). Acceptance: `du` of crates/before/fuelscape drops by roughly 215 KB and `just fuelscape-verify` is clean; or the field doc states the present fact and the measured share.

Ruled (79, decision 49): retire. Each retirement is one commit naming what settled the instrument's question; the A/B records are closed as cd171c29 closed the stacks leg. `fuzz_decode` alone stays, with one sentence at the target naming its unique payload.

### fuelscape-render-21 (low, simplification): ruling 79

`spanbands` is referenced by nothing, records no result, hand-rolls its CLI against the manifest's clap rationale, and alone justifies two `before` features and the `suanpan` dependency

Resolution: owner's call: (a) record the discriminator's result (which pair coordinate separates the `version_span` bands, and what it decided about the classify-first versus emit-always trade) in an agent note or the span kernel's docs, and delete the binary together with the `scan-meter`/`limb-meter` features and the `suanpan` dependency in this manifest; or (b) keep it, port the flags to a clap `Args` struct, add a `just spanbands` recipe with the question stated, and give it a smoke test. Acceptance: either the binary and its three manifest lines are gone and a note holds its result, or `just --list` names it, `--help` derives the flag docs, and `--sizes 64,abc` exits with a clap error rather than a panic.

Ruled (79, decision 49): retire. Each retirement is one commit naming what settled the instrument's question; the A/B records are closed as cd171c29 closed the stacks leg. `fuzz_decode` alone stays, with one sentence at the target naming its unique payload.

### fuzz-guests-pins-4 (low, simplification): ruling 79

`fuzz_decode`'s assertions are implied by `fuzz_decode_differential`'s borsh arm

Resolution: Either keep the target with one sentence in its module doc naming the payload it alone provides, or dissolve it into the differential target, re-homing its 16 committed seeds (including the two non-derivable frontier witnesses) under `seeds/fuzz_decode_differential` and updating `fuzz_seed_set.rs`. Acceptance: the target's doc names its unique payload, or the target is gone with `tests/fuzz_seeds.rs` green.

Ruled (79, decision 49): `fuzz_decode` stays. Land only the one sentence at the target naming its unique payload (byte identity of re-encoding); the Resolution's retirement branch is struck.

Ledger note: fuzz_decode stays; its payload sentence lands

### skyline-coding-28 (low, simplification): ruling 79

the allocation A/B arm is compiled into the production renderer for a settled experiment

Resolution: owner decision. Retire the `display_growth` arm (delete text.rs:351-354's cfg pair, the value from Cargo.toml:107's check-cfg list, the arm in presize.rs and benches/common/mod.rs, and the justfile case) with a DECIDED entry on 1300ced09's template, keeping the exact-size assert as the pin; or keep it and record at the check-cfg registration that the arms are a standing re-runnable record. Acceptance: if retired, `just gate` and `just ci` clean and `bench-alloc-ab`'s arm list matches the remaining cfg values; if kept, the rationale is stated once at the registration.

Ruled (79, decision 49): retire. Each retirement is one commit naming what settled the instrument's question; the A/B records are closed as cd171c29 closed the stacks leg. `fuzz_decode` alone stays, with one sentence at the target naming its unique payload.

### tools-3 (low, simplification): ruling 79

digestshare's gate role is already carried by the byte-pinned snapshots, and its ratio has no in-tree consumer

Resolution: owner call between (a) moving `digestshare` out of `gate-lints` and `ci` into the conveniences section, keeping the tool (with tools-19's floor) for interactive runs, and (b) committing a consumer of the TOTAL line (a rustdoc figure or a pinned expectation) so the leg guards a claim rather than itself. Acceptance: either gate-lints no longer lists digestshare, or a committed artifact cites the ratio and the gate checks it.

Ruled (79, decision 78): the tool is deleted; this entry's gate-role question is answered by the deletion.

Ledger note: tool deleted

## Roster members approved (ruling 104)

Lows and nits approved as this lane's roster by ruling 104, placed here by the files they touch and the kind of dissolution. Land each per its quoted Resolution and Acceptance, swept with the ruled members; report rather than choose if a Resolution conflicts with a ruling or offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89).

### fuelscape-pipeline-13 (low, simplification): roster: approved (ruling 104)

Dead accessors: counts() on both samplers, max_bits() on both tables, BitSink::len and is_empty

Resolution: Delete the six methods. If a plan-time feasibility check is intended (a plan whose span exceeds the table), write it and keep `counts()` with that caller. Acceptance: `cargo check` in crates/before-fuelscape is clean with the methods removed.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names and the retirement discipline. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuelscape-render-26 (low, simplification): roster: approved (ruling 104)

`__FS_NO_ANIM` and the `Fuelscape.parse` export have no consumer

Resolution: delete the `noAnim` branch (keeping the `prefers-reduced-motion` path) and the `parse` export; or land the DOM or node test that uses them and cite it at the hook. Acceptance: grep for `__FS_NO_ANIM` across the tree returns nothing or returns a test; `Fuelscape`'s exports are exactly what tools/fuelscape-claims and the page use.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names and the retirement discipline. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzz-guests-pins-19 (low, simplification): roster: approved (ruling 104)

`ff_reset` has no caller

Resolution: Delete it. Acceptance: `grep -rn ff_reset crates` returns nothing.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names and the retirement discipline. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzzfit-bands-18 (low, simplification): roster: approved (ruling 104)

Generated data and hand-written prose share one file, which is what the splice marker, the column-zero template, and the duplicated rustdoc exist to manage

Resolution: Keep the constant declarations and their rustdoc in `bands.rs` and have calibrate emit only the array bodies and the rustc literal into data files (`src/bands/pinned_bands.rs`, `pinned_small_bands.rs`, `pinned_refit_coverage.rs`, `pinned_rustc.rs`, each `include!`d as an expression, since `include!` cannot carry item docs), plus the `PIN_EVIDENCE` value from finding 2. The marker search, its expect, the column-zero literal, the template prose, and the whole-file rewrite of a hand-edited file all dissolve; calibrate's row rendering becomes one `fn` over `Band`. Acceptance: calibrate writes no file containing `///` lines; `bands.rs` is never rewritten by a tool; `just fuzzfit-calibrate` on unchanged code yields no diff; `just fuzzfit`'s fmt and clippy legs stay clean.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names and the retirement discipline. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

