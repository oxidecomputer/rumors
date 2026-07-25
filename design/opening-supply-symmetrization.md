# Opening-supply symmetrization: whole-subtree batching for the initiator's exclusive content

**Status (2026-07-23): implemented — see Amendments.** Approved for
implementation by design review after the link-transport review campaign
integrates; this document is the specification a fresh implementation
context starts from. Every file:line below was verified at commit
`22b61c02`; re-anchor before editing (§9).

This document is pedagogical by intent. It teaches the opening exchange
from scratch, demonstrates the hazard that disqualifies the obvious
design, and then specifies the change precisely enough to implement
red-green without re-deriving anything — because the next person to touch
this may be starting cold.

---

## 1. Motivation

### 1.1 The observed cost

The V2 streaming mirror batches supply content into byte-budgeted runs
(`src/tree/mirror/streaming/remote/adapter/encode.rs:187-232`): one
`Supply(radix, node)` reaction enumerates all the node's leaves into runs
that flush at the negotiated `target_message_size`, so a whole supplied
subtree crosses in ~`ceil(bytes/target)` frames. That machinery is
role-agnostic. What is not role-agnostic is the *granularity* handed to
it: one side of every session ships its exclusive top-level content whole,
and the other ships it decomposed one level — into near-singleton
per-child supplies for hash-uniform corpora below ~65k messages,
defeating the batching exactly where it matters most.

Measured (review-campaign probe, 2026-07-23, seeded corpora; recorded
here as the motivating evidence — these are probe figures, not committed
benchmarks):

- Symmetric 512/512 divergence: ~42 multi-record batching events in the
  whole-subtree direction vs ~4 (two-byte-prefix sibling accidents) in
  the decomposed direction.
- One-sided 768-key transfer on the decomposed path: 764 frames where
  whole-child supplies would take ~221 (≈3.5×).
- 2048-vs-1: ~2047 frames vs ~256 (≈8×), ~9 KB extra wire on ~50 KB of
  payload, plus the per-frame encode/flush/decode/waker cycle ×8 — the
  cost lever A exists to amortize.
- The decomposed content also departs a full round-trip later (§2.4).

### 1.2 Who pays, and why "who" is the real problem

The payer is the session's **initiator** — and the initiator is not
chosen by workload, intent, or who called `gossip()`. Roles are elected
after greetings cross, by **lexicographic comparison of the canonical
version encodings** (`local_speaker`,
`src/tree/mirror/streaming/remote/proxy/start.rs:297-304`: the side whose
version bytes compare greater becomes initiator). Lexicographic order
over ITC encodings is structurally arbitrary — it does not correlate with
set size, causal dominance, or bulk. Two consequences, both confirmed
from committed wire snapshots:

- **Retire-into-bootstrapper rides the decomposed path today.**
  `tests/snapshots/retire_snapshot__retire_into_bootstrapper.snap`: the
  retiree (version bytes `[0xd3…]`) beats the empty successor (`[0x40]`,
  the empty version's encoding) and initiates; its entire donation
  crosses as `Initiator stream 1 (height 30)` supplies — the root
  children's *children*, one level decomposed. The bulk-holder-initiates
  case is not hypothetical; it is pinned in the repo.
- **Bootstrap-proper lands on the good path only by byte-luck.**
  `tests/snapshots/bootstrap_snapshot__populated_provider.snap`: the
  newcomer's empty version `[0x40]` happens to beat the provider's
  `[0x10]`, so the newcomer initiates and the provider's supplies ride
  `Responder stream 0 (height 31)` whole. A provider whose version
  encoding leads above `0x40` would win the election instead, flipping
  bootstrap onto the decomposed path. Nothing pins that case; the
  fixture's good behavior is an encoding accident, not a design
  guarantee.

Mature-corpus symmetric gossip is barely affected (exclusive top-level
children are rare once radices fill in), which is why every existing
benchmark was blind to this; the recorded "lever A+B measured null on
symmetric divergence" result is explained rather than contradicted.

### 1.3 Why now

Everything here is pre-release (decision of record from the link-transport
review triage: until first deployment, a wire dialect may change in place
with a deliberate snapshot re-accept; afterward any wire change costs a
protocol version and a compatibility gate). This is the last cheap moment
to remove the parity instead of documenting it.

---

## 2. How the opening works today

Read this section with the code open; it is the ground truth the design
modifies. Heights: leaves are height 0 (`Z`), the root is height 32,
so the root's children sit at height 31 (`UnderRoot::HEIGHT`,
`src/tree/mirror/streaming/remote/codec/signal.rs:9`).

