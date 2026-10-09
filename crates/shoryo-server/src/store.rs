//! The topic's files: the state, the lock against a second server, and the endpoint file the
//! agent's commands use to find the running server.

use std::fs::{self, File, OpenOptions};
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
    let temporary = location.dir.join("state.json.tmp");
    fs::write(&temporary, state.to_json()).map_err(|error| ServerError::io(&temporary, error))?;
    fs::rename(&temporary, &path).map_err(|error| ServerError::io(&path, error))
}

/// Held while the server runs; removing it on drop lets the topic start again.
pub struct Lock {
    path: PathBuf,
}

impl Lock {
    /// Takes the topic's lock. The lock appears under its name already holding the PID, so
    /// another start never sees it without its holder. A lock left by a process that no longer
    /// exists is replaced.
    pub fn take(location: &TopicLocation) -> Result<Self, ServerError> {
        fs::create_dir_all(&location.dir).map_err(|error| ServerError::io(&location.dir, error))?;
        let path = location.lock_file();
        let own = std::process::id();
        for _ in 0..4 {
            let seen = match link_new(&location.dir, &path, own) {
                Ok(()) => return Ok(Self { path }),
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
    let temporary = dir.join(format!("lock.{pid}.tmp"));
    let linked =
        fs::write(&temporary, pid.to_string()).and_then(|()| fs::hard_link(&temporary, lock));
    let _ = fs::remove_file(&temporary);
    linked
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
        let mut file: File = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .map_err(|error| ServerError::io(&path, error))?;
        let json = serde_json::to_string(endpoint)
            .expect("the endpoint holds only an address, a number and strings");
        file.write_all(json.as_bytes())
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
