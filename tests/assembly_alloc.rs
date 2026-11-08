//! The transient memory of assembling one supplied subtree on the in-memory
//! backend, against the subtree's size.
//!
//! A reply decoder hands each supplied subtree's leaves, in path order, to the
//! backend's assembler, which builds the subtree as they arrive. On the
//! in-memory backend the build keeps only the subtree's open right spine: at
//! most one open branch per path byte, each with the children closed beneath it
//! so far. Every other block it allocates is a node of the finished subtree,
//! which joins the replica. So the transient bytes it holds at once must not
//! grow with the subtree: assembling `n` leaves may hold at most what
//! assembling one leaf holds, plus the spine's bound
//! ([`ASSEMBLY_SPINE_BYTES`]).
//!
//! The bound is relative to the one-leaf build so that it judges only the
//! build: the stream around it and the poller allocate a fixed amount whatever
//! the subtree, and the one-leaf build has no branch to open.

use alloc_meter::{Label, trace};
use rumors::testing::{ASSEMBLY_SPINE_BYTES, assemble_supplied, supplied_leaves};

/// Trace events reserved per supplied leaf: a leaf's build allocates at most a
/// branch node and a fan's heap slots for it, and a fan's growth adds a
/// reallocation now and then.
const EVENTS_PER_LEAF: usize = 8;

/// Trace events reserved for the build's fixed overhead.
const SETUP_EVENTS: usize = 1 << 10;

/// What assembling `count` leaves left: the most transient bytes it held at
/// once, and the blocks that survive in the assembled subtree.
fn assemble(count: usize) -> (usize, usize) {
    let leaves = supplied_leaves(count);
    let (trace, subtree) = trace(EVENTS_PER_LEAF * count + SETUP_EVENTS, || {
        assemble_supplied(leaves)
    });
    assert_eq!(subtree.leaves(), count, "the subtree holds every leaf");
    let lifetimes = trace
        .lifetimes()
        .unwrap_or_else(|incomplete| panic!("raise EVENTS_PER_LEAF: {incomplete:?}"));
    let build = lifetimes.label(Label::NONE);
    (build.transient_peak, build.surviving_blocks)
}

/// Assembling a supplied subtree holds transient memory bounded by its open
/// spine, at every size from one leaf to a hundred thousand.
///
/// A hundred thousand leaves is the size of a root child's subtree when a
/// replica of about 25 million messages supplies a peer that holds none.
///
/// Each build of two or more leaves must leave at least one block in the
/// subtree, its top branch, so a trace that stopped seeing the build cannot
/// pass.
#[test]
fn assembly_holds_its_open_spine_at_most() {
    let (alone, _) = assemble(1);
    for count in [2, 1_000, 100_000] {
        let (peak, surviving) = assemble(count);
        assert!(
            peak <= alone + ASSEMBLY_SPINE_BYTES,
            "assembling {count} leaves held {peak} transient bytes, over the one-leaf build's \
             {alone} plus the open spine's {ASSEMBLY_SPINE_BYTES}",
        );
        assert!(surviving >= 1, "assembling {count} leaves built no branch");
    }
}
