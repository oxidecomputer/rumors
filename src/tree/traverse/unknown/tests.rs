//! The classifier's fused-walk cost: the dominance face against the
//! two-pass placement shape it replaces, compared on a wide-divergence
//! tree with deterministic meters.

#![cfg(feature = "meter")]

use before::meter;

use crate::{
    Version, causally,
    message::Message,
    tree::{
        arb::nth_party,
        traverse::{Action, act},
        typed::{
            self, Children, Node, Path,
            height::{Height, S, Z},
        },
    },
};

use super::Unknown;

/// The two-check classification the fused dominance face replaced, kept
/// as this comparison's cost oracle.
///
/// Each check compares one node bound against the counterparty's
/// known version, so the probe stream is decoded once per check.
/// Verdict-identical to [`Unknown`] by the
/// `span_place_matches_relations` and
/// `span_dominance_coarsens_place` laws in `before::laws`; this
/// suite additionally asserts the pruned trees match.
trait TwoPass: Height {
    /// The two-check spelling of [`Unknown::unknown`] at this height.
    fn unknown(node: Option<Node<Self>>, known: &Version) -> Option<Node<Self>>;
}

impl<H: TwoPass> TwoPass for S<H>
where
    S<H>: Height,
{
    fn unknown(node: Option<Node<Self>>, known: &Version) -> Option<Node<Self>> {
        let node = node?;
        // Check one: a floor the known version does not contain
        // (concurrent with or above `known`) is a wholly unknown subtree.
        if causally::since(known).contains(node.floor()) {
            return Some(node);
        }
        // Check two: a ceiling within the known version's past is a
        // wholly known subtree — the second decode of `known` the fusion
        // ends.
        if causally::before(known).contains(node.ceiling()) {
            return None;
        }
        Node::branch({
            let mut children = Children::default();
            for (radix, child) in node.into_children() {
                if let Some(child) = TwoPass::unknown(Some(child), known) {
                    children.insert(radix, child);
                }
            }
            children
        })
    }
}

impl TwoPass for Z {
    fn unknown(node: Option<Node<Self>>, known: &Version) -> Option<Node<Self>> {
        let node = node?;
        if causally::before(known).contains(node.ceiling()) {
            return None;
        }
        Some(node)
    }
}

/// A wide-divergence tree: `known_leaves` party-0 leaves whose join is
/// the counterparty's `known` version, interleaved (by hash radix) with
/// `divergent_leaves` party-1 leaves concurrent to all of it.
///
/// Under hash placement the two populations scatter across the same
/// branches, so the classifier sees every verdict: wholly-known
/// subtrees to shed, wholly-unknown subtrees to keep whole, and mixed
/// branches to descend.
fn wide_divergence(
    known_leaves: usize,
    divergent_leaves: usize,
) -> (Option<typed::node::Root>, Version) {
    let mut actions: Vec<(Path, Version, Action)> = Vec::new();
    let mut known = Version::new();

    for (party_index, count, flagged) in [(0, known_leaves, true), (1, divergent_leaves, false)] {
        let party = nth_party(party_index);
        let mut version = Version::new();
        for _ in 0..count {
            version.tick(&party);
            let message = Message::new(());
            let path = Path::for_leaf(&version);
            actions.push((path, version.clone(), Action::Insert(message)));
            if flagged {
                known |= version.clone();
            }
        }
    }

    (act(None, actions, &mut |_| ()), known)
}

/// `body`'s result with its scanned-bits reading, on a fresh counter.
fn scanned<R>(body: impl FnOnce() -> R) -> (R, u64) {
    meter::reset_scan_bits();
    let out = body();
    (out, meter::scan_bits())
}

/// The fused classifier prunes the identical tree the two-pass shape
/// prunes, for strictly fewer scanned bits, on a wide-divergence tree.
///
/// The comparison isolates the shipping shapes: both runs make the same
/// traversal over the same structurally-shared nodes (memo cells warmed
/// by an unmetered first run), reach the same verdicts at every node —
/// the pruned trees' root hashes are asserted equal — and pay the same
/// reassembly. The measured readings are the walks themselves in every
/// build: the memoized bounds are stored as one span, ordered by
/// construction, so the fused classification pays no validating
/// comparison anywhere — there is no dev-build assertion cost to
/// subtract. The pinned inequality is one probe decode per classified
/// node against the two-pass shape's two, with the dominance bail
/// landing at the first refuting interval.
#[test]
fn fused_classifier_undercuts_the_two_pass_shape() {
    let (root, known) = wide_divergence(96, 96);

    // Warm the shared bounds memo cells so neither measured run pays
    // the lazy fold; clones are Arc bumps over the same cells.
    let _ = Unknown::unknown(root.clone(), &known);

    let (two_pass, two_pass_scan) = scanned(|| TwoPass::unknown(root.clone(), &known));
    let (fused, fused_scan) = scanned(|| Unknown::unknown(root.clone(), &known));

    assert_eq!(
        typed::Node::root_hash(&fused),
        typed::Node::root_hash(&two_pass),
        "the fused classifier and the two-pass shape must prune identically"
    );
    eprintln!("MEASURED classifier_scan: fused={fused_scan} two_pass={two_pass_scan}");
    assert!(
        fused_scan > 0 && two_pass_scan > 0,
        "a live scan meter reads nonzero on every leg"
    );
    assert!(
        fused_scan < two_pass_scan,
        "the fused classifier ({fused_scan}) must scan strictly under the \
         two-pass shape ({two_pass_scan})"
    );
}
