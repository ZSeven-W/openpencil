//! OpenPencil AI skill engine — a Rust port of the TS `pen-ai-skills`
//! package's phase-driven prompt-skill resolution engine.
//!
//! The pipeline mirrors `pen-ai-skills/src/engine`:
//!  1. **phase filter** — keep skills tagged with the requested
//!     [`Phase`] (planning / generation / validation / maintenance);
//!  2. **intent match** — keyword / flag triggers narrow that set
//!     ([`resolver`]);
//!  3. **memory injection** — design context + generation history
//!     fill `{{placeholder}}`s ([`memory`]);
//!  4. **budget trim** — per-skill caps + category priority keep the
//!     prompt within the phase's token budget ([`budget`]).
//!
//! The skill + style-guide markdown corpus is kept verbatim from the
//! TS package and embedded at compile time via `include_dir!`, so the
//! engine ships self-contained with no runtime file IO and stays
//! usable on wasm32 targets.

use include_dir::{include_dir, Dir};

pub mod budget;
pub mod color;
pub mod compose;
pub mod frontmatter;
pub mod loader;
pub mod memory;
pub mod resolve;
pub mod resolve_style;
pub mod resolver;
pub mod style_guide;
pub mod types;

pub use compose::compose_system_prompt;
pub use loader::{get_skill_by_name, get_skill_registry, get_skills_by_phase, SkillEntry};
pub use resolve::resolve_skills;
pub use types::{
    AgentContext, DropReason, DroppedSkill, Phase, ResolveOptions, ResolvedSkill, SkillCategory,
    SkillLoadEntry, SkillLoadReport, SkillMeta, SkillTrigger, DEFAULT_BUDGETS,
};

/// Return the product-design guideline text for `topic`, composed from the
/// embedded skill corpus. Mirrors Pencil's `get_guidelines(guide, …)`: each
/// topic resolves to its focused task guide plus the principle skills that
/// complete it. All content is local (embedded via `include_dir!`) — there is
/// no remote fetch.
///
/// Supported topics (aliases in parens):
/// - `"web-app"` (`webapp`) — product principles + web-app depth laws + design craft
/// - `"mobile"` (`mobile-app`) — the mobile-app three-section architecture
/// - `"code-to-design"` — agent workflow for converting frontend codebases
/// - `"landing-page"` (`landing`) — landing-page domain + design craft
/// - `"dashboard"` (`table`) — dashboard structure + product principles
/// - `"slides"` (`deck`, `presentation`) — slide layout contracts
/// - `"form"` (`form-ui`) — form-ui domain
/// - `"design-system"` — design-system composition
///
/// Returns `None` for any unrecognised topic so callers can produce a typed
/// "unknown topic" error without special-casing the string themselves.
pub fn guideline_for(topic: &str) -> Option<String> {
    // Compose the named skills (in order) into one coherent guideline doc,
    // skipping any that are absent. `None` if nothing resolved.
    fn compose(names: &[&str]) -> Option<String> {
        let parts: Vec<&str> = names
            .iter()
            .filter_map(|&n| get_skill_by_name(n).map(|s| s.content.trim()))
            .filter(|c| !c.is_empty())
            .collect();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n"))
        }
    }
    match topic {
        "web-app" | "webapp" => compose(&["product-principles", "web-app", "design-principles"]),
        "mobile" | "mobile-app" => compose(&["mobile-app"]),
        "code-to-design" => compose(&["code-to-design"]),
        "landing-page" | "landing" => compose(&["landing-page", "design-principles"]),
        "dashboard" | "table" => compose(&["dashboard", "product-principles"]),
        "slides" | "deck" | "presentation" => compose(&["slides"]),
        "form" | "form-ui" => compose(&["form-ui"]),
        "design-system" => compose(&["design-system"]),
        _ => None,
    }
}

/// Return the system prompt for the design agentic tool-loop.
///
/// The prompt is embedded at compile time from
/// `skills/phases/agent/design-agent.md` and returned verbatim — no
/// frontmatter stripping, no template substitution. Callers (e.g. the
/// `BuiltInProvider` `QueryLoop`) pass it directly as the system message
/// for a design turn.
///
/// Panics at startup if the embedded file is missing or not valid UTF-8
/// (a build-time invariant: the file is checked in alongside this crate).
pub fn design_agent_system_prompt() -> &'static str {
    SKILLS
        .get_file("phases/agent/design-agent.md")
        .and_then(|f| f.contents_utf8())
        .expect("skills/phases/agent/design-agent.md must be embedded in the op-ai-skills corpus")
}

