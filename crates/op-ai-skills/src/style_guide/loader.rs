//! Style-guide loading + selection — port of `style-guide-loader.ts`.

use std::sync::OnceLock;

use crate::frontmatter::{parse_array, split_frontmatter, unquote};
use crate::style_guide::types::{ParsedStyleGuide, Platform, StyleGuideMeta};
use crate::SKILLS;

/// Parse a style-guide markdown file's frontmatter.
fn parse_frontmatter(content: &str) -> Option<(StyleGuideMeta, String)> {
    let (fm, body) = split_frontmatter(content)?;
    let mut name = String::new();
    let mut tags: Vec<String> = Vec::new();
    let mut platform = Platform::Webapp;
    for line in fm.lines() {
        let line = line.trim();
        let Some(colon) = line.find(':') else {
            continue;
        };
        let key = line[..colon].trim();
        let value = line[colon + 1..].trim();
        match key {
            "name" => name = unquote(value),
            "tags" => {
                if value.starts_with('[') {
                    tags = parse_array(value);
                }
            }
            "platform" => platform = Platform::from_str(&unquote(value)),
            _ => {}
        }
    }
    Some((
        StyleGuideMeta {
            name,
            tags,
            platform,
        },
        body,
    ))
}

/// Parse a raw markdown string into a [`ParsedStyleGuide`]. The full
/// raw markdown is kept as `content` (TS parity — it is injected
/// verbatim into prompts).
pub fn parse_style_guide_file(raw: &str) -> Option<ParsedStyleGuide> {
    let (meta, _body) = parse_frontmatter(raw)?;
    Some(ParsedStyleGuide {
        name: meta.name,
        tags: meta.tags,
        platform: meta.platform,
        content: raw.to_string(),
    })
}

static REGISTRY: OnceLock<Vec<ParsedStyleGuide>> = OnceLock::new();

/// The embedded style-guide registry, parsed + cached on first call.
pub fn style_guide_registry() -> &'static [ParsedStyleGuide] {
    REGISTRY.get_or_init(|| {
        let mut out = Vec::new();
        if let Some(dir) = SKILLS.get_dir("style-guides") {
            for file in dir.files() {
                let is_md = file.path().extension().map(|e| e == "md").unwrap_or(false);
                if !is_md {
                    continue;
                }
                if let Some(text) = file.contents_utf8() {
                    if let Some(guide) = parse_style_guide_file(text) {
                        out.push(guide);
                    }
                }
            }
        }
        out
    })
}

/// Jaccard-like score: fraction of requested tags the guide carries.
fn match_score(guide_tags: &[String], request_tags: &[String]) -> f64 {
    if request_tags.is_empty() {
        return 0.0;
    }
    let hits = request_tags
        .iter()
        .filter(|t| guide_tags.contains(t))
        .count();
    hits as f64 / request_tags.len() as f64
}

/// Selection criteria for [`select_style_guide`].
#[derive(Debug, Clone, Default)]
pub struct SelectOptions {
    pub tags: Vec<String>,
    pub name: Option<String>,
    pub platform: Option<Platform>,
}

