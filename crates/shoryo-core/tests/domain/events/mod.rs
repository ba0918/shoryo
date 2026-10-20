mod stamps;
mod submissions;

use serde_json::json;
use shoryo_core::{
    AskId, AskState, Class, DecisionId, EventKind, Operation, OperationRefusal, QuestionId, Reply,
    Stamp, Topic,
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
fn review_request_marks_decision_in_review_and_records_event() {
    let mut topic = topic_on_a_round();

    topic
        .apply(Operation::RequestReview { decision: d("d1") }, any_time())
        .unwrap();

    assert!(topic.is_in_review(&d("d1")));
    assert_eq!(
        kinds(&topic),
        [&EventKind::ReviewRequested { decision: d("d1") }]
    );
}

#[test]
fn stopping_review_request_removes_in_review() {
    let mut topic = topic_on_a_round();
    topic
        .apply(Operation::RequestReview { decision: d("d1") }, any_time())
        .unwrap();

    topic
        .apply(Operation::StopReview { decision: d("d1") }, any_time())
        .unwrap();

    assert!(!topic.is_in_review(&d("d1")));
    assert_eq!(
        kinds(&topic)[1],
        &EventKind::ReviewStopped { decision: d("d1") }
    );
}

#[test]
fn review_request_can_be_stopped_after_the_next_round_until_its_conclusion() {
    let mut topic = topic_on_a_round();
    topic
        .apply(Operation::RequestReview { decision: d("d1") }, any_time())
        .unwrap();
    submit_current(&mut topic).unwrap();
    topic
        .apply_round(round(
            json!({ "subject": "Third", "questions": [question("q4", &[])] }),
        ))
        .unwrap();

    let stopped = topic.apply(Operation::StopReview { decision: d("d1") }, any_time());
    topic
        .apply(Operation::RequestReview { decision: d("d1") }, any_time())
        .unwrap();
    submit_current(&mut topic).unwrap();
    topic
        .apply_round(round(json!({
            "subject": "Fourth",
            "review_conclusions": [{ "decision": "d1", "outcome": "unchanged" }],
        })))
        .unwrap();
    let after_conclusion = topic.apply(Operation::StopReview { decision: d("d1") }, any_time());

    assert_eq!(stopped, Ok(()));
    assert_eq!(
        after_conclusion,
        Err(OperationRefusal::NotInReview { decision: d("d1") })
    );
}

#[test]
fn review_request_is_refused_while_the_round_is_sent() {
    let mut topic = topic_on_a_round();
    submit_current(&mut topic).unwrap();
    let ids: Vec<_> = topic.pending_events().iter().map(|e| e.id).collect();
    topic.acknowledge(&ids);
    let before = topic.clone();

    let refused = topic.apply(Operation::RequestReview { decision: d("d1") }, any_time());

    assert_eq!(refused, Err(OperationRefusal::ReviewWhileRoundSent));
    assert_eq!(topic, before);
}

#[test]
fn submit_sends_every_answer_at_once() {
    let mut topic = topic_on_a_round();
    topic
        .apply(
            Operation::Choose {
                question: q("q3"),
                option: 1,
            },
            any_time(),
        )
        .unwrap();

    submit_current(&mut topic).unwrap();

    let events = kinds(&topic);
    let [EventKind::Submitted { round, answers }] = events.as_slice() else {
        panic!("expected one submitted event, got {events:?}");
    };
    assert_eq!(*round, 2);
    let choices: Vec<_> = answers.iter().map(|a| (&a.question, a.choice)).collect();
    assert_eq!(choices, [(&q("q2"), Some(0)), (&q("q3"), Some(1))]);
}

#[test]
fn deferred_question_is_not_an_answer() {
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

    submit_current(&mut topic).unwrap();

    let EventKind::Submitted { answers, .. } = kinds(&topic)[0] else {
        panic!("expected the submitted event");
    };
    assert_eq!(answers[0].choice, None);
    assert!(answers[0].deferred);
}

#[test]
fn answers_are_refused_after_submit() {
    let mut topic = topic_on_a_round();
    submit_current(&mut topic).unwrap();

    for operation in [
        Operation::AnswerMethod {
            question: q("q2"),
            method: shoryo_core::AnswerMethod::Custom,
        },
        Operation::FreeText {
            question: q("q2"),
            text: "late".into(),
        },
        Operation::Choose {
            question: q("q2"),
            option: 1,
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
            stamped: false,
        },
        Operation::Submit {
            round: 2,
            expected_answers: None,
        },
    ] {
        assert_eq!(
            topic.apply(operation, any_time()),
            Err(OperationRefusal::RoundSubmitted)
        );
    }
}

#[test]
fn asks_and_swaps_are_refused_after_submit() {
    let mut topic = topic_on_a_round();
    submit_current(&mut topic).unwrap();

    let ask = topic.apply(
        Operation::Ask {
            question: q("q2"),
            text: "late".into(),
            follows: None,
        },
        any_time(),
    );
    let swap = topic.apply(Operation::SwapClass { question: q("q2") }, any_time());

    assert_eq!(ask, Err(OperationRefusal::RoundSubmitted));
    assert_eq!(swap, Err(OperationRefusal::RoundSubmitted));
}

#[test]
fn acknowledged_events_are_not_returned() {
    let mut topic = topic_on_a_round();
    topic
        .apply(Operation::RequestReview { decision: d("d1") }, any_time())
        .unwrap();
    submit_current(&mut topic).unwrap();
    let first = topic.pending_events()[0].id;

    topic.acknowledge(&[first]);

    let left: Vec<_> = topic.pending_events().iter().map(|e| e.id).collect();
    assert_eq!(left.len(), 1);
    assert!(!left.contains(&first));
}

#[test]
fn acknowledging_duplicates_and_unknown_events_preserves_pending_order() {
    let mut topic = topic_on_a_round();
    for _ in 0..5 {
        topic
            .apply(Operation::SwapClass { question: q("q2") }, any_time())
            .unwrap();
    }
    let before = topic.pending_events().to_vec();
    topic.acknowledge(&[]);
    assert_eq!(topic.pending_events(), before);
    topic.acknowledge(&[
        before[3].id,
        shoryo_core::EventId(99),
        before[1].id,
        before[3].id,
    ]);
    assert_eq!(
        topic.pending_events(),
        [before[0].clone(), before[2].clone(), before[4].clone()]
    );
    topic.acknowledge(&[before[2].id]);
    assert_eq!(
        topic.pending_events(),
        [before[0].clone(), before[4].clone()]
    );
    topic
        .apply(Operation::SwapClass { question: q("q2") }, any_time())
        .unwrap();
    assert!(topic.pending_events().last().unwrap().id > before[4].id);
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

#[test]
fn submit_made_on_an_earlier_round_does_not_send_the_current_one() {
    let mut topic = topic_on_a_round();

    let refused = topic.apply(
        Operation::Submit {
            round: 1,
            expected_answers: None,
        },
        any_time(),
    );

    assert_eq!(refused, Err(OperationRefusal::NotCurrentRound { round: 1 }));
    assert!(!topic.current_round().expect("round 2 is out").submitted);
    assert!(topic.pending_events().is_empty());
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
