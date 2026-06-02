// Controller. The engine owns the op-log and the causal-history DAG; the front-end is
// presentational. Each gesture calls an engine method (which rewinds + appends and
// returns the new state); we push a history entry (URL fragment) so back/forward are
// undo/redo and the link is shareable, then lay out and render. Starts at the seed.

import { Engine } from "./engine";
import { computeStampStyle, stampHeight } from "./glyph";
import { layeredLayout, tableauLayout, type Point } from "./layout";
import { parseEvent, parseId } from "./notation";
import type { IdTree, NodeIdx, State } from "./types";
import { GraphView, type GestureHandlers } from "./view";

const GAP_X = 48; // horizontal space between stamp columns (room for edges to bow)
const EXTRA_V = 46; // vertical cell padding (room for the index chip + curved edges)

async function main(): Promise<void> {
  const plate = document.getElementById("plate");
  if (plate === null) throw new Error("missing #plate element");
  const plateEl: HTMLElement = plate;
  plateEl.textContent = "";

  const engine = await Engine.create();
  let state: State = engine.load(window.location.hash.replace(/^#/, ""));
  let mode: "history" | "tableau" = "history";
  let prevTableau = new Map<NodeIdx, Point>();

  // Run an engine gesture, then commit a history entry and render. If the engine
  // rejects the op (e.g. an overlapping join), leave everything unchanged.
  const gesture = (run: () => State): void => {
    let next: State;
    try {
      next = run();
    } catch {
      return;
    }
    state = next;
    window.history.pushState(null, "", `#${engine.fragment()}`);
    render();
  };

  const handlers: GestureHandlers = {
    tick: (x) => gesture(() => engine.tick(x)),
    fork: (x) => gesture(() => engine.fork(x)),
    join: (a, b) => gesture(() => engine.join(a, b)),
    send: (from, to) => gesture(() => engine.send(from, to)),
  };

  const view = new GraphView(plateEl, handlers, (a, b) => engine.isDisjoint(a, b));

  function render(): void {
    const live = new Set<NodeIdx>(state.live);

    const ids: IdTree[] = [];
    const events = state.nodes.map((n) => {
      ids.push(parseId(n.party));
      return parseEvent(n.event);
    });
    const style = computeStampStyle(ids, events);
    const cellW = style.width + GAP_X;
    const cellH = stampHeight(style) + EXTRA_V;

    if (mode === "tableau") {
      const liveDescs = state.nodes.filter((n) => live.has(n.idx));
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
      view.update({ nodes: liveDescs, edges: [], live, style, pos, rowHeight: cellH, width: w, height: h, mode: "tableau" });
    } else {
      const layout = layeredLayout(state.nodes.length, state.edges, live, cellW, cellH);
      view.update({ nodes: state.nodes, edges: state.edges, live, style, pos: layout.pos, rowHeight: cellH, width: layout.width, height: layout.height, mode: "history" });
    }
  }

  // Back/forward reload the op-log from the fragment — undo and redo.
  window.addEventListener("popstate", () => {
    state = engine.load(window.location.hash.replace(/^#/, ""));
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
    gesture(() => engine.load(""));
  });

  const viewToggle = document.getElementById("view-toggle");
  const syncToggle = (): void => {
    if (viewToggle !== null) viewToggle.textContent = mode === "history" ? "View: history" : "View: tableau";
  };
  viewToggle?.addEventListener("click", () => {
    mode = mode === "history" ? "tableau" : "history";
    prevTableau = new Map();
    view.resetView();
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
