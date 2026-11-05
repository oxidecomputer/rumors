//! Tests of trace recording against the allocator's real requests, and of
//! the lifetime classification against a model heap.
//!
//! Recording and classification are tested apart. The recording test issues
//! raw allocator requests and predicts each event from the blocks it holds,
//! so it checks what the allocator records and nothing about what the
//! events mean. The classification tests build event sequences from a model
//! heap whose every block's fate is known, so they check the analysis
//! against an independent computation of each label's live transient bytes.

use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::Index;

use super::{
    Event, EventKind, Incomplete, Label, LabelLifetimes, Lifetimes, Trace, labelled, trace,
    untraced,
};
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

/// A block of the model heap.
#[derive(Clone, Copy, Debug)]
struct ModelBlock {
    /// The block's address, unique among live blocks.
    address: usize,
    /// The block's size.
    size: usize,
    /// The label it was allocated under.
    label: Label,
    /// Whether the trace saw it allocated: not from before the trace, and
    /// not allocated inside [`untraced`].
    traced: bool,
    /// The step that ends its lifetime, or `None` if it survives the trace.
    ends: Option<usize>,
}

/// One step of a model history, aimed at live blocks and free addresses by
/// position.
#[derive(Clone, Copy, Debug)]
enum Step {
    /// Allocate a block at a free address.
    Allocate {
        address: Index,
        size: usize,
        label: Label,
        untraced: bool,
    },
    /// Resize a live block, in place or to a free address.
    Resize {
        block: Index,
        in_place: bool,
        address: Index,
        size: usize,
        label: Label,
        untraced: bool,
    },
    /// Free a live block.
    Free { block: Index },
}

/// Addresses the model heap places blocks at: few, so that addresses recur
/// and frees must be matched to the right allocation.
const ADDRESSES: usize = 6;

/// Generates model steps under every label, traced and untraced.
fn arb_step() -> impl Strategy<Value = Step> {
    let size = 1..=64usize;
    let label = (0..LABELS).prop_map(Label);
    prop_oneof![
        (any::<Index>(), size.clone(), label.clone(), any::<bool>()).prop_map(
            |(address, size, label, untraced)| Step::Allocate {
                address,
                size,
                label,
                untraced
            }
        ),
        (
            any::<Index>(),
            any::<bool>(),
            any::<Index>(),
            size,
            label,
            any::<bool>()
        )
            .prop_map(
                |(block, in_place, address, size, label, untraced)| Step::Resize {
                    block,
                    in_place,
                    address,
                    size,
                    label,
                    untraced
                }
            ),
        any::<Index>().prop_map(|block| Step::Free { block }),
    ]
}

/// The model heap's history: every block it ever held, and the events a
/// trace would record for each step.
struct History {
    /// Every block, in allocation order, with its fate filled in.
    blocks: Vec<ModelBlock>,
    /// For each step, the blocks live after it, by index into `blocks`.
    live_after: Vec<Vec<usize>>,
    /// For each step that resizes, the old block, live during that step.
    resized: Vec<Option<usize>>,
    /// The recorded events.
    events: Vec<Event>,
}

