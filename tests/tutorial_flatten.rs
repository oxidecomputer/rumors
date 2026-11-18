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

use tutorial::{ALLOW_DEAD_CODE, Chunk, Flattened, ROOT};

/// A chunk named `name` with the source `source`, a test chunk if `test`.
fn chunk(name: &str, source: &str, test: bool) -> Chunk {
    Chunk {
        name: name.to_owned(),
        source: source.to_owned(),
        test,
    }
}

/// Chunks named `names`, with the matching sources, none of them test chunks.
fn chunks(sources: &[(&str, &str)]) -> Vec<Chunk> {
    sources
        .iter()
        .map(|(name, source)| chunk(name, source, false))
        .collect()
}

/// Each chunk's text after flattening `sources`, failing the test with the
/// flattener's reason if it rejects them.
fn flatten(sources: &[(&str, &str)]) -> Vec<String> {
    tutorial::flatten(&chunks(sources))
        .unwrap_or_else(|error| panic!("{error}"))
        .into_iter()
        .map(|chunk| chunk.text)
        .collect()
}

/// A flattened chunk other than a test chunk, whose text is `text`.
fn untested(text: &str) -> Flattened {
    Flattened {
        text: text.to_owned(),
        test_copy: None,
    }
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

/// Flattening deletes every `#[rustfmt::skip]`, at any depth: its whole line
/// when it stands alone, and the attribute and the spaces after it when it
/// shares a line with code. Every other attribute stays.
#[test]
fn flattening_deletes_rustfmt_skip_and_keeps_other_attributes() {
    let flattened = flatten(&[(
        ROOT,
        "#[rustfmt::skip]\n#[derive(Debug)]\nstruct A {\n    #[rustfmt::skip] x: u8,\n}\n\nimpl A {\n    #[rustfmt::skip]\n    fn f() {}\n}\n\n#[rustfmt::skip::macros(m)]\nfn main() {}\n",
    )]);
    assert_eq!(
        flattened,
        [
            "#[derive(Debug)]\nstruct A {\n    x: u8,\n}\n\nimpl A {\n    fn f() {}\n}\n\n#[rustfmt::skip::macros(m)]\nfn main() {}"
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

/// A test chunk's text shows its tests as written, whatever form their test
/// attributes take, without the imports flattening deletes; its test copy is
/// that text with each test attribute replaced by `#[allow(dead_code)]`.
#[test]
fn a_test_chunk_shows_its_tests_and_its_copy_replaces_each_attribute() {
    for (attribute, copied) in [
        ("#[test]\n", format!("{ALLOW_DEAD_CODE}\n")),
        (
            "#[tokio::test(flavor = \"multi_thread\")]\n",
            format!("{ALLOW_DEAD_CODE}\n"),
        ),
        ("#[test] ", format!("{ALLOW_DEAD_CODE} ")),
    ] {
        let tests = |attribute: &str| {
            format!("{attribute}fn one() {{\n    a();\n}}\n\n{attribute}async fn two() {{}}")
        };
        let source = format!("use std::fs;\nuse super::a;\n\n{}\n", tests(attribute));
        let flattened = tutorial::flatten(&[
            chunk("a", "use std::fs;\n\nfn a() {}\n", false),
            chunk("a/tests", &source, true),
        ])
        .unwrap();
        assert_eq!(flattened[0].test_copy, None, "{attribute:?}");
        assert_eq!(flattened[1].text, tests(attribute), "{attribute:?}");
        assert_eq!(
            flattened[1].test_copy,
            Some(tests(&copied)),
            "{attribute:?}"
        );
    }
}

/// A test chunk is rejected if it holds anything but tests and the imports
/// flattening deletes, which the one-file program would put into the reader's
/// non-test build: an import of a new name, or an item that is not a test.
#[test]
fn a_test_chunk_holding_anything_but_tests_is_rejected() {
    for source in [
        "use std::path::Path;\n\n#[test]\nfn checks() {}\n",
        "fn helper() {}\n\n#[test]\nfn checks() {}\n",
        "struct Fixture;\n",
    ] {
        let result = tutorial::flatten(&[chunk("tests", source, true)]);
        assert!(
            matches!(&result, Err(error) if error.contains("test chunk")),
            "{source:?}"
        );
    }
}

/// A test outside a test chunk is rejected, since the line count would count
/// it as part of the program.
#[test]
fn a_test_outside_a_test_chunk_is_rejected() {
    let result = tutorial::flatten(&chunks(&[(ROOT, "#[test]\nfn checks() {}\n")]));
    assert!(matches!(result, Err(error) if error.contains("`#[cfg(test)]`")));
}

/// `#[cfg(test)]` is rejected anywhere but alone on a module declaration: on
/// an import, a test, any other item, a method, or the whole file, and beside
/// another attribute on a declaration.
#[test]
fn cfg_test_anywhere_but_alone_on_a_module_declaration_is_rejected() {
    for source in [
        "#[cfg(test)]\nuse std::time::Duration;\n",
        "#[cfg(test)]\n#[test]\nfn checks() {}\n",
        "#[cfg(test)]\nfn helper() {}\n",
        "struct A;\n\nimpl A {\n    #[cfg(test)]\n    fn helper() {}\n}\n",
        "#![cfg(test)]\n\nstruct A;\n",
        "#[cfg(test)]\n#[path = \"other.rs\"]\nmod tests;\n",
    ] {
        let result = tutorial::flatten(&chunks(&[("a", source)]));
        assert!(
            matches!(&result, Err(error) if error.contains("cfg(test)")),
            "{source:?}"
        );
    }
}

/// Every chunk below a module declared with `#[cfg(test)]` is a test chunk,
/// the module's own and its descendants', and every other chunk is not.
#[test]
fn a_cfg_test_module_and_its_descendants_hold_test_chunks() {
    let dir = tempfile::tempdir().unwrap();
    let write = |path: &str, source: &str| {
        let path = dir.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    };
    write(
        "main.rs",
        "mod a;\n#[cfg(test)]\nmod tests;\nmod b;\n\nfn main() {}\n",
    );
    write("a.rs", "struct A;\n");
    write("tests.rs", "mod nested;\n\n#[test]\nfn checks() {}\n");
    write("tests/nested.rs", "#[test]\nfn deeper() {}\n");
    write("b.rs", "struct B;\n");

    let chunks: Vec<(String, bool)> = tutorial::chunks(dir.path())
        .unwrap()
        .into_iter()
        .map(|chunk| (chunk.name, chunk.test))
        .collect();
    let expected = [
        ("a", false),
        ("tests/nested", true),
        ("tests", true),
        ("b", false),
        (ROOT, false),
    ]
    .map(|(name, test)| (name.to_owned(), test));
    assert_eq!(chunks, expected);
}

/// The chunks of an example follow each file's module declarations, with each
/// file's own items where they stand among those declarations.
///
/// A file's own items come before its modules when they precede the
/// declarations, between them when they stand between, after them when they
/// follow, and not at all when the file has none.
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
        "mod first;\nmod second;\nmod third;\nmod fourth;\n\nfn main() {}\n",
    );
    write("first.rs", "struct First;\n\nmod a;\nmod b;\n");
    write("first/a.rs", "struct A;\n");
    write("first/b.rs", "struct B;\n");
    write("second.rs", "mod c;\n\nuse c::C;\n\nfn second(_: C) {}\n");
    write("second/c.rs", "pub(super) struct C;\n");
    write("third.rs", "mod d;\n");
    write("third/d.rs", "struct D;\n");
    write("fourth.rs", "mod e;\n\nstruct Fourth;\n\nmod f;\n");
    write("fourth/e.rs", "struct E;\n");
    write("fourth/f.rs", "struct F;\n");

    let names: Vec<String> = tutorial::chunks(dir.path())
        .unwrap()
        .into_iter()
        .map(|chunk| chunk.name)
        .collect();
    assert_eq!(
        names,
        [
            "first", "first/a", "first/b", "second/c", "second", "third/d", "fourth/e", "fourth",
            "fourth/f", ROOT
        ]
    );
}

/// A file whose own items a module declaration splits in two is rejected,
/// since its items form one chunk, which has one place in the order.
#[test]
fn own_items_split_by_a_module_declaration_are_rejected() {
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
    let flattened = [untested("struct A;"), untested("fn main() {}")];
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
        let result = tutorial::fragment(&[untested(line)], 0);
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

/// Text of one to three shown lines.
fn shown_text() -> impl Strategy<Value = String> {
    prop::collection::vec(shown_line(), 1..4).prop_map(|lines| lines.join("\n"))
}

/// A flattened chunk, a test chunk or not, whose text and test copy are shown
/// lines.
fn flattened_chunk() -> impl Strategy<Value = Flattened> {
    (shown_text(), prop::option::of(shown_text()))
        .prop_map(|(text, test_copy)| Flattened { text, test_copy })
}

proptest! {
    /// Removing a fragment's hidden markers, its `allow` line, and its `main`
    /// stub reproduces the program up to its chunk, then its test copy.
    ///
    /// That is: the code fence holds every chunk's text up to and including
    /// its own, separated by blank lines, and then, for a test chunk, a blank
    /// line and its test copy.
    #[test]
    fn a_fragment_unhidden_is_the_program_up_to_its_chunk_then_its_test_copy(
        flattened in prop::collection::vec(flattened_chunk(), 1..5),
        pick in any::<prop::sample::Index>(),
    ) {
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
                chunk.text.lines().chain(separator)
            })
            .chain(flattened[index].test_copy.iter().flat_map(|copy| std::iter::once("").chain(copy.lines())))
            .collect();
        prop_assert_eq!(body, expected);
    }
}
