//! The formal model's skeleton vocabulary, and its decoders from session
//! observability.
//!
//! This is bridge support for the mux campaign's Rust proptest bridges
//! (`formal/MUX-ADJUDICATION.md` §4, stage-2 track D): a Rust mirror of the
//! Lean `Skel` (`formal/lean/StreamingMirror/Skel.lean`), the per-party view
//! projection and `LocalEq` (`formal/lean/StreamingMirror/Mux/Strategy.lean`,
//! `viewEnc`), the `wedge` witness shape
//! (`formal/lean/StreamingMirror/Mux/Instances.lean`), and two decoders that
//! extract a skeleton from a real session:
//!
//! - [`decode`] reads the materialized progress [`Trace`] (both endpoints'
//!   internal publications) and rebuilds the session's dispute skeleton,
//!   cross-checking every resolution's `pending` count and every event's
//!   endpoint against the model's role-parity and count laws (MODEL.md
//!   §3–§4) as it goes;
//! - [`announced`] reads the payload-erased wire [`Transcript`] alone — no
//!   tree, no internal events — and rebuilds the *announced* skeleton by
//!   replaying the protocol's positional pairing, which is exactly the
//!   reconstruction bridge B5 asks for (`formal/AUDIT-NOTES.md` A5).
//!
//! Deviations from the Lean, recorded: the mirror carries `scopes` and
//! `rootH` only. `Skel.fan` and `Skel.capLevel` are model configuration with
//! a single Rust value each (`FAN = 256` and the margin-0 discipline), so
//! their equality conjuncts in `LocalEq` are vacuous here; their per-witness
//! values are asserted separately ([`Skel::max_fan`], [`Skel::max_d_count`]).
//! `viewEnc`'s fuel is dropped: decoded skeletons are finite trees, so the
//! fuel-exhaustion token `0` is unreachable and the recursion is structural.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::tree::Root as TreeRoot;
use crate::tree::mirror::streaming::materialized::{
    progress::{Event, Kind as EventKind, Trace},
    transcript::{Label, Transcript},
};
use crate::tree::typed::height::{Height, Root as RootHeight};

/// The protocol's root height: 32 levels above the leaves, one per path byte.
pub(super) const ROOT_H: usize = RootHeight::HEIGHT;

// ------------------------------------------------------------------ skeleton

/// A skeleton scope's kind (`Skel.lean` `Kind`): two-sided dispute or
/// one-sided whole-subtree request. Matches (`M`) are dropped entirely.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Kind {
    /// Two-sided dispute: both parties hold the scope with differing hashes.
    D,
    /// One-sided request: the scope's answerer lacks it and asks for the
    /// whole subtree.
    R,
}

/// One scope of a dispute skeleton, in the flattened BFS encoding
/// (`Skel.lean` `Scope`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Scope {
    /// Two-sided dispute or one-sided request.
    pub kind: Kind,
    /// Height above the leaves (leaves 0, root [`ROOT_H`]).
    pub height: usize,
    /// The scope's skeleton children, as BFS ids, in radix order.
    pub kids: Vec<usize>,
    /// Height-1 scopes only: how many leaf requests the scope carries.
    pub leaf_reqs: usize,
}

/// A dispute skeleton (`Skel.lean` `Skel`, minus the `fan`/`capLevel`
/// configuration — module doc).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Skel {
    /// The scopes; index 0 is the root, ids BFS order.
    pub scopes: Vec<Scope>,
    /// The root scope's height.
    pub root_h: usize,
}

impl Skel {
    /// The widest fan any scope exhibits: the tight per-witness value of the
    /// Lean `Skel.fan` field (Rust reality bounds it by `FAN = 256`).
    pub fn max_fan(&self) -> usize {
        self.scopes
            .iter()
            .map(|scope| scope.kids.len().max(scope.leaf_reqs))
            .max()
            .unwrap_or(0)
    }

    /// The number of disputed children of scope `i` (`Skel.lean` `dCount`).
    pub fn d_count(&self, i: usize) -> usize {
        self.scopes[i]
            .kids
            .iter()
            .filter(|&&k| self.scopes[k].kind == Kind::D)
            .count()
    }

    /// The largest per-scope dispute count: the tight per-witness value of
    /// the Lean `Skel.capLevel` field under margin-0 (`wedge_margin0`).
    pub fn max_d_count(&self) -> usize {
        (0..self.scopes.len())
            .map(|i| self.d_count(i))
            .max()
            .unwrap_or(0)
    }
}