### 2.1 Greetings: both listings cross, one is consumed

Each side's greeting carries its causal version, sizes, and its **root-fan
listing** — `(radix, hash)` pairs for its root children
(`src/tree/mirror/streaming/message.rs:51-80`). The listing is exactly the
opening question's content, carried early so the elected responder can
answer without waiting a hop (`message.rs:33-39`). Both sides send one
because neither knows who will win the election; the responder consumes
the initiator's, and **the initiator never consumes the responder's** —
the greeting's own docs call it "deliberately dead weight"
(`message.rs:41-49`). That deliberateness weighed only the byte cost of
carrying the listing; the supply-granularity consequence below was never
part of the recorded trade.

### 2.2 The reply grammar, in one paragraph

After the handshake, every stream message is a `Reply` — the complete
answer to one earlier question, paired positionally: the k-th reply on a
stream answers the k-th question its receiver asked (`message.rs:1-17`).
A reply's `Reaction`s answer the queried node child-by-child, in radix
order: `Match` (hashes agree), `Query(listing)` (descend: here are my
children of this child, react to each), or `Supply(radix, node)` (you
don't have this child at all; here it is, whole). `Match`/`Query` consume
listing positions; `Supply` carries its radix explicitly because the
counterparty could not have known to ask (`message.rs:88-115`). An
**empty** `Query` listing means "I lack this node entirely — send
everything" (`message.rs:110-114`).

### 2.3 The merge-join, and where the decomposition comes from

Answering a nonempty query is a merge-join of the two child listings
(`src/tree/mirror/streaming/materialized/work/answer.rs:47-83`):

- **Both, equal hash** → `Match`; resolved locally.
- **Both, different hash** → `Query(my listing of that child)`; the
  answering side descends — it has just asked a question it owns.
- **Left** (mine only) → `Supply(radix, survivor)` — the whole child
  subtree, pruned first by the deletion-honoring filter `unknown()`
  against the *counterparty's version* (answer.rs:67-73). One reaction,
  one scope, maximal run batching.
- **Right** (theirs only) → `Query(Vec::new())` — the empty listing:
  "send it all" (answer.rs:74-81).

The Right arm is where the penalty is born. The empty query is answered
one level down (`materialized/work/levels.rs:195-208` for internal
levels, `:285-297` at leaf parents): `unknown_providing` prunes the
node, then ships **each of its children as its own `Supply` reaction**.
It cannot do otherwise — a reply speaks the children of the queried node;
the grammar has no "here is the queried node itself" answer. Each child
is its own reaction, each reaction its own run boundary ("a run never
spans reactions", encode.rs:192-195): the subtree crosses one level
decomposed.

Below the root this is symmetric: answer-turns alternate between the
sides level by level (the stride-2 speaker schedule, signal.rs:14-21 —
initiator owns odd-distance heights, responder even; both share stream 0
at height 31 and the leaf stream), so each side's exclusive content
discovered mid-descent ships whole from its own answer-turns, and each
side eats the Right-arm decomposition for content the *other* side owns
at *its* turns. Statistical parity.

### 2.4 The root is the one asymmetric turn

The initiator's root-level act is a **pure question**
(`Work::initiator_level`, levels.rs:45-83): it yields one reply containing
only `Reaction::Query(fan_listing(&fan))` — and even that never crosses as
a frame, because its content is the listing the greeting already carried.
The initiator-side proxy consumes the local opening without writing
anything and never opens its stream 0
(`remote/proxy/work/encode.rs:93-99`); the responder-side proxy replays
the opening from the greeting listing and never claims that stream
(`remote/proxy/work/pump.rs:42-59`).

The responder's root-level act is a full merge-join answer
(`responder_level`, levels.rs:112-138: `answer::internal(ours = my fan,
theirs = initiator's listing)`). So:

- Responder-exclusive root children → Left arm → **whole-subtree
  supplies** on responder stream 0 at height 31.
- Initiator-exclusive root children → Right arm → empty queries back to
  the initiator → answered at the initiator's height-30 turn,
  **decomposed** (its stream 1 supplies at height 30) — and departing at
  t≈1.0 RTT-halves ×2 later than the greeting instead of alongside it
  (greeting lands at 0.5; the responder's empty queries land at 1.0; the
  decomposed supplies land at 1.5 — versus 0.5 for content that rides the
  responder's immediate answer).

Both committed snapshots in §1.2 exhibit exactly this shape, one per
role assignment.

---

## 3. The cascade hazard: why the obvious fix is wrong

The obvious fix: the initiator's listing is answered by the responder, so
let the initiator answer the responder's listing the same way — run the
full merge-join on both sides of the opening.

That design double-drives the descent. The merge-join's **Both-divergent
arm asks questions** (§2.3): if both sides run it at the root, both sides
ask about every divergent root child. Each divergent child now has two
question-owners; each side answers the other's copy at the next level,
spawning two interleaved descents over the same subtrees — duplicated
discovery, duplicated supplies of the same disputed content, and a
protocol whose message complexity doubles precisely on the divergent
paths the descent exists to walk efficiently. The property the current
protocol maintains — and the litmus test any fix must pass — is:

> **Every scope has exactly one question-owner.** For any node the
> descent visits, exactly one side ever asks a question about it.

The full symmetrization fails the litmus at every divergent root child.
Rejected.

---

## 4. The design: a supply-only root answer

### 4.1 The insight

Only the merge-join's **Left arm** is starved at the root — Match and
Query arms lose nothing to the parity (divergent children descend fine;
matched children cost nothing). And the Left arm asks no questions: a
`Supply` is terminal. Emitting *only* Left-arm reactions from the
initiator's opening adds no question-owner anywhere, so the litmus
property holds trivially — every question in the session is still asked
by exactly the side that asks it today.

Equally important: the initiator can also *predict* the responder's
Right-arm empty queries exactly — they are the radices in the initiator's
listing absent from the responder's listing, both of which it holds at
greeting time. Supply-only opening is therefore not a new conversation;
it is the initiator answering, one hop early and one level higher,
questions it can prove the responder is about to ask.

### 4.2 Wire shape

- **The greeting does not change.** No new fields; both listings already
  cross (`message.rs:77-79`). The change *consumes* the listing that is
  dead weight today, which re-denominates `message.rs:41-49`'s trade
  note.
- **The initiator's stream 0 — never opened today — carries the early
  supplies.** The stream slot exists in the schedule (shared FIRST slot,
  height 31, signal.rs:34-36); lazy establishment means opening it only
  when there are supplies to send costs nothing otherwise. Frame shape:
  ordinary `Supply` reactions in reply framing, radix-keyed, run-batched
  by the existing encoder — for the wire codec this is indistinguishable
  from the responder's stream-0 supplies today, which is the point.
- **The responder still emits its Right-arm empty queries unchanged**,
  preserving reply/question pairing on every stream. What changes is the
  *answers*: for a root-level empty query whose radix the initiator
  early-supplied (recomputable on both sides: radix ∈ initiator listing ∧
  ∉ responder listing — no state to thread), the initiator answers with
  an **empty reply** (zero reactions, a ~5-byte frame) instead of the
  decomposed `unknown_providing` supplies. Pairing intact; content
  relocated.
- **The responder resolves those radices from the early supplies** rather
  than from the (now-empty) height-30 replies: `responder_level`'s
  Right-arm `Resolve::Pending` slots (via answer.rs:80) are fulfilled by
  the stream-0 arrivals. Streams are unordered relative to each other
  (the Link contract), so the bookkeeping must accept either arrival
  order; determinism of *which* radices resolve this way (the set is
  computable from the two listings) means there is no ambiguity, only
  plumbing — this is the "dual opening scope" in the implementation
  notes.

### 4.3 What it buys, precisely

- Initiator-exclusive root children ship whole: ≈`count`× fewer frames
  at small fans (the §1.1 numbers), the batching now symmetric at every
  level.
- That content departs with the greeting response window (t≈0.5) instead
  of t≈1.5 — one full RTT earlier — and the root-fan empty-query round
  becomes vestigial (empty replies) rather than content-bearing.
- Role-luck stops mattering for bulk transfer: whichever side wins the
  election, exclusive top-level content ships whole from both.

### 4.4 A cheaper sibling lever, discovered during verification — decide before implementing

Because roles are elected by arbitrary byte order (§1.2), there is a
**routing** fix available that captures most of the win with far less
machinery: **bias the election so the smaller set initiates** — compare
`(set_len, version bytes)` instead of version bytes alone; `set_len`
already rides the greeting (`message.rs:58`), both sides compute
identically, and the equal-versions short-circuit
(start.rs:290-292) is untouched. Then the bulk side is always the
responder, whose exclusives already ship whole; bootstrap and
retire-into-bootstrapper land the good path *deterministically* instead
of by luck. Costs: every role-sensitive fixture re-pins (roles flip in
many snapshots), tests that rig election by ceiling inflation
(`streaming/tests/local_eq.rs:326`, `tests/wedge.rs:120`) re-rig by size,
and the residual parity remains for the (smaller) initiator's exclusives
— bounded by the smaller set, and zero in the transfer-shaped sessions
that motivated this work.

The two levers compose: election bias routes bulk to the good path;
supply-only opening removes the bad path. Honest recommendation:
implement the election bias **first** (small, wholly behavioral,
immediately fixes the confirmed retire case and de-lucks bootstrap), then
the supply-only opening as specified here (removes the residual and the
half-RTT for whichever exclusives remain initiator-side). If only one is
taken, take the supply-only opening — it fixes the mechanism rather than
routing around it — but take it knowing the bias was on the table. This
choice was not part of the reviewed decision and needs a ruling.

---

## 5. Proof obligations

Each obligation names its verification artifact; the implementation is
not done until all five exist.

**(a) Deletion-honoring on early supplies.** The Left arm's supplies are
pruned by `unknown(backend, their_version, …)` at the supplier
(answer.rs:67-73; filter semantics `materialized/unknown.rs:38-50`: a
subtree causally at-or-before the counterparty's version drops out, which
is how redactions propagate without tombstones). The filter's inputs are
the node and *their version* — nothing about solicitation — and the
initiator holds the responder's version from the greeting, so the early
supplies are filtered identically to any solicited supply. Obligation:
a red-green test in the redaction family — corpus where the responder
redacted content the initiator still holds at an exclusive root radix;
pre-change the content must not resurrect (via the decomposed path's
identical filter at levels.rs:196/:286), post-change it must not
resurrect through the early-supply path either, and the early-supply
frames must show the *survivor*, not the full subtree.

**(b) Single question-owner.** An explicit test that a divergent root
child still produces exactly one `Query` about it in the whole session
(count Query frames per scope in a capture, divergent-fixture corpus).

**(c) Clean-drain unchanged.** The session-boundary gate
(control streams rest empty; the drain asserts in the capture harness)
must hold with stream 0 now frame-bearing in the initiator direction —
in particular End(Stream) discipline on a stream that previously never
opened. The existing liveness matrix (`tests/handshake_liveness.rs`, all
cells at a one-byte window) re-run as-is is the gate; add a
bulk-initiator cell if none of the existing shapes exercises early
supplies over MIN_CAPACITY.

**(d) The Lean model's premises.** The formal statements
(`DeadlockFree` at both `AxMode`s) are proved over MODEL.md's
abstraction: bounded SPSC channels, the transport below the model; the
channel *capacities* and the reader/assembler topology are the premises
(the FAN=256 record floor among them). This change adds frames to an
existing stream slot and moves content between replies; it does not
change any channel capacity, add a channel, or alter the
question/reply pairing discipline the walks' liveness rests on. State
this in the implementation PR by enumerating the premises and showing
each untouched — do not hand-wave "transport-agnostic"; the specific
claim is that the model never distinguished which streams carry frames,
only the channel structure, which is unchanged.

**(e) Window/budget accounting.** Supplies stream outside the window
(`src/tree/mirror/streaming/window.rs:41`; the encoder holds one run per
stream against `target_message_size`, priced in the budget's supply
terms). Early supplies are ordinary supplies on one more stream — the
per-stream encode-side accounting covers them because stream 0 was
already in `STREAM_COUNT`'s census; verify no accounting assumed the
initiator's stream 0 is frameless (grep the budget derivations for
stream-count terms; the decode-side FAN-channel accounting is per-reply
and indifferent).

---

## 6. Implementation instructions

Work red-green; the wire re-pins are deliberate commits with the
protocol change, per the snapshot rule.

**Red first.**
1. `tests/gossip_snapshot.rs`: add a bulk-initiator fixture —
   transfer-shaped corpus where the bulk side wins the election (rig via
   version bytes as `retire_snapshot__retire_into_bootstrapper` does
   naturally, or reuse that fixture family). Pin the frame count and
   shape you *want*: whole-child supplies on `Initiator stream 0
   (height 31)`, empty replies at height 30. This fails today (supplies
   appear decomposed at height 30). Corpus recipe for a binding count:
   ≥2048 messages against ≤1, per the probe arithmetic in §1.1 (whole
   1-byte-prefix subtrees ≈ 8 leaves ship as single runs; decomposed,
   they ship per 2-byte-prefix near-singletons).

**The change.**
2. `materialized.rs` handshaking: route the *remote* listing into the
   initiator's state (today only the responder path consumes it —
   `Connecting`/`Accepting` retain the local fan, materialized.rs:254-274;
   the remote listing reaches only `responder_level`). The initiator
   additionally needs `their_version` for the filter — already held
   (`their_version` threads every level).
3. `Work::initiator_level` (levels.rs:45-83): accept
   `theirs: Vec<(u8, Hash)>`; compute the Left-arm-only merge over
   `(fan, theirs)`: for each local child absent from `theirs`,
   `unknown(backend, their_version, prefix, node)` → survivor →
   `Reaction::Supply(radix, survivor)`. Yield these as reply content
   *after* the opening query reply on the same responses stream (the
   grammar's Supplies-carry-radices rule means they may share the opening
   reply or follow as its continuation — pick whichever the adapter's
   reply framing makes natural, and let the red snapshot pin it).
   Resolution bookkeeping: these radices resolve `Ready(survivor)`
   locally, mirroring the Left arm's `resolved` entries
   (answer.rs:70-72), so the initiator's root assembly no longer waits on
   the height-30 answers for them.
4. Initiator-side proxy (`proxy/work/encode.rs:93-111` `opening`): the
   opening still writes no frame for the query, but must now open stream
   0 and write the supply frames when present (lazy: no supplies, no
   stream). The `opening_scope` pairing gains the supply reactions.
5. Responder-side proxy (`proxy/work/pump.rs:42-67` `opening_reply`):
   claim the initiator-direction stream 0 when the listings imply early
   supplies (computable from the two listings both proxies hold), decode
   its supplies, and route them into the root scope — the "dual opening
   scope": the synthesized opening reply (from the greeting) plus the
   streamed supplies both feed the root resolution. Arrival-order
   independence per §4.2.
6. `responder_level` (levels.rs:112-138): unchanged merge-join, but its
   Right-arm `Pending` slots for early-supplied radices are fulfilled
   from the dual scope (step 5), not from the height-30 replies.
7. Initiator's height-30 walk (`internal_level`'s empty-listing arm,
   levels.rs:195-208): for a root-level empty query whose radix ∉
   responder listing (recompute; no plumbing), answer
   `Reply { replies: vec![] }` instead of `unknown_providing`. Deeper
   levels keep today's behavior untouched.
