//! Which topics `ps` lists, and in what order (`docs/spec/ps.md`, "出す範囲と対象", "並び順").

use std::cmp::{Ordering, Reverse};
use std::path::PathBuf;

use super::{Found, ListedTopic};

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

/// The topics of `found` in `range` that `target` asks for, and how many servers run outside
/// the range.
pub(super) fn choose(found: Vec<Found>, range: &Range, target: Target) -> (Vec<Found>, usize) {
    let (inside, outside): (Vec<_>, Vec<_>) =
        found.into_iter().partition(|topic| in_range(topic, range));
    let chosen = inside
        .into_iter()
        .filter(|topic| is_target(topic, target))
        .collect();
    let running_outside = outside.iter().filter(|topic| topic.pid.is_some()).count();
    (chosen, running_outside)
}

/// Running topics first, each group from the newest update to the oldest, those without an
/// update time last.
pub(super) fn order(topics: &mut [ListedTopic]) {
    topics.sort_by(listing_order);
}

fn listing_order(left: &ListedTopic, right: &ListedTopic) -> Ordering {
    // Update times are compared as text: every one has the same fixed-width UTC form, in
    // which the text order is the time order.
    let key = |topic: &ListedTopic| {
        (
            topic.server.is_none(),
            topic.updated_at.is_none(),
            Reverse(topic.updated_at.clone()),
            repository_name(topic),
            topic.topic.clone(),
        )
    };
    key(left).cmp(&key(right))
}

/// The repository's path, or the name of its directory in the data directory when the path
/// is unknown, so that equal times still come in one order.
fn repository_name(topic: &ListedTopic) -> String {
    topic.repository.clone().unwrap_or_else(|| {
        topic
            .repository_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    })
}

fn in_range(topic: &Found, range: &Range) -> bool {
    match range {
        Range::Repository(dir) => topic.repository_dir == *dir,
        Range::AllRepositories => true,
    }
}

fn is_target(topic: &Found, target: Target) -> bool {
    match target {
        Target::Running => topic.pid.is_some(),
        Target::All => true,
    }
}
