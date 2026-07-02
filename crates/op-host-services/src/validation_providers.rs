//! Real host-side providers for the orchestrator's Class-C vision
//! validation loop (`op_orchestrator::run_post_generation_validation`).
//!
//! These replace the `SkippedScreenshotProvider` / `SkippedVisionLlmClient`
//! stubs with implementations backed by the live render pipeline and a
//! real multimodal chat provider — but they are only injected when the
//! per-request `validation_enabled` flag is ON (see
//! [`design_session`](crate::design_session) +
//! [`web_chat_standard`](crate::web_chat_standard)). With the flag OFF
//! (the default) the stubs are used exactly as before, so the default
//! generation path is byte-for-byte unchanged.
//!
//! ## [`RealScreenshotProvider`]
//! Derives a fresh layout-resolved scene from the live (post-mutation)
//! `EditorState` via `op_pen_loader::editor_state_to_layout_scene`, then
//! renders the active page to a base64 PNG through the SAME raster
//! pipeline file exports + `debug_screenshot` use
//! (`export::screenshot::capture_scene`). No GL — the headless backend
//! links only raster skia. This is the Rust analog of the TS
//! `SkiaEngine.captureRegion` live readback.
//!
//! ## [`ChatVisionLlmClient`]
//! Wraps any `op_ai::chat_provider::ChatProvider` (the same trait the
//! chat panel + planning/sub-agent `ChatProviderLlmClient` ride) and
//! issues a single multimodal turn: the critique prompt as text + the
//! screenshot PNG as a `ChatAttachment` image. Builtin / HTTP providers
//! map the attachment onto their wire's inline image block (Anthropic
//! `image`/OpenAI `image_url`); the blocking delta iterator is drained
//! to a single `String` and returned as `VisionResponse::Text` for the
//! orchestrator's `parse_validation_response` to consume. Reuses the
//! existing request/SSE plumbing — no hand-rolled HTTP client.

use std::sync::Arc;

use op_ai::chat_provider::{
    ChatAttachment, ChatDelta, ChatProvider, ChatRequest, EffortLevel, ThinkingMode,
};
use op_editor_core::EditorState;
use op_orchestrator::{ScreenshotProvider, VisionCallRequest, VisionLlmClient, VisionResponse};

use crate::export::screenshot::{capture_scene, CaptureSpec};

/// Opt-in env flag that selects the REAL vision-validation providers over
/// the no-op stubs at the host run sites. Defaults OFF (unset) so the
/// default generation path is byte-for-byte unchanged — the stubs make
/// `run_post_generation_validation` a guaranteed short-circuit (screenshot
/// returns `None` → zero vision rounds).
///
/// This is a SEPARATE switch from `DesignRequest.validation_enabled`
/// (which only gates whether the loop runs at all, with whatever providers
/// the host injected). Flipping `validation_enabled` alone keeps the stubs,
/// so it stays a no-op; the real loop only activates when BOTH are on. The
/// flag default is NOT changed — see `default_validation_enabled` in
/// `op_orchestrator::types`.
const VISION_VALIDATION_ENV: &str = "OPENPENCIL_VISION_VALIDATION";

/// Whether the host should inject the REAL vision providers
/// (`OPENPENCIL_VISION_VALIDATION=1`). Defaults `false`.
pub fn vision_validation_enabled() -> bool {
    std::env::var(VISION_VALIDATION_ENV).is_ok_and(|v| v == "1")
}

