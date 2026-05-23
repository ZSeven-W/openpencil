//! Tests for `design_system.rs` — A1 step 1 (failing tests first, TDD).

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
}
