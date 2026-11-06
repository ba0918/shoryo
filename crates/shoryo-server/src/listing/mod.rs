//! `shoryo ps`: every topic in the data directory and whether its server runs, read from the
//! topic's files only (`docs/spec/ps.md`).

mod select;

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::ServerError;
use crate::location::{ENDPOINT_FILE, LOCK_FILE, TopicName};
use crate::store::{read_endpoint_file, running_holder};

pub use select::{Range, Target, select};

/// One topic as `ps` lists it. Serialized, it is one element of the JSON output.
#[derive(Debug, Clone, Serialize)]
pub struct ListedTopic {
    /// The repository's directory in the data directory, which decides the topic's range.
    #[serde(skip)]
    pub repository_dir: PathBuf,
    pub topic: String,
    pub data_dir: String,
    /// `None` when no server runs for the topic.
    pub server: Option<ListedServer>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListedServer {
    pub pid: u32,
    pub url: Option<String>,
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
        }
    });
    ListedTopic {
        repository_dir: repository_dir.to_path_buf(),
        topic: name.to_string(),
        data_dir: topic_dir.display().to_string(),
        server,
    }
}

/// The JSON form of `ps`: an array with one object per topic.
pub fn render_json(topics: &[ListedTopic]) -> String {
    serde_json::to_string(topics).expect("a listed topic holds only strings, numbers and nulls")
}
