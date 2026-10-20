use serde_json::{Value, json};

use crate::harness::{Env, Server, first_round, text};

fn recorded_topic(env: &Env) -> Server {
    let server = env.start("records", &[]);
    let mut first: Value = serde_json::from_str(&first_round()).expect("the round fixture is JSON");
    let mut provisional = first["questions"][0].clone();
    provisional["id"] = json!("q2");
    provisional["class"] = json!("provisional");
    first["questions"]
        .as_array_mut()
        .expect("the fixture contains questions")
        .push(provisional);
    first["finished_picture"] =
        json!("agent = Agent\nstore = Store\n| agent | store |\nagent -> store : saves");
    first["questions"][0]["background"] =
        json!([{"type":"text", "body":"Historical explanation. ".repeat(400)}]);
    assert!(
        env.run(&["round", "records"], &first.to_string())
            .status
            .success()
    );
    assert_eq!(server.submit(), 200);
    let next = json!({"subject":"Result", "records":{
        "decisions":[{"id":"d1","name":"Storage","text":"Keep exact reasons, conditions, and 日本語.","decided_by":{"question":"q1"}},
            {"id":"d2","name":"Filename","text":"state.json","decided_by":{"question":"q2"}}],
        "not_building":["No remote storage"],
        "undecided":[{"text":"Retention","decider":"Person"}],
        "delegated":[{"text":"Filename","reason":"Any safe name works"}],
        "rejected":[{"text":"Database","reason":"Not needed","question":"q1"}]
    }});
    assert!(
        env.run(&["round", "records"], &next.to_string())
            .status
            .success()
    );
    server
}

#[test]
fn current_records_keep_exact_content_and_agreement_state_without_history() {
    let env = Env::new();
    let server = recorded_topic(&env);
    assert_eq!(
        server.operate(json!({"op":"request_review","decision":"d1"})),
        200
    );
    let before = server.view()["topic"].clone();
    let current = env.result("records", &["--current"]);
    assert_eq!(
        current["records"]["decisions"][0]["text"],
        "Keep exact reasons, conditions, and 日本語."
    );
    assert_eq!(
        current["records"]["decisions"][0]["origin"],
        json!({"question":"q1"})
    );
    assert_eq!(current["records"]["decisions"][0]["origin_round"], 1);
    assert_eq!(current["records"]["decisions"][0]["pre_approved"], false);
    assert_eq!(current["records"]["decisions"][1]["pre_approved"], true);
    for key in [
        "not_building",
        "undecided",
        "delegated",
        "rejected",
        "in_review",
    ] {
        assert_eq!(current["records"][key], before["records"][key]);
    }
    assert_eq!(current["latest_round"]["submitted"], false);
    assert_eq!(current["latest_round"]["number"], 2);
    assert_eq!(current["ended"], false);
    assert_eq!(
        current["pending_events"],
        before["events"].as_array().unwrap().len()
    );
    assert!(current.get("rounds").is_none());
    assert!(current["records"].get("revisions").is_none());
    assert!(current["records"]["decisions"][0].get("history").is_none());
    assert!(current.get("finished_picture").is_none());
    let full = env.result("records", &[]);
    eprintln!(
        "fixture JSON bytes: full={}, current={}",
        full.to_string().len(),
        current.to_string().len()
    );
    assert!(current.to_string().len() * 4 < full.to_string().len());
    assert_eq!(
        server.view()["topic"],
        before,
        "reading must not acknowledge events or change state"
    );
    assert!(env.run(&["stop", "records"], "").status.success());
    server.finish();
    assert_eq!(env.result("records", &["--current"]), current);
}

#[test]
fn current_records_show_only_the_latest_version_and_history_remains_retrievable() {
    let env = Env::new();
    let server = recorded_topic(&env);
    assert_eq!(server.submit(), 200);
    let revised = json!({"subject":"Revised", "review":true,
        "fixes":[{"text":"Clarify storage","decision":"d1"}],
        "records":{"decisions":[{"id":"d1","name":"New storage","text":"Updated exact condition","decided_by":"fix"}]}});
    assert!(
        env.run(&["round", "records"], &revised.to_string())
            .status
            .success()
    );
    let current = env.result("records", &["--current"]);
    assert_eq!(
        current["records"]["decisions"][0]["text"],
        "Updated exact condition"
    );
    assert!(!current.to_string().contains("Keep exact reasons"));
    let decision = env.result("records", &["--decision", "d1"]);
    assert_eq!(decision["history"].as_array().unwrap().len(), 2);
    assert_eq!(
        decision["history"][0]["content"]["text"],
        "Keep exact reasons, conditions, and 日本語."
    );
}

