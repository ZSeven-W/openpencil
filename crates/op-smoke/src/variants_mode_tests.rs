use super::*;

use futures::stream::BoxStream;
use op_editor_core::EditorState;
use op_orchestrator::{
    CallRequest, LlmChunk, LlmError, SkippedPreValidator, SkippedScreenshotProvider,
    SkippedVisionLlmClient,
};

use crate::InlineDocSink;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn plain_prompt_has_no_variants() {
    assert_eq!(
        parse_smoke_args(&args(&["做一个登录页"])),
        Ok(SmokeArgs {
            variants: None,
            prompt: "做一个登录页".into()
        })
    );
}

#[test]
fn variants_flag_accepts_both_spellings_and_any_position() {
    let expected = Ok(SmokeArgs {
        variants: Some(3),
        prompt: "brief".into(),
    });
    assert_eq!(
        parse_smoke_args(&args(&["--variants", "3", "brief"])),
        expected
    );
    assert_eq!(
        parse_smoke_args(&args(&["--variants=3", "brief"])),
        expected
    );
    assert_eq!(
        parse_smoke_args(&args(&["brief", "--variants", "3"])),
        expected
    );
}

#[test]
fn variants_flag_rejects_bad_counts_and_missing_prompt() {
    assert!(parse_smoke_args(&args(&["--variants", "1", "brief"])).is_err());
    assert!(parse_smoke_args(&args(&["--variants", "9", "brief"])).is_err());
    assert!(parse_smoke_args(&args(&["--variants", "x", "brief"])).is_err());
    assert!(parse_smoke_args(&args(&["--variants"])).is_err());
    assert!(parse_smoke_args(&args(&["--variants", "3"])).is_err());
    assert!(parse_smoke_args(&args(&[])).is_err());
    assert!(parse_smoke_args(&args(&["a", "b"])).is_err());
}

/// Every call fails immediately, so each direction's orchestrator run
/// dies in planning without touching the network.
struct FailingLlm;

impl op_orchestrator::LlmClient for FailingLlm {
    fn call(&self, _req: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>> {
        Box::pin(futures::stream::iter(vec![Err(LlmError {
            message: "offline".into(),
            aborted: false,
        })]))
    }
}

fn request(prompt: &str) -> DesignRequest {
    DesignRequest {
        prompt: prompt.into(),
        model: None,
        provider: None,
        design_md: None,
        continuation_context: None,
        append_context: None,
        concurrency: 1,
        validation_enabled: false,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    }
}

#[test]
fn plans_match_the_desktop_style_guide_choice() {
    let req = request("做一个记账 App 的首页，手机端");
    let plans = plan_variants(&req, 3);
    assert_eq!(plans, choose_variant_style_guides(&req.prompt, None, 3));
    assert_eq!(plans.len(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn run_variants_goes_through_the_shared_runner() {
    let req = request("做一个记账 App 的首页，手机端");
    let mut sink = InlineDocSink {
        state: EditorState::new(),
    };
    let pre = SkippedPreValidator;
    let shot = SkippedScreenshotProvider;
    let vision = SkippedVisionLlmClient;
    let providers = ValidationProviders {
        pre_validator: &pre,
        screenshot: &shot,
        vision: &vision,
        system_prompt: String::new(),
    };
    let mut failed = Vec::new();
    let mut on_progress = |event: Progress| {
        if let Progress::VariantFailed { index, .. } = event {
            failed.push(index);
        }
    };
    let result = run_variants(
        3,
        &req,
        &mut sink,
        &FailingLlm,
        &AbortFlag::new(),
        &providers,
        &mut on_progress,
    )
    .await;
    // `VariantFailed` per direction + the all-failed verdict only come
    // from the shared `run_design_variants` fan-out.
    assert!(
        matches!(result, Err(OrchestratorError::AllFailed(_))),
        "{result:?}"
    );
    failed.sort_unstable();
    assert_eq!(failed, vec![0, 1, 2]);
    assert!(sink.state.active_children().is_empty());
}
