//! Resource envelopes for ticking versions across adversarial shapes.

use super::*;

// ─── tick scenarios ─────────────────────────────────────────────────────────
//
// The public tick and multi-tick on the tick-designated families: the
// fused walk, route fold, and splice in one pass. The tick walk's cost
// currency is accumulator digit touches, with scanned bits beside it.

/// Ticking the dense spine stays within its envelope: the fused tick's
/// one walk and splice, with the output buffer deferred past the
/// collapse scan.
#[test]
fn tick_dense_envelope() {
    let p = Shape::Dense.build1(DENSE_DEPTH);
    let mut v = version_of(&p);
    let seed = Party::seed();
    metered("tick_dense", p.bytes.len(), &query_env::TICK_DENSE, || {
        v.tick(&seed)
    });
    drop(v);
}

/// Ticking the wide right-full chain (a bigroot magnitude over the
/// nested-full id) stays within its envelope: the range-minimum tracker walk
/// touches the wide first payload O(1) times, never once per shortcut
/// level.
#[test]
fn tick_nested_wide_envelope() {
    let ev = Shape::Bigroot.build2(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
    let id = Shape::NestedFullId.build1(TICK_CROSS_SCALE);
    let mut v = version_of(&ev);
    let p = party_of(&id);
    let input = ev.bytes.len() + id.bytes.len();
    metered(
        "tick_nested_wide",
        input,
        &query_env::TICK_NESTED_WIDE,
        || v.tick(&p),
    );
    drop(v);
}

/// Ticking the wide memo chain (a wide-tail spine under the
/// nested-left-full id) stays within its envelope: the pre-scan's
/// frame ledger stores no link for the shared wide minimum, so
/// nothing is materialized per site.
#[test]
fn tick_mirror_wide_envelope() {
    let ev = Shape::WideTail.build2(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
    let id = Shape::NestedLeftFullId.build1(TICK_CROSS_SCALE);
    let mut v = version_of(&ev);
    let p = party_of(&id);
    let input = ev.bytes.len() + id.bytes.len();
    metered(
        "tick_mirror_wide",
        input,
        &query_env::TICK_MIRROR_WIDE,
        || v.tick(&p),
    );
    drop(v);
}

/// Ticking the descending staircase under a party owning one deep
/// diverted fragment (the ownership-hole family) stays within an
/// envelope the leaf-by-leaf walk exceeds.
///
/// The tick walk's unowned regions are whole staircase runs, and the
/// block scan must fold each into O(1) accumulator work instead of
/// per-leaf work. The touch ceiling is the skip's liveness signal
/// — it sits below the per-leaf mechanism's reading, so the fast path
/// must demonstrably engage; the scan column pins that every skipped
/// bit is still read.
#[test]
fn tick_ownership_hole_envelope() {
    let ev = Shape::Staircase.build1(HOLE_STAIR_DEPTH);
    let id = Shape::IdSpine.build_flagged(HOLE_ID_DEPTH, true);
    let mut v = version_of(&ev);
    let p = party_of(&id);
    let input = ev.bytes.len() + id.bytes.len();
    metered(
        "tick_ownership_hole",
        input,
        &query_env::TICK_OWNERSHIP_HOLE,
        || v.tick(&p),
    );
    drop(v);
}

/// Ticking the alternating spine under the scattered id (the
/// alternating-ownership comb: owned fragments and absent gaps
/// interleaved at every level, so every unowned region is a single
/// leaf) stays within its envelope.
///
/// The comb is the region gate's worst case — the block scan can
/// never engage — and the pin holds the gated walk to the per-leaf
/// walk's own readings: a gate that costs anything when closed moves
/// this envelope.
#[test]
fn tick_ownership_comb_envelope() {
    let ev = Shape::AltSpine.build1(DENSE_DEPTH);
    let id = Shape::ScatteredId.build1(COMB_FRAGMENTS);
    let mut v = version_of(&ev);
    let p = party_of(&id);
    let input = ev.bytes.len() + id.bytes.len();
    metered(
        "tick_ownership_comb",
        input,
        &query_env::TICK_OWNERSHIP_COMB,
        || v.tick(&p),
    );
    drop(v);
}

/// Ticking the collapse-hole pair (deep descending collapse ranges under
/// left-full sites with absent siblings) stays within an envelope the
/// per-leaf consuming max scan exceeds.
///
/// Each unit's fully-owned range is crossed exactly once, by the walk's
/// consuming max scan at its descend arm, and the crossing must ride the
/// block summary: the touch ceiling sits below what per-leaf register
/// work over the same ranges reads, and the scan column holds every
/// folded bit still read.
#[test]
fn tick_collapse_hole_envelope() {
    let (ev, id) = Shape::CollapseHole.build_pair(SCAN_HOLE_UNITS, SCAN_HOLE_STEPS);
    let mut v = version_of(&ev);
    let p = party_of(&id);
    let input = ev.bytes.len() + id.bytes.len();
    metered(
        "tick_collapse_hole",
        input,
        &query_env::TICK_COLLAPSE_HOLE,
        || v.tick(&p),
    );
    drop(v);
}

/// Ticking the copy-hole pair (deep descending absent-child ranges inside
/// one covering pre-scan) stays within an envelope the per-leaf sub-scan
/// mechanism exceeds.
///
/// Each unit's untouched range is copied once by the pre-scan, and the
/// copy must ride the block summary — one net movement and one watermark
/// emission per range, never a virtual emission per leaf: the touch
/// ceiling sits below the per-leaf mechanism's reading, and the scan
/// column holds every folded bit still read.
#[test]
fn tick_copy_hole_envelope() {
    let (ev, id) = Shape::CopyHole.build_pair(SCAN_HOLE_UNITS, SCAN_HOLE_STEPS);
    let mut v = version_of(&ev);
    let p = party_of(&id);
    let input = ev.bytes.len() + id.bytes.len();
    metered("tick_copy_hole", input, &query_env::TICK_COPY_HOLE, || {
        v.tick(&p)
    });
    drop(v);
}

/// Ticking the site-hole pair (deep descending collapse ranges under
/// interior left-full sites inside one covering pre-scan) stays within an
/// envelope the extremum-streaming block fold exceeds.
///
/// Each unit's collapse range is crossed exactly twice — once by the
/// pre-scan's collapse skip, once by the walk's consuming max scan at the
/// site's own consume — and the pre-scan's crossing owes the web only the
/// range's net height movement: the touch ceiling sits below what a block
/// fold that also streams the range's unread minimum reads over the same
/// ranges, and the scan column holds every folded bit still read.
#[test]
fn tick_site_hole_envelope() {
    let (ev, id) = Shape::SiteHole.build_pair(SCAN_HOLE_UNITS, SCAN_HOLE_STEPS);
    let mut v = version_of(&ev);
    let p = party_of(&id);
    let input = ev.bytes.len() + id.bytes.len();
    metered("tick_site_hole", input, &query_env::TICK_SITE_HOLE, || {
        v.tick(&p)
    });
    drop(v);
}

/// Ticking the raise-hole pair (deep descending raised ranges under
/// right-full sites) stays within an envelope the per-leaf consuming max
/// scan exceeds.
///
/// Each unit's fully-owned right range is crossed exactly once, by the
/// walk's consuming max scan at its ascend arm, and the crossing must
/// ride the block summary: the touch ceiling sits below the per-leaf
/// mechanism's reading, and the scan column holds every folded bit still
/// read.
#[test]
fn tick_raise_hole_envelope() {
    let (ev, id) = Shape::RaiseHole.build_pair(SCAN_HOLE_UNITS, SCAN_HOLE_STEPS);
    let mut v = version_of(&ev);
    let p = party_of(&id);
    let input = ev.bytes.len() + id.bytes.len();
    metered(
        "tick_raise_hole",
        input,
        &query_env::TICK_RAISE_HOLE,
        || v.tick(&p),
    );
    drop(v);
}

/// The fused multi-tick on the dense spine stays within its envelope:
/// registering [`TICKS_POINT_LO`] events costs the single tick's walk and splice plus
/// only the count's gamma-width boundary codes.
#[test]
fn ticks_dense_envelope() {
    let p = Shape::Dense.build1(DENSE_DEPTH);
    let mut v = version_of(&p);
    let seed = Party::seed();
    metered(
        "ticks_dense",
        p.bytes.len(),
        &query_env::TICKS_DENSE,
        || v.ticks(&seed, TICKS_POINT_LO),
    );
    drop(v);
}

/// The fused multi-tick on the wide right-full chain stays within its
/// envelope: the `+n` splice compounds at the same site the single
/// tick's does, and the wide first payload is still touched O(1) times.
#[test]
fn ticks_nested_wide_envelope() {
    let ev = Shape::Bigroot.build2(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
    let id = Shape::NestedFullId.build1(TICK_CROSS_SCALE);
    let mut v = version_of(&ev);
    let p = party_of(&id);
    let input = ev.bytes.len() + id.bytes.len();
    metered(
        "ticks_nested_wide",
        input,
        &query_env::TICKS_NESTED_WIDE,
        || v.ticks(&p, TICKS_POINT_LO),
    );
    drop(v);
}

/// The fused multi-tick on the wide memo chain stays within its
/// envelope: the pre-scan's frame ledger behaves exactly as the single
/// tick's, count notwithstanding.
#[test]
fn ticks_mirror_wide_envelope() {
    let ev = Shape::WideTail.build2(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
    let id = Shape::NestedLeftFullId.build1(TICK_CROSS_SCALE);
    let mut v = version_of(&ev);
    let p = party_of(&id);
    let input = ev.bytes.len() + id.bytes.len();
    metered(
        "ticks_mirror_wide",
        input,
        &query_env::TICKS_MIRROR_WIDE,
        || v.ticks(&p, TICKS_POINT_LO),
    );
    drop(v);
}

// ─── grow-branch tick scenarios ─────────────────────────────────────────────
//
// The deep expansion shapes: pairs whose fill is the identity, so the
// fused tick records the inflation route on its one walk and replays it
// through the splice emit. These rows measure the whole public tick —
// walk, route fold, and splice — on the shapes whose id side dominates.

/// The version that is `1` on the leftmost `2^-depth` interval and `0`
/// everywhere else: `depth` nested nodes, all bases zero, the single
/// 1-leaf at the bottom left.
///
/// This is what ticking a version that is zero over the owned region
/// registers for a depth-`depth` unary party spine.
fn left_spike(depth: usize) -> Version {
    let mut party = Party::seed();
    for _ in 0..depth {
        let _ = party.fork();
    }
    (&uniform_version(1u8) / &party).to_version()
}

/// Ticking the empty version under a 250k-deep unary id spine stays
/// within its envelope.
///
/// The walk is one event leaf whose route fold is the iterative id
/// scan (bit-stack frames, nothing recurses), and the emit codes the
/// whole expansion chain as fresh one-bit deltas. The value witness is
/// closed-form: the expansion chain to the owned tip is exactly the
/// left spike above.
#[test]
fn tick_expand_spine_envelope() {
    let mut v = Version::new();
    let party = party_of(&Shape::IdSpine.build_flagged(ID_DEPTH, false));
    // Byte sizes of buffers this test just allocated fit `usize`.
    let input = (v.as_bytes().len() as u64 + party.encoded_bits().div_ceil(8)) as usize;
    metered(
        "tick_expand_spine",
        input,
        &query_env::TICK_EXPAND_SPINE,
        || v.tick(&party),
    );
    assert_eq!(
        v,
        left_spike(ID_DEPTH),
        "the ticked version must be the derived closed form"
    );
}

/// Ticking the alternating spine under a deep unary id spine stays
/// within its envelope.
///
/// The regimes mix — the two-cursor fused walk down the shared spine,
/// an id-only expansion fold where the id outruns the event — and the
/// splice replays the recorded route. The value witness is closed-form:
/// the unary id turns left into the spine's depth-2 zero leaf, so the
/// forced route raises exactly the owned region from 0 to 1 — the
/// pointwise max with the left spike, realized through the
/// independently-tested join, byte-exact by canonical uniqueness.
#[test]
fn tick_expand_cross_envelope() {
    let ev = Shape::AltSpine.build1(DENSE_DEPTH);
    let mut v = version_of(&ev);
    let party = party_of(&Shape::IdSpine.build_flagged(ID_DEPTH, false));
    let expected = &v | &left_spike(ID_DEPTH);
    // Byte sizes of buffers this test just allocated fit `usize`.
    let input = ev.bytes.len() + party.encoded_bits().div_ceil(8) as usize;
    metered(
        "tick_expand_cross",
        input,
        &query_env::TICK_EXPAND_CROSS,
        || v.tick(&party),
    );
    assert_eq!(
        v, expected,
        "the ticked version must be the derived closed form"
    );
}
