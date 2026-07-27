//! Skyline-first text kernels: the paper notation rendered from and parsed
//! into skyline streams directly.
//!
//! The paper's event grammar (`n | (n, e1, e2)`, separator `", "`) spells
//! the tree's min-lifted node bases, while a skyline stream stores
//! topology plus delta-coded absolute leaf heights — so each direction is a
//! change of coordinates, done here without materializing either a base
//! tree or any absolute height:
//!
//! - [`render`] derives every printed base in *relative* coordinates. For a
//!   completed subtree it keeps three delta-sized summaries — the drop from
//!   its entry leaf down to its floor, the signed span from its entry leaf
//!   to its last, and the signed delta that carried its entry leaf — and
//!   one merge step per internal node finalizes both children's printed
//!   bases (`b(child) = floor(child) − floor(node)`, parent-close
//!   information) from those summaries alone. Only the root's base needs
//!   the one absolute height the stream stores. Every summary is a sum of
//!   the subtree's own leaf deltas, so the transient is priced by the
//!   deltas' coded widths, never by the heights' (on a wide-rooted spine
//!   the heights are all wide while every summary stays word-sized).
//! - [`parse`] runs the running path sum as a *per-leaf delta*: each parsed
//!   base joins the accumulator on descent and leaves it on ascent (each
//!   base charged at most twice), and at each leaf the accumulator holds
//!   exactly the leaf-to-leaf delta the skyline payload codes — extracted,
//!   zigzag-coded, and fed to the collapsing output builder. The
//!   accumulator is the cliff-immune [`Accum`]: a plain big-integer running
//!   value re-imports the boundary comb's quadratic carry genre. The ≤2×
//!   charge is enforced structurally (one join, one leave per base) plus,
//!   in `tests/meter.rs`, the `SKYLINE_PARSE_*` aggregate ceilings and
//!   floors on the heap/segment/limb/scan columns and the parse touch pin
//!   (per-text-byte accumulator touches flat across a comb doubling, over
//!   a one-touch-per-delta liveness floor) — never by a per-base
//!   assertion. The render direction's zero-touch conservation pin lives
//!   beside it, so accumulator work cannot migrate between the two text
//!   directions without moving a committed number.
//!
//! Both walks are iterative — explicit frame vectors and a phase bit
//! stack, never the call stack — so depth grows no stack segments at any
//! input size.
//!
//! The kernels are module-private to the skyline codec (test- and
//! meter-visible) and *are* the production text path: `Display` routes to
//! [`render`] and `FromStr` to [`parse`], so the public-entry asserts in
//! the test suite pin entry agreement, not an independent value. The
//! independent legs are the construction-language transcoder — [`parse`]
//! of rendered text must land on the transcoder's stream byte for byte
//! over the generator families — the render↔parse inverse pair, and a
//! deterministic accept/reject corpus with a byte-mutation sweep pinning
//! the grammar's decisions. The resource-envelope suite
//! (`tests/meter.rs`) pins both kernels' transients on the adversarial
//! families.

use core::cmp::Ordering;
use core::fmt::Write as _;

use crate::codec::accum::Accum;
use crate::codec::text::{parse_base, Cur};
use crate::codec::{Base, BitCursor, Bits, BitsSlice, DsiCursor};
use crate::error::Parse;
use crate::step;

use super::build::SkylineBuilder;
use super::emit::signed_sum;
use super::{gamma_code, unzigzag, validate_bits, zigzag_signed};

#[cfg(test)]
mod tests;

/// The separator the paper notation prints between a node's parts.
const SEP: &str = ", ";

/// Rendered bytes an internal node adds beyond its digits: `(`, `)`, and
/// one [`SEP`] before each child.
const INTERNAL_SYNTAX_BYTES: usize = 2 + 2 * SEP.len();

/// While emitting an open node's text, which child the walk is inside.
///
/// One phase bit per open node on the emit pass's pending stack; a full
/// binary tree needs no presence bits beside it.
const LEFT_PHASE: bool = true;

/// What a completed subtree contributes to its parent's merge, all in
/// coordinates relative to the subtree's own entry (first) leaf.
///
/// `drop` is entry minus the subtree's floor (non-negative: the floor is a
/// minimum over leaves including the entry); `span` is the signed offset of
/// the subtree's last leaf from its entry; `incoming` is the signed stream
/// delta that carried the entry leaf itself. Each is a sum of leaf deltas,
/// so its width is priced by the deltas' own codes.
struct Summary {
    /// Preorder index of the subtree's root node, where the merge writes
    /// the finalized printed base.
    root: usize,
    /// Entry leaf height minus the subtree's floor (its minimum leaf
    /// height): non-negative.
    drop: Base,
    /// The last leaf's height minus the entry leaf's, as (negative,
    /// magnitude).
    span: (bool, Base),
    /// The stream delta that carried the entry leaf, as (negative,
    /// magnitude); the whole stream's first leaf stores the (unused)
    /// positive zero.
    incoming: (bool, Base),
}

