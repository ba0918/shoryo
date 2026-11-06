//! Which topics `ps` lists (`docs/spec/ps.md`, "出す範囲と対象").

use std::path::PathBuf;

use super::ListedTopic;

/// The repositories `ps` lists from.
#[derive(Debug, Clone)]
pub enum Range {
    /// Only the repository whose directory in the data directory is this one.
    Repository(PathBuf),
    AllRepositories,
}

/// Which topics in the range `ps` lists.
#[derive(Debug, Clone, Copy)]
pub enum Target {
    Running,
    /// Stopped topics too.
    All,
}

/// What `ps` lists, and what it only counts.
#[derive(Debug, Clone)]
pub struct Selection {
    pub topics: Vec<ListedTopic>,
    /// Servers running outside the range, whatever the target
    /// (`docs/spec/ps.md`, "範囲の外で動いているサーバーの知らせ").
    pub running_outside: usize,
}

/// The topics of `topics` in `range` that `target` asks for.
pub fn select(topics: Vec<ListedTopic>, range: &Range, target: Target) -> Selection {
    let (inside, outside): (Vec<_>, Vec<_>) =
        topics.into_iter().partition(|topic| in_range(topic, range));
    Selection {
        topics: inside
            .into_iter()
            .filter(|topic| is_target(topic, target))
            .collect(),
        running_outside: outside
            .iter()
            .filter(|topic| topic.server.is_some())
            .count(),
    }
}

fn in_range(topic: &ListedTopic, range: &Range) -> bool {
    match range {
        Range::Repository(dir) => topic.repository_dir == *dir,
        Range::AllRepositories => true,
    }
}

fn is_target(topic: &ListedTopic, target: Target) -> bool {
    match target {
        Target::Running => topic.server.is_some(),
        Target::All => true,
    }
}
