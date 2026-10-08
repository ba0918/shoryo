use std::collections::HashSet;

use serde::Serialize;

use crate::explanation::{Canvas, FlowKind, Message, Part, Point, SequenceEvent};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExplanationError {
    pub path: String,
    pub rule: ExplanationRule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ExplanationRule {
    Bounds {
        min: usize,
        max: usize,
        actual: usize,
    },
    Blank,
    Identifier,
    Duplicate,
    Reference,
    Order,
    MissingCondition,
    LegacySyntax,
    UnequalColumns,
}

impl std::fmt::Display for ExplanationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: ", self.path)?;
        match &self.rule {
            ExplanationRule::Bounds { min, max, actual } => {
                write!(f, "value/count {actual} must be in {min}..={max}")
            }
            ExplanationRule::Blank => write!(f, "must not be blank"),
            ExplanationRule::Identifier => write!(f, "must match [A-Za-z][A-Za-z0-9_-]{{0,63}}"),
            ExplanationRule::Duplicate => write!(f, "duplicate identifier"),
            ExplanationRule::Reference => write!(f, "reference must resolve within this diagram"),
            ExplanationRule::Order => {
                write!(f, "effective participant centers must strictly increase")
            }
            ExplanationRule::MissingCondition => {
                write!(f, "decision-origin edge requires a condition label")
            }
            ExplanationRule::LegacySyntax => write!(f, "unrecognized legacy diagram line"),
            ExplanationRule::UnequalColumns => write!(f, "grid rows must have equal column counts"),
        }
    }
}

impl std::error::Error for ExplanationError {}

type Checked = Result<(), ExplanationError>;

fn fail(path: &str, rule: ExplanationRule) -> ExplanationError {
    ExplanationError {
        path: path.into(),
        rule,
    }
}

fn bound(path: &str, actual: usize, min: usize, max: usize) -> Checked {
    if (min..=max).contains(&actual) {
        Ok(())
    } else {
        Err(fail(path, ExplanationRule::Bounds { min, max, actual }))
    }
}

fn text(path: &str, value: &str, min: usize, max: usize, nonblank: bool) -> Checked {
    bound(path, value.chars().count(), min, max)?;
    if nonblank && value.trim().is_empty() {
        Err(fail(path, ExplanationRule::Blank))
    } else {
        Ok(())
    }
}

fn id(path: &str, value: &str) -> Checked {
    text(path, value, 1, 64, true)?;
    let mut chars = value.chars();
    if !chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(fail(path, ExplanationRule::Identifier));
    }
    Ok(())
}

fn number(path: &str, value: Option<u32>, min: usize, max: usize) -> Checked {
    if let Some(value) = value {
        bound(path, value as usize, min, max)?;
    }
    Ok(())
}

fn canvas(path: &str, value: Option<&Canvas>) -> Checked {
    if let Some(value) = value {
        number(&format!("{path}.width"), Some(value.width), 64, 8192)?;
        number(&format!("{path}.height"), Some(value.height), 64, 8192)?;
    }
    Ok(())
}

fn point(path: &str, value: Option<&Point>) -> Checked {
    if let Some(value) = value {
        number(&format!("{path}.x"), Some(value.x), 0, 8192)?;
        number(&format!("{path}.y"), Some(value.y), 0, 8192)?;
    }
    Ok(())
}

fn reference(path: &str, value: &str, ids: &HashSet<&str>) -> Checked {
    id(path, value)?;
    if ids.contains(value) {
        Ok(())
    } else {
        Err(fail(path, ExplanationRule::Reference))
    }
}

fn message(
    path: &str,
    from: &str,
    to: &str,
    label: &str,
    gap: Option<u32>,
    ids: &HashSet<&str>,
) -> Checked {
    reference(&format!("{path}.from"), from, ids)?;
    reference(&format!("{path}.to"), to, ids)?;
    text(&format!("{path}.label"), label, 1, 512, true)?;
    number(&format!("{path}.gap_after"), gap, 16, 512)
}

fn messages(path: &str, values: &[Message], ids: &HashSet<&str>) -> Checked {
    bound(path, values.len(), 1, 96)?;
    for (i, m) in values.iter().enumerate() {
        message(
            &format!("{path}[{i}]"),
            &m.from,
            &m.to,
            &m.label,
            m.gap_after,
            ids,
        )?;
    }
    Ok(())
}

