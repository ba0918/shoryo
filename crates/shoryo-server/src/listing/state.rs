//! The values `ps` takes from a topic's state file (`docs/spec/ps.md`, "読めない議題").

use std::path::Path;

use super::LatestRound;
use super::fields::{Fields, Presence, read_object};
use crate::location::STATE_FILE;

#[derive(Debug, Default)]
pub(super) struct StateValues {
    pub(super) repository: Option<String>,
    pub(super) title: Option<String>,
    pub(super) ended: Option<bool>,
    pub(super) latest_round: Option<LatestRound>,
    pub(super) created_at: Option<String>,
}

impl StateValues {
    /// The values of the state file at `path`, all `None` when the topic has none yet. Each
    /// value is read on its own; why one could not be read is added to `unreadable`.
    pub(super) fn read(path: &Path, unreadable: &mut Vec<String>) -> Self {
        let Some(state) = read_object(path, STATE_FILE, unreadable) else {
            return Self::default();
        };
        let mut fields = Fields::new(STATE_FILE, &state, unreadable);
        Self {
            repository: fields.string("repository", Presence::Required),
            title: fields.string("title", Presence::Required),
            ended: fields.boolean("ended"),
            latest_round: latest_round(&mut fields),
            created_at: fields.string("created_at", Presence::Optional),
        }
    }
}

/// The last round's number and whether it was sent; `None` when there is no round.
fn latest_round(fields: &mut Fields<'_>) -> Option<LatestRound> {
    let Some(rounds) = fields.get("rounds", Presence::Required)?.as_array() else {
        fields.unexpected("rounds", "a list");
        return None;
    };
    let last = rounds.last()?;
    let round = last["number"]
        .as_u64()
        .zip(last["submitted"].as_bool())
        .map(|(number, submitted)| LatestRound { number, submitted });
    if round.is_none() {
        fields.unexpected(
            "the latest round",
            "a round with a number and whether it was sent",
        );
    }
    round
}
