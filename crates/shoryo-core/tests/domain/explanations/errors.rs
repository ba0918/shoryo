use super::{flow, input, sequence};
use serde_json::json;
use shoryo_core::{ExplanationError, ExplanationRule, RoundInput, RoundRefusal, Topic};

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
