# Placeholders

Each directory here is an empty crate that reserves its name on crates.io for
the real crate of that name in this repository, until the real crate's first
release. A placeholder is version 0.0.0 and carries one doc sentence and no
code; the real release supersedes it, and yanking the placeholder afterwards
(`cargo yank --version 0.0.0 <crate>`) keeps anyone from selecting it.

To reserve a name, publish its placeholder from its directory:

```sh
cd placeholders/<crate>
cargo publish
```

crates.io's [usage policy](https://crates.io/policies) removes crates that
exist only to hold a name for a prolonged period without development on the
corresponding repository, so a placeholder is published only for a crate in
active development here.

Each placeholder is its own workspace, so the repository's workspace, lints
and lockfile ignore it. Once a crate's first real release is published, its
placeholder can be deleted.
