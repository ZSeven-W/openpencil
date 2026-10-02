//! Shared execution of an already scoped direct-modify turn.

use super::*;

pub(super) fn launch_direct_modify_turn(
    host: &mut WidgetHostNative,
    user_text: &str,
    route: op_editor_core::LaunchRoute,
    current_chat: &mut Option<ChatSession>,
    current_design: &mut Option<DesignSession>,
) -> bool {
    let Some(provider) = provider_for_selected_model(host) else {
        return false;
    };
    let Some(plan) = op_host_services::chat_intent::build_modify_plan_for_route(
        host.editor_state(),
        user_text,
        route,
    ) else {
        return false;
    };
    let target_frame_ids = plan.target_frame_ids;
    let request = ChatRequest {
        system_prompt: plan.system_prompt,
        user_message: plan.user_message,
        max_output_tokens: 8192,
        model: selected_cli_model_id(host),
        // Structured-JSON turn: reasoning models (MiniMax-M3, GLM-5.x)
        // burn the whole output budget inside <think> and emit zero
        // nodes (measured: an M3 modify turn died in analysis prose).
        // Same policy as the orchestrator's design subtasks.
        thinking: op_ai::chat_provider::ThinkingMode::Disabled,
        ..Default::default()
    };
    let (chat_tx, chat_rx) = mpsc::channel::<ChatDelta>();
    let (executor, tool_rx) = chat_tool_channel();
    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    *current_design = None;
    super::super::finalize_design_session_if_needed(host, current_chat, "teardown-backstop");
    *current_chat = Some(ChatSession::from_channels_with_cancel(
        chat_rx,
        Some(tool_rx),
        Arc::clone(&cancel),
    ));
    let spawned = thread::Builder::new()
        .name("op-chat-modify".into())
        .spawn(move || {
            op_host_services::chat_intent::run_modify_turn_cancellable(
                provider.as_ref(),
                request,
                &chat_tx,
                &executor,
                target_frame_ids,
                cancel,
            );
        });
    if let Err(err) = spawned {
        // Worker never started — un-park the session and fall through
        // to the honest-error path instead of crashing the UI thread.
        eprintln!("openpencil-desktop: spawn op-chat-modify thread failed: {err}");
        *current_chat = None;
        return false;
    }
    true
}
