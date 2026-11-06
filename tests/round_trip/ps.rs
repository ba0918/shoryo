//! Listing topics and their servers with `shoryo ps` (`docs/spec/ps.md`).

use std::path::{Path, PathBuf};
use std::process::Output;

use serde_json::Value;

use crate::harness::{
    Env, Server, clock_ms, files_under, first_round, send_signal, stored_time, text,
};

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

#[test]
fn ps_help_states_the_text_default() {
    let env = Env::new();

    let help = env.run(&["ps", "--help"], "");

    assert!(help.status.success(), "{}", text(&help));
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("default: text"), "{help}");
}

/// A round after the first, asking one new question `id`.
fn next_round(id: &str) -> String {
    let mut round: Value = serde_json::from_str(&first_round()).expect("the round is JSON");
    round["questions"][0]["id"] = id.into();
    round.to_string()
}

#[test]
fn created_at_is_recorded_when_the_state_is_first_written() {
    let env = Env::new();
    let server = env.start("store", &[]);
    let created = |env: &Env| row(&ps_json(env, &env.work, &["-a"]), "store")["created_at"].clone();
    let unset = created(&env);

    let before = clock_ms();
    assert!(
        env.run(&["round", "store"], &first_round())
            .status
            .success()
    );
    let after = clock_ms();
    let first = created(&env);
    let stored = env.result("store", &[])["created_at"].clone();
    assert_eq!(server.submit(), 200);
    assert!(env.run(&["stop", "store"], "").status.success());
    server.finish();
    let again = env.start("store", &[]);
    assert!(
        env.run(&["round", "store"], &next_round("q2"))
            .status
            .success()
    );
    let later = created(&env);
    drop(again);

    assert_eq!(unset, Value::Null);
    let (instant, offset) = stored_time(first.as_str().expect("the creation time is set"));
    assert!(
        (before..=after).contains(&instant),
        "{first} is not between {before} and {after}"
    );
    assert_eq!(offset, 0);
    assert_eq!(stored, first);
    assert_eq!(later, first);
}

#[test]
fn topics_written_before_created_at_list_it_as_null_without_warning() {
    let env = Env::new();
    stopped_after_a_round(&env, &env.work, "older");
    let mut state = env.result("older", &[]);
    state
        .as_object_mut()
        .expect("the state is an object")
        .remove("created_at")
        .expect("the current format has a creation time");
    std::fs::write(env.state_file("older"), state.to_string()).expect("the state can be written");

    let listed = env.run(&["ps", "-a", "--format", "json"], "");
    let server = env.start("older", &[]);
    assert_eq!(server.submit(), 200);
    assert!(
        env.run(&["round", "older"], &next_round("q2"))
            .status
            .success()
    );
    let after_a_round = row(&ps_json(&env, &env.work, &["-a"]), "older")["created_at"].clone();

    assert!(listed.status.success(), "{}", text(&listed));
    assert_eq!(String::from_utf8_lossy(&listed.stderr), "");
    let rows: Vec<Value> = serde_json::from_slice(&listed.stdout).expect("ps prints JSON");
    assert_eq!(row(&rows, "older")["created_at"], Value::Null);
    assert_eq!(after_a_round, Value::Null);
}

#[test]
fn a_started_server_lists_its_start_time_version_and_repository() {
    let env = Env::new();
    let repository = env.repository("a");

    let before = clock_ms();
    let _server = env.start_in(&repository, "fresh", &[]);
    let after = clock_ms();
    let rows = ps_json(&env, &repository, &[]);

    let fresh = row(&rows, "fresh");
    let started = fresh["server"]["started_at"]
        .as_str()
        .expect("the start time is set");
    let (instant, offset) = stored_time(started);
    assert!(
        (before..=after).contains(&instant),
        "{started} is not between {before} and {after}"
    );
    assert_eq!(offset, 0);
    assert_eq!(fresh["server"]["version"], env!("CARGO_PKG_VERSION"));
    let canonical = repository.canonicalize().expect("the repository exists");
    assert_eq!(fresh["repository"], canonical.display().to_string());
    for key in ["title", "ended", "latest_round", "created_at", "updated_at"] {
        assert_eq!(fresh[key], Value::Null, "{key}");
    }
}

#[test]
fn commands_still_reach_a_server_whose_endpoint_lacks_the_new_fields() {
    let env = Env::new();
    let server = env.start("older", &[]);
    let endpoint = files_under(&env.data)
        .into_iter()
        .find(|path| path.ends_with("older/endpoint.json"))
        .expect("the running server has an endpoint file");
    let mut written: Value =
        serde_json::from_slice(&std::fs::read(&endpoint).expect("the endpoint can be read"))
            .expect("the endpoint is JSON");
    for key in ["started_at", "version", "repository"] {
        written
            .as_object_mut()
            .expect("the endpoint is an object")
            .remove(key)
            .unwrap_or_else(|| panic!("the endpoint has {key}"));
    }
    std::fs::write(&endpoint, written.to_string()).expect("the endpoint can be written");

    let rows = ps_json(&env, &env.work, &[]);
    let round = env.run(&["round", "older"], &first_round());
    let wait = env.run(&["wait", "older", "--timeout", "1"], "");
    let stop = env.run(&["stop", "older"], "");
    server.finish();

    let older = row(&rows, "older");
    assert_eq!(older["server"]["url"], written["url"]);
    for value in [
        &older["server"]["started_at"],
        &older["server"]["version"],
        &older["repository"],
    ] {
        assert_eq!(*value, Value::Null);
    }
    for output in [&round, &wait, &stop] {
        assert!(output.status.success(), "{}", text(output));
    }
}

