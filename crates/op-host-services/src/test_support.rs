use futures::stream;
use op_orchestrator::{CallRequest, LlmChunk, LlmClient, LlmError};

/// Shared scripted response used by design.md and reference-context tests.
pub(crate) struct ScriptedLlm;

impl LlmClient for ScriptedLlm {
    fn call(
        &self,
        _req: CallRequest,
    ) -> futures::stream::BoxStream<'static, Result<LlmChunk, LlmError>> {
        Box::pin(stream::iter(vec![Ok(LlmChunk::Text(
            "```markdown\n# Design System: Food App\n\n## 1. Visual Theme & Atmosphere\nWarm, compact mobile ordering UI.\n\n## 2. Color Palette & Roles\n- **Flame Orange** (#FF5A1F) — Primary action\n\n## 5. Layout Principles\nUse a sibling/root screen beside the existing app page.\n```"
                .to_string(),
        ))]))
    }
}