/// While finalizing an open internal node, what the walk still owes it.
enum Frame {
    /// The node at `index` awaits its left child.
    NeedLeft { index: usize },
    /// The node at `index` awaits its right child; the left child's
    /// summary is held for the merge.
    NeedRight { index: usize, left: Summary },
}

/// Render a skyline stream as the paper notation of the version the
/// stream codes; the public `Display` entry routes here.
///
/// Two passes. The finalize pass walks the stream once, merging the
/// module doc's relative subtree summaries bottom-up to derive every
/// node's printed base — rendered into one preorder-indexed digit arena —
/// and sizes the output exactly from the arena and the topology. The emit
/// pass writes the sized output straight through: one phase bit per open
/// node, no recursion, and the exactness of the sizing is asserted.
///
/// # Panics
///
/// Panics if the stream is not a canonical skyline encoding.
pub fn render(bits: &BitsSlice) -> String {
    let mut cursor = DsiCursor::new(bits);

    // Finalize: per-node internal flags (semantic, not the wire bits:
    // `true` = internal), and every node's printed base derived in
    // relative coordinates (the module doc's merge).
    let mut topology = Bits::new();
    let mut bases: Vec<Base> = Vec::new();
    let mut frames: Vec<Frame> = Vec::new();
    let mut first_height: Option<Base> = None;
    let root_summary = 'tree: loop {
        step!();
        // One whole descent per unary read: `k` internal nodes, then
        // the leaf whose flag terminates the run.
        let k = cursor.read_unary().expect("canonical skyline bits");
        for _ in 0..k {
            let index = topology.len();
            topology.push(true);
            bases.push(Base::ZERO);
            frames.push(Frame::NeedLeft { index });
        }
        let index = topology.len();
        topology.push(false);
        bases.push(Base::ZERO);
        // The cursor's own `read_int`: word-parallel payload decode.
        let code = cursor.read_int().expect("canonical skyline bits");
        let incoming = if first_height.is_some() {
            unzigzag(code)
        } else {
            first_height = Some(code);
            (false, Base::ZERO)
        };
        let mut summary = Summary {
            root: index,
            drop: Base::ZERO,
            span: (false, Base::ZERO),
            incoming,
        };
        // Close every subtree this leaf completes.
        loop {
            step!();
            match frames.pop() {
                None => break 'tree summary,
                Some(Frame::NeedLeft { index }) => {
                    frames.push(Frame::NeedRight {
                        index,
                        left: summary,
                    });
                    break;
                }
                Some(Frame::NeedRight { index, left }) => {
                    summary = merge(index, left, summary, &mut bases);
                }
            }
        }
    };
    assert_eq!(
        cursor.position(),
        bits.len(),
        "a canonical skyline stream is exactly one tree"
    );
    // The root's printed base is the one absolute quantity: the stored
    // first height minus the root's drop (non-negative: the global floor
    // is a leaf height, and heights are naturals).
    bases[root_summary.root] =
        first_height.expect("a skyline stream has at least one leaf") - &root_summary.drop;

    // Size exactly: every digit lands in one preorder arena, and each
    // internal node adds its fixed syntax bytes.
    let mut arena = String::new();
    let mut starts: Vec<usize> = Vec::with_capacity(bases.len() + 1);
    for base in &bases {
        starts.push(arena.len());
        write!(arena, "{base}").expect("String formatting is infallible");
    }
    starts.push(arena.len());
    let exact = arena.len() + topology.count_ones() * INTERNAL_SYNTAX_BYTES;

    // Emit: preorder over the finalized topology and arena, one phase bit
    // per open node.
    let mut out = String::with_capacity(exact);
    let mut pending = Bits::new();
    for (node, internal) in topology.iter().by_vals().enumerate() {
        step!();
        let digits = &arena[starts[node]..starts[node + 1]];
        if internal {
            out.push('(');
            out.push_str(digits);
            out.push_str(SEP);
            pending.push(LEFT_PHASE);
            continue;
        }
        out.push_str(digits);
        // Finish every node the completed subtree closes.
        loop {
            match pending.pop() {
                None => break,
                Some(LEFT_PHASE) => {
                    out.push_str(SEP);
                    pending.push(!LEFT_PHASE);
                    break;
                }
                Some(_) => out.push(')'),
            }
        }
    }
    assert_eq!(
        out.len(),
        exact,
        "the finalize pass sizes the rendered text exactly"
    );
    out
}

/// Merge a closed internal node's two child summaries, finalizing both
/// children's printed bases into `bases`.
///
/// With `t` the right child's floor relative to the left's entry leaf
/// (`span(left) + incoming(right) − drop(right)`), the node's floor is
/// `min(−drop(left), t)`, still relative to the left entry — so
/// `u = t + drop(left)` decides the min-lift: a non-negative `u` says the
/// left child sits on the node's floor (its base is zero and `u` is the
/// right child's), a negative `u` says the right child does (its magnitude
/// is the left child's base and the node's drop deepens by it).
fn merge(parent: usize, left: Summary, right: Summary, bases: &mut [Base]) -> Summary {
    let entry_step = signed_sum(
        left.span.0,
        left.span.1,
        right.incoming.0,
        &right.incoming.1,
    );
    let span = signed_sum(
        entry_step.0,
        entry_step.1.clone(),
        right.span.0,
        &right.span.1,
    );
    let t = signed_sum(entry_step.0, entry_step.1, true, &right.drop);
    let (u_negative, u) = signed_sum(t.0, t.1, false, &left.drop);
    let drop = if u_negative {
        bases[left.root] = u.clone();
        left.drop + &u
    } else {
        bases[right.root] = u;
        left.drop
    };
    Summary {
        root: parent,
        drop,
        span,
        incoming: left.incoming,
    }
}

