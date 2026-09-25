//! User-language topics for the quality report, and the one table that
//! translates internal pass names and lint categories into them.
//!
//! The deterministic repair layer names its work after implementation
//! details (`geometry-validation`, `chip-text-contrast`,
//! `spacing+footer-sink`). A user reading the report needs the *kind of
//! problem* instead — "layout overflow", "text contrast". Every mapping
//! lives here so the two vocabularies meet in exactly one place.

use serde::{Deserialize, Serialize};

/// A family of problems, as named to the user. Declaration order is
/// display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum QualityTopic {
    /// Content spilling out of its container, text that does not fit.
    Overflow,
    /// Text too close in colour to what sits behind it.
    Contrast,
    /// Padding, margins, gaps and breathing room.
    Spacing,
    /// Sizes, centring and alignment of containers.
    Sizing,
    /// Text hierarchy and text styling.
    Hierarchy,
    /// Page structure: duplicated chrome, broken shells, empty stubs.
    Structure,
    /// Siblings that should match but drifted apart.
    Consistency,
    /// Theme and colour-variable problems.
    Palette,
    /// Image slots and placeholders.
    Images,
    /// Motion and shader effects.
    Effects,
    /// Chart geometry (bars without a baseline, top-anchored bars).
    Charts,
    /// Inputs and controls without an accessible label.
    Accessibility,
    /// Screens the plan promised that never received content.
    Completeness,
    /// Fixes the vision-model review applied (count only).
    VisualReview,
}

impl QualityTopic {
    /// Every topic, in display order.
    pub const ALL: [QualityTopic; 14] = [
        QualityTopic::Overflow,
        QualityTopic::Contrast,
        QualityTopic::Spacing,
        QualityTopic::Sizing,
        QualityTopic::Hierarchy,
        QualityTopic::Structure,
        QualityTopic::Consistency,
        QualityTopic::Palette,
        QualityTopic::Images,
        QualityTopic::Effects,
        QualityTopic::Charts,
        QualityTopic::Accessibility,
        QualityTopic::Completeness,
        QualityTopic::VisualReview,
    ];

    /// The i18n key of the topic's user-facing label.
    pub fn label_key(self) -> &'static str {
        match self {
            QualityTopic::Overflow => "workspace.quality.topic.overflow",
            QualityTopic::Contrast => "workspace.quality.topic.contrast",
            QualityTopic::Spacing => "workspace.quality.topic.spacing",
            QualityTopic::Sizing => "workspace.quality.topic.sizing",
            QualityTopic::Hierarchy => "workspace.quality.topic.hierarchy",
            QualityTopic::Structure => "workspace.quality.topic.structure",
            QualityTopic::Consistency => "workspace.quality.topic.consistency",
            QualityTopic::Palette => "workspace.quality.topic.palette",
            QualityTopic::Images => "workspace.quality.topic.images",
            QualityTopic::Effects => "workspace.quality.topic.effects",
            QualityTopic::Charts => "workspace.quality.topic.charts",
            QualityTopic::Accessibility => "workspace.quality.topic.accessibility",
            QualityTopic::Completeness => "workspace.quality.topic.completeness",
            QualityTopic::VisualReview => "workspace.quality.topic.visualReview",
        }
    }

    /// Topics whose detectors are proven to have run when the orchestrator
    /// reports the check family `family` (`RepairSummary` wire keys). A
    /// family that covers several topics vouches for all of them because
    /// each of those topics' passes runs inside that family's checkpoint.
    pub fn covered_by_family(family: &str) -> &'static [QualityTopic] {
        match family {
            "layout" => &[QualityTopic::Spacing, QualityTopic::Sizing],
            "overflow" => &[QualityTopic::Overflow],
            "hierarchy" => &[QualityTopic::Hierarchy],
            "structure" => &[
                QualityTopic::Structure,
                QualityTopic::Consistency,
                QualityTopic::Images,
            ],
            "palette" => &[QualityTopic::Palette],
            _ => &[],
        }
    }
}

