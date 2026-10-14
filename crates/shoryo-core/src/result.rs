use serde::{Deserialize, Serialize};

use crate::{DecisionId, Delegated, InReview, Origin, Rejected, SentAs, Topic, Undecided};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResultSelection {
    Current {},
    Round { number: u32 },
    Decision { id: DecisionId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultReadError {
    RoundNotFound(u32),
    DecisionNotFound(DecisionId),
}

impl std::fmt::Display for ResultReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RoundNotFound(number) => write!(f, "round {number} does not exist"),
            Self::DecisionNotFound(id) => write!(f, "decision {id} does not exist"),
        }
    }
}

impl std::error::Error for ResultReadError {}

impl Topic {
    pub fn selected_result(&self, selection: &ResultSelection) -> Result<String, ResultReadError> {
        match selection {
            ResultSelection::Current {} => Ok(json(&Current {
                title: &self.title,
                original_request: &self.original_request,
                ended: self.ended,
                latest_round: self.current_round().map(|round| RoundStatus {
                    number: round.number,
                    submitted: round.submitted,
                    sent_as: round.sent_as,
                    question_count: round.questions.len(),
                }),
                pending_events: self.pending_events().len(),
                records: CurrentRecords {
                    decisions: self
                        .records
                        .decisions
                        .iter()
                        .map(|decision| CurrentDecision {
                            id: &decision.id,
                            name: &decision.current().name,
                            text: &decision.current().text,
                            origin: &decision.origin,
                            origin_round: match &decision.origin {
                                Origin::Question(id) => self
                                    .rounds
                                    .iter()
                                    .find(|round| {
                                        round.questions.iter().any(|question| question.id == *id)
                                    })
                                    .map(|round| round.number),
                                Origin::FixRound(number) => Some(*number),
                            },
                            pre_approved: self.decided_pre_approved(&decision.id),
                        })
                        .collect(),
                    not_building: &self.records.not_building,
                    undecided: &self.records.undecided,
                    delegated: &self.records.delegated,
                    rejected: &self.records.rejected,
                    in_review: &self.records.in_review,
                },
            })),
            ResultSelection::Round { number } => self
                .rounds
                .iter()
                .find(|round| round.number == *number)
                .map(json)
                .ok_or(ResultReadError::RoundNotFound(*number)),
            ResultSelection::Decision { id } => self
                .records
                .decisions
                .iter()
                .find(|decision| decision.id == *id)
                .map(json)
                .ok_or_else(|| ResultReadError::DecisionNotFound(id.clone())),
        }
    }
}

fn json(value: &impl Serialize) -> String {
    serde_json::to_string(value)
        .expect("result projections contain only JSON-compatible topic fields")
}

#[derive(Serialize)]
struct Current<'a> {
    title: &'a str,
    original_request: &'a str,
    ended: bool,
    latest_round: Option<RoundStatus>,
    pending_events: usize,
    records: CurrentRecords<'a>,
}

#[derive(Serialize)]
struct RoundStatus {
    number: u32,
    submitted: bool,
    sent_as: Option<SentAs>,
    question_count: usize,
}

#[derive(Serialize)]
struct CurrentRecords<'a> {
    decisions: Vec<CurrentDecision<'a>>,
    not_building: &'a [String],
    undecided: &'a [Undecided],
    delegated: &'a [Delegated],
    rejected: &'a [Rejected],
    in_review: &'a [InReview],
}

#[derive(Serialize)]
struct CurrentDecision<'a> {
    id: &'a DecisionId,
    name: &'a str,
    text: &'a str,
    origin: &'a Origin,
    origin_round: Option<u32>,
    pre_approved: bool,
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
