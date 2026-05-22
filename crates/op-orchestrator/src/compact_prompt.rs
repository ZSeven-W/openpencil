//! compact 模式规划 prompt —— port of
//! `orchestrator-prompt-optimizer.ts::buildCompactPlanningPrompt`。
//! 不走 skill 解析,手写一段紧凑 system prompt。

use crate::design_md_policy::{
    build_design_md_style_policy, guess_neutral_background_from_theme, infer_design_md_background,
};
use crate::design_type::{detect_design_type, DesignType};
use crate::style_guide_context::infer_tags_from_prompt;
use jian_ops_schema::DesignMdSpec;
use op_ai_skills::style_guide::{
    extract_style_guide_values, select_style_guide, style_guide_registry, Platform, SelectOptions,
};

const DESIGN_MD_STYLE_GUIDE_NAME: &str = "design-md-custom";

/// `build_compact_planning_prompt` 的产物。
pub struct CompactPlanningPrompt {
    pub system: String,
    pub user_prompt: String,
    /// compact 预选的 styleGuideName(进 `PlanningPrompt.forced_style_guide_name`)。
    pub selected_style_guide_name: String,
}

/// 固定 6 行模板头 —— verbatim 移植自 TS(`orchestrator-prompt-optimizer.ts:420-425`)。
const FIXED_HEAD: &str = "You are a UI planning assistant. Output ONLY one JSON object.\n\
Schema: {\"rootFrame\":{\"id\":\"page\",\"name\":\"Page\",\"width\":375,\"height\":812,\"layout\":\"vertical\",\"gap\":20,\"fill\":[{\"type\":\"solid\",\"color\":\"#111827\"}]},\"styleGuideName\":\"guide-name\",\"subtasks\":[{\"id\":\"section-id\",\"label\":\"Section Label\",\"elements\":\"comma-separated owned UI elements\",\"region\":{\"width\":375,\"height\":240}}]}\n\
Every subtask MUST include: id, label, elements, region.width, region.height.\n\
Elements must not overlap between subtasks.\n\
Keep form controls and their submit action in the same subtask.\n\
Start the response with { and end with }. No prose. No markdown. No tool calls.";

