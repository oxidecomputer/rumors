/-
T8 (`sigmaStarK_deadlock_free`): the window-generalized liveness of
the single-connection transport, transcribing T8-SPEC.md — for every
well-formed margin-0 skeleton (clause 1), every capacity C ≥ 1
(clause 2), every pair of advertised depths K_I, K_R ≥ 1, independent
per direction (clause 3), and EVERY pair of window-disciplined
strategies (clause 4, the class quantification — the licensing
predicate causal per clause 5), the K-deep composition cannot deadlock;
with `mux_terminatingK` (Mux/Proofs/Termination.lean) every run also
ends within `2·ρ(init)` steps, so "completes" is kernel-honest
(clause 6 — both conjuncts, per the phase-4 F5 mint).

# The proof, refute-c1 §2 at arrears K

- **The stuck bridge** (`mstuck_of_mstuckK`): a K-stuck state is
  record-stuck for the same pair — base and push arms are shared, and
  a depth-blocked cell (`chan ≥ recvDepth ≥ 1`) blocks the record
  slot a fortiori. Every landed stuck-decode lemma then applies
  verbatim, keystone and chase included.
- **Step 1** (`windowK_pipes_empty`): at a K-stuck state the pipes are
  empty. A parked head means its cell is exactly full
  (`chan = recvDepth`, the deliver guard against the parking bound),
  so the head's push-time arrears-K certificate (`PushProvenAK`, the
  class's GATE conjunct made inductive) names `rcv(c, recvdOf)` — and
  the causal keystone performs it, against `recvdOf < recvdOf`. This
  is where the arrears form is forced (Mux/SigmaStarK.lean's module
  doc): the certificate meets the full cell exactly.
- **Steps 2–3**: the landed chase, at the K bound (it consumes the
  ground facts bound-free).
- **Step 4** (`stuck_coverage_arrears`, Mux/Proofs/CausalMint.lean):
  the withheld frame is licensed at the withholding party's advertised
  arrears — inside the free window outright, else its arrears-K
  predecessor receive enters the announced closure by the minting
  ladder.
- **The class step**: the withheld frame makes the licensed set
  nonempty, the class's PROGRESS conjunct makes σ name SOME licensed
  frame, its GATE makes that frame held — a `WithheldPush` the stuck
  strategy provably declined (`mstuck_withheld`). One line per
  conjunct: this is why clause 4's quantification costs the proof
  nothing beyond existence, and why no concrete-scheduler drift is
  possible — no selector is ever consulted.

# Boundaries

Model-tier, reply-denominated, single conforming session: T8-SPEC.md's
boundary section is the honest reading; the byte caveat of record is
Mux/Basic.lean's module doc (# The byte-denomination caveat).
-/
import StreamingMirror.Mux.Proofs.SigmaStarKInv
import StreamingMirror.Mux.Proofs.CausalMint
import StreamingMirror.Mux.Proofs.Termination

namespace StreamingMirror.Mux

open Model
open Sched (Ev scheduleE performed evIdx)

variable {sk : Skel}

-- ====================================================== the stuck bridge

/-- `deliver p` is in the enumeration (local copy of the Elastic
device). -/
private theorem deliverK_mem_allMActions (sk : Skel) (p : Party) :
    MAction.deliver p ∈ allMActions sk := by
  rw [allMActions]
  refine List.mem_append.mpr (.inr ?_)
  cases p <;> simp

/-- A K-deep delivery fires whenever the pipe is nonempty and the head
frame's cell is below the receiving party's depth. -/
private theorem deliverStepK_isSome {KI KR : Nat} {p : Party}
    {s : MState} {c : Chan} {rest : List Chan}
    (hp : s.pipe p = c :: rest)
    (hlt : s.base.chan c < recvDepth KI KR p) :
    (deliverStepK KI KR p s).isSome = true := by
  unfold deliverStepK
  split
  next c' rest' hp' =>
    have hcc : c' = c ∧ rest' = rest := by
      rw [hp] at hp'
      injection hp' with h1 h2
      exact ⟨h1.symm, h2.symm⟩
    obtain ⟨rfl, rfl⟩ := hcc
    rw [if_pos hlt]
    rfl
  next hp' =>
    rw [hp] at hp'
    cases hp'

