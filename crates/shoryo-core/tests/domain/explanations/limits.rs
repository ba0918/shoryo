//! The finite limits of explanation parts: each accepted at its bound and refused one past it,
//! for rounds and replies alike, and the byte budget they share.

use super::{accept, flow, input, refused, sequence};
use serde_json::{Value, json};
use shoryo_core::{AskId, Operation, QuestionId, Reply, RoundInput, Topic};

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
}

#[test]
fn layout_numbers_accept_their_limits_and_refuse_out_of_range_values() {
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
}

#[test]
fn sequence_participant_counts_accept_their_limits_and_refuse_excess() {
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
}

#[test]
fn flow_node_counts_accept_their_limits_and_refuse_excess() {
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
}

#[test]
fn flow_edge_counts_accept_their_limits_and_refuse_excess() {
    for (count, valid) in [(0, true), (64, true), (65, false)] {
        let mut p = flow();
        p["edges"] = json!(vec![p["edges"][0].clone(); count]);
        if valid {
            accept(json!([p]));
        } else {
            refused(json!([p]));
        }
    }
}

#[test]
fn flow_routes_accept_their_limits_and_refuse_excess() {
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
}

#[test]
fn sequence_branch_counts_accept_their_limits_and_refuse_excess() {
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
                crate::common::any_time(),
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
fn round_explanations_accept_the_byte_budget_and_refuse_one_extra_byte() {
    let mut parts = vec![json!({"type":"text","body":"a".repeat(16384)}); 32];
    parts[31]["body"] = json!("a".repeat(16256));
    accept(json!(parts));
    parts[31]["body"] = json!("a".repeat(16257));
    refused(json!(parts));
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
                crate::common::any_time(),
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
                crate::common::any_time(),
            )
            .unwrap();
        topic
            .apply(
                Operation::Submit {
                    round: round + 1,
                    expected_answers: None,
                },
                crate::common::any_time(),
            )
            .unwrap();
    }
    assert_eq!(Topic::from_json(&topic.to_json()).unwrap(), topic);
    let mut stored: Value = serde_json::from_str(&topic.to_json()).unwrap();
    stored["rounds"][0]["asks"][0]["state"]["parts"][0]["body"] = json!("x".repeat(16385));
    assert!(Topic::from_json(&stored.to_string()).is_err());
}
