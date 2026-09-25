//! Side-by-side directions in the design pump: each landed direction is
//! recorded on the workspace (the "use this" bar reads it), and every
//! direction's landing or failure is narrated in the run's primary bubble.

use op_editor_core::{ChatMessage, EditorState, Locale};
use op_orchestrator::Progress;

use super::append_narration;

/// Record every direction that landed in this batch on the workspace.
/// Returns whether the workspace changed.
pub(super) fn fold_variant_progress(state: &mut EditorState, progress: &[Progress]) -> bool {
    let mut changed = false;
    for event in progress {
        if let Progress::VariantReady(variant) = event {
            // The fold itself is shared with the web host, which receives
            // the same report over SSE (`op_editor_core::variant_wire`).
            changed |= state
                .editor_ui
                .workspace
                .adopt_landed_variant(variant.clone());
        }
    }
    changed
}

/// One narration line for a direction's landing or failure.
pub(super) fn narrate_variant(msg: &mut ChatMessage, event: &Progress, locale: Locale) -> bool {
    let line = match event {
        Progress::VariantReady(variant) => op_i18n::translate_with(
            locale,
            "workspace.variants.ready",
            &[("name", &variant.name), ("style", &variant.style_label)],
        ),
        Progress::VariantFailed { name, error, .. } => op_i18n::translate_with(
            locale,
            "workspace.variants.failed",
            &[("name", name), ("error", error)],
        ),
        _ => return false,
    };
    append_narration(msg, &format!("• {line}"))
}

#[cfg(test)]
#[path = "design_session_variants_tests.rs"]
mod tests;
