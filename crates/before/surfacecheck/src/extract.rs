//! The reachability walk: every public function-like item, named as the
//! roster names it.
//!
//! The walk starts at the crate's root module and follows exactly what a
//! user of the public API can reach — public modules, `pub use`
//! re-exports (named and glob), public types and their inherent impls,
//! and public traits — recording each function-like item under the path
//! it is *reachable* at, which is the roster's own naming (`Party::seed`
//! for a root re-export, `causally::Range::since` inside a public
//! module). The `paths` table's definition paths are deliberately not
//! used: they name private modules (`party::Party`, `version::skyline`)
//! and include items the public tree never reaches.
//!
//! Out of scope, matching the roster's denomination: trait-*impl*
//! methods (`FAMILY_SURFACE`'s review-governed domain), associated
//! consts and types, statics, and macros. `#[doc(hidden)]` items never
//! appear in the JSON at all, so the ground truth here is the documented
//! public surface.

use std::collections::BTreeSet;

use rustdoc_types::{Crate, Id, Item, ItemEnum};

/// Walk the publicly reachable item tree and collect every function-like
/// item's roster-style name.
///
/// A type reachable at two public paths contributes its methods under
/// both names; the reconcile step then surfaces the extra path as an
/// unrostered finding to triage, which is the honest reading (two public
/// spellings are two rows of surface).
pub(crate) fn function_like_items(krate: &Crate) -> BTreeSet<String> {
    let root = module_of(krate, krate.root);
    let mut names = BTreeSet::new();
    let mut visited_modules = BTreeSet::new();
    walk_module(krate, &root.items, &[], &mut names, &mut visited_modules);
    names
}

/// Look up an id in the index, panicking with the id on a dangling
/// reference: the walk must never silently skip surface it was pointed
/// at.
fn item_of(krate: &Crate, id: Id) -> &Item {
    krate
        .index
        .get(&id)
        .unwrap_or_else(|| panic!("rustdoc JSON index has no item for reachable id {id:?}"))
}

/// The module payload of an item known to be a module.
fn module_of(krate: &Crate, id: Id) -> &rustdoc_types::Module {
    match &item_of(krate, id).inner {
        ItemEnum::Module(module) => module,
        other => panic!("expected a module at {id:?}, found {other:?}"),
    }
}

/// Record one module's items under `prefix`, following re-exports.
///
/// `visited` guards against re-export cycles (`pub use` loops), keyed by
/// the module id being expanded; a module re-exported at two paths is
/// walked once per path, because each path names distinct public
/// surface.
fn walk_module(
    krate: &Crate,
    items: &[Id],
    prefix: &[&str],
    names: &mut BTreeSet<String>,
    visited: &mut BTreeSet<(Vec<String>, Id)>,
) {
    for &id in items {
        let item = item_of(krate, id);
        walk_named(krate, item, prefix, names, visited);
    }
}

/// Record one reachable item under the name it is reachable at.
///
/// `item.name` is the declared name; a renaming re-export reaches its
/// target through [`walk_use`], which substitutes the `use` item's name.
fn walk_named(
    krate: &Crate,
    item: &Item,
    prefix: &[&str],
    names: &mut BTreeSet<String>,
    visited: &mut BTreeSet<(Vec<String>, Id)>,
) {
    let name = item.name.as_deref();
    match &item.inner {
        ItemEnum::Module(module) => {
            let name = name.expect("a non-root module has a name");
            let key = (path_key(prefix, name), item.id);
            if visited.insert(key) {
                let prefix = push(prefix, name);
                walk_module(krate, &module.items, &prefix, names, visited);
            }
        }
        ItemEnum::Use(use_) => walk_use(krate, use_, prefix, names, visited),
        ItemEnum::Struct(s) => {
            walk_type(krate, name, &s.impls, prefix, names);
        }
        ItemEnum::Enum(e) => {
            walk_type(krate, name, &e.impls, prefix, names);
        }
        ItemEnum::Union(u) => {
            walk_type(krate, name, &u.impls, prefix, names);
        }
        ItemEnum::Function(_) => {
            names.insert(join(prefix, name.expect("a function has a name")));
        }
        ItemEnum::Trait(t) => {
            let trait_name = name.expect("a trait has a name");
            for &member in &t.items {
                let member = item_of(krate, member);
                if let ItemEnum::Function(_) = member.inner {
                    let fn_name = member.name.as_deref().expect("a trait method has a name");
                    let owner = join(prefix, trait_name);
                    names.insert(format!("{owner}::{fn_name}"));
                }
            }
        }
        // Not function-like: consts, statics, type aliases, macros,
        // impls at module scope (reached through their types instead),
        // and the rest of the item grammar.
        _ => {}
    }
}

/// Follow a `pub use`: a named re-export surfaces its target under the
/// `use` item's (possibly renaming) name; a glob re-export splices the
/// target module's items into the current prefix.
fn walk_use(
    krate: &Crate,
    use_: &rustdoc_types::Use,
    prefix: &[&str],
    names: &mut BTreeSet<String>,
    visited: &mut BTreeSet<(Vec<String>, Id)>,
) {
    // `id` is `None` only for primitive re-exports; an id pointing
    // outside the index is another crate's item, which is not `before`
    // surface.
    let Some(id) = use_.id else { return };
    let Some(target) = krate.index.get(&id) else {
        return;
    };
    if use_.is_glob {
        // A glob over an enum re-exports variants, which are not
        // function-like; only a module glob splices surface in.
        if let ItemEnum::Module(module) = &target.inner {
            let key = (path_key(prefix, "*"), id);
            if visited.insert(key) {
                walk_module(krate, &module.items, prefix, names, visited);
            }
        }
        return;
    }
    // A named re-export: the target appears here under `use_.name`.
    let renamed = Item {
        name: Some(use_.name.clone()),
        ..target.clone()
    };
    walk_named(krate, &renamed, prefix, names, visited);
}

/// Record a public type's inherent methods as `Type::fn` rows.
///
/// Trait impls (`trait_` present) are skipped: operators, conversions,
/// and codec traits are `FAMILY_SURFACE`'s domain, rostered by family
/// and reviewed, never name-matched here.
fn walk_type(
    krate: &Crate,
    name: Option<&str>,
    impls: &[Id],
    prefix: &[&str],
    names: &mut BTreeSet<String>,
) {
    let type_name = join(prefix, name.expect("a type has a name"));
    for &impl_id in impls {
        let impl_item = item_of(krate, impl_id);
        let ItemEnum::Impl(impl_) = &impl_item.inner else {
            panic!("a type's impls list must name impl items, found {impl_item:?}");
        };
        if impl_.trait_.is_some() {
            continue;
        }
        for &member in &impl_.items {
            let member = item_of(krate, member);
            if let ItemEnum::Function(_) = member.inner {
                let fn_name = member.name.as_deref().expect("a method has a name");
                names.insert(format!("{type_name}::{fn_name}"));
            }
        }
    }
}

/// A module path extended by one segment.
fn push<'p>(prefix: &[&'p str], name: &'p str) -> Vec<&'p str> {
    let mut out = prefix.to_vec();
    out.push(name);
    out
}

/// The roster-style name of an item at `prefix`: path segments joined
/// with `::`, the crate root contributing nothing.
fn join(prefix: &[&str], name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{}::{name}", prefix.join("::"))
    }
}

/// An owned visited-set key for a module expansion at a path.
fn path_key(prefix: &[&str], name: &str) -> Vec<String> {
    prefix
        .iter()
        .map(|s| (*s).to_owned())
        .chain([name.to_owned()])
        .collect()
}
