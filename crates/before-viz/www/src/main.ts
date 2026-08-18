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

  // Transient, non-modal notice (a rejected gesture, an unloadable link):
  // self-dismissing, outside the plate so rendering never clobbers it.
  const noticeEl = document.createElement("div");
  noticeEl.className = "notice";
  noticeEl.hidden = true;
  plateEl.insertAdjacentElement("afterend", noticeEl);
  let noticeTimer = 0;
  const notice = (text: string): void => {
    noticeEl.textContent = text;
    noticeEl.hidden = false;
    window.clearTimeout(noticeTimer);
    noticeTimer = window.setTimeout(() => {
      noticeEl.hidden = true;
    }, 4000);
  };
  const describe = (err: unknown): string => (err instanceof Error ? err.message : String(err));

  const engine = await Engine.create();
  let mode: "history" | "tableau" = "history";
  let prevTableau = new Map<NodeIdx, Point>();

  // A saved link can carry a fragment the engine refuses (oversized or
  // malformed): start from the seed instead of dying before the controls
  // wire up, and say so.
  let state: State;
  try {
    state = engine.load(window.location.hash.replace(/^#/, ""));
  } catch (err) {
    state = engine.load("");
    window.history.replaceState(null, "", window.location.pathname + window.location.search);
    notice(`could not load the shared link (${describe(err)}); started fresh`);
  }

  // Run an engine gesture, then commit a history entry and render. If the
  // engine rejects the op (an overlapping join, the op budget), leave
  // everything unchanged and say why: a rejected op never mints a URL, so
  // every fragment we push reloads.
  const gesture = (run: () => State): void => {
    let next: State;
    try {
      next = run();
    } catch (err) {
      notice(describe(err));
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

  // Back/forward reload the op-log from the fragment — undo and redo. A
  // history entry can carry a fragment the engine refuses (hand-edited or
  // truncated): the engine keeps its state on a rejected load, so resync
  // the URL to the state we actually have rather than desync or discard.
  window.addEventListener("popstate", () => {
    try {
      state = engine.load(window.location.hash.replace(/^#/, ""));
    } catch (err) {
      window.history.replaceState(null, "", `#${engine.fragment()}`);
      notice(`could not load this history entry (${describe(err)})`);
      return;
    }
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
