<!-- CAVEAT LECTOR: a builder brief written by the coordinator (Claude Opus 5.5) from the rank-decode light review (session scratchpad `reviewer-rank-decode-light/ceiling-rules.txt`). It stacks on `simplify/before-rank-decode-image` (ready #53). -->

# Builder brief: every board ceiling follows the rule its doc states

Kind: test-instrument correction. It changes no production code.

## The finding

`crates/before/src/testing/meter/board/ceilings.rs` documents each per-byte
heap ceiling by a derivation rule. #53 restated two of them,
`RANK_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE` and
`RANKED_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE`, as "the largest
release-profile reading among samples of at least twice
`HEAP_INTERCEPT_BYTES`, with 25% headroom, rounded up". Both constants
reproduce exactly under that rule.

Two siblings carry the old wording, "largest reading with 25% headroom", and
no rule reproduces either value:

- `DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE` = 4.0. Over all samples the rule
  gives 20, and under the floor rule it gives 5. The reading at the floor,
  3.6, leaves about 11% headroom.
- `QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE` = 152.0. Over all samples
  the rule gives 177, and under the floor rule it gives 89. The constant is
  stale.

## The work

1. **Audit every ceiling** in `ceilings.rs`, not only these two, against its
   stated rule over the current board's readings. Bracket each the way the
   reviewer did: rows where both samples clear the floor, and rows where only
   the larger does. Report a table with columns: constant, stated rule,
   value, value the rule gives, and the largest reading.
2. **Lower** every constant the rule says can fall, and restate its doc in
   #53's wording. Lowering needs no ruling.
3. **Raising** needs the owner. Build `DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE`
   two ways, as separate commits on top of the lowering commit, so the owner
   can choose:
   - (a) raise it to the value the floor rule gives, with #53's wording;
   - (b) keep 4.0 and state, in its doc, the rule it actually follows. For
     example, a smaller headroom, stated as a rule, not a reading.

   Do the same for any other constant that would rise.
4. **Evaluate, without building**, whether a committed test could check
   every ceiling against its rule from the board's own readings, so that the
   rule can't drift from the value again. Name what it would catch and what
   it would cost. For example, it might force a re-derivation whenever
   readings fall.

## Constraints

- No performance values in tree prose; readings go in commit messages.
- One landing check, at the tip of option (a).
- Report each moved ceiling as old value and new value, with the reading
  that drives it.

## Owner's ruling (question 65)

Option 1: one derivation rule for every board ceiling, stated identically in
each doc, and the three ceilings the rule says must rise are raised (QUERY to
109). #53's two rank ceiling docs are restated in the same terms by this
branch, which stacks on #53. Values move only as the reviewed table shows.
