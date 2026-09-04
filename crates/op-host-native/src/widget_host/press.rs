//! Mouse-press dispatcher.
//!
//! `EditorState` is the host's source of truth. Canvas hit-tests run
//! against the layout-resolved `LayoutScene` (refreshed at the top
//! of `apply_press`); the resolved-scene node ids are wrapped into
//! op-editor-core `NodeId`s before feeding mutators. Press-helper
//! methods (`create_node_for_active_tool`,
//! `dispatch_agent_settings_press`) live in `press_helpers.rs`.
//!
//! The `apply_press` tier bodies live in the `press_*_tiers.rs` siblings;
//! `press_ctx.rs` carries the per-event state they share.

use super::press_ctx::PressCtx;
use super::WidgetHostNative;
use op_editor_ui::widgets::press_flow::{
    self, LayerContextMenuPress, LayerContextStep, PropertyOverlayPress,
};
use op_editor_ui::Point2D;

impl WidgetHostNative {
    // `pub(in crate::widget_host)` so the instance-panel tests can
    // drive the context-menu rows without simulating menu geometry.
    pub(in crate::widget_host) fn dispatch_layer_context_action(
        &mut self,
        action: op_editor_ui::widgets::layer_context_menu::LayerContextAction,
        target: op_editor_core::ui_draft::LayerContextTarget,
    ) {
        use op_editor_core::{
            CollabDocumentMutation as Mutation, CollabNodeField as Field,
            CollabUnsupportedFeature as Unsupported,
        };
        use op_editor_ui::widgets::layer_context_menu::LayerContextAction as Action;

        let mutation = match action {
            Action::RenameLayer => Mutation::NodeProperty(Field::Name),
            Action::Duplicate => Mutation::Unsupported(Unsupported::Duplicate),
            Action::Delete => Mutation::NodeDelete,
            Action::GroupSelection => Mutation::Group,
            Action::BooleanUnion
            | Action::BooleanSubtract
            | Action::BooleanIntersect
            | Action::BooleanExclude => Mutation::Unsupported(Unsupported::NodeReplacement),
            Action::CreateComponent | Action::DetachComponent | Action::DetachInstance => {
                Mutation::Unsupported(Unsupported::Components)
            }
            Action::ToggleLock | Action::ToggleVisibility => {
                Mutation::Unsupported(Unsupported::VisibilityAndLocking)
            }
            Action::RenamePage
            | Action::DuplicatePage
            | Action::MovePageUp
            | Action::MovePageDown
            | Action::DeletePage => Mutation::Unsupported(Unsupported::PageStructure),
        };
        if !self.collab_allows_document_mutation(mutation) {
            return;
        }
        match press_flow::apply_layer_context_action(
            &mut self.editor_state,
            &mut self.next_node_id,
            action,
            target,
            self.now_ms,
        ) {
            LayerContextStep::Done => {}
            LayerContextStep::Group => {
                let _ = self.apply_group();
            }
            LayerContextStep::Boolean(op) => {
                #[cfg(feature = "gl-host")]
                let _ = self.apply_boolean_op(op);
                #[cfg(not(feature = "gl-host"))]
                let _ = op;
            }
            LayerContextStep::Refit => {
                self.fit_active_page_after_switch(self.last_viewport_w, self.last_viewport_h);
            }
        }
        self.mark_dirty();
    }

    /// Platform tail for a press routed to an open property-panel
    /// popover (`press_flow::press_*`). Every outcome consumes the
    /// press.
    pub(in crate::widget_host) fn finish_property_overlay_press(
        &mut self,
        press: PropertyOverlayPress,
    ) -> bool {
        match press {
            PropertyOverlayPress::Action(action) => self.apply_property_action(action),
            PropertyOverlayPress::Swallow => {}
            PropertyOverlayPress::Dismissed => self.mark_dirty(),
        }
        true
    }

