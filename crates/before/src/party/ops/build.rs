use crate::codec::{BitStack, BitsBuf, BitsView, PackedBuilder, PopStack};
use crate::idbits::{IdNode, IdReader};

/// Single-buffer builder for normalized id output.
///
/// A node reserves a 2-bit tag placeholder before its children are emitted;
/// [`close_node`](Self::close_node) patches the tag from which children turned
/// out present, collapsing `(1, 1) → 1` (both terminal) and `(0, 0) → 0` (both
/// empty). The id instantiation of the crate's append-truncate discipline
/// ([`PackedBuilder`] carries the shared move set): the per-node payload is
/// only the tag bits, and both collapses are pure truncations.
pub(super) struct IdBuilder {
    out: PackedBuilder,
}

/// What an emitted child turned out to be, so its parent's
/// [`close_node`](IdBuilder::close_node) can pick a tag and collapse.
///
/// `Empty` contributed no bits (a `0`), `Terminal` a lone owned leaf (a `1`),
/// `Node` an internal subtree. Carries no position: the tag is patched in place
/// at the parent's reserved slot and the children already sit contiguously
/// after it.
#[derive(Clone, Copy)]
pub(super) enum Built {
    /// The empty `0` region: no bits emitted.
    Empty,
    /// A single owned terminal (`1`).
    Terminal,
    /// An internal subtree.
    Node,
}

/// A just-reserved tag placeholder, awaiting its children and a
/// [`close_node`](IdBuilder::close_node).
///
/// `!Clone` and `#[must_use]`: the token must be closed exactly once, and the
/// borrow checker stops it being reused or dropped silently — so an open with
/// no matching close cannot compile.
#[must_use = "an opened node must be closed with close_node"]
pub(super) struct Open(u64);

/// The width of an id node's presence tag: one bit per child.
const TAG_BITS: usize = 2;

/// The output width of a node whose two children are both terminals: its own
/// tag followed by the two terminal tags.
const TERMINAL_PAIR_BITS: u64 = 3 * TAG_BITS as u64;

impl IdBuilder {
    pub(super) fn with_capacity(capacity: u64) -> Self {
        IdBuilder {
            out: PackedBuilder::with_capacity(capacity),
        }
    }

    /// Append an owned terminal (the `1` leaf): the tag `00` (no children).
    pub(super) fn terminal(&mut self) -> Built {
        self.push_tag(false, false);
        Built::Terminal
    }

    /// Append a node's 2-bit presence tag verbatim, already final.
    ///
    /// For an emitter that knows the tag at first sight —
    /// [`sum`](crate::idbits::IdReader::sum) writes each output tag final at
    /// descent — so no placeholder or patch is needed.
    pub(super) fn push_tag(&mut self, left: bool, right: bool) {
        self.out.push_bit(left);
        self.out.push_bit(right);
    }

    /// Reserve a node's 2-bit tag; its children are emitted next, then it is
    /// closed (and normalized) with [`close_node`](Self::close_node). The
    /// placeholder is patched to the real presence bits on close.
    pub(super) fn open(&mut self) -> Open {
        Open(self.out.reserve(TAG_BITS))
    }

    /// Copy one already-normal source subtree into the output, advancing `src`
    /// past it and reporting what it was.
    ///
    /// The source subtree is copied exactly once (a verbatim bit-range splice).
    /// A synthetic empty reader contributes nothing and reports
    /// [`Built::Empty`].
    pub(super) fn copy_reader(&mut self, src: &mut IdReader) -> Built {
        if matches!(src, IdReader::Empty) {
            return Built::Empty;
        }
        let is_terminal = matches!(src.peek(), IdNode::Full);
        let start = src.pos();
        src.skip();
        // The peek and the skip above record their own reads; the splice
        // records the write.
        self.out.splice(src.bits(), start, src.pos());
        if is_terminal {
            Built::Terminal
        } else {
            Built::Node
        }
    }

    /// Append an already-normal subtree's bits — the range `start..end` of
    /// `src` — verbatim (the splice records the write), for a spliced child
    /// whose kind the caller reports to [`close_node`](Self::close_node)
    /// itself.
    pub(super) fn splice(&mut self, src: BitsView<'_>, start: u64, end: u64) {
        self.out.splice(src, start, end);
    }

