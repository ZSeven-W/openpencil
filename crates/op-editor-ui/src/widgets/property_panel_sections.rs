//! Section paint helpers for [`crate::widgets::PropertyPanel`].
//! Split out of `property_panel.rs` to honor the 800-line file
//! ceiling. Each `paint_*_section` returns the y-coordinate just
//! below itself so the parent can chain them.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::NodeSnapshot;
use crate::widgets::property_panel_inputs::{
    paint_input_with_icon_focused_state, paint_input_with_prefix_focused_state,
    paint_section_divider, paint_section_label, paint_text_input_view_value, COMPONENT_ACCENT,
    HEADER_HEIGHT, INPUT_HEIGHT, INSTANCE_ACCENT, PAD_X, SECTION_GAP, TAB_HEIGHT,
};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::PropertyFocus;

pub use crate::widgets::property_panel_effects::paint_effects_section;
pub use crate::widgets::property_panel_fill::{
    fill_type_label, paint_fill_section, paint_fill_type_picker,
};
pub use crate::widgets::property_panel_image_fill::{
    image_fill_popover_action_at, image_fill_popover_action_rects,
    image_fill_popover_adjustment_action_for_drag, image_fill_popover_contains,
    image_fill_popover_input_at, paint_image_fill_popover,
};
pub use crate::widgets::property_panel_inputs::format_color_hex as _format_color_hex_compat;
pub use crate::widgets::property_panel_interactions::paint_interactions_section;
pub use crate::widgets::property_panel_layer::paint_layer_section;

/// Hit-result for a click on the property panel — payload is the
/// input the click landed on (host stores in
/// `Document.ui.property_focus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyPanelHit {
    Input(PropertyFocus),
}

/// State plumbed from `PropertyPanel` down into the per-section
/// paint helpers so the focused input can render its primary
/// border + caret + draft text. Unused for non-editable rows.
pub struct EditContext<'a> {
    pub focus: Option<PropertyFocus>,
    pub draft: &'a str,
    pub input: &'a jian_core::text_input::TextInputState,
    /// Caret byte-index into `draft` (drafts are ASCII).
    pub caret: usize,
    /// Whether Ctrl/Cmd+A selected the focused draft.
    pub select_all: bool,
    pub now_ms: u64,
}

