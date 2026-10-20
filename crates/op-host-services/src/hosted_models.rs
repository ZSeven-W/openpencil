//! Runtime-only Hosted identity, catalog and request routing. No saved provider configuration.

use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use op_ai::chat_provider::ChatProvider;
use op_auth_bridge::{HostedAccess, HostedAccessPoll, HostedAccessRequestId};
use op_editor_core::{EditorState, HostedModelRoute, HostedProtocol};

mod catalog;
mod error;
mod provider;
pub(crate) mod request;
mod stream;
mod stream_protocol;
pub use error::HostedRuntimeError;

#[derive(Clone, PartialEq, Eq)]
struct Session {
    device_id: String,
    auth_epoch: u64,
}

struct State {
    session: Option<Session>,
    epoch: u64,
    namespace: String,
    subject: Option<String>,
    routes: Vec<HostedModelRoute>,
    status: &'static str,
    job: Option<tokio::task::JoinHandle<()>>,
    refresh_at: Instant,
}

impl Default for State {
    fn default() -> Self {
        Self {
            session: None,
            epoch: 0,
            namespace: String::new(),
            subject: None,
            routes: Vec::new(),
            status: "signed_out",
            job: None,
            refresh_at: Instant::now(),
        }
    }
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| Mutex::new(State::default()));

fn current_session() -> Option<Session> {
    match op_auth_bridge::poll(op_auth_bridge::SESSION_HANDLE) {
        op_auth_bridge::AuthStatus::SignedIn { device_id, .. } if !device_id.is_empty() => {
            Some(Session {
                device_id,
                auth_epoch: op_auth_bridge::hosted_access_provider().session_epoch(),
            })
        }
        _ => None,
    }
}

fn reset(state: &mut State, session: Option<Session>) {
    if let Some(job) = state.job.take() {
        job.abort();
    }
    state.epoch = state.epoch.wrapping_add(1);
    state.session = session;
    state.subject = None;
    state.routes.clear();
    state.status = if state.session.is_some() {
        "loading"
    } else {
        "signed_out"
    };
    state.refresh_at = Instant::now();
    let mut nonce = [0_u8; 32];
    state.namespace = if getrandom::fill(&mut nonce).is_ok() {
        nonce.iter().map(|byte| format!("{byte:02x}")).collect()
    } else {
        state.status = "failed";
        state.refresh_at = Instant::now() + Duration::from_secs(30);
        String::new()
    };
}

/// Cancel jobs and make every previously constructed provider stale.
pub fn clear() {
    let mut state = STATE.lock().unwrap_or_else(|p| p.into_inner());
    reset(&mut state, None);
}

/// Keep discovery and identity checks scheduled while a Hosted session exists.
pub fn needs_poll() -> bool {
    let state = STATE.lock().unwrap_or_else(|p| p.into_inner());
    state.session.is_some() || state.job.is_some()
}

pub fn active() -> bool {
    needs_poll()
}

/// Reject buffered output after logout or account replacement.
pub fn session_epoch_is_current(epoch: Option<u64>) -> bool {
    epoch.is_none_or(|epoch| {
        epoch != 0 && current_session().is_some_and(|session| session.auth_epoch == epoch)
    })
}

fn refresh() {
    let session = current_session();
    let mut state = STATE.lock().unwrap_or_else(|p| p.into_inner());
    if state.session != session {
        reset(&mut state, session.clone());
        tracing::info!(
            signed_in = session.is_some(),
            "Hosted model session updated"
        );
    }
    if session.is_none() {
        return;
    }
    if !op_auth_bridge::hosted_access_provider().available() {
        if state.status != "unavailable" {
            tracing::warn!("Hosted model access is unavailable in this build");
        }
        state.status = "unavailable";
        state.routes.clear();
        return;
    }
    if state.job.is_some() || Instant::now() < state.refresh_at || state.namespace.is_empty() {
        return;
    }
    let session = session.unwrap();
    let epoch = state.epoch;
    let namespace = state.namespace.clone();
    if state.routes.is_empty() {
        state.status = "loading";
    }
    state.job = Some(crate::chat_runtime::shared_runtime().spawn(async move {
        let operation = async {
            let access = access_for(&session, epoch).await?;
            let routes = catalog::fetch(&access, &namespace).await?;
            check_session(&session, epoch)?;
            Ok::<_, HostedRuntimeError>((access.subject().to_string(), routes))
        };
        let outcome = tokio::select! {
            outcome = operation => outcome,
            error = provider::watch_session(&session, epoch) => Err(error),
        };
        let mut state = STATE.lock().unwrap_or_else(|p| p.into_inner());
        if state.epoch != epoch || state.session.as_ref() != Some(&session) {
            return;
        }
        state.job = None;
        state.refresh_at = Instant::now() + Duration::from_secs(60);
        match outcome {
            Ok((subject, routes)) => {
                let model_count = routes.len();
                if accept_catalog(&mut state, &session, epoch, subject, routes) {
                    tracing::info!(model_count, "Hosted model catalog refreshed");
                }
            }
            Err(error) => {
                tracing::warn!(error = %error, "Hosted model catalog refresh failed");
                state.routes.clear();
                state.status = "failed";
            }
        }
    }));
}

