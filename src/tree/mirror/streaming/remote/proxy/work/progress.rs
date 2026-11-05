//! Ordering trace for the proxy's progress-critical publications.
//!
//! Every event identifies its endpoint, session stage, and question height. An
//! encoder publishes that question; as the walk takes each chunk of the
//! reply answering it, the take publishes the scopes the chunk's questions
//! create. The stage
//! keeps overlapping walk and terminal activity on separate ledgers at leaf
//! height.

/// One endpoint-local progress identity.
#[derive(Clone, Copy)]
pub struct Progress {
    /// Distinguishes this proxy from the other endpoint in the same trace.
    #[cfg(test)]
    work: usize,
    /// Distinguishes the final exchange from the overlapping walk.
    #[cfg(test)]
    stage: trace::Stage,
}

/// Record publications without adding runtime state outside tests.
impl Progress {
    /// Allocate a trace identity for one proxy endpoint.
    pub fn new() -> Self {
        Self {
            #[cfg(test)]
            work: trace::new_work(),
            #[cfg(test)]
            stage: trace::Stage::Walk,
        }
    }

    /// Label publications made by the final leaf exchange.
    #[cfg(not(test))]
    pub const fn terminal(self) -> Self {
        self
    }

    /// Label publications made by the final leaf exchange.
    #[cfg(test)]
    pub fn terminal(mut self) -> Self {
        self.stage = trace::Stage::Terminal;
        self
    }

    /// Record a frame handed to the multiplexer, at the height of the
    /// questions it carries.
    pub fn wire_frame(self, _height: usize, _questions: usize) {
        #[cfg(test)]
        trace::record_stage(
            self.work,
            self.stage,
            trace::Kind::WireFrame {
                questions: _questions,
            },
            _height,
        );
    }

    /// Record one question published after the frame carrying it.
    pub fn local_question(self, _height: usize) {
        #[cfg(test)]
        trace::record_stage(self.work, self.stage, trace::Kind::LocalQuestion, _height);
    }

    /// Record a taken chunk of an answer at its question's height, with the
    /// count of scopes the take publishes; `opening` marks an answer's
    /// first chunk.
    pub fn taken(self, _height: usize, _opening: bool, _scopes: usize) {
        #[cfg(test)]
        trace::record_stage(
            self.work,
            self.stage,
            if _opening {
                trace::Kind::TakenReply { scopes: _scopes }
            } else {
                trace::Kind::TakenChunk { scopes: _scopes }
            },
            _height,
        );
    }

    /// Record a derived scope at the height of the question its reply answered.
    pub fn next_scope(self, _height: usize) {
        #[cfg(test)]
        trace::record_stage(self.work, self.stage, trace::Kind::NextScope, _height);
    }
}

#[cfg(test)]
pub use trace::{Trace, with_trace};

#[cfg(test)]
mod trace;
