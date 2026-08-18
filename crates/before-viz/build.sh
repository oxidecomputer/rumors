#!/usr/bin/env bash
# Build the visualizer: compile the engine to wasm, typecheck the strict TypeScript,
# then bundle it. Output is statically hostable from `www/` (serve over http — `file://`
# cannot load wasm modules).
set -euo pipefail
cd "$(dirname "$0")"

wasm-pack build . --target web --out-dir www/pkg

cd www
# CI must install exactly what package-lock.json commits, never resolve
# anew (the Pages deploy runs this script; GitHub Actions sets CI=true).
# The local loop keeps npm install: its no-op on a warm node_modules is
# what keeps `just viz` iteration cheap, where npm ci would rebuild
# node_modules from scratch every run.
if [ "${CI:-false}" = "true" ]; then npm ci; else npm install; fi
npm run typecheck
npm run bundle

echo "built. serve with: python3 -m http.server --directory www"
