//! The reachability walk: every public item, named as its check names it.
//!
//! The walk starts at the crate's root module and follows exactly what a
//! user of the public API can reach — public modules, `pub use`
//! re-exports (named and glob), public types and their impls, and public
//! traits — recording each item under the path it is *reachable* at,
//! which is the roster's own naming (`Party::seed` for a root re-export,
//! `causally::Range::since` inside a public module). The `paths` table's
//! definition paths are deliberately not used for reachable naming: they
//! name private modules (`party::Party`, `version::skyline`) and include
//! items the public tree never reaches.
//!
//! Three categories come back, each held total by its own committed
//! roster:
//!
//! - **functions** — free functions, inherent methods, and public-trait
//!   -declared methods, reconciled against `METHOD_SURFACE` and the
//!   exception lists in [`crate::check`];
//! - **impls** — every reachable trait impl, one row per
//!   `type: impl Trait for For` spelling, reconciled against the pinned
//!   [`crate::census::TRAIT_IMPLS`]. Compiler-synthesized auto-trait
//!   impls are excluded here because `before` pins those guarantees
//!   directly (`src/auto_traits.rs` asserts `Send + Sync + Unpin` on
//!   every public API type at compile time); blanket impls are excluded
//!   because they are foreign library plumbing (`From<T> for T`,
//!   `Borrow`, `Any`, …), not API decisions made in this crate;
//! - **items** — associated consts and types, module-level consts,
//!   statics, and macros, reconciled against the pinned
//!   [`crate::census::ITEMS`].
//!
//! `#[doc(hidden)]` items never appear in the JSON at all, so the ground
//! truth here is the documented public surface.

use std::collections::BTreeSet;

use rustdoc_types::{Crate, GenericArg, GenericArgs, Id, Item, ItemEnum, Path, Type};

/// The publicly reachable surface, by category.
///
/// A type reachable at two public paths contributes its methods and
/// impls under both names; the reconcile step then surfaces the extra
/// path as an unrostered finding to triage, which is the honest reading
/// (two public spellings are two rows of surface).
#[derive(Debug, Default)]
pub(crate) struct Surface {
    /// Function-like items: `Type::fn`, `module::fn`, `Trait::fn`.
    pub functions: BTreeSet<String>,
    /// Trait impls: `Type: impl trait::Path<Args> for ForType`.
    pub impls: BTreeSet<String>,
    /// Associated consts and types, module consts, statics, and macros,
    /// named by reachable path alone (`Rank::ZERO`, `laws::VERSION_SOLO`).
    pub items: BTreeSet<String>,
}

