//! Laws of clocks.
//!
//! These laws connect each clock's party and version while checking ticking,
//! sending, receiving, synchronization, joining, splitting, codecs, and
//! variadic conservation.

use super::*;

laws! {
    /// Laws over one clock.
    ///
    /// The clock model's composite operations:
    /// `fork` preserves the version and splits the party, the balanced
    /// n-way fork's two forms agree, `tick`/`send` advance strictly and fix
    /// the party, `ticks` agrees with the version entry point, peeks are
    /// stable, an own-message receive is a bare tick, an absorb is the
    /// anonymous join with no event minted, `sync` reconciles a fork,
    /// `own_version` is the projection, and the parts/codec/text
    /// round-trips.
    pub static CLOCK_SOLO: (c: &Clock);

    /// `fork` preserves the version on both halves (§3: fork clones the causal
    /// past).
    fn fork_preserves_version {
        let mut keeper = c.dangerously_alias();
        let child = keeper.fork();
        keeper.version() == c.version() && child.version() == c.version()
    }

    /// `fork` splits the party: the two halves are disjoint, and each is
    /// covered by the original.
    fn fork_splits_the_party {
        let mut keeper = c.dangerously_alias();
        let child = keeper.fork();
        keeper.party().is_disjoint(child.party())
            && c.party().covers(keeper.party())
            && c.party().covers(child.party())
    }

    /// `fork` then `join` restores the clock exactly: the party halves rejoin
    /// and the version join is idempotent (`e ⊔ e == e`).
    fn fork_join_restores_the_clock {
        let mut keeper = c.dangerously_alias();
        let child = keeper.fork();
        keeper.join(child).is_ok() && keeper == *c
    }

    /// The two balanced-fork forms agree at the clock level: `From<Clock>`
    /// for `[Clock; N]` equals the residual the borrowing `forks(N - 1)`
    /// keeps, followed by the shares it yields (`[residual] ++ forks`).
    ///
    /// Every child pairs its party share with a clone of the parent
    /// version.
    fn clock_forks_matches_from_array {
        const N: usize = 4;
        let array: [Clock; N] = c.dangerously_alias().into();
        let mut keeper = c.dangerously_alias();
        let yielded: Vec<Clock> = keeper.forks(N as u64 - 1).collect();
        let reconstructed: Vec<Clock> = std::iter::once(keeper).chain(yielded).collect();
        array.iter().eq(reconstructed.iter())
    }

    /// `version()` (peek) does not advance the clock: repeated peeks are equal
    /// and the clock's bytes are unchanged.
    fn peek_is_stable {
        let before = c.encode();
        let first = c.version().clone();
        first == *c.version() && c.encode() == before
    }

    /// `tick` strictly advances the version and leaves the party untouched.
    fn clock_tick_advances_and_fixes_party {
        let mut ticked = c.dangerously_alias();
        ticked.tick();
        le(c.version(), ticked.version())
            && c.version() != ticked.version()
            && ticked.party() == c.party()
    }

    /// `receive` of a dominated message (here the clock's own version) equals a
    /// bare `tick`: an own-message receive is benign.
    fn own_receive_is_tick {
        let mut received = c.dangerously_alias();
        let mut ticked = c.dangerously_alias();
        let own = received.version().clone();
        received.recv(&own);
        ticked.tick();
        received == ticked
    }

    /// The clock's `ticks` agrees with the version-level `ticks` on its own
    /// parts, and returns the freshly advanced version.
    fn clock_ticks_matches_version_ticks {
        let n = Ticks::from(3u64);
        let mut via_clock = c.dangerously_alias();
        let returned = via_clock.ticks(n.clone()).clone();
        let mut expected = c.version().clone();
        expected.ticks(c.party(), n);
        returned == expected && *via_clock.version() == expected
    }

    /// `send` (event then peek) returns the freshly advanced version: the
    /// returned message equals the clock's version and strictly dominates the
    /// pre-send version.
    fn send_advances_and_returns_the_version {
        let mut sender = c.dangerously_alias();
        let sent = sender.send().clone();
        sent == *sender.version() && le(c.version(), &sent) && *c.version() != sent
    }

    /// `sync` (join then fork) reconciles a fork: after two concurrent ticks,
    /// both sides end at the ticks' join, with disjoint parties whose rejoin
    /// recovers the original region.
    fn sync_reconciles_a_fork {
        let mut left = c.dangerously_alias();
        let mut right = left.fork();
        left.tick();
        right.tick();
        let want = left.version() | right.version();
        if left.sync(&mut right).is_err() {
            return false;
        }
        let reconciled = *left.version() == want
            && *right.version() == want
            && left.party().is_disjoint(right.party());
        let (left_party, _) = left.into_parts();
        let (right_party, _) = right.into_parts();
        let mut rejoined = left_party;
        reconciled && rejoined.join(right_party).is_ok() && &rejoined == c.party()
    }

    /// `own_version` is the projection of the version onto the party.
    fn own_version_is_the_projection {
        c.own_version() == (c.version() / c.party())
    }

    /// `from_parts ∘ into_parts == id`.
    fn parts_roundtrip {
        let (party, version) = c.dangerously_alias().into_parts();
        Clock::from_parts(party, version) == *c
    }

    /// `decode ∘ encode == id`, and the round-tripped clock re-encodes to the
    /// same bytes.
    fn clock_codec_roundtrip {
        let bytes = c.encode();
        Clock::decode(&bytes[..]).is_ok_and(|decoded| decoded == *c && decoded.encode() == bytes)
    }

    /// The clock's encoding is its party's bytes then its version's, exactly
    /// (each part byte-aligned and independently canonical).
    fn encode_frames_party_then_version {
        c.encode() == [c.party().encode(), c.version().encode()].concat()
    }

    /// `encoded_bits` is the pre-pad bit length of `encode`, at the clock level
    /// too.
    fn clock_encoded_bits_matches_encode_len {
        c.encode().len() as u64 == (c.encoded_bits() + 1).div_ceil(8)
    }
}

