// Recursive-descent parsers for the paper's textual notation, turning the engine's
// Display strings into trees the glyph renderer can walk. Grammars:
//   id    = "0" | "1" | "(" id ", " id ")"
//   event = <number> | "(" <number> ", " event ", " event ")"
// Trees here are shallow (teaching-scale), so recursion is fine.

import type { EventTree, IdTree } from "./types";

class Cursor {
  i = 0;
  constructor(readonly s: string) {}

  skipWs(): void {
    while (this.i < this.s.length && this.s[this.i] === " ") this.i++;
  }

  peek(): string | undefined {
    return this.s[this.i];
  }

  expect(ch: string): void {
    this.skipWs();
    if (this.s[this.i] !== ch) {
      throw new Error(`expected '${ch}' at index ${this.i} of "${this.s}"`);
    }
    this.i++;
  }

  atEnd(): boolean {
    this.skipWs();
    return this.i >= this.s.length;
  }

  number(): number {
    this.skipWs();
    const start = this.i;
    while (this.i < this.s.length) {
      const c = this.s[this.i];
      if (c === undefined || c < "0" || c > "9") break;
      this.i++;
    }
    if (this.i === start) {
      throw new Error(`expected a number at index ${start} of "${this.s}"`);
    }
    return Number.parseInt(this.s.slice(start, this.i), 10);
  }
}

function idAt(c: Cursor): IdTree {
  c.skipWs();
  if (c.peek() === "(") {
    c.expect("(");
    const l = idAt(c);
    c.expect(",");
    const r = idAt(c);
    c.expect(")");
    return { l, r };
  }
  const ch = c.peek();
  if (ch === "0") {
    c.i++;
    return { leaf: 0 };
  }
  if (ch === "1") {
    c.i++;
    return { leaf: 1 };
  }
  throw new Error(`expected id leaf/node at index ${c.i} of "${c.s}"`);
}

function eventAt(c: Cursor): EventTree {
  c.skipWs();
  if (c.peek() === "(") {
    c.expect("(");
    const base = c.number();
    c.expect(",");
    const l = eventAt(c);
    c.expect(",");
    const r = eventAt(c);
    c.expect(")");
    return { base, l, r };
  }
  return { base: c.number() };
}

/// Parse an id (Party) in paper notation.
export function parseId(s: string): IdTree {
  const c = new Cursor(s);
  const t = idAt(c);
  if (!c.atEnd()) throw new Error(`trailing input in id "${s}"`);
  return t;
}

/// Parse an event (Version) in paper notation.
export function parseEvent(s: string): EventTree {
  const c = new Cursor(s);
  const t = eventAt(c);
  if (!c.atEnd()) throw new Error(`trailing input in event "${s}"`);
  return t;
}
