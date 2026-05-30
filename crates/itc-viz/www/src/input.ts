// Pointer gesture state machine. Translates raw pointer events on the scene into
// high-level intents. The drag's object type carries join vs merge; peek is the one
// modifier gesture (right-button OR ⌥), dual-bound for trackpad and mouse.
//
//   click clock                      → tick   (waits out the dbl-click window)
//   double-click clock               → fork
//   plain-drag clock → clock         → join   (only if ids are disjoint)
//   plain-drag token → clock         → merge
//   right/⌥-drag clock → empty       → peek   (leave a token)
//   right/⌥-drag clock → clock       → peekMerge (peek then merge, one motion)

import type { NodeIdx } from "./types";
import { asNodeIdx } from "./types";

export interface GestureHandlers {
  tick(x: NodeIdx): void;
  fork(x: NodeIdx): void;
  join(a: NodeIdx, b: NodeIdx): void;
  peek(x: NodeIdx): void;
  peekMerge(source: NodeIdx, target: NodeIdx): void;
  merge(token: NodeIdx, target: NodeIdx): void;
}

export interface NodeInfo {
  readonly kind: "clock" | "message";
  readonly live: boolean;
}

const MOVE_THRESHOLD = 5;
const DBLCLICK_MS = 220;

interface DragState {
  readonly idx: NodeIdx;
  readonly info: NodeInfo;
  readonly peekMode: boolean; // right button or ⌥
  readonly startX: number;
  readonly startY: number;
  moved: boolean;
}

/// Read the node group under a client point, if any.
function nodeUnder(x: number, y: number): { idx: NodeIdx; info: NodeInfo } | null {
  const target = document.elementFromPoint(x, y);
  const group = target?.closest<SVGGElement>(".node");
  if (group == null) return null;
  const raw = group.dataset["idx"];
  const kind = group.dataset["kind"];
  if (raw === undefined || (kind !== "clock" && kind !== "message")) return null;
  return { idx: asNodeIdx(Number(raw)), info: { kind, live: group.dataset["live"] === "1" } };
}

/// Attach gesture handling to a freshly-rendered scene. Returns a teardown function.
export function attachGestures(
  scene: SVGSVGElement,
  isDisjoint: (a: NodeIdx, b: NodeIdx) => boolean,
  handlers: GestureHandlers,
): () => void {
  let drag: DragState | null = null;
  let pendingClick: { idx: NodeIdx; timer: number } | null = null;

  const clearDropHints = (): void => {
    for (const g of scene.querySelectorAll(".node--drop-ok, .node--drop-reject")) {
      g.classList.remove("node--drop-ok", "node--drop-reject");
    }
  };

  const onMove = (e: PointerEvent): void => {
    if (drag === null) return;
    if (!drag.moved && Math.hypot(e.clientX - drag.startX, e.clientY - drag.startY) > MOVE_THRESHOLD) {
      drag.moved = true;
    }
    if (!drag.moved) return;
    clearDropHints();
    const over = nodeUnder(e.clientX, e.clientY);
    if (over === null || over.idx === drag.idx || over.info.kind !== "clock") return;
    // Highlight the drop target's validity.
    const group = scene.querySelector<SVGGElement>(`.node[data-idx="${over.idx}"]`);
    if (group === null) return;
    if (!drag.peekMode && drag.info.kind === "clock") {
      group.classList.add(isDisjoint(drag.idx, over.idx) ? "node--drop-ok" : "node--drop-reject");
    } else {
      group.classList.add("node--drop-ok"); // merge / peekMerge always valid onto a clock
    }
  };

  const onUp = (e: PointerEvent): void => {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    clearDropHints();
    const d = drag;
    drag = null;
    if (d === null) return;

    const over = nodeUnder(e.clientX, e.clientY);

    if (d.peekMode) {
      if (d.info.kind !== "clock") return;
      if (over !== null && over.idx !== d.idx && over.info.kind === "clock") {
        handlers.peekMerge(d.idx, over.idx);
      } else {
        handlers.peek(d.idx);
      }
      return;
    }

    if (!d.moved) {
      if (d.info.kind === "clock") clickClock(d.idx);
      return;
    }

    // Plain drag: deliver the grabbed object onto a clock.
    if (over === null || over.idx === d.idx || over.info.kind !== "clock") return;
    if (d.info.kind === "clock") {
      if (isDisjoint(d.idx, over.idx)) handlers.join(d.idx, over.idx);
    } else {
      handlers.merge(d.idx, over.idx);
    }
  };

  const clickClock = (idx: NodeIdx): void => {
    if (pendingClick !== null && pendingClick.idx === idx) {
      window.clearTimeout(pendingClick.timer);
      pendingClick = null;
      handlers.fork(idx);
      return;
    }
    if (pendingClick !== null) window.clearTimeout(pendingClick.timer);
    const timer = window.setTimeout(() => {
      pendingClick = null;
      handlers.tick(idx);
    }, DBLCLICK_MS);
    pendingClick = { idx, timer };
  };

  const onDown = (e: PointerEvent): void => {
    const over = nodeUnder(e.clientX, e.clientY);
    if (over === null) return;
    e.preventDefault();
    drag = {
      idx: over.idx,
      info: over.info,
      peekMode: e.button === 2 || e.altKey,
      startX: e.clientX,
      startY: e.clientY,
      moved: false,
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  };

  const onContextMenu = (e: Event): void => e.preventDefault();

  scene.addEventListener("pointerdown", onDown);
  scene.addEventListener("contextmenu", onContextMenu);

  return () => {
    scene.removeEventListener("pointerdown", onDown);
    scene.removeEventListener("contextmenu", onContextMenu);
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    if (pendingClick !== null) window.clearTimeout(pendingClick.timer);
  };
}