// -------------------------------------------------------------------- roles

/// A session role (`Skel.lean` `Party`): initiator or responder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Party {
    /// The initiator: asks about even-height scopes.
    I,
    /// The responder: asks about odd-height scopes.
    R,
}

impl Party {
    /// The counterparty.
    pub fn other(self) -> Party {
        match self {
            Party::I => Party::R,
            Party::R => Party::I,
        }
    }
}

/// Does party `p` ask (pair reply with query) for scopes at this height?
///
/// Initiator asks even heights, responder odd (`Skel.lean` `asks`;
/// MODEL.md §3's height-parity theorem).
pub(super) fn asks(p: Party, height: usize) -> bool {
    match p {
        Party::I => height.is_multiple_of(2),
        Party::R => !height.is_multiple_of(2),
    }
}

/// The role the client (the driver's first argument) will play against
/// `server`: a mirror of `streaming.rs::descend`'s canonical-byte tiebreak.
///
/// # Panics
///
/// If the advertised versions are equal: such a session short-circuits
/// without descending, so it has no skeleton to talk about.
pub(super) fn client_role<T>(client: &TreeRoot<T>, server: &TreeRoot<T>) -> Party {
    match server.ceiling.as_bytes().cmp(client.ceiling.as_bytes()) {
        std::cmp::Ordering::Less => Party::I,
        std::cmp::Ordering::Greater => Party::R,
        std::cmp::Ordering::Equal => panic!("equal versions short-circuit the descent"),
    }
}

// ------------------------------------------------------------ the projection

/// `viewEnc`'s open-bracket token for a D child (Lean literal `2`).
const TOKEN_OPEN: u8 = 2;
/// `viewEnc`'s close-bracket token for a D child (Lean literal `3`).
const TOKEN_CLOSE: u8 = 3;
/// `viewEnc`'s cut token for an asker-held R child (Lean literal `4`).
const TOKEN_CUT: u8 = 4;

/// Preorder token serialization of party `p`'s view of the skeleton: the
/// Rust mirror of `Mux/Strategy.lean` `viewEnc`.
///
/// Per child of each scope, in radix order: a D child emits an open bracket,
/// its own serialization, and a close bracket — held by `p` in both roles,
/// content recursed; an R child emits the cut token when `p` is the scope's
/// asker (a held cut: p's tree has the subtree, the skeleton does not) and
/// NOTHING when `p` is the answerer (a child the answerer lacks — invisible
/// at session start). `leafReqs` is never emitted (erased from both views).
pub(super) fn view_enc(p: Party, sk: &Skel) -> Vec<u8> {
    fn go(p: Party, sk: &Skel, i: usize, out: &mut Vec<u8>) {
        let scope = &sk.scopes[i];
        for &kid in &scope.kids {
            match sk.scopes[kid].kind {
                Kind::D => {
                    out.push(TOKEN_OPEN);
                    go(p, sk, kid, out);
                    out.push(TOKEN_CLOSE);
                }
                Kind::R => {
                    if asks(p, scope.height) {
                        out.push(TOKEN_CUT);
                    }
                }
            }
        }
    }

    let mut out = Vec::new();
    go(p, sk, 0, &mut out);
    out
}

/// Are two skeletons indistinguishable to party `p` at session start?
///
/// The Rust mirror of `Mux/Strategy.lean` `LocalEq`: equal root heights and
/// equal p-views. The Lean's `fan`/`capLevel` conjuncts are vacuous in Rust
/// (single fixed values — module doc), so they do not appear.
pub(super) fn local_eq(p: Party, a: &Skel, b: &Skel) -> bool {
    a.root_h == b.root_h && view_enc(p, a) == view_enc(p, b)
}

// ------------------------------------------------------------------ assembly

