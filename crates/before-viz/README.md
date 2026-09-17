# before-viz — an interactive Interval Tree Clocks visualizer

A browser visualizer for the `before` crate, compiled to WebAssembly. Starting from
the seed clock, you build an execution with the mouse and watch its causal history
grow as an immutable graph. Each clock shows party ownership below its version
skyline.

The operation log is the source of truth. The Rust engine replays it into clocks and
derives their causal edges and live frontier; the TypeScript front end handles layout,
interaction, and rendering.

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

The bundled fonts are Newsreader and IBM Plex Mono, both under the SIL Open Font
License 1.1; their license texts are included beside the font files.