fn accept_catalog(
    state: &mut State,
    session: &Session,
    epoch: u64,
    subject: String,
    routes: Vec<HostedModelRoute>,
) -> bool {
    if state.epoch != epoch || state.session.as_ref() != Some(session) {
        return false;
    }
    if state.subject.as_ref().is_some_and(|old| old != &subject) {
        reset(state, None);
        return false;
    }
    state.subject = Some(subject);
    state.routes = routes;
    state.status = "ready";
    true
}

/// Poll the verified auth runtime and install only the bounded public projection.
pub fn sync_editor(editor: &mut EditorState) -> bool {
    refresh();
    let routes = {
        let state = STATE.lock().unwrap_or_else(|p| p.into_inner());
        if editor.editor_ui.account.is_signed_in() {
            state.routes.clone()
        } else {
            Vec::new()
        }
    };
    if editor.editor_ui.hosted_model_routes() == routes.as_slice() {
        return false;
    }
    if routes.is_empty() || editor.replace_hosted_model_catalog(routes).is_err() {
        editor.clear_hosted_model_catalog();
    }
    true
}

/// Safe daemon projection: no identity, token, issuer or endpoint crosses this boundary.
pub fn catalog_json() -> String {
    refresh();
    let state = STATE.lock().unwrap_or_else(|p| p.into_inner());
    let routes: Vec<_> = state.routes.iter().map(|route| {
        let cap = route.capabilities();
        serde_json::json!({ "route_id": route.route_id(), "model_id": route.model_id(),
            "display_name": route.display_name(), "protocol": catalog::protocol_wire(route.protocol()),
            "capabilities": { "image_input": cap.image_input, "tools": cap.tools,
                "structured_output": cap.structured_output, "reasoning": cap.reasoning } })
    }).collect();
    serde_json::json!({ "status": state.status, "revision": state.epoch, "routes": routes })
        .to_string()
}

/// Resolve only an opaque id minted into this verified session's runtime catalog.
pub fn provider_for_route(route_id: &str) -> Result<Box<dyn ChatProvider>, HostedRuntimeError> {
    refresh();
    let state = STATE.lock().unwrap_or_else(|p| p.into_inner());
    let session = state
        .session
        .clone()
        .ok_or(HostedRuntimeError::NotSignedIn)?;
    let route = state
        .routes
        .iter()
        .find(|route| route.route_id() == route_id)
        .cloned()
        .ok_or(HostedRuntimeError::RouteUnavailable)?;
    if route.protocol() == HostedProtocol::OpenAiResponses {
        return Err(HostedRuntimeError::UnsupportedProtocol);
    }
    let subject = state
        .subject
        .clone()
        .ok_or(HostedRuntimeError::AccessRejected)?;
    Ok(Box::new(provider::HostedProvider {
        session,
        epoch: state.epoch,
        subject,
        route,
    }))
}

fn check_session(session: &Session, epoch: u64) -> Result<(), HostedRuntimeError> {
    if current_session().as_ref() != Some(session) {
        return Err(HostedRuntimeError::SessionChanged);
    }
    let state = STATE.lock().unwrap_or_else(|p| p.into_inner());
    if state.epoch != epoch || state.session.as_ref() != Some(session) {
        return Err(HostedRuntimeError::SessionChanged);
    }
    Ok(())
}

struct AccessHandle<'a>(
    &'a dyn op_auth_bridge::HostedAccessProvider,
    HostedAccessRequestId,
);
impl Drop for AccessHandle<'_> {
    fn drop(&mut self) {
        self.0.cancel(self.1);
    }
}

async fn access_for(session: &Session, epoch: u64) -> Result<HostedAccess, HostedRuntimeError> {
    let provider = op_auth_bridge::hosted_access_provider();
    if session.auth_epoch == 0 {
        return Err(HostedRuntimeError::Unavailable);
    }
    access_with(provider, || check_session(session, epoch)).await
}

async fn access_with(
    provider: &dyn op_auth_bridge::HostedAccessProvider,
    check: impl Fn() -> Result<(), HostedRuntimeError>,
) -> Result<HostedAccess, HostedRuntimeError> {
    check()?;
    let handle = AccessHandle(
        provider,
        provider.begin().map_err(|error| {
            tracing::warn!(error = %error, "Hosted access request could not start");
            HostedRuntimeError::AccessRejected
        })?,
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        check()?;
        match provider.poll(handle.1).map_err(|error| {
            tracing::warn!(error = %error, "Hosted access request could not be polled");
            HostedRuntimeError::AccessRejected
        })? {
            HostedAccessPoll::Pending if Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(50)).await
            }
            HostedAccessPoll::Ready(access) => {
                check()?;
                if access.expires_at_unix() <= now_unix() + 30 {
                    tracing::warn!("Hosted access expires too soon to use");
                    return Err(HostedRuntimeError::AccessRejected);
                }
                return Ok(access);
            }
            HostedAccessPoll::Failed(error) => {
                tracing::warn!(error = %error, "Hosted access request failed");
                return Err(HostedRuntimeError::AccessRejected);
            }
            HostedAccessPoll::Pending => {
                tracing::warn!("Hosted access request timed out");
                return Err(HostedRuntimeError::AccessRejected);
            }
        }
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn client() -> Result<reqwest::Client, HostedRuntimeError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|_| HostedRuntimeError::Network)
}

#[cfg(test)]
mod tests;
