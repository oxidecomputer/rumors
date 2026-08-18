//! Skyline-first text kernels: the paper notation rendered from and parsed into
//! skyline streams directly.
//!
//! The paper's event grammar (`n | (n, e1, e2)`, separator `", "`) spells the
//! tree's min-lifted node bases, while a skyline stream stores topology plus
//! delta-coded absolute leaf heights — so each direction is a change of
//! coordinates, done here without materializing either a base tree or any
//! absolute height:
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
//!   zigzag-coded, fed to the collapsing output builder, and *reset*.
//!   The reset settles the digit buffer's *top* (its high-water digit
//!   index, `suanpan`'s word for it) back to zero, so every extraction
//!   pays the span written since the previous leaf and a trailing run
//!   of zero-delta leaves after one wide swing stays O(1) each. That
//!   exact-top discipline is what keeps the walk linear, and it is
//!   pinned twice in `tests/meter.rs`: the wide-arming flatness band
//!   (per-byte touches flat across a size doubling on the wide-swing
//!   family), and the committed schoolbook kernel beside this module's
//!   tests — a known-bad twin that re-zeroes by compensating
//!   subtraction, which leaves the top parked at the swing's width and
//!   re-walks its dead digits once per later leaf, so it reads
//!   superlinear and proves the reset is the load-bearing move. The
//!   accumulator is the cliff-free [`Accumulator`]: a plain big-integer running
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
//! Both walks are iterative — parallel open-node stacks (one phase bit per open
//! node, with the per-level state in chunked side stacks) and never the call
//! stack — so depth grows no stack segments at any input size, and a deep
//! spine's transient is priced per open node without enum padding or a doubling
//! buffer's realloc coexistence.
//!
//! The kernels are module-private to the skyline codec (test- and
//! meter-visible) and *are* the production text path: `Display` routes to
//! [`render`] and `FromStr` to [`parse`], so the public-entry asserts in the
//! test suite pin entry agreement, not an independent value. The independent
//! legs are the construction-language transcoder — [`parse`] of rendered text
//! must land on the transcoder's stream byte for byte over the generator
//! families — the render↔parse inverse pair, and a deterministic accept/reject
//! corpus with a byte-mutation sweep pinning the grammar's decisions. The
//! resource-envelope suite (`tests/meter.rs`) pins both kernels' transients on
//! the adversarial families.

use core::cmp::Ordering;
use core::fmt::Write as _;

use suanpan::Accumulator;

use crate::codec::text::{parse_base, Cur};
use crate::codec::{Base, BitCursor, BitsMut, BitsView, DsiCursor};
use crate::error::Parse;

use super::build::SkylineBuilder;
use super::signed::{gamma_code, gamma_code_signed, signed_sum, unzigzag_base, Sign};

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

/// The widest path-sum digit buffer the parse's per-leaf re-zeroing keeps
/// pooled, in accumulator digits.
///
/// A machine-word delta deposits across at most two base-2^32 digits; four is
/// double that span, so every word-scale schedule reuses one pooled buffer and
/// never allocates per leaf. A wider buffer means a wide swing just extracted —
/// work funded by the swing's own spelled digits, as the next wide leaf's will
/// be — and keeping it would pin swing-sized capacity under everything the rest
/// of the parse allocates: the high-water genre in the memory denomination, so
/// the parse drops the buffer whole instead.
const PATH_SUM_KEEP_DIGITS: usize = 4;

/// What a completed subtree contributes to its parent's merge, all in
/// coordinates relative to the subtree's own entry (first) leaf.
///
/// `drop` is entry minus the subtree's floor (non-negative: the floor is a
/// minimum over leaves including the entry); `span` is the signed offset of the
/// subtree's last leaf from its entry; `incoming` is the signed stream delta
/// that carried the entry leaf itself. Each is a sum of leaf deltas, so its
/// width is priced by the deltas' own codes.
///
/// This is the *flowing* form, carrying the subtree's preorder root index; a
/// summary parked while its parent awaits the right child is stored as a
/// [`StoredLeft`], whose root is derivable from the parent's.
struct Summary {
    /// Preorder index of the subtree's root node, where the merge writes the
    /// finalized printed base.
    root: usize,
    /// Entry leaf height minus the subtree's floor (its minimum leaf height):
    /// non-negative.
    drop: Base,
    /// The last leaf's height minus the entry leaf's, as (sign, magnitude).
    span: (Sign, Base),
    /// The stream delta that carried the entry leaf, as (sign, magnitude);
    /// the whole stream's first leaf stores the (unused) positive zero.
    incoming: (Sign, Base),
}

