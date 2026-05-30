// Render the causal DAG as one SVG scene: edges behind, positioned stamps in front.
// Each node is a <g> tagged with its index/kind/liveness for hit-testing and later
// animation. Pure DOM construction from already-derived data + positions.

import type { Edge } from "./dag";
import { renderStamp, stampHeight, type StampStyle } from "./glyph";
import type { Point } from "./layout";
import { parseEvent, parseId } from "./notation";
import type { NodeDescriptor, NodeIdx } from "./types";

const SVG_NS = "http://www.w3.org/2000/svg";

export interface Scene {
  readonly svg: SVGSVGElement;
  readonly nodeEls: Map<NodeIdx, SVGGElement>;
}

export interface RenderInput {
  readonly nodes: readonly NodeDescriptor[];
  readonly edges: readonly Edge[];
  readonly live: ReadonlySet<NodeIdx>;
  readonly pos: ReadonlyMap<NodeIdx, Point>;
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
  const H = stampHeight(input.style);
  const W = input.style.width;

  const svg = el("svg", {
    class: "scene",
    width: `${input.width}`,
    height: `${input.height}`,
    viewBox: `0 0 ${input.width} ${input.height}`,
  });

  const defs = el("defs", {});
  defs.appendChild(arrowMarker());
  svg.appendChild(defs);

  // Edges behind nodes: parent bottom-center → child top-center.
  const edgeLayer = el("g", { class: "edges" });
  for (const e of input.edges) {
    const a = input.pos.get(e.from);
    const b = input.pos.get(e.to);
    if (a === undefined || b === undefined) continue;
    const path = el("path", {
      class: `edge edge--${e.kind}`,
      d: `M ${a.x.toFixed(1)} ${(a.y + H / 2).toFixed(1)} L ${b.x.toFixed(1)} ${(b.y - H / 2).toFixed(1)}`,
    });
    if (e.kind === "forkjoin") path.setAttribute("marker-end", "url(#arrow)");
    edgeLayer.appendChild(path);
  }
  svg.appendChild(edgeLayer);

  // Nodes in front.
  const nodeLayer = el("g", { class: "nodes" });
  const nodeEls = new Map<NodeIdx, SVGGElement>();
  for (const n of input.nodes) {
    const p = input.pos.get(n.idx);
    if (p === undefined) continue;
    const live = input.live.has(n.idx);

    const g = el("g", {
      class: `node node--${n.kind}${live ? " node--live" : " node--historical"}`,
      transform: `translate(${(p.x - W / 2).toFixed(1)}, ${(p.y - H / 2).toFixed(1)})`,
    });
    g.dataset["idx"] = `${n.idx}`;
    g.dataset["kind"] = n.kind;
    g.dataset["live"] = live ? "1" : "0";

    const title = el("title", {});
    title.textContent = n.kind === "clock" ? n.stamp : `message ${n.event}`;
    g.appendChild(title);

    // Transparent hit area covering the whole stamp box.
    g.appendChild(el("rect", { class: "node__hit", x: "0", y: "0", width: `${W}`, height: `${H}` }));

    const id = n.kind === "clock" ? parseId(n.party) : null;
    g.appendChild(renderStamp(id, parseEvent(n.event), n.kind, input.style, !live));

    const chip = el("text", { class: "node__index", x: "1", y: "-3" });
    chip.textContent = `${n.idx}`;
    g.appendChild(chip);

    nodeLayer.appendChild(g);
    nodeEls.set(n.idx, g);
  }
  svg.appendChild(nodeLayer);

  return { svg, nodeEls };
}
