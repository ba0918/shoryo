//! Ending the topic, and what is refused once it has ended.

use super::{d, q, topic_on_a_round};
use crate::common::{any_time, submit_current};
use shoryo_core::{AskState, Operation, OperationRefusal, Reply};

#[test]
fn end_is_refused_while_a_round_is_unsent() {
    let mut topic = topic_on_a_round();

    assert_eq!(topic.end(), Err(OperationRefusal::RoundUnsent { round: 2 }));
    assert!(!topic.ended);
}

#[test]
fn wait_is_refused_after_end() {
    let mut topic = topic_on_a_round();
    submit_current(&mut topic).unwrap();

    topic.end().unwrap();

    assert_eq!(topic.check_wait(), Err(OperationRefusal::TopicEnded));
}

#[test]
fn reply_is_refused_after_end() {
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
    topic.end().unwrap();

    let refused = topic.reply(
        ask,
        Reply {
            parts: vec![shoryo_core::Part::Text {
                body: "Too late.".into(),
            }],
        },
    );

    assert_eq!(refused, Err(OperationRefusal::TopicEnded));
}

#[test]
fn pending_ask_shows_no_reply_after_end() {
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
    submit_current(&mut topic).unwrap();

    topic.end().unwrap();

    assert_eq!(topic.rounds[1].asks[0].state, AskState::NoReply);
}

#[test]
fn every_input_is_refused_after_end() {
    let mut topic = topic_on_a_round();
    submit_current(&mut topic).unwrap();
    topic.end().unwrap();

    for operation in [
        Operation::RequestReview { decision: d("d1") },
        Operation::StopReview { decision: d("d1") },
        Operation::AnswerMethod {
            question: q("q2"),
            method: shoryo_core::AnswerMethod::Custom,
        },
        Operation::FreeText {
            question: q("q2"),
            text: "late".into(),
        },
        Operation::Note {
            question: q("q2"),
            text: "late".into(),
        },
        Operation::Defer {
            question: q("q2"),
            deferred: true,
        },
        Operation::Stamp {
            question: q("q2"),
            stamped: true,
        },
        Operation::Submit {
            round: 2,
            expected_answers: None,
        },
        Operation::SwapClass { question: q("q2") },
        Operation::Ask {
            question: q("q2"),
            text: "late".into(),
            follows: None,
        },
        Operation::Choose {
            question: q("q2"),
            option: 1,
        },
    ] {
        assert_eq!(
            topic.apply(operation, any_time()),
            Err(OperationRefusal::TopicEnded)
        );
    }
}
