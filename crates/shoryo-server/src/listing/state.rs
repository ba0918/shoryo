//! The values `ps` takes from a topic's state file, read without the state's types so that a
//! file that is partly unexpected still yields what it has (`docs/spec/ps.md`, "読めない議題").

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde_json::{Map, Value};

use super::LatestRound;
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
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => return Self::default(),
            Err(error) => {
                unreadable.push(format!("{STATE_FILE} cannot be read: {error}"));
                return Self::default();
            }
        };
        let state = match serde_json::from_str::<Value>(&text) {
            Ok(Value::Object(state)) => state,
            Ok(_) => {
                unreadable.push(format!("{STATE_FILE} is not a JSON object"));
                return Self::default();
            }
            Err(error) => {
                unreadable.push(format!("{STATE_FILE} is not JSON: {error}"));
                return Self::default();
            }
        };
        let mut fields = Fields {
            state: &state,
            unreadable,
        };
        Self {
            repository: fields.string("repository", Presence::Required),
            title: fields.string("title", Presence::Required),
            ended: fields.boolean("ended"),
            latest_round: fields.latest_round(),
            created_at: fields.string("created_at", Presence::Optional),
        }
    }
}

/// Whether every state file of the current format has a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Presence {
    Required,
    /// Absent from topics written before it was kept, which is not a fault.
    Optional,
}

struct Fields<'a> {
    state: &'a Map<String, Value>,
    unreadable: &'a mut Vec<String>,
}

impl<'a> Fields<'a> {
    fn get(&mut self, key: &str, presence: Presence) -> Option<&'a Value> {
        let state = self.state;
        match state.get(key) {
            Some(Value::Null) | None if presence == Presence::Optional => None,
            Some(value) => Some(value),
            None => {
                self.unreadable.push(format!("{STATE_FILE} has no {key}"));
                None
            }
        }
    }

    fn unexpected(&mut self, what: &str, kind: &str) {
        self.unreadable
            .push(format!("{what} in {STATE_FILE} is not {kind}"));
    }

    fn string(&mut self, key: &str, presence: Presence) -> Option<String> {
        let value = self.get(key, presence)?;
        let text = value.as_str().map(str::to_string);
        if text.is_none() {
            self.unexpected(key, "a string");
        }
        text
    }

    fn boolean(&mut self, key: &str) -> Option<bool> {
        let value = self.get(key, Presence::Required)?.as_bool();
        if value.is_none() {
            self.unexpected(key, "true or false");
        }
        value
    }

    /// The last round's number and whether it was sent; `None` when there is no round.
    fn latest_round(&mut self) -> Option<LatestRound> {
        let Some(rounds) = self.get("rounds", Presence::Required)?.as_array() else {
            self.unexpected("rounds", "a list");
            return None;
        };
        let last = rounds.last()?;
        let round = last["number"]
            .as_u64()
            .zip(last["submitted"].as_bool())
            .map(|(number, submitted)| LatestRound { number, submitted });
        if round.is_none() {
            self.unexpected(
                "the latest round",
                "a round with a number and whether it was sent",
            );
        }
        round
    }
}
