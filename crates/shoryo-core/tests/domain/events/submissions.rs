use super::{d, kinds, q, stamp, topic_on_a_round};
use crate::common::{any_time, round, submit_current};
use serde_json::json;
use shoryo_core::{EventKind, Operation, OperationRefusal, SentAs, Stamp, Timestamp, Topic};

#[test]
fn submit_is_refused_while_a_question_is_unstamped() {
    let mut topic = topic_on_a_round();
    let refused = topic.apply(
        Operation::Submit {
            round: 2,
            expected_answers: None,
        },
        any_time(),
    );
    assert_eq!(
        refused,
        Err(OperationRefusal::Unstamped {
            questions: vec![q("q2")]
        })
    );
    assert!(!topic.current_round().unwrap().submitted);
    assert!(topic.pending_events().is_empty());
    stamp(&mut topic, "q2", true);
    assert_eq!(
        topic.apply(
            Operation::Submit {
                round: 2,
                expected_answers: None
            },
            any_time()
        ),
        Ok(())
    );
}

#[test]
fn deferred_question_needs_a_stamp() {
    let mut topic = topic_on_a_round();
    stamp(&mut topic, "q2", true);
    topic
        .apply(
            Operation::Defer {
                question: q("q2"),
                deferred: true,
            },
            any_time(),
        )
        .unwrap();
    let refused = topic.apply(
        Operation::Submit {
            round: 2,
            expected_answers: None,
        },
        any_time(),
    );
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
    topic
        .apply(
            Operation::Submit {
                round: 2,
                expected_answers: None,
            },
            any_time(),
        )
        .unwrap();
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
    topic
        .apply(
            Operation::Submit {
                round: 3,
                expected_answers: None,
            },
            any_time(),
        )
        .unwrap();
    assert_eq!(
        kinds(&topic),
        [&EventKind::Submitted {
            round: 3,
            answers: vec![]
        }]
    );
}

#[test]
fn submitting_records_the_sent_time() {
    let mut topic = topic_on_a_round();
    stamp(&mut topic, "q2", true);
    topic
        .apply(
            Operation::Submit {
                round: 2,
                expected_answers: None,
            },
            Timestamp::new("2026-10-08T04:05:00.000Z"),
        )
        .unwrap();
    let round = topic.current_round().unwrap();
    assert!(round.submitted);
    assert_eq!(
        round.sent_at,
        Some(Timestamp::new("2026-10-08T04:05:00.000Z"))
    );
}

fn topic_on_the_result() -> Topic {
    let mut topic = topic_on_a_round();
    submit_current(&mut topic).expect("the second round can be sent");
    topic
        .apply_round(round(json!({ "subject": "Result" })))
        .expect("a round without questions is valid");
    topic
}

#[test]
fn result_round_records_whether_it_proceeded_or_asked_for_review() {
    let mut proceeded = topic_on_the_result();
    let mut reviewed = topic_on_the_result();
    reviewed
        .apply(Operation::RequestReview { decision: d("d1") }, any_time())
        .unwrap();
    proceeded
        .apply(
            Operation::Submit {
                round: 3,
                expected_answers: None,
            },
            any_time(),
        )
        .unwrap();
    reviewed
        .apply(
            Operation::Submit {
                round: 3,
                expected_answers: None,
            },
            any_time(),
        )
        .unwrap();
    assert_eq!(
        proceeded.current_round().unwrap().sent_as,
        Some(SentAs::Proceeded)
    );
    assert_eq!(
        reviewed.current_round().unwrap().sent_as,
        Some(SentAs::ReviewRequested)
    );
}

#[test]
fn a_round_with_questions_records_no_way_of_sending() {
    let mut topic = topic_on_a_round();
    submit_current(&mut topic).unwrap();
    assert_eq!(topic.current_round().unwrap().sent_as, None);
}

#[test]
fn confirmation_of_unchanged_answers_sends_exactly_those_answers() {
    let mut topic = topic_on_a_round();
    stamp(&mut topic, "q2", true);
    let answers = topic.current_round().unwrap().sent_answers().unwrap();
    topic
        .apply(
            Operation::Submit {
                round: 2,
                expected_answers: Some(answers.clone()),
            },
            any_time(),
        )
        .unwrap();
    assert_eq!(kinds(&topic), [&EventKind::Submitted { round: 2, answers }]);
}

#[test]
fn stale_or_malformed_confirmation_leaves_the_entire_topic_unchanged() {
    let mut topic = topic_on_a_round();
    stamp(&mut topic, "q2", true);
    let confirmed = topic.current_round().unwrap().sent_answers().unwrap();
    topic
        .apply(
            Operation::Note {
                question: q("q2"),
                text: "Changed after confirmation".into(),
            },
            any_time(),
        )
        .unwrap();
    let before = topic.clone();
    assert_eq!(
        topic.apply(
            Operation::Submit {
                round: 2,
                expected_answers: Some(confirmed.clone())
            },
            any_time()
        ),
        Err(OperationRefusal::AnswersChanged),
    );
    assert_eq!(topic, before);

    let mut malformed = confirmed;
    malformed[0].choice = None;
    assert_eq!(
        topic.apply(
            Operation::Submit {
                round: 2,
                expected_answers: Some(malformed)
            },
            any_time()
        ),
        Err(OperationRefusal::InvalidAnswer { question: q("q2") }),
    );
    assert_eq!(topic, before);
}
