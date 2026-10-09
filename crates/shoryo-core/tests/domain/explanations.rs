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
fn explanation_limits_accept_boundary_and_refuse_excess() {
    for (kind, max) in [("text", 16_384), ("code", 32_768)] {
        let mut part = if kind == "text" {
            json!({"type":"text","body":""})
        } else {
            json!({"type":"code","body":"","language":"x","role":"example"})
        };
        for count in [0, max] {
            part["body"] = json!("🦀".repeat(count));
            accept(json!([part]));
        }
        part["body"] = json!("a".repeat(max + 1));
        refused(json!([part]));
    }
    accept(json!([]));
    accept(json!(vec![json!({"type":"text","body":""}); 32]));
    refused(json!(vec![json!({"type":"text","body":""}); 33]));
    let mut seq = sequence();
    let message = seq["events"][0].clone();
    seq["events"] = json!(vec![message.clone(); 96]);
    accept(json!([seq]));
    seq["events"] =
        json!([{"type":"loop","condition":"Again","messages":vec![message.clone();97]}]);
    refused(json!([seq]));
    seq["events"] = json!(vec![
        json!({"type":"loop","condition":"Again","messages":[message]});
        12
    ]);
    accept(json!([seq]));
    seq["events"] = json!(vec![
        json!({"type":"loop","condition":"Again","messages":[message]});
        13
    ]);
    refused(json!([seq]));
    for (field, min, max) in [("x", 0, 8192)] {
        for value in [min, max] {
            let mut p = sequence();
            p["participants"][1][field] = json!(value);
            if value == 0 {
                p["participants"][0]["x"] = json!(0);
                p["participants"][1].as_object_mut().unwrap().remove("x");
            }
            accept(json!([p]));
        }
        for value in [json!(-1), json!(max + 1), json!(0.5)] {
            let mut p = sequence();
            p["participants"][1][field] = value;
            refused(json!([p]));
        }
    }
}

