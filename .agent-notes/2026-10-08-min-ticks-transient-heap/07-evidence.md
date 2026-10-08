# 7. Evidence

The logs behind sections 3 and 4 live in a session scratch directory,
`/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/`,
which will not survive. The lines that carry the conclusions are quoted
here verbatim; board lines are cut after the columns that matter. Paths
below are relative to that directory unless they start with
`.agent-notes/` or name a commit.

## Commits and branches

All are in the local repository; none were checked out to write this.

| Ref | Commit | Contents |
|---|---|---|
| base of every branch | `5065caebf5d98889741781665211dc7317bbfe88` | `main`'s `min_ticks` code, identical at `e7e107a2` |
| demonstration (also signed earlier as `ba094bbe`) | `08573e1598c057e994c2162a6b71e39a6d5ad23d` | adds `jump-rising-spine`; no `min_ticks` change |
| `archive/min-ticks-reanchor` | `7222d49b2c4f95d26a4016aac2010fc744273d94` | design A |
| `archive/min-ticks-chain-spike` | `966a263ea7e02ffe8b43fbc5cdd1a397b186f6ec` | design B, on top of A |
| `archive/min-ticks-variable-records` | `9088ba943c71643d07cad55647501716eed9c7ff` | design C1, on top of A |
| `archive/min-ticks-records` | `61f55dcb6030b81bda2148db69cc0a5fd5f6c033` | design C2 |
| `archive/min-ticks-records-chunked` | `de4ac8612d9832aa4736bdae2f7ecf582c83da7a` | C2 encoded against the inner neighbour |
| `fix/before-min-ticks-heap` | `1a31f8d19b367c70845ab7d5f7fe18528f9ee326` | design D, on the demonstration commit |

Each archived commit's message carries its full movement record against
the demonstration commit.

## Records in the tree

- `.agent-notes/2026-10-06-before-audit/lanes/l3-events/round-1/defect-D1-min-ticks-heap.md`:
  the defect, the auditor's probes, and every owner ruling in order.
- `.agent-notes/2026-10-06-before-audit/lanes/l3-events/round-1/fix-note-D1.md`:
  the two candidate repairs that became designs A and B.
- `.agent-notes/2026-10-06-before-audit/instrument-survey.md`, sections 1.1
  and 1.7: the `num-bigint` meter and exact board comparison proposals.
- `.agent-notes/2026-10-06-before-audit/STATE.md`: the coordinator's
  record of the stop and its open instrument items.

## Scratch notes

- `fixer-min-ticks-heap/NOTES.md`: the fixer's round-by-round record.
- `fixer-min-ticks-heap/redesign.md`: the invariant and cost derivations
  for designs B, C1, and C2, each written before code, with measured
  results appended.
- `reviewer-min-ticks/NOTES.md` and `reviewer_probe.final.rs`: the round-1
  review of design A, including the stepped spine.
- `reviewer-min-ticks-final/NOTES.md` and `reviewer_probe.rs`: the review
  of design D, including the comb shapes.
- `demonstrator-min-ticks-heap/NOTES.md`: how the `jump-rising-spine`
  family came to be a board family.

## Verbatim lines

**The defect on `main`'s code** (`fixer-min-ticks-heap/parent-board.log`,
lines 59, 5517, and 10975):

```
RED   version_min_ticks        jump-rising-spine     5011->10011    B  heap[e 1.00      124.9/B]  scan[e 1.00        8.0/B]  touch[e 1.00       12.8/B]
RED   version_min_ticks        jump-rising-spine    20011->40011    B  heap[e 1.00      125.0/B]  scan[e 1.00        8.0/B]  touch[e 1.00       12.8/B]
RED   version_min_ticks        jump-rising-spine       61->111      B  heap[e -.--      177.2/B]  scan[e -.--        7.9/B]  touch[e -.--       11.5/B]
```

