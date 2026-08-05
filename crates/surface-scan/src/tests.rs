//! The shared machinery's own adequacy tests.
//!
//! The extractor's naming (impl blocks, `pub mod` blocks, module level)
//! is pinned on fixtures, its refusal paths fire, and the witness
//! scanner admits only attributed tests.

use std::fs;
use std::path::PathBuf;

use super::{extract_public_fns, test_fns, SourceSpec};

/// Write `content` as a scratch source file and return the directory to
/// scan as a crate root, so the file-reading scanners run on fixtures.
fn fixture(name: &str, content: &str) -> PathBuf {
    // Keyed by test name and process id: the machine's temp dir is shared
    // across checkouts, so a fixed path lets one session's fixture rewrite
    // race another session's scan in the same suite.
    let dir = std::env::temp_dir().join(format!(
        "surface-scan-fixture-{name}-{}",
        std::process::id()
    ));
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
        type_overrides: &[],
    }]
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
