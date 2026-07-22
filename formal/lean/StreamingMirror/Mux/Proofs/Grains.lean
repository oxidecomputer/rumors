/-
The two locality grains' control suite: the kernel pins behind the
incomparability finding of record (Proofs/C1.lean's module doc), the
charter grain's non-triviality controls, and the oracle's nonlocality
at the grain of record — the charter-grain analogs of Oracle/Controls'
T9 suite (`localEq_nondegenerate`, `oracle_not_localStrategy`).

# The finding these pins discharge

The legacy grain (`LocalEq`/`LocalStrategy`, Mux/Strategy.lean) and the
charter grain (`CharterLocal` over announced views, Mux/Causal.lean)
are INCOMPARABLE — the a-fortiori transfer fails in both directions.
Each direction is pinned twice, once at the relation and once at the
class:

- **legacy ⇏ charter.** `legacyEq_announced_differ`: the T9 witness
  pair is `LocalEq`-equal, yet its announced views differ at a shared
  `.impl`-consistent observation (the `leafReqs` difference is
  `viewEnc`-erased but frame-announced). Class form:
  `announcedLeafProbe` is `CharterLocal` by computation and NOT a
  `LocalStrategy` (`announcedLeafProbe_charterLocal`,
  `announcedLeafProbe_not_localStrategy`).
- **charter ⇏ legacy.** `announcedEq_legacy_differ`: two well-formed
  skeletons with equal announced views at a shared `.impl`-consistent
  observation that `LocalEq` separates (the root census is unannounced
  until the opening reply arrives, yet `viewEnc`-visible). Class form:
  `viewProbe` is a `LocalStrategy` by computation and NOT
  `CharterLocal` (`viewProbe_localStrategy`,
  `viewProbe_not_charterLocal`).

# The non-triviality controls

`charterView_nondegenerate` pins two DISTINCT skeletons with equal
announced views at a shared consistent observation — without such a
pair, `CharterLocal` would be vacuously satisfiable and
`c1_charter_false`'s locality hypothesis would say nothing (the
charter-grain analog of `localEq_nondegenerate`).
`oracle_not_charterLocal` pins the C2 oracle outside the class of
record: before the opening reply arrives, the initiator's announced
view is the bare session parameters, and the oracle's second output
already depends on the root census — remote structure no arrived frame
has determined. With `oracle_not_localStrategy` (the legacy grain)
this establishes the oracle's nonlocality at BOTH grains,
independently — neither pin transfers to the other (the
incomparability above is exactly why each grain needs its own).

# The witnesses

`viewPair`/`viewPair'` are Oracle/Controls' T9 pair. `bareRoot` is the
disputeless session: a root scope with no children, well-formed, whose
sessions carry no frames below the opening exchange. `charterTrace` is
the initiator's pre-arrival history — it has opened and pushed, and
nothing has arrived — realized on both skeletons of each pair by the
same driving prefix (`charterActs`), kernel-replayed for the
`ConsistentImpl` certificates.
-/
import StreamingMirror.Mux.Causal
import StreamingMirror.Mux.Proofs.Oracle.Controls

namespace StreamingMirror.Mux

open Model
open Pin (sc)

-- ============================================================ witnesses

/-- The disputeless session: a root with no disputed children — the
smallest well-formed skeleton, and the announced-view-equal partner of
`viewPair` at every pre-arrival observation (nothing below the opening
is announced before the opening reply arrives). -/
def bareRoot : Skel :=
  { scopes := [sc .D 2 []], rootH := 2, fan := 2, capLevel := 1 }

/-- The partner is inside the theorem class: the controls below are not
artifacts of ill-formedness. -/
theorem bareRoot_wellFormed : bareRoot.wellFormed = true := by decide

/-- The initiator's pre-arrival observation: it has chosen and pushed
its opening, and nothing has arrived. Every skeleton with the same
session parameters realizes it (`charterActs` replays on both control
pairs), and at it the announced view is the bare parameters. -/
def charterTrace : List MObs := [.act (.iopenChoose .wire), .pushed 2]

/-- The driving prefix realizing `charterTrace`: open, push. The
initiator acts unilaterally, so the replay is skeleton-generic across
the control pairs. -/
def charterActs : List MAction := [.base (.iopenChoose .wire), .push .I]

-- ============================================ charter nondegeneracy

/-- The charter grain is nondegenerate: two DISTINCT skeletons with
equal announced views on a shared `.impl`-consistent observation —
without this pair `CharterLocal` would be vacuously satisfiable and
the C1 statement of record's locality hypothesis would say nothing
(the charter-grain analog of `localEq_nondegenerate`). The equal-view
observations: the initiator's pre-arrival history, and the responder's
empty history. -/
theorem charterView_nondegenerate :
    viewPair.scopes ≠ bareRoot.scopes
      ∧ aviewOf viewPair .I charterTrace = aviewOf bareRoot .I charterTrace
      ∧ aviewOf viewPair .R [] = aviewOf bareRoot .R []
      ∧ ConsistentImpl .I viewPair charterTrace
      ∧ ConsistentImpl .I bareRoot charterTrace :=
  ⟨by decide, by decide, by decide,
   ConsistentImpl.of_obsOf 1 bottomMostReady bottomMostReady charterActs
     (by decide),
   ConsistentImpl.of_obsOf 1 bottomMostReady bottomMostReady charterActs
     (by decide)⟩

-- ==================================== incomparability, relation tier

