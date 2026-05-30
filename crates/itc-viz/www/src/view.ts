// The graph view: a persistent SVG driven by D3. Nodes and edges are data-joined by
// key, so updates animate via real eased transitions. Edges are cubic Béziers (they
// bow around the layout and interpolate cleanly while the graph morphs).
//
// Two drag modes:
//   plain-drag clock → clock   = join. The actual node moves under the cursor.
//   right/⌥-drag clock → clock = send. The source stays put; an orange version ghost
//                                follows the cursor and is delivered to the target.
//   click = tick, double-click = fork.

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
const REVEAL_PER_PX = 0.7; // stagger of the load reveal, by depth
const REVEAL_MAX = 800;
const CASCADE_MIN = 3; // only cascade when this many nodes enter at once (a load)

function reducedMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export interface GestureHandlers {
  tick(x: NodeIdx): void;
  fork(x: NodeIdx): void;
  join(a: NodeIdx, b: NodeIdx): void;
  send(from: NodeIdx, to: NodeIdx): void;
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
  readonly rowHeight: number;
  readonly width: number;
  readonly height: number;
}

export class GraphView {
  private readonly svg: Selection<SVGSVGElement, unknown, null, undefined>;
  private readonly edgesG: Selection<SVGGElement, unknown, null, undefined>;
  private readonly nodesG: Selection<SVGGElement, unknown, null, undefined>;
  private w = 0;
  private h = 0;
  private topY = 0;
  private rowHeight = 60;
  private style: StampStyle = { width: 64, unit: 9, maxHeight: 1 };
  private pos = new Map<NodeIdx, Point>();
  private pendingClick: { idx: NodeIdx; timer: number } | null = null;
  private ghost: HTMLElement | null = null;

  constructor(
    private readonly container: HTMLElement,
    private readonly handlers: GestureHandlers,
    private readonly isDisjoint: (a: NodeIdx, b: NodeIdx) => boolean,
  ) {
    this.svg = select(container).append<SVGSVGElement>("svg").attr("class", "scene");
    // No arrowheads: the top-down layout already encodes time's direction, so they
    // would be redundant ink.
    this.edgesG = this.svg.append<SVGGElement>("g").attr("class", "edges");
    this.nodesG = this.svg.append<SVGGElement>("g").attr("class", "nodes");
    this.svg.on("contextmenu", (e: Event) => e.preventDefault());
  }

  update(state: ViewState): void {
    this.w = state.style.width;
    this.h = stampHeight(state.style);
    this.rowHeight = state.rowHeight;
    this.style = state.style;

    // Size the canvas to at least fill the plate, with padding, so there is room to
    // fly a version ghost and so dragged nodes never reach a clipping edge.
    const pad = 32;
    const cw = this.container.clientWidth || state.width;
    const ch = this.container.clientHeight || state.height;
    const svgW = Math.max(state.width + pad * 2, cw);
    const svgH = Math.max(state.height + pad * 2, ch);
    const offX = (svgW - state.width) / 2;
    const offY = pad;
    this.svg.attr("width", svgW).attr("height", svgH).attr("viewBox", `0 0 ${svgW} ${svgH}`);

    this.pos = new Map();
    let top = Number.POSITIVE_INFINITY;
    for (const [idx, p] of state.pos) {
      this.pos.set(idx, { x: p.x + offX, y: p.y + offY });
      top = Math.min(top, p.y + offY);
    }
    this.topY = Number.isFinite(top) ? top : 0;

    const vnodes: VNode[] = state.nodes.map((desc) => {
      const p = this.pos.get(desc.idx) ?? { x: 0, y: 0 };
      return { idx: desc.idx, desc, live: state.live.has(desc.idx), x: p.x, y: p.y };
    });
    const vedges: VEdge[] = state.edges.map((e) => ({ key: `${e.from}->${e.to}:${e.kind}`, from: e.from, to: e.to, kind: e.kind }));

    this.joinEdges(vedges);
    this.joinNodes(vnodes);
  }

  /// A vertical cubic Bézier from `from`'s bottom to `to`'s top — straight when the
  /// child is directly below, a smooth S for diagonals. `ox`/`oy` override an endpoint
  /// (used while dragging).
  /// Transition duration (0 under reduced-motion, so animations snap).
  private dur(): number {
    return reducedMotion() ? 0 : DURATION;
  }

