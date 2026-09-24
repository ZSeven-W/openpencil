//! The pinned [`op_editor_core::LaunchRoute::Variants`] turn: Studio's
//! "3 directions" send. One brief becomes N complete orchestrator runs,
//! concurrently, each pinned to a different style guide, landing side by
//! side on the page. Every provider kind (CLI, builtin API key, ACP) and
//! every model tier takes this same orchestrator path — there is no raw
//! variant path a weak model could fall into.
//!
//! Declared as a `#[path]` child of `chat_session_launch`, like the
//! refine turn beside it.

use std::sync::Arc;

use op_ai::chat_provider::ChatProvider;
use op_editor_core::{variant_letter, LaunchRoute};
use op_host_native::WidgetHostNative;
use op_host_services::chat_provider_llm::ChatProviderLlmClient;
use op_orchestrator::variants::{choose_variant_style_guides, VariantPlan};

use super::launch_design::stamp_design_turn_scenario;
use super::{
    clear_fresh_starter_frame_for_design, prepare_design_request_and_snapshot,
    provider_for_selected_model, selected_cli_model_id, stash_design_request_for_retry,
};
use crate::chat_session::ChatSession;
use op_editor_host_core::design::DesignSession;

/// Localize the plans' display names (`方案 A`, `Direction A`, …).
pub(crate) fn localize_variant_plans(plans: &mut [VariantPlan], locale: op_editor_core::Locale) {
    for plan in plans {
        let letter = variant_letter(plan.index).to_string();
        plan.name =
            op_i18n::translate_with(locale, "workspace.variants.name", &[("letter", &letter)]);
    }
}

/// Launch a variants turn for `count` directions. Returns `false` (and
/// launches nothing) when the selected agent has no provider bridge — the
/// caller then falls through to the ordinary routes, whose honest-error
/// paths explain the missing agent.
pub(super) fn launch_variants_turn(
    host: &mut WidgetHostNative,
    user_text: &str,
    count: u8,
    current_chat: &mut Option<ChatSession>,
    current_design: &mut Option<DesignSession>,
) -> bool {
    let Some(provider) = provider_for_selected_model(host) else {
        return false;
    };
    stamp_design_turn_scenario(host.editor_state_mut(), user_text);
    crate::chat_session::finalize_design_session_if_needed(host, current_chat, "teardown-backstop");
    *current_chat = None;
    let provider_arc: Arc<dyn ChatProvider> = Arc::from(provider);
    let llm =
        ChatProviderLlmClient::new(provider_arc.clone()).with_model(selected_cli_model_id(host));
    if crate::chat_session::allow_ai_bulk_write(host)
        && clear_fresh_starter_frame_for_design(host.editor_state_mut())
    {
        host.mark_editor_state_dirty();
    }
    let (request, initial_state) =
        prepare_design_request_and_snapshot(host, user_text.to_string(), None);
    stash_design_request_for_retry(host, &request);
    // Reference screenshots feed one design's skeleton extraction; a
    // variants run has no single skeleton to feed, so the staged
    // attachments are consumed here rather than leaking into the next send.
    host.editor_state_mut().chat.pending_attachments.clear();

    let count = LaunchRoute::Variants(count)
        .variant_count()
        .unwrap_or(count);
    let locale = host.editor_state().editor_ui.locale;
    let mut plans = choose_variant_style_guides(
        &request.prompt,
        request.pinned_style_guide.as_deref(),
        count as usize,
    );
    localize_variant_plans(&mut plans, locale);
    {
        let workspace = &mut host.editor_state_mut().editor_ui.workspace;
        if workspace.active {
            workspace.begin_variants(count);
        }
    }
    host.mark_editor_state_dirty();
    *current_design = Some(op_host_services::design_variants::start_variants(
        llm,
        request,
        plans,
        initial_state,
        Some(provider_arc),
    ));
    true
}