/// Return the design-agent tool-loop system prompt with the prompt-matched
/// product-depth skills appended.
///
/// The bare [`design_agent_system_prompt`] carries the tool-loop PROTOCOL
/// (read → guidelines → batch_design → screenshot) but none of the domain
/// depth the orchestrator injects per subtask — a loop-generated dashboard
/// never saw `dashboard.md`'s density floors, a mobile screen never saw the
/// three-section architecture. This variant resolves the generation-phase
/// skill set for `user_message` and appends ONLY the content skills:
///
/// - every intent-matched `domain` skill (dashboard / web-app / mobile-app /
///   landing-page / slides / form-ui / cjk-typography / anti-slop / …), and
/// - the two always-on principle bases (`product-principles`,
///   `design-principles`).
///
/// Output-protocol skills (schema / layout / text-rules / codegen-* / …)
/// are deliberately NOT appended: the loop's protocol is the tool loop
/// itself, and the model can still pull topic guides on demand via
/// `get_guidelines`. Budget trimming is the resolver's (per-skill budgets +
/// phase cap), so a keyword-rich prompt cannot balloon the system prompt.
pub fn design_agent_system_prompt_with_skills(user_message: &str) -> String {
    let base = design_agent_system_prompt();
    let ctx = resolve::resolve_skills(
        types::Phase::Generation,
        user_message,
        &types::ResolveOptions::default(),
    );
    let depth: Vec<&str> = ctx
        .skills
        .iter()
        .filter(|s| {
            s.meta.category == types::SkillCategory::Domain
                || matches!(
                    s.meta.name.as_str(),
                    "product-principles" | "design-principles"
                )
        })
        .map(|s| s.content.trim())
        .filter(|c| !c.is_empty())
        .collect();
    if depth.is_empty() {
        return base.to_string();
    }
    format!(
        "{base}\n\n---\n\n## Product-Design Depth (applies to this task)\n\n{}",
        depth.join("\n\n")
    )
}

/// The embedded `skills/` corpus — domain / knowledge / phase skill
/// markdown plus the `style-guides/` subtree. Parsed into the skill
/// registry on first access (see [`loader`]).
pub static SKILLS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/skills");

/// The embedded P2 style catalog. This is deliberately separate from
/// [`SKILLS`] so catalog entries cannot be parsed as phase skills.
pub static STYLE_CATALOG: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/style_catalog");

/// Recursively count `.md` files in an embedded directory.
#[cfg(test)]
pub(crate) fn count_md(dir: &Dir) -> usize {
    let mut n = dir
        .files()
        .filter(|f| f.path().extension().map(|e| e == "md").unwrap_or(false))
        .count();
    for sub in dir.dirs() {
        n += count_md(sub);
    }
    n
}

#[cfg(test)]
mod tests {
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
        let prompt = design_agent_system_prompt_with_skills(
            "Design an analytics dashboard with a client table",
        );
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
        let prompt =
            design_agent_system_prompt_with_skills("design a mobile fitness app home screen");
        assert!(
            prompt.contains("THREE-SECTION ARCHITECTURE"),
            "mobile-app domain skill must resolve for a mobile ask"
        );
    }

    #[test]
    fn design_agent_prompt_with_skills_excludes_protocol_skills() {
        // Output-protocol skills (schema / layout / codegen) must NOT ride
        // into the tool-loop prompt — the loop's protocol is the tool loop
        // itself (design-agent.md), and doubling protocols confuses weak
        // models. `layout.md`'s canonical opener is the marker.
        let prompt = design_agent_system_prompt_with_skills(
            "Design an analytics dashboard with a client table",
        );
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
        // slides resolves to the slide layout contracts.
        let sl = guideline_for("slides").expect("slides guideline present");
        assert!(
            sl.to_uppercase().contains("SLIDE"),
            "slides must include slide guidance"
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
            prompt.contains("no-op") || prompt.contains("NO-OP"),
            "must teach that C/U/D/M/R/G and console are no-op stubs inside a script"
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
            skill.content.contains("NO-OP") || skill.content.contains("no-op"),
            "local-edit must explain script no-op stubs"
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
    fn design_agent_prompt_says_bake_not_variables() {
        let prompt = design_agent_system_prompt();
        assert!(
            prompt.contains("Treat the returned TokenMap as reference values only"),
            "design-agent prompt must frame TokenMap values as references"
        );
        assert!(
            prompt.contains("BAKE concrete values"),
            "design-agent prompt must require baking concrete values into nodes"
        );
        assert!(
            prompt.contains("Do NOT create document variables"),
            "design-agent prompt must prohibit creating document variables"
        );
        assert!(
            prompt.contains("Do NOT call `set_variables`"),
            "design-agent prompt must prohibit set_variables"
        );
    }
}
