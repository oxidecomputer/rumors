//! WASM engine for the interactive Interval Tree Clocks visualizer.
//!
//! The engine owns the authoritative **operation log** and the immutable causal-history
//! DAG it induces. Gestures call [`Engine::apply`] (or the wasm `tick`/`fork`/`join`/
//! `send` methods), which rewinds the futures the op supersedes ([`oplog::rewind_and_apply`])
//! and appends the op; the engine then re-materializes clock values and reports the
//! derived state (node descriptors, causal edges, the live set) to the front-end, which
//! is purely presentational. The log also round-trips to a URL fragment, so any figure
//! is a shareable link.
//!
//! Immutability is achieved through the `itc` codec: every node is stored as its
//! canonical [`itc::Clock::encode`] bytes. Applying an op decodes a fresh value from a
//! source node, mutates it, and re-encodes the result; source bytes are never touched.
//! Because `encode`/`decode` round-trips canonically and `fork`/`join` are
//! deterministic, replaying the same log always reconstructs byte-identical nodes.

use itc::{Clock, Version};
use serde::Serialize;
use wasm_bindgen::prelude::*;

mod oplog;
#[cfg(test)]
mod tests;

use oplog::{analyze, decode, descendant_cone, encode, rewind_and_apply};
pub use oplog::{Edge, EdgeKind, Op};

/// A materialized node returned to the front-end: the clock's paper-notation strings.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub idx: usize,
    pub party: String,
    pub event: String,
    pub stamp: String,
}

/// The full derived state handed to the front-end after each change.
#[derive(Serialize)]
pub struct State {
    pub nodes: Vec<Descriptor>,
    pub edges: Vec<Edge>,
    pub live: Vec<usize>,
}

/// Why an operation failed. All variants are recoverable on the JS side (a rejected op
/// is not committed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    IndexOutOfRange(usize),
    JoinOverlap { a: usize, b: usize },
    Decode(String),
    BadFragment(String),
}

impl core::fmt::Display for EngineError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EngineError::IndexOutOfRange(i) => write!(f, "node index {i} out of range"),
            EngineError::JoinOverlap { a, b } => {
                write!(f, "cannot join nodes {a} and {b}: their ids overlap")
            }
            EngineError::Decode(e) => write!(f, "failed to decode stored node: {e}"),
            EngineError::BadFragment(e) => write!(f, "bad fragment: {e}"),
        }
    }
}

impl std::error::Error for EngineError {}

/// A node in the arena: canonical bytes plus its precomputed notation.
#[derive(Clone, Debug)]
struct Node {
    bytes: Vec<u8>,
    party: String,
    event: String,
    stamp: String,
}

/// The host-testable engine: owns the op-log and the arena it replays to. Carries no
/// wasm types, so it can be unit-tested (and property-tested) on the native target.
pub struct Engine {
    log: Vec<Op>,
    arena: Vec<Node>,
}

impl Default for Engine {
    fn default() -> Self {
        Engine::new()
    }
}

impl Engine {
    /// A fresh engine: the empty log, materialized to the seed clock.
    pub fn new() -> Self {
        Engine {
            log: Vec::new(),
            arena: build(&[]).expect("the seed always builds"),
        }
    }

    /// Apply an op referencing current node indices: rewind the futures it supersedes,
    /// append it, and re-materialize. On error the prior state is left intact.
    pub fn apply(&mut self, op: Op) -> Result<(), EngineError> {
        let log = rewind_and_apply(&self.log, op);
        let arena = build(&log)?;
        self.log = log;
        self.arena = arena;
        Ok(())
    }

    /// Replace the log wholesale (e.g. from a URL fragment) and re-materialize.
    pub fn load(&mut self, log: Vec<Op>) -> Result<(), EngineError> {
        let arena = build(&log)?;
        self.log = log;
        self.arena = arena;
        Ok(())
    }

    /// Replace the log from a URL fragment.
    pub fn load_fragment(&mut self, fragment: &str) -> Result<(), EngineError> {
        let log = decode(fragment).map_err(EngineError::BadFragment)?;
        self.load(log)
    }

    /// The current log as a URL fragment.
    pub fn fragment(&self) -> String {
        encode(&self.log)
    }

    /// The current op-log.
    pub fn op_log(&self) -> &[Op] {
        &self.log
    }

    /// Total node count (arena length).
    pub fn node_count(&self) -> usize {
        self.arena.len()
    }

    /// Live node indices: clocks not yet superseded by a successor.
    pub fn live_indices(&self) -> Vec<usize> {
        let superseded = analyze(&self.log).superseded;
        (0..self.arena.len())
            .filter(|i| !superseded.contains(i))
            .collect()
    }

