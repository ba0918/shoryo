//! Runs the built binary the way an agent does: a server in the background and short
//! commands against it, each with its own data directory and start directory.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdout, Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::{Value, json};

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// A data directory and a start directory that belong to one test.
pub struct Env {
    root: PathBuf,
    pub data: PathBuf,
    pub work: PathBuf,
}

impl Env {
    pub fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "shoryo-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        let data = root.join("data");
        let work = root.join("work");
        std::fs::create_dir_all(&data).expect("the temporary data directory can be created");
        std::fs::create_dir_all(&work).expect("the temporary start directory can be created");
        Self { root, data, work }
    }

    pub fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_shoryo"));
        command
            .args(args)
            .current_dir(&self.work)
            .env("XDG_DATA_HOME", &self.data)
            .env_remove("HTTP_PROXY")
            .env_remove("http_proxy");
        command
    }

    /// Runs a short command to completion, with `input` on its standard input.
    pub fn run(&self, args: &[&str], input: &str) -> Output {
        let mut child = self
            .command(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the binary starts");
        child
            .stdin
            .take()
            .expect("stdin is piped")
            .write_all(input.as_bytes())
            .expect("the input can be written");
        child.wait_with_output().expect("the command finishes")
    }

    /// Starts the server for `topic` and waits until it prints its URL.
    pub fn start(&self, topic: &str, extra: &[&str]) -> Server {
        let mut args = vec!["start", topic];
        args.extend_from_slice(extra);
        let mut child = self
            .command(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the server starts");
        let mut stdout = BufReader::new(child.stdout.take().expect("stdout is piped"));
        let mut first = String::new();
        stdout
            .read_line(&mut first)
            .expect("the server prints a line");
        let url = first
            .split_whitespace()
            .find(|word| word.starts_with("http://"))
            .unwrap_or_else(|| panic!("the first line names the URL: {first:?}"))
            .to_string();
        let stderr = child.stderr.take().expect("stderr is piped");
        Server {
            child,
            url,
            first_line: first,
            stdout,
            stderr,
        }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

pub struct Server {
    child: Child,
    pub url: String,
    pub first_line: String,
    stdout: BufReader<ChildStdout>,
    stderr: ChildStderr,
}

impl Server {
    pub fn agent() -> ureq::Agent {
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .proxy(None)
            .timeout_global(Some(Duration::from_secs(10)))
            .build()
            .into()
    }

    pub fn view(&self) -> Value {
        Self::agent()
            .get(format!("{}api/view", self.url))
            .call()
            .expect("the page API answers")
            .body_mut()
            .read_json()
            .expect("the view is JSON")
    }

    /// Performs one of the person's operations through the page API.
    pub fn operate(&self, operation: Value) -> u16 {
        Self::agent()
            .post(format!("{}api/op", self.url))
            .send(operation.to_string())
            .expect("the page API answers")
            .status()
            .as_u16()
    }

    /// Waits for the process to exit and returns everything it printed.
    pub fn finish(mut self) -> (String, String) {
        self.child.wait().expect("the server exits");
        let mut out = self.first_line.clone();
        self.stdout
            .read_to_string(&mut out)
            .expect("stdout can be read");
        let mut err = String::new();
        self.stderr
            .read_to_string(&mut err)
            .expect("stderr can be read");
        (out, err)
    }

    pub fn is_running(&mut self) -> bool {
        self.child
            .try_wait()
            .expect("the process can be polled")
            .is_none()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

pub fn option(text: &str, recommended: bool) -> Value {
    json!({
        "text": text,
        "description": format!("What {text} means."),
        "recommended": recommended,
        "consequence": format!("Choosing {text} leads here."),
    })
}

/// A first round with one human question, q1, about where the data lives.
pub fn first_round() -> String {
    json!({
        "title": "Data store",
        "original_request": "Where should the data live?",
        "subject": "Storage",
        "questions": [{
            "id": "q1",
            "text": "Which store keeps the topic data?",
            "class": "human",
            "why_now": "Everything else reads it.",
            "premises": [],
            "background": "A store is where the state lives.",
            "options": [option("A single JSON file", true), option("An embedded database", false)],
        }],
    })
    .to_string()
}

pub fn wait_until(mut condition: impl FnMut() -> bool) {
    for _ in 0..100 {
        if condition() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("the condition did not hold within 10 seconds");
}

pub fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in std::fs::read_dir(&next).expect("the directory can be read") {
            let path = entry.expect("the entry can be read").path();
            if path.is_dir() {
                stack.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}
