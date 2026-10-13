//! The topic's files: the state, the lock against a second server, and the endpoint file the
//! agent's commands use to find the running server.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{ErrorKind, Write};
use std::net::{IpAddr, SocketAddr};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use shoryo_core::Topic;

use crate::ServerError;
use crate::location::TopicLocation;

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
pub fn load(location: &TopicLocation) -> Result<StoredTopic, ServerError> {
    match fs::read_to_string(location.state_file()) {
        Ok(json) => {
            let state: StoredTopic =
                serde_json::from_str(&json).map_err(|error| ServerError::CorruptState {
                    path: location.state_file(),
                    reason: error.to_string(),
                })?;
            state
                .topic
                .validate_explanations()
                .map_err(|error| ServerError::CorruptState {
                    path: location.state_file(),
                    reason: error.to_string(),
                })?;
            state
                .topic
                .validate_answers()
                .map_err(|error| ServerError::CorruptState {
                    path: location.state_file(),
                    reason: error.to_string(),
                })?;
            Ok(state)
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(StoredTopic {
            repository: location.repository.display().to_string(),
            name: location.name.to_string(),
            topic: Topic::new("", ""),
        }),
        Err(error) => Err(ServerError::io(location.state_file(), error)),
    }
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
pub fn save(location: &TopicLocation, state: &StoredTopic) -> Result<(), ServerError> {
    let path = location.state_file();
    crate::atomic_file::write(&path, state.to_json().as_bytes())
        .map_err(|error| ServerError::io(&path, error))
}

/// Held while the server runs; removing it on drop lets the topic start again.
pub struct Lock {
    path: PathBuf,
    _guard: File,
}

impl Lock {
    /// Takes the topic's lock. The lock appears under its name already holding the PID, so
    /// another start never sees it without its holder. A lock left by a process that no longer
    /// exists is replaced.
    pub fn take(location: &TopicLocation) -> Result<Self, ServerError> {
        fs::create_dir_all(&location.dir).map_err(|error| ServerError::io(&location.dir, error))?;
        let path = location.lock_file();
        // The guard is never unlinked: contenders must keep locking the same inode even while
        // the PID marker is moved aside during stale-lock recovery.
        let guard_path = location.dir.join("lock.guard");
        let guard = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(&guard_path)
            .map_err(|error| ServerError::io(&guard_path, error))?;
        match guard.try_lock() {
            Ok(()) => {}
            Err(TryLockError::Error(error)) => return Err(ServerError::io(&guard_path, error)),
            Err(TryLockError::WouldBlock) => {
                if let Some(pid) = fs::read_to_string(&path)
                    .ok()
                    .and_then(|pid| pid.trim().parse::<u32>().ok())
                    .filter(|pid| process_exists(*pid))
                {
                    return Err(already_running(location, pid));
                }
                return Err(ServerError::io(
                    &path,
                    std::io::Error::other("another server is acquiring the topic lock"),
                ));
            }
        }
        let own = std::process::id();
        for _ in 0..4 {
            let seen = match link_new(&location.dir, &path, own) {
                Ok(()) => {
                    return Ok(Self {
                        path,
                        _guard: guard,
                    });
                }
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                    match fs::read_to_string(&path) {
                        Ok(seen) => seen,
                        Err(error) if error.kind() == ErrorKind::NotFound => continue,
                        Err(error) => return Err(ServerError::io(&path, error)),
                    }
                }
                Err(error) => return Err(ServerError::io(&path, error)),
            };
            let Ok(pid) = seen.trim().parse::<u32>() else {
                return Err(ServerError::UnreadableLock {
                    topic: location.name.to_string(),
                    path,
                });
            };
            if process_exists(pid) {
                return Err(already_running(location, pid));
            }
            replace_stale(location, &path, &seen, own)?;
        }
        Err(ServerError::io(
            &path,
            std::io::Error::other("the lock was taken again while replacing a stale one"),
        ))
    }
}

/// Writes `pid` to a temporary file and links it to the lock's name, which fails when the
/// lock exists.
fn link_new(dir: &Path, lock: &Path, pid: u32) -> std::io::Result<()> {
    let mut temporary = tempfile::NamedTempFile::new_in(dir)?;
    temporary.write_all(pid.to_string().as_bytes())?;
    fs::hard_link(temporary.path(), lock)
}

/// Moves the stale lock aside and removes it, but only when what was moved is still the lock
/// that was judged stale; a fresh lock another start linked in the meantime is put back.
fn replace_stale(
    location: &TopicLocation,
    lock: &Path,
    seen: &str,
    own: u32,
) -> Result<(), ServerError> {
    let aside = location.dir.join(format!("lock.stale.{own}"));
    match fs::rename(lock, &aside) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(ServerError::io(lock, error)),
    }
    let moved = fs::read_to_string(&aside).map_err(|error| ServerError::io(&aside, error));
    let result = match moved {
        Ok(moved) if moved == seen => Ok(()),
        Ok(moved) => {
            let restored =
                fs::hard_link(&aside, lock).map_err(|error| ServerError::io(lock, error));
            match (restored, moved.trim().parse::<u32>()) {
                (Ok(()), Ok(pid)) => Err(already_running(location, pid)),
                (Ok(()), Err(_)) => Err(ServerError::UnreadableLock {
                    topic: location.name.to_string(),
                    path: lock.to_path_buf(),
                }),
                (Err(error), _) => Err(error),
            }
        }
        Err(error) => Err(error),
    };
    let _ = fs::remove_file(&aside);
    result
}

