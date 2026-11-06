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

#[cfg(test)]
mod tests {
    use super::*;

    fn stopped(repository_dir: &str, repository: Option<&str>, topic: &str) -> ListedTopic {
        ListedTopic {
            repository_dir: PathBuf::from(repository_dir),
            repository: repository.map(str::to_string),
            topic: topic.into(),
            title: None,
            ended: None,
            latest_round: None,
            created_at: None,
            updated_at: Some("2026-11-07T01:02:03.004Z".into()),
            data_dir: format!("{repository_dir}/{topic}"),
            server: None,
            unreadable: Vec::new(),
        }
    }

    #[test]
    fn equal_update_times_are_ordered_by_repository_then_topic() {
        let topics = vec![
            stopped("/data/z", Some("/repos/b"), "second"),
            stopped("/data/y", Some("/repos/b"), "first"),
            stopped("/data/x", Some("/repos/c"), "first"),
            stopped("/data/a", Some("/repos/a"), "last"),
            stopped("/data/repos-without-path", None, "first"),
        ];

        let listed = select(topics, &Range::AllRepositories, Target::All);

        let order: Vec<(&str, &str)> = listed
            .topics
            .iter()
            .map(|topic| {
                (
                    topic.repository.as_deref().unwrap_or("-"),
                    topic.topic.as_str(),
                )
            })
            .collect();
        assert_eq!(
            order,
            [
                ("/repos/a", "last"),
                ("/repos/b", "first"),
                ("/repos/b", "second"),
                ("/repos/c", "first"),
                ("-", "first"),
            ]
        );
    }
}
