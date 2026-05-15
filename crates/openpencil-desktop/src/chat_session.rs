//! Background chat-turn runner.
//!
//! `ChatProvider::send()` hands back a *blocking* iterator — draining
//! it on the UI thread would freeze the window for the whole LLM
//! turn. `ChatSession` drains the turn on a dedicated worker thread
//! and exposes a non-blocking [`ChatSession::poll`] the winit event
//! loop pumps each frame, appending deltas to the in-flight assistant
//! message.

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use openpencil_shell_core::chat_provider::{
    ChatDelta, ChatProvider, ChatRequest, CliName,
};
use openpencil_shell_native::WidgetHostNative;

use crate::chat_claude::ClaudeCodeProvider;
use crate::chat_copilot::CopilotProvider;
use crate::chat_subprocess::SubprocessProvider;

/// One in-flight chat turn. The worker thread owns the provider and
/// drains `provider.send()` into the channel; [`poll`] consumes
/// whatever is ready without blocking.
pub struct ChatSession {
    rx: Receiver<ChatDelta>,
    finished: bool,
}

/// Result of a single non-blocking [`ChatSession::poll`].
pub struct ChatPoll {
    /// Text fragments (`TextDelta` + `Thinking`) accumulated since
    /// the last poll. Empty when nothing new arrived.
    pub text: String,
    /// First error seen this poll, if any. When set the caller
    /// should surface it as the assistant message body.
    pub error: Option<String>,
    /// True once the turn's terminal `Done` arrived, or the worker
    /// thread / channel closed.
    pub finished: bool,
}

impl ChatSession {
    /// Spawn a worker that drains `provider.send(req)` into a
    /// channel. Returns immediately — the LLM turn runs off-thread.
    pub fn start(provider: Box<dyn ChatProvider>, req: ChatRequest) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::Builder::new()
            .name("op-chat-turn".into())
            .spawn(move || {
                // `provider.send` itself returns a blocking iterator;
                // draining it here keeps the block off the UI thread.
                for delta in provider.send(req) {
                    if tx.send(delta).is_err() {
                        return; // chat panel went away — stop early
                    }
                }
            })
            .expect("spawn op-chat-turn thread");
        Self {
            rx,
            finished: false,
        }
    }

    /// Drain every delta ready right now without blocking.
    pub fn poll(&mut self) -> ChatPoll {
        let mut text = String::new();
        let mut error = None;
        loop {
            match self.rx.try_recv() {
                Ok(ChatDelta::TextDelta(s)) | Ok(ChatDelta::Thinking(s)) => {
                    text.push_str(&s);
                }
                Ok(ChatDelta::Error(msg)) => {
                    if error.is_none() {
                        error = Some(msg);
                    }
                }
                Ok(ChatDelta::Done { .. }) => self.finished = true,
                // Tool dispatch is the agent runtime's job, not the
                // chat panel's — drop it from the transcript view.
                Ok(ChatDelta::ToolUse { .. }) => {}
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.finished = true;
                    break;
                }
            }
        }
        ChatPoll {
            text,
            error,
            finished: self.finished,
        }
    }

    /// True once the turn has fully completed.
    pub fn finished(&self) -> bool {
        self.finished
    }
}

/// Drain `chat.pending_send` (raised by `ChatState::begin_send`)
/// into a fresh `ChatSession` against the Claude Code CLI.
/// `ClaudeCodeProvider` auto-discovers the `claude` binary. A send
/// fired mid-turn replaces the in-flight session — the old worker
/// thread drains harmlessly once its channel receiver drops.
/// Returns true when a turn was launched (caller redraws).
pub fn launch_if_pending(
    host: &mut WidgetHostNative,
    current: &mut Option<ChatSession>,
) -> bool {
    let Some(user_text) = host.document_mut().chat.pending_send.take() else {
        return false;
    };
    let provider = provider_for_agent(host.document().ui.chat_selected_agent);
    let req = ChatRequest {
        system_prompt: String::new(),
        user_message: user_text,
        max_output_tokens: 4096,
    };
    *current = Some(ChatSession::start(provider, req));
    true
}

/// Build the `ChatProvider` for an agent index (into
/// `AgentProvider::ALL`: 0 ClaudeCode, 1 CodexCli, 2 OpenCode,
/// 3 GithubCopilot, 4 GeminiCli). Claude Code uses its dedicated
/// SDK adapter; Copilot / Gemini use the subprocess transport.
/// Codex + OpenCode are HTTP-server CLIs whose `ChatProvider`
/// bridge isn't wired yet — they fall back to Claude Code so the
/// chat still functions rather than dead-ending.
fn provider_for_agent(agent_idx: usize) -> Box<dyn ChatProvider> {
    match agent_idx {
        3 => Box::new(CopilotProvider::new()),
        4 => SubprocessProvider::for_cli(CliName::Gemini)
            .map(|p| Box::new(p) as Box<dyn ChatProvider>)
            .unwrap_or_else(|| Box::new(ClaudeCodeProvider::new())),
        _ => Box::new(ClaudeCodeProvider::new()),
    }
}

/// Pump the in-flight turn's deltas into the trailing (assistant)
/// message. Clears `current` once the turn finishes. Returns true
/// when the transcript changed so the caller can dirty the redraw.
pub fn pump(
    host: &mut WidgetHostNative,
    current: &mut Option<ChatSession>,
) -> bool {
    let Some(session) = current.as_mut() else {
        return false;
    };
    let poll = session.poll();
    let mut changed = false;
    if poll.error.is_some() || !poll.text.is_empty() {
        if let Some(msg) = host.document_mut().chat.messages.last_mut() {
            if let Some(err) = poll.error {
                msg.content = format!("error: {err}");
            } else {
                msg.content.push_str(&poll.text);
            }
            changed = true;
        }
    }
    if poll.finished {
        *current = None;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpencil_shell_core::chat_provider::{EchoProvider, StopReason};

    #[test]
    fn session_streams_echo_provider_deltas_to_completion() {
        let provider = Box::new(EchoProvider {
            script: vec![
                ChatDelta::TextDelta("Hel".into()),
                ChatDelta::TextDelta("lo".into()),
                ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ],
        });
        let mut session = ChatSession::start(
            provider,
            ChatRequest {
                system_prompt: String::new(),
                user_message: "hi".into(),
                max_output_tokens: 256,
            },
        );
        // Drain to completion — poll in a bounded loop so a stuck
        // worker fails the test instead of hanging it.
        let mut acc = String::new();
        for _ in 0..1000 {
            let p = session.poll();
            acc.push_str(&p.text);
            if p.finished {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(session.finished(), "session must reach Done");
        assert_eq!(acc, "Hello");
    }

    #[test]
    fn session_surfaces_provider_error() {
        let provider = Box::new(EchoProvider {
            script: vec![ChatDelta::Error("boom".into())],
        });
        let mut session = ChatSession::start(
            provider,
            ChatRequest {
                system_prompt: String::new(),
                user_message: "x".into(),
                max_output_tokens: 0,
            },
        );
        let mut err = None;
        for _ in 0..1000 {
            let p = session.poll();
            if p.error.is_some() {
                err = p.error;
            }
            if p.finished {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(err.as_deref(), Some("boom"));
    }
}