/// Select the best-matching style guide. Name takes precedence (with
/// an optional platform constraint); otherwise the highest tag score
/// among the platform-filtered candidates wins. Returns `None` when
/// nothing matches — the caller falls back to `style-defaults.md`.
pub fn select_style_guide<'a>(
    guides: &'a [ParsedStyleGuide],
    options: &SelectOptions,
) -> Option<&'a ParsedStyleGuide> {
    if let Some(name) = &options.name {
        return match options.platform {
            Some(platform) => guides
                .iter()
                .find(|g| &g.name == name && g.platform == platform),
            None => guides.iter().find(|g| &g.name == name),
        };
    }

    // Platform filter — but keep all guides if the filter is empty.
    let platform_filtered: Vec<&ParsedStyleGuide> = match options.platform {
        Some(platform) => guides.iter().filter(|g| g.platform == platform).collect(),
        None => guides.iter().collect(),
    };
    let candidates: Vec<&ParsedStyleGuide> = if platform_filtered.is_empty() {
        guides.iter().collect()
    } else {
        platform_filtered
    };

    if options.tags.is_empty() {
        return None;
    }

    // Highest tag score wins; on a tie the earliest candidate (TS
    // sorts descending + keeps the first equal-score entry, so a
    // strict `>` comparison reproduces that order).
    let mut best: Option<(&ParsedStyleGuide, f64)> = None;
    for guide in candidates {
        let score = match_score(&guide.tags, &options.tags);
        if score <= 0.0 {
            continue;
        }
        if best.map(|(_, b)| score > b).unwrap_or(true) {
            best = Some((guide, score));
        }
    }
    best.map(|(g, _)| g)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guide(name: &str, tags: &[&str], platform: Platform) -> ParsedStyleGuide {
        ParsedStyleGuide {
            name: name.into(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            platform,
            content: format!("---\nname: {name}\n---\nbody"),
        }
    }

    #[test]
    fn registry_loads_the_style_guides() {
        let reg = style_guide_registry();
        // ~50 style guides ship with the corpus.
        assert!(
            reg.len() >= 40,
            "style-guide registry too small: {}",
            reg.len()
        );
        assert!(reg.iter().all(|g| !g.name.is_empty()));
    }

    #[test]
    fn parse_reads_frontmatter() {
        let raw = "---\nname: 'ai-product-dark'\ntags: [tech, dark-mode, gradient]\nplatform: webapp\n---\n## Style Summary\nbody";
        let g = parse_style_guide_file(raw).expect("parses");
        assert_eq!(g.name, "ai-product-dark");
        assert_eq!(g.tags, vec!["tech", "dark-mode", "gradient"]);
        assert_eq!(g.platform, Platform::Webapp);
        // Content keeps the full raw markdown.
        assert!(g.content.contains("## Style Summary"));
    }

    #[test]
    fn select_by_name() {
        let guides = vec![
            guide("a", &["dark-mode"], Platform::Webapp),
            guide("b", &["light-mode"], Platform::Mobile),
        ];
        let opts = SelectOptions {
            name: Some("b".into()),
            ..Default::default()
        };
        assert_eq!(select_style_guide(&guides, &opts).unwrap().name, "b");
        // Name + mismatching platform → no match.
        let opts2 = SelectOptions {
            name: Some("b".into()),
            platform: Some(Platform::Webapp),
            ..Default::default()
        };
        assert!(select_style_guide(&guides, &opts2).is_none());
    }

    #[test]
    fn select_by_tag_score() {
        let guides = vec![
            guide("low", &["dark-mode"], Platform::Webapp),
            guide("high", &["dark-mode", "gradient", "tech"], Platform::Webapp),
        ];
        let opts = SelectOptions {
            tags: vec!["dark-mode".into(), "gradient".into(), "tech".into()],
            ..Default::default()
        };
        // "high" matches 3/3, "low" only 1/3.
        assert_eq!(select_style_guide(&guides, &opts).unwrap().name, "high");
    }

    #[test]
    fn tag_score_tie_keeps_first_candidate() {
        let guides = vec![
            guide("first", &["dark-mode"], Platform::Webapp),
            guide("second", &["dark-mode"], Platform::Webapp),
        ];
        let opts = SelectOptions {
            tags: vec!["dark-mode".into()],
            ..Default::default()
        };
        // Both score 1.0 — the earlier registry entry wins (TS parity).
        assert_eq!(select_style_guide(&guides, &opts).unwrap().name, "first");
    }

    #[test]
    fn select_returns_none_without_tags_or_name() {
        let guides = vec![guide("a", &["dark-mode"], Platform::Webapp)];
        assert!(select_style_guide(&guides, &SelectOptions::default()).is_none());
    }
}
