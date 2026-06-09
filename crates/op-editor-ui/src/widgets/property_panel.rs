//! `PropertyPanel` — right-rail node inspector (Step 6).
//!
//! Mirrors `apps/web/src/components/panels/right-panel.tsx` and the
//! per-section TS files (`*-section.tsx`). The bulk of the paint
//! logic lives in [`super::property_panel_sections`] — this file
//! holds the `PropertyPanel` struct, the `Widget` impl, and wiring
//! around snapshot extraction. Splitting the file keeps the pieces
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
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::property_panel_sections as sections;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect};
use op_editor_core::PropertyFocus;

use op_editor_core::EditorState;

pub const PROPERTY_PANEL_WIDTH: f32 = 280.0;

// `PropertyPanelAction` lives in `property_panel_action.rs` (split
// out for the 800-line ceiling); re-exported so every existing
// `widgets::PropertyPanelAction` / `property_panel::PropertyPanelAction`
// path is unchanged.
pub use crate::widgets::property_panel_action::{
    FontFamilyChoice, FontWeightChoice, LayoutAlignValue, LayoutJustifyValue, PropertyPanelAction,
    TextAlignValue, TextGrowthValue, TextVerticalAlignValue,
};

// `SectionCapabilities` lives in `property_panel_layout.rs`
// alongside `VisibleSections` (the section-visibility mask it
// feeds); re-exported so `property_panel::SectionCapabilities`
// resolves unchanged.
pub(crate) use crate::widgets::property_panel_layout::SectionCapabilities;
pub use crate::widgets::property_panel_snapshot::{
    EffectKind, EffectSummary, EllipseArcSummary, GradientStopSummary, NodeSnapshot,
};

pub struct PropertyPanel {
    pub id: WidgetId,
    pub snapshot: NodeSnapshot,
    pub theme: Theme,
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
    /// Caret byte-offset into `draft` (ASCII drafts → char index).
    pub caret_pos: usize,
    /// Whether Ctrl/Cmd+A selected the full focused draft.
    pub select_all: bool,
    /// Caret-blink anchor (ms since host start) for the focused
    /// input. Drives the same `jian_core::anim::blink_visible`
    /// helper the chat caret uses.
    pub caret_anchor_ms: u64,
    /// Host clock ms; paired with `caret_anchor_ms` for caret blink.
    pub now_ms: u64,
    /// Active flex-layout button.
    pub flex_layout: op_editor_core::FlexLayout,
    /// 5 size checkboxes — fill / hug / clip.
    pub size_flags: sections::SizeFlags,
    /// Active fill type — drives the dropdown label + picker.
    pub fill_type: op_editor_core::FillType,
    pub fill_type_picker_open: bool,
    pub image_fill_popover_open: bool,
    pub font_family_picker_open: bool,
    pub font_weight_picker_open: bool,
    /// Hovered weight-dropdown row index (when the dropdown is open).
    pub font_weight_picker_hover: Option<usize>,
    /// Resolved padding edit mode (UI pin or derived from the node's
    /// values) + whether the gear popover is open.
    pub padding_edit_mode: op_editor_core::PaddingEditMode,
    pub padding_mode_popover_open: bool,
    /// Hovered padding-mode popover row index (gear popover open).
    pub padding_mode_popover_hover: Option<usize>,
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
    /// Index into `action_button_rects_with_fill_picker` of the action
    /// button the cursor is over — drives its `theme.button_hover` wash.
    pub action_hover: Option<usize>,
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
    /// Conditional builder — returns `Some` only when the editor
    /// has an active selection. Mirrors TS `{hasSelection && ...}`.
    pub fn for_selection(state: &EditorState) -> Option<Self> {
        Self::for_selection_at(state, 0)
    }

    /// Same as [`for_selection`] but threads the host's monotonic
    /// millisecond clock through so the focused-input caret can
    /// blink off the same animation timer as the chat input.
    pub fn for_selection_at(state: &EditorState, now_ms: u64) -> Option<Self> {
        if state.selection_count() == 1 {
            let node = state.selected_node()?;
            let fill_type = op_editor_core::first_fill_type(node);
            return Some(Self::build_from_snapshot(
                state,
                NodeSnapshot::from_node(node),
                fill_type,
                now_ms,
                false,
            ));
        }
        if state.selection_count() >= 2 {
            let snapshot = NodeSnapshot::from_multi_selection(state)?;
            return Some(Self::build_from_snapshot(
                state,
                snapshot,
                op_editor_core::FillType::Solid,
                now_ms,
                true,
            ));
        }
        None
    }

