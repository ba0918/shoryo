//! The endpoint file the agent's commands use to find the running server.

use std::fs;
use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::lock::running_holder;
use crate::ServerError;
use crate::location::TopicLocation;

/// How a command reaches the running server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    /// The address the commands connect to: the bound address, or loopback when the server
    /// listens on every address.
    pub host: IpAddr,
    pub port: u16,
    pub secret: String,
    /// The page's URL, as printed at start-up.
    pub url: String,
}

impl Endpoint {
    /// The base of the agent API.
    pub fn api_base(&self) -> String {
        format!(
            "http://{}/s/{}/api/",
            SocketAddr::new(self.host, self.port),
            self.secret
        )
    }
}

/// Removed when the server stops.
pub struct EndpointFile {
    path: PathBuf,
}

impl EndpointFile {
    /// Writes the endpoint, readable only by the user, replacing any file left behind.
    pub fn write(location: &TopicLocation, endpoint: &Endpoint) -> Result<Self, ServerError> {
        let path = location.endpoint_file();
        let json = serde_json::to_string(endpoint)
            .expect("the endpoint holds only an address, a number and strings");
        crate::files::atomic_write(&path, json.as_bytes())
            .map_err(|error| ServerError::io(&path, error))?;
        Ok(Self { path })
    }
}

impl Drop for EndpointFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// The endpoint of the topic's running server, or `None` when no server runs.
pub fn read_endpoint(location: &TopicLocation) -> Result<Option<Endpoint>, ServerError> {
    let path = location.endpoint_file();
    if running_holder(&location.lock_file()).is_none() {
        return Ok(None);
    }
    match fs::read_to_string(&path) {
        Ok(json) => {
            serde_json::from_str(&json)
                .map(Some)
                .map_err(|error| ServerError::CorruptState {
                    path,
                    reason: error.to_string(),
                })
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ServerError::io(&path, error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::location_in;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn replacing_an_endpoint_does_not_inherit_readable_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let location = location_in(dir.path());
        fs::write(location.endpoint_file(), "old endpoint").unwrap();
        fs::set_permissions(location.endpoint_file(), fs::Permissions::from_mode(0o644)).unwrap();
        let endpoint = Endpoint {
            host: "127.0.0.1".parse().unwrap(),
            port: 1234,
            secret: "test-secret".into(),
            url: "http://example.invalid/".into(),
        };
        let _published = EndpointFile::write(&location, &endpoint).unwrap();
        let mode = fs::metadata(location.endpoint_file())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        let stored: Endpoint =
            serde_json::from_str(&fs::read_to_string(location.endpoint_file()).unwrap()).unwrap();
        assert_eq!(stored.secret, endpoint.secret);
    }
}
