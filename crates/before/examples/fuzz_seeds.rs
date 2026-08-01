//! Regenerate the committed fuzz seed corpus from the live wire format.
//!
//! Writes every seed in the committed set (`tests/support/fuzz_seed_set.rs`)
//! into `fuzz/seeds/<target>/<name>`, atomically (temp file, then rename),
//! so an interrupted run cannot truncate a committed seed. Run after any
//! deliberate wire-format change; the `fuzz_seeds` integration test holds
//! the committed files byte-identical to this derivation, so the gate
//! stays red until the regenerated seeds are committed.

use std::fs;
use std::path::PathBuf;

#[path = "../tests/support/fuzz_seed_set.rs"]
mod fuzz_seed_set;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fuzz/seeds");
    for seed in fuzz_seed_set::seed_set() {
        let dir = root.join(seed.target);
        fs::create_dir_all(&dir).expect("creating the seed target directory");
        let path = dir.join(seed.name);
        let tmp = dir.join(format!("{}.tmp", seed.name));
        fs::write(&tmp, &seed.bytes).expect("writing the seed bytes");
        fs::rename(&tmp, &path).expect("moving the seed into place");
        println!("wrote {} ({} bytes)", path.display(), seed.bytes.len());
    }
}
