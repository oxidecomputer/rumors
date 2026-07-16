/-
Channel-wiring facts: which channel each walk count is observed on, and
the frame lemma saying a walk update at `pk` is invisible to every other
channel of `allChans`.

Why this is subtle: `sentOf`/`recvdOf` route each channel to a count at
a KEY derived from the channel's own indices (e.g. `wire p h` is consumed
by the walk `(p.other, h - 1)`), with special-cased branches for the root
channels and the absorber, and a Nat-subtraction collision at `wire I 0`
(mapped to key `(R, 0)` — a channel no process touches, and NOT in
`allChans`, which is why the frame lemma is membership-relativized).
The alignment lemmas need the `walkKeys` bounds to steer the branch
tests: initiator stages have `1 ≤ h ≤ rootH - 1`, responder stages
`h ≤ rootH - 2`.
-/
import StreamingMirror.Proofs.Lemmas

namespace StreamingMirror.Model

variable {sk : Skel} {s : State} {pk : Party × Nat}

/-- Structure of walk keys: party determines the index range. Initiator
stages consume `rootH-1, rootH-3, …, 1`; responder `rootH-2, …, 0`. -/
theorem walkKeys_cases (h : pk ∈ sk.walkKeys) :
    (pk.1 = Party.I ∧ 1 ≤ pk.2 ∧ pk.2 + 1 ≤ sk.rootH) ∨
    (pk.1 = Party.R ∧ pk.2 + 2 ≤ sk.rootH) := by
  simp only [Skel.walkKeys, List.mem_append, List.mem_map,
    List.mem_range] at h
  rcases h with ⟨k, hk, rfl⟩ | ⟨k, hk, rfl⟩
  · refine Or.inl ⟨rfl, ?_, ?_⟩
    · show 1 ≤ sk.rootH - 1 - 2 * k
      omega
    · show sk.rootH - 1 - 2 * k + 1 ≤ sk.rootH
      omega
  · refine Or.inr ⟨rfl, ?_⟩
    show sk.rootH - 2 - 2 * k + 2 ≤ sk.rootH
    omega

/-- `walkKeys_cases` at an explicit pair, so `rcases … ⟨rfl, …⟩` can
substitute the party. -/
private theorem walkKeys_cases_mk {p : Party} {k : Nat}
    (h : (p, k) ∈ sk.walkKeys) :
    (p = Party.I ∧ 1 ≤ k ∧ k + 1 ≤ sk.rootH) ∨
    (p = Party.R ∧ k + 2 ≤ sk.rootH) :=
  walkKeys_cases h

-- ================================= count/channel alignment (producer)

/-- The wire ledger of walk `pk` is the producer count of `wireOut pk`
(the walkKeys bound rules the channel out of the root special case). -/
theorem sentOf_wireOut (h : pk ∈ sk.walkKeys) :
    sentOf sk s (wireOut pk) = wkWireSent sk s pk := by
  have hne : pk.2 ≠ sk.rootH := by
    rcases walkKeys_cases h with ⟨_, _, hb⟩ | ⟨_, hb⟩ <;> omega
  simp [sentOf, wireOut, hne]

/-- `lowerOut pk`'s producer count. -/
theorem sentOf_lowerOut :
    sentOf sk s (lowerOut pk) = wkResSent sk s pk := rfl

/-- `upperOut pk`'s producer count. -/
theorem sentOf_upperOut :
    sentOf sk s (upperOut pk) = wkParentSent s pk := rfl

/-- `askedOut pk`'s producer count, for stages that can launch queries
(`1 ≤ pk.2`; the leaf responder stage `(R, 0)` has `askedOut` wired to
`leafRequests` but can never fire a query — `childIsD` is hard-false at
the leaf stage — so its exclusion is harmless). -/
theorem sentOf_askedOut (hwf : sk.wellFormed = true)
    (h : pk ∈ sk.walkKeys) (h1 : 1 ≤ pk.2) :
    sentOf sk s (askedOut pk) = wkQSentTot sk s pk := by
  -- `hwf` is load-bearing: for odd `rootH`, `pk = (R, 1)` would route
  -- `askedOut` to `leafRequests`, whose count lives at `(I, 1)`.
  -- Evenness makes responder keys even, so `pk.2 = 1` forces `(I, 1)`.
  simp only [Skel.walkKeys, List.mem_append, List.mem_map,
    List.mem_range] at h
  rcases h with ⟨j, hj, rfl⟩ | ⟨j, hj, rfl⟩
  · -- initiator: pk.2 = rootH - 1 - 2j
    have h1' : 1 ≤ sk.rootH - 1 - 2 * j := h1
    by_cases hlt : sk.rootH - 1 - 2 * j < 2
    · have hk1 : sk.rootH - 1 - 2 * j = 1 := by omega
      simp [sentOf, askedOut, hk1]
    · have ht1 : ¬ (sk.rootH - 1 - 2 * j - 2 = sk.rootH - 1) := by omega
      have hsub : sk.rootH - 1 - 2 * j - 2 + 2 = sk.rootH - 1 - 2 * j := by
        omega
      simp [sentOf, askedOut, hlt, ht1, hsub]
  · -- responder: pk.2 = rootH - 2 - 2j, even, so `1 ≤ pk.2` gives ≥ 2
    have hev : sk.rootH % 2 = 0 := (wf_rootH hwf).1
    have h1' : 1 ≤ sk.rootH - 2 - 2 * j := h1
    have hlt : ¬ (sk.rootH - 2 - 2 * j < 2) := by omega
    have ht2 : ¬ (sk.rootH - 2 - 2 * j - 2 = sk.rootH - 2) := by omega
    have hsub : sk.rootH - 2 - 2 * j - 2 + 2 = sk.rootH - 2 - 2 * j := by
      omega
    simp [sentOf, askedOut, hlt, ht2, hsub]

