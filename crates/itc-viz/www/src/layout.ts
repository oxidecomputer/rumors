// Layout for the causal DAG. History mode: layered top-down — layer = longest path
// from the seed (every edge points strictly down), then real-valued x-coordinates from
// an iterative median/separation relaxation so a node with one parent sits directly
// beneath it, forks center over their children, and joins sit between their parents.
// Pure: indices + edges + cell size → positions.

import type { Edge } from "./dag";
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

const ORDER_SWEEPS = 3;
const COORD_ITERS = 8;

function mean(xs: number[]): number {
  return xs.reduce((s, v) => s + v, 0) / xs.length;
}

export function layeredLayout(count: number, edges: readonly Edge[], cellW: number, cellH: number): Layout {
  const parents: number[][] = Array.from({ length: count }, () => []);
  const children: number[][] = Array.from({ length: count }, () => []);
  for (const e of edges) {
    parents[e.to]?.push(e.from);
    children[e.from]?.push(e.to);
  }

  // Layer = 1 + max parent layer (edges always run low→high index, so one pass).
  const layer = new Array<number>(count).fill(0);
  for (let i = 1; i < count; i++) {
    let best = -1;
    for (const p of parents[i] ?? []) best = Math.max(best, layer[p] ?? 0);
    layer[i] = best + 1;
  }
  const maxLayer = layer.reduce((m, l) => Math.max(m, l), 0);

  const byLayer: number[][] = Array.from({ length: maxLayer + 1 }, () => []);
  for (let i = 0; i < count; i++) byLayer[layer[i] ?? 0]?.push(i);

  // Within-layer ORDER (left→right) by barycenter of parents, to reduce crossings.
  const col = new Array<number>(count).fill(0);
  byLayer.forEach((row) => row.forEach((n, c) => (col[n] = c)));
  for (let s = 0; s < ORDER_SWEEPS; s++) {
    for (let l = 1; l <= maxLayer; l++) {
      const row = byLayer[l];
      if (row === undefined) continue;
      const key = new Map<number, number>();
      for (const n of row) {
        const ps = parents[n] ?? [];
        key.set(n, ps.length === 0 ? (col[n] ?? 0) : mean(ps.map((p) => col[p] ?? 0)));
      }
      row.sort((a, b) => (key.get(a) ?? 0) - (key.get(b) ?? 0));
      row.forEach((n, c) => (col[n] = c));
    }
  }

  // Real-valued x. Init by order; then relax toward parents (down) and children (up),
  // enforcing min separation left→right within each layer after each move.
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

  const down = byLayer.slice(1); // layers 1..max, aligning to parents
  const up = byLayer.slice(0, maxLayer).reverse(); // layers max-1..0, aligning to children
  for (let k = 0; k < COORD_ITERS; k++) {
    relax(down, parents);
    relax(up, children);
  }

  // Normalize to a left margin of cellW/2 (room for the half-stamp).
  const minX = Math.min(...x);
  const maxX = Math.max(...x);
  const shift = cellW / 2 - minX;
  const pos = new Map<NodeIdx, Point>();
  for (let i = 0; i < count; i++) {
    pos.set(i as NodeIdx, { x: (x[i] ?? 0) + shift, y: (layer[i] ?? 0) * cellH + cellH / 2 });
  }

  return { pos, width: maxX - minX + cellW, height: (maxLayer + 1) * cellH };
}
