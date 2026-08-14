#!/usr/bin/env bash
# Build the visualizer: compile the engine to wasm, typecheck the strict TypeScript,
# then bundle it. Output is statically hostable from `www/` (serve over http — `file://`
# cannot load wasm modules).
set -euo pipefail
cd "$(dirname "$0")"

wasm-pack build . --target web --out-dir www/pkg

cd www
# npm ci: install exactly what package-lock.json commits, never resolve anew —
# the Pages deploy runs this script, so its dependency set must be the
# committed one.
npm ci
npm run typecheck
npm run bundle

echo "built. serve with: python3 -m http.server --directory www"
