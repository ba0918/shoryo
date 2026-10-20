use serde::Serialize;

use super::{ResultReadError, json};
use crate::{AskId, AskState, QuestionId, SentAnswer, Timestamp, Topic};

pub(super) fn answer(topic: &Topic, id: &QuestionId) -> Result<String, ResultReadError> {
    let (round, question) = topic
        .find_question(id)
        .ok_or_else(|| ResultReadError::QuestionNotFound(id.clone()))?;
    let sent = round
        .submitted
        .then(|| question.sent_answer())
        .transpose()
        .map_err(ResultReadError::InvalidAnswer)?;
    let selected_option = sent.as_ref().and_then(|answer| answer.choice).map(|index| {
        let option = &question.options[index];
        SelectedOption {
            text: &option.text,
            consequence: &option.consequence,
        }
    });
    let asks = round
        .asks
        .iter()
        .filter(|ask| ask.question == *id)
        .map(|ask| AskLink {
            id: ask.id,
            follows: ask.follows,
            status: match &ask.state {
                AskState::Waiting => AskStatus::Waiting,
                AskState::Replied(_) => AskStatus::Replied,
                AskState::NoReply => AskStatus::NoReply,
            },
        })
        .collect();
    Ok(json(&AnswerReceipt {
        question: &question.id,
        text: &question.text,
        round: round.number,
        submitted: round.submitted,
        sent_at: round.sent_at.as_ref(),
        answer: sent,
        selected_option,
        asks,
    }))
}

#[derive(Serialize)]
struct AnswerReceipt<'a> {
    question: &'a QuestionId,
    text: &'a str,
    round: u32,
    submitted: bool,
    sent_at: Option<&'a Timestamp>,
    answer: Option<SentAnswer>,
    selected_option: Option<SelectedOption<'a>>,
    asks: Vec<AskLink>,
}

#[derive(Serialize)]
struct SelectedOption<'a> {
    text: &'a str,
    consequence: &'a str,
}

#[derive(Serialize)]
struct AskLink {
    id: AskId,
    follows: Option<AskId>,
    status: AskStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum AskStatus {
    Waiting,
    Replied,
    NoReply,
}
