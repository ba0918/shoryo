//! Views the screen draws, computed from the state: the prerequisite chains and the 代決 mark
//! here, the map with its paths and review marks in `map` (`docs/spec/screen.md`, "前提の行",
//! "地図").

mod map;

use serde::Serialize;

use crate::ids::{DecisionId, QuestionId};
use crate::state::{Decision, Origin, Question, Round, Stamp, Topic};

pub use map::{Column, Map, MapEdge, MapNode, NodeKind};

/// How many names a chain shows at most.
const CHAIN_LENGTH: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChainLink {
    pub decision: DecisionId,
    pub name: String,
    pub in_review: bool,
}

impl Topic {
    /// 前提の行: for each direct prerequisite of the question, that decision followed back
    /// through the first prerequisite of the question it was decided in.
    pub fn chains(&self, question: &QuestionId) -> Vec<Vec<ChainLink>> {
        let Some((_, question)) = self.find_question(question) else {
            return Vec::new();
        };
        question
            .premises
            .iter()
            .map(|premise| self.chain_from(premise))
            .collect()
    }

    fn chain_from(&self, start: &DecisionId) -> Vec<ChainLink> {
        let mut chain = Vec::new();
        let mut next = Some(start.clone());
        while let Some(id) = next.take() {
            if chain.len() == CHAIN_LENGTH {
                break;
            }
            let Some(decision) = self.find_decision(&id) else {
                break;
            };
            chain.push(ChainLink {
                name: decision.current().name.clone(),
                in_review: self.is_in_review(&id),
                decision: id,
            });
            next = self
                .origin_question(decision)
                .and_then(|(_, question)| question.premises.first().cloned())
                .filter(|premise| !chain.iter().any(|link| &link.decision == premise));
        }
        chain
    }

    /// 代決の印: the decision came from an answer sent with the LLM's stamp, and has not been
    /// revised since (a revision is answered again by the person).
    pub fn decided_pre_approved(&self, id: &DecisionId) -> bool {
        self.find_decision(id).is_some_and(|decision| {
            !decision.was_revised()
                && self
                    .origin_question(decision)
                    .is_some_and(|(_, question)| question.answer.stamp == Some(Stamp::PreApproved))
        })
    }

    fn origin_question(&self, decision: &Decision) -> Option<(&Round, &Question)> {
        match &decision.origin {
            Origin::Question(question) => self.find_question(question),
            Origin::FixRound(_) => None,
        }
    }
}
