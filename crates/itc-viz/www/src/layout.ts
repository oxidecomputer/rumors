// Layout for the causal DAG. History mode: a layered top-down placement — layer =
// longest path from the seed (so every edge points strictly downward), order within
// a layer by barycenter of parents (to reduce crossings). Pure: indices + edges +
// cell size → positions. (Tableau force-layout is added in a later step.)

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

const BARYCENTER_SWEEPS = 3;

/// Layered top-down layout. `count` is the number of nodes (indices 0…count-1);
/// edges always run from a lower to a higher index (operands precede outputs), which
/// makes the longest-path layering a single forward pass.
export function layeredLayout(count: number, edges: readonly Edge[], cellW: number, cellH: number): Layout {
  const parents: number[][] = Array.from({ length: count }, () => []);
  for (const e of edges) parents[e.to]?.push(e.from);

  // Layer = 1 + max parent layer (seed and any parentless node sit at layer 0).
  const layer = new Array<number>(count).fill(0);
  for (let i = 1; i < count; i++) {
    let best = -1;
    for (const p of parents[i] ?? []) best = Math.max(best, layer[p] ?? 0);
    layer[i] = best + 1;
  }

  const maxLayer = layer.reduce((m, l) => Math.max(m, l), 0);
  const byLayer: number[][] = Array.from({ length: maxLayer + 1 }, () => []);
  for (let i = 0; i < count; i++) byLayer[layer[i] ?? 0]?.push(i);

  // Column of each node within its layer; refined by barycenter sweeps over parents.
  const col = new Array<number>(count).fill(0);
  const reindex = (row: number[]): void => row.forEach((n, c) => (col[n] = c));
  byLayer.forEach(reindex);

  for (let sweep = 0; sweep < BARYCENTER_SWEEPS; sweep++) {
    for (let l = 1; l <= maxLayer; l++) {
      const row = byLayer[l];
      if (row === undefined) continue;
      const bary = new Map<number, number>();
      for (const n of row) {
        const ps = parents[n] ?? [];
        const mean = ps.length === 0 ? col[n] ?? 0 : ps.reduce((s, p) => s + (col[p] ?? 0), 0) / ps.length;
        bary.set(n, mean);
      }
      row.sort((a, b) => (bary.get(a) ?? 0) - (bary.get(b) ?? 0));
      reindex(row);
    }
  }

  const widthCols = byLayer.reduce((m, row) => Math.max(m, row.length), 0);
  const width = Math.max(1, widthCols) * cellW;

  const pos = new Map<NodeIdx, Point>();
  byLayer.forEach((row, l) => {
    const startX = (width - row.length * cellW) / 2;
    row.forEach((n, c) => {
      pos.set(n as NodeIdx, { x: startX + c * cellW + cellW / 2, y: l * cellH + cellH / 2 });
    });
  });

  return { pos, width, height: (maxLayer + 1) * cellH };
}
