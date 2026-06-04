//! Generic `Insert`/`Redact` action sequences for single-`Known`
//! tests. Shared by `pairwise` (async wire) and `sync_wire`, so both
//! exercise the same shapes of input — including redactions, which
//! the original String-T and Sync wire tests skipped.

use borsh::{BorshDeserialize, BorshSerialize};
use proptest::collection::vec;
use proptest::prelude::*;
use rumors::Key;
use rumors::sync::Known;

const MAX_ACTIONS: usize = 16;

#[derive(Debug, Clone)]
pub enum LocalAction<T> {
    Insert(T),
    Redact(usize),
}

/// Strategy over `Vec<LocalAction<T>>`, weighted 4:1 toward inserts.
/// `value_strategy` supplies the value type; a `Redact(idx)` picks
/// `keys[idx % len]` at build time (or is dropped if no keys yet).
pub fn arb_actions<T, S>(value_strategy: S) -> impl Strategy<Value = Vec<LocalAction<T>>>
where
    T: Clone + std::fmt::Debug + 'static,
    S: Strategy<Value = T> + Clone + 'static,
{
    vec(
        prop_oneof![
            4 => value_strategy.prop_map(LocalAction::Insert),
            1 => any::<usize>().prop_map(LocalAction::Redact),
        ],
        0..=MAX_ACTIONS,
    )
}

/// `u64`-valued action strategy: the default for tests that don't
/// care about the value type.
pub fn arb_local_actions() -> impl Strategy<Value = Vec<LocalAction<u64>>> {
    arb_actions(any::<u64>())
}

/// `String`-valued action strategy: bounded lowercase ASCII for
/// human-readable shrinking output.
pub fn arb_string_actions() -> impl Strategy<Value = Vec<LocalAction<String>>> {
    arb_actions("[a-z]{0,8}".prop_map(String::from))
}

/// Apply a `LocalAction` sequence to the given `Known<T>`, returning it.
///
/// The caller supplies `local` already [`fork`](Known::fork)ed from the shared
/// universe seed, so independently-built locals stay pairwise disjoint and can
/// later [`learn`](Known::learn) from one another.
pub fn build_local<T>(mut local: Known<T>, actions: &[LocalAction<T>]) -> Known<T>
where
    T: Send + Sync + Clone + BorshSerialize + BorshDeserialize + 'static,
{
    // The sync callback bound only requires `Send + 'a`, so the closure can
    // borrow `keys` directly for the duration of each `message` call.
    let mut keys: Vec<Key> = Vec::new();
    for a in actions {
        match a {
            LocalAction::Insert(v) => {
                local.message_then([v.clone()], |k, _, _| keys.push(k));
            }
            LocalAction::Redact(idx) => {
                if !keys.is_empty() {
                    local.redact([keys[idx % keys.len()]]);
                }
            }
        }
    }
    local
}