/// Resolved once at panel-construction time from `Document::t` so
/// every section's `paint_section_label` call gets the
/// locale-appropriate text without each helper hitting the
/// translation layer.
#[derive(Debug, Clone, Copy)]
pub struct PropertyLabels {
    pub tab_design: &'static str,
    pub tab_interact: &'static str,
    pub tab_code: &'static str,
    pub create_component: &'static str,
    pub detach_component: &'static str,
    pub go_to_component: &'static str,
    pub detach_instance: &'static str,
    pub swap_component: &'static str,
    pub position: &'static str,
    pub corner_per_corner: &'static str,
    pub mixed: &'static str,
    pub flex_layout: &'static str,
    pub size: &'static str,
    pub layer: &'static str,
    pub opacity: &'static str,
    pub polygon_sides: &'static str,
    pub ellipse_start: &'static str,
    pub ellipse_sweep: &'static str,
    pub ellipse_inner_radius: &'static str,
    pub fill: &'static str,
    pub stroke: &'static str,
    pub effects: &'static str,
    pub fill_rule_nonzero: &'static str,
    pub fill_rule_evenodd: &'static str,
    pub effects_add_shadow: &'static str,
    pub effects_add_layer_blur: &'static str,
    pub effects_add_background_blur: &'static str,
    pub export: &'static str,
    pub fill_width: &'static str,
    pub fill_height: &'static str,
    pub hug_width: &'static str,
    pub hug_height: &'static str,
    pub clip_content: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct TabStripState {
    pub active: op_editor_core::PropertyTab,
    pub hover: Option<op_editor_core::PropertyTab>,
    pub show_interact: bool,
    pub touch_controls: bool,
}

impl PropertyLabels {
    /// Resolve every PropertyPanel chrome string against the editor's
    /// active locale (`EditorUiState.locale`).
    pub fn for_editor_ui(ui: &op_editor_core::editor_ui_state::EditorUiState) -> Self {
        // `pick` returns either the localised value or the English
        // fallback when the key isn't in the table. Both branches
        // are `&'static str` so the whole struct is `Copy` and
        // zero-allocation per build.
        let pick = |key: &'static str, fallback: &'static str| -> &'static str {
            let translated = crate::widgets::editor_state_ext::translate(ui, key);
            if translated == key {
                fallback
            } else {
                translated
            }
        };
        Self {
            tab_design: pick("rightPanel.design", "Design"),
            tab_interact: pick("rightPanel.interact", "Interact"),
            tab_code: pick("rightPanel.code", "Code"),
            create_component: pick("property.createComponent", "Create Component"),
            detach_component: pick("property.detachComponent", "Detach Component"),
            go_to_component: pick("property.goToComponent", "Go to component"),
            detach_instance: pick("property.detachInstance", "Detach instance"),
            swap_component: pick("property.swapComponent", "Swap"),
            position: pick("size.position", "Position"),
            corner_per_corner: pick("property.cornerPerCorner", "Edit corners independently"),
            mixed: pick("property.mixed", "Mixed"),
            flex_layout: pick("layout.flexLayout", "Flex Layout"),
            size: pick("layout.dimensions", "Size"),
            layer: pick("appearance.layer", "Layer"),
            opacity: pick("appearance.opacity", "Opacity"),
            polygon_sides: pick("polygon.sides", "Sides"),
            ellipse_start: pick("ellipse.start", "Start"),
            ellipse_sweep: pick("ellipse.sweep", "Sweep"),
            ellipse_inner_radius: pick("ellipse.innerRadius", "Inner"),
            fill: pick("fill.title", "Fill"),
            stroke: pick("stroke.title", "Stroke"),
            effects: pick("effects.title", "Effects"),
            fill_rule_nonzero: pick("fill.ruleNonzero", "Nonzero"),
            fill_rule_evenodd: pick("fill.ruleEvenodd", "Even-odd"),
            effects_add_shadow: pick("effects.addShadow", "Shadow"),
            effects_add_layer_blur: pick("effects.addLayerBlur", "Layer blur"),
            effects_add_background_blur: pick("effects.addBackgroundBlur", "Background blur"),
            export: pick("export.title", "Export"),
            fill_width: pick("layout.fillWidth", "Fill Width"),
            fill_height: pick("layout.fillHeight", "Fill Height"),
            hug_width: pick("layout.hugWidth", "Hug Width"),
            hug_height: pick("layout.hugHeight", "Hug Height"),
            clip_content: pick("layout.clipContent", "Clip Content"),
        }
    }
}

impl<'a> EditContext<'a> {
    /// Convenience for paint helpers — returns the value to render
    /// for the given field: the live edit draft when this field is
    /// focused, or the snapshot fallback otherwise.
    pub fn value_for<'b>(&'b self, focus: PropertyFocus, fallback: &'b str) -> &'b str {
        if self.focus == Some(focus) {
            self.draft
        } else {
            fallback
        }
    }

    /// Whether the caret blink is currently in its visible phase —
    /// for editable surfaces (effect params) that don't key off a
    /// `PropertyFocus`.
    pub fn caret_blink_on(&self) -> bool {
        self.input.caret_visible(self.now_ms)
    }

    /// Caret byte-offset for `focus` when it is the focused field
    /// and the blink is on — `None` otherwise. Drives caret paint;
    /// the offset is clamped into the draft so a stale value is safe.
    pub fn caret_at(&self, focus: PropertyFocus) -> Option<usize> {
        if self.focus == Some(focus) && self.input.caret_visible(self.now_ms) {
            Some(self.input.caret().min(self.draft.len()))
        } else {
            None
        }
    }

    pub fn select_all_at(&self, focus: PropertyFocus) -> bool {
        self.focus == Some(focus) && self.select_all && !self.draft.is_empty()
    }

