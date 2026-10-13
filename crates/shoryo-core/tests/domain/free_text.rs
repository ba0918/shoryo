use super::common::{any_time, question, topic_from};
use serde_json::json;
use shoryo_core::{Operation, Topic};

fn apply(topic: &mut Topic, value: serde_json::Value) {
    topic
        .apply(
            serde_json::from_value::<Operation>(value)
                .expect("the test operation has the declared wire shape"),
            any_time(),
        )
        .expect("the test's valid operation is accepted");
}

fn custom(text: &str) -> Topic {
    let mut topic = topic_from(vec![
        json!({"subject":"Answers", "questions":[question("q1", &[])]}),
    ]);
    apply(
        &mut topic,
        json!({"op":"answer_method","question":"q1","method":"custom"}),
    );
    apply(
        &mut topic,
        json!({"op":"free_text","question":"q1","text":text}),
    );
    topic
}

#[test]
fn custom_projection_refuses_preapproval_and_invalid_retained_options_without_mutation() {
    let mut topic = custom("valid");
    topic.rounds[0].questions[0].answer.stamp = Some(shoryo_core::Stamp::PreApproved);
    let before = topic.clone();
    assert!(
        topic
            .apply(
                serde_json::from_value(json!({"op":"submit","round":1})).unwrap(),
                any_time()
            )
            .is_err()
    );
    assert_eq!(topic, before);
    topic.rounds[0].questions[0].answer.stamp = Some(shoryo_core::Stamp::Person);
    topic.rounds[0].questions[0].answer.selected = 99;
    let before = topic.clone();
    assert!(
        topic
            .apply(
                serde_json::from_value(json!({"op":"submit","round":1})).unwrap(),
                any_time()
            )
            .is_err()
    );
    assert_eq!(topic, before);
}

#[test]
fn byte_order_mark_is_a_nonblank_custom_body() {
    let mut topic = custom("\u{feff}");
    apply(
        &mut topic,
        json!({"op":"stamp","question":"q1","stamped":true}),
    );
    apply(&mut topic, json!({"op":"submit","round":1}));
    let event = serde_json::to_value(topic.pending_events().last().unwrap()).unwrap();
    assert_eq!(event["answers"][0]["free_text"], "\u{feff}");
}

#[test]
fn changed_custom_text_requires_a_new_stamp_but_identical_saves_do_not() {
    let mut topic = custom("one");
    apply(
        &mut topic,
        json!({"op":"stamp","question":"q1","stamped":true}),
    );
    let stamped = topic.current_round().unwrap().questions[0].answer.clone();
    apply(
        &mut topic,
        json!({"op":"free_text","question":"q1","text":"one"}),
    );
    apply(
        &mut topic,
        json!({"op":"answer_method","question":"q1","method":"custom"}),
    );
    assert_eq!(topic.current_round().unwrap().questions[0].answer, stamped);
    apply(
        &mut topic,
        json!({"op":"free_text","question":"q1","text":"two"}),
    );
    assert_eq!(
        topic.current_round().unwrap().questions[0].answer.stamp,
        None
    );
    assert_eq!(
        topic.current_round().unwrap().questions[0]
            .answer
            .stamped_at,
        None
    );
}

#[test]
fn invalid_custom_send_leaves_the_entire_topic_unchanged() {
    let mut topic = custom("valid");
    apply(
        &mut topic,
        json!({"op":"stamp","question":"q1","stamped":true}),
    );
    topic.rounds[0].questions[0].answer.free_text = "　".into();
    let before = topic.clone();
    assert!(
        topic
            .apply(
                serde_json::from_value(json!({"op":"submit","round":1})).unwrap(),
                any_time()
            )
            .is_err()
    );
    assert_eq!(topic, before);
}