/// A completed left-child summary parked until its parent's right subtree
/// closes: [`Summary`] without the root index.
///
/// The root is always the parent's preorder successor (a left child directly
/// follows its parent in preorder), so the merge re-derives it instead of
/// storing one word per parked level.
struct StoredLeft {
    /// Entry leaf height minus the subtree's floor: non-negative.
    drop: Base,
    /// The last leaf's height minus the entry leaf's, as (sign, magnitude).
    span: (Sign, Base),
    /// The stream delta that carried the entry leaf, as (sign, magnitude).
    incoming: (Sign, Base),
}

/// How many parked entries one [`ParkedStack`] chunk holds: small enough that a
/// shallow walk's single chunk sits inside the board's flat heap allowance,
/// large enough that the chunk spine stays negligible.
const PARKED_CHUNK: usize = 64;

/// A LIFO in fixed-size chunks: both text walks' open-node side stacks.
///
/// The render parks one left-child summary per open node past its left phase;
/// the parse parks each open node's written base and its left summary the same
/// way — so on a deep spine these stacks hold one entry per level, the walks'
/// dominant transient. Chunking bounds the live slack to one chunk and, unlike
/// a doubling `Vec`, never holds an old and a new buffer at once during growth:
/// that realloc coexistence spike is exactly what pushed the deep left-full
/// shapes over the board's heap ceiling, and a chunk never moves once
/// allocated.
struct ParkedStack<T> {
    /// The stack's chunks, oldest first; every chunk but the last is full, and
    /// no empty chunk is kept.
    chunks: Vec<Vec<T>>,
    /// One drained chunk cached across a boundary, so a push/pop oscillation at
    /// a chunk edge does not thrash the allocator.
    spare: Option<Vec<T>>,
}

impl<T> ParkedStack<T> {
    fn new() -> ParkedStack<T> {
        ParkedStack {
            chunks: Vec::new(),
            spare: None,
        }
    }

    fn push(&mut self, entry: T) {
        if self
            .chunks
            .last()
            .is_none_or(|chunk| chunk.len() == PARKED_CHUNK)
        {
            let chunk = self
                .spare
                .take()
                .unwrap_or_else(|| Vec::with_capacity(PARKED_CHUNK));
            self.chunks.push(chunk);
        }
        self.chunks
            .last_mut()
            .expect("a chunk with room is on top")
            .push(entry);
    }

    fn pop(&mut self) -> Option<T> {
        let chunk = self.chunks.last_mut()?;
        let entry = chunk.pop().expect("no empty chunk is kept");
        if chunk.is_empty() {
            self.spare = self.chunks.pop();
        }
        Some(entry)
    }

    fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }
}

