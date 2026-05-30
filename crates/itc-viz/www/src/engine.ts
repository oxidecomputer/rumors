// Typed wrapper over the wasm replay engine. The wasm boundary hands us JSON; strict
// TypeScript treats `JSON.parse` as `unknown`, so we validate and narrow it into the
// typed model here, keeping the rest of the app fully typed.

import init, { Engine as WasmEngine } from "../pkg/itc_viz.js";
import { asNodeIdx, type NodeDescriptor, type NodeIdx, type OpLog } from "./types";

/// A replay error surfaced by the engine (e.g. an overlapping join). The offending op
/// is simply not committed to the log.
export class ReplayError extends Error {}

export class Engine {
  private constructor(private readonly wasm: WasmEngine) {}

  /// Load the wasm module and construct an engine. Must be awaited before use.
  static async create(): Promise<Engine> {
    await init();
    return new Engine(new WasmEngine());
  }

  /// Replay an op-log (implicit leading seed) and return the materialized nodes in
  /// creation order. Throws `ReplayError` if the engine rejects an op.
  replay(log: OpLog): NodeDescriptor[] {
    let json: string;
    try {
      json = this.wasm.replay(JSON.stringify(log));
    } catch (cause) {
      throw new ReplayError(String(cause));
    }
    return parseDescriptors(json);
  }

  /// Whether nodes `a` and `b` are clocks with disjoint ids (a join would succeed).
  isDisjoint(a: NodeIdx, b: NodeIdx): boolean {
    return this.wasm.is_disjoint(a, b);
  }
}

/// Validate the engine's JSON payload into `NodeDescriptor[]`, rejecting anything that
/// doesn't match the expected shape rather than trusting the boundary.
function parseDescriptors(json: string): NodeDescriptor[] {
  const parsed: unknown = JSON.parse(json);
  if (!Array.isArray(parsed)) {
    throw new ReplayError("engine returned a non-array payload");
  }
  return parsed.map((value, i) => parseDescriptor(value, i));
}

function parseDescriptor(value: unknown, i: number): NodeDescriptor {
  if (typeof value !== "object" || value === null) {
    throw new ReplayError(`node ${i}: not an object`);
  }
  const record = value as Record<string, unknown>;
  const idx = record["idx"];
  const kind = record["kind"];
  const event = record["event"];
  if (typeof idx !== "number" || typeof event !== "string") {
    throw new ReplayError(`node ${i}: missing idx/event`);
  }
  if (kind === "clock") {
    const party = record["party"];
    const stamp = record["stamp"];
    if (typeof party !== "string" || typeof stamp !== "string") {
      throw new ReplayError(`clock node ${i}: missing party/stamp`);
    }
    return { idx: asNodeIdx(idx), kind: "clock", party, event, stamp };
  }
  if (kind === "message") {
    return { idx: asNodeIdx(idx), kind: "message", party: null, event };
  }
  throw new ReplayError(`node ${i}: unknown kind ${String(kind)}`);
}
