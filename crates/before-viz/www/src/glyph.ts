// A clock glyph: the party is a thin ownership bar over [0,1), and the version is a
// skyline over the same interval.
//
// Geometry is pure (unit-space); rendering maps it to SVG at a *shared* style so all
// stamps use one interval width and one baseline, making subdivisions and heights
// comparable across the whole figure. Colors come from CSS classes.

import type { PartyRegion, VersionPlateau } from "./types";

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
  maxSkylinePx: 120, // cap total skyline height; deep towers compress the unit
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

function collectParty(regions: readonly PartyRegion[]): IdSegment[] {
  const out: IdSegment[] = [];
  let x = 0;
  for (const region of regions) {
    const next = x + 2 ** -region.depth;
    out.push({ x0: x, x1: next, owned: region.owned });
    x = next;
  }
  return out;
}

function collectVersion(plateaus: readonly VersionPlateau[]): { slabs: EventSlab[]; maxHeight: number } {
  const slabs: EventSlab[] = [];
  let x = 0;
  let height = 0;
  let maxHeight = 0;
  for (const plateau of plateaus) {
    height += plateau.rise;
    const next = x + 2 ** -plateau.depth;
    if (height > 0) slabs.push({ x0: x, x1: next, y0: 0, y1: height });
    maxHeight = Math.max(maxHeight, height);
    x = next;
  }
  return { slabs, maxHeight };
}

/// Compute a glyph's geometry. `party` is null for message nodes.
export function glyphGeometry(party: readonly PartyRegion[] | null, version: readonly VersionPlateau[]): GlyphGeometry {
  const idSegments = party === null ? [] : collectParty(party);
  const { slabs, maxHeight } = collectVersion(version);
  return { idSegments, slabs, maxHeight };
}

/// The deepest party region, which fixes the shared width.
export function partyDepth(regions: readonly PartyRegion[]): number {
  return regions.reduce((depth, region) => Math.max(depth, region.depth), 0);
}

/// Derive a shared style from all clocks: width wide enough that the
/// finest leaf stays legible (clamped), and the global tallest skyline for a common
/// baseline.
export function computeStampStyle(parties: readonly (readonly PartyRegion[])[], versions: readonly (readonly VersionPlateau[])[]): StampStyle {
  let maxDepth = 0;
  for (const party of parties) maxDepth = Math.max(maxDepth, partyDepth(party));
  let maxHeight: number = METRICS.minHeightUnits;
  for (const version of versions) maxHeight = Math.max(maxHeight, glyphGeometry(null, version).maxHeight);
  const wanted = METRICS.minLeafPx * 2 ** maxDepth;
  const width = Math.min(METRICS.maxWidth, Math.max(METRICS.minWidth, wanted));
  // Compress the per-level unit so even a tall tower's skyline stays bounded (all
  // stamps share the unit, so heights remain comparable).
  const unit = Math.min(METRICS.unit, METRICS.maxSkylinePx / maxHeight);
  return { width, unit, maxHeight };
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

/// Build an `<svg>` stamp sized to the shared style. A null `party` (a bare version, as
/// used by the send ghost) omits the ownership bar. Liveness desaturation is driven by the
/// node group's class in CSS.
export function renderStamp(party: readonly PartyRegion[] | null, version: readonly VersionPlateau[], style: StampStyle): SVGSVGElement {
  const geo = glyphGeometry(party, version);
  const W = style.width;
  const H = stampHeight(style);
  const baseline = H - METRICS.idBarHeight - METRICS.gap;

  const svg = document.createElementNS(SVG_NS, "svg");
  svg.setAttribute("width", W.toFixed(2));
  svg.setAttribute("height", H.toFixed(2));
  svg.setAttribute("viewBox", `0 0 ${W.toFixed(2)} ${H.toFixed(2)}`);
  svg.setAttribute("class", "glyph");

  for (const s of geo.slabs) {
    const x = s.x0 * W;
    const w = (s.x1 - s.x0) * W;
    const yTop = baseline - s.y1 * style.unit;
    const h = (s.y1 - s.y0) * style.unit;
    svg.appendChild(rect(x, yTop, w, h, "glyph__slab"));
  }

  if (party !== null) {
    const y = H - METRICS.idBarHeight;
    for (const seg of geo.idSegments) {
      const x = seg.x0 * W;
      const w = (seg.x1 - seg.x0) * W;
      svg.appendChild(rect(x, y, w, METRICS.idBarHeight, seg.owned ? "glyph__id" : "glyph__id glyph__id--empty"));
    }
  }

  return svg;
}
