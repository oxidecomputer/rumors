/-
`MuxInv` preservation (MUX-ADJUDICATION.md §4, the stage-F obligation;
stage-3 track E): the ground facts of Chase/Ground.lean hold at every
reachable state of the muxed system, for EVERY strategy pair — the
hypothesis both stage-3 theorems (T5 here, T4 next) discharge through
`muxInv_reachable`.

# The decomposition

The muxed step relation has three kinds of arms, each with its own
preservation argument:

- **base arms** — the local invariant's preservation is the per-arm
  extraction from the monolithic base proofs
  (Preserve/{TopFin,WalkAsm,Fire}.lean: the wk/asm/top bullets, minus
  the flow bullets a muxed state cannot satisfy), and the counting
  fields ride the per-arm deltas: no base action touches a wire send,
  wire slots only drain, and internal channels keep the unmuxed
  conservation law arm by arm.
- **push** — the one arm that fires a wire send: the hand's base
  effect (opener flag or walk ledger advance, `firePush_inv`) raises
  exactly one `sentOf` as the history gains exactly one flush receipt,
  and the pipe append is the pushed suffix growing by its own new
  frame.
- **deliver** — the one arm that moves a frame: the head leaves the
  pipe as the delivery count rises and the slot fills, which is
  `hist_del`/`hist_pipe` shifting their split point by one.

The dispatch (`muxInv_preserved`) is strategy-parametric: nothing in
preservation consults σ — only WHICH pushes happen depends on the
strategy, never what a push does to the ground facts.
-/
import StreamingMirror.Mux.Proofs.Preserve.Glue
import StreamingMirror.Mux.Proofs.Preserve.TopFin
import StreamingMirror.Mux.Proofs.Preserve.WalkAsm
import StreamingMirror.Mux.Proofs.Preserve.Fire

namespace StreamingMirror.Mux

open Model

variable {sk : Skel}

-- ================================================== small party glue

/-- Two-point party algebra: every party is `p` or `p.other`. -/
theorem party_eq_or_other (q p : Party) : q = p ∨ q = p.other := by
  cases q <;> cases p <;> simp [Party.other]

/-- Deliveries never outrun pushes: the FIFO split point is in range. -/
theorem delTotal_le_pushes {s : MState} (hm : MuxInv sk s) (p : Party) :
    delTotal (s.hist p.other) ≤ (pushHeights (s.hist p)).length := by
  have hlen := congrArg List.length (hm.hist_del p)
  rw [List.length_take] at hlen
  unfold delTotal at hlen ⊢
  omega

/-- An internal channel is never a wire tag. -/
theorem ne_wire_of_not_isWire {c : Chan} (hw : isWire c = false)
    (p : Party) (h : Nat) : c ≠ Chan.wire p h := by
  intro heq
  rw [heq] at hw
  cases hw

/-- The opening wires are flow channels. -/
theorem wire_rootH_mem (sk : Skel) (p : Party) :
    Chan.wire p sk.rootH ∈ allChans sk := by
  unfold allChans
  cases p <;> simp

/-- A walk key's outgoing wire is a flow channel. -/
theorem wireOut_mem (sk : Skel) {pk : Party × Nat}
    (hpk : pk ∈ sk.walkKeys) : Chan.wire pk.1 pk.2 ∈ allChans sk := by
  unfold allChans
  refine List.mem_append.mpr (.inl (List.mem_append.mpr (.inl ?_)))
  refine List.mem_flatMap.mpr ⟨pk, hpk, ?_⟩
  unfold wireOut
  simp

-- ============================================ base arms: the dispatch

