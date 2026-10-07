use serde_json::json;
use shoryo_core::{
    AskId, AskState, Class, DecisionId, EventKind, Operation, OperationRefusal, QuestionId, Reply,
    Stamp, Topic,
};

use crate::common::{decision, question, round, submit_current};

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
        .apply(Operation::Choose {
            question: q("q2"),
            option: 1,
        })
        .unwrap();

    assert_eq!(current_answer(&topic, "q2").selected, 1);
}

#[test]
fn choosing_a_missing_option_is_refused() {
    let mut topic = topic_on_a_round();

    let refusal = topic
        .apply(Operation::Choose {
            question: q("q2"),
            option: 5,
        })
        .unwrap_err();

    assert_eq!(refusal, OperationRefusal::UnknownOption { option: 5 });
}

#[test]
fn writing_a_note_keeps_it() {
    let mut topic = topic_on_a_round();

    topic
        .apply(Operation::Note {
            question: q("q2"),
            text: "Only for now.".into(),
        })
        .unwrap();

    assert_eq!(current_answer(&topic, "q2").note, "Only for now.");
}

#[test]
fn defer_switch_can_be_set_and_cleared() {
    let mut topic = topic_on_a_round();

    topic
        .apply(Operation::Defer {
            question: q("q2"),
            deferred: true,
        })
        .unwrap();
    assert!(current_answer(&topic, "q2").deferred);
    topic
        .apply(Operation::Defer {
            question: q("q2"),
            deferred: false,
        })
        .unwrap();

    assert!(!current_answer(&topic, "q2").deferred);
}

#[test]
fn swapping_class_moves_question_and_records_event() {
    let mut topic = topic_on_a_round();

    topic
        .apply(Operation::SwapClass { question: q("q3") })
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
        .apply(Operation::Ask {
            question: q("q2"),
            text: "Explain more".into(),
            follows: None,
        })
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
        .apply(Operation::Ask {
            question: q("q2"),
            text: "Explain more".into(),
            follows: None,
        })
        .unwrap();
    let first = topic.current_round().unwrap().asks[0].id;

    topic
        .apply(Operation::Ask {
            question: q("q2"),
            text: "And then?".into(),
            follows: Some(first),
        })
        .unwrap();

    assert_eq!(topic.current_round().unwrap().asks[1].follows, Some(first));
}

#[test]
fn follow_up_to_an_unknown_ask_is_refused() {
    let mut topic = topic_on_a_round();

    let refusal = topic
        .apply(Operation::Ask {
            question: q("q2"),
            text: "And then?".into(),
            follows: Some(AskId(99)),
        })
        .unwrap_err();

    assert_eq!(refusal, OperationRefusal::UnknownAsk { ask: AskId(99) });
}

#[test]
fn follow_up_to_an_ask_of_another_question_is_refused() {
    let mut topic = topic_on_a_round();
    topic
        .apply(Operation::Ask {
            question: q("q3"),
            text: "Explain more".into(),
            follows: None,
        })
        .unwrap();
    let other = topic.current_round().unwrap().asks[0].id;

    let refusal = topic
        .apply(Operation::Ask {
            question: q("q2"),
            text: "And then?".into(),
            follows: Some(other),
        })
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
        .apply(Operation::RequestReview { decision: d("d1") })
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
        .apply(Operation::RequestReview { decision: d("d1") })
        .unwrap();

    topic
        .apply(Operation::StopReview { decision: d("d1") })
        .unwrap();

    assert!(!topic.is_in_review(&d("d1")));
    assert_eq!(
        kinds(&topic)[1],
        &EventKind::ReviewStopped { decision: d("d1") }
    );
}

#[test]
fn review_request_cannot_be_stopped_after_next_round() {
    let mut topic = topic_on_a_round();
    topic
        .apply(Operation::RequestReview { decision: d("d1") })
        .unwrap();
    submit_current(&mut topic).unwrap();
    topic
        .apply_round(round(
            json!({ "subject": "Third", "questions": [question("q4", &[])] }),
        ))
        .unwrap();

    let refusal = topic
        .apply(Operation::StopReview { decision: d("d1") })
        .unwrap_err();

    assert_eq!(refusal, OperationRefusal::NextRoundArrived);
    assert!(topic.is_in_review(&d("d1")));
}

