// Typed wrapper over the wasm engine. The engine owns the op-log and the causal-history
// DAG; gesture methods rewind+append and return the new derived state. The wasm boundary
// hands us JSON; strict TypeScript treats it as `unknown`, so we validate and narrow it
// into the typed model here. Gesture methods throw if the engine rejects the op (e.g. an
// overlapping join); the caller declines to commit.

import init, { Engine as WasmEngine } from "../pkg/itc_viz.js";
import { asNodeIdx, type Edge, type EdgeKind, type NodeDescriptor, type NodeIdx, type State } from "./types";

export class Engine {
  private constructor(private readonly wasm: WasmEngine) {}

  /// Load the wasm module and construct an engine (seeded). Must be awaited before use.
  static async create(): Promise<Engine> {
    await init({ module_or_path: new URL("../pkg/itc_viz_bg.wasm", import.meta.url) });
    return new Engine(new WasmEngine());
  }

  tick(x: NodeIdx): State {
    return parseState(this.wasm.tick(x));
  }
  fork(x: NodeIdx): State {
    return parseState(this.wasm.fork(x));
  }
  join(a: NodeIdx, b: NodeIdx): State {
    return parseState(this.wasm.join(a, b));
  }
  send(from: NodeIdx, to: NodeIdx): State {
    return parseState(this.wasm.send(from, to));
  }

  /// Replace the state from a URL fragment (empty string → the seed).
  load(fragment: string): State {
    return parseState(this.wasm.load(fragment));
  }

  /// The current op-log as a URL fragment.
  fragment(): string {
    return this.wasm.fragment();
  }

  /// Whether nodes `a` and `b` have disjoint ids (join validity, for drag feedback).
  isDisjoint(a: NodeIdx, b: NodeIdx): boolean {
    return this.wasm.is_disjoint(a, b);
  }
}

const EDGE_KINDS: ReadonlySet<string> = new Set<EdgeKind>(["event", "forkjoin", "message"]);

function parseState(json: string): State {
  const parsed: unknown = JSON.parse(json);
  if (typeof parsed !== "object" || parsed === null) throw new Error("engine returned a non-object state");
  const record = parsed as Record<string, unknown>;
  const nodes = record["nodes"];
  const edges = record["edges"];
  const live = record["live"];
  if (!Array.isArray(nodes) || !Array.isArray(edges) || !Array.isArray(live)) {
    throw new Error("malformed state payload");
  }
  return {
    nodes: nodes.map(parseNode),
    edges: edges.map(parseEdge),
    live: live.map((i) => {
      if (typeof i !== "number") throw new Error("malformed live index");
      return asNodeIdx(i);
    }),
  };
}

function parseNode(value: unknown, i: number): NodeDescriptor {
  if (typeof value !== "object" || value === null) throw new Error(`node ${i}: not an object`);
  const r = value as Record<string, unknown>;
  const { idx, party, event, stamp } = r;
  if (typeof idx !== "number" || typeof party !== "string" || typeof event !== "string" || typeof stamp !== "string") {
    throw new Error(`node ${i}: malformed descriptor`);
  }
  return { idx: asNodeIdx(idx), party, event, stamp };
}

function parseEdge(value: unknown, i: number): Edge {
  if (typeof value !== "object" || value === null) throw new Error(`edge ${i}: not an object`);
  const r = value as Record<string, unknown>;
  const { from, to, kind } = r;
  if (typeof from !== "number" || typeof to !== "number" || typeof kind !== "string" || !EDGE_KINDS.has(kind)) {
    throw new Error(`edge ${i}: malformed`);
  }
  return { from: asNodeIdx(from), to: asNodeIdx(to), kind: kind as EdgeKind };
}
