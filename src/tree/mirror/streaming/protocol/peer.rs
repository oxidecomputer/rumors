use super::*;

/// Declare the [`Peer`] trait and its blanket impl with the long `::Next`
/// chains spelled out as nested associated-type bounds in supertrait
/// position.
///
/// The macro emits the chains tt-munched ahead of time: the
/// initiator side wraps `$init_terminal` in N `Exchange<T, Next: …>`
/// layers (where N is the count of `_` tokens in `init: […]`), and the
/// responder side does the same for `$resp_terminal`.
///
/// Plain `where`-clauses on a trait definition don't propagate to callers,
/// but `Trait<AssocType: Bound>` in supertrait position does, so the entire
/// chain is expressed at the supertrait level rather than as a `where`
/// predicate.
macro_rules! define_peer {
(
    init: [$($init_count:tt)*],
    resp: [$($resp_count:tt)*],
    $(,)?
) => {
    define_peer!(@step
        init: [$($init_count)*],
        resp: [$($resp_count)*],
        init_chain: (CloseInitiator<T>),
        resp_chain: (CompleteResponder<T>),
    );
};

// Wrap one `Exchange<T, Next: …>` around the init-chain accumulator
// until the init-side counter is exhausted.
(@step
    init: [_ $($init_rest:tt)*],
    resp: [$($resp_count:tt)*],
    init_chain: ($($init_chain:tt)*),
    resp_chain: ($($resp_chain:tt)*) $(,)?
) => {
    define_peer!(@step
        init: [$($init_rest)*],
        resp: [$($resp_count)*],
        init_chain: (Exchange<T, Next: $($init_chain)*>),
        resp_chain: ($($resp_chain)*),
    );
};

// Init side done; munch the resp-side counter the same way.
(@step
    init: [],
    resp: [_ $($resp_rest:tt)*],
    init_chain: ($($init_chain:tt)*),
    resp_chain: ($($resp_chain:tt)*) $(,)?
) => {
    define_peer!(@step
        init: [],
        resp: [$($resp_rest)*],
        init_chain: ($($init_chain)*),
        resp_chain: (Exchange<T, Next: $($resp_chain)*>),
    );
};

// Both counters exhausted: emit the trait and blanket impl with the
// fully-built chain expressions inlined.
(@step
    init: [],
    resp: [],
    init_chain: ($($init_chain:tt)*),
    resp_chain: ($($resp_chain:tt)*) $(,)?
) => {
    /// A type that can play either side of the mirror protocol.
    ///
    /// It implements both [`Initiator`] and [`Responder`] at the root, and
    /// in either role the entire chain of `::Next` projections the
    /// session driver walks implements the right protocol trait at
    /// every height.
    ///
    /// Both `local::Exchange` and `remote::Exchange` pick this up for
    /// free via the blanket impl below; downstream call sites take a
    /// single `Peer<T>` bound on each argument and the chain bounds
    /// propagate.
    pub trait Peer<T>:
        Initiator<T, Next: OpenInitiator<T, Next: $($init_chain)*>> + Responder<T, Next: $($resp_chain)*>
    where
        T: Send + Sync,
    {
    }

    impl<X, T> Peer<T> for X
    where
        T: Send + Sync,
        X: Initiator<T, Next: OpenInitiator<T, Next: $($init_chain)*>> + Responder<T, Next: $($resp_chain)*>
    {
    }

    /// A [`Peer`] entered through the server side of the connect phase:
    /// [`Accept`] first, then either descent role. The whole-session
    /// bound the wire-facing driver takes for the remote party.
    pub trait Server<T>:
        Accept<T, Next: Initiator<T, Next: OpenInitiator<T, Next: $($init_chain)*>> + Responder<T, Next: $($resp_chain)*>>
    where
        T: Send + Sync,
    {
    }

    impl<X, T> Server<T> for X
    where
        T: Send + Sync,
        X: Accept<T, Next: Initiator<T, Next: OpenInitiator<T, Next: $($init_chain)*>> + Responder<T, Next: $($resp_chain)*>>
    {
    }

    /// A [`Peer`] entered through the client side of the connect phase:
    /// [`Connect`] then [`CompleteConnect`], then either descent role.
    /// The whole-session bound the drivers take for the local party.
    pub trait Client<T>:
        Connect<T, Next: CompleteConnect<T, Next: Initiator<T, Next: OpenInitiator<T, Next: $($init_chain)*>> + Responder<T, Next: $($resp_chain)*>>>
    where
        T: Send + Sync,
    {
    }

    impl<X, T> Client<T> for X
    where
        T: Send + Sync,
        X: Connect<T, Next: CompleteConnect<T, Next: Initiator<T, Next: OpenInitiator<T, Next: $($init_chain)*>> + Responder<T, Next: $($resp_chain)*>>>
    {
    }

};
}

// The initiator chain visits 14 `Exchange` levels (heights `S^30 → S^28 →
// … → S^4`) before terminating in `CloseInitiator` at `S<S<Z>>`. The
// terminal `CompleteInitiator` after `CloseInitiator` is already implied
// by `CloseInitiator::Next`'s own trait bound, so it doesn't need
// re-stating here.
//
// The responder chain visits 15 `Exchange` levels (heights `S^31 → S^29
// → … → S^3`) before terminating in `CompleteResponder` at `S<Z>`.
define_peer! {
    init: [_ _ _ _ _ _ _ _ _ _ _ _ _ _],
    resp: [_ _ _ _ _ _ _ _ _ _ _ _ _ _ _],
}
