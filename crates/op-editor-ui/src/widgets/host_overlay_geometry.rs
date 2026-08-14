//! On-screen rects for the floating overlays — dropdowns anchored to
//! the TopBar / toolbar, and the three draggable floating panels — plus
//! the StatusBar's two zoom actions.
//!
//! Both hosts carried byte-identical copies in
//! `widget_host/overlay_rects.rs` (native) and
//! `widget_host/{overlay_rects,geometry}.rs` (web). Every entry here is
//! pure over `EditorState` (plus the layout scene, for zoom-to-fit), so
//! it lives here; the hosts keep only the `mark_dirty` / scene-refresh
//! glue and the genuinely platform-shaped rects (the native Git panel,
//! the File menu's traffic-light-aware anchor).
//!
//! Paint and event-time hit-testing MUST read the same answer from these
//! — a dropdown whose painted rect and hit rect disagree eats clicks on
//! its own edge.

use op_editor_core::EditorState;

use crate::layout_scene::LayoutScene;
use crate::widgets::host_canvas_geometry::{
    canvas_rect, canvas_region, TOOLBAR_INSET_X, TOOLBAR_INSET_Y,
};
use crate::widgets::{
    ExportQuickMenu, ImportMenu, LayoutCx, LocalePicker, ShapePicker, Toolbar, TopBar, Widget,
    COMPONENT_BROWSER_PANEL_H, COMPONENT_BROWSER_PANEL_W, DESIGN_MD_PANEL_H, DESIGN_MD_PANEL_W,
    ICON_PICKER_PANEL_H, ICON_PICKER_PANEL_W, IMPORT_MENU_WIDTH, LOCALE_PICKER_WIDTH,
    PROMPT_CENTER_MIN_H, PROMPT_CENTER_MIN_W, PROMPT_CENTER_VIEWPORT_H_RATIO,
    PROMPT_CENTER_VIEWPORT_W_RATIO, SCENE_TEMPLATE_GALLERY_INSET, SHAPE_PICKER_WIDTH,
    TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
};
use crate::{Point2D, Rect};

/// The full-width TopBar rect — the anchor frame every TopBar-hosted
/// dropdown measures its button from.
pub fn top_bar_rect(viewport_w: f32) -> Rect {
    Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(viewport_w, TOP_BAR_HEIGHT),
    }
}

/// Step the canvas zoom from a StatusBar `[-]` / `[+]` click, anchored
/// at the canvas-region centre so the visible content scales in place.
/// `zoom_in` picks the sign; the magnitude maps to ≈ ±20 % per click
/// through `Viewport::zoom_at`.
pub fn status_bar_zoom(state: &mut EditorState, zoom_in: bool, viewport_w: f32, viewport_h: f32) {
    // `zoom_at`'s factor is `exp(delta * 0.0015)`; ±120 ≈ ±20 %.
    const STEP: f32 = 120.0;
    // `Viewport::zoom_at` works in CANVAS-LOCAL coordinates (every other
    // call feeds it `window_pt - canvas_origin`), so the anchor is the
    // canvas-region centre relative to the canvas origin — NOT the
    // window-space centre.
    let (_cx0, _cy0, cw, ch) = canvas_region(state, viewport_w, viewport_h);
    let center = Point2D::new(cw / 2.0, ch / 2.0);
    state
        .viewport
        .zoom_at(center, if zoom_in { STEP } else { -STEP });
}

/// Zoom + pan so the active page's content is framed within the canvas
/// region (the StatusBar's search action). No-op for an empty page.
/// Callers refresh the scene first and `mark_dirty` after.
pub fn zoom_to_fit(state: &mut EditorState, scene: &LayoutScene, viewport_w: f32, viewport_h: f32) {
    if let Some(content) = scene.content_bounds() {
        let (_l, _t, canvas_w, canvas_h) = canvas_region(state, viewport_w, viewport_h);
        state
            .viewport
            .fit_to_with_max_zoom(content, canvas_w, canvas_h, 64.0, 1.0);
    }
}

