// The level-generic open stage: one internal walk plus its two assemblers,
// with a havoc environment constrained only by the axioms and the channel
// disciplines (Phase B assume-guarantee module; see ../MODEL.md and the
// full-pipeline model in streamingMirror.qnt).
//
// Why this module exists: the full model fixes a skeleton and checks the
// whole pipeline; this module cuts out ONE interior stage and replaces its
// neighbors with contract-abiding havoc processes. The walk/assembler
// programs are transcribed from streamingMirror.qnt unchanged (same
// committed-choice publisher, same axiom guards); the per-scope child
// structure is chosen nondeterministically at init and frozen, so a single
// check covers EVERY stage input shape up to the K/F bounds — this is the
// "relaxation loses progress at every level" result without depth-long BMC.
//
// Level-genericity: height and party never appear below. The interior walk
// program is identical at every consume-height and on both parties — only
// the channel names differ, and those are positional here. The leaf-parent
// stage is the same shape with qCount playing leafReqs (its "queries" are
// wire-only leaf requests and its terminal assembler consumes resolutions
// with no Pending slots — a degenerate instance of this module). The root
// openings are NOT instances of this shape; their controls live in the
// full model (n1/n2 instances) and in production.qnt.
//
// The environment contract, per edge (transcribing the per-edge contracts
// documented in work/queues.rs):
//   - wireIn / askedIn: one reply and one query per stage scope, in order,
//     unconditionally entitled — the producers (the counterparty walk one
//     above, the same-party walk two above) owe these regardless of
//     anything this stage does; their ability to deliver is their own
//     stages' guarantee.
//   - levelIn: one completed return per downward query, in order — but a
//     return for child c is entitled only after this stage has BOTH sent
//     its wire reply for c (the counterparty walk below cannot process
//     scope c without it) AND sent the query (the braid: the query pairs,
//     two stages down, with a reply enabled by our wire send). This
//     conditioning is load-bearing: it is what lets a wire- or
//     query-withholding walk starve its own assembler.
//   - wireOut / askedOut / levelOut: the downstream consumers eventually
//     drain each item (their stages' guarantee).
//
// Environment actions are enabled exactly when the contract permits them
// and may always be deferred (the checker owns the schedule). Therefore in
// any globally stuck state the environment is already maximally generous:
// every env-drained channel it could drain is empty, and every entitled
// supply is either delivered or blocked on a full channel the stage
// refuses to drain. A stuck state is thus a STAGE jam — intrinsic to the
// stage's own publication ordering — which is exactly the predicate the
// positive theorem excludes and the dropped-axiom controls must reach.
//
// Composition (the meta-step, proved in Lean in Phase C): each real
// stage's environment obligations are discharged by its neighbors'
// guarantees, well-founded by height; this module is the induction's
// interior case.
module openStage {
  // ========================================================== parameters

  /// Max scopes this stage processes (the real bound is the parent
  /// stage's total dispute count; runs are exhaustive up to K).
  const K: int

  /// Fan bound (Rust FAN = 256): max children per scope and max queries
  /// per disputed child.
  const F: int

  /// Capacity of the level channels (Rust default: FAN).
  const CAP_LEVEL: int

  /// Axiom W: internal publication consumes a prior wire action.
  const AX_W: bool
  /// Axiom D1 internally: a child query requires its scope's resolution.
  const AX_D1_INT: bool
  /// Axiom D2: a parent resolution requires its D children's resolutions.
  const AX_D2: bool
  /// Axiom D3 (sibling contiguity): a child's resolution may be chosen
  /// only once every already-resolved sibling's queries have been sent.
  const AX_D3: bool
  /// Control scaffolding, NOT an axiom (see streamingMirror.qnt): force
  /// each child's queries after its wire send even when D1 is dropped.
  const WIRE_FIRST: bool

  // ================================================================ state
  //
  // The stage structure is havoc'd at init and frozen: nScopes scopes,
  // each with kidsN(s) children; child (s, i) is disputed iff dKind(s, i)
  // and then launches qCnt(s, i) downward queries. Junk entries beyond
  // kidsN(s) or on R children are never read (guards check childIsD
  // first), so every havoc combination is a legal stage input — there is
  // no rejection predicate to keep the init from being vacuous.

