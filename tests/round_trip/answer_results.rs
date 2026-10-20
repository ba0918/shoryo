use crate::harness::{Env, Server, first_round, text};
use serde_json::{Value, json};

fn topic(env: &Env) -> Server {
    let server = env.start("answers", &[]);
    let mut round: Value = serde_json::from_str(&first_round()).expect("valid fixture");
    round["questions"][0]["background"] =
        json!([{"type":"text","body":"Unrelated background. ".repeat(400)}]);
    assert!(
        env.run(&["round", "answers"], &round.to_string())
            .status
            .success()
    );
    server
}

#[test]
fn unsent_answers_do_not_expose_drafts_even_when_stamped() {
    let env = Env::new();
    let server = topic(&env);
    assert_eq!(
        server.operate(json!({"op":"note","question":"q1","text":"Draft secret"})),
        200
    );
    server.stamp_all();
    let answer = env.result("answers", &["--answer", "q1"]);
    assert_eq!(answer["submitted"], false);
    assert_eq!(answer["answer"], Value::Null);
    assert_eq!(answer["selected_option"], Value::Null);
    assert!(!answer.to_string().contains("Draft secret"));
    assert!(!answer.to_string().contains("Unrelated background"));
}

#[test]
fn sent_choice_keeps_exact_reason_and_consequence_without_explanations() {
    let env = Env::new();
    let server = topic(&env);
    let reason = "条件: ネット接続がなくても読めること。";
    assert_eq!(
        server.operate(json!({"op":"note","question":"q1","text":reason})),
        200
    );
    assert_eq!(server.submit(), 200);
    let before = env.result("answers", &[]);
    let answer = env.result("answers", &["--answer", "q1"]);
    assert_eq!(answer["question"], "q1");
    assert_eq!(answer["round"], 1);
    assert_eq!(answer["submitted"], true);
    assert_eq!(answer["sent_at"], before["rounds"][0]["sent_at"]);
    assert_eq!(answer["answer"]["note"], reason);
    assert_eq!(answer["answer"]["stamp"], "person");
    assert_eq!(
        answer["selected_option"]["text"],
        before["rounds"][0]["questions"][0]["options"][0]["text"]
    );
    assert_eq!(
        answer["selected_option"]["consequence"],
        before["rounds"][0]["questions"][0]["options"][0]["consequence"]
    );
    assert!(!answer.to_string().contains("Unrelated background"));
    assert!(answer["selected_option"].get("description").is_none());
    let question = env.result("answers", &["--question", "q1"]);
    assert_eq!(question, before["rounds"][0]["questions"][0]);
    assert_eq!(env.result("answers", &[]), before);
    assert!(env.run(&["stop", "answers"], "").status.success());
    server.finish();
    assert_eq!(env.result("answers", &["--answer", "q1"]), answer);
    assert_eq!(env.result("answers", &["--question", "q1"]), question);
}

#[test]
fn custom_and_deferred_receipts_match_actual_submitted_events_without_inactive_drafts() {
    for deferred in [false, true] {
        let env = Env::new();
        let server = topic(&env);
        assert_eq!(
            server.operate(json!({"op":"note","question":"q1","text":"Inactive note"})),
            200
        );
        assert_eq!(
            server.operate(
                json!({"op":"free_text","question":"q1","text":"Exact custom answer 日本語"})
            ),
            200
        );
        assert_eq!(
            server.operate(json!({"op":"answer_method","question":"q1","method":"custom"})),
            200
        );
        if deferred {
            assert_eq!(
                server.operate(json!({"op":"defer","question":"q1","deferred":true})),
                200
            );
        }
        assert_eq!(server.submit(), 200);
        let receipt = env.result("answers", &["--answer", "q1"]);
        let wait = env.run(&["wait", "answers", "--timeout", "0"], "");
        assert!(wait.status.success(), "{}", text(&wait));
        let events: Value = serde_json::from_slice(&wait.stdout).unwrap();
        assert_eq!(receipt["answer"], events["events"][0]["answers"][0]);
        assert_eq!(receipt["selected_option"], Value::Null);
        assert!(!receipt.to_string().contains("Inactive note"));
        if deferred {
            assert!(!receipt.to_string().contains("Exact custom answer"));
        } else {
            assert_eq!(receipt["answer"]["free_text"], "Exact custom answer 日本語");
        }
    }
}

