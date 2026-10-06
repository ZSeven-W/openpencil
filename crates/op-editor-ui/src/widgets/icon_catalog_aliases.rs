//! Legacy lucide icon names → their current names.
//!
//! Lucide renamed whole families over 2024 (`*-circle` → `circle-*`,
//! `bar-chart*` → `chart-*`, `more-*` → `ellipsis*`, `edit*` → `*pen*`),
//! but the old names are what models learned and keep emitting. The
//! catalog only carries current names, so without this table those glyphs
//! render as nothing. Every target here must exist in the core catalog —
//! the test below checks that against the shipped JSON.

/// Current lucide name for a legacy one, if it was renamed.
pub fn lucide_current_name(legacy: &str) -> Option<&'static str> {
    LUCIDE_RENAMES
        .iter()
        .find(|(old, _)| *old == legacy)
        .map(|(_, current)| *current)
}

const LUCIDE_RENAMES: &[(&str, &str)] = &[
    ("more-horizontal", "ellipsis"),
    ("more-vertical", "ellipsis-vertical"),
    ("check-circle", "circle-check"),
    ("check-circle-2", "circle-check-big"),
    ("x-circle", "circle-x"),
    ("alert-circle", "circle-alert"),
    ("alert-triangle", "triangle-alert"),
    ("alert-octagon", "octagon-alert"),
    ("help-circle", "circle-question-mark"),
    ("plus-circle", "circle-plus"),
    ("minus-circle", "circle-minus"),
    ("play-circle", "circle-play"),
    ("pause-circle", "circle-pause"),
    ("stop-circle", "circle-stop"),
    ("arrow-up-circle", "circle-arrow-up"),
    ("arrow-down-circle", "circle-arrow-down"),
    ("arrow-left-circle", "circle-arrow-left"),
    ("arrow-right-circle", "circle-arrow-right"),
    ("chevron-down-circle", "circle-chevron-down"),
    ("chevron-up-circle", "circle-chevron-up"),
    ("chevron-left-circle", "circle-chevron-left"),
    ("chevron-right-circle", "circle-chevron-right"),
    ("user-circle", "circle-user"),
    ("user-circle-2", "circle-user-round"),
    ("edit", "square-pen"),
    ("edit-2", "pen"),
    ("edit-3", "pen-line"),
    ("sliders", "sliders-vertical"),
    ("grid", "grid-3x3"),
    ("layout", "panels-top-left"),
    ("sidebar", "panel-left"),
    ("sidebar-open", "panel-left-open"),
    ("sidebar-close", "panel-left-close"),
    ("home", "house"),
    ("verified", "badge-check"),
    ("wand-2", "wand-sparkles"),
    ("loader-2", "loader-circle"),
    ("bar-chart", "chart-no-axes-column"),
    ("bar-chart-2", "chart-no-axes-column-increasing"),
    ("bar-chart-3", "chart-column"),
    ("bar-chart-4", "chart-column-increasing"),
    ("bar-chart-big", "chart-column-big"),
    ("bar-chart-horizontal", "chart-bar"),
    ("bar-chart-horizontal-big", "chart-bar-big"),
    ("line-chart", "chart-line"),
    ("pie-chart", "chart-pie"),
    ("area-chart", "chart-area"),
    ("scatter-chart", "chart-scatter"),
    ("candlestick-chart", "chart-candlestick"),
    ("gantt-chart", "chart-gantt"),
    ("form-input", "rectangle-ellipsis"),
    ("mic-2", "mic-vocal"),
    ("shield-close", "shield-x"),
    ("sort-asc", "arrow-up-narrow-wide"),
    ("sort-desc", "arrow-down-wide-narrow"),
];

#[cfg(test)]
mod tests {
    use super::{lucide_current_name, LUCIDE_RENAMES};

    #[test]
    fn every_rename_target_exists_in_the_shipped_catalog() {
        let missing: Vec<&str> = LUCIDE_RENAMES
            .iter()
            .filter(|(_, current)| {
                super::super::icon_catalog::lookup_icon("lucide", current).is_none()
            })
            .map(|(old, _)| *old)
            .collect();
        assert!(
            missing.is_empty(),
            "rename targets absent from catalog: {missing:?}"
        );
    }

    #[test]
    fn legacy_names_resolve_through_the_catalog_lookup() {
        for legacy in [
            "more-horizontal",
            "check-circle",
            "grid",
            "edit",
            "sliders",
            "bar-chart-2",
        ] {
            assert!(
                lucide_current_name(legacy).is_some(),
                "{legacy} needs an alias"
            );
            assert!(
                super::super::icon_catalog::lookup_icon("lucide", legacy).is_some(),
                "{legacy} must paint"
            );
        }
        // A feather-only name with no rename still paints through the fallback.
        assert!(super::super::icon_catalog::lookup_icon("lucide", "grid").is_some());
        // Garbage stays unpainted.
        assert!(super::super::icon_catalog::lookup_icon("lucide", "no-such-glyph-xyz").is_none());
    }
}
