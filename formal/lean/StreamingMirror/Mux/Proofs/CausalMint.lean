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


/-- Embed a chunk-layer sublist into the scope block (under the
prologue and before the parent). -/
private theorem sublist_scopeBlockE_of_chunks {q : Party} {h k : Nat}
    {l : List Ev}
    (hl : l.Sublist ((List.range (sk.nChildren h (sk.stageScope h k))).flatMap
      (Sched.childChunk sk (q, h) k))) :
    l.Sublist (Sched.scopeBlockE sk (q, h) k) := by
  unfold Sched.scopeBlockE Sched.scopeSendsE
  refine List.Sublist.trans ?_ (List.sublist_cons_self ..)
  refine List.Sublist.trans ?_ (List.sublist_cons_self ..)
  refine List.Sublist.trans ?_ (List.sublist_append_left ..)
  rw [← List.flatMap_def]
  exact hl

/-- Pair a chunk's wire head strictly before a later event of the same
block, inside the stage trace. -/
private theorem chunk_pair_walk (hwf : sk.wellFormed = true)
    {q : Party} {h : Nat} (hq : (q, h) ∈ sk.walkKeys) {k : Nat}
    (hk : k < sk.stageLen h) {j : Nat}
    (hj : j < sk.nChildren h (sk.stageScope h k)) {e : Ev}
    (hcases : (∃ i', j < i' ∧ i' < sk.nChildren h (sk.stageScope h k)
        ∧ e ∈ Sched.childChunk sk (q, h) k i')
      ∨ (e ∈ Sched.childChunk sk (q, h) k j
          ∧ e ≠ ((wireOut (q, h), true, sk.wiresBefore h k + j) : Ev))
      ∨ e = ((upperOut (q, h), true, k) : Ev)) :
    ([((wireOut (q, h), true, sk.wiresBefore h k + j) : Ev), e]
        : List Ev).Sublist (Sched.walkEventsE sk (q, h)) := by
  have hwmem := wire_mem_childChunk (sk := sk) (q, h) k j
  have hhead : Sched.childChunk sk (q, h) k j
      = ((wireOut (q, h), true, sk.wiresBefore h k + j) : Ev)
        :: (Sched.childChunk sk (q, h) k j).tail := by
    unfold Sched.childChunk
    by_cases hD : sk.childIsD h (sk.stageScope h k) j = true
    · rw [if_pos hD]
      rfl
    · rw [if_neg hD]
      rfl
  have hblock : ([((wireOut (q, h), true, sk.wiresBefore h k + j) : Ev),
      e] : List Ev).Sublist (Sched.scopeBlockE sk (q, h) k) := by
    rcases hcases with ⟨i', hji, hi'n, hei⟩ | ⟨hej, hne⟩ | hpar
    · exact sublist_scopeBlockE_of_chunks
        (sublist_flatMap_pair hji hi'n hwmem hei)
    · have hej' : e ∈ ((wireOut (q, h), true,
          sk.wiresBefore h k + j) : Ev)
          :: (Sched.childChunk sk (q, h) k j).tail := by
        rw [← hhead]
        exact hej
      rcases pair_of_mem_cons hej' with heq | hsub
      · exact absurd heq hne
      · refine sublist_scopeBlockE_of_chunks
          (sublist_flatMap_block hj ?_)
        rw [hhead]
        exact hsub
    · subst hpar
      have hs : ([((wireOut (q, h), true,
          sk.wiresBefore h k + j) : Ev),
          ((upperOut (q, h), true, k) : Ev)] : List Ev).Sublist
          (Sched.scopeSendsE sk (q, h) k) := by
        show List.Sublist _
          (((List.range (sk.nChildren h (sk.stageScope h k))).map
              (Sched.childChunk sk (q, h) k)).flatten
            ++ [((upperOut (q, h), true, k) : Ev)])
        rw [← List.flatMap_def]
        exact List.Sublist.append
          (sublist_flatMap_block hj (List.singleton_sublist.mpr hwmem))
          (List.Sublist.refl _)
      unfold Sched.scopeBlockE
      refine List.Sublist.trans ?_ (List.sublist_cons_self ..)
      refine List.Sublist.trans ?_ (List.sublist_cons_self ..)
      exact hs
  unfold Sched.walkEventsE
  exact sublist_flatMap_block hk hblock

/-- A D kid's record is announced once its own wire frame's send is
scheduled below the wall (rule 2, kid-position form). -/
private theorem Wall.kid_minted {s : MState} {N : Nat} (W : Wall sk s N)
    (p : Party) {h : Nat} (hpk : (p.other, h) ∈ sk.walkKeys)
    {k j : Nat} (hk : k < sk.stageLen h)
    (hj : j < (sk.scope (sk.stageScope h k)).kids.length)
    (h0 : h ≠ 0)
    (hmem : ((Chan.wire p.other h, true, sk.wiresBefore h k + j) : Ev)
      ∈ scheduleE sk)
    (hτ : evIdx ((Chan.wire p.other h, true, sk.wiresBefore h k + j)
      : Ev) (scheduleE sk) < N) :
    (sk.scope (sk.stageScope h k)).kids.getD j 0
      ∈ announcedIds sk p (s.hist p) := by
  have hh : h < sk.rootH := (walkKeys_stageParty W.wf hpk).2
  have hnc : sk.nChildren h (sk.stageScope h k)
      = (sk.scope (sk.stageScope h k)).kids.length := by
    unfold Skel.nChildren
    rw [if_neg (by simpa using h0)]
  have hlt1 : sk.wiresBefore h k + j < sk.wiresBefore h (k + 1) := by
    have := Sched.wiresBefore_succ sk hk
    omega
  have hlen : sk.wiresBefore h k + j < (sk.scopesAt h).length := by
    have h1 := Sched.wiresBefore_mono (sk := sk) h
      (show k + 1 ≤ sk.stageLen h by omega)
    have h2 := wiresBefore_le_scopesAt W.wf (show 1 ≤ h by omega) hh
      (Nat.le_refl (sk.stageLen h))
    omega
  have hmint := W.minted_of_send p hpk h0 hmem hτ hlen
  have hkid := stageScope_kid W.wf (show 1 ≤ h by omega) hh hk
    (show j < (sk.scope (sk.stageScope h k)).kids.length from hj)
  rw [hkid] at hmint
  exact hmint.1

/-- The leaf stage's chunk pass emits every supply wire at once. -/
private theorem leaf_chunks_mem {p : Party} {tr : List MObs}
    (q : Party) {k : Nat} (hk : k < sk.stageLen 0)
    (hann : sk.stageScope 0 k ∈ announcedIds sk p tr) {i : Nat}
    (hi : i < sk.nChildren 0 (sk.stageScope 0 k)) (d qa : Nat) :
    ((Chan.wire q 0, true, sk.wiresBefore 0 k + i) : Ev)
      ∈ (peerBlockA.chunks (aviewOf sk p tr) q 0 (sk.wiresBefore 0 k)
          (sk.scope (sk.stageScope 0 k))
          (if ((0 : Nat) == 0) = true then
            (sk.scope (sk.stageScope 0 k)).leafReqs
          else (sk.scope (sk.stageScope 0 k)).kids.length)
          0 (sk.wiresBefore 0 k) d qa
          ((sk.scope (sk.stageScope 0 k)).kids.length + 1)).1 := by
  have hz : ((0 : Nat) == 0) = true := rfl
  have hn : sk.nChildren 0 (sk.stageScope 0 k)
      = (sk.scope (sk.stageScope 0 k)).leafReqs := by
    unfold Skel.nChildren
    rw [if_pos hz]
  have hnf : (if ((0 : Nat) == 0) = true then
      (sk.scope (sk.stageScope 0 k)).leafReqs
    else (sk.scope (sk.stageScope 0 k)).kids.length)
      = (sk.scope (sk.stageScope 0 k)).leafReqs := by
    rw [if_pos hz]
  have hstep : peerBlockA.chunks (aviewOf sk p tr) q 0
      (sk.wiresBefore 0 k) (sk.scope (sk.stageScope 0 k))
      (if ((0 : Nat) == 0) = true then
        (sk.scope (sk.stageScope 0 k)).leafReqs
      else (sk.scope (sk.stageScope 0 k)).kids.length)
      0 (sk.wiresBefore 0 k) d qa
      ((sk.scope (sk.stageScope 0 k)).kids.length + 1)
      = ((List.range (sk.scope (sk.stageScope 0 k)).leafReqs).map
          fun j => ((Chan.wire q 0 : Chan), true,
            sk.wiresBefore 0 k + j),
         (sk.wiresBefore 0 k + (sk.scope (sk.stageScope 0 k)).leafReqs,
          d, qa), true) := by
    simp only [peerBlockA.chunks, hnf]
    rw [if_pos hz]
  rw [hstep]
  refine List.mem_map.mpr ⟨i, ?_, rfl⟩
  rw [hn] at hi
  exact List.mem_range.mpr hi


/-- A disputed child names a real kid above the leaf stage. -/
private theorem childIsD_facts {h s i : Nat}
    (hD : sk.childIsD h s i = true) :
    h ≠ 0 ∧ i < (sk.scope s).kids.length := by
  unfold Skel.childIsD at hD
  by_cases h0 : (h == 0) = true
  · rw [if_pos h0] at hD
    cases hD
  · rw [if_neg h0] at hD
    refine ⟨by simpa using h0, ?_⟩
    cases hget : (sk.scope s).kids[i]? with
    | none => rw [hget] at hD; cases hD
    | some v => exact (List.getElem?_eq_some_iff.mp hget).1

