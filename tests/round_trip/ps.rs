//! Listing topics and their servers with `shoryo ps` (`docs/spec/ps.md`).

use std::path::{Path, PathBuf};
use std::process::Output;

use serde_json::Value;

use crate::harness::{Env, Server, first_round, text};

/// Runs `ps` from `dir`; the command must succeed.
fn ps(env: &Env, dir: &Path, flags: &[&str]) -> Output {
    let mut args = vec!["ps"];
    args.extend_from_slice(flags);
    let output = env.run_in(dir, &args, "");
    assert!(output.status.success(), "{}", text(&output));
    output
}

/// The rows `ps --format json` prints from `dir`.
fn ps_json(env: &Env, dir: &Path, flags: &[&str]) -> Vec<Value> {
    let mut args = flags.to_vec();
    args.extend_from_slice(&["--format", "json"]);
    let output = ps(env, dir, &args);
    serde_json::from_slice::<Value>(&output.stdout)
        .expect("ps prints JSON")
        .as_array()
        .expect("ps prints an array")
        .clone()
}

/// The topic names of `rows`, sorted.
fn names(rows: &[Value]) -> Vec<String> {
    let mut names: Vec<String> = rows
        .iter()
        .map(|row| {
            row["topic"]
                .as_str()
                .expect("each row names its topic")
                .to_string()
        })
        .collect();
    names.sort();
    names
}

fn row<'a>(rows: &'a [Value], topic: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["topic"] == topic)
        .unwrap_or_else(|| panic!("topic {topic} is listed in {rows:?}"))
}

/// Starts `topic` from `dir`, sends it a first round, and stops it again.
fn stopped_after_a_round(env: &Env, dir: &Path, topic: &str) {
    let server = env.start_in(dir, topic, &[]);
    assert!(
        env.run_in(dir, &["round", topic], &first_round())
            .status
            .success()
    );
    assert!(env.run_in(dir, &["stop", topic], "").status.success());
    server.finish();
}

/// Two simulated repositories, each with a running topic and a topic stopped after a round;
/// repository A also has a topic stopped before its first round.
struct TwoRepositories {
    a: PathBuf,
    b: PathBuf,
    a_running: Server,
    _b_running: Server,
}

fn two_repositories(env: &Env) -> TwoRepositories {
    let a = env.repository("a");
    let b = env.repository("b");
    stopped_after_a_round(env, &a, "a-stopped");
    stopped_after_a_round(env, &b, "b-stopped");
    let fresh = env.start_in(&a, "a-fresh", &[]);
    assert!(env.run_in(&a, &["stop", "a-fresh"], "").status.success());
    fresh.finish();
    let a_running = env.start_in(&a, "a-running", &[]);
    let b_running = env.start_in(&b, "b-running", &[]);
    TwoRepositories {
        a,
        b,
        a_running,
        _b_running: b_running,
    }
}

#[test]
fn ps_lists_only_running_topics_of_the_current_repository() {
    let env = Env::new();
    let fixture = two_repositories(&env);

    let rows = ps_json(&env, &fixture.a, &[]);

    assert_eq!(names(&rows), ["a-running"]);
    let server = &row(&rows, "a-running")["server"];
    assert_eq!(server["pid"], fixture.a_running.pid());
    assert_eq!(server["url"], fixture.a_running.url.as_str());
}

#[test]
fn ps_all_and_all_repos_select_independently() {
    let env = Env::new();
    let fixture = two_repositories(&env);

    let all = ps_json(&env, &fixture.a, &["-a"]);
    let all_long = ps_json(&env, &fixture.a, &["--all"]);
    let all_repos = ps_json(&env, &fixture.a, &["--all-repos"]);
    let everything = ps_json(&env, &fixture.a, &["-a", "--all-repos"]);
    let all_in_b = ps_json(&env, &fixture.b, &["-a"]);

    assert_eq!(names(&all), ["a-fresh", "a-running", "a-stopped"]);
    assert_eq!(names(&all_long), names(&all));
    assert_eq!(names(&all_repos), ["a-running", "b-running"]);
    assert_eq!(
        names(&everything),
        [
            "a-fresh",
            "a-running",
            "a-stopped",
            "b-running",
            "b-stopped"
        ]
    );
    assert_eq!(names(&all_in_b), ["b-running", "b-stopped"]);
    let fresh = row(&all, "a-fresh")
        .as_object()
        .expect("a row is an object");
    let mut filled: Vec<&str> = fresh
        .iter()
        .filter(|(_, value)| !value.is_null())
        .map(|(key, _)| key.as_str())
        .collect();
    filled.sort_unstable();
    assert_eq!(filled, ["data_dir", "topic"]);
}

/// The keys of `object`, sorted.
fn keys(object: &Value) -> Vec<String> {
    let mut keys: Vec<String> = object
        .as_object()
        .expect("an object")
        .keys()
        .cloned()
        .collect();
    keys.sort();
    keys
}

#[test]
fn ps_json_rows_have_the_same_keys_whether_running_or_stopped() {
    let env = Env::new();
    let fixture = two_repositories(&env);

    let rows = ps_json(&env, &fixture.a, &["-a"]);

    let running = row(&rows, "a-running");
    let stopped = row(&rows, "a-stopped");
    let expected = [
        "created_at",
        "data_dir",
        "ended",
        "latest_round",
        "repository",
        "server",
        "title",
        "topic",
        "updated_at",
    ];
    assert_eq!(keys(running), expected);
    assert_eq!(keys(stopped), expected);
    assert_eq!(stopped["server"], Value::Null);
    assert_eq!(
        keys(&running["server"]),
        ["pid", "started_at", "url", "version"]
    );
}
