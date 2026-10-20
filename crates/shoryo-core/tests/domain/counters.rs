use serde_json::json;
use shoryo_core::{
    AskId, DecisionId, EventId, IdExhaustion, Operation, OperationRefusal, QuestionId,
    RoundRefusal, Topic,
};

use crate::common::{any_time, decision, question, round, submit_current, topic_from};

fn with_counter(topic: &Topic, counter: &str, next: u64) -> Topic {
    let mut stored = serde_json::to_value(topic).expect("the topic is serializable");
    stored[counter] = json!(next);
    Topic::from_json(&stored.to_string()).expect("exhausted counters remain readable")
}

fn open_topic() -> Topic {
    let mut topic = topic_from(vec![json!({
        "subject":"Answers", "questions":[question("q1", &[])],
        "records":{"decisions":[decision("d1", "q1", "One")]},
    })]);
    topic
        .apply(
            Operation::Stamp {
                question: QuestionId::new("q1"),
                stamped: true,
            },
            any_time(),
        )
        .expect("the open question can be stamped");
    topic
}

#[test]
fn asking_with_either_identifier_exhausted_allocates_neither_identifier() {
    for (counter, refusal) in [
        ("next_ask", IdExhaustion::Ask),
        ("next_event", IdExhaustion::Event),
    ] {
        let mut topic = with_counter(&open_topic(), counter, u64::MAX);
        let before = topic.clone();
        assert_eq!(
            topic.apply(
                Operation::Ask {
                    question: QuestionId::new("q1"),
                    text: "Why?".into(),
                    follows: None,
                },
                any_time()
            ),
            Err(OperationRefusal::IdExhausted { counter: refusal })
        );
        assert_eq!(topic, before);
    }
}

#[test]
fn the_last_event_identifier_is_used_once_and_stays_exhausted_after_reloading() {
    let swap = || Operation::SwapClass {
        question: QuestionId::new("q1"),
    };
    let mut topic = with_counter(&open_topic(), "next_event", u64::MAX - 1);
    topic.apply(swap(), any_time()).unwrap();
    assert_eq!(topic.pending_events()[0].id, EventId(u64::MAX - 1));
    let mut reloaded = Topic::from_json(&topic.to_json()).unwrap();
    let before = reloaded.clone();
    assert_eq!(
        reloaded.apply(swap(), any_time()),
        Err(OperationRefusal::IdExhausted {
            counter: IdExhaustion::Event
        })
    );
    assert_eq!(reloaded, before);
}

#[test]
fn the_last_ask_and_event_pair_is_saved_once_and_never_reused() {
    let mut topic = with_counter(&open_topic(), "next_ask", u64::MAX - 1);
    topic = with_counter(&topic, "next_event", u64::MAX - 1);
    let ask = Operation::Ask {
        question: QuestionId::new("q1"),
        text: "Why?".into(),
        follows: None,
    };
    topic.apply(ask.clone(), any_time()).unwrap();
    assert_eq!(
        topic.current_round().unwrap().asks[0].id,
        AskId(u64::MAX - 1)
    );
    assert_eq!(topic.pending_events()[0].id, EventId(u64::MAX - 1));
    let mut reloaded = Topic::from_json(&topic.to_json()).unwrap();
    let before = reloaded.clone();
    assert_eq!(
        reloaded.apply(ask, any_time()),
        Err(OperationRefusal::IdExhausted {
            counter: IdExhaustion::Ask
        })
    );
    assert_eq!(reloaded, before);
}

#[test]
fn event_exhaustion_refuses_class_swaps_reviews_and_submission_without_partial_changes() {
    for operation in [
        Operation::SwapClass {
            question: QuestionId::new("q1"),
        },
        Operation::RequestReview {
            decision: DecisionId::new("d1"),
        },
        Operation::StopReview {
            decision: DecisionId::new("d1"),
        },
        Operation::Submit {
            round: 1,
            expected_answers: None,
        },
    ] {
        let mut topic = open_topic();
        if matches!(&operation, Operation::StopReview { .. }) {
            topic
                .apply(
                    Operation::RequestReview {
                        decision: DecisionId::new("d1"),
                    },
                    any_time(),
                )
                .unwrap();
        }
        let mut topic = with_counter(&topic, "next_event", u64::MAX);
        let before = topic.clone();
        assert_eq!(
            topic.apply(operation, any_time()),
            Err(OperationRefusal::IdExhausted {
                counter: IdExhaustion::Event
            })
        );
        assert_eq!(topic, before);
    }
}

