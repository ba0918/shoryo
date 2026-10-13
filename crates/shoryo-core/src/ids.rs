//! Identifiers that stay the same for the life of a topic.
//!
//! The agent names what it writes (questions, decisions); the topic numbers what it creates
//! itself (asks, events) from counters that only go up.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A question's identifier, written by the agent.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct QuestionId(String);

/// A decision's identifier, written by the agent.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DecisionId(String);

/// An ask's identifier, assigned by the topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AskId(pub u64);

/// An event's identifier, assigned by the topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(pub u64);

/// The identifier counter has no representable successor. `u64::MAX` is reserved as the
/// exhausted next-counter value so exhaustion survives saving and reloading without ID reuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdExhaustion {
    Ask,
    Event,
}

impl fmt::Display for IdExhaustion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ask => write!(f, "no more ask identifiers can be allocated"),
            Self::Event => write!(f, "no more event identifiers can be allocated"),
        }
    }
}

impl std::error::Error for IdExhaustion {}

impl QuestionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl DecisionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for QuestionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for DecisionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for AskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
