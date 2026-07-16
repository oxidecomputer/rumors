// The abstract model of the streaming mirror protocol (see ../MODEL.md).
//
// One parameterized module: the skeleton, fan bound, root height, level
// capacity, and axiom mode are all consts, so every instance and every
// negative control is an instantiation of this single spec.
//
// State design (MODEL.md §5): every process is a finite program over
// channel operations. Sequential parts (prologues, assemblers, terminals)
// are phase counters; the per-scope publication obligations are a
// committed-choice poset — the process nondeterministically commits to its
// next obligation (choice constrained by the active axiom guards, which
// transcribe the ledgers of progress.rs::Trace::assert_valid), then
// must complete that operation before choosing again. Committed choice is
// load-bearing: it models "an arbitrary sequential implementation whose
// trace satisfies assert_valid"; may-fire semantics would let the checker
// dodge every jam by firing the withheld operation.
//
// State layout (Phase B): every state variable holds a primitive, a Set,
// or a map with primitive/Set codomain — one variable per logical record
// field. Apalache's inductive-invariant mode havocs a state from the
// invariant itself, so each variable's first occurrence must be an
// assignment-style binding (`x.in(S)`), and the only set constructors
// that stay symbolic there are intervals, powersets, setOfMaps, and small
// literal sets: record-valued variables cannot be bound at all (a record
// set can only be built via `.map()`, which Apalache expands eagerly).
// The record types survive as read-side views (`wk`, `asmV`), so guards
// read exactly as they did when the state was record-shaped; only the
// update sites are split per field.
module streamingMirror {
  // ========================================================== parameters

  /// Root height; even (Rust: 32). Initiator walk stages consume odd
  /// message indices ROOT_H-1 .. 1, responder even ROOT_H-2 .. 0.
  const ROOT_H: int

  /// Fan bound (Rust FAN = 256): max children per scope. Only used for
  /// well-formedness and the qSent map domain.
  const F: int

  /// Capacity of every AssemblyLevelReturns channel (Rust default: FAN).
  const CAP_LEVEL: int

  /// Axiom W: internal publication consumes a prior wire action
  /// (progress.rs wire ledger).
  const AX_W: bool
  /// Axiom D1 at the root: ROpen's child queries require the root
  /// resolution first (dependent ledger, root scope).
  const AX_D1_ROOT: bool
  /// Axiom D1 at internal levels: a child query requires its scope's
  /// resolution first (dependent ledger).
  const AX_D1_INT: bool
  /// Axiom D2: a parent resolution requires all its D children's
  /// resolutions first (lower ledger).
  const AX_D2: bool

  /// Axiom D3 (sibling contiguity): a child's resolution may be chosen
  /// only once every already-resolved sibling's dependent queries have
  /// been sent. Enforced in Rust by yield_resolve_query!'s per-child
  /// expansion plus the sequential reaction loop, and checked by
  /// Trace::assert_valid's sibling-contiguity rule (added 2026-07-15
  /// after this model surfaced the gap: the original three ledgers
  /// admitted a "all wires, all resolutions, then all queries"
  /// implementation that deadlocks — the ledgerGap instance).
  const AX_D3: bool

  /// Control scaffolding, NOT an axiom: force each child's queries after
  /// its wire send even when D1 is dropped. The wire ledger never
  /// constrains DependentWork, so a bare D1 drop frees queries before the
  /// wire reply and the checker finds that (fan-2-reachable) cycle first;
  /// WIRE_FIRST isolates the resolution-vs-query reorder the N2/N3
  /// predictions are about. Positive instances leave it false (with W and
  /// D1 on it is implied).
  const WIRE_FIRST: bool

  /// The dispute skeleton (MODEL.md §2), flattened. Index 0 is the root.
  /// Ids are BFS order: parent < child, siblings ascending. kind "D" or
  /// "R"; R scopes have no kids. Scopes at height 1 have kids = [] and
  /// carry leafReqs; all others have leafReqs = 0.
  const SKEL: List[{ kind: str, height: int, kids: List[int], leafReqs: int }]

  /// SKEL.length(), duplicated as a const because Apalache's
  /// inductive-invariant mode only picks from constant integer ranges
  /// (it does not fold Len over the inlined skeleton literal). wellFormed
  /// asserts the two agree, and check.sh gates on wellFormed.
  const NSC: int

  // ==================================================== derived structure

  pure def other(p: str): str = if (p == "I") "R" else "I"

  /// Scope ids at height h, in processing order (= id order; BFS).
  /// (Folded with an index accumulator rather than range(0, length):
  /// Apalache requires constant list-range bounds.)
  pure def scopesAt(h: int): List[int] =
    SKEL.foldl((0, []), (acc, sc) =>
      (acc._1 + 1, if (sc.height == h) acc._2.append(acc._1) else acc._2))._2

  /// Walk stage keys: (party, consumed message index).
  pure val walkKeys: Set[(str, int)] =
    range(0, ROOT_H / 2).foldl(Set(), (acc, k) => acc.union(Set(("I", ROOT_H - 1 - 2 * k))))
      .union(range(0, ROOT_H / 2).foldl(Set(), (acc, k) => acc.union(Set(("R", ROOT_H - 2 - 2 * k)))))

  /// Assembler keys: (party, assembled scope height).
  pure val asmKeys: Set[(str, int)] =
    range(1, ROOT_H + 1).foldl(Set(), (acc, j) => acc.union(Set(("I", j))))
      .union(range(1, ROOT_H).foldl(Set(), (acc, j) => acc.union(Set(("R", j)))))

  /// Does party p ask (pair reply with query) for scopes at height j?
  /// Initiator asks even heights, responder odd (MODEL.md §3).
  pure def asks(p: str, j: int): bool =
    if (p == "I") j % 2 == 0 else j % 2 == 1

  // Per-stage scope structure. At h == 0 (leaf_level) the "children" are
  // the scope's leaf requests: wire-send-only, like R children.
  pure def stageScopes(h: int): List[int] = scopesAt(h + 1)
  pure def nChildren(h: int, s: int): int =
    if (h == 0) SKEL[s].leafReqs else SKEL[s].kids.length()
  pure def childIsD(h: int, s: int, i: int): bool =
    if (h == 0 or i < 0 or i >= SKEL[s].kids.length()) false
    else SKEL[SKEL[s].kids[i]].kind == "D"
  /// Queries launched for D child i of scope s at stage h: one per kid of
  /// the child scope, or its leafReqs at height 1.
  pure def qCount(h: int, s: int, i: int): int =
    if (not(childIsD(h, s, i))) 0 else {
      val c = SKEL[s].kids[i]
      if (SKEL[c].height == 1) SKEL[c].leafReqs else SKEL[c].kids.length()
    }
  pure def dCount(s: int): int =
    SKEL[s].kids.select(k => SKEL[k].kind == "D").length()

  pure val totalLeafReqs: int =
    scopesAt(1).select(s => SKEL[s].kind == "D").foldl(0, (acc, s) => acc + SKEL[s].leafReqs)

