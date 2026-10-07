//! The domain of shoryo: a topic's state and the rules that change it. No I/O lives here.
#![deny(clippy::print_stdout, clippy::print_stderr)]

mod ids;
mod state;

pub use ids::{AskId, DecisionId, EventId, QuestionId};
pub use state::{
    Answer, Ask, AskState, Choice, Class, Decision, DecisionChange, DecisionContent, Delegated,
    Fix, InReview, LoadError, NodeRef, Origin, Question, Records, Rejected, Reply,
    ReviewConclusion, ReviewOutcome, Revision, Round, Topic, Undecided,
};
