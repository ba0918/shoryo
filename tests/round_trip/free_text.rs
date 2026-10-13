use super::harness::{Env, Server, files_under, first_round};
use serde_json::{Value, json};

#[test]
fn blank_custom_and_contradictory_guards_leave_real_storage_unchanged() {
    let env = Env::new();
    let server = env.start("answers", &[]);
    env.run(&["round", "answers"], &first_round());
    server.operate(json!({"op":"answer_method","question":"q1","method":"custom"}));
    server.operate(json!({"op":"free_text","question":"q1","text":"\u{85}　"}));
    let before = server.view();
    for op in [
        json!({"op":"stamp","question":"q1","stamped":true}),
        json!({"op":"submit","round":1}),
    ] {
        assert_eq!(server.operate(op), 409);
        assert_eq!(server.view()["topic"], before["topic"]);
        assert_eq!(server.view()["version"], before["version"]);
    }
    server.operate(json!({"op":"free_text","question":"q1","text":"answer\nconditions"}));
    server.stamp_all();
    let before = server.view();
    let state = files_under(&env.data)
        .into_iter()
        .find(|p| p.ends_with("answers/state.json"))
        .unwrap();
    let bytes = std::fs::read(&state).unwrap();
    let mut invalid = before["expected_answers"].clone();
    invalid[0]["deferred"] = json!(true);
    assert_eq!(
        server.operate(json!({"op":"submit","round":1,"expected_answers":invalid})),
        409
    );
    assert_eq!(std::fs::read(&state).unwrap(), bytes);
    assert_eq!(server.view()["version"], before["version"]);
    assert_eq!(
        server.operate(
            json!({"op":"submit","round":1,"expected_answers":before["expected_answers"]})
        ),
        200
    );
    let sent = env.run(&["wait", "answers", "--timeout", "0"], "");
    let sent: Value = serde_json::from_slice(&sent.stdout).unwrap();
    assert_eq!(
        sent["events"][0]["answers"][0]["free_text"],
        "answer\nconditions"
    );
}

#[test]
fn concurrent_edit_and_guarded_submit_accept_only_the_locked_answer() {
    let env = Env::new();
    let server = env.start("answers", &[]);
    env.run(&["round", "answers"], &first_round());
    server.stamp_all();
    let expected = server.view()["expected_answers"].clone();
    let barrier = std::sync::Barrier::new(2);
    let url = &server.url;
    let statuses = std::thread::scope(|scope| {
        let edit = scope.spawn(|| {
            barrier.wait();
            Server::agent()
                .post(format!("{url}api/op"))
                .send(json!({"op":"note","question":"q1","text":"changed"}).to_string())
                .unwrap()
                .status()
                .as_u16()
        });
        barrier.wait();
        let send = server.operate(json!({"op":"submit","round":1,"expected_answers":expected}));
        (edit.join().unwrap(), send)
    });
    assert!(matches!(statuses, (200, 409) | (409, 200)), "{statuses:?}");
    let view = server.view();
    assert_eq!(view["topic"]["rounds"][0]["submitted"], statuses.1 == 200);
    if statuses.1 == 200 {
        assert_eq!(view["sent_answers"]["1"][0]["note"], "");
    }
}

#[test]
fn guarded_send_refuses_changed_answers_without_saving_or_publishing() {
    let env = Env::new();
    let server = env.start("answers", &[]);
    assert!(
        env.run(&["round", "answers"], &first_round())
            .status
            .success()
    );
    server.stamp_all();
    let snapshot = server.view()["expected_answers"].clone();
    assert!(snapshot.is_array());
    assert_eq!(
        server.operate(json!({"op":"note","question":"q1","text":"new note"})),
        200
    );
    let before = server.view();
    let state = files_under(&env.data)
        .into_iter()
        .find(|p| p.ends_with("answers/state.json"))
        .unwrap();
    let bytes = std::fs::read(&state).unwrap();
    let mut response = Server::agent()
        .post(format!("{}api/op", server.url))
        .send(json!({"op":"submit","round":1,"expected_answers":snapshot}).to_string())
        .unwrap();
    assert_eq!(response.status().as_u16(), 409);
    let refusal: Value = response.body_mut().read_json().unwrap();
    assert_eq!(refusal["refusal"]["kind"], "answers_changed");
    assert_eq!(std::fs::read(&state).unwrap(), bytes);
    assert_eq!(server.view()["version"], before["version"]);
    let latest = server.view()["expected_answers"].clone();
    assert_eq!(
        server.operate(json!({"op":"ask","question":"q1","text":"unrelated"})),
        200
    );
    assert_eq!(
        server.operate(json!({"op":"submit","round":1,"expected_answers":latest})),
        200
    );
}

#[test]
fn custom_drafts_and_old_pending_answers_survive_real_restart() {
    let env = Env::new();
    let server = env.start("answers", &[]);
    env.run(&["round", "answers"], &first_round());
    server.operate(json!({"op":"free_text","question":"q1","text":"  first\nsecond  "}));
    server.operate(json!({"op":"note","question":"q1","text":"hidden"}));
    server.operate(json!({"op":"answer_method","question":"q1","method":"custom"}));
    env.run(&["stop", "answers"], "");
    server.finish();
    let again = env.start("answers", &[]);
    let answer = again.view()["topic"]["rounds"][0]["questions"][0]["answer"].clone();
    assert_eq!(answer["free_text"], "  first\nsecond  ");
    assert_eq!(answer["note"], "hidden");
    again.operate(json!({"op":"choose","question":"q1","option":0}));
    assert_eq!(again.submit(), 200);
    env.run(&["stop", "answers"], "");
    again.finish();
    let state = files_under(&env.data)
        .into_iter()
        .find(|p| p.ends_with("answers/state.json"))
        .unwrap();
    let mut old: Value = serde_json::from_slice(&std::fs::read(&state).unwrap()).unwrap();
    let answer = old["rounds"][0]["questions"][0]["answer"]
        .as_object_mut()
        .unwrap();
    answer.remove("method");
    answer.remove("free_text");
    std::fs::write(&state, old.to_string()).unwrap();
    let restarted = env.start("answers", &[]);
    let received = env.run(&["wait", "answers", "--timeout", "0"], "");
    let received: Value = serde_json::from_slice(&received.stdout).unwrap();
    assert_eq!(received["events"][0]["answers"][0]["choice"], 0);
    let id = received["events"][0]["id"].to_string();
    let ack = env.run(&["wait", "answers", "--ack", &id, "--timeout", "0"], "");
    assert_eq!(
        serde_json::from_slice::<Value>(&ack.stdout).unwrap()["events"],
        json!([])
    );
    assert_eq!(restarted.view()["sent_answers"]["1"][0]["note"], "hidden");
    env.run(&["stop", "answers"], "");
    restarted.finish();
    old["events"][0]["answers"][0]["choice"] = Value::Null;
    std::fs::write(&state, old.to_string()).unwrap();
    use std::io::{BufRead, BufReader};
    let mut corrupt = env
        .command(&["start", "answers"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(corrupt.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let _ = corrupt.kill();
    corrupt.wait().unwrap();
    assert!(
        !line.contains("http://"),
        "malformed answers must not start a server"
    );
}
