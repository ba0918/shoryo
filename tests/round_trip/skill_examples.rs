//! Every input example in the shoryo skill's references is accepted, and the documented
//! output matches what `wait` prints.

use serde_json::{Value, json};

use crate::harness::{Env, Server, events, first_round, text};

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
    let reply_server = env.start("reply-examples", &[]);
    let output = env.run(&["round", "reply-examples"], &first_round());
    assert!(output.status.success(), "{}", text(&output));
    for reply in &replies {
        let id = fresh_example_ask(&reply_server);
        let output = env.run(&["reply", "reply-examples", &id.to_string()], reply);
        assert!(output.status.success(), "reply example: {}", text(&output));
    }
    for diagram in &diagrams {
        let id = fresh_example_ask(&reply_server);
        let reply = json!({ "parts": [{"type":"diagram","title":"Diagram example","role":"example","source":diagram}] }).to_string();
        let output = env.run(&["reply", "reply-examples", &id.to_string()], &reply);
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

fn fresh_example_ask(server: &Server) -> u64 {
    assert_eq!(
        server.operate(json!({"op":"ask","question":"q1","text":"Explain this example"})),
        200
    );
    server.view()["topic"]["rounds"]
        .as_array()
        .expect("the view contains rounds")
        .iter()
        .flat_map(|round| round["asks"].as_array().expect("each round contains asks"))
        .map(|ask| ask["id"].as_u64().expect("ask IDs are unsigned integers"))
        .max()
        .expect("the fresh example ask exists")
}