-- ================================= count/channel alignment (consumer)

/-- The prologue-wire count of walk `pk` is the consumer count of
`wireIn pk` (the responder top stage `(I, rootH-1)` reads the root wire
channel; the bound `pk.2 + 2 ≤ rootH` for responder keys rules out the
`wire I rootH` collision). -/
theorem recvdOf_wireIn (h : pk ∈ sk.walkKeys) :
    recvdOf sk s (wireIn pk) = wkWireRecvd sk s pk := by
  obtain ⟨p, k⟩ := pk
  rcases walkKeys_cases_mk h with ⟨rfl, hk1, hk2⟩ | ⟨rfl, hk2⟩
  · by_cases hr : k + 1 = sk.rootH
    · have hk : sk.rootH - 1 = k := by omega
      simp [recvdOf, wireIn, Party.other, hr, hk]
    · simp [recvdOf, wireIn, Party.other, hr]
  · have hr : k + 1 ≠ sk.rootH := by omega
    simp [recvdOf, wireIn, Party.other, hr]

/-- `askedIn pk`'s consumer count (unconditional: the `asked` routing is
direct). -/
theorem recvdOf_askedIn :
    recvdOf sk s (askedIn pk) = wkAskedRecvd sk s pk := rfl

-- ============================================== the setWalk flow frame

set_option linter.unusedVariables false in
/-- Producer-side frame: a walk update at `pk` is invisible to `sentOf`
away from `pk`'s four output channels.

