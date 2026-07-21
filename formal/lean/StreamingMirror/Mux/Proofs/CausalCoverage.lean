/-
The causal coverage layer (MUX-PROGRESS §4, the σ*-locality residue's
liveness half): what the ANNOUNCED closure (`Mux/Causal.lean`) proves at
a stuck, drained state — every event τ-below the chase's withheld push
enters `inevitableA`, so the withheld frame is `demandedA` and σ*-causal
cannot be idling.

# The plan (refute-c1 §2 re-run at the causal grain)

The omniscient liveness proof (Proofs/SigmaStarLive.lean) runs: push
certificates drain the pipes through the keystone (Step 1), the chase
names the τ-least withheld push (Steps 2–3), and the coverage induction
proves it demanded (Step 4). This file re-runs Steps 1 and 4 with every
closure read routed through the announced view:

- **The A-closure theory** mirrors Chase/Closure.lean verbatim: the
  saturation chain is monotone, and members invert to grounded evidence
  or an I-step against the closure itself (`inevitableA_inv`).
- **The receive ledger** (`RecvLedger`) is the one genuinely new ground
  fact: a machine's recorded receive actions never outrun its base
  consumer counts, which is what makes the C-own evidence arm of
  `groundedA` (own performed receives) sound at a stuck state. It is
  strategy-generic and rides the same per-arm Steps decomposition as
  `SInv`.
- **The transcription lemmas** cash the module-doc claim of
  Mux/Causal.lean that announced traces are literal prefixes of the
  true `.impl` traces: each `announcedProcs` entry is a prefix of its
  peer process's `procsE` trace, because the layouts read the true
  records (`aviewOf` stores `sk.scope u` verbatim) and truncate at the
  first unannounced quantity.
- **The causal keystone** re-runs Chase/Keystone.lean over
  `inevitableA`: at a stuck state every event of a push-time causal
  closure has been performed. The grounded walls gain the receive
  ledger; the I-step trace decode routes through the announced-prefix
  property instead of `evUniv` membership.
- **The minting ladder and coverage** re-run SigmaStarLive's Step 4:
  at a drained stuck state every peer-trace event τ-below the withheld
  push is announced-laid — every consulted record's minting arrival
  sits τ-below the consulting event, walked as E1/trace-order hops up
  the stage ladder — and then enters `inevitableA` by its own τ stage.

The deliverable at the bottom is `causal_coverage`: at any reachable
`mstuck` state of the σ*-causal×σ*-causal composition with pipes
drained, the chase's withheld push is `demandedA`. CausalLive.lean
assembles it with Steps 1–3 into `sigmaStarCausal_deadlock_free`; C1's
charter refutation then drops its liveness hypothesis (the F3 statement
of record becomes unconditional).
-/
import StreamingMirror.Mux.Causal

namespace StreamingMirror.Mux

open Model
open Sched (Ev)

variable {sk : Skel}

-- ================================================ the A-closure theory
-- Chase/Closure.lean's lemma suite, re-proven over the announced
-- closure. The chain is parametric in the universe and trace family,
-- so every lemma is stated that way and instantiated at `inevitableA`.

/-- Every causal closure stage stays inside its universe. -/
theorem closureNA_subset_univ {av : AView} {tr : List MObs}
    {univ : List Ev} {procsL : List (List Ev)} :
    ∀ n, ∀ e ∈ closureNA av tr univ procsL n, e ∈ univ := by
  intro n e he
  cases n with
  | zero => exact (List.mem_filter.mp he).1
  | succ n => exact (List.mem_filter.mp he).1

/-- Each causal saturation pass keeps its members. -/
theorem closureNA_le_succ {av : AView} {tr : List MObs} {univ : List Ev}
    {procsL : List (List Ev)} (n : Nat) :
    ∀ e ∈ closureNA av tr univ procsL n,
      e ∈ closureNA av tr univ procsL (n + 1) := by
  intro e he
  show e ∈ closureStepA av tr univ procsL (closureNA av tr univ procsL n)
  refine List.mem_filter.mpr
    ⟨closureNA_subset_univ n e he, ?_⟩
  rw [Bool.or_eq_true, Bool.or_eq_true]
  exact Or.inl (Or.inl ((List.contains_iff_mem ..).mpr he))

/-- The causal saturation chain is increasing. -/
theorem closureNA_le {av : AView} {tr : List MObs} {univ : List Ev}
    {procsL : List (List Ev)} {m n : Nat} (hmn : m ≤ n) :
    ∀ e ∈ closureNA av tr univ procsL m,
      e ∈ closureNA av tr univ procsL n := by
  induction n with
  | zero =>
      intro e he
      have : m = 0 := by omega
      exact this ▸ he
  | succ n ih =>
      intro e he
      by_cases hlast : m = n + 1
      · exact hlast ▸ he
      · exact closureNA_le_succ n e (ih (by omega) e he)

