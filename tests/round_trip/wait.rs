use crate::harness::{Env, Server, first_round};
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
    let mut response = Server::agent()
        .post(format!("{}api/wait", server.url))
        .send(json!({"ack":[id], "timeout":u64::MAX}).to_string())
        .expect("the server returns a refusal rather than dropping the request");
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
    let mut response = Server::agent()
        .post(format!("{}api/wait", server.url))
        .send(json!({"timeout":0}).to_string())
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(
        response.body_mut().read_json::<Value>().unwrap(),
        json!({"events":[]})
    );
    assert_eq!(server.view()["agent"]["waiting"], false);
}
