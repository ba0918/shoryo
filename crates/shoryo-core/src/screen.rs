//! Everything the screen draws besides the topic itself, computed from the state in one go
//! (`docs/spec/screen.md`).

use std::collections::BTreeMap;

use crate::ids::{DecisionId, QuestionId};
use crate::operations::SentAnswer;
use crate::state::Topic;
use crate::views::{ChainLink, Map};

/// The derived views the page receives with each change of the topic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenView {
    /// 前提の行 of every question, by question.
    pub chains: BTreeMap<QuestionId, Vec<Vec<ChainLink>>>,
    pub map: Map,
    /// 道筋 from every node of the map, by node key.
    pub paths: BTreeMap<String, Vec<String>>,
    pub unstamped: Vec<QuestionId>,
    /// The decisions that carry the 代決 mark.
    pub pre_approved: Vec<DecisionId>,
    /// The answers the current round would send now; `None` while it cannot be sent.
    pub expected_answers: Option<Vec<SentAnswer>>,
    /// The answers each sent round sent, by round number.
    pub sent_answers: BTreeMap<u32, Vec<SentAnswer>>,
}

impl Topic {
    pub fn screen_view(&self) -> ScreenView {
        let chains = self
            .rounds
            .iter()
            .flat_map(|round| &round.questions)
            .map(|question| (question.id.clone(), self.chains(&question.id)))
            .collect();
        let pre_approved = self
            .records
            .decisions
            .iter()
            .filter(|decision| self.decided_pre_approved(&decision.id))
            .map(|decision| decision.id.clone())
            .collect();
        let map = self.map();
        let paths = map.paths();
        ScreenView {
            chains,
            map,
            paths,
            unstamped: self.unstamped(),
            pre_approved,
            expected_answers: self.current_round().and_then(|r| r.sent_answers().ok()),
            sent_answers: self
                .rounds
                .iter()
                .filter(|r| r.submitted)
                .filter_map(|r| r.sent_answers().ok().map(|a| (r.number, a)))
                .collect(),
        }
    }
}
