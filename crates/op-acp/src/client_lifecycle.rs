//! Cancellation-safe ACP transport and child-process teardown.

use super::*;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::protocol::{
    AuthenticateRequest, AuthenticateResponse, CloseSessionRequest, CloseSessionResponse,
    DeleteSessionRequest, DeleteSessionResponse, METHOD_AUTHENTICATE, METHOD_SESSION_CLOSE,
    METHOD_SESSION_DELETE,
};

/// Session cleanup is best-effort at host boundaries and must never hold
/// process/transport teardown for the much longer prompt timeout.
const SESSION_LIFECYCLE_TIMEOUT: Duration = Duration::from_secs(2);

impl AcpConnection {
    async fn typed_request<Request, Response>(
        &self,
        method: &str,
        request: &Request,
        timeout: Duration,
    ) -> Result<(), AcpError>
    where
        Request: Serialize,
        Response: DeserializeOwned,
    {
        let params =
            serde_json::to_value(request).map_err(|error| AcpError::Protocol(error.to_string()))?;
        let result = self.engine.call(method, params, timeout).await?;
        serde_json::from_value::<Response>(result)
            .map_err(|error| AcpError::Protocol(error.to_string()))?;
        Ok(())
    }

    /// Authenticate with an exact method advertised by `initialize`.
    pub async fn authenticate(&self, method_id: &str) -> Result<(), AcpError> {
        if !self
            .auth_methods
            .iter()
            .any(|method| method.id().0.as_ref() == method_id)
        {
            return Err(AcpError::Protocol(format!(
                "agent did not advertise authentication method '{method_id}'"
            )));
        }
        self.typed_request::<_, AuthenticateResponse>(
            METHOD_AUTHENTICATE,
            &AuthenticateRequest::new(method_id.to_owned()),
            HANDSHAKE_TIMEOUT,
        )
        .await
    }

    /// Retry-safe authentication for an `auth_required` session/new error.
    /// Stable v1 agent auth carries no credentials, but choosing between
    /// multiple methods is a user decision and OpenPencil has no picker yet.
    pub(super) async fn authenticate_unambiguous(&self) -> Result<(), AcpError> {
        match self.auth_methods.as_slice() {
            [method] => self.authenticate(method.id().0.as_ref()).await,
            [] => Err(AcpError::Protocol(
                "agent required authentication but advertised no authMethods".into(),
            )),
            methods => Err(AcpError::Config(format!(
                "agent requires authentication and advertised {} methods; OpenPencil needs an authentication method picker before it can choose one",
                methods.len()
            ))),
        }
    }

    /// Whether stable-v1 `session/close` is available on this connection.
    pub fn supports_session_close(&self) -> bool {
        self.agent_capabilities.session_capabilities.close.is_some()
    }

    /// Close an active session only when the agent advertised support.
    /// Returns whether a wire request was sent.
    pub async fn close_session_if_supported(&self, session_id: &str) -> Result<bool, AcpError> {
        if !self.supports_session_close() {
            return Ok(false);
        }
        self.typed_request::<_, CloseSessionResponse>(
            METHOD_SESSION_CLOSE,
            &CloseSessionRequest::new(session_id.to_owned()),
            SESSION_LIFECYCLE_TIMEOUT,
        )
        .await?;
        Ok(true)
    }

    /// Whether stable-v1 `session/delete` is available on this connection.
    pub fn supports_session_delete(&self) -> bool {
        self.agent_capabilities
            .session_capabilities
            .delete
            .is_some()
    }

    /// Delete a session only when the agent advertised support. Probe callers
    /// use this for their deliberately ephemeral validation session.
    pub async fn delete_session_if_supported(&self, session_id: &str) -> Result<bool, AcpError> {
        if !self.supports_session_delete() {
            return Ok(false);
        }
        self.typed_request::<_, DeleteSessionResponse>(
            METHOD_SESSION_DELETE,
            &DeleteSessionRequest::new(session_id.to_owned()),
            SESSION_LIFECYCLE_TIMEOUT,
        )
        .await?;
        Ok(true)
    }

    fn abort_io_tasks(&mut self) {
        for task in self.tasks.drain(..) {
            task.abort();
        }
    }

    /// Deterministically stop the transport and reap a local agent's whole
    /// process tree. Child/task ownership stays on `self` across every await,
    /// so cancelling this future leaves Drop able to finish cleanup.
    pub async fn shutdown(&mut self) {
        self.abort_io_tasks();
        let mut release_child = false;
        let mut reap_signalled_child = false;
        if let Some(child) = self.child.as_mut() {
            match op_process_io::terminate_tokio_process_tree(child, PROCESS_SHUTDOWN_GRACE).await {
                Ok(_) => release_child = true,
                Err(_) => {
                    // A tree signal can fail even though the exact leader
                    // accepted its kill. Re-observe and retry the owned leader
                    // handle so that case still reaches the reaper, while two
                    // genuine signal failures retain ownership for Drop.
                    if let Some(needs_reap) = force_child_for_reap(child) {
                        release_child = true;
                        reap_signalled_child = needs_reap;
                    }
                }
            }
        }
        if release_child {
            let child = self.child.take().expect("release requires a child");
            if reap_signalled_child {
                reap_in_background(child);
            }
        }
        if let Some(task) = self.stderr_task.as_mut() {
            if tokio::time::timeout(STDERR_DRAIN_GRACE, &mut *task)
                .await
                .is_err()
            {
                task.abort();
            }
        }
        self.stderr_task.take();
    }

    /// Immediate process-tree kill used by Drop and cancellation-unwind paths.
    pub fn disconnect(&mut self) {
        self.abort_io_tasks();
        let disposition = self.child.as_mut().and_then(force_child_for_reap);
        if let Some(needs_reap) = disposition {
            let child = self.child.take().expect("disposition requires a child");
            if needs_reap {
                reap_in_background(child);
            }
        }
        if let Some(task) = self.stderr_task.take() {
            task.abort();
        }
    }
}

/// Return `Some(needs_reap)` only when the child is already reaped or at least
/// one termination request was accepted. `None` keeps the live child owned so
/// another disconnect/Drop attempt (plus `kill_on_drop`) can retry safely.
fn force_child_for_reap(child: &mut Child) -> Option<bool> {
    match child.try_wait() {
        Ok(Some(_)) => Some(false),
        Ok(None) => {
            if op_process_io::kill_tokio_process_tree(child).is_ok() {
                return Some(true);
            }
            // The shared tree helper deliberately returns an error when
            // descendant cleanup failed even if its direct leader kill was
            // accepted. A second exact-handle kill makes acceptance observable.
            match child.start_kill() {
                Ok(()) => Some(true),
                Err(_) => match child.try_wait() {
                    Ok(Some(_)) => Some(false),
                    Ok(None) | Err(_) => None,
                },
            }
        }
        Err(_) => None,
    }
}

impl Drop for AcpConnection {
    fn drop(&mut self) {
        self.disconnect();
    }
}

/// Tokio's async wait cannot be relied on from synchronous Drop or while its
/// runtime is shutting down. `try_wait` is non-async, so a short-lived OS
/// thread can always reap the already-killed direct child and avoid zombies.
fn reap_in_background(mut child: Child) {
    if matches!(child.try_wait(), Ok(Some(_)) | Err(_)) {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("op-acp-child-reaper".into())
        .spawn(move || loop {
            match child.try_wait() {
                Ok(Some(_)) | Err(_) => break,
                Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            }
        });
}
