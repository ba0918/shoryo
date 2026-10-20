//! 見直す: asking for a decision to be reviewed, and stopping that request.

use super::{d, kinds, topic_on_a_round};
use crate::common::{any_time, question, round, submit_current};
use serde_json::json;
use shoryo_core::{EventKind, Operation, OperationRefusal};

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
