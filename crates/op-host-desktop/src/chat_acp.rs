//! ACP chat bridge — adapts an `op_acp` connection to the
//! [`ChatProvider`] trait.
//!
//! `ChatProviderKind::Acp` is the catch-all for third-party agents
//! OpenPencil ships no dedicated adapter for. An [`AcpProvider`] is
//! built from a persisted [`AcpAgentConfig`]; each `send` connects,
//! opens a session, drives one prompt turn, and streams the agent's
//! `session/update` notifications back as `ChatDelta`s.

use op_acp::{connect_acp_agent, session_update_to_delta, AcpAgentConfig, ConnectionType};
use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, EffortLevel, StopReason};
use tokio::sync::mpsc;

use crate::chat_attachment::TempGuard;
use crate::chat_runtime::{prompt_with_system_prompt, shared_runtime, BlockingRecvIter};

/// `ChatProvider` backed by a third-party ACP agent.
pub struct AcpProvider {
    config: AcpAgentConfig,
    label: String,
}

impl AcpProvider {
    /// Build an ACP provider for a persisted agent config.
    #[allow(dead_code)]
    pub fn new(config: AcpAgentConfig) -> Self {
        let label = format!("ACP: {}", config.display_name);
        Self { config, label }
    }
}

impl ChatProvider for AcpProvider {
    fn provider_label(&self) -> &str {
        &self.label
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let config = self.config.clone();
        // ACP `session/prompt` carries plain text. A local agent can
        // read temp-file path lines; a remote (WebSocket) agent
        // cannot, so for remote agents the attachments are omitted
        // with an honest note rather than passing meaningless local
        // paths. The thinking / effort knobs ride in-band either way
        // (ACP exposes no dedicated reasoning channel).
        let (mut prompt, guard) = if config.connection_type == ConnectionType::Local {
            match crate::chat_attachment::prompt_with_attachments(
                &request.user_message,
                &request.attachments,
            ) {
                Ok(pair) => pair,
                Err(e) => return crate::chat_attachment::attachment_error_turn(e),
            }
        } else {
            let mut prompt = request.user_message.clone();
            if !request.attachments.is_empty() {
                prompt.push_str(&format!(
                    "\n\n[note: {} attachment(s) omitted — a remote ACP agent \
                     cannot read local files]",
                    request.attachments.len()
                ));
            }
            (prompt, None)
        };
        let mut directive = String::new();
        if let Some(d) = crate::chat_attachment::thinking_directive(request.thinking) {
            directive.push_str(d);
        }
        if request.effort != EffortLevel::Low {
            if !directive.is_empty() {
                directive.push(' ');
            }
            directive.push_str(&format!(
                "Apply {} reasoning effort.",
                request.effort.as_str()
            ));
        }
        if !directive.is_empty() {
            prompt = format!("{directive}\n\n{prompt}");
        }
        prompt = prompt_with_system_prompt(&request.system_prompt, prompt);
        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        shared_runtime().spawn(async move {
            run_acp_turn(config, prompt, guard, tx).await;
        });
        Box::new(BlockingRecvIter::new(rx))
    }
}

/// Connect, open a session, and drive one prompt turn — streaming
/// `session/update` notifications into `tx` as they arrive and
/// emitting a terminal `Done` once `session/prompt` returns.
async fn run_acp_turn(
    config: AcpAgentConfig,
    prompt: String,
    // Held for the turn so staged attachment temp files survive until
    // the agent has read them; dropped (and cleaned up) on return.
    _guard: Option<TempGuard>,
    tx: mpsc::Sender<ChatDelta>,
) {
    let mut conn = match connect_acp_agent(&config).await {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(ChatDelta::Error(format!("acp connect: {e}"))).await;
            let _ = tx
                .send(ChatDelta::Done {
                    stop_reason: StopReason::Aborted,
                })
                .await;
            return;
        }
    };
    let mut notes = match conn.take_notifications() {
        Some(n) => n,
        None => {
            let _ = tx
                .send(ChatDelta::Error(
                    "acp: notification channel unavailable".into(),
                ))
                .await;
            let _ = tx
                .send(ChatDelta::Done {
                    stop_reason: StopReason::Aborted,
                })
                .await;
            return;
        }
    };
    let session = match conn.new_session().await {
        Ok(s) => s,
        Err(e) => {
            let _ = tx.send(ChatDelta::Error(format!("acp session: {e}"))).await;
            let _ = tx
                .send(ChatDelta::Done {
                    stop_reason: StopReason::Aborted,
                })
                .await;
            return;
        }
    };

    // Run the prompt turn while concurrently streaming notifications.
    let prompt_fut = conn.prompt(&session, &prompt);
    tokio::pin!(prompt_fut);
    let mut notes_open = true;
    let result = loop {
        tokio::select! {
            biased;
            res = &mut prompt_fut => break res,
            note = notes.recv(), if notes_open => match note {
                Some(note) => {
                    if let Some(delta) = session_update_to_delta(&note) {
                        if tx.send(delta).await.is_err() {
                            return; // chat panel went away
                        }
                    }
                }
                None => notes_open = false,
            }
        }
    };
    // Flush any notifications buffered before the turn resolved.
    while let Ok(note) = notes.try_recv() {
        if let Some(delta) = session_update_to_delta(&note) {
            let _ = tx.send(delta).await;
        }
    }
    match result {
        Ok(()) => {
            let _ = tx
                .send(ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                })
                .await;
        }
        Err(e) => {
            let _ = tx.send(ChatDelta::Error(format!("acp prompt: {e}"))).await;
            let _ = tx
                .send(ChatDelta::Done {
                    stop_reason: StopReason::Aborted,
                })
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_acp::ConnectionType;
    use std::sync::Arc;

    fn config() -> AcpAgentConfig {
        AcpAgentConfig {
            id: "acp-1".into(),
            display_name: "Test Agent".into(),
            connection_type: ConnectionType::Local,
            command: Some("test-acp-agent".into()),
            args: vec![],
            env: std::collections::BTreeMap::new(),
            url: None,
            enabled: true,
        }
    }

    #[test]
    fn provider_label_names_the_agent() {
        let p = AcpProvider::new(config());
        assert_eq!(p.provider_label(), "ACP: Test Agent");
    }

    #[test]
    fn provider_constructs_as_chat_provider_trait_object() {
        let _: Arc<dyn ChatProvider> = Arc::new(AcpProvider::new(config()));
    }
}
