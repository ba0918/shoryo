//! Listing topics and their servers with `shoryo ps` (`docs/spec/ps.md`).

use std::path::{Path, PathBuf};
use std::process::Output;

use serde_json::Value;

use crate::harness::{Env, Server, first_round, stored_time, text};

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

/// The modification time of `path`, in milliseconds since the epoch.
fn modified_ms(path: &Path) -> i64 {
    let modified = std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .expect("the file has a modification time");
    let since_epoch = modified
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the file was written after the epoch");
    i64::try_from(since_epoch.as_millis()).expect("the time fits in i64")
}

#[test]
fn ps_json_rows_carry_what_the_topic_files_hold() {
    let env = Env::new();
    let fixture = two_repositories(&env);

    let rows = ps_json(&env, &fixture.a, &["-a"]);

    let stopped = row(&rows, "a-stopped");
    let repository = fixture.a.canonicalize().expect("the repository exists");
    assert_eq!(stopped["repository"], repository.display().to_string());
    assert_eq!(stopped["title"], "Data store");
    assert_eq!(stopped["ended"], false);
    assert_eq!(stopped["latest_round"]["number"], 1);
    assert_eq!(stopped["latest_round"]["submitted"], false);
    let state = env.state_file("a-stopped");
    assert_eq!(
        stopped["data_dir"],
        state
            .parent()
            .expect("the state is in the topic's directory")
            .display()
            .to_string()
    );
    let updated = stopped["updated_at"]
        .as_str()
        .expect("the update time is set");
    assert_eq!(stored_time(updated), (modified_ms(&state), 0));
}

#[test]
fn ps_text_shows_pid_and_full_url() {
    let env = Env::new();
    let fixture = two_repositories(&env);

    let output = ps(&env, &fixture.a, &[]);

    let table = String::from_utf8_lossy(&output.stdout);
    assert!(
        table.contains(&fixture.a_running.pid().to_string()),
        "{table}"
    );
    assert!(table.contains(&fixture.a_running.url), "{table}");
}

#[test]
fn ps_with_nothing_to_list_prints_one_line_or_an_empty_array() {
    let env = Env::new();

    let table = ps(&env, &env.work, &[]);
    let rows = ps_json(&env, &env.work, &[]);

    let listing = Env::new();
    let _running = listing.start("listed", &[]);
    let listed = ps(&listing, &listing.work, &[]);
    let words = |line: Option<&str>| -> Vec<String> {
        line.unwrap_or_default()
            .split_whitespace()
            .map(str::to_string)
            .collect()
    };
    let header = words(String::from_utf8_lossy(&listed.stdout).lines().next());
    let table = String::from_utf8_lossy(&table.stdout);
    assert_eq!(table.lines().count(), 1, "{table}");
    assert!(!table.trim().is_empty(), "{table}");
    assert_ne!(
        words(table.lines().next()),
        header,
        "the line replaces the table"
    );
    assert!(rows.is_empty());
}

#[test]
fn ps_text_notes_servers_running_in_other_repositories() {
    let env = Env::new();
    let b = env.repository("b");
    let c = env.repository("c");
    stopped_after_a_round(&env, &b, "b-stopped");
    let _b = env.start_in(&b, "b-running", &[]);
    let _c = env.start_in(&c, "c-running", &[]);

    let running = ps(&env, &env.work, &[]);
    let all = ps(&env, &env.work, &["-a"]);
    let json = ps(&env, &env.work, &["--format", "json"]);
    let every = ps(&env, &env.work, &["--all-repos"]);

    for output in [&running, &all] {
        let note = String::from_utf8_lossy(&output.stdout);
        let last = note.lines().last().unwrap_or_default();
        assert!(last.contains('2') && last.contains("--all-repos"), "{note}");
        assert!(!last.contains('3'), "{note}");
    }
    assert_eq!(String::from_utf8_lossy(&json.stdout).trim(), "[]");
    let every = String::from_utf8_lossy(&every.stdout);
    assert!(!every.contains("--all-repos"), "{every}");
}