    pub fn input_at(&self, focus: PropertyFocus) -> Option<&jian_core::text_input::TextInputState> {
        (self.focus == Some(focus)).then_some(self.input)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn paint_input_view_at(
        &self,
        cx: &mut PaintCx<'_>,
        theme: &Theme,
        focus: PropertyFocus,
        rect: Rect,
        font_size: f32,
        pad_x: f32,
        baseline_y: f32,
    ) -> bool {
        let Some(input) = self.input_at(focus) else {
            return false;
        };
        paint_text_input_view_value(
            cx,
            theme,
            input,
            rect,
            font_size,
            pad_x,
            baseline_y,
            self.now_ms,
        );
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub fn paint_selection_at(
        &self,
        cx: &mut PaintCx<'_>,
        theme: &Theme,
        focus: PropertyFocus,
        value: &str,
        text_x: f32,
        baseline_y: f32,
        font_size: f32,
        max_x: f32,
    ) {
        if self.select_all_at(focus) {
            crate::widgets::text_selection::paint_single_line_selection(
                cx, theme, value, text_x, baseline_y, font_size, max_x,
            );
        }
    }
}

// Layout constants + shared paint helpers live in
// `property_panel_inputs.rs` and are imported via the pub-use
// block above.

/// Re-exports for back-compat — the layout walkers + the
/// `VisibleSections` / `SizeFlags` state live in
/// `property_panel_layout.rs` now.
pub use crate::widgets::property_panel_layout::{
    action_button_rects, action_button_rects_with_fill_picker, editable_input_rects,
    fill_body_height, fill_type_toggle_action_rect, property_panel_content_height, SizeFlags,
    VisibleSections,
};

// ── Tab strip ─────────────────────────────────────────────────────

/// Backend-free estimate of `measure_text(label, 13.0)` for the tab strip,
/// so `paint_tab_strip` and `tab_strip_hit` derive identical geometry and a
/// click always lands on what's drawn (CJK-aware: ASCII ~0.55em, full-width
/// glyphs ~1em at 13 px).
fn tab_label_width(label: &str) -> f32 {
    label
        .chars()
        .map(|c| if c.is_ascii() { 7.0 } else { 13.0 })
        .sum()
}

/// The tab rects (Design, [Interact when `show_interact`], Code) for
/// the pinned strip at panel top-left `(x, y)`, in paint order.
/// Single source of truth shared by paint + hit-test — a click always
/// lands on what's drawn because both walk this same vec.
pub fn tab_strip_rects(
    labels: &PropertyLabels,
    x: f32,
    y: f32,
    show_interact: bool,
    touch_controls: bool,
) -> Vec<(op_editor_core::PropertyTab, Rect)> {
    use op_editor_core::PropertyTab;
    let pad = 14.0;
    let tab_height = if touch_controls { 30.0 } else { 26.0 };
    let tab_y = y + (TAB_HEIGHT - tab_height) / 2.0;
    let mut cursor_x = x + pad;
    let mut rects = Vec::with_capacity(3);
    let design_w = (tab_label_width(labels.tab_design) + 24.0).max(48.0);
    rects.push((
        PropertyTab::Design,
        Rect {
            origin: Point2D::new(cursor_x, tab_y),
            size: Point2D::new(design_w, tab_height),
        },
    ));
    cursor_x += design_w + 6.0;
    if show_interact {
        let interact_w = (tab_label_width(labels.tab_interact) + 24.0).max(48.0);
        rects.push((
            PropertyTab::Interact,
            Rect {
                origin: Point2D::new(cursor_x, tab_y),
                size: Point2D::new(interact_w, tab_height),
            },
        ));
        cursor_x += interact_w + 6.0;
    }
    let code_w = (tab_label_width(labels.tab_code) + 24.0).max(48.0);
    rects.push((
        PropertyTab::Code,
        Rect {
            origin: Point2D::new(cursor_x, tab_y),
            size: Point2D::new(code_w, tab_height),
        },
    ));
    rects
}

/// Hit-test the pinned tab strip. `x`/`y` are the panel's top-left
/// (unscrolled — the strip is pinned). Returns the tab the point
/// lands on, or `None`. Geometry comes from [`tab_strip_rects`], the
/// same source `paint_tab_strip` uses, so clicks match the painted
/// tabs.
pub fn tab_strip_hit(
    labels: &PropertyLabels,
    x: f32,
    y: f32,
    point: Point2D,
    show_interact: bool,
    touch_controls: bool,
) -> Option<op_editor_core::PropertyTab> {
    tab_strip_rects(labels, x, y, show_interact, touch_controls)
        .into_iter()
        .find(|(_, rect)| rect.contains(point))
        .map(|(tab, _)| tab)
}

pub fn paint_tab_strip(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    labels: &PropertyLabels,
    state: TabStripState,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    use op_editor_core::PropertyTab;
    let active = state.active;
    let hover = state.hover;
    let label_for = |tab: PropertyTab| -> &'static str {
        match tab {
            PropertyTab::Design => labels.tab_design,
            PropertyTab::Interact => labels.tab_interact,
            PropertyTab::Code => labels.tab_code,
        }
    };
    for (tab, rect) in tab_strip_rects(labels, x, y, state.show_interact, state.touch_controls) {
        let is_active = tab == active;
        let is_hovered = hover == Some(tab) && !is_active;
        if is_active || is_hovered {
            cx.backend.fill_round_rect(rect, 6.0, theme.muted);
        }
        let color = if is_active {
            theme.foreground
        } else {
            theme.muted_foreground
        };
        let label = TextLayout::single_run(
            label_for(tab),
            "system-ui",
            13.0,
            (color).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &label,
            Point2D::new(
                rect.origin.x + 12.0,
                jian_widgets::centered_text_baseline_y(rect, 13.0),
            ),
        );
    }
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(x, y + TAB_HEIGHT - 1.0),
            size: Point2D::new(width, 1.0),
        },
        theme.border,
    );
    y + TAB_HEIGHT
}

