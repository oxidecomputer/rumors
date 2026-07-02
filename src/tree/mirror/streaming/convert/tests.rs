//! Boundary tests: conversion between a material backend and an immaterial
//! one, the crossing a wire transport performs implicitly.

use proptest::prelude::*;

use crate::tree::arb::arb_root_node;
use crate::tree::typed::{Prefix, height};

use super::super::backend::{Backend, Flat, Local, Material};
use super::subtree;

proptest! {
    /// A subtree survives the crossing to an immaterial backend and back.
    ///
    /// Exploding a `Local` tree into `Flat`'s bare leaf sequence discards
    /// every intermediate hash and version bound, yet reassembling the
    /// sequence in `Local` reproduces the original Merkle root exactly —
    /// the leaves alone determine the tree.
    #[test]
    fn roundtrips_through_immaterial(root in arb_root_node(0, 1..=8)) {
        let node = root.expect("at least one leaf makes a root");
        let flat = pollster::block_on(subtree::<Local, Flat, (), height::Root>(
            &Local,
            &Flat,
            Prefix::new(),
            node.clone(),
        ))
        .expect("both backends are infallible");
        let back = pollster::block_on(subtree::<Flat, Local, (), height::Root>(
            &Flat,
            &Local,
            Prefix::new(),
            flat,
        ))
        .expect("both backends are infallible");
        prop_assert_eq!(back.hash(), node.hash());
    }
}

/// The design boundary, pinned at compile time: `Flat` implements the
/// weakened `Backend`, but not the session's materiality requirement.
///
/// Uncommenting the last line fails with E0271 (expected `Material`,
/// found `Immaterial`).
#[test]
fn flat_is_a_backend_but_not_material() {
    fn backend<B: Backend<()>>() {}
    #[allow(dead_code)]
    fn material<B: Backend<(), Materialized = Material>>() {}
    backend::<Flat>();
    material::<Local>();
    // material::<Flat>(); // E0271: expected `Material`, found `Immaterial`
}