/// Assemble a [`Skel`] from classified scope prefixes, returning it with the
/// BFS id → byte-prefix table.
///
/// Validates the structural laws the Lean `wellFormed` demands of the
/// encoding: the root exists and is disputed, every scope's parent is a
/// disputed scope (R scopes are childless), and leaf requests sit under
/// height-1 disputed scopes only. BFS ids follow from sorting by (depth,
/// prefix bytes): within a level, lexicographic prefix order is exactly
/// (parent BFS order, radix order).
fn assemble(
    d: &BTreeSet<Vec<u8>>,
    r: &BTreeSet<Vec<u8>>,
    leaf_requests: &BTreeSet<Vec<u8>>,
) -> (Skel, Vec<Vec<u8>>) {
    assert!(
        d.contains(&Vec::new()),
        "the root scope is always disputed (MODEL.md §2)"
    );
    assert!(
        d.intersection(r).next().is_none(),
        "a scope cannot be both disputed and requested"
    );

    let mut prefixes: Vec<(Vec<u8>, Kind)> = d
        .iter()
        .map(|p| (p.clone(), Kind::D))
        .chain(r.iter().map(|p| (p.clone(), Kind::R)))
        .collect();
    prefixes.sort_by(|(a, _), (b, _)| (a.len(), a).cmp(&(b.len(), b)));

    let ids: BTreeMap<Vec<u8>, usize> = prefixes
        .iter()
        .enumerate()
        .map(|(i, (p, _))| (p.clone(), i))
        .collect();

    let mut scopes: Vec<Scope> = prefixes
        .iter()
        .map(|(prefix, kind)| Scope {
            kind: *kind,
            height: ROOT_H - prefix.len(),
            kids: Vec::new(),
            leaf_reqs: 0,
        })
        .collect();

    for (i, (prefix, _)) in prefixes.iter().enumerate().skip(1) {
        let parent = &prefix[..prefix.len() - 1];
        let pid = *ids
            .get(parent)
            .unwrap_or_else(|| panic!("scope {prefix:02x?} has no parent scope"));
        assert_eq!(
            scopes[pid].kind,
            Kind::D,
            "scope {prefix:02x?} hangs under a non-disputed parent: R scopes are childless"
        );
        scopes[pid].kids.push(i);
    }

    for leaf in leaf_requests {
        assert_eq!(leaf.len(), ROOT_H, "a leaf request names a full path");
        let parent = &leaf[..leaf.len() - 1];
        let pid = *ids
            .get(parent)
            .unwrap_or_else(|| panic!("leaf request {leaf:02x?} has no parent scope"));
        assert_eq!(
            (scopes[pid].kind, scopes[pid].height),
            (Kind::D, 1),
            "leaf requests sit under height-1 disputed scopes only"
        );
        scopes[pid].leaf_reqs += 1;
    }

    let table = prefixes.into_iter().map(|(p, _)| p).collect();
    (
        Skel {
            scopes,
            root_h: ROOT_H,
        },
        table,
    )
}

// ------------------------------------------------------------ trace decoding

/// A session's dispute skeleton as decoded from the progress trace, plus the
/// identities needed to talk about it.
#[derive(Debug)]
pub(super) struct Decoded {
    /// The session's dispute skeleton.
    pub skel: Skel,
    /// BFS id → scope byte prefix.
    pub prefixes: Vec<Vec<u8>>,
    /// The requested leaves' full paths (the `leafReqs` witnesses).
    pub leaf_requests: BTreeSet<Vec<u8>>,
    /// The initiator endpoint's work identity.
    pub initiator: usize,
}

