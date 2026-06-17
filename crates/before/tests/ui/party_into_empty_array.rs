//! `From<Party>` for `[Party; N]` requires `N >= 1`: a `Party` owns a nonempty
//! region and cannot vanish into zero shares. The `const { assert!(N >= 1) }`
//! guard makes `[Party; 0]::from` a compile error rather than a region-losing
//! conversion.

use before::Party;

fn main() {
    let _empty: [Party; 0] = Party::seed().into();
}
