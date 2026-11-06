//! The values `ps` takes from a running server's endpoint file (`docs/spec/ps.md`,
//! "読めない議題").

use std::path::Path;

use shoryo_core::Timestamp;

use super::fields::{Fields, Presence, read_object};
use crate::location::ENDPOINT_FILE;

#[derive(Debug, Default)]
pub(super) struct EndpointValues {
    pub(super) url: Option<String>,
    pub(super) started_at: Option<Timestamp>,
    pub(super) version: Option<String>,
    pub(super) repository: Option<String>,
}

impl EndpointValues {
    /// The values of the endpoint file at `path`, all `None` when there is none yet. Each value
    /// is read on its own, and one an older server did not write is not a fault; why one could
    /// not be read is added to `unreadable`.
    pub(super) fn read(path: &Path, unreadable: &mut Vec<String>) -> Self {
        let Some(endpoint) = read_object(path, ENDPOINT_FILE, unreadable) else {
            return Self::default();
        };
        let mut fields = Fields::new(ENDPOINT_FILE, &endpoint, unreadable);
        Self {
            url: fields.string("url", Presence::Optional),
            started_at: fields
                .string("started_at", Presence::Optional)
                .map(Timestamp::new),
            version: fields.string("version", Presence::Optional),
            repository: fields.string("repository", Presence::Optional),
        }
    }
}
