//! design.md → 规划 prompt policy —— port of TS `ai-prompts.ts`
//! `buildDesignMdStylePolicy` + `orchestrator-prompt-optimizer.ts`
//! 的 `inferDesignMdBackground` / `guessNeutralBackgroundFromTheme`。

use jian_ops_schema::DesignMdSpec;

/// 按 char 数截断,超出补 `...`。
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        let head: String = s.chars().take(max).collect();
        format!("{head}...")
    } else {
        s.to_string()
    }
}

/// design.md → 一段风格 policy 文本(规划 prompt 用)。无可提取字段
/// → 空串。各块对应 `DesignMdSpec` 字段,缺则跳过。
pub fn build_design_md_style_policy(spec: &DesignMdSpec) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(theme) = &spec.visual_theme {
        parts.push(format!("VISUAL THEME: {}", truncate(theme, 200)));
    }

    if let Some(palette) = &spec.color_palette {
        if !palette.is_empty() {
            let lines: Vec<String> = palette
                .iter()
                .take(10)
                .map(|c| format!("{} ({}) — {}", c.name, c.hex, c.role))
                .collect();
            parts.push(format!("COLOR PALETTE:\n- {}", lines.join("\n- ")));

            let surface: Vec<String> = palette
                .iter()
                .filter(|c| {
                    let r = c.role.to_lowercase();
                    ["surface", "card", "panel", "sidebar", "tile", "chip"]
                        .iter()
                        .any(|k| r.contains(k))
                })
                .take(6)
                .map(|c| format!("{} ({}) — {}", c.name, c.hex, c.role))
                .collect();
            if !surface.is_empty() {
                parts.push(format!(
                    "SURFACE COLORS (use ONLY as `fill` on visually distinct components \
                     placed on top of the page background — cards, sidebars, floating \
                     panels, chips, badges. DO NOT fill section root frames or generic \
                     wrapper frames with these; section containers must stay transparent \
                     and inherit the page background. NEVER use these as the \
                     page/rootFrame fill):\n- {}",
                    surface.join("\n- ")
                ));
            }
        }
    }

    if let Some(typo) = &spec.typography {
        if let Some(f) = &typo.font_family {
            parts.push(format!("FONT: {f}"));
        }
        if let Some(h) = &typo.headings {
            parts.push(format!("Headings: {h}"));
        }
        if let Some(b) = &typo.body {
            parts.push(format!("Body: {b}"));
        }
    }

    if let Some(c) = &spec.component_styles {
        parts.push(format!("COMPONENT STYLES:\n{}", truncate(c, 300)));
    }
    if let Some(l) = &spec.layout_principles {
        parts.push(format!("LAYOUT PRINCIPLES:\n{}", truncate(l, 400)));
    }
    if let Some(n) = &spec.generation_notes {
        parts.push(format!("GENERATION NOTES:\n{}", truncate(n, 400)));
    }

    parts.join("\n\n")
}

/// 从 design.md 调色板推断页面背景色 —— 给 role 评分取最高。
/// 全 0 分(无 background 角色)→ `None`。
pub fn infer_design_md_background(spec: &DesignMdSpec) -> Option<String> {
    let palette = spec.color_palette.as_ref()?;
    let score_role = |role: &str| -> i32 {
        let r = role.to_lowercase();
        let is_page_bg = r.contains("primary") && r.contains("background")
            || r.contains("app background")
            || r.contains("main background")
            || r.contains("page background")
            || r.contains("canvas");
        if is_page_bg {
            return 3;
        }
        let is_surface = ["surface", "card", "tile", "chip", "panel"]
            .iter()
            .any(|k| r.contains(k));
        if r.contains("background") && !is_surface {
            return 2;
        }
        0
    };
    let mut best: Option<(&str, i32)> = None;
    for c in palette {
        let s = score_role(&c.role);
        if s > 0 && best.map(|(_, bs)| s > bs).unwrap_or(true) {
            best = Some((&c.hex, s));
        }
    }
    best.map(|(hex, _)| hex.to_string())
}

/// theme 文本 → 中性背景色。dark 类关键词 → `#111111`,否则 `#FFFFFF`。
pub fn guess_neutral_background_from_theme(theme: Option<&str>) -> String {
    let Some(theme) = theme else {
        return "#FFFFFF".to_string();
    };
    let t = theme.to_lowercase();
    let dark = [
        "dark", "night", "noir", "cyber", "neon", "terminal", "midnight", "obsidian", "onyx",
    ];
    if dark
        .iter()
        .any(|k| crate::design_type::contains_word(&t, k))
    {
        "#111111".to_string()
    } else {
        "#FFFFFF".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::{DesignMdColor, DesignMdSpec};

    fn spec_with_palette(colors: Vec<DesignMdColor>) -> DesignMdSpec {
        DesignMdSpec {
            raw: String::new(),
            project_name: None,
            visual_theme: None,
            color_palette: Some(colors),
            typography: None,
            component_styles: None,
            layout_principles: None,
            generation_notes: None,
        }
    }

    #[test]
    fn neutral_background_dark_vs_light() {
        assert_eq!(
            guess_neutral_background_from_theme(Some("dark moody")),
            "#111111"
        );
        assert_eq!(
            guess_neutral_background_from_theme(Some("airy and calm")),
            "#FFFFFF"
        );
        assert_eq!(guess_neutral_background_from_theme(None), "#FFFFFF");
    }

    #[test]
    fn infer_background_picks_page_background_role() {
        let spec = spec_with_palette(vec![
            DesignMdColor {
                name: "Card".into(),
                hex: "#1E293B".into(),
                role: "card surface".into(),
            },
            DesignMdColor {
                name: "Base".into(),
                hex: "#0A0F1C".into(),
                role: "page background".into(),
            },
        ]);
        assert_eq!(
            infer_design_md_background(&spec).as_deref(),
            Some("#0A0F1C")
        );
    }

    #[test]
    fn infer_background_none_when_no_background_role() {
        let spec = spec_with_palette(vec![DesignMdColor {
            name: "Accent".into(),
            hex: "#FF0000".into(),
            role: "accent".into(),
        }]);
        assert_eq!(infer_design_md_background(&spec), None);
    }

    #[test]
    fn style_policy_emits_theme_and_palette_blocks() {
        let mut spec = spec_with_palette(vec![DesignMdColor {
            name: "Primary".into(),
            hex: "#2563EB".into(),
            role: "accent".into(),
        }]);
        spec.visual_theme = Some("Calm minimal".into());
        let policy = build_design_md_style_policy(&spec);
        assert!(policy.contains("VISUAL THEME: Calm minimal"));
        assert!(policy.contains("COLOR PALETTE:"));
        assert!(policy.contains("Primary (#2563EB) — accent"));
    }

    #[test]
    fn style_policy_empty_when_no_fields() {
        let spec = DesignMdSpec {
            raw: String::new(),
            project_name: None,
            visual_theme: None,
            color_palette: None,
            typography: None,
            component_styles: None,
            layout_principles: None,
            generation_notes: None,
        };
        assert_eq!(build_design_md_style_policy(&spec), "");
    }
}
