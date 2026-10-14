use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;
use shoryo_core::{AskId, EventId, Reply, RoundInput};

use super::{ApiError, ApiResult, Shared, ok, parse};

pub(super) async fn round(State(shared): State<Arc<Shared>>, body: String) -> ApiResult {
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
    timeout: Option<u64>,
}

pub(super) async fn wait(State(shared): State<Arc<Shared>>, body: String) -> ApiResult {
    let request: WaitRequest = parse(&body)?;
    let deadline = match request.timeout {
        Some(seconds) => Some(
            tokio::time::Instant::now()
                .checked_add(Duration::from_secs(seconds))
                .ok_or_else(|| {
                    ApiError::new(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "the wait timeout is too large to represent",
                    )
                })?,
        ),
        None => None,
    };
    let _in_progress = shared.wait_started();
    if !request.ack.is_empty() {
        shared.change(|state| {
            state.topic.check_wait()?;
            state.topic.acknowledge(&request.ack);
            Ok::<(), shoryo_core::OperationRefusal>(())
        })?;
    }
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

pub(super) async fn reply(State(shared): State<Arc<Shared>>, body: String) -> ApiResult {
    shared.heard_from_agent();
    let request: ReplyRequest = parse(&body)?;
    let reply = Reply {
        parts: request.parts,
    };
    shared.change(|state| state.topic.reply(request.ask, reply))?;
    ok(json!({}))
}

pub(super) async fn end(State(shared): State<Arc<Shared>>) -> ApiResult {
    shared.heard_from_agent();
    shared.change(|state| state.topic.end())?;
    ok(json!({}))
}

pub(super) async fn result(State(shared): State<Arc<Shared>>) -> Response {
    let body = shared.lock().to_json();
    ([(header::CONTENT_TYPE, "application/json")], body).into_response()
}

pub(super) async fn selected_result(State(shared): State<Arc<Shared>>, body: String) -> ApiResult {
    let selection = parse::<shoryo_core::ResultSelection>(&body)?;
    let body = shared
        .lock()
        .topic
        .selected_result(&selection)
        .map_err(|error| {
            let status = match &error {
                shoryo_core::ResultReadError::InvalidAnswer(_) => StatusCode::INTERNAL_SERVER_ERROR,
                shoryo_core::ResultReadError::RoundNotFound(_)
                | shoryo_core::ResultReadError::DecisionNotFound(_)
                | shoryo_core::ResultReadError::QuestionNotFound(_)
                | shoryo_core::ResultReadError::AskNotFound(_) => StatusCode::NOT_FOUND,
            };
            ApiError::new(status, error)
        })?;
    Ok(([(header::CONTENT_TYPE, "application/json")], body).into_response())
}

pub(super) async fn stop(State(shared): State<Arc<Shared>>) -> ApiResult {
    shared.shutdown.send_replace(true);
    ok(json!({}))
}
