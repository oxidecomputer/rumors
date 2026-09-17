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

/// One constant-ownership region in the unit interval.
export type PartyRegion = {
  readonly owned: boolean;
  readonly depth: number;
};

/// One constant-height plateau in the unit interval.
export type VersionPlateau = {
  readonly rise: number;
  readonly depth: number;
};

/// A materialized clock node returned by the engine.
export type NodeDescriptor = {
  readonly idx: NodeIdx;
  readonly party: readonly PartyRegion[];
  readonly version: readonly VersionPlateau[];
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
