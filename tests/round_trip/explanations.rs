use serde_json::{Value, json};

use super::harness::{Env, Server, files_under, first_round, text};

fn parts() -> Value {
    json!([
        {"type":"text","body":"Before"},
        {"type":"code","body":"  code(); \n","language":"unknown","role":"example","title":"Sample"},
        {"type":"sequence","title":"Exchange","role":"proposal","participants":[{"id":"A","label":"Reader"}],"events":[{"type":"message","from":"A","to":"A","label":"Read","kind":"return","gap_after":16}]},
        {"type":"flow","title":"Process","role":"confirmed","nodes":[{"id":"N","kind":"process","label":"Work","position":{"x":200,"y":120}}],"edges":[]},
        {"type":"diagram","title":"Old grid","role":"example","source":"a = Label"},
        {"type":"text","body":"After"}
    ])
}

fn round() -> String {
    let mut value: Value = serde_json::from_str(&first_round()).expect("valid fixture");
    value["questions"][0]["background"] = parts();
    value["questions"][0]["options"][0]["description"] = parts();
    value.to_string()
}

fn state(env: &Env) -> std::path::PathBuf {
    files_under(&env.data)
        .into_iter()
        .find(|p| p.ends_with("store/state.json"))
        .expect("synthetic state exists")
}

fn ask(server: &Server, follows: Option<u64>) -> u64 {
    assert_eq!(
        server.operate(json!({"op":"ask","question":"q1","text":"Why?","follows":follows})),
        200
    );
    server.view()["topic"]["rounds"][0]["asks"]
        .as_array()
        .expect("asks")
        .last()
        .expect("new ask")["id"]
        .as_u64()
        .expect("ask ID")
}

fn post(server: &Server, route: &str, body: Value) -> u16 {
    Server::agent()
        .post(format!("{}api/{route}", server.url))
        .send(body.to_string())
        .expect("local request")
        .status()
        .as_u16()
}

#[test]
fn parts_survive_restart_and_result() {
    let env = Env::new();
    let server = env.start("store", &[]);
    let output = env.run(&["round", "store"], &round());
    assert!(output.status.success(), "{}", text(&output));
    let id = ask(&server, None);
    let output = env.run(
        &["reply", "store", &id.to_string()],
        &json!({"parts":parts()}).to_string(),
    );
    assert!(output.status.success(), "{}", text(&output));
    let before = server.view()["topic"].clone();
    assert_eq!(before["rounds"][0]["asks"][0]["state"]["parts"], parts());
    assert!(env.run(&["stop", "store"], "").status.success());
    server.finish();
    let offline = env.run(&["result", "store"], "");
    assert!(offline.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&offline.stdout).unwrap(),
        before
    );
    let again = env.start("store", &[]);
    assert_eq!(again.view()["topic"], before);
}

#[test]
fn invalid_round_or_reply_keeps_saved_bytes() {
    let env = Env::new();
    let server = env.start("store", &[]);
    assert!(env.run(&["round", "store"], &round()).status.success());
    let id = ask(&server, None);
    assert_eq!(server.submit(), 200);
    let path = state(&env);
    let bytes = std::fs::read(&path).unwrap();
    let before = server.view()["topic"].clone();
    for (part, status) in [
        (json!({"type":"text","body":"x".repeat(16385)}), 409),
        (
            json!({"type":"code","body":"x","language":"rust","role":"invalid"}),
            422,
        ),
        (
            json!({"type":"flow","title":"Bad","role":"example","nodes":[],"edges":[]}),
            409,
        ),
    ] {
        let reply = json!({"ask":id,"parts":[part]});
        assert_eq!(post(&server, "reply", reply), status);
        let mut next: Value = serde_json::from_str(&first_round()).unwrap();
        next["questions"][0]["id"] = json!("q2");
        next["questions"][0]["background"] = json!([part]);
        assert_eq!(post(&server, "round", next), status);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(server.view()["topic"], before);
    }
}

