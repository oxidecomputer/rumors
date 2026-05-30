// The stamp glyph: the paper's interval + skyline notation. The id (Party) is a thin
// bar along [0,1) — owned leaves filled, unowned hollow, the interval halved per tree
// node. The event (Version) is a stacked skyline over the same interval.
//
// Geometry is pure (unit-space); rendering maps it to SVG at a *shared* style so all
// stamps use one interval width and one baseline, making subdivisions and heights
// comparable across the whole figure. Colors come from CSS classes.

import type { EventTree, IdTree } from "./types";

const SVG_NS = "http://www.w3.org/2000/svg";

/// Fixed vertical metrics (px). Interval width and event unit are shared, computed.
export const METRICS = {
  idBarHeight: 7,
  gap: 5,
  unit: 9, // px per event level
  minLeafPx: 7, // smallest legible id leaf segment
  minWidth: 64,
  maxWidth: 220,
  minHeightUnits: 1, // reserve at least this much skyline room
} as const;

/// The shared paint style for a batch of stamps.
export interface StampStyle {
  readonly width: number; // px for the [0,1) interval
  readonly unit: number; // px per event level
  readonly maxHeight: number; // global tallest skyline, for a shared baseline
}

export interface IdSegment {
  readonly x0: number;
  readonly x1: number;
  readonly owned: boolean;
}
export interface EventSlab {
  readonly x0: number;
  readonly x1: number;
  readonly y0: number;
  readonly y1: number;
}
export interface GlyphGeometry {
  readonly idSegments: IdSegment[];
  readonly slabs: EventSlab[];
  readonly maxHeight: number;
}

function collectId(t: IdTree, x0: number, x1: number, out: IdSegment[]): void {
  if ("leaf" in t) {
    out.push({ x0, x1, owned: t.leaf === 1 });
    return;
  }
  const mid = (x0 + x1) / 2;
  collectId(t.l, x0, mid, out);
  collectId(t.r, mid, x1, out);
}

function collectEvent(t: EventTree, x0: number, x1: number, floor: number, out: EventSlab[]): number {
  const top = floor + t.base;
  if (t.base > 0) out.push({ x0, x1, y0: floor, y1: top });
  let max = top;
  if (t.l !== undefined && t.r !== undefined) {
    const mid = (x0 + x1) / 2;
    max = Math.max(max, collectEvent(t.l, x0, mid, top, out));
    max = Math.max(max, collectEvent(t.r, mid, x1, top, out));
  }
  return max;
}

/// Compute a glyph's geometry. `id` is null for message nodes (history only).
export function glyphGeometry(id: IdTree | null, event: EventTree): GlyphGeometry {
  const idSegments: IdSegment[] = [];
  if (id !== null) collectId(id, 0, 1, idSegments);
  const slabs: EventSlab[] = [];
  const maxHeight = collectEvent(event, 0, 1, 0, slabs);
  return { idSegments, slabs, maxHeight };
}

/// Depth of the id tree (0 for a leaf). The deepest tree fixes the shared width.
export function idDepth(t: IdTree): number {
  if ("leaf" in t) return 0;
  return 1 + Math.max(idDepth(t.l), idDepth(t.r));
}

/// Derive a shared style from all stamps' id/event trees: width wide enough that the
/// finest leaf stays legible (clamped), and the global tallest skyline for a common
/// baseline.
export function computeStampStyle(ids: readonly IdTree[], events: readonly EventTree[]): StampStyle {
  let maxDepth = 0;
  for (const id of ids) maxDepth = Math.max(maxDepth, idDepth(id));
  let maxHeight: number = METRICS.minHeightUnits;
  for (const ev of events) maxHeight = Math.max(maxHeight, glyphGeometry(null, ev).maxHeight);
  const wanted = METRICS.minLeafPx * 2 ** maxDepth;
  const width = Math.min(METRICS.maxWidth, Math.max(METRICS.minWidth, wanted));
  return { width, unit: METRICS.unit, maxHeight };
}

/// Total pixel height of a stamp at a given style (uniform across the batch).
export function stampHeight(style: StampStyle): number {
  return METRICS.idBarHeight + METRICS.gap + style.maxHeight * style.unit;
}

function rect(x: number, y: number, w: number, h: number, className: string): SVGRectElement {
  const r = document.createElementNS(SVG_NS, "rect");
  r.setAttribute("x", x.toFixed(2));
  r.setAttribute("y", y.toFixed(2));
  r.setAttribute("width", Math.max(0, w).toFixed(2));
  r.setAttribute("height", Math.max(0, h).toFixed(2));
  r.setAttribute("class", className);
  return r;
}

/// Build an `<svg>` stamp sized to the shared style. `kind` selects the color
/// treatment; liveness desaturation is driven by the node group's class in CSS.
export function renderStamp(id: IdTree | null, event: EventTree, kind: "clock" | "message", style: StampStyle): SVGSVGElement {
  const geo = glyphGeometry(id, event);
  const W = style.width;
  const H = stampHeight(style);
  const baseline = H - METRICS.idBarHeight - METRICS.gap;

  const svg = document.createElementNS(SVG_NS, "svg");
  svg.setAttribute("width", W.toFixed(2));
  svg.setAttribute("height", H.toFixed(2));
  svg.setAttribute("viewBox", `0 0 ${W.toFixed(2)} ${H.toFixed(2)}`);
  svg.setAttribute("class", `glyph glyph--${kind}`);

  for (const s of geo.slabs) {
    const x = s.x0 * W;
    const w = (s.x1 - s.x0) * W;
    const yTop = baseline - s.y1 * style.unit;
    const h = (s.y1 - s.y0) * style.unit;
    svg.appendChild(rect(x, yTop, w, h, "glyph__slab"));
  }

  if (id !== null) {
    const y = H - METRICS.idBarHeight;
    for (const seg of geo.idSegments) {
      const x = seg.x0 * W;
      const w = (seg.x1 - seg.x0) * W;
      svg.appendChild(rect(x, y, w, METRICS.idBarHeight, seg.owned ? "glyph__id" : "glyph__id glyph__id--empty"));
    }
  }

  return svg;
}