8. V1 (`alternating/`) is untouched; V1 snapshots must stay
   byte-identical — that is a review gate.

**Deliberate re-pins.**
9. Re-accept the V2 snapshot families whose fixtures have
   initiator-exclusive root children: `gossip_snapshot__*` (one-sided,
   deep-trie, redaction families), `retire_snapshot__*` (the
   into-bootstrapper fixture should now show height-31 whole supplies —
   that diff *is* the win, byte-visible), `bootstrap_snapshot__*` only if
   the election lever (§4.4) also lands. Verify each diff shows only the
   expected relocation (supplies moving up one height, empty replies
   appearing) before accepting.
10. `tests/hop_trace.rs`: the pinned counts (7 insertion-shaped, 7
    redaction-shaped, 3 converged — :484/:497/:515) re-derive; converged
    must stay 3 (no exclusives, no early supplies). Derive the new counts
    from the trace model before running, then confirm — do not accept
    numbers you did not predict.
11. `tests/target_message_size.rs`: the binding-minimum test's
    role-attribution comment describes whole-subtree batching as the
    responder direction's; after this change both directions batch whole
    — update the comment and, if the mixed-cell margins shift (they are
    derived from decomposition arithmetic), re-derive the constants. Ride
    the residual correction from the review campaign: the direction
    labels in that file were capture-direction, not protocol-role.

