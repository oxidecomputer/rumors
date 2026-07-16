#!/usr/bin/env node
// Generates production.qnt: streamingMirror instances at the Rust
// production constants (ROOT_H = 32, F = 256, CAP_LEVEL = 256).
//
// Deterministic: output depends only on this source file. No
// dependencies, no clock, no randomness. Run from anywhere:
//
//   node gen-production.mjs            # writes production.qnt next to this script
//   node gen-production.mjs -          # emits to stdout instead
//
// Skeleton conventions (MODEL.md §2, mirrored from instances.qnt):
// index 0 = root at height ROOT_H, BFS ids (parent < child, siblings
// ascend), kind "D"|"R", R scopes childless, only height-1 D scopes
// carry leafReqs > 0, fan and leafReqs <= F.

import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT_H = 32;
const F = 256;
const CAP_LEVEL = 256;

// ---------------------------------------------------------- skeleton helpers

/** One skeleton scope. */
const scope = (kind, height, kids, leafReqs = 0) => ({ kind, height, kids, leafReqs });

/** [lo, lo+1, ..., hi] inclusive. */
const range = (lo, hi) => Array.from({ length: hi - lo + 1 }, (_, i) => lo + i);

/**
 * n1Prod shape: instances.qnt's n1DropW stretched to depth ROOT_H.
 * Root (height ROOT_H) with kids [1, 2]; scope 1 heads a D-chain
 * descending one scope per height ROOT_H-1 .. 1 (leafReqs 1 at the
 * bottom); scope 2 is a childless D at height ROOT_H-1.
 */
function chainWithStub() {
  const skel = [
    scope("D", ROOT_H, [1, 2]),
    scope("D", ROOT_H - 1, [3]), // chain head; its tail continues at id 3
    scope("D", ROOT_H - 1, []),  // the childless sibling stub
  ];
  for (let h = ROOT_H - 2; h >= 1; h--) {
    const id = skel.length;
    skel.push(h === 1 ? scope("D", 1, [], 1) : scope("D", h, [id + 1]));
  }
  return skel;
}

/** n2Prod* shape: root with `n` childless D kids at height ROOT_H - 1. */
function rootFan(n) {
  return [
    scope("D", ROOT_H, range(1, n)),
    ...range(1, n).map(() => scope("D", ROOT_H - 1, [])),
  ];
}

/**
 * prodComb32 shape: instances.qnt's comb6 stretched to ROOT_H. A D-spine
 * from the root to height 1 (leafReqs 2 at the bottom); each spine scope
 * at heights ROOT_H-1 .. 2 has the next spine scope plus one childless
 * extra kid, the extra's kind alternating D/R by the kid's height parity
 * (even = D, odd = R — the same mix comb6 shows at heights 4 and 3).
 */
function comb() {
  // BFS ids: root = 0; spine at height ROOT_H-1 = 1 (the root's only
  // kid, so no extra at that level); below that each level holds the
  // spine scope then its extra sibling.
  const spineId = (h) => (h === ROOT_H - 1 ? 1 : 2 + 2 * (ROOT_H - 2 - h));
  const extraKind = (h) => (h % 2 === 0 ? "D" : "R");
  const skel = [scope("D", ROOT_H, [1])];
  for (let h = ROOT_H - 1; h >= 1; h--) {
    skel.push(
      h === 1
        ? scope("D", 1, [], 2)
        : scope("D", h, [spineId(h - 1), spineId(h - 1) + 1])
    );
    if (h < ROOT_H - 1) skel.push(scope(extraKind(h), h, []));
  }
  return skel;
}

/**
 * prodFan256 shape: root with one D kid at height ROOT_H-1; that kid has
 * F childless D kids at height ROOT_H-2 (the full production fan one
 * level down).
 */
function deepFan(n) {
  return [
    scope("D", ROOT_H, [1]),
    scope("D", ROOT_H - 1, range(2, n + 1)),
    ...range(2, n + 1).map(() => scope("D", ROOT_H - 2, [])),
  ];
}

// ------------------------------------------------------------------ emission