#[test]
fn diagram_and_string_bounds_are_checked_without_truncation() {
    for (part, path, max) in [
        (sequence(), "/title", 120),
        (sequence(), "/participants/0/id", 64),
        (sequence(), "/participants/0/label", 512),
        (sequence(), "/events/0/label", 512),
        (flow(), "/nodes/0/label", 512),
        (flow(), "/edges/0/label/text", 512),
        (
            json!({"type":"code","body":"","language":"x","role":"example"}),
            "/language",
            64,
        ),
        (
            json!({"type":"code","body":"","language":"x","role":"example","title":"x"}),
            "/title",
            120,
        ),
    ] {
        for value in ["x".to_string(), "x".repeat(max)] {
            let mut p = part.clone();
            *p.pointer_mut(path).unwrap() = json!(value);
            if path.ends_with("/id") {
                p["events"][0]["from"] = json!(value);
            }
            accept(json!([p]));
        }
        for value in ["".to_string(), " \n".to_string(), "x".repeat(max + 1)] {
            let mut p = part.clone();
            *p.pointer_mut(path).unwrap() = json!(value);
            refused(json!([p]));
        }
    }
    for (part, path, min, max) in [
        (sequence(), "/layout/participant_gap", 16, 512),
        (sequence(), "/layout/event_gap", 16, 512),
        (sequence(), "/layout/self_loop_width", 16, 512),
        (sequence(), "/events/0/gap_after", 16, 512),
        (flow(), "/nodes/0/width", 80, 640),
        (flow(), "/nodes/0/height", 32, 640),
        (flow(), "/canvas/width", 64, 8192),
        (flow(), "/canvas/height", 64, 8192),
        (flow(), "/nodes/0/position/x", 0, 8192),
        (flow(), "/nodes/0/position/y", 0, 8192),
        (flow(), "/edges/0/label/position/x", 0, 8192),
        (flow(), "/edges/0/label/position/y", 0, 8192),
    ] {
        let mut base = part;
        base["canvas"] = json!({"width":64,"height":64});
        if base["type"] == "sequence" {
            base["layout"] = json!({"participant_gap":220,"event_gap":72,"self_loop_width":48});
        } else {
            base["nodes"][0]["position"] = json!({"x":100,"y":100});
            base["nodes"][0]["width"] = json!(180);
            base["nodes"][0]["height"] = json!(64);
            base["edges"][0]["label"]["position"] = json!({"x":300,"y":300});
        }
        if path.ends_with("gap_after") {
            base["events"][0]["gap_after"] = json!(16);
        }
        for value in [min, max] {
            let mut p = base.clone();
            *p.pointer_mut(path).unwrap() = json!(value);
            accept(json!([p]));
        }
        for value in [json!(max + 1), json!(-1), json!(0.5)] {
            let mut p = base.clone();
            *p.pointer_mut(path).unwrap() = value;
            refused(json!([p]));
        }
        if min > 0 {
            let mut p = base.clone();
            *p.pointer_mut(path).unwrap() = json!(min - 1);
            refused(json!([p]));
        }
    }
    for (count, valid) in [(1, true), (12, true), (0, false), (13, false)] {
        let mut p = sequence();
        p["participants"] = json!(
            (0..count)
                .map(|i| json!({"id":format!("P{i}"),"label":"A"}))
                .collect::<Vec<_>>()
        );
        p["events"][0]["from"] = json!("P0");
        p["events"][0]["to"] = json!("P0");
        if valid {
            accept(json!([p]));
        } else {
            refused(json!([p]));
        }
    }
    for (count, valid) in [(1, true), (32, true), (0, false), (33, false)] {
        let mut p = flow();
        p["nodes"] = json!(
            (0..count)
                .map(|i| json!({"id":format!("N{i}"),"kind":"process","label":"A"}))
                .collect::<Vec<_>>()
        );
        p["edges"] = json!([]);
        if valid {
            accept(json!([p]));
        } else {
            refused(json!([p]));
        }
    }
    for (count, valid) in [(0, true), (64, true), (65, false)] {
        let mut p = flow();
        p["edges"] = json!(vec![p["edges"][0].clone(); count]);
        if valid {
            accept(json!([p]));
        } else {
            refused(json!([p]));
        }
    }
    for (count, valid) in [(0, true), (12, true), (13, false)] {
        let mut p = flow();
        p["edges"][0]["via"] = json!(vec![json!({"x":8192,"y":0}); count]);
        p["edges"] = json!(vec![p["edges"][0].clone(); 64]);
        if valid {
            accept(json!([p]));
        } else {
            refused(json!([p]));
        }
    }
    for (count, valid) in [(2, true), (8, true), (1, false), (9, false)] {
        let mut p = sequence();
        let message = p["events"][0].clone();
        p["events"] = json!([{"type":"alt","branches":vec![json!({"condition":"x".repeat(512),"messages":[message]});count]}]);
        if valid {
            accept(json!([p]));
        } else {
            refused(json!([p]));
        }
    }
    let mut parts = vec![json!({"type":"text","body":"a".repeat(16384)}); 32];
    parts[31]["body"] = json!("a".repeat(16256));
    accept(json!(parts));
    parts[31]["body"] = json!("a".repeat(16257));
    refused(json!(parts));
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
fn corrupt_stored_explanations_are_refused_without_altering_the_source() {
    let topic = accept(json!([{"type":"text","body":"Fine"}]));
    let mut stored: Value = serde_json::from_str(&topic.to_json()).unwrap();
    stored["rounds"][0]["questions"][0]["background"][0]["body"] = json!("x".repeat(16385));
    let bytes = stored.to_string();
    assert!(Topic::from_json(&bytes).is_err());
    assert_eq!(bytes, stored.to_string());
}

#[test]
fn legacy_token_punctuation_and_repeated_grid_cells_keep_existing_meanings() {
    accept(
        json!([{"type":"diagram","title":"Tokens","role":"example","source":"a=b = Label\nc->d = Other\n| a=b | a=b |\na=b -> c->d : Done"}]),
    );
    accept(
        json!([{"type":"diagram","title":"Tokens","role":"example","source":"a=b=c\nz=Other\na=b->z"}]),
    );
}

#[test]
fn legacy_limits_have_reachable_acceptance_and_refusal_pairs() {
    let diagram = |source: String| json!([{"type":"diagram","title":"Grid","role":"example","source":source}]);
    for (n, valid) in [(1, true), (64, true), (0, false), (65, false)] {
        let source = (0..n).map(|i| format!("n{i} =\n")).collect::<String>();
        if valid {
            accept(diagram(source));
        } else {
            refused(diagram(source));
        }
    }
    for (n, valid) in [(0, true), (128, true), (129, false)] {
        let source = format!("a =\n{}", "a -> a :\n".repeat(n));
        if valid {
            accept(diagram(source));
        } else {
            refused(diagram(source));
        }
    }
    for (n, valid) in [(0, true), (64, true), (65, false)] {
        let source = format!("a =\n{}", "| a |\n".repeat(n));
        if valid {
            accept(diagram(source));
        } else {
            refused(diagram(source));
        }
    }
    for (n, valid) in [(1, true), (32, true), (33, false)] {
        let source = format!("a =\n| {}", "a |".repeat(n));
        if valid {
            accept(diagram(source));
        } else {
            refused(diagram(source));
        }
    }
    for (n, valid) in [(1, true), (64, true), (65, false)] {
        let name = "!".repeat(n);
        let source = format!("{name} =\n| {name} |\n{name} -> {name}");
        if valid {
            accept(diagram(source));
        } else {
            refused(diagram(source));
        }
    }
    for (n, valid) in [(0, true), (512, true), (513, false)] {
        let label = "x".repeat(n);
        let source = format!("a = {label}\na -> a : {label}");
        if valid {
            accept(diagram(source));
        } else {
            refused(diagram(source));
        }
    }
    for (n, valid) in [(32768, true), (32769, false)] {
        let source = format!("a =\n{}", " ".repeat(n - 4));
        if valid {
            accept(diagram(source));
        } else {
            refused(diagram(source));
        }
    }
    refused(diagram("a =\n| a | . |\n| a |".into()));
}

#[test]
fn every_nested_input_object_refuses_unknown_fields_and_null_optionals() {
    let mut seq = sequence();
    seq["layout"] = json!({"event_gap":72});
    seq["canvas"] = json!({"width":64,"height":64});
    let message = seq["events"][0].clone();
    seq["events"] = json!([
        message,{"type":"alt","branches":[{"condition":"Yes","messages":[message]},{"condition":"No","messages":[message]}]},
        {"type":"loop","condition":"Again","messages":[message]}]);
    let mut path = flow();
    path["canvas"] = json!({"width":64,"height":64});
    path["nodes"][0]["position"] = json!({"x":100,"y":100});
    path["edges"][0]["via"] = json!([{"x":100,"y":100}]);
    path["edges"][0]["label"]["position"] = json!({"x":100,"y":100});
    for (base, paths) in [
        (
            seq,
            vec![
                "",
                "/layout",
                "/canvas",
                "/participants/0",
                "/events/0",
                "/events/1",
                "/events/1/branches/0",
                "/events/1/branches/0/messages/0",
                "/events/2",
                "/events/2/messages/0",
            ],
        ),
        (
            path,
            vec![
                "",
                "/canvas",
                "/nodes/0",
                "/nodes/0/position",
                "/edges/0",
                "/edges/0/via/0",
                "/edges/0/label",
                "/edges/0/label/position",
            ],
        ),
    ] {
        accept(json!([base]));
        for ptr in paths {
            let mut p = base.clone();
            p.pointer_mut(ptr)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("unexpected".into(), json!("x"));
            assert!(
                RoundInput::from_json(&input(json!([p])).to_string()).is_err(),
                "unknown field at {ptr}"
            );
        }
    }
    for field in ["position", "width", "height"] {
        let mut p = flow();
        p["nodes"][0][field] = Value::Null;
        refused(json!([p]));
    }
    for field in ["from_port", "to_port", "via", "label"] {
        let mut p = flow();
        p["edges"][0][field] = Value::Null;
        refused(json!([p]));
    }
    assert!(RoundInput::from_json(r#"{"subject":"x","questions":[{"id":"q","text":"x","class":"human","why_now":"x","background":[{"type":"text","body":"\ud800"}],"options":[] }]}"#).is_err());
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

#[test]
fn remaining_leaf_bounds_are_shared_by_rounds_and_replies() {
    let message = sequence()["events"][0].clone();
    let mut seq = sequence();
    seq["canvas"] = json!({"width":64,"height":64});
    seq["events"] = json!([
        {"type":"alt","gap_after":16,"branches":[{"condition":"Yes","messages":[message]},{"condition":"No","messages":[message]}]},
        {"type":"loop","condition":"Again","gap_after":16,"messages":[message]}]);
    seq["events"][0]["branches"][0]["messages"][0]["gap_after"] = json!(16);
    seq["events"][1]["messages"][0]["gap_after"] = json!(16);
    let mut path = flow();
    path["edges"][0]["via"] = json!([{"x":0,"y":0}]);
    let mut cases = Vec::new();
    for ptr in ["/canvas/width", "/canvas/height"] {
        cases.push((seq.clone(), ptr, 64, 8192));
    }
    for ptr in [
        "/events/0/gap_after",
        "/events/1/gap_after",
        "/events/0/branches/0/messages/0/gap_after",
        "/events/1/messages/0/gap_after",
    ] {
        cases.push((seq.clone(), ptr, 16, 512));
    }
    for ptr in ["/edges/0/via/0/x", "/edges/0/via/0/y"] {
        cases.push((path.clone(), ptr, 0, 8192));
    }
    let reply_checked = |parts: Value, valid: bool| {
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
        let reply = serde_json::from_value::<Reply>(json!({"parts":parts}));
        let accepted = reply.is_ok_and(|reply| topic.reply(AskId(1), reply).is_ok());
        assert_eq!(accepted, valid);
        if !valid {
            assert_eq!(topic, before);
        }
    };
    for (base, ptr, min, max) in cases {
        for number in [
            json!(min),
            json!(max),
            json!(max + 1),
            json!(-1),
            json!(0.5),
            json!(min - 1),
        ] {
            let valid = number.as_i64().is_some_and(|n| n >= min && n <= max);
            let mut p = base.clone();
            *p.pointer_mut(ptr).unwrap() = number;
            if valid {
                accept(json!([p]));
            } else {
                refused(json!([p]));
            }
            reply_checked(json!([p]), valid);
        }
    }
    for (base, ptr) in [
        (seq.clone(), "/events/0/branches/0/condition"),
        (seq.clone(), "/events/1/condition"),
        (seq.clone(), "/events/0/branches/0/messages/0/label"),
        (seq.clone(), "/events/1/messages/0/label"),
        (path.clone(), "/title"),
    ] {
        let max = if ptr == "/title" { 120 } else { 512 };
        for (label, valid) in [
            ("界".into(), true),
            ("界".repeat(max), true),
            ("界".repeat(max + 1), false),
            (String::new(), false),
            (" \n".into(), false),
        ] {
            let mut p = base.clone();
            *p.pointer_mut(ptr).unwrap() = json!(label);
            if valid {
                accept(json!([p]));
            } else {
                refused(json!([p]));
            }
            reply_checked(json!([p]), valid);
        }
    }
    for count in [1, 96, 0, 97] {
        let mut p = seq.clone();
        p["events"] =
            json!([{"type":"loop","condition":"Again","messages":vec![message.clone();count]}]);
        let valid = (1..=96).contains(&count);
        if valid {
            accept(json!([p]));
        } else {
            refused(json!([p]));
        }
        reply_checked(json!([p]), valid);
    }
    for (ptr, max) in [("/nodes/0/id", 64), ("/nodes/1/id", 64)] {
        for n in [1, max, max + 1] {
            let mut p = path.clone();
            let id = "X".repeat(n);
            *p.pointer_mut(ptr).unwrap() = json!(id);
            if ptr == "/nodes/0/id" {
                p["edges"][0]["from"] = json!(id);
            } else {
                p["edges"][0]["to"] = json!(id);
            }
            if n <= max {
                accept(json!([p]));
            } else {
                refused(json!([p]));
            }
            reply_checked(json!([p]), n <= max);
        }
    }
}

#[test]
fn reply_and_historical_submission_budgets_are_not_cumulative() {
    let mut topic = Topic::new("", "");
    let mut parts = vec![json!({"type":"text","body":"a".repeat(16384)}); 32];
    parts[31]["body"] = json!("a".repeat(16256));
    for round in 0_u32..2 {
        let mut value = input(json!(parts));
        let id = format!("q{round}");
        value["questions"][0]["id"] = json!(id);
        topic
            .apply_round(RoundInput::from_json(&value.to_string()).unwrap())
            .unwrap();
        topic
            .apply(
                Operation::Ask {
                    question: QuestionId::new(&id),
                    text: "Why?".into(),
                    follows: None,
                },
                super::common::any_time(),
            )
            .unwrap();
        let before = topic.clone();
        let mut too_big = parts.clone();
        too_big[31]["body"] = json!("a".repeat(16257));
        assert!(
            topic
                .reply(
                    AskId(u64::from(round + 1)),
                    serde_json::from_value(json!({"parts":too_big})).unwrap()
                )
                .is_err()
        );
        assert_eq!(topic, before);
        topic
            .reply(
                AskId(u64::from(round + 1)),
                serde_json::from_value(json!({"parts":parts})).unwrap(),
            )
            .unwrap();
        topic
            .apply(
                Operation::Stamp {
                    question: QuestionId::new(&id),
                    stamped: true,
                },
                super::common::any_time(),
            )
            .unwrap();
        topic
            .apply(
                Operation::Submit { round: round + 1 },
                super::common::any_time(),
            )
            .unwrap();
    }
    assert_eq!(Topic::from_json(&topic.to_json()).unwrap(), topic);
    let mut stored: Value = serde_json::from_str(&topic.to_json()).unwrap();
    stored["rounds"][0]["asks"][0]["state"]["parts"][0]["body"] = json!("x".repeat(16385));
    assert!(Topic::from_json(&stored.to_string()).is_err());
}

#[test]
fn required_nested_fields_and_optional_nulls_are_not_silently_defaulted() {
    let message = sequence()["events"][0].clone();
    let mut seq = sequence();
    seq["events"] = json!([{ "type":"alt", "branches":[{"condition":"Yes","messages":[message]},{"condition":"No","messages":[message]}] },{"type":"loop","condition":"Again","messages":[message]}]);
    let code = json!({"type":"code","body":"","language":"rust","role":"example"});
    let legacy = json!({"type":"diagram","title":"Legacy","role":"example","source":"a ="});
    let text = json!({"type":"text","body":""});
    for (base, ptr, fields) in [
        (text, "", vec!["type", "body"]),
        (code, "", vec!["type", "body", "language", "role"]),
        (legacy, "", vec!["type", "title", "role", "source"]),
        (
            sequence(),
            "",
            vec!["type", "title", "role", "participants", "events"],
        ),
        (sequence(), "/participants/0", vec!["id", "label"]),
        (
            sequence(),
            "/events/0",
            vec!["type", "from", "to", "label", "kind"],
        ),
        (seq.clone(), "/events/0", vec!["type", "branches"]),
        (
            seq.clone(),
            "/events/0/branches/0",
            vec!["condition", "messages"],
        ),
        (
            seq.clone(),
            "/events/1",
            vec!["type", "condition", "messages"],
        ),
        (
            seq.clone(),
            "/events/1/messages/0",
            vec!["type", "from", "to", "label", "kind"],
        ),
        (flow(), "", vec!["type", "title", "role", "nodes", "edges"]),
        (flow(), "/nodes/0", vec!["id", "kind", "label"]),
        (flow(), "/edges/0", vec!["from", "to"]),
        (flow(), "/edges/0/label", vec!["text"]),
    ] {
        accept(json!([base]));
        let mut unknown = base.clone();
        unknown
            .pointer_mut(ptr)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown".into(), json!(true));
        refused(json!([unknown]));
        for field in fields {
            let mut p = base.clone();
            p.pointer_mut(ptr)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert!(
                RoundInput::from_json(&input(json!([p])).to_string()).is_err(),
                "missing {ptr}/{field}"
            );
        }
    }
    for ptr in ["/participants/0/x", "/events/0/gap_after"] {
        let mut p = sequence();
        let fields: Vec<_> = ptr.trim_start_matches('/').split('/').collect();
        p[fields[0]][0][fields[2]] = Value::Null;
        refused(json!([p]));
    }
    for path in ["/events/0/branches/0/messages", "/events/1/messages"] {
        let mut p = seq.clone();
        *p.pointer_mut(path).unwrap() = json!([]);
        refused(json!([p]));
    }
}
