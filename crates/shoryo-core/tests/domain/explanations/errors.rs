use super::{accept, flow, input, sequence};
use serde_json::{Value, json};
use shoryo_core::{
    AskId, ExplanationError, ExplanationRule, LoadError, Operation, QuestionId, RoundInput,
    RoundRefusal, Topic,
};

#[test]
fn invalid_diagrams_report_the_failed_rule_and_location_without_changing_the_topic() {
    let mut missing_participant = sequence();
    missing_participant["events"][0]["to"] = json!("Absent");
    let mut missing_condition = flow();
    missing_condition["edges"][0]
        .as_object_mut()
        .unwrap()
        .remove("label");
    for (part, location, rule) in [
        (
            missing_participant,
            "events[0].to",
            ExplanationRule::Reference,
        ),
        (
            missing_condition,
            "edges[0].label",
            ExplanationRule::MissingCondition,
        ),
        (
            json!({"type":"diagram", "title":"Duplicate", "role":"example", "source":"a = One\na = Two"}),
            "source.line[2]",
            ExplanationRule::Duplicate,
        ),
    ] {
        let mut topic = Topic::new("Original", "Keep this");
        let before = topic.clone();
        let round = RoundInput::from_json(&input(json!([part])).to_string()).unwrap();
        assert_eq!(
            topic.apply_round(round),
            Err(RoundRefusal::Explanation(ExplanationError {
                path: format!("question q1 background.part[0].{location}"),
                rule,
            }))
        );
        assert_eq!(topic, before);
    }
}

#[test]
fn invalid_stored_explanations_report_their_round_and_location() {
    let mut topic = accept(json!([{"type":"text","body":"Fine"}]));
    topic
        .apply(
            Operation::Ask {
                question: QuestionId::new("q1"),
                text: "Why?".into(),
                follows: None,
            },
            crate::common::any_time(),
        )
        .unwrap();
    topic
        .reply(
            AskId(1),
            serde_json::from_value(json!({"parts":[{"type":"text","body":"Because."}]})).unwrap(),
        )
        .unwrap();
    let stored: Value = serde_json::from_str(&topic.to_json()).unwrap();
    for (pointer, location) in [
        (
            "/rounds/0/questions/0/background/0/body",
            "round 1 question q1 background.part[0].body",
        ),
        (
            "/rounds/0/questions/0/options/0/description",
            "round 1 question q1 option 0 description.part[0].body",
        ),
        (
            "/rounds/0/asks/0/state/parts/0/body",
            "round 1 ask 1 reply.part[0].body",
        ),
    ] {
        let mut corrupt = stored.clone();
        let too_long = json!("x".repeat(16385));
        *corrupt.pointer_mut(pointer).unwrap() = if pointer.ends_with("description") {
            json!([{"type":"text","body":too_long}])
        } else {
            too_long
        };
        let error = Topic::from_json(&corrupt.to_string()).unwrap_err();
        assert!(
            matches!(
                &error,
                LoadError::Explanation(ExplanationError { path, rule: ExplanationRule::Bounds { .. } })
                    if path == location
            ),
            "{location}: {error:?}"
        );
    }
}
