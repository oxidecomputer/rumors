<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the instrument rescue's integrator, from sections 00 to 09 and spot checks against the branches; for Finch's review. -->

# 17. Where lanes built the same thing

Each lane catalogued its own instruments, so overlaps between lanes appear
only here. For each group of overlapping instruments, this section says
which belong together, which subsumes which, and what a single fold-in
would keep. Instrument IDs (`02.1`, `09.A3`) name a section and an entry in
it; the tables in sections 10 to 16 rank them.

Marks: *verified* means I checked it against the branch or `main`;
*reported* means a section says so; *inferred* is my reasoning, not checked
by a run. Claims a section already marks are not re-marked here unless I
checked them again.

## 1. Independent models of versions

Four instruments model a version as a step function over `[0, 1)`, as the
committed function-space oracle does, but in forms that scale with depth:

| ID | Representation | Operations | Reach |
|---|---|---|---|
| 02.1 | an ordered list of `(depth, height)` leaves, absolute heights, positions as `BigUint` numerators | join, meet, order, projection, shapes, canonical encoders (02.6) | depth 300 |
| 04.3 | the same list of `(depth, height)` leaves with absolute heights | rank, distance, lag, the sign of `rank_cmp` | depth 3,000 |
| 05.7 | a vector of heights over one leaf partition shared by every version in a test | order, join, meet, projection, placement and its coarsenings | depth 40, up to 64 cells |
| 05.2 | sets of such vectors, closed under join and meet | exact coverage of queries | any finite vector world |

- **02.1 and 04.3 are one model.** Both lanes chose the same leaf list with
  absolute heights (verified by the measures cataloguer against
  `explore/l2-algebra:crates/before/tests/l2_probe/model.rs`, and consistent
  with 02.1's description). 04.3 adds the measures 02.1 lacks; 02.1 adds the
  lattice, order, projection, and encoders 04.3 lacks. A fold-in should
  build one oracle module from both, with 04.3's functions as its measures.
  Neither subsumes the other.
- **05.7 is the same model on a common refinement.** A vector over a shared
  partition is a leaf list whose leaves are the partition's cells (inferred,
  from both representations). 05.7 adds what no other model has: spans,
  queries, and the nine-way placement. Its cataloguer proposes building it
  on `oracles::function` and the committed `ev_vector`; building it on the
  merged leaf list instead would let the span and query oracles reach the
  leaf list's depth (inferred). Either way it should not become a third
  parallel version type.
- **05.2 sits on top of 05.7.** It is an algorithm over vectors, not a
  representation, and it is the only exact-coverage oracle for arbitrary
  worlds.
- **Order of fold-in.** 02.1 with 04.3 first, since 02.2, 02.4, 04.10, and
  04.13 depend on them; 05.7 and 05.2 next, since 05.1, 05.5, 05.10, 05.12,
  and 05.14 depend on them.
- **The committed function-space oracle stays.** It is the only committed
  oracle independent of the tree recursion, and it checks policy
  independence through random `fork` and `event` choices, which none of
  these models do. These models add depth, not a replacement.

## 2. Independent models of parties

| ID | Representation | Operations | Reach |
|---|---|---|---|
| 01.2 | sorted disjoint dyadic intervals, `u128` numerators over `2^126` | set algebra, `fork`, balanced `shares(n)`, `sync`, `sync_all`, pointwise multiplicity, its own codec | depth 126 |
| 02.1's party half | an ordered list of `(depth, owned)` regions, `BigUint` positions | masks for projection and shapes | depth 300 |
| 06.1's party half | a specification decoder of the party wire format | acceptance and error classes | any encoding |

- **01.2 is the party oracle to keep.** It alone computes exact results for
  `fork`, `forks`, the array splits, `sync`, and `sync_all`. Its 126-level
  ceiling exceeds every generator built for it (01.1 stops at 80).
- **02.1's party half is a mask, not an identity algebra.** It needs no
  separate fold-in: once 01.2 exists, 02.1's masks can come from it, or stay
  as they are inside the merged version model (inferred).
- **06.1 judges encodings, not values.** It overlaps 01.2 only at the
  codec, where 01.2's own decoder asserts canonicity of production bytes.
  06.1 is the broader check there.
- **Multiplicity.** 01.2's `same_multiset` duplicates #40's
  `tree::Party::same_multiplicity` and #81's laws-side comparison; keep
  #40's and #81's (reported, 01).

## 3. Encoders and codecs

