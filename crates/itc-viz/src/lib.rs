//! WASM replay engine for the interactive Interval Tree Clocks visualizer.
//!
//! The browser holds the authoritative *operation log*; this crate is a pure,
//! deterministic replay engine that turns a log into materialized clock values.
//! It owns no graph, layout, or undo logic — those are derived from the log on
//! the TypeScript side.
//!
//! Immutability of the causal-history DAG is achieved through the `itc` codec:
//! every node is stored as its canonical [`itc::Clock::encode`] (or
//! [`itc::Version::encode`]) bytes. Applying an operation decodes a *fresh*
//! value from a source node, mutates it, and re-encodes the result(s) into new
//! nodes; source bytes are never touched. Because `encode`/`decode` round-trips
//! canonically and `fork`/`join` are deterministic, replaying the same log
//! always reconstructs byte-identical nodes.

use itc::{Clock, Version};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[cfg(test)]
mod tests;

/// A single primitive applied during replay. Operands reference nodes by their
/// 0-based creation index; index `0` is always the implicit seed clock.
///
/// There is no `Seed` variant: every log begins with an implicit seed, mirroring
/// the TypeScript `OpLog` model.
#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Op {
    /// Advance node `x`'s own component by one event.
    Tick { x: usize },
    /// Split node `x`'s id in two; emits the kept half then the forked child.
    Fork { x: usize },
    /// Merge disjoint clock `b` into clock `a`; emits the union (errors on overlap).
    Join { a: usize, b: usize },
    /// Snapshot clock `x`'s history as a message; does not advance `x`.
    Peek { x: usize },
    /// Merge message `m` into clock `t`; does not tick and does not change ids.
    Merge { t: usize, m: usize },
}

/// Whether a stored node is a clock (id + history) or a peeked message (history only).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Clock,
    Message,
}

/// A materialized node returned to the front-end: paper-notation strings plus
/// its kind. Clock nodes carry `party`, `event`, and the combined `stamp`;
/// message nodes carry only `event` (with `party` serialized as `null`).
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub idx: usize,
    pub kind: NodeKind,
    pub party: Option<String>,
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stamp: Option<String>,
}

/// A node held in the arena: canonical bytes plus its precomputed notation.
#[derive(Clone, Debug)]
struct Node {
    bytes: Vec<u8>,
    kind: NodeKind,
    party: Option<String>,
    event: String,
    stamp: Option<String>,
}

/// Why a replay failed. All variants are recoverable on the JS side (a rejected
/// op simply is not committed to the log).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    /// An operand referenced a node index that does not exist.
    IndexOutOfRange(usize),
    /// An op expected a clock at this index but found a message.
    NotAClock(usize),
    /// An op expected a message at this index but found a clock.
    NotAMessage(usize),
    /// A `join` was attempted on clocks whose ids overlap.
    JoinOverlap { a: usize, b: usize },
    /// A stored node failed to decode (should be impossible for engine-produced bytes).
    Decode(String),
}

impl core::fmt::Display for EngineError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EngineError::IndexOutOfRange(i) => write!(f, "node index {i} out of range"),
            EngineError::NotAClock(i) => write!(f, "node {i} is a message, expected a clock"),
            EngineError::NotAMessage(i) => write!(f, "node {i} is a clock, expected a message"),
            EngineError::JoinOverlap { a, b } => {
                write!(f, "cannot join nodes {a} and {b}: their ids overlap")
            }
            EngineError::Decode(e) => write!(f, "failed to decode stored node: {e}"),
        }
    }
}

impl std::error::Error for EngineError {}

/// The host-testable replay core. Holds the arena materialized by the last
/// successful [`Engine::replay`]; carries no wasm types so it can be unit-tested
/// on the native target.
#[derive(Default)]
pub struct Engine {
    arena: Vec<Node>,
}

impl Engine {
    /// A fresh engine with an empty arena (no seed until the first replay).
    pub fn new() -> Self {
        Engine { arena: Vec::new() }
    }

    /// Replay an operation log from scratch: seed node `0`, then apply each op,
    /// committing the rebuilt arena only on success. On error the previous arena
    /// is left intact.
    pub fn replay(&mut self, ops: &[Op]) -> Result<(), EngineError> {
        let mut arena: Vec<Node> = Vec::with_capacity(ops.len() + 1);
        push_clock(&mut arena, Clock::seed());
        for op in ops {
            apply(&mut arena, *op)?;
        }
        self.arena = arena;
        Ok(())
    }

    /// Descriptors for every node, in creation order.
    pub fn descriptors(&self) -> Vec<Descriptor> {
        self.arena
            .iter()
            .enumerate()
            .map(|(idx, n)| Descriptor {
                idx,
                kind: n.kind,
                party: n.party.clone(),
                event: n.event.clone(),
                stamp: n.stamp.clone(),
            })
            .collect()
    }