    fn build_from_snapshot(
        state: &EditorState,
        snapshot: NodeSnapshot,
        fill_type: op_editor_core::FillType,
        now_ms: u64,
        is_multi: bool,
    ) -> Self {
        let ui = &state.editor_ui;
        let flex_layout = snapshot.flex_layout;
        let size_flags = sections::SizeFlags {
            fill_width: snapshot.size_fill_width,
            fill_height: snapshot.size_fill_height,
            hug_width: snapshot.size_hug_width,
            hug_height: snapshot.size_hug_height,
            clip_content: snapshot.size_clip_content,
        };
        // Padding edit mode: the user's gear pin (only while it still
        // applies to the selected node — see `padding_edit_mode_anchor`),
        // else derived from the node's four effective values (TS
        // default-derives each frame). Anchor-scoping stops one node's
        // pinned mode leaking into the next selection.
        let pin_applies = ui.padding_edit_mode_anchor == state.selection.anchor.as_str();
        let padding_edit_mode = ui
            .padding_edit_mode
            .filter(|_| pin_applies)
            .unwrap_or_else(|| {
                let p = snapshot.layout_padding;
                op_editor_core::PaddingEditMode::from_values(p.top, p.right, p.bottom, p.left)
            });
        Self {
            id: WidgetId::new(2000),
            snapshot,
            theme: theme_for(ui),
            labels: sections::PropertyLabels::for_editor_ui(ui),
            // Multi-select inputs are inert in v1 — broadcast edits
            // to all selected nodes lands later. Force focus to None
            // so the panel paints all values muted and hit_test
            // returns None (see `hit_test` is_multi short-circuit).
            focus: if is_multi {
                None
            } else {
                state.ui.property_focus
            },
            draft: if is_multi {
                String::new()
            } else {
                state.ui.property_input_draft.clone()
            },
            caret_pos: if is_multi {
                0
            } else {
                state.ui.property_caret_pos
            },
            select_all: !is_multi && state.ui.property_draft_select_all,
            caret_anchor_ms: state.ui.property_caret_anchor_ms,
            now_ms,
            flex_layout,
            size_flags,
            fill_type,
            fill_type_picker_open: ui.fill_type_picker_open,
            image_fill_popover_open: ui.image_fill_popover_open,
            font_family_picker_open: ui.font_family_picker_open,
            font_weight_picker_open: ui.font_weight_picker_open,
            font_weight_picker_hover: ui.font_weight_picker_hover,
            action_hover: if is_multi {
                None
            } else {
                ui.property_action_hover
            },
            padding_edit_mode,
            padding_mode_popover_open: ui.padding_mode_popover_open,
            padding_mode_popover_hover: ui.padding_mode_popover_hover,
            is_multi,
            tab: ui.property_tab,
            tab_hover: ui.property_tab_hover,
            export_format: ui.export_format,
            export_scale: ui.export_scale,
            export_scale_picker_open: ui.export_scale_picker_open,
            export_format_picker_open: ui.export_format_picker_open,
            export_picker_hover: ui.export_picker_hover,
            scroll: ui.property_panel_scroll.max(0.0),
            locale: ui.locale,
            // Inert in the multi-select aggregate view.
            effect_param_focus: if is_multi {
                None
            } else {
                ui.effect_param_focus
            },
            codegen: state.codegen.clone(),
        }
    }

    /// `self.scroll` clamped to the current content's scrollable
    /// range. The host only re-clamps the stored offset on a wheel
    /// event, so selecting a shorter node (fewer sections / effects)
    /// could otherwise leave the panel scrolled past its end —
    /// every paint / hit-test reads through this so the view
    /// self-corrects on the very next frame.
    fn effective_scroll(&self, panel_rect: Rect) -> f32 {
        let max = (self.content_height(panel_rect) - panel_rect.size.y).max(0.0);
        self.scroll.clamp(0.0, max)
    }

