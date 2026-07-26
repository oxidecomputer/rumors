# The tick/fill cost specification

Status: the statement of record for #34 (the tick limb cure and
fusion), committed 2026-07-25 at **revision 3** — converged after two
adversarial attack/fix rounds, then **reopened at the kernel tier at
round 3** (the record is §9): the landed realization's memo
resolution is refuted in cost (red-pinned) and both candidate cure
shapes are refuted ahead of implementation, so the memo discipline
returns to the design loop with the position-anchored seed; the
walk, the watermark web, L6 (corrected multiplicative), and the
orbit lemma stand pinned. Under the user's standing authorization of
2026-07-25 (fused tick pre-approved given
linearity with small constants; confer only on a superlinear honest
optimum, a §6 denomination change, or a representation-forced
redesign). Formal ratification by Finch lands as a dated amendment
here. Revision 3 (fix round two, 2026-07-25) integrates attack round
two's amendments — the adversarial record is
§9, which now carries the convergence assessment: **converged at the
spec tier; the residual obligations are implementation-tier and
already normative in §7**.
*Amendment 2026-07-25 (round 5, the reveal-comb refutation)*:
**T-tick is REFUTED-pending-revision at the kernel tier** — the
frame-ledger cure's adversarial review found an unfunded
width-circulation cycle (arm-up/close-reveal hops, outside I4's
funded-cascade enumeration); L1/I4 reopen at the spec tier, the
family is red-pinned in the gate, and the model tier reproduces it.
The record is §9 round 5.
Spec-first: this is the English statement of record the cure
implements against; deviations get dated amendments.
Statement-faithfulness governs every claim below — never weaker than
stated, never stronger than proven; each clause carries its epistemic
tag: **[measured]** (instrumented run, source named),
**[measured-on-model]** (the executable discipline model, not the
kernel), **[derived]** (argument from code or arithmetic),
**[open]** (known unknown; must be resolved or the claim weakened
before it is committed as prose).

Evidence artifacts: probes at `/tmp/fillprobe-spec` (Rust: `widedeep`,
`widedeep_mirror`, both release + debug-assertions, deterministic
meters; Python: `watermark_model.py`, oracle-asserted); attack round
one's harness at `/tmp/fillprobe-attack/attack_model.py` (the model
copy-extended with a full-stack value oracle, three new schedules,
and the zero-run-compressed stack variant); fix round one's
reproductions at `/tmp/fillprobe-fix1` (the attack harness re-run —
every transcribed number reproduced exactly — plus `slack_probe.py`,
the independently rewritten post-collapse slack probe: the attack's
own slack run was not persisted, so its bound was re-established
fresh before being cited); attack round two's harness at
`/tmp/fillprobe-attack2` (`emit_model.py` — the composed discipline,
compressed stack + anchored emissions + tagged `last` + faithful
materialization, under a full-stack value oracle extended with
per-emission `d_out` exactness and the `last` invariant, with
LIMB-FAITHFUL costs mirrored against `codec/accum.rs`'s touch
placement; `slack_probe2.py`, structured cancellations); fix round
two's reproductions at `/tmp/fillprobe-fix2` (every round-two number
reproduced exactly; the `run-boundary-churn` schedule — defined but
not wired into the committed harness's schedule list or oracle loop —
was oracle-validated and re-measured there before being cited, and
the four cost-placement claims were confirmed by line-read of
`accum.rs`: `apply_limbs` touches every limb including zeros,
`add_accum_shl` every held digit, `sign` per scanned digit plus
collapse, `sign_magnitude` per held digit plus the complement pass).
The review's probes at `/tmp/fillprobe` are independent reference;
every number in this document was reproduced by the round that cites
it.

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
`event`); the skyline kernels are byte-identical to it under the
differential suite, and `norm` is realized by the collapsing builder's
equal-sibling normalization plus min-sinking, never as a separate
pass. **Fill and grow never run separately**: every public entry is
`tick = fill, falling back to grow iff fill changed nothing`, decided
today by byte-comparing `fill`'s output against the input (sound
because canonical coding is unique).

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
emitted when the decision is made — the landed kernel's deferral gets
it in-pass. The left-full arm's min (`min(e′r)`) is over a range the
walk has *not yet reached* when the raised leaf must be emitted —
hence the (memoized) pre-scan. Both needs are LIFO-nested range
extrema over one preorder stream.

## 2. The refutation this spec answers

The landed kernel (92d2fc31) returns per-subtree `SubtreeOut =
(min, net)` as **materialized `Base` magnitudes**, combined by
`signed_sum_base` at every paired node — Θ(width) limb work per
ancestor. Depth × width is not bounded by input bits.

**[measured]** (`/tmp/fillprobe-spec`, this draft's runs, doubling
b = d):

- Right-full chain, `tick(bigroot(b,d) × nested_full_id(d))`: limb
  step exponent 1.57 → 1.71 → 1.83 (→ 2), constant 29.4 → 126.3
  ops/B, crossing the board's 128/B ceiling at b = d = 8000. Scan
  flat 14.2 bits/B, e 1.00 at every step.
- **Left-full (memoized pre-scan) chain** — not covered by the
  review's finding — `tick` of the mirror id `(1, (1, … (1, 0))))`
  over a right-leaning spine with one wide bottom leaf: limb e
  1.92 → 1.94 → 1.97, constant **143 → 1015 ops/B** — over the
  ceiling already at d = 1000, steeper than the right-full arm.
  `min_fill_rec`'s own `l_net`/`r_net` summation chains carry the
  same genre. Scan flat 28.4 bits/B, e 1.00.

Both arms are public-API-reachable through `Version::tick`/
`Clock::tick`. Consistent with §3's finding entry: the multiplier is
the *local* id's paired depth — a pricing obligation under the §6
invariant, not a hostile-peer exploit.

## 3. The candidate theorem and its decomposition

> **T-tick (candidate, the ratification target).** `tick` (hence
> `fill` and `grow`) is computable over skyline streams in amortized
> **O(n + m) Accum digit touches** in the two packed operands' bits,
> with no bound on value magnitude, tree depth, or encoded size —
> alongside the already-achieved O(n + m) scan bits.

*Amendment 2026-07-25 (round 5)*: **T-tick is
REFUTED-pending-revision at the kernel tier** — the reveal-comb
family measures Θ(k·b) touches on a Θ(k + b) input whose output is
Θ(k + b) too, through an unfunded width-circulation cycle I4's
funded-cascade clause does not enumerate; L1/I4 reopen at the spec
tier, and the model tier reproduces the family (§9 round 5, the
record of record). The status paragraph below describes the
document as of round 2 and is superseded where §9's later rounds
say so.

Status: **[validated-on-model, kernel-pending]** — every lemma is
[measured], [measured-on-model], or [derived] with no load-bearing
[open] clause remaining after attack round two; what remains is
implementation-tier and normative in §7 (the red pin's mandatory
kernel re-measurement; the cure's re-enumeration of its own
comparison sites). The reduction: every signed quantity fill needs
lives in **one shared anchor web** — the running height `h`, and
per-open-range extremum watermarks represented as *differences*,
never absolutes.

**Representation compliance (user ruling 2026-07-25)**: everything
in this candidate — the watermark web, the diff coding, the residue
propagation, the fusion, the copy-on-first-divergence builder — is
**auxiliary in-memory walk state over the unchanged skyline coded
form**. The at-rest coding, byte equality as `Eq`/`Hash`, and the
identity/persistence story are untouched; no clause of the
derivation requires or pushes toward a third representation. The
ruling's confer-level escalation (§5) is recorded for completeness,
not because any current evidence points there.

