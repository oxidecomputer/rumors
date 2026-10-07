# Tie order in `advance_set` (the `>` to `>=` survivor at `overlay.rs:142`)

Verdict for the lane's two `CursorSet` users (`version/place.rs:410`,
`version/place/filter.rs:418` and `:716`): the order in which tied cursors
step cannot change any `Placement`, `Dominance`, `Precedence`, membership, or
`Coverage` result. The mutant is equivalent for these users. The argument
follows; the empirical check is recorded at the end.

## Argument

1. **The same set of slots steps, whatever the order.** `advance_set` steps
   the chosen deepest slot, then every other slot whose depth is at least the
   returned flip level. Two slots at the maximal depth whose current leaves
   contain the same position share their whole path, so they return the same
   flip level. (`advance`'s equal-depth arm asserts exactly this.) The set of
   slots that step, `{slot : depth(slot) >= flip}`, is therefore the same
   whether the first or the last maximal slot is picked. Done and dropped
   slots are strictly shallower than any live, unfinished slot (the
   `CursorSet` doc's invariant), so neither choice can pick one, and
   `step_bound`'s `expect` stays unreachable.
2. **Reads happen only between advances.** Every verdict read (`side.read()`
   in `place::walk`, `Comparison::read` in `admits` and `coverage`), every
   hook, every settle (`live` flags), and every `lo_live`/`hi_live` update
   happens in the loop body. None happens inside `CursorSet::step`. So only
   the state after the whole tied group has stepped is ever observed.
3. **The state after the group is order-independent.**
   - The placement walk (`place.rs:428-441`) folds each crossing into
     `BoundSide::diff` by accumulator addition, which is commutative. The
     cursors are independent.
   - The filter walks (`filter.rs:440-462`, `747-791`) update each absolute
     height by addition (commutative). Each private difference
     (`Comparison::fold`, `filter.rs:184-192`) is either updated by the same
     commutative addition or discarded, depending on a width test of the
     intermediate heights. That test can depend on order, but its two
     outcomes are "exact difference with every delta applied" or "no
     difference". A discarded difference never comes back mid-group, because
     `fold` applies nothing to `None`. The next read either takes the sign of
     an exact difference or recomputes it from the final absolute heights
     (`compare_heights`), so the sign it reads is the true sign of
     `probe - bound` either way.

So tie order can change only accumulator representation and work: which
difference is discarded and rebuilt, and digit-touch counts. It cannot change
results.

## What this means for the docs

- `CursorSet::priority`'s doc already says "The folds are commutative, so the
  order does not change the result, but fixing it keeps accumulator work
  reproducible". That is accurate. The `>=` variant still picks
  deterministically (the last maximal slot), so reproducibility would also
  survive it.
- Two user docs give a stronger reason than the code needs:
  - `place.rs:407-409`: "The probe goes first when boundaries tie because
    each difference treats it as the left operand."
  - `filter.rs:414-417` (and `711-715` for coverage, "probe endpoints step first"): "the probe steps first
    on every tie because it is every comparison's first operand and the
    overlay law's equal-depth arm steps that operand first."

  Both read as correctness requirements. Item 3 shows they are not:
  differences take the probe as the left operand through `Side::A` whatever
  the stepping order. The accurate statement is that the order is fixed for
  reproducible accumulator work only.
- The priority iterator itself cannot be removed outright, because
  `advance_set` must still enumerate the slots to find the maximum and the
  ties. Its *ordering* semantics ("first deepest slot", "orders the tied
  advances") could be relaxed to "enumerates the slots" if the owner does not
  want reproducible accumulator work. Recommendation: keep the fixed order
  (deterministic touch counts are what the board's touch currency relies on)
  and correct the two user docs to the reproducibility reason. That is a
  self-contained prose change; I have not written it as a separate brief.

## Scope

`projection.rs:420` is the fourth user, and it is outside this lane. The same
three-step argument applies if its step bodies only fold commutatively and
its reads happen between advances. I did not verify that.

## Empirical check

I applied the exact swap (`depth > max` to `depth >= max` at
`overlay.rs:142`) on `explore/l5-spans` @ `2874e0c3` and ran the following on
ox-east-1 (box logs `target/l5tie*.log`):

- the L5 harness, `PROPTEST_CASES=300 cargo test ... --lib -- l5_`:
  `test result: ok. 17 passed; 0 failed; 7 ignored`
- the exhaustive cube: `checked=921456`, the same verdict tally as unmutated
- `l5_touch_carry_family` and `l5_refine_partial_scan_cost`: touch and scan
  readings byte-identical to the unmutated run, at all four sizes
- the committed lane suite (`causally::`, `span::`, `version::place`,
  `algebraic_laws`, `laws::`, `projection`, `own`, `verdict_matrix`,
  `coincident_span`): `94 tests run: 94 passed`

Then I reverted the swap; `git diff` is empty.
