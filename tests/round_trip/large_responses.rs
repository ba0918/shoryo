use crate::harness::{Env, first_round, text};
use serde_json::{Value, json};

#[test]
fn large_topic_histories_are_returned_in_full() {
    let env = Env::new();
    let server = env.start("large", &[]);
    let payload = "x".repeat(3 * 1024 * 1024);
    for number in 1..=2 {
        let id = format!("d{number}");
        let input = json!({"subject":"Large records", "review":true,
            "fixes":[{"text":"Added a decision", "decision":id}],
            "records":{"decisions":[{"id":id,"name":"Decision","text":payload,"decided_by":"fix"}]}});
        let done = env.run(&["round", "large"], &input.to_string());
        assert!(done.status.success(), "{}", text(&done));
        assert_eq!(server.operate(json!({"op":"submit", "round":number})), 200);
    }
    let output = env.run(&["result", "large"], "");
    assert!(output.status.success(), "{}", text(&output));
    assert!(output.stdout.len() > 10 * 1024 * 1024);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    for decision in result["records"]["decisions"].as_array().unwrap() {
        assert_eq!(decision["history"][0]["content"]["text"], payload);
    }
}

#[test]
fn large_pending_event_batches_are_returned_without_loss() {
    let env = Env::new();
    let server = env.start("large", &[]);
    assert!(
        env.run(&["round", "large"], &first_round())
            .status
            .success()
    );
    let payload = "\u{1}".repeat(200_000);
    for _ in 0..9 {
        assert_eq!(
            server.operate(json!({"op":"ask","question":"q1","text":payload})),
            200
        );
    }
    let output = env.run(&["wait", "large", "--timeout", "0"], "");
    assert!(output.status.success(), "{}", text(&output));
    assert!(output.stdout.len() > 10 * 1024 * 1024);
    let received: Value = serde_json::from_slice(&output.stdout).unwrap();
    let events = received["events"].as_array().unwrap();
    assert_eq!(events.len(), 9);
    for event in events {
        assert_eq!(event["text"], payload);
    }
}
