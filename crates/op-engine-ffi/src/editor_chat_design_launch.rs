//! Whole-design mobile launcher, separate from scoped modification.

use super::*;

/// Spawn the REAL desktop design agent loop
/// (`op_chat_agent::chat_agent_loop`) for one prepared turn: the
/// tool-request channel bridges the worker to the engine thread, whose pump
/// executes each call against the live document (`execute_tool_requests`),
/// including the loop's finalize / blocker probes.
pub(super) fn start_design_turn(
    turn: BuiltinChatTurn,
    running_tab: usize,
    root_seed_hints: (bool, bool),
) -> Result<ChatTurnJob, String> {
    let runtime = chat_runtime().map_err(|error| format!("error: {error}"))?;
    let (executor, tool_rx) = chat_tool_channel();
    let (std_tx, std_rx) = mpsc::channel::<ChatDelta>();
    let disable_thinking = design_turn_disable_thinking(Some(&turn.model));
    let url = match turn.kind {
        BuiltinAgentKind::Anthropic => provider_endpoint(&turn.base_url, "/v1/messages"),
        BuiltinAgentKind::OpenAiCompat => provider_endpoint(&turn.base_url, "/chat/completions"),
    };
    let kind = turn.kind;
    let cfg = AgentLoopConfig {
        url,
        api_key: turn.api_key,
        model: turn.model,
        system_prompt: turn.system_prompt,
        history: turn.history,
        user_prompt: turn.prompt,
        max_output_tokens: turn.max_output_tokens,
        tools: mobile_design_tool_defs(),
        executor: Arc::new(executor),
        max_turns: DESIGN_LOOP_MAX_TURNS,
        finalize_on_exit: true,
        disable_thinking,
        // Operator-entered API-key settings — the same trust level as the
        // desktop settings modal.
        dial_policy: EndpointDialPolicy::Trusted,
    };
    // Low-frequency lifecycle line: release mobile builds emit no logs at
    // all, which turned the first field failure of this path into an
    // hours-long blind diagnosis. One line per design turn is cheap and
    // makes "did the loop even start, on which wire" answerable from the
    // device console.
    eprintln!(
        "openpencil-mobile: design turn start (model={}, wire={:?}, disable_thinking={})",
        cfg.model, kind, disable_thinking
    );
    // Indicator epoch BEFORE the worker can apply its first batch (desktop
    // parity): badges, frame glows, and entrance reveals adopt it.
    let epoch = op_editor_core::agent_indicators::begin_with_root_seed_hint(
        root_seed_hints.0,
        root_seed_hints.1,
    );
    let task = runtime.spawn(async move {
        // The shared loop streams into a tokio channel; forward into the
        // std receiver `ChatSession` polls (std send never blocks).
        let (tx, mut rx) = tokio::sync::mpsc::channel::<ChatDelta>(64);
        let forward = tokio::spawn(async move {
            while let Some(delta) = rx.recv().await {
                if std_tx.send(delta).is_err() {
                    break;
                }
            }
        });
        let outcome = match kind {
            BuiltinAgentKind::Anthropic => run_anthropic_agent_loop(cfg, &tx).await,
            BuiltinAgentKind::OpenAiCompat => run_openai_agent_loop(cfg, &tx).await,
        };
        // Terminal-Done contract (desktop `send_inner` parity): every exit
        // retires the bubble instead of leaving it on "Thinking…".
        match outcome {
            Ok(true) => {}
            Ok(false) => {
                let _ = tx
                    .send(ChatDelta::Done {
                        stop_reason: StopReason::EndTurn,
                    })
                    .await;
            }
            Err(error) => {
                let _ = tx.send(ChatDelta::Error(error.to_string())).await;
                let _ = tx
                    .send(ChatDelta::Done {
                        stop_reason: StopReason::Aborted,
                    })
                    .await;
            }
        }
        drop(tx);
        let _ = forward.await;
    });
    Ok(ChatTurnJob {
        session: ChatSession::from_channels(std_rx, Some(tool_rx)).into_design_loop(),
        document_identity: (0, 0),
        running_tab,
        abort: Some(task.abort_handle()),
        indicator_epoch: Some(epoch),
        page_edit_fence: None,
        document_mutated: false,
    })
}