    /// `panel_rect` shifted up by the (clamped) scroll offset. Both
    /// paint and every hit-test walker start their y-walk from this
    /// rect, so the panel scrolls as one piece and clicks stay
    /// aligned with what is drawn.
    fn scrolled_rect(&self, panel_rect: Rect) -> Rect {
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
        point.y >= panel_rect.origin.y + crate::widgets::property_panel_inputs::TAB_HEIGHT
    }

    /// Total height (px) of the panel's section content — drives the
    /// scroll clamp so the inspector can't scroll past its end.
    pub fn content_height(&self, panel_rect: Rect) -> f32 {
        sections::property_panel_content_height(
            panel_rect,
            self.visible_sections(),
            &self.snapshot.effects,
        )
    }

    /// Section-visibility mask for the current selection, threaded
    /// into every layout walker so paint + hit-test stay aligned.
    fn visible_sections(&self) -> sections::VisibleSections {
        let caps = self.capabilities();
        sections::VisibleSections {
            create_component: caps.create_component && self.snapshot.can_create_component,
            flex_layout: caps.flex_layout,
            flex_layout_mode: self.snapshot.flex_layout,
            padding_edit_mode: self.padding_edit_mode,
            layout_justify: self.snapshot.layout_justify,
            layout_align: self.snapshot.layout_align,
            size_options: caps.size_options,
            size_fill_width: self.snapshot.size_fill_width,
            size_fill_height: self.snapshot.size_fill_height,
            size_hug_width: self.snapshot.size_hug_width,
            size_hug_height: self.snapshot.size_hug_height,
            clip_content: self.snapshot.can_clip_content,
            text: caps.text && self.snapshot.text.is_some(),
            icon: self.snapshot.icon.is_some(),
            image: caps.image && self.snapshot.is_image_node,
            opacity: caps.opacity,
            corner_radius: self.snapshot.has_corner_radius,
            polygon_sides: self.snapshot.polygon_sides.is_some(),
            ellipse_arc: self.snapshot.ellipse_arc.is_some(),
            fill: caps.fill,
            stroke: caps.stroke,
            effects: caps.effects,
            export: caps.export,
            fill_type: self.fill_type,
            gradient_stop_count: self.snapshot.gradient_stops.len(),
        }
    }

    /// Hit-test the flex / size buttons + checkboxes. Returns the
    /// action the host should dispatch, or `None` if the cursor
    /// missed every clickable shape. Called AFTER `hit_test` so
    /// text inputs win over the action rects they overlap with.
    pub fn hit_test_action(&self, panel_rect: Rect, point: Point2D) -> Option<PropertyPanelAction> {
        // Design / Code tab strip — clickable on either tab, incl. multi-select.
        if let Some(tab) = sections::tab_strip_hit(
            &self.labels,
            panel_rect.origin.x,
            panel_rect.origin.y,
            point,
        ) {
            return Some(PropertyPanelAction::SetPropertyTab(tab));
        }
        if self.is_multi {
            // Multi-select inputs / toggles are inert in v1.
            return None;
        }
        if matches!(self.tab, op_editor_core::PropertyTab::Code) {
            return code_action_hit(panel_rect, &self.codegen, point);
        }
        if self.image_fill_popover_open {
            if let Some(action) = sections::image_fill_popover_action_at(
                self.scrolled_rect(panel_rect),
                self.visible_sections(),
                &self.snapshot,
                point,
            ) {
                return Some(action);
            }
        }
        if !self.point_in_section_viewport(panel_rect, point) {
            return None;
        }
        let rects = sections::action_button_rects_with_fill_picker(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &self.snapshot.effects,
            self.fill_type_picker_open,
            self.font_family_picker_open,
            self.font_weight_picker_open,
            self.export_scale_picker_open,
            self.export_format_picker_open,
            self.padding_mode_popover_open,
        );
        // Picker rows live in `rects` AFTER the dropdown rect, so
        // a row hit takes priority — `rev()` makes the picker rows
        // tested first and short-circuits before the dropdown
        // toggle, otherwise clicking a row would just re-toggle.
        for (action, rect) in rects.into_iter().rev() {
            if rect_contains(rect, point) {
                return Some(action);
            }
        }
        None
    }

