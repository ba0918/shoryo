//! The two forms `ps` prints (`docs/spec/ps.md`, "出力の形").

use std::path::Path;

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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
    let columns = columns(range, target);
    let header = columns
        .iter()
        .map(|column| column.heading.to_string())
        .collect();
    let rows = topics.iter().map(|topic| {
        columns
            .iter()
            .map(|column| (column.cell)(topic, home))
            .collect()
    });
    table(std::iter::once(header).chain(rows).collect())
}

/// One column of the table: its heading, and what a topic shows in it given the home directory.
struct Column {
    heading: &'static str,
    cell: fn(&ListedTopic, Option<&Path>) -> String,
}

/// Under every repository the repository comes first; with stopped topics too, the update time
/// and the end come before the PID.
fn columns(range: &Range, target: Target) -> Vec<Column> {
    let mut columns = Vec::new();
    if matches!(range, Range::AllRepositories) {
        columns.push(Column {
            heading: "REPOSITORY",
            cell: |topic, home| {
                topic
                    .repository
                    .as_deref()
                    .map(|repository| visible(&shorten_home(repository, home)))
                    .unwrap_or_default()
            },
        });
    }
    columns.extend([
        Column {
            heading: "TOPIC",
            cell: |topic, _| visible(&topic.topic),
        },
        Column {
            heading: "TITLE",
            cell: |topic, _| {
                cut(
                    &visible(topic.title.as_deref().unwrap_or_default()),
                    TITLE_WIDTH,
                )
            },
        },
        Column {
            heading: "ROUND",
            cell: |topic, _| {
                topic
                    .latest_round
                    .as_ref()
                    .map(round_cell)
                    .unwrap_or_default()
            },
        },
    ]);
    if matches!(target, Target::All) {
        columns.extend([
            Column {
                heading: "UPDATED",
                cell: |topic, _| topic.updated_at.clone().unwrap_or_default(),
            },
            Column {
                heading: "ENDED",
                cell: |topic, _| ended_cell(topic.ended).to_string(),
            },
        ]);
    }
    columns.extend([
        Column {
            heading: "PID",
            cell: |topic, _| {
                topic
                    .server
                    .as_ref()
                    .map(|server| server.pid.to_string())
                    .unwrap_or_default()
            },
        },
        Column {
            heading: "URL",
            cell: |topic, _| {
                topic
                    .server
                    .as_ref()
                    .and_then(|server| server.url.as_deref())
                    .map(visible)
                    .unwrap_or_default()
            },
        },
    ]);
    columns
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

/// `text` with each control character written as its escape (`\n`, `\u{1b}`), so that a value
/// read from a file cannot split a row or drive the terminal; other characters are kept.
fn visible(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() {
                character.escape_default().to_string()
            } else {
                character.to_string()
            }
        })
        .collect()
}

/// `text` cut to `width` terminal columns, the last one an ellipsis when anything was cut.
/// Columns, not characters: a Japanese character takes two.
fn cut(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_string();
    }
    let mut kept = String::new();
    let mut used = 0;
    for character in text.chars() {
        let next = character.width().unwrap_or(0);
        if used + next > width - 1 {
            break;
        }
        kept.push(character);
        used += next;
    }
    format!("{kept}…")
}

/// `rows`, the first being the header, each column padded to its widest cell; the last column
/// is not padded, so nothing trails a long URL.
fn table(rows: Vec<Vec<String>>) -> String {
    let columns = rows.first().map_or(0, Vec::len);
    let mut widths = vec![0; columns];
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.width());
        }
    }
    let mut out = String::new();
    for row in &rows {
        let mut line = String::new();
        for (index, cell) in row.iter().enumerate() {
            if index + 1 == columns {
                line.push_str(cell);
            } else {
                // `format!` pads by characters, which misaligns double-width ones.
                line.push_str(cell);
                line.push_str(&" ".repeat(widths[index] - cell.width() + 2));
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
