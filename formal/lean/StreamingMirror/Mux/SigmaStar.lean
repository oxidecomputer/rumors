/-
σ*, demand-lockstep with forward derivation (T4's strategy, as
ratified by the adjudication): push a frame only when the receiver's
consumption of its per-stream predecessor is proven — derivable from
the machine's own observation history by the Certified ∪ Inevitable
closure — and among the proven-demanded held streams push the τ-least.

# The demand rule

Frame seqs are 0-based here (`Sched.Ev` numbering): the next frame on
stream `h` is the `pushedCount tr h`-th, and it is *proven-demanded*
iff it is the stream's first frame or `rcv(c, k−1)` is in the closure —
the ratified rule `k = 1 ∨ rcv(c, k−1) ∈ Certified ∪ Inevitable`, with
`certified ⊆ inevitable` by construction (`certified_subset_inevitable`)
collapsing the union to one membership test.

# What σ* consults

A machine identifies itself from its own history (`partyOf`: every
`.act` observation is its own — machines never observe each other's
actions), reconstructs its committed wire hands from commit-vs-flush
counts (`committedInHist`, the `bottomMostReady` device), and runs the
demand closure. The closure (`inevitable`) and the τ order
(`scheduleE`) read the full skeleton: this σ* is the OMNISCIENT-closure
formulation the stage-3 charter names — the causal (A_p-limited)
restriction that the stage-0 probe validated is definitionally tighter,
and `LocalStrategy`-tier locality of this form is NOT claimed here (see
SigmaStarLive.lean's companion notes for the precise gap).

# Tie-break

τ-least among the candidates, ties broken toward the list head. The
liveness proof never uses the tie-break (any member of the candidate
set refutes stuckness — any fixed order works); the
τ-least choice matches the demand-order intuition and keeps the
strategy deterministic.
-/
import StreamingMirror.Mux.Proofs.Chase.Decode
import StreamingMirror.Mux.Instances

namespace StreamingMirror.Mux

open Model

-- ===================================================== party inference

/-- The machine's own identity, read off its history: the party of the
first recorded base action. Every `.act` in machine `p`'s history is
`p`'s own (histories record only the acting machine's observations), so
any hit is correct; `none` means the machine has not acted yet — and a
machine that has never acted holds no committed wire hand, so idling is
the only sound answer anyway. -/
def partyOf (tr : List MObs) : Option Party :=
  tr.findSome? fun o =>
    match o with
    | .act a => some (actionParty a)
    | _ => none

-- ======================================================== demand rule

/-- Is the next frame on stream `h` proven-demanded at observation `tr`?
First frames are unconditionally demanded (every consumer's first
wire-channel operation is the receive itself); later
frames demand a closure proof that the predecessor was consumed. -/
def demanded (sk : Skel) (p : Party) (tr : List MObs) (h : Nat) : Bool :=
  pushedCount tr h == 0 ||
    (inevitable sk p tr).contains
      (Chan.wire p h, false, pushedCount tr h - 1)

/-- The evidence-only demand rule: the closure replaced by bare push
evidence (`certified`). The `evidence_only_starves` control pins that
this variant wedges where σ* completes — the Inevitable closure is
load-bearing, not decoration. -/
def demandedEv (sk : Skel) (p : Party) (tr : List MObs) (h : Nat) : Bool :=
  pushedCount tr h == 0 ||
    (certified sk p tr).contains
      (Chan.wire p h, false, pushedCount tr h - 1)

-- ==================================================== τ-least selection

/-- First argmin of `f` over a list: `none` iff the list is empty, ties
to the earlier element. -/
def argminBy (f : Nat → Nat) : List Nat → Option Nat
  | [] => none
  | x :: xs =>
      match argminBy f xs with
      | none => some x
      | some b => if f x ≤ f b then some x else some b

