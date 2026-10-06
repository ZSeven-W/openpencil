//! Corpus, guideline and design-agent prompt tests for this crate.
//!
//! Split out of `lib.rs` as pure code motion to keep that spine under the
//! repo's 800-line cap; the module still reaches the crate's private items
//! through `use super::*`, exactly as it did inline.

use super::*;

#[test]
fn embeds_the_skill_corpus() {
    // The TS package ships ~95 skill + style-guide markdown files;
    // the full corpus must travel with the crate.
    assert!(
        count_md(&SKILLS) >= 91,
        "expected the full skill corpus to embed, found {}",
        count_md(&SKILLS)
    );
}

#[test]
fn guideline_for_web_app_returns_product_design_content() {
    let content = guideline_for("web-app").expect("web-app guideline must be present");
    // Both source skills must be represented.
    assert!(
        content.contains("PURPOSE FIRST"),
        "web-app guideline must contain product-principles content"
    );
    assert!(
        content.contains("DESIGN CRAFT"),
        "web-app guideline must contain design-principles content"
    );
    assert!(!content.is_empty());
}

#[test]
fn design_agent_prompt_with_skills_appends_matched_domain_depth() {
    // A dashboard ask must carry dashboard.md + the always-on principle
    // bases on top of the unchanged protocol base.
    let prompt =
        design_agent_system_prompt_with_skills("Design an analytics dashboard with a client table");
    assert!(
        prompt.starts_with(design_agent_system_prompt()),
        "protocol base must lead the assembled prompt"
    );
    assert!(
        prompt.contains("Product-Design Depth"),
        "depth section header present"
    );
    assert!(
        prompt.contains("PURPOSE FIRST"),
        "always-on product-principles must ride along"
    );
    assert!(
        prompt.contains("DASHBOARD / ADMIN / DATA-TABLE DEPTH"),
        "dashboard domain skill must resolve for a dashboard ask"
    );
}

#[test]
fn design_agent_prompt_with_skills_matches_mobile_domain() {
    let prompt = design_agent_system_prompt_with_skills("design a mobile fitness app home screen");
    assert!(
        prompt.contains("THREE-SECTION ARCHITECTURE"),
        "mobile-app domain skill must resolve for a mobile ask"
    );
    assert!(
        prompt.contains("transparent root-direct section frames")
            && prompt.contains("height=\"fit_content\"")
            && prompt.contains("padding: [0,24]"),
        "mobile domain must teach Hug Height and the section-owned content rail"
    );
}

#[test]
fn builtin_design_loop_prompt_mounts_the_shared_first_class_widget_contract() {
    let widgets = get_skill_by_name("jian-components")
        .expect("jian-components must be registered")
        .content
        .trim();
    let base = design_agent_system_prompt();
    let builtin = design_agent_system_prompt_with_skills(
        "Continue this mobile app with an interactive settings screen",
    );

    assert!(
        base.contains(widgets),
        "bare tool-loop prompt must mount the exact shared generation contract"
    );
    assert!(
        builtin.contains(widgets),
        "the prompt passed to builtin design turns must retain the shared contract"
    );
    assert!(!base.contains(JIAN_COMPONENTS_PLACEHOLDER));
    assert_eq!(
        builtin.matches("FIRST-CLASS OUTPUT").count(),
        1,
        "the builtin prompt must mount one authoritative widget contract"
    );
    for kind in [
        "text_input",
        "text_area",
        "select",
        "switch",
        "checkbox",
        "slider",
        "radio_group",
        "number_input",
        "progress",
        "tabs",
    ] {
        assert!(
            builtin.contains(kind),
            "builtin design loop must receive first-class widget `{kind}`"
        );
    }
    for contract in [
        "options: [{value,label}]",
        "`checked`",
        "`min`, `max`, `step`, and `value`",
        "MUST explicitly carry `fill`, `stroke`, and",
        "`cornerRadius`",
        "`fill` is the active/accent paint",
        "`stroke.fill` is the inactive track/border paint",
        "LEGACY COMPATIBILITY ONLY",
    ] {
        assert!(
            builtin.contains(contract),
            "builtin design loop lost native-widget contract {contract:?}"
        );
    }
}