  // ============================================================= channels
  // ChanId = (kindTag, party, height). Uniform naming (MODEL.md §4):
  //   ("wire",  p, h) : Walk(p,h) yields -> Walk(other p, h-1);
  //                     ("wire", p, ROOT_H) are the openings' messages;
  //                     ("wire", "R", 0) feeds Absorb.
  //   ("asked", p, h) : queries consumed by Walk(p,h); produced by
  //                     Walk(p,h+2) or an opening; ("asked","I",-1) is
  //                     LeafRequests, consumed by Absorb.
  //   ("upper", p, h) : Walk(p,h) epilogue resolutions -> Asm(p,h+1).
  //   ("lower", p, h) : Walk(p,h) D-arm resolutions   -> Asm(p,h).
  //   ("level", p, j) : Asm(p,j) returns -> Asm(p,j+1); ("level","I",0)
  //                     is produced by Absorb.
  //   ("rootret","I",0), ("rootrets","R",0), ("rootres","R",0): the three
  //   cap-1 root singletons (InitiatorRootReturn, ResponderRootReturns,
  //   ResponderRootResolution). ("asked","I",ROOT_H-1) doubles as
  //   InitiatorRootQuery.

  pure val allChans: Set[(str, str, int)] =
    walkKeys.map(pk => ("wire", pk._1, pk._2))
      .union(Set(("wire", "I", ROOT_H), ("wire", "R", ROOT_H)))
      .union(walkKeys.map(pk => ("asked", pk._1, pk._2)))
      .union(Set(("asked", "I", -1)))
      .union(walkKeys.map(pk => ("upper", pk._1, pk._2)))
      .union(walkKeys.map(pk => ("lower", pk._1, pk._2)))
      .union(asmKeys.map(pk => ("level", pk._1, pk._2)))
      .union(Set(("level", "I", 0)))
      .union(Set(("rootret", "I", 0), ("rootrets", "R", 0), ("rootres", "R", 0)))

  pure def cap(c: (str, str, int)): int =
    if (c._1 == "level") CAP_LEVEL else 1

  // Per-walk channel wiring.
  pure def wireIn(pk: (str, int)): (str, str, int) = ("wire", other(pk._1), pk._2 + 1)
  pure def askedIn(pk: (str, int)): (str, str, int) = ("asked", pk._1, pk._2)
  pure def wireOut(pk: (str, int)): (str, str, int) = ("wire", pk._1, pk._2)
  pure def lowerOut(pk: (str, int)): (str, str, int) = ("lower", pk._1, pk._2)
  pure def upperOut(pk: (str, int)): (str, str, int) = ("upper", pk._1, pk._2)
  // Note: askedOut(("R", 0)) names a channel outside allChans; it is
  // unreachable because childIsD is hard-false at h = 0 (leaf_level's
  // children are wire-only leaf requests). A future height-0 D-kind would
  // fail loudly on the missing map key rather than silently miswire.
  pure def askedOut(pk: (str, int)): (str, str, int) = ("asked", pk._1, pk._2 - 2)

  // Per-assembler wiring (MODEL.md §4): the asker side's resolutions come
  // up through "upper" from the stage below; the answerer side's through
  // "lower" at the same height.
  pure def asmResChan(pk: (str, int)): (str, str, int) =
    if (asks(pk._1, pk._2)) ("upper", pk._1, pk._2 - 1) else ("lower", pk._1, pk._2)
  pure def asmLevelChan(pk: (str, int)): (str, str, int) = ("level", pk._1, pk._2 - 1)
  pure def asmOutChan(pk: (str, int)): (str, str, int) =
    if (pk._1 == "I" and pk._2 == ROOT_H) ("rootret", "I", 0)
    else if (pk._1 == "R" and pk._2 == ROOT_H - 1) ("rootrets", "R", 0)
    else ("level", pk._1, pk._2)

  /// Pending counts of the resolutions Asm(p,j) consumes, in arrival
  /// order. Asker side: one resolution per scope at j, pending = #D kids
  /// (R kids are Ready for the asker). Answerer side: one per D scope at
  /// j, pending = #kids (D and R both recurse for the answerer), or
  /// leafReqs at height 1.
  pure def asmResList(pk: (str, int)): List[int] =
    if (asks(pk._1, pk._2))
      scopesAt(pk._2).foldl([], (acc, s) => acc.append(dCount(s)))
    else
      scopesAt(pk._2).select(s => SKEL[s].kind == "D").foldl([], (acc, s) =>
        acc.append(if (SKEL[s].height == 1) SKEL[s].leafReqs else SKEL[s].kids.length()))

  /// The responder-side root resolution's pending count (RFinish).
  pure val rootPending: int = SKEL[0].kids.length()
  /// The initiator-side root pending (Asm(I, ROOT_H)); derived, unused
  /// directly — kept for documentation symmetry.
  pure val rootPendingAsker: int = dCount(0)

  // ================================================================ state
  // One variable per logical field (see the module doc's State layout
  // note); `wk`/`asmV` rebuild the records for the read side.

  type WalkSt = {
    scope: int,          // index into stageScopes
    phase: int,          // 0 recvWire, 1 recvAsked, 2 publishing, 3 closeWire, 4 closeAsked, 5 done
    wireDone: Set[int],  // child positions of the current scope
    resDone: Set[int],
    qSent: int -> int,   // child position -> queries already sent
    parentDone: bool,
    chKind: str,         // committed obligation: "none"|"wire"|"res"|"query"|"parent"
    chChild: int
  }

  type AsmSt = {
    idx: int,            // resolution index
    phase: int,          // 0 recvRes, 1 recvLevels, 2 send, 3 closeRes, 4 done
    got: int
  }

  var wkScope: (str, int) -> int
  var wkPhase: (str, int) -> int
  var wkWire: (str, int) -> Set[int]
  var wkRes: (str, int) -> Set[int]
  var wkQ: (str, int) -> (int -> int)
  var wkParent: (str, int) -> bool
  var wkChKind: (str, int) -> str
  var wkChChild: (str, int) -> int

  var asmIdx: (str, int) -> int
  var asmPhase: (str, int) -> int
  var asmGot: (str, int) -> int

  var chan: (str, str, int) -> int

  var iopenWire: bool      // opening message yielded
  var iopenQuery: bool     // root query sent
  var iopenCh: str         // committed obligation: "none"|"wire"|"query"

  var ropenGotWire: bool   // opening message received
  var ropenWire: bool      // wire reply yielded
  var ropenRes: bool       // root resolution sent
  var ropenQ: int          // root child queries sent
  var ropenCh: str         // committed obligation: "none"|"wire"|"res"|"query"

  var absorbIdx: int
  var absorbPhase: int     // phases as WalkSt 0/1/2 then 3 closeWire, 4 closeAsked, 5 done

  var ifin: bool

  var rfinGotRes: bool
  var rfinGot: int

  /// The walk record at pk, rebuilt from the per-field variables.
  def wk(pk: (str, int)): WalkSt = {
    scope: wkScope.get(pk), phase: wkPhase.get(pk),
    wireDone: wkWire.get(pk), resDone: wkRes.get(pk), qSent: wkQ.get(pk),
    parentDone: wkParent.get(pk), chKind: wkChKind.get(pk), chChild: wkChChild.get(pk)
  }

  /// The assembler record at pk, rebuilt from the per-field variables.
  def asmV(pk: (str, int)): AsmSt =
    { idx: asmIdx.get(pk), phase: asmPhase.get(pk), got: asmGot.get(pk) }

  // Per-family frame helpers: writing one record back into its field
  // variables, and leaving a family unchanged. Every action assigns every
  // variable exactly once via these.

  action wkSet(pk: (str, int), ws: WalkSt): bool = all {
    wkScope' = wkScope.set(pk, ws.scope),
    wkPhase' = wkPhase.set(pk, ws.phase),
    wkWire' = wkWire.set(pk, ws.wireDone),
    wkRes' = wkRes.set(pk, ws.resDone),
    wkQ' = wkQ.set(pk, ws.qSent),
    wkParent' = wkParent.set(pk, ws.parentDone),
    wkChKind' = wkChKind.set(pk, ws.chKind),
    wkChChild' = wkChChild.set(pk, ws.chChild)
  }

