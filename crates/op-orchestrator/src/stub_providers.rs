//! Production-visible stub provider implementations for the vision
//! validation pipeline (S3c).
//!
//! Hosts use these when the real implementations are not yet wired —
//! `op-design-lint` integration is pending host wiring, screenshot
//! capture is awaiting `jian-skia::captureRegion` exposure, and the
//! multimodal vision LLM client has no Rust crate yet. With these
//! stubs `run_post_generation_validation` becomes a guaranteed
//! short-circuit:
//!
//! - `SkippedPreValidator::run_pre_validation_fixes` → zero fixes,
//!   no side effects.
//! - `SkippedScreenshotProvider::capture_root_frame` → `None`, so the
//!   loop exits before any vision round.
//! - `SkippedVisionLlmClient::validate` → `VisionResponse::Skipped`
//!   (defensive in case `capture_root_frame` ever returns `Some`).
//!
//! Net result: the Progress stream emits `ValidationStarted` →
//! `ValidationPreCheckDone { applied: 0, .. }` →
//! `ValidationDone { total_applied: 0 }` and the document is untouched.
//! Hosts can ship the orchestrator end-to-end and swap real
//! implementations in piecewise.

use crate::types::{
    DocSink, PreValidationResult, PreValidator, ScreenshotProvider, VisionCallRequest,
    VisionLlmClient, VisionResponse,
};

/// Stub `PreValidator` — always returns zero fixes; no side effects.
pub struct SkippedPreValidator;

impl PreValidator for SkippedPreValidator {
    fn run_pre_validation_fixes(&self, _sink: &mut dyn DocSink) -> PreValidationResult {
        PreValidationResult::default()
    }
}

/// Stub `ScreenshotProvider` — always returns `None`. Vision loop
/// exits after pre-validation with zero rounds.
pub struct SkippedScreenshotProvider;

impl ScreenshotProvider for SkippedScreenshotProvider {
    fn capture_root_frame(&self) -> Option<String> {
        None
    }
}

/// Stub `VisionLlmClient` — always returns `VisionResponse::Skipped`.
pub struct SkippedVisionLlmClient;

impl VisionLlmClient for SkippedVisionLlmClient {
    fn validate(&self, _req: VisionCallRequest) -> VisionResponse {
        VisionResponse::Skipped { reason: None }
    }
}
