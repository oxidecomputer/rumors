# The sync budget: session memory bounded by occupancy, priced by the backend

Status: phases 1–3 landed (2026-07-22, `a3da46c4..4d6ff0f0` on
`link-transport`); phase 4 landed the same day (`b9679a1a..922db57a`),
spec-first — each §2 section carries its landed status, and
transcription deviations from the original spec text are recorded as
dated amendments in place.
Companions: `design/streaming-latency-serialization.md` (the
serialization diagnosis this campaign grew from, amended in place as
the knob evolved). The occupancy mathematics lives inline at the
functions that implement it
(`src/tree/mirror/streaming/window.rs`), with
`examples/envelope_sim.rs` as its certifying tool.

**Model of record**: uniform-hash, authenticated-honest-peer. Content
addresses are uniform 32-byte strings; hostile-peer regimes are
off-model; no argument below rests on adversary economics. Claims are
tagged \[derived\] (premises stated), \[measured\] (instrument named),
\[fitted\] (envelope, tightness stated), or \[decision\].

## 1. What landed

### 1.1 The interface

One optional knob: `Peer::sync_memory_budget(budget_bytes)`, default
`DEFAULT_SYNC_MEMORY_BUDGET` — 512 MiB, **a stated policy choice,
minted from no expression**. What any budget buys is read off the
window the derivation solves for it — the committed trade-off table
renders that solve at the spec BDP — with the closed form
`slowdown(budget, m) ≈ max(1, BDP × E / (budget × (28 + m)))` as the
estimation tier (`E = 4,865 B` the pinned per-scope envelope, §2.4's
landed status records its derivation and pin; 28 B the calibrated
per-message wire intercept; `m` the mean encoded record size; §1.6's
2026-07-24 amendment calibrates the form's band). At the spec BDP of
12.5 MB the default's slowdown-1 crossover is `m* = 51 B`, pinned
against the solve. The budget is a worst-case envelope per session,
never an allocation; concurrent sessions on separate links each carry
their own.

Each session resolves the budget into **static per-height channel
capacities** at handshake time, from the set sizes the two replicas
exchange: the V2 greeting's version frame leads with the sender's
exact O(1) `len()` (eight little-endian bytes — a deliberate wire
change, every V2 snapshot re-accepted byte-verified, V1 untouched).
Channels stay plain bounded queues; the `Link` remains the only
backpressure boundary with runtime semantics. `Protocol::V1` ignores
the knob.

### 1.2 The derivation

Capacities are `max(1, min(K, S(depth)))` per typed height, where
`S` is the depth's integer population envelope under uniform hashing
(`window.rs`, imported from the B0.5 analysis in its sweep-certified
integer forms; envelopes hold jointly at 2⁻⁴⁰ per session
\[derived\]), and `K` is the widest width whose summed charge fits
the budget.

`S` is **pair-based** \[decision, the campaign's load-bearing
correction\]: disputes exist only at *shared prefixes*, so the joint
terms take the product `A·B/256ʲ` of the two exchanged sizes
(deterministically capped by the smaller corpus; the root itself is
jointly occupied only when both sides are non-empty), while the
occupied-slot and per-parent-fan terms take `max(A, B)`, whose
children a reply lists. Consequences pinned by test: symmetric
sessions reduce to the previous `N²`; an empty-side session derives
floor dispute windows — correctly, being all supply, which streams
under `target_message_size`, outside the window.

Degradation is one-directional \[derived, measured\]: a population
beyond its capacity — a set outgrown mid-session, the sub-2⁻⁴⁰ tail,
an off-model key distribution — serializes behind its channel.
Latency, never memory growth, never deadlock: every edge keeps the
one-slot liveness floor the deadlock-freedom argument certifies, and
test builds still run every schedule at that floor.

### 1.3 The wave model, measured

The knee is a throughput ceiling, not a compounding cliff
\[measured, `tests/window_knee.rs`\]: above the binding capacity the
descent drains in capacity-sized waves, marginal cost constant per
message (slopes 0.0072–0.0127 hops/message bracketing the predicted
`2/capacity` = 0.0097), and window stall hides under bandwidth-bound
transfer once the link's BDP-in-messages is at or below the binding
capacity (39 vs 94 hops at 16× capacity). Effective wire cost
calibrates at ~200 B per disputed message \[measured\]. These figures
are configuration-specific: the suite's derived binding capacities
moved between derivations, and
`design/streaming-latency-serialization.md` §5.2's second-wave
amendment records the same claims measured at the earlier
configuration (~5× larger slopes; both honest). This is what
licenses fixed constants in place of per-deployment tuning.

**All wire costs are measured by delay-sweep slope** \[decision,
`ca686533`\]: every timing site runs its shape at two delays and
differences, isolating serialized wire structure from compute. The
harness reports their sum, and single-point division counts compute
as phantom hops — harmless at small divergences, badly diluting at
scale (a 20k-message catch-up read 41 hops; the true wire cost is 7).
The pipe stays fixed across the sweep so a tight pipe's transfer time
survives the slope; scaling it with delay differences the transfer
away.

**Amendment (2026-07-23): wire costs are measured in exact virtual
time; the sweep is retired from every asserting suite.** The
paused-clock harness's virtual component is a deterministic function
of the session shape — compute costs zero virtual time, no thread
outside the current-thread runtime participates, and every pipe
deadline lands on the delay lattice — so
`DelayedWire::round_trip_virtual` reads serialized hops exactly, and
machine load cannot move them (pinned by determinism and lattice
tests in `tests/latency_link.rs`). The sweep's differencing cancelled
the harness-reported wall component only in expectation: under fleet
load the residue read as phantom hops (the catch-up corner observed
at 31 hops against its then-24 bound), and the suites needed
whole-machine nextest isolation, now removed. Exact re-measurement
moved the recorded figures — catch-up 8 hops in either direction
(the sweep read 7 and 5, the spread being wall residue), knee cells
7 below / 23 above the knee at marginals 0.0137–0.0146 hops/message,
stall-under-transfer 37 vs 79 hops — and every noise allowance
tightened: catch-up bound 24 → 12, pipelined bounds 24 → 12
(`window_knee`) and 64 → 24 (`gossip_pipelining`), required knee
growth 6 → 12, wave-model accuracy bands 0.4–2.5× → 0.5–2.0×,
stall-under-transfer to a plain ≤, parity allowance transfer/2 →
transfer/8. The benches keep sweeping: their wall intercept is the
compute figure they exist to show, and load noise averages out
across samples.

### 1.4 The instruments

- **Node census** (`test-internals`): every tree-node handle counted
  through one constructor funnel, clone, and drop — exact concurrent
  residency, read via `testing::node_census`.
- **`tests/window_census.rs`**: isolates window-attributable
  residency by differencing identical divergences under different
  budgets (a session's peak is otherwise dominated by content: both
  generations plus the output tree coexist at the commit join) and
  holds it inside the derived admittance.
- **`tests/window_knee.rs`**: reads derived capacities back through
  `testing::window_capacities`, places divergences astride the
  predicted knee, and pins flat-below / linear-above / stall-under-
  transfer.
- **`tests/window_corners.rs`**: the boundary honesty suite —
  asymmetric catch-up at 8 hops in either direction regardless of
  budget, zero-budget serialization (521 hops at 2k mutual) that
  converges,
  one-byte pipes under floor windows live under the deterministic
  quiescence witness, mid-session growth only serializes, and the
  claims re-verified on a running clock (the virtual model
  overcharges, never undercharges).
- **`examples/window_tradeoff.rs`** → `just window-tradeoff` →
  `src/tree/mirror/streaming/window/tradeoff.md`, compiled into
  `sync_memory_budget`'s rustdoc: the budget × record-size slowdown
  grid — each row's window solved by the real derivation at the
  design corpus, each cell the measured wave form. Deterministic
  arithmetic, byte-compared against the generator in the gate; the
  committed table is the figure of record.
- **`benches/window_wallclock.rs`**: criterion over the same delayed
  pipes on a running clock; cross-checks the virtual figures by eye
  (pipelined cell within noise; serialized cells 20–60 % under the
  conservative wave bound).

### 1.5 Decisions of record

- **Static per-height caps, not byte meters** \[decision\]: channels
  stay dumb bounded queues; runtime backpressure belongs to the
  `Link` alone. The B0.5 meters/`L(N)` division remain earmarked for
  the transport receive-window task, not this one.
- **Exact `len()` over `min_ticks`** \[decision\]: the version's
  event floor over-counts (redactions tick); the replica knows its
  size O(1), so the greeting carries it.
- **Default adopted at 512 MiB and reverted the same day**
  \[decision, `90c50167`/`4d6ff0f0`\]: "4× headroom over the design
  BDP" argues the smaller default suffices; the margin insures only
  against loaded-fabric RTT inflation and fitted-constant slop, whose
  lapse costs small constant factors on heal timescales the covered
  bandwidth makes negligible, while the larger bound is paid in every
  worst-case memory account. The trade-off table keeps a 512 MiB row
  for operators who want the margin.
- **Integer-quantile ripple accepted** \[decision\]: capacities move
  across 256^k set-size crossings by the drift plus ≤ ~¼
  multiplicative ripple from bit-length-granular quantiles — the
  price of keeping the dominance-certified integer forms; the
  smoothness proptest pins that whole-level cliffs (33–50 %) stay
  excluded.

### 1.6 The operator equations (added 2026-07-23)

The wave model closes into a pair of forms fit for capacity planning,
because divergence cancels: stall time is `D × RTT / K` \[measured,
the knee suite\], transfer time is `D × w / bandwidth`, and their
ratio is independent of `D`. With `BDP = bandwidth × RTT`:

> `slowdown ≈ max(1, BDP_messages / K)`, exactly, with `K` the
> derived binding window; and, substituting the large-window
> simplification `K ≈ budget / E`:
> **`slowdown ≈ max(1, (E/w) × BDP / budget)`** and
> **`budget_min ≈ (E/w) × BDP / slowdown`**, with `E/w = 25`.

The default budget is the second form at slowdown 1 on the design
link — an identity, pinned to the byte with the ratio kept as the
exact quotient `E/w = 4,865/200` (plus the flat supply-decode
envelope; see the amendment below). The rounded `E/w = 25` form an
operator applies runs ≲3% above the exact one, in the conservative
direction. The scalar forms hold in the operating regime and degrade
in two known directions, both measured:

- **The near-root band** \[measured\]: `charge(K)` is piecewise —
  windows under a few hundred scopes pay full-fan reference prices
  (`c_q` saturates at 256 near the root), so small budgets buy
  ~3× less window than `budget / E` suggests. The committed
  trade-off table is the record for that regime; the exact form
  (`K` from the derivation itself) stays accurate through it.
- **Corpus and backend growth** \[derived, §2.4\]: `E` is really
  `E(n, f)` — it grows slowly with set size through the children
  quantiles and directly with the backend's `node_bytes`, so the
  pinned `E = 4,865 B` is the design-corpus, in-memory-backend
  evaluation.

Pins: `tests/window_operator.rs` (the
exact form against measured sessions on a bandwidth-limited pipe,
within the knee suite's accuracy band, and the parity direction —
the inverse-form budget measured 94 hops against a 96-hop transfer
bound; the link rate is *self-calibrated* from an unbounded-budget
run because the pipe carries several concurrent streams, so its
aggregate rate is measured, never assumed; the calibration cancels
out of the accuracy band, keeping the pin robust to scheduling
variance).

Two instrument findings recorded for future readers \[measured\]:
on a *latency-only* pipe the observed stall runs ~2–2.5× the
single-wave `2D/K` at deep constriction — concurrent wave systems
stack where no transfer exists to hide under — which is why the
trade-off table's cells are worst-case factors a real link's
bandwidth absorbs; and the delayed-pipe harness's per-stream
capacity understates a session's aggregate rate (supplies ride
several streams), which is what forced self-calibration.