// ───────────────────────────── Clock: pairs ─────────────────────────────

laws! {
    /// Laws over a pair of clocks.
    ///
    /// The representational pair laws [`Version`] and [`Party`] each carry,
    /// closed over the whole stamp: `Eq` rides the canonical byte encoding,
    /// and equal clocks hash equally — what container keys on stamps rest
    /// on.
    pub static CLOCK_PAIR: (a: &Clock, b: &Clock);

    /// `Eq` is canonical-byte equality on whole stamps: `a == b ⟺
    /// encode(a) == encode(b)`, and clock `Eq` is exactly componentwise
    /// party and version `Eq`.
    ///
    /// Both directions matter: equal clocks must encode identically (the
    /// canonical encoding is a function of the value), and distinct clocks
    /// must encode distinctly (injectivity — what byte-level `Eq`/`Hash`
    /// uses rest on). With the frame pinned as the party's bytes then the
    /// version's ([`encode_frames_party_then_version`]), the biconditional
    /// also pins the party/version boundary: a difference in either
    /// component alone changes the composite bytes. The componentwise
    /// clause *executes* the link the first clause otherwise assumes —
    /// with an `Eq` refactored to whole-encode comparison, the
    /// biconditional alone would degrade toward tautology and a
    /// party-frame prefix ambiguity could hide behind it.
    fn clock_eq_iff_bytes_eq {
        (a == b) == (a.encode() == b.encode())
            && (a == b) == (a.party() == b.party() && a.version() == b.version())
    }

    /// `Eq`/`Hash` coherence: equal clocks hash equally.
    fn clock_eq_implies_hash_eq {
        a != b || hash_of(a) == hash_of(b)
    }
}

// ───────────────────────────── Clock × Version ─────────────────────────────