    /// Right-click handler — opens the LayerPanel context menu on
    /// a layer row OR page row.
    pub fn apply_right_press(&mut self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        let cancelled = self.cancel_native_touch_gestures();
        if self.editor_state.editor_ui.agent_settings_open {
            return true;
        }
        // Codex stop-gate: right-click outside the variables panel
        // must commit any pending row focus first.
        self.commit_variable_row_focus_if_any();
        self.close_image_popovers_for_higher_overlay();
        // Any top-most floating panel swallows a right-click on its
        // rect — no context menu opens under them.
        if self.over_topmost_panel(x, y, viewport_w, viewport_h) {
            return true;
        }
        // The model picker can extend across the LayerPanel. A secondary
        // press on its visible card belongs to that floating surface and must
        // never open a context menu for the covered layer/page underneath.
        if self
            .chat_model_picker_rect(viewport_w, viewport_h)
            .is_some_and(|rect| rect.contains(Point2D::new(x, y)))
        {
            return true;
        }
        // Select-tool right-click on a path anchor / handle opens the
        // point-type menu (`pen_press.rs`).
        if self.try_open_path_anchor_menu(x, y, viewport_w, viewport_h) {
            return true;
        }
        if !self.layers_panel_visible() {
            // No layer panel to hit — the right press is blank chrome;
            // blur inputs like the sidebar-open fall-through below.
            return self.blur_text_inputs_on_blank_press() || cancelled;
        }
        let layer_rect = self.layers_content_rect(viewport_w, viewport_h);
        let panel = self.layer_panel();
        let hit = panel.hit_test(layer_rect, Point2D::new(x, y));
        match press_flow::open_layer_context_menu(&mut self.editor_state, hit, x, y) {
            LayerContextMenuPress::Opened | LayerContextMenuPress::Dismissed => true,
            // Right-press on blank chrome — blur inputs like a left press.
            LayerContextMenuPress::Missed => self.blur_text_inputs_on_blank_press() || cancelled,
        }
    }