- **06.1 subsumes 02.6.** 02.6 encodes versions and parties only, in one
  direction. 06.1 encodes and decodes all six wire types with error classes.
  02.6 is already wired to the algebra's outputs, which is its only
  advantage; once 06.1 is folded in, 02.1's results can be encoded by 06.1's
  encoder instead (inferred). The algebra cataloguer reached the same
  conclusion ("one of the two suffices").
- **06.7 and 02.2's byte checks overlap.** Both compare production's bytes
  with an independent encoder on deep joins; 02.1's topology reaches bushier
  deep trees than 06.7's spines. Keep the byte check in whichever property
  folds in first.
- **04.5 and 06.1's rank half answer different questions.** 04.5 judges
  `Rank` values and the order they induce; 06.1 judges whether rank bytes
  are valid. Neither subsumes the other.

## 4. Deep and correlated generators

Seven generators reach past the committed depth of 4, each aimed at a
different mechanism:

| ID | Reaches | Aimed at |
|---|---|---|
| 01.1 | parties to depth 80; accepted families of up to 10 | identity operations, `join_all` success |
| 02.3, 02.7 | versions and parties to depth 300 | the algebra's walks |
| 04.1 | versions to depth 130 (about 3,000 in its deep setting), heights to 2,209 bits | the integrator's deferral and freeze paths |
| 05.8 | span shapes to depth 40 | span and query cursors |
| 06.5 | non-canonical encodings, spines of 30 to 399 levels | decoders |
| 03.4 with #74 | tick spines to 2,999 levels | the pre-scan and memo |
| 01.3 | populations of many clocks | multi-party operations |

- **Topology and heights separate cleanly.** 02.3's partition generator is
  the general topology source (about 45 lines). The height palettes aim at
  distinct boundaries and do not overlap: 02.5 at the writer's 63-bit code,
  04.1's step script at the integrator's freezing scale, 05.9's alphabets at
  near-equal wide values. A fold-in could offer one topology strategy with
  several height strategies (inferred).
- **01.1 and 02.7 overlap for parties.** Both produce deep masks. 01.1 also
  produces accepted `join_all` families, which 02.7 cannot, so 01.1 is the
  one to keep; 02.7 adds nothing 01.1 lacks except depth past 80 (inferred,
  from their reach figures).
- **Correlated pairs come from three places.** 02.4 (perturbations and
  decompositions forcing writer cascades), 04.1's related modes (mirrors
  and rotations giving exact rank ties), and 05.9's pool closure (rare
  placements). Each aims at a different consumer. The committed families
  already supply equal and ordered pairs at depth 4 at most and with at
  most four distinct values (`00-baseline.md` section 2.3).
- **01.3 and 03.2 extend the trace vocabulary in two directions.** 01.3 adds
  `forks`, splits, `join_all`, `sync_all`, and byte moves, checked against
  01.2; 03.2's history world adds wide `ticks`, absorbs, and foreign ticks,
  checked against a per-clock ledger. The events cataloguer suggests merging
  them into one production-only world. That is sound if the foreign tick is
  in your model (decision 7 in the main file).

## 5. Fork counts and wide counts

01.6 (share identity past `usize`, counts to `2^100`), 09.A26 (exact hints
and the last share at every depth to 140), #43 (hints through real steps at
four depths), and 01.8's fork-step family (count width apart from party
size, for cost) cover different parts of one plan. 01.6 and 09.A26 both
need #43's stepping; 09.A26 must first move to `skip_shares` (reported,
09). None subsumes another.

## 6. Stack safety

#34 and #75 carry the committed form: every walk-bearing entry point at
depth `2^18` on left and right spines. Around them:

- 01.7 adds the one shape neither branch builds, an alternating zigzag
  spine (verified by the identity cataloguer with `git grep`); a few lines
  in #75.
- 03.4 adds random tick content between depth 128 and 2,999.
- 09.A17 calibrates #34 and #75 (PAMT is why the depth is `2^18`); 02.9 is a
  weaker form of the same calibration and is subsumed by it.
- 01.14 (unoptimized builds) was declined under question 68.

## 7. `min_ticks`

- **Semantics.** 03.1 (attainment by a constructed history) and 03.2
  (per-clock causal floor) are distinct predicates. 03.2 strengthens the
  committed `min_ticks_floors_every_history`, which states the weaker
  whole-history floor (verified, `version/tests.rs:682`).
- **Heap and limb work, all waiting on question 87.** 03.5 and 09.A8 are
  one instrument: the lane's probe and its port to the board in
  `08573e159`. 03.7 is the same spine without the jump. 09.A4 measures the
  same regime at sizes off the board's ladder, with a closed-form value
  oracle; 09.A15 measures it at 91 MB; 09.A9 counts limb work on combs. The
  write-up in `.agent-notes/2026-10-08-min-ticks-transient-heap/` names
  three of these as instruments that must exist before a redesign.
