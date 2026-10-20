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
    running(location)?.ok_or_else(|| {
        format!(
            "no server is running for topic {name}; start it with `shoryo start {name}`",
            name = location.name
        )
    })
}

/// A client to the topic's server, or `None` when no server runs.
fn running(location: &TopicLocation) -> Result<Option<Client>, String> {
    let endpoint = store::read_endpoint(location).map_err(|error| error.to_string())?;
    Ok(endpoint.map(|endpoint| Client {
        agent: agent(),
        endpoint,
    }))
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
        // Histories and unacknowledged event batches can exceed the default 10 MiB limit.
        let answer: Value = response
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_json()
            .map_err(|error| format!("the server's answer is not JSON: {error}"))?;
        if status.is_success() {
            Ok(answer)
        } else {
            Err(server_error(&answer))
        }
    }

    fn get_text(&self, path: &str) -> Result<String, String> {
        let mut response = self
            .agent
            .get(format!("{}{path}", self.endpoint.api_base()))
            .call()
            .map_err(|error| format!("the server did not answer: {error}"))?;
        let status = response.status();
        let body = response
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_to_string()
            .map_err(|error| format!("the server's answer cannot be read: {error}"))?;
        if status.is_success() {
            Ok(body)
        } else {
            Err(serde_json::from_str::<Value>(&body)
                .map(|answer| server_error(&answer))
                .unwrap_or_else(|_| {
                    format!("the server returned HTTP {}: {body}", status.as_u16())
                }))
        }
    }
}

fn server_error(answer: &Value) -> String {
    answer["error"]
        .as_str()
        .map_or_else(|| answer.to_string(), str::to_string)
}

/// The topic's whole data: from the running server, or from the file when none runs.
pub fn result(location: &TopicLocation) -> Result<String, String> {
    if let Some(client) = running(location)? {
        return client.get_text("result");
    }
    store::read_state(location)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("topic {} has no data yet", location.name))
}

pub fn selected_result(
    location: &TopicLocation,
    selection: &shoryo_server::ResultSelection,
) -> Result<String, String> {
    if let Some(client) = running(location)? {
        let request = serde_json::to_string(selection)
            .expect("result selections contain only strings and a round number");
        return client
            .post("result", request)
            .map(|value| value.to_string());
    }
    store::load_existing(location)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("topic {} has no data yet", location.name))?
        .topic
        .selected_result(selection)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn responding(status: &str, body: &str) -> (Client, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a test listener can bind");
        let address = listener.local_addr().expect("the listener has an address");
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the client connects");
            let mut request = [0; 1024];
            assert!(stream.read(&mut request).expect("the request arrives") > 0);
            stream
                .write_all(response.as_bytes())
                .expect("the response is written");
        });
        let client = Client {
            agent: agent(),
            endpoint: Endpoint {
                host: address.ip(),
                port: address.port(),
                secret: "test-secret".into(),
                url: "http://example.invalid/".into(),
            },
        };
        (client, server)
    }

    #[test]
    fn failed_result_requests_return_the_servers_reason_instead_of_successful_data() {
        let (client, server) = responding(
            "500 Internal Server Error",
            r#"{"error":"The state cannot be read"}"#,
        );
        assert_eq!(
            client.get_text("result"),
            Err("The state cannot be read".into())
        );
        server.join().unwrap();
    }

    #[test]
    fn successful_results_preserve_the_exact_response_text() {
        let body = "{\n  \"title\": \"Keep whitespace\"\n}\n";
        let (client, server) = responding("200 OK", body);
        assert_eq!(client.get_text("result").unwrap(), body);
        server.join().unwrap();
    }
}
