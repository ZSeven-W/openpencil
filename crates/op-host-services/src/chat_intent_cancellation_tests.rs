//! Cancellation regressions for the CLI intent router.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest};
use op_editor_core::EditorState;
use op_editor_host_core::chat::ChatSession;
use op_orchestrator::{AbortFlag, DesignRequest};

use super::*;
use crate::chat_canvas_tools::chat_tool_channel;

struct SilentCancellable {
    started_tx: Option<mpsc::Sender<()>>,
    canceled_tx: mpsc::Sender<()>,
}

impl ChatProvider for SilentCancellable {
    fn provider_label(&self) -> &str {
        "silent-cancellable"
    }

    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        panic!("cancellable routing must use send_cancellable")
    }

    fn supports_cancellable_send(&self) -> bool {
        true
    }

    fn send_cancellable(
        &self,
        _request: ChatRequest,
        cancel: Arc<AtomicBool>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        if let Some(tx) = &self.started_tx {
            let _ = tx.send(());
        }
        Box::new(SilentCancelIter {
            cancel,
            canceled_tx: Some(self.canceled_tx.clone()),
        })
    }
}

struct SilentCancelIter {
    cancel: Arc<AtomicBool>,
    canceled_tx: Option<mpsc::Sender<()>>,
}

impl Iterator for SilentCancelIter {
    type Item = ChatDelta;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.cancel.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(1));
        }
        if let Some(tx) = self.canceled_tx.take() {
            let _ = tx.send(());
        }
        None
    }
}

struct UnexpectedProvider;

impl ChatProvider for UnexpectedProvider {
    fn provider_label(&self) -> &str {
        "unexpected"
    }

    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        panic!("provider should not run on this route")
    }
}

fn design_request() -> DesignRequest {
    DesignRequest {
        prompt: "unused".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    }
}

#[test]
fn llm_classifier_timeout_cancels_silent_transport() {
    let (canceled_tx, canceled_rx) = mpsc::channel();
    let provider = SilentCancellable {
        started_tx: None,
        canceled_tx,
    };

    let got =
        classify_intent_llm_with_timeout(&provider, "anything", None, Duration::from_millis(30));

    assert_eq!(got, DesignIntent::Chat);
    canceled_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("classifier deadline must abort a silent provider transport");
}

#[test]
fn external_stop_cancels_a_silent_classifier_transport() {
    let (started_tx, started_rx) = mpsc::channel();
    let (canceled_tx, canceled_rx) = mpsc::channel();
    let provider = SilentCancellable {
        started_tx: Some(started_tx),
        canceled_tx,
    };
    let external_cancel = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::clone(&external_cancel);
    let stopper = std::thread::spawn(move || {
        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("classifier provider starts");
        stop_flag.store(true, Ordering::Release);
    });

    let got = classify_intent_llm_with_timeout_and_cancel(
        &provider,
        "anything",
        None,
        Duration::from_secs(5),
        Some(external_cancel.as_ref()),
    );

    stopper.join().expect("stopper exits");
    assert_eq!(got, DesignIntent::Chat);
    canceled_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("Stop/New Chat must abort a silent classifier transport");
}

#[test]
fn cli_turn_chat_route_propagates_abort_to_silent_provider() {
    let (started_tx, started_rx) = mpsc::channel();
    let (canceled_tx, canceled_rx) = mpsc::channel();
    let abort = AbortFlag::new();
    let plan = CliTurnPlan {
        // Punctuation-only input is routed to Chat without invoking the
        // classifier LLM, isolating cancellation of the routed transport.
        user_text: "？？？？".into(),
        page_children_empty: false,
        classify_provider: Box::new(UnexpectedProvider),
        chat_provider: Box::new(SilentCancellable {
            started_tx: Some(started_tx),
            canceled_tx,
        }),
        design_provider: Box::new(UnexpectedProvider),
        chat_request: ChatRequest::default(),
        modify_request: Some(ChatRequest::default()),
        design_request: design_request(),
        initial_state: EditorState::new(),
        indicator_epoch: 0,
        abort: abort.clone(),
        model: None,
    };
    let (chat_tx, chat_rx) = mpsc::channel();
    let (executor, _tool_rx) = chat_tool_channel();
    let session = ChatSession::from_channels_with_cancel(chat_rx, None, abort.shared_atomic());
    let (delta_tx, _delta_rx) = mpsc::channel();
    let (cmd_tx, _cmd_rx) = mpsc::channel();
    let worker =
        std::thread::spawn(move || run_cli_turn(plan, chat_tx, executor, delta_tx, cmd_tx));

    started_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("routed provider starts");
    drop(session);

    canceled_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("Stop/New Chat abort must reach the silent routed provider");
    worker.join().expect("canceled router worker exits");
}

#[test]
fn direct_modify_session_drop_cancels_silent_provider() {
    let (started_tx, started_rx) = mpsc::channel();
    let (canceled_tx, canceled_rx) = mpsc::channel();
    let provider = SilentCancellable {
        started_tx: Some(started_tx),
        canceled_tx,
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let (chat_tx, chat_rx) = mpsc::channel();
    let (executor, _tool_rx) = chat_tool_channel();
    let session = ChatSession::from_channels_with_cancel(chat_rx, None, Arc::clone(&cancel));
    let worker_cancel = Arc::clone(&cancel);
    let worker = std::thread::spawn(move || {
        run_modify_turn_cancellable(
            &provider,
            ChatRequest::default(),
            &chat_tx,
            &executor,
            Vec::new(),
            worker_cancel,
        );
    });

    started_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("direct modify provider starts");
    drop(session);

    canceled_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("dropping direct modify session must cancel its provider");
    worker.join().expect("canceled modify worker exits");
}