**The auditor's probes** (quoted from the defect record, which quotes
`run-heap-release-2.log` and `run-heap-release-3.log` on the explore branch
at `1a8601b3`):

```
FINE min_ticks rising-right-spine n=1025 stored_bytes=641 encoded_bytes=641 peak=12656 ratio=19.74 board_ceiling=13844 green
FINE min_ticks rising-right-spine n=1026 stored_bytes=642 encoded_bytes=642 peak=25264 ratio=39.35 board_ceiling=13864 RED
FINE min_ticks rising-right-spine n=65538 stored_bytes=40962 encoded_bytes=40962 peak=1613872 ratio=39.40 board_ceiling=820264 RED
FINE min_ticks rising-right-spine n=262146 stored_bytes=163842 encoded_bytes=163842 peak=6455344 ratio=39.40 board_ceiling=3277864 RED
JUMP min_ticks jump=2^16 n=65536 encoded_bytes=40965 peak=806960 ratio=19.70 board_ceiling=820324 green
JUMP min_ticks jump=2^31 n=1024 encoded_bytes=649 peak=78192 ratio=120.48 board_ceiling=14004 RED
JUMP min_ticks jump=2^31 n=65536 encoded_bytes=40969 peak=5001264 ratio=122.07 board_ceiling=820404 RED
JUMP min_ticks jump=2^31 n=98304 encoded_bytes=61449 peak=10002480 ratio=162.78 board_ceiling=1230004 RED
JUMP min_ticks jump=2^200 n=98304 encoded_bytes=61491 peak=12100312 ratio=196.78 board_ceiling=1230844 RED
```

**Design A on the board** (`fixer-min-ticks-heap/r1-board.log`):

```
GREEN version_min_ticks        jump-rising-spine     5011->10011    B  heap[e 0.99       14.3/B]
GREEN version_min_ticks        jump-rising-spine    20011->40011    B  heap[e 0.99       14.1/B]
GREEN version_min_ticks        jump-rising-spine       61->111      B  heap[e -.--       23.9/B]
```

**Design A past its prefix field** (`reviewer-min-ticks/run1-release.log`,
release; `inline=Some(1)` forces the inline prefix bound to 1):

```
PROBE REAL b=31 wide=8388624 unit=0 inline=None bytes=70254727 peak=1077678048 per_byte=15.34 ceiling=1405095564 green
PROBE REAL b=31 wide=8388624 unit=33554432 inline=None bytes=91226247 peak=6135222240 per_byte=67.25 ceiling=1824525964 RED
PROBE JR b=40 wide=1 unit=8000 inline=None bytes=5012 peak=71584 per_byte=14.28 ceiling=101264 green
PROBE JR b=40 wide=1 unit=8000 inline=Some(1) bytes=5012 peak=629792 per_byte=125.66 ceiling=101264 RED
PROBE JR b=40 wide=1 unit=128000 inline=Some(1) bytes=80012 peak=10012832 per_byte=125.14 ceiling=1601264 RED
```

**Design B on the board** (`fixer-min-ticks-heap/rd-board.log`, the only
two failing samples):

```
RED   version_min_ticks        reveal-comb       751->1501     B  heap[e 0.35        2.3/B]  scan[e 1.00        8.0/B]  touch[e 1.97      219.4/B]
RED   version_min_ticks        reveal-comb      3001->6001     B  heap[e 0.35        1.6/B]  scan[e 1.00        8.0/B]  touch[e 1.97      840.1/B]
```

**Design C1 at real scale** (`fixer-min-ticks-heap/rv2.log`):

```
PROBE REAL b=31 wide=8388624 unit=0 bytes=70254727 peak=1018167296 per_byte=14.49 ceiling=1405095564 green
PROBE REAL b=31 wide=8388624 unit=33554432 bytes=91226247 peak=1048576000 per_byte=11.49 ceiling=1824525964 green
```