**The discipline's invariants** (the cure implements these; each
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
  stack** (restated at fix round one; the uncompressed form is
  FALSIFIED, §9/F1): the diff stack stores zero-diff *runs* as one
  O(1) entry and nonzero diffs individually. A min-update reaching k
  outward frames folds the *dying nonzero* diffs into the running
  residue (each nonzero diff dies at most once after creation),
  passes whole zero runs in O(1) each — their minima track the
  innermost watermark implicitly — and makes exactly one surviving
  fold at the stopping frame, bounded by the update's own priced
  width. Never fold the residue into surviving frames. The ledger
  clause is per-object, and wide *content* may nonetheless cascade
  through a chain of deaths — a dying payload folds into the residue,
  which survives as a new diff that can die again — **provided every
  hop is separately funded**: a full-penetration undercut reaching a
  wide diff requires the running height to have descended past that
  diff's span, which the input paid in delta codes (attack round
  two's `resurrection-cycle` schedule measures funded hop chains
  flat, 5.05/unit; partial penetrations never reach the wide diff).
  Zero runs cannot absorb a shortfall, so propagation never stops
  inside a run and runs only grow or shrink at their ends — no split
  operation exists to churn (round two's `run-boundary-churn`,
  6.23/unit flat). Without the compression, zero-diff frames cost
  ≥ 2 touches each and never absorb, and a full-penetration update
  pays Θ(open depth) — the descending-staircase family is Θ(d²)
  **[measured]** (naive 8002 touches/unit at d = 8000, ratio
  4.00/doubling; compressed 7.00 flat, ratio 2.00 — attack round
  one's harness, re-run and reproduced at fix round one; 13.00 flat
  under round two's limb-faithful accounting, class unchanged).

**L0 (scan) [measured, landed]**: O(n + m) scan bits — every position
read at most twice (walk + at most one memoized pre-scan; flat ×2
absent-sibling scans). e 1.00 on every family, both arms (§2's runs).

**L1 (the watermark stack) [measured-on-model]**: a LIFO stack of
range-min watermarks over one running height, coded as
`T = below(innermost)` plus outward nonneg diffs
(`below = h − min`, over emitted values) **on the zero-run-compressed
stack (I4 as restated — the compression is normative, not an
optimization)**, supports open (move), delta (one fold into T —
uniform shift), emit (arm / compare / residue-propagate), and close
(one sign read + one dying-diff fold or run decrement) in amortized
O(1) digit touches plus O(width) charged to dying operands.
Evidence: attack round one's harness (`attack_model.py`,
copy-extending `watermark_model.py`: same faithful balanced
signed-digit semantics — lazy zone, recenter carries, |s| ≥ 3
domination sign fold with collapse — under a **full-stack** value
oracle asserting every armed frame's watermark at every step
including closes). Linear on all **seven** adversarial schedules —
flat 3.20–10.38 touches/unit, ratio 2.00 per doubling at
d = 1k → 8k: the four originals (`chain-narrow`, `bigroot-chain`,
`wide-dip` — the shared-wide-min cascade — and `cliff-churn`) plus
attack round one's three (`descending-staircase`: every leaf
undercuts every open ancestor, the family that falsified the
uncompressed form; `sawtooth-partial`: repeated k-of-d penetration;
`rearming-churn`: sibling open/emit/close churn at depth). Numbers
reproduced at fix round one. Re-validated at attack round two under
limb-faithful cost accounting, composed with the L2 emission
discipline (fourteen schedules, `emit_model.py`): constants
~1.6–2× round one's — faithful accounting plus real materialization
at every emission — classes unchanged everywhere (6.2–19.1
touches/unit, ratio 2.00 per doubling; reproduced at fix round two).
Model limitations (normative for later rounds): (i) round one's
model's `add_int` skipped zero digits while the real `apply_limbs`
touches every limb including zeros, so its sparse-wide folds
undercounted Θ(width) — **resolved at round two**: `emit_model.py`
is limb-faithful (per-limb touches including zeros, per-held-digit
`add_accum`, per-scanned-digit `sign` plus collapse, per-held-digit
`sign_magnitude` plus the complement pass — each confirmed against
`accum.rs` by line-read at fix round two), and all L1/L2 conclusions
now rest on the faithful runs; (ii) the oracle asserts *values*
(every armed frame's watermark at every step, `d_out` exactness,
the `last` invariant); costs are validated by structural mirroring
against `codec/accum.rs`'s touch placement, not by differential
against the Rust meter — kernel-side re-measurement at the red pin
remains mandatory (§7).