- **Counters.** 03.9 checks touch and scan ceilings on random pairs and
  finds `min_ticks` reading exactly 8.00 scan bits per byte.

## 8. Wasm fuel

07.2 (six operations added to #52's ladder), 09.A9 (comb families in the
same ladder, survey 1.1), 09.A19 (compile-time mutant switches, the record
behind survey 1.9), and 09.A5 (comparing fuel between commits) all extend
the fuzz-fit harness. 07.2 and 09.A9 become cells of #52's ladder; 09.A5
becomes a capture-and-compare mode beside the ladder, modeled on #95; 09.A19
is a template for known-bad variants. They do not overlap in what they
measure.

## 9. 32-bit and memory readings

- **Memory size.** 04.16 and 09.A11 are two builds of the same reading of
  the guest's `memory.size`; fold in one.
- **Rank on 32-bit.** 04.15 (deep-first `Sum` footprint), 09.A30 (gaps
  around `2^32`), and 06.11 (rank decode one doubling further) each overlap
  #29 or #63 and add little.
- **Decoder growth.** 06.2, 06.11, 06.13, 06.16, and 09.A34 come from the
  growth defect your input-size ruling dissolved. Only 06.2 checks something
  still in scope: a borsh decode past bit position `2^32`. Its reviewed form
  is commit `8208efaeb`, on `fix/before-wasm32-buffer-growth` (verified:
  that branch contains the commit), which is slated for deletion
  (decision 5).
- **Varied histories.** 09.A14 is the only wasm32 test over varied
  accumulator histories, but #50's committed pin catches the truncating
  mutant its oracle may bless (reported, 09).

## 10. Mutant sets and calibration

Every lane, and most builders and reviewers, built hand-chosen mutants:
01.12, 02.13, 02.14, 03.6, 04.11, 04.17, 05.17, 06.10, 06.14, 07.3, 08.2,
09.A12, 09.A16 to 09.A19, and the 24 sets of 09.A36. The adequacy lane's
cargo-mutants campaign (08.5) and its classified survivors (08.4) are the
mechanical counterpart. They overlap only in target files: the hand-chosen
sets include forms cargo-mutants does not generate (dropped blocks, added
lookups, narrowing casts, loops rewritten as recursion; reported, 07 and
01). What they share is the missing home for production-source mutants
(decision 3). The one set whose target is an instrument of record is 08.2,
the wasm32 pins' injection table.

## 11. Censuses and diagnostics

08.1 is the shared census; the others measure what it does not:

| ID | Metric it adds to 08.1 |
|---|---|
| 01.11 | two-child branch counts and unary chains |
| 02.8 | writer collapse paths and code widths |
| 03.13, 03.14 | tick lookahead sites and memo slots (03.14 is superseded by #74's floors) |
| 04.7 | integrator deferrals (needs a four-line `cfg(test)` tap) |
| 05.16, 05.19 | span placement mix and verdict counts |
| 06.12 | violation rates of non-canonical encodings |

One census module with these metrics, and floors only where a folded-in
generator relies on them, would replace eight printers (inferred).

## 12. Readers, writers, and serde

06.4 (misbehaving readers for five decoders and borsh) should generalize
#78's scripted reader and exact oracle, not stand beside it; 09.A12 is the
mutant schema that showed #78's oracle is right. 06.6 (failing writers) has
no counterpart. 09.A20 (eight real serde formats) and 06.15 (CBOR bridging)
stand beside #51.

## 13. Exact coverage of queries

#61's grid extension and the committed two-party grid check single holes
exactly on two cells. 05.4 (the boolean cube) does the same on four cells
with no new oracle. 05.1, 05.5, and 05.3 need 05.2's census. 09.A22's third
case, an off-grid point inside the two-party grid's interval, is the
counterexample behind the exactness argument that 05.3 checks; the two
belong together (inferred).

## 14. Identical instruments catalogued twice

- 08.3 and 09.A23 are the same file, `window_witness.rs` (verified by the
  adequacy cataloguer by checksum). The tables list it once, as 08.3.
- 03.5 and 09.A8 are one family at two stages, the probe and its board
  port. The tables list it once, as 03.5.
- 04.16 and 09.A11 measure the same quantity but are separate code; both
  are listed.
- The events section names section 09's entries by an earlier draft's
  numbers. In the final section 09 they are: the fixer's heap probe, 09.A4;
  the board family, 09.A8; the limb probe, 09.A9; the real-scale stepped
  spine, 09.A15; and #74's reviewer's memo property, 09.A10 (verified,
  section 09's headings).
