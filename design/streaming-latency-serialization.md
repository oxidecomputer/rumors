# Streaming latency serialization: diagnosis and fix space

Status: diagnosis complete (2026-07-17, on `link-transport` at
`d571c21f` plus the latency benchmark harness); root cause confirmed
by experiment. **Fix (a) landed the same day** as
`Peer::max_in_flight_nodes` — see §5.2 for the shipped shape, §5.3
for the per-leaf fan-buffer follow-up — and
`tests/gossip_pipelining.rs` pins the pipelining behavior against
regression. **The zero-latency compute gap was profiled and largely
closed on 2026-07-18** (§5.4: bulk conversion at the `Local`
backend boundary, 80.3 → 31.6 ms at I = 5000). §9 lists what
remains. Companion to
`design/streaming-wire-deadlock.md`: that
document explains why the transport contract demands independent
streams; this one explains why the protocol running *on* those
streams originally failed to exploit them.

Question under test (the `gossip_latency_*` groups in
`benches/gossip_fixed.rs`, over the delayed-pipe link in
`benches/support/latency.rs`):

1. Does the streaming protocol (V2) cost more compute than the
   alternating protocol (V1) at zero link latency — and how much?
2. Does V2 ameliorate link latency relative to V1 — and how does it
   scale?

Verdict:

1. **Yes: as shipped, V2 cost 2×–4× V1's compute at zero latency**
   [checked], growing with divergence (details in §2). §5.4's
   conversion-boundary fix has since cut this to 1.5×–1.9× — a
   flat ~11 ms premium at I = 5000 rather than a multiple.
2. **No — as shipped, V2 is dramatically *more* latency-sensitive
   than V1** [checked]. V1's session time grows with latency at
   ~8–9 serialized one-way hops regardless of divergence; V2's grows
   at ~250–650 hops, *scaling with the number of disputed scopes*.
   At 100 ms one-way delay and 5 000 divergent insertions, V1
   completes in 0.92 s and V2 in 64.8 s — a 70× inversion of the
   design intent.
3. The cause is not the descent structure: it is the **capacity-1
   bounded channels** between same-side pipeline stages, sized for
   the deadlock-freedom argument's floor ("one slot is sufficient")
   rather than for keeping work in flight [derived, then checked].
   Widening those channels to 256 slots cuts V2 to ~18–27 hops
   (64.8 s → 2.8 s); widening to 65 536 cuts it to ~7 hops, *below*
   V1 — with every wire proptest and wire-format snapshot passing
   unchanged [checked].

Epistemic key, following `formal/PROGRESS.md`: **[checked]** =
measured or observed in a run described here; **[derived]** =
argument from the code; **[open]** = known unknown.

## 1. The instrument

`benches/support/latency.rs` builds a `Link` whose every stream
(control and data) is a *delayed pipe*: bytes written at virtual
instant `t` become readable at `t + delay`, under a byte-bounded
in-flight window. Sessions run on a current-thread Tokio runtime
with a **paused clock**, so delay is charged to virtual time and a
100 ms sweep costs no wall time. The reported duration is

    wall compute  +  virtual wire stall

and the virtual component is deterministic: measured points are
linear in the delay to three significant figures, so

    hops  =  (T(delay) − T(0)) / delay

is an exact count of the session's longest serialized chain of
one-way stream hops. The conformance suite passes for this link at
zero and nonzero delay (`tests/latency_link.rs`), so the numbers
below measure the protocols, not a nonconforming transport.

Instrument caveats, all shared equally by both protocols:

- Both peers' compute serializes on one thread (same convention as
  the zero-latency wire harness), so wall components overstate a
  real deployment's compute by up to 2×.
- The per-stream in-flight window is 8 MiB — far above any session's
  transfer at these sizes — so bandwidth-delay throttling contributes
  nothing; the hop counts are pure dependency structure.
- Tokio's timer wheel quantizes sub-millisecond deadlines; all swept
  delays are whole milliseconds.

Fixtures are the `gossip_fixed` ones: a 10 000-message universe of
single-byte payloads whose keys are 32-byte content hashes; radix-256
trie, so at this size branching is effectively exhausted at the first
byte (all 256 root-child slots occupied, ~39 keys under each) and
path compression makes the tree ~2–3 effective levels deep.

## 2. Results: the shipped protocol

Compute at zero latency (mean session time, both peers serialized;
`I` = total bidirectional insertions, `R` = total bidirectional
redactions) [checked]:

| fixture   | V1       | V2       | V2/V1 |
|-----------|----------|----------|-------|
| I = 0     | 17 µs    | 39 µs    | 2.3×  |
| I = 500   | 4.12 ms  | 14.58 ms | 3.5×  |
| I = 5000  | 21.0 ms  | 87.8 ms  | 4.2×  |
| R = 2500  | 7.2 ms   | 20.5 ms  | 2.9×  |

Serialized one-way hops (slope of session time in one-way delay;
exact to the precision shown, identical at 1/5/10/25/100 ms points)
[checked]:

| fixture   | V1 hops | V2 hops | at 100 ms: V1 → V2 |
|-----------|---------|---------|---------------------|
| I = 0     | 2       | 3       | 0.20 s → 0.30 s     |
| I = 500   | 8.0     | 246     | 0.80 s → 24.6 s     |
| I = 5000  | 9.0     | 647     | 0.92 s → 64.8 s     |
| R = 2500  | 9.0     | 536     | 0.91 s → 53.6 s     |

Two readings:

- V1's hop count is a small constant: one exchange per active tree
  level, each level's entire frontier batched into one message. Its
  latency exposure is `O(depth)` and depth is ~4 here.
