//! Image-node section assets: the node-derived view (search/prompt
//! seeds + local-asset warning) and the Search / Generate popovers.
//!
//! Port of `apps/web/src/components/panels/image-section.tsx` +
//! `image-search-popover.tsx` + `image-generate-popover.tsx` +
//! `use-image-asset-state.ts` + `local-image-warning.tsx`.
//!
//! Layout note: the popovers are LATE overlays (TS portals with
//! `side="left"`) — they anchor to the section's Search / Generate
//! buttons and extend out of the right rail, so they are painted via
//! `PropertyPanel::paint_overlays` and hit-tested before the generic
//! action walker. The popover button strings are literal English
//! here because the TS components hardcode them (no i18n keys).
//!
//! Divergences from TS (documented):
//! - The "missing file" check is a host fs probe written into
//!   `ImagePanelState.asset_check` (TS detects it via an <img>
//!   onerror). The wasm host never writes a check, so the warning
//!   row is desktop-only for now.
//! - The asset path under the warning truncates to one line (TS
//!   wraps with `break-all`).

use crate::widgets::property_panel::PropertyPanelAction;
use crate::widgets::property_panel_inputs::{
    INPUT_HEIGHT, PAD_X, SECTION_GAP, SECTION_HEADER_HEIGHT,
};
use crate::widgets::property_panel_layout::VisibleSections;
use crate::{Point2D, Rect};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::SizingBehavior;
use op_editor_core::image_panel_state::{ImageAssetStatus, ImageGeneratePhase, ImagePanelState};
use op_editor_core::EditorState;

// --- Section row metrics (also used by the layout walkers) --------

/// Height of the local-asset warning row (TS bordered box with a
/// message + path line).
pub const IMAGE_WARNING_H: f32 = 40.0;
/// Height of the Search / Generate buttons (TS `h-7`).
pub const IMAGE_BUTTON_H: f32 = 28.0;
/// Vertical gap between the section's stacked rows (TS space-y-1.5).
pub const IMAGE_ROW_GAP: f32 = 6.0;

const SEARCH_POPOVER_W: f32 = 320.0; // TS w-80
const GENERATE_POPOVER_W: f32 = 288.0; // TS w-72
pub(crate) const POPOVER_PAD: f32 = 12.0; // TS p-3
const POPOVER_GAP: f32 = 8.0; // TS sideOffset=8
const EMPTY_STATE_H: f32 = 90.0; // TS py-8 icon + label block

// --- Node-derived view ---------------------------------------------

/// Warning surfaced under the thumbnail row (TS `LocalImageWarning`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageAssetWarning {
    /// Literal TS message string.
    pub message: &'static str,
    pub asset_path: String,
}

/// Per-frame snapshot of the selected image node's panel inputs.
#[derive(Debug, Clone, PartialEq)]
pub struct ImagePanelView {
    pub node_id: String,
    pub src: Option<String>,
    /// `imageSearchQuery ?? name ?? ''` (TS `initialQuery`).
    pub search_seed: String,
    /// `imagePrompt ?? name ?? ''` (TS `initialPrompt`).
    pub prompt_seed: String,
    /// Concrete node dimensions, when numeric — passed to generation
    /// for aspect-ratio-aware output (TS width/height props).
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub warning: Option<ImageAssetWarning>,
}

/// Build the view for a selected image node. `None` for any other
/// node kind.
pub fn image_panel_view(state: &EditorState, node: &PenNode) -> Option<ImagePanelView> {
    let PenNode::Image(image) = node else {
        return None;
    };
    let node_id = image.base.id.as_str().to_string();
    let name = image.base.name.clone().unwrap_or_default();
    let src = (!image.src.trim().is_empty()).then(|| image.src.to_string());
    let warning = src
        .as_deref()
        .filter(|s| is_local_asset_path(s))
        .and_then(|s| {
            let message = match state.editor_ui.image_panel.status_for(&node_id, s) {
                Some(ImageAssetStatus::Unresolved) => "Relative image path cannot be resolved yet",
                Some(ImageAssetStatus::Missing) => "Image file is missing",
                Some(ImageAssetStatus::Ok) | None => return None,
            };
            Some(ImageAssetWarning {
                message,
                asset_path: s.to_string(),
            })
        });
    Some(ImagePanelView {
        node_id,
        search_seed: image
            .image_search_query
            .clone()
            .filter(|q| !q.trim().is_empty())
            .unwrap_or_else(|| name.clone()),
        prompt_seed: image
            .image_prompt
            .clone()
            .filter(|p| !p.trim().is_empty())
            .unwrap_or(name),
        width: sizing_number(&image.width),
        height: sizing_number(&image.height),
        src,
        warning,
    })
}