  action wkSame: bool = all {
    wkScope' = wkScope, wkPhase' = wkPhase, wkWire' = wkWire, wkRes' = wkRes,
    wkQ' = wkQ, wkParent' = wkParent, wkChKind' = wkChKind, wkChChild' = wkChChild
  }

  action asmSet(pk: (str, int), a: AsmSt): bool = all {
    asmIdx' = asmIdx.set(pk, a.idx),
    asmPhase' = asmPhase.set(pk, a.phase),
    asmGot' = asmGot.set(pk, a.got)
  }

  action asmSame: bool = all {
    asmIdx' = asmIdx, asmPhase' = asmPhase, asmGot' = asmGot
  }

  action iopenSet(w: bool, q: bool, c: str): bool = all {
    iopenWire' = w, iopenQuery' = q, iopenCh' = c
  }

  action iopenSame: bool = iopenSet(iopenWire, iopenQuery, iopenCh)

  action ropenSet(g: bool, w: bool, r: bool, q: int, c: str): bool = all {
    ropenGotWire' = g, ropenWire' = w, ropenRes' = r, ropenQ' = q, ropenCh' = c
  }

  action ropenSame: bool = ropenSet(ropenGotWire, ropenWire, ropenRes, ropenQ, ropenCh)

  action absorbSet(i: int, p: int): bool = all {
    absorbIdx' = i, absorbPhase' = p
  }

  action absorbSame: bool = absorbSet(absorbIdx, absorbPhase)

  action finSet(i: bool, gr: bool, g: int): bool = all {
    ifin' = i, rfinGotRes' = gr, rfinGot' = g
  }

  action finSame: bool = finSet(ifin, rfinGotRes, rfinGot)

  // ======================================================= process status

  pure val emptyQ: int -> int = range(0, F).foldl(Map(), (m, i) => m.put(i, 0))

  /// A fresh walk state at scope index k of stage h (skips the publishing
  /// machinery reset).
  pure def freshWalk(h: int, k: int): WalkSt = {
    scope: k,
    phase: if (k < stageScopes(h).length()) 0 else 3,
    wireDone: Set(), resDone: Set(), qSent: emptyQ, parentDone: false,
    chKind: "none", chChild: -1
  }

  pure def scopeComplete(h: int, ws: WalkSt): bool =
    if (ws.scope >= stageScopes(h).length()) true else {
    val s = stageScopes(h)[ws.scope]
    ws.parentDone and
    0.to(F - 1).forall(i =>
      i >= nChildren(h, s) or (
      ws.wireDone.contains(i) and
      (not(childIsD(h, s, i)) or (ws.resDone.contains(i) and ws.qSent.get(i) == qCount(h, s, i)))))
    }

  /// Advance past completed scopes after an obligation fires.
  pure def normWalk(h: int, ws: WalkSt): WalkSt =
    if (ws.phase == 2 and scopeComplete(h, ws)) freshWalk(h, ws.scope + 1) else ws

  pure def doneWalk(ws: WalkSt): bool = ws.phase == 5
  pure def doneAsm(a: AsmSt): bool = a.phase == 4
  def doneIOpen: bool = iopenWire and iopenQuery
  def doneROpen: bool =
    ropenGotWire and ropenWire and ropenRes and ropenQ == SKEL[0].kids.length()

  /// Is the producer of channel c finished (its senders dropped, so a
  /// consumer's recv-on-empty takes the end branch)? MODEL.md §5 recvClose.
  def producerDone(c: (str, str, int)): bool =
    if (c._1 == "wire" and c._3 == ROOT_H)
      (if (c._2 == "I") doneIOpen else doneROpen)
    else if (c._1 == "wire") doneWalk(wk((c._2, c._3)))
    else if (c._1 == "asked" and c._3 == ROOT_H - 1) doneIOpen
    else if (c._1 == "asked" and c._3 == ROOT_H - 2 and c._2 == "R") doneROpen
    else if (c._1 == "asked") doneWalk(wk((c._2, c._3 + 2)))
    else if (c._1 == "upper" or c._1 == "lower") doneWalk(wk((c._2, c._3)))
    else false // level/root channels: never close-recv'd

  // ================================================================= init

  action init = all {
    wellFormed,
    wkScope' = walkKeys.mapBy(pk => 0),
    wkPhase' = walkKeys.mapBy(pk => freshWalk(pk._2, 0).phase),
    wkWire' = walkKeys.mapBy(pk => Set()),
    wkRes' = walkKeys.mapBy(pk => Set()),
    wkQ' = walkKeys.mapBy(pk => emptyQ),
    wkParent' = walkKeys.mapBy(pk => false),
    wkChKind' = walkKeys.mapBy(pk => "none"),
    wkChChild' = walkKeys.mapBy(pk => -1),
    asmIdx' = asmKeys.mapBy(pk => 0),
    asmPhase' = asmKeys.mapBy(pk => if (asmResList(pk).length() > 0) 0 else 3),
    asmGot' = asmKeys.mapBy(pk => 0),
    chan' = allChans.mapBy(c => 0),
    iopenWire' = false, iopenQuery' = false, iopenCh' = "none",
    ropenGotWire' = false, ropenWire' = false, ropenRes' = false, ropenQ' = 0, ropenCh' = "none",
    absorbIdx' = 0, absorbPhase' = if (totalLeafReqs > 0) 0 else 3,
    ifin' = false, rfinGotRes' = false, rfinGot' = 0
  }

  // ====================================================== IOpen / ROpen
  // IOpen: yield the opening message, send the root query. ROpen: receive
  // the opening, then publish the root's obligations (wire reply, root
  // resolution, one query per root child) under the axiom guards.

  def iopenChoosable(k: str): bool =
    if (k == "wire") not(iopenWire)
    else if (k == "query") not(iopenQuery) and (not(AX_W) or iopenWire)
    else false

  action iopenChoose = all {
    iopenCh == "none",
    any {
      all { iopenChoosable("wire"), iopenSet(iopenWire, iopenQuery, "wire") },
      all { iopenChoosable("query"), iopenSet(iopenWire, iopenQuery, "query") }
    },
    wkSame, asmSame, chan' = chan, ropenSame, absorbSame, finSame
  }

  action iopenFire = all {
    iopenCh != "none",
    any {
      all {
        iopenCh == "wire",
        chan.get(("wire", "I", ROOT_H)) < 1,
        chan' = chan.set(("wire", "I", ROOT_H), chan.get(("wire", "I", ROOT_H)) + 1),
        iopenSet(true, iopenQuery, "none")
      },
      all {
        iopenCh == "query",
        chan.get(("asked", "I", ROOT_H - 1)) < 1,
        chan' = chan.set(("asked", "I", ROOT_H - 1), chan.get(("asked", "I", ROOT_H - 1)) + 1),
        iopenSet(iopenWire, true, "none")
      }
    },
    wkSame, asmSame, ropenSame, absorbSame, finSame
  }

  action ropenRecv = all {
    not(ropenGotWire),
    chan.get(("wire", "I", ROOT_H)) > 0,
    chan' = chan.set(("wire", "I", ROOT_H), chan.get(("wire", "I", ROOT_H)) - 1),
    ropenSet(true, ropenWire, ropenRes, ropenQ, ropenCh),
    wkSame, asmSame, iopenSame, absorbSame, finSame
  }

  def ropenChoosable(k: str): bool =
    ropenGotWire and (
      if (k == "wire") not(ropenWire)
      else if (k == "res") not(ropenRes) and (not(AX_W) or ropenWire)
      else if (k == "query")
        ropenQ < SKEL[0].kids.length() and (not(AX_D1_ROOT) or ropenRes) and
        (not(WIRE_FIRST) or ropenWire)
      else false)

