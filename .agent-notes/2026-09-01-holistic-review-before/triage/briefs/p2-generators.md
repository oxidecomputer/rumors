<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from ruling 111 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P2 lane: generators draw their constraints

## Goal

Ruling 111: a property test's generator produces values that satisfy the
test's constraints, and `prop_assume!` survives only where a constraint
cannot be generated, with the reason stated at the site. A `prop_assume!`
that rejects most draws caps the suite at proptest's global-reject limit,
so the suite stops scaling with `PROPTEST_CASES` exactly when CI raises
it (the rumors campaign saw `grow_matches_brute_force` and `grow_minimal`
abort at 1000 cases after 322 and 343 successes). The invariant restored:
every property test scales with its case count, and every surviving
`prop_assume!` names why generation cannot replace it.

## Ground rules

The standard ones: base `main` at the SHA the coordinator names; never
EnterWorktree; touch nothing outside the worktree and the lane's
scratchpad; builds and tests on the illumos box through the wrapper with
`--locked`, `CARGO_BUILD_JOBS=32 NEXTEST_TEST_THREADS=32`, no pset; no
`cases` literal anywhere (ruling 109); every prose change passes
`PROSE.md`; annotation rows per changed region; one commit per site or
per file; report, never decide, on any deviation. Every test keeps its
doc comment's invariant true.

## Members

The seven sites, each with the same acceptance: the test runs at
`PROPTEST_CASES=4000` on the box without a global-reject abort, and its
generator's doc says what it draws.

- `crates/before/src/version/tests.rs:560` and `:588`
  (`grow_matches_brute_force`, `grow_minimal`): `prop_assume!(
  ov.fill_for_test(&op) == ov)`. Draw overlays the fill fixes directly (a
  strategy that yields only fixed points of the fill for the drawn op),
  or draw the op from the overlay; the brute-force oracle stays the
  oracle.
- `crates/before/src/oracle/tests.rs:598`: `prop_assume!(!cands.is_empty())`.
  Draw a non-empty candidate set.
- `crates/before/src/clock/tests.rs:1135`, `:1138`, `:1141`, `:1213`:
  assumptions that a drawn byte body fails to decode as a `Party`,
  `Version`, or `Clock`. Draw the rejecting encoding as a corruption of a
  valid one (a truncation, a flipped tag, a non-canonical padding) so
  every draw is a rejection by construction, or keep the `prop_assume!`
  with the reason stated if random bytes are the property under test.

## Hazards and stops

- A generator that narrows the distribution below what the test's
  invariant needs (a fixed point drawn from one family only) is a
  weaker test; the doc states the family the generator covers, and a
  narrowing the coordinator cannot see from the doc is a finding.
- No public signature moves; no pin moves.

## Widened by ruling 113

No strategy rejects at all: `prop_filter` and `prop_filter_map` go the
same way as `prop_assume!` (the constraint is generated, never
filtered), and no collection strategy carries a nonzero minimum size.
The seven `prop_filter` sites: `fuzzfit/harness/src/strategies.rs`
(non-empty program: draw at least one op), `src/codec/tests.rs` and
`src/borsh_impls/tests.rs` (two each: live bits ending on a byte
boundary, drawn by construction from a byte count), `src/testing/
generators.rs` (`arb_oracle_party_nonempty`: draw a party with at least
one owned leaf), `src/version/skyline/query/tests.rs` (the filtered
range: draw the admitted values directly). Acceptance per site: the
suite runs at `PROPTEST_CASES=4000` with
`PROPTEST_MAX_GLOBAL_REJECTS=0 PROPTEST_MAX_LOCAL_REJECTS=0` exported,
and a grep for the four spellings finds nothing under `crates/before`
and `crates/suanpan`.
