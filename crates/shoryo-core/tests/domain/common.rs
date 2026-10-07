//! Builders for the rounds the tests send, written as the agent writes them.

use serde_json::{Value, json};
use shoryo_core::RoundInput;

pub fn option(text: &str, recommended: bool) -> Value {
    json!({
        "text": text,
        "description": format!("About {text}."),
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
        "background": "Terms used here.",
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
            topic
                .apply(shoryo_core::Operation::Submit)
                .expect("an applied round can be sent");
        }
    }
    topic
}