    /// Row index of the open Export select popup under `point`, or
    /// `None` when no popup is open / the cursor is off every row.
    /// The index counts only the option rows (`SetExportScale` /
    /// `SetExportFormat`), matching `paint_select_popup`'s row walk,
    /// so it can drive the popup's hover highlight.
    pub fn export_picker_row_at(&self, panel_rect: Rect, point: Point2D) -> Option<usize> {
        if !self.export_scale_picker_open && !self.export_format_picker_open {
            return None;
        }
        if !self.point_in_section_viewport(panel_rect, point) {
            return None;
        }
        sections::action_button_rects_with_fill_picker(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &self.snapshot.effects,
            self.fill_type_picker_open,
            self.font_family_picker_open,
            self.font_weight_picker_open,
            self.export_scale_picker_open,
            self.export_format_picker_open,
            self.padding_mode_popover_open,
        )
        .into_iter()
        .filter(|(a, _)| {
            matches!(
                a,
                PropertyPanelAction::SetExportScale(_) | PropertyPanelAction::SetExportFormat(_)
            )
        })
        .position(|(_, rect)| rect_contains(rect, point))
    }

    pub fn image_adjustment_drag_action(
        &self,
        panel_rect: Rect,
        field: op_editor_core::ImageAdjustmentField,
        x: f32,
    ) -> Option<PropertyPanelAction> {
        if self.is_multi || !self.image_fill_popover_open {
            return None;
        }
        sections::image_fill_popover_adjustment_action_for_drag(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            field,
            x,
        )
    }

    pub fn image_fill_popover_contains(&self, panel_rect: Rect, point: Point2D) -> bool {
        !self.is_multi
            && self.image_fill_popover_open
            && sections::image_fill_popover_contains(
                self.scrolled_rect(panel_rect),
                self.visible_sections(),
                point,
            )
    }

    /// Hit-test the panel at `point` and return which input row
    /// (if any) contains the click. The layout walk mirrors the
    /// per-kind section filtering applied in `paint`, so rects
    /// after a skipped section don't drift out of alignment.
    pub fn hit_test(&self, panel_rect: Rect, point: Point2D) -> Option<PropertyFocus> {
        if self.is_multi {
            // Inputs inert in v1 multi-select aggregate view.
            return None;
        }
        if !self.point_in_section_viewport(panel_rect, point) {
            return None;
        }
        for (focus, rect) in
            sections::editable_input_rects(self.scrolled_rect(panel_rect), self.visible_sections())
        {
            if rect_contains(rect, point) {
                return Some(focus);
            }
        }
        None
    }

    /// Index into `action_button_rects_with_fill_picker` of the action
    /// button under `point`, or `None`. Design-tab single-select only —
    /// drives the per-button `theme.button_hover` wash. Shares the
    /// walker geometry with `hit_test_action` + paint so it can't drift.
    pub fn action_hover_index(&self, panel_rect: Rect, point: Point2D) -> Option<usize> {
        if self.is_multi || matches!(self.tab, op_editor_core::PropertyTab::Code) {
            return None;
        }
        if !self.point_in_section_viewport(panel_rect, point) {
            return None;
        }
        sections::action_button_rects_with_fill_picker(
            self.scrolled_rect(panel_rect),
            self.visible_sections(),
            &self.snapshot.effects,
            self.fill_type_picker_open,
            self.font_family_picker_open,
            self.font_weight_picker_open,
            self.export_scale_picker_open,
            self.export_format_picker_open,
            self.padding_mode_popover_open,
        )
        .iter()
        .position(|(_, r)| rect_contains(*r, point))
    }

    /// Pinned Design / Code tab under the cursor.
    pub fn tab_hover_at(
        &self,
        panel_rect: Rect,
        point: Point2D,
    ) -> Option<op_editor_core::PropertyTab> {
        sections::tab_strip_hit(
            &self.labels,
            panel_rect.origin.x,
            panel_rect.origin.y,
            point,
        )
    }
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

use crate::widgets::property_panel_code::{code_action_hit, paint_code_panel_in_panel};

impl Widget for PropertyPanel {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        // Vertical extent is "as much as you give me" — the host
        // clips at the rail rect. Reporting 800 here is just a
        // placeholder for the abstract widget tree.
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(cx.available_width, 800.0),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_rect(rect, self.theme.card);
        cx.backend.fill_rect(
            Rect {
                origin: rect.origin,
                size: Point2D::new(1.0, rect.size.y),
            },
            self.theme.border,
        );

