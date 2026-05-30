// Smoothly morph the scene from a start layout to a target layout: interpolate every
// node's position each frame (edges follow, since applyPositions re-routes them) and
// fade new nodes in. Honors prefers-reduced-motion by snapping.

import type { Point } from "./layout";
import { applyPositions, type Scene } from "./render";
import type { NodeIdx } from "./types";

const DURATION_MS = 260;

const easeOutCubic = (t: number): number => 1 - (1 - t) ** 3;

function reducedMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export function animatePositions(
  scene: Scene,
  start: ReadonlyMap<NodeIdx, Point>,
  target: ReadonlyMap<NodeIdx, Point>,
  newNodes: ReadonlySet<NodeIdx>,
): void {
  const setOpacity = (idx: NodeIdx, v: number): void => {
    const g = scene.nodeEls.get(idx);
    if (g !== undefined) g.style.opacity = `${v}`;
  };

  if (reducedMotion()) {
    applyPositions(scene, target);
    for (const idx of newNodes) setOpacity(idx, 1);
    return;
  }

  for (const idx of newNodes) setOpacity(idx, 0);

  const begin = performance.now();
  const frame = (now: number): void => {
    const t = Math.min(1, (now - begin) / DURATION_MS);
    const e = easeOutCubic(t);
    const interp = new Map<NodeIdx, Point>();
    for (const [idx, tp] of target) {
      const sp = start.get(idx) ?? tp;
      interp.set(idx, { x: sp.x + (tp.x - sp.x) * e, y: sp.y + (tp.y - sp.y) * e });
    }
    applyPositions(scene, interp);
    for (const idx of newNodes) setOpacity(idx, e);
    if (t < 1) requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}