/// The PID `ps` prints for the running topic `topic` of the work directory.
fn listed_pid(env: &Env, topic: &str) -> u64 {
    row(&ps_json(env, &env.work, &[]), topic)["server"]["pid"]
        .as_u64()
        .expect("a running topic is listed with its PID")
}

#[test]
fn sigterm_to_the_listed_pid_stops_the_server_and_the_topic_starts_again() {
    let env = Env::new();
    let server = env.start("stopped", &[]);

    send_signal(listed_pid(&env, "stopped"), "TERM");
    server.finish();
    let after = ps_json(&env, &env.work, &[]);
    let again = env.try_start("stopped", &[]);

    assert!(names(&after).is_empty(), "{after:?}");
    assert!(again.is_ok(), "the topic starts again");
}

#[test]
fn sigkill_to_the_listed_pid_leaves_a_topic_that_starts_again() {
    let env = Env::new();
    let server = env.start("killed", &[]);

    send_signal(listed_pid(&env, "killed"), "KILL");
    server.finish();
    let after = ps_json(&env, &env.work, &[]);
    let left = files_under(&env.data);
    let again = env.try_start("killed", &[]);

    assert!(names(&after).is_empty(), "{after:?}");
    for file in ["killed/lock", "killed/endpoint.json"] {
        assert!(
            left.iter().any(|path| path.ends_with(file)),
            "{file} remains"
        );
    }
    assert!(again.is_ok(), "the topic starts again");
}

#[test]
fn a_server_that_does_not_answer_is_still_listed_with_its_pid_and_url() {
    let env = Env::new();
    let server = env.start("frozen", &[]);
    let pid = listed_pid(&env, "frozen");

    send_signal(pid, "STOP");
    let started = std::time::Instant::now();
    let rows = ps_json(&env, &env.work, &[]);
    let took = started.elapsed();
    send_signal(pid, "KILL");
    let url = server.url.clone();
    server.finish();

    let frozen = row(&rows, "frozen");
    assert_eq!(frozen["server"]["pid"], pid);
    assert_eq!(frozen["server"]["url"], url);
    assert!(took < std::time::Duration::from_secs(5), "ps took {took:?}");
}

#[test]
fn a_corrupt_topic_is_listed_with_a_warning_and_the_rest_still_appear() {
    let env = Env::new();
    stopped_after_a_round(&env, &env.work, "healthy");
    stopped_after_a_round(&env, &env.work, "broken");
    std::fs::write(env.state_file("broken"), "{ not json").expect("the state can be written");
    let running = env.start("running-broken", &[]);
    assert!(
        env.run(&["round", "running-broken"], &first_round())
            .status
            .success()
    );
    std::fs::write(env.state_file("running-broken"), "[1, 2").expect("the state can be written");

    let output = env.run(&["ps", "-a", "--format", "json"], "");

    assert!(output.status.success(), "{}", text(&output));
    let rows: Vec<Value> = serde_json::from_slice(&output.stdout).expect("ps prints JSON");
    assert_eq!(names(&rows), ["broken", "healthy", "running-broken"]);
    assert_eq!(row(&rows, "healthy")["title"], "Data store");
    for topic in ["broken", "running-broken"] {
        let listed = row(&rows, topic);
        assert_eq!(listed["title"], Value::Null, "{topic}");
        assert!(listed["updated_at"].is_string(), "{topic}");
    }
    let listed = row(&rows, "running-broken");
    assert_eq!(listed["server"]["pid"], running.pid());
    assert_eq!(listed["server"]["url"], running.url.as_str());
    let warnings = String::from_utf8_lossy(&output.stderr);
    let lines: Vec<&str> = warnings.lines().collect();
    assert_eq!(lines.len(), 2, "{warnings}");
    for topic in ["broken", "running-broken"] {
        let dir = env.state_file(topic);
        let dir = dir.parent().expect("the state is in the topic's directory");
        let named = format!("shoryo: warning: {}: ", dir.display());
        assert!(
            lines.iter().any(|line| line.starts_with(&named)),
            "{warnings}"
        );
    }
}

/// Sets the modification time of `topic`'s state file to `minutes` after an hour ago.
fn updated_at_minute(env: &Env, topic: &str, minutes: u64) {
    let time = std::time::SystemTime::now() - std::time::Duration::from_secs(3600)
        + std::time::Duration::from_secs(60 * minutes);
    std::fs::File::options()
        .write(true)
        .open(env.state_file(topic))
        .and_then(|file| file.set_modified(time))
        .expect("the state's modification time can be set");
}

#[test]
fn running_topics_come_first_then_newest_updates() {
    let env = Env::new();
    let a = env.repository("a");
    let b = env.repository("b");
    let fresh = env.start_in(&a, "a-no-state", &[]);
    assert!(env.run_in(&a, &["stop", "a-no-state"], "").status.success());
    fresh.finish();
    stopped_after_a_round(&env, &b, "b-old");
    stopped_after_a_round(&env, &a, "a-middle");
    stopped_after_a_round(&env, &b, "b-newest");
    let _running = env.start_in(&a, "a-running", &[]);
    assert!(
        env.run_in(&a, &["round", "a-running"], &first_round())
            .status
            .success()
    );
    updated_at_minute(&env, "a-running", 10);
    updated_at_minute(&env, "b-old", 20);
    updated_at_minute(&env, "a-middle", 30);
    updated_at_minute(&env, "b-newest", 40);

    let rows = ps_json(&env, &a, &["-a", "--all-repos"]);

    let order: Vec<&str> = rows
        .iter()
        .map(|row| row["topic"].as_str().expect("each row names its topic"))
        .collect();
    assert_eq!(
        order,
        ["a-running", "b-newest", "a-middle", "b-old", "a-no-state"],
        "{rows:?}"
    );
}
