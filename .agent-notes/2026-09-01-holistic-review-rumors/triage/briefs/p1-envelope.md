<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P1 lane: the envelope certificate and the operating-envelope figures

## Goal

The window's 2^-40 per-session bound rests on integer envelopes dominating
exact tails. At the reviewed commit that dominance is certified only by
`examples/envelope_sim.rs`, which no recipe runs, imports nothing from the
crate, certifies the one-corpus family rather than the shipped pair-product
functions, and carries hand-copied constants that have rotted twice; so
lowering a shipped quantile passes the gate. Separately, the crate doc's
operating-envelope figures have no committed derivation, and `results/` is
dated, BLAKE3-denominated, and cites removed files. The invariant restored:
every number the crate's docs state is derived by a committed test from
the shipped functions, and the certificate of a bound is a test the gate
runs over the code that ships.

## Ground rules

These apply to every P1 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `0926fe32` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `0926fe32`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the topic document in
  `.agent-notes/2026-09-01-holistic-review-rumors/`; the entry's full record
  (evidence, construction, demonstration) is under `### <id>:` there and in
  `evidence/`. Line anchors are at the reviewed commit `9e5784fb`; re-anchor
  from the quoted evidence, never from the line numbers.
- **Goal beside mechanism.** Where a quoted resolution and the goal above
  come apart, the goal wins, and the discrepancy is reported.
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin; any change to a public
  signature or public rustdoc contract the resolution does not name;
  anything that contradicts a ruling in `triage/rulings.md`; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop on
  one entry does not block the others.
- **Negative controls.** Every repaired instrument lands with a committed
  demonstration that a known-bad artifact fails it. The constructions in
  `evidence/witness.md` and each entry's Construction line are those
  artifacts; convert each into a committed test (`should_panic`, an asserted
  `Err`, or a reversible mutation whose observed failure the commit message
  records verbatim).
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement. One full `just gate` before each commit,
  run in the background redirected to a log under
  `<scratchpad>/p1-envelope/`, polled with short foreground checks (the
  foreground command cap is ten minutes). Keep every working file under that
  directory.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and ruling. Commit every proptest seed
  file that appears. Prose speaks in the present tense: no reference to code
  that no longer exists, no dated rationale at a declaration site. Comments
  use spaced double-hyphens, never em-dashes; every test has a doc comment
  stating its invariant. Never delete anything outside your worktree; if the
  disk fills, stop and report.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why. Your report is data: the coordinator
  verifies each entry's Acceptance against the tree at the reported sha
  before the ledger records it. Report what you could not do rather than
  working around it.

## Ordering inside the lane

1. The differential proptest in `window/tests.rs` lands first and is shown
   to catch a lowered quantile (the negative control), then
2. `examples/envelope_sim.rs` is deleted whole, with its `[[example]]`
   entry in `Cargo.toml` and every recipe, workflow step, and prose
   reference to it, then
3. `window.rs` cites the test, then
4. the operating-envelope test and the `results/` excision.

Rebase onto `p1-gate` before your gate run if it has landed: both lanes
touch `src/lib.rs`.

## Members

Rulings T10 and T43 govern the envelope entries. T10 moves the certificate
into the crate: the exact-Chernoff oracle becomes a differential proptest
in `window/tests.rs` over the shipped `occupied`, `jointly_occupied`,
`children_quantile`, and `stage_population`, extended to asymmetric
`(A, B)`. T43 then deletes the example whole once that proptest
demonstrably fails on a lowered shipped quantile; nothing of its Monte
Carlo tiers is kept, and the deletion is not a stop. Every
`benches-envelope-*` entry anchored in the example (`-28`, `-29`, `-31`,
`-33`, `-34`) is resolved by the deletion and needs no work of its own.

### benches-envelope-32 (medium): ruling T10

