//! Why an operation was refused, as the agent reads it and as the screen words it.

use std::fmt;

use serde::Serialize;

use crate::ids::{AskId, DecisionId, IdExhaustion, QuestionId};

/// Why an operation was not carried out. Serialised with its `kind`, so the screen can say it
/// in the person's language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OperationRefusal {
    IdExhausted {
        counter: IdExhaustion,
    },
    AnswersChanged,
    EmptyFreeText {
        questions: Vec<QuestionId>,
    },
    InvalidAnswer {
        question: QuestionId,
    },
    Explanation(crate::ExplanationError),
    NoRound,
    RoundSubmitted,
    RoundUnsent {
        round: u32,
    },
    TopicEnded,
    UnknownQuestion {
        question: QuestionId,
    },
    UnknownOption {
        option: usize,
    },
    UnknownAsk {
        ask: AskId,
    },
    UnknownDecision {
        decision: DecisionId,
    },
    AlreadyReplied {
        ask: AskId,
    },
    NotInReview {
        decision: DecisionId,
    },
    /// A review request between sending a round and the next one: the agent would not read it.
    ReviewWhileRoundSent,
    NotCurrentRound {
        round: u32,
    },
    FollowsAnotherQuestion {
        ask: AskId,
    },
    /// The round cannot be sent while these questions have no stamp.
    Unstamped {
        questions: Vec<QuestionId>,
    },
}

impl fmt::Display for OperationRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdExhausted { counter } => counter.fmt(f),
            Self::AnswersChanged => write!(
                f,
                "answers changed; confirm the latest answers before sending"
            ),
            Self::InvalidAnswer { question } => write!(f, "invalid sent answer for {question}"),
            Self::EmptyFreeText { questions } => {
                write!(f, "custom answers are empty: {questions:?}")
            }
            Self::Explanation(error) => error.fmt(f),
            Self::NoRound => write!(f, "no round has been sent yet"),
            Self::RoundSubmitted => write!(f, "the round has already been sent"),
            Self::RoundUnsent { round } => write!(f, "round {round} has not been sent yet"),
            Self::TopicEnded => write!(f, "the topic has ended"),
            Self::UnknownQuestion { question } => {
                write!(f, "question {question} is not in the current round")
            }
            Self::UnknownOption { option } => write!(f, "there is no option {option}"),
            Self::UnknownAsk { ask } => write!(f, "there is no ask {ask}"),
            Self::UnknownDecision { decision } => write!(f, "there is no decision {decision}"),
            Self::AlreadyReplied { ask } => write!(f, "ask {ask} already has a reply"),
            Self::NotInReview { decision } => write!(f, "decision {decision} is not in review"),
            Self::ReviewWhileRoundSent => write!(
                f,
                "the round has been sent; review requests can be sent once the next round arrives"
            ),
            Self::FollowsAnotherQuestion { ask } => {
                write!(
                    f,
                    "ask {ask} is about another question, so this cannot follow it"
                )
            }
            Self::Unstamped { questions } => {
                let list: Vec<String> = questions.iter().map(ToString::to_string).collect();
                write!(
                    f,
                    "questions {} have no stamp yet; stamp every question before sending",
                    list.join(", ")
                )
            }
            Self::NotCurrentRound { round } => write!(
                f,
                "round {round} is no longer the current round; the screen shows the current one"
            ),
        }
    }
}

impl std::error::Error for OperationRefusal {}

impl From<IdExhaustion> for OperationRefusal {
    fn from(counter: IdExhaustion) -> Self {
        Self::IdExhausted { counter }
    }
}
