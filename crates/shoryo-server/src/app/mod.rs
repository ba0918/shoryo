//! The HTTP routes: the page, the page API, the live push to the page, and the agent API.
//! Every path sits under the URL secret. Handlers only translate between HTTP and the
//! domain; the rules live in `shoryo-core`.

mod agent;
mod shared;

pub(crate) use shared::Shared;

use std::convert::Infallible;
use std::sync::Arc;

use axum::Router;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{StatusCode, header};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use futures_util::stream::{self, Stream};
use serde::Serialize;
use serde_json::json;
use shoryo_core::{Operation, OperationRefusal};

use crate::clock;
use crate::config::{Config, ConfigChange, ConfigError};

pub fn router(shared: Arc<Shared>, secret: &str) -> Router {
    let base = format!("/s/{secret}");
    Router::new()
        .route(&base, get(Redirect::permanent(&format!("{base}/"))))
        .route(&format!("{base}/"), get(|| async { asset("index.html") }))
        .route(&format!("{base}/api/view"), get(view))
        .route(&format!("{base}/api/events"), get(events))
        .route(&format!("{base}/api/op"), post(operate))
        .route(
            &format!("{base}/api/config"),
            get(config).post(change_config),
        )
        .route(
            &format!("{base}/api/round"),
            post(agent::round).layer(DefaultBodyLimit::max(EXPLANATION_REQUEST_BYTES)),
        )
        .route(&format!("{base}/api/wait"), post(agent::wait))
        .route(
            &format!("{base}/api/reply"),
            post(agent::reply).layer(DefaultBodyLimit::max(EXPLANATION_REQUEST_BYTES)),
        )
        .route(&format!("{base}/api/end"), post(agent::end))
        .route(
            &format!("{base}/api/result"),
            get(agent::result).post(agent::selected_result),
        )
        .route(&format!("{base}/api/stop"), post(agent::stop))
        .route(
            &format!("{base}/{{*file}}"),
            get(|Path(file): Path<String>| async move { asset(&file) }),
        )
        .with_state(shared)
}

// The decoded budget alone is too small for six-byte JSON escapes. Keep transport bounded
// while allowing that expansion plus 1 MiB for structure and unrelated round metadata.
const EXPLANATION_REQUEST_BYTES: usize = 6 * 524_288 + 1_048_576;

fn asset(path: &str) -> Response {
    match shoryo_webview::file(path) {
        Some(file) => ([(header::CONTENT_TYPE, file.content_type)], file.bytes).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// A refusal or failure, returned as `{"error": "..."}` with its status. A refusal of a page
/// operation also carries `refusal`, its kind and details, for the screen to word.
struct ApiError {
    status: StatusCode,
    message: String,
    refusal: Option<serde_json::Value>,
}

impl ApiError {
    fn new(status: StatusCode, error: impl std::fmt::Display) -> Self {
        Self {
            status,
            message: error.to_string(),
            refusal: None,
        }
    }

    fn refused(refusal: &OperationRefusal) -> Self {
        Self {
            refusal: serde_json::to_value(refusal).ok(),
            ..Self::new(StatusCode::CONFLICT, refusal)
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = match self.refusal {
            Some(refusal) => json!({ "error": self.message, "refusal": refusal }),
            None => json!({ "error": self.message }),
        };
        (self.status, axum::Json(body)).into_response()
    }
}

type ApiResult = Result<Response, ApiError>;

fn parse<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, ApiError> {
    serde_json::from_str(body)
        .map_err(|error| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, error))
}

fn ok(body: serde_json::Value) -> ApiResult {
    Ok(axum::Json(body).into_response())
}

async fn view(State(shared): State<Arc<Shared>>) -> Response {
    axum::Json(shared.view()).into_response()
}

async fn events(
    State(shared): State<Arc<Shared>>,
) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    let version = shared.version.subscribe();
    let shutdown = shared.shutdown.subscribe();
    let first = true;
    let stream = stream::unfold(
        (shared, version, shutdown, first),
        |(shared, mut version, mut shutdown, first)| async move {
            if !first {
                tokio::select! {
                    changed = version.changed() => changed.ok()?,
                    _ = shutdown.wait_for(|stop| *stop) => return None,
                }
            }
            version.borrow_and_update();
            let data = serde_json::to_string(&shared.view())
                .expect("the view holds only strings, numbers, booleans and lists of them");
            let event = SseEvent::default().event("view").data(data);
            Some((Ok(event), (shared, version, shutdown, false)))
        },
    );
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn operate(State(shared): State<Arc<Shared>>, body: String) -> ApiResult {
    let operation: Operation = parse(&body)?;
    shared.change_or(
        |state| state.topic.apply(operation, clock::now()),
        |refusal| ApiError::refused(&refusal),
    )?;
    Ok(axum::Json(shared.view()).into_response())
}

/// The language and theme as the page shows them, read from the file each time the page
/// opens; `unreadable` tells the page that switches will not be kept.
#[derive(Serialize)]
struct ConfigView {
    #[serde(flatten)]
    config: Config,
    unreadable: bool,
}

async fn config(State(shared): State<Arc<Shared>>) -> Response {
    let view = match shared.config.read() {
        Ok(config) => ConfigView {
            config,
            unreadable: false,
        },
        Err(_) => ConfigView {
            config: Config::default(),
            unreadable: true,
        },
    };
    axum::Json(view).into_response()
}

async fn change_config(State(shared): State<Arc<Shared>>, body: String) -> ApiResult {
    let change: ConfigChange = parse(&body)?;
    let config = shared.config.change(change).map_err(|error| {
        let status = match error {
            ConfigError::NoConfigDirectory | ConfigError::Unreadable { .. } => StatusCode::CONFLICT,
            ConfigError::Write { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        };
        ApiError::new(status, error)
    })?;
    Ok(axum::Json(ConfigView {
        config,
        unreadable: false,
    })
    .into_response())
}