// ── Header row ────────────────────────────────────────────────────

pub fn paint_node_header(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    x: f32,
    y: f32,
    _w: f32,
) -> f32 {
    // Show the node's name so the header matches the LayerPanel
    // row; fall back to the kind label for an unnamed node.
    let title = if snapshot.name.is_empty() {
        snapshot.kind.as_str()
    } else {
        snapshot.name.as_str()
    };
    // Component / instance badging — TS tints the header purple and
    // prefixes a Diamond glyph (property-panel.tsx:182-187).
    let (title_color, badge) = if snapshot.is_reusable {
        (COMPONENT_ACCENT, true)
    } else if snapshot.is_instance {
        (INSTANCE_ACCENT, true)
    } else {
        (theme.foreground, false)
    };
    let mut text_x = x + PAD_X;
    if badge {
        draw_icon(
            cx.backend,
            Icon::Diamond,
            Point2D::new(text_x, y + 11.0),
            12.0,
            title_color,
            1.4,
        );
        text_x += 18.0;
    }
    let label = TextLayout::single_run(
        title,
        "system-ui",
        14.0,
        (title_color).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&label, Point2D::new(text_x, y + 22.0));
    y + HEADER_HEIGHT
}

// ── Create-component button ───────────────────────────────────────
// ── Position section ──────────────────────────────────────────────

