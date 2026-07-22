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
open Sched (Ev performed pends PendOkE)

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

-- ======================================================== the census
-- The announced BFS census (`levelA`) against the true level slices:
-- a known prefix whose members carry known kinds, exact when complete.

/-- A record hit names an announced id carrying the true record. -/
theorem rec?_some_inv {p : Party} {tr : List MObs} {u : Nat} {sc : Scope}
    (h : (aviewOf sk p tr).rec? u = some sc) :
    sc = sk.scope u ∧ u ∈ announcedIds sk p tr := by
  rw [rec?_aviewOf] at h
  by_cases hmem : u ∈ announcedIds sk p tr
  · rw [if_pos hmem] at h
    injection h with h
    exact ⟨h.symm, hmem⟩
  · rw [if_neg hmem] at h
    cases h

/-- A flatMap over a `take` prefix is a prefix of the flatMap. -/
private theorem flatMap_take_prefix {α β : Type _} (f : α → List β)
    (l : List α) (m : Nat) :
    (l.take m).flatMap f <+: l.flatMap f := by
  conv => rhs; rw [← List.take_append_drop m l]
  rw [List.flatMap_append]
  exact ⟨(l.drop m).flatMap f, rfl⟩

/-- Prefixes lift through `flatMap`. -/
private theorem prefix_flatMap {α β : Type _} (f : α → List β)
    {l₁ l₂ : List α} (h : l₁ <+: l₂) :
    l₁.flatMap f <+: l₂.flatMap f := by
  obtain ⟨t, rfl⟩ := h
  rw [List.flatMap_append]
  exact ⟨t.flatMap f, rfl⟩

/-- The collect pass of `levelA`, characterized: over a list of real,
kind-known scopes it emits the kid lists of a prefix — cut at the first
dispute whose record is missing — with the flag reporting totality, and
every dispute in the emitted prefix announced. -/
private theorem collect_spec (hwf : sk.wellFormed = true)
    {p : Party} {tr : List MObs} :
    ∀ l : List Nat,
      (∀ u ∈ l, u < sk.scopes.length
        ∧ (aviewOf sk p tr).kind? u = some ((sk.scope u).kind)) →
      ∃ m, m ≤ l.length
        ∧ (levelA.collect (aviewOf sk p tr) l).1
            = (l.take m).flatMap (fun u => (sk.scope u).kids)
        ∧ ((levelA.collect (aviewOf sk p tr) l).2 = true → m = l.length)
        ∧ (∀ u ∈ l.take m, (sk.scope u).kind = Kind.D →
            u ∈ announcedIds sk p tr) := by
  intro l
  induction l with
  | nil =>
      intro _
      exact ⟨0, by omega, rfl, fun _ => rfl, fun u hu => absurd hu
        (by simp)⟩
  | cons sid rest ih =>
      intro hl
      obtain ⟨hreal, hkind⟩ := hl sid (List.mem_cons_self ..)
      rw [levelA.collect]
      by_cases hD : (sk.scope sid).kind = Kind.D
      · rw [if_pos (by rw [hkind, hD]; rfl)]
        cases hrec : (aviewOf sk p tr).rec? sid with
        | none =>
            refine ⟨0, by omega, by simp, ?_, fun u hu => absurd hu
              (by simp)⟩
            intro hcomp
            simp at hcomp
        | some sc =>
            obtain ⟨rfl, hann⟩ := rec?_some_inv hrec
            obtain ⟨m, hm, hitems, hcomp, hDs⟩ := ih
              (fun u hu => hl u (List.mem_cons_of_mem _ hu))
            refine ⟨m + 1, by simpa using hm, ?_, ?_, ?_⟩
            · show (sk.scope sid).kids
                  ++ (levelA.collect (aviewOf sk p tr) rest).1 = _
              rw [hitems, List.take_succ_cons, List.flatMap_cons]
            · intro hc
              have : (levelA.collect (aviewOf sk p tr) rest).2
                  = true := hc
              rw [List.length_cons, hcomp this]
            · intro u hu hDu
              rw [List.take_succ_cons] at hu
              rcases List.mem_cons.mp hu with rfl | hu'
              · exact hann
              · exact hDs u hu' hDu
      · rw [if_neg (by rw [hkind]; simp; intro hc; exact hD hc)]
        obtain ⟨m, hm, hitems, hcomp, hDs⟩ := ih
          (fun u hu => hl u (List.mem_cons_of_mem _ hu))
        have hkids : (sk.scope sid).kids = [] :=
          (wf_scope_nonD hwf hreal hD).1
        refine ⟨m + 1, by simpa using hm, ?_, ?_, ?_⟩
        · rw [hitems, List.take_succ_cons, List.flatMap_cons, hkids,
            List.nil_append]
        · intro hc
          rw [List.length_cons, hcomp hc]
        · intro u hu hDu
          rw [List.take_succ_cons] at hu
          rcases List.mem_cons.mp hu with rfl | hu'
          · exact absurd hDu hD
          · exact hDs u hu' hDu

