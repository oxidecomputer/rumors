# `before`: the Version encoding against the information-theoretic floor — gamma as built, delta and omega as alternatives

Status: analysis of record, 2026-07-27, for the packed `Version`
encoding at `cb6a252d5552a8d0e0f95b1ff654534a76998c21` (the skyline
coding; every grammar fact below is extracted from that tree, with the
enforcement point named). Scope: the **worst-case/counting** side of
the value-code question — how far each coding's worst case sits above
the information-theoretic floor for the set it covers. The
**workload-typical** side (value histograms over realistic corpora,
deciding which code is shorter on the streams organic histories
actually produce) is a separate companion study, open at this writing;
nothing here assumes its results, and §8 explains why the two must be
read together before acting on either.

Epistemic key: **[derived]** = exact arithmetic or a kernel of algebra
from the grammar, cross-checked by census where marked; **[census]** =
exact integer count by dynamic program or exhaustive enumeration,
pinned against independent recomputations (§6); **[bracketed]** =
rigorously bounded above and below by exact counts of super- and
sub-families; **[fitted]** = extrapolation with the fit stated;
**[argued]** = a probabilistic argument from the literature,
corroborated by census but not kernel-checked.

Headline (multiplicative distance of the worst case above the floor,
`n / H(n)` per §1, same grammar, only the value code substituted):

| value code | asymptotic | 2 kB | 100 B | 50 B |
| --- | --- | --- | --- | --- |
| gamma (as built) | **1.043017** [derived] | ≈ 1.0447 [fitted; ≥ 1.0444] | **1.0669–1.0673** [bracketed] | 1.0863–1.0865 [bracketed] |
| delta (hypothetical) | **1** exactly [derived] | ≈ 1.002 [fitted] | 1.0231–1.0234 [bracketed] | 1.0416–1.0424 [bracketed] |
| omega (hypothetical) | **1** exactly [derived] | ≈ 1.0014 [fitted] | 1.0221–1.0222 [bracketed] | 1.0423–1.0426 [bracketed] |

By the counting metric alone, delta and omega dominate: their
asymptotic tax is zero and their 100-byte tax is a third of gamma's.
§8 explains why this alone must not decide the code choice — the same
structural fact that makes delta/omega counting-optimal (giant values
priced logarithmically) is irrelevant-to-harmful on small-delta
workloads, where gamma is pointwise better or tied on every value
below 31.

## 1. The question, and the framing that makes it answerable

"How far is the packed `Version` representation from the
information-theoretic minimum?" needs a set to count. Two candidate
framings:

- **(a) Self-parametrized (used throughout).** Let `S(n)` = the set of
  versions whose canonical encoding fits in `n` bits, and
  `H(n) = log2 |S(n)|`. Any injective code — ours, or the best
  conceivable replacement — that represents all of `S(n)` must assign
  some member at least `⌈log2(|S(n)|+1)⌉ − 1 ≈ H(n)` bits. Our worst
  member of `S(n)` costs `n`. The multiplicative overhead is
  `n / H(n)`, uniform in scale: the statement holds at every prefix
  length simultaneously, so no single `n` is cherry-picked. This is
  the standard competitive framing for universal codes. Note `S(n)`
  differs per value code: each code is measured against its own
  covered set (the sets are mutually incomparable — a many-small-delta
  version may fit a gamma budget but not a delta budget, and a
  giant-value version the reverse).
- **(b) Natural semantic families.** Fix versions with `ℓ` plateaus
  and heights below `2^w`, and compare a code's worst case on that
  family to `log2` of the family's size. The framings **diverge
  materially** (§7): on bounded-uniform families gamma's ratio
  approaches 2 as `w` grows. Any published claim quoting (a) without
  naming this would mislead.

The floor in (a) is against *any injective mapping to bit strings* —
at least as generous to the hypothetical competitor as requiring a
prefix code, so the bound is conservative. `S(n)` is the coding's
whole representable domain at that size — `Version::decode` accepts
every canonical stream, whether or not a particular operational
history produces it.

