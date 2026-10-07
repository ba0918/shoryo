//! The round trip between the agent's commands, the server and the page API, run against
//! the built binary (`docs/spec/server.md`, "起動", "往復", "状態データ", "記録の置き場所と寿命").

mod harness;

use std::io::{BufRead, BufReader};
use std::process::Command;

use harness::{Env, Server, files_under, first_round, text, wait_until};
use serde_json::{Value, json};

fn events(output: &std::process::Output) -> Vec<Value> {
    let parsed: Value =
        serde_json::from_slice(&output.stdout).expect("wait prints the events as JSON");
    parsed["events"]
        .as_array()
        .expect("the events are a list")
        .clone()
}

#[test]
fn version_prints_the_package_version() {
    let env = Env::new();

    let output = env.run(&["--version"], "");

    assert!(text(&output).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn second_start_of_same_topic_is_refused() {
    let env = Env::new();
    let _first = env.start("store", &[]);

    let second = env.run(&["start", "store"], "");

    assert!(!second.status.success());
}

#[test]
fn restart_restores_rounds_and_pre_submit_state() {
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    server.operate(json!({ "op": "choose", "question": "q1", "option": 1 }));
    server.operate(json!({ "op": "defer", "question": "q1", "deferred": true }));
    assert!(env.run(&["stop", "store"], "").status.success());
    server.finish();

    let again = env.start("store", &[]);

    let answer = &again.view()["topic"]["rounds"][0]["questions"][0]["answer"];
    assert_eq!(answer["selected"], 1);
    assert_eq!(answer["deferred"], true);
}

#[test]
fn refused_save_leaves_the_state_as_it_was() {
    use std::os::unix::fs::PermissionsExt;
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    let state = files_under(&env.data)
        .into_iter()
        .find(|path| path.ends_with("store/state.json"))
        .expect("the round is stored");
    let dir = state
        .parent()
        .expect("the state file is in the topic directory");
    let before = std::fs::read_to_string(&state).expect("the state can be read");
    let set_mode = |mode| {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(mode))
            .expect("the topic directory's mode can be set");
    };

    set_mode(0o500);
    let refused = server.submit();
    let submitted = server.view()["topic"]["rounds"][0]["submitted"].clone();
    let after = std::fs::read_to_string(&state).expect("the state can be read");
    set_mode(0o700);
    let retried = server.submit();

    assert_eq!(refused, 500);
    assert_eq!(submitted, false);
    assert_eq!(after, before);
    assert_eq!(retried, 200);
}

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
fn default_bind_is_loopback() {
    let env = Env::new();

    let server = env.start("store", &[]);

    assert!(
        server.url.starts_with("http://127.0.0.1:"),
        "{}",
        server.url
    );
}

#[test]
fn bind_elsewhere_prints_plain_http_warning() {
    let env = Env::new();
    let server = env.start("store", &["--bind", "0.0.0.0"]);

    env.run(&["stop", "store"], "");
    let (_, err) = server.finish();

    assert!(!err.trim().is_empty(), "no warning was printed");
}

#[test]
fn commands_reach_a_server_bound_elsewhere() {
    let env = Env::new();
    let mut server = env.start("store", &["--bind", "127.0.0.2"]);

    let round = env.run(&["round", "store"], &first_round());
    let wait = env.run(&["wait", "store", "--timeout", "1"], "");
    let stop = env.run(&["stop", "store"], "");

    assert!(round.status.success(), "{}", text(&round));
    assert!(wait.status.success(), "{}", text(&wait));
    assert!(stop.status.success(), "{}", text(&stop));
    wait_until(|| !server.is_running());
}

#[test]
fn port_flag_fixes_the_port() {
    let env = Env::new();
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("a free port can be found")
        .local_addr()
        .expect("the port is known")
        .port();

    let server = env.start("store", &["--port", &port.to_string()]);

    assert!(server.url.contains(&format!(":{port}/")), "{}", server.url);
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
    std::thread::sleep(std::time::Duration::from_millis(500));

    server.submit();

    let output = waiting.wait_with_output().expect("wait finishes");
    assert_eq!(events(&output)[0]["kind"], "submitted");
}

fn read_page_stream_until(server: &Server, needle: &str) -> bool {
    let mut response = Server::agent()
        .get(format!("{}api/events", server.url))
        .call()
        .expect("the event stream opens");
    let reader = BufReader::new(response.body_mut().as_reader());
    for line in reader.lines() {
        let Ok(line) = line else { return false };
        if line.contains(needle) {
            return true;
        }
    }
    false
}

#[test]
fn reply_reaches_open_page_without_reload() {
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    server.operate(json!({ "op": "ask", "question": "q1", "text": "Explain more" }));
    let ask = events(&env.run(&["wait", "store", "--timeout", "1"], ""))[0]["ask"].to_string();

    std::thread::scope(|scope| {
        let page = scope.spawn(|| read_page_stream_until(&server, "It is one file"));
        std::thread::sleep(std::time::Duration::from_millis(500));
        let reply = env.run(
            &["reply", "store", &ask],
            &json!({ "text": "It is one file on disk." }).to_string(),
        );
        assert!(reply.status.success(), "{}", text(&reply));
        assert!(page.join().expect("the page reader finishes"));
    });
}

#[test]
fn two_open_pages_receive_the_same_change() {
    let env = Env::new();
    let server = env.start("store", &[]);

    std::thread::scope(|scope| {
        let first = scope.spawn(|| read_page_stream_until(&server, "Which store keeps"));
        let second = scope.spawn(|| read_page_stream_until(&server, "Which store keeps"));
        std::thread::sleep(std::time::Duration::from_millis(500));
        env.run(&["round", "store"], &first_round());
        assert!(first.join().expect("the first page finishes"));
        assert!(second.join().expect("the second page finishes"));
    });
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
fn short_commands_ask_for_the_server_when_none_runs() {
    let env = Env::new();

    let output = env.run(&["round", "store"], &first_round());

    assert!(!output.status.success());
}

#[test]
fn ended_topic_page_stays_served_until_stop() {
    let env = Env::new();
    let mut server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    server.submit();
    env.run(&["end", "store"], "");

    let page = Server::agent()
        .get(&server.url)
        .call()
        .expect("the page answers");
    assert_eq!(page.status().as_u16(), 200);
    assert!(server.is_running());

    env.run(&["stop", "store"], "");
    wait_until(|| !server.is_running());
}

#[test]
fn wait_and_reply_are_refused_after_end() {
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    server.submit();
    env.run(&["end", "store"], "");

    let wait = env.run(&["wait", "store", "--timeout", "1"], "");
    let reply = env.run(&["reply", "store", "1"], r#"{ "text": "Too late." }"#);

    assert!(!wait.status.success());
    assert!(!reply.status.success());
}

#[test]
fn data_is_written_outside_the_repository() {
    let env = Env::new();
    let init = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&env.work)
        .status()
        .expect("git runs");
    assert!(init.success());
    let before = files_under(&env.work);
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    server.submit();
    env.run(&["end", "store"], "");
    env.run(&["stop", "store"], "");
    server.finish();

    assert_eq!(files_under(&env.work), before);
    let stored = files_under(&env.data);
    assert!(
        stored.iter().any(|path| path.ends_with("store/state.json")),
        "{stored:?}"
    );
}

/// The fenced blocks of `info` (for example "json round") in the skill's reference files.
fn skill_examples(info: &str) -> Vec<String> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("skills/shoryo/references");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("the skill's references exist")
        .map(|entry| entry.expect("the entry can be read").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    files.sort();
    let fence = format!("```{info}\n");
    let mut blocks = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("the reference can be read");
        let mut rest = text.as_str();
        while let Some(start) = rest.find(&fence) {
            let body = &rest[start + fence.len()..];
            let end = body.find("```").expect("every fence is closed");
            blocks.push(body[..end].to_string());
            rest = &body[end..];
        }
    }
    blocks
}

fn shapes(events: &[Value]) -> Vec<(String, Vec<String>)> {
    let mut shapes: Vec<(String, Vec<String>)> = events
        .iter()
        .map(|event| {
            let object = event.as_object().expect("an event is an object");
            let mut keys: Vec<String> = object.keys().cloned().collect();
            keys.sort();
            (event["kind"].as_str().unwrap_or_default().to_string(), keys)
        })
        .collect();
    shapes.sort();
    shapes.dedup();
    shapes
}

#[test]
fn skill_examples_are_accepted() {
    let rounds = skill_examples("json round");
    let replies = skill_examples("json reply");
    let diagrams = skill_examples("text diagram");
    let outputs = skill_examples("json output");
    assert!(
        rounds.len() >= 3 && !replies.is_empty() && !diagrams.is_empty() && !outputs.is_empty()
    );
    let env = Env::new();
    let server = env.start("examples", &[]);

    let mut seen: Vec<Value> = Vec::new();
    for (index, round) in rounds.iter().enumerate() {
        let output = env.run(&["round", "examples"], round);
        assert!(
            output.status.success(),
            "round example {}: {}",
            index + 1,
            text(&output)
        );
        if index == 0 {
            server.operate(json!({ "op": "ask", "question": "q1", "text": "Explain more" }));
            server.operate(json!({ "op": "swap_class", "question": "q2" }));
        }
        if index == 1 {
            server.operate(json!({ "op": "ask", "question": "q3", "text": "Show me" }));
            server.operate(json!({ "op": "request_review", "decision": "d1" }));
            server.operate(json!({ "op": "stop_review", "decision": "d1" }));
            server.operate(json!({ "op": "request_review", "decision": "d2" }));
        }
        server.submit();
        // Acknowledge what was received, as the skill tells the agent to.
        let ack: Vec<String> = seen.iter().map(|event| event["id"].to_string()).collect();
        let mut args = vec!["wait", "examples", "--timeout", "1"];
        let joined = ack.join(",");
        if !ack.is_empty() {
            args.extend(["--ack", joined.as_str()]);
        }
        seen.extend(events(&env.run(&args, "")));
    }
    for reply in &replies {
        let output = env.run(&["reply", "examples", "1"], reply);
        assert!(output.status.success(), "reply example: {}", text(&output));
    }
    for diagram in &diagrams {
        let reply = json!({ "text": "The diagram example.", "diagram": diagram }).to_string();
        let output = env.run(&["reply", "examples", "2"], &reply);
        assert!(
            output.status.success(),
            "diagram example: {}",
            text(&output)
        );
    }

    let actual = shapes(&seen);
    for output in &outputs {
        let documented: Value = serde_json::from_str(output).expect("the output example is JSON");
        let documented = shapes(documented["events"].as_array().expect("it lists events"));
        assert_eq!(
            documented, actual,
            "the documented events differ from what wait prints"
        );
    }
}
