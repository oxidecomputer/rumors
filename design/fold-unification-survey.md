# Survey 108: the fold vocabulary of the skyline kernels

Read-only feasibility survey (branch `folds108`, base
`9af37276932119a5b609bfd9b2d16a25724dcc3c`, 2026-07-29). Product: an
honest recursion-scheme inventory of every kernel in `crates/before/src`,
the minimal fold vocabulary the code actually wants, the kernels that
resist it and why, and a migration/verification plan with an effort
estimate. No code changes; every claim cites the code it is checkable
against.

Owner intent (Finch, 2026-07-29): the exposition observes that every
operation is a constant number of folds over the skyline structure, or
over two skylines in parallel. Can all (or almost all) kernels factor
into a few fold definitions plus per-operator applications — the sketch:
at most 5 folds (1 party, 1 version, 1 party×version, 1 version×version,
1 party×party)? Secondary hypothesis, suspected over-generic: one fold
generic over whether its arguments are boolean or zigzag-encoded
skylines.

## Verdict: FEASIBLE-PARTIAL, with a different cut than the sketch

The unifying object is **one overlay-advance law over plateau cursors,
not five arity-indexed folds**. Three findings drive everything below:

1. **The sketch's `version×version` fold already exists and already
   serves three of the five arities.** `sweep.rs`'s machinery
   (`LeafCursor`/`advance`/`Step`/`fold`) carries comparison, join/meet
   emission, distance/lag, and — as its single-stream degenerate — rank
   and min_ticks; the same boundary law, re-implemented, carries
   projection (skyline×id), the masked comparisons (3–4 streams), and
   the id difference (id×id). Arity is the wrong index: at the walk seam
   the five folds collapse to **one law**, instantiated per cursor set.
