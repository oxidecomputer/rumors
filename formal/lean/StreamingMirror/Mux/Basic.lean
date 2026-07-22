/-
The mux harness of record (the phase-2 adjudication's topology,
stated in full here): the single-pipe
transport wrapped around the untouched base model.

# Topology: hand + pipe(C) + slot(1), no staging cell

Per direction the harness has exactly three buffering sites between a
sender's choice and the consumer:

- **the committed hand** — the base model's blocked-sender-holds-item
  device (MODEL.md §5): the opener's chosen `wire` obligation or a walk's
  committed `.wire i`. This is the `WriteRequest` slot of the shipped mux
  read faithfully: the producer awaits the write+flush receipt, so its
  program order gates on the push itself. There is NO separate sender-side
  outbox a producer could walk away from — a staged harness would give
  every strategy slack the shipped system never had (§2.1 reason (i)).
- **the pipe** — one bounded FIFO of `Chan` tags per direction, capacity
  `C`, message-counted (one entry = one scope-level reply; byte-level
  soundness of one-reply slots is §5A's W = 1 argument, assumed at the
  model boundary). Entries are bare tags: payloads
  are opaque and identity is positional everywhere in the model, so the
  n-th occurrence of `wire p h` in the push history is seq n by the same
  canonical-numbering argument `Numbering.lean` proves for channel sides.
- **the demux slot** — the base model's cap-1 wire cell, reinterpreted as
  receiver-side. This reinterpretation is the whole trick: the cell moves
  to the receiving machine and the mux inserts the pipe *behind* it, so
  every wire *receive* arm and every receiver-side counting lemma
  transports verbatim.

# The extended alphabet

`MAction.base` carries the 23 base arms with three dispositions
(the adjudicated arm dispositions, each restated at its arm):

- 18 arms verbatim (wire receives read the demux slot exactly as the base
  model reads the wire cell);
- the three wire-send cases (`iopenFire`/`ropenFire` on a `wire`
  obligation, `walkFire` on a committed `.wire i`) are DISABLED as base
  actions — a wire send happens only through `MAction.push`, the
  strategy-gated mux move;
- the two wire close-receives (`walkCloseWire`, `absorbCloseWire`) gain
  the no-in-flight conjunct (a cross-examination repair): a close is sound only when
  no frame of the channel can still arrive, else a receiver could close
  under an in-flight frame and manufacture a spurious terminal. The
  must-fail pin for this guard is a Mux/Controls obligation (T0's negative
  control below in Mux/Controls.lean).

`MAction.push p` consults party `p`'s strategy and fires the named
committed wire obligation into pipe `p`; `MAction.deliver p` moves the
pipe head into its demux slot, head-of-line (the shipped FIFO discipline,
incoming.rs:60-92). Commits stay adversarial — σ gates pushes only
(the commits-stay-adversarial ruling); under `.impl` this costs nothing because the ledgers
totally order each scope's publications (`commit_totality`, the T1
obligation).

# Quantifier posture

`MuxDeadlockFree` fixes the strategies and leaves the endpoint
interleaving fully adversarial, exactly as the base `DeadlockFree`
(the adopted endpoint-adversary decision: strategies gate pushes only;
everything else stays scheduler-adversarial). Idling is not
a move: a strategy that returns `none` while nothing else is enabled
leaves the state `mstuck` — an idler carries a real liveness obligation
(the M3 posture).

# Conservativity

`mcanStep` enumerates `allMActions`; an accidental omission makes
`mstuck` easier to satisfy and `MuxDeadlockFree` harder to prove, so the
enumeration cannot silently weaken a claim (the Statement.lean:127-131
argument, verbatim). `mterminal`'s pipes-empty conjuncts are redundant
given flow conservation but stating them avoids needing that lemma before
the definition exists (`terminal_drained`, a stage-2 door).

# The byte-denomination caveat (canonical statement)

Every capacity in this harness is denominated in MESSAGES: one pipe
entry = one scope-level reply. Byte-level soundness of one-reply slots
is design/streaming-wire-deadlock.md §5A's W = 1 structural argument,
ASSUMED at the model boundary and not re-proven here
(the reply-denomination ruling). The direction that does not transfer for
free is liveness: a positive (deadlock-freedom or completion) theorem
at message denomination says less than its byte-level reading, so
every positive statement of record carries a one-line pointer to this
section — this is the §1 ruling's canonical home. Impossibilities
transfer unweakened (a jam at message grain is a jam at byte grain).
-/
import StreamingMirror.Model