/-- The walk minting lemma: every stage-trace event scheduled below the
wall is announced-laid — the census reaches its block, the records its
layout consults were all minted by frame sends strictly τ-below it, and
the announced walk trace therefore contains it. -/
theorem walk_laid {s : MState} {N : Nat} (W : Wall sk s N) (p : Party)
    {h : Nat} (hpk : (p.other, h) ∈ sk.walkKeys) {e : Ev}
    (he : e ∈ Sched.walkEventsE sk (p.other, h))
    (hτ : evIdx e (scheduleE sk) < N) :
    e ∈ peerWalkTraceA (aviewOf sk p (s.hist p)) h := by
  obtain ⟨hsp, hh⟩ := walkKeys_stageParty W.wf hpk
  have hTmem := Sched.walkEventsE_mem_procsE sk W.wf hpk
  have hemem : e ∈ scheduleE sk :=
    (Sched.trace_sublistE sk W.wf W.m0 hTmem).mem he
  unfold Sched.walkEventsE at he
  obtain ⟨k, hkr0, hek⟩ := List.mem_flatMap.mp he
  have hkr : k < sk.stageLen h := List.mem_range.mp hkr0
  -- the block prologue sits at or before e in τ
  have hek' : e ∈ ((wireIn (p.other, h), false, k) : Ev)
      :: (((askedIn (p.other, h), false, k) : Ev)
        :: Sched.scopeSendsE sk (p.other, h) k) := hek
  have hrfacts : ((wireIn (p.other, h), false, k) : Ev) ∈ scheduleE sk
      ∧ evIdx ((wireIn (p.other, h), false, k) : Ev) (scheduleE sk)
          ≤ evIdx e (scheduleE sk) := by
    rcases pair_of_mem_cons hek' with heq | hpair
    · subst heq
      exact ⟨hemem, Nat.le_refl _⟩
    · have hlift : ([((wireIn (p.other, h), false, k) : Ev), e]
          : List Ev).Sublist (Sched.walkEventsE sk (p.other, h)) := by
        unfold Sched.walkEventsE
        exact sublist_flatMap_block hkr hpair
      obtain ⟨hm, hlt⟩ := tau_prior W.wf W.m0 hTmem hlift
      exact ⟨hm, by omega⟩
  -- the census reaches block k
  have hconv : (Chan.wire (stageParty h).other (h + 1) : Chan)
      = wireIn (p.other, h) := by
    rw [← hsp]
    rfl
  have hcen := census_reach W p
    (evIdx ((wireIn (p.other, h), false, k) : Ev) (scheduleE sk) + 1)
    h k hh hkr
    (by rw [hconv]; exact hrfacts.1)
    (by rw [hconv]; omega)
    (by rw [hconv]; omega)
  obtain ⟨hclen, hsc, -⟩ := hcen
  -- the kid-record harvest, against any pair reason
  have hkidrec : ∀ k' j, k' ≤ k → sk.childIsD h (sk.stageScope h k') j
        = true →
      ((k' < k) ∨ ((∃ i', j < i'
          ∧ i' < sk.nChildren h (sk.stageScope h k')
          ∧ e ∈ Sched.childChunk sk (p.other, h) k' i')
        ∨ (e ∈ Sched.childChunk sk (p.other, h) k' j
            ∧ e ≠ ((wireOut (p.other, h), true,
                sk.wiresBefore h k' + j) : Ev))
        ∨ e = ((upperOut (p.other, h), true, k') : Ev))) →
      (sk.scope (sk.stageScope h k')).kids.getD j 0
        ∈ announcedIds sk p (s.hist p) := by
    intro k' j hk' hD hreason
    obtain ⟨h0, hjlen⟩ := childIsD_facts hD
    have hk'st : k' < sk.stageLen h := by omega
    have hnc : sk.nChildren h (sk.stageScope h k')
        = (sk.scope (sk.stageScope h k')).kids.length := by
      unfold Skel.nChildren
      rw [if_neg (by simpa using h0)]
    have hjn : j < sk.nChildren h (sk.stageScope h k') := by omega
    have hpair : ([((wireOut (p.other, h), true,
        sk.wiresBefore h k' + j) : Ev), e] : List Ev).Sublist
        (Sched.walkEventsE sk (p.other, h)) := by
      rcases hreason with hlt | hcases
      · -- cross-block pair
        have hwmem := wire_mem_childChunk (sk := sk) (p.other, h) k' j
        have hwblk : ((wireOut (p.other, h), true,
            sk.wiresBefore h k' + j) : Ev)
            ∈ Sched.scopeBlockE sk (p.other, h) k' :=
          chunk_mem_scopeBlockE hjn hwmem
        unfold Sched.walkEventsE
        exact sublist_flatMap_pair hlt hkr hwblk hek
      · exact chunk_pair_walk W.wf hpk hk'st hjn hcases
    obtain ⟨hwm, hwlt⟩ := tau_prior W.wf W.m0 hTmem hpair
    have hwm' : ((Chan.wire p.other h, true, sk.wiresBefore h k' + j)
        : Ev) ∈ scheduleE sk := hwm
    have hwlt' : evIdx ((Chan.wire p.other h, true,
        sk.wiresBefore h k' + j) : Ev) (scheduleE sk)
        < evIdx e (scheduleE sk) := hwlt
    exact W.kid_minted p hpk hk'st hjlen h0 hwm' (by omega)
  -- every earlier block completes
  have hoks : ∀ j, 0 ≤ j → j < k →
      (peerBlockA (aviewOf sk p (s.hist p)) p.other h j
        (sk.stageScope h j) (sk.wiresBefore h j) (sk.dsBefore h j)
        (sk.qsBefore h j)).2.2 = true := by
    intro j _ hj
    refine peerBlockA_ok W.wf p.other (by omega) (hsc j (by omega))
      (fun j' hD' => ?_) _ _ _
    exact hkidrec j j' (by omega) hD' (Or.inl hj)
  -- block k's announced layout contains e
  have hblockmem : e ∈ (peerBlockA (aviewOf sk p (s.hist p)) p.other h k
      (sk.stageScope h k) (sk.wiresBefore h k) (sk.dsBefore h k)
      (sk.qsBefore h k)).1 := by
    rcases pair_of_mem_cons hek' with heq | -
    · subst heq
      exact prologue_mem_peerBlockA p.other h k _ _ _ _ (Or.inl rfl)
    rcases List.mem_cons.mp hek' with heq | hek2
    · subst heq
      exact prologue_mem_peerBlockA p.other h k _ _ _ _ (Or.inl rfl)
    rcases List.mem_cons.mp hek2 with heq | hek3
    · subst heq
      exact prologue_mem_peerBlockA p.other h k _ _ _ _ (Or.inr rfl)
    -- e is one of the block's sends
    have hann := hsc k (Nat.le_refl _)
    unfold Sched.scopeSendsE at hek3
    rcases List.mem_append.mp hek3 with hflat | hpar
    · -- a chunk event
      rw [← List.flatMap_def] at hflat
      obtain ⟨it, hitr, heit⟩ := List.mem_flatMap.mp hflat
      rw [List.mem_range] at hitr
      by_cases h0 : h = 0
      · -- the leaf stage: the whole supply run is laid at once
        subst h0
        have hDi : sk.childIsD 0 (sk.stageScope 0 k) it = false := rfl
        have hcc : Sched.childChunk sk (p.other, 0) k it
            = [((wireOut (p.other, 0), true,
                sk.wiresBefore 0 k + it) : Ev)] := by
          unfold Sched.childChunk
          rw [if_neg (by simp [hDi])]
        rw [hcc, List.mem_singleton] at heit
        subst heit
        exact chunkOut_mem_peerBlockA p.other hkr hann
          (leaf_chunks_mem p.other hkr hann hitr _ _)
      · -- an interior stage: cover the target chunk
        have h0' : (h == 0) = false := by simpa using h0
        have hnc : sk.nChildren h (sk.stageScope h k)
            = (sk.scope (sk.stageScope h k)).kids.length := by
          unfold Skel.nChildren
          rw [if_neg (by simpa using h0)]
        have hitr' : it < sk.nChildren h (sk.stageScope h k) := hitr
        have hit : it < (sk.scope (sk.stageScope h k)).kids.length := by
          omega
        have hbelow : ∀ j, 0 ≤ j → j < it →
            sk.childIsD h (sk.stageScope h k) j = true →
            (sk.scope (sk.stageScope h k)).kids.getD j 0
              ∈ announcedIds sk p (s.hist p) := by
          intro j _ hjit hD
          exact hkidrec k j (Nat.le_refl _) hD
            (Or.inr (Or.inl ⟨it, hjit, hitr, heit⟩))
        have hcov := chunksA_covers W.wf p.other hkr h0' hann
          (sk.wiresBefore h k) it hit
          ((sk.scope (sk.stageScope h k)).kids.length + 1) 0
          (sk.wiresBefore h k) (sk.dsBefore h k) (sk.qsBefore h k)
          (Nat.zero_le _) (by omega) (by omega) (by simp) (by simp)
          hbelow
        by_cases hwire : e = ((wireOut (p.other, h), true,
            sk.wiresBefore h k + it) : Ev)
        · subst hwire
          exact chunkOut_mem_peerBlockA p.other hkr hann hcov.1
        · refine chunkOut_mem_peerBlockA p.other hkr hann
            (hcov.2 (fun hD => ?_) e heit)
          exact hkidrec k it (Nat.le_refl _) hD
            (Or.inr (Or.inr (Or.inl ⟨heit, hwire⟩)))
    · -- the parent summary: the whole block is complete
      rw [List.mem_singleton] at hpar
      subst hpar
      have hok := peerBlockA_ok W.wf p.other hkr hann
        (fun j' hD' => hkidrec k j' (Nat.le_refl _) hD'
          (Or.inr (Or.inr (Or.inr rfl))))
        (sk.wiresBefore h k) (sk.dsBefore h k) (sk.qsBefore h k)
      exact parent_mem_peerBlockA p.other hann hok
  -- assemble through the go loop
  have hpre := stageScopesA_prefix W.wf p (s.hist p) hh
  show e ∈ peerWalkTraceA.go (aviewOf sk p (s.hist p)) h
    ((aviewOf sk p (s.hist p)).party).other
    (stageScopesA (aviewOf sk p (s.hist p)) h).1 0 0 0 0
  have hgo := goA_mem (p := p) (tr := s.hist p) W.wf hh
    (stageScopesA (aviewOf sk p (s.hist p)) h).1 0 k
    (fun j hj => by
      have := prefix_getD hpre.1 hj 0
      rw [← this]
      show _ = sk.stageScope h (0 + j)
      rw [Nat.zero_add]
      rfl)
    (Nat.zero_le _) (by omega) hkr
    (fun j hj hjk => hoks j hj hjk) hblockmem
  exact hgo


-- ============================= the opener, finale, and absorber laid

/-- The root record is announced once the peer's root-wire send is
scheduled below the wall. -/
private theorem Wall.root_minted {s : MState} {N : Nat} (W : Wall sk s N)
    (p : Party)
    (hmem : ((Chan.wire p.other sk.rootH, true, 0) : Ev) ∈ scheduleE sk)
    (hτ : evIdx ((Chan.wire p.other sk.rootH, true, 0) : Ev)
      (scheduleE sk) < N) :
    (aviewOf sk p (s.hist p)).rec? 0 = some (sk.scope 0) := by
  rw [rec?_aviewOf, if_pos (announced_root (W.root_delivered p hmem hτ))]

/-- The peer opener's events below the wall are announced-laid. -/
theorem open_laid {s : MState} {N : Nat} (W : Wall sk s N) (p : Party)
    {e : Ev}
    (he : e ∈ (if p = Party.I then Sched.ropenEvents sk
      else Sched.iopenEvents sk))
    (hτ : evIdx e (scheduleE sk) < N) :
    e ∈ peerOpenTraceA (aviewOf sk p (s.hist p)) := by
  cases p with
  | I =>
      rw [if_pos rfl] at he
      unfold Sched.ropenEvents at he
      show e ∈ [((Chan.wire Party.I sk.rootH : Chan), false, 0),
          ((Chan.wire Party.R sk.rootH : Chan), true, 0),
          ((Chan.rootres : Chan), true, 0)]
        ++ (match (aviewOf sk Party.I (s.hist Party.I)).rec? 0 with
            | none => []
            | some sc =>
                (List.range sc.kids.length).map fun j =>
                  ((Chan.asked Party.R (sk.rootH - 2) : Chan), true, j))
      rcases List.mem_cons.mp he with rfl | he2
      · exact List.mem_append.mpr (.inl (List.mem_cons_self ..))
      rcases List.mem_cons.mp he2 with rfl | he3
      · exact List.mem_append.mpr (.inl (List.mem_cons_of_mem _
          (List.mem_cons_self ..)))
      rcases List.mem_cons.mp he3 with rfl | he4
      · exact List.mem_append.mpr (.inl (List.mem_cons_of_mem _
          (List.mem_cons_of_mem _ (List.mem_cons_self ..))))
      -- a root query: the peer's reply is trace-prior, minting the root
      obtain ⟨j, hj, rfl⟩ := List.mem_map.mp he4
      rw [List.mem_range] at hj
      have hpair : ([((Chan.wire Party.R sk.rootH, true, 0) : Ev),
          ((Chan.asked Party.R (sk.rootH - 2), true, j) : Ev)]
            : List Ev).Sublist (Sched.ropenEvents sk) := by
        unfold Sched.ropenEvents
        refine List.Sublist.trans ?_ (List.sublist_cons_self ..)
        refine List.cons_sublist_cons.mpr ?_
        refine List.Sublist.trans ?_ (List.sublist_cons_self ..)
        exact List.singleton_sublist.mpr
          (List.mem_map.mpr ⟨j, List.mem_range.mpr hj, rfl⟩)
      obtain ⟨hm, hlt⟩ := tau_prior W.wf W.m0
        (Sched.fixed_mem_procsE sk).2.1 hpair
      have hm' : ((Chan.wire Party.I.other sk.rootH, true, 0) : Ev)
          ∈ scheduleE sk := hm
      have hlt' : evIdx ((Chan.wire Party.I.other sk.rootH, true, 0)
          : Ev) (scheduleE sk)
          < evIdx ((Chan.asked Party.R (sk.rootH - 2), true, j) : Ev)
            (scheduleE sk) := hlt
      have hrec := W.root_minted Party.I hm' (by omega)
      rw [hrec]
      refine List.mem_append.mpr (.inr ?_)
      refine List.mem_map.mpr ⟨j, List.mem_range.mpr ?_, rfl⟩
      show j < (sk.scope 0).kids.length
      exact hj
  | R =>
      rw [if_neg (by simp)] at he
      exact he

/-- The peer finale's events below the wall are announced-laid. -/
theorem fin_laid {s : MState} {N : Nat} (W : Wall sk s N) (p : Party)
    {e : Ev}
    (he : e ∈ (if p = Party.I then Sched.finEvents sk
      else [((Chan.rootret : Chan), false, 0)])) :
    evIdx e (scheduleE sk) < N →
    ∃ T ∈ peerFinTracesA (aviewOf sk p (s.hist p)), e ∈ T := by
  intro hτ
  cases p with
  | I =>
      rw [if_pos rfl] at he
      refine ⟨((Chan.rootres : Chan), false, 0)
        :: (match (aviewOf sk Party.I (s.hist Party.I)).rec? 0 with
            | none => []
            | some sc =>
                (List.range sc.kids.length).map fun j =>
                  ((Chan.rootrets : Chan), false, j)),
        List.mem_singleton.mpr rfl, ?_⟩
      unfold Sched.finEvents at he
      rcases List.mem_cons.mp he with rfl | he2
      · exact List.mem_cons_self ..
      -- a root return: chain through the finale's own resolution
      -- receive and the opener's reply to the root mint
      obtain ⟨j, hj, rfl⟩ := List.mem_map.mp he2
      rw [List.mem_range] at hj
      have hpair : ([((Chan.rootres : Chan), false, 0),
          ((Chan.rootrets, false, j) : Ev)] : List Ev).Sublist
          (Sched.finEvents sk) := by
        unfold Sched.finEvents
        refine List.cons_sublist_cons.mpr ?_
        exact List.singleton_sublist.mpr
          (List.mem_map.mpr ⟨j, List.mem_range.mpr hj, rfl⟩)
      obtain ⟨hm, hlt⟩ := tau_prior W.wf W.m0
        (Sched.fixed_mem_procsE sk).2.2.2.2 hpair
      obtain ⟨hsm, hslt⟩ := tau_e1 W.wf hm
      have hpair2 : ([((Chan.wire Party.R sk.rootH, true, 0) : Ev),
          ((Chan.rootres, true, 0) : Ev)] : List Ev).Sublist
          (Sched.ropenEvents sk) := by
        unfold Sched.ropenEvents
        refine List.Sublist.trans ?_ (List.sublist_cons_self ..)
        exact List.cons_sublist_cons.mpr
          (List.singleton_sublist.mpr (List.mem_cons_self ..))
      obtain ⟨hm2, hlt2⟩ := tau_prior W.wf W.m0
        (Sched.fixed_mem_procsE sk).2.1 hpair2
      have hm2' : ((Chan.wire Party.I.other sk.rootH, true, 0) : Ev)
          ∈ scheduleE sk := hm2
      have hlt2' : evIdx ((Chan.wire Party.I.other sk.rootH, true, 0)
          : Ev) (scheduleE sk)
          < evIdx ((Chan.rootres, true, 0) : Ev) (scheduleE sk) := hlt2
      have hrec := W.root_minted Party.I hm2' (by omega)
      rw [hrec]
      refine List.mem_cons_of_mem _ ?_
      refine List.mem_map.mpr ⟨j, List.mem_range.mpr ?_, rfl⟩
      show j < (sk.scope 0).kids.length
      exact hj
  | R =>
      rw [if_neg (by simp)] at he
      exact ⟨[((Chan.rootret : Chan), false, 0)],
        List.mem_singleton.mpr rfl, he⟩

/-- The announced absorb total covers every record-known census
position's leaf requests. -/
private theorem totalA_reach (hwf : sk.wellFormed = true)
    {p : Party} {tr : List MObs} :
    ∀ (l : List Nat) (n : Nat), n < l.length →
      (∀ u ∈ l, u < sk.scopes.length
        ∧ (aviewOf sk p tr).kind? u = some ((sk.scope u).kind)) →
      (∀ i, i ≤ n → (sk.scope (l.getD i 0)).kind = Kind.D →
        l.getD i 0 ∈ announcedIds sk p tr) →
      ((l.take (n + 1)).map
        (fun u => (sk.scope u).leafReqs)).sum
        ≤ (peerAbsorbTraceA.total (aviewOf sk p tr) l).1 := by
  intro l
  induction l with
  | nil =>
      intro n hn
      simp at hn
  | cons u rest ih =>
      intro n hn hkinds hrecs
      obtain ⟨hreal, hkind⟩ := hkinds u (List.mem_cons_self ..)
      have htail : ((rest.take n).map
          (fun u => (sk.scope u).leafReqs)).sum
          ≤ (peerAbsorbTraceA.total (aviewOf sk p tr) rest).1 := by
        cases n with
        | zero =>
            show (([] : List Nat).map _).sum ≤ _
            simp
        | succ n' =>
            refine ih n' (by simpa using hn)
              (fun v hv => hkinds v (List.mem_cons_of_mem _ hv))
              (fun i hi hD => ?_)
            have := hrecs (i + 1) (by omega)
            rw [List.getD_cons_succ] at this
            exact this hD
      rw [List.take_succ_cons, List.map_cons, List.sum_cons]
      rw [peerAbsorbTraceA.total]
      by_cases hD : (sk.scope u).kind = Kind.D
      · rw [if_pos (by rw [hkind, hD]; rfl)]
        have hann : u ∈ announcedIds sk p tr := by
          have := hrecs 0 (by omega)
          rw [List.getD_cons_zero] at this
          exact this hD
        have hrec : (aviewOf sk p tr).rec? u = some (sk.scope u) := by
          rw [rec?_aviewOf, if_pos hann]
        rw [hrec]
        rcases hc : peerAbsorbTraceA.total (aviewOf sk p tr) rest
          with ⟨t, ok⟩
        rw [hc] at htail
        show (sk.scope u).leafReqs + _ ≤ (sk.scope u).leafReqs + t
        omega
      · rw [if_neg (by
          rw [hkind]
          simp only [beq_iff_eq, Option.some.injEq]
          exact fun hc => hD hc)]
        have hzero : (sk.scope u).leafReqs = 0 :=
          (wf_scope_nonD hwf hreal hD).2
        rw [hzero]
        omega

/-- The absorber's events below the wall are announced-laid (laid only
on the responder's side, whose peer owns the absorber). -/
theorem absorb_laid {s : MState} {N : Nat} (W : Wall sk s N)
    (hp : Party.R = Party.R) {e : Ev}
    (he : e ∈ Sched.absorbEvents sk)
    (hτ : evIdx e (scheduleE sk) < N) :
    e ∈ peerAbsorbTraceA (aviewOf sk Party.R (s.hist Party.R)) := by
  have hev : sk.rootH % 2 = 0 := (wf_rootH W.wf).1
  have h2 : 2 ≤ sk.rootH := (wf_rootH W.wf).2
  unfold Sched.absorbEvents at he
  obtain ⟨j, hjr, hej⟩ := List.mem_flatMap.mp he
  have hj : j < sk.totalLeafReqs := List.mem_range.mp hjr
  -- the block head receive is trace-at-or-before e
  have hrfacts : ((Chan.wire Party.R 0, false, j) : Ev) ∈ scheduleE sk
      ∧ evIdx ((Chan.wire Party.R 0, false, j) : Ev) (scheduleE sk)
        ≤ evIdx e (scheduleE sk) := by
    have hemem : e ∈ scheduleE sk :=
      (Sched.trace_sublistE sk W.wf W.m0
        (Sched.fixed_mem_procsE sk).2.2.1).mem he
    rcases pair_of_mem_cons hej with heq | hpair
    · subst heq
      exact ⟨hemem, Nat.le_refl _⟩
    · have hlift : ([((Chan.wire Party.R 0, false, j) : Ev), e]
          : List Ev).Sublist (Sched.absorbEvents sk) := by
        unfold Sched.absorbEvents
        exact sublist_flatMap_block hj hpair
      obtain ⟨hm, hlt⟩ := tau_prior W.wf W.m0
        (Sched.fixed_mem_procsE sk).2.2.1 hlift
      exact ⟨hm, by omega⟩
  -- its send is the responder's own leaf wire; locate and recurse
  obtain ⟨hsm, hslt⟩ := tau_e1 W.wf hrfacts.1
  have hR0 : (Party.R, 0) ∈ sk.walkKeys :=
    Sched.mem_walkKeys_of sk W.wf (by omega) (Or.inr ⟨rfl, rfl⟩)
  have hjw : j < sk.wiresBefore 0 (sk.stageLen 0) := by
    rw [wiresBefore_full_leaf W.wf]
    exact hj
  obtain ⟨m₀, hm₀, hle₀, hlt₀, hpairs⟩ := wire_send_locate W.wf hR0 hjw
  obtain ⟨hr'm, hr'lt⟩ := tau_prior W.wf W.m0
    (Sched.walkEventsE_mem_procsE sk W.wf hR0) hpairs
  -- the census reaches the covering scope
  have hconv : (Chan.wire (stageParty 0).other (0 + 1) : Chan)
      = wireIn (Party.R, 0) := rfl
  have hcen := census_reach W Party.R
    (evIdx ((wireIn (Party.R, 0), false, m₀) : Ev) (scheduleE sk) + 1)
    0 m₀ (by omega) hm₀
    (by rw [hconv]; exact hr'm)
    (by rw [hconv]
        have hsl : evIdx ((Chan.wire Party.R 0, true, j) : Ev)
            (scheduleE sk) < N := by omega
        omega)
    (by rw [hconv]; omega)
  obtain ⟨hclen, hsc, -⟩ := hcen
  -- the announced total covers block j
  have hpre := stageScopesA_prefix W.wf Party.R (s.hist Party.R)
    (show 0 < sk.rootH by omega)
  have hitems : (stageScopesA (aviewOf sk Party.R (s.hist Party.R)) 0).1
      = (levelA (aviewOf sk Party.R (s.hist Party.R))
          (sk.rootH - 1)).1 := by
    unfold stageScopesA
    rw [if_neg (by show ¬ ((0 + 1 == sk.rootH) = true); simp; omega),
      if_neg (by show ¬ (sk.rootH < 0 + 1); omega)]
    rfl
  have hkinds : ∀ u ∈ (stageScopesA (aviewOf sk Party.R
      (s.hist Party.R)) 0).1, u < sk.scopes.length
      ∧ (aviewOf sk Party.R (s.hist Party.R)).kind? u
        = some ((sk.scope u).kind) := by
    intro u hu
    refine ⟨?_, hpre.2 u hu⟩
    exact (mem_scopesAt (hpre.1.sublist.mem hu)).1
  have hrecs : ∀ i, i ≤ m₀ →
      (sk.scope ((stageScopesA (aviewOf sk Party.R
        (s.hist Party.R)) 0).1.getD i 0)).kind = Kind.D →
      (stageScopesA (aviewOf sk Party.R (s.hist Party.R)) 0).1.getD i 0
        ∈ announcedIds sk Party.R (s.hist Party.R) := by
    intro i hi hD
    have hgd : (stageScopesA (aviewOf sk Party.R
        (s.hist Party.R)) 0).1.getD i 0 = sk.stageScope 0 i := by
      have := prefix_getD hpre.1 (show i
        < (stageScopesA (aviewOf sk Party.R
            (s.hist Party.R)) 0).1.length from by omega) 0
      rw [← this]
      rfl
    rw [hgd]
    exact hsc i hi
  have hcov := totalA_reach W.wf
    (stageScopesA (aviewOf sk Party.R (s.hist Party.R)) 0).1 m₀
    (by omega) hkinds hrecs
  -- the taken census prefix's leaf sum is the wire prefix sum
  have htake : (stageScopesA (aviewOf sk Party.R
      (s.hist Party.R)) 0).1.take (m₀ + 1)
      = (sk.stageScopes 0).take (m₀ + 1) := by
    obtain ⟨t, ht⟩ := hpre.1
    rw [← ht, List.take_append_of_le_length (by omega)]
  have hsum : (((sk.stageScopes 0).take (m₀ + 1)).map
      (fun u => (sk.scope u).leafReqs)).sum
      = sk.wiresBefore 0 (m₀ + 1) := by
    unfold Skel.wiresBefore
    rw [foldl_add_eq_sum, Nat.zero_add]
    congr 1
  have hjtot : j < (peerAbsorbTraceA.total
      (aviewOf sk Party.R (s.hist Party.R))
      (stageScopesA (aviewOf sk Party.R (s.hist Party.R)) 0).1).1 := by
    rw [htake, hsum] at hcov
    omega
  -- assemble
  show e ∈ peerAbsorbTraceA (aviewOf sk Party.R (s.hist Party.R))
  unfold peerAbsorbTraceA
  rw [if_neg (show ¬ (((aviewOf sk Party.R (s.hist Party.R)).party
    == Party.I) = true) from fun hc => nomatch hc)]
  show e ∈ (List.range ((peerAbsorbTraceA.total
      (aviewOf sk Party.R (s.hist Party.R))
      (levelA (aviewOf sk Party.R (s.hist Party.R))
        (sk.rootH - 1)).1).1)).flatMap fun j =>
    [((Chan.wire Party.R 0 : Chan), false, j),
     ((Chan.leafRequests : Chan), false, j),
     ((Chan.level Party.I 0 : Chan), true, j)]
  rw [← hitems]
  refine List.mem_flatMap.mpr ⟨j, List.mem_range.mpr hjtot, ?_⟩
  exact hej


-- ==================================================== the asm family

/-- Locate a position under any prefix-sum ledger anchored at zero. -/
private theorem exists_prefix_block (f : Nat → Nat) (hf0 : f 0 = 0)
    {M k : Nat} (hk : k < f M) :
    ∃ n, n < M ∧ f n ≤ k ∧ k < f (n + 1) := by
  induction M with
  | zero =>
      rw [hf0] at hk
      omega
  | succ M ih =>
      by_cases hin : k < f M
      · obtain ⟨n, hn, h1, h2⟩ := ih hin
        exact ⟨n, by omega, h1, h2⟩
      · exact ⟨M, by omega, by omega, hk⟩

/-- An assembler key's height bounds. -/
private theorem asmKeys_bounds {q : Party} {j : Nat}
    (hpk : (q, j) ∈ sk.asmKeys) :
    1 ≤ j ∧ j ≤ sk.rootH ∧ (q = Party.R → j ≤ sk.rootH - 1) := by
  unfold Skel.asmKeys at hpk
  rcases List.mem_append.mp hpk with hI | hR
  · obtain ⟨m, hm, hme⟩ := List.mem_map.mp hI
    rw [List.mem_range] at hm
    rw [Prod.mk.injEq] at hme
    refine ⟨by omega, by omega, fun hq => ?_⟩
    rw [← hme.1] at hq
    exact absurd hq (by decide)
  · obtain ⟨m, hm, hme⟩ := List.mem_map.mp hR
    rw [List.mem_range] at hm
    rw [Prod.mk.injEq] at hme
    exact ⟨by omega, by omega, fun _ => by omega⟩

/-- The go loop lays a target assembler block once the pending entries
through it are known with their true values. -/
private theorem goAsm_mem {p : Party} {j : Nat} :
    ∀ (ps : List (Option Nat)) (idx₀ idx got : Nat) {e : Ev},
      idx₀ ≤ idx → idx - idx₀ < ps.length →
      (∀ m, idx₀ ≤ m → m ≤ idx →
        ps.getD (m - idx₀) none = some (sk.pendAt p.other j m)) →
      got = sk.pendsBefore p.other j idx₀ →
      idx < (sk.asmResList p.other j).length →
      e ∈ Sched.asmBlock sk (p.other, j) idx →
      e ∈ peerAsmTraceA.go (asmResChan (p.other, j))
          (asmLevelChan (p.other, j)) (sk.asmOutChan (p.other, j))
          ps idx₀ got := by
  intro ps
  induction ps with
  | nil =>
      intro idx₀ idx got e _ hlen
      simp at hlen
  | cons pe rest ih =>
      intro idx₀ idx got e hle hlen hvals hgot hidx he
      have hpe : pe = some (sk.pendAt p.other j idx₀) := by
        have := hvals idx₀ (Nat.le_refl _) hle
        rw [show idx₀ - idx₀ = 0 from by omega,
          List.getD_cons_zero] at this
        exact this
      subst hpe
      have hgo : peerAsmTraceA.go (asmResChan (p.other, j))
          (asmLevelChan (p.other, j)) (sk.asmOutChan (p.other, j))
          (some (sk.pendAt p.other j idx₀) :: rest) idx₀ got
          = (asmResChan (p.other, j), false, idx₀)
            :: ((List.range (sk.pendAt p.other j idx₀)).map fun t =>
                (asmLevelChan (p.other, j), false, got + t))
            ++ (sk.asmOutChan (p.other, j), true, idx₀)
            :: peerAsmTraceA.go (asmResChan (p.other, j))
                (asmLevelChan (p.other, j)) (sk.asmOutChan (p.other, j))
                rest (idx₀ + 1)
                (got + sk.pendAt p.other j idx₀) := by
        simp only [peerAsmTraceA.go]
      rw [hgo]
      by_cases hii : idx = idx₀
      · -- the target block: the emitted block IS the true block
        subst hii
        unfold Sched.asmBlock at he
        rw [hgot]
        rcases List.mem_cons.mp he with rfl | he2
        · exact List.mem_cons_self ..
        rcases List.mem_append.mp he2 with hmap | hout
        · refine List.mem_cons_of_mem _ (List.mem_append.mpr
            (.inl ?_))
          exact hmap
        · rw [List.mem_singleton] at hout
          subst hout
          refine List.mem_cons_of_mem _ (List.mem_append.mpr
            (.inr ?_))
          exact List.mem_cons_self ..
      · -- advance: the counters track the prefix sums
        have hlt : idx₀ < idx := by omega
        refine List.mem_cons_of_mem _ (List.mem_append.mpr (.inr
          (List.mem_cons_of_mem _ ?_)))
        refine ih (idx₀ + 1) idx
          (got + sk.pendAt p.other j idx₀) (by omega)
          (by simp only [List.length_cons] at hlen; omega)
          (fun m hm1 hm2 => by
            have := hvals m (by omega) hm2
            rw [show m - idx₀ = (m - (idx₀ + 1)) + 1 from by omega,
              List.getD_cons_succ] at this
            exact this)
          (by rw [hgot, Sched.pendsBefore_succ sk (by omega)])
          hidx he


/-- The assembler census items are the stage-below's announced
census. -/
private theorem asm_items_eq (p : Party) (tr : List MObs) {j : Nat}
    (hj1 : 1 ≤ j) (hjr : j ≤ sk.rootH) :
    (if (j == (aviewOf sk p tr).rootH) = true then ([0], true)
     else levelA (aviewOf sk p tr) ((aviewOf sk p tr).rootH - j)).1
      = (stageScopesA (aviewOf sk p tr) (j - 1)).1 := by
  unfold stageScopesA
  rw [show j - 1 + 1 = j from by omega]
  by_cases hjtop : (j == (aviewOf sk p tr).rootH) = true
  · rw [if_pos hjtop, if_pos hjtop]
  · rw [if_neg hjtop, if_neg hjtop,
      if_neg (show ¬ ((aviewOf sk p tr).rootH < j) from by
        show ¬ (sk.rootH < j)
        omega)]

/-- The asker side of the assembler minting: block `idx`'s pends are
the stage-below scopes' dispute counts, and their records ride the
census reached from the parent summaries the assembler consumes. -/
private theorem asm_laid_asker {s : MState} {N : Nat} (W : Wall sk s N)
    (p : Party) {j : Nat} (hpk : (p.other, j) ∈ sk.asmKeys)
    (hside : asks p.other j = true) {idx : Nat}
    (hidx : idx < (sk.asmResList p.other j).length) {e : Ev}
    (he : e ∈ Sched.asmBlock sk (p.other, j) idx)
    (hheadm : ((asmResChan (p.other, j), false, idx) : Ev)
      ∈ scheduleE sk)
    (hheadτ : evIdx ((asmResChan (p.other, j), false, idx) : Ev)
      (scheduleE sk) < N) :
    e ∈ peerAsmTraceA (aviewOf sk p (s.hist p)) j := by
  obtain ⟨hj1, hjr, -⟩ := asmKeys_bounds hpk
  -- the resolution channel is the stage-below's parent summary
  have hres : asmResChan (p.other, j) = Chan.upper p.other (j - 1) := by
    unfold asmResChan
    rw [if_pos hside]
  obtain ⟨hsm, hslt⟩ := tau_e1 W.wf hheadm
  have hj1r : j - 1 < sk.rootH := by omega
  -- the parent's producer stage is a walk key by asking parity
  have hstg : (p.other, j - 1) ∈ sk.walkKeys := by
    have hev : sk.rootH % 2 = 0 := (wf_rootH W.wf).1
    cases hq : p.other with
    | I =>
        rw [hq] at hside
        have hpar : j % 2 = 0 := by
          unfold asks at hside
          simpa using hside
        exact Sched.mem_walkKeys_of sk W.wf hj1r
          (Or.inl ⟨rfl, by omega⟩)
    | R =>
        rw [hq] at hside
        have hpar : j % 2 = 1 := by
          unfold asks at hside
          simpa using hside
        exact Sched.mem_walkKeys_of sk W.wf hj1r
          (Or.inr ⟨rfl, by omega⟩)
  have hidxs : idx < sk.stageLen (j - 1) := by
    have hlen := asmResList_asker_length (sk := sk) hside
    have : sk.stageLen (j - 1) = (sk.scopesAt j).length := by
      unfold Skel.stageLen Skel.stageScopes
      rw [show j - 1 + 1 = j from by omega]
    omega
  -- the parent summary sits in the stage trace above its prologue
  have hparent_mem : ((upperOut (p.other, j - 1), true, idx) : Ev)
      ∈ Sched.scopeBlockE sk (p.other, j - 1) idx := by
    unfold Sched.scopeBlockE
    refine List.mem_cons_of_mem _ (List.mem_cons_of_mem _ ?_)
    unfold Sched.scopeSendsE
    exact List.mem_append.mpr (.inr (List.mem_singleton.mpr rfl))
  have hppair := prologue_pair hparent_mem (by
    intro hc
    have := congrArg (fun ev : Ev => ev.2.1) hc
    simp at this)
  have hlift : ([((wireIn (p.other, j - 1), false, idx) : Ev),
      ((upperOut (p.other, j - 1), true, idx) : Ev)] : List Ev).Sublist
      (Sched.walkEventsE sk (p.other, j - 1)) := by
    unfold Sched.walkEventsE
    exact sublist_flatMap_block hidxs hppair
  obtain ⟨hr'm, hr'lt⟩ := tau_prior W.wf W.m0
    (Sched.walkEventsE_mem_procsE sk W.wf hstg) hlift
  -- the parent IS the resolution send: align the τ chain
  have hsend_eq : ((upperOut (p.other, j - 1), true, idx) : Ev)
      = ((asmResChan (p.other, j), true, idx) : Ev) := by
    rw [hres]
    rfl
  have hr'N : evIdx ((wireIn (p.other, j - 1), false, idx) : Ev)
      (scheduleE sk) < N := by
    rw [hsend_eq] at hr'lt
    omega
  -- the census reaches block idx of the stage below
  have hsp := (walkKeys_stageParty W.wf hstg).1
  have hconv : (Chan.wire (stageParty (j - 1)).other ((j - 1) + 1)
      : Chan) = wireIn (p.other, j - 1) := by
    rw [← hsp]
    rfl
  have hcen := census_reach W p
    (evIdx ((wireIn (p.other, j - 1), false, idx) : Ev)
      (scheduleE sk) + 1)
    (j - 1) idx hj1r hidxs
    (by rw [hconv]; exact hr'm)
    (by rw [hconv]; omega)
    (by rw [hconv]; omega)
  obtain ⟨hclen, hsc, -⟩ := hcen
  -- the pending entries through idx are known with true values
  have hpre := stageScopesA_prefix W.wf p (s.hist p) hj1r
  have hraw : asmPendsA (aviewOf sk p (s.hist p)) j
      = if asks p.other j = true then
          (if (j == (aviewOf sk p (s.hist p)).rootH) = true then
            (([0] : List Nat), true)
          else levelA (aviewOf sk p (s.hist p))
            ((aviewOf sk p (s.hist p)).rootH - j)).1.map
            (fun u => match (aviewOf sk p (s.hist p)).rec? u with
              | none => none
              | some sc => some (sc.kids.countP
                  fun v => (aviewOf sk p (s.hist p)).kind? v
                    == some Kind.D))
        else
          ((if (j == (aviewOf sk p (s.hist p)).rootH) = true then
            (([0] : List Nat), true)
          else levelA (aviewOf sk p (s.hist p))
            ((aviewOf sk p (s.hist p)).rootH - j)).1.filter
            (fun u => (aviewOf sk p (s.hist p)).kind? u
              == some Kind.D)).map
            (fun u => match (aviewOf sk p (s.hist p)).rec? u with
              | none => none
              | some sc => some (if (j == 1) = true then sc.leafReqs
                  else sc.kids.length)) := rfl
  have hshape : asmPendsA (aviewOf sk p (s.hist p)) j
      = (stageScopesA (aviewOf sk p (s.hist p)) (j - 1)).1.map
          (fun u => match (aviewOf sk p (s.hist p)).rec? u with
            | none => none
            | some sc => some (sc.kids.countP
                fun v => (aviewOf sk p (s.hist p)).kind? v
                  == some Kind.D)) := by
    rw [hraw, if_pos hside, asm_items_eq p (s.hist p) hj1 hjr]
  have hentries : ∀ m, m ≤ idx →
      (asmPendsA (aviewOf sk p (s.hist p)) j).getD m none
        = some (sk.pendAt p.other j m) := by
    intro m hm
    have hmlen : m < (stageScopesA (aviewOf sk p (s.hist p))
        (j - 1)).1.length := by omega
    have hgd : (stageScopesA (aviewOf sk p (s.hist p)) (j - 1)).1.getD
        m 0 = sk.stageScope (j - 1) m := by
      have := prefix_getD hpre.1 hmlen 0
      rw [← this]
      rfl
    have hann := hsc m hm
    rw [← hgd] at hann
    have hrec : (aviewOf sk p (s.hist p)).rec?
        ((stageScopesA (aviewOf sk p (s.hist p)) (j - 1)).1.getD m 0)
        = some (sk.scope ((stageScopesA (aviewOf sk p (s.hist p))
            (j - 1)).1.getD m 0)) := by
      rw [rec?_aviewOf, if_pos hann]
    have hsome : (asmPendsA (aviewOf sk p (s.hist p)) j).getD m none
        = some ((sk.scope ((stageScopesA (aviewOf sk p (s.hist p))
            (j - 1)).1.getD m 0)).kids.countP
            fun v => (aviewOf sk p (s.hist p)).kind? v
              == some Kind.D) := by
      rw [hshape, List.getD_eq_getElem?_getD, List.getElem?_map,
        List.getElem?_eq_getElem hmlen]
      show (some _).getD none = _
      rw [Option.getD_some]
      rw [show (stageScopesA (aviewOf sk p (s.hist p)) (j - 1)).1[m]
        = (stageScopesA (aviewOf sk p (s.hist p)) (j - 1)).1.getD m 0
        from by
          rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hmlen]
          rfl]
      show (match (aviewOf sk p (s.hist p)).rec?
          ((stageScopesA (aviewOf sk p (s.hist p)) (j - 1)).1.getD m 0)
        with
        | none => none
        | some sc => some (sc.kids.countP
            fun v => (aviewOf sk p (s.hist p)).kind? v
              == some Kind.D)) = _
      rw [hrec]
    obtain ⟨-, hvals⟩ := asmPendsA_spec (sk := sk) W.wf p (s.hist p)
      hj1 hjr
    have := hvals m _ hsome
    rw [hsome, this]
  -- assemble through the go loop
  show e ∈ peerAsmTraceA.go (asmResChan (p.other, j))
    (asmLevelChan (p.other, j)) (sk.asmOutChan (p.other, j))
    (asmPendsA (aviewOf sk p (s.hist p)) j) 0 0
  refine goAsm_mem (asmPendsA (aviewOf sk p (s.hist p)) j) 0 idx 0
    (Nat.zero_le _) ?_ (fun m _ hm => by
      rw [Nat.sub_zero]
      exact hentries m hm) rfl hidx he
  rw [hshape, List.length_map]
  omega


/-- D-scope counting through a mid-block cut: the level-`j` D scopes
below wire position `wiresBefore j k + c` are the resolution seq
`dsBefore j k + dRank k c` — the walk's resolution coordinate meets the
level's position order. -/
private theorem countD_take_mid (hwf : sk.wellFormed = true)
    {j : Nat} (h1 : 1 ≤ j) (hjr : j < sk.rootH) {k : Nat}
    (hk : k < sk.stageLen j) (q : Party) :
    ∀ c, c ≤ (sk.scope (sk.stageScope j k)).kids.length →
      (((sk.scopesAt j).take (sk.wiresBefore j k + c)).filter
        (fun s => (sk.scope s).kind == Kind.D)).length
        = sk.dsBefore j k + Sched.dRank sk (q, j) k c := by
  intro c
  induction c with
  | zero =>
      intro _
      show (((sk.scopesAt j).take (sk.wiresBefore j k + 0)).filter
        _).length = _
      rw [Nat.add_zero, ds_wires hwf h1 hjr k]
      rfl
  | succ c ih =>
      intro hc
      have hc' : c < (sk.scope (sk.stageScope j k)).kids.length := by
        omega
      have hpos : sk.wiresBefore j k + c < (sk.scopesAt j).length := by
        have hstep := Sched.wiresBefore_succ sk hk
        have hnc : sk.nChildren j (sk.stageScope j k)
            = (sk.scope (sk.stageScope j k)).kids.length := by
          unfold Skel.nChildren
          rw [if_neg (by simp; omega)]
        have hbound := wiresBefore_le_scopesAt hwf h1 hjr
          (show k + 1 ≤ sk.stageLen j by omega)
        omega
      have htake : (sk.scopesAt j).take (sk.wiresBefore j k + (c + 1))
          = (sk.scopesAt j).take (sk.wiresBefore j k + c)
            ++ [(sk.scopesAt j).getD (sk.wiresBefore j k + c) 0] := by
        rw [show sk.wiresBefore j k + (c + 1)
          = (sk.wiresBefore j k + c) + 1 from by omega]
        rw [List.take_succ, List.getElem?_eq_getElem hpos]
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hpos]
        rfl
      rw [htake, List.filter_append, List.length_append, ih (by omega)]
      have hkid := stageScope_kid hwf h1 hjr hk hc'
      have hDdec : ((sk.scope ((sk.scopesAt j).getD
          (sk.wiresBefore j k + c) 0)).kind == Kind.D)
          = sk.childIsD j (sk.stageScope j k) c := by
        rw [hkid]
        unfold Skel.childIsD
        rw [if_neg (by simp; omega), List.getElem?_eq_getElem hc']
        rw [show (sk.scope (sk.stageScope j k)).kids.getD c 0
          = (sk.scope (sk.stageScope j k)).kids[c] from by
            rw [List.getD_eq_getElem?_getD,
              List.getElem?_eq_getElem hc']
            rfl]
      rw [Sched.dRank_succ]
      show _ + (List.filter _ [_]).length = _
      rw [List.filter_cons]
      by_cases hD : sk.childIsD j (sk.stageScope j k) c = true
      · rw [if_pos (by rw [hDdec]; exact hD), if_pos hD]
        simp only [List.filter_nil, List.length_cons, List.length_nil]
        omega
      · rw [if_neg (by rw [hDdec]; simpa using hD),
          if_neg (by simpa using hD)]
        simp only [List.filter_nil, List.length_nil]
        omega


/-- The answerer side of the assembler minting: block `idx`'s pend is
the `idx`-th level-`j` D scope's own census, minted by that scope's own
frame — which departs strictly before the resolution the assembler
consumes. -/
private theorem asm_laid_answerer {s : MState} {N : Nat}
    (W : Wall sk s N) (p : Party) {j : Nat}
    (hpk : (p.other, j) ∈ sk.asmKeys)
    (hside : asks p.other j = false) {idx : Nat}
    (hidx : idx < (sk.asmResList p.other j).length) {e : Ev}
    (he : e ∈ Sched.asmBlock sk (p.other, j) idx)
    (hheadm : ((asmResChan (p.other, j), false, idx) : Ev)
      ∈ scheduleE sk)
    (hheadτ : evIdx ((asmResChan (p.other, j), false, idx) : Ev)
      (scheduleE sk) < N) :
    e ∈ peerAsmTraceA (aviewOf sk p (s.hist p)) j := by
  obtain ⟨hj1, hjr, hjR⟩ := asmKeys_bounds hpk
  have hev : sk.rootH % 2 = 0 := (wf_rootH W.wf).1
  have hres : asmResChan (p.other, j) = Chan.lower p.other j := by
    unfold asmResChan
    rw [if_neg (by simp [hside])]
  obtain ⟨hsm, hslt⟩ := tau_e1 W.wf hheadm
  -- the answerer's stage is its own walk key
  have hstg : (p.other, j) ∈ sk.walkKeys := by
    cases hq : p.other with
    | I =>
        rw [hq] at hside
        have hpar : j % 2 = 1 := by
          unfold asks at hside
          simp only [beq_eq_false_iff_ne, ne_eq] at hside
          omega
        exact Sched.mem_walkKeys_of sk W.wf (by omega)
          (Or.inl ⟨rfl, hpar⟩)
    | R =>
        rw [hq] at hside
        have hpar : j % 2 = 0 := by
          unfold asks at hside
          simp only [beq_eq_false_iff_ne, ne_eq] at hside
          omega
        have hjR' := hjR hq
        exact Sched.mem_walkKeys_of sk W.wf (by omega)
          (Or.inr ⟨rfl, hpar⟩)
  have hjlt : j < sk.rootH := (walkKeys_stageParty W.wf hstg).2
  -- the resolution's block and D-child coordinates
  have hidxd : idx < sk.dsBefore j (sk.stageLen j) := by
    have := answerer_resList_total W.wf hside hj1 hjlt
    omega
  obtain ⟨ks, hks, hdle, hdlt⟩ := exists_prefix_block
    (sk.dsBefore j) rfl hidxd
  have hdstep := Sched.dsBefore_succ sk hks
  have hrlt : idx - sk.dsBefore j ks
      < Sched.dRank sk (p.other, j) ks
          (sk.nChildren j (sk.stageScope j ks)) := by
    rw [Sched.dRank_total]
    have : sk.dOf j (sk.stageScope j ks)
        = sk.dOf (p.other, j).2 (sk.stageScope (p.other, j).2 ks) := rfl
    omega
  obtain ⟨cs, hcs, hrle, hrlt2⟩ := exists_prefix_block
    (fun c => Sched.dRank sk (p.other, j) ks c) rfl hrlt
  have hrsucc := Sched.dRank_succ sk (p.other, j) ks cs
  have hcsD : sk.childIsD j (sk.stageScope j ks) cs = true := by
    by_cases hD : sk.childIsD (p.other, j).2
        (sk.stageScope (p.other, j).2 ks) cs = true
    · exact hD
    · rw [if_neg hD] at hrsucc
      omega
  have hrank : Sched.dRank sk (p.other, j) ks cs
      = idx - sk.dsBefore j ks := by
    rw [if_pos hcsD] at hrsucc
    omega
  obtain ⟨hj0, hcslen⟩ := childIsD_facts hcsD
  have hnc : sk.nChildren j (sk.stageScope j ks)
      = (sk.scope (sk.stageScope j ks)).kids.length := by
    unfold Skel.nChildren
    rw [if_neg (by simpa using hj0)]
  -- the resolution is the D child's chunk event, after its own wire
  have hchunk : Sched.childChunk sk (p.other, j) ks cs
      = ((wireOut (p.other, j), true, sk.wiresBefore j ks + cs) : Ev)
        :: ((lowerOut (p.other, j), true, sk.dsBefore j ks
            + ((List.range cs).filter
                (fun i' => sk.childIsD j (sk.stageScope j ks)
                  i')).length) : Ev)
        :: ((List.range (sk.qCount j (sk.stageScope j ks) cs)).map
            fun t => (askedOut (p.other, j), true,
              sk.qsBefore j ks + ((List.range cs).map
                (fun i' => sk.qCount j (sk.stageScope j ks) i')).sum
                + t)) := by
    unfold Sched.childChunk
    rw [if_pos hcsD]
  have hfr : ((List.range cs).filter
      (fun i' => sk.childIsD j (sk.stageScope j ks) i')).length
      = Sched.dRank sk (p.other, j) ks cs := rfl
  have hresev : ((lowerOut (p.other, j), true, sk.dsBefore j ks
      + ((List.range cs).filter
          (fun i' => sk.childIsD j (sk.stageScope j ks) i')).length)
        : Ev)
      = ((asmResChan (p.other, j), true, idx) : Ev) := by
    rw [hres]
    show (Chan.lower p.other j, true, _)
      = (Chan.lower p.other j, true, _)
    rw [Prod.mk.injEq, Prod.mk.injEq]
    refine ⟨rfl, rfl, ?_⟩
    rw [hfr, hrank]
    omega
  -- pair the wire under the resolution inside the chunk, and both
  -- under the block prologue
  have hrmem_chunk : ((asmResChan (p.other, j), true, idx) : Ev)
      ∈ Sched.childChunk sk (p.other, j) ks cs := by
    rw [hchunk, ← hresev]
    exact List.mem_cons_of_mem _ (List.mem_cons_self ..)
  have hwirepair : ([((wireOut (p.other, j), true,
      sk.wiresBefore j ks + cs) : Ev),
      ((asmResChan (p.other, j), true, idx) : Ev)] : List Ev).Sublist
      (Sched.walkEventsE sk (p.other, j)) := by
    unfold Sched.walkEventsE
    refine sublist_flatMap_block hks (sublist_scopeBlockE_of_chunks
      (sublist_flatMap_block (by omega) ?_))
    rw [hchunk, ← hresev]
    exact List.cons_sublist_cons.mpr (List.singleton_sublist.mpr
      (List.mem_cons_self ..))
  have hτresN : evIdx ((asmResChan (p.other, j), true, idx) : Ev)
      (scheduleE sk) < N := by omega
  obtain ⟨hwm, hwlt⟩ := tau_prior W.wf W.m0
    (Sched.walkEventsE_mem_procsE sk W.wf hstg) hwirepair
  have hwm' : ((Chan.wire p.other j, true,
      sk.wiresBefore j ks + cs) : Ev) ∈ scheduleE sk := hwm
  have hwlt' : evIdx ((Chan.wire p.other j, true,
      sk.wiresBefore j ks + cs) : Ev) (scheduleE sk)
      < evIdx ((asmResChan (p.other, j), true, idx) : Ev)
        (scheduleE sk) := hwlt
  -- the frame sends through the D child's wire are all delivered
  have hdel : sk.wiresBefore j ks + cs
      < deliveredCount (s.hist p) j := by
    refine W.delivered_of_send p (mem_allChans_wireOut hstg) hwm' ?_
    omega
  have hlvlrec : ∀ pos, pos ≤ sk.wiresBefore j ks + cs →
      (sk.scopesAt j).getD pos 0 ∈ announcedIds sk p (s.hist p) := by
    intro pos hpos
    have hposlen : pos < (sk.scopesAt j).length := by
      have hb1 := Sched.wiresBefore_succ sk hks
      have hb2 := wiresBefore_le_scopesAt W.wf hj1 hjlt
        (show ks + 1 ≤ sk.stageLen j by omega)
      omega
    exact (announced_of_delivered (mem_peerMintHeights W.wf p hstg)
      hj0 (by omega) hposlen).1
  -- the block prologue chains the census to the stage above
  have hrespair : ([((wireIn (p.other, j), false, ks) : Ev),
      ((asmResChan (p.other, j), true, idx) : Ev)] : List Ev).Sublist
      (Sched.walkEventsE sk (p.other, j)) := by
    unfold Sched.walkEventsE
    refine sublist_flatMap_block hks (prologue_pair
      (chunk_mem_scopeBlockE (by omega) hrmem_chunk) ?_)
    intro hc
    have := congrArg (fun ev : Ev => ev.2.1) hc
    simp at this
  obtain ⟨hr'm, hr'lt⟩ := tau_prior W.wf W.m0
    (Sched.walkEventsE_mem_procsE sk W.wf hstg) hrespair
  have hsp := (walkKeys_stageParty W.wf hstg).1
  have hconv : (Chan.wire (stageParty j).other (j + 1) : Chan)
      = wireIn (p.other, j) := by
    rw [← hsp]
    rfl
  have hcen := census_reach W p
    (evIdx ((wireIn (p.other, j), false, ks) : Ev) (scheduleE sk) + 1)
    j ks hjlt hks
    (by rw [hconv]; exact hr'm)
    (by rw [hconv]; omega)
    (by rw [hconv]; omega)
  obtain ⟨hclen, hsc, -⟩ := hcen
  -- the level-j census below reaches past the D child's position
  have hj' : j - 1 + 1 = j := by omega
  have hsucc := stageScopesA_succ p (s.hist p)
    (show (j - 1) + 1 < sk.rootH from by omega)
  rw [hj'] at hsucc
  have hpreJ := stageScopesA_prefix W.wf p (s.hist p) hjlt
  have hkindsJ : ∀ u ∈ (stageScopesA (aviewOf sk p (s.hist p)) j).1,
      u < sk.scopes.length
      ∧ (aviewOf sk p (s.hist p)).kind? u
        = some ((sk.scope u).kind) := by
    intro u hu
    refine ⟨?_, hpreJ.2 u hu⟩
    exact (mem_scopesAt (hpreJ.1.sublist.mem hu)).1
  have hrecsJ : ∀ i, i ≤ ks →
      (sk.scope ((stageScopesA (aviewOf sk p (s.hist p)) j).1.getD
        i 0)).kind = Kind.D →
      (stageScopesA (aviewOf sk p (s.hist p)) j).1.getD i 0
        ∈ announcedIds sk p (s.hist p) := by
    intro i hi hD
    have hgd : (stageScopesA (aviewOf sk p (s.hist p)) j).1.getD i 0
        = sk.stageScope j i := by
      have := prefix_getD hpreJ.1 (show i
        < (stageScopesA (aviewOf sk p (s.hist p)) j).1.length from by
          omega) 0
      rw [← this]
      rfl
    rw [hgd]
    exact hsc i hi
  have hcov := collect_reach W.wf
    (stageScopesA (aviewOf sk p (s.hist p)) j).1 ks (by omega)
    hkindsJ hrecsJ
  rw [← hsucc] at hcov
  have htakeJ : (stageScopesA (aviewOf sk p (s.hist p)) j).1.take
      (ks + 1) = (sk.stageScopes j).take (ks + 1) := by
    obtain ⟨t, ht⟩ := hpreJ.1
    rw [← ht, List.take_append_of_le_length (by omega)]
  obtain ⟨restJ, -, hflenJ⟩ := bfs_split W.wf hj1 hjlt (ks + 1)
    (by omega)
  have hlow_len : sk.wiresBefore j (ks + 1)
      ≤ (stageScopesA (aviewOf sk p (s.hist p)) (j - 1)).1.length := by
    have := hcov.length_le
    rw [htakeJ] at this
    omega
  -- the announced level-j prefix positions carry the true scopes
  have hpreLow := stageScopesA_prefix W.wf p (s.hist p)
    (show j - 1 < sk.rootH by omega)
  have hlowtake : (stageScopesA (aviewOf sk p (s.hist p)) (j - 1)).1.take
      (sk.wiresBefore j ks + cs + 1)
      = (sk.scopesAt j).take (sk.wiresBefore j ks + cs + 1) := by
    obtain ⟨t, ht⟩ := hpreLow.1
    have hb1 := Sched.wiresBefore_succ sk hks
    have hlen1 : sk.wiresBefore j ks + cs + 1
        ≤ (stageScopesA (aviewOf sk p (s.hist p)) (j - 1)).1.length := by
      omega
    rw [← List.take_append_of_le_length (l₂ := t) hlen1, ht,
      show sk.stageScopes (j - 1) = sk.scopesAt j from by
        unfold Skel.stageScopes
        rw [hj']]
  -- entries through idx are known with true values
  have hraw : asmPendsA (aviewOf sk p (s.hist p)) j
      = if asks p.other j = true then
          (if (j == (aviewOf sk p (s.hist p)).rootH) = true then
            (([0] : List Nat), true)
          else levelA (aviewOf sk p (s.hist p))
            ((aviewOf sk p (s.hist p)).rootH - j)).1.map
            (fun u => match (aviewOf sk p (s.hist p)).rec? u with
              | none => none
              | some sc => some (sc.kids.countP
                  fun v => (aviewOf sk p (s.hist p)).kind? v
                    == some Kind.D))
        else
          ((if (j == (aviewOf sk p (s.hist p)).rootH) = true then
            (([0] : List Nat), true)
          else levelA (aviewOf sk p (s.hist p))
            ((aviewOf sk p (s.hist p)).rootH - j)).1.filter
            (fun u => (aviewOf sk p (s.hist p)).kind? u
              == some Kind.D)).map
            (fun u => match (aviewOf sk p (s.hist p)).rec? u with
              | none => none
              | some sc => some (if (j == 1) = true then sc.leafReqs
                  else sc.kids.length)) := rfl
  have hshape : asmPendsA (aviewOf sk p (s.hist p)) j
      = ((stageScopesA (aviewOf sk p (s.hist p)) (j - 1)).1.filter
          (fun u => (aviewOf sk p (s.hist p)).kind? u
            == some Kind.D)).map
          (fun u => match (aviewOf sk p (s.hist p)).rec? u with
            | none => none
            | some sc => some (if (j == 1) = true then sc.leafReqs
                else sc.kids.length)) := by
    rw [hraw, if_neg (by rw [hside]; simp),
      asm_items_eq p (s.hist p) hj1 hjr]
  -- the filtered prefix through the D child covers idx + 1 entries
  have hpreLow2 := hpreLow
  have hkindsLow : ∀ u ∈ (stageScopesA (aviewOf sk p (s.hist p))
      (j - 1)).1, (aviewOf sk p (s.hist p)).kind? u
        = some ((sk.scope u).kind) := hpreLow.2
  have hfiltake : ((stageScopesA (aviewOf sk p (s.hist p))
      (j - 1)).1.take (sk.wiresBefore j ks + cs + 1)).filter
      (fun u => (aviewOf sk p (s.hist p)).kind? u == some Kind.D)
      = ((sk.scopesAt j).take (sk.wiresBefore j ks + cs + 1)).filter
        (fun u => (sk.scope u).kind == Kind.D) := by
    rw [hlowtake]
    refine List.filter_congr fun u hu => ?_
    have humem : u ∈ (stageScopesA (aviewOf sk p (s.hist p))
        (j - 1)).1 := by
      have hu2 : u ∈ (stageScopesA (aviewOf sk p (s.hist p))
          (j - 1)).1.take (sk.wiresBefore j ks + cs + 1) := by
        rw [hlowtake]
        exact hu
      exact List.mem_of_mem_take hu2
    rw [hkindsLow u humem]
    cases (sk.scope u).kind <;> simp
  have hcount := countD_take_mid W.wf hj1 hjlt hks p.other (cs + 1)
    (by omega)
  have hcount' : (((sk.scopesAt j).take
      (sk.wiresBefore j ks + cs + 1)).filter
      (fun u => (sk.scope u).kind == Kind.D)).length = idx + 1 := by
    rw [show sk.wiresBefore j ks + cs + 1
      = sk.wiresBefore j ks + (cs + 1) from by omega] at *
    rw [hcount]
    rw [if_pos hcsD] at hrsucc
    omega
  have hfillen : idx < ((stageScopesA (aviewOf sk p (s.hist p))
      (j - 1)).1.filter
      (fun u => (aviewOf sk p (s.hist p)).kind? u
        == some Kind.D)).length := by
    have hsub : (((stageScopesA (aviewOf sk p (s.hist p))
        (j - 1)).1.take (sk.wiresBefore j ks + cs + 1)).filter
        (fun u => (aviewOf sk p (s.hist p)).kind? u
          == some Kind.D)).Sublist
        ((stageScopesA (aviewOf sk p (s.hist p)) (j - 1)).1.filter
          (fun u => (aviewOf sk p (s.hist p)).kind? u
            == some Kind.D)) :=
      (List.take_sublist _ _).filter _
    have hlen := hsub.length_le
    rw [hfiltake, hcount'] at hlen
    omega
  have hentries : ∀ m, m ≤ idx →
      (asmPendsA (aviewOf sk p (s.hist p)) j).getD m none
        = some (sk.pendAt p.other j m) := by
    intro m hm
    -- the m-th filtered scope sits inside the covered prefix
    have hmfil : m < (((stageScopesA (aviewOf sk p (s.hist p))
        (j - 1)).1.take (sk.wiresBefore j ks + cs + 1)).filter
        (fun u => (aviewOf sk p (s.hist p)).kind? u
          == some Kind.D)).length := by
      rw [hfiltake, hcount']
      omega
    have hmfilS : m < (((sk.scopesAt j).take
        (sk.wiresBefore j ks + cs + 1)).filter
        (fun u => (sk.scope u).kind == Kind.D)).length := by
      rw [← hfiltake]
      exact hmfil
    have hpref : (((stageScopesA (aviewOf sk p (s.hist p))
        (j - 1)).1.take (sk.wiresBefore j ks + cs + 1)).filter
        (fun u => (aviewOf sk p (s.hist p)).kind? u
          == some Kind.D)) <+: ((stageScopesA (aviewOf sk p
            (s.hist p)) (j - 1)).1.filter
        (fun u => (aviewOf sk p (s.hist p)).kind? u
          == some Kind.D)) :=
      (List.take_prefix _ _).filter _
    -- name the m-th D scope and mint its record
    have hv? : (((sk.scopesAt j).take
        (sk.wiresBefore j ks + cs + 1)).filter
        (fun u => (sk.scope u).kind == Kind.D))[m]?
        = some ((((sk.scopesAt j).take
            (sk.wiresBefore j ks + cs + 1)).filter
            (fun u => (sk.scope u).kind == Kind.D))[m]'hmfilS) :=
      List.getElem?_eq_getElem hmfilS
    have hfull? : ((stageScopesA (aviewOf sk p (s.hist p))
        (j - 1)).1.filter
        (fun u => (aviewOf sk p (s.hist p)).kind? u
          == some Kind.D))[m]?
        = some ((((sk.scopesAt j).take
            (sk.wiresBefore j ks + cs + 1)).filter
            (fun u => (sk.scope u).kind == Kind.D))[m]'hmfilS) := by
      obtain ⟨t, ht⟩ := hpref
      rw [← ht, List.getElem?_append_left hmfil, hfiltake]
      exact hv?
    have hvann : (((sk.scopesAt j).take
        (sk.wiresBefore j ks + cs + 1)).filter
        (fun u => (sk.scope u).kind == Kind.D))[m]'hmfilS
        ∈ announcedIds sk p (s.hist p) := by
      have hvmem : (((sk.scopesAt j).take
          (sk.wiresBefore j ks + cs + 1)).filter
          (fun u => (sk.scope u).kind == Kind.D))[m]'hmfilS
          ∈ (sk.scopesAt j).take (sk.wiresBefore j ks + cs + 1) :=
        (List.mem_filter.mp (List.getElem_mem hmfilS)).1
      obtain ⟨pos, hpos, hposeq⟩ := List.getElem_of_mem hvmem
      have hposlt : pos < sk.wiresBefore j ks + cs + 1 := by
        have := List.length_take_le (sk.wiresBefore j ks + cs + 1)
          (sk.scopesAt j)
        omega
      have hposlen : pos < (sk.scopesAt j).length := by
        rw [List.length_take] at hpos
        omega
      have hposval : (sk.scopesAt j).getD pos 0
          = (((sk.scopesAt j).take
            (sk.wiresBefore j ks + cs + 1)).filter
            (fun u => (sk.scope u).kind == Kind.D))[m]'hmfilS := by
        rw [← hposeq, List.getElem_take, List.getD_eq_getElem?_getD,
          List.getElem?_eq_getElem hposlen]
        rfl
      rw [← hposval]
      exact hlvlrec pos (by omega)
    have hrec : (aviewOf sk p (s.hist p)).rec?
        ((((sk.scopesAt j).take
          (sk.wiresBefore j ks + cs + 1)).filter
          (fun u => (sk.scope u).kind == Kind.D))[m]'hmfilS)
        = some (sk.scope ((((sk.scopesAt j).take
            (sk.wiresBefore j ks + cs + 1)).filter
            (fun u => (sk.scope u).kind == Kind.D))[m]'hmfilS)) := by
      rw [rec?_aviewOf, if_pos hvann]
    have hsome : (asmPendsA (aviewOf sk p (s.hist p)) j).getD m none
        = some (if (j == 1) = true then
            (sk.scope ((((sk.scopesAt j).take
              (sk.wiresBefore j ks + cs + 1)).filter
              (fun u => (sk.scope u).kind == Kind.D))[m]'hmfilS)).leafReqs
          else (sk.scope ((((sk.scopesAt j).take
              (sk.wiresBefore j ks + cs + 1)).filter
              (fun u => (sk.scope u).kind
                == Kind.D))[m]'hmfilS)).kids.length) := by
      rw [hshape, List.getD_eq_getElem?_getD, List.getElem?_map, hfull?,
        Option.map_some, Option.getD_some]
      show (match (aviewOf sk p (s.hist p)).rec?
          ((((sk.scopesAt j).take
            (sk.wiresBefore j ks + cs + 1)).filter
            (fun u => (sk.scope u).kind == Kind.D))[m]'hmfilS) with
        | none => none
        | some sc => some (if (j == 1) = true then sc.leafReqs
            else sc.kids.length)) = _
      rw [hrec]
    obtain ⟨-, hvals⟩ := asmPendsA_spec (sk := sk) W.wf p (s.hist p)
      hj1 hjr
    have := hvals m _ hsome
    rw [hsome, this]
  -- assemble through the go loop
  show e ∈ peerAsmTraceA.go (asmResChan (p.other, j))
    (asmLevelChan (p.other, j)) (sk.asmOutChan (p.other, j))
    (asmPendsA (aviewOf sk p (s.hist p)) j) 0 0
  refine goAsm_mem (asmPendsA (aviewOf sk p (s.hist p)) j) 0 idx 0
    (Nat.zero_le _) ?_ (fun m _ hm => by
      rw [Nat.sub_zero]
      exact hentries m hm) rfl hidx he
  rw [hshape, List.length_map]
  omega


/-- The assembler minting lemma: every peer assembler event scheduled
below the wall is announced-laid. -/
theorem asm_laid {s : MState} {N : Nat} (W : Wall sk s N) (p : Party)
    {j : Nat} (hpk : (p.other, j) ∈ sk.asmKeys) {e : Ev}
    (he : e ∈ Sched.asmEvents sk (p.other, j))
    (hτ : evIdx e (scheduleE sk) < N) :
    e ∈ peerAsmTraceA (aviewOf sk p (s.hist p)) j := by
  have hTmem := Sched.asmEvents_mem_procsE sk hpk
  have hemem : e ∈ scheduleE sk :=
    (Sched.trace_sublistE sk W.wf W.m0 hTmem).mem he
  unfold Sched.asmEvents at he
  obtain ⟨idx, hidxr, heidx⟩ := List.mem_flatMap.mp he
  have hidx : idx < (sk.asmResList p.other j).length :=
    List.mem_range.mp hidxr
  -- the block head sits at or before e in τ
  have hek' : e ∈ ((asmResChan (p.other, j), false, idx) : Ev)
      :: (((List.range (sk.pendAt p.other j idx)).map fun t =>
          ((asmLevelChan (p.other, j) : Chan), false,
            sk.pendsBefore p.other j idx + t))
        ++ [((sk.asmOutChan (p.other, j) : Chan), true, idx)]) := heidx
  have hrfacts : ((asmResChan (p.other, j), false, idx) : Ev)
      ∈ scheduleE sk
      ∧ evIdx ((asmResChan (p.other, j), false, idx) : Ev)
          (scheduleE sk) ≤ evIdx e (scheduleE sk) := by
    rcases pair_of_mem_cons hek' with heq | hpair
    · subst heq
      exact ⟨hemem, Nat.le_refl _⟩
    · have hlift : ([((asmResChan (p.other, j), false, idx) : Ev), e]
          : List Ev).Sublist (Sched.asmEvents sk (p.other, j)) := by
        unfold Sched.asmEvents
        exact sublist_flatMap_block (List.mem_range.mp hidxr) hpair
      obtain ⟨hm, hlt⟩ := tau_prior W.wf W.m0 hTmem hlift
      exact ⟨hm, by omega⟩
  by_cases hside : asks p.other j = true
  · exact asm_laid_asker W p hpk hside hidx heidx hrfacts.1 (by omega)
  · exact asm_laid_answerer W p hpk (by simpa using hside) hidx heidx
      hrfacts.1 (by omega)


-- ================================ the announced-flatten dispatcher

/-- Is this channel internal to party `p`'s PEER endpoint? The closure's
I-step predecessors live on exactly these: no internal channel crosses
the link, so a peer trace's E1/E2 past stays at the peer. -/
def PeerInternal (p : Party) : Chan → Prop
  | .asked q _ => q = p.other
  | .upper q _ => q = p.other
  | .lower q _ => q = p.other
  | .level q _ => q = p.other
  | .leafRequests => p = Party.R
  | .rootres => p = Party.I
  | .rootrets => p = Party.I
  | .rootret => p = Party.R
  | .wire _ _ => False

/-- Lift a laid walk-stage event into the announced flatten. -/
private theorem flatten_of_walk {s : MState} {N : Nat} (W : Wall sk s N)
    (p : Party) {h : Nat} (hpk : (p.other, h) ∈ sk.walkKeys) {e : Ev}
    (he : e ∈ Sched.walkEventsE sk (p.other, h))
    (hτ : evIdx e (scheduleE sk) < N) :
    e ∈ (announcedProcs (aviewOf sk p (s.hist p))).flatten := by
  refine List.mem_flatten.mpr
    ⟨peerWalkTraceA (aviewOf sk p (s.hist p)) h, ?_,
      walk_laid W p hpk he hτ⟩
  unfold announcedProcs
  refine List.mem_append.mpr (.inl (List.mem_append.mpr (.inl
    (List.mem_append.mpr (.inl (List.mem_append.mpr (.inr ?_)))))))
  exact List.mem_map.mpr ⟨h, mem_peerStagesA W.wf p (s.hist p) hpk, rfl⟩

/-- Lift a laid assembler event into the announced flatten. -/
private theorem flatten_of_asm {s : MState} {N : Nat} (W : Wall sk s N)
    (p : Party) {j : Nat} (hpk : (p.other, j) ∈ sk.asmKeys) {e : Ev}
    (he : e ∈ Sched.asmEvents sk (p.other, j))
    (hτ : evIdx e (scheduleE sk) < N) :
    e ∈ (announcedProcs (aviewOf sk p (s.hist p))).flatten := by
  refine List.mem_flatten.mpr
    ⟨peerAsmTraceA (aviewOf sk p (s.hist p)) j, ?_,
      asm_laid W p hpk he hτ⟩
  unfold announcedProcs
  refine List.mem_append.mpr (.inl (List.mem_append.mpr (.inr ?_)))
  exact List.mem_map.mpr ⟨j, mem_peerAsmHeightsA p (s.hist p) hpk, rfl⟩

/-- Lift a laid opener event into the announced flatten. -/
private theorem flatten_of_open {s : MState} {N : Nat} (W : Wall sk s N)
    (p : Party) {e : Ev}
    (he : e ∈ (if p = Party.I then Sched.ropenEvents sk
      else Sched.iopenEvents sk))
    (hτ : evIdx e (scheduleE sk) < N) :
    e ∈ (announcedProcs (aviewOf sk p (s.hist p))).flatten := by
  refine List.mem_flatten.mpr
    ⟨peerOpenTraceA (aviewOf sk p (s.hist p)), ?_, open_laid W p he hτ⟩
  unfold announcedProcs
  refine List.mem_append.mpr (.inl (List.mem_append.mpr (.inl
    (List.mem_append.mpr (.inl (List.mem_append.mpr (.inl ?_)))))))
  exact List.mem_singleton.mpr rfl

/-- Lift a laid absorber event into the announced flatten. -/
private theorem flatten_of_absorb {s : MState} {N : Nat}
    (W : Wall sk s N) (hp : Party.R = Party.R) {e : Ev}
    (he : e ∈ Sched.absorbEvents sk)
    (hτ : evIdx e (scheduleE sk) < N) :
    e ∈ (announcedProcs (aviewOf sk Party.R
      (s.hist Party.R))).flatten := by
  refine List.mem_flatten.mpr
    ⟨peerAbsorbTraceA (aviewOf sk Party.R (s.hist Party.R)), ?_,
      absorb_laid W rfl he hτ⟩
  unfold announcedProcs
  refine List.mem_append.mpr (.inl (List.mem_append.mpr (.inl
    (List.mem_append.mpr (.inr ?_)))))
  exact List.mem_singleton.mpr rfl

/-- Lift a laid finale event into the announced flatten. -/
private theorem flatten_of_fin {s : MState} {N : Nat} (W : Wall sk s N)
    (p : Party) {e : Ev}
    (he : e ∈ (if p = Party.I then Sched.finEvents sk
      else [((Chan.rootret : Chan), false, 0)]))
    (hτ : evIdx e (scheduleE sk) < N) :
    e ∈ (announcedProcs (aviewOf sk p (s.hist p))).flatten := by
  obtain ⟨T, hT, heT⟩ := fin_laid W p he hτ
  refine List.mem_flatten.mpr ⟨T, ?_, heT⟩
  unfold announcedProcs
  exact List.mem_append.mpr (.inr hT)

/-- The flatten dispatcher: any scheduled event below the wall on a
peer-internal channel is announced-laid — the trace decode routes it to
its owning family's laid lemma, and the channel's party pins the family
to the peer endpoint. -/
theorem flatten_of_sched {s : MState} {N : Nat} (W : Wall sk s N)
    (p : Party) {c : Chan} {b : Bool} {n : Nat}
    (hmem : ((c, b, n) : Ev) ∈ scheduleE sk)
    (hτ : evIdx ((c, b, n) : Ev) (scheduleE sk) < N)
    (hchan : PeerInternal p c) :
    ((c, b, n) : Ev)
      ∈ (announcedProcs (aviewOf sk p (s.hist p))).flatten := by
  obtain ⟨T, hT, heT⟩ := mem_evUniv.mp (mem_evUniv_of_mem_scheduleE hmem)
  rcases Sched.procsE_cases sk hT with rfl | rfl | ⟨i, hir, rfl⟩ | rfl
    | ⟨pk, hpk, rfl⟩ | rfl | rfl
  · -- iopen: the root query is the responder's peer opener
    unfold Sched.iopenEvents at heT
    rcases List.mem_cons.mp heT with heq | he2
    · injection heq with h1 h2
      rw [h1] at hchan
      cases hchan
    rcases List.mem_cons.mp he2 with heq | he3
    · have hc : c = Chan.asked Party.I (sk.rootH - 1) := by
        injection heq
      rw [hc] at hchan
      have hp : p = Party.R := by
        cases p
        · exact absurd hchan (fun hc => nomatch hc)
        · rfl
      subst hp
      refine flatten_of_open W Party.R ?_ hτ
      rw [if_neg (by decide)]
      exact heT
    · cases he3
  · -- ropen: the reply-side events are the initiator's peer opener
    have hropen : ∀ x ∈ Sched.ropenEvents sk,
        x.2.1 = false → x.1 = Chan.wire Party.I sk.rootH := by
      intro x hx
      unfold Sched.ropenEvents at hx
      rcases List.mem_cons.mp hx with rfl | hx2
      · intro _; rfl
      rcases List.mem_cons.mp hx2 with rfl | hx3
      · intro hc; cases hc
      rcases List.mem_cons.mp hx3 with rfl | hx4
      · intro hc; cases hc
      · obtain ⟨t, -, rfl⟩ := List.mem_map.mp hx4
        intro hc; cases hc
    have hp : p = Party.I := by
      unfold Sched.ropenEvents at heT
      rcases List.mem_cons.mp heT with heq | he2
      · injection heq with h1 h2
        rw [h1] at hchan
        cases hchan
      rcases List.mem_cons.mp he2 with heq | he3
      · injection heq with h1 h2
        rw [h1] at hchan
        cases hchan
      rcases List.mem_cons.mp he3 with heq | he4
      · have hc : c = Chan.rootres := by injection heq
        rw [hc] at hchan
        exact hchan
      · obtain ⟨t, -, heq⟩ := List.mem_map.mp he4
        have hc : c = Chan.asked Party.R (sk.rootH - 2) := by
          injection heq.symm
        rw [hc] at hchan
        cases hp2 : p
        · rfl
        · rw [hp2] at hchan
          exact absurd hchan (fun hc => nomatch hc)
    subst hp
    refine flatten_of_open W Party.I ?_ hτ
    rw [if_pos rfl]
    exact heT
  · -- a walk stage: the channel's party is the stage's owner
    have hpk := Sched.walkOrder_mem_keys sk W.wf hir
    generalize hpk_def : ((if (sk.rootH - 1 - i) % 2 == 1 then Party.I
        else Party.R), sk.rootH - 1 - i) = pk at hpk heT
    obtain ⟨q', h'⟩ := pk
    have hsupp := (Sched.walkEvents_support sk (q', h')) _
      ((Sched.walkEventsE_perm sk (q', h')).mem_iff.mp heT)
    have hq' : q' = p.other := by
      cases b with
      | true =>
          rcases hsupp.1 rfl with hc | hc | hc | ⟨hc, hne⟩
          · rw [show upperOut (q', h') = Chan.upper q' h' from rfl]
              at hc
            rw [show ((c, true, n) : Ev).1 = c from rfl] at hc
            rw [hc] at hchan
            exact hchan
          · rw [show wireOut (q', h') = Chan.wire q' h' from rfl] at hc
            rw [show ((c, true, n) : Ev).1 = c from rfl] at hc
            rw [hc] at hchan
            cases hchan
          · rw [show lowerOut (q', h') = Chan.lower q' h' from rfl]
              at hc
            rw [show ((c, true, n) : Ev).1 = c from rfl] at hc
            rw [hc] at hchan
            exact hchan
          · rw [show ((c, true, n) : Ev).1 = c from rfl] at hc
            rw [askedOut] at hc
            by_cases h2 : h' < 2
            · rw [if_pos h2] at hc
              -- leafRequests: the query stage is (I, 1)
              rw [hc] at hchan
              have hI : q' = Party.I := by
                obtain ⟨h1, hpar⟩ := Sched.walkKeys_parity sk W.wf hpk
                rcases hpar with ⟨hq, -⟩ | ⟨hq, hpar⟩
                · exact hq
                · -- (R, 0): the leaf stage launches no queries
                  exfalso
                  have h0 : h' = 0 := by omega
                  exact hne h0
              rw [hchan, hI]
              rfl
            · rw [if_neg h2] at hc
              rw [hc] at hchan
              exact hchan
      | false =>
          rcases hsupp.2 rfl with hc | hc
          · rw [show wireIn (q', h')
              = Chan.wire q'.other (h' + 1) from rfl] at hc
            rw [show ((c, false, n) : Ev).1 = c from rfl] at hc
            rw [hc] at hchan
            cases hchan
          · rw [show askedIn (q', h') = Chan.asked q' h' from rfl] at hc
            rw [show ((c, false, n) : Ev).1 = c from rfl] at hc
            rw [hc] at hchan
            exact hchan
    subst hq'
    exact flatten_of_walk W p hpk heT hτ
  · -- the absorber: initiator-side, the responder's peer process
    have hp : p = Party.R := by
      unfold Sched.absorbEvents at heT
      obtain ⟨jj, -, hj⟩ := List.mem_flatMap.mp heT
      rcases List.mem_cons.mp hj with heq | hj2
      · injection heq with h1 h2
        rw [h1] at hchan
        cases hchan
      rcases List.mem_cons.mp hj2 with heq | hj3
      · have hc : c = Chan.leafRequests := by injection heq
        rw [hc] at hchan
        exact hchan
      rcases List.mem_cons.mp hj3 with heq | hj4
      · have hc : c = Chan.level Party.I 0 := by injection heq
        rw [hc] at hchan
        cases hp2 : p
        · rw [hp2] at hchan
          exact absurd hchan (fun hc => nomatch hc)
        · rfl
      · cases hj4
    subst hp
    exact flatten_of_absorb W rfl heT hτ
  · -- an assembler: the channel's party is the assembler's owner
    obtain ⟨q', j⟩ := pk
    have hsupp := (Sched.asmEvents_support sk (q', j)) _ heT
    have hq' : q' = p.other := by
      cases b with
      | true =>
          have hc := hsupp.1 rfl
          rw [show ((c, true, n) : Ev).1 = c from rfl] at hc
          rw [Skel.asmOutChan] at hc
          by_cases h1 : (q' == Party.I && j == sk.rootH) = true
          · rw [if_pos h1] at hc
            rw [hc] at hchan
            simp only [Bool.and_eq_true, beq_iff_eq] at h1
            rw [h1.1]
            cases hp2 : p
            · rw [hp2] at hchan
              exact absurd hchan (fun hc => nomatch hc)
            · rfl
          · rw [if_neg h1] at hc
            by_cases h2 : (q' == Party.R && j == sk.rootH - 1) = true
            · rw [if_pos h2] at hc
              rw [hc] at hchan
              simp only [Bool.and_eq_true, beq_iff_eq] at h2
              rw [h2.1]
              cases hp2 : p
              · rfl
              · rw [hp2] at hchan
                exact absurd hchan (fun hc => nomatch hc)
            · rw [if_neg h2] at hc
              rw [hc] at hchan
              exact hchan
      | false =>
          rcases hsupp.2 rfl with hc | hc
          · rw [show ((c, false, n) : Ev).1 = c from rfl] at hc
            rw [asmResChan] at hc
            by_cases ha : asks q' j = true
            · rw [if_pos ha] at hc
              rw [hc] at hchan
              exact hchan
            · rw [if_neg ha] at hc
              rw [hc] at hchan
              exact hchan
          · rw [show ((c, false, n) : Ev).1 = c from rfl] at hc
            rw [show asmLevelChan (q', j) = Chan.level q' (j - 1)
              from rfl] at hc
            rw [hc] at hchan
            exact hchan
    subst hq'
    exact flatten_of_asm W p hpk heT hτ
  · -- the floating rootret receive: the responder's peer finale
    have heq := List.mem_singleton.mp heT
    have hc : c = Chan.rootret := by injection heq
    rw [hc] at hchan
    have hp : p = Party.R := hchan
    subst hp
    refine flatten_of_fin W Party.R ?_ hτ
    rw [if_neg (by decide)]
    exact heT
  · -- fins: the initiator's peer finale
    have hp : p = Party.I := by
      unfold Sched.finEvents at heT
      rcases List.mem_cons.mp heT with heq | he2
      · have hc : c = Chan.rootres := by injection heq
        rw [hc] at hchan
        exact hchan
      · obtain ⟨t, -, heq⟩ := List.mem_map.mp he2
        have hc : c = Chan.rootrets := by injection heq.symm
        rw [hc] at hchan
        exact hchan
    subst hp
    refine flatten_of_fin W Party.I ?_ hτ
    rw [if_pos rfl]
    exact heT


/-- An announced stage names a peer walk key. -/
private theorem walkKeys_of_peerStagesA (hwf : sk.wellFormed = true)
    (p : Party) (tr : List MObs) {h : Nat}
    (hh : h ∈ peerStagesA (aviewOf sk p tr)) :
    (p.other, h) ∈ sk.walkKeys := by
  have hev : sk.rootH % 2 = 0 := (wf_rootH hwf).1
  unfold peerStagesA at hh
  cases p with
  | I =>
      rw [if_pos (show (((aviewOf sk Party.I tr).party == Party.I)
        = true) from rfl)] at hh
      obtain ⟨k, hk, hke⟩ := List.mem_map.mp hh
      rw [List.mem_range] at hk
      have hkr : (aviewOf sk Party.I tr).rootH = sk.rootH := rfl
      rw [hkr] at hk hke
      exact Sched.mem_walkKeys_of sk hwf (by omega)
        (Or.inr ⟨rfl, by omega⟩)
  | R =>
      rw [if_neg (show ¬ (((aviewOf sk Party.R tr).party == Party.I)
        = true) from fun hc => nomatch hc)] at hh
      obtain ⟨k, hk, hke⟩ := List.mem_map.mp hh
      rw [List.mem_range] at hk
      have hkr : (aviewOf sk Party.R tr).rootH = sk.rootH := rfl
      rw [hkr] at hk hke
      exact Sched.mem_walkKeys_of sk hwf (by omega)
        (Or.inl ⟨rfl, by omega⟩)

/-- An announced assembler height names a peer assembler key. -/
private theorem asmKeys_of_peerAsmHeightsA (p : Party) (tr : List MObs)
    {j : Nat} (hj : j ∈ peerAsmHeightsA (aviewOf sk p tr)) :
    (p.other, j) ∈ sk.asmKeys := by
  unfold peerAsmHeightsA at hj
  unfold Skel.asmKeys
  cases p with
  | I =>
      rw [if_pos (show (((aviewOf sk Party.I tr).party == Party.I)
        = true) from rfl)] at hj
      obtain ⟨m, hm, hme⟩ := List.mem_map.mp hj
      rw [List.mem_range] at hm
      refine List.mem_append.mpr (.inr ?_)
      refine List.mem_map.mpr ⟨m, List.mem_range.mpr hm, ?_⟩
      rw [Prod.mk.injEq]
      exact ⟨rfl, hme⟩
  | R =>
      rw [if_neg (show ¬ (((aviewOf sk Party.R tr).party == Party.I)
        = true) from fun hc => nomatch hc)] at hj
      obtain ⟨m, hm, hme⟩ := List.mem_map.mp hj
      rw [List.mem_range] at hm
      refine List.mem_append.mpr (.inl ?_)
      refine List.mem_map.mpr ⟨m, List.mem_range.mpr hm, ?_⟩
      rw [Prod.mk.injEq]
      exact ⟨rfl, hme⟩

/-- Announced-flatten events live on wires or peer-internal channels:
no internal channel crosses the link. -/
theorem peer_internal_of_flatten (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    (p : Party) (tr : List MObs) {e : Ev}
    (he : e ∈ (announcedProcs (aviewOf sk p tr)).flatten)
    (hnw : isWire e.1 = false) : PeerInternal p e.1 := by
  obtain ⟨TA, hTA, heTA⟩ := List.mem_flatten.mp he
  unfold announcedProcs at hTA
  rcases List.mem_append.mp hTA with hTA | hfin
  rcases List.mem_append.mp hTA with hTA | hasm
  rcases List.mem_append.mp hTA with hTA | habs
  rcases List.mem_append.mp hTA with hopen | hwalk
  · -- the peer opener
    have hTo : TA = peerOpenTraceA (aviewOf sk p tr) := by
      simpa using hopen
    subst hTo
    have hpre := peerOpenTraceA_prefix hwf p tr
    cases p with
    | I =>
        rw [if_pos rfl] at hpre
        have heT := hpre.sublist.mem heTA
        unfold Sched.ropenEvents at heT
        obtain ⟨c, b, n⟩ := e
        rcases List.mem_cons.mp heT with heq | he2
        · have hc : c = Chan.wire Party.I sk.rootH := by injection heq
          rw [hc] at hnw
          cases hnw
        rcases List.mem_cons.mp he2 with heq | he3
        · have hc : c = Chan.wire Party.R sk.rootH := by injection heq
          rw [hc] at hnw
          cases hnw
        rcases List.mem_cons.mp he3 with heq | he4
        · have hc : c = Chan.rootres := by injection heq
          rw [hc]
          rfl
        · obtain ⟨t, -, heq⟩ := List.mem_map.mp he4
          have hc : c = Chan.asked Party.R (sk.rootH - 2) := by
            injection heq.symm
          rw [hc]
          rfl
    | R =>
        rw [if_neg (by simp)] at hpre
        have heT := hpre.sublist.mem heTA
        unfold Sched.iopenEvents at heT
        obtain ⟨c, b, n⟩ := e
        rcases List.mem_cons.mp heT with heq | he2
        · have hc : c = Chan.wire Party.I sk.rootH := by injection heq
          rw [hc] at hnw
          cases hnw
        rcases List.mem_cons.mp he2 with heq | he3
        · have hc : c = Chan.asked Party.I (sk.rootH - 1) := by
            injection heq
          rw [hc]
          rfl
        · cases he3
  · -- a peer walk stage
    obtain ⟨h, hh, hTe⟩ := List.mem_map.mp hwalk
    have hpk := walkKeys_of_peerStagesA hwf p tr hh
    have hhlt : h < sk.rootH := (walkKeys_stageParty hwf hpk).2
    have hpre := peerWalkTraceA_prefix hwf p tr hhlt
    rw [← hTe] at heTA
    have heT : e ∈ Sched.walkEventsE sk (p.other, h) :=
      hpre.sublist.mem heTA
    have hsupp := (Sched.walkEvents_support sk (p.other, h)) _
      ((Sched.walkEventsE_perm sk (p.other, h)).mem_iff.mp heT)
    obtain ⟨c, b, n⟩ := e
    cases b with
    | true =>
        rcases hsupp.1 rfl with hc | hc | hc | ⟨hc, hne⟩
        · rw [show ((c, true, n) : Ev).1 = c from rfl] at hc
          rw [show upperOut (p.other, h) = Chan.upper p.other h
            from rfl] at hc
          rw [hc]
          rfl
        · rw [show ((c, true, n) : Ev).1 = c from rfl] at hc
          rw [show wireOut (p.other, h) = Chan.wire p.other h
            from rfl] at hc
          rw [show ((c, true, n) : Ev).1 = c from rfl,
            hc] at hnw
          cases hnw
        · rw [show ((c, true, n) : Ev).1 = c from rfl] at hc
          rw [show lowerOut (p.other, h) = Chan.lower p.other h
            from rfl] at hc
          rw [hc]
          rfl
        · rw [show ((c, true, n) : Ev).1 = c from rfl] at hc
          rw [askedOut] at hc
          by_cases h2 : (p.other, h).2 < 2
          · rw [if_pos h2] at hc
            rw [hc]
            -- leafRequests rides only the initiator's stage 1
            obtain ⟨hlt2, hpar⟩ := Sched.walkKeys_parity sk hwf hpk
            show p = Party.R
            rcases hpar with ⟨hq, hodd⟩ | ⟨hq, heven⟩
            · cases hp2 : p
              · rw [hp2] at hq
                exact absurd hq (by decide)
              · rfl
            · exfalso
              have h0 : h = 0 := by
                simp only at h2 hne
                omega
              exact hne h0
          · rw [if_neg h2] at hc
            rw [hc]
            rfl
    | false =>
        rcases hsupp.2 rfl with hc | hc
        · rw [show ((c, false, n) : Ev).1 = c from rfl] at hc
          rw [show wireIn (p.other, h)
            = Chan.wire (p.other).other (h + 1) from rfl] at hc
          rw [show ((c, false, n) : Ev).1 = c from rfl, hc] at hnw
          cases hnw
        · rw [show ((c, false, n) : Ev).1 = c from rfl] at hc
          rw [show askedIn (p.other, h) = Chan.asked p.other h
            from rfl] at hc
          rw [hc]
          rfl
  · -- the absorber
    have hTa : TA = peerAbsorbTraceA (aviewOf sk p tr) := by
      simpa using habs
    subst hTa
    cases p with
    | I =>
        exfalso
        unfold peerAbsorbTraceA at heTA
        rw [if_pos (show (((aviewOf sk Party.I tr).party == Party.I)
          = true) from rfl)] at heTA
        cases heTA
    | R =>
        have hpre := peerAbsorbTraceA_prefix hwf Party.R tr
        have heT := hpre.sublist.mem heTA
        unfold Sched.absorbEvents at heT
        obtain ⟨jj, -, hj⟩ := List.mem_flatMap.mp heT
        obtain ⟨c, b, n⟩ := e
        rcases List.mem_cons.mp hj with heq | hj2
        · have hc : c = Chan.wire Party.R 0 := by injection heq
          rw [show ((c, b, n) : Ev).1 = c from rfl, hc] at hnw
          cases hnw
        rcases List.mem_cons.mp hj2 with heq | hj3
        · have hc : c = Chan.leafRequests := by injection heq
          rw [hc]
          rfl
        rcases List.mem_cons.mp hj3 with heq | hj4
        · have hc : c = Chan.level Party.I 0 := by injection heq
          rw [hc]
          rfl
        · cases hj4
  · -- a peer assembler
    obtain ⟨j, hj, hTe⟩ := List.mem_map.mp hasm
    have hpk := asmKeys_of_peerAsmHeightsA p tr hj
    obtain ⟨hj1, hjr, -⟩ := asmKeys_bounds hpk
    have hpre := peerAsmTraceA_prefix hwf p tr hj1 hjr
    rw [← hTe] at heTA
    have heT : e ∈ Sched.asmEvents sk (p.other, j) :=
      hpre.sublist.mem heTA
    have hsupp := (Sched.asmEvents_support sk (p.other, j)) _ heT
    obtain ⟨c, b, n⟩ := e
    cases b with
    | true =>
        have hc := hsupp.1 rfl
        rw [show ((c, true, n) : Ev).1 = c from rfl] at hc
        rw [Skel.asmOutChan] at hc
        by_cases h1 : (p.other == Party.I && j == sk.rootH) = true
        · rw [if_pos h1] at hc
          rw [hc]
          simp only [Bool.and_eq_true, beq_iff_eq] at h1
          show p = Party.R
          cases hp2 : p
          · rw [hp2] at h1
            exact absurd h1.1 (by decide)
          · rfl
        · rw [if_neg h1] at hc
          by_cases h2 : (p.other == Party.R && j == sk.rootH - 1)
              = true
          · rw [if_pos h2] at hc
            rw [hc]
            simp only [Bool.and_eq_true, beq_iff_eq] at h2
            show p = Party.I
            cases hp2 : p
            · rfl
            · rw [hp2] at h2
              exact absurd h2.1 (by decide)
          · rw [if_neg h2] at hc
            rw [hc]
            rfl
    | false =>
        rcases hsupp.2 rfl with hc | hc
        · rw [show ((c, false, n) : Ev).1 = c from rfl] at hc
          rw [asmResChan] at hc
          by_cases ha : asks p.other j = true
          · rw [if_pos ha] at hc
            rw [hc]
            rfl
          · rw [if_neg ha] at hc
            rw [hc]
            rfl
        · rw [show ((c, false, n) : Ev).1 = c from rfl] at hc
          rw [show asmLevelChan (p.other, j)
            = Chan.level p.other (j - 1) from rfl] at hc
          rw [hc]
          rfl
  · -- the peer finale
    have hpre := peerFinTracesA_prefix hwf p tr TA hfin
    cases p with
    | I =>
        rw [if_pos rfl] at hpre
        have heT := hpre.sublist.mem heTA
        unfold Sched.finEvents at heT
        obtain ⟨c, b, n⟩ := e
        rcases List.mem_cons.mp heT with heq | he2
        · have hc : c = Chan.rootres := by injection heq
          rw [hc]
          rfl
        · obtain ⟨t, -, heq⟩ := List.mem_map.mp he2
          have hc : c = Chan.rootrets := by injection heq.symm
          rw [hc]
          rfl
    | R =>
        rw [if_neg (by simp)] at hpre
        have heT := hpre.sublist.mem heTA
        obtain ⟨c, b, n⟩ := e
        have heq := List.mem_singleton.mp heT
        have hc : c = Chan.rootret := by injection heq
        rw [hc]
        rfl


/-- Own pushed frames are announced-universe members. -/
private theorem mem_evUnivA_own_push {p : Party} {tr : List MObs}
    {g n : Nat} (hgm : g ∈ wireHeights sk p)
    (hn : n < pushedCount tr g) :
    ((Chan.wire p g, true, n) : Ev) ∈ evUnivA (aviewOf sk p tr) tr := by
  unfold evUnivA
  refine List.mem_append.mpr (.inl (List.mem_append.mpr (.inl
    (List.mem_append.mpr (.inl ?_)))))
  refine List.mem_flatMap.mpr ⟨g, ?_, ?_⟩
  · rw [show (aviewOf sk p tr).party = p from rfl, wireHeightsA_aviewOf]
    exact hgm
  · exact List.mem_map.mpr ⟨n, List.mem_range.mpr hn, rfl⟩

/-- Delivered peer frames are announced-universe members. -/
private theorem mem_evUnivA_peer_send {p : Party} {tr : List MObs}
    {g n : Nat} (hgm : g ∈ wireHeights sk p.other)
    (hn : n < deliveredCount tr g) :
    ((Chan.wire p.other g, true, n) : Ev)
      ∈ evUnivA (aviewOf sk p tr) tr := by
  unfold evUnivA
  refine List.mem_append.mpr (.inl (List.mem_append.mpr (.inl
    (List.mem_append.mpr (.inr ?_)))))
  refine List.mem_flatMap.mpr ⟨g, ?_, ?_⟩
  · rw [show (aviewOf sk p tr).party = p from rfl, wireHeightsA_aviewOf]
    exact hgm
  · exact List.mem_map.mpr ⟨n, List.mem_range.mpr hn, rfl⟩

-- =============================== Step 4: the causal coverage induction

/-- The first occurrence splits off the `takeWhile` prefix (private
copy of SigmaStarLive's device). -/
private theorem dropWhile_first' {l : List Ev} {e : Ev} (he : e ∈ l) :
    ∃ rest, l.dropWhile (fun x => !(x == e)) = e :: rest := by
  induction l with
  | nil => cases he
  | cons a t ih =>
      by_cases hae : a = e
      · subst hae
        exact ⟨t, by simp⟩
      · rw [List.dropWhile_cons, if_pos (by simp [hae])]
        rcases List.mem_cons.mp he with rfl | het
        · exact absurd rfl hae
        · exact ih het

/-- Causal coverage (refute-c1 §2.4 over the ANNOUNCED closure): at the
stuck drained wall, every announced-laid event scheduled below the wall
enters the causal closure by its own τ stage — wire-send E1/E2
predecessors are grounded against the drained counts, internal
predecessors are announced-laid by the minting lemmas and recurse, and
the E3 past is the announced trace itself, τ-below by trace order.

The bound `k` drives the strong induction; instantiate `k := N`. -/
theorem causal_closure_coverage {s : MState} {N : Nat} (W : Wall sk s N)
    (p : Party) :
    ∀ (k : Nat), ∀ e ∈ scheduleE sk, evIdx e (scheduleE sk) < N →
      evIdx e (scheduleE sk) < k →
      e ∈ (announcedProcs (aviewOf sk p (s.hist p))).flatten →
      e ∈ closureNA (aviewOf sk p (s.hist p)) (s.hist p)
          (evUnivA (aviewOf sk p (s.hist p)) (s.hist p))
          (announcedProcs (aviewOf sk p (s.hist p)))
          (evIdx e (scheduleE sk) + 1) := by
  intro k
  induction k with
  | zero =>
      intro e _ _ hk
      omega
  | succ k ih =>
      intro e he hN hk hfl
      have hu : e ∈ evUnivA (aviewOf sk p (s.hist p)) (s.hist p) := by
        unfold evUnivA
        exact List.mem_append.mpr (.inr hfl)
      obtain ⟨c, b, n⟩ := e
      by_cases hpw : (isWire c && b) = true
      · -- a wire send: grounded against the drained counts
        rw [Bool.and_eq_true] at hpw
        obtain ⟨hw, rfl⟩ := hpw
        obtain ⟨q, g, rfl⟩ := isWire_eq hw
        have hch : Chan.wire q g ∈ allChans sk :=
          evUniv_wire_mem W.wf (mem_evUniv_of_mem_scheduleE he)
        have hsent := W.sent_of_below he hN
        have hg : groundedA (aviewOf sk p (s.hist p)) (s.hist p)
            (Chan.wire q g, true, n) = true := by
          refine groundedA_of_push ?_
          rw [groundedPush]
          simp only [isWire, wireParty, wireHeight, Bool.true_and]
          by_cases hqp : q = p
          · subst hqp
            rw [if_pos (show (q == (aviewOf sk q
              (s.hist q)).party) = true from by
                show (q == q) = true
                simp)]
            have hpe := W.drained_pushed q g hch
            exact decide_eq_true (by omega)
          · have hqo : q = p.other := by
              cases q <;> cases p <;>
                first
                  | rfl
                  | (exact absurd rfl hqp)
                  | (exfalso; exact hqp rfl)
            subst hqo
            rw [if_neg (by
              intro hcon
              exact hqp (beq_iff_eq.mp hcon))]
            have hdel := W.drained_delivered p.other g hch
            rw [Party.other_other] at hdel
            exact decide_eq_true (by omega)
        have h0 : ((Chan.wire q g, true, n) : Ev)
            ∈ closureNA (aviewOf sk p (s.hist p)) (s.hist p)
              (evUnivA (aviewOf sk p (s.hist p)) (s.hist p))
              (announcedProcs (aviewOf sk p (s.hist p))) 0 :=
          List.mem_filter.mpr ⟨hu, hg⟩
        exact closureNA_le (Nat.zero_le _) _ h0
      · -- a non-push event: I-step against the τ-stage below
        have hstep : istepOkA (aviewOf sk p (s.hist p))
            (announcedProcs (aviewOf sk p (s.hist p)))
            (closureNA (aviewOf sk p (s.hist p)) (s.hist p)
              (evUnivA (aviewOf sk p (s.hist p)) (s.hist p))
              (announcedProcs (aviewOf sk p (s.hist p)))
              (evIdx ((c, b, n) : Ev) (scheduleE sk)))
            (c, b, n) = true := by
          rw [istepOkA]
          simp only [Bool.and_eq_true]
          refine ⟨⟨⟨by simp [hpw], ?_⟩, ?_⟩, ?_⟩
          · -- E1: the receive's send is τ-below or grounded
            cases b with
            | true => simp
            | false =>
                rw [Bool.false_or]
                obtain ⟨hsm, hτs⟩ := tau_e1 W.wf he
                refine (List.contains_iff_mem ..).mpr ?_
                by_cases hwc : isWire c = true
                · -- a wire send: grounded evidence with delivered
                  -- membership in the announced universe
                  obtain ⟨q, g, rfl⟩ := isWire_eq hwc
                  have hch : Chan.wire q g ∈ allChans sk :=
                    evUniv_wire_mem W.wf
                      (mem_evUniv_of_mem_scheduleE hsm)
                  have hsent := W.sent_of_below hsm (by omega)
                  have hgm : g ∈ wireHeights sk q :=
                    wireHeights_of_allChans hch
                  have hg : groundedA (aviewOf sk p (s.hist p))
                      (s.hist p) (Chan.wire q g, true, n) = true := by
                    refine groundedA_of_push ?_
                    rw [groundedPush]
                    simp only [isWire, wireParty, wireHeight,
                      Bool.true_and]
                    by_cases hqp : q = p
                    · subst hqp
                      rw [if_pos (show (q == (aviewOf sk q
                        (s.hist q)).party) = true from by
                          show (q == q) = true
                          simp)]
                      have hpe := W.drained_pushed q g hch
                      exact decide_eq_true (by omega)
                    · have hqo : q = p.other := by
                        cases q <;> cases p <;>
                          first
                            | rfl
                            | (exact absurd rfl hqp)
                            | (exfalso; exact hqp rfl)
                      subst hqo
                      rw [if_neg (by
                        intro hcon
                        exact hqp (beq_iff_eq.mp hcon))]
                      have hdel := W.drained_delivered p.other g hch
                      rw [Party.other_other] at hdel
                      exact decide_eq_true (by omega)
                  have husend : ((Chan.wire q g, true, n) : Ev)
                      ∈ evUnivA (aviewOf sk p (s.hist p))
                        (s.hist p) := by
                    by_cases hqp : q = p
                    · subst hqp
                      refine mem_evUnivA_own_push hgm ?_
                      have hpe := W.drained_pushed q g hch
                      omega
                    · have hqo : q = p.other := by
                        cases q <;> cases p <;>
                          first
                            | rfl
                            | (exact absurd rfl hqp)
                            | (exfalso; exact hqp rfl)
                      subst hqo
                      refine mem_evUnivA_peer_send hgm ?_
                      have hdel := W.drained_delivered p.other g hch
                      rw [Party.other_other] at hdel
                      omega
                  have h0 : ((Chan.wire q g, true, n) : Ev)
                      ∈ closureNA (aviewOf sk p (s.hist p)) (s.hist p)
                        (evUnivA (aviewOf sk p (s.hist p)) (s.hist p))
                        (announcedProcs (aviewOf sk p (s.hist p))) 0 :=
                    List.mem_filter.mpr ⟨husend, hg⟩
                  exact closureNA_le (Nat.zero_le _) _ h0
                · -- an internal send: announced-laid, recurse
                  have hchan := peer_internal_of_flatten W.wf W.m0 p
                    (s.hist p) hfl (by simpa using hwc)
                  have hflS := flatten_of_sched W p hsm (by omega)
                    hchan
                  have hin := ih _ hsm (by omega) (by omega) hflS
                  exact closureNA_le (by omega) _ hin
          · -- E2: the send's cap-window receive is τ-below
            cases b with
            | false => simp
            | true =>
                have hwc : isWire c = false := by
                  cases hcw : isWire c with
                  | false => rfl
                  | true =>
                      exfalso
                      rw [Bool.and_eq_true] at hpw
                      exact hpw ⟨hcw, rfl⟩
                rw [capA_aviewOf]
                by_cases hcap : n < sk.cap c
                · simp [hcap]
                · obtain ⟨hrm, hτr⟩ := tau_e2 W.wf he (by omega)
                  rw [Bool.or_eq_true]
                  refine Or.inr ((List.contains_iff_mem ..).mpr ?_)
                  have hchan := peer_internal_of_flatten W.wf W.m0 p
                    (s.hist p) hfl hwc
                  have hflR := flatten_of_sched W p hrm (by omega)
                    hchan
                  have hin := ih _ hrm (by omega) (by omega) hflR
                  exact closureNA_le (by omega) _ hin
          · -- E3: the announced trace past is τ-below, elementwise
            rw [List.all_eq_true]
            intro T hT
            rw [Bool.or_eq_true]
            by_cases heT : ((c, b, n) : Ev) ∈ T
            · refine Or.inr ?_
              rw [List.all_eq_true]
              intro x hx
              obtain ⟨T', hT', hpre⟩ := announcedProcs_prefix W.wf p
                (s.hist p) T hT
              have hxT : x ∈ T :=
                (List.takeWhile_prefix _).sublist.mem hx
              have hxfl : x ∈ (announcedProcs (aviewOf sk p
                  (s.hist p))).flatten :=
                List.mem_flatten.mpr ⟨T, hT, hxT⟩
              have hxm : x ∈ scheduleE sk :=
                (Sched.trace_sublistE sk W.wf W.m0 hT').mem
                  (hpre.sublist.mem hxT)
              -- x precedes e in the true trace
              obtain ⟨tail, htail⟩ := dropWhile_first' heT
              have hpair : ([x, ((c, b, n) : Ev)] : List Ev).Sublist
                  T := by
                have hsplit := List.takeWhile_append_dropWhile
                  (p := fun y => !(y == ((c, b, n) : Ev))) (l := T)
                rw [htail] at hsplit
                rw [← hsplit]
                have h1 : ([x] : List Ev).Sublist
                    (T.takeWhile (fun y => !(y == ((c, b, n) : Ev)))) :=
                  List.singleton_sublist.mpr hx
                have h2 : ([((c, b, n) : Ev)] : List Ev).Sublist
                    (((c, b, n) : Ev) :: tail) :=
                  List.singleton_sublist.mpr (List.mem_cons_self ..)
                exact List.Sublist.append h1 h2
              have hτx : evIdx x (scheduleE sk)
                  < evIdx ((c, b, n) : Ev) (scheduleE sk) :=
                tau_lt_of_trace_pair W.wf W.m0 hT'
                  (hpair.trans hpre.sublist)
              have hin := ih _ hxm (by omega) (by omega) hxfl
              refine (List.contains_iff_mem ..).mpr ?_
              exact closureNA_le (by omega) _ hin
            · refine Or.inl ?_
              rw [Bool.not_eq_true']
              cases hcont : T.contains ((c, b, n) : Ev) with
              | false => rfl
              | true =>
                  exact absurd ((List.contains_iff_mem ..).mp hcont)
                    heT
        show ((c, b, n) : Ev) ∈ closureStepA (aviewOf sk p (s.hist p))
          (s.hist p) (evUnivA (aviewOf sk p (s.hist p)) (s.hist p))
          (announcedProcs (aviewOf sk p (s.hist p))) _
        refine List.mem_filter.mpr ⟨hu, ?_⟩
        rw [Bool.or_eq_true, Bool.or_eq_true]
        exact Or.inr hstep


-- ======================================= the coverage theorem itself

/-- A scheduled receive on the party's own stream is announced-laid:
its consumer is a peer process (the responder opening, the stage below,
or the absorber), and the corresponding laid lemma covers it. -/
private theorem flatten_of_own_wire_recv {s : MState} {N : Nat}
    (W : Wall sk s N) (p : Party) {g n : Nat}
    (hmem : ((Chan.wire p g, false, n) : Ev) ∈ scheduleE sk)
    (hτ : evIdx ((Chan.wire p g, false, n) : Ev) (scheduleE sk) < N) :
    ((Chan.wire p g, false, n) : Ev)
      ∈ (announcedProcs (aviewOf sk p (s.hist p))).flatten := by
  obtain ⟨T, hT, heT⟩ := mem_evUniv.mp (mem_evUniv_of_mem_scheduleE hmem)
  rcases Sched.procsE_cases sk hT with rfl | rfl | ⟨i, hir, rfl⟩ | rfl
    | ⟨pk, hpk, rfl⟩ | rfl | rfl
  · -- iopen holds no wire receives
    exfalso
    unfold Sched.iopenEvents at heT
    rcases List.mem_cons.mp heT with heq | he2
    · have := congrArg (fun ev : Ev => ev.2.1) heq
      simp at this
    rcases List.mem_cons.mp he2 with heq | he3
    · have := congrArg (fun ev : Ev => ev.2.1) heq
      simp at this
    · cases he3
  · -- ropen consumes the initiator's opening
    have hp : p = Party.I := by
      unfold Sched.ropenEvents at heT
      rcases List.mem_cons.mp heT with heq | he2
      · have hc : Chan.wire p g = Chan.wire Party.I sk.rootH := by
          injection heq
        exact (Chan.wire.inj hc).1
      · exfalso
        rcases List.mem_cons.mp he2 with heq | he3
        · have := congrArg (fun ev : Ev => ev.2.1) heq
          simp at this
        rcases List.mem_cons.mp he3 with heq | he4
        · have := congrArg (fun ev : Ev => ev.2.1) heq
          simp at this
        · obtain ⟨t, -, heq⟩ := List.mem_map.mp he4
          have := congrArg (fun ev : Ev => ev.2.1) heq.symm
          simp at this
    subst hp
    refine flatten_of_open W Party.I ?_ hτ
    rw [if_pos rfl]
    exact heT
  · -- a walk stage: its input is the stage above's output stream
    have hpk := Sched.walkOrder_mem_keys sk W.wf hir
    generalize hpk_def : ((if (sk.rootH - 1 - i) % 2 == 1 then Party.I
        else Party.R), sk.rootH - 1 - i) = pk at hpk heT
    obtain ⟨q', h'⟩ := pk
    have hsupp := (Sched.walkEvents_support sk (q', h')) _
      ((Sched.walkEventsE_perm sk (q', h')).mem_iff.mp heT)
    have hq' : q' = p.other := by
      rcases hsupp.2 rfl with hc | hc
      · rw [show ((Chan.wire p g, false, n) : Ev).1 = Chan.wire p g
          from rfl] at hc
        rw [show wireIn (q', h') = Chan.wire q'.other (h' + 1)
          from rfl] at hc
        have hqq := (Chan.wire.inj hc).1
        cases q' <;> cases p <;>
          first
            | rfl
            | (exact absurd hqq (by decide))
      · exfalso
        rw [show ((Chan.wire p g, false, n) : Ev).1 = Chan.wire p g
          from rfl] at hc
        rw [show askedIn (q', h') = Chan.asked q' h' from rfl] at hc
        cases hc
    subst hq'
    exact flatten_of_walk W p hpk heT hτ
  · -- the absorber consumes the responder's supplies
    have hp : p = Party.R := by
      unfold Sched.absorbEvents at heT
      obtain ⟨jj, -, hj⟩ := List.mem_flatMap.mp heT
      rcases List.mem_cons.mp hj with heq | hj2
      · have hc : Chan.wire p g = Chan.wire Party.R 0 := by
          injection heq
        exact (Chan.wire.inj hc).1
      · exfalso
        rcases List.mem_cons.mp hj2 with heq | hj3
        · have hc : Chan.wire p g = Chan.leafRequests := by
            injection heq
          cases hc
        rcases List.mem_cons.mp hj3 with heq | hj4
        · have := congrArg (fun ev : Ev => ev.2.1) heq
          simp at this
        · cases hj4
    subst hp
    exact flatten_of_absorb W rfl heT hτ
  · -- assemblers touch no wires
    exfalso
    obtain ⟨q', j⟩ := pk
    have hsupp := (Sched.asmEvents_support sk (q', j)) _ heT
    rcases hsupp.2 rfl with hc | hc
    · rw [show ((Chan.wire p g, false, n) : Ev).1 = Chan.wire p g
        from rfl] at hc
      rw [asmResChan] at hc
      by_cases ha : asks q' j = true
      · rw [if_pos ha] at hc
        cases hc
      · rw [if_neg ha] at hc
        cases hc
    · rw [show ((Chan.wire p g, false, n) : Ev).1 = Chan.wire p g
        from rfl] at hc
      rw [show asmLevelChan (q', j) = Chan.level q' (j - 1)
        from rfl] at hc
      cases hc
  · -- nor the floating rootret receive
    exfalso
    have heq := List.mem_singleton.mp heT
    have hc : Chan.wire p g = Chan.rootret := by injection heq
    cases hc
  · -- nor the finale
    exfalso
    unfold Sched.finEvents at heT
    rcases List.mem_cons.mp heT with heq | he2
    · have hc : Chan.wire p g = Chan.rootres := by injection heq
      cases hc
    · obtain ⟨t, -, heq⟩ := List.mem_map.mp he2
      have hc : Chan.wire p g = Chan.rootrets := by injection heq.symm
      cases hc

/-- Step 4, discharged: `CausalStuckCoverage` holds for every
well-formed margin-0 skeleton — at a reachable stuck drained
σ*-causal×σ*-causal state, a held stream whose τ-prefix is performed
is proven-demanded under the ANNOUNCED closure.

The withheld frame's predecessor receive is scheduled τ-below the
withheld send (the wire's unit-capacity E2 edge), the minting ladder
lays it in the announced family, the coverage induction walks it into
the causal closure by its own τ stage, and saturation absorbs the
stage into `inevitableA`. A root-height hold is vacuous here: a
committed opener hand means the opening frame has not flushed, so
nothing was ever pushed on that stream. -/
theorem causalStuckCoverage (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) :
    CausalStuckCoverage sk := by
  intro C s hr hstuck hpI hpR p hh hhold hcover
  have hm := sinv_reachable hwf hr
  have W : Wall sk s (evIdx ((Chan.wire p hh, true,
      sentOf sk s.base (Chan.wire p hh)) : Ev) (scheduleE sk)) :=
    ⟨hwf, hm0, hm.mux, hpI, hpR, hcover⟩
  rw [demandedA, Bool.or_eq_true]
  by_cases hK : pushedCount (s.hist p) hh = 0
  · exact Or.inl (by simpa using hK)
  refine Or.inr ((List.contains_iff_mem ..).mpr ?_)
  have hL : InvL sk .impl s.base := hm.mux.invl
  have hch : Chan.wire p hh ∈ allChans sk :=
    mem_allChans_of_wireHeights (holdsWire_mem_wireHeights hhold)
  have hKs : pushedCount (s.hist p) hh
      = sentOf sk s.base (Chan.wire p hh) := W.drained_pushed p hh hch
  -- the held stream is a walk stream: an opener hand pins sentOf = 0
  rw [holdsWire.eq_def] at hhold
  by_cases hhr : (hh == sk.rootH) = true
  · exfalso
    rw [if_pos hhr] at hhold
    have hhr' : hh = sk.rootH := beq_iff_eq.mp hhr
    have htop := hL.top
    unfold topLocalOk at htop
    simp only [Bool.and_eq_true] at htop
    cases p with
    | I =>
        have hio : s.base.iopenCh = some IOblig.wire :=
          beq_iff_eq.mp hhold
        have hc1 := htop.1.1.1.1.1.1.1.1.1.1.1
        rw [Bool.or_eq_true, bne_iff_ne] at hc1
        have hnw : s.base.iopenWire = false := by
          rcases hc1 with hne | hnw
          · exact absurd hio hne
          · rwa [Bool.not_eq_true'] at hnw
        have hz : sentOf sk s.base (Chan.wire Party.I hh) = 0 := by
          show (if (hh == sk.rootH) = true then
            (if (Party.I == Party.I) = true then b2n s.base.iopenWire
             else b2n s.base.ropenWire)
          else wkWireSent sk s.base (Party.I, hh)) = 0
          rw [if_pos hhr,
            if_pos (show ((Party.I == Party.I) = true) from rfl), hnw]
          rfl
        rw [hz] at hKs
        exact hK (by omega)
    | R =>
        have hro : s.base.ropenCh = some ROblig.wire :=
          beq_iff_eq.mp hhold
        have hc6 := htop.1.1.1.1.1.1.2
        rw [Bool.or_eq_true, bne_iff_ne] at hc6
        have hnw : s.base.ropenWire = false := by
          rcases hc6 with hne | hnw
          · exact absurd hro hne
          · rwa [Bool.not_eq_true'] at hnw
        have hz : sentOf sk s.base (Chan.wire Party.R hh) = 0 := by
          show (if (hh == sk.rootH) = true then
            (if (Party.R == Party.I) = true then b2n s.base.iopenWire
             else b2n s.base.ropenWire)
          else wkWireSent sk s.base (Party.R, hh)) = 0
          rw [if_pos hhr,
            if_neg (show ¬ ((Party.R == Party.I) = true) by decide),
            hnw]
          rfl
        rw [hz] at hKs
        exact hK (by omega)
  · -- the walk hand's pending fire places the withheld send
    rw [if_neg hhr] at hhold
    simp only [Bool.and_eq_true] at hhold
    obtain ⟨⟨hcon, hph⟩, hcm⟩ := hhold
    have hpk : (p, hh) ∈ sk.walkKeys :=
      (List.contains_iff_mem ..).mp hcon
    have hph2 : (s.base.walk (p, hh)).phase = 2 := by
      simpa using hph
    obtain ⟨i, hcmi⟩ : ∃ i, (s.base.walk (p, hh)).committed
        = some (Oblig.wire i) := by
      cases hcmv : (s.base.walk (p, hh)).committed with
      | none => rw [hcmv] at hcm; cases hcm
      | some o =>
          cases o with
          | wire i => exact ⟨i, rfl⟩
          | res i => rw [hcmv] at hcm; cases hcm
          | query i => rw [hcmv] at hcm; cases hcm
          | parent => rw [hcmv] at hcm; cases hcm
    have hwk : Sched.wkPend sk s.base (p, hh)
        = [((wireOut (p, hh), true,
            sk.wiresBefore hh (s.base.walk (p, hh)).scope + i),
           Action.walkFire (p, hh))] := by
      simp [Sched.wkPend, hph2, hcmi]
    have hpmem : ((wireOut (p, hh), true,
        sk.wiresBefore hh (s.base.walk (p, hh)).scope + i),
        Action.walkFire (p, hh)) ∈ Sched.pends sk s.base := by
      refine (Sched.pends_lift sk).2.2.1 (p, hh) hpk _ ?_
      rw [hwk]
      exact List.mem_singleton.mpr rfl
    have hioh := mstuck_ioh (sk := sk) hstuck
    have hroh := mstuck_roh (sk := sk) hL hstuck
    have hwkh := mstuck_wkh hwf hL hstuck rfl
    obtain ⟨hok, T, pre, suf, hT, hdec, -⟩ :=
      Sched.pends_soundE sk hwf hL hioh hroh hwkh _ hpmem
    have hseq : sk.wiresBefore hh (s.base.walk (p, hh)).scope + i
        = sentOf sk s.base (Chan.wire p hh) := by
      have hs := hok.seq
      simp only [] at hs
      exact hs
    have hfmem : ((Chan.wire p hh, true,
        sentOf sk s.base (Chan.wire p hh)) : Ev) ∈ scheduleE sk := by
      have hfT : ((wireOut (p, hh), true,
          sk.wiresBefore hh (s.base.walk (p, hh)).scope + i) : Ev)
          ∈ T := by
        rw [hdec]
        exact List.mem_append.mpr (.inr (List.mem_cons_self ..))
      have := (Sched.trace_sublistE sk hwf hm0 hT).mem hfT
      rw [show ((wireOut (p, hh), true,
          sk.wiresBefore hh (s.base.walk (p, hh)).scope + i) : Ev)
          = ((Chan.wire p hh, true,
              sentOf sk s.base (Chan.wire p hh)) : Ev) from by
        rw [Prod.mk.injEq, Prod.mk.injEq]
        exact ⟨rfl, rfl, hseq⟩] at this
      exact this
    -- the predecessor receive, τ-below the withheld send
    have hcap1 : sk.cap (Chan.wire p hh) = 1 := rfl
    obtain ⟨hrmem, hrτ⟩ := tau_e2 hwf hfmem (by
      rw [hcap1]
      omega)
    rw [hcap1] at hrmem hrτ
    have hfl := flatten_of_own_wire_recv W p hrmem hrτ
    have hcov := causal_closure_coverage W p
      (evIdx ((Chan.wire p hh, false,
        sentOf sk s.base (Chan.wire p hh) - 1) : Ev) (scheduleE sk) + 1)
      _ hrmem hrτ (by omega) hfl
    have hinev := mem_inevitableA_of_closureNA hcov
    rw [show (aviewOf sk p (s.hist p)).party = p from rfl, hKs]
    exact hinev

end StreamingMirror.Mux
