//! Left-rail tab selection + the slides tab's pointer bookkeeping.
//!
//! The left rail is a three-tab surface: the Agent conversation
//! (`Chat`), the ordinary Pages + Layers tree, and a page navigator
//! that lists one thumbnail per top-level board. Which tab is showing,
//! which row the cursor is on, and the reorder gesture in flight are
//! all transient UI state and live here. The Chat tab is desktop-only
//! (touch chrome hosts the conversation as its own bottom sheet), and
//! the Slides tab only exists for documents that have boards to list.
//!
//! The slide ORDER is deliberately absent: it is the document's own
//! top-level child order (`crate::preview_slideshow::active_page_boards`)
//! and a copy here would be a second answer to "what order are the
//! slides in".

use jian_core::scroll::ScrollState;

use super::EditorUiState;

/// Width the rail is raised to the FIRST time the Chat tab is shown in
/// a session: the chat's transcript needs more column than the Layers
/// tree's 240 px default. Afterwards the user's own drag wins and is
/// never overridden.
pub const CHAT_TAB_MIN_WIDTH: f32 = 320.0;
/// Drag range for `layer_panel_width` — the rail is the chat's column
/// now, so it may run wider than the old Layers-only limits.
pub const LAYER_PANEL_MIN_WIDTH: f32 = 240.0;
pub const LAYER_PANEL_MAX_WIDTH: f32 = 440.0;

/// Which tab the left rail is showing.
///
/// `Layers` is the default, so a document that nobody interacted with
/// opens exactly as it did before the tab row existed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LeftPanelTab {
    #[default]
    Layers,
    /// The Agent conversation, pinned into the rail's body.
    Chat,
    Slides,
}

/// A slides-list reorder gesture in flight.
///
/// `press_y` is kept alongside the live `pointer_y` so the release can
/// tell a click (jump to the slide) from a drag (reorder the deck) —
/// the two are the same gesture until the pointer has travelled far
/// enough, and deciding on release is what lets a shaky click still
/// navigate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlidesDrag {
    /// Index of the slide being dragged, in page order.
    pub from: usize,
    /// Where the press landed, in screen px.
    pub press_y: f32,
    /// Where the cursor is now, in screen px.
    pub pointer_y: f32,
}

/// What a press on the slides panel landed on. Hover and press both
/// carry one of these so the two never disagree about what is under
/// the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlidesPanelTarget {
    /// The "Chat" tab in the tab row.
    ChatTab,
    /// The "Layers" tab in the tab row.
    LayersTab,
    /// The "Slides" / "Cards" tab in the tab row.
    SlidesTab,
    /// A slide row, by index in page order.
    Slide(usize),
    /// The action bar's present button.
    Present,
    /// The action bar's `Export PDF ⌄` button — toggles the menu below.
    ExportMenu,
    /// Export-menu row: every slide on the page.
    ExportAllSlides,
    /// Export-menu row: only the slides the selection covers. Only ever
    /// produced while that row is ENABLED, so a disabled row can neither
    /// be hovered nor activated.
    ExportSelectedSlides,
}

/// Left-rail tab selection + the slides tab's transient pointer state.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SlidesPanelState {
    /// Which tab the rail is showing.
    pub tab: LeftPanelTab,
    /// What the cursor is over — drives every hover wash in the panel.
    pub hover: Option<SlidesPanelTarget>,
    /// What a press landed on. Activates on RELEASE while the cursor is
    /// still on it, the same contract the presenting toolbar uses.
    pub pressed: Option<SlidesPanelTarget>,
    /// The reorder gesture in flight, once the pointer has moved.
    pub drag: Option<SlidesDrag>,
    /// Whether the action bar's export dropdown is showing.
    pub export_menu_open: bool,
    /// Vertical scroll of the slide list.
    pub scroll: ScrollState,
}

impl SlidesPanelState {
    /// Forget every in-flight pointer interaction, keeping the selected
    /// tab and the scroll position. Returns whether anything was live —
    /// hosts use that as the repaint signal.
    ///
    /// Called whenever the panel stops being reachable (the sidebar
    /// closes, a presentation starts), so a gesture cannot survive the
    /// surface it belongs to. The export dropdown goes with it for the
    /// same reason: it is painted INTO the panel, so a rail that stops
    /// being drawn would otherwise strand an open menu that reappears —
    /// still open — the next time the tab is shown.
    pub fn clear_pointer(&mut self) -> bool {
        let live = self.hover.is_some() || self.pressed.is_some() || self.drag.is_some();
        self.hover = None;
        self.pressed = None;
        self.drag = None;
        self.close_export_menu() || live
    }

    /// Close the export dropdown. Returns whether it was open, which is
    /// what makes this usable as an Escape-ladder rung: exactly one
    /// layer is dismissed per press.
    pub fn close_export_menu(&mut self) -> bool {
        let was_open = self.export_menu_open;
        self.export_menu_open = false;
        // A hover left on a menu row belongs to a menu that no longer
        // exists; it would light the row up again the moment the menu
        // reopened, before the cursor had moved.
        if was_open
            && matches!(
                self.hover,
                Some(SlidesPanelTarget::ExportAllSlides | SlidesPanelTarget::ExportSelectedSlides)
            )
        {
            self.hover = None;
        }
        was_open
    }
}

