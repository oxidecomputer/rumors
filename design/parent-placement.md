# Parent placement: the deadlock-freedom / pipelining trade

Status: analysis complete (2026-07-18). Origin: the formal deadlock-
freedom campaign (`formal/PROGRESS.md` §7) discovered that the model's
schedule and the streaming encoder place a scope's parent resolution on
opposite sides of the scope's final sends, and that the two placements
are the two corners of a genuine design space: one buys deadlock
freedom at any assembler capacity, the other buys maximal pipelining
and pays for it with a hard capacity floor. Both corners are now
characterized — one by a kernel-checked counterexample and capacity
bound, the other by a machine-checked theorem in progress.

Epistemic key, following `formal/PROGRESS.md`: **[kernel]** = a
Lean-kernel-checked theorem (no native trust); **[checked]** = observed
in an executable run (model fuzz sweep or Rust test, committed as a
pin); **[derived]** = argument from the code or model in this document;
**[assumed]** = explicitly adopted without proof.

## 1. The two placements

A walk resolving a scope emits, per disputed child: the wire (its own
child summary), the child's resolution, then the child's queries — the
sends that launch the grandchild descent. Witnessed (agreeing) children
contribute only wires. The scope's *parent resolution* — its summary,
handed up the assembler tower for reassembly — can go in two places:

- **Parent-late (the shipping encoder)**: after the child loop, in the
  scope epilogue — after the last disputed child's queries and any
  trailing witnessed-child wires. Deliberate: "Launch every `Pending`
  slot's work before publishing its enclosing parent resolution"
  (`src/tree/mirror/streaming/materialized/levels.rs`). **[checked]**