fn sizing_number(size: &Option<SizingBehavior>) -> Option<f64> {
    match size {
        Some(SizingBehavior::Number(px)) => Some(*px),
        _ => None,
    }
}

/// TS `isLocalAssetPath` (document-assets.ts:22-28): everything that
/// is not a `data:` / `http(s):` / `blob:` URL or a same-origin
/// runtime route is a file-system asset path.
pub fn is_local_asset_path(path: &str) -> bool {
    let t = path.trim();
    if t.is_empty() {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("data:")
        || lower.starts_with("http:")
        || lower.starts_with("https:")
        || lower.starts_with("blob:")
    {
        return false;
    }
    // Same-origin runtime endpoints, not fs paths (TS
    // SAME_ORIGIN_ROUTE_RE).
    if t.starts_with("/api/") || t.starts_with("/_/") {
        return false;
    }
    true
}

/// Active image-generation profile summary (TS
/// `getActiveImageGenProfile()`): the profile matching the active id,
/// else the first profile. `configured` mirrors `!!profile?.apiKey`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageGenProfileView {
    pub configured: bool,
    pub name: String,
    pub provider: &'static str,
    pub model: String,
}

pub fn image_gen_profile_view(state: &EditorState) -> Option<ImageGenProfileView> {
    let settings = &state.editor_ui.agent_settings;
    let profile = settings
        .image_gen_profiles
        .iter()
        .find(|p| Some(&p.id) == settings.active_image_gen_profile_id.as_ref())
        .or_else(|| settings.image_gen_profiles.first())?;
    Some(ImageGenProfileView {
        configured: !profile.api_key.trim().is_empty(),
        name: profile.name.clone(),
        provider: profile.provider.label(),
        model: profile.model.clone(),
    })
}

// --- Section geometry ----------------------------------------------

/// Top y of the image section (after icon + text sections), mirroring
/// the walker in `action_button_rects_with_fill_picker`. `None` when
/// the section is hidden.
pub(crate) fn image_section_top(panel_rect: Rect, visible: VisibleSections) -> Option<f32> {
    if !visible.image {
        return None;
    }
    let mut y = crate::widgets::property_panel_text::sections_top_before_text(panel_rect, visible);
    if visible.text {
        y += crate::widgets::property_panel_text::text_section_height();
        y += SECTION_GAP;
    }
    Some(y)
}

/// The section's Search (left) and Generate (right) button rects.
pub(crate) fn image_buttons_rects(
    panel_rect: Rect,
    visible: VisibleSections,
) -> Option<(Rect, Rect)> {
    let top = image_section_top(panel_rect, visible)?;
    let x0 = panel_rect.origin.x;
    let usable_w = panel_rect.size.x - PAD_X * 2.0;
    let mut y = top + SECTION_HEADER_HEIGHT + INPUT_HEIGHT;
    if visible.image_warning {
        y += IMAGE_ROW_GAP + IMAGE_WARNING_H;
    }
    y += IMAGE_ROW_GAP;
    let half_w = (usable_w - 4.0) / 2.0; // TS flex gap-1
    let search = Rect {
        origin: Point2D::new(x0 + PAD_X, y),
        size: Point2D::new(half_w, IMAGE_BUTTON_H),
    };
    let generate = Rect {
        origin: Point2D::new(x0 + PAD_X + half_w + 4.0, y),
        size: Point2D::new(half_w, IMAGE_BUTTON_H),
    };
    Some((search, generate))
}

// --- Search popover --------------------------------------------------

pub struct SearchPopoverLayout {
    pub popup: Rect,
    pub input: Rect,
    pub submit: Rect,
    /// One rect per result cell (3-column grid).
    pub cells: Vec<Rect>,
    /// Footer license strip rect, when painted.
    pub footer: Option<Rect>,
    /// Empty / loading body rect (no results yet).
    pub body: Rect,
}

