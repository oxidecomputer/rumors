//! Embeds the locked Wasmtime version, one of the identities fuel readings
//! depend on.
//!
//! The version is read from the root workspace's `Cargo.lock`, the version
//! every build of the harness links. The guest's compiler, the other
//! identity, is read from the guest module itself at run time.

use std::path::Path;

fn main() {
    let lock = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../Cargo.lock");
    println!("cargo:rerun-if-changed={}", lock.display());
    let text = std::fs::read_to_string(&lock).expect("the workspace's Cargo.lock is readable");
    println!(
        "cargo:rustc-env=BOARD_FUEL_WASMTIME_VERSION={}",
        locked_version(&text, "wasmtime")
    );
}

/// Returns the version `lock` records for package `name`.
///
/// # Panics
///
/// Panics unless `lock` records exactly one version of `name`.
fn locked_version<'a>(lock: &'a str, name: &str) -> &'a str {
    let name_line = format!("name = \"{name}\"");
    let mut lines = lock.lines();
    let mut versions = Vec::new();
    while let Some(line) = lines.next() {
        if line == name_line {
            let version = lines
                .next()
                .and_then(|line| line.strip_prefix("version = \""))
                .and_then(|rest| rest.strip_suffix('"'))
                .unwrap_or_else(|| panic!("Cargo.lock: {name}'s version follows its name"));
            versions.push(version);
        }
    }
    match versions.as_slice() {
        [version] => version,
        _ => panic!("Cargo.lock records {} versions of {name}", versions.len()),
    }
}
