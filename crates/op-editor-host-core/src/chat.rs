//! Shared chat turn worker and transcript folding logic.

use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};
use std::sync::Mutex;
use std::thread;

use op_ai::chat_provider::{
    ChatDelta, ChatHistoryRole, ChatProvider, ChatRequest, ChatToolExecutor, ChatToolResult,
};
use op_editor_core::{ChatMessage, ChatRole, ChatToolCall};

/// One tool call forwarded from an agent-loop worker to a host UI thread.
pub struct ChatToolRequest {
    pub name: String,
    pub args_json: String,
    pub ack: SyncSender<ChatToolResult>,
}

/// Worker-side [`ChatToolExecutor`] that forwards calls over a channel and
/// blocks until the host acks with the real tool result.
pub struct UiChatToolExecutor {
    tx: Mutex<Sender<ChatToolRequest>>,
}

impl UiChatToolExecutor {
    pub fn new(tx: Sender<ChatToolRequest>) -> Self {
        Self { tx: Mutex::new(tx) }
    }
}

impl ChatToolExecutor for UiChatToolExecutor {
    fn execute(&self, name: &str, args_json: &str) -> ChatToolResult {
        let (ack_tx, ack_rx) = std::sync::mpsc::sync_channel::<ChatToolResult>(1);
        let req = ChatToolRequest {
            name: name.to_string(),
            args_json: args_json.to_string(),
            ack: ack_tx,
        };
        let sent = match self.tx.lock() {
            Ok(tx) => tx.send(req).is_ok(),
            Err(_) => false,
        };
        if !sent {
            return aborted_result();
        }
        match ack_rx.recv_timeout(std::time::Duration::from_secs(60)) {
            Ok(result) => result,
            Err(_) => timeout_result(),
        }
    }
}

fn timeout_result() -> ChatToolResult {
    ChatToolResult {
        content: r#"{"success":false,"error":"tool execution timed out waiting for the editor"}"#
            .into(),
        is_error: true,
    }
}

fn aborted_result() -> ChatToolResult {
    ChatToolResult {
        content: r#"{"success":false,"error":"chat turn aborted before the tool ran"}"#.into(),
        is_error: true,
    }
}

/// Create the worker-to-host tool channel for one chat turn.
pub fn chat_tool_channel() -> (UiChatToolExecutor, Receiver<ChatToolRequest>) {
    let (tx, rx) = std::sync::mpsc::channel::<ChatToolRequest>();
    (UiChatToolExecutor::new(tx), rx)
}

/// One in-flight chat turn.
pub struct ChatSession {
    rx: Receiver<ChatDelta>,
    tool_rx: Option<Receiver<ChatToolRequest>>,
    finished: bool,
}

/// Result of a single non-blocking [`ChatSession::poll`].
pub struct ChatPoll {
    pub text: String,
    pub thinking: String,
    pub tool_calls: Vec<ChatToolCall>,
    pub error: Option<String>,
    pub finished: bool,
}

impl ChatPoll {
    /// True when this poll carried no new content and the turn has not ended.
    pub fn is_idle(&self) -> bool {
        self.text.is_empty()
            && self.thinking.is_empty()
            && self.tool_calls.is_empty()
            && self.error.is_none()
            && !self.finished
    }
}

/// Fold one [`ChatPoll`] into the trailing assistant `message`.
pub fn apply_poll_to_message(message: &mut ChatMessage, poll: &ChatPoll) {
    if let Some(err) = &poll.error {
        message.content = format!("error: {err}");
    } else {
        message.content.push_str(&poll.text);
    }
    message.thinking.push_str(&poll.thinking);
    if poll.tool_calls.iter().any(tool_call_defaults_open) {
        message.tools_collapsed = false;
    }
    message.tool_calls.extend(poll.tool_calls.iter().cloned());
    if poll.finished {
        message.streaming = false;
    }
}

