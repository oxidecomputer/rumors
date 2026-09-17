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

/// Define the complete client, server, and connected-participant phase chains.
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
        /// A participant's complete connected-to-terminal phase chain.
        pub trait Peer<I>:
            CompleteEqual<I>
            + Initiator<I, Next: $($init_chain)*>
            + Responder<I, Next: $($resp_chain)*>
        where
            I: Backend<Node<Z>: Leaf>,
                    {
        }

        /// Every participant implementing the complete chain is a peer.
        impl<X, I> Peer<I> for X
        where
            I: Backend<Node<Z>: Leaf>,
                        X: CompleteEqual<I>
                + Initiator<I, Next: $($init_chain)*>
                + Responder<I, Next: $($resp_chain)*>,
        {
        }

        /// A server's complete accept-to-terminal phase chain.
        pub trait Server<I>:
            Accept<I, Next: Initiator<I, Next: $($init_chain)*> + Responder<I, Next: $($resp_chain)*>>
        where
            I: Backend<Node<Z>: Leaf>,
                    {
        }

        /// Every participant implementing the complete server chain is a server.
        impl<X, I> Server<I> for X
        where
            I: Backend<Node<Z>: Leaf>,
                        X: Accept<I, Next: Initiator<I, Next: $($init_chain)*> + Responder<I, Next: $($resp_chain)*>>,
        {
        }

        /// A client's complete connect-to-terminal phase chain.
        pub trait Client<I>:
            Connect<I, Next: CompleteConnect<I, Next: Initiator<I, Next: $($init_chain)*> + Responder<I, Next: $($resp_chain)*>>>
        where
            I: Backend<Node<Z>: Leaf>,
                    {
        }

        /// Every participant implementing the complete client chain is a client.
        impl<X, I> Client<I> for X
        where
            I: Backend<Node<Z>: Leaf>,
                        X: Connect<I, Next: CompleteConnect<I, Next: Initiator<I, Next: $($init_chain)*> + Responder<I, Next: $($resp_chain)*>>>,
        {
        }
    };
}

// One `_` per exchange round: the initiator descends heights 31 → 1 in fifteen
// rounds of two heights each, the responder 30 → 2 in fourteen.
// `driver::mirror_connected` drives this schedule. If its loop count and these
// chains disagree, the terminal trait bounds fail to compile; macro repetition
// and `seq!` cannot share a named count.
define_peer! {
    init: [_ _ _ _ _ _ _ _ _ _ _ _ _ _ _],
    resp: [_ _ _ _ _ _ _ _ _ _ _ _ _ _],
}