/-- A K-stuck state is record-stuck for the same strategies and
capacity: base and push arms are definitionally shared, and a
depth-blocked parked cell (`chan ≥ recvDepth ≥ 1`) blocks the record
cap-1 slot a fortiori. The master key of the K liveness argument —
every landed `mstuck` decode (the choice-point lemmas, the keystone,
the chase, `mstuck_withheld`) applies to K-stuck states through it. -/
theorem mstuck_of_mstuckK {KI KR : Nat} (hKI : 1 ≤ KI) (hKR : 1 ≤ KR)
    {ax : AxMode} {C : Nat} {σI σR : Strategy} {s : MState}
    (hst : mstuckK sk ax KI KR C σI σR s = true) :
    mstuck sk ax C σI σR s = true := by
  rw [mstuckK, Bool.and_eq_true, Bool.not_eq_true',
    Bool.not_eq_true'] at hst
  obtain ⟨hnt, hcan⟩ := hst
  rw [mcanStepK, List.any_eq_false] at hcan
  rw [mstuck, hnt, Bool.not_false, Bool.true_and, Bool.not_eq_true',
    mcanStep, List.any_eq_false]
  intro a ha
  have hK := hcan a ha
  cases a with
  | base a => exact hK
  | push p => exact hK
  | deliver p =>
      intro hsome
      have hK' : ¬ (deliverStepK KI KR p s).isSome = true := hK
      simp only [apply] at hsome
      split at hsome
      next c rest hp =>
          split at hsome
          case isFalse => simp at hsome
          case isTrue h0 =>
            have hz : s.base.chan c = 0 := by simpa using h0
            refine hK' (deliverStepK_isSome hp ?_)
            have h1 : 1 ≤ recvDepth KI KR p := by
              cases p
              · exact hKR
              · exact hKI
            omega
      next => simp at hsome

/-- At a K-stuck state a nonempty pipe's head is depth-blocked: the
receiving party's parked cells are full — the K analog of
`mstuck_deliver_blocked`, and with the parking bound it pins
`chan = recvDepth` exactly. -/
theorem mstuckK_deliver_blocked {KI KR C : Nat} {σI σR : Strategy}
    {s : MState}
    (hst : mstuckK sk .impl KI KR C σI σR s = true)
    {p : Party} {c : Chan} {rest : List Chan}
    (hp : s.pipe p = c :: rest) :
    ¬ s.base.chan c < recvDepth KI KR p := by
  intro hlt
  rw [mstuckK, Bool.and_eq_true, Bool.not_eq_true',
    Bool.not_eq_true'] at hst
  have hcan := hst.2
  rw [mcanStepK, List.any_eq_false] at hcan
  have hd := hcan (.deliver p) (deliverK_mem_allMActions sk p)
  have hd' : ¬ (deliverStepK KI KR p s).isSome = true := hd
  exact hd' (deliverStepK_isSome hp hlt)

-- =========================================================== Step 1 at K

/-- Step 1 at the K grain: at a K-stuck state of any window-disciplined
pair, both pipes are empty. The parked head's cell is exactly full
(guard against bound), so its push-time arrears-K certificate names
`rcv(c, recvdOf)` — which the causal keystone performs at the bridged
record-stuck state, an outright contradiction. -/
theorem windowK_pipes_empty {KI KR : Nat} (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    (hKI : 1 ≤ KI) (hKR : 1 ≤ KR) {C : Nat} {σI σR : Strategy}
    {s : MState}
    (hm : SInvK KI KR sk s) (hrl : RecvLedger sk s)
    (hpp : PushProvenAK KI KR sk s)
    (hstuck : mstuckK sk .impl KI KR C σI σR s = true)
    (p : Party) : s.pipe p = [] := by
  cases hp : s.pipe p with
  | nil => rfl
  | cons c rest =>
      exfalso
      obtain ⟨g, rfl⟩ := hm.mux.pipe_mem_wire (p := p) (c := c)
        (by rw [hp]; exact List.mem_cons_self ..)
      -- the parked cell is exactly full
      have hblock : ¬ s.base.chan (Chan.wire p g) < recvDepth KI KR p :=
        mstuckK_deliver_blocked hstuck hp
      have hstuck' : mstuck sk .impl C σI σR s = true :=
        mstuck_of_mstuckK hKI hKR hstuck
      -- the head frame's position in the push order
      have hpipe := hm.mux.hist_pipe p
      rw [hp] at hpipe
      have hget : (pushHeights (s.hist p))[delTotal (s.hist p.other)]?
          = some g := by
        have h1 : ((pushHeights (s.hist p)).drop
            (delTotal (s.hist p.other)))[0]? = some g := by
          have h2 := congrArg (List.map wireHeight) hpipe
          rw [List.map_map,
            show wireHeight ∘ Chan.wire p = id from rfl,
            List.map_id] at h2
          rw [← h2]
          rfl
        rw [List.getElem?_drop] at h1
        simpa using h1
      have hmem_ch : Chan.wire p g ∈ allChans sk := by
        refine hm.mux.pushed_mem p g ?_
        intro hcz
        have : g ∈ pushHeights (s.hist p) :=
          List.mem_of_getElem? hget
        rw [pushedCount] at hcz
        exact absurd (List.count_pos_iff.mpr this) (by omega)
      have hpark := hm.mux.slot (Chan.wire p g) hmem_ch
      have hpark' : s.base.chan (Chan.wire p g) ≤ recvDepth KI KR p :=
        hpark
      obtain ⟨i₀, a₀, hi₀, hfa₀, htake₀⟩ := filterMapA_take_index _
        (s.hist p) (delTotal (s.hist p.other)) g hget
      have ha₀ : a₀ = .pushed g := pushedA_of_extract hfa₀
      subst ha₀
      have hkeq : pushedCount ((s.hist p).take i₀) g
          = deliveredCount (s.hist p.other) g := by
        rw [pushedCount, deliveredCount, hm.mux.hist_del p]
        show ((s.hist p).take i₀ |>.filterMap _).count g = _
        rw [htake₀]
        rfl
      have hslot := hm.mux.delivered_eq p g hmem_ch
      -- the push-time count clears the free window exactly
      have hKguard : recvDepth KI KR p
          ≤ pushedCount ((s.hist p).take i₀) g := by
        omega
      have hcert := hpp p i₀ g hi₀ hKguard
      -- the causal keystone performs the certified receive
      have htkpre : (s.hist p).take i₀ <+: s.hist p :=
        List.take_prefix i₀ (s.hist p)
      have hperf := keystoneA hwf hm0 hm.mux hstuck' p
        ((s.hist p).take i₀)
        (hm.mux.pushtime_delivered p htake₀)
        (fun h' => deliveredCount_le_of_prefix htkpre h')
        (fun h' => by
          by_cases hz' : ownRecvs sk.rootH p ((s.hist p).take i₀) h' = 0
          · rw [hz']
            exact Nat.zero_le _
          · have hle := ownRecvs_le_of_prefix
              (rootH := sk.rootH) (p := p) htkpre h'
            have hmem := hrl.mem p h' (by omega)
            have := hrl.bound p h' hmem
            omega)
        (fun h' hz' => hm.mux.pushed_mem p h' (fun hc => hz'
          (Nat.le_antisymm
            (hc ▸ pushedCount_le_of_prefix htkpre h') (Nat.zero_le _))))
        (fun h' hz' => by
          have hle := deliveredCount_le_of_prefix htkpre h'
          have hlp := hm.mux.delivered_le_pushed p.other h'
          rw [Party.other_other] at hlp
          refine hm.mux.pushed_mem p.other h' ?_
          omega)
        _ hcert
      rw [performed_rcv_iff] at hperf
      omega

-- ================================================================== T8

/-- **T8** (`sigmaStarK_deadlock_free`, T8-SPEC.md's claim): for every
pair of trees the protocol can synchronize at all — every well-formed
margin-0 skeleton — every single-channel capacity C ≥ 1, every pair of
advertised window depths K_I, K_R ≥ 1 (independent, possibly unequal),
and EVERY pair of window-disciplined strategies (each gated at its
PEER's advertised depth: any selection rule among licensed frames,
licensing causal — the announced closure at parking arrears K), the
K-deep single-connection composition cannot deadlock.

With `mux_terminatingK` (every K run ends within `2·ρ(init)` steps)
and `muxK_greedy_run_terminal`, "always completes" is kernel-honest:
no reachable stuck state, no infinite run, greedy drains reach
`mterminal`. The canonical pair is `sigmaStarK_pair_deadlock_free`;
the shipped ladder pair is inside the class by
`sigmaLadderK_windowDisciplined`. Rests on message-denominated
liveness; the W = 1 byte caveat of record is Mux/Basic.lean's module
doc (# The byte-denomination caveat). -/
theorem sigmaStarK_deadlock_free (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    (KI KR : Nat) (hKI : 1 ≤ KI) (hKR : 1 ≤ KR)
    (C : Nat) (hC : 1 ≤ C) {σI σR : Strategy}
    (hWI : WindowDisciplined KR .I σI)
    (hWR : WindowDisciplined KI .R σR) :
    MuxDeadlockFreeK sk .impl KI KR C σI σR := by
  intro s hr
  cases hst : mstuckK sk .impl KI KR C σI σR s with
  | false => rfl
  | true =>
      exfalso
      have hm := sinvK_reachable hwf hKI hKR hr
      have hrl := recvLedger_reachableK hwf hKI hKR hr
      have hpp := pushProvenAK_reachable hwf hKI hKR hWI hWR hr
      -- Step 1: the pipes drain
      have hpI : s.pipe .I = [] :=
        windowK_pipes_empty hwf hm0 hKI hKR hm hrl hpp hst .I
      have hpR : s.pipe .R = [] :=
        windowK_pipes_empty hwf hm0 hKI hKR hm hrl hpp hst .R
      have hstuck : mstuck sk .impl C σI σR s = true :=
        mstuck_of_mstuckK hKI hKR hst
      have hnt : mterminal sk s = false := by
        rw [mstuckK, Bool.and_eq_true, Bool.not_eq_true'] at hst
        exact hst.1
      -- Steps 2–3: the τ-least withheld push, everything below performed
      obtain ⟨f, a, p, hh, hfc, hfb, hfseq, hfsched, hfnp, hleast,
        hcover, hpend, hok, hhold⟩ :=
        chase hwf hm0 hm.mux hpI hpR hstuck hnt
      obtain ⟨c', b', n'⟩ := f
      simp only at hfc hfb hfseq
      subst hfc
      subst hfb
      subst hfseq
      have hhold' : holdsWire sk p hh s.base = true := by
        rw [holdsWire.eq_def] at hhold ⊢
        exact hhold
      have hcm : committedInHist sk.rootH (s.hist p) hh = true := by
        rw [committedInHist_iff_holdsWire hm.hist]
        exact hhold'
      have hKp1 : 1 ≤ recvDepth KI KR p := by
        cases p
        · exact hKR
        · exact hKI
      -- Step 4: the withheld frame is licensed at the advertised arrears
      have hdem : demandedAK (recvDepth KI KR p)
          (aviewOf sk p (s.hist p)) (s.hist p) hh = true := by
        rw [demandedAK, Bool.or_eq_true]
        by_cases hfree : pushedCount (s.hist p) hh < recvDepth KI KR p
        · exact Or.inl (decide_eq_true hfree)
        · refine Or.inr ((List.contains_iff_mem ..).mpr ?_)
          exact stuck_coverage_arrears hwf hm0 hm.mux hstuck hpI hpR
            p hh hhold' hcover (recvDepth KI KR p) hKp1 (by omega)
      have hlic : hh ∈ licensedK sk (recvDepth KI KR p) p (s.hist p) := by
        refine List.mem_filter.mpr ⟨?_, ?_⟩
        · show hh ∈ wireHeightsA (aviewOf sk p (s.hist p))
            (aviewOf sk p (s.hist p)).party
          rw [wireHeightsA_aviewOf]
          exact holdsWire_mem_wireHeights hhold'
        · rw [Bool.and_eq_true]
          exact ⟨hcm, hdem⟩
      -- the class step: progress names a licensed frame, the gate holds it
      have hKC : KConsistentAny p sk (s.hist p) :=
        ⟨.impl, KI, KR, C, σI, σR, s, hr, rfl⟩
      obtain ⟨h', hσ', hlic'⟩ : ∃ h',
          (match p with | .I => σI | .R => σR) sk (s.hist p) = some h'
          ∧ h' ∈ licensedK sk (recvDepth KI KR p) p (s.hist p) := by
        cases p with
        | I =>
            have hcls := hWI sk (s.hist .I) hKC
            obtain ⟨h', hh'⟩ := Option.isSome_iff_exists.mp
              (hcls.2 (List.ne_nil_of_mem hlic))
            exact ⟨h', hh', hcls.1 h' hh'⟩
        | R =>
            have hcls := hWR sk (s.hist .R) hKC
            obtain ⟨h', hh'⟩ := Option.isSome_iff_exists.mp
              (hcls.2 (List.ne_nil_of_mem hlic))
            exact ⟨h', hh', hcls.1 h' hh'⟩
      obtain ⟨-, hpred'⟩ := List.mem_filter.mp hlic'
      rw [Bool.and_eq_true] at hpred'
      have hcm' : committedInHist sk.rootH (s.hist p) h' = true :=
        hpred'.1
      have hwp : WithheldPush sk C p h' s := by
        refine ⟨?_, ?_⟩
        · rw [← committedInHist_iff_holdsWire hm.hist]
          exact hcm'
        · have hempty : s.pipe p = [] := by
            cases p
            · exact hpI
            · exact hpR
          rw [hempty]
          simp only [List.length_nil]
          omega
      exact mstuck_withheld hstuck hwp hσ'

/-- The canonical instantiation (the C1.lean stub's intended positive
theorem, upgraded to a corollary of the class form): the σ*ₖ pair —
each side's lookahead the depth its PEER's demux parks — is
deadlock-free at every depth pair and capacity. -/
theorem sigmaStarK_pair_deadlock_free (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    (KI KR : Nat) (hKI : 1 ≤ KI) (hKR : 1 ≤ KR)
    (C : Nat) (hC : 1 ≤ C) :
    MuxDeadlockFreeK sk .impl KI KR C (sigmaStarK KR) (sigmaStarK KI) :=
  sigmaStarK_deadlock_free hwf hm0 KI KR hKI hKR C hC
    (sigmaStarK_windowDisciplined KR .I)
    (sigmaStarK_windowDisciplined KI .R)

/-- T8's "completes", assembled: under any window-disciplined pair the
greedy K drain reaches `mterminal` within `2·ρ(init)` steps —
progress (`sigmaStarK_deadlock_free`) and bounded termination
(`mux_terminatingK`/`mdrainK_quiescent`) in one kernel fact, clause
6's two conjuncts. Message-denominated (Mux/Basic.lean, # The
byte-denomination caveat). -/
theorem sigmaStarK_completes (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    (KI KR : Nat) (hKI : 1 ≤ KI) (hKR : 1 ≤ KR)
    (C : Nat) (hC : 1 ≤ C) {σI σR : Strategy}
    (hWI : WindowDisciplined KR .I σI)
    (hWR : WindowDisciplined KI .R σR) :
    mterminal sk
      (mdrainK sk .impl KI KR C σI σR (2 * rho sk (Model.init sk))
        (init sk)) = true :=
  muxK_greedy_run_terminal
    (sigmaStarK_deadlock_free hwf hm0 KI KR hKI hKR C hC hWI hWR)

end StreamingMirror.Mux