  action ropenChoose = all {
    ropenCh == "none",
    nondet k = oneOf(Set("wire", "res", "query"))
    all {
      ropenChoosable(k),
      ropenSet(ropenGotWire, ropenWire, ropenRes, ropenQ, k)
    },
    wkSame, asmSame, chan' = chan, iopenSame, absorbSame, finSame
  }

  action ropenFire = all {
    ropenCh != "none",
    any {
      all {
        ropenCh == "wire",
        chan.get(("wire", "R", ROOT_H)) < 1,
        chan' = chan.set(("wire", "R", ROOT_H), chan.get(("wire", "R", ROOT_H)) + 1),
        ropenSet(true, true, ropenRes, ropenQ, "none")
      },
      all {
        ropenCh == "res",
        chan.get(("rootres", "R", 0)) < 1,
        chan' = chan.set(("rootres", "R", 0), chan.get(("rootres", "R", 0)) + 1),
        ropenSet(true, ropenWire, true, ropenQ, "none")
      },
      all {
        ropenCh == "query",
        chan.get(("asked", "R", ROOT_H - 2)) < 1,
        chan' = chan.set(("asked", "R", ROOT_H - 2), chan.get(("asked", "R", ROOT_H - 2)) + 1),
        ropenSet(true, ropenWire, ropenRes, ropenQ + 1, "none")
      }
    },
    wkSame, asmSame, iopenSame, absorbSame, finSame
  }

  // ================================================================ walks

  def wkChoosable(pk: (str, int), k: str, i: int): bool = {
    val ws = wk(pk)
    val h = pk._2
    if (ws.phase != 2) false else {
      val s = stageScopes(h)[ws.scope]
      // Program structure, never relaxed: within a scope, each channel's
      // sends fire in child order. Positional pairing is the protocol's
      // identity carrier (work.rs:512-515); an implementation that
      // reorders within a channel is functionally incorrect and outside
      // the theorem's scope, and the counting abstraction is unsound for
      // it (MODEL.md §5). Checked in Rust by Trace::assert_valid's
      // radix-order rule.
      if (k == "wire")
        i >= 0 and i < nChildren(h, s) and not(ws.wireDone.contains(i)) and
        0.to(F - 1).forall(j => j >= i or ws.wireDone.contains(j))
      else if (k == "res")
        i >= 0 and i < nChildren(h, s) and childIsD(h, s, i) and not(ws.resDone.contains(i)) and
        0.to(F - 1).forall(j => j >= i or not(childIsD(h, s, j)) or ws.resDone.contains(j)) and
        (not(AX_W) or ws.wireDone.contains(i)) and
        (not(AX_D3) or 0.to(F - 1).forall(j =>
          j >= nChildren(h, s) or not(ws.resDone.contains(j)) or ws.qSent.get(j) == qCount(h, s, j)))
      else if (k == "query")
        i >= 0 and i < nChildren(h, s) and childIsD(h, s, i) and ws.qSent.get(i) < qCount(h, s, i) and
        0.to(F - 1).forall(j => j >= i or ws.qSent.get(j) == qCount(h, s, j)) and
        (not(AX_D1_INT) or ws.resDone.contains(i)) and
        (not(WIRE_FIRST) or ws.wireDone.contains(i))
      else if (k == "parent")
        not(ws.parentDone) and
        (not(AX_D2) or 0.to(F - 1).forall(j =>
          j >= nChildren(h, s) or not(childIsD(h, s, j)) or ws.resDone.contains(j)))
      else false
    }
  }

  /// One committed obligation of walk pk fires into its channel.
  def wkFireGuard(pk: (str, int)): bool = {
    val ws = wk(pk)
    if (ws.chKind == "none") false
    else if (ws.chKind == "wire") chan.get(wireOut(pk)) < 1
    else if (ws.chKind == "res") chan.get(lowerOut(pk)) < 1
    else if (ws.chKind == "query") chan.get(askedOut(pk)) < 1
    else chan.get(upperOut(pk)) < 1
  }

  action walkStep(pk: (str, int)): bool = {
    val ws = wk(pk)
    val h = pk._2
    all {
    asmSame, iopenSame, ropenSame, absorbSame, finSame,
    any {
      // prologue: recv the k-th wire reply, then the k-th queued query
      all {
        ws.phase == 0,
        chan.get(wireIn(pk)) > 0,
        chan' = chan.set(wireIn(pk), chan.get(wireIn(pk)) - 1),
        wkSet(pk, { scope: ws.scope, phase: 1, wireDone: ws.wireDone, resDone: ws.resDone, qSent: ws.qSent, parentDone: ws.parentDone, chKind: "none", chChild: -1 })
      },
      all {
        ws.phase == 1,
        chan.get(askedIn(pk)) > 0,
        chan' = chan.set(askedIn(pk), chan.get(askedIn(pk)) - 1),
        wkSet(pk, normWalk(h, { scope: ws.scope, phase: 2, wireDone: ws.wireDone, resDone: ws.resDone, qSent: ws.qSent, parentDone: ws.parentDone, chKind: "none", chChild: -1 }))
      },
      // publishing: commit to the next obligation (axiom-guarded choice)
      all {
        ws.phase == 2, ws.chKind == "none",
        nondet k = oneOf(Set("wire", "res", "query", "parent"))
        nondet i = oneOf((-1).to(F - 1))
        all {
          wkChoosable(pk, k, i),
          if (k == "parent") i == -1 else true,
          wkSet(pk, { scope: ws.scope, phase: 2, wireDone: ws.wireDone, resDone: ws.resDone, qSent: ws.qSent, parentDone: ws.parentDone, chKind: k, chChild: i }),
          chan' = chan
        }
      },
      // publishing: fire the committed obligation
      all {
        ws.phase == 2, ws.chKind != "none",
        wkFireGuard(pk),
        val ch = if (ws.chKind == "wire") wireOut(pk)
                 else if (ws.chKind == "res") lowerOut(pk)
                 else if (ws.chKind == "query") askedOut(pk)
                 else upperOut(pk)
        val ws2 = {
          scope: ws.scope, phase: 2,
          wireDone: if (ws.chKind == "wire") ws.wireDone.union(Set(ws.chChild)) else ws.wireDone,
          resDone: if (ws.chKind == "res") ws.resDone.union(Set(ws.chChild)) else ws.resDone,
          qSent: if (ws.chKind == "query") ws.qSent.set(ws.chChild, ws.qSent.get(ws.chChild) + 1) else ws.qSent,
          parentDone: if (ws.chKind == "parent") true else ws.parentDone,
          chKind: "none", chChild: -1
        }
        all {
          chan' = chan.set(ch, chan.get(ch) + 1),
          wkSet(pk, normWalk(h, ws2))
        }
      },
      // end-of-stream: producer done and channel drained (MODEL.md §5.4)
      all {
        ws.phase == 3,
        producerDone(wireIn(pk)),
        chan.get(wireIn(pk)) == 0,
        chan' = chan,
        wkSet(pk, { scope: ws.scope, phase: 4, wireDone: ws.wireDone, resDone: ws.resDone, qSent: ws.qSent, parentDone: ws.parentDone, chKind: "none", chChild: -1 })
      },
      all {
        ws.phase == 4,
        producerDone(askedIn(pk)),
        chan.get(askedIn(pk)) == 0,
        chan' = chan,
        wkSet(pk, { scope: ws.scope, phase: 5, wireDone: ws.wireDone, resDone: ws.resDone, qSent: ws.qSent, parentDone: ws.parentDone, chKind: "none", chChild: -1 })
      }
    }
    }
  }

