//! The tutorial build script's flattening of the kv example into one program,
//! and its rendering of that program as doctest fragments.
//!
//! The build script compiles the same module, `build/tutorial.rs`; the
//! generated fragments' own doctests check that every prefix of the real
//! example compiles.

#[path = "../build/tutorial.rs"]
mod tutorial;

use std::fs;

use proptest::prelude::*;

use tutorial::{Chunk, ROOT};

/// Chunks named `names`, with the matching sources.
fn chunks(sources: &[(&str, &str)]) -> Vec<Chunk> {
    sources
        .iter()
        .map(|(name, source)| Chunk {
            name: (*name).to_owned(),
            source: (*source).to_owned(),
        })
        .collect()
}

/// Flatten `sources`, failing the test with the flattener's reason if it
/// rejects them.
fn flatten(sources: &[(&str, &str)]) -> Vec<String> {
    tutorial::flatten(&chunks(sources)).unwrap_or_else(|error| panic!("{error}"))
}

/// Flattening deletes module declarations, imports that stay inside the
/// crate, visibility on items, fields, and methods, and module docs, and
/// keeps every other byte, comments and formatting included.
#[test]
fn flattening_deletes_module_structure_and_keeps_the_rest() {
    let flattened = flatten(&[
        (
            "shape",
            "//! Shapes.\n\nuse std::fmt;\n\npub(crate) struct Shape {\n    pub(super) sides: u8, // counted\n}\n\nmod area;\n",
        ),
        (
            "shape/area",
            "use super::Shape;\nuse crate::shape::Shape as Same;\n\nimpl Shape {\n    pub(crate) fn area(&self)  ->  u8 {\n        self.sides\n    }\n}\n",
        ),
    ]);
    assert_eq!(
        flattened,
        [
            "use std::fmt;\n\nstruct Shape {\n    sides: u8, // counted\n}",
            "impl Shape {\n    fn area(&self)  ->  u8 {\n        self.sides\n    }\n}",
        ]
    );
}

/// An import of a name an earlier chunk imported from the same path is
/// dropped, and a group that repeats only some names keeps the new ones.
#[test]
fn a_repeated_import_is_dropped_and_a_partial_repeat_keeps_its_new_names() {
    let flattened = flatten(&[
        (
            "a",
            "use std::fs;\nuse tokio::io::{AsyncRead, AsyncWrite};\n\nstruct A;\n",
        ),
        (
            "b",
            "use std::fs::{self, File};\nuse tokio::io::{AsyncWrite, AsyncWriteExt};\nuse tokio::io::AsyncRead;\n\nstruct B;\n",
        ),
    ]);
    assert_eq!(
        flattened[1],
        "use std::fs::File;\nuse tokio::io::AsyncWriteExt;\n\nstruct B;"
    );
}

/// A module imported as `self` beside a name an earlier chunk imported keeps
/// just the module, as a plain import of it.
#[test]
fn a_module_imported_as_self_survives_its_repeated_sibling() {
    let flattened = flatten(&[
        ("a", "use axum::extract::State;\n\nstruct A;\n"),
        ("b", "use axum::extract::{self, State};\n\nstruct B;\n"),
    ]);
    assert_eq!(flattened[1], "use axum::extract;\n\nstruct B;");
}

/// One name bound to two different things anywhere in the program is
/// rejected, whether both are imports or one is an item.
#[test]
fn two_meanings_for_one_name_are_rejected() {
    let imports = tutorial::flatten(&chunks(&[
        ("a", "use std::path::Path;\n"),
        ("b", "use axum::extract::Path;\n"),
    ]));
    assert!(matches!(imports, Err(error) if error.contains("`Path`")));

    let item = tutorial::flatten(&chunks(&[
        ("a", "struct Path;\n"),
        ("b", "use std::path::Path;\n"),
    ]));
    assert!(matches!(item, Err(error) if error.contains("`Path`")));
}

/// A path that reaches into the crate outside an import is rejected, written
/// directly, inside a macro's arguments, or through one of the example's
/// modules, since flattening deletes the imports such paths rely on.
#[test]
fn a_path_into_the_crate_outside_an_import_is_rejected() {
    for body in [
        "fn f() -> crate::a::A { todo!() }",
        "fn f() { let _ = vec![super::A]; }",
        "fn f() -> shape::Shape { todo!() }",
    ] {
        let source = format!("mod shape;\n\n{body}\n");
        let result = tutorial::flatten(&chunks(&[("a", &source), ("a/shape", "struct Shape;\n")]));
        assert!(
            matches!(result, Err(error) if error.contains("inside the crate")),
            "{body}"
        );
    }
}

/// A glob import is rejected, since the names it binds cannot be
/// deduplicated.
#[test]
fn a_glob_import_is_rejected() {
    let result = tutorial::flatten(&chunks(&[("a", "use std::io::*;\n")]));
    assert!(matches!(result, Err(error) if error.contains("glob")));
}

