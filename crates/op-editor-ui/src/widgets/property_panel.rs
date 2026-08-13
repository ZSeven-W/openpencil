//! `PropertyPanel` — right-rail node inspector (Step 6).
//!
//! Mirrors `apps/web/src/components/panels/right-panel.tsx` and the
//! per-section TS files (`*-section.tsx`). The bulk of the paint
//! logic lives in [`super::property_panel_sections`] — this file
//! holds the `PropertyPanel` struct, its scroll / capability
//! geometry, and the hover-wash helper. Construction, hit-testing,
//! floating menus, and the `Widget` paint pass live in the sibling
//! `property_panel/` submodules. Splitting the file keeps the pieces
//! under the openpencil 800-line ceiling.
//!
//! Sections (top → bottom, mirroring TS order):
//!   1. Tab strip (Design / Code)
//!   2. Header (kind label) + Create component button
//!   3. Position — X / Y / rotation / R
//!   4. Flex layout — 3 layout-mode buttons
//!   5. Size — W / H + 5 sizing checkboxes
//!   6. Layer — opacity row
//!   7. Fill — solid color rows + add affordance
//!   8. Stroke — color + width row
//!   9. Effects — empty list + add affordance
//!  10. Export — scale + format dropdowns
//!
//! Conditional rendering: TS app does `{hasSelection && <RightPanel/>}`.
//! Host calls [`PropertyPanel::for_selection`] which returns
//! `Option<Self>`; `None` = panel hidden entirely.

use crate::theme::Theme;
use crate::widgets::property_panel_sections as sections;
use crate::widgets::text_metrics;
use crate::widgets::WidgetId;
use crate::{Point2D, Rect};
use jian_widgets::components::select::SelectState;
use op_editor_core::PropertyFocus;

// Construction / hit-test / floating-menu / paint halves of
// `impl PropertyPanel` — sibling submodules kept under the 800-line
// ceiling. They contribute inherent methods, so every existing call
// site resolves unchanged.
mod build;
mod density;
mod hit;
mod menus;
mod paint;

pub use build::ColorVariableOption;

pub const PROPERTY_PANEL_WIDTH: f32 = 280.0;

// `PropertyPanelAction` lives in `property_panel_action.rs` (split
// out for the 800-line ceiling); re-exported so every existing
// `widgets::PropertyPanelAction` / `property_panel::PropertyPanelAction`
// path is unchanged.
pub use crate::widgets::property_panel_action::{
    CompositingTarget, FontWeightChoice, LayoutAlignValue, LayoutJustifyValue, PropertyPanelAction,
    TextAlignValue, TextGrowthValue, TextVerticalAlignValue,
};

// `SectionCapabilities` lives in `property_panel_layout.rs`
// alongside `VisibleSections` (the section-visibility mask it
// feeds); re-exported so `property_panel::SectionCapabilities`
// resolves unchanged.
pub(crate) use crate::widgets::property_panel_layout::SectionCapabilities;
pub use crate::widgets::property_panel_snapshot::{
    EffectKind, EffectSummary, EllipseArcSummary, FillSummary, GradientStopSummary, NodeSnapshot,
    WidgetKind, WidgetSummary,
};

/// Result of hit-testing the Effects "+" add-menu popover.
#[derive(Debug, Clone, PartialEq)]
pub enum EffectAddMenuHit {
    /// A choice row was clicked — apply this action, then close.
    Row(PropertyPanelAction),
    /// Inside the menu chrome (not a row) — swallow, keep open.
    Inside,
    /// Outside the menu — dismiss.
    Outside,
}

