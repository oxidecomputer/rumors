# The constants frontier: pushing per-bit costs below the hardened baseline

Status: ideation of record, 2026-07-27 — candidate directions for a
future constants campaign, none committed. The measured basis is the
P5 closeout judge table (quick sampling, tip 61c068ca, this machine,
release profile): every cost below is that table's ×4-scale median
divided by the cell's own packed-byte denominator, and every yield
estimate is labeled as such — an estimate, to be refuted or confirmed
by a probe before any commitment. The asymptotic campaign's rules
stay in force: instruments before cures, probe first, byte-identical
wire format, and the trust model unchanged.

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

Mechanism: the gamma/tag reading throughout the codec and the walks
is bit-serial. A 64-bit-window reader — `count_leading_zeros` to take
a gamma length in one instruction, small tables for tag runs —
processes prefix codes word-at-a-time. Pure readers (decode, cmp,
predicates) have no seam at all and could adopt it wholesale;
estimate: decode 22 → under 8 ns/B.

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

Mechanism: every walk allocates its builders per call; the
capacity-phase artifact on the board is buffer-doubling mid-walk. Two
independent moves: (a) a caller-opaque reusable scratch (thread-local
or a `&mut Scratch` parameter on the sessions that already thread
state), amortizing allocation across calls; (b) exact or
one-sided-bound pre-sizing from the operands' known sizes (an output
of size ≤ n+m is derivable for the joins; the builder can reserve
once). (b) also dissolves the capacity-phase red as a side effect.

Seam: none semantic; the meters price it immediately (heap column
movement must be annotated).

Probe: (b) first — it is a two-line reserve change per builder with a
board diff as the verdict.

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
