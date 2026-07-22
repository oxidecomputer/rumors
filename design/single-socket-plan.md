# The single-socket refactor: execution plan

Companion to `single-socket.md` (the rationale of record — read it first
for *why*; nothing here re-argues a decision made there). This document
is the *how*: stages as a dependency DAG, concrete tasks with verified
`file:line` anchors, tests named before code, and gates per stage. Every
anchor below was re-derived from the code on this branch
(`single-connection` at `9cc3c79a`); where the design doc's citations
drifted, the code wins and §8 records the discrepancy.

Theorem-status snapshot (2026-07-21; the landed status of record is
`formal/lean/StreamingMirror/Mux/Statement.lean`; sweep §3's
reconciliation table at each stage
boundary): T3 `wc_impossibility` ✓ kernel; T4 `sigmaStar_deadlock_free`
✓ kernel; termination ✓ kernel; `elastic_deadlock_free` ✓ kernel (seam
closure in flight, track T10); `wc_impossibility_K` ✓ kernel for
KR ∈ {1,2,3}, ∀KI, KR ≥ 4 [derived]; T8 `sigmaStarK_deadlock_free`
stubbed, in the campaign's queue — reconciles with S1 (§3); σ\*-causal
(the local witness whose closure S1 transcribes) in flight; T5/T6 in
flight. **No stage waits on any of these** — see §3 for the stance.

## 0a. Standing note (2026-07-22): the contingency posture

Per the conclusion of record (main: `formal/doc/exposition.typ`
@consequence; the T11 charter in
`formal/lean/StreamingMirror/Mux/Charters.lean`; `single-socket.md`'s
revision of record), the `Link` contract stands as the library's
product surface and this plan is the CONTINGENCY plan. The stages were
built independently valuable and remain so under that posture: R0's
decisions, R1's receiver widening and greeting window advertisement,
and the acceptance harness benefit `link-transport` itself
(heterogeneous-window interop on any transport; the K-dial law —
`mux-latency.md` §7 — is now the principled tuning guide for
`Window::scopes`). S1, the σ\*ₖ engine, is the shelf's one unbuilt
component, with its liveness theorem already landed ahead of it
(`sigmaStarK_deadlock_free`); M1/V/L execute only if a deployment
without multi-stream transports materializes. Stage L's gate is
expected never to fire; that expectation is the plan succeeding, not
stalling.

## 0. The stage DAG

Edges below are **code dependencies only** — a stage needs another's
artifacts in the tree, nothing else. Theorem work runs concurrently
(§3) and gates nothing.

```
start now, in parallel:   R0    R1    S1.1 (ledger + parity tests)    A (harness)
                                 │        │
   R1.4 peer_window field ───────┼────> S1.2 (send gate)
   R1.2 widened queues ──────────┼────────┼────> M1 (socket transport)
                                 │        │        └─> V (acceptance runs)
                                 │        │              └─> L (Link removal;
                                 │        │                   irreversible)
```

