<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P6 lane, board 6 of 6: meter core (meter.rs and its tests)

## Goal

The meter core's entries (the generators, `Packed`, the traffic taps), landed per their Resolutions inside an approved roster, under rulings 8 (instrument surface; a change lands with the rumors test update), 50 (meter-core-8), 62 (the scan-meter scope), 74 (the comb docs), 43, and 88.

## Awaiting individual ruling

The coordinator is walking these mediums with Finch; nothing below lands for them until the ruling is appended here: meter-core-2.

## Roster summary

0 ruled (); 1 medium awaiting ruling; 6 pending roster approval (5 low, 1 nit).

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

Every P6 lane runs after the P1 to P5 and P7 lanes that touch its files have landed on main, or rebases onto them before its final gate run; the coordinator names the base SHA at launch. Lows and nits inside this lane's approved roster are swept without a question to Finch; every high and medium has, or awaits, an individual ruling. A change that would alter a rendered `before` doc panel is a stop (ruling 89). Follows `p1-suites`, `p2-surface`, `p4-ghosts`, `p4-rosters`. Owns `src/meter.rs` and `src/meter/tests.rs`.

## Mediums awaiting individual ruling

Listed with their Resolution so the lane knows the files they touch; not landed until ruled.

### meter-core-2 (medium, verification): awaiting individual ruling

Twenty registry-dispatched generators have no size or canonicality pin; four exact closed forms are wrong; six event shapes never meet a strict decode

- Owner-gated: no

Resolution: Add `check_version`/`check_party` pins for all twenty at two sizes, correcting the six closed forms above (spell the gamma-length sums out as `hole_region_bits` already does; `staircase` is exactly `6d + 2`) and fixing the capacity hints; then add one roster-wide pin that walks every `Shape` variant through the matching `check_*` so a new constructor cannot enter `Shape::builder` unpinned. Consider additionally routing `Packed::version()` through `Version::decode` of the transcoded bytes under `cfg(any(test, feature = "meter"))`, since construction happens before counters are reset and the only cost is one validation pass per shape. Acceptance: every arm of `Shape::builder` names a generator that appears in a meter/tests.rs pin asserting its `bits` against a closed form and round-tripping the shape; the construction below fails a committed test; the module doc's sentence at 29-32 is true of every variant.

Awaiting individual ruling: the coordinator is walking the P6 mediums with Finch now. Do not land this entry and do not choose among its alternatives; when the ruling arrives it is appended to this brief by the coordinator.

## Roster members pending Finch's approval

Lows and nits no ruling has reached, placed here by the files they touch. Land only after the coordinator confirms the roster is approved.

### meter-core-10 (low, documentation): roster: pending Finch's approval

`arming_train`'s band is documented as `32w + ⌈log₂ n⌉ + 2` but computed as `32w + bitlen(n) + 2`, and the test re-spells the code instead of the doc

- Owner-gated: no

Resolution: State the band as `32w + bitlen(n) + 2` (equivalently `32w + ⌊log₂ n⌋ + 3`) in both docs, and have the test call `bitlen(n)` or, better, assert the doc's own spelling as an independent expression. Acceptance: the doc formula evaluated by hand at `n = 1, 2, 4` equals the generator's `band`; tests.rs:1259 no longer re-spells `bitlen`.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### meter-core-13 (low, verification): roster: pending Finch's approval

The pointwise-dominance premise of two rank bands is witnessed by rank order and by the rank fold itself, not by the comparison sweep

- Owner-gated: no

Resolution: Replace the `checked_sub` assertion in both tests with `assert_eq!(a.partial_cmp(&b), Some(Ordering::Greater), ..)`; keep `lag == ZERO` if the rank identity is wanted as a second leg, named as such. Acceptance: both tests witness dominance through `partial_cmp`; no message claims dominance over a rank-order check.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### meter-core-14 (low, verification): roster: pending Finch's approval

The seam-plunge control's wire-prefix check contradicts its comment and carries an underived 200-bit slack

- Owner-gated: no

Resolution: Either check the identity exactly (bit-level prefix equality up to the control stream's live length less its final code, via `BitsView`), or keep the byte-prefix check with a named, derived slack (`SEAM_CONTROL_FINAL_CODE_BITS = 1 + 2 * 68 - 1` plus a stated padding-and-divergence allowance) and rewrite the doc and comment to describe the check performed. Acceptance: the assertion's bound is a named constant with its derivation beside it; the test doc, the comment, and the code describe the same check; the test passes at the three `(k, r)` points.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### meter-core-6 (low, simplification): roster: pending Finch's approval

Construction bodies are pasted across families that differ by one knob

- Owner-gated: no

Resolution: Extract `rearm_block(bits, arm: &Base)` for the four block loops; `memo_site(bits, left: &Base, right: &Base)` and `memo_site_id(bits)`; `hole_units(ev, id, k, m)` plus one root-site helper for the hole pairs; give `reveal_comb` a `floor: &Base` parameter with two thin wrappers like `ascend_spine`; hoist the seam-plunge asserts into one shared check; give `gap_spine` a turn-leaf and phase parameter and route `arming_train`, `jump_pair_operand`, and `puncture_product` through it, collapsing the two stride constants into one. Land the pins from meter-core-2 first so the refactor has a byte-identity oracle for every family it touches. Acceptance: each listed bit pattern has one definition site; every existing and new `check_version`/`check_party` pin passes unchanged; tests/meter.rs needs no re-pin.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### meter-core-9 (low, documentation): roster: pending Finch's approval

`factor_digit` and `dense_factor` docs differ from their code in the mixing input and a forced bit

- Owner-gated: no

Resolution: State the mix as the finalizer over `seed ⊕ (i · φ)` naming the golden-ratio constant, and add the fourth forcing to `dense_factor`'s list: "bit 0 forced clear and bit 1 forced set, so digit 0 is even and nonzero". Acceptance: both docs match the code line for line.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### meter-core-12 (nit, verification): roster: pending Finch's approval

`check_version`'s doc claims "canonicality of both codings" while no validator for the construction language exists; the hoisted-window comment describes a rank agreement the body reduces to `>=`.

Where: `crates/before/src/meter/tests.rs:27-29`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Restate both docs to what the bodies check.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

