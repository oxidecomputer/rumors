# The constants frontier: pushing per-bit costs below the hardened baseline

Status: ideation of record, 2026-07-27 — candidate directions for a
future constants campaign, none committed. The measured basis is the
P5 closeout judge table (quick sampling, tip 61c068ca, this machine,
release profile): every cost below is that table's ×4-scale median
divided by the cell's own packed-byte denominator, and every yield
estimate is labeled as such — an estimate, to be refuted or confirmed
by a probe before any commitment. Probe results land in place as
dated amendments at the direction they decide. The asymptotic
campaign's rules stay in force: instruments before cures, probe
first, byte-identical wire format, and the trust model unchanged.

## 1. Where the baseline sits

The public surface today, in ns per packed input byte (÷8 for
per-bit), groups into tiers:

| Tier | Ops | ns/B (median) | Distance from floor |
| --- | --- | --- | --- |
| 0 | encode, hash, fork | below the 10 µs judgment floor (encode ≈ 0.1 ns/B) | at it — encode is the at-rest form |
| 1 | decode (all outcomes), cmp, text parse/display, party predicates | 2–25 | ~4–8× a bit-serial validating scan |
| 2 | tick, join, meet, project, without, own_version, rank, min_ticks | 40–135 | est. 2–5× constant headroom |
| 3 | distance, lag, recv, sync, worst-shape tick | 200–460 | the most recoverable multipliers |
| — | version_join_all (n-ary fold) | 600–1,100 | constant, linear, unpriced potential |

The floor for every operation is "read each input bit once": the
packed form has no random access — positions, extents, and values are
implicit in the gamma/tag codes — so a structural walk pays a
sequential parse by construction. That reading cost is
representation-inherent, deliberately traded for compactness and
memcpy-class encode. **Interior operations validate nothing**:
canonicality is enforced once at the decode/parse boundary, and
everything interior walks trusted bits. Tier-2/3 costs are reading,
arithmetic, and emission — there is no validation to shave.

What is already owned by the standing ledger, distinct from the ideas
below: the render merge re-fold (the one surviving superlinearity,
display × mirror-wide limb e ≈ 1.55; anchor-web discipline is the
named cure candidate), the display finalize materialization (14–33
B/B heap), the builder capacity-phase allocator artifact, and the
n-ary fold's marginal (~1.15) and constant. Those are cures with
mechanisms already named; this document is about the frontier beyond
them.

## 2. Candidate directions

Ordered by estimated yield per unit of risk. Each entry names the
mechanism, the seam it must respect, and the probe that decides.

### 2.1 Word-parallel code reading

Mechanism: payload gamma decode is already word-parallel
(`decode_int_window`: one 64-bit big-endian window plus a
`leading_zeros` settles a whole code); the bit-serial remainder is
the topology/tag bit reading in the walks and the per-code window
re-loads. A word-at-a-time reader — tag runs taken from whole
windows, window loads amortized across codes — is the remaining
headroom. Pure readers (decode, cmp, predicates) have no seam at all
and can adopt it wholesale.

The seam, for the walks: the fused tick's route fold reads each
skipped id subtree's tag bits per-bit during skips (the A2
interaction). The standing decision — keep per-bit rather than trade
a correctness seam for a scan constant — was correct for a skip-only
change. A word-parallel *route fold* (folding tag bits from a whole
word of skipped stream at once) is designable and would lift the
decision's premise; it must keep the five tick-and-flag differentials
green and is exactly the kind of change the byte-identity suite makes
safe to attempt.

Probe: convert `Version::partial_cmp`'s reader alone (no emission, no
seam), pin the board row before/after, and read the yield off one
diff.

