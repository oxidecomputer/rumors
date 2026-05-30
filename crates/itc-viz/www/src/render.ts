// Render the causal DAG as one SVG scene. Structure (nodes, edges) is built once per
// op; positions are applied separately (and re-applied each animation frame) via
// `applyPositions`, so node transforms and edge endpoints stay in sync while the graph
// morphs. Pure DOM construction from already-derived data.

import type { Edge } from "./dag";
import { renderStamp, stampHeight, type StampStyle } from "./glyph";
import type { Point } from "./layout";
import { parseEvent, parseId } from "./notation";
import type { NodeDescriptor, NodeIdx } from "./types";

const SVG_NS = "http://www.w3.org/2000/svg";

interface EdgeEl {
  readonly el: SVGPathElement;
  readonly from: NodeIdx;
  readonly to: NodeIdx;
}

export interface Scene {
  readonly svg: SVGSVGElement;
  readonly nodeEls: Map<NodeIdx, SVGGElement>;
  readonly edges: EdgeEl[];
  readonly w: number;
  readonly h: number;
}

export interface RenderInput {
  readonly nodes: readonly NodeDescriptor[];
  readonly edges: readonly Edge[];
  readonly live: ReadonlySet<NodeIdx>;
  readonly style: StampStyle;
  readonly width: number;
  readonly height: number;
}

function el<K extends keyof SVGElementTagNameMap>(name: K, attrs: Record<string, string>): SVGElementTagNameMap[K] {
  const node = document.createElementNS(SVG_NS, name);
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
  return node;
}

function arrowMarker(): SVGMarkerElement {
  const marker = el("marker", {
    id: "arrow",
    viewBox: "0 0 8 8",
    refX: "7",
    refY: "4",
    markerWidth: "7",
    markerHeight: "7",
    orient: "auto-start-reverse",
  });
  marker.appendChild(el("path", { d: "M0,0 L8,4 L0,8 z", class: "edge__arrow" }));
  return marker;
}

export function renderDag(input: RenderInput): Scene {
  const h = stampHeight(input.style);
  const w = input.style.width;

  const svg = el("svg", {
    class: "scene",
    width: `${input.width}`,
    height: `${input.height}`,
    viewBox: `0 0 ${input.width} ${input.height}`,
  });

  const defs = el("defs", {});
  defs.appendChild(arrowMarker());
  svg.appendChild(defs);

  const edgeLayer = el("g", { class: "edges" });
  const edges: EdgeEl[] = [];
  for (const e of input.edges) {
    const path = el("path", { class: `edge edge--${e.kind}`, d: "" });
    if (e.kind === "forkjoin") path.setAttribute("marker-end", "url(#arrow)");
    edgeLayer.appendChild(path);
    edges.push({ el: path, from: e.from, to: e.to });
  }
  svg.appendChild(edgeLayer);

  const nodeLayer = el("g", { class: "nodes" });
  const nodeEls = new Map<NodeIdx, SVGGElement>();
  for (const n of input.nodes) {
    const live = input.live.has(n.idx);
    const g = el("g", { class: `node node--${n.kind}${live ? " node--live" : " node--historical"}` });
    g.dataset["idx"] = `${n.idx}`;
    g.dataset["kind"] = n.kind;
    g.dataset["live"] = live ? "1" : "0";

    const title = el("title", {});
    title.textContent = n.kind === "clock" ? n.stamp : `message ${n.event}`;
    g.appendChild(title);
    g.appendChild(el("rect", { class: "node__hit", x: "0", y: "0", width: `${w}`, height: `${h}` }));

    const id = n.kind === "clock" ? parseId(n.party) : null;
    g.appendChild(renderStamp(id, parseEvent(n.event), n.kind, input.style, !live));

    const chip = el("text", { class: "node__index", x: "1", y: "-3" });
    chip.textContent = `${n.idx}`;
    g.appendChild(chip);

    nodeLayer.appendChild(g);
    nodeEls.set(n.idx, g);
  }
  svg.appendChild(nodeLayer);

  return { svg, nodeEls, edges, w, h };
}

/// Position nodes (top-left = center − half-extent) and route edges (parent
/// bottom-center → child top-center) from a position map. Called per animation frame.
export function applyPositions(scene: Scene, pos: ReadonlyMap<NodeIdx, Point>): void {
  for (const [idx, g] of scene.nodeEls) {
    const p = pos.get(idx);
    if (p === undefined) continue;
    g.setAttribute("transform", `translate(${(p.x - scene.w / 2).toFixed(1)}, ${(p.y - scene.h / 2).toFixed(1)})`);
  }
  for (const e of scene.edges) {
    const a = pos.get(e.from);
    const b = pos.get(e.to);
    if (a === undefined || b === undefined) continue;
    e.el.setAttribute(
      "d",
      `M ${a.x.toFixed(1)} ${(a.y + scene.h / 2).toFixed(1)} L ${b.x.toFixed(1)} ${(b.y - scene.h / 2).toFixed(1)}`,
    );
  }
}
