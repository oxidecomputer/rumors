//! Checks for the `usize` scan over rustdoc's type grammar, and for the
//! reachability traversal over module graphs with cycles.

use std::collections::{BTreeSet, HashMap};

use rustdoc_types::{
    Abi, AssocItemConstraint, AssocItemConstraintKind, Crate, Function, FunctionHeader,
    FunctionSignature, GenericArg, GenericArgs, GenericBound, Generics, Id, Item, ItemEnum, Module,
    Path, Target, Term, TraitBoundModifier, Type, Use, Visibility,
};

use super::{Node, mentions_usize, public_surface};

/// A primitive type by name.
fn primitive(name: &str) -> Type {
    Type::Primitive(name.to_owned())
}

/// A path with angle-bracketed type arguments and associated-type
/// equalities.
fn path(name: &str, args: Vec<Type>, equalities: Vec<(&str, Type)>) -> Path {
    Path {
        path: name.to_owned(),
        id: Id(0),
        args: Some(Box::new(GenericArgs::AngleBracketed {
            args: args.into_iter().map(GenericArg::Type).collect(),
            constraints: equalities
                .into_iter()
                .map(|(name, ty)| AssocItemConstraint {
                    name: name.to_owned(),
                    args: None,
                    binding: AssocItemConstraintKind::Equality(Term::Type(ty)),
                })
                .collect(),
        })),
    }
}

/// Whether one type mentions `usize`.
fn mentions(ty: &Type) -> bool {
    mentions_usize([Node::Type(ty)], None)
}

/// The scan finds `usize` bare, as a generic argument, inside a tuple, behind
/// a reference to a slice, and as an `impl Trait`'s associated type, and finds
/// nothing in the same positions holding `u64`.
#[test]
fn usize_scan_reaches_each_nested_position() {
    let positions = |leaf: &str| {
        let option = Type::ResolvedPath(path("Option", vec![primitive(leaf)], vec![]));
        vec![
            primitive(leaf),
            option.clone(),
            Type::Tuple(vec![primitive("u8"), option]),
            Type::BorrowedRef {
                lifetime: None,
                is_mutable: false,
                type_: Box::new(Type::Slice(Box::new(primitive(leaf)))),
            },
            Type::ImplTrait(vec![GenericBound::TraitBound {
                trait_: path("Iterator", vec![], vec![("Item", primitive(leaf))]),
                generic_params: Vec::new(),
                modifier: TraitBoundModifier::None,
            }]),
        ]
    };
    for (index, ty) in positions("usize").iter().enumerate() {
        assert!(mentions(ty), "position {index} holds usize: {ty:?}");
    }
    for (index, ty) in positions("u64").iter().enumerate() {
        assert!(!mentions(ty), "position {index} holds only u64: {ty:?}");
    }
}

/// Inside an impl, `Self` stands for the implementing type, so a `Self` in a
/// signature mentions `usize` exactly when the implementing type does; with
/// no implementing type known, it mentions nothing.
#[test]
fn self_reads_as_the_implementing_type() {
    let result_of_self = Type::ResolvedPath(path(
        "Result",
        vec![Type::Generic("Self".to_owned()), primitive("u8")],
        vec![],
    ));
    let usize = primitive("usize");
    let u64 = primitive("u64");
    assert!(mentions_usize([Node::Type(&result_of_self)], Some(&usize)));
    assert!(!mentions_usize([Node::Type(&result_of_self)], Some(&u64)));
    assert!(!mentions_usize([Node::Type(&result_of_self)], None));
}

/// A public item with the given id, name, and payload.
fn item(id: u32, name: Option<&str>, inner: ItemEnum) -> Item {
    Item {
        id: Id(id),
        crate_id: 0,
        name: name.map(str::to_owned),
        span: None,
        visibility: Visibility::Public,
        docs: None,
        links: HashMap::new(),
        attrs: Vec::new(),
        deprecation: None,
        stability: None,
        const_stability: None,
        inner,
    }
}

/// A module holding the items with the given ids.
fn module(id: u32, name: Option<&str>, items: &[u32]) -> Item {
    let inner = ItemEnum::Module(Module {
        is_crate: name.is_none(),
        items: items.iter().copied().map(Id).collect(),
        is_stripped: false,
    });
    item(id, name, inner)
}

/// A re-export of the item `target`, named `name`, or a glob of it.
fn reexport(id: u32, name: &str, target: u32, is_glob: bool) -> Item {
    let inner = ItemEnum::Use(Use {
        source: name.to_owned(),
        name: name.to_owned(),
        id: Some(Id(target)),
        is_glob,
    });
    item(id, Some(name), inner)
}

/// A function with no parameters and no result.
fn function(id: u32, name: &str) -> Item {
    let inner = ItemEnum::Function(Function {
        sig: FunctionSignature {
            inputs: Vec::new(),
            output: None,
            is_c_variadic: false,
        },
        generics: Generics {
            params: Vec::new(),
            where_predicates: Vec::new(),
        },
        header: FunctionHeader {
            is_const: false,
            is_unsafe: false,
            is_async: false,
            abi: Abi::Rust,
        },
        has_body: true,
    });
    item(id, Some(name), inner)
}

/// The traversal terminates on a module graph with cycles, and records every
/// spelling whose path passes through no module twice.
///
/// Module `a` re-exports itself by name as `again`, which lengthens the path
/// on every lap, and its child `c` splices `a` back in with
/// `pub use super::*`; modules `g` and `h` splice each other in by glob. The
/// root also re-exports `a` as `alias`, a second path to `a` that passes
/// through it once, so `a`'s functions are rows at both paths, and at no
/// cyclic spelling.
#[test]
fn walk_terminates_on_module_cycles() {
    let items = [
        module(0, None, &[1, 4, 5, 7]),
        module(1, Some("a"), &[2, 3, 11]),
        function(2, "f"),
        reexport(3, "again", 1, false),
        reexport(4, "alias", 1, false),
        module(5, Some("g"), &[6, 9]),
        reexport(6, "h", 7, true),
        module(7, Some("h"), &[8, 10]),
        reexport(8, "g", 5, true),
        function(9, "in_g"),
        function(10, "in_h"),
        module(11, Some("c"), &[12, 13]),
        reexport(12, "super", 1, true),
        function(13, "in_c"),
    ];
    let krate = Crate {
        root: Id(0),
        crate_version: None,
        includes_private: false,
        index: items.into_iter().map(|item| (item.id, item)).collect(),
        paths: HashMap::new(),
        external_crates: HashMap::new(),
        target: Target {
            triple: String::new(),
            target_features: Vec::new(),
        },
        format_version: 0,
    };
    let expected: BTreeSet<String> = [
        "a::f",
        "a::c::in_c",
        "alias::f",
        "alias::c::in_c",
        "g::in_g",
        "g::in_h",
        "h::in_g",
        "h::in_h",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(public_surface(&krate).functions, expected);
}
