//! `shoryo ps`: every topic in the data directory and whether its server runs, read from the
//! topic's files only (`docs/spec/ps.md`).

mod render;
mod select;

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;
use shoryo_core::Timestamp;

use crate::ServerError;
use crate::clock;
use crate::location::{ENDPOINT_FILE, LOCK_FILE, STATE_FILE, TopicName};
use crate::store::{Endpoint, read_endpoint_file, running_holder};

pub use render::{render_json, render_text};
pub use select::{Range, Selection, Target, select};

/// One topic as `ps` lists it. Serialized, it is one element of the JSON output.
#[derive(Debug, Clone, Serialize)]
pub struct ListedTopic {
    /// The repository's directory in the data directory, which decides the topic's range.
    #[serde(skip)]
    pub repository_dir: PathBuf,
    pub repository: Option<String>,
    pub topic: String,
    pub title: Option<String>,
    pub ended: Option<bool>,
    pub latest_round: Option<LatestRound>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub data_dir: String,
    /// `None` when no server runs for the topic.
    pub server: Option<ListedServer>,
    /// Why some values could not be read; they are `None` above.
    #[serde(skip)]
    pub unreadable: Vec<String>,
}

impl ListedTopic {
    /// One line saying what of this topic could not be read, if anything.
    pub fn warning(&self) -> Option<String> {
        (!self.unreadable.is_empty())
            .then(|| format!("{}: {}", self.data_dir, self.unreadable.join("; ")))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LatestRound {
    pub number: u64,
    /// Whether the person has sent the round.
    pub submitted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListedServer {
    pub pid: u32,
    pub url: Option<String>,
    pub started_at: Option<Timestamp>,
    pub version: Option<String>,
}

/// Every topic under `root` (`<data dir>/shoryo`). No data directory yet means no topics.
pub fn scan(root: &Path) -> Result<Vec<ListedTopic>, ServerError> {
    let mut topics = Vec::new();
    for repository_dir in subdirectories(root)? {
        for topic_dir in subdirectories(&repository_dir)? {
            let Some(name) = topic_dir
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| TopicName::parse(name).ok())
            else {
                continue;
            };
            topics.push(read_topic(&repository_dir, &topic_dir, &name));
        }
    }
    Ok(topics)
}

/// The directories directly in `dir`; none when `dir` does not exist.
fn subdirectories(dir: &Path) -> Result<Vec<PathBuf>, ServerError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(ServerError::io(dir, error)),
    };
    let mut found = Vec::new();
    for entry in entries {
        let path = entry.map_err(|error| ServerError::io(dir, error))?.path();
        if path.is_dir() {
            found.push(path);
        }
    }
    Ok(found)
}

fn read_topic(repository_dir: &Path, topic_dir: &Path, name: &TopicName) -> ListedTopic {
    let pid = running_holder(&topic_dir.join(LOCK_FILE));
    let endpoint = pid.and_then(|_| {
        read_endpoint_file(&topic_dir.join(ENDPOINT_FILE))
            .ok()
            .flatten()
    });
    let mut unreadable = Vec::new();
    let state_file = topic_dir.join(STATE_FILE);
    let state = read_json(&state_file, &mut unreadable);
    let updated_at = fs::metadata(&state_file)
        .and_then(|metadata| metadata.modified())
        .ok()
        .map(clock::rfc3339);
    let repository = string(&state, "repository").or_else(|| {
        endpoint
            .as_ref()
            .and_then(|endpoint| endpoint.repository.clone())
    });
    ListedTopic {
        repository_dir: repository_dir.to_path_buf(),
        repository,
        topic: name.to_string(),
        title: string(&state, "title"),
        ended: state["ended"].as_bool(),
        latest_round: latest_round(&state),
        created_at: string(&state, "created_at"),
        updated_at,
        data_dir: topic_dir.display().to_string(),
        server: pid.map(|pid| listed_server(pid, endpoint)),
        unreadable,
    }
}

/// The JSON in the file at `path`; `Null` when there is no such file, or when it cannot be
/// read, which is noted in `unreadable`.
fn read_json(path: &Path, unreadable: &mut Vec<String>) -> Value {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Value::Null,
        Err(error) => {
            unreadable.push(format!("{name} cannot be read: {error}"));
            return Value::Null;
        }
    };
    serde_json::from_str(&text).unwrap_or_else(|error| {
        unreadable.push(format!("{name} is not JSON: {error}"));
        Value::Null
    })
}

/// The server whose process `pid` holds the lock. Its endpoint is `None` in the instant
/// between taking the lock and writing the endpoint.
fn listed_server(pid: u32, endpoint: Option<Endpoint>) -> ListedServer {
    let Some(endpoint) = endpoint else {
        return ListedServer {
            pid,
            url: None,
            started_at: None,
            version: None,
        };
    };
    ListedServer {
        pid,
        url: Some(endpoint.url),
        started_at: endpoint.started_at,
        version: endpoint.version,
    }
}

fn string(state: &Value, key: &str) -> Option<String> {
    state[key].as_str().map(str::to_string)
}

fn latest_round(state: &Value) -> Option<LatestRound> {
    let round = state["rounds"].as_array()?.last()?;
    Some(LatestRound {
        number: round["number"].as_u64()?,
        submitted: round["submitted"].as_bool()?,
    })
}