#[test]
fn submit_sends_every_answer_at_once() {
    let mut topic = topic_on_a_round();
    topic
        .apply(Operation::Choose {
            question: q("q3"),
            option: 1,
        })
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
        .apply(Operation::Defer {
            question: q("q2"),
            deferred: true,
        })
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
        Operation::Submit { round: 2 },
    ] {
        assert_eq!(
            topic.apply(operation),
            Err(OperationRefusal::RoundSubmitted)
        );
    }
}

#[test]
fn asks_and_swaps_are_refused_after_submit() {
    let mut topic = topic_on_a_round();
    submit_current(&mut topic).unwrap();

    let ask = topic.apply(Operation::Ask {
        question: q("q2"),
        text: "late".into(),
        follows: None,
    });
    let swap = topic.apply(Operation::SwapClass { question: q("q2") });

    assert_eq!(ask, Err(OperationRefusal::RoundSubmitted));
    assert_eq!(swap, Err(OperationRefusal::RoundSubmitted));
}

#[test]
fn unacknowledged_events_are_returned_again() {
    let mut topic = topic_on_a_round();
    topic
        .apply(Operation::RequestReview { decision: d("d1") })
        .unwrap();
    submit_current(&mut topic).unwrap();

    let first: Vec<_> = topic.pending_events().to_vec();
    let again: Vec<_> = topic.pending_events().to_vec();

    assert_eq!(first.len(), 2);
    assert_eq!(first, again);
}

#[test]
fn acknowledged_events_are_not_returned() {
    let mut topic = topic_on_a_round();
    topic
        .apply(Operation::RequestReview { decision: d("d1") })
        .unwrap();
    submit_current(&mut topic).unwrap();
    let first = topic.pending_events()[0].id;

    topic.acknowledge(&[first]);

    let left: Vec<_> = topic.pending_events().iter().map(|e| e.id).collect();
    assert_eq!(left.len(), 1);
    assert!(!left.contains(&first));
}

#[test]
fn reply_appears_with_its_ask() {
    let mut topic = topic_on_a_round();
    topic
        .apply(Operation::Ask {
            question: q("q2"),
            text: "Explain more".into(),
            follows: None,
        })
        .unwrap();
    let ask = topic.current_round().unwrap().asks[0].id;
    let reply = Reply {
        text: "It means one file.".into(),
        diagram: Some("a = File".into()),
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
        .apply(Operation::Ask {
            question: q("q2"),
            text: "Explain more".into(),
            follows: None,
        })
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
                text: "Late reply.".into(),
                diagram: None,
            },
        )
        .unwrap();

    assert!(matches!(
        &topic.rounds[1].asks[0].state,
        AskState::Replied(reply) if reply.text == "Late reply."
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
        .apply(Operation::Ask {
            question: q("q2"),
            text: "Explain more".into(),
            follows: None,
        })
        .unwrap();
    let ask = topic.current_round().unwrap().asks[0].id;
    submit_current(&mut topic).unwrap();
    topic.end().unwrap();

    let refused = topic.reply(
        ask,
        Reply {
            text: "Too late.".into(),
            diagram: None,
        },
    );

    assert_eq!(refused, Err(OperationRefusal::TopicEnded));
}

#[test]
fn pending_ask_shows_no_reply_after_end() {
    let mut topic = topic_on_a_round();
    topic
        .apply(Operation::Ask {
            question: q("q2"),
            text: "Explain more".into(),
            follows: None,
        })
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
        assert_eq!(topic.apply(operation), Err(OperationRefusal::TopicEnded));
    }
}

#[test]
fn submit_made_on_an_earlier_round_does_not_send_the_current_one() {
    let mut topic = topic_on_a_round();

    let refused = topic.apply(Operation::Submit { round: 1 });

    assert_eq!(refused, Err(OperationRefusal::NotCurrentRound { round: 1 }));
    assert!(!topic.current_round().expect("round 2 is out").submitted);
    assert!(topic.pending_events().is_empty());
}

fn stamp(topic: &mut Topic, id: &str, stamped: bool) {
    topic
        .apply(Operation::Stamp {
            question: q(id),
            stamped,
        })
        .expect("a question of the open round can be stamped");
}

fn stamp_of(topic: &Topic, id: &str) -> Option<Stamp> {
    current_answer(topic, id).stamp
}

