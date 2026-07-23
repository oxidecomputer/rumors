# Review packet: link-transport → main

> **Where these commits live.** Every commit hash in this packet —
> the pin below and all per-series review-fix pointers — resolves
> on the archive branch `wave1/integration`, not on
> `link-transport`: the reviewed work was cherry-picked onto
> `link-transport` under new SHAs. Keep `wave1/integration` fetched
> to follow the pointers.

Point-in-time review aid, pinned at `069ec491` on
`wave1/integration` (2026-07-18). 72
commits, 198 files, +19.1k/−5.1k excluding `formal/`. Every
substantive change below already survived its own adversarial
review round and landed gate-clean; per-series pointers to the
review-fix commits are included so the merge review can focus on
cross-series interaction and taste rather than re-derivation.

## The tally

| category | files | Δ |
|---|---|---|
| product `src/` (rumors)          | 63 | +5,277/−1,580 |
| product `crates/before`          | 10 | +431/−46 |
| test siblings (`src/**/tests`)   | 43 | +5,343/−1,154 |
| integration `tests/`             | 34 | +3,032/−585 |
| wire snapshots (`.snap`)         | 24 | +694/−1,500 |
| `design/` docs                   | 5  | +3,044/−0 |

The must-read core is the ~5,700 added product lines; tests are
invariant-documented and lower review density; snapshots were
byte-verified at each re-accept; docs are the intent record for
every code change.

## Suggested order (by semantic risk, not chronology)

1. **Series E — single-preimage hashing** (small, highest weight
   per line): `src/tree/typed/hash.rs`, `untyped.rs::hash`.
2. **Series F — greeting-carried opening** (wire format + pairing
   integrity): `remote/proxy/start.rs`, `materialized.rs`,
   `adapter/encode.rs`, `work/pump.rs`.
3. **Series C — supply-run batching** (largest single chunk):
   `codec/frame.rs`, `codec/budget.rs`, `adapter/{encode,decode}.rs`.
4. **Series B — `before` codec ladder** (backed by differential
   oracles): `codec/{cursor,gamma,tree}.rs`, `borsh_impls.rs`,
   `version/{compare,batch}.rs`.
5. **Series A — transport rework** (oldest; had its own
   RED/GREEN conformance cycle and cleanup passes).
6. Series D, G, H (instruments, hardening tests, docs): skim.

## Series A — Link transport rework (pre-campaign)

`06dc10c2..d571c21f` (26 commits). Per-stream sessions replace
the mux (fixing the streaming wire deadlock; determination in
`design/streaming-wire-deadlock.md`), `SessionState`/link
poisoning, the V2 epilogue completion contract, the conformance
suite's soundness holes demonstrated RED then closed GREEN,
rumormill merge-verdict modeling, and four cleanup passes.
Key commits: `741a1143` (the transport), `b65828f8` (epilogue
certifies peer completion), `d07b2ac9` (poisoning),
`020ecb49`/`1453618d` (conformance RED→GREEN).

## Series B — measurement + the conversion-boundary fix

`39048381..e63b2850` (16 commits, interleaved code/docs).
- Product: `39048381` (`Peer::max_in_flight_nodes` window
  pipelining), `88baceea` (bulk leaf conversion at the Local
  boundary — the 82 % gap cut, §5.4), `e1b25e13` (query-first
  dequeue), `d92da79d` (capacity coverage repair).
- Instruments: `7550a7ff` (delayed-pipe bench links, paused
  clock), `34a53849` (hop tracer), `6e8c418c` (bench line
  tables).
- Docs: `ab1e870d` + five §-commits building
  `design/streaming-latency-serialization.md` §5–§11, including
  the trusted-counterparty [decision] (§10) and the hop ledger.

## Series C — `before` low-hanging fruit (D-ladder + sweep)

`0003ad4c..afd94df3` (6 commits; sweep record in
`design/before-lowhang-sweep.md`).
- `0003ad4c` byte-equality Version eq; O(1) `is_empty`.
- `1636f9d4` join/meet lattice-identity short-circuits (+ the
  `Batch::materialize()` test hook — note the identity-join
  test idiom this replaced).
- `d01e08aa` D1: fieldless cursor errors (asm-verified).
- `dcca3974` D2: word-window gamma decode/skip/encode — the
  per-bit loop remains sole arbiter of every reject (canonical
  identity by construction); differential suites against the
  pre-window cursor as oracle.
- `6cab66f2` D3: ReaderCursor's buffer is both decode window and
  the stored value (zero-copy finish); smallvec parse stacks.
- Measured: V2 ins 31.6→29.0, V1 20.7→19.2 ms; ratio unchanged.

## Series D — lever A+B: supply-run batching

`3009ece8..58076c35` (5 commits). One `Supply` frame carries a
byte-budgeted run of leaf records; `Peer::target_message_size`
(default 1,114,624 B = maximally-disputed-reply derivation);
lever B landed as borrow-based serialization (`into_parts`
impossible: encode-side leaves are Arc-shared with the tree).
Review-fix commit `773ad56b`: whole-frame budget accounting,
eager u32 capacity check, knob pinned on the wire by frame
counts, multi-record run under the snapshot pin.
**Measured null on symmetric divergence** (batching factor 1.017
at I=5000 — singleton subtrees at the dispute frontier); kept
for clustering workloads and as lever C's future home
(`58076c35` records the analysis).

## Series E — lever E: single-preimage node hashing