/// Parse the paper notation into the canonical skyline stream of the
/// version it spells; the public `FromStr` entry routes here.
///
/// One iterative pass: bases parse through the delegated digit-run reader,
/// the per-leaf delta accumulator turns path-sum movement into skyline
/// payloads (the module doc's discipline), and the collapsing output
/// builder assembles the stream. Canonicality (a zero-base child under
/// every node, no equal sibling leaves) is checked at each close and
/// reported after the whole syntax pass, so syntax errors — including
/// trailing junk — outrank [`Parse::NotCanonical`]; the built stream is
/// then gated through the strict validator.
pub fn parse(s: &str) -> Result<Bits, Parse> {
    /// What a parsed subtree contributes to its parent's normal-form
    /// check: its written base and whether it is a single leaf.
    struct Child {
        base: Base,
        is_leaf: bool,
    }
    /// While parsing an open node's text, what the stream still owes it.
    enum EvFrame {
        /// Consumed `(`, the base, and the first separator; the left
        /// child's text is next.
        NeedLeft { base: Base },
        /// Consumed the left child and the second separator; the right
        /// child's text is next.
        NeedRight { base: Base, left: Child },
    }

    let mut cur = Cur::new(s);
    let mut builder = SkylineBuilder::with_capacity(s.len());
    let mut frames: Vec<EvFrame> = Vec::new();
    // The signed height movement since the last emitted leaf.
    let mut delta = Accum::new();
    let mut emitted_first = false;
    let mut canonical = true;

    'nodes: loop {
        step!();
        // Parse one node at the cursor.
        match cur.peek() {
            Some(b'(') => {
                cur.bump();
                let base = parse_base(&mut cur)?;
                if cur.bump() != Some(b',') {
                    return Err(Parse::Syntax);
                }
                delta.add_base(&base);
                frames.push(EvFrame::NeedLeft { base });
                continue 'nodes;
            }
            Some(c) if c.is_ascii_digit() => {}
            _ => return Err(Parse::Syntax),
        }
        let base = parse_base(&mut cur)?;
        delta.add_base(&base);

        // Emit the leaf's plateau: the accumulated movement is exactly the
        // leaf-to-leaf delta (absolute for the first leaf), and extracting
        // it re-zeroes the accumulator for the next one.
        let (sign, magnitude) = delta.sign_magnitude();
        let code = if emitted_first {
            gamma_code(&zigzag_signed(
                sign == Ordering::Less,
                Base::from(magnitude.clone()),
            ))
        } else {
            emitted_first = true;
            debug_assert_ne!(sign, Ordering::Less, "a path sum of naturals is a natural");
            gamma_code(&Base::from(magnitude.clone()))
        };
        match sign {
            Ordering::Greater => delta.sub_wide(&magnitude),
            Ordering::Less => delta.add_wide(&magnitude),
            Ordering::Equal => {}
        }
        builder.leaf(frames.len(), code);
        delta.sub_base(&base); // the leaf's base exits the path

        // Close every node the leaf completes.
        let mut summary = Child {
            base,
            is_leaf: true,
        };
        loop {
            step!();
            match frames.pop() {
                None => break 'nodes,
                Some(EvFrame::NeedLeft { base }) => {
                    if cur.bump() != Some(b',') {
                        return Err(Parse::Syntax);
                    }
                    frames.push(EvFrame::NeedRight {
                        base,
                        left: summary,
                    });
                    continue 'nodes;
                }
                Some(EvFrame::NeedRight { base, left }) => {
                    if cur.bump() != Some(b')') {
                        return Err(Parse::Syntax);
                    }
                    // Normal form: a zero-base child under every node, and
                    // no equal sibling leaves (which the builder collapses
                    // rather than stores, so the flag is the only witness).
                    if left.base != Base::ZERO && summary.base != Base::ZERO {
                        canonical = false;
                    }
                    if left.is_leaf && summary.is_leaf && left.base == summary.base {
                        canonical = false;
                    }
                    delta.sub_base(&base); // the closed node's base exits the path
                    summary = Child {
                        base,
                        is_leaf: false,
                    };
                }
            }
        }
    }
    if cur.peek().is_some() {
        return Err(Parse::Syntax); // trailing junk
    }
    if !canonical {
        return Err(Parse::NotCanonical);
    }
    let bits = builder.finish();
    validate_bits(&bits).expect("a canonical text parse builds a canonical skyline stream");
    // Canonicalizing the storage is `Version::from_bits`'s job, the
    // single gate a stream passes through when it becomes a stored value.
    Ok(bits)
}