**Amendment (2026-07-23): the default is policy; the equations
re-denominate in record size.** `DEFAULT_SYNC_MEMORY_BUDGET` is a
stated round choice — 512 MiB — minted from no expression; the
derivation chain above becomes documentation. The operator form
carries the record size explicitly: `slowdown(budget, m) = max(1,
BDP × E / (budget × (28 + m)))`, with `E = 4,865 B` pinned by
recomputation and the 28 B per-message intercept pinned by
deterministic byte-count calibration (`tests/dispute_wire.rs`, three
collinear cells). Its inversions answer the three operator questions
— minimum record size at a budget, minimum budget at a record size,
slowdown given both — worked in `Peer::sync_memory_budget`'s docs;
at the spec BDP (12.5 MB, where 1 Gbps × 100 ms and 100 Gbps × 1 ms
coincide) the default's slowdown-1 crossover is `m* = BDP × E /
budget − 28 ≈ 85.3 B`, and u64 corpora serialize at worst ~3.1×,
latency never memory. The ratio pin retires with the ratio (no
quoted `E/w` remains to hold); `DISPUTE_WIRE_BYTES` survives only as
the design-record anchor (`28 + 172 = 200`), with nothing deriving
from it. The trade-off table is re-axed to budget × m and generated
deterministically from the closed form (`just window-tradeoff`,
byte-compared in the gate); the superseded measured
budget × divergence table lives in git history. Sanity, before its
deletion: the closed form's m = 172 column against the measured
table's 50k-divergence column (nearest regime: 62,500 vs 50,000
crossing messages) runs 2–5× above the wall-clock factors with the
same ordering and the same parity knee between 64 MiB and the
512 MiB row — the conservative direction expected of an envelope
held against measurements a real pipe's transfer structure dilutes;
exact agreement is not claimed.

**Amendment (2026-07-24): the table carries the solve's own windows;
the closed form is the estimation tier, its band calibrated.** A
one-shot hop-exact validation run \[measured,
`tests/tradeoff_probe.rs`, ignore-gated; virtual-clock counts,
byte-identical across runs; design corpus; link BDP self-calibrated\]
measured wire-time slowdowns of 1.33–1.45× the closed form's figures
at 10–31 MB budgets (derived windows of 1,015–4,968 scopes), exactly
1.00× at the comfortable cell, and at or inside the wave form
evaluated at the actually derived window everywhere (to within one
hop at the near-crossover cell). Adjudicated the same day: the
trade-off table is now generated from the real derivation — each
budget row's window solved by `Window::from_budget` at the design
session, each cell the measured wave form `max(1, BDP_messages / K)`,
the corpus assumption stated in the header — so the table asserts
exactness where it is exact, and the closed form serves as the
mental-arithmetic tier in `sync_memory_budget`'s prose with its band
stated.

The band, by verified decomposition rather than back-fit \[derived,
recomputed from the solve at the design corpus\]: the real charge is
piecewise affine, `charge(K) = F + K × marginal`. For windows between
the deep-tail and depth-5 populations, `F` = 4.73 MB — 0.21 MB decode
fans, 4.31 MB of root-adjacent stages at full-fan reference prices
(257 scopes × 16,768 B), 0.21 MB of deep population tail — with
marginal = 5,324 B/scope; depth-5 saturation (≈5,500 scopes) lifts
`F` to 7.93 MB and settles the marginal at 4,741 B/scope, 2.5% under
the 4,865 B saturation average, out to the population ceiling. The
decomposition reproduces the measured budget/K gaps: the back-fit
`budget − K × 4,865` at the probe cells (5.20 / 7.01 / 5.66 MB)
equals `F + K × (marginal − 4,865)` — exactly at the first two cells
(K = 1,015 and 4,968 sit below depth-5 saturation, where
F = 4.73 MB is insensitive to the probe session's 2,048 common
messages atop the design divergence), and at the third once `F` is
taken at the session's own corpus (64,548 a side: depth-5 population
5,782, F = 8.10 MB), giving 5.663 against the measured 5.664 MB —
the residue is the solve's sub-scope slack. The closed
form's window error is therefore ~`F/budget` (plus the ≤10% marginal
offset): the slowdown it returns runs ~2× low at 10 MB, ~1.5× low at
16 MiB, within a few percent past ~300 MB.

Re-derived under the solve and pinned
(`default_crossover_matches_the_solve`): the default's slowdown-1
crossover, evaluated self-consistently (corpus = BDP/(28 + m) a
side), is **m* = 51 B** — the closed form's 85.3 B estimate is its
safe-side reading. u64 corpora at the default run ≈4.2× at a
BDP-scale corpus (82,214-scope window), the factor growing with set
size as the window narrows (~14.8× at 10⁷ messages, ~27.5× at 10¹⁰)
\[derived\]. `budget*` for the design record is 304.2 MB by the solve
(the form: ~304 MB — the design point is where the envelope is
pinned); for u64, 1.11 GB by the solve against the form's ~1.7 GB
(population caps thin the deep charge at BDP-scale corpora; the
estimate is conservative in that direction). The 512 MiB and 2 GiB
rows sit at the design session's population ceiling (62,500 scopes),
where the stated corpus is never window-constricted and the
sub-design-record cells are corpus-scale envelopes; the table header
states this.

## 2. What remains: backend-priced budgeting (phase 4, spec-first)

The one dishonesty left in the envelope: `Backend::NODE_BYTES` is a
flat constant, forcing any backend whose node representation scales
with its child table to average over shapes it cannot know, and the
version term inside a placeholder is priced by assumption. The plan
replaces both with exchanged and tracked inputs. Statements below are
the spec of record; transcription drift gets dated amendments here.

### 2.1 The join-size lemma (probe first)

> **Claim to validate**: for `before`'s encoded `Version`,
> `encoded_len(a ∨ b) ≤ encoded_len(a) + encoded_len(b)`.

Executable probe in `before` (proptest over arbitrary version pairs,
including fork/retire-churned party trees) **before** anything leans
on it — probe first, prove second, pin forever. If it holds, joins
of mixed-provenance leaf versions are bounded by summed leaf maxima
and §2.2's scalar suffices. If a normalization corner falsifies it,
the fallback is a per-node aggregate of materialized bound sizes
(which a database backend stores anyway); the plan proceeds either
way, with the lemma's status recorded here.

**Status (2026-07-22): probed and pinned.**
`version::tests::join_encoding_is_subadditive` (churned
fork/send/sync/retire populations) and
`…_subadditive_arbitrary` (unrelated normal-form pairs with
large-base leaves) hold at 20,000 cases each. The pin rides the
public `encode`/`|` surface only, deliberately: the in-flight
`before` representation rework must keep the property — the pin is
the tripwire, not a bystander. §2.2's scalar therefore suffices.

### 2.2 The version-size aggregate

Each replica tracks the **maximum encoded size over its live leaf
versions**, exactly — as a per-node aggregate, not a monotone
scalar, so redaction resizes it down instead of drifting loose
forever \[decision\]: each leaf's value is its own version's
encoded size (in hand at construction), each internal node's is the
max over its children, held as an eager field beside the `leaves`
count. Because mutation is copy-on-write spine rebuilding, the
aggregate is automatically correct under deletion — the rebuilt
path recomputes it at construction, no separate invalidation — and
the root's value is the replica's exact current maximum, O(1) to
read. A database backend maintains the same aggregate as a cached
query, recomputed on subtree deletion. Only leaf versions cross the
wire (supply records; queries and listings carry hashes), so with
§2.1 every version a session can hold — including freshly assembled
interior joins of mixed provenance — is bounded by
`local_max + remote_max`.

**Status (2026-07-22): landed**, in two shapes. The in-memory tree
holds `version_bytes` as the eager field beside `leaves`
(`untyped.rs`; both branch constructors recompute it, and proptests
pin exactness against a recomputed oracle through inserts, argmax
forgets, and the merge path). Then, by direction during
implementation, the aggregate — and `len` with it — moved up to the
**`Node` trait itself**: leaf = one / its own version's encoded
length, parent = sum / max over children, fixed at
`Backend::parent`. That is the auto-derivable propagation rule a
persistent backend keeps as stored fields, it lets the materialized
handshake read both greeting values off the root (no caller-stated
sizes to go stale — the `set_len`/`max_version_bytes` builders were
deleted), and §2.5's suite pins the recurrence per assembled parent.

One precision the spec sentence above glosses: interior ceilings and
floors are joins over *many* leaves, and §2.1's lemma is pairwise —
summing over k leaves is vacuous at tree scale. The pairwise bound is
the *priced* claim, and §2.5's lemma-slack pin is its empirical
guard: a three-party controlled divergence measured every per-node
bound of the reconciled trees at 11 B against a priced 10 + 7 — the
11 genuinely mixed-provenance — with the fallback (per-node aggregate
of materialized bound sizes) still recorded in §2.1 should a workload
ever trip the pin.

**Amendment (2026-07-23): the pin tripped; §2.1's fallback is
adopted.** A constructed adversarial-honest workload broke the
leaf-denominated exchange: 32 parties forked in doubling generations
(shallow intervals), each
stamping a *different* number of times with no cross-sync — ragged
counts defeat frontier saturation — gathered into one replica. Every
leaf stamp stays small while the gathered interior ceilings join all
32 frontiers: measured max bound 41 B against a priced 5 + 5
(`window_census::wide_concurrent_frontiers_stay_inside_the_exchanged_bound`,
the regression pin, red before the fix). The aggregate is therefore
re-denominated over every bound the tree holds: a leaf answers its
own version's encoded length, a branch the max over its children's
values *and its own ceiling and floor encodings*, memoized lazily
beside the bounds themselves (they force the same memos) rather than
eagerly at construction — a session forces bound memos along every
divergent path it walks anyway, so the aggregate rides along, and the
copy-on-write spine rebuild resets exactly the memos a mutation
invalidates. The greeting's shape is unchanged and the two-party
causally-chained snapshot fixtures exchange the same values
byte-for-byte (the latest stamp dominates their interior joins), so
no snapshot moved. Derivation status after the change: a bound one
replica materializes is covered by its own side's aggregate exactly;
a cross-side assembly joins (ceiling) or meets (floor) one
contribution from each side, priced within the exchanged sum by
§2.1's join lemma and its meet dual, probed and pinned alongside it
(`meet_encoding_is_subadditive`, `…_arbitrary`). The one residual
*priced* step: deletion-honoring can prune a side's contribution to
a survivor subset whose recomputed bound is not one that side
materialized (`Unknown::unknown` reassembles pruned spines, and a
subset join can out-encode the full join it was pruned from), so
§2.5's census pin remains the guard for exactly that arm, restated
in its docs.

### 2.3 The greeting carries it

The version frame's body grows from `len(8)` to
`len(8) ‖ max_version_bytes(8)`: the same deliberate-wire-change
procedure as the set size (one isolated snapshot re-accept,
byte-verified version-frames-only, V1 untouched). After it, **every
input to worst-case memory is on the table at handshake time**.

**Status (2026-07-22): landed** as specified: all 17 V2 snapshots
re-accepted after mechanical byte verification (only the version
frame changed; `set_len` word and version bytes preserved, length
header +8, bound zero iff the sender is empty), V1 byte-identical.

### 2.4 The cost function

`Backend`'s pricing becomes a function, not a constant:

```rust
fn node_bytes(children: usize, version_bound: usize) -> usize
```

— the resident bytes of one interior placeholder with `children`
child entries whose two version bounds each encode within
`version_bound`. Contract: an upper bound, monotone in both
arguments (debug-asserted at derivation time; monotonicity is what
keeps quantile evaluation an upper bound). Leaves are deliberately
out of scope: leaf payloads are priced by `target_message_size`,
and the docs state the split once.

> **Amendment (2026-07-22, transcription)**: the two sentences above
> disagreed — "each encode within `version_bound`" versus the
> call-site doubling below. Landed semantics: `version_bound` bounds
> the node's resident bounds **together** (the ceiling/floor pair),
> and the caller doubles once, centrally. A hidden per-backend ×2
> would be the kind of forgotten constant whose lapse breaches the
> memory envelope; the doubling now lives at the one audited call
> site in `from_budget`.

The derivation evaluates it per depth at
`(c_q(depth), 2 × (local_max + remote_max))` — the per-parent fan
quantile it already computes, and §2.2's exchanged bound doubled for
the ceiling/floor pair. `Local` implements `|_, _| size_of::<ptr>()`
(handles into a session-resident tree); `Failing<B>` delegates. The
default budget's `SCOPE_ENVELOPE_BYTES` \[fitted\] is then
re-derived through `Local`'s function and either confirmed or
corrected — the design-link statement stays, its constant stops
being hand-fitted.

**Status (2026-07-22): landed, and the re-derivation corrected the
constant.** The held reference at depth `d` is priced at its own
fan quantile `c_q(d)`, and the recomputed design-session charge —
BDP-scale corpora (62,500 messages a side) in full divergence, every
stage population in flight, through `Local`'s function — is
**4,339 B per scope**, not the fitted 2 KiB: the fit, made across
the trade-off table's smaller corpora, under-covered the design
point by ~2.1×. The constant is now that derived value, pinned by
exact recomputation (`scope_envelope_matches_the_derivation`, which
also asserts end-to-end that the default admits the whole BDP in
flight at the design session), so it fails loudly instead of
drifting. Consequences, all regenerated: the default budget is
~271 MB (`62,500 × 4,339`); the trade-off table's default row sits
at parity in every column (the 128 MB fit had read 1.4× at 50k
mutual divergence); the operator throughput check re-derives to
`budget / (22 × RTT)`, the envelope-to-wire ratio `4,339 / 200`;
and the table's default row is labeled from the constant at
generation time so it cannot go stale. §1.5's same-day 512 MiB
adopt-and-revert reasoning is unchanged in kind — the margin
question was about slack *above* the design point, while this
correction is the design point's own charge, priced honestly.

**Audit rule**: this closes the derivation's one asymmetric error
direction. Everywhere else, mis-estimation costs latency; an
underpriced node breaches the *memory* envelope. After §2.1–§2.4 no
input is estimated, so the envelope's status becomes \[derived\]
from exchanged measurements plus one pinned lemma.

**Amendment (2026-07-23): two slot-pricing corrections re-derive the
envelope to 4,865 B.** First, the per-child slot constant is now
`size_of` of the real container types instead of a hand count: the
`(u8, Resolve)` resolution slot is 24 B, not 16 (`Option<Node>`
consumes the handle's only null niche, so the `Ready`/`Pending` tag
sits out of line), moving the slot family from 49 to 57 B/child.
Second, the leaf-request edge is charged at the width the capacity
assignment grants it — `population[KEY_DEPTH]`, whose depth-30 joint
quantile is zero for every representable corpus (nonzero needs
pair ≥ 2¹⁹⁰; the leaf stage's own depth-31 statistic floors
identically at pair ≥ 2¹⁹⁸) — in place of a corpus-wide `n × 40 B`
term the assignment provably never granted (the phantom charge
returned up to 2.5 MB at the design session to the real stages); the
one-slot liveness floor stays granted-but-uncharged, like every other
stage's floor slot. Net: envelope 4,339 → 4,865 B, default
budget ~271 → ~304 MB, operator ratio `E/w` 22 → 25; all three
remain pinned by the same recomputation tests.

**Amendment (2026-07-23): the leaf seam enters the account; the
decode fans are charged flat.** `Leaf::leaf` is now async and
fallible — the backend's one chance to take custody of a supplied
payload (persist it eagerly, or stage it in its own priced
write-behind buffer) before the leaf enters the decode fan — so a
leaf's whole resident price is `node_bytes(0, bounds)`, and "leaf
payloads are out of scope" narrows to payload bytes still crossing
inside one wire message (`target_message_size`'s unit). What the fan
channels hold is therefore backend-priced, and their residency is
deterministic rather than population-driven: one channel of `FAN`
slots plus the record in the reader's hand per reply stream
(`STREAM_COUNT` of them; capacity is a liveness floor no
configuration may shrink). `from_budget` pre-charges that flat term
through the session's own `node_bytes`, and the default budget gains
its `Local`-priced value, `SUPPLY_DECODE_ENVELOPE_BYTES` = 17 × 257
× 48 = 209,712 B \[derived: slot layout by `size_of`, handle pinned
pointer-sized\] — the design point keeps slowdown 1 by construction.
The operator forms become affine in it: `slowdown ≈ max(1, 25 × BDP
/ (budget − fans))`, `budget_min ≈ fans + 25 × BDP / slowdown`. The
conformance suite checks the seam pointwise (`underpriced leaf`, at
construction) with a lying-leaf negative control, and the census
charges leaves at their measured post-custody residency.

### 2.5 Validation

- Census suite gains the lemma-slack pin: max encoded node-version
  size measured against `local_max + remote_max` across the
  controlled-divergence runs.
- `conformance::backend` (the namespace already leaves room): a
  caller-built backend runs the controlled-divergence protocol under
  a byte-charging census decorator (the `Failing<B>` wrapping
  pattern), asserting measured peak against the caller's stated
  budget, plus the pointwise cost check — nodes of fan `k` measured
  against `node_bytes(k, observed_bound)`. This is the opening
  artifact of the database-backend campaign and the answer to "does
  the user's bound actually hold" for backends this crate has never
  seen.

**Status (2026-07-22): both landed.** The lemma-slack pin
(`version_bounds_stay_inside_the_priced_pair_bound`) reconciles a
three-party divergence and measures every per-node bound with memos
forced — the numbers in §2.2's status. `conformance::backend` runs
the byte-charging decorator (`Charged<B>` over a `Measure` oracle
the backend supplies as ground truth) end to end: identical corpora
at the floor and under a stated budget, peak *measured-byte*
difference held inside the budget, pointwise `node_bytes` and
`len`/`version_bytes`-recurrence checks per assembled parent, and
convergence witnessed by root-hash equality. Exercised against
`Local` and against a DB-row-shaped materializing reference backend
whose node values own real buffers; an underpriced variant of the
same backend is pinned to fail by name (the suite's teeth are
themselves tested). One deliberate deviation: the `Backend` trait is
crate-internal today, so the suite is crate-gated and runs as this
crate's own gate — its entry point goes public together with the
storage-backend boundary, the way `conformance::link` shipped with
`Link`. Accounting premise stated in its module docs: leaf values
charge nothing (leaf payloads belong to `target_message_size`).

### 2.6 Acceptance

Phase 4 is done when: the lemma is probed and pinned (or the
fallback recorded here with a dated amendment); `NODE_BYTES` no
longer exists; the envelope claim in `sync_memory_budget`'s docs
carries no \[fitted\] input; the census lemma-slack pin and the
pointwise conformance check are in the gate; and the trade-off table
is regenerated under the function-priced derivation.

**Met, 2026-07-22**: every clause checked mechanically — no
`NODE_BYTES` token survives in `src/` (the token lives on in this
plan's sibling design documents and the envelope simulator,
`examples/envelope_sim.rs`, which speak about the rejected shape);
the only "fitted" mentions in `src/` are
the two negations documenting that the envelope no longer is; the
pins run under the ordinary test gate; the committed table carries
the function-priced default row.

## 3. Deliberately out of scope

- Byte-metered channels and per-stream budget division (`L(N)`):
  earmarked for the transport receive-window task; runtime
  backpressure stays the `Link`'s alone.
- The B0.5 Chernoff sharpening headroom (stride-2 direction split,
  top-K order statistics): recorded, untaken; adopt only if a regime
  wants the ~2× and pays its coupling to stream parity.
- Per-deployment tuning guidance beyond the table: the default's
  design-link derivation plus the `budget / (25 × RTT)` operator
  check are the whole story on purpose.
