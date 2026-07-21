/-
Local preservation and counting deltas for the top-level action arms
(openers, absorber, finishes), extracted from the monolithic
`preserve_<X>` proofs in Proofs/Preserve/{Top,AbsorbFin}.lean.

The muxed chase (Mux/Proofs/Chase) needs, per base action arm, (a) that
the local invariant fragment `InvL` (wk/asm/top) is preserved and (b)
the exact per-channel counting deltas the arm induces. The base
monoliths prove `InvP → InvP` and interleave both concerns with
`hi.flow`, which a muxed state does not satisfy (frames ride the pipe),
so they cannot be applied there. Each `preserveL_<X>` here keeps the
monolith's guard-inversion prelude plus its wk/asm/top bullets (which
never touch `hi.flow`); each `delta_<X>` re-derives the per-channel
counting facts the flow bullet computed — wire sends untouched, the
wire sum `chan + recvdOf` conserved, wire occupancy monotone, and
internal-channel flow/capacity preserved — with the touched channel's
occupancy facts recovered from the action's own guard instead of the
flow equation.
-/
import StreamingMirror.Mux.Basic
import StreamingMirror.Proofs.Preserve

namespace StreamingMirror.Mux

open Model

variable {sk : Skel} {ax : AxMode} {s s' : State}

-- ========================================================= shared plumbing

/-- An internal channel differs from every wire channel.

The converse direction of `isWire_eq`, re-derived locally so this file
depends only on Mux/Basic. -/
private theorem ne_wire {c : Chan} (hw : isWire c = false)
    (p : Party) (h : Nat) : c ≠ Chan.wire p h := by
  intro he
  rw [he] at hw
  simp [isWire] at hw

/-- The five counting-delta conjuncts, for an arm that moves nothing.

Covers the chooses and closes: occupancy, every producer count, and
every consumer count are unchanged. -/
private theorem deltas_of_frame
    (hchan : s'.chan = s.chan)
    (hsent : ∀ c, sentOf sk s' c = sentOf sk s c)
    (hrecv : ∀ c, recvdOf sk s' c = recvdOf sk s c) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  refine ⟨fun p h => hsent _, fun p h => ?_, fun p h => ?_,
    fun c _ _ heq => ?_, fun c _ _ hcap => ?_⟩
  · rw [hchan, hrecv]
  · simp [hchan]
  · rw [hchan, hsent, hrecv]
    exact heq
  · rw [hchan]
    exact hcap

/-- The five counting-delta conjuncts, for a wire-channel receive.

On the touched wire channel occupancy drops by one exactly as the
consumer count rises by one, so the wire sum is conserved and occupancy
decreases; every other channel frames, and no producer count moves. -/
private theorem deltas_of_wire_recv (q : Party) (k : Nat)
    (hchan : s'.chan = bump s.chan (Chan.wire q k) (-1))
    (hpos : s.chan (Chan.wire q k) > 0)
    (hsent : ∀ c, sentOf sk s' c = sentOf sk s c)
    (hrecv0 : ∀ c, c ≠ Chan.wire q k →
      recvdOf sk s' c = recvdOf sk s c)
    (hrecv1 : recvdOf sk s' (Chan.wire q k)
      = recvdOf sk s (Chan.wire q k) + 1) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  refine ⟨fun p h => hsent _, fun p h => ?_, fun p h => ?_,
    fun c _ hw heq => ?_, fun c _ hw hcap => ?_⟩
  · by_cases he : Chan.wire p h = Chan.wire q k
    · rw [he, hchan, bump_neg_one, hrecv1]
      omega
    · rw [hchan, bump_ne _ _ he, hrecv0 _ he]
  · by_cases he : Chan.wire p h = Chan.wire q k
    · rw [he, hchan, bump_neg_one]
      omega
    · simp [hchan, bump_ne _ _ he]
  · have hne := ne_wire hw q k
    rw [hchan, bump_ne _ _ hne, hrecv0 _ hne, hsent]
    exact heq
  · have hne := ne_wire hw q k
    rw [hchan, bump_ne _ _ hne]
    exact hcap

/-- The five counting-delta conjuncts, for an internal-channel receive.

The touched channel is off the wire family, so every wire conjunct
frames; on the touched channel occupancy drops by one exactly as the
consumer count rises by one, conserving the flow sum, and the drop
keeps occupancy within capacity. -/
private theorem deltas_of_internal_recv (c₀ : Chan)
    (hint : isWire c₀ = false)
    (hchan : s'.chan = bump s.chan c₀ (-1))
    (hpos : s.chan c₀ > 0)
    (hsent : ∀ c, sentOf sk s' c = sentOf sk s c)
    (hrecv0 : ∀ c, c ≠ c₀ → recvdOf sk s' c = recvdOf sk s c)
    (hrecv1 : recvdOf sk s' c₀ = recvdOf sk s c₀ + 1) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  have hwne : ∀ p h, Chan.wire p h ≠ c₀ :=
    fun p h => (ne_wire hint p h).symm
  refine ⟨fun p h => hsent _, fun p h => ?_, fun p h => ?_,
    fun c _ _ heq => ?_, fun c _ _ hcap => ?_⟩
  · rw [hchan, bump_ne _ _ (hwne p h), hrecv0 _ (hwne p h)]
  · simp [hchan, bump_ne _ _ (hwne p h)]
  · by_cases he : c = c₀
    · subst he
      rw [hchan, bump_neg_one, hrecv1, hsent]
      omega
    · rw [hchan, bump_ne _ _ he, hrecv0 _ he, hsent]
      exact heq
  · by_cases he : c = c₀
    · subst he
      rw [hchan, bump_neg_one]
      omega
    · rw [hchan, bump_ne _ _ he]
      exact hcap

/-- The five counting-delta conjuncts, for an internal-channel send.

The touched channel is off the wire family, so every wire conjunct
frames; on the touched channel occupancy rises by one exactly as the
producer count rises by one, conserving the flow sum, and the send
guard `chan < cap` keeps the new occupancy within capacity. -/
private theorem deltas_of_internal_send (c₀ : Chan)
    (hint : isWire c₀ = false)
    (hchan : s'.chan = bump s.chan c₀ 1)
    (hlt : s.chan c₀ < sk.cap c₀)
    (hsent0 : ∀ c, c ≠ c₀ → sentOf sk s' c = sentOf sk s c)
    (hsent1 : sentOf sk s' c₀ = sentOf sk s c₀ + 1)
    (hrecv : ∀ c, recvdOf sk s' c = recvdOf sk s c) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  have hwne : ∀ p h, Chan.wire p h ≠ c₀ :=
    fun p h => (ne_wire hint p h).symm
  refine ⟨fun p h => hsent0 _ (hwne p h), fun p h => ?_, fun p h => ?_,
    fun c _ _ heq => ?_, fun c _ _ hcap => ?_⟩
  · rw [hchan, bump_ne _ _ (hwne p h), hrecv]
  · simp [hchan, bump_ne _ _ (hwne p h)]
  · by_cases he : c = c₀
    · subst he
      rw [hchan, bump_one, hrecv, hsent1]
      omega
    · rw [hchan, bump_ne _ _ he, hrecv, hsent0 _ he]
      exact heq
  · by_cases he : c = c₀
    · subst he
      rw [hchan, bump_one]
      omega
    · rw [hchan, bump_ne _ _ he]
      exact hcap

-- ============================================================ iopenChoose

/-- Local (`InvL`) preservation for `.iopenChoose`.

Extracted from `preserve_iopenChoose` (Proofs/Preserve/Top.lean): the
guard-inversion prelude plus the wk/asm/top bullets; the flow bullet
(and with it every use of `hi.flow`) is dropped. -/
theorem preserveL_iopenChoose (_hwf : sk.wellFormed = true) (o : IOblig)
    (hstep : Model.apply sk ax (.iopenChoose o) s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨hnone, hch⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
      have hCh : s'.iopenCh = some o := by rw [← hs']
      rw [topLocalOk] at htop ⊢
      rw [hCh,
        show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.ropenCh = s.ropenCh from by rw [← hs'],
        show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
        show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
        show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
        show s'.rfinGot = s.rfinGot from by rw [← hs']]
      cases o <;> simp_all [iopenChoosable]

/-- Counting deltas for `.iopenChoose`: a pure commit, nothing moves.

Extracted from the flow bullet of `preserve_iopenChoose`
(Proofs/Preserve/Top.lean): occupancy and both derived counts are
unchanged on every channel. -/
theorem delta_iopenChoose (_hwf : sk.wellFormed = true) (o : IOblig)
    (hstep : Model.apply sk ax (.iopenChoose o) s = some s')
    (_hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue _hg =>
    injection hstep with hs'
    have hchan : s'.chan = s.chan := by rw [← hs']
    have hsent : ∀ c, sentOf sk s' c = sentOf sk s c := by
      intro c; rw [← hs']; cases c <;> rfl
    have hrecv : ∀ c, recvdOf sk s' c = recvdOf sk s c := by
      intro c; rw [← hs']; cases c <;> rfl
    exact deltas_of_frame hchan hsent hrecv

-- ============================================================== iopenFire

/-- Local (`InvL`) preservation for `.iopenFire` (both committed arms).

Extracted from `preserve_iopenFire` (Proofs/Preserve/Top.lean): the
guard-inversion prelude, the freshness facts `hiw`/`hiq` (which read
only `hi.top`), and the wk/asm/top bullets of each arm; the flow
bullets are dropped. -/
theorem preserveL_iopenFire (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .iopenFire s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  next hch =>
    -- committed obligation: .wire
    split at hstep
    case isFalse => simp at hstep
    case isTrue _hg =>
      injection hstep with hs'
      have hiw : s.iopenWire = false := by
        have htop := hi.top
        rw [topLocalOk] at htop
        simp only [Bool.and_eq_true] at htop
        have h1 := htop.1.1.1.1.1.1.1.1.1.1.1
        rw [hch] at h1
        simpa using h1
      refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
      · rw [← hs']; exact hi.wk pk hpk
      · rw [← hs']; exact hi.asm pk hpk
      · have htop := hi.top
        rw [topLocalOk] at htop ⊢
        rw [show s'.iopenCh = none from by rw [← hs'],
          show s'.iopenWire = true from by rw [← hs'],
          show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
          show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
          show s'.ropenWire = s.ropenWire from by rw [← hs'],
          show s'.ropenRes = s.ropenRes from by rw [← hs'],
          show s'.ropenQ = s.ropenQ from by rw [← hs'],
          show s'.ropenCh = s.ropenCh from by rw [← hs'],
          show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
          show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
          show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
          show s'.rfinGot = s.rfinGot from by rw [← hs']]
        rw [hch] at htop
        simp_all
  next hch =>
    -- committed obligation: .query
    split at hstep
    case isFalse => simp at hstep
    case isTrue _hg =>
      injection hstep with hs'
      have hiq : s.iopenQuery = false := by
        have htop := hi.top
        rw [topLocalOk] at htop
        simp only [Bool.and_eq_true] at htop
        have h2 := htop.1.1.1.1.1.1.1.1.1.1.2
        rw [hch] at h2
        simp at h2
        exact h2.1
      refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
      · rw [← hs']; exact hi.wk pk hpk
      · rw [← hs']; exact hi.asm pk hpk
      · have htop := hi.top
        rw [topLocalOk] at htop ⊢
        rw [show s'.iopenCh = none from by rw [← hs'],
          show s'.iopenWire = s.iopenWire from by rw [← hs'],
          show s'.iopenQuery = true from by rw [← hs'],
          show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
          show s'.ropenWire = s.ropenWire from by rw [← hs'],
          show s'.ropenRes = s.ropenRes from by rw [← hs'],
          show s'.ropenQ = s.ropenQ from by rw [← hs'],
          show s'.ropenCh = s.ropenCh from by rw [← hs'],
          show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
          show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
          show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
          show s'.rfinGot = s.rfinGot from by rw [← hs']]
        rw [hch] at htop
        simp_all
  next => simp at hstep

/-- Counting deltas for `.iopenFire` away from the `.wire` commitment.

Extracted from the `.query`-arm flow bullet of `preserve_iopenFire`
(Proofs/Preserve/Top.lean). With `hnw` the enabled arm is `.query`,
whose send lands on the internal channel
`Chan.asked Party.I (sk.rootH - 1)`: wire channels are untouched, and
the touched channel's producer count `b2n iopenQuery` rises 0→1 (the
freshness comes from `hi.top`) exactly as its occupancy does (the
`chan < 1` guard supplies the capacity bound). -/
theorem delta_iopenFire (_hwf : sk.wellFormed = true)
    (hnw : s.iopenCh ≠ some IOblig.wire)
    (hstep : Model.apply sk ax .iopenFire s = some s')
    (hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  next hch => exact absurd hch hnw
  next hch =>
    -- committed obligation: .query
    split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      injection hstep with hs'
      have hiq : s.iopenQuery = false := by
        have htop := hi.top
        rw [topLocalOk] at htop
        simp only [Bool.and_eq_true] at htop
        have h2 := htop.1.1.1.1.1.1.1.1.1.1.2
        rw [hch] at h2
        simp at h2
        exact h2.1
      have hchan :
          s'.chan = bump s.chan (Chan.asked Party.I (sk.rootH - 1)) 1 := by
        rw [← hs']
      have hcap : s.chan (Chan.asked Party.I (sk.rootH - 1))
          < sk.cap (Chan.asked Party.I (sk.rootH - 1)) := hg
      have hrecv : ∀ c, recvdOf sk s' c = recvdOf sk s c := by
        intro c; rw [← hs']; cases c <;> rfl
      have hsent1 : sentOf sk s' (Chan.asked Party.I (sk.rootH - 1))
          = sentOf sk s (Chan.asked Party.I (sk.rootH - 1)) + 1 := by
        have h1 : sentOf sk s' (Chan.asked Party.I (sk.rootH - 1)) = 1 := by
          rw [← hs']; simp [sentOf, b2n]
        have h0 : sentOf sk s (Chan.asked Party.I (sk.rootH - 1)) = 0 := by
          simp [sentOf, hiq, b2n]
        rw [h1, h0]
      have hsent0 : ∀ c, c ≠ Chan.asked Party.I (sk.rootH - 1) →
          sentOf sk s' c = sentOf sk s c := by
        intro c hne
        rw [← hs']
        cases c with
        | asked p h =>
            by_cases h1 : (p == Party.I && h == sk.rootH - 1) = true
            · simp only [Bool.and_eq_true, beq_iff_eq] at h1
              exact absurd (by rw [h1.1, h1.2]) hne
            · simp [sentOf, wkQSentTot, wkQSum, h1]
        | wire p h => rfl
        | leafRequests => rfl
        | upper p h => rfl
        | lower p h => rfl
        | level p j => rfl
        | rootret => rfl
        | rootrets => rfl
        | rootres => rfl
      exact deltas_of_internal_send (Chan.asked Party.I (sk.rootH - 1))
        rfl hchan hcap hsent0 hsent1 hrecv
  next => simp at hstep

-- ============================================================== ropenRecv

/-- Local (`InvL`) preservation for `.ropenRecv`.

Extracted from `preserve_ropenRecv` (Proofs/Preserve/Top.lean): the
guard-inversion prelude plus the wk/asm/top bullets; the flow bullet is
dropped. -/
theorem preserveL_ropenRecv (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .ropenRecv s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_true_eq] at hg
    obtain ⟨hgot, _hpos⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
      rw [topLocalOk] at htop ⊢
      rw [show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.iopenCh = s.iopenCh from by rw [← hs'],
        show s'.ropenGotWire = true from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.ropenCh = s.ropenCh from by rw [← hs'],
        show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
        show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
        show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
        show s'.rfinGot = s.rfinGot from by rw [← hs']]
      simp_all

/-- Counting deltas for `.ropenRecv`, a wire receive.

Extracted from the flow bullet of `preserve_ropenRecv`
(Proofs/Preserve/Top.lean). On `Chan.wire Party.I sk.rootH` occupancy
drops by one exactly as `b2n ropenGotWire` rises 0→1 (the guard gives
both the freshness and `chan > 0`), so the wire sum is conserved and
occupancy decreases; every other channel frames, and no producer count
moves. -/
theorem delta_ropenRecv (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .ropenRecv s = some s')
    (_hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_true_eq] at hg
    obtain ⟨hgot, hpos⟩ := hg
    injection hstep with hs'
    have hchan :
        s'.chan = bump s.chan (Chan.wire Party.I sk.rootH) (-1) := by
      rw [← hs']
    have hsent : ∀ c, sentOf sk s' c = sentOf sk s c := by
      intro c; rw [← hs']; cases c <;> rfl
    have hrecv1 : recvdOf sk s' (Chan.wire Party.I sk.rootH)
        = recvdOf sk s (Chan.wire Party.I sk.rootH) + 1 := by
      have h1 : recvdOf sk s' (Chan.wire Party.I sk.rootH) = 1 := by
        rw [← hs']; simp [recvdOf, b2n]
      have h0 : recvdOf sk s (Chan.wire Party.I sk.rootH) = 0 := by
        simp [recvdOf, hgot, b2n]
      rw [h1, h0]
    have hrecv0 : ∀ c, c ≠ Chan.wire Party.I sk.rootH →
        recvdOf sk s' c = recvdOf sk s c := by
      intro c hne
      rw [← hs']
      cases c with
      | wire p h =>
          by_cases hh : (h == sk.rootH) = true
          · by_cases hp : (p == Party.I) = true
            · rw [beq_iff_eq] at hh hp
              exact absurd (by rw [hh, hp]) hne
            · simp [recvdOf, wkWireRecvd, hh, hp]
          · simp [recvdOf, wkWireRecvd, absorbWireRecvd, hh]
      | asked p h => rfl
      | leafRequests => rfl
      | upper p h => rfl
      | lower p h => rfl
      | level p j => rfl
      | rootret => rfl
      | rootrets => rfl
      | rootres => rfl
    exact deltas_of_wire_recv Party.I sk.rootH hchan hpos hsent
      hrecv0 hrecv1

-- ============================================================ ropenChoose

/-- Local (`InvL`) preservation for `.ropenChoose`.

Extracted from `preserve_ropenChoose` (Proofs/Preserve/Top.lean): the
guard-inversion prelude plus the wk/asm/top bullets; the flow bullet is
dropped. -/
theorem preserveL_ropenChoose (_hwf : sk.wellFormed = true) (o : ROblig)
    (hstep : Model.apply sk ax (.ropenChoose o) s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨hnone, hch⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
      have hCh : s'.ropenCh = some o := by rw [← hs']
      rw [topLocalOk] at htop ⊢
      rw [hCh,
        show s'.iopenWire = s.iopenWire from by rw [← hs'],
        show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
        show s'.iopenCh = s.iopenCh from by rw [← hs'],
        show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
        show s'.ropenWire = s.ropenWire from by rw [← hs'],
        show s'.ropenRes = s.ropenRes from by rw [← hs'],
        show s'.ropenQ = s.ropenQ from by rw [← hs'],
        show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
        show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
        show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
        show s'.rfinGot = s.rfinGot from by rw [← hs']]
      cases o <;> simp_all [ropenChoosable, Skel.rootPending]

/-- Counting deltas for `.ropenChoose`: a pure commit, nothing moves.

Extracted from the flow bullet of `preserve_ropenChoose`
(Proofs/Preserve/Top.lean): occupancy and both derived counts are
unchanged on every channel. -/
theorem delta_ropenChoose (_hwf : sk.wellFormed = true) (o : ROblig)
    (hstep : Model.apply sk ax (.ropenChoose o) s = some s')
    (_hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue _hg =>
    injection hstep with hs'
    have hchan : s'.chan = s.chan := by rw [← hs']
    have hsent : ∀ c, sentOf sk s' c = sentOf sk s c := by
      intro c; rw [← hs']; cases c <;> rfl
    have hrecv : ∀ c, recvdOf sk s' c = recvdOf sk s c := by
      intro c; rw [← hs']; cases c <;> rfl
    exact deltas_of_frame hchan hsent hrecv

-- ============================================================== ropenFire

/-- Local (`InvL`) preservation for `.ropenFire` (all committed arms).

Extracted from `preserve_ropenFire` (Proofs/Preserve/Top.lean): the
guard-inversion prelude, the freshness facts `hrw`/`hrr`/`hq` (which
read only `hi.top`), and the wk/asm/top bullets of each arm; the flow
bullets are dropped. -/
theorem preserveL_ropenFire (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .ropenFire s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  next hch =>
    -- committed obligation: .wire
    split at hstep
    case isFalse => simp at hstep
    case isTrue _hg =>
      injection hstep with hs'
      have hrw : s.ropenWire = false := by
        have htop := hi.top
        rw [topLocalOk] at htop
        simp only [Bool.and_eq_true] at htop
        have h6 := htop.1.1.1.1.1.1.2
        rw [hch] at h6
        simpa using h6
      refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
      · rw [← hs']; exact hi.wk pk hpk
      · rw [← hs']; exact hi.asm pk hpk
      · have htop := hi.top
        rw [topLocalOk] at htop ⊢
        rw [show s'.iopenWire = s.iopenWire from by rw [← hs'],
          show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
          show s'.iopenCh = s.iopenCh from by rw [← hs'],
          show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
          show s'.ropenWire = true from by rw [← hs'],
          show s'.ropenRes = s.ropenRes from by rw [← hs'],
          show s'.ropenQ = s.ropenQ from by rw [← hs'],
          show s'.ropenCh = none from by rw [← hs'],
          show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
          show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
          show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
          show s'.rfinGot = s.rfinGot from by rw [← hs']]
        rw [hch] at htop
        simp_all
  next hch =>
    -- committed obligation: .res
    split at hstep
    case isFalse => simp at hstep
    case isTrue _hg =>
      injection hstep with hs'
      have hrr : s.ropenRes = false := by
        have htop := hi.top
        rw [topLocalOk] at htop
        simp only [Bool.and_eq_true] at htop
        have h7 := htop.1.1.1.1.1.2
        rw [hch] at h7
        simp at h7
        exact h7.1
      refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
      · rw [← hs']; exact hi.wk pk hpk
      · rw [← hs']; exact hi.asm pk hpk
      · have htop := hi.top
        rw [topLocalOk] at htop ⊢
        rw [show s'.iopenWire = s.iopenWire from by rw [← hs'],
          show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
          show s'.iopenCh = s.iopenCh from by rw [← hs'],
          show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
          show s'.ropenWire = s.ropenWire from by rw [← hs'],
          show s'.ropenRes = true from by rw [← hs'],
          show s'.ropenQ = s.ropenQ from by rw [← hs'],
          show s'.ropenCh = none from by rw [← hs'],
          show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
          show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
          show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
          show s'.rfinGot = s.rfinGot from by rw [← hs']]
        rw [hch] at htop
        simp_all
  next hch =>
    -- committed obligation: .query
    split at hstep
    case isFalse => simp at hstep
    case isTrue _hg =>
      injection hstep with hs'
      have hq : s.ropenQ < sk.rootPending := by
        have htop := hi.top
        rw [topLocalOk] at htop
        simp only [Bool.and_eq_true] at htop
        have h8 := htop.1.1.1.1.2
        rw [hch] at h8
        simp at h8
        exact h8.1.1
      refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
      · rw [← hs']; exact hi.wk pk hpk
      · rw [← hs']; exact hi.asm pk hpk
      · have htop := hi.top
        rw [topLocalOk] at htop ⊢
        rw [show s'.iopenWire = s.iopenWire from by rw [← hs'],
          show s'.iopenQuery = s.iopenQuery from by rw [← hs'],
          show s'.iopenCh = s.iopenCh from by rw [← hs'],
          show s'.ropenGotWire = s.ropenGotWire from by rw [← hs'],
          show s'.ropenWire = s.ropenWire from by rw [← hs'],
          show s'.ropenRes = s.ropenRes from by rw [← hs'],
          show s'.ropenQ = s.ropenQ + 1 from by rw [← hs'],
          show s'.ropenCh = none from by rw [← hs'],
          show s'.absorbIdx = s.absorbIdx from by rw [← hs'],
          show s'.absorbPhase = s.absorbPhase from by rw [← hs'],
          show s'.rfinGotRes = s.rfinGotRes from by rw [← hs'],
          show s'.rfinGot = s.rfinGot from by rw [← hs']]
        rw [hch] at htop
        simp_all
        omega
  next => simp at hstep