  type Stru = {
    nScopes: int,
    kidsN: int -> int,
    dKind: (int, int) -> bool,
    qCnt: (int, int) -> int
  }

  /// Walk state. Unlike streamingMirror.qnt's per-scope ledgers (reset by
  /// freshWalk), the ledgers here are cumulative, keyed by (scope, child):
  /// the environment's levelIn entitlement needs the walk's whole send
  /// history, not just the current scope's. Semantically identical —
  /// guards only ever consult the current scope's entries.
  type WalkSt = {
    scope: int,             // current scope index
    phase: int,             // 0 recvWire, 1 recvAsked, 2 publishing, 3 closeWire, 4 closeAsked, 5 done
    wireDone: Set[(int, int)],
    resDone: Set[(int, int)],
    qSent: (int, int) -> int,
    parentDone: Set[int],
    chKind: str,            // committed obligation: "none"|"wire"|"res"|"query"|"parent"
    chChild: int
  }

  /// Assembler state: pos is a flat (scope * F + child) cursor for the
  /// lower assembler, a scope index for the upper one.
  type AsmSt = { pos: int, phase: int, got: int }  // phase: 0 recvRes, 1 recvLevels, 2 send, 4 done

  /// Channel occupancies. wi/ai: wireIn/askedIn (env-fed, cap 1);
  /// wo/ao: wireOut/askedOut (env-drained, cap 1); lo/uo: the walk's
  /// lower/upper resolution channels (internal, cap 1); li: levelIn
  /// (env-fed, cap CAP_LEVEL); lm: levelMid, the lower assembler's output
  /// consumed by the upper assembler (internal, cap CAP_LEVEL — the
  /// ledgerGap jam locus); lout: the upper assembler's output (env-
  /// drained, cap CAP_LEVEL: in the pipeline it is the next level channel
  /// up, not a cap-1 root singleton).
  type Chans = {
    wi: int, ai: int, wo: int, ao: int,
    lo: int, uo: int, li: int, lm: int, lout: int
  }

  /// Environment supply counters: wire replies and queries delivered
  /// (entitlement: one per scope), level returns delivered (entitlement:
  /// levelEntitled below).
  type Env = { w: int, a: int, l: int }

  var stru: Stru
  var wk: WalkSt
  var loA: AsmSt
  var upA: AsmSt
  var ch: Chans
  var env: Env

  // ==================================================== derived structure

  pure def flatS(f: int): int = f / F
  pure def flatI(f: int): int = f % F

  def nKids(s: int): int =
    if (s < 0 or s >= K) 0 else stru.kidsN.get(s)

  def childIsD(s: int, i: int): bool =
    s >= 0 and s < K and i >= 0 and i < F and
    i < stru.kidsN.get(s) and stru.dKind.get((s, i))

  def qCount(s: int, i: int): int =
    if (not(childIsD(s, i))) 0 else stru.qCnt.get((s, i))

  def dCount(s: int): int =
    0.to(F - 1).fold(0, (a, i) => a + (if (childIsD(s, i)) 1 else 0))

  def childIsDFlat(f: int): bool = childIsD(flatS(f), flatI(f))

  /// The smallest flat D-child position >= f, or K*F if none — the lower
  /// assembler's resolution order (the walk sends lower resolutions for D
  /// children in scope-then-child order).
  def nextD(f: int): int =
    0.to(K * F - 1).fold(K * F, (a, g) =>
      if (g >= f and g < a and childIsDFlat(g)) g else a)

  /// Level returns the environment may have delivered by now: one per
  /// downward query already sent, gated per child on the walk's own wire
  /// send for that child (the braid; see the module doc).
  def levelEntitled: int =
    tuples(0.to(K - 1), 0.to(F - 1)).fold(0, (a, t) =>
      a + (if (childIsD(t._1, t._2) and wk.wireDone.contains(t))
             wk.qSent.get(t) else 0))

