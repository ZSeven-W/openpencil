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
#[cfg(test)]
mod card_corpus_tests;
pub mod color;
pub mod compose;
#[cfg(test)]
mod deck_corpus_tests;
pub mod design_systems;
pub mod frontmatter;
pub mod loader;
#[cfg(test)]
mod logo_corpus_tests;
pub mod memory;
pub mod resolve;
pub mod resolve_style;
pub mod resolver;
#[cfg(test)]
mod scroll_orchestration_corpus_tests;
#[cfg(test)]
mod shader_fill_corpus_tests;
pub mod style_guide;
pub mod types;

pub use compose::compose_system_prompt;
pub use loader::{get_skill_by_name, get_skill_registry, get_skills_by_phase, SkillEntry};
pub use resolve::resolve_skills;
pub use types::{
    AgentContext, DropReason, DroppedSkill, Phase, ResolveOptions, ResolvedSkill, SkillCategory,
    SkillLoadEntry, SkillLoadReport, SkillMeta, SkillTrigger, DEFAULT_BUDGETS,
};

/// Authoritative tail for any design-agent prompt that appends task-specific
/// skills after the base protocol. Initial asset choice may follow domain
/// guidance; screenshot-driven self-check must not turn into image curation.
pub const IMAGE_SELF_CHECK_SCOPE: &str = r#"## Image self-check scope (authoritative)

During automatic screenshot-driven self-check, assess image presentation and rendering integrity only: intended photographic slots render exactly one visible image with valid bounds, crop/fit, clipping, radius, and overlay order; deliberately authored icon or illustration tiles render as intended. A raster slot is intended only when the user request or domain explicitly calls for raster media; a model-invented image slot in a text-only social card is a structural intent violation, not image curation. Do not judge or replace an authorized rendered image based on subject relevance, aesthetics, perceived quality, resolution, tone, stock-photo choice, search-query quality, generation quality, or whether another asset might look better. This restriction does not apply to initial image-query/image-prompt authoring or to an explicit user request to replace, retarget, or restyle an image."#;

/// Place the authoritative image-review scope after any task-specific prompt
/// sections, moving an existing copy instead of duplicating it.
pub fn append_image_self_check_scope(prompt: &mut String) {
    let block = format!("\n\n---\n\n{IMAGE_SELF_CHECK_SCOPE}");
    while let Some(index) = prompt.find(&block) {
        prompt.replace_range(index..index + block.len(), "");
    }
    prompt.push_str(&block);
}

/// Single source of truth for `guideline_for`'s dispatch AND for every
/// caller (e.g. `op-mcp`'s `get_guidelines` unknown-topic error) that wants
/// to name the full supported-topic set without hardcoding a second,
/// driftable copy. Each row is `(primary_name, aliases, skill_names)`: the
/// primary name plus its aliases all resolve to the same composed guideline,
/// built by concatenating `skill_names` in order (see [`compose_skills`]).
///
/// Adding a topic is a ONE-LINE change here — `guideline_for` and
/// [`guideline_topics`] both read this table, so neither can go stale
/// relative to the other.
const GUIDELINE_TOPICS: &[(&str, &[&str], &[&str])] = &[
    (
        "web-app",
        &["webapp"],
        &["product-principles", "web-app", "design-principles"],
    ),
    ("mobile", &["mobile-app"], &["mobile-app"]),
    ("code-to-design", &[], &["code-to-design"]),
    (
        "landing-page",
        &["landing"],
        &["landing-page", "design-principles"],
    ),
    (
        "dashboard",
        &["table"],
        &["dashboard", "product-principles"],
    ),
    (
        "slides",
        &["deck", "presentation"],
        &["slides", "deck-patterns"],
    ),
    ("form", &["form-ui"], &["form-ui"]),
    ("design-system", &[], &["design-system-composition"]),
    ("interactivity", &[], &["interactivity"]),
    // The page-scroll contract for external agents driving the MCP tools:
    // K3 and Grok over MCP built a 900 px clipped viewport root when the
    // prompt did not spell the contract out (2026-09-03 matrix), because
    // no guideline topic carried it.
    (
        "scroll",
        &["scroll-orchestration", "parallax", "page-scroll"],
        &["scroll-orchestration"],
    ),
    // Card boards had no MCP-side contract either (0825 A/B: the gap was
    // the contract, not the model).
    ("card", &["cards", "card-item"], &["cards"]),
];

