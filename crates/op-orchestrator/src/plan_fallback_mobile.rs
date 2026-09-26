//! Heuristic fallback plan for a single mobile screen.
//!
//! Split out of `plan.rs` (800-line cap). A brief that enumerates its own
//! sections gets one subtask per section (`plan_fallback_brief`); otherwise
//! the generic header / main-content pair stands in.

use crate::design_type::DesignTypePreset;
use crate::plan::{OrchestratorPlan, PlanFill, Region, RootFrameSpec, Subtask};
use crate::types::DesignRequest;

pub(crate) fn build_fallback_mobile_plan(
    req: &DesignRequest,
    preset: DesignTypePreset,
) -> OrchestratorPlan {
    let (width, height) = crate::plan::explicit_mobile_size(&req.prompt)
        .unwrap_or((preset.width, preset.root_height.max(preset.height)));
    // A brief that enumerates its own sections gets one subtask per
    // section instead of the generic header/content pair.
    let brief_subtasks = crate::plan_fallback_brief::brief_section_subtasks(
        &req.prompt,
        width,
        Some(height),
        false,
        Some("page"),
    );
    let top_h = (height * 0.24).round().clamp(140.0, 220.0);
    let main_h = (height - top_h).max(320.0);
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "page".into(),
            name: "Page".into(),
            width,
            height,
            layout: Some("vertical".into()),
            gap: Some(0.0),
            padding: Some(0.0),
            fill: Some(vec![PlanFill {
                kind: "solid".into(),
                color: "#FFFFFF".into(),
            }]),
        },
        subtasks: brief_subtasks
            .map(|(subtasks, _)| subtasks)
            .unwrap_or_else(|| {
                vec![
            Subtask {
                id: "top-summary".into(),
                label: "Top Summary".into(),
                region: Region {
                    width,
                    height: top_h,
                },
                bleed_hero: false,
                id_prefix: "top-summary".into(),
                parent_frame_id: Some("page".into()),
                insert_after_sibling_id: None,
                elements: Some(
                    "the screen's header / context for this product (title or greeting, \
                     and the key top-level action[s] this app needs); no status bar. \
                     Choose what fits the prompt — not a fixed delivery-app header."
                        .into(),
                ),
                screen: None,
                generated_root_id: None,
                existing_section_labels: None,
                covers: None,
                retry_feedback: None,
            },
            Subtask {
                id: "main-content".into(),
                label: "Main Content".into(),
                region: Region {
                    width,
                    height: main_h,
                },
                bleed_hero: false,
                id_prefix: "main-content".into(),
                parent_frame_id: Some("page".into()),
                insert_after_sibling_id: None,
                elements: Some(
                    "this screen's primary content for the product — the main job-to-be-done \
                     and whatever modules genuinely fit it; do not repeat the top summary. \
                     Vary the composition per prompt, not a fixed search + banner + card stack."
                        .into(),
                ),
                screen: None,
                generated_root_id: None,
                existing_section_labels: None,
                covers: None,
                retry_feedback: None,
            },
        ]
            }),
        style_guide_name: None,
    }
}