pub(crate) fn validate_parts(
    parts: &[Part],
    path: &str,
    minimum: usize,
    budget: &mut usize,
) -> Checked {
    bound(path, parts.len(), minimum, 32)?;
    for (i, part) in parts.iter().enumerate() {
        let path = format!("{path}.part[{i}]");
        validate_part(part, &path)?;
        let value = serde_json::to_value(part)
            .expect("parts contain only serializable strings, integers and lists");
        *budget += string_bytes(&value);
        bound("explanation UTF-8 bytes", *budget, 0, 524_288)?;
    }
    Ok(())
}

fn string_bytes(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::String(value) => value.len(),
        serde_json::Value::Array(values) => values.iter().map(string_bytes).sum(),
        serde_json::Value::Object(values) => values.values().map(string_bytes).sum(),
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => 0,
    }
}

fn validate_part(part: &Part, path: &str) -> Checked {
    match part {
        Part::Text { body } => text(&format!("{path}.body"), body, 0, 16384, false),
        Part::Code {
            body,
            language,
            title,
            role: _,
        } => {
            text(&format!("{path}.body"), body, 0, 32768, false)?;
            text(&format!("{path}.language"), language, 1, 64, true)?;
            if let Some(title) = title {
                text(&format!("{path}.title"), title, 1, 120, true)?;
            }
            Ok(())
        }
        Part::Diagram {
            title,
            source,
            role: _,
        } => {
            text(&format!("{path}.title"), title, 1, 120, true)?;
            text(&format!("{path}.source"), source, 1, 32768, false)?;
            legacy(source, &format!("{path}.source"))
        }
        Part::Sequence {
            title,
            participants,
            events,
            layout,
            canvas: area,
            role: _,
        } => {
            text(&format!("{path}.title"), title, 1, 120, true)?;
            bound(&format!("{path}.participants"), participants.len(), 1, 12)?;
            bound(&format!("{path}.events"), events.len(), 1, 96)?;
            let gap = layout
                .as_ref()
                .and_then(|l| l.participant_gap)
                .unwrap_or(220);
            if let Some(l) = layout {
                for (name, value) in [
                    ("participant_gap", l.participant_gap),
                    ("event_gap", l.event_gap),
                    ("self_loop_width", l.self_loop_width),
                ] {
                    number(&format!("{path}.layout.{name}"), value, 16, 512)?;
                }
            }
            canvas(&format!("{path}.canvas"), area.as_ref())?;
            let mut ids = HashSet::new();
            let mut previous = None;
            for (i, p) in participants.iter().enumerate() {
                let ppath = format!("{path}.participants[{i}]");
                id(&format!("{ppath}.id"), &p.id)?;
                if !ids.insert(p.id.as_str()) {
                    return Err(fail(&format!("{ppath}.id"), ExplanationRule::Duplicate));
                }
                text(&format!("{ppath}.label"), &p.label, 1, 512, true)?;
                number(&format!("{ppath}.x"), p.x, 0, 8192)?;
                let center = p.x.unwrap_or(124 + i as u32 * gap);
                if previous.is_some_and(|x| center <= x) {
                    return Err(fail(&format!("{ppath}.x"), ExplanationRule::Order));
                }
                previous = Some(center);
            }
            let mut count = 0;
            let mut frames = 0;
            for (i, event) in events.iter().enumerate() {
                let epath = format!("{path}.events[{i}]");
                match event {
                    SequenceEvent::Message {
                        from,
                        to,
                        label,
                        gap_after,
                        kind: _,
                    } => {
                        message(&epath, from, to, label, *gap_after, &ids)?;
                        count += 1;
                    }
                    SequenceEvent::Loop {
                        condition,
                        messages: values,
                        gap_after,
                    } => {
                        frames += 1;
                        count += values.len();
                        text(&format!("{epath}.condition"), condition, 1, 512, true)?;
                        messages(&format!("{epath}.messages"), values, &ids)?;
                        number(&format!("{epath}.gap_after"), *gap_after, 16, 512)?;
                    }
                    SequenceEvent::Alt {
                        branches,
                        gap_after,
                    } => {
                        frames += 1;
                        bound(&format!("{epath}.branches"), branches.len(), 2, 8)?;
                        number(&format!("{epath}.gap_after"), *gap_after, 16, 512)?;
                        for (j, branch) in branches.iter().enumerate() {
                            let bpath = format!("{epath}.branches[{j}]");
                            text(
                                &format!("{bpath}.condition"),
                                &branch.condition,
                                1,
                                512,
                                true,
                            )?;
                            messages(&format!("{bpath}.messages"), &branch.messages, &ids)?;
                            count += branch.messages.len();
                        }
                    }
                }
            }
            bound(&format!("{path}.total messages"), count, 1, 96)?;
            bound(&format!("{path}.frames"), frames, 0, 12)
        }
        Part::Flow {
            title,
            nodes,
            edges,
            canvas: area,
            role: _,
        } => {
            text(&format!("{path}.title"), title, 1, 120, true)?;
            bound(&format!("{path}.nodes"), nodes.len(), 1, 32)?;
            bound(&format!("{path}.edges"), edges.len(), 0, 64)?;
            canvas(&format!("{path}.canvas"), area.as_ref())?;
            let mut ids = HashSet::new();
            for (i, node) in nodes.iter().enumerate() {
                let npath = format!("{path}.nodes[{i}]");
                id(&format!("{npath}.id"), &node.id)?;
                if !ids.insert(node.id.as_str()) {
                    return Err(fail(&format!("{npath}.id"), ExplanationRule::Duplicate));
                }
                text(&format!("{npath}.label"), &node.label, 1, 512, true)?;
                point(&format!("{npath}.position"), node.position.as_ref())?;
                number(&format!("{npath}.width"), node.width, 80, 640)?;
                number(&format!("{npath}.height"), node.height, 32, 640)?;
            }
            let mut count = 0;
            for (i, edge) in edges.iter().enumerate() {
                let epath = format!("{path}.edges[{i}]");
                reference(&format!("{epath}.from"), &edge.from, &ids)?;
                reference(&format!("{epath}.to"), &edge.to, &ids)?;
                if nodes
                    .iter()
                    .any(|n| n.id == edge.from && n.kind == FlowKind::Decision)
                    && edge.label.is_none()
                {
                    return Err(fail(
                        &format!("{epath}.label"),
                        ExplanationRule::MissingCondition,
                    ));
                }
                if let Some(via) = &edge.via {
                    bound(&format!("{epath}.via"), via.len(), 0, 12)?;
                    count += via.len();
                    for (j, p) in via.iter().enumerate() {
                        point(&format!("{epath}.via[{j}]"), Some(p))?;
                    }
                }
                if let Some(label) = &edge.label {
                    text(&format!("{epath}.label.text"), &label.text, 1, 512, true)?;
                    point(&format!("{epath}.label.position"), label.position.as_ref())?;
                }
            }
            bound(&format!("{path}.total via points"), count, 0, 768)
        }
    }
}

