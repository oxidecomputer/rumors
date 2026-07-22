/-
The chartered follow-on campaigns: documentation-only module.

Two theorem campaigns were chartered at the mux campaign's close and
deliberately not begun. This module is their charter of record — the
statements a future prover transcribes, spec-first (the English fixed
before the build; transcription choices recorded, dated, before any
theorem lands — the method that carried T8 to an every-clause-EXACT
crosswalk, Proofs/SigmaStarKLive.lean).

# T11: the forced-window theorem

QUESTION. The campaign proved windowing SUFFICIENT (T8). Is it
NECESSARY — must every correct local scheduler effectively implement
it?

THE KERNEL-PROVEN BRACKET, pointing at yes: work conservation is fatal
(`wc_impossibility`), so some withholding is necessary; wrong static
orders are fatal (`static_oracle_jams`), so the withholding must be
informed; erased-trace inference starves (`wedge_evidence_starves`'s
cousin at the observation tier: the payload finding, Mux/Causal.lean's
module doc), so the information must come from frame contents; the
omniscient license is not locally computable (`oracle_not_local`), so
it must be inferred from announcements; and the announcement-inferred
window discipline suffices (`sigmaStarK_deadlock_free`).

THE STATEMENT SHAPE. Every charter-local strategy pair that is
deadlock-free on the full well-formed margin-0 class is
LICENSE-BOUNDED at every reachable observation: its pushes never
exceed the causal closure's license by more than the harness slack.
Extensionally this is "behaves as if tracking receiver occupancy and
imposing per-stream backpressure on itself" — no theorem can constrain
how a function computes, only what it computes.

THE PROOF PLAN. The fooling argument, one level up: given a live local
σ and a reachable observation where σ pushes an unlicensed frame,
construct a skeleton extension — consistent with everything σ has
observed — where that push initiates a burial no continuation escapes.
In hand: charter-grain indistinguishability (`CharterLocal`, its
nondegeneracy pins), the burial gadget family
(Proofs/WcImpossibilityK.lean's widened wedges), the closures. NEW
WORK: the adversarial extension lemma (realize a punishing skeleton
behind any observation), and the slack accounting — how many
unlicensed frames the pipe floats harmlessly is where the theorem's
teeth live.

SCOPE CAVEAT. Necessity holds relative to liveness on ALL skeletons; a
scheduler tuned to a restricted workload class may legitimately
over-push, and the statement must say so.

VALUE. Converts the product conclusion (the `Link` contract stands —
design/single-socket.md's revision of record) from theorem-backed to
theorem-stated: windowing is not just sufficient but what correctness
means over one channel.

# The latency conjectures

QUESTION. Does an RTT-optimal scheduler exist that is computable from
local causal information only — and is that formally decidable with a
constructive witness?

SETTLED AT EVIDENCE TIER. The per-skeleton RTT-optimum is the
δ-weighted critical path of the event DAG (the multi-link baseline
achieves it; the deepest dispute path is information-theoretically
sequential). σ*ₖ at K ≥ P\* + 1 matches it — the K-dial law,
probe-exact on a 54-cell sweep (design/mux-latency.md §7) — so a LOCAL
optimal scheduler exists at sufficient window.

THE CONJECTURE (constrained K < P\* + 1): no local strategy is
optimal; the local penalty is exactly one reverse-evidence leg per
pacing cycle — asymptotically a factor ~2 on the width term (the
probe's omniscient/causal 50/50 split is the motivating datum) — and
σ*ₖ is optimal among local strategies. Locality costs nothing for
liveness and one wire leg per cycle for latency, vanishing when the
window clears the frontier. A fourth construction sharpens it:
credit-windowed multi-link at constrained W is conjectured
width-term-identical to σ*ₖ (real credits are as lagged as real
inference — the zero-lag baseline is the shared-memory model's
fiction, not a transport), and strictly worse on silent runs.

THE THREE-THEOREM PLAN, after a timed-run semantics lands (a thin
layer; the RTT-metered harness in design/mux-latency/ is its
calibration oracle, probe-first): (i) the DAG critical-path lower
bound over all schedulers (EventDag.lean's depth machinery is the
unweighted version); (ii) the constructive optimal witness — σ*ₖ at
K ≥ P\* + 1 via the formalized K-dial law, or the timed
send-projection oracle; (iii) the local impossibility at constrained
K — quantitative fooling over view-equal pairs (the two-witness
structure of `oracle_not_local`, upgraded from "the orders differ" to
"the achievable times differ by at least the proof-lag term").

SEQUENCING. After the tracecheck plan (design/tracecheck.md) or
independently; both wait on the single-R/W refactor landing first.
-/

/-- Documentation-only module: the chartered follow-on campaigns (T11,
the forced-window theorem; the latency conjectures). See the module
doc. -/
def StreamingMirror.Mux.charters : Unit := ()