// Paint-context + geometry args threaded through; a struct adds no gain.
#[allow(clippy::too_many_arguments)]
pub fn paint_position_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    labels: &PropertyLabels,
    show_radius: bool,
    corner_expanded: bool,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let per_corner_expanded = corner_expanded && snapshot.supports_per_corner;
    let mut y = paint_section_label(cx, theme, labels.position, x, y, width);
    let usable_w = width - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;
    let x_rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    let y_rect = Rect {
        origin: Point2D::new(x + PAD_X + half_w + 8.0, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    let x_value = snapshot.x.to_string();
    paint_input_with_prefix_focused_state(
        cx,
        theme,
        x_rect,
        "X",
        edit.value_for(PropertyFocus::PositionX, &x_value),
        edit.focus == Some(PropertyFocus::PositionX),
        edit.caret_at(PropertyFocus::PositionX),
        edit.select_all_at(PropertyFocus::PositionX),
        edit.input_at(PropertyFocus::PositionX),
        edit.now_ms,
    );
    let y_value = snapshot.y.to_string();
    paint_input_with_prefix_focused_state(
        cx,
        theme,
        y_rect,
        "Y",
        edit.value_for(PropertyFocus::PositionY, &y_value),
        edit.focus == Some(PropertyFocus::PositionY),
        edit.caret_at(PropertyFocus::PositionY),
        edit.select_all_at(PropertyFocus::PositionY),
        edit.input_at(PropertyFocus::PositionY),
        edit.now_ms,
    );
    y += INPUT_HEIGHT + 6.0;
    // Rotation input — TS uses RotateCw glyph; we render with an
    // icon-prefixed pill that accepts editable degree values.
    let rotation_rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    let rotation_value = format!("{}", snapshot.rotation_deg.round() as i32);
    paint_input_with_icon_focused_state(
        cx,
        theme,
        rotation_rect,
        Icon::RotateCw,
        edit.value_for(PropertyFocus::Rotation, &rotation_value),
        Some("°"),
        edit.focus == Some(PropertyFocus::Rotation),
        edit.caret_at(PropertyFocus::Rotation),
        edit.select_all_at(PropertyFocus::Rotation),
        edit.input_at(PropertyFocus::Rotation),
        edit.now_ms,
    );
    if show_radius {
        // Corner radius (R) — editable input bound to Node::corner_radius
        // via PropertyFocus::PositionR.
        let (r_rect, toggle_rect) = if snapshot.supports_per_corner {
            crate::widgets::property_panel_corner::uniform_and_toggle_rects(x, y, width)
        } else {
            (
                Rect {
                    origin: Point2D::new(x + PAD_X + half_w + 8.0, y),
                    size: Point2D::new(half_w, INPUT_HEIGHT),
                },
                Rect::xywh(0.0, 0.0, 0.0, 0.0),
            )
        };
        if !per_corner_expanded {
            let mixed =
                !crate::widgets::property_panel_corner::radii_are_uniform(snapshot.corner_radii);
            let r_value = if mixed {
                labels.mixed.to_string()
            } else {
                format!("{}", snapshot.corner_radius.round() as i32)
            };
            paint_input_with_prefix_focused_state(
                cx,
                theme,
                r_rect,
                "R",
                edit.value_for(PropertyFocus::PositionR, &r_value),
                edit.focus == Some(PropertyFocus::PositionR),
                edit.caret_at(PropertyFocus::PositionR),
                edit.select_all_at(PropertyFocus::PositionR),
                edit.input_at(PropertyFocus::PositionR),
                edit.now_ms,
            );
        }
        if snapshot.supports_per_corner {
            crate::widgets::property_panel_corner::paint_toggle(
                cx,
                theme,
                toggle_rect,
                corner_expanded,
            );
        }
    }
    y += INPUT_HEIGHT;
    if show_radius && per_corner_expanded {
        crate::widgets::property_panel_corner::paint_grid(
            cx,
            theme,
            edit,
            snapshot.corner_radii,
            x,
            y,
            width,
        );
        y += crate::widgets::property_panel_corner::CORNER_GRID_EXTRA_HEIGHT;
    }
    y += 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

// ── Size section ──────────────────────────────────────────────────

// Paint-context + geometry args threaded through; a struct adds no gain.
#[allow(clippy::too_many_arguments)]
pub fn paint_size_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    labels: &PropertyLabels,
    flags: SizeFlags,
    show_clip_content: bool,
    touch_controls: bool,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, labels.size, x, y, width);
    let usable_w = width - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;
    let w_rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    let h_rect = Rect {
        origin: Point2D::new(x + PAD_X + half_w + 8.0, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    // Sizing mode and the numeric editor are complementary. Fill / Hug
    // stays selected below while this row exposes the resolved snapshot
    // size; committing a number replaces that axis with fixed sizing.
    let w_value = snapshot.width.to_string();
    paint_input_with_prefix_focused_state(
        cx,
        theme,
        w_rect,
        "W",
        edit.value_for(PropertyFocus::SizeW, &w_value),
        edit.focus == Some(PropertyFocus::SizeW),
        edit.caret_at(PropertyFocus::SizeW),
        edit.select_all_at(PropertyFocus::SizeW),
        edit.input_at(PropertyFocus::SizeW),
        edit.now_ms,
    );
    let h_value = snapshot.height.to_string();
    paint_input_with_prefix_focused_state(
        cx,
        theme,
        h_rect,
        "H",
        edit.value_for(PropertyFocus::SizeH, &h_value),
        edit.focus == Some(PropertyFocus::SizeH),
        edit.caret_at(PropertyFocus::SizeH),
        edit.select_all_at(PropertyFocus::SizeH),
        edit.input_at(PropertyFocus::SizeH),
        edit.now_ms,
    );
    y += INPUT_HEIGHT + 10.0;
    let row_h = crate::widgets::property_panel_inputs::size_check_row_height(touch_controls);
    paint_check_row(
        cx,
        theme,
        x + PAD_X,
        y,
        half_w,
        labels.fill_width,
        flags.fill_width,
        row_h,
    );
    paint_check_row(
        cx,
        theme,
        x + PAD_X + half_w + 8.0,
        y,
        half_w,
        labels.fill_height,
        flags.fill_height,
        row_h,
    );
    y += row_h;
    paint_check_row(
        cx,
        theme,
        x + PAD_X,
        y,
        half_w,
        labels.hug_width,
        flags.hug_width,
        row_h,
    );
    paint_check_row(
        cx,
        theme,
        x + PAD_X + half_w + 8.0,
        y,
        half_w,
        labels.hug_height,
        flags.hug_height,
        row_h,
    );
    y += row_h;
    if show_clip_content {
        paint_check_row(
            cx,
            theme,
            x + PAD_X,
            y,
            usable_w,
            labels.clip_content,
            flags.clip_content,
            row_h,
        );
        y += row_h;
    }
    y += 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

/// Checkbox + label. `w` is the half-column the pair occupies; the label is
/// fitted to what is left of it after the 16px box and its gutter, because a
/// long localized label ("Remplir la hauteur", "高さに合わせる") otherwise
/// runs straight over the neighbouring column and off the rail's right edge.
#[allow(clippy::too_many_arguments)]
fn paint_check_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    checked: bool,
    row_h: f32,
) {
    let touch_controls = row_h > crate::widgets::property_panel_inputs::SIZE_CHECK_ROW_HEIGHT;
    let box_size = crate::widgets::property_panel_inputs::size_check_box_size(touch_controls);
    let label_x = crate::widgets::property_panel_inputs::size_check_label_offset(touch_controls);
    let box_rect = Rect {
        origin: Point2D::new(x, y + (row_h - box_size) / 2.0),
        size: Point2D::new(box_size, box_size),
    };
    jian_widgets::components::checkbox::Checkbox {
        checked,
        enabled: true,
    }
    .paint(
        cx.backend,
        box_rect,
        &crate::widgets::button::tokens_from_theme(theme),
    );
    let label =
        crate::widgets::text_metrics::fit_chrome(cx.backend, label, (w - label_x).max(0.0), 12.0);
    let lbl = TextLayout::single_run(
        &label,
        "system-ui",
        12.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&lbl, Point2D::new(x + label_x, y + row_h / 2.0 + 5.0));
}

// ── Effects section ───────────────────────────────────────────────
// `paint_effects_section` + its row helpers live in
// `property_panel_effects.rs` (split out at the 800-line cap);
// re-exported below so callers keep using `sections::*`.

// ── Export section ────────────────────────────────────────────────
// Paint code lives in `property_panel_export.rs` (split out to keep
// this file under the 800-line ceiling); re-exported so callers
// keep using `sections::*`.
pub use crate::widgets::property_panel_export::{
    export_scale_label, paint_export_picker, paint_export_section,
};

// All shared paint primitives + layout constants are imported
// from `property_panel_inputs` via the `pub use` block earlier in
// this file. Keep section-paint helpers here.
