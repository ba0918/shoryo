//! The domain of shoryo: a topic's state and the rules that change it. No I/O lives here.
#![deny(clippy::print_stdout, clippy::print_stderr)]

mod explanation;
mod explanation_validation;
mod ids;
mod operations;
mod result;
mod round;
mod screen;
mod state;
mod views;

pub use explanation::{Part, Role};
pub use explanation_validation::{ExplanationError, ExplanationRule};
pub use ids::{AskId, DecisionId, EventId, IdExhaustion, QuestionId};
pub use operations::{Event, EventKind, Operation, OperationRefusal, SentAnswer};
pub use result::{ResultReadError, ResultSelection};
pub use round::{
    DecidedBy, DecisionInput, FixInput, InputError, QuestionInput, RecordsInput, RoundInput,
    RoundRefusal,
};
pub use screen::ScreenView;
pub use state::{
    Answer, AnswerMethod, Ask, AskState, Choice, Class, Decision, DecisionChange, DecisionContent,
    Delegated, Fix, InReview, LoadError, NodeRef, Origin, Question, Records, Rejected, Reply,
    ReviewConclusion, ReviewOutcome, Revision, Round, RoundRecords, SentAs, Stamp, Timestamp,
    Topic, Undecided,
};
pub use views::{ChainLink, Column, Map, MapEdge, MapNode, NodeKind};
