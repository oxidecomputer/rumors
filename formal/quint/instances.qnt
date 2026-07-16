// Instances of the streaming mirror model: positive validation shapes,
// the capacity-tightness pair, and the N1–N4 axiom-mode controls.
// Expected outcomes are encoded in check.sh; a control instance "passes"
// when the checker FINDS a stuck state.
//
// Skeleton conventions (MODEL.md §2): index 0 = root, BFS order, kind
// "D"|"R", R scopes have no kids, height-1 scopes carry leafReqs.

// --- positive: one dispute chain exercising every structurally distinct
// stage at D=3 (ROOT_H=4): both openings, one internal walk per party,
// leaf parent, leaf level, absorb, every assembler, both finishes.
module smokeChain {
  import streamingMirror(
    ROOT_H = 4, F = 2, CAP_LEVEL = 2, NSC = 4,
    AX_W = true, AX_D1_ROOT = true, AX_D1_INT = true, AX_D2 = true, AX_D3 = true, WIRE_FIRST = false,
    SKEL = [
      { kind: "D", height: 4, kids: [1], leafReqs: 0 },
      { kind: "D", height: 3, kids: [2], leafReqs: 0 },
      { kind: "D", height: 2, kids: [3], leafReqs: 0 },
      { kind: "D", height: 1, kids: [], leafReqs: 1 }
    ]
  ).* from "./streamingMirror"
}

// --- positive: R children mixed in at every level where they can occur.
module rMix {
  import streamingMirror(
    ROOT_H = 4, F = 3, CAP_LEVEL = 3, NSC = 7,
    AX_W = true, AX_D1_ROOT = true, AX_D1_INT = true, AX_D2 = true, AX_D3 = true, WIRE_FIRST = false,
    SKEL = [
      { kind: "D", height: 4, kids: [1, 2], leafReqs: 0 },
      { kind: "D", height: 3, kids: [3, 4], leafReqs: 0 },
      { kind: "R", height: 3, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [5, 6], leafReqs: 0 },
      { kind: "R", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 1, kids: [], leafReqs: 2 },
      { kind: "R", height: 1, kids: [], leafReqs: 0 }
    ]
  ).* from "./streamingMirror"
}

// --- the tightness family: root fans out to 2 parents, each disputing a
// full fan of 4 children that resolve immediately (scaled analog of the
// Rust [32, 256] pyramid witness). N = 4 completing children per parent:
// completes iff CAP_LEVEL >= N - 2 = 2 (MODEL.md §8).
module pyramidFull {   // CAP_LEVEL = F: production stance, must be safe
  import streamingMirror(
    ROOT_H = 4, F = 4, CAP_LEVEL = 4, NSC = 11,
    AX_W = true, AX_D1_ROOT = true, AX_D1_INT = true, AX_D2 = true, AX_D3 = true, WIRE_FIRST = false,
    SKEL = [
      { kind: "D", height: 4, kids: [1, 2], leafReqs: 0 },
      { kind: "D", height: 3, kids: [3, 4, 5, 6], leafReqs: 0 },
      { kind: "D", height: 3, kids: [7, 8, 9, 10], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 }
    ]
  ).* from "./streamingMirror"
}

module pyramidC2 {   // N - 2: must still be safe under every schedule
  import streamingMirror(
    ROOT_H = 4, F = 4, CAP_LEVEL = 2, NSC = 11,
    AX_W = true, AX_D1_ROOT = true, AX_D1_INT = true, AX_D2 = true, AX_D3 = true, WIRE_FIRST = false,
    SKEL = [
      { kind: "D", height: 4, kids: [1, 2], leafReqs: 0 },
      { kind: "D", height: 3, kids: [3, 4, 5, 6], leafReqs: 0 },
      { kind: "D", height: 3, kids: [7, 8, 9, 10], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 }
    ]
  ).* from "./streamingMirror"
}

module pyramidC1 {   // N - 3: the stall witness — stuck must be reachable
  import streamingMirror(
    ROOT_H = 4, F = 4, CAP_LEVEL = 1, NSC = 11,
    AX_W = true, AX_D1_ROOT = true, AX_D1_INT = true, AX_D2 = true, AX_D3 = true, WIRE_FIRST = false,
    SKEL = [
      { kind: "D", height: 4, kids: [1, 2], leafReqs: 0 },
      { kind: "D", height: 3, kids: [3, 4, 5, 6], leafReqs: 0 },
      { kind: "D", height: 3, kids: [7, 8, 9, 10], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 }
    ]
  ).* from "./streamingMirror"
}

