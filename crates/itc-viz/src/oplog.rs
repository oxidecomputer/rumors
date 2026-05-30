//! The operation log and its pure algebra: the causal edges it induces, which nodes it
//! supersedes, the descendant cone of a set of nodes, the rewind-and-append rewrite,
//! and the URL-fragment codec. No `itc` or wasm here — just the combinatorics of the
//! log. Node indices follow creation order: index 0 is the implicit seed, then each op
//! appends its outputs in order.

use std::collections::{HashMap, HashSet};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};

/// A single primitive. Operands reference nodes by creation index.
#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Op {
    /// Advance node `x`'s own component by one event.
    Tick { x: usize },
    /// Split node `x`'s id in two (kept half, then forked child).
    Fork { x: usize },
    /// Merge disjoint clock `b` into clock `a`.
    Join { a: usize, b: usize },
    /// Merge `from`'s history into `to` (no tick, no id change); `from` is read-only.
    Send { from: usize, to: usize },
}

/// The three kinds of causal edge. A `message` edge runs from a sender to the receiver's
/// updated clock (a sent version), with no node between.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EdgeKind {
    Event,
    Forkjoin,
    Message,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub kind: EdgeKind,
}

/// Everything derivable from one forward pass over the log.
pub struct Analysis {
    pub edges: Vec<Edge>,
    /// `per_op[i]` is the node indices appended by `log[i]`.
    pub per_op: Vec<Vec<usize>>,
    /// Nodes consumed by a later op (tick/fork/join operands, the send receiver).
    pub superseded: HashSet<usize>,
}

pub fn analyze(log: &[Op]) -> Analysis {
    let mut edges = Vec::new();
    let mut per_op = Vec::with_capacity(log.len());
    let mut superseded = HashSet::new();
    let mut next = 1usize; // node 0 is the seed

    for op in log {
        match *op {
            Op::Tick { x } => {
                let o = next;
                next += 1;
                edges.push(Edge {
                    from: x,
                    to: o,
                    kind: EdgeKind::Event,
                });
                superseded.insert(x);
                per_op.push(vec![o]);
            }
            Op::Fork { x } => {
                let (a, b) = (next, next + 1);
                next += 2;
                edges.push(Edge {
                    from: x,
                    to: a,
                    kind: EdgeKind::Forkjoin,
                });
                edges.push(Edge {
                    from: x,
                    to: b,
                    kind: EdgeKind::Forkjoin,
                });
                superseded.insert(x);
                per_op.push(vec![a, b]);
            }
            Op::Join { a, b } => {
                let o = next;
                next += 1;
                edges.push(Edge {
                    from: a,
                    to: o,
                    kind: EdgeKind::Forkjoin,
                });
                edges.push(Edge {
                    from: b,
                    to: o,
                    kind: EdgeKind::Forkjoin,
                });
                superseded.insert(a);
                superseded.insert(b);
                per_op.push(vec![o]);
            }
            Op::Send { from, to } => {
                let o = next;
                next += 1;
                edges.push(Edge {
                    from: to,
                    to: o,
                    kind: EdgeKind::Event,
                }); // receiver's lineage
                edges.push(Edge {
                    from,
                    to: o,
                    kind: EdgeKind::Message,
                }); // the sent version
                superseded.insert(to); // only the receiver; the sender lives on
                per_op.push(vec![o]);
            }
        }
    }
    Analysis {
        edges,
        per_op,
        superseded,
    }
}

/// The nodes an op supersedes — those whose futures must be rewound when the op is
/// applied to historical nodes. (A send's `from` is read-only and is not superseded.)
pub fn anchors_of(op: Op) -> Vec<usize> {
    match op {
        Op::Tick { x } | Op::Fork { x } => vec![x],
        Op::Join { a, b } => vec![a, b],
        Op::Send { to, .. } => vec![to],
    }
}

/// Every node reachable from any anchor along any edge (excluding the anchors): the
/// futures that depend on them.
pub fn descendant_cone(edges: &[Edge], anchors: &[usize]) -> HashSet<usize> {
    let mut out: HashMap<usize, Vec<usize>> = HashMap::new();
    for e in edges {
        out.entry(e.from).or_default().push(e.to);
    }
    let mut cone = HashSet::new();
    let mut stack: Vec<usize> = anchors
        .iter()
        .filter_map(|a| out.get(a))
        .flatten()
        .copied()
        .collect();
    while let Some(n) = stack.pop() {
        if cone.insert(n) {
            if let Some(children) = out.get(&n) {
                stack.extend(children);
            }
        }
    }
    cone
}