2. **The law is currently hand-rolled four times** — the real
   duplication, and the strongest argument for acting: the subtlest
   bookkeeping in the crate (the tie rule's three dyadic facts) has four
   independent copies, so a hardening of one does not reach the others.
   This is exactly the genre the DRY round dissolved for
   `Party`/`Clock::join_all` (near-verbatim copies of the mechanism
   whose one-sided hardening produced the `meet_all` quadratic).
3. **The other two sketch slots (`party`, `party×party`) are mostly not
   folds over the skyline structure at all.** Their kernels
   (`sum`, `is_disjoint`, `covers`, `split`, `IdIndex`) are pruned or
   splicing *structural* walks whose entire value is skipping subtrees a
   plateau fold would visit. Forcing them onto a fold seam would be a
   regression, not a unification.

The boolean-vs-zigzag generic: **share the walk, not the algebra** — a
generic plateau-cursor trait is cheap and honest; a generic fold *body*
is the over-genericization the owner suspected, one seam higher than
where the suspicion was aimed.

Recommended effort: 2 rounds core + 1 optional. Zero behavioral delta,
total verification available (differential suites + byte-identical
deterministic meter boards). **Next-campaign**: the tree is at the
acceptance boundary, and this is a wide-review-surface pure refactor —
the right shape for a campaign opener, the wrong one for an acceptance
tail.

## 1. The inventory

Recursion schemes are classified honestly: *cata* consumes stream(s) to
a value; *ana* builds a stream from a seed; *hylo* consumes and emits in
one pass; *para* reads the raw unconsumed input, not just folded state;
*zygo* carries a second fold as a passenger.

### 1.1 Skyline plateau kernels (the overlay-law family)

All in `crates/before/src/version/skyline/`. "Overlay" = the merge walk
over the operands' elementary intervals (`sweep.rs` module doc).

| kernel | operands | scheme | fold state | early exit | emits |
|---|---|---|---|---|---|
| `sweep::causal_cmp`/`eq`/`le` | skyline × skyline | cata over overlay | `Accumulator` D + (le, ge) verdict lattice | yes (`Mode`) | `Option<Ordering>` / `bool` |
| `emit::join`/`meet` | skyline × skyline | hylo (overlay → plateaus) | D + sticky `Side` + `SkylineBuilder` | no | skyline stream |
| `query::rank` | skyline | cata (single-stream instance of the pair integral) | `Integrator` (total/live/parked/seg/base/window + promotion ledger) | no | `Rank` |
| `query::distance`/`lag` | skyline × skyline | cata (fused co-sweep) | `Integrator` + orientation σ | no | `Rank` |
| `query::min_ticks` | skyline | cata with node-close events | live `Accumulator` + `MinWeb` + `EpochLedger` (`query/web.rs`) | no | `Ticks` |
| `query::project` | skyline × id | hylo | height `Accumulator` + owned flag + `SkylineBuilder` | no | skyline stream |
| `masked::causal_cmp`/`eq` | skyline × id? × skyline × id? (3–4 streams) | cata | D, optional h_a, h_b + verdict lattice | yes (`Mode`) | `Option<Ordering>` / `bool` |

### 1.2 Skyline non-plateau kernels

| kernel | operands | scheme | fold state | early exit | emits |
|---|---|---|---|---|---|
| `validate::validate_bits`/`_prefix`/`_from` | untrusted bits | cata, strict | ~2 bits/level stack + nonnegativity `Accumulator` | yes (first violation) | `Result<(), Decode>` |
| `query::max_depth` | skyline topology | cata, payloads *skipped* (`skip_int`) | path bits + running max | no | `usize` |
| `text::render` | skyline | **tree** cata (per-node 3-summary merge) | subtree summaries (drop/span/incoming) + phase stack | no | `String` |
| `text::parse` | text | ana into `SkylineBuilder` | path-sum `Accumulator` + frames | yes (reject) | skyline stream |
| `fill::fused_fill` (`tick`/`ticks`) | skyline × id | hylo + **para** (`Out::Verbatim` references raw input) + **zygo** (`RouteProbe` cost-DP passenger) | h, `MinStack` watermark web, `Memo` ledger, `Out`, `RouteProbe` | no | skyline stream *or* `Route` |
| `grow::emit` | skyline × id × `Route` | splice ana (verbatim ranges + 2 re-coded payloads) | pending path bits + `SkylineBuilder` | no | skyline stream |
| `literal::leaf`/`node` | literals | ana | — | reject | skyline stream |
| `encode_bits`/`decode_bits` (test/meter-only) | construction language / bytes | transcoder / validate-and-adopt | path sums / as validate | — | stream / `Version` |

### 1.3 Id kernels (`crates/before/src/party/ops/`, `idbits.rs`)

| kernel | operands | scheme | fold state | early exit | emits |
|---|---|---|---|---|---|
| `diff.rs` `IdReader::diff` | id × id | hylo (plateau sweep + covered-**block splices**) | two owned bits + `Item` + `IdSkylineBuilder` | no | id stream |
| `sum.rs` `IdReader::sum` | id × id | structural merge hylo (peek + verbatim copy splices) | `Frames` (2–3 bits/node) + `IdBuilder` | yes (overlap → `None`) | `Option<id stream>` |
| `compare.rs` `is_disjoint`/`covers` | id × id | **pruned** structural cata (subtree skips) | `Lockstep` (2 bits/level) | yes (verdict) | `bool` |
| `index.rs` `IdIndex` | id (build); id × index (query) | tabulating fold; indexed pruned walk | right-child table | yes | `bool` |
| `split.rs` `IdReader::split` | id | spine walk + splice ana | — | — | (id, id) |
| `forks.rs` `Split` | id | lazy ana | stack of (Party, count) | — | share iterator |
| `idbits::skip_subtree` | id | iterative skip | O(1) counter | — | position |

### 1.4 N-ary folds and the counter (`crates/before/src/fold.rs`)

| kernel | operands | scheme | fold state | notes |
|---|---|---|---|---|
| `fold::balanced_try_fold`/`balanced_reduce` | k operands | balanced binary-counter reduction | counter stack + rejection channel | serves `Version::join_all`/`meet_all`/`Sum`/`FromIterator`, `Party::join_all`, `Clock::join_all` |
| `Integrator::settle_armings` (`query.rs`) | promotion ledger | offline **mass**-balanced product tree, iterative | `Aggregate`s + explicit control stack | deliberately hand-rolled; §4(d) |

### 1.5 Composition level and the substrate (no new walks)

- `causally::Range::placement_of`/`contains`: two comparisons composed.
- The `Version` comparison/binop matrices and `OwnVersion`'s
  (`version.rs`, `version/own.rs`): macros fanning reference
  combinations over the kernels above.
- Below the fold seam: `codec::DsiCursor`/`SliceCursor`/gamma (the read
  vocabulary), `codec::PackedBuilder` (the append-truncate move set),
  `suanpan::Accumulator` (the arithmetic every fold's state rides —
  the cliff-immunity and amortized sign reads live there, one seam
  below anything this survey moves).

## 2. The floor: what is already unified

The owner's observation is already about 70% load-bearing structure in
the tree, unnamed as such:

- **The pair walk**: `sweep.rs`'s `LeafCursor` + `advance` + `Step` +
  `fold` serve eight public operations — the comparison family, join,
  meet, distance, lag, and (single-cursor) rank, min_ticks, project.
  The module doc's boundary bookkeeping is the shared correctness
  argument, written once.
- **The n-ary seam**: `crate::fold`'s counter serves every public
  n-ary fold, with the fallible combiner carrying the party/clock
  aliased-input hand-back policy.
- **The unfold seam**: `SkylineBuilder` receives every emitted skyline
  (join, meet, project, fill, grow splice, text parse);
  `IdSkylineBuilder`/`IdBuilder` receive every emitted id (diff, sum,
  split). Both instantiate `PackedBuilder`'s append-truncate discipline.

## 3. The duplication census: what is not unified

The overlay-advance law — *the deeper cursor steps; the other steps in
the same round exactly when the flip level rises to or above its depth;
tied sides close to one shared flip level* — is implemented four times:

1. `sweep::advance` (LeafCursor × LeafCursor), `skyline/sweep.rs`;
2. `query::advance_overlay` (LeafCursor × `query::IdLeafCursor`),
   `skyline/query.rs`;
3. `masked::Walk::advance` (n ≤ 4 heterogeneous slots),
   `skyline/masked.rs`;
4. `diff`'s `advance` (two of `diff`'s own id cursors),
   `party/ops/diff.rs`.