// --- positive at D=5 (ROOT_H=6): a comb with a dispute branching at
// every height, exercising the internal->internal handoff on both parties
// (scaled analog of full_depth_comb_pair).
module comb6 {
  import streamingMirror(
    ROOT_H = 6, F = 2, CAP_LEVEL = 2, NSC = 8,
    AX_W = true, AX_D1_ROOT = true, AX_D1_INT = true, AX_D2 = true, AX_D3 = true, WIRE_FIRST = false,
    SKEL = [
      { kind: "D", height: 6, kids: [1], leafReqs: 0 },
      { kind: "D", height: 5, kids: [2, 3], leafReqs: 0 },
      { kind: "D", height: 4, kids: [4, 5], leafReqs: 0 },
      { kind: "D", height: 4, kids: [], leafReqs: 0 },
      { kind: "D", height: 3, kids: [6], leafReqs: 0 },
      { kind: "R", height: 3, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [7], leafReqs: 0 },
      { kind: "D", height: 1, kids: [], leafReqs: 2 }
    ]
  ).* from "./streamingMirror"
}

// --- N1: drop Axiom W. Prediction (plan): 3-process cross-party cycle at
// F=2, D=3 — ROpen commits to queries before the root wire reply; the
// second query blocks behind a starved responder walk. Stuck reachable.
module n1DropW {
  import streamingMirror(
    ROOT_H = 4, F = 2, CAP_LEVEL = 2, NSC = 5,
    AX_W = false, AX_D1_ROOT = true, AX_D1_INT = true, AX_D2 = true, AX_D3 = true, WIRE_FIRST = false,
    SKEL = [
      { kind: "D", height: 4, kids: [1, 2], leafReqs: 0 },
      { kind: "D", height: 3, kids: [3], leafReqs: 0 },
      { kind: "D", height: 3, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [4], leafReqs: 0 },
      { kind: "D", height: 1, kids: [], leafReqs: 1 }
    ]
  ).* from "./streamingMirror"
}

// --- N2: drop Axiom D1 at the root only. Prediction: the responder root
// return boundary (rootReturns, cap 1) jams while ROpen still owes
// queries; deadlock appears at root fan 6, not at fan 5.
module n2fan6 {
  import streamingMirror(
    ROOT_H = 4, F = 6, CAP_LEVEL = 6, NSC = 7,
    AX_W = true, AX_D1_ROOT = false, AX_D1_INT = true, AX_D2 = true, AX_D3 = true, WIRE_FIRST = true,
    SKEL = [
      { kind: "D", height: 4, kids: [1, 2, 3, 4, 5, 6], leafReqs: 0 },
      { kind: "D", height: 3, kids: [], leafReqs: 0 },
      { kind: "D", height: 3, kids: [], leafReqs: 0 },
      { kind: "D", height: 3, kids: [], leafReqs: 0 },
      { kind: "D", height: 3, kids: [], leafReqs: 0 },
      { kind: "D", height: 3, kids: [], leafReqs: 0 },
      { kind: "D", height: 3, kids: [], leafReqs: 0 }
    ]
  ).* from "./streamingMirror"
}

module n2fan5 {
  import streamingMirror(
    ROOT_H = 4, F = 5, CAP_LEVEL = 5, NSC = 6,
    AX_W = true, AX_D1_ROOT = false, AX_D1_INT = true, AX_D2 = true, AX_D3 = true, WIRE_FIRST = true,
    SKEL = [
      { kind: "D", height: 4, kids: [1, 2, 3, 4, 5], leafReqs: 0 },
      { kind: "D", height: 3, kids: [], leafReqs: 0 },
      { kind: "D", height: 3, kids: [], leafReqs: 0 },
      { kind: "D", height: 3, kids: [], leafReqs: 0 },
      { kind: "D", height: 3, kids: [], leafReqs: 0 },
      { kind: "D", height: 3, kids: [], leafReqs: 0 }
    ]
  ).* from "./streamingMirror"
}

// --- N3: drop Axiom D1 internally (with WIRE_FIRST isolating the
// resolution-vs-query reorder). Outcome, matching the plan's prediction:
// NO deadlock at documented capacities — and, beyond the prediction, none
// at reduced capacity either (n3Reduced): with D3 and per-channel order
// in force, the early-query relaxation cannot pile more returns than the
// N <= C+2 law already governs. Internally, D1 appears not to be
// individually load-bearing at all; its progress role is at the root's
// cap-1 return boundary (see n2fan6/n2fan5). Phase B/C should prove or
// refute this conjecture parametrically.
module n3Internal {
  import streamingMirror(
    ROOT_H = 4, F = 4, CAP_LEVEL = 4, NSC = 7,
    AX_W = true, AX_D1_ROOT = true, AX_D1_INT = false, AX_D2 = true, AX_D3 = true, WIRE_FIRST = true,
    SKEL = [
      { kind: "D", height: 4, kids: [1], leafReqs: 0 },
      { kind: "D", height: 3, kids: [2, 3, 4, 5], leafReqs: 0 },
      { kind: "D", height: 2, kids: [6], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 1, kids: [], leafReqs: 1 }
    ]
  ).* from "./streamingMirror"
}

