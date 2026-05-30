// Controller. The op-log is the source of truth: every gesture rewrites it (via
// rewindAndApply — a plain append for live tips, a rewind-then-append for past
// nodes), then we replay → derive → layout → hand the new state to the view, which
// animates the morph. Starts at the seed.

import { deriveEdges, liveNodes, rewindAndApply } from "./dag";
import { Engine } from "./engine";
import { computeStampStyle, stampHeight } from "./glyph";
import { layeredLayout, tableauLayout, type Point } from "./layout";
import { parseEvent, parseId } from "./notation";
import { decodeLog, encodeLog } from "./oplog";
import type { IdTree, NodeIdx, Op, OpLog } from "./types";
import { GraphView, type GestureHandlers } from "./view";

/// The current op-log from the address fragment (empty/invalid → the bare seed).
function logFromHash(): OpLog {
  try {
    return decodeLog(window.location.hash.replace(/^#/, ""));
  } catch {
    return [];
  }
}

const GAP_X = 48; // horizontal space between stamp columns (room for edges to bow)
const EXTRA_V = 46; // vertical cell padding (room for the index chip + curved edges)

async function main(): Promise<void> {
  const plate = document.getElementById("plate");
  if (plate === null) throw new Error("missing #plate element");
  const plateEl: HTMLElement = plate;
  plateEl.textContent = "";

  const engine = await Engine.create();
  let log: OpLog = logFromHash();
  let mode: "history" | "tableau" = "history";
  let prevTableau = new Map<NodeIdx, Point>();

  // Commit a new log: push a history entry (so back = undo, forward = redo, and the
  // URL is always a shareable snapshot), then re-render.
  const commit = (next: OpLog): void => {
    log = next;
    window.history.pushState(null, "", `#${encodeLog(log)}`);
    render();
  };

  const apply = (anchor: NodeIdx, makeOp: (remap: (i: NodeIdx) => NodeIdx) => Op): void => {
    commit(rewindAndApply(log, anchor, makeOp));
  };

  const handlers: GestureHandlers = {
    tick: (x) => apply(x, (r) => ({ kind: "tick", x: r(x) })),
    fork: (x) => apply(x, (r) => ({ kind: "fork", x: r(x) })),
    join: (a, b) => apply(a, (r) => ({ kind: "join", a: r(a), b: r(b) })),
    // Anchor on the receiver (which is superseded); the sender survives the rewind.
    send: (from, to) => apply(to, (r) => ({ kind: "send", from: r(from), to: r(to) })),
  };

  const view = new GraphView(plateEl, handlers, (a, b) => engine.isDisjoint(a, b));

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

    if (mode === "tableau") {
      // Only the current live clocks, arranged by the force simulation.
      const liveDescs = nodes.filter((n) => live.has(n.idx));
      const w = Math.max(plateEl.clientWidth, 320);
      const h = Math.max(plateEl.clientHeight, 320);
      const pos = tableauLayout(
        liveDescs.map((n) => n.idx),
        prevTableau,
        style,
        w,
        h,
      );
      prevTableau = pos;
      view.update({ nodes: liveDescs, edges: [], live, style, pos, rowHeight: cellH, width: w, height: h });
    } else {
      const layout = layeredLayout(nodes.length, edges, live, cellW, cellH);
      view.update({ nodes, edges, live, style, pos: layout.pos, rowHeight: cellH, width: layout.width, height: layout.height });
    }
  }

  // Back/forward walk the op-log history — undo and redo.
  window.addEventListener("popstate", () => {
    log = logFromHash();
    render();
  });

  const copyBtn = document.getElementById("copy-link");
  copyBtn?.addEventListener("click", () => {
    void navigator.clipboard?.writeText(window.location.href);
    copyBtn.textContent = "Copied";
    window.setTimeout(() => {
      copyBtn.textContent = "Copy link";
    }, 1200);
  });
  document.getElementById("reset")?.addEventListener("click", () => {
    view.resetView();
    commit([]);
  });

  const viewToggle = document.getElementById("view-toggle");
  const syncToggle = (): void => {
    if (viewToggle !== null) viewToggle.textContent = mode === "history" ? "View: history" : "View: tableau";
  };
  viewToggle?.addEventListener("click", () => {
    mode = mode === "history" ? "tableau" : "history";
    prevTableau = new Map(); // settle freshly when (re)entering tableau
    view.resetView(); // re-fit for the new mode
    syncToggle();
    render();
  });
  syncToggle();

  render();
}

main().catch((err: unknown) => {
  const plate = document.getElementById("plate");
  const message = err instanceof Error ? err.message : String(err);
  if (plate !== null) plate.textContent = `failed to start: ${message}`;
  console.error(err);
});
