# The single-socket transport: undoing the Link abstraction with inferred windows

Status: design of record (2026-07-21, against `link-transport` at
`65092b4a`; revised same day per Finch's review — Link is transitional
scaffolding, not a retained alternative, and the receive window is
advertised in the greeting). This document supersedes
`streaming-wire-deadlock.md` §5A as the single-socket design of
record — §5A's credit mux is retired, not superseded by silence: the
mux conjectures campaign (`formal/MUX-PROGRESS.md`,
`formal/MUX-ADJUDICATION.md`, on the `mux-conjectures` branch) proved
that the credit *messages* carry no information the protocol does not
already announce, and the eager-absorption assessment
(`design/eager-absorption.md`, same branch) verified in the code that
the buffer a credit's grant promises can be denominated in *decoded
logical replies* — dissolving §5A's unit-mismatch discontinuity. What
remains of a credit scheme once its messages are inferred and its
buffers are the decoded replies the adapter already produces is: a
wider queue, a counter, one advertised integer, and a proof. That is
this design.

Epistemic key as in `streaming-wire-deadlock.md`: **[proven]** =
kernel-checked in `formal/lean` on `mux-conjectures`; **[checked]** =
verified by reading the cited code or by executable evidence;
**[derived]** = argued here from checked premises; **[open]** = needs
a spike.

The perspective, per Finch's directive: `link-transport` is the base
point, and the target end-state **undoes the Link abstraction
entirely**. The remote transport API returns to a single
`AsyncRead + AsyncWrite` pair per peer — the external interface the
crate had before the deadlock forced the multi-stream contract — and
carries all seventeen logical streams on it, deadlock-free with **zero
steady-state wire overhead and zero logical-protocol changes** (one
greeting field is added — §3.1; every non-handshake frame is
byte-for-byte unchanged), by the campaign's two results:

- **Receive side, eager conversion**: wire frames become logical
  replies at arrival (this is already the adapter's architecture —
  `remote/adapter.rs:52-60` decodes each record as it arrives through
  the fan-bounded channel into `Convert::assemble`, retaining only the
  reply skeleton and node handles). Parking up to K decoded replies
  per stream costs K·O(fan) handles, not K subtrees
  (`eager-absorption.md` §7.1 [checked]).
- **Send side, σ\*ₖ inferred flow control**: the sender starts reply
  r on stream s only while fewer than K prior replies on s are
  un-provably-consumed — K being the *peer's advertised* receive
  window (§3.1) — the proof built from its own pushes, decoded
  arrivals, and the inevitability closure over silent consumptions:
  "W = K credits inferred instead of sent"
  (`MUX-ADJUDICATION.md` §1.3, generalized).

## 1. The end-state, and Link as scaffolding

### 1.1 The target external interface

One peer, one connection, two byte-stream halves: the remote transport
API is `AsyncRead + AsyncWrite`, full stop. No `Connector`, no
`Acceptor`, no stream supply, no conformance obligations on the caller
beyond "reliable ordered duplex". The session's control traffic
(preamble, greeting, identity hand-off, epilogue) and all seventeen
logical streams' frames interleave on the one connection; frames
already carry their logical stream index in the dense signal byte
(`remote/streams.rs:22-27` — the codec was born mux-ready), so routing
needs no additional framing.

