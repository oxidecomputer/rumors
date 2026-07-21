/-
The strategy interface: observations, the strategy classes, and locality
(MUX-ADJUDICATION.md §2.3–§2.4).

The `MObs` alphabet and the `Strategy` type themselves live in
Mux/Basic.lean — the `push` arm of `Mux.apply` consults σ, so the types
must precede the transition function in the import spine. This file holds
everything ABOUT strategies: observation extraction from runs, trace
consistency, the work-conserving class (the impossibility theorem T3's
hypothesis), the per-party view of a skeleton, and locality as
invariance.

# The observation ruling (MUX-ADJUDICATION §2.3)

Included: every base action the machine executed (`.act`), its own flush
receipts (`.pushed`), its demux's deliveries (`.delivered` — at delivery,
pre-consumption: the slot-peek ruling, ratified per attack-refute F2;
decision-for-Finch #1). Excluded: remote delivery, remote consumption,
own-pipe occupancy drain — a consumption receipt is a covert credit and
would dissolve the frozen-message-set charter from inside the observation
type (decision-for-Finch #2).

# Locality as invariance, not a view type (MUX-ADJUDICATION §2.4)

σ receives the FULL skeleton; locality is the hypothesis that σ cannot
USE what party `p` cannot see. `LocalEq p sk sk'` computes equality of
the two skeletons' p-views — the projection keeping, per scope, the
existence and content of p-held children and NOTHING peer-side:

- at a scope where `p` is the asker, held children range over D
  (recurse) and R (a cut: the subtree exists in p's tree, absent from
  the skeleton); both are visible, in radix order;
- at a scope where `p` is the answerer, only D children are visible —
  an R child is one the answerer LACKS, invisible at session start
  (oracle-c2 §3.1's corrected role-dependent alphabet);
- `leafReqs` is erased from both views (not view-determined on either
  side — oracle-c2 §3.2's adjudicated erasure).

`LocalStrategy` then demands invariance across `LocalEq` pairs — guarded
by `Consistent` (added by the synthesis): without the guard, an
unreachable trace that announces the skeletons' difference would break a
genuinely local strategy's invariance vacuously-wrongly.

The mandatory controls (LocalEq nondegeneracy [decide], the oracle is
not local [decide], the Rust same-p-tree proptest bridge) are stage-2/4
obligations (MUX-ADJUDICATION §2.4, §4).
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
general statement (MUX-ADJUDICATION §2.4). -/
def Consistent (p : Party) (sk : Skel) (tr : List MObs) : Prop :=
  ∃ (ax : AxMode) (C : Nat) (σI σR : Strategy) (s : MState),
    MReachable sk ax C σI σR s ∧ s.hist p = tr

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
"push before your observation changes", never frame choice (attack-prove
F2's restatement). A work-conserving strategy may still *wait* on a full
pipe (the room conjunct lives in `enabledPushes`); it may never idle
with room and a committed hand. This is T3's hypothesis class — the
shipped mux's `bottomMostReady` (Mux/Instances.lean) is the pinned
concrete instance (`bottomMostReady_wc`, Mux/Proofs/Inhabitation.lean),
and σ* is definitionally outside it (the right to idle is the entire
frontier, MUX-ADJUDICATION §1.2). -/
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

Deviation from oracle-c2 §3.2 recorded: the `PView`/`PKid` nested
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

/-- Skeletons `sk` and `sk'` are indistinguishable to party `p` at
session start: equal session parameters and equal p-views.

The parameters (`rootH`, `fan`, `capLevel`) are commonly-known
configuration, not remote information, so they belong to the view. This
relation is where C1's meaning lives — too coarse and the impossibility
is trivial, too fine and strategies see remote structure; the definition
of record is the oracle-c2 §3.2 projection with the adjudicated
corrections (module doc). Nondegeneracy (two DISTINCT LocalEq skeletons
exist) is a mandatory stage-2 `decide` control — without it
`LocalStrategy` would be vacuous. -/
def LocalEq (p : Party) (sk sk' : Skel) : Bool :=
  sk.rootH == sk'.rootH && sk.fan == sk'.fan &&
  sk.capLevel == sk'.capLevel &&
  viewEnc p sk (sk.rootH + 1) 0 == viewEnc p sk' (sk'.rootH + 1) 0

/-- σ is a LOCAL strategy for party `p`: invariant across skeletons `p`
cannot tell apart, on every trace both skeletons can produce.

σ may *read* remote structure but provably cannot *use* it. The
`Consistent` guards (added by the synthesis, MUX-ADJUDICATION §2.4) keep
the quantification honest: a trace only one skeleton can produce already
announces the difference, and demanding invariance there would wrongly
disqualify strategies that are local in every reachable situation. The
C2 oracle is then literally a `Strategy` that is NOT `LocalStrategy`
(the `oracle_not_local` pin, stage 3), so the necessity corollary T6 is
a statement about this one hypothesis. -/
def LocalStrategy (p : Party) (σ : Strategy) : Prop :=
  ∀ (sk sk' : Skel) (tr : List MObs), LocalEq p sk sk' = true →
    Consistent p sk tr → Consistent p sk' tr →
    σ sk tr = σ sk' tr

end StreamingMirror.Mux