/// Render a skyline stream as the paper notation of the version the stream
/// codes; the public `Display` entry routes here.
///
/// Two passes. The finalize pass walks the stream once, merging the module
/// doc's relative subtree summaries bottom-up; each merge renders the one
/// printed base it finalizes straight into a digit arena keyed by preorder node
/// index (zero bases — every on-floor child — occupy no arena bytes at all),
/// and the output is sized exactly from the arena, the zero-base node count,
/// and the topology. The emit pass sorts the arena keys once and writes the
/// sized output straight through: one phase bit per open node, no recursion,
/// and the exactness of the sizing is asserted. The transient is priced
/// accordingly: per node one topology bit, per open node one phase bit and one
/// index word (plus the parked left summary where the node awaits its right
/// child), and digits only for the bases that actually print — never a per-node
/// `Base` vector or offset table.
///
/// # Panics
///
/// Panics if the stream is not a canonical skyline encoding.
pub fn render(bits: BitsView<'_>) -> String {
    let mut cursor = DsiCursor::new(bits);

    // Finalize state. `topology`: per-node internal flags (semantic, not the
    // wire bits: `true` = internal). The open-node stacks, innermost last:
    // `phase` holds one bit per open node ([`LEFT_PHASE`] while it awaits its
    // left child), `open` its preorder index, and `lefts` the parked left-child
    // summary of each open node past its left phase — parallel stacks, where an
    // enum-of-frames layout would pad every open level to its widest variant.
    let mut topology = BitsMut::new();
    let mut phase = BitsMut::new();
    let mut open: Vec<usize> = Vec::new();
    let mut lefts = ParkedStack::new();
    // The digit arena: every printed nonzero base, rendered at the merge that
    // finalizes it (so entries appear in merge order, not preorder), each
    // terminated by [`ARENA_SEP`]; `entries` maps preorder node index to its
    // arena start.
    let mut arena = String::new();
    let mut entries: Vec<(usize, usize)> = Vec::new();
    let mut first_height: Option<Base> = None;
    let root_summary = 'tree: loop {
        // One whole descent per unary read: the run's internal nodes, then the
        // leaf whose flag terminates the run.
        let internal_nodes = cursor.read_unary().expect("canonical skyline bits");
        for _ in 0..internal_nodes {
            phase.push(LEFT_PHASE);
            open.push(topology.len());
            topology.push(true);
        }
        let index = topology.len();
        topology.push(false);
        // The cursor's own `read_int`: word-parallel payload decode.
        let code = cursor
            .read_int()
            .expect("canonical skyline bits")
            .into_base();
        let incoming = if first_height.is_some() {
            unzigzag_base(code)
        } else {
            first_height = Some(code);
            (Sign::Positive, Base::ZERO)
        };
        let mut summary = Summary {
            root: index,
            drop: Base::ZERO,
            span: (Sign::Positive, Base::ZERO),
            incoming,
        };
        // Close every subtree this leaf completes.
        loop {
            match phase.pop() {
                None => break 'tree summary,
                Some(LEFT_PHASE) => {
                    // The top open node's left child just completed: park its
                    // summary and await the right child. Its root is the
                    // parent's preorder successor, so the parked form drops it.
                    debug_assert_eq!(
                        summary.root,
                        open.last().expect("an open node owns this phase bit") + 1,
                        "a left child directly follows its parent in preorder"
                    );
                    phase.push(!LEFT_PHASE);
                    lefts.push(StoredLeft {
                        drop: summary.drop,
                        span: summary.span,
                        incoming: summary.incoming,
                    });
                    break;
                }
                Some(_) => {
                    let parent = open.pop().expect("an open node owns this phase bit");
                    let left = lefts
                        .pop()
                        .expect("a node past its left phase parked a summary");
                    summary = merge(parent, left, summary, &mut arena, &mut entries);
                }
            }
        }
    };
    assert_eq!(
        cursor.position_u64(),
        bits.len(),
        "a canonical skyline stream is exactly one tree"
    );
    debug_assert!(
        open.is_empty() && lefts.is_empty(),
        "a canonical stream closes every node it opens"
    );
    // The root's printed base is the one absolute quantity: the stored first
    // height minus the root's drop (non-negative: the global floor is a leaf
    // height, and heights are naturals).
    let root_base =
        first_height.expect("a skyline stream has at least one leaf") - &root_summary.drop;
    push_base(&mut arena, &mut entries, root_summary.root, &root_base);

    // Size exactly: printed digits are the arena entries (less their
    // terminators) plus one `0` per node without an entry, and each internal
    // node adds its fixed syntax bytes.
    let exact = (arena.len() - entries.len())
        + (topology.len() - entries.len())
        + topology.count_ones() * INTERNAL_SYNTAX_BYTES;

    // The finalize-only stacks are drained; release them before the output
    // materializes rather than holding their capacity across the emit pass.
    drop(phase);
    drop(open);
    drop(lefts);
    // Merge order becomes preorder with one sort of the (node, start) keys —
    // node indexes are distinct, so the order is total and the emit below
    // consumes the entries with a single forward cursor.
    entries.sort_unstable();

    // Emit: preorder over the finalized topology and arena, one phase bit per
    // open node.
    //
    // Allocation-strategy seam: the shipped arm requests the exact size, one
    // allocation, never grown (the assert below pins the exactness). The
    // `before_alloc_ab` cfg — `RUSTFLAGS`-only, never a cargo feature, so no
    // dependent build can select it — compiles in the growth-from-empty arm so
    // the allocation benchmark can price the exact request against doubling
    // growth on this site; shipped builds always take the exact arm.
    #[cfg(not(before_alloc_ab = "display_growth"))]
    let mut out = String::with_capacity(exact);
    #[cfg(before_alloc_ab = "display_growth")]
    let mut out = String::new();
    let mut pending = BitsMut::new();
    let mut next_entry = 0usize;
    for (node, internal) in topology.iter().by_vals().enumerate() {
        let digits: &str = match entries.get(next_entry) {
            Some(&(entry_node, start)) if entry_node == node => {
                next_entry += 1;
                let len = arena[start..]
                    .find(ARENA_SEP)
                    .expect("every arena entry is terminated");
                &arena[start..start + len]
            }
            // No arena entry: the node's printed base is zero.
            _ => "0",
        };
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
    debug_assert_eq!(next_entry, entries.len(), "every arena entry printed");
    assert_eq!(
        out.len(),
        exact,
        "the finalize pass sizes the rendered text exactly"
    );
    out
}

