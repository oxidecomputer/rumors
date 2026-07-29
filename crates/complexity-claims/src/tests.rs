//! The shared machinery's own adequacy tests.
//!
//! The section scanner is not vacuously green, the extractor's naming
//! (impl blocks, `pub mod` blocks, module level) is pinned on fixtures,
//! its refusal paths fire, and the render lead is fixed.
//!
//! The full template-by-template byte pin lives with each consuming
//! roster's binding tests (the reviewed diff a template edit must pass
//! through); here the fixtures hold the *machinery's* behavior.

use std::fs;
use std::path::PathBuf;

use super::{doc_index, extract_public_fns, section_of, test_fns, Bound, Site, SourceSpec};

/// Write `content` as a scratch source file and return the directory to
/// scan as a crate root, so the file-reading scanners run on fixtures.
fn fixture(name: &str, content: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("complexity-claims-fixture-{name}"));
    fs::create_dir_all(&dir).expect("creating the fixture dir");
    fs::write(dir.join("lib.rs"), content).expect("writing the fixture");
    dir
}

/// The fixture's uniform spec: one file named lib.rs, no module-level
/// namespace unless the test supplies one.
const fn spec(module_prefix: Option<&'static str>) -> [SourceSpec; 1] {
    [SourceSpec {
        path: "lib.rs",
        module_prefix,
        type_override: None,
    }]
}

/// A doc block with no `# Complexity` section scans as missing, and a
/// present section carries its own tokens and ends at the next heading —
/// the scanner is not vacuously green.
#[test]
fn scanner_detects_missing_sections_and_boundaries() {
    assert_eq!(
        section_of("Summary line.\n\nNo sections here.\n"),
        None,
        "a block with no Complexity section must scan as missing"
    );
    let section = section_of("Summary.\n\n# Complexity\n\n`O(|v|)` time.\n\n# Panics\n\nNever.\n")
        .expect("the section exists");
    assert!(
        section.contains("`O(|v|)`") && !section.contains("Never"),
        "the section slice must carry its own tokens and end at the next heading"
    );
    let fenced =
        section_of("# Complexity\n\n`O(n)`.\n\n```\nexample\n```\n").expect("the section exists");
    assert!(
        !fenced.contains("example"),
        "the section slice must end at an example fence"
    );
}

/// The extractor names inherent methods `Type::fn`, `pub mod`-block
/// functions `mod::fn`, and module-level functions under the spec's
/// prefix — and skips trait impls and non-`pub` items.
#[test]
fn extractor_names_every_context() {
    let root = fixture(
        "extract",
        "pub mod meter {\n    pub fn read() -> u64 {\n        0\n    }\n\
         \n    pub(crate) fn hidden() {}\n}\n\
         \npub struct Thing {\n    x: u8,\n}\n\
         \nimpl Thing {\n    pub fn poke(&self) {}\n}\n\
         \nimpl Default for Thing {\n    fn default() -> Thing {\n        Thing { x: 0 }\n    }\n}\n\
         \npub fn top_level() {}\n",
    );
    let fns = extract_public_fns(&root, &spec(Some("fixture")));
    let want: Vec<&str> = vec!["Thing::poke", "fixture::top_level", "meter::read"];
    assert_eq!(fns.iter().map(String::as_str).collect::<Vec<_>>(), want);
}

/// A `pub fn` outside every naming context panics: the extractor must
/// never under-report the surface it exists to pin.
#[test]
#[should_panic(expected = "unexpected module-level")]
fn extractor_refuses_an_unnamed_module_level_fn() {
    let root = fixture("refuse-top", "pub fn stray() {}\n");
    extract_public_fns(&root, &spec(None));
}

/// An impl block nested inside a `pub mod` block panics: it is beyond
/// the line discipline, and silence would under-report its methods.
#[test]
#[should_panic(expected = "nested inside a `pub mod` block")]
fn extractor_refuses_a_nested_impl() {
    let root = fixture(
        "refuse-nested",
        "pub mod inner {\n    impl Thing {\n        pub fn poke(&self) {}\n    }\n}\n",
    );
    extract_public_fns(&root, &spec(None));
}

/// The doc scanner indexes `# Complexity` sections at every site kind —
/// fn (inherent and `pub mod`-block), struct, trait impl, and module
/// doc — and resolves them through [`super::DocIndex::section`].
#[test]
fn doc_scanner_indexes_every_site_kind() {
    let root = fixture(
        "docs",
        "//! Module summary.\n//!\n//! # Complexity\n//!\n//! module line.\n\
         \npub mod meter {\n    /// Read.\n    ///\n    /// # Complexity\n    ///\n    \
         /// mod-fn line.\n    pub fn read() -> u64 {\n        0\n    }\n}\n\
         \n/// A thing.\n///\n/// # Complexity\n///\n/// struct line.\npub struct Thing {\n    x: u8,\n}\n\
         \nimpl Thing {\n    /// Poke.\n    ///\n    /// # Complexity\n    ///\n    \
         /// fn line.\n    pub fn poke(&self) {}\n}\n\
         \n/// Default.\n///\n/// # Complexity\n///\n/// impl line.\n\
         impl Default for Thing {\n    fn default() -> Thing {\n        Thing { x: 0 }\n    }\n}\n",
    );
    let index = doc_index(&root, &spec(None));
    let cases = [
        ("Thing::poke", Site::Fn, "fn line."),
        ("meter::read", Site::Fn, "mod-fn line."),
        ("thing", Site::TypeDoc("lib.rs", "Thing"), "struct line."),
        ("module", Site::ModuleDoc("lib.rs"), "module line."),
        (
            "default",
            Site::ImplDoc("lib.rs", "impl Default for Thing"),
            "impl line.",
        ),
    ];
    for (op, site, want) in cases {
        let section = index
            .section(op, site)
            .unwrap_or_else(|e| panic!("{op}: {e}"));
        assert!(
            section.contains(want),
            "{op}: section {section:?} misses {want:?}"
        );
    }
}

/// The render lead is fixed and a custom line passes through verbatim,
/// so a roster's byte-compare pins exactly the committed text.
#[test]
fn render_lead_and_custom_passthrough_are_fixed() {
    assert_eq!(Bound::Constant.render(), "**Complexity**: `O(1)`.");
    assert_eq!(
        Bound::Custom {
            line: "priced elsewhere.",
            reason: "fixture",
        }
        .render(),
        "**Complexity**: priced elsewhere."
    );
}

/// The witness scanner admits only `#[test]`-attributed functions —
/// attributes between `#[test]` and the fn keep the arming, and helpers
/// or prose mentions never count.
#[test]
fn witness_scanner_admits_only_attributed_tests() {
    let names = test_fns(
        "#[test]\nfn plain() {}\n\
         #[cfg(feature = \"x\")]\n#[test]\nfn cfg_before() {}\n\
         #[test]\n#[ignore]\nfn attr_between() {}\n\
         fn helper() {}\n// #[test] fn commented() {}\n",
    );
    let want: Vec<&str> = vec!["attr_between", "cfg_before", "plain"];
    assert_eq!(names.iter().map(String::as_str).collect::<Vec<_>>(), want);
}