/// Compose the named skills (in order) into one coherent guideline doc,
/// skipping any that are absent. `None` if nothing resolved.
fn compose_skills(names: &[&str]) -> Option<String> {
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

/// Return the product-design guideline text for `topic`, composed from the
/// embedded skill corpus. Mirrors Pencil's `get_guidelines(guide, …)`: each
/// topic resolves to its focused task guide plus the principle skills that
/// complete it. All content is local (embedded via `include_dir!`) — there is
/// no remote fetch.
///
/// Supported topics (aliases in parens) — see [`GUIDELINE_TOPICS`] for the
/// canonical table; [`guideline_topics`] returns the flattened name list:
/// - `"web-app"` (`webapp`) — product principles + web-app depth laws + design craft
/// - `"mobile"` (`mobile-app`) — the mobile-app three-section architecture
/// - `"code-to-design"` — agent workflow for converting frontend codebases
/// - `"landing-page"` (`landing`) — landing-page domain + design craft
/// - `"dashboard"` (`table`) — dashboard structure + product principles
/// - `"slides"` (`deck`, `presentation`) — slide layout contracts
/// - `"form"` (`form-ui`) — form-ui domain
/// - `"design-system"` — design-system composition
/// - `"interactivity"` — multi-screen navigation contract (`screen` markers
///   + `events.onTap` actions) for tappable App Mode preview
/// - `"scroll"` (`scroll-orchestration`, `parallax`, `page-scroll`) — the
///   page-scroll contract: the page root is the `$scroll` source, `pin: true`
///   pins, paint-only `translateX/Y` moves
/// - `"card"` (`cards`, `card-item`) — card-board contract
///
/// Returns `None` for any unrecognised topic so callers can produce a typed
/// "unknown topic" error without special-casing the string themselves.
pub fn guideline_for(topic: &str) -> Option<String> {
    let (_, _, skill_names) = GUIDELINE_TOPICS
        .iter()
        .find(|(name, aliases, _)| *name == topic || aliases.contains(&topic))?;
    compose_skills(skill_names)
}

/// The primary name of every topic [`guideline_for`] accepts, in table
/// order — for callers that need to name the full supported-topic set (e.g.
/// an "unknown topic" error hint) without hardcoding a copy that can drift
/// out of sync as topics are added. Aliases are omitted; each is a synonym
/// for the primary name already listed.
pub fn guideline_topics() -> Vec<&'static str> {
    GUIDELINE_TOPICS.iter().map(|(name, _, _)| *name).collect()
}

const JIAN_COMPONENTS_PLACEHOLDER: &str = "{{jianComponents}}";
const TOOL_SELECT_PLACEHOLDER: &str = "{{toolSelect}}";
const VERIFY_STEP_PLACEHOLDER: &str = "{{verifyStep}}";
const FINISH_CONDITION_PLACEHOLDER: &str = "{{finishCondition}}";

/// How the executing host verifies visual output — the one part of the
/// design-agent protocol that is host-capability-dependent. The empty-canvas
/// postmortem: the prompt made the screenshot check "mandatory, not
/// optional" and made it the END condition, while the mobile host cannot
/// render screenshots at all — a trailing capability note was not enough for
/// a weak model on a starved budget, so the protocol text itself must be
/// host-aware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignVerifyProtocol {
    /// Desktop / daemon: `get_screenshot` renders, the screenshot check is
    /// mandatory, and finishing is gated on it (the historical text).
    Screenshot,
    /// Hosts without a render arm (the mobile FFI): verification runs on
    /// `snapshot_layout` + per-batch `layoutIssues`, and finishing is gated
    /// on those instead. `get_screenshot` is not mentioned as available.
    LayoutSnapshot,
}