#[test]
fn refused_part_save_keeps_previous_content() {
    use std::os::unix::fs::PermissionsExt;
    let env = Env::new();
    let server = env.start("store", &[]);
    assert!(env.run(&["round", "store"], &round()).status.success());
    let id = ask(&server, None);
    let path = state(&env);
    let dir = path.parent().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let before = server.view()["topic"].clone();
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o500)).unwrap();
    let status = post(&server, "reply", json!({"ask":id,"parts":parts()}));
    let after = server.view()["topic"].clone();
    let saved = std::fs::read(&path).unwrap();
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(status, 500);
    assert_eq!(after, before);
    assert_eq!(saved, bytes);
    assert_eq!(
        post(&server, "reply", json!({"ask":id,"parts":parts()})),
        200
    );
}

#[test]
fn late_parts_reply_stays_in_original_round() {
    let env = Env::new();
    let server = env.start("store", &[]);
    assert!(env.run(&["round", "store"], &round()).status.success());
    let id = ask(&server, None);
    assert_eq!(server.submit(), 200);
    assert!(
        env.run(&["round", "store"], r#"{"subject":"Next","questions":[]}"#)
            .status
            .success()
    );
    let output = env.run(
        &["reply", "store", &id.to_string()],
        &json!({"parts":parts()}).to_string(),
    );
    assert!(output.status.success(), "{}", text(&output));
    let view = server.view();
    assert_eq!(
        view["topic"]["rounds"][0]["asks"][0]["state"]["parts"],
        parts()
    );
    assert_eq!(view["topic"]["rounds"][1]["asks"], json!([]));
}

#[test]
fn same_title_follow_up_targets_original_ask() {
    let env = Env::new();
    let server = env.start("store", &[]);
    assert!(env.run(&["round", "store"], &round()).status.success());
    let one = ask(&server, None);
    let two = ask(&server, None);
    for id in [one, two] {
        let output=env.run(&["reply","store",&id.to_string()],&json!({"parts":[{"type":"code","body":id.to_string(),"title":"Same","language":"rust","role":"example"}]}).to_string());
        assert!(output.status.success());
    }
    ask(&server, Some(one));
    let view = server.view();
    assert_eq!(view["topic"]["rounds"][0]["asks"][2]["follows"], one);
}

#[test]
fn old_explanation_state_refuses_start_without_overwrite() {
    for old_reply in [false, true] {
        let env = Env::new();
        let server = env.start("store", &[]);
        assert!(env.run(&["round", "store"], &round()).status.success());
        if old_reply {
            ask(&server, None);
        }
        assert!(env.run(&["stop", "store"], "").status.success());
        server.finish();
        let path = state(&env);
        let mut value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        if old_reply {
            value["rounds"][0]["asks"][0]["state"] =
                json!({"status":"replied","text":"Old","diagram":null});
        } else {
            value["rounds"][0]["questions"][0]["background"] = json!("Old background");
        }
        let bytes = value.to_string();
        std::fs::write(&path, &bytes).unwrap();
        let output = env.run(&["start", "store"], "");
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), bytes);
        let output = env.run(&["result", "store"], "");
        assert!(output.status.success(), "{}", text(&output));
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            value
        );
    }
}

