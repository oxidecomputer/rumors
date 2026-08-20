use super::Peer;

/// The peer's debug view is the promised summary — network, protocol,
/// latest version, live count — with the messages themselves (and `T`'s
/// own debug) never touched.
#[test]
fn peer_debug_is_a_summary() {
    let peer: Peer<String> = Peer::seed();
    let rendered = format!("{peer:?}");
    for field in ["Peer", "network", "protocol", "latest", "len"] {
        assert!(rendered.contains(field), "missing {field}: {rendered}");
    }
}
