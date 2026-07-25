/-
Channel-wiring facts for the O receive counts: which channel each
O count is observed on, and the frame lemma saying a walk update at
`pk` is invisible to every other channel's O count.

Mirrors Proofs/Wiring.lean's consumer-side lemmas over `recvdOfO`
(the producer side is order-blind — `sentOf` and its wiring lemmas are
consumed as-is by the O preservation files). Each proof is the base
proof with a `cases ord.walk pk` (or `ord.absorb`) dispatch: both
branches reduce to the two existing formulas, which read the same walk
fields, so the base reasoning applies verbatim in each.

Chain (ord, stage A): channel routing and frames; consumed by
Ord/Init and Ord/Preserve. Base mirror: Proofs/Wiring.lean. Map:
PROGRESS.md §13.
-/
import StreamingMirror.Proofs.Wiring
import StreamingMirror.Ord.Invariant

namespace StreamingMirror.Ord

open Model

variable {sk : Skel} {ord : OrdMap} {s : State} {pk : Party × Nat}

-- ================================= count/channel alignment (consumer)

/-- The O prologue-wire count of walk `pk` is the O consumer count of
`wireIn pk` (cf. `recvdOf_wireIn`). -/
theorem recvdOfO_wireIn (h : pk ∈ sk.walkKeys) :
    recvdOfO sk ord s (wireIn pk) = wkWireRecvdO sk ord s pk := by
  obtain ⟨p, k⟩ := pk
  rcases walkKeys_cases h with ⟨hp, hk1, hk2⟩ | ⟨hp, hk2⟩ <;> simp only at hp <;> subst hp
  · by_cases hr : k + 1 = sk.rootH
    · have hk : sk.rootH - 1 = k := by omega
      simp [recvdOfO, wireIn, Party.other, hr, hk]
    · simp [recvdOfO, wireIn, Party.other, hr]
  · have hr : k + 1 ≠ sk.rootH := by omega
    simp [recvdOfO, wireIn, Party.other, hr]

/-- `askedIn pk`'s O consumer count (cf. `recvdOf_askedIn`). -/
theorem recvdOfO_askedIn :
    recvdOfO sk ord s (askedIn pk) = wkAskedRecvdO sk ord s pk := rfl

-- ============================================== the setWalk flow frame

/-- The phantom channel (a private twin of Wiring.lean's, which is not
exported): no `allChans` member is `wire I 0` once the skeleton has
real height. -/
private theorem wire_I_zero_not_mem (hge : 2 ≤ sk.rootH) :
    Chan.wire Party.I 0 ∉ allChans sk := by
  intro hc
  simp [allChans, wireOut, askedIn, upperOut, lowerOut] at hc
  rcases hc with hq | h0
  · rcases walkKeys_cases hq with ⟨_, hb, _⟩ | ⟨hR, _⟩
    · exact absurd (hb : 1 ≤ 0) (by omega)
    · exact Party.noConfusion (hR : Party.I = Party.R)
  · omega

