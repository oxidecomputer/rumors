# Ownership-gated walks: skipping the id-space a party doesn't own

STATUS: PARTIALLY LANDED (2026-08-04, perf-probe campaign). Consumer 1
(`fill`/`tick`/`ticks`, both branches, plus the pre-scan) is landed and
pinned; consumers 2–3 (`masked`, `project`) await an owner decision in
light of the measured findings below. Owner review pending.

## Findings from execution (2026-08-04)

- **The bench corpus's tick pair is not hole-shaped.** Its organic
  party (18,920 id bits against 37,835 event bits) interleaves finely
  with the event tree: unowned regions are overwhelmingly single
  leaves, so the block scan almost never opens and organic tick moves
  ±1% (neutral; paired A/B at the parent). The premise "at the bench
  corpus's shapes most event leaves sit under id-space the ticking
  party does not own" was wrong — ownership fraction is small, but
  scattered rather than blocky.
- **On the hole shape the design targets, the win is large.** The
  probe's `holetick` op (a 26-bit party over a 40,337-bit joined
  version, the small-custody-peer shape): 320µs → 195µs (−39%).
- **Region routing must be free.** Peeking topology flags to size
  regions cost ~2% on the organic corpus; routing on the first
  descent's depth (bits consumed either way; `d1 ≥ 2` opens the block)
  made the gate cost unmeasurable. Single-leaf and leaf-first-pair
  regions stay per-leaf: the block summary's fixed cost (two register
  materializations, one watermark emission) exceeds their freight.
- **The minimum is eager, not lazy.** The summary folds the region's
  minimum as it scans (one extra word-scale fold per leaf) instead of
  the proposed lazy replay handle: the watermark web is a streaming
  structure, and threading deferred minima through it is exactly the
  re-entry-state risk this document names, bought back for one or two
  nanoseconds per skipped leaf. Revisit only if a measured gap demands
  it.
- **The remaining organic tick gap (~2.5x oracle) is per-node walk
  interpretation** — frame stacks, watermark open/close churn,
  per-leaf builder feeds under interleaved ids, signed folds — spread
  with no single dominant term. Reaching parity on interleaved shapes
  is interpreter-level work (packed frame bits, run-splice batching of
  pass-through emissions), out of this document's scope.
- The committed instruments are the `tick_ownership_hole` envelope
  (touch ceiling below the leaf-by-leaf mechanism's measured reading,
  so the skip must engage; scan pinned identical) and the
  `tick_ownership_comb` envelope (single-leaf regions everywhere; the
  gated walk pinned to the ungated walk's own readings). The touch
  meter is quick-register-blind for narrow values, so the hole family
  rides the staircase, whose digit work the meter sees.

## The observation

The recursive oracle's `tick` is fast for a structural reason, not a
constant-factor one: the paper's `fill` recurses only into subtrees the
party owns and returns every other subtree untouched — an unowned
subtree costs one pointer. Our fused fill walk pays full freight on
every leaf regardless of ownership: payload decode, running-height
fold, watermark boundary pushes, route-DP frames, builder feeds — even
under a subtree the party is wholly absent from, where *by the
operation's own semantics* nothing can change (fill collapses only
owned regions; grow sites live only in owned regions).

The packed representation cannot reach O(1) per unowned subtree (no
random access: extent is only learnable by scanning, and the bytes must
move into the output). But it can reach *skip-scan* cost: a tight loop
of word-parallel unary topology reads plus gamma `skip_int`s, folding
each delta into exactly one net-height accumulator — no watermark
traffic, no frames, no builder feeds, no per-leaf branching — with the
region's bits spliced verbatim into the output through the word-staged
builder (memcpy-scale). Measured context: the fill walk's per-leaf
constant is ~15x the validator's today, and the validator itself does
more per leaf (nonnegativity) than the skip loop needs.

This is the same principle two mechanisms in-tree already embody:

- `fill`'s verbatim mode: while emitted output matches consumed input,
  nothing is built; the first divergence materializes the prefix
  wholesale.
- The id builders' block moves: party `sum`/`diff` splice whole
  already-normal subtrees (`copy_reader`, `IdSkylineBuilder::subtree`)
  instead of walking them leaf by leaf.

The proposal extends the principle to every event×id overlay: **a
maximal unowned region is one block, not a leaf sequence.**

## The abstraction: a gated leaf cursor

One new crate-private cursor in the skyline module, shared by every
event×id walk:

```text
GatedLeafCursor::open(ev: &BitsSlice, id: &BitsSlice)
  -> yields, in sweep order:
     Owned(leaf)            // per-leaf, exactly today's LeafCursor items
     Unowned(RegionSummary) // one item per maximal unowned region
```

with `RegionSummary` carrying what the consumers need and nothing more:

- the region's bit range in `ev` (for verbatim splice and for
  re-anchoring: first/last leaf relative depths and last code length —
  the coordinates `continue_verbatim` already takes);
- the entry-to-exit signed height delta (folded in the skip loop; the
  currency is `codec::Int`, so word-scale regions never touch dashu);
