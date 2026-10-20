mod errors;
mod legacy;
mod limits;
mod schema;

use serde_json::{Value, json};
use shoryo_core::{AskId, AskState, Operation, QuestionId, Reply, RoundInput, Topic};

fn input(parts: Value) -> Value {
    json!({"subject":"Reading", "questions":[{
        "id":"q1", "text":"Which approach?", "class":"human", "why_now":"Choose a direction.",
        "background":parts, "options":[{"text":"A", "description":[], "recommended":true,"consequence":"Continue."}]
    }]})
}

#[test]
fn ordered_parts_preserve_all_metadata() {
    let parts = json!([
        {"type":"text","body":"Before\n<script>"},
        {"type":"code","body":"  run(); \n","language":"unknown","role":"example","title":"A sample"},
        {"type":"text","body":"After"}
    ]);
    let mut topic = Topic::new("", "");
    let round = RoundInput::from_json(&input(parts.clone()).to_string()).unwrap();
    topic.apply_round(round).unwrap();
    let stored: Value = serde_json::from_str(&topic.to_json()).unwrap();
    assert_eq!(stored["rounds"][0]["questions"][0]["background"], parts);
    assert_eq!(Topic::from_json(&topic.to_json()).unwrap(), topic);
}

fn sequence() -> Value {
    json!({"type":"sequence","title":"Exchange","role":"proposal",
        "participants":[{"id":"A","label":"Reader"},{"id":"B","label":"Store"}],
        "events":[{"type":"message","from":"A","to":"B","label":"Read","kind":"call"}]})
}

fn flow() -> Value {
    json!({"type":"flow","title":"Path","role":"confirmed",
        "nodes":[{"id":"A","kind":"decision","label":"Ready?"},{"id":"B","kind":"end","label":"Done"}],
        "edges":[{"from":"A","to":"B","label":{"text":"Yes"}}]})
}

fn accept(parts: Value) -> Topic {
    let mut topic = Topic::new("", "");
    topic
        .apply_round(
            RoundInput::from_json(&input(parts).to_string()).expect("valid explanation schema"),
        )
        .expect("valid explanation content");
    topic
}

fn refused(parts: Value) {
    let mut topic = Topic::new("Original", "Keep this");
    let before = topic.clone();
    if let Ok(round) = RoundInput::from_json(&input(parts).to_string()) {
        assert!(
            topic.apply_round(round).is_err(),
            "invalid explanation accepted"
        );
    }
    assert_eq!(topic, before);
}

#[test]
fn diagrams_preserve_supplied_structure_without_materializing_defaults() {
    let mut seq = sequence();
    seq["layout"] = json!({"participant_gap":230});
    seq["participants"][1]["x"] = json!(400);
    seq["events"] = json!([
        {"type":"alt","branches":[
            {"condition":"Ready","messages":[{"type":"message","from":"A","to":"B","label":"Go","kind":"call","gap_after":16}]},
            {"condition":"Not ready","messages":[{"type":"message","from":"B","to":"B","label":"Wait","kind":"return"}]}]},
        {"type":"loop","condition":"Until done","messages":[{"type":"message","from":"B","to":"A","label":"Retry","kind":"call"}]}]);
    let mut path = flow();
    path["nodes"][0]["position"] = json!({"x":100,"y":200});
    path["edges"][0]["via"] = json!([{"x":300,"y":200},{"x":300,"y":400}]);
    let parts = json!([seq,path,{"type":"diagram","title":"Legacy","role":"example","source":"a = ? Label\nb =\n| a | b |\na -> b : "}]);
    let topic = accept(parts.clone());
    let stored: Value = serde_json::from_str(&topic.to_json()).unwrap();
    assert_eq!(stored["rounds"][0]["questions"][0]["background"], parts);
}

