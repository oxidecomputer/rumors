#!/bin/sh
# Rebuild the print edition from the Markdown chapters. Requires pandoc and typst.
set -eu
cd "$(dirname "$0")/.."

pandoc README.md 01-model.md 02-storage.md 03-operations.md \
  04-implementation.md 05-review.md \
  --from=markdown --to=typst --standalone \
  --lua-filter=print/edition.lua --template=print/edition.typ \
  --syntax-highlighting=none --fail-if-warnings \
  --output=print/manuscript.typ

typst compile --root . --creation-timestamp 1788868800 \
  print/manuscript.typ rumors-persistence-plan.pdf
