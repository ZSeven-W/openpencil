//! Rescue and terminal fallback paths for [`CodegenPipeline`].

use super::{CodegenPipeline, InFlight, Phase};
use crate::ai::deterministic_fallback;
use crate::ai::parse::clean_code;
use crate::ai::pipeline_preflight::{model_output_failure, MAX_USER_PROMPT_BYTES};
use crate::ai::prompts::rescue_request;
use crate::ai::types::{PipelineStep, RequestKind};

impl CodegenPipeline {
    pub(super) fn step_rescue(&mut self) -> PipelineStep {
        if let Some((_id, flight)) = self.take_settled_inflight() {
            return self.resolve_rescue(flight);
        }
        if self.has_inflight() {
            return PipelineStep::Waiting;
        }
        let asset_paths = self
            .assets
            .iter()
            .map(|asset| asset.relative_path.clone())
            .collect::<Vec<_>>();
        let failure_summary = self.failure_summary();
        let id = self.register_inflight(RequestKind::Assembly);
        let req = rescue_request(
            id,
            &self.sanitized_nodes_json,
            &failure_summary,
            &self.input,
            &asset_paths,
        );
        if req.user_message.len() > MAX_USER_PROMPT_BYTES {
            self.in_flight.remove(&id);
            self.record_failure(
                "single-file rescue",
                format!(
                    "prompt is {} bytes (limit {MAX_USER_PROMPT_BYTES}); request was not dispatched",
                    req.user_message.len()
                ),
            );
            return self.finish_with_deterministic_fallback();
        }
        self.assembly_done = Some(false);
        PipelineStep::Dispatch(vec![req])
    }

    fn resolve_rescue(&mut self, flight: InFlight) -> PipelineStep {
        if let Some(message) = flight.error {
            self.record_failure("single-file rescue", message);
            return self.finish_with_deterministic_fallback();
        }
        let code = clean_code(&flight.buffer);
        if let Some(message) = model_output_failure(self.input.framework, &code) {
            self.record_failure("single-file rescue", message);
            return self.finish_with_deterministic_fallback();
        }
        self.assembly_done = Some(true);
        let step = PipelineStep::Done {
            code,
            degraded: true,
            assets: self.assets.clone(),
        };
        self.phase = Phase::Terminal(step.clone());
        step
    }

    fn finish_with_deterministic_fallback(&mut self) -> PipelineStep {
        self.finish_deterministic(true)
    }

    /// Deterministic targets never dispatch a model request: the first
    /// `step()` generates the whole file and reports every phase done.
    pub(super) fn finish_deterministic_target(&mut self) -> PipelineStep {
        self.planning_done = Some(true);
        self.finish_deterministic(false)
    }

    fn finish_deterministic(&mut self, degraded: bool) -> PipelineStep {
        let context = deterministic_fallback::DocumentContext {
            variables_json: self.input.variables_json.as_deref(),
            themes_json: self.input.themes_json.as_deref(),
            components_json: self.input.components_json.as_deref(),
        };
        match deterministic_fallback::generate(
            &self.sanitized_nodes_json,
            context,
            self.input.framework,
        ) {
            Ok(code) => {
                self.assembly_done = Some(true);
                let step = PipelineStep::Done {
                    code,
                    degraded,
                    assets: self.assets.clone(),
                };
                self.phase = Phase::Terminal(step.clone());
                step
            }
            Err(error) => {
                // `record_failure` takes `impl AsRef<str>`; render the typed
                // error to keep the recorded text byte-identical.
                if degraded {
                    self.record_failure("deterministic fallback", error.to_string());
                    self.fail_with_history("Code generation failed after every fallback")
                } else {
                    self.record_failure("deterministic generator", error.to_string());
                    self.fail_with_history("Code generation failed")
                }
            }
        }
    }

    pub(super) fn record_failure(&mut self, stage: impl AsRef<str>, reason: impl AsRef<str>) {
        self.failures
            .push(format!("{}: {}", stage.as_ref(), reason.as_ref()));
    }

    fn failure_summary(&self) -> String {
        if self.failures.is_empty() {
            return "- no usable intermediate output".to_string();
        }
        self.failures
            .iter()
            .map(|failure| format!("- {failure}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn fail_with_history(&mut self, heading: &str) -> PipelineStep {
        self.assembly_done = Some(false);
        let step = PipelineStep::Failed {
            message: format!("{heading}. Failure history:\n{}", self.failure_summary()),
        };
        self.phase = Phase::Terminal(step.clone());
        step
    }
}
