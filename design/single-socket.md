# The single-socket transport: undoing the Link requirement with inferred windows

Status: design of record (2026-07-21, against `link-transport` at
`65092b4a`). This document supersedes `streaming-wire-deadlock.md` §5A
as the single-socket design of record — §5A's credit mux is retired,
not superseded by silence: the mux conjectures campaign
(`formal/MUX-PROGRESS.md`, `formal/MUX-ADJUDICATION.md`, on the
`mux-conjectures` branch) proved that the credit *messages* carry no
information the protocol does not already announce, and the
eager-absorption assessment (`design/eager-absorption.md`, same branch)
verified in the code that the buffer a credit's grant promises can be
denominated in *decoded logical replies* — dissolving §5A's
unit-mismatch discontinuity. What remains of a credit scheme once its
messages are inferred and its buffers are the decoded replies the
adapter already produces is: a wider queue, a counter, and a proof.
That is this design.

Epistemic key as in `streaming-wire-deadlock.md`: **[proven]** =
kernel-checked in `formal/lean` on `mux-conjectures`; **[checked]** =
verified by reading the cited code or by executable evidence;
**[derived]** = argued here from checked premises; **[open]** = needs
a spike.

The perspective, per Finch's directive: `link-transport` is the base
point. Everything that branch fixed and hardened is kept; what is
*undone* is one requirement — that the transport supply seventeen
independent, individually flow-controlled streams per direction. A
deployment that has QUIC keeps using the `Link` exactly as today. A
deployment that has one TCP socket gets a transport mode that carries
all seventeen logical streams on it, deadlock-free with **zero new
wire alphabet and zero logical-protocol changes**, by the campaign's
two results:

- **Receive side, eager conversion**: wire frames become logical
  replies at arrival (this is already the adapter's architecture —
  `remote/adapter.rs:52-60` decodes each record as it arrives through
  the fan-bounded channel into `Convert::assemble`, retaining only the
  reply skeleton and node handles). Parking up to K decoded replies
  per stream costs K·O(fan) handles, not K subtrees
  (`eager-absorption.md` §7.1 [checked]).
- **Send side, σ\*ₖ inferred flow control**: the sender starts reply
  r on stream s only while fewer than K prior replies on s are
  un-provably-consumed, the proof built from its own pushes, decoded
  arrivals, and the inevitability closure over silent consumptions —
  "W = K credits inferred instead of sent"
  (`MUX-ADJUDICATION.md` §1.3, generalized).

## 1. What is undone, what is kept

### 1.1 Kept, unchanged — the branch's improvements are the base

- **The `Link` type and its session machinery** (`src/link.rs`):
  the control stream and its phase discipline (preamble, greeting
  with root-fan listing, identity hand-off, epilogue marker byte);
  `SessionState` — the epoch counter as stream-label tripwire and the
  poison latch (`link.rs:173-214`); the epilogue/poisoning dual
  integrity mechanisms (`streaming-wire-deadlock.md` §8.10). The
  socket mode reuses all of it verbatim: a single-socket session is
  still a session on a link whose control stream is the socket.
- **The frame codec and its self-describing frames**
  (`remote/codec/`): every frame's dense signal byte already restates
  its logical stream index (`remote/streams.rs:22-27`) — the codec
  was born mux-ready, and the socket mode adds no framing.
- **The adapter** (`remote/adapter/`): the conversion boundary
  (`Reply ⇄ Frame` runs), the `Scope` FIFO as the decode-context
  ledger, run batching by `RunBudget`. Untouched.
- **The proxy** (`remote/proxy/`): the typed states, the
  publish-then-park discipline, `yield_reply_scopes!`. One queue
  widens (§2); nothing else moves.
- **The `Window` mechanism** (`window.rs`) and its public knob
  (`Peer::max_in_flight_nodes`). The socket mode *unifies* with it
  (§1.3) rather than adding a second dial.
- **The conformance posture**: the `conformance` cargo feature and
  its per-clause probes remain the validation story for caller-built
  multi-stream `Link`s; the socket mode adds its own obligations
  (§5.2) rather than diluting them.
- **Lazy establishment, label validation, double-checked stream ends,
  deferred supply failures** (§8.10): each survives with its socket
  analogue noted in §2/§3.

