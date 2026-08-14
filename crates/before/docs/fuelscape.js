// fuelscape widget: the interactive measured-growth explorer embedded in
// the rustdoc's # Complexity islands. Dependency-free single-file source,
// served verbatim inside every rustdoc page's head via
// docs/fuelscape-header.html (`just fuelscape-header` regenerates the
// header; build.rs refuses a stale one).
//
// Honesty rule: instruction counts are WASM operations metered in a
// sandboxed build, so the widget presents shapes and ratios only — growth
// against a compensation hypothesis, quantile bands, spread — and never
// an absolute count. Axis gridlines carry no numbers; readouts are ratios.

"use strict";
(function () {

// ---------- tiny DOM helpers (replaces d3-selection) ----------
const NS = "http://www.w3.org/2000/svg";
const SVGTAGS = new Set(["svg","g","rect","line","circle","text","tspan",
  "polyline","path","defs","clipPath","filter","feGaussianBlur","title"]);
function mk(parent, tag, cls) {
  const el = SVGTAGS.has(tag) ? document.createElementNS(NS, tag) : document.createElement(tag);
  if (cls) el.setAttribute("class", cls);
  if (parent) parent.appendChild(el);
  return el;
}
function attrs(el, o) { for (const k in o) el.setAttribute(k, o[k]); return el; }

// ---------- linear scale (replaces d3-scale) ----------
function scaleLinear(d0, d1, r0, r1) {
  const f = v => r0 + (v - d0) / (d1 - d0) * (r1 - r0);
  f.invert = p => d0 + (p - r0) / (r1 - r0) * (d1 - d0);
  return f;
}

// ---------- attribute tweening (replaces d3-transition/ease) ----------
const easeCubicOut = t => (--t) * t * t + 1;
const NUMRE = /-?\d*\.?\d+(?:e-?\d+)?/g;
function interp(from, to) {
  const fa = from.match(NUMRE), ta = to.match(NUMRE);
  if (!fa || !ta || fa.length !== ta.length) return null;
  const parts = to.split(NUMRE);           // static text between numbers
  const f = fa.map(Number), t = ta.map(Number);
  return e => {
    let s = parts[0];
    for (let i = 0; i < t.length; i++)
      s += (f[i] + (t[i] - f[i]) * e) + parts[i + 1];
    return s;
  };
}
const ANIMS = new Set();
let RAF = null;
function stepAnims(now) {
  for (const a of ANIMS) {
    // Clamped below as well as above: an rAF timestamp is the frame's
    // vsync time and routinely precedes the performance.now() captured
    // when the tween was scheduled, and a negative t through the cubic
    // ease overshoots backward — a one-frame flash away from the start
    // state on every animation begun from an input handler.
    const t = Math.min(1, Math.max(0, (now - a.t0) / a.dur)), e = easeCubicOut(t);
    for (const [k, fn] of a.props) a.el.setAttribute(k, fn(e));
    if (t >= 1) ANIMS.delete(a);
  }
  RAF = ANIMS.size ? requestAnimationFrame(stepAnims) : null;
}
function tween(el, props, dur) {
  for (const a of ANIMS) if (a.el === el) ANIMS.delete(a);
  const compiled = [];
  for (const k in props) {
    const to = String(props[k]);
    const from = el.getAttribute(k);
    if (from == null || from === to || dur <= 0) { el.setAttribute(k, to); continue; }
    const fn = interp(from, to);
    if (!fn) { el.setAttribute(k, to); continue; }
    compiled.push([k, fn]);
  }
  if (!compiled.length) return;
  ANIMS.add({ el, props: compiled, t0: performance.now(), dur });
  if (!RAF) RAF = requestAnimationFrame(stepAnims);
}

// ---------- monotone cubic path (replaces d3-shape curveMonotoneX) ----------
// Fritsch–Carlson tangents; port of d3-shape's monotoneX, string-emitting.
// `vertical` transposes the axes' roles (d3's monotoneY): the curve is
// single-valued along y instead of x, which is what a violin silhouette
// needs — it doubles back in x but never in y.
function monotonePathAxis(pts0, vertical) {
  const pts = vertical ? pts0.map(p => [p[1], p[0]]) : pts0;
  const XY = (a, b) => vertical
    ? b.toFixed(2) + "," + a.toFixed(2)
    : a.toFixed(2) + "," + b.toFixed(2);
  const n = pts.length;
  if (n === 0) return "";
  if (n === 1) return "M" + XY(pts[0][0], pts[0][1]);
  const sign = x => x < 0 ? -1 : 1;
  const slope3 = (x0, y0, x1, y1, x2, y2) => {
    const h0 = x1 - x0, h1 = x2 - x1;
    const s0 = (y1 - y0) / (h0 || (h1 < 0 && -0));
    const s1 = (y2 - y1) / (h1 || (h0 < 0 && -0));
    const p = (s0 * h1 + s1 * h0) / (h0 + h1);
    return (sign(s0) + sign(s1)) * Math.min(Math.abs(s0), Math.abs(s1), 0.5 * Math.abs(p)) || 0;
  };
  const slope2 = (x0, y0, x1, y1, t) => {
    const h = x1 - x0;
    return h ? (3 * (y1 - y0) / h - t) / 2 : t;
  };
  let d = "M" + XY(pts[0][0], pts[0][1]);
  const bez = (x0, y0, x1, y1, t0, t1) => {
    const dx = (x1 - x0) / 3;
    d += "C" + XY(x0 + dx, y0 + dx * t0) +
         "," + XY(x1 - dx, y1 - dx * t1) +
         "," + XY(x1, y1);
  };
  let t0;
  for (let i = 1; i < n; i++) {
    const [x0, y0] = pts[i - 1], [x1, y1] = pts[i];
    let t1;
    if (i < n - 1) t1 = slope3(x0, y0, x1, y1, pts[i + 1][0], pts[i + 1][1]);
    else t1 = slope2(x0, y0, x1, y1, t0 === undefined ? 0 : t0);
    if (i === 1) t0 = slope2(x0, y0, x1, y1, t1);
    bez(x0, y0, x1, y1, t0, t1);
    t0 = t1;
  }
  return d;
}
const monotonePath = pts => monotonePathAxis(pts, false);
const monotonePathV = pts => monotonePathAxis(pts, true);

// ---------- log-domain arithmetic: {s: sign, l: log2|x|} ----------
const LN = {
  real(v) { return v === 0 ? { s: 1, l: -Infinity } : { s: v < 0 ? -1 : 1, l: Math.log2(Math.abs(v)) }; },
  neg(a) { return { s: -a.s, l: a.l }; },
  mul(a, b) { return { s: a.s * b.s, l: a.l + b.l }; },
  div(a, b) { return { s: a.s * b.s, l: a.l - b.l }; },
  add(a, b) {
    if (a.l === -Infinity) return b;
    if (b.l === -Infinity) return a;
    const hi = a.l >= b.l ? a : b, lo = a.l >= b.l ? b : a;
    const d = Math.pow(2, lo.l - hi.l);
    if (a.s === b.s) return { s: a.s, l: hi.l + Math.log2(1 + d) };
    if (a.l === b.l) return { s: 1, l: -Infinity };
    return { s: hi.s, l: hi.l + Math.log2(1 - d) };
  },
  sub(a, b) { return LN.add(a, LN.neg(b)); },
  pow(a, b) {
    const e = b.s * Math.pow(2, b.l);
    if (!isFinite(e)) throw "exponent overflows";
    if (a.l === -Infinity) return { s: 1, l: e > 0 ? -Infinity : Infinity };
    if (a.s < 0) throw "negative base under ^";
    return { s: 1, l: e * a.l };
  },
};
const LNFN = {
  log2: a => { if (a.s <= 0) throw "log of non-positive value"; return LN.real(a.l); },
  ln: a => { if (a.s <= 0) throw "log of non-positive value"; return LN.real(a.l * Math.LN2); },
  log10: a => { if (a.s <= 0) throw "log of non-positive value"; return LN.real(a.l * Math.log10(2)); },
  sqrt: a => { if (a.s < 0) throw "sqrt of negative value"; return { s: 1, l: a.l / 2 }; },
};
LNFN.log = LNFN.log2; LNFN.lg = LNFN.log2;

// ---------- expression grammar (compiles to n -> LogNum) ----------
function parseBound(src) {
  const toks = [];
  const re = /\s*([0-9]*\.?[0-9]+|[A-Za-z][A-Za-z0-9]*|\*\*|[-+*/^()\u00b7])/y;
  let i = 0;
  while (i < src.length) {
    re.lastIndex = i;
    const m = re.exec(src);
    if (!m) return { err: 'unexpected "' + src[i] + '"' };
    toks.push(m[1] === "**" ? "^" : m[1] === "\u00b7" ? "*" : m[1]);
    i = re.lastIndex;
  }
  if (!/\S/.test(src)) return { err: "empty expression" };
  let p = 0;
  const peek = () => toks[p];
  const FN = LNFN;
  function primary() {
    const t = toks[p];
    if (t === undefined) throw "incomplete expression";
    if (/^[0-9]/.test(t)) { p++; const v = LN.real(parseFloat(t)); return () => v; }
    if (t === "(") {
      p++; const e = expr();
      if (toks[p] !== ")") throw 'missing ")"';
      p++; return e;
    }
    if (t === "n" || t === "N") { p++; return n => LN.real(n); }
    throw 'unknown "' + t + '" \u2014 try n, log, ln, log10, sqrt';
  }
  // paren-free application with calculator precedence: operators bind
  // tighter than application, so "log n^2" = log(n^2) and "log log n"
  // nests, while multiplication is not absorbed: "n log n" = n * log n
  function funcapp() {
    const t = toks[p];
    if (t !== undefined && FN[t]) {
      p++;
      let arg;
      if (toks[p] === "(") {
        p++; arg = expr();
        if (toks[p] !== ")") throw 'missing ")"';
        p++;
      } else {
        arg = unary();
      }
      const fn = FN[t];
      return n => fn(arg(n));
    }
    return primary();
  }
  function power() {
    const base = funcapp();
    if (peek() === "^") { p++; const ex = unary(); return n => LN.pow(base(n), ex(n)); }
    return base;
  }
  function unary() {
    if (peek() === "-") { p++; const e = unary(); return n => LN.neg(e(n)); }
    if (peek() === "+") { p++; return unary(); }
    return power();
  }
  function term() {
    let l = unary();
    for (;;) {
      const t = peek();
      if (t === "*") { p++; const r = unary(); const a = l; l = n => LN.mul(a(n), r(n)); }
      else if (t === "/") { p++; const r = unary(); const a = l; l = n => LN.div(a(n), r(n)); }
      else if (t !== undefined && t !== ")" && (t === "(" || /^[0-9A-Za-z]/.test(t))) {
        const r = unary(); const a = l; l = n => LN.mul(a(n), r(n));
      } else break;
    }
    return l;
  }
  function expr() {
    let l = term();
    for (;;) {
      const t = peek();
      if (t === "+") { p++; const r = term(); const a = l; l = n => LN.add(a(n), r(n)); }
      else if (t === "-") { p++; const r = term(); const a = l; l = n => LN.sub(a(n), r(n)); }
      else break;
    }
    return l;
  }
  try {
    const f = expr();
    if (p !== toks.length) return { err: "unexpected '" + toks[p] + "'" };
    return { f };
  } catch (e) { return { err: String(e) }; }
}

// Whether a hypothesis is usable over the measured sizes, and the
// *lift* that makes it so: the smallest constant c such that
// g(n + c) >= 1 at every measured size. "n log n" is the obvious way
// to write that bound even though it is 0 at n=1; shifting the
// argument by a constant is asymptotically identity, and the >= 1
// floor (rather than bare positivity, which has no smallest satisfying
// constant: it is an open condition, and log-compensating a
// nearly-zero value would blow the chart's y-range) keeps every
// compensation log-shift nonnegative and smooth. Formulae already
// valid everywhere get lift 0.
function acceptBound(src, sizes) {
  const r = parseBound(src);
  if (r.err) return r;
  // min over the measured sizes of log2 g(n + c), or -Infinity where
  // any evaluation is non-positive, non-finite, or throws
  const minLog = c => {
    let lo = Infinity;
    for (const n of sizes) {
      let v;
      try { v = r.f(n + c); } catch (e) { return -Infinity; }
      if (!(v.s > 0) || !isFinite(v.l)) return -Infinity;
      lo = Math.min(lo, v.l);
    }
    return lo;
  };
  let lift = 0;
  if (minLog(0) < 0) {
    // bracket the crossing on a log-spaced grid, then bisect; the
    // final candidate is verified against every size, so a
    // non-monotone formula degrades to a valid (if not provably
    // minimal) lift rather than a wrong one
    let lo = 0, hi = null;
    for (let c = 1e-6; c <= 65536; c *= 2) {
      if (minLog(c) >= 0) { hi = c; break; }
      lo = c;
    }
    if (hi === null) {
      const n = sizes[sizes.length - 1];
      return { err: "no constant shift makes this positive at n=" + n };
    }
    for (let i = 0; i < 40 && hi - lo > 1e-9; i++) {
      const mid = (lo + hi) / 2;
      if (minLog(mid) >= 0) hi = mid;
      else lo = mid;
    }
    lift = hi;
  }
  const f = r.f;
  return { f: n => f(n + lift), lift };
}

// A hypothesis value as a log2 for geometry: trace sampling extends
// below the smallest measured size, where even a lifted bound may go
// non-positive; such points dive below the plot instead of plotting a
// meaningless height. At measured sizes the lift guarantees a finite
// nonnegative value, so compensation never hits the guard.
function boundLog(f, n) {
  let v;
  try { v = f(n); } catch (e) { return -Infinity; }
  return v.s > 0 && isFinite(v.l) ? v.l : -Infinity;
}


function densRamp(dark) {
  const stops = dark
    ? [[0, [58, 70, 92]], [0.35, [88, 124, 176]], [0.7, [132, 170, 220]], [1, [190, 218, 250]]]
    : [[0, [227, 236, 247]], [0.35, [160, 190, 224]], [0.7, [80, 122, 176]], [1, [22, 60, 105]]];
  return v => {
    let a = stops[0], b = stops[stops.length - 1];
    for (let i = 0; i < stops.length - 1; i++)
      if (v >= stops[i][0] && v <= stops[i + 1][0]) { a = stops[i]; b = stops[i + 1]; break; }
    const t = (v - a[0]) / (b[0] - a[0] || 1);
    return "rgb(" + a[1].map((x, i) => Math.round(x + (b[1][i] - x) * t)).join(",") + ")";
  };
}

// centered gaussian smoothing with mirror-extension padding: endpoints are
// preserved exactly (so trace ends agree with the raw right-hand labels and
// the guide anchor), while interior sampling noise is averaged out
function smoothSeries(vals, radius, sigma) {
  const n = vals.length;
  const at = j => {
    if (j >= 0 && j < n) return vals[j];
    if (j < 0) return 2 * vals[0] - vals[-j];
    return 2 * vals[n - 1] - vals[2 * (n - 1) - j];
  };
  const out = new Array(n);
  for (let i = 0; i < n; i++) {
    let acc = 0, wsum = 0;
    for (let d = -radius; d <= radius; d++) {
      const w = Math.exp(-(d * d) / (2 * sigma * sigma));
      acc += at(i + d) * w; wsum += w;
    }
    out[i] = acc / wsum;
  }
  return out;
}



// ---------- formatting ----------
// A platform-transferable ratio from a log2 difference: spread and
// compensation readouts show these, never absolute counts.
function fmtRatio(l) {
  if (!isFinite(l)) return "?";
  const v = Math.pow(2, l);
  return "×" + (v >= 100 ? Math.round(v).toLocaleString("en-US") : v.toPrecision(2));
}
function pow2Text(t, k) {
  t.textContent = "2";
  const ts = mk(t, "tspan");
  attrs(ts, { dy: "-0.45em", "font-size": "72%" });
  ts.textContent = String(k);
}

const W = 960, H = 480;
// The left margin holds only the rotated axis label (the y-axis is
// deliberately unnumbered); the right margin is exactly the quantile
// slider column, so the plot fills the card. The column is sized to
// its widest label, "maximum", set in the slider's mono at 11pt.
const M = { l: 46, r: 108, t: 34, b: 60 };
const PW = W - M.l - M.r, PH = H - M.t - M.b;
const SLX = 22;
let UID = 0;

// ---------- widget ----------
class Widget {
  constructor(host, data) {
    // res is required, never defaulted: a dataset missing its binning
    // resolution would render at the wrong bin height forever, silently.
    if (!(typeof data.res === "number" && data.res > 0))
      throw new Error("fuelscape dataset carries no positive res");
    this.data = data;
    this.res = data.res;
    this.sizes = data.sizes;
    this.lx = this.sizes.map(Math.log2);
    this.uid = ++UID;
    this.detectTheme();
    if (typeof MutationObserver === "function") {
      new MutationObserver(() => this.retheme())
        .observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    }
    if (typeof matchMedia === "function")
      matchMedia("(prefers-color-scheme: dark)").addEventListener?.("change", () => this.retheme());

    this.q = 0.5;
    // The anchor column: where guides pin to the probe and what the
    // slider spans. Defaults to the largest n (the asymptotic end);
    // clicking a column locks it there instead, clicking again unlocks.
    // Locking is tracked apart from the index so the default column
    // reads unhighlighted until a click deliberately locks it: the
    // rightmost lock is a no-op for the geometry, kept for the visual
    // symmetry of every column answering a click the same way.
    this.anchorIdx = data.sizes.length - 1;
    this.locked = false;
    this._hoverCol = null;
    // Density display mode: constant-width columns (the default), or
    // violin silhouettes — the same clip paths with every half-width
    // forced full in column mode, so the toggle tweens between the two.
    this.violin = false;
    this.qref = {};
    for (const [key, qq] of [["min", 0], ["med", 0.5], ["max", 1]])
      this.qref[key] = data.cols.map(col => this.quantAt(col, qq));

    this.guides = [];
    for (const g of ["1", "n", "n log n", "n^2"]) this.addGuideSilent(g);
    const def = (data.default || "").trim();
    if (def && !this.guides.some(g => g.text === def)) this.addGuideSilent(def);
    this.active = this.guides.some(g => g.text === def) ? def : null;

    this.buildDom(host);
    this.buildStatic();
    this.update(false);
  }

  detectTheme() {
    const rdTheme = document.documentElement.getAttribute("data-theme");
    this.dark = rdTheme ? rdTheme !== "light"
      : (typeof matchMedia === "function" && matchMedia("(prefers-color-scheme: dark)").matches);
    this.ramp = densRamp(this.dark);
  }

  retheme() {
    const was = this.dark;
    this.detectTheme();
    if (this.dark !== was && this.binEls)
      for (const [el, v] of this.binEls) el.setAttribute("fill", this.ramp(v));
  }

  quantAt(col, q) {
    const res = this.res;
    if (q <= 0) {
      for (let j = 0; j < col.c.length; j++) if (col.c[j]) return (col.k0 + j) * res;
    }
    if (q >= 1) {
      for (let j = col.c.length - 1; j >= 0; j--) if (col.c[j]) return (col.k0 + j + 1) * res;
    }
    const total = col.c.reduce((a, b) => a + b, 0);
    let target = q * total, cum = 0;
    for (let j = 0; j < col.c.length; j++) {
      if (cum + col.c[j] >= target) {
        const frac = col.c[j] ? (target - cum) / col.c[j] : 0.5;
        return (col.k0 + j + frac) * res;
      }
      cum += col.c[j];
    }
    return (col.k0 + col.c.length) * res;
  }

  // The quantile of log-fuel `ly` within column `ci`: the inverse of
  // quantAt, so a pointer height converts to a probe quantile.
  cdfAt(ci, ly) {
    const col = this.data.cols[ci];
    if (ly <= this.qref.min[ci] + 1e-9) return 0;
    if (ly >= this.qref.max[ci] - 1e-9) return 1;
    const total = col.c.reduce((a, b) => a + b, 0);
    const pos = ly / this.res - col.k0;
    let cum = 0;
    for (let j = 0; j < col.c.length; j++) {
      if (j + 1 <= pos) { cum += col.c[j]; continue; }
      if (j < pos) cum += col.c[j] * (pos - j);
      break;
    }
    return Math.min(1, Math.max(0, cum / total));
  }

  traceValues(q) {
    const raw = this.data.cols.map(col => this.quantAt(col, q));
    const sm = (q <= 0 || q >= 1) ? raw.slice() : smoothSeries(raw, 2, 0.9);
    return { raw, sm };
  }

  setAnchorValue(lyRaw) {
    const li = this.anchorIdx;
    const lo = this.qref.min[li], hi = this.qref.max[li], med = this.qref.med[li];
    let ly = Math.min(hi, Math.max(lo, lyRaw));
    if (this.Y && this.anchorShift !== undefined &&
        Math.abs(this.Y(ly - this.anchorShift) - this.Y(med - this.anchorShift)) < 3) ly = med;
    this.q = ly === med ? 0.5 : this.cdfAt(li, ly);
    this.update(false);
  }

  // Lock the guide/probe anchor to column `ci`; clicking the locked
  // column again unlocks, sending the anchor back to the default (the
  // largest n).
  toggleAnchor(ci) {
    if (this.locked && ci === this.anchorIdx) {
      this.locked = false;
      this.anchorIdx = this.sizes.length - 1;
    } else {
      this.locked = true;
      this.anchorIdx = ci;
    }
    this.update(true);
  }

  setMode(m) {
    const violin = m === "violin";
    if (violin === this.violin) return;
    this.violin = violin;
    this.syncModeBtns();
    this.update(true);
  }

  syncModeBtns() {
    for (const m in this.modeBtns) {
      const on = (m === "violin") === this.violin;
      this.modeBtns[m].classList.toggle("fs-on", on);
      this.modeBtns[m].setAttribute("aria-pressed", String(on));
    }
  }

  setQ(q) {
    this.q = Math.min(1, Math.max(0, q));
    this.update(false);
  }

  // Up/down quantile stepping, shared by every chart surface that can
  // hold focus — the probe answers arrows no matter what is selected.
  // One percentile per press, five with shift. Returns whether the key
  // was consumed.
  quantKey(ev) {
    const step = ev.shiftKey ? 0.05 : 0.01;
    if (ev.key === "ArrowUp") this.setQ(Math.round((this.q + step) * 100) / 100);
    else if (ev.key === "ArrowDown") this.setQ(Math.round((this.q - step) * 100) / 100);
    else return false;
    ev.preventDefault();
    return true;
  }

  // Wires a press-drag: `move` runs on every pointermove until the
  // press ends, `end` once when it does. The pointer is captured so a
  // release outside the window (or a pointercancel) still ends the
  // drag: an unfinished drag would leave `move` running on every later
  // idle mouse motion, repainting the widget forever.
  trackPointer(ev, move, end) {
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", up);
      window.removeEventListener("blur", up);
      if (end) end();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", up);
    // A release the browser never delivers (focus stolen mid-press,
    // capture dropped) must still end the drag: blur is the backstop.
    window.addEventListener("blur", up);
    if (ev.pointerId !== undefined && ev.target && ev.target.setPointerCapture) {
      try { ev.target.setPointerCapture(ev.pointerId); } catch (e) { /* detached target */ }
    }
    if (ev.preventDefault) ev.preventDefault();
  }

  // Dragging the probe trace: the pointer's height converts to a
  // quantile through the nearest column's own distribution, with the
  // same snap-to-median assist as the anchor drag.
  dragTrace(ev) {
    const svgEl = this.svg;
    const move = e => {
      const rect = svgEl.getBoundingClientRect();
      const px = (e.clientX - rect.left) * (W / (rect.width || W));
      const py = (e.clientY - rect.top) * (H / (rect.height || H));
      const lx = this.X.invert(px);
      let ci = 0;
      for (let i = 1; i < this.lx.length; i++)
        if (Math.abs(this.lx[i] - lx) < Math.abs(this.lx[ci] - lx)) ci = i;
      const shift = this.shiftAt(this.sizes[ci]);
      const ly = Math.min(
        this.qref.max[ci],
        Math.max(this.qref.min[ci], this.Y.invert(py) + shift)
      );
      const med = this.qref.med[ci];
      if (Math.abs(this.Y(ly - shift) - this.Y(med - shift)) < 3) this.setQ(0.5);
      else this.setQ(this.cdfAt(ci, ly));
    };
    this.trackPointer(ev, move);
  }

  // Dragging a guide line sweeps quantiles by keeping the grabbed line
  // under the pointer: every guide is drawn offset so it crosses the
  // probe at the anchor, so the anchor value is solved from requiring
  // the guide's curve to pass through the pointer at the pointer's own
  // x. For the selected (compensated-flat) hypothesis the relative
  // term vanishes and this is exactly the vertical anchor drag.
  dragGuide(ev, g) {
    const svgEl = this.svg;
    const move = e => {
      const rect = svgEl.getBoundingClientRect();
      const px = (e.clientX - rect.left) * (W / (rect.width || W));
      const py = (e.clientY - rect.top) * (H / (rect.height || H));
      if (Math.abs(e.clientY - ev.clientY) > 3 ||
          Math.abs(e.clientX - ev.clientX) > 3) this._dragMoved = true;
      if (!this._dragMoved) return;
      const rel = n => boundLog(g.f, n) - this.shiftAt(n);
      const aN = this.sizes[this.anchorIdx];
      const nPtr = Math.pow(2, this.X.invert(px));
      this.setAnchorValue(
        this.Y.invert(py) + this.anchorShift + rel(aN) - rel(nPtr));
    };
    this._dragMoved = false;
    this.trackPointer(ev, move, () => {
      if (this._dragMoved) this._squelch = true;
      this._dragMoved = false;
    });
  }

  dragStart(ev) {
    const svgEl = this.svg;
    const move = e => {
      const rect = svgEl.getBoundingClientRect();
      const py = (e.clientY - rect.top) * (H / (rect.height || H));
      if (Math.abs(e.clientY - ev.clientY) > 3) this._dragMoved = true;
      if (this._dragMoved)
        this.setAnchorValue(this.Y.invert(py) + this.anchorShift);
    };
    this._dragMoved = false;
    this.trackPointer(ev, move, () => {
      if (this._dragMoved) this._squelch = true;
      this._dragMoved = false;
    });
  }

  addGuideSilent(text) {
    const r = acceptBound(text, this.sizes);
    if (r.err) return r.err;
    // sort key: clamped log-slope across the measured range (constants
    // cancel), tiebroken by magnitude at the largest n, then text
    const l0 = boundLog(r.f, this.sizes[0]);
    const l1 = boundLog(r.f, this.sizes[this.sizes.length - 1]);
    this.guides.push({ text: text.trim(), f: r.f, slope: l1 - l0, end: l1 });
    this.guides.sort((a, b) => a.slope - b.slope || a.end - b.end || (a.text < b.text ? -1 : 1));
    return null;
  }

  setActive(text) {
    this.active = text === this.active ? null : text;
    this.update(true);
  }

  removeGuide(text) {
    this.guides = this.guides.filter(g => g.text !== text);
    if (this.active === text) this.active = null;
    this.update(true);
  }

  // Keyboard deletion: removing the focused element would strand focus
  // on the document body, so hand it to the neighbor that slid into
  // the removed guide's place — the entry input when none remains.
  removeGuideFocus(text, kind) {
    const idx = this.guides.findIndex(g => g.text === text);
    this.removeGuide(text);
    const nb = this.guides[Math.min(idx, this.guides.length - 1)];
    if (!nb) this.addInput.focus();
    else if (kind === "chip") this.chipEls.get(nb.text).btn.focus();
    else this.hitEls.get(nb.text).poly.focus();
  }

  isRaw() { return this.active === null; }
  shiftAt(n) {
    if (this.active === null) return 0;
    // Total for safety, but the guard is unreachable at measured sizes:
    // acceptance lifts every guide to >= 1 there.
    const l = boundLog(this.guides.find(g => g.text === this.active).f, n);
    return isFinite(l) ? l : 0;
  }

  // The chart's own identity, derived from where its island renders:
  // the enclosing rustdoc item section names the method and provides a
  // same-page anchor, the page URL names the owning type — together
  // "Clock::tick" at "#method.tick", with nothing to hand-maintain.
  // Islands outside any item section (the crate docs' Complexity tour)
  // and pages whose DOM this doesn't recognize get no suffix — every
  // reading of this identity degrades to the bare title.
  itemIdentity(host) {
    const page = (location.pathname.split("/").pop() || "").split(".");
    if (page.length !== 3 || page[2] !== "html") return null;
    const [kind, item] = page;
    const sec = host.closest("details.method-toggle")
      ?.querySelector(":scope > summary > section[id]");
    const name = sec?.querySelector(".code-header a.fn");
    if (sec && name) return { path: item + "::" + name.textContent, href: "#" + sec.id };
    if (kind === "fn") return { path: item, href: "" };
    return null;
  }

  buildDom(host) {
    host.classList.add("fs-root");
    const title = mk(host, "h5", "fs-title");
    title.textContent = "Measured growth";
    const id = this.itemIdentity(host);
    if (id) {
      title.appendChild(document.createTextNode(": "));
      const holder = id.href ? mk(title, "a") : title;
      if (id.href) holder.href = id.href;
      mk(holder, "code").textContent = id.path;
    }
    const sub = mk(host, "div", "fs-subtitle");
    sub.appendChild(document.createTextNode(
      "Flat bands indicate agreement with the selected hypothesis ("));
    // The full explanation lives in the crate docs' Complexity section;
    // rustdoc's own root-path meta bridges the varying page depths
    // (these islands render only in before's docs).
    const more = mk(sub, "a");
    const vars = document.querySelector('meta[name="rustdoc-vars"]');
    more.href = ((vars && vars.dataset.rootPath) || "../") + "before/index.html#complexity";
    mk(more, "i").textContent = "more details";
    sub.appendChild(document.createTextNode(")"));
    const bar1 = mk(host, "div", "fs-bar");
    const hyp = mk(bar1, "span", "fs-hyplabel");
    hyp.title = "divide instructions by a growth rate \u2014 the right one flattens the band";
    hyp.textContent = "Hypothesis:";
    this.chipBox = mk(bar1, "span", "fs-chips");
    this.entryWrap = mk(this.chipBox, "span", "fs-chip fs-newchip");
    this.addInput = mk(this.entryWrap, "input", "fs-expr");
    attrs(this.addInput, { type: "text", spellcheck: "false",
      placeholder: "n^1.5, 2^n, sqrt n log n \u2026",
      "aria-label": "add a compensation guide expression" });
    this.addInput.addEventListener("keydown", ev => { if (ev.key === "Enter") this.tryAdd(); });
    const plus = mk(this.entryWrap, "span", "fs-chipplus");
    plus.title = "add this hypothesis"; plus.textContent = "+";
    plus.addEventListener("click", () => this.tryAdd());
    this.errEl = mk(this.entryWrap, "span", "fs-err");
    this.errEl.setAttribute("role", "alert");

    this.svg = mk(host, "svg");
    attrs(this.svg, { viewBox: `0 0 ${W} ${H}`, width: "100%", role: "img",
      "aria-label": this.data.name + ": distribution of instruction counts by input size, log-log" });
    this.footer = mk(host, "div", "fs-footer");
    const modeWrap = mk(this.footer, "span", "fs-modewrap");
    mk(modeWrap, "span", "fs-modelabel").textContent = "Display:";
    const mode = mk(modeWrap, "span", "fs-mode");
    attrs(mode, { role: "group", "aria-label": "density display mode" });
    this.modeBtns = {};
    for (const m of ["column", "violin"]) {
      const b = mk(mode, "button", "fs-modebtn");
      b.type = "button";
      b.textContent = m;
      b.title = "draw each distribution as " +
        (m === "violin" ? "a width-for-density violin" : "a constant-width column");
      b.addEventListener("click", () => this.setMode(m));
      this.modeBtns[m] = b;
    }
    this.syncModeBtns();
    const d = this.data;
    // One quiet provenance line; the details (full commit, the size
    // measure's exact denominator, the shapes-not-absolutes caveat)
    // live in its tooltip rather than as visible noise.
    const prov = mk(this.footer, "span", "fs-prov");
    prov.innerHTML = "commit <code>" + String(d.commit).slice(0, 8) + "</code> \u00b7 seed <code>" +
      d.seed + "</code> \u00b7 avg <code>" + d.spc + "</code> samples/column";
    prov.title = "commit " + d.commit + "\nsize measure: " + d.size_measure +
      "\ninstructions are counted as WASM operations, deterministically, in a " +
      "sandboxed build of this crate: growth shapes and ratios transfer to native " +
      "builds; absolute counts do not";
  }

  tryAdd() {
    const text = this.addInput.value.trim();
    if (!text) return;
    if (this.guides.some(g => g.text === text)) {
      if (this.active !== text) { this.active = text; this.update(true); }
      this.addInput.value = ""; this.errEl.textContent = "";
      this.entryWrap.classList.remove("fs-bad"); return;
    }
    const err = this.addGuideSilent(text);
    if (err) { this.errEl.textContent = err; this.entryWrap.classList.add("fs-bad"); return; }
    this.errEl.textContent = ""; this.entryWrap.classList.remove("fs-bad");
    this.addInput.value = "";
    this.active = text;
    this._justAdded = text;
    this.update(true);
  }

  buildStatic() {
    const svg = this.svg;
    const clipId = "fsclip" + this.uid, blurId = "fsblur" + this.uid;
    const defs = mk(svg, "defs");
    const clip = mk(defs, "clipPath"); clip.setAttribute("id", clipId);
    attrs(mk(clip, "rect"), { x: M.l, y: M.t - 2, width: PW, height: PH + 4 });
    // The guides get their own exact plot-box clip, no vertical slack:
    // a sloped stroke overhanging the border reads as sticking out of
    // the chart, and the clip's cut runs along the border itself (an
    // occlusion edge), never across the line's own direction the way a
    // shortened polyline's end cap would. The padded clip above stays
    // for the density and probe layers, whose marks at the data
    // extremes must not be shaved.
    const clipExact = mk(defs, "clipPath"); clipExact.setAttribute("id", clipId + "g");
    attrs(mk(clipExact, "rect"), { x: M.l, y: M.t, width: PW, height: PH });
    const filt = mk(defs, "filter");
    attrs(filt, { id: blurId, x: "-4%", y: "-4%", width: "108%", height: "108%" });
    // Vertical-only blur; its width is set per repaint, scaled to the
    // bin height (update()), so tight-range charts with tall bins soften
    // the same as wide-range charts with hairline bins.
    this.blurEl = attrs(mk(filt, "feGaussianBlur"), { stdDeviation: "0 0.8" });

    // stacking, bottom to top: grid < density < guides < quantile probe
    // < column hover < guide hit strokes < labels < slider
    attrs(mk(svg, "rect", "fs-plotbg"), { x: M.l, y: M.t, width: PW, height: PH });
    this.gGrid = mk(svg, "g");
    this.gCols = mk(svg, "g");
    attrs(this.gCols, { "clip-path": `url(#${clipId})`, filter: `url(#${blurId})` });
    this.gGuides = mk(svg, "g"); this.gGuides.setAttribute("clip-path", `url(#${clipId}g)`);
    this.gQuants = mk(svg, "g"); this.gQuants.setAttribute("clip-path", `url(#${clipId})`);
    this.gHover = mk(svg, "g");
    this.gHits = mk(svg, "g"); this.gHits.setAttribute("clip-path", `url(#${clipId})`);
    this.gLabels = mk(svg, "g");

    this.ylabel = mk(svg, "text", "fs-caption");
    attrs(this.ylabel, { transform: `translate(${M.l - 14},${M.t + PH / 2}) rotate(-90)`, "text-anchor": "middle" });
    const xcap = mk(svg, "text", "fs-caption");
    attrs(xcap, { x: M.l + PW / 2, y: H - 12, "text-anchor": "middle" });
    xcap.textContent = "total input size (bytes, log scale)";
    attrs(mk(svg, "line", "fs-axis"), { x1: M.l, x2: M.l, y1: M.t, y2: H - M.b });
    attrs(mk(svg, "line", "fs-axis"), { x1: M.l, x2: W - M.r, y1: H - M.b, y2: H - M.b });

    const x0 = this.lx[0] - 0.62, x1 = this.lx[this.lx.length - 1] + 0.62;
    this.X = scaleLinear(x0, x1, M.l, W - M.r);
    // The plot's full log-size domain: guides and the probe trace run
    // wall to wall, their stroke caps shaved crisp by the plot clips.
    this.lx0 = x0;
    this.lx1 = x1;
    const gaps = this.lx.slice(1).map((v, i) => v - this.lx[i]);
    const minGap = gaps.length ? Math.min(...gaps) : 1;
    this.colW = PW / (x1 - x0) * 0.86 * minGap;
    for (let ci = 0; ci < this.sizes.length; ci++) {
      const t = mk(svg, "text", "fs-tick");
      attrs(t, { "text-anchor": "middle", x: this.X(this.lx[ci]), y: H - M.b + 18 });
      if (this.sizes[ci] < 1024) t.textContent = String(this.sizes[ci]);
      else pow2Text(t, Math.round(this.lx[ci]));
    }

    // display bins: a truncated smoothing kernel plus a light SVG blur.
    // The kernel renormalizes at the support's edges rather than padding
    // past them: the painted band must end exactly where the data ends,
    // or the quantile slider (whose track is the true min-to-max span)
    // reads as stopping short of the band. Quantiles use raw counts.
    // The kernel's width is fixed in octaves of fuel, not bins, so a
    // re-binned dataset (compact.rs's RES) sharpens the profile's
    // resolution without changing how much the display smooths.
    const SIG_OCT = 0.05;
    const SIG = SIG_OCT / this.res, R = Math.max(3, Math.ceil(3 * SIG));
    const kern = [];
    for (let i = -R; i <= R; i++) kern.push(Math.exp(-(i * i) / (2 * SIG * SIG)));
    this.bins = [];
    this.violins = [];
    this.smExt = [];
    for (let ci = 0; ci < this.data.cols.length; ci++) {
      const col = this.data.cols[ci];
      const len = col.c.length;
      const sm = new Array(len).fill(0);
      for (let j = 0; j < len; j++) {
        let acc = 0, wsum = 0;
        for (let i = -R; i <= R; i++) {
          const k = j + i;
          if (k < 0 || k >= len) continue;
          acc += col.c[k] * kern[i + R];
          wsum += kern[i + R];
        }
        sm[j] = acc / wsum;
      }
      const peak = Math.max(...sm);
      // The violin profile: a half-width fraction at each bin center,
      // pinched to zero at the exact data extent so the painted shape
      // still ends where the data ends. Width takes a milder gamma than
      // color (0.8 vs 0.6): enough boost that thin tails don't vanish
      // into slivers, while the profile stays close to honest linear
      // density; color keeps the stronger boost so tails stay tinted.
      const prof = [[col.k0 * this.res, 0]];
      for (let j = 0; j < len; j++) {
        const v = sm[j] / peak;
        prof.push([(col.k0 + j + 0.5) * this.res,
          v <= 0 ? 0 : Math.pow(Math.max(v, 0.015), 0.8)]);
        if (v <= 0) continue;
        this.bins.push({ ci, vTop: (col.k0 + j + 1) * this.res,
          v: Math.pow(Math.max(v, 0.015), 0.6) });
      }
      prof.push([(col.k0 + len) * this.res, 0]);
      this.violins.push(prof);
      this.smExt.push([col.k0 * this.res, (col.k0 + len) * this.res]);
    }
    // Each column's color bands render at full column width and are
    // carved to the violin silhouette by a per-column clip; update()
    // shears the clip's path vertically exactly as it shears the bands.
    this.violEls = [];
    const colGs = [];
    for (let ci = 0; ci < this.sizes.length; ci++) {
      const cp = mk(defs, "clipPath");
      cp.setAttribute("id", `fsviol${this.uid}_${ci}`);
      this.violEls.push(mk(cp, "path"));
      const g = mk(this.gCols, "g");
      g.setAttribute("clip-path", `url(#fsviol${this.uid}_${ci})`);
      colGs.push(g);
    }
    this.binEls = new Map();
    for (const b of this.bins) {
      const r = mk(colGs[b.ci], "rect");
      attrs(r, { x: this.X(this.lx[b.ci]) - this.colW / 2, width: this.colW, fill: this.ramp(b.v) });
      r.__bin = b;
      this.binEls.set(r, b.v);
    }

    this.qHalo = mk(this.gQuants, "path", "fs-qhalo");
    this.qLine = mk(this.gQuants, "path", "fs-qline fs-q-med");
    // A fat invisible stroke over the probe trace: dragging the trace
    // itself sweeps quantiles, inverted through the density of whichever
    // column the pointer is nearest (so sensitivity follows the data).
    this.qHit = mk(this.gHits, "path", "fs-hit fs-active-hit fs-qhit");
    this.qHit.addEventListener("pointerdown", ev => this.dragTrace(ev));
    const qhTip = mk(this.qHit, "title");
    qhTip.textContent = "drag to sweep quantiles";
    this.gSlider = mk(svg, "g", "fs-slider");
    this.slTrack = mk(this.gSlider, "line", "fs-sl-track");
    this.slNotch = mk(this.gSlider, "line", "fs-sl-notch");
    this.slTickMin = mk(this.gSlider, "line", "fs-sl-tick");
    this.slTickMax = mk(this.gSlider, "line", "fs-sl-tick");
    this.slHandle = mk(this.gSlider, "circle", "fs-sl-handle");
    attrs(this.slHandle, { r: 6.5, cx: W - M.r + SLX, tabindex: 0, role: "slider",
      "aria-label": "quantile probe, arrow keys step one percentile" });
    this.slHandle.addEventListener("pointerdown", ev => this.dragStart(ev));
    this.slHandle.addEventListener("keydown", ev => {
      const step = ev.shiftKey ? 0.05 : 0.01;
      if (ev.key === "ArrowUp" || ev.key === "ArrowRight") this.setQ(Math.round((this.q + step) * 100) / 100);
      else if (ev.key === "ArrowDown" || ev.key === "ArrowLeft") this.setQ(Math.round((this.q - step) * 100) / 100);
      else if (ev.key === "Home") this.setQ(0);
      else if (ev.key === "End") this.setQ(1);
      else if (ev.key === "m" || ev.key === "M") this.setQ(0.5);
      else return;
      ev.preventDefault();
    });
    const htitle = mk(this.slHandle, "title");
    htitle.textContent = "drag to sweep P0\u2013P100 (snaps to median)";
    this.slCaption = mk(this.gSlider, "text", "fs-slcaption");
    this.slCaption.addEventListener("pointerdown", ev => this.dragStart(ev));
    this.qLabel = mk(this.gLabels, "text", "fs-qlabel");
    this.qLabel.setAttribute("x", W - M.r + SLX + 11);
    this.qLabel.addEventListener("pointerdown", ev => this.dragStart(ev));
    // The label's tooltip is created once here: a per-update mk() would
    // append one <title> child per repaint, and they accumulate.
    this.qLabelTip = mk(this.qLabel, "title");
    this.qLabelTip.textContent = "drag to sweep P0–P100";

    this.readout = mk(this.gLabels, "text", "fs-readout");
    attrs(this.readout, { x: W - M.r, y: M.t - 8, "text-anchor": "end" });
    this.hoverEls = [];
    this.colHits = [];
    this._colFocus = this.sizes.length - 1;
    for (let ci = 0; ci < this.sizes.length; ci++) {
      // The visible tint stays column-wide, but the pointer region
      // tiles the plot to the midlines between neighbors (edge columns
      // run to the plot border): every plot position has a nearest
      // column, so the hover and readout never blink out in a gap.
      const tint = mk(this.gHover, "rect", "fs-hoverrect");
      attrs(tint, { x: this.X(this.lx[ci]) - this.colW / 2 - 2, y: M.t,
        width: this.colW + 4, height: PH });
      const xl = ci === 0 ? M.l
        : (this.X(this.lx[ci - 1]) + this.X(this.lx[ci])) / 2;
      const xr = ci === this.sizes.length - 1 ? W - M.r
        : (this.X(this.lx[ci]) + this.X(this.lx[ci + 1])) / 2;
      // Roving tabindex: the column bank is a single tab stop (the
      // rightmost column by default), and arrows move the stop within.
      const hit = mk(this.gHover, "rect", "fs-hoverhit");
      attrs(hit, { x: xl, y: M.t, width: xr - xl, height: PH,
        tabindex: ci === this.sizes.length - 1 ? 0 : -1, role: "button",
        "aria-label": "input size " + this.sizes[ci].toLocaleString("en-US") +
          " column: arrow keys move between columns, space anchors" });
      hit.addEventListener("click", () => this.toggleAnchor(ci));
      // Keyboard column browsing: focus tints the column exactly as
      // pointer hover does (the aura is the highlight itself), arrows
      // walk the columns, space or enter toggles the anchor lock.
      hit.addEventListener("focus", () => {
        if (ci !== this._colFocus) {
          this.colHits[this._colFocus].setAttribute("tabindex", -1);
          this._colFocus = ci;
          hit.setAttribute("tabindex", 0);
        }
        this.setHoverCol(ci);
      });
      hit.addEventListener("blur", () => {
        if (this._hoverCol === ci) this.setHoverCol(null);
      });
      hit.addEventListener("keydown", ev => {
        if (this.quantKey(ev)) return;
        const d = ev.key === "ArrowLeft" ? -1 : ev.key === "ArrowRight" ? 1 : null;
        if (d !== null) {
          const nb = this.colHits[ci + d];
          if (nb) nb.focus();
        } else if (ev.key === "Enter" || ev.key === " ") {
          this.toggleAnchor(ci);
        } else return;
        ev.preventDefault();   // arrows and space must not scroll the page
      });
      const tip = mk(hit, "title");
      tip.textContent = "click to anchor the hypotheses and quantile here";
      this.hoverEls.push(tint);
      this.colHits.push(hit);
    }
    // Column hover follows the pointer on the svg itself, never the
    // tiles' own enter/leave: the guide and probe hit strokes stack
    // above the tiles (they must, to stay grabbable), so tile events
    // would lose the hover — a flicker — at every line crossing.
    const hoverAt = e => {
      const rect = svg.getBoundingClientRect();
      const px = (e.clientX - rect.left) * (W / (rect.width || W));
      const py = (e.clientY - rect.top) * (H / (rect.height || H));
      let ci = null;
      if (px >= M.l && px <= W - M.r && py >= M.t && py <= H - M.b) {
        ci = 0;
        for (let i = 1; i < this.lx.length; i++)
          if (Math.abs(this.X(this.lx[i]) - px) < Math.abs(this.X(this.lx[ci]) - px)) ci = i;
      }
      const direct = !!(e.target && e.target.closest && e.target.closest(".fs-hoverhit"));
      this.setHoverCol(ci, direct);
    };
    svg.addEventListener("pointermove", hoverAt);
    svg.addEventListener("pointerleave", () => this.setHoverCol(null));

    // keyed element maps for dynamic layers
    this.tickEls = new Map();    // k -> {g, line, text}
    this.guideEls = new Map();   // text -> {g, halo, ref}
    this.hitEls = new Map();     // text -> {poly, title}
    this.glabelEls = new Map();  // text -> text el
    this.chipEls = new Map();    // text -> {btn, label, x}
  }

  // The hovered column, or null: tints it, and (unlocked only — while
  // a lock holds, the readout belongs to the locked column, steady for
  // screenshots) reads out its stats. `direct` says the column itself
  // is what a click would hit; when something else owns the click (a
  // guide or probe stroke on top), the tint softens so it can't
  // promise an anchor the click won't deliver — the readout stays
  // either way.
  setHoverCol(ci, direct = true) {
    if (ci === this._hoverCol && direct === this._hoverDirect) return;
    this._hoverCol = ci;
    this._hoverDirect = direct;
    this.hoverEls.forEach((r, i) => {
      r.classList.toggle("fs-hover", i === ci && direct);
      r.classList.toggle("fs-hover-soft", i === ci && !direct);
    });
    if (!this.locked) {
      if (ci === null) this.readout.textContent = "";
      else this.showReadout(ci);
    }
  }

  showReadout(ci) {
    // Ratios only: the column's min-to-max spread, and where the median
    // sits above the column minimum \u2014 both platform-transferable.
    // Prose labels set in the page's own face; the values in mono.
    const n = this.sizes[ci];
    const spread = fmtRatio(this.qref.max[ci] - this.qref.min[ci]);
    const medUp = fmtRatio(this.qref.med[ci] - this.qref.min[ci]);
    const medDown = fmtRatio(this.qref.max[ci] - this.qref.med[ci]);
    this.readout.textContent = "";
    // En spaces around the bullets (breathing room XML whitespace
    // collapsing would eat from plain spaces); three voices in one
    // line: semibold keys, receded connective prose (both on the
    // element's fill), and the values in ink-and-mono tspans.
    const seg = (cls, text) => {
      if (cls) mk(this.readout, "tspan", cls).textContent = text;
      else this.readout.appendChild(document.createTextNode(text));
    };
    seg("fs-rk", "input size: ");
    seg("fs-rv", n.toLocaleString("en-US"));
    seg(null, " bytes\u2002\u00b7\u2002");
    seg("fs-rk", "output spread: ");
    seg("fs-rv", spread);
    seg(null, "\u2002\u00b7\u2002");
    seg("fs-rk", "median: ");
    seg("fs-rv", medUp);
    seg(null, " above minimum, ");
    seg("fs-rv", medDown);
    seg(null, " below maximum");
  }

  update(animate) {
    const res = this.res;
    const shifts = this.sizes.map(n => this.shiftAt(n));
    const shiftOf = n => this.shiftAt(n);
    const raw = this.isRaw();
    if (this.readout) {
      if (this.locked) this.showReadout(this.anchorIdx);
      else if (this._hoverCol != null) this.showReadout(this._hoverCol);
      else this.readout.textContent = "";
    }

    let ylo = Infinity, yhi = -Infinity;
    this.smExt.forEach((ext, ci) => {
      ylo = Math.min(ylo, ext[0] - shifts[ci]);
      yhi = Math.max(yhi, ext[1] - shifts[ci]);
    });
    const pad = (yhi - ylo) * 0.03;
    ylo -= pad; yhi += pad;
    const Y = scaleLinear(ylo, yhi, H - M.b, M.t);
    this.Y = Y;

    const reduce = typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
    const noAnim = typeof window !== "undefined" && window.__FS_NO_ANIM;
    const dur = reduce ? 160 : 500;
    const T = (el, props) => {
      if (animate && !noAnim) tween(el, props, dur);
      else for (const k in props) el.setAttribute(k, props[k]);
    };

    this.ylabel.textContent = raw ? "instructions (log scale)"
      : `instructions / ${this.active}  (log scale)`;

    // gridlines at whole octaves, deliberately unnumbered: absolute
    // counts are a guest measure, and the constant-ratio spacing still
    // reads as log-scale structure
    const step = Math.max(1, Math.round((yhi - ylo) / 6));
    const ticks = new Set();
    for (let k = Math.ceil(ylo); k <= Math.floor(yhi); k += step) ticks.add(k);
    for (const [k, t] of this.tickEls)
      if (!ticks.has(k)) { t.g.remove(); this.tickEls.delete(k); }
    for (const k of ticks) {
      let t = this.tickEls.get(k);
      if (!t) {
        const g = mk(this.gGrid, "g");
        g.setAttribute("opacity", 0);
        const line = mk(g, "line", "fs-grid");
        attrs(line, { x1: M.l, x2: W - M.r, y1: Y(k), y2: Y(k) });
        t = { g, line };
        this.tickEls.set(k, t);
      }
      T(t.g, { opacity: 1 });
      T(t.line, { y1: Y(k), y2: Y(k) });
    }

    // density: pure vertical shear
    const binH = Math.max(res / (yhi - ylo) * PH + 0.35, 0.9);
    // Softening scaled to the bin, not the pixel: a fixed width reads
    // as unblurred stair-steps wherever a tight y-range makes bins
    // tall. Capped so the bleed stays under a bin and the band keeps
    // ending where the data does.
    this.blurEl.setAttribute(
      "stdDeviation",
      "0 " + Math.min(Math.max(0.25 * binH, 0.5), 2.4).toFixed(2)
    );
    for (const [el] of this.binEls) {
      const b = el.__bin;
      T(el, { y: Y(b.vTop - shifts[b.ci]), height: binH });
    }
    // The silhouettes shear with their bands: one smooth outline per
    // side, top-to-bottom down the right edge, bottom-to-top up the
    // left, meeting at the zero-width tips.
    this.violEls.forEach((p, ci) => {
      const cx = this.X(this.lx[ci]), half = this.colW / 2;
      const prof = this.violins[ci];
      const hw = w => this.violin ? w : 1;
      const right = [], left = [];
      for (let i = prof.length - 1; i >= 0; i--)
        right.push([cx + half * hw(prof[i][1]), Y(prof[i][0] - shifts[ci])]);
      for (const [v, w] of prof) left.push([cx - half * hw(w), Y(v - shifts[ci])]);
      T(p, { d: monotonePathV(right) + monotonePathV(left).replace(/^M/, "L") + "Z" });
    });

    // probe values + anchor: guides intersect the probe at the anchor
    // column — the largest n by default (the asymptotic end), or the
    // column a click locked
    const tv = this.traceValues(this.q);
    const li = this.anchorIdx;
    this.anchorShift = shifts[li];
    const aI = li;
    const aN = this.sizes[aI];
    const anchorLy = tv.raw[aI];

    // guide geometry
    const samples = 60;
    const lxLo = this.lx0, lxHi = this.lx1;
    const geom = new Map();
    for (const g of this.guides) {
      // The same clamped evaluation as compensation itself, so the
      // selected hypothesis renders exactly horizontal — left edge
      // included — and trace sampling below the smallest measured n
      // (where growth bounds go non-positive) flattens instead of
      // spiking.
      const rel = n => boundLog(g.f, n) - shiftOf(n);
      const off = anchorLy - shiftOf(aN) - rel(aN);
      const pts = [];
      const vals = [];
      let tip = null;
      // The clamp only keeps coordinates finite (bounds run to -inf
      // below their lift): it sits several plot-heights off-view, so
      // the corner it folds into the polyline — which moves against
      // the grain of a tween — can never be seen. A clamp at the plot
      // edge creases the line exactly where the reader is looking.
      const flee = 4 * (yhi - ylo) + 8;
      for (let i = 0; i <= samples; i++) {
        const lx = lxLo + (lxHi - lxLo) * i / samples;
        const v = off + rel(Math.pow(2, lx));
        vals.push([lx, v]);
        pts.push(this.X(lx).toFixed(1) + "," +
          Y(Math.min(yhi + flee, Math.max(ylo - flee, v))).toFixed(1));
      }
      for (let i = 0; i <= samples; i++) {
        const lx = lxLo + (lxHi - lxLo) * i / samples;
        const v = off + rel(Math.pow(2, lx));
        if (v >= ylo + 0.1 && v <= yhi - 0.1) { tip = [lx, v]; break; }
      }
      if (!tip)
        tip = [lxLo, Math.min(yhi - 0.15, Math.max(ylo + 0.15, off + rel(Math.pow(2, lxLo))))];
      geom.set(g.text, { points: pts.join(" "), tip, vals });
    }
    const hoverLink = (text, on) => {
      for (const [t, e] of this.guideEls) e.g.classList.toggle("fs-hover", on && t === text);
      for (const [t, el] of this.glabelEls) el.classList.toggle("fs-hover", on && t === text);
    };

    // guide traces
    const liveTexts = new Set(this.guides.map(g => g.text));
    for (const [t, e] of this.guideEls)
      if (!liveTexts.has(t)) { e.g.remove(); this.guideEls.delete(t); }
    for (const g of this.guides) {
      let e = this.guideEls.get(g.text);
      if (!e) {
        const grp = mk(this.gGuides, "g", "guide");
        e = { g: grp, halo: mk(grp, "polyline", "fs-refhalo"), ref: mk(grp, "polyline", "fs-ref") };
        e.halo.setAttribute("points", geom.get(g.text).points);
        e.ref.setAttribute("points", geom.get(g.text).points);
        this.guideEls.set(g.text, e);
      }
      e.g.classList.toggle("fs-active-guide", g.text === this.active);
      T(e.halo, { points: geom.get(g.text).points });
      T(e.ref, { points: geom.get(g.text).points });
    }
    const act = this.guideEls.get(this.active);
    if (act) act.g.parentNode.appendChild(act.g);   // active hypothesis on top

    // hit strokes
    for (const [t, e] of this.hitEls)
      if (!liveTexts.has(t)) { e.poly.remove(); this.hitEls.delete(t); }
    for (const g of this.guides) {
      let e = this.hitEls.get(g.text);
      if (!e) {
        const poly = mk(this.gHits, "polyline", "fs-hit");
        attrs(poly, { tabindex: 0, role: "button" });
        // Every guide is draggable (the drag sweeps quantiles by
        // keeping the grabbed line under the pointer); a motionless
        // press stays a click, which selects. The squelch flag is what
        // separates the two.
        poly.addEventListener("pointerdown", ev => this.dragGuide(ev, g));
        poly.addEventListener("click", () => {
          if (this._squelch) { this._squelch = false; return; }
          this.setActive(g.text);
        });
        poly.addEventListener("keydown", ev => {
          if (this.quantKey(ev)) return;
          if (ev.key === "Enter" || ev.key === " ") this.setActive(g.text);
          else if (ev.key === "Delete" || ev.key === "Backspace")
            this.removeGuideFocus(g.text, "line");
          else return;
          ev.preventDefault();   // space must not also scroll the page
        });
        poly.addEventListener("mouseenter", () => hoverLink(g.text, true));
        poly.addEventListener("mouseleave", () => hoverLink(g.text, false));
        e = { poly, title: mk(poly, "title") };
        this.hitEls.set(g.text, e);
      }
      e.poly.classList.toggle("fs-active-hit", g.text === this.active);
      e.poly.setAttribute("points", geom.get(g.text).points);
      e.title.textContent = (g.text === this.active
        ? "drag to sweep quantiles · click to deselect "
        : "drag to sweep quantiles · click to compensate by ") + g.text;
    }

    // guide labels
    for (const [t, el] of this.glabelEls)
      if (!liveTexts.has(t)) { el.remove(); this.glabelEls.delete(t); }
    for (const g of this.guides) {
      let el = this.glabelEls.get(g.text);
      const tip = geom.get(g.text).tip;
      // slide the label along the trace (staying above the line) until the
      // above-line placement clears the ceiling; if that slide would run the
      // text into the right border, flip below the line, anchored leftward
      const lift = g.text === this.active ? -9 : -6;
      const vals = geom.get(g.text).vals;
      // Placement is continuous in the curve's geometry: the scan finds
      // the first sample inside the bounds, then interpolates back to
      // the exact boundary crossing. Snapping to whole samples would
      // make the label step between them while the quantile drag
      // shifts the curve — a judder wherever a bound is binding.
      const vLo = ylo + 0.1, vTop = yhi - 0.02;
      const vHi = Math.min(vTop, Y.invert(M.t + 14 - lift));
      const cross = (a, b, bound) => {
        const t = (bound - a[1]) / (b[1] - a[1]);
        return [a[0] + t * (b[0] - a[0]), bound];
      };
      const seek = (lo, hi) => {
        for (let i = 0; i < vals.length; i++) {
          const [, v] = vals[i];
          if (v < lo || v > hi) continue;
          const prev = vals[i - 1];
          if (!prev) return vals[i];
          const bound = prev[1] > hi ? hi : prev[1] < lo ? lo : null;
          return bound === null ? vals[i] : cross(prev, vals[i], bound);
        }
        return null;
      };
      let pos = seek(vLo, vHi);
      let flip = false;
      if (!pos) {
        flip = true;
        // below the line, as far right as the curve stays on the plot;
        // the same interpolation, scanning from the right
        for (let i = vals.length - 1; i >= 0; i--) {
          const [, v] = vals[i];
          if (v < vLo || v > vTop) continue;
          const nxt = vals[i + 1];
          if (!nxt) { pos = vals[i]; break; }
          const bound = nxt[1] > vTop ? vTop : nxt[1] < vLo ? vLo : null;
          pos = bound === null ? vals[i] : cross(vals[i], nxt, bound);
          break;
        }
      }
      if (!pos) pos = tip;
      const wpx = (g.text.length + (g.text === this.active ? 2 : 0)) * 9.5 + 8;
      let lx = this.X(pos[0]) + 6;
      if (!flip && lx + wpx > W - M.r - 2) flip = true;
      let ly;
      if (flip) { lx = this.X(pos[0]) - 6; ly = Y(pos[1]) + 16; }
      else ly = Y(pos[1]) + lift;
      if (!el) {
        el = mk(this.gLabels, "text", "fs-reflabel fs-guidelabel");
        el.addEventListener("click", () => this.setActive(g.text));
        el.addEventListener("mouseenter", () => hoverLink(g.text, true));
        el.addEventListener("mouseleave", () => hoverLink(g.text, false));
        attrs(el, { x: lx, y: ly });
        this.glabelEls.set(g.text, el);
      }
      el.textContent = g.text + (g.text === this.active ? " \u25c0" : "");
      el.classList.toggle("fs-active-label", g.text === this.active);
      el.setAttribute("text-anchor", flip ? "end" : "start");
      T(el, { x: lx, y: ly });
    }

    // the probe trace
    const ends = (() => {
      const v = tv.sm;
      const nCol = this.sizes.length;
      const sL = nCol > 1 ? (v[1] - shifts[1] - (v[0] - shifts[0])) / (this.lx[1] - this.lx[0]) : 0;
      const sR = nCol > 1 ? (v[nCol - 1] - shifts[nCol - 1] - (v[nCol - 2] - shifts[nCol - 2])) / (this.lx[nCol - 1] - this.lx[nCol - 2]) : 0;
      return {
        left: [this.lx0, v[0] - shifts[0] - sL * (this.lx[0] - this.lx0)],
        right: [this.lx1, v[nCol - 1] - shifts[nCol - 1] + sR * (this.lx1 - this.lx[nCol - 1])],
      };
    })();
    const pts = [[this.X(ends.left[0]), Y(ends.left[1])]];
    this.sizes.forEach((n, ci) => pts.push([this.X(this.lx[ci]), Y(tv.sm[ci] - shifts[ci])]));
    pts.push([this.X(ends.right[0]), Y(ends.right[1])]);
    const qd = monotonePath(pts);
    T(this.qHalo, { d: qd });
    T(this.qLine, { d: qd });
    // The hit stroke tracks without tweening: a pointer target that
    // lags its line drops drags on the way.
    this.qHit.setAttribute("d", qd);

    // The anchor column's tint marks a deliberate lock only: the
    // default focal column (the largest n) stays unhighlighted until
    // clicked.
    this.hoverEls.forEach((r, ci) => {
      const on = this.locked && ci === li;
      r.classList.toggle("fs-anchored", on);
      this.colHits[ci].setAttribute("aria-pressed", String(on));
    });

    // slider
    const slx = W - M.r + SLX;
    const yMin = Math.min(H - M.b, Y(this.qref.min[li] - shifts[li]));
    const yMax = Math.max(M.t, Y(this.qref.max[li] - shifts[li]));
    const yMed = Y(this.qref.med[li] - shifts[li]);
    const yCur = Math.max(M.t, Math.min(H - M.b, Y(tv.raw[li] - shifts[li])));
    T(this.slTrack, { x1: slx, x2: slx, y1: yMax, y2: yMin });
    T(this.slNotch, { x1: slx - 5, x2: slx + 5, y1: yMed, y2: yMed });
    T(this.slTickMin, { x1: slx - 4, x2: slx + 4, y1: yMin, y2: yMin });
    T(this.slTickMax, { x1: slx - 4, x2: slx + 4, y1: yMax, y2: yMax });
    attrs(this.slHandle, { "aria-valuenow": Math.round(this.q * 100),
      "aria-valuemin": 0, "aria-valuemax": 100 });
    T(this.slHandle, { cx: slx, cy: yCur });
    const qName = this.q <= 0 ? "minimum" : this.q >= 1 ? "maximum"
      : Math.abs(this.q - 0.5) < 1e-9 ? "median" : "p" + Math.round(this.q * 100);
    // The quantile's name only: its absolute height is a guest count.
    this.qLabel.textContent = qName;
    this.qLabel.appendChild(this.qLabelTip);
    T(this.qLabel, { y: yCur + 4 });
    T(this.slCaption, { y: yCur - 13 });
    this.slCaption.setAttribute("x", W - M.r + SLX + 11);
    this.slCaption.textContent = "Quantile";

    this.renderChips();
  }

  renderChips() {
    const liveTexts = new Set(this.guides.map(g => g.text));
    for (const [t, e] of this.chipEls)
      if (!liveTexts.has(t)) { e.btn.remove(); this.chipEls.delete(t); }
    for (const g of this.guides) {
      let e = this.chipEls.get(g.text);
      if (!e) {
        const btn = document.createElement("button");
        btn.className = "fs-chip";
        btn.type = "button";
        if (g.text === this._justAdded) btn.classList.add("fs-born");
        // Activation rides the whole button, padding included; the
        // remove × opts out by stopping propagation.
        btn.addEventListener("click", () => this.setActive(g.text));
        btn.addEventListener("keydown", ev => {
          if (ev.key !== "Delete" && ev.key !== "Backspace") return;
          this.removeGuideFocus(g.text, "chip");
          ev.preventDefault();
        });
        const label = document.createElement("span");
        label.className = "fs-chiplabel";
        const x = document.createElement("span");
        x.className = "fs-chipx";
        x.textContent = "\u00d7";
        x.title = "remove guide";
        x.addEventListener("click", ev => { ev.stopPropagation(); this.removeGuide(g.text); });
        btn.appendChild(label); btn.appendChild(x);
        this.chipBox.appendChild(btn);
        e = { btn, label, x };
        this.chipEls.set(g.text, e);
      }
      e.btn.classList.toggle("fs-on", g.text === this.active);
      e.btn.setAttribute("aria-pressed", String(g.text === this.active));
      e.label.textContent = g.text;
    }
    this._justAdded = null;
    // DOM order = growth order, entry last — but touch the DOM only
    // when the order actually changed: re-appending a node under the
    // pointer resets the browser's hover state, flickering the cursor
    // on every repaint (each drag frame repaints).
    const want = this.guides.map(g => this.chipEls.get(g.text).btn);
    want.push(this.entryWrap);
    if (Array.from(this.chipBox.children).some((el, i) => el !== want[i]))
      for (const el of want) this.chipBox.appendChild(el);
  }
}

function hydrateOne(el) {
  if (el.__fs) return;
  const island = el.querySelector('script[type="application/json"]');
  if (!island) return;
  el.__fs = new Widget(el, JSON.parse(island.textContent));
}

// ---------- complexity typesetting (docblock code spans) ----------
// Every complexity expression in the docs — island summaries and the
// prose contracts, aux-space clauses, and O(1) claims alike — renders
// through this one pass, so they cannot drift apart in style. A code
// span is transformed only when its whole text is complexity-shaped:
// O(...)/Θ(...)/Ω(...), a norm or magnitude like ‖r‖, or a lone
// letter. Within an expression: single letters set italic (math
// variables), multi-letter identifiers stay monospace (they are code
// names), function names stay upright roman, and ^ superscripts its
// following token. Deliberately not KaTeX: this grammar is the whole
// demand, and the page's own serif carries it at zero added weight.
// Standalone lone letters transform only in lowercase: a bare uppercase
// code span in prose is a type parameter (`W`, `R`), not a math
// variable, while uppercase inside a full expression is unambiguous.
const MATH_SHAPED = /^(?:[OΘΩ]\(.+\)|‖.+‖|\|.+\||[a-z])$/;
const MATH_FNS = new Set(["log", "ln", "lg", "log2", "log10", "sqrt", "min", "max"]);

function typesetInto(span, text) {
  let i = 0;
  const emitWord = (parent, w) => {
    if (MATH_FNS.has(w)) {
      parent.appendChild(document.createTextNode(w));
    } else if (w.length === 1) {
      mk(parent, "i").textContent = w;
    } else {
      mk(parent, "span", "fs-var").textContent = w;
    }
  };
  while (i < text.length) {
    const c = text[i];
    if (/[A-Za-z]/.test(c)) {
      let j = i + 1;
      while (j < text.length && /[A-Za-z0-9]/.test(text[j])) j++;
      emitWord(span, text.slice(i, j));
      i = j;
    } else if (c === "^") {
      let j = i + 1;
      while (j < text.length && /[A-Za-z0-9.]/.test(text[j])) j++;
      const sup = mk(span, "sup");
      const w = text.slice(i + 1, j);
      if (/^[A-Za-z]/.test(w)) emitWord(sup, w);
      else sup.textContent = w;
      i = j;
    } else {
      span.appendChild(document.createTextNode(c));
      i++;
    }
  }
}

function typesetDocMath(scope) {
  (scope || document).querySelectorAll(".docblock code").forEach(code => {
    if (code.closest("pre") || !MATH_SHAPED.test(code.textContent)) return;
    const span = document.createElement("span");
    span.className = "fs-math";
    typesetInto(span, code.textContent);
    code.replaceWith(span);
  });
}

// rustdoc's main.js preventDefaults any click on a `.toggle > summary`
// whose target is not the summary element itself or a link — and this
// summary is almost entirely <code> and <span> children, so most
// honest clicks would silently not toggle. Re-run the toggle by hand
// whenever that suppression hit: listening on the details in the
// bubble phase guarantees we run after rustdoc's summary-level
// handler regardless of registration order.
function armSummaryClicks(details) {
  if (details.__fsClicks) return;
  details.__fsClicks = true;
  details.addEventListener("click", e => {
    if (!e.defaultPrevented) return;
    const summary = e.target.closest("summary");
    if (!summary || summary.parentNode !== details) return;
    if (e.target.closest("a")) return;
    details.open = !details.open;
  });
}

const Fuelscape = {
  parse: parseBound,
  accepts: acceptBound,
  // Hydration is lazy, per island, and pre-warmed by scroll: a rustdoc
  // type page carries every method's island, and each widget is
  // thousands of SVG nodes \u2014 building them all at page load would tax
  // every visit. Instead each island hydrates when its expander scrolls
  // near the viewport (even while closed \u2014 an SVG builds fine inside a
  // closed details), so opening one is instant; the first `toggle`
  // remains the fallback wake (it fires for programmatic `open` changes
  // too, so rustdoc's expand-all control hydrates what it reveals).
  hydrate(scope) {
    typesetDocMath(scope);
    const io = typeof IntersectionObserver === "function"
      ? new IntersectionObserver(entries => {
          for (const e of entries) {
            if (!e.isIntersecting) continue;
            io.unobserve(e.target);
            hydrateOne(e.target.__fsIsland);
          }
        }, { rootMargin: "400px 0px" })
      : null;
    (scope || document).querySelectorAll(".fuelscape").forEach(el => {
      if (el.__fs || el.__fsArmed) return;
      const details = el.closest("details");
      if (!details) { hydrateOne(el); return; }
      el.__fsArmed = true;
      armSummaryClicks(details);
      if (details.open) { hydrateOne(el); return; }
      details.addEventListener(
        "toggle",
        () => { if (details.open) hydrateOne(el); },
        { once: true }
      );
      if (io) {
        details.__fsIsland = el;
        io.observe(details);
      }
    });
  },
};
globalThis.Fuelscape = Fuelscape;
if (typeof window !== "undefined" && typeof document !== "undefined") {
  if (document.readyState === "loading")
    document.addEventListener("DOMContentLoaded", () => Fuelscape.hydrate());
  else Fuelscape.hydrate();
}
})();
