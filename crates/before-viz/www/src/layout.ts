// Layout for the causal DAG. History mode: layered top-down — layer = longest path
// from the seed (every edge points strictly down). Live (frontier) nodes are pulled to
// the bottom row so "now" sits together at the leading edge. Within-layer order is
// refined by alternating up/down barycenter sweeps to reduce crossings, then real x
// coordinates come from a median/separation relaxation so a single child sits beneath
// its parent and forks center over their children. Pure.

import { forceCenter, forceCollide, forceManyBody, forceSimulation, forceX, forceY } from "d3-force";

import type { Edge } from "./types";
import { stampHeight, type StampStyle } from "./glyph";
import type { NodeIdx } from "./types";

export interface Point {
  readonly x: number;
  readonly y: number;
}

export interface Layout {
  readonly pos: Map<NodeIdx, Point>;
  readonly width: number;
  readonly height: number;
}

const ORDER_SWEEPS = 6;
const COORD_ITERS = 10;

function mean(xs: number[]): number {
  return xs.reduce((s, v) => s + v, 0) / xs.length;
}

export function layeredLayout(
  count: number,
  edges: readonly Edge[],
  live: ReadonlySet<NodeIdx>,
  cellW: number,
  cellH: number,
): Layout {
  const parents: number[][] = Array.from({ length: count }, () => []);
  const children: number[][] = Array.from({ length: count }, () => []);
  for (const e of edges) {
    parents[e.to]?.push(e.from);
    children[e.from]?.push(e.to);
  }

  // Layer = 1 + max parent layer (edges run low→high index, so one pass suffices).
  const layer = new Array<number>(count).fill(0);
  for (let i = 1; i < count; i++) {
    let best = -1;
    for (const p of parents[i] ?? []) best = Math.max(best, layer[p] ?? 0);
    layer[i] = best + 1;
  }
  // Pull live *sinks* to the bottom row so "now" sits together at the leading edge.
  // Only sinks (no outgoing edge) are pinned: a live sender still has a descendant, so
  // pinning it could create a same-row or upward edge.
  const maxLayer = layer.reduce((m, l) => Math.max(m, l), 0);
  for (let i = 0; i < count; i++) {
    if (live.has(i as NodeIdx) && (children[i]?.length ?? 0) === 0) layer[i] = maxLayer;
  }

  const byLayer: number[][] = Array.from({ length: maxLayer + 1 }, () => []);
  for (let i = 0; i < count; i++) byLayer[layer[i] ?? 0]?.push(i);

  // Within-layer order: alternate down (parents) and up (children) barycenter sweeps.
  const col = new Array<number>(count).fill(0);
  byLayer.forEach((row) => row.forEach((n, c) => (col[n] = c)));
  for (let s = 0; s < ORDER_SWEEPS; s++) {
    const downward = s % 2 === 0;
    const neigh = downward ? parents : children;
    const order = downward ? range(1, maxLayer) : range(maxLayer - 1, 0);
    for (const l of order) {
      const row = byLayer[l];
      if (row === undefined) continue;
      const key = new Map<number, number>();
      for (const n of row) {
        const ns = neigh[n] ?? [];
        key.set(n, ns.length === 0 ? (col[n] ?? 0) : mean(ns.map((m) => col[m] ?? 0)));
      }
      row.sort((a, b) => (key.get(a) ?? 0) - (key.get(b) ?? 0));
      row.forEach((n, c) => (col[n] = c));
    }
  }

  // Real-valued x: relax toward parents (down) and children (up), enforcing min
  // separation left→right within each layer after each move.
  const x = new Array<number>(count).fill(0);
  byLayer.forEach((row) => row.forEach((n, i) => (x[n] = i * cellW)));

  const relax = (rows: number[][], neigh: number[][]): void => {
    for (const row of rows) {
      let prev = Number.NEGATIVE_INFINITY;
      for (const n of row) {
        const ns = neigh[n] ?? [];
        const desired = ns.length === 0 ? (x[n] ?? 0) : mean(ns.map((m) => x[m] ?? 0));
        x[n] = Math.max(desired, prev + cellW);
        prev = x[n] ?? 0;
      }
    }
  };
  const down = byLayer.slice(1);
  const up = byLayer.slice(0, maxLayer).reverse();
  for (let k = 0; k < COORD_ITERS; k++) {
    relax(down, parents);
    relax(up, children);
  }

  const minX = Math.min(...x);
  const maxX = Math.max(...x);
  const shift = cellW / 2 - minX;
  const pos = new Map<NodeIdx, Point>();
  for (let i = 0; i < count; i++) {
    pos.set(i as NodeIdx, { x: (x[i] ?? 0) + shift, y: (layer[i] ?? 0) * cellH + cellH / 2 });
  }
  return { pos, width: maxX - minX + cellW, height: (maxLayer + 1) * cellH };
}