- **Parent-early (the formal model's weave)**: immediately after the
  final disputed child's resolution, before that child's queries and
  before trailing wires. This placement is load-bearing for the
  campaign's counting proofs (`formal/PROGRESS.md` §5) and is minted as
  the model's `d5` ordering ledger. **[checked]** against the model;
  the mismatch with the encoder was finding #7 of the campaign.

The parent resolution never crosses the wire: it is local pipeline
traffic between levels. Everything below follows from that one fact
plus channel capacities. **[derived]**

## 2. What parent-late costs: a hard capacity floor

With the parent withheld until the epilogue, a walk can be parked on a
jammed send (a query into a full cap-1 asked channel; a trailing wire)
while the level above starves for the very resolution it is
withholding. At tight assembler capacity the wait closes into a cycle:

    walk commits a last-chunk query, parent unsent
      → assembler above starves for that parent
      → the backed-up tower stops draining an upper channel
      → the walk that would drain the jammed channel is itself
        parked on an upper send → cycle closed.

This is not hypothetical in the model: `Control.parentTrap`
(`formal/lean/StreamingMirror/Controls.lean`) exhibits a well-formed,
schedulable skeleton whose parent-delaying run is stuck — refuting
deadlock-freedom for the parent-late order at small capacities
**[kernel]**. The boundary is exact: the trap needs a scope disputing
`capLevel + 2` children (the model's `schedulable` bound, met with
equality); one child fewer completes **[checked]** (executable
minimality search, `formal/lean/EventDag.lean`).

The Rust side pins the same boundary from both directions: on the
[32,256] dispute pyramid the pipeline stalls with the assembler channel
at 253 and completes at 254 = fan − 2
(`capacity_stress_witness_requires_inter_level_fan`) **[checked]**.

The −2 floor is **poll-schedule-specific, not interleaving-robust**
**[checked]**: under the model's epilogue ledger (`AxMode.impl`, which
*forces* the encoder's per-walk order), the boundary skeleton `pdelay`
still stalls under adversarial cross-process interleaving — the
stalling run is epilogue-legal by construction. The encoder's observed
completion at the −2 boundary depends on the poll schedules its
runtime actually produces. The interleaving-robust floor is margin 0
(assembler ≥ max per-scope disputes), which is the theorem hypothesis
adopted in §6.

Why the floor is capacity − 2 rather than capacity: a bounded channel
accommodates two in-flight items beyond its buffer, one borrowed at
each end **[checked]** (in the Rust by the 253/254 pins; in the model
by `pdelay`'s `.impl` stuck-state accounting — level occupancy 2, one
assembler mid-collection, three walks parked on committed sends —
exhibiting all three loci). A producer parked on `send` has already
computed its item and holds it in hand (in the model, a committed-but-
unfired send; in the Rust, the parked `Sender::send` future); the
consumer holds one popped item while it works. So a scope with
`dCount ≤ C + 2` can have every return simultaneously in flight with
no downstream progress required — C buffered, one in each hand — while
the `C + 3`rd return needs the assembler to have disposed of one,
which at the trap shapes transitively requires the walk's own further
progress. Both borrowed slots are implementation-contingent, not
interface facts (rendezvous-style parked senders; a pop-then-process
consumer loop) — which is why `C ≥ dCount`, needing no borrows, is the
bound one can prove against the channel *interface*, and the bound the
shipping `FAN = 256` was chosen by.
Hence the floor: **parent-late is live iff the assembler channel
capacity is at least the maximum per-scope dispute count minus 2** —
at radix 256, capacity 254; the shipping `FAN = 256` clears it with
margin 2. The floor does not compound across scopes: return backlog
never spans sibling parents, so stage width is irrelevant — stages of
9/27/81 scopes complete at capacity 1 per-walk channels
(`parent_delay_no_cross_parent_backlog`) **[checked]**.

## 3. What parent-early buys: liveness at any capacity

Parent-early is a token-release discipline: a walk never withholds its
upward obligation while entering sends that can jam. The cycle in §2
cannot form — by the time the walk can park on a last-chunk query or
trailing wire, the level above already holds the parent. The formal
statement is the campaign's `d5` theorem (in progress at this writing):
under the `d5` ledger, deadlock freedom holds at **every** assembler
capacity ≥ 1, with no shape condition beyond `schedulable`.

Parent-early is realizable as an encoder, not an oracular artifact
**[derived]**: the only lookahead it needs is which disputed child is
the scope's last, and the protocol delivers the scope's full child
classification before the walk emits anything (the walk receives the
peer's wire for the scope — one bounded radix-256 node — before its
sends; model edge family E3, fuzz-validated). The parent's content is
also available at that point: it summarizes the scope's decisions, not
the grandchildren's eventual results, and every decision is made when
the last disputed child resolves. Caveat: content-availability rests
on the model's dependency edges being faithful to the real message
contents, which is fuzz-validated, not re-derived from `src/`; verify
directly before ever building this encoder.

## 4. The pipelining cost of parent-early

Where the cost lands:

- **Local and distributed, but distributed only via departure pacing.**
  Since parents never cross the wire, placement cannot change the
  peer's logical dependencies. But under parent-early the walk parks on
  assembler acceptance *before* emitting the trailing wires (which the
  peer's corresponding scope waits on) and the last chunk's queries
  (which launch the subtree descents). A purely local backpressure
  event therefore delays peer-visible traffic. Under parent-late,
  assembler backpressure lands on the epilogue, where nothing
  downstream or across the wire depends on it. **[derived]**
- **Round-trips: unchanged in count and RTT-depth.** The wire exchange
  structure is fixed by tree shape and dispute structure. What
  parent-early degrades is overlap: it inserts a rendezvous between
  each scope's tail and the upward assembly path. Worst case (deep
  dispute chains through last-disputed children, tight assembler
  capacity) the rendezvous chains and latency degrades from
  max-over-paths toward sum-over-levels — the lockstep the streaming
  design exists to escape. At generous capacity the block is rare and
  the cost shrinks toward one send's latency, but the coupling is
  structural. **[derived]**

The underlying asymmetry is a criticality ordering: the parent
resolution is the *least* urgent send a walk makes — its consumer must
wait for subtree results from below regardless, so early delivery buys
nothing — while the sends it would preempt are the *most* urgent,
heading the protocol's longest dependency chains. Parent-late is "emit
by criticality; defer the deferrable." **[derived]**

## 5. The design space

| | parent-early (`d5`) | parent-late (shipping) |
|---|---|---|
| assembler floor | none (any capacity ≥ 1) | ≥ max per-scope disputes − 2 (254 at radix 256) |
| descent/assembly coupling | rendezvous at every scope tail | fully decoupled |
| peer-visible departure pacing | gated by local assembler slack | never gated |
| round-trip count / RTT-depth | unchanged | unchanged |
| formal status | theorem in progress **[kernel]** target | conditional theorem planned (§6) |

Revisit parent-early only if the capacity floor becomes untenable —
e.g. a memory-constrained peer that cannot afford FAN-deep assembler
buffers. It is the priced retreat position, not a latent bug fix: at
production capacities the probes show the shipping order is safe, and
moving it would be a pipelining regression. **[derived]**

## 6. Resolution adopted

The implementation-facing theorem is being re-targeted at the shipping
order: deadlock freedom under the encoder's per-walk emission order
(the epilogue placement, minted as a model ledger so `Trace::
assert_valid` can pin it on real traces) with arbitrary cross-process
interleaving, walk channels at capacity 1, matching the verification
stress regime. The capacity hypothesis is deliberately the robust
margin-0 bound — assembler capacity ≥ max per-scope dispute count,
the shipping `FAN ≥ kids ≥ dCount` discipline — not §2's tight −2
floor. Margin 0 means level sends never park at all, so the two
borrowed slots (implementation-contingent) never enter the proof, the
level-channel back-pressure edges never bind, and the boundary's
interleaving-sensitivity questions dissolve; the tight floor remains
characterized by the kernel counterexample and the executable pins of
§2 rather than carried through the kernel proof. Production
capacities only widen channels; coverage of widened configurations is
by capacity monotonicity — **[proven]** for this flagship since
2026-07-21 (`Sched.deadlock_free_wide`, formal/lean/StreamingMirror/
Proofs/Wide.lean: deadlock freedom and the run-length bound at every
pointwise capacity vector κ ≥ the floor). The Kahn
argument (with per-walk order fixed, each process is deterministic in
its I/O behavior, and in such process networks added buffer capacity
only relaxes back-pressure and cannot introduce deadlock) remains the
informal coverage for the `d5` corner only. The `d5`
theorem is retained as the capacity-universal counterpart documenting
the other corner. Campaign state of record: `formal/PROGRESS.md` §7.
