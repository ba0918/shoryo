use serde::{Deserialize, Serialize};

mod current;
mod details;

use crate::{AskId, DecisionId, OperationRefusal, QuestionId, Topic};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResultSelection {
    Current {},
    Round { number: u32 },
    Decision { id: DecisionId },
    Answer { id: QuestionId },
    Question { id: QuestionId },
    Ask { id: AskId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultReadError {
    RoundNotFound(u32),
    DecisionNotFound(DecisionId),
    QuestionNotFound(QuestionId),
    AskNotFound(AskId),
    InvalidAnswer(OperationRefusal),
}

impl std::fmt::Display for ResultReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RoundNotFound(number) => write!(f, "round {number} does not exist"),
            Self::DecisionNotFound(id) => write!(f, "decision {id} does not exist"),
            Self::QuestionNotFound(id) => write!(f, "question {id} does not exist"),
            Self::AskNotFound(id) => write!(f, "ask {id} does not exist"),
            Self::InvalidAnswer(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ResultReadError {}

impl Topic {
    pub fn selected_result(&self, selection: &ResultSelection) -> Result<String, ResultReadError> {
        match selection {
            ResultSelection::Current {} => Ok(current::result(self)),
            ResultSelection::Round { number } => self
                .rounds
                .iter()
                .find(|round| round.number == *number)
                .map(json)
                .ok_or(ResultReadError::RoundNotFound(*number)),
            ResultSelection::Decision { id } => self
                .find_decision(id)
                .map(json)
                .ok_or_else(|| ResultReadError::DecisionNotFound(id.clone())),
            ResultSelection::Answer { id } => details::answer(self, id),
            ResultSelection::Question { id } => self
                .find_question(id)
                .map(|(_, question)| json(question))
                .ok_or_else(|| ResultReadError::QuestionNotFound(id.clone())),
            ResultSelection::Ask { id } => self
                .find_ask(*id)
                .map(json)
                .ok_or(ResultReadError::AskNotFound(*id)),
        }
    }
}

fn json(value: &impl Serialize) -> String {
    serde_json::to_string(value)
        .expect("result projections contain only JSON-compatible topic fields")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn missing_source_questions_are_not_assigned_an_invented_round() {
        let mut topic = Topic::new("Topic", "Request");
        topic.records.decisions.push(
            serde_json::from_value(json!({
                "id":"d1", "origin":{"question":"missing"},
                "history":[{"round":7,"content":{"name":"Name","text":"Exact text"}}]
            }))
            .unwrap(),
        );
        let current: Value =
            serde_json::from_str(&topic.selected_result(&ResultSelection::Current {}).unwrap())
                .unwrap();
        assert_eq!(
            current["records"]["decisions"][0]["origin_round"],
            Value::Null
        );
        assert_eq!(current["records"]["decisions"][0]["text"], "Exact text");
        assert_eq!(current["records"]["decisions"][0]["pre_approved"], false);
    }

    #[test]
    fn fixes_keep_their_source_round_without_a_source_question() {
        let mut topic = Topic::new("Topic", "Request");
        topic.records.decisions.push(
            serde_json::from_value(json!({
                "id":"d1", "origin":{"fix_round":7},
                "history":[{"round":7,"content":{"name":"Name","text":"Exact text"}}]
            }))
            .unwrap(),
        );
        let current: Value =
            serde_json::from_str(&topic.selected_result(&ResultSelection::Current {}).unwrap())
                .unwrap();
        assert_eq!(current["records"]["decisions"][0]["origin_round"], 7);
    }

    #[test]
    fn absent_objects_return_typed_refusals_without_changing_the_topic() {
        let topic = Topic::new("Topic", "Request");
        let before = topic.clone();
        assert_eq!(
            topic.selected_result(&ResultSelection::Round { number: 1 }),
            Err(ResultReadError::RoundNotFound(1))
        );
        let id = DecisionId::new("missing");
        assert_eq!(
            topic.selected_result(&ResultSelection::Decision { id: id.clone() }),
            Err(ResultReadError::DecisionNotFound(id))
        );
        assert_eq!(topic, before);
    }
}
