//! Router worker: the pre-computed `CliTurnPlan` and the two turn runners
//! it dispatches to (`run_cli_turn` for the classify → chat/design route,
//! `run_modify_turn` for the selection-scoped edit route). Split out of
//! `chat_intent.rs` to keep the spine under the 800-line cap.

use super::*;

// ---------------------------------------------------------------------------
// Router worker
// ---------------------------------------------------------------------------

/// Everything the router worker needs, pre-computed on the UI thread.
pub struct CliTurnPlan {
    pub user_text: String,
    /// TS `pageChildren.length === 0` (modify degrades to new).
    pub page_children_empty: bool,
    /// Session-untracked transport for the classification call.
    pub classify_provider: Box<dyn ChatProvider>,
    /// Chat-session-tracked transport for the plain chat route.
    pub chat_provider: Box<dyn ChatProvider>,
    /// Session-untracked transport for the design / modify routes.
    pub design_provider: Box<dyn ChatProvider>,
    /// Fully-assembled plain-chat request (system prompt + history +
    /// per-turn knobs + attachments).
    pub chat_request: ChatRequest,
    /// `generateDesignModification` request; `None` when the page had
    /// no modification target (route degrades to new).
    pub modify_request: Option<ChatRequest>,
    /// Orchestrator request for the new-design route (append context
    /// already detected + attached).
    pub design_request: DesignRequest,
    /// State snapshot for the design route's `RemoteDocSink` mirror.
    pub initial_state: EditorState,
    /// Host-owned indicator epoch shared with the parked `DesignSession`.
    /// The worker registers frame/node indicators under this value and
    /// `DesignSession::drop` clears that same run when Done / Stop /
    /// New Chat retires the session.
    pub indicator_epoch: u64,
    /// Shared with the parked `DesignSession` so Stop / New Chat can cancel
    /// the design route even while its concurrent screen workers are active.
    pub abort: AbortFlag,
    pub model: Option<String>,
}

pub use op_chat_agent::chat_modify::{run_modify_turn, run_modify_turn_cancellable, MODIFY_STEP};

/// The TS degrade rules (`ai-chat-handlers.ts:700-705`): a modify
/// intent on an empty page becomes a new design, and `isModification`
/// additionally requires a usable target (here: a built modify plan —
/// after the empty-page degrade a surviving Modify always has one in
/// practice, so the plan check is a belt-and-braces guard).
pub(super) fn resolve_route(
    classified: DesignIntent,
    page_children_empty: bool,
    has_modify_plan: bool,
) -> DesignIntent {
    match classified {
        DesignIntent::Modify if page_children_empty => DesignIntent::New,
        DesignIntent::Modify if !has_modify_plan => DesignIntent::New,
        other => other,
    }
}

/// Run one CLI standard-mode turn end-to-end on the worker thread:
/// classify, then route to chat / modification / design. The caller
/// parked a `ChatSession` on `chat_tx` + the tool channel behind
/// `executor`, and a `DesignSession` on `delta_tx` / `cmd_tx`; the
/// routes not taken drop their senders so the matching pump retires
/// its session.
pub fn run_cli_turn(
    plan: CliTurnPlan,
    chat_tx: Sender<ChatDelta>,
    executor: UiChatToolExecutor,
    delta_tx: Sender<DesignDelta>,
    cmd_tx: Sender<DesignCmdReq>,
) {
    let modify_target_frame_ids =
        modify_target_frame_ids(&plan.initial_state, &plan.user_text).unwrap_or_default();
    let turn_cancel = plan.abort.shared_atomic();
    let classified = classify_intent_for_standard_route_cancellable(
        plan.classify_provider.as_ref(),
        &plan.initial_state,
        &plan.user_text,
        plan.model.clone(),
        Arc::clone(&turn_cancel),
    );
    // Stop / New Chat may arrive while the classifier LLM is in flight. The
    // provider call itself has its own bounded timeout, but once it returns we
    // must not launch a fresh chat, modify, or concurrent design run for the
    // discarded session.
    if plan.abort.is_set() {
        return;
    }
    let modify_request = plan.modify_request;
    let intent = resolve_route(
        classified,
        plan.page_children_empty,
        modify_request.is_some(),
    );

    match intent {
        DesignIntent::Chat => {
            drop(delta_tx);
            drop(cmd_tx);
            drop(executor);
            for delta in plan
                .chat_provider
                .send_cancellable(plan.chat_request, turn_cancel)
            {
                if chat_tx.send(delta).is_err() {
                    return; // turn aborted (Stop / New Chat)
                }
            }
        }
        DesignIntent::Modify => {
            drop(delta_tx);
            drop(cmd_tx);
            let request = modify_request.expect("checked above");
            run_modify_turn_cancellable(
                plan.design_provider.as_ref(),
                request,
                &chat_tx,
                &executor,
                modify_target_frame_ids,
                turn_cancel,
            );
        }
        DesignIntent::New => {
            // Hold the chat channel open for the duration so the
            // trailing bubble keeps its streaming state while the
            // design pumps fill it.
            let _chat_hold = chat_tx;
            drop(executor);
            // Share one provider Arc between the design LLM and the
            // (flag-gated) vision validator so the real Class-C loop reuses
            // the user's selected auth/model.
            let provider_arc: Arc<dyn op_ai::chat_provider::ChatProvider> =
                Arc::from(plan.design_provider);
            let llm = ChatProviderLlmClient::new(provider_arc.clone()).with_model(plan.model);
            run_design_worker(
                llm,
                plan.design_request,
                plan.initial_state,
                delta_tx,
                cmd_tx,
                plan.indicator_epoch,
                plan.abort,
                Some(provider_arc),
                // The CLI standard route already moved this turn's
                // attachments into `chat_request`; the design route has no
                // attachment channel of its own here.
                Vec::new(),
            );
        }
    }
}
