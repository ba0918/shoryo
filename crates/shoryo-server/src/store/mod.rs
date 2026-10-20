//! The topic's files: the state, the lock against a second server, and the endpoint file the
//! agent's commands use to find the running server.

mod endpoint;
mod lock;

use std::fs;
use std::io::ErrorKind;

use serde::{Deserialize, Serialize};
use shoryo_core::Topic;

use crate::ServerError;
use crate::location::TopicLocation;

pub(crate) use endpoint::EndpointFile;
pub use endpoint::{Endpoint, read_endpoint};
pub(crate) use lock::Lock;

/// The state file: the topic, and which repository and name it belongs to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTopic {
    pub repository: String,
    pub name: String,
    #[serde(flatten)]
    pub topic: Topic,
}

impl StoredTopic {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self)
            .expect("the state holds only strings, numbers, booleans and lists of them")
    }
}

/// Reads the topic's state, or starts a new one when there is none yet.
pub(crate) fn load(location: &TopicLocation) -> Result<StoredTopic, ServerError> {
    Ok(load_existing(location)?.unwrap_or_else(|| StoredTopic {
        repository: location.repository.display().to_string(),
        name: location.name.to_string(),
        topic: Topic::new("", ""),
    }))
}

/// Reads and validates an existing topic without inventing an empty topic when absent.
pub fn load_existing(location: &TopicLocation) -> Result<Option<StoredTopic>, ServerError> {
    let Some(json) = read_state(location)? else {
        return Ok(None);
    };
    let corrupt = |reason: String| ServerError::CorruptState {
        path: location.state_file(),
        reason,
    };
    let state: StoredTopic =
        serde_json::from_str(&json).map_err(|error| corrupt(error.to_string()))?;
    state
        .topic
        .validate()
        .map_err(|error| corrupt(error.to_string()))?;
    Ok(Some(state))
}

/// The state file's text, if the topic has one.
pub fn read_state(location: &TopicLocation) -> Result<Option<String>, ServerError> {
    match fs::read_to_string(location.state_file()) {
        Ok(json) => Ok(Some(json)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ServerError::io(location.state_file(), error)),
    }
}

/// Writes the state through a temporary file in the same directory and a rename, so a crash
/// leaves either the old state or the new one.
pub(crate) fn save(location: &TopicLocation, state: &StoredTopic) -> Result<(), ServerError> {
    let path = location.state_file();
    crate::files::atomic_write(&path, state.to_json().as_bytes())
        .map_err(|error| ServerError::io(&path, error))
}

/// A topic location whose files all live in `dir`, for the tests of the store.
#[cfg(test)]
fn location_in(dir: &std::path::Path) -> TopicLocation {
    TopicLocation {
        name: crate::location::TopicName::parse("test").expect("a valid topic name"),
        repository: dir.join("repository"),
        dir: dir.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn state() -> StoredTopic {
        StoredTopic {
            repository: "repository".into(),
            name: "test".into(),
            topic: Topic::new("New", "Request"),
        }
    }

    #[test]
    fn saving_state_does_not_follow_a_preexisting_temporary_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let location = location_in(dir.path());
        let protected = dir.path().join("unrelated.txt");
        fs::write(&protected, "Keep this").unwrap();
        symlink(&protected, dir.path().join("state.json.tmp")).unwrap();
        save(&location, &state()).unwrap();
        assert_eq!(fs::read_to_string(&protected).unwrap(), "Keep this");
        assert!(
            !fs::symlink_metadata(location.state_file())
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(load(&location).unwrap().topic.title, "New");
    }

    #[test]
    fn failed_publication_preserves_the_destination_and_removes_temporary_files() {
        let dir = tempfile::tempdir().unwrap();
        let location = location_in(dir.path());
        fs::create_dir(location.state_file()).unwrap();
        let sentinel = location.state_file().join("existing.txt");
        fs::write(&sentinel, "Keep this").unwrap();
        assert!(save(&location, &state()).is_err());
        assert_eq!(fs::read_to_string(sentinel).unwrap(), "Keep this");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
