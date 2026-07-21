# Alignment audit notes

Side channel of the mux-conjectures campaign (MUX-PROGRESS.md): anything
found *along the way* suggesting misalignment between the stated theorems,
the Rust implementation, and its tests gets recorded here — including
items that turn out benign, with the reasoning that cleared them. Epistemic
key as in PROGRESS.md; additionally **[reported]** = surfaced by a phase-1
reader agent, not yet re-verified by the coordinator.

## A1. Termination is not a kernel theorem — CONFIRMED, misalignment between prose and artifact

**[verified by coordinator, 2026-07-21]** MODEL.md §1 lists "(ii)
Termination: every maximal run reaches `Terminal`" under "**Proved**
about the model", via §7's ρ argument ("every step fires 1 op, so ρ
strictly decreases; run length ≤ ρ(init)"). Verified directly: the Lean
artifact contains **no ρ definition, no step-decreases-measure lemma,
and no termination theorem** (grepped `Proofs/`, `Model.lean`,
`EventDag.lean` for rho/decrease/measure/termination shapes — nothing).
The termination evidence is: the §7 paper argument [derived]; Apalache
BMC exhaustive at depth ρ(init)+1 on the Phase A instances [checked,
per-instance]; schedule-replay-to-terminal witnesses [checked,
per-instance]. The kernel-proven flagships (`Sched.deadlock_free`,
`deadlock_free_d5`) claim progress only — "no reachable stuck state" —
which is genuine deadlock-freedom, unaffected by this note.

So the honest statement of the artifact is: **deadlock-freedom
kernel-proven for all well-formed schedulable skeletons; termination
[derived] in general and [checked] per pinned instance.** MODEL.md §1's
wording ("Proved") is accurate for the Quint/Apalache phase's exhaustive
per-instance tier but overstates the Lean tier's general claim.

Remedies, either sufficient: (i) prove the ρ-decrease lemma in Lean —
likely small (define ρ as summed remaining program lengths; 23-case
action analysis, each case a list-length computation) — and derive
`terminating : ∀ runs, finite ∧ ends Terminal`; or (ii) soften MODEL.md
§1(ii) to [checked]/[derived] status. NOTE: the mux campaign's C2
positive half needs a "completes" (not just "never stuck") statement, so
remedy (i) may fall out of phase 3 anyway — prefer it.

## A2. `schedulable ⟺ event-DAG acyclicity` is checked, not proven — documented, no misalignment

