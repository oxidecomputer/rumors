<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P6 lane, board 5 of 6: the oracle and the laws (and the paper transcription)

## Goal

The oracle's and laws' entries, landed per their Resolutions inside an approved roster, under rulings 78 (the sequential `join_all`), 92 (the compile-time law-group tie, the bundled laws), 87 (the pair law), and 43.

## Awaiting individual ruling

The coordinator is walking these mediums with Finch; nothing below lands for them until the ruling is appended here: oracle-laws-13.

## Roster summary

2 ruled (2 low); 1 medium awaiting ruling; 18 pending roster approval (8 low, 10 nit).

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

Every P6 lane runs after the P1 to P5 and P7 lanes that touch its files have landed on main, or rebases onto them before its final gate run; the coordinator names the base SHA at launch. Lows and nits inside this lane's approved roster are swept without a question to Finch; every high and medium has, or awaits, an individual ruling. A change that would alter a rendered `before` doc panel is a stop (ruling 89). Follows `p5-buffers` and `p7-api`. Owns `src/oracle/**`, `src/laws.rs`, `reference/`.

## Members

### oracle-laws-16 (low, verification): ruling 92

Bundled laws report one name for seven to ten independent clauses

- Owner-gated: yes (`Law<F>` is a `pub type` under the `laws` feature; splitting moves roster citations)

Resolution: Either split each bundle into one law per clause (the `surface.rs` citations of `ranked_carries_own_rank` and `span_is_the_pair_hull` move with them), or change the predicate to `fn(...) -> Result<(), &'static str>` returning the failing clause's name and have the three consumers report `law/clause`. The second keeps roster and citations stable but changes a feature-gated public type. Acceptance: a deliberately negated inner conjunct (say `commutative` in `span_is_the_pair_hull`, in a scratch build) produces a driver or fuzz report naming that clause.

Approved roster (92, decision 72): lands per the quoted Resolution inside this lane.

### oracle-laws-25 (low, simplification): ruling 92

The law-group totality pin is a source-text scan where a compile-time tie exists, and the scan is not total over the syntax `laws!` accepts

- Owner-gated: yes (replaces an instrument; the replacement must demonstrate it catches what the scan catches)

Resolution: Have `emit_registration` (already expanded from the roster, `#[cfg(test)]`) also emit `mod rostered { pub(super) use super::{VERSION_SOLO, ...}; }` from the roster, and have `laws!` append a `#[cfg(test)] fn` (or `const _`) whose body references `rostered::$group`; an unrostered group then fails `cargo check --tests` at its own declaration, the text scan and the convention comment at laws.rs:187-190 go, and `REGISTERED_GROUPS` dissolves or reduces to the alias module. `diff_ops.rs` carries the same scan pattern and would take the same tie. If the scan is kept instead, strip leading `#[...]` groups before matching (or match `pub static ` anywhere on lines without `$`) so it is total over the macro's syntax. Acceptance: laws/tests.rs has no `fs::read_to_string`; declaring `laws! { pub static PHANTOM: (a: &Version); fn x { true } }` without a roster entry fails to compile; or, in the fallback, the attributed-header case fails `every_law_group_is_registered`.

Approved roster (92, decision 72): lands per the quoted Resolution inside this lane. Under ruling 43: the law-group tie is a compile-time, typed reference, never a source scan.

## Mediums awaiting individual ruling

Listed with their Resolution so the lane knows the files they touch; not landed until ruled.

### oracle-laws-13 (medium, verification): awaiting individual ruling

Two oracle test doc comments claim invariants their bodies never assert

- Owner-gated: no

Resolution: Assert the claims or trim the docs. Additivity is constructible on every call: fork a trace member's party into `keep`/`give`, take `x = v / &keep` and `y = v / &give`, assert `(x | y).min_ticks() == x.min_ticks() + y.min_ticks()` and `(x | y).min_ticks() >= x.min_ticks().max(y.min_ticks())`. For `clock_own_version`, tick a clone and assert `own_version` strictly rises under `leq`. Delete the redundant line 809 either way. Acceptance: each sentence of both doc comments corresponds to an assertion in its body.

