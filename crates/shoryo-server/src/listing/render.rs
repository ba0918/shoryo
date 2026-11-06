//! The two forms `ps` prints (`docs/spec/ps.md`, "出力の形").

use std::path::Path;

use super::{LatestRound, ListedTopic, Range, Selection, Target};

/// The JSON form: an array with one object per topic, a form other tools may read.
pub fn render_json(topics: &[ListedTopic]) -> String {
    serde_json::to_string(topics).expect("a listed topic holds only strings, numbers and nulls")
}

/// The text form: a table for a person to read, whose look may change. Under every
/// repository the repository comes first; with stopped topics too, the update time and the end
/// are added. A line after it counts the servers running outside the range.
pub fn render_text(selection: &Selection, range: &Range, target: Target) -> String {
    let mut text = text_with_home(
        &selection.topics,
        range,
        target,
        dirs::home_dir().as_deref(),
    );
    if selection.running_outside > 0 {
        text.push_str(&format!(
            "Servers running in other repositories: {}; list them with --all-repos.\n",
            selection.running_outside
        ));
    }
    text
}

/// Titles longer than this are cut, so a long one does not push the URL off the line.
const TITLE_WIDTH: usize = 40;

fn text_with_home(
    topics: &[ListedTopic],
    range: &Range,
    target: Target,
    home: Option<&Path>,
) -> String {
    if topics.is_empty() {
        return "No topics to list.\n".to_string();
    }
    let with_repository = matches!(range, Range::AllRepositories);
    let with_stopped = matches!(target, Target::All);
    let mut header = Vec::new();
    if with_repository {
        header.push("REPOSITORY");
    }
    header.extend(["TOPIC", "TITLE", "ROUND"]);
    if with_stopped {
        header.extend(["UPDATED", "ENDED"]);
    }
    header.extend(["PID", "URL"]);
    let rows = topics.iter().map(|topic| {
        let server = topic.server.as_ref();
        let mut row = Vec::new();
        if with_repository {
            row.push(
                topic
                    .repository
                    .as_deref()
                    .map(|repository| shorten_home(repository, home))
                    .unwrap_or_default(),
            );
        }
        row.push(topic.topic.clone());
        row.push(cut(topic.title.as_deref().unwrap_or_default(), TITLE_WIDTH));
        row.push(
            topic
                .latest_round
                .as_ref()
                .map(round_cell)
                .unwrap_or_default(),
        );
        if with_stopped {
            row.push(topic.updated_at.clone().unwrap_or_default());
            row.push(ended_cell(topic.ended).to_string());
        }
        row.push(
            server
                .map(|server| server.pid.to_string())
                .unwrap_or_default(),
        );
        row.push(
            server
                .and_then(|server| server.url.clone())
                .unwrap_or_default(),
        );
        row
    });
    let header = header.into_iter().map(str::to_string).collect();
    table(std::iter::once(header).chain(rows).collect())
}

/// The round's number, and whether the person sent it or it waits for their answers.
fn round_cell(round: &LatestRound) -> String {
    let state = if round.submitted { "sent" } else { "waiting" };
    format!("#{} {state}", round.number)
}

fn ended_cell(ended: Option<bool>) -> &'static str {
    match ended {
        Some(true) => "yes",
        Some(false) => "no",
        None => "",
    }
}

/// `path` with the home directory written as `~`.
fn shorten_home(path: &str, home: Option<&Path>) -> String {
    home.and_then(|home| Path::new(path).strip_prefix(home).ok())
        .map_or_else(
            || path.to_string(),
            |rest| Path::new("~").join(rest).display().to_string(),
        )
}

/// `text` cut to `width` characters, the last one an ellipsis when anything was cut.
fn cut(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let kept: String = text.chars().take(width - 1).collect();
    format!("{kept}…")
}

/// `rows`, the first being the header, each column padded to its widest cell; the last column
/// is not padded, so nothing trails a long URL.
fn table(rows: Vec<Vec<String>>) -> String {
    let columns = rows.first().map_or(0, Vec::len);
    let mut widths = vec![0; columns];
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }
    let mut out = String::new();
    for row in &rows {
        let mut line = String::new();
        for (index, cell) in row.iter().enumerate() {
            if index + 1 == columns {
                line.push_str(cell);
            } else {
                line.push_str(&format!("{cell:<width$}  ", width = widths[index]));
            }
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::listing::ListedServer;

    fn topic(name: &str, running: bool) -> ListedTopic {
        ListedTopic {
            repository_dir: PathBuf::from("/data/shoryo/a-000000000000"),
            repository: Some("/home/person/src/a".into()),
            topic: name.into(),
            title: Some("Data store".into()),
            ended: Some(false),
            latest_round: Some(LatestRound {
                number: 2,
                submitted: true,
            }),
            created_at: None,
            updated_at: Some("2026-11-07T01:02:03.004Z".into()),
            data_dir: format!("/data/shoryo/a-000000000000/{name}"),
            server: running.then(|| ListedServer {
                pid: 4321,
                url: Some("http://127.0.0.1:8080/s/0123/".into()),
                started_at: None,
                version: None,
            }),
            unreadable: Vec::new(),
        }
    }

    fn lines(text: &str) -> Vec<&str> {
        text.lines().collect()
    }

    fn home() -> Option<&'static Path> {
        Some(Path::new("/home/person"))
    }

    #[test]
    fn every_repository_puts_the_repository_first_with_the_home_directory_shortened() {
        let text = text_with_home(
            &[topic("store", true)],
            &Range::AllRepositories,
            Target::Running,
            home(),
        );

        let lines = lines(&text);
        assert!(lines[1].starts_with("~/src/a "), "{text}");
    }

    #[test]
    fn the_current_repository_alone_has_no_repository_column() {
        let range = Range::Repository(PathBuf::from("/data/shoryo/a-000000000000"));
        let text = text_with_home(&[topic("store", true)], &range, Target::Running, home());

        assert!(!text.contains("~/src/a"), "{text}");
    }

    #[test]
    fn stopped_topics_too_add_the_update_time_and_the_end_and_leave_pid_and_url_empty() {
        let range = Range::Repository(PathBuf::from("/data/shoryo/a-000000000000"));
        let text = text_with_home(&[topic("stopped", false)], &range, Target::All, home());

        let lines = lines(&text);
        assert!(lines[1].contains("2026-11-07T01:02:03.004Z"), "{text}");
        assert!(
            !lines[1].contains("4321") && !lines[1].contains("http://"),
            "{text}"
        );
    }

    #[test]
    fn long_titles_are_cut_and_urls_are_not() {
        let mut long = topic("store", true);
        long.title = Some("t".repeat(200));
        let url = format!("http://127.0.0.1:8080/s/{}/", "0".repeat(200));
        long.server.as_mut().expect("the topic runs").url = Some(url.clone());
        let range = Range::Repository(PathBuf::from("/data/shoryo/a-000000000000"));

        let text = text_with_home(&[long], &range, Target::Running, home());

        assert!(!text.contains(&"t".repeat(200)), "{text}");
        assert!(text.contains(&url), "{text}");
    }
}
