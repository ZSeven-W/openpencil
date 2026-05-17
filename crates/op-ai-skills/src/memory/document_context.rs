//! Document design context — port of `memory/document-context.ts`.

use crate::types::{DesignContext, PreferenceOverride};

/// Style-guide slice of an orchestrator plan ([`OrchestratorPlan`]).
#[derive(Debug, Clone, Default)]
pub struct PlanStyleGuide {
    pub palette: Option<Vec<String>>,
    pub fonts: Option<Vec<String>>,
    pub aesthetic: Option<String>,
}

/// One labelled subtask of an orchestrator plan.
#[derive(Debug, Clone)]
pub struct PlanSubtask {
    pub label: String,
}

/// The subset of an orchestrator plan [`extract_design_context`]
/// reads (TS `OrchestratorPlanLike`).
#[derive(Debug, Clone, Default)]
pub struct OrchestratorPlan {
    pub style_guide: Option<PlanStyleGuide>,
    pub subtasks: Option<Vec<PlanSubtask>>,
}

/// Create an empty design context stamped with `now` (an ISO-8601
/// timestamp the caller supplies).
pub fn create_design_context(document_path: Option<String>, now: &str) -> DesignContext {
    DesignContext {
        document_path,
        created_at: now.to_string(),
        updated_at: now.to_string(),
        ..DesignContext::default()
    }
}

/// Merge an orchestrator plan's style-guide + subtasks into `existing`,
/// bumping `updated_at` to `now`.
pub fn extract_design_context(
    existing: &DesignContext,
    plan: &OrchestratorPlan,
    now: &str,
) -> DesignContext {
    let mut ctx = existing.clone();
    ctx.updated_at = now.to_string();
    if let Some(sg) = &plan.style_guide {
        if let Some(palette) = &sg.palette {
            ctx.design_system.palette = palette.clone();
        }
        if let Some(aesthetic) = &sg.aesthetic {
            ctx.design_system.aesthetic = Some(aesthetic.clone());
        }
        if let Some(fonts) = &sg.fonts {
            ctx.design_system.typography = Some(fonts.join(", "));
        }
    }
    if let Some(subtasks) = &plan.subtasks {
        ctx.structure.sections = subtasks.iter().map(|s| s.label.clone()).collect();
    }
    ctx
}

/// Add or replace a preference override (keyed by `what`), bumping
/// `updated_at` to `now`.
pub fn merge_preference(
    ctx: &DesignContext,
    override_: PreferenceOverride,
    now: &str,
) -> DesignContext {
    let mut next = ctx.clone();
    next.updated_at = now.to_string();
    match next
        .preferences
        .overrides
        .iter_mut()
        .find(|o| o.what == override_.what)
    {
        Some(slot) => *slot = override_,
        None => next.preferences.overrides.push(override_),
    }
    next
}

/// Render a design context as a prompt-ready markdown block.
pub fn context_to_prompt_string(ctx: &DesignContext) -> String {
    let mut parts: Vec<String> = vec!["## Document Design Context".to_string()];
    if let Some(aesthetic) = &ctx.design_system.aesthetic {
        parts.push(format!("Aesthetic: {aesthetic}"));
    }
    if !ctx.design_system.palette.is_empty() {
        parts.push(format!("Palette: {}", ctx.design_system.palette.join(", ")));
    }
    if let Some(typography) = &ctx.design_system.typography {
        parts.push(format!("Typography: {typography}"));
    }
    if let Some(page_type) = &ctx.structure.page_type {
        parts.push(format!("Page Type: {page_type}"));
    }
    if !ctx.preferences.overrides.is_empty() {
        parts.push("User Preferences:".to_string());
        for o in &ctx.preferences.overrides {
            parts.push(format!(
                "  - {}: changed from {} to {}",
                o.what, o.from, o.to
            ));
        }
    }
    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_stamps_both_timestamps() {
        let ctx = create_design_context(Some("/a.op".into()), "2026-05-17T00:00:00Z");
        assert_eq!(ctx.document_path.as_deref(), Some("/a.op"));
        assert_eq!(ctx.created_at, "2026-05-17T00:00:00Z");
        assert_eq!(ctx.updated_at, "2026-05-17T00:00:00Z");
        assert!(ctx.design_system.palette.is_empty());
    }

    #[test]
    fn extract_merges_style_guide_and_subtasks() {
        let base = create_design_context(None, "t0");
        let plan = OrchestratorPlan {
            style_guide: Some(PlanStyleGuide {
                palette: Some(vec!["#000".into(), "#fff".into()]),
                fonts: Some(vec!["Inter".into(), "Playfair".into()]),
                aesthetic: Some("minimal".into()),
            }),
            subtasks: Some(vec![
                PlanSubtask {
                    label: "hero".into(),
                },
                PlanSubtask {
                    label: "pricing".into(),
                },
            ]),
        };
        let out = extract_design_context(&base, &plan, "t1");
        assert_eq!(out.updated_at, "t1");
        assert_eq!(out.design_system.palette, vec!["#000", "#fff"]);
        assert_eq!(
            out.design_system.typography.as_deref(),
            Some("Inter, Playfair")
        );
        assert_eq!(out.design_system.aesthetic.as_deref(), Some("minimal"));
        assert_eq!(out.structure.sections, vec!["hero", "pricing"]);
    }

    #[test]
    fn merge_preference_adds_then_replaces() {
        let ctx = create_design_context(None, "t0");
        let ctx = merge_preference(
            &ctx,
            PreferenceOverride {
                what: "button color".into(),
                from: "blue".into(),
                to: "red".into(),
            },
            "t1",
        );
        assert_eq!(ctx.preferences.overrides.len(), 1);
        // Re-merging the same `what` replaces, not appends.
        let ctx = merge_preference(
            &ctx,
            PreferenceOverride {
                what: "button color".into(),
                from: "red".into(),
                to: "green".into(),
            },
            "t2",
        );
        assert_eq!(ctx.preferences.overrides.len(), 1);
        assert_eq!(ctx.preferences.overrides[0].to, "green");
    }

    #[test]
    fn prompt_string_includes_set_fields_only() {
        let mut ctx = create_design_context(None, "t0");
        ctx.design_system.aesthetic = Some("brutalist".into());
        let s = context_to_prompt_string(&ctx);
        assert!(s.starts_with("## Document Design Context"));
        assert!(s.contains("Aesthetic: brutalist"));
        assert!(!s.contains("Palette:"));
    }
}