## 2. The exact grammar, from the code

A stored `Version` is one bit stream (`version/skyline.rs`):

- **Topology**: one preorder flag bit per node of a full binary tree
  (`0` internal, `1` leaf, so a descent is one unary run). Internal
  nodes carry no numbers. The flag polarity is a bijection on
  encodings, so every count in this analysis is invariant under it.
- **Payloads**, in-stream at each leaf: the first leaf's absolute
  height as `code(h₁)`; each later leaf as `code(zigzag(hᵢ − hᵢ₋₁))`,
  `zigzag: k ≥ 0 → 2k, k < 0 → 2|k| − 1`.
- **Value code** (`codec/gamma.rs::encode_int` as built): bucket
  `k = ⌊log2(v+1)⌋` holds exactly `2^k` values, all at one cost:

```
gamma:  cost(k) = 2k + 1
delta:  cost(0) = 1;  cost(k) = k + 2⌊log2(k+1)⌋ + 1
omega:  cost(0) = 1;  cost(k) = k + 1 + L(k),  L(1) = 1,
        L(N) = ⌊log2 N⌋ + 1 + L(⌊log2 N⌋)
```

  All three code zero in one bit, so the canonical-form exclusion
  below removes cost 1 from the sibling position in every model, and
  the bucket/zigzag structure is code-independent — the hypothetical
  substitutions change only the per-bucket bit price.

Canonical form, enforced bit-exactly by
`version/skyline/validate.rs::validate_from` (every reject is
`NotCanonical`/`Truncated`/`TrailingBits`):

1. **Minimal topology**: no internal node whose two children are both
   leaves with a zero right delta (the collapsible sibling pair). A
   zero delta between *non-sibling* consecutive leaves is canonical.
2. **Nonnegativity**: every running leaf height stays `≥ 0`.
3. **Exactness**: exactly one complete tree, no trailing bits.

Byte level (`Version::decode` in `version.rs` +
`codec/bits.rs::require_zero_padding`): the stream is zero-padded to a
byte boundary, and decode rejects both nonzero padding and any whole
spurious zero byte, so byte strings biject with canonical streams.
Gamma has one spelling per natural and zigzag is a bijection with no
negative zero, so no code-level non-canonical spelling exists to
prune. Canonical streams are unique representations of the step
function (module doc's uniqueness argument; pinned by the
byte-uniqueness and exhaustive small-scope injectivity tests). Counting
canonical streams by exact bit length therefore counts versions:
`f(n)` below is both. The analysis runs in bits pre-padding; §7 folds
the ≤ 7 pad bits back in.

## 3. Four nested counting models

With `f_model(n)` = number of streams of exactly `n` bits, per value
code:

| model | constraints kept | role |
| --- | --- | --- |
| `free` | none (any payloads on any full tree) | Kraft baseline |
| `unc` | sibling rule (1), exactness (3) | analytic workhorse |
| `con` | all three — **the real family** | ground truth |
| `lower`/`upper` | clamp brackets around `con` | rigor at large `n` |

`f_lower ≤ f_con ≤ f_upper ≤ f_unc ≤ f_free` for every `n` (each is a
sub-family of the next; pinned computationally in §6).

## 4. The analytic constants [derived]

Generating functions in `x` marking bits. The value-code GF
`G(x) = Σ_k 2^k x^{cost(k)}` counts naturals by code length — and,
via zigzag, all integers; the first leaf's absolute payload has the
same GF, so no special-casing. A leaf: `A = xG`. A right-sibling leaf
(zero excluded): `A′ = x(G − x)`. Internal subtrees, by the four
child-shape cases (the sibling rule binds exactly when both children
are leaves):

```
B = x·(A·A′ + A·B + B·A + B·B),        V = A + B
```

`B` solves `xB² + (2xA − 1)B + xAA′ = 0`; the census growth rate is
set by whichever comes first: the branch point where
`Δ(x) = (1 − 2xA)² − 4x²AA′` hits zero, or the value code's own
singularity.

