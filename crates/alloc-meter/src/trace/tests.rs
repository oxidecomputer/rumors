//! Tests of trace recording against the allocator's real requests.
//!
//! The recording test issues raw allocator requests and predicts each event
//! from the blocks it holds, so it checks what the allocator records and
//! nothing about what the events mean.

use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::Index;

use super::{Event, EventKind, Label, labelled, trace, untraced};
use crate::tests::Block;

/// Labels the generated histories draw from: few enough that each recurs.
const LABELS: u32 = 3;

/// One allocator request, with the label and tracing scope it runs under.
#[derive(Clone, Copy, Debug)]
struct Op {
    /// The request itself.
    request: Request,
    /// The label current while it runs.
    label: Label,
    /// Whether it runs inside [`untraced`].
    untraced: bool,
}

/// An allocator request, aimed at a live block by position where it needs
/// one.
#[derive(Clone, Copy, Debug)]
enum Request {
    /// Allocate a fresh block of `size` bytes.
    Allocate { size: usize },
    /// Reallocate a live block to `size` bytes; skipped when nothing is live.
    Resize { block: Index, size: usize },
    /// Free a live block; skipped when nothing is live.
    Free { block: Index },
}

/// Generates requests of a few kilobytes at most under every label, inside
/// and outside [`untraced`].
fn arb_op() -> impl Strategy<Value = Op> {
    let size = 1..=4_096usize;
    let request = prop_oneof![
        size.clone().prop_map(|size| Request::Allocate { size }),
        (any::<Index>(), size).prop_map(|(block, size)| Request::Resize { block, size }),
        any::<Index>().prop_map(|block| Request::Free { block }),
    ];
    (request, 0..LABELS, any::<bool>()).prop_map(|(request, label, untraced)| Op {
        request,
        label: Label(label),
        untraced,
    })
}

/// Issues `ops` against the allocator, keeping live blocks in `blocks`, and
/// appends to `expected` the events a trace records for them.
///
/// `blocks` and `expected` must have spare capacity for every block and
/// event, so that no request but the ops' own reaches the allocator.
fn apply(blocks: &mut Vec<Block>, expected: &mut Vec<Event>, ops: &[Op]) {
    for op in ops {
        assert!(
            expected.len() + 2 <= expected.capacity(),
            "expected never reallocates"
        );
        let mut issue = || match op.request {
            Request::Allocate { size } => {
                assert!(blocks.len() < blocks.capacity(), "blocks never reallocates");
                let block = Block::new(size, 1, false);
                if !op.untraced {
                    expected.push(event(EventKind::Allocated, op.label, &block));
                }
                blocks.push(block);
            }
            Request::Resize { block, size } => {
                if blocks.is_empty() {
                    return;
                }
                let index = block.index(blocks.len());
                let block = &mut blocks[index];
                let old = event(EventKind::Freed, op.label, block);
                block.resize(size);
                if op.untraced {
                    expected.push(old);
                } else {
                    expected.push(Event {
                        kind: EventKind::Resized,
                        ..old
                    });
                    expected.push(event(EventKind::Allocated, op.label, block));
                }
            }
            Request::Free { block } => {
                if blocks.is_empty() {
                    return;
                }
                let block = blocks.swap_remove(block.index(blocks.len()));
                expected.push(event(EventKind::Freed, op.label, &block));
                drop(block);
            }
        };
        labelled(op.label, || {
            if op.untraced {
                untraced(issue);
            } else {
                issue();
            }
        });
    }
}

/// The event a request of `kind` under `label` records for `block` as it
/// stands.
fn event(kind: EventKind, label: Label, block: &Block) -> Event {
    Event {
        kind,
        label,
        address: block.ptr.as_ptr().addr(),
        size: block.layout.size(),
    }
}

proptest! {
    /// A trace records exactly the events each request implies, in request
    /// order, each under the label current at the request.
    ///
    /// An allocation records `Allocated`, a free `Freed`, and a reallocation
    /// `Resized` for the old block and then `Allocated` for the new one.
    /// Inside `untraced`, allocations are omitted, and a reallocation records
    /// only its old block's free, as a plain `Freed`.
    ///
    /// Blocks allocated before the trace may be resized or freed inside it,
    /// so the trace also records frees of blocks it never saw allocated.
    #[test]
    fn traces_record_every_request(
        before in vec(arb_op(), 0..16),
        during in vec(arb_op(), 0..48),
    ) {
        let mut blocks = Vec::with_capacity(before.len() + during.len());
        let mut discarded = Vec::with_capacity(2 * before.len() + 2);
        apply(&mut blocks, &mut discarded, &before);
        let mut expected = Vec::with_capacity(2 * during.len() + 2);
        let (trace, ()) = trace(2 * during.len(), || apply(&mut blocks, &mut expected, &during));
        drop(blocks);

        prop_assert_eq!(trace.dropped(), 0);
        prop_assert_eq!(trace.events(), &expected[..]);
    }
}