fn remap_op(op: Op, remap: &HashMap<usize, usize>) -> Op {
    let r = |i: usize| *remap.get(&i).unwrap_or(&i);
    match op {
        Op::Tick { x } => Op::Tick { x: r(x) },
        Op::Fork { x } => Op::Fork { x: r(x) },
        Op::Join { a, b } => Op::Join { a: r(a), b: r(b) },
        Op::Send { from, to } => Op::Send {
            from: r(from),
            to: r(to),
        },
    }
}

/// Rewind history to the nodes `op` supersedes (drop the union of their descendant
/// cones) and append `op`. Returns a canonical minimal log producing exactly the
/// surviving DAG plus the new frontier. Every superseded operand must be rewound, or a
/// still-historical operand's future would survive alongside the new lineage and
/// duplicate its id-space. When all anchors are live tips the cones are empty and this
/// is a plain append.
pub fn rewind_and_apply(log: &[Op], op: Op) -> Vec<Op> {
    let Analysis { edges, per_op, .. } = analyze(log);
    let cone = descendant_cone(&edges, &anchors_of(op));

    let mut remap: HashMap<usize, usize> = HashMap::new();
    remap.insert(0, 0); // the seed always survives
    let mut kept: Vec<Op> = Vec::new();
    let mut next = 1usize;

    for (i, original) in log.iter().enumerate() {
        if per_op[i].iter().any(|n| cone.contains(n)) {
            continue; // dropped with its cone
        }
        kept.push(remap_op(*original, &remap));
        for &p in &per_op[i] {
            remap.insert(p, next);
            next += 1;
        }
    }
    kept.push(remap_op(op, &remap));
    kept
}

// ───────────────────────────── URL-fragment codec ─────────────────────────────

fn push_varint(out: &mut Vec<u8>, mut v: usize) {
    while v >= 0x80 {
        out.push((v as u8 & 0x7f) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn read_varint(bytes: &[u8], pos: &mut usize) -> Result<usize, String> {
    let mut result = 0usize;
    let mut shift = 0u32;
    loop {
        let b = *bytes.get(*pos).ok_or("truncated varint")?;
        *pos += 1;
        result |= ((b & 0x7f) as usize) << shift;
        if b & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    Ok(result)
}

/// Encode a log to a URL-safe fragment (1-byte tag + LEB128 varint operands, base64url).
/// An empty log is the empty string.
pub fn encode(log: &[Op]) -> String {
    let mut bytes = Vec::new();
    for op in log {
        match *op {
            Op::Tick { x } => {
                bytes.push(1);
                push_varint(&mut bytes, x);
            }
            Op::Fork { x } => {
                bytes.push(2);
                push_varint(&mut bytes, x);
            }
            Op::Join { a, b } => {
                bytes.push(3);
                push_varint(&mut bytes, a);
                push_varint(&mut bytes, b);
            }
            Op::Send { from, to } => {
                bytes.push(4);
                push_varint(&mut bytes, from);
                push_varint(&mut bytes, to);
            }
        }
    }
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Decode a fragment back into a log. Errs on malformed input.
pub fn decode(fragment: &str) -> Result<Vec<Op>, String> {
    if fragment.is_empty() {
        return Ok(Vec::new());
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(fragment)
        .map_err(|e| e.to_string())?;
    let mut log = Vec::new();
    let mut pos = 0;
    while pos < bytes.len() {
        let tag = bytes[pos];
        pos += 1;
        match tag {
            1 => log.push(Op::Tick {
                x: read_varint(&bytes, &mut pos)?,
            }),
            2 => log.push(Op::Fork {
                x: read_varint(&bytes, &mut pos)?,
            }),
            3 => {
                let a = read_varint(&bytes, &mut pos)?;
                let b = read_varint(&bytes, &mut pos)?;
                log.push(Op::Join { a, b });
            }
            4 => {
                let from = read_varint(&bytes, &mut pos)?;
                let to = read_varint(&bytes, &mut pos)?;
                log.push(Op::Send { from, to });
            }
            _ => return Err(format!("unknown op tag {tag}")),
        }
    }
    Ok(log)
}
