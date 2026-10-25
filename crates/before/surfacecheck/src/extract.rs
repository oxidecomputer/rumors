//! The reachability walk: every public item, named as its check names it.
//!
//! The walk starts at the crate's root module and follows exactly what a
//! user of the public API can reach — public modules, `pub use`
//! re-exports (named and glob), public types and their impls, and public
//! traits — recording each item under the path it is *reachable* at
//! (`Party::seed` for a root re-export,
//! `causally::since` inside a public module). The `paths` table's
//! definition paths are deliberately not used for reachable naming: they
//! name private modules (`party::Party`, `version::io`) and include
//! items the public tree never reaches.
//!
//! Three categories come back, each checked against its owning inventory:
//!
//! - **functions** — free functions, inherent methods, and public-trait
//!   -declared methods, reconciled against the board's operation inventory and
//!   the exception lists in [`crate::check`];
//! - **impls** — every reachable trait impl, one row per
//!   `type: impl Trait for For` spelling, reconciled against the pinned
//!   [`crate::census::TRAIT_IMPLS`]. Compiler-synthesized auto-trait
//!   impls are excluded here because `before` pins those guarantees
//!   directly (`src/testing/auto_traits.rs` asserts `Send + Sync + Unpin` on
//!   every public API type at compile time); blanket impls are excluded
//!   because they are foreign library plumbing (`From<T> for T`,
//!   `Borrow`, `Any`, …), not API decisions made in this crate;
//! - **items** — associated consts and types, module-level consts,
//!   statics, and macros, reconciled against the pinned
//!   [`crate::census::ITEMS`];
//! - **`usize` signatures** — every place a public `usize` can appear: a
//!   function-like item's parameters, return type, or own generics, trait-impl
//!   methods included; an impl's header, trait or inherent (its trait's
//!   arguments, implementing type, or generics); a module const's or static's
//!   type; an associated const's or type's type, in impls and traits; a public
//!   field's type, of a struct, a union, or an enum variant's payload; and a
//!   type definition's own generics. A `Self` inside an impl counts as its
//!   implementing type. The rows reconcile against the pinned
//!   [`crate::census::USIZE_SIGNATURES`]. A `usize` means something different
//!   on a 32-bit target, so each one in a public signature or stored quantity
//!   is a reviewed decision.
//!
//! `#[doc(hidden)]` items never appear in the JSON at all, so the ground
//! truth here is the documented public surface.

use std::collections::BTreeSet;

use rustdoc_types::{
    AssocItemConstraintKind, Crate, Function, GenericArg, GenericArgs, GenericBound,
    GenericParamDefKind, Generics, Id, Item, ItemEnum, Path, StructKind, Term, Type, VariantKind,
    WherePredicate,
};

#[cfg(test)]
mod tests;

