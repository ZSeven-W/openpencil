//! YAML-subset frontmatter parser for skill / style-guide markdown.
//!
//! The TS package parses frontmatter with `gray-matter` (a full YAML
//! engine). The skill corpus only uses a small slice of YAML — scalar
//! `key: value` pairs, inline `[a, b]` arrays (some spanning several
//! lines with `#` comments, e.g. `trigger.keywords`), and one nested
//! level under `trigger:`. This hand-rolled parser covers exactly
//! that slice, keeping the crate dependency-free.

use crate::types::{Phase, SkillCategory, SkillMeta, SkillTrigger};

/// Split a markdown file into its `---`-fenced frontmatter block and
/// the trimmed body. Returns `None` when the file has no frontmatter.
pub(crate) fn split_frontmatter(raw: &str) -> Option<(String, String)> {
    let mut lines = raw.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut fm = String::new();
    let mut body = String::new();
    let mut found_end = false;
    for line in lines {
        if found_end {
            body.push_str(line);
            body.push('\n');
        } else if line.trim() == "---" {
            found_end = true;
        } else {
            fm.push_str(line);
            fm.push('\n');
        }
    }
    if !found_end {
        return None;
    }
    Some((fm, body.trim().to_string()))
}

/// Strip a trailing `# comment` from one frontmatter line. A `#`
/// outside a quoted string starts a comment. The skill / style-guide
/// frontmatter schema carries no `#` in any value (hex colours live
/// in the markdown body, not the frontmatter), so this is sound.
pub(crate) fn strip_comment(line: &str) -> String {
    let mut out = String::new();
    let mut quote: Option<char> = None;
    for ch in line.chars() {
        match quote {
            Some(q) => {
                out.push(ch);
                if ch == q {
                    quote = None;
                }
            }
            None => {
                if ch == '\'' || ch == '"' {
                    quote = Some(ch);
                    out.push(ch);
                } else if ch == '#' {
                    break;
                } else {
                    out.push(ch);
                }
            }
        }
    }
    out
}

/// True when `s` contains at least one `[` and `[`/`]` counts match.
fn brackets_balanced(s: &str) -> bool {
    let open = s.matches('[').count();
    let close = s.matches(']').count();
    open > 0 && open == close
}

/// Strip a matching pair of surrounding single / double quotes.
pub(crate) fn unquote(s: &str) -> String {
    let t = s.trim();
    let bytes = t.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'\'' || first == b'"') && first == last {
            return t[1..t.len() - 1].to_string();
        }
    }
    t.to_string()
}

/// Parse an inline `[a, b, c]` array into trimmed, unquoted, non-empty
/// items. Exactly one outer `[` / `]` is stripped (not every trailing
/// bracket) so an item that itself contains `]` survives.
pub(crate) fn parse_array(s: &str) -> Vec<String> {
    let t = s.trim();
    let inner = t.strip_prefix('[').unwrap_or(t);
    let inner = inner.strip_suffix(']').unwrap_or(inner);
    inner
        .split(',')
        .map(|x| unquote(x.trim()))
        .filter(|x| !x.is_empty())
        .collect()
}

/// Index of the first non-blank line at or after `from`.
fn next_nonblank(lines: &[String], from: usize) -> Option<usize> {
    (from..lines.len()).find(|&i| !lines[i].trim().is_empty())
}

/// One `key: value` pair extracted from a frontmatter line, with the
/// value's brackets already balanced across continuation lines.
struct Entry {
    indent: usize,
    key: String,
    value: String,
}

/// Walk the comment-stripped frontmatter into `(indent, key, value)`
/// entries, joining multi-line `[...]` arrays into one value.
fn entries(fm: &str) -> Vec<Entry> {
    let lines: Vec<String> = fm.lines().map(strip_comment).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim_end();
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();
        let Some(colon) = trimmed.find(':') else {
            i += 1;
            continue;
        };
        let key = trimmed[..colon].trim().to_string();
        let mut value = trimmed[colon + 1..].trim().to_string();
        if value.starts_with('[') && !brackets_balanced(&value) {
            // A `[` array opened on this line continues onto the
            // following lines — accumulate until brackets balance.
            while i + 1 < lines.len() && !brackets_balanced(&value) {
                i += 1;
                value.push(' ');
                value.push_str(lines[i].trim());
            }
        } else if value.is_empty() {
            // An empty value may be a parent key (`trigger:`), or a
            // YAML continuation: an inline `[...]` array or a `- `
            // block list starting on the next line.
            if let Some(next) = next_nonblank(&lines, i + 1) {
                let nt = lines[next].trim();
                if nt.starts_with('[') {
                    // Inline array beginning on the following line.
                    i = next;
                    value = lines[i].trim().to_string();
                    while i + 1 < lines.len() && !brackets_balanced(&value) {
                        i += 1;
                        value.push(' ');
                        value.push_str(lines[i].trim());
                    }
                } else if nt.starts_with("- ") {
                    // Block list — collect consecutive `- item` lines
                    // into a synthetic inline array.
                    let mut items: Vec<String> = Vec::new();
                    let mut j = next;
                    while j < lines.len() {
                        match lines[j].trim().strip_prefix("- ") {
                            Some(item) => {
                                items.push(item.trim().to_string());
                                j += 1;
                            }
                            None => break,
                        }
                    }
                    i = j - 1;
                    value = format!("[{}]", items.join(","));
                }
            }
        }
        out.push(Entry { indent, key, value });
        i += 1;
    }
    out
}