**Link is transitional scaffolding.** It remains in place, carrying
production traffic, while the receiver widening and the σ\*ₖ engine
are developed and validated (§7's staging); once the socket transport
passes the acceptance gates (§5) it becomes the only transport and the
Link machinery is deleted (§7 stage L). If the single-socket transport
works, the Link abstraction is removed, not retained as an alternate
path.

### 1.2 What the removal deletes, what it keeps

Deleted with Link (stage L):

- `src/link.rs` entire: `Link`, `LinkParts`, `Connector`, `Acceptor`,
  the in-memory instantiation (`memory`, `memory_with_capacity`), the
  `erased` funnel.
- The `conformance` cargo feature and its suite: its clauses validate
  a contract no caller implements anymore. Its *obligations*
  transmute into socket-transport assertions (§5.2) rather than
  vanishing.
- `remote/streams.rs`, the 1:1 stream-binding layer: `StreamSender`
  lazy opens, the 2-byte open label, `AcceptDriver` and claim slots,
  `StreamReceiver` label validation. The per-frame signal-byte
  restatement — today the label's cross-check — becomes the routing
  authority outright.
- The stream-label role of the session epoch. The `SessionState`
  poison latch is kept (below); whether the epoch counter survives as
  a diagnostic or dies with its only consumer is a stage-L detail
  decision [open].

Kept, because never Link-specific (each predates or sits beside the
contract):

- **The Window mechanism and its capacity plumbing** (`window.rs`,
  `Peer::max_in_flight_nodes`, the widened proxy edges) — §1.4 makes
  it this design's one dial.
- **The handshake improvements**: the greeting's root-fan listing
  (one-hop opening, `message.rs:30-51`), the epilogue marker's
  both-replicas-committed certification, the preamble version gate
  (`lib.rs:241-253`).
- **Session-boundary integrity**: the poison latch (a session
  interrupted mid-frame poisons the connection for later sessions;
  `link.rs:173-214` today) relocates to the socket session wrapper —
  it was always about the control stream's byte position, which the
  single socket has more of, not less.
- **BoxResponses** and the structural adversarial-input coverage for
  every wire ingress — codec/protocol-level hardening,
  transport-agnostic.
- The codec, the adapter, the proxy, the capture apparatus (§5.3 for
  what capture can still pin).

### 1.3 Negative space: what is lost, and that we accept it

Recorded per the house rule that the negative space stays on the
record:

- **Multi-stream transports stop being pluggable.** The Link contract
  let a deployment hand the protocol seventeen QUIC streams or
  seventeen TCP connections and buy the two transport-physics
  properties endpoint cleverness cannot reconstruct (§6): per-stream
  **loss isolation** (one lost segment stalls one stream, not all
  seventeen) and kernel/transport-managed per-stream flow control.
  Deleting Link deletes that option. A QUIC deployment still works —
  one bidirectional QUIC stream is a fine `AsyncRead + AsyncWrite` —
  but it degrades to the same loss-coupling class as TCP (§6). This
  is the price of the simpler external interface, and it is accepted
  by design: the deployments this crate targets pay the
  seventeen-fold connection/stream overhead and the contract's
  conformance burden on every integration, every day, while the loss
  residual costs only under loss and only in tail latency.
- The two instantiations that exercised the Link seam are affected
  knowingly: the integration tests' per-stream TCP link dies with the
  contract it tested; rumormill's iroh binding simplifies from
  streams-one-to-one to a single bidirectional stream [migration
  noted for its owner].
- §5A's credit bytes (reserved signal states 170..=203) remain
  reserved and unused: pacing-by-wire-credit stays a recorded
  alternative should a future deployment need pacing without proofs.
  This design needs none of them.

### 1.4 The Window relation: one knob, advertised

`window.rs` already denominates the session's pipelining in node
references and widens the two proxy scope edges
(`proxy/work/queues.rs:32-46`). On the Link transport, the third
edge — `ProxyResponses`, the decoded-reply relay — stays at one slot
*because the transport's flow control holds the rest of the in-flight
replies on the wire* (`queues.rs:11-14`). The socket transport
relocates exactly that bound: the decoded-reply queue becomes the
per-stream window buffer, K deep.

K is derived exactly as today — `Window::scopes()` from
`Peer::max_in_flight_nodes`, `Window::FLOOR` giving K = 1 — and is
**receiver-side configuration**: it prices the receiver's parked-reply
memory. It is therefore advertised, not assumed (§3.1): each sender
gates on the *peer's* advertised window. Test builds keep the floor
(`window.rs:100-114`), so every session shape stays exercised at the
K = 1 corner where a bad ordering *would* deadlock.

Memory accounting for the widened edge [checked, eager-absorption
§7.1]: a parked decoded reply is a `Vec<Reaction>` of ≤ fan entries —
a `Supply` is one node handle (shared structure, not a copy), a
`Query` listing ≤ fan hashes. Worst case ≈ fan² hashes ≈ 2 MB for a
maximally disputed reply; the provision case this design exists for is
O(fan) handles, the subtree's bytes having already streamed into
backend custody through `Convert::assemble`. Contrast §5A's byte
windows (17·(W + max frame) of *raw buffer*): the grant unit and the
buffer unit finally match, which is the whole reason reply-denominated
K > 1 is sound here and was not in §5A. Backend custody is storage the
transfer was going to consume anyway; abort reclamation is handle-drop
(`eager-absorption.md` §6 [checked for Local; documented obligation
for persistent backends]).

## 2. The receive side: eager conversion with K-deep parking

What exists [checked]: the demux-fed pump coroutines already decode
every frame at arrival and drive `Backend::parent` folds pre-cursor;
the `Scope` FIFO already registers, at question-emission time, exactly
the context decoding needs (prefix, listed radices, height); decoding
is batching-agnostic and never accumulates a subtree
(`remote/adapter.rs:39-60`).

What changes:

1. **`ProxyResponses` widens from 1 to K** (`queues.rs:22-24` takes a
   capacity argument like its two siblings), K = the local
   `Window::scopes()` — the same value advertised to the peer. Under
   the transitional Link transport this is inert-but-harmless extra
   parking; under the socket transport it is the window buffer.
2. **The socket demux**: one reader per direction, owning the read
   half; routes each frame by its signal byte's stream index to the
   pump input; preserves the `AfterEnd` discipline
   (`streaming-wire-deadlock.md` §8.10) against the explicit `End`
   controls.
3. **Over-window arrival is a protocol violation, not backpressure**
   [derived]. The peer received this side's window in the greeting
   (§3.1), so a frame that would park the (K+1)-th reply on one
   stream is *provably a peer fault* — no configuration skew can
   excuse it — and surfaces as a `Violation` through the session's
   one-slot error route (the publish-then-park discipline of
   `streams.rs:29-34`). The single reader never blocks on a full pump
   queue, so the §2 six-link cycle of the deadlock doc is
   *unconstructible* rather than merely avoided, and any inference
   bug on either side converts from a silent wedge into a loud,
   attributable failure.
4. **Context-registration causality** gets its named proptest: every
   arriving frame finds its decode context already registered by a
   prior local emission. Verified arm-by-arm across the message
   vocabulary in `eager-absorption.md` §3.3 [checked]; the proptest
   pins it against drift. It is the receive-side mirror of the
   announcement-completeness that makes σ\* local [proven-adjacent,
   MUX-ADJUDICATION §1.2].

One [open] carried from the assessment: the `ProxyLocalQuestions`
occupancy bound (questions in flight per stream) is not K-bounded from
below by anything structural; entries are tiny, but the spike deriving
its true bound from the walk's channel capacities is unfinished
(`eager-absorption.md` §7.2). It gates nothing here — the edge is
already Window-wide on the branch — but the derivation belongs in
`window.rs`'s docs when it lands.

## 3. The send side: advertisement and the σ\*ₖ engine

### 3.1 The window advertisement

K is receiver-side configuration, invisible on the wire until stated;
peers with different windows must interoperate. So the greeting states
it: the `Handshake` (`message.rs:53-58`) gains one field — the
sender's **receive window**, per-stream parking capacity,
reply-denominated. Each direction is independent: each sender's σ\*ₖ
gate uses the *peer's* advertised value for the direction it sends;
there is no `min()`, no negotiation beyond the advertisement, and no
constraint that the two ends agree. The default advertised value is
the local `Window::scopes()`.

A peer advertising K = 1 degenerates the counterparty's sender to
demand-lockstep — exactly σ\* — and the session remains live at every
capacity [T4, gated theorem; probe-cleared 4,970/4,970 at the causal
tier]. General K is T8's statement. Liveness never depends on the
*value* advertised, only on the sender honoring it; performance scales
as the frontier law (K scopes per RTT per stream — `MUX-LATENCY.md`,
forthcoming).

Consequences, recorded plainly:

- **This is a wire-format change, made deliberately.** It is confined
  to the handshake: the greeting frame gains a field, and **every
  non-handshake frame is byte-for-byte unchanged** (this supersedes
  the previous revision's blanket "wire format unchanged" claim —
  that claim now holds exactly off the greeting). Per the repo hard
  rule, `tests/gossip_snapshot.rs` and the insta pins are re-accepted
  as a deliberate protocol change, in the same commit that changes
  the greeting.
- **Versioning**: the preamble carries the protocol version and
  rejects mismatches before any frame content is trusted
  (`lib.rs:241-253`, `Error::VersionMismatch`), and the house rule is
  "a wire change introduces a new protocol version rather than
  silently changing an existing one" (`lib.rs:255-260`). V2 is
  unreleased — it ships with `link-transport` itself, the precedent
  §5A's own cost bullet set when it budgeted V2 wire changes. So: if
  this design lands while V2 remains unreleased, **amend V2** (the
  greeting is V2's greeting; there is no deployed old-greeting peer
  to interoperate with); if V2 has shipped by then, **mint V3**, and
  the preamble gate cleanly rejects cross-version pairs. In neither
  case does a window-advertising peer ever parse a windowless
  greeting — no in-band gating, no mixed-mode parser, no interop
  matrix. [derived from the preamble discipline]
- **Zero steady-state overhead**: one integer on a hop that exists
  anyway, nothing per-reply, nothing per-frame — the same trade the
  root-fan listing already made on the same frame
  (`message.rs:43-51`).

### 3.2 The σ\*ₖ engine

The engine gates one thing: **starting** a new reply on a stream.
Frames of a started reply flow freely (reply-atomicity: pumps never
park mid-reply — the structural fact §5A's W = 1 leaned on); `End`
controls are free. The gate: stream s admits a new reply while
strictly fewer than K_peer prior replies on s are
un-provably-consumed.

"Provably consumed" is built from three sources, all local
[proven for K = 1 at the model tier, pending T4/T8 — §4]:

- **Own pushes**: reply boundaries counted at the encode loop's top
  (the natural gating point — `eager-absorption.md` §7.3). Flush
  receipts are flush-paced, not consumption-paced, and stay that way:
  the standing ruling (MUX-PROGRESS §1) keeps consumption receipts
  out of the observation — the engine must never mistake "the kernel
  took my bytes" for "the peer consumed my reply".
- **Arrivals as evidence**: a decoded arrival on stream s' whose
  content is causally downstream of the peer consuming reply k on s
  certifies that consumption. Per-channel order only: audit finding
  A10 (global interleaving draws executor randomness) forbids any
  inference from cross-stream arrival order.
- **The inevitability closure**: silent consumptions — provision
  absorptions and all-M scopes produce no reverse traffic, ever — are
  derived, not observed: consumption of reply k is *inevitable* when
  everything the peer must still do before it needs no further input
  from this side. This closure is the load-bearing novelty; its probe
  implementation survived 4,970/4,970 causal runs including the
  adversarial families built to starve it (stage-0 gate P1,
  MUX-PROGRESS log 2026-07-21) [checked].

The writer's cross-stream ordering — control-frame priority, chunk
granularity, and everything else about *which* eligible frame goes
next — is §3.3's subject: within the window discipline it is provably
a latency-only concern.

The byte-budget variant (§5A's "window dial", state-11 reserved
bytes) remains the recorded alternative denomination for a deployment
whose parked-reply worst case (the ≈ 2 MB maximally disputed reply) is
unaffordable ×17·K: it trades wire bytes for a tighter RAM bound. Not
built here; the reservation and the sizing math stay in §5A. Its
advertisement would ride the same greeting field family.

### 3.3 Frame scheduling: order freedom and the priority ladder

Between the window gate (§3.2) and the wire sits one remaining
choice: which eligible frame the writer mux sends next. Within a
stream the order is fixed — the wire is positional — so the choice is
cross-stream interleaving plus chunk placement. The campaign's result
here is the strongest foundation a scheduler can be given: the choice
cannot affect correctness at all.

**Order freedom [derived].** Under the advertised-window discipline —
every sender within the peer's advertised K on every stream — **any**
work-conserving order over window-eligible frames is deadlock-free and
terminating:

- *Safety is order-free.* The window gate is exactly what guarantees
  the demux never blocks (§2 item 3: every arrival parks within a
  bound the receiver chose), and a never-blocking demux makes the
  socket composition a refinement of the independent-channel system,
  whose liveness is the kernel-proven flagships. No step of that
  argument mentions which eligible frame goes first. The theorem
  dependencies are named plainly: the elastic-parking simulation
  theorem and T8, both in flight (§4; status in
  `formal/MUX-PROGRESS.md` §5), T8 with the asymmetric-window
  (K_I ≠ K_R) parameterization §4 already flags.
- *Starvation is impossible without fairness machinery.* Every pushed
  frame fires a protocol operation and a session's total is finite
  (the ρ argument, `formal/MODEL.md` §7, being minted as a kernel
  theorem in the campaign's stage 3), so an order that neglects a
  stream can only delay it, never strand it: eventually the neglected
  stream's frames are the only eligible ones.

Consequence, stated as policy: **frame ordering is a pure latency
dial**. The scheduler carries no proof obligation — only the window
gate does (S1's soundness hooks, §5.2) — so the ladder below may be
tuned, rearranged, or replaced from measurement without
re-verification. The order cannot be gotten wrong, only slow.

**The recommended ladder [derived; rationale per `MUX-LATENCY.md`].**
The latency analysis splits the socket's costs cleanly: small
label-carrying frames sit on the δ (round-trip) critical path — each
one delayed behind bulk adds directly to the fresh-dispute pacing law
(`MUX-LATENCY.md` §2.2/§3.1), because it advances both the peer's walk
and the peer's demand-proof inference — while bulk provision bytes sit
only on the bandwidth term (§3.3 there) and pipeline unboundedly
wherever they are placed. So: strict priority classes, round-robin
within a class:

1. **Session control** — preamble/greeting, `End` controls, the
   epilogue: the §8.5 router lesson, unchanged.
2. **Frontier control** — dispute-scope replies, queries,
   resolutions: the small frames whose delay is priced in round
   trips.
3. **Active-descent data** — deepest-stream-first as the tiebreak.
   (The old mux's bottom-most-ready heuristic aimed exactly here; its
   fatal ingredient was eagerness — pushing without a window — not
   the priority. The heuristic is rehabilitated one rung down the
   ladder.)
4. **Bulk provision runs**, chunked at `RunBudget`, with
   **chunk-boundary preemption**: a pending higher-class frame may
   interleave between chunks, never mid-frame. This bounds the byte
   head-of-line term at one chunk's transmission time — the term's
   floor (§6) — and leaves reply atomicity intact: preemption sits at
   chunk boundaries *between* a run's frames, which still arrive in
   order on their own stream.

K-general pricing of the ladder — how the frontier term scales with
the advertised window — is the latency doc's K-dial addendum
[forthcoming]; this section's rationale stands on the committed base
analysis.

**What the scheduler must not be trusted with** (negative space,
recorded so a future reader does not add machinery the theorems make
unnecessary):

- It cannot repair a window-gate bug — no ordering can create safety,
  because the never-block property comes from the gate alone. And
  symmetrically, no stall or violation is ever attributable to the
  ladder: violations attribute to the gate or the peer (§3.1, §5.2);
  the scheduler is exonerated by construction.
- No fairness, aging, or anti-starvation machinery is warranted: the
  finite-ρ argument makes starvation structurally impossible.
  Priority aging here would be dead weight in front of a standing
  liveness proof.
- The §5.2 priority assertion tests the ladder's *observable class
  behavior* (frontier control never queued behind bulk by more than
  one chunk), not its optimality. Optimality is measurement's job —
  and safety-free tuning is the point.

## 4. The theorem interface

What is already kernel-proven on `mux-conjectures` [proven]:

- `wc_impossibility` (`Mux/Proofs/WcImpossibility.lean`): **every**
  work-conserving sender pair deadlocks the wedge skeleton at
  **every** capacity C ≥ 1. This is why there is an inference engine
  at all: no eager discipline, however informed, survives a fixed
  parking depth — the *right to idle* is the essential ingredient.
  Its controls pin the design's two load-bearing hypotheses from both
  sides: `wedge_idler_completes` (a withholding strategy completes
  the same skeleton — the escape hatch is real) and
  `wedge_unboundedSlot_completes` (elastic parking alone revives even
  the eager scheduler — the receive half of this design, isolated).
- `commit_totality` (T1) and the harness pins (`wedge_wellFormed`,
  `wedge_margin0`, `wedge_bottomMostReady_jams`,
  `smokeChain_mux_completes`).
- The static-oracle refutation [checked, doubly]: pushing a
  *precomputed* schedule — even one computed from both trees — jams
  (muxprobe's pinned `rand2` witness; stage-0 P2's independent
  11-scope counterexample). Adaptivity is necessary, information is
  not sufficient: the engine must consult live arrivals, which it
  does by construction.

Pending, in dependency order (status lives in `MUX-PROGRESS.md` §5):

- **T2** (the keystone/chase infrastructure): landing at time of
  writing.
- **T4** `sigmaStar_deadlock_free`: σ\* live at K = 1, every C ≥ 1 —
  probe-cleared (stage-0 P1) and unblocked. This is also the K = 1
  advertisement's liveness theorem (§3.1).
- **T8** `sigmaStarK_deadlock_free` / `wc_impossibility_K`: the
  K-deep generalization — **T8's statement is this engine's
  specification**, and per house posture (the deadlock doc §4's
  closing lesson: the last "derived, obvious" liveness argument
  shipped a deadlock) the sender engine does not merge before T8 is
  kernel-checked. Asymmetric advertised windows (K_I ≠ K_R) must be
  covered by T8's statement — flag to the campaign [open].
- **The elastic simulation theorem**: unbounded parking makes the
  muxed system a refinement of the independent-channel system, so the
  receive half alone inherits the base flagships — the cheap theorem
  that lets receiver work proceed while T8 cooks.

Sequencing consequence, stated as policy: **receiver half first** (§2
— safe under both transports, inert under Link, covered by the
simulation argument), **socket transport + engine second, gated on
T8**, **Link removal last, gated on §5**. The Rust bridges from the
campaign (wedge realizability, LocalEq, B5 announced-skeleton
reconstruction — landed on `mux-conjectures`) transfer with the suite.

## 5. Acceptance — the gates for stage L (Link removal)

### 5.1 The seeds and the sweep

The two committed wire-deadlock regressions
(`tests/pairwise.proptest-regressions`,
`tests/shadow_validity.proptest-regressions`) are the reason this
whole area exists. The socket transport must complete both **at every
K, including K = 1** (`Window::FLOOR`), and at asymmetric
advertisements (K = 1 one way, production the other), under the
deterministic quiescence harness — the exact configuration whose stall
condemned the old mux. Plus the standard sweep: full gate, capacity
floor tests, muxprobe cross-check, and a soak: extended
randomized-schedule runs at mixed window sizes before Link deletion is
irreversible-in-practice.

### 5.2 Conformance obligations, transmuted

During transition the Link conformance suite stands. At stage L it is
deleted with the contract it validates, and its load-bearing clauses
transmute into socket-transport assertions, testable without a peer:

- routing: every frame reaches the pump its signal byte names;
  `AfterEnd` detection preserved;
- never-block: the demux reader is never blocked by any pump queue —
  over-window surfaces as `Violation` (§2 item 3), and the violation
  is attributable (§3.1: the peer knew the bound);
- priority: the §3.3 ladder's observable class behavior — frontier
  control is never queued behind bulk by more than one chunk's
  transmission (chunk-boundary preemption visible in the write
  order);
- inference soundness hooks: the occupancy ledger's estimate never
  exceeds the true unconsumed count (under-estimation is a latency
  bug, over-estimation is the deadlock bug — assert the direction);
- honoring: the sender never starts a reply past the peer's advertised
  window (the sender-side twin of the receiver's violation check).

### 5.3 Snapshots

The greeting change re-accepts the gossip/bootstrap/retire snapshot
pins consciously (§3.1, the repo hard rule). Per-stream captures
(`codec/capture.rs`) remain stable off the greeting: per-stream byte
sequences are unchanged. Any whole-socket capture of the interleaving
is **not** pinnable — interleave order is scheduler- and
inference-dependent (and A10 says the cross-stream publication order
never was deterministic); pin per-stream projections only. Flagged now
so it is a decision, not a surprise.

## 6. Honest residuals

- **Bandwidth head-of-line**: one FIFO interleaves at frame
  granularity; worst added wait for an urgent frame ≈ one `RunBudget`
  chunk's transmission time (default ≈ 1.1 MB ⇒ ~9 ms at 1 Gbps,
  ~90 ms at 100 Mbps). Tunable against throughput overhead.
  Multi-stream transports did not have the term; with Link removed,
  every deployment has it. [derived]
- **Loss-recovery coupling**: one lost segment stalls all seventeen
  streams for the retransmit. On the Link, QUIC instantiations
  confined this per-stream; with Link removed the property is not
  purchasable at any configuration — the accepted price of §1.3,
  restated where it bites: it costs tail latency under loss, nothing
  on clean links. [derived]
- **Latency parity**: in round-trip counting the socket transport at
  production window matches the multi-link construction — the
  frontier law (K scopes per RTT per stream, vs 1 at the σ\* floor)
  and the full derivation land in the campaign's latency analysis
  (`MUX-LATENCY.md`, forthcoming); no quantitative claim is made here
  beyond the structure. At K = 1 the width cost is §5A's honestly
  recorded ~n·RTT per n-wide level — the floor exists for tests, not
  deployments. [derived, pending that doc]

## 7. Staged plan

Dependency-ordered; risk noted per stage. Estimates are
change-in-place sizes against this branch, informed by
`eager-absorption.md` §8 (which measured main; branch deltas noted).
Link carries production traffic until stage L.

- **R0 — spikes (small, first):** the `ProxyLocalQuestions` depth
  derivation (§2 [open]); a `cargo llvm-lines` spot-check that the
  socket module keeps the §8.9 monomorphization gate; confirm V2's
  release status for the §3.1 amend-vs-V3 fork. Risk: low.
- **R1 — receiver widening + advertisement (~250 lines + tests):**
  `queues::responses` takes the Window capacity like its siblings;
  the `Handshake` window field with snapshot re-acceptance (§3.1) —
  landed together so the greeting changes once; the context-causality
  proptest; the parked-memory accounting test (the ≈ 2 MB worst-case
  reply, asserted shared not copied). Inert under Link; lands first.
  Risk: low.
- **S0 — the theorems:** T4, then T8 (+ the simulation theorem, any
  time after T2), with T8's statement covering asymmetric windows
  (§4 [open]). Owned by the `formal/` campaign; this branch consumes
  their names in doc comments. Risk: the campaign's two named risks;
  mitigations recorded there.
- **S1 — the σ\*ₖ engine (~1–2k lines + test apparatus):** the
  occupancy ledger (per-stream counters + evidence intake + the
  inevitability closure), the encode-loop gate against the peer's
  advertised window, the direction-asserted soundness hooks (§5.2).
  Transcription-parity tests against the campaign's Python causal σ\*
  on the pinned families. Gated on T8. Risk: **the** risk; bounded by
  the theorem it must refine.
- **M1 — the socket transport (~400–700 lines):** the writer mux
  (the §3.3 ladder: priority classes + chunk-boundary preemption,
  plus the S1 gate), the reader demux (routing + validation +
  over-window violation), the poison latch relocated to the socket
  session, wired behind a transitional constructor switch beside the
  Link path. Risk: moderate — mostly careful reuse of the end/error
  discipline; the scheduler itself is safety-free (§3.3) and tunable
  after landing without re-verification.
- **V — acceptance (§5):** seeds at K ∈ {1, floor+1, production} and
  asymmetric; the transmuted conformance assertions; gate +
  `just all`; soak; docs (`remote.rs`, `window.rs` — K's second
  consumer and its advertisement; MODEL.md's scope note per
  AUDIT-NOTES A6).
- **L — Link removal (the final stage):** delete `src/link.rs`, the
  `conformance` feature and suite, `remote/streams.rs`, the
  transitional constructor switch; resolve the epoch's fate (§1.2
  [open]); public API migration notes for `Link` users (rumormill's
  iroh binding → single stream); the `streams.rs` module-doc sentence
  ("supplied by the link contract, not reconstructed here") dies with
  its module — its socket counterpart already says the opposite.
  Gated on V, including soak. Risk: low mechanically; irreversible
  externally — hence last, behind everything.
- **Deferred, recorded:** the byte-budget variant (§3.2); a
  loss-coupling measurement against a QUIC single-stream baseline (§6
  is [derived]; a number would be better); ladder tuning under
  measurement (§3.3 makes it safety-free, so it never gates a stage);
  erasure interactions per `height-erasure.md` if the socket session
  wants `dyn` seams.

## Appendix: relation to the campaign documents

- `formal/MUX-ADJUDICATION.md` — why no eager scheme works and why
  σ\* does: the verdicts this design implements.
- `design/eager-absorption.md` — the code-level feasibility this
  design turns into a plan; its §7.4 is this document's §1.4 in
  embryo.
- `streaming-wire-deadlock.md` §5A — the credit design this document
  retires as single-socket design of record; its window-dial theory
  and sizing math remain authoritative for the byte-budget variant.
  §8's contract remains authoritative for the Link while the Link
  exists (through stage V), and becomes historical at stage L.
- `formal/MUX-LATENCY.md` — the round-trip pricing §3.3's ladder
  rationale cites and §6 defers to. The base analysis (the σ\* pacing
  law, the width term, the shape table) is committed on the campaign's
  `mux-latency` branch; the K-dial addendum (how the frontier term
  scales with the advertised window) is forthcoming.
