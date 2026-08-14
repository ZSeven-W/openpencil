//! Responsive size classes for the editor chrome.
//!
//! Replaces the one-time `mobile_layout: bool` decision with a live,
//! three-way layout class computed from the safe-area content rectangle.
//! Keyboard occlusion affects available surface height, not layout class. Layout
//! geometry lives in `op-editor-ui`.

/// The editor's responsive layout class.
///
/// Rules (handoff spec, based on usable width × height in logical
/// points):
///
/// - **Compact**: width `< 600` **or** height `< 500` — phones, phone
///   landscape, narrow split-window.
/// - **Medium**: width `600–959` — tablet portrait, large foldables,
///   medium split-window.
/// - **Expanded**: width `>= 960` **and** height `>= 600` — tablet
///   landscape and large tablets.
///
/// Height can force a downgrade: a wide-but-short viewport (e.g.
/// `1000 × 520`) lands in Medium, not Expanded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorSizeClass {
    Compact,
    Medium,
    Expanded,
}

impl EditorSizeClass {
    /// Phone and narrow split-view layout.
    pub fn is_compact(self) -> bool {
        matches!(self, EditorSizeClass::Compact)
    }

    /// Tablet portrait and medium split-view layout.
    pub fn is_medium(self) -> bool {
        matches!(self, EditorSizeClass::Medium)
    }

    /// Tablet landscape and large-window layout.
    pub fn is_expanded(self) -> bool {
        matches!(self, EditorSizeClass::Expanded)
    }

    /// True for phone-class layouts: one primary surface + one transient
    /// secondary surface (bottom sheets), rails overlay the canvas.
    ///
    /// Medium is deliberately excluded. A tablet portrait window needs
    /// tablet side surfaces and popovers, not a stretched phone sheet.
    pub fn is_sheet_layout(self) -> bool {
        self.is_compact()
    }

    /// True when a persistent side rail may push the canvas (tablet
    /// landscape and larger).
    pub fn is_rail_layout(self) -> bool {
        matches!(self, EditorSizeClass::Expanded)
    }
}

/// Resolve the size class from the usable content rectangle (logical
/// points). Pure so the engine and tests share one definition.
pub fn size_class(width: f32, height: f32) -> EditorSizeClass {
    if width < 600.0 || height < 500.0 {
        EditorSizeClass::Compact
    } else if width < 960.0 {
        EditorSizeClass::Medium
    } else if height < 600.0 {
        // Wide but short: downgrade Expanded to Medium.
        EditorSizeClass::Medium
    } else {
        EditorSizeClass::Expanded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_portrait_is_compact() {
        assert_eq!(size_class(375.0, 812.0), EditorSizeClass::Compact);
        assert_eq!(size_class(411.0, 838.0), EditorSizeClass::Compact);
        assert_eq!(size_class(320.0, 568.0), EditorSizeClass::Compact);
    }

    #[test]
    fn phone_landscape_downgrades_on_height() {
        // 844 wide but 390 tall → Compact (height forces the downgrade).
        assert_eq!(size_class(844.0, 390.0), EditorSizeClass::Compact);
        assert_eq!(size_class(667.0, 375.0), EditorSizeClass::Compact);
    }

    #[test]
    fn tablet_portrait_is_medium() {
        assert_eq!(size_class(768.0, 1024.0), EditorSizeClass::Medium);
        assert_eq!(size_class(744.0, 1133.0), EditorSizeClass::Medium);
        assert_eq!(size_class(600.0, 900.0), EditorSizeClass::Medium);
    }

    #[test]
    fn tablet_landscape_is_expanded() {
        assert_eq!(size_class(1024.0, 768.0), EditorSizeClass::Expanded);
        assert_eq!(size_class(1280.0, 800.0), EditorSizeClass::Expanded);
        assert_eq!(size_class(960.0, 600.0), EditorSizeClass::Expanded);
    }

    #[test]
    fn wide_but_short_downgrades_to_medium() {
        // Not Compact (height >= 500), not Expanded (height < 600).
        assert_eq!(size_class(1000.0, 520.0), EditorSizeClass::Medium);
        assert_eq!(size_class(960.0, 540.0), EditorSizeClass::Medium);
    }

    #[test]
    fn boundaries_are_exclusive() {
        assert_eq!(size_class(599.9, 800.0), EditorSizeClass::Compact);
        assert_eq!(size_class(600.0, 800.0), EditorSizeClass::Medium);
        assert_eq!(size_class(959.9, 700.0), EditorSizeClass::Medium);
        assert_eq!(size_class(960.0, 700.0), EditorSizeClass::Expanded);
        assert_eq!(size_class(700.0, 499.9), EditorSizeClass::Compact);
        assert_eq!(size_class(700.0, 500.0), EditorSizeClass::Medium);
    }

    #[test]
    fn sheet_vs_rail_layouts() {
        assert!(EditorSizeClass::Compact.is_sheet_layout());
        assert!(!EditorSizeClass::Medium.is_sheet_layout());
        assert!(!EditorSizeClass::Expanded.is_sheet_layout());
        assert!(EditorSizeClass::Expanded.is_rail_layout());
        assert!(!EditorSizeClass::Compact.is_rail_layout());
    }
}

/// The single transient touch surface. Compact uses bottom sheets while
/// Medium uses bounded side surfaces and popovers; opening one closes the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MobileSheetKind {
    #[default]
    Layers,
    Properties,
    Ai,
    More,
}
