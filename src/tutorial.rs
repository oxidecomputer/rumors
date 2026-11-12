//! A practical guide: using `rumors` to build an eventually-consistent
//! replicated KV store.
//!
//! In this tutorial, we will start from scratch and build up a complete running
//! application: a replicated key-value store using `rumors` to synchronize
//! state between its nodes.
//!
//! **We will do this in only
#![doc = include_str!(concat!(env!("OUT_DIR"), "/tutorial/kv/lines.txt"))]
//! lines of code.** Despite this brevity, our application will feature:
//!
//! - authentication and authorization via mutual TLS
//! - a RESTful HTTP API
//! - identity persistence
//! - graceful shutdown
//! - error handling
//! - logging
//! - ...and more!
//!
//! The intent of this tutorial is to present this application *as you might
//! actually write it*: succinct, but idiomatic, making use of the third-party
//! libraries you would actually want to use.
//!
//! # A sneak preview
//!
//! This tutorial is included in the `rumors` crate as
//! [`example/kv`](https://github.com/oxidecomputer/rumors/tree/main/examples/kv),
//! so you can run it immediately. Let's take a peek at what we'll be
//! building...
//!
//! ## Running KV nodes
//!
//! First, we'll start two different KV store nodes.
//!
//! Each node needs to bind two ports: its *HTTP* port, for the API we'll
//! interact with, and its *gossip* port, for the `rumors` protocol between
//! nodes. We'll use ports in the 800x range for HTTP, and ports in the 900x
//! range for gossip, so that our two nodes will be *reachable by us* at
//! `localhost:8000` and `:8001`, but will *reach each other* at
//! `localhost:9000` and `:9001`, respectively.
//!
//! Let's set them up!
//!
//! ```bash
//! $ cargo run --release --example kv -- --http 8000 --gossip 9000
//! 2026-11-12T22:20:11.503269Z  INFO kv::credentials::store: created path=kv-credentials/credentials.pem
//! 2026-11-12T22:20:11.543013Z  INFO kv: listening gossip=0.0.0.0:9000 http=127.0.0.1:8000
//! ```
//!
//! The second one should be told to peer with the first, so that they can talk.
//!
//! ```bash
//! $ cargo run --release --example kv -- --http 8001 --gossip 9001 --peer 9000
//! 2026-11-12T22:20:42.635466Z  INFO kv: listening gossip=0.0.0.0:9001 http=127.0.0.1:8001
//! ```
//!
//! Notice that when not otherwise specified, HTTP binds to `127.0.0.1`, so it's
//! only reachable locally, but gossip binds to `0.0.0.0`. You may also notice
//! that the first node created a file `credentials.pem` when we started it. If
//! you copy `credentials.pem` to a second machine (or a third...), it will
//! authenticate that machine so that it can join the same gossip network. We'll
//! see how this works later on --- for now, let's keep playing on one machine.
//!
//! ## Using the replicated KV store
//!
//! Then in a third terminal, we'll interact with the replicated store, and see
//! how it works.
//!
//! ```bash
//! # We write "greeting" = "hello" to the first node...
//! $ curl -d greeting=hello localhost:8000
//!
//! # ...and then we can read the value of "greeting" from the second node:
//! $ curl localhost:8001/greeting
//! hello
//!
//! # And if we delete "greeting" from the second node...
//! $ curl -X DELETE localhost:8001/greeting
//!
//! # ...then it's gone on the first node as well:
//! $ curl -i localhost:8000/greeting
//! HTTP/1.1 404 Not Found
//! content-length: 0
//! date: Thu, 12 Nov 2026 22:10:07 GMT
//! ```
//!
//! ## What you might learn
//!
//! If this tutorial is successful, *you will know how to use `rumors` to make
//! this application work*, from the bottom to the top.
//!
//! Hopefully, you'll also understand how to extend and modify this simple
//! example to add your own functionality. Feel free to take this tutorial and
//! bend it to your own needs --- that's what it's for.
//!
//! # How to read this tutorial
//!
//! The rest of this document is a kind of "literate Rust program": each code
//! block progressively builds on the prior, so that any prefix of the tutorial
//! will compile on its own, depending only on things we've already discussed.
//! We'll `use` new dependencies right when we need them, in-line with the rest
//! of the code.
//!
//! The only exception is `fn main() {}`, which we'll unveil at the bottom of
//! the tutorial, but which, in order to compile your code, you need defined
//! from the get-go. So, if you want to follow along, you'll start with a
//! `main.rs` which looks like this:
//!
//! ```rust
//! // Follow the tutorial here...
//!
//! fn main() {} // <-- Replace this at the very end!
//! ```
//!
//! When we finish building the application, your final step will be to replace
//! that `main()` stub with the *actual* main function which runs the key-value
//! store, using everything we've built above it.
//!
//! You can also consult [this crate's `kv`
//! example](https://github.com/oxidecomputer/rumors/tree/main/examples/kv),
//! which shows the same project split into hierarchical Rust modules, rather
//! than this tutorial's presentation as one flat file. The two are otherwise
//! entirely identical: `cargo run --example kv` runs precisely the code you'll
//! read (and write!) below.
//!
//! # Setting up the project
//!
//! Start a fresh binary crate (`cargo new kv`) and set up your crate's
//! dependencies in your `Cargo.toml`:
//!
#![doc = tutorial!("Cargo.toml")]
//!# The model: a key-value store
//!
//!## Keys and their values
//!
#![doc = tutorial!("table/key")]
//!## A table is a map from keys to values
//!
#![doc = tutorial!("table/value")]
//!## A table mapping keys to values
//!
#![doc = tutorial!("table")]
//!## Updating the table
//!
#![doc = tutorial!("table/update")]
//!## Reading the table
//!
#![doc = tutorial!("table/read")]
//!# A replicated table
//!
//!## A view is a table, plus a stream of updates
//!
#![doc = tutorial!("view")]
//!## A replica is a view, plus a way to send updates
//!
#![doc = tutorial!("replica")]
//!## Updating the replica, and keeping it up to date
//!
//!### Catching up on what we missed
//!
#![doc = tutorial!("replica/catch_up")]
//!### Applying local updates
//!
#![doc = tutorial!("replica/update")]
//!### Keeping up to date with remote updates
//!
#![doc = tutorial!("replica/keep_updated")]
//!### Reading the current state
//!
#![doc = tutorial!("replica/read")]
//!# mTLS: authentication and authorization between replicas
//!
//!## Credentials for membership in the gossip network
//!
#![doc = tutorial!("credentials")]
//!### Generating a self-signed certificate
//!
#![doc = tutorial!("credentials/generate")]
//!### Storing credentials on disk
//!
#![doc = tutorial!("credentials/store")]
//!### Loading credentials from disk
//!
#![doc = tutorial!("credentials/load")]
//!## Configuring mTLS using our credentials
//!
#![doc = tutorial!("tls")]
//!## Connecting, and accepting connections
//!
#![doc = tutorial!("tls/connect")]
#![doc = tutorial!("tls/accept")]
//!# A node is a replica which can talk to peers
//!
#![doc = tutorial!("node")]
//!## Reading the inner replica
//!
#![doc = tutorial!("node/read")]
//!## A bookmark: recovering "identity space" after a restart
//!
#![doc = tutorial!("node/claim_bookmark")]
//!## Creating a gossip network
//!
#![doc = tutorial!("node/seed")]
//!## Joining a gossip network
//!
#![doc = tutorial!("node/join")]
//!## Submitting updates
//!
#![doc = tutorial!("node/wait_for_replica")]
//!
#![doc = tutorial!("node/update")]
//!## Gossiping with other nodes
//!
//!### When two gossip networks collide, whose "wins"?
//!
#![doc = tutorial!("node/give_way")]
//!### Gossip, built to converge on one "winning" network
//!
#![doc = tutorial!("node/gossip_over")]
//!## A node is a server *and* a client
//!
//!### Dialing other nodes
//!
#![doc = tutorial!("node/dial")]
//!### Serving incoming connections
//!
#![doc = tutorial!("node/serve")]
//!## Starting the node
//!
#![doc = tutorial!("node/start")]
//!## Stopping the node
//!
//!### Preserving "identity space" during retirement
//!
#![doc = tutorial!("node/retire")]
//!### It is now safe to turn off your computer
//!
#![doc = tutorial!("node/shutdown")]
//!# A RESTful HTTP API for each and every node
//!
//!## Read operations
//!
//!### Getting the whole table
//!
#![doc = tutorial!("api/list")]
//!### Reading a specific key
//!
#![doc = tutorial!("api/read")]
//!## Write operations
//!
#![doc = tutorial!("api/update")]
//!### Setting the value for a key (or the values for several keys)
//!
#![doc = tutorial!("api/set")]
//!### Deleting keys
//!
#![doc = tutorial!("api/delete")]
//!## Putting the whole API together
//!
#![doc = tutorial!("api")]
//!# A command-line interface for the node
//!
#![doc = tutorial!("cli")]
//!# Logging
//!
#![doc = tutorial!("log_to_stderr")]
//!# And finally... `main()`
//!
//!Okay, we're *almost* at `main()`, just one more helper function:
//!
#![doc = tutorial!("listen")]
//!And, if you've made it this far, here we have it: a *proper* `main()`
//!function, which --- if you've been following along this whole time --- you
//!can paste over that stub `main() {}` at the bottom of your `main.rs`.
//!
//!This function shouldn't be particularly surprising. It just puts together all
//!the top-level bits we've built so far:
//!
//!- Install our `tracing` subscriber so that our logs print to the console;
//!- Parse the provided command-line options into our `Cli` struct;
//!- Start listening on both the gossip and HTTP addresses specified on the CLI;
//!- Load (or create) our self-signed mTLS credentials from/to disk;
//!- Start the node, configured with those credentials;
//!- Serve the HTTP API, waiting for Ctrl-C to gracefully shut down the API
//!  server;
//!- Finally, gracefully shut down the gossip node.
//!
#![doc = tutorial!("main")]
//! # What have we built?
