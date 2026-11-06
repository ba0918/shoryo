//! The two forms `ps` prints (`docs/spec/ps.md`, "出力の形").

use super::ListedTopic;

/// The JSON form: an array with one object per topic, a form other tools may read.
pub fn render_json(topics: &[ListedTopic]) -> String {
    serde_json::to_string(topics).expect("a listed topic holds only strings, numbers and nulls")
}

/// The text form: a table for a person to read, whose look may change.
pub fn render_text(topics: &[ListedTopic]) -> String {
    let header = ["TOPIC", "TITLE", "ROUND", "PID", "URL"].map(str::to_string);
    let rows: Vec<[String; 5]> = topics
        .iter()
        .map(|topic| {
            let server = topic.server.as_ref();
            [
                topic.topic.clone(),
                topic.title.clone().unwrap_or_default(),
                topic
                    .latest_round
                    .as_ref()
                    .map(|round| {
                        let state = if round.submitted { "sent" } else { "waiting" };
                        format!("#{} {state}", round.number)
                    })
                    .unwrap_or_default(),
                server
                    .map(|server| server.pid.to_string())
                    .unwrap_or_default(),
                server
                    .and_then(|server| server.url.clone())
                    .unwrap_or_default(),
            ]
        })
        .collect();
    table(&header, &rows)
}

/// `rows` under `header`, each column padded to its widest cell; the last column is not
/// padded, so nothing trails a long URL.
fn table<const N: usize>(header: &[String; N], rows: &[[String; N]]) -> String {
    let mut widths = [0; N];
    for row in std::iter::once(header).chain(rows) {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }
    let mut out = String::new();
    for row in std::iter::once(header).chain(rows) {
        let mut line = String::new();
        for (index, cell) in row.iter().enumerate() {
            if index + 1 == N {
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
