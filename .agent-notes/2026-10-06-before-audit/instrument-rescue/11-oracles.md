<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the instrument rescue's integrator, generated from one ranked list of every instrument in sections 01 to 09; for Finch's review. -->

# 11. Independent models and oracles

An oracle adds coverage through independence: what counts is the code and
structure it does not share with production.
[`00-baseline.md`](00-baseline.md) section 3 lists the committed oracles and
the production code each one shares: the recursive tree oracle shares
production's recursion, the bridge shares its writer and readers, and the
function-space oracle judges only shallow inputs. Several lanes built
independent models of the same objects; section 1 of
[`17-overlaps.md`](17-overlaps.md) reconciles them.

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

This category holds 11 of the 159 rows.

| Rank | Overall | Instrument | Adds beyond `main` and the ready branches | Evidence | Fold-in work and runtime | Depends on |
|---:|---:|---|---|---|---|---|
| 1 | 3 | [04.5](04-measures.md) Rational `Rank` oracle `Q` with `render`, `arb_q`, `arb_related_q` | Integral ranks 33%, even integers 24.5%, zero 6.8%, equal-value pairs 30%, against 0 for the committed `RANK_TRIPLE` driver; text rendered without production code. | Its property caught R2 once the same-class mode existed (the committed suite also does). | About 60 lines of oracle into `testing::oracles`, generators into `testing::generators`, and a second `RANK_TRIPLE` driver. Under 1 s. | None (it would also drive #83's new law). |
| 2 | 19 | [02.1](02-algebra.md) Leaf-list model of versions and parties | An oracle independent of both the tree representation and production's `VersionWriter`, practical at depth 300; the committed lattice differential builds its expected values through that writer. | Its own assertions killed 9 of 20 round-1 mutants (the committed suite killed all 20). | A new `testing::oracles` module (newtypes, docs) and a public-API differential beside the table. Merge with 04.3. | None. |
| 3 | 20 | [04.3](04-measures.md) Flat sweep oracle for rank, distance, lag, and `rank_cmp` | Measures computed without the recursion or a sampling grid, linear in leaves; it judged inputs 3,000 levels deep. It uses 02.1's representation. | Kills the 12 integrator mutants (as does the committed suite). | Merge into 02.1's module as its measures. 3.2 s per 256 cases. | None. |
| 4 | 21 | [06.1](06-codecs.md) Specification-codec decode differential | Every decoder's accept set and error classes, through every entry point (slice, chunked reader, text, postcard, CBOR, borsh, JSON spans), against a specification written from the docs, on deep and non-canonical inputs (version depth above 32 in 11.8%, collapsible pairs 15.5%, truncation 32.2%). | 17 of 17 mutants (committed also; M6 and M13 to M15 rest on one or two committed tests); 300,000 cases per type twice. | Port about 1,800 lines as a test-only `oracles::codec`; make eight helpers iterative; replace its runner with `proptest!`; decide its narrowed verdicts first. 10.3 s for all eleven tests. A cheaper partial fold-in, entry-point agreement without the model, is about 150 lines. | Your precedence decision (section 1 of the main file). |
| 5 | 22 | [07.1](07-suanpan.md) Pool model's exact-oracle checks | A `num-bigint` oracle over operands with their own histories (33% of operand uses hold two or more zero ranges, against 0), values to 5,112 digits (against 116), extreme digits at `normalize` and shifts six to eight times as often; on #37's generator, 5.6% of steps leave uncompacted cancellation. Four predicates no committed test states. | M1 to M5, M7, M9, N1 (N1 now caught by #66); no unique catch at `main` plus ready. | About 360 lines of checks as a second property over #37's `arb_program`, adjusted for #28 and #86. 3.7 s; about 5 to 15 s on #37's generator (inferred). | #37, #28, #86. |
| 6 | 25 | [05.2](05-spans.md) Exact coverage oracle (`census`, `sublattice`) | Exact coverage on any finite vector world, where committed exact checks need a complete grid; its exactness argument re-derived by the cataloguer. | Behind every coverage kill (committed also); never hit its cap in 857,820 checks. | Moderate, ideally on top of the function-space oracle; count capped skips and fail on any. | None. |
| 7 | 26 | [05.7](05-spans.md) Vector model and verdict oracles (spans and queries) | The only span and query oracle sharing no machinery with production `partial_cmp`, which shares the overlay advance, `OrderState`, readers, and accumulator with the walks it judges. | Behind every verdict kill (committed also). | Small to moderate: pointwise helpers beside `oracles::function` and the existing `ev_vector`. | None. |
| 8 | 27 | [01.2](01-identity.md) Interval-set model of `Party` | Exact canonical results for `fork`, `forks`, array splits, `sync`, and `sync_all` independent of tree recursion, at depths 20 to 126. | Properties on it caught 21 of 24 mutants; M19, M20 only through its multiplicity check (now #40, #81). | A third party oracle in `testing::oracles`; about 15 doc comments; keep one multiplicity helper. 0.15 to 0.96 s per property. | None (#40 for the helper). |
| 9 | 68 | [02.6](02-algebra.md) Independent canonical encoders (`encode_version`, `encode_party`) | Ties every algebra result to the written wire specification, byte for byte. | Writer mutants were killed first by production assertions. | Tiny with 02.1. | 02.1. |
| 10 | 129 | [09.A33](09-build-and-review-probes.md) Differentials against replaced implementations | Old against new for the one-sweep lattice, `refine_partial`, and `sync_all`. | No divergence; sweep mutants fail. | Would keep deleted implementations as oracles; the sweep took 386 s. | #30, #61, #76. |
| 11 | 159 | [05.21](05-spans.md) Witness heuristic (`witness_coverage`) | Nothing; dead code. | None. | Delete it. | None. |
