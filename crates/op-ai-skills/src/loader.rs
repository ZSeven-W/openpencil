//! Skill registry — parses the embedded skill markdown corpus into an
//! in-memory `Vec<SkillEntry>` on first access. Mirrors the TS
//! `engine/loader.ts` registry accessors.
//!
//! The `style-guides/` subtree is excluded here — those files are not
//! phase skills; they are loaded separately by [`crate::style_guide`].

use std::sync::OnceLock;

use include_dir::Dir;

use crate::frontmatter::parse_skill_frontmatter;
use crate::types::{Phase, SkillMeta};
use crate::SKILLS;

/// One registered skill — parsed frontmatter plus its markdown body.
#[derive(Debug, Clone)]
pub struct SkillEntry {
    pub meta: SkillMeta,
    pub content: String,
}

static REGISTRY: OnceLock<Vec<SkillEntry>> = OnceLock::new();

/// Recursively collect skill entries, skipping the `style-guides`
/// subtree.
fn collect(dir: &Dir, out: &mut Vec<SkillEntry>) {
    for sub in dir.dirs() {
        let name = sub
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if name == "style-guides" {
            continue;
        }
        collect(sub, out);
    }
    for file in dir.files() {
        let is_md = file.path().extension().map(|e| e == "md").unwrap_or(false);
        if !is_md {
            continue;
        }
        if let Some(text) = file.contents_utf8() {
            if let Some((meta, content)) = parse_skill_frontmatter(text) {
                out.push(SkillEntry { meta, content });
            }
        }
    }
}

fn build_registry() -> Vec<SkillEntry> {
    let mut out = Vec::new();
    collect(&SKILLS, &mut out);
    out
}

/// The full skill registry, parsed + cached on first call.
pub fn get_skill_registry() -> &'static [SkillEntry] {
    REGISTRY.get_or_init(build_registry)
}

/// Every skill tagged with `phase`.
pub fn get_skills_by_phase(phase: Phase) -> Vec<&'static SkillEntry> {
    get_skill_registry()
        .iter()
        .filter(|e| e.meta.phase.contains(&phase))
        .collect()
}

/// Look up a single skill by its frontmatter `name`.
pub fn get_skill_by_name(name: &str) -> Option<&'static SkillEntry> {
    get_skill_registry().iter().find(|e| e.meta.name == name)
}

/// Look up a skill's metadata by name — convenience over
/// [`get_skill_by_name`].
pub fn get_skill_meta(name: &str) -> Option<&'static SkillMeta> {
    get_skill_by_name(name).map(|e| &e.meta)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_loads_the_skill_corpus() {
        let reg = get_skill_registry();
        // ~45 phase / domain / knowledge skills (the corpus minus the
        // ~50 style guides).
        assert!(reg.len() >= 40, "registry too small: {}", reg.len());
        // Every entry parsed a non-empty name + at least one phase.
        for e in reg {
            assert!(!e.meta.name.is_empty());
            assert!(!e.meta.phase.is_empty());
        }
    }

    #[test]
    fn no_style_guide_leaked_into_the_registry() {
        // Style guides have no `phase` field, so even if one slipped
        // through the dir filter `parse_skill_frontmatter` would drop
        // it — assert the filter holds regardless.
        let reg = get_skill_registry();
        assert!(reg.iter().all(|e| !e.meta.phase.is_empty()));
    }

    #[test]
    fn generation_phase_has_skills() {
        assert!(!get_skills_by_phase(Phase::Generation).is_empty());
        assert!(!get_skills_by_phase(Phase::Planning).is_empty());
    }

    #[test]
    fn generation_format_emits_text_input_widgets() {
        // Phase 1: form fields must be generated as real `text_input`
        // nodes (interactive in preview), not `role=input` mockup frames.
        // The jsonl-format skill's node-type list must mention text_input.
        let mentions = get_skills_by_phase(Phase::Generation)
            .iter()
            .any(|s| s.content.contains("text_input"));
        assert!(
            mentions,
            "the generation format must list text_input as an emittable node"
        );
    }

    #[test]
    fn mobile_skill_carries_non_template_nav_and_spacing_rules() {
        let skill = get_skill_by_name("mobile-app").expect("mobile-app skill present");
        assert!(skill.content.contains("Mobile top rhythm"));
        assert!(skill.content.contains("Do not force bottom navigation"));
        assert!(skill.content.contains("Not a floating pill"));
        assert!(skill
            .content
            .contains("Do not repeat the same predictable mobile stack"));
        assert!(
            !skill.content.contains("BOTTOM TAB BAR — PILL STYLE"),
            "mobile skill must not force the old pill-tab template"
        );
    }

    #[test]
    fn mobile_style_guides_do_not_force_pill_nav_template() {
        for guide in crate::style_guide::style_guide_registry() {
            if guide.platform == crate::style_guide::Platform::Mobile {
                assert!(
                    !guide.content.contains("Pill tab bar"),
                    "{} must not force the old pill tab bar template",
                    guide.name
                );
                assert!(
                    !guide.content.contains("bottom navigation pill"),
                    "{} must not force a detached pill nav",
                    guide.name
                );
            }
        }
    }

    #[test]
    fn block_list_and_next_line_array_triggers_load_non_empty() {
        // role-definitions / copywriting open the keyword array on the
        // line after `keywords:`; cjk-typography uses a `- ` block
        // list. All three must parse to a non-empty keyword trigger
        // (a regression here silently disables the skill).
        for name in ["role-definitions", "copywriting", "cjk-typography"] {
            let skill =
                get_skill_by_name(name).unwrap_or_else(|| panic!("{name} should be registered"));
            match &skill.meta.trigger {
                crate::types::SkillTrigger::Keywords(kw) => {
                    assert!(!kw.is_empty(), "{name} keyword trigger must not be empty");
                }
                other => panic!("{name}: expected keyword trigger, got {other:?}"),
            }
        }
    }

    #[test]
    fn lookup_known_skill_by_name() {
        let form = get_skill_by_name("form-ui").expect("form-ui skill present");
        assert_eq!(form.meta.name, "form-ui");
        assert!(get_skill_by_name("definitely-not-a-skill").is_none());
    }
}