/// Runs `steps` on a heap that starts with `preexisting` untraced blocks of
/// the given sizes, recording the trace's events and each block's fate.
fn history(preexisting: &[usize], steps: &[Step]) -> History {
    let mut blocks: Vec<ModelBlock> = Vec::new();
    let mut live: Vec<usize> = Vec::new();
    for (address, &size) in preexisting.iter().enumerate().take(ADDRESSES) {
        live.push(blocks.len());
        blocks.push(ModelBlock {
            address,
            size,
            label: Label::NONE,
            traced: false,
            ends: None,
        });
    }
    let mut history = History {
        blocks: Vec::new(),
        live_after: Vec::new(),
        resized: Vec::new(),
        events: Vec::new(),
    };
    let free_address = |blocks: &[ModelBlock], live: &[usize], pick: Index| {
        let free: Vec<usize> = (0..ADDRESSES)
            .filter(|address| !live.iter().any(|&b| blocks[b].address == *address))
            .collect();
        (!free.is_empty()).then(|| free[pick.index(free.len())])
    };
    for (step_index, step) in steps.iter().enumerate() {
        let mut resized = None;
        match *step {
            Step::Allocate {
                address,
                size,
                label,
                untraced,
            } => {
                if let Some(address) = free_address(&blocks, &live, address) {
                    if !untraced {
                        history.events.push(Event {
                            kind: EventKind::Allocated,
                            label,
                            address,
                            size,
                        });
                    }
                    live.push(blocks.len());
                    blocks.push(ModelBlock {
                        address,
                        size,
                        label,
                        traced: !untraced,
                        ends: None,
                    });
                }
            }
            Step::Resize {
                block,
                in_place,
                address,
                size,
                label,
                untraced,
            } => {
                if !live.is_empty() {
                    let position = block.index(live.len());
                    let old = live[position];
                    let address = if in_place {
                        Some(blocks[old].address)
                    } else {
                        free_address(&blocks, &live, address)
                    };
                    if let Some(address) = address {
                        let freed = Event {
                            kind: EventKind::Freed,
                            label,
                            address: blocks[old].address,
                            size: blocks[old].size,
                        };
                        if untraced {
                            history.events.push(freed);
                        } else {
                            history.events.push(Event {
                                kind: EventKind::Resized,
                                ..freed
                            });
                            history.events.push(Event {
                                kind: EventKind::Allocated,
                                label,
                                address,
                                size,
                            });
                        }
                        blocks[old].ends = Some(step_index);
                        resized = Some(old);
                        live[position] = blocks.len();
                        blocks.push(ModelBlock {
                            address,
                            size,
                            label,
                            traced: !untraced,
                            ends: None,
                        });
                    }
                }
            }
            Step::Free { block } => {
                if !live.is_empty() {
                    let old = live.swap_remove(block.index(live.len()));
                    history.events.push(Event {
                        kind: EventKind::Freed,
                        label: Label::NONE,
                        address: blocks[old].address,
                        size: blocks[old].size,
                    });
                    blocks[old].ends = Some(step_index);
                }
            }
        }
        history.live_after.push(live.clone());
        history.resized.push(resized);
    }
    history.blocks = blocks;
    history
}

/// Computes each label's classified blocks from the model's known fates.
///
/// A traced block is transient when some step ends it. Each step's level is
/// recomputed from the whole live list rather than tracked as a running
/// total: the transient blocks live after the step, plus, for a resize, the
/// old block it replaced, which the step held together with the new one.
fn expected_lifetimes(history: &History) -> Lifetimes {
    let transient = |block: &ModelBlock| block.traced && block.ends.is_some();
    let mut expected = Lifetimes::default();
    for block in history.blocks.iter().filter(|block| block.traced) {
        let entry = expected.labels.entry(block.label).or_default();
        if transient(block) {
            entry.transient_blocks += 1;
            entry.transient_bytes += block.size;
        } else {
            entry.surviving_blocks += 1;
            entry.surviving_bytes += block.size;
        }
    }
    for (live, resized) in history.live_after.iter().zip(&history.resized) {
        for label in (0..LABELS).map(Label) {
            let level: usize = live
                .iter()
                .chain(resized)
                .map(|&b| &history.blocks[b])
                .filter(|block| transient(block) && block.label == label)
                .map(|block| block.size)
                .sum();
            if let Some(entry) = expected.labels.get_mut(&label) {
                entry.transient_peak = entry.transient_peak.max(level);
            }
        }
    }
    expected
}