#[test]
fn old_reply_input_is_refused() {
    let env = Env::new();
    let server = env.start("store", &[]);
    assert!(env.run(&["round", "store"], &round()).status.success());
    let id = ask(&server, None);
    let path = state(&env);
    let before = std::fs::read(&path).unwrap();
    let output = env.run(
        &["reply", "store", &id.to_string()],
        r#"{"text":"Old","diagram":"a = Label"}"#,
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn every_explanation_limit_refuses_real_cli_and_http_requests_atomically() {
    let env = Env::new();
    let server = env.start("store", &[]);
    assert!(env.run(&["round", "store"], &round()).status.success());
    let id = ask(&server, None);
    assert_eq!(server.submit(), 200);
    let file = state(&env);
    let bytes = std::fs::read(&file).unwrap();
    let content = server.view()["topic"].clone();
    let message =
        json!({"type":"message","from":"A","to":"B","kind":"call","label":"Send","gap_after":16});
    let seq = json!({"type":"sequence","title":"Exchange","role":"example","participants":[{"id":"A","label":"Reader","x":124},{"id":"B","label":"Store","x":344}],"events":[message,{"type":"alt","gap_after":16,"branches":[{"condition":"Yes","messages":[message]},{"condition":"No","messages":[message]}]},{"type":"loop","condition":"Again","gap_after":16,"messages":[message]}],"layout":{"participant_gap":220,"event_gap":72,"self_loop_width":48},"canvas":{"width":64,"height":64}});
    let path = json!({"type":"flow","title":"Path","role":"proposal","nodes":[{"id":"A","label":"Choose","kind":"decision","position":{"x":100,"y":100},"width":180,"height":96},{"id":"B","label":"Done","kind":"end"}],"edges":[{"from":"A","to":"B","label":{"text":"Yes","position":{"x":300,"y":200}},"via":[{"x":200,"y":100}]}],"canvas":{"width":64,"height":64}});
    let legacy = |source: String| json!({"type":"diagram","title":"Legacy","role":"example","source":source});
    let mut cases = vec![
        (json!(vec![json!({"type":"text","body":""}); 33]), 409),
        (json!([{"type":"text","body":"x".repeat(16385)}]), 409),
        (
            json!([{"type":"code","body":"x".repeat(32769),"language":"x","role":"example"}]),
            409,
        ),
        (json!([legacy(format!("a =\n{}", " ".repeat(32765)))]), 409),
        (
            json!([legacy((0..65).map(|i| format!("n{i} =\n")).collect())]),
            409,
        ),
        (
            json!([legacy(format!("a =\n{}", "a -> a\n".repeat(129)))]),
            409,
        ),
        (
            json!([legacy(format!("a =\n{}", "| a |\n".repeat(65)))]),
            409,
        ),
        (json!([legacy(format!("a =\n| {}", "a |".repeat(33)))]), 409),
        (json!([legacy(format!("{} =", "!".repeat(65)))]), 409),
        (json!([legacy(format!("a = {}", "x".repeat(513)))]), 409),
        (
            json!([legacy(format!("a =\na -> a : {}", "x".repeat(513)))]),
            409,
        ),
        (
            json!([{"type":"code","body":"","language":"x".repeat(65),"role":"example"}]),
            409,
        ),
    ];
    for (base, ptr, max) in [
        (&seq, "/title", 120),
        (&seq, "/participants/0/label", 512),
        (&seq, "/events/0/label", 512),
        (&seq, "/events/1/branches/0/condition", 512),
        (&seq, "/events/2/condition", 512),
        (&seq, "/events/1/branches/0/messages/0/label", 512),
        (&seq, "/events/2/messages/0/label", 512),
        (&path, "/nodes/0/label", 512),
        (&path, "/edges/0/label/text", 512),
    ] {
        for value in ["x".repeat(max + 1), String::new()] {
            let mut part = base.clone();
            *part.pointer_mut(ptr).unwrap() = json!(value);
            cases.push((json!([part]), 409));
        }
    }
    for (base, ptr, min, max) in [
        (&seq, "/layout/participant_gap", 16, 512),
        (&seq, "/layout/event_gap", 16, 512),
        (&seq, "/layout/self_loop_width", 16, 512),
        (&seq, "/events/0/gap_after", 16, 512),
        (&seq, "/events/1/gap_after", 16, 512),
        (&seq, "/events/2/gap_after", 16, 512),
        (&seq, "/events/1/branches/0/messages/0/gap_after", 16, 512),
        (&seq, "/events/2/messages/0/gap_after", 16, 512),
        (&seq, "/participants/1/x", 0, 8192),
        (&seq, "/canvas/width", 64, 8192),
        (&seq, "/canvas/height", 64, 8192),
        (&path, "/nodes/0/position/x", 0, 8192),
        (&path, "/nodes/0/position/y", 0, 8192),
        (&path, "/nodes/0/width", 80, 640),
        (&path, "/nodes/0/height", 32, 640),
        (&path, "/edges/0/via/0/x", 0, 8192),
        (&path, "/edges/0/via/0/y", 0, 8192),
        (&path, "/edges/0/label/position/x", 0, 8192),
        (&path, "/edges/0/label/position/y", 0, 8192),
        (&path, "/canvas/width", 64, 8192),
        (&path, "/canvas/height", 64, 8192),
    ] {
        for (value, status) in [(json!(max + 1), 409), (json!(-1), 422), (json!(0.5), 422)] {
            let mut part = base.clone();
            *part.pointer_mut(ptr).unwrap() = value;
            cases.push((json!([part]), status));
        }
        if min > 0 {
            let mut part = base.clone();
            *part.pointer_mut(ptr).unwrap() = json!(min - 1);
            cases.push((json!([part]), 409));
        }
    }
    for (base, ptr, value) in [
        (
            &seq,
            "/participants",
            json!(
                (0..13)
                    .map(|i| json!({"id":format!("N{i}"),"label":"N"}))
                    .collect::<Vec<_>>()
            ),
        ),
        (&seq, "/events", json!([])),
        (
            &seq,
            "/events",
            json!([{"type":"loop","condition":"Again","messages":vec![message.clone();97]}]),
        ),
        (
            &seq,
            "/events",
            json!(vec![
                json!({"type":"loop","condition":"Again","messages":[message]});
                13
            ]),
        ),
        (
            &seq,
            "/events/1/branches",
            json!(vec![json!({"condition":"Yes","messages":[message]}); 9]),
        ),
        (
            &seq,
            "/events/1/branches",
            json!([{ "condition":"Only","messages":[message]}]),
        ),
        (&seq, "/events/2/messages", json!([])),
        (
            &path,
            "/nodes",
            json!(
                (0..33)
                    .map(|i| json!({"id":format!("N{i}"),"label":"N","kind":"process"}))
                    .collect::<Vec<_>>()
            ),
        ),
        (&path, "/nodes", json!([])),
        (&path, "/edges", json!(vec![path["edges"][0].clone(); 65])),
        (&path, "/edges/0/via", json!(vec![json!({"x":0,"y":0}); 13])),
    ] {
        let mut part = base.clone();
        *part.pointer_mut(ptr).unwrap() = value;
        cases.push((json!([part]), 409));
    }
    let mut budget = vec![json!({"type":"text","body":"a".repeat(16384)}); 32];
    budget[31]["body"] = json!("a".repeat(16257));
    cases.push((json!(budget), 409));
    for (index, (parts, status)) in cases.into_iter().enumerate() {
        let reply = json!({"ask":id,"parts":parts});
        assert_eq!(post(&server, "reply", reply), status, "reply case {index}");
        let out = env.run(
            &["reply", "store", &id.to_string()],
            &json!({"parts":parts}).to_string(),
        );
        assert_eq!(out.status.code(), Some(1), "CLI reply case {index}");
        let mut next: Value = serde_json::from_str(&first_round()).unwrap();
        next["questions"][0]["id"] = json!("next");
        next["questions"][0]["background"] = parts;
        assert_eq!(
            post(&server, "round", next.clone()),
            status,
            "round case {index}"
        );
        assert_eq!(
            env.run(&["round", "store"], &next.to_string())
                .status
                .code(),
            Some(1),
            "CLI round case {index}"
        );
        assert_eq!(
            std::fs::read(&file).unwrap(),
            bytes,
            "saved bytes case {index}"
        );
        assert_eq!(server.view()["topic"], content, "topic case {index}");
    }
}
