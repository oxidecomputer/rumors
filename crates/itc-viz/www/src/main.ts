// Controller. The op-log is the source of truth: every gesture rewrites it (via
// rewindAndApply, which is a plain append for live tips and a rewind-then-append for
// past nodes), then we replay → derive → layout → render. Starts at the seed.

import { deriveEdges, liveNodes, rewindAndApply } from "./dag";
import { Engine } from "./engine";
import { computeStampStyle, stampHeight } from "./glyph";
import { attachGestures, type GestureHandlers } from "./input";
import { layeredLayout } from "./layout";
import { parseEvent, parseId } from "./notation";
import { renderDag } from "./render";
import { asNodeIdx, type IdTree, type NodeDescriptor, type NodeIdx, type Op, type OpLog } from "./types";

const GAP_X = 30; // horizontal space between stamp columns
const EXTRA_V = 36; // vertical cell padding (room for the index chip + gap)

async function main(): Promise<void> {
  const plate = document.getElementById("plate");
  if (plate === null) throw new Error("missing #plate element");

  const engine = await Engine.create();

  let log: OpLog = [];
  let nodes: readonly NodeDescriptor[] = [];
  let live: ReadonlySet<NodeIdx> = new Set();
  let detach: (() => void) | null = null;

  const apply = (anchor: NodeIdx, makeOp: (remap: (i: NodeIdx) => NodeIdx) => Op): void => {
    log = rewindAndApply(log, anchor, makeOp);
    render();
  };

  const handlers: GestureHandlers = {
    tick: (x) => apply(x, (r) => ({ kind: "tick", x: r(x) })),
    fork: (x) => apply(x, (r) => ({ kind: "fork", x: r(x) })),
    join: (a, b) => apply(a, (r) => ({ kind: "join", a: r(a), b: r(b) })),
    peek: (x) => apply(x, (r) => ({ kind: "peek", x: r(x) })),
    merge: (token, target) => apply(target, (r) => ({ kind: "merge", t: r(target), m: r(token) })),
    peekMerge: (source, target) => {
      // The UI only offers this between two live clocks, so no rewind is needed:
      // peek appends a message at the current count, then merge consumes it.
      if (live.has(source) && live.has(target)) {
        const msg = asNodeIdx(nodes.length);
        log = [...log, { kind: "peek", x: source }, { kind: "merge", t: target, m: msg }];
        render();
      } else {
        apply(source, (r) => ({ kind: "peek", x: r(source) }));
      }
    },
  };

  function render(): void {
    nodes = engine.replay(log);
    const edges = deriveEdges(log);
    live = liveNodes(log, nodes);

    const ids: IdTree[] = [];
    const events = nodes.map((n) => {
      if (n.kind === "clock") ids.push(parseId(n.party));
      return parseEvent(n.event);
    });
    const style = computeStampStyle(ids, events);
    const cellW = style.width + GAP_X;
    const cellH = stampHeight(style) + EXTRA_V;
    const layout = layeredLayout(nodes.length, edges, cellW, cellH);

    detach?.();
    const scene = renderDag({
      nodes,
      edges,
      live,
      pos: layout.pos,
      style,
      width: layout.width,
      height: layout.height,
    });
    plate?.replaceChildren(scene.svg);
    detach = attachGestures(scene.svg, (a, b) => engine.isDisjoint(a, b), handlers);

    // Auto-scroll to keep the newest frontier (bottom) in view.
    if (plate !== null) plate.scrollTop = plate.scrollHeight;
  }

  render();
}

main().catch((err: unknown) => {
  const plate = document.getElementById("plate");
  const message = err instanceof Error ? err.message : String(err);
  if (plate !== null) plate.textContent = `failed to start: ${message}`;
  console.error(err);
});
