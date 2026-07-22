/-
The minting ladder and Step-4 coverage (MUX-PROGRESS §4, the residue's
final lemma): at a reachable stuck drained σ*-causal×σ*-causal state,
every peer-process event τ-below the withheld push is ANNOUNCED-LAID —
its layout's every consulted record was minted by an arrival whose
send sits τ-below the consulting event — and therefore enters the
causal closure at its own τ stage. `causalStuckCoverage` at the bottom
discharges `CausalStuckCoverage`, and C1's charter refutation becomes
unconditional (Proofs/C1.lean).

# The drained decode

At a stuck drained state the announced view is exactly the peer's send
frontier: pipes empty makes `deliveredCount = sentOf` on every peer
stream (`drained_delivered`), so "the minting frame's send is τ-below
the consulting event" — a STATIC schedule fact — implies "the record is
announced". Every chain below reduces to that one currency.

# The ladder

The τ-chains are two-hop rungs, one level per rung: a stage-`h` block
`k` event sits trace-above its prologue receive (E3), whose send is the
height-`h+1` frame `k` (E1); that send sits in the stage-`h+1` trace
inside its parent block `n`, trace-above THAT block's prologue — and
the recursion continues on `(h+1, n)`. The rung indices are exactly the
BFS ancestors, so the harvested frame sends are exactly the mints the
census (`levelA`) consults; `census_reach` runs this ladder by strong
induction on τ. The per-family laid lemmas (`walk_laid`,
`absorb_laid`, `asm_laid`, the opener/finale specials) then extend each
announced layout past any true-trace event below the wall.

# The coverage induction

`closure_coverage` (Proofs/SigmaStarLive.lean) re-run over
`inevitableA`, restricted to the announced flatten (own-endpoint events
are never consulted: no internal channel crosses the link, and wire
sends are grounded evidence): every flatten member τ-below the wall
I-steps into the causal closure by its own τ stage — wire-send E1/E2
predecessors are grounded against the drained counts, internal
predecessors are announced-laid by the minting lemmas, and the E3 past
is the announced trace itself. The closure stage is capped through
`mem_inevitableA_of_closureNA` (the saturation argument), since τ can
exceed the announced universe's length.
-/
import StreamingMirror.Mux.Proofs.CausalLive
import StreamingMirror.Mux.Proofs.SigmaStarLive

namespace StreamingMirror.Mux

open Model
open Sched (Ev procsE scheduleE performed evIdx)

variable {sk : Skel}

-- ======================================================== the list kit

/-- Membership in a canonical projection is a seq bound. -/
private theorem mem_canon_iff {c : Chan} {b : Bool} {m n : Nat} :
    ((c, b, n) : Ev) ∈ Sched.canon c b m ↔ n < m := by
  unfold Sched.canon
  constructor
  · intro h
    obtain ⟨j, hj, hje⟩ := List.mem_map.mp h
    have hjn : j = n := by
      have := congrArg (fun e : Ev => e.2.2) hje
      simpa using this
    rw [← hjn]
    exact List.mem_range.mp hj
  · intro h
    exact List.mem_map.mpr ⟨n, List.mem_range.mpr h, rfl⟩

/-- Transfer a `(c, b, ·)`-shaped membership through a canonical
projection: the trace's projection is canon, so membership is exactly
the seq bound. -/
private theorem mem_of_canon {c : Chan} {b : Bool} {n m : Nat}
    {T : List Ev} (hc : Sched.proj c b T = Sched.canon c b m) :
    (((c, b, n) : Ev) ∈ T ↔ n < m) := by
  have hproj : (((c, b, n) : Ev) ∈ T ↔ ((c, b, n) : Ev)
      ∈ Sched.proj c b T) := by
    unfold Sched.proj
    constructor
    · intro h
      exact List.mem_filter.mpr ⟨h, by simp⟩
    · intro h
      exact (List.mem_filter.mp h).1
  rw [hproj, hc, mem_canon_iff]