#[test]
fn human_question_starts_unstamped_and_provisional_starts_pre_approved() {
    let topic = topic_on_a_round();

    assert_eq!(stamp_of(&topic, "q2"), None);
    assert_eq!(stamp_of(&topic, "q3"), Some(Stamp::PreApproved));
}

#[test]
fn changing_choice_removes_stamp_but_changing_note_does_not() {
    let mut topic = topic_on_a_round();
    stamp(&mut topic, "q2", true);

    topic
        .apply(Operation::Note {
            question: q("q2"),
            text: "Only for now.".into(),
        })
        .unwrap();
    assert_eq!(stamp_of(&topic, "q2"), Some(Stamp::Person));
    topic
        .apply(Operation::Choose {
            question: q("q2"),
            option: 1,
        })
        .unwrap();

    assert_eq!(stamp_of(&topic, "q2"), None);
}

#[test]
fn changing_defer_switch_removes_stamp() {
    let mut topic = topic_on_a_round();
    stamp(&mut topic, "q2", true);

    topic
        .apply(Operation::Defer {
            question: q("q2"),
            deferred: true,
        })
        .unwrap();

    assert_eq!(stamp_of(&topic, "q2"), None);
}

#[test]
fn unstamping_pre_approved_row_is_a_send_back() {
    let mut topic = topic_on_a_round();

    stamp(&mut topic, "q3", false);
    assert_eq!(stamp_of(&topic, "q3"), None);
    stamp(&mut topic, "q3", true);

    assert_eq!(stamp_of(&topic, "q3"), Some(Stamp::Person));
}

#[test]
fn swap_resets_stamp_to_new_class_initial_state() {
    let mut topic = topic_on_a_round();
    stamp(&mut topic, "q2", true);

    topic
        .apply(Operation::SwapClass { question: q("q2") })
        .unwrap();
    assert_eq!(stamp_of(&topic, "q2"), Some(Stamp::PreApproved));
    topic
        .apply(Operation::SwapClass { question: q("q3") })
        .unwrap();

    assert_eq!(stamp_of(&topic, "q3"), None);
}

#[test]
fn submit_is_refused_while_a_question_is_unstamped() {
    let mut topic = topic_on_a_round();

    let refused = topic.apply(Operation::Submit { round: 2 });

    assert_eq!(
        refused,
        Err(OperationRefusal::Unstamped {
            questions: vec![q("q2")]
        })
    );
    assert!(!topic.current_round().unwrap().submitted);
    assert!(topic.pending_events().is_empty());
    stamp(&mut topic, "q2", true);
    assert_eq!(topic.apply(Operation::Submit { round: 2 }), Ok(()));
}

#[test]
fn deferred_question_needs_a_stamp() {
    let mut topic = topic_on_a_round();
    stamp(&mut topic, "q2", true);
    topic
        .apply(Operation::Defer {
            question: q("q2"),
            deferred: true,
        })
        .unwrap();

    let refused = topic.apply(Operation::Submit { round: 2 });

    assert_eq!(
        refused,
        Err(OperationRefusal::Unstamped {
            questions: vec![q("q2")]
        })
    );
}

#[test]
fn submitted_event_carries_stamp_kind() {
    let mut topic = topic_on_a_round();
    stamp(&mut topic, "q2", true);

    topic.apply(Operation::Submit { round: 2 }).unwrap();

    let EventKind::Submitted { answers, .. } = kinds(&topic)[0] else {
        panic!("expected the submitted event");
    };
    let stamps: Vec<_> = answers.iter().map(|a| (&a.question, a.stamp)).collect();
    assert_eq!(
        stamps,
        [(&q("q2"), Stamp::Person), (&q("q3"), Stamp::PreApproved)]
    );
}

#[test]
fn round_without_questions_is_submitted_with_no_answers() {
    let mut topic = topic_on_a_round();
    submit_current(&mut topic).unwrap();
    topic
        .apply_round(round(json!({ "subject": "Result" })))
        .unwrap();
    let ids: Vec<_> = topic.pending_events().iter().map(|e| e.id).collect();
    topic.acknowledge(&ids);

    topic.apply(Operation::Submit { round: 3 }).unwrap();

    assert_eq!(
        kinds(&topic),
        [&EventKind::Submitted {
            round: 3,
            answers: vec![]
        }]
    );
}
