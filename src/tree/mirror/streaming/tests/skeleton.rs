//! A session's dispute skeleton, its per-party views, and its decoders.
//!
//! This module defines the dispute skeleton ([`Skel`]), each party's view of
//! it ([`view_enc`] and [`local_eq`]), and the one-dispute-ahead-of-six-supplies
//! shape ([`dispute_before_supplies`]). Two independent decoders extract a
//! skeleton from a session:
//!
//! - [`decode`] reads the materialized progress [`Trace`] (both endpoints'
//!   internal publications), checking resolution counts and endpoint roles
//!   against [`asks`] and the per-scope pending-count laws;
//! - [`announced`] reads only the payload-erased reply [`Transcript`] and
//!   reconstructs the same skeleton from choices announced by the protocol.
//!
//! A skeleton records topology only: fan is fixed by `FAN`, and witness tests
//! check a skeleton's widest fan and largest per-scope dispute count through
//! [`Skel::max_fan`] and [`Skel::max_d_count`].

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::left_initiates;
use crate::tree::Root as TreeRoot;
use crate::tree::mirror::streaming::materialized::{
    progress::{Event, Kind as EventKind, Trace},
    transcript::{Label, Transcript},
};
use crate::tree::typed::height::{Height, Root as RootHeight};

/// The protocol's root height: 32 levels above the leaves, one per path byte.
pub(super) const ROOT_H: usize = RootHeight::HEIGHT;

// ------------------------------------------------------------------ skeleton

/// A skeleton scope's kind: two-sided dispute or one-sided whole-subtree
/// request. Matches (`M`) are dropped entirely.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Kind {
    /// Two-sided dispute: both parties hold the scope with differing hashes.
    D,
    /// One-sided request: the scope's answerer lacks it and asks for the
    /// whole subtree.
    R,
}

/// One scope of a dispute skeleton, in the flattened BFS encoding.
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

/// A session's dispute skeleton: its disputed and requested scopes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Skel {
    /// The scopes; index 0 is the root, ids BFS order.
    pub scopes: Vec<Scope>,
    /// The root scope's height.
    pub root_h: usize,
}

impl Skel {
    /// The widest fan any scope exhibits, bounded by the protocol's `FAN`.
    pub fn max_fan(&self) -> usize {
        self.scopes
            .iter()
            .map(|scope| scope.kids.len().max(scope.leaf_reqs))
            .max()
            .unwrap_or(0)
    }

    /// The number of disputed children of scope `i`.
    pub fn d_count(&self, i: usize) -> usize {
        self.scopes[i]
            .kids
            .iter()
            .filter(|&&k| self.scopes[k].kind == Kind::D)
            .count()
    }

    /// The largest per-scope dispute count: the most disputed children any
    /// one scope has.
    pub fn max_d_count(&self) -> usize {
        (0..self.scopes.len())
            .map(|i| self.d_count(i))
            .max()
            .unwrap_or(0)
    }
}

// -------------------------------------------------------------------- roles

/// A session role: initiator or responder.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
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
/// Initiator asks even heights, responder odd. [`decode`] checks that every
/// publication in a session's trace lands on the endpoint this parity assigns.
pub(super) fn asks(p: Party, height: usize) -> bool {
    match p {
        Party::I => height.is_multiple_of(2),
        Party::R => !height.is_multiple_of(2),
    }
}

/// The role the client (the driver's first argument) will play against
/// `server`: a mirror of `streaming.rs::descend`'s election (the smaller
/// set initiates, canonical version bytes break ties).
///
/// # Panics
///
/// If the advertised versions are equal: such a session short-circuits
/// without descending, so it has no skeleton to talk about.
pub(super) fn client_role(client: &TreeRoot, server: &TreeRoot) -> Party {
    if left_initiates(client, server) {
        Party::I
    } else {
        Party::R
    }
}

// ------------------------------------------------------------ the projection

/// [`view_enc`]'s open-bracket token for a D child.
const TOKEN_OPEN: u8 = 2;
/// [`view_enc`]'s close-bracket token for a D child.
const TOKEN_CLOSE: u8 = 3;
/// [`view_enc`]'s cut token for an asker-held R child.
const TOKEN_CUT: u8 = 4;

/// Preorder token serialization of party `p`'s view of the skeleton.
///
/// Per child of each scope, in radix order: a D child emits an open bracket,
/// its own serialization, and a close bracket — held by `p` in both roles,
/// content recursed; an R child emits the cut token when `p` is the scope's
/// asker (a held cut: p's tree has the subtree, the skeleton does not) and
/// NOTHING when `p` is the answerer (a child the answerer lacks — invisible
/// at session start). `leaf_reqs` is never emitted (erased from both views).
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
/// They are when their root heights are equal and their p-views
/// ([`view_enc`]) are equal.
pub(super) fn local_eq(p: Party, a: &Skel, b: &Skel) -> bool {
    a.root_h == b.root_h && view_enc(p, a) == view_enc(p, b)
}

// ------------------------------------------------------------------ assembly

