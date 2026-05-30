// The graph view: a persistent SVG driven by D3. Nodes and edges are data-joined by
// key, so updates animate via real eased transitions (enter fades in, update glides,
// exit fades out). Dragging uses d3-drag to move the actual node under the cursor with
// its edges re-routing live; on release the gesture is classified and committed, or
// the node eases back. Gesture mapping mirrors the spec: click=tick, dbl-click=fork,
// plain-drag clock→clock=join, token→clock=merge, right/⌥-drag=peek / peekMerge.

import { drag, type D3DragEvent } from "d3-drag";
import { select, type Selection } from "d3-selection";
import "d3-transition";

import type { Edge } from "./dag";
import { renderStamp, stampHeight, type StampStyle } from "./glyph";
import type { Point } from "./layout";
import { parseEvent, parseId } from "./notation";
import { asNodeIdx, type NodeDescriptor, type NodeIdx } from "./types";

const DURATION = 280;
const MOVE_THRESHOLD = 5;
const DBLCLICK_MS = 220;

export interface GestureHandlers {
  tick(x: NodeIdx): void;
  fork(x: NodeIdx): void;
  join(a: NodeIdx, b: NodeIdx): void;
  peek(x: NodeIdx): void;
  peekMerge(source: NodeIdx, target: NodeIdx): void;
  merge(token: NodeIdx, target: NodeIdx): void;
}

interface VNode {
  readonly idx: NodeIdx;
  readonly desc: NodeDescriptor;
  readonly live: boolean;
  x: number;
  y: number;
}

interface VEdge {
  readonly key: string;
  readonly from: NodeIdx;
  readonly to: NodeIdx;
  readonly kind: Edge["kind"];
}

export interface ViewState {
  readonly nodes: readonly NodeDescriptor[];
  readonly edges: readonly Edge[];
  readonly live: ReadonlySet<NodeIdx>;
  readonly style: StampStyle;
  readonly pos: ReadonlyMap<NodeIdx, Point>;
  readonly width: number;
  readonly height: number;
}

export class GraphView {
  private readonly svg: Selection<SVGSVGElement, unknown, null, undefined>;
  private readonly edgesG: Selection<SVGGElement, unknown, null, undefined>;
  private readonly nodesG: Selection<SVGGElement, unknown, null, undefined>;
  private w = 0;
  private h = 0;
  private pos = new Map<NodeIdx, Point>();
  private pendingClick: { idx: NodeIdx; timer: number } | null = null;

  constructor(
    private readonly container: HTMLElement,
    private readonly handlers: GestureHandlers,
    private readonly isDisjoint: (a: NodeIdx, b: NodeIdx) => boolean,
  ) {
    this.svg = select(container).append<SVGSVGElement>("svg").attr("class", "scene");
    const marker = this.svg
      .append("defs")
      .append("marker")
      .attr("id", "arrow")
      .attr("viewBox", "0 0 8 8")
      .attr("refX", "7")
      .attr("refY", "4")
      .attr("markerWidth", "7")
      .attr("markerHeight", "7")
      .attr("orient", "auto-start-reverse");
    marker.append("path").attr("d", "M0,0 L8,4 L0,8 z").attr("class", "edge__arrow");
    this.edgesG = this.svg.append<SVGGElement>("g").attr("class", "edges");
    this.nodesG = this.svg.append<SVGGElement>("g").attr("class", "nodes");
    this.svg.on("contextmenu", (e: Event) => e.preventDefault());
  }

  update(state: ViewState): void {
    this.w = state.style.width;
    this.h = stampHeight(state.style);

    // Size the canvas to at least fill the plate, with padding, so there is empty room
    // to drop a peeked message and so dragged nodes never reach a clipping edge. Center
    // the DAG within it; the seed sits near the top and the frontier grows downward.
    const pad = 32;
    const cw = this.container.clientWidth || state.width;
    const ch = this.container.clientHeight || state.height;
    const svgW = Math.max(state.width + pad * 2, cw);
    const svgH = Math.max(state.height + pad * 2, ch);
    const offX = (svgW - state.width) / 2;
    const offY = pad;
    this.svg.attr("width", svgW).attr("height", svgH).attr("viewBox", `0 0 ${svgW} ${svgH}`);

    this.pos = new Map();
    for (const [idx, p] of state.pos) this.pos.set(idx, { x: p.x + offX, y: p.y + offY });

    const vnodes: VNode[] = state.nodes.map((desc) => {
      const p = this.pos.get(desc.idx) ?? { x: 0, y: 0 };
      return { idx: desc.idx, desc, live: state.live.has(desc.idx), x: p.x, y: p.y };
    });
    const vedges: VEdge[] = state.edges.map((e) => ({ key: `${e.from}->${e.to}:${e.kind}`, from: e.from, to: e.to, kind: e.kind }));

    this.joinEdges(vedges);
    this.joinNodes(vnodes, state.style);
  }