**Amendment (2026-07-27, probed): the pure-reader estimate is
refuted; verdict adopt-with-conditions.** The probe (branch
`probe-cmp-56`, fuel instrument) measured the reader swap on `cmp` at
−22.4% — 165.5 → 128.4 fuel/bit at 1000 bits, slope unchanged — and
the decode band did not move. The original estimate (decode 22 →
under 8 ns/B) priced a bit-serial payload decode the tree does not
have: with payload gamma already word-parallel, a reader swap wins
only the topology-bit reads and the amortized window loads, and the
residual `cmp` constant is sweep bookkeeping (path stack, accumulator
folds, the per-leaf step) that no reader touches. `dsi-bitstream`
fitness: FIT — its big-endian bit order equals our Msb0, MSRV is 1.85
exactly, and the adapter is ~40 safe zero-copy lines with the one
`ptr::read` off the read path — under one mandatory condition: its
`read_gamma` caps at 2^64 guarded by `debug_assert` only (a silent
mis-decode in release), so a wide-arm wrapper is required (unary
prefix via `read_unary`; k ≥ 64 fills the `UBig` from word chunks),
with witnesses pinned at k = 63, 64, 65, and ~100. As a constants
cure alone the crate under-clears — a hand-rolled window extension to
the topology bits could capture most of the 22% dependency-free; the
case for adopting it is dissolution of hand-rolled coding surface.
Write-side impedance (word sinks against the byte-backed stores) is
real; prototype before committing. Future-protocol note: inverting
the internal-node topology bit would make descend a single
word-parallel unary read. Taken 2026-07-27 (#67): flag inverted and
the read side adopted wholesale inside the same unshipped revision.

### 2.2 Fusing the composites

Mechanism: `recv` runs decode + join + tick as separate walks over
the same stream; `sync` similarly. The fill+grow fusion showed the
shape: one walk, per-arm work merged, output built once. Composites
paying ~3× their parts' scans is the genre tick fusion already beat.
Estimate: worst-shape recv 455 → ~150 ns/B.

Seam: the fused walk must preserve each part's error surface exactly
(the rejection rows pin those paths byte-for-byte), and the changed
flag/hand-back contracts ride the existing differentials.

Probe: fuse decode+join only (the pair with no grow branch), measure,
then decide whether the third stage joins.

### 2.3 Scratch reuse and exact pre-sizing

Re-scoped 2026-07-27 (the presize-61 probe): pre-sizing from operand
sizes is landed everywhere it has a site — every output builder
reserves once at construction — and the capacity-phase red is owned
by the one op whose output is not size-derivable: projection output
is mandatorily Θ(|v|·|p|) bits on a Θ(|v|+|p|) input, so the ev+id
reserve under-runs by the output/input ratio (45–119 at the board's
probe points) and the builder's doubling chain, anchored at the input
size, steps across the default probe pair. The amplification ledger's
§12 (2026-07-27 capacity-phase finding) carries the fitted law and
the priced cures — a size pre-walk feeding one exact reserve, or a
segmented output — with the disposition open. What remains of this
direction: (a) a caller-opaque reusable scratch (thread-local or a
`&mut Scratch` parameter on the sessions that already thread state),
amortizing builder allocation across calls.

Seam: none semantic; the meters price it immediately (heap column
movement must be annotated).

Probe: (a) on the walks a session repeats, board diff as the verdict.

### 2.4 Rank extraction without materialized intermediates

Re-scoped 2026-07-27 (owner correction): dashu already stores small
values inline in a single word, so there is no missing small-value
fast path at the bignum layer. The headroom in `distance`/`lag`
(200–400 ns/B vs. rank's own 55) is *above* dashu: the extraction
materializes intermediate `Base` values per site — allocate,
normalize, compare, drop — where the anchor-web discipline that cured
tick's limb work holds the same quantities as differences and
watermarks without materializing them. Extending difference-coding
into the rank/distance/lag folds is the direct analogue of the I2
invariant applied to one more walk family.

Seam: `Rank`'s exact-value semantics (the drift-charged Abel
accounting) must hold to the digit; the freeze-discipline pins are
the guard.

Probe: count materialized `Base` constructions per site in the lag
walk (a one-line counter), then prototype the watermark form on
`distance` alone.

**Amendment (2026-07-27, probed): hypothesis confirmed in full.** The
probe (branch `probe-rank-57`, a deterministic construction counter
co-read with the limb meter over the board corpus) found `distance`
running ≈ 4.5× rank's `Base` constructions per byte (`lag` ≈ 2.5×)
and 2.7–7.0× its limb work — bracketing the observed 200–400 ns/B
band shape-for-shape. On the separating families, ~78% of the
constructions and ~86% of the limb work belong to the
emit-two-streams-then-re-rank architecture (dense `distance`: 112,525
limb-ops measured against ~16,008 mandatory). Dissolution path:
distance = ∫|h_a − h_b| and lag = ∫(h_b − h_a)₊, computed as one
co-sweep on the existing accumulator with no lattice value
materialized — expected landing at rank's own ~5.4 limb-ops/B (the
~55 ns/B class). Gating conditions before any cure (instruments
before cures): a two-operand jump-comb separating family (the
cross-stream drift wedge — one input's cheap codes re-arming drift
the other paid for; the freeze-funding certification must extend to
sign crossings of the difference), a concurrent-pair family (the
corpus-of-record pairing leaves the side-switch paths nearly
unexercised), overlay-scale re-derivation of the freeze positions,
and a digit-exact differential pin against both the composed form and
the oracle.