/// Assemble a [`Skel`] from classified scope prefixes, returning it with the
/// BFS id → byte-prefix table.
///
/// Validates the encoding's structural laws: the root exists and is
/// disputed, every scope's parent is a disputed scope (R scopes are
/// childless), and leaf requests sit under height-1 disputed scopes only.
/// BFS ids follow from sorting by (depth, prefix bytes): within a level,
/// lexicographic prefix order is exactly (parent BFS order, radix order).
fn assemble(
    d: &BTreeSet<Vec<u8>>,
    r: &BTreeSet<Vec<u8>>,
    leaf_requests: &BTreeSet<Vec<u8>>,
) -> (Skel, Vec<Vec<u8>>) {
    assert!(d.contains(&Vec::new()), "the root scope is always disputed");
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
    /// The requested leaves' full paths (the leaves each `leaf_reqs` counts).
    pub leaf_requests: BTreeSet<Vec<u8>>,
    /// The initiator endpoint's work identity.
    pub initiator: usize,
}

/// Decode a completed two-endpoint session's dispute skeleton from its
/// progress trace, cross-checking the skeleton's count and parity laws.
///
/// Classification: a scope is `D` iff its answerer published a `Resolution`
/// for it; `R` iff its supplier published a `Ready` at an internal prefix;
/// a `Ready` at a full 32-byte path is a supplied leaf request. On the way
/// out, every event is audited against these count and parity laws:
/// `Resolution` pending counts are `d + r` (height ≥ 2) or `leaf_reqs`
/// (height 1) and come from the scope's answerer; `ParentResolution` pending
/// counts are `d` and come from the asker; suppliers, requesters, and
/// dependent-work issuers land on the endpoint the height parity ([`asks`])
/// assigns them.
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

    // Translate the height-parity role predicate into work identities.
    let asker_at = |height| {
        if asks(Party::I, height) {
            initiator
        } else {
            responder
        }
    };
    let answerer_at = |height| {
        if asks(Party::I, height) {
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
            EventKind::Wire | EventKind::Chunk | EventKind::InitialQuery => {}
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

    // Audit every publication against the count and parity laws.
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
                     or leaf_reqs at height 1"
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
    }
    assert!(
        !dependents.contains_key(&Vec::new()),
        "the root is never dependent work"
    );
    for leaf in &leaf_requests {
        // Height-1 scopes are answered by the initiator (height 1 is odd,
        // and `asks` gives odd heights to the responder to ask): it
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

/// The announced skeleton reconstructed from outgoing replies alone.
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

/// Reconstruct the announced dispute skeleton from payload-erased replies,
/// without tree access or internal events.
///
/// The reconstruction replays the protocol's positional pairing. Each reply
/// answers the oldest unanswered question on its stream. `Match` and `Query`
/// consume the asker's radices in order, while `Supply` carries its own radix.
/// A nonempty query announces a deeper dispute; an empty one requests a whole
/// subtree, or a leaf at leaf height. The transcript's causal order keeps each
/// question ahead of its answer.
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
    // The opening is the root-listing query alone.
    let [Label::Query(root_radices)] = opening.labels.as_slice() else {
        panic!("the opening is the root-listing query alone: {opening:?}");
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

/// One progress channel, with event payloads removed.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Channel {
    /// Replies published by the walk.
    Reply,
    /// The initiator's opening query.
    InitialQuery,
    /// A scope resolution.
    Resolution,
    /// Work released by a preceding resolution.
    DependentWork,
    /// A whole-subtree supply becoming ready.
    Ready,
    /// A completed child returned to its parent.
    ParentResolution,
}

/// Group publications by channel: endpoint, event kind, and scope depth.
///
/// Preserve order within each channel. The payload-independence comparison
/// permits different interleavings between channels, which remain scheduler
/// freedom.
pub(super) fn trace_channels(
    trace: &Trace,
) -> BTreeMap<(usize, Channel, usize), Vec<(Vec<u8>, usize)>> {
    let mut channels: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for event in trace.events() {
        let (kind, pending) = match event.kind {
            EventKind::Wire => (Channel::Reply, 0),
            EventKind::InitialQuery => (Channel::InitialQuery, 0),
            EventKind::Resolution { pending } => (Channel::Resolution, pending),
            EventKind::DependentWork => (Channel::DependentWork, 0),
            EventKind::Ready => (Channel::Ready, 0),
            EventKind::ParentResolution { pending } => (Channel::ParentResolution, pending),
            // A reply's later chunks ride the channel its first chunk's wire
            // action already entered.
            EventKind::Chunk => continue,
        };
        channels
            .entry((event.work, kind, event.scope.len()))
            .or_default()
            .push((event.scope.clone(), pending));
    }
    channels
}

/// A transcript's per-stream projection: the ordered payload-erased replies
/// for each endpoint and stream height.
///
/// Cross-stream interleaving remains scheduler freedom, as for
/// [`trace_channels`].
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

// ---------------------------------------- one dispute ahead of six supplies

/// One dispute ahead of six whole-subtree supplies, at a chosen root height.
///
/// The shape: root fan 7 — the first radix child deep-disputed, a
/// single-child chain descending every level to a height-1 scope carrying one
/// leaf request, with six whole-subtree provisions behind it. At
/// `root_h = 6` this equals a hand-written literal
/// (`generator_matches_the_literal_at_height_six`); at `root_h = 32` it is the
/// same shape at the protocol's real depth, which is what a session realizes.
pub(super) fn dispute_before_supplies(root_h: usize) -> Skel {
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
