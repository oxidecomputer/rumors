//! Scoped leaf addresses for wire fixtures with chosen tree geometry.

use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

use super::Path;
use crate::Version;

/// A fixture's complete assignment of canonical versions to leaf addresses.
type Paths = BTreeMap<Vec<u8>, Path>;

thread_local! {
    /// The active fixture on this thread; ordinary tests use the version hash.
    static PATHS: RefCell<Option<Paths>> = const { RefCell::new(None) };
}

/// Restore the enclosing fixture when a scope returns or unwinds.
struct Restore(Option<Paths>);

impl Drop for Restore {
    /// Reinstate the previous mapping, including after a failed assertion.
    fn drop(&mut self) {
        PATHS.with(|paths| paths.replace(self.0.take()));
    }
}

impl Path {
    /// Run a fixture with a distinct address assigned to each leaf version.
    ///
    /// Build, use, and drop its trees inside `run`, and drive both wire
    /// endpoints on this thread. The decoder uses this same mapping to
    /// recover each received leaf's address. An unlisted version panics.
    /// Nested calls restore the enclosing mapping even if `run` panics.
    pub(crate) fn with_leaf_paths<R>(
        paths: impl IntoIterator<Item = (Version, Self)>,
        run: impl FnOnce() -> R,
    ) -> R {
        let mut versions = Paths::new();
        let mut addresses = BTreeSet::new();
        for (version, path) in paths {
            assert!(
                addresses.insert(path),
                "fixture leaf paths must be distinct"
            );
            assert!(
                versions.insert(version.as_bytes().to_vec(), path).is_none(),
                "fixture leaf versions must be distinct"
            );
        }
        let _restore = Restore(PATHS.with(|paths| paths.replace(Some(versions))));
        run()
    }
}

/// Resolve a fixture leaf, or use normal hashing when no fixture is active.
pub(super) fn get(version: &Version) -> Option<Path> {
    PATHS.with(|paths| {
        paths.borrow().as_ref().map(|paths| {
            *paths
                .get(version.as_bytes())
                .expect("leaf version is missing from the active fixture")
        })
    })
}
