//! Generic `Insert`/`Redact` action sequences for single-peer tests.
//! Shared by `pairwise`, `async_wire`, and `sync_wire`, so all three
//! exercise the same shapes of input — including redactions. [`build_local`]
//! applies a sequence to a synchronous `sync::Rumors`; [`build_local_async`]
//! applies it to an asynchronous `rumors::Rumors`. (Sends are synchronous on
//! both surfaces now — the names refer to which API surface is built, not to
//! the function's color.)

use borsh::{BorshDeserialize, BorshSerialize};
use proptest::collection::vec;
use proptest::prelude::*;
use rumors::{Key, Snapshot, Version, causally};

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

/// The `Key` of the single live leaf in `snapshot` above the causal frontier
/// `pre`: how a builder recovers the key a `send` just minted, given the
/// `latest()` it recorded before sending. Panics unless exactly one leaf
/// qualifies.
pub fn minted_key<T: Send + Sync>(snapshot: &Snapshot<T>, pre: &Version) -> Key {
    let mut fresh = snapshot.range(causally::since(pre)).map(|(k, _, _)| k);
    let key = fresh.next().expect("a send mints exactly one live leaf");
    assert!(
        fresh.next().is_none(),
        "a single send must mint exactly one live leaf"
    );
    key
}

/// Apply a `LocalAction` sequence to the given `sync::Rumors<T>`, returning
/// it.
///
/// The caller supplies `local` already bootstrapped from the shared universe
/// seed, so independently-built locals stay pairwise disjoint and can later
/// reconcile over the wire.
pub fn build_local<T>(
    local: rumors::sync::Rumors<T>,
    actions: &[LocalAction<T>],
) -> rumors::sync::Rumors<T>
where
    T: Send + Sync + Clone + BorshSerialize + BorshDeserialize + 'static,
{
    let mut keys: Vec<Key> = Vec::new();
    for a in actions {
        match a {
            LocalAction::Insert(v) => {
                let pre = local.snapshot().latest().clone();
                local.send(v.clone());
                keys.push(minted_key(&local.snapshot(), &pre));
            }
            LocalAction::Redact(idx) => {
                if !keys.is_empty() {
                    local.redact(keys[idx % keys.len()]);
                }
            }
        }
    }
    local
}

/// Counterpart of [`build_local`] on the asynchronous [`rumors::Rumors`]: the
/// same `LocalAction` replay for the tests that exercise the genuinely
/// concurrent wire protocol. As in [`build_local`], `local` must already be
/// bootstrapped from the shared universe seed.
pub fn build_local_async<T>(
    local: rumors::Rumors<T>,
    actions: &[LocalAction<T>],
) -> rumors::Rumors<T>
where
    T: Send + Sync + Clone + BorshSerialize + BorshDeserialize + 'static,
{
    let mut keys: Vec<Key> = Vec::new();
    for a in actions {
        match a {
            LocalAction::Insert(v) => {
                let pre = local.snapshot().latest().clone();
                local.send(v.clone());
                keys.push(minted_key(&local.snapshot(), &pre));
            }
            LocalAction::Redact(idx) => {
                if !keys.is_empty() {
                    local.redact(keys[idx % keys.len()]);
                }
            }
        }
    }
    local
}
