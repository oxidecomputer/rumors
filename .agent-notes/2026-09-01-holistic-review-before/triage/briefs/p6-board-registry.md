<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P6 lane, board 4 of 6: the family registry and tier2

## Goal

The registry's and tier2's entries, landed per their Resolutions inside an approved roster, under ruling 43 (typed references; the rosters made drift-proof and idiomatic: this lane is where that direction bites hardest), 61, 78 (no `decided` dates), 79 (the tier2 prose and formula), and 88.

## Rulings on this lane's mediums

Every medium in this lane is ruled (rulings 93 to 103): meter-registry-tier2-16. The decisions stand beside each entry under Members.

## Roster summary

0 ruled (); 1 medium ruled (93 to 103); 7 roster members approved (ruling 104) (2 low, 5 nit).

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

Every P6 lane runs after the P1 to P5 and P7 lanes that touch its files have landed on main, or rebases onto them before its final gate run; the coordinator names the base SHA at launch. Lows and nits inside this lane's approved roster are swept without a question to Finch; every high and medium has an individual ruling. A change that would alter a rendered `before` doc panel is a stop (ruling 89). Follows `p2-surface` (meter-registry-tier2-10), `p4-rosters`, `p5-buffers`. Owns `src/meter/{registry,tier2}.rs` and their tests.

## Mediums ruled 93 to 103

Each medium below now carries its ruling and any amendment beside its quoted Resolution; land per the ruling.

### meter-registry-tier2-16 (medium, simplification): ruling 97

The 1-Lipschitz coding pin is implied pointwise by the subadditivity pin over the same emitters and populations; its leaf clause asserts a count, not containment

- Owner-gated: no

Resolution: move the leaf clause into `check_subadditive` and state it as what it is (a leaf-count bound; or check containment by comparing boundary positions through the oracle's dyadic intervals if containment is the claim the board's denomination rests on); rename the merged helper to state both clauses; delete `JOIN_MEET_BOUNDARY_SLACK_BITS`, its derivation, `check_join_meet_lipschitz`, and the four Lipschitz tests; carry the Lipschitz grid's larger operands (dense(512), bigroot(200, 100), hugeleaf(500), cliff_comb(64, 64)) into `adversarial_crosses_hold_subadditivity`; move the sentence "the statement the board's input denomination of the packed-output mutators rests on" onto the subadditivity constant's doc; re-point cell.rs:72-75 and this file's module doc (1-3) to the subadditivity pin; fix the two emitter docs. Acceptance: one coding-lemma helper and one constant; four fewer tests; `grep -n Lipschitz crates/before/src` finds no referent in meter or board; the leaf clause survives inside the merged check and its doc matches its assertion. Construction: dominance needs no run. To confirm the moved leaf clause is live, weaken it to `<=` in a scratch build and observe `empty_pair_is_the_subadditivity_equality_case`'s operands (1 + 1 leaves, output 1 leaf) still pass while `so.leaves == sa.leaves + sb.leaves - 1` cases distinguish `<` from `<=`; restore.

Ruled (97): Fold the leaf clause into the subadditivity check as a leaf-count bound; delete the Lipschitz helper, its constant, and its four tests; carry its larger operands into the subadditivity grid; re-point the two docs. The containment-check alternative is struck. See ../rulings.md.

## Roster members approved (ruling 104)

Lows and nits approved as this lane's roster by ruling 104. Land each per its quoted Resolution and Acceptance, swept with the ruled members; report rather than choose if a Resolution conflicts with a ruling or offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89).

### meter-registry-tier2-15 (low, simplification): roster: approved (ruling 104)

The sizer suite builds shapes through the raw generators, bypassing the registry door

- Owner-gated: no

Resolution: route both suites through `Shape` (`Shape::Dense.packed1(512)`, `Shape::Bigroot.packed2(200, 100)`, `Shape::Hugeleaf.packed1(500)`, `Shape::CliffComb.packed2(64, 64)`, and so on), after which registry.rs:14-15 is exactly true; or name the child suites in the registry doc as the sanctioned exception. Acceptance: no generator name is imported into tier2/tests.rs or board/tests.rs, or the registry doc names them.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### meter-registry-tier2-21 (low, documentation): roster: approved (ruling 104)

`grid_version::build` recurses on depth outside the `recurse.rs` inventory of test-local recursive witnesses

- Owner-gated: no

Resolution: build the grid iteratively by pairing bottom-up (which removes the recursion and the inventory question), or route the two recursive calls through `descend!` and add the site to recurse.rs's inventory with the bounded-depth note. Acceptance: recurse.rs's inventory names every test-local recursive fn, or `build` is iterative.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### meter-registry-tier2-18 (nit, verification): roster: approved (ruling 104)

The tier-2 ratio-floor loop divides two closed-form literals already asserted; no measured quantity enters.

Where: `crates/before/src/meter/tier2/tests.rs:213-220`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Delete the loop and the three hand-computed ratios.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### meter-registry-tier2-2 (nit, documentation): roster: approved (ruling 104)

The module doc calls the band-to-family link compiler-checked; the compiler checks only band-to-Shape

Where: `crates/before/src/meter/registry.rs:46-50`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): reword to "the band-to-shape link is a compiler-checked construction site; the band-to-family link is the spec's `Bands` roster ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### meter-registry-tier2-20 (nit, verification): roster: approved (ruling 104)

The plain-sweep witness's `>= 1.8` floor is underived and sits 0.04 above the deterministic 1.841 reading; `closing` is built inside the metered region.

Where: `crates/before/src/meter/tier2/tests.rs:261-268`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): State or compute the expected ratio; hoist `closing` above the reset.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### meter-registry-tier2-4 (nit, documentation): roster: approved (ruling 104)

Shape notation letters collide: B, F, W, A each name two shapes

Where: `crates/before/src/meter/registry.rs:100-174`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): give the later coinages distinct abbreviations (the two-letter style the newer families already use: `MB`, `MF`, `DR` ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### meter-registry-tier2-5 (nit, documentation): roster: approved (ruling 104)

`wrong_door` points at the variant doc instead of naming the accessor; the door argument is stated twice

Where: `crates/before/src/meter/registry.rs:384-386`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): give `Builder` an `fn accessor(&self) -> &'static str` and have `wrong_door` print "{self:?} builds through {right}, not {called}" ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

