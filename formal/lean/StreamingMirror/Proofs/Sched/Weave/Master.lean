/-
Layer D's master induction, the consumption half (PROGRESS.md §7 3b):
the pointwise emission-readiness property `EmitOKOn` of the weave's
ghost future, and the fuel induction that rides it through the
interpreter to `WEdge sk [] (weaveState sk)`.

# Shape

`wEdge_emit` wants `enabled` at every manual emission. Everything a
guard consults is determined by the REMAINING future: the counting
invariant pins each owned count to its whole-trace total minus the
future's share (`count_pin`), so a site's enabledness is a property
of the future's filter shapes — a pure list property. `EmitOKOn l
rest` states it pointwise: at every position of `l`, the event is
emittable from ANY state that satisfies `WEdge` over the position's
suffix (with `rest` glued after `l`), sits at a pump fixpoint, and
holds the event's `manDep` predecessor in its output (supplied at
consumption time by the precedence layer's `DepOK`).

The fuel induction (`weaveGo_wedge`) consumes the property one
emission at a time, exactly as `weaveGo_preserves` consumes
`WCount`: pump steps are free (`wEdge_step`), and each manual
emission discharges its guard from the property's head, `DepOK`'s
head, and the pump fixpoint the previous `wEmitP` left behind. The
one state the interpreter ever emits from that is NOT a pump
fixpoint is `weaveInit`, whose first emission is iopen's seq-0
opening wire — `weaveState_wedge_of_emitOK` peels it by hand with
`enabled_snd_low` before entering the induction.

Establishing `EmitOKOn` over the opening worklist — the tree
induction threading the rolling ancestor context through the scope
recursion — is the production half (see the RestCtx sections below).
-/
import StreamingMirror.Proofs.Sched.Weave.Site

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ============================================ the pointwise property

/-- Pointwise emission-readiness of a future against a glued tail.

At every position of `l`, the event is emittable from any state that
satisfies the edge invariant over the position's suffix (with `rest`
appended), sits at a pump fixpoint, and has the event's manual
predecessor already emitted. -/
def EmitOKOn (l rest : List Ev) : Prop :=
  ∀ n e, l[n]? = some e →
    ∀ st : MState, WEdge sk (l.drop n ++ rest) st →
      step sk st = none →
      (∀ d, manDep e = some d → d ∈ st.out) →
      enabled sk st.sent st.rcvd e = true

theorem emitOKOn_nil (rest : List Ev) : EmitOKOn sk [] rest := by
  intro n e h
  simp at h

/-- Extend readiness by one head whose own discharge is supplied. -/
theorem emitOKOn_cons {e : Ev} {l rest : List Ev}
    (hhead : ∀ st : MState, WEdge sk (e :: (l ++ rest)) st →
      step sk st = none →
      (∀ d, manDep e = some d → d ∈ st.out) →
      enabled sk st.sent st.rcvd e = true)
    (htail : EmitOKOn sk l rest) : EmitOKOn sk (e :: l) rest := by
  intro n e' hn st hW hfix hpred
  match n with
  | 0 =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hn
      subst hn
      exact hhead st hW hfix hpred
  | n + 1 =>
      simp only [List.getElem?_cons_succ] at hn
      exact htail n e' hn st hW hfix hpred

/-- Consuming the head keeps the readiness of the tail. -/
theorem emitOKOn_tail {e : Ev} {l rest : List Ev}
    (h : EmitOKOn sk (e :: l) rest) : EmitOKOn sk l rest :=
  fun n e' hn st hW hfix hpred =>
    h (n + 1) e' (by simpa using hn) st hW hfix hpred

/-- Glue readiness: the left part sees the right as its tail. -/
theorem emitOKOn_append {A B rest : List Ev}
    (hA : EmitOKOn sk A (B ++ rest)) (hB : EmitOKOn sk B rest) :
    EmitOKOn sk (A ++ B) rest := by
  intro n e hn st hW hfix hpred
  rcases Nat.lt_or_ge n A.length with hlt | hge
  · rw [List.getElem?_append_left hlt] at hn
    refine hA n e hn st ?_ hfix hpred
    rwa [List.drop_append_of_le_length (Nat.le_of_lt hlt),
      List.append_assoc] at hW
  · rw [List.getElem?_append_right hge] at hn
    refine hB (n - A.length) e hn st ?_ hfix hpred
    rw [show n = A.length + (n - A.length) from by omega,
      List.drop_append] at hW
    rwa [List.drop_eq_nil_of_le (by omega), Nat.add_sub_cancel_left,
      List.nil_append] at hW

-- ======================================= output growth through pumps

/-- One merge step only appends to the output. -/
theorem mem_out_step {st st' : MState} (hstep : step sk st = some st')
    {x : Ev} (hx : x ∈ st.out) : x ∈ st'.out := by
  unfold step at hstep
  cases hscan : scan sk st.sent st.rcvd st.rem with
  | none => rw [hscan] at hstep; simp at hstep
  | some pr =>
      obtain ⟨e, rem'⟩ := pr
      rw [hscan] at hstep
      simp only [Option.map] at hstep
      obtain ⟨c, sd, n⟩ := e
      cases sd <;> cases hstep <;> exact List.mem_append_left _ hx

/-- The merge only appends to the output, any amount of fuel. -/
theorem mem_out_mergeN (fuel : Nat) :
    ∀ {st : MState} {x : Ev}, x ∈ st.out →
      x ∈ (mergeN sk fuel st).out := by
  induction fuel with
  | zero => intro st x hx; exact hx
  | succ f ih =>
      intro st x hx
      unfold mergeN
      cases hstep : step sk st with
      | some st' => exact ih (mem_out_step sk hstep hx)
      | none => exact hx

/-- Emit-then-pump keeps the emitted prefix and the new event. -/
theorem mem_out_wEmitP {st : MState} {e x : Ev}
    (hx : x ∈ st.out ++ [e]) : x ∈ (wEmitP sk st e).out := by
  unfold wEmitP wPump
  refine mem_out_mergeN sk _ ?_
  rw [wEmit_out]
  exact hx

-- ======================================= the consumption induction

/-- THE CONSUMPTION INDUCTION: the edge invariant rides the
interpreter, each manual guard discharged from the pointwise
readiness property, the precedence layer, and the pump fixpoint the
previous emission left behind. -/
theorem weaveGo_wedge (fuel : Nat) :
    ∀ (ops : List WOp) (st : MState) (done : List Ev),
      WEdge sk (goEvents sk fuel ops) st →
      DepOK done (goEvents sk fuel ops) →
      (∀ x ∈ done, x ∈ st.out) →
      EmitOKOn sk (goEvents sk fuel ops) [] →
      step sk st = none →
      WEdge sk [] (weaveGo sk fuel ops st) := by
  induction fuel with
  | zero => intro ops st done hW _ _ _ _; exact hW
  | succ f ih =>
      intro ops st done hW hdep hdone hemit hfix
      match ops with
      | [] => exact hW
      | .emit e :: rest =>
          have hgo : goEvents sk (f + 1) (.emit e :: rest)
              = e :: goEvents sk f rest := rfl
          rw [hgo] at hW hdep hemit
          have hen : enabled sk st.sent st.rcvd e = true := by
            refine hemit 0 e rfl st (by simpa using hW) hfix ?_
            intro d hd
            exact hdone d (depOK_head hdep d hd)
          show WEdge sk [] (weaveGo sk f rest (wEmitP sk st e))
          refine ih rest (wEmitP sk st e) (done ++ [e])
            (wEdge_emitP sk hen hW) (depOK_tail hdep) ?_
            (emitOKOn_tail sk hemit) (wPump_fixpoint sk _)
          intro x hx
          rcases List.mem_append.1 hx with hx | hx
          · exact mem_out_wEmitP sk
              (List.mem_append_left _ (hdone x hx))
          · have hxe : x = e := List.mem_singleton.1 hx
            subst hxe
            exact mem_out_wEmitP sk
              (List.mem_append_right _ (List.mem_cons_self ..))
      | .scope h' k feed :: rest =>
          exact ih _ st done hW hdep hdone hemit hfix
      | .kid h' k s lastD kidBase i feed :: rest =>
          exact ih _ st done hW hdep hdone hemit hfix

-- =============================================== the top assembly

/-- The weave respects every edge GIVEN the pointwise readiness of
the opening worklist's future.

The initial alignment and the precedence layer are already closed
(`weave_initial_alignment`, `weave_goEvents_depOK`); the first
emission — iopen's seq-0 opening wire, the only emission from a
state that is not a pump fixpoint — is peeled by hand with
`enabled_snd_low` before the consumption induction takes over. -/
theorem weaveState_wedge_of_emitOK (hwf : sk.wellFormed = true)
    (hemit : EmitOKOn sk ((weaveOps sk).flatMap (opEvents sk)) []) :
    WEdge sk [] (weaveState sk) := by
  obtain ⟨hown, halign⟩ := weave_initial_alignment sk hwf
  have hgo : goEvents sk (weaveFuel sk) (weaveOps sk)
      = (weaveOps sk).flatMap (opEvents sk) :=
    goEvents_weave sk (weave_events_length sk hwf)
  have hinit : WEdge sk (goEvents sk (weaveFuel sk) (weaveOps sk))
      (weaveInit sk) :=
    wEdge_init sk (by rw [hgo]; exact halign)
      (by rw [hgo]; exact hown)
  have hdep : DepOK [] (goEvents sk (weaveFuel sk) (weaveOps sk)) :=
    weave_goEvents_depOK sk hwf
  obtain ⟨f, hfuel⟩ : ∃ f, weaveFuel sk = f + 1 :=
    ⟨4 * totalEvents sk + 7, by unfold weaveFuel; omega⟩
  -- the head opener, and the worklist behind it
  obtain ⟨e₁, opsTail, hops, he₁⟩ :
      ∃ (e₁ : Ev) (opsTail : List WOp),
        weaveOps sk = .emit e₁ :: opsTail
          ∧ e₁ = ((Chan.wire Party.I sk.rootH, true, 0) : Ev) :=
    ⟨_, _, rfl, rfl⟩
  have hgo1 : goEvents sk (weaveFuel sk) (weaveOps sk)
      = e₁ :: goEvents sk f opsTail := by
    rw [hfuel, hops]
    rfl
  have hen : enabled sk (weaveInit sk).sent (weaveInit sk).rcvd e₁
      = true := by
    rw [he₁]
    exact enabled_snd_low sk (cap_pos hwf _)
  have hW1 : WEdge sk (e₁ :: goEvents sk f opsTail) (weaveInit sk) := by
    rw [← hgo1]
    exact hinit
  show WEdge sk []
    (wPump sk (weaveGo sk (weaveFuel sk) (weaveOps sk) (weaveInit sk)))
  have hstep1 : weaveGo sk (weaveFuel sk) (weaveOps sk) (weaveInit sk)
      = weaveGo sk f opsTail (wEmitP sk (weaveInit sk) e₁) := by
    rw [hfuel, hops]
    rfl
  rw [hstep1]
  refine wEdge_pump sk ?_
  refine weaveGo_wedge sk f opsTail _ [e₁]
    (wEdge_emitP sk hen hW1) ?_ ?_ ?_ (wPump_fixpoint sk _)
  · have hd1 : DepOK [] (e₁ :: goEvents sk f opsTail) := by
      rw [← hgo1]
      exact hdep
    simpa using depOK_tail hd1
  · intro x hx
    have hxe : x = e₁ := List.mem_singleton.1 hx
    refine mem_out_wEmitP sk ?_
    rw [hxe]
    exact List.mem_append_right _ (List.mem_cons_self ..)
  · refine emitOKOn_tail sk (e := e₁) ?_
    rw [← hgo1, hgo]
    exact hemit

-- ==================================== the rolling ancestor telescope

/-- The rolling ancestor context of a site: the in-flight coordinates
of every stage above `h`, with the future's per-ancestor owner
filters.

`A G` is ancestor `G`'s in-flight scope index, `j G` its in-flight
kid slot, `t G` its feed cursor into the slot's query chunk. `coh`
chains adjacent coordinates through the kid-base prefix sums; the
site's own link to `(A (h+1), j (h+1))` rides separately as `hcoh0`
at each consumer. The D flag is carried only from two stages up
(`isD`): a slot two or more stages above any site is disputed by the
childless-W geometry, while the immediate parent's flag, where
needed, is re-derived from the site's own scope being nonempty
(`parent_slot_isD`). -/
structure AncTele (h : Nat) (A j t : Nat → Nat) (fut : List Ev) :
    Prop where
  rng : ∀ G, h < G → G < sk.rootH →
    A G < sk.stageLen G
      ∧ j G < sk.nChildren G (sk.stageScope G (A G))
  isD : ∀ G, h + 2 ≤ G → G < sk.rootH →
    sk.childIsD G (sk.stageScope G (A G)) (j G) = true
  coh : ∀ G, h + 1 ≤ G → G + 1 < sk.rootH →
    A G = sk.wiresBefore (G + 1) (A (G + 1)) + j (G + 1)
  fil : ∀ G, h < G → G < sk.rootH →
    fut.filter (fun e => evOwner sk e == walkIdx sk G)
      = (chunkQ sk G (A G) (j G)).drop (t G)
        ++ (List.range' (j G + 1)
              (sk.nChildren G (sk.stageScope G (A G)) - (j G + 1))).flatMap
             (splicedChunk sk G (A G) (lastDOf sk G (A G)))
        ++ walkSeg sk G (A G + 1) (sk.stageLen G)

/-- A parent slot pointing at a nonempty scope is disputed: W and R
subtrees are childless, so any scope with slots hangs off a D slot. -/
theorem parent_slot_isD (hwf : sk.wellFormed = true) {h k : Nat}
    (hr : h + 1 < sk.rootH) (hk : k < sk.stageLen h) {A1 j1 : Nat}
    (hA1 : A1 < sk.stageLen (h + 1))
    (hj1 : j1 < sk.nChildren (h + 1) (sk.stageScope (h + 1) A1))
    (hcoh : k = sk.wiresBefore (h + 1) A1 + j1)
    (hkids : 0 < sk.nChildren h (sk.stageScope h k)) :
    sk.childIsD (h + 1) (sk.stageScope (h + 1) A1) j1 = true := by
  have hkind := childIsD_eq_kid_kind sk hwf (by omega) hr hA1 hj1
  rw [show h + 1 - 1 = h from rfl, ← hcoh] at hkind
  rw [hkind]
  by_cases hD : (sk.scope (sk.stageScope h k)).kind = Kind.D
  · simp [hD]
  · exfalso
    have hmem : sk.stageScope h k ∈ sk.scopesAt (h + 1) := by
      unfold Skel.stageScope
      have hlen : k < (sk.stageScopes h).length := hk
      rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hlen]
      exact List.getElem_mem _
    obtain ⟨hknil, hlr⟩ :=
      wf_scope_notD hwf (mem_scopesAt hmem).1 hD
    unfold Skel.nChildren at hkids
    split at hkids
    · omega
    · rw [hknil] at hkids
      simp at hkids

-- ======================================= parity and channel spelling

/-- Asking is invariant two stages at a time, iterated. -/
theorem asks_add_two_mul (p : Party) (h m : Nat) :
    asks p (h + 2 * m) = asks p h := by
  induction m with
  | zero => rfl
  | succ m ih =>
      rw [show h + 2 * (m + 1) = h + 2 * m + 2 from by omega,
        asks_add_two, ih]

/-- Two stages an answerer answers have the same parity. -/
theorem asks_false_parity {p : Party} {a b : Nat}
    (ha : asks p a = false) (hb : asks p b = false) :
    a % 2 = b % 2 := by
  cases p <;>
    simp only [asks, beq_eq_false_iff_ne, ne_eq] at ha hb <;> omega

/-- The window channels' answerer-party spelling, upper side. -/
theorem upperOut_eq_of_answerer {p : Party} {g : Nat}
    (hna : asks p g = false) :
    Chan.upper p g = upperOut (wpk g) := by
  rw [show upperOut (wpk g) = Chan.upper (wpk g).1 g from rfl,
    wpk_fst_of_answerer hna]

/-- The window channels' answerer-party spelling, lower side. -/
theorem lowerOut_eq_of_answerer {p : Party} {g : Nat}
    (hna : asks p g = false) :
    Chan.lower p g = lowerOut (wpk g) := by
  rw [show lowerOut (wpk g) = Chan.lower (wpk g).1 g from rfl,
    wpk_fst_of_answerer hna]

/-- The answerer party's tower top: `rootH` for the initiator's odd
stages, `rootH - 1` for the responder's even ones. -/
def wtop (h : Nat) : Nat :=
  if h % 2 == 1 then sk.rootH else sk.rootH - 1

/-- The tower-top instance carried by a stage's own walk party. -/
theorem wpk_htop (h : Nat) :
    (wpk h).1 = Party.I ∧ wtop sk h = sk.rootH
      ∨ (wpk h).1 = Party.R ∧ wtop sk h = sk.rootH - 1 := by
  unfold wpk wtop
  rcases Nat.mod_two_eq_zero_or_one h with hp | hp
  · rw [hp]
    exact Or.inr ⟨rfl, rfl⟩
  · rw [hp]
    exact Or.inl ⟨rfl, rfl⟩

theorem wtop_le (h : Nat) : wtop sk h ≤ sk.rootH := by
  unfold wtop
  split <;> omega

/-- A stage sits below its own party's tower top. -/
theorem wtop_ge {sk : Skel} (hwf : sk.wellFormed = true) {h : Nat}
    (hh : h < sk.rootH) : h + 1 ≤ wtop sk h := by
  have hev := (wf_rootH hwf).1
  have hge := (wf_rootH hwf).2
  unfold wtop
  rcases Nat.mod_two_eq_zero_or_one h with hp | hp
  · rw [hp]
    simp only [Nat.zero_ne_one, beq_iff_eq, if_false]
    omega
  · rw [hp]
    simp only [beq_self_eq_true, if_true]
    omega

/-- An answerer stage under the top stays under the root: the party's
parity excludes the initiator's even root. -/
theorem answerer_lt_rootH (hwf : sk.wellFormed = true) {h G : Nat}
    (hGt : G ≤ wtop sk h) (hna : asks (wpk h).1 G = false) :
    G < sk.rootH := by
  have hev := (wf_rootH hwf).1
  have hge := (wf_rootH hwf).2
  have hle := wtop_le sk h
  rcases Nat.lt_or_ge G sk.rootH with h' | h'
  · exact h'
  · exfalso
    have hG : G = sk.rootH := by omega
    subst hG
    rcases wpk_htop sk h with ⟨hp, -⟩ | ⟨hp, ht⟩
    · rw [hp] at hna
      simp only [asks, hev, beq_self_eq_true] at hna
      exact Bool.noConfusion hna
    · rw [ht] at hGt
      omega

/-- `qsBefore` is monotone in the cursor (same saturation argument as
`wiresBefore_mono`). -/
theorem qsBefore_mono (h : Nat) : ∀ {k k' : Nat}, k ≤ k' →
    sk.qsBefore h k ≤ sk.qsBefore h k' := by
  intro k k' hkk
  induction k' with
  | zero =>
      have hk0 : k = 0 := by omega
      subst hk0
      exact Nat.le_refl _
  | succ k' ih =>
      by_cases hlast : k = k' + 1
      · subst hlast
        exact Nat.le_refl _
      · have hstep : sk.qsBefore h k' ≤ sk.qsBefore h (k' + 1) := by
          by_cases hin : k' < sk.stageLen h
          · rw [qsBefore_succ sk hin]
            omega
          · unfold Skel.qsBefore
            rw [List.take_of_length_le (by
                unfold Skel.stageLen at hin
                omega),
              List.take_of_length_le (by
                unfold Skel.stageLen at hin
                omega)]
            exact Nat.le_refl _
        exact Nat.le_trans (ih (by omega)) hstep

-- ============================================= deep-window count pins

/-- A deep stage parked at its window start has emitted exactly the
resolutions before the window. -/
theorem deep_lower_count (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {g c : Nat}
    (hgr : g < sk.rootH) (hc : c ≤ sk.stageLen g)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk g)
      = walkSeg sk g c (sk.stageLen g)) :
    sndCount (lowerOut (wpk g)) st.out = sk.dsBefore g c := by
  have hfl := futLen_walkSeg_res sk hc (Nat.le_refl _) hfil
  have hpin := lower_snd_pin sk hwf h hgr
  have hmono := dsBefore_mono sk g hc
  omega

/-- A deep stage parked at its window start has emitted exactly the
summaries before the window. -/
theorem deep_upper_count (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {g c : Nat}
    (hgr : g < sk.rootH) (hc : c ≤ sk.stageLen g)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk g)
      = walkSeg sk g c (sk.stageLen g)) :
    sndCount (upperOut (wpk g)) st.out = c := by
  have hfl := futLen_walkSeg_upper sk hc hfil
  have hpin := upper_snd_pin sk hwf h hgr
  omega

/-- A deep stage parked at its window start has emitted exactly the
wires before the window. -/
theorem deep_wire_count (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {g c : Nat}
    (hgr : g < sk.rootH) (hc : c ≤ sk.stageLen g)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk g)
      = walkSeg sk g c (sk.stageLen g)) :
    sndCount (wireOut (wpk g)) st.out = sk.wiresBefore g c := by
  have hfl := futLen_walkSeg_wire sk hc (Nat.le_refl _) hfil
  have hpin := wire_snd_pin sk hwf h hgr
  have hmono := wiresBefore_mono sk g hc
  omega

/-- A deep stage parked at its window start has emitted exactly the
queries before the window. -/
theorem deep_q_count (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {g c : Nat} (h1 : 1 ≤ g)
    (hgr : g < sk.rootH) (hc : c ≤ sk.stageLen g)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk g)
      = walkSeg sk g c (sk.stageLen g)) :
    sndCount (askedOut (wpk g)) st.out = sk.qsBefore g c := by
  have hfl := futLen_walkSeg_q sk hc (Nat.le_refl _) hfil
  have hpin := asked_snd_pin sk hwf h h1 hgr
  have hmono := qsBefore_mono sk g hc
  omega

-- ================================================== ancestor pins

/-- An in-flight ancestor's count pins, read off the telescope. -/
theorem ancTele_counts (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (hW : WCount sk fut st) {h : Nat} {A j t : Nat → Nat}
    (hanc : AncTele sk h A j t fut) {G : Nat} (hG : h < G)
    (hGr : G < sk.rootH)
    (hD : sk.childIsD G (sk.stageScope G (A G)) (j G) = true) :
    sndCount (upperOut (wpk G)) st.out
        = A G + (if lastDOf sk G (A G) == some (j G) then 1 else 0)
      ∧ sndCount (lowerOut (wpk G)) st.out
        = sk.dsBefore G (A G) + dRank sk (wpk G) (A G) (j G) + 1 := by
  obtain ⟨hA, hj⟩ := hanc.rng G hG hGr
  exact anc_position_counts sk hwf hW hGr hA hj hD
    (futLen_anc_upper sk hA hj hD (hanc.fil G hG hGr))
    (futLen_anc_lower sk hA hj hD (hanc.fil G hG hGr))

/-- An in-flight ancestor's `P1` overhang fact, read off the
telescope. -/
theorem ancTele_p1 (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) {fut : List Ev} {st : MState}
    (hW : WCount sk fut st) {h : Nat} {A j t : Nat → Nat}
    (hanc : AncTele sk h A j t fut) {p : Party} {G : Nat} (hG : h < G)
    (hGr : G < sk.rootH) (hna : asks p G = false)
    (hD : sk.childIsD G (sk.stageScope G (A G)) (j G) = true) :
    sndCount (Chan.lower p G) st.out
      ≤ sk.dsBefore G (sndCount (Chan.upper p G) st.out)
        + sk.capLevel + 1 := by
  obtain ⟨hA, hj⟩ := hanc.rng G hG hGr
  exact p1_of_anc sk hwf hsched hW hna hGr hA hj hD
    (futLen_anc_upper sk hA hj hD (hanc.fil G hG hGr))
    (futLen_anc_lower sk hA hj hD (hanc.fil G hG hGr))

/-- The root resolution is banked at any position whose owner-1 share
is a suffix of ropen's query tail. -/
theorem root_banked (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (hW : WCount sk fut st)
    (hfeed : ∃ i₀, fut.filter (fun e => evOwner sk e == 1)
      = ((ropenEvents sk).drop 3).drop i₀) :
    1 ≤ sndCount Chan.rootres st.out := by
  obtain ⟨i₀, hf⟩ := hfeed
  exact rootres_pin sk hwf hW (feed_rootres_silent sk hf)

-- ====================================================== the ladders

/-- One ladder rung above the immediate parent: the counts two stages
down give the stage's link, pre-splice by `base`, post-splice by
`step` through the previous link. -/
theorem ladder_rung (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (hW : WCount sk fut st) {h : Nat} {A j t : Nat → Nat}
    (hanc : AncTele sk h A j t fut) {p : Party} {g : Nat}
    (hg : h < g) (hg1 : 1 ≤ g) (hna : asks p (g + 2) = false)
    (hgr : g + 2 < sk.rootH)
    (hD : sk.childIsD g (sk.stageScope g (A g)) (j g) = true)
    (hD2 : sk.childIsD (g + 2) (sk.stageScope (g + 2) (A (g + 2)))
      (j (g + 2)) = true)
    (prev : lastDOf sk g (A g) = some (j g) → SpineLink sk st p g) :
    SpineLink sk st p (g + 2) := by
  have hnag : asks p g = false := by
    have hs := asks_add_two p g
    rw [hna] at hs
    exact hs.symm
  obtain ⟨hcu, hcl⟩ := ancTele_counts sk hwf hW hanc hg (by omega) hD
  obtain ⟨-, hcl2⟩ := ancTele_counts sk hwf hW hanc
    (show h < g + 2 by omega) hgr hD2
  rw [← upperOut_eq_of_answerer hnag] at hcu
  rw [← lowerOut_eq_of_answerer hnag] at hcl
  rw [← lowerOut_eq_of_answerer hna] at hcl2
  obtain ⟨hA2, hj2⟩ := hanc.rng (g + 2) (by omega) hgr
  obtain ⟨hA1, hj1⟩ := hanc.rng (g + 1) (by omega) (by omega)
  obtain ⟨hAg, -⟩ := hanc.rng g hg (by omega)
  have hcoh1 := hanc.coh (g + 1) (by omega) (by omega)
  have hcohg := hanc.coh g (by omega) (by omega)
  have hmid : sk.wiresBefore (g + 1)
        (sk.wiresBefore (g + 2) (A (g + 2)) + j (g + 2)) + j (g + 1)
      = A g := by
    rw [← hcoh1]
    exact hcohg.symm
  have ht' : j (g + 1) < sk.nChildren (g + 1)
      (sk.stageScope (g + 1)
        (sk.wiresBefore (g + 2) (A (g + 2)) + j (g + 2))) := by
    rw [← hcoh1]
    exact hj1
  by_cases hbe : lastDOf sk g (A g) = some (j g)
  · -- post-splice below: step through the splice identity
    have hb : (lastDOf sk g (A g) == some (j g)) = true := by
      simp [hbe]
    rw [hb, if_pos rfl] at hcu
    have hdl := dRank_lastD sk hbe
    have hds := dsBefore_succ sk hAg
    refine spineLink_step_at sk hwf hg1 hna hgr hA2 hj2 hD2
      (t := j (g + 1)) ht' ?_ ?_ hcl2 (prev hbe)
    · rw [show sk.wiresBefore (g + 1)
            (sk.wiresBefore (g + 2) (A (g + 2)) + j (g + 2))
            + j (g + 1) + 1
          = A g + 1 from by omega]
      omega
    · rw [show sk.wiresBefore (g + 1)
            (sk.wiresBefore (g + 2) (A (g + 2)) + j (g + 2))
            + j (g + 1) + 1
          = A g + 1 from by omega, hds]
      omega
  · -- pre-splice below: the summary sits strictly inside the cut
    have hb : (lastDOf sk g (A g) == some (j g)) = false := by
      simp [hbe]
    rw [hb, if_neg (by simp)] at hcu
    refine spineLink_base_at sk hwf hna hgr hA2 hj2 hD2
      (t := j (g + 1)) ht' ?_ hcl2
    rw [hmid]
    omega

/-- The spine ladder above a site: every same-parity stage from two
above the emission links, chains bottoming at the site's own
strictly-in-cut summary count. -/
theorem ancTele_ladder (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (hW : WCount sk fut st) {h : Nat} {A j t : Nat → Nat}
    (hanc : AncTele sk h A j t fut) {p : Party} {k : Nat}
    (hp : asks p h = false)
    (hcoh0 : h + 1 < sk.rootH →
      k = sk.wiresBefore (h + 1) (A (h + 1)) + j (h + 1))
    (hup0 : sndCount (Chan.upper p h) st.out = k) :
    ∀ m, h + 2 + 2 * m < sk.rootH →
      SpineLink sk st p (h + 2 + 2 * m) := by
  intro m
  induction m with
  | zero =>
      intro hr
      have hna2 : asks p (h + 2) = false := by
        rw [asks_add_two]
        exact hp
      obtain ⟨hA2, hj2⟩ := hanc.rng (h + 2) (by omega) hr
      have hD2 := hanc.isD (h + 2) (by omega) hr
      obtain ⟨hA1, hj1⟩ := hanc.rng (h + 1) (by omega) (by omega)
      have hcohl := hanc.coh (h + 1) (by omega) (by omega)
      obtain ⟨-, hcl⟩ := ancTele_counts sk hwf hW hanc
        (show h < h + 2 by omega) hr hD2
      rw [← lowerOut_eq_of_answerer hna2] at hcl
      refine spineLink_base_at sk hwf hna2 hr hA2 hj2 hD2
        (t := j (h + 1)) ?_ ?_ hcl
      · rw [← hcohl]
        exact hj1
      · rw [← hcohl, ← hcoh0 (by omega)]
        exact hup0
  | succ m ihm =>
      intro hr
      have hrm : h + 2 + 2 * m + 2 < sk.rootH := by omega
      have hDg : sk.childIsD (h + 2 + 2 * m)
          (sk.stageScope (h + 2 + 2 * m) (A (h + 2 + 2 * m)))
          (j (h + 2 + 2 * m)) = true :=
        hanc.isD (h + 2 + 2 * m) (by omega) (by omega)
      have hDG : sk.childIsD (h + 2 + 2 * m + 2)
          (sk.stageScope (h + 2 + 2 * m + 2)
            (A (h + 2 + 2 * m + 2)))
          (j (h + 2 + 2 * m + 2)) = true :=
        hanc.isD (h + 2 + 2 * m + 2) (by omega) hrm
      have hnaG : asks p (h + 2 + 2 * m + 2) = false := by
        have hs := asks_add_two_mul p h (m + 2)
        rw [show h + 2 * (m + 2) = h + 2 + 2 * m + 2 from by omega]
          at hs
        rw [hs]
        exact hp
      have hlink := ladder_rung sk hwf hW hanc
        (g := h + 2 + 2 * m) (by omega) (by omega) hnaG hrm hDg hDG
        (fun _ => ihm (by omega))
      exact hlink

/-- The leaf sites' spine ladder: the initiator chain from the
absorber's stage-1 bottom through every odd ancestor stage. -/
theorem ancTele_ladder_leaf (hwf : sk.wellFormed = true)
    {fut : List Ev} {st : MState} (hW : WCount sk fut st)
    {A j t : Nat → Nat} (hanc : AncTele sk 0 A j t fut)
    (hr : 1 < sk.rootH) {k i0 : Nat} (hk : k < sk.stageLen 0)
    (hcoh0 : k = sk.wiresBefore 1 (A 1) + j 1)
    (hi0 : i0 < sk.nChildren 0 (sk.stageScope 0 k))
    (hreq0 : sndCount Chan.leafRequests st.out
      = sk.wiresBefore 0 k + i0) :
    ∀ m, 1 + 2 * m < sk.rootH →
      SpineLink sk st Party.I (1 + 2 * m) := by
  obtain ⟨hA1, hj1⟩ := hanc.rng 1 (by omega) hr
  have hD1 : sk.childIsD 1 (sk.stageScope 1 (A 1)) (j 1) = true :=
    parent_slot_isD sk hwf hr hk hA1 hj1 hcoh0 (by omega)
  have habs : SpineLink sk st Party.I 1 := by
    obtain ⟨-, hcl⟩ := ancTele_counts sk hwf hW hanc (by omega) hr hD1
    refine spineLink_absorb_at sk hwf hr hA1 hj1 hD1 (i0 := i0)
      ?_ hcl ?_
    · rw [← hcoh0]
      exact hi0
    · rw [← hcoh0]
      exact hreq0
  intro m
  induction m with
  | zero => intro _; exact habs
  | succ m ihm =>
      intro hrm
      have hDg : sk.childIsD (1 + 2 * m)
          (sk.stageScope (1 + 2 * m) (A (1 + 2 * m)))
          (j (1 + 2 * m)) = true := by
        rcases Nat.eq_zero_or_pos m with rfl | hm
        · exact hD1
        · exact hanc.isD (1 + 2 * m) (by omega) (by omega)
      have hDG := hanc.isD (1 + 2 * m + 2) (by omega) (by omega)
      have hnaG : asks Party.I (1 + 2 * m + 2) = false := by
        have hs := asks_add_two_mul Party.I 1 (m + 1)
        rw [show 1 + 2 * (m + 1) = 1 + 2 * m + 2 from by omega] at hs
        rw [hs]
        rfl
      have hlink := ladder_rung sk hwf hW hanc (g := 1 + 2 * m)
        (by omega) (by omega) hnaG (by omega) hDg hDG
        (fun _ => ihm (by omega))
      exact hlink

-- ============================================ coverage assemblies

/-- The ascent coverage of an interior site, assembled: the ladder
supplies each covered stage's spine link, the telescope's pins the
overhang facts. -/
theorem ancTele_cov (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) {fut : List Ev} {st : MState}
    (hW : WEdge sk fut st) {h : Nat} {A j t : Nat → Nat}
    (hanc : AncTele sk h A j t fut) {k : Nat}
    (hcoh0 : h + 1 < sk.rootH →
      k = sk.wiresBefore (h + 1) (A (h + 1)) + j (h + 1))
    (hup0 : sndCount (Chan.upper ((wpk h).1) h) st.out = k) :
    AscCover sk st ((wpk h).1) (h + 2) (wtop sk h) := by
  refine ascCover_of_spine sk hwf hW (wpk_htop sk h) ?_ ?_
  · intro G hG2 hGt hna
    have hGr : G < sk.rootH := answerer_lt_rootH sk hwf hGt hna
    have hpar := asks_false_parity hna (asks_wpk_self h)
    obtain ⟨m, rfl⟩ : ∃ m, G = h + 2 + 2 * m :=
      ⟨(G - h - 2) / 2, by omega⟩
    exact ancTele_ladder sk hwf hW.toWCount hanc (asks_wpk_self h)
      hcoh0 hup0 m hGr
  · intro G hG2 hGt hna
    have hGr : G < sk.rootH := answerer_lt_rootH sk hwf hGt hna
    exact ancTele_p1 sk hwf hsched hW.toWCount hanc (by omega) hGr
      hna (hanc.isD G (by omega) hGr)

/-- The leaf sites' ascent coverage: the initiator tower covered from
the absorber's stage-1 bottom to the root. -/
theorem ancTele_cov_leaf (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) {fut : List Ev} {st : MState}
    (hW : WEdge sk fut st) {A j t : Nat → Nat}
    (hanc : AncTele sk 0 A j t fut) (hr : 1 < sk.rootH)
    {k i0 : Nat} (hk : k < sk.stageLen 0)
    (hcoh0 : k = sk.wiresBefore 1 (A 1) + j 1)
    (hi0 : i0 < sk.nChildren 0 (sk.stageScope 0 k))
    (hreq0 : sndCount Chan.leafRequests st.out
      = sk.wiresBefore 0 k + i0) :
    AscCover sk st Party.I 1 sk.rootH := by
  have hev := (wf_rootH hwf).1
  obtain ⟨hA1, hj1⟩ := hanc.rng 1 (by omega) hr
  have hD1 : sk.childIsD 1 (sk.stageScope 1 (A 1)) (j 1) = true :=
    parent_slot_isD sk hwf hr hk hA1 hj1 hcoh0 (by omega)
  refine ascCover_of_spine sk hwf hW (Or.inl ⟨rfl, rfl⟩) ?_ ?_
  · intro G hG1 hGt hna
    have hGr : G < sk.rootH := by
      rcases Nat.lt_or_ge G sk.rootH with h' | h'
      · exact h'
      · exfalso
        have hG : G = sk.rootH := by omega
        subst hG
        simp [asks, hev] at hna
    have hodd : G % 2 = 1 := by
      simp only [asks, beq_eq_false_iff_ne, ne_eq] at hna
      omega
    obtain ⟨m, rfl⟩ : ∃ m, G = 1 + 2 * m := ⟨(G - 1) / 2, by omega⟩
    exact ancTele_ladder_leaf sk hwf hW.toWCount hanc hr hk hcoh0
      hi0 hreq0 m hGr
  · intro G hG1 hGt hna
    have hGr : G < sk.rootH := by
      rcases Nat.lt_or_ge G sk.rootH with h' | h'
      · exact h'
      · exfalso
        have hG : G = sk.rootH := by omega
        subst hG
        simp [asks, hev] at hna
    rcases Nat.lt_or_ge G 2 with hG2 | hG2
    · have hG1' : G = 1 := by omega
      subst hG1'
      exact ancTele_p1 sk hwf hsched hW.toWCount hanc (by omega) hGr
        hna hD1
    · exact ancTele_p1 sk hwf hsched hW.toWCount hanc (by omega) hGr
        hna (hanc.isD G (by omega) hGr)

-- ============================================ own-stage floor counts

private theorem qSum_mono (pk : Party × Nat) (k : Nat) :
    ∀ {i i' : Nat}, i ≤ i' → qSum sk pk k i ≤ qSum sk pk k i' := by
  intro i i' hii
  induction i' with
  | zero =>
      have h0 : i = 0 := by omega
      subst h0
      exact Nat.le_refl _
  | succ i' ih =>
      by_cases hlast : i = i' + 1
      · subst hlast
        exact Nat.le_refl _
      · have hstep : qSum sk pk k i' ≤ qSum sk pk k (i' + 1) := by
          have := qSum_succ sk pk k i'
          omega
        exact Nat.le_trans (ih (by omega)) hstep

/-- The prologue-summary site's resolution floor: every resolution of
this scope and later is still ahead. -/
private theorem futLen_S1_res {fut : List Ev} {h k : Nat}
    (hk : k < sk.stageLen h)
    (hown : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((upperOut (wpk h), true, k) : Ev)
          :: ((List.range' 0 (sk.nChildren h (sk.stageScope h k))).flatMap
                (splicedChunk sk h k (lastDOf sk h k))
              ++ walkSeg sk h (k + 1) (sk.stageLen h))) :
    futLen sk fut (walkIdx sk h) (lowerOut (wpk h)) true
      = sk.dsBefore h (sk.stageLen h) - sk.dsBefore h k := by
  rw [futLen_of_filter sk hown,
    proj_cons_ne_chan (by simp [upperOut, lowerOut]),
    proj_append, chunks_proj_res sk h k (lastDOf sk h k) _ 0,
    walkSeg_proj_res sk (show k + 1 ≤ sk.stageLen h from hk)
      (Nat.le_refl _)]
  simp only [List.length_append, seg_len]
  have h0 : dRank sk (wpk h) k 0 = 0 := rfl
  have htot : dRank sk (wpk h) k
      (0 + sk.nChildren h (sk.stageScope h k))
      = sk.dOf h (sk.stageScope h k) := by
    rw [Nat.zero_add]
    exact dRank_total sk (wpk h) k
  have hds := dsBefore_succ sk hk
  have hmono := dsBefore_mono sk h
    (show k + 1 ≤ sk.stageLen h from hk)
  omega

/-- The prologue-summary site's query floor. -/
private theorem futLen_S1_q {fut : List Ev} {h k : Nat}
    (hk : k < sk.stageLen h)
    (hown : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((upperOut (wpk h), true, k) : Ev)
          :: ((List.range' 0 (sk.nChildren h (sk.stageScope h k))).flatMap
                (splicedChunk sk h k (lastDOf sk h k))
              ++ walkSeg sk h (k + 1) (sk.stageLen h))) :
    futLen sk fut (walkIdx sk h) (askedOut (wpk h)) true
      = sk.qsBefore h (sk.stageLen h) - sk.qsBefore h k := by
  rw [futLen_of_filter sk hown,
    proj_cons_ne_chan (by
      unfold askedOut upperOut
      split <;> simp),
    proj_append, chunks_proj_q sk h k (lastDOf sk h k) _ 0,
    walkSeg_proj_q sk (show k + 1 ≤ sk.stageLen h from hk)
      (Nat.le_refl _)]
  simp only [List.length_append, seg_len]
  have h0 : qSum sk (wpk h) k 0 = 0 := rfl
  have htot : qSum sk (wpk h) k
      (0 + sk.nChildren h (sk.stageScope h k))
      = sk.qOf h (sk.stageScope h k) := by
    rw [Nat.zero_add]
    exact qSum_total sk (wpk h) k
  have hqs := qsBefore_succ sk hk
  have hmono := qsBefore_mono sk h
    (show k + 1 ≤ sk.stageLen h from hk)
  omega

/-- The splice-summary site's resolution floor: the resolutions
through the last D slot's are already out. -/
private theorem futLen_S2_res {fut : List Ev} {h k jL : Nat}
    (hk : k < sk.stageLen h) (hlast : lastDOf sk h k = some jL)
    (hown : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((upperOut (wpk h), true, k) : Ev)
          :: (chunkQ sk h k jL
              ++ (List.range' (jL + 1)
                    (sk.nChildren h (sk.stageScope h k)
                      - (jL + 1))).flatMap
                   (splicedChunk sk h k (lastDOf sk h k))
              ++ walkSeg sk h (k + 1) (sk.stageLen h))) :
    futLen sk fut (walkIdx sk h) (lowerOut (wpk h)) true
      = sk.dsBefore h (sk.stageLen h)
        - (sk.dsBefore h k + dRank sk (wpk h) k jL + 1) := by
  obtain ⟨hD, hjn⟩ := lastDOf_isD sk hlast
  have hqne : proj (lowerOut (wpk h)) true (chunkQ sk h k jL) = [] :=
    chunkQ_proj_ne sk h k jL (by
      rintro ⟨hc, -⟩
      simp only [askedOut, lowerOut] at hc
      split at hc <;> exact Chan.noConfusion hc)
  rw [futLen_of_filter sk hown,
    proj_cons_ne_chan (by simp [upperOut, lowerOut]),
    proj_append, proj_append, hqne,
    chunks_proj_res sk h k (lastDOf sk h k) _ (jL + 1),
    walkSeg_proj_res sk (show k + 1 ≤ sk.stageLen h from hk)
      (Nat.le_refl _)]
  simp only [List.nil_append, List.length_append, seg_len]
  have hidx : jL + 1 + (sk.nChildren h (sk.stageScope h k) - (jL + 1))
      = sk.nChildren h (sk.stageScope h k) := by omega
  rw [hidx]
  have htot : dRank sk (wpk h) k (sk.nChildren h (sk.stageScope h k))
      = sk.dOf h (sk.stageScope h k) := dRank_total sk (wpk h) k
  have hds := dRank_succ sk (wpk h) k jL
  rw [show sk.childIsD (wpk h).2 (sk.stageScope (wpk h).2 k) jL
      = sk.childIsD h (sk.stageScope h k) jL from rfl, hD,
    if_pos rfl] at hds
  have hsc := dsBefore_succ sk hk
  have hmono := dsBefore_mono sk h
    (show k + 1 ≤ sk.stageLen h from hk)
  have hle : dRank sk (wpk h) k jL + 1
      ≤ sk.dOf h (sk.stageScope h k) :=
    dRank_succ_le_dOf sk (wpk h) hjn hD
  omega

/-- The splice-summary site's query floor. -/
private theorem futLen_S2_q {fut : List Ev} {h k jL : Nat}
    (hk : k < sk.stageLen h) (hlast : lastDOf sk h k = some jL)
    (hown : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((upperOut (wpk h), true, k) : Ev)
          :: (chunkQ sk h k jL
              ++ (List.range' (jL + 1)
                    (sk.nChildren h (sk.stageScope h k)
                      - (jL + 1))).flatMap
                   (splicedChunk sk h k (lastDOf sk h k))
              ++ walkSeg sk h (k + 1) (sk.stageLen h))) :
    futLen sk fut (walkIdx sk h) (askedOut (wpk h)) true
      = sk.qsBefore h (sk.stageLen h)
        - (sk.qsBefore h k + qSum sk (wpk h) k jL) := by
  obtain ⟨hD, hjn⟩ := lastDOf_isD sk hlast
  have hcq : chunkQ sk h k jL
      = seg (askedOut (wpk h)) true
          (sk.qsBefore h k + qSum sk (wpk h) k jL)
          (sk.qCount h (sk.stageScope h k) jL) := rfl
  rw [futLen_of_filter sk hown,
    proj_cons_ne_chan (by
      unfold askedOut upperOut
      split <;> simp),
    proj_append, proj_append, hcq, proj_seg_self,
    chunks_proj_q sk h k (lastDOf sk h k) _ (jL + 1),
    walkSeg_proj_q sk (show k + 1 ≤ sk.stageLen h from hk)
      (Nat.le_refl _)]
  simp only [List.length_append, seg_len]
  have hidx : jL + 1 + (sk.nChildren h (sk.stageScope h k) - (jL + 1))
      = sk.nChildren h (sk.stageScope h k) := by omega
  rw [hidx]
  have htot : qSum sk (wpk h) k (sk.nChildren h (sk.stageScope h k))
      = sk.qOf h (sk.stageScope h k) := qSum_total sk (wpk h) k
  have hqs1 : qSum sk (wpk h) k (jL + 1)
      = qSum sk (wpk h) k jL
        + sk.qCount h (sk.stageScope h k) jL :=
    qSum_succ sk (wpk h) k jL
  have hqsuc := qsBefore_succ sk hk
  have hmono := qsBefore_mono sk h
    (show k + 1 ≤ sk.stageLen h from hk)
  have hqm : qSum sk (wpk h) k (jL + 1)
      ≤ qSum sk (wpk h) k (sk.nChildren h (sk.stageScope h k)) :=
    qSum_mono sk (wpk h) k hjn
  omega

/-- The resolution site's query floor: the slot's own queries are
still ahead. -/
private theorem futLen_SL_q {fut : List Ev} {h k i : Nat}
    (hk : k < sk.stageLen h)
    (hi : i < sk.nChildren h (sk.stageScope h k))
    (hown : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((lowerOut (wpk h), true,
            sk.dsBefore h k + dRank sk (wpk h) k i) : Ev)
          :: ((if lastDOf sk h k == some i
                then [((upperOut (wpk h), true, k) : Ev)] else [])
              ++ chunkQ sk h k i
              ++ (List.range' (i + 1)
                    (sk.nChildren h (sk.stageScope h k)
                      - (i + 1))).flatMap
                   (splicedChunk sk h k (lastDOf sk h k))
              ++ walkSeg sk h (k + 1) (sk.stageLen h))) :
    futLen sk fut (walkIdx sk h) (askedOut (wpk h)) true
      = sk.qsBefore h (sk.stageLen h)
        - (sk.qsBefore h k + qSum sk (wpk h) k i) := by
  have hspl : proj (askedOut (wpk h)) true
      (if lastDOf sk h k == some i
        then [((upperOut (wpk h), true, k) : Ev)] else []) = [] := by
    split
    · rw [proj_cons_ne_chan (by
        unfold askedOut upperOut
        split <;> simp), proj_nil]
    · rfl
  have hcq : chunkQ sk h k i
      = seg (askedOut (wpk h)) true
          (sk.qsBefore h k + qSum sk (wpk h) k i)
          (sk.qCount h (sk.stageScope h k) i) := rfl
  rw [futLen_of_filter sk hown,
    proj_cons_ne_chan (by
      unfold askedOut lowerOut
      split <;> simp),
    proj_append, proj_append, proj_append, hspl,
    hcq, proj_seg_self,
    chunks_proj_q sk h k (lastDOf sk h k) _ (i + 1),
    walkSeg_proj_q sk (show k + 1 ≤ sk.stageLen h from hk)
      (Nat.le_refl _)]
  simp only [List.nil_append, List.length_append, seg_len]
  have hidx : i + 1 + (sk.nChildren h (sk.stageScope h k) - (i + 1))
      = sk.nChildren h (sk.stageScope h k) := by omega
  rw [hidx]
  have htot : qSum sk (wpk h) k (sk.nChildren h (sk.stageScope h k))
      = sk.qOf h (sk.stageScope h k) := qSum_total sk (wpk h) k
  have hqs1 : qSum sk (wpk h) k (i + 1)
      = qSum sk (wpk h) k i + sk.qCount h (sk.stageScope h k) i :=
    qSum_succ sk (wpk h) k i
  have hqsuc := qsBefore_succ sk hk
  have hmono := qsBefore_mono sk h
    (show k + 1 ≤ sk.stageLen h from hk)
  have hqm : qSum sk (wpk h) k (i + 1)
      ≤ qSum sk (wpk h) k (sk.nChildren h (sk.stageScope h k)) :=
    qSum_mono sk (wpk h) k hi
  omega

/-- The leaf-request site's wire count: the slot's wire is already
out. -/
private theorem futLen_Q0_wire {fut : List Ev} {k i0 : Nat}
    (hk : k < sk.stageLen 0)
    (hi0 : i0 < sk.nChildren 0 (sk.stageScope 0 k))
    (hown : fut.filter (fun e => evOwner sk e == walkIdx sk 0)
      = (List.range' (i0 + 1)
            (sk.nChildren 0 (sk.stageScope 0 k) - (i0 + 1))).flatMap
            (splicedChunk sk 0 k (lastDOf sk 0 k))
          ++ walkSeg sk 0 (k + 1) (sk.stageLen 0)) :
    futLen sk fut (walkIdx sk 0) (wireOut (wpk 0)) true
      = sk.wiresBefore 0 (sk.stageLen 0)
        - (sk.wiresBefore 0 k + i0 + 1) := by
  rw [futLen_of_filter sk hown, proj_append,
    chunks_proj_wire sk 0 k (lastDOf sk 0 k) _ (i0 + 1),
    walkSeg_proj_wire sk (show k + 1 ≤ sk.stageLen 0 from hk)
      (Nat.le_refl _)]
  simp only [List.length_append, seg_len]
  have hws := wiresBefore_succ sk hk
  have hmono := wiresBefore_mono sk 0
    (show k + 1 ≤ sk.stageLen 0 from hk)
  omega

-- ================================= descent packages from the context

/-- The upper sites' descent package: the deep windows at the summary
cursor plus the site's own floor counts supply every level. -/
theorem descSupply_upper_of_ctx (hwf : sk.wellFormed = true)
    {fut : List Ev} {st : MState} (hW : WCount sk fut st) {h k : Nat}
    (h1 : 1 ≤ h) (hhr : h < sk.rootH) (hk : k < sk.stageLen h)
    (hasks : asks ((wpk h).1) (h + 1) = true) {X : Nat}
    (hXW : sk.wiresBefore h k ≤ X)
    (hXle : X ≤ sk.stageLen (h - 1))
    (hdeep : ∀ g', g' < h →
      fut.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSeg sk g' (descIdx sk g' (h - 1 - g') X)
            (sk.stageLen g'))
    (hlowh : sk.dsBefore h k ≤ sndCount (lowerOut (wpk h)) st.out)
    (hq1 : h = 1 →
      sk.qsBefore 1 k ≤ sndCount Chan.leafRequests st.out) :
    DescSupply sk st ((wpk h).1) h
      (sk.pendsBefore ((wpk h).1) (h + 1) k) := by
  have hcle : ∀ g', g' < h →
      descIdx sk g' (h - 1 - g') X ≤ sk.stageLen g' := by
    intro g' hg'
    refine descIdx_le_stageLen sk hwf ?_ ?_
    · rw [show g' + (h - 1 - g') = h - 1 from by omega]
      omega
    · rw [show g' + (h - 1 - g') = h - 1 from by omega]
      exact hXle
  have hkX : ∀ g', g' < h →
      descIdx sk g' (h - g') k ≤ descIdx sk g' (h - 1 - g') X := by
    intro g' hg'
    rw [show h - g' = h - 1 - g' + 1 from by omega, descIdx_succ,
      show g' + (h - 1 - g') + 1 = h from by omega]
    exact descIdx_mono sk g' (h - 1 - g') hXW
  refine descSupply_upper_site sk hwf h1 hhr hasks ?_ ?_ ?_
  · -- covered answerers' resolutions
    intro g hg1 hgh hna_g
    by_cases hgh' : g = h
    · subst hgh'
      rw [Nat.sub_self, descIdx_zero, lowerOut_eq_of_answerer hna_g]
      exact hlowh
    · have hlt : g < h := by omega
      rw [lowerOut_eq_of_answerer hna_g,
        deep_lower_count sk hwf hW (by omega) (hcle g hlt)
          (hdeep g hlt)]
      exact dsBefore_mono sk g (hkX g hlt)
  · -- covered askers' summaries
    intro g hg2 hasker_g
    have hna_g : asks ((wpk h).1) g = false := by
      have hs := asks_succ ((wpk h).1) g
      rw [hasker_g] at hs
      simpa using hs.symm
    rw [upperOut_eq_of_answerer hna_g,
      deep_upper_count sk hwf hW (by omega) (hcle g (by omega))
        (hdeep g (by omega))]
    exact hkX g (by omega)
  · -- the absorber feeds
    intro _
    have hk0 : descIdx sk 0 h k ≤ descIdx sk 0 (h - 1) X := by
      have hx := hkX 0 h1
      rw [Nat.sub_zero] at hx
      exact hx
    constructor
    · have hd0 := hdeep 0 h1
      rw [Nat.sub_zero] at hd0
      have hc0 := hcle 0 h1
      rw [Nat.sub_zero] at hc0
      rw [show Chan.wire Party.R 0 = wireOut (wpk 0) from rfl,
        deep_wire_count sk hwf hW (by omega) hc0 hd0]
      exact wiresBefore_mono sk 0 hk0
    · by_cases h1' : h = 1
      · subst h1'
        have hpeel : descIdx sk 0 1 k = sk.wiresBefore 1 k :=
          descIdx_peel sk 0 0 k
        rw [hpeel,
          ← qs_wires sk hwf (Nat.le_refl 1) hhr (Nat.le_of_lt hk)]
        exact hq1 rfl
      · have h2 : 2 ≤ h := by omega
        have hd1 := hdeep 1 (by omega)
        rw [show h - 1 - 1 = h - 2 from by omega] at hd1
        have hc1 := hcle 1 (by omega)
        rw [show h - 1 - 1 = h - 2 from by omega] at hc1
        rw [show Chan.leafRequests = askedOut (wpk 1) from rfl,
          deep_q_count sk hwf hW (Nat.le_refl 1) (by omega) hc1 hd1,
          qs_wires sk hwf (Nat.le_refl 1) (by omega) hc1]
        refine Nat.le_trans (wiresBefore_mono sk 0 hk0) ?_
        refine Nat.le_of_eq ?_
        have hp := descIdx_peel sk (h - 2) 0 X
        rw [show h - 2 + 1 = h - 1 from by omega] at hp
        exact congrArg (sk.wiresBefore 0) hp

/-- The lower site's descent package: the deep windows at the kid-slot
cursor plus the site's own floor counts supply every level below. -/
theorem descSupply_lower_of_ctx (hwf : sk.wellFormed = true)
    {fut : List Ev} {st : MState} (hW : WCount sk fut st)
    {h k i : Nat} (h1 : 1 ≤ h) (hhr : h < sk.rootH)
    (hk : k < sk.stageLen h)
    (hi : i ≤ sk.nChildren h (sk.stageScope h k))
    (hna : asks ((wpk h).1) h = false)
    (hdeep : ∀ g', g' < h →
      fut.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSeg sk g'
            (descIdx sk g' (h - 1 - g') (sk.wiresBefore h k + i))
            (sk.stageLen g'))
    (hq1 : h = 1 →
      sk.wiresBefore 0 (sk.wiresBefore 1 k + i)
        ≤ sndCount Chan.leafRequests st.out) :
    DescSupply sk st ((wpk h).1) (h - 1)
      (sk.pendsBefore ((wpk h).1) h
        (sk.dsBefore h k + dRank sk (wpk h) k i)) := by
  have hXle : sk.wiresBefore h k + i ≤ sk.stageLen (h - 1) := by
    have h1' := wiresBefore_succ sk hk
    have h2' := wiresBefore_mono sk h
      (show k + 1 ≤ sk.stageLen h from hk)
    have h3' := wiresBefore_total sk hwf h1 hhr
    omega
  have hcle : ∀ g', g' < h →
      descIdx sk g' (h - 1 - g') (sk.wiresBefore h k + i)
        ≤ sk.stageLen g' := by
    intro g' hg'
    refine descIdx_le_stageLen sk hwf ?_ ?_
    · rw [show g' + (h - 1 - g') = h - 1 from by omega]
      omega
    · rw [show g' + (h - 1 - g') = h - 1 from by omega]
      exact hXle
  refine descSupply_lower_site sk hwf hna h1 hhr hk hi ?_ ?_ ?_
  · intro g hg1 hgh hna_g
    rw [lowerOut_eq_of_answerer hna_g,
      deep_lower_count sk hwf hW (by omega) (hcle g (by omega))
        (hdeep g (by omega))]
    exact Nat.le_refl _
  · intro g hg2 hasker_g
    have hna_g : asks ((wpk h).1) g = false := by
      have hs := asks_succ ((wpk h).1) g
      rw [hasker_g] at hs
      simpa using hs.symm
    rw [upperOut_eq_of_answerer hna_g,
      deep_upper_count sk hwf hW (by omega) (hcle g (by omega))
        (hdeep g (by omega))]
    exact Nat.le_refl _
  · intro _
    constructor
    · have hd0 := hdeep 0 h1
      rw [Nat.sub_zero] at hd0
      have hc0 := hcle 0 h1
      rw [Nat.sub_zero] at hc0
      rw [show Chan.wire Party.R 0 = wireOut (wpk 0) from rfl,
        deep_wire_count sk hwf hW (by omega) hc0 hd0]
      exact Nat.le_refl _
    · by_cases h1' : h = 1
      · subst h1'
        exact hq1 rfl
      · have h2 : 2 ≤ h := by omega
        have hd1 := hdeep 1 (by omega)
        rw [show h - 1 - 1 = h - 2 from by omega] at hd1
        have hc1 := hcle 1 (by omega)
        rw [show h - 1 - 1 = h - 2 from by omega] at hc1
        rw [show Chan.leafRequests = askedOut (wpk 1) from rfl,
          deep_q_count sk hwf hW (Nat.le_refl 1) (by omega) hc1 hd1,
          qs_wires sk hwf (Nat.le_refl 1) (by omega) hc1]
        refine Nat.le_of_eq ?_
        have hp := descIdx_peel sk (h - 2) 0
          (sk.wiresBefore h k + i)
        rw [show h - 2 + 1 = h - 1 from by omega] at hp
        exact congrArg (sk.wiresBefore 0) hp

/-- Coverage extends one asker stage down vacuously. -/
theorem ascCover_pred {st : MState} {p : Party} {j top : Nat}
    (h : AscCover sk st p (j + 1) top) (hasker : asks p j = true) :
    AscCover sk st p j top := by
  intro g hjg hgt hna
  by_cases hgj : g = j
  · subst hgj
    rw [hna] at hasker
    exact Bool.noConfusion hasker
  · exact h g (by omega) hgt hna

-- ==================================================== window sites

/-- A window conclusion opens the guard: the cap has a free slot. -/
private theorem enabled_of_window {st : MState} {c : Chan} {n : Nat}
    (hwf : sk.wellFormed = true) (hwin : n ≤ rcvCount c st.out)
    (hrcvd : st.rcvd c = rcvCount c st.out) :
    enabled sk st.sent st.rcvd (c, true, n) = true := by
  simp only [enabled, decide_eq_true_eq]
  have hcap := cap_pos hwf c
  omega

/-- The prologue-summary site (U1): an undisputed scope's parent
summary is emittable through the upper window. -/
theorem ready_upper_prologue (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) {fut : List Ev} {h k : Nat}
    {A j t : Nat → Nat} (hhr : h < sk.rootH) (hk : k < sk.stageLen h)
    (hlast : lastDOf sk h k = none) (hanc : AncTele sk h A j t fut)
    (hcoh0 : h + 1 < sk.rootH →
      k = sk.wiresBefore (h + 1) (A (h + 1)) + j (h + 1))
    (hfeed : ∃ i₀, fut.filter (fun e => evOwner sk e == 1)
      = ((ropenEvents sk).drop 3).drop i₀)
    (hdeep : ∀ g', g' < h →
      fut.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSeg sk g'
            (descIdx sk g' (h - 1 - g') (sk.wiresBefore h k))
            (sk.stageLen g'))
    (hown : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((upperOut (wpk h), true, k) : Ev)
          :: ((List.range' 0 (sk.nChildren h (sk.stageScope h k))).flatMap
                (splicedChunk sk h k (lastDOf sk h k))
              ++ walkSeg sk h (k + 1) (sk.stageLen h))) :
    ∀ st : MState, WEdge sk fut st → step sk st = none →
      enabled sk st.sent st.rcvd (upperOut (wpk h), true, k)
        = true := by
  intro st hW hfix
  have hna : asks ((wpk h).1) h = false := asks_wpk_self h
  have hasks : asks ((wpk h).1) (h + 1) = true := by
    have hs := asks_succ ((wpk h).1) h
    rw [hna] at hs
    simpa using hs
  have hfu := futLen_site_upper_prologue sk hk hlast hown
  have hsnd : sndCount (Chan.upper ((wpk h).1) h) st.out = k :=
    upper_site_hsnd sk hwf hW.toWCount hna hhr hk hfu
  have hcov := ancTele_cov sk hwf hsched hW hanc hcoh0 hsnd
  have hroot := root_banked sk hwf hW.toWCount hfeed
  have hdesc : DescSupply sk st ((wpk h).1) h
      (sk.pendsBefore ((wpk h).1) (h + 1) k) := by
    rcases Nat.eq_zero_or_pos h with rfl | h1
    · exact descSupply_upper_site_zero sk hasks _
    · have hflow := futLen_S1_res sk hk hown
      have hlpin := lower_snd_pin sk hwf hW.toWCount hhr
      have hdmono := dsBefore_mono sk h
        (show k ≤ sk.stageLen h from Nat.le_of_lt hk)
      have hXle : sk.wiresBefore h k ≤ sk.stageLen (h - 1) := by
        have h2' := wiresBefore_mono sk h
          (show k ≤ sk.stageLen h from Nat.le_of_lt hk)
        have h3' := wiresBefore_total sk hwf h1 hhr
        omega
      refine descSupply_upper_of_ctx sk hwf hW.toWCount h1 hhr hk
        hasks (Nat.le_refl _) hXle hdeep (by omega) ?_
      intro h1'
      subst h1'
      have hfq := futLen_S1_q sk hk hown
      have hqpin := asked_snd_pin sk hwf hW.toWCount (Nat.le_refl 1)
        hhr
      have hqmono := qsBefore_mono sk 1
        (show k ≤ sk.stageLen 1 from Nat.le_of_lt hk)
      rw [show Chan.leafRequests = askedOut (wpk 1) from rfl]
      omega
  have hwin := upper_window sk hwf hW hfix (wpk_htop sk h) hasks
    (wtop_ge hwf hhr) hk hsnd hdesc hcov hroot
  exact enabled_of_window sk hwf hwin (hW.rcvd_eq _)

/-- The splice-summary site (U2): a disputed scope's parent summary,
emitted after its last resolution, is emittable through the upper
window. -/
theorem ready_upper_splice (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) {fut : List Ev} {h k jL : Nat}
    {A j t : Nat → Nat} (hhr : h < sk.rootH) (hk : k < sk.stageLen h)
    (hlast : lastDOf sk h k = some jL) (hanc : AncTele sk h A j t fut)
    (hcoh0 : h + 1 < sk.rootH →
      k = sk.wiresBefore (h + 1) (A (h + 1)) + j (h + 1))
    (hfeed : ∃ i₀, fut.filter (fun e => evOwner sk e == 1)
      = ((ropenEvents sk).drop 3).drop i₀)
    (hdeep : ∀ g', g' < h →
      fut.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSeg sk g'
            (descIdx sk g' (h - 1 - g') (sk.wiresBefore h k + jL))
            (sk.stageLen g'))
    (hown : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((upperOut (wpk h), true, k) : Ev)
          :: (chunkQ sk h k jL
              ++ (List.range' (jL + 1)
                    (sk.nChildren h (sk.stageScope h k)
                      - (jL + 1))).flatMap
                   (splicedChunk sk h k (lastDOf sk h k))
              ++ walkSeg sk h (k + 1) (sk.stageLen h))) :
    ∀ st : MState, WEdge sk fut st → step sk st = none →
      enabled sk st.sent st.rcvd (upperOut (wpk h), true, k)
        = true := by
  intro st hW hfix
  obtain ⟨hDL, hjn⟩ := lastDOf_isD sk hlast
  have h1 : 1 ≤ h := by
    rcases Nat.eq_zero_or_pos h with rfl | h1
    · exact Bool.noConfusion
        ((show sk.childIsD 0 (sk.stageScope 0 k) jL = false from rfl)
          ▸ hDL)
    · exact h1
  have hna : asks ((wpk h).1) h = false := asks_wpk_self h
  have hasks : asks ((wpk h).1) (h + 1) = true := by
    have hs := asks_succ ((wpk h).1) h
    rw [hna] at hs
    simpa using hs
  have hfu := futLen_site_upper_splice sk hk hlast hown
  have hsnd : sndCount (Chan.upper ((wpk h).1) h) st.out = k :=
    upper_site_hsnd sk hwf hW.toWCount hna hhr hk hfu
  have hcov := ancTele_cov sk hwf hsched hW hanc hcoh0 hsnd
  have hroot := root_banked sk hwf hW.toWCount hfeed
  have hdesc : DescSupply sk st ((wpk h).1) h
      (sk.pendsBefore ((wpk h).1) (h + 1) k) := by
    have hflow := futLen_S2_res sk hk hlast hown
    have hlpin := lower_snd_pin sk hwf hW.toWCount hhr
    have hdmono := dsBefore_mono sk h
      (show k ≤ sk.stageLen h from Nat.le_of_lt hk)
    have hXle : sk.wiresBefore h k + jL ≤ sk.stageLen (h - 1) := by
      have h1' := wiresBefore_succ sk hk
      have h2' := wiresBefore_mono sk h
        (show k + 1 ≤ sk.stageLen h from hk)
      have h3' := wiresBefore_total sk hwf h1 hhr
      omega
    refine descSupply_upper_of_ctx sk hwf hW.toWCount h1 hhr hk
      hasks (by omega) hXle hdeep (by omega) ?_
    intro h1'
    subst h1'
    have hfq := futLen_S2_q sk hk hlast hown
    have hqpin := asked_snd_pin sk hwf hW.toWCount (Nat.le_refl 1)
      hhr
    have hqmono := qsBefore_mono sk 1
      (show k ≤ sk.stageLen 1 from Nat.le_of_lt hk)
    rw [show Chan.leafRequests = askedOut (wpk 1) from rfl]
    omega
  have hwin := upper_window sk hwf hW hfix (wpk_htop sk h) hasks
    (wtop_ge hwf hhr) hk hsnd hdesc hcov hroot
  exact enabled_of_window sk hwf hwin (hW.rcvd_eq _)

/-- The resolution site (L): a disputed slot's resolution is
emittable through the lower window. -/
theorem ready_lower (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) {fut : List Ev} {h k i : Nat}
    {A j t : Nat → Nat} (hhr : h < sk.rootH) (hk : k < sk.stageLen h)
    (hi : i < sk.nChildren h (sk.stageScope h k))
    (hD : sk.childIsD h (sk.stageScope h k) i = true)
    (hanc : AncTele sk h A j t fut)
    (hcoh0 : h + 1 < sk.rootH →
      k = sk.wiresBefore (h + 1) (A (h + 1)) + j (h + 1))
    (hfeed : ∃ i₀, fut.filter (fun e => evOwner sk e == 1)
      = ((ropenEvents sk).drop 3).drop i₀)
    (hdeep : ∀ g', g' < h →
      fut.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSeg sk g'
            (descIdx sk g' (h - 1 - g') (sk.wiresBefore h k + i))
            (sk.stageLen g'))
    (hown : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((lowerOut (wpk h), true,
            sk.dsBefore h k + dRank sk (wpk h) k i) : Ev)
          :: ((if lastDOf sk h k == some i
                then [((upperOut (wpk h), true, k) : Ev)] else [])
              ++ chunkQ sk h k i
              ++ (List.range' (i + 1)
                    (sk.nChildren h (sk.stageScope h k)
                      - (i + 1))).flatMap
                   (splicedChunk sk h k (lastDOf sk h k))
              ++ walkSeg sk h (k + 1) (sk.stageLen h))) :
    ∀ st : MState, WEdge sk fut st → step sk st = none →
      enabled sk st.sent st.rcvd
        (lowerOut (wpk h), true,
          sk.dsBefore h k + dRank sk (wpk h) k i) = true := by
  intro st hW hfix
  have h1 : 1 ≤ h := by
    rcases Nat.eq_zero_or_pos h with rfl | h1
    · exact Bool.noConfusion
        ((show sk.childIsD 0 (sk.stageScope 0 k) i = false from rfl)
          ▸ hD)
    · exact h1
  have hna : asks ((wpk h).1) h = false := asks_wpk_self h
  have hasks : asks ((wpk h).1) (h + 1) = true := by
    have hs := asks_succ ((wpk h).1) h
    rw [hna] at hs
    simpa using hs
  obtain ⟨hfl, hbnd, hfu⟩ := futLen_site_lower sk hk hi hD hown
  have hsnd := lower_site_hsnd sk hwf hW.toWCount hna hhr hfl hbnd
  have hupk := upper_site_hsnd sk hwf hW.toWCount hna hhr hk hfu
  have hp1full := p1_of_lower_site sk hsched hk hi hD hupk hsnd
  have hroot := root_banked sk hwf hW.toWCount hfeed
  have hcov : AscCover sk st ((wpk h).1) (h + 1) (wtop sk h) :=
    ascCover_pred sk (ancTele_cov sk hwf hsched hW hanc hcoh0 hupk)
      hasks
  have hq1 : h = 1 →
      sk.wiresBefore 0 (sk.wiresBefore 1 k + i)
        ≤ sndCount Chan.leafRequests st.out := by
    intro h1'
    subst h1'
    have hfq := futLen_SL_q sk hk hi hown
    have hqpin := asked_snd_pin sk hwf hW.toWCount (Nat.le_refl 1)
      hhr
    have hqw := qs_wires_mid sk hwf (Nat.le_refl 1) hhr hk
      (Nat.le_of_lt hi)
    rw [show (1 : Nat) - 1 = 0 from rfl] at hqw
    have hqs1 : qSum sk (wpk 1) k i
        ≤ qSum sk (wpk 1) k (sk.nChildren 1 (sk.stageScope 1 k)) :=
      qSum_mono sk (wpk 1) k (Nat.le_of_lt hi)
    have htotq : qSum sk (wpk 1) k
        (sk.nChildren 1 (sk.stageScope 1 k))
        = sk.qOf 1 (sk.stageScope 1 k) := qSum_total sk (wpk 1) k
    have hqsuc := qsBefore_succ sk hk
    have hqmono := qsBefore_mono sk 1
      (show k + 1 ≤ sk.stageLen 1 from hk)
    rw [show Chan.leafRequests = askedOut (wpk 1) from rfl]
    omega
  have hdesc := descSupply_lower_of_ctx sk hwf hW.toWCount h1 hhr hk
    (Nat.le_of_lt hi) hna hdeep hq1
  have hd : sk.dsBefore h k + dRank sk (wpk h) k i
      < (sk.asmResList ((wpk h).1) h).length := by
    rw [answerer_resList_total hwf hna h1 hhr]
    exact hbnd
  rw [hsnd] at hp1full
  have hwin := lower_window sk hwf hW hfix (wpk_htop sk h) hna h1
    (show h < wtop sk h from by have := wtop_ge hwf hhr; omega)
    hd hsnd hp1full hdesc hcov hroot
  exact enabled_of_window sk hwf hwin (hW.rcvd_eq _)

/-- The leaf-wire site (W0): a leaf slot's wire is emittable through
the absorber's wire window. -/
theorem ready_wire0 (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) {fut : List Ev} {k i0 : Nat}
    {A j t : Nat → Nat} (hr : 0 < sk.rootH) (hk : k < sk.stageLen 0)
    (hi0 : i0 < sk.nChildren 0 (sk.stageScope 0 k))
    (hanc : AncTele sk 0 A j t fut)
    (hcoh0 : k = sk.wiresBefore 1 (A 1) + j 1) (ht1 : t 1 = i0)
    (hfeed : ∃ i₀, fut.filter (fun e => evOwner sk e == 1)
      = ((ropenEvents sk).drop 3).drop i₀)
    (hown : fut.filter (fun e => evOwner sk e == walkIdx sk 0)
      = (List.range' i0
            (sk.nChildren 0 (sk.stageScope 0 k) - i0)).flatMap
            (splicedChunk sk 0 k (lastDOf sk 0 k))
          ++ walkSeg sk 0 (k + 1) (sk.stageLen 0)) :
    ∀ st : MState, WEdge sk fut st → step sk st = none →
      enabled sk st.sent st.rcvd
        (wireOut (wpk 0), true, sk.wiresBefore 0 k + i0) = true := by
  intro st hW hfix
  have hr2 : 1 < sk.rootH := by have := (wf_rootH hwf).2; omega
  obtain ⟨hA1, hj1⟩ := hanc.rng 1 (by omega) hr2
  obtain ⟨hfw, hwbnd⟩ := futLen_site_wire sk hk hi0 hown
  have hsnd := wire0_site_hsnd sk hwf hW.toWCount hr hfw hwbnd
  have hw : sk.wiresBefore 0 k + i0 < sk.totalLeafReqs := by
    have := wiresBefore_full_leaf hwf
    omega
  have hqc : sk.qCount 1 (sk.stageScope 1 (A 1)) (j 1)
      = sk.nChildren 0 (sk.stageScope 0 k) := by
    have hq := qCount_eq_kid_nChildren sk hwf (Nat.le_refl 1) hr2
      hA1 hj1
    rw [show (1 : Nat) - 1 = 0 from rfl, ← hcoh0] at hq
    exact hq
  obtain ⟨hfq, hqbnd⟩ := futLen_site_q sk hA1 hj1
    (by rw [ht1, hqc]; exact hi0) (hanc.fil 1 (by omega) hr2)
  have hsndq := leafreq_site_hsnd sk hwf hW.toWCount hr2 hfq hqbnd
  have hqw := qs_wires_mid sk hwf (Nat.le_refl 1) hr2 hA1
    (Nat.le_of_lt hj1)
  rw [show (1 : Nat) - 1 = 0 from rfl] at hqw
  rw [hqw, ← hcoh0, ht1] at hsndq
  have hreq : sk.wiresBefore 0 k + i0
      ≤ sndCount Chan.leafRequests st.out + 1 := by omega
  have hcov := ancTele_cov_leaf sk hwf hsched hW hanc hr2 hk hcoh0
    hi0 hsndq
  have hroot := root_banked sk hwf hW.toWCount hfeed
  have hwin := wire0_window sk hwf hW hfix hw hsnd hreq hcov hroot
  exact enabled_of_window sk hwf hwin (hW.rcvd_eq _)

/-- The leaf-request site (Q0): a leaf slot's feed query is emittable
through the absorber's request window. -/
theorem ready_leafreq (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) {fut : List Ev} {k i0 : Nat}
    {A j t : Nat → Nat} (hr : 0 < sk.rootH) (hk : k < sk.stageLen 0)
    (hi0 : i0 < sk.nChildren 0 (sk.stageScope 0 k))
    (hanc : AncTele sk 0 A j t fut)
    (hcoh0 : k = sk.wiresBefore 1 (A 1) + j 1) (ht1 : t 1 = i0)
    (hfeed : ∃ i₀, fut.filter (fun e => evOwner sk e == 1)
      = ((ropenEvents sk).drop 3).drop i₀)
    (hown : fut.filter (fun e => evOwner sk e == walkIdx sk 0)
      = (List.range' (i0 + 1)
            (sk.nChildren 0 (sk.stageScope 0 k) - (i0 + 1))).flatMap
            (splicedChunk sk 0 k (lastDOf sk 0 k))
          ++ walkSeg sk 0 (k + 1) (sk.stageLen 0)) :
    ∀ st : MState, WEdge sk fut st → step sk st = none →
      enabled sk st.sent st.rcvd
        (askedOut (wpk 1), true, sk.wiresBefore 0 k + i0) = true := by
  intro st hW hfix
  have hr2 : 1 < sk.rootH := by have := (wf_rootH hwf).2; omega
  obtain ⟨hA1, hj1⟩ := hanc.rng 1 (by omega) hr2
  have hqc : sk.qCount 1 (sk.stageScope 1 (A 1)) (j 1)
      = sk.nChildren 0 (sk.stageScope 0 k) := by
    have hq := qCount_eq_kid_nChildren sk hwf (Nat.le_refl 1) hr2
      hA1 hj1
    rw [show (1 : Nat) - 1 = 0 from rfl, ← hcoh0] at hq
    exact hq
  obtain ⟨hfq, hqbnd⟩ := futLen_site_q sk hA1 hj1
    (by rw [ht1, hqc]; exact hi0) (hanc.fil 1 (by omega) hr2)
  have hsndq := leafreq_site_hsnd sk hwf hW.toWCount hr2 hfq hqbnd
  have hqw := qs_wires_mid sk hwf (Nat.le_refl 1) hr2 hA1
    (Nat.le_of_lt hj1)
  rw [show (1 : Nat) - 1 = 0 from rfl] at hqw
  rw [hqw, ← hcoh0, ht1] at hsndq
  have hq : sk.wiresBefore 0 k + i0 < sk.totalLeafReqs := by
    have hfull := qsBefore_full_leaf hwf
    have hcong : sk.wiresBefore 0 k
        = sk.wiresBefore 0 (sk.wiresBefore 1 (A 1) + j 1) := by
      rw [hcoh0]
    omega
  have hfw := futLen_Q0_wire sk hk hi0 hown
  have hwpin := wire_snd_pin sk hwf hW.toWCount hr
  have hwire : sk.wiresBefore 0 k + i0
      ≤ sndCount (Chan.wire Party.R 0) st.out := by
    rw [show Chan.wire Party.R 0 = wireOut (wpk 0) from rfl]
    have hws := wiresBefore_succ sk hk
    have hmono := wiresBefore_mono sk 0
      (show k + 1 ≤ sk.stageLen 0 from hk)
    omega
  have hcov := ancTele_cov_leaf sk hwf hsched hW hanc hr2 hk hcoh0
    hi0 hsndq
  have hroot := root_banked sk hwf hW.toWCount hfeed
  have hwin := leafreq_window sk hwf hW hfix hq hsndq hwire hcov
    hroot
  exact enabled_of_window sk hwf hwin (hW.rcvd_eq _)

-- ====================================== the tree induction's plumbing

/-- Walk indices are distinct across stages under the root. -/
theorem walkIdx_inj {sk : Skel} {a b : Nat} (ha : a < sk.rootH)
    (hb : b < sk.rootH) (h : walkIdx sk a = walkIdx sk b) : a = b := by
  unfold walkIdx at h
  omega

/-- A subtree owns nothing at a foreign index: everything inside is
the feeder's or a covered walk's. -/
theorem scope_filter_ne (hwf : sk.wellFormed = true) {h k : Nat}
    {F : List Ev} {mF M : Nat} (hh : h < sk.rootH)
    (hk : k < sk.stageLen h)
    (hF : F.length = sk.nChildren h (sk.stageScope h k))
    (hFo : ∀ e ∈ F, evOwner sk e = mF) (hmF : mF < walkIdx sk h)
    (hMne : mF ≠ M) (hMhigh : ∀ h', h' ≤ h → walkIdx sk h' ≠ M) :
    (opEvents sk (.scope h k F)).filter
      (fun e => evOwner sk e == M) = [] := by
  rw [List.filter_eq_nil_iff]
  intro e he
  rcases (align_scope sk hwf h k F mF hh hk hF hFo hmF).1 e he with
    ho | ⟨h', hle, ho⟩
  · simp only [ho, beq_iff_eq]
    exact hMne
  · simp only [ho, beq_iff_eq]
    exact hMhigh h' hle

/-- A kid suffix owns nothing at a foreign index. -/
theorem kids_filter_ne (hwf : sk.wellFormed = true) {h k : Nat}
    {F : List Ev} {mF M : Nat} (hh : h < sk.rootH)
    (hk : k < sk.stageLen h)
    (hF : F.length = sk.nChildren h (sk.stageScope h k))
    (hFo : ∀ e ∈ F, evOwner sk e = mF) (hmF : mF < walkIdx sk h)
    {i : Nat} (hi : i ≤ sk.nChildren h (sk.stageScope h k))
    (hMne : mF ≠ M) (hMhigh : ∀ h', h' ≤ h → walkIdx sk h' ≠ M) :
    ((List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
        (fun i' => opEvents sk (.kid h k (sk.stageScope h k)
          (lastDOf sk h k) (sk.wiresBefore h k) i' F))).filter
      (fun e => evOwner sk e == M) = [] := by
  rw [List.filter_eq_nil_iff]
  intro e he
  rcases (align_kids_suffix sk hwf hh hk hF hFo hmF hi).1 e he with
    ho | ⟨h', hle, ho⟩
  · simp only [ho, beq_iff_eq]
    exact hMne
  · simp only [ho, beq_iff_eq]
    exact hMhigh h' hle

/-- The consumed-head merge: a positional read glued back onto the
remaining suffix is the suffix one shorter. -/
theorem toList_drop_merge {α : Type _} {l : List α} {i : Nat}
    (hi : i < l.length) :
    l[i]?.toList ++ l.drop (i + 1) = l.drop i := by
  rw [List.getElem?_eq_getElem hi, Option.toList_some,
    List.singleton_append, ← List.drop_eq_getElem_cons hi]

/-- Rebase the telescope across a local prefix: stages two or more up
see nothing in the prefix, and the immediate parent's cursor moves to
the prefix's feed position. -/
theorem ancTele_rebase {h : Nat} {A j t : Nat → Nat}
    {pre rest : List Ev} (hanc : AncTele sk h A j t rest)
    (hnil : ∀ G, h + 2 ≤ G → G < sk.rootH →
      pre.filter (fun e => evOwner sk e == walkIdx sk G) = [])
    {c : Nat}
    (hpar : h + 1 < sk.rootH →
      (pre ++ rest).filter
          (fun e => evOwner sk e == walkIdx sk (h + 1))
        = (chunkQ sk (h + 1) (A (h + 1)) (j (h + 1))).drop c
          ++ (List.range' (j (h + 1) + 1)
                (sk.nChildren (h + 1)
                    (sk.stageScope (h + 1) (A (h + 1)))
                  - (j (h + 1) + 1))).flatMap
               (splicedChunk sk (h + 1) (A (h + 1))
                 (lastDOf sk (h + 1) (A (h + 1))))
          ++ walkSeg sk (h + 1) (A (h + 1) + 1)
              (sk.stageLen (h + 1))) :
    AncTele sk h A j (fun G => if G = h + 1 then c else t G)
      (pre ++ rest) := by
  refine ⟨hanc.rng, hanc.isD, hanc.coh, ?_⟩
  intro G hG hGr
  by_cases hG1 : G = h + 1
  · subst hG1
    simp only [reduceIte]
    exact hpar hGr
  · simp only [if_neg hG1]
    rw [List.filter_append, hnil G (by omega) hGr, List.nil_append]
    exact hanc.fil G hG hGr

/-- The deep windows at a mid-scope slot: the kid suffix's windows
glued to the after-scope remainder. -/
theorem deep_glue (hwf : sk.wellFormed = true) {h k : Nat}
    (hhr : h < sk.rootH) (hk : k < sk.stageLen h) {i : Nat}
    (hi : i ≤ sk.nChildren h (sk.stageScope h k))
    {suffix rest : List Ev}
    (hsuf : ∀ g', g' < h →
      suffix.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSeg sk g'
            (descIdx sk g' (h - 1 - g') (sk.wiresBefore h k + i))
            (descIdx sk g' (h - 1 - g')
              (sk.wiresBefore h k
                + sk.nChildren h (sk.stageScope h k))))
    (hrest : ∀ g', g' ≤ h →
      rest.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSeg sk g' (descIdx sk g' (h - g') (k + 1))
            (sk.stageLen g')) :
    ∀ g', g' < h →
      (suffix ++ rest).filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSeg sk g'
            (descIdx sk g' (h - 1 - g') (sk.wiresBefore h k + i))
            (sk.stageLen g') := by
  intro g' hg'
  rw [List.filter_append, hsuf g' hg', hrest g' (Nat.le_of_lt hg')]
  have hbc : descIdx sk g' (h - g') (k + 1)
      = descIdx sk g' (h - 1 - g')
          (sk.wiresBefore h k
            + sk.nChildren h (sk.stageScope h k)) := by
    rw [show sk.wiresBefore h k + sk.nChildren h (sk.stageScope h k)
        = sk.wiresBefore h (k + 1) from (wiresBefore_succ sk hk).symm,
      show h - g' = h - 1 - g' + 1 from by omega, descIdx_succ,
      show g' + (h - 1 - g') + 1 = h from by omega]
  rw [hbc]
  have hmono := descIdx_mono sk g' (h - 1 - g')
    (show sk.wiresBefore h k + i
        ≤ sk.wiresBefore h k + sk.nChildren h (sk.stageScope h k)
      from by omega)
  have hend : descIdx sk g' (h - 1 - g')
      (sk.wiresBefore h k + sk.nChildren h (sk.stageScope h k))
      ≤ sk.stageLen g' := by
    rw [← hbc]
    refine descIdx_le_stageLen sk hwf ?_ ?_
    · rw [show g' + (h - g') = h from by omega]
      exact hhr
    · rw [show g' + (h - g') = h from by omega]
      exact hk
  exact walkSeg_glue sk hmono hend

/-- A prologue wire receive discharges from its in-flight send. -/
theorem head_rcv_wire (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (hW : WCount sk fut st) {p : Party} {hh n : Nat}
    (hpred : ∀ d, manDep ((Chan.wire p hh, false, n) : Ev) = some d →
      d ∈ st.out) :
    enabled sk st.sent st.rcvd (Chan.wire p hh, false, n) = true :=
  enabled_rcv_of_mem sk hwf hW (hpred _ (manDep_wire_rcv p hh n))

/-- A prologue asked receive discharges from its in-flight send. -/
theorem head_rcv_asked (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (hW : WCount sk fut st) {p : Party} {hh n : Nat}
    (hpred : ∀ d, manDep ((Chan.asked p hh, false, n) : Ev) = some d →
      d ∈ st.out) :
    enabled sk st.sent st.rcvd (Chan.asked p hh, false, n) = true :=
  enabled_rcv_of_mem sk hwf hW (hpred _ (manDep_asked_rcv p hh n))

/-- A manual-consumed wire send discharges from its predecessor
receive, or opens on a fresh window at seq zero. -/
theorem head_snd_wire (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (hW : WCount sk fut st) {p : Party} {hh n : Nat}
    (hh1 : 1 ≤ hh)
    (hpred : ∀ d, manDep ((Chan.wire p hh, true, n) : Ev) = some d →
      d ∈ st.out) :
    enabled sk st.sent st.rcvd (Chan.wire p hh, true, n) = true := by
  rcases Nat.eq_zero_or_pos n with rfl | hn
  · exact enabled_snd_low sk (cap_pos hwf _)
  · have hc : sk.cap (Chan.wire p hh) = 1 := rfl
    refine enabled_snd_of_mem sk hwf hW ?_ (by omega)
    have hmem := hpred _ (manDep_wire_snd_pos (by omega) (by omega))
    rw [hc]
    exact hmem

/-- A manual-consumed query send discharges from its predecessor
receive, or opens on a fresh window at seq zero. -/
theorem head_snd_asked (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (hW : WCount sk fut st) {p : Party} {hh n : Nat}
    (hpred : ∀ d, manDep ((Chan.asked p hh, true, n) : Ev) = some d →
      d ∈ st.out) :
    enabled sk st.sent st.rcvd (Chan.asked p hh, true, n) = true := by
  rcases Nat.eq_zero_or_pos n with rfl | hn
  · exact enabled_snd_low sk (cap_pos hwf _)
  · have hc : sk.cap (Chan.asked p hh) = 1 := rfl
    refine enabled_snd_of_mem sk hwf hW ?_ (by omega)
    have hmem := hpred _ (manDep_asked_snd_pos (by omega))
    rw [hc]
    exact hmem

-- ============================================== the leaf-stage scopes

/-- The master induction's leaf case: every emission of a leaf scope
is ready — the prologue receives from their in-flight predecessors,
the summary through the upper window, each slot's wire and feed query
through the absorber windows. -/
theorem emitOK_scope_zero (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) {k : Nat} {rest : List Ev}
    {A j t : Nat → Nat} (hk : k < sk.stageLen 0)
    (hlow : ∀ g', g' ≤ 0 →
      rest.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSeg sk g' (descIdx sk g' (0 - g') (k + 1))
            (sk.stageLen g'))
    (hanc : AncTele sk 0 A j t rest)
    (hcoh0 : k = sk.wiresBefore 1 (A 1) + j 1)
    (hsat : (chunkQ sk 1 (A 1) (j 1)).drop (t 1) = [])
    (hfd : ∃ i₀, rest.filter (fun e => evOwner sk e == 1)
      = ((ropenEvents sk).drop 3).drop i₀) :
    EmitOKOn sk (opEvents sk (.scope 0 k (scopeFeed sk 0 k))) rest := by
  have hr2 : 1 < sk.rootH := by have := (wf_rootH hwf).2; omega
  have hr0 : 0 < sk.rootH := by omega
  obtain ⟨hA1, hj1⟩ := hanc.rng 1 (by omega) hr2
  -- the feed's provenance
  have hF : (scopeFeed sk 0 k).length
      = sk.nChildren 0 (sk.stageScope 0 k) := scopeFeed_length sk 0 k
  have hFo : ∀ e ∈ scopeFeed sk 0 k, evOwner sk e = walkIdx sk 1 := by
    intro e he
    unfold scopeFeed seg at he
    obtain ⟨j', -, rfl⟩ := List.mem_map.1 he
    exact evOwner_askedOut sk (Nat.le_refl 1) hr2 _
  have hmF : walkIdx sk 1 < walkIdx sk 0 :=
    walkIdx_lt sk (by omega) hr2
  have hw2 : ∀ h', h' < sk.rootH → 2 ≤ walkIdx sk h' := by
    intro h' _
    unfold walkIdx
    omega
  have hFeq : chunkQ sk 1 (A 1) (j 1) = scopeFeed sk 0 k := by
    have hq := chunkQ_eq_feed sk hwf (hp := 0) hr2 hA1 hj1
    rw [← hcoh0] at hq
    exact hq
  -- the after-scope stage-0 window
  have hlow0 : rest.filter (fun e => evOwner sk e == walkIdx sk 0)
      = walkSeg sk 0 (k + 1) (sk.stageLen 0) := by
    have h0 := hlow 0 (Nat.le_refl 0)
    rw [show ((0 : Nat) - 0) = 0 from rfl, descIdx_zero] at h0
    exact h0
  -- the immediate parent's remainder, feed cursor spent
  have hfil1 : rest.filter (fun e => evOwner sk e == walkIdx sk 1)
      = (List.range' (j 1 + 1)
            (sk.nChildren 1 (sk.stageScope 1 (A 1)) - (j 1 + 1))).flatMap
           (splicedChunk sk 1 (A 1) (lastDOf sk 1 (A 1)))
        ++ walkSeg sk 1 (A 1 + 1) (sk.stageLen 1) := by
    have hf := hanc.fil 1 (by omega) hr2
    rw [hsat, List.nil_append] at hf
    exact hf
  -- the parent-cursor rebuild at feed position c
  have hpar : ∀ (pre : List Ev) (c : Nat),
      pre.filter (fun e => evOwner sk e == walkIdx sk 1)
        = (scopeFeed sk 0 k).drop c →
      (pre ++ rest).filter (fun e => evOwner sk e == walkIdx sk 1)
        = (chunkQ sk 1 (A 1) (j 1)).drop c
          ++ (List.range' (j 1 + 1)
                (sk.nChildren 1 (sk.stageScope 1 (A 1))
                  - (j 1 + 1))).flatMap
               (splicedChunk sk 1 (A 1) (lastDOf sk 1 (A 1)))
          ++ walkSeg sk 1 (A 1 + 1) (sk.stageLen 1) := by
    intro pre c hpre
    rw [List.filter_append, hpre, hfil1, hFeq, ← List.append_assoc]
  -- the openers' share is never touched below the root
  have hone : ∀ (pre : List Ev),
      pre.filter (fun e => evOwner sk e == 1) = [] →
      ∃ i₀, (pre ++ rest).filter (fun e => evOwner sk e == 1)
        = ((ropenEvents sk).drop 3).drop i₀ := by
    intro pre hpre
    obtain ⟨i₀, hf⟩ := hfd
    exact ⟨i₀, by rw [List.filter_append, hpre, List.nil_append, hf]⟩
  -- the expansion
  have hLn : lastDOf sk 0 k = none := by
    unfold lastDOf
    rw [List.getLast?_eq_none_iff, List.filter_eq_nil_iff]
    intro a _
    rw [show sk.childIsD 0 (sk.stageScope 0 k) a = false from rfl]
    exact Bool.false_ne_true
  have hbe : (lastDOf sk 0 k == none) = true := by rw [hLn]; rfl
  have hE := opEvents_scope_eq sk (Nat.le_of_lt hk) (scopeFeed sk 0 k)
  rw [if_pos hbe, List.range_eq_range'] at hE
  rw [hE]
  -- kid-suffix clauses from the top
  obtain ⟨-, hks2, hks3, -⟩ :=
    align_kids_suffix sk hwf hr0 hk hF hFo hmF (Nat.zero_le _)
  rw [Nat.sub_zero] at hks2 hks3
  rw [List.drop_zero] at hks2
  -- peel the two prologue receives
  refine emitOKOn_cons sk
    (fun st hW hfix hpred => head_rcv_wire sk hwf hW.toWCount hpred) ?_
  refine emitOKOn_cons sk
    (fun st hW hfix hpred => head_rcv_asked sk hwf hW.toWCount hpred)
    ?_
  -- the summary, then the slots
  refine emitOKOn_append sk ?_ ?_
  · -- U1 at the leaf stage
    refine emitOKOn_cons sk ?_ (emitOKOn_nil sk _)
    intro st hW hfix hpred
    rw [List.nil_append] at hW
    have hprenil : ∀ G, 0 + 2 ≤ G → G < sk.rootH →
        (((upperOut (wpk 0), true, k) : Ev)
            :: (List.range' 0
                (sk.nChildren 0 (sk.stageScope 0 k))).flatMap
              (fun i => opEvents sk (.kid 0 k (sk.stageScope 0 k)
                (lastDOf sk 0 k) (sk.wiresBefore 0 k) i
                (scopeFeed sk 0 k)))).filter
          (fun e => evOwner sk e == walkIdx sk G) = [] := by
      intro G hG2 hGr
      rw [List.filter_cons_of_neg (by
          simp only [evOwner_upperOut, beq_iff_eq]
          intro hc
          exact absurd (walkIdx_inj hr0 hGr hc) (by omega))]
      have hkn := kids_filter_ne sk hwf hr0 hk hF hFo hmF
        (Nat.zero_le _) (M := walkIdx sk G)
        (fun hc => absurd (walkIdx_inj hr2 hGr hc) (by omega))
        (fun h' hle hc =>
          absurd (walkIdx_inj (by omega) hGr hc) (by omega))
      rw [Nat.sub_zero] at hkn
      exact hkn
    have hancU : AncTele sk 0 A j
        (fun G => if G = 0 + 1 then 0 else t G)
        ((((upperOut (wpk 0), true, k) : Ev)
            :: (List.range' 0
                (sk.nChildren 0 (sk.stageScope 0 k))).flatMap
              (fun i => opEvents sk (.kid 0 k (sk.stageScope 0 k)
                (lastDOf sk 0 k) (sk.wiresBefore 0 k) i
                (scopeFeed sk 0 k)))) ++ rest) := by
      refine ancTele_rebase sk hanc hprenil ?_
      intro _
      refine hpar _ 0 ?_
      rw [List.drop_zero,
        List.filter_cons_of_neg (by
          simp only [evOwner_upperOut, beq_iff_eq]
          intro hc
          exact absurd (walkIdx_inj hr0 hr2 hc) (by omega))]
      exact hks2
    have hown : (((upperOut (wpk 0), true, k) : Ev)
          :: ((List.range' 0
                (sk.nChildren 0 (sk.stageScope 0 k))).flatMap
              (fun i => opEvents sk (.kid 0 k (sk.stageScope 0 k)
                (lastDOf sk 0 k) (sk.wiresBefore 0 k) i
                (scopeFeed sk 0 k)))
            ++ rest)).filter (fun e => evOwner sk e == walkIdx sk 0)
        = ((upperOut (wpk 0), true, k) : Ev)
            :: ((List.range' 0
                  (sk.nChildren 0 (sk.stageScope 0 k))).flatMap
                  (splicedChunk sk 0 k (lastDOf sk 0 k))
                ++ walkSeg sk 0 (k + 1) (sk.stageLen 0)) := by
      rw [List.filter_cons_of_pos (by
          simp only [evOwner_upperOut, beq_self_eq_true]),
        List.filter_append, hks3, hlow0]
    have hfeedU : ∃ i₀,
        (((upperOut (wpk 0), true, k) : Ev)
            :: ((List.range' 0
                  (sk.nChildren 0 (sk.stageScope 0 k))).flatMap
                (fun i => opEvents sk (.kid 0 k (sk.stageScope 0 k)
                  (lastDOf sk 0 k) (sk.wiresBefore 0 k) i
                  (scopeFeed sk 0 k)))
              ++ rest)).filter (fun e => evOwner sk e == 1)
          = ((ropenEvents sk).drop 3).drop i₀ := by
      have h1 := hone (((upperOut (wpk 0), true, k) : Ev)
          :: (List.range' 0
              (sk.nChildren 0 (sk.stageScope 0 k))).flatMap
            (fun i => opEvents sk (.kid 0 k (sk.stageScope 0 k)
              (lastDOf sk 0 k) (sk.wiresBefore 0 k) i
              (scopeFeed sk 0 k)))) ?_
      · obtain ⟨i₀, hf⟩ := h1
        exact ⟨i₀, hf⟩
      · rw [List.filter_cons_of_neg (by
            simp only [evOwner_upperOut, beq_iff_eq]
            have := hw2 0 hr0
            omega)]
        have hkn := kids_filter_ne sk hwf hr0 hk hF hFo hmF
          (Nat.zero_le _) (M := 1)
          (by have := hw2 1 hr2; omega)
          (fun h' hle => by have := hw2 h' (by omega); omega)
        rw [Nat.sub_zero] at hkn
        exact hkn
    exact ready_upper_prologue sk hwf hsched hr0 hk hLn hancU
      (fun _ => hcoh0) hfeedU
      (fun g' hg' => absurd hg' (Nat.not_lt_zero g'))
      hown st hW hfix
  · -- the leaf slots, folded
    have hfold : ∀ (m i : Nat),
        i + m = sk.nChildren 0 (sk.stageScope 0 k) →
        EmitOKOn sk
          ((List.range' i m).flatMap fun i' =>
            opEvents sk (.kid 0 k (sk.stageScope 0 k)
              (lastDOf sk 0 k) (sk.wiresBefore 0 k) i'
              (scopeFeed sk 0 k)))
          rest := by
      intro m
      induction m with
      | zero =>
          intro i _
          exact emitOKOn_nil sk rest
      | succ m ihm =>
          intro i hin
          have hi : i < sk.nChildren 0 (sk.stageScope 0 k) := by
            omega
          obtain ⟨-, hksI2, hksI3, -⟩ :=
            align_kids_suffix sk hwf hr0 hk hF hFo hmF
              (i := i + 1) (by omega)
          -- the slot's two events
          have hkidE : opEvents sk (.kid 0 k (sk.stageScope 0 k)
                (lastDOf sk 0 k) (sk.wiresBefore 0 k) i
                (scopeFeed sk 0 k))
              = ((wireOut (wpk 0), true,
                    sk.wiresBefore 0 k + i) : Ev)
                :: (scopeFeed sk 0 k)[i]?.toList := by
            rw [opEvents_kid_eq sk 0 k (lastDOf sk 0 k)
                (sk.wiresBefore 0 k) i (scopeFeed sk 0 k),
              if_neg (by
                rw [show sk.childIsD 0 (sk.stageScope 0 k) i = false
                    from rfl]
                exact Bool.false_ne_true),
              if_pos (show ((0 : Nat) == 0) = true from rfl),
              List.append_nil]
          have hqel := scopeFeed_getElem? sk (h := 0) (k := k) hi
          rw [show (0 : Nat) + 1 = 1 from rfl] at hqel
          have hdropi : (scopeFeed sk 0 k).drop i
              = ((askedOut (wpk 1), true,
                    sk.wiresBefore 0 k + i) : Ev)
                :: (scopeFeed sk 0 k).drop (i + 1) := by
            have hm := toList_drop_merge
              (l := scopeFeed sk 0 k) (i := i) (by rw [hF]; exact hi)
            rw [hqel, Option.toList_some, List.singleton_append] at hm
            exact hm.symm
          have hpeel : (List.range' i
                (sk.nChildren 0 (sk.stageScope 0 k) - i)).flatMap
                (splicedChunk sk 0 k (lastDOf sk 0 k))
              = ((wireOut (wpk 0), true,
                    sk.wiresBefore 0 k + i) : Ev)
                :: (List.range' (i + 1)
                    (sk.nChildren 0 (sk.stageScope 0 k)
                      - (i + 1))).flatMap
                  (splicedChunk sk 0 k (lastDOf sk 0 k)) := by
            rw [show sk.nChildren 0 (sk.stageScope 0 k) - i
                = (sk.nChildren 0 (sk.stageScope 0 k) - (i + 1)) + 1
                from by omega, List.range'_succ, List.flatMap_cons]
            rfl
          -- foreign-owner silence of the later slots
          have hknG : ∀ G, 2 ≤ G → G < sk.rootH →
              ((List.range' (i + 1)
                  (sk.nChildren 0 (sk.stageScope 0 k)
                    - (i + 1))).flatMap
                (fun i' => opEvents sk (.kid 0 k (sk.stageScope 0 k)
                  (lastDOf sk 0 k) (sk.wiresBefore 0 k) i'
                  (scopeFeed sk 0 k)))).filter
                (fun e => evOwner sk e == walkIdx sk G) = [] := by
            intro G hG2 hGr
            exact kids_filter_ne sk hwf hr0 hk hF hFo hmF
              (i := i + 1) (by omega) (M := walkIdx sk G)
              (fun hc => absurd (walkIdx_inj hr2 hGr hc) (by omega))
              (fun h' hle hc =>
                absurd (walkIdx_inj (by omega) hGr hc) (by omega))
          have hkn1 : ((List.range' (i + 1)
              (sk.nChildren 0 (sk.stageScope 0 k)
                - (i + 1))).flatMap
              (fun i' => opEvents sk (.kid 0 k (sk.stageScope 0 k)
                (lastDOf sk 0 k) (sk.wiresBefore 0 k) i'
                (scopeFeed sk 0 k)))).filter
              (fun e => evOwner sk e == 1) = [] :=
            kids_filter_ne sk hwf hr0 hk hF hFo hmF
              (i := i + 1) (by omega) (M := 1)
              (by have := hw2 1 hr2; omega)
              (fun h' hle => by have := hw2 h' (by omega); omega)
          rw [List.range'_succ, List.flatMap_cons, hkidE, hqel,
            Option.toList_some,
            show m = sk.nChildren 0 (sk.stageScope 0 k) - (i + 1)
              from by omega]
          refine emitOKOn_append sk ?_ (by
            have hih := ihm (i + 1) (by omega)
            rwa [show m = sk.nChildren 0 (sk.stageScope 0 k) - (i + 1)
              from by omega] at hih)
          generalize hLgen : (List.range' (i + 1)
              (sk.nChildren 0 (sk.stageScope 0 k)
                - (i + 1))).flatMap
              (fun i' => opEvents sk (.kid 0 k (sk.stageScope 0 k)
                (lastDOf sk 0 k) (sk.wiresBefore 0 k) i'
                (scopeFeed sk 0 k))) = L
            at hksI2 hksI3 hknG hkn1 ⊢
          refine emitOKOn_cons sk ?_ ?_
          · -- W0: the slot's wire through the absorber's wire window
            intro st hW hfix hpred
            have hpreW : (((wireOut (wpk 0), true,
                  sk.wiresBefore 0 k + i) : Ev)
                :: ([((askedOut (wpk 1), true,
                      sk.wiresBefore 0 k + i) : Ev)] ++ L)).filter
                (fun e => evOwner sk e == walkIdx sk 1)
                = (scopeFeed sk 0 k).drop i := by
              rw [List.filter_cons_of_neg (by
                  simp only [evOwner_wireOut sk hr0, beq_iff_eq]
                  intro hc
                  exact absurd (walkIdx_inj hr0 hr2 hc) (by omega)),
                List.filter_append,
                List.filter_cons_of_pos (by
                  simp only [evOwner_askedOut sk (Nat.le_refl 1) hr2,
                    beq_self_eq_true]),
                List.filter_nil, List.singleton_append, hksI2,
                ← hdropi]
            refine ready_wire0 sk hwf hsched hr0 hk hi (A := A)
              (j := j) (t := fun G => if G = 0 + 1 then i else t G)
              ?_ hcoh0 rfl ?_ ?_ st hW hfix
            · refine ancTele_rebase sk
                (pre := ((wireOut (wpk 0), true,
                    sk.wiresBefore 0 k + i) : Ev)
                  :: ([((askedOut (wpk 1), true,
                        sk.wiresBefore 0 k + i) : Ev)] ++ L))
                hanc ?_ ?_
              · intro G hG2 hGr
                rw [List.filter_cons_of_neg (by
                    simp only [evOwner_wireOut sk hr0, beq_iff_eq]
                    intro hc
                    exact absurd (walkIdx_inj hr0 hGr hc) (by omega)),
                  List.filter_append,
                  List.filter_cons_of_neg (by
                    simp only [evOwner_askedOut sk (Nat.le_refl 1) hr2,
                      beq_iff_eq]
                    intro hc
                    exact absurd (walkIdx_inj hr2 hGr hc) (by omega)),
                  List.filter_nil, List.nil_append]
                exact hknG G hG2 hGr
              · intro _
                exact hpar _ i hpreW
            · refine hone (((wireOut (wpk 0), true,
                  sk.wiresBefore 0 k + i) : Ev)
                :: ([((askedOut (wpk 1), true,
                      sk.wiresBefore 0 k + i) : Ev)] ++ L)) ?_
              rw [List.filter_cons_of_neg (by
                  simp only [evOwner_wireOut sk hr0, beq_iff_eq]
                  have := hw2 0 hr0
                  omega),
                List.filter_append,
                List.filter_cons_of_neg (by
                  simp only [evOwner_askedOut sk (Nat.le_refl 1) hr2,
                    beq_iff_eq]
                  have := hw2 1 hr2
                  omega),
                List.filter_nil, List.nil_append]
              exact hkn1
            · rw [List.filter_cons_of_pos (by
                  simp only [evOwner_wireOut sk hr0, beq_self_eq_true]),
                List.filter_append,
                List.filter_cons_of_neg (by
                  simp only [evOwner_askedOut sk (Nat.le_refl 1) hr2,
                    beq_iff_eq]
                  intro hc
                  exact absurd (walkIdx_inj hr2 hr0 hc) (by omega)),
                List.filter_nil, List.nil_append, List.filter_append,
                hksI3, hlow0, hpeel, List.cons_append]
          · -- Q0: the slot's feed query through the request window
            refine emitOKOn_cons sk ?_ (emitOKOn_nil sk _)
            intro st hW hfix hpred
            rw [List.nil_append] at hW
            have hpreQ : (((askedOut (wpk 1), true,
                  sk.wiresBefore 0 k + i) : Ev) :: L).filter
                (fun e => evOwner sk e == walkIdx sk 1)
                = (scopeFeed sk 0 k).drop i := by
              rw [List.filter_cons_of_pos (by
                  simp only [evOwner_askedOut sk (Nat.le_refl 1) hr2,
                    beq_self_eq_true]),
                hksI2, ← hdropi]
            refine ready_leafreq sk hwf hsched hr0 hk hi (A := A)
              (j := j) (t := fun G => if G = 0 + 1 then i else t G)
              ?_ hcoh0 rfl ?_ ?_ st hW hfix
            · refine ancTele_rebase sk
                (pre := ((askedOut (wpk 1), true,
                    sk.wiresBefore 0 k + i) : Ev) :: L) hanc ?_ ?_
              · intro G hG2 hGr
                rw [List.filter_cons_of_neg (by
                    simp only [evOwner_askedOut sk (Nat.le_refl 1) hr2,
                      beq_iff_eq]
                    intro hc
                    exact absurd (walkIdx_inj hr2 hGr hc) (by omega))]
                exact hknG G hG2 hGr
              · intro _
                exact hpar _ i hpreQ
            · refine hone (((askedOut (wpk 1), true,
                  sk.wiresBefore 0 k + i) : Ev) :: L) ?_
              rw [List.filter_cons_of_neg (by
                  simp only [evOwner_askedOut sk (Nat.le_refl 1) hr2,
                    beq_iff_eq]
                  have := hw2 1 hr2
                  omega)]
              exact hkn1
            · rw [List.filter_cons_of_neg (by
                  simp only [evOwner_askedOut sk (Nat.le_refl 1) hr2,
                    beq_iff_eq]
                  intro hc
                  exact absurd (walkIdx_inj hr2 hr0 hc) (by omega)),
                List.filter_append, hksI3, hlow0]
    have hgoal := hfold (sk.nChildren 0 (sk.stageScope 0 k)) 0
      (by omega)
    exact hgoal

-- ============================================ the interior-stage fold

/-- The kids-fold of the master induction at an interior stage:
every emission of kid slots `i, i+1, …` of one scope is ready.

The wires and feed queries discharge from their manual predecessors,
the resolutions and splice summaries through the windows, and each
kid's subtree through the stage-below induction hypothesis `IH` with
the pushed rolling context: the ancestor telescope gains the current
scope's coordinates, the parent feed cursor advances past the
consumed query, and the deep windows re-anchor at the kid cursor. -/
private theorem emitOK_kids (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) {hp : Nat}
    (hh : hp + 1 < sk.rootH) {k : Nat}
    (hk : k < sk.stageLen (hp + 1)) {rest : List Ev}
    {A j t : Nat → Nat} {mF : Nat}
    (hFo : ∀ e ∈ scopeFeed sk (hp + 1) k, evOwner sk e = mF)
    (hmF : mF < walkIdx sk (hp + 1))
    (hmFeq : hp + 1 + 1 < sk.rootH → mF = walkIdx sk (hp + 1 + 1))
    (hlow : ∀ g', g' ≤ hp + 1 →
      rest.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSeg sk g' (descIdx sk g' (hp + 1 - g') (k + 1))
            (sk.stageLen g'))
    (hanc : AncTele sk (hp + 1) A j t rest)
    (hcoh0 : hp + 1 + 1 < sk.rootH →
      k = sk.wiresBefore (hp + 1 + 1) (A (hp + 1 + 1))
          + j (hp + 1 + 1))
    (hsat : hp + 1 + 1 < sk.rootH →
      (chunkQ sk (hp + 1 + 1) (A (hp + 1 + 1)) (j (hp + 1 + 1))).drop
        (t (hp + 1 + 1)) = [])
    (hfd : ∀ (pre : List Ev) (c : Nat),
      pre.filter (fun e => evOwner sk e == mF)
        = (scopeFeed sk (hp + 1) k).drop c →
      (∀ M, (∀ h', h' ≤ hp + 1 → walkIdx sk h' ≠ M) → mF ≠ M →
        pre.filter (fun e => evOwner sk e == M) = []) →
      ∃ i₀, (pre ++ rest).filter (fun e => evOwner sk e == 1)
        = ((ropenEvents sk).drop 3).drop i₀)
    (IH : ∀ (k' : Nat) (rest' : List Ev) (A' j' t' : Nat → Nat),
      k' < sk.stageLen hp →
      (∀ g', g' ≤ hp →
        rest'.filter (fun e => evOwner sk e == walkIdx sk g')
          = walkSeg sk g' (descIdx sk g' (hp - g') (k' + 1))
              (sk.stageLen g')) →
      AncTele sk hp A' j' t' rest' →
      (hp + 1 < sk.rootH →
        k' = sk.wiresBefore (hp + 1) (A' (hp + 1)) + j' (hp + 1)) →
      (hp + 1 < sk.rootH →
        (chunkQ sk (hp + 1) (A' (hp + 1)) (j' (hp + 1))).drop
          (t' (hp + 1)) = []) →
      (∀ (pre : List Ev) (c : Nat),
        pre.filter (fun e => evOwner sk e == walkIdx sk (hp + 1))
          = (scopeFeed sk hp k').drop c →
        (∀ M, (∀ h', h' ≤ hp → walkIdx sk h' ≠ M) →
          walkIdx sk (hp + 1) ≠ M →
          pre.filter (fun e => evOwner sk e == M) = []) →
        ∃ i₀, (pre ++ rest').filter (fun e => evOwner sk e == 1)
          = ((ropenEvents sk).drop 3).drop i₀) →
      EmitOKOn sk (opEvents sk (.scope hp k' (scopeFeed sk hp k')))
        rest') :
    ∀ (m i : Nat),
      i + m = sk.nChildren (hp + 1) (sk.stageScope (hp + 1) k) →
      EmitOKOn sk
        ((List.range' i m).flatMap fun i' =>
          opEvents sk (.kid (hp + 1) k (sk.stageScope (hp + 1) k)
            (lastDOf sk (hp + 1) k) (sk.wiresBefore (hp + 1) k) i'
            (scopeFeed sk (hp + 1) k)))
        rest := by
  intro m
  induction m with
  | zero =>
      intro i _
      exact emitOKOn_nil sk rest
  | succ m ihm =>
      intro i hin
      have hi : i < sk.nChildren (hp + 1) (sk.stageScope (hp + 1) k) :=
        by omega
      have hkid : sk.wiresBefore (hp + 1) k + i < sk.stageLen hp :=
        kid_index_lt sk hwf (by omega) hh hk hi
      have hw2 : ∀ h', 2 ≤ walkIdx sk h' := by
        intro h'
        unfold walkIdx
        omega
      -- the feed read at slot i
      have hqel := scopeFeed_getElem? sk (h := hp + 1) (k := k) hi
      have hfQmem : ((askedOut (wpk (hp + 1 + 1)), true,
          sk.wiresBefore (hp + 1) k + i) : Ev)
          ∈ scopeFeed sk (hp + 1) k :=
        List.mem_of_getElem? hqel
      have hfQo : evOwner sk ((askedOut (wpk (hp + 1 + 1)), true,
          sk.wiresBefore (hp + 1) k + i) : Ev) = mF :=
        hFo _ hfQmem
      have hdropi : (scopeFeed sk (hp + 1) k).drop i
          = ((askedOut (wpk (hp + 1 + 1)), true,
              sk.wiresBefore (hp + 1) k + i) : Ev)
            :: (scopeFeed sk (hp + 1) k).drop (i + 1) := by
        have hm := toList_drop_merge (l := scopeFeed sk (hp + 1) k)
          (i := i) (by rw [scopeFeed_length]; exact hi)
        rw [hqel, Option.toList_some, List.singleton_append] at hm
        exact hm.symm
      -- the subtree's feed and alignment
      have hFeq : chunkQ sk (hp + 1) k i
          = scopeFeed sk hp (sk.wiresBefore (hp + 1) k + i) :=
        chunkQ_eq_feed sk hwf hh hk hi
      have hF' : (scopeFeed sk hp
            (sk.wiresBefore (hp + 1) k + i)).length
          = sk.nChildren hp
              (sk.stageScope hp (sk.wiresBefore (hp + 1) k + i)) :=
        scopeFeed_length sk hp _
      have hFo' : ∀ e ∈ scopeFeed sk hp
            (sk.wiresBefore (hp + 1) k + i),
          evOwner sk e = walkIdx sk (hp + 1) := by
        intro e he
        unfold scopeFeed seg at he
        obtain ⟨j', -, rfl⟩ := List.mem_map.1 he
        exact evOwner_askedOut sk (by omega) hh _
      have hmF' : walkIdx sk (hp + 1) < walkIdx sk hp :=
        walkIdx_lt sk (by omega) hh
      obtain ⟨-, hsc2, hsc3⟩ := align_scope sk hwf hp
        (sk.wiresBefore (hp + 1) k + i)
        (scopeFeed sk hp (sk.wiresBefore (hp + 1) k + i))
        (walkIdx sk (hp + 1)) (by omega) hkid hF' hFo' hmF'
      have hscnil : ∀ M, walkIdx sk (hp + 1) ≠ M →
          (∀ h', h' ≤ hp → walkIdx sk h' ≠ M) →
          (opEvents sk (.scope hp (sk.wiresBefore (hp + 1) k + i)
              (scopeFeed sk hp
                (sk.wiresBefore (hp + 1) k + i)))).filter
            (fun e => evOwner sk e == M) = [] :=
        fun M h1 h2 => scope_filter_ne sk hwf (by omega) hkid hF' hFo'
          hmF' h1 h2
      -- expand the current slot in the goal
      rw [List.range'_succ, List.flatMap_cons,
        show m = sk.nChildren (hp + 1) (sk.stageScope (hp + 1) k)
          - (i + 1) from by omega]
      -- the kid-suffix clauses at i + 1
      obtain ⟨-, hks2, hks3, hks4⟩ := align_kids_suffix sk hwf hh hk
        (scopeFeed_length sk (hp + 1) k) hFo hmF (i := i + 1)
        (by omega)
      have hksU : ∀ M, (∀ h', h' ≤ hp + 1 → walkIdx sk h' ≠ M) →
          mF ≠ M →
          ((List.range' (i + 1)
              (sk.nChildren (hp + 1) (sk.stageScope (hp + 1) k)
                - (i + 1))).flatMap
            (fun i' => opEvents sk (.kid (hp + 1) k
              (sk.stageScope (hp + 1) k) (lastDOf sk (hp + 1) k)
              (sk.wiresBefore (hp + 1) k) i'
              (scopeFeed sk (hp + 1) k)))).filter
            (fun e => evOwner sk e == M) = [] :=
        fun M h1 h2 => kids_filter_ne sk hwf hh hk
          (scopeFeed_length sk (hp + 1) k) hFo hmF (by omega) h2 h1
      generalize hLgen : (List.range' (i + 1)
          (sk.nChildren (hp + 1) (sk.stageScope (hp + 1) k)
            - (i + 1))).flatMap
          (fun i' => opEvents sk (.kid (hp + 1) k
            (sk.stageScope (hp + 1) k) (lastDOf sk (hp + 1) k)
            (sk.wiresBefore (hp + 1) k) i'
            (scopeFeed sk (hp + 1) k))) = L
        at hks2 hks3 hks4 hksU ⊢
      -- the deep windows over the tail
      have hglue := deep_glue sk hwf (h := hp + 1) hh hk (i := i + 1)
        (by omega) hks4 hlow
      -- the tail of the fold, converted
      have hih : EmitOKOn sk L rest := by
        have h0 := ihm (i + 1) (by omega)
        rwa [show m = sk.nChildren (hp + 1)
            (sk.stageScope (hp + 1) k) - (i + 1) from by omega,
          hLgen] at h0
      -- foreign-owner silence of the tail
      have hnilU : ∀ G, hp + 1 + 2 ≤ G → G < sk.rootH →
          L.filter (fun e => evOwner sk e == walkIdx sk G) = [] := by
        intro G hG2 hGr
        refine hksU (walkIdx sk G)
          (fun h' hle hc =>
            absurd (walkIdx_inj (by omega) hGr hc) (by omega)) ?_
        rw [hmFeq (by omega)]
        exact fun hc => absurd (walkIdx_inj (by omega) hGr hc)
          (by omega)
      -- the parent chunk is the feed
      have hcq : hp + 1 + 1 < sk.rootH →
          chunkQ sk (hp + 1 + 1) (A (hp + 1 + 1)) (j (hp + 1 + 1))
            = scopeFeed sk (hp + 1) k := by
        intro hGr
        obtain ⟨hA2, hj2⟩ := hanc.rng (hp + 1 + 1) (by omega) hGr
        rw [chunkQ_eq_feed sk hwf (hp := hp + 1) hGr hA2 hj2,
          ← hcoh0 hGr]
      -- the tail telescope: parent cursor at i + 1
      have hparlat : hp + 1 + 1 < sk.rootH →
          (L ++ rest).filter
              (fun e => evOwner sk e == walkIdx sk (hp + 1 + 1))
            = (chunkQ sk (hp + 1 + 1) (A (hp + 1 + 1))
                  (j (hp + 1 + 1))).drop (i + 1)
              ++ (List.range' (j (hp + 1 + 1) + 1)
                    (sk.nChildren (hp + 1 + 1)
                        (sk.stageScope (hp + 1 + 1) (A (hp + 1 + 1)))
                      - (j (hp + 1 + 1) + 1))).flatMap
                   (splicedChunk sk (hp + 1 + 1) (A (hp + 1 + 1))
                     (lastDOf sk (hp + 1 + 1) (A (hp + 1 + 1))))
              ++ walkSeg sk (hp + 1 + 1) (A (hp + 1 + 1) + 1)
                  (sk.stageLen (hp + 1 + 1)) := by
        intro hGr
        have hks2' : L.filter
            (fun e => evOwner sk e == walkIdx sk (hp + 1 + 1))
            = (scopeFeed sk (hp + 1) k).drop (i + 1) := by
          rw [← hmFeq hGr]
          exact hks2
        rw [List.filter_append, hks2',
          hanc.fil (hp + 1 + 1) (by omega) hGr, hsat hGr,
          List.nil_append, hcq hGr, ← List.append_assoc]
      have htele_lat : AncTele sk (hp + 1) A j
          (fun G => if G = hp + 1 + 1 then i + 1 else t G)
          (L ++ rest) :=
        ancTele_rebase sk hanc hnilU hparlat
      -- the pushed subtree telescope
      have hqcnt : (chunkQ sk (hp + 1) k i).length
          = sk.qCount (hp + 1) (sk.stageScope (hp + 1) k) i :=
        chunkQ_length sk (hp + 1) k i
      have hlow1 : rest.filter
            (fun e => evOwner sk e == walkIdx sk (hp + 1))
          = walkSeg sk (hp + 1) (k + 1) (sk.stageLen (hp + 1)) := by
        have hl := hlow (hp + 1) (Nat.le_refl _)
        rw [Nat.sub_self, descIdx_zero] at hl
        exact hl
      have htele_sub : AncTele sk hp
          (fun G => if G = hp + 1 then k else A G)
          (fun G => if G = hp + 1 then i else j G)
          (fun G => if G = hp + 1
            then sk.qCount (hp + 1) (sk.stageScope (hp + 1) k) i
            else if G = hp + 1 + 1 then i + 1 else t G)
          (L ++ rest) := by
        refine ⟨?_, ?_, ?_, ?_⟩
        · intro G hG hGr
          by_cases hG1 : G = hp + 1
          · subst hG1
            simp only [reduceIte]
            exact ⟨hk, hi⟩
          · simp only [if_neg hG1]
            exact hanc.rng G (by omega) hGr
        · intro G hG2 hGr
          by_cases hG1 : G = hp + 1 + 1
          · subst hG1
            simp only [if_neg (show ¬(hp + 1 + 1 = hp + 1) from
              by omega)]
            obtain ⟨hA2, hj2⟩ := hanc.rng (hp + 1 + 1) (by omega) hGr
            exact parent_slot_isD sk hwf hGr hk hA2 hj2 (hcoh0 hGr)
              (by omega)
          · simp only [if_neg (show ¬(G = hp + 1) from by omega)]
            exact hanc.isD G (by omega) hGr
        · intro G hG1 hGr1
          by_cases hG : G = hp + 1
          · subst hG
            simp only [reduceIte,
              if_neg (show ¬(hp + 1 + 1 = hp + 1) from by omega)]
            exact hcoh0 hGr1
          · simp only [if_neg hG,
              if_neg (show ¬(G + 1 = hp + 1) from by omega)]
            exact hanc.coh G (by omega) hGr1
        · intro G hG hGr
          by_cases hG1 : G = hp + 1
          · subst hG1
            simp only [reduceIte]
            rw [List.filter_append, hks3, hlow1,
              show (chunkQ sk (hp + 1) k i).drop
                  (sk.qCount (hp + 1) (sk.stageScope (hp + 1) k) i)
                  = [] from by rw [← hqcnt]; exact List.drop_length,
              List.nil_append]
          · by_cases hG2 : G = hp + 1 + 1
            · subst hG2
              simp only [if_neg (show ¬(hp + 1 + 1 = hp + 1) from
                by omega), reduceIte]
              have hf := htele_lat.fil (hp + 1 + 1) (by omega) hGr
              simp only [reduceIte] at hf
              exact hf
            · simp only [if_neg hG1, if_neg hG2]
              have hf := htele_lat.fil G (by omega) hGr
              rw [if_neg hG2] at hf
              exact hf
      -- the pushed low windows
      have hlowsub : ∀ g', g' ≤ hp →
          (L ++ rest).filter
              (fun e => evOwner sk e == walkIdx sk g')
            = walkSeg sk g'
                (descIdx sk g' (hp - g')
                  (sk.wiresBefore (hp + 1) k + i + 1))
                (sk.stageLen g') :=
        fun g' hg' => hglue g' (by omega)
      -- the pushed owner-1 clause
      have hfdsub : ∀ (pre : List Ev) (c : Nat),
          pre.filter (fun e => evOwner sk e == walkIdx sk (hp + 1))
            = (scopeFeed sk hp
                (sk.wiresBefore (hp + 1) k + i)).drop c →
          (∀ M, (∀ h', h' ≤ hp → walkIdx sk h' ≠ M) →
            walkIdx sk (hp + 1) ≠ M →
            pre.filter (fun e => evOwner sk e == M) = []) →
          ∃ i₀, (pre ++ (L ++ rest)).filter
              (fun e => evOwner sk e == 1)
            = ((ropenEvents sk).drop 3).drop i₀ := by
        intro pre c hpre hprU
        obtain ⟨i₀, hlr⟩ := hfd L (i + 1) hks2 hksU
        refine ⟨i₀, ?_⟩
        rw [List.filter_append,
          hprU 1 (fun h' _ => by have := hw2 h'; omega)
            (by have := hw2 (hp + 1); omega),
          List.nil_append]
        exact hlr
      -- the subtree, ready
      have hsubOK : EmitOKOn sk
          (opEvents sk (.scope hp (sk.wiresBefore (hp + 1) k + i)
            (scopeFeed sk hp (sk.wiresBefore (hp + 1) k + i))))
          (L ++ rest) := by
        refine IH (sk.wiresBefore (hp + 1) k + i) _ _ _ _ hkid hlowsub
          htele_sub ?_ ?_ hfdsub
        · intro _
          simp only [reduceIte]
        · intro _
          simp only [reduceIte]
          rw [← hqcnt]
          exact List.drop_length
      -- the slot's event list
      have hkidE := opEvents_kid_eq sk (hp + 1) k
        (lastDOf sk (hp + 1) k) (sk.wiresBefore (hp + 1) k) i
        (scopeFeed sk (hp + 1) k)
      rw [hqel, Option.toList_some,
        show hp + 1 - 1 = hp from rfl, hFeq] at hkidE
      -- shared head facts
      have hOg : ∀ g', g' < hp + 1 → mF < walkIdx sk g' := by
        intro g' hg'
        have := walkIdx_lt sk (show g' < hp + 1 from hg') hh
        omega
      by_cases hD : sk.childIsD (hp + 1) (sk.stageScope (hp + 1) k) i
          = true
      · -- a disputed slot: wire, resolution, splice?, query, subtree
        rw [if_pos hD] at hkidE
        by_cases hsp : lastDOf sk (hp + 1) k = some i
        · -- the spliced slot: the parent summary follows the last
          -- resolution
          have hspb : (lastDOf sk (hp + 1) k == some i) = true := by
            rw [hsp]
            exact beq_self_eq_true _
          rw [if_pos hspb] at hkidE
          simp only [List.cons_append, List.nil_append] at hkidE
          rw [hkidE]
          simp only [List.cons_append]
          -- W: the slot's wire, manual
          refine emitOKOn_cons sk (fun st hW hfix hpred =>
            head_snd_wire sk hwf hW.toWCount (show 1 ≤ hp + 1 by omega) hpred) ?_
          -- L: the resolution through the lower window
          refine emitOKOn_cons sk ?_ ?_
          · intro st hW hfix _
            rw [List.cons_append, List.cons_append] at hW
            refine ready_lower sk hwf hsched
              (t := fun G => if G = hp + 1 + 1 then i else t G)
              hh hk hi hD ?_ hcoh0 ?_ ?_ ?_ st hW hfix
            · -- the telescope at this site
              refine ancTele_rebase sk
                (pre := ((lowerOut (wpk (hp + 1)), true,
                      sk.dsBefore (hp + 1) k
                        + dRank sk (wpk (hp + 1)) k i) : Ev)
                  :: ((upperOut (wpk (hp + 1)), true, k) : Ev)
                  :: ((askedOut (wpk (hp + 1 + 1)), true,
                        sk.wiresBefore (hp + 1) k + i) : Ev)
                  :: (opEvents sk (.scope hp
                        (sk.wiresBefore (hp + 1) k + i)
                        (scopeFeed sk hp
                          (sk.wiresBefore (hp + 1) k + i))) ++ L))
                hanc ?_ ?_
              · intro G hG2 hGr
                rw [List.filter_cons_of_neg (by
                    simp only [evOwner_lowerOut, beq_iff_eq]
                    exact fun hc =>
                      absurd (walkIdx_inj hh hGr hc) (by omega)),
                  List.filter_cons_of_neg (by
                    simp only [evOwner_upperOut, beq_iff_eq]
                    exact fun hc =>
                      absurd (walkIdx_inj hh hGr hc) (by omega)),
                  List.filter_cons_of_neg (by
                    simp only [hfQo, beq_iff_eq]
                    rw [hmFeq (by omega)]
                    exact fun hc =>
                      absurd (walkIdx_inj (by omega) hGr hc)
                        (by omega)),
                  List.filter_append,
                  hscnil (walkIdx sk G)
                    (fun hc =>
                      absurd (walkIdx_inj hh hGr hc) (by omega))
                    (fun h' hle hc =>
                      absurd (walkIdx_inj (by omega) hGr hc)
                        (by omega)),
                  List.nil_append,
                  hnilU G hG2 hGr]
              · intro hGr
                rw [List.cons_append, List.cons_append,
                  List.cons_append,
                  List.filter_cons_of_neg (by
                    simp only [evOwner_lowerOut, beq_iff_eq]
                    exact fun hc =>
                      absurd (walkIdx_inj hh (by omega) hc)
                        (by omega)),
                  List.filter_cons_of_neg (by
                    simp only [evOwner_upperOut, beq_iff_eq]
                    exact fun hc =>
                      absurd (walkIdx_inj hh (by omega) hc)
                        (by omega)),
                  List.filter_cons_of_pos (by
                    simp only [hfQo, hmFeq hGr, beq_self_eq_true]),
                  List.append_assoc, List.filter_append,
                  hscnil (walkIdx sk (hp + 1 + 1))
                    (fun hc =>
                      absurd (walkIdx_inj hh (by omega) hc)
                        (by omega))
                    (fun h' hle hc =>
                      absurd (walkIdx_inj (by omega) (by omega) hc)
                        (by omega)),
                  List.nil_append, hparlat hGr, hcq hGr, hdropi]
                simp only [List.cons_append]
            · -- the openers' share
              have hpm : (((lowerOut (wpk (hp + 1)), true,
                      sk.dsBefore (hp + 1) k
                        + dRank sk (wpk (hp + 1)) k i) : Ev)
                    :: ((upperOut (wpk (hp + 1)), true, k) : Ev)
                    :: ((askedOut (wpk (hp + 1 + 1)), true,
                          sk.wiresBefore (hp + 1) k + i) : Ev)
                    :: (opEvents sk (.scope hp
                          (sk.wiresBefore (hp + 1) k + i)
                          (scopeFeed sk hp
                            (sk.wiresBefore (hp + 1) k + i)))
                        ++ L)).filter
                  (fun e => evOwner sk e == mF)
                  = (scopeFeed sk (hp + 1) k).drop i := by
                rw [List.filter_cons_of_neg (by
                    simp only [evOwner_lowerOut, beq_iff_eq]
                    omega),
                  List.filter_cons_of_neg (by
                    simp only [evOwner_upperOut, beq_iff_eq]
                    omega),
                  List.filter_cons_of_pos (by
                    simp only [hfQo, beq_self_eq_true]),
                  List.filter_append,
                  hscnil mF (by omega)
                    (fun h' hle => by
                      have := hOg h' (by omega)
                      omega),
                  List.nil_append, hks2, ← hdropi]
              have hpU : ∀ M, (∀ h', h' ≤ hp + 1 →
                    walkIdx sk h' ≠ M) → mF ≠ M →
                  (((lowerOut (wpk (hp + 1)), true,
                        sk.dsBefore (hp + 1) k
                          + dRank sk (wpk (hp + 1)) k i) : Ev)
                    :: ((upperOut (wpk (hp + 1)), true, k) : Ev)
                    :: ((askedOut (wpk (hp + 1 + 1)), true,
                          sk.wiresBefore (hp + 1) k + i) : Ev)
                    :: (opEvents sk (.scope hp
                          (sk.wiresBefore (hp + 1) k + i)
                          (scopeFeed sk hp
                            (sk.wiresBefore (hp + 1) k + i)))
                        ++ L)).filter
                    (fun e => evOwner sk e == M) = [] := by
                intro M hM1 hM2
                rw [List.filter_cons_of_neg (by
                    simp only [evOwner_lowerOut, beq_iff_eq]
                    exact hM1 (hp + 1) (Nat.le_refl _)),
                  List.filter_cons_of_neg (by
                    simp only [evOwner_upperOut, beq_iff_eq]
                    exact hM1 (hp + 1) (Nat.le_refl _)),
                  List.filter_cons_of_neg (by
                    simp only [hfQo, beq_iff_eq]
                    exact hM2),
                  List.filter_append,
                  hscnil M (hM1 (hp + 1) (Nat.le_refl _))
                    (fun h' hle => hM1 h' (by omega)),
                  List.nil_append, hksU M hM1 hM2]
              obtain ⟨i₀, hf⟩ := hfd _ i hpm hpU
              exact ⟨i₀, hf⟩
            · -- the deep windows
              intro g' hg'
              rw [List.filter_cons_of_neg (by
                  simp only [evOwner_lowerOut, beq_iff_eq]
                  exact fun hc =>
                    absurd (walkIdx_inj hh (by omega) hc) (by omega)),
                List.filter_cons_of_neg (by
                  simp only [evOwner_upperOut, beq_iff_eq]
                  exact fun hc =>
                    absurd (walkIdx_inj hh (by omega) hc) (by omega)),
                List.filter_cons_of_neg (by
                  simp only [hfQo, beq_iff_eq]
                  have := hOg g' hg'
                  omega),
                List.append_assoc, List.filter_append,
                hsc3 g' (by omega), hglue g' hg']
              refine walkSeg_glue sk
                (descIdx_mono sk g' (hp - g') (by omega)) ?_
              have hle : sk.wiresBefore (hp + 1) k + i + 1
                  ≤ sk.stageLen (g' + (hp - g')) := by
                rw [show g' + (hp - g') = hp from by omega]
                omega
              exact descIdx_le_stageLen sk hwf
                (by rw [show g' + (hp - g') = hp from by omega]
                    omega) hle
            · -- the own-stage filter
              rw [List.filter_cons_of_pos (by
                  simp only [evOwner_lowerOut, beq_self_eq_true]),
                List.filter_cons_of_pos (by
                  simp only [evOwner_upperOut, beq_self_eq_true]),
                List.filter_cons_of_neg (by
                  simp only [hfQo, beq_iff_eq]
                  omega),
                List.append_assoc, List.filter_append, List.filter_append,
                hsc2, hks3,
                hlow1, if_pos hspb, ← hFeq]
              simp only [List.cons_append, List.nil_append,
                List.append_assoc]
          · -- U2: the splice summary through the upper window
            refine emitOKOn_cons sk ?_ ?_
            · intro st hW hfix _
              rw [List.cons_append] at hW
              refine ready_upper_splice sk hwf hsched
                (t := fun G => if G = hp + 1 + 1 then i else t G)
                hh hk hsp ?_ hcoh0 ?_ ?_ ?_ st hW hfix
              · refine ancTele_rebase sk
                  (pre := ((upperOut (wpk (hp + 1)), true, k) : Ev)
                    :: ((askedOut (wpk (hp + 1 + 1)), true,
                          sk.wiresBefore (hp + 1) k + i) : Ev)
                    :: (opEvents sk (.scope hp
                          (sk.wiresBefore (hp + 1) k + i)
                          (scopeFeed sk hp
                            (sk.wiresBefore (hp + 1) k + i))) ++ L))
                  hanc ?_ ?_
                · intro G hG2 hGr
                  rw [List.filter_cons_of_neg (by
                      simp only [evOwner_upperOut, beq_iff_eq]
                      exact fun hc =>
                        absurd (walkIdx_inj hh hGr hc) (by omega)),
                    List.filter_cons_of_neg (by
                      simp only [hfQo, beq_iff_eq]
                      rw [hmFeq (by omega)]
                      exact fun hc =>
                        absurd (walkIdx_inj (by omega) hGr hc)
                          (by omega)),
                    List.filter_append,
                    hscnil (walkIdx sk G)
                      (fun hc =>
                        absurd (walkIdx_inj hh hGr hc) (by omega))
                      (fun h' hle hc =>
                        absurd (walkIdx_inj (by omega) hGr hc)
                          (by omega)),
                    List.nil_append,
                    hnilU G hG2 hGr]
                · intro hGr
                  rw [List.cons_append, List.cons_append,
                    List.filter_cons_of_neg (by
                      simp only [evOwner_upperOut, beq_iff_eq]
                      exact fun hc =>
                        absurd (walkIdx_inj hh (by omega) hc)
                          (by omega)),
                    List.filter_cons_of_pos (by
                      simp only [hfQo, hmFeq hGr, beq_self_eq_true]),
                    List.append_assoc, List.filter_append,
                    hscnil (walkIdx sk (hp + 1 + 1))
                      (fun hc =>
                        absurd (walkIdx_inj hh (by omega) hc)
                          (by omega))
                      (fun h' hle hc =>
                        absurd (walkIdx_inj (by omega) (by omega) hc)
                          (by omega)),
                    List.nil_append, hparlat hGr, hcq hGr, hdropi]
                  simp only [List.cons_append]
              · have hpm : (((upperOut (wpk (hp + 1)), true, k) : Ev)
                      :: ((askedOut (wpk (hp + 1 + 1)), true,
                            sk.wiresBefore (hp + 1) k + i) : Ev)
                      :: (opEvents sk (.scope hp
                            (sk.wiresBefore (hp + 1) k + i)
                            (scopeFeed sk hp
                              (sk.wiresBefore (hp + 1) k + i)))
                          ++ L)).filter
                    (fun e => evOwner sk e == mF)
                    = (scopeFeed sk (hp + 1) k).drop i := by
                  rw [List.filter_cons_of_neg (by
                      simp only [evOwner_upperOut, beq_iff_eq]
                      omega),
                    List.filter_cons_of_pos (by
                      simp only [hfQo, beq_self_eq_true]),
                    List.filter_append,
                    hscnil mF (by omega)
                      (fun h' hle => by
                        have := hOg h' (by omega)
                        omega),
                    List.nil_append, hks2, ← hdropi]
                have hpU : ∀ M, (∀ h', h' ≤ hp + 1 →
                      walkIdx sk h' ≠ M) → mF ≠ M →
                    (((upperOut (wpk (hp + 1)), true, k) : Ev)
                      :: ((askedOut (wpk (hp + 1 + 1)), true,
                            sk.wiresBefore (hp + 1) k + i) : Ev)
                      :: (opEvents sk (.scope hp
                            (sk.wiresBefore (hp + 1) k + i)
                            (scopeFeed sk hp
                              (sk.wiresBefore (hp + 1) k + i)))
                          ++ L)).filter
                      (fun e => evOwner sk e == M) = [] := by
                  intro M hM1 hM2
                  rw [List.filter_cons_of_neg (by
                      simp only [evOwner_upperOut, beq_iff_eq]
                      exact hM1 (hp + 1) (Nat.le_refl _)),
                    List.filter_cons_of_neg (by
                      simp only [hfQo, beq_iff_eq]
                      exact hM2),
                    List.filter_append,
                    hscnil M (hM1 (hp + 1) (Nat.le_refl _))
                      (fun h' hle => hM1 h' (by omega)),
                    List.nil_append, hksU M hM1 hM2]
                obtain ⟨i₀, hf⟩ := hfd _ i hpm hpU
                exact ⟨i₀, hf⟩
              · intro g' hg'
                rw [List.filter_cons_of_neg (by
                    simp only [evOwner_upperOut, beq_iff_eq]
                    exact fun hc =>
                      absurd (walkIdx_inj hh (by omega) hc)
                        (by omega)),
                  List.filter_cons_of_neg (by
                    simp only [hfQo, beq_iff_eq]
                    have := hOg g' hg'
                    omega),
                  List.append_assoc, List.filter_append,
                  hsc3 g' (by omega), hglue g' hg']
                refine walkSeg_glue sk
                  (descIdx_mono sk g' (hp - g') (by omega)) ?_
                have hle : sk.wiresBefore (hp + 1) k + i + 1
                    ≤ sk.stageLen (g' + (hp - g')) := by
                  rw [show g' + (hp - g') = hp from by omega]
                  omega
                exact descIdx_le_stageLen sk hwf
                  (by rw [show g' + (hp - g') = hp from by omega]
                      omega) hle
              · rw [List.filter_cons_of_pos (by
                    simp only [evOwner_upperOut, beq_self_eq_true]),
                  List.filter_cons_of_neg (by
                    simp only [hfQo, beq_iff_eq]
                    omega),
                  List.append_assoc, List.filter_append, List.filter_append,
                hsc2, hks3,
                  hlow1, ← hFeq]
                simp only [List.append_assoc]
            · -- Q: the feed query, manual
              refine emitOKOn_cons sk ?_ ?_
              · intro st hW hfix hpred
                have haq : askedOut (wpk (hp + 1 + 1))
                    = Chan.asked (wpk (hp + 1 + 1)).1 hp := by
                  unfold askedOut
                  rw [if_neg (show ¬((wpk (hp + 1 + 1)).2 < 2) from by
                    show ¬(hp + 1 + 1 < 2)
                    omega)]
                  rfl
                rw [haq] at hpred ⊢
                exact head_snd_asked sk hwf hW.toWCount hpred
              · -- the subtree, then the tail
                exact emitOKOn_append sk hsubOK hih
        · -- the unspliced disputed slot
          have hspb : (lastDOf sk (hp + 1) k == some i) = false := by
            rw [beq_eq_false_iff_ne]
            exact hsp
          rw [if_neg (by rw [hspb]; exact Bool.false_ne_true)]
            at hkidE
          simp only [List.cons_append, List.nil_append] at hkidE
          rw [hkidE]
          simp only [List.cons_append]
          refine emitOKOn_cons sk (fun st hW hfix hpred =>
            head_snd_wire sk hwf hW.toWCount (show 1 ≤ hp + 1 by omega) hpred) ?_
          refine emitOKOn_cons sk ?_ ?_
          · intro st hW hfix _
            rw [List.cons_append] at hW
            refine ready_lower sk hwf hsched
              (t := fun G => if G = hp + 1 + 1 then i else t G)
              hh hk hi hD ?_ hcoh0 ?_ ?_ ?_ st hW hfix
            · refine ancTele_rebase sk
                (pre := ((lowerOut (wpk (hp + 1)), true,
                      sk.dsBefore (hp + 1) k
                        + dRank sk (wpk (hp + 1)) k i) : Ev)
                  :: ((askedOut (wpk (hp + 1 + 1)), true,
                        sk.wiresBefore (hp + 1) k + i) : Ev)
                  :: (opEvents sk (.scope hp
                        (sk.wiresBefore (hp + 1) k + i)
                        (scopeFeed sk hp
                          (sk.wiresBefore (hp + 1) k + i))) ++ L))
                hanc ?_ ?_
              · intro G hG2 hGr
                rw [List.filter_cons_of_neg (by
                    simp only [evOwner_lowerOut, beq_iff_eq]
                    exact fun hc =>
                      absurd (walkIdx_inj hh hGr hc) (by omega)),
                  List.filter_cons_of_neg (by
                    simp only [hfQo, beq_iff_eq]
                    rw [hmFeq (by omega)]
                    exact fun hc =>
                      absurd (walkIdx_inj (by omega) hGr hc)
                        (by omega)),
                  List.filter_append,
                  hscnil (walkIdx sk G)
                    (fun hc =>
                      absurd (walkIdx_inj hh hGr hc) (by omega))
                    (fun h' hle hc =>
                      absurd (walkIdx_inj (by omega) hGr hc)
                        (by omega)),
                  List.nil_append,
                  hnilU G hG2 hGr]
              · intro hGr
                rw [List.cons_append, List.cons_append,
                  List.filter_cons_of_neg (by
                    simp only [evOwner_lowerOut, beq_iff_eq]
                    exact fun hc =>
                      absurd (walkIdx_inj hh (by omega) hc)
                        (by omega)),
                  List.filter_cons_of_pos (by
                    simp only [hfQo, hmFeq hGr, beq_self_eq_true]),
                  List.append_assoc, List.filter_append,
                  hscnil (walkIdx sk (hp + 1 + 1))
                    (fun hc =>
                      absurd (walkIdx_inj hh (by omega) hc)
                        (by omega))
                    (fun h' hle hc =>
                      absurd (walkIdx_inj (by omega) (by omega) hc)
                        (by omega)),
                  List.nil_append, hparlat hGr, hcq hGr, hdropi]
                simp only [List.cons_append]
            · have hpm : (((lowerOut (wpk (hp + 1)), true,
                      sk.dsBefore (hp + 1) k
                        + dRank sk (wpk (hp + 1)) k i) : Ev)
                    :: ((askedOut (wpk (hp + 1 + 1)), true,
                          sk.wiresBefore (hp + 1) k + i) : Ev)
                    :: (opEvents sk (.scope hp
                          (sk.wiresBefore (hp + 1) k + i)
                          (scopeFeed sk hp
                            (sk.wiresBefore (hp + 1) k + i)))
                        ++ L)).filter
                  (fun e => evOwner sk e == mF)
                  = (scopeFeed sk (hp + 1) k).drop i := by
                rw [List.filter_cons_of_neg (by
                    simp only [evOwner_lowerOut, beq_iff_eq]
                    omega),
                  List.filter_cons_of_pos (by
                    simp only [hfQo, beq_self_eq_true]),
                  List.filter_append,
                  hscnil mF (by omega)
                    (fun h' hle => by
                      have := hOg h' (by omega)
                      omega),
                  List.nil_append, hks2, ← hdropi]
              have hpU : ∀ M, (∀ h', h' ≤ hp + 1 →
                    walkIdx sk h' ≠ M) → mF ≠ M →
                  (((lowerOut (wpk (hp + 1)), true,
                        sk.dsBefore (hp + 1) k
                          + dRank sk (wpk (hp + 1)) k i) : Ev)
                    :: ((askedOut (wpk (hp + 1 + 1)), true,
                          sk.wiresBefore (hp + 1) k + i) : Ev)
                    :: (opEvents sk (.scope hp
                          (sk.wiresBefore (hp + 1) k + i)
                          (scopeFeed sk hp
                            (sk.wiresBefore (hp + 1) k + i)))
                        ++ L)).filter
                    (fun e => evOwner sk e == M) = [] := by
                intro M hM1 hM2
                rw [List.filter_cons_of_neg (by
                    simp only [evOwner_lowerOut, beq_iff_eq]
                    exact hM1 (hp + 1) (Nat.le_refl _)),
                  List.filter_cons_of_neg (by
                    simp only [hfQo, beq_iff_eq]
                    exact hM2),
                  List.filter_append,
                  hscnil M (hM1 (hp + 1) (Nat.le_refl _))
                    (fun h' hle => hM1 h' (by omega)),
                  List.nil_append, hksU M hM1 hM2]
              obtain ⟨i₀, hf⟩ := hfd _ i hpm hpU
              exact ⟨i₀, hf⟩
            · intro g' hg'
              rw [List.filter_cons_of_neg (by
                  simp only [evOwner_lowerOut, beq_iff_eq]
                  exact fun hc =>
                    absurd (walkIdx_inj hh (by omega) hc) (by omega)),
                List.filter_cons_of_neg (by
                  simp only [hfQo, beq_iff_eq]
                  have := hOg g' hg'
                  omega),
                List.append_assoc, List.filter_append,
                hsc3 g' (by omega), hglue g' hg']
              refine walkSeg_glue sk
                (descIdx_mono sk g' (hp - g') (by omega)) ?_
              have hle : sk.wiresBefore (hp + 1) k + i + 1
                  ≤ sk.stageLen (g' + (hp - g')) := by
                rw [show g' + (hp - g') = hp from by omega]
                omega
              exact descIdx_le_stageLen sk hwf
                (by rw [show g' + (hp - g') = hp from by omega]
                    omega) hle
            · rw [List.filter_cons_of_pos (by
                  simp only [evOwner_lowerOut, beq_self_eq_true]),
                List.filter_cons_of_neg (by
                  simp only [hfQo, beq_iff_eq]
                  omega),
                List.append_assoc, List.filter_append, List.filter_append,
                hsc2, hks3,
                hlow1, if_neg (by rw [hspb]; exact Bool.false_ne_true),
                ← hFeq]
              simp only [List.nil_append, List.append_assoc]
          · refine emitOKOn_cons sk ?_ ?_
            · intro st hW hfix hpred
              have haq : askedOut (wpk (hp + 1 + 1))
                  = Chan.asked (wpk (hp + 1 + 1)).1 hp := by
                unfold askedOut
                rw [if_neg (show ¬((wpk (hp + 1 + 1)).2 < 2) from by
                  show ¬(hp + 1 + 1 < 2)
                  omega)]
                rfl
              rw [haq] at hpred ⊢
              exact head_snd_asked sk hwf hW.toWCount hpred
            · exact emitOKOn_append sk hsubOK hih
      · -- an undisputed slot: wire, query, childless subtree
        have hDf : sk.childIsD (hp + 1) (sk.stageScope (hp + 1) k) i
            = false := by
          cases hDb : sk.childIsD (hp + 1) (sk.stageScope (hp + 1) k) i
          · rfl
          · exact absurd hDb hD
        rw [if_neg hD,
          if_neg (show ¬((hp + 1 == 0) = true) from by simp)]
          at hkidE
        have hn0 : sk.nChildren hp
            (sk.stageScope hp (sk.wiresBefore (hp + 1) k + i)) = 0 :=
          nChildren_kid_notD sk hwf (by omega) hh hk hi hDf
        rw [show opEvents sk (.scope hp
              (sk.wiresBefore (hp + 1) k + i) [])
            = opEvents sk (.scope hp (sk.wiresBefore (hp + 1) k + i)
                (scopeFeed sk hp (sk.wiresBefore (hp + 1) k + i)))
          from by rw [scopeFeed_nil sk hn0]] at hkidE
        simp only [List.cons_append, List.nil_append] at hkidE
        rw [hkidE]
        simp only [List.cons_append]
        refine emitOKOn_cons sk (fun st hW hfix hpred =>
          head_snd_wire sk hwf hW.toWCount (show 1 ≤ hp + 1 by omega) hpred) ?_
        refine emitOKOn_cons sk ?_ ?_
        · intro st hW hfix hpred
          have haq : askedOut (wpk (hp + 1 + 1))
              = Chan.asked (wpk (hp + 1 + 1)).1 hp := by
            unfold askedOut
            rw [if_neg (show ¬((wpk (hp + 1 + 1)).2 < 2) from by
              show ¬(hp + 1 + 1 < 2)
              omega)]
            rfl
          rw [haq] at hpred ⊢
          exact head_snd_asked sk hwf hW.toWCount hpred
        · exact emitOKOn_append sk hsubOK hih

end StreamingMirror.Sched