    /// Mouse-press handler. Returns whether anything visible changed.
    ///
    /// A strictly ordered hit-test ladder: overlays before panels before
    /// canvas (see `crates/CLAUDE.md`). Each tier helper returns
    /// `Option<bool>` — `None` means "declined, fall through to the next
    /// tier", `Some(dirty)` means "claimed the press, and this is the
    /// repaint signal".
    ///
    /// THE CALL ORDER BELOW *IS* THE BEHAVIOUR. The tier bodies live in
    /// the `press_*_tiers.rs` siblings only to respect the per-file line
    /// cap; reordering these calls changes which surface owns a click.
    pub fn apply_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        self.apply_press_inner(x, y, viewport_width, viewport_height, true)
    }

    /// Timestamped press variant: `t_ms` is the event's factual
    /// monotonic timestamp. The ordinary tier ladder runs
    /// byte-identically; only the live-preview pointer path
    /// (`preview_dispatch_press` and anything it cancels) stamps
    /// `PointerEvent.t_ms` with `t_ms` instead of the host's global
    /// clock. The global clock is NOT advanced here — callers advance
    /// it separately (monotonically) via [`Self::set_now_ms`], so an
    /// out-of-order event keeps its factual time without regressing
    /// the clock. The scoped context is restored afterwards; a panic
    /// inside the ladder poisons the engine at the ABI boundary, at
    /// which point no further event can ever observe a stale ticket.
    pub fn apply_press_at(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
        t_ms: u64,
    ) -> bool {
        let previous = self.preview_event_time_ms.replace(t_ms);
        let changed = self.apply_press_inner(x, y, viewport_width, viewport_height, true);
        self.preview_event_time_ms = previous;
        changed
    }

    /// Re-enter the ordinary press ladder after a touch-panel tap has been
    /// confirmed on release. `allow_touch_panel_defer = false` prevents the
    /// replayed point from arming itself again.
    pub(in crate::widget_host) fn replay_touch_panel_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        self.apply_press_inner(x, y, viewport_width, viewport_height, false)
    }

    fn apply_press_inner(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
        allow_touch_panel_defer: bool,
    ) -> bool {
        self.last_viewport_w = viewport_width;
        self.last_viewport_h = viewport_height;
        // Tier 0 — the mobile save-name dialog is fully modal while open
        // (only the FFI hosts ever open it; desktop state stays closed).
        if let Some(consumed) =
            self.press_save_name_dialog_tier(x, y, viewport_width, viewport_height)
        {
            return consumed;
        }
        let presenting = self.preview_slideshow_active();
        if !presenting
            && self.begin_agent_settings_touch_gesture(x, y, viewport_width, viewport_height)
        {
            return true;
        }
        if !presenting
            && allow_touch_panel_defer
            && self.begin_asset_center_touch_gesture(x, y, viewport_width, viewport_height)
        {
            return true;
        }
        // Blur-commit rename + text-edit; track to repaint on click.
        let rename_mutation =
            self.editor_state
                .ui
                .layer_rename
                .as_ref()
                .map(|rename| match &rename.target {
                    op_editor_core::ui_draft::LayerContextTarget::Layer(_) => {
                        op_editor_core::CollabDocumentMutation::NodeProperty(
                            op_editor_core::CollabNodeField::Name,
                        )
                    }
                    op_editor_core::ui_draft::LayerContextTarget::Page(_) => {
                        op_editor_core::CollabDocumentMutation::Unsupported(
                            op_editor_core::CollabUnsupportedFeature::PageStructure,
                        )
                    }
                });
        let rename_committed = match rename_mutation {
            Some(mutation) if !self.collab_allows_document_mutation(mutation) => {
                self.editor_state.rename_cancel()
            }
            Some(_) => self.editor_state.rename_commit(),
            None => false,
        };
        let text_edit_was_active = self.editor_state.ui.text_editing.is_some();
        // EXCEPTION to commit-on-press: a press INSIDE the edited
        // text node places the caret instead of committing (TS
        // textarea parity — a click inside the overlay moves the
        // caret; only an outside click blurs + commits). The probe
        // returns `None` whenever any overlay / modal owns the point,
        // so those presses still commit + route normally.
        let text_edit_caret_press =
            self.text_edit_press_offset(x, y, viewport_width, viewport_height);
        let text_edit_committed =
            text_edit_caret_press.is_none() && self.editor_state.text_edit_commit();
        if rename_committed || text_edit_committed {
            self.mark_dirty();
        }
        if let Some(offset) = text_edit_caret_press {
            self.place_text_edit_caret(offset);
            return true;
        }
        let mut ctx = PressCtx {
            x,
            y,
            viewport_width,
            viewport_height,
            rename_committed,
            text_edit_was_active,
            text_edit_committed,
            // Resolved below, at the exact point the flat ladder did.
            in_git_panel: false,
            in_chat_model_picker: false,
        };
        // Tier 1 — top-most modals / floating panels / context menus.
        if let Some(consumed) = self.press_topmost_overlay_tiers(&ctx) {
            return consumed;
        }
        // 0aa. Commit-on-blur for property-panel inputs +
        // variable-row inline editor.
        self.commit_variable_row_focus_if_any();
        if self.editor_state.ui.property_focus.is_some() {
            let property_left = if self.editor_state.property_panel_visible() {
                self.property_rect(viewport_width, viewport_height).origin.x
            } else {
                viewport_width
            };
            if x < property_left {
                self.commit_property_focus_if_any();
            }
        }

        // The floating Git panel paints on top of the right-rail
        // panels (see `paint.rs` §8.2) — and in diff mode it widens
        // to 620 px, which overlaps the property / variables rail
        // (and its resize gutter) on a narrow window. Hit-test order
        // must mirror paint Z-order: a click inside the Git-panel
        // rect belongs to the Git panel (tier 9 below), so every
        // rail tier in between skips a click it would otherwise own.
        ctx.in_git_panel = self
            .git_panel_outer_rect(viewport_width, viewport_height)
            .is_some_and(|r| (r).contains(Point2D::new(x, y)));
        ctx.in_chat_model_picker = self
            .chat_model_picker_rect(viewport_width, viewport_height)
            .is_some_and(|r| r.contains(Point2D::new(x, y)));

        // Tier 2 — shape picker, file / export / figma / login / account
        // modals, import + locale dropdowns.
        if let Some(consumed) = self.press_menu_modal_tiers(&ctx) {
            return consumed;
        }
        // Tier 3 — image-fill popover, StatusBar, resize gutter, and the
        // model-picker slice that lifts above the TopBar.
        if let Some(consumed) = self.press_rail_overlay_tiers(&ctx) {
            return consumed;
        }
        // Tier 3b — an open touch surface owns its outside scrim and close
        // affordance before app-bar/page/dock controls can claim the tap.
        if !self.preview_slideshow_active() {
            if let Some(consumed) = self.press_mobile_modal_surface_tier(&ctx) {
                return consumed;
            }
        }
        // Tier 4 — TopBar chrome (and its blank-press gaps); mobile
        // layout replaces it with the floating action cluster.
        let presenting = self.preview_slideshow_active();
        if self.editor_state.editor_ui.touch_chrome() && !presenting {
            if let Some(consumed) = self.press_mobile_app_bar_tier(&ctx) {
                return consumed;
            }
        } else if !self.editor_state.editor_ui.touch_chrome() {
            if let Some(consumed) = self.press_top_bar_tier(&ctx) {
                return consumed;
            }
        }
        // Tier 5 — Preview (Play) mode swallows everything else.
        if let Some(consumed) = self.press_preview_tier(&ctx) {
            return consumed;
        }
        // Tier 6 — property-panel popovers.
        if let Some(consumed) = self.press_property_overlay_tiers(&ctx, allow_touch_panel_defer) {
            return consumed;
        }
        // Tier 7 — model picker, theme presets, VariablesPanel.
        if let Some(consumed) = self.press_panel_dispatch_tiers(&ctx) {
            return consumed;
        }
        // Tier 8 — PropertyPanel input row.
        if let Some(consumed) = self.press_property_panel_tier(&ctx, allow_touch_panel_defer) {
            return consumed;
        }
        // Tier 9 — floating Git panel, then the AI chat panel.
        if let Some(consumed) = self.press_git_and_chat_tiers(&ctx, allow_touch_panel_defer) {
            return consumed;
        }
        // Tier 9a — selected-node actions paint above the canvas and own
        // their hit area before page navigation, the dock, layers or canvas.
        if let Some(consumed) = self.press_selection_actions_tier(&ctx) {
            return consumed;
        }
        // Tier 9b — shared page switcher (touch layouts).
        if self.editor_state.editor_ui.touch_chrome()
            && !self.mobile_sheet_is_modal()
            && !self.editor_state.editor_ui.variables_panel_open
        {
            let page_count = self
                .editor_state
                .doc
                .pages
                .as_ref()
                .map(|pages| pages.len())
                .unwrap_or(1);
            if page_count > 1 {
                let pill = op_editor_ui::widgets::mobile_chrome::page_pill_rect_for(
                    &self.editor_state,
                    ctx.viewport_width,
                    ctx.viewport_height,
                );
                let point = Point2D::new(ctx.x, ctx.y);
                if let Some(hit) = op_editor_ui::widgets::mobile_chrome::page_pill_hit(pill, point)
                {
                    let current = self.editor_state.ui.active_page_index;
                    let target = match hit {
                        op_editor_ui::widgets::mobile_chrome::PagePillHit::Prev => {
                            current.saturating_sub(1)
                        }
                        op_editor_ui::widgets::mobile_chrome::PagePillHit::Next => {
                            (current + 1).min(page_count - 1)
                        }
                    };
                    if target != current && self.editor_state.set_active_page(target) {
                        self.fit_active_page_after_switch(ctx.viewport_width, ctx.viewport_height);
                    }
                    self.mark_dirty();
                    return true;
                }
                if pill.contains(point) {
                    return true;
                }
            }
        }
        // Tier 10 — toolbar + floating align toolbar (desktop); on touch
        // layouts the bottom tool dock replaces the desktop toolbar.
        if self.editor_state.editor_ui.touch_chrome() && !self.preview_slideshow_active() {
            if !self.mobile_sheet_is_modal() && !self.editor_state.editor_ui.variables_panel_open {
                if let Some(consumed) = self.press_mobile_dock_tier(&ctx) {
                    return consumed;
                }
            }
        } else if let Some(consumed) = self.press_toolbar_tiers(&ctx) {
            return consumed;
        }
        // Tier 12 — LayerPanel drag peek, then `apply_click`.
        if let Some(consumed) = self.press_layer_and_click_tiers(&ctx, allow_touch_panel_defer) {
            return consumed;
        }
        // Blank padding inside an open touch surface still belongs to that
        // surface; never let it select or create something on the canvas
        // underneath.
        if self.editor_state.editor_ui.touch_chrome() {
            if let Some(kind) = self.editor_state.editor_ui.mobile_sheet {
                let panel = self.mobile_sheet_rect(ctx.viewport_width, ctx.viewport_height, kind);
                if panel.contains(Point2D::new(ctx.x, ctx.y)) {
                    return true;
                }
            }
        }
        // Tier 14 — the canvas, branching on the active tool.
        if let Some(consumed) = self.press_canvas_tier(&ctx) {
            return consumed;
        }
        // Final fall-through — the press hit no interactive chrome
        // (panel-rail gaps, property-panel padding, …): blank press.
        let blurred = self.blur_text_inputs_on_blank_press();
        blurred || rename_committed || text_edit_committed
    }
}
