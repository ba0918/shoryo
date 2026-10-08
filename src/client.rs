//! The agent's side of the agent API: a short-lived client to the topic's running server.

use serde_json::Value;
use shoryo_server::location::TopicLocation;
use shoryo_server::store::{self, Endpoint};

pub struct Client {
    agent: ureq::Agent,
    endpoint: Endpoint,
}

/// Finds the topic's running server.
pub fn connect(location: &TopicLocation) -> Result<Client, String> {
    let endpoint = store::read_endpoint(location)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            format!(
                "no server is running for topic {name}; start it with `shoryo start {name}`",
                name = location.name
            )
        })?;
    Ok(Client {
        agent: agent(),
        endpoint,
    })
}

// The server is on this machine, so no proxy applies; and waiting has no time limit unless
// the agent gives one, so the client sets none either.
fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .proxy(None)
        .build()
        .into()
}

impl Client {
    /// Posts `body` to the agent API and returns the JSON answer, or the server's reason.
    pub fn post(&self, path: &str, body: String) -> Result<Value, String> {
        let mut response = self
            .agent
            .post(format!("{}{path}", self.endpoint.api_base()))
            .send(body)
            .map_err(|error| format!("the server did not answer: {error}"))?;
        let status = response.status();
        let answer: Value = response
            .body_mut()
            .read_json()
            .map_err(|error| format!("the server's answer is not JSON: {error}"))?;
        if status.is_success() {
            Ok(answer)
        } else {
            Err(answer["error"]
                .as_str()
                .map_or_else(|| answer.to_string(), str::to_string))
        }
    }

    fn get_text(&self, path: &str) -> Result<String, String> {
        self.agent
            .get(format!("{}{path}", self.endpoint.api_base()))
            .call()
            .map_err(|error| format!("the server did not answer: {error}"))?
            .body_mut()
            .read_to_string()
            .map_err(|error| format!("the server's answer cannot be read: {error}"))
    }
}

/// The topic's whole data: from the running server, or from the file when none runs.
pub fn result(location: &TopicLocation) -> Result<String, String> {
    if let Some(endpoint) = store::read_endpoint(location).map_err(|error| error.to_string())? {
        return Client {
            agent: agent(),
            endpoint,
        }
        .get_text("result");
    }
    store::read_state(location)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("topic {} has no data yet", location.name))
}