    /// Normalize and close the node opened at `node` from what its two children
    /// turned out to be, consuming the open token:
    ///
    /// - both empty ⇒ collapse to `0` (drop the tag, emit nothing);
    /// - both terminal ⇒ collapse to a single `1` (`(1, 1) → 1`);
    /// - otherwise patch the tag to record which children are present.
    pub(super) fn close_node(&mut self, node: Open, left: Built, right: Built) -> Built {
        let node = node.0;
        match (left, right) {
            (Built::Empty, Built::Empty) => {
                self.out.truncate(node); // (0, 0) → 0
                Built::Empty
            }
            (Built::Terminal, Built::Terminal) => {
                self.out.truncate(node); // (1, 1) → 1
                self.terminal()
            }
            (left, right) => {
                // The tag patched in place: bit 0 = left present, bit 1 =
                // right present.
                self.out.patch_bit(node, !matches!(left, Built::Empty));
                self.out.patch_bit(node + 1, !matches!(right, Built::Empty));
                Built::Node
            }
        }
    }

    /// Normalize the node just completed with two terminal children: retract
    /// its tag and both terminals — the trailing [`TERMINAL_PAIR_BITS`] of the
    /// output — and emit the single terminal the pair collapses to (`(1, 1) →
    /// 1`).
    ///
    /// The truncation twin of [`close_node`](Self::close_node)'s
    /// terminal-collapse arm, for an emitter that writes final tags at descent
    /// ([`sum`](crate::idbits::IdReader::sum)): such a node's tag and children
    /// are the last three tags in the output, so no recorded position is
    /// needed.
    pub(super) fn collapse_terminal_pair(&mut self) -> Built {
        self.out.truncate(self.out.len() - TERMINAL_PAIR_BITS);
        self.terminal()
    }

    pub(super) fn finish(self) -> BitsBuf {
        self.out.finish()
    }
}

/// Leaf-driven builder for normalized id output: append one plateau per
/// elementary interval of a dyadic tiling, in preorder, and take the canonical
/// id of the region the owned plateaus tile.
///
/// A whole already-normal subtree of the tiling may be appended in one splice
/// instead of plateau by plateau ([`subtree`](Self::subtree)).
///
/// The id-side sibling of the event emission's collapsing builder (the skyline
/// build module): the preorder leaf depths of a dyadic tiling determine the
/// tree, so the builder derives every presence tag itself — reserving each
/// node's tag as it is entered ([`IdBuilder::open`]) and normalizing as each
/// node closes ([`IdBuilder::close_node`]: both collapses plus the presence
/// patch). An unowned plateau contributes no bits, exactly as a stored `0`
/// occupies none.
///
/// Transient state is bits per open ancestor and nothing per node: the
/// branch-direction path, two kind bits per right-branch level, and the
/// reserved tags' positions on a delta-coded bit stack ([`PosStack`]) — never a
/// stack frame or a per-level machine word, so a deep output costs bits, not
/// grown segments.
pub(super) struct IdSkylineBuilder {
    out: IdBuilder,
    /// Root-to-current branch directions: `false` inside a left child,
    /// `true` inside a right.
    path: BitStack,
    /// Two bits per right-branch level: what the completed left sibling
    /// built (see [`push_kind`](Self::push_kind)).
    left_kinds: BitStack,
    /// The open ancestors' reserved tag positions, innermost last.
    tags: PosStack,
    /// The whole tiling's result, set when the last plateau closes the
    /// root.
    root: Option<Built>,
}

impl IdSkylineBuilder {
    /// Create a builder with room for `capacity` output bits.
    pub(super) fn with_capacity(capacity: u64) -> Self {
        IdSkylineBuilder {
            out: IdBuilder::with_capacity(capacity),
            path: BitStack::new(),
            left_kinds: BitStack::new(),
            tags: PosStack::new(),
            root: None,
        }
    }

    /// Append the next plateau: a leaf at `depth` (its interval has width
    /// `2^-depth`), owned or unowned.
    ///
    /// The plateau sequence must be the preorder tiling of one dyadic tree:
    /// each new depth must be reachable from the last by the forced
    /// flip-and-descend, which the builder debug-asserts.
    pub(super) fn leaf(&mut self, depth: u64, owned: bool) {
        debug_assert!(
            self.root.is_none(),
            "a plateau arrived after the final one: the tiling is complete"
        );
        debug_assert!(
            depth >= self.path.len(),
            "a plateau depth above its forced flip level: the input is not one preorder tiling"
        );
        // Open an ancestor per level entered, its tag reserved for the
        // close-time patch.
        for _ in self.path.len()..depth {
            let Open(at) = self.out.open();
            self.tags.push(at);
            self.path.push(false);
        }
        let kind = if owned {
            self.out.terminal()
        } else {
            Built::Empty
        };
        self.close_up(kind);
    }