/// Zoom + pan so ONE node fills the canvas region — the slides rail's
/// click-to-navigate. Returns whether the camera moved.
///
/// Same `fit_to_with_max_zoom` the StatusBar's search action uses, so a
/// slide is framed by exactly the math a whole page is; only the content
/// rect differs. Camera-only: nothing here touches the document, so
/// navigating a deck never lands on the undo stack.
pub fn zoom_to_fit_node(
    state: &mut EditorState,
    scene: &LayoutScene,
    node_id: &str,
    viewport_w: f32,
    viewport_h: f32,
) -> bool {
    let Some(bounds) = scene
        .active_page()
        .and_then(|page| page.find(node_id))
        .map(|node| node.aggregate_bounds())
    else {
        return false;
    };
    let (_l, _t, canvas_w, canvas_h) = canvas_region(state, viewport_w, viewport_h);
    let before = state.viewport;
    state
        .viewport
        .fit_to_with_max_zoom(bounds, canvas_w, canvas_h, 48.0, 1.0);
    state.viewport != before
}

/// Intrinsic height of the floating tool column for the current tool
/// set, at `dpi`.
fn toolbar_height(toolbar: &Toolbar, dpi: f32) -> f32 {
    toolbar
        .layout(&LayoutCx {
            available_width: TOOLBAR_WIDTH,
            dpi,
        })
        .rect
        .size
        .y
}

/// Shape-picker dropdown rect — anchored to the right of the toolbar's
/// shape slot, clamped so it never overhangs the canvas right edge.
pub fn shape_picker_rect(state: &EditorState, viewport_w: f32, viewport_h: f32) -> Rect {
    // Mobile: anchored above the dock's Shape slot, centered.
    if state.editor_ui.touch_chrome() {
        let dock = super::host_canvas_geometry::touch_dock_rect(state, viewport_w, viewport_h);
        let x = ((viewport_w - SHAPE_PICKER_WIDTH) / 2.0).max(4.0);
        return Rect {
            origin: Point2D::new(x, dock.origin.y - ShapePicker::panel_height() - 8.0),
            size: Point2D::new(SHAPE_PICKER_WIDTH, ShapePicker::panel_height()),
        };
    }
    let (cx0, _cy, cw, _ch) = canvas_region(state, viewport_w, viewport_h);
    let toolbar = Toolbar::for_editor(state);
    let toolbar_rect = Rect {
        origin: Point2D::new(cx0 + TOOLBAR_INSET_X, TOP_BAR_HEIGHT + TOOLBAR_INSET_Y),
        size: Point2D::new(TOOLBAR_WIDTH, toolbar_height(&toolbar, 1.0)),
    };
    let slot = toolbar
        .shape_slot_rect(toolbar_rect)
        .unwrap_or(toolbar_rect);
    let max_x = cx0 + cw - SHAPE_PICKER_WIDTH - 4.0;
    let toolbar_right = toolbar_rect.origin.x + toolbar_rect.size.x;
    Rect {
        origin: Point2D::new((toolbar_right + 8.0).min(max_x), slot.origin.y),
        size: Point2D::new(SHAPE_PICKER_WIDTH, ShapePicker::panel_height()),
    }
}

/// `(anchor, viewport)` for the top-bar import dropdown, so both chromes
/// clamp the popup identically.
pub fn import_menu_anchor(state: &EditorState, viewport_w: f32, viewport_h: f32) -> (Rect, Rect) {
    let button =
        TopBar::for_editor_ui(&state.editor_ui).import_button_rect(top_bar_rect(viewport_w));
    let anchor = Rect {
        origin: button.origin,
        size: Point2D::new(IMPORT_MENU_WIDTH, button.size.y),
    };
    let viewport = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(viewport_w, viewport_h),
    };
    (anchor, viewport)
}

/// Painted bounds of the import dropdown. The overlay predicates need
/// the popup rect, not just its anchor — otherwise the part of the menu
/// hanging over the layer panel is treated as panel chrome, which
/// swallowed hover on the menu's left edge.
pub fn import_menu_rect(state: &EditorState, viewport_w: f32, viewport_h: f32) -> Rect {
    let (anchor, viewport) = import_menu_anchor(state, viewport_w, viewport_h);
    ImportMenu::for_editor_ui(&state.editor_ui).popup_rect(anchor, viewport)
}

/// Close the import dropdown and clear its hover row (both the legacy
/// `import_menu_open` flag and the panel's own `open`, which must not
/// disagree).
pub fn close_import_menu(state: &mut EditorState) {
    state.editor_ui.import_menu_open = false;
    state.editor_ui.import_menu.open = false;
    state.editor_ui.import_menu.hover = None;
}

