// Entry point. Phase 2: replay a representative op sequence and render every node as a
// paper-style stamp glyph inside the Monograph plate. (The interactive immutable DAG,
// layout, gestures, and URL state arrive in Phases 3–7; for now this showcases the
// glyph + design language.)

import { Engine } from "./engine";
import { renderStamp } from "./glyph";
import { parseEvent, parseId } from "./notation";
import { asNodeIdx, type NodeDescriptor, type Op } from "./types";

// A sample run: tick twice, fork, tick each half, then peek one half and merge it into
// the other — the merge collapses the event tree to a single integer, showing the
// paper's normalization in the skyline.
const SAMPLE: readonly Op[] = [
  { kind: "tick", x: asNodeIdx(0) },
  { kind: "tick", x: asNodeIdx(1) },
  { kind: "fork", x: asNodeIdx(2) },
  { kind: "tick", x: asNodeIdx(3) },
  { kind: "tick", x: asNodeIdx(4) },
  { kind: "peek", x: asNodeIdx(5) },
  { kind: "merge", t: asNodeIdx(6), m: asNodeIdx(7) },
];

function figureForNode(node: NodeDescriptor): HTMLElement {
  const figure = document.createElement("figure");
  figure.className = "stamp";

  const id = node.kind === "clock" ? parseId(node.party) : null;
  const event = parseEvent(node.event);
  figure.appendChild(renderStamp(id, event, node.kind));

  const caption = document.createElement("figcaption");
  const index = document.createElement("span");
  index.className = "stamp__index";
  index.textContent = `${node.idx}`;
  const notation = document.createElement("span");
  notation.className = "stamp__notation";
  notation.textContent = node.kind === "clock" ? node.stamp : node.event;
  caption.append(index, notation);
  figure.appendChild(caption);

  return figure;
}

async function main(): Promise<void> {
  const plate = document.getElementById("plate");
  if (plate === null) throw new Error("missing #plate element");

  const engine = await Engine.create();
  const nodes = engine.replay(SAMPLE);

  plate.replaceChildren(...nodes.map(figureForNode));
}

main().catch((err: unknown) => {
  const plate = document.getElementById("plate");
  const message = err instanceof Error ? err.message : String(err);
  if (plate !== null) plate.textContent = `failed to start: ${message}`;
  console.error(err);
});