    /// Append a whole canonical internal subtree at `depth` as one verbatim
    /// splice: the block form of [`leaf`](Self::leaf), for a region whose
    /// plateaus are one operand's own tiling unchanged.
    ///
    /// `src` must be the complete packed encoding of one *internal* subtree in
    /// normal form (a fully-owned region is a [`leaf`](Self::leaf), and an
    /// unowned one contributes no bits). The splice preserves the builder's
    /// normalization invariants at its boundary: the interior needs no repair
    /// because a subtree of a normal id is itself normal, and the subtree
    /// closes upward as [`Built::Node`] — exactly what re-deriving it plateau
    /// by plateau would close as (its root has a child that is neither
    /// both-empty nor both-terminal), so the ancestors' presence patches and
    /// collapses are unchanged.
    pub(super) fn subtree(&mut self, depth: u64, src: BitsView<'_>, start: u64, end: u64) {
        debug_assert!(
            self.root.is_none(),
            "a subtree arrived after the final plateau: the tiling is complete"
        );
        debug_assert!(
            depth >= self.path.len(),
            "a subtree depth above its forced flip level: the input is not one preorder tiling"
        );
        debug_assert!(
            src.bit(start) || src.bit(start + 1),
            "a spliced block is an internal subtree, never a lone terminal"
        );
        // Open an ancestor per level entered, exactly as a leaf would.
        for _ in self.path.len()..depth {
            let Open(at) = self.out.open();
            self.tags.push(at);
            self.path.push(false);
        }
        self.out.splice(src, start, end);
        self.close_up(Built::Node);
    }

    /// Take the finished canonical stream (empty for a wholly unowned tiling).
    pub(super) fn finish(self) -> BitsBuf {
        debug_assert!(
            self.root.is_some(),
            "an id tiling closes its root exactly once"
        );
        self.out.finish()
    }

    /// Close finished subtrees upward from a completed child of kind `kind`:
    /// flip a left child to its right sibling and stop, or pop a right child's
    /// level, normalize its node, and continue upward.
    ///
    /// The root's completion records the whole tiling's result.
    fn close_up(&mut self, mut kind: Built) {
        loop {
            match self.path.pop() {
                None => {
                    self.root = Some(kind);
                    return;
                }
                Some(false) => {
                    // The left child completed: its right sibling's plateaus
                    // are next.
                    self.path.push(true);
                    self.push_kind(kind);
                    return;
                }
                Some(true) => {
                    // The right child completed: normalize and close the node,
                    // and continue with what it built.
                    let left = self.pop_kind();
                    kind = self.out.close_node(Open(self.tags.pop()), left, kind);
                }
            }
        }
    }

    /// Record a completed left sibling's kind: two bits, is-node then
    /// is-terminal (`Empty` is neither).
    fn push_kind(&mut self, kind: Built) {
        self.left_kinds.push(matches!(kind, Built::Node));
        self.left_kinds.push(matches!(kind, Built::Terminal));
    }

    /// Pop the innermost recorded kind.
    fn pop_kind(&mut self) -> Built {
        let terminal = self.left_kinds.pop().expect("kind entries are two bits");
        let node = self.left_kinds.pop().expect("kind entries are two bits");
        match (node, terminal) {
            (false, false) => Built::Empty,
            (false, true) => Built::Terminal,
            (true, false) => Built::Node,
            (true, true) => unreachable!("a kind is one of three values"),
        }
    }
}

/// A pop-able stack of the open ancestors' reserved tag positions: delta-coded
/// bits plus one absolute register.
///
/// Each entry stores its delta from the entry under it on a [`PopStack`], with
/// the top entry's absolute position in one register. Positions increase up the
/// stack, and a descent chain reserves adjacent tags (delta 2), so an entry
/// typically costs ~4 bits where a machine word would cost 64: depth costs bits
/// here the same way it does in the path stacks.
struct PosStack {
    /// The innermost entry's absolute position (0 when empty).
    top: u64,
    /// The entries' deltas from the entry under them, stored off by one
    /// so the width is nonzero even at delta 0 (the first entry at
    /// position 0).
    deltas: PopStack,
}

impl PosStack {
    fn new() -> Self {
        PosStack {
            top: 0,
            deltas: PopStack::new(),
        }
    }

    /// Push a position at or above the current top.
    fn push(&mut self, pos: u64) {
        debug_assert!(pos >= self.top, "reserved tag positions never move left");
        self.deltas.push(pos - self.top + 1);
        self.top = pos;
    }

    /// Pop the innermost position.
    ///
    /// # Panics
    ///
    /// Panics if the stack is empty.
    fn pop(&mut self) -> u64 {
        let pos = self.top;
        self.top -= self.deltas.pop() - 1;
        pos
    }
}
