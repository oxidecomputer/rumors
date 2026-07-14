use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        protocol::{
            Accept, CompleteConnect, CompleteInitiator, CompleteResponder, Connect, Initiator,
            Reply, Responder,
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
            init_chain: (Reply<I, T, Next: CompleteInitiator<I, T>>),
            resp_chain: (Reply<I, T, Next: CompleteResponder<I, T>>),
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
            init_chain: (Reply<I, T, Next: $($init_chain)*>),
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
            resp_chain: (Reply<I, T, Next: $($resp_chain)*>),
        );
    };

    (@step
        init: [],
        resp: [],
        init_chain: ($($init_chain:tt)*),
        resp_chain: ($($resp_chain:tt)*) $(,)?
    ) => {
        pub trait Peer<I, T>:
            Initiator<I, T, Next: $($init_chain)*>
            + Responder<I, T, Next: $($resp_chain)*>
        where
            I: Backend<T, Node<Z>: Leaf<T>>,
            T: Send + Sync + 'static,
        {
        }

        impl<X, I, T> Peer<I, T> for X
        where
            I: Backend<T, Node<Z>: Leaf<T>>,
            T: Send + Sync + 'static,
            X: Initiator<I, T, Next: $($init_chain)*>
                + Responder<I, T, Next: $($resp_chain)*>,
        {
        }

        pub trait Server<I, T>:
            Accept<I, T, Next: Initiator<I, T, Next: $($init_chain)*> + Responder<I, T, Next: $($resp_chain)*>>
        where
            I: Backend<T, Node<Z>: Leaf<T>>,
            T: Send + Sync + 'static,
        {
        }

        impl<X, I, T> Server<I, T> for X
        where
            I: Backend<T, Node<Z>: Leaf<T>>,
            T: Send + Sync + 'static,
            X: Accept<I, T, Next: Initiator<I, T, Next: $($init_chain)*> + Responder<I, T, Next: $($resp_chain)*>>,
        {
        }

        pub trait Client<I, T>:
            Connect<I, T, Next: CompleteConnect<I, T, Next: Initiator<I, T, Next: $($init_chain)*> + Responder<I, T, Next: $($resp_chain)*>>>
        where
            I: Backend<T, Node<Z>: Leaf<T>>,
            T: Send + Sync + 'static,
        {
        }

        impl<X, I, T> Client<I, T> for X
        where
            I: Backend<T, Node<Z>: Leaf<T>>,
            T: Send + Sync + 'static,
            X: Connect<I, T, Next: CompleteConnect<I, T, Next: Initiator<I, T, Next: $($init_chain)*> + Responder<I, T, Next: $($resp_chain)*>>>,
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