  private edgePath(from: NodeIdx, to: NodeIdx): string {
    const a = this.pos.get(from);
    const b = this.pos.get(to);
    if (a === undefined || b === undefined) return "";
    return `M ${a.x.toFixed(1)} ${(a.y + this.h / 2).toFixed(1)} L ${b.x.toFixed(1)} ${(b.y - this.h / 2).toFixed(1)}`;
  }

  private joinEdges(vedges: VEdge[]): void {
    const sel = this.edgesG.selectAll<SVGPathElement, VEdge>("path.edge").data(vedges, (d) => d.key);
    sel.exit().transition().duration(DURATION).style("opacity", 0).remove();
    const ent = sel
      .enter()
      .append("path")
      .attr("class", (d) => `edge edge--${d.kind}`)
      .attr("marker-end", (d) => (d.kind === "forkjoin" ? "url(#arrow)" : null))
      .attr("d", (d) => this.edgePath(d.from, d.to))
      .style("opacity", 0);
    // One transition per element: a separate opacity transition would be interrupted
    // by the `d` transition (both unnamed), leaving enter elements stuck transparent.
    ent
      .merge(sel)
      .transition()
      .duration(DURATION)
      .style("opacity", 1)
      .attr("d", (d) => this.edgePath(d.from, d.to));
  }

  private joinNodes(vnodes: VNode[], style: StampStyle): void {
    const self = this;
    const transform = (d: VNode): string => `translate(${(d.x - self.w / 2).toFixed(1)}, ${(d.y - self.h / 2).toFixed(1)})`;

    const sel = this.nodesG.selectAll<SVGGElement, VNode>("g.node").data(vnodes, (d) => `${d.idx}`);
    sel.exit().transition().duration(DURATION).style("opacity", 0).remove();

    const ent = sel
      .enter()
      .append<SVGGElement>("g")
      .attr("transform", transform)
      .style("opacity", 0)
      .each(function (d) {
        this.dataset["idx"] = `${d.idx}`;
        this.dataset["kind"] = d.desc.kind;
        const g = select(this);
        g.append("title").text(d.desc.kind === "clock" ? d.desc.stamp : `message ${d.desc.event}`);
        g.append("rect").attr("class", "node__hit").attr("x", 0).attr("y", 0).attr("width", style.width).attr("height", stampHeight(style));
        const id = d.desc.kind === "clock" ? parseId(d.desc.party) : null;
        this.appendChild(renderStamp(id, parseEvent(d.desc.event), d.desc.kind, style));
        g.append("text").attr("class", "node__index").attr("x", 1).attr("y", -3).text(`${d.idx}`);
      })
      .call(this.makeDrag());

    const merged = ent.merge(sel);
    merged.attr("class", (d) => `node node--${d.desc.kind} ${d.live ? "node--live" : "node--historical"}`);
    // Single transition for both transform (glide) and opacity (enter fade-in); two
    // unnamed transitions would interrupt each other and strand enter nodes invisible.
    merged.transition().duration(DURATION).attr("transform", transform).style("opacity", 1);
  }

  /// Live re-route of edges incident to a dragged node, using a transient position.
  private dragEdges(idx: NodeIdx, x: number, y: number): void {
    this.edgesG.selectAll<SVGPathElement, VEdge>("path.edge").each((d, i, nodes) => {
      if (d.from !== idx && d.to !== idx) return;
      const a = d.from === idx ? { x, y } : this.pos.get(d.from);
      const b = d.to === idx ? { x, y } : this.pos.get(d.to);
      const node = nodes[i];
      if (a === undefined || b === undefined || node === undefined) return;
      node.setAttribute("d", `M ${a.x.toFixed(1)} ${(a.y + this.h / 2).toFixed(1)} L ${b.x.toFixed(1)} ${(b.y - this.h / 2).toFixed(1)}`);
    });
  }

  private clearDropHints(): void {
    this.nodesG.selectAll(".node--drop-ok, .node--drop-reject").classed("node--drop-ok", false).classed("node--drop-reject", false);
  }

  private nodeUnder(clientX: number, clientY: number): { idx: NodeIdx; kind: "clock" | "message" } | null {
    const g = document.elementFromPoint(clientX, clientY)?.closest<SVGGElement>("g.node");
    const raw = g?.dataset["idx"];
    const kind = g?.dataset["kind"];
    if (raw === undefined || (kind !== "clock" && kind !== "message")) return null;
    return { idx: asNodeIdx(Number(raw)), kind };
  }

