// Instances of the open-stage assume-guarantee module (openStage.qnt):
// the positive theorem, the Phase A conjecture check, and the dropped-
// axiom stage controls. Expected outcomes are encoded in checkStage.sh;
// a control instance "passes" when the checker FINDS a stage jam.
//
// Every instance havocs the stage structure at init (up to K scopes, F
// kids per scope, F queries per disputed kid), so one run covers every
// stage input shape at those bounds — there is no skeleton to pick.

// --- positive: all axioms on, production capacity stance CAP_LEVEL = F.
// Expected: no stage jam reachable, done reachable.
module stageAll {
  import openStage(
    K = 2, F = 4, CAP_LEVEL = 4,
    AX_W = true, AX_D1_INT = true, AX_D2 = true, AX_D3 = true, WIRE_FIRST = false
  ).* from "./openStage"
}

// --- the Phase A conjecture (instances.qnt n3Internal/n3Reduced):
// internally, D1 is not individually load-bearing once D3 and the
// per-channel order hold. Here that is checked over ALL stage structures
// at K=2/F=4, not just sampled skeletons. Expected: SAFE — a jam here
// REFUTES the conjecture and is a finding, not a check failure.
module stageNoD1 {
  import openStage(
    K = 2, F = 4, CAP_LEVEL = 4,
    AX_W = true, AX_D1_INT = false, AX_D2 = true, AX_D3 = true, WIRE_FIRST = true
  ).* from "./openStage"
}

// --- the D3 control: the ledgerGap shape, level-generically. A walk
// that resolves a later sibling while an earlier resolved sibling still
// owes queries starves the lower assembler through the cap-1 lower
// channel. Expected: jam reachable.
module stageNoD3 {
  import openStage(
    K = 2, F = 4, CAP_LEVEL = 4,
    AX_W = true, AX_D1_INT = true, AX_D2 = true, AX_D3 = false, WIRE_FIRST = false
  ).* from "./openStage"
}

// --- the W control at stage level. The braid conditioning on levelIn
// (a return is entitled only after the walk's own wire send for that
// child) makes W visible here: a walk that resolves and queries a child
// while withholding its wire reply starves its own assembler, and its
// next committed resolution blocks forever on the cap-1 lower channel.
// Expected: jam reachable.
module stageNoW {
  import openStage(
    K = 2, F = 4, CAP_LEVEL = 4,
    AX_W = false, AX_D1_INT = true, AX_D2 = true, AX_D3 = true, WIRE_FIRST = false
  ).* from "./openStage"
}

// --- the positive at production fan: symbolic-at-256 stage structures.
// Run-length bounds at F=256 are ~660k steps, so the exhaustive tier is
// out of reach; checkStage.sh runs a bounded-depth verify plus a sampled
// simulator sweep and records exactly what was covered.
module stageFan256 {
  import openStage(
    K = 2, F = 256, CAP_LEVEL = 256,
    AX_W = true, AX_D1_INT = true, AX_D2 = true, AX_D3 = true, WIRE_FIRST = false
  ).* from "./openStage"
}