  def wkCanStep(pk: (str, int)): bool = {
    val ws = wk(pk)
    val h = pk._2
    if (ws.phase == 0) chan.get(wireIn(pk)) > 0
    else if (ws.phase == 1) chan.get(askedIn(pk)) > 0
    else if (ws.phase == 2 and ws.chKind == "none")
      Set("wire", "res", "query", "parent").exists(k =>
        (-1).to(F - 1).exists(i =>
          wkChoosable(pk, k, i) and (if (k == "parent") i == -1 else true)))
    else if (ws.phase == 2) wkFireGuard(pk)
    else if (ws.phase == 3) producerDone(wireIn(pk)) and chan.get(wireIn(pk)) == 0
    else if (ws.phase == 4) producerDone(askedIn(pk)) and chan.get(askedIn(pk)) == 0
    else false
  }

  // =========================================================== assemblers
  // Sequential: recv resolution, recv one level item per Pending slot,
  // send the assembled return upward; then observe the resolution
  // channel's close. (assemble + return_into collapse to one sequential
  // process: the stream only runs while polled — MODEL.md §4.)

  /// Pending count of resolution i for Asm pk; total (0 past the end).
  pure def pendAt(pk: (str, int), i: int): int = {
    val l = asmResList(pk)
    if (i < l.length()) l[i] else 0
  }

  action asmStep(pk: (str, int)): bool = {
    val a = asmV(pk)
    val resList = asmResList(pk)
    all {
    wkSame, iopenSame, ropenSame, absorbSame, finSame,
    any {
      all {
        a.phase == 0,
        chan.get(asmResChan(pk)) > 0,
        chan' = chan.set(asmResChan(pk), chan.get(asmResChan(pk)) - 1),
        asmSet(pk, { idx: a.idx, phase: if (pendAt(pk, a.idx) > 0) 1 else 2, got: 0 })
      },
      all {
        a.phase == 1,
        chan.get(asmLevelChan(pk)) > 0,
        chan' = chan.set(asmLevelChan(pk), chan.get(asmLevelChan(pk)) - 1),
        asmSet(pk, { idx: a.idx, phase: if (a.got + 1 == pendAt(pk, a.idx)) 2 else 1, got: a.got + 1 })
      },
      all {
        a.phase == 2,
        chan.get(asmOutChan(pk)) < cap(asmOutChan(pk)),
        chan' = chan.set(asmOutChan(pk), chan.get(asmOutChan(pk)) + 1),
        asmSet(pk, { idx: a.idx + 1, phase: if (a.idx + 1 < resList.length()) 0 else 3, got: 0 })
      },
      all {
        a.phase == 3,
        producerDone(asmResChan(pk)),
        chan.get(asmResChan(pk)) == 0,
        chan' = chan,
        asmSet(pk, { idx: a.idx, phase: 4, got: 0 })
      }
    }
    }
  }

  def asmCanStep(pk: (str, int)): bool = {
    val a = asmV(pk)
    if (a.phase == 0) chan.get(asmResChan(pk)) > 0
    else if (a.phase == 1) chan.get(asmLevelChan(pk)) > 0
    else if (a.phase == 2) chan.get(asmOutChan(pk)) < cap(asmOutChan(pk))
    else if (a.phase == 3) producerDone(asmResChan(pk)) and chan.get(asmResChan(pk)) == 0
    else false
  }

  // =============================================== absorb and the finishes

  action absorbStep = all {
    any {
      all {
        absorbPhase == 0,
        chan.get(("wire", "R", 0)) > 0,
        chan' = chan.set(("wire", "R", 0), chan.get(("wire", "R", 0)) - 1),
        absorbSet(absorbIdx, 1)
      },
      all {
        absorbPhase == 1,
        chan.get(("asked", "I", -1)) > 0,
        chan' = chan.set(("asked", "I", -1), chan.get(("asked", "I", -1)) - 1),
        absorbSet(absorbIdx, 2)
      },
      all {
        absorbPhase == 2,
        chan.get(("level", "I", 0)) < cap(("level", "I", 0)),
        chan' = chan.set(("level", "I", 0), chan.get(("level", "I", 0)) + 1),
        absorbSet(absorbIdx + 1, if (absorbIdx + 1 < totalLeafReqs) 0 else 3)
      },
      all {
        absorbPhase == 3,
        producerDone(("wire", "R", 0)),
        chan.get(("wire", "R", 0)) == 0,
        chan' = chan,
        absorbSet(absorbIdx, 4)
      },
      all {
        absorbPhase == 4,
        producerDone(("asked", "I", -1)),
        chan.get(("asked", "I", -1)) == 0,
        chan' = chan,
        absorbSet(absorbIdx, 5)
      }
    },
    wkSame, asmSame, iopenSame, ropenSame, finSame
  }

  def absorbCanStep: bool =
    if (absorbPhase == 0) chan.get(("wire", "R", 0)) > 0
    else if (absorbPhase == 1) chan.get(("asked", "I", -1)) > 0
    else if (absorbPhase == 2) chan.get(("level", "I", 0)) < cap(("level", "I", 0))
    else if (absorbPhase == 3) producerDone(("wire", "R", 0)) and chan.get(("wire", "R", 0)) == 0
    else if (absorbPhase == 4) producerDone(("asked", "I", -1)) and chan.get(("asked", "I", -1)) == 0
    else false

  action finStep = all {
    any {
      all {
        not(ifin),
        chan.get(("rootret", "I", 0)) > 0,
        chan' = chan.set(("rootret", "I", 0), chan.get(("rootret", "I", 0)) - 1),
        finSet(true, rfinGotRes, rfinGot)
      },
      all {
        not(rfinGotRes),
        chan.get(("rootres", "R", 0)) > 0,
        chan' = chan.set(("rootres", "R", 0), chan.get(("rootres", "R", 0)) - 1),
        finSet(ifin, true, rfinGot)
      },
      all {
        rfinGotRes, rfinGot < rootPending,
        chan.get(("rootrets", "R", 0)) > 0,
        chan' = chan.set(("rootrets", "R", 0), chan.get(("rootrets", "R", 0)) - 1),
        finSet(ifin, rfinGotRes, rfinGot + 1)
      }
    },
    wkSame, asmSame, iopenSame, ropenSame, absorbSame
  }

  def finCanStep: bool =
    (not(ifin) and chan.get(("rootret", "I", 0)) > 0) or
    (not(rfinGotRes) and chan.get(("rootres", "R", 0)) > 0) or
    (rfinGotRes and rfinGot < rootPending and chan.get(("rootrets", "R", 0)) > 0)

  // =========================================================== the system

  val terminal: bool =
    walkKeys.forall(pk => doneWalk(wk(pk))) and
    asmKeys.forall(pk => doneAsm(asmV(pk))) and
    doneIOpen and doneROpen and
    absorbPhase == 5 and ifin and rfinGotRes and rfinGot == rootPending

  def iopenCanStep: bool =
    if (iopenCh == "none") iopenChoosable("wire") or iopenChoosable("query")
    else if (iopenCh == "wire") chan.get(("wire", "I", ROOT_H)) < 1
    else chan.get(("asked", "I", ROOT_H - 1)) < 1