/// 构造 compact 规划 prompt —— port of `buildCompactPlanningPrompt`。
pub fn build_compact_planning_prompt(
    prompt: &str,
    design_md: Option<&DesignMdSpec>,
) -> CompactPlanningPrompt {
    let preset = detect_design_type(prompt);
    let platform = if preset.type_ == DesignType::MobileScreen {
        Platform::Mobile
    } else {
        Platform::Webapp
    };
    let tags = infer_tags_from_prompt(prompt);

    // 选 guide(无 design.md 时)。
    let selected_guide = if design_md.is_some() {
        None
    } else {
        select_style_guide(
            style_guide_registry(),
            &SelectOptions {
                tags: tags.clone(),
                name: None,
                platform: Some(platform),
            },
        )
    };
    let guide_bg =
        selected_guide.and_then(|g| extract_style_guide_values(&g.content).colors.background);

    // background_color 优先级。
    let design_md_bg = design_md.and_then(infer_design_md_background);
    let background_color = design_md_bg
        .or_else(|| {
            design_md.map(|s| guess_neutral_background_from_theme(s.visual_theme.as_deref()))
        })
        .or(guide_bg)
        .unwrap_or_else(|| {
            if preset.type_ == DesignType::MobileScreen {
                "#111827".to_string()
            } else {
                "#F8FAFC".to_string()
            }
        });

    let default_gap = match preset.type_ {
        DesignType::MobileScreen | DesignType::DesktopScreen => 20,
        _ => 0,
    };

    let subtask_hint = match preset.type_ {
        DesignType::MobileScreen => {
            "Create 2-4 cohesive subtasks for one mobile app screen. Group related UI together."
        }
        DesignType::DesktopScreen => {
            "Create 2-5 cohesive workspace sections. Keep related dashboard panels together."
        }
        DesignType::Component => {
            "Create exactly 1 subtask for this single component (no surrounding screen, no chrome)."
        }
        DesignType::LandingPage => "Create 4-8 scrollable page sections in top-to-bottom order.",
    };

    let mobile_rules: Vec<&str> = match preset.type_ {
        DesignType::MobileScreen => vec![
            "This is a direct mobile screen, not a phone mockup.",
            "Do NOT create a status bar section. The status bar is inserted separately.",
            "Use width=375 and height=812 on the root frame.",
        ],
        DesignType::Component => vec![
            "This is a single component (Type 0), not a screen.",
            "Do NOT create a status bar, navigation, or footer section.",
            "Use width=400 and height=0 on the root frame.",
            "Use exactly 1 subtask for the component itself.",
        ],
        _ => vec!["Use width=1200 and height=0 on the root frame."],
    };

    let style_rule = if design_md.is_some() {
        format!(
            "Use styleGuideName=\"{DESIGN_MD_STYLE_GUIDE_NAME}\" and rootFrame background \
             {background_color} (from the user's design.md — overrides any catalog default)."
        )
    } else if let Some(g) = selected_guide {
        format!(
            "Use styleGuideName=\"{}\" and rootFrame background {background_color}.",
            g.name
        )
    } else {
        format!(
            "Pick a suitable styleGuideName for platform={} and set rootFrame background to \
             {background_color}.",
            platform.as_str()
        )
    };

    // 组装 system prompt。
    let mut lines: Vec<String> = vec![FIXED_HEAD.to_string(), subtask_hint.to_string(), style_rule];
    for r in &mobile_rules {
        lines.push((*r).to_string());
    }
    lines.push(format!(
        "Always set rootFrame layout=\"vertical\" and gap={default_gap}."
    ));
    if let Some(spec) = design_md {
        let policy = build_design_md_style_policy(spec);
        if !policy.is_empty() {
            lines.push(String::new());
            lines.push(
                "USER DESIGN SYSTEM (design.md — follow these EXACTLY; they OVERRIDE any \
                 default):"
                    .to_string(),
            );
            lines.push(policy);
        }
    }

    let selected_style_guide_name = if design_md.is_some() {
        DESIGN_MD_STYLE_GUIDE_NAME.to_string()
    } else {
        selected_guide.map(|g| g.name.clone()).unwrap_or_default()
    };

    CompactPlanningPrompt {
        system: lines.join("\n"),
        user_prompt: prompt.to_string(),
        selected_style_guide_name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_mobile_prompt_shape() {
        let cp = build_compact_planning_prompt("a mobile login screen", None);
        assert!(cp.system.starts_with("You are a UI planning assistant."));
        assert!(cp.system.contains("width=375 and height=812"));
        assert!(cp.system.contains("Create 2-4 cohesive subtasks"));
        assert_eq!(cp.user_prompt, "a mobile login screen");
    }

    #[test]
    fn compact_landing_prompt_picks_a_guide() {
        let cp = build_compact_planning_prompt("a fintech marketing site", None);
        assert!(cp.system.contains("width=1200 and height=0"));
        // 无 design.md → 从 catalog 选了个 guide 名
        assert!(!cp.selected_style_guide_name.is_empty());
    }

    #[test]
    fn compact_design_md_forces_custom_name() {
        let spec = jian_ops_schema::DesignMdSpec {
            raw: String::new(),
            project_name: None,
            visual_theme: Some("dark".into()),
            color_palette: None,
            typography: None,
            component_styles: None,
            layout_principles: None,
            generation_notes: None,
        };
        let cp = build_compact_planning_prompt("a page", Some(&spec));
        assert_eq!(cp.selected_style_guide_name, "design-md-custom");
        assert!(cp.system.contains("USER DESIGN SYSTEM"));
    }
}