fn already_running(location: &TopicLocation, pid: u32) -> ServerError {
    ServerError::AlreadyRunning {
        topic: location.name.to_string(),
        pid,
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

// The release targets Linux only, where a running process has a directory under /proc.
fn process_exists(pid: u32) -> bool {
    Path::new("/proc").join(pid.to_string()).exists()
}

/// How a command reaches the running server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    /// The address the commands connect to: the bound address, or loopback when the server
    /// listens on every address.
    pub host: IpAddr,
    pub port: u16,
    pub secret: String,
    /// The page's URL, as printed at start-up.
    pub url: String,
}

impl Endpoint {
    /// The base of the agent API.
    pub fn api_base(&self) -> String {
        format!(
            "http://{}/s/{}/api/",
            SocketAddr::new(self.host, self.port),
            self.secret
        )
    }
}

/// Removed when the server stops.
pub struct EndpointFile {
    path: PathBuf,
}

impl EndpointFile {
    /// Writes the endpoint, readable only by the user, replacing any file left behind.
    pub fn write(location: &TopicLocation, endpoint: &Endpoint) -> Result<Self, ServerError> {
        let path = location.endpoint_file();
        let json = serde_json::to_string(endpoint)
            .expect("the endpoint holds only an address, a number and strings");
        crate::atomic_file::write(&path, json.as_bytes())
            .map_err(|error| ServerError::io(&path, error))?;
        Ok(Self { path })
    }
}

impl Drop for EndpointFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// The endpoint of the topic's running server, or `None` when no server runs.
pub fn read_endpoint(location: &TopicLocation) -> Result<Option<Endpoint>, ServerError> {
    let path = location.endpoint_file();
    let running = fs::read_to_string(location.lock_file())
        .ok()
        .and_then(|pid| pid.trim().parse::<u32>().ok())
        .is_some_and(process_exists);
    if !running {
        return Ok(None);
    }
    match fs::read_to_string(&path) {
        Ok(json) => {
            serde_json::from_str(&json)
                .map(Some)
                .map_err(|error| ServerError::CorruptState {
                    path,
                    reason: error.to_string(),
                })
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ServerError::io(&path, error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location::TopicName;
    use std::os::unix::fs::{PermissionsExt, symlink};

    fn location(dir: &Path) -> TopicLocation {
        TopicLocation {
            name: TopicName::parse("test").expect("a valid topic name"),
            repository: dir.join("repository"),
            dir: dir.to_path_buf(),
        }
    }

    fn state() -> StoredTopic {
        StoredTopic {
            repository: "repository".into(),
            name: "test".into(),
            topic: Topic::new("New", "Request"),
        }
    }

    #[test]
    fn replacing_an_endpoint_does_not_inherit_readable_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let location = location(dir.path());
        fs::write(location.endpoint_file(), "old endpoint").unwrap();
        fs::set_permissions(location.endpoint_file(), fs::Permissions::from_mode(0o644)).unwrap();
        let endpoint = Endpoint {
            host: "127.0.0.1".parse().unwrap(),
            port: 1234,
            secret: "test-secret".into(),
            url: "http://example.invalid/".into(),
        };
        let _published = EndpointFile::write(&location, &endpoint).unwrap();
        let mode = fs::metadata(location.endpoint_file())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        let stored: Endpoint =
            serde_json::from_str(&fs::read_to_string(location.endpoint_file()).unwrap()).unwrap();
        assert_eq!(stored.secret, endpoint.secret);
    }

    #[test]
    fn saving_state_does_not_follow_a_preexisting_temporary_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let location = location(dir.path());
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
        let location = location(dir.path());
        fs::create_dir(location.state_file()).unwrap();
        let sentinel = location.state_file().join("existing.txt");
        fs::write(&sentinel, "Keep this").unwrap();
        assert!(save(&location, &state()).is_err());
        assert_eq!(fs::read_to_string(sentinel).unwrap(), "Keep this");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn a_running_topic_remains_exclusive_while_its_pid_marker_is_absent() {
        let dir = tempfile::tempdir().unwrap();
        let location = location(dir.path());
        let _first = Lock::take(&location).unwrap();
        fs::remove_file(location.lock_file()).unwrap();
        assert!(
            Lock::take(&location).is_err(),
            "a missing marker must not allow a second owner"
        );
    }

    #[test]
    fn a_stale_topic_can_be_taken_again_after_its_owner_finishes() {
        let dir = tempfile::tempdir().unwrap();
        let location = location(dir.path());
        fs::write(location.lock_file(), "0").unwrap();
        let first = Lock::take(&location).unwrap();
        assert_eq!(
            fs::read_to_string(location.lock_file()).unwrap(),
            std::process::id().to_string()
        );
        drop(first);
        let _next = Lock::take(&location).unwrap();
    }

    #[test]
    fn unreadable_pid_markers_are_refused_without_overwriting_them() {
        let dir = tempfile::tempdir().unwrap();
        let location = location(dir.path());
        fs::write(location.lock_file(), "not a pid").unwrap();
        assert!(matches!(
            Lock::take(&location),
            Err(ServerError::UnreadableLock { .. })
        ));
        assert_eq!(
            fs::read_to_string(location.lock_file()).unwrap(),
            "not a pid"
        );
    }
}