/-- Consumer-side O frame: a walk update at `pk` is invisible to
`recvdOfO` away from `pk`'s two input channels, for `allChans` members
(cf. `recvdOf_setWalk_frame`, including the phantom `wire I 0`
exclusion). -/
theorem recvdOfO_setWalk_frame (hwf : sk.wellFormed = true)
    (s : State) (pk : Party × Nat) (ws' : WalkSt)
    {c : Chan} (hc : c ∈ allChans sk)
    (h5 : c ≠ wireIn pk) (h6 : c ≠ askedIn pk) :
    recvdOfO sk ord (setWalk s pk ws') c = recvdOfO sk ord s c := by
  have hbase := recvdOf_setWalk_frame (sk := sk) hwf s pk ws' hc h5 h6
  have hge : 2 ≤ sk.rootH := (wf_rootH hwf).2
  cases c with
  | wire p h =>
      by_cases hr : h = sk.rootH
      · subst hr
        cases p with
        | I => simp [recvdOfO, setWalk]
        | R =>
            have hq : (Party.I, sk.rootH - 1) ≠ pk := by
              intro he
              apply h5
              rw [← he]
              show Chan.wire Party.R sk.rootH
                  = Chan.wire Party.R (sk.rootH - 1 + 1)
              have hn : sk.rootH - 1 + 1 = sk.rootH := by omega
              rw [hn]
            cases hord : ord.walk (Party.I, sk.rootH - 1) <;>
              simp [recvdOfO, wkWireRecvdO, hord, wkWireRecvd, wkAskedRecvd,
                setWalk_walk_ne s ws' hq]
      · by_cases hh : h = 0
        · subst hh
          cases p with
          | I => exact absurd hc (wire_I_zero_not_mem hge)
          | R =>
              cases hord : ord.absorb <;>
                simp [recvdOfO, hr, absorbWireRecvdO, hord, absorbWireRecvd,
                  absorbAskedRecvd, setWalk]
        · have hq : (p.other, h - 1) ≠ pk := by
            intro he
            apply h5
            rw [← he]
            show Chan.wire p h = Chan.wire p.other.other (h - 1 + 1)
            have hn : h - 1 + 1 = h := by omega
            cases p <;> simp [Party.other, hn]
          cases hord : ord.walk (p.other, h - 1) <;>
            simp [recvdOfO, hr, hh, wkWireRecvdO, hord, wkWireRecvd,
              wkAskedRecvd, setWalk_walk_ne s ws' hq]
  | asked p h =>
      have hq : (p, h) ≠ pk := by
        intro he; subst he; exact h6 rfl
      cases hord : ord.walk (p, h) <;>
        simp [recvdOfO, wkAskedRecvdO, hord, wkWireRecvd, wkAskedRecvd,
          setWalk_walk_ne s ws' hq]
  | leafRequests =>
      cases hord : ord.absorb <;>
        simp [recvdOfO, absorbAskedRecvdO, hord, absorbWireRecvd,
          absorbAskedRecvd, setWalk]
  | upper p h => exact hbase
  | lower p h => exact hbase
  | level p j => exact hbase
  | rootret => exact hbase
  | rootrets => exact hbase
  | rootres => exact hbase

/-- Receives frame for a walk update whose two O consumer counts at
`pk` are unchanged (cf. `recvdOf_setWalk_same`). -/
theorem recvdOfO_setWalk_same (hwf : sk.wellFormed = true)
    (s : State) (pk : Party × Nat) (ws' : WalkSt)
    (hmem : pk ∈ sk.walkKeys)
    (hWr : wkWireRecvdO sk ord (setWalk s pk ws') pk = wkWireRecvdO sk ord s pk)
    (hAr : wkAskedRecvdO sk ord (setWalk s pk ws') pk = wkAskedRecvdO sk ord s pk)
    {c : Chan} (hc : c ∈ allChans sk) :
    recvdOfO sk ord (setWalk s pk ws') c = recvdOfO sk ord s c := by
  by_cases h5 : c = wireIn pk
  · subst h5; rw [recvdOfO_wireIn hmem, recvdOfO_wireIn hmem]; exact hWr
  by_cases h6 : c = askedIn pk
  · subst h6; rw [recvdOfO_askedIn, recvdOfO_askedIn]; exact hAr
  · exact recvdOfO_setWalk_frame hwf s pk ws' hc h5 h6

-- =========================== non-walk state deltas are recvdOfO-framed

/-- Everything `recvdOfO` can read — the observation set of
`recvdOf_ext`, unchanged: both of a loop's formulas read only the
walk's `scope`/`phase` (and the absorber cursor/phase, fin/ropen
scalars), so the O counts see exactly what the base counts see. -/
theorem recvdOfO_ext {s s' : State}
    (hasm : ∀ pk, s'.asm pk = s.asm pk)
    (hsc : ∀ pk, (s'.walk pk).scope = (s.walk pk).scope)
    (hph : ∀ pk, (s'.walk pk).phase = (s.walk pk).phase)
    (hgotw : s'.ropenGotWire = s.ropenGotWire)
    (habsi : s'.absorbIdx = s.absorbIdx)
    (habsp : s'.absorbPhase = s.absorbPhase)
    (hifin : s'.ifin = s.ifin)
    (hrgot : s'.rfinGot = s.rfinGot)
    (hrres : s'.rfinGotRes = s.rfinGotRes)
    (c : Chan) :
    recvdOfO sk ord s' c = recvdOfO sk ord s c := by
  cases c with
  | wire p h =>
      cases hord : ord.walk (Party.I, sk.rootH - 1) <;>
      cases hord2 : ord.walk (p.other, h - 1) <;>
      cases horda : ord.absorb <;>
        simp [recvdOfO, wkWireRecvdO, hord, hord2, horda, wkWireRecvd,
          wkAskedRecvd, absorbWireRecvdO, absorbWireRecvd, absorbAskedRecvd,
          hsc, hph, hgotw, habsi, habsp]
  | asked p h =>
      cases hord : ord.walk (p, h) <;>
        simp [recvdOfO, wkAskedRecvdO, hord, wkWireRecvd, wkAskedRecvd,
          hsc, hph]
  | leafRequests =>
      cases horda : ord.absorb <;>
        simp [recvdOfO, absorbAskedRecvdO, horda, absorbWireRecvd,
          absorbAskedRecvd, habsi, habsp]
  | upper p h => simp [recvdOfO, recvdOf, asmResRecvd, hasm]
  | lower p h => simp [recvdOfO, recvdOf, asmResRecvd, hasm]
  | level p j => simp [recvdOfO, recvdOf, asmLevelRecvd, hasm]
  | rootret => simp [recvdOfO, recvdOf, hifin]
  | rootrets => simp [recvdOfO, recvdOf, hrgot]
  | rootres => simp [recvdOfO, recvdOf, hrres]

-- ===================== both-formula congruence (the reuse workhorse)

/-- Both base formulas unchanged ⟹ the O wire count unchanged, at any
assignment. Wherever a base preservation proof establishes the two
base receive counts invariant, the O counts follow — no per-assignment
reasoning at the consumption site. -/
theorem wkWireRecvdO_congr {s s' : State} (pk : Party × Nat)
    (hW : wkWireRecvd sk s' pk = wkWireRecvd sk s pk)
    (hA : wkAskedRecvd sk s' pk = wkAskedRecvd sk s pk) :
    wkWireRecvdO sk ord s' pk = wkWireRecvdO sk ord s pk := by
  cases hord : ord.walk pk <;> simp [wkWireRecvdO, hord, hW, hA]

/-- Both base formulas unchanged ⟹ the O query count unchanged. -/
theorem wkAskedRecvdO_congr {s s' : State} (pk : Party × Nat)
    (hW : wkWireRecvd sk s' pk = wkWireRecvd sk s pk)
    (hA : wkAskedRecvd sk s' pk = wkAskedRecvd sk s pk) :
    wkAskedRecvdO sk ord s' pk = wkAskedRecvdO sk ord s pk := by
  cases hord : ord.walk pk <;> simp [wkAskedRecvdO, hord, hW, hA]

/-- Both base absorber counts unchanged ⟹ the O wire count unchanged. -/
theorem absorbWireRecvdO_congr {s s' : State}
    (hW : absorbWireRecvd sk s' = absorbWireRecvd sk s)
    (hA : absorbAskedRecvd sk s' = absorbAskedRecvd sk s) :
    absorbWireRecvdO sk ord s' = absorbWireRecvdO sk ord s := by
  cases hord : ord.absorb <;> simp [absorbWireRecvdO, hord, hW, hA]

/-- Both base absorber counts unchanged ⟹ the O request count
unchanged. -/
theorem absorbAskedRecvdO_congr {s s' : State}
    (hW : absorbWireRecvd sk s' = absorbWireRecvd sk s)
    (hA : absorbAskedRecvd sk s' = absorbAskedRecvd sk s) :
    absorbAskedRecvdO sk ord s' = absorbAskedRecvdO sk ord s := by
  cases hord : ord.absorb <;> simp [absorbAskedRecvdO, hord, hW, hA]

/-- A committed-choice update is invisible to every O consumer count
(cf. `recvdOf_setWalk_committed`). -/
theorem recvdOfO_setWalk_committed (s : State)
    (pk : Party × Nat) (co : Option Oblig) (c : Chan) :
    recvdOfO sk ord (setWalk s pk { s.walk pk with committed := co }) c
      = recvdOfO sk ord s c := by
  apply recvdOfO_ext <;>
    first
      | rfl
      | exact fun _ => rfl
      | (intro pk'
         by_cases h : pk' = pk
         · subst h; simp
         · simp [h])

end StreamingMirror.Ord