#[test]
fn old_pending_answers_keep_their_meaning_and_malformed_variants_are_refused() {
    let mut topic = topic_from(vec![
        json!({"subject":"Answers", "questions":[question("q1", &[])]}),
    ]);
    apply(
        &mut topic,
        json!({"op":"stamp","question":"q1","stamped":true}),
    );
    apply(&mut topic, json!({"op":"submit","round":1}));
    let mut old = serde_json::to_value(&topic).unwrap();
    old["rounds"][0]["questions"][0]["answer"]
        .as_object_mut()
        .unwrap()
        .remove("method");
    old["rounds"][0]["questions"][0]["answer"]
        .as_object_mut()
        .unwrap()
        .remove("free_text");
    let loaded = Topic::from_json(&old.to_string()).unwrap();
    assert_eq!(loaded.pending_events(), topic.pending_events());
    old["events"][0]["answers"][0]["choice"] = serde_json::Value::Null;
    assert!(Topic::from_json(&old.to_string()).is_err());
}

#[test]
fn empty_custom_confirmation_leaves_topic_unchanged() {
    for blank in ["", " \n\t", "　", "\u{85}"] {
        let mut topic = custom(blank);
        let before = topic.clone();
        let result = topic.apply(
            serde_json::from_value(json!({"op":"stamp","question":"q1","stamped":true})).unwrap(),
            any_time(),
        );
        assert!(result.is_err(), "{blank:?}");
        assert_eq!(topic, before);
    }
}

#[test]
fn custom_class_swaps_never_preapprove() {
    let mut topic = custom("My answer");
    apply(
        &mut topic,
        json!({"op":"stamp","question":"q1","stamped":true}),
    );
    apply(&mut topic, json!({"op":"swap_class","question":"q1"}));
    assert_eq!(
        topic.current_round().unwrap().questions[0].answer.stamp,
        None
    );
}

#[test]
fn custom_send_excludes_inactive_drafts_and_preserves_exact_text() {
    let mut topic = custom("  My answer\n<script>literal</script>  ");
    apply(
        &mut topic,
        json!({"op":"note","question":"q1","text":"hidden note"}),
    );
    apply(
        &mut topic,
        json!({"op":"stamp","question":"q1","stamped":true}),
    );
    apply(&mut topic, json!({"op":"submit","round":1}));
    let event = serde_json::to_value(topic.pending_events().last().unwrap()).unwrap();
    assert_eq!(event["answers"][0]["choice"], serde_json::Value::Null);
    assert_eq!(
        event["answers"][0]["free_text"],
        "  My answer\n<script>literal</script>  "
    );
    assert_eq!(event["answers"][0]["note"], "");
}

#[test]
fn deferred_custom_send_excludes_all_drafts() {
    let mut topic = custom("");
    apply(
        &mut topic,
        json!({"op":"note","question":"q1","text":"hidden"}),
    );
    apply(
        &mut topic,
        json!({"op":"defer","question":"q1","deferred":true}),
    );
    apply(
        &mut topic,
        json!({"op":"stamp","question":"q1","stamped":true}),
    );
    apply(&mut topic, json!({"op":"submit","round":1}));
    let event = serde_json::to_value(topic.pending_events().last().unwrap()).unwrap();
    assert!(event["answers"][0].get("free_text").is_none());
    assert_eq!(event["answers"][0]["note"], "");
    assert_eq!(event["answers"][0]["deferred"], true);
}

#[test]
fn custom_drafts_survive_method_switches() {
    let mut topic = topic_from(vec![
        json!({"subject":"Answers", "questions":[question("q1", &[])]}),
    ]);
    apply(
        &mut topic,
        json!({"op":"note","question":"q1","text":"retained note"}),
    );
    apply(
        &mut topic,
        json!({"op":"free_text","question":"q1","text":"  My answer\ncondition  "}),
    );
    apply(
        &mut topic,
        json!({"op":"answer_method","question":"q1","method":"custom"}),
    );
    apply(
        &mut topic,
        json!({"op":"choose","question":"q1","option":1}),
    );
    apply(
        &mut topic,
        json!({"op":"answer_method","question":"q1","method":"custom"}),
    );
    let answer = serde_json::to_value(&topic.current_round().unwrap().questions[0].answer).unwrap();
    assert_eq!(answer["method"], "custom");
    assert_eq!(answer["selected"], 1);
    assert_eq!(answer["free_text"], "  My answer\ncondition  ");
    assert_eq!(answer["note"], "retained note");
}
