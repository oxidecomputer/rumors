<!-- CAVEAT LECTOR: a machinery brief written by the coordinator (Claude Opus 5.5) from lane L8's finding D1, after Finch ruled on question 11. -->

# Machinery brief: stop pinning the worst-case family for `count_display × heap`

Kind: machinery, a change to the amplification board. This is the owner's
ruling on question 11.

## The ruling

- Remove the board's worst-case *ranking* pin for the `count_display × heap`
  row only. Keep that row's ceiling and its liveness floor exactly as they
  are.
- Document on the board which readings depend on the target, and why.

## Why

- `Count`'s `Display` delegates to `num-bigint` 0.4.8, which chooses its
  decimal conversion base with
  `FAST_DIV_WIDE = cfg!(any(target_arch = "x86", target_arch = "x86_64"))`
  (`biguint/division.rs:15`). Peak heap during formatting therefore differs
  by architecture.
- On non-x86, the pinned family `hugeleaf` wins by about 0.1%. On x86, the
  top is a cluster of near-equal values.
- So the pinned ranking measures `num-bigint`'s internals, not our code. The
  ceiling is the guarantee.

Full record: `lanes/l8-adequacy/round-1/findings/count-display-heap.md`. The
probe that reproduced the box's readings is on `explore/l8-adequacy`
(`l8/nbprobe/`).

## What to do

1. **Remove the ranking pin.** Find where the worst-case pins live (the
   `worst-cases-pin` recipe and its pin data, and `worst.rs:30-39` for the
   policy). Remove the ranking pin for this row only.
   - Prefer expressing the exemption *positively at the declaration site*: a
     statement that this row's ranking is target-dependent by design. The
     owner's doctrine forbids any mechanism for accepting known failures,
     even an empty one, so this must read as a declared property of the row,
     not an allow-list of failures.
   - If the board's structure only allows a generic "skip" list, stop and
     report the design before building it.
2. **Document target-dependent readings.** In the board's docs, say which
   readings depend on the target, and why: this row, through `num-bigint`'s
   architecture-dependent base. Check whether any other row shares the
   mechanism. The auditor says only `count_display` reaches this path,
   because `Rank` formats in binary. Confirm that.
3. **Re-read the ranking policy.** It lives at `worst.rs:30-39`. Leave it
   true as written for every other row, and restate it if the exemption
   changes its scope.

## What to verify

- On the box, the board leg (`just board`, or whichever recipe the gate
  runs) passes. The two `count_display × heap` drift lines recorded in
  `baseline.md` disappear, and nothing else changes.
- **Calibration.** The ceiling must still catch a regression on this row.
  Show it by a reversible injection, for example an extra allocation
  proportional to the input in `Count`'s `Display`. Every other row's ranking
  pin must still fail when its pinned family is swapped. Show one such swap.
- `just gate` on the box. The board leg now passes, so the result *improves*
  on `baseline.md`. Report exactly which lines changed, so the coordinator
  can update the baseline when this lands.
