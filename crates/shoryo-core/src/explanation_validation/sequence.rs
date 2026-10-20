use std::collections::HashSet;

use super::{
    Checked, ExplanationRule, bound, canvas, fail, id, message, messages, nonblank, number,
};
use crate::explanation::{Canvas, Participant, SequenceEvent, SequenceLayout};

// The screen's placement of participants without `x` (`web/explanation-sequence.js`): the
// order check must judge the centres the screen will draw, so these follow the screen.
const FIRST_CENTER: u32 = 124;
const DEFAULT_PARTICIPANT_GAP: u32 = 220;

pub(super) fn validate(
    title: &str,
    participants: &[Participant],
    events: &[SequenceEvent],
    layout: Option<&SequenceLayout>,
    area: Option<&Canvas>,
    path: &str,
) -> Checked {
    nonblank(&format!("{path}.title"), title, 120)?;
    bound(&format!("{path}.participants"), participants.len(), 1, 12)?;
    bound(&format!("{path}.events"), events.len(), 1, 96)?;
    let gap = layout
        .and_then(|l| l.participant_gap)
        .unwrap_or(DEFAULT_PARTICIPANT_GAP);
    if let Some(l) = layout {
        for (name, value) in [
            ("participant_gap", l.participant_gap),
            ("event_gap", l.event_gap),
            ("self_loop_width", l.self_loop_width),
        ] {
            number(&format!("{path}.layout.{name}"), value, 16, 512)?;
        }
    }
    canvas(&format!("{path}.canvas"), area)?;
    let mut ids = HashSet::new();
    let mut previous = None;
    for (i, p) in participants.iter().enumerate() {
        let ppath = format!("{path}.participants[{i}]");
        id(&format!("{ppath}.id"), &p.id)?;
        if !ids.insert(p.id.as_str()) {
            return Err(fail(&format!("{ppath}.id"), ExplanationRule::Duplicate));
        }
        nonblank(&format!("{ppath}.label"), &p.label, 512)?;
        number(&format!("{ppath}.x"), p.x, 0, 8192)?;
        let center = p.x.unwrap_or(FIRST_CENTER + i as u32 * gap);
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
                nonblank(&format!("{epath}.condition"), condition, 512)?;
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
                    nonblank(&format!("{bpath}.condition"), &branch.condition, 512)?;
                    messages(&format!("{bpath}.messages"), &branch.messages, &ids)?;
                    count += branch.messages.len();
                }
            }
        }
    }
    bound(&format!("{path}.total messages"), count, 1, 96)?;
    bound(&format!("{path}.frames"), frames, 0, 12)
}