laws! {
    /// Laws over a clock and a message version.
    ///
    /// The receive laws (join-then-event: the result dominates both the old
    /// version and the message, strictly past their join, with the party
    /// untouched), the composition laws (`recv` and `sync` equal the
    /// compositions of the public operations they fuse, value for value),
    /// and the anonymous-join operators.
    pub static CLOCK_VERSION: (c: &Clock, msg: &Version);

    /// `recv` (join then event) learns the message and advances past it: the
    /// result dominates `old | msg` strictly, and the returned reference is the
    /// clock's new version.
    fn recv_learns_and_advances {
        let mut receiver = c.dangerously_alias();
        let old = receiver.version().clone();
        let returned = receiver.recv(msg).clone();
        let now = receiver.version().clone();
        let lub = &old | msg;
        returned == now && le(&lub, &now) && lub != now
    }

    /// `recv` never changes the party: message reception is an anonymous join.
    fn recv_fixes_party {
        let mut receiver = c.dangerously_alias();
        receiver.recv(msg);
        receiver.party() == c.party()
    }

    /// `recv` equals its stated composition — join the message into the
    /// version, then [`Clock::tick`] — value for value, returned reference
    /// included: reception is exactly the two public operations, however it
    /// is computed.
    fn recv_is_join_then_tick {
        let mut fused = c.dangerously_alias();
        let returned = fused.recv(msg).clone();
        let (party, version) = c.dangerously_alias().into_parts();
        let mut composed = Clock::from_parts(party, version | msg);
        composed.tick();
        returned == *composed.version() && fused == composed
    }

    /// `sync` equals its stated composition — [`Clock::join`] then
    /// [`Clock::fork`] — outcome for outcome.
    ///
    /// The disjoint arm is constructed by forking the clock and letting the
    /// sides diverge (one ticking, one receiving the message): it must
    /// reconcile to exactly the joined-then-reforked pair. The overlap arm
    /// is the clock against its own alias: it must be refused with neither
    /// side moved, exactly where `join` refuses.
    fn sync_is_join_then_fork {
        // The disjoint arm.
        let mut a = c.dangerously_alias();
        let mut b = a.fork();
        a.tick();
        b.recv(msg);
        let mut fused_a = a.dangerously_alias();
        let mut fused_b = b.dangerously_alias();
        let Ok(returned) = fused_a.sync(&mut fused_b).cloned() else {
            return false; // forked halves are disjoint: sync must accept
        };
        let mut composed_a = a.dangerously_alias();
        if composed_a.join(b.dangerously_alias()).is_err() {
            return false; // forked halves are disjoint: join must accept
        }
        let composed_b = composed_a.fork();
        if fused_a != composed_a || fused_b != composed_b || returned != *composed_a.version() {
            return false;
        }
        // The overlap arm: a clock shares its whole region with its alias.
        let mut x = c.dangerously_alias();
        let mut y = c.dangerously_alias();
        x.sync(&mut y).is_err()
            && c.dangerously_alias().join(c.dangerously_alias()).is_err()
            && x == *c
            && y == *c
    }

    /// The anonymous joins `Clock | Version` and `Version | Clock` merge the
    /// versions and keep the clock's party, and agree with each other.
    fn anonymous_join_merges_versions {
        let clock_version = c.dangerously_alias() | msg.clone();
        let version_clock = msg.clone() | c.dangerously_alias();
        clock_version.party() == c.party()
            && *clock_version.version() == (c.version() | msg)
            && version_clock == clock_version
    }

    /// `absorb` is the anonymous join with no event minted: the version
    /// becomes exactly `old | msg`, the party never moves, and the returned
    /// reference is the clock's new version.
    ///
    /// Absorbing the same message a second time changes nothing.
    fn absorb_is_the_anonymous_join {
        let mut fused = c.dangerously_alias();
        let returned = fused.absorb(msg).clone();
        let (party, version) = c.dangerously_alias().into_parts();
        let composed = Clock::from_parts(party, version | msg);
        let mut again = fused.dangerously_alias();
        again.absorb(msg);
        returned == *fused.version() && fused == composed && again == fused
    }
}

// ───────────────────── Clock: a receiver and items ─────────────────────