`86a5ec50..e6e05143` (6 commits; design of record
`design/node-hash-preimage.md`, superseding the spine-wrap
draft). `blake3(TAG ‖ prefix_len ‖ prefix ‖ u16 count ‖
(radix ‖ hash)*)` — one compression per frontier node vs ~30.
**Compression invariance moves from by-construction to
by-canonicity**: rests on the ≥2-children maximal-compression
invariant the untagged serializer already depends on; pinned by
canonicity + virtual-level proptests + injectivity units.
Review-fix commit `1b2770ab`: call-site debug_asserts (radix
order, no one-child fan), u16 high-byte pin, `# Panics`.
All hash values changed, both protocols; layouts untouched
(byte-verified). No version gate [decision: nothing deployed].
Measured: V2 29.5→19.25, V1 19.6→9.20 ms; gap unchanged ~10 ms.

## Series F — the hop campaign

`f0a3cc7d..f8c222b5` (6 commits incl. merge `2f0e5ce6`).
- `fe82aa5b` + fix round `f2a0063d`: V2 greetings carry root-fan
  listings (second control frame); elected responder answers the
  opening from the greeting; opening stream never opens.
  Divergent sessions 8→7 hops (trace scale). Always-carry
  [decision]; single listing derivation (`fan_listing`) after
  review; listing-ingress validation tests. Bench-confirmed
  `6f90fe68`: every delayed cell −1 one-way delay exactly
  (d=100: −98.9/−99.5 ms), converged cells pay the documented
  listing cost.
- `f0a3cc7d`: **version hop retained** — read-only speculation
  unsound (linearity re-derivation); fork-up-front rejected for
  party-space fragmentation under bootstrap contention. Full
  implementation preserved at `3677920c` (reflog).
- `f8c222b5`: **tail marker retained** — one hop not worth
  weakening what `Ok` certifies. Ledger closed: 8 hops at
  I=5000, 3-hop heartbeats; remainder inherent or deliberate.

## Series G — review-to-gate hardening

- `c053202f` party conservation: disjointness, join-conservation,
  donated-once, and the fragmentation bound (bit-exact
  renormalization under interleaved bootstrap/retire — pins the
  regression class that killed the version-hop design).
- `8edb3509` **product change**: V1 greeting send/receive under
  `try_join` (was symmetric write-then-read: deadlocks once the
  version frame outgrows the transport window). Bytes unchanged.
- `1c5a38e3` 12-cell liveness matrix over one-byte-window links;
  proven by revert (5 V1 cells deadlock without `8edb3509`);
  `Link` contract now states any positive capacity suffices.
- `a0a30038` clean-drain invariant: control streams rest empty at
  every successful session boundary (17 sites + ~20 inheriting
  suites + negative control; tripped on nothing existing).
- `b4347628` ingress inventory (module doc in `mirror.rs`) +
  systematic malformed coverage for the thin ingresses (V1
  `recv_msg`, party frame, epilogue); no wire-reachable panic.

## Series H — post-campaign

- `069ec491` Force BoxResponses for Initiator (finch, direct).
  Outside the campaign's review rounds and outside the final
  cross-interaction review's pinned scope (`b4347628`).

## Cross-cutting facts the merge review can lean on

- Three adversarial review rounds (A+B, E, opening-fold) found
  **zero bugs**; all findings were hardening/test-gap tier and
  every one was addressed in a named fix commit before merge.
- Wire snapshots were re-accepted exactly three times, each in
  isolation, each byte-verified: supply-run framing (V2 only),
  hash values (both protocols, values only), greeting listing +
  opening removal (V2 only). V1 bytes changed in none of them.
- Hop counts are pinned exactly by `tests/hop_trace.rs` inside
  the gate; identity invariants by `tests/party_conservation.rs`;
  liveness by `tests/handshake_liveness.rs`; drain by the
  harness assert. The gate at `b4347628`: 812 tests, all green.
- A final cross-interaction adversarial review (hash×listing,
  framing×budget, cursor×ingresses, drain×staging, shared
  assumptions incl. the full-snapshot-diff attribution scan) ran
  at pin `b4347628`: **zero correctness findings at any
  severity**. Verified along the way: the Merkle and
  content-address hash families stayed disjoint (`path.rs` has
  zero diff vs main); every snapshot hunk in `main...HEAD`
  attributes to one of the three sanctioned re-accepts; the
  gate's all-features passes do run the V1 cells; the
  quiescence oracle cannot mask a livelock (poll-budget
  failure). Its one actionable nit (`message.rs`'s "≈ 2 MB"
  reply figure, right only when both encoded and decoded copies
  are charged) is fixed in the closeout commit; its remaining
  for-the-record risks were all already-recorded decisions (the
  nothing-deployed assumption; the uncapped trusted-peer frame
  lengths; quiescent-boundary-only conservation equalities).

## Measured outcome of the whole campaign (d=0, quiet machine)

| cell | campaign start | post-§5.4 | at pin | Δ (whole campaign) |
|---|---|---|---|---|
| V2 insertions I=5000 | 87.8 ms | 31.6 ms | 19.4 ms | −78 % |
| V1 insertions I=5000 | 21.0 ms | 20.7 ms | 9.2 ms  | −56 % |
| V2 redactions R=2500 | 20.5 ms | 12.3 ms | 10.0 ms | −51 % |
| V1 redactions R=2500 | 7.2 ms  | 6.5 ms  | 3.9 ms  | −46 % |

(Campaign-start and post-§5.4 figures are
`design/streaming-latency-serialization.md`'s checked baselines —
its opening d=0 table and its §5.4 after-column. The
conversion-boundary fix is inside this packet's own Series B, so
the whole-campaign Δ is measured from the true start; the
post-§5.4 column is the baseline the per-series comparisons above
quote.)

Plus one full RTT removed from every delayed divergent session
(hop 9→8 at bench scale). Residual V2−V1 gap ≈ 10 ms, owned by
per-leaf allocation/channel glue; open levers: C (leaf paths in
the run records, ~1 ms) and D4 (decode-less-often, unmeasured).
