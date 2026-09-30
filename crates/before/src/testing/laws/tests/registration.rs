//! Checks that the law registry and the declared law groups cannot drift apart.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// Every registered law has a unique name.
#[test]
fn law_names_are_unique_across_groups() {
    let names = super::super::registered_names();
    let mut seen = BTreeSet::new();
    let duplicates: Vec<&str> = names
        .iter()
        .filter(|name| !seen.insert(**name))
        .copied()
        .collect();
    assert!(duplicates.is_empty(), "duplicate law names: {duplicates:?}");
}

/// Every declared law group is registered for execution.
#[test]
fn every_law_group_is_registered() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/testing/laws");
    let mut declared = BTreeSet::new();
    collect_group_declarations(&root, &mut declared);

    let registered: BTreeSet<String> = super::super::REGISTERED_GROUPS
        .iter()
        .map(|group| (*group).to_string())
        .collect();
    assert_eq!(
        declared, registered,
        "declared and registered law groups differ"
    );
}

/// Adds every `pub static` law-group declaration beneath `path` to `groups`.
fn collect_group_declarations(path: &Path, groups: &mut BTreeSet<String>) {
    for entry in
        fs::read_dir(path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
    {
        let path = entry.expect("law module directory entry").path();
        if path.is_dir() {
            collect_group_declarations(&path, groups);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            collect_file_declarations(&path, groups);
        }
    }
}

/// Adds every `pub static` law-group declaration in `path` to `groups`.
fn collect_file_declarations(path: &Path, groups: &mut BTreeSet<String>) {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix("pub static ") else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|character| character.is_alphanumeric() || *character == '_')
            .collect();
        assert!(!name.is_empty(), "unnamed public law group: {line}");
        assert!(
            groups.insert(name.clone()),
            "law group {name} is declared more than once"
        );
    }
}