**Design C2 at real scale** (`fixer-min-ticks-heap/rr1.log`; `rc1.log`,
for the inner-neighbour variant, prints the same two lines):

```
PROBE REAL b=31 wide=8388624 unit=0 bytes=70254727 peak=305135664 per_byte=4.34 ceiling=1405095564 green
PROBE REAL b=31 wide=8388624 unit=33554432 bytes=91226247 peak=305135664 per_byte=3.34 ceiling=1824525964 green
```

**Design D on the board** (`fixer-min-ticks-heap/rf1-board.log`):

```
GREEN version_min_ticks        jump-rising-spine     5011->10011    B  heap[e 1.00        5.1/B]
GREEN version_min_ticks        jump-rising-spine    20011->40011    B  heap[e 1.00        5.1/B]
GREEN version_min_ticks        jump-rising-spine       61->111      B  heap[e -.--        5.6/B]
```

**Design D's quadratic limb work**, branch then parent
(`reviewer-min-ticks-final/run1-branch.log` and `run2-parent.log`, debug
profile; the branch log interleaves one line with test output, so its
last same-prefix line is reassembled here from its two halves):

```
PROBE cross-prefix b=16000 k=250 bytes=4314 limbs=125751 limbs_per_byte=29.15 peak=20856 peak_per_byte=4.83 answer_bits=16001 answer_low=1f4
PROBE cross-prefix b=32000 k=500 bytes=8626 limbs=501501 limbs_per_byte=58.14 peak=41016 peak_per_byte=4.75 answer_bits=32001 answer_low=3e8
PROBE cross-prefix b=64000 k=1000 bytes=17251 limbs=2003001 limbs_per_byte=116.11 peak=81336 peak_per_byte=4.71 answer_bits=64001 answer_low=7d0
PROBE cross-prefix b=128000 k=2000 bytes=34501 limbs=8006001 limbs_per_byte=232.05 peak=161976 peak_per_byte=4.69 answer_bits=128001 answer_low=fa0
PROBE same-prefix b=128000 k=2000 bytes=98501 limbs=8007997 limbs_per_byte=81.30 peak=210640 peak_per_byte=2.14 answer_bits=128002 answer_low=f9f

PROBE cross-prefix b=16000 k=250 bytes=4314 limbs=251 limbs_per_byte=0.06 peak=20808 peak_per_byte=4.82 answer_bits=16001 answer_low=1f4
PROBE cross-prefix b=32000 k=500 bytes=8626 limbs=501 limbs_per_byte=0.06 peak=40840 peak_per_byte=4.73 answer_bits=32001 answer_low=3e8
PROBE cross-prefix b=64000 k=1000 bytes=17251 limbs=1001 limbs_per_byte=0.06 peak=80904 peak_per_byte=4.69 answer_bits=64001 answer_low=7d0
PROBE cross-prefix b=128000 k=2000 bytes=34501 limbs=2001 limbs_per_byte=0.06 peak=161032 peak_per_byte=4.67 answer_bits=128001 answer_low=fa0
PROBE same-prefix b=128000 k=2000 bytes=98501 limbs=2000 limbs_per_byte=0.02 peak=178072 peak_per_byte=1.81 answer_bits=128002 answer_low=f9f
```

The counting hooks were temporary edits that the reviewer reverted; only
the probe file and the `main`-side `mark` hook survive as files
(`reviewer_probe.rs`, `hook-main-min_ticks.diff`). The exact placement of
the `count_limbs` calls on each side is therefore known from the
reviewer's notes, not from a saved diff.

## What this document did not verify

- No number here was re-measured; each is quoted from a log or a commit
  message, as labelled.
- `main` has never been run on the 91 MB input (section 3).
- Design D was never run on the real-scale input (section 4).
- No `num-bigint` time measurement exists for `main` or for designs A, B,
  or C1.
- The text of the per-push repair that question 87 refers to is not in
  the records I read; section 4's reading of the owner's suspicion is my
  inference.
