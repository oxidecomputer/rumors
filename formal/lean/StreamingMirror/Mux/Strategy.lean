/-
The strategy interface: observations, the strategy classes, and locality
(the adjudicated observation and locality rulings, stated in full below).

The `MObs` alphabet and the `Strategy` type themselves live in
Mux/Basic.lean — the `push` arm of `Mux.apply` consults σ, so the types
must precede the transition function in the import spine. This file holds
everything ABOUT strategies: observation extraction from runs, trace
consistency, the work-conserving class (the impossibility theorem T3's
hypothesis), the per-party view of a skeleton, and locality as
invariance.

# The observation ruling

Included: every base action the machine executed (`.act`), its own flush
receipts (`.pushed`), its demux's deliveries (`.delivered` — at delivery,
pre-consumption: the slot-peek ruling, cross-examination-ratified and
load-bearing for the landed coverage proof, though stage-0 P4 showed it
is not a demonstrated liveness necessity;
decision-for-Finch #1). Excluded: remote delivery, remote consumption,
own-pipe occupancy drain — a consumption receipt is a covert credit and
would dissolve the frozen-message-set charter from inside the observation
type (decision-for-Finch #2).

# Locality as invariance, not a view type

σ receives the FULL skeleton; locality is the hypothesis that σ cannot
USE what party `p` cannot see. `LocalEq p sk sk'` computes equality of
the two skeletons' p-views — the projection keeping, per scope, the
existence and content of p-held children and NOTHING peer-side:

- at a scope where `p` is the asker, held children range over D
  (recurse) and R (a cut: the subtree exists in p's tree, absent from
  the skeleton); both are visible, in radix order;
- at a scope where `p` is the answerer, only D children are visible —
  an R child is one the answerer LACKS, invisible at session start
  (the adjudicated role-dependent alphabet: the asker of a scope sees
  its D and R-cut children, the answerer its D children only);
- `leafReqs` is erased from both views (not view-determined on either
  side — the adjudicated erasure: leaf-request counts vanish from BOTH
  parties' views).

`LocalStrategy` then demands invariance across `LocalEq` pairs — guarded
by `Consistent` (added by the synthesis): without the guard, an
unreachable trace that announces the skeletons' difference would break a
genuinely local strategy's invariance vacuously-wrongly.

The mandatory controls are landed: `LocalEq` nondegeneracy
(`localEq_nondegenerate`) and the strategy-level oracle refutation
(`oracle_not_localStrategy`, with `Consistent` certificates) in
Oracle/Controls.lean; the Rust same-p-tree proptest bridge in track D
(the adjudicated invariance form). `LocalEq`'s label-visibility residue —
the relation is finer than session-start indistinguishability — is
recorded on its docstring; the charter-honest re-grounding landed as
`CharterLocal` (Mux/Causal.lean, the σ*-causal track), a grain
INCOMPARABLE to this one — Proofs/C1.lean's module doc is the finding
of record.
-/
import StreamingMirror.Mux.Basic

namespace StreamingMirror.Mux

open Model

-- ============================================================ observations

/-- The observation trace machine `p` accumulates along a run: replay the
action list from init and read off `p`'s history.

`none` when some action's guard fails. Histories are state-carried
(`MState.hist`), so extraction is a projection of the reached state —
this wrapper is the run-level form the locality statements quantify
over. -/
def obsOf (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy)
    (acts : List MAction) (p : Party) : Option (List MObs) :=
  (mrun sk ax C σI σR (init sk) acts).map fun s => s.hist p

/-- Trace `tr` is a possible observation history of machine `p` under
skeleton `sk`: some reachable muxed state carries it.

The axiom mode, capacity, and strategy pair are existential — consistency
asks only "could `p` ever see this?", which is what the `LocalStrategy`
guard needs: it must exclude traces no run produces (whose divergence
could leak the skeletons' difference) without tying locality to one
scheduler. Decidable on pins via the run machinery; a `Prop` for the
general statement: a strategy is local when equal views plus equal
consistent observations force equal choices. -/
def Consistent (p : Party) (sk : Skel) (tr : List MObs) : Prop :=
  ∃ (ax : AxMode) (C : Nat) (σI σR : Strategy) (s : MState),
    MReachable sk ax C σI σR s ∧ s.hist p = tr

/-- A kernel-checked replay certifies consistency: if `obsOf` computes
the trace, some reachable state carries it — the glue that turns a
`decide`-checked `mrun` into a `Consistent` witness. -/
theorem Consistent.of_obsOf (ax : AxMode) (C : Nat) (σI σR : Strategy)
    (acts : List MAction) {p : Party} {sk : Skel} {tr : List MObs}
    (h : obsOf sk ax C σI σR acts p = some tr) : Consistent p sk tr := by
  rw [obsOf] at h
  cases hm : mrun sk ax C σI σR (init sk) acts with
  | none => rw [hm] at h; cases h
  | some s =>
      rw [hm] at h
      injection h with h
      exact ⟨ax, C, σI, σR, s, mrun_reachable hm, h⟩

-- ======================================================== work conservation

/-- Reachable under SOME axiom mode, capacity, and strategy pair — the
state universe the strategy-class predicates quantify over.

Existential so the classes stay usable at any concrete instantiation:
a state reachable under the specific pair being refuted is a fortiori
`MReachableAny`. -/
def MReachableAny (sk : Skel) (s : MState) : Prop :=
  ∃ (ax : AxMode) (C : Nat) (σI σR : Strategy), MReachable sk ax C σI σR s

/-- The wire streams party `p` could push right now: pipe room plus a
committed hand (`holdsWire`), enumerated over `p`'s stream heights.

This is the enabled-push set `WorkConserving` ranges over; it mirrors the
`push` guard exactly (`firePush` succeeds on `h` iff `h` is in this
list — the intended stage-2 lemma `enabledPushes_spec`). -/
def enabledPushes (sk : Skel) (C : Nat) (p : Party) (s : MState) :
    List Nat :=
  if (s.pipe p).length < C then
    (wireHeights sk p).filter fun h => holdsWire sk p h s.base
  else []

/-- σ never declines while it holds a pushable frame: at any reachable
state where `p`'s enabled-push set is nonempty, σ names a member.

The choice of WHICH member is free — work-conservation is precisely
"push before your observation changes", never frame choice (the
cross-examination's restatement). A work-conserving strategy may still *wait* on a full
pipe (the room conjunct lives in `enabledPushes`); it may never idle
with room and a committed hand. This is T3's hypothesis class — the
shipped mux's `bottomMostReady` (Mux/Instances.lean) is the pinned
concrete instance (`bottomMostReady_wc`, Mux/Proofs/Inhabitation.lean),
and σ* is definitionally outside it (the right to idle is the entire
frontier of the impossibility). -/
def WorkConserving (p : Party) (σ : Strategy) : Prop :=
  ∀ (sk : Skel) (C : Nat) (s : MState), MReachableAny sk s →
    enabledPushes sk C p s ≠ [] →
    ∃ h, σ sk (s.hist p) = some h ∧ h ∈ enabledPushes sk C p s

-- ================================================================ locality

/-- Preorder token serialization of party `p`'s view of the subtree at
scope `i`, with `fuel` bounding the descent (heights strictly decrease,
so `rootH + 1` suffices from the root; the fuel keeps the recursion
structural per the house rule).

Per child of the scope, in radix (kid-list) order: a D child emits an
open bracket `2`, its own serialization, and a close bracket `3` — held
by `p` on both sides of the role split, content recursed; an R child
emits the leaf token `4` when `p` is the scope's asker (a held cut:
p's tree has the subtree, the skeleton does not) and NOTHING when `p`
is the answerer (a child the answerer lacks — invisible at session
start). `leafReqs` is never emitted (erased from both views). The
bracketing makes the encoding prefix-unambiguous, so token-list equality
is view equality; heights are depth-determined once `rootH` is fixed
(`LocalEq` compares it separately), so no height token is needed.

Deviation from the adjudicated encoding recorded: the nested
inductive is flattened to a token list — same information, and the
flat form is kernel-`decide`-friendly without nested-inductive
`DecidableEq` machinery. -/
def viewEnc (p : Party) (sk : Skel) : Nat → Nat → List Nat
  | 0, _ => [0]
  | fuel + 1, i =>
      let sc := sk.scope i
      sc.kids.flatMap fun k =>
        if (sk.scope k).kind == Kind.D then
          2 :: viewEnc p sk fuel k ++ [3]
        else if asks p sc.height then [4]
        else []

/-- Equality of the adjudicated view projections: equal session
parameters and equal `viewEnc` token lists for party `p`.

The parameters (`rootH`, `fan`, `capLevel`) are commonly-known
configuration, not remote information, so they belong to the view.
Nondegeneracy (two DISTINCT LocalEq skeletons exist) is the
`localEq_nondegenerate` kernel control — without it `LocalStrategy`
would be vacuous.

# The label-visibility residue (phase-4 F3)

This relation is provably FINER than "indistinguishable to `p` at
session start": per p-held child, `viewEnc` emits the SKELETON's label
(D recursed, asker-side R-cut as `[4]`), and with `p`'s own tree held
fixed a held child's label ranges over {D, R-cut, absent} as a function
of the PEER's tree (the adjudicated ground-truth table: D vs R-cut vs
M-absent labels of p-held children are peer-determined facts). So skeleton
pairs realizable by one common p-tree exist that `LocalEq` refuses to
relate: every `LocalEq`-related pair is indistinguishable, not
conversely. The charter-honest re-grounding landed as `CharterLocal`
(Mux/Causal.lean): invariance across equal ANNOUNCED views. The two
grains are INCOMPARABLE, not nested — `LocalEq` pairs may differ in
announced content (answerer-side R children and `leafReqs` of
announced scopes are `viewEnc`-erased yet frame-announced), while
announced-view pairs may differ in unannounced view structure, so the
a-fortiori transfer fails in BOTH directions — kernel-pinned at
relation and class strength in Mux/Proofs/Grains.lean;
Proofs/C1.lean's module doc is the finding of record. In particular
refutations do NOT transfer: the T9 witnesses differ only in
`leafReqs`, which `AView.recs` carries, so `oracle_not_localStrategy`
says nothing about `CharterLocal` — the oracle's charter-grain
nonlocality has its own pin (`oracle_not_charterLocal`, Grains.lean).
This definition stays as the landed controls' vocabulary. -/
def LocalEq (p : Party) (sk sk' : Skel) : Bool :=
  sk.rootH == sk'.rootH && sk.fan == sk'.fan &&
  sk.capLevel == sk'.capLevel &&
  viewEnc p sk (sk.rootH + 1) 0 == viewEnc p sk' (sk'.rootH + 1) 0

/-- σ is a LOCAL strategy for party `p`: invariant across skeletons `p`
cannot tell apart, on every trace both skeletons can produce.

σ may *read* remote structure but provably cannot *use* it. The
`Consistent` guards (added by the phase-2 synthesis) keep
the quantification honest: a trace only one skeleton can produce already
announces the difference, and demanding invariance there would wrongly
disqualify strategies that are local in every reachable situation. The
C2 oracle is then literally a `Strategy` that is NOT `LocalStrategy` —
`oracle_not_localStrategy` (Oracle/Controls.lean) refutes it at this
definition's full strength, `Consistent` certificates included — so
the necessity corollary T6 is a statement about this one hypothesis.
Grain caveat: this class is INCOMPARABLE to the charter-grain
`CharterLocal` (Mux/Causal.lean) — neither memberships nor
refutations transfer in either direction (Proofs/C1.lean's module doc
records why; Mux/Proofs/Grains.lean pins both directions in the
kernel), so every claim through this definition is about THIS grain
only. -/
def LocalStrategy (p : Party) (σ : Strategy) : Prop :=
  ∀ (sk sk' : Skel) (tr : List MObs), LocalEq p sk sk' = true →
    Consistent p sk tr → Consistent p sk' tr →
    σ sk tr = σ sk' tr

end StreamingMirror.Mux