function range(from: number, to: number): number[] {
  const out: number[] = [];
  if (from <= to) for (let i = from; i <= to; i++) out.push(i);
  else for (let i = from; i >= to; i--) out.push(i);
  return out;
}

interface FNode {
  idx: NodeIdx;
  x: number;
  y: number;
  fx?: number;
  fy?: number;
}

/// Tableau layout: only the current live clocks, settled by a force simulation into a
/// pleasant cluster (charge repulsion + collision). For stability across ops, every
/// clock that was already on screen is *pinned* at its previous position, so only newly
/// appeared clocks move — they settle into the gaps around the fixed ones. A fresh
/// (cold) layout, with no prior positions, runs a full centered simulation. Determin-
/// istic (no randomness), run to a fixed tick count.
export function tableauLayout(
  ids: readonly NodeIdx[],
  prev: ReadonlyMap<NodeIdx, Point>,
  style: StampStyle,
  width: number,
  height: number,
): Map<NodeIdx, Point> {
  const radius = Math.max(style.width, stampHeight(style)) * 0.62;
  const survivors = ids.map((id) => prev.get(id)).filter((p): p is Point => p !== undefined);
  const warm = survivors.length > 0;

  // New clocks start near the centroid of the surviving ones (or center, when cold).
  let cx = width / 2;
  let cy = height / 2;
  if (warm) {
    cx = survivors.reduce((s, p) => s + p.x, 0) / survivors.length;
    cy = survivors.reduce((s, p) => s + p.y, 0) / survivors.length;
  }

  const nodes: FNode[] = ids.map((idx, i) => {
    const p = prev.get(idx);
    if (p !== undefined) return { idx, x: p.x, y: p.y, fx: p.x, fy: p.y }; // pin survivors
    return { idx, x: cx + Math.cos(i * 2.4) * 30, y: cy + Math.sin(i * 2.4) * 30 };
  });

  // Warm: pinned survivors don't move; a gentle pull toward the centroid keeps each
  // new clock near the cluster (collision finds it a gap) instead of being flung out by
  // repulsion. Cold: a stronger, centered settle of the whole set.
  const sim = forceSimulation<FNode>(nodes)
    .force("collide", forceCollide<FNode>(radius).iterations(3))
    .force("charge", forceManyBody<FNode>().strength(-radius * (warm ? 2 : 7)))
    .force("x", forceX<FNode>(warm ? cx : width / 2).strength(warm ? 0.13 : 0.06))
    .force("y", forceY<FNode>(warm ? cy : height / 2).strength(warm ? 0.13 : 0.06))
    .stop();
  if (!warm) sim.force("center", forceCenter<FNode>(width / 2, height / 2));

  const ticks = warm ? 120 : 320;
  for (let i = 0; i < ticks; i++) sim.tick();

  const out = new Map<NodeIdx, Point>();
  for (const n of nodes) out.set(n.idx, { x: n.x, y: n.y });
  return out;
}