  def ropenCanStep: bool =
    (not(ropenGotWire) and chan.get(("wire", "I", ROOT_H)) > 0) or
    (ropenCh == "none" and Set("wire", "res", "query").exists(k => ropenChoosable(k))) or
    (ropenCh == "wire" and chan.get(("wire", "R", ROOT_H)) < 1) or
    (ropenCh == "res" and chan.get(("rootres", "R", 0)) < 1) or
    (ropenCh == "query" and chan.get(("asked", "R", ROOT_H - 2)) < 1)

  val canStep: bool =
    iopenCanStep or ropenCanStep or
    walkKeys.exists(pk => wkCanStep(pk)) or
    asmKeys.exists(pk => asmCanStep(pk)) or
    absorbCanStep or finCanStep

  /// The deadlock predicate: the model twin of Quiescence::Stalled.
  val stuck: bool = not(terminal) and not(canStep)

  /// The safety theorem's invariant.
  val safe: bool = not(stuck)

  /// Occupancy sanity: no channel over capacity (encoding self-check).
  val occupancyOk: bool = allChans.forall(c => chan.get(c) >= 0 and chan.get(c) <= cap(c))

  /// Conservation: terminal states must have drained every channel.
  ///
  /// The level and root channels have no close observation, so without
  /// this a miscounted edge could silently "complete" while its Rust twin
  /// hangs (review finding, lane 1).
  val drainedAtTerminal: bool = not(terminal) or allChans.forall(c => chan.get(c) == 0)

  /// What check.sh verifies on every positive instance.
  val phaseAInvariant: bool = safe and occupancyOk and drainedAtTerminal

  /// Skeleton well-formedness (MODEL.md §2). init refuses malformed
  /// instances; check.sh queries this val so the refusal is loud rather
  /// than vacuous. Everything the counting abstraction assumes is here:
  /// BFS ids, height decrements, R childless, leaf requests only on
  /// height-1 D scopes, fan and leafReqs within F.
  pure val wellFormed: bool = {
    val n = SKEL.length()
    val perScope = SKEL.foldl((0, true), (acc, sc) => {
      val i = acc._1
      val kidsOk = sc.kids.foldl((i, true), (ka, k) =>
        (k, ka._2 and k > ka._1 and k < n and SKEL[k].height == sc.height - 1))._2
      (i + 1, acc._2 and
        (sc.kind == "D" or sc.kind == "R") and
        sc.height >= 1 and
        sc.kids.length() <= F and
        sc.leafReqs >= 0 and sc.leafReqs <= F and
        (sc.kind == "D" or (sc.kids.length() == 0 and sc.leafReqs == 0)) and
        (sc.height != 1 or sc.kids.length() == 0) and
        (sc.leafReqs == 0 or (sc.height == 1 and sc.kind == "D")) and
        kidsOk)
    })._2
    val kidCount = SKEL.foldl(0, (acc, sc) => acc + sc.kids.length())
    val kidSet = SKEL.foldl(Set(), (acc, sc) =>
      acc.union(sc.kids.foldl(Set(), (s2, k) => s2.union(Set(k)))))
    n > 0 and SKEL[0].height == ROOT_H and SKEL[0].kind == "D" and
    ROOT_H % 2 == 0 and perScope and
    kidCount == n - 1 and kidSet.size() == n - 1 and not(kidSet.contains(0)) and
    NSC == n and CAP_LEVEL >= 1
  }

  action stutter = all {
    terminal,
    wkSame, asmSame, chan' = chan, iopenSame, ropenSame, absorbSame, finSame
  }

  // Per-family step actions: step is their union. The split lets the
  // Phase B consecution obligation (indInv and family-step implies
  // indInv') be checked one family at a time — much smaller SMT problems,
  // run in parallel — while `step` remains the single system relation.
  action stepIOpenF = any { iopenChoose, iopenFire }
  action stepROpenF = any { ropenRecv, ropenChoose, ropenFire }
  action stepWalkF = { nondet pk = oneOf(walkKeys) walkStep(pk) }
  action stepAsmF = { nondet pk = oneOf(asmKeys) asmStep(pk) }

  action step = any {
    stepIOpenF, stepROpenF, stepWalkF, stepAsmF,
    absorbStep, finStep, stutter
  }

  // ============================================= run-length bound (ρ + commits)
  // Every action except stutter either fires a channel op, advances a
  // phase, or commits an obligation — all drawn from finite skeleton-
  // derived budgets, so runs are bounded and BMC to this depth is
  // exhaustive (MODEL.md §7).

  pure def walkOblig(h: int): int =
    stageScopes(h).foldl(0, (acc, s) =>
      acc + 2 // prologue recvs
        + 0.to(F - 1).fold(0, (a2, i) =>
            a2 + (if (i >= nChildren(h, s)) 0
                  else 1 + (if (childIsD(h, s, i)) 1 + qCount(h, s, i) else 0)))
        + 1) // parent resolution
      + 2 // closes

  pure val totalSteps: int =
    // walks: 2 * walkOblig doubles everything the walk does, including
    // its prologue recvs and closes, which only commit-free obligations
    // strictly need — a deliberate over-approximation (recvs and closes
    // are single steps), so this stays an upper bound; do not "tighten"
    // it below the true count of commit + fire + recv + close steps.
    walkKeys.fold(0, (acc, pk) => acc + 2 * walkOblig(pk._2))
    + asmKeys.fold(0, (acc, pk) =>
        acc + asmResList(pk).foldl(0, (a2, p) => a2 + p + 2) + 1)
    + 3 * totalLeafReqs + 2 // absorb
    + 2 * 2 + 2 * (2 + SKEL[0].kids.length()) + 1 // openings (commit+fire), ropen recv
    + 2 + rootPending // finishes

  // ================================== inductive invariant (Phase B)
  // IndInv = indTypeOk and occupancyOk and indLocal and indFlow. Checked
  // by `quint verify --inductive-invariant indInv --invariant
  // phaseAInvariant`: (1) init satisfies indInv, (2) indInv is closed
  // under step, (3) indInv implies the safety property. Apalache havocs a
  // state from indInv itself, so indTypeOk must bind every variable
  // assignment-style (`x.in(S)`) before anything else mentions it, and
  // only interval / powerset / setOfMaps / literal-set domains stay
  // symbolic (a record set can only be built by `.map()`, which Apalache
  // expands eagerly — hence the per-field state layout above).

  pure def b2i(b: bool): int = if (b) 1 else 0

  /// stageScopes(h)[k], total (root as dummy past the end): IndInv is
  /// evaluated on havoc'd states, where scope may sit at the stage end.
  pure def scAt(h: int, k: int): int =
    if (k >= 0 and k < stageScopes(h).length()) stageScopes(h)[k] else 0

  pure def stageLen(h: int): int = stageScopes(h).length()

  /// D children of scope s as counted at stage h (leaf stage has none).
  pure def dOf(h: int, s: int): int = if (h == 0) 0 else dCount(s)

  /// Queries scope s owes at stage h, all children.
  pure def qOf(h: int, s: int): int =
    0.to(F - 1).fold(0, (acc, i) => acc + qCount(h, s, i))

  // Prefix sums over a stage's first k scopes: cumulative sends of a walk
  // that has completed k scopes.
  pure def wiresBefore(h: int, k: int): int =
    stageScopes(h).foldl((0, 0), (acc, s) =>
      (acc._1 + 1, if (acc._1 < k) acc._2 + nChildren(h, s) else acc._2))._2
  pure def dsBefore(h: int, k: int): int =
    stageScopes(h).foldl((0, 0), (acc, s) =>
      (acc._1 + 1, if (acc._1 < k) acc._2 + dOf(h, s) else acc._2))._2
  pure def qsBefore(h: int, k: int): int =
    stageScopes(h).foldl((0, 0), (acc, s) =>
      (acc._1 + 1, if (acc._1 < k) acc._2 + qOf(h, s) else acc._2))._2
  pure def pendsBefore(pk: (str, int), k: int): int =
    asmResList(pk).foldl((0, 0), (acc, p) =>
      (acc._1 + 1, if (acc._1 < k) acc._2 + p else acc._2))._2