/// Locale-picker dropdown rect — centred under the TopBar globe button,
/// clamped 8 px inside both viewport edges.
pub fn locale_picker_rect(state: &EditorState, viewport_w: f32) -> Rect {
    let globe = TopBar::for_editor_ui(&state.editor_ui).globe_rect(top_bar_rect(viewport_w));
    let x = (globe.origin.x + globe.size.x / 2.0 - LOCALE_PICKER_WIDTH / 2.0)
        .max(8.0)
        .min(viewport_w - LOCALE_PICKER_WIDTH - 8.0);
    Rect {
        origin: Point2D::new(x, globe.origin.y + globe.size.y + 6.0),
        size: Point2D::new(LOCALE_PICKER_WIDTH, LocalePicker::panel_height()),
    }
}

/// Export quick-menu dropdown rect — hangs under the TopBar download
/// button, right-aligned to it and clamped inside the viewport.
pub fn export_quick_menu_rect(state: &EditorState, viewport_w: f32) -> Rect {
    let bar = top_bar_rect(viewport_w);
    let anchor = TopBar::for_editor_ui(&state.editor_ui).export_button_rect(bar);
    ExportQuickMenu::for_editor_ui(&state.editor_ui).rect_at(anchor, viewport_w)
}

/// Placement shared by all three draggable floating panels: centred on
/// first open, then the stored top-left clamped so at least the header
/// bar (80 × 40 px of it) stays reachable after a viewport resize.
pub fn floating_panel_rect(
    pos: Option<(f32, f32)>,
    panel_w: f32,
    panel_h: f32,
    viewport_w: f32,
    viewport_h: f32,
) -> Rect {
    let (px, py) = pos.unwrap_or_else(|| {
        (
            ((viewport_w - panel_w) / 2.0).max(0.0),
            ((viewport_h - panel_h) / 2.0).max(0.0),
        )
    });
    Rect {
        origin: Point2D::new(
            px.clamp(0.0, (viewport_w - 80.0).max(0.0)),
            py.clamp(0.0, (viewport_h - 40.0).max(0.0)),
        ),
        size: Point2D::new(panel_w, panel_h),
    }
}

/// Floating Design-MD panel rect — `None` when the panel is closed.
pub fn design_md_panel_rect(state: &EditorState, viewport_w: f32, viewport_h: f32) -> Option<Rect> {
    let ui = &state.editor_ui;
    if !ui.design_md_panel.open {
        return None;
    }
    Some(floating_panel_rect(
        ui.design_md_panel.pos,
        DESIGN_MD_PANEL_W,
        DESIGN_MD_PANEL_H,
        viewport_w,
        viewport_h,
    ))
}

/// Floating Component-Browser panel rect — `None` when closed.
pub fn component_browser_panel_rect(
    state: &EditorState,
    viewport_w: f32,
    viewport_h: f32,
) -> Option<Rect> {
    let ui = &state.editor_ui;
    if !ui.component_browser_open {
        return None;
    }
    Some(floating_panel_rect(
        ui.component_browser_pos,
        COMPONENT_BROWSER_PANEL_W,
        COMPONENT_BROWSER_PANEL_H,
        viewport_w,
        viewport_h,
    ))
}

/// Prompt Center rect — a fraction of the viewport, centred horizontally
/// within the live canvas region so open side rails do not visually displace
/// it, and vertically within the band below the top bar.
///
/// It has no intrinsic size. Like the Asset Center it is a gallery: the
/// bigger the window, the bigger the previews, because everything inside it
/// derives from this rect. It used to be a 720x520 box, which on a 1800 px
/// window left a fifth of the screen showing under two rows of cards.
///
/// The ratios ([`PROMPT_CENTER_VIEWPORT_W_RATIO`] /
/// [`PROMPT_CENTER_VIEWPORT_H_RATIO`]) supply the margin, and the floors
/// ([`PROMPT_CENTER_MIN_W`] / [`PROMPT_CENTER_MIN_H`]) keep a small window
/// usable — each floor is itself clamped to what the viewport has, so a
/// window smaller than the floor yields a full-bleed panel rather than one
/// hanging off the edge. Height is measured against the space below the top
/// bar rather than the whole viewport, so the panel is centred in the room it
/// actually has instead of being clamped up under the chrome.
pub fn prompt_center_panel_rect(
    state: &EditorState,
    viewport_w: f32,
    viewport_h: f32,
) -> Option<Rect> {
    if !state.editor_ui.prompt_center.open {
        return None;
    }
    let canvas = canvas_rect(state, viewport_w, viewport_h);
    let viewport_w = viewport_w.max(0.0);
    let viewport_h = viewport_h.max(0.0);
    let available_h = (viewport_h - TOP_BAR_HEIGHT).max(0.0);
    let panel_w =
        (viewport_w * PROMPT_CENTER_VIEWPORT_W_RATIO).max(PROMPT_CENTER_MIN_W.min(viewport_w));
    let panel_h =
        (available_h * PROMPT_CENTER_VIEWPORT_H_RATIO).max(PROMPT_CENTER_MIN_H.min(available_h));
    let x = canvas.origin.x + (canvas.size.x - panel_w) / 2.0;
    let y = viewport_h - available_h + (available_h - panel_h) / 2.0;
    Some(Rect {
        origin: Point2D::new(
            x.clamp(0.0, (viewport_w - panel_w).max(0.0)),
            y.clamp(0.0, (viewport_h - panel_h).max(0.0)),
        ),
        size: Point2D::new(panel_w, panel_h),
    })
}