    /// Descriptors for every node, in creation order.
    pub fn descriptors(&self) -> Vec<Descriptor> {
        self.arena
            .iter()
            .enumerate()
            .map(|(idx, n)| Descriptor {
                idx,
                party: n.party.clone(),
                event: n.event.clone(),
                stamp: n.stamp.clone(),
            })
            .collect()
    }

    /// The full derived state: descriptors, causal edges, and the live set.
    pub fn state(&self) -> State {
        let a = analyze(&self.log);
        let live = (0..self.arena.len())
            .filter(|i| !a.superseded.contains(i))
            .collect();
        State {
            nodes: self.descriptors(),
            edges: a.edges,
            live,
        }
    }

    /// Whether nodes `a` and `b` have disjoint ids (join validity).
    pub fn is_disjoint(&self, a: usize, b: usize) -> bool {
        match (clock_at(&self.arena, a), clock_at(&self.arena, b)) {
            (Ok(ca), Ok(cb)) => ca.party().is_disjoint(cb.party()),
            _ => false,
        }
    }

    /// The descendant cone of `anchors` in the current log (test/diagnostic helper).
    pub fn descendant_cone(&self, anchors: &[usize]) -> std::collections::HashSet<usize> {
        descendant_cone(&analyze(&self.log).edges, anchors)
    }
}

/// Replay a log into a fresh arena (seed at index 0, then each op's outputs).
fn build(log: &[Op]) -> Result<Vec<Node>, EngineError> {
    let mut arena = Vec::with_capacity(log.len() + 1);
    push_clock(&mut arena, Clock::seed());
    for op in log {
        apply_to_arena(&mut arena, *op)?;
    }
    Ok(arena)
}

fn clock_at(arena: &[Node], i: usize) -> Result<Clock, EngineError> {
    let node = arena.get(i).ok_or(EngineError::IndexOutOfRange(i))?;
    Clock::decode(&node.bytes).map_err(|e| EngineError::Decode(e.to_string()))
}

fn push_clock(arena: &mut Vec<Node>, clock: Clock) {
    arena.push(Node {
        party: clock.party().to_string(),
        event: clock.version().to_string(),
        stamp: clock.to_string(),
        bytes: clock.encode(),
    });
}

fn apply_to_arena(arena: &mut Vec<Node>, op: Op) -> Result<(), EngineError> {
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
        Op::Send { from, to } => {
            let version: Version = clock_at(arena, from)?.version();
            let mut receiver = clock_at(arena, to)?;
            receiver |= version;
            push_clock(arena, receiver);
        }
    }
    Ok(())
}

/// The `#[wasm_bindgen]` surface, exported to JavaScript as `Engine`. Gesture methods
/// return the new [`State`] as JSON; the front-end derives layout and renders from it.
#[wasm_bindgen(js_name = Engine)]
pub struct WasmEngine {
    inner: Engine,
}

#[wasm_bindgen(js_class = Engine)]
impl WasmEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmEngine {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();
        WasmEngine {
            inner: Engine::new(),
        }
    }

    pub fn tick(&mut self, x: usize) -> Result<String, JsValue> {
        self.commit(Op::Tick { x })
    }

    pub fn fork(&mut self, x: usize) -> Result<String, JsValue> {
        self.commit(Op::Fork { x })
    }

    pub fn join(&mut self, a: usize, b: usize) -> Result<String, JsValue> {
        self.commit(Op::Join { a, b })
    }

    pub fn send(&mut self, from: usize, to: usize) -> Result<String, JsValue> {
        self.commit(Op::Send { from, to })
    }

    /// Load state from a URL fragment, returning the new state JSON.
    pub fn load(&mut self, fragment: &str) -> Result<String, JsValue> {
        self.inner.load_fragment(fragment).map_err(to_js)?;
        self.state_json()
    }

    /// The current op-log as a URL fragment.
    pub fn fragment(&self) -> String {
        self.inner.fragment()
    }

    /// Whether nodes `a` and `b` have disjoint ids (join validity).
    pub fn is_disjoint(&self, a: usize, b: usize) -> bool {
        self.inner.is_disjoint(a, b)
    }
}

impl WasmEngine {
    fn commit(&mut self, op: Op) -> Result<String, JsValue> {
        self.inner.apply(op).map_err(to_js)?;
        self.state_json()
    }

    fn state_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner.state()).map_err(to_js)
    }
}

impl Default for WasmEngine {
    fn default() -> Self {
        WasmEngine::new()
    }
}

fn to_js<E: core::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&e.to_string())
}
