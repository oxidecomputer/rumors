/-
Glue for the `MuxInv` preservation induction (MUX-ADJUDICATION.md §4,
the stage-F obligation; stage-3 track E): everything the assembly
(Mux/Proofs/Preserve.lean) needs that is not a per-arm extraction —
observation-history counting under `recordObs`, channel-blindness of
the local invariant and the derived counts, the two opener push
shapes, and full inversions of `applyBase` and `firePush`.

The per-arm extractions from the monolithic base preservation proofs
live in Preserve/{TopFin,WalkAsm,Fire}.lean; this file is deliberately
free of them so it can anchor the statement shapes they target.
-/
import StreamingMirror.Mux.Proofs.Oracle.Order

namespace StreamingMirror.Mux

open Model

variable {sk : Skel}

-- ================================================ history counting

/-- Deliveries see only `.delivered` observations (act twin). -/
theorem delHeights_append_act (tr : List MObs) (a : Action) :
    delHeights (tr ++ [.act a]) = delHeights tr := by
  unfold delHeights
  rw [List.filterMap_append]
  simp

/-- Deliveries see only `.delivered` observations (push twin). -/
theorem delHeights_append_pushed (tr : List MObs) (h : Nat) :
    delHeights (tr ++ [.pushed h]) = delHeights tr := by
  unfold delHeights
  rw [List.filterMap_append]
  simp

/-- A delivery receipt appends its height. -/
theorem delHeights_append_delivered (tr : List MObs) (h : Nat) :
    delHeights (tr ++ [.delivered h]) = delHeights tr ++ [h] := by
  unfold delHeights
  rw [List.filterMap_append]
  rfl

/-- `recordObs` case split: a machine's history either gains the one
observation or is untouched. -/
theorem recordObs_cases (hist : Party → List MObs) (p : Party)
    (o : MObs) (q : Party) :
    recordObs hist p o q = hist q
      ∨ (q = p ∧ recordObs hist p o q = hist q ++ [o]) := by
  unfold recordObs
  by_cases hq : (q == p) = true
  · exact Or.inr ⟨by simpa using hq, by rw [if_pos hq]⟩
  · exact Or.inl (by rw [if_neg (by simpa using hq)])

-- ============================================== channel blindness

/-- The local invariant never reads channel occupancy. -/
theorem invL_chan {ax : AxMode} {s : State} (hi : InvL sk ax s)
    (ch : Chan → Nat) : InvL sk ax { s with chan := ch } :=
  ⟨fun pk hpk => by
      rw [wkLocalOk_congr sk ax pk rfl]
      exact hi.wk pk hpk,
   fun pk hpk => by
      rw [asmLocalOk_congr sk pk rfl]
      exact hi.asm pk hpk,
   by
      rw [topLocalOk_congr sk ax rfl rfl rfl rfl rfl rfl rfl rfl rfl rfl
        rfl rfl]
      exact hi.top⟩

/-- Producer counts never read channel occupancy. -/
theorem sentOf_chan (s : State) (ch : Chan → Nat) (c : Chan) :
    sentOf sk { s with chan := ch } c = sentOf sk s c := by
  cases c <;> rfl

/-- Consumer counts never read channel occupancy. -/
theorem recvdOf_chan (s : State) (ch : Chan → Nat) (c : Chan) :
    recvdOf sk { s with chan := ch } c = recvdOf sk s c := by
  cases c <;> rfl

-- ============================================ the opener push shapes

/-- The initiator's opening push effect preserves the local invariant:
the choice slot empties and the fired flag sets — both invisible to
walks and assemblers, and the opener conjuncts only loosen. -/
theorem preserveL_iopenWire {ax : AxMode} {s : State}
    (hi : InvL sk ax s) :
    InvL sk ax { s with iopenWire := true, iopenCh := none } := by
  refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
  · rw [wkLocalOk_congr sk ax pk rfl]
    exact hi.wk pk hpk
  · rw [asmLocalOk_congr sk pk rfl]
    exact hi.asm pk hpk
  · have htop := hi.top
    unfold topLocalOk at htop ⊢
    simp only [Bool.and_eq_true] at htop ⊢
    obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨-, -⟩, h3⟩, h4⟩, h5⟩, h6⟩, h7⟩, h8⟩, h9⟩,
      h10⟩, h11⟩, h12⟩ := htop
    refine ⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨?_, ?_⟩, h3⟩, h4⟩, h5⟩, h6⟩, h7⟩, h8⟩, h9⟩,
      h10⟩, h11⟩, h12⟩
    · simp
    · simp

/-- The responder's opening push effect preserves the local invariant.

