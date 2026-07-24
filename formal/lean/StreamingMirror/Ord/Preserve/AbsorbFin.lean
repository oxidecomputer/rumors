/-
O preservation for the absorber and the two finish processes. The
finish arms and `absorbSend` are base arms (`applyO`'s catch-all
delegates); their touched channels are root singletons whose O
consumer counts ARE the base counts, so those proofs transcribe
verbatim with the walk/absorber counts framed by the ord-dispatch.
The four ABSORB receive/close arms are order-dispatched: each branch
is one of the base file's two absorber shapes — a reply-first branch
re-derives its own base body over `recvdOfO`, a query-first branch
re-derives the PARTNER arm's base body (the phase 0→1 shape for the
first receive, the 1→2 shape for the second; closes at 3→4/4→5
per the flipped end-of-stream order) — with the rising O count routed
to the branch's channel and the partner channel's count pinned. The
absorber state is scalar (cursor and phase), so no per-key `setWalk`
plumbing appears: the close arms share one phase-bump core per phase
(every count is saturated at phase ≥ 3 in both orders).

Chain (ord, stage B): the absorber/finish preservation cases, consumed
by Ord/Preserve.lean. Base mirror: Proofs/Preserve/AbsorbFin.lean.
Map: PROGRESS.md §13.
-/
import StreamingMirror.Ord.Wiring

namespace StreamingMirror.Ord

open Model

