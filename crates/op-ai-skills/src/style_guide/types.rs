//! Style-guide types — port of `style-guide-types.ts`.

/// Target platform a style guide is written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Webapp,
    Mobile,
    LandingPage,
    Slides,
}

impl Platform {
    /// Wire token, matching the TS `platform` string union.
    pub fn as_str(self) -> &'static str {
        match self {
            Platform::Webapp => "webapp",
            Platform::Mobile => "mobile",
            Platform::LandingPage => "landing-page",
            Platform::Slides => "slides",
        }
    }

    /// Parse a platform token; unknown / missing → `Webapp` (TS parity).
    pub fn from_str(s: &str) -> Platform {
        match s.trim() {
            "mobile" => Platform::Mobile,
            "landing-page" => Platform::LandingPage,
            "slides" => Platform::Slides,
            _ => Platform::Webapp,
        }
    }
}

/// A style guide's frontmatter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleGuideMeta {
    pub name: String,
    pub tags: Vec<String>,
    pub platform: Platform,
}

/// A style guide's frontmatter plus its full markdown content (the
/// content is injected verbatim into generation prompts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedStyleGuide {
    pub name: String,
    pub tags: Vec<String>,
    pub platform: Platform,
    pub content: String,
}

/// The controlled vocabulary of style-guide tags (TS `STYLE_GUIDE_TAGS`).
pub const STYLE_GUIDE_TAGS: &[&str] = &[
    // visual
    "brutalist",
    "classical",
    "clean",
    "colorful",
    "editorial",
    "elegant",
    "geometric",
    "minimal",
    "organic",
    "playful",
    "scandinavian",
    "swiss",
    "bauhaus",
    "zen",
    // tone
    "calm",
    "dark-mode",
    "high-contrast",
    "light-mode",
    "monochrome",
    "neon",
    "pastel",
    "vibrant",
    "warm-tones",
    // industry
    "corporate",
    "creative",
    "data-focused",
    "developer",
    "education",
    "enterprise",
    "fintech",
    "luxury",
    "tech",
    "terminal",
    "wellness",
    // typography
    "bold-typography",
    "condensed",
    "display",
    "dual-font",
    "magazine",
    "monospace",
    "serif",
    "uppercase",
    // layout
    "bento-grid",
    "floating-nav",
    "flush-layout",
    "horizontal-nav",
    "icon-sidebar",
    "sidebar",
    // mood
    "austere",
    "confident",
    "cozy",
    "crisp",
    "electric",
    "friendly",
    "quiet",
    "refined",
    "sophisticated",
    "urban",
    // accent
    "blue-accent",
    "cyan-accent",
    "gold-accent",
    "lime-accent",
    "orange-accent",
    "red-accent",
    "sage-green",
    // technique
    "flat",
    "gradient",
    "mesh-gradient",
    "rounded",
    "sharp-corners",
    "soft-corners",
    "soft-shadows",
    "stroke-based",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_round_trips_with_webapp_default() {
        for p in [
            Platform::Webapp,
            Platform::Mobile,
            Platform::LandingPage,
            Platform::Slides,
        ] {
            assert_eq!(Platform::from_str(p.as_str()), p);
        }
        assert_eq!(Platform::from_str("???"), Platform::Webapp);
    }

    #[test]
    fn tag_vocabulary_is_populated() {
        assert!(STYLE_GUIDE_TAGS.contains(&"brutalist"));
        assert!(STYLE_GUIDE_TAGS.contains(&"dark-mode"));
        assert!(STYLE_GUIDE_TAGS.len() > 50);
    }
}
