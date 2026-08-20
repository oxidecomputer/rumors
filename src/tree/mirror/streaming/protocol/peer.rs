use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        protocol::{
            Accept, CompleteConnect, CompleteEqual, CompleteInitiator, CompleteResponder, Connect,
            Initiator, Reply, Responder,
        },
    },
    typed::height::Z,
};

macro_rules! define_peer {
    (
        init: [$($init_count:tt)*],
        resp: [$($resp_count:tt)*],
        $(,)?
    ) => {
        define_peer!(@step
            init: [$($init_count)*],
            resp: [$($resp_count)*],
            init_chain: (Reply<I, Next: CompleteInitiator<I>>),
            resp_chain: (Reply<I, Next: CompleteResponder<I>>),
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
            init_chain: (Reply<I, Next: $($init_chain)*>),
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
            resp_chain: (Reply<I, Next: $($resp_chain)*>),
        );
    };

    (@step
        init: [],
        resp: [],
        init_chain: ($($init_chain:tt)*),
        resp_chain: ($($resp_chain:tt)*) $(,)?
    ) => {
        pub trait Peer<I>:
            CompleteEqual<I>
            + Initiator<I, Next: $($init_chain)*>
            + Responder<I, Next: $($resp_chain)*>
        where
            I: Backend<Node<Z>: Leaf>,
                    {
        }

        impl<X, I> Peer<I> for X
        where
            I: Backend<Node<Z>: Leaf>,
                        X: CompleteEqual<I>
                + Initiator<I, Next: $($init_chain)*>
                + Responder<I, Next: $($resp_chain)*>,
        {
        }

        pub trait Server<I>:
            Accept<I, Next: Initiator<I, Next: $($init_chain)*> + Responder<I, Next: $($resp_chain)*>>
        where
            I: Backend<Node<Z>: Leaf>,
                    {
        }

        impl<X, I> Server<I> for X
        where
            I: Backend<Node<Z>: Leaf>,
                        X: Accept<I, Next: Initiator<I, Next: $($init_chain)*> + Responder<I, Next: $($resp_chain)*>>,
        {
        }

        pub trait Client<I>:
            Connect<I, Next: CompleteConnect<I, Next: Initiator<I, Next: $($init_chain)*> + Responder<I, Next: $($resp_chain)*>>>
        where
            I: Backend<Node<Z>: Leaf>,
                    {
        }

        impl<X, I> Client<I> for X
        where
            I: Backend<Node<Z>: Leaf>,
                        X: Connect<I, Next: CompleteConnect<I, Next: Initiator<I, Next: $($init_chain)*> + Responder<I, Next: $($resp_chain)*>>>,
        {
        }
    };
}

// One `_` per exchange round: the initiator descends heights 31 → 1 in
// fifteen rounds of two heights each, the responder 30 → 2 in fourteen.
// `mirror_connected` in streaming.rs drives this same schedule; the counts
// must move together.
define_peer! {
    init: [_ _ _ _ _ _ _ _ _ _ _ _ _ _ _],
    resp: [_ _ _ _ _ _ _ _ _ _ _ _ _ _],
}