variable {sk : Skel} {ax : AxMode} {ord : OrdMap} {s s' : State}

/-- Every O consumer count frames when the walk map, the asm map, the
top scalars, and BOTH base absorber counts are unchanged: the
absorber-granularity companion of `recvdOfO_ext` (which pins the
cursor and phase instead), for the arms that move the absorber past
saturation. -/
private theorem recvdOfO_counts_frame {s s' : State}
    (hwalk : s'.walk = s.walk) (hasm : s'.asm = s.asm)
    (hgotw : s'.ropenGotWire = s.ropenGotWire)
    (habsW : absorbWireRecvd sk s' = absorbWireRecvd sk s)
    (habsA : absorbAskedRecvd sk s' = absorbAskedRecvd sk s)
    (hifin : s'.ifin = s.ifin) (hrgot : s'.rfinGot = s.rfinGot)
    (hrres : s'.rfinGotRes = s.rfinGotRes)
    (c : Chan) : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
  cases c with
  | wire p h =>
      cases hordw : ord.walk (Party.I, sk.rootH - 1) <;>
      cases hordw2 : ord.walk (p.other, h - 1) <;>
      cases horda : ord.absorb <;>
        simp [recvdOfO, wkWireRecvdO, hordw, hordw2, wkWireRecvd,
          wkAskedRecvd, absorbWireRecvdO, horda, hwalk, hgotw, habsW, habsA]
  | asked p h =>
      cases hordw : ord.walk (p, h) <;>
        simp [recvdOfO, wkAskedRecvdO, hordw, wkWireRecvd, wkAskedRecvd,
          hwalk]
  | leafRequests =>
      cases horda : ord.absorb <;>
        simp [recvdOfO, absorbAskedRecvdO, horda, habsW, habsA]
  | upper p h => simp [recvdOfO, recvdOf, asmResRecvd, hasm]
  | lower p h => simp [recvdOfO, recvdOf, asmResRecvd, hasm]
  | level p j => simp [recvdOfO, recvdOf, asmLevelRecvd, hasm]
  | rootret => simp [recvdOfO, recvdOf, hifin]
  | rootrets => simp [recvdOfO, recvdOf, hrgot]
  | rootres => simp [recvdOfO, recvdOf, hrres]

-- ====================================================== the finish arms

/-- `finRet` under the assignment (a shared base arm): occupancy of
`rootret` drops by one exactly as its order-blind consumer bit flips
0→1 (cf. `preserve_finRet`). -/
theorem preserve_finRetO (_hwf : sk.wellFormed = true)
    (hstep : applyO sk ax ord .finRet s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hstep' : apply sk ax .finRet s = some s' := hstep
  simp only [apply] at hstep'
  split at hstep'
  case isFalse => simp at hstep'
  case isTrue hg =>
    simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_true_eq] at hg
    obtain ⟨hifin, hpos⟩ := hg
    injection hstep' with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · rw [← hs']; exact hi.top
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = bump s.chan Chan.rootret (-1) := by rw [← hs']
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']; cases c <;> rfl
      by_cases hne : c = Chan.rootret
      · subst hne
        have hrecv : recvdOfO sk ord s' Chan.rootret = 1 := by
          rw [← hs']; simp [recvdOfO, recvdOf, b2n]
        have hrecv0 : recvdOfO sk ord s Chan.rootret = 0 := by
          simp [recvdOfO, recvdOf, b2n, hifin]
        rw [hchan, hsent, hrecv, bump_neg_one]
        rw [hrecv0] at heq
        exact ⟨by omega, by omega⟩
      · have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
          rw [← hs']
          cases c <;> first | rfl | exact absurd rfl hne
        rw [hchan, hsent, hrecv, bump_ne _ _ hne]
        exact ⟨heq, hcap⟩

/-- `finRes` under the assignment (a shared base arm): occupancy of
`rootres` drops by one exactly as its order-blind consumer bit flips
0→1, and the `rfinGotRes || rfinGot == 0` conjunct re-establishes by
its left arm (cf. `preserve_finRes`). -/
theorem preserve_finResO (_hwf : sk.wellFormed = true)
    (hstep : applyO sk ax ord .finRes s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hstep' : apply sk ax .finRes s = some s' := hstep
  simp only [apply] at hstep'
  split at hstep'
  case isFalse => simp at hstep'
  case isTrue hg =>
    simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_true_eq] at hg
    obtain ⟨hres, hpos⟩ := hg
    injection hstep' with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
      rw [topLocalOk] at htop ⊢
      rw [show s'.rfinGotRes = true from by rw [← hs'],
        show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.iopenCh = s.iopenCh from by rw [← hs'],
        show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.ropenCh = s.ropenCh from by rw [← hs'],
        show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
        show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
        show s'.rfinGot = s.rfinGot from by rw [← hs']]
      simp only [Bool.and_eq_true] at htop ⊢
      obtain ⟨⟨hpre, _h11⟩, h12⟩ := htop
      exact ⟨⟨hpre, rfl⟩, h12⟩
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = bump s.chan Chan.rootres (-1) := by rw [← hs']
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']; cases c <;> rfl
      by_cases hne : c = Chan.rootres
      · subst hne
        have hrecv : recvdOfO sk ord s' Chan.rootres = 1 := by
          rw [← hs']; simp [recvdOfO, recvdOf, b2n]
        have hrecv0 : recvdOfO sk ord s Chan.rootres = 0 := by
          simp [recvdOfO, recvdOf, b2n, hres]
        rw [hchan, hsent, hrecv, bump_neg_one]
        rw [hrecv0] at heq
        exact ⟨by omega, by omega⟩
      · have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
          rw [← hs']
          cases c <;> first | rfl | exact absurd rfl hne
        rw [hchan, hsent, hrecv, bump_ne _ _ hne]
        exact ⟨heq, hcap⟩

/-- `finRets` under the assignment (a shared base arm): the `rootrets`
consumer count `rfinGot` rises with the drop in occupancy, within the
guard's strict bound (cf. `preserve_finRets`). -/
theorem preserve_finRetsO (_hwf : sk.wellFormed = true)
    (hstep : applyO sk ax ord .finRets s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hstep' : apply sk ax .finRets s = some s' := hstep
  simp only [apply] at hstep'
  split at hstep'
  case isFalse => simp at hstep'
  case isTrue hg =>
    simp only [Bool.and_eq_true, decide_eq_true_eq] at hg
    obtain ⟨⟨hres, hlt⟩, hpos⟩ := hg
    injection hstep' with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
      rw [topLocalOk] at htop ⊢
      rw [show s'.rfinGot = s.rfinGot + 1 from by rw [← hs'],
        show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.iopenCh = s.iopenCh from by rw [← hs'],
        show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.ropenCh = s.ropenCh from by rw [← hs'],
        show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
        show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
        show s'.rfinGotRes = s.rfinGotRes from by rw [← hs']]
      simp only [Bool.and_eq_true] at htop ⊢
      obtain ⟨⟨hpre, _h11⟩, _h12⟩ := htop
      refine ⟨⟨hpre, ?_⟩, ?_⟩
      · rw [hres]; rfl
      · simp only [decide_eq_true_eq]
        omega
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = bump s.chan Chan.rootrets (-1) := by rw [← hs']
      have hsent : sentOf sk s' c = sentOf sk s c := by
        rw [← hs']; cases c <;> rfl
      by_cases hne : c = Chan.rootrets
      · subst hne
        have hrecv : recvdOfO sk ord s' Chan.rootrets = s.rfinGot + 1 := by
          rw [← hs']; simp [recvdOfO, recvdOf]
        have hrecv0 : recvdOfO sk ord s Chan.rootrets = s.rfinGot := rfl
        rw [hchan, hsent, hrecv, bump_neg_one]
        rw [hrecv0] at heq
        exact ⟨by omega, by omega⟩
      · have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
          rw [← hs']
          cases c <;> first | rfl | exact absurd rfl hne
        rw [hchan, hsent, hrecv, bump_ne _ _ hne]
        exact ⟨heq, hcap⟩

-- ================================================== the absorber closes

/-- The absorber's first end-of-stream wait, either order, either
channel: phase 3 → 4 with nothing else moving. Both base counts are
saturated at phase ≥ 3, so every O count frames. -/
private theorem preserve_absorbClose3
    (hph : s.absorbPhase = 3)
    (hs' : s' = { s with absorbPhase := 4 })
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
  · rw [hs']; exact hi.wk pk hpk
  · rw [hs']; exact hi.asm pk hpk
  · have htop := hi.top
    rw [topLocalOk] at htop ⊢
    rw [show s'.absorbPhase = 4 from by rw [hs'],
      show s'.iopenWire = s.iopenWire from by rw [hs'],
      show s'.iopenQuery = s.iopenQuery from by rw [hs'],
      show s'.iopenCh = s.iopenCh from by rw [hs'],
      show s'.ropenGotWire = s.ropenGotWire from by rw [hs'],
      show s'.ropenWire = s.ropenWire from by rw [hs'],
      show s'.ropenRes = s.ropenRes from by rw [hs'],
      show s'.ropenQ = s.ropenQ from by rw [hs'],
      show s'.ropenCh = s.ropenCh from by rw [hs'],
      show s'.absorbIdx = s.absorbIdx from by rw [hs'],
      show s'.rfinGotRes = s.rfinGotRes from by rw [hs'],
      show s'.rfinGot = s.rfinGot from by rw [hs']]
    simp only [Bool.and_eq_true] at htop ⊢
    obtain ⟨⟨⟨⟨hpre, h9⟩, _h10⟩, h11⟩, h12⟩ := htop
    refine ⟨⟨⟨⟨hpre, ?_⟩, rfl⟩, h11⟩, h12⟩
    rw [hph] at h9
    simpa using h9
  · obtain ⟨heq, hcap⟩ := hi.flow c hc
    have hchan : s'.chan = s.chan := by rw [hs']
    have hsent : sentOf sk s' c = sentOf sk s c := by
      rw [hs']; cases c <;> rfl
    have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
      rw [hs']
      refine recvdOfO_counts_frame ?_ ?_ ?_ ?_ ?_ ?_ ?_ ?_ c
      · rfl
      · rfl
      · rfl
      · simp [absorbWireRecvd, hph]
      · simp [absorbAskedRecvd, hph]
      · rfl
      · rfl
      · rfl
    rw [hchan, hsent, hrecv]
    exact ⟨heq, hcap⟩

/-- The absorber's second end-of-stream wait: phase 4 → 5, the same
frame shape. -/
private theorem preserve_absorbClose4
    (hph : s.absorbPhase = 4)
    (hs' : s' = { s with absorbPhase := 5 })
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
  · rw [hs']; exact hi.wk pk hpk
  · rw [hs']; exact hi.asm pk hpk
  · have htop := hi.top
    rw [topLocalOk] at htop ⊢
    rw [show s'.absorbPhase = 5 from by rw [hs'],
      show s'.iopenWire = s.iopenWire from by rw [hs'],
      show s'.iopenQuery = s.iopenQuery from by rw [hs'],
      show s'.iopenCh = s.iopenCh from by rw [hs'],
      show s'.ropenGotWire = s.ropenGotWire from by rw [hs'],
      show s'.ropenWire = s.ropenWire from by rw [hs'],
      show s'.ropenRes = s.ropenRes from by rw [hs'],
      show s'.ropenQ = s.ropenQ from by rw [hs'],
      show s'.ropenCh = s.ropenCh from by rw [hs'],
      show s'.absorbIdx = s.absorbIdx from by rw [hs'],
      show s'.rfinGotRes = s.rfinGotRes from by rw [hs'],
      show s'.rfinGot = s.rfinGot from by rw [hs']]
    simp only [Bool.and_eq_true] at htop ⊢
    obtain ⟨⟨⟨⟨hpre, h9⟩, _h10⟩, h11⟩, h12⟩ := htop
    refine ⟨⟨⟨⟨hpre, ?_⟩, rfl⟩, h11⟩, h12⟩
    rw [hph] at h9
    simpa using h9
  · obtain ⟨heq, hcap⟩ := hi.flow c hc
    have hchan : s'.chan = s.chan := by rw [hs']
    have hsent : sentOf sk s' c = sentOf sk s c := by
      rw [hs']; cases c <;> rfl
    have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
      rw [hs']
      refine recvdOfO_counts_frame ?_ ?_ ?_ ?_ ?_ ?_ ?_ ?_ c
      · rfl
      · rfl
      · rfl
      · simp [absorbWireRecvd, hph]
      · simp [absorbAskedRecvd, hph]
      · rfl
      · rfl
      · rfl
    rw [hchan, hsent, hrecv]
    exact ⟨heq, hcap⟩

/-- `absorbCloseWire` under the assignment: phase 3 → 4 (reply-first)
or 4 → 5 (query-first); the cores cover both. -/
theorem preserve_absorbCloseWireO (_hwf : sk.wellFormed = true)
    (hstep : applyO sk ax ord .absorbCloseWire s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  cases hord : ord.absorb <;> simp only [applyO, hord] at hstep
  -- reply-first: phase 3 → 4
  · split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hg
      obtain ⟨⟨hph, _hprod⟩, _hzero⟩ := hg
      injection hstep with hs'
      exact preserve_absorbClose3 hph hs'.symm hi
  -- query-first: phase 4 → 5
  · split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hg
      obtain ⟨⟨hph, _hprod⟩, _hzero⟩ := hg
      injection hstep with hs'
      exact preserve_absorbClose4 hph hs'.symm hi

/-- `absorbCloseAsked` under the assignment: phase 4 → 5 (reply-first)
or 3 → 4 (query-first). -/
theorem preserve_absorbCloseAskedO (_hwf : sk.wellFormed = true)
    (hstep : applyO sk ax ord .absorbCloseAsked s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  cases hord : ord.absorb <;> simp only [applyO, hord] at hstep
  -- reply-first: phase 4 → 5
  · split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hg
      obtain ⟨⟨hph, _hprod⟩, _hzero⟩ := hg
      injection hstep with hs'
      exact preserve_absorbClose4 hph hs'.symm hi
  -- query-first: phase 3 → 4
  · split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hg
      obtain ⟨⟨hph, _hprod⟩, _hzero⟩ := hg
      injection hstep with hs'
      exact preserve_absorbClose3 hph hs'.symm hi

-- ================================================ the absorber receives

/-- `absorbRecvWire` under the assignment: the first receive when
reply-first (phase 0 → 1), the second when query-first (phase 1 → 2).
Either way occupancy on `Chan.wire Party.R 0` drops by one exactly as
its O consumer count — the branch's SELECTED base formula — rises from
`absorbIdx` to `absorbIdx + 1`, and the request channel's count is
pinned by the same phase move. -/
theorem preserve_absorbRecvWireO (hwf : sk.wellFormed = true)
    (hstep : applyO sk ax ord .absorbRecvWire s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hrH : (0 == sk.rootH) = false := by
    have h2 := (wf_rootH hwf).2
    have : (0 : Nat) ≠ sk.rootH := by omega
    simp [this]
  cases hord : ord.absorb <;> simp only [applyO, hord] at hstep
  -- reply-first: phase 0 → 1
  · split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
      obtain ⟨hph, hpos⟩ := hg
      injection hstep with hs'
      refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
      · rw [← hs']; exact hi.wk pk hpk
      · rw [← hs']; exact hi.asm pk hpk
      · have htop := hi.top
        rw [topLocalOk] at htop ⊢
        rw [show s'.absorbPhase = 1 from by rw [← hs'],
          show s'.iopenWire = s.iopenWire from by rw [← hs'],
          show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
          show s'.iopenCh = s.iopenCh from by rw [← hs'],
          show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
          show s'.ropenWire = s.ropenWire from by rw [← hs'],
          show s'.ropenRes = s.ropenRes from by rw [← hs'],
          show s'.ropenQ = s.ropenQ from by rw [← hs'],
          show s'.ropenCh = s.ropenCh from by rw [← hs'],
          show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
          show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
          show s'.rfinGot = s.rfinGot from by rw [← hs']]
        simp only [Bool.and_eq_true] at htop ⊢
        obtain ⟨⟨⟨⟨hpre, h9⟩, _h10⟩, h11⟩, h12⟩ := htop
        refine ⟨⟨⟨⟨hpre, ?_⟩, rfl⟩, h11⟩, h12⟩
        rw [hph] at h9
        simpa using h9
      · obtain ⟨heq, hcap⟩ := hi.flow c hc
        have hchan : s'.chan = bump s.chan (Chan.wire Party.R 0) (-1) := by
          rw [← hs']
        have hsent : sentOf sk s' c = sentOf sk s c := by
          rw [← hs']; cases c <;> rfl
        by_cases hne : c = Chan.wire Party.R 0
        · subst hne
          have hrecv : recvdOfO sk ord s' (Chan.wire Party.R 0)
              = s.absorbIdx + 1 := by
            rw [← hs']
            simp [recvdOfO, hrH, absorbWireRecvdO, hord, absorbWireRecvd]
          have hrecv0 : recvdOfO sk ord s (Chan.wire Party.R 0)
              = s.absorbIdx := by
            simp [recvdOfO, hrH, absorbWireRecvdO, hord, absorbWireRecvd, hph]
          rw [hchan, hsent, hrecv, bump_neg_one]
          rw [hrecv0] at heq
          exact ⟨by omega, by omega⟩
        · have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
            rw [← hs']
            cases c with
            | wire p h =>
                by_cases hz : (p == Party.R && h == 0) = true
                · exfalso
                  simp only [Bool.and_eq_true, beq_iff_eq] at hz
                  exact hne (by rw [hz.1, hz.2])
                · cases hordw : ord.walk (Party.I, sk.rootH - 1) <;>
                  cases hordw2 : ord.walk (p.other, h - 1) <;>
                    simp [recvdOfO, hz, wkWireRecvdO, hordw, hordw2,
                      wkWireRecvd, wkAskedRecvd]
            | leafRequests =>
                simp [recvdOfO, absorbAskedRecvdO, hord, absorbAskedRecvd,
                  hph]
            | _ => rfl
          rw [hchan, hsent, hrecv, bump_ne _ _ hne]
          exact ⟨heq, hcap⟩
  -- query-first: phase 1 → 2 (the base second-receive shape)
  · split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
      obtain ⟨hph, hpos⟩ := hg
      injection hstep with hs'
      refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
      · rw [← hs']; exact hi.wk pk hpk
      · rw [← hs']; exact hi.asm pk hpk
      · have htop := hi.top
        rw [topLocalOk] at htop ⊢
        rw [show s'.absorbPhase = 2 from by rw [← hs'],
          show s'.iopenWire = s.iopenWire from by rw [← hs'],
          show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
          show s'.iopenCh = s.iopenCh from by rw [← hs'],
          show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
          show s'.ropenWire = s.ropenWire from by rw [← hs'],
          show s'.ropenRes = s.ropenRes from by rw [← hs'],
          show s'.ropenQ = s.ropenQ from by rw [← hs'],
          show s'.ropenCh = s.ropenCh from by rw [← hs'],
          show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
          show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
          show s'.rfinGot = s.rfinGot from by rw [← hs']]
        simp only [Bool.and_eq_true] at htop ⊢
        obtain ⟨⟨⟨⟨hpre, h9⟩, _h10⟩, h11⟩, h12⟩ := htop
        refine ⟨⟨⟨⟨hpre, ?_⟩, rfl⟩, h11⟩, h12⟩
        rw [hph] at h9
        simpa using h9
      · obtain ⟨heq, hcap⟩ := hi.flow c hc
        have hchan : s'.chan = bump s.chan (Chan.wire Party.R 0) (-1) := by
          rw [← hs']
        have hsent : sentOf sk s' c = sentOf sk s c := by
          rw [← hs']; cases c <;> rfl
        by_cases hne : c = Chan.wire Party.R 0
        · subst hne
          have hrecv : recvdOfO sk ord s' (Chan.wire Party.R 0)
              = s.absorbIdx + 1 := by
            rw [← hs']
            simp [recvdOfO, hrH, absorbWireRecvdO, hord, absorbAskedRecvd]
          have hrecv0 : recvdOfO sk ord s (Chan.wire Party.R 0)
              = s.absorbIdx := by
            simp [recvdOfO, hrH, absorbWireRecvdO, hord, absorbAskedRecvd,
              hph]
          rw [hchan, hsent, hrecv, bump_neg_one]
          rw [hrecv0] at heq
          exact ⟨by omega, by omega⟩
        · have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
            rw [← hs']
            cases c with
            | wire p h =>
                by_cases hz : (p == Party.R && h == 0) = true
                · exfalso
                  simp only [Bool.and_eq_true, beq_iff_eq] at hz
                  exact hne (by rw [hz.1, hz.2])
                · cases hordw : ord.walk (Party.I, sk.rootH - 1) <;>
                  cases hordw2 : ord.walk (p.other, h - 1) <;>
                    simp [recvdOfO, hz, wkWireRecvdO, hordw, hordw2,
                      wkWireRecvd, wkAskedRecvd]
            | leafRequests =>
                simp [recvdOfO, absorbAskedRecvdO, hord, absorbWireRecvd,
                  hph]
            | _ => rfl
          rw [hchan, hsent, hrecv, bump_ne _ _ hne]
          exact ⟨heq, hcap⟩

/-- `absorbRecvAsked` under the assignment: the second receive when
reply-first (phase 1 → 2), the first when query-first (phase 0 → 1).
Either way occupancy on `Chan.leafRequests` drops by one exactly as
its O consumer count rises from `absorbIdx` to `absorbIdx + 1`, and
the wire channel's count is pinned by the same phase move. -/
theorem preserve_absorbRecvAskedO (_hwf : sk.wellFormed = true)
    (hstep : applyO sk ax ord .absorbRecvAsked s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  cases hord : ord.absorb <;> simp only [applyO, hord] at hstep
  -- reply-first: phase 1 → 2
  · split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
      obtain ⟨hph, hpos⟩ := hg
      injection hstep with hs'
      refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
      · rw [← hs']; exact hi.wk pk hpk
      · rw [← hs']; exact hi.asm pk hpk
      · have htop := hi.top
        rw [topLocalOk] at htop ⊢
        rw [show s'.absorbPhase = 2 from by rw [← hs'],
          show s'.iopenWire = s.iopenWire from by rw [← hs'],
          show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
          show s'.iopenCh = s.iopenCh from by rw [← hs'],
          show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
          show s'.ropenWire = s.ropenWire from by rw [← hs'],
          show s'.ropenRes = s.ropenRes from by rw [← hs'],
          show s'.ropenQ = s.ropenQ from by rw [← hs'],
          show s'.ropenCh = s.ropenCh from by rw [← hs'],
          show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
          show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
          show s'.rfinGot = s.rfinGot from by rw [← hs']]
        simp only [Bool.and_eq_true] at htop ⊢
        obtain ⟨⟨⟨⟨hpre, h9⟩, _h10⟩, h11⟩, h12⟩ := htop
        refine ⟨⟨⟨⟨hpre, ?_⟩, rfl⟩, h11⟩, h12⟩
        rw [hph] at h9
        simpa using h9
      · obtain ⟨heq, hcap⟩ := hi.flow c hc
        have hchan : s'.chan = bump s.chan Chan.leafRequests (-1) := by
          rw [← hs']
        have hsent : sentOf sk s' c = sentOf sk s c := by
          rw [← hs']; cases c <;> rfl
        by_cases hne : c = Chan.leafRequests
        · subst hne
          have hrecv : recvdOfO sk ord s' Chan.leafRequests
              = s.absorbIdx + 1 := by
            rw [← hs']
            simp [recvdOfO, absorbAskedRecvdO, hord, absorbAskedRecvd]
          have hrecv0 : recvdOfO sk ord s Chan.leafRequests
              = s.absorbIdx := by
            simp [recvdOfO, absorbAskedRecvdO, hord, absorbAskedRecvd, hph]
          rw [hchan, hsent, hrecv, bump_neg_one]
          rw [hrecv0] at heq
          exact ⟨by omega, by omega⟩
        · have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
            rw [← hs']
            cases c with
            | wire p h =>
                cases hordw : ord.walk (Party.I, sk.rootH - 1) <;>
                cases hordw2 : ord.walk (p.other, h - 1) <;>
                  simp [recvdOfO, wkWireRecvdO, hordw, hordw2, wkWireRecvd,
                    wkAskedRecvd, absorbWireRecvdO, hord, absorbWireRecvd,
                    hph]
            | leafRequests => exact absurd rfl hne
            | _ => rfl
          rw [hchan, hsent, hrecv, bump_ne _ _ hne]
          exact ⟨heq, hcap⟩
  -- query-first: phase 0 → 1 (the base first-receive shape)
  · split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
      obtain ⟨hph, hpos⟩ := hg
      injection hstep with hs'
      refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
      · rw [← hs']; exact hi.wk pk hpk
      · rw [← hs']; exact hi.asm pk hpk
      · have htop := hi.top
        rw [topLocalOk] at htop ⊢
        rw [show s'.absorbPhase = 1 from by rw [← hs'],
          show s'.iopenWire = s.iopenWire from by rw [← hs'],
          show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
          show s'.iopenCh = s.iopenCh from by rw [← hs'],
          show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
          show s'.ropenWire = s.ropenWire from by rw [← hs'],
          show s'.ropenRes = s.ropenRes from by rw [← hs'],
          show s'.ropenQ = s.ropenQ from by rw [← hs'],
          show s'.ropenCh = s.ropenCh from by rw [← hs'],
          show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
          show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
          show s'.rfinGot = s.rfinGot from by rw [← hs']]
        simp only [Bool.and_eq_true] at htop ⊢
        obtain ⟨⟨⟨⟨hpre, h9⟩, _h10⟩, h11⟩, h12⟩ := htop
        refine ⟨⟨⟨⟨hpre, ?_⟩, rfl⟩, h11⟩, h12⟩
        rw [hph] at h9
        simpa using h9
      · obtain ⟨heq, hcap⟩ := hi.flow c hc
        have hchan : s'.chan = bump s.chan Chan.leafRequests (-1) := by
          rw [← hs']
        have hsent : sentOf sk s' c = sentOf sk s c := by
          rw [← hs']; cases c <;> rfl
        by_cases hne : c = Chan.leafRequests
        · subst hne
          have hrecv : recvdOfO sk ord s' Chan.leafRequests
              = s.absorbIdx + 1 := by
            rw [← hs']
            simp [recvdOfO, absorbAskedRecvdO, hord, absorbWireRecvd]
          have hrecv0 : recvdOfO sk ord s Chan.leafRequests
              = s.absorbIdx := by
            simp [recvdOfO, absorbAskedRecvdO, hord, absorbWireRecvd, hph]
          rw [hchan, hsent, hrecv, bump_neg_one]
          rw [hrecv0] at heq
          exact ⟨by omega, by omega⟩
        · have hrecv : recvdOfO sk ord s' c = recvdOfO sk ord s c := by
            rw [← hs']
            cases c with
            | wire p h =>
                cases hordw : ord.walk (Party.I, sk.rootH - 1) <;>
                cases hordw2 : ord.walk (p.other, h - 1) <;>
                  simp [recvdOfO, wkWireRecvdO, hordw, hordw2, wkWireRecvd,
                    wkAskedRecvd, absorbWireRecvdO, hord, absorbAskedRecvd,
                    hph]
            | leafRequests => exact absurd rfl hne
            | _ => rfl
          rw [hchan, hsent, hrecv, bump_ne _ _ hne]
          exact ⟨heq, hcap⟩

-- ==================================================== the absorber send

/-- `absorbSend` under the assignment (a shared base arm): the
`Chan.level Party.I 0` producer count is the cursor, which rises with
the send; both base absorber counts are constant across the move, so
every O consumer count frames at any assignment (cf.
`preserve_absorbSend`). -/
theorem preserve_absorbSendO (_hwf : sk.wellFormed = true)
    (hstep : applyO sk ax ord .absorbSend s = some s')
    (hi : InvPO sk ax ord s) : InvPO sk ax ord s' := by
  have hstep' : apply sk ax .absorbSend s = some s' := hstep
  simp only [apply] at hstep'
  split at hstep'
  case isFalse => simp at hstep'
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨hph, hlt⟩ := hg
    have hidx : s.absorbIdx < sk.totalLeafReqs := by
      have htop := hi.top
      rw [topLocalOk] at htop
      simp only [Bool.and_eq_true] at htop
      have h9 := htop.1.1.1.2
      rw [hph] at h9
      simpa using h9
    injection hstep' with hs'
    -- the absorb consumer counts are constant across the send
    have habsW : absorbWireRecvd sk s' = absorbWireRecvd sk s := by
      rw [← hs']
      by_cases hlt2 : s.absorbIdx + 1 < sk.totalLeafReqs
      · simp [absorbWireRecvd, hph, hlt2]
      · simp [absorbWireRecvd, hph, hlt2]
        omega
    have habsA : absorbAskedRecvd sk s' = absorbAskedRecvd sk s := by
      rw [← hs']
      by_cases hlt2 : s.absorbIdx + 1 < sk.totalLeafReqs
      · simp [absorbAskedRecvd, hph, hlt2]
      · simp [absorbAskedRecvd, hph, hlt2]
        omega
    -- untouched components, at observation granularity
    have hwalk : s'.walk = s.walk := by rw [← hs']
    have hasm : s'.asm = s.asm := by rw [← hs']
    have hgw : s'.ropenGotWire = s.ropenGotWire := by rw [← hs']
    have hifin : s'.ifin = s.ifin := by rw [← hs']
    have hrg : s'.rfinGot = s.rfinGot := by rw [← hs']
    have hrgr : s'.rfinGotRes = s.rfinGotRes := by rw [← hs']
    have hio1 : s'.iopenWire = s.iopenWire := by rw [← hs']
    have hio2 : s'.iopenQuery = s.iopenQuery := by rw [← hs']
    have hro1 : s'.ropenWire = s.ropenWire := by rw [← hs']
    have hro2 : s'.ropenRes = s.ropenRes := by rw [← hs']
    have hro3 : s'.ropenQ = s.ropenQ := by rw [← hs']
    have hrecv : ∀ c, recvdOfO sk ord s' c = recvdOfO sk ord s c :=
      fun c => recvdOfO_counts_frame hwalk hasm hgw habsW habsA
        hifin hrg hrgr c
    have hsent : ∀ c, c ≠ Chan.level Party.I 0 →
        sentOf sk s' c = sentOf sk s c := by
      intro c hnc
      cases c with
      | level p j =>
          cases p with
          | I =>
              have hj : (j == 0) = false := by
                have : j ≠ 0 := fun h0 => hnc (by rw [h0])
                simp [this]
              simp [sentOf, asmOutSent, hj, hasm]
          | R => simp [sentOf, asmOutSent, hasm]
      | wire p h =>
          simp [sentOf, wkWireSent, wkWireCount, hwalk, hio1, hro1]
      | asked p h =>
          simp [sentOf, wkQSentTot, wkQSum, hwalk, hio2, hro3]
      | leafRequests => simp [sentOf, wkQSentTot, wkQSum, hwalk]
      | upper p h => simp [sentOf, wkParentSent, hwalk]
      | lower p h => simp [sentOf, wkResSent, wkResCount, hwalk]
      | rootret => simp [sentOf, asmOutSent, hasm]
      | rootrets => simp [sentOf, asmOutSent, hasm]
      | rootres => simp [sentOf, hro2]
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_, fun c hc => ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
      rw [topLocalOk] at htop ⊢
      rw [show s'.absorbIdx = s.absorbIdx + 1 from by rw [← hs'],
        show s'.absorbPhase
            = (if s.absorbIdx + 1 < sk.totalLeafReqs then 0 else 3) from by
          rw [← hs'],
        show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.iopenCh = s.iopenCh from by rw [← hs'],
        show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.ropenCh = s.ropenCh from by rw [← hs'],
        show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
        show s'.rfinGot = s.rfinGot from by rw [← hs']]
      simp only [Bool.and_eq_true] at htop ⊢
      obtain ⟨⟨⟨⟨hpre, _h9⟩, _h10⟩, h11⟩, h12⟩ := htop
      refine ⟨⟨⟨⟨hpre, ?_⟩, ?_⟩, h11⟩, h12⟩
      · by_cases hlt2 : s.absorbIdx + 1 < sk.totalLeafReqs
        · simp [hlt2]
        · simp [hlt2]
          omega
      · have h5 : (if s.absorbIdx + 1 < sk.totalLeafReqs then 0 else 3)
            ≤ 5 := by
          split <;> omega
        simpa using h5
    · obtain ⟨heq, hcap⟩ := hi.flow c hc
      have hchan : s'.chan = bump s.chan (Chan.level Party.I 0) 1 := by
        rw [← hs']
      by_cases hne : c = Chan.level Party.I 0
      · subst hne
        have hs1 : sentOf sk s' (Chan.level Party.I 0)
            = s.absorbIdx + 1 := by
          rw [← hs']; simp [sentOf]
        have hs0 : sentOf sk s (Chan.level Party.I 0) = s.absorbIdx := by
          simp [sentOf]
        rw [hchan, bump_one, hrecv, hs1]
        rw [hs0] at heq
        exact ⟨by omega, by omega⟩
      · rw [hchan, bump_ne _ _ hne, hsent c hne, hrecv c]
        exact ⟨heq, hcap⟩

end StreamingMirror.Ord
