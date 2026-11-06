//! The lock that keeps a second server off a running topic.

use std::fs::{self, File, TryLockError};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::ServerError;
use crate::location::TopicLocation;

/// Held while the server runs; removing it on drop lets the topic start again.
pub struct Lock {
    path: PathBuf,
    _guard: File,
}

impl Lock {
    /// Takes the topic's lock. The guard stays locked while the server runs, which is what
    /// tells a running server apart (`docs/spec/ps.md`, "動いているかの決め方"). The lock appears
    /// under its name already holding the PID, so another start never sees it without its
    /// holder. With the guard taken no server runs, so a lock left behind is replaced, whatever
    /// process its PID now names.
    pub fn take(location: &TopicLocation) -> Result<Self, ServerError> {
        fs::create_dir_all(&location.dir).map_err(|error| ServerError::io(&location.dir, error))?;
        let path = location.lock_file();
        // The guard is never unlinked: contenders must keep locking the same inode even while
        // the PID marker is moved aside during stale-lock recovery.
        let guard_path = location.guard_file();
        // Checked before the guard is created: once it exists, a server of an older version
        // that keeps none could no longer be told apart.
        if !guard_path.exists()
            && let Ok(Some(pid)) = lock_pid(&path)
            && process_exists(pid)
        {
            return Err(already_running(location, pid));
        }
        let guard = crate::files::open_lock(&guard_path)
            .map_err(|error| ServerError::io(&guard_path, error))?;
        let mut attempts = 0;
        loop {
            match guard.try_lock() {
                Ok(()) => break,
                Err(TryLockError::Error(error)) => {
                    return Err(ServerError::io(&guard_path, error));
                }
                // A command checking whether the topic runs holds the guard for an instant.
                Err(TryLockError::WouldBlock) if attempts < PROBE_RETRIES => {
                    attempts += 1;
                    std::thread::sleep(PROBE_WAIT);
                }
                Err(TryLockError::WouldBlock) => {
                    if let Ok(Some(pid)) = lock_pid(&path) {
                        return Err(already_running(location, pid));
                    }
                    return Err(ServerError::io(
                        &path,
                        std::io::Error::other("another server is acquiring the topic lock"),
                    ));
                }
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
            if parse_pid(&seen).is_none() {
                return Err(unreadable_lock(location, path));
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
            match (restored, parse_pid(&moved)) {
                (Ok(()), Some(pid)) => Err(already_running(location, pid)),
                (Ok(()), None) => Err(unreadable_lock(location, lock.to_path_buf())),
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

fn unreadable_lock(location: &TopicLocation, path: PathBuf) -> ServerError {
    ServerError::UnreadableLock {
        topic: location.name.to_string(),
        path,
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// How long and how often `Lock::take` waits for a command that is only checking the guard.
const PROBE_WAIT: Duration = Duration::from_millis(5);
const PROBE_RETRIES: u32 = 20;

/// Whether a server runs for the topic whose guard and lock are at `guard` and `lock`: its
/// guard is locked. Checking takes a shared lock for an instant, which `Lock::take` waits out.
/// Without a guard only a version before 0.1.2, which kept none, can have run the topic, and
/// then it runs while the process its lock names does, as those versions judged.
pub(crate) fn is_running(guard: &Path, lock: &Path) -> std::io::Result<bool> {
    let file = match File::open(guard) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(lock_pid(lock)?.is_some_and(process_exists));
        }
        Err(error) => return Err(error),
    };
    match file.try_lock_shared() {
        Ok(()) => Ok(false),
        Err(TryLockError::WouldBlock) => Ok(true),
        Err(TryLockError::Error(error)) => Err(error),
    }
}

/// The PID the lock at `path` names, whether or not that process still runs; `None` without a
/// lock. An error when the lock cannot be read or names no process.
pub(crate) fn lock_pid(path: &Path) -> std::io::Result<Option<u32>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    parse_pid(&text)
        .map(Some)
        .ok_or_else(|| std::io::Error::new(ErrorKind::InvalidData, "it names no process"))
}

fn parse_pid(text: &str) -> Option<u32> {
    text.trim().parse().ok()
}

// The release targets Linux only, where a running process has a directory under /proc.
fn process_exists(pid: u32) -> bool {
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
    fn a_left_lock_naming_another_live_process_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let location = location_in(dir.path());
        // The test's own process stands for an unrelated process that was given the PID of a
        // server that died without removing its lock. The guard is there, as every server of
        // this version leaves it.
        fs::write(location.lock_file(), std::process::id().to_string()).unwrap();
        fs::write(location.guard_file(), "").unwrap();

        let taken = Lock::take(&location);

        assert!(taken.is_ok(), "{:?}", taken.err());
    }

    #[test]
    fn a_live_server_of_a_version_without_the_guard_keeps_its_topic() {
        let dir = tempfile::tempdir().unwrap();
        let location = location_in(dir.path());
        // Versions before 0.1.2 kept no guard; the test's own process stands for such a server.
        fs::write(location.lock_file(), std::process::id().to_string()).unwrap();

        let taken = Lock::take(&location);

        assert!(
            matches!(taken, Err(ServerError::AlreadyRunning { .. })),
            "{:?}",
            taken.err()
        );
        assert!(!location.guard_file().exists(), "the guard was created");
    }

    #[test]
    fn a_start_is_not_refused_while_another_command_checks_the_lock() {
        let dir = tempfile::tempdir().unwrap();
        let location = location_in(dir.path());
        let probe = crate::files::open_lock(&dir.path().join("lock.guard")).unwrap();
        probe.try_lock_shared().unwrap();
        let release = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(5));
            drop(probe);
        });

        let taken = Lock::take(&location);
        release.join().unwrap();

        assert!(taken.is_ok(), "{:?}", taken.err());
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