#[test]
fn invalid_part_refuses_entire_round() {
    let valid = json!({"type":"code","body":"","language":"rust","role":"proposal"});
    for invalid in [
        json!({"type":"html","body":"<p>Unsafe</p>"}),
        json!({"type":"code","body":"","language":"rust","role":"unknown"}),
        json!({"type":"text"}),
        json!({"type":"text","body":"valid","html":"<script>"}),
        json!({"type":"code","body":"","language":" ","role":"example"}),
        json!({"type":"code","body":"","language":"rust","role":"example","title":null}),
        json!({"type":"diagram","source":"a = One\na = Two","title":"Duplicate","role":"example"}),
        json!({"type":"diagram","source":"a = One\na -> absent","title":"Dangling","role":"example"}),
        json!({"type":"diagram","source":"not recognized","title":"Invalid","role":"example"}),
    ] {
        refused(json!([valid, invalid]));
    }
    for field in ["participants", "events", "canvas", "layout"] {
        let mut part = sequence();
        part[field] = Value::Null;
        refused(json!([part]));
    }
    let mut part = sequence();
    part["participants"][0]["svg"] = json!("<svg>");
    refused(json!([part]));
    let mut part = sequence();
    part["events"][0]["to"] = json!("Absent");
    refused(json!([part]));
    let mut part = sequence();
    part["participants"][1]["id"] = json!("A");
    refused(json!([part]));
    let mut part = sequence();
    part["participants"][1]["x"] = json!(124);
    refused(json!([part]));
}

#[test]
fn sequence_frames_reject_nesting() {
    let mut part = sequence();
    part["events"] = json!([{"type":"loop","condition":"Again","messages":[{"type":"loop","condition":"Nested","messages":[]}]}]);
    refused(json!([part]));
}

#[test]
fn decision_branches_require_conditions() {
    let mut part = flow();
    part["edges"][0].as_object_mut().unwrap().remove("label");
    refused(json!([part]));
    let mut part = flow();
    part["edges"][0]["label"]["text"] = json!(" \n");
    refused(json!([part]));
}

#[test]
fn invalid_reply_leaves_ask_waiting() {
    let mut topic = accept(json!([]));
    topic
        .apply(
            Operation::Ask {
                question: QuestionId::new("q1"),
                text: "Why?".into(),
                follows: None,
            },
            super::common::any_time(),
        )
        .unwrap();
    let before = topic.clone();
    for parts in [
        json!([]),
        json!([{"type":"text","body":"a".repeat(16385)}]),
        json!([flow(),{"type":"code","body":"","language":" ","role":"example"}]),
    ] {
        let reply: Reply = serde_json::from_value(json!({"parts":parts})).unwrap();
        assert!(topic.reply(AskId(1), reply).is_err());
        assert_eq!(topic, before);
        assert_eq!(topic.rounds[0].asks[0].state, AskState::Waiting);
    }
    let parts = json!([sequence(),{"type":"text","body":"After"}]);
    let reply: Reply = serde_json::from_value(json!({"parts":parts})).unwrap();
    topic.reply(AskId(1), reply).unwrap();
    let stored: Value = serde_json::from_str(&topic.to_json()).unwrap();
    assert_eq!(
        stored["rounds"][0]["asks"][0]["state"],
        json!({"status":"replied","parts":parts})
    );
}

#[test]
fn corrupt_stored_explanations_are_refused() {
    let topic = accept(json!([{"type":"text","body":"Fine"}]));
    let mut stored: Value = serde_json::from_str(&topic.to_json()).unwrap();
    stored["rounds"][0]["questions"][0]["background"][0]["body"] = json!("x".repeat(16385));
    let bytes = stored.to_string();
    assert!(Topic::from_json(&bytes).is_err());
}

#[test]
fn empty_events_frames_and_invalid_references_are_refused() {
    let mut p = sequence();
    p["events"] = json!([]);
    refused(json!([p]));
    for (kind, condition) in [("loop", "Again"), ("loop", " ")] {
        let mut p = sequence();
        p["events"] = json!([{"type":kind,"condition":condition,"messages":[]}]);
        refused(json!([p]));
    }
    for value in ["1bad", "with spaces", "é", "", " "] {
        let mut p = flow();
        p["nodes"][0]["id"] = json!(value);
        refused(json!([p]));
    }
    for field in ["from", "to"] {
        let mut p = flow();
        p["edges"][0][field] = json!("Absent");
        refused(json!([p]));
    }
    let mut p = flow();
    p["nodes"][1]["id"] = json!("A");
    refused(json!([p]));
    let mut p = flow();
    p["edges"][0]["from_port"] = json!("corner");
    refused(json!([p]));
    let mut p = flow();
    p["edges"][0]["via"] = json!([{"x":8193,"y":0}]);
    refused(json!([p]));
}
