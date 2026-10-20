mod flow;
mod legacy;
mod sequence;

use std::collections::HashSet;

use serde::Serialize;

use crate::explanation::{Canvas, Message, Part, Point};
use crate::state::Choice;

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

/// A string of `min..=max` characters, which may be only whitespace.
fn length(path: &str, value: &str, min: usize, max: usize) -> Checked {
    bound(path, value.chars().count(), min, max)
}

/// A string of 1..=`max` characters that is not only whitespace.
fn nonblank(path: &str, value: &str, max: usize) -> Checked {
    length(path, value, 1, max)?;
    if value.trim().is_empty() {
        Err(fail(path, ExplanationRule::Blank))
    } else {
        Ok(())
    }
}

fn id(path: &str, value: &str) -> Checked {
    nonblank(path, value, 64)?;
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
    nonblank(&format!("{path}.label"), label, 512)?;
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

/// A question's background, then each option's description. `question` names the question
/// in error paths; `budget` is shared by every question of the round.
pub(crate) fn validate_question(
    question: &str,
    background: &[Part],
    options: &[Choice],
    budget: &mut usize,
) -> Checked {
    validate_parts(background, &format!("{question} background"), 0, budget)?;
    for (i, option) in options.iter().enumerate() {
        validate_parts(
            &option.description,
            &format!("{question} option {i} description"),
            0,
            budget,
        )?;
    }
    Ok(())
}

/// A reply's parts: at least one, with a byte budget of its own.
pub(crate) fn validate_reply(parts: &[Part], path: &str) -> Checked {
    validate_parts(parts, path, 1, &mut 0)
}

fn validate_parts(parts: &[Part], path: &str, minimum: usize, budget: &mut usize) -> Checked {
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
        Part::Text { body } => length(&format!("{path}.body"), body, 0, 16384),
        Part::Code {
            body,
            language,
            title,
            role: _,
        } => {
            length(&format!("{path}.body"), body, 0, 32768)?;
            nonblank(&format!("{path}.language"), language, 64)?;
            if let Some(title) = title {
                nonblank(&format!("{path}.title"), title, 120)?;
            }
            Ok(())
        }
        Part::Diagram {
            title,
            source,
            role: _,
        } => {
            nonblank(&format!("{path}.title"), title, 120)?;
            length(&format!("{path}.source"), source, 1, 32768)?;
            legacy::validate(source, &format!("{path}.source"))
        }
        Part::Sequence {
            title,
            participants,
            events,
            layout,
            canvas: area,
            role: _,
        } => sequence::validate(
            title,
            participants,
            events,
            layout.as_ref(),
            area.as_ref(),
            path,
        ),
        Part::Flow {
            title,
            nodes,
            edges,
            canvas: area,
            role: _,
        } => flow::validate(title, nodes, edges, area.as_ref(), path),
    }
}