Needs the hand (`ropenCh = some .wire`): the got-wire shadow conjunct
must survive `ropenWire := true`, and a committed wire hand certifies
`ropenGotWire` through the invariant itself. -/
theorem preserveL_ropenWire {ax : AxMode} {s : State}
    (hch : s.ropenCh = some ROblig.wire) (hi : InvL sk ax s) :
    InvL sk ax { s with ropenWire := true, ropenCh := none } := by
  have htop := hi.top
  unfold topLocalOk at htop
  simp only [Bool.and_eq_true] at htop
  obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨h1, h2⟩, h3⟩, h4⟩, h5⟩, h6⟩, h7⟩, h8⟩, h9⟩,
    h10⟩, h11⟩, h12⟩ := htop
  have hgw : s.ropenGotWire = true := by
    cases hgw : s.ropenGotWire with
    | true => rfl
    | false =>
        rw [hgw] at h3
        simp only [Bool.false_or, Bool.and_eq_true] at h3
        rw [hch] at h3
        simp at h3
  refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
  · rw [wkLocalOk_congr sk ax pk rfl]
    exact hi.wk pk hpk
  · rw [asmLocalOk_congr sk pk rfl]
    exact hi.asm pk hpk
  · unfold topLocalOk
    simp only [Bool.and_eq_true]
    refine ⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨h1, h2⟩, ?_⟩, h4⟩, ?_⟩, ?_⟩, ?_⟩, ?_⟩, h9⟩,
      h10⟩, h11⟩, h12⟩
    · rw [hgw]
      simp
    · simp
    · simp
    · simp
    · simp

