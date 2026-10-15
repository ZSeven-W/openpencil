//! One color contract shared by section generation and document variables.
use crate::plan::OrchestratorPlan;

pub(super) fn build_plan_palette_instruction(
    plan: &OrchestratorPlan,
    has_variables: bool,
) -> Option<String> {
    let colors = plan.palette.as_ref()?.token_overrides();
    if colors.is_empty() {
        return None;
    }
    let mut lines = vec![
        "SHARED BRIEF PALETTE — overrides conflicting style-guide color examples. Keep the guide's typography and layout treatment.".to_string(),
    ];
    if has_variables {
        lines.push("Use these document variable references for the declared roles. Existing document brand values take precedence; the planned hex values below only describe newly seeded defaults. Do not invent a different accent per section.".into());
        lines.extend(
            colors
                .iter()
                .map(|(token, hex)| format!("- ${token} (planned default {hex})")),
        );
    } else {
        lines.push("Use these exact colors consistently for the declared roles:".into());
        lines.extend(
            colors
                .iter()
                .map(|(token, hex)| format!("- {token}: {hex}")),
        );
    }
    lines.push("Use primary for brand actions/selected states and primary-foreground for their text/icons. Keep semantic success, warning, error and data-series colors distinct; those are not brand overrides.".into());
    Some(lines.join("\n"))
}