pub fn search_popover_layout(
    panel_rect: Rect,
    visible: VisibleSections,
    state: &ImagePanelState,
) -> Option<SearchPopoverLayout> {
    let (search_btn, _) = image_buttons_rects(panel_rect, visible)?;
    let x = search_btn.origin.x - SEARCH_POPOVER_W - POPOVER_GAP;
    let y = search_btn.origin.y;
    let inner_w = SEARCH_POPOVER_W - POPOVER_PAD * 2.0;
    let input_h = 28.0;
    let input = Rect {
        origin: Point2D::new(x + POPOVER_PAD, y + POPOVER_PAD),
        size: Point2D::new(inner_w - input_h - 6.0, input_h),
    };
    let submit = Rect {
        origin: Point2D::new(input.origin.x + input.size.x + 6.0, input.origin.y),
        size: Point2D::new(input_h, input_h),
    };
    let body_y = input.origin.y + input_h + POPOVER_PAD;
    let mut cells = Vec::new();
    let mut footer = None;
    let body_h;
    if state.search_loading || state.search_results.is_empty() {
        body_h = EMPTY_STATE_H;
    } else {
        let cell = (inner_w - 12.0) / 3.0;
        let rows = state.search_results.len().div_ceil(3);
        for (i, _) in state.search_results.iter().enumerate() {
            let col = i % 3;
            let row = i / 3;
            cells.push(Rect {
                origin: Point2D::new(
                    x + POPOVER_PAD + col as f32 * (cell + 6.0),
                    body_y + row as f32 * (cell + 6.0),
                ),
                size: Point2D::new(cell, cell),
            });
        }
        let grid_h = rows as f32 * cell + (rows.saturating_sub(1)) as f32 * 6.0;
        let footer_h = if state.search_source.is_some() {
            36.0
        } else {
            0.0
        };
        if footer_h > 0.0 {
            footer = Some(Rect {
                origin: Point2D::new(x + POPOVER_PAD, body_y + grid_h + 8.0),
                size: Point2D::new(inner_w, footer_h - 8.0),
            });
        }
        body_h = grid_h + footer_h;
    }
    let body = Rect {
        origin: Point2D::new(x + POPOVER_PAD, body_y),
        size: Point2D::new(inner_w, body_h),
    };
    let popup = Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(
            SEARCH_POPOVER_W,
            POPOVER_PAD + input_h + POPOVER_PAD + body_h + POPOVER_PAD,
        ),
    };
    Some(SearchPopoverLayout {
        popup,
        input,
        submit,
        cells,
        footer,
        body,
    })
}

// --- Generate popover ------------------------------------------------

/// Which view the generate popover paints (TS state machine + the
/// not-configured gate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratePopoverView {
    NotConfigured,
    Loading,
    Preview,
    Idle,
}

pub struct GeneratePopoverLayout {
    pub popup: Rect,
    pub view: GeneratePopoverView,
    /// Primary button: Open Settings / Generate / Apply.
    pub primary: Option<Rect>,
    /// Secondary button: Retry (preview view only).
    pub secondary: Option<Rect>,
    pub textarea: Option<Rect>,
    pub preview: Option<Rect>,
}

pub fn generate_popover_view(
    state: &ImagePanelState,
    profile: Option<&ImageGenProfileView>,
) -> GeneratePopoverView {
    if !profile.is_some_and(|p| p.configured) {
        return GeneratePopoverView::NotConfigured;
    }
    match state.generate_phase {
        ImageGeneratePhase::Loading => GeneratePopoverView::Loading,
        ImageGeneratePhase::Preview if state.generate_preview.is_some() => {
            GeneratePopoverView::Preview
        }
        // Error paints inside the idle view (TS passes `error` to
        // IdleView).
        _ => GeneratePopoverView::Idle,
    }
}

