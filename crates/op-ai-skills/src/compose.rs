//! The single skill-text expander shared by the desktop transport and the
//! backend proxy. Maps skill NAMES to their markdown content, concatenates
//! in order, and trims to a character budget so the same names always
//! yield byte-identical prompts on every host.

use crate::loader::get_skill_by_name;

/// Compose a system prompt from skill names. Unknown names are skipped
/// (the corpus is the source of truth). `budget_chars` caps the result
/// length (0 = unlimited); when exceeded, whole skills are dropped from
/// the END so earlier (higher-priority) skills survive intact.
pub fn compose_system_prompt(names: &[&str], budget_chars: usize) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for name in names {
        if let Some(entry) = get_skill_by_name(name) {
            parts.push(entry.content.as_str());
        }
    }
    let mut out = String::new();
    for part in parts {
        let sep_len = if out.is_empty() { 0 } else { 2 };
        if budget_chars != 0 && out.len() + sep_len + part.len() > budget_chars {
            break;
        }
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(part);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::get_skill_by_name;

    #[test]
    fn composes_known_codegen_skills_in_order() {
        let prompt = compose_system_prompt(&["codegen-planning"], 0);
        assert!(!prompt.is_empty());
        let direct = get_skill_by_name("codegen-planning").unwrap();
        assert!(prompt.contains(direct.content.trim_end()));
    }

    #[test]
    fn unknown_names_are_skipped() {
        let only_known = compose_system_prompt(&["codegen-planning"], 0);
        let with_bogus = compose_system_prompt(&["codegen-planning", "not-a-skill"], 0);
        assert_eq!(only_known, with_bogus);
    }

    #[test]
    fn budget_drops_trailing_skills_whole() {
        let first = get_skill_by_name("codegen-chunk").unwrap();
        let budget = first.content.len() + 1; // not enough for a second skill
        let prompt = compose_system_prompt(&["codegen-chunk", "codegen-react"], budget);
        assert_eq!(prompt, first.content);
    }

    #[test]
    fn empty_names_yield_empty_prompt() {
        assert_eq!(compose_system_prompt(&[], 0), "");
    }
}
