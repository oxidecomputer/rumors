// Controller. The op-log is the source of truth: every gesture rewrites it (via
// rewindAndApply — a plain append for live tips, a rewind-then-append for past
// nodes), then we replay → derive → layout → hand the new state to the view, which
// animates the morph. Starts at the seed.

import { deriveEdges, liveNodes, rewindAndApply } from "./dag";
import { Engine } from "./engine";
import { computeStampStyle, stampHeight } from "./glyph";
import { layeredLayout } from "./layout";
import { parseEvent, parseId } from "./notation";
import type { IdTree, NodeIdx, Op, OpLog } from "./types";
import { GraphView, type GestureHandlers } from "./view";

const GAP_X = 30; // horizontal space between stamp columns
const EXTRA_V = 36; // vertical cell padding (room for the index chip + gap)

async function main(): Promise<void> {
  const plate = document.getElementById("plate");
  if (plate === null) throw new Error("missing #plate element");
  plate.textContent = "";

  const engine = await Engine.create();
  let log: OpLog = [];

  const apply = (anchor: NodeIdx, makeOp: (remap: (i: NodeIdx) => NodeIdx) => Op): void => {
    log = rewindAndApply(log, anchor, makeOp);
    render();
  };

  const handlers: GestureHandlers = {
    tick: (x) => apply(x, (r) => ({ kind: "tick", x: r(x) })),
    fork: (x) => apply(x, (r) => ({ kind: "fork", x: r(x) })),
    join: (a, b) => apply(a, (r) => ({ kind: "join", a: r(a), b: r(b) })),
    // Anchor on the receiver (which is superseded); the sender survives the rewind.
    send: (from, to) => apply(to, (r) => ({ kind: "send", from: r(from), to: r(to) })),
  };

  const view = new GraphView(plate, handlers, (a, b) => engine.isDisjoint(a, b));

  function render(): void {
    const nodes = engine.replay(log);
    const edges = deriveEdges(log);
    const live = liveNodes(log, nodes);

    const ids: IdTree[] = [];
    const events = nodes.map((n) => {
      ids.push(parseId(n.party));
      return parseEvent(n.event);
    });
    const style = computeStampStyle(ids, events);
    const cellW = style.width + GAP_X;
    const cellH = stampHeight(style) + EXTRA_V;
    const layout = layeredLayout(nodes.length, edges, live, cellW, cellH);

    view.update({ nodes, edges, live, style, pos: layout.pos, width: layout.width, height: layout.height });

    plate?.scrollTo({ top: plate.scrollHeight, behavior: "smooth" });
  }

  render();
}

main().catch((err: unknown) => {
  const plate = document.getElementById("plate");
  const message = err instanceof Error ? err.message : String(err);
  if (plate !== null) plate.textContent = `failed to start: ${message}`;
  console.error(err);
});
