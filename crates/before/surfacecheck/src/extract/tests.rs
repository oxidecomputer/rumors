//! Checks for the `usize` scan over rustdoc's type grammar.

use rustdoc_types::{
    AssocItemConstraint, AssocItemConstraintKind, GenericArg, GenericArgs, GenericBound, Id, Path,
    Term, TraitBoundModifier, Type,
};

use super::{Node, mentions_usize};

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
