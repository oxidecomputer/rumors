<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P6 lane: the surface roster

## Goal

Retire the surface roster's per-row citations under ruling 98 and replace them with coverage totality over the public surface; land the lane's remaining rows only where they survive that regime. Rulings 47 and 49 (`p2-surface`) and 78 (`p5-scanners`) reshape the machinery this lane builds on; ruling 40's binding work is moot (see below).

## Ruling 98: the regime this lane lands

Finch's words: "If there's a part of the surface roster that can't be mechanically enforced in a meaningful way, it really doesn't deserve to exist; it's just window-dressing, verification cosplay, at that point."

- **What dissolves.** The per-row test citations on the three coverage legs, the `Trans` legs' "anchors the reduction" claims, the exclusion payloads' prose reasons, and the `FAMILY_SURFACE` rows. Nothing can check that a cited test exercises the item it is cited for, so none of it stays.
- **What replaces it.** surfacecheck enumerates the public surface from rustdoc JSON (with hidden items, ruling 91, and auto traits, ruling 49), and a coverage floor requires every public item's body to be hit by the test suite: covcheck's per-file presence floor (ruling 91, `p6-tools`) extended to per-public-item over the surfacecheck census. The only surviving roster payload is the fail-closed exclusion of a deliberately untested item with its reason; an item leaving the coverage report reads red.
- **Retirement discipline.** The coverage floor demonstrably fires on an unexercised public item (a committed known-bad or a deliberately emptied test body, shown red) before any citation is deleted.
- **Consequences for other entries.** surface-roster-7 (ruling 40, `p2-surface`): the `FAMILY_SURFACE` binding is moot and its three prose sites go with the roster; the missing impls are covered by totality. gate-legs-8 (ruling 50, `p1-gate`): the `encode_to_matches_encode` law extension over the rank, ranked, and span writers still lands as a test; its citation half is moot. surface-roster-6 (ruling 77, dup): same. tests-other-16 (ruling 99, `p6-harness`): the foreign re-export census rows land in surfacecheck here (foreign re-exports, foreign-target type aliases, extern crates reconciled against a committed empty set), and `tests/foreign_reexport.rs` is deleted once the census demonstrably catches `pub use bytes;`.
- **The roster rows below are re-read under this regime.** Each low and nit in this lane is a finding about a roster mechanism that is dissolving; the lane lands a row only if its defect survives in the replacement (surfacecheck, the coverage floor, the exclusion list), and otherwise reports it as moot, naming what dissolved it. The coordinator records each as `fix` or `dup` against ruling 98.

## Rulings on this lane's mediums

Every medium in this lane is ruled (rulings 93 to 103): surface-roster-4. The decisions stand beside each entry under Members.

## Roster summary

0 ruled (); 1 medium ruled (93 to 103); 11 roster members approved (ruling 104) (3 low, 8 nit).

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

Every P6 lane runs after the P1 to P5 and P7 lanes that touch its files have landed on main, or rebases onto them before its final gate run; the coordinator names the base SHA at launch. Lows and nits inside this lane's approved roster are swept without a question to Finch; every high and medium has an individual ruling. A change that would alter a rendered `before` doc panel is a stop (ruling 89). Follows `p2-surface`, `p5-scanners`, and `p6-tools` (covcheck's per-file floor, ruling 91). Owns `src/surface.rs`, `src/testing/surface_coverage/**`, `crates/before/surfacecheck/**`, `crates/surface-scan/**`, `tests/foreign_reexport.rs`, and covcheck's per-public-item extension.

## Mediums ruled 93 to 103

Each medium below now carries its ruling and any amendment beside its quoted Resolution; land per the ruling.

### surface-roster-4 (medium, verification): ruling 98

The `Party::tick` row cites tests that never exercise `Party::tick`; the reduction law that does is cited by nothing

- Owner-gated: no

Resolution: cite `party_tick_matches_version_tick` on the three `Trans` legs of the `Party::tick` row, mirroring `Party::ticks`. Acceptance: `every_cited_binding_test_exists` resolves the new citation via `laws::registered_names()`; deleting the law from laws.rs turns the roster red. Construction: replace the body of `Party::tick` (party.rs:180) with `let _ = version;`; `roster_is_total_over_the_public_fn_surface`, `every_cited_binding_test_exists`, and `exclusion_payload_citations_resolve` stay green, and only the doctests and the uncited law fail. Then delete `party_tick_matches_version_tick`: the roster suite is still green.