pub struct PropertyPanel {
    pub id: WidgetId,
    pub snapshot: NodeSnapshot,
    pub theme: Theme,
    /// Paint / hit-test scale for touch chrome. Desktop stays exactly 1:1.
    pub(crate) density_scale: f32,
    /// Design-tab page inspector target. It is built only when no
    /// node is selected and uses a dedicated paint / hit-test branch;
    /// node sections never consult the neutral snapshot in this mode.
    pub page_only: bool,
    pub page_name: String,
    /// Raw authored page background (`#RRGGBB` / `#RRGGBBAA`). Keep
    /// `None` distinct from white so focus + blur is side-effect free.
    pub page_background: Option<String>,
    /// Localised chrome strings — `Document::t` lookups resolved
    /// once at construction time so every section paint hands
    /// straight to the renderer without re-walking the i18n table.
    pub labels: sections::PropertyLabels,
    /// Which input row the user is editing. `None` when no input
    /// is focused (panel paints all values from the snapshot).
    pub focus: Option<PropertyFocus>,
    /// Live edit-buffer for the focused input. Empty when nothing
    /// is focused. The host fills this on click + mutates on
    /// keystroke; the panel paints it as the field's value.
    pub draft: String,
    /// Shared text input state for the focused property/effect field.
    pub input: jian_core::text_input::TextInputState,
    /// Caret byte-offset into `draft` (ASCII drafts → char index).
    pub caret_pos: usize,
    /// Whether Ctrl/Cmd+A selected the full focused draft.
    pub select_all: bool,
    /// Host clock ms for caret blink.
    pub now_ms: u64,
    /// Active flex-layout button.
    pub flex_layout: op_editor_core::FlexLayout,
    /// 5 size checkboxes — fill / hug / clip.
    pub size_flags: sections::SizeFlags,
    /// Active fill type — drives the dropdown label + picker.
    pub fill_type: op_editor_core::FillType,
    pub fill_type_picker: SelectState,
    /// Which fill row the open fill-type dropdown targets (the Fill
    /// section stacks one type dropdown per fill).
    pub fill_type_picker_index: usize,
    /// Shared two-column Blend / one-column Mask picker state.
    pub compositing_picker: SelectState,
    pub compositing_picker_target: Option<CompositingTarget>,
    /// Registered document components offered by the selected Ref's
    /// inline Swap control. Empty for non-Refs and when no alternative
    /// target exists.
    pub instance_component_options:
        std::sync::Arc<[crate::widgets::property_panel_instance::InstanceComponentOption]>,
    /// Current canonical `ref` target of the selected Ref.
    pub instance_component_target: Option<String>,
    /// Whether the inline component list is expanded for this anchor.
    pub instance_component_picker_open: bool,
    pub corner_expand_open: bool,
    /// Whether the Effects "+" add-menu is
    /// open.
    pub effect_add_picker_open: bool,
    /// Whether the Interactions section's Navigate/Back/Remove popover
    /// is open.
    pub interaction_menu_open: bool,
    /// Row index hovered in the open Interactions popover (`None` =
    /// none).
    pub interaction_menu_hover: Option<usize>,
    /// Every `screen` route path authored on the active page's
    /// top-level frames — the Interactions popover's "Navigate to…"
    /// row source (see `property_panel_interactions::document_screen_paths`).
    pub screen_paths: Vec<String>,
    pub color_variable_picker_open: Option<op_editor_core::ColorTarget>,
    /// Scroll offset (px, ≥ 0) of the open colour-variable popup's own
    /// list. The popup is height-capped, so a long variable set scrolls
    /// inside it instead of stretching the inspector.
    pub color_variable_picker_scroll: f32,
    /// Row slot of the open colour-variable popup under the cursor
    /// (`None` = none), mirrored from
    /// `editor_ui.property_color_variable_picker_hover`.
    pub color_variable_picker_hover: Option<usize>,
    pub color_variables: Vec<ColorVariableOption>,
    pub fill_variable_ref: Option<String>,
    pub stroke_variable_ref: Option<String>,
    pub color_variable_count: usize,
    pub image_fill_popover_open: bool,
    pub font_picker: SelectState,
    /// Live type-ahead filter + scroll + hovered entry of the
    /// font-family picker, plus the host-enumerated system families
    /// (see `property_panel_typography`).
    pub font_picker_search: String,
    pub system_font_families: std::sync::Arc<Vec<String>>,
    pub bundled_font_families: std::sync::Arc<Vec<String>>,
    /// User-imported font families (see `editor_ui.imported_font_families`).
    /// The picker paints these first, above bundled + system.
    pub imported_font_families: std::sync::Arc<Vec<String>>,
    /// Whether the host supports font import (desktop true / web false) —
    /// gates the picker's Import row so web shows no dead control.
    pub font_import_supported: bool,
    /// Whether the cursor is over the picker's "Import font…" row —
    /// drives that row's hover wash (host tracks it on cursor-move).
    pub font_picker_import_hover: bool,
    /// Image-node Search / Generate popover state (cloned from
    /// `editor_ui.image_panel`; result thumbs are `Arc`s so this
    /// per-frame clone stays cheap).
    pub image_panel: op_editor_core::image_panel_state::ImagePanelState,
    /// Node-derived image-section inputs (seeds + warning) — `Some`
    /// only when a single image node is selected.
    pub image_panel_view: Option<crate::widgets::property_panel_image_assets::ImagePanelView>,
    /// Active image-generation profile summary for the Generate
    /// popover's configured / not-configured gate.
    pub image_gen_profile: Option<crate::widgets::property_panel_image_assets::ImageGenProfileView>,
    pub font_weight_picker_open: bool,
    /// Hovered weight-dropdown row index (when the dropdown is open).
    pub font_weight_picker_hover: Option<usize>,
    pub font_weight_picker_pressed: Option<usize>,
    /// Resolved padding edit mode (UI pin or derived from the node's
    /// values) + whether the gear popover is open.
    pub padding_edit_mode: op_editor_core::PaddingEditMode,
    pub padding_mode_popover_open: bool,
    /// Hovered padding-mode popover row index (gear popover open).
    pub padding_mode_popover_hover: Option<usize>,
    pub stroke_edit_mode: op_editor_core::PaddingEditMode,
    pub stroke_mode_popover_open: bool,
    /// Hovered stroke-mode popover row index (gear popover open).
    pub stroke_mode_popover_hover: Option<usize>,
    /// True for multi-select aggregate (inputs inert, "N items").
    pub is_multi: bool,
    /// Active header tab — toggled by Cmd+Shift+C.
    pub tab: op_editor_core::PropertyTab,
    /// Header tab currently hovered. Used only for the pinned tab strip.
    pub tab_hover: Option<op_editor_core::PropertyTab>,
    /// Current export format + scale, shown on the Export section's
    /// two dropdowns. Clicking a dropdown opens its inline select
    /// popup (NOT the Export modal).
    pub export_format: op_editor_core::ExportFormat,
    pub export_scale: f32,
    /// Whether the Export section's scale / format inline select
    /// popups are open.
    pub export_scale_picker_open: bool,
    pub export_format_picker_open: bool,
    /// Row index the cursor is over in the open Export select
    /// popup — `None` when no popup is open or no row is hovered.
    pub export_picker_hover: Option<usize>,
    /// Row index the cursor is over in the open Effects "+" add-menu
    /// (`None` when closed or no row hovered) — drives the row highlight.
    pub effect_add_menu_hover: Option<usize>,
    /// Vertical scroll offset (px, ≥ 0) — paint + hit-test shift the
    /// section content up by this so a tall inspector stays usable.
    pub scroll: f32,
    /// Active UI locale — threaded into the Fill section so its
    /// type label / picker / body sub-labels translate.
    pub locale: op_editor_core::Locale,
    /// Focused effect-parameter value, if any — drives the Effects
    /// section's editable value boxes.
    pub effect_param_focus: Option<op_editor_core::editor_ui_state::EffectParamFocus>,
    /// Code-generation state painted by the Code tab. Cloned from the
    /// `EditorState` at construction (like `snapshot`) so the panel
    /// owns an immutable view; generation logic is wired later (P3).
    pub codegen: op_editor_core::codegen::CodegenState,
    /// Pressed Code-tab action currently held by the primary pointer.
    pub codegen_pressed: Option<op_editor_core::codegen::CodegenHover>,
    /// Index into `action_button_rects_with_fill_picker` of the action
    /// button the cursor is over — drives its `theme.button_hover` wash.
    pub action_hover: Option<usize>,
    /// Index into `action_button_rects_with_fill_picker` of the action
    /// button currently pressed by the primary pointer.
    pub action_pressed: Option<usize>,
}

