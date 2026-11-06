//! Reading a topic's JSON file value by value, without its stored types, so that a file that is
//! partly unexpected still yields what it has (`docs/spec/ps.md`, "読めない議題").

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde_json::{Map, Value};

/// The JSON object in the file at `path`, named `file` in warnings; `None` when the file does
/// not exist, and also when it cannot be read or is not an object, which is added to
/// `unreadable`.
pub(super) fn read_object(
    path: &Path,
    file: &str,
    unreadable: &mut Vec<String>,
) -> Option<Map<String, Value>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return None,
        Err(error) => {
            unreadable.push(format!("{file} cannot be read: {error}"));
            return None;
        }
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(object)) => Some(object),
        Ok(_) => {
            unreadable.push(format!("{file} is not a JSON object"));
            None
        }
        Err(error) => {
            unreadable.push(format!("{file} is not JSON: {error}"));
            None
        }
    }
}

/// Whether every file of the current format has a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Presence {
    Required,
    /// Absent from files written before it was kept, which is not a fault.
    Optional,
}

/// The values of one file's object; why one could not be read is added to `unreadable`.
pub(super) struct Fields<'a> {
    file: &'a str,
    object: &'a Map<String, Value>,
    unreadable: &'a mut Vec<String>,
}

impl<'a> Fields<'a> {
    pub(super) fn new(
        file: &'a str,
        object: &'a Map<String, Value>,
        unreadable: &'a mut Vec<String>,
    ) -> Self {
        Self {
            file,
            object,
            unreadable,
        }
    }

    pub(super) fn get(&mut self, key: &str, presence: Presence) -> Option<&'a Value> {
        let object = self.object;
        match object.get(key) {
            Some(Value::Null) | None if presence == Presence::Optional => None,
            Some(value) => Some(value),
            None => {
                self.unreadable.push(format!("{} has no {key}", self.file));
                None
            }
        }
    }

    pub(super) fn unexpected(&mut self, what: &str, kind: &str) {
        self.unreadable
            .push(format!("{what} in {} is not {kind}", self.file));
    }

    pub(super) fn string(&mut self, key: &str, presence: Presence) -> Option<String> {
        let value = self.get(key, presence)?;
        let text = value.as_str().map(str::to_string);
        if text.is_none() {
            self.unexpected(key, "a string");
        }
        text
    }

    pub(super) fn boolean(&mut self, key: &str) -> Option<bool> {
        let value = self.get(key, Presence::Required)?.as_bool();
        if value.is_none() {
            self.unexpected(key, "true or false");
        }
        value
    }
}
