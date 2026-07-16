//! Ordering trace for the proxy's progress-critical publications.

use crate::tree::typed::height::Height;

/// One endpoint-local progress identity.
#[derive(Clone, Copy)]
pub struct Progress {
    #[cfg(test)]
    work: usize,
}

impl Progress {
    /// Allocate a trace identity for one proxy endpoint.
    pub fn new() -> Self {
        Self {
            #[cfg(test)]
            work: trace::new_work(),
        }
    }

    /// Record one complete outgoing wire reply and its question count.
    pub fn wire_reply<H: Height>(self, _questions: usize) {
        #[cfg(test)]
        trace::record(
            self.work,
            trace::Kind::WireReply {
                questions: _questions,
            },
            H::HEIGHT,
        );
    }

    /// Record one question published after its wire reply.
    pub fn local_question<H: Height>(self) {
        #[cfg(test)]
        trace::record(self.work, trace::Kind::LocalQuestion, H::HEIGHT);
    }

    /// Record one decoded reply and its dependent-scope count.
    pub fn decoded_reply<H: Height>(self, _scopes: usize) {
        #[cfg(test)]
        trace::record(
            self.work,
            trace::Kind::DecodedReply { scopes: _scopes },
            H::HEIGHT,
        );
    }

    /// Record one dependent scope published after its decoded reply.
    pub fn next_scope<H: Height>(self) {
        #[cfg(test)]
        trace::record(self.work, trace::Kind::NextScope, H::HEIGHT);
    }
}

#[cfg(test)]
pub use trace::{Trace, with_trace};

#[cfg(test)]
mod trace;