/// Decode a completed two-endpoint session's dispute skeleton from its
/// progress trace, cross-checking the model's count and parity laws.
///
/// Classification: a scope is `D` iff its answerer published a `Resolution`
/// for it; `R` iff its supplier published a `Ready` at an internal prefix;
/// a `Ready` at a full 32-byte path is a supplied leaf request. On the way
/// out, every event is audited against MODEL.md: `Resolution` pending counts
/// are `d + r` (height ≥ 2) or `leafReqs` (height 1) and come from the
/// scope's answerer; `ParentResolution` pending counts are `d` and come from
/// the asker; suppliers, requesters, and dependent-work issuers land on the
/// endpoint the height-parity theorem (§3) assigns them.
pub(super) fn decode(trace: &Trace) -> Decoded {
    let events = trace.events();
    let works: BTreeSet<usize> = events.iter().map(|event| event.work).collect();
    assert_eq!(works.len(), 2, "a two-endpoint session has two work ids");

    let initials: Vec<&Event> = events
        .iter()
        .filter(|event| event.kind == EventKind::InitialQuery)
        .collect();
    let [initial] = initials.as_slice() else {
        panic!("expected exactly one InitialQuery, got {initials:?}");
    };
    assert!(initial.scope.is_empty(), "the initial query is the root's");
    let initiator = initial.work;
    let responder = *works.iter().find(|&&work| work != initiator).unwrap();

    // The endpoint the height-parity theorem assigns each role at height `h`.
    let asker_at = |h: usize| {
        if h.is_multiple_of(2) {
            initiator
        } else {
            responder
        }
    };
    let answerer_at = |h: usize| {
        if h.is_multiple_of(2) {
            responder
        } else {
            initiator
        }
    };

    let unique = |map: &mut BTreeMap<Vec<u8>, (usize, usize)>, event: &Event, pending| {
        let previous = map.insert(event.scope.clone(), (event.work, pending));
        assert!(previous.is_none(), "duplicate publication: {event:?}");
    };
    let mut resolutions = BTreeMap::new();
    let mut parents = BTreeMap::new();
    let mut readies = BTreeMap::new();
    let mut dependents = BTreeMap::new();
    for event in events {
        match event.kind {
            EventKind::Resolution { pending } => unique(&mut resolutions, event, pending),
            EventKind::ParentResolution { pending } => unique(&mut parents, event, pending),
            EventKind::Ready => unique(&mut readies, event, 0),
            EventKind::DependentWork => unique(&mut dependents, event, 0),
            EventKind::Wire | EventKind::InitialQuery => {}
        }
    }

    let d: BTreeSet<Vec<u8>> = resolutions.keys().cloned().collect();
    let r: BTreeSet<Vec<u8>> = readies
        .keys()
        .filter(|scope| scope.len() < ROOT_H)
        .cloned()
        .collect();
    let leaf_requests: BTreeSet<Vec<u8>> = readies
        .keys()
        .filter(|scope| scope.len() == ROOT_H)
        .cloned()
        .collect();
    let (skel, prefixes) = assemble(&d, &r, &leaf_requests);

    // Audit every publication against the model's laws.
    let ids: BTreeMap<&Vec<u8>, usize> = prefixes.iter().enumerate().map(|(i, p)| (p, i)).collect();
    for (i, prefix) in prefixes.iter().enumerate() {
        let scope = &skel.scopes[i];
        let h = scope.height;
        match scope.kind {
            Kind::D => {
                let d_kids = skel.d_count(i);
                let r_kids = scope.kids.len() - d_kids;
                let expected = if h == 1 {
                    scope.leaf_reqs
                } else {
                    d_kids + r_kids
                };
                assert_eq!(
                    resolutions[prefix],
                    (answerer_at(h), expected),
                    "answerer-side resolution of {prefix:02x?} (h={h}): pending = d + r, \
                     or leafReqs at height 1 (MODEL.md §4)"
                );
                assert_eq!(
                    parents[prefix],
                    (asker_at(h), d_kids),
                    "asker-side parent resolution of {prefix:02x?} (h={h}): pending = d"
                );
            }
            Kind::R => {
                // The supplier of a request is the parent scope's asker; the
                // requester (who asked, then assembles the supply) is the
                // parent's answerer.
                assert_eq!(
                    readies[prefix].0,
                    asker_at(h + 1),
                    "request {prefix:02x?} supplied by the parent's asker"
                );
                assert_eq!(
                    parents[prefix],
                    (answerer_at(h + 1), 0),
                    "request {prefix:02x?} resolved pending-free by the requester"
                );
            }
        }
        if !prefix.is_empty() {
            let parent_h = ROOT_H - (prefix.len() - 1);
            assert_eq!(
                dependents.get(prefix).map(|&(work, _)| work),
                Some(answerer_at(parent_h)),
                "scope {prefix:02x?} was asked exactly once, by its parent's answerer"
            );
        }
        let _ = ids;
    }
    assert!(
        !dependents.contains_key(&Vec::new()),
        "the root is never dependent work"
    );
    for leaf in &leaf_requests {
        // Height-1 scopes are answered by the initiator (MODEL.md §4): it
        // issues the leaf requests, and the responder supplies them.
        assert_eq!(
            dependents.get(leaf).map(|&(work, _)| work),
            Some(initiator),
            "leaf request {leaf:02x?} issued by the initiator"
        );
        assert_eq!(
            readies[leaf].0, responder,
            "leaf request {leaf:02x?} supplied by the responder"
        );
    }
    for key in parents.keys().chain(dependents.keys()) {
        assert!(
            d.contains(key) || r.contains(key) || leaf_requests.contains(key),
            "stray publication at {key:02x?}: not a skeleton scope or leaf request"
        );
    }

    Decoded {
        skel,
        prefixes,
        leaf_requests,
        initiator,
    }
}

// ------------------------------------------------------- transcript decoding

