/-
T6, the necessity corollary (MUX-ADJUDICATION.md §3), and T5 in its
final, unconditional form: the stage-4 closure of the mux campaign's
trichotomy.

# Class-relativity, spelled out

The charter's necessity half ("the oracle's dependence on remote
information is essential; that necessity is exactly C1") died with
C1-literal and is restated class-relatively (MUX-ADJUDICATION §1.3):
nonlocal information is necessary for liveness UNDER WORK-CONSERVATION
— one fixed, tree-realizable skeleton (`wedge`) defeats every
work-conserving pair at every capacity, locality not even assumed — and
NOT for liveness alone (the σ* refutation of C1-literal, T4's
territory; kernel-adjacent here: `wedge_idler_completes` shows the same
skeleton completes once idling is allowed).

# The mysterious third thing, named

The signal strictly weaker than the full remote skeleton that suffices
is: the announcement prefix the protocol already carries, plus FIFO
positional arithmetic, plus the inevitability closure — nothing new on
the wire at all. What credits smuggle across is not information but
per-stream consumption evidence one hop early (the per-stream E2 edge
family the single pipe conflates); an idling scheduler can DERIVE that
evidence, a work-conserving one must push regardless, and that
asymmetry is exactly where the impossibility class splits
(MUX-ADJUDICATION §1.3–1.4).

Track E's sharpening: the oracle in the positive conjunct is a FIXED
send-order list (`sendProj`), consulted only through the machine's own
push count — non-adaptive, and nonlocal only through the skeleton
(`oracle_not_localStrategy`, the strategy-level refutation with
`Consistent` certificates). So within the work-conservation reading, what the
oracle buys with remote structure is the ORDER, and the receive-order
static pusher shows the wrong order is fatal even with the same
knowledge (`static_oracle_jams`). Neither adaptivity nor extra wire
signal appears anywhere in the trichotomy's positive half at C = 1.
-/
import StreamingMirror.Mux.Proofs.SigmaStarInv
import StreamingMirror.Mux.Proofs.Oracle
import StreamingMirror.Mux.Proofs.WcImpossibility

namespace StreamingMirror.Mux

variable {sk : Skel}

/-- T5, `oracle_deadlock_free`, unconditional (MUX-ADJUDICATION §3 T5 —
the adjudication's fallback SLOT, realized as the static
send-projection pusher: no state feedback needed, the track-E
sharpening): the send-projection pusher completes every well-formed
margin-0 skeleton over the single-pipe transport at every capacity
C ≥ 1 — C₀ = 1 suffices.

"Completes" is two kernel facts: no reachable stuck state (this
theorem) and no infinite run (`mux_terminating`,
Mux/Proofs/Termination.lean) — packaged there as
`oracle_greedy_run_terminal`, the greedy oracle drain reaching
`mterminal` within 2·ρ(init) steps.

Capacity is denominated in messages (= scope replies); byte-level
soundness of one-reply slots is design/streaming-wire-deadlock.md §5A's
W = 1 structural argument, assumed at the model boundary
(MUX-ADJUDICATION §2.5). Overlap/latency optimality is NOT claimed
(H-c is executable-tier only). -/
theorem oracle_deadlock_free (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) (C : Nat) (hC : 1 ≤ C) :
    MuxDeadlockFree sk .impl C (oracle .I) (oracle .R) :=
  oracle_deadlock_free_of_muxInv hwf hm0 C hC fun _ hr =>
    muxInv_reachable hwf hr

/-- T6, `necessity` (MUX-ADJUDICATION §3): the trichotomy's two halves
conjoined — the wedge kills every work-conserving pair at every
capacity, while the oracle completes every margin-0 skeleton at
capacity one ("completes" in T5's grounded sense: stuck-freedom here,
termination via `mux_terminating`, Mux/Proofs/Termination.lean).

Read per the module doc: nonlocal information is necessary for
liveness under work-conservation, and not for liveness alone. The
work-conserving class in the first conjunct is kernel-inhabited
(`bottomMostReady_wc`, Mux/Proofs/Inhabitation.lean). Capacities are
message-denominated; the byte caveat of record is Mux/Basic.lean's
module doc (# The byte-denomination caveat). -/
theorem necessity (C : Nat) (hC : 1 ≤ C) :
    (∀ σI σR : Strategy, WorkConserving .I σI → WorkConserving .R σR →
        ¬ MuxDeadlockFree wedge .impl C σI σR)
    ∧ (∀ sk : Skel, sk.wellFormed = true →
        (∀ sc, sk.dCount sc ≤ sk.capLevel) →
        MuxDeadlockFree sk .impl 1 (oracle .I) (oracle .R)) :=
  ⟨fun σI σR hWI hWR => wc_impossibility C hC σI σR hWI hWR,
   fun _ hwf hm0 => oracle_deadlock_free hwf hm0 1 (Nat.le_refl 1)⟩

end StreamingMirror.Mux
