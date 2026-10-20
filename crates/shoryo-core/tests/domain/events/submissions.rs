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
