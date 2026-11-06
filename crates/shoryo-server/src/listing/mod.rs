//! `shoryo ps`: every topic in the data directory and whether its server runs, read from the
//! topic's files only (`docs/spec/ps.md`).

mod endpoint;
mod fields;
mod render;
mod select;
mod state;

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Serialize;
use shoryo_core::Timestamp;

use crate::ServerError;
use crate::clock;
use crate::location::{ENDPOINT_FILE, LOCK_FILE, STATE_FILE, TopicName};
use crate::store::read_holder;

use endpoint::EndpointValues;
pub use render::{render_json, render_text};
pub use select::{Range, Selection, Target, select};
use state::StateValues;

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

/// What a scan of the data directory found.
#[derive(Debug, Default)]
pub struct Scan {
    pub topics: Vec<ListedTopic>,
    /// One line for each repository's directory that could not be read, saying which and why.
    /// Its topics are left out, and the others are still listed.
    pub unreadable_repositories: Vec<String>,
}

/// Every topic under `root` (`<data dir>/shoryo`). No data directory yet means no topics; only
/// `root` itself not being readable fails the scan.
pub fn scan(root: &Path) -> Result<Scan, ServerError> {
    let mut scan = Scan::default();
    for repository_dir in subdirectories(root)? {
        let topic_dirs = match subdirectories(&repository_dir) {
            Ok(topic_dirs) => topic_dirs,
            Err(error) => {
                scan.unreadable_repositories.push(error.to_string());
                continue;
            }
        };
        for topic_dir in topic_dirs {
            let Some(name) = topic_dir
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| TopicName::parse(name).ok())
            else {
                continue;
            };
            scan.topics
                .push(read_topic(&repository_dir, &topic_dir, &name));
        }
    }
    Ok(scan)
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
    let mut unreadable = Vec::new();
    let pid = read_holder(&topic_dir.join(LOCK_FILE)).unwrap_or_else(|error| {
        unreadable.push(format!("{LOCK_FILE} cannot be read: {error}"));
        None
    });
    let mut endpoint = match pid {
        Some(_) => EndpointValues::read(&topic_dir.join(ENDPOINT_FILE), &mut unreadable),
        None => EndpointValues::default(),
    };
    let state_file = topic_dir.join(STATE_FILE);
    let state = StateValues::read(&state_file, &mut unreadable);
    let updated_at = match fs::metadata(&state_file).and_then(|metadata| metadata.modified()) {
        Ok(modified) => Some(clock::rfc3339(modified)),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => {
            unreadable.push(format!(
                "when {STATE_FILE} was written cannot be read: {error}"
            ));
            None
        }
    };
    let repository = state.repository.or(endpoint.repository.take());
    ListedTopic {
        repository_dir: repository_dir.to_path_buf(),
        repository,
        topic: name.to_string(),
        title: state.title,
        ended: state.ended,
        latest_round: state.latest_round,
        created_at: state.created_at,
        updated_at,
        data_dir: topic_dir.display().to_string(),
        server: pid.map(|pid| listed_server(pid, endpoint)),
        unreadable,
    }
}

/// The server whose process `pid` holds the lock. Its endpoint values are all `None` in the
/// instant between taking the lock and writing the endpoint.
fn listed_server(pid: u32, endpoint: EndpointValues) -> ListedServer {
    ListedServer {
        pid,
        url: endpoint.url,
        started_at: endpoint.started_at,
        version: endpoint.version,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A data root with one repository holding one topic directory, `topic`.
    fn root_with_topic(topic: &str) -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("repository-000000000000").join(topic);
        fs::create_dir_all(&dir).unwrap();
        (root, dir)
    }

    fn only(root: &Path) -> ListedTopic {
        let mut topics = scan(root).unwrap().topics;
        assert_eq!(topics.len(), 1, "{topics:?}");
        topics.remove(0)
    }

    #[test]
    fn a_state_value_of_an_unexpected_type_is_null_with_a_warning_and_the_rest_stay() {
        let (root, dir) = root_with_topic("odd");
        let state = r#"{"repository":"/r","name":"odd","title":5,"ended":true,"rounds":[]}"#;
        fs::write(dir.join(STATE_FILE), state).unwrap();

        let topic = only(root.path());

        assert_eq!(topic.title, None);
        assert_eq!(topic.ended, Some(true));
        assert_eq!(topic.repository.as_deref(), Some("/r"));
        assert!(topic.warning().unwrap().contains("title"), "{topic:?}");
    }

    #[test]
    fn an_unreadable_lock_leaves_the_server_unknown_with_a_warning() {
        let (root, dir) = root_with_topic("locked");
        fs::create_dir(dir.join(LOCK_FILE)).unwrap();

        let topic = only(root.path());

        assert!(topic.server.is_none());
        assert!(topic.warning().unwrap().contains(LOCK_FILE), "{topic:?}");
    }

    #[test]
    fn an_unreadable_endpoint_keeps_the_pid_with_a_warning() {
        let (root, dir) = root_with_topic("served");
        fs::write(dir.join(LOCK_FILE), std::process::id().to_string()).unwrap();
        fs::write(dir.join(ENDPOINT_FILE), "not json").unwrap();

        let topic = only(root.path());

        let server = topic.server.as_ref().unwrap();
        assert_eq!(server.pid, std::process::id());
        assert_eq!(server.url, None);
        assert!(
            topic.warning().unwrap().contains(ENDPOINT_FILE),
            "{topic:?}"
        );
    }

    #[test]
    fn an_endpoint_value_of_an_unexpected_type_is_null_with_a_warning_and_the_rest_stay() {
        let (root, dir) = root_with_topic("served");
        fs::write(dir.join(LOCK_FILE), std::process::id().to_string()).unwrap();
        let endpoint = r#"{"host":"127.0.0.1","port":1234,"secret":"s","url":"http://127.0.0.1:1234/s/s/","started_at":"2026-11-07T01:02:03.004Z","version":42,"repository":"/r"}"#;
        fs::write(dir.join(ENDPOINT_FILE), endpoint).unwrap();

        let topic = only(root.path());

        let server = topic.server.as_ref().unwrap();
        assert_eq!(server.pid, std::process::id());
        assert_eq!(server.url.as_deref(), Some("http://127.0.0.1:1234/s/s/"));
        assert!(server.started_at.is_some(), "{topic:?}");
        assert_eq!(server.version, None);
        assert_eq!(topic.repository.as_deref(), Some("/r"));
        assert!(topic.warning().is_some(), "{topic:?}");
    }

    #[test]
    fn a_topic_with_nothing_unusual_has_no_warning() {
        let (root, dir) = root_with_topic("plain");
        let state = r#"{"repository":"/r","name":"plain","title":"T","ended":false,"rounds":[]}"#;
        fs::write(dir.join(STATE_FILE), state).unwrap();

        let topic = only(root.path());

        assert_eq!(topic.warning(), None);
        assert_eq!(topic.created_at, None);
        assert!(topic.latest_round.is_none());
    }
}
