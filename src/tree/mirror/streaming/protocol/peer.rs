use super::*;

macro_rules! define_peer {
    (
        init: [$($init_count:tt)*],
        resp: [$($resp_count:tt)*],
        $(,)?
    ) => {
        define_peer!(@step
            init: [$($init_count)*],
            resp: [$($resp_count)*],
            init_chain: (CloseInitiator<B, T>),
            resp_chain: (CompleteResponder<B, T>),
        );
    };

    (@step
        init: [_ $($init_rest:tt)*],
        resp: [$($resp_count:tt)*],
        init_chain: ($($init_chain:tt)*),
        resp_chain: ($($resp_chain:tt)*) $(,)?
    ) => {
        define_peer!(@step
            init: [$($init_rest)*],
            resp: [$($resp_count)*],
            init_chain: (Exchange<B, T, Next: $($init_chain)*>),
            resp_chain: ($($resp_chain)*),
        );
    };

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
            resp_chain: (Exchange<B, T, Next: $($resp_chain)*>),
        );
    };

    (@step
        init: [],
        resp: [],
        init_chain: ($($init_chain:tt)*),
        resp_chain: ($($resp_chain:tt)*) $(,)?
    ) => {
        pub trait Peer<B, T>:
            Initiator<B, T, Next: OpenInitiator<B, T, Next: $($init_chain)*>>
            + Responder<B, T, Next: $($resp_chain)*>
        where
            B: Backend<T>,
            T: Send + Sync,
        {
        }

        impl<X, B, T> Peer<B, T> for X
        where
            B: Backend<T>,
            T: Send + Sync,
            X: Initiator<B, T, Next: OpenInitiator<B, T, Next: $($init_chain)*>>
                + Responder<B, T, Next: $($resp_chain)*>,
        {
        }

        pub trait Server<B, T>:
            Accept<B, T, Next: Initiator<B, T, Next: OpenInitiator<B, T, Next: $($init_chain)*>> + Responder<B, T, Next: $($resp_chain)*>>
        where
            B: Backend<T>,
            T: Send + Sync,
        {
        }

        impl<X, B, T> Server<B, T> for X
        where
            B: Backend<T>,
            T: Send + Sync,
            X: Accept<B, T, Next: Initiator<B, T, Next: OpenInitiator<B, T, Next: $($init_chain)*>> + Responder<B, T, Next: $($resp_chain)*>>,
        {
        }

        pub trait Client<B, T>:
            Connect<B, T, Next: CompleteConnect<B, T, Next: Initiator<B, T, Next: OpenInitiator<B, T, Next: $($init_chain)*>> + Responder<B, T, Next: $($resp_chain)*>>>
        where
            B: Backend<T>,
            T: Send + Sync,
        {
        }

        impl<X, B, T> Client<B, T> for X
        where
            B: Backend<T>,
            T: Send + Sync,
            X: Connect<B, T, Next: CompleteConnect<B, T, Next: Initiator<B, T, Next: OpenInitiator<B, T, Next: $($init_chain)*>> + Responder<B, T, Next: $($resp_chain)*>>>,
        {
        }
    };
}

// One `_` per exchange round: the initiator descends heights 30 → 2 in
// fourteen rounds of two heights each, the responder 31 → 1 in fifteen.
// `mirror_connected` in streaming.rs drives this same schedule; the counts
// must move together.
define_peer! {
    init: [_ _ _ _ _ _ _ _ _ _ _ _ _ _],
    resp: [_ _ _ _ _ _ _ _ _ _ _ _ _ _ _],
}