- the region's minimum, **lazily**: a closure/replay handle rather than
  an eager value, because only `fill`'s shortcut arms ever ask, and
  only for some regions — the existing `PreScan::replay_max` deferral
  is exactly this shape and can be shared.

The skip loop is one function (`skip_region`), and it is the only new
kernel: unary topology run + `skip_int` per leaf, one `Int` fold, one
optional min-track. Everything else is consumers choosing the block arm.

Boundary discipline: a region summary's delta re-anchors the consumer's
running height exactly as if the leaves had been folded one by one, so
byte-identity with the ungated walk is the acceptance criterion
end-to-end (differential suites unchanged; the fused-walk pins
unchanged).

## Consumers, in order of payoff

1. **`fill` (`tick`, and `ticks` automatically — it is the same fused
   walk, `skyline::fill::ticks`).** Owned regions run today's full
   machinery untouched. Unowned regions: the changed-flag cannot trip
   (fill is the identity there), the route DP records no sites, the
   watermark web takes one folded boundary whose min resolves through
   the lazy handle only if an enclosing shortcut decision asks, and the
   emit side splices the region verbatim. The payoff scales with how
   *blocky* the party's absence is: large on hole-shaped custody (a
   peer owning a vanishing fraction of a wide version), nil on finely
   interleaved organic parties whose unowned regions are single leaves
   (the findings above).

2. **`masked` (`OwnVersion`/`OwnSpan` lazy fused comparisons).** The
   gate is per-operand: over a region where `p` is unowned, `&v / &p`'s
   projected skyline is constantly 0 *whatever `v`'s topology does*, so
   `v`'s cursor may skip whole subtrees under the region (delta-sum for
   re-entry only) while the other operand's side walks normally. The
   overlay refinement legitimately coarsens: interior boundaries of a
   constant-0 region change no pointwise verdict. `OwnSpan` rides the
   same walk.

3. **`query::project` (`OwnVersion::to_version` materialization).** An
   unowned region emits exactly one zero plateau (today it emits one
   zero-delta per elementary interval and lets the builder collapse
   them); the input side skips as in the masked walk. Output shrinks to
   its canonical form directly.

4. **Party ops (id×id).** Already block-structured (`copy_reader`,
   `subtree`); no change, but the doc records them as the pattern's
   precedent so the abstraction's home (one gated cursor, several
   consumers) is legible.

Out of scope, deliberately: version×version walks (`sweep`, `emit`,
`place`, `admit`) have no id gate — their skip analogue is the existing
identity fast-path ladder and early exits. `grow`'s splice emit is
already path-directed.

## Instruments and acceptance

- **Byte-identity oracle**: every consumer's output and verdicts are
  unchanged by construction; the differential suites and wire snapshots
  pin it. Movement is allowed only in resource readings.
- **Scan meter**: the skip loop reads every bit it skips and records it
  (`skip_int`/unary reads already record); scan stays byte-identical —
  the honest signal that no input is left unexamined.
- **Touch/limb meters**: drop further on skipped regions (one fold per
  leaf instead of several, no watermark traffic). Organic-family
  envelope rows re-pin; the adversarial families that pin the full
  machinery (fully-owned inputs — the id gate never opens the skip)
  stand untouched, which is the doctrine's requirement: the worst case
  keeps its instruments.
- **New committed families**: ownership-hole shapes — a party owning a
  vanishing fraction of a wide version (the skip's best case, pinned so
  the win is held), and an alternating ownership comb (maximal
  region-boundary traffic, the skip's worst case, pinned so the
  boundary bookkeeping cannot quietly go superlinear).
- **Liveness**: a floor proving the skip actually engages on the hole
  family (e.g. watermark boundary pushes bounded by owned-region leaf
  count, not total leaf count) — the fast path must be demonstrably
  taken, per the meters' liveness discipline.

## Cost model and expectation

Per skipped leaf: one unary read share + one decode-plus-two-folds
(≈ validator cost, measured ~9 ns/leaf at bench shapes) versus the
per-leaf fill constant (~54 ns/leaf at n=8192 after the word-valued
payload round). Measured on the landed consumer: hole-shaped custody
−39% wall (320µs → 195µs), finely interleaved organic parties neutral
(single-leaf regions; the gate never opens and costs nothing closed);
fully-owned inputs unchanged by construction. The masked/project
consumers would gain proportionally to their masks' holes; their walks
are already cheaper per leaf, so the absolute win is smaller still on
interleaved masks — which is the open question their go/no-go decision
turns on.

## Risks

- The skip loop must maintain exact re-entry state (running height,
  builder re-anchoring coordinates). Both have precedents
  (`continue_verbatim`, `scan_subtree`) whose invariants the summaries
  reuse; the differential suites are dense here.
- `fill`'s width-circulation accounting is stated per consumed delta;
  the skip substitutes one net fold per region. The accounting argument
  needs one new paragraph (a region's net fold is priced by the codes
  the skip read), not a new discipline.
- Region-boundary thrash: an adversarial ownership comb makes regions
  one leaf wide, so the gate must cost nothing when it never opens —
  the gated cursor's owned arm must stay exactly today's `LeafCursor`
  fast path. The alternating-comb family pins this.