**[documented in-repo]** Statement.lean says so explicitly ("the event-DAG
analysis's checked (not kernel-proven) equivalence"). Recorded here only
because the mux campaign's C1 instances will lean on "no schedule
completes" claims for adversarial skeletons; any such claim inherits
[checked] status unless the specific instance is kernel-decided (as
pyramid1's greedy jam is). Campaign rule: every "no schedule completes"
used in a theorem hypothesis must be per-instance kernel-checked.

## A3. The base theorems' transport premise vs the deployed mux — documented, the campaign's raison d'être

**[documented in-repo]** design/streaming-wire-deadlock.md (the Lean-model
gap statement): MODEL.md's premise "the pump's capacity-1 channel IS the
wire" describes the Local topology; `DeadlockFree` held while the deployed
mux composition deadlocked, because demux wire-order coupling and
flush-paced receipts were unmodeled. Not a proof defect — the theorem's
hypotheses were simply false of the composition — but it is the sharpest
known instance of "stated theorem true, system deadlocks", and the mux
campaign's model must include exactly the two unmodeled couplings.

## A5. Payload-independence is now the load-bearing boundary of C1's falsity — promote its verification

**[panel finding, 2026-07-21]** MODEL.md §1's extraction premise
("channel-op count and order depend only on each child's merge-join arm,
never on payloads") was verified once, by reading `answer.rs`/
`resolver.rs`. The mux adjudication makes it load-bearing in a new way:
σ*'s locality (hence C1's falsity) rests on every consumption-order
discriminator being announced in-band. If ANY receiver branching consumed
content beyond labels, C1 would flip true. Remedy adopted into the
phase-3 plan: Rust proptest bridge B5 — reconstruct the announced
skeleton from a frame transcript alone and check it determines the
session's channel-op structure. Until B5 lands, the premise stays
[derived] with a single manual audit behind it.

## A6. MODEL.md scope statement needs a cross-reference once the mux suite lands

**[panel finding, 2026-07-21]** "The pump's capacity-1 channel IS the
wire" is true of `mirror_connected` (Local) only; once `wc_impossibility`
lands, the single-pipe transport the base model omits is formally
indicted, and MODEL.md §1's "Explicitly not modeled" should point at the
Mux/ subtree so nobody reads `DeadlockFree` as covering the old remote
transport.

## A7. Capacity monotonicity: assumed in prose, consumed by nothing — keep it that way

**[panel finding, 2026-07-21]** The artifact's standing capacity-
monotonicity claim (window.rs: "every schedule live at the floor stays
live at any width"; the latency doc's Kahn argument) is consumed by NO
theorem of record in the mux suite (σ*'s final formulation dropped it;
the probe's early embedding remark that leaned on it is superseded). It
is [derived]-tier only. If it reappears in any phase-3 proof, that is a
finding — either prove it or reroute.

## A8. Probe transcription deviation, reconciled by a theorem

**[panel finding, 2026-07-21]** The Python probe fuses walkCommit +
walkFire when driving σ*, while the model of record keeps commits
adversarial. `commit_totality` (suite item T1: W/D1/D4/D6 totally order
each scope's publications under `.impl`) proves the fusion WLOG.
Recorded so the probe is not read as modeling a different system; if T1
fails to close, the probe's σ* evidence weakens accordingly.

## A10. Global publication order is not a function of the trees — A5's premise holds per channel only

**[checked, 2026-07-21, stage-2D]** B5's first formulation ("the trace
is a function of the skeleton") is FALSE at global-interleaving
granularity even for identical inputs run back-to-back:
`complete_initiator`'s terminal `tokio::select!` is unbiased, so branch
order draws tokio's thread-local RNG. Discovered by committed regression
seeds (`proptest-regressions/tree/mirror/streaming/tests/announced.txt`).
The landed B5 bridge states payload-independence PER CHANNEL — which is
the granularity MODEL.md §1's premise actually uses; cross-channel
interleaving is scheduler freedom the model quantifies over
adversarially. No Lean misalignment; recorded so A5's phrase
"channel-op count and order" is never read globally. Corollary for the
mux campaign: any σ\* implementation's inference must likewise never
assume a deterministic global interleaving, only per-channel order.

## A9. The F8 close-guard conjunct is vacuous on well-formed skeletons — boundary hardening, not a protocol fix

**[proven-adjacent, 2026-07-21, stage-2A]** The adjudication required the
strengthened wire `recvClose` guard (no in-flight frames for the channel
in the producer's pipe) with a must-fail control showing the unstrength-
ened guard admits a bogus terminal. Track A's formalization found the
control necessarily lives on an ILL-FORMED gadget: on well-formed
skeletons the conjunct never bites, because a wire close requires the
consumer past its last scope, and BFS alignment equates consumer
receives with producer sends — no frame can be in flight at close time
(`Mux/Controls.lean` module doc; `gadget_not_wellFormed` pins the
gadget's status deliberately). So F8 defends the totality boundary of
`mstuck`/`mterminal` over arbitrary `Skel`, not a reachable protocol
state. No misalignment — recorded so nobody later reads the F8 control
as evidence of a live protocol hazard.

## A4. Reader-visible claims to spot-check opportunistically

**[reported]** Low-priority, none currently believed wrong:

- rust-streaming reader: `Trace::assert_valid` has **seven** checks on
  this branch (d6 parent placement added), while README.md's
  assumption/theorem table describes six — confirm the README table was
  updated with the d6 row when the flagship landed.
- lean-model reader: `lake build` warm ≈ 40s was inferred from olean
  mtimes, not a clean build — measure before scheduling decide-heavy new
  modules.
