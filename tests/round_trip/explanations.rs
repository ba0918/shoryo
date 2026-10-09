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
