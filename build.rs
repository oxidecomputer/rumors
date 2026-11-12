//! Renders the kv example as the tutorial's code fragments, when the
//! `tutorial` feature is on; `build/tutorial.rs` explains how.

#[cfg(feature = "tutorial")]
#[path = "build/tutorial.rs"]
mod tutorial;

/// The name of the fragment showing the example's manifest, which the
/// tutorial references before any chunk of code.
#[cfg(feature = "tutorial")]
const MANIFEST: &str = "Cargo.toml";

/// The file holding the example's manifest. Cargo leaves a subdirectory
/// holding a `Cargo.toml` out of a package, which would drop the example from
/// the published crate, so the manifest goes by another name in the source.
#[cfg(feature = "tutorial")]
const MANIFEST_TEMPLATE: &str = "Cargo.toml.template";

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    #[cfg(feature = "tutorial")]
    if let Err(error) = generate() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

/// Write the manifest's fragment and each chunk's below `$OUT_DIR/tutorial/kv`,
/// laid out as the example is, and check that `src/tutorial.rs` references the
/// manifest and then every chunk, in teaching order.
#[cfg(feature = "tutorial")]
fn generate() -> Result<(), String> {
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    let var = |name: &str| env::var(name).map_err(|error| format!("${name}: {error}"));
    let crate_dir = PathBuf::from(var("CARGO_MANIFEST_DIR")?);
    let example = crate_dir.join("examples/kv");
    let tutorial_source = crate_dir.join("src/tutorial.rs");
    for watched in [
        crate_dir.join("build"),
        example.clone(),
        tutorial_source.clone(),
    ] {
        println!("cargo::rerun-if-changed={}", watched.display());
    }

    let chunks = tutorial::chunks(&example)?;
    let flattened = tutorial::flatten(&chunks)?;
    let out = PathBuf::from(var("OUT_DIR")?).join("tutorial/kv");
    for (index, chunk) in chunks.iter().enumerate() {
        let path = out.join(format!("{}.md", chunk.name));
        let parent = path.parent().expect("a fragment's path has a directory");
        fs::create_dir_all(parent)
            .map_err(|error| format!("creating {}: {error}", parent.display()))?;
        fs::write(&path, tutorial::fragment(&flattened, index)?)
            .map_err(|error| format!("writing {}: {error}", path.display()))?;
    }

    let template_path = example.join(MANIFEST_TEMPLATE);
    let template = fs::read_to_string(&template_path)
        .map_err(|error| format!("reading {}: {error}", template_path.display()))?;
    let manifest = out.join(format!("{}.md", MANIFEST));
    fs::write(&manifest, tutorial::manifest_fragment(&template))
        .map_err(|error| format!("writing {}: {error}", manifest.display()))?;

    let tutorial = fs::read_to_string(&tutorial_source)
        .map_err(|error| format!("reading {}: {error}", tutorial_source.display()))?;
    let names: Vec<String> = std::iter::once(MANIFEST.to_owned())
        .chain(chunks.into_iter().map(|chunk| chunk.name))
        .collect();
    tutorial::check_order(&names, &tutorial::references(&tutorial)?)
}