/// The publicly reachable surface, by category.
///
/// A type reachable at two public paths contributes its methods and
/// impls under both names; the reconcile step then surfaces the extra
/// path as an uncovered item to triage, which is the honest reading
/// (two public spellings are two rows of surface).
#[derive(Debug, Default)]
pub(crate) struct Surface {
    /// Function-like items: `Type::fn`, `module::fn`, `Trait::fn`.
    pub functions: BTreeSet<String>,
    /// Trait impls: `Type: impl trait::Path<Args> for ForType`.
    pub impls: BTreeSet<String>,
    /// Associated consts and types, module consts, statics, and macros,
    /// named by reachable path alone (`Rank::ZERO`,
    /// `testing::laws::VERSION_SOLO`).
    pub items: BTreeSet<String>,
    /// Signatures that mention `usize`, named as `owner: usize in sites`.
    ///
    /// The owner is a `functions` or `items` row; an `impls` row, alone for
    /// its header or followed by `:: method`, `:: const NAME`, or
    /// `:: type NAME`; `Type: inherent impl` for an inherent impl's header;
    /// `Type.field` or `Enum::Variant.field` for a public field, a tuple
    /// field named by its index; or `Type: definition` for a type
    /// definition's generics. The sites are the parameters, `return`,
    /// `generics`, `header`, or `type` that mention `usize`.
    pub usize_signatures: BTreeSet<String>,
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
/// `visited` guards against glob re-export cycles (`pub use m::*` loops),
/// keyed by the path and the module id being expanded; a module
/// re-exported at two paths is walked once per path, because each path
/// names distinct public surface.
///
/// Recurses, through [`walk_named`] and [`walk_use`], once per module level
/// along each reachable path, a depth `before`'s own modules and re-exports
/// fix. That bound holds only while `before` has no named re-export of an
/// enclosing module: `visited` keys on the path, which such a re-export would
/// lengthen on every pass.
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
            let type_name = join(prefix, name.expect("a type has a name"));
            record_usize_definition(&s.generics, &type_name, surface);
            let fields = match &s.kind {
                StructKind::Unit => Vec::new(),
                StructKind::Tuple(fields) => fields.iter().flatten().copied().collect(),
                StructKind::Plain { fields, .. } => fields.clone(),
            };
            record_usize_fields(krate, &fields, &type_name, surface);
            walk_type(krate, name, &s.impls, prefix, surface);
        }
        ItemEnum::Enum(e) => {
            let type_name = join(prefix, name.expect("a type has a name"));
            record_usize_definition(&e.generics, &type_name, surface);
            for &variant in &e.variants {
                let variant = item_of(krate, variant);
                let ItemEnum::Variant(payload) = &variant.inner else {
                    panic!("an enum's variants list must name variants, found {variant:?}");
                };
                let fields = match &payload.kind {
                    VariantKind::Plain => Vec::new(),
                    VariantKind::Tuple(fields) => fields.iter().flatten().copied().collect(),
                    VariantKind::Struct { fields, .. } => fields.clone(),
                };
                let variant_name = variant.name.as_deref().expect("a variant has a name");
                let owner = format!("{type_name}::{variant_name}");
                record_usize_fields(krate, &fields, &owner, surface);
            }
            walk_type(krate, name, &e.impls, prefix, surface);
        }
        ItemEnum::Union(u) => {
            let type_name = join(prefix, name.expect("a type has a name"));
            record_usize_definition(&u.generics, &type_name, surface);
            record_usize_fields(krate, &u.fields, &type_name, surface);
            walk_type(krate, name, &u.impls, prefix, surface);
        }
        ItemEnum::Function(function) => {
            let row = join(prefix, name.expect("a function has a name"));
            record_usize_signature(function, &row, None, surface);
            surface.functions.insert(row);
        }
        ItemEnum::Trait(t) => {
            let trait_name = name.expect("a trait has a name");
            let owner = join(prefix, trait_name);
            for &member in &t.items {
                let member = item_of(krate, member);
                let member_name = member.name.as_deref();
                match &member.inner {
                    ItemEnum::Function(function) => {
                        let fn_name = member_name.expect("a trait method has a name");
                        let row = format!("{owner}::{fn_name}");
                        record_usize_signature(function, &row, None, surface);
                        surface.functions.insert(row);
                    }
                    ItemEnum::AssocConst { .. } | ItemEnum::AssocType { .. } => {
                        let item_name = member_name.expect("a trait member has a name");
                        let row = format!("{owner}::{item_name}");
                        record_usize_associated(&member.inner, &row, None, surface);
                        surface.items.insert(row);
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
        ItemEnum::Constant { type_, .. }
        | ItemEnum::Static(rustdoc_types::Static { type_, .. }) => {
            let row = join(prefix, name.expect("a const or static has a name"));
            if mentions_usize([Node::Type(type_)], None) {
                surface
                    .usize_signatures
                    .insert(format!("{row}: usize in type"));
            }
            surface.items.insert(row);
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
        let self_type = Some(&impl_.for_);
        let mut header = vec![Node::Type(&impl_.for_)];
        header.extend(generics_nodes(&impl_.generics));
        if mentions_usize(header, self_type) {
            surface
                .usize_signatures
                .insert(format!("{type_name}: inherent impl: usize in header"));
        }
        for &member in &impl_.items {
            let member = item_of(krate, member);
            let member_name = member.name.as_deref();
            match &member.inner {
                ItemEnum::Function(function) => {
                    let fn_name = member_name.expect("a method has a name");
                    let row = format!("{type_name}::{fn_name}");
                    record_usize_signature(function, &row, self_type, surface);
                    surface.functions.insert(row);
                }
                ItemEnum::AssocConst { .. } | ItemEnum::AssocType { .. } => {
                    let item_name = member_name.expect("an associated item has a name");
                    let row = format!("{type_name}::{item_name}");
                    record_usize_associated(&member.inner, &row, self_type, surface);
                    surface.items.insert(row);
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
/// `src/testing/auto_traits.rs`, which is where that guarantee is reviewed —
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
    let row = format!(
        "{owner}: impl {} for {}",
        render_trait(krate, trait_),
        render_type(krate, &impl_.for_),
    );
    let self_type = Some(&impl_.for_);
    let mut header = vec![Node::Type(&impl_.for_)];
    header.extend(trait_.args.as_deref().map(Node::Args));
    header.extend(generics_nodes(&impl_.generics));
    if mentions_usize(header, self_type) {
        surface
            .usize_signatures
            .insert(format!("{row}: usize in header"));
    }
    for &member in &impl_.items {
        let member = item_of(krate, member);
        let member_name = member.name.as_deref().expect("an impl member has a name");
        match &member.inner {
            ItemEnum::Function(function) => {
                let row = format!("{row} :: {member_name}");
                record_usize_signature(function, &row, self_type, surface);
            }
            ItemEnum::AssocConst { .. } => {
                let row = format!("{row} :: const {member_name}");
                record_usize_associated(&member.inner, &row, self_type, surface);
            }
            ItemEnum::AssocType { .. } => {
                let row = format!("{row} :: type {member_name}");
                record_usize_associated(&member.inner, &row, self_type, surface);
            }
            _ => {}
        }
    }
    surface.impls.insert(row);
}

/// Record `function`'s `usize` signature row under the function-like row
/// `row`, when any parameter, the return type, or the function's own generic
/// parameters mention `usize`.
///
/// `self_type` is the implementing type when the function lives in an impl,
/// so a `Self` in the signature counts as that type; inside a trait
/// definition `Self` is unknown. The enclosing impl's own generics belong to
/// its header row, which [`record_impl`] and the inherent-impl walk record.
fn record_usize_signature(
    function: &Function,
    row: &str,
    self_type: Option<&Type>,
    surface: &mut Surface,
) {
    let mut sites: Vec<String> = function
        .sig
        .inputs
        .iter()
        .filter(|(_, ty)| mentions_usize([Node::Type(ty)], self_type))
        .map(|(name, _)| format!("parameter {name}"))
        .collect();
    if function
        .sig
        .output
        .as_ref()
        .is_some_and(|ty| mentions_usize([Node::Type(ty)], self_type))
    {
        sites.push("return".to_owned());
    }
    if mentions_usize(generics_nodes(&function.generics), self_type) {
        sites.push("generics".to_owned());
    }
    if !sites.is_empty() {
        surface
            .usize_signatures
            .insert(format!("{row}: usize in {}", sites.join(", ")));
    }
}

/// Record a public field's `usize` row, `owner.field: usize in type`, for
/// each of `fields` whose type mentions `usize`.
///
/// The owner is a struct or union path, or an enum variant's `Enum::Variant`;
/// a tuple field's name is its index. Rustdoc's public JSON strips private
/// fields, so every field listed here is public.
fn record_usize_fields(krate: &Crate, fields: &[Id], owner: &str, surface: &mut Surface) {
    for &field in fields {
        let field = item_of(krate, field);
        let ItemEnum::StructField(type_) = &field.inner else {
            panic!("a fields list must name struct fields, found {field:?}");
        };
        if mentions_usize([Node::Type(type_)], None) {
            let field_name = field.name.as_deref().expect("a field has a name");
            surface
                .usize_signatures
                .insert(format!("{owner}.{field_name}: usize in type"));
        }
    }
}

/// Record a type definition's `usize` row, `Type: definition: usize in
/// generics`, when its own generic parameters or where clauses mention
/// `usize`.
fn record_usize_definition(generics: &Generics, type_name: &str, surface: &mut Surface) {
    if mentions_usize(generics_nodes(generics), None) {
        surface
            .usize_signatures
            .insert(format!("{type_name}: definition: usize in generics"));
    }
}

/// Record an associated const's or type's `usize` row under `row`, when its
/// type, its bounds, or its generics mention `usize`.
fn record_usize_associated(
    member: &ItemEnum,
    row: &str,
    self_type: Option<&Type>,
    surface: &mut Surface,
) {
    let nodes: Vec<Node<'_>> = match member {
        ItemEnum::AssocConst { type_, .. } => vec![Node::Type(type_)],
        ItemEnum::AssocType {
            generics,
            bounds,
            type_,
        } => bounds
            .iter()
            .map(Node::Bound)
            .chain(type_.iter().map(Node::Type))
            .chain(generics_nodes(generics))
            .collect(),
        other => panic!("record_usize_associated takes associated items, found {other:?}"),
    };
    if mentions_usize(nodes, self_type) {
        surface
            .usize_signatures
            .insert(format!("{row}: usize in type"));
    }
}

/// The pieces of a generics list that can contain types: each type
/// parameter's bounds and default, each const parameter's type, and each
/// where clause.
fn generics_nodes(generics: &Generics) -> Vec<Node<'_>> {
    let params = generics.params.iter().flat_map(|param| match &param.kind {
        GenericParamDefKind::Lifetime { .. } => Vec::new(),
        GenericParamDefKind::Type {
            bounds, default, ..
        } => bounds
            .iter()
            .map(Node::Bound)
            .chain(default.iter().map(Node::Type))
            .collect(),
        GenericParamDefKind::Const { type_, .. } => vec![Node::Type(type_)],
    });
    let predicates = generics
        .where_predicates
        .iter()
        .flat_map(|predicate| match predicate {
            WherePredicate::BoundPredicate { type_, bounds, .. } => {
                let mut nodes = vec![Node::Type(type_)];
                nodes.extend(bounds.iter().map(Node::Bound));
                nodes
            }
            WherePredicate::LifetimePredicate { .. } => Vec::new(),
            WherePredicate::EqPredicate { lhs, rhs } => {
                let mut nodes = vec![Node::Type(lhs)];
                if let Term::Type(rhs) = rhs {
                    nodes.push(Node::Type(rhs));
                }
                nodes
            }
        });
    params.chain(predicates).collect()
}

/// A piece of a signature that can contain types.
#[derive(Clone, Copy)]
pub(crate) enum Node<'a> {
    /// A type.
    Type(&'a Type),
    /// A path's generic arguments, associated-item constraints included.
    Args(&'a GenericArgs),
    /// A bound on a generic parameter or an `impl Trait`.
    Bound(&'a GenericBound),
}

/// Whether any of `nodes` mentions the primitive `usize` anywhere inside it,
/// reading `Self` as `self_type` when one is given.
///
/// The traversal keeps its pending pieces on an explicit stack, so a deeply
/// nested type cannot overflow the call stack. An implementing type never
/// mentions `Self`, so resolving it adds no cycle.
pub(crate) fn mentions_usize<'a>(
    nodes: impl IntoIterator<Item = Node<'a>>,
    self_type: Option<&'a Type>,
) -> bool {
    let mut pending: Vec<Node<'a>> = nodes.into_iter().collect();
    while let Some(node) = pending.pop() {
        match node {
            Node::Type(ty) => match ty {
                Type::Primitive(name) => {
                    if name == "usize" {
                        return true;
                    }
                }
                Type::ResolvedPath(path) => pending.extend(path.args.as_deref().map(Node::Args)),
                Type::DynTrait(dyn_trait) => pending.extend(
                    dyn_trait
                        .traits
                        .iter()
                        .filter_map(|poly| poly.trait_.args.as_deref().map(Node::Args)),
                ),
                Type::FunctionPointer(pointer) => {
                    let sig = &pointer.sig;
                    pending.extend(sig.inputs.iter().map(|(_, ty)| Node::Type(ty)));
                    pending.extend(sig.output.iter().map(Node::Type));
                }
                Type::Tuple(parts) => pending.extend(parts.iter().map(Node::Type)),
                Type::Slice(inner) => pending.push(Node::Type(inner)),
                Type::Array { type_, .. }
                | Type::Pat { type_, .. }
                | Type::RawPointer { type_, .. }
                | Type::BorrowedRef { type_, .. } => pending.push(Node::Type(type_)),
                Type::ImplTrait(bounds) => pending.extend(bounds.iter().map(Node::Bound)),
                Type::QualifiedPath {
                    args,
                    self_type,
                    trait_,
                    ..
                } => {
                    pending.push(Node::Type(self_type));
                    pending.extend(args.as_deref().map(Node::Args));
                    pending.extend(
                        trait_
                            .iter()
                            .filter_map(|path| path.args.as_deref().map(Node::Args)),
                    );
                }
                Type::Generic(name) => {
                    if name == "Self" {
                        pending.extend(self_type.map(Node::Type));
                    }
                }
                Type::Infer => {}
            },
            Node::Args(args) => match args {
                GenericArgs::AngleBracketed { args, constraints } => {
                    pending.extend(args.iter().filter_map(|arg| match arg {
                        GenericArg::Type(ty) => Some(Node::Type(ty)),
                        GenericArg::Lifetime(_) | GenericArg::Const(_) | GenericArg::Infer => None,
                    }));
                    for constraint in constraints {
                        pending.extend(constraint.args.as_deref().map(Node::Args));
                        match &constraint.binding {
                            AssocItemConstraintKind::Equality(Term::Type(ty)) => {
                                pending.push(Node::Type(ty));
                            }
                            AssocItemConstraintKind::Equality(Term::Constant(_)) => {}
                            AssocItemConstraintKind::Constraint(bounds) => {
                                pending.extend(bounds.iter().map(Node::Bound));
                            }
                        }
                    }
                }
                GenericArgs::Parenthesized { inputs, output } => {
                    pending.extend(inputs.iter().map(Node::Type));
                    pending.extend(output.iter().map(Node::Type));
                }
                GenericArgs::ReturnTypeNotation => {}
            },
            Node::Bound(bound) => {
                if let GenericBound::TraitBound { trait_, .. } = bound {
                    pending.extend(trait_.args.as_deref().map(Node::Args));
                }
            }
        }
    }
    false
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
/// bare names keep the pinned census legible. A type shape this crate's
/// surface never uses panics rather than rendering approximately — a
/// new shape must be named here before it can be pinned.
///
/// Recurses, with [`render_args`], once per level of type nesting, a depth
/// `before`'s own signatures fix.
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

/// The public name of an item at `prefix`: path segments joined
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
