use crate::harness::{Env, events, first_round, text, wait_until};
use serde_json::{Value, json};

#[test]
fn an_unrepresentable_wait_timeout_is_refused_without_acknowledging_events() {
    let env = Env::new();
    let server = env.start("timeout", &[]);
    assert!(
        env.run(&["round", "timeout"], &first_round())
            .status
            .success()
    );
    assert_eq!(
        server.operate(json!({"op":"ask", "question":"q1", "text":"Why?"})),
        200
    );
    let before = server.view()["topic"].clone();
    let id = before["events"][0]["id"].clone();
    let mut response = server.post("wait", &json!({"ack":[id], "timeout":u64::MAX}));
    assert_eq!(response.status().as_u16(), 422);
    let refusal: Value = response.body_mut().read_json().unwrap();
    assert!(refusal["error"].as_str().unwrap().contains("timeout"));
    let after = server.view();
    assert_eq!(after["topic"], before);
    assert_eq!(after["agent"]["waiting"], false);
}

#[test]
fn a_zero_wait_timeout_returns_no_events_and_releases_the_waiting_state() {
    let env = Env::new();
    let server = env.start("timeout", &[]);
    let mut response = server.post("wait", &json!({"timeout":0}));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(
        response.body_mut().read_json::<Value>().unwrap(),
        json!({"events":[]})
    );
    assert_eq!(server.view()["agent"]["waiting"], false);
}

#[test]
fn events_while_not_waiting_are_received_later() {
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    server.operate(json!({ "op": "ask", "question": "q1", "text": "Explain more" }));
    server.submit();

    let first = events(&env.run(&["wait", "store", "--timeout", "1"], ""));
    let again = events(&env.run(&["wait", "store", "--timeout", "1"], ""));

    let kinds: Vec<&str> = first.iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["ask", "submitted"]);
    assert_eq!(first, again);
}

#[test]
fn acknowledged_events_are_not_returned_again() {
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    server.submit();
    let first = events(&env.run(&["wait", "store", "--timeout", "1"], ""));
    let id = first[0]["id"].to_string();

    let after = events(&env.run(&["wait", "store", "--ack", &id, "--timeout", "1"], ""));

    assert!(after.is_empty(), "{after:?}");
}

#[test]
fn wait_returns_when_an_event_happens() {
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    let waiting = env
        .command(&["wait", "store", "--timeout", "20"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("wait starts");
    wait_until(|| server.view()["agent"]["waiting"] == true);

    server.submit();

    let output = waiting.wait_with_output().expect("wait finishes");
    assert_eq!(events(&output)[0]["kind"], "submitted");
}

#[test]
fn status_reports_whether_a_wait_is_in_progress() {
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    let waiting_now = || server.view()["agent"]["waiting"].as_bool();
    assert_eq!(waiting_now(), Some(false));

    let waiting = env
        .command(&["wait", "store", "--timeout", "20"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("wait starts");
    wait_until(|| waiting_now() == Some(true));

    server.submit();
    let output = waiting.wait_with_output().expect("wait finishes");
    assert_eq!(events(&output)[0]["kind"], "submitted");
    wait_until(|| waiting_now() == Some(false));
}

#[test]
fn counted_commands_restart_the_quiet_time_and_result_does_not() {
    let env = Env::new();
    let server = env.start("store", &[]);
    let quiet_ms = || {
        server.view()["agent"]["quiet_ms"]
            .as_u64()
            .expect("the view reports elapsed milliseconds")
    };
    let counted = |args: &[&str], input: &str| {
        wait_until(|| quiet_ms() >= 10);
        let began = std::time::Instant::now();
        let output = env.run(args, input);
        assert!(output.status.success(), "{}", text(&output));
        let quiet = quiet_ms();
        assert!(
            u128::from(quiet) <= began.elapsed().as_millis(),
            "the command did not restart elapsed time"
        );
    };
    counted(&["round", "store"], &first_round());
    server.operate(json!({ "op": "ask", "question": "q1", "text": "Why one file?" }));
    counted(
        &["reply", "store", "1"],
        r#"{ "parts": [{"type":"text","body":"It is simple."}] }"#,
    );
    wait_until(|| quiet_ms() >= 10);
    let before_result = quiet_ms();
    env.run(&["result", "store"], "");
    let after_result = quiet_ms();
    server.submit();
    counted(&["end", "store"], "");
    assert!(
        after_result >= before_result,
        "result restarted the quiet time: {before_result} ms, then {after_result} ms"
    );
}