/-- The local invariant survives every muxed base arm: the per-arm
extractions, dispatched (wire fires are disabled, and the fire arm
sees only non-wire committals). -/
theorem preserveL_base (hwf : sk.wellFormed = true) {ax : AxMode}
    {a : Action} {s s' : State} (hnf : isWireFire s a = false)
    (hstep : Model.apply sk ax a s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  cases a with
  | iopenChoose o => exact preserveL_iopenChoose hwf o hstep hi
  | iopenFire => exact preserveL_iopenFire hwf hstep hi
  | ropenRecv => exact preserveL_ropenRecv hwf hstep hi
  | ropenChoose o => exact preserveL_ropenChoose hwf o hstep hi
  | ropenFire => exact preserveL_ropenFire hwf hstep hi
  | walkRecvWire pk => exact preserveL_walkRecvWire hwf pk hstep hi
  | walkRecvAsked pk => exact preserveL_walkRecvAsked hwf pk hstep hi
  | walkCommit pk o => exact preserveL_walkCommit hwf pk o hstep hi
  | walkFire pk => exact preserveL_walkFire hwf pk hstep hi
  | walkCloseWire pk => exact preserveL_walkCloseWire hwf pk hstep hi
  | walkCloseAsked pk => exact preserveL_walkCloseAsked hwf pk hstep hi
  | asmRecvRes pk => exact preserveL_asmRecvRes hwf pk hstep hi
  | asmRecvLevel pk => exact preserveL_asmRecvLevel hwf pk hstep hi
  | asmSend pk => exact preserveL_asmSend hwf pk hstep hi
  | asmClose pk => exact preserveL_asmClose hwf pk hstep hi
  | absorbRecvWire => exact preserveL_absorbRecvWire hwf hstep hi
  | absorbRecvAsked => exact preserveL_absorbRecvAsked hwf hstep hi
  | absorbSend => exact preserveL_absorbSend hwf hstep hi
  | absorbCloseWire => exact preserveL_absorbCloseWire hwf hstep hi
  | absorbCloseAsked => exact preserveL_absorbCloseAsked hwf hstep hi
  | finRet => exact preserveL_finRet hwf hstep hi
  | finRes => exact preserveL_finRes hwf hstep hi
  | finRets => exact preserveL_finRets hwf hstep hi

/-- `iopenFire`'s hand is not the opening wire when the fire is not a
wire fire. -/
theorem iopenCh_ne_wire_of_fire_false {s : State}
    (hnf : isWireFire s .iopenFire = false) :
    s.iopenCh ≠ some IOblig.wire := by
  intro heq
  have : isWireFire s .iopenFire = true := by
    show (s.iopenCh == some IOblig.wire) = true
    rw [heq]
    rfl
  rw [this] at hnf
  cases hnf

/-- `ropenFire`'s hand is not the opening wire when the fire is not a
wire fire. -/
theorem ropenCh_ne_wire_of_fire_false {s : State}
    (hnf : isWireFire s .ropenFire = false) :
    s.ropenCh ≠ some ROblig.wire := by
  intro heq
  have : isWireFire s .ropenFire = true := by
    show (s.ropenCh == some ROblig.wire) = true
    rw [heq]
    rfl
  rw [this] at hnf
  cases hnf

/-- Weaken an unguarded wire-sum delta to the guarded form the
`MuxInv` induction consumes (only `walkRecvWire`, whose phantom corner
genuinely fails, produces the guarded form natively). -/
private theorem guard2 {sk : Skel} {s s' : State}
    (d : ((∀ p h, sentOf sk s' (Chan.wire p h)
          = sentOf sk s (Chan.wire p h))
        ∧ (∀ p h, Chan.wire p h ∈ allChans sk →
            s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
            = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
        ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
        ∧ (∀ c ∈ allChans sk, isWire c = false →
            s.chan c + recvdOf sk s c = sentOf sk s c →
            s'.chan c + recvdOf sk s' c = sentOf sk s' c)
        ∧ (∀ c ∈ allChans sk, isWire c = false →
            s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c))
      ∨ ((∀ p h, sentOf sk s' (Chan.wire p h)
          = sentOf sk s (Chan.wire p h))
        ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
            = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
        ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
        ∧ (∀ c ∈ allChans sk, isWire c = false →
            s.chan c + recvdOf sk s c = sentOf sk s c →
            s'.chan c + recvdOf sk s' c = sentOf sk s' c)
        ∧ (∀ c ∈ allChans sk, isWire c = false →
            s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c))) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, Chan.wire p h ∈ allChans sk →
        s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  rcases d with ⟨d1, d2, d3, d4, d5⟩ | ⟨d1, d2, d3, d4, d5⟩
  · exact ⟨d1, d2, d3, d4, d5⟩
  · exact ⟨d1, fun p h _ => d2 p h, d3, d4, d5⟩

/-- The counting deltas of every muxed base arm, dispatched: wire sends
frame, wire slot-plus-consumption sums frame, wire slots only drain,
and internal channels keep flow and capacity arm by arm. -/
theorem delta_base (hwf : sk.wellFormed = true) {ax : AxMode}
    {a : Action} {s s' : State} (hnf : isWireFire s a = false)
    (hstep : Model.apply sk ax a s = some s')
    (hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, Chan.wire p h ∈ allChans sk →
        s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  refine guard2 ?_
  cases a with
  | iopenChoose o => exact Or.inr (delta_iopenChoose hwf o hstep hi)
  | iopenFire =>
      exact Or.inr (delta_iopenFire hwf
        (iopenCh_ne_wire_of_fire_false hnf) hstep hi)
  | ropenRecv => exact Or.inr (delta_ropenRecv hwf hstep hi)
  | ropenChoose o => exact Or.inr (delta_ropenChoose hwf o hstep hi)
  | ropenFire =>
      exact Or.inr (delta_ropenFire hwf
        (ropenCh_ne_wire_of_fire_false hnf) hstep hi)
  | walkRecvWire pk => exact Or.inl (delta_walkRecvWire hwf pk hstep hi)
  | walkRecvAsked pk => exact Or.inr (delta_walkRecvAsked hwf pk hstep hi)
  | walkCommit pk o => exact Or.inr (delta_walkCommit hwf pk o hstep hi)
  | walkFire pk =>
      exact Or.inr (delta_walkFire hwf pk
        (not_wire_committed_of_fire_false hnf) hstep hi)
  | walkCloseWire pk => exact Or.inr (delta_walkCloseWire hwf pk hstep hi)
  | walkCloseAsked pk => exact Or.inr (delta_walkCloseAsked hwf pk hstep hi)
  | asmRecvRes pk => exact Or.inr (delta_asmRecvRes hwf pk hstep hi)
  | asmRecvLevel pk => exact Or.inr (delta_asmRecvLevel hwf pk hstep hi)
  | asmSend pk => exact Or.inr (delta_asmSend hwf pk hstep hi)
  | asmClose pk => exact Or.inr (delta_asmClose hwf pk hstep hi)
  | absorbRecvWire => exact Or.inr (delta_absorbRecvWire hwf hstep hi)
  | absorbRecvAsked => exact Or.inr (delta_absorbRecvAsked hwf hstep hi)
  | absorbSend => exact Or.inr (delta_absorbSend hwf hstep hi)
  | absorbCloseWire => exact Or.inr (delta_absorbCloseWire hwf hstep hi)
  | absorbCloseAsked => exact Or.inr (delta_absorbCloseAsked hwf hstep hi)
  | finRet => exact Or.inr (delta_finRet hwf hstep hi)
  | finRes => exact Or.inr (delta_finRes hwf hstep hi)
  | finRets => exact Or.inr (delta_finRets hwf hstep hi)

/-- `MuxInv` survives every muxed base arm. -/
theorem muxInv_base (hwf : sk.wellFormed = true) {a : Action}
    {s s' : MState} (hstep : applyBase sk .impl a s = some s')
    (hm : MuxInv sk s) : MuxInv sk s' := by
  obtain ⟨hbase, hnf, hpipe, hhist⟩ := applyBase_inv hstep
  obtain ⟨hws, hwsum, hwmono, hint, hslot⟩ :=
    delta_base hwf hnf hbase hm.invl
  have hists : ∀ q, s'.hist q = s.hist q
      ∨ s'.hist q = s.hist q ++ [.act a] := by
    intro q
    rw [hhist]
    rcases recordObs_cases s.hist (actionParty a) (.act a) q with hc |
      ⟨-, hc⟩
    · exact Or.inl hc
    · exact Or.inr hc
  have hpushH : ∀ q, pushHeights (s'.hist q) = pushHeights (s.hist q) := by
    intro q
    rcases hists q with hc | hc <;> rw [hc]
    rw [pushHeights_append_act]
  have hdelH : ∀ q, delHeights (s'.hist q) = delHeights (s.hist q) := by
    intro q
    rcases hists q with hc | hc <;> rw [hc]
    rw [delHeights_append_act]
  have hdelT : ∀ q, delTotal (s'.hist q) = delTotal (s.hist q) := by
    intro q
    unfold delTotal
    rw [hdelH q]
  refine ⟨preserveL_base hwf hnf hbase hm.invl, ?_, ?_, ?_, ?_, ?_, ?_,
    ?_⟩
  · intro c hc
    cases hw : isWire c with
    | true =>
        obtain ⟨p, hh, rfl⟩ := isWire_eq hw
        have h1 := hwmono p hh
        have h2 := hm.slot _ hc
        omega
    | false => exact hslot c hc hw (hm.slot c hc)
  · intro c hc hwv
    exact hint c hc hwv (hm.flow_int c hc hwv)
  · intro p hh
    unfold pushedCount
    rw [hpushH p, hws p hh]
    exact hm.pushed_eq p hh
  · intro p
    rw [hdelH p.other, hpushH p, hdelT p.other]
    exact hm.hist_del p
  · intro p
    rw [hpipe, hpushH p, hdelT p.other]
    exact hm.hist_pipe p
  · intro p hh hmem
    unfold deliveredCount
    rw [hdelH p.other]
    have h1 := hm.delivered_eq p hh hmem
    have h2 := hwsum p hh hmem
    unfold deliveredCount at h1
    omega
  · intro p hh hph
    unfold pushedCount
    rw [hpushH p]
    exact hm.pushed_real p hh hph

-- ======================================================= the push arm

/-- Assemble `MuxInv` after a push from its base-effect deltas: one
wire send up, one flush receipt on, one pipe entry in. -/
theorem muxInv_push_assemble {p : Party}
    {h : Nat} {s s' : MState} (hm : MuxInv sk s)
    (hhmem : Chan.wire p h ∈ allChans sk)
    (hinvl : InvL sk .impl s'.base)
    (hchan : s'.base.chan = s.base.chan)
    (hsent_touch : sentOf sk s'.base (Chan.wire p h)
      = sentOf sk s.base (Chan.wire p h) + 1)
    (hsent_other : ∀ c, c ≠ Chan.wire p h →
      sentOf sk s'.base c = sentOf sk s.base c)
    (hrecv : ∀ c, recvdOf sk s'.base c = recvdOf sk s.base c)
    (hhist : s'.hist = recordObs s.hist p (.pushed h))
    (hpipe : s'.pipe = fun q => if q == p then s.pipe q ++ [Chan.wire p h]
      else s.pipe q) :
    MuxInv sk s' := by
  have histp : s'.hist p = s.hist p ++ [.pushed h] := by
    rw [hhist]
    unfold recordObs
    simp
  have histo : s'.hist p.other = s.hist p.other := by
    rw [hhist]
    unfold recordObs
    rw [if_neg (by cases p <;> simp [Party.other])]
  have hDle := delTotal_le_pushes hm p
  have hpipep : s'.pipe p = s.pipe p ++ [Chan.wire p h] := by
    have h1 := congrFun hpipe p
    rw [h1]
    simp
  have hpipeo : s'.pipe p.other = s.pipe p.other := by
    have h1 := congrFun hpipe p.other
    rw [h1]
    rw [if_neg (by cases p <;> simp [Party.other])]
  have hoo : p.other.other = p := Party.other_other p
  refine ⟨hinvl, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · -- slot: occupancies untouched
    intro c hc
    rw [hchan]
    exact hm.slot c hc
  · -- internal flow: everything frames off the pushed wire
    intro c hc hwv
    rw [hchan, hrecv, hsent_other c (ne_wire_of_not_isWire hwv p h)]
    exact hm.flow_int c hc hwv
  · -- pushed_eq: the one new receipt matches the one new send
    intro q hh
    rcases party_eq_or_other q p with rfl | rfl
    · unfold pushedCount
      rw [histp, pushHeights_append_pushed, List.count_append]
      by_cases hhh : hh = h
      · subst hhh
        rw [hsent_touch]
        have h1 := hm.pushed_eq q hh
        unfold pushedCount at h1
        have h2 : List.count hh [hh] = 1 := by simp
        omega
      · rw [hsent_other _ (by simp [Chan.wire.injEq, hhh])]
        have h1 := hm.pushed_eq q hh
        unfold pushedCount at h1
        have h2 : List.count hh [h] = 0 := by
          simp [show ¬ h = hh from fun e => hhh e.symm]
        omega
    · unfold pushedCount
      rw [histo, hsent_other _ (by
        intro heq
        rw [Chan.wire.injEq] at heq
        obtain ⟨heq1, -⟩ := heq
        cases p <;> simp [Party.other] at heq1)]
      exact hm.pushed_eq p.other hh
  · -- hist_del: the delivered prefix is below the appended push
    intro q
    rcases party_eq_or_other q p with rfl | rfl
    · rw [histo, histp, pushHeights_append_pushed,
        List.take_append_of_le_length hDle]
      exact hm.hist_del q
    · rw [hoo, histp, delHeights_append_pushed, histo]
      unfold delTotal
      rw [delHeights_append_pushed]
      have hbase := hm.hist_del p.other
      rw [hoo] at hbase
      unfold delTotal at hbase
      exact hbase
  · -- hist_pipe: the pushed suffix grows by its own new frame
    intro q
    rcases party_eq_or_other q p with rfl | rfl
    · rw [hpipep, histo, histp, pushHeights_append_pushed,
        List.drop_append_of_le_length hDle, List.map_append,
        ← hm.hist_pipe q]
      rfl
    · rw [hpipeo, hoo, histp, histo]
      unfold delTotal
      rw [delHeights_append_pushed]
      have hbase := hm.hist_pipe p.other
      rw [hoo] at hbase
      unfold delTotal at hbase
      exact hbase
  · -- delivered_eq: deliveries and slots untouched
    intro q hh hmem
    unfold deliveredCount
    rcases party_eq_or_other q p with rfl | rfl
    · rw [histo, hchan, hrecv]
      exact hm.delivered_eq q hh hmem
    · rw [hoo, histp, delHeights_append_pushed, hchan, hrecv]
      have hbase := hm.delivered_eq p.other hh hmem
      rw [hoo] at hbase
      exact hbase
  · -- pushed_real: the one new receipt is on a real stream
    intro q hh hph
    rcases party_eq_or_other q p with rfl | rfl
    · unfold pushedCount
      rw [histp, pushHeights_append_pushed, List.count_append]
      have hne : ¬ h = hh := by
        intro heq
        rw [heq] at hhmem
        exact hph hhmem
      have h1 := hm.pushed_real q hh hph
      unfold pushedCount at h1
      have h2 : List.count hh [h] = 0 := by simp [hne]
      omega
    · unfold pushedCount
      rw [histo]
      exact hm.pushed_real p.other hh hph

/-- `MuxInv` survives a push: the three hand shapes, assembled. -/
theorem muxInv_firePush (hwf : sk.wellFormed = true) {C : Nat}
    {p : Party} {h : Nat} {s s' : MState}
    (hf : firePush sk C p h s = some s') (hm : MuxInv sk s) :
    MuxInv sk s' := by
  obtain ⟨hroom, hhist, hpipe, hshape⟩ := firePush_inv hf
  rcases hshape with ⟨hrh, hpI, hio, hbase⟩ | ⟨hrh, hpR, hro, hbase⟩ |
    ⟨i, hmem, hph2, hcm, hbase⟩
  · -- initiator opening
    subst hpI
    subst hrh
    obtain ⟨ht, ho, hr⟩ := sentOf_iopenWire (sk := sk) hio hm.invl
    exact muxInv_push_assemble hm (wire_rootH_mem sk .I)
      (by rw [hbase]; exact preserveL_iopenWire hm.invl)
      (by rw [hbase]) (by rw [hbase]; exact ht)
      (fun c hc => by rw [hbase]; exact ho c hc)
      (fun c => by rw [hbase]; exact hr c) hhist hpipe
  · -- responder opening
    subst hpR
    subst hrh
    obtain ⟨ht, ho, hr⟩ := sentOf_ropenWire (sk := sk) hro hm.invl
    exact muxInv_push_assemble hm (wire_rootH_mem sk .R)
      (by rw [hbase]; exact preserveL_ropenWire hro hm.invl)
      (by rw [hbase]) (by rw [hbase]; exact ht)
      (fun c hc => by rw [hbase]; exact ho c hc)
      (fun c => by rw [hbase]; exact hr c) hhist hpipe
  · -- walk wire: the raw fire shape
    have hobl : obligChan (p, h) (Oblig.wire i) = Chan.wire p h := rfl
    obtain ⟨ht, ho, hr⟩ := delta_fire hwf (p, h) hmem hph2 hcm hm.invl
    exact muxInv_push_assemble hm (wireOut_mem sk hmem)
      (by rw [hbase]; exact preserveL_fire hwf (p, h) hmem hph2 hcm hm.invl)
      (by rw [hbase]; rfl)
      (by rw [hbase]; rw [← hobl]; exact ht)
      (fun c hc => by rw [hbase]; exact ho c (by rw [hobl]; exact hc))
      (fun c => by rw [hbase]; exact hr c) hhist hpipe

-- ==================================================== the deliver arm

/-- `MuxInv` survives a delivery: the FIFO split point advances by one
as the head frame fills its slot. -/
theorem muxInv_deliver {C : Nat} {σI σR : Strategy} {p : Party}
    {s s' : MState}
    (hstep : apply sk .impl C σI σR (.deliver p) s = some s')
    (hm : MuxInv sk s) : MuxInv sk s' := by
  simp only [apply] at hstep
  cases hp : s.pipe p with
  | nil =>
      rw [hp] at hstep
      dsimp only at hstep
      cases hstep
  | cons c rest =>
  rw [hp] at hstep
  dsimp only at hstep
  split at hstep
  case isFalse => cases hstep
  case isTrue hslot0 =>
  injection hstep with hs'
  obtain ⟨h₀, hc⟩ := hm.pipe_mem_wire (hp ▸ List.mem_cons_self ..)
  subst hc
  have hchan0 : s.base.chan (Chan.wire p h₀) = 0 := by simpa using hslot0
  -- the state components
  have hbase : s'.base = { s.base with
      chan := bump s.base.chan (Chan.wire p h₀) 1 } := by rw [← hs']
  have hpipe' : s'.pipe = fun q => if q == p then rest else s.pipe q := by
    rw [← hs']
  have hhist : s'.hist = recordObs s.hist p.other
      (.delivered h₀) := by
    rw [← hs']
    rfl
  have histp : s'.hist p = s.hist p := by
    rw [hhist]
    unfold recordObs
    rw [if_neg (by cases p <;> simp [Party.other])]
  have histo : s'.hist p.other = s.hist p.other ++ [.delivered h₀] := by
    rw [hhist]
    unfold recordObs
    simp
  -- chan facts
  have hchan_at : s'.base.chan (Chan.wire p h₀) = 1 := by
    rw [hbase]
    simp [bump, hchan0]
  have hchan_ne : ∀ c', c' ≠ Chan.wire p h₀ →
      s'.base.chan c' = s.base.chan c' := by
    intro c' hne
    rw [hbase]
    simp only [bump]
    rw [if_neg (by simpa using hne)]
  have hpipep : s'.pipe p = rest := by
    have h1 := congrFun hpipe' p
    rw [h1]
    simp
  have hpipeo : s'.pipe p.other = s.pipe p.other := by
    have h1 := congrFun hpipe' p.other
    rw [h1]
    rw [if_neg (by cases p <;> simp [Party.other])]
  have hoo : p.other.other = p := Party.other_other p
  -- count facts through the FIFO split
  have hDle := delTotal_le_pushes hm p
  have hdrop : (pushHeights (s.hist p)).drop (delTotal (s.hist p.other))
      = h₀ :: ((pushHeights (s.hist p)).drop
          (delTotal (s.hist p.other))).tail := by
    have hmp := hm.hist_pipe p
    rw [hp] at hmp
    cases hd : (pushHeights (s.hist p)).drop (delTotal (s.hist p.other))
      with
    | nil => rw [hd] at hmp; cases hmp
    | cons x xs =>
        rw [hd] at hmp
        simp only [List.map_cons] at hmp
        injection hmp with h1 h2
        simp only [List.tail_cons]
        have hx : x = h₀ := by
          have h3 := congrArg wireHeight h1
          simpa [wireHeight] using h3.symm
        rw [hx]
  have htail : rest = (((pushHeights (s.hist p)).drop
      (delTotal (s.hist p.other))).tail).map (Chan.wire p) := by
    have hmp := hm.hist_pipe p
    rw [hp, hdrop] at hmp
    simp only [List.map_cons, List.cons.injEq] at hmp
    first
      | exact hmp
      | exact hmp.2
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · -- invl: channel-blind
    rw [hbase]
    exact invL_chan hm.invl _
  · -- slot
    intro c hc
    by_cases hne : c = Chan.wire p h₀
    · subst hne
      rw [hchan_at]
      exact Nat.le_refl 1
    · rw [hchan_ne c hne]
      exact hm.slot c hc
  · -- internal flow: the bumped channel is a wire
    intro c hc hwv
    rw [hchan_ne c (ne_wire_of_not_isWire hwv p h₀), hbase,
      sentOf_chan, recvdOf_chan]
    exact hm.flow_int c hc hwv
  · -- pushed_eq: receipts and sends untouched
    intro q hh
    unfold pushedCount
    have hph : pushHeights (s'.hist q) = pushHeights (s.hist q) := by
      rcases party_eq_or_other q p with rfl | rfl
      · rw [histp]
      · rw [histo, pushHeights_append_delivered]
    rw [hph, hbase, sentOf_chan]
    exact hm.pushed_eq q hh
  · -- hist_del: the split point advances on the delivering side
    intro q
    rcases party_eq_or_other q p with rfl | rfl
    · rw [histo, histp, delHeights_append_delivered]
      unfold delTotal
      rw [delHeights_append_delivered, List.length_append,
        List.length_singleton]
      have hget : (pushHeights (s.hist q))[(delHeights
          (s.hist q.other)).length]? = some h₀ := by
        have hsplit := congrArg (fun l => l[0]?) hdrop
        simp only [List.getElem?_drop, Nat.add_zero] at hsplit
        unfold delTotal at hsplit
        simpa using hsplit
      have hbase := hm.hist_del q
      unfold delTotal at hbase
      rw [List.take_add_one, hget, ← hbase]
      rfl
    · rw [hoo, histp, histo, pushHeights_append_delivered]
      have hbase := hm.hist_del p.other
      rw [hoo] at hbase
      exact hbase
  · -- hist_pipe: the pushed suffix loses its head
    intro q
    rcases party_eq_or_other q p with rfl | rfl
    · rw [hpipep, histp, histo]
      unfold delTotal
      rw [delHeights_append_delivered, List.length_append,
        List.length_singleton, htail]
      congr 1
      unfold delTotal at hdrop ⊢
      rw [show (delHeights (s.hist q.other)).length + 1
          = (delHeights (s.hist q.other)).length + 1 from rfl]
      have hdd : (pushHeights (s.hist q)).drop
          ((delHeights (s.hist q.other)).length + 1)
          = ((pushHeights (s.hist q)).drop
              ((delHeights (s.hist q.other)).length)).drop 1 := by
        rw [List.drop_drop]
      rw [hdd, hdrop]
      rfl
    · rw [hpipeo, hoo, histo, pushHeights_append_delivered, histp]
      have hbase := hm.hist_pipe p.other
      rw [hoo] at hbase
      exact hbase
  · -- delivered_eq: the new receipt matches the new slot occupancy
    intro q hh hmem
    unfold deliveredCount
    rcases party_eq_or_other q p with rfl | rfl
    · rw [histo, delHeights_append_delivered, List.count_append]
      by_cases hhh : hh = h₀
      · subst hhh
        rw [hchan_at, hbase, recvdOf_chan]
        have h1 := hm.delivered_eq q hh hmem
        unfold deliveredCount at h1
        have h2 : List.count hh [hh] = 1 := by simp
        omega
      · rw [hchan_ne _ (by simp [Chan.wire.injEq, hhh]), hbase,
          recvdOf_chan]
        have h1 := hm.delivered_eq q hh hmem
        unfold deliveredCount at h1
        have h2 : List.count hh [h₀] = 0 := by
          simp [show ¬ h₀ = hh from fun e => hhh e.symm]
        omega
    · rw [hoo, histp,
        hchan_ne _ (by
          intro heq
          rw [Chan.wire.injEq] at heq
          obtain ⟨heq1, heq2⟩ := heq
          cases p <;> simp [Party.other] at heq1), hbase, recvdOf_chan]
      have h1 := hm.delivered_eq p.other hh hmem
      rw [hoo] at h1
      exact h1
  · -- pushed_real: no push happened
    intro q hh hph
    unfold pushedCount
    have hph' : pushHeights (s'.hist q) = pushHeights (s.hist q) := by
      rcases party_eq_or_other q p with rfl | rfl
      · rw [histp]
      · rw [histo, pushHeights_append_delivered]
    rw [hph']
    exact hm.pushed_real q hh hph

-- ======================================================== the dispatch

/-- Consecution for the muxed system: every arm of every strategy pair
preserves the ground facts (MUX-ADJUDICATION §4 stage F). -/
theorem muxInv_preserved (hwf : sk.wellFormed = true) {C : Nat}
    {σI σR : Strategy} {a : MAction} {s s' : MState}
    (hstep : apply sk .impl C σI σR a s = some s')
    (hm : MuxInv sk s) : MuxInv sk s' := by
  cases a with
  | base a => exact muxInv_base hwf hstep hm
  | push p =>
      have hσ : apply sk .impl C σI σR (.push p) s
          = (match (match p with | .I => σI | .R => σR) sk (s.hist p) with
             | some h => firePush sk C p h s
             | none => none) := by
        cases p <;> rfl
      rw [hσ] at hstep
      cases hname : (match p with | .I => σI | .R => σR) sk (s.hist p) with
      | none => rw [hname] at hstep; cases hstep
      | some h =>
          rw [hname] at hstep
          exact muxInv_firePush hwf hstep hm
  | deliver p => exact muxInv_deliver hstep hm

/-- The ground facts hold at every reachable state of the muxed system,
for every strategy pair: the stage-F obligation, discharged — both
stage-3 theorems consume `MuxInv` through this. -/
theorem muxInv_reachable (hwf : sk.wellFormed = true) {C : Nat}
    {σI σR : Strategy} {s : MState}
    (hr : MReachable sk .impl C σI σR s) : MuxInv sk s := by
  induction hr with
  | init => exact muxInv_init sk
  | step a _ hstep ih => exact muxInv_preserved hwf hstep ih

end StreamingMirror.Mux
