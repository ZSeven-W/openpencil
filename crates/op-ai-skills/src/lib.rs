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
pub mod compose;
pub mod frontmatter;
pub mod loader;
pub mod memory;
pub mod resolve;
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

/// The embedded `skills/` corpus — domain / knowledge / phase skill
/// markdown plus the `style-guides/` subtree. Parsed into the skill
/// registry on first access (see [`loader`]).
pub static SKILLS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/skills");

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
}