Awaiting individual ruling: the coordinator is walking the P6 mediums with Finch now. Do not land this entry and do not choose among its alternatives; when the ruling arrives it is appended to this brief by the coordinator.

## Roster members pending Finch's approval

Lows and nits no ruling has reached, placed here by the files they touch. Land only after the coordinator confirms the roster is approved.

### oracle-laws-1 (low, simplification): roster: pending Finch's approval

The oracle Clock's one-to-one mirror claim is false: `has_seen` has no caller, the observer trio and `receive` mirror deleted or renamed production methods, and the module doc claims a single omission

- Owner-gated: no

Resolution: Delete `has_seen` (then `Version::leq` at oracle/version.rs:99 can drop from `pub(super)` to private; its only other caller is `PartialOrd for Version`). Either rename `receive` to `recv` and replace `happens_before`/`concurrent_with` at their four call sites (oracle/tests.rs:386-387, 401; clock/tests.rs:188) with `partial_cmp`-based spellings, or keep them and rewrite oracle/clock.rs:9-11 to say the oracle mirrors the paper's operations under its own names, listing the divergences (owned-`Version` messages, `receive` for `recv`, no n-ary or absorb entries). Rewrite oracle.rs:9-12 to name what the oracle mirrors (the paper's operations) rather than claim one omission. Reword clock/tests.rs:172-174 to name the comparisons the body performs (`>=`, `<`, `concurrent`); that site belongs to the clock partition and should be carried there. `Default for oracle::Version` stays. Acceptance: `grep -rn has_seen crates/before` returns nothing; both mirror sentences are true of the code beneath them; `just gate` clean.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### oracle-laws-10 (low, verification): roster: pending Finch's approval

The arbitrary generators' normal-form claim has no direct pin

- Owner-gated: no

Resolution: Add one proptest beside `normal_form` (or in `testing/generators/tests.rs`) asserting `arb_oracle_party()` and `arb_oracle_version()` outputs satisfy `is_normal()`. Acceptance: removing the `debase` step from `Version::node` (version.rs:82-84) or the collapse arm from `Party::node` (party.rs:24-25) fails the new test by name.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### oracle-laws-14 (low, verification): roster: pending Finch's approval

Two incidental-only laws lack a constructed arm, and nothing measures antecedent liveness

- Owner-gated: no

Resolution: Add a constructed arm to each (the `constructed && incidental` shape at 933-937): `disjoint_projections_share_nothing` on `(keep, give)` from `p.dangerously_alias().fork()`; the eq/hash trio on `(a, decode(encode(a)))`. Optionally one deterministic test asserting each incidental antecedent is satisfiable on a small fixed population, as a liveness pin. Acceptance: every conditional law either runs a constructed arm on every call or carries a comment naming why none is constructible; law names unchanged.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### oracle-laws-17 (low, simplification): roster: pending Finch's approval

Law predicates that `unwrap` lose the failure's name

- Owner-gated: no

Resolution: Use the let-else `return false` idiom at 699, 713, 725, 726, 1219, 1222, 1314, 1426, 1491, 1848 (a small `fn ordered(lo, hi) -> Option<Span<'_>>` beside `within` keeps the bodies short); `operand_spans` at 1705 may keep its `expect` or return `Option`. Acceptance: `grep -n 'unwrap()\|expect(' crates/before/src/laws.rs` returns only the two infallible `split_last` sites (and optionally 1705).

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### oracle-laws-21 (low, verification): roster: pending Finch's approval

The acceptance laws' `Err` arms assert less than their docs say, and the clock group has no best-effort law

- Owner-gated: no

Resolution: Weaken both docs to what the clause checks (the accumulator is never corrupted on refusal), and add `clock_join_all_is_best_effort_at_any_width` as the twin of the party law (fork `width` children, tick them apart, plant an alias of the keeper mid-stream, expect exactly the alias back and the keeper's party restored with the join of every line's version). Acceptance: each `Err`-arm sentence maps to a clause; a fail-fast `Clock::join_all` fails a `CLOCK_AND_LIST` law by name.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### oracle-laws-9 (low, documentation): roster: pending Finch's approval

The oracle test module doc says trees are never fabricated directly; three suites draw generated or literal trees

- Owner-gated: no

