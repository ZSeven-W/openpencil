//! The "Key aesthetics" bullets of a style guide's `## Style Summary`.
//!
//! Every shipped guide opens with a prose summary followed by a short list
//! of `- **Name**: sentence` bullets that name the display face, how the
//! accent is spent, card depth and the hero idiom. That list is the
//! guide's expression layer — the part a weak-model prompt lost when the
//! compact summary sent only palette, fonts and radii.

/// `- **Label**: text` bullets under `## Style Summary`, in order, at most
/// `limit`. Markdown emphasis is stripped so the lines read as plain rules.
pub fn key_aesthetics(content: &str, limit: usize) -> Vec<String> {
    let mut in_summary = false;
    let mut out = Vec::new();
    for raw in content.lines() {
        let line = raw.trim();
        if let Some(heading) = line.strip_prefix("## ") {
            in_summary = heading.trim().eq_ignore_ascii_case("Style Summary");
            continue;
        }
        if !in_summary {
            continue;
        }
        let Some(item) = line.strip_prefix("- ") else {
            continue;
        };
        let cleaned = item.replace("**", "").trim().to_string();
        if cleaned.is_empty() {
            continue;
        }
        out.push(cleaned);
        if out.len() >= limit {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::key_aesthetics;

    const GUIDE: &str = "\
---
name: 'x'
---

## Style Summary

A high-energy dark mobile interface.

Key aesthetics:

- **Electric lime on black**: #C4F82A as the sole accent
- **Bold geometric type**: Space Grotesk at 800 weight for hero metrics
- **Gradient cards**: dark gradient fills for depth

## Color System

- **Not an aesthetic**: palette rows live here
";

    #[test]
    fn reads_the_summary_bullets_and_stops_at_the_next_section() {
        let got = key_aesthetics(GUIDE, 5);
        assert_eq!(got.len(), 3);
        assert_eq!(got[0], "Electric lime on black: #C4F82A as the sole accent");
        assert!(got[2].starts_with("Gradient cards"));
    }

    #[test]
    fn honours_the_limit_and_tolerates_a_guide_without_a_summary() {
        assert_eq!(key_aesthetics(GUIDE, 2).len(), 2);
        assert!(key_aesthetics("## Color System\n- **Accent**: #fff\n", 5).is_empty());
    }

    #[test]
    fn every_shipped_mobile_guide_carries_key_aesthetics() {
        let registry = crate::style_guide::style_guide_registry();
        let mobile: Vec<_> = registry
            .iter()
            .filter(|g| g.platform == crate::style_guide::Platform::Mobile)
            .collect();
        assert!(!mobile.is_empty());
        for guide in mobile {
            assert!(
                !key_aesthetics(&guide.content, 5).is_empty(),
                "{} has no Key aesthetics bullets",
                guide.name
            );
        }
    }
}
