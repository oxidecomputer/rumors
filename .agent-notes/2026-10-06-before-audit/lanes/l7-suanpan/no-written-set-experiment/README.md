<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the audit's coordinator, condensing the experiment agent's report; for Finch. -->

# What suanpan's written-position set buys

The owner asked what removing `suanpan`'s written-position set would cost.
Its only production reader is `last_below`, in the two descents; everything
else maintains it. The experiment removed the set and its bitset (1,434
source lines), made both descents step one position at a time with a jump to
position zero below `lowest_written` (sound because every digit below the
watermark is zero at every completed operation), and measured both boards
and three constructed families at base `1d09cc795` and branch `7302bab63`.
The branch is pinned as `archive/experiment-no-written-set-7302bab63`; its
first commit `6999695fb` holds the temporary fuzzfit probes.

## Verdict

The set stays. Without it, `before`'s rank, distance, lag, and every
`Ranked` operation are quadratic on the committed `weight-comb` family, and
`suanpan` breaks its documented bound for an accumulator operand at a shift
that is not a multiple of 32 bits. Both boards catch it (native 9 red, fuel 16
red across the ladder) and eight cost-meter tests fail; no value test fails.
The saving is small: median fuel −1.0%, heap down to 15% on cells holding many
accumulators (the set took 32 bytes inline in every `Digits`).

## Measurements

`weight-comb`, `version_rank`: fuel exponent 1.00 at base, 1.37 to 1.52 at
the branch; fuel per byte 2,569 at base, 4,490 at the default scale and 10,299
at the top scale at the branch.

`Version::rank` on comb-and-teeth (the `WC(n)` family), fuel per encoded byte:

| K | Base | Branch |
|---|---:|---:|
| 1,024 | 2,243 | 2,493 |
| 4,096 | 2,240 | 3,489 |
| 16,384 | 2,239 | 7,481 |
| 65,536 | 2,239 | 23,456 |

Mixed-sign accumulator operand (`[2^32, −1]` repeated n times, then 3, added
at shift `32n + 1`), fuel per operand byte: base flat at about 96 to 98;
branch 1,730, 6,721, 26,691, 106,563 at n = 256, 1,024, 4,096, 16,384.

Direct gap walk (top cancelling across a never-written gap), marginal fuel
per round: base flat (608, or 1,085 through a sign query); branch linear in
the gap (1,704,364 and 5,571,202 at 65,536 digits).

## What it exposed at base

`before`'s O(`M(n)`) bound for rank, distance, and lag does not follow from
`suanpan`'s documented contract: the branch stays within that contract (each
re-raise of the top is charged its growth `G`) yet rank goes quadratic. The
bound rests on the set's actual behavior, a re-raise across a cancelled gap
costing O(1 + log₆₄ d), which no public text states (question 168).

Leads not built: whether the mixed-sign mechanism reaches `before` through
`Integrator::interval` or `Integrator::jump`; and the fuel board's small-input
cells moving with heap state left by operand setup (seven cells whose measured
code never reaches `suanpan` moved +9% to +34%).

## The mixed-sign lead, answered

`before` reaches the mixed-sign operand through `Version::rank`, by way of
`Integrator::interval`: a right spine of depth `64n + 129` whose last three
leaves leave exactly `[2^32, -1]^n, 3` in the integrator's live accumulator,
added at a shift above the total's top. The probe that builds it is
`mixed-sign-reach-probe.rs` (an in-crate test, never committed to the tree).
Touches per encoded byte at n = 64 to 1024: 0.30 at every n with the written
set; 0.98, 1.65, 2.98, 5.65, 10.98 without it (quadratic); a control with the
same widths and no `2^32` digits stays flat in both trees. `jump`, `finish`,
and shift-zero additions cannot produce the pattern (argued from the code in
the small-items builder's report).