  /// Enter delay for the load reveal: nodes/edges ink in top-to-bottom. Only when a
  /// batch enters (a load/share-link), not for a single gesture, and never reduced.
  private revealDelay(entering: boolean, batch: number, y: number): number {
    if (!entering || batch < CASCADE_MIN || reducedMotion()) return 0;
    return Math.min(REVEAL_MAX, (y - this.topY) * REVEAL_PER_PX);
  }

  private bezier(from: NodeIdx, to: NodeIdx, override?: { idx: NodeIdx; x: number; y: number }): string {
    const a = override?.idx === from ? override : this.pos.get(from);
    const b = override?.idx === to ? override : this.pos.get(to);
    if (a === undefined || b === undefined) return "";
    const sx = a.x;
    const sy = a.y + this.h / 2;
    const ex = b.x;
    const ey = b.y - this.h / 2;
    // Edges spanning more than one row bow sideways so they swing clear of the nodes
    // in the intervening rows rather than cutting straight through them.
    const span = Math.max(1, Math.round((b.y - a.y) / this.rowHeight));
    const dir = ex >= sx ? 1 : -1;
    const bow = span > 1 ? Math.min(this.w * 1.5, (span - 1) * this.rowHeight * 0.34) * dir : 0;
    const c1y = sy + (ey - sy) * 0.4;
    const c2y = ey - (ey - sy) * 0.4;
    return `M ${sx.toFixed(1)} ${sy.toFixed(1)} C ${(sx + bow).toFixed(1)} ${c1y.toFixed(1)}, ${(ex + bow).toFixed(1)} ${c2y.toFixed(1)}, ${ex.toFixed(1)} ${ey.toFixed(1)}`;
  }

  private joinEdges(vedges: VEdge[]): void {
    const sel = this.edgesG.selectAll<SVGPathElement, VEdge>("path.edge").data(vedges, (d) => d.key);
    sel.exit().transition().duration(this.dur()).style("opacity", 0).remove();
    const entering = new Set<string>();
    const ent = sel
      .enter()
      .append("path")
      .each((d) => entering.add(d.key))
      .attr("class", (d) => `edge edge--${d.kind}`)
      .attr("d", (d) => this.bezier(d.from, d.to))
      .style("opacity", 0);
    ent
      .merge(sel)
      .transition()
      .duration(this.dur())
      .delay((d) => this.revealDelay(entering.has(d.key), entering.size, this.pos.get(d.to)?.y ?? 0))
      .style("opacity", 1)
      .attr("d", (d) => this.bezier(d.from, d.to));
  }

  private joinNodes(vnodes: VNode[]): void {
    const self = this;
    const transform = (d: VNode): string => `translate(${(d.x - self.w / 2).toFixed(1)}, ${(d.y - self.h / 2).toFixed(1)})`;

    const sel = this.nodesG.selectAll<SVGGElement, VNode>("g.node").data(vnodes, (d) => `${d.idx}`);
    sel.exit().transition().duration(this.dur()).style("opacity", 0).remove();

    const entering = new Set<NodeIdx>();
    const ent = sel
      .enter()
      .append<SVGGElement>("g")
      .attr("transform", transform)
      .style("opacity", 0)
      .each(function (d) {
        entering.add(d.idx);
        this.dataset["idx"] = `${d.idx}`;
        const g = select(this);
        g.append("title").text(d.desc.stamp);
        g.append("rect").attr("class", "node__hit").attr("x", 0).attr("y", 0).attr("width", self.style.width).attr("height", stampHeight(self.style));
        this.appendChild(renderStamp(parseId(d.desc.party), parseEvent(d.desc.event), self.style));
        g.append("text").attr("class", "node__index").attr("x", 1).attr("y", -3).text(`${d.idx}`);
      })
      .call(this.makeDrag());

    const merged = ent.merge(sel);
    merged.attr("class", (d) => `node ${d.live ? "node--live" : "node--historical"}`);
    merged
      .transition()
      .duration(this.dur())
      .delay((d) => this.revealDelay(entering.has(d.idx), entering.size, d.y))
      .attr("transform", transform)
      .style("opacity", 1);
  }

  /// Live re-route of edges incident to a dragged node, using a transient position.
  private dragEdges(idx: NodeIdx, x: number, y: number): void {
    this.edgesG.selectAll<SVGPathElement, VEdge>("path.edge").each((d, i, nodes) => {
      if (d.from !== idx && d.to !== idx) return;
      const node = nodes[i];
      if (node !== undefined) node.setAttribute("d", this.bezier(d.from, d.to, { idx, x, y }));
    });
  }

  private clearDropHints(): void {
    this.nodesG.selectAll(".node--drop-ok, .node--drop-reject").classed("node--drop-ok", false).classed("node--drop-reject", false);
  }

