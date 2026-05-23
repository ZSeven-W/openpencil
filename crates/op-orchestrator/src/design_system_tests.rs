//! Tests for `design_system.rs` — A1 step 1 (failing tests first, TDD).
//! B1 tests appended at the bottom.

#[cfg(test)]
mod tests {
    use crate::design_system::{default_design_system, parse_design_system, DesignSystem};
    use crate::types::{DesignRequest, Progress};

    // ── parse_design_system: direct JSON round-trip ───────────────────────────

    #[test]
    fn parse_design_system_direct_json() {
        let ds = default_design_system();
        let json = serde_json::to_string(ds).expect("serialize default");
        let parsed = parse_design_system(&json);
        assert_eq!(parsed.palette, ds.palette);
        assert_eq!(parsed.aesthetic, ds.aesthetic);
    }

    // ── parse_design_system: code-fence stripping ─────────────────────────────

    #[test]
    fn parse_design_system_strips_json_code_fence() {
        let ds = default_design_system();
        let inner = serde_json::to_string(ds).expect("serialize");
        let fenced = format!("```json\n{inner}\n```");
        let parsed = parse_design_system(&fenced);
        assert_eq!(parsed.palette, ds.palette);
    }

    #[test]
    fn parse_design_system_strips_bare_code_fence() {
        let ds = default_design_system();
        let inner = serde_json::to_string(ds).expect("serialize");
        let fenced = format!("```\n{inner}\n```");
        let parsed = parse_design_system(&fenced);
        assert_eq!(parsed.palette, ds.palette);
    }

    // ── parse_design_system: brace extraction ────────────────────────────────

    #[test]
    fn parse_design_system_brace_extract() {
        let ds = default_design_system();
        let inner = serde_json::to_string(ds).expect("serialize");
        let wrapped = format!("Some preamble text\n{inner}\nSome trailing text.");
        let parsed = parse_design_system(&wrapped);
        assert_eq!(parsed.palette, ds.palette);
    }

    // ── parse_design_system: fallback to DEFAULT on garbage ──────────────────

    #[test]
    fn parse_design_system_fallback_on_garbage() {
        let ds = default_design_system();
        let parsed = parse_design_system("not json at all");
        assert_eq!(parsed.palette, ds.palette);
        assert_eq!(parsed.aesthetic, ds.aesthetic);
    }

    #[test]
    fn parse_design_system_fallback_on_missing_palette() {
        // Valid JSON but missing palette field → fallback
        let ds = default_design_system();
        let parsed = parse_design_system(r#"{"aesthetic": "flat"}"#);
        assert_eq!(parsed.palette, ds.palette);
    }

    // ── DEFAULT_DESIGN_SYSTEM round-trips via serde ───────────────────────────

    #[test]
    fn default_design_system_serde_round_trip() {
        let ds = default_design_system();
        let json = serde_json::to_string(ds).expect("serialize");
        let back: DesignSystem = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.palette, ds.palette);
        assert_eq!(back.typography.heading_font, ds.typography.heading_font);
        assert_eq!(back.typography.body_font, ds.typography.body_font);
        assert_eq!(back.typography.scale, ds.typography.scale);
        assert_eq!(back.spacing.unit, ds.spacing.unit);
        assert_eq!(back.spacing.scale, ds.spacing.scale);
        assert_eq!(back.radius, ds.radius);
        assert_eq!(back.aesthetic, ds.aesthetic);
    }

    // ── DEFAULT_DESIGN_SYSTEM exact values (port faithful to TS) ─────────────

    #[test]
    fn default_design_system_palette_values() {
        let p = &default_design_system().palette;
        assert_eq!(p["background"], "#F8FAFC");
        assert_eq!(p["surface"], "#FFFFFF");
        assert_eq!(p["text"], "#0F172A");
        assert_eq!(p["textSecondary"], "#475569");
        assert_eq!(p["primary"], "#2563EB");
        assert_eq!(p["primaryLight"], "#DBEAFE");
        assert_eq!(p["accent"], "#0EA5E9");
        assert_eq!(p["border"], "#E2E8F0");
    }