#[test]
fn exhaustion_does_not_block_edits_or_an_already_requested_review() {
    let mut topic = open_topic();
    topic
        .apply(
            Operation::RequestReview {
                decision: DecisionId::new("d1"),
            },
            any_time(),
        )
        .unwrap();
    let mut topic = with_counter(&topic, "next_event", u64::MAX);
    let before = topic.clone();
    topic
        .apply(
            Operation::RequestReview {
                decision: DecisionId::new("d1"),
            },
            any_time(),
        )
        .unwrap();
    assert_eq!(topic, before);
    topic
        .apply(
            Operation::Note {
                question: QuestionId::new("q1"),
                text: "Keep this".into(),
            },
            any_time(),
        )
        .unwrap();
    assert_eq!(
        topic.current_round().unwrap().questions[0].answer.note,
        "Keep this"
    );
    assert_eq!(topic.pending_events(), before.pending_events());
}

#[test]
fn exhaustion_errors_keep_their_kind_and_counter_in_the_page_api() {
    for (counter, name) in [(IdExhaustion::Ask, "ask"), (IdExhaustion::Event, "event")] {
        assert_eq!(
            serde_json::to_value(OperationRefusal::IdExhausted { counter }).unwrap(),
            json!({"kind":"id_exhausted", "counter":name})
        );
    }
}

fn submitted_topic_at(number: u32) -> Topic {
    let mut topic = topic_from(vec![json!({"subject":"First"})]);
    submit_current(&mut topic).expect("the first round can be sent");
    let received: Vec<_> = topic
        .pending_events()
        .iter()
        .map(|event| event.id)
        .collect();
    topic.acknowledge(&received);
    topic.rounds[0].number = number;
    Topic::from_json(&topic.to_json()).expect("the last round number remains readable")
}

#[test]
fn exhausted_round_numbers_leave_metadata_records_and_ended_state_unchanged() {
    let mut topic = submitted_topic_at(u32::MAX);
    topic.end().unwrap();
    let before = topic.clone();
    assert_eq!(
        topic.apply_round(round(json!({
            "title":"Changed", "original_request":"Changed", "finished_picture":"Changed",
            "subject":"Next", "records":{"not_building":["Changed"]},
        }))),
        Err(RoundRefusal::RoundNumberExhausted)
    );
    assert_eq!(topic, before);
}

#[test]
fn the_last_round_number_can_be_used_and_exhaustion_survives_reloading() {
    let mut topic = submitted_topic_at(u32::MAX - 1);
    assert_eq!(
        topic.apply_round(round(json!({"subject":"Last"}))),
        Ok(u32::MAX)
    );
    submit_current(&mut topic).unwrap();
    let mut loaded = Topic::from_json(&topic.to_json()).unwrap();
    let before = loaded.clone();
    for _ in 0..2 {
        assert_eq!(
            loaded.apply_round(round(json!({"subject":"Refused"}))),
            Err(RoundRefusal::RoundNumberExhausted)
        );
        assert_eq!(loaded, before);
    }
}

#[test]
fn unsent_round_refusal_keeps_priority_over_number_exhaustion() {
    let mut topic = topic_from(vec![json!({"subject":"First"})]);
    topic.rounds[0].number = u32::MAX;
    let before = topic.clone();
    assert_eq!(
        topic.apply_round(round(json!({"subject":"Next"}))),
        Err(RoundRefusal::PreviousRoundNotSubmitted { round: u32::MAX })
    );
    assert_eq!(topic, before);
}
