/-
Weave precedence (PROGRESS.md §7 3b, edge-respect): the manual-manual
half of the guard discharges — at every position of the weave's manual
event order, the emission's predecessor was emitted strictly earlier.

# The predecessor relation

On the channels whose sends AND receives are both manual (the walk
wires above the leaf stage, every `asked` channel), each event has at
most one predecessor its guard consults: a receive waits for its
own-seq send (E1), a cap-1 send waits for the previous receive (E2).
`manDep` names it; everything else — pump-consumed channels (`upper`,
`lower`, the leaf wire, `leafRequests`), seq-0 sends — is `none` here
and either trivial or the pump-progress layer's business.

# The property and its proof

`DepOK done l`: at every position `i` of `l`, the predecessor of
`l[i]` lies in `done ++ l.take i`. It is established ONCE for the
initial ghost future by the master induction `dep_scope` (this file's
core, mirroring `align_scope`'s architecture): a subtree op's events
are dep-closed given the subtree's entry context — the in-flight wire
and query for the scope itself, and, for each deeper stage, the
prologue receives of the last scope emitted before this subtree
(`descIdx` names the boundary). Kids advance the context with their
own prologues (`prologue_mem`, read off `align_scope`'s clause 1) and
receive their feed's seg form through the query-base identity
`queries_base`: the parent's chunk-query seqs ARE the kid-stage scope
indices — the cap-1 asked channel's alternation, in prefix-sum form.

The master induction (layer D) then consumes `DepOK` pointwise: each
consumed head keeps the property (`depOK_tail`), and the head's
predecessor is already in `out` (`depOK_head` + conservation).
-/
import StreamingMirror.Proofs.Sched.Weave.Align

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ==================================================== the predecessor

/-- The manual-manual predecessor of an event: what its guard consults
when both sides of its channel are manual.

Receives wait for their own-seq send; cap-1 sends wait for the
previous receive. Leaf wires (`hh = 0`), seq-0 sends, and every
channel with a pump-side consumer map to `none`. -/
def manDep : Ev → Option Ev
  | (Chan.wire p hh, false, n) => some (Chan.wire p hh, true, n)
  | (Chan.asked p hh, false, n) => some (Chan.asked p hh, true, n)
  | (Chan.wire p hh, true, n) =>
      if hh = 0 ∨ n = 0 then none
      else some (Chan.wire p hh, false, n - 1)
  | (Chan.asked p hh, true, n) =>
      if n = 0 then none else some (Chan.asked p hh, false, n - 1)
  | _ => none

theorem manDep_wire_rcv (p : Party) (hh n : Nat) :
    manDep (Chan.wire p hh, false, n) = some (Chan.wire p hh, true, n) :=
  rfl

theorem manDep_asked_rcv (p : Party) (hh n : Nat) :
    manDep (Chan.asked p hh, false, n)
      = some (Chan.asked p hh, true, n) := rfl

theorem manDep_wire_snd_none {p : Party} {hh n : Nat}
    (h : hh = 0 ∨ n = 0) :
    manDep (Chan.wire p hh, true, n) = none := by
  simp only [manDep]
  rw [if_pos h]

theorem manDep_wire_snd_pos {p : Party} {hh n : Nat} (hh0 : hh ≠ 0)
    (hn : n ≠ 0) :
    manDep (Chan.wire p hh, true, n)
      = some (Chan.wire p hh, false, n - 1) := by
  simp only [manDep]
  rw [if_neg (by omega)]

theorem manDep_asked_snd_none {p : Party} {hh : Nat} :
    manDep (Chan.asked p hh, true, 0) = none := by
  simp [manDep]

theorem manDep_asked_snd_pos {p : Party} {hh n : Nat} (hn : n ≠ 0) :
    manDep (Chan.asked p hh, true, n)
      = some (Chan.asked p hh, false, n - 1) := by
  simp only [manDep]
  rw [if_neg hn]

theorem manDep_lower_snd (p : Party) (hh n : Nat) :
    manDep (Chan.lower p hh, true, n) = none := rfl

theorem manDep_upper_snd (p : Party) (hh n : Nat) :
    manDep (Chan.upper p hh, true, n) = none := rfl

theorem manDep_leafReq_snd (n : Nat) :
    manDep (Chan.leafRequests, true, n) = none := rfl

theorem manDep_rootres_snd (n : Nat) :
    manDep (Chan.rootres, true, n) = none := rfl

-- ===================================================== the property

/-- Dep-closure of a future against a past: every position's
predecessor lies strictly before it. -/
def DepOK (done l : List Ev) : Prop :=
  ∀ i e d, l[i]? = some e → manDep e = some d → d ∈ done ++ l.take i

theorem depOK_nil (done : List Ev) : DepOK done [] := by
  intro i e d h
  simp at h

/-- Dep-closure only reads the past through membership. -/
theorem depOK_mono {done done' l : List Ev}
    (hsub : ∀ x ∈ done, x ∈ done') (h : DepOK done l) :
    DepOK done' l := by
  intro i e d hi hd
  rcases List.mem_append.1 (h i e d hi hd) with hin | hin
  · exact List.mem_append_left _ (hsub _ hin)
  · exact List.mem_append_right _ hin

/-- Dep-free lists are dep-closed against anything. -/
theorem depOK_of_none {done l : List Ev}
    (h : ∀ e ∈ l, manDep e = none) : DepOK done l := by
  intro i e d hi hd
  rw [h e (List.mem_of_getElem? hi)] at hd
  cases hd

/-- Glue dep-closures: the right part sees the left as past. -/
theorem depOK_append {done A B : List Ev}
    (hA : DepOK done A) (hB : DepOK (done ++ A) B) :
    DepOK done (A ++ B) := by
  intro i e d hi hd
  rcases Nat.lt_or_ge i A.length with hlt | hge
  · rw [List.getElem?_append_left hlt] at hi
    rw [List.take_append_of_le_length (Nat.le_of_lt hlt)]
    exact hA i e d hi hd
  · rw [List.getElem?_append_right hge] at hi
    have hmem := hB (i - A.length) e d hi hd
    rw [List.take_append, List.take_of_length_le hge, List.append_assoc]
      at *
    exact hmem

/-- Extend a dep-closure by one head whose predecessor is past. -/
theorem depOK_cons {done : List Ev} {e : Ev} {l : List Ev}
    (hhead : ∀ d, manDep e = some d → d ∈ done)
    (htail : DepOK (done ++ [e]) l) : DepOK done (e :: l) := by
  intro i e' d hi hd
  match i with
  | 0 =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hi
      subst hi
      simpa using hhead d hd
  | i + 1 =>
      simp only [List.getElem?_cons_succ] at hi
      have hmem := htail i e' d hi hd
      rw [List.append_assoc] at hmem
      simpa using hmem

theorem depOK_singleton {done : List Ev} {e : Ev}
    (hhead : ∀ d, manDep e = some d → d ∈ done) : DepOK done [e] :=
  depOK_cons hhead (depOK_nil _)

/-- Read off the head's predecessor obligation. -/
theorem depOK_head {done : List Ev} {e : Ev} {l : List Ev}
    (h : DepOK done (e :: l)) :
    ∀ d, manDep e = some d → d ∈ done := by
  intro d hd
  simpa using h 0 e d rfl hd

/-- Consuming the head keeps the closure, with the head as past. -/
theorem depOK_tail {done : List Ev} {e : Ev} {l : List Ev}
    (h : DepOK done (e :: l)) : DepOK (done ++ [e]) l := by
  intro i e' d hi hd
  have hmem := h (i + 1) e' d (by simpa using hi) hd
  rw [List.take_succ_cons] at hmem
  rw [List.append_assoc]
  simpa using hmem

-- ============================================= channel parity bridges

/-- A stage's wire input is the wire output one stage up. -/
theorem wireIn_eq {h : Nat} (h1 : 1 ≤ h) :
    wireIn (wpk (h - 1)) = wireOut (wpk h) := by
  unfold wireIn wireOut wpk
  rcases Nat.mod_two_eq_zero_or_one h with hp | hp
  · have hp' : (h - 1) % 2 = 1 := by omega
    have hh : h - 1 + 1 = h := by omega
    rw [hp, hp', hh]
    rfl
  · have hp' : (h - 1) % 2 = 0 := by omega
    have hh : h - 1 + 1 = h := by omega
    rw [hp, hp', hh]
    rfl

/-- A stage's asked input is the query output two stages up. -/
theorem askedOut_eq (h : Nat) :
    askedOut (wpk (h + 2)) = askedIn (wpk h) := by
  unfold askedIn askedOut wpk
  rw [if_neg (by omega)]
  have hp : (h + 2) % 2 = h % 2 := by omega
  rw [hp, show h + 2 - 2 = h from rfl]

/-- Positional read of a seg. -/
theorem seg_getElem? (c : Chan) (b : Bool) (lo n i : Nat) (hi : i < n) :
    (seg c b lo n)[i]? = some (c, b, lo + i) := by
  unfold seg
  rw [List.getElem?_map, List.getElem?_range hi]
  rfl

-- =========================================== the query-base identity
-- The parent's chunk-query seqs ARE the kid-stage scope indices: the
-- prefix-sum form of the asked channel's alternation with the wires.

/-- One scope's worth: descending one stage, the wire cursor advances
by exactly the queries owed so far. -/
private theorem wires_qSum (hwf : sk.wellFormed = true) {h : Nat}
    (h1 : 1 ≤ h) (hh : h < sk.rootH) {k : Nat}
    (hk : k < sk.stageLen h) :
    ∀ i, i ≤ sk.nChildren h (sk.stageScope h k) →
      sk.wiresBefore (h - 1) (sk.wiresBefore h k + i)
        = sk.wiresBefore (h - 1) (sk.wiresBefore h k)
          + qSum sk (wpk h) k i := by
  intro i
  induction i with
  | zero =>
      intro _
      show _ = _ + qSum sk (wpk h) k 0
      unfold qSum
      simp
  | succ i ih =>
      intro hi
      have hilt : i < sk.nChildren h (sk.stageScope h k) := by omega
      have hcursor : sk.wiresBefore h k + i < sk.stageLen (h - 1) := by
        have htot := wiresBefore_total sk hwf h1 hh
        have hmono := wiresBefore_mono sk h
          (show k + 1 ≤ sk.stageLen h from hk)
        have hsucc := wiresBefore_succ sk hk
        omega
      rw [show sk.wiresBefore h k + (i + 1)
            = (sk.wiresBefore h k + i) + 1 from by omega,
        wiresBefore_succ sk hcursor, ih (by omega), qSum_succ,
        show (wpk h).2 = h from rfl]
      have hkid : sk.nChildren (h - 1)
          (sk.stageScope (h - 1) (sk.wiresBefore h k + i))
          = sk.qCount h (sk.stageScope h k) i := by
        cases hD : sk.childIsD h (sk.stageScope h k) i with
        | true =>
            exact (qCount_eq_kid_nChildren sk hwf h1 hh hk hilt).symm
        | false =>
            rw [nChildren_kid_notD sk hwf h1 hh hk hilt hD]
            simp [Skel.qCount, hD]
      omega

/-- Whole-prefix form: the queries owed by a stage prefix are the
wire cursor of the descended prefix. -/
private theorem qsBefore_wires (hwf : sk.wellFormed = true) {h : Nat}
    (h1 : 1 ≤ h) (hh : h < sk.rootH) :
    ∀ k, k ≤ sk.stageLen h →
      sk.qsBefore h k = sk.wiresBefore (h - 1) (sk.wiresBefore h k) := by
  intro k
  induction k with
  | zero => intro _; rfl
  | succ k ih =>
      intro hk1
      have hk : k < sk.stageLen h := by omega
      have hq := qSum_total sk (wpk h) k
      rw [show (wpk h).2 = h from rfl] at hq
      rw [qsBefore_succ sk hk, ih (by omega), wiresBefore_succ sk hk,
        ← hq,
        wires_qSum sk hwf h1 hh hk _ (Nat.le_refl _)]

/-- THE QUERY-BASE IDENTITY: a chunk's query seqs start at its kid's
stage index — the seq every chunk query carries is the scope index of
the kid-stage scope that will consume it. -/
theorem queries_base (hwf : sk.wellFormed = true) {h : Nat}
    (h1 : 1 ≤ h) (hh : h < sk.rootH) {k : Nat} (hk : k < sk.stageLen h)
    {i : Nat} (hi : i ≤ sk.nChildren h (sk.stageScope h k)) :
    sk.qsBefore h k + qSum sk (wpk h) k i
      = sk.wiresBefore (h - 1) (sk.wiresBefore h k + i) := by
  rw [qsBefore_wires sk hwf h1 hh k (Nat.le_of_lt hk),
    wires_qSum sk hwf h1 hh hk i hi]

/-- A chunk's queries, as the seg its kid subtree is fed: based at the
kid's stage index, one per grandchild. -/
theorem chunkQ_eq_seg (hwf : sk.wellFormed = true) {h : Nat}
    (h1 : 1 ≤ h) (hh : h < sk.rootH) {k : Nat} (hk : k < sk.stageLen h)
    {i : Nat} (hi : i < sk.nChildren h (sk.stageScope h k)) :
    chunkQ sk h k i
      = seg (askedOut (wpk h)) true
          (sk.wiresBefore (h - 1) (sk.wiresBefore h k + i))
          (sk.qCount h (sk.stageScope h k) i) := by
  have hbase := queries_base sk hwf h1 hh hk (Nat.le_of_lt hi)
  unfold chunkQ seg
  simp only [hbase]

-- ============================================== prologue membership
-- Read off `align_scope`: a subtree op contains the prologue receives
-- of every scope in each of its stage windows.

/-- A subtree contains the two prologue receives of every scope inside
any of its `descIdx` windows. -/
theorem prologue_mem (hwf : sk.wellFormed = true) {h k : Nat}
    {F : List Ev} {mF : Nat} (hh : h < sk.rootH)
    (hk : k < sk.stageLen h)
    (hF : F.length = sk.nChildren h (sk.stageScope h k))
    (hFo : ∀ e ∈ F, evOwner sk e = mF) (hmF : mF < walkIdx sk h) :
    ∀ h', h' ≤ h → ∀ j, descIdx sk h' (h - h') k ≤ j →
      j < descIdx sk h' (h - h') (k + 1) →
      ((wireIn (wpk h'), false, j) : Ev) ∈ opEvents sk (.scope h k F)
        ∧ ((askedIn (wpk h'), false, j) : Ev)
            ∈ opEvents sk (.scope h k F) := by
  intro h' hh' j hjl hjr
  obtain ⟨-, -, hclause⟩ :=
    align_scope sk hwf h k F mF hh hk hF hFo hmF
  have hfil := hclause h' hh'
  have hjmem : j ∈ List.range' (descIdx sk h' (h - h') k)
      (descIdx sk h' (h - h') (k + 1) - descIdx sk h' (h - h') k) := by
    rw [List.mem_range'_1]
    omega
  have hin : ∀ e ∈ scopeBlock sk (wpk h') j,
      e ∈ walkSeg sk h' (descIdx sk h' (h - h') k)
        (descIdx sk h' (h - h') (k + 1)) := by
    intro e he
    unfold walkSeg
    exact List.mem_flatMap.2 ⟨j, hjmem, he⟩
  constructor
  · have hw := hin (wireIn (wpk h'), false, j)
      (List.mem_cons_self ..)
    rw [← hfil] at hw
    exact (List.mem_filter.1 hw).1
  · have ha := hin (askedIn (wpk h'), false, j)
      (List.mem_cons_of_mem _ (List.mem_cons_self ..))
    rw [← hfil] at ha
    exact (List.mem_filter.1 ha).1

-- ======================================================== scope feeds

/-- The feed a stage scope receives: one query per kid, based at the
scope's kid-stage cursor. -/
def scopeFeed (h k : Nat) : List Ev :=
  seg (askedOut (wpk (h + 1))) true (sk.wiresBefore h k)
    (sk.nChildren h (sk.stageScope h k))

theorem scopeFeed_length (h k : Nat) :
    (scopeFeed sk h k).length
      = sk.nChildren h (sk.stageScope h k) := by
  simp [scopeFeed, seg]

theorem scopeFeed_nil {h k : Nat}
    (h0 : sk.nChildren h (sk.stageScope h k) = 0) :
    scopeFeed sk h k = [] := by
  unfold scopeFeed
  rw [h0, seg_zero]

theorem scopeFeed_getElem? {h k i : Nat}
    (hi : i < sk.nChildren h (sk.stageScope h k)) :
    (scopeFeed sk h k)[i]?
      = some (askedOut (wpk (h + 1)), true, sk.wiresBefore h k + i) := by
  unfold scopeFeed
  exact seg_getElem? _ _ _ _ _ hi

/-- A chunk's queries are its kid scope's feed: the query-base
identity plus the grandchild count. -/
theorem chunkQ_eq_feed (hwf : sk.wellFormed = true) {hp : Nat}
    (hh : hp + 1 < sk.rootH) {k : Nat} (hk : k < sk.stageLen (hp + 1))
    {i : Nat} (hi : i < sk.nChildren (hp + 1) (sk.stageScope (hp + 1) k)) :
    chunkQ sk (hp + 1) k i
      = scopeFeed sk hp (sk.wiresBefore (hp + 1) k + i) := by
  rw [chunkQ_eq_seg sk hwf (by omega) hh hk hi]
  unfold scopeFeed
  rw [qCount_eq_kid_nChildren sk hwf (by omega) hh hk hi]
  rfl

-- ================================================ the master induction

/-- The kid cursor stays inside the kid stage. -/
private theorem cursor_lt (hwf : sk.wellFormed = true) {hp : Nat}
    (hh : hp + 1 < sk.rootH) {k : Nat} (hk : k < sk.stageLen (hp + 1))
    {i : Nat}
    (hi : i < sk.nChildren (hp + 1) (sk.stageScope (hp + 1) k)) :
    sk.wiresBefore (hp + 1) k + i < sk.stageLen hp := by
  have htot : sk.wiresBefore (hp + 1) (sk.stageLen (hp + 1))
      = sk.stageLen hp :=
    wiresBefore_total sk hwf (show 1 ≤ hp + 1 by omega) hh
  have hmono := wiresBefore_mono sk (hp + 1)
    (show k + 1 ≤ sk.stageLen (hp + 1) from hk)
  have hsucc := wiresBefore_succ sk hk
  omega

/-- The kids-fold of the master induction: processing kid slots
`i, i+1, …` of one scope preserves dep-closure, given the rolling
context — the last kid-stage prologue emitted so far and, per deeper
stage, the last prologue before the current cursor's window. -/
private theorem dep_kids (hwf : sk.wellFormed = true) {hp : Nat}
    (hh : hp + 1 < sk.rootH) {k : Nat} (hk : k < sk.stageLen (hp + 1))
    (IH : ∀ (k' mF' : Nat) (done' : List Ev), k' < sk.stageLen hp →
      (∀ e ∈ scopeFeed sk hp k', evOwner sk e = mF') →
      mF' < walkIdx sk hp →
      ((wireIn (wpk hp), true, k') : Ev) ∈ done' →
      ((askedIn (wpk hp), true, k') : Ev) ∈ done' →
      (∀ h'', h'' < hp → 0 < descIdx sk h'' (hp - h'') k' →
        ((wireIn (wpk h''), false,
            descIdx sk h'' (hp - h'') k' - 1) : Ev) ∈ done'
        ∧ ((askedIn (wpk h''), false,
            descIdx sk h'' (hp - h'') k' - 1) : Ev) ∈ done') →
      DepOK done' (opEvents sk (.scope hp k' (scopeFeed sk hp k')))) :
    ∀ (m i : Nat),
      i + m = sk.nChildren (hp + 1) (sk.stageScope (hp + 1) k) →
    ∀ done' : List Ev,
      (0 < sk.wiresBefore (hp + 1) k + i →
        ((wireIn (wpk hp), false,
            sk.wiresBefore (hp + 1) k + i - 1) : Ev) ∈ done'
        ∧ ((askedIn (wpk hp), false,
            sk.wiresBefore (hp + 1) k + i - 1) : Ev) ∈ done') →
      (∀ h'', h'' < hp →
        0 < descIdx sk h'' (hp - h'') (sk.wiresBefore (hp + 1) k + i) →
        ((wireIn (wpk h''), false, descIdx sk h'' (hp - h'')
            (sk.wiresBefore (hp + 1) k + i) - 1) : Ev) ∈ done'
        ∧ ((askedIn (wpk h''), false, descIdx sk h'' (hp - h'')
            (sk.wiresBefore (hp + 1) k + i) - 1) : Ev) ∈ done') →
      DepOK done' ((List.range' i m).flatMap fun i' =>
        opEvents sk (.kid (hp + 1) k (sk.stageScope (hp + 1) k)
          (lastDOf sk (hp + 1) k) (sk.wiresBefore (hp + 1) k) i'
          (scopeFeed sk (hp + 1) k))) := by
  intro m
  induction m with
  | zero =>
      intro i _ done' _ _
      show DepOK done' ([].flatMap _)
      exact depOK_nil done'
  | succ m ihm =>
      intro i hin done' hroll hdeep
      have hi : i < sk.nChildren (hp + 1) (sk.stageScope (hp + 1) k) := by
        omega
      have hcur := cursor_lt sk hwf hh hk hi
      rw [List.range'_succ, List.flatMap_cons]
      -- the kid's event list, feed read resolved
      have hq := scopeFeed_getElem? sk (h := hp + 1) (k := k) hi
      have hkid := opEvents_kid_eq sk (hp + 1) k (lastDOf sk (hp + 1) k)
        (sk.wiresBefore (hp + 1) k) i (scopeFeed sk (hp + 1) k)
      rw [hq] at hkid
      have hbridge : wireIn (wpk hp) = wireOut (wpk (hp + 1)) := by
        exact wireIn_eq (h := hp + 1) (by omega)
      -- the wire's predecessor: the previous kid-stage prologue
      have hwire_head : ∀ done'' : List Ev,
          (∀ x ∈ done', x ∈ done'') →
          ∀ d, manDep ((wireOut (wpk (hp + 1)), true,
            sk.wiresBefore (hp + 1) k + i) : Ev) = some d →
            d ∈ done'' := by
        intro done'' hsub d hd
        rw [show (wireOut (wpk (hp + 1)) : Chan)
            = Chan.wire (wpk (hp + 1)).1 (hp + 1) from rfl] at hd
        by_cases h0 : sk.wiresBefore (hp + 1) k + i = 0
        · rw [manDep_wire_snd_none (Or.inr h0)] at hd
          cases hd
        · rw [manDep_wire_snd_pos (by omega) h0] at hd
          cases hd
          have hmem := (hroll (by omega)).1
          rw [hbridge] at hmem
          exact hsub _ hmem
      -- the query's predecessor: the previous kid-stage prologue
      have hq_head : ∀ done'' : List Ev,
          (∀ x ∈ done', x ∈ done'') →
          ∀ d, manDep ((askedOut (wpk (hp + 2)), true,
            sk.wiresBefore (hp + 1) k + i) : Ev) = some d →
            d ∈ done'' := by
        intro done'' hsub d hd
        rw [show (askedOut (wpk (hp + 2)) : Chan)
            = askedIn (wpk hp) from askedOut_eq hp,
          show (askedIn (wpk hp) : Chan)
            = Chan.asked (wpk hp).1 hp from rfl] at hd
        by_cases h0 : sk.wiresBefore (hp + 1) k + i = 0
        · rw [h0, manDep_asked_snd_none] at hd
          cases hd
        · rw [manDep_asked_snd_pos h0] at hd
          cases hd
          have hmem := (hroll (by omega)).2
          exact hsub _ hmem
      -- the kid subtree: instantiate the outer induction hypothesis
      have hsub_dep : ∀ done'' : List Ev,
          (∀ x ∈ done', x ∈ done'') →
          ((wireOut (wpk (hp + 1)), true,
            sk.wiresBefore (hp + 1) k + i) : Ev) ∈ done'' →
          ((askedOut (wpk (hp + 2)), true,
            sk.wiresBefore (hp + 1) k + i) : Ev) ∈ done'' →
          DepOK done'' (opEvents sk (.scope hp
            (sk.wiresBefore (hp + 1) k + i)
            (scopeFeed sk hp (sk.wiresBefore (hp + 1) k + i)))) := by
        intro done'' hsub hwmem hqmem
        refine IH (sk.wiresBefore (hp + 1) k + i) (walkIdx sk (hp + 1))
          done'' hcur ?_ ?_ ?_ ?_ ?_
        · -- feed owners: the chunk queries belong to this stage's walk
          by_cases hD : sk.childIsD (hp + 1) (sk.stageScope (hp + 1) k) i
          · rw [← chunkQ_eq_feed sk hwf hh hk hi]
            exact chunkQ_owner sk (by omega) hh k i
          · intro e he
            have h0 : sk.nChildren hp
                (sk.stageScope hp (sk.wiresBefore (hp + 1) k + i)) = 0 :=
              nChildren_kid_notD sk hwf (by omega) hh hk hi
                (by simpa using hD)
            rw [scopeFeed_nil sk h0] at he
            cases he
        · exact walkIdx_lt sk (by omega) hh
        · rw [hbridge]
          exact hwmem
        · rw [show (askedIn (wpk hp) : Chan)
            = askedOut (wpk (hp + 2)) from (askedOut_eq hp).symm]
          exact hqmem
        · intro h'' hlt hpos
          obtain ⟨hw, ha⟩ := hdeep h'' hlt hpos
          exact ⟨hsub _ hw, hsub _ ha⟩
      -- rolling context after this kid: its scope's own prologues
      have hself : ∀ (F : List Ev),
          ((wireIn (wpk hp), false,
              sk.wiresBefore (hp + 1) k + i) : Ev)
            ∈ opEvents sk (.scope hp (sk.wiresBefore (hp + 1) k + i) F)
          ∧ ((askedIn (wpk hp), false,
              sk.wiresBefore (hp + 1) k + i) : Ev)
            ∈ opEvents sk (.scope hp (sk.wiresBefore (hp + 1) k + i) F) := by
        intro F
        rw [opEvents_scope_eq sk (Nat.le_of_lt hcur) F]
        exact ⟨List.mem_cons_self ..,
          List.mem_cons_of_mem _ (List.mem_cons_self ..)⟩
      -- deeper rolling context after this kid, from the subtree windows
      have hdeep_next : ∀ (F : List Ev) {mF' : Nat},
          F.length = sk.nChildren hp
            (sk.stageScope hp (sk.wiresBefore (hp + 1) k + i)) →
          (∀ e ∈ F, evOwner sk e = mF') → mF' < walkIdx sk hp →
          ∀ h'', h'' < hp →
          0 < descIdx sk h'' (hp - h'')
            (sk.wiresBefore (hp + 1) k + i + 1) →
          (((wireIn (wpk h''), false, descIdx sk h'' (hp - h'')
              (sk.wiresBefore (hp + 1) k + i + 1) - 1) : Ev)
            ∈ done'
            ∨ ((wireIn (wpk h''), false, descIdx sk h'' (hp - h'')
              (sk.wiresBefore (hp + 1) k + i + 1) - 1) : Ev)
            ∈ opEvents sk (.scope hp (sk.wiresBefore (hp + 1) k + i) F))
          ∧ (((askedIn (wpk h''), false, descIdx sk h'' (hp - h'')
              (sk.wiresBefore (hp + 1) k + i + 1) - 1) : Ev)
            ∈ done'
            ∨ ((askedIn (wpk h''), false, descIdx sk h'' (hp - h'')
              (sk.wiresBefore (hp + 1) k + i + 1) - 1) : Ev)
            ∈ opEvents sk (.scope hp (sk.wiresBefore (hp + 1) k + i) F)) := by
        intro F mF' hF hFo hmF' h'' hlt hpos
        have hmono := descIdx_mono sk h'' (hp - h'')
          (show sk.wiresBefore (hp + 1) k + i
            ≤ sk.wiresBefore (hp + 1) k + i + 1 by omega)
        by_cases heq : descIdx sk h'' (hp - h'')
            (sk.wiresBefore (hp + 1) k + i + 1)
            = descIdx sk h'' (hp - h'') (sk.wiresBefore (hp + 1) k + i)
        · rw [heq]
          obtain ⟨hw, ha⟩ := hdeep h'' hlt (by omega)
          exact ⟨Or.inl hw, Or.inl ha⟩
        · have hwin := prologue_mem sk hwf
            (show hp < sk.rootH by omega) hcur hF hFo hmF'
            h'' (Nat.le_of_lt hlt)
            (descIdx sk h'' (hp - h'')
              (sk.wiresBefore (hp + 1) k + i + 1) - 1)
            (by omega) (by omega)
          exact ⟨Or.inr hwin.1, Or.inr hwin.2⟩
      -- now assemble the kid's own dep-closure and recurse
      rw [hkid]
      by_cases hD : sk.childIsD (hp + 1) (sk.stageScope (hp + 1) k) i
      · -- disputed kid: wire, res, splice summary, feed query, subtree
        rw [if_pos hD]
        simp only [Option.toList, List.cons_append, List.append_assoc]
        refine depOK_cons (hwire_head done' fun x hx => hx) ?_
        refine depOK_cons ?_ ?_
        · intro d hd
          rw [show (lowerOut (wpk (hp + 1)) : Chan)
              = Chan.lower (wpk (hp + 1)).1 (wpk (hp + 1)).2 from rfl,
            manDep_lower_snd] at hd
          cases hd
        refine depOK_append (depOK_of_none ?_) ?_
        · intro e he
          split at he
          · rcases he with _ | ⟨_, he⟩
            · exact manDep_upper_snd (wpk (hp + 1)).1 (wpk (hp + 1)).2 k
            · cases he
          · cases he
        refine depOK_append (depOK_singleton (hq_head _ ?_)) ?_
        · intro x hx
          exact List.mem_append_left _ (List.mem_append_left _
            (List.mem_append_left _ hx))
        refine depOK_append ?_ ?_
        · -- the kid subtree, with its chunk feed
          have hout := hsub_dep
            (((((done' ++ [((wireOut (wpk (hp + 1)), true,
                  sk.wiresBefore (hp + 1) k + i) : Ev)])
              ++ [((lowerOut (wpk (hp + 1)), true,
                  sk.dsBefore (hp + 1) k
                    + dRank sk (wpk (hp + 1)) k i) : Ev)])
              ++ (if (lastDOf sk (hp + 1) k == some i) then
                  [((upperOut (wpk (hp + 1)), true, k) : Ev)] else []))
              ++ [((askedOut (wpk (hp + 2)), true,
                  sk.wiresBefore (hp + 1) k + i) : Ev)]))
            (fun x hx => List.mem_append_left _ (List.mem_append_left _
              (List.mem_append_left _ (List.mem_append_left _ hx))))
            (List.mem_append_left _ (List.mem_append_left _
              (List.mem_append_left _ (List.mem_append_right _
                (List.mem_cons_self ..)))))
            (List.mem_append_right _ (List.mem_cons_self ..))
          rw [← chunkQ_eq_feed sk hwf hh hk hi] at hout
          exact hout
        · -- the remaining kids of this scope
          refine ihm (i + 1) (by omega) _ ?_ ?_
          · intro hpos
            have hidx : sk.wiresBefore (hp + 1) k + (i + 1) - 1
                = sk.wiresBefore (hp + 1) k + i := by omega
            rw [hidx]
            exact ⟨List.mem_append_right _ (hself _).1,
              List.mem_append_right _ (hself _).2⟩
          · intro h'' hlt hpos
            have hidx : sk.wiresBefore (hp + 1) k + (i + 1)
                = sk.wiresBefore (hp + 1) k + i + 1 := by omega
            rw [hidx] at hpos ⊢
            have hFlen : (chunkQ sk (hp + 1) k i).length
                = sk.nChildren hp
                    (sk.stageScope hp (sk.wiresBefore (hp + 1) k + i)) := by
              rw [chunkQ_length]
              exact qCount_eq_kid_nChildren sk hwf (by omega) hh hk hi
            obtain ⟨hw, ha⟩ := hdeep_next (chunkQ sk (hp + 1) k i)
              hFlen (chunkQ_owner sk (by omega) hh k i)
              (walkIdx_lt sk (by omega) hh) h'' hlt hpos
            constructor
            · rcases hw with hw | hw
              · exact List.mem_append_left _ (List.mem_append_left _
                  (List.mem_append_left _ (List.mem_append_left _
                    (List.mem_append_left _ hw))))
              · exact List.mem_append_right _ hw
            · rcases ha with ha | ha
              · exact List.mem_append_left _ (List.mem_append_left _
                  (List.mem_append_left _ (List.mem_append_left _
                    (List.mem_append_left _ ha))))
              · exact List.mem_append_right _ ha
      · -- undisputed kid: wire, feed query, childless subtree
        rw [if_neg hD, if_neg (by simp)]
        simp only [Option.toList, List.cons_append, List.append_assoc]
        have h0 : sk.nChildren hp
            (sk.stageScope hp (sk.wiresBefore (hp + 1) k + i)) = 0 :=
          nChildren_kid_notD sk hwf (by omega) hh hk hi
            (by simpa using hD)
        refine depOK_cons (hwire_head done' fun x hx => hx) ?_
        refine depOK_append (depOK_singleton (hq_head _ ?_)) ?_
        · intro x hx
          exact List.mem_append_left _ hx
        refine depOK_append ?_ ?_
        · -- the childless subtree
          have hout := hsub_dep
            ((done' ++ [((wireOut (wpk (hp + 1)), true,
                sk.wiresBefore (hp + 1) k + i) : Ev)])
              ++ [((askedOut (wpk (hp + 2)), true,
                sk.wiresBefore (hp + 1) k + i) : Ev)])
            (fun x hx => List.mem_append_left _
              (List.mem_append_left _ hx))
            (List.mem_append_left _ (List.mem_append_right _
              (List.mem_cons_self ..)))
            (List.mem_append_right _ (List.mem_cons_self ..))
          rw [scopeFeed_nil sk h0] at hout
          exact hout
        · -- the remaining kids of this scope
          refine ihm (i + 1) (by omega) _ ?_ ?_
          · intro hpos
            have hidx : sk.wiresBefore (hp + 1) k + (i + 1) - 1
                = sk.wiresBefore (hp + 1) k + i := by omega
            rw [hidx]
            exact ⟨List.mem_append_right _ (hself _).1,
              List.mem_append_right _ (hself _).2⟩
          · intro h'' hlt hpos
            have hidx : sk.wiresBefore (hp + 1) k + (i + 1)
                = sk.wiresBefore (hp + 1) k + i + 1 := by omega
            rw [hidx] at hpos ⊢
            obtain ⟨hw, ha⟩ := hdeep_next ([] : List Ev)
              (by simp [h0]) (fun e he => nomatch he)
              (walkIdx_lt sk (by omega) hh) h'' hlt hpos
            constructor
            · rcases hw with hw | hw
              · exact List.mem_append_left _ (List.mem_append_left _
                  (List.mem_append_left _ hw))
              · exact List.mem_append_right _ hw
            · rcases ha with ha | ha
              · exact List.mem_append_left _ (List.mem_append_left _
                  (List.mem_append_left _ ha))
              · exact List.mem_append_right _ ha

/-- A prologue wire receive waits exactly for the in-flight wire. -/
private theorem prologue_head {done : List Ev} {h k : Nat}
    (hwin : ((wireIn (wpk h), true, k) : Ev) ∈ done) :
    ∀ d, manDep ((wireIn (wpk h), false, k) : Ev) = some d →
      d ∈ done := by
  intro d hd
  rw [show (wireIn (wpk h) : Chan)
      = Chan.wire (wpk h).1.other ((wpk h).2 + 1) from rfl,
    manDep_wire_rcv] at hd
  cases hd
  exact hwin

/-- A prologue asked receive waits exactly for the in-flight query. -/
private theorem prologue_head_asked {done : List Ev} {h k : Nat}
    (hain : ((askedIn (wpk h), true, k) : Ev) ∈ done) :
    ∀ d, manDep ((askedIn (wpk h), false, k) : Ev) = some d →
      d ∈ done := by
  intro d hd
  rw [show (askedIn (wpk h) : Chan)
      = Chan.asked (wpk h).1 (wpk h).2 from rfl,
    manDep_asked_rcv] at hd
  cases hd
  exact hain

/-- THE MASTER INDUCTION: a subtree op's events are dep-closed against
any past holding the subtree's entry context — the in-flight wire and
query for the scope itself, and per deeper stage the prologue receives
of the last scope before this subtree's window. -/
theorem dep_scope (hwf : sk.wellFormed = true) :
    ∀ (h : Nat), h < sk.rootH →
    ∀ (k mF : Nat) (done : List Ev),
      k < sk.stageLen h →
      (∀ e ∈ scopeFeed sk h k, evOwner sk e = mF) →
      mF < walkIdx sk h →
      ((wireIn (wpk h), true, k) : Ev) ∈ done →
      ((askedIn (wpk h), true, k) : Ev) ∈ done →
      (∀ h'', h'' < h → 0 < descIdx sk h'' (h - h'') k →
        ((wireIn (wpk h''), false,
            descIdx sk h'' (h - h'') k - 1) : Ev) ∈ done
        ∧ ((askedIn (wpk h''), false,
            descIdx sk h'' (h - h'') k - 1) : Ev) ∈ done) →
      DepOK done (opEvents sk (.scope h k (scopeFeed sk h k))) := by
  intro h
  induction h with
  | zero =>
      intro hh k mF done hk hFo hmF hwin hain hdeep
      have hD0 : ∀ i, sk.childIsD 0 (sk.stageScope 0 k) i = false :=
        fun _ => rfl
      have hLn : lastDOf sk 0 k = none := by
        unfold lastDOf
        rw [List.getLast?_eq_none_iff, List.filter_eq_nil_iff]
        intro a _
        rw [hD0 a]
        exact Bool.false_ne_true
      have hE := opEvents_scope_eq sk (Nat.le_of_lt hk)
        (scopeFeed sk 0 k)
      rw [hLn,
        if_pos (show ((none : Option Nat) == none) = true by rfl)] at hE
      rw [hE]
      refine depOK_cons (prologue_head hwin) ?_
      refine depOK_cons
        (prologue_head_asked (List.mem_append_left _ hain)) ?_
      refine depOK_of_none ?_
      intro e he
      rcases List.mem_append.1 he with he | he
      · rcases he with _ | ⟨_, he⟩
        · exact manDep_upper_snd (wpk 0).1 (wpk 0).2 k
        · cases he
      · obtain ⟨i, hir, hei⟩ := List.mem_flatMap.1 he
        have hilt : i < sk.nChildren 0 (sk.stageScope 0 k) :=
          List.mem_range.1 hir
        rw [opEvents_kid_eq sk 0 k none (sk.wiresBefore 0 k) i
            (scopeFeed sk 0 k),
          if_neg (by rw [hD0 i]; exact Bool.false_ne_true),
          if_pos (show ((0 : Nat) == 0) = true from rfl),
          List.append_nil] at hei
        rcases hei with _ | ⟨_, hei⟩
        · exact manDep_wire_snd_none (Or.inl rfl)
        · cases hfi : (scopeFeed sk 0 k)[i]? with
          | none => rw [hfi] at hei; cases hei
          | some q =>
              rw [hfi] at hei
              rw [scopeFeed_getElem? sk hilt] at hfi
              cases hfi
              rcases hei with _ | ⟨_, hei⟩
              · exact manDep_leafReq_snd _
              · cases hei
  | succ hp ih =>
      intro hh k mF done hk hFo hmF hwin hain hdeep
      have hE := opEvents_scope_eq sk (Nat.le_of_lt hk)
        (scopeFeed sk (hp + 1) k)
      rw [hE]
      refine depOK_cons (prologue_head hwin) ?_
      refine depOK_cons
        (prologue_head_asked (List.mem_append_left _ hain)) ?_
      refine depOK_append (depOK_of_none ?_) ?_
      · intro e he
        split at he
        · rcases he with _ | ⟨_, he⟩
          · exact manDep_upper_snd (wpk (hp + 1)).1 (wpk (hp + 1)).2 k
          · cases he
        · cases he
      · rw [List.range_eq_range']
        refine dep_kids sk hwf hh hk
          (fun k' mF' done' hk' hFo' hmF' hw ha hd =>
            ih (by omega) k' mF' done' hk' hFo' hmF' hw ha hd)
          _ 0 (by omega) _ ?_ ?_
        · -- the kid-stage rolling prologue, from the entry context
          intro hpos
          have h1 : hp + 1 - hp = 1 := by omega
          have hd := hdeep hp (by omega)
          rw [h1] at hd
          have hd' := hd hpos
          exact ⟨List.mem_append_left _ (List.mem_append_left _
              (List.mem_append_left _ hd'.1)),
            List.mem_append_left _ (List.mem_append_left _
              (List.mem_append_left _ hd'.2))⟩
        · -- the deeper rolling context, telescoped one stage down
          intro h'' hlt hpos
          have hstep : descIdx sk h'' (hp + 1 - h'') k
              = descIdx sk h'' (hp - h'') (sk.wiresBefore (hp + 1) k) := by
            rw [show hp + 1 - h'' = (hp - h'') + 1 from by omega,
              descIdx_succ,
              show h'' + (hp - h'') + 1 = hp + 1 from by omega]
          have hd := hdeep h'' (by omega)
          rw [hstep] at hd
          have hd' := hd hpos
          exact ⟨List.mem_append_left _ (List.mem_append_left _
              (List.mem_append_left _ hd'.1)),
            List.mem_append_left _ (List.mem_append_left _
              (List.mem_append_left _ hd'.2))⟩

-- ====================================================== top assembly

/-- The root scope's feed is ropen's query tail. -/
private theorem ropen_drop_eq_feed (hwf : sk.wellFormed = true) :
    (ropenEvents sk).drop 3 = scopeFeed sk (sk.rootH - 1) 0 := by
  have heven := (wf_rootH hwf).1
  have hge := (wf_rootH hwf).2
  have hwpk : wpk sk.rootH = (Party.R, sk.rootH) := by
    simp [wpk, heven]
  have hchan : askedOut (wpk (sk.rootH - 1 + 1))
      = Chan.asked Party.R (sk.rootH - 2) := by
    rw [show sk.rootH - 1 + 1 = sk.rootH from by omega, hwpk]
    unfold askedOut
    rw [if_neg (by omega)]
  have hcount : sk.nChildren (sk.rootH - 1)
      (sk.stageScope (sk.rootH - 1) 0) = sk.rootPending := by
    rw [wf_stageScope_top sk hwf]
    unfold Skel.nChildren Skel.rootPending
    rw [if_neg (by simp; omega)]
  unfold scopeFeed
  rw [hchan, hcount, show sk.wiresBefore (sk.rootH - 1) 0 = 0 from rfl]
  show ((List.range sk.rootPending).map fun j =>
      ((Chan.asked Party.R (sk.rootH - 2), true, j) : Ev)) = _
  unfold seg
  refine List.map_congr_left fun j _ => ?_
  rw [Nat.zero_add]

/-- The whole opening future is dep-closed: the openers carry their
own seq-0 predecessors, and the root scope enters with the openers as
context. -/
theorem weave_depOK (hwf : sk.wellFormed = true) :
    DepOK [] ((weaveOps sk).flatMap (opEvents sk)) := by
  have heven := (wf_rootH hwf).1
  have hge := (wf_rootH hwf).2
  rw [weave_flatMap]
  have htake : (ropenEvents sk).take 3
      = [(Chan.wire Party.I sk.rootH, false, 0),
         (Chan.wire Party.R sk.rootH, true, 0),
         (Chan.rootres, true, 0)] := rfl
  refine depOK_append ?_ ?_
  · -- the five opener events
    rw [htake]
    show DepOK []
      [(Chan.wire Party.I sk.rootH, true, 0),
       (Chan.asked Party.I (sk.rootH - 1), true, 0),
       (Chan.wire Party.I sk.rootH, false, 0),
       (Chan.wire Party.R sk.rootH, true, 0),
       (Chan.rootres, true, 0)]
    refine depOK_cons (fun d hd => ?_) ?_
    · rw [manDep_wire_snd_none (Or.inr rfl)] at hd
      cases hd
    refine depOK_cons (fun d hd => ?_) ?_
    · rw [show manDep (Chan.asked Party.I (sk.rootH - 1), true, 0)
          = none from manDep_asked_snd_none] at hd
      cases hd
    refine depOK_cons (fun d hd => ?_) ?_
    · rw [manDep_wire_rcv] at hd
      cases hd
      simp
    refine depOK_cons (fun d hd => ?_) ?_
    · rw [manDep_wire_snd_none (Or.inr rfl)] at hd
      cases hd
    refine depOK_singleton (fun d hd => ?_)
    rw [manDep_rootres_snd] at hd
    cases hd
  · -- the root scope
    rw [ropen_drop_eq_feed sk hwf]
    have hodd : (sk.rootH - 1) % 2 = 1 := by omega
    have hwpkr : wpk (sk.rootH - 1) = (Party.I, sk.rootH - 1) := by
      simp [wpk, hodd]
    refine dep_scope sk hwf (sk.rootH - 1) (by omega) 0 1 _
      (by rw [wf_stageLen_top sk hwf]; omega) ?_ ?_ ?_ ?_ ?_
    · -- feed owners: ropen's tail belongs to slot 1
      intro e he
      rw [← ropen_drop_eq_feed sk hwf] at he
      exact ropen_owner sk hwf e (List.mem_of_mem_drop he)
    · -- 1 < walkIdx (rootH - 1) = 2
      unfold walkIdx
      omega
    · -- the in-flight wire: ropen's answering wire
      have hwr : wireIn (wpk (sk.rootH - 1))
          = Chan.wire Party.R sk.rootH := by
        rw [show wireIn (wpk (sk.rootH - 1))
            = Chan.wire (wpk (sk.rootH - 1)).1.other
                ((wpk (sk.rootH - 1)).2 + 1) from rfl, hwpkr]
        show Chan.wire Party.I.other (sk.rootH - 1 + 1) = _
        rw [show sk.rootH - 1 + 1 = sk.rootH from by omega]
        rfl
      rw [hwr]
      simp [htake, iopenEvents]
    · -- the in-flight query: iopen's root query
      have har : askedIn (wpk (sk.rootH - 1))
          = Chan.asked Party.I (sk.rootH - 1) := by
        rw [show askedIn (wpk (sk.rootH - 1))
            = Chan.asked (wpk (sk.rootH - 1)).1
                (wpk (sk.rootH - 1)).2 from rfl, hwpkr]
      rw [har]
      simp [htake, iopenEvents]
    · -- no deeper context: descent from index 0 stays at 0
      intro h'' _ hpos
      rw [descIdx_zero_arg] at hpos
      omega

/-- Layer D's form: the interpreter's initial ghost future is
dep-closed. -/
theorem weave_goEvents_depOK (hwf : sk.wellFormed = true) :
    DepOK [] (goEvents sk (weaveFuel sk) (weaveOps sk)) := by
  rw [goEvents_weave sk (weave_events_length sk hwf)]
  exact weave_depOK sk hwf

end StreamingMirror.Sched