/-- Legacy ⇏ charter, at the relation: the T9 pair is `LocalEq`-equal,
yet its announced views differ at a shared `.impl`-consistent
observation — the responder's history after the height-1 arrival,
whose decode announces the `leafReqs` count that `viewEnc` erases.
Equal legacy views carry NO announced-view information. -/
theorem legacyEq_announced_differ :
    LocalEq .R viewPair viewPair' = true
      ∧ aviewOf viewPair .R nonlocalTrace ≠ aviewOf viewPair' .R nonlocalTrace
      ∧ ConsistentImpl .R viewPair nonlocalTrace
      ∧ ConsistentImpl .R viewPair' nonlocalTrace :=
  ⟨by decide, by decide,
   ConsistentImpl.of_obsOf 1 bottomMostReady bottomMostReady nonlocalActs
     (by decide),
   ConsistentImpl.of_obsOf 1 bottomMostReady bottomMostReady nonlocalActs
     (by decide)⟩

/-- Charter ⇏ legacy, at the relation: `viewPair` and `bareRoot` have
equal announced views at the initiator's pre-arrival observation (the
root census is not announced until the opening reply arrives), yet
`LocalEq` separates them (the census is `viewEnc`-visible at session
start). Equal announced views carry NO legacy-view information. The
consistency certificates ride `charterView_nondegenerate`. -/
theorem announcedEq_legacy_differ :
    aviewOf viewPair .I charterTrace = aviewOf bareRoot .I charterTrace
      ∧ LocalEq .I viewPair bareRoot = false := by
  decide

-- ======================================= incomparability, class tier

/-- A strategy reading exactly the announced view: the total announced
`leafReqs` census. A control object (nothing ships it); minted because
it separates the classes — charter-local by computation, not
legacy-local (`announcedLeafProbe_not_localStrategy`). -/
def announcedLeafProbe : Strategy := fun sk tr =>
  some ((aviewOf sk .R tr).recs.foldl (fun a r => a + r.2.leafReqs) 0)

/-- The probe is charter-local, by computation: its one skeleton read
is the announced view, so equal views rewrite. -/
theorem announcedLeafProbe_charterLocal :
    CharterLocal .R announcedLeafProbe := by
  intro sk sk' tr hav _ _
  simp only [announcedLeafProbe, hav]

/-- The probe is NOT legacy-local: it separates the `LocalEq` T9 pair
at the shared consistent trace of `oracle_not_localStrategy` — a
charter-local strategy outside `LocalStrategy`, so charter-grain
membership does not transfer to the legacy grain. -/
theorem announcedLeafProbe_not_localStrategy :
    ¬ LocalStrategy .R announcedLeafProbe := by
  intro hloc
  have h := hloc viewPair viewPair' nonlocalTrace
    (by decide)
    (Consistent.of_obsOf .impl 1 bottomMostReady bottomMostReady
      nonlocalActs (by decide))
    (Consistent.of_obsOf .impl 1 bottomMostReady bottomMostReady
      nonlocalActs (by decide))
  exact absurd h (by decide)

/-- A strategy reading exactly the initiator's session-start view
(`viewEnc`'s token count). A control object; minted because it
separates the classes the other way — legacy-local by computation,
not charter-local (`viewProbe_not_charterLocal`). -/
def viewProbe : Strategy := fun sk _ =>
  some (viewEnc .I sk (sk.rootH + 1) 0).length

/-- The probe is legacy-local, by computation: its skeleton reads are
`rootH` and the `viewEnc` projection, both `LocalEq` conjuncts. -/
theorem viewProbe_localStrategy : LocalStrategy .I viewProbe := by
  intro sk sk' tr hleq _ _
  rw [LocalEq] at hleq
  simp only [Bool.and_eq_true, beq_iff_eq] at hleq
  obtain ⟨⟨⟨hroot, -⟩, -⟩, henc⟩ := hleq
  rw [hroot] at henc
  simp only [viewProbe, hroot, henc]

/-- The probe is NOT charter-local: it separates the equal-announced-
view pair at the shared pre-arrival observation — a legacy-local
strategy outside `CharterLocal`, so legacy-grain membership does not
transfer to the charter grain. With
`announcedLeafProbe_not_localStrategy` this pins the incomparability
finding of record (Proofs/C1.lean's module doc) at class strength in
both directions. -/
theorem viewProbe_not_charterLocal : ¬ CharterLocal .I viewProbe := by
  intro hloc
  have h := hloc viewPair bareRoot charterTrace
    (by decide)
    (ConsistentImpl.of_obsOf 1 bottomMostReady bottomMostReady
      charterActs (by decide))
    (ConsistentImpl.of_obsOf 1 bottomMostReady bottomMostReady
      charterActs (by decide))
  exact absurd h (by decide)

-- ============================== the oracle, at the grain of record

/-- The C2 oracle is NOT charter-local: at the initiator's pre-arrival
observation — announced view = bare parameters — its second output
already depends on the root census (`viewPair`'s send projection has a
stage-1 frame to name; `bareRoot`'s does not), which no arrived frame
has determined. The charter-grain companion of
`oracle_not_localStrategy`: the oracle's nonlocality now stands at
BOTH grains, each with its own witness — neither pin transfers, per
the incomparability above. T6's necessity reading is therefore about
a hypothesis the oracle genuinely fails at the grain of record. -/
theorem oracle_not_charterLocal : ¬ CharterLocal .I (oracle .I) := by
  intro hloc
  have h := hloc viewPair bareRoot charterTrace
    (by decide)
    (ConsistentImpl.of_obsOf 1 bottomMostReady bottomMostReady
      charterActs (by decide))
    (ConsistentImpl.of_obsOf 1 bottomMostReady bottomMostReady
      charterActs (by decide))
  exact absurd h (by decide)

end StreamingMirror.Mux
