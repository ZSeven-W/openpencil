//! The pinned variants turn on the web standard route: Studio Home's
//! "3 directions" send, run by the serve-web daemon.
//!
//! The daemon twin of desktop's `launch_variants_turn`: the same plan choice
//! (`choose_variant_style_guides`), the same localized direction names and
//! the same concurrent driver (`crate::design_variants::drive_design_variants`)
//! — only the landing differs: directions land through the
//! [`WebDesignDocSink`], so their boards reach the browser by live sync like
//! any single design's.
//!
//! Progress streams as the ordinary `thinking` lines, plus one versioned
//! `{"variant": …}` frame per settled direction
//! ([`op_editor_core::variant_wire`]) that the browser folds into its
//! workspace exactly as the desktop pump folds `Progress::VariantReady`.

use std::io::Write;
use std::sync::Arc;

use op_ai::chat_provider::ChatProvider;
use op_editor_core::variant_wire::VariantEventWire;
use op_editor_core::{EditorState, LaunchRoute, Locale};
use op_orchestrator::variants::choose_variant_style_guides;
use op_orchestrator::{AbortFlag, DesignRequest, Progress};

use super::events::{progress_label, write_delta_event, write_done_event, write_error_event};
use super::{CanvasWriteTarget, WebDesignDocSink};
use crate::chat_provider_llm::ChatProviderLlmClient;
use crate::design_variants::{drive_design_variants, localize_variant_plans, VariantsRun};

/// What one web variants turn asks for.
pub(super) struct WebVariantsRun {
    /// The whole-design request every direction starts from.
    pub(super) request: DesignRequest,
    /// Directions requested (clamped again here, like desktop).
    pub(super) count: u8,
    /// Locale the direction names are written in.
    pub(super) locale: Locale,
}

/// The `variant` frame for a direction-level report; `None` for every
/// other progress event.
pub(super) fn variant_wire_for(event: &Progress) -> Option<VariantEventWire> {
    match event {
        Progress::VariantReady(variant) => Some(VariantEventWire::ready(variant)),
        Progress::VariantFailed { index, name, error } => Some(VariantEventWire::failed(
            *index,
            name.clone(),
            error.clone(),
        )),
        _ => None,
    }
}

/// `data: {"variant": …}` — the browser's `AiEvent::Variant`.
pub(super) fn write_variant_event<W: Write>(
    out: &mut W,
    event: &VariantEventWire,
) -> std::io::Result<()> {
    let payload = serde_json::json!({ "variant": event });
    out.write_all(format!("data: {payload}\n\n").as_bytes())?;
    out.flush()
}

pub(super) fn stream_variants_route<W: Write>(
    out: &mut W,
    run: WebVariantsRun,
    snapshot: EditorState,
    provider: Box<dyn ChatProvider>,
    target: CanvasWriteTarget<'_>,
) -> std::io::Result<()> {
    let WebVariantsRun {
        request,
        count,
        locale,
    } = run;
    let count = LaunchRoute::Variants(count)
        .variant_count()
        .unwrap_or(count);
    let mut plans = choose_variant_style_guides(
        &request.prompt,
        request.pinned_style_guide.as_deref(),
        count as usize,
    );
    localize_variant_plans(&mut plans, locale);
    // Reference pages and screenshots feed one design's skeleton; a
    // variants run has no single skeleton to feed (desktop parity), so the
    // turn's attachments are simply not used here.
    let provider_arc: Arc<dyn ChatProvider> = Arc::from(provider);
    let llm = ChatProviderLlmClient::new(provider_arc.clone()).with_model(request.model.clone());
    let mut sink = WebDesignDocSink::new(
        target.state,
        target.hub,
        target.write_barrier,
        snapshot.clone(),
    );
    let abort = AbortFlag::new();
    let epoch = op_editor_core::agent_indicators::begin();
    let mut landed = 0usize;
    let summary = {
        let out_ref = &mut *out;
        let abort_ref = &abort;
        let landed_ref = &mut landed;
        let mut on_progress = move |event: Progress| {
            let mut written = write_thinking(out_ref, &event);
            if let Some(wire) = variant_wire_for(&event) {
                *landed_ref += usize::from(wire.to_workspace_variant().is_some());
                written = written.and_then(|()| write_variant_event(out_ref, &wire));
            }
            if written.is_err() {
                // The browser is gone (Stop, reload, closed tab): stop
                // spending N runs' worth of tokens on a turn nobody reads.
                abort_ref.set();
            }
        };
        drive_design_variants(
            VariantsRun {
                request: &request,
                plans: &plans,
                base_state: &snapshot,
                abort: &abort,
                indicator_epoch: Some(epoch),
                vision_provider: Some(provider_arc),
            },
            &llm,
            &mut sink,
            &mut on_progress,
        )
    };
    if abort.is_set() {
        op_editor_core::agent_indicators::end_if_epoch(epoch);
    } else {
        op_editor_core::agent_indicators::finish_if_epoch(epoch);
    }
    match summary {
        Ok(summary) => {
            write_delta_event(
                out,
                &format!(
                    "\n\nDone — {landed} of {} direction(s) landed, {} node(s) total.",
                    plans.len(),
                    summary.total_nodes
                ),
            )?;
            write_done_event(out)
        }
        Err(error) => write_error_event(out, &error.to_string()),
    }
}

fn write_thinking<W: Write>(out: &mut W, event: &Progress) -> std::io::Result<()> {
    super::events::write_thinking_event(out, &format!("\n{}", progress_label(event)))
}

#[cfg(test)]
#[path = "web_chat_standard_variants_tests.rs"]
mod tests;
