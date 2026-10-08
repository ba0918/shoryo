//! Builders for the rounds the tests send, written as the agent writes them.

use serde_json::{Value, json};
use shoryo_core::{RoundInput, Timestamp};

pub fn option(text: &str, recommended: bool) -> Value {
    json!({
        "text": text,
        "description": [{"type":"text","body":format!("About {text}.")}],
        "recommended": recommended,
        "consequence": format!("With {text}, this follows."),
    })
}

pub fn question(id: &str, premises: &[&str]) -> Value {
    json!({
        "id": id,
        "text": format!("Question {id}?"),
        "class": "human",
        "why_now": "Later questions rest on it.",
        "premises": premises,
        "background": [{"type":"text","body":"Terms used here."}],
        "options": [option("A", true), option("B", false)],
    })
}

pub fn decision(id: &str, question: &str, name: &str) -> Value {
    json!({
        "id": id,
        "name": name,
        "text": format!("{name}, in full."),
        "decided_by": { "question": question },
    })
}

/// A time the server could have read when it accepted an operation, for tests where the
/// time itself does not matter.
pub fn any_time() -> Timestamp {
    Timestamp::new("2026-10-08T04:00:00.000Z")
}

pub fn round(body: Value) -> RoundInput {
    RoundInput::from_json(&body.to_string()).expect("the test writes a valid round")
}

/// A topic built from rounds as the agent sends them; every round but the last is sent.
pub fn topic_from(rounds: Vec<Value>) -> shoryo_core::Topic {
    let mut topic = shoryo_core::Topic::new("", "");
    let count = rounds.len();
    for (index, body) in rounds.into_iter().enumerate() {
        topic
            .apply_round(round(body))
            .expect("the test writes valid rounds");
        if index + 1 < count {
            submit_current(&mut topic).expect("an applied round can be sent");
        }
    }
    topic
}

/// Stamps every unstamped question and sends the current round, as the person does with the
/// round on the screen.
pub fn submit_current(topic: &mut shoryo_core::Topic) -> Result<(), shoryo_core::OperationRefusal> {
    let Some(current) = topic.current_round() else {
        return topic.apply(shoryo_core::Operation::Submit { round: 0 }, any_time());
    };
    let round = current.number;
    let unstamped: Vec<_> = current
        .questions
        .iter()
        .filter(|question| question.answer.stamp.is_none())
        .map(|question| question.id.clone())
        .collect();
    for question in unstamped {
        topic.apply(
            shoryo_core::Operation::Stamp {
                question,
                stamped: true,
            },
            any_time(),
        )?;
    }
    topic.apply(shoryo_core::Operation::Submit { round }, any_time())
}