/// The announced skeleton as reconstructed from the wire transcript alone.
#[derive(Debug)]
pub(super) struct Announced {
    /// The announced dispute skeleton.
    pub skel: Skel,
    /// The endpoint the transcript itself identifies as the initiator.
    pub initiator: usize,
}

/// One expected reply, from the receiver's point of view: the positional
/// pairing state the reconstruction replays.
enum Question {
    /// A dispute: the asker announced these child radices for this scope.
    Dispute { prefix: Vec<u8>, radices: Vec<u8> },
    /// A whole-subtree request: the reply must be pure supplies.
    Request,
    /// A leaf request: the reply is that leaf's supply, or empty if pruned.
    LeafRequest { prefix: Vec<u8> },
}

/// Reconstruct the announced dispute skeleton from the payload-erased frame
/// transcript alone — no tree access, no internal events.
///
/// This is bridge B5 (`formal/AUDIT-NOTES.md` A5): the reconstruction
/// replays the protocol's positional pairing, so it works precisely because
/// every consumption-order discriminator is announced in-band. Per stream,
/// each reply answers the oldest unanswered question at its height; a
/// dispute reply's `Match`/`Query` reactions consume the asker's announced
/// radices in order (`Supply` reactions carry their own radix and consume
/// none); a nonempty `Query` listing announces a deeper dispute, an empty
/// one a whole-subtree request (a leaf request at leaf height). The global
/// capture order is causally consistent (transcript module doc), so
/// processing entries in order keeps every queue ahead of its consumer.
pub(super) fn announced(transcript: &Transcript) -> Announced {
    let entries = transcript.sent();
    let works: BTreeSet<usize> = entries.iter().map(|sent| sent.work).collect();
    assert_eq!(works.len(), 2, "a two-endpoint session has two work ids");
    let peer = |work: usize| *works.iter().find(|&&other| other != work).unwrap();

    // The opening: the initiator's root-listing question is causally first.
    let [opening, rest @ ..] = entries else {
        panic!("empty transcript");
    };
    assert_eq!(
        opening.height,
        ROOT_H - 1,
        "the opening rides the top stream"
    );
    let [Label::Query(root_radices)] = opening.labels.as_slice() else {
        panic!("the opening is a single root-listing query: {opening:?}");
    };
    let initiator = opening.work;

    let mut d = BTreeSet::from([Vec::new()]);
    let mut r = BTreeSet::new();
    let mut leaf_requests = BTreeSet::new();
    let mut queues: BTreeMap<(usize, usize), VecDeque<Question>> = BTreeMap::new();
    queues
        .entry((peer(initiator), ROOT_H - 1))
        .or_default()
        .push_back(Question::Dispute {
            prefix: Vec::new(),
            radices: root_radices.clone(),
        });

    for sent in rest {
        let question = queues
            .get_mut(&(sent.work, sent.height))
            .and_then(VecDeque::pop_front)
            .unwrap_or_else(|| panic!("reply without a pending question: {sent:?}"));
        match question {
            Question::Dispute { prefix, radices } => {
                let mut fan = radices.iter().copied();
                for label in &sent.labels {
                    match label {
                        // A match or an answerer-only supply is `M`: dropped
                        // from the skeleton, zero further channel ops.
                        Label::Match => {
                            fan.next().expect("Match beyond the announced listing");
                        }
                        Label::Supply(_) => {}
                        Label::Query(listing) => {
                            let radix = fan.next().expect("Query beyond the announced listing");
                            let mut child = prefix.clone();
                            child.push(radix);
                            if sent.height == 0 {
                                assert!(
                                    listing.is_empty(),
                                    "a leaf-height query is always a leaf request"
                                );
                                assert!(leaf_requests.insert(child.clone()));
                                queues
                                    .entry((peer(sent.work), 0))
                                    .or_default()
                                    .push_back(Question::LeafRequest { prefix: child });
                            } else if listing.is_empty() {
                                assert!(r.insert(child));
                                queues
                                    .entry((peer(sent.work), sent.height - 1))
                                    .or_default()
                                    .push_back(Question::Request);
                            } else {
                                assert!(d.insert(child.clone()));
                                queues
                                    .entry((peer(sent.work), sent.height - 1))
                                    .or_default()
                                    .push_back(Question::Dispute {
                                        prefix: child,
                                        radices: listing.clone(),
                                    });
                            }
                        }
                    }
                }
                assert!(
                    fan.next().is_none(),
                    "every announced radix draws a Match or Query reaction"
                );
            }
            Question::Request => {
                assert!(
                    sent.labels
                        .iter()
                        .all(|label| matches!(label, Label::Supply(_))),
                    "a whole-subtree request is answered by pure supplies: {sent:?}"
                );
            }
            Question::LeafRequest { prefix } => match sent.labels.as_slice() {
                [] => {}
                [Label::Supply(radix)] => {
                    assert_eq!(Some(radix), prefix.last(), "a leaf supply names its radix");
                }
                _ => panic!("a leaf request draws at most its own supply: {sent:?}"),
            },
        }
    }
    assert!(
        queues.values().all(VecDeque::is_empty),
        "every announced question was answered"
    );

    let (skel, _) = assemble(&d, &r, &leaf_requests);
    Announced { skel, initiator }
}

