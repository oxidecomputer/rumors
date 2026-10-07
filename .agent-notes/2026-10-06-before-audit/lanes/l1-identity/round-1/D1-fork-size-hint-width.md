# Defect D1 (low): the fork iterators' size hint depends on `usize` width

## Contract clause breached

`common.md`, "The contract you audit against", `usize` invariance: "Both
crates should behave identically whatever the width of `usize`." The rustdoc
on `PartyForks` (`crates/before/src/party/forks.rs:348-349`) and `ClockForks`
(`src/clock/forks.rs:15-16`) currently *permits* the behavior below ("Wider
counts report a sound lower bound and no upper bound"), so the two contract
sources disagree; resolving it needs the owner's ruling (see the question in
the report).

## What happens

`Plan` classifies the *initial* count against `usize::MAX` once, at
construction (`Remaining::new`, `src/party/forks.rs:122-132`):

- at most `usize::MAX`: `Exact`, decremented each step;
- up to twice `usize::MAX`: `Near`, which becomes `Exact(usize::MAX)` once
  that many shares remain;
- beyond that: `Distant`, which never becomes exact. Its hint is
  `(usize::MAX, None)` while at least `usize::MAX` *base paths* remain
  (`has_saturated_lower_bound`, `:203-214`), and `(0, None)` after that, down
  to the last share.

So a fixed count, after a fixed number of steps, yields different hints on
different targets. Take `k = 2^34`: on a 64-bit target the plan is `Exact` and
reports `(5, Some(5))` with five shares left; on wasm32 it is `Distant` and
reports `(0, None)` at the same point. The shares yielded and the iteration's
end are identical on both; only the hint differs. Every hint is sound, so no
`Iterator` contract is broken.

The existing test `distant_size_hint_stays_sound_near_exhaustion`
(`src/party/forks/tests.rs:167-182`) pins the 64-bit analog of this
behavior as intended.

## Reproduction (verified)

Explore commit `1a5f5f8b` adds
`party::forks::tests::explore_distant_plan_is_exact_once_the_remainder_fits`:

```
on-illumos.sh /Users/oxide/src/rumors-audit-l1-identity 'unset CARGO_TARGET_DIR; nice -n 10 cargo nextest run -p before --all-features --locked --build-jobs 24 --lib -E "test(explore_distant_plan)"'
```

Exit status 100. Verbatim:

```
assertion `left == right` failed
  left: (0, None)
 right: (5, Some(5))
```

## Failure family

It appears for every initial count above `2 * usize::MAX` (on 64-bit, above
`2^65 - 2`; on wasm32, above `2^33 - 2`) once fewer than `usize::MAX` base
paths remain. It disappears for counts at most `2 * usize::MAX`, whose hint is
already "exact whenever the remainder fits a word". Reaching the divergent
region by iteration takes about `2^33` steps on wasm32 and `2^65` on 64-bit,
which is why the demonstration sets the plan's index directly, as the existing
test does.

## Root cause

The remaining count is not tracked exactly for wide counts; the state machine
approximates it relative to the machine word, by a deliberate choice
("Remaining shares without an arbitrary-width duplicate of the fork count",
`src/party/forks.rs:110`).

## Fix note

Track the remainder exactly as a `BigUint` in `Plan`: set it from the count at
construction, decrement it in `advance`, and test `== 0` for exhaustion.
Derive the hint by one rule on every target:
`(min(remaining, usize::MAX), Some(remaining) if it fits usize)`.

- Costs: one more `O(log k)`-bit integer in the plan (it already holds two:
  `index` and `extra`); the decrement is amortized `O(1)` and `O(log k)` at
  worst, inside the documented `O(|p| + log k)` per step.
- Deletes `Remaining`, `has_saturated_lower_bound`, and the `Distant` end test
  (`index.bit(depth)`), about 60 lines.
- Public API and formats are unchanged. The documented hint guarantee changes
  (strengthens) to "exact whenever the remaining count fits `usize`", and
  `distant_size_hint_stays_sound_near_exhaustion` must change its expectation
  at the near-exhaustion point from `(0, None)` to the exact remainder. That
  is a change to pinned semantics, so it needs the owner's ruling first.
- The board's `party_forks` and `clock_forks` heap rows gain one count-sized
  allocation; the builder must check them against `baseline.md`.

## Test brief

- **Invariant:** a fork plan reports an exact size hint whenever its remaining
  count fits `usize`, whatever the initial count.
- **Form:** a unit test over the finite set of boundary cases, in
  `src/party/forks/tests.rs` (it needs `Plan`'s private `index`, as the
  existing test does).
- **Construction:** the plan must be positioned near exhaustion without
  iterating. Add one test-only helper beside the test,
  `#[cfg(test)] fn position_at_remaining(&mut self, remaining: &BigUint)`,
  so the assertion reads only `size_hint` and survives the fix. On the base
  commit it can only be implemented for *distant* plans, whose hint is
  derived from `index` (`plan.index = count - remaining`, valid when the
  count is a power of two, since then no base path forks again); `Exact` and
  `Near` plans keep their own counter, which `index` does not move. The fix
  reimplements the helper by setting both `index` and the exact remainder.
  Cases: `count = 2^d` for `d` in `{usize::BITS + 1, usize::BITS + 2, 128, 130}`
  (all distant on every target) and remainders `r` in
  `{0, 1, 5, usize::MAX - 1, usize::MAX}`.
- **Assertion:** `plan.size_hint() == (r, Some(r))` with `r` as `usize`; and
  for `r = usize::MAX + 1`, `(usize::MAX, None)`.
- **Base-commit failure:**
  `left: (0, None)` against `right: (5, Some(5))` for `d = usize::BITS + 2`,
  `r = 5` (as above).

## Owner's ruling

Yes. The fork iterators report an exact `size_hint` whenever the remaining
count fits `usize`, by one rule on every target. This supersedes their
rustdoc's allowance for wide counts, and the test that pins it.

The fix stores the remainder as a `BigUint` in the fork plan, which adds one
count-sized allocation per fork iterator. The board's fork count is as wide
as the party, so that cost grows with the input: every `party_forks` and
`clock_forks` family's heap reading rises about 0.5 B/byte. `copy-hole`'s
tiny default-scale input rises more (9.4 to 10.8 B/byte), presumably from
per-allocation overhead, which flips the default-scale `party_forks × heap`
worst case from `ascend-cliff,ascend-plateau` (10.1 to 10.6 B/byte) to
`copy-hole`. Every committed ceiling still holds. The owner accepted the
allocation and the re-pin, rather than deriving the remainder without
storing it.