**Gamma.** `G = x/(1 − 2x²)` (analytic out to `1/√2`), so
`Δ(x) = 0` reduces to `(1 − 2x² − 2x³)² = 8x⁸`, i.e.
`1 − 2x² − 2x³ − 2√2·x⁴ = 0`, smallest positive root
`x_c = 0.514500091561…` (the conjugate factor's root 0.7057 and the
pole `1/√2` lie beyond; the census ratio test in §6 confirms this is
the operative singularity). Hence

```
α = log2(1/x_c) = 0.958756761…      1/α = 1.043017417…
f_unc(n) = K·2^{αn}·n^{−3/2}·(1 + O(1/n)),   K = √(−x_c·Δ′(x_c))/(4·x_c·√π) = 0.599185
H_unc(n) = αn − 1.5·log2 n + log2 K − log2(1 − 2^{−α}) + O(1/n)
```

The closed form agrees with the exact census to **−0.0045 bits at
n = 800** [census], so its use at 16384 carries error well under 0.01
bits.

**Why gamma's constant is pure canonicality tax.** Gamma satisfies
Kraft with equality (`Σ_k 2^k·2^{−(2k+1)} = 1`), as does the 1-bit
leaf/internal flag; the composite free code is prefix-complete: every
infinite bit string begins with exactly one free stream, and the free
characteristic equation `1 − 2x² − 4x³ = 0` has its root at exactly
`x = 1/2` — `α_free = 1`, zero asymptotic redundancy. Pinned two ways
[census]: `T(1/2) = 1` algebraically, and the census Kraft partial sum
to `N = 200` shows deficit `8.9243×10⁻²` against the predicted
polynomial tail `2K_free/√N = 8.9206×10⁻²` (`K_free = √5/(2√π)`; the
tail is polynomial precisely because the singularity sits *at* 1/2),
agreement 0.04%. Dropping from `α_free = 1` to `α = 0.9588` is
entirely the sibling-collapse pruning; nonnegativity costs nothing
asymptotically (§5).

**Delta and omega: α = 1 exactly.** Both are also Kraft-complete
(delta: `Σ_k 2^k 2^{−cost(k)} = ½·Σ_k 2^{−2⌊log2(k+1)⌋} = ½·Σ_j
2^j·2^{−2j} = 1`; omega: the self-similar identity `S = ½ + ½S` from
`L(N) = ⌊log2 N⌋ + 1 + L(⌊log2 N⌋)` forces `S = 1`), so
`G(1/2) = 1` — but unlike gamma, their `G` has radius of convergence
*exactly* 1/2: the value code is itself critical at capacity. At
`x = 1/2` the discriminant of the pruned grammar is then
`Δ(1/2) = (1 − 2·½·½)² − 4·¼·½·¼ = ¼ − ⅛ = ⅛ > 0` — an identity
that holds for *any* complete value code with a 1-bit zero — and
`Δ > 0` on the whole interval (numeric grid, min 0.127 for delta,
0.161 for omega [census]): **no branch point exists below 1/2**. The
dominant singularity is the value code's own at `x = 1/2`, so
`α = 1`: the canonicality pruning is asymptotically free under delta
and omega. Structurally: a single-leaf stream already realizes the
capacity — delta prices `2^c/Θ(c²)` values at cost `c` (a
bounded-ripple constant from the floors), so leaf-only trees alone
contribute `2^n/poly(n)` streams — and the counting measure at
criticality concentrates on few-leaf, giant-payload streams. The distance from the floor is then all
finite-size: `n − H(n) = Θ(log n)`, overhead `1 + Θ(log n / n) → 1`.

## 5. The nonnegativity pruning [argued + census + bracketed]

**Cost symmetry [derived].** A delta `+m` codes as zigzag `2m`, `−m`
as `2m − 1`; both land in bucket `⌊log2(2m+1)⌋ = ⌊log2(2m)⌋` (they
could differ only if a power of two lay in `(2m, 2m+1]`, and
`2m+1 > 1` is odd). So the height walk has sign-symmetric step
weights under **any** bucket-priced value code — gamma, delta, and
omega alike.