#[test]
fn design_agent_base_prompt_keeps_final_hug_height_invariant() {
    let prompt = design_agent_system_prompt();
    assert!(
        prompt.contains("Final sizing invariant")
            && prompt.contains("ordinary content wrappers default to `height:\"fit_content\"`"),
        "the tool-loop base must carry the sizing invariant even when protocol skills are excluded"
    );
}

#[test]
fn design_agent_prompt_with_skills_excludes_protocol_skills() {
    // Output-protocol skills (schema / layout / codegen) must NOT ride
    // into the tool-loop prompt — the loop's protocol is the tool loop
    // itself (design-agent.md), and doubling protocols confuses weak
    // models. `layout.md`'s canonical opener is the marker.
    let prompt =
        design_agent_system_prompt_with_skills("Design an analytics dashboard with a client table");
    assert!(
        !prompt.contains("LAYOUT ENGINE (flexbox-based)"),
        "generation-protocol layout.md must not be appended"
    );
}

#[test]
fn guideline_for_mobile_returns_mobile_app_content() {
    let content = guideline_for("mobile").expect("mobile guideline must be present");
    // Canonical phrase from mobile-app.md.
    assert!(
        content.contains("THREE-SECTION ARCHITECTURE"),
        "mobile guideline must contain the three-section architecture content"
    );
    assert!(!content.is_empty());
}

#[test]
fn code_to_design_guideline_resolves() {
    let content = guideline_for("code-to-design").expect("code-to-design guideline present");
    for must in [
        "Five-step",
        "upsert_variables",
        "upsert_component",
        "upsert_screen",
        "conversion_status",
        "lint_document",
        "get_screenshot",
    ] {
        assert!(content.contains(must), "guideline missing: {must}");
    }
}

#[test]
fn code_to_design_component_example_uses_valid_pen_node_json() {
    let content = guideline_for("code-to-design").expect("code-to-design guideline present");
    let marker = "{\n  \"key\": \"src/components/Button.tsx#Button\"";
    let start = content
        .find(marker)
        .expect("component example JSON block must exist");
    let end = content[start..]
        .find("\n```")
        .map(|offset| start + offset)
        .expect("component example JSON block must close");
    let example: serde_json::Value =
        serde_json::from_str(&content[start..end]).expect("component example parses as JSON");
    let node = example
        .get("node_json")
        .cloned()
        .expect("component example includes node_json");

    serde_json::from_value::<jian_ops_schema::node::PenNode>(node)
        .expect("component example node_json must deserialize as a PenNode");
}

#[test]
fn guideline_for_unknown_topic_returns_none() {
    assert!(
        guideline_for("unknown-topic").is_none(),
        "unknown topics must return None"
    );
    assert!(guideline_for("").is_none(), "empty topic must return None");
    assert!(
        guideline_for("desktop").is_none(),
        "unsupported topics must return None"
    );
}

#[test]
fn guideline_for_extended_topics_resolve() {
    // landing-page composes the landing domain + design craft.
    let lp = guideline_for("landing-page").expect("landing-page guideline present");
    assert!(
        lp.contains("DESIGN CRAFT"),
        "landing-page must include design craft"
    );
    // slides resolves to the slide layout contracts AND the pattern
    // skeletons — an external agent asking for slide guidance needs both
    // the tier/format rules and the structures that satisfy them.
    let sl = guideline_for("slides").expect("slides guideline present");
    assert!(
        sl.to_uppercase().contains("SLIDE"),
        "slides must include slide guidance"
    );
    assert!(
        sl.contains("## Style tiers — pick ONE for the whole deck"),
        "slides guideline must carry the style tiers"
    );
    assert!(
        sl.contains("DECK PATTERNS — SLIDE SKELETONS"),
        "slides guideline must carry the pattern skeletons"
    );
    // dashboard / table both resolve.
    assert!(
        guideline_for("dashboard").is_some(),
        "dashboard guideline present"
    );
    assert!(guideline_for("table").is_some(), "table alias resolves");
    // web-app now also carries the web-app depth laws.
    let wa = guideline_for("web-app").expect("web-app guideline present");
    assert!(
        wa.contains("PROGRESSIVE DISCLOSURE"),
        "web-app must include the web-app depth laws"
    );
    // aliases resolve.
    assert!(guideline_for("webapp").is_some(), "webapp alias resolves");
    assert!(
        guideline_for("presentation").is_some(),
        "presentation alias resolves"
    );
}