// Same relaxation at the tightness boundary CAP_LEVEL = N-2: still
// completes (see the N3 note above; expectation: safe).
module n3Reduced {
  import streamingMirror(
    ROOT_H = 4, F = 4, CAP_LEVEL = 2, NSC = 7,
    AX_W = true, AX_D1_ROOT = true, AX_D1_INT = false, AX_D2 = true, AX_D3 = true, WIRE_FIRST = true,
    SKEL = [
      { kind: "D", height: 4, kids: [1], leafReqs: 0 },
      { kind: "D", height: 3, kids: [2, 3, 4, 5], leafReqs: 0 },
      { kind: "D", height: 2, kids: [6], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 1, kids: [], leafReqs: 1 }
    ]
  ).* from "./streamingMirror"
}

// --- N4: drop Axiom D2 (parent resolution may precede its children's).
// Prediction: likely no reachable deadlock at small instances; if
// confirmed, that slack gets reported against the materialized.rs docs.
module n4DropD2 {
  import streamingMirror(
    ROOT_H = 4, F = 3, CAP_LEVEL = 3, NSC = 7,
    AX_W = true, AX_D1_ROOT = true, AX_D1_INT = true, AX_D2 = false, AX_D3 = true, WIRE_FIRST = false,
    SKEL = [
      { kind: "D", height: 4, kids: [1, 2], leafReqs: 0 },
      { kind: "D", height: 3, kids: [3, 4], leafReqs: 0 },
      { kind: "D", height: 3, kids: [5], leafReqs: 0 },
      { kind: "D", height: 2, kids: [6], leafReqs: 0 },
      { kind: "R", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 1, kids: [], leafReqs: 2 }
    ]
  ).* from "./streamingMirror"
}

// --- Finding-control: a bare D1 drop (no wire-first scaffolding) admits
// deadlock already at root fan 2, via ROpen committing to a query before
// the root wire reply — the wire ledger never constrains DependentWork,
// so W does not order dependent work after the wire. This is the
// unrestricted form of N2/N3; the scaffolded modules above isolate the
// resolution-late cycle the docs' one-slot argument is about.
module n2unrestricted {
  import streamingMirror(
    ROOT_H = 4, F = 2, CAP_LEVEL = 2, NSC = 3,
    AX_W = true, AX_D1_ROOT = false, AX_D1_INT = true, AX_D2 = true, AX_D3 = true, WIRE_FIRST = false,
    SKEL = [
      { kind: "D", height: 4, kids: [1, 2], leafReqs: 0 },
      { kind: "D", height: 3, kids: [], leafReqs: 0 },
      { kind: "D", height: 3, kids: [], leafReqs: 0 }
    ]
  ).* from "./streamingMirror"
}

// --- The D3 gap witness (the N5 control): fan-with-depth shape with the
// three original ledger axioms on and D3 off. A linearization consistent
// with those three (wires, then resolutions, deferring child 0's queries)
// jams the cap-1 lower channel. This instance is why assert_valid gained
// its sibling-contiguity check (2026-07-15): before that check, a
// ledger-passing implementation could deadlock. Expected: stuck reachable.
module ledgerGap {
  import streamingMirror(
    ROOT_H = 4, F = 4, CAP_LEVEL = 4, NSC = 7,
    AX_W = true, AX_D1_ROOT = true, AX_D1_INT = true, AX_D2 = true, AX_D3 = false, WIRE_FIRST = false,
    SKEL = [
      { kind: "D", height: 4, kids: [1], leafReqs: 0 },
      { kind: "D", height: 3, kids: [2, 3, 4, 5], leafReqs: 0 },
      { kind: "D", height: 2, kids: [6], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 1, kids: [], leafReqs: 1 }
    ]
  ).* from "./streamingMirror"
}

// --- The positive twin of ledgerGap: with Axiom D3 restored, the same
// shape is safe. Together they pin D3 as load-bearing.
module fanDepthPositive {
  import streamingMirror(
    ROOT_H = 4, F = 4, CAP_LEVEL = 4, NSC = 7,
    AX_W = true, AX_D1_ROOT = true, AX_D1_INT = true, AX_D2 = true, AX_D3 = true, WIRE_FIRST = false,
    SKEL = [
      { kind: "D", height: 4, kids: [1], leafReqs: 0 },
      { kind: "D", height: 3, kids: [2, 3, 4, 5], leafReqs: 0 },
      { kind: "D", height: 2, kids: [6], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 2, kids: [], leafReqs: 0 },
      { kind: "D", height: 1, kids: [], leafReqs: 1 }
    ]
  ).* from "./streamingMirror"
}
