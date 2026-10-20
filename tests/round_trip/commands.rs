//! The agent's round, reply, end and result commands against a running server
//! (`docs/spec/server.md`, "往復").

use serde_json::{Value, json};

use crate::harness::{Env, events, first_round, text};

#[test]
fn round_output_contains_no_question_text() {
    let env = Env::new();
    let server = env.start("store", &[]);
    let server_url = server.url.clone();

    let output = env.run(&["round", "store"], &first_round());
    env.run(&["stop", "store"], "");
    let (server_out, server_err) = server.finish();

    let printed = format!("{}{server_out}{server_err}", text(&output));
    assert!(output.status.success(), "{printed}");
    assert!(text(&output).contains(&server_url), "{printed}");
    for content in [
        "Which store keeps",
        "A single JSON file",
        "An embedded database",
        "Choosing",
    ] {
        assert!(!printed.contains(content), "printed {content:?}");
    }
}

#[test]
fn refused_round_returns_the_reason() {
    let env = Env::new();
    let _server = env.start("store", &[]);
    let mut round: Value = serde_json::from_str(&first_round()).expect("the round is JSON");
    round["questions"][0]["options"][1]["recommended"] = json!(true);

    let output = env.run(&["round", "store"], &round.to_string());

    assert!(!output.status.success());
    assert!(text(&output).contains("q1"));
}

#[test]
fn end_refuses_while_a_round_is_unsent() {
    let env = Env::new();
    let _server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());

    let output = env.run(&["end", "store"], "");

    assert!(!output.status.success());
}

#[test]
fn reply_that_is_not_an_object_is_refused_with_status_one() {
    let env = Env::new();
    let _server = env.start("store", &[]);

    let output = env.run(&["reply", "store", "1"], "[1]");

    assert_eq!(output.status.code(), Some(1), "{}", text(&output));
}

#[test]
fn result_is_available_after_end() {
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    server.submit();
    assert!(env.run(&["end", "store"], "").status.success());

    let output = env.run(&["result", "store"], "");

    let result: Value = serde_json::from_slice(&output.stdout).expect("the result is JSON");
    assert_eq!(result["ended"], true);
    assert_eq!(result["rounds"][0]["questions"][0]["id"], "q1");
}

#[test]
fn result_reads_the_file_when_no_server_runs() {
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    let while_running = env.run(&["result", "store"], "");
    env.run(&["stop", "store"], "");
    server.finish();

    let stopped = env.run(&["result", "store"], "");

    assert!(stopped.status.success(), "{}", text(&stopped));
    assert_eq!(stopped.stdout, while_running.stdout);
}

#[test]
fn past_result_round_reads_back_the_picture_and_records_it_was_sent_with() {
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    server.submit();
    let result_round = json!({
        "subject": "Result",
        "finished_picture": "store = One JSON file",
        "records": { "undecided": [{ "text": "Backups", "decider": "The person" }] },
    });
    env.run(&["round", "store"], &result_round.to_string());
    server.submit();
    let later = json!({
        "subject": "After the result",
        "finished_picture": "store = Two files",
        "records": { "undecided": [] },
    });
    env.run(&["round", "store"], &later.to_string());

    let output = env.run(&["result", "store"], "");

    let result: Value = serde_json::from_slice(&output.stdout).expect("the result is JSON");
    let sent = &result["rounds"][1];
    assert_eq!(sent["finished_picture"], "store = One JSON file");
    assert_eq!(sent["records"]["undecided"][0]["text"], "Backups");
    assert_eq!(sent["sent_as"], "proceeded");
    assert_eq!(result["finished_picture"], "store = Two files");
}

#[test]
fn review_request_between_sending_and_the_next_round_is_refused_and_leaves_no_trace() {
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    server.submit();
    let result_round = json!({
        "subject": "Result",
        "records": { "decisions": [{
            "id": "d1",
            "name": "One file",
            "text": "The data lives in one JSON file.",
            "decided_by": { "question": "q1" },
        }] },
    });
    env.run(&["round", "store"], &result_round.to_string());
    server.submit();

    let status = server.operate(json!({ "op": "request_review", "decision": "d1" }));

    let received = events(&env.run(&["wait", "store", "--timeout", "1"], ""));
    assert_eq!(status, 409);
    assert!(
        received
            .iter()
            .all(|event| event["kind"] != "review_requested"),
        "{received:?}"
    );
    let output = env.run(&["result", "store"], "");
    let result: Value = serde_json::from_slice(&output.stdout).expect("the result is JSON");
    assert_eq!(result["records"]["in_review"], json!([]));
}

#[test]
fn wait_and_reply_are_refused_after_end() {
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    server.submit();
    env.run(&["end", "store"], "");

    let wait = env.run(&["wait", "store", "--timeout", "1"], "");
    let reply = env.run(
        &["reply", "store", "1"],
        r#"{ "parts": [{"type":"text","body":"Too late."}] }"#,
    );

    assert!(!wait.status.success());
    assert!(!reply.status.success());
}