`sentOf` reads walk state only at keys derived from the channel's own
indices, and every such key that collides with `pk` reconstructs one of
the four outputs — so membership is never needed on this side. -/
theorem sentOf_setWalk_frame (s : State) (pk : Party × Nat) (ws' : WalkSt)
    {c : Chan} (hc : c ∈ allChans sk)
    (h1 : c ≠ wireOut pk) (h2 : c ≠ lowerOut pk) (h3 : c ≠ askedOut pk)
    (h4 : c ≠ upperOut pk) :
    sentOf sk (setWalk s pk ws') c = sentOf sk s c := by
  cases c with
  | wire p h =>
      cases hb : h == sk.rootH with
      | true => simp [sentOf, hb, setWalk]
      | false =>
          have hq : (p, h) ≠ pk := by
            intro he; subst he; exact h1 rfl
          simp [sentOf, hb, wkWireSent, wkWireCount,
            setWalk_walk_ne s ws' hq]
  | asked p h =>
      cases hb1 : p == Party.I && h == sk.rootH - 1 with
      | true => simp [sentOf, hb1, setWalk]
      | false =>
          cases hb2 : p == Party.R && h == sk.rootH - 2 with
          | true => simp [sentOf, hb1, hb2, setWalk]
          | false =>
              have hlt : ¬ (h + 2 < 2) := by omega
              have hq : (p, h + 2) ≠ pk := by
                intro he; subst he
                exact h3 (by simp [askedOut, hlt])
              simp [sentOf, hb1, hb2, wkQSentTot, wkQSum,
                setWalk_walk_ne s ws' hq]
  | leafRequests =>
      have hq : (Party.I, 1) ≠ pk := by
        intro he; subst he; exact h3 rfl
      simp [sentOf, wkQSentTot, wkQSum, setWalk_walk_ne s ws' hq]
  | upper p h =>
      have hq : (p, h) ≠ pk := by
        intro he; subst he; exact h4 rfl
      simp [sentOf, wkParentSent, setWalk_walk_ne s ws' hq]
  | lower p h =>
      have hq : (p, h) ≠ pk := by
        intro he; subst he; exact h2 rfl
      simp [sentOf, wkResSent, wkResCount, setWalk_walk_ne s ws' hq]
  | level p j => rfl
  | rootret => rfl
  | rootrets => rfl
  | rootres => rfl

/-- The phantom channel: no `allChans` member is `wire I 0` once the
skeleton has real height (walk initiator wires sit at index ≥ 1, root
wires at `rootH ≥ 2`, and everything else is a different constructor). -/
private theorem wire_I_zero_not_mem (hge : 2 ≤ sk.rootH) :
    Chan.wire Party.I 0 ∉ allChans sk := by
  intro hc
  simp [allChans, wireOut, askedIn, upperOut, lowerOut] at hc
  rcases hc with hq | h0
  · rcases walkKeys_cases hq with ⟨_, hb, _⟩ | ⟨hR, _⟩
    · exact absurd (hb : 1 ≤ 0) (by omega)
    · exact Party.noConfusion (hR : Party.I = Party.R)
  · omega

/-- Consumer-side frame: a walk update at `pk` is invisible to `recvdOf`
away from `pk`'s two input channels, for `allChans` members (membership
excludes the phantom `wire I 0`, whose consumer key collides at `(R, 0)`
by Nat subtraction; `wellFormed` keeps the root wire channels above the
absorber's, which `rootH = 0` would alias). -/
theorem recvdOf_setWalk_frame (hwf : sk.wellFormed = true)
    (s : State) (pk : Party × Nat) (ws' : WalkSt)
    {c : Chan} (hc : c ∈ allChans sk)
    (h5 : c ≠ wireIn pk) (h6 : c ≠ askedIn pk) :
    recvdOf sk (setWalk s pk ws') c = recvdOf sk s c := by
  have hge : 2 ≤ sk.rootH := (wf_rootH hwf).2
  cases c with
  | wire p h =>
      by_cases hr : h = sk.rootH
      · subst hr
        cases p with
        | I => simp [recvdOf, setWalk]
        | R =>
            have hq : (Party.I, sk.rootH - 1) ≠ pk := by
              intro he
              apply h5
              rw [← he]
              show Chan.wire Party.R sk.rootH
                  = Chan.wire Party.R (sk.rootH - 1 + 1)
              have hn : sk.rootH - 1 + 1 = sk.rootH := by omega
              rw [hn]
            simp [recvdOf, wkWireRecvd, setWalk_walk_ne s ws' hq]
      · by_cases hh : h = 0
        · subst hh
          cases p with
          | I => exact absurd hc (wire_I_zero_not_mem hge)
          | R => simp [recvdOf, hr, absorbWireRecvd, setWalk]
        · have hq : (p.other, h - 1) ≠ pk := by
            intro he
            apply h5
            rw [← he]
            show Chan.wire p h = Chan.wire p.other.other (h - 1 + 1)
            have hn : h - 1 + 1 = h := by omega
            cases p <;> simp [Party.other, hn]
          simp [recvdOf, hr, hh, wkWireRecvd, setWalk_walk_ne s ws' hq]
  | asked p h =>
      have hq : (p, h) ≠ pk := by
        intro he; subst he; exact h6 rfl
      simp [recvdOf, wkAskedRecvd, setWalk_walk_ne s ws' hq]
  | leafRequests => rfl
  | upper p h => rfl
  | lower p h => rfl
  | level p j => rfl
  | rootret => rfl
  | rootrets => rfl
  | rootres => rfl

/-- A walk update at `pk` is invisible to every `allChans` channel other
than `pk`'s own six: the two it consumes and the four it produces.
(Membership matters: the phantom channel `wire I 0` reads key `(R, 0)`
by Nat-subtraction collision but is not in `allChans`.) -/
theorem flow_setWalk_frame (hwf : sk.wellFormed = true)
    (s : State) (pk : Party × Nat) (ws' : WalkSt)
    {c : Chan} (hc : c ∈ allChans sk)
    (h1 : c ≠ wireOut pk) (h2 : c ≠ lowerOut pk) (h3 : c ≠ askedOut pk)
    (h4 : c ≠ upperOut pk) (h5 : c ≠ wireIn pk) (h6 : c ≠ askedIn pk) :
    sentOf sk (setWalk s pk ws') c = sentOf sk s c ∧
    recvdOf sk (setWalk s pk ws') c = recvdOf sk s c :=
  ⟨sentOf_setWalk_frame s pk ws' hc h1 h2 h3 h4,
   recvdOf_setWalk_frame hwf s pk ws' hc h5 h6⟩

end StreamingMirror.Model