        let x = rect.origin.x;
        let w = rect.size.x;
        // The Design / Code tab strip is pinned to the panel top —
        // painted fixed, above (and never scrolled with) the section
        // content.
        let tab_bottom = sections::paint_tab_strip(
            cx,
            &self.theme,
            &self.labels,
            sections::TabStripState {
                active: self.tab,
                hover: self.tab_hover,
            },
            x,
            rect.origin.y,
            w,
        );
        let edit_ctx = sections::EditContext {
            focus: self.focus,
            draft: self.draft.as_str(),
            caret: self.caret_pos,
            select_all: self.select_all,
            caret_anchor_ms: self.caret_anchor_ms,
            now_ms: self.now_ms,
        };
        let caps = self.capabilities();
        if matches!(self.tab, op_editor_core::PropertyTab::Code) {
            paint_code_panel_in_panel(cx, &self.theme, &self.codegen, rect, self.now_ms);
            return;
        }
        // Section content scrolls below the pinned tab strip; clip it
        // so a scrolled-up section can't paint over the tabs or bleed
        // onto the neighbouring rail. Overlays (fill / export pickers)
        // anchor to `scrolled` — the same shifted rect the layout
        // walker uses (it adds `TAB_HEIGHT`), so paint + hit-test of
        // the sections agree.
        cx.backend.save();
        cx.backend.clip_rect(Rect {
            origin: Point2D::new(x, tab_bottom),
            size: Point2D::new(w, (rect.origin.y + rect.size.y - tab_bottom).max(0.0)),
        });
        let scroll = self.effective_scroll(rect);
        let scrolled = Rect {
            origin: Point2D::new(rect.origin.x, rect.origin.y - scroll),
            size: rect.size,
        };
        // First section sits just below the pinned tab strip:
        // `tab_bottom - scroll` == `scrolled.origin.y + TAB_HEIGHT`,
        // matching the layout walker's `+= TAB_HEIGHT` step.
        let mut y = tab_bottom - scroll;
        y = sections::paint_node_header(cx, &self.theme, &self.snapshot, x, y, w);
        if caps.create_component && self.snapshot.can_create_component {
            y = sections::paint_create_component(cx, &self.theme, &self.labels, x, y, w);
        }
        y = sections::paint_position_section(
            cx,
            &self.theme,
            &self.snapshot,
            &edit_ctx,
            &self.labels,
            self.snapshot.has_corner_radius,
            x,
            y,
            w,
        );
        let flex_section_y = y;
        if caps.flex_layout {
            y = crate::widgets::property_panel_flex::paint_flex_section(
                cx,
                &self.theme,
                &self.snapshot,
                &edit_ctx,
                &self.labels,
                self.locale,
                self.padding_edit_mode,
                x,
                y,
                w,
            );
        }
        if caps.size_options {
            y = sections::paint_size_section(
                cx,
                &self.theme,
                &self.snapshot,
                &edit_ctx,
                &self.labels,
                self.size_flags,
                self.snapshot.can_clip_content,
                x,
                y,
                w,
            );
        }
        if self.snapshot.icon.is_some() {
            y = crate::widgets::property_panel_icon::paint_icon_section(
                cx,
                &self.theme,
                &self.snapshot,
                self.locale,
                x,
                y,
                w,
            );
        }
        if caps.text && self.snapshot.text.is_some() {
            y = crate::widgets::property_panel_text::paint_text_section(
                cx,
                &self.theme,
                &self.snapshot,
                &edit_ctx,
                self.locale,
                x,
                y,
                w,
            );
        }
        if caps.image && self.snapshot.is_image_node {
            y = crate::widgets::property_panel_image_node::paint_image_node_section(
                cx,
                &self.theme,
                &self.snapshot,
                self.locale,
                x,
                y,
                w,
            );
        }
        if caps.opacity {
            y = sections::paint_layer_section(
                cx,
                &self.theme,
                &self.snapshot,
                &self.labels,
                &edit_ctx,
                x,
                y,
                w,
            );
        }
        if caps.fill {
            y = sections::paint_fill_section(
                cx,
                &self.theme,
                &self.snapshot,
                &edit_ctx,
                &self.labels,
                self.fill_type,
                self.fill_type_picker_open,
                self.locale,
                x,
                y,
                w,
            );
        }
        if caps.stroke {
            y = sections::paint_stroke_section(
                cx,
                &self.theme,
                &self.snapshot,
                &edit_ctx,
                &self.labels,
                x,
                y,
                w,
            );
        }
        if caps.effects {
            y = sections::paint_effects_section(
                cx,
                &self.theme,
                &self.labels,
                &self.snapshot.effects,
                &edit_ctx,
                self.effect_param_focus,
                x,
                y,
                w,
            );
        }
        if caps.export {
            let _ = sections::paint_export_section(
                cx,
                &self.theme,
                &self.labels,
                self.export_format,
                self.export_scale,
                x,
                y,
                w,
            );
        }
        // Fill-type picker overlay sits on top of everything below
        // the Fill section so it can extend past the section divider.
        if caps.fill && self.fill_type_picker_open {
            sections::paint_fill_type_picker(
                cx,
                &self.theme,
                scrolled,
                self.visible_sections(),
                self.fill_type,
                self.locale,
            );
        }
        if caps.text && self.font_family_picker_open {
            if let Some(text) = self.snapshot.text.as_ref() {
                crate::widgets::property_panel_text::paint_font_family_picker(
                    cx,
                    &self.theme,
                    scrolled,
                    self.visible_sections(),
                    &text.font_family,
                );
            }
        }
        if caps.text && self.font_weight_picker_open {
            if let Some(text) = self.snapshot.text.as_ref() {
                crate::widgets::property_panel_text::paint_font_weight_picker(
                    cx,
                    &self.theme,
                    scrolled,
                    self.visible_sections(),
                    self.locale,
                    text.font_weight,
                    self.font_weight_picker_hover,
                );
            }
        }
        // Padding mode-selector popover — overlays the sections below
        // the gear. Anchored off the flex section's body top (after its
        // header), matching the y the action-rect walker passes to
        // `push_flex_action_rects`.
        if caps.flex_layout && self.padding_mode_popover_open {
            crate::widgets::property_panel_flex::paint_padding_mode_popover(
                cx,
                &self.theme,
                self.locale,
                self.padding_edit_mode,
                self.padding_mode_popover_hover,
                x,
                flex_section_y + crate::widgets::property_panel_inputs::SECTION_HEADER_HEIGHT,
                w,
            );
        }
        // Export-section inline select popups — painted last so the
        // scale / format dropdown overlays sit above every section.
        if caps.export && (self.export_scale_picker_open || self.export_format_picker_open) {
            sections::paint_export_picker(
                cx,
                &self.theme,
                scrolled,
                self.visible_sections(),
                &self.snapshot.effects,
                self.export_scale_picker_open,
                self.export_format_picker_open,
                self.export_scale,
                self.export_format,
                self.export_picker_hover,
            );
        }
        // Per-button hover wash — one translucent overlay on the action
        // button under the cursor (flex / size / fill / effects / export
        // / create-component). Index into the same walker the host's
        // hover update + hit-test use, so it lands exactly on the button.
        if let Some(i) = self.action_hover {
            let rects = sections::action_button_rects_with_fill_picker(
                self.scrolled_rect(rect),
                self.visible_sections(),
                &self.snapshot.effects,
                self.fill_type_picker_open,
                self.font_family_picker_open,
                self.font_weight_picker_open,
                self.export_scale_picker_open,
                self.export_format_picker_open,
                self.padding_mode_popover_open,
            );
            if let Some((_, r)) = rects.get(i) {
                cx.backend.fill_round_rect(*r, 6.0, self.theme.button_hover);
            }
        }
        cx.backend.restore();
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_label(self.snapshot.kind.clone());
        node
    }
}

impl PropertyPanel {
    /// Paint inspector overlays that are allowed to extend out of the
    /// right rail. Hosts call this late in their composition pass so
    /// the image-fill popover is above floating canvas controls.
    pub fn paint_overlays(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let caps = self.capabilities();
        if !(caps.fill || caps.image) || !self.image_fill_popover_open {
            return;
        }
        let scroll = self.effective_scroll(rect);
        let scrolled = Rect {
            origin: Point2D::new(rect.origin.x, rect.origin.y - scroll),
            size: rect.size,
        };
        sections::paint_image_fill_popover(
            cx,
            &self.theme,
            scrolled,
            self.visible_sections(),
            &self.snapshot,
            self.locale,
        );
    }
}