/-- The announced census: at every depth, a prefix of the true level
slice whose members carry their true kinds — the whole slice when the
completeness flag is up. -/
theorem levelA_spec (hwf : sk.wellFormed = true) (p : Party)
    (tr : List MObs) :
    ∀ steps, steps ≤ sk.rootH →
      ((levelA (aviewOf sk p tr) steps).1
          <+: sk.scopesAt (sk.rootH - steps))
      ∧ (∀ u ∈ (levelA (aviewOf sk p tr) steps).1,
          (aviewOf sk p tr).kind? u = some ((sk.scope u).kind))
      ∧ ((levelA (aviewOf sk p tr) steps).2 = true →
          (levelA (aviewOf sk p tr) steps).1
            = sk.scopesAt (sk.rootH - steps)) := by
  intro steps
  induction steps with
  | zero =>
      intro _
      have hroot : sk.scopesAt (sk.rootH - 0) = [0] := by
        rw [show sk.rootH - 0 = sk.rootH from rfl,
          Sched.wf_root_stage hwf]
      refine ⟨?_, ?_, ?_⟩
      · rw [hroot]
        exact List.prefix_refl _
      · intro u hu
        have hu' : u ∈ [(0 : Nat)] := hu
        rw [List.mem_singleton] at hu'
        subst hu'
        show (aviewOf sk p tr).kind? 0 = _
        rw [AView.kind?, if_pos (show ((0 : Nat) == 0) = true from rfl),
          wf_root_kind hwf]
      · intro _
        rw [hroot]
        rfl
  | succ steps ih =>
      intro hle
      obtain ⟨hpre, hkinds, hcomp⟩ := ih (by omega)
      have hreal : ∀ u ∈ (levelA (aviewOf sk p tr) steps).1,
          u < sk.scopes.length
            ∧ (aviewOf sk p tr).kind? u = some ((sk.scope u).kind) := by
        intro u hu
        exact ⟨(mem_scopesAt (hpre.sublist.mem hu)).1, hkinds u hu⟩
      obtain ⟨m, hm, hitems, hflag, hDs⟩ :=
        collect_spec hwf (levelA (aviewOf sk p tr) steps).1 hreal
      have hbfs := wf_bfs_aligned hwf
        (show sk.rootH - (steps + 1) < sk.rootH from by omega)
      have hbfs' : (sk.scopesAt (sk.rootH - steps)).flatMap
          (fun s => (sk.scope s).kids)
          = sk.scopesAt (sk.rootH - (steps + 1)) := by
        rw [show sk.rootH - (steps + 1) + 1 = sk.rootH - steps from by
          omega] at hbfs
        exact hbfs
      have hshape : levelA (aviewOf sk p tr) (steps + 1)
          = ((levelA.collect (aviewOf sk p tr)
              (levelA (aviewOf sk p tr) steps).1).1,
             (levelA (aviewOf sk p tr) steps).2
              && (levelA.collect (aviewOf sk p tr)
                  (levelA (aviewOf sk p tr) steps).1).2) := by
        rw [levelA]
      refine ⟨?_, ?_, ?_⟩
      · rw [hshape]
        show (levelA.collect _ _).1 <+: _
        rw [hitems, ← hbfs']
        exact (flatMap_take_prefix _ _ m).trans
          (prefix_flatMap _ hpre)
      · intro v hv
        rw [hshape] at hv
        have hv' : v ∈ ((levelA (aviewOf sk p tr) steps).1.take m).flatMap
            (fun u => (sk.scope u).kids) := by
          rw [← hitems]
          exact hv
        obtain ⟨u, hu, hvk⟩ := List.mem_flatMap.mp hv'
        have humem := (List.take_prefix m _).sublist.mem hu
        have hureal := (hreal u humem).1
        by_cases hD : (sk.scope u).kind = Kind.D
        · exact kind?_aviewOf_of_kid hwf (hDs u hu hD) hvk
        · rw [(wf_scope_nonD hwf hureal hD).1] at hvk
          cases hvk
      · intro hc
        rw [hshape] at hc ⊢
        have hc' : (levelA (aviewOf sk p tr) steps).2 = true
            ∧ (levelA.collect (aviewOf sk p tr)
                (levelA (aviewOf sk p tr) steps).1).2 = true := by
          simpa using hc
        show (levelA.collect _ _).1 = _
        rw [hitems, hflag hc'.2, List.take_length, ← hbfs',
          hcomp hc'.1]

-- =============================================== the walk transcription
-- The announced walk layouts against the true `.impl` traces: block by
-- block, chunk by chunk, the layout is a literal prefix, exact (with
-- the prefix-sum counters advanced) while every consulted record is
-- announced.

/-- Filtering positions equals filtering values. -/
private theorem length_filter_range_getD (p' : Nat → Bool) :
    ∀ l : List Nat,
      ((List.range l.length).filter fun i => p' (l.getD i 0)).length
        = (l.filter p').length := by
  intro l
  induction l with
  | nil => rfl
  | cons a l ih =>
      rw [List.length_cons, List.range_succ_eq_map, List.filter_cons]
      have hmap : (((List.range l.length).map (· + 1)).filter
          fun i => p' ((a :: l).getD i 0))
          = ((List.range l.length).filter
              fun i => p' (l.getD i 0)).map (· + 1) := by
        rw [List.filter_map]
        congr 1
      by_cases hpa : p' a = true
      · rw [if_pos (by simpa using hpa), List.length_cons, hmap,
          List.length_map, ih, List.filter_cons, if_pos hpa,
          List.length_cons]
      · rw [if_neg (by simpa using hpa), hmap, List.length_map, ih,
          List.filter_cons, if_neg (by simpa using hpa)]

/-- The chunk loop of `peerBlockA`, characterized: from child `i` with
the prefix-sum counters, the emitted events are a prefix of the true
remaining chunks, exact with the counters advanced to the block end
when the loop completes. -/
private theorem chunksA_spec (hwf : sk.wellFormed = true)
    {p : Party} {tr : List MObs} (q : Party) {h k : Nat}
    (h0 : h ≠ 0) (hk : k < sk.stageLen h)
    (hann : sk.stageScope h k ∈ announcedIds sk p tr)
    (wires : Nat) :
    ∀ (fuel i w d qacc : Nat),
      i ≤ (sk.scope (sk.stageScope h k)).kids.length →
      (sk.scope (sk.stageScope h k)).kids.length - i < fuel →
      w = sk.wiresBefore h k + i →
      d = sk.dsBefore h k
        + ((List.range i).filter
            (sk.childIsD h (sk.stageScope h k))).length →
      qacc = sk.qsBefore h k
        + ((List.range i).map
            (sk.qCount h (sk.stageScope h k))).sum →
      (peerBlockA.chunks (aviewOf sk p tr) q h wires
          (sk.scope (sk.stageScope h k))
          (sk.scope (sk.stageScope h k)).kids.length
          i w d qacc fuel).1
        <+: (List.range' i
              ((sk.scope (sk.stageScope h k)).kids.length - i)).flatMap
            (Sched.childChunk sk (q, h) k)
      ∧ ((peerBlockA.chunks (aviewOf sk p tr) q h wires
            (sk.scope (sk.stageScope h k))
            (sk.scope (sk.stageScope h k)).kids.length
            i w d qacc fuel).2.2 = true →
          (peerBlockA.chunks (aviewOf sk p tr) q h wires
              (sk.scope (sk.stageScope h k))
              (sk.scope (sk.stageScope h k)).kids.length
              i w d qacc fuel).1
            = (List.range' i
                ((sk.scope (sk.stageScope h k)).kids.length - i)).flatMap
                (Sched.childChunk sk (q, h) k)
          ∧ (peerBlockA.chunks (aviewOf sk p tr) q h wires
              (sk.scope (sk.stageScope h k))
              (sk.scope (sk.stageScope h k)).kids.length
              i w d qacc fuel).2.1
            = (sk.wiresBefore h (k + 1), sk.dsBefore h (k + 1),
               sk.qsBefore h (k + 1))) := by
  have hureal : sk.stageScope h k < sk.scopes.length :=
    stageScope_lt_scopes sk hk
  have hn : sk.nChildren h (sk.stageScope h k)
      = (sk.scope (sk.stageScope h k)).kids.length := by
    unfold Skel.nChildren
    rw [if_neg (by simpa using h0)]
  intro fuel
  induction fuel with
  | zero =>
      intro i w d qacc hi hfuel
      omega
  | succ fuel ih =>
      intro i w d qacc hi hfuel hw hd hq
      by_cases hin : i = (sk.scope (sk.stageScope h k)).kids.length
      · -- past the last child: the loop closes the block
        have hnone : (sk.scope (sk.stageScope h k)).kids[i]? = none := by
          rw [hin]
          exact List.getElem?_eq_none (Nat.le_refl _)
        have hstep : peerBlockA.chunks (aviewOf sk p tr) q h wires
            (sk.scope (sk.stageScope h k))
            (sk.scope (sk.stageScope h k)).kids.length
            i w d qacc (fuel + 1)
            = ([], (w, d, qacc), true) := by
          rw [peerBlockA.chunks]
          simp only [hnone]
          rw [if_neg (by simpa using h0)]
        rw [hstep, hin]
        simp only [Nat.sub_self, List.range'_zero, List.flatMap_nil]
        refine ⟨List.prefix_refl _, fun _ => ?_⟩
        refine ⟨by simp, ?_⟩
        show (w, d, qacc) = _
        have hdof : sk.dOf h (sk.stageScope h k)
            = ((List.range
                (sk.scope (sk.stageScope h k)).kids.length).filter
                  (sk.childIsD h (sk.stageScope h k))).length := by
          unfold Skel.dOf Skel.dCount
          rw [if_neg (by simpa using h0),
            ← length_filter_range_getD
              (fun v => (sk.scope v).kind == Kind.D)]
          congr 1
          refine (List.filter_congr ?_).symm
          intro i' hi'
          rw [List.mem_range] at hi'
          unfold Skel.childIsD
          rw [if_neg (by simpa using h0),
            List.getElem?_eq_getElem hi',
            List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi']
          rfl
        have hqof : sk.qOf h (sk.stageScope h k)
            = ((List.range
                (sk.scope (sk.stageScope h k)).kids.length).map
                  (sk.qCount h (sk.stageScope h k))).sum := by
          unfold Skel.qOf
          rw [hn, foldl_add_eq_sum, Nat.zero_add]
        rw [hw, hd, hq, Sched.wiresBefore_succ sk hk,
          Sched.dsBefore_succ sk hk, Sched.qsBefore_succ sk hk, hn,
          hdof, hqof, hin]
      · -- a live child: transcribe its chunk and recurse
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
        have hD : sk.childIsD h (sk.stageScope h k) i
            = ((sk.scope ((sk.scope (sk.stageScope h k)).kids[i])).kind
                == Kind.D) := by
          unfold Skel.childIsD
          rw [if_neg (by simpa using h0), hget]
        have hrange : List.range' i
            ((sk.scope (sk.stageScope h k)).kids.length - i)
            = i :: List.range' (i + 1)
                ((sk.scope (sk.stageScope h k)).kids.length - (i + 1)) := by
          rw [show (sk.scope (sk.stageScope h k)).kids.length - i
            = ((sk.scope (sk.stageScope h k)).kids.length - (i + 1)) + 1
            from by omega]
          rfl
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
          · rw [List.filter_cons,
              if_neg (by simpa using hDi),
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
            -- an R child: bare wire, no bookkeeping
            have hDi : sk.childIsD h (sk.stageScope h k) i = false := by
              rw [hD, hkindv]
              rfl
            have hchunk : Sched.childChunk sk (q, h) k i
                = [(wireOut (q, h), true, sk.wiresBefore h k + i)] := by
              unfold Sched.childChunk
              rw [if_neg (by simp [hDi])]
            have hq0 : sk.qCount h (sk.stageScope h k) i = 0 := by
              unfold Skel.qCount
              rw [if_pos (by simp [hDi])]
            obtain ⟨hpre', hcomp'⟩ := ih (i + 1) (w + 1) d qacc
              (by omega) (by omega) (by omega)
              (by rw [hd, hdrank, hDi]; simp)
              (by rw [hq, hqsum, hq0]; omega)
            rcases hcall : peerBlockA.chunks (aviewOf sk p tr) q h wires
                (sk.scope (sk.stageScope h k))
                (sk.scope (sk.stageScope h k)).kids.length
                (i + 1) (w + 1) d qacc fuel with ⟨evs, cnts, ok⟩
            rw [hcall] at hpre' hcomp'
            have hstep : peerBlockA.chunks (aviewOf sk p tr) q h wires
                (sk.scope (sk.stageScope h k))
                (sk.scope (sk.stageScope h k)).kids.length
                i w d qacc (fuel + 1)
                = ((Chan.wire q h, true, w) :: evs, cnts, ok) := by
              simp only [peerBlockA.chunks]
              rw [if_neg (by simpa using h0)]
              simp only [hget, hkind, hkindv, hcall]
            rw [hstep, hrange, List.flatMap_cons, hchunk]
            constructor
            · show (Chan.wire q h, true, w) :: evs <+: _
              rw [← hw]
              show ((Chan.wire q h, true, w) : Ev) :: evs
                <+: ((Chan.wire q h, true, w) : Ev) :: _
              obtain ⟨t, ht⟩ := hpre'
              refine ⟨t, ?_⟩
              rw [List.cons_append, show evs ++ t
                = List.flatMap (Sched.childChunk sk (q, h) k)
                    (List.range' (i + 1)
                      ((sk.scope (sk.stageScope h k)).kids.length
                        - (i + 1))) from ht]
              rfl
            · intro hok
              obtain ⟨heq, hcnts⟩ := hcomp' hok
              have heq' : evs = List.flatMap
                  (Sched.childChunk sk (q, h) k)
                  (List.range' (i + 1)
                    ((sk.scope (sk.stageScope h k)).kids.length
                      - (i + 1))) := heq
              refine ⟨?_, hcnts⟩
              show (Chan.wire q h, true, w) :: evs = _
              rw [heq', hw]
              rfl
        | D =>
            -- a D child: wire, then its resolution and query train
            have hDi : sk.childIsD h (sk.stageScope h k) i = true := by
              rw [hD, hkindv]
              rfl
            have hchunk : Sched.childChunk sk (q, h) k i
                = (wireOut (q, h), true, sk.wiresBefore h k + i)
                  :: (lowerOut (q, h), true, sk.dsBefore h k
                      + ((List.range i).filter
                          (fun i' => sk.childIsD h
                            (sk.stageScope h k) i')).length)
                  :: ((List.range
                        (sk.qCount h (sk.stageScope h k) i)).map
                      fun t => (askedOut (q, h), true,
                        sk.qsBefore h k
                          + ((List.range i).map
                              (fun i' => sk.qCount h
                                (sk.stageScope h k) i')).sum + t)) := by
              unfold Sched.childChunk
              rw [if_pos hDi]
            cases hrecv : (aviewOf sk p tr).rec?
                ((sk.scope (sk.stageScope h k)).kids[i]) with
            | none =>
                -- the kid record is missing: cut after the wire
                have hstep : peerBlockA.chunks (aviewOf sk p tr) q h wires
                    (sk.scope (sk.stageScope h k))
                    (sk.scope (sk.stageScope h k)).kids.length
                    i w d qacc (fuel + 1)
                    = ([(Chan.wire q h, true, w)],
                       (w + 1, d, qacc), false) := by
                  simp only [peerBlockA.chunks]
                  rw [if_neg (by simpa using h0)]
                  simp only [hget, hkind, hkindv, qCountA, hrecv,
                    Option.map]
                rw [hstep, hrange, List.flatMap_cons]
                constructor
                · rw [hchunk]
                  refine List.IsPrefix.trans ?_ (List.prefix_append _ _)
                  rw [← hw]
                  exact ⟨(lowerOut (q, h), true, _) :: _, rfl⟩
                · intro hok
                  cases hok
            | some sc' =>
                obtain ⟨hsc', hann'⟩ := rec?_some_inv hrecv
                have hheight : (sk.scope
                    ((sk.scope (sk.stageScope h k)).kids[i])).height
                    = h := by
                  have hkf := (Sched.wf_kid_facts hwf hureal _ hvmem).2
                  have hsh := Sched.stageScope_height sk
                    (h := h) (k := k) hk
                  omega
                have hqc : sk.qCount h (sk.stageScope h k) i
                    = (if (h == 1) = true then
                        (sk.scope ((sk.scope
                          (sk.stageScope h k)).kids[i])).leafReqs
                      else (sk.scope ((sk.scope
                          (sk.stageScope h k)).kids[i])).kids.length) := by
                  unfold Skel.qCount
                  rw [if_neg (by simp [hDi])]
                  simp only [hget, hheight]
                obtain ⟨hpre', hcomp'⟩ := ih (i + 1) (w + 1) (d + 1)
                  (qacc + sk.qCount h (sk.stageScope h k) i)
                  (by omega) (by omega) (by omega)
                  (by rw [hd, hdrank, hDi, if_pos rfl]; omega)
                  (by rw [hq, hqsum]; omega)
                rcases hcall : peerBlockA.chunks (aviewOf sk p tr) q h
                    wires (sk.scope (sk.stageScope h k))
                    (sk.scope (sk.stageScope h k)).kids.length
                    (i + 1) (w + 1) (d + 1)
                    (qacc + sk.qCount h (sk.stageScope h k) i)
                    fuel with ⟨evs, cnts, ok⟩
                rw [hcall] at hpre' hcomp'
                have hstep : peerBlockA.chunks (aviewOf sk p tr) q h wires
                    (sk.scope (sk.stageScope h k))
                    (sk.scope (sk.stageScope h k)).kids.length
                    i w d qacc (fuel + 1)
                    = ((Chan.wire q h, true, w)
                        :: (Chan.lower q h, true, d)
                        :: ((List.range (sk.qCount h
                              (sk.stageScope h k) i)).map fun t =>
                            (askedOut (q, h), true, qacc + t))
                        ++ evs,
                       cnts, ok) := by
                  simp only [peerBlockA.chunks]
                  rw [if_neg (by simpa using h0)]
                  simp only [hget, hkind, hkindv, qCountA, hrecv,
                    Option.map, hsc', ← hqc, hcall]
                have hheads : ((Chan.wire q h, true, w) : Ev)
                      :: (Chan.lower q h, true, d)
                      :: ((List.range (sk.qCount h
                            (sk.stageScope h k) i)).map fun t =>
                          (askedOut (q, h), true, qacc + t))
                    = Sched.childChunk sk (q, h) k i := by
                  rw [hchunk, hw, hd]
                  show _ = ((wireOut (q, h) : Chan), true, _) :: _
                  rw [show (wireOut (q, h) : Chan) = Chan.wire q h
                    from rfl,
                    show (lowerOut (q, h) : Chan) = Chan.lower q h
                    from rfl]
                  congr 2
                  refine List.map_congr_left fun t _ => ?_
                  rw [hq]
                rw [hstep, hrange, List.flatMap_cons]
                have hsplit : ((Chan.wire q h, true, w) : Ev)
                      :: (Chan.lower q h, true, d)
                      :: ((List.range (sk.qCount h
                            (sk.stageScope h k) i)).map fun t =>
                          (askedOut (q, h), true, qacc + t))
                      ++ evs
                    = Sched.childChunk sk (q, h) k i ++ evs := by
                  rw [← hheads]
                constructor
                · rw [hsplit]
                  obtain ⟨t, ht⟩ := hpre'
                  exact ⟨t, by rw [List.append_assoc, ht]⟩
                · intro hok
                  obtain ⟨heq, hcnts⟩ := hcomp' hok
                  have heq' : evs = List.flatMap
                      (Sched.childChunk sk (q, h) k)
                      (List.range' (i + 1)
                        ((sk.scope (sk.stageScope h k)).kids.length
                          - (i + 1))) := heq
                  refine ⟨?_, hcnts⟩
                  rw [hsplit, heq']


/-- A flatMap of singletons is a map. -/
private theorem flatMap_singleton {α β : Type _} (g : α → β)
    (l : List α) : l.flatMap (fun a => [g a]) = l.map g := by
  induction l with
  | nil => rfl
  | cons a l ih => rw [List.flatMap_cons, List.map_cons, ih]; rfl

/-- The leaf stage owes no queries. -/
private theorem qOf_zero (u : Nat) : sk.qOf 0 u = 0 := by
  unfold Skel.qOf
  have hz : ∀ i, sk.qCount 0 u i = 0 := by
    intro i
    unfold Skel.qCount Skel.childIsD
    rw [if_pos (by simp)]
  induction List.range (sk.nChildren 0 u) with
  | nil => rfl
  | cons a l ih =>
      rw [List.foldl_cons, hz a]
      exact ih

/-- One announced block against its true block: a prefix of
`scopeBlockE`, exact — with the counters advanced one scope — when the
layout saw every record it needed. -/
private theorem peerBlockA_spec (hwf : sk.wellFormed = true)
    {p : Party} {tr : List MObs} (q : Party) {h k : Nat}
    (hh : h < sk.rootH) (hk : k < sk.stageLen h) :
    (peerBlockA (aviewOf sk p tr) q h k (sk.stageScope h k)
        (sk.wiresBefore h k) (sk.dsBefore h k) (sk.qsBefore h k)).1
      <+: Sched.scopeBlockE sk (q, h) k
    ∧ ((peerBlockA (aviewOf sk p tr) q h k (sk.stageScope h k)
          (sk.wiresBefore h k) (sk.dsBefore h k) (sk.qsBefore h k)).2.2
        = true →
        (peerBlockA (aviewOf sk p tr) q h k (sk.stageScope h k)
            (sk.wiresBefore h k) (sk.dsBefore h k)
            (sk.qsBefore h k)).1
          = Sched.scopeBlockE sk (q, h) k
        ∧ (peerBlockA (aviewOf sk p tr) q h k (sk.stageScope h k)
            (sk.wiresBefore h k) (sk.dsBefore h k)
            (sk.qsBefore h k)).2.1
          = (sk.wiresBefore h (k + 1), sk.dsBefore h (k + 1),
             sk.qsBefore h (k + 1))) := by
  have hpro : Sched.scopeBlockE sk (q, h) k
      = (Chan.wire q.other (h + 1), false, k)
        :: (Chan.asked q h, false, k)
        :: Sched.scopeSendsE sk (q, h) k := rfl
  cases hrec : (aviewOf sk p tr).rec? (sk.stageScope h k) with
  | none =>
      have hstep : peerBlockA (aviewOf sk p tr) q h k
          (sk.stageScope h k) (sk.wiresBefore h k) (sk.dsBefore h k)
          (sk.qsBefore h k)
          = ([(Chan.wire q.other (h + 1), false, k),
              (Chan.asked q h, false, k)],
             (sk.wiresBefore h k, sk.dsBefore h k, sk.qsBefore h k),
             false) := by
        simp only [peerBlockA, hrec]
      rw [hstep, hpro]
      refine ⟨⟨Sched.scopeSendsE sk (q, h) k, rfl⟩, ?_⟩
      intro hok
      cases hok
  | some sc =>
      obtain ⟨hsc, hann⟩ := rec?_some_inv hrec
      by_cases h0 : h = 0
      · -- the leaf stage: one supply wire per request, never disputed
        subst h0
        have hkids : (sk.scope (sk.stageScope 0 k)).kids = [] := by
          have hht := Sched.stageScope_height sk (h := 0) (k := k) hk
          have hreal := stageScope_lt_scopes sk hk
          unfold Skel.wellFormed at hwf
          simp only [Bool.and_eq_true, List.all_eq_true,
            decide_eq_true_eq] at hwf
          have hper := (hwf.1.1.1.1.1.2 (sk.stageScope 0 k)
            (List.mem_range.mpr hreal)).1.1.2
          rw [Bool.or_eq_true] at hper
          rcases hper with hne | hemp
          · exfalso
            simp only [bne_iff_ne, ne_eq] at hne
            omega
          · simpa using hemp
        have hz : ((0 : Nat) == 0) = true := rfl
        have hn : (if ((0 : Nat) == 0) = true then sc.leafReqs
            else sc.kids.length) = sc.leafReqs := by
          rw [if_pos hz]
        have hchunks : peerBlockA.chunks (aviewOf sk p tr) q 0
            (sk.wiresBefore 0 k) sc
            (if ((0 : Nat) == 0) = true then sc.leafReqs
              else sc.kids.length)
            0 (sk.wiresBefore 0 k) (sk.dsBefore 0 k) (sk.qsBefore 0 k)
            (sc.kids.length + 1)
            = ((List.range sc.leafReqs).map fun j =>
                ((Chan.wire q 0 : Chan), true, sk.wiresBefore 0 k + j),
               (sk.wiresBefore 0 k + sc.leafReqs, sk.dsBefore 0 k,
                sk.qsBefore 0 k), true) := by
          simp only [peerBlockA.chunks, hn]
          rw [if_pos hz]
        have hstep : peerBlockA (aviewOf sk p tr) q 0 k
            (sk.stageScope 0 k) (sk.wiresBefore 0 k) (sk.dsBefore 0 k)
            (sk.qsBefore 0 k)
            = ((Chan.wire q.other 1, false, k)
                :: (Chan.asked q 0, false, k)
                :: ((List.range sc.leafReqs).map fun j =>
                    ((Chan.wire q 0 : Chan), true, sk.wiresBefore 0 k + j))
                ++ [(Chan.upper q 0, true, k)],
               (sk.wiresBefore 0 k + sc.leafReqs, sk.dsBefore 0 k,
                sk.qsBefore 0 k), true) := by
          simp only [peerBlockA, hrec, hchunks]
          rfl
        have hnc : sk.nChildren 0 (sk.stageScope 0 k) = sc.leafReqs := by
          unfold Skel.nChildren
          rw [if_pos hz, ← hsc]
        have hcc : ∀ i, Sched.childChunk sk (q, 0) k i
            = [((Chan.wire q 0 : Chan), true,
                sk.wiresBefore 0 k + i)] := by
          intro i
          unfold Sched.childChunk
          rw [if_neg (by unfold Skel.childIsD; simp)]
          rfl
        have hsred : Sched.scopeSendsE sk (q, 0) k
            = ((List.range (sk.nChildren 0 (sk.stageScope 0 k))).map
                (Sched.childChunk sk (q, 0) k)).flatten
              ++ [(Chan.upper q 0, true, k)] := rfl
        have htrue : Sched.scopeBlockE sk (q, 0) k
            = (Chan.wire q.other 1, false, k)
              :: (Chan.asked q 0, false, k)
              :: ((List.range sc.leafReqs).map fun j =>
                  ((Chan.wire q 0 : Chan), true, sk.wiresBefore 0 k + j))
              ++ [(Chan.upper q 0, true, k)] := by
          rw [hpro, hsred, hnc, List.map_congr_left
            (fun i _ => hcc i), ← List.flatMap_def,
            flatMap_singleton (fun i =>
              ((Chan.wire q 0 : Chan), true, sk.wiresBefore 0 k + i))]
          rfl
        rw [hstep, htrue]
        refine ⟨List.prefix_refl _, fun _ => ⟨rfl, ?_⟩⟩
        show (sk.wiresBefore 0 k + sc.leafReqs, sk.dsBefore 0 k,
          sk.qsBefore 0 k) = _
        have hdz : sk.dOf 0 (sk.stageScope 0 k) = 0 := by
          unfold Skel.dOf
          rw [if_pos hz]
        rw [Sched.wiresBefore_succ sk hk, Sched.dsBefore_succ sk hk,
          Sched.qsBefore_succ sk hk, hnc, hdz, qOf_zero]
        simp
      · -- an interior stage: the chunk loop transcribes
        obtain ⟨hpre, hcomp⟩ := chunksA_spec hwf q h0 hk hann
          (sk.wiresBefore h k)
          ((sk.scope (sk.stageScope h k)).kids.length + 1) 0
          (sk.wiresBefore h k) (sk.dsBefore h k) (sk.qsBefore h k)
          (by omega) (by omega) (by omega) (by simp)
          (by simp)
        have hn' : (if (h == 0) = true then
              (sk.scope (sk.stageScope h k)).leafReqs
            else (sk.scope (sk.stageScope h k)).kids.length)
            = (sk.scope (sk.stageScope h k)).kids.length := by
          rw [if_neg (by simpa using h0)]
        rcases hcall : peerBlockA.chunks (aviewOf sk p tr) q h
            (sk.wiresBefore h k) (sk.scope (sk.stageScope h k))
            (sk.scope (sk.stageScope h k)).kids.length
            0 (sk.wiresBefore h k) (sk.dsBefore h k) (sk.qsBefore h k)
            ((sk.scope (sk.stageScope h k)).kids.length + 1)
            with ⟨evs, cnts, ok⟩
        rw [hcall] at hpre hcomp
        have hflat : (List.range' 0
            ((sk.scope (sk.stageScope h k)).kids.length - 0)).flatMap
              (Sched.childChunk sk (q, h) k)
            = ((List.range ((sk.scope (sk.stageScope h k)).kids.length)).map
                (Sched.childChunk sk (q, h) k)).flatten := by
          rw [← List.flatMap_def, Nat.sub_zero, List.range_eq_range']
        have hsred2 : Sched.scopeSendsE sk (q, h) k
            = ((List.range (sk.nChildren h (sk.stageScope h k))).map
                (Sched.childChunk sk (q, h) k)).flatten
              ++ [(Chan.upper q h, true, k)] := rfl
        have hn2 : sk.nChildren h (sk.stageScope h k)
            = (sk.scope (sk.stageScope h k)).kids.length := by
          unfold Skel.nChildren
          rw [if_neg (by simpa using h0)]
        have hsends : Sched.scopeSendsE sk (q, h) k
            = ((List.range ((sk.scope (sk.stageScope h k)).kids.length)).map
                (Sched.childChunk sk (q, h) k)).flatten
              ++ [(Chan.upper q h, true, k)] := by
          rw [hsred2, hn2]
        cases ok with
        | true =>
            obtain ⟨heq, hcnts⟩ := hcomp rfl
            have heq' : evs = (List.range' 0
                ((sk.scope (sk.stageScope h k)).kids.length - 0)).flatMap
                  (Sched.childChunk sk (q, h) k) := heq
            have hstep : peerBlockA (aviewOf sk p tr) q h k
                (sk.stageScope h k) (sk.wiresBefore h k)
                (sk.dsBefore h k) (sk.qsBefore h k)
                = ((Chan.wire q.other (h + 1), false, k)
                    :: (Chan.asked q h, false, k)
                    :: evs ++ [(Chan.upper q h, true, k)],
                   cnts, true) := by
              simp only [peerBlockA, hrec, hsc, hn', hcall]
              rfl
            rw [hstep, hpro, hsends]
            have hcnts' : cnts = (sk.wiresBefore h (k + 1),
                sk.dsBefore h (k + 1), sk.qsBefore h (k + 1)) := hcnts
            refine ⟨?_, fun _ => ⟨?_, hcnts'⟩⟩
            · rw [heq', hflat]
              exact List.prefix_refl _
            · rw [heq', hflat]
              rfl
        | false =>
            have hstep : peerBlockA (aviewOf sk p tr) q h k
                (sk.stageScope h k) (sk.wiresBefore h k)
                (sk.dsBefore h k) (sk.qsBefore h k)
                = ((Chan.wire q.other (h + 1), false, k)
                    :: (Chan.asked q h, false, k)
                    :: evs ++ [],
                   cnts, false) := by
              simp only [peerBlockA, hrec, hsc, hn', hcall]
              rfl
            rw [hstep, hpro, hsends]
            refine ⟨?_, fun hok => by cases hok⟩
            obtain ⟨t, ht⟩ := hpre
            refine ⟨t ++ [(Chan.upper q h, true, k)], ?_⟩
            show ((Chan.wire q.other (h + 1), false, k)
              :: (Chan.asked q h, false, k) :: (evs ++ [])) ++ _ = _
            rw [List.append_nil]
            show (Chan.wire q.other (h + 1), false, k)
              :: (Chan.asked q h, false, k) :: (evs ++ (t
                ++ [(Chan.upper q h, true, k)])) = _
            rw [← List.append_assoc, ht, hflat]

/-- The announced walk layout from block `k` onward is a prefix of the
true remaining blocks, provided the announced items name the true
stage scopes. -/
private theorem goA_spec (hwf : sk.wellFormed = true)
    {p : Party} {tr : List MObs} {h : Nat} (hh : h < sk.rootH) :
    ∀ (is : List Nat) (k : Nat),
      (∀ j, j < is.length → is.getD j 0 = sk.stageScope h (k + j)) →
      k + is.length ≤ sk.stageLen h →
      peerWalkTraceA.go (aviewOf sk p tr) h p.other is k
          (sk.wiresBefore h k) (sk.dsBefore h k) (sk.qsBefore h k)
        <+: (List.range' k (sk.stageLen h - k)).flatMap
            (Sched.scopeBlockE sk (p.other, h)) := by
  intro is
  induction is with
  | nil =>
      intro k _ _
      show ([] : List Ev) <+: _
      exact List.nil_prefix
  | cons u rest ih =>
      intro k hjs hlen
      have hu : u = sk.stageScope h k := by
        have := hjs 0 (by simp)
        simpa using this
      have hk : k < sk.stageLen h := by
        have := hlen
        simp only [List.length_cons] at this
        omega
      obtain ⟨hpre, hcomp⟩ := peerBlockA_spec hwf p.other hh hk
        (p := p) (tr := tr)
      have hrange : List.range' k (sk.stageLen h - k)
          = k :: List.range' (k + 1) (sk.stageLen h - (k + 1)) := by
        rw [show sk.stageLen h - k = (sk.stageLen h - (k + 1)) + 1
          from by omega]
        rfl
      rcases hcall : peerBlockA (aviewOf sk p tr) p.other h k
          (sk.stageScope h k) (sk.wiresBefore h k) (sk.dsBefore h k)
          (sk.qsBefore h k) with ⟨evs, cnts, ok⟩
      rw [hcall] at hpre hcomp
      rw [hrange, List.flatMap_cons]
      cases ok with
      | false =>
          have hstep : peerWalkTraceA.go (aviewOf sk p tr) h p.other
              (u :: rest) k (sk.wiresBefore h k) (sk.dsBefore h k)
              (sk.qsBefore h k) = evs := by
            simp [peerWalkTraceA.go, hu, hcall]
          rw [hstep]
          exact hpre.trans (List.prefix_append _ _)
      | true =>
          obtain ⟨heq, hcnts⟩ := hcomp rfl
          have heq' : evs = Sched.scopeBlockE sk (p.other, h) k := heq
          have hcnts' : cnts = (sk.wiresBefore h (k + 1),
              sk.dsBefore h (k + 1), sk.qsBefore h (k + 1)) := hcnts
          subst hcnts'
          have hstep : peerWalkTraceA.go (aviewOf sk p tr) h p.other
              (u :: rest) k (sk.wiresBefore h k) (sk.dsBefore h k)
              (sk.qsBefore h k)
              = evs ++ peerWalkTraceA.go (aviewOf sk p tr) h p.other
                  rest (k + 1) (sk.wiresBefore h (k + 1))
                  (sk.dsBefore h (k + 1)) (sk.qsBefore h (k + 1)) := by
            simp [peerWalkTraceA.go, hu, hcall]
          rw [hstep, heq']
          have hrec := ih (k + 1)
            (fun j hj => by
              have := hjs (j + 1) (by simpa using Nat.succ_lt_succ hj)
              simpa [Nat.add_assoc, Nat.add_comm 1 j,
                Nat.add_left_comm] using this)
            (by simp only [List.length_cons] at hlen; omega)
          obtain ⟨t, ht⟩ := hrec
          exact ⟨t, by rw [List.append_assoc, ht]⟩

/-- The announced walk trace is a prefix of the true `.impl` walk
trace (the module-doc claim of Mux/Causal.lean, walk family). -/
theorem peerWalkTraceA_prefix (hwf : sk.wellFormed = true)
    (p : Party) (tr : List MObs) {h : Nat} (hh : h < sk.rootH) :
    peerWalkTraceA (aviewOf sk p tr) h
      <+: Sched.walkEventsE sk (p.other, h) := by
  have hitems : (stageScopesA (aviewOf sk p tr) h).1
      <+: sk.stageScopes h := by
    unfold stageScopesA
    by_cases htop : (h + 1 == (aviewOf sk p tr).rootH) = true
    · rw [if_pos htop]
      have htop' : h + 1 = sk.rootH := beq_iff_eq.mp htop
      show [0] <+: _
      unfold Skel.stageScopes
      rw [htop', Sched.wf_root_stage hwf]
      exact List.prefix_refl _
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
      have := (levelA_spec hwf p tr ((aviewOf sk p tr).rootH - (h + 1))
        (by show sk.rootH - (h + 1) ≤ sk.rootH; omega)).1
      rw [hsteps] at this
      exact this
  have hlen := hitems.length_le
  have hjs : ∀ j, j < (stageScopesA (aviewOf sk p tr) h).1.length →
      (stageScopesA (aviewOf sk p tr) h).1.getD j 0
        = sk.stageScope h j := by
    intro j hj
    obtain ⟨t, ht⟩ := hitems
    unfold Skel.stageScope
    rw [← ht, List.getD_eq_getElem?_getD, List.getD_eq_getElem?_getD,
      List.getElem?_append_left hj]
  show peerWalkTraceA.go (aviewOf sk p tr) h p.other
      (stageScopesA (aviewOf sk p tr) h).1 0 0 0 0 <+: _
  have hstageLen : sk.stageLen h = (sk.stageScopes h).length := rfl
  have hgo := goA_spec (p := p) (tr := tr) hwf hh
    (stageScopesA (aviewOf sk p tr) h).1 0
    (fun j hj => by rw [Nat.zero_add]; exact hjs j hj)
    (by rw [hstageLen]; omega)
  unfold Sched.walkEventsE
  rw [List.range_eq_range']
  have h0w : sk.wiresBefore h 0 = 0 := rfl
  have h0d : sk.dsBefore h 0 = 0 := rfl
  have h0q : sk.qsBefore h 0 = 0 := rfl
  rw [h0w, h0d, h0q, Nat.sub_zero] at hgo
  exact hgo


-- ==================== the opener, finale, and absorber transcriptions

/-- The announced opener trace is a prefix of the peer opener's true
trace. -/
theorem peerOpenTraceA_prefix (hwf : sk.wellFormed = true)
    (p : Party) (tr : List MObs) :
    peerOpenTraceA (aviewOf sk p tr)
      <+: (if p = Party.I then Sched.ropenEvents sk
           else Sched.iopenEvents sk) := by
  cases p with
  | I =>
      rw [if_pos rfl]
      show ([((Chan.wire Party.I sk.rootH : Chan), false, 0),
        ((Chan.wire Party.R sk.rootH : Chan), true, 0),
        ((Chan.rootres : Chan), true, 0)]
          ++ (match (aviewOf sk Party.I tr).rec? 0 with
              | none => []
              | some sc =>
                  (List.range sc.kids.length).map fun j =>
                    ((Chan.asked Party.R (sk.rootH - 2) : Chan), true, j)))
        <+: _
      unfold Sched.ropenEvents
      cases hrec : (aviewOf sk Party.I tr).rec? 0 with
      | none =>
          refine ⟨(List.range sk.rootPending).map fun j =>
            ((Chan.asked Party.R (sk.rootH - 2) : Chan), true, j), ?_⟩
          rfl
      | some sc =>
          obtain ⟨hsc, -⟩ := rec?_some_inv hrec
          refine ⟨[], ?_⟩
          rw [List.append_nil]
          show _ = (Chan.wire Party.I sk.rootH, false, 0)
            :: (Chan.wire Party.R sk.rootH, true, 0)
            :: (Chan.rootres, true, 0)
            :: ((List.range sk.rootPending).map fun j =>
                (Chan.asked Party.R (sk.rootH - 2), true, j))
          rw [show sk.rootPending = sc.kids.length from by
            rw [hsc]; rfl]
          rfl
  | R =>
      rw [if_neg (by simp)]
      show [((Chan.wire Party.I sk.rootH : Chan), true, 0),
        ((Chan.asked Party.I (sk.rootH - 1) : Chan), true, 0)] <+: _
      exact List.prefix_refl _

/-- Each announced finale trace is a prefix of its true finale trace. -/
theorem peerFinTracesA_prefix (hwf : sk.wellFormed = true)
    (p : Party) (tr : List MObs) :
    ∀ T ∈ peerFinTracesA (aviewOf sk p tr),
      T <+: (if p = Party.I then Sched.finEvents sk
             else [((Chan.rootret : Chan), false, 0)]) := by
  intro T hT
  cases p with
  | I =>
      rw [if_pos rfl]
      have hT' : T = ((Chan.rootres : Chan), false, 0)
          :: (match (aviewOf sk Party.I tr).rec? 0 with
              | none => []
              | some sc =>
                  (List.range sc.kids.length).map fun j =>
                    ((Chan.rootrets : Chan), false, j)) := by
        have : T ∈ [((Chan.rootres : Chan), false, 0)
            :: (match (aviewOf sk Party.I tr).rec? 0 with
                | none => []
                | some sc =>
                    (List.range sc.kids.length).map fun j =>
                      ((Chan.rootrets : Chan), false, j))] := hT
        simpa using this
      subst hT'
      unfold Sched.finEvents
      cases hrec : (aviewOf sk Party.I tr).rec? 0 with
      | none =>
          exact ⟨(List.range sk.rootPending).map fun j =>
            ((Chan.rootrets : Chan), false, j), rfl⟩
      | some sc =>
          obtain ⟨hsc, -⟩ := rec?_some_inv hrec
          refine ⟨[], ?_⟩
          rw [List.append_nil]
          rw [show sk.rootPending = sc.kids.length from by
            rw [hsc]; rfl]
  | R =>
      rw [if_neg (by simp)]
      have hT' : T = [((Chan.rootret : Chan), false, 0)] := by
        have : T ∈ [[((Chan.rootret : Chan), false, 0)]] := hT
        simpa using this
      subst hT'
      exact List.prefix_refl _

/-- Nat list sums grow along sublists. -/
private theorem sublist_sum_le :
    ∀ {l₁ l₂ : List Nat}, l₁.Sublist l₂ → l₁.sum ≤ l₂.sum := by
  intro l₁ l₂ hs
  induction hs with
  | slnil => exact Nat.le_refl _
  | cons a hs ih =>
      rw [List.sum_cons]
      omega
  | cons_cons a hs ih =>
      rw [List.sum_cons, List.sum_cons]
      omega

/-- The absorber's announced total against a kind-known list: at most
the list's own D-request sum. -/
private theorem totalA_le_filter {p : Party} {tr : List MObs} :
    ∀ (l : List Nat),
      (∀ u ∈ l, (aviewOf sk p tr).kind? u
        = some ((sk.scope u).kind)) →
      (peerAbsorbTraceA.total (aviewOf sk p tr) l).1
        ≤ ((l.filter (fun s => (sk.scope s).kind == Kind.D)).map
            (fun s => (sk.scope s).leafReqs)).sum := by
  intro l
  induction l with
  | nil =>
      intro _
      show (0 : Nat) ≤ _
      omega
  | cons u rest ih =>
      intro hkinds
      have hrest := ih (fun v hv => hkinds v (List.mem_cons_of_mem _ hv))
      rw [peerAbsorbTraceA.total]
      by_cases hD : ((aviewOf sk p tr).kind? u == some Kind.D) = true
      · have hkindu : (sk.scope u).kind = Kind.D := by
          have := hkinds u (List.mem_cons_self ..)
          rw [this] at hD
          simpa using hD
        rw [if_pos hD, List.filter_cons, if_pos (by simp [hkindu]),
          List.map_cons, List.sum_cons]
        cases hrec : (aviewOf sk p tr).rec? u with
        | none =>
            show (0 : Nat) ≤ _
            omega
        | some sc =>
            obtain ⟨hsc, -⟩ := rec?_some_inv hrec
            show sc.leafReqs
              + (peerAbsorbTraceA.total (aviewOf sk p tr) rest).1 ≤ _
            rw [hsc]
            omega
      · have hkindu : (sk.scope u).kind ≠ Kind.D := by
          intro hc
          have := hkinds u (List.mem_cons_self ..)
          rw [this, hc] at hD
          simp at hD
        rw [if_neg hD, List.filter_cons,
          if_neg (by simpa using hkindu)]
        exact hrest

/-- The absorber's announced supply total never exceeds the true
one. -/
private theorem totalA_le (hwf : sk.wellFormed = true)
    {p : Party} {tr : List MObs} {l : List Nat}
    (hsub : l.Sublist (sk.scopesAt 1))
    (hkinds : ∀ u ∈ l, (aviewOf sk p tr).kind? u
      = some ((sk.scope u).kind)) :
    (peerAbsorbTraceA.total (aviewOf sk p tr) l).1
      ≤ sk.totalLeafReqs := by
  have htotal : sk.totalLeafReqs
      = (((sk.scopesAt 1).filter
          (fun s => (sk.scope s).kind == Kind.D)).map
            (fun s => (sk.scope s).leafReqs)).sum := by
    unfold Skel.totalLeafReqs
    rw [foldl_add_eq_sum, Nat.zero_add]
  rw [htotal]
  exact Nat.le_trans (totalA_le_filter l hkinds)
    (sublist_sum_le ((hsub.filter _).map _))

/-- The announced absorber trace is a prefix of the true one: the
blocks are shape-constant, so the count bound is the whole story. -/
theorem peerAbsorbTraceA_prefix (hwf : sk.wellFormed = true)
    (p : Party) (tr : List MObs) :
    peerAbsorbTraceA (aviewOf sk p tr) <+: Sched.absorbEvents sk := by
  unfold peerAbsorbTraceA
  by_cases hp : (p == Party.I) = true
  · rw [if_pos (show ((aviewOf sk p tr).party == Party.I) = true
      from hp)]
    exact List.nil_prefix
  · rw [if_neg (show ¬ ((aviewOf sk p tr).party == Party.I) = true
      from hp)]
    have hge := (wf_rootH hwf).2
    have hlvl := levelA_spec hwf p tr (sk.rootH - 1) (by omega)
    have hsteps : sk.rootH - (sk.rootH - 1) = 1 := by omega
    rw [hsteps] at hlvl
    have hle := totalA_le hwf (hlvl.1.sublist)
      (fun u hu => hlvl.2.1 u hu)
    unfold Sched.absorbEvents
    have hpre : List.range ((peerAbsorbTraceA.total (aviewOf sk p tr)
        (levelA (aviewOf sk p tr) (sk.rootH - 1)).1).1)
        <+: List.range sk.totalLeafReqs := by
      rw [show List.range ((peerAbsorbTraceA.total (aviewOf sk p tr)
          (levelA (aviewOf sk p tr) (sk.rootH - 1)).1).1)
          = (List.range sk.totalLeafReqs).take
              ((peerAbsorbTraceA.total (aviewOf sk p tr)
                (levelA (aviewOf sk p tr) (sk.rootH - 1)).1).1) from by
        rw [List.take_range, Nat.min_eq_left hle]]
      exact List.take_prefix _ _
    show (List.range ((peerAbsorbTraceA.total (aviewOf sk p tr)
        (levelA (aviewOf sk p tr)
          ((aviewOf sk p tr).rootH - 1)).1).1)).flatMap _ <+: _
    exact prefix_flatMap _ hpre


-- ================================== the assembler transcription

/-- The announced pending entries name the true resolution list: every
`some` entry is `pendAt`, and the list never outruns the true one. -/
private theorem asmPendsA_spec (hwf : sk.wellFormed = true)
    (p : Party) (tr : List MObs) {j : Nat}
    (hj1 : 1 ≤ j) (hjr : j ≤ sk.rootH) :
    (asmPendsA (aviewOf sk p tr) j).length
      ≤ (sk.asmResList p.other j).length
    ∧ (∀ m v, (asmPendsA (aviewOf sk p tr) j).getD m none = some v →
        v = sk.pendAt p.other j m) := by
  -- the census the entries are read off
  have hitems : (if (j == (aviewOf sk p tr).rootH) = true
        then ([0], true)
        else levelA (aviewOf sk p tr) ((aviewOf sk p tr).rootH - j)).1
        <+: sk.scopesAt j
      ∧ ∀ u ∈ (if (j == (aviewOf sk p tr).rootH) = true
        then (([0] : List Nat), true)
        else levelA (aviewOf sk p tr) ((aviewOf sk p tr).rootH - j)).1,
        (aviewOf sk p tr).kind? u = some ((sk.scope u).kind) := by
    by_cases hj : (j == (aviewOf sk p tr).rootH) = true
    · rw [if_pos hj]
      have hj' : j = sk.rootH := beq_iff_eq.mp hj
      subst hj'
      refine ⟨?_, ?_⟩
      · rw [Sched.wf_root_stage hwf]
        exact List.prefix_refl _
      · intro u hu
        have hu' : u ∈ [(0 : Nat)] := hu
        rw [List.mem_singleton] at hu'
        subst hu'
        show (aviewOf sk p tr).kind? 0 = _
        rw [AView.kind?, if_pos (show ((0 : Nat) == 0) = true from rfl),
          wf_root_kind hwf]
    · rw [if_neg hj]
      have hj' : j ≠ sk.rootH := fun hc => hj (beq_iff_eq.mpr hc)
      have hlvl := levelA_spec hwf p tr (sk.rootH - j) (by omega)
      have hsteps : sk.rootH - (sk.rootH - j) = j := by omega
      rw [hsteps] at hlvl
      exact ⟨hlvl.1, hlvl.2.1⟩
  obtain ⟨hpre, hkinds⟩ := hitems
  -- positional identification of the census items
  have hgetd : ∀ m, m < (if (j == (aviewOf sk p tr).rootH) = true
        then (([0] : List Nat), true)
        else levelA (aviewOf sk p tr) ((aviewOf sk p tr).rootH - j)).1.length →
      (if (j == (aviewOf sk p tr).rootH) = true
        then (([0] : List Nat), true)
        else levelA (aviewOf sk p tr) ((aviewOf sk p tr).rootH - j)).1.getD m 0
        = (sk.scopesAt j).getD m 0 := by
    intro m hm
    obtain ⟨t, ht⟩ := hpre
    rw [← ht, List.getD_eq_getElem?_getD, List.getD_eq_getElem?_getD,
      List.getElem?_append_left hm]
  cases hside : asks p.other j with
  | true =>
      -- asker side: one entry per level scope, pending its D count
      have hshape : asmPendsA (aviewOf sk p tr) j
          = (if (j == (aviewOf sk p tr).rootH) = true
              then (([0] : List Nat), true)
              else levelA (aviewOf sk p tr)
                ((aviewOf sk p tr).rootH - j)).1.map
              (fun u => match (aviewOf sk p tr).rec? u with
                | none => none
                | some sc => some (sc.kids.countP
                    fun v => (aviewOf sk p tr).kind? v == some Kind.D)) := by
        have hraw : asmPendsA (aviewOf sk p tr) j
            = if asks p.other j = true then
                (if (j == (aviewOf sk p tr).rootH) = true
                  then (([0] : List Nat), true)
                  else levelA (aviewOf sk p tr)
                    ((aviewOf sk p tr).rootH - j)).1.map
                  (fun u => match (aviewOf sk p tr).rec? u with
                    | none => none
                    | some sc => some (sc.kids.countP
                        fun v => (aviewOf sk p tr).kind? v
                          == some Kind.D))
              else
                ((if (j == (aviewOf sk p tr).rootH) = true
                  then (([0] : List Nat), true)
                  else levelA (aviewOf sk p tr)
                    ((aviewOf sk p tr).rootH - j)).1.filter
                  (fun u => (aviewOf sk p tr).kind? u
                    == some Kind.D)).map
                  (fun u => match (aviewOf sk p tr).rec? u with
                    | none => none
                    | some sc => some (if (j == 1) = true
                        then sc.leafReqs else sc.kids.length)) := rfl
        rw [hraw, if_pos hside]
      have hres : sk.asmResList p.other j
          = (sk.scopesAt j).map (fun s => sk.dCount s) := by
        unfold Skel.asmResList
        rw [if_pos hside]
      constructor
      · rw [hshape, hres, List.length_map, List.length_map]
        exact hpre.length_le
      · intro m v hv
        rw [hshape] at hv
        have hm : m < (if (j == (aviewOf sk p tr).rootH) = true
            then (([0] : List Nat), true)
            else levelA (aviewOf sk p tr)
              ((aviewOf sk p tr).rootH - j)).1.length := by
          by_contra hge
          rw [List.getD_eq_getElem?_getD,
            List.getElem?_eq_none (by
              rw [List.length_map]; omega)] at hv
          cases hv
        rw [List.getD_eq_getElem?_getD, List.getElem?_map,
          List.getElem?_eq_getElem hm] at hv
        simp only [Option.map_some, Option.getD_some] at hv
        cases hrec : (aviewOf sk p tr).rec?
            ((if (j == (aviewOf sk p tr).rootH) = true
              then (([0] : List Nat), true)
              else levelA (aviewOf sk p tr)
                ((aviewOf sk p tr).rootH - j)).1[m]) with
        | none => rw [hrec] at hv; cases hv
        | some sc =>
            rw [hrec] at hv
            injection hv with hv
            obtain ⟨hsc, hann⟩ := rec?_some_inv hrec
            have hcnt : sc.kids.countP
                (fun v => (aviewOf sk p tr).kind? v == some Kind.D)
                = sk.dCount ((if (j == (aviewOf sk p tr).rootH) = true
                    then (([0] : List Nat), true)
                    else levelA (aviewOf sk p tr)
                      ((aviewOf sk p tr).rootH - j)).1[m]) := by
              unfold Skel.dCount
              rw [← List.countP_eq_length_filter, hsc]
              refine List.countP_congr ?_
              intro v hvk
              rw [kind?_aviewOf_of_kid hwf hann hvk]
              cases (sk.scope v).kind <;> simp
            have hm2 : m < (sk.scopesAt j).length := by
              have := hpre.length_le
              omega
            have hq2 : (sk.scopesAt j)[m]?
                = (if (j == (aviewOf sk p tr).rootH) = true
                  then (([0] : List Nat), true)
                  else levelA (aviewOf sk p tr)
                    ((aviewOf sk p tr).rootH - j)).1[m]? := by
              obtain ⟨t, ht⟩ := hpre
              rw [← ht, List.getElem?_append_left hm]
            rw [List.getElem?_eq_getElem hm2,
              List.getElem?_eq_getElem hm] at hq2
            injection hq2 with hq2
            rw [← hv, hcnt]
            unfold Skel.pendAt
            rw [hres, List.getD_eq_getElem?_getD, List.getElem?_map,
              List.getElem?_eq_getElem hm2]
            simp only [Option.map_some, Option.getD_some]
            rw [hq2]
  | false =>
      -- answerer side: one entry per D scope, pending its census
      have hfilter : (if (j == (aviewOf sk p tr).rootH) = true
            then (([0] : List Nat), true)
            else levelA (aviewOf sk p tr)
              ((aviewOf sk p tr).rootH - j)).1.filter
            (fun u => (aviewOf sk p tr).kind? u == some Kind.D)
          <+: (sk.scopesAt j).filter
            (fun s => (sk.scope s).kind == Kind.D) := by
        rw [List.filter_congr (fun u hu => by rw [hkinds u hu])]
        exact hpre.filter _
      have hshape : asmPendsA (aviewOf sk p tr) j
          = ((if (j == (aviewOf sk p tr).rootH) = true
              then (([0] : List Nat), true)
              else levelA (aviewOf sk p tr)
                ((aviewOf sk p tr).rootH - j)).1.filter
                (fun u => (aviewOf sk p tr).kind? u == some Kind.D)).map
              (fun u => match (aviewOf sk p tr).rec? u with
                | none => none
                | some sc => some (if (j == 1) = true then sc.leafReqs
                    else sc.kids.length)) := by
        have hraw : asmPendsA (aviewOf sk p tr) j
            = if asks p.other j = true then
                (if (j == (aviewOf sk p tr).rootH) = true
                  then (([0] : List Nat), true)
                  else levelA (aviewOf sk p tr)
                    ((aviewOf sk p tr).rootH - j)).1.map
                  (fun u => match (aviewOf sk p tr).rec? u with
                    | none => none
                    | some sc => some (sc.kids.countP
                        fun v => (aviewOf sk p tr).kind? v
                          == some Kind.D))
              else
                ((if (j == (aviewOf sk p tr).rootH) = true
                  then (([0] : List Nat), true)
                  else levelA (aviewOf sk p tr)
                    ((aviewOf sk p tr).rootH - j)).1.filter
                  (fun u => (aviewOf sk p tr).kind? u
                    == some Kind.D)).map
                  (fun u => match (aviewOf sk p tr).rec? u with
                    | none => none
                    | some sc => some (if (j == 1) = true
                        then sc.leafReqs else sc.kids.length)) := rfl
        rw [hraw, if_neg (by rw [hside]; simp)]
      have hres : sk.asmResList p.other j
          = ((sk.scopesAt j).filter
              (fun s => (sk.scope s).kind == Kind.D)).map
              (fun s => if (sk.scope s).height == 1
                then (sk.scope s).leafReqs
                else (sk.scope s).kids.length) := by
        unfold Skel.asmResList
        rw [if_neg (by rw [hside]; simp)]
      constructor
      · rw [hshape, hres, List.length_map, List.length_map]
        exact hfilter.length_le
      · intro m v hv
        rw [hshape] at hv
        have hm : m < ((if (j == (aviewOf sk p tr).rootH) = true
            then (([0] : List Nat), true)
            else levelA (aviewOf sk p tr)
              ((aviewOf sk p tr).rootH - j)).1.filter
              (fun u => (aviewOf sk p tr).kind? u
                == some Kind.D)).length := by
          by_contra hge
          rw [List.getD_eq_getElem?_getD,
            List.getElem?_eq_none (by
              rw [List.length_map]; omega)] at hv
          cases hv
        rw [List.getD_eq_getElem?_getD, List.getElem?_map,
          List.getElem?_eq_getElem hm] at hv
        simp only [Option.map_some, Option.getD_some] at hv
        cases hrec : (aviewOf sk p tr).rec?
            (((if (j == (aviewOf sk p tr).rootH) = true
              then (([0] : List Nat), true)
              else levelA (aviewOf sk p tr)
                ((aviewOf sk p tr).rootH - j)).1.filter
                (fun u => (aviewOf sk p tr).kind? u
                  == some Kind.D))[m]) with
        | none => rw [hrec] at hv; cases hv
        | some sc =>
            rw [hrec] at hv
            injection hv with hv
            obtain ⟨hsc, hann⟩ := rec?_some_inv hrec
            have hm2 : m < ((sk.scopesAt j).filter
                (fun s => (sk.scope s).kind == Kind.D)).length := by
              have := hfilter.length_le
              omega
            have hgd : ((if (j == (aviewOf sk p tr).rootH) = true
                then (([0] : List Nat), true)
                else levelA (aviewOf sk p tr)
                  ((aviewOf sk p tr).rootH - j)).1.filter
                  (fun u => (aviewOf sk p tr).kind? u
                    == some Kind.D))[m]
                = ((sk.scopesAt j).filter
                    (fun s => (sk.scope s).kind == Kind.D))[m] := by
              have hq : ((sk.scopesAt j).filter
                  (fun s => (sk.scope s).kind == Kind.D))[m]?
                  = ((if (j == (aviewOf sk p tr).rootH) = true
                    then (([0] : List Nat), true)
                    else levelA (aviewOf sk p tr)
                      ((aviewOf sk p tr).rootH - j)).1.filter
                      (fun u => (aviewOf sk p tr).kind? u
                        == some Kind.D))[m]? := by
                obtain ⟨t, ht⟩ := hfilter
                rw [← ht, List.getElem?_append_left hm]
              rw [List.getElem?_eq_getElem hm,
                List.getElem?_eq_getElem hm2] at hq
              injection hq with hq
              exact hq.symm
            unfold Skel.pendAt
            rw [hres, List.getD_eq_getElem?_getD, List.getElem?_map,
              List.getElem?_eq_getElem hm2]
            simp only [Option.map_some, Option.getD_some]
            have hheight : (sk.scope (((sk.scopesAt j).filter
                (fun s => (sk.scope s).kind == Kind.D))[m])).height
                = j := by
              have hmem := List.getElem_mem hm2
              exact (mem_scopesAt (List.mem_filter.mp hmem).1).2
            rw [← hv, hsc, hgd]
            by_cases h1 : j = 1
            · rw [if_pos (by rw [h1]; rfl),
                if_pos (by rw [hheight, h1]; rfl)]
            · rw [if_neg (by simpa using h1),
                if_neg (by rw [hheight]; simpa using h1)]

/-- The announced assembler layout from resolution `idx` onward is a
prefix of the true remaining blocks. -/
private theorem goAsm_spec (hwf : sk.wellFormed = true)
    {p : Party} {tr : List MObs} {j : Nat} :
    ∀ (ps : List (Option Nat)) (idx got : Nat),
      (∀ m v, ps.getD m none = some v →
        v = sk.pendAt p.other j (idx + m)) →
      idx + ps.length ≤ (sk.asmResList p.other j).length →
      got = sk.pendsBefore p.other j idx →
      peerAsmTraceA.go (asmResChan (p.other, j))
          (asmLevelChan (p.other, j)) (sk.asmOutChan (p.other, j))
          ps idx got
        <+: (List.range' idx ((sk.asmResList p.other j).length - idx)).flatMap
            (Sched.asmBlock sk (p.other, j)) := by
  intro ps
  induction ps with
  | nil =>
      intro idx got _ _ _
      show ([] : List Ev) <+: _
      exact List.nil_prefix
  | cons e rest ih =>
      intro idx got hvals hlen hgot
      cases e with
      | none =>
          show ([] : List Ev) <+: _
          exact List.nil_prefix
      | some pend =>
          have hpend : pend = sk.pendAt p.other j idx := by
            have := hvals 0 pend (by simp)
            simpa using this
          have hidx : idx < (sk.asmResList p.other j).length := by
            simp only [List.length_cons] at hlen
            omega
          have hrange : List.range' idx
              ((sk.asmResList p.other j).length - idx)
              = idx :: List.range' (idx + 1)
                  ((sk.asmResList p.other j).length - (idx + 1)) := by
            rw [show (sk.asmResList p.other j).length - idx
              = ((sk.asmResList p.other j).length - (idx + 1)) + 1
              from by omega]
            rfl
          have hstep : peerAsmTraceA.go (asmResChan (p.other, j))
              (asmLevelChan (p.other, j)) (sk.asmOutChan (p.other, j))
              (some pend :: rest) idx got
              = (asmResChan (p.other, j), false, idx)
                :: ((List.range pend).map fun t =>
                    (asmLevelChan (p.other, j), false, got + t))
                ++ (sk.asmOutChan (p.other, j), true, idx)
                :: peerAsmTraceA.go (asmResChan (p.other, j))
                    (asmLevelChan (p.other, j))
                    (sk.asmOutChan (p.other, j))
                    rest (idx + 1) (got + pend) := by
            simp only [peerAsmTraceA.go]
          have hblock : Sched.asmBlock sk (p.other, j) idx
              = (asmResChan (p.other, j), false, idx)
                :: ((List.range (sk.pendAt p.other j idx)).map fun t =>
                    (asmLevelChan (p.other, j), false,
                      sk.pendsBefore p.other j idx + t))
                ++ [(sk.asmOutChan (p.other, j), true, idx)] := rfl
          rw [hstep, hrange, List.flatMap_cons, hblock, hpend, hgot]
          have hrec := ih (idx + 1) (got + pend)
            (fun m v hv => by
              have := hvals (m + 1) v (by simpa using hv)
              rw [this]
              congr 1
              omega)
            (by simp only [List.length_cons] at hlen; omega)
            (by rw [hgot, hpend, Sched.pendsBefore_succ sk hidx])
          obtain ⟨t, ht⟩ := hrec
          refine ⟨t, ?_⟩
          rw [hgot, hpend] at ht
          show (asmResChan (p.other, j), false, idx)
            :: (((List.range (sk.pendAt p.other j idx)).map fun t =>
                (asmLevelChan (p.other, j), false,
                  sk.pendsBefore p.other j idx + t))
              ++ (sk.asmOutChan (p.other, j), true, idx)
                :: peerAsmTraceA.go (asmResChan (p.other, j))
                    (asmLevelChan (p.other, j))
                    (sk.asmOutChan (p.other, j)) rest (idx + 1)
                    (sk.pendsBefore p.other j idx
                      + sk.pendAt p.other j idx)) ++ t = _
          simp only [List.cons_append, List.append_assoc,
            List.singleton_append, List.nil_append]
          rw [ht]


/-- The announced assembler trace is a prefix of the true one. -/
theorem peerAsmTraceA_prefix (hwf : sk.wellFormed = true)
    (p : Party) (tr : List MObs) {j : Nat}
    (hj1 : 1 ≤ j) (hjr : j ≤ sk.rootH) :
    peerAsmTraceA (aviewOf sk p tr) j
      <+: Sched.asmEvents sk (p.other, j) := by
  obtain ⟨hlen, hvals⟩ := asmPendsA_spec hwf p tr hj1 hjr
  show peerAsmTraceA.go (asmResChan (p.other, j))
      (asmLevelChan (p.other, j)) (sk.asmOutChan (p.other, j))
      (asmPendsA (aviewOf sk p tr) j) 0 0 <+: _
  have hgo := goAsm_spec (p := p) (tr := tr) hwf
    (asmPendsA (aviewOf sk p tr) j) 0 0
    (fun m v hv => by rw [Nat.zero_add]; exact hvals m v hv)
    (by omega) rfl
  unfold Sched.asmEvents
  rw [List.range_eq_range']
  rw [Nat.sub_zero] at hgo
  exact hgo

-- ============================== the announced family, paired and prefixed

/-- Every announced trace is a prefix of a true `.impl` process trace:
the transcription lemmas assembled over the whole family — the deferred
containment input named by Mux/Causal.lean's module doc. -/
theorem announcedProcs_prefix (hwf : sk.wellFormed = true)
    (p : Party) (tr : List MObs) :
    ∀ T ∈ announcedProcs (aviewOf sk p tr),
      ∃ T' ∈ Sched.procsE sk, T <+: T' := by
  have hev : sk.rootH % 2 = 0 := (wf_rootH hwf).1
  have hge : 2 ≤ sk.rootH := (wf_rootH hwf).2
  intro T hT
  unfold announcedProcs at hT
  rcases List.mem_append.mp hT with hT | hfin
  rcases List.mem_append.mp hT with hT | hasm
  rcases List.mem_append.mp hT with hT | habs
  rcases List.mem_append.mp hT with hopen | hwalk
  · -- the peer opener
    have hTo : T = peerOpenTraceA (aviewOf sk p tr) := by
      simpa using hopen
    subst hTo
    cases p with
    | I =>
        refine ⟨Sched.ropenEvents sk, ?_, ?_⟩
        · unfold Sched.procsE
          refine List.mem_append.mpr (.inl ?_)
          refine List.mem_append.mpr (.inl ?_)
          refine List.mem_append.mpr (.inl ?_)
          refine List.mem_append.mpr (.inl ?_)
          simp
        · have := peerOpenTraceA_prefix hwf Party.I tr
          rwa [if_pos rfl] at this
    | R =>
        refine ⟨Sched.iopenEvents sk, ?_, ?_⟩
        · unfold Sched.procsE
          refine List.mem_append.mpr (.inl ?_)
          refine List.mem_append.mpr (.inl ?_)
          refine List.mem_append.mpr (.inl ?_)
          refine List.mem_append.mpr (.inl ?_)
          simp
        · have := peerOpenTraceA_prefix hwf Party.R tr
          rwa [if_neg (by simp)] at this
  · -- a peer walk stage
    obtain ⟨h, hh, hTe⟩ := List.mem_map.mp hwalk
    have hbound : h < sk.rootH ∧ (h % 2 == 1) = (p.other == Party.I) := by
      unfold peerStagesA at hh
      cases p with
      | I =>
          rw [if_pos (show ((aviewOf sk Party.I tr).party == Party.I)
            = true from rfl)] at hh
          obtain ⟨k, hk, hke⟩ := List.mem_map.mp hh
          rw [List.mem_range,
            show (aviewOf sk Party.I tr).rootH / 2 = sk.rootH / 2
              from rfl] at hk
          rw [show (aviewOf sk Party.I tr).rootH = sk.rootH
            from rfl] at hke
          constructor
          · omega
          · rw [← hke]
            have : (sk.rootH - 2 - 2 * k) % 2 = 0 := by omega
            rw [show ((sk.rootH - 2 - 2 * k) % 2 == 1) = false from by
              simp [this]]
            rfl
      | R =>
          rw [if_neg (show ¬ ((aviewOf sk Party.R tr).party == Party.I)
            = true from fun hc => nomatch hc)] at hh
          obtain ⟨k, hk, hke⟩ := List.mem_map.mp hh
          rw [List.mem_range,
            show (aviewOf sk Party.R tr).rootH / 2 = sk.rootH / 2
              from rfl] at hk
          rw [show (aviewOf sk Party.R tr).rootH = sk.rootH
            from rfl] at hke
          constructor
          · omega
          · rw [← hke]
            have : (sk.rootH - 1 - 2 * k) % 2 = 1 := by omega
            rw [show ((sk.rootH - 1 - 2 * k) % 2 == 1) = true from by
              simp [this]]
            rfl
    refine ⟨Sched.walkEventsE sk (p.other, h), ?_, ?_⟩
    · unfold Sched.procsE
      refine List.mem_append.mpr (.inl ?_)
      refine List.mem_append.mpr (.inl ?_)
      refine List.mem_append.mpr (.inl ?_)
      refine List.mem_append.mpr (.inr ?_)
      refine List.mem_map.mpr ⟨(p.other, h), ?_, rfl⟩
      refine List.mem_map.mpr ⟨sk.rootH - 1 - h, ?_, ?_⟩
      · rw [List.mem_range]
        omega
      · rw [show sk.rootH - 1 - (sk.rootH - 1 - h) = h from by omega]
        rw [Prod.mk.injEq]
        refine ⟨?_, rfl⟩
        cases hpo : p.other with
        | I =>
            rw [hpo] at hbound
            rw [show ((h % 2 == 1) = true) from by
              rw [hbound.2]; rfl]
            rfl
        | R =>
            rw [hpo] at hbound
            have : (h % 2 == 1) = false := by
              rw [hbound.2]
              rfl
            rw [this]
            rfl
    · rw [← hTe]
      exact peerWalkTraceA_prefix hwf p tr hbound.1
  · -- the absorber
    have hTa : T = peerAbsorbTraceA (aviewOf sk p tr) := by
      simpa using habs
    subst hTa
    refine ⟨Sched.absorbEvents sk, ?_, peerAbsorbTraceA_prefix hwf p tr⟩
    unfold Sched.procsE
    refine List.mem_append.mpr (.inl ?_)
    refine List.mem_append.mpr (.inl ?_)
    refine List.mem_append.mpr (.inr ?_)
    simp
  · -- a peer assembler
    obtain ⟨j, hj, hTe⟩ := List.mem_map.mp hasm
    have hbound : 1 ≤ j ∧ j ≤ sk.rootH
        ∧ (p.other, j) ∈ sk.asmKeys := by
      unfold peerAsmHeightsA at hj
      cases p with
      | I =>
          rw [if_pos (show ((aviewOf sk Party.I tr).party == Party.I)
            = true from rfl)] at hj
          obtain ⟨m, hm, hme⟩ := List.mem_map.mp hj
          rw [List.mem_range,
            show (aviewOf sk Party.I tr).rootH - 1 = sk.rootH - 1
              from rfl] at hm
          refine ⟨by omega, by omega, ?_⟩
          unfold Skel.asmKeys
          refine List.mem_append.mpr (.inr ?_)
          refine List.mem_map.mpr ⟨m, List.mem_range.mpr hm, ?_⟩
          rw [← hme]
          rfl
      | R =>
          rw [if_neg (show ¬ ((aviewOf sk Party.R tr).party == Party.I)
            = true from fun hc => nomatch hc)] at hj
          obtain ⟨m, hm, hme⟩ := List.mem_map.mp hj
          rw [List.mem_range,
            show (aviewOf sk Party.R tr).rootH = sk.rootH
              from rfl] at hm
          refine ⟨by omega, by omega, ?_⟩
          unfold Skel.asmKeys
          refine List.mem_append.mpr (.inl ?_)
          refine List.mem_map.mpr ⟨m, List.mem_range.mpr hm, ?_⟩
          rw [← hme]
          rfl
    refine ⟨Sched.asmEvents sk (p.other, j), ?_, ?_⟩
    · unfold Sched.procsE
      refine List.mem_append.mpr (.inl ?_)
      refine List.mem_append.mpr (.inr ?_)
      exact List.mem_map.mpr ⟨(p.other, j), hbound.2.2, rfl⟩
    · rw [← hTe]
      exact peerAsmTraceA_prefix hwf p tr hbound.1 hbound.2.1
  · -- the peer finale
    cases p with
    | I =>
        refine ⟨Sched.finEvents sk, ?_, ?_⟩
        · unfold Sched.procsE
          refine List.mem_append.mpr (.inr ?_)
          simp
        · have := peerFinTracesA_prefix hwf Party.I tr T hfin
          rwa [if_pos rfl] at this
    | R =>
        refine ⟨[((Chan.rootret : Chan), false, 0)], ?_, ?_⟩
        · unfold Sched.procsE
          refine List.mem_append.mpr (.inr ?_)
          simp
        · have := peerFinTracesA_prefix hwf Party.R tr T hfin
          rwa [if_neg (by simp)] at this


-- ======================================================= the causal keystone

/-- Non-evidence universe members are announced-trace events. -/
private theorem evUnivA_flatten {av : AView} {tr : List MObs} {e : Ev}
    (he : e ∈ evUnivA av tr) (hng : groundedA av tr e = false) :
    e ∈ (announcedProcs av).flatten := by
  unfold evUnivA at he
  rcases List.mem_append.mp he with he | hfl
  rcases List.mem_append.mp he with he | hrecv
  rcases List.mem_append.mp he with hown | hdel
  · exfalso
    obtain ⟨h, -, hin⟩ := List.mem_flatMap.mp hown
    obtain ⟨n, hn, rfl⟩ := List.mem_map.mp hin
    rw [List.mem_range] at hn
    have : groundedA av tr (Chan.wire av.party h, true, n) = true := by
      refine groundedA_of_push ?_
      rw [groundedPush]
      simp only [isWire, wireParty, wireHeight, Bool.true_and]
      rw [if_pos (by simp)]
      exact decide_eq_true hn
    rw [hng] at this
    cases this
  · exfalso
    obtain ⟨h, -, hin⟩ := List.mem_flatMap.mp hdel
    obtain ⟨n, hn, rfl⟩ := List.mem_map.mp hin
    rw [List.mem_range] at hn
    have : groundedA av tr (Chan.wire av.party.other h, true, n)
        = true := by
      refine groundedA_of_push ?_
      rw [groundedPush]
      simp only [isWire, wireParty, wireHeight, Bool.true_and]
      rw [if_neg (by cases av.party <;> simp [Party.other])]
      exact decide_eq_true hn
    rw [hng] at this
    cases this
  · exfalso
    obtain ⟨h, -, hin⟩ := List.mem_flatMap.mp hrecv
    obtain ⟨n, hn, rfl⟩ := List.mem_map.mp hin
    rw [List.mem_range] at hn
    have : groundedA av tr (Chan.wire av.party.other h, false, n)
        = true := by
      unfold groundedA
      rw [Bool.or_eq_true]
      refine Or.inr ?_
      show (av.party.other == av.party.other
        && decide (n < ownRecvCount av tr h)) = true
      rw [Bool.and_eq_true]
      exact ⟨by simp, decide_eq_true hn⟩
    rw [hng] at this
    cases this
  · exact hfl

/-- Members of a `takeWhile` satisfy its predicate. -/
private theorem pred_of_mem_takeWhile {α : Type _} {p' : α → Bool} :
    ∀ {l : List α} {x}, x ∈ l.takeWhile p' → p' x = true := by
  intro l
  induction l with
  | nil =>
      intro x h
      cases h
  | cons a t ih =>
      intro x h
      rw [List.takeWhile_cons] at h
      by_cases hpa : p' a = true
      · rw [if_pos hpa] at h
        rcases List.mem_cons.mp h with rfl | h'
        · exact hpa
        · exact ih h'
      · rw [if_neg hpa] at h
        cases h

/-- Inside a prefix that holds the pivot, the pivot's `takeWhile` past
agrees with the full list's. -/
private theorem takeWhile_prefix_eq {T₁ T : List Ev} {e : Ev}
    (hpre : T₁ <+: T) (he : e ∈ T₁) :
    T.takeWhile (fun x => !(x == e))
      = T₁.takeWhile (fun x => !(x == e)) := by
  obtain ⟨t, rfl⟩ := hpre
  rw [List.takeWhile_append, if_neg (fun hc => ?_)]
  have heq : T₁.takeWhile (fun x => !(x == e)) = T₁ :=
    (List.takeWhile_prefix _).eq_of_length hc
  have hmem : e ∈ T₁.takeWhile (fun x => !(x == e)) := by
    rw [heq]
    exact he
  have := pred_of_mem_takeWhile hmem
  simp at this

/-- The causal keystone (T2 re-run at the announced grain): at a stuck
muxed state, every event of a push-time CAUSAL closure has been
performed.

`tr` is the observation at push time; the count walls are the landed
keystone's (`hfifo`, `harr`) plus the receive ledger's
(`hrecv` — recorded receives never outrun the base consumer counts,
which grounds the C-own evidence arm) and the membership walls that
replace `evUniv` lookups for count-grounded events. -/
theorem keystoneA (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel)
    {C : Nat} {σI σR : Strategy} {s : MState}
    (hm : MuxInv sk s)
    (hstuck : mstuck sk .impl C σI σR s = true)
    (p : Party) (tr : List MObs)
    (hfifo : ∀ h, pushedCount tr h ≤ deliveredCount (s.hist p.other) h)
    (harr : ∀ h, deliveredCount tr h ≤ deliveredCount (s.hist p) h)
    (hrecv : ∀ h, ownRecvs sk.rootH p tr h
      ≤ recvdOf sk s.base (Chan.wire p.other h))
    (hpmem : ∀ h, pushedCount tr h ≠ 0 → Chan.wire p h ∈ allChans sk)
    (hdmem : ∀ h, deliveredCount tr h ≠ 0 →
      Chan.wire p.other h ∈ allChans sk) :
    ∀ e ∈ inevitableA (aviewOf sk p tr) tr, performed sk s.base e := by
  -- causal evidence is performed outright, against the count walls
  have hground_perf : ∀ x, groundedA (aviewOf sk p tr) tr x = true →
      performed sk s.base x := by
    intro x hg
    rcases groundedA_inv hg with hp | ⟨h, n, rfl, hlt⟩
    · obtain ⟨q, hh, n, rfl, hcase⟩ := groundedPush_inv hp
      rw [performed_snd_iff]
      rcases hcase with ⟨hq, hlt⟩ | ⟨hq, hlt⟩
      · have hq' : q = p := by
          rw [hq]
          rfl
        subst hq'
        have hmem : Chan.wire q hh ∈ allChans sk :=
          hpmem hh (by omega)
        have h1 := hfifo hh
        have h2 := hm.delivered_eq q hh hmem
        have h3 := hm.flow_wire q hh hmem
        omega
      · have hq' : q = p.other := by
          rw [hq]
          rfl
        subst hq'
        have hmem : Chan.wire p.other hh ∈ allChans sk :=
          hdmem hh (by omega)
        have h1 := harr hh
        have h2 := hm.delivered_eq p.other hh hmem
        rw [Party.other_other] at h2
        have h3 := hm.flow_wire p.other hh hmem
        omega
    · -- an own performed receive: the receive ledger's wall
      rw [performed_rcv_iff]
      rw [ownRecvCount_aviewOf] at hlt
      have := hrecv h
      show n < recvdOf sk s.base
        (Chan.wire ((aviewOf sk p tr).party).other h)
      have hpp : ((aviewOf sk p tr).party).other = p.other := rfl
      rw [hpp]
      omega
  intro e₀ he₀
  by_contra hnp₀
  -- the τ-least unperformed closure member
  have hne : (inevitableA (aviewOf sk p tr) tr).filter
      (fun x => !decide (performed sk s.base x)) ≠ [] := by
    intro hnil
    have hmem : e₀ ∈ (inevitableA (aviewOf sk p tr) tr).filter
        (fun x => !decide (performed sk s.base x)) :=
      List.mem_filter.mpr ⟨he₀, by simp [hnp₀]⟩
    rw [hnil] at hmem
    cases hmem
  obtain ⟨e, heU, hmin⟩ :=
    Sched.exists_min_image (fun x => Sched.evIdx x (Sched.scheduleE sk))
      hne
  obtain ⟨heI, hnpb⟩ := List.mem_filter.mp heU
  have hnp : ¬ performed sk s.base e := by simpa using hnpb
  have hperf_lt : ∀ x ∈ inevitableA (aviewOf sk p tr) tr,
      Sched.evIdx x (Sched.scheduleE sk)
        < Sched.evIdx e (Sched.scheduleE sk) →
      performed sk s.base x := by
    intro x hx hlt
    by_cases hperf : performed sk s.base x
    · exact hperf
    · have := hmin x (List.mem_filter.mpr ⟨hx, by simp [hperf]⟩)
      omega
  -- how did e enter the closure?
  rcases inevitableA_inv heI with hg | hstep
  · exact hnp (hground_perf e hg)
  -- I-step member: it is an announced-trace event; decode its true trace
  have hng : groundedA (aviewOf sk p tr) tr e = false := by
    cases hgb : groundedA (aviewOf sk p tr) tr e with
    | false => rfl
    | true => exact absurd (hground_perf e hgb) hnp
  obtain ⟨TA, hTA, heTA⟩ := List.mem_flatten.mp
    (evUnivA_flatten (inevitableA_subset_univ e heI) hng)
  obtain ⟨T, hT, hpreT⟩ := announcedProcs_prefix hwf p tr TA hTA
  have heT : e ∈ T := hpreT.sublist.mem heTA
  have hL : InvL sk .impl s.base := hm.invl
  have hioh := mstuck_ioh (sk := sk) hstuck
  have hroh := mstuck_roh (sk := sk) hL hstuck
  have hwkh := mstuck_wkh hwf hL hstuck rfl
  rcases trace_frontier hwf hL hioh hroh hwkh hT with hall | hfr
  · exact hnp (hall e heT)
  obtain ⟨f, a, pre, suf, hfa, hdec, hpre, hok⟩ := hfr
  have heT' := heT
  rw [hdec] at heT'
  rcases List.mem_append.1 heT' with hepre | hecons
  · exact hnp (hpre e hepre)
  rcases List.mem_cons.1 hecons with heqf | hesuf
  case inr =>
    -- e sits above the frontier: the frontier is in e's announced past
    have hfmem : f ∈ T.takeWhile (fun x => !(x == e)) :=
      frontier_mem_takeWhile hdec (trace_count_le_one hT e) hesuf
    rw [takeWhile_prefix_eq hpreT heTA] at hfmem
    have hfI : f ∈ inevitableA (aviewOf sk p tr) tr :=
      istepOkA_prefix hstep hTA heTA f hfmem
    have hfnp : ¬ performed sk s.base f :=
      Sched.pend_not_performedE sk hok
    have hτ : Sched.evIdx f (Sched.scheduleE sk)
        < Sched.evIdx e (Sched.scheduleE sk) := by
      refine tau_lt_of_trace_pair hwf hm0 hT ?_
      rw [hdec]
      refine List.Sublist.trans ?_ (List.sublist_append_right ..)
      exact List.cons_sublist_cons.2 (List.singleton_sublist.2 hesuf)
    have := hmin f (List.mem_filter.mpr ⟨hfI, by simp [hfnp]⟩)
    omega
  case inl =>
    -- e IS the frontier: open its guard and fire, against stuckness
    subst heqf
    have hkill : isWireFire s.base a = false →
        (Model.apply sk .impl a s.base).isSome = true → False := by
      intro hnf hsome
      obtain ⟨hncw, hnab⟩ := pends_not_close hfa
      have hbase : (applyBase sk .impl a s).isSome = true := by
        rw [applyBase_isSome_of_not_close hnf hncw hnab]
        exact hsome
      have hen := mcanStep_of_base (C := C) (σI := σI) (σR := σR)
        hok.act hbase
      have hno : mcanStep sk .impl C σI σR s = false := by
        rw [mstuck, Bool.and_eq_true, Bool.not_eq_true',
          Bool.not_eq_true'] at hstuck
        exact hstuck.2
      rw [hen] at hno
      cases hno
    have hmem_e : e ∈ Sched.scheduleE sk :=
      (Sched.trace_sublistE sk hwf hm0 hT).mem heT
    obtain ⟨c, b, n⟩ := e
    cases b with
    | false =>
        -- a receive: data is present
        have hseq : n = recvdOf sk s.base c := by simpa using hok.seq
        have hpred : ((c, true, n) : Ev)
            ∈ inevitableA (aviewOf sk p tr) tr :=
          istepOkA_e1 hstep rfl
        have hguard : 0 < s.base.chan c := by
          by_cases hw : isWire c = true
          · -- wire receive: the send is grounded evidence, and
            -- grounded sends are DELIVERED, not merely pushed
            obtain ⟨q, hh, rfl⟩ := isWire_eq hw
            have hpg : groundedPush p tr (Chan.wire q hh, true, n)
                = true := by
              rcases inevitableA_inv hpred with hg | hst
              · rcases groundedA_inv hg with hp | ⟨h', n', heq, -⟩
                · exact hp
                · exact absurd heq (by simp)
              · have h2 := istepOkA_not_push hst
                simp [isWire] at h2
            obtain ⟨q', hh', n', heq, hcase⟩ := groundedPush_inv hpg
            simp only [Prod.mk.injEq, Chan.wire.injEq, true_and] at heq
            obtain ⟨⟨rfl, rfl⟩, rfl⟩ := heq
            have hmemc : Chan.wire q hh ∈ allChans sk :=
              evUniv_wire_mem hwf (mem_evUniv.mpr ⟨T, hT, heT⟩)
            rcases hcase with ⟨rfl, hlt⟩ | ⟨rfl, hlt⟩
            · have h1 := hfifo hh
              have h2 := hm.delivered_eq q hh hmemc
              omega
            · have h1 := harr hh
              have h2 := hm.delivered_eq p.other hh hmemc
              rw [Party.other_other] at h2
              omega
          · -- internal receive: the send is τ-below, hence performed
            obtain ⟨-, hτlt⟩ := tau_e1 hwf hmem_e
            have hpp := hperf_lt _ hpred hτlt
            rw [performed_snd_iff] at hpp
            have hflow := hm.flow_int c hok.chan_mem
              (by simpa using hw)
            omega
        exact hkill (by
          cases hIF : isWireFire s.base a with
          | false => rfl
          | true =>
              obtain ⟨q₂, hh₂, -, hfb, -⟩ := pends_wireFire hfa hIF
              simp at hfb)
          (hok.fire (by simpa using hguard))
    | true =>
        -- a send: never a push (I-step), so an internal channel
        have hw : isWire c = false := by
          have := istepOkA_not_push hstep
          simpa using this
        have hseq : n = sentOf sk s.base c := by simpa using hok.seq
        have hflow := hm.flow_int c hok.chan_mem hw
        have hguard : s.base.chan c < sk.cap c := by
          by_cases hcap : n < sk.cap c
          · omega
          · have hpred : ((c, false, n - sk.cap c) : Ev)
                ∈ inevitableA (aviewOf sk p tr) tr := by
              have := istepOkA_e2 hstep rfl
                (by rw [capA_aviewOf]; exact hcap)
              rwa [capA_aviewOf] at this
            obtain ⟨-, hτlt⟩ := tau_e2 hwf hmem_e (by omega)
            have hpp := hperf_lt _ hpred hτlt
            rw [performed_rcv_iff] at hpp
            omega
        exact hkill (by
          cases hIF : isWireFire s.base a with
          | false => rfl
          | true =>
              obtain ⟨q₂, hh₂, hfc, -, -⟩ := pends_wireFire hfa hIF
              simp only at hfc
              rw [hfc] at hw
              simp [isWire] at hw)
          (hok.fire (by simpa using hguard))


-- ============================== the causal certificates and Step 1

/-- σ*-causal's push certificates: every recorded push was
proven-demanded against its own push-time ANNOUNCED closure (INV-A at
the causal grain). -/
def PushProvenA (sk : Skel) (s : MState) : Prop :=
  ∀ p i h, (s.hist p)[i]? = some (.pushed h) →
    pushedCount ((s.hist p).take i) h ≠ 0 →
    (Chan.wire p h, false, pushedCount ((s.hist p).take i) h - 1)
      ∈ inevitableA (aviewOf sk p ((s.hist p).take i))
          ((s.hist p).take i)

/-- What a σ*-causal verdict means: the named stream is history-held
and proven-demanded under the announced closure, for the history's own
party. -/
theorem sigmaStarCausal_some_inv {sk : Skel} {tr : List MObs} {h : Nat}
    (hs : sigmaStarCausal sk tr = some h) :
    ∃ p, partyOf tr = some p
      ∧ committedInHist sk.rootH tr h = true
      ∧ demandedA (aviewOf sk p tr) tr h = true := by
  rw [sigmaStarCausal] at hs
  cases hp : partyOf tr with
  | none => rw [hp] at hs; cases hs
  | some p =>
      rw [hp] at hs
      have hfind := List.find?_some hs
      rw [Bool.and_eq_true] at hfind
      exact ⟨p, rfl, hfind.1, hfind.2⟩

/-- Extending a history by one observation keeps every causal push
certificate, provided the new observation carries its own. -/
private theorem certsA_snoc {p : Party} {tr : List MObs} {o : MObs}
    (hcert : ∀ i h, tr[i]? = some (.pushed h) →
      pushedCount (tr.take i) h ≠ 0 →
      (Chan.wire p h, false, pushedCount (tr.take i) h - 1)
        ∈ inevitableA (aviewOf sk p (tr.take i)) (tr.take i))
    (hnew : ∀ h, o = .pushed h → pushedCount tr h ≠ 0 →
      (Chan.wire p h, false, pushedCount tr h - 1)
        ∈ inevitableA (aviewOf sk p tr) tr) :
    ∀ i h, (tr ++ [o])[i]? = some (.pushed h) →
      pushedCount ((tr ++ [o]).take i) h ≠ 0 →
      (Chan.wire p h, false, pushedCount ((tr ++ [o]).take i) h - 1)
        ∈ inevitableA (aviewOf sk p ((tr ++ [o]).take i))
            ((tr ++ [o]).take i) := by
  intro i h hget hcnt
  rcases Nat.lt_trichotomy i tr.length with hlt | heq | hgt
  · rw [List.getElem?_append_left hlt] at hget
    rw [List.take_append_of_le_length (Nat.le_of_lt hlt)] at hcnt ⊢
    exact hcert i h hget hcnt
  · subst heq
    rw [List.getElem?_concat_length] at hget
    injection hget with hget
    rw [List.take_append_of_le_length (Nat.le_refl _),
      List.take_length] at hcnt ⊢
    exact hnew h hget hcnt
  · rw [List.getElem?_eq_none (by
      rw [List.length_append, List.length_cons, List.length_nil]
      omega)] at hget
    cases hget

/-- Every σ*-causal-run step preserves the causal push certificates:
non-push observations are neutral, and a push observation carries the
demand proof σ*-causal itself computed. -/
theorem pushProvenA_step (hwf : sk.wellFormed = true) {C : Nat}
    {ma : MAction} {s s' : MState}
    (hstep : apply sk .impl C sigmaStarCausal sigmaStarCausal ma s
      = some s')
    (hm : SInv sk s) (hp : PushProvenA sk s) : PushProvenA sk s' := by
  have hgen : ∀ (q₀ : Party) (o : MObs),
      (∀ h, o = .pushed h → pushedCount (s.hist q₀) h ≠ 0 →
        (Chan.wire q₀ h, false, pushedCount (s.hist q₀) h - 1)
          ∈ inevitableA (aviewOf sk q₀ (s.hist q₀)) (s.hist q₀)) →
      s'.hist = recordObs s.hist q₀ o →
      PushProvenA sk s' := by
    intro q₀ o hnew hh
    have hrec : ∀ q, s'.hist q
        = if q == q₀ then s.hist q ++ [o] else s.hist q := by
      intro q
      rw [hh]
      rfl
    intro q i h
    rw [hrec]
    by_cases hq : q = q₀
    · subst hq
      rw [if_pos (by simp)]
      exact certsA_snoc (hp q) hnew i h
    · rw [if_neg (by simp [hq])]
      exact hp q i h
  cases ma with
  | base a =>
      obtain ⟨-, b, -, hs'⟩ := applyBase_inv hstep
      refine hgen (actionParty a) (.act a) ?_ (by rw [hs'])
      intro h hcon
      cases hcon
  | deliver p =>
      simp only [apply] at hstep
      split at hstep
      case h_2 => cases hstep
      case h_1 c rest hpp =>
          split at hstep
          case isFalse => cases hstep
          case isTrue h0 =>
            injection hstep with hs'
            refine hgen p.other (.delivered (wireHeight c)) ?_
              (by rw [← hs'])
            intro h hcon
            cases hcon
  | push p =>
      obtain ⟨-, h, hσ, hh⟩ := sinv_push hwf hstep hm
      have hσ' : sigmaStarCausal sk (s.hist p) = some h := by
        cases p <;> exact hσ
      refine hgen p (.pushed h) ?_ hh
      intro h' heq hcnt
      have heq' : h = h' := by
        injection heq
      subst heq'
      obtain ⟨p₀, hpo, -, hdem⟩ := sigmaStarCausal_some_inv hσ'
      have hpe : p₀ = p := partyOf_eq hm.hist hpo
      subst hpe
      rw [demandedA, Bool.or_eq_true] at hdem
      rcases hdem with hz | hmem
      · exfalso
        rw [Nat.beq_eq_true_eq] at hz
        exact hcnt hz
      · exact (List.contains_iff_mem ..).mp hmem

/-- σ*-causal's push certificates hold along every σ*-causal×σ*-causal
run. -/
theorem pushProvenA_reachable (hwf : sk.wellFormed = true) {C : Nat}
    {s : MState}
    (hr : MReachable sk .impl C sigmaStarCausal sigmaStarCausal s) :
    PushProvenA sk s := by
  induction hr with
  | init =>
      intro p i h hget
      rw [show (init sk).hist p = [] from rfl] at hget
      cases i <;> cases hget
  | step a hr' hstep ih =>
      exact pushProvenA_step hwf hstep (sinv_reachable hwf hr') ih

/-- Locate the `n`-th hit of a `filterMap` inside its source (private
copy of SigmaStarLive's device). -/
private theorem filterMapA_take_index {α β : Type _} (f : α → Option β) :
    ∀ (l : List α) (n : Nat) (b : β),
      (l.filterMap f)[n]? = some b →
      ∃ i a, l[i]? = some a ∧ f a = some b
        ∧ (l.take i).filterMap f = (l.filterMap f).take n := by
  intro l
  induction l with
  | nil =>
      intro n b hget
      simp at hget
  | cons x t ih =>
      intro n b hget
      cases hfx : f x with
      | none =>
          have hfm : (x :: t).filterMap f = t.filterMap f := by
            simp [hfx]
          rw [hfm] at hget
          obtain ⟨i, a, hia, hfa, htake⟩ := ih n b hget
          refine ⟨i + 1, a, by simpa using hia, hfa, ?_⟩
          rw [List.take_succ_cons, List.filterMap_cons, hfx, htake, hfm]
      | some c =>
          have hfm : (x :: t).filterMap f = c :: t.filterMap f := by
            simp [hfx]
          rw [hfm] at hget
          cases n with
          | zero =>
              simp only [List.getElem?_cons_zero, Option.some.injEq]
                at hget
              subst hget
              exact ⟨0, x, rfl, hfx, by simp⟩
          | succ m =>
              simp only [List.getElem?_cons_succ] at hget
              obtain ⟨i, a, hia, hfa, htake⟩ := ih m b hget
              refine ⟨i + 1, a, by simpa using hia, hfa, ?_⟩
              rw [List.take_succ_cons, List.filterMap_cons, hfx, htake,
                hfm, List.take_succ_cons]

/-- The push-tag extractor only hits `.pushed` observations. -/
private theorem pushedA_of_extract {a : MObs} {g : Nat}
    (h : (match a with
          | MObs.pushed h => some h
          | _ => none) = some g) : a = .pushed g := by
  cases a with
  | pushed h' =>
      injection h with h
      rw [h]
  | act a' => cases h
  | delivered h' => cases h

/-- Step 1 at the causal grain: at a σ*-causal-stuck state both pipes
are empty — the causal push certificate derived the head's
predecessor-consumption at push time, and the causal keystone performs
it. -/
theorem sigmaStarCausal_pipes_empty (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) {C : Nat} {s : MState}
    (hm : SInv sk s) (hrl : RecvLedger sk s) (hpp : PushProvenA sk s)
    (hstuck : mstuck sk .impl C sigmaStarCausal sigmaStarCausal s
      = true)
    (p : Party) : s.pipe p = [] := by
  cases hp : s.pipe p with
  | nil => rfl
  | cons c rest =>
      exfalso
      obtain ⟨g, rfl⟩ := hm.mux.pipe_mem_wire (p := p) (c := c)
        (by rw [hp]; exact List.mem_cons_self ..)
      have hz : s.base.chan (Chan.wire p g) ≠ 0 :=
        mstuck_deliver_blocked hstuck hp
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
      have hcap := hm.mux.slot (Chan.wire p g) hmem_ch
      have hcap1 : sk.cap (Chan.wire p g) = 1 := rfl
      have hkpos : pushedCount ((s.hist p).take i₀) g ≠ 0 := by
        omega
      have hcert := hpp p i₀ g hi₀ hkpos
      -- the causal keystone performs the certified receive
      have htkpre : (s.hist p).take i₀ <+: s.hist p :=
        List.take_prefix i₀ (s.hist p)
      have hperf := keystoneA hwf hm0 hm.mux hstuck p
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

end StreamingMirror.Mux
