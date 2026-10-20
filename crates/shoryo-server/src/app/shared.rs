use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use axum::http::StatusCode;
use serde::Serialize;
use shoryo_core::{ChainLink, DecisionId, Map, QuestionId, ScreenView, SentAnswer};
use tokio::sync::watch;

use super::ApiError;
use crate::config::ConfigFile;
use crate::location::TopicLocation;
use crate::store::{self, StoredTopic};

pub(crate) struct Shared {
    state: Mutex<StoredTopic>,
    location: TopicLocation,
    pub(super) config: ConfigFile,
    pub(super) url: String,
    agent: Mutex<AgentActivity>,
    pub(super) version: watch::Sender<u64>,
    pub(crate) shutdown: watch::Sender<bool>,
}

impl Shared {
    pub(crate) fn new(
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

    pub(super) fn lock(&self) -> MutexGuard<'_, StoredTopic> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Runs a change on a copy; it becomes the state only after the copy is saved.
    pub(super) fn change<T, E: std::fmt::Display>(
        &self,
        change: impl FnOnce(&mut StoredTopic) -> Result<T, E>,
    ) -> Result<T, ApiError> {
        self.change_or(change, |error| ApiError::new(StatusCode::CONFLICT, error))
    }

    pub(super) fn change_or<T, E>(
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

    pub(super) fn heard_from_agent(&self) {
        self.agent().heard = Instant::now();
        self.version.send_modify(|version| *version += 1);
    }

    pub(super) fn wait_started(self: &Arc<Self>) -> WaitInProgress {
        self.agent().waits += 1;
        self.heard_from_agent();
        WaitInProgress(Arc::clone(self))
    }

    pub(super) fn view(&self) -> View {
        let agent = self.agent().view();
        let state = self.lock();
        let ScreenView {
            chains,
            map,
            paths,
            unstamped,
            pre_approved,
            expected_answers,
            sent_answers,
        } = state.topic.screen_view();
        View {
            version: *self.version.borrow(),
            agent,
            map,
            paths,
            chains,
            unstamped,
            pre_approved,
            expected_answers,
            sent_answers,
            topic: state.clone(),
        }
    }
}

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

#[derive(Serialize)]
struct AgentView {
    waiting: bool,
    quiet_ms: u64,
}

pub(super) struct WaitInProgress(Arc<Shared>);

impl Drop for WaitInProgress {
    fn drop(&mut self) {
        self.0.agent().waits -= 1;
        self.0.heard_from_agent();
    }
}

#[derive(Serialize)]
pub(super) struct View {
    expected_answers: Option<Vec<SentAnswer>>,
    sent_answers: BTreeMap<u32, Vec<SentAnswer>>,
    version: u64,
    agent: AgentView,
    topic: StoredTopic,
    chains: BTreeMap<QuestionId, Vec<Vec<ChainLink>>>,
    map: Map,
    paths: BTreeMap<String, Vec<String>>,
    unstamped: Vec<QuestionId>,
    pre_approved: Vec<DecisionId>,
}