**L2 (emission pricing) [measured-on-model — VALIDATED at attack
round two on the limb-faithful composed model]**: every materialized
quantity at an emission point is O(the emitted code's width + its
consuming scan's own disjoint range content). The draft's original
charge ("by construction") was refuted at attack round one: the
fold-in/sign/fold-*back-out* comparison path is quadratic under wide
offsets with cheap codes — `wide-off-churn` (one armed range, d
emissions each carrying a dense-wide offset, every output code after
the first O(1) bits) measures 95 → 729 touches/unit, ratio → 3.99,
on the naive AND compressed stacks alike (§9/F2; the compression is
orthogonal to this failure). The discipline of record replacing it —
implemented and validated as a composed model at round two
(`emit_model.py`: **the committed tripwire reads 8.14 touches/unit
flat, ratio 2.00 per doubling**, on the discipline that read
95 → 729 without it; all fourteen schedules linear; reproduced at
fix round two):

1. **Never fold-and-restore a wide operand**: a comparison folds a
   *dying* operand into the survivor and reads one sign; no wide
   operand is ever folded in only to be folded back out. Where a
   fold-and-restore is unavoidable at comparable scales, **only ever
   the NARROW (priced) side** is folded and restored (§9/round 2,
   A1).
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
   read (§9/round 2, A2; the funded-flip schedule measures the
   constant, 3.5–3.9 touches/unit flat). The wide-off family under
   this rule: consecutive raise targets differ by cheap priced
   deltas, so the offset rides ONE live accumulator maintained
   incrementally (`last = value_prev − anchor`, folding input deltas
   between emissions — one more I1 citizen), never k fresh wide
   operands. `last` is **anchor-tagged**: h-relative between
   pass-throughs (folds input deltas), watermark-relative between
   raises (drift-invariant, adjusted only at priced min-movements
   and at funded anchor deaths); its anchor generalizes to *the
   accumulator that sourced the last emission* — a consumed memo
   entry (L4) staying live on the walk's LIFO stack anchors it the
   same way, and its death re-anchors `last` by one funded fold,
   the same mechanics as a watermark-anchor death (§9/round 2).
3. **The anchored-entry discipline** (§9/round 2, A1 — round one's
   quadratic lived equally in the watermark stack's own min-update
   comparison, and rules 1–2 alone leave that path expressible; an
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
   digits over 2000 randomized cancelling constructions (fix round
   one, `slack_probe.py`) and over 548 structured adversarial
   constructions — deep cancelling prefixes to 4096 bits,
   multi-scale alternations, telescoping staircases, collapse/
   rebuild cycles, near-threshold partials (round two,
   `slack_probe2.py`; reproduced at fix round two). So every
   materialization is O(the emitted code's width). The emitted
   output delta is `d_out`: fold the dying `last` in (plus the `t`
   bridge on an anchor switch, per rule 2c), collapse, materialize
   — priced by the code.
5. **The gap accumulator survives** by folding the materialized
   `d_out` back in — O(code width), priced by the output the code
   is.

**The pricing chain (normative lemma; §9/round 2, A3)**: L2's
per-emission costs are priced by the **emitted code's width**; Σ
emitted code widths ≤ input content + O(id) by **L6**; hence L2's
total limb cost is O(n + m) **input-denominated**. This sentence is
load-bearing, not decorative: the `anchor-flip-UNFUNDED` schedule
measures 61.7 → 444 touches/unit (ratio 3.96) when the denominator
is denied the output-pricing — superlinear against input-only
content — and that reading is NOT a kernel counterexample precisely
and only because of this chain: for a bridge to be wide, the
tracked min must sit far below the running height; an h-anchored
emission is then genuinely far from the previous value, so `d_out`
is wide, its output code is wide, and L6 telescopes the output back
to input content (emitted values are input-derived extrema, and
nested range mins are monotone — outer ≤ inner — so a raise chain's
total movement telescopes against the content that created the
spread). T-tick's proof outline reads: L1 (stack maintenance,
amortized O(1) per event) + L2 (emissions priced by output codes) +
the pricing chain (output ≤ input + O(id), L6) ⟹ amortized
O(n + m). If any link fails at kernel measurement, §5 applies.

**L3 (decisions are sign reads) [derived; enumeration verified at
the current tip]**: `signed_min`/`signed_max`/the equal-sibling test
need orderings only; under I2 every comparison is `sign()` of a
same-anchor difference, amortized O(1) by the Accum's domination
bound + collapse amortization (`codec::accum` module doc). Attack
round one enumerated every comparison site at the current tip and
found none uncovered: the four `signed_min`/`signed_max` join sites
and both `signed_sum_base` re-anchoring chains in `fill.rs` dissolve
under the h-anchored stack (no per-subtree returns exist in the
target design); the raise comparisons are fold-on-death (L2's rule
1); emission sites are L2's discipline; `min_fill_rec`'s joins are
the pre-scan's own stack instance (L4); the builder's equal-sibling
seam (`build.rs`) is a zero-delta *code* check — one bit pattern,
O(1), no magnitude comparison. L2's top-index domination guard
(rule 3) is itself a comparison-site rule and joins this coverage
list: an armed comparison at disparate scales decides post-sign in
O(1), never by folding the wide side. The obligation that survives:
the cure's implementation re-runs this enumeration against its OWN
final comparison sites (the enumeration is of today's code; the
rewrite must not mint an uncovered site).

**L4 (the pre-scan inherits the discipline) [derived]**:
`min_fill_rec` has the same walk shape (LIFO ranges over the same
stream), so the same watermark web applies (**[measured]** need:
§2's mirror probe shows today's materialized form is the *worse*
half of the quadratic). Two clauses made normative at fix round one:

- **The anchor seam is sound as-is** (verified at attack round one
  by inspection of the landed kernel): a memo entry is recorded at
  stream position `l_end` relative to the height there and consumed
  by the walk at the same position, where the walk's height equals
  it — height is a function of stream position; no re-anchoring
  arithmetic exists or is needed at consumption.
- **Memo entries are diff-coded against the site forest's own
  final minima** (realization dated 2026-07-25, round 4: sibling
  links at their closes, first-child links deferred to the forest
  parent's close where its minimum is final, the scan-entry height
  as the outermost base; zero links unstored; one live head plus
  immutable suspended diffs carry the recording state, and the
  walk needs no carry at all — its live relation IS each link's
  reference at arrival, re-anchored from its own web at every
  site close) (§9/F3 — without this sentence an implementor
  can faithfully rebuild the quadratic: k nested left-full sites
  sharing one wide minimum would materialize k wide entries at
  creation, Θ(k·W/64) inside the pre-scan). Consecutive nested
  entries differ by cheap priced content; at consumption an outer
  entry's accumulator stays live while the walk is inside its range,
  so inner entries resolve as diffs against it — the ranges nest, so
  LIFO holds, and L2's per-operand lifetime accounting then covers
  the memo. Held under mixed-nesting attack at round two: the seam
  is sound by projection — entries are recorded and consumed at the
  same stream positions (same heights), consumption order is stream
  order, memo ranges nest within walk ranges, and a projected
  subsequence of a LIFO nesting is LIFO; two independent fresh-scan
  chains cover disjoint ranges with independently funded heads, so
  no cross-chain diff is ever needed. A live consumed entry may
  anchor `last` (L2 rule 2's generalized anchor); its death
  re-anchors by one funded fold.

**L5 (auxiliary space) [derived]**: heap = O(paired depth) frames
plus total live Accum digits ≤ O(digits ever placed) = O(content
folded) = O(n + m); the memo holds ≤ one entry per left-full site,
each priced by its disjoint fresh-scan range. Held under attack at
round one, and *strengthened* by its fixes: zero-run compression
only removes stack entries, and L4's diff-coded memo removes the
fan-out threat (one wide value shared by k entries no longer
multiplies into k wide materializations). Held again at round two
under the composed discipline: entries ≤ armed frames, run
descriptors O(1) each, and per-emission `d_out` accumulators are
created and dropped at ≤ value width + 2 digits (L2 rule 4's slack
bound).

## 4. Per-dimension cost model (what the board holds tick to)

| dimension | target profile | status |
|---|---|---|
| scan bits | O(n + m), constant ≤ 2 reads/position + flat ×2 sibling scans | **[measured]** landed, e 1.00 both arms |
| limb ops | **T-tick: amortized O(n + m)** | **[validated-on-model, kernel-pending]**: L1 + L2 validated composed on the limb-faithful model, fourteen schedules (compressed stack and anchored-entry/lifetime disciplines normative); the pricing chain (L2×L6) stated; kernel re-measurement mandatory at the red pin; current kernel quadratic both arms **[measured]** |
| heap | O(depth) frames + O(n + m) total digits; builder output | **[derived]** L5, pinned by existing heap columns |
| segments | today O(paired depth) recursion (red-pinned at ×4, owner **P4.2**); eventual profile **O(1) grown segments** via explicit stacks — the watermark stack and the grow probe's bit-coded frames are the natural vehicle, so P4.2 implements against this line. Sequencing note (attack round one, A2): on the fused walk, the route-DP fold reads skipped id subtrees per-bit on leaf-under-internal-id arms — P4.2's §11.4 word-scale skip on those arms interacts with the fused path; P4.2's sequencing decision must name the fused walk, and the before/after table judges the interaction | **[measured]** red today; target [derived] |
| denominator | **input-denominated stands** (n + m packed bits; §6's do-not-re-denominate list). Supporting lemma L6 **[measured, pinned; the additive form is REFUTED]**: `size(tick(e, i)) ≤ 2·size(e) + 4·size(i) + 32` (`tick_output_is_input_bounded`, committed with its shrunk counterexample seeds — grow's zero leaf and a raise's landing can each re-code one delta against a wide neighbor, duplicating one input code's width once, so no additive slack survives; the honest constant is the factor 2, realized at 1.5 by construction). Fill's output deltas otherwise telescope input deltas; grow adds one increment or one expansion chain ≤ O(m). The pricing chain carries the constant: Σ emitted ≤ 2·input + O(id), still input-denominated. The ORBIT is separately pinned (the round-3 lemma below): the factor cannot compound along `tick^k`. | |

## 5. If T-tick fails

No committed fallback currently satisfies the asymptotic bar: the
honest worst case of the landed kernel is O(n·m/64) limb ops
(depth × width), which is quadratic in total input — **below the
bar**. There is no known intermediate (an O((n+m)·log) shape has no
candidate mechanism here — the problem is re-touching, not sorting).
So the spec's position: T-tick is the target; the model tier is
validated (round two), so the remaining refutation surface is the
kernel — if any lemma's realization in implementation refutes (the
red pin's re-measurement, the cure's own comparison sites), the
finding comes back to this document with the counterexample family,
and the decision is Finch's, made against the recorded evidence —
not silently absorbed into prose.

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
   Finch's decision. No current evidence points there: every
   identified obstruction (L2's gap entanglement, L3's comparison
   sites) is walk-structural, not coding-structural.

## 6. The fusion assessment (fill + grow → one tick pass)

Today's tick on the **grow branch** traverses the event stream ~4×:
fill's walk (+ builder emission), the byte compare, grow's
topology-only probe, the splice emit. The grow branch is the common
one for the hottest real pattern — a peer repeatedly ticking its own
version (after the first tick, fill usually changes nothing). Fill
and grow are never observable separately (§1), so fusing inside
`tick` breaks no API.

**Fused shape [derived]**: one walk that (a) emits fill's output
through the builder, (b) maintains a **changed flag** — true iff any
emitted plateau (topology or code) differs from the input range it
replaces, compared in-pass against the input cursor — and (c)
piggybacks grow's route DP (the probe's lexicographic
(expansions, depth) cost fold is topology-only and visits exactly
the nodes fill's walk visits; the bit-coded frame stack already
exists). If the flag says changed: fill's output stands. Else: the
splice emit runs from the recorded route (verbatim-copy machinery
landed at #14/#16). Grow branch: 4 traversals → 2. Fill branch: one
walk, plus dead route bookkeeping (a few bits/node — the probe's
frames are 3 control bits + value deltas).

**Refinement the fusion unlocks — copy-on-first-divergence
[derived]**: until the first divergent emission, fill's output is
byte-identical to the input prefix, so the fused builder can run as
a verbatim reference (no emission work, no allocation) and
materialize only at the first divergence — one wholesale prefix
copy, priced once — exactly the `SkylineBuilder` verbatim-
continuation machinery (#14). On the grow branch the flag never
trips, so fill's discarded output is **never built at all**: the
repeated-local-tick hot path becomes one flag+route walk plus one
splice, with near-zero transient heap. This is the strongest
constants case for fusion and is unreachable by the unfused
composition (which must always build fill's output to byte-compare
it).

**Equivalence obligation [derived, held under attack at rounds one
and two, must be pinned]**: `changed flag ≡ (fill(e) ≠ e)`. With the
flag defined as *emitted-differs-from-input* (never "an arm fired" —
a raise can reproduce the existing leaf value exactly, and
`max(L, …) ≥ L` means value-unchanged raises are real), the
equivalence is structural: canonical uniqueness makes byte
inequality ⟺ some plateau differs. Attack round one pressed both
directions and held: a value-reproducing raise does not trip the
flag (correct — `fill(1, leaf) = leaf`), and every collapse arm over
a non-leaf range is a topology divergence (trips, correct). Round
two *strengthened* the alignment story: the flag's comparison only
needs to run while every emitted plateau equals its input range,
during which output position ≡ input position exactly; at the first
divergence the flag trips and the comparison stops mattering —
alignment drift after a collapse is structurally impossible to
observe. **The flag is precisely a first-divergence detector**,
which is also why copy-on-first-divergence composes with it for
free. One special case is normative (round one, A1): **the FIRST
emitted leaf is coded absolute while later leaves are deltas — the
flag's comparison at the first-leaf site compares the absolute code
against the input's absolute code, never a delta against an
absolute**; the comparison alignment is by plateau (depth, code), so
a collapse shifting which input leaf is "first" already trips the
flag on topology before any code comparison is reached. Pin it totally: a
differential asserting flag ≡ byte-compare across every family +
arbitraries, and — strongest and simplest — fused `tick`
byte-identical to the composed `fill`/compare/`grow` path (which
stays in-tree as the oracle-facing composition for exactly this
differential). Route-data reachability also held: Full-id-over-node
is unreachable on the grow branch (fill would have changed the
tree), Full-over-leaf routes trivially, and the
leaf-under-internal-id arm computes its route fold during the id
`skip` the walk already pays.

**Costs and risks**: route bookkeeping taxes the fill branch (small,
measurable — the before/after table judges it under the parity
floor); on leaf-under-internal-id arms the route fold reads the
skipped id subtree per-bit, which degrades P4.2's word-scale skip on
exactly those arms if that lands later — the interaction is recorded
in §4's segments row and P4.2's sequencing decision must name the
fused walk (round one, A2). Benign-input constants (round one, A3):
the watermark stack's per-range state must be **pool-reused** — an
`Accum::new()` per range open is an allocation per node; today's
kernel allocates comparably, but the parity floor judges the
difference, so pooling is named in the cure charter, not left to
taste. The changed-flag equivalence is correctness-critical (pinned
as above; keep the byte compare as a debug_assert through at least
one release cycle). Legibility is an obligation on the
implementation's prose, not a design criterion here (user ruling
2026-07-25): the recursive oracle remains the readable reference
matching the paper's equations, the differential suite guarantees
semantic accuracy against it, and the fused module doc must explain
the walk against those equations (the flag and route live in their
own state struct; the arms call into them at ≤ 3 sites).

**Recommendation**: **adopt — fused tick is the performance-best
design** (user ruling 2026-07-25: performance under the campaign's
bars decides; fusion is pre-approved given linearity with small
constants). It preserves the asymptotic profile (linear given
T-tick), roughly halves the hot grow branch's traversals, and its
copy-on-first-divergence refinement eliminates the discarded-output
build entirely — a win the unfused composition cannot reach. No
competing design matches it on measured merit, so no tie-breaker is
needed. Sequencing is engineering risk only: land it as a
**separate commit after the limb cure lands green** (bisectable,
independently reviewable), never compounded with the watermark
rewrite in one change. It needs no red pin and is judged by the
before/after table on the tick benign/deep rows.

## 7. Witness families and #34's acceptance contract

**Red pins (land first; instruments before cures; both scales;
measured exponents in the commit message):**

1. `version_tick`/`clock_tick` × **bigroot(b,d) × nested_full_id(d)**
   — the right-full chain. Red today on the existing limb ceilings
   (exponent 1.15 everywhere; constant 128/B from b = d = 8000).
2. `version_tick`/`clock_tick` × **the mirror cross**: a new
   generator pair — `nested_left_full_id(d)` (`(1, ·)` down the
   spine) × a right-leaning spine with one wide tail leaf (new event
   generator, e.g. `wide_tail(b, d)`) — the memo arm, the worse half
   (§2). Red today at every scale. This cross is also the kernel
   realization of L2's `wide-off-churn` tripwire (repeated raises
   near one wide minimum under cheap codes), so the cure's L2
   discipline is judged by the same cells.
3. `version_tick`/`clock_tick` × **descending-staircase**: a dense
   event spine with monotone-descending unit-delta leaves under a
   paired id spine (every level paired-internal, so every emission
   is a full-penetration min-update) — the family that falsified
   uncompressed I4 (§9/F1); all-narrow, pure depth — a distinct
   genre from both wide×deep crosses and from all four original
   model schedules. Expected red on the landed kernel's per-node
   materialized sums? **Measure, don't assume**: the red-pin agent
   pins whatever the meters read, red or green, and the cell exists
   either way — its job is to hold the CURE to linearity on the
   shape that breaks naive watermark propagation.
4. Judge roster entries for the new red cells, owned by #34.

**Model-side committed schedules** (the spec's own tripwires, run
per design round, not in the kernel gate): the fourteen-schedule
set of `emit_model.py` — the seven L1 schedules plus
`descending-staircase` (I4's tripwire), `wide-off-churn` (L2's
tripwire: quadratic against the pre-A1 discipline, 8.14/unit flat
against the discipline of record), `run-boundary-churn`,
`resurrection-cycle`, `burst-arm-close`, `anchor-flip-funded`, and
`benign` — plus `anchor-flip-UNFUNDED` kept deliberately as the
demonstration that the pricing chain (L2×L6) is load-bearing (it
MUST read superlinear against input-only content; if it ever reads
flat, the model has stopped charging emissions and is broken —
a liveness tripwire for the model itself).

**Green pins landing with the red-pin commit** (the review's
Finding 2, the memo path's negative space): mirror × *narrow* cross
cells (the memoized pre-scan exercised with unit deltas) — green
with a linear envelope and liveness floors derived from the honest
walk's counts on that shape (floors above vacuity, meaningfully near
the measured constant — never the generic 1 bit/B).

*Amendment 2026-07-25 (the cure landing)*: the discipline landed in
two stages (the walk's anchor web, then the chained-memo pre-scan)
and every #34-owned cell flipped at both scales — nested-wide limb
e 1.00 at 5.4/B flat, mirror-wide limb e 1.00 at 8.9/B with heap
under the ceiling and zero grown segments, mirror-narrow's memo
heap 13.9/9.0 per byte (the diff-coded memo stores one machine
word per site plus only *nonzero* chain links, so the pure chain
stores no accumulator at all), staircase flat at 16.0 limb/B —
under byte-identity across the full differential suite; the L6
output-bound pin (`tick_output_is_input_bounded`) and the
`TICK_NESTED_WIDE`/`TICK_MIRROR_WIDE` envelope rows landed with
it, and the four judge legs left the roster. **The pin corrected
L6's stated form** [measured, kernel tier]: the additive
`size(e) + O(size(i))` is refuted by honest arithmetic — grow's
zero leaf (and a raise's landing) can each re-code one delta
against a wide neighbor, duplicating one input code's width once
(a 175-bit event under a 6-bit id ticks to 255 bits, the shrunk
counterexample; committed as a seed) — and the honest bound is
the constant-factor `size(tick(e, i)) ≤ 2·size(e) + 4·size(i) +
32`, pinned (the committed pin runs at proptest's default 256
cases per execution; 8,192 cases was a one-off verification run,
recorded here as such). The pricing chain (A3) survives with
the constant carried: Σ emitted code widths ≤ 2·input + O(id) is
still input-denominated, so T-tick's O(n + m) conclusion is
unchanged; §4's L6 row reads with this correction. Realization choices, each conforming
to the invariants as stated: memo entries chain in *recording*
order (`m_j − m_{j−1}`, the recording relation riding the
pre-scan's stack as a follower) rather than literally against the
enclosing site's entry — the walk resolves each site against its
innermost open anchor by a chain-interval fold, with anchors on
the recursion's own frames; the walk-side relation is
anchor-tagged (height-carried after a max-side raise,
watermark-carried after a min-side one), realizing L2 rule 2's
generalized anchor; the drain assertion pairs an order-sensitive
position checksum with the queue-drained check so debug state
stays O(1). The ×4 segments residual (recursion depth, P4.2)
stays red on the tick-walk legs with counts re-pinned to the new
call shape.

*Amendment 2026-07-25 (the red-pin landing; measured dispositions
where this section predicted)*: the mirror-narrow cells landed
**RED on the heap constant** (93.2/B default, 95.6/B ×4, exponent
1.00 — one owned heap entry per left-full site; linear in count,
honestly over the 16/B ceiling), not green: the meters read the
memo's real constant, the pin keeps the honest reading, and the
cure's L4/L5 diff-coded memo flips it. The staircase cells landed
green on every work column at the default scale (the landed kernel
is linear on narrow full-penetration schedules — the cell holds
the cure to the same reading) and red at ×4 on segments only (the
recursion-depth genre, P4.2-owned, as every tick-walk family). The
mirror-wide cross also collapses under fill (the deepest raise
meets the tail and the equal-pair collapse telescopes to the root
— the deep witness derives it), so its tick takes the fill branch;
the memo entries it exercises are all wide, which is exactly the
L5 fan-out threat live in today's kernel (heap e 1.63–1.84,
red-pinned). Scan floors landed at full examination (8 bits/B,
derivation-backed) rather than a per-family measured fraction;
measured constants sit 1.8–5× above. The four wide×deep judge
cells are rostered; mirror-narrow and staircase are not
(wall-linear legs — their reds are board-column reds the judge
cannot see).

**The cure's acceptance**: all four wide×deep cells AND the
descending-staircase cells flip (or stay) green at both scales
(three identical runs each); scan columns unchanged
(e 1.00, byte-identical constants on non-tick cells); byte-identity
against the recursive oracle across every family including both new
crosses, exhaustive small scope, arbitraries, organic histories, and
the deep closed-form witnesses; the memo-drain and gap debug
asserts hold; heap within allowance (L5); roster entries leave with
measured linear exponents recorded; `fill.rs`'s `# Cost` restates
exactly what is then proven (per-dimension, tagged — if L2 closed by
argument, cite the invariant; if by measurement, name the families);
§13/§17.3 amendments with sums restated; envelope rows in
`tests/meter.rs` for the tick × wide×deep crosses pinned at
cure-earned constants. **No green, no merge.** If fusion is
ratified: its separate commit adds the flag-equivalence differential
+ tick-composition byte-identity, and the before/after table gains
the grow-branch traversal delta.

Cure sequencing within #34: red pin commit → watermark rewrite of
the walk (right-full arm first — it is self-contained) → the
pre-scan/memo conversion (L4) → fusion (if ratified). Each stage
byte-identical; one gate per agent; the board is the arbiter at
every stage.

## 8. Open decisions for Finch

Coordinator dispositions (2026-07-25, under the standing
authorization; each awaiting Finch's ratification alongside the
document): 1 — proceed, T-tick is #34's target; 2 — adopt fusion per
the pre-approval; 3 — yes, the L6 pin lands with the cure (the
two-ways-computable pin rule is standing policy); 4 — yes, the
segments residual stays P4.2-owned; 5 — delegated to the red-pin
agent as written.

1. **Ratify T-tick as #34's target?** REC: yes — the full
   discipline (compressed stack + anchored entries + lifetime
   accounting + the pricing chain) is validated composed on the
   limb-faithful model across fourteen schedules including every
   tripwire (round two), the slack bound is derived and
   twice-probe-confirmed, L3's enumeration is verified, and the
   landed precedents (V6 drift charging, the probe's bit-coded
   frames, the emit gap discipline) make amortized O(n + m) the
   right target; the pinned cells are the arbiter. **The condition
   this decision previously waited on — L2's model validation — is
   met**; the cure charter can be written against revision 3.
2. **Fusion (§6)** — pre-approved by ruling (2026-07-25) given
   linearity with small constants; no further go/no-go is needed
   unless the honest optimum turns out superlinear (§5's escalation
   path). Recorded here as the spec's recommendation: adopt, as the
   separate post-cure commit inside #34's charter; judged by the
   before/after table, no red pin.
3. **L6, the output-bound pin** — landed with the cure in its
   corrected multiplicative form (`≤ 2·size(e) + 4·size(i) + 32`;
   the additive form is refuted, §4's row and the round-3 record):
   two-ways-computable gets a pin, and it protects the input
   denomination the board rests on.
4. **Segments ownership**: keep the ×4 recursion-depth residual with
   P4.2 (the iterative rewrite rides the watermark stack naturally)?
   REC: yes; the cure must not silently change the segments profile
   without re-pinning.
5. **The board mechanics** for the new crosses (extend `tick_cross`
   vs a second operand slot): implementation detail — delegate to
   the red-pin agent. The smoke-pin count is **derived from what
   actually lands and re-verified then** (the §7 set as written
   implies 207 → 215: four wide×deep red cells + two
   descending-staircase cells + two mirror-narrow green cells — but
   the landed generator/slot structure decides, never this
   arithmetic transcribed).

## 9. Adversarial record (the design loop's convergence log)

**Round 1** (attack: `tick-spec-attack-1.md`; fix: revision 2).
Attack verdict: FALSIFIED. Fix round re-ran the attack harness
(every number reproduced exactly) and independently rewrote the one
non-persisted probe (the F2 slack bound) before citing it.

- **F1 — FALSIFIED, integrated**: L1/I4's uncompressed residue
  propagation is Θ(open depth) per full-penetration min-update
  (zero-diff frames cost touches and never absorb);
  descending-staircase measured Θ(d²), value-correct throughout
  (cost-only failure). Cure validated in the same harness: the
  zero-run-compressed diff stack, linear on all seven schedules.
  I4 restated as normative compression; the family joins the model
  schedule set and §7's kernel set.
- **F2 — restore path FALSIFIED, resolution derived, NOT closed**:
  wide-off-churn measured quadratic on naive and compressed stacks
  alike. L2 now carries the per-operand lifetime discipline as the
  resolution of record ([derived]; post-collapse slack bound
  measured worst 0/2000, twice independently); model validation is
  round two's first job, wide-off-churn the tripwire.
- **F3 — under-specification, cured in prose**: L4 gains the
  normative diff-coding of memo entries against the enclosing
  pre-scan's min-stack + LIFO carry at consumption; the anchor seam
  itself was verified sound (same position ⇒ same height).
- **HELD under attack**: L1 value-semantics (full-stack oracle,
  strictly stronger than the spec round's); L3 (comparison-site
  enumeration at the tip — none uncovered; obligation transferred
  to the cure's own final sites); L5 (strengthened by both fixes);
  L6 (telescoping survived the cliff-boundary attack); fusion —
  both equivalence directions, route-data reachability, and the
  4 → 2 traversal arithmetic.
- **Advisories dispositioned**: A1 first-leaf absolute-code flag
  case (normative sentence, §6); A2 route-fold × word-scale-skip
  interaction (recorded §4 segments row + §6 costs; P4.2 sequencing
  must name the fused walk); A3 Accum pooling for benign parity
  (named in §6 costs and the cure charter); A4 the model's
  sparse-wide undercount (L1's model-limitations clause; dense
  wides or limb-faithful accounting in future schedules).
- **No representation-level obstruction found** (both rounds
  concur): every fix is auxiliary walk state within the skyline
  coding.

**Per-lemma status after round 1**:

| clause | status |
|---|---|
| L0 scan | [measured, landed] — held |
| L1 watermark stack | [measured-on-model], seven schedules, compressed stack normative (F1 integrated) |
| L2 emission pricing | [derived] resolution of record; model validation PENDING (round two's first job) |
| L3 sign-read decisions | [derived], enumeration verified at tip; re-enumeration owed by the cure |
| L4 pre-scan/memo | [derived], seam verified; diff-coding now normative (F3) |
| L5 auxiliary space | [derived] — held, strengthened |
| L6 output bound | [derived, pin candidate] — held under attack |
| Fusion | [derived] — held both directions; A1–A3 normative notes added |
| T-tick overall | [open] until L2's model validation lands |

**Round 2** (attack: `tick-spec-attack-2.md`; fix: this revision).
Attack verdict: **HOLDS UNDER ATTACK** — no falsification of
T-tick; L2 VALIDATED on the limb-faithful composed model
(`emit_model.py`: the discipline implemented, not charged by
construction, under a full-stack + `d_out`-exactness + `last`-
invariant oracle, costs mirrored against `codec/accum.rs` and
confirmed by line-read); three normative under-specifications
(round 1's F3 genre: mechanism right, words didn't force it). Fix
round two reproduced every number (the committed tripwire
`wide-off-churn` 8.14 touches/unit flat, ratio 2.00 — was 95 → 729
quadratic; all fourteen schedules linear; `anchor-flip-UNFUNDED`
61.7 → 444, ratio 3.96, the demonstration for A3), closed one
provenance gap (`run-boundary-churn` was defined but unwired in the
committed harness — oracle-validated and re-measured at fix round
two, 6.23/unit flat), and confirmed the four cost-placement claims
against `accum.rs` directly.

- **A1 — integrated (L2 rule 3)**: the anchored-entry discipline is
  normative — raises enter anchor-relative (never h-relative wide),
  `T` never folded while it survives, arming moves, post-sign
  top-index domination at disparate scales, fold-and-restore of the
  narrow side only. Round 1's quadratic lived equally in the stack's
  own min-update comparison; rules 1–2 alone left it expressible.
- **A2 — integrated (L2 rule 2)**: "read O(1) times" was mis-stated;
  the bound is by *pricing*, not count — every wide read is a death,
  an O(1)-per-lifetime read, or priced by the code emitted at that
  site (unified with I3's priced-by-output arm; anchor-switch
  bridges are the case that forced it).
- **A3 — integrated (the pricing chain, L2's closing lemma)**:
  L2's costs are output-code-priced; L6 telescopes output to
  input + O(id); hence input-denominated O(n + m). The UNFUNDED
  schedule is kept as the model-liveness tripwire proving the chain
  is load-bearing.
- **Upgrades**: the post-collapse slack bound moved to [derived,
  probe-confirmed] (domination ⟹ slack ≤ 2 digits; worst 0 over
  2000 randomized + 548 structured constructions); fusion's flag
  alignment strengthened to the first-divergence-detector argument;
  I4 gains the funded-cascade clarification (reincarnation chains
  measured flat) and the no-split-operation note (run-boundary
  churn measured flat); L1's sparse-wide model limitation resolved
  by the limb-faithful model.
- **HELD under attack**: the compressed stack's new seams (run
  bookkeeping, `last` lifetime under close cascades), L4 under
  mixed nesting (sound by projection; composition note integrated),
  L5 under the composed discipline, benign constants flat
  (~12.6/unit — kernel-side parity remains the before/after table's
  question, pooling stays named in the cure charter).

**Per-lemma status after round 2 (this revision)**:

| clause | status |
|---|---|
| L0 scan | [measured, landed] — held |
| L1 watermark stack | [measured-on-model], fourteen schedules, limb-faithful, composed with L2 |
| L2 emission pricing | [measured-on-model] — VALIDATED; A1–A3 normative |
| L3 sign-read decisions | [derived], enumeration verified; + the domination guard; re-enumeration owed by the cure |
| L4 pre-scan/memo | [derived], held under mixed nesting; diff-coding + anchor generalization normative |
| L5 auxiliary space | [derived] — held both rounds |
| L6 output bound | [derived, pin candidate] — held both rounds; now load-bearing via the pricing chain |
| Fusion | [derived] — held both rounds; first-divergence alignment |
| T-tick overall | **[validated-on-model, kernel-pending]** |

**Round 3** (the kernel tier: the cure review + its fix round,
2026-07-25; findings F1–F7 in the review's report, dispositions
here). Attack verdict on the landed cure: semantically sound and
linear on everything then committed, but **T-tick's realization
REFUTED in the memo resolution** — and the fix round then found the
same machinery **semantically wrong** on a family the suite had
never crossed.

- **F1 — the memo resolution is Θ(k²) in touch currency
  [measured, kernel]**: the walk resolves each consumed site
  against an anchor by folding the recorded chain interval between
  two recording sequence numbers; consumption order (range starts)
  permutes recording order (range closes), so links are re-read
  once per crossing instead of dying at first read. Witness
  families committed as generators with gate-enforced red pins
  (`memo_chain(k, distinct)`: k consumption-sibling sites, touch
  growth ×3.94/doubling; `memo_comb(d)`: interleaved shallow and
  covering sites, ×3.92; the shared-minimum control flat at ×1.25
  per byte — the records are not the cost, the resolution is).
- **Both charter cure shapes REFUTED before implementation**
  [derived + modeled, the fix round's telescoping check]:
  anchoring to the *previously consumed* site telescopes only
  sibling chains — `memo_comb`'s interleaving keeps consecutive
  consumptions Θ(d) apart in recording order under it too
  (modeled: ×3.97/doubling, same class as the current
  realization); recording diffs *literally against the enclosing
  site* is unrecordable (the parent's minimum is not final at the
  child's close, and finalization corrections fan out). The cure
  was therefore NOT landed this round — no green, no merge, and no
  mechanism that satisfies the pin without the property. The next
  design round's seed: **position-anchored per-site records** —
  each site's minimum recorded relative to the walk's own live
  state at the site's range start (the innermost armed watermark,
  or the height where none is armed), so each record is created
  once, moved at its consume, and dies — with two named open
  corners: (i) pre-arming site blocks share one wide seed (the
  fan-out threat re-enters unless the block's records ride a
  register/diff structure of their own); (ii) the pre-scan emits
  interior raises in post-order while the walk emits them in
  pre-order, so instant-anchored watermark readings differ
  between recorder and consumer exactly where an ancestor's
  deferred raise is pending — the recorder must anchor to
  quantities invariant under that reordering.
- **A SEMANTIC bug in the chained memo, found by the new
  families' first pool crossing [measured, minimized, fixed]**:
  in the watermark-carried max arm, the re-anchored relation
  follower installed AFTER the raise emission missed that
  emission's own arm fold and went stale by the arm's delta —
  later sites resolved high by it (minimized: `memo_chain(3) ×
  memo_comb_id(2)` raised a leaf to 4 where the oracle says 3).
  Fixed by installing the follower before the emission; the
  differential pools now carry both families event- and id-side
  including cross-family pairs (the genre that caught it) and a
  4096-site closed-form witness. The lesson joins the record: the
  cost families and the semantic suite must cross — a family
  landed only as a cost witness leaves its shapes' semantics
  untested.
- **F2 — the acceptance currency was watched nowhere on the tick
  surface [confirmed, fixed]**: the tick envelope rows moved onto
  the five-meter harness (heap, segments, limb, scan, touch),
  ceilings ×1.25 and liveness floors ×0.75 over fresh
  measurements; every touch-carrying envelope row now floors the
  touch column (a ceiling over a stopped counter proves nothing).
  Board cells carry no touch column until #35's currencies axis;
  the red pins are gate-enforced tests, which a board red is not.
- **F3 — the ORBIT LEMMA [measured, pinned]**: iterated tick does
  not compound. Orbit values are bounded by max input value + k,
  so codes stay within max(input width, log k) + O(1); live node
  count ≤ input leaves + id internal nodes; expansion recurrence
  under alternating ids is size-idempotent (re-coding, not
  stacking — measured oscillating in a fixed band over 2048
  ticks); hence `bits(tick^k) ≤ bits(tick^1) + 4·bits(id) +
  4·⌈log2(k+1)⌉ + 8` after the one-step ≤2× transient. Pinned:
  `tick_orbit_growth_is_transient_plus_log` (arbitrary pairs, 48
  ticks) and `tick_deep_orbits_stay_banded` (the fixed wide pair
  over 4096 ticks, frozen at +24 bits; the alternating disjoint
  pair over 2048, banded). Note the two conjecture clauses the
  measurements refuted en route: "once expanded, never expands
  again" fails under alternation, and fill's collapse does
  recreate the duplicating configuration — both harmlessly,
  because re-firing replaces rather than accumulates.
- **F4/F6/F7 — prose and provenance corrected**: §4's L6 row and
  §8's item 3 now carry the multiplicative form (the additive form
  presented as held was stale); the 8,192-case provenance is
  restated as the one-off it was (the committed pin runs at the
  256-case default); `fill.rs`'s `# Cost` states the quadratic
  memo resolution honestly with the red pins named, and the
  pre-scan recorder's per-site sign read joins the comparison
  enumeration. F5 (the memo-drain checksum) was verified adequate
  unchanged.

**Per-lemma status after round 3**: L0 [measured, landed] — held;
L1 [measured-on-model] — held (the walk and web read linear on
every committed family); **L2/L4's memo-resolution realization
[REFUTED at kernel, red-pinned; the position-anchored redesign is
the next round's charter]**; L3 [derived] + the recorder's sign
read; L5 [derived] — held; L6 **[measured, pinned, multiplicative]**;
the orbit lemma **[measured, pinned]**; fusion [derived] — held,
BLOCKED behind the memo redesign (no fusion over a refuted
realization); **T-tick overall: [open at the kernel tier] — the
target stands, its realization returns to the design loop.**

**Round 4** (the kernel tier: the frame-ledger cure, landed
2026-07-25, commits `4934db86` + `952159f7`). The memo's resolution
is linear on every committed family: `memo_chain(k, distinct)`
60,023 → 120,023 touches across the doubling (×2.00 exactly; ×3.94
under the refuted chain), `memo_comb` 43,532 → 87,032 (×2.00; was
×3.92), both gate asserts flipped to ≤ ×2.5 in the cure commit —
re-pinned, never deleted — with the shared control still flat.
Byte-identity across the full differential suite; the five-meter
tick rows: dense and nested-wide byte-identical on every column,
mirror-wide at heap parity (27,397 vs 27,389) with touches 105,559
inside the prior ceiling and scan byte-identical; board sums
unchanged at both scales (198/17, 189/26 over 215; the ×4 segments
residual's counts moved 11→14/5→6/12→16/22→28 with the pre-scan's
deeper frames — the P4.2-owned genre, same reds, same owner).

- **Realization deviation, with derivation (dated 2026-07-25)**:
  the landed ledger realizes revision 3's architecture (one link
  per site, sibling chaining, deferred-to-final references, zero
  links unstored, create-once/read-once/die) but NOT its literal
  position-stamped anchoring: a stamp read at a later position
  needs the net input movement between the two positions, and
  across suspended site-nesting levels no O(1)-per-event carrier
  of that net exists (a per-frame live net re-admits the churn
  quadratic; stamped nets of a global net accumulator re-admit
  the interval folds). The landed anchoring instead keeps ONE
  live min-relative head (the existing follower discipline — one
  funded fold per min-moving event), sibling-chains within each
  level (so head operands stay at the local link width — the A1
  fan-out killed FLOOR-anchoring, and the sibling chain plus
  unstored zero links is what dodges it), suspends outer levels'
  heads as immutable value diffs (both endpoints final minima —
  nothing maintains them, dodging A2), and DEFERS each level's
  first-child link to its forest parent's own record, where the
  parent's minimum is final — the reference the walk actually
  carries at that consume (the walk's pre-order predecessor of a
  first child is its parent; of a later sibling, the previous
  sibling, re-anchored at zero cost from the walk's own web at
  the sibling's close, where the node frame's minimum IS the
  site's). The recording-order queue is written out of order and
  consumed in order.
- **Two in-flight catches by the committed instruments** (the
  round's own evidence that the pins work): the first-draft
  resolve folded the wide suspended diff into the narrow resolver
  (×18 touches on the mirror-wide envelope row — L2 rule 1's own
  violation, cured by folding the narrow dying side into the wide
  survivor); and a per-resolve accumulator mint grew the pre-scan
  pool by one buffer per site (~51 B/site on the same row's heap
  column, cured by buffer reuse — heap back to parity).
- **The four §7 families landed** as generators + gate pins +
  differential-pool members: `memo_fanout` (zero sibling links +
  one deferred wide link; the absolute touch ceiling is the
  k-independence assert), `memo_oscillating` (the funding
  control), `memo_churn` (one live head follows d in-flight
  records through full-penetration drops — flat; the
  live-anchored followers' tombstone), `descending_raises` (the
  ordering tripwire, verified LIVE: an install-after-emit kernel
  fails its family_pairs differential). The pin harness
  re-denominated against the version's own stored stream (the
  construction language's absolute leaf codes overstate a plateau
  family's input by orders of magnitude).

**Per-lemma status after round 4**: L0 [measured, landed]; L1
[measured-on-model + kernel, linear on every committed family];
**L2/L4's memo realization [measured at kernel — CURED, linear,
the flip pinned]**; L3 [derived] + the ledger's sites in the
`# Cost` enumeration; L5 [measured — heap parity, one queue word
per site]; L6 [measured, pinned, multiplicative]; the orbit lemma
[measured, pinned]; fusion [derived] — **UNBLOCKED pending this
cure's adversarial review**; **T-tick: [measured at kernel on
every committed family; the §7 acceptance stands met at the
board and gate tier]**.

**Round 5** (the kernel tier: the frame-ledger cure's adversarial
review + its red-pin round, 2026-07-25). Attack verdict:
**T-tick REFUTED at the kernel by a new family** (2026-07-25;
semantics exact everywhere — the failure is cost-only). Recorded
here as instruments-before-cures: generators, differential-pool
membership, and gate pins land with this record; the cure is the
next design round's charter, not this one's.

- **The family — `reveal_comb(k, b)`, red-pinned [measured,
  kernel, three identical runs; the review's probe reproduced
  exactly on the committed generators]**: one covering left-full
  site over a left-leaning comb `a_i = node(0, a_{i−1}, site_i)`,
  floor `leaf(0)` at the deepest left, `site_i = (1, (1, 0)) over
  node(0, leaf(2^b − 1), leaf(2^b))` — k sibling sites sharing one
  wide minimum `W = 2^b`, the left-leaning spine closing each
  site's node frame back into the 0-floor frame between
  consecutive consumes. Touches are **Θ(k·b) on input Θ(k + b)**:
  per-byte 146.9 → 267.7 → 478.7 → 808.8 as b doubles at
  k = 1,000; the joint doubling (1,000, 1,024) → (2,000, 2,048)
  reads 738,449 → 2,884,881 touches — ×3.91 on a ×2.00 input.
  **The denominator**: the output is Θ(k + b) too (every site's
  fill collapses to the shared plateau leaf; per-site output
  deltas are γ-unit codes), so the blowup survives the
  input+output denomination — this is a true amplifier under the
  spec's own denominator, not an output-priced cost.
- **The mechanism — an unfunded width-circulation cycle; I4's
  funded-cascade enumeration is incomplete**: per site, the
  consume decision mints a width-b boundary `Diff(W)` between the
  site's frame and the floor frame (`arm_relative`, fed by the
  negated relation follower through `compare_above_vs`); the
  site's close pops it (`close`'s dying-diff fold), refilling the
  base stack's `t` and the relation follower with the width; the
  next consume negates the follower into the next `d_arm` and
  mints the next `Diff(W)`. Every object individually satisfies
  create-once/read-once/die — the width circulates through
  per-object-legal moves with NO input delta, NO output code, and
  NO undercut descent funding any hop. I4's funded-cascade clause
  prices only full-penetration *undercut* hops (height descent
  paid in input delta codes); arm-up and close-reveal hops are
  unfunded by its enumeration. **L1 and I4 reopen at the spec
  tier.**
- **Attribution, both layers pinned [measured]**: `pure_comb(k, b)`
  — the same comb with no left-full site anywhere (bare
  leaf-under-internal-id frames; no memo, no pre-scan, no site
  consume) — is width-scaling at ~2 wide folds per site (per-byte
  30.4 → 50.8 → 82.0 as b doubles at k = 1,000): the base L1
  watermark stack's own arm-move + close-pop cycle, predating the
  frame ledger. The ledger amplifies it ~10× — `reveal_comb` runs
  ~21 width-b folds per site (the follower ferry,
  `compare_above_vs`, `arm_relative`, and the pre-scan head's
  mirrored folds).
- **The control — `reveal_comb_hifloor(k, b)`, green-pinned**:
  identical forest, identical ids, identical deferral and
  close-reveal cycle, floor raised to `W − 2` so the consume-time
  gap is 2 — flat and width-independent (per-byte 21.4 → 18.9
  across the width *quadrupling* b = 512 → 2,048 at k = 1,000;
  absolute band 56,831 ×1.25/×0.75). The wide GAP is the driver,
  not the site forest, not the deferral, not the close-reveal
  schedule.
- **Model reproduction — YES [measured-on-model]**: the committed
  round-two harness (`emit_model.py`, unmodified) fed a reveal
  schedule (k sites armed at one shared wide minimum over a
  0-floor frame, close-reveal between consecutive consumes; the
  input pays the width once) reproduces the class: per-unit
  13.72 → 21.62 → 37.25 → 67.77 as b doubles at k = 1,000 (the
  width term's ratio → ×2 as it dominates the additive constant),
  flat per unit in k, joint-doubling ratio 3.16 → 3.47 → 3.69
  (→ ×4). The width lives in `close_consume`'s dying-diff fold
  and the relation bookkeeping — the same placement as the
  kernel's. The model's per-site constant (~2.4 wide folds)
  matches the BASE layer (`pure_comb`'s ~2), not the
  ledger-amplified kernel (~21): the model contains no pre-scan
  and no ledger, so it reproduces exactly the L1 half of the
  attribution split. The model is sound on this genre; the coming
  design round can calibrate candidate disciplines against it.
- **The instruments landed with this record**: three generators
  (`reveal_comb`, `reveal_comb_hifloor`, `pure_comb` + ids) in
  the differential pools event- and id-side at two scales plus a
  4096-site closed-form deep witness (semantics exact against the
  recursive oracle before any cost was pinned); gate red pins in
  `tests/meter.rs` (`reveal_comb` ≥ ×3.5 across the joint
  doubling, `pure_comb` ≥ ×1.45 per-byte across the width
  doubling — each paired with its shape's closed-form tick and
  the one-touch-per-byte liveness floor, each failing the moment
  the circulation is cured, to be re-pinned flat, never deleted)
  and the hifloor green pin; board tick-cross rows for all three
  at both scales (215 → 221 cells; every counter column green at
  the default scale — the touch currency rides no board column,
  so the four `version_tick`/`clock_tick` × `reveal-comb`/
  `pure-comb` cells joined the bench judge's roster as owned
  reds: the fitted time exponent is the one board-side leg that
  sees the circulation).

**Per-lemma status after round 5**: L0 [measured, landed] — held;
**L1/I4 [REOPENED at the spec tier: the funded-cascade clause
enumerates undercut hops only; arm-up/close-reveal hops circulate
width unfunded — red-pinned at the kernel, reproduced on the
model]**; L2/L4's memo realization [measured at kernel — linear on
the consumption-order families; the ledger's ferry additionally
*amplifies* the reopened L1 cycle ~10× on the reveal comb]; L3
[derived] — held; L5 [derived + measured] — held; L6 [measured,
pinned, multiplicative] — held; the orbit lemma [measured,
pinned] — held; fusion [derived] — BLOCKED again behind the L1/I4
revision (no fusion over a refuted realization); **T-tick:
REFUTED-pending-revision (2026-07-25) — the statement's target
stands, its I4 funding argument does not; the revision is the
next design round's charter, seeded with the width-circulation
mechanism and both attribution layers.**

**Convergence assessment (fix round two)**: **CONVERGED at the spec
tier, revision 3.** The loop's convergence rule — a round with no
falsifications whose findings are wording/normativity — was met by
round two, and its amendments are now integrated. The honest
residue, named rather than hidden: (i) the model exercises the
discipline over abstract emission schedules, not over fill's actual
arm structure driven by real (id, event) pairs — the mapping from
the arms to the model's primitives is [derived] in §1/§3, and its
verification is exactly the red pin's mandatory kernel
re-measurement (§7, normative); (ii) L3's enumeration binds today's
code, and the cure re-runs it against its own final sites (§7,
normative); (iii) benign-input parity is a kernel-side question the
before/after table judges (§6). None of these is a model-tier open
— a round three against this document would re-attack validated
clauses rather than new surface. Recommendation to the coordinator:
declare convergence, commit the spec, and charter the cure against
revision 3.