**Start now, zero dependency on anything in flight:** R0.1 (one
question), R1.1, R1.2 — and S1.1's ledger scaffolding plus A's
socket-pair harness helper are equally unblocked (file-disjoint from
R1; §9's parallelism table). Link carries all traffic until L. R1, A,
and S1.1 are useful and safe even if everything after them stalled
forever.

## 1. Stage R0 — spikes and decisions (no production code)

Entry: none. Exit: three recorded decisions + two spike notes. Risk:
low — the failure mode is starting R1 with an unmade decision and
churning the greeting twice.

- **R0.1 — V2 release status → amend-V2 vs mint-V3.** Owner: **Finch**.
  Code fact: `Protocol::V2 = 2` (`src/protocol.rs:25`); the preamble
  gate rejects version mismatches before any frame content is trusted
  (`src/lib.rs` wire-compatibility section, ~:241–260: "a wire change
  introduces a new protocol version rather than silently changing an
  existing one"). If V2 is unreleased (it ships with link-transport
  itself), R1.3 amends V2's greeting; else R1.3 mints V3. Unblocks
  R1.3. One question, zero code.
- **R0.2 — snapshot policy for socket interleaving.** Owner: **Finch**.
  Per-stream captures (`remote/codec/capture.rs`, snapshots under
  `remote/codec/snapshots/`) stay stable off the greeting; a
  whole-socket interleave capture is not pinnable (cross-stream order
  was never deterministic — campaign audit finding A10). Decision to
  record: per-stream projections are the only pinned artifacts; V.4
  implements it. Unblocks M1.2/V.4 wording, gates no code.
- **R0.3 — `ProxyLocalQuestions` depth derivation** (carried [open]
  from `eager-absorption.md` §7.2). The queue is already window-wide
  (`remote/proxy/work/queues.rs:32–37`, capacity from
  `Window::scopes()`); the spike derives its true occupancy bound from
  the walk's channel capacities and lands the derivation in
  `window.rs`'s module doc. Gates nothing; timeboxed half a day.
- **R0.4 — monomorphization baseline.** `cargo llvm-lines` snapshot of
  the current `remote/` instantiation cost, so M1's socket module has
  a number to hold (the design doc's §8.9 concern). Record in the
  spike note; re-run in V.6.
- **R0.5 — reconciliation ritual.** One-liner: at each stage
  boundary, re-read `Mux/Statement.lean` and sweep §3's
  reconciliation table — reconcile, don't wait. A landed theorem with
  a different shape is a diff to apply, not a blocker that was
  secretly there all along.

## 2. Stage R1 — receiver widening + window advertisement (~150–300 lines + ~400 test)

Entry: R0.1 decided. Exit: `just gate` green; both historical seeds
green (transport untouched — these tests exercise the Link path and
must not notice R1); snapshots re-accepted once. Risk: low; failure
mode is greeting churn (mitigated by landing the greeting change once,
R1.3+R1.4 in one commit) and silent memory regression (mitigated by
R1.1 landing first).

Tasks, in order:

- **R1.1 — test first: parked-reply memory accounting.**
  New test beside `remote/adapter/tests/`: decode a maximally
  disputed reply and a max-fan supply run; assert the parked decoded
  form holds node *handles* (shared structure), not copies — the
  invariant that makes reply-denominated K RAM-sound. (The ≈ 1.1 MB
  encoded / ≈ 2 MB transient figure: `streaming/message.rs:14–17`.)
  Invariant sentence: "a parked decoded reply costs O(fan) handles,
  never a subtree."
- **R1.2 — widen `ProxyResponses`.**
  `remote/proxy/work/queues.rs:21–24`: `responses<T, H>()` grows a
  `capacity: usize` parameter like its two siblings (:32, :41). Call
  site: `remote/proxy/work.rs:120` (`self::queues::responses::<_, H>()`
  inside `respond()`), which has `self.window` in scope (`work.rs:47`,
  `:81`) — pass `self.window.scopes()`. REWRITE the two doc comments
  that state the one-slot rationale, else the docs lie:
  `queues.rs:11–14` ("its single slot is what bounds decoded replies
  in flight per stage") and `work.rs:108–113` ("One buffered response
  is sufficient…") — both become "the per-stream window buffer, K
  deep; under the Link transport this is inert extra parking"
  (cite `single-socket.md` §1.4). Test: existing capacity-floor tests
  stay green at `Window::FLOOR` (test builds pin K = 1,
  `window.rs:100–114`); add one widened-window smoke
  (`Window::from_nodes` large) through the Link path.
- **R1.3 — the greeting's window field.**
  Struct: `streaming/message.rs:52–58` — `Handshake` gains
  `pub window: u32` (per-stream parking capacity, reply-denominated;
  document the default source `Window::scopes()` and that 0 is
  invalid). Wire: the greeting is two length-framed fields written in
  `remote/proxy/start.rs:205–216` (`send`: version frame, then borsh
  listing frame) and read in `:224–235` (`receive`, which validates
  the peer-controlled listing) — add the third frame, borsh `u32`,
  validated `>= 1` at decode (a zero or absent advertisement is a
  handshake error, same class as a non-canonical listing). Populate at
  both construction sites: `start.rs:124–127` (`connect`) and
  `:179–184` (`accept`). Version per R0.1. Snapshots: re-accept
  `tests/gossip_snapshot.rs`, `tests/bootstrap_snapshot.rs`, and the
  codec captures in the SAME commit (repo hard rule: deliberate wire
  change, one conscious re-acceptance). Test first:
  `handshake_carries_window` — encode/decode round-trip + the
  validation rejections; extend `tests/handshake_liveness.rs`'s
  12-cell one-byte-window matrix to confirm the fatter greeting still
  completes at every cell (the strict-alternation property is why the
  greeting is a non-edge — keep it pinned).
- **R1.4 — record the peer's window.**
  Thread `remote.window` from the decoded `Handshake` through
  `connected()` (`start.rs:240–260`) into the session state
  (`remote/proxy/state.rs`) as `peer_window: u32`, stored and
  deliberately unconsumed until S1 (doc comment says exactly that,
  citing `single-socket.md` §3.1 — "liveness never depends on the
  value advertised, only on the sender honoring it"). Invariant test:
  asymmetric session (FLOOR one side, widened other) completes over
  Link and each side reports the other's value.
- **R1.5 — the context-registration-causality proptest.**
  The receive-side mirror of announcement-completeness
  (`single-socket.md` §2 item 4): every arriving reply finds its
  `Scope` already registered by a prior local emission. The ledger is
  `remote/adapter/scope.rs:11–19` (parent prefix + positional radices
  + cursor); registration happens at encode time
  (`adapter.rs` module doc: "attaches each newly asked scope to the
  exact outgoing frame which makes the question publishable"; release
  on write success via `Encoded::write_with`). Proptest over random
  tree pairs through the in-memory driver: decode never reaches for a
  scope that is not the FIFO head. Invariant sentence: "no wire reply
  ever arrives before the question that scopes it was flushed."
- **R1.6 — gate pass.** `just gate` (justfile:105 — fmt-check,
  doclint, testdoc, readme-check, clippy, docs, docs-internal,
  test-all, doctest). `testdoc` will hold every new test to its
  invariant sentence; `readme-check` catches the crate-doc drift if
  R1.3's docs touch `lib.rs`.

## 3. The concurrent verification workstream (reconciliation points, not gates)

The stance of record, verbatim from Finch: **"We want to assume the
verification will work and implement the algorithm we just realized can
possibly work."** Implementation starts now and proceeds concurrently
with the theorem effort; no stage's entry waits on a theorem landing.
The basis for proceeding is the **evidence tier**: the 4,970-run causal
σ\* sweep (stage-0 gate P1), the 54-cell K-dial validation
(probe-exact, both corners), and the 2,150-run structural sweep. The
kernel proofs harden certainty — their landing is *expected, not
awaited*.

Where the workstream lives: branch `mux-conjectures`, worktree
`/Users/oxide/src/rumors-mux`; the landed status of record is
`formal/lean/StreamingMirror/Mux/Statement.lean`. In flight at
writing: **T8** `sigmaStarK_deadlock_free` (stubbed, with the
per-direction `(K_I, K_R)` parameterization already recorded);
**σ\*-causal** (branch `mux-causal`) — the causal closure that *is*
the S1 engine's inference spec; **T10** capacity monotonicity + the
elastic seam closure (branch `mux-t10`); **track E** oracle/necessity
(branch `mux-s3e`).

Reconciliation points — what gets revisited **if** a theorem lands
with a different shape, or a probe/proof finds a wedge:

| Implementation stage | Reconciles against | Action on landing / on a wedge |
|---|---|---|
| S1.1 ledger inference rules | σ\*-causal's `inevitableA` guard set | adopt the landed guard set **verbatim**; any divergence between it and S1.1's transcription is a bug on whichever side diverged, adjudicated against the probe traces |
| S1.2 send gate + R1.4 `peer_window` | T8's final statement shape | the per-direction form is already the spec here; if T8 lands narrower, the theorem widens, not the code |
| M1.3 admission/Violation logic | any late-arriving correction to admission rules | the fail-fast design contains the blast radius: an admission bug is a loud, attributable `Violation`, never a silent wedge — a correction is a local rule change plus a regression test |
| R1 widening | T10's elastic seam closure | none expected (R1 rests on the landed `elastic_deadlock_free` + controls); if the seam closure surprises, R1 is still Link-safe (inert parking) |
| any stage | a wedge found against σ\*ₖ anywhere (probe or proof) | the wedging skeleton becomes a committed regression test on this branch; the design doc's §4 posture note is updated; S1's gate rules amended per the finding |

The Lean names, once final after the campaign's legibility pass, get
cited in S1/M1 doc comments — by name only, never restated.

## 4. Stage S1 — the σ\*ₖ engine (~1–2.2k lines + test apparatus)

Entry: S1.1 has none — new files plus tests, start now. S1.2 needs
R1.4's `peer_window` field in the tree (a code dependency, one field).
Reconciliation: §3's table, first two rows. Exit: engine green under Link
(it gates sends identically under either transport — develop and test
it against the Link path, where a bug stalls rather than corrupts);
transcription-parity suite green. Risk: **the** stage risk — an
occupancy ledger that undercounts is the session-fatal Violation edge
(`single-socket.md` §3.4c); mitigated by S1.3's direction-asserted
hooks, the fail-fast containment (§3 table, row 3), and reconciliation
against σ\*-causal's guard set when it lands.

- **S1.1 — the occupancy ledger.** New module
  `remote/proxy/work/ledger.rs` (sibling of `progress.rs`, which
  already observes per-height decode/emit events — read it first;
  extend rather than duplicate if its counters suffice). Per-stream:
  replies started (counted at the encode pump's reply boundary),
  consumption evidence (arrivals whose content is causally downstream
  of a consumption — **per-channel order only**, audit finding A10),
  and the inevitability closure for silent consumptions — transcribed
  today from the stage-0 probe's Python reference (`causal.py`, the
  4,970-run-validated implementation), reconciled verbatim against the
  Lean `inevitableA` guard set when σ\*-causal lands (§3).
  Test-first: transcription-parity — replay the campaign's pinned
  families (wedge, combs, all-M tails, provision walls) through the
  Rust ledger and assert decision agreement with the pinned
  probe traces.
- **S1.2 — the send gate.** At the encode pump's reply start (the
  producer loop in `remote/proxy/work/pump.rs` / `encode.rs` — the
  reply boundary is where `RunBudget` batching begins), gate: stream s
  admits a new reply while `< peer_window` prior replies on s are
  un-provably-consumed. Frames of a started reply flow freely
  (reply-atomicity; pumps never park mid-reply). `End` controls are
  exempt.
- **S1.3 — soundness hooks** (`test-internals`-gated assertions):
  the ledger's unconsumed estimate **never exceeds** the true count
  (over-estimation = latency bug; under-estimation = the deadlock
  bug — assert the direction against ground truth the in-memory
  driver can see).
- **S1.4 — the honoring assertion**: the sender-side twin of the
  receiver's violation check — never start reply `peer_window + 1`.
  This is `single-socket.md` §5.2's clause, implemented early so V
  only has to *run* it.

## 5. Stage M1 — the socket transport (~400–700 lines)

Entry: R1.2's widened queues and S1.2's gate interface in the tree
(code dependencies; the writer mux consults the gate — a stub gate
compiles M1 earlier if sessions overlap). Exit: a full session completes end-to-end over one
in-memory duplex at K ∈ {1, default}; gate green. Risk: moderate —
mostly careful reuse of the end/error discipline; the scheduler is
safety-free (`single-socket.md` §3.3) and tunable post-landing.

- **M1.1 — module `remote/socket.rs`** (naming settled by Link's
  removal: it is the replacement, not a sibling). Owns the session
  over one `AsyncRead + AsyncWrite` pair: preamble/greeting/epilogue
  bytes unchanged — the control stream IS the socket; data frames
  interleave behind it after the greeting.
- **M1.2 — the writer mux.** Priority ladder (§3.3: session control >
  frontier control > active-descent data, deepest-first > bulk), with
  chunk-boundary preemption at `RunBudget` boundaries
  (`remote/codec/budget.rs`), S1.2's gate consulted at reply starts.
  Frames self-identify (`(Stream, Frame)` through
  `FrameWrite::frame`, `streams.rs:180`) so no additional framing.
- **M1.3 — the reader demux.** One reader per direction; routes by the
  frame's stream component (`FrameRead::frame::<T>()` yields
  `(Stream, Frame)` — `streams.rs:374`); per-stream `End`/AfterEnd
  discipline preserved exactly as `read_frames` does it today
  (`streams.rs:395–414` — lift, don't rewrite); an arrival that would
  park reply `local_window + 1` on one stream is a `Violation`
  through the session's one-slot error route (`error_route()`,
  `streams.rs:469–478` — reuse), never backpressure. Test-first: the
  never-block assertion — the reader is never blocked by any pump
  queue (V's transmuted conformance clause, written here).
- **M1.4 — session integrity relocation.** `SessionState`
  (`link.rs:161–215`: epoch counter + poison latch, `begin`/`finish`)
  moves conceptually to the socket session: the poison latch semantics
  are unchanged (mid-frame interruption poisons; only clean completion
  clears); the epoch's *stream-label* role has no socket counterpart —
  keep the counter as a diagnostic pending L.3.
- **M1.5 — transitional entry points.** Sibling public constructors
  beside the Link path (e.g. `Peer::bootstrap_socket(read, write)`
  mirroring `peer.rs:235–244`, and the `Rumors` gossip analogues) —
  additive, documented as the target interface; the Link methods keep
  working until L. No feature flag: both paths compile, tests exercise
  both.

## 6. Stage V — acceptance (the workstream A tasks, run for real)

Entry: M1 merged. Exit: every gate below green; soak clean. Risk: low
per task; collectively this is what makes L safe.

- **V.1 — the seeds, on the socket.** The two historical stall
  regressions (`tests/pairwise.proptest-regressions`,
  `tests/shadow_validity.proptest-regressions`) replayed over the
  socket transport at K ∈ {1, 2, production-default} **and asymmetric
  {K=1 vs default}** — the asymmetric cells are load-bearing
  (§3.4c: the full-window edge is exercised by a small window facing a
  productive peer). Harness: extend `tests/common/wire.rs` (the
  `block_on`/`run_to_quiescence` deterministic-deadlock harness —
  `rumors::testing::run_to_quiescence`, re-exported at the crate
  root, NOT `streaming/testing.rs`, which is the fault-injection
  module) with a socket-pair constructor beside `MemoryLink`.
- **V.2 — transmuted conformance assertions** (from
  `single-socket.md` §5.2): routing (every frame reaches the pump its
  stream component names), never-block (M1.3's test, promoted),
  priority observable behavior (frontier control never queued behind
  bulk by more than one chunk), inference direction (S1.3), honoring
  (S1.4).
- **V.3 — soak.** Extended randomized-schedule runs at mixed window
  sizes; a `just` recipe (`soak`, sibling of `test-all`,
  justfile:44–48) with seed-count override, run before L and on
  demand, not in the inner gate.
- **V.4 — snapshots per R0.2**: per-stream capture pins only; assert
  no test pins whole-socket interleaving.
- **V.5 — docs pass**: `remote.rs` module doc (currently Link-language
  throughout, `remote.rs:1–56`), `window.rs` (K's second consumer:
  the advertisement), `link.rs` module doc gains the transitional
  notice; the campaign's MODEL.md scope note (audit A6) belongs to the
  campaign branch — cross-reference only.
- **V.6 — `just all`** + the R0.4 llvm-lines re-check.

## 7. Stage L — Link removal (irreversible; last)

Entry: V complete including soak; **Finch's explicit go** (this is the
one stage that breaks external users). Exit: gate + `just all` green;
README regenerated (`readme-check` — `link` is in the crate docs).

- **L.1 — deletions**: `src/link.rs` (and its `STREAM_COUNT` — the
  codec's `Stream::COUNT`, `remote/codec/signal.rs:29`, is the
  survivor; the cross-assert `codec_stream_count()` in `remote.rs:71`
  dies with its twin); the `conformance` cargo feature and suite
  (`lib.rs` cfg at the module list); `remote/streams.rs` entire
  (StreamSender/Receiver, AcceptDriver, claims, labels); the
  transitional Link entry points from `peer.rs`/`rumors.rs`.
- **L.2 — migration notes**: `Link` users (rumormill's iroh binding →
  one bidirectional stream as the `AsyncRead + AsyncWrite` pair);
  CHANGELOG-grade doc in the crate root.
- **L.3 — epoch fate** (owner: Finch): diagnostic counter or deletion.
- **L.4 — sweep**: grep for `link`-language in every module doc
  (`streams.rs`'s "supplied by the link contract, not reconstructed
  here" died with its module; `remote.rs`, `peer.rs` examples at
  `peer.rs:56–89` use `rumors::link::memory()` — replace with the
  socket constructor).

## 8. Design-doc/code discrepancies found while anchoring (code wins)

1. `responses()` takes no capacity argument today
   (`queues.rs:21–24`) — the design doc describes the *change*
   correctly but cite-checks should use the current signature.
2. The greeting's wire form lives in `start.rs:205–235` (two framed
   fields), not in `message.rs` (which holds only the struct) — R1.3
   touches both; the design doc cites only the struct.
3. The poison latch is `link.rs:161–215` (`SessionState` + impl), not
   `:173–214` as the design doc has it — trivial drift.
4. `run_to_quiescence` is `rumors::testing` at the crate root
   (consumed via `tests/common/wire.rs:14`), not
   `streaming/testing.rs` (fault injection) — matters for V.1's
   harness work.
5. Two one-slot rationale doc comments must be rewritten with R1.2
   (`queues.rs:11–14`, `work.rs:108–113`) or the docs contradict the
   code — doclint won't catch semantic drift; the task list does.
6. `Window` is `pub(crate)` (`window.rs:71`) — R1.3's default
   advertisement is derived where the session already holds a
   `Window` (`start.rs` construction sites), no visibility change
   needed.

## 9. Estimates and the critical path

| Stage | Diff size | Agent-sessions | Risk |
|---|---|---|---|
| R0 | ~0 (notes) | ½ | low |
| R1 | ~150–300 + ~400 test | 1–2 | low |
| S1 | ~1–2.2k + tests | 3–5 | **high** (bounded by T8) |
| M1 | ~400–700 + tests | 2–3 | moderate |
| V | ~300–600 test | 1–2 | low |
| L | ~−1.5k, +docs | 1 | low, irreversible |

Critical path (code dependencies only): **R1.4 → S1.2 → M1 → V → L**,
with R0.1 feeding R1.3's version choice. No theorem appears on it.

Honest parallelism — what runs in concurrent agent-sessions without
merge conflicts, by file-disjointness:

- **R0** (notes only) ∥ everything.
- **R1** (`queues.rs`, `work.rs:120` + its doc comment, `message.rs`,
  `start.rs`, `state.rs`, snapshots, adapter tests) ∥ **S1.1**
  (`ledger.rs` NEW + its parity tests; reads `progress.rs`, touches
  nothing R1 touches) ∥ **A** (`tests/common/wire.rs` + new test
  files only).
- The one deliberate serialization: **S1.2** (`pump.rs`/`encode.rs`)
  waits for R1.4's `peer_window` field to exist — a one-field code
  dependency, not a review boundary.
- **M1** (`remote/socket.rs` NEW + entry points in `peer.rs`) overlaps
  S1's tail if S1.2's gate trait is stubbed first.
- **V** runs against whatever is merged; **L** alone is strictly
  serial and last.

If the theorem workstream stalls entirely, every stage still lands on
evidence-tier confidence; §3's table says what to revisit when it
unstalls.

## 10. Parking lot (deferred ≠ lost)

| Item | Trigger to revisit |
|---|---|
| Byte-budget window dial (§5A's states 170..=203, reserved) | a deployment where 17·K·2 MB worst-case parked RAM is unaffordable, or §3.4d's storage-churn accounting binds in practice |
| Ladder tuning | measurement after M1; safety-free, never gates |
| Epoch counter fate | L.3, Finch |
| Loss-coupling measurement vs QUIC single-stream | when a lossy-network deployment materializes; §6 is [derived] only |
| `ProxyLocalQuestions` bound derivation into window.rs docs | R0.3's spike note, land opportunistically |
| Erasure/`dyn` seams for the socket session | if `height-erasure.md`'s concerns meet M1's module in practice |
