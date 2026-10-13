use super::{accept, flow, input, refused, sequence};
use serde_json::{Value, json};
use shoryo_core::RoundInput;

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