Each carries its own copy of the tie-rule `debug_assert`s, and each is
correct only through the three dyadic facts `sweep.rs`'s module doc
derives. Additionally there are **two id plateau cursor types** with
the same flip bookkeeping written twice: `query::IdLeafCursor`
(project/masked; synthesizes absent children as unowned regions) and
`diff.rs`'s `IdLeafCursor` (adds `Enter`/`Item::Splice` covered-block
capability).

The hazard is the `join_all` genre: a boundary hardening (say, at
flush-right ties at unequal depths) applied to `sweep::advance` reaches
comparison, emission, and the pair integrals — and silently does not
reach projection, the masked walks, or the id difference. Four copies
of the subtlest invariant in the crate is precisely the shape that
produced the one-sided `meet_all` hardening.

## 4. The vocabulary

### (a) The masked walks demand their arity — composition is refuted

The candidate reduction — mask application as a stream transformer, so
`(v/p) ⋚ w` becomes a *binary* fold over a derived "projected plateau
stream" — imports the projection's materialization cost into every
comparison. The derived stream must yield the projection's deltas, and
at every ownership transition that delta is an **absolute height**:
`query::project` materializes it (`absolute_height`) and is entitled to
— the emitted code itself prices the read (the project doc: the comb ×
scattered-party cross is Θ(teeth · magnitude) *output* from linear
input, and the sweep is I/O-linear on it). A comparison emits nothing,
so there is no code to price those reads; `masked.rs` instead answers
every interval with an amortized-O(1) *sign read* on a running
accumulator and never materializes any height. Its per-operand funding
certificate (each charge names a deposit from the operand that funded
it) has no deposit that covers a transition-height materialization the
mask side triggers against the event side's width. Composition would
turn every `OwnVersion` comparison into `to_version()` cost paid
implicitly — the exact regression `OwnVersion` exists to prevent
(`version/own.rs`: "every lazy comparison costs the operands' packed
sizes, never the projection's").

Structurally the reduction also under-delivers: on single-owner
intervals the verdict needs the *other* side's height sign (the
trichotomy's zero-check) — state a projected-stream adapter hides.

So: the 3-/4-stream walks keep their arity. What they share with the
binary walks is the advance law, and that is exactly the piece worth
extracting — `masked::Walk::advance` is already written as "deepest
slot steps, then every slot whose depth reaches the flip level," i.e.
the n-ary statement of the same law.

### (b) Boolean vs zigzag: generic cursor yes, generic fold no

What the two stream types share at the fold seam is the tiling
geometry: depth, exhaustion, and step-with-flip. What they do not share
is everything downstream of the payload: the skyline crossing is a
signed wide delta feeding suanpan accumulators under a funded-width
discipline; the id crossing is one bit. The honest generic surface is
therefore the cursor:

```rust
/// A cursor over one dyadic tiling of the unit interval, yielding its
/// plateaus in preorder. The overlay law is stated once over this
/// trait; the payload each boundary carries is the cursor's own.
trait PlateauCursor {
    /// What crossing a boundary carries: the skyline's signed height
    /// delta (`Step`), the id's ownership change.
    type Crossing;
    /// The current plateau's depth (width `2^-depth`).
    fn depth(&self) -> usize;
    /// Whether the current plateau is the tiling's last.
    fn done(&self) -> bool;
    /// Advance past the current plateau: the flip level, and the
    /// crossing for the caller's algebra to fold.
    fn step(&mut self) -> (usize, Self::Crossing);
}

/// The overlay law, once: step the deeper cursor, and the other in the
/// same round when the flip level reaches its depth. Returns each
/// side's crossing (`None` for a side that did not step).
fn advance<A: PlateauCursor, B: PlateauCursor>(
    a: &mut A,
    b: &mut B,
) -> (Option<A::Crossing>, Option<B::Crossing>)
```

One precondition, itself a legibility win: today `LeafCursor::step`
*folds its delta during the step* (`step(&mut diff, side)` threads the
caller's accumulator through the cursor). The generic advance requires
separating traversal from algebra — `step` returns the `Step`, the
client folds it. Emission and the pair integral already consume the
returned `Step`s; the separation completes a pattern half-present. The
clearest beneficiary is `masked::Walk::step`, which today exists mostly
to route one delta into the right subset of three accumulators inside
the cursor dispatch; separated, that routing becomes the client loop's
visible algebra.

One seam higher — a single generic fold *body* parameterized over a
height semiring so that, say, `meet` and `diff` are one kernel — is
where the owner's over-genericization suspicion is confirmed. The
generic body would have to abstract: the id side's absent-child
synthesis (regions occupying zero stored bits), `diff`'s covered-block
splices (a boolean-only fast path with no zigzag analogue — §4(d)),
and the freeze/funding machinery (meaningless on booleans, mandatory on
heights). Every one of those hooks would appear in the signature of
every instantiation. The abstraction boundary that stays legible is the
cursor; the algebras stay concrete.

### (c) The unfold companions and the tick hylomorphism

The honest unfold vocabulary is the two collapsing builders that
already exist, and exactly two:

- `SkylineBuilder` (`skyline/build.rs`): topology from the preorder
  depth sequence; equal-sibling collapse by absorb/re-anchor
  truncation around a held code.
- `IdSkylineBuilder` (`party/ops/build.rs`): topology from the same
  depth sequence; collapse by presence-tag patching
  (`(1,1) → 1`, `(0,0) → 0`).

Both instantiate `PackedBuilder`'s append-truncate move set, and that
shared substrate is already factored. Do **not** genericize across
them: the collapse mechanisms differ in kind (held-code recognition and
re-anchor cascade vs reserved-tag patch), and the shared residue
(path/flip bookkeeping) is a minority of each builder. A
`TilingBuilder<P: Plateau>` would parameterize most of what each
builder *is*.

Tick and grow: `fused_fill` is already the hylomorphism the question
asks about, and deliberately more — the walk-plus-splice shape is the
committed design of the ticks probe (`design/probe-ticks-68.md`), with
the route DP fused as a passenger precisely so the tick is one pass.
Re-expressing it as fold-then-unfold would unfuse a fusion that was the
round's point. It stays (§4(d)).

### (d) The resisters

Each named, each with the specific reason; "resists" means the fold
abstraction would cost correctness structure, a funded-cost argument,
or a measured constant.

1. **`Integrator::settle_armings`** (`skyline/query.rs`): an *offline
   mass balancer* whose combiner charges the running total as a side
   effect of every merge, with the closing drain's association pinned
   by the committed settle readings. The cross-binding comments are
   already in the tree in both directions (`fold.rs` module doc;
   `settle_armings` doc: "the counter is an online entry-count
   balancer, this is an offline mass balancer"). Routing it through
   `crate::fold` would change the association the readings pin and
   discard the mass balance the settle bound rests on. Stays
   hand-rolled.
2. **`fill::fused_fill`** (tick): para + zygo, not a plain fold — the
   verbatim-prefix output mode reads the raw input stream, and the
   route DP rides post-order. Its walk is *structural* (`IdReader`
   driving subtree-level decisions: `fill(1, e) = max(e)` collapses a
   whole owned subtree in one block), which a plateau cursor
   linearizes away. And its limb-cost invariant is certified against
   the exact fusion (the watermark register discipline, the memo
   ledger's lifetime rules — `fill.rs` and `fill/watermark.rs` module
   docs). Migrating any layer of it onto a generic seam forfeits block
   decisions or re-opens the funding certificate. Stays.
3. **`grow::emit`**: a route-driven splice — verbatim bit-range copies
   plus exactly two re-coded payloads. There is no fold here to
   unify; the builder seam it already uses is its whole overlap with
   the vocabulary. Stays.
4. **`sum`, `is_disjoint`, `covers`, `split`, `IdIndex`**: structural
   id walks whose value is *not visiting* what a plateau fold must
   visit — `sum` copies whole subtrees verbatim on a `0` side,
   `is_disjoint`/`covers` skip a dominated side's subtree,
   `split` splices spine prefixes, `IdIndex` random-accesses right
   children precisely because cursor re-walks were the quadratic. A
   plateau-fold expression of any of them walks what they skip. Stay
   structural. (Optional same-genre cleanup: `is_disjoint` and
   `covers` share `Lockstep` but duplicate the loop shell; a
   mode-parameterized predicate walk in the style of `sweep::Mode`
   could merge them — small, bounded, not load-bearing.)
5. **`validate`**: the trust boundary. Every plateau cursor *panics* on
   non-canonical input by contract; the validator is what establishes
   that contract, so it cannot ride the cursors. Its ~2-bit frame
   stack and error taxonomy are its own. Stays.
6. **`query::max_depth`**: topology-only, payloads deliberately
   skipped (`skip_int`). `LeafCursor` decodes every payload; putting
   this pre-scan on it would decode what the walk exists to skip — a
   deterministic scan-meter regression. Stays.
7. **`text::render`**: the one genuine *tree* catamorphism — per-node
   merge of three delta-sized subtree summaries, parent-close
   information. It shares no overlay, no boundary law, and its
   transient-width argument is its own. `parse` already drives
   `SkylineBuilder` (the seam where it belongs). Both stay.
8. **`codec::DsiCursor` and the gamma vocabulary**: the read layer
   below the fold seam — the cursors are built on it; it is not a
   fold and does not enter the vocabulary.
9. **`Range::placement_of` and the comparison/binop matrices**:
   compositions over the kernels, already thin.

### The recommended vocabulary, in one table

| # | item | covers | status |
|---|---|---|---|
| 1 | `PlateauCursor` trait + one generic binary `advance` (the overlay law, stated once) | cmp family, join/meet, distance/lag, rank/min_ticks (single-cursor), project, diff's sweep | **new** — dissolves 4 hand-rolled copies |
| 2 | Two cursor payloads: skyline (`Step`) and id (ownership) | the two stream types | **new** — merges the two id plateau cursors (Phase C, measure first) |
| 3 | The n-ary loop of `masked::Walk` on trait methods, arity kept | 3-/4-stream comparisons | reshaped, not reduced (§4a) |
| 4 | `crate::fold` balanced counter | every public n-ary fold | existing, unchanged |
| 5 | `SkylineBuilder` + `IdSkylineBuilder` over `PackedBuilder` | every emitted stream | existing, deliberately two |
| 6 | (optional) mode-parameterized `Lockstep` predicate walk | `is_disjoint` + `covers` | optional cleanup |
| — | 9 named resisters | §4(d) | stay as they are |

So the honest count, against the sketch: **one** fold law at the walk
seam (not five), **two** cursor payloads, **two** unfold builders,
**one** n-ary reduction (already owned), **one** structural-lockstep
genre — plus the resisters, which are the majority of the *code* but
the minority of the *duplication*.

## 5. Migration map and verification

Ordered by dependency and risk; each phase is independently landable
and independently verifiable.

**Phase A (1 round): the law and the separation.**
Separate traversal from algebra (`LeafCursor::step` returns its `Step`;
callers fold), introduce `PlateauCursor` + the generic binary
`advance`, migrate `sweep`, `emit`, and `query`'s `pair_integral`
(rank/min_ticks touch only the step-signature change — single cursor,
no advance). Verification: the differential suites are total (recursive
oracle, exhaustive small scope, organic histories; canonical uniqueness
makes emission checks byte-identity), and the meter boards are
deterministic counters — a read-order-preserving refactor keeps **every
pinned reading byte-identical**, so the acceptance gate is "boards move
zero." Any movement is a mechanism change to investigate, never to
annotate away.

**Phase B (1 round): the overlays.**
`advance_overlay` (project) onto the generic advance;
`masked::Walk` onto trait methods, keeping its n-ary loop and its
concrete four-slot struct (no `dyn` — the slots stay monomorphic
fields; the loop's dispatch stays a static match). Gates: same suites
plus the `masked_cmp_*` and `skyline_project_*` meter rows, byte-
identical.

**Phase C (1 round, optional; measure before committing).**
Unify the two id plateau cursors — `diff`'s cursor absorbs the simple
interface, `Enter`/`Item::Splice` stay diff-side capabilities — and/or
the `Lockstep` mode merge. This phase has the least payoff and the most
shape mismatch (diff's cursor is settle-driven, query's is
synthesis-driven); at migration time, if the merged cursor is not
smaller and clearer than the two, drop the phase. Its tag-read batching
may legitimately change read order — then the id meter rows move, and
each movement is measured at the parent commit and annotated per the
metering practice.

**Non-goals, permanently**: fill/tick, grow's splice, `settle_armings`,
text render/parse internals, validate, cross-type builder generics.

**The re-pin bill**: expected **zero** for Phases A and B — the
refactor preserves which bits are read and in what order, and the
counters are deterministic. This survey promises no numbers beyond
that expectation: any nonzero movement at migration time is a finding
about the migration, and Phase C's potential movements are measured
then, not estimated now.

**Performance risk, flagged for migration time** (both on paths with
pinned constants): (i) the step/fold separation returns `Step` by value
before the fold — the `Step` is already constructed and returned today
(`advance`'s return feeds emission), so the expected delta is zero, but
the comparison rows pin the constants: check scan/limb identity, and if
a wall-clock confirmation is wanted, run it under the quiet-machine
rule; (ii) `masked`'s slot dispatch must stay static — an enum- or
trait-object-based cursor array would put dynamic dispatch on the
comparison hot path; the concrete-struct shape above avoids it by
construction.

## 6. The payoff, argued

**Legibility.** The tie rule and its debug-asserts get one home and one
doc instead of four; the two id cursors' flip bookkeeping stops being
written twice. The step/fold separation makes each kernel's algebra
visible in its own loop — today the most instructive line of the masked
walk (which accumulators a delta feeds) is buried in cursor dispatch.
The LOC delta at the walk seam is modest (order of a hundred lines);
the point is not volume but that the law becomes a named, single
artifact.

**Uniformity of hardening — the strongest case.** A boundary fix,
assert, or meter tap added to the one `advance` reaches comparison,
emission, the integrals, projection, the masked walks, and the id
difference at once. Today it reaches one of the four copies. The crate
has already paid for this lesson once at the n-ary seam; the walk seam
is the same genre with four copies instead of two.

**What it does not buy, honestly.** The accumulator algebras — the
`Integrator`, the `MinWeb`/`EpochLedger`, the watermark web, the
side-switch algebra — are the majority of the kernel code and all of
the funding arguments, and they are irreducibly per-operation. The
vocabulary shrinks the *walk* layer only. Anyone hoping the crate gets
dramatically smaller will be disappointed; anyone maintaining the
boundary bookkeeping will not.

## 7. Recommendation

Adopt the vocabulary of §4's table; run Phases A and B as the opening
rounds of the next campaign (2 rounds), with Phase C (1 round) decided
by measurement at that boundary. Do not attempt it inside this
campaign: the tree is at the acceptance boundary, the refactor's value
is zero behavioral delta with total verification, and both properties
are best served by a quiet base and an undivided review, not by racing
an acceptance tail.

Where the owner's sketch is right: there *is* a small closed fold
vocabulary, and `version×version` is its exemplar — the code already
proves it, eight operations on one walk. Where it is wrong, the
inventory says why: arity is the wrong index (three of the five arities
ride one law, so 5 collapses to 1 at the walk seam); the `party` and
`party×party` slots are mostly structural walks whose value is
precisely not being folds; and the vocabulary is incomplete without its
unfold companions and the already-owned n-ary counter. The
boolean-vs-zigzag generic is right at the cursor seam and over-generic
one seam higher — the suspicion was correct, aimed slightly low.

## 8. Outcome (2026-07-29, amendment at the track's close)

Phases A and B landed as surveyed (Phase A: `PlateauCursor`, the
generic binary `advance`, traversal separated from algebra, with
`sweep`, `emit`, and the pair integrals migrated; Phase B: projection,
the masked walk, and the id difference onto the law, plus `OpenedPair`
as the one home of the two-skyline opening move). The §4 table's rows 1
and 2 are in the tree; row 3 landed as specified (arity kept, static
dispatch, no `dyn`); row 6 remains open as the optional cleanup it was.

**The §5 re-pin bill, as measured.** The zero-movement expectation held
exactly where it was denominated — the deterministic meter boards: every
limb, scan, touch, and segment reading held byte-identical at both
scales of record through both phases, with zero verdict flips. Outside
that denomination it did not hold: the fuzzfit fuel bands re-pinned at
each phase (guest code layout from the recompile plus third-decimal
walk-kernel constants; the `bands.rs` pin-of-record annotations carry
the parent-measured decompositions), and Phase B moved the board's heap
column deliberately — `OpenedPair` drops the unread opening-height
buffers at the destructure instead of holding them to function end, so
per-byte transient peaks moved *down* on the wide-opening families.
That movement is accepted under the owner's identical-or-better ruling
(2026-07-29, mid-flight in the implementing round), which supersedes
this survey's byte-identity acceptance gate for deliberate, accounted
improvements; regressions remain findings.

**Phase C: measured at the boundary and dropped**, by this survey's own
criterion. The merged id cursor is not smaller and clearer than the
two: the residual duplication is ~12 mechanical lines of flip
bookkeeping, while the cursors' descent policies genuinely differ —
`query`'s settles eagerly inside its step (its plateaus are the stored
regions), `diff`'s defers settlement so the covered-block scans can
skip what the sweep never visits. A merge either wraps an eager adapter
newtype around the settle-driven cursor or imports covered-block
machinery into the altitude of `project` and the masked walk, and
carries a nonzero re-pin bill either way — machinery and cost bought
against twelve lines.
