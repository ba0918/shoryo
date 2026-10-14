use serde::Serialize;

use super::json;
use crate::{DecisionId, Delegated, InReview, Origin, Rejected, SentAs, Topic, Undecided};

pub(super) fn result(topic: &Topic) -> String {
    json(&Current {
        title: &topic.title,
        original_request: &topic.original_request,
        ended: topic.ended,
        latest_round: topic.current_round().map(|round| RoundStatus {
            number: round.number,
            submitted: round.submitted,
            sent_as: round.sent_as,
            question_count: round.questions.len(),
        }),
        pending_events: topic.pending_events().len(),
        records: CurrentRecords {
            decisions: topic
                .records
                .decisions
                .iter()
                .map(|decision| CurrentDecision {
                    id: &decision.id,
                    name: &decision.current().name,
                    text: &decision.current().text,
                    origin: &decision.origin,
                    origin_round: match &decision.origin {
                        Origin::Question(id) => topic
                            .rounds
                            .iter()
                            .find(|round| round.questions.iter().any(|question| question.id == *id))
                            .map(|round| round.number),
                        Origin::FixRound(number) => Some(*number),
                    },
                    pre_approved: topic.decided_pre_approved(&decision.id),
                })
                .collect(),
            not_building: &topic.records.not_building,
            undecided: &topic.records.undecided,
            delegated: &topic.records.delegated,
            rejected: &topic.records.rejected,
            in_review: &topic.records.in_review,
        },
    })
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