/// Parse a skill markdown file into its [`SkillMeta`] + trimmed body.
/// Returns `None` when the file has no frontmatter or is missing the
/// required `name` / `phase` fields (TS parity: such files are
/// skipped with a warning).
pub fn parse_skill_frontmatter(raw: &str) -> Option<(SkillMeta, String)> {
    let (fm, body) = split_frontmatter(raw)?;
    let mut name: Option<String> = None;
    let mut description = String::new();
    let mut phase: Vec<Phase> = Vec::new();
    let mut trigger = SkillTrigger::Always;
    let mut priority: i32 = 50;
    let mut budget: u32 = 2000;
    let mut category = SkillCategory::Domain;
    let mut model_families: Vec<String> = Vec::new();

    for e in entries(&fm) {
        if e.indent == 0 {
            match e.key.as_str() {
                "name" => name = Some(unquote(&e.value)),
                "description" => description = unquote(&e.value),
                "phase" => {
                    phase = if e.value.starts_with('[') {
                        parse_array(&e.value)
                            .iter()
                            .filter_map(|s| Phase::from_str(s))
                            .collect()
                    } else {
                        Phase::from_str(&unquote(&e.value)).into_iter().collect()
                    };
                }
                "priority" => priority = unquote(&e.value).parse().unwrap_or(50),
                "budget" => budget = unquote(&e.value).parse().unwrap_or(2000),
                "category" => category = SkillCategory::from_str(&unquote(&e.value)),
                // Model-family gate for the DS P2-a overlay mechanism — an
                // inline `[deepseek]` array or a `- item` block list, both
                // covered by the shared `entries` / `parse_array` path.
                "model_families" => model_families = parse_array(&e.value),
                // `trigger:` itself carries no value worth keeping —
                // `null` / empty both mean `Always`; the nested
                // keywords / flags lines below set the real trigger.
                _ => {}
            }
        } else {
            // Indented entries belong to `trigger:` — it is the only
            // key in the skill schema with nested children.
            match e.key.as_str() {
                "keywords" => trigger = SkillTrigger::Keywords(parse_array(&e.value)),
                "flags" => trigger = SkillTrigger::Flags(parse_array(&e.value)),
                _ => {}
            }
        }
    }

    let name = name?;
    if phase.is_empty() {
        return None;
    }
    Some((
        SkillMeta {
            name,
            description,
            phase,
            trigger,
            priority,
            budget,
            category,
            model_families,
        },
        body,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixtures are plain multi-line string literals — NOT `\`-joined
    // ones, which would strip the leading whitespace that the nested
    // `trigger:` block depends on.
    const KEYWORD_SKILL: &str = "---
name: form-ui
description: Form, input, and interactive element design guidelines
phase: [generation]
trigger:
  keywords: [
      # English: form-specific
      form,
      contact form,
      # auth
      login,
      表单,
    ]
priority: 30
budget: 1500
category: domain
---

BODY TEXT HERE
";

    #[test]
    fn parses_a_keyword_trigger_skill_with_multiline_array() {
        let (meta, body) = parse_skill_frontmatter(KEYWORD_SKILL).expect("parses");
        assert_eq!(meta.name, "form-ui");
        assert_eq!(
            meta.description,
            "Form, input, and interactive element design guidelines"
        );
        assert_eq!(meta.phase, vec![Phase::Generation]);
        assert_eq!(meta.priority, 30);
        assert_eq!(meta.budget, 1500);
        assert_eq!(meta.category, SkillCategory::Domain);
        match &meta.trigger {
            SkillTrigger::Keywords(kw) => {
                assert!(kw.contains(&"form".to_string()));
                assert!(kw.contains(&"contact form".to_string()));
                assert!(kw.contains(&"login".to_string()));
                assert!(kw.contains(&"表单".to_string()));
                // The `#` comment lines must not leak in as keywords.
                assert!(!kw.iter().any(|k| k.contains('#')));
            }
            other => panic!("expected keyword trigger, got {other:?}"),
        }
        assert_eq!(body, "BODY TEXT HERE");
    }

    #[test]
    fn parses_keywords_as_block_list() {
        // YAML block-list (`- item`) trigger syntax.
        let raw = "---
name: cjk
description: d
phase: [generation]
trigger:
  keywords:
    - form
    - 表单
    - login
priority: 25
budget: 500
category: domain
---
body
";
        let (meta, _) = parse_skill_frontmatter(raw).expect("parses");
        assert_eq!(
            meta.trigger,
            SkillTrigger::Keywords(vec!["form".into(), "表单".into(), "login".into()])
        );
    }

    #[test]
    fn parses_keywords_array_opening_on_next_line() {
        // The `[` opens on the line AFTER `keywords:` (role-definitions
        // / copywriting use this form).
        let raw = "---
name: role-defs
description: d
phase: [generation]
trigger:
  keywords:
    [
      landing,
      hero,
      表格,
    ]
priority: 35
budget: 2000
category: knowledge
---
body
";
        let (meta, _) = parse_skill_frontmatter(raw).expect("parses");
        assert_eq!(
            meta.trigger,
            SkillTrigger::Keywords(vec!["landing".into(), "hero".into(), "表格".into()])
        );
    }

    #[test]
    fn parses_a_flag_trigger_skill() {
        let raw = "---
name: codegen-swiftui
description: SwiftUI code generation rules
phase: [generation]
trigger:
  flags: [isCodeGen]
priority: 20
budget: 2000
category: knowledge
---
content
";
        let (meta, _) = parse_skill_frontmatter(raw).expect("parses");
        assert_eq!(meta.category, SkillCategory::Knowledge);
        assert_eq!(meta.trigger, SkillTrigger::Flags(vec!["isCodeGen".into()]));
    }

    #[test]
    fn parses_a_base_skill_with_null_trigger() {
        let raw = "---
name: icon-catalog
description: Icon usage rules and available icon names
phase: [generation]
trigger: null
priority: 20
budget: 1000
category: base
---
catalog
";
        let (meta, _) = parse_skill_frontmatter(raw).expect("parses");
        assert_eq!(meta.category, SkillCategory::Base);
        // `trigger: null` is the unconditional `Always` arm.
        assert_eq!(meta.trigger, SkillTrigger::Always);
    }

    #[test]
    fn multi_phase_array() {
        let raw = "---
name: x
description: d
phase: [maintenance, generation]
---
b
";
        let (meta, _) = parse_skill_frontmatter(raw).expect("parses");
        assert_eq!(meta.phase, vec![Phase::Maintenance, Phase::Generation]);
        // Defaults apply when the field is absent.
        assert_eq!(meta.priority, 50);
        assert_eq!(meta.budget, 2000);
        assert_eq!(meta.trigger, SkillTrigger::Always);
    }

    #[test]
    fn parses_a_model_families_gate() {
        // DS P2-a overlay frontmatter: an inline family list parses into
        // `SkillMeta.model_families`.
        let raw = "---
name: overlay
description: d
phase: [generation]
model_families: [deepseek, deepseek-reasoner]
category: domain
---
body
";
        let (meta, _) = parse_skill_frontmatter(raw).expect("parses");
        assert_eq!(
            meta.model_families,
            vec!["deepseek".to_string(), "deepseek-reasoner".to_string()]
        );
    }

    #[test]
    fn model_families_parses_as_block_list() {
        let raw = "---
name: overlay
description: d
phase: [generation]
model_families:
  - deepseek
  - kimi
category: domain
---
body
";
        let (meta, _) = parse_skill_frontmatter(raw).expect("parses");
        assert_eq!(
            meta.model_families,
            vec!["deepseek".to_string(), "kimi".to_string()]
        );
    }

    #[test]
    fn missing_model_families_defaults_to_ungated() {
        // No field = empty gate = the historical behaviour for every skill.
        let (meta, _) = parse_skill_frontmatter(KEYWORD_SKILL).expect("parses");
        assert!(meta.model_families.is_empty());
    }

    #[test]
    fn missing_name_or_phase_is_rejected() {
        let no_phase = "---\nname: x\ndescription: d\n---\nbody\n";
        assert!(parse_skill_frontmatter(no_phase).is_none());
        let no_name = "---\ndescription: d\nphase: [generation]\n---\nbody\n";
        assert!(parse_skill_frontmatter(no_name).is_none());
        let no_frontmatter = "just body text";
        assert!(parse_skill_frontmatter(no_frontmatter).is_none());
    }

    #[test]
    fn strip_comment_drops_unquoted_comments() {
        assert_eq!(strip_comment("form,  # trailing"), "form,  ");
        assert_eq!(strip_comment("  # whole-line comment"), "  ");
        // A `#` inside quotes is part of the value, not a comment.
        assert_eq!(strip_comment("name: 'a#b'"), "name: 'a#b'");
    }
}
