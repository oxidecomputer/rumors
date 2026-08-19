//! Generic `Insert`/`Redact` action sequences shared by reconciliation tests.

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::{Snapshot, Version, causally};

use serde::Serialize;
use serde::de::DeserializeOwned;
const MAX_ACTIONS: usize = 16;

#[derive(Debug, Clone)]
pub enum LocalAction<T> {
    Insert(T),
    Redact(usize),
}

/// Strategy over `Vec<LocalAction<T>>`, weighted 4:1 toward inserts.
/// `value_strategy` supplies the value type; a `Redact(idx)` picks
/// `versions[idx % len]` at build time (or is dropped if nothing has
/// been sent yet).
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

/// Returns the [`Version`] of the single live leaf in `snapshot` above
/// the causal frontier `pre`.
///
/// This is how a builder recovers the version a `send` just minted,
/// given the `latest()` it recorded before sending.
///
/// # Panics
///
/// Panics unless exactly one leaf qualifies.
pub fn minted_version<T: Send + Sync>(snapshot: &Snapshot<T>, pre: &Version) -> Version {
    let mut fresh = snapshot.range(causally::since(pre)).map(|(v, _)| v);
    let version = fresh.next().expect("a send mints exactly one live leaf");
    assert!(
        fresh.next().is_none(),
        "a single send must mint exactly one live leaf"
    );
    version.clone()
}

/// Apply a `LocalAction` sequence to an already-bootstrapped local replica.
pub fn build_local<T>(local: rumors::Rumors<T>, actions: &[LocalAction<T>]) -> rumors::Rumors<T>
where
    T: Send + Sync + Clone + Serialize + DeserializeOwned + 'static,
{
    let mut versions: Vec<Version> = Vec::new();
    for a in actions {
        match a {
            LocalAction::Insert(v) => {
                let pre = local.snapshot().latest().clone();
                local.send(v.clone());
                versions.push(minted_version(&local.snapshot(), &pre));
            }
            LocalAction::Redact(idx) => {
                if !versions.is_empty() {
                    local.redact(&versions[idx % versions.len()]);
                }
            }
        }
    }
    local
}