/// Map the chat transcript into `(role, text)` history pairs for the in-flight
/// turn, excluding the current user message and trailing streaming assistant.
pub fn chat_history_from_transcript(messages: &[ChatMessage]) -> Vec<(ChatHistoryRole, String)> {
    let mut end = messages.len();
    if end > 0 && messages[end - 1].role == ChatRole::Assistant && messages[end - 1].streaming {
        end -= 1;
    }
    if end > 0 && messages[end - 1].role == ChatRole::User {
        end -= 1;
    }
    messages[..end]
        .iter()
        .filter(|m| !m.content.trim().is_empty())
        .map(|m| {
            let role = match m.role {
                ChatRole::User => ChatHistoryRole::User,
                ChatRole::Assistant => ChatHistoryRole::Assistant,
            };
            (role, m.content.clone())
        })
        .collect()
}

fn tool_call_defaults_open(call: &ChatToolCall) -> bool {
    if let Some(level) = tool_level_from_args(&call.args) {
        return matches!(level.as_str(), "modify" | "delete" | "orchestrate");
    }
    matches!(
        call.name.as_str(),
        "update_node"
            | "replace_node"
            | "move_node"
            | "set_variables"
            | "set_themes"
            | "load_theme_preset"
            | "rename_page"
            | "reorder_page"
            | "batch_design"
            | "set_design_md"
            | "export_design_md"
            | "delete_node"
            | "remove_page"
    )
}

fn tool_level_from_args(args: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(args)
        .ok()?
        .get("level")?
        .as_str()
        .map(str::to_string)
}

impl ChatSession {
    /// Spawn a worker that drains `provider.send(req)` into a channel.
    pub fn start(provider: Box<dyn ChatProvider>, req: ChatRequest) -> Self {
        Self::start_with_tools(provider, req, None)
    }

    /// [`start`](Self::start) plus a tool-request receiver for providers that
    /// execute host tools.
    pub fn start_with_tools(
        provider: Box<dyn ChatProvider>,
        req: ChatRequest,
        tool_rx: Option<Receiver<ChatToolRequest>>,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::Builder::new()
            .name("op-chat-turn".into())
            .spawn(move || {
                for delta in provider.send(req) {
                    if tx.send(delta).is_err() {
                        return;
                    }
                }
            })
            .expect("spawn op-chat-turn thread");
        Self {
            rx,
            tool_rx,
            finished: false,
        }
    }

    /// Wrap externally supplied channels, used when a host owns its own
    /// routing worker but wants the shared poll/finish behavior.
    pub fn from_channels(
        rx: Receiver<ChatDelta>,
        tool_rx: Option<Receiver<ChatToolRequest>>,
    ) -> Self {
        Self {
            rx,
            tool_rx,
            finished: false,
        }
    }

    /// Drain every delta ready right now without blocking.
    pub fn poll(&mut self) -> ChatPoll {
        let mut text = String::new();
        let mut thinking = String::new();
        let mut tool_calls = Vec::new();
        let mut error = None;
        loop {
            match self.rx.try_recv() {
                Ok(ChatDelta::TextDelta(s)) => text.push_str(&s),
                Ok(ChatDelta::Thinking(s)) => thinking.push_str(&s),
                Ok(ChatDelta::ToolUse { name, args }) => {
                    tool_calls.push(ChatToolCall { name, args });
                }
                Ok(ChatDelta::Error(msg)) => {
                    if error.is_none() {
                        error = Some(msg);
                    }
                }
                Ok(ChatDelta::Done { .. }) => self.finished = true,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.finished = true;
                    break;
                }
            }
        }
        ChatPoll {
            text,
            thinking,
            tool_calls,
            error,
            finished: self.finished,
        }
    }

    /// Drain pending host tool requests without blocking.
    pub fn drain_tool_requests(&mut self) -> Vec<ChatToolRequest> {
        let Some(tool_rx) = self.tool_rx.as_ref() else {
            return Vec::new();
        };
        let mut requests = Vec::new();
        while let Ok(req) = tool_rx.try_recv() {
            requests.push(req);
        }
        requests
    }

    pub fn finished(&self) -> bool {
        self.finished
    }
}
