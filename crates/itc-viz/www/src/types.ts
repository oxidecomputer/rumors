// Core domain types for the ITC visualizer, mirroring the Rust engine's `Op` and
// `Descriptor` types. The op-log is the single source of truth; everything else is
// derived from it.

/// A node's creation-order index. Branded so a raw `number` can't be passed where a
/// node index is expected.
export type NodeIdx = number & { readonly __brand: "NodeIdx" };

/// Treat a number as a node index. Replay assigns indices in creation order, so this
/// is only sound for values produced by the engine / op-log.
export function asNodeIdx(n: number): NodeIdx {
  return n as NodeIdx;
}

/// A materialized node, as returned by the engine. Every node is a clock: an id share
/// plus its history, with the combined stamp.
export type NodeDescriptor = {
  readonly idx: NodeIdx;
  readonly party: string;
  readonly event: string;
  readonly stamp: string;
};

/// The three kinds of causal edge. A `message` edge runs from a sender to the receiver's
/// updated clock (a sent version), with no node between.
export type EdgeKind = "event" | "forkjoin" | "message";

export type Edge = {
  readonly from: NodeIdx;
  readonly to: NodeIdx;
  readonly kind: EdgeKind;
};

/// The full derived state the engine returns after each change: the nodes, the causal
/// edges, and the live (current-frontier) node indices.
export type State = {
  readonly nodes: readonly NodeDescriptor[];
  readonly edges: readonly Edge[];
  readonly live: readonly NodeIdx[];
};

/// A parsed id tree: a leaf (owned `1` / unowned `0`) or an internal split.
export type IdTree = { readonly leaf: 0 | 1 } | { readonly l: IdTree; readonly r: IdTree };

/// A parsed event tree: a base height plus optional left/right subtrees over the
/// halved interval.
export type EventTree = {
  readonly base: number;
  readonly l?: EventTree;
  readonly r?: EventTree;
};
