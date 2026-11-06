//! Which topics `ps` lists (`docs/spec/ps.md`, "出す範囲と対象").

use std::path::PathBuf;

use super::ListedTopic;

/// The repositories `ps` lists from.
#[derive(Debug, Clone)]
pub enum Range {
    /// Only the repository whose directory in the data directory is this one.
    Repository(PathBuf),
}

/// Which topics in the range `ps` lists.
#[derive(Debug, Clone, Copy)]
pub enum Target {
    Running,
}

/// The topics of `topics` in `range` that `target` asks for.
pub fn select(topics: Vec<ListedTopic>, range: &Range, target: Target) -> Vec<ListedTopic> {
    topics
        .into_iter()
        .filter(|topic| in_range(topic, range) && is_target(topic, target))
        .collect()
}

fn in_range(topic: &ListedTopic, range: &Range) -> bool {
    match range {
        Range::Repository(dir) => topic.repository_dir == *dir,
    }
}

fn is_target(topic: &ListedTopic, target: Target) -> bool {
    match target {
        Target::Running => topic.server.is_some(),
    }
}
