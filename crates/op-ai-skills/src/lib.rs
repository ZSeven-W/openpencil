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
    AgentContext, Phase, ResolveOptions, ResolvedSkill, SkillCategory, SkillMeta, SkillTrigger,
    DEFAULT_BUDGETS,
};

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
            count_md(&SKILLS) >= 90,
            "expected the full skill corpus to embed, found {}",
            count_md(&SKILLS)
        );
    }
}