/-- An argmin is a member. -/
theorem argminBy_mem {f : Nat → Nat} :
    ∀ {l : List Nat} {x : Nat}, argminBy f l = some x → x ∈ l := by
  intro l
  induction l with
  | nil => intro x h; cases h
  | cons a as ih =>
      intro x h
      rw [argminBy] at h
      split at h
      next =>
        injection h with h
        exact h ▸ List.mem_cons_self ..
      next b heq =>
        split at h
        · injection h with h
          exact h ▸ List.mem_cons_self ..
        · injection h with h
          exact h ▸ List.mem_cons_of_mem a (ih heq)

/-- A nonempty list has an argmin. -/
theorem argminBy_isSome {f : Nat → Nat} {l : List Nat} (hne : l ≠ []) :
    (argminBy f l).isSome = true := by
  cases l with
  | nil => exact absurd rfl hne
  | cons a as =>
      rw [argminBy]
      split
      · rfl
      · split <;> rfl

-- ================================================================= σ*

/-- The candidate streams: held by the history's commit/flush ledger
AND proven-demanded. -/
def sigmaCands (sk : Skel) (p : Party) (tr : List MObs) : List Nat :=
  (wireHeights sk p).filter fun h =>
    committedInHist sk.rootH tr h && demanded sk p tr h

/-- σ*, demand-lockstep: the τ-least proven-demanded held stream, or
idle when no held stream's demand is proven (the right to idle is the
entire frontier of the impossibility). -/
def sigmaStar : Strategy := fun sk tr =>
  match partyOf tr with
  | none => none
  | some p =>
      argminBy
        (fun h => Sched.evIdx (Chan.wire p h, true, pushedCount tr h)
          (Sched.scheduleE sk))
        (sigmaCands sk p tr)

/-- The evidence-only control variant of σ*: same selection, `demanded`
weakened to `demandedEv`. Never shipped; minted because it marks C1's
boundary (an all-M scope is invisible in traffic, so evidence alone
cannot prove its consumption; only the Inevitable closure can). -/
def sigmaEvidence : Strategy := fun sk tr =>
  match partyOf tr with
  | none => none
  | some p =>
      argminBy
        (fun h => Sched.evIdx (Chan.wire p h, true, pushedCount tr h)
          (Sched.scheduleE sk))
        ((wireHeights sk p).filter fun h =>
          committedInHist sk.rootH tr h && demandedEv sk p tr h)

-- ===================================================== inversion lemmas

/-- What a σ* verdict means: the named stream is history-held and
proven-demanded for the history's own party. -/
theorem sigmaStar_some_inv {sk : Skel} {tr : List MObs} {h : Nat}
    (hs : sigmaStar sk tr = some h) :
    ∃ p, partyOf tr = some p ∧ h ∈ wireHeights sk p
      ∧ committedInHist sk.rootH tr h = true
      ∧ demanded sk p tr h = true := by
  rw [sigmaStar] at hs
  cases hp : partyOf tr with
  | none => rw [hp] at hs; cases hs
  | some p =>
      rw [hp] at hs
      have hmem := argminBy_mem hs
      rw [sigmaCands, List.mem_filter, Bool.and_eq_true] at hmem
      exact ⟨p, rfl, hmem.1, hmem.2.1, hmem.2.2⟩

/-- σ* never idles on a nonempty candidate set: whenever the machine has
identified itself and some held stream is proven-demanded, a push is
named. -/
theorem sigmaStar_isSome {sk : Skel} {tr : List MObs} {p : Party}
    {h : Nat} (hp : partyOf tr = some p)
    (hmem : h ∈ wireHeights sk p)
    (hcm : committedInHist sk.rootH tr h = true)
    (hdem : demanded sk p tr h = true) :
    (sigmaStar sk tr).isSome = true := by
  rw [sigmaStar, hp]
  refine argminBy_isSome ?_
  intro hnil
  have : h ∈ sigmaCands sk p tr :=
    List.mem_filter.mpr ⟨hmem, by rw [hcm, hdem]; rfl⟩
  rw [hnil] at this
  cases this

end StreamingMirror.Mux
