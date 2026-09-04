//! `ChatProvider` adapter and transport-level cancellation for Claude Code.

use super::*;

pub(super) fn unexpected_stream_end_deltas() -> [ChatDelta; 2] {
    [
        ChatDelta::Error("claude stream ended before a terminal result".into()),
        ChatDelta::Done {
            stop_reason: StopReason::Aborted,
        },
    ]
}

impl ChatProvider for ClaudeCodeProvider {
    fn provider_label(&self) -> &str {
        &self.label
    }

    fn supports_cancellable_send(&self) -> bool {
        true
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.send_inner(request, None)
    }

    fn send_cancellable(
        &self,
        request: ChatRequest,
        cancel: Arc<AtomicBool>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.send_inner(request, Some(cancel))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_ai::chat_provider::{ChatHistoryRole, EffortLevel, ThinkingMode};

    #[test]
    fn effective_options_threads_system_prompt_without_global_resume() {
        use anthropic_agent_sdk::types::options::SystemPrompt;
        let request = ChatRequest {
            system_prompt: "You are a design assistant.".into(),
            ..Default::default()
        };
        let options = effective_options(None, &request);
        assert!(
            matches!(
                options.system_prompt,
                Some(SystemPrompt::String(ref value)) if value == "You are a design assistant."
            ),
            "per-turn system prompt must ride options.system_prompt"
        );
        assert!(
            options.resume.is_none(),
            "chat context must not ride a process-global Claude session"
        );
    }

    #[test]
    fn request_history_keeps_chat_tabs_isolated() {
        let tab_a = ChatRequest {
            history: vec![(ChatHistoryRole::User, "tab-a-private-context".into())],
            ..Default::default()
        };
        let tab_b = ChatRequest {
            history: vec![(ChatHistoryRole::User, "tab-b-private-context".into())],
            ..Default::default()
        };
        let prompt_a = prompt_with_request_history(&tab_a, "next-a".into());
        let prompt_b = prompt_with_request_history(&tab_b, "next-b".into());

        assert!(prompt_a.contains("tab-a-private-context"));
        assert!(!prompt_a.contains("tab-b-private-context"));
        assert!(prompt_b.contains("tab-b-private-context"));
        assert!(!prompt_b.contains("tab-a-private-context"));
    }

    #[test]
    fn claude_is_abortable_but_not_safe_for_untrusted_evidence() {
        let provider = ClaudeCodeProvider::new();
        assert!(provider.supports_cancellable_send());
        assert!(!provider.supports_evidence_only_send());
    }

    #[test]
    fn options_env_never_carries_path() {
        // The SDK rejects PATH in options.env as dangerous; GUI PATH repair
        // happens process-wide before the provider is constructed.
        let request = ChatRequest {
            system_prompt: "design something".into(),
            user_message: "hi".into(),
            history: vec![],
            max_output_tokens: 1024,
            thinking: ThinkingMode::Adaptive,
            effort: EffortLevel::Low,
            attachments: vec![],
            model: None,
        };
        let options = effective_options(None, &request);
        assert!(
            !options.env.contains_key("PATH"),
            "PATH must never ride options.env: {:?}",
            options.env.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn unexpected_stream_end_aborts_instead_of_ending_turn() {
        let deltas = unexpected_stream_end_deltas();
        assert!(matches!(
            deltas.first(),
            Some(ChatDelta::Error(message)) if message.contains("terminal result")
        ));
        assert!(matches!(
            deltas.get(1),
            Some(ChatDelta::Done {
                stop_reason: StopReason::Aborted
            })
        ));
        assert!(!deltas.iter().any(|delta| matches!(
            delta,
            ChatDelta::Done {
                stop_reason: StopReason::EndTurn
            }
        )));
    }
}