Resolution: Move the exact-Chernoff oracle (`p_occ`, `binom_tail_log`, `chernoff_quantile`, `occ_hi`, `joint_hi`, `occ_quantile`, `stage_pop`; about 120 lines) into `src/tree/mirror/streaming/window/tests.rs` as a differential test `integer_envelopes_dominate_exact_chernoff` that sweeps the same `(N, depth)` grid against the real `occupied`, `jointly_occupied`, `children_quantile`, and `stage_population`, extending the oracle to asymmetric `(A, B)` (joint per-slot probability `p_occ(A) · p_occ(B)`) so the shipped pair-product path is covered. Re-state window.rs:570-578 to name that test. Delete the example's integer copies (538-682) and `check_integer_dominates`; if the Monte Carlo tiers are worth keeping they become an `#[ignore]`-gated test beside it, and what remains of the example is design exploration that retires to `.agent-notes/`. The cheaper interim step is `test = true` on the example in Cargo.toml (the `swarm` precedent at 194-198) with the two certification calls under `#[test]`, which closes the gating gap but not the replica gap. Acceptance: `just gate` runs a test that fails when any shipped integer quantile is lowered below its exact-Chernoff counterpart at a sampled `(A, B, depth)` (demonstrate once by a deliberate `- 1` on `bernstein`'s return); window.rs no longer names `examples/envelope_sim.rs`; the example's integer copies are gone or the example itself is.

Amendment (T10, T43): the "cheaper interim step" is not taken, the Monte
Carlo tiers are not kept, and nothing retires to `.agent-notes/`: the
example is deleted outright in step 2. The oracle moves from the example
into the test before the file goes; read it from the reviewed commit
`9e5784fb` if you need it after deletion.

### streaming-backend-window-32 (medium): ruling T10

Resolution: Add a committed proptest in window/tests.rs over log-uniform `(a, b)` corpora and `depth in 1..=KEY_DEPTH` asserting each integer quantile is at least the exact-distribution quantile at its stated tail level: `leaves_quantile(n, j)` against the least `q` with `P(Binomial(n, 256^-j) >= q) <= 2^-(48 + 8 min(j, 40))` via a log-space survival function; `jointly_occupied(n, pair, j)` against the Poisson upper tail at mean `pair / 256^j` and level 2^-48; `child_slots_quantile` against the occupied-child-slot count. Then decide the example's fate: a `just` leg (`cargo run --release --example envelope_sim`) or retirement once the in-tree test demonstrably catches what the example caught. Acceptance: a committed test fails when any integer quantile is lowered below its exact counterpart (demonstrate by changing `+ 2` to `+ 1` in `small_mean_quantile`, or `BERNSTEIN_TAIL` to 20, and observing the failure), and the module comment cites that test by name.

This and `benches-envelope-32` are one test; write it once, satisfying
both acceptances (both mutations are demonstrated and recorded in the
commit message). The example's fate is ruled: retirement (T43).

### benches-envelope-28, -29, -31, -33, -34: rulings T10, T43

Resolved by the deletion. No work beyond confirming, in the report, that
no reference to `envelope_sim` survives anywhere in the tree
(`grep -rn envelope_sim --exclude-dir=.agent-notes --exclude-dir=target .`
is empty, `.claude/` excluded).

### verification-infra-6 (medium): ruling T17

Resolution: derive the doc's figures in a committed test from the pinned per-message wire law (the `tests/dispute_wire.rs` constants) plus stated assumptions (fan-out, rounds per second), and have the doc cite the constant and its validity band; the owner decides the wording. The disposition of `results/` (re-denominate against SHA3-256 with its references restored, move its derivation into the test, or excise) is an owner call recorded under open questions. Acceptance: a test names each figure in src/lib.rs and fails when the wire law moves it out of band.

Ruled (T17): `results/` is deleted. The crate doc's sentence citing the
constant and band is Finch's prose: write it, and flag the exact sentence
in the report for his read; that is not a stop. `README.md` is derived
from `src/lib.rs` (`just readme`); regenerate it in the same commit.

## Hazards and stops

- The tail-level exponents and the `(A, B)` extension are the mathematics
  the bound rests on. If the differential fails on HEAD for any shipped
  function before any mutation is applied, that is a finding about the
  shipped envelope, a stop, reported with the failing `(A, B, depth)`.
- `window.rs` is production code; the only edit there is the citation
  comment naming the test.
- `results/` deletion touches nothing the gate reads (verify with a grep
  before deleting); if something does read it, stop.
- Deleting the example is ruled (T43); a recipe or workflow step that runs
  it is deleted with it, not stubbed.