/// The digit arena's entry terminator: printed bases are decimal digits, so any
/// non-digit byte delimits an entry, and the emit pass scans to it for the
/// entry's length.
const ARENA_SEP: char = ';';

/// Render one finalized printed base into the digit arena, keyed by its node's
/// preorder index — unless it is zero.
///
/// Every on-floor child's base is zero: zero bases take no arena bytes, and the
/// emit pass prints `0` for any node without an entry.
fn push_base(arena: &mut String, entries: &mut Vec<(usize, usize)>, node: usize, base: &Base) {
    if base.bits() == 0 {
        return;
    }
    entries.push((node, arena.len()));
    write!(arena, "{base}").expect("String formatting is infallible");
    arena.push(ARENA_SEP);
}

/// Merge a closed internal node's two child summaries, rendering the one
/// printed base the merge finalizes into the digit arena.
///
/// With `right_floor` the right child's floor relative to the left's entry
/// leaf (`span(left) + incoming(right) − drop(right)`), the node's floor is
/// `min(−drop(left), right_floor)`, still relative to the left entry — so the
/// min-lift decision quantity `lift = right_floor + drop(left)` settles which
/// child sits on the node's floor. Frame-free, `lift` is
/// `floor(right) − floor(left)`: whichever child's floor is lower sits on
/// the node's floor, and the other child's printed base is the gap. So a
/// non-negative `lift` says the left child
/// does (its base is zero and `lift` is the right child's base), a negative
/// `lift` says the right child does (its magnitude is the left child's base
/// and the node's drop deepens by it). The child the merge leaves off the
/// arena prints as zero, so each merge stores at most one base.
fn merge(
    parent: usize,
    left: StoredLeft,
    right: Summary,
    arena: &mut String,
    entries: &mut Vec<(usize, usize)>,
) -> Summary {
    // The right child's entry leaf, relative to the left child's entry.
    let entry_step = signed_sum(
        left.span.0,
        left.span.1,
        right.incoming.0,
        &right.incoming.1,
    );
    // The node's span: its last leaf is the right child's last.
    let span = signed_sum(
        entry_step.0,
        entry_step.1.clone(),
        right.span.0,
        &right.span.1,
    );
    // The right child's floor.
    let right_floor = signed_sum(entry_step.0, entry_step.1, Sign::Negative, &right.drop);
    // The lift: floor(right) − floor(left).
    let (lift_sign, lift) = signed_sum(right_floor.0, right_floor.1, Sign::Positive, &left.drop);
    let drop = if lift_sign.is_negative() {
        // The left child's root is the parent's preorder successor.
        push_base(arena, entries, parent + 1, &lift);
        left.drop + &lift
    } else {
        push_base(arena, entries, right.root, &lift);
        left.drop
    };
    Summary {
        root: parent,
        drop,
        span,
        incoming: left.incoming,
    }
}

