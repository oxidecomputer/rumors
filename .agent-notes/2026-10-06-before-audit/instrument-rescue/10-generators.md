<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the instrument rescue's integrator, generated from one ranked list of every instrument in sections 01 to 09; for Finch's review. -->

# 10. Generators and reach

A generator adds coverage through the inputs it reaches that the committed
generators do not. [`00-baseline.md`](00-baseline.md) section 2 gives the committed
reach, and its section 2.7 lists the regimes the committed generators never
or almost never produce; most rows here are claims about that list. Some
generators fold in only with a model (02.4 calls 02.1's operations, 05.1
needs 05.2 and 05.7); the "Depends on" column says so.

Each row condenses the linked entry in sections 01 to 09, which gives the
full account and marks every figure as verified, reported, or inferred; a
row's figures carry the linked entry's marks. "Rank" orders this category;
"Overall" is the instrument's position in the single ranking in
[`../instrument-rescue.md`](../instrument-rescue.md), section 3, and both
rank by the coverage an instrument adds against the work of folding it in.
`#n` is entry `n` of `QUESTIONS.md`; "slot 23", "slot 26", "slot 33",
"slot 45", "slot 46", and "slot 47" are ruled branches not yet ready. Where two lanes
built overlapping instruments, [`17-overlaps.md`](17-overlaps.md) says which
belong together or which subsumes which.

This category holds 17 of the 159 rows.

| Rank | Overall | Instrument | Adds beyond `main` and the ready branches | Evidence | Fold-in work and runtime | Depends on |
|---:|---:|---|---|---|---|---|
| 1 | 5 | [04.1](04-measures.md) Multi-scale partition generator with related-pair modes (`arb_leaves`, `arb_related`) | Sends arbitrary versions through the integrator's deferral path (50.8% of pairs, against 0 committed) and exact rank ties at freezing scale (29.1%, against 0); depth to 130, heights to 2,209 bits (a deep setting reaches about 3,000 levels). | Its harness needed the deep prefix to catch M25; no defect. | Strategies into `testing::generators` as a third input for the law drivers and the measure tests, with no new oracle; 3.2 s per 256 cases. The deep setting needs an explicit-stack thread (ruling 68). | None (the deep setting waits on the stack-guard removal). |
| 2 | 6 | [01.1](01-identity.md) Party generators `arb_set`, `arb_disjoint` | Parties to depth 80 (above 32 in 47%), 17 or more two-child branches in 42%, and accepted `join_all` families of up to 10 arbitrary parties (the committed families accept none above one item). | No unique catch. | About 90 lines of builder into `testing::generators`, returning `tree::Party`; the function-space leg must cap depth with `arb_set_upto`. 2.8 s per 4,000 draws. | None. |
| 3 | 14 | [02.3](02-algebra.md) Deep random partitions (`gen::partition`, `limits`) | Random topology to depth 300 and 1,034 leaves (median depth 12), where committed random versions stop at depth 4 and 11 leaves. | No unique catch. | About 45 lines as a `prop_flat_map` strategy, with a size budget per component (the tape starves later draws) and a census floor. | None. |
| 4 | 24 | [05.1](05-spans.md) Pairwise-concurrent holes (`with_bumps` and the antichain properties) | Three or more concurrent holes under an exact oracle, at depth and width: 20,053 refinement-decided checks per 256 shaped cases (15,255 with two or more holes); the committed exact checks reach three holes 8 times per 256 cases, on two cells only. | Uniform form killed 10 mutants (committed also); returns without checking in 2% to 8% of cases. | Moderate: needs 05.2, 05.7, and the world generators; construct one hole directly. Shaped form 26 s. | None. |
| 5 | 33 | [09.A13](09-build-and-review-probes.md) Swarm close bound for the range-minima property | Raises #82's detection of its target mutant from 16 of 20 seeds to 20 of 20. | Measured over 20 seeds each. | One strategy change in #82's property. | #82. |
| 6 | 37 | [02.7](02-algebra.md) Party region generator (`gen::party`) | Deep fragmented masks (depth to 300, up to 834 regions) for the masked walks. | No unique catch. | Small, with 02.3; needs the late-draw starvation fixed. | None. |
| 7 | 38 | [02.5](02-algebra.md) Boundary-height palette (`gen::level`, `gen::levels`) | Heights on the writer's 63-bit code boundary inside random deep topology and collapse cascades (63-bit codes in 22.9% of first operands). | No unique catch. | A few lines beside `arb_magnitude`; useful only with a topology generator. | None. |
| 8 | 39 | [02.4](02-algebra.md) Correlated pair families (`gen::pair`) | Equal pairs 27.2%, comparable 38.7%, and forced writer collapse cascades at random depth; committed independent pairs are never equal. | No unique catch. | Folds in with 02.1, whose operations it calls. | 02.1. |
| 9 | 43 | [05.9](05-spans.md) Grid worlds and near-equal wide heights | `Before` placements 9.6% and one-sided concurrency about 12.5%, against 1.5% to 2.4% committed; near-equal wide heights in adjacent cells. | No unique catch. | Small: value alphabets beside `arb_magnitude`, pool closure as in `verdict_matrix`. | None. |
| 10 | 44 | [05.8](05-spans.md) Deep and random span shapes (`Layout`, `arb_layout`) | Shapes to depth 40 (338 of 771 versions at 17 to 40) for span and query checks under an exact oracle. | No mutant evidence. | Moderate: a strategy in `testing::generators` with an explicit layout argument, and a census floor. | None. |
| 11 | 46 | [05.11](05-spans.md) Clause model and conjunction folds | Operand order on both sides of every typed `&` cell, and the `!before` spelling, inside long conjunctions. | Part of 05.1 and 05.5's kills. | Small, as a delta to the committed clause enums. | None. |
| 12 | 49 | [01.3](01-identity.md) Rule-respecting history driver | `forks`, array splits, `join_all`, `sync_all`, and byte round trips inside histories, with 51% of steps holding 8 or more clocks (committed traces exceed 8 in about 0.3% to 0.6%). | No unique catch. | Extend `optrace::Op` with five kinds for all three models and re-derive the function-space grid (moderate to high), or keep it production-only. 0.55 s. | None. |
| 13 | 50 | [06.5](06-codecs.md) Near-valid encoding generators | The only random source of structurally whole but non-canonical encodings and deep spines of either kind. | Through 06.1, all 17 mutants. | Into `testing::generators` with model trees; usable alone for a totality property and entry-point agreement. | None. |
| 14 | 76 | [05.15](05-spans.md) Organic overlays | Exact coverage on organic shapes (depth at most 5). | No mutant evidence. | Small if 05.2 and 05.7 are folded. | 05.2, 05.7. |
| 15 | 109 | [01.13](01-identity.md) Deep-party builders | The alternating spine and comb, and shallow-plus-deep parties. | Inputs to 01.7 to 01.9. | Small; worth taking only with those entries. | #34. |
| 16 | 131 | [03.10](03-events.md) Bushy strategy as a property | Balanced non-spine shapes 7 to 10 levels deep. | Caught nothing #74 missed. | Low. | #74. |
| 17 | 132 | [03.11](03-events.md) Second late perturbation | A second divergence site at an off-palette height. | None attributable. | Adds a third to each case's runtime. | #74. |