  pure val emptyQ: (int, int) -> int =
    tuples(0.to(K - 1), 0.to(F - 1)).mapBy(t => 0)

  // ================================================================= init

  action init = {
    nondet ns = oneOf(0.to(K))
    nondet kn = oneOf(0.to(K - 1).setOfMaps(0.to(F)))
    nondet dk = oneOf(tuples(0.to(K - 1), 0.to(F - 1)).setOfMaps(Set(true, false)))
    nondet qc = oneOf(tuples(0.to(K - 1), 0.to(F - 1)).setOfMaps(0.to(F)))
    all {
      stru' = { nScopes: ns, kidsN: kn, dKind: dk, qCnt: qc },
      wk' = {
        scope: 0, phase: if (ns > 0) 0 else 3,
        wireDone: Set(), resDone: Set(), qSent: emptyQ, parentDone: Set(),
        chKind: "none", chChild: -1
      },
      loA' = { pos: 0, phase: 0, got: 0 },
      upA' = { pos: 0, phase: 0, got: 0 },
      ch' = { wi: 0, ai: 0, wo: 0, ao: 0, lo: 0, uo: 0, li: 0, lm: 0, lout: 0 },
      env' = { w: 0, a: 0, l: 0 }
    }
  }

  // ================================================================ walk
  // Transcribed from streamingMirror.qnt's wkChoosable/walkStep: the
  // committed-choice publisher (commit to the next axiom-consistent
  // obligation, then complete it before choosing again — load-bearing:
  // may-fire semantics would let the checker dodge every jam by firing
  // the withheld operation), with the per-channel in-order program
  // structure that is never relaxed.

  def wkChoosable(k: str, i: int): bool = {
    val s = wk.scope
    if (wk.phase != 2) false
    else if (k == "wire")
      i >= 0 and i < nKids(s) and not(wk.wireDone.contains((s, i))) and
      0.to(F - 1).forall(j => j >= i or wk.wireDone.contains((s, j)))
    else if (k == "res")
      childIsD(s, i) and not(wk.resDone.contains((s, i))) and
      0.to(F - 1).forall(j => j >= i or not(childIsD(s, j)) or wk.resDone.contains((s, j))) and
      (not(AX_W) or wk.wireDone.contains((s, i))) and
      (not(AX_D3) or 0.to(F - 1).forall(j =>
        j >= nKids(s) or not(wk.resDone.contains((s, j))) or wk.qSent.get((s, j)) == qCount(s, j)))
    else if (k == "query")
      childIsD(s, i) and wk.qSent.get((s, i)) < qCount(s, i) and
      0.to(F - 1).forall(j => j >= i or wk.qSent.get((s, j)) == qCount(s, j)) and
      (not(AX_D1_INT) or wk.resDone.contains((s, i))) and
      (not(WIRE_FIRST) or wk.wireDone.contains((s, i)))
    else if (k == "parent")
      not(wk.parentDone.contains(s)) and
      (not(AX_D2) or 0.to(F - 1).forall(j =>
        j >= nKids(s) or not(childIsD(s, j)) or wk.resDone.contains((s, j))))
    else false
  }

  def scopeComplete(s: int): bool =
    wk.parentDone.contains(s) and
    0.to(F - 1).forall(i =>
      i >= nKids(s) or (
        wk.wireDone.contains((s, i)) and
        (not(childIsD(s, i)) or (wk.resDone.contains((s, i)) and wk.qSent.get((s, i)) == qCount(s, i)))))

  def wkFireGuard: bool =
    if (wk.chKind == "none") false
    else if (wk.chKind == "wire") ch.wo < 1
    else if (wk.chKind == "res") ch.lo < 1
    else if (wk.chKind == "query") ch.ao < 1
    else ch.uo < 1

  action walkStep = all {
    stru' = stru, loA' = loA, upA' = upA, env' = env,
    any {
      // prologue: recv the scope's wire reply, then its queued query
      all {
        wk.phase == 0,
        ch.wi > 0,
        ch' = { wi: ch.wi - 1, ai: ch.ai, wo: ch.wo, ao: ch.ao, lo: ch.lo, uo: ch.uo, li: ch.li, lm: ch.lm, lout: ch.lout },
        wk' = { scope: wk.scope, phase: 1, wireDone: wk.wireDone, resDone: wk.resDone, qSent: wk.qSent, parentDone: wk.parentDone, chKind: "none", chChild: -1 }
      },
      all {
        wk.phase == 1,
        ch.ai > 0,
        ch' = { wi: ch.wi, ai: ch.ai - 1, wo: ch.wo, ao: ch.ao, lo: ch.lo, uo: ch.uo, li: ch.li, lm: ch.lm, lout: ch.lout },
        wk' = { scope: wk.scope, phase: 2, wireDone: wk.wireDone, resDone: wk.resDone, qSent: wk.qSent, parentDone: wk.parentDone, chKind: "none", chChild: -1 }
      },
      // publishing: commit to the next obligation (axiom-guarded choice)
      all {
        wk.phase == 2, wk.chKind == "none",
        nondet k = oneOf(Set("wire", "res", "query", "parent"))
        nondet i = oneOf((-1).to(F - 1))
        all {
          wkChoosable(k, i),
          if (k == "parent") i == -1 else true,
          wk' = { scope: wk.scope, phase: 2, wireDone: wk.wireDone, resDone: wk.resDone, qSent: wk.qSent, parentDone: wk.parentDone, chKind: k, chChild: i },
          ch' = ch
        }
      },
      // publishing: fire the committed obligation
      all {
        wk.phase == 2, wk.chKind != "none",
        wkFireGuard,
        val s = wk.scope
        val wd = if (wk.chKind == "wire") wk.wireDone.union(Set((s, wk.chChild))) else wk.wireDone
        val rd = if (wk.chKind == "res") wk.resDone.union(Set((s, wk.chChild))) else wk.resDone
        val qs = if (wk.chKind == "query") wk.qSent.set((s, wk.chChild), wk.qSent.get((s, wk.chChild)) + 1) else wk.qSent
        val pd = if (wk.chKind == "parent") wk.parentDone.union(Set(s)) else wk.parentDone
        // does this fire complete the scope? (checked on the post state)
        val done = pd.contains(s) and
          0.to(F - 1).forall(i =>
            i >= nKids(s) or (
              wd.contains((s, i)) and
              (not(childIsD(s, i)) or (rd.contains((s, i)) and qs.get((s, i)) == qCount(s, i)))))
        all {
          ch' = {
            wi: ch.wi, ai: ch.ai,
            wo: if (wk.chKind == "wire") ch.wo + 1 else ch.wo,
            ao: if (wk.chKind == "query") ch.ao + 1 else ch.ao,
            lo: if (wk.chKind == "res") ch.lo + 1 else ch.lo,
            uo: if (wk.chKind == "parent") ch.uo + 1 else ch.uo,
            li: ch.li, lm: ch.lm, lout: ch.lout
          },
          wk' = {
            scope: if (done) s + 1 else s,
            phase: if (not(done)) 2 else if (s + 1 < stru.nScopes) 0 else 3,
            wireDone: wd, resDone: rd, qSent: qs, parentDone: pd,
            chKind: "none", chChild: -1
          }
        }
      },
      // end-of-stream: producer exhausted and channel drained
      all {
        wk.phase == 3,
        env.w == stru.nScopes, ch.wi == 0,
        ch' = ch,
        wk' = { scope: wk.scope, phase: 4, wireDone: wk.wireDone, resDone: wk.resDone, qSent: wk.qSent, parentDone: wk.parentDone, chKind: "none", chChild: -1 }
      },
      all {
        wk.phase == 4,
        env.a == stru.nScopes, ch.ai == 0,
        ch' = ch,
        wk' = { scope: wk.scope, phase: 5, wireDone: wk.wireDone, resDone: wk.resDone, qSent: wk.qSent, parentDone: wk.parentDone, chKind: "none", chChild: -1 }
      }
    }
  }

  def wkCanStep: bool =
    if (wk.phase == 0) ch.wi > 0
    else if (wk.phase == 1) ch.ai > 0
    else if (wk.phase == 2 and wk.chKind == "none")
      Set("wire", "res", "query", "parent").exists(k =>
        (-1).to(F - 1).exists(i =>
          wkChoosable(k, i) and (if (k == "parent") i == -1 else true)))
    else if (wk.phase == 2) wkFireGuard
    else if (wk.phase == 3) env.w == stru.nScopes and ch.wi == 0
    else if (wk.phase == 4) env.a == stru.nScopes and ch.ai == 0
    else false

  def walkDone: bool = wk.phase == 5

  // =========================================================== assemblers
  // The lower assembler consumes the walk's lower resolutions (one per D
  // child, in order); each carries qCount Pending slots filled from
  // levelIn; the assembled node goes out on levelMid. The upper assembler
  // consumes the walk's parent resolutions (one per scope, dCount Pending
  // slots each), filled from levelMid, out on lout. Both observe the
  // resolution channel's close after their last item (recv-on-empty with
  // the walk done takes the end branch).

  action lowerStep = all {
    stru' = stru, wk' = wk, upA' = upA, env' = env,
    any {
      all {
        loA.phase == 0,
        ch.lo > 0,
        val d = nextD(loA.pos)
        all {
          ch' = { wi: ch.wi, ai: ch.ai, wo: ch.wo, ao: ch.ao, lo: ch.lo - 1, uo: ch.uo, li: ch.li, lm: ch.lm, lout: ch.lout },
          loA' = { pos: loA.pos, phase: if (qCount(flatS(d), flatI(d)) > 0) 1 else 2, got: 0 }
        }
      },
      all {
        loA.phase == 1,
        ch.li > 0,
        val d = nextD(loA.pos)
        all {
          ch' = { wi: ch.wi, ai: ch.ai, wo: ch.wo, ao: ch.ao, lo: ch.lo, uo: ch.uo, li: ch.li - 1, lm: ch.lm, lout: ch.lout },
          loA' = { pos: loA.pos, phase: if (loA.got + 1 == qCount(flatS(d), flatI(d))) 2 else 1, got: loA.got + 1 }
        }
      },
      all {
        loA.phase == 2,
        ch.lm < CAP_LEVEL,
        ch' = { wi: ch.wi, ai: ch.ai, wo: ch.wo, ao: ch.ao, lo: ch.lo, uo: ch.uo, li: ch.li, lm: ch.lm + 1, lout: ch.lout },
        loA' = { pos: nextD(loA.pos) + 1, phase: 0, got: 0 }
      },
      all {
        loA.phase == 0,
        walkDone, ch.lo == 0,
        ch' = ch,
        loA' = { pos: loA.pos, phase: 4, got: 0 }
      }
    }
  }

  def loCanStep: bool =
    if (loA.phase == 0) ch.lo > 0 or (walkDone and ch.lo == 0)
    else if (loA.phase == 1) ch.li > 0
    else if (loA.phase == 2) ch.lm < CAP_LEVEL
    else false

  action upperStep = all {
    stru' = stru, wk' = wk, loA' = loA, env' = env,
    any {
      all {
        upA.phase == 0,
        ch.uo > 0,
        ch' = { wi: ch.wi, ai: ch.ai, wo: ch.wo, ao: ch.ao, lo: ch.lo, uo: ch.uo - 1, li: ch.li, lm: ch.lm, lout: ch.lout },
        upA' = { pos: upA.pos, phase: if (dCount(upA.pos) > 0) 1 else 2, got: 0 }
      },
      all {
        upA.phase == 1,
        ch.lm > 0,
        ch' = { wi: ch.wi, ai: ch.ai, wo: ch.wo, ao: ch.ao, lo: ch.lo, uo: ch.uo, li: ch.li, lm: ch.lm - 1, lout: ch.lout },
        upA' = { pos: upA.pos, phase: if (upA.got + 1 == dCount(upA.pos)) 2 else 1, got: upA.got + 1 }
      },
      all {
        upA.phase == 2,
        ch.lout < CAP_LEVEL,
        ch' = { wi: ch.wi, ai: ch.ai, wo: ch.wo, ao: ch.ao, lo: ch.lo, uo: ch.uo, li: ch.li, lm: ch.lm, lout: ch.lout + 1 },
        upA' = { pos: upA.pos + 1, phase: 0, got: 0 }
      },
      all {
        upA.phase == 0,
        walkDone, ch.uo == 0,
        ch' = ch,
        upA' = { pos: upA.pos, phase: 4, got: 0 }
      }
    }
  }

  def upCanStep: bool =
    if (upA.phase == 0) ch.uo > 0 or (walkDone and ch.uo == 0)
    else if (upA.phase == 1) ch.lm > 0
    else if (upA.phase == 2) ch.lout < CAP_LEVEL
    else false

  // ========================================================== environment
  // Each action is enabled exactly when the contract permits and may
  // always be deferred; see the module doc for why global stuckness then
  // implies a maximally generous environment.

  action envSupplyWire = all {
    env.w < stru.nScopes, ch.wi < 1,
    ch' = { wi: ch.wi + 1, ai: ch.ai, wo: ch.wo, ao: ch.ao, lo: ch.lo, uo: ch.uo, li: ch.li, lm: ch.lm, lout: ch.lout },
    env' = { w: env.w + 1, a: env.a, l: env.l },
    stru' = stru, wk' = wk, loA' = loA, upA' = upA
  }

  action envSupplyAsked = all {
    env.a < stru.nScopes, ch.ai < 1,
    ch' = { wi: ch.wi, ai: ch.ai + 1, wo: ch.wo, ao: ch.ao, lo: ch.lo, uo: ch.uo, li: ch.li, lm: ch.lm, lout: ch.lout },
    env' = { w: env.w, a: env.a + 1, l: env.l },
    stru' = stru, wk' = wk, loA' = loA, upA' = upA
  }

  action envSupplyLevel = all {
    env.l < levelEntitled, ch.li < CAP_LEVEL,
    ch' = { wi: ch.wi, ai: ch.ai, wo: ch.wo, ao: ch.ao, lo: ch.lo, uo: ch.uo, li: ch.li + 1, lm: ch.lm, lout: ch.lout },
    env' = { w: env.w, a: env.a, l: env.l + 1 },
    stru' = stru, wk' = wk, loA' = loA, upA' = upA
  }

  action envDrainWire = all {
    ch.wo > 0,
    ch' = { wi: ch.wi, ai: ch.ai, wo: ch.wo - 1, ao: ch.ao, lo: ch.lo, uo: ch.uo, li: ch.li, lm: ch.lm, lout: ch.lout },
    env' = env, stru' = stru, wk' = wk, loA' = loA, upA' = upA
  }

  action envDrainAsked = all {
    ch.ao > 0,
    ch' = { wi: ch.wi, ai: ch.ai, wo: ch.wo, ao: ch.ao - 1, lo: ch.lo, uo: ch.uo, li: ch.li, lm: ch.lm, lout: ch.lout },
    env' = env, stru' = stru, wk' = wk, loA' = loA, upA' = upA
  }

  action envDrainOut = all {
    ch.lout > 0,
    ch' = { wi: ch.wi, ai: ch.ai, wo: ch.wo, ao: ch.ao, lo: ch.lo, uo: ch.uo, li: ch.li, lm: ch.lm, lout: ch.lout - 1 },
    env' = env, stru' = stru, wk' = wk, loA' = loA, upA' = upA
  }

  def envCanStep: bool =
    (env.w < stru.nScopes and ch.wi < 1) or
    (env.a < stru.nScopes and ch.ai < 1) or
    (env.l < levelEntitled and ch.li < CAP_LEVEL) or
    ch.wo > 0 or ch.ao > 0 or ch.lout > 0

  // =========================================================== the system

  /// The stage has finished and everything it published has been drained.
  def stageDone: bool =
    walkDone and loA.phase == 4 and upA.phase == 4 and
    ch.wo == 0 and ch.ao == 0 and ch.lout == 0

  def canStepAny: bool = wkCanStep or loCanStep or upCanStep or envCanStep

  /// The jam predicate: a non-done state where neither the stage nor the
  /// environment can act. Because env enabledness coincides with
  /// contractual permission, any such state has envGenerous below — the
  /// jam is intrinsic to the stage's ordering, the stage-local twin of
  /// Quiescence::Stalled.
  def stageJam: bool = not(stageDone) and not(canStepAny)

  /// The positive theorem's invariant.
  def safeStage: bool = not(stageJam)

  /// Documentation self-check: at any stuck state the environment has
  /// drained everything it could and delivered everything entitled that
  /// the stage left room for. Implied by construction; kept as a checked
  /// val so the claim in the module doc stays honest.
  def envGenerous: bool =
    ch.wo == 0 and ch.ao == 0 and ch.lout == 0 and
    (env.w == stru.nScopes or ch.wi > 0) and
    (env.a == stru.nScopes or ch.ai > 0) and
    (env.l == levelEntitled or ch.li > 0)

  def jamIsGenuine: bool = not(stageJam) or envGenerous

  /// Occupancy sanity: no channel over capacity (encoding self-check).
  def occupancyOk: bool =
    ch.wi >= 0 and ch.wi <= 1 and ch.ai >= 0 and ch.ai <= 1 and
    ch.wo >= 0 and ch.wo <= 1 and ch.ao >= 0 and ch.ao <= 1 and
    ch.lo >= 0 and ch.lo <= 1 and ch.uo >= 0 and ch.uo <= 1 and
    ch.li >= 0 and ch.li <= CAP_LEVEL and
    ch.lm >= 0 and ch.lm <= CAP_LEVEL and
    ch.lout >= 0 and ch.lout <= CAP_LEVEL

  /// Conservation: a done stage must have drained every channel.
  def drainedAtDone: bool =
    not(stageDone) or (
      ch.wi == 0 and ch.ai == 0 and ch.lo == 0 and ch.uo == 0 and
      ch.li == 0 and ch.lm == 0)

  /// What checkStage.sh verifies on every positive instance.
  def stageInvariant: bool =
    safeStage and occupancyOk and drainedAtDone and jamIsGenuine

  action stutter = all {
    stageDone,
    stru' = stru, wk' = wk, loA' = loA, upA' = upA, ch' = ch, env' = env
  }

  action step = any {
    walkStep, lowerStep, upperStep,
    envSupplyWire, envSupplyAsked, envSupplyLevel,
    envDrainWire, envDrainAsked, envDrainOut,
    stutter
  }

  // ============================================= run-length bound (ρ + commits)
  // Every action draws on a finite budget derived from the worst havoc
  // shape (K scopes, F kids, F queries per kid), so runs are bounded and
  // BMC to this depth is exhaustive. Deliberately over-approximated from
  // the MAX shape; do not "tighten" it to a particular structure.

  pure val totalStepsStage: int =
    2 * K                              // walk prologue recvs
    + K * (2 * F + 2 * F + 2 * F * F + 2)  // walk commits + fires
    + 2                                // walk closes
    + K * F * (2 + F) + 1              // lower assembler
    + K * (2 + F) + 1                  // upper assembler
    + 2 * K + K * F * F                // env supplies
    + K * F + K * F * F + K            // env drains

  // ============================================ coverage witnesses (run)

  def reachedDone: bool = stageDone
  def usedLevelIn: bool = ch.li > 0
  def usedLevelMid: bool = ch.lm > 0
  def sawDispute: bool = ch.lo > 0

  /// Some process is committed to or ready for a send whose channel is
  /// full — the backpressure witness.
  def blockedSend: bool =
    (wk.chKind != "none" and not(wkFireGuard)) or
    (loA.phase == 2 and ch.lm >= CAP_LEVEL) or
    (upA.phase == 2 and ch.lout >= CAP_LEVEL)
}