**Growth constant unchanged [argued].** Under the size-`n` Boltzmann
measure the per-leaf payloads are independent draws from a fixed
sign-symmetric distribution (two types: sibling positions exclude
zero). Sparre–Andersen-type universality for symmetric walks puts the
probability of `ℓ` partial sums staying nonnegative at `Θ(ℓ^{−1/2})`
— polynomial, so the exponential rate survives the constraint. This is
the one step resting on a probabilistic argument; everything quoted at
finite `n` is independent of it (bracketed or exact), and the census
corroborates it for gamma directly: the deficit
`δ(n) = log2(cum_unc/cum_con)` runs 0.42 (n=16), 0.71 (24), 0.91
(32), 1.19 (48), 1.39 (64) [census], fit `δ(n) ≈ 0.469·log2 n −
1.429` on `n ∈ [36, 64]` — slope consistent with the predicted
`½·log2 ℓ + O(1)` under `ℓ ∝ n` [fitted]; extrapolated to `n = 800`
it predicts 3.10 bits where the rigorous bracket pins `δ(800) ∈
[3.03, 3.29]`. For delta/omega the deficit is much smaller (0.28 and
0.37 bits at `n = 21`, the exact-reference limit) because the
counting measure concentrates on few-leaf streams with an
unconstrained first payload — fewer chances to go negative.

**Rigorous finite-`n` brackets [bracketed].** Two families with
exactly countable state, per code:

- *Lower (sub-family)*: track `d` = a certified floor under the true
  height, saturating at `C` (`d = min(h, C)` while exact); admit a
  descent only when `m ≤ d`. Every accepted stream is canonical, and
  `d` is a deterministic function of the stream prefix, so each is
  counted once: `f_lower ≤ f_con`.
- *Upper (super-family)*: track `u = min(h, C)`, exact until first
  saturation (`u < C ⟹ u = h`), then absorbing with free dynamics;
  reject a descent only when it provably violates (`m > u`, `u < C`).
  Every canonical stream is accepted exactly once:
  `f_con ≤ f_upper ≤ f_unc`.

At `C = 256`, `n ≤ 800`, the gamma bracket closes to 0.26 bits of
`H`: `H_con(800) ∈ [749.558, 749.813]` against `H_unc(800) =
752.848`; delta and omega bracket to ~0.2 bits likewise (delta
`H_con(800) ∈ [781.71, 781.93]`, omega `∈ [782.66, 782.73]`).

**Where the exact constrained census can and cannot go.** The exact
DP for `con` collapses all heights above `D[R]` (the maximum descent
purchasable with the remaining `R` bits) into one SAFE state — exact,
because `D[R] ≥ drop(c) + D[R−c]` makes SAFE absorbing and
continuation-equivalent. Under gamma a descent costs ~2 bits per
magnitude bit, so `D[R] ≈ 2^{R/2}` and the census reaches `n = 64`
(23 s). Under delta/omega a descent of magnitude `m` costs only
`~log2 m + O(log log m)` bits, `D[R]` explodes to `2^{R−O(log R)}`,
and the safe-collapse DP is infeasible at any useful `n` — the exact
constrained reference for those codes is exhaustive enumeration to
`n = 21` instead (§6). The clamp brackets, whose state is
height-structure-independent, carry the large-`n` rigor for all
three.

## 6. The census machinery and its pins

Instruments, cross-pinned per the metering doctrine (any quantity
computable two ways gets a pin). All exact-integer except the
brackets and the large-`n` series (float64 over all-positive sums;
~10⁻¹³ relative, invisible at `log2` scale — and both float engines
are themselves pinned against exact-bigint twins).

1. **Series extraction** [census]: `f_unc` per code to `n = 800` by
   coefficient recursion on §4's algebraic equation (the `B·B`
   convolution never references the coefficient being computed);
   `f_free` for gamma to 200.