    /// Whether the two nodes are clocks with disjoint ids (a join would succeed).
    /// `false` if either index is out of range or names a message.
    pub fn is_disjoint(&self, a: usize, b: usize) -> bool {
        match (clock_at(&self.arena, a), clock_at(&self.arena, b)) {
            (Ok(ca), Ok(cb)) => ca.party().is_disjoint(cb.party()),
            _ => false,
        }
    }
}

/// Decode the clock stored at `i`, or report why it is unavailable.
fn clock_at(arena: &[Node], i: usize) -> Result<Clock, EngineError> {
    let node = arena.get(i).ok_or(EngineError::IndexOutOfRange(i))?;
    match node.kind {
        NodeKind::Clock => {
            Clock::decode(&node.bytes).map_err(|e| EngineError::Decode(e.to_string()))
        }
        NodeKind::Message => Err(EngineError::NotAClock(i)),
    }
}

/// Decode the message (version) stored at `i`, or report why it is unavailable.
fn version_at(arena: &[Node], i: usize) -> Result<Version, EngineError> {
    let node = arena.get(i).ok_or(EngineError::IndexOutOfRange(i))?;
    match node.kind {
        NodeKind::Message => {
            Version::decode(&node.bytes).map_err(|e| EngineError::Decode(e.to_string()))
        }
        NodeKind::Clock => Err(EngineError::NotAMessage(i)),
    }
}

/// Push a clock node, precomputing its paper-notation strings.
fn push_clock(arena: &mut Vec<Node>, clock: Clock) {
    arena.push(Node {
        party: Some(clock.party().to_string()),
        event: clock.version().to_string(),
        stamp: Some(clock.to_string()),
        bytes: clock.encode(),
        kind: NodeKind::Clock,
    });
}

/// Push a message node (a peeked [`Version`]), precomputing its event notation.
fn push_message(arena: &mut Vec<Node>, version: Version) {
    arena.push(Node {
        party: None,
        event: version.to_string(),
        stamp: None,
        bytes: version.encode(),
        kind: NodeKind::Message,
    });
}

/// Apply one op, appending the node(s) it produces.
fn apply(arena: &mut Vec<Node>, op: Op) -> Result<(), EngineError> {
    match op {
        Op::Tick { x } => {
            let mut clock = clock_at(arena, x)?;
            clock.tick();
            push_clock(arena, clock);
        }
        Op::Fork { x } => {
            let mut clock = clock_at(arena, x)?;
            let child = clock.fork();
            push_clock(arena, clock);
            push_clock(arena, child);
        }
        Op::Join { a, b } => {
            let mut left = clock_at(arena, a)?;
            let right = clock_at(arena, b)?;
            match left.join(right) {
                Ok(()) => push_clock(arena, left),
                Err(_) => return Err(EngineError::JoinOverlap { a, b }),
            }
        }
        Op::Peek { x } => {
            let clock = clock_at(arena, x)?;
            push_message(arena, clock.version());
        }
        Op::Merge { t, m } => {
            let mut clock = clock_at(arena, t)?;
            let version = version_at(arena, m)?;
            clock |= version;
            push_clock(arena, clock);
        }
    }
    Ok(())
}

/// The `#[wasm_bindgen]` surface, exported to JavaScript as `Engine`. A thin
/// wrapper that crosses the boundary with JSON strings and `JsValue` errors.
#[wasm_bindgen(js_name = Engine)]
pub struct WasmEngine {
    inner: Engine,
}

#[wasm_bindgen(js_class = Engine)]
impl WasmEngine {
    /// Construct an engine and install a panic hook that surfaces Rust panics in
    /// the browser console (no-op when the feature is disabled).
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmEngine {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();
        WasmEngine {
            inner: Engine::new(),
        }
    }

    /// Replay an op-log given as a JSON array of [`Op`] and return the resulting
    /// node descriptors as a JSON array. Rejected ops (e.g. an overlapping join)
    /// produce an `Err` and leave the prior state intact.
    pub fn replay(&mut self, ops_json: &str) -> Result<String, JsValue> {
        let ops: Vec<Op> = serde_json::from_str(ops_json).map_err(to_js)?;
        self.inner.replay(&ops).map_err(to_js)?;
        serde_json::to_string(&self.inner.descriptors()).map_err(to_js)
    }

    /// Whether nodes `a` and `b` are clocks with disjoint ids (join validity).
    pub fn is_disjoint(&self, a: usize, b: usize) -> bool {
        self.inner.is_disjoint(a, b)
    }
}

impl Default for WasmEngine {
    fn default() -> Self {
        WasmEngine::new()
    }
}

/// Render any `Display` error as a `JsValue` for the wasm boundary.
fn to_js<E: core::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&e.to_string())
}