impl EditorUiState {
    /// Show the Agent conversation in the left rail: select the Chat
    /// tab and apply the one-time width bump (see
    /// [`CHAT_TAB_MIN_WIDTH`]). Returns whether anything changed.
    ///
    /// The bump is armed the FIRST time the tab becomes active in a
    /// session, whether or not the width was below the bar — afterwards
    /// the user's own drag is the only thing that moves the width.
    /// Touch chrome never takes the tab: its chat is a bottom sheet,
    /// and a Chat tab with no body would blank the rail.
    pub fn enter_chat_tab(&mut self) -> bool {
        if self.touch_chrome() {
            return false;
        }
        let mut changed = false;
        // The rail IS the destination, so opening it is part of going
        // there: a shut rail would swallow the send the user just made.
        if !self.sidebar_open {
            self.sidebar_open = true;
            changed = true;
        }
        if self.slides_panel.tab != LeftPanelTab::Chat {
            self.slides_panel.tab = LeftPanelTab::Chat;
            self.slides_panel.clear_pointer();
            changed = true;
        }
        if !self.chat_tab_width_bumped {
            self.chat_tab_width_bumped = true;
            if self.layer_panel_width < CHAT_TAB_MIN_WIDTH {
                self.layer_panel_width = CHAT_TAB_MIN_WIDTH;
                changed = true;
            }
        }
        changed
    }

    /// Clamp a dragged rail width into the shared 240–440 range — the
    /// one clamp every drag path (the professional gutter, the
    /// workspace dock handle) writes through, so the two columns that
    /// are now one panel can never disagree about the limits.
    pub fn set_layer_panel_width(&mut self, width: f32) {
        self.layer_panel_width = width.clamp(LAYER_PANEL_MIN_WIDTH, LAYER_PANEL_MAX_WIDTH);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layers_tab_is_the_default() {
        assert_eq!(SlidesPanelState::default().tab, LeftPanelTab::Layers);
    }

    #[test]
    fn clearing_pointer_state_keeps_the_tab_and_scroll() {
        let mut state = SlidesPanelState {
            tab: LeftPanelTab::Slides,
            hover: Some(SlidesPanelTarget::Slide(2)),
            pressed: Some(SlidesPanelTarget::Slide(2)),
            drag: Some(SlidesDrag {
                from: 2,
                press_y: 10.0,
                pointer_y: 40.0,
            }),
            export_menu_open: false,
            scroll: ScrollState { offset: 24.0 },
        };
        assert!(state.clear_pointer());
        assert_eq!(state.tab, LeftPanelTab::Slides);
        assert_eq!(state.scroll.offset, 24.0);
        assert_eq!(state.hover, None);
        assert_eq!(state.pressed, None);
        assert_eq!(state.drag, None);
        // Idle again — nothing to repaint for.
        assert!(!state.clear_pointer());
    }

    #[test]
    fn clearing_pointer_state_closes_the_export_menu() {
        let mut state = SlidesPanelState {
            tab: LeftPanelTab::Slides,
            export_menu_open: true,
            ..SlidesPanelState::default()
        };
        // Live even though no hover / press / drag is in flight: the open
        // menu is itself something the next paint has to stop drawing.
        assert!(state.clear_pointer());
        assert!(!state.export_menu_open);
    }

    #[test]
    fn closing_the_export_menu_drops_a_hover_on_one_of_its_rows() {
        let mut state = SlidesPanelState {
            export_menu_open: true,
            hover: Some(SlidesPanelTarget::ExportSelectedSlides),
            ..SlidesPanelState::default()
        };
        assert!(state.close_export_menu());
        assert_eq!(state.hover, None);
        // Already closed — nothing left to dismiss, so the Escape ladder
        // moves on to the next rung.
        assert!(!state.close_export_menu());
    }

    #[test]
    fn closing_the_export_menu_keeps_a_hover_outside_it() {
        let mut state = SlidesPanelState {
            export_menu_open: true,
            hover: Some(SlidesPanelTarget::Present),
            ..SlidesPanelState::default()
        };
        assert!(state.close_export_menu());
        assert_eq!(state.hover, Some(SlidesPanelTarget::Present));
    }

    #[test]
    fn the_chat_tab_bumps_a_narrow_rail_once_and_never_again() {
        let mut ui = EditorUiState::default();
        assert_eq!(ui.layer_panel_width, 240.0);
        assert!(ui.enter_chat_tab());
        assert_eq!(ui.slides_panel.tab, LeftPanelTab::Chat);
        assert_eq!(ui.layer_panel_width, CHAT_TAB_MIN_WIDTH);

        // The user drags the rail narrower; a later visit to the Chat
        // tab must not override their choice.
        ui.set_layer_panel_width(300.0);
        assert!(!ui.enter_chat_tab(), "already showing, already bumped");
        assert_eq!(ui.layer_panel_width, 300.0);
    }

    #[test]
    fn a_rail_already_wide_enough_consumes_its_one_bump_without_changing() {
        let mut ui = EditorUiState {
            layer_panel_width: 404.0,
            ..EditorUiState::default()
        };
        assert!(ui.enter_chat_tab());
        assert_eq!(ui.layer_panel_width, 404.0, "nothing to raise");
        // ...and the bump is spent: a later narrow rail stays narrow.
        ui.layer_panel_width = 260.0;
        ui.slides_panel.tab = LeftPanelTab::Layers;
        assert!(ui.enter_chat_tab());
        assert_eq!(ui.layer_panel_width, 260.0);
    }

    #[test]
    fn a_dragged_rail_width_clamps_into_the_shared_range() {
        let mut ui = EditorUiState::default();
        ui.set_layer_panel_width(120.0);
        assert_eq!(ui.layer_panel_width, LAYER_PANEL_MIN_WIDTH);
        ui.set_layer_panel_width(9_999.0);
        assert_eq!(ui.layer_panel_width, LAYER_PANEL_MAX_WIDTH);
        ui.set_layer_panel_width(360.0);
        assert_eq!(ui.layer_panel_width, 360.0);
    }
}