/-- Producer-count delta of the initiator's opening push: the opening
wire rises by one, everything else frames. -/
theorem sentOf_iopenWire {s : State} (hch : s.iopenCh = some IOblig.wire)
    (hi : InvL sk .impl s) :
    sentOf sk { s with iopenWire := true, iopenCh := none }
        (Chan.wire .I sk.rootH)
      = sentOf sk s (Chan.wire .I sk.rootH) + 1
    ∧ (∀ c, c ≠ Chan.wire .I sk.rootH →
        sentOf sk { s with iopenWire := true, iopenCh := none } c
          = sentOf sk s c)
    ∧ ∀ c, recvdOf sk { s with iopenWire := true, iopenCh := none } c
        = recvdOf sk s c := by
  have hnw : s.iopenWire = false := by
    have htop := hi.top
    unfold topLocalOk at htop
    simp only [Bool.and_eq_true] at htop
    have h1 := htop.1.1.1.1.1.1.1.1.1.1.1
    rw [hch] at h1
    simpa using h1
  refine ⟨?_, ?_, ?_⟩
  · simp [sentOf, hnw, b2n]
  · intro c hc
    cases c with
    | wire p h =>
        by_cases hr : (h == sk.rootH) = true
        · cases p with
          | I =>
              exfalso
              exact hc (by rw [show h = sk.rootH from by simpa using hr])
          | R => simp [sentOf, hr]
        · simp [sentOf, hr, wkWireSent, wkWireCount]
    | _ => simp [sentOf, wkQSentTot, wkQSum, wkParentSent, wkResSent,
        wkResCount, asmOutSent, b2n]
  · intro c
    exact recvdOf_ext sk (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
      rfl rfl rfl rfl rfl rfl c

/-- Producer-count delta of the responder's opening push: the opening
wire rises by one, everything else frames. -/
theorem sentOf_ropenWire {s : State} (hch : s.ropenCh = some ROblig.wire)
    (hi : InvL sk .impl s) :
    sentOf sk { s with ropenWire := true, ropenCh := none }
        (Chan.wire .R sk.rootH)
      = sentOf sk s (Chan.wire .R sk.rootH) + 1
    ∧ (∀ c, c ≠ Chan.wire .R sk.rootH →
        sentOf sk { s with ropenWire := true, ropenCh := none } c
          = sentOf sk s c)
    ∧ ∀ c, recvdOf sk { s with ropenWire := true, ropenCh := none } c
        = recvdOf sk s c := by
  have hnw : s.ropenWire = false := by
    have htop := hi.top
    unfold topLocalOk at htop
    simp only [Bool.and_eq_true] at htop
    have h1 := htop.1.1.1.1.1.1.2
    rw [hch] at h1
    simpa using h1
  refine ⟨?_, ?_, ?_⟩
  · simp [sentOf, hnw, b2n]
  · intro c hc
    cases c with
    | wire p h =>
        by_cases hr : (h == sk.rootH) = true
        · cases p with
          | I => simp [sentOf, hr]
          | R =>
              exfalso
              exact hc (by rw [show h = sk.rootH from by simpa using hr])
        · simp [sentOf, hr, wkWireSent, wkWireCount]
    | _ => simp [sentOf, wkQSentTot, wkQSum, wkParentSent, wkResSent,
        wkResCount, asmOutSent, b2n]
  · intro c
    exact recvdOf_ext sk (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
      rfl rfl rfl rfl rfl rfl c

-- ================================================== full inversions

/-- Full inversion of `applyBase`: the base state steps by the model
arm, the pipes are untouched, the history gains the `.act`, and the
disabled wire fires stay disabled. -/
theorem applyBase_inv {ax : AxMode} {a : Action} {s s' : MState}
    (hstep : applyBase sk ax a s = some s') :
    Model.apply sk ax a s.base = some s'.base
      ∧ isWireFire s.base a = false
      ∧ s'.pipe = s.pipe
      ∧ s'.hist = recordObs s.hist (actionParty a) (.act a) := by
  have hnf' : isWireFire s.base a = false := by
    cases hIF : isWireFire s.base a with
    | false => rfl
    | true =>
        exfalso
        unfold applyBase at hstep
        rw [hIF] at hstep
        simp at hstep
  have hshape : applyBase sk ax a s = none
      ∨ applyBase sk ax a s = (Model.apply sk ax a s.base).map fun b =>
          { s with base := b
                   hist := recordObs s.hist (actionParty a) (.act a) } := by
    unfold applyBase
    dsimp only
    repeat' split
    all_goals first | exact Or.inl rfl | exact Or.inr rfl
  rcases hshape with hnone | hmap
  · rw [hnone] at hstep
    cases hstep
  · rw [hmap] at hstep
    cases hb : Model.apply sk ax a s.base with
    | none => rw [hb] at hstep; cases hstep
    | some b =>
        rw [hb] at hstep
        injection hstep with hs'
        rw [← hs']
        exact ⟨rfl, hnf', rfl, rfl⟩

/-- Full inversion of `firePush`: pipe room, the hand, and the shape of
the fired base effect — opener (either side) or walk wire. -/
theorem firePush_inv {C : Nat} {p : Party} {h : Nat} {s s' : MState}
    (hf : firePush sk C p h s = some s') :
    (s.pipe p).length < C
    ∧ s'.hist = recordObs s.hist p (.pushed h)
    ∧ s'.pipe = (fun q => if q == p then s.pipe q ++ [Chan.wire p h]
        else s.pipe q)
    ∧ ((h = sk.rootH ∧ p = .I ∧ s.base.iopenCh = some IOblig.wire
          ∧ s'.base = { s.base with iopenWire := true, iopenCh := none })
       ∨ (h = sk.rootH ∧ p = .R ∧ s.base.ropenCh = some ROblig.wire
          ∧ s'.base = { s.base with ropenWire := true, ropenCh := none })
       ∨ (∃ i, (p, h) ∈ sk.walkKeys ∧ (s.base.walk (p, h)).phase = 2
          ∧ (s.base.walk (p, h)).committed = some (Oblig.wire i)
          ∧ s'.base = setWalk s.base (p, h)
              (normWalk sk h (fireOblig (s.base.walk (p, h))
                (Oblig.wire i))))) := by
  simp only [firePush] at hf
  split at hf
  next hroom =>
    split at hf
    next hr =>
      have hrh : h = sk.rootH := eq_of_beq hr
      cases p with
      | I =>
          cases hio : s.base.iopenCh with
          | none => rw [hio] at hf; cases hf
          | some o =>
              cases o with
              | query => rw [hio] at hf; cases hf
              | wire =>
                  rw [hio] at hf
                  injection hf with hs'
                  rw [← hs']
                  exact ⟨hroom, rfl, rfl, Or.inl ⟨hrh, rfl, rfl, rfl⟩⟩
      | R =>
          cases hro : s.base.ropenCh with
          | none => rw [hro] at hf; cases hf
          | some o =>
              cases o with
              | query => rw [hro] at hf; cases hf
              | res => rw [hro] at hf; cases hf
              | wire =>
                  rw [hro] at hf
                  injection hf with hs'
                  rw [← hs']
                  exact ⟨hroom, rfl, rfl,
                    Or.inr (Or.inl ⟨hrh, rfl, rfl, rfl⟩)⟩
    next hr =>
      split at hf
      next i hcm =>
          split at hf
          next hg =>
              simp only [Bool.and_eq_true, beq_iff_eq] at hg
              obtain ⟨hcon, hph⟩ := hg
              injection hf with hs'
              rw [← hs']
              exact ⟨hroom, rfl, rfl, Or.inr (Or.inr ⟨i,
                (List.contains_iff_mem ..).mp (by simpa using hcon),
                hph, hcm, rfl⟩)⟩
          next hg => cases hf
      next hcm => cases hf
  next hroom => cases hf

/-- Non-wire committals, read off a disabled fire: what the muxed
`walkFire` arm knows about the hand. -/
theorem not_wire_committed_of_fire_false {s : State}
    {pk : Party × Nat} (hnf : isWireFire s (.walkFire pk) = false) :
    ∀ i, (s.walk pk).committed ≠ some (Oblig.wire i) := by
  intro i heq
  rw [show isWireFire s (.walkFire pk)
      = (match (s.walk pk).committed with
         | some (Oblig.wire _) => true
         | _ => false) from rfl, heq] at hnf
  simp at hnf

end StreamingMirror.Mux