### 1.2 Undone — and what replaces it

Undone: the requirement that `Connector`/`Acceptor` exist — i.e. that
the transport supply independent streams with per-stream receiver-paced
flow control (`link.rs:35-45`, the Independence and Flow control
clauses). Those clauses were how the deadlock-freedom argument was
*bought from the transport*; the campaign showed they can instead be
*reconstructed at the endpoints* over one ordered byte stream per
direction.

The replacement is **not** a `Link` instantiation, and deliberately so
[derived]. Two reasons, each sufficient:

- A single-socket `Connector`/`Acceptor` satisfying the flow-control
  clause transport-side would have to carry per-stream credit traffic —
  §5A's design, which the campaign's charter rules out as superfluous
  and which `wc_impossibility` shows cannot be replaced by any
  clever *eager* scheduling (§4). The Link contract is honest: a
  transport either has real per-stream flow control or it is not a
  `Link`.
- σ\*ₖ's pacing decision is made at *reply* boundaries with *protocol*
  knowledge (which stream a reply starts on, which arrivals prove
  consumption). The `Link` boundary deliberately erases exactly that
  (anonymous streams, opaque bytes — `link.rs:57-60`). Pacing that
  needs protocol eyes belongs above the boundary the contract drew.

So the socket mode enters at the **`streams.rs` seam**, not the
`link.rs` seam: a sibling module (`remote/single.rs`, name bikeshed
welcome) implementing the same session-facing surface — per-stream
senders, per-stream receivers, the accept/claim driver — over one
`AsyncRead + AsyncWrite` pair. `streams.rs:1-8` states the layer's own
premise ("nothing multiplexes... supplied by the link contract, not
reconstructed here"); the socket mode is the module for which that
sentence is false by design, and its module doc will say so with the
same bluntness. The session chooses its transport mode at
construction; everything above the seam (proxy, adapter, protocol) is
identical in both modes.

Negative space, recorded: (a) hiding per-stream buffering *inside* a
fake `Link` instantiation to satisfy the letter of the contract is
option C of the deadlock doc — unbounded memory or a lie about
boundedness; rejected again here. (b) Carrying §5A's credit bytes
(reserved signal states 170..=203) remains a valid *future* transport
choice for a deployment that wants pacing-by-contract instead of
pacing-by-proof; the reservation stays. This design needs none of
those bytes — the V2 wire format is byte-for-byte unchanged.

### 1.3 The Window relation: one knob, both mechanisms [derived]

`window.rs` already denominates the session's pipelining in node
references and widens the two proxy scope edges
(`proxy/work/queues.rs:32-46`). On a multi-stream `Link`, the third
edge — `ProxyResponses`, the decoded-reply relay — stays at one slot
*because the transport's flow control holds the rest of the in-flight
replies on the wire* (`queues.rs:11-14`: "its single slot is what
bounds decoded replies in flight per stage").

The socket mode relocates exactly that bound: the wire can no longer
hold per-stream overrun (one FIFO, shared), so the decoded-reply queue
becomes the per-stream window buffer. K — the per-stream parked-reply
depth — derives from the same `Window`:

    K = Window::scopes()          (per stream, in decoded replies)

with `Window::FLOOR` giving K = 1, the campaign's base case. One knob
(`Peer::max_in_flight_nodes`), one derivation, two consumers: channel
capacities (as today) and the σ\*ₖ send gate (new). Test builds keep
the floor (`window.rs:100-114`), so every session shape stays
exercised at the K = 1 corner where a bad ordering *would* deadlock —
the same discipline the branch already applies.

Memory accounting for the widened edge [checked, eager-absorption §7.1]:
a parked decoded reply is a `Vec<Reaction>` of ≤ fan entries — a
`Supply` is one node handle (shared structure, not a copy), a
`Query` listing ≤ fan hashes. Worst case ≈ fan² hashes ≈ 2 MB for a
maximally disputed reply; the provision case this design exists for is
O(fan) handles, the subtree's bytes having already streamed into
backend custody through `Convert::assemble`. Contrast §5A's byte
windows (17·(W + max frame) of *raw buffer*): the grant unit and the
buffer unit finally match, which is the whole reason reply-denominated
K > 1 is sound here and was not in §5A. The backend custody itself is
storage the transfer was going to consume anyway; abort reclamation is
handle-drop (`eager-absorption.md` §6 [checked for Local; documented
obligation for persistent backends]).

## 2. The receive side: eager conversion with K-deep parking

What exists [checked]: the demux-fed pump coroutines already decode
every frame at arrival and drive `Backend::parent` folds pre-cursor;
the `Scope` FIFO already registers, at question-emission time, exactly
the context decoding needs (prefix, listed radices, height); decoding
is batching-agnostic and never accumulates a subtree
(`remote/adapter.rs:39-60`).

What changes:

1. **`ProxyResponses` widens from 1 to K** (`queues.rs:22-24` takes a
   capacity argument like its two siblings). Under a multi-stream
   `Link` this is inert-but-harmless extra parking (the transport
   still paces); under the socket mode it is the window buffer.
2. **The socket demux**: one reader per direction, owning the read
   half; routes each frame by its signal byte's stream index to the
   pump input; validates epoch/labels with the same rules the
   `AcceptDriver` applies today (first-frame label → per-frame
   restatement, `StreamError::Mislabeled` on disagreement).
3. **Over-K arrival is a protocol violation, not backpressure**
   [derived]. A conformant σ\*ₖ peer never exceeds K, so a frame that
   would park the (K+1)-th reply on one stream surfaces as a
   `Violation` through the session's one-slot error route — the same
   publish-then-park discipline `streams.rs:29-34` uses. This converts
   any inference bug, on either side, from a silent wedge into a loud,
   attributable failure: the single reader never blocks on a full pump
   queue, so the §2 six-link cycle of the deadlock doc is
   *unconstructible* rather than merely avoided.
4. **Context-registration causality** gets its named proptest: every
   arriving frame finds its decode context already registered by a
   prior local emission. The property was verified arm-by-arm across
   the message vocabulary (handshake, opening, replies, nested scopes,
   stream end) in `eager-absorption.md` §3.3 [checked]; the proptest
   pins it against drift. It is the receive-side mirror of the
   announcement-completeness that makes σ\* local [proven-adjacent,
   MUX-ADJUDICATION §1.2].

One [open] carried from the assessment: the `ProxyLocalQuestions`
occupancy bound (questions in flight per stream) is not K-bounded from
below by anything structural; entries are tiny, but the spike deriving
its true bound from the walk's channel capacities is unfinished
(`eager-absorption.md` §7.2). It gates nothing in this design — the
edge is already Window-wide on the branch — but the derivation belongs
in `window.rs`'s docs when it lands.

## 3. The send side: the σ\*ₖ inference engine

The engine gates one thing: **starting** a new reply on a stream.
Frames of a started reply flow freely (reply-atomicity: pumps never
park mid-reply — the same structural fact §5A's W = 1 leaned on);
`End` controls are free. The gate: stream s admits a new reply while
strictly fewer than K prior replies on s are un-provably-consumed.

"Provably consumed" is built from three sources, all local
[proven for K = 1 at the model tier, pending T4/T8 — §4]:

- **Own pushes**: reply boundaries counted at the encode loop's top
  (the natural gating point — `eager-absorption.md` §7.3). Flush
  receipts are flush-paced, not consumption-paced, and stay that way:
  the standing ruling (MUX-PROGRESS §1) keeps consumption receipts out
  of the observation — the engine must never mistake "the kernel took
  my bytes" for "the peer consumed my reply".
- **Arrivals as evidence**: a decoded arrival on stream s' whose
  content is causally downstream of the peer consuming reply k on s
  certifies that consumption. Per-channel order only: audit finding
  A10 (global interleaving draws executor randomness) forbids any
  inference from cross-stream arrival order.
- **The inevitability closure**: silent consumptions — provision
  absorptions and all-M scopes produce no reverse traffic, ever — are
  derived, not observed: consumption of reply k is *inevitable* when
  everything the peer must still do before it needs no further input
  from this side. This closure is the load-bearing novelty; its
  probe implementation survived 4,970/4,970 causal runs including the
  adversarial families built to starve it (stage-0 gate P1,
  MUX-PROGRESS log 2026-07-21) [checked].

Two engine-adjacent disciplines ride along:

- **Control-frame priority**: the socket mux drains session-control
  traffic (stream ends, the epilogue) ahead of data frames — the §8.5
  router lesson applied to our own mux.
- **Chunk granularity**: the bandwidth head-of-line term (an urgent
  frame waiting behind a bulk chunk in transit) is bounded by
  `RunBudget` and tunable; §6 prices it. No priority *within* data
  frames is needed for liveness — only for the tail of that term.

The byte-budget variant (§5A's "window dial", state-11 reserved
bytes) remains the recorded alternative denomination for a deployment
whose parked-reply worst case (the ≈ 2 MB maximally disputed reply)
is unaffordable ×17·K: it trades wire bytes for a tighter RAM bound.
Not built here; the reservation and the sizing math stay in §5A.

## 4. The theorem interface

What is already kernel-proven on `mux-conjectures` [proven]:

- `wc_impossibility` (`Mux/Proofs/WcImpossibility.lean`): **every**
  work-conserving sender pair deadlocks the wedge skeleton at **every**
  capacity C ≥ 1. This is why there is an inference engine at all: no
  eager discipline, however informed, survives a fixed parking depth —
  the *right to idle* is the essential ingredient. Its controls pin
  the design's two load-bearing hypotheses from both sides:
  `wedge_idler_completes` (a withholding strategy completes the same
  skeleton — the escape hatch is real) and
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
  probe-cleared (stage-0 P1) and unblocked.
- **T8** `sigmaStarK_deadlock_free` / `wc_impossibility_K`: the K-deep
  generalization — **T8's statement is this engine's specification**,
  and per house posture (the deadlock doc §4's closing lesson: the
  last "derived, obvious" liveness argument shipped a deadlock) the
  sender engine does not merge before T8 is kernel-checked.
- **The elastic simulation theorem**: unbounded parking makes the
  muxed system a refinement of the independent-channel system, so the
  receive half alone inherits the base flagships — the cheap theorem
  that lets receiver work proceed while T8 cooks.

Sequencing consequence, stated as policy: **receiver half first**
(§2 — safe under both transport modes, inert under `Link`, covered by
the simulation argument), **socket mode + engine second, gated on T8**.
The Rust bridges from the campaign (wedge realizability, LocalEq,
B5 announced-skeleton reconstruction — landed on `mux-conjectures`)
transfer to this branch with the suite.

## 5. Acceptance

### 5.1 The seeds

The two committed wire-deadlock regressions
(`tests/pairwise.proptest-regressions`,
`tests/shadow_validity.proptest-regressions`) are the reason this
whole area exists. Acceptance for the socket mode: both complete **at
every K, including K = 1** (`Window::FLOOR`), under the deterministic
quiescence harness — the exact configuration whose stall condemned
the old mux. Plus the standard sweep: full gate, capacity floor tests,
and the muxprobe cross-check (the Lean executable tier's golden
matrix already exercises the model twin of this transport).

### 5.2 Conformance

The `Link` conformance suite is untouched — it validates multi-stream
instantiations against the contract this mode deliberately does not
claim. The socket mode's own obligations, testable without a peer:

- routing: every frame reaches the pump its signal byte names; label
  and epoch validation preserved; `AfterEnd` detection preserved;
- never-block: the demux reader is never blocked by any pump queue —
  over-K surfaces as `Violation` (§2.3);
- priority: control traffic is not queued behind data;
- inference soundness hooks: the occupancy ledger's estimate never
  exceeds true unconsumed count (under-estimation is a latency bug,
  over-estimation is the deadlock bug — assert the direction).

### 5.3 Snapshot churn [open]

Per-stream captures (`codec/capture.rs`) remain stable: per-stream
byte sequences are unchanged. Any whole-wire snapshot of the socket's
interleaving is **not** pinnable — the interleave order is scheduler-
and inference-dependent (and A10 says even the logical publication
order across streams never was deterministic). If a whole-socket
capture exists by then, re-accept it as deliberately unpinned or pin
per-stream projections only. Flagged now so it is a decision, not a
surprise.

## 6. Honest residuals

- **Bandwidth head-of-line**: one FIFO interleaves at frame
  granularity; worst added wait for an urgent frame ≈ one
  `RunBudget` chunk's transmission time (default ≈ 1.1 MB ⇒ ~9 ms at
  1 Gbps, ~90 ms at 100 Mbps). Tunable against throughput overhead;
  multi-stream transports do not have the term at all. [derived]
- **Loss-recovery coupling**: one TCP segment loss stalls all
  seventeen streams for the retransmit; independent connections or
  QUIC streams confine it. Irreducible on one ordered byte stream —
  this is the one thing the `Link` keeps that no endpoint cleverness
  recovers, and the honest answer to "why keep the Link at all"
  alongside kernel-managed flow control and boringness. [derived]
- **Latency parity**: in round-trip counting the socket mode with
  K = production Window matches the multi-link construction — the
  frontier law (K scopes per RTT per stream, vs 1 at the σ\* floor)
  and the full derivation land in the campaign's latency analysis
  (`MUX-LATENCY.md`, forthcoming); no quantitative claim is made here
  beyond the structure. At K = 1 the width cost is §5A's honestly
  recorded ~n·RTT per n-wide level — the floor exists for tests, not
  deployments. [derived, pending that doc]

## 7. Staged plan

Dependency-ordered; risk noted per stage. Estimates are
change-in-place sizes against this branch, informed by
`eager-absorption.md` §8 (which measured main; the branch deltas are
noted).

- **R0 — spikes (small, first):** the `ProxyLocalQuestions` depth
  derivation (§2 [open]); a `cargo llvm-lines` spot-check that the
  socket module keeps the §8.9 monomorphization gate. Risk: low.
- **R1 — receiver widening (~150 lines + tests):** `queues::responses`
  takes the Window capacity like its siblings; the context-causality
  proptest; the parked-memory accounting test (the ≈ 2 MB worst-case
  reply, asserted not copied). Inert under `Link`; lands first. Risk:
  low. (Smaller than the assessment's 360 — the branch already did the
  Window plumbing the main-based estimate included.)
- **S0 — the theorems:** T4, then T8 (+ the simulation theorem, which
  can land any time after T2). Owned by the `formal/` campaign; this
  branch consumes their names in doc comments. Risk: the campaign's
  two named risks; mitigations recorded there.
- **S1 — the σ\*ₖ engine (~1–2k lines + test apparatus):** the
  occupancy ledger (per-stream counters + evidence intake + the
  inevitability closure), the encode-loop gate, the direction-asserted
  soundness hooks (§5.2). Transcription-parity tests against the
  campaign's Python causal σ\* on the pinned families. Gated on T8.
  Risk: **the** risk; bounded by the theorem it must refine.
- **M1 — the socket mode (~400–700 lines):** `remote/single.rs`
  implementing the `streams.rs` surface over one byte stream per
  direction: the writer mux (priority + chunking + the S1 gate), the
  reader demux (routing + validation + over-K violation), session
  wiring behind a constructor switch, `SessionState` reuse. Risk:
  moderate — mostly the careful reuse of label/end/error discipline.
- **V — acceptance (§5):** the seeds at K ∈ {1, floor+1, production};
  conformance additions; gate + `just all`; the snapshot decision
  (§5.3); docs — `remote.rs`, `streams.rs` ("supplied by the link
  contract" gains its counterpart sentence), `window.rs` (K's second
  consumer), MODEL.md's scope note per AUDIT-NOTES A6.
- **Deferred, recorded:** the byte-budget variant (§3); a QUIC
  binding comparison benchmark (the loss-coupling residual, §6, is
  measurable only there); erasing the mode switch behind the eventual
  `dyn`-erased session if `height-erasure.md`'s project wants it.

## Appendix: relation to the campaign documents

- `formal/MUX-ADJUDICATION.md` — why no eager scheme works and why
  σ\* does: the verdicts this design implements.
- `design/eager-absorption.md` — the code-level feasibility this
  design turns into a plan; its §7.4 is this document's §1.3 in
  embryo.
- `streaming-wire-deadlock.md` §5A — the credit design this document
  retires as primary single-socket design of record; its window-dial
  theory and sizing math remain authoritative for the byte-budget
  variant; §8's contract remains authoritative for multi-stream
  `Link`s.
- `MUX-LATENCY.md` (forthcoming) — the round-trip pricing §6 defers
  to.
