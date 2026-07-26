# The tick/fill cost specification

Status: the statement of record for #34 (the tick limb cure and
fusion). **T-tick stands as the theorem under I4′ — [measured at
kernel] on every committed family — realized inside the fused tick.**
The design loop that produced it (seven adversarial attack/fix rounds
plus the fusion landing, spec tier through kernel tier) is §9's
compact record; each round's full narrative, harness transcripts, and
per-round number tables are in git history
(`git log design/before-tick-cost-spec.md`) at the revisions the
round dates name. The loop ran under the user's standing
authorization of 2026-07-25 (fused tick pre-approved given linearity
with small constants; confer only on a superlinear honest optimum, a
§6-of-the-campaign-doc denomination change, or a
representation-forced redesign); formal ratification by Finch lands
here as a dated amendment — the formal campaign's Phase 0
(`design/before-formal-tick.md`) schedules the ratification read.
Spec-first: this is the English statement of record the kernel
implements against; deviations get dated amendments, never silent
drift. Statement-faithfulness governs every claim below — never
weaker than stated, never stronger than proven; each clause carries
its epistemic tag: **[measured]** (instrumented kernel run),
**[measured-on-model]** (the executable discipline model, not the
kernel), **[derived]** (argument from code or arithmetic), **[open]**
(known unknown; must be resolved or the claim weakened before it is
committed as prose).

Model-tier provenance: the executable discipline models and attack
harnesses were per-round transient probes; every number cited from
them was independently reproduced by the round that cites it before
being recorded, and the durable instruments are the committed
generators, gate pins, envelope rows, and board cells named below.
The model-tier cost placements were confirmed against
`codec/accum.rs` by line-read (`apply_limbs` touches every limb
including zeros, `add_accum_shl` every held digit, `sign` per
scanned digit plus collapse, `sign_magnitude` per held digit plus
the complement pass).

## 1. The function of record