const SCREENSHOT_TOOL_SELECT: &str = "select:get_editor_state,get_guidelines,get_style_guide_tags,get_style_guide,get_variables,batch_get,snapshot_layout,batch_design,get_screenshot,find_empty_space,spawn_agents";
const LAYOUT_TOOL_SELECT: &str = "select:get_editor_state,get_guidelines,get_style_guide_tags,get_style_guide,get_variables,batch_get,snapshot_layout,batch_design,find_empty_space";

const LAYOUT_VERIFY_STEP: &str = "### Step 8 — Verify with `snapshot_layout`\n\n\
This host cannot render screenshots — `get_screenshot` does not exist here and its absence is \
never a reason to stop or skip building. Verify with `snapshot_layout` and each `batch_design` \
result's `layoutIssues` list instead: those are the REAL resolved layout's measured facts \
(collapsed containers, overflowing rows, clipped text). Iterate until no layout issue remains \
and every section shell is filled. Do not declare the design done while a known defect stands.";

const LAYOUT_FINISH_CONDITION: &str = "End the turn when `snapshot_layout` confirms every \
section is present and filled and the last `batch_design` results carry no unresolved \
`layoutIssues`.";

const SCREENSHOT_FINISH_CONDITION: &str = "End the turn when the `get_screenshot` output \
verifies the design is complete and visually polished.";

/// The design-agent template with the shared `jian-components` contract
/// mounted and the host-dependent placeholders still in place.
fn design_agent_template() -> &'static str {
    static TEMPLATE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    TEMPLATE.get_or_init(|| {
        let template = phase_corpus_dir()
            .and_then(|dir| dir.get_file("phases/agent/design-agent.md"))
            .and_then(|f| f.contents_utf8())
            .expect(
                "skills/phases/agent/design-agent.md must be embedded in the op-ai-skills corpus \
                 (a slim-corpus build must never reach the design-agent template)",
            );
        for placeholder in [
            JIAN_COMPONENTS_PLACEHOLDER,
            TOOL_SELECT_PLACEHOLDER,
            VERIFY_STEP_PLACEHOLDER,
            FINISH_CONDITION_PLACEHOLDER,
        ] {
            assert!(
                template.contains(placeholder),
                "design-agent.md must carry the {placeholder} mount point"
            );
        }
        let widgets = get_skill_by_name("jian-components")
            .expect("jian-components must be registered for the design-agent prompt");
        template.replace(JIAN_COMPONENTS_PLACEHOLDER, widgets.content.trim())
    })
}

/// The screenshot-verification Step 8 text (historical wording, embedded in
/// its own corpus file so the two variants stay reviewable side by side).
const SCREENSHOT_VERIFY_STEP: &str = include_str!("design_agent_verify_screenshot.md");

/// Return the design-agent tool-loop system prompt for a given host
/// verification protocol.
pub fn design_agent_system_prompt_for(verify: DesignVerifyProtocol) -> String {
    let (tool_select, verify_step, finish) = match verify {
        DesignVerifyProtocol::Screenshot => (
            SCREENSHOT_TOOL_SELECT,
            SCREENSHOT_VERIFY_STEP.trim(),
            SCREENSHOT_FINISH_CONDITION,
        ),
        DesignVerifyProtocol::LayoutSnapshot => (
            LAYOUT_TOOL_SELECT,
            LAYOUT_VERIFY_STEP,
            LAYOUT_FINISH_CONDITION,
        ),
    };
    design_agent_template()
        .replace(TOOL_SELECT_PLACEHOLDER, tool_select)
        .replace(VERIFY_STEP_PLACEHOLDER, verify_step)
        .replace(FINISH_CONDITION_PLACEHOLDER, finish)
}