2. **Sequential stream DP** (independent mechanics: explicit
   excess/context state machine over preorder symbols): recount of
   `f_unc` to 300 per code. **Pin: equals the series exactly, all
   three codes.**
3. **Safe-collapse exact DP** [census]: `f_con` for gamma to
   `n = 64`, exact bigints (§5 explains why gamma only).
4. **Brute force**: every bit string to length 18 (gamma) / 21
   (delta, omega) validated by a direct transcription of
   `validate_from` with an independently written decoder per code
   (gamma/delta/omega readers), in all three constraint modes.
   **Pins: equals the con DP (gamma), the unc series (all three), and
   the free series (gamma), term by term.** For delta/omega this
   enumeration *is* the exact constrained reference.
5. **Clamp brackets** (§5) at `C ∈ {16, 256}` to `n = 800` per code,
   vectorized float64. **Pins: the float engine at `C = 16` matches
   an independent exact-bigint clamp DP to ≤ 4×10⁻¹⁶ relative (all
   three codes); lower ≤ con ≤ upper ≤ unc on every computed
   overlap.**
6. **Scaled series** [census]: `f_unc·2^{−n}` per code to
   `n = 16384` in float64 by the rescaled recurrence. **Pin: matches
   the exact bigint series to ≤ 8×10⁻¹⁶ relative at `n ≤ 800`, all
   three codes.**
7. **Analytic vs census** (gamma): ratio `f_unc(n)/f_unc(n−1) →
   2^α = 1.943634` with the predicted `n^{−3/2}`-consistent
   correction (relative error +8.3×10⁻⁴ at `n = 800`, halving per
   doubling); fitted transfer constant → `K_analytic` (ratio 1.0012
   at 800). (Delta/omega): discriminant grid strictly positive on
   `(0, ½)` — no branch point, `α = 1`.
8. **Hand values** (worked from the grammar in one-leaf/two-leaf
   cases): gamma `f_con(2,4,6,7,8) = 1,2,4,1,8`, `f_unc(7) = 2`;
   delta `f_con(2,5,6,7,8) = 1,2,4,0,1`, `f_unc(8) = 2`; omega
   `f_con(2,4,7) = 1,2,5`, `f_unc(7) = 6`. All green.

Provenance: single-file deterministic script (no seeds), stages and
all pins included:
`/Users/oxide/.claude/jobs/5de24d9f/tmp/entropy-census.py` (outside
the repo; the algorithms are fully specified above and in its
docstrings). Recompute (~5 min total, single core; numpy needed only
for `npclamp`/`bigseries`):

```
python3 entropy-census.py free 200
python3 entropy-census.py con gamma 64
python3 entropy-census.py brute gamma 18
python3 entropy-census.py brute delta 21 ; python3 entropy-census.py brute omega 21
for c in gamma delta omega; do
  python3 entropy-census.py series $c 800
  python3 entropy-census.py uncdp $c 300
  python3 entropy-census.py clamp $c 300 16
  <numpy-python> entropy-census.py npclamp $c lower 300 16
  <numpy-python> entropy-census.py npclamp $c lower 800 256
  <numpy-python> entropy-census.py npclamp $c upper 800 256
  <numpy-python> entropy-census.py bigseries $c 16384
done
python3 entropy-census.py report        # exits nonzero on any pin failure
```

## 7. Results

Exact small-`n` census of the real family, gamma as built [census]:

| n | f_con(n) | cumulative | H(n) | n/H(n) |
| --- | --- | --- | --- | --- |
| 2 | 1 | 1 | 0.000 | — |
| 8 | 8 | 16 | 4.000 | 2.000 |
| 16 | 342 | 783 | 9.613 | 1.664 |
| 24 | 27,764 | 63,040 | 15.944 | 1.505 |
| 32 | 3,097,466 | 6,877,767 | 22.714 | 1.409 |
| 64 | 1,337,315,482,328,506 | 2,852,186,577,854,486 | 51.341 | 1.247 |

