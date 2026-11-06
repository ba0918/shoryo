//! `shoryo ps`: every topic in the data directory and whether its server runs, read from the
//! topic's files only (`docs/spec/ps.md`).

mod render;
mod select;

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use crate::ServerError;
use crate::clock;
use crate::location::{ENDPOINT_FILE, LOCK_FILE, STATE_FILE, TopicName};
use crate::store::{read_endpoint_file, running_holder};

pub use render::{render_json, render_text};
pub use select::{Range, Target, select};

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
    pub started_at: Option<String>,
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
    let server = running_holder(&topic_dir.join(LOCK_FILE)).map(|pid| {
        let endpoint = read_endpoint_file(&topic_dir.join(ENDPOINT_FILE))
            .ok()
            .flatten();
        ListedServer {
            pid,
            url: endpoint.map(|endpoint| endpoint.url),
            started_at: None,
            version: None,
        }
    });
    let state_file = topic_dir.join(STATE_FILE);
    let state = fs::read_to_string(&state_file)
        .ok()
        .and_then(|json| serde_json::from_str::<Value>(&json).ok())
        .unwrap_or(Value::Null);
    let updated_at = fs::metadata(&state_file)
        .and_then(|metadata| metadata.modified())
        .ok()
        .map(clock::rfc3339);
    ListedTopic {
        repository_dir: repository_dir.to_path_buf(),
        repository: string(&state, "repository"),
        topic: name.to_string(),
        title: string(&state, "title"),
        ended: state["ended"].as_bool(),
        latest_round: latest_round(&state),
        created_at: None,
        updated_at,
        data_dir: topic_dir.display().to_string(),
        server,
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