impl PropertyPanel {
    /// Capability mask that drives which sections paint for this
    /// panel state. Multi-select uses a dedicated mask (`for_multi`)
    /// that keeps Size + Position + Layer + Effects + Export and
    /// hides Flex + Fill + Stroke; single-select falls back to the
    /// snapshot's `kind_variant` (`for_kind`). Paint + hit-test
    /// must call this rather than `SectionCapabilities::for_kind`
    /// directly so the multi-select carve-out can't regress
    /// silently.
    pub(crate) fn capabilities(&self) -> SectionCapabilities {
        if self.is_multi {
            SectionCapabilities::for_multi()
        } else {
            SectionCapabilities::for_kind(&self.snapshot.kind_variant)
        }
    }
}

impl PropertyPanel {
    /// `self.scroll` clamped to the current content's scrollable
    /// range. The host only re-clamps the stored offset on a wheel
    /// event, so selecting a shorter node (fewer sections / effects)
    /// could otherwise leave the panel scrolled past its end —
    /// every paint / hit-test reads through this so the view
    /// self-corrects on the very next frame.
    pub(crate) fn effective_scroll(&self, panel_rect: Rect) -> f32 {
        let max = (self.logical_content_height(panel_rect) - panel_rect.size.y).max(0.0);
        self.logical_length(self.scroll).clamp(0.0, max)
    }

