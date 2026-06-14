//! Section paint helpers for [`crate::widgets::PropertyPanel`].
//! Split out of `property_panel.rs` to honor the 800-line file
//! ceiling. Each `paint_*_section` returns the y-coordinate just
//! below itself so the parent can chain them.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::NodeSnapshot;
use crate::widgets::property_panel_color_variables::paint_color_variable_button;
use crate::widgets::property_panel_inputs::{
    create_component_block_height, format_color_hex, paint_input_with_icon_focused_state,
    paint_input_with_prefix_focused_state, paint_section_divider, paint_section_label,
    paint_text_input_view_value, COLOR_VARIABLE_BUTTON_W, COLOR_VARIABLE_GAP, COMPONENT_ACCENT,
    CREATE_COMPONENT_BTN_H, CREATE_COMPONENT_ICON, CREATE_COMPONENT_PAD_TOP,
    CREATE_COMPONENT_ROW_GAP, HEADER_HEIGHT, INPUT_HEIGHT, INPUT_RADIUS, INSTANCE_ACCENT, PAD_X,
    SECTION_GAP, TAB_HEIGHT,
};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::PropertyFocus;

// Re-exports — fill paint moved to `property_panel_fill.rs`,
// layout walkers + visibility flags to `property_panel_layout.rs`.
pub use crate::widgets::property_panel_effects::paint_effects_section;
pub use crate::widgets::property_panel_fill::{
    fill_type_label, paint_fill_section, paint_fill_type_picker,
};
pub use crate::widgets::property_panel_image_fill::{
    image_fill_popover_action_at, image_fill_popover_action_rects,
    image_fill_popover_adjustment_action_for_drag, image_fill_popover_contains,
    paint_image_fill_popover,
};
pub use crate::widgets::property_panel_inputs::format_color_hex as _format_color_hex_compat;
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

