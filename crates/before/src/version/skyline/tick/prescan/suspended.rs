//! Compact storage for outer minimum-recording levels.

use suanpan::Accumulator;

use crate::codec::{BitStack, PopStack};

use super::super::frames::Position;
use super::super::StoredAccumulator;

/// One outer recording level parked while a deeper level is active.
pub(super) struct Level {
    /// First nested minimum minus the suspended outer reference.
    pub(super) first_from_outer: StoredAccumulator,
    /// Latest recorded minimum minus the first at the outer level.
    pub(super) latest_from_first: StoredAccumulator,
    /// The outer level's deferred first memo slot, when it has one.
    pub(super) first_slot: Option<usize>,
    /// The nesting level served by the outer relation.
    pub(super) level: u64,
}

/// A LIFO stack of retained accumulator values.
///
/// Small and wide values have separate vectors. A tag selects the matching
/// vector on pop, and the shared LIFO order keeps them aligned.
struct AccumulatorStack {
    /// Whether each retained value is wide.
    wide: BitStack,
    /// Machine-sized values in insertion order.
    small: Vec<i64>,
    /// Wide values in insertion order. Existing boxes move between owners
    /// without reallocating the wide accumulator.
    #[allow(clippy::vec_box)]
    large: Vec<Box<Accumulator>>,
}

impl AccumulatorStack {
    /// Construct an empty value stack.
    fn new() -> Self {
        Self {
            wide: BitStack::new(),
            small: Vec::new(),
            large: Vec::new(),
        }
    }

    /// Push one retained value.
    fn push(&mut self, value: StoredAccumulator) {
        match value {
            StoredAccumulator::Small(value) => {
                self.wide.push(false);
                self.small.push(value);
            }
            StoredAccumulator::Wide(value) => {
                self.wide.push(true);
                self.large.push(value);
            }
        }
    }

    /// Pop the newest retained value, or `None` if the stack is empty.
    fn pop(&mut self) -> Option<StoredAccumulator> {
        let wide = self.wide.pop()?;
        Some(if wide {
            StoredAccumulator::Wide(self.large.pop().expect("a wide tag has a wide value"))
        } else {
            StoredAccumulator::Small(self.small.pop().expect("a small tag has a small value"))
        })
    }

    /// Whether the stack is empty.
    fn is_empty(&self) -> bool {
        self.wide.len() == 0
    }
}

/// Suspended recording levels, stored in compact parallel stacks.
pub(super) struct Levels {
    /// Suspended differences from the outer recording references.
    first_from_outer: AccumulatorStack,
    /// Suspended differences between each level's latest and first minima.
    latest_from_first: AccumulatorStack,
    /// Whether each suspended level has a deferred first memo entry.
    has_first_slot: BitStack,
    /// Differences between deferred first memo slots.
    first_slot_deltas: PopStack,
    /// Most recently suspended first memo slot.
    first_slots: Position,
    /// Differences between parked nesting levels.
    level_deltas: PopStack,
    /// Most recently parked nesting level.
    levels: Position,
}

impl Levels {
    /// Construct an empty stack.
    pub(super) fn new() -> Self {
        Self {
            first_from_outer: AccumulatorStack::new(),
            latest_from_first: AccumulatorStack::new(),
            has_first_slot: BitStack::new(),
            first_slot_deltas: PopStack::new(),
            first_slots: Position::new(),
            level_deltas: PopStack::new(),
            levels: Position::new(),
        }
    }

    /// Whether no outer recording level is parked.
    pub(super) fn is_empty(&self) -> bool {
        self.first_from_outer.is_empty()
    }

    /// Park one outer recording level.
    pub(super) fn push(&mut self, level: Level) {
        self.first_from_outer.push(level.first_from_outer);
        self.latest_from_first.push(level.latest_from_first);
        self.has_first_slot.push(level.first_slot.is_some());
        if let Some(slot) = level.first_slot {
            self.first_slots.push(
                &mut self.first_slot_deltas,
                u64::try_from(slot).expect("memo slots fit u64"),
            );
        }
        self.levels.push(&mut self.level_deltas, level.level);
    }

    /// Restore the innermost parked recording level.
    pub(super) fn pop(&mut self) -> Option<Level> {
        let first_from_outer = self.first_from_outer.pop()?;
        let latest_from_first = self
            .latest_from_first
            .pop()
            .expect("a suspended level carries both minimum differences");
        let first_slot = if self
            .has_first_slot
            .pop()
            .expect("a suspended level carries a slot tag")
        {
            Some(
                usize::try_from(self.first_slots.pop(&mut self.first_slot_deltas))
                    .expect("a stored memo slot came from usize"),
            )
        } else {
            None
        };
        Some(Level {
            first_from_outer,
            latest_from_first,
            first_slot,
            level: self.levels.pop(&mut self.level_deltas),
        })
    }
}