/// Resolve the vision LLM's system prompt from the `validation` phase of
/// `op-ai-skills` (mirrors the TS `resolveSkills('validation', '')` seam
/// the `ValidationProviders.system_prompt` field documents). Concatenates
/// the matched validation-phase skill bodies; falls back to a terse
/// built-in rubric if the phase resolves empty so the vision call always
/// has a usable critique instruction.
pub fn validation_system_prompt() -> String {
    let options = op_ai_skills::ResolveOptions::default();
    let ctx = op_ai_skills::resolve_skills(op_ai_skills::Phase::Validation, "", &options);
    let knowledge = ctx
        .skills
        .iter()
        .map(|s| s.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    if knowledge.trim().is_empty() {
        DEFAULT_VALIDATION_SYSTEM_PROMPT.to_string()
    } else {
        knowledge
    }
}

/// Terse built-in critique rubric used when the `validation` phase
/// resolves no skill content. Constrains the model to the JSON verdict
/// shape `parse_validation_response` consumes (issues / fixes /
/// structuralFixes / qualityScore).
const DEFAULT_VALIDATION_SYSTEM_PROMPT: &str = "You are a UI design validator. \
You are given a screenshot of a generated design and its node-tree structure. \
Identify concrete visual issues (overlap, clipping, low contrast, broken spacing, \
misaligned or missing elements) and return ONLY a JSON object of the shape \
{\"issues\":[string],\"fixes\":[{\"nodeId\":string,\"property\":string,\"value\":any}],\
\"structuralFixes\":[{\"action\":\"removeNode\"|\"addChild\",...}],\"qualityScore\":number} \
where qualityScore is 1-10. Use the real node IDs from the provided tree. \
Return an empty fixes array and a high qualityScore when the design looks good.";

/// Real `ScreenshotProvider` — renders the live document's active page
/// to a base64 PNG via the headless raster pipeline.
///
/// The validation loop calls `capture_root_frame(sink.state())` AFTER
/// each round's fixes are applied, so the captured pixels reflect the
/// current mutated document (not a stale pre-loop snapshot).
pub struct RealScreenshotProvider;

impl ScreenshotProvider for RealScreenshotProvider {
    fn capture_root_frame(&self, state: &EditorState) -> Option<String> {
        // Same derive the canvas painter + debug_screenshot use, so the
        // pixels match what the user sees at zoom 1.
        let scene = op_pen_loader::editor_state_to_layout_scene(state);
        let spec = CaptureSpec {
            // `None` = whole active-page content (the "root frame" the
            // validation loop wants).
            node_id: None,
            padding: 0.0,
            scale: 1.0,
        };
        // A render failure (empty page, oversized bounds) downgrades to
        // `None` — the loop then skips the vision round, exactly like the
        // stub. Capture errors must never abort the design turn.
        capture_scene(&scene, &spec)
            .ok()
            .map(|shot| shot.png_base64)
    }
}

/// Real `VisionLlmClient` — issues a single multimodal turn through a
/// `ChatProvider`, sending the screenshot as an image attachment + the
/// critique prompt as text, and returns the model's reply text.
///
/// `Arc<dyn ChatProvider>` so the host can share one provider instance
/// (the same one driving the design turn's `ChatProviderLlmClient`).
pub struct ChatVisionLlmClient {
    provider: Arc<dyn ChatProvider>,
    /// Vision-capable model id (forwarded into the `ChatRequest`); `None`
    /// keeps the provider's own default.
    model: Option<String>,
}

impl ChatVisionLlmClient {
    pub fn new(provider: Arc<dyn ChatProvider>) -> Self {
        Self {
            provider,
            model: None,
        }
    }

    /// Attach the vision model id to every request this client issues.
    pub fn with_model(mut self, model: Option<String>) -> Self {
        self.model = model;
        self
    }

    /// Decode the screenshot base64 into raw PNG bytes for the
    /// `ChatAttachment`. Providers re-encode (Anthropic image block) or
    /// data-URL it as their wire demands.
    fn screenshot_attachment(image_base64: &str) -> Option<ChatAttachment> {
        use base64::Engine as _;
        let data = base64::engine::general_purpose::STANDARD
            .decode(image_base64)
            .ok()?;
        Some(ChatAttachment {
            name: "design-screenshot.png".to_string(),
            media_type: "image/png".to_string(),
            data,
        })
    }
}

impl VisionLlmClient for ChatVisionLlmClient {
    fn validate(&self, req: VisionCallRequest) -> VisionResponse {
        // A malformed / empty screenshot string can't drive a vision
        // call — skip rather than send a text-only critique that would
        // hallucinate against no image.
        let Some(attachment) = Self::screenshot_attachment(&req.image_base64) else {
            return VisionResponse::Skipped {
                reason: Some("screenshot was not valid base64 PNG".to_string()),
            };
        };

        // Inline the vision system prompt into the user message: the
        // CLI-backed `ChatProvider` impls ignore `ChatRequest.system_prompt`
        // (no per-turn system slot), so putting it in the field would
        // silently drop the critique rubric. Same prepend the planning /
        // sub-agent `ChatProviderLlmClient` uses.
        let user_message = if req.system.is_empty() {
            req.message.clone()
        } else {
            format!("{}\n\n---\n\n{}", req.system, req.message)
        };

        let chat_req = ChatRequest {
            system_prompt: String::new(),
            user_message,
            history: Vec::new(),
            // Validation replies are compact JSON critiques — a modest
            // budget is plenty and keeps the call cheap.
            max_output_tokens: 4096,
            // The validation rubric wants a deterministic JSON verdict,
            // not extended reasoning.
            thinking: ThinkingMode::Disabled,
            effort: EffortLevel::Low,
            attachments: vec![attachment],
            model: self.model.clone().or_else(|| req.model.clone()),
        };

        // `provider.send` is a blocking delta iterator (the same shape the
        // chat session + `ChatProviderLlmClient` drain). Accumulate the
        // text deltas into one reply string. An `Error` delta aborts to
        // `Skipped` so a transport failure can't crash the design turn.
        let mut text = String::new();
        for delta in self.provider.send(chat_req) {
            match delta {
                ChatDelta::TextDelta(s) => text.push_str(&s),
                // Thinking tokens are not part of the JSON verdict.
                ChatDelta::Thinking(_) => {}
                ChatDelta::Error(msg) => {
                    return VisionResponse::Skipped {
                        reason: Some(format!("vision provider error: {msg}")),
                    };
                }
                // `Done` closes the turn; tool-use is not expected here.
                ChatDelta::Done { .. } => break,
                ChatDelta::ToolUse { .. } => {}
            }
        }

        if text.trim().is_empty() {
            VisionResponse::Skipped {
                reason: Some("vision provider returned no text".to_string()),
            }
        } else {
            VisionResponse::Text(text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_ai::chat_provider::StopReason;
    use op_editor_core::EditorState;
    use std::sync::Mutex;

    // ── RealScreenshotProvider ───────────────────────────────────────────────

    /// An empty document has no page content → the real provider returns
    /// `None` (loop skips the round), never panics.
    #[test]
    fn real_screenshot_provider_returns_none_on_empty_doc() {
        let state = EditorState::new();
        assert!(RealScreenshotProvider.capture_root_frame(&state).is_none());
    }

    /// A document with real content renders to a base64 PNG payload.
    #[test]
    fn real_screenshot_provider_renders_png_for_a_populated_doc() {
        let doc: jian_ops_schema::PenDocument = serde_json::from_str(
            r##"{
                "version":"1.0",
                "children":[
                    {"type":"frame","id":"root","name":"Root",
                     "x":0,"y":0,"width":200,"height":120,
                     "fill":[{"type":"solid","color":"#ffffff"}],
                     "children":[
                        {"type":"rectangle","id":"r1","name":"Box",
                         "x":10,"y":10,"width":80,"height":40,
                         "fill":[{"type":"solid","color":"#3b82f6"}]}
                     ]}
                ]
            }"##,
        )
        .expect("fixture parses");
        let state = EditorState::from_document(doc);
        let b64 = RealScreenshotProvider
            .capture_root_frame(&state)
            .expect("populated doc renders a screenshot");
        use base64::Engine as _;
        let png = base64::engine::general_purpose::STANDARD
            .decode(&b64)
            .expect("valid base64");
        assert_eq!(
            &png[..4],
            &[0x89, b'P', b'N', b'G'],
            "must be a PNG payload"
        );
    }

    // ── ChatVisionLlmClient ──────────────────────────────────────────────────

    /// A scripted provider that records the `ChatRequest` it received and
    /// replies with a fixed JSON verdict.
    struct RecordingVisionProvider {
        seen: Arc<Mutex<Vec<ChatRequest>>>,
        reply: String,
    }

    impl ChatProvider for RecordingVisionProvider {
        fn provider_label(&self) -> &str {
            "recording-vision"
        }
        fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
            self.seen.lock().unwrap().push(request);
            Box::new(
                vec![
                    ChatDelta::TextDelta(self.reply.clone()),
                    ChatDelta::Done {
                        stop_reason: StopReason::EndTurn,
                    },
                ]
                .into_iter(),
            )
        }
    }

    fn b64_png() -> String {
        use base64::Engine as _;
        // 4-byte PNG magic is enough to prove the attachment decodes.
        base64::engine::general_purpose::STANDARD.encode([0x89, b'P', b'N', b'G'])
    }

    fn vision_req(image_base64: &str) -> VisionCallRequest {
        VisionCallRequest {
            system: "You are a design validator. Return JSON.".into(),
            message: "Analyze this screenshot.".into(),
            image_base64: image_base64.to_string(),
            model: Some("vision-model".into()),
            provider: None,
            timeout: std::time::Duration::from_secs(30),
        }
    }

    /// The real client sends the screenshot as an image attachment, inlines
    /// the system prompt into the user message, and returns the reply text.
    #[test]
    fn chat_vision_client_sends_image_attachment_and_returns_text() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let provider = Arc::new(RecordingVisionProvider {
            seen: seen.clone(),
            reply: r#"{"issues":[],"fixes":[],"qualityScore":9}"#.into(),
        });
        let client = ChatVisionLlmClient::new(provider).with_model(Some("vision-model".into()));

        let resp = client.validate(vision_req(&b64_png()));

        // Reply text surfaced verbatim for parse_validation_response.
        match resp {
            VisionResponse::Text(t) => {
                assert!(t.contains("qualityScore"), "got: {t}");
            }
            VisionResponse::Skipped { reason } => panic!("unexpected skip: {reason:?}"),
        }

        // The request carried exactly one image attachment with the PNG bytes.
        let reqs = seen.lock().unwrap();
        let r = reqs.first().expect("provider was called");
        assert_eq!(r.attachments.len(), 1, "one screenshot attachment");
        assert!(r.attachments[0].is_image(), "attachment is an image");
        assert_eq!(r.attachments[0].media_type, "image/png");
        assert_eq!(&r.attachments[0].data[..4], &[0x89, b'P', b'N', b'G']);
        // System prompt inlined into the user message (CLI providers ignore
        // the system field).
        assert!(
            r.user_message.contains("design validator"),
            "system prompt inlined; got: {}",
            r.user_message
        );
        assert!(r.user_message.contains("Analyze this screenshot"));
        assert_eq!(r.model.as_deref(), Some("vision-model"));
    }

    /// A non-base64 screenshot string can't drive a vision call → Skipped.
    #[test]
    fn chat_vision_client_skips_on_bad_base64() {
        let provider = Arc::new(RecordingVisionProvider {
            seen: Arc::new(Mutex::new(Vec::new())),
            reply: "{}".into(),
        });
        let client = ChatVisionLlmClient::new(provider);
        let resp = client.validate(vision_req("@@@ not base64 @@@"));
        assert!(matches!(resp, VisionResponse::Skipped { .. }));
    }

    /// An `Error` delta downgrades to `Skipped` (never crashes the turn).
    #[test]
    fn chat_vision_client_skips_on_provider_error() {
        struct ErroringProvider;
        impl ChatProvider for ErroringProvider {
            fn provider_label(&self) -> &str {
                "erroring"
            }
            fn send(&self, _r: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
                Box::new(
                    vec![
                        ChatDelta::Error("quota exhausted".into()),
                        ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        },
                    ]
                    .into_iter(),
                )
            }
        }
        let client = ChatVisionLlmClient::new(Arc::new(ErroringProvider));
        let resp = client.validate(vision_req(&b64_png()));
        match resp {
            VisionResponse::Skipped { reason } => {
                assert!(reason.unwrap().contains("quota exhausted"));
            }
            VisionResponse::Text(_) => panic!("error should downgrade to Skipped"),
        }
    }

    /// An empty reply downgrades to `Skipped` (parse_validation_response
    /// would otherwise treat "" as a parse failure anyway).
    #[test]
    fn chat_vision_client_skips_on_empty_reply() {
        let provider = Arc::new(RecordingVisionProvider {
            seen: Arc::new(Mutex::new(Vec::new())),
            reply: "   ".into(),
        });
        let client = ChatVisionLlmClient::new(provider);
        let resp = client.validate(vision_req(&b64_png()));
        assert!(matches!(resp, VisionResponse::Skipped { .. }));
    }

    // ── End-to-end wiring test ───────────────────────────────────────────────
    //
    // Drives `op_orchestrator::run_post_generation_validation` with the REAL
    // host providers (`RealScreenshotProvider` renders an actual PNG of the
    // live document; `ChatVisionLlmClient` runs a fake-but-real-shaped
    // `ChatProvider`). Proves the wiring is LIVE end-to-end: screenshot →
    // vision → a safe fix lands on the live document. The default-off variant
    // proves the stubs keep the loop a no-op (default path unchanged).

    use op_orchestrator::DocSink;

    /// Minimal `DocSink` over a live `EditorState` (mirrors
    /// `pre_validator::tests::TestSink`).
    struct LiveSink {
        editor: EditorState,
    }
    impl DocSink for LiveSink {
        fn state(&self) -> &EditorState {
            &self.editor
        }
        fn apply(&mut self, cmd: op_editor_core::EditorCommand) -> bool {
            self.editor.apply(cmd)
        }
        fn begin_undo_batch(&mut self) {}
        fn end_undo_batch(&mut self) {}
    }

    /// A `ChatProvider` that returns a fixed JSON validation verdict and
    /// records how many times it was asked (proves the real vision client
    /// actually called the provider, i.e. the loop reached the vision step).
    struct ScriptedVisionProvider {
        calls: Arc<std::sync::atomic::AtomicUsize>,
        reply: String,
    }
    impl ChatProvider for ScriptedVisionProvider {
        fn provider_label(&self) -> &str {
            "scripted-vision"
        }
        fn send(&self, _r: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Box::new(
                vec![
                    ChatDelta::TextDelta(self.reply.clone()),
                    ChatDelta::Done {
                        stop_reason: StopReason::EndTurn,
                    },
                ]
                .into_iter(),
            )
        }
    }

    /// Build a renderable doc whose active page holds >= 30 nodes (clears the
    /// `VALIDATION_NODE_COUNT_THRESHOLD = 30` gate) including a frame named
    /// `target` the vision verdict will fix.
    fn doc_over_threshold() -> jian_ops_schema::PenDocument {
        let mut children = String::new();
        // The fix target: a frame with cornerRadius 0 the verdict bumps to 24.
        children.push_str(
            r##"{"type":"frame","id":"target","name":"Target",
                "x":10,"y":10,"width":120,"height":48,"cornerRadius":0,
                "fill":[{"type":"solid","color":"#3b82f6"}]}"##,
        );
        // Pad to >= 30 total nodes (1 root + 1 target + 30 fillers).
        for i in 0..30 {
            children.push_str(&format!(
                r##",{{"type":"rectangle","id":"r{i}","name":"R{i}",
                    "x":0,"y":{y},"width":40,"height":20,
                    "fill":[{{"type":"solid","color":"#e2e8f0"}}]}}"##,
                y = 60 + i * 24
            ));
        }
        let json = format!(
            r##"{{"version":"1.0","children":[
                {{"type":"frame","id":"root","name":"Root",
                  "x":0,"y":0,"width":200,"height":900,
                  "fill":[{{"type":"solid","color":"#ffffff"}}],
                  "children":[{children}]}}
            ]}}"##
        );
        serde_json::from_str(&json).expect("fixture parses")
    }

    fn request_with_validation(enabled: bool) -> op_orchestrator::DesignRequest {
        op_orchestrator::DesignRequest {
            prompt: "p".into(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 1,
            append_context: None,
            validation_enabled: enabled,
            visual_ref_enabled: false,
        }
    }

    /// REAL providers + validation_enabled=true → the loop captures a real
    /// PNG, calls the vision provider, and applies the cornerRadius safe fix
    /// to the live document. (The vision call only happens because the real
    /// screenshot returned `Some(png)` — proving the screenshot wiring is
    /// live, not the stub's `None`.)
    #[test]
    fn real_providers_drive_loop_capture_vision_and_apply_fix() {
        let mut sink = LiveSink {
            editor: EditorState::from_document(doc_over_threshold()),
        };

        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        // qualityScore 5 (< threshold 8) so the loop proceeds to apply fixes.
        let provider = Arc::new(ScriptedVisionProvider {
            calls: calls.clone(),
            reply: r#"{"issues":["target corner radius too sharp"],
                       "fixes":[{"nodeId":"target","property":"cornerRadius","value":24}],
                       "structuralFixes":[],"qualityScore":5}"#
                .into(),
        });

        let screenshot = RealScreenshotProvider;
        let vision = ChatVisionLlmClient::new(provider);
        let pre_validator = op_orchestrator::SkippedPreValidator;
        let abort = op_orchestrator::AbortFlag::new();
        let mut events: Vec<op_orchestrator::Progress> = Vec::new();

        let summary = op_orchestrator::run_post_generation_validation(
            &mut sink,
            &pre_validator,
            &screenshot,
            &vision,
            "validator system prompt",
            &request_with_validation(true),
            &mut |p| events.push(p),
            &abort,
        )
        .expect("loop runs");

        // The vision provider was actually called — proves screenshot → vision
        // wiring is live (stub would have returned None and skipped this).
        assert!(
            calls.load(std::sync::atomic::Ordering::SeqCst) >= 1,
            "real vision provider must have been called at least once"
        );
        // At least one fix was applied to the live document.
        assert!(
            summary.total_applied >= 1,
            "expected the cornerRadius fix to apply, got {}",
            summary.total_applied
        );
        assert!(summary.rounds_run >= 1, "at least one vision round ran");

        // The fix actually LANDED on the live document: the target frame's
        // container cornerRadius is now Uniform(24). Reading the concrete
        // post-mutation value (not just the applied count) proves the fix
        // wrote through the real sink, not a counter that lied.
        let target = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &op_editor_core::node_id::NodeId::new("target"),
        )
        .expect("target node present");
        match target {
            jian_ops_schema::node::PenNode::Frame(f) => {
                use jian_ops_schema::node::container::CornerRadius;
                assert!(
                    matches!(f.container.corner_radius, Some(CornerRadius::Uniform(r)) if (r - 24.0).abs() < 1e-6),
                    "cornerRadius fix must be written to the live document, got {:?}",
                    f.container.corner_radius
                );
            }
            other => panic!("target is not a frame: {other:?}"),
        }

        // Progress stream reached a real vision round.
        assert!(
            events
                .iter()
                .any(|p| matches!(p, op_orchestrator::Progress::ValidationRoundStarted { .. })),
            "a vision round must have started"
        );
    }

    /// Default-off proof: the STUB providers (selected when
    /// `OPENPENCIL_VISION_VALIDATION` is unset) keep the loop a no-op even at
    /// node_count >= 30 with validation_enabled=true — zero rounds, document
    /// untouched. This is the byte-for-byte default path.
    #[test]
    fn stub_providers_keep_loop_a_noop_even_above_threshold() {
        let mut sink = LiveSink {
            editor: EditorState::from_document(doc_over_threshold()),
        };
        let before = sink.state().active_children().len();

        let screenshot = op_orchestrator::SkippedScreenshotProvider;
        let vision = op_orchestrator::SkippedVisionLlmClient;
        let pre_validator = op_orchestrator::SkippedPreValidator;
        let abort = op_orchestrator::AbortFlag::new();
        let mut events: Vec<op_orchestrator::Progress> = Vec::new();

        let summary = op_orchestrator::run_post_generation_validation(
            &mut sink,
            &pre_validator,
            &screenshot,
            &vision,
            "",
            &request_with_validation(true),
            &mut |p| events.push(p),
            &abort,
        )
        .expect("loop runs");

        assert_eq!(summary.total_applied, 0, "stub loop applies nothing");
        assert_eq!(summary.rounds_run, 0, "stub loop runs zero vision rounds");
        // Document untouched — cornerRadius stays 0, node count unchanged.
        assert_eq!(sink.state().active_children().len(), before);
        let target = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &op_editor_core::node_id::NodeId::new("target"),
        )
        .expect("target node present");
        match target {
            jian_ops_schema::node::PenNode::Frame(f) => {
                use jian_ops_schema::node::container::CornerRadius;
                assert!(
                    matches!(f.container.corner_radius, Some(CornerRadius::Uniform(r)) if r == 0.0),
                    "stub path must NOT mutate cornerRadius, got {:?}",
                    f.container.corner_radius
                );
            }
            other => panic!("target is not a frame: {other:?}"),
        }
        // No vision round ever started (screenshot stub returned None first).
        assert!(
            !events
                .iter()
                .any(|p| matches!(p, op_orchestrator::Progress::ValidationRoundDone { .. })),
            "no vision round should complete on the stub path"
        );
    }
}