/** Renders a kids list, wrapping long lists for auditability. */
function fmtKids(kids, indent) {
  if (kids.length <= 16) return `[${kids.join(", ")}]`;
  const lines = [];
  for (let i = 0; i < kids.length; i += 16) {
    lines.push(kids.slice(i, i + 16).join(", "));
  }
  const pad = " ".repeat(indent + 2);
  return `[\n${pad}${lines.join(`,\n${pad}`)}\n${" ".repeat(indent)}]`;
}

function fmtScope(sc) {
  const indent = 6;
  const kids = fmtKids(sc.kids, indent);
  return `${" ".repeat(indent)}{ kind: "${sc.kind}", height: ${sc.height}, kids: ${kids}, leafReqs: ${sc.leafReqs} }`;
}

function fmtModule({ name, comment, axioms, skel }) {
  const ax = Object.entries(axioms)
    .map(([k, v]) => `${k} = ${v}`)
    .join(", ");
  return [
    ...comment.map((l) => `// ${l}`.trimEnd()),
    `module ${name} {`,
    `  import streamingMirror(`,
    `    ROOT_H = ${ROOT_H}, F = ${F}, CAP_LEVEL = ${CAP_LEVEL}, NSC = ${skel.length},`,
    `    ${ax},`,
    `    SKEL = [`,
    skel.map(fmtScope).join(",\n"),
    `    ]`,
    `  ).* from "./streamingMirror"`,
    `}`,
  ].join("\n");
}

const allOn = {
  AX_W: true,
  AX_D1_ROOT: true,
  AX_D1_INT: true,
  AX_D2: true,
  AX_D3: true,
  WIRE_FIRST: false,
};

const modules = [
  {
    name: "n1Prod",
    comment: [
      "--- N1 at production scale: drop Axiom W. n1DropW's shape stretched",
      "to depth 32: a full-depth dispute chain plus a childless sibling",
      "under the root. Expected: stuck reachable.",
    ],
    axioms: { ...allOn, AX_W: false },
    skel: chainWithStub(),
  },
  {
    name: "n2Prod6",
    comment: [
      "--- N2 at production scale: drop Axiom D1 at the root only, wire",
      "replies forced first. Root fan 6 (the deadlock side of the n2fan6/",
      "n2fan5 boundary). Expected: stuck reachable.",
    ],
    axioms: { ...allOn, AX_D1_ROOT: false, WIRE_FIRST: true },
    skel: rootFan(6),
  },
  {
    name: "n2Prod5",
    comment: [
      "--- Root fan 5: the safe side of the same boundary at depth 32.",
    ],
    axioms: { ...allOn, AX_D1_ROOT: false, WIRE_FIRST: true },
    skel: rootFan(5),
  },
  {
    name: "n2Prod256",
    comment: [
      "--- Root fan 256: the full production fan under the same D1-root",
      "drop (257 scopes).",
    ],
    axioms: { ...allOn, AX_D1_ROOT: false, WIRE_FIRST: true },
    skel: rootFan(256),
  },
  {
    name: "prodComb32",
    comment: [
      "--- Positive at production depth: comb6 stretched to ROOT_H = 32.",
      "A D-spine to height 1 (leafReqs 2), one childless extra kid per",
      "spine level, kind alternating D/R by the kid's height parity.",
    ],
    axioms: { ...allOn },
    skel: comb(),
  },
  {
    name: "prodFan256",
    comment: [
      "--- Positive at production fan: one D kid under the root, which",
      "fans out to 256 childless D scopes at height 30.",
    ],
    axioms: { ...allOn },
    skel: deepFan(F),
  },
];

const header = [
  "// production.qnt — streamingMirror instances at the Rust production",
  "// constants (ROOT_H = 32, F = 256, CAP_LEVEL = 256).",
  "//",
  "// GENERATED by gen-production.mjs. Do not hand-edit; edit the",
  "// generator and re-run `node gen-production.mjs`.",
].join("\n");

const output = `${header}\n\n${modules.map(fmtModule).join("\n\n")}\n`;

if (process.argv[2] === "-") {
  process.stdout.write(output);
} else {
  const out = join(dirname(fileURLToPath(import.meta.url)), "production.qnt");
  writeFileSync(out, output);
  process.stderr.write(`wrote ${out}\n`);
}