**Ordering note.** If the §4.4 election bias is ruled in, land it as its
own reviewed commit *before* this change: it flips roles in several
fixtures, and stacking both re-pins in one commit makes the diffs
unreviewable.

---

## 7. Rejected alternatives (decision records)

- **Full merge-join symmetrization** — rejected: double question-owners
  on every divergent root child; cascade (§3).
- **Sequenced absorption** (responder waits for early supplies, absorbs,
  answers with `Match` for covered radices — no dual scope, no empty
  replies): rejected: serializes the responder's entire descent behind
  the initiator's bulk supplies; in mixed workloads that un-pipelines the
  session for exactly the transfers being optimized. The pending-slot
  design (§4.2) keeps both flows concurrent.
- **Documenting the parity instead of fixing it** — rejected by the
  design ruling (pre-release economics; the confirmed retire-case
  snapshot made "document a permanent 8× donation penalty" untenable).
- **Grammar extension** (a "whole-node answer" reaction so empty queries
  can be answered undecomposed): considered during drafting; rejected as
  strictly dominated — it adds a wire variant and still pays the extra
  round-trip, where the supply-only opening needs no new grammar
  (`Supply` already carries radices for exactly the
  "couldn't-have-asked" case, message.rs:88-93) and saves the trip.

## 8. Non-goals

