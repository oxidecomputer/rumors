//! The kv example, run as processes: its HTTP interface, replication and
//! network membership between nodes, its credentials and bookmark slots, its
//! command line, and its shutdown.
//!
//! Each test builds the example once, starts nodes on loopback ports the
//! operating system picks, and finds each node's addresses in the `listening`
//! event it logs once it is ready. No test waits a fixed time: every wait polls
//! for the state it expects, up to a deadline.
//!
//! Every node seeds an empty network when it starts, and an empty node gives
//! way to any node holding data, joining that node's network. So a partition
//! cannot be formed by racing two nodes' first writes against their first
//! meeting; the scenarios that need one let each side write before any node
//! can reach the other.
//!
//! The suite interrupts nodes with SIGINT to shut them down as Ctrl-C does,
//! and so runs on Unix only.

#![cfg(unix)]

#[path = "kv/fixture.rs"]
mod fixture;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::thread;

use http::StatusCode;

use fixture::{
    CREATED_CREDENTIALS, CREDENTIALS_FILE, Dirs, FORM, Node, Relay, all_read, eventually,
    idle_keep_alive, slots,
};

/// The entries `pairs` name, as `GET /` lists them.
fn entries(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|&(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

/// The one value every node reports in `seen`, describing `seen` as `what`
/// when the nodes differ or any reports none.
fn agreement(what: &str, seen: &[Option<String>]) -> Result<String, String> {
    match seen {
        [Some(first), rest @ ..] if rest.iter().all(|other| other.as_ref() == Some(first)) => {
            Ok(first.clone())
        }
        _ => Err(format!("{what}: {seen:?}")),
    }
}

/// A node's writes are readable in its very next request.
///
/// The last value of a key repeated in one write wins, a percent-encoded value
/// decodes, a key never written reads as 404, and a delete answers 204 whether
/// or not the key is present, leaving it absent.
#[test]
fn one_node_reads_its_own_writes_and_deletes() {
    let dirs = Dirs::new();
    let a = dirs.node("a").start();
    assert_eq!(a.list(), entries(&[]));

    a.set(&[("x", "1"), ("y", "2")]);
    assert_eq!(a.get("x").as_deref(), Some("1"));

    a.set(&[("x", "3"), ("x", "4")]);
    assert_eq!(a.get("x").as_deref(), Some("4"));

    a.set(&[("spaced", "a value & more=yes")]);
    assert_eq!(a.get("spaced").as_deref(), Some("a value & more=yes"));

    assert_eq!(a.get("missing"), None);
    assert_eq!(a.delete("y"), StatusCode::NO_CONTENT);
    assert_eq!(a.get("y"), None);
    assert_eq!(a.delete("never-set"), StatusCode::NO_CONTENT);

    assert_eq!(
        a.list(),
        entries(&[("spaced", "a value & more=yes"), ("x", "4")])
    );
}

/// A write with an empty key or an empty value anywhere in it is rejected
/// whole with 422, storing none of its pairs, and a write without the form
/// content type is rejected with 415.
#[test]
fn a_write_with_an_empty_key_or_value_stores_nothing() {
    let dirs = Dirs::new();
    let a = dirs.node("a").start();

    for body in ["=v", "ok=1&=v", "e="] {
        assert_eq!(
            a.post_raw(body, Some(FORM)),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{body:?}"
        );
    }
    assert_eq!(a.post_raw("k=v", None), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(a.list(), entries(&[]));
}

/// A write reaches every node, wherever it lands: of three nodes, one only
/// accepts, one dials it, and one dials both; each in turn overwrites one key,
/// and every node reads each new value.
#[test]
fn a_write_on_any_node_reaches_every_node() {
    let dirs = Dirs::new();
    let a = dirs.node("a").start();
    let b = dirs.node("b").peer(a.gossip()).start();
    let c = dirs.node("c").peer(a.gossip()).peer(b.gossip()).start();
    let nodes = [&a, &b, &c];

    for (writer, value) in [(&b, "from-b"), (&c, "from-c"), (&a, "from-a")] {
        writer.set(&[("k", value)]);
        eventually(|| all_read(&nodes, "k", Some(value)));
    }
}

/// Ten writes sent to one node at once all apply, none lost to another's
/// concurrent update.
#[test]
fn concurrent_writes_to_one_node_all_apply() {
    let dirs = Dirs::new();
    let a = dirs.node("a").start();
    let pairs: Vec<(String, String)> = (0..10)
        .map(|i| (format!("key{i}"), i.to_string()))
        .collect();

    thread::scope(|scope| {
        for (key, value) in &pairs {
            let a = &a;
            scope.spawn(move || a.set(&[(key, value)]));
        }
    });

    assert_eq!(a.list(), pairs.into_iter().collect());
}

/// An empty write stores nothing, so the node still gives way: when it first
/// meets a node holding a write, it joins that node's network and reads the
/// write.
#[test]
fn a_node_whose_only_write_was_empty_joins_a_network_with_data() {
    let dirs = Dirs::new();
    let relay = Relay::new();
    let a = dirs.node("a").peer(relay.address()).start();
    a.set(&[]);
    assert_eq!(a.list(), entries(&[]));

    let c = dirs.node("c").start();
    c.set(&[("k", "from-c")]);
    relay.forward(c.gossip());

    eventually(|| a.reads("k", Some("from-c")));
}

/// Nodes that meet before anyone writes take on one network and merge the
/// writes they later make apart.
///
/// Nodes b and c meet through a, and all three log taking on one network; a is
/// killed, b and c each overwrite one key while they cannot reach each other,
/// and a restarted a that dials both brings all three to one of their values.
#[test]
fn nodes_that_meet_empty_merge_writes_made_apart() {
    let dirs = Dirs::new();
    let mut a = dirs.node("a").start();
    let b = dirs.node("b").peer(a.gossip()).start();
    let c = dirs.node("c").peer(a.gossip()).start();
    eventually(|| {
        agreement(
            "a, b, and c took on networks",
            &[a.network(), b.network(), c.network()],
        )
    });

    a.kill();
    b.set(&[("k", "from-b")]);
    c.set(&[("k", "from-c")]);

    let a = dirs
        .node("a again")
        .peer(b.gossip())
        .peer(c.gossip())
        .start();
    let agreed =
        eventually(|| agreement("a, b, and c read k", &[a.get("k"), b.get("k"), c.get("k")]));
    assert!(["from-b", "from-c"].contains(&agreed.as_str()), "{agreed}");
}

/// Two networks, b's and c's, each holding its own value of `k`, bridged by a
/// node a that joined one of them: returned as (a, b, c) once a reports
/// meeting the other.
fn partition(dirs: &Dirs) -> (Node, Node, Node) {
    let b = dirs.node("b").start();
    b.set(&[("k", "from-b")]);
    let c = dirs.node("c").start();
    c.set(&[("k", "from-c")]);

    let a = dirs.node("a").peer(b.gossip()).peer(c.gossip()).start();
    eventually(|| {
        if a.logged("peer belongs to network") {
            Ok(())
        } else {
            Err("a has not met a peer of another network".to_owned())
        }
    });
    (a, b, c)
}

/// Networks that each hold data never merge.
///
/// Nodes b and c write before any node can reach both, and once a joins one of
/// them and meets the other, b and c keep their own networks and values, and a
/// holds the value of the network it joined.
#[test]
fn networks_that_each_hold_data_stay_apart() {
    let dirs = Dirs::new();
    let (a, b, c) = partition(&dirs);

    assert_eq!(b.get("k").as_deref(), Some("from-b"));
    assert_eq!(c.get("k").as_deref(), Some("from-c"));
    let b_network = b.network().expect("b logs its network before it listens");
    let c_network = c.network().expect("c logs its network before it listens");
    assert_ne!(b_network, c_network);
    let expected = match a.network() {
        Some(joined) if joined == b_network => "from-b",
        Some(joined) if joined == c_network => "from-c",
        joined => panic!("a took on {joined:?}, neither b's {b_network} nor c's {c_network}"),
    };
    assert_eq!(a.get("k").as_deref(), Some(expected));
}

/// A partition heals when one side restarts: c, apart from b as above, is
/// interrupted and restarted, holding nothing, dials b, and reads b's value.
#[test]
fn a_restarted_node_heals_its_partition() {
    let dirs = Dirs::new();
    let (_a, b, mut c) = partition(&dirs);

    let status = c.interrupt();
    assert!(status.success(), "{status}");
    let c = dirs.node("c again").peer(b.gossip()).start();

    eventually(|| c.reads("k", Some("from-b")));
}

/// The credentials file alone admits a node on another machine: a node whose
/// directories hold nothing but a copy of the first node's `credentials.pem`
/// creates no credentials of its own and reads the first node's write.
#[test]
fn the_credentials_file_alone_admits_a_node_elsewhere() {
    let here = Dirs::new();
    let a = here.node("a").start();
    a.set(&[("k", "from-a")]);

    let elsewhere = Dirs::new();
    fs::create_dir_all(elsewhere.credentials()).unwrap();
    fs::copy(
        here.credentials().join(CREDENTIALS_FILE),
        elsewhere.credentials().join(CREDENTIALS_FILE),
    )
    .unwrap();
    let b = elsewhere.node("b").peer(a.gossip()).start();

    eventually(|| b.reads("k", Some("from-a")));
    assert!(!b.logged(CREATED_CREDENTIALS));
    assert_eq!(fs::read_dir(elsewhere.credentials()).unwrap().count(), 1);
}

/// Nodes starting at once on an empty credentials directory agree on one
/// credential.
///
/// Exactly one logs creating it, the directory holds only `credentials.pem`,
/// readable and writable by its owner alone, and a fourth node dialing all
/// three replicates its write to each.
#[test]
fn nodes_starting_at_once_agree_on_one_credential() {
    let dirs = Dirs::new();
    let mut nodes = ["a", "b", "c"].map(|name| dirs.node(name).spawn());
    for node in &mut nodes {
        node.wait_listening();
    }

    let creators = nodes
        .iter()
        .filter(|node| node.logged(CREATED_CREDENTIALS))
        .count();
    assert_eq!(creators, 1);
    let files: Vec<_> = fs::read_dir(dirs.credentials())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(files, [CREDENTIALS_FILE]);
    let mode = fs::metadata(dirs.credentials().join(CREDENTIALS_FILE))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600, "{mode:o}");

    let [a, b, c] = &nodes;
    let d = dirs
        .node("d")
        .peer(a.gossip())
        .peer(b.gossip())
        .peer(c.gossip())
        .start();
    d.set(&[("k", "from-d")]);
    eventually(|| all_read(&[a, b, c], "k", Some("from-d")));
}

/// Nodes reuse free bookmark slots before creating new ones.
///
/// Three nodes hold slots 0, 1, and 2; after one is interrupted and another
/// killed, the two nodes started next hold all three again; and with every
/// slot held, the next node creates slot 3.
#[test]
fn nodes_reuse_free_bookmark_slots_before_creating_more() {
    let dirs = Dirs::new();
    let held = |expected: &[u32]| {
        let expected: BTreeSet<u32> = expected.iter().copied().collect();
        assert_eq!(slots(&dirs.bookmarks()), (expected.clone(), expected));
    };

    let _a = dirs.node("a").start();
    let mut b = dirs.node("b").start();
    let mut c = dirs.node("c").start();
    held(&[0, 1, 2]);

    let status = b.interrupt();
    assert!(status.success(), "{status}");
    c.kill();
    let _d = dirs.node("d").start();
    let _e = dirs.node("e").start();
    held(&[0, 1, 2]);

    let _f = dirs.node("f").start();
    held(&[0, 1, 2, 3]);
}

/// The command line rejects a peer that is not `host`, `host:port`, `port`,
/// or `:port`.
///
/// Each rejected value gets clap's usage error, exit status 2, naming the
/// value; the command line accepts `localhost`, and `port` and `:port`, each
/// of which reaches a node on that port of this machine.
#[test]
fn peers_parse_as_a_host_and_an_optional_port() {
    let dirs = Dirs::new();
    for value in [
        "host:notaport",
        "host:99999",
        "99999",
        ":notaport",
        ":",
        "user@host:7101",
        "a b",
        "::1",
        "",
    ] {
        let mut node = dirs.node("rejected").peer(value).spawn();
        let status = node.wait_exit();
        assert_eq!(status.code(), Some(2), "{value:?}");
        assert!(node.logged(&format!("'{value}'")), "{value:?}");
    }

    drop(dirs.node("localhost").peer("localhost").start());

    let a = dirs.node("a").start();
    a.set(&[("k", "from-a")]);
    let b = dirs
        .node("b")
        .peer(format!(":{}", a.gossip().port()))
        .start();
    let c = dirs.node("c").peer(a.gossip().port()).start();
    eventually(|| all_read(&[&b, &c], "k", Some("from-a")));
}

/// The command line takes `port` or `:port` for `--gossip` and `--http`,
/// keeping each flag's default host.
///
/// A node started with `--gossip :0 --http 0` gossips on every interface,
/// where another node reaches it, and serves HTTP on loopback alone; `:notaport`
/// for either flag exits with clap's usage error, exit status 2, naming the
/// value.
#[test]
fn listening_addresses_take_a_bare_port() {
    let dirs = Dirs::new();
    for launch in [
        dirs.node("rejected gossip").gossip(":notaport"),
        dirs.node("rejected http").http(":notaport"),
    ] {
        let mut node = launch.spawn();
        assert_eq!(node.wait_exit().code(), Some(2));
        assert!(node.logged("':notaport'"));
    }

    let a = dirs.node("a").gossip(":0").http("0").start();
    assert_eq!(a.gossip().ip(), Ipv4Addr::UNSPECIFIED);
    assert_eq!(a.http().ip(), Ipv4Addr::LOCALHOST);
    a.set(&[("k", "from-a")]);
    let b = dirs
        .node("b")
        .peer(format!(":{}", a.gossip().port()))
        .start();
    eventually(|| b.reads("k", Some("from-a")));
}

/// A node whose HTTP port is taken exits with an error naming the address,
/// before it creates credentials or claims a bookmark.
#[test]
fn a_taken_port_fails_startup_before_anything_else() {
    let dirs = Dirs::new();
    let taken = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = taken.local_addr().unwrap();

    let mut a = dirs.node("a").http(&address.to_string()).spawn();
    let status = a.wait_exit();

    assert!(!status.success(), "{status}");
    assert!(a.logged(&format!("listening on {address}")));
    assert!(!dirs.credentials().exists());
    assert!(!dirs.bookmarks().exists());
}

/// A node dialing `[::1]:port` replicates from a node gossiping on the IPv6
/// loopback address.
#[test]
fn a_node_replicates_from_a_peer_over_ipv6() {
    let dirs = Dirs::new();
    let a = dirs.node("a").gossip("[::1]:0").start();
    a.set(&[("k", "from-a")]);

    let b = dirs.node("b").peer(a.gossip()).start();
    eventually(|| b.reads("k", Some("from-a")));
}

/// How many times the shutdown test interrupts a node.
const SHUTDOWN_ROUNDS: usize = 20;

/// Interrupting a node with idle HTTP connections open shuts it down cleanly,
/// with exit status 0, in every one of many rounds: one connection has sent
/// nothing, and another has finished a request and waits in keep-alive.
#[test]
fn interrupting_a_node_with_idle_connections_exits_cleanly() {
    let dirs = Dirs::new();
    for round in 0..SHUTDOWN_ROUNDS {
        let mut a = dirs.node("a").start();
        a.set(&[("k", "v")]);
        // The node accepts connections in order, so once the second has its
        // response, the first is accepted too.
        let _silent = TcpStream::connect(a.http()).unwrap();
        let _kept = idle_keep_alive(a.http());

        let status = a.interrupt();
        assert!(status.success(), "round {round}: {status}");
    }
}

/// Interrupting a node that has accepted a connection on its gossip port that
/// has sent nothing, not even the start of a TLS handshake, shuts it down with
/// exit status 0.
#[test]
fn interrupting_a_node_with_a_silent_gossip_connection_exits_cleanly() {
    let dirs = Dirs::new();
    let mut a = dirs.node("a").start();
    a.set(&[("k", "from-a")]);
    let _silent = TcpStream::connect(a.gossip()).unwrap();
    // The node accepts connections in order, so once b, dialing after the
    // silent connection, replicates, the silent connection is accepted too.
    let b = dirs.node("b").peer(a.gossip()).start();
    eventually(|| b.reads("k", Some("from-a")));

    let status = a.interrupt();
    assert!(status.success(), "{status}");
}

/// Interrupting a node whose only peer accepts connections but never answers
/// a TLS handshake shuts it down with exit status 0.
///
/// The node stops dialing in the middle of a handshake, and its attempt to
/// retire to that peer gives up.
#[test]
fn interrupting_a_node_whose_peer_never_answers_exits_cleanly() {
    let dirs = Dirs::new();
    let peer = TcpListener::bind("127.0.0.1:0").unwrap();
    peer.set_nonblocking(true).unwrap();
    let mut a = dirs.node("a").peer(peer.local_addr().unwrap()).start();
    let _dialed = eventually(|| {
        peer.accept()
            .map_err(|error| format!("a has not dialed its peer: {error}"))
    });

    let status = a.interrupt();
    assert!(status.success(), "{status}");
}

/// A node that departs cleanly gives its peer nothing to report: b dials a
/// and replicates a write, b is interrupted and retires to a, and once a shuts
/// down too, a's log holds no line about a connection ending.
#[test]
fn a_clean_departure_logs_nothing_on_the_peer() {
    let dirs = Dirs::new();
    let mut a = dirs.node("a").start();
    let mut b = dirs.node("b").peer(a.gossip()).start();
    b.set(&[("k", "from-b")]);
    eventually(|| a.reads("k", Some("from-b")));

    let status = b.interrupt();
    assert!(status.success(), "{status}");
    let status = a.interrupt();
    assert!(status.success(), "{status}");

    let ended: Vec<String> = a
        .log_lines()
        .into_iter()
        .filter(|line| line.contains("connection from peer ended"))
        .collect();
    assert_eq!(ended, Vec::<String>::new());
}