#[test]
fn related_asks_are_indexed_without_automatically_returning_replies_or_followup_chains() {
    let env = Env::new();
    let server = topic(&env);
    assert_eq!(
        server.operate(json!({"op":"ask","question":"q1","text":"Why?"})),
        200
    );
    let reply = json!({"parts":[{"type":"text","body":"Exact reply conditions"}]});
    assert!(
        env.run(&["reply", "answers", "1"], &reply.to_string())
            .status
            .success()
    );
    assert_eq!(
        server.operate(json!({"op":"ask","question":"q1","text":"And then?","follows":1})),
        200
    );
    let receipt = env.result("answers", &["--answer", "q1"]);
    assert_eq!(
        receipt["asks"],
        json!([{"id":1,"follows":null,"status":"replied"},{"id":2,"follows":1,"status":"waiting"}])
    );
    assert!(!receipt.to_string().contains("Exact reply conditions"));
    let full = env.result("answers", &[]);
    assert_eq!(
        env.result("answers", &["--ask", "1"]),
        full["rounds"][0]["asks"][0]
    );
    assert_eq!(
        env.result("answers", &["--ask", "2"]),
        full["rounds"][0]["asks"][1]
    );
    assert_eq!(server.submit(), 200);
    assert!(env.run(&["end", "answers"], "").status.success());
    assert_eq!(
        env.result("answers", &["--ask", "2"])["state"]["status"],
        "no_reply"
    );
    assert!(env.run(&["stop", "answers"], "").status.success());
    server.finish();
    assert_eq!(
        env.result("answers", &["--ask", "1"]),
        full["rounds"][0]["asks"][0]
    );
}

#[test]
fn choice_deferral_keeps_its_reason_but_never_exposes_the_inactive_custom_draft() {
    let env = Env::new();
    let server = topic(&env);
    assert_eq!(
        server.operate(json!({"op":"free_text","question":"q1","text":"Inactive custom draft"})),
        200
    );
    assert_eq!(
        server.operate(json!({"op":"note","question":"q1","text":"Decide after the trial"})),
        200
    );
    assert_eq!(
        server.operate(json!({"op":"defer","question":"q1","deferred":true})),
        200
    );
    assert_eq!(server.submit(), 200);
    let receipt = env.result("answers", &["--answer", "q1"]);
    assert_eq!(receipt["answer"]["deferred"], true);
    assert_eq!(receipt["answer"]["note"], "Decide after the trial");
    assert_eq!(receipt["answer"]["choice"], Value::Null);
    assert_eq!(receipt["selected_option"], Value::Null);
    assert!(!receipt.to_string().contains("Inactive custom draft"));
}

#[test]
fn sent_provisional_answers_preserve_the_preapproved_stamp() {
    let env = Env::new();
    let server = env.start("answers", &[]);
    let mut round: Value = serde_json::from_str(&first_round()).unwrap();
    round["questions"][0]["class"] = json!("provisional");
    assert!(
        env.run(&["round", "answers"], &round.to_string())
            .status
            .success()
    );
    assert_eq!(server.send(), 200);
    assert_eq!(
        env.result("answers", &["--answer", "q1"])["answer"]["stamp"],
        "pre_approved"
    );
}

#[test]
fn all_selectors_are_exclusive_and_unknown_ids_never_return_the_whole_topic() {
    let env = Env::new();
    let _server = topic(&env);
    for flags in [
        vec!["--answer", "missing"],
        vec!["--question", "missing"],
        vec!["--ask", "99"],
        vec!["--current", "--answer", "q1"],
        vec!["--round", "1", "--question", "q1"],
        vec!["--decision", "d1", "--ask", "1"],
        vec!["--question", "q1", "--answer", "q1"],
        vec!["--ask", "1", "--answer", "q1"],
    ] {
        let mut args = vec!["result", "answers"];
        args.extend(flags);
        let output = env.run(&args, "");
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
}