pub fn generate_popover_layout(
    panel_rect: Rect,
    visible: VisibleSections,
    state: &ImagePanelState,
    profile: Option<&ImageGenProfileView>,
) -> Option<GeneratePopoverLayout> {
    let (_, generate_btn) = image_buttons_rects(panel_rect, visible)?;
    let x = generate_btn.origin.x - GENERATE_POPOVER_W - POPOVER_GAP;
    let y = generate_btn.origin.y;
    let inner_w = GENERATE_POPOVER_W - POPOVER_PAD * 2.0;
    let view = generate_popover_view(state, profile);
    let mut primary = None;
    let mut secondary = None;
    let mut textarea = None;
    let mut preview = None;
    let content_h;
    match view {
        GeneratePopoverView::NotConfigured => {
            // Icon 32 + text + Open Settings button, all centred.
            let btn_w = 110.0;
            primary = Some(Rect {
                origin: Point2D::new(
                    x + (GENERATE_POPOVER_W - btn_w) / 2.0,
                    y + POPOVER_PAD + 32.0 + 10.0 + 16.0 + 10.0,
                ),
                size: Point2D::new(btn_w, 28.0),
            });
            content_h = 32.0 + 10.0 + 16.0 + 10.0 + 28.0;
        }
        GeneratePopoverView::Loading => {
            content_h = 60.0;
        }
        GeneratePopoverView::Preview => {
            let img_h = 200.0; // TS maxHeight: 200
            preview = Some(Rect {
                origin: Point2D::new(x + POPOVER_PAD, y + POPOVER_PAD),
                size: Point2D::new(inner_w, img_h),
            });
            let retry_w = 64.0;
            let apply_w = inner_w - retry_w - 8.0;
            let row_y = y + POPOVER_PAD + img_h + 12.0;
            primary = Some(Rect {
                origin: Point2D::new(x + POPOVER_PAD, row_y),
                size: Point2D::new(apply_w, 28.0),
            });
            secondary = Some(Rect {
                origin: Point2D::new(x + POPOVER_PAD + apply_w + 8.0, row_y),
                size: Point2D::new(retry_w, 28.0),
            });
            content_h = img_h + 12.0 + 28.0;
        }
        GeneratePopoverView::Idle => {
            let ta_h = 48.0; // TS rows={2}
            textarea = Some(Rect {
                origin: Point2D::new(x + POPOVER_PAD, y + POPOVER_PAD),
                size: Point2D::new(inner_w, ta_h),
            });
            let error_h = if state.generate_phase == ImageGeneratePhase::Error {
                30.0
            } else {
                0.0
            };
            primary = Some(Rect {
                origin: Point2D::new(x + POPOVER_PAD, y + POPOVER_PAD + ta_h + error_h + 12.0),
                size: Point2D::new(inner_w, 28.0),
            });
            // + footer "profile · provider · model" line.
            content_h = ta_h + error_h + 12.0 + 28.0 + 8.0 + 14.0;
        }
    }
    let popup = Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(GENERATE_POPOVER_W, content_h + POPOVER_PAD * 2.0),
    };
    Some(GeneratePopoverLayout {
        popup,
        view,
        primary,
        secondary,
        textarea,
        preview,
    })
}

// --- Hit-testing ------------------------------------------------------

/// Action for a press while one of the popovers is open. `None` for
/// presses inside the popup body that hit no control (the host
/// swallows those via [`image_popovers_contain`]).
pub fn image_popover_action_at(
    panel_rect: Rect,
    visible: VisibleSections,
    state: &ImagePanelState,
    profile: Option<&ImageGenProfileView>,
    point: Point2D,
) -> Option<PropertyPanelAction> {
    if state.search_open {
        if let Some(layout) = search_popover_layout(panel_rect, visible, state) {
            if (layout.submit).contains(point) {
                return Some(PropertyPanelAction::RunImageSearch);
            }
            for (i, cell) in layout.cells.iter().enumerate() {
                if (*cell).contains(point) {
                    return Some(PropertyPanelAction::SelectImageSearchResult(i));
                }
            }
        }
    }
    if state.generate_open {
        if let Some(layout) = generate_popover_layout(panel_rect, visible, state, profile) {
            if layout.primary.is_some_and(|r| (r).contains(point)) {
                return Some(match layout.view {
                    GeneratePopoverView::NotConfigured => PropertyPanelAction::OpenImageGenSettings,
                    GeneratePopoverView::Preview => PropertyPanelAction::ApplyGeneratedImage,
                    GeneratePopoverView::Idle => PropertyPanelAction::RunImageGenerate,
                    GeneratePopoverView::Loading => return None,
                });
            }
            if layout.secondary.is_some_and(|r| (r).contains(point)) {
                return Some(PropertyPanelAction::RetryImageGenerate);
            }
        }
    }
    None
}

/// Whether `point` is inside an open popover's popup body.
pub fn image_popovers_contain(
    panel_rect: Rect,
    visible: VisibleSections,
    state: &ImagePanelState,
    profile: Option<&ImageGenProfileView>,
    point: Point2D,
) -> bool {
    if state.search_open
        && search_popover_layout(panel_rect, visible, state)
            .is_some_and(|l| (l.popup).contains(point))
    {
        return true;
    }
    if state.generate_open
        && generate_popover_layout(panel_rect, visible, state, profile)
            .is_some_and(|l| (l.popup).contains(point))
    {
        return true;
    }
    false
}