Ruled (98): Amendment: the citation is not repaired; under ruling 98 the per-row citations dissolve and coverage totality over the public surface replaces them (see this brief's regime section). The `party_tick_matches_version_tick` law itself stays. See ../rulings.md.

## Roster members approved (ruling 104)

Lows and nits approved as this lane's roster by ruling 104. Land each per its quoted Resolution and Acceptance, swept with the ruled members; report rather than choose if a Resolution conflicts with a ruling or offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89).

### surface-roster-12 (low, verification): roster: approved (ruling 104)

Two of the three negative probes in the citation-haystack test cannot fail, and its doc cites a scan that no longer exists

- Owner-gated: no

Resolution: probe helpers declared under `src/` that an attribute-blind scan would admit (`declared_test_names_by_file`, `crate_root`, `extract_public_fns` from this module, or a helper `fn` adjacent to a `proptest!` block) and rewrite the doc positively. Acceptance: every name in the negative loop is found by `grep -rn 'fn <name>' crates/before/src`; temporarily removing the `if test_pending` guard at surface_coverage.rs:345 makes every negative probe fail; restored, the test is green.

Roster note: approved by ruling 104 and re-read under ruling 98: lands per the quoted Resolution only if the defect survives the roster's dissolution; otherwise reported as moot, naming what dissolved it. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-22 (low, verification): roster: approved (ruling 104)

The item-grammar catch-all is silent where the type grammar is fail-loud, main.rs overclaims "every public item", and the skip's rationale is inaccurate

- Owner-gated: no for the enumerate-and-panic and the prose; widening the census to fields and variants is the owner's call

Resolution: enumerate the deliberately ignored variants explicitly (`Variant`, `StructField`, `TypeAlias`, `Primitive`, `ExternCrate`, ...) and panic on the remainder, matching `render_type`; correct main.rs:1-3 and 11-12 to name the three categories the gate covers; replace the rationale at 175 with the true one (shape is outside the roster's operation vocabulary). Owner's call: record public fields as `Type::field` and variants as `Type::Variant` item rows pinned in `census::ITEMS`, so a shape change reads red. Acceptance: an unhandled `ItemEnum` variant panics naming it in a unit test over a synthetic `Crate`; main.rs claims what the gate covers; or, under the widening, adding `pub extra: u8` to `shape::Plateau` turns `just surface-totality` red until pinned. Construction: add `pub extra: u8` to `pub struct Plateau` (shape.rs:88) and initialize it where `Plateau` is built. `just surface-totality` and `roster_is_total_over_the_public_fn_surface` stay green.

Roster note: approved by ruling 104 and re-read under ruling 98: lands per the quoted Resolution only if the defect survives the roster's dissolution; otherwise reported as moot, naming what dissolved it. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-24 (low, simplification): roster: approved (ruling 104)

Census rows key trait impls by private definition paths and generic parameter names, record cross-type impls under both owners, and pin the compiler-internal `StructuralPartialEq`

- Owner-gated: no (internal to surfacecheck; one re-pin of `TRAIT_IMPLS`, attributed in its commit)

Resolution: render the trait as `<crate>::<TraitName><Args>` (first and last `paths` segments), which still disambiguates same-named traits across crates; render generic parameters positionally (`_`); dedupe in `record_impl` by impl `Id` so a cross-type impl is one row (disambiguating same-named for-types only on collision), and rewrite extract.rs:38-43 to name the actual mechanism if double rows are kept; decide `StructuralPartialEq` explicitly (skip beside `is_synthetic` with a one-line reason, or keep with the derived-vs-manual rationale stated). One re-pin of `TRAIT_IMPLS`, attributed to the renderer change. Acceptance: a unit test renders a synthetic `Path` whose `paths` entry is `before::causally::polarity::Polarity` as `before::Polarity`; after re-pinning, the gate is green and renaming the private `polarity` module moves no pin.

Roster note: approved by ruling 104 and re-read under ruling 98: lands per the quoted Resolution only if the defect survives the roster's dissolution; otherwise reported as moot, naming what dissolved it. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### clippy-pedantic-6 (nit, documentation): roster: approved (ruling 104)

Six items whose first doc paragraph runs three to four lines, so the module listing shows a block instead of a name

Where: `crates/before/src/surface.rs:319-321`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): insert a blank `///` line after the first sentence at each site;

Roster note: approved by ruling 104 and re-read under ruling 98: lands per the quoted Resolution only if the defect survives the roster's dissolution; otherwise reported as moot, naming what dissolved it. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-13 (nit, verification): roster: approved (ruling 104)

`exclusion_payload_citations_resolve`'s doc omits registered descriptor names its body admits; `render_names_the_findings` says every category and exercises four of ten.

Where: `crates/before/src/testing/surface_coverage/tests.rs:181-185`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Fix the first doc; populate all ten `Findings` fields or narrow the second doc.

Roster note: approved by ruling 104 and re-read under ruling 98: lands per the quoted Resolution only if the defect survives the roster's dissolution; otherwise reported as moot, naming what dissolved it. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-14 (nit, verification): roster: approved (ruling 104)

`tripwires_are_labeled` asserts the shape of labels nothing consumes (`cited_test_names` and citecheck read only the test name).

Where: `crates/before/src/testing/surface_coverage/tests.rs:343-353`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Delete the test.

Roster note: approved by ruling 104 and re-read under ruling 98: lands per the quoted Resolution only if the defect survives the roster's dissolution; otherwise reported as moot, naming what dissolved it. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-19 (nit, simplification): roster: approved (ruling 104)

Em-dashes in `//` comments at six sites

Where: `crates/before/surfacecheck/src/check.rs:259-261`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Colons or semicolons in four files

Roster note: approved by ruling 104 and re-read under ruling 98: lands per the quoted Resolution only if the defect survives the roster's dissolution; otherwise reported as moot, naming what dissolved it. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-26 (nit, simplification): roster: approved (ruling 104)

Idiom nits: a mutation inside `Option::inspect`, leading `::` on an external crate path, a fully qualified `Range`, `push_str(&format!(..))`, an order-sensitive roster comparison, and an unnamed `20` at four sites

Where: `crates/before/surfacecheck/src/main.rs:44-46`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `if let` for `--list`; imports; `writeln!`; a named `MIN_REASON_LEN`

Roster note: approved by ruling 104 and re-read under ruling 98: lands per the quoted Resolution only if the defect survives the roster's dissolution; otherwise reported as moot, naming what dissolved it. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-27 (nit, documentation): roster: approved (ruling 104)

The clean-sweep census line mixes item counts with exception-entry counts, so its arithmetic identity does not hold

Where: `crates/before/surfacecheck/src/main.rs:118-133`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): report the parenthetical as item counts (items under item exceptions, items under module exceptions ...

Roster note: approved by ruling 104 and re-read under ruling 98: lands per the quoted Resolution only if the defect survives the roster's dissolution; otherwise reported as moot, naming what dissolved it. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-30 (nit, verification): roster: approved (ruling 104)

surface-scan fixtures hand-roll pid-keyed temp dirs under `temp_dir()` and never remove them.

Where: `crates/surface-scan/src/tests.rs:14-25`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `tempfile` as a dev-dependency; return the `TempDir`.

Roster note: approved by ruling 104 and re-read under ruling 98: lands per the quoted Resolution only if the defect survives the roster's dissolution; otherwise reported as moot, naming what dissolved it. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### surface-roster-8 (nit, simplification): roster: approved (ruling 104)

Long `op` literals keep rustfmt from formatting `FAMILY_SURFACE`, leaving hand-formatted one-line rows beside vertical ones

Where: `crates/before/src/surface.rs:1128-1129`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Keep `op` to the identity; move the parentheticals to comments

Roster note: approved by ruling 104 and re-read under ruling 98: lands per the quoted Resolution only if the defect survives the roster's dissolution; otherwise reported as moot, naming what dissolved it. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