fn legacy(source: &str, path: &str) -> Checked {
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
                    text(&lpath, cell, 1, 64, true)?;
                    references.push(cell);
                }
            }
        } else if let Some((from, to, label)) = legacy_edge(line) {
            for value in [from, to] {
                text(&lpath, value, 1, 64, true)?;
                if value.chars().any(char::is_whitespace) {
                    return Err(fail(&lpath, ExplanationRule::LegacySyntax));
                }
                references.push(value);
            }
            text(&format!("{lpath}.label"), label.trim(), 0, 512, false)?;
            edges += 1;
        } else if let Some((name, label)) = legacy_definition(line) {
            let name = name.trim();
            text(&format!("{lpath}.id"), name, 1, 64, true)?;
            if name.chars().any(char::is_whitespace) {
                return Err(fail(&lpath, ExplanationRule::LegacySyntax));
            }
            if !ids.insert(name) {
                return Err(fail(&lpath, ExplanationRule::Duplicate));
            }
            let label = label.trim();
            text(
                &format!("{lpath}.label"),
                label.strip_prefix('?').unwrap_or(label).trim_start(),
                0,
                512,
                false,
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

fn token(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_whitespace)
}

fn legacy_definition(line: &str) -> Option<(&str, &str)> {
    line.match_indices('=').rev().find_map(|(i, _)| {
        let name = line[..i].trim();
        token(name).then_some((name, &line[i + 1..]))
    })
}

fn legacy_edge(line: &str) -> Option<(&str, &str, &str)> {
    line.match_indices("->")
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .find_map(|(i, _)| {
            let from = line[..i].trim();
            let tail = line[i + 2..].trim();
            let (to, label) = tail
                .char_indices()
                .find(|(i, c)| *i > 0 && *c == ':')
                .map_or((tail, ""), |(j, _)| (&tail[..j], &tail[j + 1..]));
            let to = to.trim();
            (token(from) && token(to)).then_some((from, to, label))
        })
}