proptest! {
    /// The classification and each label's transient peak agree with a model
    /// heap whose every block's fate is known.
    ///
    /// A block allocated in the trace and later freed or resized away is
    /// transient, one still live at the end survives, and blocks from before
    /// the trace or allocated untraced count toward neither. Each label's
    /// transient peak is the highest sum of its live transient blocks after
    /// any step, counting a resize's old and new blocks together. `survivors`
    /// lists exactly the surviving blocks, in allocation order.
    ///
    /// Six addresses serve the whole history, so addresses recur, resizes
    /// happen in place, and every free must be matched to the most recent
    /// allocation at its address.
    #[test]
    fn lifetimes_follow_block_fates(
        preexisting in vec(1..=64usize, 0..=3),
        steps in vec(arb_step(), 0..64),
    ) {
        let history = history(&preexisting, &steps);
        prop_assert_eq!(Lifetimes::of(&history.events), expected_lifetimes(&history));

        let expected_survivors: Vec<(usize, usize, Label)> = history
            .blocks
            .iter()
            .filter(|block| block.traced && block.ends.is_none())
            .map(|block| (block.address, block.size, block.label))
            .collect();
        let trace = Trace { events: history.events, dropped: 0 };
        let survivors: Vec<(usize, usize, Label)> = trace
            .survivors()
            .expect("a trace that dropped nothing is complete")
            .iter()
            .map(|event| (event.address, event.size, event.label))
            .collect();
        prop_assert_eq!(survivors, expected_survivors);
    }
}

proptest! {
    /// Classifying under a grouping of labels agrees with the model heap
    /// whose blocks carry the grouped labels from the start.
    ///
    /// Each group's transient peak is the highest sum of the live transient
    /// blocks of every label in it, which a model that never saw the
    /// original labels computes independently. The grouping here merges
    /// labels by parity, so one group holds two labels and the other one.
    #[test]
    fn grouped_lifetimes_follow_grouped_block_fates(
        preexisting in vec(1..=64usize, 0..=3),
        steps in vec(arb_step(), 0..64),
    ) {
        let group = |label: Label| Label(label.0 % 2);
        let history = history(&preexisting, &steps);
        let trace = Trace { events: history.events.clone(), dropped: 0 };
        let mut grouped = history;
        for block in &mut grouped.blocks {
            block.label = group(block.label);
        }
        for event in &mut grouped.events {
            event.label = group(event.label);
        }
        prop_assert_eq!(
            trace.lifetimes_grouped(group).expect("a trace that dropped nothing is complete"),
            expected_lifetimes(&grouped)
        );
    }
}

/// When an address is allocated again while the trace still holds a live
/// block there, the earlier block's lifetime ends at the new allocation,
/// after the new block is counted, and the reuse is counted.
///
/// That sequence arises when another thread frees the earlier block, which
/// this thread's trace cannot see.
#[test]
fn reuse_of_a_live_address_ends_the_earlier_block() {
    let at = |kind, size| Event {
        kind,
        label: Label::NONE,
        address: 0x1000,
        size,
    };
    let events = [
        at(EventKind::Allocated, 8),
        at(EventKind::Allocated, 16),
        at(EventKind::Freed, 16),
    ];
    let lifetimes = Lifetimes::of(&events);
    assert_eq!(lifetimes.reused_while_live, 1);
    assert_eq!(
        lifetimes.label(Label::NONE),
        LabelLifetimes {
            transient_peak: 24,
            transient_blocks: 2,
            transient_bytes: 24,
            surviving_blocks: 0,
            surviving_bytes: 0,
        }
    );
}

/// A trace whose buffer fills counts every event it had no room for, keeps
/// the events it recorded, and refuses to classify.
#[test]
fn a_full_trace_counts_what_it_drops_and_refuses_to_classify() {
    let (trace, blocks) = trace(2, || {
        [
            Block::new(8, 1, false),
            Block::new(16, 1, false),
            Block::new(32, 1, false),
        ]
    });
    drop(blocks);

    assert_eq!(trace.events().len(), 2);
    assert_eq!(trace.dropped(), 1);
    assert_eq!(
        trace.lifetimes(),
        Err(Incomplete {
            recorded: 2,
            dropped: 1,
        })
    );
}
