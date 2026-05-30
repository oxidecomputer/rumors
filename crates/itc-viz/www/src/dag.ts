// Pure derivations from the op-log: the causal edge set, node liveness, the
// descendant cone of a node, and the "rewind to a past node then apply" log
// rewrite. The op-log is the single source of truth; nothing here touches the DOM
// or the wasm engine. Node indices match the engine's replay order exactly: index
// 0 is the implicit seed, then each op appends its outputs in order.

import { asNodeIdx, type EdgeKind, type NodeDescriptor, type NodeIdx, type Op, type OpLog } from "./types";

export interface Edge {
  readonly from: NodeIdx;
  readonly to: NodeIdx;
  readonly kind: EdgeKind;
}

interface Analysis {
  /// Causal edges (event / forkjoin / message), in creation order.
  readonly edges: Edge[];
  /// Indices each op produced; `perOp[i]` are the nodes appended by `log[i]`.
  readonly perOp: NodeIdx[][];
  /// Clock nodes that were consumed by a later tick/fork/join/merge-target.
  readonly superseded: Set<NodeIdx>;
  /// Message nodes that were consumed by their one merge.
  readonly consumed: Set<NodeIdx>;
  /// Total node count (seed + all op outputs).
  readonly nodeCount: number;
}

/// Single forward pass over the log, deriving edges, produced-index lists, and the
/// superseded/consumed sets — everything else is computed from this.
function analyze(log: OpLog): Analysis {
  const edges: Edge[] = [];
  const perOp: NodeIdx[][] = [];
  const superseded = new Set<NodeIdx>();
  const consumed = new Set<NodeIdx>();
  let next = 1; // node 0 is the seed

  const fresh = (): NodeIdx => asNodeIdx(next++);
  const edge = (from: NodeIdx, to: NodeIdx, kind: EdgeKind): void => {
    edges.push({ from, to, kind });
  };

  for (const op of log) {
    switch (op.kind) {
      case "tick": {
        const out = fresh();
        edge(op.x, out, "event");
        superseded.add(op.x);
        perOp.push([out]);
        break;
      }
      case "fork": {
        const a = fresh();
        const b = fresh();
        edge(op.x, a, "forkjoin");
        edge(op.x, b, "forkjoin");
        superseded.add(op.x);
        perOp.push([a, b]);
        break;
      }
      case "join": {
        const out = fresh();
        edge(op.a, out, "forkjoin");
        edge(op.b, out, "forkjoin");
        superseded.add(op.a);
        superseded.add(op.b);
        perOp.push([out]);
        break;
      }
      case "peek": {
        const msg = fresh();
        edge(op.x, msg, "message");
        // peek does NOT supersede its source
        perOp.push([msg]);
        break;
      }
      case "merge": {
        const out = fresh();
        edge(op.t, out, "event");
        edge(op.m, out, "message");
        superseded.add(op.t);
        consumed.add(op.m);
        perOp.push([out]);
        break;
      }
    }
  }

  return { edges, perOp, superseded, consumed, nodeCount: next };
}

/// The causal edges for a log.
export function deriveEdges(log: OpLog): Edge[] {
  return analyze(log).edges;
}

/// The set of live node indices: clocks not yet superseded, messages not yet
/// consumed. Live nodes are the colored, interactive frontier.
export function liveNodes(log: OpLog, nodes: readonly NodeDescriptor[]): Set<NodeIdx> {
  const { superseded, consumed } = analyze(log);
  const live = new Set<NodeIdx>();
  for (const n of nodes) {
    const isLive = n.kind === "clock" ? !superseded.has(n.idx) : !consumed.has(n.idx);
    if (isLive) live.add(n.idx);
  }
  return live;
}

/// Every node reachable from `x` along any edge (excluding `x` itself): the future
/// that depends on `x`.
export function descendantCone(edges: readonly Edge[], x: NodeIdx): Set<NodeIdx> {
  const out = new Map<NodeIdx, NodeIdx[]>();
  for (const e of edges) {
    const list = out.get(e.from);
    if (list === undefined) out.set(e.from, [e.to]);
    else list.push(e.to);
  }
  const cone = new Set<NodeIdx>();
  const stack: NodeIdx[] = [...(out.get(x) ?? [])];
  while (stack.length > 0) {
    const n = stack.pop();
    if (n === undefined || cone.has(n)) continue;
    cone.add(n);
    for (const m of out.get(n) ?? []) stack.push(m);
  }
  return cone;
}

/// Rewind history to `target` (drop its descendant cone) and append `newOp`, which
/// references the *current* indices of `target` and any other operands. Returns a
/// canonical minimal op-log producing exactly the surviving DAG plus the new
/// frontier. When `target` is a live tip the cone is empty and this is a plain
/// append.
export function rewindAndApply(log: OpLog, target: NodeIdx, makeOp: (remap: (i: NodeIdx) => NodeIdx) => Op): OpLog {
  const { edges, perOp } = analyze(log);
  const cone = descendantCone(edges, target);

  // Keep ops whose outputs all survive; the cone is reachability-closed, so any
  // kept op's operands also survive. Build the old→new index remap as we go.
  const remap = new Map<NodeIdx, NodeIdx>();
  remap.set(asNodeIdx(0), asNodeIdx(0)); // seed always survives
  const kept: Op[] = [];
  let next = 1;

  for (let i = 0; i < log.length; i++) {
    const produced = perOp[i];
    const op = log[i];
    if (produced === undefined || op === undefined) continue;
    if (produced.some((idx) => cone.has(idx))) continue; // dropped with its cone
    kept.push(remapOp(op, remap));
    for (const idx of produced) remap.set(idx, asNodeIdx(next++));
  }

  const lookup = (i: NodeIdx): NodeIdx => remap.get(i) ?? i;
  return [...kept, makeOp(lookup)];
}

function remapOp(op: Op, remap: Map<NodeIdx, NodeIdx>): Op {
  const r = (i: NodeIdx): NodeIdx => remap.get(i) ?? i;
  switch (op.kind) {
    case "tick":
      return { kind: "tick", x: r(op.x) };
    case "fork":
      return { kind: "fork", x: r(op.x) };
    case "join":
      return { kind: "join", a: r(op.a), b: r(op.b) };
    case "peek":
      return { kind: "peek", x: r(op.x) };
    case "merge":
      return { kind: "merge", t: r(op.t), m: r(op.m) };
  }
}