  private makeDrag(): (sel: Selection<SVGGElement, VNode, SVGGElement, unknown>) => void {
    const self = this;
    let peekMode = false;
    let moved = false;
    let startClientX = 0;
    let startClientY = 0;

    const behavior = drag<SVGGElement, VNode>()
      .container(() => self.svg.node() as SVGSVGElement)
      .filter((event: PointerEvent) => event.button === 0 || event.button === 2)
      .on("start", (event: D3DragEvent<SVGGElement, VNode, VNode>) => {
        const src = event.sourceEvent as PointerEvent;
        peekMode = src.button === 2 || src.altKey;
        moved = false;
        startClientX = src.clientX;
        startClientY = src.clientY;
      })
      .on("drag", function (event: D3DragEvent<SVGGElement, VNode, VNode>, d: VNode) {
        const src = event.sourceEvent as PointerEvent;
        if (!moved && Math.hypot(src.clientX - startClientX, src.clientY - startClientY) > MOVE_THRESHOLD) moved = true;
        if (!moved) return;
        // pointer-events:none so the node under the cursor (a drop target) is found,
        // not the dragged node riding on top.
        select(this)
          .interrupt()
          .raise()
          .style("pointer-events", "none")
          .attr("transform", `translate(${(event.x - self.w / 2).toFixed(1)}, ${(event.y - self.h / 2).toFixed(1)})`);
        if (peekMode) select(this).classed("node--peeking", true);
        self.dragEdges(d.idx, event.x, event.y);
        self.clearDropHints();
        const over = self.nodeUnder(src.clientX, src.clientY);
        if (over === null || over.idx === d.idx || over.kind !== "clock") return;
        const tgt = self.nodesG.select<SVGGElement>(`g.node[data-idx="${over.idx}"]`);
        if (!peekMode && d.desc.kind === "clock") tgt.classed(self.isDisjoint(d.idx, over.idx) ? "node--drop-ok" : "node--drop-reject", true);
        else tgt.classed("node--drop-ok", true);
      })
      .on("end", function (event: D3DragEvent<SVGGElement, VNode, VNode>, d: VNode) {
        const src = event.sourceEvent as PointerEvent;
        self.clearDropHints();
        select(this).classed("node--peeking", false);
        if (!moved) {
          select(this).style("pointer-events", null);
          if (d.desc.kind === "clock") self.click(d.idx);
          return;
        }
        // Hit-test the drop target while the dragged node is still transparent to
        // pointer events, then restore it.
        const over = self.nodeUnder(src.clientX, src.clientY);
        select(this).style("pointer-events", null);
        const valid =
          over !== null &&
          over.idx !== d.idx &&
          over.kind === "clock" &&
          (peekMode || d.desc.kind !== "clock" || self.isDisjoint(d.idx, over.idx));

        if (peekMode && d.desc.kind === "clock") {
          if (valid && over !== null) self.handlers.peekMerge(d.idx, over.idx);
          else self.handlers.peek(d.idx);
          return; // a re-render follows; nothing to snap back
        }
        if (valid && over !== null) {
          if (d.desc.kind === "clock") self.handlers.join(d.idx, over.idx);
          else self.handlers.merge(d.idx, over.idx);
          return;
        }
        self.snapBack(d); // no-op drop: ease the node home
      });

    return (sel) => {
      sel.call(behavior);
    };
  }

  /// Ease a dragged node (and its edges) back to its layout position.
  private snapBack(d: VNode): void {
    const p = this.pos.get(d.idx);
    if (p === undefined) return;
    this.nodesG
      .select<SVGGElement>(`g.node[data-idx="${d.idx}"]`)
      .transition()
      .duration(DURATION)
      .attr("transform", `translate(${(p.x - this.w / 2).toFixed(1)}, ${(p.y - this.h / 2).toFixed(1)})`);
    this.edgesG
      .selectAll<SVGPathElement, VEdge>("path.edge")
      .filter((e) => e.from === d.idx || e.to === d.idx)
      .transition()
      .duration(DURATION)
      .attr("d", (e) => this.edgePath(e.from, e.to));
  }

  private click(idx: NodeIdx): void {
    if (this.pendingClick !== null && this.pendingClick.idx === idx) {
      window.clearTimeout(this.pendingClick.timer);
      this.pendingClick = null;
      this.handlers.fork(idx);
      return;
    }
    if (this.pendingClick !== null) window.clearTimeout(this.pendingClick.timer);
    const timer = window.setTimeout(() => {
      this.pendingClick = null;
      this.handlers.tick(idx);
    }, DBLCLICK_MS);
    this.pendingClick = { idx, timer };
  }
}
