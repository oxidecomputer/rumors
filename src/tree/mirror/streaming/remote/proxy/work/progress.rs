//! Ordering trace for the proxy's progress-critical publications.

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

    /// Record one complete outgoing wire reply and its question count,
    /// at the reply's height.
    pub fn wire_reply(self, _height: usize, _questions: usize) {
        #[cfg(test)]
        trace::record(
            self.work,
            trace::Kind::WireReply {
                questions: _questions,
            },
            _height,
        );
    }

    /// Record one question published after its wire reply.
    pub fn local_question(self, _height: usize) {
        #[cfg(test)]
        trace::record(self.work, trace::Kind::LocalQuestion, _height);
    }

    /// Record one decoded reply and its dependent-scope count.
    pub fn decoded_reply(self, _height: usize, _scopes: usize) {
        #[cfg(test)]
        trace::record(
            self.work,
            trace::Kind::DecodedReply { scopes: _scopes },
            _height,
        );
    }

    /// Record one dependent scope published after its decoded reply.
    pub fn next_scope(self, _height: usize) {
        #[cfg(test)]
        trace::record(self.work, trace::Kind::NextScope, _height);
    }
}

#[cfg(test)]
pub use trace::{Trace, with_trace};

#[cfg(test)]
mod trace;
