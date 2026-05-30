// Entry point. Phase 0: load the engine, replay the empty log (the seed clock), and
// show its descriptor as proof the wasm pipeline is live. Later phases replace this
// with the full DAG model, layout, and SVG rendering.

import { Engine } from "./engine";

async function main(): Promise<void> {
  const app = document.getElementById("app");
  if (app === null) {
    throw new Error("missing #app element");
  }

  const engine = await Engine.create();
  const nodes = engine.replay([]);

  const lines = nodes.map((n) =>
    n.kind === "clock"
      ? `#${n.idx} clock   stamp=${n.stamp}`
      : `#${n.idx} message event=${n.event}`,
  );
  app.textContent = `ITC engine online. Seed:\n${lines.join("\n")}`;
}

main().catch((err: unknown) => {
  const app = document.getElementById("app");
  const message = err instanceof Error ? err.message : String(err);
  if (app !== null) {
    app.textContent = `failed to start: ${message}`;
  }
  console.error(err);
});
