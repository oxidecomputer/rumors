//! Generic `Insert`/`Redact` action sequences for single-`Local`
//! tests. Shared by `pairwise` (async wire) and `sync_wire`, so both
//! exercise the same shapes of input — including redactions, which
//! the original String-T and Sync wire tests skipped.

use std::sync::{Arc, Mutex};

use borsh::{BorshDeserialize, BorshSerialize};
use proptest::collection::vec;
use proptest::prelude::*;
use rumors::Key;
use rumors::sync::Local;

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

/// Apply a `LocalAction` sequence to a fresh `Local<T>` tagged with
/// `party`, returning the result.
pub fn build_local<T>(party: &str, actions: &[LocalAction<T>]) -> Local<T>
where
    T: Send + Sync + Clone + BorshSerialize + BorshDeserialize + 'static,
{
    let mut local = Local::for_party(party, 0).unwrap();
    // `Arc<Mutex<_>>` rather than `&mut keys`: the sync API's callback
    // bound is `Send + 'static`, which a borrow can't satisfy.
    let keys: Arc<Mutex<Vec<Key>>> = Arc::new(Mutex::new(Vec::new()));
    for a in actions {
        match a {
            LocalAction::Insert(v) => {
                let keys_in = Arc::clone(&keys);
                local.message([v.clone()], move |k, _, _| keys_in.lock().unwrap().push(k));
            }
            LocalAction::Redact(idx) => {
                let keys_guard = keys.lock().unwrap();
                if !keys_guard.is_empty() {
                    let k = keys_guard[idx % keys_guard.len()];
                    drop(keys_guard);
                    local.redact([k]);
                }
            }
        }
    }
    local.fork()
}