- No change to the greeting, the election short-circuit on equal
  versions, V1, the descent below the root, the run batcher, or any
  channel capacity.
- No attempt to batch the *interior* Right-arm decompositions
  (mid-descent empty queries keep today's one-level decomposition; they
  are fan-bounded and parity-symmetric there — §2.3).

## 9. Resumption context for a fresh agent

You are starting after the link-transport review campaign's fleet
integrated onto `link-transport`. The files this design cites were
verified at `22b61c02`, but the campaign's branches touched
`message.rs` (greeting summary prose), `proxy/start.rs` (greeting-layout
constants), `streams.rs`/codec (constants, docs), `window.rs`/backend
pricing (async `Leaf::leaf`, budget terms), and the test harnesses —
expect line numbers to have moved and one semantic interaction to
re-verify: the async-leaf change made leaf construction fallible/async on
the decode path; step 5's supply decoding rides that path and inherits
its contract. Before writing code: re-run the §1.2 snapshot reads at the
integrated HEAD (roles and heights as labeled), re-grep the six §6 sites,
and confirm whether the §4.4 election-bias ruling happened — it changes
your step 1 fixture rigging and step 9's scope. The red fixture from §6
step 1 is the first commit; nothing else lands until it fails for the
documented reason.

## Amendments (2026-07-23, post-implementation review)

