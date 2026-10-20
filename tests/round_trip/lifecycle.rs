//! Starting and stopping a topic's server, where it listens, and where it keeps the data
//! (`docs/spec/server.md`, "起動", "記録の置き場所と寿命").

use std::process::Command;

use serde_json::json;

use crate::harness::{Env, Server, block_state_writes, files_under, first_round, text, wait_until};

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
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    let state = env.state_file("store");
    server.stamp_all();
    let before = std::fs::read(&state).expect("the state can be read");
    let blocked = block_state_writes(&state);
    let refused = server.send();
    let submitted = server.view()["topic"]["rounds"][0]["submitted"].clone();
    let after = blocked.stored_bytes();
    drop(blocked);
    let retried = server.send();

    assert_eq!(refused, 500);
    assert_eq!(submitted, false);
    assert_eq!(after, before);
    assert_eq!(retried, 200);
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
    let wait = env.run(&["wait", "store", "--timeout", "0"], "");
    let stop = env.run(&["stop", "store"], "");

    assert!(round.status.success(), "{}", text(&round));
    assert!(wait.status.success(), "{}", text(&wait));
    assert!(stop.status.success(), "{}", text(&stop));
    wait_until(|| !server.is_running());
}

#[test]
fn port_flag_fixes_the_port() {
    let env = Env::new();
    // A port found free can be taken by another process before the server binds it, so a
    // refused start is retried on another free port.
    let mut refusals = Vec::new();
    let (server, port) = (0..5)
        .find_map(|_| {
            let port = std::net::TcpListener::bind("127.0.0.1:0")
                .expect("a free port can be found")
                .local_addr()
                .expect("the port is known")
                .port();
            match env.try_start("store", &["--port", &port.to_string()]) {
                Ok(server) => Some((server, port)),
                Err(refused) => {
                    refusals.push(text(&refused));
                    None
                }
            }
        })
        .unwrap_or_else(|| panic!("no start on a free port within five tries: {refusals:?}"));

    assert!(server.url.contains(&format!(":{port}/")), "{}", server.url);
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