Resolution: Restate the header: values come from seed-derived op traces (pairwise party-disjoint populations), from the normalizing arbitrary generators, or from paper literals whose normality the test asserts or which are already normal. Acceptance: the module doc names all three input sources the file uses.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### paper-fidelity-6 (low, documentation): roster: pending Finch's approval

laws.rs header attributes the crate's extensions to the paper and cites §2-§4

- Owner-gated: no

Resolution: cite §3-§4 (and §5 for the trees) and split the sentence into the paper's algebra (join semilattice whose order is causality; ids under disjoint sum with fork as split; event as strict inflation within the id) and the crate's extensions (meet and the distributive lattice, rank as valuation and metric, projection, span, causally); fix `semantic_oracle.rs:11` to §4. Acceptance: every property the header attributes to the paper appears in §3-§5 of the transcription.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### paper-fidelity-8 (low, documentation): roster: pending Finch's approval

the oracle's grow defines two arms the paper does not, undocumented at the oracle

- Owner-gated: no

Resolution: a short paragraph on `grow` naming both extensions: the `1`-over-node arm is the paper's `(il, ir)` rule read on the unnormalized `(1, 1)`, present so the optimality proptests can quantify over arbitrary pairs; the empty-id arm is the infeasible sentinel; `event` reaches neither. Acceptance: a paper-reader diffing the oracle against §5.3.4 finds every untranscribed arm named with its reason.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### oracle-laws-11 (nit, simplification): roster: pending Finch's approval

Pool indices are drawn as `0..64` and reduced modulo the population at fourteen sites where the crate elsewhere uses `prop::sample::Index`

Where: `crates/before/src/oracle/tests.rs:46-55`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `prop::sample::Index` at the fourteen sites

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### oracle-laws-12 (nit, verification): roster: pending Finch's approval

A `prop_assume!(!cands.is_empty())` that never rejects (the party is drawn nonempty), beside siblings that spell the premise as `.expect(..)`.

Where: `crates/before/src/oracle/tests.rs:597-600`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Drop the assume; use the siblings' `expect`.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### oracle-laws-15 (nit, simplification): roster: pending Finch's approval

`le` is `le_by` at one type

Where: `crates/before/src/laws.rs:248-258`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Keep `le`; delete `le_by`

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### oracle-laws-18 (nit, verification): roster: pending Finch's approval

`merge_is_least_upper_bound` and its meet dual check only a join-built bound; the clause that means "least" (an arbitrary `c` above both implies above the join) is absent.

Where: `crates/before/src/laws.rs:861-877`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Conjoin the incidental implication clause, here and in oracle/tests.rs.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### oracle-laws-20 (nit, verification): roster: pending Finch's approval

`span_all_is_the_family_hull`'s containment clause admits `Concurrent` placements its own argument rules out.

Where: `crates/before/src/laws.rs:1850-1851`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Use the existing `within` helper.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### oracle-laws-23 (nit, verification): roster: pending Finch's approval

`clock_ticks_matches_version_ticks` pins a hard-coded count of 3 where the version-level twin draws `a.min_ticks()`.

Where: `crates/before/src/laws.rs:3033-3033`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Use the operand's count.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### oracle-laws-3 (nit, simplification): roster: pending Finch's approval

`unreachable!("party overlap")` is a label, not a proof, and a denormal literal reaches the arm

Where: `crates/before/src/oracle/party.rs:82-82`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): State the premise in the message, or make `sum` total

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### oracle-laws-5 (nit, simplification): roster: pending Finch's approval

Three spellings of the grow-cost tuple with a hand-maintained "matches" comment

Where: `crates/before/src/oracle/version.rs:12-12`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `pub(crate) Cost` returned from `grow_for_test`; drop `GrowCost`

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### oracle-laws-6 (nit, simplification): roster: pending Finch's approval

`join_off` and `meet_off` are one recursion differing only in the leaf combiner

Where: `crates/before/src/oracle/version.rs:114-154`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One `lattice_off` helper taking the leaf combiner

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### oracle-laws-8 (nit, simplification): roster: pending Finch's approval

Em-dashes in `//` line comments at ten sites

Where: `crates/before/src/oracle/version.rs:449-453`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Colons or ` -- `; batch with the crate-wide sweep

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