/// Localised chrome strings for the PropertyPanel sections.
/// Resolved once at panel-construction time from `Document::t` so
/// every section's `paint_section_label` call gets the
/// locale-appropriate text without each helper hitting the
/// translation layer.
#[derive(Debug, Clone, Copy)]
pub struct PropertyLabels {
    pub tab_design: &'static str,
    pub tab_code: &'static str,
    pub create_component: &'static str,
    pub detach_component: &'static str,
    pub go_to_component: &'static str,
    pub detach_instance: &'static str,
    pub position: &'static str,
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
            tab_code: pick("rightPanel.code", "Code"),
            create_component: pick("property.createComponent", "Create Component"),
            detach_component: pick("property.detachComponent", "Detach Component"),
            go_to_component: pick("property.goToComponent", "Go to component"),
            detach_instance: pick("property.detachInstance", "Detach instance"),
            position: pick("size.position", "Position"),
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
    fill_body_height, property_panel_content_height, SizeFlags, VisibleSections,
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

/// The two tab rects (Design, Code) for the pinned strip at panel top-left
/// `(x, y)`. Single source of truth shared by paint + hit-test.
fn tab_strip_rects(labels: &PropertyLabels, x: f32, y: f32) -> (Rect, Rect) {
    let pad = 14.0;
    let tab_y = y + 6.0;
    let design_w = (tab_label_width(labels.tab_design) + 24.0).max(48.0);
    let design_rect = Rect {
        origin: Point2D::new(x + pad, tab_y),
        size: Point2D::new(design_w, 26.0),
    };
    let code_w = (tab_label_width(labels.tab_code) + 24.0).max(48.0);
    let code_rect = Rect {
        origin: Point2D::new(design_rect.origin.x + design_rect.size.x + 6.0, tab_y),
        size: Point2D::new(code_w, 26.0),
    };
    (design_rect, code_rect)
}

/// Hit-test the pinned Design / Code tab strip. `x`/`y` are the panel's
/// top-left (unscrolled — the strip is pinned). Returns the tab the point
/// lands on, or `None`. Geometry comes from [`tab_strip_rects`], the same
/// source `paint_tab_strip` uses, so clicks match the painted tabs.
pub fn tab_strip_hit(
    labels: &PropertyLabels,
    x: f32,
    y: f32,
    point: Point2D,
) -> Option<op_editor_core::PropertyTab> {
    use op_editor_core::PropertyTab;
    let (design_rect, code_rect) = tab_strip_rects(labels, x, y);
    let inside = |r: Rect| {
        point.x >= r.origin.x
            && point.x <= r.origin.x + r.size.x
            && point.y >= r.origin.y
            && point.y <= r.origin.y + r.size.y
    };
    if inside(design_rect) {
        Some(PropertyTab::Design)
    } else if inside(code_rect) {
        Some(PropertyTab::Code)
    } else {
        None
    }
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
    let (design_rect, code_rect) = tab_strip_rects(labels, x, y);
    let active = state.active;
    let hover = state.hover;
    if matches!(hover, Some(PropertyTab::Design)) && !matches!(active, PropertyTab::Design) {
        cx.backend.fill_round_rect(design_rect, 6.0, theme.muted);
    }
    if matches!(hover, Some(PropertyTab::Code)) && !matches!(active, PropertyTab::Code) {
        cx.backend.fill_round_rect(code_rect, 6.0, theme.muted);
    }
    if matches!(active, PropertyTab::Design) {
        cx.backend.fill_round_rect(design_rect, 6.0, theme.muted);
    } else {
        cx.backend.fill_round_rect(code_rect, 6.0, theme.muted);
    }
    let design_color = if matches!(active, PropertyTab::Design) {
        theme.foreground
    } else {
        theme.muted_foreground
    };
    let code_color = if matches!(active, PropertyTab::Code) {
        theme.foreground
    } else {
        theme.muted_foreground
    };
    let design_label = TextLayout::single_run(
        labels.tab_design,
        "system-ui",
        13.0,
        (design_color).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &design_label,
        Point2D::new(design_rect.origin.x + 12.0, design_rect.origin.y + 18.0),
    );
    let code_label = TextLayout::single_run(
        labels.tab_code,
        "system-ui",
        13.0,
        (code_color).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &code_label,
        Point2D::new(code_rect.origin.x + 12.0, code_rect.origin.y + 18.0),
    );
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

pub fn paint_create_component(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    labels: &PropertyLabels,
    state: crate::widgets::property_panel_visibility::ComponentButtonState,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    use crate::widgets::property_panel_visibility::ComponentButtonState as S;
    let first_row_y = y + CREATE_COMPONENT_PAD_TOP;
    match state {
        // Plain container: "Create component" (neutral).
        S::Create => paint_component_button(
            cx,
            theme,
            labels.create_component,
            Icon::Component,
            theme.foreground,
            x,
            first_row_y,
            width,
        ),
        // Reusable component: purple "Detach component" — TS paints
        // the same slot with the unlink affordance + purple accent.
        S::DetachComponent => paint_component_button(
            cx,
            theme,
            labels.detach_component,
            Icon::Diamond,
            COMPONENT_ACCENT,
            x,
            first_row_y,
            width,
        ),
        // Instance: the Go-to-component / Detach-instance row pair.
        S::Instance => {
            paint_component_button(
                cx,
                theme,
                labels.go_to_component,
                Icon::Component,
                INSTANCE_ACCENT,
                x,
                first_row_y,
                width,
            );
            paint_component_button(
                cx,
                theme,
                labels.detach_instance,
                Icon::Diamond,
                INSTANCE_ACCENT,
                x,
                first_row_y + CREATE_COMPONENT_BTN_H + CREATE_COMPONENT_ROW_GAP,
                width,
            );
        }
    }
    y + create_component_block_height(state)
}

/// One compact button row in the create-component block — shared by
/// all three [`ComponentButtonState`] variants so their geometry
/// matches the layout walker's rects exactly.
#[allow(clippy::too_many_arguments)]
fn paint_component_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    label_text: &str,
    icon_glyph: Icon,
    accent: crate::Color,
    x: f32,
    row_y: f32,
    width: f32,
) {
    let btn_h = CREATE_COMPONENT_BTN_H;
    let btn_rect = Rect {
        origin: Point2D::new(x + PAD_X, row_y),
        size: Point2D::new(width - PAD_X * 2.0, btn_h),
    };
    cx.backend.fill_round_rect(btn_rect, 8.0, theme.muted);
    cx.backend
        .stroke_round_rect(btn_rect, 8.0, theme.border, 1.0);
    let icon = CREATE_COMPONENT_ICON;
    draw_icon(
        cx.backend,
        icon_glyph,
        Point2D::new(
            btn_rect.origin.x + 12.0,
            btn_rect.origin.y + (btn_h - icon) / 2.0,
        ),
        icon,
        accent,
        1.3,
    );
    let label = TextLayout::single_run(
        label_text,
        "system-ui",
        13.0,
        (accent).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    let label_w = cx.backend.measure_text(label_text, 13.0);
    cx.backend.draw_text(
        &label,
        Point2D::new(
            btn_rect.origin.x + (btn_rect.size.x - label_w) / 2.0 + 12.0,
            btn_rect.origin.y + btn_h / 2.0 + 4.5,
        ),
    );
}

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
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
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
        let r_rect = Rect {
            origin: Point2D::new(x + PAD_X + half_w + 8.0, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        };
        let r_value = format!("{}", snapshot.corner_radius.round() as i32);
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
    y += INPUT_HEIGHT + 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

// ── Flex layout section ──────────────────────────────────────────

pub fn paint_flex_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    labels: &PropertyLabels,
    active: op_editor_core::FlexLayout,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, labels.flex_layout, x, y, width);
    // TS layout-section.tsx uses Columns3 / Rows3 / LayoutGrid for
    // the three flex modes; LayoutGrid is the default-active mode
    // (Free layout).
    let btn_w = 56.0;
    let gap = 8.0;
    let row_x = x + PAD_X;
    use op_editor_core::FlexLayout;
    let modes = [
        (FlexLayout::Free, Icon::LayoutGrid),
        (FlexLayout::Vertical, Icon::Rows3),
        (FlexLayout::Horizontal, Icon::Columns3),
    ];
    for (i, (mode, icon)) in modes.iter().enumerate() {
        let bx = row_x + i as f32 * (btn_w + gap);
        let rect = Rect {
            origin: Point2D::new(bx, y),
            size: Point2D::new(btn_w, 32.0),
        };
        let is_active = *mode == active;
        if is_active {
            cx.backend.fill_round_rect(rect, 6.0, theme.primary);
        } else {
            cx.backend.fill_round_rect(rect, 6.0, theme.muted);
            cx.backend.stroke_round_rect(rect, 6.0, theme.border, 1.0);
        }
        let icon_color = if is_active {
            theme.primary_foreground
        } else {
            theme.muted_foreground
        };
        draw_icon(
            cx.backend,
            *icon,
            Point2D::new(rect.origin.x + (btn_w - 18.0) / 2.0, rect.origin.y + 7.0),
            18.0,
            icon_color,
            1.4,
        );
        let _ = mode; // intentionally unused beyond the active-check
    }
    y += 32.0 + 12.0;
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
    // Hide the W/H box entirely when its dimension is fill/hug —
    // matching TS size-section.tsx, which renders the NumberInput only
    // when the dimension is a concrete number. Visible dimensions flow
    // left-to-right, so when W is hidden, H slides into the left slot
    // (no dangling empty half). The fixed `y += INPUT_HEIGHT + 10.0`
    // below keeps the row height (and every later section's offset)
    // unchanged regardless of how many boxes paint.
    let w_visible = !flags.fill_width && !flags.hug_width;
    let h_visible = !flags.fill_height && !flags.hug_height;
    if w_visible {
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
    }
    if h_visible {
        let h_value = snapshot.height.to_string();
        let h_target = if w_visible { h_rect } else { w_rect };
        paint_input_with_prefix_focused_state(
            cx,
            theme,
            h_target,
            "H",
            edit.value_for(PropertyFocus::SizeH, &h_value),
            edit.focus == Some(PropertyFocus::SizeH),
            edit.caret_at(PropertyFocus::SizeH),
            edit.select_all_at(PropertyFocus::SizeH),
            edit.input_at(PropertyFocus::SizeH),
            edit.now_ms,
        );
    }
    // Collapse the whole input row when BOTH dimensions are fill/hug —
    // the section shrinks up so the checkboxes sit under the label with
    // no dangling empty row. `size_input_row_h` keeps the layout
    // walkers in lockstep with this advance.
    if w_visible || h_visible {
        y += INPUT_HEIGHT + 10.0;
    }
    let row_h = 22.0;
    paint_check_row(
        cx,
        theme,
        x + PAD_X,
        y,
        half_w,
        labels.fill_width,
        flags.fill_width,
    );
    paint_check_row(
        cx,
        theme,
        x + PAD_X + half_w + 8.0,
        y,
        half_w,
        labels.fill_height,
        flags.fill_height,
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
    );
    paint_check_row(
        cx,
        theme,
        x + PAD_X + half_w + 8.0,
        y,
        half_w,
        labels.hug_height,
        flags.hug_height,
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
        );
        y += row_h;
    }
    y += 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

fn paint_check_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    y: f32,
    _w: f32,
    label: &str,
    checked: bool,
) {
    let box_rect = Rect {
        origin: Point2D::new(x, y + 3.0),
        size: Point2D::new(16.0, 16.0),
    };
    if checked {
        cx.backend.fill_round_rect(box_rect, 4.0, theme.primary);
        draw_icon(
            cx.backend,
            Icon::Check,
            Point2D::new(box_rect.origin.x + 1.0, box_rect.origin.y + 1.0),
            14.0,
            theme.primary_foreground,
            1.8,
        );
    } else {
        cx.backend
            .stroke_round_rect(box_rect, 4.0, theme.border, 1.0);
    }
    let lbl = TextLayout::single_run(
        label,
        "system-ui",
        12.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&lbl, Point2D::new(x + 22.0, y + 16.0));
}

