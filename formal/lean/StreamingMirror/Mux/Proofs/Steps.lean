/-
The step vocabulary for the stage-F `MuxInv` preservation sweep: what
one enabled base action does to
the counting layer, the demux hands, and the local invariant — stated
once, so the per-arm extraction files (Steps/Top.lean, Steps/WalkAsm.lean,
Steps/Fire.lean) all deliver the same three-part contract and the
`MuxInv` dispatcher (SigmaStarInv.lean) consumes it uniformly.

# Why this layer exists (the flagged stage-3 cost)

The base model's preservation monoliths (Proofs/Preserve/*) prove
`InvP → InvP`, and `InvP.flow` — the unmuxed conservation law — is FALSE
at muxed states with frames in flight, so the monoliths cannot be
invoked on the muxed base state at all: a wire channel's occupancy is
the demux slot, not `sentOf − recvdOf`. There is no companion-state
detour either — the flow equation pins `chan` to `sentOf − recvdOf`,
which exceeds the wire cap exactly when frames are in flight — so the
per-arm facts are re-proved here against `InvL` (the chan-blind local
fragment) plus explicit count deltas. The monoliths are NOT rewritten;
their exported frame lemmas (`sentOf_setWalk_same`, `sentOf_ext`,
`wkLocalOk_congr`, …) are the engine, and each arm's local bullet is
the monolith's own, minus the flow field.

# The contract

Each arm lands one lemma concluding `InvL sk ax s'` together with:

- `QuietStep` — commits, choices, closes: no channel operation.
- `RecvStep c₀` — one receive on `c₀`: occupancy down one, the
  consumer count up one, everything else framed.
- `SendStep c₀` — one send into `c₀`: occupancy up one within the cap,
  the producer count up one, everything else framed.

and a hands clause: `HandsEq` (no committed wire hand appears or
disappears) or the commit-arm flip (`holdsWire` turns on at exactly one
stream). Count deltas are `allChans`-relativized, exactly like the
monolith frame lemmas they come from: the phantom channel `wire I 0`
aliases walk `(R, 0)`'s consumer count by Nat subtraction (Wiring.lean's
note) and is outside `allChans`, so no unrelativized delta is true.
-/
import StreamingMirror.Mux.Basic
import StreamingMirror.Proofs.Preserve.Walk

namespace StreamingMirror.Mux

open Model

