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

end StreamingMirror.Mux