**Instruments landed (2026-07-27):** the two gating families are
committed and pinned — the two-operand jump comb reads the wedge red
(board `version_distance × jump-pair`, limb/touch exponents 1.67–1.93
across both scale pairs, plus the envelope growth floor in
`tests/meter.rs`), and the concurrent pair exercises the emit side
switch at every overlay boundary — so the co-sweep cure has five
committed readings to move (the amplification doc's §12 instruments
entry itemizes them).

**Cure landed (2026-07-27, #66 phase 2):** the fused co-sweep shipped
with the anchored-segment freeze re-derivation (per-operand potential,
every charge naming its deposit; the derivation lives in
`version/skyline/query.rs`'s pair-co-sweep section) and all five
readings moved in one commit — board cell green at both scales,
`DISTANCE_JUMP_PAIR` limb 973,702 → 53,905 at the enforced scale, the
growth floor flipped to a flatness bound, the judge roster entry
retired, the complexity class re-classed linear. The landing
overshoots this item's expectation: distance reads 0.40 limb-ops/B on
the wedge family (rank's own dense constant is ~5.4/B — the wedge
family is wide-code-dominated, so the honest comparison is the board's
per-family cells, all green), and the dissolution deleted the
emit-two-streams-then-re-rank architecture from the distance/lag path
entirely. §12's CURED entry in the amplification doc carries the full
before → after record.

**Accumulator prior-art verdict (2026-07-27, surveyed): keep.** No
maintained external tool provides the accumulator's contract: every
general-purpose bignum crate normalizes on every add, which is
structurally the known-bad plain-sweep artifact the meters already
reject. The representation belongs to a citable tradition — redundant
signed-digit arithmetic (Avizienis, carry-save), Okasaki-style
redundant numerical representations, Kulisch accumulators, and
unsaturated crypto limbs as production precedent — but no packaged
general-purpose form of it exists, and the collapsing sign fold plus
the domination-floor query layer appears novel as an integrated
contract. DECIDED 2026-07-27 (owner): the accumulator is marked for
extraction into its own workspace crate post-campaign — the
dissolution dual, taken — with the tradition cited as inspiration in
its module docs during the legibility pass, where its crate name is
also chosen; the touch-meter instrumentation and the envelope suite's
ceilings and liveness floors move with it under the instrument
ratchet (the moved meter demonstrates it still catches before the
old wiring is deleted).

### 2.5 Branchless sweep arms

Mechanism: the three-arm advance/tie loops in the sweep kernels are
branch-predictor-hostile on adversarial interleavings (alternating
advance directions is exactly what comb shapes construct).
Conditional-move selects / arithmetic arm-selection are the classic
cure. Yield is genuinely unknown — modern predictors are good — so
this is a measure-first curiosity, not a plan. The wall judge, not
the deterministic board, is the deciding instrument (instruction
counts may rise while time falls).

### 2.6 Profile-guided optimization

PGO (and BOLT-style layout) over the bench corpus: boring,
build-system-only, typically 5–15% on branchy walks, zero semantic
risk. The only design question is whether the gate's build times
tolerate a PGO-instrumented profile lane; likely a release-artifact
concern, not a development-loop one.

## 3. Additional thoughts beyond the original list

- **Session-scoped operand caches.** The IdIndex cure was one
  instance of a general pattern: gossip workloads re-run ops against
  the same operands (a peer's party, the local version). A session
  object caching the decoded skeleton (index, extents) of its
  recurring operands amortizes tier-1 reading across tier-2 calls.
  The cache is semantically invisible (keyed by the bytes it mirrors,
  and bytes are identity); the design question is API surface, not
  correctness.
- **Parallel n-ary fold.** `join_all`'s binary-counter fold is
  associative over disjoint slots; rayon-parallelizing the counter
  levels cuts wall-clock (not work) for large n. Off-model for the
  library's current no-runtime-deps posture — recorded as a
  caller-side recipe (the fold is expressible over the public API)
  rather than a library change.
- **Interleaved clock stream.** Clock ops scan the id stream and the
  version stream separately; a single interleaved scan would halve
  pass count for `own_version`/`recv`-class ops. This is a
  representation change — squarely confer-level under the standing
  representation ruling — and the compactness/identity story is the
  cost. Recorded to name it, with the expectation that the answer
  stays no.
- **Denominate before chasing.** Any constants campaign should open
  by re-denominating tier 3 the way C3 re-denominated the text
  pipeline: the 382 ns/B on `distance` may partly be honest
  mandatory work per spelled value, and the honest target is the gap
  between measured and mandatory, not the raw number. The board's
  per-currency floors are the instrument; extending mandatory-work
  floors to the rank family is the first deliverable, before any
  optimization.

### 3.1 The integer-code question, answered two ways (2026-07-27, probed) — ruling: keep gamma

Two instruments, one per metric.

Workload side (branch `code-study-60`, measure-only, its
reconciliation pins exact over 165,834 versions): payload zeros are
27.2% of values in the dynamic regime and 10.5% in the static — no
majority-zero assumption holds — but the mass concentrates small
regardless (P(v ≤ 15) is 85–93%), and gamma wins the data-dynamic
regime that produces ~91% of realistic bytes. The best safe
alternative, zeta₂, buys −0.17% combined bytes — worth nothing
against a protocol change; delta and omega cost +6–9% on realistic
traffic; Rice is structurally disqualified (code length linear in
the value, and arbitrary-width magnitudes are legitimate canonical
streams). The unifying frame: gamma is zeta₁, and the measured shape
parameter sits between 1 (dynamic) and 2 (static).

Worst-case side (the counting study,
`design/before-version-entropy.md`): gamma-as-built is 1.043× the
information-theoretic floor asymptotically — the entire tax is
canonicality pruning, since topology-plus-gamma is Kraft-complete —
and 1.067 bracketed at 100 B; delta and omega are exactly 1×
asymptotically, 1.022 at 100 B.

So the counting metric mildly favors delta/omega while the workload
metric favors gamma, and the pointwise cost table has gamma
better-or-tied on every value below 31 — exactly where the organic
mass sits. Ruling: keep gamma. Revisit trigger: a
static-process-dominant deployment; the move is re-running the
committed study binary at the paper's weights, with zeta₂ the
candidate. The entropy doc's §9 proposed docs claim (a `# Redundancy`
section for `version/skyline.rs`, deliberately not landed while the
code question was open) awaits the owner with the prose review.

## 4. What not to do, and why this is safe to attempt at all

Not doing: trusted-bytes decode (skipping boundary validation for
provenance-tagged input). The entire yield is confined to the
boundary crossing — already tier-1 cheap — and the price is a second,
weaker trust regime under the invariant that byte equality is
semantic equality. Rejected 2026-07-27 (owner concurrence).

Also not doing: anything here before the standing ledger's owned
items (§1) are dispatched, and nothing anywhere without its probe.

The reason this list is attemptable at moderate risk is the
asymptotic campaign's actual product: total byte-identity
differentials over the public surface, the deterministic board with
per-cell floors and movement-annotation discipline, the wall judge
with its roster and known-bad tripwire, and instruction-band
enforcement in the gate. Every idea above lands as pins moving down
through a reviewed diff — or gets refused by the instruments. The
campaign bought the right to tune.
