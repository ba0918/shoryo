//! The HTTP routes: the page, the page API, the live push to the page, and the agent API.
//! Every path sits under the URL secret. Handlers only translate between HTTP and the
//! domain; the rules live in `shoryo-core`.

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{StatusCode, header};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use futures_util::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shoryo_core::{
    AskId, ChainLink, DecisionId, EventId, Map, Operation, OperationRefusal, QuestionId, Reply,
    RoundInput,
};
use tokio::sync::watch;

use crate::clock;
use crate::config::{Config, ConfigChange, ConfigError, ConfigFile};
use crate::location::TopicLocation;
use crate::store::{self, StoredTopic};

/// What every handler shares.
pub struct Shared {
    state: Mutex<StoredTopic>,
    location: TopicLocation,
    config: ConfigFile,
    url: String,
    /// What the page shows of the agent; not part of the topic, so never saved.
    agent: Mutex<AgentActivity>,
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
            agent: Mutex::new(AgentActivity {
                waits: 0,
                heard: Instant::now(),
            }),
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
        self.change_or(change, |error| ApiError::new(StatusCode::CONFLICT, error))
    }

    /// `change`, with `refuse` turning a refusal into the response.
    fn change_or<T, E>(
        &self,
        change: impl FnOnce(&mut StoredTopic) -> Result<T, E>,
        refuse: impl FnOnce(E) -> ApiError,
    ) -> Result<T, ApiError> {
        let mut state = self.lock();
        let mut changed = state.clone();
        let value = change(&mut changed).map_err(refuse)?;
        store::save(&self.location, &changed)
            .map_err(|error| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, error))?;
        *state = changed;
        drop(state);
        self.version.send_modify(|version| *version += 1);
        Ok(value)
    }

    fn agent(&self) -> MutexGuard<'_, AgentActivity> {
        self.agent
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Notes that the agent sent a command, and tells the page.
    fn heard_from_agent(&self) {
        self.agent().heard = Instant::now();
        self.version.send_modify(|version| *version += 1);
    }

    /// Marks a `wait` in progress until the returned guard is dropped, however it ends.
    fn wait_started(self: &Arc<Self>) -> WaitInProgress {
        self.agent().waits += 1;
        self.heard_from_agent();
        WaitInProgress(Arc::clone(self))
    }

    fn view(&self) -> View {
        let agent = self.agent().view();
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
            agent,
            map,
            paths,
            chains,
            unstamped: topic.unstamped(),
            pre_approved,
            topic: state.clone(),
        }
    }
}

/// Whether the agent is waiting for the screen, and when it last sent a command.
struct AgentActivity {
    waits: usize,
    heard: Instant,
}

impl AgentActivity {
    fn view(&self) -> AgentView {
        AgentView {
            waiting: self.waits > 0,
            quiet_ms: u64::try_from(self.heard.elapsed().as_millis()).unwrap_or(u64::MAX),
        }
    }
}

/// The agent as the page shows it; the page decides from `quiet_ms` when it stopped responding.
#[derive(Serialize)]
struct AgentView {
    waiting: bool,
    /// Milliseconds since the agent's last command, when the view was made.
    quiet_ms: u64,
}

struct WaitInProgress(Arc<Shared>);

impl Drop for WaitInProgress {
    fn drop(&mut self) {
        self.0.agent().waits -= 1;
        self.0.heard_from_agent();
    }
}

/// Everything the page draws, sent whole on every change.
#[derive(Serialize)]
struct View {
    version: u64,
    agent: AgentView,
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
        .route(
            &format!("{base}/api/round"),
            post(round).layer(DefaultBodyLimit::max(EXPLANATION_REQUEST_BYTES)),
        )
        .route(&format!("{base}/api/wait"), post(wait))
        .route(
            &format!("{base}/api/reply"),
            post(reply).layer(DefaultBodyLimit::max(EXPLANATION_REQUEST_BYTES)),
        )
        .route(&format!("{base}/api/end"), post(end))
        .route(&format!("{base}/api/result"), get(result))
        .route(&format!("{base}/api/stop"), post(stop))
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

async fn round(State(shared): State<Arc<Shared>>, body: String) -> ApiResult {
    shared.heard_from_agent();
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
    let _in_progress = shared.wait_started();
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
    parts: Vec<shoryo_core::Part>,
}

async fn reply(State(shared): State<Arc<Shared>>, body: String) -> ApiResult {
    shared.heard_from_agent();
    let request: ReplyRequest = parse(&body)?;
    let reply = Reply {
        parts: request.parts,
    };
    shared.change(|state| state.topic.reply(request.ask, reply))?;
    ok(json!({}))
}

async fn end(State(shared): State<Arc<Shared>>) -> ApiResult {
    shared.heard_from_agent();
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
