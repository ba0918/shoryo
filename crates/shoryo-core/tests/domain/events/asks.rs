//! 質問: asking about a question before sending, follow-ups, and the agent's replies.

use super::{kinds, q, topic_on_a_round};
use crate::common::{any_time, question, round, submit_current};
use serde_json::json;
use shoryo_core::{AskId, AskState, EventKind, Operation, OperationRefusal, Reply};

#[test]
fn asking_records_a_waiting_ask_and_an_event() {
    let mut topic = topic_on_a_round();

    topic
        .apply(
            Operation::Ask {
                question: q("q2"),
                text: "Explain more".into(),
                follows: None,
            },
            any_time(),
        )
        .unwrap();

    let ask = &topic.current_round().unwrap().asks[0];
    assert_eq!(ask.state, AskState::Waiting);
    assert_eq!(
        kinds(&topic),
        [&EventKind::Ask {
            ask: ask.id,
            round: 2,
            question: q("q2"),
            text: "Explain more".into(),
            follows: None,
        }]
    );
}

#[test]
fn follow_up_names_the_ask_it_continues() {
    let mut topic = topic_on_a_round();
    topic
        .apply(
            Operation::Ask {
                question: q("q2"),
                text: "Explain more".into(),
                follows: None,
            },
            any_time(),
        )
        .unwrap();
    let first = topic.current_round().unwrap().asks[0].id;

    topic
        .apply(
            Operation::Ask {
                question: q("q2"),
                text: "And then?".into(),
                follows: Some(first),
            },
            any_time(),
        )
        .unwrap();

    assert_eq!(topic.current_round().unwrap().asks[1].follows, Some(first));
}

#[test]
fn follow_up_to_an_unknown_ask_is_refused() {
    let mut topic = topic_on_a_round();

    let refusal = topic
        .apply(
            Operation::Ask {
                question: q("q2"),
                text: "And then?".into(),
                follows: Some(AskId(99)),
            },
            any_time(),
        )
        .unwrap_err();

    assert_eq!(refusal, OperationRefusal::UnknownAsk { ask: AskId(99) });
}

#[test]
fn follow_up_to_an_ask_of_another_question_is_refused() {
    let mut topic = topic_on_a_round();
    topic
        .apply(
            Operation::Ask {
                question: q("q3"),
                text: "Explain more".into(),
                follows: None,
            },
            any_time(),
        )
        .unwrap();
    let other = topic.current_round().unwrap().asks[0].id;

    let refusal = topic
        .apply(
            Operation::Ask {
                question: q("q2"),
                text: "And then?".into(),
                follows: Some(other),
            },
            any_time(),
        )
        .unwrap_err();

    assert_eq!(
        refusal,
        OperationRefusal::FollowsAnotherQuestion { ask: other }
    );
    assert_eq!(topic.current_round().unwrap().asks.len(), 1);
}

#[test]
fn reply_appears_with_its_ask() {
    let mut topic = topic_on_a_round();
    topic
        .apply(
            Operation::Ask {
                question: q("q2"),
                text: "Explain more".into(),
                follows: None,
            },
            any_time(),
        )
        .unwrap();
    let ask = topic.current_round().unwrap().asks[0].id;
    let reply = Reply {
        parts: vec![
            shoryo_core::Part::Text {
                body: "It means one file.".into(),
            },
            shoryo_core::Part::Diagram {
                source: "a = File".into(),
                title: "File".into(),
                role: shoryo_core::Role::Example,
            },
        ],
    };

    topic.reply(ask, reply.clone()).unwrap();

    assert_eq!(
        topic.current_round().unwrap().asks[0].state,
        AskState::Replied(reply)
    );
}

#[test]
fn reply_to_ask_in_submitted_round_is_kept_with_that_round() {
    let mut topic = topic_on_a_round();
    topic
        .apply(
            Operation::Ask {
                question: q("q2"),
                text: "Explain more".into(),
                follows: None,
            },
            any_time(),
        )
        .unwrap();
    let ask = topic.current_round().unwrap().asks[0].id;
    submit_current(&mut topic).unwrap();
    topic
        .apply_round(round(
            json!({ "subject": "Third", "questions": [question("q4", &[])] }),
        ))
        .unwrap();

    topic
        .reply(
            ask,
            Reply {
                parts: vec![shoryo_core::Part::Text {
                    body: "Late reply.".into(),
                }],
            },
        )
        .unwrap();

    assert!(matches!(
        &topic.rounds[1].asks[0].state,
        AskState::Replied(reply) if reply.parts == vec![shoryo_core::Part::Text {body:"Late reply.".into()}]
    ));
}