- V2's hop count tracks *work*: it sits at 1.1×–2.5× the number of
  disputed level-31 scopes (the root-child subtrees the two sides
  disagree on: an expected ~220 of 256 slots at I = 500, all 256 at
  I = 5000 and R = 2500) [derived, from the fit]. The protocol that
  exists to stream is descending the tree essentially one disputed
  scope at a time.

## 3. Root cause: one-slot channels serialize the descent

### 3.1 The mechanism

The materialized walk
(`src/tree/mirror/streaming/materialized/work/levels.rs`) processes
each incoming reply like this (order fixed by the
`yield_resolve_query!` macro, `materialized.rs`):

    while let Some(reply) = requests.next().await {   // wire, in order
        let query = queries.recv().await;             // pair positionally
        for each disputed child C in reply {
            yield message(C);                         // wire out
            lower.send(resolution(C)).await;          // capacity 1
            for q in C's child queries {
                asked.send(q).await;                  // capacity 1
            }
        }
        upper.send(parent resolution).await;          // capacity 1
    }

Every inter-stage edge — child queries, parent resolutions, child
resolutions, leaf requests, and all three remote-proxy edges — is a
bounded channel of **capacity 1**
(`materialized/work/queues.rs`, `remote/proxy/work/queues.rs`).
The one exception is the assembly fan queue, deliberately sized to a
full fan of 256.

Follow one stage boundary [derived]:

1. Stage *h* processes the reply for scope `P`, whose disputed
   children are `C1..Ck`. It yields the wire message for `C1`, then
   enqueues `Query(C1)` into the capacity-1 `asked` channel. So far
   so good.
2. The next stage dequeues `Query(C1)` only from inside *its* loop —
   and that loop awaits the **wire reply first**
   (`requests.next().await` before `queries.recv().await`,
   `levels.rs`; likewise `absorb` in `materialized.rs`). The reply
   to `C1` exists only after our `C1` message crossed the wire and
   the peer's stage answered it: a full round trip.
3. Stage *h* meanwhile has yielded the message for `C2` and now
   blocks in `asked.send(Query(C2))` — the slot is still occupied by
   `Query(C1)`. The walk stops consuming its own input stream. No
   messages for `C3..Ck` go out, and nothing behind them moves,
   until `C1`'s round trip completes.

The resolution channels serialize the same way one level up: the
assembler (`work/assembly.rs`) holds resolution `C1` while awaiting
the returns that fill its `Pending` slots — which arrive only after
`C1`'s whole subtree reconciles — and the capacity-1 resolution
channel therefore admits only ~2 further scopes before the walk
blocks behind *subtree completion*, not just one reply.