    /// `panel_rect` shifted up by the (clamped) scroll offset. Both
    /// paint and every hit-test walker start their y-walk from this
    /// rect, so the panel scrolls as one piece and clicks stay
    /// aligned with what is drawn.
    pub(crate) fn scrolled_rect(&self, panel_rect: Rect) -> Rect {
        Rect {
            origin: Point2D::new(
                panel_rect.origin.x,
                panel_rect.origin.y - self.effective_scroll(panel_rect),
            ),
            size: panel_rect.size,
        }
    }

    /// Whether `point` is inside the scrolling section viewport —
    /// the panel below the pinned tab strip. A click in the tab-strip
    /// band must not fall through to a section row scrolled up
    /// under it (paint clips there; hit-test must agree).
    fn point_in_section_viewport(&self, panel_rect: Rect, point: Point2D) -> bool {
        panel_rect.contains(point)
            && point.y >= panel_rect.origin.y + crate::widgets::property_panel_inputs::TAB_HEIGHT
    }

    /// Total height (px) of the panel's section content — drives the
    /// scroll clamp so the inspector can't scroll past its end.
    pub fn content_height(&self, panel_rect: Rect) -> f32 {
        self.physical_length(self.logical_content_height(self.logical_rect(panel_rect)))
    }

    fn logical_content_height(&self, panel_rect: Rect) -> f32 {
        if self.page_only {
            return crate::widgets::property_panel_page::content_height();
        }
        sections::property_panel_content_height(
            panel_rect,
            self.visible_sections(),
            &self.snapshot.effects,
            &self.snapshot.fills,
            &self.snapshot.interactions,
        )
    }

    /// Section-visibility mask for the current selection, threaded
    /// into every layout walker so paint + hit-test stay aligned.
    pub(crate) fn visible_sections(&self) -> sections::VisibleSections {
        let caps = self.capabilities();
        let component_button = if self.snapshot.is_instance {
            crate::widgets::property_panel_visibility::ComponentButtonState::Instance {
                component_count: self.instance_component_options.len(),
                picker_open: self.instance_component_picker_open,
            }
        } else if self.snapshot.is_reusable {
            crate::widgets::property_panel_visibility::ComponentButtonState::DetachComponent
        } else {
            crate::widgets::property_panel_visibility::ComponentButtonState::Create
        };
        sections::VisibleSections {
            create_component: self.snapshot.is_instance
                || (caps.create_component && self.snapshot.can_create_component),
            component_button,
            flex_layout: caps.flex_layout,
            flex_layout_mode: self.snapshot.flex_layout,
            padding_edit_mode: self.padding_edit_mode,
            layout_justify: self.snapshot.layout_justify,
            layout_align: self.snapshot.layout_align,
            size_options: caps.size_options,
            touch_controls: self.density_scale > 1.0,
            size_fill_width: self.snapshot.size_fill_width,
            size_fill_height: self.snapshot.size_fill_height,
            size_hug_width: self.snapshot.size_hug_width,
            size_hug_height: self.snapshot.size_hug_height,
            clip_content: self.snapshot.can_clip_content,
            text: caps.text && self.snapshot.text.is_some(),
            icon: self.snapshot.icon.is_some(),
            widget: self.snapshot.widget.as_ref().map(|w| w.kind),
            widget_checked: self.snapshot.widget.as_ref().is_some_and(|w| w.checked),
            image: caps.image && self.snapshot.is_image_node,
            image_warning: caps.image
                && self
                    .image_panel_view
                    .as_ref()
                    .is_some_and(|v| v.warning.is_some()),
            opacity: caps.opacity,
            compositing: !self.is_multi,
            corner_radius: self.snapshot.has_corner_radius,
            corner_per_corner: self.snapshot.supports_per_corner,
            corner_expand: self.corner_expand_open,
            path_fill_rule: self.snapshot.path_fill_rule,
            polygon_sides: self.snapshot.polygon_sides.is_some(),
            ellipse_arc: self.snapshot.ellipse_arc.is_some(),
            fill: caps.fill,
            stroke: caps.stroke,
            stroke_edit_mode: self.stroke_edit_mode,
            stroke_mode_popover_open: self.stroke_mode_popover_open,
            color_variable_count: self.color_variable_count,
            fill_variable_bound: self.fill_variable_ref.is_some(),
            stroke_variable_bound: self.stroke_variable_ref.is_some(),
            effects: caps.effects,
            export: caps.export,
            fill_type: self.fill_type,
            gradient_stop_count: self.snapshot.gradient_stops.len(),
            interactions: caps.interactions,
        }
    }
}

/// L/R padding around a fit-content action-button hover wash (④) so the
/// highlight isn't flush against the checkbox/icon it hugs.
const ACTION_WASH_PAD_X: f32 = 6.0;