namespace StreamingMirror.Mux

open Model

-- ================================================== observations, strategies

/-- One observation on a party's machine.

`.act` records every base action the machine executed (commits, fires,
internal sends and receives — everything on the machine is observable,
which maximizes locality and so strengthens the impossibility theorems);
`.pushed` is the machine's own flush receipt — the push action's
completion, never the remote drain (a consumption receipt would be a
covert credit; excluded, decision-for-Finch #2); `.delivered` fires at
demux delivery, pre-consumption — the slot-peek ruling of
the adjudicated slot-peek ruling, cross-examination-ratified (peek is
load-bearing for the LANDED coverage proof — `groundedPush` grounds
arrivals in `.delivered` — and faithful to the Rust demux, which
decodes every frame before routing; stage-0 P4 showed it is NOT a
demonstrated liveness necessity: no-peek causal σ* also survived the
probe sweep). -/
inductive MObs
  | act (a : Action)
  | pushed (h : Nat)
  | delivered (h : Nat)
  deriving DecidableEq, Repr

/-- A send-order strategy: given the session skeleton and this machine's
observation history, name the wire stream height to push next, or idle.

`none` = idle — the door σ* walks through, closed again for the
impossibility class by `WorkConserving` (Mux/Strategy.lean). Totality
makes the charter's "deterministic" free. The strategy classes
(`WorkConserving`, `LocalStrategy`) and the locality relation live in
Mux/Strategy.lean; the type is minted here because the `push` arm of
`Mux.apply` consults it. -/
def Strategy := Skel → List MObs → Option Nat

-- ========================================================== the mux surface

/-- Is this channel in the muxed wire family? Only the `Chan.wire`
channels cross the link (MODEL.md §4: "the pump's capacity-1 channel is
the wire"); every other channel is endpoint-internal and untouched. -/
def isWire : Chan → Bool
  | .wire _ _ => true
  | _ => false

/-- The producing party of a wire channel: `wire p h` rides pipe `p`.

Off the wire family the value is arbitrary (`I`); guards keep every use
on the family. -/
def wireParty : Chan → Party
  | .wire p _ => p
  | _ => .I

/-- The stream height of a wire channel — the `h` an `MObs.pushed h` or
`MObs.delivered h` observation names. Off the wire family the value is
arbitrary (0); pipe entries are `wire _ _` by construction. -/
def wireHeight : Chan → Nat
  | .wire _ h => h
  | _ => 0

/-- The wire stream heights party `p` produces: the opening wire
(`wire p rootH` — the openings route through the mux, old-mux faithful,
the adopted opening-route decision, old-mux faithful) plus one stream per
p-side walk stage. -/
def wireHeights (sk : Skel) (p : Party) : List Nat :=
  sk.rootH :: sk.walkKeys.filterMap fun pk =>
    if pk.1 == p then some pk.2 else none

-- =============================================================== the state

/-- The muxed system state: the untouched base state plus the two pipes
and the per-machine observation histories.

`base.chan` on wire channels now denotes the receiver-side demux slot
(see the module doc); `pipe p` holds the frames in flight from `p` to
`p.other`, head oldest, entries `Chan.wire p _` only (an invariant, not
enforced by the type — stage 2's `MInv` owns it); `hist p` is machine
`p`'s observation history, newest last. -/
structure MState where
  base : State
  pipe : Party → List Chan
  hist : Party → List MObs

/-- The extended action alphabet.

`base` carries the 23 protocol arms (wire sends disabled, wire closes
strengthened — module doc); `push p` is the strategy-gated mux move;
`deliver p` is the demux move — head-of-line, non-strategic. -/
inductive MAction
  | base (a : Action)
  | push (p : Party)
  | deliver (p : Party)
  deriving DecidableEq, Repr

/-- The machine a base action executes on, for observation attribution.

Openers and `finRet` live with their party's endpoint; walks and
assemblers carry their party in the key; the absorber is initiator-side
(it consumes the `wire R 0` provisions and feeds `level I 0`);
`finRes`/`finRets` are the responder-side finale (they drain `rootres`/
`rootrets`, both R-produced). -/
def actionParty : Action → Party
  | .iopenChoose _ | .iopenFire => .I
  | .ropenRecv | .ropenChoose _ | .ropenFire => .R
  | .walkRecvWire pk | .walkRecvAsked pk | .walkCommit pk _
  | .walkFire pk | .walkCloseWire pk | .walkCloseAsked pk => pk.1
  | .asmRecvRes pk | .asmRecvLevel pk | .asmSend pk | .asmClose pk => pk.1
  | .absorbRecvWire | .absorbRecvAsked | .absorbSend
  | .absorbCloseWire | .absorbCloseAsked => .I
  | .finRet => .I
  | .finRes | .finRets => .R

/-- Append observation `o` to machine `p`'s history, newest last. -/
def recordObs (hist : Party → List MObs) (p : Party) (o : MObs) :
    Party → List MObs :=
  fun q => if q == p then hist q ++ [o] else hist q

/-- Does some `p`-process hold a committed obligation on `wire p h`?

This is the "hand" of the adjudicated topology: the opener's chosen
`wire` obligation at `h = rootH`, or walk `(p, h)`'s committed `.wire i`
in its publishing phase. At most one `p`-process can hold `wire p h`
(the opener at `rootH`, the single walk at `h < rootH`), so the height
names the holder uniquely — which is why a strategy's output is a bare
height. `firePush` succeeds exactly when this holds and the pipe has
room (the intended stage-2 lemma `firePush_isSome`). -/
def holdsWire (sk : Skel) (p : Party) (h : Nat) (s : State) : Bool :=
  if h == sk.rootH then
    match p with
    | .I => s.iopenCh == some .wire
    | .R => s.ropenCh == some .wire
  else
    let ws := s.walk (p, h)
    sk.walkKeys.contains (p, h) && ws.phase == 2 &&
      (match ws.committed with
       | some (.wire _) => true
       | _ => false)

-- ============================================================== transitions

/-- Fire party `p`'s committed obligation on stream `h` into pipe `p`.

The mux move's engine: guard = pipe room (`length < C`) plus a committed
hand on `wire p h` (`holdsWire`); effect = the base wire-send effect with
the channel bump replaced by a pipe append (the frame is now in flight,
not in the receiver's slot), plus the flush receipt `.pushed h` on `p`'s
history. Returns `none` when the guard fails — a strategy naming an
unheld stream or pushing into a full pipe is simply disabled. -/
def firePush (sk : Skel) (C : Nat) (p : Party) (h : Nat) (s : MState) :
    Option MState :=
  if (s.pipe p).length < C then
    let c := Chan.wire p h
    let push (b : State) : MState :=
      { base := b
        pipe := fun q => if q == p then s.pipe q ++ [c] else s.pipe q
        hist := recordObs s.hist p (.pushed h) }
    if h == sk.rootH then
      match p with
      | .I =>
          match s.base.iopenCh with
          | some .wire =>
              some (push { s.base with iopenWire := true, iopenCh := none })
          | _ => none
      | .R =>
          match s.base.ropenCh with
          | some .wire =>
              some (push { s.base with ropenWire := true, ropenCh := none })
          | _ => none
    else
      let pk := (p, h)
      let ws := s.base.walk pk
      match ws.committed with
      | some (.wire i) =>
          if sk.walkKeys.contains pk && ws.phase == 2 then
            some (push (setWalk s.base pk
              (normWalk sk h (fireOblig ws (.wire i)))))
          else none
      | _ => none
  else none

/-- Is base action `a` a wire *send* in state `s`?

These are exactly the three arms the mux absorbs into `push`: the two
opener fires on a chosen `wire` obligation and a walk fire on a committed
`.wire i`. Disabling them as base actions is what forces every wire frame
through the strategy gate. -/
def isWireFire (s : State) : Action → Bool
  | .iopenFire => s.iopenCh == some .wire
  | .ropenFire => s.ropenCh == some .wire
  | .walkFire pk =>
      match (s.walk pk).committed with
      | some (.wire _) => true
      | _ => false
  | _ => false

/-- The strengthened wire close-receive guard: no frame of `c` may still
be in flight in the producer's pipe (the strengthened-close repair;
its must-fail control is `noF8_bogus_terminal` in Mux/Controls.lean).

`producerDone` and the empty slot are the base conjuncts; without this
third one a receiver could close under an in-flight frame and reach a
bogus terminal. The must-fail pin lives in Mux/Controls (stage 2). -/
def pipeClear (s : MState) (c : Chan) : Bool :=
  !((s.pipe (wireParty c)).contains c)

/-- Run base action `a` on the wrapped state, recording the observation.

The verbatim lift of the adjudicated arm dispositions: wire sends are disabled
(they are `push`es now), the two wire close-receives carry the F8
conjunct, and the remaining 18 arms — including every wire *receive*,
which reads `base.chan` as the demux slot — delegate to `Model.apply`
unchanged. -/
def applyBase (sk : Skel) (ax : AxMode) (a : Action) (s : MState) :
    Option MState :=
  if isWireFire s.base a then none
  else
    let blocked : Bool :=
      match a with
      | .walkCloseWire pk => !pipeClear s (wireIn pk)
      | .absorbCloseWire => !pipeClear s (Chan.wire .R 0)
      | _ => false
    if blocked then none
    else
      (Model.apply sk ax a s.base).map fun b =>
        { s with base := b
                 hist := recordObs s.hist (actionParty a) (.act a) }

/-- Guarded transition of the muxed system: `none` when the guard fails.

The strategy-driven semantics is this function: `push p` is enabled only
when `p`'s strategy names a held stream (so σ gates pushes and nothing
else), while base actions and deliveries stay scheduler-adversarial. -/
def apply (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy)
    (a : MAction) (s : MState) : Option MState :=
  match a with
  | .base a => applyBase sk ax a s
  | .push p =>
      let σ := match p with | .I => σI | .R => σR
      match σ sk (s.hist p) with
      | some h => firePush sk C p h s
      | none => none
  | .deliver p =>
      match s.pipe p with
      | c :: rest =>
          if s.base.chan c == 0 then
            some { base := { s.base with chan := bump s.base.chan c 1 }
                   pipe := fun q => if q == p then rest else s.pipe q
                   hist := recordObs s.hist p.other
                     (.delivered (wireHeight c)) }
          else none
      | [] => none

-- ======================================================= the step relation

/-- Every mux action that could ever be enabled: the base enumeration
mapped, then the four mux moves. The order is fixed for `mdrain`
determinism; completeness is what the conservativity note (module doc)
rests on. -/
def allMActions (sk : Skel) : List MAction :=
  (Model.allActions sk).map .base ++
    [.push .I, .push .R, .deliver .I, .deliver .R]

/-- The initial muxed state: base init, empty pipes, empty histories. -/
def init (sk : Skel) : MState :=
  ⟨Model.init sk, fun _ => [], fun _ => []⟩

/-- The muxed session is complete: base terminal with both pipes drained.

The pipe conjuncts are redundant given flow conservation (a base-terminal
state has every wire receive fired) but stating them conjunctively avoids
needing that lemma before the definition exists — prove `terminal_drained`
in stage 2 if a proof wants it. -/
def mterminal (sk : Skel) (s : MState) : Bool :=
  Model.terminal sk s.base && (s.pipe .I).isEmpty && (s.pipe .R).isEmpty

/-- Some process, mux, or demux can act. -/
def mcanStep (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy)
    (s : MState) : Bool :=
  (allMActions sk).any fun a => (apply sk ax C σI σR a s).isSome

/-- The muxed deadlock predicate: not finished and nobody can move —
including the muxes, so a strategy idling on the last enabled push leaves
the state stuck (idling is not a move; the M3 posture). -/
def mstuck (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy)
    (s : MState) : Bool :=
  !mterminal sk s && !mcanStep sk ax C σI σR s

/-- Reachability of the muxed system: init plus closure under `apply`,
the verbatim `Model.Reachable` pattern. -/
inductive MReachable (sk : Skel) (ax : AxMode) (C : Nat)
    (σI σR : Strategy) : MState → Prop
  | init : MReachable sk ax C σI σR (init sk)
  | step {s s' : MState} (a : MAction) :
      MReachable sk ax C σI σR s → apply sk ax C σI σR a s = some s' →
      MReachable sk ax C σI σR s'

/-- Deadlock freedom of the muxed composition under a fixed strategy
pair: no reachable state is stuck.

Strategies fix the push choices; the endpoint interleaving — which
process runs, which obligation a walk commits to among ledger-legal
ones — stays fully adversarial. A strategy must survive every local
schedule; a refutation may pick the schedule. -/
def MuxDeadlockFree (sk : Skel) (ax : AxMode) (C : Nat)
    (σI σR : Strategy) : Prop :=
  ∀ s, MReachable sk ax C σI σR s → mstuck sk ax C σI σR s = false

-- ==================================================== the executable spine

/-- Run a list of mux actions from a state, failing on the first disabled
action — the executable spine of the `decide`-checked witnesses, the
verbatim `Model.run` pattern. -/
def mrun (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy)
    (s : MState) : List MAction → Option MState
  | [] => some s
  | a :: rest =>
      match apply sk ax C σI σR a s with
      | some s' => mrun sk ax C σI σR s' rest
      | none => none

/-- A successful `mrun` from init lands on a reachable state: the glue
that turns a kernel-checked replay into a `MReachable` witness. -/
theorem mrun_reachable {sk : Skel} {ax : AxMode} {C : Nat}
    {σI σR : Strategy} {acts : List MAction} {s' : MState}
    (h : mrun sk ax C σI σR (init sk) acts = some s') :
    MReachable sk ax C σI σR s' := by
  suffices general : ∀ (acts : List MAction) (s s' : MState),
      MReachable sk ax C σI σR s → mrun sk ax C σI σR s acts = some s' →
      MReachable sk ax C σI σR s' by
    exact general acts _ _ (.init) h
  intro acts
  induction acts with
  | nil =>
      intro s s' hr hrun
      simp only [mrun, Option.some.injEq] at hrun
      exact hrun ▸ hr
  | cons a rest ih =>
      intro s s' hr hrun
      unfold mrun at hrun
      cases happ : apply sk ax C σI σR a s with
      | none => simp [happ] at hrun
      | some s₁ =>
          exact ih s₁ s' (.step a hr happ) (by simpa [happ] using hrun)

/-- Greedy scheduler for completion pins: take the first enabled mux
action until quiescent — the verbatim `Control.drain` pattern, with the
strategy gate inside `apply` so a σ-driven drain needs no other
plumbing. -/
def mdrain (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy) :
    Nat → MState → MState
  | 0, s => s
  | fuel + 1, s =>
      match (allMActions sk).firstM (fun a => apply sk ax C σI σR a s) with
      | some s' => mdrain sk ax C σI σR fuel s'
      | none => s

/-- `firstM` over `Option` succeeds only through one of its elements. -/
private theorem firstM_eq_some {α β : Type _} {f : α → Option β} {b : β} :
    ∀ {l : List α}, l.firstM f = some b → ∃ a ∈ l, f a = some b := by
  intro l
  induction l with
  | nil => intro h; simp [List.firstM] at h
  | cons x xs ih =>
      intro h
      cases hfx : f x with
      | some b' =>
          simp [List.firstM, hfx] at h
          exact ⟨x, List.mem_cons_self .., by rw [hfx, h]⟩
      | none =>
          simp [List.firstM, hfx] at h
          obtain ⟨a, ha, hfa⟩ := ih h
          exact ⟨a, List.mem_cons_of_mem x ha, hfa⟩

/-- The greedy drain preserves reachability: every step it takes is the
application of some enabled mux action. -/
theorem mdrain_reachable (sk : Skel) (ax : AxMode) (C : Nat)
    (σI σR : Strategy) (fuel : Nat) :
    ∀ {s : MState}, MReachable sk ax C σI σR s →
      MReachable sk ax C σI σR (mdrain sk ax C σI σR fuel s) := by
  induction fuel with
  | zero => intro s h; exact h
  | succ n ih =>
      intro s h
      unfold mdrain
      cases hf : (allMActions sk).firstM
          (fun a => apply sk ax C σI σR a s) with
      | none => exact h
      | some s' =>
          obtain ⟨a, -, ha⟩ := firstM_eq_some hf
          exact ih (.step a h ha)

end StreamingMirror.Mux
