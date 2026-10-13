use super::{current_answer, q, stamp, stamp_of, topic_on_a_round};
use crate::common::any_time;
use shoryo_core::{Operation, Stamp, Timestamp, Topic};

#[test]
fn pressing_a_stamp_records_the_time_and_lifting_it_clears_it() {
    let mut topic = topic_on_a_round();
    let press = |topic: &mut Topic, stamped, at: &str| {
        topic
            .apply(
                Operation::Stamp {
                    question: q("q2"),
                    stamped,
                },
                Timestamp::new(at),
            )
            .unwrap();
    };
    press(&mut topic, true, "2026-10-08T04:01:00.000Z");
    let first = current_answer(&topic, "q2").stamped_at.clone();
    press(&mut topic, true, "2026-10-08T04:02:00.000Z");
    let pressed_twice = current_answer(&topic, "q2").stamped_at.clone();
    press(&mut topic, false, "2026-10-08T04:03:00.000Z");
    let lifted = current_answer(&topic, "q2").stamped_at.clone();
    press(&mut topic, true, "2026-10-08T04:04:00.000Z");
    let pressed_again = current_answer(&topic, "q2").stamped_at.clone();
    assert_eq!(first, Some(Timestamp::new("2026-10-08T04:01:00.000Z")));
    assert_eq!(pressed_twice, first);
    assert_eq!(lifted, None);
    assert_eq!(
        pressed_again,
        Some(Timestamp::new("2026-10-08T04:04:00.000Z"))
    );
}

#[test]
fn stamp_time_goes_with_the_stamp_when_the_answer_changes() {
    let mut topic = topic_on_a_round();
    stamp(&mut topic, "q2", true);
    topic
        .apply(
            Operation::Choose {
                question: q("q2"),
                option: 1,
            },
            any_time(),
        )
        .unwrap();
    let after_choose = current_answer(&topic, "q2").stamped_at.clone();
    stamp(&mut topic, "q2", true);
    topic
        .apply(Operation::SwapClass { question: q("q2") }, any_time())
        .unwrap();
    assert_eq!(after_choose, None);
    assert_eq!(current_answer(&topic, "q2").stamped_at, None);
}

#[test]
fn pre_approved_stamp_has_no_time_of_its_own() {
    let topic = topic_on_a_round();
    assert_eq!(current_answer(&topic, "q3").stamped_at, None);
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
        .apply(
            Operation::Note {
                question: q("q2"),
                text: "Only for now.".into(),
            },
            any_time(),
        )
        .unwrap();
    assert_eq!(stamp_of(&topic, "q2"), Some(Stamp::Person));
    topic
        .apply(
            Operation::Choose {
                question: q("q2"),
                option: 1,
            },
            any_time(),
        )
        .unwrap();
    assert_eq!(stamp_of(&topic, "q2"), None);
}

#[test]
fn changing_defer_switch_removes_stamp() {
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
        .apply(Operation::SwapClass { question: q("q2") }, any_time())
        .unwrap();
    assert_eq!(stamp_of(&topic, "q2"), Some(Stamp::PreApproved));
    topic
        .apply(Operation::SwapClass { question: q("q3") }, any_time())
        .unwrap();
    assert_eq!(stamp_of(&topic, "q3"), None);
}

#[test]
fn saving_identical_choice_and_defer_values_keeps_the_stamp_and_its_time() {
    let mut topic = topic_on_a_round();
    stamp(&mut topic, "q2", true);
    let before = current_answer(&topic, "q2").clone();
    for operation in [
        Operation::Choose {
            question: q("q2"),
            option: before.selected,
        },
        Operation::Defer {
            question: q("q2"),
            deferred: before.deferred,
        },
        Operation::AnswerMethod {
            question: q("q2"),
            method: before.method,
        },
    ] {
        topic
            .apply(operation, Timestamp::new("2026-10-08T05:00:00.000Z"))
            .unwrap();
        assert_eq!(current_answer(&topic, "q2"), &before);
    }
}
