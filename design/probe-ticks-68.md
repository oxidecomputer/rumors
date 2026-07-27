# Probe 68: `ticks(n)` — the fused multi-tick

Measure-and-design probe (branch `tickby-68`, never merges). Product: a
verified equivalence hypothesis, deterministic prices, and a design
sketch for a public `ticks(n)` operation ("skip forward by n events")
plus what it implies for the batch API.

Owner intent (Finch, 2026-07-27): fuse tick runs in batches; add a
`ticks(n: <unbounded natural>)` computed efficiently rather than
iteratively. Mid-probe ruling: the operation is named `ticks` (not
`tick_by`), and it mirrors the full `tick` surface (`Party::ticks`,
`Version::ticks`, `Clock::ticks`, and the batch twins).

## Verdict: TRUE, with one structural refinement

`n` sequential `tick()`s at a fixed party are computable in **at most
two fused walks plus one splice** — never `n` walks — byte-identical to
the iterated public tick. The owner's exact one-walk form ("grow
increment +n instead of +1") holds verbatim on the grow branch, which is
the steady state; the refinement is the fill branch:

- **`fill(i, e) = e`** (fill cannot simplify the tree): ONE fused walk
  records the route, and one `+n` splice registers all `n` events.
  Exactly the hypothesized one-walk form.
- **`fill(i, e) ≠ e`**: the first tick is the fill output; the remaining
  `n − 1` events are grows on that output, whose route needs a SECOND
  fused walk — the first walk's route probe dies at the divergence, and
  the route over the changed tree is a different fold. Fusing the route
  DP into the builder's output stream would recover one walk here, but
  it couples the DP to the builder's collapse decisions; the second walk
  costs one more `O(input)` pass, at most once per `ticks` call, and is
  the recommended landing shape.

So: `ticks(n)` = `fused_fill`; on `Changed`, run `fused_fill` once more
on the output (which must report `Unchanged` — fill is idempotent) and
splice `+(n−1)`; on `Unchanged`, splice `+n`. `n = 0` is the identity,
`n = 1` degenerates to today's tick.

### The structural argument

Byte identity reduces to semantic equality because canonical skyline
streams are unique per value. Three lemmas carry the semantic claim; the
paper's `event = fill if it changed, else grow` does the rest.

**Lemma 1 — fill is idempotent** (`fill(i, fill(i, e)) = fill(i, e)`).
By induction on the paired tree. The full arm collapses to `Leaf(max)`,
a fixpoint. The shortcut arms raise the owned leaf to
`x = max(max(el), min(fill(ir, er)))`; on the second pass the owned side
is `Leaf x` with `x` already ≥ the (unchanged, by induction) sibling
minimum, so the raise re-derives `x`. Normalization's min-lifting shifts
values uniformly and fill commutes with a uniform shift. Hence at most
the FIRST of the `n` ticks takes the fill branch.

**Lemma 2 — grow preserves fill-fixedness.** On a fill-fixed tree every
fully-owned region is a single leaf, and at any node whose id has a full
child (a shortcut-arm node) that full child's leaf costs
`(0 expansions, d+1)` — strictly cheaper than any site in the sibling
subtree (depth ≥ d+2, or expansions ≥ 1), with the tie rule unable to
flip it (a `(1, 1)` id node is non-canonical). So the route never enters
the non-full sibling of any arm node, meaning every raise minimum
`min(fill(ir, er))` is computed over a grow-free region and cannot move;
the grown leaf itself only rises (`a ≥ min` stays `a + 1 ≥ min`); and no
fully-owned multi-leaf region is created (an expansion chain's owned
terminal is a single leaf). Fill stays the identity after every grow, so
ticks 2..n are ALL grows.