/-- Counting deltas for `.ropenFire` away from the `.wire` commitment.

Extracted from the `.res`- and `.query`-arm flow bullets of
`preserve_ropenFire` (Proofs/Preserve/Top.lean). With `hnw` the enabled
arms are `.res` (channel `Chan.rootres`) and `.query` (channel
`Chan.asked Party.R (sk.rootH - 2)`), both internal: wire channels are
untouched, and the touched channel's producer count rises by one
(`b2n ropenRes` 0→1 with freshness from `hi.top`, or `ropenQ` +1)
exactly as its occupancy does (the `chan < 1` guard supplies the
capacity bound). -/
theorem delta_ropenFire (_hwf : sk.wellFormed = true)
    (hnw : s.ropenCh ≠ some ROblig.wire)
    (hstep : Model.apply sk ax .ropenFire s = some s')
    (hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  next hch => exact absurd hch hnw
  next hch =>
    -- committed obligation: .res
    split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      injection hstep with hs'
      have hrr : s.ropenRes = false := by
        have htop := hi.top
        rw [topLocalOk] at htop
        simp only [Bool.and_eq_true] at htop
        have h7 := htop.1.1.1.1.1.2
        rw [hch] at h7
        simp at h7
        exact h7.1
      have hchan : s'.chan = bump s.chan Chan.rootres 1 := by
        rw [← hs']
      have hcap : s.chan Chan.rootres < sk.cap Chan.rootres := hg
      have hrecv : ∀ c, recvdOf sk s' c = recvdOf sk s c := by
        intro c; rw [← hs']; cases c <;> rfl
      have hsent1 : sentOf sk s' Chan.rootres
          = sentOf sk s Chan.rootres + 1 := by
        have h1 : sentOf sk s' Chan.rootres = 1 := by
          rw [← hs']; simp [sentOf, b2n]
        have h0 : sentOf sk s Chan.rootres = 0 := by
          simp [sentOf, hrr, b2n]
        rw [h1, h0]
      have hsent0 : ∀ c, c ≠ Chan.rootres →
          sentOf sk s' c = sentOf sk s c := by
        intro c hne
        rw [← hs']
        cases c <;> first | rfl | exact absurd rfl hne
      exact deltas_of_internal_send Chan.rootres rfl hchan hcap
        hsent0 hsent1 hrecv
  next hch =>
    -- committed obligation: .query
    split at hstep
    case isFalse => simp at hstep
    case isTrue hg =>
      injection hstep with hs'
      have hchan :
          s'.chan = bump s.chan (Chan.asked Party.R (sk.rootH - 2)) 1 := by
        rw [← hs']
      have hcap : s.chan (Chan.asked Party.R (sk.rootH - 2))
          < sk.cap (Chan.asked Party.R (sk.rootH - 2)) := hg
      have hrecv : ∀ c, recvdOf sk s' c = recvdOf sk s c := by
        intro c; rw [← hs']; cases c <;> rfl
      have hsent1 : sentOf sk s' (Chan.asked Party.R (sk.rootH - 2))
          = sentOf sk s (Chan.asked Party.R (sk.rootH - 2)) + 1 := by
        have h1 : sentOf sk s' (Chan.asked Party.R (sk.rootH - 2))
            = s.ropenQ + 1 := by
          rw [← hs']; simp [sentOf]
        have h0 : sentOf sk s (Chan.asked Party.R (sk.rootH - 2))
            = s.ropenQ := by
          simp [sentOf]
        rw [h1, h0]
      have hsent0 : ∀ c, c ≠ Chan.asked Party.R (sk.rootH - 2) →
          sentOf sk s' c = sentOf sk s c := by
        intro c hne
        rw [← hs']
        cases c with
        | asked p h =>
            by_cases h2 : (p == Party.R && h == sk.rootH - 2) = true
            · simp only [Bool.and_eq_true, beq_iff_eq] at h2
              exact absurd (by rw [h2.1, h2.2]) hne
            · simp [sentOf, wkQSentTot, wkQSum, h2]
        | wire p h => rfl
        | leafRequests => rfl
        | upper p h => rfl
        | lower p h => rfl
        | level p j => rfl
        | rootret => rfl
        | rootrets => rfl
        | rootres => rfl
      exact deltas_of_internal_send (Chan.asked Party.R (sk.rootH - 2))
        rfl hchan hcap hsent0 hsent1 hrecv
  next => simp at hstep

-- ========================================================= absorbRecvWire

/-- Local (`InvL`) preservation for `.absorbRecvWire`.

Extracted from `preserve_absorbRecvWire` (Proofs/Preserve/AbsorbFin.lean):
the guard-inversion prelude plus the wk/asm/top bullets; the flow
bullet (and the `wf_rootH` fact it alone needed) is dropped. -/
theorem preserveL_absorbRecvWire (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .absorbRecvWire s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨hph, _hpos⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
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

/-- Counting deltas for `.absorbRecvWire`, a wire receive.

Extracted from the flow bullet of `preserve_absorbRecvWire`
(Proofs/Preserve/AbsorbFin.lean). On `Chan.wire Party.R 0` occupancy
drops by one exactly as `absorbWireRecvd` rises from `absorbIdx` to
`absorbIdx + 1` (`wf_rootH` keeps the channel out of the root-wire
branch; the guard gives phase 0 and `chan > 0`), so the wire sum is
conserved and occupancy decreases; every other channel frames, and no
producer count moves. -/
theorem delta_absorbRecvWire (hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .absorbRecvWire s = some s')
    (_hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨hph, hpos⟩ := hg
    have hrH : (0 == sk.rootH) = false := by
      have h2 := (wf_rootH hwf).2
      have : (0 : Nat) ≠ sk.rootH := by omega
      simp [this]
    injection hstep with hs'
    have hchan : s'.chan = bump s.chan (Chan.wire Party.R 0) (-1) := by
      rw [← hs']
    have hsent : ∀ c, sentOf sk s' c = sentOf sk s c := by
      intro c; rw [← hs']; cases c <;> rfl
    have hrecv1 : recvdOf sk s' (Chan.wire Party.R 0)
        = recvdOf sk s (Chan.wire Party.R 0) + 1 := by
      have h1 : recvdOf sk s' (Chan.wire Party.R 0)
          = s.absorbIdx + 1 := by
        rw [← hs']; simp [recvdOf, absorbWireRecvd, hrH]
      have h0 : recvdOf sk s (Chan.wire Party.R 0) = s.absorbIdx := by
        simp [recvdOf, absorbWireRecvd, hrH, hph]
      rw [h1, h0]
    have hrecv0 : ∀ c, c ≠ Chan.wire Party.R 0 →
        recvdOf sk s' c = recvdOf sk s c := by
      intro c hne
      rw [← hs']
      cases c with
      | wire p h =>
          cases p with
          | I => rfl
          | R =>
              have hh : h ≠ 0 := fun h0 => hne (by rw [h0])
              have h0 : (h == 0) = false := by simp [hh]
              simp [recvdOf, wkWireRecvd, h0]
      | leafRequests => simp [recvdOf, absorbAskedRecvd, hph]
      | _ => rfl
    exact deltas_of_wire_recv Party.R 0 hchan hpos hsent hrecv0 hrecv1

-- ======================================================== absorbRecvAsked

/-- Local (`InvL`) preservation for `.absorbRecvAsked`.

Extracted from `preserve_absorbRecvAsked`
(Proofs/Preserve/AbsorbFin.lean): the guard-inversion prelude plus the
wk/asm/top bullets; the flow bullet is dropped. -/
theorem preserveL_absorbRecvAsked (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .absorbRecvAsked s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨hph, _hpos⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
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

/-- Counting deltas for `.absorbRecvAsked`, an internal receive.

Extracted from the flow bullet of `preserve_absorbRecvAsked`
(Proofs/Preserve/AbsorbFin.lean). On `Chan.leafRequests` occupancy
drops by one exactly as `absorbAskedRecvd` rises from `absorbIdx` to
`absorbIdx + 1` (the guard gives phase 1 and `chan > 0`);
`absorbWireRecvd` reads `absorbIdx + 1` in both phases, so every wire
channel frames, and no producer count moves. -/
theorem delta_absorbRecvAsked (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .absorbRecvAsked s = some s')
    (_hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨hph, hpos⟩ := hg
    injection hstep with hs'
    have hchan : s'.chan = bump s.chan Chan.leafRequests (-1) := by
      rw [← hs']
    have hsent : ∀ c, sentOf sk s' c = sentOf sk s c := by
      intro c; rw [← hs']; cases c <;> rfl
    have hrecv1 : recvdOf sk s' Chan.leafRequests
        = recvdOf sk s Chan.leafRequests + 1 := by
      have h1 : recvdOf sk s' Chan.leafRequests = s.absorbIdx + 1 := by
        rw [← hs']; simp [recvdOf, absorbAskedRecvd]
      have h0 : recvdOf sk s Chan.leafRequests = s.absorbIdx := by
        simp [recvdOf, absorbAskedRecvd, hph]
      rw [h1, h0]
    have hrecv0 : ∀ c, c ≠ Chan.leafRequests →
        recvdOf sk s' c = recvdOf sk s c := by
      intro c hne
      rw [← hs']
      cases c with
      | wire p h => simp [recvdOf, wkWireRecvd, absorbWireRecvd, hph]
      | leafRequests => exact absurd rfl hne
      | _ => rfl
    exact deltas_of_internal_recv Chan.leafRequests rfl hchan hpos
      hsent hrecv0 hrecv1

-- ============================================================= absorbSend

/-- Local (`InvL`) preservation for `.absorbSend`.

Extracted from `preserve_absorbSend` (Proofs/Preserve/AbsorbFin.lean):
the guard-inversion prelude, the cursor bound `hidx` (which reads only
`hi.top`), and the wk/asm/top bullets; the flow bullet and its frame
machinery are dropped. -/
theorem preserveL_absorbSend (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .absorbSend s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨hph, _hlt⟩ := hg
    have hidx : s.absorbIdx < sk.totalLeafReqs := by
      have htop := hi.top
      rw [topLocalOk] at htop
      simp only [Bool.and_eq_true] at htop
      have h9 := htop.1.1.1.2
      rw [hph] at h9
      simpa using h9
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
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

/-- Counting deltas for `.absorbSend`, an internal send.

Extracted from the flow bullet of `preserve_absorbSend`
(Proofs/Preserve/AbsorbFin.lean). The send lands on the internal
channel `Chan.level Party.I 0`, whose producer count `absorbIdx` rises
by one exactly as its occupancy does (the `chan < cap` guard supplies
the capacity bound); both absorb consumer counts stay constant across
the phase move (the failed guard and the cursor bound from `hi.top` pin
the closing phase's `totalLeafReqs` to `absorbIdx + 1`), so every wire
channel frames. -/
theorem delta_absorbSend (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .absorbSend s = some s')
    (hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
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
    injection hstep with hs'
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
    have hrecv : ∀ c, recvdOf sk s' c = recvdOf sk s c := by
      intro c
      cases c <;>
        simp [recvdOf, wkWireRecvd, wkAskedRecvd, asmResRecvd,
          asmLevelRecvd, habsW, habsA, hwalk, hasm, hgw, hifin, hrg, hrgr]
    have hsent0 : ∀ c, c ≠ Chan.level Party.I 0 →
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
    have hchan : s'.chan = bump s.chan (Chan.level Party.I 0) 1 := by
      rw [← hs']
    have hsent1 : sentOf sk s' (Chan.level Party.I 0)
        = sentOf sk s (Chan.level Party.I 0) + 1 := by
      have hs1 : sentOf sk s' (Chan.level Party.I 0)
          = s.absorbIdx + 1 := by
        rw [← hs']; simp [sentOf]
      have hs0 : sentOf sk s (Chan.level Party.I 0) = s.absorbIdx := by
        simp [sentOf]
      rw [hs1, hs0]
    exact deltas_of_internal_send (Chan.level Party.I 0) rfl hchan hlt
      hsent0 hsent1 hrecv

-- ======================================================== absorbCloseWire

/-- Local (`InvL`) preservation for `.absorbCloseWire`.

Extracted from `preserve_absorbCloseWire`
(Proofs/Preserve/AbsorbFin.lean): the guard-inversion prelude plus the
wk/asm/top bullets; the flow bullet is dropped. -/
theorem preserveL_absorbCloseWire (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .absorbCloseWire s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨hph, _hprod⟩, _hzero⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
      rw [topLocalOk] at htop ⊢
      rw [show s'.absorbPhase = 4 from by rw [← hs'],
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

/-- Counting deltas for `.absorbCloseWire`: a phase move, nothing flows.

Extracted from the flow bullet of `preserve_absorbCloseWire`
(Proofs/Preserve/AbsorbFin.lean): no channel or cursor changes, and
both absorb recvd counts read `phase ≥ 3` on either side of the 3→4
move, so every channel frames. -/
theorem delta_absorbCloseWire (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .absorbCloseWire s = some s')
    (_hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨hph, _hprod⟩, _hzero⟩ := hg
    injection hstep with hs'
    have hchan : s'.chan = s.chan := by rw [← hs']
    have hsent : ∀ c, sentOf sk s' c = sentOf sk s c := by
      intro c; rw [← hs']; cases c <;> rfl
    have hrecv : ∀ c, recvdOf sk s' c = recvdOf sk s c := by
      intro c
      rw [← hs']
      cases c <;>
        simp [recvdOf, wkWireRecvd, wkAskedRecvd, asmResRecvd,
          asmLevelRecvd, absorbWireRecvd, absorbAskedRecvd, hph]
    exact deltas_of_frame hchan hsent hrecv

-- ======================================================= absorbCloseAsked

/-- Local (`InvL`) preservation for `.absorbCloseAsked`.

Extracted from `preserve_absorbCloseAsked`
(Proofs/Preserve/AbsorbFin.lean): the guard-inversion prelude plus the
wk/asm/top bullets; the flow bullet is dropped. -/
theorem preserveL_absorbCloseAsked (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .absorbCloseAsked s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨hph, _hprod⟩, _hzero⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · have htop := hi.top
      rw [topLocalOk] at htop ⊢
      rw [show s'.absorbPhase = 5 from by rw [← hs'],
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

/-- Counting deltas for `.absorbCloseAsked`: a phase move, nothing flows.

Extracted from the flow bullet of `preserve_absorbCloseAsked`
(Proofs/Preserve/AbsorbFin.lean): no channel or cursor changes, and
both absorb recvd counts read `phase ≥ 3` on either side of the 4→5
move, so every channel frames. -/
theorem delta_absorbCloseAsked (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .absorbCloseAsked s = some s')
    (_hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨hph, _hprod⟩, _hzero⟩ := hg
    injection hstep with hs'
    have hchan : s'.chan = s.chan := by rw [← hs']
    have hsent : ∀ c, sentOf sk s' c = sentOf sk s c := by
      intro c; rw [← hs']; cases c <;> rfl
    have hrecv : ∀ c, recvdOf sk s' c = recvdOf sk s c := by
      intro c
      rw [← hs']
      cases c <;>
        simp [recvdOf, wkWireRecvd, wkAskedRecvd, asmResRecvd,
          asmLevelRecvd, absorbWireRecvd, absorbAskedRecvd, hph]
    exact deltas_of_frame hchan hsent hrecv

-- ================================================================= finRet

/-- Local (`InvL`) preservation for `.finRet`.

Extracted from `preserve_finRet` (Proofs/Preserve/AbsorbFin.lean): the
guard-inversion prelude plus the wk/asm/top bullets (all three are pure
frames — `topLocalOk` reads neither `ifin` nor `chan`); the flow bullet
is dropped. -/
theorem preserveL_finRet (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .finRet s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue _hg =>
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
    · rw [← hs']; exact hi.wk pk hpk
    · rw [← hs']; exact hi.asm pk hpk
    · rw [← hs']; exact hi.top

/-- Counting deltas for `.finRet`, an internal receive.

Extracted from the flow bullet of `preserve_finRet`
(Proofs/Preserve/AbsorbFin.lean). On `Chan.rootret` occupancy drops by
one exactly as `b2n ifin` rises 0→1 (the guard gives both the
freshness and `chan > 0`); every wire channel frames, and no producer
count moves. -/
theorem delta_finRet (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .finRet s = some s')
    (_hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_true_eq] at hg
    obtain ⟨hifin, hpos⟩ := hg
    injection hstep with hs'
    have hchan : s'.chan = bump s.chan Chan.rootret (-1) := by rw [← hs']
    have hsent : ∀ c, sentOf sk s' c = sentOf sk s c := by
      intro c; rw [← hs']; cases c <;> rfl
    have hrecv1 : recvdOf sk s' Chan.rootret
        = recvdOf sk s Chan.rootret + 1 := by
      have h1 : recvdOf sk s' Chan.rootret = 1 := by
        rw [← hs']; simp [recvdOf, b2n]
      have h0 : recvdOf sk s Chan.rootret = 0 := by
        simp [recvdOf, b2n, hifin]
      rw [h1, h0]
    have hrecv0 : ∀ c, c ≠ Chan.rootret →
        recvdOf sk s' c = recvdOf sk s c := by
      intro c hne
      rw [← hs']
      cases c <;> first | rfl | exact absurd rfl hne
    exact deltas_of_internal_recv Chan.rootret rfl hchan hpos hsent
      hrecv0 hrecv1

-- ================================================================= finRes

/-- Local (`InvL`) preservation for `.finRes`.

Extracted from `preserve_finRes` (Proofs/Preserve/AbsorbFin.lean): the
guard-inversion prelude plus the wk/asm/top bullets; the flow bullet is
dropped. -/
theorem preserveL_finRes (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .finRes s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue _hg =>
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
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

/-- Counting deltas for `.finRes`, an internal receive.

Extracted from the flow bullet of `preserve_finRes`
(Proofs/Preserve/AbsorbFin.lean). On `Chan.rootres` occupancy drops by
one exactly as `b2n rfinGotRes` rises 0→1 (the guard gives both the
freshness and `chan > 0`); every wire channel frames, and no producer
count moves. -/
theorem delta_finRes (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .finRes s = some s')
    (_hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_true_eq] at hg
    obtain ⟨hres, hpos⟩ := hg
    injection hstep with hs'
    have hchan : s'.chan = bump s.chan Chan.rootres (-1) := by rw [← hs']
    have hsent : ∀ c, sentOf sk s' c = sentOf sk s c := by
      intro c; rw [← hs']; cases c <;> rfl
    have hrecv1 : recvdOf sk s' Chan.rootres
        = recvdOf sk s Chan.rootres + 1 := by
      have h1 : recvdOf sk s' Chan.rootres = 1 := by
        rw [← hs']; simp [recvdOf, b2n]
      have h0 : recvdOf sk s Chan.rootres = 0 := by
        simp [recvdOf, b2n, hres]
      rw [h1, h0]
    have hrecv0 : ∀ c, c ≠ Chan.rootres →
        recvdOf sk s' c = recvdOf sk s c := by
      intro c hne
      rw [← hs']
      cases c <;> first | rfl | exact absurd rfl hne
    exact deltas_of_internal_recv Chan.rootres rfl hchan hpos hsent
      hrecv0 hrecv1

-- ================================================================ finRets

/-- Local (`InvL`) preservation for `.finRets`.

Extracted from `preserve_finRets` (Proofs/Preserve/AbsorbFin.lean): the
guard-inversion prelude plus the wk/asm/top bullets; the flow bullet is
dropped. -/
theorem preserveL_finRets (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .finRets s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, decide_eq_true_eq] at hg
    obtain ⟨⟨hres, hlt⟩, _hpos⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩
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

/-- Counting deltas for `.finRets`, an internal receive.

Extracted from the flow bullet of `preserve_finRets`
(Proofs/Preserve/AbsorbFin.lean). On `Chan.rootrets` occupancy drops by
one exactly as the consumer count `rfinGot` rises by one (the guard
gives `chan > 0`); every wire channel frames, and no producer count
moves. -/
theorem delta_finRets (_hwf : sk.wellFormed = true)
    (hstep : Model.apply sk ax .finRets s = some s')
    (_hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, decide_eq_true_eq] at hg
    obtain ⟨⟨_hres, _hlt⟩, hpos⟩ := hg
    injection hstep with hs'
    have hchan : s'.chan = bump s.chan Chan.rootrets (-1) := by rw [← hs']
    have hsent : ∀ c, sentOf sk s' c = sentOf sk s c := by
      intro c; rw [← hs']; cases c <;> rfl
    have hrecv1 : recvdOf sk s' Chan.rootrets
        = recvdOf sk s Chan.rootrets + 1 := by
      have h1 : recvdOf sk s' Chan.rootrets = s.rfinGot + 1 := by
        rw [← hs']; simp [recvdOf]
      have h0 : recvdOf sk s Chan.rootrets = s.rfinGot := rfl
      rw [h1, h0]
    have hrecv0 : ∀ c, c ≠ Chan.rootrets →
        recvdOf sk s' c = recvdOf sk s c := by
      intro c hne
      rw [← hs']
      cases c <;> first | rfl | exact absurd rfl hne
    exact deltas_of_internal_recv Chan.rootrets rfl hchan hpos hsent
      hrecv0 hrecv1

end StreamingMirror.Mux
