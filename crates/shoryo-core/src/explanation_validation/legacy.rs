use std::collections::HashSet;

use super::{Checked, ExplanationRule, bound, fail, length, nonblank};

pub(super) fn validate(source: &str, path: &str) -> Checked {
    let mut ids = HashSet::new();
    let mut references = Vec::new();
    let mut edges = 0;
    let mut rows = 0;
    let mut columns = None;
    for (i, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let lpath = format!("{path}.line[{}]", i + 1);
        if let Some(row) = line.strip_prefix('|') {
            rows += 1;
            let cells: Vec<_> = row
                .strip_suffix('|')
                .unwrap_or(row)
                .split('|')
                .map(str::trim)
                .collect();
            bound(&format!("{lpath}.columns"), cells.len(), 0, 32)?;
            if columns.is_some_and(|n| n != cells.len()) {
                return Err(fail(&lpath, ExplanationRule::UnequalColumns));
            }
            columns = Some(cells.len());
            for cell in cells {
                if cell != "." && !cell.is_empty() {
                    nonblank(&lpath, cell, 64)?;
                    references.push(cell);
                }
            }
        } else if let Some((from, to, label)) = legacy_edge(line) {
            for value in [from, to] {
                nonblank(&lpath, value, 64)?;
                if value.chars().any(char::is_whitespace) {
                    return Err(fail(&lpath, ExplanationRule::LegacySyntax));
                }
                references.push(value);
            }
            legacy_label(label.trim(), &format!("{lpath}.label"))?;
            edges += 1;
        } else if let Some((name, label)) = legacy_definition(line) {
            let name = name.trim();
            nonblank(&format!("{lpath}.id"), name, 64)?;
            if name.chars().any(char::is_whitespace) {
                return Err(fail(&lpath, ExplanationRule::LegacySyntax));
            }
            if !ids.insert(name) {
                return Err(fail(&lpath, ExplanationRule::Duplicate));
            }
            let label = label.trim();
            legacy_label(
                label.strip_prefix('?').unwrap_or(label).trim_start(),
                &format!("{lpath}.label"),
            )?;
        } else {
            return Err(fail(&lpath, ExplanationRule::LegacySyntax));
        }
    }
    bound(&format!("{path}.nodes"), ids.len(), 1, 64)?;
    bound(&format!("{path}.edges"), edges, 0, 128)?;
    bound(&format!("{path}.rows"), rows, 0, 64)?;
    if references.iter().any(|r| !ids.contains(r)) {
        return Err(fail(path, ExplanationRule::Reference));
    }
    Ok(())
}

fn legacy_label(label: &str, path: &str) -> Checked {
    // The unchanged browser grammar uses non-dotAll labels; accepting these would lose lines.
    if label.contains(['\r', '\u{2028}', '\u{2029}']) {
        return Err(fail(path, ExplanationRule::LegacySyntax));
    }
    length(path, label, 0, 512)
}

fn is_token(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_whitespace)
}

fn legacy_definition(line: &str) -> Option<(&str, &str)> {
    line.match_indices('=').rev().find_map(|(i, _)| {
        let name = line[..i].trim();
        is_token(name).then_some((name, &line[i + 1..]))
    })
}

fn legacy_edge(line: &str) -> Option<(&str, &str, &str)> {
    line.rmatch_indices("->").find_map(|(i, _)| {
        let from = line[..i].trim();
        let tail = line[i + 2..].trim();
        let (to, label) = tail
            .char_indices()
            .find(|(i, c)| *i > 0 && *c == ':')
            .map_or((tail, ""), |(j, _)| (&tail[..j], &tail[j + 1..]));
        let to = to.trim();
        (is_token(from) && is_token(to)).then_some((from, to, label))
    })
}