Regime table, per value code (bits; `H = log2` cumulative count;
brackets from the `C = 256` runs; 1600/16384 for gamma via §4's
validated closed form plus the §5 fit, for delta/omega via the exact
scaled series with the deficit bounded by its bracketed 800-bit value
plus logarithmic growth):

| size | n | gamma | delta | omega |
| --- | --- | --- | --- | --- |
| 50 B | 400 | 1.0863–1.0865 | 1.0416–1.0424 | 1.0423–1.0426 |
| 100 B | 800 | **1.0669–1.0673** | **1.0231–1.0234** | **1.0221–1.0222** |
| 200 B | 1600 | ≈ 1.0563 | ≈ 1.013 | ≈ 1.012 |
| 2 kB | 16384 | ≈ 1.0447 | ≈ 1.0017 | ≈ 1.0014 |
| ∞ | — | 1.043017 | 1 | 1 |

Decomposition at 100 B, gamma: of the 6.7%, 4.30% is the asymptotic
sibling-collapse tax, ~1.9% the universal finite-size term
(`1.5·log2 n + 0.74 − 1.04 ≈ 14.2` bits of `H`; the census confirms
the closed-form decomposition to a tenth of a bit), ~0.42%
nonnegativity (~3.1 bits). For delta/omega the whole distance is
finite-size (`Θ(log n)` bits) plus a sub-half-bit nonnegativity
deficit.

Cross-code coverage: the counting metric can also be read as "how
many versions fit in the budget" — at 800 bits gamma's canonical
family has `H ≈ 749.7`, delta's `≈ 781.8`, omega's `≈ 782.6`:
delta/omega cover ~2³² more versions in the same worst-case budget.
The covered sets are incomparable (§1), so this is a statement about
counts, not about any particular version's cost — §8.

**Bytes.** Stored size is `⌈n/8⌉` bytes with the pad enforced
canonical, so a `B`-byte budget covers exactly the streams with
`n ≤ 8B`: the byte-denominated comparison is the table at `n = 8B`
(rows chosen accordingly), and byte-quantizing both sides shifts the
gamma 100 B ratio from 1.067 to `100/94 = 1.064` — under half a
percent, in the code's favor.

**Framing (b) divergence [derived].** For versions with `ℓ` plateaus
and heights uniform in `[0, 2^w)`: the family counts
`≈ Catalan(ℓ−1)·2^{ℓw}` (`log2 ≈ ℓ(w + 2)`), while gamma's worst
case is `(2ℓ − 1) + ℓ(2w + 1)` (every payload can land in the top
bucket), ratio `→ (2w+3)/(w+2) → 2` as `w` grows. Delta's worst case
on the same family is `ℓ(w + 2⌊log2(w+1)⌋ + 2)`-ish: ratio `→ 1`.
Same codes, same floor definition, different family: the honest
number is framing-dependent, which any published claim must carry.

## 8. What this does and does not say about the code choice

The counting verdict is unambiguous: **delta and omega are
asymptotically floor-tight and three times closer to the floor at
100 B than gamma**. But the counting metric rewards exactly one
thing — covering many versions per worst-case bit — and the versions
delta gains are giant-value streams (the measure at delta's
criticality concentrates on few-leaf, huge-payload trees, §4).
Whether those are the versions that occur is the *workload* question,
explicitly out of scope here and owned by the companion
value-histogram study. The pointwise costs frame it [derived]:

| v | gamma | delta | omega |
| --- | --- | --- | --- |
| 0 | 1 | 1 | 1 |
| 1–2 | 3 | 4 | 3 |
| 3–6 | 5 | 5 | 6 |
| 7–14 | 7 | 8 | 7 |
| 15–30 | 9 | 9 | 11 |
| 31–62 | 11 | 10 | 12 |
| 63–126 | 13 | 11 | 13 |
| 1000 | 19 | 16 | 17 |
| 10⁶ | 39 | 28 | 31 |

