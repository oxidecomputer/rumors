# The `kv` example for `rumors`

This example implements an eventually consistent replicated key-value store,
using `rumors` for causal gossip between nodes, with an HTTP API for
manipulating the store.

The source files herein are identical in meaningful content to the source as
presented in the
[`rumors::tutorial`](https://docs.rs/rumors/latest/rumors/tutorial/index.html)
module of the crate. The tutorial presents a "flattened" view, where each code
snippet can be progressively pasted into `main.rs` to form a single, long file.
The flattened view is generated from this example's source at build time, so
this is the authoritative source for the tutorial.

Because it is presented in detail in the running context of the `rumors`
tutorial, this source is intentionally light on interior comments. To understand
in more detail how it works, or how to run it, refer to the tutorial.