// ------------------------------------------------------- channel projections

/// A trace's per-channel projection: for each (endpoint, event kind, scope
/// depth) — one model channel instance — the ordered publications on it.
///
/// This is the granularity at which MODEL.md §1's payload-independence
/// premise speaks ("the count and order of CHANNEL operations"): the
/// cross-channel interleaving of one run is scheduler freedom the model
/// already quantifies over adversarially, and empirically it is not even a
/// function of the trees — the terminal `tokio::select!` in
/// `complete_initiator` is unbiased, so its branch order draws tokio's
/// thread-local RNG and varies run to run.
pub(super) fn trace_channels(
    trace: &Trace,
) -> BTreeMap<(usize, &'static str, usize), Vec<(Vec<u8>, usize)>> {
    let mut channels: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for event in trace.events() {
        let (kind, pending) = match event.kind {
            EventKind::Wire => ("wire", 0),
            EventKind::InitialQuery => ("initial-query", 0),
            EventKind::Resolution { pending } => ("resolution", pending),
            EventKind::DependentWork => ("dependent-work", 0),
            EventKind::Ready => ("ready", 0),
            EventKind::ParentResolution { pending } => ("parent-resolution", pending),
        };
        channels
            .entry((event.work, kind, event.scope.len()))
            .or_default()
            .push((event.scope.clone(), pending));
    }
    channels
}

/// A transcript's per-stream projection: for each (endpoint, stream height),
/// the ordered payload-erased replies it carried.
///
/// Per-stream order IS the wire order (transcript module doc); the
/// cross-stream interleaving is scheduler freedom, as for [`trace_channels`].
pub(super) fn transcript_streams(
    transcript: &Transcript,
) -> BTreeMap<(usize, usize), Vec<Vec<Label>>> {
    let mut streams: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for sent in transcript.sent() {
        streams
            .entry((sent.work, sent.height))
            .or_default()
            .push(sent.labels.clone());
    }
    streams
}

// ------------------------------------------------------------------ the wedge

/// The wedge witness shape at a chosen root height: the Rust mirror of the
/// Lean literal's generator.
///
/// The shape (`formal/lean/StreamingMirror/Mux/Instances.lean` `wedge`,
/// MUX-ADJUDICATION §1.2/§3 T0): root fan 7 — the FIRST radix child
/// deep-disputed, a single-child chain descending every level to a height-1
/// scope carrying one leaf request, with six whole-subtree provisions behind
/// it. At `root_h = 6` this is exactly the Lean literal
/// (`wedge_generator_matches_the_lean_literal`); at `root_h = 32` it is the
/// same shape at the protocol's real depth, which is what a session realizes.
pub(super) fn wedge(root_h: usize) -> Skel {
    assert!(root_h >= 3, "the chain needs depth of at least 2");
    let mut scopes = vec![
        Scope {
            kind: Kind::D,
            height: root_h,
            kids: (1..=7).collect(),
            leaf_reqs: 0,
        },
        Scope {
            kind: Kind::D,
            height: root_h - 1,
            kids: vec![8],
            leaf_reqs: 0,
        },
    ];
    scopes.extend((2..=7).map(|_| Scope {
        kind: Kind::R,
        height: root_h - 1,
        kids: Vec::new(),
        leaf_reqs: 0,
    }));
    for height in (1..=root_h - 2).rev() {
        let id = scopes.len();
        scopes.push(Scope {
            kind: Kind::D,
            height,
            kids: if height == 1 {
                Vec::new()
            } else {
                vec![id + 1]
            },
            leaf_reqs: usize::from(height == 1),
        });
    }
    Skel { scopes, root_h }
}