/// Parse the paper notation into the canonical skyline stream of the version it
/// spells; the public `FromStr` entry routes here.
///
/// One iterative pass: bases parse through the delegated digit-run reader, the
/// per-leaf delta accumulator turns path-sum movement into skyline payloads
/// (the module doc's discipline), and the collapsing output builder assembles
/// the stream. Canonicality (a zero-base child under every node, no equal
/// sibling leaves) is checked at each close and reported after the whole syntax
/// pass, so syntax errors — including trailing junk — outrank
/// [`Parse::NotCanonical`].
pub fn parse(text: &str) -> Result<BitsMut, Parse> {
    /// What a parsed subtree contributes to its parent's normal-form check: its
    /// written base and whether it is a single leaf.
    struct Child {
        base: Base,
        is_leaf: bool,
    }

    let mut cursor = Cur::new(text);
    let mut builder = SkylineBuilder::with_capacity(text.len());
    // The open-node stacks, innermost last — the render's parallel-stack
    // discipline: `phase` holds one bit per open node ([`LEFT_PHASE`] while it
    // awaits its left child), `bases` the node's own written base, and `lefts`
    // the parked left-child summary of each open node past its left phase. An
    // enum-of-frames layout would pad every open level to its widest variant,
    // and a flat `Vec` of frames holds an old and a new buffer at once while it
    // doubles — on a deep spine that padded coexistence alone is most of the
    // parse's transient.
    let mut phase = BitsMut::new();
    let mut bases: ParkedStack<Base> = ParkedStack::new();
    let mut lefts: ParkedStack<Child> = ParkedStack::new();
    // The signed height movement since the last emitted leaf.
    let mut delta = Accumulator::new();
    let mut emitted_first = false;
    let mut canonical = true;

    'nodes: loop {
        // Parse one node at the cursor.
        match cursor.peek() {
            Some(b'(') => {
                cursor.bump();
                let base = parse_base(&mut cursor)?;
                if cursor.bump() != Some(b',') {
                    return Err(Parse::Syntax);
                }
                delta.add_magnitude(&base);
                phase.push(LEFT_PHASE);
                bases.push(base);
                continue 'nodes;
            }
            Some(byte) if byte.is_ascii_digit() => {}
            _ => return Err(Parse::Syntax),
        }
        let base = parse_base(&mut cursor)?;
        delta.add_magnitude(&base);

        // Emit the leaf's plateau: the accumulated movement is exactly the
        // leaf-to-leaf delta (absolute for the first leaf), and the re-zeroing
        // below readies the accumulator for the next one. The re-zeroing is a
        // reset — not a compensating subtraction of the extracted magnitude —
        // which is what keeps the walk linear: it settles the digit buffer's
        // top back to zero, so the next extraction pays the span written since
        // *this* leaf, never a stale wide spelling left cancelling above it (a
        // value-zero subtraction can leave the top parked at the widest swing,
        // and every later leaf would re-walk those dead digits — the
        // exact-`top` genre the wide-arming pins hold).
        let (sign, magnitude) = delta.sign_magnitude();
        if delta.digit_count() > PATH_SUM_KEEP_DIGITS {
            // The same genre in the memory denomination: a reset keeps the
            // buffer's capacity, so one wide swing would otherwise pin a
            // swing-sized buffer under everything the rest of the parse
            // allocates. A wide extraction was funded by its own spelled
            // digits, and so is the next one — drop the buffer whole and let
            // the pool hold word-scale buffers only.
            delta = Accumulator::new();
        } else {
            delta.reset();
        }
        let code = if emitted_first {
            gamma_code_signed(
                Sign::from_is_negative(sign == Ordering::Less),
                &Base::from(magnitude),
            )
        } else {
            emitted_first = true;
            debug_assert_ne!(sign, Ordering::Less, "a path sum of naturals is a natural");
            gamma_code(&Base::from(magnitude))
        };
        builder.leaf(phase.len(), code);
        delta.sub_magnitude(&base); // the leaf's base exits the path

        // Close every node the leaf completes.
        let mut summary = Child {
            base,
            is_leaf: true,
        };
        loop {
            match phase.pop() {
                None => break 'nodes,
                Some(LEFT_PHASE) => {
                    if cursor.bump() != Some(b',') {
                        return Err(Parse::Syntax);
                    }
                    phase.push(!LEFT_PHASE);
                    lefts.push(summary);
                    continue 'nodes;
                }
                Some(_) => {
                    if cursor.bump() != Some(b')') {
                        return Err(Parse::Syntax);
                    }
                    let left = lefts
                        .pop()
                        .expect("a node past its left phase parked a summary");
                    let base = bases.pop().expect("an open node owns this phase bit");
                    // Normal form: a zero-base child under every node, and no
                    // equal sibling leaves (which the builder collapses rather
                    // than stores, so the flag is the only witness).
                    if left.base != Base::ZERO && summary.base != Base::ZERO {
                        canonical = false;
                    }
                    if left.is_leaf && summary.is_leaf && left.base == summary.base {
                        canonical = false;
                    }
                    delta.sub_magnitude(&base); // the closed node's base exits the path
                    summary = Child {
                        base,
                        is_leaf: false,
                    };
                }
            }
        }
    }
    if cursor.peek().is_some() {
        return Err(Parse::Syntax); // trailing junk
    }
    if !canonical {
        return Err(Parse::NotCanonical);
    }
    let bits = builder.finish();
    // Canonicality of the built stream is pinned by the render↔parse inverse
    // pair and the transcoder differential (the module doc's independent legs).
    // Canonicalizing the storage is `Version::from_bits`'s job, the
    // single gate a stream passes through when it becomes a stored value.
    Ok(bits)
}