/-- A causal I-step member is never a push. -/
theorem istepOkA_not_push {av : AView} {procsL : List (List Ev)}
    {D : List Ev} {e : Ev} (h : istepOkA av procsL D e = true) :
    (isWire e.1 && e.2.1) = false := by
  rw [istepOkA] at h
  simp only [Bool.and_eq_true] at h
  have := h.1.1.1
  rwa [Bool.not_eq_true'] at this

/-- A causal I-step receive's send is a member. -/
theorem istepOkA_e1 {av : AView} {procsL : List (List Ev)} {D : List Ev}
    {e : Ev} (h : istepOkA av procsL D e = true) (hb : e.2.1 = false) :
    (e.1, true, e.2.2) ∈ D := by
  rw [istepOkA] at h
  simp only [Bool.and_eq_true] at h
  have := h.1.1.2
  rw [hb] at this
  simp only [Bool.false_or] at this
  exact (List.contains_iff_mem ..).mp this

/-- A causal I-step send's cap-window predecessor is a member, past the
free window. -/
theorem istepOkA_e2 {av : AView} {procsL : List (List Ev)} {D : List Ev}
    {e : Ev} (h : istepOkA av procsL D e = true) (hb : e.2.1 = true)
    (hcap : ¬ e.2.2 < capA av e.1) :
    (e.1, false, e.2.2 - capA av e.1) ∈ D := by
  rw [istepOkA] at h
  simp only [Bool.and_eq_true] at h
  have := h.1.2
  rw [hb] at this
  simp only [Bool.not_true, Bool.false_or, Bool.or_eq_true,
    decide_eq_true_eq] at this
  rcases this with hlt | hmem
  · exact absurd hlt hcap
  · exact (List.contains_iff_mem ..).mp hmem

/-- A causal I-step member's whole announced-trace past is a member
set. -/
theorem istepOkA_prefix {av : AView} {procsL : List (List Ev)}
    {D : List Ev} {e : Ev} (h : istepOkA av procsL D e = true)
    {T : List Ev} (hT : T ∈ procsL) (heT : e ∈ T) :
    ∀ x ∈ T.takeWhile (fun x => !(x == e)), x ∈ D := by
  rw [istepOkA] at h
  simp only [Bool.and_eq_true] at h
  have := List.all_eq_true.mp h.2 T hT
  rw [Bool.or_eq_true] at this
  rcases this with hnc | hall
  · rw [Bool.not_eq_true', ← Bool.not_eq_true] at hnc
    exact absurd ((List.contains_iff_mem ..).mpr heT) hnc
  · intro x hx
    exact (List.contains_iff_mem ..).mp (List.all_eq_true.mp hall x hx)

/-- The causal I-step check is monotone in the candidate set. -/
theorem istepOkA_mono {av : AView} {procsL : List (List Ev)}
    {D D' : List Ev} (hsub : ∀ x ∈ D, x ∈ D') {e : Ev}
    (h : istepOkA av procsL D e = true) :
    istepOkA av procsL D' e = true := by
  rw [istepOkA] at h ⊢
  simp only [Bool.and_eq_true] at h ⊢
  obtain ⟨⟨⟨hnp, he1⟩, he2⟩, he3⟩ := h
  have hc : ∀ x : Ev, D.contains x = true → D'.contains x = true := by
    intro x hx
    exact (List.contains_iff_mem ..).mpr
      (hsub x ((List.contains_iff_mem ..).mp hx))
  refine ⟨⟨⟨hnp, ?_⟩, ?_⟩, ?_⟩
  · rw [Bool.or_eq_true] at he1 ⊢
    rcases he1 with hb | hm
    · exact Or.inl hb
    · exact Or.inr (hc _ hm)
  · rw [Bool.or_eq_true, Bool.or_eq_true] at he2 ⊢
    rcases he2 with (hb | hlt) | hm
    · exact Or.inl (Or.inl hb)
    · exact Or.inl (Or.inr hlt)
    · exact Or.inr (hc _ hm)
  · rw [List.all_eq_true] at he3 ⊢
    intro T hT
    have := he3 T hT
    rw [Bool.or_eq_true] at this ⊢
    rcases this with hnc | hall
    · exact Or.inl hnc
    · refine Or.inr ?_
      rw [List.all_eq_true] at hall ⊢
      intro x hx
      exact hc _ (hall x hx)

/-- Causal evidence is causally inevitable. -/
theorem certifiedA_subset_inevitableA {av : AView} {tr : List MObs} :
    ∀ e ∈ certifiedA av tr, e ∈ inevitableA av tr :=
  closureNA_le (Nat.zero_le _)

/-- Causally inevitable events live in the announced universe. -/
theorem inevitableA_subset_univ {av : AView} {tr : List MObs} :
    ∀ e ∈ inevitableA av tr, e ∈ evUnivA av tr :=
  closureNA_subset_univ _

/-- The causal closure inversion: a member is grounded causal evidence
or passes the causal I-step check against the closure itself.

The causal keystone's induction handle, exactly as `inevitable_inv` is
the omniscient keystone's: monotonicity lifts the entry stage to the
saturated set, so consumers never track stages. -/
theorem inevitableA_inv {av : AView} {tr : List MObs} {e : Ev}
    (he : e ∈ inevitableA av tr) :
    groundedA av tr e = true
      ∨ istepOkA av (announcedProcs av) (inevitableA av tr) e = true := by
  suffices h : ∀ n, ∀ e ∈ closureNA av tr (evUnivA av tr)
      (announcedProcs av) n,
      groundedA av tr e = true
        ∨ istepOkA av (announcedProcs av)
            (closureNA av tr (evUnivA av tr) (announcedProcs av) n) e
          = true by
    rcases h _ e he with hg | hstep
    · exact Or.inl hg
    · exact Or.inr hstep
  intro n
  induction n with
  | zero =>
      intro e he
      exact Or.inl (List.mem_filter.mp he).2
  | succ n ih =>
      intro e he
      obtain ⟨-, hcond⟩ := List.mem_filter.mp he
      rw [Bool.or_eq_true, Bool.or_eq_true] at hcond
      rcases hcond with (hmem | hg) | hstep
      · rcases ih e ((List.contains_iff_mem ..).mp hmem) with hg | hstep
        · exact Or.inl hg
        · exact Or.inr (istepOkA_mono (closureNA_le_succ n) hstep)
      · exact Or.inl hg
      · exact Or.inr (istepOkA_mono (closureNA_le_succ n) hstep)

-- ============================================== the announced-view decode
-- What `aviewOf` actually stores: the true records of the announced
-- ids and the true kinds of their kids. Every transcription lemma
-- reduces to these three facts plus the id decode of `announcedIds`.

/-- `capA` over a skeleton's announced view is the skeleton's capacity
function: `Skel.cap` reads nothing but `capLevel`, which the view
carries verbatim. -/
theorem capA_aviewOf (p : Party) (tr : List MObs) :
    capA (aviewOf sk p tr) = sk.cap := by
  funext c
  cases c <;> rfl

/-- A `lookup` hit names a member entry. -/
private theorem mem_of_lookup {α β : Type _} [BEq α] [LawfulBEq α]
    {l : List (α × β)} {a : α} {b : β} (h : l.lookup a = some b) :
    (a, b) ∈ l := by
  obtain ⟨l₁, l₂, rfl, -⟩ := List.lookup_eq_some_iff.mp h
  exact List.mem_append.mpr (.inr (List.mem_cons_self ..))

/-- Looking up a key in a graph-of-a-function pair list yields the
function's value exactly on the key set. -/
private theorem lookup_map_graph {α β : Type _} [BEq α] [LawfulBEq α]
    (f : α → β) (u : α) :
    ∀ (l : List α),
      (l.map fun v => (v, f v)).lookup u
        = if u ∈ l then some (f u) else none := by
  intro l
  induction l with
  | nil => simp
  | cons a l ih =>
      by_cases hua : u = a
      · subst hua
        simp
      · have hbeq : (u == a) = false := by simpa using hua
        simp [List.lookup_cons, hbeq, ih, List.mem_cons, hua]

/-- The announced record table stores the true records: `rec?` answers
`sk.scope u` exactly on the announced ids. -/
theorem rec?_aviewOf (p : Party) (tr : List MObs) (u : Nat) :
    (aviewOf sk p tr).rec? u
      = if u ∈ announcedIds sk p tr then some (sk.scope u) else none := by
  show ((announcedIds sk p tr).eraseDups.map
      fun v => (v, sk.scope v)).lookup u = _
  rw [lookup_map_graph]
  by_cases hmem : u ∈ announcedIds sk p tr
  · rw [if_pos hmem, if_pos (List.mem_eraseDups.mpr hmem)]
  · rw [if_neg hmem, if_neg (fun hc => hmem (List.mem_eraseDups.mp hc))]

/-- Every entry of the announced kind stratum carries the true kind of
its key. -/
private theorem kinds_aviewOf_entry {p : Party} {tr : List MObs}
    {v : Nat} {k : Kind}
    (hmem : (v, k) ∈ (aviewOf sk p tr).kinds) :
    k = (sk.scope v).kind := by
  have hmem' : (v, k) ∈ (announcedIds sk p tr).eraseDups.flatMap
      fun u => (sk.scope u).kids.map fun w => (w, (sk.scope w).kind) :=
    hmem
  obtain ⟨u, -, hin⟩ := List.mem_flatMap.mp hmem'
  obtain ⟨w, -, heq⟩ := List.mem_map.mp hin
  injection heq with h1 h2
  rw [h1] at h2
  exact h2.symm

/-- The root's record is a dispute, extracted from `wellFormed`. -/
theorem wf_root_kind {sk : Skel} (hwf : sk.wellFormed = true) :
    (sk.scope 0).kind = Kind.D := by
  unfold Skel.wellFormed at hwf
  simp only [Bool.and_eq_true, beq_iff_eq] at hwf
  exact hwf.1.1.1.1.1.1.1.2

/-- A `kind?` hit is the true kind (the root's hardwired `D` included,
via well-formedness). -/
theorem kind?_aviewOf_eq {p : Party} {tr : List MObs} {v : Nat} {k : Kind}
    (hwf : sk.wellFormed = true)
    (hk : (aviewOf sk p tr).kind? v = some k) :
    k = (sk.scope v).kind := by
  rw [AView.kind?] at hk
  by_cases hv : v = 0
  · subst hv
    rw [if_pos (show ((0 : Nat) == 0) = true from rfl)] at hk
    injection hk with hk
    rw [← hk, wf_root_kind hwf]
  · rw [if_neg (by simpa using hv)] at hk
    exact kinds_aviewOf_entry (mem_of_lookup hk)

/-- A keyed entry makes `lookup` answer (some entry's value). -/
private theorem lookup_isSome_of_mem {α β : Type _} [BEq α] [LawfulBEq α]
    {l : List (α × β)} {a : α} {b : β} (h : (a, b) ∈ l) :
    ∃ c, l.lookup a = some c := by
  have hs : (l.lookup a).isSome = true :=
    List.lookup_isSome_iff.mpr ⟨(a, b), h, by simp⟩
  exact Option.isSome_iff_exists.mp hs

/-- An announced scope's kids all have announced kinds: the kid-label
stratum is minted with the parent's record. -/
theorem kind?_aviewOf_of_kid {p : Party} {tr : List MObs} {u v : Nat}
    (hwf : sk.wellFormed = true)
    (hu : u ∈ announcedIds sk p tr) (hv : v ∈ (sk.scope u).kids) :
    (aviewOf sk p tr).kind? v = some ((sk.scope v).kind) := by
  rw [AView.kind?]
  by_cases hv0 : v = 0
  · subst hv0
    rw [if_pos (show ((0 : Nat) == 0) = true from rfl),
      wf_root_kind hwf]
  · rw [if_neg (by simpa using hv0)]
    have hentry : (v, (sk.scope v).kind) ∈ (aviewOf sk p tr).kinds := by
      show (v, (sk.scope v).kind)
        ∈ (announcedIds sk p tr).eraseDups.flatMap
          fun u => (sk.scope u).kids.map fun w => (w, (sk.scope w).kind)
      exact List.mem_flatMap.mpr ⟨u, List.mem_eraseDups.mpr hu,
        List.mem_map.mpr ⟨v, hv, rfl⟩⟩
    obtain ⟨k, hk⟩ := lookup_isSome_of_mem hentry
    rw [hk, kinds_aviewOf_entry (mem_of_lookup hk)]

-- ================================================== the minting decode
-- Which delivery counts announce which scope ids: the positional B5
-- decode of `announcedIds`, inverted into the three minting rules the
-- ladder walks.

/-- The peer stream heights `announcedIds` decodes, standalone. -/
def peerMintHeights (sk : Skel) (p : Party) : List Nat :=
  if p == Party.I then
    (List.range (sk.rootH / 2)).map fun k => sk.rootH - 2 - 2 * k
  else
    (List.range (sk.rootH / 2)).map fun k => sk.rootH - 1 - 2 * k

/-- The opening arrival mints the root (rule 1, both parties). -/
theorem announced_root {p : Party} {tr : List MObs}
    (hdel : 0 < deliveredCount tr sk.rootH) :
    0 ∈ announcedIds sk p tr := by
  rw [announcedIds]
  refine List.mem_append.mpr (.inl ?_)
  rw [if_pos (by simpa using hdel)]
  exact List.mem_cons_self ..

/-- The opening arrival mints the root's kids on the initiator side
(rule 1's second half: the responder's reply answers the stage below,
so its arrival announces the root listing). -/
theorem announced_root_kids {tr : List MObs} {v : Nat}
    (hdel : 0 < deliveredCount tr sk.rootH)
    (hv : v ∈ (sk.scope 0).kids) :
    v ∈ announcedIds sk Party.I tr := by
  rw [announcedIds]
  refine List.mem_append.mpr (.inl ?_)
  rw [if_pos (by simpa using hdel)]
  refine List.mem_cons_of_mem _ ?_
  rw [if_pos (show (Party.I == Party.I) = true from rfl)]
  exact hv

/-- Frame `n` on a peer stream mints its about-scope and that scope's
kids (rule 2): the `n`-th scope of level `h`, positionally. -/
theorem announced_of_delivered {p : Party} {tr : List MObs} {h n : Nat}
    (hh : h ∈ peerMintHeights sk p) (hh0 : h ≠ 0)
    (hn : n < deliveredCount tr h) (hlen : n < (sk.scopesAt h).length) :
    (sk.scopesAt h).getD n 0 ∈ announcedIds sk p tr
      ∧ ∀ v ∈ (sk.scope ((sk.scopesAt h).getD n 0)).kids,
          v ∈ announcedIds sk p tr := by
  have hmem : ∀ x ∈ (sk.scopesAt h).getD n 0
        :: (sk.scope ((sk.scopesAt h).getD n 0)).kids,
      x ∈ announcedIds sk p tr := by
    intro x hx
    rw [announcedIds]
    refine List.mem_append.mpr (.inr ?_)
    rw [show (if p == Party.I then
        (List.range (sk.rootH / 2)).map fun k => sk.rootH - 2 - 2 * k
      else
        (List.range (sk.rootH / 2)).map fun k => sk.rootH - 1 - 2 * k)
        = peerMintHeights sk p from rfl]
    refine List.mem_flatMap.mpr ⟨h, hh, ?_⟩
    rw [if_neg (by simpa using hh0)]
    refine List.mem_flatMap.mpr ⟨n, ?_, hx⟩
    exact List.mem_range.mpr (by omega)
  exact ⟨hmem _ (List.mem_cons_self ..),
    fun v hv => hmem v (List.mem_cons_of_mem _ hv)⟩

/-- Party filtering over one party's tagged range keeps everything on
a party match. -/
private theorem filterMap_party_hit (q : Party) (f : Nat → Nat) :
    ∀ l : List Nat,
      (l.map fun k => (q, f k)).filterMap
        (fun pk : Party × Nat => if pk.1 == q then some pk.2 else none)
      = l.map f := by
  intro l
  induction l with
  | nil => rfl
  | cons a l ih =>
      rw [List.map_cons, List.filterMap_cons_some (b := f a) (by simp),
        ih, List.map_cons]

/-- Party filtering over the other party's tagged range drops
everything. -/
private theorem filterMap_party_miss {q q' : Party}
    (hne : (q == q') = false) (f : Nat → Nat) :
    ∀ l : List Nat,
      (l.map fun k => (q, f k)).filterMap
        (fun pk : Party × Nat => if pk.1 == q' then some pk.2 else none)
      = [] := by
  intro l
  induction l with
  | nil => rfl
  | cons a l ih =>
      rw [List.map_cons, List.filterMap_cons_none (by simp [hne]), ih]

/-- The announced wire-height lists are the true ones: `wireHeightsA`
reads only the session parameters, which the view carries verbatim. -/
theorem wireHeightsA_aviewOf (p : Party) (tr : List MObs) (q : Party) :
    wireHeightsA (aviewOf sk p tr) q = wireHeights sk q := by
  show (sk.rootH ::
      (if (q == Party.I) = true then
        (List.range (sk.rootH / 2)).map fun k => sk.rootH - 1 - 2 * k
      else
        (List.range (sk.rootH / 2)).map fun k => sk.rootH - 2 - 2 * k))
    = wireHeights sk q
  rw [wireHeights, Skel.walkKeys, List.filterMap_append]
  cases q with
  | I =>
      rw [filterMap_party_hit, filterMap_party_miss rfl,
        List.append_nil, if_pos (show (Party.I == Party.I) = true from rfl)]
  | R =>
      rw [filterMap_party_hit, filterMap_party_miss rfl,
        List.nil_append,
        if_neg (show ¬ (Party.R == Party.I) = true from by simp)]

-- ============================================= the causal evidence decode

/-- Causal evidence names a push-grounded wire send or one of the
machine's own performed wire receives. -/
theorem groundedA_inv {av : AView} {tr : List MObs} {e : Ev}
    (hg : groundedA av tr e = true) :
    groundedPush av.party tr e = true
      ∨ ∃ h n, e = (Chan.wire av.party.other h, false, n)
          ∧ n < ownRecvCount av tr h := by
  unfold groundedA at hg
  rw [Bool.or_eq_true] at hg
  rcases hg with hp | hown
  · exact Or.inl hp
  · refine Or.inr ?_
    split at hown
    next q h n =>
      simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq]
        at hown
      exact ⟨h, n, by rw [hown.1], hown.2⟩
    next => cases hown

/-- Push evidence is causal evidence. -/
theorem groundedA_of_push {av : AView} {tr : List MObs} {e : Ev}
    (hp : groundedPush av.party tr e = true) :
    groundedA av tr e = true := by
  unfold groundedA
  rw [Bool.or_eq_true]
  exact Or.inl hp

-- ==================================================== the receive ledger
-- The C-own evidence's ground fact: recorded receive actions never
-- outrun the base consumer counts. Strategy-generic, preserved by the
-- same per-arm Steps decomposition as `SInv`.

/-- Does this action record a wire receive on party `p`'s stream-`h`
demux slot? The counting pattern of `ownRecvCount`, exposed for the
per-arm neutrality side conditions (cf. `wireCommitOn`). -/
def recvActOn (rootH : Nat) (p : Party) (a : Action) (h : Nat) : Bool :=
  match a with
  | .walkRecvWire pk => pk == (p, h - 1) && h != 0
  | .ropenRecv => p == Party.R && h == rootH
  | .absorbRecvWire => p == Party.I && h == 0
  | _ => false

/-- The machine's own recorded wire receives on stream `h`, standalone
(the view-free restatement of `ownRecvCount`). -/
def ownRecvs (rootH : Nat) (p : Party) (tr : List MObs) (h : Nat) : Nat :=
  tr.countP fun o =>
    match o with
    | .act a => recvActOn rootH p a h
    | _ => false

/-- `ownRecvCount` over a skeleton's announced view is the standalone
count: the view contributes only its party and root height. -/
theorem ownRecvCount_aviewOf (p : Party) (tr : List MObs) (h : Nat) :
    ownRecvCount (aviewOf sk p tr) tr h = ownRecvs sk.rootH p tr h := by
  unfold ownRecvCount ownRecvs
  congr 1
  funext o
  cases o with
  | act a => cases a <;> rfl
  | pushed g => rfl
  | delivered g => rfl

/-- `ownRecvs` under an `.act` append. -/
theorem ownRecvs_append_act (rootH : Nat) (p : Party) (tr : List MObs)
    (a : Action) (h : Nat) :
    ownRecvs rootH p (tr ++ [.act a]) h
      = ownRecvs rootH p tr h
        + (if recvActOn rootH p a h then 1 else 0) := by
  rw [ownRecvs, List.countP_append, ownRecvs]
  congr 1
  rw [List.countP_cons]
  simp

/-- `ownRecvs` under a non-`.act` append. -/
theorem ownRecvs_append_other (rootH : Nat) (p : Party)
    (tr : List MObs) {o : MObs} (hno : ∀ a, o ≠ .act a) (h : Nat) :
    ownRecvs rootH p (tr ++ [o]) h = ownRecvs rootH p tr h := by
  rw [ownRecvs, List.countP_append, ownRecvs]
  cases o with
  | act a => exact absurd rfl (hno a)
  | pushed g => simp
  | delivered g => simp

/-- Recorded receives only accumulate along an observation history. -/
theorem ownRecvs_le_of_prefix {rootH : Nat} {p : Party}
    {tr tr' : List MObs} (hp : tr <+: tr') (h : Nat) :
    ownRecvs rootH p tr h ≤ ownRecvs rootH p tr' h :=
  hp.sublist.countP_le

/-- The receive ledger: a machine's recorded wire receives never outrun
its base consumer counts, and recorded receives name real channels.

This is the ground fact behind the C-own arm of `groundedA` (the causal
closure's own-receive evidence): at a stuck state a receive the ledger
recorded really was consumed, so the evidence is performed. It is
strategy-generic; `recvLedger_reachable` runs the preservation
induction. -/
structure RecvLedger (sk : Skel) (s : MState) : Prop where
  bound : ∀ p h, Chan.wire p.other h ∈ allChans sk →
    ownRecvs sk.rootH p (s.hist p) h
      ≤ recvdOf sk s.base (Chan.wire p.other h)
  mem : ∀ p h, ownRecvs sk.rootH p (s.hist p) h ≠ 0 →
    Chan.wire p.other h ∈ allChans sk

/-- One recv-neutral base arm preserves the receive ledger: the wire
consumer counts are framed and the appended action is not counted. -/
private theorem RecvLedger.neutral_assemble {s : MState} {b : State}
    {a : Action} (hrl : RecvLedger sk s)
    (hrecvd : ∀ q g, Chan.wire q g ∈ allChans sk →
      recvdOf sk b (Chan.wire q g) = recvdOf sk s.base (Chan.wire q g))
    (hnr : ∀ p h, recvActOn sk.rootH p a h = false) :
    RecvLedger sk { s with base := b
                           hist := recordObs s.hist (actionParty a)
                             (.act a) } := by
  have hcnt : ∀ p h, ownRecvs sk.rootH p
      (recordObs s.hist (actionParty a) (.act a) p) h
      = ownRecvs sk.rootH p (s.hist p) h := by
    intro p h
    show ownRecvs sk.rootH p
      (if p == actionParty a then s.hist p ++ [.act a] else s.hist p) h = _
    by_cases hq : (p == actionParty a) = true
    · rw [if_pos hq, ownRecvs_append_act, hnr p h]
      simp
    · rw [if_neg hq]
  refine ⟨?_, ?_⟩
  · intro p h hmem
    show ownRecvs sk.rootH p
        (recordObs s.hist (actionParty a) (.act a) p) h
      ≤ recvdOf sk b (Chan.wire p.other h)
    rw [hcnt p h, hrecvd p.other h hmem]
    exact hrl.bound p h hmem
  · intro p h hne
    have hne' : ownRecvs sk.rootH p
        (recordObs s.hist (actionParty a) (.act a) p) h ≠ 0 := hne
    rw [hcnt p h] at hne'
    exact hrl.mem p h hne'

/-- The counted-receive assembly: one wire receive bumps exactly one
consumer count while its machine's ledger gains exactly one recorded
receive — the books balance on both sides. -/
private theorem RecvLedger.recv_assemble {s : MState} {b : State}
    {a : Action} {p₀ : Party} {h₀ : Nat} (hrl : RecvLedger sk s)
    (hr : RecvStep sk s.base b (Chan.wire p₀.other h₀))
    (hmem : Chan.wire p₀.other h₀ ∈ allChans sk)
    (hap : actionParty a = p₀)
    (hcnt : ∀ p h, recvActOn sk.rootH p a h = decide (p = p₀ ∧ h = h₀)) :
    RecvLedger sk { s with base := b
                           hist := recordObs s.hist (actionParty a)
                             (.act a) } := by
  have hhist : ∀ p, recordObs s.hist (actionParty a) (.act a) p
      = if p == p₀ then s.hist p ++ [.act a] else s.hist p := by
    intro p
    rw [hap]
    rfl
  refine ⟨?_, ?_⟩
  · intro p h hmemc
    show ownRecvs sk.rootH p
        (recordObs s.hist (actionParty a) (.act a) p) h
      ≤ recvdOf sk b (Chan.wire p.other h)
    rw [hhist p, hr.recvd _ hmemc]
    by_cases hq : (p == p₀) = true
    · have hpq : p = p₀ := beq_iff_eq.mp hq
      rw [if_pos hq, ownRecvs_append_act, hcnt p h]
      by_cases hh : h = h₀
      · rw [if_pos (by simp [hpq, hh]), if_pos (by rw [hpq, hh])]
        have := hrl.bound p h hmemc
        omega
      · rw [if_neg (by simp [hh]), if_neg (by simp [hh])]
        have := hrl.bound p h hmemc
        omega
    · rw [if_neg hq]
      have hce : Chan.wire p.other h ≠ Chan.wire p₀.other h₀ := by
        intro hc
        injection hc with hc1 hc2
        have hpp : p = p₀ := by
          cases p <;> cases p₀ <;> first | rfl | (exact Party.noConfusion hc1)
        rw [hpp] at hq
        simp at hq
      rw [if_neg hce]
      exact hrl.bound p h hmemc
  · intro p h hne0
    have hne : ownRecvs sk.rootH p
        (recordObs s.hist (actionParty a) (.act a) p) h ≠ 0 := hne0
    clear hne0
    rw [hhist p] at hne
    by_cases hq : (p == p₀) = true
    · rw [if_pos hq, ownRecvs_append_act, hcnt p h] at hne
      by_cases hph : p = p₀ ∧ h = h₀
      · rw [hph.1, hph.2]
        exact hmem
      · rw [if_neg (by simpa using hph)] at hne
        exact hrl.mem p h (by omega)
    · rw [if_neg hq] at hne
      exact hrl.mem p h hne

/-- A non-`.act` observation preserves the receive ledger whenever the
wire consumer counts are framed (push and deliver arms). -/
private theorem RecvLedger.obs_assemble {s s' : MState} {q₀ : Party}
    {o : MObs} (hrl : RecvLedger sk s) (hno : ∀ a, o ≠ .act a)
    (hb : ∀ q g, Chan.wire q g ∈ allChans sk →
      recvdOf sk s'.base (Chan.wire q g)
        = recvdOf sk s.base (Chan.wire q g))
    (hh : s'.hist = recordObs s.hist q₀ o) :
    RecvLedger sk s' := by
  have hcnt : ∀ p h, ownRecvs sk.rootH p (s'.hist p) h
      = ownRecvs sk.rootH p (s.hist p) h := by
    intro p h
    rw [hh]
    show ownRecvs sk.rootH p
      (if p == q₀ then s.hist p ++ [o] else s.hist p) h = _
    by_cases hq : (p == q₀) = true
    · rw [if_pos hq, ownRecvs_append_other _ _ _ hno]
    · rw [if_neg hq]
  refine ⟨?_, ?_⟩
  · intro p h hmem
    rw [hcnt p h, hb p.other h hmem]
    exact hrl.bound p h hmem
  · intro p h hne
    rw [hcnt p h] at hne
    exact hrl.mem p h hne

-- =================================== the receive-ledger preservation

/-- `recvActOn` decode for the stage receive: counted exactly at the
walk's own party and its input stream height. -/
private theorem recvActOn_walkRecvWire (rootH : Nat) (pk : Party × Nat)
    (p : Party) (h : Nat) :
    recvActOn rootH p (.walkRecvWire pk) h
      = decide (p = pk.1 ∧ h = pk.2 + 1) := by
  obtain ⟨q₀, g₀⟩ := pk
  show ((q₀, g₀) == (p, h - 1) && h != 0) = _
  by_cases hp : p = q₀
  · subst hp
    by_cases hh : h = g₀ + 1
    · subst hh
      simp
    · have hg : ¬(g₀ = h - 1 ∧ h ≠ 0) := by omega
      rcases Nat.eq_zero_or_pos h with hz | hpos
      · subst hz
        simp [hh]
      · have hne : g₀ ≠ h - 1 := by omega
        simp [hne, hh]
  · have h1 : ((q₀, g₀) == (p, h - 1)) = ((q₀ == p) && (g₀ == h - 1)) :=
      rfl
    have h2 : (q₀ == p) = false :=
      beq_eq_false_iff_ne.mpr (fun hc => hp hc.symm)
    rw [h1, h2]
    simp [hp]

/-- `recvActOn` decode for the responder's opening receive. -/
private theorem recvActOn_ropenRecv (rootH : Nat) (p : Party) (h : Nat) :
    recvActOn rootH p .ropenRecv h
      = decide (p = Party.R ∧ h = rootH) := by
  show (p == Party.R && h == rootH) = _
  cases p <;> by_cases hh : h = rootH <;> simp [hh]

/-- `recvActOn` decode for the absorber's supply receive. -/
private theorem recvActOn_absorbRecvWire (rootH : Nat) (p : Party)
    (h : Nat) :
    recvActOn rootH p .absorbRecvWire h
      = decide (p = Party.I ∧ h = 0) := by
  show (p == Party.I && h == 0) = _
  cases p <;> by_cases hh : h = 0 <;> simp [hh]

/-- A `walkRecvWire` success names a real stage key (the guard's key
membership, re-extracted). -/
private theorem apply_walkRecvWire_key {ax : AxMode} {pk : Party × Nat}
    {s : State} {b : State}
    (hb : Model.apply sk ax (.walkRecvWire pk) s = some b) :
    pk ∈ sk.walkKeys := by
  simp only [Model.apply] at hb
  split at hb
  case isTrue hg =>
    simp only [Bool.and_eq_true] at hg
    exact (List.contains_iff_mem ..).mp hg.1.1
  case isFalse => cases hb

/-- An internal receive preserves the receive ledger: the wire counts
are framed and no wire receive is recorded. -/
private theorem RecvLedger.recv_internal_assemble {s : MState} {b : State}
    {a : Action} {c₀ : Chan} (hrl : RecvLedger sk s)
    (hr : RecvStep sk s.base b c₀) (hw : isWire c₀ = false)
    (hnr : ∀ p h, recvActOn sk.rootH p a h = false) :
    RecvLedger sk { s with base := b
                           hist := recordObs s.hist (actionParty a)
                             (.act a) } := by
  refine RecvLedger.neutral_assemble hrl (fun q g hm => ?_) hnr
  have hne : Chan.wire q g ≠ c₀ := by
    intro hc
    rw [← hc] at hw
    simp [isWire] at hw
  rw [hr.recvd _ hm, if_neg hne]
  omega

/-- Every enabled base action preserves the receive ledger: the 23-arm
dispatch, reusing the Steps decomposition. -/
theorem recvLedger_base (hwf : sk.wellFormed = true) {a : Action}
    {s s' : MState} (hstep : applyBase sk .impl a s = some s')
    (hm : SInv sk s) (hrl : RecvLedger sk s) : RecvLedger sk s' := by
  obtain ⟨hnf, b, hb, hs'⟩ := applyBase_inv hstep
  have hL := hm.mux.invl
  subst hs'
  cases a with
  | iopenChoose o =>
      cases o with
      | wire =>
          obtain ⟨-, hq, -, -, -⟩ := step_iopenChoose_wire hb hL
          exact RecvLedger.neutral_assemble hrl
            (fun q g hm => hq.recvd _ hm) (fun _ _ => rfl)
      | query =>
          obtain ⟨-, hq, -⟩ := step_iopenChoose_query hb hL
          exact RecvLedger.neutral_assemble hrl
            (fun q g hm => hq.recvd _ hm) (fun _ _ => rfl)
  | iopenFire =>
      have hch : s.base.iopenCh = some .query := by
        rw [Model.apply] at hb
        cases hio : s.base.iopenCh with
        | none => rw [hio] at hb; cases hb
        | some o =>
            cases o with
            | wire =>
                exfalso
                rw [isWireFire, hio] at hnf
                simp at hnf
            | query => rfl
      obtain ⟨-, hsend, -⟩ := step_iopenFire_query hch hb hL
      exact RecvLedger.neutral_assemble hrl
        (fun q g hm => hsend.recvd _ hm) (fun _ _ => rfl)
  | ropenRecv =>
      obtain ⟨-, hr, -⟩ := step_ropenRecv hb hL
      exact RecvLedger.recv_assemble (p₀ := Party.R) (h₀ := sk.rootH)
        hrl hr (mem_allChans_wire_root _) rfl
        (fun p h => recvActOn_ropenRecv sk.rootH p h)
  | ropenChoose o =>
      cases o with
      | wire =>
          obtain ⟨-, hq, -, -, -⟩ := step_ropenChoose_wire hb hL
          exact RecvLedger.neutral_assemble hrl
            (fun q g hm => hq.recvd _ hm) (fun _ _ => rfl)
      | res =>
          obtain ⟨-, hq, -⟩ := step_ropenChoose_res hb hL
          exact RecvLedger.neutral_assemble hrl
            (fun q g hm => hq.recvd _ hm) (fun _ _ => rfl)
      | query =>
          obtain ⟨-, hq, -⟩ := step_ropenChoose_query hb hL
          exact RecvLedger.neutral_assemble hrl
            (fun q g hm => hq.recvd _ hm) (fun _ _ => rfl)
  | ropenFire =>
      have hch : s.base.ropenCh = some .res
          ∨ s.base.ropenCh = some .query := by
        rw [Model.apply] at hb
        cases hro : s.base.ropenCh with
        | none => rw [hro] at hb; cases hb
        | some o =>
            cases o with
            | wire =>
                exfalso
                rw [isWireFire, hro] at hnf
                simp at hnf
            | res => exact Or.inl rfl
            | query => exact Or.inr rfl
      rcases hch with hch | hch
      · obtain ⟨-, hsend, -⟩ := step_ropenFire_res hch hb hL
        exact RecvLedger.neutral_assemble hrl
          (fun q g hm => hsend.recvd _ hm) (fun _ _ => rfl)
      · obtain ⟨-, hsend, -⟩ := step_ropenFire_query hch hb hL
        exact RecvLedger.neutral_assemble hrl
          (fun q g hm => hsend.recvd _ hm) (fun _ _ => rfl)
  | walkRecvWire pk =>
      obtain ⟨-, hr, -⟩ := step_walkRecvWire hwf pk hb hL
      have hkey := apply_walkRecvWire_key hb
      exact RecvLedger.recv_assemble (p₀ := pk.1) (h₀ := pk.2 + 1) hrl hr
        (mem_allChans_wireIn hwf hkey) rfl
        (fun p h => recvActOn_walkRecvWire sk.rootH pk p h)
  | walkRecvAsked pk =>
      obtain ⟨-, hr, -⟩ := step_walkRecvAsked hwf pk hb hL
      exact RecvLedger.recv_internal_assemble hrl hr rfl (fun _ _ => rfl)
  | walkCommit pk o =>
      cases o with
      | wire i =>
          obtain ⟨-, hq, -, -, -, -⟩ := step_walkCommit_wire hwf pk i hb hL
          exact RecvLedger.neutral_assemble hrl
            (fun q g hm => hq.recvd _ hm) (fun _ _ => rfl)
      | res i =>
          obtain ⟨-, hq, -⟩ := step_walkCommit_res pk i hb hL
          exact RecvLedger.neutral_assemble hrl
            (fun q g hm => hq.recvd _ hm) (fun _ _ => rfl)
      | query i =>
          obtain ⟨-, hq, -⟩ := step_walkCommit_query pk i hb hL
          exact RecvLedger.neutral_assemble hrl
            (fun q g hm => hq.recvd _ hm) (fun _ _ => rfl)
      | parent =>
          obtain ⟨-, hq, -⟩ := step_walkCommit_parent pk hb hL
          exact RecvLedger.neutral_assemble hrl
            (fun q g hm => hq.recvd _ hm) (fun _ _ => rfl)
  | walkFire pk =>
      simp only [Model.apply] at hb
      split at hb
      next o hcm =>
        split at hb
        case isFalse => cases hb
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq]
            at hg
          obtain ⟨⟨hmem, hph2⟩, hlt1⟩ := hg
          have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
          injection hb with hbeq
          obtain ⟨-, -, hrecv, -, -, -⟩ :=
            step_fire (s' := setWalk s.base pk
              (normWalk sk pk.2 (fireOblig (s.base.walk pk) o)))
              hwf pk o hmem' hph2 hcm rfl hL
          refine RecvLedger.neutral_assemble hrl (fun q g hm => ?_)
            (fun _ _ => rfl)
          rw [← hbeq]
          show recvdOf sk { setWalk s.base pk
              (normWalk sk pk.2 (fireOblig (s.base.walk pk) o)) with
              chan := bump s.base.chan (obligChan pk o) 1 } _ = _
          rw [recvdOf_chan_blind]
          exact hrecv _ hm
      next hcm => cases hb
  | walkCloseWire pk =>
      obtain ⟨-, hq, -⟩ := step_walkCloseWire hwf pk hb hL
      exact RecvLedger.neutral_assemble hrl
        (fun q g hm => hq.recvd _ hm) (fun _ _ => rfl)
  | walkCloseAsked pk =>
      obtain ⟨-, hq, -⟩ := step_walkCloseAsked hwf pk hb hL
      exact RecvLedger.neutral_assemble hrl
        (fun q g hm => hq.recvd _ hm) (fun _ _ => rfl)
  | asmRecvRes pk =>
      obtain ⟨-, hr, -⟩ := step_asmRecvRes hwf pk hb hL
      refine RecvLedger.recv_internal_assemble hrl hr ?_ (fun _ _ => rfl)
      rw [asmResChan]
      split <;> rfl
  | asmRecvLevel pk =>
      obtain ⟨-, hr, -⟩ := step_asmRecvLevel hwf pk hb hL
      exact RecvLedger.recv_internal_assemble hrl hr rfl (fun _ _ => rfl)
  | asmSend pk =>
      obtain ⟨-, hsend, -⟩ := step_asmSend hwf pk hb hL
      exact RecvLedger.neutral_assemble hrl
        (fun q g hm => hsend.recvd _ hm) (fun _ _ => rfl)
  | asmClose pk =>
      obtain ⟨-, hq, -⟩ := step_asmClose hwf pk hb hL
      exact RecvLedger.neutral_assemble hrl
        (fun q g hm => hq.recvd _ hm) (fun _ _ => rfl)
  | absorbRecvWire =>
      obtain ⟨-, hr, -⟩ := step_absorbRecvWire hwf hb hL
      have h2 : 2 ≤ sk.rootH := (wf_rootH hwf).2
      have hev : sk.rootH % 2 = 0 := (wf_rootH hwf).1
      exact RecvLedger.recv_assemble (p₀ := Party.I) (h₀ := 0) hrl hr
        (mem_allChans_wireOut
          (Sched.mem_walkKeys_of sk hwf (by omega) (Or.inr ⟨rfl, rfl⟩)))
        rfl (fun p h => recvActOn_absorbRecvWire sk.rootH p h)
  | absorbRecvAsked =>
      obtain ⟨-, hr, -⟩ := step_absorbRecvAsked hb hL
      exact RecvLedger.recv_internal_assemble hrl hr rfl (fun _ _ => rfl)
  | absorbSend =>
      obtain ⟨-, hsend, -⟩ := step_absorbSend hb hL
      exact RecvLedger.neutral_assemble hrl
        (fun q g hm => hsend.recvd _ hm) (fun _ _ => rfl)
  | absorbCloseWire =>
      obtain ⟨-, hq, -⟩ := step_absorbCloseWire hb hL
      exact RecvLedger.neutral_assemble hrl
        (fun q g hm => hq.recvd _ hm) (fun _ _ => rfl)
  | absorbCloseAsked =>
      obtain ⟨-, hq, -⟩ := step_absorbCloseAsked hb hL
      exact RecvLedger.neutral_assemble hrl
        (fun q g hm => hq.recvd _ hm) (fun _ _ => rfl)
  | finRet =>
      obtain ⟨-, hr, -⟩ := step_finRet hb hL
      exact RecvLedger.recv_internal_assemble hrl hr rfl (fun _ _ => rfl)
  | finRes =>
      obtain ⟨-, hr, -⟩ := step_finRes hb hL
      exact RecvLedger.recv_internal_assemble hrl hr rfl (fun _ _ => rfl)
  | finRets =>
      obtain ⟨-, hr, -⟩ := step_finRets hb hL
      exact RecvLedger.recv_internal_assemble hrl hr rfl (fun _ _ => rfl)

/-- A push preserves the receive ledger: the flush receipt is not a
receive record, and the fire's cursor effect never touches a consumer
count. -/
theorem recvLedger_push (hwf : sk.wellFormed = true) {C : Nat}
    {q : Party} {h₀ : Nat} {s s' : MState}
    (hstep : firePush sk C q h₀ s = some s') (hm : SInv sk s)
    (hrl : RecvLedger sk s) : RecvLedger sk s' := by
  have hL := hm.mux.invl
  rw [firePush] at hstep
  simp only [] at hstep
  split at hstep
  case isFalse => cases hstep
  case isTrue hroom =>
    split at hstep
    · -- the opening stream
      cases q with
      | I =>
          cases hob : s.base.iopenCh with
          | none => rw [hob] at hstep; cases hstep
          | some ob =>
              cases ob with
              | query => rw [hob] at hstep; cases hstep
              | wire =>
                  rw [hob] at hstep
                  injection hstep with hs'
                  refine RecvLedger.obs_assemble (q₀ := Party.I)
                    (o := .pushed h₀) hrl
                    (fun a hc => by cases hc) (fun q g hmc => ?_)
                    (by rw [← hs'])
                  rw [← hs']
                  exact recvdOf_ext sk (fun pk => rfl) (fun pk => rfl)
                    (fun pk => rfl) rfl rfl rfl rfl rfl rfl _
      | R =>
          cases hob : s.base.ropenCh with
          | none => rw [hob] at hstep; cases hstep
          | some ob =>
              cases ob with
              | res => rw [hob] at hstep; cases hstep
              | query => rw [hob] at hstep; cases hstep
              | wire =>
                  rw [hob] at hstep
                  injection hstep with hs'
                  refine RecvLedger.obs_assemble (q₀ := Party.R)
                    (o := .pushed h₀) hrl
                    (fun a hc => by cases hc) (fun q g hmc => ?_)
                    (by rw [← hs'])
                  rw [← hs']
                  exact recvdOf_ext sk (fun pk => rfl) (fun pk => rfl)
                    (fun pk => rfl) rfl rfl rfl rfl rfl rfl _
    · -- a walk stream
      split at hstep
      next i hcm =>
        split at hstep
        case isFalse => cases hstep
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq] at hg
          obtain ⟨hmem, hph2⟩ := hg
          have hmem' : (q, h₀) ∈ sk.walkKeys := by simpa using hmem
          injection hstep with hs'
          obtain ⟨-, -, hrecv, -, -, -⟩ :=
            step_fire (s' := setWalk s.base (q, h₀)
              (normWalk sk h₀ (fireOblig (s.base.walk (q, h₀))
                (.wire i))))
              hwf (q, h₀) (.wire i) hmem' (by simpa using hph2) hcm rfl hL
          refine RecvLedger.obs_assemble (q₀ := q) (o := .pushed h₀) hrl
            (fun a hc => by cases hc) (fun q' g hmc => ?_)
            (by rw [← hs'])
          rw [← hs']
          exact hrecv _ hmc
      next => cases hstep

/-- Every muxed step preserves the receive ledger. -/
theorem recvLedger_step (hwf : sk.wellFormed = true) {C : Nat}
    {σI σR : Strategy} {ma : MAction} {s s' : MState}
    (hstep : apply sk .impl C σI σR ma s = some s')
    (hm : SInv sk s) (hrl : RecvLedger sk s) : RecvLedger sk s' := by
  cases ma with
  | base a => exact recvLedger_base hwf hstep hm hrl
  | push q =>
      simp only [apply] at hstep
      split at hstep
      next h₀ _ => exact recvLedger_push hwf hstep hm hrl
      next => cases hstep
  | deliver q =>
      simp only [apply] at hstep
      split at hstep
      case h_2 => cases hstep
      case h_1 c rest hpp =>
          split at hstep
          case isFalse => cases hstep
          case isTrue h0 =>
            injection hstep with hs'
            refine RecvLedger.obs_assemble (q₀ := q.other)
              (o := .delivered (wireHeight c)) hrl
              (fun a hc => by cases hc) (fun q' g hmc => ?_)
              (by rw [← hs'])
            rw [← hs']
            exact recvdOf_chan_blind _ _

/-- The receive ledger holds at every reachable muxed state:
strategy-generic, like `sinv_reachable`. -/
theorem recvLedger_reachable (hwf : sk.wellFormed = true) {C : Nat}
    {σI σR : Strategy} {s : MState}
    (hr : MReachable sk .impl C σI σR s) : RecvLedger sk s := by
  induction hr with
  | init =>
      refine ⟨?_, ?_⟩
      · intro p h hmem
        show ownRecvs sk.rootH p [] h ≤ _
        rw [show ownRecvs sk.rootH p [] h = 0 from rfl]
        omega
      · intro p h hne
        exact absurd rfl hne
  | step ma hr' hstep ih =>
      exact recvLedger_step hwf hstep (sinv_reachable hwf hr') ih

end StreamingMirror.Mux
