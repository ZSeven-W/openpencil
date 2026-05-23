//! Tests for `visual_ref.rs` — S4 B2.

#[cfg(test)]
mod tests {
    use crate::design_system::default_design_system;
    use crate::test_support::{ScriptResponse, ScriptedLlm};
    use crate::types::AbortFlag;
    use crate::visual_ref::{
        build_enhanced_prompt, extract_structure_summary, generate_design_code,
    };

    // ── generate_design_code ──────────────────────────────────────────────────

    #[tokio::test]
    async fn generate_design_code_returns_llm_output_verbatim() {
        let expected = "<html><body>Hello</body></html>";
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text(expected.to_string())]);
        let abort = AbortFlag::new();
        let ds = default_design_system();

        let result =
            generate_design_code("a login page", ds, 1440.0, 900.0, &llm, None, None, &abort).await;

        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn generate_design_code_returns_empty_on_llm_error() {
        use crate::types::LlmError;
        let llm = ScriptedLlm::new(vec![ScriptResponse::Fail(LlmError {
            message: "timeout".into(),
            aborted: false,
        })]);
        let abort = AbortFlag::new();
        let ds = default_design_system();

        let result =
            generate_design_code("a dashboard", ds, 1440.0, 900.0, &llm, None, None, &abort).await;

        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn generate_design_code_respects_model_and_provider() {
        // This test just checks the call doesn't panic with model/provider set.
        let llm = ScriptedLlm::new(vec![ScriptResponse::Text("<!DOCTYPE html>".to_string())]);
        let abort = AbortFlag::new();
        let ds = default_design_system();

        let result = generate_design_code(
            "a settings page",
            ds,
            375.0,
            812.0,
            &llm,
            Some("claude-3-5-sonnet"),
            Some("anthropic"),
            &abort,
        )
        .await;

        assert!(!result.is_empty());
    }

    // ── extract_structure_summary ─────────────────────────────────────────────

    #[test]
    fn extract_structure_summary_header_line() {
        let result = extract_structure_summary("<div></div>");
        assert_eq!(
            result.lines().next().unwrap(),
            "DESIGN REFERENCE STRUCTURE:"
        );
    }

    #[test]
    fn extract_structure_summary_section_with_class() {
        let html = r#"<section class="hero"><h1>Welcome</h1></section>"#;
        let result = extract_structure_summary(html);
        assert!(
            result.contains("- Section: hero"),
            "expected Section: hero in:\n{result}"
        );
        assert!(
            result.contains("- H1: \"Welcome\""),
            "expected H1: Welcome in:\n{result}"
        );
    }

    #[test]
    fn extract_structure_summary_section_h1_cta() {
        // This is the byte-exact test from the plan:
        // `extract_structure_summary("<section><h1>Hero</h1><a>CTA</a></section>")` → exact format
        // Note: <section> has no class/id → no Section line.
        // <h1> → H1 line.
        // <a> has no class → no CTA line.
        let html = "<section><h1>Hero</h1><a>CTA</a></section>";
        let result = extract_structure_summary(html);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], "DESIGN REFERENCE STRUCTURE:");
        assert_eq!(lines[1], "- H1: \"Hero\"");
        // Only 2 lines (header + H1) — no section/CTA since no class attrs
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn extract_structure_summary_skips_bem_modifier_classes() {
        // classOrId containing `__` must be skipped
        let html = r#"<div class="hero__inner">content</div>"#;
        let result = extract_structure_summary(html);
        assert!(
            !result.contains("hero__inner"),
            "BEM modifier should be skipped"
        );
    }

    #[test]
    fn extract_structure_summary_cta_with_btn_class() {
        let html = r#"<button class="btn-primary">Get Started</button>"#;
        let result = extract_structure_summary(html);
        assert!(
            result.contains("- CTA: \"Get Started\""),
            "expected CTA in:\n{result}"
        );
    }

    #[test]
    fn extract_structure_summary_cta_with_cta_class() {
        let html = "<a class=\"cta-link\" href=\"#\">Learn More</a>";
        let result = extract_structure_summary(html);
        assert!(
            result.contains("- CTA: \"Learn More\""),
            "expected CTA in:\n{result}"
        );
    }

    #[test]
    fn extract_structure_summary_cta_with_button_class() {
        let html = "<a class=\"button-primary\" href=\"#\">Sign Up</a>";
        let result = extract_structure_summary(html);
        assert!(
            result.contains("- CTA: \"Sign Up\""),
            "expected CTA in:\n{result}"
        );
    }

    #[test]
    fn extract_structure_summary_headings_truncated_to_60_chars() {
        let long_title = "A".repeat(80);
        let html = format!("<h2>{long_title}</h2>");
        let result = extract_structure_summary(&html);
        // Should be truncated to 60 chars
        let expected_content: String = "A".repeat(60);
        assert!(
            result.contains(&format!("- H2: \"{expected_content}\"")),
            "should truncate to 60 chars:\n{result}"
        );
    }

    #[test]
    fn extract_structure_summary_cta_text_truncated_to_30_chars() {
        let long_text = "B".repeat(50);
        let html = format!(r#"<button class="btn">{long_text}</button>"#);
        let result = extract_structure_summary(&html);
        let expected_text: String = "B".repeat(30);
        assert!(
            result.contains(&format!("- CTA: \"{expected_text}\"")),
            "should truncate to 30 chars:\n{result}"
        );
    }

    #[test]
    fn extract_structure_summary_fallback_when_no_structure() {
        let html = "<div><span>plain</span></div>";
        let result = extract_structure_summary(html);
        assert!(
            result.contains("(HTML structure extracted"),
            "expected fallback line:\n{result}"
        );
    }

    #[test]
    fn extract_structure_summary_section_with_id_attribute() {
        let html = r#"<section id="about">content</section>"#;
        let result = extract_structure_summary(html);
        assert!(
            result.contains("- Section: about"),
            "expected Section: about in:\n{result}"
        );
    }

    #[test]
    fn extract_structure_summary_all_heading_levels() {
        let html = "<h1>One</h1><h2>Two</h2><h3>Three</h3><h4>Four</h4><h5>Five</h5><h6>Six</h6>";
        let result = extract_structure_summary(html);
        for (level, text) in [
            (1, "One"),
            (2, "Two"),
            (3, "Three"),
            (4, "Four"),
            (5, "Five"),
            (6, "Six"),
        ] {
            assert!(
                result.contains(&format!("- H{level}: \"{text}\"")),
                "missing H{level} in:\n{result}"
            );
        }
    }

    #[test]
    fn extract_structure_summary_strips_tags_from_headings() {
        let html = "<h1><span>Hello</span> <em>World</em></h1>";
        let result = extract_structure_summary(html);
        assert!(
            result.contains("- H1: \"Hello World\""),
            "should strip tags:\n{result}"
        );
    }

    // ── build_enhanced_prompt ─────────────────────────────────────────────────

    #[test]
    fn build_enhanced_prompt_exact_template() {
        let original = "Design a login screen";
        let structure = "DESIGN REFERENCE STRUCTURE:\n- H1: \"Login\"";
        let ds_context = "DESIGN SYSTEM (use these values consistently):\nColors: bg #F8FAFC";

        let result = build_enhanced_prompt(original, structure, ds_context);

        let expected = format!(
            "{original}\n\n{structure}\n\n{ds_context}\n\nIMPORTANT: Follow the design reference structure closely. The design system colors, fonts, and spacing have already been determined — use them consistently. The reference structure shows the intended layout — match its section order and composition."
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn build_enhanced_prompt_contains_important_instruction() {
        let result = build_enhanced_prompt("p", "s", "d");
        assert!(result.contains("IMPORTANT: Follow the design reference structure closely."));
        assert!(result.contains("use them consistently"));
        assert!(result.contains("match its section order and composition."));
    }

    #[test]
    fn build_enhanced_prompt_double_newline_separators() {
        let result = build_enhanced_prompt("p", "s", "d");
        // Check the exact separators: p\n\ns\n\nd\n\nIMPORTANT...
        assert!(result.starts_with("p\n\ns\n\nd\n\nIMPORTANT:"));
    }

    #[test]
    fn build_enhanced_prompt_empty_inputs() {
        let result = build_enhanced_prompt("", "", "");
        // Should still produce the IMPORTANT line
        assert!(result.contains("IMPORTANT:"));
        assert_eq!(result, "\n\n\n\n\n\nIMPORTANT: Follow the design reference structure closely. The design system colors, fonts, and spacing have already been determined — use them consistently. The reference structure shows the intended layout — match its section order and composition.");
    }
}