**Lemma 3 — the grow site is stable and increments compound.** Grow's
cost `(expansions, depth)` is a function of the `(id, event)` topology
alone — it never reads a leaf value (`oracle.rs`'s `grow` arms make this
visible). A free increment changes no topology: a collapse would need
the grown leaf to rise into equality with a sibling leaf it was strictly
below, and fixedness (`a ≥ min(sibling)`, Lemma 2's inequality) forbids
that; min-lift re-anchoring would need the grown leaf to have been the
strict unique minimum below a sibling, forbidden the same way. So `k`
sequential grows re-derive the identical route (deterministic
tie-breaking) and compound `+k` at one leaf. The expansion case: the
chain was chosen cheapest, so no zero-expansion site existed before it;
after it, the chain's terminal is the UNIQUE zero-expansion site, and
the remaining `k − 1` grows free-increment it — the terminal fresh leaf
simply carries `k`. Either way, one splice with `+k` boundary arithmetic
(grown code `+k`, preorder successor `−k`, chain terminal `k`) equals
`k` sequential grows.

### Empirical verification (all committed on the branch)

`crates/before/src/version/skyline/ticks_probe.rs`, prototype `ticks` +
`grow::emit_by` (the `+k` splice, `crates/before/src/version/skyline/grow.rs`,
`#[cfg(test)]`; production `emit` untouched so the differentials compare
against the real splice). All pass first-run; comparisons are stored-form
byte equality through the public tick:

- `ticks_matches_iterated_ticks_arbitrary` — proptest, arbitrary
  normal-form (version, party) pairs incl. beyond-`u64` magnitudes,
  n ∈ {0, 1, 2, 3, 7, 64}.
- `ticks_matches_iterated_ticks_shapes` — the adversarial shape corpus
  (4 event shapes × 3 scales × {4 id shapes × 3 scales, the bushy
  expansion id}), n up to 1000.
- `ticks_covers_fill_changed_branch` — deterministic fill-branch witness
  (full owner over a bushy tree), n up to 1000.
- `fill_is_idempotent`, `grow_branch_is_absorbing` — Lemmas 1 and 2
  pinned directly through the fused walk's own verdicts.
- `emit_by_one_matches_production_emit` — the probe splice at `k = 1` is
  byte-identical to production `emit` on the same route.
- `ticks_composes_at_wide_n` — beyond any iterative reference:
  `ticks(2^100)` twice = `ticks(2^101)`, and
  `ticks(2^100 + 1)` = `tick ∘ ticks(2^100)` (the wide arm seamed to
  ground truth).
- `ticks_from_empty_is_the_counter` — closed form:
  `ticks(123456789012345)` on the empty version renders as
  `"123456789012345"`.

The oracle envelope is respected: the recursive oracle is never handed
large n; the iterative PUBLIC tick is the reference at n ≤ 1000, and
wide n is covered by the composition law plus closed forms.

## Pricing

Today's k-tick batch: `Batch::tick` = `Version::from_bits(fill::tick(…))`
per call — `Version::tick` and `Clock::tick` route through the same line.
"Commits as it runs" is mechanically: every tick pays one full fused
walk over the CURRENT stream, one splice, and one materialized stored
stream; a k-run costs k walks, k splices, k intermediate streams, and
there is no fusion of any kind today. Confirmed by the meters: the
sequential column below scales ~linearly in k (slightly above on shapes
whose leaf codes widen as heights grow).

Deterministic meters only (packed-stream scan bits, accumulator digit
touches, traversal steps; no wall-clock), `ticks_pricing_table` in the
probe module, run with:

```
cargo nextest run -p before --features scan-meter,limb-meter ticks_pricing --no-capture
```

| shape   |   k | seq scan | seq limb | seq steps | ticks scan | ticks limb | ticks steps | scan ratio |
|---------|----:|---------:|---------:|----------:|-----------:|-----------:|------------:|-----------:|
| organic |   8 |    1 008 |      234 |       232 |        106 |         24 |          29 |        9.5 |
| organic |  64 |   10 924 |    1 914 |     1 856 |        118 |         24 |          29 |       92.6 |
| organic | 512 |  111 672 |   15 354 |    14 848 |        130 |         24 |          29 |      859.0 |
| dense   |   8 |   11 112 |    3 677 |     5 184 |      1 379 |        457 |         648 |        8.1 |
| dense   |  64 |   90 326 |   29 437 |    41 472 |      1 385 |        457 |         648 |       65.2 |
| dense   | 512 |  734 748 |  235 517 |   331 776 |      1 391 |        457 |         648 |      528.2 |
| wide    |   8 |   19 666 |    1 367 |     1 276 |      5 528 |      1 087 |         424 |        3.6 |
| wide    |  64 |  153 330 |    3 999 |     9 228 |      5 534 |      1 087 |         424 |       27.7 |
| wide    | 512 |1 236 250 |   25 055 |    72 844 |      5 540 |      1 087 |         424 |      223.1 |

Shapes: `organic` = a four-share public-API gossip world (forks, ticks,
joins); `dense` = `meter::dense(64)` × `nested_full_id(64)`; `wide` =
`meter::wide_tail(256, 32)` × `nested_left_full_id(32)`. Every cell's
fused result is asserted byte-equal to its sequential twin.

Reading the table:

- **Sequential cost is Θ(k · input)**; fused cost is flat in k. The
  savings factor is ≈ k (organic exceeds k because sequential ticking
  widens the stream it must re-walk; wide sits below k at these k
  because the fused side's one walk of the very wide input dominates
  until k grows further).
- **The only n-dependence is the gamma width of the boundary codes**:
  the fused scan column moves +12 bits per 8× in k on organic (two
  codes carry k: the grown delta and the `−k` repair; gamma grows 2
  bits per doubling → 2 codes × 6 bits) and +6 on dense/wide (one code
  reaches k there). Limb touches and steps are exactly flat — at
  machine-word k the increment is one digit fold.

**Cost shape confirmed**: `O(|v| + |p| + log n)` time and space — the
`log n` is the gamma code of n in the output (and, for astronomically
wide n, the `O(limbs(n))` boundary arithmetic, priced by the output
code's own width). Fill-branch inputs pay one extra `O(|v| + |p|)` walk.

## Design sketch (nothing here lands from this branch)

### API: the carrier of the unbounded n

Constraints: no dashu types in the public API; the crate already has
unbounded naturals behind opaque public types (`Rank`) and decimal text
parsing for unbounded values.

Candidates:

- **`n: u64`** — covers every physically clocked count, but the crate
  stores unbounded heights and parses unbounded text, so a version
  legitimately reaches heights ≥ 2^64; a skip-forward API that cannot
  express them undercuts the crate's own stance, and widening later
  (`u64` → wider) is a breaking signature change. Rejected as the
  primary form.
- **`n: u128`** — same widening critique one word later. Rejected.
- **`n: &str`** — stringly-typed numerics, runtime parse errors on a
  numeric argument. Rejected.
- **`Rank`** — already public and unbounded, but it is a dyadic-rational
  *valuation* (an area), not a count of events; wrong denomination.
  Rejected.
- **A public opaque count newtype** — RECOMMENDED. Working name `Ticks`
  (`Count` if the type/method echo with the `ticks` methods reads badly
  in review): a `Base` inside, construction via `From<u64>`,
  `From<u128>`, and `FromStr` (decimal, reusing the codec's text path
  for unbounded naturals; also `TryFrom<&str>` via that). The methods
  take `n: impl Into<Ticks>`, so call sites read `v.ticks(&p, 5u64)`
  and the type only appears when someone genuinely carries a wide
  count. Compiler-caught, no dashu leak, widening built in, `Rank`
  precedent for an opaque numeric.

Surface (per the mid-probe ruling, mirroring `tick`'s five entry
points):

- `Version::ticks(&mut self, party: &Party, n: impl Into<Ticks>)`
- `Party::ticks(&self, version: &mut Version, n: impl Into<Ticks>)`
- `Clock::ticks(&mut self, n: impl Into<Ticks>) -> &Version`
- `batch::Version::ticks(&mut self, party: &Party, n: impl Into<Ticks>) -> &mut Self`
- `batch::Clock::ticks(&mut self, n: impl Into<Ticks>) -> &mut Self`

`ticks(0)` is the identity (the sequential semantics' empty run);
document it rather than reject it — folds and replay drivers hit zero
naturally.

### Batch fusion: recommend NOT building it

Transparent run-collapse (Batch accumulates a pending `(party, count)`
and flushes on reads, other ops, and drop) is semantically feasible:
the `&mut` borrow means mid-batch state is only observable through the
batch, so flushing before every read preserves the documented
"commits as it runs" contract observably. But it buys nothing `ticks`
doesn't already give a caller who can say what they mean, and it costs:
hidden divergent state in a type whose contract is *no divergent
state*, a flush in `Drop` (nontrivial work during unwind, no error
channel), a flush check on every read path, and a boundary analysis
forever after (what fuses: maximal same-party tick runs; what forces a
boundary: a different party's tick — ticks by different parties do not
commute through the tree — any join/meet, snapshot, comparison, drop).
That is machinery whose justifications are all internal to itself: the
scaffolding-audit tell. `batch.ticks(&p, n)` IS the fusion, spelled at
the call site. If a future profile shows an unfusable one-at-a-time
caller, revisit with that evidence.

### Landing shape

Generalize `grow::emit` to take the increment (`k: &Base`); the
production tick calls it with 1 — one splice path, no probe/production
twin, and the probe's `emit_by(1) ≡ emit` pin retires with the merge.
`ticks` is the two-branch conditional over `fused_fill` exactly as the
probe's `ticks` function; `tick` remains `ticks(1)` or stays as-is
(zero-cost either way — same walk, same splice).

### Landing obligations (the ratchets will demand these)

1. **Oracle transcription**: `oracle::Version::ticks(party, n)` as the
   definitionally honest `for _ in 0..n { tick }` — the paper has no
   fused form, so the oracle iterates. Envelope implication: the
   oracle-facing differentials must cap n (small n only, both because
   the oracle iterates in O(n·tree) and because it recurses natively);
   wide-n coverage lives impl-side — the composition law
   (`ticks(a) ∘ ticks(b) = ticks(a+b)`), the `+1` seam to a single
   ground-truth tick, and closed-form witnesses (the counter identity).
   State the n-cap in the oracle's operating-envelope doc.
2. **Triangle rows** for `ticks` (impl / recursive oracle / semantic
   oracle), small-n per the envelope; the semantic oracle states it as
   n applied events.
3. **Board rows**: `ticks` cells on the tick-designated families (the
   nested-full × dense cross, the wide memo cross, the benign control)
   with scan/limb/heap envelopes, plus a **liveness floor** (a dead
   meter passes any ceiling) and the flatness claim as a committed
   **shape-over-point check**: two n points (say n and 8n) whose cost
   difference must equal the gamma-width delta within a pinned bound —
   so "O(input + log n)" is a check, not prose.
4. **complexity_claims entry**: `O(|v| + |p| + log n)`, provenance
   *measured* (this probe's table; re-measure at landing — never
   transcribe a probe's constants).
5. **Exhaustive small scope**: `ticks(n)`, n ∈ 0..=4, against the
   iterated tick over the exhaustive corpus — a total check on both
   branches.
6. **Op-trace vocabulary**: the organic-history generator gains a
   `Ticks(n)` op (small n weights) so every downstream differential
   sees fused ticks inside histories.
7. **Fuzz**: the fuzz targets' op vocabulary gains `ticks` — unbounded
   n is safe impl-side (cost is O(input + bits(n))), but any
   oracle-differential fuzz leg needs the same n cap as the triangle.
8. **Benches**: a `ticks` point in `benches/version.rs` (and the board
   bench if its policy is every-public-op), through benchjudge.
9. **Docs**: rustdoc with the Complexity section and the
   n-denomination stated; `Ticks`' own docs (construction, FromStr,
   the saturation note on `min_ticks` — it already documents
   saturating-at-`u64::MAX`, which wide `ticks` now makes reachable in
   two calls); `just readme` if crate-level docs move; doctest examples
   on all five surfaces.
10. **Proptest seeds**: commit whatever regression files the new suites
    mint.

### Wire format: NO implications (verified)

`ticks(n)` emits only canonical skyline streams that n sequential ticks
already produce — byte-for-byte, which is exactly what the probe's
differentials assert. No new code shapes, no new wire vocabulary, no
protocol version, no snapshot re-acceptance anywhere in the workspace.
The only wire-adjacent fact is benign: outputs of `ticks(2^100)` have
leaf codes wider than any u64 history could reach, and the codec already
handles unbounded magnitudes everywhere (the wide-magnitude generator
families exist precisely to pin this).

## Probe artifacts

- `crates/before/src/version/skyline/ticks_probe.rs` — prototype +
  differentials + pricing driver (all `#[cfg(test)]`).
- `crates/before/src/version/skyline/grow.rs` — `emit_by` (`#[cfg(test)]`),
  the `+k` splice; production `emit` byte-untouched.
- Reproduce: `cargo nextest run -p before ticks_probe` and the pricing
  command above.