#[test]
fn guideline_for_interactivity_teaches_screen_and_on_tap_contract() {
    let content = guideline_for("interactivity").expect("interactivity guideline must be present");
    assert!(
        content.contains("\"screen\""),
        "must teach the screen marker"
    );
    assert!(
        content.contains("events.onTap") || content.contains("onTap"),
        "must teach the events.onTap binding"
    );
    assert!(
        content.contains(r#"{ "replace": "\"/profile\"" } "#)
            || content.contains(r#"{ "replace": "\"/profile\"" }"#),
        "must show the exact quote-literal replace example: {content:?}"
    );
    assert!(
        content.contains(r#"{"pop": null}"#) || content.contains(r#"{ "pop": null }"#),
        "must show the pop (no-path) example"
    );
    assert!(
        content.contains("`route` field") && content.contains("schema-only"),
        "must forbid the schema-only route field: {content:?}"
    );
}

#[test]
fn design_agent_system_prompt_resolves_and_contains_protocol_markers() {
    let prompt = design_agent_system_prompt();
    assert!(
        !prompt.is_empty(),
        "design_agent_system_prompt must not be empty"
    );

    // Tool-loop protocol markers.
    assert!(
        prompt.contains("get_editor_state"),
        "must reference get_editor_state"
    );
    assert!(
        prompt.contains("get_style_guide"),
        "must reference get_style_guide"
    );
    assert!(
        prompt.contains("get_guidelines"),
        "must reference get_guidelines"
    );
    assert!(
        prompt.contains("batch_design"),
        "must reference batch_design"
    );
    assert!(
        prompt.contains("script"),
        "must reference batch_design's script mode (the preferred build path)"
    );
    assert!(
        prompt.contains("I(parent") || prompt.contains("I(null"),
        "must teach the I(parent, obj) script-gen call syntax"
    );
    assert!(
        prompt.contains("unsupported in script mode") && prompt.contains("rejects the script"),
        "must teach that unsupported script mutations fail loudly"
    );
    assert!(
        prompt.contains("get_screenshot"),
        "must reference get_screenshot"
    );
    assert!(
        prompt.contains("spawn_agents"),
        "must reference spawn_agents"
    );
    assert!(
        prompt.contains("find_empty_space"),
        "must reference find_empty_space"
    );

    // DSL / node model markers.
    assert!(
        prompt.contains("placeholder"),
        "must describe placeholder scaffolding"
    );
    assert!(
        prompt.contains("instanceId"),
        "must describe instanceId path editing"
    );

    // Batch size limit.
    assert!(
        prompt.contains("25"),
        "must state the 25-operation batch limit"
    );

    // De-templating rules.
    assert!(
        prompt.contains("space-between") || prompt.contains("space_between"),
        "must teach spacing / spread rule (space-between)"
    );
    assert!(
        prompt.contains("right"),
        "must state new-screen opens to the right"
    );
    assert!(
        prompt.contains("icon"),
        "must describe icon in nav tab rule"
    );
    assert!(
        prompt.contains("label"),
        "must describe label in nav tab rule"
    );
}

#[test]
fn local_edit_skill_outputs_script_gen_protocol() {
    let skill = get_skill_by_name("local-edit").expect("local-edit skill must be registered");

    assert!(
        skill.content.contains("JavaScript program"),
        "local-edit must request a script-gen JavaScript program"
    );
    assert!(
        skill.content.contains("I(parent") || skill.content.contains("I(null"),
        "local-edit must teach the I(parent, obj) call syntax"
    );
    assert!(
        skill.content.contains("are rejected") && skill.content.contains("operations"),
        "local-edit must explain that unsupported script mutations fail loudly"
    );
    assert!(
        !skill.content.contains("json` code block"),
        "local-edit must not ask for the retired flat JSON code block"
    );
}

#[test]
fn design_agent_prompt_mentions_style_fetch() {
    let prompt = design_agent_system_prompt();
    assert!(
        prompt.contains("get_guidelines(category:\"style\""),
        "design-agent prompt must tell the loop to fetch a style guideline"
    );
    assert!(
        prompt.contains("colorPalette:\""),
        "design-agent prompt must show the flat colorPalette style param"
    );
}

#[test]
fn design_agent_prompt_requires_one_palette_source_of_truth() {
    let prompt = design_agent_system_prompt();
    assert!(
        prompt.contains("Choose exactly ONE source of truth"),
        "design-agent prompt must make palette selection explicit"
    );
    assert!(
        prompt.contains("Existing project variables/design system"),
        "existing project tokens must win over presets"
    );
    assert!(
        prompt.contains("No matching built-in"),
        "a style-guide concrete-value path must remain available"
    );
    assert!(
        prompt.contains("do not mix this palette with an unrelated preset"),
        "the prompt must forbid competing palette sources"
    );
}

#[test]
fn design_agent_and_base_layout_teach_front_to_back_overlay_order() {
    let prompt = design_agent_system_prompt();
    for marker in [
        "front-to-back by child index",
        "`children[0]` is TOPMOST",
        "M(overlayId, stackId, 0)",
        "separate EMPTY frame/rectangle image slot",
    ] {
        assert!(
            prompt.contains(marker),
            "design-agent prompt must contain {marker:?}"
        );
    }

    let layout = get_skill_by_name("layout").expect("base layout skill must be registered");
    assert!(layout.content.contains("front-to-back by array index"));
    assert!(layout.content.contains("`children[0]` is TOPMOST"));
    assert!(layout
        .content
        .contains("separate EMPTY frame/rectangle slot"));
}

#[test]
fn design_agent_prompt_distinguishes_seed_from_explicit_viewport() {
    let prompt = design_agent_system_prompt();
    assert!(prompt.contains("CONSTRUCTION seed, not automatically the final height"));
    assert!(prompt.contains("switch an ordinary content-driven page root"));
    assert!(prompt.contains("A user-specified numeric viewport is authoritative"));
    assert!(!prompt.contains("grow the height later if content exceeds it"));
    assert!(!prompt.contains("numeric viewport such as 390x844 is an AUTHORED CONTRACT"));
}

#[test]
fn design_agent_prompt_forbids_cross_call_destructive_rebuilds() {
    let prompt = design_agent_system_prompt();
    assert!(prompt.contains("Preserve the last working visual state"));
    assert!(prompt.contains("NEVER delete a visible working section in one tool call"));
    assert!(prompt.contains("ONE transactional `batch_design` call"));
    assert!(prompt.contains("keep the working section intact"));
}

#[test]
fn image_self_check_is_limited_to_rendering_integrity() {
    let prompt = design_agent_system_prompt();
    assert!(prompt.contains("Image self-check is presentation-only"));
    assert!(prompt.contains("do NOT judge or replace it based on subject relevance"));
    assert!(prompt.contains("does not restrict initial asset selection"));
    assert!(prompt.contains("an explicit user request to replace, retarget, or restyle"));

    let mobile = get_skill_by_name("mobile-ui").expect("mobile-ui skill must be registered");
    assert!(mobile.content.contains("MOBILE IMAGE PRESENTATION"));
    assert!(mobile
        .content
        .contains("During initial image-query or image-prompt authoring"));
    assert!(mobile.content.contains("coherent in subject category"));
    assert!(mobile.content.contains("verify only rendering integrity"));
    assert!(mobile
        .content
        .contains("icon or illustration tile is valid"));
    assert!(mobile
        .content
        .contains("explicit user-requested image edit remains allowed"));
    assert!(!mobile.content.contains("MOBILE IMAGE QUALITY"));
    assert!(!mobile.content.contains("random low-quality"));

    let landing = design_agent_system_prompt_with_skills(
        "Design a landing page for a climate travel service",
    );
    assert!(landing.contains("initial selection heuristic before inserting the image"));
    assert!(landing.contains("self-check is presentation-only"));
    assert!(landing.contains("automatic screenshot-driven self-check"));
    assert!(landing.contains("unless the user explicitly requests an image edit"));
    assert!(!landing.contains("If not, change it"));
    assert!(landing
        .trim_end()
        .ends_with(IMAGE_SELF_CHECK_SCOPE.trim_end()));

    let mut contextual = landing;
    contextual.push_str("\n\nEXISTING CANVAS CONTEXT");
    append_image_self_check_scope(&mut contextual);
    assert_eq!(
        contextual.matches(IMAGE_SELF_CHECK_SCOPE).count(),
        1,
        "the authoritative scope must be moved to the end, not duplicated"
    );
    assert!(contextual
        .trim_end()
        .ends_with(IMAGE_SELF_CHECK_SCOPE.trim_end()));
}

/// External agents over MCP only see contracts through `get_guidelines`;
/// the page-scroll contract and the card-board contract must be reachable
/// there (2026-09-03 matrix: K3 / Grok built a clipped 900 px root when the
/// prompt did not carry the contract).
#[test]
fn guideline_for_scroll_and_card_carry_their_contracts() {
    let scroll = guideline_for("scroll").expect("scroll guideline must be present");
    for needle in [
        "window.scrollY",
        "\"pin\": true",
        "translateY",
        "$scroll.progress",
    ] {
        assert!(scroll.contains(needle), "scroll guideline lacks {needle:?}");
    }
    for alias in ["scroll-orchestration", "parallax", "page-scroll"] {
        assert!(guideline_for(alias).is_some(), "{alias} alias resolves");
    }
    let card = guideline_for("card").expect("card guideline must be present");
    assert!(!card.trim().is_empty());
    assert!(guideline_for("cards").is_some(), "cards alias resolves");
    let topics = guideline_topics();
    assert!(topics.contains(&"scroll") && topics.contains(&"card"));
}

#[test]
fn guideline_for_icons_carries_the_catalog() {
    let icons = guideline_for("icons").expect("icons guideline must be present");
    assert!(icons.contains("icon_font"));
    assert!(icons.contains("NEVER `path`"));
    assert!(icons.contains("lucide"));
    for alias in ["icon", "icon-font", "lucide"] {
        assert!(guideline_for(alias).is_some(), "{alias} alias resolves");
    }
    assert!(guideline_topics().contains(&"icons"));
}

#[test]
fn planning_resolves_the_mobile_screen_archetype_for_phone_briefs() {
    // "外卖 App 首页（375×812）" never says mobile / 手机; the archetype package
    // still resolves from the app keyword and phone-sized brief.
    let ctx = crate::resolve_skills(
        crate::Phase::Planning,
        "外卖 App 首页（375×812）：顶部地址与搜索、分类九宫格、商家列表",
        &crate::ResolveOptions::default(),
    );
    let names: Vec<&str> = ctx.skills.iter().map(|s| s.meta.name.as_str()).collect();
    assert!(
        names.contains(&"mobile-screen-archetypes"),
        "mobile screen archetypes must resolve for an App brief; got {names:?}"
    );
    assert!(
        !names.contains(&"landing-page-predesign"),
        "an app brief must not pull the landing-page predesign; got {names:?}"
    );
    let ctx = crate::resolve_skills(
        crate::Phase::Planning,
        "为开源向量数据库做官网首页（1440，浅色）",
        &crate::ResolveOptions::default(),
    );
    let names: Vec<&str> = ctx.skills.iter().map(|s| s.meta.name.as_str()).collect();
    assert!(
        !names.contains(&"mobile-screen-archetypes"),
        "a website brief must not pull the mobile screen archetypes; got {names:?}"
    );

    let ctx = crate::resolve_skills(
        crate::Phase::Planning,
        "Mobile banking home",
        &crate::ResolveOptions::default(),
    );
    let names: Vec<&str> = ctx.skills.iter().map(|s| s.meta.name.as_str()).collect();
    assert!(
        names.contains(&"mobile-screen-archetypes"),
        "mobile banking must resolve screen archetypes; got {names:?}"
    );
}
