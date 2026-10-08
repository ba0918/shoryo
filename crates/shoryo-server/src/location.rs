//! Where a topic's data lives: `<data dir>/shoryo/<repository key>/<topic name>/`, outside
//! the repository (`docs/spec/server.md`, "議題と起動", "記録の置き場所と寿命").

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::ServerError;

/// A topic's name: letters, digits, `.`, `_` and `-`, so it is safe as a directory name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicName(String);

impl TopicName {
    pub fn parse(name: &str) -> Result<Self, ServerError> {
        let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-');
        if name.is_empty() || name == "." || name == ".." || !name.chars().all(allowed) {
            return Err(ServerError::InvalidTopicName(name.to_string()));
        }
        Ok(Self(name.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TopicName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The files of one topic.
#[derive(Debug, Clone)]
pub struct TopicLocation {
    pub name: TopicName,
    /// The repository top, or the start directory outside a repository.
    pub repository: PathBuf,
    pub dir: PathBuf,
}

impl TopicLocation {
    /// Locates `name` for a command started in `start_dir`.
    pub fn find(name: TopicName, start_dir: &Path) -> Result<Self, ServerError> {
        let repository = repository_top(start_dir);
        let data = dirs::data_dir().ok_or(ServerError::NoDataDirectory)?;
        let dir = data
            .join("shoryo")
            .join(repository_key(&repository))
            .join(name.as_str());
        Ok(Self {
            name,
            repository,
            dir,
        })
    }

    pub fn state_file(&self) -> PathBuf {
        self.dir.join("state.json")
    }

    pub fn lock_file(&self) -> PathBuf {
        self.dir.join("lock")
    }

    pub fn endpoint_file(&self) -> PathBuf {
        self.dir.join("endpoint.json")
    }
}

/// `git rev-parse --show-toplevel` run in `start_dir`; the start directory itself when that
/// fails (outside a repository, or without git).
fn repository_top(start_dir: &Path) -> PathBuf {
    let top = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(start_dir)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|top| PathBuf::from(top.trim_end()));
    let path = top.unwrap_or_else(|| start_dir.to_path_buf());
    path.canonicalize().unwrap_or(path)
}

/// The last path component, made safe for a directory name, then the first 12 hex digits of
/// the SHA-256 of the whole path, so two repositories with the same name stay apart.
fn repository_key(path: &Path) -> String {
    let last = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let safe: String = last
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let digest = Sha256::digest(path.as_os_str().as_encoded_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("{safe}-{}", &hex[..12])
}