Net effect: at each level boundary at most ~2 disputed scopes are in
flight, so the descent degenerates into per-scope round trips. The
measured hop counts fit `hops ≈ c × disputed scopes` with
`c ∈ [1.1, 2.5]` — the constant rising with divergence as leaf pulls
lengthen each scope's stall — and are wholly insensitive to depth,
the opposite of the pipelined ideal [checked, from §2's table].

The same one-slot waker ping-pong is a plausible contributor to the
zero-latency compute gap (§2): every scope pays several
blocked-send/wake cycles even on an instant wire. Widening the
channels reduced zero-latency compute 8 % (I = 5000) and 24 %
(R = 2500) [checked], so it is a real but minor share; the rest —
per-scope message framing, per-phase boxed streams, per-reaction
allocation — is unattributed [open: needs profiling].

### 3.2 Why capacity 1 was the natural — and wrong — choice

The module documentation (`materialized.rs`, "Why this is
deadlock-free") proves one slot **sufficient** for every query and
resolution channel: the walk publishes each wire action before its
in-process twin, and each resolution before the work that fills its
slots, so a blocked sender can never withhold what the counterparty
needs to advance. That argument is about *liveness*, and it is
correct — the session never deadlocks at capacity 1.

But sufficiency for liveness was silently taken as the sizing rule,
and it prices every slot as if the only cost of blocking were
deadlock risk. The actual cost of a blocked `asked.send` is one
wire round trip of dead air per disputed scope. Capacity bounds
*memory* (a `Query` may own a fan of node handles, so a K-slot
queue holds up to K·fan handles); the design bought the minimum
memory at a per-scope latency price the deadlock argument never
models. The one place the code already reasons about throughput —
the assembly fan queue, capacity 256 so "every child completion can
enqueue while the walk finishes the reaction loop"
(`queues.rs`) — is precisely the reasoning the recursive edges lack.

## 4. The experiment: capacity as the knob

Patch: every capacity-1 protocol channel in
`materialized/work/queues.rs` and `remote/proxy/work/queues.rs`
raised to K (the cardinality-1 root edges can never hold a second
item, so raising them is inert). No other change; the patch is not
wire-visible, and `gossip_snapshot` byte-for-byte snapshots pass
unchanged [checked].

Hops and totals [checked]:

| fixture  | K = 1 (shipped) | K = 256      | K = 65 536 |
|----------|-----------------|--------------|------------|
| I = 0    | 3               | 3            | —          |
| I = 5000 | 647 (64.8 s)    | 27 (2.78 s)  | 6.6        |
| R = 2500 | 536 (53.6 s)    | 18 (1.82 s)  | 7.5        |

(Totals at 100 ms one-way delay; V1 reference: 9 hops, 0.92 s.)

Zero-latency compute moved the right direction: 87.8 → 80.5 ms
(I = 5000) and 20.5 → 15.5 ms (R = 2500) at K = 256 [checked].

At K = 256 the residual 18–27 hops show capacity still binding (256
disputed scopes across several boundaries drain in waves); at
K = 65 536 — effectively unbounded — V2 reaches ~7 hops, beating
V1's 9 while still streaming leaf content incrementally. The
structural floor is the phase ladder itself (~2–3 active levels ×
2 hops, plus the leaf pull), consistent with the I = 0 floor of 3
[derived].

Correctness under K = 65 536: `pairwise`, `shadow_validity`,
`async_wire`, and `gossip_snapshot` all pass, including the two
proptests whose committed regression seeds caught the original
streaming deadlock [checked]. This is expected, not incidental:
raising capacity only removes blocking edges from the wait graph,
so any schedule live at capacity 1 remains live at capacity K — the
one-slot proof still holds as the floor case [derived].

## 5. Fix space

Ordered by leverage per unit of change:

**(a) Make the recursive-edge capacity a tunable pipeline window.**
The direct fix, and the experiment above is its proof of concept.
Replace the literal `1` on the recursive edges (child queries,
parent/child resolutions, leaf requests, proxy edges) with a window
`K` chosen at session or protocol construction. Semantics: `K` is
the number of disputed scopes a session keeps in flight per level
boundary; completion time is approximately

    compute + RTT × (c₁·depth + c₂·scopes/K)      [derived envelope]

so `K ≥ expected disputed scopes` recovers full pipelining, and any
`K > 1` divides the latency term proportionally. Memory: worst case
~K·fan node handles per recursive boundary (handles, not nodes —
`imbl` structural sharing keeps the bytes shared), which is exactly
the trade the assembly fan queue already makes at K = fan. A default
of one full fan (256) is the natural unit: it collapses the
pathology by 24×–30× here, costs at most one fan of `Query` values
per boundary, and leaves the fixed-memory story intact in kind —
merely with an honest coefficient. Deployments on fat, long pipes
can raise it; memory-starved ones can keep 1 and accept V1-era
latency only if their divergence is small.

**(b) Dequeue the query before awaiting its reply.** The stage loops
pair `requests.next().await` then `queries.recv().await`; reversing
the order frees each slot one reply earlier. Constant-factor only
(~1 hop per scope of the 2–2.5 measured) and worthless alone, but it
makes `K`'s accounting exact: with reply-first pairing, a K-slot
queue admits only K−1 truly in-flight scopes [derived].

**(c) Batch sibling scopes per message (V1's trick inside V2's
schedule).** One wire message per level carrying every disputed
scope's reactions would make hop count `O(depth)` at *any* channel
capacity, since a single message-and-reply round trip covers the
whole frontier. This is a wire-format change (the codec's
scope-per-reply framing, `remote/codec.rs`, and the snapshot suite
would all move) and it reintroduces V1's per-level memory spike —
the frontier must materialize to be batched — which is exactly what
V2 exists to avoid. Not recommended while (a) reaches the same hop
regime without touching the wire [derived].

**(d) Credit-based adaptive window.** The principled endpoint of
(a): grant scope credits the way the transport grants bytes, sizing
the window to observed RTT × scope rate. Real engineering surface
(credit accounting spanning the proxy), justified only if static
`K` proves awkward to choose in deployments. Defer until (a) has
field data [derived].

Recommendation: (a) with (b) as a rider, default K = 256, exposed as
a protocol/session knob documented in units of "disputed scopes in
flight per level" with the memory envelope stated next to it (§6).
The deadlock argument needs no re-proof — only a note that one slot
is the liveness floor and K the throughput choice.

### 5.1 Where the knob reaches

K must be threaded through **both** protocol implementations — the
materialized walk *and* the remote proxy — and, session-wide,
through **both ends**: the effective window is the minimum over the
whole chain (walk queries → response pump → proxy encoder →
`local_questions` → wire), so any capacity-1 edge on the question
path restores the pathology alone. Note in particular that the
*responder's* walk channels gate its answer production the same way
the initiator's gate its questions — `yield_resolve_query!` blocks
on its sends before processing the next incoming scope, and those
sends drain against the wire — so an end cannot delegate its K to
its peer. Ends with different K interoperate without negotiation:
pairing is positional and the codec is untouched, so each direction
simply runs at the asking end's window [derived].

The edge-by-edge map. **Scale with K** (the load-bearing windows):

- Materialized, `materialized/work/queues.rs`:
  `responder_child_queries`, `internal_child_queries`,
  `leaf_requests` — the in-flight question queues, the literal
  definition of the window — and `internal_parent_resolutions`,
  `internal_child_resolutions`, `leaf_parent_resolutions`,
  `leaf_child_resolutions` — resolutions block behind whole-subtree
  completion (§3.1), so at capacity 1 they stall the walk ~2 scopes
  in regardless of the query queues' width.
- Proxy, `remote/proxy/work/queues.rs`: `local_questions` — **the
  wire-facing window**. It holds flushed-but-unanswered question
  scopes; `encode.rs`'s `publish` blocks on it mid-batch, which
  stops the encoder consuming the walk's messages, so capacity 1
  here re-imposes ~2 questions in flight per height no matter what
  the walk's channels allow. Also `next_scopes` — the decode-side
  hand-off of the peer's questions to the next level's encoder.
  Strictly it is a local relay register (its consumer never waits
  on the wire directly), but a parked mid-batch send stops the
  decoder from yielding already-arrived replies to the walk, and a
  `Scope` is ~110–330 B (a 40 B prefix, the asked radices, a
  cursor), so sizing it with K removes the starvation bubble for
  free [derived].

**Leave alone**:

- Cardinality-bound edges (can never hold a second item; widening
  is inert): `initiator_root_query`, `initiator_root_return`,
  `responder_root_resolution`, `responder_root_returns`.
- Relay pumps (in-order registers; a full slot only stalls when the
  consumer is itself stalled, so they add pipeline stages, not
  round trips): `outgoing_responses` on the walk side, `responses`
  (`ProxyResponses`) on the proxy side. Keeping the latter at 1 is
  also what preserves `streaming/message.rs`'s "at most one maximal
  reply in flight per stage" memory clause verbatim.
- `terminal_leaf_resolutions`: consumed at local compute pace with
  no wire wait downstream.
- `assembly_level_returns` stays at FAN: its burst unit is one
  reply's fan, and returns still arrive in resolution order under
  K > 1 with each resolution published before its dependent work,
  so the backlog stays one fan deep [derived — re-verify this
  argument in review when the knob lands].

Plumbing: every capacity is a literal inside a constructor in the
two `queues.rs` modules today, so K travels as a field on the two
`Work` contexts (`materialized::Work::new`, proxy `Work::new`),
reaching them from `materialized::Handshaking::start` and the proxy
`Handshaking` respectively — both constructed by the peer session
layer, which is where the public knob belongs, beside the existing
`Protocol` selection.

### 5.2 The shipped knob

Landed as described in §5.1, with the knob denominated per §7's
unit question — charged in node references, the quantity §6 prices:

- **`Peer::max_in_flight_nodes(nodes)`**, following the peer like
  `protocol()` through `into_rumors`, cloning, bookmarking, and
  retirement. Internally a `Window`
  (`src/tree/mirror/streaming/window.rs`) derives the per-edge scope
  capacity as `K = nodes / (FAN × SATURABLE_LEVELS)`, floored at one
  slot: each in-flight scope is charged a full possible fan, and at
  most three level boundaries can run at full occupancy against
  terabyte-scale sets (§6.5), so the budget is session-global rather
  than per edge.
- **Default: `DEFAULT_MAX_IN_FLIGHT_NODES = 3 × 256³`** (≈ 50M
  references) ⇒ K = 65 536 = fan²: a fully fanned level's entire
  cascade never blocks, for the ≈ 10 GB bounded worst case §6.5's
  last row prices — reached only against multi-gigabyte divergence.
- **Test builds default to the floor.** Under `cfg(test)` and the
  `test-internals` feature the window defaults to one slot, so every
  existing test keeps exercising the capacity-one orderings whose
  deadlock-freedom the §3.2 argument certifies; benches opt into the
  production window explicitly. The assembly fan queues are
  untouched by the window: their one-full-fan capacity is a hard
  correctness floor (`underbuffered_mirror_stalls` demonstrates the
  stall below it), documented at the constructor.
- Verification: `tests/gossip_pipelining.rs` gossips ~500 disputed
  scopes over a 10 ms delayed link and bounds the session at 64
  hops; at the floor the same session measures ~370 hops (3.7 s),
  so the test fails loudly if any edge regresses to serial. The §2
  sweep rerun through the public knob measures 8–9 hops at I = 5000
  and R = 2500 (from the 10 ms and 100 ms columns; the 1 ms column
  drowns in wall-intercept noise), matching §4's fully pipelined
  regime: at 100 ms one-way delay V2 now completes I = 5000 in
  0.99 s against V1's 0.92 s, and R = 2500 in 0.82 s against V1's
  0.91 s [checked].

### 5.3 The per-leaf compute follow-up

Chasing the zero-latency compute gap (§2, §7) surfaced a second
one-slot family, on the compute path rather than the wire path: the
adapter's supply-reassembly channel ("one leaf in flight",
`remote/adapter/decode.rs`) and the walk's terminal leaf
resolutions. Neither waits on the wire — their cost is a waker
round trip *per supplied or requested leaf* — so they were rightly
outside the latency fix, but at I = 5000 they charged ~2.3 µs to
each of 5 000 leaves. Both now buffer one constant fan (256), not a
window derivation: a supply run belongs to exactly one in-flight
scope, whose full fan the memory model already charges, so the
buffer adds no memory term and there is nothing for the knob to
express. (Contrast the assembly fan queues, where one fan is a
correctness *floor*; here it is an amortization, and the two are
documented against each other at the constructors.)

Measured [checked], against the same-session §5.2 baseline with V1
as the drift control: V2's I = 5000 zero-latency compute fell
91.9 → 80.3 ms (−12.7 %, versus −4.5 % V1 drift); the leaf-free
R = 2500 cell moved within drift, and hop counts were unchanged at
9.0 — the clean signature of a compute-path-only fix.

### 5.4 The conversion-boundary fix: stop unwrapping compressed spines

§9's item 1 — profile the remaining 80 vs 20 ms — ran on
2026-07-18 (samply over the paused-clock bench binary, both V1 and
V2 `I = 5000, d = 0` cells, V1 as the subtraction baseline). The
profile acquitted every §7 suspect and convicted one nobody had
named:

| per iteration, `I = 5000, d = 0` | V2 before | V1 | Δ |
|---|---|---|---|
| allocator self time              | ~26 ms  | ~3 ms   | **+23 ms** |
| `async_stream` generator glue    | ~11.6 ms| ~0      | **+11.6 ms** |
| blake3                           | ~7.6 ms | ~7.3 ms | ≈ 0 |
| memcpy/memset + `Arc::make_mut`  | ~9 ms   | ~1.5 ms | +7.5 ms |

Attributing the allocator time to its nearest crate-level caller
put ~45 % under the conversion machinery (`into_children`,
`Node::branch`, `beneath`, `fold_parents`, `explode`, `assemble`)
and most of the rest under the encode/decode task closures those
frames inline into. The mechanism [checked, then derived]: the
conversion boundary un-did and re-did **path compression one byte
at a time**. `Convert::explode` descends every *typed* height, so
a supplied single-leaf subtree — the common case at I = 5000,
where divergent keys sit alone under their 2-byte slot — had its
~30-byte compressed spine unwrapped level by level on encode
(each level: an `Arc::make_mut` clone of the node innards plus a
fresh single-entry `OrdMap`), and rebuilt level by level through
`fold_parents` on decode (each level: a one-entry child group, a
map, a `beneath`). Roughly 600 000 virtual-level crossings per
session, every one through a nested `try_stream` generator frame
— which is also where the `async_stream` self time came from. The
tree's path compression exists precisely to skip those levels; the
conversion chain paid them anyway.

The fix reads the seam the design had already left open:
`Backend::leaves` was documented as overridable for backends that
"can obtain this more efficiently", and `Local` now does — a
direct owned leaf walk (`typed::Node::leaves`, built on the
`RangeOwned` spine walk, which steps over compressed spans and
clones one child handle at a time). Decode gained the symmetric
seam: a new `Backend::assemble` method (default: the old
level-by-level `Convert` fold, which `Failing` and any future
database backend keep) that `Local` overrides by buffering each
maximal same-prefix run and bulk-building its subtree
(`typed::Node::from_sorted_leaves`): sorted input makes every
divergence byte a first/last comparison, so construction is
proportional to the *materialized* structure — one node per real
branch point or leaf spine — not to virtual levels. The buffered
run is transient state for a subtree the in-memory backend is
about to hold whole anyway, so the §6 memory story is unchanged.
Three property tests pin both overrides to observational
equivalence with the default chain over deep-spine and multi-run
inputs (`streaming/backend/local/tests.rs`).

Measured [checked], same-session sweep, V1 cells as drift control
(V1 moved < 3 %):

| cell, d = 0        | before  | after   | vs V1 |
|--------------------|---------|---------|-------|
| insertions I = 5000| 80.3 ms | 31.6 ms | 20.7 ms → 1.52× (was 4.0×) |
| redactions R = 2500| 17.3 ms | 12.3 ms | 6.5 ms → 1.89× (was 2.7×) |
| empty session      | ~35 µs  | ~25 µs  | ≈ parity |

Hop counts unchanged at every latency (9.0 at I = 5000, d = 100 ms
for both protocols; 3 for the V2 empty session) — again the
compute-only signature. The re-profile shows the conversion frames
reduced to noise; the residual ~11 ms per session is diffuse:
blake3 at near-parity with V1 (~8.4 ms, now the largest single
item), branch-hash memoization for freshly assembled subtrees,
per-frame codec and channel glue. Redaction break-even against V1
drops from ~11 ms to ~6 ms one-way; insertion-only sync now costs
a flat ~11 ms premium at equal hops, rather than 3× V1's compute.

## 6. The memory price of K

What does a window of K actually cost, worst case? The answer
depends on what a node handle *is*, so take the two backend regimes
separately: the in-memory `Local` backend, and the floor any backend
can reach. Sizes below are [derived] from the current
representations; the arithmetic assumes 64-bit pointers.

### 6.1 What a `Local` handle is

`typed::Node<T, H>` is `#[repr(transparent)]` over
`untyped::Node<T>`, which is one `Arc<NodeInner<T>>`
(`src/tree/typed/untyped.rs`): **a handle is 8 bytes, and cloning it
is a refcount bump**. A `(radix, handle)` entry in a `Query` or
`Resolution` vector is 16 bytes after alignment.

Structural sharing is what makes those 8 bytes the whole story. The
session holds its pre-session root alive from handshake to output
(`Handshaking.root`, then the assembly's spine), so every subtree a
buffered `Query` or `Resolution` points into is resident *anyway*;
K× more handles pin K× more refcounts, not K× more tree. And no path
in the walk deep-copies: `Backend::children` iterates `Arc` clones
out of the persistent map, and `Backend::parent` builds fresh spine
nodes *from* shared children. The imbl `OrdMap` diff machinery the
tree rests on prunes pointer-equal spans wholesale for the same
reason.

### 6.2 What K does not buy (paid identically at K = 1)

- **The delta itself.** Peer-supplied leaves (a `Version` plus a
  `Message<T>` — an `Arc<T>` and its shared `Bytes` serialization)
  and the rebuilt spine nodes become the output tree. Channel slots
  hold `Arc`s to those same allocations, never copies, so buffering
  more of them in flight does not duplicate them; it only receives
  them earlier. End-state footprint is independent of K.
- **The in-hand reply batch.** Processing one maximally disputed
  reply already materializes up to fan² hashes ≈ 2 MiB transiently
  (`streaming/message.rs`'s memory-unit note, and the fan² clause in
  `materialized.rs`'s memory model). K sits on top of, not under,
  this existing worst case.

### 6.3 What K does buy: the window, itemized

Per disputed **child** sitting in the window, the K-attributable
bytes on the walk side are:

| item                                            | bytes |
|-------------------------------------------------|-------|
| its `(u8, Node)` slot in a buffered `Query`      | 16    |
| its `(u8, Resolve)` slot in a buffered `Resolution` | 16 |
| its `(u8, Hash)` entry in the reply's `Query` listing | 33 |
| **total per child in flight**                    | **65** |

plus ~130 B fixed per scope (two 40-byte inline `Prefix`es and the
`Vec` headers). A worst-case scope — full 256-fan, every child
disputed — therefore holds ≈ 256 × 65 + 130 ≈ **16.4 KiB**, of which
the actual handles are 4 KiB of pointers into already-resident tree.

Occupancy is self-limiting: a level with S_h disputed scopes fills
at most `o_h = min(K, S_h)` slots, and the disputed forest is
geometric, so `Σ_h S_h ≲ 2Δ` for Δ divergent leaves. The window
envelope is

    window ≈ Σ_h o_h × (fan_h × 65 B + 130 B)
           ≤ min(K × L_active, 2Δ) × 16.4 KiB        (full fan)

Concretely [derived from the formula, geometry checked against §2's
fixtures]:

| configuration                                        | window   |
|------------------------------------------------------|----------|
| K = 1 (shipped): ~2 scopes per boundary               | ~100 KiB |
| K = 256, benchmark geometry (256 scopes, fan ≈ 50)    | ~0.9 MiB |
| K = 256, adversarial full fan, 3 engaged levels       | ~12 MiB  |
| K = 256, absolute ceiling: all 17 boundaries saturated | ~68 MiB |

Three qualifiers. First, K is a cap, not a commitment: once
K ≥ S_h, occupancy stops growing, which is why K = 65 536 in the
benchmark holds exactly what K = 256 holds. Second, the absolute
ceiling requires Δ in the millions (K scopes disputed at *every* of
17 levels), at which point the delta itself is hundreds of MB and
dwarfs the window. That is the general shape: at ~65 B per in-flight
child versus ≥150 B per child of delta content that must ship and be
retained regardless, **the window overhead is bounded by ~40 % of
the touched delta, and only for its in-flight portion**. Third, the
proxy's `ProxyResponses` edge buffers whole decoded `Reply` values;
a K-deep buffer there would hold the *same* listing bytes already
counted above (a fan²-hash reply is exactly fan scopes × fan
listing entries), so widening it changes arrival timing, not the
count — and §5.1's knob map keeps that edge at capacity 1 anyway,
so `streaming/message.rs`'s "at most one [maximal reply] in flight
per stage" clause survives verbatim.

### 6.4 The floor: a minimal placeholder node

`Local`'s 8-byte handle is a gift of structural sharing with a live
in-RAM tree. A backend with no such tree — a database-backed one,
or a hypothetical thin proxy handle — still cannot go below what
the `Node`/`Leaf` contract forces it to keep resident
(`streaming/backend.rs`): `hash()` returns a 32-byte `Hash` by
value, and `ceiling()`/`floor()` return `&Version` — **references,
so both versions must be owned, materialized, in memory**; lazy
fetch is not an option without changing the trait. The minimal
placeholder is therefore

    branch:  hash [32 B] + ceiling Version + floor Version
    leaf:    the above + Message<T> (&-returned: owned Bytes + Arc<T>)

A `Version` is a `BitVec<u8>` — ~24 B of header plus the encoded
ITC party/event tree, single-digit heap bytes for the two-party
benchmark universe but growing with party-tree complexity (fork and
retire churn), which makes it the one unbounded term in the floor.
At benchmark-scale versions a placeholder branch node is
**≈ 130–160 B**; a placeholder leaf adds the serialized payload.
(The prefix need not be stored: it always travels beside the handle
in the `Query`/listing structures already counted in §6.3, and it
doubles as the database locator.)

So for a database-backed backend, §6.3's per-child cost rises from
65 B of containers to ≈ 215 B of containers-plus-placeholder
(~150 B of which is the placeholder), and the table scales by ~3.3×:
the benchmark-geometry window becomes ~2.7 MiB at K = 256, the
adversarial 3-level case ~40 MiB. The conclusion survives the
regime change: the delta still dominates whenever the window is
actually full, because every disputed child's placeholder is
carrying version-and-hash data the session had to receive or read
anyway. But the planning number for K in a memory-budgeted
deployment should be the ~215 B placeholder rate, not `Local`'s
pointer rate — equivalently: **budget K ≈ RAM / (fan × 215 B) per
saturated level**, i.e. each unit of K costs ≈ 55 KB per full-fan
level it saturates. §6.5 turns this into a table.

### 6.5 Choosing K from a RAM budget

Putting §6.3–§6.4 together into a sizing rule. Charge the window at
the placeholder rate (the cross-backend planning number): a
saturated full-fan scope costs ≈ 256 × 215 B ≈ 55 KB, so

    window ≈ L_sat × K × 55 KB  +  ~4 MB fixed

where the fixed term is the in-hand fan² reply batches (§6.2) and
`L_sat` is how many level boundaries can be *simultaneously*
saturated at full fan. That last factor is where realistic set
sizes bite [derived]:

- Saturating L consecutive boundaries with K full-fan scopes each
  requires the disputed forest to span `K × 256^L` leaves. Capping
  the set at 1 TB — ≈ 10^10 messages at a ~100 B/message floor
  (version + key + node overhead; bigger payloads only shrink N) —
  gives `L_sat ≤ 3` for K up to ~600 and `L_sat ≤ 2` for K up to
  ~150 000. The geometry is self-limiting: deep saturation and wide
  saturation compete for the same leaves.
- The *useful* ceiling on K is the largest per-level scope count,
  `S_max ≈ N / fan_min` — up to ~5 × 10^9 for a pathological 1 TB
  set of minimal messages branching at fan 2. No plausible RAM
  budget reaches that, so **against large sets K is always
  RAM-bound, never set-bound**; conversely, for a set that fits
  comfortably in RAM, S_h itself is small and any K ≥ N/256 already
  buys full pipelining (the benchmark's universe saturates at
  K ≈ 256). Occupancy caps at S_h either way — over-provisioned K
  idles rather than allocates.

The budget table (placeholder rate; worst-case full fan; 1 TB set
ceiling; `Local` backends can multiply K by ~3.3, or equivalently
keep the same K with 3.3× headroom):

| RAM budget | K       | worst-case window (+ ~4 MB fixed) |
|------------|---------|-----------------------------------|
| 10 MB      | 32      | ~5.3 MB (L_sat = 3)               |
| 100 MB     | 512     | ~85 MB (L_sat = 3)                |
| 1 GB       | 8 192   | ~0.9 GB (L_sat = 2)               |
| 10 GB      | 65 536  | ~7.2 GB (L_sat = 2)               |

Each row is the largest power of two whose worst case fits the
budget. In `Peer::max_in_flight_nodes` units (§5.2), a target K is
requested as `nodes = K × 768` (fan × saturable levels); the
shipped default is the 10 GB row. The envelope is
conservative twice over (every in-flight
scope at full fan, every saturable level saturated), so typical
occupancy runs far below it — the benchmark geometry at K = 512
holds ~2.8 MB at the placeholder rate (~0.9 MB `Local`), not
85 MB. §5's default of 256 sits at ≈ 42 MB worst
case: safely inside a 100 MB budget with 2× headroom, ruinous for
a 10 MB one — which is the argument for exposing K rather than
hard-coding any value.

Two boundary notes. At the small end, the ~4 MB of in-hand reply
batches is K-independent protocol overhead: a 10 MB deployment pays
it regardless, and K = 32 still collapses the §2 pathology by an
order of magnitude — the knob's returns are steepest exactly where
memory is scarcest. At the large end, a divergence big enough to
keep a 65 536-scope window saturated for many refill waves is
shipping gigabytes of delta, and the session is bandwidth-bound
long before it is RTT-bound: past the table's scale, more K moves
nothing but the envelope.

## 7. Open questions

- **[open]** Where exactly do the residual ~7 hops at K = 65 536
  sit? Plausibly: opening exchange (3, per I = 0) + leaf request →
  supply round + assembly ordering. Frame-level tracing over the
  delayed pipe would attribute them.
- **[resolved in §5.2]** The right *unit* for K: the landed knob is
  charged in node references (children), priced by §6 at 65 B per
  in-flight child for `Local` and ~215 B at the placeholder floor,
  divided across fan and saturable levels so the budget reads
  session-global.
- **[open, narrowed]** The §4 experiment widened all edges together;
  §5.1 has since decomposed them analytically — `local_questions`
  is load-bearing, `ProxyResponses` is not, `next_scopes` is a
  register sized with K defensively. What remains empirical is
  confirming the decomposition: rerun the sweep with only §5.1's
  "scale with K" set widened and check the hop counts match §4's.
- **[resolved in §5.4]** The remaining zero-latency compute gap.
  The earlier exclusions held up [checked]: waker cost (§5.3,
  ~12 %) and hashing (≤3 %). Prefix-shipping was rejected here
  partly on trust grounds ("a trusted prefix breaks the
  content-address commitment") — that leg is retracted by the
  §10 adversary-model decision: the counterparty is trusted, so
  shipping paths is a wire-format trade, not a security question
  (§10, lever C). The profile pinned the bulk of the gap on
  none of the named suspects but on the conversion boundary
  unwrapping path-compressed spines one virtual level at a time;
  §5.4's bulk `leaves`/`assemble` overrides removed ~82 % of the
  gap. What remains (~11 ms at I = 5000) is itemized in §10 as
  independent parity levers.
- **[open]** Whether redaction-heavy sessions (version-bound pruning
  on the leaf path) have a different optimal K than insertion-heavy
  ones; the R = 2500 cell pipelined slightly better (18 vs 27 at
  K = 256) for reasons not yet attributed.

## 8. Reproduction

    # Baseline sweep (V1 requires the protocol-v1 feature):
    cargo bench --features protocol-v1 --bench gossip_fixed -- gossip_latency

    # Hop counts: (T(d) − T(0)) / d, using the 1 ms column.

    # The capacity experiment (not landed; reproduce by patch):
    #   raise every capacity-1 channel in
    #     src/tree/mirror/streaming/materialized/work/queues.rs
    #     src/tree/mirror/streaming/remote/proxy/work/queues.rs
    #   to 256 (then 65536), rerun the sweep filtered to V2, and run:
    cargo nextest run -E 'binary(pairwise) or binary(shadow_validity) or binary(async_wire) or binary(gossip_snapshot)'

    # Link conformance for the measurement transport:
    cargo nextest run -E 'binary(latency_link)'

    # Time profile of one bench cell (§5.4; the bench profile carries
    # line tables for symbolication). The paused clock serializes both
    # peers onto one thread, so a single track holds the whole session:
    cargo bench --features protocol-v1 --bench gossip_fixed --no-run
    samply record -- target/release/deps/gossip_fixed-<hash> --bench \
        --profile-time 15 'gossip_latency_bidir_insertions/V2/divergence=5000/0$'

## 9. Next steps

Ordered by leverage, with each item's blocking relationship stated;
none blocks the landed work.

1. **Profile the compute gap** — **done 2026-07-18, §5.4**: the
   profile convicted the conversion boundary's virtual-level
   unwrapping; the bulk `leaves`/`assemble` overrides landed and
   cut the gap 82 % (80.3 → 31.6 ms at I = 5000, hop counts
   unchanged). The residual ~11 ms is itemized as independent
   levers in §10; the plan is to fan out and try each one.
2. **Isolate the §5.1 decomposition empirically**: rerun the sweep
   with only the analytically load-bearing edges widened (walk
   queues/resolutions + `local_questions`), confirming
   `next_scopes` is the defensive register the analysis says it is.
   Cheap; firms the knob map before anyone builds on it.
3. **Attribute the residual ~9 hops** via frame-level tracing over
   the delayed pipe: the floor is 3 (opening) plus the leaf
   request→supply round; if the remainder contains an avoidable
   serialization, V2's latency edge over V1 widens further.
4. **Land fix-space rider (b)**: flip the stage loops'
   reply-then-query pairing so a K-slot queue admits exactly K
   in-flight scopes rather than K−1. Constant-factor; makes the
   §6.5 accounting exact.
5. **Review the assembly-fan argument under K > 1** (flagged in
   §5.1): the one-fan bound rests on returns arriving in
   resolution order with resolutions preceding their dependent
   work; a reviewer should re-derive it with the window in place.
6. **Optimal K by workload** (from §7): the R = 2500 cell
   pipelines slightly better than insertions at equal K, for
   unattributed reasons; revisit with field data once a deployment
   runs windows other than the default.

## 10. Parity levers: the last ~11 ms

After §5.4, V2 costs 31.6 ms against V1's 20.7 ms at
`I = 5000, d = 0`, with identical hop counts — so on
insertion-shaped workloads the residual is a flat compute premium
at every latency, and closing it is the whole remaining parity
story. (Redaction-shaped workloads are already past parity beyond
~6 ms one-way.) Comparing the post-§5.4 V2 profile against V1's
decomposes the premium [checked, per session, both sides summed]:

| component | ≈ cost | mechanism |
|---|---|---|
| allocator traffic above V1  | ~2.7 ms | per-leaf `Frame` values on encode, twins on decode; branch-hash memo inputs |
| stream/channel glue         | ~2.9 ms | `async_stream` layers plus waker/memcpy: each of ~10k supply frames crosses reader → assembler → walk → encoder |
| blake3 above V1             | ~1.1 ms | `Path::for_leaf` ×3 per received leaf: V2 *derives* leaf paths because the wire ships flat leaf runs; V1's paths are implicit in shipped structure |
| per-scope walk machinery    | ~1 ms   | ~2k disputed scopes × channel sends and scope bookkeeping across the 34 phases |
| tail/noise                  | ~3 ms   | diffuse |

**Adversary model [decision, 2026-07-18]**: the counterparty is
*trusted*. A peer supplies the leaves themselves and can already
corrupt state arbitrarily, so in-session hash verification defends
against nothing in this model — path derivation and scope
containment checks are bug tripwires, not security boundaries.
This retracts the trust leg of §7's prefix-shipping rejection and
unlocks lever C below. Cheap structural checks (leaf ordering,
scope containment) stay: they protect the assembler's invariants
against implementation bugs at negligible cost.

The levers, each independently landable and measurable. Estimates
are [derived] from the profile deltas above; every lever's
acceptance test is the §8 sweep (V1 cells as drift control, hop
counts unchanged) plus a green gate.

- **A. Batch supply runs into one wire frame** [est. 4–6 ms; wire
  format change, `gossip_snapshot` re-accept]. One `Supply` frame
  carrying a count-prefixed run of leaves, chunked at fan size so
  the fixed-memory decode story survives arbitrarily large
  supplies. Kills, per leaf: a frame allocation, a borsh header
  and `Flow` byte, and most of the per-frame channel hops — the
  two largest ledger rows at once. V2's format is still ours to
  change; V1 interop is unaffected (separate formats).
- **B. Consume, don't clone, at the adapter boundary** [est.
  1–2 ms; no format change]. Encode does `ceiling().clone()` +
  `message().clone()` off a leaf node it then drops; an
  `into_parts(self)` on the `Leaf` trait saves a `Version` clone
  (ITC allocations) and an `Arc` bump per leaf per side. Touches
  the same adapter surface as A — natural rider.
- **C. Ship leaf paths, skip derivation** [est. ~1 ms compute for
  ~5–15 B/leaf wire; format change, unlocked by the adversary
  decision]. The wire's flat leaf runs force decode to re-derive
  each path (3 blake3) purely for *placement*. Sorted runs
  delta-compress paths well (shared-prefix length + suffix).
  Interacts strongly with A — the run frame is where the paths
  would live — so C should be designed with A, or explicitly
  after it. Keep derivation under `debug_assertions` as the bug
  tripwire.
- **D. Cheapen the codec's speculative-parse error path** [est.
  ~0.9 ms, shared with V1 so roughly parity-neutral; `before`
  crate]. Both protocols burn ~0.9 ms/session constructing and
  dropping `before::error::Decode` values on the happy path —
  probe parses in the bit codec construct real error values.
  Improves absolute time everywhere.
- **E. Batch the spine-wrap hash** [est. ~7–8 ms *per protocol*,
  parity-neutral; a coordinated hash-format break]. `hash()`
  folds a compressed prefix one `Hash::branch` per byte: a
  28-byte spine costs 28 blake3 calls at first read — the same
  per-virtual-level disease §5.4 cured at the conversion
  boundary, alive in the hash convention. A one-compression
  spine wrap (`blake3(SPINE_TAG ‖ prefix ‖ child_hash)`) is now
  the largest absolute compute lever in the whole system, but it
  changes every tree hash — both protocols, all snapshots, any
  persisted state — so it moves total sync time, not the V2−V1
  gap. A design note of its own, not a parity play.

Fan-out protocol: each lever in its own worktree branched from a
common baseline commit, measured against the same four §8 cells
before merging; A and B may share a branch (same surface), C
declares its dependency on A's frame shape. Levers touching the
wire re-accept snapshots deliberately and in isolation, so each
diff's snapshot delta reads as exactly its own format change.
Realistic outcome [derived]: A+B+C recover 6–8 of the 11 ms,
putting V2 within ~15 % of V1 on equal-hop workloads with the
remainder in the diffuse tail; D and E then move both protocols'
absolute times rather than the gap.