Both levers landed, election bias first, as the §6 ordering note prescribed — that is the §4.4 ruling of record. The sections above are the specification the implementation was built from; where the landed shape diverges, the entries below govern.

**A1 (§4.3, departure arithmetic).** "One full RTT earlier" compared the new *departure* (t≈0.5) against the old *landing* (t≈1.5). Like for like: departure t≈1.0 → t≈0.5, landing t≈1.5 → t≈1.0 — **one hop (half an RTT) earlier**, not a full RTT. The pinned trace (`tests/hop_trace.rs::trace_bulk_initiator_session`) confirms: opening supplies written at hop 2 where the decomposed answer would be written at hop 3, with the session's total hops unchanged at 5 — the closing hop is still bounded by the empty-reply pairing round. On latency-only links the lever buys earlier bulk *departure* (bandwidth overlap), not a shorter critical path; the frame-count win (§1.1) is unaffected.

**A2 (§6 step 1, red-fixture recipe).** The recipe ("≥2048 messages against ≤1, rig via version bytes") predates the §4.4 lever landing first: under the `(set_len, bytes)` election a 2048-vs-1 corpus routes the bulk side into the responder role, so no version rigging can produce a bulk *initiator* at that shape. The landed red fixture makes the initiator the *smaller* set holding one exclusive root child (two leaves splitting at the second key byte) against three ballast messages, and pins the *shape* — one batched two-record Supply run on `Initiator stream 0 (height 31)`, bare `End(Reply)` at height 30 — rather than a probe-arithmetic frame count (`tests/gossip_snapshot.rs::bulk_initiator_ships_opening_supplies`). §1.1's counts remain the motivation, not the pin.

**A3 (§6 step 9, which commit carries the retire diff).** The into-bootstrapper "diff is the win" expectation landed with the *election* commit, not this design's change: the size election flips the retiree into the responder role, whose exclusives already ship whole, so the supply-only opening leaves that snapshot byte-identical. The byte-visible height-31 relocation is pinned in the election commit's re-accept of `retire_snapshot__retire_into_bootstrapper.snap`.

**A4 (§6 step 4, landed API name).** The opening's encode-side helper landed as `opening_parts` — it splits the canonical query-then-supplies reply into the question's listing and the early supplies — with the supplies sharing the opening protocol reply (the step 3 latitude), published question-first at the proxy so the responder's root reply stays decodable while supply bulk flushes.
