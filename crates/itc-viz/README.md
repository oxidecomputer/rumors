# itc-viz — an interactive Interval Tree Clocks visualizer

A browser visualizer for the `itc` crate, compiled to WebAssembly. You build up an
ITC execution by mouse, starting from the seed clock, and watch the causal history
grow as an immutable DAG drawn in the paper's interval + skyline notation (Almeida,
Baquero & Fonte, 2008).

The crate is a thin, deterministic **replay engine**: the browser holds the
operation log (the source of truth), and `Engine::replay` turns it into materialized
clock values. Everything else — the causal edges, liveness, layout, undo — is derived
from the log on the TypeScript side.

## Gestures

| Gesture | Operation |
|---|---|
| click a clock | **tick** (advance its own component) |
| double-click a clock | **fork** (split its id in two) |
| drag one clock onto another | **join** (only if their ids are disjoint) |
| right-drag or ⌥-drag a clock onto another | **send** its version (merge, no tick) |
| scroll / pinch | zoom · drag the background to pan |

History desaturates as you supersede it; the live frontier stays in color (teal id,
orange version skyline). **Back / forward** are undo / redo. The address-bar fragment
captures the whole figure, so the URL is a shareable link with no server state.
Toggle **History ⇄ Tableau** to see just the current clocks in a force-directed
arrangement.

## Build

Prerequisites: a Rust toolchain with the wasm target, `wasm-pack`, and Node.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack          # if not already installed
./build.sh                       # wasm-pack → tsc --noEmit (strict) → esbuild bundle
```

`build.sh` writes the wasm package to `www/pkg/` and the bundled front-end to
`www/dist/app.js` (both git-ignored).

## Run

Serve `www/` over HTTP (a `file://` page cannot load WebAssembly modules):

```sh
python3 -m http.server --directory www 8000
# then open http://localhost:8000
```

Any static file server works; the site is fully static and self-contained.

## Layout

- `src/lib.rs` — the wasm replay engine (`Engine`, the `Op` log, descriptors).
- `src/tests.rs` — host-target engine tests (`cargo test -p itc-viz`).
- `www/src/*.ts` — strict TypeScript: `engine` (typed wasm bridge), `oplog` (URL
  codec), `dag` (edges / liveness / cone / rewrite), `notation` (paper-notation
  parsers), `glyph` (stamp geometry + SVG), `layout` (layered + force), `view` (D3
  scene, transitions, drag, zoom), `main` (controller).
- `www/fonts/` — Newsreader and IBM Plex Mono, both under the SIL Open Font License
  1.1 (license texts alongside the `woff2` files).
