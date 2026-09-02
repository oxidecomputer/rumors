<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P6 lane: the surface roster

## Goal

The surface roster's remaining entries, landed per their Resolutions inside an approved roster, after rulings 40, 47, and 49 (`p2-surface`) and 78 (`p5-scanners`) reshaped the machinery.

## Awaiting individual ruling

The coordinator is walking these mediums with Finch; nothing below lands for them until the ruling is appended here: surface-roster-4.

## Roster summary

0 ruled (); 1 medium awaiting ruling; 11 pending roster approval (3 low, 8 nit).

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

Every P6 lane runs after the P1 to P5 and P7 lanes that touch its files have landed on main, or rebases onto them before its final gate run; the coordinator names the base SHA at launch. Lows and nits inside this lane's approved roster are swept without a question to Finch; every high and medium has, or awaits, an individual ruling. A change that would alter a rendered `before` doc panel is a stop (ruling 89). Follows `p2-surface` and `p5-scanners`. Owns `src/surface.rs`, `src/testing/surface_coverage/**`, `crates/before/surfacecheck/**`, `crates/surface-scan/**`.

## Mediums awaiting individual ruling

Listed with their Resolution so the lane knows the files they touch; not landed until ruled.

### surface-roster-4 (medium, verification): awaiting individual ruling

The `Party::tick` row cites tests that never exercise `Party::tick`; the reduction law that does is cited by nothing

- Owner-gated: no

Resolution: cite `party_tick_matches_version_tick` on the three `Trans` legs of the `Party::tick` row, mirroring `Party::ticks`. Acceptance: `every_cited_binding_test_exists` resolves the new citation via `laws::registered_names()`; deleting the law from laws.rs turns the roster red. Construction: replace the body of `Party::tick` (party.rs:180) with `let _ = version;`; `roster_is_total_over_the_public_fn_surface`, `every_cited_binding_test_exists`, and `exclusion_payload_citations_resolve` stay green, and only the doctests and the uncited law fail. Then delete `party_tick_matches_version_tick`: the roster suite is still green.

Awaiting individual ruling: the coordinator is walking the P6 mediums with Finch now. Do not land this entry and do not choose among its alternatives; when the ruling arrives it is appended to this brief by the coordinator.

## Roster members pending Finch's approval

Lows and nits no ruling has reached, placed here by the files they touch. Land only after the coordinator confirms the roster is approved.

### surface-roster-12 (low, verification): roster: pending Finch's approval

Two of the three negative probes in the citation-haystack test cannot fail, and its doc cites a scan that no longer exists

- Owner-gated: no

Resolution: probe helpers declared under `src/` that an attribute-blind scan would admit (`declared_test_names_by_file`, `crate_root`, `extract_public_fns` from this module, or a helper `fn` adjacent to a `proptest!` block) and rewrite the doc positively. Acceptance: every name in the negative loop is found by `grep -rn 'fn <name>' crates/before/src`; temporarily removing the `if test_pending` guard at surface_coverage.rs:345 makes every negative probe fail; restored, the test is green.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-22 (low, verification): roster: pending Finch's approval

The item-grammar catch-all is silent where the type grammar is fail-loud, main.rs overclaims "every public item", and the skip's rationale is inaccurate

- Owner-gated: no for the enumerate-and-panic and the prose; widening the census to fields and variants is the owner's call