/// Walk the publicly reachable item tree and collect every public item
/// into its [`Surface`] category.
pub(crate) fn public_surface(krate: &Crate) -> Surface {
    let root = module_of(krate, krate.root);
    let mut surface = Surface::default();
    let mut visited_modules = BTreeSet::new();
    walk_module(krate, &root.items, &[], &mut surface, &mut visited_modules);
    surface
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
    surface: &mut Surface,
    visited: &mut BTreeSet<(Vec<String>, Id)>,
) {
    for &id in items {
        let item = item_of(krate, id);
        walk_named(krate, item, prefix, surface, visited);
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
    surface: &mut Surface,
    visited: &mut BTreeSet<(Vec<String>, Id)>,
) {
    let name = item.name.as_deref();
    match &item.inner {
        ItemEnum::Module(module) => {
            let name = name.expect("a non-root module has a name");
            let key = (path_key(prefix, name), item.id);
            if visited.insert(key) {
                let prefix = push(prefix, name);
                walk_module(krate, &module.items, &prefix, surface, visited);
            }
        }
        ItemEnum::Use(use_) => walk_use(krate, use_, prefix, surface, visited),
        ItemEnum::Struct(s) => {
            walk_type(krate, name, &s.impls, prefix, surface);
        }
        ItemEnum::Enum(e) => {
            walk_type(krate, name, &e.impls, prefix, surface);
        }
        ItemEnum::Union(u) => {
            walk_type(krate, name, &u.impls, prefix, surface);
        }
        ItemEnum::Function(_) => {
            surface
                .functions
                .insert(join(prefix, name.expect("a function has a name")));
        }
        ItemEnum::Trait(t) => {
            let trait_name = name.expect("a trait has a name");
            let owner = join(prefix, trait_name);
            for &member in &t.items {
                let member = item_of(krate, member);
                let member_name = member.name.as_deref();
                match &member.inner {
                    ItemEnum::Function(_) => {
                        let fn_name = member_name.expect("a trait method has a name");
                        surface.functions.insert(format!("{owner}::{fn_name}"));
                    }
                    ItemEnum::AssocConst { .. } | ItemEnum::AssocType { .. } => {
                        let item_name = member_name.expect("a trait member has a name");
                        surface.items.insert(format!("{owner}::{item_name}"));
                    }
                    _ => {}
                }
            }
            // Impls of a public trait for types the type walk never
            // visits (primitives, foreign types) are reachable only
            // from the trait's page; record them under the trait's
            // reachable path.
            for &impl_id in &t.implementations {
                record_impl(krate, impl_id, &owner, surface);
            }
        }
        ItemEnum::Constant { .. } | ItemEnum::Static(_) => {
            surface
                .items
                .insert(join(prefix, name.expect("a const or static has a name")));
        }
        ItemEnum::Macro(_) | ItemEnum::ProcMacro(_) => {
            surface
                .items
                .insert(join(prefix, name.expect("a macro has a name")));
        }
        // Not reachable surface in this crate's grammar: enum variants
        // and struct fields (reached through their types' rows), type
        // aliases (aliases of already-walked types), and the rest of
        // the item grammar.
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
    surface: &mut Surface,
    visited: &mut BTreeSet<(Vec<String>, Id)>,
) {
    // `id` is `None` only for primitive re-exports; an id absent from the
    // index is another crate's item (recorded in the `paths` table), which
    // is not `before` surface. An id in neither table is malformed rustdoc
    // JSON, and a malformed input must never read as a smaller surface —
    // same discipline as `item_of`'s dangling-reference panic.
    let Some(id) = use_.id else { return };
    let Some(target) = krate.index.get(&id) else {
        assert!(
            krate.paths.contains_key(&id),
            "rustdoc JSON has use target {id:?} in neither index nor paths"
        );
        return;
    };
    if use_.is_glob {
        // A glob over an enum re-exports variants, which are not
        // reachable surface on their own; only a module glob splices
        // surface in.
        if let ItemEnum::Module(module) = &target.inner {
            let key = (path_key(prefix, "*"), id);
            if visited.insert(key) {
                walk_module(krate, &module.items, prefix, surface, visited);
            }
        }
        return;
    }
    // A named re-export: the target appears here under `use_.name`.
    let renamed = Item {
        name: Some(use_.name.clone()),
        ..target.clone()
    };
    walk_named(krate, &renamed, prefix, surface, visited);
}

/// Record a public type's impls: inherent methods as `Type::fn` function
/// rows, inherent associated consts and types as `Type::NAME` item rows,
/// and trait impls as census rows.
fn walk_type(
    krate: &Crate,
    name: Option<&str>,
    impls: &[Id],
    prefix: &[&str],
    surface: &mut Surface,
) {
    let type_name = join(prefix, name.expect("a type has a name"));
    for &impl_id in impls {
        let impl_item = item_of(krate, impl_id);
        let ItemEnum::Impl(impl_) = &impl_item.inner else {
            panic!("a type's impls list must name impl items, found {impl_item:?}");
        };
        if impl_.trait_.is_some() {
            record_impl(krate, impl_id, &type_name, surface);
            continue;
        }
        for &member in &impl_.items {
            let member = item_of(krate, member);
            let member_name = member.name.as_deref();
            match &member.inner {
                ItemEnum::Function(_) => {
                    let fn_name = member_name.expect("a method has a name");
                    surface.functions.insert(format!("{type_name}::{fn_name}"));
                }
                ItemEnum::AssocConst { .. } | ItemEnum::AssocType { .. } => {
                    let item_name = member_name.expect("an associated item has a name");
                    surface.items.insert(format!("{type_name}::{item_name}"));
                }
                _ => {}
            }
        }
    }
}

/// Record one trait impl as a census row under the reachable path it was
/// found at (`owner`: the implementing type's path, or the trait's path
/// for impls reached from a trait page).
///
/// Compiler-synthesized auto-trait impls are excluded — `before` pins
/// `Send`/`Sync`/`Unpin` on every public API type at compile time in
/// `src/auto_traits.rs`, which is where that guarantee is reviewed —
/// and blanket impls are excluded as foreign library plumbing, not API
/// decisions of this crate. Everything else, derives included, is a
/// row: deriving a trait is exactly the kind of API event the census
/// exists to make diff-visible.
fn record_impl(krate: &Crate, impl_id: Id, owner: &str, surface: &mut Surface) {
    let impl_item = item_of(krate, impl_id);
    let ItemEnum::Impl(impl_) = &impl_item.inner else {
        panic!("an implementations list must name impl items, found {impl_item:?}");
    };
    if impl_.is_synthetic || impl_.blanket_impl.is_some() {
        return;
    }
    let trait_ = impl_
        .trait_
        .as_ref()
        .expect("record_impl is called on trait impls only");
    surface.impls.insert(format!(
        "{owner}: impl {} for {}",
        render_trait(krate, trait_),
        render_type(krate, &impl_.for_),
    ));
}

/// Render a trait reference for a census row: the definition path from
/// the `paths` table (unambiguous across same-named traits), plus any
/// generic arguments.
fn render_trait(krate: &Crate, trait_: &Path) -> String {
    let mut out = krate
        .paths
        .get(&trait_.id)
        .map(|summary| summary.path.join("::"))
        .unwrap_or_else(|| trait_.path.clone());
    if let Some(args) = &trait_.args {
        out.push_str(&render_args(krate, args));
    }
    out
}

/// Render a type for a census row.
///
/// Named types render as their bare name (the last segment of the
/// definition path): census rows are keyed under the reachable path of
/// the type they were found at, which carries the disambiguation, and
/// bare names keep the pinned roster legible. A type shape this crate's
/// surface never uses panics rather than rendering approximately — a
/// new shape must be named here before it can be pinned.
fn render_type(krate: &Crate, ty: &Type) -> String {
    match ty {
        Type::ResolvedPath(path) => {
            let mut out = krate
                .paths
                .get(&path.id)
                .and_then(|summary| summary.path.last().cloned())
                .unwrap_or_else(|| {
                    path.path
                        .rsplit("::")
                        .next()
                        .expect("rsplit yields at least one segment")
                        .to_owned()
                });
            if let Some(args) = &path.args {
                out.push_str(&render_args(krate, args));
            }
            out
        }
        Type::Generic(name) => name.clone(),
        Type::Primitive(name) => name.clone(),
        Type::BorrowedRef {
            is_mutable, type_, ..
        } => {
            format!(
                "&{}{}",
                if *is_mutable { "mut " } else { "" },
                render_type(krate, type_)
            )
        }
        Type::Tuple(parts) => {
            let parts: Vec<String> = parts.iter().map(|p| render_type(krate, p)).collect();
            format!("({})", parts.join(", "))
        }
        Type::Slice(inner) => format!("[{}]", render_type(krate, inner)),
        Type::Array { type_, len } => format!("[{}; {len}]", render_type(krate, type_)),
        other => panic!("surfacecheck: unhandled type shape in an impl census row: {other:?}"),
    }
}

/// Render generic arguments for a census row: types and consts, with
/// lifetimes elided (they never distinguish two impls) and associated
/// bindings elided (they are constraints, not identity).
fn render_args(krate: &Crate, args: &GenericArgs) -> String {
    let GenericArgs::AngleBracketed { args, .. } = args else {
        panic!("surfacecheck: unhandled generic-args shape in an impl census row: {args:?}");
    };
    let rendered: Vec<String> = args
        .iter()
        .filter_map(|arg| match arg {
            GenericArg::Type(ty) => Some(render_type(krate, ty)),
            GenericArg::Const(constant) => Some(constant.expr.clone()),
            GenericArg::Lifetime(_) => None,
            GenericArg::Infer => Some("_".to_owned()),
        })
        .collect();
    if rendered.is_empty() {
        String::new()
    } else {
        format!("<{}>", rendered.join(", "))
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