  pure def qSum(q: int -> int): int =
    0.to(F - 1).fold(0, (acc, i) => acc + q.get(i))

  // Cumulative channel operations, derived from process-local state.
  // Prologue recvs happen once per scope: the wire in phase 0 -> 1, the
  // query in phase 1 -> 2; past phase 2 every scope of the stage is done.
  def wkWireRecvd(pk: (str, int)): int =
    if (wkPhase.get(pk) >= 3) stageLen(pk._2)
    else wkScope.get(pk) + (if (wkPhase.get(pk) == 1 or wkPhase.get(pk) == 2) 1 else 0)
  def wkAskedRecvd(pk: (str, int)): int =
    if (wkPhase.get(pk) >= 3) stageLen(pk._2)
    else wkScope.get(pk) + (if (wkPhase.get(pk) == 2) 1 else 0)
  // Sends: completed scopes contribute their totals (prefix sums); the
  // current scope contributes its live ledgers (empty outside phase 2).
  def wkWireSent(pk: (str, int)): int =
    wiresBefore(pk._2, wkScope.get(pk)) + wkWire.get(pk).size()
  def wkResSent(pk: (str, int)): int =
    dsBefore(pk._2, wkScope.get(pk)) + wkRes.get(pk).size()
  def wkQSentTot(pk: (str, int)): int =
    qsBefore(pk._2, wkScope.get(pk)) + qSum(wkQ.get(pk))
  def wkParentSent(pk: (str, int)): int =
    wkScope.get(pk) + (if (wkPhase.get(pk) == 2 and wkParent.get(pk)) 1 else 0)

  def asmResRecvd(pk: (str, int)): int =
    asmIdx.get(pk) + (if (asmPhase.get(pk) == 1 or asmPhase.get(pk) == 2) 1 else 0)
  def asmLevelRecvd(pk: (str, int)): int =
    pendsBefore(pk, asmIdx.get(pk)) + asmGot.get(pk)

  def absorbWireRecvd: int =
    if (absorbPhase >= 3) totalLeafReqs
    else absorbIdx + (if (absorbPhase == 1 or absorbPhase == 2) 1 else 0)
  def absorbAskedRecvd: int =
    if (absorbPhase >= 3) totalLeafReqs
    else absorbIdx + (if (absorbPhase == 2) 1 else 0)

  /// Does Asm pk send to a root singleton instead of a level channel?
  pure def isRootOutKey(pk: (str, int)): bool =
    (pk._1 == "I" and pk._2 == ROOT_H) or (pk._1 == "R" and pk._2 == ROOT_H - 1)

  /// Cumulative sends into channel c by its (unique) producer.
  def sentOf(c: (str, str, int)): int =
    if (c._1 == "wire" and c._3 == ROOT_H)
      (if (c._2 == "I") b2i(iopenWire) else b2i(ropenWire))
    else if (c._1 == "wire") wkWireSent((c._2, c._3))
    else if (c._1 == "asked" and c._2 == "I" and c._3 == ROOT_H - 1) b2i(iopenQuery)
    else if (c._1 == "asked" and c._2 == "R" and c._3 == ROOT_H - 2) ropenQ
    else if (c._1 == "asked") wkQSentTot((c._2, c._3 + 2))
    else if (c._1 == "upper") wkParentSent((c._2, c._3))
    else if (c._1 == "lower") wkResSent((c._2, c._3))
    else if (c._1 == "level" and c._2 == "I" and c._3 == 0) absorbIdx
    else if (c._1 == "level")
      (if (asmKeys.contains((c._2, c._3)) and not(isRootOutKey((c._2, c._3))))
        asmIdx.get((c._2, c._3)) else 0)
    else if (c._1 == "rootret") asmIdx.get(("I", ROOT_H))
    else if (c._1 == "rootrets") asmIdx.get(("R", ROOT_H - 1))
    else b2i(ropenRes) // rootres

  /// Cumulative receives from channel c by its (unique) consumer.
  def recvdOf(c: (str, str, int)): int =
    if (c._1 == "wire" and c._3 == ROOT_H)
      (if (c._2 == "I") b2i(ropenGotWire) else wkWireRecvd(("I", ROOT_H - 1)))
    else if (c._1 == "wire" and c._2 == "R" and c._3 == 0) absorbWireRecvd
    else if (c._1 == "wire") wkWireRecvd((other(c._2), c._3 - 1))
    else if (c._1 == "asked" and c._3 == -1) absorbAskedRecvd
    else if (c._1 == "asked") wkAskedRecvd((c._2, c._3))
    else if (c._1 == "upper") asmResRecvd((c._2, c._3 + 1))
    else if (c._1 == "lower")
      (if (asmKeys.contains((c._2, c._3))) asmResRecvd((c._2, c._3)) else 0)
    else if (c._1 == "level")
      (if (asmKeys.contains((c._2, c._3 + 1))) asmLevelRecvd((c._2, c._3 + 1)) else 0)
    else if (c._1 == "rootret") b2i(ifin)
    else if (c._1 == "rootrets") rfinGot
    else b2i(rfinGotRes) // rootres

  /// Assignment-style domain bindings for every state variable. MUST stay
  /// the syntactically first occurrence of each variable in indInv, and
  /// every range bound must be a constant expression (Apalache does not
  /// fold Len/fold over the inlined skeleton; occupancyOk and the local
  /// invariants re-tighten the loose bounds).
  val indTypeOk: bool =
    wkScope.in(setOfMaps(walkKeys, 0.to(NSC))) and
    wkPhase.in(setOfMaps(walkKeys, 0.to(5))) and
    wkWire.in(setOfMaps(walkKeys, 0.to(F - 1).powerset())) and
    wkRes.in(setOfMaps(walkKeys, 0.to(F - 1).powerset())) and
    wkQ.in(setOfMaps(walkKeys, setOfMaps(0.to(F - 1), 0.to(F)))) and
    wkParent.in(setOfMaps(walkKeys, Bool)) and
    wkChKind.in(setOfMaps(walkKeys, Set("none", "wire", "res", "query", "parent"))) and
    wkChChild.in(setOfMaps(walkKeys, (-1).to(F - 1))) and
    asmIdx.in(setOfMaps(asmKeys, 0.to(NSC))) and
    asmPhase.in(setOfMaps(asmKeys, 0.to(4))) and
    asmGot.in(setOfMaps(asmKeys, 0.to(F))) and
    chan.in(setOfMaps(allChans, 0.to(CAP_LEVEL))) and
    iopenWire.in(Bool) and iopenQuery.in(Bool) and
    iopenCh.in(Set("none", "wire", "query")) and
    ropenGotWire.in(Bool) and ropenWire.in(Bool) and ropenRes.in(Bool) and
    ropenQ.in(0.to(F)) and
    ropenCh.in(Set("none", "wire", "res", "query")) and
    absorbIdx.in(0.to(NSC * F)) and absorbPhase.in(0.to(5)) and
    ifin.in(Bool) and
    rfinGotRes.in(Bool) and rfinGot.in(0.to(F))