/// Return the system prompt for the design agentic tool-loop (desktop
/// screenshot protocol — the historical text, byte-stable for existing
/// callers).
///
/// The protocol template is embedded from
/// `skills/phases/agent/design-agent.md`. Its native-widget placeholder is
/// expanded from the same `jian-components` generation skill used by the
/// single-shot pipeline, so builtin turns and spawned design sub-agents cannot
/// drift onto a second interactive-control contract.
///
/// Panics at startup if the embedded file is missing or not valid UTF-8
/// (a build-time invariant: the file is checked in alongside this crate).
pub fn design_agent_system_prompt() -> &'static str {
    static PROMPT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PROMPT.get_or_init(|| design_agent_system_prompt_for(DesignVerifyProtocol::Screenshot))
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
/// itself. The one shared protocol dependency, `jian-components`, is already
/// mounted inside [`design_agent_system_prompt`] because native widget syntax
/// must be identical across both generation paths. The model can still pull
/// topic guides on demand via `get_guidelines`. Budget trimming is the
/// resolver's (per-skill budgets + phase cap), so a keyword-rich prompt cannot
/// balloon the system prompt.
pub fn design_agent_system_prompt_with_skills(user_message: &str) -> String {
    design_agent_system_prompt_with_skills_for(user_message, DesignVerifyProtocol::Screenshot)
}

/// Host-aware variant of [`design_agent_system_prompt_with_skills`]: same
/// depth-skill assembly over the protocol for the given verification
/// protocol (mobile passes [`DesignVerifyProtocol::LayoutSnapshot`]).
pub fn design_agent_system_prompt_with_skills_for(
    user_message: &str,
    verify: DesignVerifyProtocol,
) -> String {
    let base = design_agent_system_prompt_for(verify);
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
    let mut prompt = base;
    if !depth.is_empty() {
        prompt.push_str("\n\n---\n\n## Product-Design Depth (applies to this task)\n\n");
        prompt.push_str(&depth.join("\n\n"));
    }
    append_image_self_check_scope(&mut prompt);
    prompt
}

/// The embedded `skills/` corpus — domain / knowledge / phase skill
/// markdown plus the `style-guides/` subtree. Parsed into the skill
/// registry on first access (see [`loader`]).
// The slim gate is feature AND target: cargo feature unification opens
// the feature for every crate in a workspace build that includes
// op-host-web, and without the target gate that build would strip the
// corpus out from under the desktop and daemon. Only an actual wasm32
// compilation slims.
#[cfg(not(all(feature = "slim-corpus", target_arch = "wasm32")))]
pub static SKILLS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/skills");

/// slim-corpus: only the style-guides subtree is embedded. Everything
/// that walks [`SKILLS`] must go through [`style_guides_dir`] /
/// [`phase_corpus_dir`] so both embeds answer the same questions.
#[cfg(all(feature = "slim-corpus", target_arch = "wasm32"))]
pub static STYLE_GUIDES_ONLY: Dir<'static> =
    include_dir!("$CARGO_MANIFEST_DIR/skills/style-guides");

/// The style-guides subtree, wherever this build embedded it.
pub fn style_guides_dir() -> Option<&'static Dir<'static>> {
    #[cfg(not(all(feature = "slim-corpus", target_arch = "wasm32")))]
    {
        SKILLS.get_dir("style-guides")
    }
    #[cfg(all(feature = "slim-corpus", target_arch = "wasm32"))]
    {
        Some(&STYLE_GUIDES_ONLY)
    }
}

/// The phase/domain/knowledge corpus root, absent on a slim build — the
/// browser never composes prompts, so callers must treat `None` as "not
/// this build's job", never as an error.
pub fn phase_corpus_dir() -> Option<&'static Dir<'static>> {
    #[cfg(not(all(feature = "slim-corpus", target_arch = "wasm32")))]
    {
        Some(&SKILLS)
    }
    #[cfg(all(feature = "slim-corpus", target_arch = "wasm32"))]
    {
        None
    }
}

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
#[path = "lib_tests.rs"]
mod lib_tests;