Resolution: enumerate the deliberately ignored variants explicitly (`Variant`, `StructField`, `TypeAlias`, `Primitive`, `ExternCrate`, ...) and panic on the remainder, matching `render_type`; correct main.rs:1-3 and 11-12 to name the three categories the gate covers; replace the rationale at 175 with the true one (shape is outside the roster's operation vocabulary). Owner's call: record public fields as `Type::field` and variants as `Type::Variant` item rows pinned in `census::ITEMS`, so a shape change reads red. Acceptance: an unhandled `ItemEnum` variant panics naming it in a unit test over a synthetic `Crate`; main.rs claims what the gate covers; or, under the widening, adding `pub extra: u8` to `shape::Plateau` turns `just surface-totality` red until pinned. Construction: add `pub extra: u8` to `pub struct Plateau` (shape.rs:88) and initialize it where `Plateau` is built. `just surface-totality` and `roster_is_total_over_the_public_fn_surface` stay green.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-24 (low, simplification): roster: pending Finch's approval

Census rows key trait impls by private definition paths and generic parameter names, record cross-type impls under both owners, and pin the compiler-internal `StructuralPartialEq`

- Owner-gated: no (internal to surfacecheck; one re-pin of `TRAIT_IMPLS`, attributed in its commit)

Resolution: render the trait as `<crate>::<TraitName><Args>` (first and last `paths` segments), which still disambiguates same-named traits across crates; render generic parameters positionally (`_`); dedupe in `record_impl` by impl `Id` so a cross-type impl is one row (disambiguating same-named for-types only on collision), and rewrite extract.rs:38-43 to name the actual mechanism if double rows are kept; decide `StructuralPartialEq` explicitly (skip beside `is_synthetic` with a one-line reason, or keep with the derived-vs-manual rationale stated). One re-pin of `TRAIT_IMPLS`, attributed to the renderer change. Acceptance: a unit test renders a synthetic `Path` whose `paths` entry is `before::causally::polarity::Polarity` as `before::Polarity`; after re-pinning, the gate is green and renaming the private `polarity` module moves no pin.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### clippy-pedantic-6 (nit, documentation): roster: pending Finch's approval

Six items whose first doc paragraph runs three to four lines, so the module listing shows a block instead of a name

Where: `crates/before/src/surface.rs:319-321`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): insert a blank `///` line after the first sentence at each site;

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-13 (nit, verification): roster: pending Finch's approval

`exclusion_payload_citations_resolve`'s doc omits registered descriptor names its body admits; `render_names_the_findings` says every category and exercises four of ten.

Where: `crates/before/src/testing/surface_coverage/tests.rs:181-185`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Fix the first doc; populate all ten `Findings` fields or narrow the second doc.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-14 (nit, verification): roster: pending Finch's approval

`tripwires_are_labeled` asserts the shape of labels nothing consumes (`cited_test_names` and citecheck read only the test name).

Where: `crates/before/src/testing/surface_coverage/tests.rs:343-353`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Delete the test.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-19 (nit, simplification): roster: pending Finch's approval

Em-dashes in `//` comments at six sites

Where: `crates/before/surfacecheck/src/check.rs:259-261`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Colons or semicolons in four files

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-26 (nit, simplification): roster: pending Finch's approval

Idiom nits: a mutation inside `Option::inspect`, leading `::` on an external crate path, a fully qualified `Range`, `push_str(&format!(..))`, an order-sensitive roster comparison, and an unnamed `20` at four sites

Where: `crates/before/surfacecheck/src/main.rs:44-46`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `if let` for `--list`; imports; `writeln!`; a named `MIN_REASON_LEN`

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-27 (nit, documentation): roster: pending Finch's approval

The clean-sweep census line mixes item counts with exception-entry counts, so its arithmetic identity does not hold

Where: `crates/before/surfacecheck/src/main.rs:118-133`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): report the parenthetical as item counts (items under item exceptions, items under module exceptions ...

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-30 (nit, verification): roster: pending Finch's approval

surface-scan fixtures hand-roll pid-keyed temp dirs under `temp_dir()` and never remove them.

Where: `crates/surface-scan/src/tests.rs:14-25`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `tempfile` as a dev-dependency; return the `TempDir`.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-8 (nit, simplification): roster: pending Finch's approval

Long `op` literals keep rustfmt from formatting `FAMILY_SURFACE`, leaving hand-formatted one-line rows beside vertical ones

Where: `crates/before/src/surface.rs:1128-1129`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Keep `op` to the identity; move the parentheticals to comments

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

