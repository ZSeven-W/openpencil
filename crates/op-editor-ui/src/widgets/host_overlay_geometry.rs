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
    ImportMenu, LayoutCx, LocalePicker, ShapePicker, Toolbar, TopBar, Widget,
    COMPONENT_BROWSER_PANEL_H, COMPONENT_BROWSER_PANEL_W, DESIGN_MD_PANEL_H, DESIGN_MD_PANEL_W,
    ICON_PICKER_PANEL_H, ICON_PICKER_PANEL_W, IMPORT_MENU_WIDTH, LOCALE_PICKER_WIDTH,
    PROMPT_CENTER_PANEL_H, PROMPT_CENTER_PANEL_W, SCENE_TEMPLATE_PANEL_H, SCENE_TEMPLATE_PANEL_W,
    SHAPE_PICKER_WIDTH, TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
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

/// Prompt Center rect — centred within the live canvas region, rather
/// than the whole viewport, so open side rails do not visually displace it.
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
    let panel_w = PROMPT_CENTER_PANEL_W.min(viewport_w);
    let panel_h = PROMPT_CENTER_PANEL_H.min(viewport_h);
    let x = canvas.origin.x + (canvas.size.x - panel_w) / 2.0;
    let y = canvas.origin.y + (canvas.size.y - panel_h) / 2.0;
    Some(Rect {
        origin: Point2D::new(
            x.clamp(0.0, (viewport_w - panel_w).max(0.0)),
            y.clamp(0.0, (viewport_h - panel_h).max(0.0)),
        ),
        size: Point2D::new(panel_w, panel_h),
    })
}

/// Scene Template Center rect — centred in the canvas region, same as the
/// Prompt Center so the two panels occupy identical space.
pub fn scene_template_panel_rect(
    state: &EditorState,
    viewport_w: f32,
    viewport_h: f32,
) -> Option<Rect> {
    if !state.editor_ui.scene_template_center.open {
        return None;
    }
    let canvas = canvas_rect(state, viewport_w, viewport_h);
    let viewport_w = viewport_w.max(0.0);
    let viewport_h = viewport_h.max(0.0);
    let panel_w = SCENE_TEMPLATE_PANEL_W.min(viewport_w);
    let panel_h = SCENE_TEMPLATE_PANEL_H.min(viewport_h);
    let x = canvas.origin.x + (canvas.size.x - panel_w) / 2.0;
    let y = canvas.origin.y + (canvas.size.y - panel_h) / 2.0;
    Some(Rect {
        origin: Point2D::new(
            x.clamp(0.0, (viewport_w - panel_w).max(0.0)),
            y.clamp(0.0, (viewport_h - panel_h).max(0.0)),
        ),
        size: Point2D::new(panel_w, panel_h),
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
