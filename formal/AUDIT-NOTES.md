# Alignment audit notes

Side channel of the mux-conjectures campaign (MUX-PROGRESS.md): anything
found *along the way* suggesting misalignment between the stated theorems,
the Rust implementation, and its tests gets recorded here — including
items that turn out benign, with the reasoning that cleared them. Epistemic
key as in PROGRESS.md; additionally **[reported]** = surfaced by a phase-1
reader agent, not yet re-verified by the coordinator.

## A1. Termination: prose claims a theorem, the artifact has a witness — RESOLVED, wording gap only

**[reported, 2026-07-21]** MODEL.md §1 lists "(ii) Termination: every
maximal run reaches `Terminal`" under "**Proved** about the model", with
the ρ-decrease argument (§7). The phase-1 Lean reader found **no
standalone kernel termination theorem**: termination evidence is (a) the
ρ argument as prose, (b) `replaySchedule` running the canonical schedule
to `terminal` on the pinned matrix, gate-checked executably. If accurate,
"proved" in MODEL.md §1(ii) overstates the Lean artifact for the general
statement (every maximal run, every well-formed schedulable skeleton),
though the DeadlockFree flagships are unaffected.

To verify: search for a lemma of shape `ρ`-decrease
(`apply … → rho s' < rho s`) or `∀ run, maximal → terminal` in Proofs/;
check whether MODEL.md §7's "no fairness hypothesis: every action strictly
decreases ρ" is a Lean lemma or only a design argument. If confirmed,
either (i) prove the ρ-decrease lemma (likely small: 23-case action
analysis) and derive termination, or (ii) soften MODEL.md §1(ii) to
[checked] status. Surfaced to Finch in the phase-1 report.

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

## A4. Reader-visible claims to spot-check opportunistically

**[reported]** Low-priority, none currently believed wrong:

- rust-streaming reader: `Trace::assert_valid` has **seven** checks on
  this branch (d6 parent placement added), while README.md's
  assumption/theorem table describes six — confirm the README table was
  updated with the d6 row when the flagship landed.
- lean-model reader: `lake build` warm ≈ 40s was inferred from olean
  mtimes, not a clean build — measure before scheduling decide-heavy new
  modules.
