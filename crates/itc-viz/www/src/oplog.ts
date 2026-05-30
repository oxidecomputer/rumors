// The op-log codec: a compact, URL-safe encoding of the operation log for the address
// fragment, so any constructed example is a shareable link with no server state. The
// log replays deterministically through the wasm engine, so the fragment fully
// reconstructs the DAG. Each op is a 1-byte tag + LEB128 varint operands (node
// indices), then base64url. An empty log encodes to the empty string.

import { asNodeIdx, type Op, type OpLog } from "./types";

const TAG = { tick: 1, fork: 2, join: 3, send: 4 } as const;

function pushVarint(bytes: number[], n: number): void {
  let v = n >>> 0;
  while (v >= 0x80) {
    bytes.push((v & 0x7f) | 0x80);
    v >>>= 7;
  }
  bytes.push(v);
}

function readVarint(bytes: Uint8Array, pos: number): [value: number, next: number] {
  let result = 0;
  let shift = 0;
  let p = pos;
  for (;;) {
    const b = bytes[p++];
    if (b === undefined) throw new Error("truncated varint");
    result |= (b & 0x7f) << shift;
    if ((b & 0x80) === 0) break;
    shift += 7;
  }
  return [result >>> 0, p];
}

function base64urlEncode(bytes: Uint8Array): string {
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function base64urlDecode(s: string): Uint8Array {
  const b64 = s.replace(/-/g, "+").replace(/_/g, "/");
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

/// Encode an op-log to a URL-safe fragment string.
export function encodeLog(log: OpLog): string {
  const bytes: number[] = [];
  for (const op of log) {
    switch (op.kind) {
      case "tick":
        bytes.push(TAG.tick);
        pushVarint(bytes, op.x);
        break;
      case "fork":
        bytes.push(TAG.fork);
        pushVarint(bytes, op.x);
        break;
      case "join":
        bytes.push(TAG.join);
        pushVarint(bytes, op.a);
        pushVarint(bytes, op.b);
        break;
      case "send":
        bytes.push(TAG.send);
        pushVarint(bytes, op.from);
        pushVarint(bytes, op.to);
        break;
    }
  }
  return base64urlEncode(new Uint8Array(bytes));
}

/// Decode a fragment string back into an op-log. Throws on malformed input.
export function decodeLog(fragment: string): OpLog {
  if (fragment === "") return [];
  const bytes = base64urlDecode(fragment);
  const log: Op[] = [];
  let pos = 0;
  while (pos < bytes.length) {
    const tag = bytes[pos++];
    if (tag === TAG.tick) {
      const [x, p] = readVarint(bytes, pos);
      pos = p;
      log.push({ kind: "tick", x: asNodeIdx(x) });
    } else if (tag === TAG.fork) {
      const [x, p] = readVarint(bytes, pos);
      pos = p;
      log.push({ kind: "fork", x: asNodeIdx(x) });
    } else if (tag === TAG.join) {
      const [a, p1] = readVarint(bytes, pos);
      const [b, p2] = readVarint(bytes, p1);
      pos = p2;
      log.push({ kind: "join", a: asNodeIdx(a), b: asNodeIdx(b) });
    } else if (tag === TAG.send) {
      const [from, p1] = readVarint(bytes, pos);
      const [to, p2] = readVarint(bytes, p1);
      pos = p2;
      log.push({ kind: "send", from: asNodeIdx(from), to: asNodeIdx(to) });
    } else {
      throw new Error(`unknown op tag ${tag}`);
    }
  }
  return log;
}
