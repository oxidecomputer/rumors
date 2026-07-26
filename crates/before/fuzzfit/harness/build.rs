//! Embeds the building toolchain's identity so the suite can refuse to
//! judge fuel against bands pinned under a different codegen.
//!
//! The guest and the harness are built back-to-back by the `just
//! fuzzfit-*` recipes under one active toolchain, so the harness's rustc
//! stands in for the guest's; `bands::PINNED_RUSTC` records the pinning
//! toolchain and `tests/enforce.rs` asserts the two match.

fn main() {
    let version = rustc_version::version_meta()
        .expect("rustc_version reads the active toolchain")
        .short_version_string;
    println!("cargo:rustc-env=FUZZFIT_RUSTC_VERSION={version}");
    println!("cargo:rerun-if-env-changed=RUSTC");
}