  private nodeUnder(clientX: number, clientY: number): NodeIdx | null {
    const g = document.elementFromPoint(clientX, clientY)?.closest<SVGGElement>("g.node");
    const raw = g?.dataset["idx"];
    return raw === undefined ? null : asNodeIdx(Number(raw));
  }

  private makeGhost(d: VNode): void {
    if (this.ghost !== null) return;
    const el = document.createElement("div");
    el.className = "version-ghost";
    el.appendChild(renderStamp(null, parseEvent(d.desc.event), this.style));
    document.body.appendChild(el);
    this.ghost = el;
  }

  private moveGhost(x: number, y: number): void {
    if (this.ghost === null) return;
    this.ghost.style.left = `${x}px`;
    this.ghost.style.top = `${y}px`;
  }

  private removeGhost(): void {
    this.ghost?.remove();
    this.ghost = null;
  }

  private makeDrag(): (sel: Selection<SVGGElement, VNode, SVGGElement, unknown>) => void {
    const self = this;
    let sendMode = false;
    let moved = false;
    let startX = 0;
    let startY = 0;

    const behavior = drag<SVGGElement, VNode>()
      .container(() => self.svg.node() as SVGSVGElement)
      .filter((event: PointerEvent) => event.button === 0 || event.button === 2)
      .on("start", (event: D3DragEvent<SVGGElement, VNode, VNode>) => {
        const src = event.sourceEvent as PointerEvent;
        sendMode = src.button === 2 || src.altKey;
        moved = false;
        startX = src.clientX;
        startY = src.clientY;
      })
      .on("drag", function (event: D3DragEvent<SVGGElement, VNode, VNode>, d: VNode) {
        const src = event.sourceEvent as PointerEvent;
        if (!moved && Math.hypot(src.clientX - startX, src.clientY - startY) > MOVE_THRESHOLD) moved = true;
        if (!moved) return;
        if (sendMode) {
          select(this).classed("node--sending", true);
          self.makeGhost(d);
          self.moveGhost(src.clientX, src.clientY);
        } else {
          // pointer-events:none so the drop target beneath is hit-tested, not the
          // dragged node riding on top.
          select(this)
            .interrupt()
            .raise()
            .style("pointer-events", "none")
            .attr("transform", `translate(${(event.x - self.w / 2).toFixed(1)}, ${(event.y - self.h / 2).toFixed(1)})`);
          self.dragEdges(d.idx, event.x, event.y);
        }
        self.clearDropHints();
        const over = self.nodeUnder(src.clientX, src.clientY);
        if (over === null || over === d.idx) return;
        const tgt = self.nodesG.select<SVGGElement>(`g.node[data-idx="${over}"]`);
        if (sendMode) tgt.classed("node--drop-ok", true);
        else tgt.classed(self.isDisjoint(d.idx, over) ? "node--drop-ok" : "node--drop-reject", true);
      })
      .on("end", function (event: D3DragEvent<SVGGElement, VNode, VNode>, d: VNode) {
        const src = event.sourceEvent as PointerEvent;
        self.clearDropHints();
        self.removeGhost();
        select(this).classed("node--sending", false);
        if (!moved) {
          select(this).style("pointer-events", null);
          self.click(d.idx);
          return;
        }
        // Hit-test while the dragged node is still transparent (join), then restore.
        const over = self.nodeUnder(src.clientX, src.clientY);
        select(this).style("pointer-events", null);
        if (over === null || over === d.idx) {
          if (!sendMode) self.snapBack(d);
          return;
        }
        if (sendMode) self.handlers.send(d.idx, over);
        else if (self.isDisjoint(d.idx, over)) self.handlers.join(d.idx, over);
        else self.snapBack(d);
      });

    return (sel) => {
      sel.call(behavior);
    };
  }

  /// Ease a dragged node (and its edges) back to its layout position after a no-op drop.
  private snapBack(d: VNode): void {
    const p = this.pos.get(d.idx);
    if (p === undefined) return;
    this.nodesG
      .select<SVGGElement>(`g.node[data-idx="${d.idx}"]`)
      .transition()
      .duration(this.dur())
      .attr("transform", `translate(${(p.x - this.w / 2).toFixed(1)}, ${(p.y - this.h / 2).toFixed(1)})`);
    this.edgesG
      .selectAll<SVGPathElement, VEdge>("path.edge")
      .filter((e) => e.from === d.idx || e.to === d.idx)
      .transition()
      .duration(this.dur())
      .attr("d", (e) => this.bezier(e.from, e.to));
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