/// Asset Center rect — inset by [`SCENE_TEMPLATE_GALLERY_INSET`] from the
/// usable host viewport. Desktop keeps the persistent top bar clear; touch
/// hosts hand the gallery the full safe-area-adjusted viewport because their
/// desktop top bar is not painted.
///
/// Unlike the Prompt Center it has no intrinsic size. It is a gallery: the
/// bigger the window, the bigger the previews, because everything inside
/// derives from this rect rather than from a pair of width/height constants.
///
/// Horizontally it spans the viewport rather than the canvas region — the
/// same span as its scrim. Centring inside the canvas region put the gallery
/// visibly right of centre whenever the left rail was open, because the rail's
/// width came off one side only. It overlaps both rails, which is what a
/// full-window gallery is meant to do; the scrim already swallows every press
/// that reaches them.
///
/// The inset shrinks on a small viewport instead of pushing the rect negative
/// — an eighth of the shorter axis, capped at the nominal margin — so a
/// narrow window still yields a usable (if tight) gallery rather than an empty
/// or inverted rect.
pub fn scene_template_panel_rect(
    state: &EditorState,
    viewport_w: f32,
    viewport_h: f32,
) -> Option<Rect> {
    if !state.editor_ui.scene_template_center.open {
        return None;
    }
    let available_w = viewport_w.max(0.0);
    let top = if state.editor_ui.touch_chrome() {
        0.0
    } else {
        TOP_BAR_HEIGHT
    };
    let available_h = (viewport_h - top).max(0.0);
    let inset = SCENE_TEMPLATE_GALLERY_INSET
        .min(available_w / 8.0)
        .min(available_h / 8.0)
        .max(0.0);
    Some(Rect {
        origin: Point2D::new(inset, top + inset),
        size: Point2D::new(
            (available_w - inset * 2.0).max(0.0),
            (available_h - inset * 2.0).max(0.0),
        ),
    })
}

/// The dimming layer painted under the Asset Center gallery — the whole
/// viewport, chrome included.
///
/// It covers more than the panel's own canvas region on purpose: dimming only
/// the canvas would leave the top bar and rails at full brightness around a
/// darkened middle, which reads as a popup on a lit page rather than as an
/// editor that stepped back. A press anywhere on it dismisses the gallery.
pub fn scene_template_scrim_rect(
    state: &EditorState,
    viewport_w: f32,
    viewport_h: f32,
) -> Option<Rect> {
    if !state.editor_ui.scene_template_center.open {
        return None;
    }
    Some(Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(viewport_w.max(0.0), viewport_h.max(0.0)),
    })
}

/// Floating Icon-picker panel rect — `None` when closed.
pub fn icon_picker_panel_rect(
    state: &EditorState,
    viewport_w: f32,
    viewport_h: f32,
) -> Option<Rect> {
    let ui = &state.editor_ui;
    if !ui.icon_picker.open {
        return None;
    }
    Some(floating_panel_rect(
        ui.icon_picker_panel_pos,
        ICON_PICKER_PANEL_W,
        ICON_PICKER_PANEL_H,
        viewport_w,
        viewport_h,
    ))
}

#[cfg(test)]
#[path = "host_overlay_geometry_tests.rs"]
mod host_overlay_geometry_tests;