  /// Per-walk structural consistency: what init establishes and every
  /// walk transition preserves.
  def wkLocalOk(pk: (str, int)): bool = {
    val ws = wk(pk)
    val h = pk._2
    val len = stageLen(h)
    val s = scAt(h, ws.scope)
    val n = nChildren(h, s)
    and {
      // scope cursor vs phase
      if (ws.phase <= 2) ws.scope < len else ws.scope == len,
      // publishing machinery is live only in phase 2
      (ws.phase != 2) implies and {
        ws.wireDone == Set(), ws.resDone == Set(), ws.qSent == emptyQ,
        ws.parentDone == false, ws.chKind == "none", ws.chChild == -1
      },
      (ws.phase == 2) implies and {
        // an in-progress scope is not complete (normWalk advances past)
        not(scopeComplete(h, ws)),
        // ledger bounds, in-order prefixes, axiom shadows
        0.to(F - 1).forall(j => and {
          ws.wireDone.contains(j) implies
            (j < n and (j == 0 or ws.wireDone.contains(j - 1))),
          ws.resDone.contains(j) implies (j < n and childIsD(h, s, j)),
          ws.qSent.get(j) <= qCount(h, s, j),
          (ws.qSent.get(j) > 0) implies
            0.to(F - 1).forall(j2 => (j2 < j) implies ws.qSent.get(j2) == qCount(h, s, j2)),
          ws.resDone.contains(j) implies
            0.to(F - 1).forall(j2 =>
              (j2 < j and childIsD(h, s, j2)) implies ws.resDone.contains(j2)),
          AX_W implies (ws.resDone.contains(j) implies ws.wireDone.contains(j)),
          AX_D1_INT implies ((ws.qSent.get(j) > 0) implies ws.resDone.contains(j)),
          // exactly the guard: WIRE_FIRST only (W does not order queries).
          WIRE_FIRST implies
            ((ws.qSent.get(j) > 0) implies ws.wireDone.contains(j)),
          AX_D3 implies
            ((ws.resDone.contains(j) and ws.qSent.get(j) < qCount(h, s, j)) implies
              0.to(F - 1).forall(j2 =>
                (j2 != j and ws.resDone.contains(j2)) implies
                  ws.qSent.get(j2) == qCount(h, s, j2)))
        }),
        // a committed obligation was choosable when committed, and the
        // walk is sequential, so the choosable facts still hold
        (ws.chKind == "none") implies ws.chChild == -1,
        (ws.chKind == "wire") implies and {
          ws.chChild == ws.wireDone.size(),
          ws.chChild < n
        },
        (ws.chKind == "res") implies and {
          ws.chChild >= 0, ws.chChild < n,
          childIsD(h, s, ws.chChild),
          not(ws.resDone.contains(ws.chChild)),
          0.to(F - 1).forall(j =>
            (j < ws.chChild and childIsD(h, s, j)) implies ws.resDone.contains(j)),
          AX_W implies ws.wireDone.contains(ws.chChild),
          AX_D3 implies 0.to(F - 1).forall(j =>
            ws.resDone.contains(j) implies ws.qSent.get(j) == qCount(h, s, j))
        },
        (ws.chKind == "query") implies and {
          ws.chChild >= 0, ws.chChild < n,
          childIsD(h, s, ws.chChild),
          ws.qSent.get(ws.chChild) < qCount(h, s, ws.chChild),
          0.to(F - 1).forall(j =>
            (j < ws.chChild) implies ws.qSent.get(j) == qCount(h, s, j)),
          AX_D1_INT implies ws.resDone.contains(ws.chChild),
          WIRE_FIRST implies ws.wireDone.contains(ws.chChild)
        },
        (ws.chKind == "parent") implies and {
          ws.chChild == -1,
          not(ws.parentDone),
          AX_D2 implies 0.to(F - 1).forall(j =>
            (j < n and childIsD(h, s, j)) implies ws.resDone.contains(j))
        }
      }
    }
  }

  /// Per-assembler structural consistency.
  def asmLocalOk(pk: (str, int)): bool = {
    val a = asmV(pk)
    val len = asmResList(pk).length()
    and {
      if (a.phase <= 2) a.idx < len else a.idx == len,
      (a.phase == 1) implies a.got < pendAt(pk, a.idx),
      (a.phase == 2) implies a.got == pendAt(pk, a.idx),
      (a.phase == 0 or a.phase >= 3) implies a.got == 0
    }
  }

  /// Openers, absorb, finishes: structural consistency.
  val topLocalOk: bool = and {
    (iopenCh == "wire") implies not(iopenWire),
    (iopenCh == "query") implies (not(iopenQuery) and (AX_W implies iopenWire)),
    not(ropenGotWire) implies and {
      not(ropenWire), not(ropenRes), ropenQ == 0, ropenCh == "none"
    },
    ropenQ <= rootPending,
    // fired-fact shadow of the res guard: the root resolution can only
    // have fired after the wire reply (under W). Without this, a havoc'd
    // res-without-wire state satisfies the invariant and then commits a
    // query that the committed-query conjunct rejects (first CTI).
    AX_W implies (ropenRes implies ropenWire),
    (ropenCh == "wire") implies not(ropenWire),
    (ropenCh == "res") implies (not(ropenRes) and (AX_W implies ropenWire)),
    (ropenCh == "query") implies and {
      ropenQ < rootPending,
      AX_D1_ROOT implies ropenRes,
      // exactly the guard: W alone does NOT order queries after the wire
      // (the wire ledger never constrains DependentWork — the
      // n2unrestricted finding, mirrored here).
      WIRE_FIRST implies ropenWire
    },
    if (absorbPhase <= 2) absorbIdx < totalLeafReqs else absorbIdx == totalLeafReqs,
    absorbIdx <= totalLeafReqs,
    not(rfinGotRes) implies rfinGot == 0,
    rfinGot <= rootPending
  }

  val indLocal: bool =
    walkKeys.forall(pk => wkLocalOk(pk)) and
    asmKeys.forall(pk => asmLocalOk(pk)) and
    topLocalOk

  /// Flow conservation: every channel holds exactly what its producer
  /// sent minus what its consumer received, both derived from
  /// process-local state. With chan >= 0 (indTypeOk) this pins the whole
  /// channel layer to the process states.
  val indFlow: bool =
    allChans.forall(c => chan.get(c) == sentOf(c) - recvdOf(c))

  /// The Phase B inductive invariant.
  val indInv: bool = indTypeOk and occupancyOk and indLocal and indFlow

  // ============================================ coverage witnesses (run)

  val usedLevel: bool = allChans.exists(c => c._1 == "level" and chan.get(c) > 0)
  val usedWire: bool = allChans.exists(c => c._1 == "wire" and chan.get(c) > 0)
  val usedAsked: bool = allChans.exists(c => c._1 == "asked" and chan.get(c) > 0)
  val usedUpper: bool = allChans.exists(c => c._1 == "upper" and chan.get(c) > 0)
  val usedLower: bool = allChans.exists(c => c._1 == "lower" and chan.get(c) > 0)
  val usedRoots: bool =
    chan.get(("rootret", "I", 0)) > 0 or chan.get(("rootrets", "R", 0)) > 0 or
    chan.get(("rootres", "R", 0)) > 0

  /// Some process is committed to a send whose channel is full — the
  /// backpressure witness (capacity_stress requires blocked_send_polls>0).
  val blockedSend: bool =
    walkKeys.exists(pk => wkChKind.get(pk) != "none" and not(wkFireGuard(pk))) or
    asmKeys.exists(pk => asmPhase.get(pk) == 2 and chan.get(asmOutChan(pk)) >= cap(asmOutChan(pk))) or
    (absorbPhase == 2 and chan.get(("level", "I", 0)) >= cap(("level", "I", 0)))

  val reachedTerminal: bool = terminal
}