laws! {
    /// Laws for operations that combine any number of clocks.
    ///
    /// They require every collection operation to agree with its definition in
    /// terms of pairwise operations, including success and error behavior.
    pub static CLOCK_AND_LIST: (c: &Clock, items: &[Clock]);

    /// `join_all` succeeds exactly when all parties, including the receiver's,
    /// are pairwise disjoint.
    ///
    /// On success its party and version equal sequential calls to
    /// [`Clock::join`], and it returns the resulting version. On error the
    /// receiver still covers its original region and history, and at least one
    /// clock is returned.
    fn clock_join_all_accepts_iff_parties_pairwise_disjoint {
        let pairwise_disjoint = {
            let family: Vec<&Party> = core::iter::once(c.party())
                .chain(items.iter().map(Clock::party))
                .collect();
            family
                .iter()
                .enumerate()
                .all(|(i, a)| family[i + 1..].iter().all(|b| a.is_disjoint(b)))
        };
        let mut acc = c.dangerously_alias();
        match acc.join_all(items.iter().map(Clock::dangerously_alias)) {
            Ok(returned) => {
                let returned = returned.clone();
                let mut seq = c.dangerously_alias();
                let sequential = items
                    .iter()
                    .all(|item| seq.join(item.dangerously_alias()).is_ok());
                pairwise_disjoint && sequential && acc == seq && returned == *acc.version()
            }
            Err(returned) => {
                !pairwise_disjoint
                    && !returned.is_empty()
                    && acc.party().covers(c.party())
                    && le(c.version(), acc.version())
            }
        }
    }

    /// `join_all` reunites `k` independently ticked forks for every `k`.
    ///
    /// The final party is the original party. The final and returned versions
    /// both equal the sequential join of the children's versions. With no
    /// children, the receiver is unchanged.
    fn clock_join_all_reunites_forks_at_any_width {
        let width = items.len();
        let mut keeper = c.dangerously_alias();
        let mut children: Vec<Clock> = keeper.forks(width as u64).collect();
        for child in &mut children {
            child.tick();
        }
        let expected = children
            .iter()
            .fold(keeper.version().clone(), |acc, child| {
                &acc | child.version()
            });
        match keeper.join_all(children) {
            Ok(returned) => {
                let returned = returned.clone();
                returned == expected && *keeper.version() == expected && keeper.party() == c.party()
            }
            Err(_) => false,
        }
    }

    /// An unsuccessful `join_all` preserves every input region and version.
    ///
    /// Together, the receiver and returned clocks cover the original receiver
    /// and every input. Returned clocks may combine several inputs, so the law
    /// compares their joined state rather than individual values.
    fn clock_join_all_err_conserves_the_region_union {
        let mut acc = c.dangerously_alias();
        match acc.join_all(items.iter().map(Clock::dangerously_alias)) {
            Ok(_) => true, // an accepted fold equals the sequential pair joins
            Err(returned) => {
                let (mut union, mut history) = acc.into_parts();
                for back in returned {
                    let (party, version) = back.into_parts();
                    if let Some(missing) = party.without(&union) {
                        if union.join(missing).is_err() {
                            return false; // the remainder is disjoint by construction
                        }
                    }
                    history = &history | &version;
                }
                union.covers(c.party())
                    && le(c.version(), &history)
                    && items
                        .iter()
                        .all(|item| union.covers(item.party()) && le(item.version(), &history))
            }
        }
    }

    /// `sync_all` has the same result as `join_all` followed by `forks`.
    ///
    /// For disjoint clocks, both forms must leave the receiver, every child,
    /// and the returned version equal. If the receiver overlaps an input, or
    /// two inputs overlap each other, `sync_all` must reject the operation
    /// without changing any clock.
    fn sync_all_is_join_all_then_forks {
        // Compare both forms on independently advanced forks.
        let mut parent = c.dangerously_alias();
        let mut children: Vec<Clock> = parent.forks(items.len() as u64).collect();
        for (child, item) in children.iter_mut().zip(items) {
            *child |= item.version();
            child.tick();
        }
        let mut fused = parent.dangerously_alias();
        let mut fused_children: Vec<Clock> =
            children.iter().map(Clock::dangerously_alias).collect();
        let Ok(returned) = fused.sync_all(fused_children.iter_mut()).cloned() else {
            return false; // fork shares are disjoint: sync_all must accept
        };
        let mut composed = parent.dangerously_alias();
        if composed
            .join_all(children.iter().map(Clock::dangerously_alias))
            .is_err()
        {
            return false; // fork shares are disjoint: join_all must accept
        }
        let composed_children: Vec<Clock> = composed.forks(children.len() as u64).collect();
        if fused != composed
            || returned != *composed.version()
            || fused_children != composed_children
        {
            return false;
        }
        // Overlap with the receiver must leave both clocks unchanged.
        let mut x = c.dangerously_alias();
        let mut y = c.dangerously_alias();
        let receiver_overlap = x.sync_all([&mut y]).is_err() && x == *c && y == *c;
        // Overlap between inputs must also leave every clock unchanged.
        let mut p = c.dangerously_alias();
        let mut child = p.fork();
        let mut dup = child.dangerously_alias();
        let (p0, child0, dup0) = (
            p.dangerously_alias(),
            child.dangerously_alias(),
            dup.dangerously_alias(),
        );
        let item_overlap = p.sync_all([&mut child, &mut dup]).is_err()
            && p == p0
            && child == child0
            && dup == dup0;
        receiver_overlap && item_overlap
    }

    /// `recv_all` equals joining each input version and then calling
    /// [`Clock::tick`] once.
    ///
    /// The comparison includes the returned version and the complete clock. It
    /// also covers an empty input, which still advances the receiver once.
    fn recv_all_is_joins_then_tick {
        let mut fused = c.dangerously_alias();
        let returned = fused
            .recv_all(items.iter().map(|item| item.version()))
            .clone();
        let mut composed = c.dangerously_alias();
        for item in items {
            composed |= item.version();
        }
        composed.tick();
        returned == *composed.version() && fused == composed
    }

    /// `absorb_all` equals joining each input version without advancing the
    /// receiver.
    ///
    /// The comparison includes the returned version and the complete clock. It
    /// also covers an empty input, which leaves the receiver unchanged.
    fn absorb_all_is_the_sequential_joins {
        let mut fused = c.dangerously_alias();
        let returned = fused
            .absorb_all(items.iter().map(|item| item.version()))
            .clone();
        let mut composed = c.dangerously_alias();
        for item in items {
            composed |= item.version();
        }
        returned == *composed.version() && fused == composed
    }
}
