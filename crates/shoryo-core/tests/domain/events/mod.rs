//! The person's operations on the screen, the events they leave for the agent, and the
//! agent's replies. This module holds the edits of an open answer; the others are split by
//! what they act on.

mod acknowledgement;
mod asks;
mod end;
mod reviews;
mod stamps;
mod submissions;

use serde_json::json;
use shoryo_core::{
    Class, DecisionId, EventKind, Operation, OperationRefusal, QuestionId, Stamp, Topic,
};

use crate::common::{any_time, decision, question, round, submit_current};

fn q(id: &str) -> QuestionId {
    QuestionId::new(id)
}

fn d(id: &str) -> DecisionId {
    DecisionId::new(id)
}

/// A topic on its second round: q2 (human) and q3 (provisional), resting on decision d1.
fn topic_on_a_round() -> Topic {
    let mut topic = Topic::new("", "");
    topic
        .apply_round(round(json!({
            "subject": "First",
            "questions": [question("q1", &[])],
        })))
        .expect("the first round is valid");
    submit_current(&mut topic).expect("the first round can be sent");
    let mut provisional = question("q3", &["d1"]);
    provisional["class"] = json!("provisional");
    topic
        .apply_round(round(json!({
            "subject": "Second",
            "questions": [question("q2", &["d1"]), provisional],
            "records": { "decisions": [decision("d1", "q1", "One file")] },
        })))
        .expect("the second round is valid");
    let ids: Vec<_> = topic.pending_events().iter().map(|e| e.id).collect();
    topic.acknowledge(&ids);
    topic
}

fn current_answer<'a>(topic: &'a Topic, id: &str) -> &'a shoryo_core::Answer {
    &topic
        .current_round()
        .expect("the topic has a round")
        .questions
        .iter()
        .find(|question| question.id == q(id))
        .expect("the question is in the current round")
        .answer
}

fn kinds(topic: &Topic) -> Vec<&EventKind> {
    topic.pending_events().iter().map(|e| &e.kind).collect()
}

#[test]
fn choosing_an_option_selects_it() {
    let mut topic = topic_on_a_round();

    topic
        .apply(
            Operation::Choose {
                question: q("q2"),
                option: 1,
            },
            any_time(),
        )
        .unwrap();

    assert_eq!(current_answer(&topic, "q2").selected, 1);
}

#[test]
fn choosing_a_missing_option_is_refused() {
    let mut topic = topic_on_a_round();

    let refusal = topic
        .apply(
            Operation::Choose {
                question: q("q2"),
                option: 5,
            },
            any_time(),
        )
        .unwrap_err();

    assert_eq!(refusal, OperationRefusal::UnknownOption { option: 5 });
}

#[test]
fn writing_a_note_keeps_it() {
    let mut topic = topic_on_a_round();

    topic
        .apply(
            Operation::Note {
                question: q("q2"),
                text: "Only for now.".into(),
            },
            any_time(),
        )
        .unwrap();

    assert_eq!(current_answer(&topic, "q2").note, "Only for now.");
}

#[test]
fn defer_switch_can_be_set_and_cleared() {
    let mut topic = topic_on_a_round();

    topic
        .apply(
            Operation::Defer {
                question: q("q2"),
                deferred: true,
            },
            any_time(),
        )
        .unwrap();
    assert!(current_answer(&topic, "q2").deferred);
    topic
        .apply(
            Operation::Defer {
                question: q("q2"),
                deferred: false,
            },
            any_time(),
        )
        .unwrap();

    assert!(!current_answer(&topic, "q2").deferred);
}

#[test]
fn swapping_class_moves_question_and_records_event() {
    let mut topic = topic_on_a_round();

    topic
        .apply(Operation::SwapClass { question: q("q3") }, any_time())
        .unwrap();

    let swapped = topic.current_round().unwrap().questions[1].class;
    assert_eq!(swapped, Class::Human);
    assert_eq!(
        kinds(&topic),
        [&EventKind::ClassSwapped {
            question: q("q3"),
            class: Class::Human
        }]
    );
}

fn stamp(topic: &mut Topic, id: &str, stamped: bool) {
    topic
        .apply(
            Operation::Stamp {
                question: q(id),
                stamped,
            },
            any_time(),
        )
        .expect("a question of the open round can be stamped");
}

fn stamp_of(topic: &Topic, id: &str) -> Option<Stamp> {
    current_answer(topic, id).stamp
}

#[test]
fn edits_of_an_unknown_question_never_change_state_or_allocate_events() {
    let mut topic = topic_on_a_round();
    let before = topic.clone();
    for operation in [
        Operation::AnswerMethod {
            question: q("absent"),
            method: shoryo_core::AnswerMethod::Custom,
        },
        Operation::FreeText {
            question: q("absent"),
            text: "answer".into(),
        },
        Operation::Choose {
            question: q("absent"),
            option: 0,
        },
        Operation::Note {
            question: q("absent"),
            text: "note".into(),
        },
        Operation::Defer {
            question: q("absent"),
            deferred: true,
        },
        Operation::Stamp {
            question: q("absent"),
            stamped: true,
        },
        Operation::SwapClass {
            question: q("absent"),
        },
        Operation::Ask {
            question: q("absent"),
            text: "why?".into(),
            follows: None,
        },
    ] {
        assert_eq!(
            topic.apply(operation, any_time()),
            Err(OperationRefusal::UnknownQuestion {
                question: q("absent")
            })
        );
        assert_eq!(topic, before);
    }
}