    #[test]
    fn default_design_system_typography_values() {
        let t = &default_design_system().typography;
        assert_eq!(t.heading_font, "Space Grotesk");
        assert_eq!(t.body_font, "Inter");
        assert_eq!(t.scale, vec![14.0, 16.0, 20.0, 28.0, 40.0, 56.0]);
    }

    #[test]
    fn default_design_system_spacing_values() {
        let s = &default_design_system().spacing;
        assert_eq!(s.unit, 8.0);
        assert_eq!(s.scale, vec![8.0, 16.0, 24.0, 32.0, 48.0, 64.0]);
    }

    #[test]
    fn default_design_system_radius_values() {
        assert_eq!(default_design_system().radius, vec![8.0, 12.0, 16.0]);
    }

    #[test]
    fn default_design_system_aesthetic_value() {
        assert_eq!(default_design_system().aesthetic, "clean modern blue");
    }

    // ── DesignRequest.visual_ref_enabled defaults to false ────────────────────

    #[test]
    fn design_request_visual_ref_enabled_defaults_false() {
        // JSON without `visualRefEnabled` field
        let json = r#"{"prompt":"test","concurrency":1}"#;
        let req: DesignRequest = serde_json::from_str(json).expect("deserialize");
        assert!(
            !req.visual_ref_enabled,
            "visual_ref_enabled should default to false"
        );
    }

    #[test]
    fn design_request_visual_ref_enabled_can_be_set_true() {
        let json = r#"{"prompt":"test","concurrency":1,"visualRefEnabled":true}"#;
        let req: DesignRequest = serde_json::from_str(json).expect("deserialize");
        assert!(req.visual_ref_enabled);
    }

    #[test]
    fn design_request_visual_ref_enabled_literal_compiles() {
        // Verify that the field can be written in struct literal form
        let req = DesignRequest {
            prompt: "test".into(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 1,
            append_context: None,
            validation_enabled: true,
            visual_ref_enabled: false,
        };
        assert!(!req.visual_ref_enabled);
    }

    // ── Progress::VisualRef* variants compile and pattern-match ──────────────

    #[test]
    fn progress_visual_ref_variants_compile() {
        let variants = vec![
            Progress::VisualRefStarted,
            Progress::VisualRefDesignSystem { var_count: 25 },
            Progress::VisualRefHtmlGenerated { byte_len: 4096 },
            Progress::VisualRefScreenshotReady { skipped: false },
            Progress::VisualRefFallback {
                reason: "LLM returned empty HTML".into(),
            },
        ];
        for v in variants {
            match v {
                Progress::VisualRefStarted => {}
                Progress::VisualRefDesignSystem { var_count } => {
                    assert_eq!(var_count, 25);
                }
                Progress::VisualRefHtmlGenerated { byte_len } => {
                    assert_eq!(byte_len, 4096);
                }
                Progress::VisualRefScreenshotReady { skipped } => {
                    assert!(!skipped);
                }
                Progress::VisualRefFallback { reason } => {
                    assert!(!reason.is_empty());
                }
                _ => {}
            }
        }
    }

    // ── DesignSystem struct fields exist and are accessible ───────────────────

    #[test]
    fn design_system_struct_fields_accessible() {
        let ds = DesignSystem {
            palette: {
                let mut m = std::collections::BTreeMap::new();
                m.insert("background".to_string(), "#F8FAFC".to_string());
                m
            },
            typography: crate::design_system::Typography {
                heading_font: "Space Grotesk".into(),
                body_font: "Inter".into(),
                scale: vec![14.0, 16.0],
            },
            spacing: crate::design_system::Spacing {
                unit: 8.0,
                scale: vec![8.0, 16.0],
            },
            radius: vec![8.0],
            aesthetic: "clean".into(),
        };
        assert_eq!(ds.palette["background"], "#F8FAFC");
        assert_eq!(ds.typography.heading_font, "Space Grotesk");
        assert_eq!(ds.spacing.unit, 8.0);
    }

    // ── Task B1: generate_design_system ──────────────────────────────────────

    /// Scripted LLM returning valid JSON design-system → parsed DesignSystem
    /// (not the default — the LLM-provided values win).
    #[tokio::test]
    async fn generate_design_system_happy_path() {
        use crate::design_system::generate_design_system;
        use crate::test_support::{ScriptResponse, ScriptedLlm};
        use crate::types::AbortFlag;

        // Craft a valid JSON that differs from DEFAULT_DESIGN_SYSTEM so we
        // can confirm the LLM value was used.
        // Build the JSON string using serde_json to avoid raw-string delimiter conflicts.
        let custom_json = serde_json::json!({
            "palette": {
                "background": "\u{23}111111",
                "surface": "\u{23}222222",
                "text": "\u{23}FFFFFF",
                "textSecondary": "\u{23}AAAAAA",
                "primary": "\u{23}FF0000",
                "primaryLight": "\u{23}FF9999",
                "accent": "\u{23}00FF00",
                "border": "\u{23}333333"
            },
            "typography": {
                "headingFont": "Roboto",
                "bodyFont": "Open Sans",
                "scale": [12.0_f64, 14.0, 18.0, 24.0, 36.0, 48.0]
            },
            "spacing": { "unit": 4.0_f64, "scale": [4.0_f64, 8.0, 12.0, 16.0, 24.0, 32.0] },
            "radius": [4.0_f64, 8.0, 12.0],
            "aesthetic": "dark minimal"
        })
        .to_string();

        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(custom_json.to_string())]);
        let abort = AbortFlag::new();
        let ds = generate_design_system("a dark app", &llm, None, None, &abort).await;

