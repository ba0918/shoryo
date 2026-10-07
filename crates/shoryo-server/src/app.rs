//! The HTTP routes: the page, the page API, the live push to the page, and the agent API.
//! Every path sits under the URL secret. Handlers only translate between HTTP and the
//! domain; the rules live in `shoryo-core`.

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use futures_util::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shoryo_core::{
    AskId, ChainLink, DecisionId, EventId, Map, Operation, QuestionId, Reply, RoundInput,
};
use tokio::sync::watch;

use crate::config::{Config, ConfigChange, ConfigError, ConfigFile};
use crate::location::TopicLocation;
use crate::store::{self, StoredTopic};

/// What every handler shares.
pub struct Shared {
    state: Mutex<StoredTopic>,
    location: TopicLocation,
    config: ConfigFile,
    url: String,
    /// Increases on every change; the page stream and `wait` watch it.
    version: watch::Sender<u64>,
    /// Becomes true when the server is to stop.
    pub shutdown: watch::Sender<bool>,
}

impl Shared {
    pub fn new(
        state: StoredTopic,
        location: TopicLocation,
        config: ConfigFile,
        url: String,
    ) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(state),
            location,
            config,
            url,
            version: watch::channel(0).0,
            shutdown: watch::channel(false).0,
        })
    }

    fn lock(&self) -> MutexGuard<'_, StoredTopic> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Runs a change on a copy of the state; when it succeeds and the copy is saved, the copy
    /// becomes the state and the watchers are told. Otherwise the state stays as it was, so it
    /// never differs from the file.
    fn change<T, E: std::fmt::Display>(
        &self,
        change: impl FnOnce(&mut StoredTopic) -> Result<T, E>,
    ) -> Result<T, ApiError> {
        let mut state = self.lock();
        let mut changed = state.clone();
        let value =
            change(&mut changed).map_err(|error| ApiError::new(StatusCode::CONFLICT, error))?;
        store::save(&self.location, &changed)
            .map_err(|error| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, error))?;
        *state = changed;
        drop(state);
        self.version.send_modify(|version| *version += 1);
        Ok(value)
    }

    fn view(&self) -> View {
        let state = self.lock();
        let topic = &state.topic;
        let chains = topic
            .rounds
            .iter()
            .flat_map(|round| &round.questions)
            .map(|question| (question.id.clone(), topic.chains(&question.id)))
            .collect();
        let pre_approved = topic
            .records
            .decisions
            .iter()
            .filter(|decision| topic.decided_pre_approved(&decision.id))
            .map(|decision| decision.id.clone())
            .collect();
        let map = topic.map();
        let paths = map
            .nodes
            .iter()
            .map(|node| (node.key.clone(), map.path(&node.key)))
            .collect();
        View {
            version: *self.version.borrow(),
            map,
            paths,
            chains,
            unstamped: topic.unstamped(),
            pre_approved,
            topic: state.clone(),
        }
    }
}

/// Everything the page draws, sent whole on every change.
#[derive(Serialize)]
struct View {
    version: u64,
    topic: StoredTopic,
    chains: BTreeMap<QuestionId, Vec<Vec<ChainLink>>>,
    map: Map,
    /// 道筋 for each map node: the keys shown when that node is selected.
    paths: BTreeMap<String, Vec<String>>,
    /// The current round's questions that have no stamp yet.
    unstamped: Vec<QuestionId>,
    /// The decisions that carry the 代決 mark.
    pre_approved: Vec<DecisionId>,
}

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
        .route(&format!("{base}/api/round"), post(round))
        .route(&format!("{base}/api/wait"), post(wait))
        .route(&format!("{base}/api/reply"), post(reply))
        .route(&format!("{base}/api/end"), post(end))
        .route(&format!("{base}/api/result"), get(result))
        .route(&format!("{base}/api/stop"), post(stop))
        .route(
            &format!("{base}/{{*file}}"),
            get(|Path(file): Path<String>| async move { asset(&file) }),
        )
        .with_state(shared)
}

fn asset(path: &str) -> Response {
    match shoryo_webview::file(path) {
        Some(file) => ([(header::CONTENT_TYPE, file.content_type)], file.bytes).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// A refusal or failure, returned as `{"error": "..."}` with its status.
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, error: impl std::fmt::Display) -> Self {
        Self {
            status,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, axum::Json(json!({ "error": self.message }))).into_response()
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
    shared.change(|state| state.topic.apply(operation))?;
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

async fn round(State(shared): State<Arc<Shared>>, body: String) -> ApiResult {
    let input = RoundInput::from_json(&body)
        .map_err(|error| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, error))?;
    let number = shared.change(|state| state.topic.apply_round(input))?;
    ok(json!({ "round": number, "url": shared.url }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WaitRequest {
    #[serde(default)]
    ack: Vec<EventId>,
    /// Seconds to wait at most; without it, wait until something happens.
    timeout: Option<u64>,
}

async fn wait(State(shared): State<Arc<Shared>>, body: String) -> ApiResult {
    let request: WaitRequest = parse(&body)?;
    if !request.ack.is_empty() {
        shared.change(|state| {
            state.topic.check_wait()?;
            state.topic.acknowledge(&request.ack);
            Ok::<(), shoryo_core::OperationRefusal>(())
        })?;
    }
    let deadline = request
        .timeout
        .map(|seconds| tokio::time::Instant::now() + Duration::from_secs(seconds));
    let mut version = shared.version.subscribe();
    let mut shutdown = shared.shutdown.subscribe();
    loop {
        version.borrow_and_update();
        let pending = {
            let state = shared.lock();
            state
                .topic
                .check_wait()
                .map_err(|error| ApiError::new(StatusCode::CONFLICT, error))?;
            state.topic.pending_events().to_vec()
        };
        if !pending.is_empty() {
            return ok(json!({ "events": pending }));
        }
        let timed_out = async {
            match deadline {
                Some(deadline) => tokio::time::sleep_until(deadline).await,
                None => std::future::pending().await,
            }
        };
        tokio::select! {
            _ = version.changed() => {}
            () = timed_out => return ok(json!({ "events": [] })),
            _ = shutdown.wait_for(|stop| *stop) => {
                return Err(ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "the server is stopping"));
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplyRequest {
    ask: AskId,
    text: String,
    diagram: Option<String>,
}

async fn reply(State(shared): State<Arc<Shared>>, body: String) -> ApiResult {
    let request: ReplyRequest = parse(&body)?;
    let reply = Reply {
        text: request.text,
        diagram: request.diagram,
    };
    shared.change(|state| state.topic.reply(request.ask, reply))?;
    ok(json!({}))
}

async fn end(State(shared): State<Arc<Shared>>) -> ApiResult {
    shared.change(|state| state.topic.end())?;
    ok(json!({}))
}

async fn result(State(shared): State<Arc<Shared>>) -> Response {
    let body = shared.lock().to_json();
    ([(header::CONTENT_TYPE, "application/json")], body).into_response()
}

async fn stop(State(shared): State<Arc<Shared>>) -> ApiResult {
    shared.shutdown.send_replace(true);
    ok(json!({}))
}
