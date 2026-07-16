/-
Cross-pinning the Lean model to the Phase A Quint matrix
(formal/quint/instances.qnt): the same skeletons, executed to
completion by a deterministic greedy scheduler, must reach `terminal`
with every channel within capacity. Negative-control instances get
their stuck witnesses later, as explicit action lists converted from
the checked-in ITF traces (they are schedule-dependent, so the greedy
scheduler proves nothing about them either way).

`#eval` gives the fast feedback; the `decide` theorems pin the facts in
the kernel so `lake build` fails if the model drifts from the matrix.
-/
import StreamingMirror.Model

namespace StreamingMirror.Pin

open Model

/-- Shorthand for skeleton literals. -/
def sc (kind : Kind) (height : Nat) (kids : List Nat) (leafReqs : Nat := 0) : Scope :=
  ⟨kind, height, kids, leafReqs⟩

/-- instances.qnt `smokeChain`: one dispute chain exercising every
structurally distinct stage at ROOT_H = 4. -/
def smokeChain : Skel :=
  { scopes :=
      [sc .D 4 [1], sc .D 3 [2], sc .D 2 [3], sc .D 1 [] (leafReqs := 1)]
    rootH := 4, fan := 2, capLevel := 2 }

/-- instances.qnt `rMix`: R children at every legal height. -/
def rMix : Skel :=
  { scopes :=
      [sc .D 4 [1, 2], sc .D 3 [3, 4], sc .R 3 [], sc .D 2 [5, 6],
       sc .R 2 [], sc .D 1 [] (leafReqs := 2), sc .R 1 []]
    rootH := 4, fan := 3, capLevel := 3 }

/-- instances.qnt `comb6`: dispute branching at every height at
ROOT_H = 6 (internal→internal handoff on both parties). -/
def comb6 : Skel :=
  { scopes :=
      [sc .D 6 [1], sc .D 5 [2, 3], sc .D 4 [4, 5], sc .D 4 [],
       sc .D 3 [6], sc .R 3 [], sc .D 2 [7], sc .D 1 [] (leafReqs := 2)]
    rootH := 6, fan := 2, capLevel := 2 }

/-- instances.qnt `pyramidFull` (and, with `capLevel` overridden, the
`pyramidC2`/`pyramidC1` tightness twins): root fans to 2 parents, each
disputing a full fan of 4 immediately-resolving children. -/
def pyramid (capLevel : Nat) : Skel :=
  { scopes :=
      [sc .D 4 [1, 2],
       sc .D 3 [3, 4, 5, 6], sc .D 3 [7, 8, 9, 10],
       sc .D 2 [], sc .D 2 [], sc .D 2 [], sc .D 2 [],
       sc .D 2 [], sc .D 2 [], sc .D 2 [], sc .D 2 []]
    rootH := 4, fan := 4, capLevel := capLevel }

/-- Greedy deterministic scheduler: fire the first enabled action, up to
`fuel` steps. Returns the final state (fixed point or fuel exhaustion). -/
def drive (sk : Skel) (ax : AxMode) : Nat → State → State
  | 0, s => s
  | fuel + 1, s =>
      match (allActions sk).firstM (fun a => apply sk ax a s) with
      | some s' => drive sk ax fuel s'
      | none => s

/-- One pinning verdict: greedy execution reaches `terminal` with every
channel drained (conservation) — the Lean twin of check.sh's positive
expectation plus `drainedAtTerminal`. -/
def completes (sk : Skel) (fuel : Nat := 2000) : Bool :=
  let s := drive sk .full fuel (init sk)
  terminal sk s && (allActionsDrained sk s)
where
  /-- All channels empty at rest (the level/root channels have no close
  observation, so this is the conservation check). -/
  allActionsDrained (sk : Skel) (s : State) : Bool :=
    -- occupancy is a function; check the channels the model can touch
    (sk.walkKeys.all fun pk =>
      s.chan (wireIn pk) == 0 && s.chan (askedIn pk) == 0 &&
      s.chan (wireOut pk) == 0 && s.chan (lowerOut pk) == 0 &&
      s.chan (upperOut pk) == 0) &&
    (sk.asmKeys.all fun pk =>
      s.chan (asmResChan pk) == 0 && s.chan (asmLevelChan pk) == 0 &&
      s.chan (sk.asmOutChan pk) == 0) &&
    s.chan Chan.leafRequests == 0 && s.chan (Chan.level Party.I 0) == 0 &&
    s.chan Chan.rootret == 0 && s.chan Chan.rootrets == 0 &&
    s.chan Chan.rootres == 0

-- Fast feedback while developing:
#eval [smokeChain.wellFormed, rMix.wellFormed, comb6.wellFormed,
       (pyramid 4).wellFormed, (pyramid 2).wellFormed, (pyramid 1).wellFormed]
#eval [completes smokeChain, completes rMix, completes comb6,
       completes (pyramid 4), completes (pyramid 2)]

/-- Phantom-key regression (transcription-review finding): a walk action
at a key outside `walkKeys` must be rejected. `(R, 3)` is not a smokeChain
stage, but its `wireIn` aliases the opening channel `wire (I, 4)` — before
the key-membership guards, this exact trace stole the opening message and
reached a stuck state on a positive instance. -/
theorem phantom_walk_rejected :
    run smokeChain .full (init smokeChain)
      [.iopenChoose .wire, .iopenFire, .walkRecvWire (Party.R, 3)] = none := by
  native_decide

/-- The Phase A positive matrix, pinned in the kernel: every positive
skeleton is well-formed and greedily completes with conservation. -/
theorem positives_complete :
    (smokeChain.wellFormed && completes smokeChain) &&
    (rMix.wellFormed && completes rMix) &&
    (comb6.wellFormed && completes comb6) &&
    ((pyramid 4).wellFormed && completes (pyramid 4)) &&
    ((pyramid 2).wellFormed && completes (pyramid 2)) = true := by
  native_decide

end StreamingMirror.Pin
