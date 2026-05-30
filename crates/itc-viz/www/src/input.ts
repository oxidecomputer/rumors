// Pointer gesture state machine. Translates raw pointer events on the scene into
// high-level intents. The drag's object type carries join vs merge; peek is the one
// modifier gesture (right-button OR ⌥), dual-bound for trackpad and mouse. While
// dragging, a ghost clone of the grabbed stamp follows the cursor.
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

interface Hit {
  readonly idx: NodeIdx;
  readonly info: NodeInfo;
  readonly group: SVGGElement;
}

interface DragState {
  readonly hit: Hit;
  readonly peekMode: boolean; // right button or ⌥
  readonly startX: number;
  readonly startY: number;
  moved: boolean;
  ghost: HTMLElement | null;
}

function hitUnder(x: number, y: number): Hit | null {
  const target = document.elementFromPoint(x, y);
  const group = target?.closest<SVGGElement>(".node");
  if (group == null) return null;
  const raw = group.dataset["idx"];
  const kind = group.dataset["kind"];
  if (raw === undefined || (kind !== "clock" && kind !== "message")) return null;
  return { idx: asNodeIdx(Number(raw)), info: { kind, live: group.dataset["live"] === "1" }, group };
}

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

  const makeGhost = (d: DragState): void => {
    const stamp = d.hit.group.querySelector("svg.glyph");
    if (stamp === null) return;
    const ghost = document.createElement("div");
    ghost.className = `drag-ghost${d.peekMode ? " drag-ghost--peek" : ""}`;
    ghost.appendChild(stamp.cloneNode(true));
    document.body.appendChild(ghost);
    d.ghost = ghost;
    d.hit.group.classList.add("node--dragging");
  };

  const moveGhost = (d: DragState, x: number, y: number): void => {
    if (d.ghost === null) return;
    d.ghost.style.left = `${x}px`;
    d.ghost.style.top = `${y}px`;
  };

  const dropGhost = (d: DragState): void => {
    d.ghost?.remove();
    d.hit.group.classList.remove("node--dragging");
  };

  const onMove = (e: PointerEvent): void => {
    if (drag === null) return;
    if (!drag.moved && Math.hypot(e.clientX - drag.startX, e.clientY - drag.startY) > MOVE_THRESHOLD) {
      drag.moved = true;
      makeGhost(drag);
    }
    if (!drag.moved) return;
    moveGhost(drag, e.clientX, e.clientY);
    clearDropHints();
    const over = hitUnder(e.clientX, e.clientY);
    if (over === null || over.idx === drag.hit.idx || over.info.kind !== "clock") return;
    if (!drag.peekMode && drag.hit.info.kind === "clock") {
      over.group.classList.add(isDisjoint(drag.hit.idx, over.idx) ? "node--drop-ok" : "node--drop-reject");
    } else {
      over.group.classList.add("node--drop-ok"); // merge / peekMerge always valid onto a clock
    }
  };

  const onUp = (e: PointerEvent): void => {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    clearDropHints();
    const d = drag;
    drag = null;
    if (d === null) return;
    dropGhost(d);

    const over = hitUnder(e.clientX, e.clientY);

    if (d.peekMode) {
      if (d.hit.info.kind !== "clock") return;
      if (over !== null && over.idx !== d.hit.idx && over.info.kind === "clock") {
        handlers.peekMerge(d.hit.idx, over.idx);
      } else {
        handlers.peek(d.hit.idx);
      }
      return;
    }

    if (!d.moved) {
      if (d.hit.info.kind === "clock") clickClock(d.hit.idx);
      return;
    }

    // Plain drag: deliver the grabbed object onto a clock.
    if (over === null || over.idx === d.hit.idx || over.info.kind !== "clock") return;
    if (d.hit.info.kind === "clock") {
      if (isDisjoint(d.hit.idx, over.idx)) handlers.join(d.hit.idx, over.idx);
    } else {
      handlers.merge(d.hit.idx, over.idx);
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
    const hit = hitUnder(e.clientX, e.clientY);
    if (hit === null) return;
    e.preventDefault();
    drag = { hit, peekMode: e.button === 2 || e.altKey, startX: e.clientX, startY: e.clientY, moved: false, ghost: null };
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
    drag?.ghost?.remove();
    if (pendingClick !== null) window.clearTimeout(pendingClick.timer);
  };
}
