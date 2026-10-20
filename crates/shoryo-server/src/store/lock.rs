//! The lock that keeps a second server off a running topic.

use std::fs::{self, File, TryLockError};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use crate::ServerError;
use crate::location::TopicLocation;

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
        let guard = crate::files::open_lock(&guard_path)
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
pub(super) fn process_exists(pid: u32) -> bool {
    Path::new("/proc").join(pid.to_string()).exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::location_in;

    #[test]
    fn a_running_topic_remains_exclusive_while_its_pid_marker_is_absent() {
        let dir = tempfile::tempdir().unwrap();
        let location = location_in(dir.path());
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
        let location = location_in(dir.path());
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
        let location = location_in(dir.path());
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
