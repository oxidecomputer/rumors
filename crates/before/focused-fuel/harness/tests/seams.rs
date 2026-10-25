//! The minimum-tick fold's wide-hop guards and parked-latent decision, at
//! the knobs the board's propagate-seam and latent-ladder families hold
//! fixed.

use focused_fuel_harness::{assert_flat, assert_less, assert_mixed_below, measure_per, Probe};

/// Sites of the seam probes' small runs; the large runs double them.
const SMALL_SITES: u64 = 512;

/// The plunge residue's digits at the guards' minimal clearance: the
/// three-digit boundaries plus the two digits that prove domination without
/// subtraction.
const TIGHT_CLEARANCE: u64 = 5;

/// The clearance band's second point: five digits of daylight, every dying
/// width unchanged.
const WIDE_CLEARANCE: u64 = 10;

/// The residue widths of the hop-guard grid, in digits: ten, and sixty-four
/// times that.
const GRID_RESIDUES: [u64; 2] = [10, 640];

/// The descending cascade costs a constant per site whatever the residue's
/// width, and each hop costs less than a drain park of the same boundary.
///
/// The domination hops never read the surviving residue across its width.
///
/// A hop that read the residue's width would add at least one instruction
/// per site per residue digit, so over sites 1,024 and 2,048 and residue
/// widths 10 and 640 the plunge's mixed second difference would reach
/// `1024 · 630`. The residue's own wide codes cost input-proportional work
/// once per run, which an additive term absorbs. The bound sits at half that
/// term, so it catches the term whatever amortized wobble of either sign the
/// honest cost carries, up to half the term. The two runs share every
/// consume and arm, so the cost each extra site adds, the k-marginal, isolates
/// the hops against the parks. Forms: flat per unit (F-lin) per encoded byte
/// across a site doubling at the minimal clearance; no product term (F-mix)
/// below half that derived term; and strict saving (F-lt) of the plunge's
/// k-marginal under the control's, at each width.
#[test]
fn descending_cascade_costs_a_constant_per_site() {
    assert_flat(
        "seam plunge per byte",
        measure_per(Probe::SeamPlunge, SMALL_SITES, TIGHT_CLEARANCE),
        measure_per(Probe::SeamPlunge, 2 * SMALL_SITES, TIGHT_CLEARANCE),
    );
    let sites = [2 * SMALL_SITES, 4 * SMALL_SITES];
    let reading = |probe: Probe, k: u64, r: u64| measure_per(probe, k, r).0;
    let [narrow, wide] = GRID_RESIDUES;
    assert_mixed_below(
        "seam plunge over sites and residue width",
        [
            reading(Probe::SeamPlunge, sites[0], narrow),
            reading(Probe::SeamPlunge, sites[1], narrow),
            reading(Probe::SeamPlunge, sites[0], wide),
            reading(Probe::SeamPlunge, sites[1], wide),
        ],
        i128::from(sites[1] - sites[0]) * i128::from(wide - narrow) / 2,
    );
    for r in GRID_RESIDUES {
        let marginal = |probe: Probe| reading(probe, sites[1], r) - reading(probe, sites[0], r);
        assert_less(
            &format!("plunge vs control cost per extra 1,024 sites at residue width {r}"),
            marginal(Probe::SeamPlunge),
            marginal(Probe::SeamPlungeControl),
        );
    }
}

/// The wide-hop guards decide at their minimal clearance: fuel per byte
/// agrees within ×1.25 in both directions between hops with two digits of
/// daylight and hops with five, at equal site count and dying widths.
///
/// A guard that demanded more daylight would reroute exactly the
/// minimal-clearance point's hops onto the comparable-scale fold, inflating
/// it against the wide point. Form: flat per unit (F-lin) per encoded byte,
/// in both directions.
#[test]
fn wide_hop_guards_decide_at_their_minimal_clearance() {
    let tight = measure_per(Probe::SeamPlunge, 2 * SMALL_SITES, TIGHT_CLEARANCE);
    let wide = measure_per(Probe::SeamPlunge, 2 * SMALL_SITES, WIDE_CLEARANCE);
    assert_flat("tight over wide clearance per byte", wide, tight);
    assert_flat("wide over tight clearance per byte", tight, wide);
}

/// The stopping arm uses O(1) work per hop beside the retiring fold: the
/// stop's cost, and its excess over its control, stay flat per site.
///
/// The two runs share every consume, arm, and residue; the excess holds the
/// stop's hops, freezes, and merges, so a per-hop read of the surviving
/// boundary's width lands whole in it. Form: flat per unit (F-lin) of the
/// stop per encoded byte, and of the excess per site, across a site
/// doubling.
#[test]
fn stopping_arm_is_flat_per_site() {
    let stop = [SMALL_SITES, 2 * SMALL_SITES].map(|sites| measure_per(Probe::SeamStop, sites, 0));
    let control =
        [SMALL_SITES, 2 * SMALL_SITES].map(|sites| measure_per(Probe::SeamStopControl, sites, 0));
    assert_flat("seam stop per byte", stop[0], stop[1]);
    let excess = |index: usize| {
        stop[index]
            .0
            .checked_sub(control[index].0)
            .expect("the stop does at least its control's work")
    };
    assert_flat(
        "seam stop excess per site",
        (excess(0), SMALL_SITES),
        (excess(1), 2 * SMALL_SITES),
    );
}

/// Width of the latent ladder's parked latent at the narrow point, in
/// digits; the wide point doubles it.
const LADDER_WIDTH: u64 = 64;

/// Decision leaves of the latent ladder's base runs; the marginal doubles
/// them.
const LADDER_DECISIONS: u64 = 512;

/// The parked-latent undercut decision answers scale-disparate drops in
/// O(1): the marginal cost of the extra decision leaves agrees within ×1.25,
/// in both directions, across a doubling of the parked latent's width.
///
/// The marginal `T(w, 2k) − T(w, k)` isolates the extra decision leaves, as
/// the parked boundary's own work cancels; a decision, or a reference merge,
/// that read the latent across its width would double the marginal with the
/// width. Form: flat per unit (F-lin) of the marginal, in both directions.
#[test]
fn latent_ladder_decisions_are_independent_of_the_latent_width() {
    let marginal = |width: u64| {
        let base = measure_per(Probe::LatentLadder, width, LADDER_DECISIONS).0;
        let doubled = measure_per(Probe::LatentLadder, width, 2 * LADDER_DECISIONS).0;
        doubled
            .checked_sub(base)
            .expect("more decisions cost more fuel")
    };
    let (narrow, wide) = (marginal(LADDER_WIDTH), marginal(2 * LADDER_WIDTH));
    assert_flat("wide over narrow marginal", (narrow, 1), (wide, 1));
    assert_flat("narrow over wide marginal", (wide, 1), (narrow, 1));
}
