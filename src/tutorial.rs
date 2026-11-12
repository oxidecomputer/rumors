//! A practical guide: using `rumors` to build an eventually-consistent
//! replicated KV store.
//!
//! In this tutorial, we will start from scratch and build up a complete,
//! running application, which implements a key-value store using `rumors` to
//! synchronize state. To follow along, paste or re-type each Rust code block
//! successively into your `main.rs`, unless otherwise noted.
//!
//! # Step 1: dependencies
//!
//! Start a fresh binary crate (`cargo new kv`) and add three dependencies:
//!
#![doc = tutorial!("Cargo.toml")]
#![doc = tutorial!("table")]
#![doc = tutorial!("table/apply")]
#![doc = tutorial!("table/read")]
#![doc = tutorial!("replica")]
#![doc = tutorial!("replica/catch_up")]
#![doc = tutorial!("replica/read")]
#![doc = tutorial!("replica/gossip")]
#![doc = tutorial!("replica/give_way")]
#![doc = tutorial!("replica/into_rumors")]
#![doc = tutorial!("credentials")]
#![doc = tutorial!("credentials/generate")]
#![doc = tutorial!("credentials/store")]
#![doc = tutorial!("credentials/load")]
#![doc = tutorial!("tls")]
#![doc = tutorial!("tls/connect")]
#![doc = tutorial!("tls/accept")]
#![doc = tutorial!("node")]
#![doc = tutorial!("node/read")]
#![doc = tutorial!("node/claim_bookmark")]
#![doc = tutorial!("node/seed")]
#![doc = tutorial!("node/join")]
#![doc = tutorial!("node/update")]
#![doc = tutorial!("node/gossip_over")]
#![doc = tutorial!("node/dial")]
#![doc = tutorial!("node/serve")]
#![doc = tutorial!("node/start")]
#![doc = tutorial!("node/retire")]
#![doc = tutorial!("node/shutdown")]
#![doc = tutorial!("api/list")]
#![doc = tutorial!("api/read")]
#![doc = tutorial!("api/update")]
#![doc = tutorial!("api/set")]
#![doc = tutorial!("api/delete")]
#![doc = tutorial!("api")]
#![doc = tutorial!("cli")]
#![doc = tutorial!("main")]
