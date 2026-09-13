//! Ordering trace for the proxy's progress-critical publications.
//!
//! Every event uses the question's height: an encoder publishes that question,
//! and a decoder answers it before publishing any derived scopes. This keeps
//! the last internal stage's derived leaf scopes distinct from the terminal
//! exchange's answers.

/// One endpoint-local progress identity.
#[derive(Clone, Copy)]
pub struct Progress {
    /// Distinguishes this proxy from the other endpoint in the same trace.
    #[cfg(test)]
    work: usize,
}

/// Record publications without adding runtime state outside tests.
impl Progress {
    /// Allocate a trace identity for one proxy endpoint.
    pub fn new() -> Self {
        Self {
            #[cfg(test)]
            work: trace::new_work(),
        }
    }

    /// Record a flushed wire reply at the height of the questions it asks.
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

    /// Record a decoded answer at its question's height, with its derived scope count.
    pub fn decoded_reply(self, _height: usize, _scopes: usize) {
        #[cfg(test)]
        trace::record(
            self.work,
            trace::Kind::DecodedReply { scopes: _scopes },
            _height,
        );
    }

    /// Record a derived scope at the height of the question its reply answered.
    pub fn next_scope(self, _height: usize) {
        #[cfg(test)]
        trace::record(self.work, trace::Kind::NextScope, _height);
    }
}

#[cfg(test)]
pub use trace::{Trace, with_trace};

#[cfg(test)]
mod trace;
