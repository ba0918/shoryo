//! The HTTP server of shoryo: the page API, the agent API and the topic's data directory.
#![deny(clippy::print_stdout, clippy::print_stderr)]

mod app;
mod clock;
pub mod config;
pub mod location;
pub mod store;

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use config::ConfigFile;
use location::TopicLocation;
use store::{Endpoint, EndpointFile, Lock};

#[derive(Debug)]
pub enum ServerError {
    InvalidTopicName(String),
    NoDataDirectory,
    AlreadyRunning {
        topic: String,
        pid: u32,
    },
    /// The lock exists but names no process, so it cannot be told stale.
    UnreadableLock {
        topic: String,
        path: PathBuf,
    },
    CorruptState {
        path: PathBuf,
        reason: String,
    },
    Io {
        path: PathBuf,
        error: std::io::Error,
    },
    Bind {
        address: SocketAddr,
        error: std::io::Error,
    },
    Runtime(std::io::Error),
}

impl ServerError {
    pub(crate) fn io(path: impl AsRef<Path>, error: std::io::Error) -> Self {
        Self::Io {
            path: path.as_ref().to_path_buf(),
            error,
        }
    }
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTopicName(name) => write!(
                f,
                "{name:?} is not a topic name: use letters, digits, '.', '_' and '-'"
            ),
            Self::NoDataDirectory => write!(f, "the per-user data directory cannot be found"),
            Self::AlreadyRunning { topic, pid } => write!(
                f,
                "topic {topic} is already running (process {pid}); use that server"
            ),
            Self::UnreadableLock { topic, path } => write!(
                f,
                "topic {topic} has a lock that names no process ({}); if no server runs for \
                 it, remove that file",
                path.display()
            ),
            Self::CorruptState { path, reason } => {
                write!(f, "{} cannot be read: {reason}", path.display())
            }
            Self::Io { path, error } => write!(f, "{}: {error}", path.display()),
            Self::Bind { address, error } => write!(f, "cannot listen on {address}: {error}"),
            Self::Runtime(error) => write!(f, "the server could not run: {error}"),
        }
    }
}

impl std::error::Error for ServerError {}

pub struct ServeOptions {
    pub location: TopicLocation,
    pub bind: IpAddr,
    /// `None` takes a free port.
    pub port: Option<u16>,
}

/// What the binary prints once the server listens.
pub struct Started {
    pub url: String,
    /// What the person should know: the page reachable from other machines over plain HTTP,
    /// or a config file that cannot be read.
    pub warnings: Vec<String>,
}

/// Runs the topic's server until it is told to stop, a stop command or SIGINT or SIGTERM.
pub fn serve(options: ServeOptions, on_started: impl FnOnce(&Started)) -> Result<(), ServerError> {
    let lock = Lock::take(&options.location)?;
    let state = store::load(&options.location)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(ServerError::Runtime)?;
    runtime.block_on(async move {
        let address = SocketAddr::new(options.bind, options.port.unwrap_or(0));
        let listener = tokio::net::TcpListener::bind(address)
            .await
            .map_err(|error| ServerError::Bind { address, error })?;
        let port = listener
            .local_addr()
            .map_err(|error| ServerError::Bind { address, error })?
            .port();
        let secret = secret()?;
        let host = if options.bind.is_unspecified() {
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        } else {
            options.bind
        };
        let url = format!("http://{}/s/{secret}/", SocketAddr::new(host, port));
        let endpoint = EndpointFile::write(
            &options.location,
            &Endpoint {
                host,
                port,
                secret: secret.clone(),
                url: url.clone(),
            },
        )?;
        let config = ConfigFile::per_user();
        let config_warning = config.read().err().map(|error| error.to_string());
        let bind_warning = (!options.bind.is_loopback()).then(|| {
            format!(
                "listening on {}: the page is served over plain HTTP, readable by anyone on \
                 the network path; anyone who has the URL can answer for you",
                options.bind
            )
        });

        let warnings = bind_warning.into_iter().chain(config_warning).collect();

        let shared = app::Shared::new(state, options.location, config, url.clone());
        let mut stopping = shared.shutdown.subscribe();
        let signals = shared.shutdown.clone();
        tokio::spawn(async move {
            stop_on_signal().await;
            signals.send_replace(true);
        });
        on_started(&Started { url, warnings });

        axum::serve(listener, app::router(shared, &secret))
            .with_graceful_shutdown(async move {
                let _ = stopping.wait_for(|stop| *stop).await;
            })
            .await
            .map_err(ServerError::Runtime)?;
        drop(endpoint);
        drop(lock);
        Ok(())
    })
}

async fn stop_on_signal() {
    use tokio::signal::unix::{SignalKind, signal};
    let (Ok(mut interrupt), Ok(mut terminate)) = (
        signal(SignalKind::interrupt()),
        signal(SignalKind::terminate()),
    ) else {
        return std::future::pending().await;
    };
    tokio::select! {
        _ = interrupt.recv() => {}
        _ = terminate.recv() => {}
    }
}

/// 16 random bytes as hex, so the page and the API cannot be found without the URL.
fn secret() -> Result<String, ServerError> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        ServerError::Runtime(std::io::Error::other(format!(
            "no random source for the URL secret: {error}"
        )))
    })?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}