/// The chunks of an example follow each file's module declarations, with a
/// file's own items before its modules when they precede the declarations,
/// after them when they follow, and absent when the file has none.
#[test]
fn chunks_follow_declarations_with_own_items_where_they_stand() {
    let dir = tempfile::tempdir().unwrap();
    let write = |path: &str, source: &str| {
        let path = dir.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    };
    write(
        "main.rs",
        "mod first;\nmod second;\nmod third;\n\nfn main() {}\n",
    );
    write("first.rs", "struct First;\n\nmod a;\nmod b;\n");
    write("first/a.rs", "struct A;\n");
    write("first/b.rs", "struct B;\n");
    write("second.rs", "mod c;\n\nuse c::C;\n\nfn second(_: C) {}\n");
    write("second/c.rs", "pub(super) struct C;\n");
    write("third.rs", "mod d;\n");
    write("third/d.rs", "struct D;\n");

    let names: Vec<String> = tutorial::chunks(dir.path())
        .unwrap()
        .into_iter()
        .map(|chunk| chunk.name)
        .collect();
    assert_eq!(
        names,
        [
            "first", "first/a", "first/b", "second/c", "second", "third/d", ROOT
        ]
    );
}

/// A file whose own items stand on both sides of its module declarations is
/// rejected, since its chunk would have no single place in the order.
#[test]
fn own_items_on_both_sides_of_module_declarations_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "struct Before;\n\nmod a;\n\nfn main() {}\n",
    )
    .unwrap();
    fs::write(dir.path().join("a.rs"), "struct A;\n").unwrap();
    let result = tutorial::chunks(dir.path());
    assert!(matches!(result, Err(error) if error.contains("together")));
}

/// Every fragment but the last allows dead code and ends with a hidden
/// `main`, and the last shows its chunk with neither.
#[test]
fn only_prefix_fragments_allow_dead_code_and_stub_main() {
    let flattened = ["struct A;".to_owned(), "fn main() {}".to_owned()];
    let prefix = tutorial::fragment(&flattened, 0).unwrap();
    assert_eq!(
        prefix,
        "\n```no_run\n# #![allow(dead_code)]\nstruct A;\n# fn main() {}\n```\n"
    );
    let last = tutorial::fragment(&flattened, 1).unwrap();
    assert_eq!(last, "\n```no_run\n# struct A;\n#\nfn main() {}\n```\n");
}

/// A chunk line that rustdoc would hide or unescape is rejected, since it
/// would vanish from the fragment the reader sees.
#[test]
fn a_chunk_line_rustdoc_would_hide_is_rejected() {
    for line in ["#", "  # hidden", "## unescaped"] {
        let result = tutorial::fragment(&[line.to_owned()], 0);
        assert!(result.is_err(), "{line:?}");
    }
}

/// The tutorial passes the order check exactly when it references every
/// chunk once, in teaching order, and a failure names the first difference.
#[test]
fn the_tutorial_references_every_chunk_once_in_order() {
    let expected: Vec<String> = ["a", "a/b", ROOT].map(str::to_owned).to_vec();
    let tutorial = |names: &[&str]| -> String {
        names
            .iter()
            .map(|name| format!("#![doc = tutorial!(\"{name}\")]\n"))
            .collect::<String>()
            + "//! Prose between references is ignored.\n"
    };
    let check = |names: &[&str]| {
        let found = tutorial::references(&tutorial(names)).unwrap();
        tutorial::check_order(&expected, &found)
    };

    assert!(check(&["a", "a/b", ROOT]).is_ok());
    for wrong in [
        &["a", ROOT][..],
        &["a/b", "a", ROOT],
        &["a", "a/b", ROOT, ROOT],
        &["a", "a/b", "a/c", ROOT],
    ] {
        assert!(
            matches!(check(wrong), Err(error) if error.contains("position")),
            "{wrong:?}"
        );
    }
}

/// The manifest's fragment shows the template byte for byte in a TOML block,
/// which rustdoc renders without compiling.
#[test]
fn the_manifest_fragment_shows_the_template_verbatim() {
    let template = "[package]\nname = \"kv\"\n\n[dependencies]\nrumors = \"0.1.0\"\n";
    assert_eq!(
        tutorial::manifest_fragment(template),
        format!("\n```toml\n{template}```\n")
    );
}

/// Lines a chunk may hold: anything rustdoc shows as written, which a line
/// without `#` always is.
fn shown_line() -> impl Strategy<Value = String> {
    "[a-z ;{}()]{0,12}"
}

proptest! {
    /// Removing a fragment's hidden markers, its `allow` line, and its `main`
    /// stub reproduces every chunk up to and including its own, separated by
    /// blank lines, inside the code fence.
    #[test]
    fn a_fragment_unhidden_is_the_program_up_to_its_chunk(
        chunk_lines in prop::collection::vec(prop::collection::vec(shown_line(), 1..4), 1..5),
        pick in any::<prop::sample::Index>(),
    ) {
        let flattened: Vec<String> = chunk_lines.iter().map(|lines| lines.join("\n")).collect();
        let index = pick.index(flattened.len());
        let fragment = tutorial::fragment(&flattened, index).unwrap();

        let body: Vec<&str> = fragment
            .strip_prefix("\n```no_run\n").unwrap()
            .strip_suffix("```\n").unwrap()
            .lines()
            .filter(|line| *line != "# #![allow(dead_code)]" && *line != "# fn main() {}")
            .map(|line| line.strip_prefix("# ").unwrap_or(if line == "#" { "" } else { line }))
            .collect();
        let expected: Vec<&str> = flattened[..=index]
            .iter()
            .enumerate()
            .flat_map(|(at, chunk)| {
                let separator = (at < index).then_some("");
                chunk.lines().chain(separator)
            })
            .collect();
        prop_assert_eq!(body, expected);
    }
}