// ── Stroke section ────────────────────────────────────────────────

// Paint-context + geometry args threaded through; a struct adds no gain.
#[allow(clippy::too_many_arguments)]
pub fn paint_stroke_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    labels: &PropertyLabels,
    stroke_variable_ref: Option<&str>,
    show_variable_button: bool,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, labels.stroke, x, y, width);
    let usable_w = width - PAD_X * 2.0;
    let stroke_color = snapshot.stroke_swatch_color();
    let stroke_width = snapshot.stroke.map(|s| s.width).unwrap_or(0.0);
    let width_w = 60.0;
    let variable_w = if show_variable_button {
        COLOR_VARIABLE_BUTTON_W + COLOR_VARIABLE_GAP
    } else {
        0.0
    };
    let hex_rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(usable_w - width_w - 8.0 - variable_w, INPUT_HEIGHT),
    };
    let hex_focused = edit.focus == Some(PropertyFocus::StrokeHex);
    cx.backend
        .fill_round_rect(hex_rect, INPUT_RADIUS, theme.muted);
    if hex_focused && stroke_variable_ref.is_none() {
        cx.backend
            .stroke_round_rect(hex_rect, INPUT_RADIUS, theme.primary, 1.5);
    }
    cx.backend.fill_round_rect(
        Rect {
            // Vertically centre the 16-tall swatch in the 30-tall
            // row: `(30 - 16) / 2 == 7`.
            origin: Point2D::new(hex_rect.origin.x + 6.0, hex_rect.origin.y + 7.0),
            size: Point2D::new(16.0, 16.0),
        },
        3.0,
        stroke_color,
    );
    let hex_owned = format_color_hex(stroke_color);
    let variable_text = stroke_variable_ref.map(|name| format!("${name}"));
    let hex_text = variable_text
        .as_deref()
        .unwrap_or_else(|| edit.value_for(PropertyFocus::StrokeHex, &hex_owned));
    let hex_x = hex_rect.origin.x + 30.0;
    let painted_hex = stroke_variable_ref.is_none()
        && edit.paint_input_view_at(
            cx,
            theme,
            PropertyFocus::StrokeHex,
            Rect {
                origin: Point2D::new(hex_x, hex_rect.origin.y),
                size: Point2D::new(
                    (hex_rect.origin.x + hex_rect.size.x - 8.0 - hex_x).max(0.0),
                    hex_rect.size.y,
                ),
            },
            12.0,
            0.0,
            hex_rect.origin.y + 19.0,
        );
    if !painted_hex {
        let hex_layout = TextLayout::single_run(
            hex_text,
            "system-ui",
            12.0,
            (theme.foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        if stroke_variable_ref.is_none() {
            edit.paint_selection_at(
                cx,
                theme,
                PropertyFocus::StrokeHex,
                hex_text,
                hex_x,
                hex_rect.origin.y + 19.0,
                12.0,
                hex_rect.origin.x + hex_rect.size.x - 8.0,
            );
        }
        cx.backend
            .draw_text(&hex_layout, Point2D::new(hex_x, hex_rect.origin.y + 19.0));
        if stroke_variable_ref.is_none() {
            if let Some(pos) = edit.caret_at(PropertyFocus::StrokeHex) {
                let w = cx
                    .backend
                    .measure_text(&hex_text[..pos.min(hex_text.len())], 12.0);
                cx.backend.fill_rect(
                    Rect {
                        origin: Point2D::new(hex_x + w, hex_rect.origin.y + 6.0),
                        size: Point2D::new(1.5, hex_rect.size.y - 12.0),
                    },
                    theme.foreground,
                );
            }
        }
    }
    if show_variable_button {
        paint_color_variable_button(
            cx,
            theme,
            Rect {
                origin: Point2D::new(hex_rect.origin.x + hex_rect.size.x + COLOR_VARIABLE_GAP, y),
                size: Point2D::new(COLOR_VARIABLE_BUTTON_W, INPUT_HEIGHT),
            },
            stroke_variable_ref.is_some(),
        );
    }
    let w_rect = Rect {
        origin: Point2D::new(hex_rect.origin.x + hex_rect.size.x + variable_w + 8.0, y),
        size: Point2D::new(width_w, INPUT_HEIGHT),
    };
    let w_focused = edit.focus == Some(PropertyFocus::StrokeWidth);
    cx.backend
        .fill_round_rect(w_rect, INPUT_RADIUS, theme.muted);
    if w_focused {
        cx.backend
            .stroke_round_rect(w_rect, INPUT_RADIUS, theme.primary, 1.5);
    }
    let w_owned = format!("{}", stroke_width.round() as i32);
    let w_text = edit.value_for(PropertyFocus::StrokeWidth, &w_owned);
    let w_x = w_rect.origin.x + 12.0;
    if !edit.paint_input_view_at(
        cx,
        theme,
        PropertyFocus::StrokeWidth,
        Rect {
            origin: Point2D::new(w_x, w_rect.origin.y),
            size: Point2D::new(
                (w_rect.origin.x + w_rect.size.x - 8.0 - w_x).max(0.0),
                w_rect.size.y,
            ),
        },
        12.0,
        0.0,
        w_rect.origin.y + 19.0,
    ) {
        let w_layout = TextLayout::single_run(
            w_text,
            "system-ui",
            12.0,
            (theme.foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        edit.paint_selection_at(
            cx,
            theme,
            PropertyFocus::StrokeWidth,
            w_text,
            w_x,
            w_rect.origin.y + 19.0,
            12.0,
            w_rect.origin.x + w_rect.size.x - 8.0,
        );
        cx.backend
            .draw_text(&w_layout, Point2D::new(w_x, w_rect.origin.y + 19.0));
        if let Some(pos) = edit.caret_at(PropertyFocus::StrokeWidth) {
            let w = cx
                .backend
                .measure_text(&w_text[..pos.min(w_text.len())], 12.0);
            cx.backend.fill_rect(
                Rect {
                    origin: Point2D::new(w_x + w, w_rect.origin.y + 6.0),
                    size: Point2D::new(1.5, w_rect.size.y - 12.0),
                },
                theme.foreground,
            );
        }
    }
    y += INPUT_HEIGHT + 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
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