/// Shrink the hover/press wash for the Size checkboxes and the alignment
/// segmented buttons to hug their visible content (checkbox + label, or the
/// centred icon) plus a little L/R padding — instead of washing the full
/// half-width / full cell the walker rect spans. Every other action keeps its
/// walker rect. Only the painted highlight shrinks; the hit target (the walker
/// rect the host hovers + clicks) is unchanged.
pub(super) fn action_wash_rect(
    action: &PropertyPanelAction,
    r: Rect,
    labels: &sections::PropertyLabels,
    locale: op_editor_core::Locale,
    backend: &mut dyn crate::RenderBackend,
) -> Rect {
    // Layout-justify rows (space-between / space-around) paint a radio at
    // `r.origin.x` and a 10px label RADIO_GUTTER further right, but the
    // action rect spans the whole gap column. Hug the radio + label so the
    // hover wash reads as a fit-content pill instead of a full-width bar.
    if let PropertyPanelAction::SetLayoutJustify(v) = action {
        // RADIO_GUTTER = 6 + RADIO_SIZE(13) — see `property_panel_flex`.
        const RADIO_GUTTER: f32 = 19.0;
        let key = match v {
            LayoutJustifyValue::SpaceBetween => Some("layout.spaceBetween"),
            LayoutJustifyValue::SpaceAround => Some("layout.spaceAround"),
            // `Start` is the circle-only numeric row — its rect is already
            // just the radio gutter, so leave it untouched.
            _ => None,
        };
        if let Some(key) = key {
            let label = op_i18n::translate(locale, key);
            let content_right =
                r.origin.x + RADIO_GUTTER + text_metrics::measure_chrome(backend, label, 10.0);
            let left = r.origin.x - ACTION_WASH_PAD_X;
            let right = (content_right + ACTION_WASH_PAD_X).min(r.origin.x + r.size.x);
            return Rect {
                origin: Point2D::new(left, r.origin.y),
                size: Point2D::new((right - left).max(0.0), r.size.y),
            };
        }
    }
    let size_label = match action {
        PropertyPanelAction::ToggleSizeFillWidth => Some(labels.fill_width),
        PropertyPanelAction::ToggleSizeFillHeight => Some(labels.fill_height),
        PropertyPanelAction::ToggleSizeHugWidth => Some(labels.hug_width),
        PropertyPanelAction::ToggleSizeHugHeight => Some(labels.hug_height),
        PropertyPanelAction::ToggleSizeClipContent => Some(labels.clip_content),
        _ => None,
    };
    if let Some(label) = size_label {
        // `paint_check_row` paints a compact box at `r.origin.x` then the label
        // after its shared gutter — so the content runs from the
        // box's left edge to the label's right edge. The left padding spills
        // into the gutter / inter-column gap (both empty), but the right edge
        // is clamped to the cell so a long localized label can't wash over the
        // adjacent column.
        let touch_controls =
            r.size.y > crate::widgets::property_panel_inputs::SIZE_CHECK_ROW_HEIGHT;
        let label_x =
            crate::widgets::property_panel_inputs::size_check_label_offset(touch_controls);
        let cell_right = r.origin.x + r.size.x;
        let content_right =
            r.origin.x + label_x + text_metrics::measure_chrome(backend, label, 12.0);
        let left = r.origin.x - ACTION_WASH_PAD_X;
        let right = (content_right + ACTION_WASH_PAD_X).min(cell_right);
        return Rect {
            origin: Point2D::new(left, r.origin.y),
            size: Point2D::new((right - left).max(0.0), r.size.y),
        };
    }
    if matches!(
        action,
        PropertyPanelAction::SetTextAlign(_) | PropertyPanelAction::SetTextVerticalAlign(_)
    ) {
        // Icon-only segmented cell — the jian ToggleGroup centres a ~16px glyph
        // in the cell, so hug that glyph rather than the whole cell. Align cells
        // are adjacent (no gap), so clamp the pill within the cell so it can't
        // bleed into the neighbouring button.
        const ICON_W: f32 = 16.0;
        let center_x = r.origin.x + r.size.x / 2.0;
        let left = (center_x - ICON_W / 2.0 - ACTION_WASH_PAD_X).max(r.origin.x);
        let right = (center_x + ICON_W / 2.0 + ACTION_WASH_PAD_X).min(r.origin.x + r.size.x);
        return Rect {
            origin: Point2D::new(left, r.origin.y),
            size: Point2D::new((right - left).max(0.0), r.size.y),
        };
    }
    r
}
