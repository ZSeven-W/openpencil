//! Background worker for a side-by-side design-directions ("variants") turn.
//!
//! The same channel-shaped [`DesignSession`] a single design turn uses —
//! the host's `pump_commands` / `pump_progress` drain it unchanged — but
//! the worker drives `op_orchestrator::variants_run::run_design_variants`:
//! N complete orchestrator runs, concurrently, each pinned to its own style
//! guide and built into a private document, landing on the live page (via
//! the `RemoteDocSink`) as each one finishes.

use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread;

use op_ai::chat_provider::ChatProvider;
use op_editor_core::EditorState;
use op_editor_host_core::design::{DesignCmdReq, DesignDelta, DesignSession, RemoteDocSink};
use op_orchestrator::variants::VariantPlan;
use op_orchestrator::{
    AbortFlag, DesignRequest, DocSink, LlmClient, OrchestratorError, Progress, RunSummary,
    SkippedScreenshotProvider, SkippedVisionLlmClient, ValidationProviders,
};

use crate::chat_runtime::block_on_anywhere;
use crate::pre_validator::LintPreValidator;
use crate::validation_providers::{
    validation_system_prompt, vision_validation_enabled, ChatVisionLlmClient,
    RealScreenshotProvider,
};

/// Spawn a worker that generates `plans.len()` directions of `request`
/// side by side. `vision_provider` has the same meaning as in
/// [`crate::design_session::start`]: the Class-C vision loop runs per
/// direction only when it is supplied AND `OPENPENCIL_VISION_VALIDATION=1`.
pub fn start_variants<L: LlmClient + Send + 'static>(
    llm: L,
    request: DesignRequest,
    plans: Vec<VariantPlan>,
    initial_state: EditorState,
    vision_provider: Option<Arc<dyn ChatProvider>>,
) -> DesignSession {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();

    let indicator_epoch = op_editor_core::agent_indicators::begin();
    let abort = AbortFlag::new();
    let worker_abort = abort.clone();

    let failure_tx = delta_tx.clone();
    if let Err(err) = thread::Builder::new()
        .name("op-design-variants".into())
        .spawn(move || {
            run_variants_worker(
                llm,
                request,
                plans,
                initial_state,
                delta_tx,
                cmd_tx,
                indicator_epoch,
                worker_abort,
                vision_provider,
            )
        })
    {
        eprintln!("[design-variants] failed to spawn op-design-variants thread: {err}");
        op_editor_core::agent_indicators::end_if_epoch(indicator_epoch);
        let _ = failure_tx.send(DesignDelta::Done(Err(
            op_orchestrator::OrchestratorError::Internal(format!(
                "failed to spawn design variants worker thread: {err}"
            )),
        )));
    }

    DesignSession::from_channels_with_epoch_and_abort(delta_rx, cmd_rx, indicator_epoch, abort)
}

/// Body of [`start_variants`]'s worker thread.
#[allow(clippy::too_many_arguments)]
fn run_variants_worker<L: LlmClient + Send>(
    llm: L,
    request: DesignRequest,
    plans: Vec<VariantPlan>,
    initial_state: EditorState,
    delta_tx: Sender<DesignDelta>,
    cmd_tx: Sender<DesignCmdReq>,
    indicator_epoch: u64,
    abort: AbortFlag,
    vision_provider: Option<Arc<dyn ChatProvider>>,
) {
    let mut sink = RemoteDocSink::new(cmd_tx, initial_state.clone());
    let summary = {
        let mut on_progress = |event: Progress| {
            let _ = delta_tx.send(DesignDelta::Progress(event));
        };
        drive_design_variants(
            VariantsRun {
                request: &request,
                plans: &plans,
                base_state: &initial_state,
                abort: &abort,
                indicator_epoch: Some(indicator_epoch),
                vision_provider,
            },
            &llm,
            &mut sink,
            &mut on_progress,
        )
    };
    let _ = delta_tx.send(DesignDelta::Done(summary));
}

/// Localize the plans' display names (`方案 A`, `Direction A`, …). The
/// name is also stamped on every board a direction lands, so each host
/// localizes with the locale of the user who asked for the run.
pub fn localize_variant_plans(plans: &mut [VariantPlan], locale: op_editor_core::Locale) {
    for plan in plans {
        let letter = op_editor_core::variant_letter(plan.index).to_string();
        plan.name =
            op_i18n::translate_with(locale, "workspace.variants.name", &[("letter", &letter)]);
    }
}

/// Everything one variants run is about, apart from where its boards and
/// progress go.
pub struct VariantsRun<'a> {
    pub request: &'a DesignRequest,
    pub plans: &'a [VariantPlan],
    /// The document the directions start from (each gets a private copy
    /// with the active page emptied).
    pub base_state: &'a EditorState,
    pub abort: &'a AbortFlag,
    /// The host's agent-indicator epoch, adopted by every direction.
    pub indicator_epoch: Option<u64>,
    /// Same meaning as in [`start_variants`]: the Class-C vision loop runs
    /// per direction only when supplied AND `OPENPENCIL_VISION_VALIDATION=1`.
    pub vision_provider: Option<Arc<dyn ChatProvider>>,
}

/// Run every direction to completion on the calling thread, landing each
/// one through `sink` as it finishes. The desktop worker above lands
/// through its `RemoteDocSink`; the serve-web daemon lands straight into
/// the document it serves.
pub fn drive_design_variants(
    run: VariantsRun<'_>,
    llm: &dyn LlmClient,
    sink: &mut dyn DocSink,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<RunSummary, OrchestratorError> {
    let VariantsRun {
        request,
        plans,
        base_state,
        abort,
        indicator_epoch,
        vision_provider,
    } = run;
    let pre_validator = LintPreValidator;
    let real = vision_provider
        .filter(|_| vision_validation_enabled())
        .map(|provider| {
            (
                RealScreenshotProvider,
                ChatVisionLlmClient::new(provider).with_model(request.model.clone()),
                validation_system_prompt(),
            )
        });
    let stub_screenshot = SkippedScreenshotProvider;
    let stub_vision = SkippedVisionLlmClient;
    let (screenshot, vision, system_prompt): (
        &dyn op_orchestrator::ScreenshotProvider,
        &dyn op_orchestrator::VisionLlmClient,
        String,
    ) = match &real {
        Some((shot, vis, prompt)) => (shot, vis, prompt.clone()),
        None => (&stub_screenshot, &stub_vision, String::new()),
    };
    let providers = ValidationProviders {
        pre_validator: &pre_validator,
        screenshot,
        vision,
        system_prompt,
    };
    block_on_anywhere(op_orchestrator::variants_run::run_design_variants(
        request,
        plans,
        base_state,
        llm,
        sink,
        on_progress,
        abort,
        &providers,
        indicator_epoch,
    ))
}