#[test]
fn a_new_topic_has_no_latest_round_and_no_invented_decisions() {
    let env = Env::new();
    let _server = env.start("records", &[]);
    let current = env.result("records", &["--current"]);
    assert_eq!(current["latest_round"], Value::Null);
    assert_eq!(current["records"]["decisions"], json!([]));
    assert_eq!(current["pending_events"], 0);
}

#[test]
fn current_records_report_result_submission_and_ended_state() {
    let env = Env::new();
    let server = recorded_topic(&env);
    assert_eq!(server.submit(), 200);
    let current = env.result("records", &["--current"]);
    assert_eq!(current["latest_round"]["submitted"], true);
    assert_eq!(current["latest_round"]["question_count"], 0);
    assert_eq!(current["latest_round"]["sent_as"], "proceeded");
    assert!(env.run(&["end", "records"], "").status.success());
    assert_eq!(env.result("records", &["--current"])["ended"], true);
}

#[test]
fn selected_history_matches_full_data_online_and_after_stop() {
    let env = Env::new();
    let server = recorded_topic(&env);
    let full = env.result("records", &[]);
    let round = env.result("records", &["--round", "1"]);
    let decision = env.result("records", &["--decision", "d1"]);
    assert_eq!(round, full["rounds"][0]);
    assert_eq!(decision, full["records"]["decisions"][0]);
    assert!(env.run(&["stop", "records"], "").status.success());
    server.finish();
    assert_eq!(env.result("records", &["--round", "1"]), round);
    assert_eq!(env.result("records", &["--decision", "d1"]), decision);
}

#[test]
fn unknown_or_conflicting_selectors_fail_without_falling_back_to_full_data() {
    let env = Env::new();
    let server = recorded_topic(&env);
    for flags in [
        vec!["--round", "99"],
        vec!["--decision", "missing"],
        vec!["--current", "--round", "1"],
        vec!["--round", "1", "--decision", "d1"],
    ] {
        let mut args = vec!["result", "records"];
        args.extend(flags);
        let output = env.run(&args, "");
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    assert!(env.run(&["stop", "records"], "").status.success());
    server.finish();
    for flags in [["--round", "99"], ["--decision", "missing"]] {
        let output = env.run(&["result", "records", flags[0], flags[1]], "");
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(text(&output).contains("does not exist"));
    }
}

#[test]
fn selective_reads_do_not_create_a_missing_topic() {
    let env = Env::new();
    for flags in [
        vec!["--current"],
        vec!["--round", "1"],
        vec!["--decision", "d1"],
    ] {
        let mut args = vec!["result", "records"];
        args.extend(flags);
        let output = env.run(&args, "");
        assert!(!output.status.success());
        assert!(text(&output).contains("no data yet"));
        assert!(output.stdout.is_empty());
    }
    assert!(crate::harness::files_under(&env.data).is_empty());
}

#[test]
fn selective_http_reads_validate_the_request_and_unknown_ids_return_not_found() {
    let env = Env::new();
    let server = recorded_topic(&env);
    for (request, expected) in [
        (json!({"kind":"current","unexpected":true}), 422),
        (json!({"kind":"round","number":99}), 404),
        (json!({"kind":"decision","id":"missing"}), 404),
        (json!({"kind":"unknown"}), 422),
    ] {
        let mut response = server.post("result", &request);
        assert_eq!(response.status().as_u16(), expected);
        let error: Value = response.body_mut().read_json().unwrap();
        assert!(error["error"].is_string());
        assert!(error.get("rounds").is_none());
    }
}

#[test]
fn omitted_updates_keep_records_and_picture_and_explicit_empty_lists_clear_them() {
    let env = Env::new();
    let server = recorded_topic(&env);
    let original = env.result("records", &[]);
    assert_eq!(server.submit(), 200);
    assert!(
        env.run(&["round", "records"], r#"{"subject":"Unchanged result"}"#)
            .status
            .success()
    );
    let unchanged = env.result("records", &[]);
    assert_eq!(unchanged["records"], original["records"]);
    assert_eq!(unchanged["finished_picture"], original["finished_picture"]);
    assert_eq!(
        unchanged["rounds"][2]["records"],
        original["rounds"][1]["records"]
    );
    assert_eq!(server.submit(), 200);
    assert!(env.run(&["round", "records"], r#"{"subject":"Clear lists","records":{"not_building":[],"undecided":[],"delegated":[],"rejected":[]}}"#).status.success());
    let cleared = env.result("records", &[]);
    for key in ["not_building", "undecided", "delegated", "rejected"] {
        assert_eq!(cleared["records"][key], json!([]));
    }
    assert_eq!(
        cleared["records"]["decisions"],
        original["records"]["decisions"]
    );
    assert_eq!(
        cleared["rounds"][1]["records"],
        original["rounds"][1]["records"]
    );
}