variable {sk : Skel} {ax : AxMode} {s s' : State}

-- ======================================================== count deltas

/-- No channel operation: every occupancy and count is untouched
(commits, obligation choices, close observations). -/
structure QuietStep (sk : Skel) (s s' : State) : Prop where
  chan : s'.chan = s.chan
  sent : ∀ c ∈ allChans sk, sentOf sk s' c = sentOf sk s c
  recvd : ∀ c ∈ allChans sk, recvdOf sk s' c = recvdOf sk s c

/-- One receive on `c₀`: the guard saw data, occupancy drops by one,
the consumer count rises by one, and every other `allChans` count is
framed. -/
structure RecvStep (sk : Skel) (s s' : State) (c₀ : Chan) : Prop where
  hpos : 0 < s.chan c₀
  chan : s'.chan = bump s.chan c₀ (-1)
  sent : ∀ c ∈ allChans sk, sentOf sk s' c = sentOf sk s c
  recvd : ∀ c ∈ allChans sk,
    recvdOf sk s' c = recvdOf sk s c + (if c = c₀ then 1 else 0)

/-- One send into `c₀`: the guard saw cap room, occupancy rises by one,
the producer count rises by one, and every other `allChans` count is
framed. -/
structure SendStep (sk : Skel) (s s' : State) (c₀ : Chan) : Prop where
  hcap : s.chan c₀ < sk.cap c₀
  chan : s'.chan = bump s.chan c₀ 1
  sent : ∀ c ∈ allChans sk,
    sentOf sk s' c = sentOf sk s c + (if c = c₀ then 1 else 0)
  recvd : ∀ c ∈ allChans sk, recvdOf sk s' c = recvdOf sk s c

-- ============================================================== hands

/-- No committed wire hand appears or disappears: the `holdsWire` map is
pointwise unchanged. Every arm but the wire commits (and the push
itself) satisfies this. -/
def HandsEq (sk : Skel) (s s' : State) : Prop :=
  ∀ p h, holdsWire sk p h s' = holdsWire sk p h s

/-- Does this walk record hold a committed wire obligation in its
publishing phase? The walk-side kernel of `holdsWire`. -/
def wireHand (ws : WalkSt) : Bool :=
  ws.phase == 2 &&
    (match ws.committed with
     | some (.wire _) => true
     | _ => false)

/-- `holdsWire` off the root reads exactly key membership plus the
walk's `wireHand`. -/
theorem holdsWire_eq_wireHand {p : Party} {h : Nat} (hr : h ≠ sk.rootH) :
    holdsWire sk p h s = (sk.walkKeys.contains (p, h) && wireHand (s.walk (p, h))) := by
  rw [holdsWire.eq_def, if_neg (by simpa using hr), wireHand]
  simp only []
  rcases hcm : (s.walk (p, h)).committed with - | o
  · simp
  · cases o <;> simp

/-- Hands framing for an arm that touches only walk `pk`, leaving its
`wireHand` off on both sides (receives, non-wire fires, closes). -/
theorem handsEq_of_walk (pk : Party × Nat)
    (hio : s'.iopenCh = s.iopenCh) (hro : s'.ropenCh = s.ropenCh)
    (hne : ∀ pk', pk' ≠ pk → s'.walk pk' = s.walk pk')
    (hoff : wireHand (s.walk pk) = false)
    (hoff' : wireHand (s'.walk pk) = false) :
    HandsEq sk s s' := by
  intro p h
  by_cases hr : h = sk.rootH
  · subst hr
    rw [holdsWire.eq_def, holdsWire.eq_def]
    simp only [beq_self_eq_true, if_pos]
    cases p
    · rw [hio]
    · rw [hro]
  · rw [holdsWire_eq_wireHand hr, holdsWire_eq_wireHand hr]
    by_cases hpk : ((p, h) : Party × Nat) = pk
    · rw [hpk, hoff, hoff']
    · rw [hne _ hpk]

/-- Hands framing for an arm that touches neither the openers' choice
slots nor any walk (assemblers, absorber, finishes). -/
theorem handsEq_of_other
    (hio : s'.iopenCh = s.iopenCh) (hro : s'.ropenCh = s.ropenCh)
    (hwk : ∀ pk, s'.walk pk = s.walk pk) :
    HandsEq sk s s' := by
  intro p h
  by_cases hr : h = sk.rootH
  · subst hr
    rw [holdsWire.eq_def, holdsWire.eq_def]
    simp only [beq_self_eq_true, if_pos]
    cases p
    · rw [hio]
    · rw [hro]
  · rw [holdsWire_eq_wireHand hr, holdsWire_eq_wireHand hr, hwk]

-- =========================================== chan-blindness of `InvL`

/-- The local invariant never reads channel occupancy: `wkLocalOk`,
`asmLocalOk`, and `topLocalOk` are cursor predicates. This is what lets
the muxed system — whose occupancies obey a pipe-mediated law — reuse
every per-arm local bullet unchanged. -/
theorem InvL.chan_blind {ch : Chan → Nat} (hL : InvL sk ax s) :
    InvL sk ax { s with chan := ch } := by
  refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
  · rw [wkLocalOk_congr sk ax pk rfl]
    exact hL.wk pk hpk
  · rw [asmLocalOk_congr sk pk rfl]
    exact hL.asm pk hpk
  · rw [topLocalOk_congr sk ax rfl rfl rfl rfl rfl rfl rfl rfl rfl rfl
      rfl rfl]
    exact hL.top

/-- Producer counts never read channel occupancy (the WalkFire-monolith
fact, restated exported). -/
theorem sentOf_chan_blind (ch : Chan → Nat) (c : Chan) :
    sentOf sk { s with chan := ch } c = sentOf sk s c := by
  cases c <;> rfl

/-- Consumer counts never read channel occupancy. -/
theorem recvdOf_chan_blind (ch : Chan → Nat) (c : Chan) :
    recvdOf sk { s with chan := ch } c = recvdOf sk s c := by
  cases c <;> rfl

/-- `holdsWire` never reads channel occupancy. -/
theorem holdsWire_chan_blind (ch : Chan → Nat) (p : Party) (h : Nat) :
    holdsWire sk p h { s with chan := ch } = holdsWire sk p h s := by
  rw [holdsWire.eq_def, holdsWire.eq_def]

end StreamingMirror.Mux
