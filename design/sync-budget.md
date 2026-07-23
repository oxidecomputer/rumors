# The sync budget: session memory bounded by occupancy, priced by the backend

Status: phases 1–3 landed (2026-07-22, `a3da46c4..4d6ff0f0` on
`link-transport`); phase 4 in progress below, spec-first.
Companions: `design/streaming-latency-serialization.md` (the
serialization diagnosis this campaign grew from, amended in place as
the knob evolved) and `design/b05-uniformity-envelope.md` (the
occupancy mathematics imported here; its own header maps what was and
was not adopted).

**Model of record**: uniform-hash, authenticated-honest-peer. Content
addresses are uniform 32-byte strings; hostile-peer regimes are
off-model; no argument below rests on adversary economics. Claims are
tagged \[derived\] (premises stated), \[measured\] (instrument named),
\[fitted\] (envelope, tightness stated), or \[decision\].

## 1. What landed

### 1.1 The interface

One optional knob: `Peer::sync_memory_budget(budget_bytes)`, default
`DEFAULT_SYNC_MEMORY_BUDGET` — 128 MB, **computed in-code from its
premises** (a 100 Gbps × 1 ms-RTT design link's 12.5 MB
bandwidth-delay product, filled at one disputed scope per 200 measured
wire bytes, each charged 2 KiB of fitted envelope; `window.rs` holds
the constants and the multiplication). The budget is a worst-case
envelope per session, never an allocation; concurrent sessions on
separate links each carry their own.

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
calibrates at ~200 B per disputed message \[measured\]. This is what
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
  asymmetric catch-up at 7 hops (5 reverse) regardless of budget,
  zero-budget serialization (520 hops at 2k mutual) that converges,
  one-byte pipes under floor windows live under the deterministic
  quiescence witness, mid-session growth only serializes, and the
  claims re-verified on a running clock (the virtual model
  overcharges, never undercharges).
- **`examples/window_tradeoff.rs`** → `just window-tradeoff` →
  `src/tree/mirror/streaming/window/tradeoff.md`, compiled into
  `sync_memory_budget`'s rustdoc: the budget × divergence slowdown
  grid, latency-only worst case (256 KiB at 50k mutual: ~635×; the
  default at parity).
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

### 2.3 The greeting carries it

The version frame's body grows from `len(8)` to
`len(8) ‖ max_version_bytes(8)`: the same deliberate-wire-change
procedure as the set size (one isolated snapshot re-accept,
byte-verified version-frames-only, V1 untouched). After it, **every
input to worst-case memory is on the table at handshake time**.

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

The derivation evaluates it per depth at
`(c_q(depth), 2 × (local_max + remote_max))` — the per-parent fan
quantile it already computes, and §2.2's exchanged bound doubled for
the ceiling/floor pair. `Local` implements `|_, _| size_of::<ptr>()`
(handles into a session-resident tree); `Failing<B>` delegates. The
default budget's `SCOPE_ENVELOPE_BYTES` \[fitted\] is then
re-derived through `Local`'s function and either confirmed or
corrected — the design-link statement stays, its constant stops
being hand-fitted.

**Audit rule**: this closes the derivation's one asymmetric error
direction. Everywhere else, mis-estimation costs latency; an
underpriced node breaches the *memory* envelope. After §2.1–§2.4 no
input is estimated, so the envelope's status becomes \[derived\]
from exchanged measurements plus one pinned lemma.

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

### 2.6 Acceptance

Phase 4 is done when: the lemma is probed and pinned (or the
fallback recorded here with a dated amendment); `NODE_BYTES` no
longer exists; the envelope claim in `sync_memory_budget`'s docs
carries no \[fitted\] input; the census lemma-slack pin and the
pointwise conformance check are in the gate; and the trade-off table
is regenerated under the function-priced derivation.

## 3. Deliberately out of scope

- Byte-metered channels and per-stream budget division (`L(N)`):
  earmarked for the transport receive-window task; runtime
  backpressure stays the `Link`'s alone.
- The B0.5 Chernoff sharpening headroom (stride-2 direction split,
  top-K order statistics): recorded, untaken; adopt only if a regime
  wants the ~2× and pays its coupling to stream parity.
- Per-deployment tuning guidance beyond the table: the default's
  design-link derivation plus the `budget / (3 × RTT)` operator
  check are the whole story on purpose.