        // LLM-provided values must win over default
        assert_eq!(ds.palette["background"], "#111111");
        assert_eq!(ds.typography.heading_font, "Roboto");
        assert_eq!(ds.typography.body_font, "Open Sans");
        assert_eq!(ds.aesthetic, "dark minimal");
        assert_eq!(ds.radius, vec![4.0, 8.0, 12.0]);
    }

    /// Scripted LLM returning garbage → fallback to DEFAULT_DESIGN_SYSTEM.
    #[tokio::test]
    async fn generate_design_system_garbage_falls_back_to_default() {
        use crate::design_system::generate_design_system;
        use crate::test_support::{ScriptResponse, ScriptedLlm};
        use crate::types::AbortFlag;

        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
            "not valid json at all!".to_string(),
        )]);
        let abort = AbortFlag::new();
        let ds = generate_design_system("any prompt", &llm, None, None, &abort).await;
        let default = default_design_system();

        assert_eq!(ds.palette, default.palette);
        assert_eq!(ds.aesthetic, default.aesthetic);
    }

    /// LLM returning JSON wrapped in code fence → parsed correctly.
    #[tokio::test]
    async fn generate_design_system_code_fence_response() {
        use crate::design_system::generate_design_system;
        use crate::test_support::{ScriptResponse, ScriptedLlm};
        use crate::types::AbortFlag;

        let ds_default = default_design_system();
        let inner = serde_json::to_string(ds_default).unwrap();
        let fenced = format!("```json\n{inner}\n```");

        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(fenced)]);
        let abort = AbortFlag::new();
        let ds = generate_design_system("prompt", &llm, None, None, &abort).await;
        assert_eq!(ds.palette, ds_default.palette);
    }

    // ── Task B1: design_system_to_seed_commands ───────────────────────────────

    /// DEFAULT_DESIGN_SYSTEM → expected number of SetVariable* commands.
    /// Faithful to TS `designSystemToVariables` (L134-156): typography is NOT
    /// seeded into document variables, only colors + spacing + radius.
    /// 8 palette (color) + 6 spacing scale + 3 radius = 17.
    #[test]
    fn seed_commands_default_count() {
        use crate::design_system::design_system_to_seed_commands;
        let ds = default_design_system();
        let cmds = design_system_to_seed_commands(ds);
        assert_eq!(
            cmds.len(),
            17,
            "expected 17 seed commands (8 palette + 6 spacing + 3 radius), got {}",
            cmds.len()
        );
    }

    /// DEFAULT_DESIGN_SYSTEM → no typography variables emitted.
    /// Faithful to TS — typography reaches the LLM via prompt context, not vars.
    #[test]
    fn seed_commands_no_typography_variables() {
        use crate::design_system::design_system_to_seed_commands;
        use op_editor_core::EditorCommand;
        let ds = default_design_system();
        let cmds = design_system_to_seed_commands(ds);

        let has_font_var = cmds.iter().any(|c| match c {
            EditorCommand::SetVariableColor { name, .. }
            | EditorCommand::SetVariableScalar { name, .. } => {
                name.starts_with("font-") || name.starts_with("typography-")
            }
            _ => false,
        });
        assert!(
            !has_font_var,
            "typography MUST NOT be seeded into document variables (faithful to TS)"
        );
    }

    /// Palette colors → SetVariableColor with kebab-case names.
    #[test]
    fn seed_commands_palette_color_names() {
        use crate::design_system::design_system_to_seed_commands;
        use op_editor_core::EditorCommand;

        let ds = default_design_system();
        let cmds = design_system_to_seed_commands(ds);

        // Collect all SetVariableColor names
        let color_names: Vec<String> = cmds
            .iter()
            .filter_map(|c| {
                if let EditorCommand::SetVariableColor { name, .. } = c {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        // Verify all 8 palette keys are present in kebab-case
        assert!(
            color_names.contains(&"color-background".to_string()),
            "missing color-background"
        );
        assert!(
            color_names.contains(&"color-text".to_string()),
            "missing color-text"
        );
        assert!(
            color_names.contains(&"color-text-secondary".to_string()),
            "missing color-text-secondary (textSecondary → text-secondary)"
        );
        assert!(
            color_names.contains(&"color-primary-light".to_string()),
            "missing color-primary-light (primaryLight → primary-light)"
        );
        assert_eq!(color_names.len(), 8, "expected 8 color variables");
    }

    /// Palette color value is correctly mapped.
    #[test]
    fn seed_commands_palette_color_value() {
        use crate::design_system::design_system_to_seed_commands;
        use op_editor_core::EditorCommand;

        let ds = default_design_system();
        let cmds = design_system_to_seed_commands(ds);

        let bg_cmd = cmds.iter().find(|c| {
            matches!(c, EditorCommand::SetVariableColor { name, .. } if name == "color-background")
        });
        assert!(bg_cmd.is_some(), "missing color-background command");
        if let Some(EditorCommand::SetVariableColor { hex, .. }) = bg_cmd {
            assert_eq!(hex, "#F8FAFC", "wrong color-background value");
        }
    }

    /// Spacing scale → SetVariableScalar::Number with spacing-xs/sm/... names.
    #[test]
    fn seed_commands_spacing_scale_names() {
        use crate::design_system::design_system_to_seed_commands;
        use op_editor_core::{EditorCommand, VariableScalarPayload};

        let ds = default_design_system();
        let cmds = design_system_to_seed_commands(ds);

        let spacing_names: Vec<String> = cmds
            .iter()
            .filter_map(|c| {
                if let EditorCommand::SetVariableScalar {
                    name,
                    scalar: VariableScalarPayload::Number(_),
                } = c
                {
                    if name.starts_with("spacing-") {
                        return Some(name.clone());
                    }
                }
                None
            })
            .collect();

        assert!(
            spacing_names.contains(&"spacing-xs".to_string()),
            "missing spacing-xs"
        );
        assert!(
            spacing_names.contains(&"spacing-sm".to_string()),
            "missing spacing-sm"
        );
        assert!(
            spacing_names.contains(&"spacing-md".to_string()),
            "missing spacing-md"
        );
        assert!(
            spacing_names.contains(&"spacing-lg".to_string()),
            "missing spacing-lg"
        );
        assert!(
            spacing_names.contains(&"spacing-xl".to_string()),
            "missing spacing-xl"
        );
        assert!(
            spacing_names.contains(&"spacing-2xl".to_string()),
            "missing spacing-2xl"
        );
        assert_eq!(spacing_names.len(), 6, "expected 6 spacing variables");
    }

    /// Radius steps → SetVariableScalar::Number with radius-sm/md/lg names.
    #[test]
    fn seed_commands_radius_names() {
        use crate::design_system::design_system_to_seed_commands;
        use op_editor_core::{EditorCommand, VariableScalarPayload};

        let ds = default_design_system();
        let cmds = design_system_to_seed_commands(ds);

        let radius_names: Vec<String> = cmds
            .iter()
            .filter_map(|c| {
                if let EditorCommand::SetVariableScalar {
                    name,
                    scalar: VariableScalarPayload::Number(_),
                } = c
                {
                    if name.starts_with("radius-") {
                        return Some(name.clone());
                    }
                }
                None
            })
            .collect();

        assert!(
            radius_names.contains(&"radius-sm".to_string()),
            "missing radius-sm"
        );
        assert!(
            radius_names.contains(&"radius-md".to_string()),
            "missing radius-md"
        );
        assert!(
            radius_names.contains(&"radius-lg".to_string()),
            "missing radius-lg"
        );
        assert_eq!(radius_names.len(), 3, "expected 3 radius variables");
    }

    // ── Task B1: design_system_to_prompt_context ─────────────────────────────

    /// `design_system_to_prompt_context` produces the exact TS template.
    #[test]
    fn prompt_context_exact_template() {
        use crate::design_system::design_system_to_prompt_context;

        let ds = default_design_system();
        let ctx = design_system_to_prompt_context(ds);

        // Verify structural lines (port of TS L161-170 format)
        assert!(
            ctx.starts_with("DESIGN SYSTEM (use these values consistently):"),
            "wrong header: {ctx}"
        );
        assert!(ctx.contains("Colors: bg #F8FAFC"), "missing Colors line");
        assert!(ctx.contains("surface #FFFFFF"), "missing surface");
        assert!(ctx.contains("text #0F172A"), "missing text");
        assert!(ctx.contains("muted #475569"), "missing muted");
        assert!(ctx.contains("primary #2563EB"), "missing primary");
        assert!(ctx.contains("primaryLight #DBEAFE"), "missing primaryLight");
        assert!(ctx.contains("accent #0EA5E9"), "missing accent");
        assert!(ctx.contains("border #E2E8F0"), "missing border");
        assert!(
            ctx.contains(r#"Fonts: heading "Space Grotesk""#),
            "missing heading font"
        );
        assert!(ctx.contains(r#"body "Inter""#), "missing body font");
        assert!(
            ctx.contains("Type scale: 14, 16, 20, 28, 40, 56px"),
            "wrong type scale line: {ctx}"
        );
        assert!(
            ctx.contains("Spacing: 8, 16, 24, 32, 48, 64px (8px grid)"),
            "wrong spacing line: {ctx}"
        );
        assert!(
            ctx.contains("Radius: 8, 12, 16px"),
            "wrong radius line: {ctx}"
        );
        assert!(
            ctx.contains("Style: clean modern blue"),
            "wrong style line: {ctx}"
        );
    }

    /// Byte-exact match of the TS template for DEFAULT_DESIGN_SYSTEM.
    #[test]
    fn prompt_context_byte_exact() {
        use crate::design_system::design_system_to_prompt_context;

        let ds = default_design_system();
        let ctx = design_system_to_prompt_context(ds);

        let expected = "DESIGN SYSTEM (use these values consistently):\n\
Colors: bg #F8FAFC, surface #FFFFFF, text #0F172A, muted #475569, primary #2563EB, primaryLight #DBEAFE, accent #0EA5E9, border #E2E8F0\n\
Fonts: heading \"Space Grotesk\", body \"Inter\"\n\
Type scale: 14, 16, 20, 28, 40, 56px\n\
Spacing: 8, 16, 24, 32, 48, 64px (8px grid)\n\
Radius: 8, 12, 16px\n\
Style: clean modern blue";

        assert_eq!(
            ctx, expected,
            "prompt context does not match TS template byte-exactly"
        );
    }
}