/-- Split `range n` around a member `k`. -/
private theorem range_split {k n : Nat} (hk : k < n) :
    List.range n
      = List.range k ++ k :: List.range' (k + 1) (n - (k + 1)) := by
  rw [List.range_eq_range', List.range_eq_range',
    show n = k + (n - k) from by omega, ← List.range'_append (step := 1),
    show k + (n - k) = n from by omega]
  congr 1
  rw [show n - k = 1 + (n - (k + 1)) from by omega,
    ← List.range'_append (step := 1)]
  simp

/-- A sublist of one flatMap block embeds into the flatMap. -/
private theorem sublist_flatMap_block {f : Nat → List Ev}
    {l : List Ev} {k n : Nat} (hk : k < n)
    (hl : l.Sublist (f k)) :
    l.Sublist ((List.range n).flatMap f) := by
  rw [range_split hk, List.flatMap_append, List.flatMap_cons]
  refine List.Sublist.trans ?_ (List.sublist_append_right ..)
  exact hl.trans (List.sublist_append_left ..)

/-- A pair across two flatMap blocks embeds into the flatMap. -/
private theorem sublist_flatMap_pair {f : Nat → List Ev}
    {x y : Ev} {j k n : Nat} (hjk : j < k) (hk : k < n)
    (hx : x ∈ f j) (hy : y ∈ f k) :
    ([x, y] : List Ev).Sublist ((List.range n).flatMap f) := by
  rw [range_split hk, List.flatMap_append, List.flatMap_cons]
  have h1 : ([x] : List Ev).Sublist ((List.range k).flatMap f) :=
    sublist_flatMap_block hjk (List.singleton_sublist.mpr hx)
  have h2 : ([y] : List Ev).Sublist (f k ++ (List.range' (k + 1)
      (n - (k + 1))).flatMap f) :=
    (List.singleton_sublist.mpr hy).trans (List.sublist_append_left ..)
  exact List.Sublist.append h1 h2

/-- An element of a list headed by `x` is `x` or pairs after it. -/
private theorem pair_of_mem_cons {x e : Ev} {rest : List Ev}
    (he : e ∈ x :: rest) :
    e = x ∨ ([x, e] : List Ev).Sublist (x :: rest) := by
  rcases List.mem_cons.mp he with rfl | hr
  · exact Or.inl rfl
  · exact Or.inr (List.cons_sublist_cons.mpr
      (List.singleton_sublist.mpr hr))

/-- Pointwise-implied filters are sublists. -/
private theorem filter_sublist_of_impl {α : Type _} {P Q : α → Bool} :
    ∀ {l : List α}, (∀ x ∈ l, P x = true → Q x = true) →
      (l.filter P).Sublist (l.filter Q) := by
  intro l
  induction l with
  | nil => intro _; exact List.Sublist.refl _
  | cons a t ih =>
      intro h
      have ht := ih fun x hx => h x (List.mem_cons_of_mem a hx)
      rw [List.filter_cons, List.filter_cons]
      by_cases hP : P a = true
      · rw [if_pos hP, if_pos (h a (List.mem_cons_self ..) hP)]
        exact List.cons_sublist_cons.mpr ht
      · rw [if_neg (by simpa using hP)]
        by_cases hQ : Q a = true
        · rw [if_pos hQ]
          exact ht.trans (List.sublist_cons_self ..)
        · rw [if_neg (by simpa using hQ)]
          exact ht

-- =============================================== closure saturation
-- The coverage induction enters events at stage τ+1, and τ can exceed
-- the announced universe's length; saturation absorbs any stage into
-- `inevitableA` (which runs to universe depth).

/-- Every causal closure stage is a filter of the universe with a
predicate the next stage implies: consecutive stages are sublists. -/
private theorem closureNA_sublist_succ {av : AView} {tr : List MObs}
    {univ : List Ev} {procsL : List (List Ev)} (n : Nat) :
    (closureNA av tr univ procsL n).Sublist
      (closureNA av tr univ procsL (n + 1)) := by
  cases n with
  | zero =>
      show (univ.filter _).Sublist (univ.filter _)
      refine filter_sublist_of_impl fun x hx hg => ?_
      rw [Bool.or_eq_true, Bool.or_eq_true]
      exact Or.inl (Or.inr hg)
  | succ m =>
      show (univ.filter _).Sublist (univ.filter _)
      refine filter_sublist_of_impl fun x hx hcond => ?_
      have hmem : x ∈ closureNA av tr univ procsL (m + 1) :=
        List.mem_filter.mpr ⟨hx, hcond⟩
      rw [Bool.or_eq_true, Bool.or_eq_true]
      exact Or.inl (Or.inl ((List.contains_iff_mem ..).mpr hmem))

/-- A plateau of the closure chain propagates forward. -/
private theorem closureNA_plateau {av : AView} {tr : List MObs}
    {univ : List Ev} {procsL : List (List Ev)} {n : Nat}
    (heq : closureNA av tr univ procsL (n + 1)
      = closureNA av tr univ procsL n) :
    ∀ m, n ≤ m →
      closureNA av tr univ procsL m = closureNA av tr univ procsL n := by
  intro m
  induction m with
  | zero =>
      intro h0
      have : n = 0 := by omega
      rw [this]
  | succ m ih =>
      intro hm
      by_cases hlast : n = m + 1
      · rw [hlast]
      · have hnm : n ≤ m := by omega
        show closureStepA av tr univ procsL
          (closureNA av tr univ procsL m) = _
        rw [ih hnm]
        exact heq

/-- The closure chain's stage lengths are bounded by the universe. -/
private theorem closureNA_length_le {av : AView} {tr : List MObs}
    {univ : List Ev} {procsL : List (List Ev)} (n : Nat) :
    (closureNA av tr univ procsL n).length ≤ univ.length := by
  cases n with
  | zero => exact List.Sublist.length_le (List.filter_sublist)
  | succ m => exact List.Sublist.length_le (List.filter_sublist)

/-- Saturation: a member of any causal closure stage is causally
inevitable — the chain plateaus within universe-many passes, and the
inevitable set is the universe-depth stage. -/
theorem mem_inevitableA_of_closureNA {av : AView} {tr : List MObs}
    {e : Ev} {n : Nat}
    (he : e ∈ closureNA av tr (evUnivA av tr) (announcedProcs av) n) :
    e ∈ inevitableA av tr := by
  suffices h : ∀ (univ : List Ev) (procsL : List (List Ev)),
      e ∈ closureNA av tr univ procsL n →
      e ∈ closureNA av tr univ procsL univ.length by
    exact h (evUnivA av tr) (announcedProcs av) he
  clear he
  intro univ procsL he
  generalize hLdef : univ.length = L
  by_cases hn : n ≤ L
  · exact closureNA_le hn e he
  -- find a plateau at or below L
  have hplat : ∃ n₀, n₀ ≤ L ∧ closureNA av tr univ procsL (n₀ + 1)
      = closureNA av tr univ procsL n₀ := by
    by_contra hno
    have hno' : ∀ n₀, n₀ ≤ L →
        closureNA av tr univ procsL (n₀ + 1)
          ≠ closureNA av tr univ procsL n₀ :=
      fun n₀ h hc => hno ⟨n₀, h, hc⟩
    have hgrow : ∀ m, m ≤ L + 1 →
        m ≤ (closureNA av tr univ procsL m).length := by
      intro m
      induction m with
      | zero => intro _; exact Nat.zero_le _
      | succ m ih =>
          intro hm
          have hsub := closureNA_sublist_succ
            (av := av) (tr := tr) (univ := univ) (procsL := procsL) m
          have hne := hno' m (by omega)
          have hlt : (closureNA av tr univ procsL m).length
              < (closureNA av tr univ procsL (m + 1)).length := by
            rcases Nat.lt_or_ge (closureNA av tr univ procsL m).length
              (closureNA av tr univ procsL (m + 1)).length with h | h
            · exact h
            · exact absurd (hsub.eq_of_length
                (Nat.le_antisymm hsub.length_le h)).symm hne
          have := ih (by omega)
          omega
    have h1 := hgrow (L + 1) (Nat.le_refl _)
    have h2 := closureNA_length_le
      (av := av) (tr := tr) (univ := univ) (procsL := procsL) (L + 1)
    rw [hLdef] at h2
    omega
  obtain ⟨n₀, hn₀L, heq⟩ := hplat
  have hn₀n : closureNA av tr univ procsL n
      = closureNA av tr univ procsL n₀ :=
    closureNA_plateau heq n (by omega)
  have hn₀L' : closureNA av tr univ procsL L
      = closureNA av tr univ procsL n₀ :=
    closureNA_plateau heq L hn₀L
  rw [hn₀L']
  rw [hn₀n] at he
  exact he

-- =========================================== the stuck drained wall
-- Every minting and coverage lemma runs against the same ambient: a
-- well-formed margin-0 session, the transport ground facts, both pipes
-- drained, and the chase's τ-wall `perf` — everything scheduled below
-- `N` is performed.

/-- The stuck drained wall: the ambient facts of the coverage theorem's
call site, bundled so the ladder's signatures stay readable. -/
structure Wall (sk : Skel) (s : MState) (N : Nat) : Prop where
  wf : sk.wellFormed = true
  m0 : ∀ sc, sk.dCount sc ≤ sk.capLevel
  minv : MuxInv sk s
  pipeI : s.pipe .I = []
  pipeR : s.pipe .R = []
  perf : ∀ g ∈ scheduleE sk, evIdx g (scheduleE sk) < N →
    performed sk s.base g

namespace Wall

variable {s : MState} {N : Nat}

/-- Both pipes drained, party-generic. -/
theorem pipe_empty (W : Wall sk s N) (q : Party) : s.pipe q = [] := by
  cases q
  · exact W.pipeI
  · exact W.pipeR

/-- The drained decode: with the pipes empty, every real stream's
delivered count IS its producer's send count — the receiving machine's
announced view sees everything the peer has committed to the wire. -/
theorem drained_delivered (W : Wall sk s N) (q : Party) (g : Nat)
    (hmem : Chan.wire q g ∈ allChans sk) :
    deliveredCount (s.hist q.other) g
      = sentOf sk s.base (Chan.wire q g) := by
  have h1 := W.minv.delivered_eq q g hmem
  have h2 := W.minv.flow_wire q g hmem
  have h3 : pipeCount s (Chan.wire q g) = 0 := by
    rw [pipeCount]
    show (List.count _ (s.pipe (wireParty (Chan.wire q g)))) = 0
    rw [show wireParty (Chan.wire q g) = q from rfl, W.pipe_empty q]
    rfl
  omega

/-- Own pushes decode to the send counter at the stuck state. -/
theorem drained_pushed (W : Wall sk s N) (q : Party) (g : Nat)
    (hmem : Chan.wire q g ∈ allChans sk) :
    pushedCount (s.hist q) g = sentOf sk s.base (Chan.wire q g) :=
  W.minv.pushed_eq q g hmem

/-- A scheduled send below the wall is performed: its seq is below the
send counter. -/
theorem sent_of_below (W : Wall sk s N) {c : Chan} {n : Nat}
    (hmem : ((c, true, n) : Ev) ∈ scheduleE sk)
    (hτ : evIdx ((c, true, n) : Ev) (scheduleE sk) < N) :
    n < sentOf sk s.base c := by
  have := W.perf _ hmem hτ
  rwa [performed_snd_iff] at this

/-- A scheduled receive below the wall is performed: its seq is below
the consume counter. -/
theorem recvd_of_below (W : Wall sk s N) {c : Chan} {n : Nat}
    (hmem : ((c, false, n) : Ev) ∈ scheduleE sk)
    (hτ : evIdx ((c, false, n) : Ev) (scheduleE sk) < N) :
    n < recvdOf sk s.base c := by
  have := W.perf _ hmem hτ
  rwa [performed_rcv_iff] at this

end Wall

-- ================================================ the τ-chain helpers

/-- A strict trace-prior of an event is scheduled strictly τ-below
it. -/
theorem tau_prior (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    {T : List Ev} (hT : T ∈ procsE sk) {x y : Ev}
    (hxy : ([x, y] : List Ev).Sublist T) :
    x ∈ scheduleE sk
      ∧ evIdx x (scheduleE sk) < evIdx y (scheduleE sk) := by
  have hx : x ∈ T := hxy.mem (List.mem_cons_self ..)
  exact ⟨(Sched.trace_sublistE sk hwf hm0 hT).mem hx,
    tau_lt_of_trace_pair hwf hm0 hT hxy⟩

-- =========================================== stage/height arithmetic

/-- The owning party of walk stage `h`, as `procsE`'s walk order fixes
it: initiator on odd consume heights, responder on even. -/
def stageParty (h : Nat) : Party :=
  if h % 2 == 1 then .I else .R

/-- A walk key is its stage's owner at its stage's height. -/
theorem walkKeys_stageParty (hwf : sk.wellFormed = true)
    {q : Party} {h : Nat} (hpk : (q, h) ∈ sk.walkKeys) :
    q = stageParty h ∧ h < sk.rootH := by
  obtain ⟨hlt, hpar⟩ := Sched.walkKeys_parity sk hwf hpk
  unfold stageParty
  rcases hpar with ⟨rfl, hodd⟩ | ⟨rfl, heven⟩
  · rw [if_pos (by simpa using hodd)]
    exact ⟨rfl, hlt⟩
  · rw [if_neg (by simp [heven])]
    exact ⟨rfl, hlt⟩

/-- The stage owner's key is a walk key. -/
theorem stageParty_mem_walkKeys (hwf : sk.wellFormed = true)
    {h : Nat} (hh : h < sk.rootH) :
    (stageParty h, h) ∈ sk.walkKeys := by
  unfold stageParty
  by_cases hpar : h % 2 = 1
  · rw [if_pos (by simpa using hpar)]
    exact Sched.mem_walkKeys_of sk hwf hh (Or.inl ⟨rfl, hpar⟩)
  · rw [if_neg (by simpa using hpar)]
    exact Sched.mem_walkKeys_of sk hwf hh (Or.inr ⟨rfl, by omega⟩)

/-- Stage owners alternate. -/
theorem stageParty_succ (h : Nat) :
    stageParty (h + 1) = (stageParty h).other := by
  unfold stageParty
  rcases Nat.mod_two_eq_zero_or_one h with he | ho
  · have h1 : (h + 1) % 2 = 1 := by omega
    rw [if_pos (by simpa using h1), if_neg (by simp [he])]
    rfl
  · have h0 : (h + 1) % 2 = 0 := by omega
    rw [if_neg (by simp [h0]), if_pos (by simpa using ho)]
    rfl

/-- A peer walk stage's height is a minting height: the announced-id
decode reads exactly the peer's streams. -/
theorem mem_peerMintHeights (hwf : sk.wellFormed = true) (p : Party)
    {g : Nat} (hpk : (p.other, g) ∈ sk.walkKeys) :
    g ∈ peerMintHeights sk p := by
  obtain ⟨hlt, hpar⟩ := Sched.walkKeys_parity sk hwf hpk
  have hev : sk.rootH % 2 = 0 := (wf_rootH hwf).1
  unfold peerMintHeights
  cases p with
  | I =>
      rw [if_pos (show (Party.I == Party.I) = true from rfl)]
      have hR : g % 2 = 0 := by
        rcases hpar with ⟨hc, -⟩ | ⟨-, h⟩
        · exact absurd hc (by decide)
        · exact h
      exact List.mem_map.mpr ⟨(sk.rootH - 2 - g) / 2,
        List.mem_range.mpr (by omega), by omega⟩
  | R =>
      rw [if_neg (show ¬((Party.R == Party.I) = true) by decide)]
      have hI : g % 2 = 1 := by
        rcases hpar with ⟨-, h⟩ | ⟨hc, -⟩
        · exact h
        · exact absurd hc (by decide)
      exact List.mem_map.mpr ⟨(sk.rootH - 1 - g) / 2,
        List.mem_range.mpr (by omega), by omega⟩

/-- A peer walk stage appears in the announced stage list. -/
theorem mem_peerStagesA (hwf : sk.wellFormed = true) (p : Party)
    (tr : List MObs) {h : Nat} (hpk : (p.other, h) ∈ sk.walkKeys) :
    h ∈ peerStagesA (aviewOf sk p tr) := by
  have hmint := mem_peerMintHeights hwf p hpk
  unfold peerMintHeights at hmint
  unfold peerStagesA
  cases p with
  | I =>
      rw [if_pos (show (Party.I == Party.I) = true from rfl)] at hmint
      rw [show (aviewOf sk Party.I tr).party = Party.I from rfl,
        if_pos (show (Party.I == Party.I) = true from rfl),
        show (aviewOf sk Party.I tr).rootH = sk.rootH from rfl]
      exact hmint
  | R =>
      rw [if_neg (show ¬((Party.R == Party.I) = true) by decide)] at hmint
      rw [show (aviewOf sk Party.R tr).party = Party.R from rfl,
        if_neg (show ¬((Party.R == Party.I) = true) by decide),
        show (aviewOf sk Party.R tr).rootH = sk.rootH from rfl]
      exact hmint

/-- A peer assembler's height appears in the announced height list. -/
theorem mem_peerAsmHeightsA (p : Party) (tr : List MObs) {j : Nat}
    (hpk : (p.other, j) ∈ sk.asmKeys) :
    j ∈ peerAsmHeightsA (aviewOf sk p tr) := by
  unfold Skel.asmKeys at hpk
  unfold peerAsmHeightsA
  rcases List.mem_append.mp hpk with hI | hR
  · obtain ⟨m, hm, hme⟩ := List.mem_map.mp hI
    rw [List.mem_range] at hm
    rw [Prod.mk.injEq] at hme
    have hp : p = Party.R := by
      cases p
      · exact absurd hme.1 (by decide)
      · rfl
    subst hp
    rw [show (aviewOf sk Party.R tr).party = Party.R from rfl,
      if_neg (show ¬((Party.R == Party.I) = true) by decide),
      show (aviewOf sk Party.R tr).rootH = sk.rootH from rfl]
    exact List.mem_map.mpr ⟨m, List.mem_range.mpr hm, hme.2⟩
  · obtain ⟨m, hm, hme⟩ := List.mem_map.mp hR
    rw [List.mem_range] at hm
    rw [Prod.mk.injEq] at hme
    have hp : p = Party.I := by
      cases p
      · rfl
      · exact absurd hme.1 (by decide)
    subst hp
    rw [show (aviewOf sk Party.I tr).party = Party.I from rfl,
      if_pos (show (Party.I == Party.I) = true from rfl),
      show (aviewOf sk Party.I tr).rootH = sk.rootH from rfl]
    exact List.mem_map.mpr ⟨m, List.mem_range.mpr hm, hme.2⟩

/-- A real wire channel's height is one of its party's stream heights:
`allChans` holds root wires and walk outputs (as wires) only. -/
theorem wireHeights_of_allChans {q : Party} {g : Nat}
    (hmem : Chan.wire q g ∈ allChans sk) :
    g ∈ wireHeights sk q := by
  unfold allChans at hmem
  rcases List.mem_append.mp hmem with h1 | h2
  rcases List.mem_append.mp h1 with h3 | h4
  · obtain ⟨pk, hpk, hin⟩ := List.mem_flatMap.mp h3
    obtain ⟨p₁, g₁⟩ := pk
    simp only [wireOut, askedIn, upperOut, lowerOut, List.mem_cons,
      List.mem_singleton, List.not_mem_nil, or_false] at hin
    rcases hin with h | h | h | h
    · obtain ⟨hq, hg⟩ := Chan.wire.inj h
      subst hq
      subst hg
      rw [wireHeights]
      exact List.mem_cons_of_mem _
        (List.mem_filterMap.mpr ⟨(q, g), hpk, by simp⟩)
    · cases h
    · cases h
    · cases h
  · obtain ⟨pk, -, hin⟩ := List.mem_map.mp h4
    cases hin
  · simp only [List.mem_cons, List.mem_singleton, List.not_mem_nil,
      or_false] at h2
    rcases h2 with h | h | h | h | h | h | h
    · obtain ⟨hq, hg⟩ := Chan.wire.inj h
      subst hq
      subst hg
      rw [wireHeights]
      exact List.mem_cons_self ..
    · obtain ⟨hq, hg⟩ := Chan.wire.inj h
      subst hq
      subst hg
      rw [wireHeights]
      exact List.mem_cons_self ..
    all_goals cases h

/-- A stream height names a real channel. -/
theorem mem_allChans_of_wireHeights {q : Party} {g : Nat}
    (hg : g ∈ wireHeights sk q) : Chan.wire q g ∈ allChans sk := by
  rw [wireHeights] at hg
  rcases List.mem_cons.mp hg with rfl | hmem
  · exact mem_allChans_wire_root q
  · obtain ⟨pk, hpk, hval⟩ := List.mem_filterMap.mp hmem
    by_cases hq : (pk.1 == q) = true
    · rw [if_pos hq] at hval
      injection hval with hval
      have hpkeq : pk = (q, g) := by
        obtain ⟨p₁, h₁⟩ := pk
        simp only at hval
        rw [Prod.mk.injEq]
        exact ⟨by simpa using hq, hval⟩
      rw [hpkeq] at hpk
      exact mem_allChans_wireOut hpk
    · rw [if_neg hq] at hval
      cases hval

-- ================================================= the mint bridges
-- From a frame send scheduled below the wall to an announced record:
-- performed ⇒ sent ⇒ (pipes empty) delivered ⇒ minted.

namespace Wall

variable {s : MState} {N : Nat}

/-- A peer frame send below the wall is delivered: the drained decode
turns the send counter into the announced view's delivery count. -/
theorem delivered_of_send (W : Wall sk s N) (p : Party) {g n : Nat}
    (hch : Chan.wire p.other g ∈ allChans sk)
    (hmem : ((Chan.wire p.other g, true, n) : Ev) ∈ scheduleE sk)
    (hτ : evIdx ((Chan.wire p.other g, true, n) : Ev) (scheduleE sk)
      < N) :
    n < deliveredCount (s.hist p) g := by
  have hsent := W.sent_of_below hmem hτ
  have hdel := W.drained_delivered p.other g hch
  rw [Party.other_other] at hdel
  omega

/-- A peer walk frame send below the wall mints its about-scope and
that scope's kids (rule 2 cashed at the stuck state). -/
theorem minted_of_send (W : Wall sk s N) (p : Party) {g n : Nat}
    (hpk : (p.other, g) ∈ sk.walkKeys) (hg0 : g ≠ 0)
    (hmem : ((Chan.wire p.other g, true, n) : Ev) ∈ scheduleE sk)
    (hτ : evIdx ((Chan.wire p.other g, true, n) : Ev) (scheduleE sk)
      < N)
    (hlen : n < (sk.scopesAt g).length) :
    (sk.scopesAt g).getD n 0 ∈ announcedIds sk p (s.hist p)
      ∧ ∀ v ∈ (sk.scope ((sk.scopesAt g).getD n 0)).kids,
          v ∈ announcedIds sk p (s.hist p) :=
  announced_of_delivered (mem_peerMintHeights W.wf p hpk) hg0
    (W.delivered_of_send p (mem_allChans_wireOut hpk) hmem hτ) hlen

/-- The peer's opening frame send below the wall delivers the session's
first arrival (rule 1's currency). -/
theorem root_delivered (W : Wall sk s N) (p : Party)
    (hmem : ((Chan.wire p.other sk.rootH, true, 0) : Ev) ∈ scheduleE sk)
    (hτ : evIdx ((Chan.wire p.other sk.rootH, true, 0) : Ev)
      (scheduleE sk) < N) :
    0 < deliveredCount (s.hist p) sk.rootH :=
  W.delivered_of_send p (mem_allChans_wire_root _) hmem hτ

end Wall

-- ============================================ BFS positional reading
-- The alignment conjunct read positionally: above the leaf stage a
-- stage's kid lists flattened are the level below, so wire seqs
-- decompose into (parent block, child index) and back.

/-- The BFS split at a stage cursor: the level below decomposes as the
first `n` blocks' kids — whose flattened length is the wire prefix
sum — plus a remainder. -/
private theorem bfs_split (hwf : sk.wellFormed = true)
    {h : Nat} (h1 : 1 ≤ h) (hh : h < sk.rootH) :
    ∀ n, n ≤ sk.stageLen h →
      ∃ rest, sk.scopesAt h
          = ((sk.stageScopes h).take n).flatMap
              (fun u => (sk.scope u).kids) ++ rest
        ∧ (((sk.stageScopes h).take n).flatMap
              (fun u => (sk.scope u).kids)).length
            = sk.wiresBefore h n := by
  intro n
  induction n with
  | zero =>
      intro _
      exact ⟨sk.scopesAt h, rfl, rfl⟩
  | succ n ih =>
      intro hn
      have hn' : n < sk.stageLen h := by omega
      obtain ⟨rest, hsplit, hlen⟩ := ih (by omega)
      have htake : (sk.stageScopes h).take (n + 1)
          = (sk.stageScopes h).take n ++ [sk.stageScope h n] := by
        unfold Skel.stageLen at hn'
        rw [List.take_succ, List.getElem?_eq_getElem hn']
        unfold Skel.stageScope
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hn']
        rfl
      -- the full BFS split pins the remainder's head segment
      have hfull : sk.scopesAt h
          = ((sk.stageScopes h).take (n + 1)).flatMap
              (fun u => (sk.scope u).kids)
            ++ ((sk.stageScopes h).drop (n + 1)).flatMap
              (fun u => (sk.scope u).kids) := by
        rw [← List.flatMap_append, List.take_append_drop]
        exact (wf_bfs_aligned hwf hh).symm
      rw [htake, List.flatMap_append] at hfull
      rw [List.append_assoc] at hfull
      have hrest : rest
          = (sk.scope (sk.stageScope h n)).kids
              ++ ((sk.stageScopes h).drop (n + 1)).flatMap
                (fun u => (sk.scope u).kids) := by
        have h2 := hsplit.symm.trans hfull
      -- kids ++ dropFlat, flatMap over singleton reduces
        rwa [List.flatMap_cons, List.flatMap_nil, List.append_nil,
          List.append_cancel_left_eq] at h2
      refine ⟨((sk.stageScopes h).drop (n + 1)).flatMap
        (fun u => (sk.scope u).kids), ?_, ?_⟩
      · rw [htake, List.flatMap_append, List.flatMap_cons,
          List.flatMap_nil, List.append_nil, List.append_assoc,
          ← hrest]
        exact hsplit
      · rw [htake, List.flatMap_append, List.flatMap_cons,
          List.flatMap_nil, List.append_nil, List.length_append, hlen,
          Sched.wiresBefore_succ sk hn']
        congr 1
        unfold Skel.nChildren
        rw [if_neg (by simpa using show h ≠ 0 by omega)]

/-- Wire prefix sums stay inside the level below. -/
private theorem wiresBefore_le_scopesAt (hwf : sk.wellFormed = true)
    {h : Nat} (h1 : 1 ≤ h) (hh : h < sk.rootH) {n : Nat}
    (hn : n ≤ sk.stageLen h) :
    sk.wiresBefore h n ≤ (sk.scopesAt h).length := by
  obtain ⟨rest, hsplit, hlen⟩ := bfs_split hwf h1 hh n hn
  have := congrArg List.length hsplit
  rw [List.length_append, hlen] at this
  omega

/-- The `k`-th scope of the level below stage `h` is the `i`-th kid of
its parent block `n`, positionally: `k = wiresBefore h n + i`. -/
private theorem stageScope_kid (hwf : sk.wellFormed = true)
    {h : Nat} (h1 : 1 ≤ h) (hh : h < sk.rootH) {n i : Nat}
    (hn : n < sk.stageLen h)
    (hi : i < (sk.scope (sk.stageScope h n)).kids.length) :
    (sk.scopesAt h).getD (sk.wiresBefore h n + i) 0
      = (sk.scope (sk.stageScope h n)).kids.getD i 0 := by
  obtain ⟨rest, hsplit, hlen⟩ := bfs_split hwf h1 hh n (by omega)
  have hfull : sk.scopesAt h
      = ((sk.stageScopes h).take n).flatMap
          (fun u => (sk.scope u).kids)
        ++ ((sk.scope (sk.stageScope h n)).kids
          ++ ((sk.stageScopes h).drop (n + 1)).flatMap
            (fun u => (sk.scope u).kids)) := by
    have htake : (sk.stageScopes h).take (n + 1)
        = (sk.stageScopes h).take n ++ [sk.stageScope h n] := by
      unfold Skel.stageLen at hn
      rw [List.take_succ, List.getElem?_eq_getElem hn]
      unfold Skel.stageScope
      rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hn]
      rfl
    calc sk.scopesAt h
        = ((sk.stageScopes h).take (n + 1)
            ++ (sk.stageScopes h).drop (n + 1)).flatMap
            (fun u => (sk.scope u).kids) := by
          rw [List.take_append_drop]
          exact (wf_bfs_aligned hwf hh).symm
      _ = _ := by
          rw [List.flatMap_append, htake, List.flatMap_append,
            List.flatMap_cons, List.flatMap_nil, List.append_nil,
            List.append_assoc]
  rw [hfull, List.getD_eq_getElem?_getD,
    List.getElem?_append_right (by rw [hlen]; omega), hlen,
    show sk.wiresBefore h n + i - sk.wiresBefore h n = i from by omega,
    List.getElem?_append_left hi]
  rw [List.getD_eq_getElem?_getD]

/-- Locate a wire seq's parent block: the least block whose prefix sum
exceeds it. -/
private theorem exists_parent_block {h k : Nat}
    (hk : k < sk.wiresBefore h (sk.stageLen h)) :
    ∃ n, n < sk.stageLen h ∧ sk.wiresBefore h n ≤ k
      ∧ k < sk.wiresBefore h (n + 1) := by
  suffices hgen : ∀ M, M ≤ sk.stageLen h → k < sk.wiresBefore h M →
      ∃ n, n < M ∧ sk.wiresBefore h n ≤ k
        ∧ k < sk.wiresBefore h (n + 1) by
    obtain ⟨n, hn, hle, hlt⟩ := hgen (sk.stageLen h) (Nat.le_refl _) hk
    exact ⟨n, hn, hle, hlt⟩
  intro M
  induction M with
  | zero =>
      intro _ hlt
      exact absurd hlt (by unfold Skel.wiresBefore; simp)
  | succ M ih =>
      intro hM hlt
      by_cases hin : k < sk.wiresBefore h M
      · obtain ⟨n, hn, h1, h2⟩ := ih (by omega) hin
        exact ⟨n, by omega, h1, h2⟩
      · exact ⟨M, by omega, by omega, hlt⟩

/-- A chunk's wire event is its head. -/
private theorem wire_mem_childChunk (pk : Party × Nat) (k i : Nat) :
    ((wireOut pk, true, sk.wiresBefore pk.2 k + i) : Ev)
      ∈ Sched.childChunk sk pk k i := by
  unfold Sched.childChunk
  by_cases hD : sk.childIsD pk.2 (sk.stageScope pk.2 k) i = true
  · rw [if_pos hD]
    exact List.mem_cons_self ..
  · rw [if_neg hD]
    exact List.mem_cons_self ..

/-- A chunk member is a scope-block member (encoder order). -/
private theorem chunk_mem_scopeBlockE {pk : Party × Nat} {k i : Nat}
    {x : Ev} (hi : i < sk.nChildren pk.2 (sk.stageScope pk.2 k))
    (hx : x ∈ Sched.childChunk sk pk k i) :
    x ∈ Sched.scopeBlockE sk pk k := by
  unfold Sched.scopeBlockE
  refine List.mem_cons_of_mem _ (List.mem_cons_of_mem _ ?_)
  unfold Sched.scopeSendsE
  refine List.mem_append.mpr (.inl ?_)
  exact List.mem_flatten.mpr ⟨Sched.childChunk sk pk k i,
    List.mem_map.mpr ⟨i, List.mem_range.mpr hi, rfl⟩, hx⟩

/-- A scope block's prologue receive pairs before any other member. -/
private theorem prologue_pair {pk : Party × Nat} {k : Nat} {x : Ev}
    (hx : x ∈ Sched.scopeBlockE sk pk k)
    (hne : x ≠ ((wireIn pk, false, k) : Ev)) :
    ([((wireIn pk, false, k) : Ev), x] : List Ev).Sublist
      (Sched.scopeBlockE sk pk k) := by
  unfold Sched.scopeBlockE at hx ⊢
  rcases pair_of_mem_cons hx with heq | hpair
  · exact absurd heq hne
  · exact hpair

/-- Locate a wire send inside its stage's encoder trace, paired after
its parent block's prologue receive: the ladder's hop-B engine. -/
theorem wire_send_locate (hwf : sk.wellFormed = true)
    {q : Party} {h : Nat} (hq : (q, h) ∈ sk.walkKeys) {m : Nat}
    (hm : m < sk.wiresBefore h (sk.stageLen h)) :
    ∃ n, n < sk.stageLen h ∧ sk.wiresBefore h n ≤ m
      ∧ m < sk.wiresBefore h (n + 1)
      ∧ ([((wireIn (q, h), false, n) : Ev),
          ((Chan.wire q h, true, m) : Ev)] : List Ev).Sublist
          (Sched.walkEventsE sk (q, h)) := by
  obtain ⟨n, hn, hle, hlt⟩ := exists_parent_block hm
  refine ⟨n, hn, hle, hlt, ?_⟩
  have hi : m - sk.wiresBefore h n
      < sk.nChildren h (sk.stageScope h n) := by
    have := Sched.wiresBefore_succ sk hn
    omega
  have hwmem : ((Chan.wire q h, true, m) : Ev)
      ∈ Sched.childChunk sk (q, h) n (m - sk.wiresBefore h n) := by
    have := wire_mem_childChunk (sk := sk) (q, h) n
      (m - sk.wiresBefore h n)
    rw [show sk.wiresBefore h n + (m - sk.wiresBefore h n) = m from by
      omega] at this
    exact this
  have hblock : ((Chan.wire q h, true, m) : Ev)
      ∈ Sched.scopeBlockE sk (q, h) n :=
    chunk_mem_scopeBlockE hi hwmem
  have hpair : ([((wireIn (q, h), false, n) : Ev),
      ((Chan.wire q h, true, m) : Ev)] : List Ev).Sublist
      (Sched.scopeBlockE sk (q, h) n) := by
    refine prologue_pair hblock ?_
    intro hc
    have := congrArg (fun e : Ev => e.2.1) hc
    simp at this
  unfold Sched.walkEventsE
  exact sublist_flatMap_block hn hpair


-- ==================================================== the census ladder

/-- A prefix agrees with its extension positionally. -/
private theorem prefix_getD {l₁ l₂ : List Nat} (hpre : l₁ <+: l₂)
    {i : Nat} (hi : i < l₁.length) (d : Nat) :
    l₂.getD i d = l₁.getD i d := by
  obtain ⟨t, rfl⟩ := hpre
  rw [List.getD_eq_getElem?_getD, List.getD_eq_getElem?_getD,
    List.getElem?_append_left hi]

/-- The announced stage census is a true-prefix with known kinds (the
predecessor's `peerWalkTraceA_prefix` opening, extracted). -/
theorem stageScopesA_prefix (hwf : sk.wellFormed = true)
    (p : Party) (tr : List MObs) {h : Nat} (hh : h < sk.rootH) :
    (stageScopesA (aviewOf sk p tr) h).1 <+: sk.stageScopes h
      ∧ ∀ u ∈ (stageScopesA (aviewOf sk p tr) h).1,
          (aviewOf sk p tr).kind? u = some ((sk.scope u).kind) := by
  unfold stageScopesA
  by_cases htop : (h + 1 == (aviewOf sk p tr).rootH) = true
  · rw [if_pos htop]
    have htop' : h + 1 = sk.rootH := beq_iff_eq.mp htop
    constructor
    · show [0] <+: _
      unfold Skel.stageScopes
      rw [htop', Sched.wf_root_stage hwf]
      exact List.prefix_refl _
    · intro u hu
      have hu' : u ∈ [(0 : Nat)] := hu
      rw [List.mem_singleton] at hu'
      subst hu'
      show (aviewOf sk p tr).kind? 0 = _
      rw [AView.kind?, if_pos (show ((0 : Nat) == 0) = true from rfl),
        wf_root_kind hwf]
  · rw [if_neg htop,
      if_neg (show ¬ ((aviewOf sk p tr).rootH < h + 1) from by
        have hne : h + 1 ≠ sk.rootH := fun hc =>
          htop (beq_iff_eq.mpr hc)
        show ¬ (sk.rootH < h + 1)
        omega)]
    have hsteps : sk.rootH - ((aviewOf sk p tr).rootH - (h + 1))
        = h + 1 := by
      show sk.rootH - (sk.rootH - (h + 1)) = h + 1
      omega
    have hlvl := levelA_spec hwf p tr ((aviewOf sk p tr).rootH - (h + 1))
      (by show sk.rootH - (h + 1) ≤ sk.rootH; omega)
    rw [hsteps] at hlvl
    exact ⟨hlvl.1, hlvl.2.1⟩

/-- One census descent step: below the top, a stage's announced census
is the collect pass over the stage above's. -/
private theorem stageScopesA_succ (p : Party) (tr : List MObs)
    {h : Nat} (hh : h + 1 < sk.rootH) :
    (stageScopesA (aviewOf sk p tr) h).1
      = (levelA.collect (aviewOf sk p tr)
          (stageScopesA (aviewOf sk p tr) (h + 1)).1).1 := by
  have hne1 : ¬ ((h + 1 == (aviewOf sk p tr).rootH) = true) := by
    show ¬ ((h + 1 == sk.rootH) = true)
    simp only [beq_iff_eq]
    omega
  have hlt1 : ¬ ((aviewOf sk p tr).rootH < h + 1) := by
    show ¬ (sk.rootH < h + 1)
    omega
  have hlhs : (stageScopesA (aviewOf sk p tr) h).1
      = (levelA (aviewOf sk p tr) (sk.rootH - (h + 1))).1 := by
    unfold stageScopesA
    rw [if_neg hne1, if_neg hlt1]
    rfl
  have hstep : sk.rootH - (h + 1) = (sk.rootH - (h + 2)) + 1 := by
    omega
  have hshape : levelA (aviewOf sk p tr) ((sk.rootH - (h + 2)) + 1)
      = ((levelA.collect (aviewOf sk p tr)
            (levelA (aviewOf sk p tr) (sk.rootH - (h + 2))).1).1,
         (levelA (aviewOf sk p tr) (sk.rootH - (h + 2))).2
          && (levelA.collect (aviewOf sk p tr)
              (levelA (aviewOf sk p tr) (sk.rootH - (h + 2))).1).2) := by
    rw [levelA]
  have hrhs : (stageScopesA (aviewOf sk p tr) (h + 1)).1
      = (levelA (aviewOf sk p tr) (sk.rootH - (h + 2))).1 := by
    unfold stageScopesA
    by_cases htop : (h + 1 + 1 == (aviewOf sk p tr).rootH) = true
    · rw [if_pos htop]
      have htop' : h + 2 = sk.rootH := beq_iff_eq.mp htop
      rw [show sk.rootH - (h + 2) = 0 from by omega]
      rfl
    · rw [if_neg htop,
        if_neg (show ¬ ((aviewOf sk p tr).rootH < h + 1 + 1) from by
          have : h + 2 ≠ sk.rootH := fun hc => htop (beq_iff_eq.mpr hc)
          show ¬ (sk.rootH < h + 2)
          omega)]
      rfl
  rw [hlhs, hstep, hshape, hrhs]

/-- The collect pass reaches past every record-known position: with
kinds known throughout and records announced on the first `n + 1`
entries, the emitted kids cover those entries' kids as a prefix. -/
private theorem collect_reach (hwf : sk.wellFormed = true)
    {p : Party} {tr : List MObs} :
    ∀ (l : List Nat) (n : Nat), n < l.length →
      (∀ u ∈ l, u < sk.scopes.length
        ∧ (aviewOf sk p tr).kind? u = some ((sk.scope u).kind)) →
      (∀ i, i ≤ n → (sk.scope (l.getD i 0)).kind = Kind.D →
        l.getD i 0 ∈ announcedIds sk p tr) →
      ((l.take (n + 1)).flatMap (fun u => (sk.scope u).kids))
        <+: (levelA.collect (aviewOf sk p tr) l).1 := by
  intro l
  induction l with
  | nil =>
      intro n hn
      simp at hn
  | cons u rest ih =>
      intro n hn hkinds hrecs
      obtain ⟨hreal, hkind⟩ := hkinds u (List.mem_cons_self ..)
      have htail : ((rest.take n).flatMap fun v => (sk.scope v).kids)
          <+: (levelA.collect (aviewOf sk p tr) rest).1 := by
        cases n with
        | zero =>
            show ([] : List Nat).flatMap _ <+: _
            exact List.nil_prefix
        | succ n' =>
            refine ih n' (by simpa using hn)
              (fun v hv => hkinds v (List.mem_cons_of_mem _ hv))
              (fun i hi hD => ?_)
            have := hrecs (i + 1) (by omega)
            rw [List.getD_cons_succ] at this
            exact this hD
      rw [List.take_succ_cons, List.flatMap_cons]
      by_cases hD : (sk.scope u).kind = Kind.D
      · have hann : u ∈ announcedIds sk p tr := by
          have := hrecs 0 (by omega)
          rw [List.getD_cons_zero] at this
          exact this hD
        have hrec : (aviewOf sk p tr).rec? u = some (sk.scope u) := by
          rw [rec?_aviewOf, if_pos hann]
        have hshape : (levelA.collect (aviewOf sk p tr) (u :: rest)).1
            = (sk.scope u).kids
              ++ (levelA.collect (aviewOf sk p tr) rest).1 := by
          rcases hc : levelA.collect (aviewOf sk p tr) rest
            with ⟨items, comp⟩
          rw [levelA.collect, if_pos (by rw [hkind, hD]; rfl), hrec, hc]
        rw [hshape]
        obtain ⟨t, ht⟩ := htail
        exact ⟨t, by rw [List.append_assoc, ht]⟩
      · have hshape : (levelA.collect (aviewOf sk p tr) (u :: rest)).1
            = (levelA.collect (aviewOf sk p tr) rest).1 := by
          rw [levelA.collect, if_neg (by
            rw [hkind]
            simp only [beq_iff_eq, Option.some.injEq]
            exact fun hc => hD hc)]
        rw [hshape, (wf_scope_nonD hwf hreal hD).1, List.nil_append]
        exact htail

/-- The census ladder (the minting lemma's core): at the stuck drained
wall, a stage's block-`k` prologue receive scheduled below the wall
forces the announced census to cover block `k` — the stage scopes
through `k` are announced, and (on the party's own stages, where the
input stream is the peer's) so are their kid listings.

The τ-recursion climbs one stage per rung: the prologue's E1 send is
the stage above's wire seq `k`, which sits trace-above ITS parent
block's prologue — the BFS ancestors, walked by strong induction on
τ. -/
theorem census_reach {s : MState} {N : Nat} (W : Wall sk s N)
    (p : Party) :
    ∀ (τb h k : Nat), h < sk.rootH → k < sk.stageLen h →
      ((Chan.wire (stageParty h).other (h + 1), false, k) : Ev)
        ∈ scheduleE sk →
      evIdx ((Chan.wire (stageParty h).other (h + 1), false, k) : Ev)
        (scheduleE sk) < N →
      evIdx ((Chan.wire (stageParty h).other (h + 1), false, k) : Ev)
        (scheduleE sk) < τb →
      k < (stageScopesA (aviewOf sk p (s.hist p)) h).1.length
        ∧ (∀ j, j ≤ k →
            sk.stageScope h j ∈ announcedIds sk p (s.hist p))
        ∧ (stageParty h = p → ∀ j, j ≤ k →
            ∀ v ∈ (sk.scope (sk.stageScope h j)).kids,
              v ∈ announcedIds sk p (s.hist p)) := by
  intro τb
  induction τb with
  | zero =>
      intro h k _ _ _ _ hb
      omega
  | succ τb ih =>
      intro h k hh hk hrmem hrN hrb
      -- hop A: the prologue's own frame send, τ-below
      obtain ⟨hsA, hτA⟩ := tau_e1 W.wf hrmem
      by_cases htop : h + 1 = sk.rootH
      · -- the top stage: rule 1 supplies the root record (and, on the
        -- initiator's own top stage, the root's kid listing)
        have hk0 : k = 0 := by
          have hlen : sk.stageLen h = 1 := by
            unfold Skel.stageLen Skel.stageScopes
            rw [htop, Sched.wf_root_stage W.wf]
            rfl
          omega
        subst hk0
        have hs0 : sk.stageScope h 0 = 0 := by
          unfold Skel.stageScope Skel.stageScopes
          rw [htop, Sched.wf_root_stage W.wf]
          rfl
        -- the delivery currency, by stream side
        have hdel : 0 < deliveredCount (s.hist p) sk.rootH := by
          by_cases hpp : stageParty h = p
          · -- the peer's reply/opening arrives on the peer stream
            refine W.root_delivered p ?_ ?_
            · rw [show p.other = (stageParty h).other from by
                rw [hpp], ← htop]
              exact hsA
            · rw [show p.other = (stageParty h).other from by
                rw [hpp], ← htop]
              have := hτA
              omega
          · -- own stream: hop through the responder opening's receive
            have hpo : stageParty h = p.other := by
              cases hq : stageParty h <;> cases hp : p <;>
                first
                  | rfl
                  | (exact absurd (hq ▸ hp ▸ rfl) hpp)
                  | (rw [hq, hp] at hpp; exact absurd rfl hpp)
            -- the top stage is the initiator's: h = rootH - 1 is odd
            have hI : stageParty h = Party.I := by
              have hev : sk.rootH % 2 = 0 := (wf_rootH W.wf).1
              have h2 : 2 ≤ sk.rootH := (wf_rootH W.wf).2
              unfold stageParty
              rw [if_pos (by simp; omega)]
            have hpR : p = Party.R := by
              rw [hI] at hpo
              cases p
              · exact absurd hpo.symm (by decide)
              · rfl
            -- sA = (wire R rootH, true, 0) sits in ropen at position 1
            have hIo : Party.I.other = Party.R := rfl
            rw [hI, hIo, htop] at hsA hτA hrN
            have hpair : ([((Chan.wire Party.I sk.rootH, false, 0) : Ev),
                ((Chan.wire Party.R sk.rootH, true, 0) : Ev)] :
                  List Ev).Sublist (Sched.ropenEvents sk) := by
              unfold Sched.ropenEvents
              exact List.cons_sublist_cons.mpr
                (List.singleton_sublist.mpr (List.mem_cons_self ..))
            obtain ⟨hr'mem, hτ'⟩ := tau_prior W.wf W.m0
              (Sched.fixed_mem_procsE sk).2.1 hpair
            obtain ⟨hs'mem, hτ''⟩ := tau_e1 W.wf hr'mem
            refine W.root_delivered p ?_ ?_
            · rw [show p.other = Party.I from by rw [hpR]; rfl]
              exact hs'mem
            · rw [show p.other = Party.I from by rw [hpR]; rfl]
              omega
        refine ⟨?_, ?_, ?_⟩
        · -- census: the top branch is the literal root singleton
          unfold stageScopesA
          rw [if_pos (show (h + 1 == (aviewOf sk p (s.hist p)).rootH)
            = true from by
              show (h + 1 == sk.rootH) = true
              simp [htop])]
          simp
        · intro j hj
          have hj0 : j = 0 := by omega
          subst hj0
          rw [hs0]
          exact announced_root hdel
        · intro hpp j hj v hv
          have hj0 : j = 0 := by omega
          subst hj0
          rw [hs0] at hv
          -- the guarded branch is the initiator's own top stage
          have hI : stageParty h = Party.I := by
            have hev : sk.rootH % 2 = 0 := (wf_rootH W.wf).1
            have h2 : 2 ≤ sk.rootH := (wf_rootH W.wf).2
            unfold stageParty
            rw [if_pos (by simp; omega)]
          have hpI : p = Party.I := by rw [← hpp, hI]
          subst hpI
          exact announced_root_kids hdel hv
      · -- generic rung: locate the frame in the stage above and recurse
        have hh1 : h + 1 < sk.rootH := by omega
        have hq₁ : (stageParty (h + 1), h + 1) ∈ sk.walkKeys :=
          stageParty_mem_walkKeys W.wf hh1
        have htot : sk.wiresBefore (h + 1) (sk.stageLen (h + 1))
            = sk.stageLen h := by
          have := Sched.wiresBefore_total (sk := sk) W.wf
            (show 1 ≤ h + 1 by omega) hh1
          simpa using this
        have hkw : k < sk.wiresBefore (h + 1) (sk.stageLen (h + 1)) := by
          omega
        -- rewrite the frame send onto the stage above's output
        have hchan : (stageParty h).other = stageParty (h + 1) :=
          (stageParty_succ h).symm
        rw [hchan] at hrmem hrN hrb hsA hτA
        obtain ⟨n, hn, hle, hlt, hpairs⟩ :=
          wire_send_locate W.wf hq₁ hkw
        obtain ⟨hr'mem, hτ'⟩ := tau_prior W.wf W.m0
          (Sched.walkEventsE_mem_procsE sk W.wf hq₁) hpairs
        have hτchain : evIdx ((wireIn (stageParty (h + 1), h + 1),
            false, n) : Ev) (scheduleE sk)
            < evIdx ((Chan.wire (stageParty (h + 1)) (h + 1), false, k)
                : Ev) (scheduleE sk) := by
          have h2 := hτ'
          omega
        have hrin : wireIn (stageParty (h + 1), h + 1)
            = Chan.wire (stageParty (h + 1)).other (h + 1 + 1) := rfl
        have hIH := ih (h + 1) n hh1 hn
          (by rwa [hrin] at hr'mem)
          (by rw [← hrin]; omega)
          (by rw [← hrin]; omega)
        obtain ⟨hlen₁, hsc₁, hkid₁⟩ := hIH
        -- level-(h+1) records, by which side owns the minting stream
        have hscopes : ∀ j, j ≤ k →
            sk.stageScope h j ∈ announcedIds sk p (s.hist p) := by
          by_cases hpp : stageParty h = p
          · -- peer stream: rule-2 about-scopes, direct harvest
            have hpk₂ : (p.other, h + 1) ∈ sk.walkKeys := by
              have : stageParty (h + 1) = p.other := by
                rw [← hchan, hpp]
              rwa [this] at hq₁
            have hdelk : k < deliveredCount (s.hist p) (h + 1) := by
              refine W.delivered_of_send p
                (mem_allChans_wireOut hpk₂) ?_ ?_
              · rw [show p.other = stageParty (h + 1) from by
                  rw [← hchan, hpp]]
                exact hsA
              · rw [show p.other = stageParty (h + 1) from by
                  rw [← hchan, hpp]]
                omega
            intro j hj
            have hj2 : j < (sk.scopesAt (h + 1)).length := by
              show j < sk.stageLen h
              omega
            have := (announced_of_delivered
              (sk := sk) (p := p) (tr := s.hist p)
              (mem_peerMintHeights W.wf p hpk₂)
              (show h + 1 ≠ 0 by omega)
              (show j < deliveredCount (s.hist p) (h + 1) by omega)
              hj2).1
            exact this
          · -- own stream: the stage above's kid listings carry them
            have hpo : stageParty (h + 1) = p := by
              rw [← hchan]
              cases hq : stageParty h <;> cases hp : p <;>
                first
                  | rfl
                  | (rw [hq, hp] at hpp; exact absurd rfl hpp)
            intro j hj
            have hjw : j < sk.wiresBefore (h + 1) (n + 1) := by omega
            obtain ⟨nⱼ, hnⱼ, hleⱼ, hltⱼ⟩ := exists_parent_block
              (show j < sk.wiresBefore (h + 1) (sk.stageLen (h + 1))
                from by omega)
            have hnn : nⱼ ≤ n := by
              by_contra hgt
              have := Sched.wiresBefore_mono (sk := sk) (h + 1)
                (show n + 1 ≤ nⱼ by omega)
              omega
            have hkids : (sk.scope (sk.stageScope (h + 1) nⱼ)).kids.length
                = sk.nChildren (h + 1) (sk.stageScope (h + 1) nⱼ) := by
              unfold Skel.nChildren
              rw [if_neg (by simp)]
            have hi : j - sk.wiresBefore (h + 1) nⱼ
                < (sk.scope (sk.stageScope (h + 1) nⱼ)).kids.length := by
              have := Sched.wiresBefore_succ sk hnⱼ
              omega
            have hkid := stageScope_kid W.wf (show 1 ≤ h + 1 by omega)
              hh1 hnⱼ hi
            rw [show sk.wiresBefore (h + 1) nⱼ
              + (j - sk.wiresBefore (h + 1) nⱼ) = j from by omega]
              at hkid
            have hvmem : (sk.scope (sk.stageScope (h + 1) nⱼ)).kids.getD
                (j - sk.wiresBefore (h + 1) nⱼ) 0
                ∈ (sk.scope (sk.stageScope (h + 1) nⱼ)).kids := by
              rw [List.getD_eq_getElem?_getD,
                List.getElem?_eq_getElem hi]
              exact List.getElem_mem hi
            have := hkid₁ hpo nⱼ hnn _ hvmem
            show sk.stageScope h j ∈ announcedIds sk p (s.hist p)
            unfold Skel.stageScope Skel.stageScopes
            rw [hkid]
            exact this
        refine ⟨?_, hscopes, ?_⟩
        · -- census length: collect over the stage above's census
          rw [stageScopesA_succ p (s.hist p) hh1]
          have hpre := stageScopesA_prefix W.wf p (s.hist p) hh1
          have hkinds : ∀ u ∈ (stageScopesA (aviewOf sk p (s.hist p))
              (h + 1)).1, u < sk.scopes.length
              ∧ (aviewOf sk p (s.hist p)).kind? u
                = some ((sk.scope u).kind) := by
            intro u hu
            refine ⟨?_, hpre.2 u hu⟩
            have hmem := hpre.1.sublist.mem hu
            exact (mem_scopesAt hmem).1
          have hrecs : ∀ i, i ≤ n →
              (sk.scope ((stageScopesA (aviewOf sk p (s.hist p))
                (h + 1)).1.getD i 0)).kind = Kind.D →
              (stageScopesA (aviewOf sk p (s.hist p)) (h + 1)).1.getD i 0
                ∈ announcedIds sk p (s.hist p) := by
            intro i hi hD
            have hgd : (stageScopesA (aviewOf sk p (s.hist p))
                (h + 1)).1.getD i 0 = sk.stageScope (h + 1) i := by
              have := prefix_getD hpre.1 (show i
                < (stageScopesA (aviewOf sk p (s.hist p)) (h + 1)).1.length
                from by omega) 0
              rw [← this]
              rfl
            rw [hgd]
            exact hsc₁ i hi
          have hcov := collect_reach W.wf
            (stageScopesA (aviewOf sk p (s.hist p)) (h + 1)).1 n
            (by omega) hkinds hrecs
          have htake : (stageScopesA (aviewOf sk p (s.hist p))
              (h + 1)).1.take (n + 1)
              = (sk.stageScopes (h + 1)).take (n + 1) := by
            obtain ⟨t, ht⟩ := hpre.1
            rw [← ht, List.take_append_of_le_length (by omega)]
          obtain ⟨rest₂, -, hflen⟩ := bfs_split W.wf
            (show 1 ≤ h + 1 by omega) hh1 (n + 1) (by omega)
          have hlenle := hcov.length_le
          rw [htake] at hlenle
          have hkids2 : (((sk.stageScopes (h + 1)).take (n + 1)).flatMap
              (fun u => (sk.scope u).kids)).length
              = sk.wiresBefore (h + 1) (n + 1) := by
            have : sk.stageScopes (h + 1) = sk.scopesAt (h + 1 + 1) := rfl
            exact hflen
          omega
        · -- the kids-half: available exactly on the party's own stages
          intro hpp j hj v hv
          have hpk₂ : (p.other, h + 1) ∈ sk.walkKeys := by
            have : stageParty (h + 1) = p.other := by
              rw [← hchan, hpp]
            rwa [this] at hq₁
          have hdelk : k < deliveredCount (s.hist p) (h + 1) := by
            refine W.delivered_of_send p
              (mem_allChans_wireOut hpk₂) ?_ ?_
            · rw [show p.other = stageParty (h + 1) from by
                rw [← hchan, hpp]]
              exact hsA
            · rw [show p.other = stageParty (h + 1) from by
                rw [← hchan, hpp]]
              omega
          have hj2 : j < (sk.scopesAt (h + 1)).length := by
            show j < sk.stageLen h
            omega
          have := (announced_of_delivered
            (sk := sk) (p := p) (tr := s.hist p)
            (mem_peerMintHeights W.wf p hpk₂)
            (show h + 1 ≠ 0 by omega)
            (show j < deliveredCount (s.hist p) (h + 1) by omega)
            hj2).2
          exact this v hv


-- =================================================== the walk family
-- Block-level membership in the announced walk layout: the chunk loop
-- covers every true chunk whose D-kid records are announced, and the
-- whole block completes when all of them are.

/-- The chunk loop completes once every remaining D kid's record is
announced (the `ok` flag half of the transcription; exactness then
comes from `peerBlockA_spec`). -/
private theorem chunksA_ok (hwf : sk.wellFormed = true)
    {p : Party} {tr : List MObs} (q : Party) {h k : Nat}
    (hk : k < sk.stageLen h)
    (hann : sk.stageScope h k ∈ announcedIds sk p tr) (wires : Nat) :
    ∀ (fuel i w d qacc : Nat),
      i ≤ (sk.scope (sk.stageScope h k)).kids.length →
      (sk.scope (sk.stageScope h k)).kids.length - i < fuel →
      (∀ j, i ≤ j → sk.childIsD h (sk.stageScope h k) j = true →
        (sk.scope (sk.stageScope h k)).kids.getD j 0
          ∈ announcedIds sk p tr) →
      (peerBlockA.chunks (aviewOf sk p tr) q h wires
          (sk.scope (sk.stageScope h k))
          (if (h == 0) = true then
            (sk.scope (sk.stageScope h k)).leafReqs
          else (sk.scope (sk.stageScope h k)).kids.length)
          i w d qacc fuel).2.2 = true := by
  intro fuel
  induction fuel with
  | zero =>
      intro i w d qacc hi hfuel
      omega
  | succ fuel ih =>
      intro i w d qacc hi hfuel hrecs
      by_cases h0 : (h == 0) = true
      · rw [peerBlockA.chunks, if_pos h0]
      · by_cases hin : i = (sk.scope (sk.stageScope h k)).kids.length
        · have hnone : (sk.scope (sk.stageScope h k)).kids[i]? = none := by
            rw [hin]
            exact List.getElem?_eq_none (Nat.le_refl _)
          rw [peerBlockA.chunks, if_neg h0]
          simp only [hnone]
        · have hilt : i < (sk.scope (sk.stageScope h k)).kids.length := by
            omega
          have hget : (sk.scope (sk.stageScope h k)).kids[i]?
              = some ((sk.scope (sk.stageScope h k)).kids[i]) :=
            List.getElem?_eq_getElem hilt
          have hvmem : (sk.scope (sk.stageScope h k)).kids[i]
              ∈ (sk.scope (sk.stageScope h k)).kids :=
            List.getElem_mem hilt
          have hkind := kind?_aviewOf_of_kid (p := p) (tr := tr) hwf hann
            hvmem
          cases hkindv : (sk.scope
              ((sk.scope (sk.stageScope h k)).kids[i])).kind with
          | R =>
              have hrec := ih (i + 1) (w + 1) d qacc (by omega) (by omega)
                (fun j hj hD => hrecs j (by omega) hD)
              rcases hcall : peerBlockA.chunks (aviewOf sk p tr) q h wires
                  (sk.scope (sk.stageScope h k))
                  (if (h == 0) = true then
                    (sk.scope (sk.stageScope h k)).leafReqs
                  else (sk.scope (sk.stageScope h k)).kids.length)
                  (i + 1) (w + 1) d qacc fuel with ⟨evs, cnts, ok⟩
              rw [hcall] at hrec
              rw [peerBlockA.chunks, if_neg h0]
              simp only [hget, hkind, hkindv, hcall]
              exact hrec
          | D =>
              have hD : sk.childIsD h (sk.stageScope h k) i = true := by
                unfold Skel.childIsD
                rw [if_neg h0, hget]
                show ((sk.scope
                  ((sk.scope (sk.stageScope h k)).kids[i])).kind
                    == Kind.D) = true
                rw [hkindv]
                rfl
              have hannv := hrecs i (Nat.le_refl _) hD
              rw [List.getD_eq_getElem?_getD,
                List.getElem?_eq_getElem hilt, Option.getD_some] at hannv
              have hrecv : (aviewOf sk p tr).rec?
                  ((sk.scope (sk.stageScope h k)).kids[i])
                  = some (sk.scope
                      ((sk.scope (sk.stageScope h k)).kids[i])) := by
                rw [rec?_aviewOf, if_pos hannv]
              have hrec := ih (i + 1) (w + 1) (d + 1)
                (qacc + (if ((h : Nat) == 1) = true then
                  (sk.scope
                    ((sk.scope (sk.stageScope h k)).kids[i])).leafReqs
                else (sk.scope
                  ((sk.scope (sk.stageScope h k)).kids[i])).kids.length))
                (by omega) (by omega)
                (fun j hj hDj => hrecs j (by omega) hDj)
              rcases hcall : peerBlockA.chunks (aviewOf sk p tr) q h wires
                  (sk.scope (sk.stageScope h k))
                  (if (h == 0) = true then
                    (sk.scope (sk.stageScope h k)).leafReqs
                  else (sk.scope (sk.stageScope h k)).kids.length)
                  (i + 1) (w + 1) (d + 1)
                  (qacc + (if ((h : Nat) == 1) = true then
                    (sk.scope
                      ((sk.scope (sk.stageScope h k)).kids[i])).leafReqs
                  else (sk.scope
                    ((sk.scope (sk.stageScope h k)).kids[i])).kids.length))
                  fuel with ⟨evs, cnts, ok⟩
              rw [hcall] at hrec
              rw [peerBlockA.chunks, if_neg h0]
              simp only [hget, hkind, hkindv, qCountA, hrecv, Option.map,
                hcall]
              exact hrec


/-- The chunk loop covers a target chunk: with the counters aligned and
the D-kid records announced strictly below the target, the target's
wire is emitted; with the target's own record too (when disputed), its
whole true chunk is. -/
private theorem chunksA_covers (hwf : sk.wellFormed = true)
    {p : Party} {tr : List MObs} (q : Party) {h k : Nat}
    (hk : k < sk.stageLen h) (h0 : (h == 0) = false)
    (hann : sk.stageScope h k ∈ announcedIds sk p tr) (wires : Nat)
    (it : Nat) (hit : it < (sk.scope (sk.stageScope h k)).kids.length) :
    ∀ (fuel i w d qacc : Nat),
      i ≤ it →
      (sk.scope (sk.stageScope h k)).kids.length - i < fuel →
      w = sk.wiresBefore h k + i →
      d = sk.dsBefore h k
        + ((List.range i).filter
            (sk.childIsD h (sk.stageScope h k))).length →
      qacc = sk.qsBefore h k
        + ((List.range i).map
            (sk.qCount h (sk.stageScope h k))).sum →
      (∀ j, i ≤ j → j < it → sk.childIsD h (sk.stageScope h k) j = true →
        (sk.scope (sk.stageScope h k)).kids.getD j 0
          ∈ announcedIds sk p tr) →
      (((Chan.wire q h, true, sk.wiresBefore h k + it) : Ev)
          ∈ (peerBlockA.chunks (aviewOf sk p tr) q h wires
              (sk.scope (sk.stageScope h k))
              (if (h == 0) = true then
                (sk.scope (sk.stageScope h k)).leafReqs
              else (sk.scope (sk.stageScope h k)).kids.length)
              i w d qacc fuel).1)
      ∧ ((sk.childIsD h (sk.stageScope h k) it = true →
            (sk.scope (sk.stageScope h k)).kids.getD it 0
              ∈ announcedIds sk p tr) →
          ∀ x ∈ Sched.childChunk sk (q, h) k it,
            x ∈ (peerBlockA.chunks (aviewOf sk p tr) q h wires
                (sk.scope (sk.stageScope h k))
                (if (h == 0) = true then
                  (sk.scope (sk.stageScope h k)).leafReqs
                else (sk.scope (sk.stageScope h k)).kids.length)
                i w d qacc fuel).1) := by
  have h0' : h ≠ 0 := by simpa using h0
  intro fuel
  induction fuel with
  | zero =>
      intro i w d qacc hi hfuel
      omega
  | succ fuel ih =>
      intro i w d qacc hi hfuel hw hd hq hrecs
      have hilt : i < (sk.scope (sk.stageScope h k)).kids.length := by
        omega
      have hget : (sk.scope (sk.stageScope h k)).kids[i]?
          = some ((sk.scope (sk.stageScope h k)).kids[i]) :=
        List.getElem?_eq_getElem hilt
      have hvmem : (sk.scope (sk.stageScope h k)).kids[i]
          ∈ (sk.scope (sk.stageScope h k)).kids :=
        List.getElem_mem hilt
      have hkind := kind?_aviewOf_of_kid (p := p) (tr := tr) hwf hann
        hvmem
      have hureal : sk.stageScope h k < sk.scopes.length :=
        stageScope_lt_scopes sk hk
      have hheight : (sk.scope
          ((sk.scope (sk.stageScope h k)).kids[i])).height = h := by
        have hkf := (Sched.wf_kid_facts hwf hureal _ hvmem).2
        have hsh := Sched.stageScope_height sk (h := h) (k := k) hk
        omega
      -- range prefix-sum steps
      have hdrank : ((List.range (i + 1)).filter
          (sk.childIsD h (sk.stageScope h k))).length
          = ((List.range i).filter
              (sk.childIsD h (sk.stageScope h k))).length
            + (if sk.childIsD h (sk.stageScope h k) i then 1 else 0) := by
        rw [List.range_succ, List.filter_append, List.length_append]
        congr 1
        by_cases hDi : sk.childIsD h (sk.stageScope h k) i = true
        · rw [List.filter_cons, if_pos hDi, if_pos hDi]
          rfl
        · rw [List.filter_cons, if_neg (by simpa using hDi),
            if_neg (by simpa using hDi)]
          rfl
      have hqsum : ((List.range (i + 1)).map
          (sk.qCount h (sk.stageScope h k))).sum
          = ((List.range i).map
              (sk.qCount h (sk.stageScope h k))).sum
            + sk.qCount h (sk.stageScope h k) i := by
        rw [List.range_succ, List.map_append, List.sum_append]
        rfl
      cases hkindv : (sk.scope
          ((sk.scope (sk.stageScope h k)).kids[i])).kind with
      | R =>
          have hDi : sk.childIsD h (sk.stageScope h k) i = false := by
            unfold Skel.childIsD
            rw [if_neg (by simpa using h0'), hget]
            show ((sk.scope
              ((sk.scope (sk.stageScope h k)).kids[i])).kind
                == Kind.D) = false
            rw [hkindv]
            rfl
          have hq0 : sk.qCount h (sk.stageScope h k) i = 0 := by
            unfold Skel.qCount
            rw [if_pos (by simp [hDi])]
          rcases hcall : peerBlockA.chunks (aviewOf sk p tr) q h wires
              (sk.scope (sk.stageScope h k))
              (if (h == 0) = true then
                (sk.scope (sk.stageScope h k)).leafReqs
              else (sk.scope (sk.stageScope h k)).kids.length)
              (i + 1) (w + 1) d qacc fuel with ⟨evs, cnts, ok⟩
          have hstep : peerBlockA.chunks (aviewOf sk p tr) q h wires
              (sk.scope (sk.stageScope h k))
              (if (h == 0) = true then
                (sk.scope (sk.stageScope h k)).leafReqs
              else (sk.scope (sk.stageScope h k)).kids.length)
              i w d qacc (fuel + 1)
              = ((Chan.wire q h, true, w) :: evs, cnts, ok) := by
            rw [peerBlockA.chunks, if_neg (by simpa using h0')]
            simp only [hget, hkind, hkindv, hcall]
          by_cases hii : i = it
          · -- the target is an R chunk: its wire is the whole chunk
            subst hii
            rw [hstep]
            constructor
            · rw [hw]
              exact List.mem_cons_self ..
            · intro _ x hx
              have hcc : Sched.childChunk sk (q, h) k i
                  = [((Chan.wire q h, true,
                      sk.wiresBefore h k + i) : Ev)] := by
                unfold Sched.childChunk
                rw [if_neg (by simp [hDi])]
                rfl
              rw [hcc, List.mem_singleton] at hx
              subst hx
              rw [hw]
              exact List.mem_cons_self ..
          · have hlt : i < it := by omega
            have hrec := ih (i + 1) (w + 1) d qacc (by omega) (by omega)
              (by omega)
              (by rw [hd, hdrank, hDi]; simp)
              (by rw [hq, hqsum, hq0]; omega)
              (fun j hj hjt hD => hrecs j (by omega) hjt hD)
            rw [hcall] at hrec
            rw [hstep]
            exact ⟨List.mem_cons_of_mem _ hrec.1,
              fun hr x hx => List.mem_cons_of_mem _ (hrec.2 hr x hx)⟩
      | D =>
          have hDi : sk.childIsD h (sk.stageScope h k) i = true := by
            unfold Skel.childIsD
            rw [if_neg (by simpa using h0'), hget]
            show ((sk.scope
              ((sk.scope (sk.stageScope h k)).kids[i])).kind
                == Kind.D) = true
            rw [hkindv]
            rfl
          have hqc : sk.qCount h (sk.stageScope h k) i
              = (if ((h : Nat) == 1) = true then
                  (sk.scope
                    ((sk.scope (sk.stageScope h k)).kids[i])).leafReqs
                else (sk.scope
                  ((sk.scope (sk.stageScope h k)).kids[i])).kids.length)
              := by
            unfold Skel.qCount
            rw [if_neg (by simp [hDi])]
            simp only [hget, hheight]
          by_cases hii : i = it
          · -- the target chunk itself
            subst hii
            constructor
            · -- the wire departs whether or not the record arrived
              cases hrecv : (aviewOf sk p tr).rec?
                  ((sk.scope (sk.stageScope h k)).kids[i]) with
              | none =>
                  have hstep : peerBlockA.chunks (aviewOf sk p tr) q h
                      wires (sk.scope (sk.stageScope h k))
                      (if (h == 0) = true then
                        (sk.scope (sk.stageScope h k)).leafReqs
                      else (sk.scope (sk.stageScope h k)).kids.length)
                      i w d qacc (fuel + 1)
                      = ([(Chan.wire q h, true, w)],
                         (w + 1, d, qacc), false) := by
                    rw [peerBlockA.chunks, if_neg (by simpa using h0')]
                    simp only [hget, hkind, hkindv, qCountA, hrecv,
                      Option.map]
                  rw [hstep, hw]
                  exact List.mem_cons_self ..
              | some sc =>
                  obtain ⟨hsc, -⟩ := rec?_some_inv hrecv
                  rcases hcall : peerBlockA.chunks (aviewOf sk p tr) q h
                      wires (sk.scope (sk.stageScope h k))
                      (if (h == 0) = true then
                        (sk.scope (sk.stageScope h k)).leafReqs
                      else (sk.scope (sk.stageScope h k)).kids.length)
                      (i + 1) (w + 1) (d + 1)
                      (qacc + sk.qCount h (sk.stageScope h k) i)
                      fuel with ⟨evs, cnts, ok⟩
                  have hstep : peerBlockA.chunks (aviewOf sk p tr) q h
                      wires (sk.scope (sk.stageScope h k))
                      (if (h == 0) = true then
                        (sk.scope (sk.stageScope h k)).leafReqs
                      else (sk.scope (sk.stageScope h k)).kids.length)
                      i w d qacc (fuel + 1)
                      = ((Chan.wire q h, true, w)
                          :: (Chan.lower q h, true, d)
                          :: ((List.range (sk.qCount h
                                (sk.stageScope h k) i)).map fun t =>
                              (askedOut (q, h), true, qacc + t))
                          ++ evs,
                         cnts, ok) := by
                    rw [peerBlockA.chunks, if_neg (by simpa using h0')]
                    simp only [hget, hkind, hkindv, qCountA, hrecv,
                      Option.map, hsc, ← hqc, hcall]
                  rw [hstep, hw]
                  exact List.mem_cons_self ..
            · -- with the record, the whole chunk is emitted
              intro hr x hx
              have hannv := hr hDi
              rw [List.getD_eq_getElem?_getD,
                List.getElem?_eq_getElem hilt, Option.getD_some] at hannv
              have hrecv : (aviewOf sk p tr).rec?
                  ((sk.scope (sk.stageScope h k)).kids[i])
                  = some (sk.scope
                      ((sk.scope (sk.stageScope h k)).kids[i])) := by
                rw [rec?_aviewOf, if_pos hannv]
              rcases hcall : peerBlockA.chunks (aviewOf sk p tr) q h
                  wires (sk.scope (sk.stageScope h k))
                  (if (h == 0) = true then
                    (sk.scope (sk.stageScope h k)).leafReqs
                  else (sk.scope (sk.stageScope h k)).kids.length)
                  (i + 1) (w + 1) (d + 1)
                  (qacc + sk.qCount h (sk.stageScope h k) i)
                  fuel with ⟨evs, cnts, ok⟩
              have hstep : peerBlockA.chunks (aviewOf sk p tr) q h
                  wires (sk.scope (sk.stageScope h k))
                  (if (h == 0) = true then
                    (sk.scope (sk.stageScope h k)).leafReqs
                  else (sk.scope (sk.stageScope h k)).kids.length)
                  i w d qacc (fuel + 1)
                  = ((Chan.wire q h, true, w)
                      :: (Chan.lower q h, true, d)
                      :: ((List.range (sk.qCount h
                            (sk.stageScope h k) i)).map fun t =>
                          (askedOut (q, h), true, qacc + t))
                      ++ evs,
                     cnts, ok) := by
                rw [peerBlockA.chunks, if_neg (by simpa using h0')]
                simp only [hget, hkind, hkindv, qCountA, hrecv,
                  Option.map, ← hqc, hcall]
              have hcc : Sched.childChunk sk (q, h) k i
                  = ((Chan.wire q h, true, w) : Ev)
                    :: (Chan.lower q h, true, d)
                    :: ((List.range (sk.qCount h
                          (sk.stageScope h k) i)).map fun t =>
                        (askedOut (q, h), true, qacc + t)) := by
                unfold Sched.childChunk
                rw [if_pos hDi]
                rw [hw, hd, hq]
                rfl
              rw [hcc] at hx
              rw [hstep]
              rcases List.mem_cons.mp hx with rfl | hx2
              · exact List.mem_cons_self ..
              rcases List.mem_cons.mp hx2 with rfl | hx3
              · refine List.mem_cons_of_mem _ ?_
                show _ ∈ (Chan.lower q h, true, d)
                  :: (_ ++ evs)
                exact List.mem_cons_self ..
              · refine List.mem_cons_of_mem _ (List.mem_cons_of_mem _ ?_)
                exact List.mem_append.mpr (.inl hx3)
          · -- below the target: the record is available, recurse
            have hlt : i < it := by omega
            have hannv := hrecs i (Nat.le_refl _) hlt hDi
            rw [List.getD_eq_getElem?_getD,
              List.getElem?_eq_getElem hilt, Option.getD_some] at hannv
            have hrecv : (aviewOf sk p tr).rec?
                ((sk.scope (sk.stageScope h k)).kids[i])
                = some (sk.scope
                    ((sk.scope (sk.stageScope h k)).kids[i])) := by
              rw [rec?_aviewOf, if_pos hannv]
            rcases hcall : peerBlockA.chunks (aviewOf sk p tr) q h
                wires (sk.scope (sk.stageScope h k))
                (if (h == 0) = true then
                  (sk.scope (sk.stageScope h k)).leafReqs
                else (sk.scope (sk.stageScope h k)).kids.length)
                (i + 1) (w + 1) (d + 1)
                (qacc + sk.qCount h (sk.stageScope h k) i)
                fuel with ⟨evs, cnts, ok⟩
            have hstep : peerBlockA.chunks (aviewOf sk p tr) q h
                wires (sk.scope (sk.stageScope h k))
                (if (h == 0) = true then
                  (sk.scope (sk.stageScope h k)).leafReqs
                else (sk.scope (sk.stageScope h k)).kids.length)
                i w d qacc (fuel + 1)
                = ((Chan.wire q h, true, w)
                    :: (Chan.lower q h, true, d)
                    :: ((List.range (sk.qCount h
                          (sk.stageScope h k) i)).map fun t =>
                        (askedOut (q, h), true, qacc + t))
                    ++ evs,
                   cnts, ok) := by
              rw [peerBlockA.chunks, if_neg (by simpa using h0')]
              simp only [hget, hkind, hkindv, qCountA, hrecv,
                Option.map, ← hqc, hcall]
            have hrec := ih (i + 1) (w + 1) (d + 1)
              (qacc + sk.qCount h (sk.stageScope h k) i)
              (by omega) (by omega) (by omega)
              (by rw [hd, hdrank, hDi, if_pos rfl]; omega)
              (by rw [hq, hqsum]; omega)
              (fun j hj hjt hD => hrecs j (by omega) hjt hD)
            rw [hcall] at hrec
            rw [hstep]
            refine ⟨?_, fun hr x hx => ?_⟩
            · refine List.mem_cons_of_mem _ (List.mem_cons_of_mem _ ?_)
              exact List.mem_append.mpr (.inr hrec.1)
            · refine List.mem_cons_of_mem _ (List.mem_cons_of_mem _ ?_)
              exact List.mem_append.mpr (.inr (hrec.2 hr x hx))


/-- The block prologue is laid before any record arrives. -/
private theorem prologue_mem_peerBlockA {p : Party} {tr : List MObs}
    (q : Party) (h k u w d qa : Nat) {e : Ev}
    (he : e = ((Chan.wire q.other (h + 1), false, k) : Ev)
      ∨ e = ((Chan.asked q h, false, k) : Ev)) :
    e ∈ (peerBlockA (aviewOf sk p tr) q h k u w d qa).1 := by
  have hpro : e ∈ [((Chan.wire q.other (h + 1), false, k) : Ev),
      ((Chan.asked q h, false, k) : Ev)] := by
    rcases he with rfl | rfl
    · exact List.mem_cons_self ..
    · exact List.mem_cons_of_mem _ (List.mem_cons_self ..)
  cases hrec : (aviewOf sk p tr).rec? u with
  | none =>
      have hstep : peerBlockA (aviewOf sk p tr) q h k u w d qa
          = ([(Chan.wire q.other (h + 1), false, k),
              (Chan.asked q h, false, k)], (w, d, qa), false) := by
        simp only [peerBlockA, hrec]
      rw [hstep]
      exact hpro
  | some sc =>
      rcases hcall : peerBlockA.chunks (aviewOf sk p tr) q h w sc
          (if (h == 0) = true then sc.leafReqs else sc.kids.length)
          0 w d qa (sc.kids.length + 1) with ⟨evs, cnts, ok⟩
      obtain ⟨w', rest⟩ := cnts
      obtain ⟨d', q'⟩ := rest
      have hstep : peerBlockA (aviewOf sk p tr) q h k u w d qa
          = ((([(Chan.wire q.other (h + 1), false, k),
              (Chan.asked q h, false, k)] : List Ev)
              ++ evs
              ++ (if ok then [((Chan.upper q h, true, k) : Ev)] else [])),
             (w', d', q'), ok) := by
        simp only [peerBlockA, hrec, hcall]
      rw [hstep]
      exact List.mem_append.mpr (.inl (List.mem_append.mpr (.inl hpro)))

/-- A chunk-loop emission is laid once the block's record arrived. -/
private theorem chunkOut_mem_peerBlockA {p : Party} {tr : List MObs}
    (q : Party) {h k : Nat} (hk : k < sk.stageLen h)
    (hann : sk.stageScope h k ∈ announcedIds sk p tr) {w d qa : Nat}
    {e : Ev}
    (he : e ∈ (peerBlockA.chunks (aviewOf sk p tr) q h w
        (sk.scope (sk.stageScope h k))
        (if (h == 0) = true then
          (sk.scope (sk.stageScope h k)).leafReqs
        else (sk.scope (sk.stageScope h k)).kids.length)
        0 w d qa ((sk.scope (sk.stageScope h k)).kids.length + 1)).1) :
    e ∈ (peerBlockA (aviewOf sk p tr) q h k (sk.stageScope h k)
        w d qa).1 := by
  have hrec : (aviewOf sk p tr).rec? (sk.stageScope h k)
      = some (sk.scope (sk.stageScope h k)) := by
    rw [rec?_aviewOf, if_pos hann]
  rcases hcall : peerBlockA.chunks (aviewOf sk p tr) q h w
      (sk.scope (sk.stageScope h k))
      (if (h == 0) = true then
        (sk.scope (sk.stageScope h k)).leafReqs
      else (sk.scope (sk.stageScope h k)).kids.length)
      0 w d qa ((sk.scope (sk.stageScope h k)).kids.length + 1)
      with ⟨evs, cnts, ok⟩
  obtain ⟨w', rest⟩ := cnts
  obtain ⟨d', q'⟩ := rest
  rw [hcall] at he
  have hstep : peerBlockA (aviewOf sk p tr) q h k (sk.stageScope h k)
      w d qa
      = ((([(Chan.wire q.other (h + 1), false, k),
          (Chan.asked q h, false, k)] : List Ev)
          ++ evs
          ++ (if ok then [((Chan.upper q h, true, k) : Ev)] else [])),
         (w', d', q'), ok) := by
    simp only [peerBlockA, hrec, hcall]
  rw [hstep]
  exact List.mem_append.mpr (.inl (List.mem_append.mpr (.inr he)))

/-- The whole block completes once its record and every D kid's are
announced. -/
private theorem peerBlockA_ok (hwf : sk.wellFormed = true)
    {p : Party} {tr : List MObs} (q : Party) {h k : Nat}
    (hk : k < sk.stageLen h)
    (hann : sk.stageScope h k ∈ announcedIds sk p tr)
    (hkids : ∀ j, sk.childIsD h (sk.stageScope h k) j = true →
      (sk.scope (sk.stageScope h k)).kids.getD j 0
        ∈ announcedIds sk p tr) (w d qa : Nat) :
    (peerBlockA (aviewOf sk p tr) q h k (sk.stageScope h k)
        w d qa).2.2 = true := by
  have hrec : (aviewOf sk p tr).rec? (sk.stageScope h k)
      = some (sk.scope (sk.stageScope h k)) := by
    rw [rec?_aviewOf, if_pos hann]
  have hok := chunksA_ok hwf q hk hann w
    ((sk.scope (sk.stageScope h k)).kids.length + 1) 0 w d qa
    (Nat.zero_le _) (by omega) (fun j _ hD => hkids j hD)
  rcases hcall : peerBlockA.chunks (aviewOf sk p tr) q h w
      (sk.scope (sk.stageScope h k))
      (if (h == 0) = true then
        (sk.scope (sk.stageScope h k)).leafReqs
      else (sk.scope (sk.stageScope h k)).kids.length)
      0 w d qa ((sk.scope (sk.stageScope h k)).kids.length + 1)
      with ⟨evs, cnts, ok⟩
  obtain ⟨w', rest⟩ := cnts
  obtain ⟨d', q'⟩ := rest
  rw [hcall] at hok
  have hstep : peerBlockA (aviewOf sk p tr) q h k (sk.stageScope h k)
      w d qa
      = ((([(Chan.wire q.other (h + 1), false, k),
          (Chan.asked q h, false, k)] : List Ev)
          ++ evs
          ++ (if ok then [((Chan.upper q h, true, k) : Ev)] else [])),
         (w', d', q'), ok) := by
    simp only [peerBlockA, hrec, hcall]
  rw [hstep]
  exact hok

/-- The parent summary is laid on a completed block. -/
private theorem parent_mem_peerBlockA {p : Party} {tr : List MObs}
    (q : Party) {h k : Nat}
    (hann : sk.stageScope h k ∈ announcedIds sk p tr) {w d qa : Nat}
    (hok : (peerBlockA (aviewOf sk p tr) q h k (sk.stageScope h k)
        w d qa).2.2 = true) :
    ((Chan.upper q h, true, k) : Ev)
      ∈ (peerBlockA (aviewOf sk p tr) q h k (sk.stageScope h k)
          w d qa).1 := by
  have hrec : (aviewOf sk p tr).rec? (sk.stageScope h k)
      = some (sk.scope (sk.stageScope h k)) := by
    rw [rec?_aviewOf, if_pos hann]
  rcases hcall : peerBlockA.chunks (aviewOf sk p tr) q h w
      (sk.scope (sk.stageScope h k))
      (if (h == 0) = true then
        (sk.scope (sk.stageScope h k)).leafReqs
      else (sk.scope (sk.stageScope h k)).kids.length)
      0 w d qa ((sk.scope (sk.stageScope h k)).kids.length + 1)
      with ⟨evs, cnts, ok⟩
  obtain ⟨w', rest⟩ := cnts
  obtain ⟨d', q'⟩ := rest
  have hstep : peerBlockA (aviewOf sk p tr) q h k (sk.stageScope h k)
      w d qa
      = ((([(Chan.wire q.other (h + 1), false, k),
          (Chan.asked q h, false, k)] : List Ev)
          ++ evs
          ++ (if ok then [((Chan.upper q h, true, k) : Ev)] else [])),
         (w', d', q'), ok) := by
    simp only [peerBlockA, hrec, hcall]
  rw [hstep] at hok ⊢
  have hoktrue : ok = true := hok
  rw [hoktrue]
  refine List.mem_append.mpr (.inr ?_)
  rw [if_pos rfl]
  exact List.mem_singleton.mpr rfl

/-- The go loop lays a target block's emission once every earlier block
completes: the announced walk trace covers everything the records
reach. -/
private theorem goA_mem (hwf : sk.wellFormed = true)
    {p : Party} {tr : List MObs} {h : Nat} (hh : h < sk.rootH) :
    ∀ (is : List Nat) (k₀ k : Nat) {e : Ev},
      (∀ j, j < is.length → is.getD j 0 = sk.stageScope h (k₀ + j)) →
      k₀ ≤ k → k - k₀ < is.length → k < sk.stageLen h →
      (∀ j, k₀ ≤ j → j < k →
        (peerBlockA (aviewOf sk p tr) p.other h j (sk.stageScope h j)
          (sk.wiresBefore h j) (sk.dsBefore h j)
          (sk.qsBefore h j)).2.2 = true) →
      e ∈ (peerBlockA (aviewOf sk p tr) p.other h k (sk.stageScope h k)
          (sk.wiresBefore h k) (sk.dsBefore h k) (sk.qsBefore h k)).1 →
      e ∈ peerWalkTraceA.go (aviewOf sk p tr) h p.other is k₀
          (sk.wiresBefore h k₀) (sk.dsBefore h k₀)
          (sk.qsBefore h k₀) := by
  intro is
  induction is with
  | nil =>
      intro k₀ k e _ hk₀ hklen
      simp at hklen
  | cons u rest ih =>
      intro k₀ k e hjs hk₀ hklen hkst hoks hmem
      have hu : u = sk.stageScope h k₀ := by
        have := hjs 0 (by simp)
        simpa using this
      rcases hcall : peerBlockA (aviewOf sk p tr) p.other h k₀
          (sk.stageScope h k₀) (sk.wiresBefore h k₀)
          (sk.dsBefore h k₀) (sk.qsBefore h k₀) with ⟨evs, cnts, ok⟩
      obtain ⟨w', rest'⟩ := cnts
      obtain ⟨d', q'⟩ := rest'
      by_cases hkk : k = k₀
      · -- the target block: its emission lands in either branch
        subst hkk
        rw [hcall] at hmem
        have hgo : peerWalkTraceA.go (aviewOf sk p tr) h p.other
            (u :: rest) k (sk.wiresBefore h k) (sk.dsBefore h k)
            (sk.qsBefore h k)
            = if ok then evs ++ peerWalkTraceA.go (aviewOf sk p tr) h
                p.other rest (k + 1) w' d' q'
              else evs := by
          simp [peerWalkTraceA.go, hu, hcall]
        rw [hgo]
        by_cases hok : ok = true
        · rw [if_pos hok]
          exact List.mem_append.mpr (.inl hmem)
        · rw [if_neg hok]
          exact hmem
      · -- an earlier block: it completes exactly and go advances
        have hklt : k₀ < k := by omega
        have hok₀ := hoks k₀ (Nat.le_refl _) hklt
        rw [hcall] at hok₀
        have hoktrue : ok = true := hok₀
        subst hoktrue
        have hk₀st : k₀ < sk.stageLen h := by omega
        obtain ⟨-, hcomp⟩ := peerBlockA_spec (p := p) (tr := tr) hwf
          p.other hh hk₀st
        rw [hcall] at hcomp
        obtain ⟨-, hcnts⟩ := hcomp rfl
        have hcnts' : (w', d', q') = (sk.wiresBefore h (k₀ + 1),
            sk.dsBefore h (k₀ + 1), sk.qsBefore h (k₀ + 1)) := hcnts
        have hgo : peerWalkTraceA.go (aviewOf sk p tr) h p.other
            (u :: rest) k₀ (sk.wiresBefore h k₀) (sk.dsBefore h k₀)
            (sk.qsBefore h k₀)
            = evs ++ peerWalkTraceA.go (aviewOf sk p tr) h p.other rest
                (k₀ + 1) w' d' q' := by
          simp [peerWalkTraceA.go, hu, hcall]
        rw [hgo]
        refine List.mem_append.mpr (.inr ?_)
        rw [show w' = sk.wiresBefore h (k₀ + 1) from by
            injection hcnts' with h1 h2,
          show d' = sk.dsBefore h (k₀ + 1) from by
            injection hcnts' with h1 h2
            injection h2 with h3 h4,
          show q' = sk.qsBefore h (k₀ + 1) from by
            injection hcnts' with h1 h2
            injection h2 with h3 h4]
        refine ih (k₀ + 1) k
          (fun j hj => by
            have := hjs (j + 1) (by simpa using Nat.succ_lt_succ hj)
            rw [List.getD_cons_succ] at this
            rw [this]
            congr 1
            omega)
          (by omega) (by simp at hklen ⊢; omega) hkst
          (fun j hj hjk => hoks j (by omega) hjk) hmem

end StreamingMirror.Mux