The paper's equations (ITC 2008 §5.3.4; `reference/itc2008.md`),
transcribed exactly:

    event(i, e) = { (i, fill(i, e))  if fill(i, e) ≠ e,
                    (i, e′)          otherwise, where (e′, c) = grow(i, e).

    fill(0, e) = e,
    fill(1, e) = max(e),
    fill(i, n) = n,
    fill((1, ir), (n, el, er)) = norm((n, max(max(el), min(e′r)), e′r)),
                                 where e′r = fill(ir, er),
    fill((il, 1), (n, el, er)) = norm((n, e′l, max(max(er), min(e′l)))),
                                 where e′l = fill(il, el),
    fill((il, ir), (n, el, er)) = norm((n, fill(il, el), fill(ir, er))).

    grow(1, n) = (n + 1, 0),
    grow(i, n) = (e′, c + N), where (e′, c) = grow(i, (n, 0, 0)),
    grow((0, ir), (n, el, er)) = ((n, el, e′r), cr + 1),
    grow((il, 0), (n, el, er)) = ((n, e′l, er), cl + 1),
    grow((il, ir), (n, el, er)) = the cheaper child, cost + 1
                                  (lexicographic (expansions, depth),
                                  ties right — the oracle's realization).

`before` renames `event` to `tick`. The semantic definition of record
is the recursive oracle (`oracle/version.rs`: `fill`, `grow`,
`event`); the skyline kernel is byte-identical to it under the
differential suite, and `norm` is realized by the collapsing builder's
equal-sibling normalization plus min-sinking, never as a separate
pass. **Fill and grow never run separately**: every public entry is
`tick = fill, falling back to grow iff fill changed nothing`, decided
inside the fused walk by the changed flag (§6).

Fill's per-node needs, read off the equations — this is the whole
semantic inventory the cost design must serve:

- **range max** (`max(e)` / `max(el)` / `max(er)`): the collapse value
  of a fully-owned region and one argument of every raise;
- **range min of a *filled* subtree** (`min(e′r)` / `min(e′l)`): the
  other raise argument — a min over *emitted* values, not input
  values;
- **orderings only, at the joins**: `max(a, b)` and the builder's
  equal-sibling test need which-is-larger; a *materialized* magnitude
  is needed only where a code is emitted, and each emitted code prices
  its own width.

Stream-order asymmetry (why the two shortcut arms differ): the
right-full arm's min (`min(e′l)`) is over a range the walk has already
emitted when the decision is made — the kernel's deferral gets it
in-pass. The left-full arm's min (`min(e′r)`) is over a range the
walk has *not yet reached* when the raised leaf must be emitted —
hence the (memoized) pre-scan. Both needs are LIFO-nested range
extrema over one preorder stream.

## 2. The refutation this spec answers

The fill kernel as first landed (`92d2fc31`) returned per-subtree
`SubtreeOut = (min, net)` as **materialized `Base` magnitudes**,
combined by a signed sum at every paired node — Θ(width) limb work
per ancestor. Depth × width is not bounded by input bits.

**[measured]** at that tip (doubling b = d):

- Right-full chain, `tick(bigroot(b,d) × nested_full_id(d))`: limb
  step exponent 1.57 → 1.71 → 1.83 (→ 2), constant 29.4 → 126.3
  ops/B, crossing the board's 128/B ceiling at b = d = 8000. Scan
  flat 14.2 bits/B, e 1.00 at every step.
- **Left-full (memoized pre-scan) chain** — `tick` of the mirror id
  `(1, (1, … (1, 0))))` over a right-leaning spine with one wide
  bottom leaf: limb e 1.92 → 1.94 → 1.97, constant
  **143 → 1015 ops/B** — over the ceiling already at d = 1000,
  steeper than the right-full arm. The pre-scan's own summation
  chains carried the same genre. Scan flat 28.4 bits/B, e 1.00.

Both arms are public-API-reachable through `Version::tick`/
`Clock::tick`. Consistent with the campaign doc's §3 entry: the
multiplier is the *local* id's paired depth — a pricing obligation
under the campaign's §6 invariant, not a hostile-peer exploit.

## 3. The theorem and its decomposition

> **T-tick.** `tick` (hence `fill` and `grow`) is computable over
> skyline streams in amortized **O(n + m) Accum digit touches** in
> the two packed operands' bits, with no bound on value magnitude,
> tree depth, or encoded size — alongside the already-achieved
> O(n + m) scan bits.

Status: **the theorem under I4′, [measured at kernel] on every
committed family, realized inside the fused tick** (§9 rounds 6–8).
Every lemma below is [measured], [measured-on-model], or [derived]
with no load-bearing [open] clause. The reduction: every signed
quantity fill needs lives in **one shared anchor web** — the running
height `h`, and per-open-range extremum watermarks represented as
*differences*, never absolutes.

**Representation compliance (user ruling 2026-07-25)**: everything
here — the watermark web, the diff coding, the residue propagation,
the fusion, the copy-on-first-divergence builder — is **auxiliary
in-memory walk state over the unchanged skyline coded form**. The
at-rest coding, byte equality as `Eq`/`Hash`, and the
identity/persistence story are untouched; no clause of the
derivation requires or pushes toward a third representation. The
ruling's confer-level escalation (§5) is recorded for completeness,
not because any evidence points there.

**The discipline's invariants** (the kernel implements these; each
echoes a landed precedent):

- **I1 — single-fold**: each consumed input code folds into O(1)
  accumulators (h, the innermost armed watermark, the live gap) —
  never one per open range. (Precedent: the emit kernel's
  gap/offset discipline.)
- **I2 — difference-coded cross-references**: wide content is stored
  once; every cross-level relation (outer vs inner range watermark)
  is a nonnegative diff `Accum`; pushes are moves, not copies.
  (Precedent: the grow probe's key-delta registers.)
- **I3 — reads at death or priced by output**: a wide magnitude is
  materialized only when its operand dies (folded once into a
  survivor) or when an emitted code prices it (the emission's own
  width). (Precedent: the V6 freeze cure's drift charging.)
- **I4 — residue-directed propagation over a zero-run-compressed
  stack** (the uncompressed form is FALSIFIED, §9 round 1): the diff
  stack stores zero-diff *runs* as one O(1) entry and nonzero diffs
  individually. A min-update reaching k outward frames folds the
  *dying nonzero* diffs into the running residue (each nonzero diff
  dies at most once after creation), passes whole zero runs in O(1)
  each — their minima track the innermost watermark implicitly —
  and makes exactly one surviving fold at the stopping frame,
  bounded by the update's own priced width. Never fold the residue
  into surviving frames. The ledger clause is per-object, and wide
  *content* may nonetheless cascade through a chain of deaths — a
  dying payload folds into the residue, which survives as a new
  diff that can die again — **provided every hop is separately
  funded**: a full-penetration undercut reaching a wide diff
  requires the running height to have descended past that diff's
  span, which the input paid in delta codes (the
  `resurrection-cycle` schedule measures funded hop chains flat,
  5.05/unit; partial penetrations never reach the wide diff). Zero
  runs cannot absorb a shortfall, so propagation never stops inside
  a run and runs only grow or shrink at their ends — no split
  operation exists to churn (`run-boundary-churn`, 6.23/unit flat).
  Without the compression, zero-diff frames cost ≥ 2 touches each
  and never absorb, and a full-penetration update pays Θ(open
  depth) — the descending-staircase family is Θ(d²) **[measured]**
  (naive 8002 touches/unit at d = 8000, ratio 4.00/doubling;
  compressed 7.00 flat, ratio 2.00; 13.00 flat under limb-faithful
  accounting, class unchanged).
- **I4′ — width conservation (SUBSUMES I4's funding enumeration —
  I4 is retained as mechanism)**: define the potential
  Φ = c · Σ over live accumulators of (held digit count), c a fixed
  small constant. Every digit touch the walk performs is paid by one
  of exactly three sources: an input code being consumed (which may
  also GROW Φ, by at most the code's own width), an output code being
  emitted (materializations read at most the code's width + 2, the
  collapse-slack bound), or a DROP in Φ. Φ never grows except at
  input-consuming folds; hence total touches ≤ O(1)·events +
  O(n + m + Σ emitted code widths), and by L6 the total is O(n + m),
  input-denominated. The operational rules (the normative content;
  the potential is their bookkeeping): (1) moves are free — a buffer
  transfer costs O(1) and leaves Φ unchanged; (2) internal folds are
  min-into-max with a dying source — the narrower buffer's content
  folds into the wider buffer and the survivor takes the wider buffer
  (an O(1) swap), never the reverse; cost = the narrower side's
  digits = Φ's drop; (3) fold-and-restore only of priced operands
  (L2 rule 1 unchanged); (4) dying-operand fan-out is a constant —
  one death event funds O(1) reads of the dying value (the survivor
  fold, the d_out fold, ≤ 2 follower fixes); (5) sign reads decide by
  domination before any fold — scale-disparate comparisons answer
  post-sign from top indexes in O(1) (`sign_dominates_at`, the |s| ≥ 3
  bound at an arbitrary index floor), and only comparable scales
  fold, Φ-funded by rule 2. The complete hop enumeration ("hop" = any
  event moving wide content between accumulators), each with its
  funding source:

  | hop kind | what moves | funding |
  |---|---|---|
  | input fold | a consumed code into the O(1) I1 targets | the code itself; Φ may grow by ≤ its width |
  | undercut cascade | dying diffs into the residue, one surviving stopping fold | each diff dies (rule 2, Φ); the stopping fold ≤ the update's priced width (I4 as before) |
  | arm-up | `t_old` into the new boundary diff | a MOVE (rule 1); the narrow `v − A` adjustment folds at its priced width; a wide `t_old` was input-funded by the climb that built it |
  | close-reveal | the popped boundary | a MOVE into the latent register (rule 1) — no fold, no follower touch; the fold-into-`t` form is FORBIDDEN by rule 2 |
  | latent-recycle | the latent buffer into the next arm's boundary | a MOVE; the narrow anchor-relative offset folds into the wide latent buffer (rule 2 direction) |
  | latent-merge | two stacked latents | the narrower dies into the wider buffer (rule 2, Φ) |
  | latent-collapse | the latent into `t` (anchor re-base) | the latent dies; min-into-max buffer swap (rule 2, Φ); triggered only at comparable-scale decisions, true undercut penetration (annihilation), or output-priced reads — never by schedule alone |
  | ledger link | a link into the consume decision / the recording head into the queue | the link's one death (the frame ledger's discipline, §9 round 4); wide links are input-funded at mint |
  | bridge / anchor switch | the surviving `t` (+ latent) into a d_out | priced by the switch emission's code (L2 rule 2c); the watermark-to-height switch cancels the latent symbolically; the height-to-watermark switch retires it (collapse-first, Φ) |
  | web death (last armed close) | the dying `t` and latent | the dying operands, rule-2 direction only; kernel-side the hop is vacuous — followers die first (asserted) and the contents drop unread; the model's swap realization with its O(1) sign tag binds any future cross-web survivor |
  | follower maintenance | arm/undercut/collapse operands into ≤ 2 followers | the same funded operand, fan-out constant (rule 4); closes touch NO follower — the σ tag (`f_true = f_stored − Λ`) carries the relation across the reveal, resolved at each follower's own death |

**L0 (scan) [measured, landed]**: O(n + m) scan bits — every position
read at most twice (walk + at most one memoized pre-scan; flat ×2
absent-sibling scans). e 1.00 on every family, both arms.

**L1 (the watermark stack) [measured-on-model + kernel]**: a LIFO
stack of range-min watermarks over one running height, coded as
`T = below(innermost)` plus outward nonneg diffs
(`below = h − min`, over emitted values) **on the zero-run-compressed
stack (I4 — the compression is normative, not an optimization)**,
supports open (move), delta (one fold into T — uniform shift), emit
(arm / compare / residue-propagate), and close (one sign read + one
dying-diff fold or run decrement) in amortized O(1) digit touches
plus O(width) charged to dying operands. Model evidence: the
limb-faithful executable of the composed discipline, under a
full-stack value oracle asserting every armed frame's watermark at
every step (plus per-emission `d_out` exactness and the `last`
invariant), measures all fourteen committed schedules linear —
6.2–19.1 touches/unit, ratio 2.00 per doubling at d = 1k → 8k —
including `descending-staircase` (Θ(d²) uncompressed),
`sawtooth-partial`, `rearming-churn`, `wide-dip`, and `cliff-churn`.
Model limitation (normative): the oracle asserts *values*; costs are
validated by structural mirroring against `codec/accum.rs`'s touch
placement — which is why kernel-side re-measurement at the red pins
was mandatory (§7) and is done (§9 rounds 4–7).

**L1′ (L1's cost clause under the latent boundary) [measured,
kernel + model]**: under I4′'s rules 1–5 the latent-boundary
`MinStack` — `t = h − A` for a stale anchor `A`, the excess
`Λ = A − m` parked in one optional register at the top of the diff
stack, followers anchor-relative under a one-bit σ tag — supports
open, delta, emit (all arms), close, consume, and materialize in
amortized O(1) digit touches per event plus O(width) charged to
consumed input codes and emitted output codes. Per operation: open
O(1); delta folds into ≤ 4 fixed accumulators at the code's own
width (I1); a non-min-moving emit is one amortized sign read, with
domination reads O(1) post-collapse; an arming emit moves the
boundary, folds the priced offset narrow, and recycles the latent by
a rule-2 fold; an undercut pays dying digits (Φ-drop) plus one
stopping fold at the update's priced width, annihilating a
penetrated latent — and this clause holds *in the code*:
`propagate`'s top-index domination decides each hop's direction in
O(1) before any fold, and the dying side's digits fund each
pass-through hop (§9 round 7); a close is O(1) — a run decrement, a
mint move, or a merge costing the dying narrower latent's digits; a
collapse fires only at comparable scales, alongside an output-priced
read, or at a death event, kills the latent, and cannot recur on one
funding (re-widening costs the input a fresh climb); a consume's
link dies into the anchor-relative decision; a materialization is
O(the emitted code's width) post-collapse (the +2 slack bound).
Summing: touches ≤ O(1)·events + Σ input widths + Σ output widths +
Φ's total drop, and Φ's rise ≤ Σ input widths + O(1)·events with
Φ ≥ 0, so the drop telescopes; the pricing chain (L2×L6) converts
the output term. Every hop kind of I4′'s enumeration appears in
exactly one clause. Kernel realization: the `watermark` module's
latent register, with `sign_dominates_at` and `merge_into_wider` as
the accum-layer primitives; validated [measured] by the flipped pins
and the green bands (§9 rounds 6–7).

**L2 (emission pricing) [measured-on-model, composed and
limb-faithful]**: every materialized quantity at an emission point is
O(the emitted code's width + its consuming scan's own disjoint range
content). A charge "by construction" is insufficient — the
fold-in/sign/fold-*back-out* comparison path is quadratic under wide
offsets with cheap codes (`wide-off-churn`: 95 → 729 touches/unit,
ratio → 3.99, on naive AND compressed stacks alike; §9 round 1) —
so the discipline below is normative, and the committed tripwire
reads 8.14 touches/unit flat (ratio 2.00 per doubling) on the same
schedule under it:

1. **Never fold-and-restore a wide operand**: a comparison folds a
   *dying* operand into the survivor and reads one sign; no wide
   operand is ever folded in only to be folded back out. Where a
   fold-and-restore is unavoidable at comparable scales, **only ever
   the NARROW (priced) side** is folded and restored.
2. **Per-operand lifetime accounting, priced**: every wide quantity
   is created once (funded by input content or a dying predecessor)
   and dies once (folded into a survivor); every wide READ is (a)
   its death, at most once per operand; (b) an O(1)-per-lifetime
   read; or (c) **priced by the width of the code emitted at that
   site** — clause (c) is I3's priced-by-output arm, restated here
   because anchor-switch bridges validly read the surviving `T`
   once per switch, unboundedly often per lifetime, and the bound
   is by pricing, not count: for the bridge to be wide, `T` must be
   wide, so the emitted `d_out` is wide and its own code covers the
   read (the funded-flip schedule measures the constant, 3.5–3.9
   touches/unit flat). The wide-off family under this rule:
   consecutive raise targets differ by cheap priced deltas, so the
   offset rides ONE live accumulator maintained incrementally
   (`last = value_prev − anchor`, folding input deltas between
   emissions — one more I1 citizen), never k fresh wide operands.
   `last` is **anchor-tagged**: h-relative between pass-throughs
   (folds input deltas), watermark-relative between raises
   (drift-invariant, adjusted only at priced min-movements and at
   funded anchor deaths); its anchor generalizes to *the
   accumulator that sourced the last emission* — a consumed memo
   entry (L4) staying live on the walk's LIFO stack anchors it the
   same way, and its death re-anchors `last` by one funded fold,
   the same mechanics as a watermark-anchor death.
3. **The anchored-entry discipline** (the comparison-path quadratic
   lives equally in the watermark stack's own min-update comparison,
   and rules 1–2 alone leave that path expressible; an
   implementation violating any clause here is non-conforming):
   emissions enter the stack **anchor-relative** — a raise near the
   tracked min enters as (watermark anchor, small adjustment), never
   as an h-relative wide offset, so an undercut grows `T` by the
   adjustment plus a cheap residue propagation; **`T` is never
   folded into anything while it survives**; arming **moves** `t`
   into the diff stack (I2's pushes-are-moves, restated at the emit
   path where it is load-bearing); armed comparisons run `t.sign()`
   first (collapse, amortized), then **top-index domination** when
   the scales are disparate (post-sign, top ≥ 3 with positive sign
   decides against any word-scale adjustment in O(1) — the accum
   module doc's |s| ≥ 3 bound; this guard is also a comparison-site
   rule under L3), and only at comparable scales fold — narrow side
   only, per rule 1.
4. **Materialize post-collapse only**: `sign()` first (the collapse
   is amortized against the writes that built any cancelling prefix
   — `codec::accum`'s own argument), then `sign_magnitude`. After
   collapse the held digit count exceeds the value's width by **at
   most 2 digits [derived, probe-confirmed]**: `sign()` decides at
   index i with |partial| ≥ 3, unscanned digits below contribute
   < 2.01·2^(32i) (the module doc's domination bound), so |value| ≥
   0.99·2^(32i) — width ≥ 32·i — while the collapse leaves top ≈ i
   plus ≤ 2 redeposit carries. Probe-confirmed twice: worst slack 0
   digits over 2000 randomized cancelling constructions and over
   548 structured adversarial constructions (deep cancelling
   prefixes to 4096 bits, multi-scale alternations, telescoping
   staircases, collapse/rebuild cycles, near-threshold partials).
   So every materialization is O(the emitted code's width). The
   emitted output delta is `d_out`: fold the dying `last` in (plus
   the `t` bridge on an anchor switch, per rule 2c), collapse,
   materialize — priced by the code.
5. **The gap accumulator survives** by folding the materialized
   `d_out` back in — O(code width), priced by the output the code
   is.

**The pricing chain (normative lemma)**: L2's per-emission costs are
priced by the **emitted code's width**; Σ emitted code widths ≤
2·input + O(id) by **L6**; hence L2's total limb cost is O(n + m)
**input-denominated**. This sentence is load-bearing, not
decorative: the `anchor-flip-UNFUNDED` schedule measures
61.7 → 444 touches/unit (ratio 3.96) when the denominator is denied
the output-pricing — superlinear against input-only content — and
that reading is NOT a kernel counterexample precisely and only
because of this chain: for a bridge to be wide, the tracked min must
sit far below the running height; an h-anchored emission is then
genuinely far from the previous value, so `d_out` is wide, its
output code is wide, and L6 telescopes the output back to input
content (emitted values are input-derived extrema, and nested range
mins are monotone — outer ≤ inner — so a raise chain's total
movement telescopes against the content that created the spread).
T-tick's proof outline reads: L1/L1′ (stack maintenance, amortized
O(1) per event) + L2 (emissions priced by output codes) + the
pricing chain (output ≤ 2·input + O(id), L6) ⟹ amortized O(n + m).
If any link fails at a future kernel measurement, §5 applies.

**L3 (decisions are sign reads) [derived; enumeration maintained at
each landing]**: `signed_min`/`signed_max`/the equal-sibling test
need orderings only; under I2 every comparison is `sign()` of a
same-anchor difference, amortized O(1) by the Accum's domination
bound + collapse amortization (`codec::accum` module doc). The
comparison-site enumeration is re-run against the kernel's own
final sites at every cure landing (a rewrite must not mint an
uncovered site); the current sites, each enumerated at its
realization: the raise comparisons (fold-on-death, L2 rule 1),
the emission sites (L2's discipline), the pre-scan's own stack
instance with its recorder's per-site sign read (L4), the builder's
equal-sibling seam (`build.rs` — a zero-delta *code* check, one bit
pattern, O(1), no magnitude comparison), the domination ladder, the
anchor-relative consume decision, the recorder's pre-resolve, and
the propagate hop's domination read.

**L4 (the pre-scan inherits the discipline) [derived + measured]**:
`min_fill_rec` has the same walk shape (LIFO ranges over the same
stream), so the same watermark web applies. Normative clauses:

- **The anchor seam is sound as-is** (verified by inspection): a
  memo entry is recorded at stream position `l_end` relative to the
  height there and consumed by the walk at the same position, where
  the walk's height equals it — height is a function of stream
  position; no re-anchoring arithmetic exists or is needed at
  consumption. The seam is sound under mixed nesting by projection:
  entries are recorded and consumed at the same stream positions,
  consumption order is stream order, memo ranges nest within walk
  ranges, and a projected subsequence of a LIFO nesting is LIFO;
  two independent fresh-scan chains cover disjoint ranges with
  independently funded heads, so no cross-chain diff is ever
  needed.
- **Memo entries are diff-coded along the recording chain against
  the site forest's own final minima** (the frame ledger, §9
  round 4): one link per site — sibling links at their closes,
  first-child links deferred to the forest parent's close where its
  minimum is final, the scan-entry height as the outermost base;
  zero links unstored; one live min-relative head plus immutable
  suspended diffs carry the recording state, and the walk needs no
  carry at all — its live relation IS each link's reference at
  arrival, re-anchored from its own web at every site close.
  Without this clause an implementor can faithfully rebuild the
  quadratic: k nested left-full sites sharing one wide minimum
  would materialize k wide entries at creation, Θ(k·W/64) inside
  the pre-scan. Consecutive nested entries differ by cheap priced
  content; at consumption an outer entry's accumulator stays live
  while the walk is inside its range, so inner entries resolve as
  diffs against it — the ranges nest, so LIFO holds, and L2's
  per-operand lifetime accounting then covers the memo. A live
  consumed entry may anchor `last` (L2 rule 2's generalized
  anchor); its death re-anchors by one funded fold. Ledger links
  and suspends never snapshot a latent-relative quantity
  (asserted).

**L5 (auxiliary space) [derived + measured]**: heap = O(paired
depth) frames plus total live Accum digits ≤ O(digits ever placed) =
O(content folded) = O(n + m); the memo holds ≤ one entry per
left-full site, each priced by its disjoint fresh-scan range;
zero-run compression only removes stack entries; the diff-coded memo
removes the fan-out threat (one wide value shared by k entries never
multiplies into k wide materializations); per-emission `d_out`
accumulators are created and dropped at ≤ value width + 2 digits (L2
rule 4's slack bound); the latent register is one pooled buffer and
the σ tags are bits. Measured at heap parity on the ledger families
(one queue word per site); the two owned linear heap *constants* —
mirror-narrow's per-site memo entries and the ascending cliff's k
simultaneously-armed unit-width difference buffers — are honest
board reds with named candidate cures (the campaign doc's §17.3).

## 4. Per-dimension cost model (what the board holds tick to)

| dimension | target profile | status |
|---|---|---|
| scan bits | O(n + m), constant ≤ 2 reads/position + flat ×2 sibling scans | **[measured]** landed, e 1.00 both arms |
| limb ops | **T-tick: amortized O(n + m)** | **[measured at kernel]**: I4′/L1′ + L2 realized by the latent-boundary register and the fold-direction cure; linear on every committed family including the close-reveal genre (reveal-comb ×2.00 across the joint doubling, pure-comb flat per byte, hifloor/mirror-wide/fanout inside their bands) and the undercut-cascade genre (ascend-cliff ×2.00 across the joint doubling); the model tier validated the discipline composed across the fourteen committed schedules plus the seven round-6 attack schedules |
| heap | O(depth) frames + O(n + m) total digits; builder output | **[derived]** L5, pinned by existing heap columns |
| segments | today O(paired depth) recursion (red-pinned at ×4, owner **P4.2**); eventual profile **O(1) grown segments** via explicit stacks — the watermark stack's discipline and the fused walk's bit-coded expansion frames (`fill/fuse.rs`) are the natural vehicle, so P4.2 implements against this line. Sequencing note: the fused walk's route fold reads each skipped id subtree per 2-bit tag on leaf-under-internal-id arms (the expansion DP visits every node the skip visits) — P4.2's word-scale skip on those arms interacts with the landed fused walk; P4.2's sequencing decision must name it, and §9 round 8's before/after table carries the landed interaction baseline | **[measured]** red today; target [derived] |
| denominator | **input-denominated stands** (n + m packed bits; the campaign doc's §6 do-not-re-denominate list). Supporting lemma L6 **[measured, pinned; the additive form is REFUTED]**: `size(tick(e, i)) ≤ 2·size(e) + 4·size(i) + 32` (`tick_output_is_input_bounded`, committed with its shrunk counterexample seeds — grow's zero leaf and a raise's landing can each re-code one delta against a wide neighbor, duplicating one input code's width once, so no additive slack survives; the honest constant is the factor 2, realized at 1.5 by construction; the committed pin runs at proptest's default 256 cases). Fill's output deltas otherwise telescope input deltas; grow adds one increment or one expansion chain ≤ O(m). The pricing chain carries the constant: Σ emitted ≤ 2·input + O(id), still input-denominated. The ORBIT is separately pinned: the factor cannot compound along `tick^k` (§9 round 3's orbit lemma: orbit values ≤ max input value + k, codes within max(input width, log k) + O(1), live nodes ≤ input leaves + id internal nodes, `bits(tick^k) ≤ bits(tick^1) + 4·bits(id) + 4·⌈log2(k+1)⌉ + 8` after the one-step ≤2× transient — pinned by `tick_orbit_growth_is_transient_plus_log` and `tick_deep_orbits_stay_banded`). | |

## 5. If a lemma's realization is ever refuted

T-tick stands measured on every committed family; §9's rounds 3, 5,
and 7 are the precedent that a new family can refute a realization
the committed set blessed. No fallback below the theorem satisfies
the asymptotic bar: the honest worst case of a per-subtree
materializing kernel is O(n·m/64) limb ops (depth × width),
quadratic in total input — **below the bar** — and there is no known
intermediate (the problem is re-touching, not sorting). So the
spec's position: a future refutation comes back to this document
with the counterexample family, red-pinned, and the decision is
Finch's, made against the recorded evidence — not silently absorbed
into prose.

The escalation ladder at that point (user ruling 2026-07-25 — stay
within the skyline representation if at all possible):

1. **Next in-skyline candidates first** (all auxiliary in-memory
   state, fair game): the epoch/interning lens on the same idea
   (wide folds into `h` mint epoch anchors; relative quantities
   carry (epoch, narrow offset); cross-epoch orderings memoized once
   per pair), or a two-pass position-first walk (extrema located by
   stream position in a value-free pass, values derived in a second
   anchored pass).
2. **Accept a stated-band residual on tick** — the band and family
   committed, the board cell red with a permanent owner — if the
   obstruction is fundamental but narrow.
3. **A representation change is a CONFER-LEVEL finding, never a
   recommendation from this document**: if the evidence ever shows
   linear tick is achievable *only* by replacing the skyline at-rest
   coding, the deliverable is the obstruction (which lemma fails and
   why it is representation-forced, with the counterexample), the
   sketch of what a third coding would buy, and an honest blast
   radius accounting in the C2 flag day's reference class (identity/
   persistence break, bookmark version, every kernel and snapshot
   re-pinned, the compactness envelope re-measured) — presented for
   Finch's decision. No evidence points there: every obstruction
   found so far was walk-structural, not coding-structural, and
   every cure was auxiliary walk state.

## 6. The fusion (fill + grow → one tick pass; LANDED)

`tick` is one fused fill walk plus at most one splice (`fill.rs` +
its `fuse` submodule; the splice in `grow.rs`), landed as one
bisectable commit (`80131954`; the record is §9 round 8) under two
owner rulings of 2026-07-26: **no composed fill/compare/grow path is
retained** — the differentials of record pin the fused `tick`
directly to the recursive oracle — and **no runtime byte-compare
assert is retained** — the committed differentials are the entire
pin of the flag seam (the standing practice retires
shadow-recompute asserts as soon as proptests cover semantic
equivalence to the oracle, and the same round landed exactly that
coverage).

**The landed shape**: the walk's output starts as a *verbatim
reference* — while every emitted plateau equals the input plateau it
replaces (pass-throughs by construction; collapse/raise emissions
iff the consumed range was a single leaf and the offset's value is
zero; the raise-to-minimum arms diverge unconditionally, their value
strictly exceeding the range maximum), nothing is built — and the
first divergence materializes the matched prefix wholesale into the
collapsing builder (copy-on-first-divergence, realized as a builder
mode, not a shadow build). Grow's (expansions, depth) route DP rides
the same walk: two fold sites in the recursion (the internal-node
join and the leaf-under-internal-id expansion fold, whose per-tag id
reads are exactly the reads the lazy skips already paid) record one
direction bit per id branch position; the probe dies at the first
divergence, so the fill branch pays route bookkeeping only over its
matched prefix. The unchanged branch hands the route to the splice
emit, whose full-id-over-event-node arm is asserted unreachable
(fill would have collapsed the region and tripped the flag),
shrinking the route to id-keyed bits only. On the grow branch —
the common one for the hottest real pattern, a peer repeatedly
ticking its own version — the flag never trips, so fill's discarded
output is **never built at all**: one flag+route walk plus one
splice, near-zero transient heap, 2 stream traversals where the
composition needed 4.

**The changed flag** is `changed ≡ (fill(e) ≠ e)`, defined as
*emitted-differs-from-input* (never "an arm fired" — a raise can
reproduce the existing leaf value exactly, and `max(L, …) ≥ L` means
value-unchanged raises are real). The equivalence is structural:
canonical uniqueness makes byte inequality ⟺ some plateau differs,
and the comparison only needs to run while every emitted plateau
equals its input range, during which output position ≡ input
position exactly; at the first divergence the flag trips and the
comparison stops mattering — the flag is precisely a
first-divergence detector, which is also why
copy-on-first-divergence composes with it for free. One normative
special case: **the FIRST emitted leaf is coded absolute while later
leaves are deltas — the flag's comparison at the first-leaf site
compares the absolute code against the input's absolute code, never
a delta against an absolute**; the alignment is by plateau (depth,
code), so a collapse shifting which input leaf is "first" already
trips the flag on topology before any code comparison is reached.

**The flag's pins, both axes**: the size axis rides the oracle
differentials (flag ≡ the oracle's `fill` moved the tree; `tick` ≡
the oracle's `event`), the deep 4096-level closed-form witnesses,
the meter suite's closed-form output asserts, and the board's
determinism tripwire. The width axis is pinned separately — those
instruments cover size only, and a low-64-bit-truncated value
compare at the flag site would pass all of them (a raise offset
that is a nonzero multiple of 2^64 reads as zero under truncation;
the flag stays clear and the pair mis-routes to grow): committed
full-width flag witnesses (an offset of exactly 2^64 must trip the
flag; a wide value-reproducing raise's full-width-zero offset must
not), a 2^64-aligned arm in the arbitrary base generator keeping
the low-limb-zero class under ongoing generator mass (the arm
caught the re-planted mutant through the arbitrary-pairs
differential independently of the fixed witnesses; the shrunk seed
is committed), and the acceptance demonstration that the re-planted
mutant reads red under both (commit `74d6ec5e`).

**Costs, measured at the landing** (§9 round 8's table): the route
bookkeeping tax on the fill branch is invisible at board resolution
(benign, dense, nested-wide byte-identical on every work column —
the parity question answered at exact parity; the one measurable
trace is +2 limb ops and one extra stack segment on the
tick_nested_wide envelope row, inside standing ceilings). The
watermark stack's per-range state is pool-reused (an `Accum::new()`
per range open would be an allocation per node); the fused walk's
new paths never bypass the pool. On leaf-under-internal-id arms the
route fold reads the skipped id subtree per 2-bit tag — the
interaction P4.2's word-scale-skip sequencing must name (§4's
segments row). Legibility is an obligation on the implementation's
prose, not a design criterion (user ruling 2026-07-25): the
recursive oracle remains the readable reference matching the
paper's equations, the differential suite guarantees semantic
accuracy against it, and the fused module doc explains the walk
against those equations (the flag and route live in their own state
struct; the arms call into them at ≤ 3 sites).

## 7. Witness families, pins, and the acceptance contract

The committed kernel families and their pins, as they stand (all in
the differential pools event- and id-side — semantics before cost —
with gate pins in `tests/meter.rs` and board rows at both scales;
the campaign doc's §2 carries the family vocabulary):

- **bigroot × nested_full_id** (the right-full chain) and **the
  mirror crosses** (`nested_left_full_id` × wide-tail; the
  memo arm): the #34 wide×deep red pins, flipped by the anchor-web
  walk and the chained-memo pre-scan — limb/heap e 1.00 at flat
  constants, envelope rows `TICK_NESTED_WIDE`/`TICK_MIRROR_WIDE`
  pinned at cure-earned constants. The mirror-narrow cells hold the
  memo's honest heap constant (an owned board red, the diff-coded
  memo's linear per-site word); the mirror-wide cross collapses
  under fill (the deep witness derives it), so its tick takes the
  fill branch.
- **descending-staircase**: all-narrow full-penetration
  min-updates — the family that falsified uncompressed I4; green on
  every work column, held to linearity by its cells.
- **memo_chain / memo_comb** (+ the shared-minimum flat control):
  the frame ledger's resolution cost, pinned flat ×2.00 across the
  doubling (re-pinned ≤ ×2.5 in the cure commit, never deleted).
- **memo_fanout / memo_oscillating / memo_churn /
  descending_raises**: the ledger's four guard adversaries —
  k-independence by absolute touch ceiling, the funding control,
  one live head through full-penetration drops, and the
  decide-then-emit ordering tripwire (verified live: an
  install-after-emit kernel fails its family_pairs differential).
- **reveal_comb / pure_comb / reveal_comb_hifloor**: the
  width-circulation genre — reveal-comb pinned flat ×2.00 across
  the joint doubling with an absolute band, pure-comb flat per byte
  with a band, the hifloor control green-banded flat and
  width-independent (each pin paired with its shape's closed-form
  tick and a one-touch-per-byte liveness floor).
- **ascend_cliff / ascend_cliff_plateau**: the undercut-cascade
  fold-direction genre — pinned flat ×2.00 across the joint
  doubling with an absolute band; the leveled control flat. The
  cliff pair's k-live-differences heap constant is an owned board
  red (the campaign doc's §17.3).

**The witness-axis convention (normative)**: an
undercut/propagation witness needs both a depth axis and a
residue-width axis before the genre is covered. The staircase
descends, so every residue it propagates is narrow (unit deltas —
it prices hop *count*, never hop *width*); the ascending cliff is
its mirror (narrow dying differences, wide surviving residue). One
axis per family is fine; the genre needs both axes present in the
committed set.

**Model-side committed schedules** (the spec's own tripwires, run
per design round, not in the kernel gate): the fourteen-schedule
set — the seven L1 schedules plus `descending-staircase` (I4's
tripwire), `wide-off-churn` (L2's tripwire), `run-boundary-churn`,
`resurrection-cycle`, `burst-arm-close`, `anchor-flip-funded`, and
`benign` — plus the seven round-6 attack schedules as permanent
axes, plus `anchor-flip-UNFUNDED` kept deliberately as the
demonstration that the pricing chain (L2×L6) is load-bearing: it
MUST read superlinear against input-only content; if it ever reads
flat, the model has stopped charging emissions and is broken — a
liveness tripwire for the model itself.

**The acceptance contract, met and standing for any future cure
round**: red pins land first (instruments before cures; measured
exponents in the commit message; the red-pin agent pins whatever
the meters read — its job is to hold the CURE to linearity);
green-pinned controls land with the red-pin commit, with liveness
floors derived from the honest walk's counts (meaningfully near the
measured constant, never the generic 1 bit/B); the cure flips the
pins at both scales with byte-identity against the recursive oracle
across every family, exhaustive small scope, arbitraries, organic
histories, and the deep closed-form witnesses; scan columns
unchanged on non-tick cells; heap within allowance (L5); roster
entries leave with measured linear exponents recorded; `fill.rs`'s
`# Cost` restates exactly what is then proven (per-dimension,
tagged); the campaign doc's §13/§17.3 restate the sums; envelope
rows re-pin at cure-earned constants; at-risk floors on green
families are measured FIRST, and a cure-removed-work breach re-pins
downward as a distinct outcome from a liveness failure. **No green,
no merge.** The cure's implementation re-runs L3's comparison-site
enumeration against its own final sites.

## 8. Decisions and their dispositions

Coordinator dispositions under the standing authorization of
2026-07-25, each realized in the tree; the package awaits Finch's
ratification read (the formal campaign's Phase 0 deliverable),
which lands here as a dated amendment:

1. **T-tick as #34's target**: adopted and realized — the full
   discipline validated composed on the limb-faithful model, then
   measured at the kernel on every committed family; the pinned
   cells are the arbiter.
2. **Fusion**: pre-approved by ruling given linearity with small
   constants; landed (§6, §9 round 8) as the separate post-cure
   commit, judged by the before/after table, no red pin.
3. **L6, the output-bound pin**: landed with the cure in its
   corrected multiplicative form (`≤ 2·size(e) + 4·size(i) + 32`;
   the additive form is refuted — §4's row); two-ways-computable
   gets a pin, and it protects the input denomination the board
   rests on.
4. **Segments ownership**: the ×4 recursion-depth residual stays
   with P4.2 (the iterative rewrite rides the watermark stack
   naturally); no cure silently changes the segments profile
   without re-pinning.
5. **Board mechanics for the tick crosses**: delegated to the
   red-pin agents; smoke-pin counts derived from what actually
   landed and re-verified there, never transcribed arithmetic.

## 9. Adversarial record (the design loop, compact)

Seven adversarial rounds plus the fusion landing; per round: the
refutation found, the cure, and the commits. Full harness
transcripts and number tables are in git history at the revisions
the round dates name. Model-tier numbers were reproduced by the
round citing them before being recorded (the header's provenance
note).

**Round 1** (2026-07-25, spec tier; attack on revision 1 —
FALSIFIED twice + one prose gap):
- **F1**: uncompressed residue propagation is Θ(open depth) per
  full-penetration min-update — descending-staircase Θ(d²)
  [measured-on-model], value-correct throughout (cost-only). Cure:
  the zero-run-compressed diff stack, linear on all seven
  schedules; I4 restated with the compression normative.
- **F2**: the fold-in/sign/fold-back-out comparison path is
  quadratic under wide offsets with cheap codes — `wide-off-churn`
  95 → 729 touches/unit on naive and compressed stacks alike.
  Cure: L2's per-operand lifetime discipline (rules 1–2).
- **F3**: the memo's diff-coding was under-specified (an
  implementor could faithfully rebuild the fan-out quadratic).
  Cure in prose: L4's normative diff-coding clause.
- Held: L1's value semantics (full-stack oracle), L3's enumeration,
  L5 (strengthened by both fixes), L6, the fusion's equivalence
  both directions. Advisories integrated: the first-leaf absolute
  flag case (§6), the route-fold × word-scale-skip interaction
  (§4), Accum pooling (§6), the model's sparse-wide undercount
  (resolved by round 2's limb-faithful model).

**Round 2** (2026-07-25, spec tier — HOLDS; convergence declared at
revision 3):
- L2 VALIDATED on the limb-faithful composed model (the discipline
  implemented, not charged by construction): all fourteen schedules
  linear; the tripwire reads 8.14/unit flat where the pre-discipline
  path read 95 → 729.
- Normative integrations: A1 the anchored-entry discipline (L2
  rule 3 — round 1's quadratic lived equally in the stack's own
  min-update comparison); A2 pricing-not-count for wide reads (L2
  rule 2 — anchor-switch bridges forced it); A3 the pricing chain
  as L2's closing lemma, with `anchor-flip-UNFUNDED` (61.7 → 444,
  ratio 3.96) kept as the model-liveness tripwire.
- Upgrades: the post-collapse slack bound to [derived,
  probe-confirmed] (worst 0 over 2000 + 548 constructions); the
  flag strengthened to the first-divergence-detector argument; I4's
  funded-cascade and no-split-churn clauses measured flat.

**Round 3** (2026-07-25, kernel tier — the first cure's review:
T-tick's realization REFUTED in the memo resolution):
- **F1**: the chain-interval memo resolution is Θ(k²) in touch
  currency [measured, kernel] — consumption order permutes
  recording order, so links are re-read once per crossing instead
  of dying at first read. Red-pinned: `memo_chain` ×3.94/doubling,
  `memo_comb` ×3.92, the shared-minimum control flat. Both charter
  cure shapes REFUTED ahead of implementation (previously-consumed
  anchoring telescopes only sibling chains, modeled ×3.97;
  literal-parent anchoring is unrecordable — the parent's minimum
  is not final at the child's close). No cure landed this round —
  no green, no merge.
- **A semantic bug found by the new families' first differential
  crossing** [measured, minimized, fixed]: the re-anchored relation
  follower installed AFTER a raise emission went stale by the arm's
  delta. Fixed (install before the emission); the pools now carry
  both families event- and id-side including cross-family pairs and
  a 4096-site closed-form witness. Lesson, standing: cost families
  and the semantic suite must cross.
- **F2**: the acceptance currency (touches) was watched nowhere on
  the tick surface. Fixed: the tick envelope rows moved onto the
  five-meter harness with touch floors.
- **F3, the orbit lemma** [measured, pinned]: iterated tick does
  not compound — `bits(tick^k) ≤ bits(tick^1) + 4·bits(id) +
  4·⌈log2(k+1)⌉ + 8` after the one-step ≤2× transient (§4's
  denominator row); pinned by
  `tick_orbit_growth_is_transient_plus_log` and
  `tick_deep_orbits_stay_banded`. Two conjecture clauses refuted en
  route, both harmless (re-firing replaces rather than
  accumulates).
- **F4/F6/F7**: L6 corrected to the multiplicative form
  (`≤ 2·size(e) + 4·size(i) + 32`, shrunk counterexample seed
  committed — a 175-bit event under a 6-bit id ticks to 255 bits);
  provenance corrected (the 8,192-case run restated as a one-off);
  `fill.rs # Cost` restated honestly.

**Round 4** (2026-07-25, kernel tier; the frame ledger, commits
`4934db86` + `952159f7`): the memo resolution cured — one link per
site, sibling-chained within each level, first-child links deferred
to the forest parent's close (where its minimum is final), zero
links unstored, one live min-relative head with outer levels
suspended as immutable value diffs, the recording-order queue
written out of order and consumed in order. `memo_chain`/`memo_comb`
flipped ×2.00 exactly (re-pinned ≤ ×2.5, never deleted); the four
guard families landed (`memo_fanout`, `memo_oscillating`,
`memo_churn`, `descending_raises` — the last verified live);
byte-identity across the suite; heap parity. Two in-flight catches
by the committed instruments (the round's own evidence the pins
work): a rule-1 fold-direction violation (×18 touches on the
mirror-wide envelope row) and a per-resolve buffer mint (~51 B/site)
— both cured before landing. The pin harness re-denominated against
the version's own stored stream (construction-language absolute
codes overstate a plateau family's input by orders of magnitude).

**Round 5** (2026-07-25, kernel tier; the ledger cure's adversarial
review — T-tick REFUTED at the kernel, semantics exact, cost-only):
- **The family**: `reveal_comb(k, b)` — k sibling left-full sites
  sharing one wide minimum `W = 2^b` over a 0-floor, the
  left-leaning spine closing each site's frame back into the floor
  frame between consecutive consumes. Θ(k·b) touches on Θ(k + b)
  input AND output (738,449 → 2,884,881 across the joint doubling,
  ×3.91 on ×2.00 input) — a true amplifier under the spec's own
  denominator. Red-pinned ≥ ×3.5 with the instruments landed at the
  record (generators oracle-verified first, gate pins, board rows,
  the closed-form deep witness).
- **The mechanism**: an unfunded width-circulation cycle — the
  consume decision mints a width-b boundary diff, the site's close
  pops it back into the base stack and the relation follower, the
  next consume re-mints it; every object individually
  create-once/read-once/die, with NO input delta, NO output code,
  and NO undercut descent funding any hop. I4's funded-cascade
  clause enumerated undercut hops only; L1/I4 reopened at the spec
  tier.
- **Attribution, both layers pinned**: `pure_comb` (no site, no
  memo) pays ~2 wide folds per site in the base watermark stack —
  the defect predated the ledger, whose follower ferry amplified it
  ~10× (~21 wide folds/site). The `reveal_comb_hifloor` control
  (identical forest and cycle, consume-time gap 2) is flat and
  width-independent: the wide GAP is the driver. The unmodified
  round-2 model reproduced the class at the base layer's constant.

**Round 6** (2026-07-26, kernel tier; the latent boundary register,
commits `43d625e7` + `12a85d2f`): design → attack → fix → landing.
The cure: closes move the popped boundary into a per-stack latent
register instead of folding it (the follower ferry is deleted); arms
recycle the register at the narrow anchor-relative offset; undercut
decisions go by `sign_dominates_at` domination with funded collapse
only at comparable scales; followers carry a per-slot σ tag resolved
at their own deaths. **I4 subsumed by I4′ (width conservation); L1's
cost clause restated as L1′.** The attack round sustained the
mechanism across six schedules and returned four accounting/
validation findings, all fixed pre-landing (the two-directional
acceptance argument — at-risk floors measured FIRST, re-pin-downward
distinct from liveness failure; the web-death hop added to the
enumeration, whose re-derivation also fixed an unexercised σ-path
value defect in the model; three model–prose divergences; the value
read pricing totalized). The pins flipped: reveal-comb
×3.91 → ×2.00 exactly (re-pinned ≤ ×2.5 + absolute band), pure-comb
per-byte falling (re-pinned ≤ ×1.15 + band); no at-risk floor
breached (hifloor re-pinned downward to the cured measurement under
the ratchet); byte-identity across the suite; board sums unchanged.
Kernel realization deviations, each dated and value-equal: the
web-death hop is vacuous kernel-side (followers die first,
asserted); the height-to-watermark bridge retires the latent
unconditionally at emit-at-minimum (cost ≤ the model's on every
path); the undercut's follower fix folds the anchor-relative drop
alone. **One kernel–prose divergence disclosed, not fixed** —
`propagate` folded the residue into each popped diff where I4′
rule 2 demands the dying side's digits; costs equal on every
committed family, Θ(k·W) vs Θ(k + W) on a then-uncommitted shape —
recorded as a named candidate for the next attack round rather than
silently absorbed. T-tick restored: the theorem under I4′.

**Round 7** (2026-07-26, kernel tier; the fold-direction cure —
red pin `8fdba3d4`..`b0136781`, cure `b31ca059`, flip `5f40eec3`):
round 6's disclosed divergence was reachable after all — the
adversarial review of the landing constructed `ascend_cliff(k, b)`
(k ascending wide leaves stacking k − 1 nonzero unit boundary
differences under one wide terminal undercut): Θ(k·b) touches on
Θ(k + b) I/O, 203,435 → 790,851 across the joint doubling (×3.89),
and the same defect a heap amplifier (every popped difference's
buffer widened to residue width, board heap e 1.82). Round 6's
closing claim ("no kernel-expressible schedule read superlinear")
was bounded by the committed schedule set and is corrected by this
record. The cure inverts the hop to I4′ rule 2: top-index
domination decides each hop's direction in O(1) before any fold
(the dominated difference dies by its one fold into the surviving
residue; a dominating difference absorbs the dying residue's single
terminal fold; only comparable scales run fold-then-sign, priced by
the near-cancellation; width guards skip undecidable domination
reads for free). Flip: ×2.00 exactly, re-pinned ≤ ×2.5 + band
(31,542/18,925); heap e 1.82 → 1.00; every other committed MEASURED
reading byte-identical across the cure; the leveled control flat
and byte-identical (the zero-run cascade never enters the changed
arms). The witness-axis convention recorded (§7). The negative-
residue model sketch was NOT needed — the kernel realization is
cheaper, value-equal (a dated deviation-or-confirmation in round
6's genre).

**Round 8** (2026-07-26, the fusion landing, commit `80131954`;
flag width pin `74d6ec5e`): §6's shape landed as one bisectable
commit under the two owner rulings recorded there. Differentials
all green on first run: `tick` ≡ the oracle's `event`, flag ≡ (the
oracle's `fill` moved the tree), over the family grids, exhaustive
small scope, arbitrary pairs, organic histories, the flag's worked
corner cases, and the deep closed-form witnesses re-denominated in
flag terms; the grow suite decides the branch per pair and pins its
coverage exactly — 182 grow-branch family pairs and 114,621
exhaustive pairs (tamper-evident against a regression rerouting
pairs to the fill branch) — holding each to the oracle's inflation,
the brute-force minimal inflation, and a reference recursive probe
whose route must equal the walk's bit for bit. Deleted with the
composition: the iterative topology-only probe and its bit-coded
frame machinery, the ev-keyed route block, the standalone
fill/grow entry points, and the probe's envelope rows (the two
reachable grow scenarios re-target the full fused tick as
`tick_expand_spine`/`tick_expand_cross`, fresh five-meter
baselines, the grow-only measurements of record kept in the
annotation). The before/after table (version_tick board rows,
default scale, per-byte constants; the record scale moves
identically; artifacts `board-fusion-{lo,hi}.txt` against
`board-joinallcure-{lo,hi}.txt`) — the landed baseline P4.2's
word-scale-skip sequencing measures against:

  | family | branch | limb/B | scan/B | touch/B | heap/B |
  |---|---|---|---|---|---|
  | benign | fill | 4.3 = | 8.0 = | 5.2 = | byte-identical |
  | dense | fill | 5.3 = | 8.0 = | 2.7 = | 0.2 → 0.0 |
  | nested-wide | fill | 5.4 = | 14.2 = | 8.1 = | byte-identical |
  | mirror-wide | fill | 8.9 = | 28.4 = | 24.3 = | e 0.94 → 0.91 |
  | reveal-comb | fill | 8.0 = | 22.5 → 22.9 | 17.8 = | 0.3 → 0.0 |
  | nested-full | grow | 9.1 → 6.9 | 37.7 → 24.0 | 10.3 → 8.0 | 0.3 → 0.0 |
  | staircase | grow | 11.4 → 4.6 | 40.0 → 24.0 | 12.6 → 9.1 | 0.5 → 0.0 |
  | hugeleaf | grow | 0.4 → 0.3 | 48.0 → 32.0 | 0.6 → 0.5 | 4.7 → 2.6 |
  | pure-comb | grow | 4.4 → 1.5 | 37.1 → 22.5 | 5.2 → 3.7 | = |
  | ascend-cliff | grow | 4.6 → 1.6 | 41.4 → 25.4 | 8.9 → 7.4 | 64.4 → 63.0 (owned red stays) |
  | ascend-plateau | grow | 5.6 → 2.0 | 41.8 → 25.8 | 5.6 → 3.8 | 0.3 → 0.0 |

By mechanism: the grow-branch scan drops are the eliminated probe
pass and byte compare (4 traversals → 2); the touch/limb drops are
copy-on-first-divergence (fill's discarded output never built); the
heap drops on BOTH branches are the deferred output buffer (the
verbatim-reference mode keeps the builder and the collapse scan's
transient from coexisting at peak). Board: 51 of the 83 tick-carrying rows moved per scale with
zero status flips; grow-branch touch bands re-pinned downward under
the at-risk-floor protocol in the fusion commit; one committed
tripwire reshaped with its seed (`grow_bushy_is_linear` — the
deleted probe pass had added a deterministic ~n floor that made a
two-point step budget stable; the pin now drives `(bushy(scale), 1)`
so the terminal pins the route scale-independent); the splice's
equal-sibling collapse at the inflation point is fill-preempted on
every pair the fused tick can route to it (reachability derived,
the grow suite's worked examples carry the reachable genres).

**Per-lemma status (current)**:

| clause | status |
|---|---|
| L0 scan | [measured, landed] |
| L1/L1′ watermark stack + latent boundary | [measured at kernel]: linear on every committed family, both undercut-cascade axes and the close-reveal genre |
| L2 emission pricing | [measured-on-model], composed and limb-faithful; its kernel realizations pinned by the tick cells |
| L3 sign-read decisions | [derived]; enumeration maintained at each landing |
| L4 pre-scan/memo | [measured at kernel]: the frame ledger, linear on the consumption-order families |
| L5 auxiliary space | [derived + measured]: heap parity; two owned linear constants board-red with named candidates |
| L6 output bound | [measured, pinned, multiplicative]; load-bearing via the pricing chain |
| the orbit lemma | [measured, pinned] |
| fusion | LANDED [measured at kernel, both scales, zero status flips] |
| **T-tick** | **the theorem under I4′, [measured at kernel] on every committed family, realized inside the fused tick** |