Gamma is better or tied on every value below 31 — the zigzag range
covering deltas in `[−15, +15]` — and loses steadily above. If the
histogram study confirms that skyline payloads under organic
normalization are overwhelmingly in gamma's winning range, gamma's
4.3% counting tax is the correct price for pointwise wins where the
mass is; if payload mass turns out to sit in wide buckets (large
first-leaf absolutes on long histories are one candidate), delta's
numbers here become decision-grade. The two instruments must be read
together; neither alone decides.

## 9. Proposed docs claim, and recommendation

Two-part recommendation:

1. **Do not land a rustdoc redundancy claim while the value-code
   question is open.** Stating "within 4.3% of the floor" in the
   module docs the same season the code might change would churn
   user-facing prose; the design doc (this file) carries the numbers
   until the decision settles.
2. **If gamma is confirmed**, add a short `# Redundancy` section to
   `version/skyline.rs` (after `# Canonical form`); the crate already
   makes the qualitative claim (`codec/gamma.rs`: "close to minimal
   for this distribution") that this makes precise. Proposed text,
   self-contained per the hard rule (no design-doc citation from
   code):

   > Among the versions whose canonical stream fits in `n` bits, any
   > injective coding must spend at least `H(n) = log2(count)` bits on
   > some member; this coding's worst case exceeds that floor by 4.3%
   > asymptotically, and by about 7% at 100-byte streams (derived: the
   > canonical grammar's growth constant is `log2(1/x)` for the
   > positive root of `1 − 2x² − 2x³ − 2√2·x⁴ = 0`, cross-checked by
   > exact census; topology-plus-gamma is itself Kraft-complete, so
   > the entire asymptotic gap is the code space canonical form
   > excludes — the price of byte equality being value equality). The
   > floor is relative to the sets this coding covers: against
   > versions with uniformly large heights the ratio would instead
   > approach 2, the deliberate trade for logarithmic cost on the
   > small deltas normalization produces.

   If a shorter form is wanted, the last sentence is the one that
   must survive: the framing caveat is what keeps the 4.3% honest.

## 10. Committed pin (specified for a later agent; needs cargo)

The strongest cross-check binds this document's census to the real
validator. Add to `version/skyline/tests.rs`:

- **Census pin**: for every `n ∈ 1..=20`, iterate all `2^n` bit
  strings, run the strict validator (`skyline::validate`) on each,
  and assert the per-`n` accept counts equal the committed table
  `f_con(1..=20) = [0, 1, 0, 2, 0, 4, 1, 8, 6, 18, 17, 48, 52, 124,
  160, 342, 488, 984, 1521, 2874]` (~2M validations; well under a
  second in release — if slow under nextest debug, cap at 18). Doc
  comment: "the number of canonical skyline streams at each exact bit
  length, independently derived by exhaustive census of the coding
  grammar (topology bits + gamma payloads under the three
  canonical-form rules); pins the validator's accept-set cardinality
  itself, not just individual accept/reject cases."
- Optionally, the byte-level twin: all byte strings of length ≤ 2
  through `Version::decode`, accept counts per byte length must equal
  `[#{n ∈ (0,8]}, #{n ∈ (8,16]}]` from the same table — pinning the
  padding bijection.

This is deliberately a *count* pin: any drift in the accept set (a
lost reject, an over-eager reject) moves a committed integer even if
every existing point-test still passes. The delta/omega columns are
hypotheticals with no in-tree encoder; they get no code pin.

## 11. Deferred

- `C = 256` brackets at `n = 1600` (`npclamp <code> {lower,upper}
  1600 256`): ~90 s each but ~5 GB peak (the lazy target-array span);
  re-block the excess dimension or run with memory headroom. Effect:
  upgrades the 1600 rows from [fitted] to [bracketed]; every fit
  computed so far sits inside every bracket computed.
- Tighter delta/omega brackets, if wanted: replace the linear clamp
  with a log-scale certified floor/ceiling (state = `⌊log2 h⌋`-band,
  ~`n` states), which keeps giant descents inside the sub-family;
  the linear clamp at `C = 256` was already tight enough (≤ 0.3 bits)
  that this was not needed.
- The §10 Rust pin (needs cargo).
- The companion workload histogram study (owned elsewhere; §8).
