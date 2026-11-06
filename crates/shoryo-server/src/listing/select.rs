//! Which topics `ps` lists, and in what order (`docs/spec/ps.md`, "出す範囲と対象", "並び順").

use std::cmp::{Ordering, Reverse};
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

/// The topics of `topics` in `range` that `target` asks for: running ones first, each group
/// from the newest update to the oldest, those without an update time last.
pub fn select(topics: Vec<ListedTopic>, range: &Range, target: Target) -> Selection {
    let (inside, outside): (Vec<_>, Vec<_>) =
        topics.into_iter().partition(|topic| in_range(topic, range));
    let mut listed: Vec<ListedTopic> = inside
        .into_iter()
        .filter(|topic| is_target(topic, target))
        .collect();
    listed.sort_by(listing_order);
    Selection {
        topics: listed,
        running_outside: outside
            .iter()
            .filter(|topic| topic.server.is_some())
            .count(),
    }
}

fn listing_order(left: &ListedTopic, right: &ListedTopic) -> Ordering {
    // Update times are compared as text: every one has the same fixed-width UTC form, in
    // which the text order is the time order.
    let key = |topic: &ListedTopic| {
        (
            topic.server.is_none(),
            topic.updated_at.is_none(),
            Reverse(topic.updated_at.clone()),
        )
    };
    key(left).cmp(&key(right))
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