/// Pass (or pass-group) name → topic. Exact names, after stripping a
/// `loop-finalize:` prefix. Anything missing falls back to its check
/// family through [`topic_for_family`].
const PASS_TOPICS: &[(&str, QualityTopic)] = &[
    ("geometry-validation", QualityTopic::Overflow),
    ("geometry_validate_and_fix", QualityTopic::Overflow),
    ("text-fit", QualityTopic::Overflow),
    ("board-text-wrap", QualityTopic::Overflow),
    ("none-stack-inset", QualityTopic::Overflow),
    ("absolute-child-clamp", QualityTopic::Overflow),
    ("absolute-child-shrink", QualityTopic::Overflow),
    ("mobile-trailing-nav-reflow", QualityTopic::Overflow),
    ("text_contrast_repair", QualityTopic::Contrast),
    ("chip-text-contrast", QualityTopic::Contrast),
    ("slide-padding-floor", QualityTopic::Spacing),
    ("unify-section-margins", QualityTopic::Spacing),
    ("spacing+footer-sink", QualityTopic::Spacing),
    ("card-inner-padding", QualityTopic::Spacing),
    ("mobile-bottom-breathing", QualityTopic::Spacing),
    ("table-gap", QualityTopic::Spacing),
    ("container-geometry", QualityTopic::Sizing),
    ("card-board-centre", QualityTopic::Sizing),
    ("radial+root-height", QualityTopic::Sizing),
    ("touch-target-floor", QualityTopic::Sizing),
    ("mobile-chrome+content-rail", QualityTopic::Sizing),
    ("text-hierarchy+strokes", QualityTopic::Hierarchy),
    ("duplicate-root-dedupe", QualityTopic::Structure),
    // Website import: repeated parts turned into one component + instances,
    // and brand colours bound to the kit's variables.
    ("site-import:components", QualityTopic::Consistency),
    ("site-import:brand-binding", QualityTopic::Palette),
    ("app-shell+table-regroup", QualityTopic::Structure),
    ("chip+ring-extract", QualityTopic::Structure),
    ("hero-bleed", QualityTopic::Structure),
    ("category-grid-density", QualityTopic::Structure),
    ("empty-content-bar", QualityTopic::Structure),
    ("radial+stub+shell", QualityTopic::Structure),
    ("chrome-dedupe", QualityTopic::Structure),
    ("avatar+nav-anchor", QualityTopic::Structure),
    ("shared-chrome+nav", QualityTopic::Structure),
    ("chrome-dedupe+avatar+nav-anchor", QualityTopic::Structure),
    ("equalize-sibling-items", QualityTopic::Consistency),
    ("sibling-style-drift", QualityTopic::Consistency),
    ("theme-variable-polarity", QualityTopic::Palette),
    ("light-mobile-nav-surface", QualityTopic::Palette),
    ("materialize-image-slots", QualityTopic::Images),
    ("map-placeholder", QualityTopic::Images),
    ("image-fallback-policy", QualityTopic::Images),
    ("motion-recipes", QualityTopic::Effects),
];

/// Design-lint category (its kebab-case wire key) → topic. Categories
/// absent from this table are deliberately NOT reported to the user:
/// code-shape findings (redundant wrappers, nesting depth, absolute
/// share, empty paths) are invisible on the canvas, and the "slop"
/// rules are taste opinions, not facts.
pub(super) const LINT_TOPICS: &[(&str, QualityTopic)] = &[
    ("text-bg-contrast", QualityTopic::Contrast),
    ("edge-section-padding", QualityTopic::Spacing),
    ("stacked-horizontal-padding", QualityTopic::Spacing),
    ("mixed-sibling-padding", QualityTopic::Spacing),
    ("text-explicit-height", QualityTopic::Overflow),
    ("unexpected-rotation", QualityTopic::Sizing),
    ("text-effect", QualityTopic::Hierarchy),
    ("text-stroke", QualityTopic::Hierarchy),
    ("text-corner-radius", QualityTopic::Hierarchy),
    ("empty-filled-panel", QualityTopic::Structure),
    ("invisible-container", QualityTopic::Structure),
    ("sibling-inconsistency", QualityTopic::Consistency),
    ("mixed-sibling-corner-radius", QualityTopic::Consistency),
    ("excessive-frame-effects", QualityTopic::Consistency),
    ("motion-budget", QualityTopic::Effects),
    ("shader-budget", QualityTopic::Effects),
    ("shader-invalid", QualityTopic::Effects),
    ("top-anchored-bars", QualityTopic::Charts),
    ("no-baseline-bars", QualityTopic::Charts),
    ("widget-a11y", QualityTopic::Accessibility),
];

/// The topic a check family's unlisted passes fall back to.
pub fn topic_for_family(family: &str) -> QualityTopic {
    match family {
        "overflow" => QualityTopic::Overflow,
        "hierarchy" => QualityTopic::Hierarchy,
        "structure" => QualityTopic::Structure,
        "palette" => QualityTopic::Palette,
        _ => QualityTopic::Sizing,
    }
}

/// Translate a repair pass (group) name into its topic. `family` is the
/// check family the orchestrator credited the edit to, used when the
/// pass is not in the table (a new pass must still land somewhere true).
pub fn topic_for_pass(pass: &str, family: &str) -> QualityTopic {
    let bare = pass.strip_prefix("loop-finalize:").unwrap_or(pass);
    PASS_TOPICS
        .iter()
        .find(|(name, _)| *name == bare)
        .map(|(_, topic)| *topic)
        .unwrap_or_else(|| topic_for_family(family))
}

/// Translate a design-lint category key into its topic, or `None` when
/// the category is not one the user is shown.
pub fn topic_for_lint(category: &str) -> Option<QualityTopic> {
    LINT_TOPICS
        .iter()
        .find(|(name, _)| *name == category)
        .map(|(_, topic)| *topic)
}

/// Every lint topic the final audit covers — the topics a completed
/// audit may truthfully mark as checked.
pub fn lint_topics() -> Vec<QualityTopic> {
    let mut topics: Vec<QualityTopic> = LINT_TOPICS.iter().map(|(_, topic)| *topic).collect();
    topics.sort();
    topics.dedup();
    topics
}
