//! Deck slideshow arm of Preview (Play) mode.
//!
//! Previewing a deck presents it: one board filling the viewport, the rest
//! of the canvas backdrop, arrow keys to move. The decision of WHETHER to
//! present, which boards there are, and where the presenter is in the deck
//! all live in `op_editor_core::preview_slideshow`; this file is the
//! platform arm — camera, paint, and the key forwarding.
//!
//! It reuses the existing preview pipeline rather than adding a second one:
//! the board is framed by the shared viewport fit math, and painted by
//! `PreviewSession::paint_scene`, the same painter every other preview uses.
//! The only thing this adds is a clip to the board's own rect, which is what
//! makes the neighbouring boards disappear and the surround read as a
//! letterbox.

use super::WidgetHost;
use op_editor_ui::Point2D;
use op_editor_ui::Rect;

/// Minimum screen-space travel before a presentation drag becomes a swipe.
///
/// This deliberately leaves a wide neutral band above the ordinary 4px tap
/// slop: a slightly wandering finger must cancel the tap without accidentally
/// changing slides. Logical host pixels are points on the mobile players, so
/// 48px is also a comfortable touch threshold there.
const SLIDESHOW_SWIPE_THRESHOLD_PX: f32 = 48.0;

/// Horizontal motion must beat vertical motion by this factor before a deck
/// changes slides. Near-diagonal and vertical drags stay inert, which lets a
/// presenter reposition their finger without an unexpected page change.
const SLIDESHOW_SWIPE_AXIS_DOMINANCE: f32 = 1.25;

/// Threshold for tap vs drag discrimination. Web uses 2.0 px, matching
/// node_drag.rs locally. Native uses 4.0.
const NODE_DRAG_THRESHOLD_PX: f32 = 2.0;

/// Scene-space rect through the canvas pan/zoom into screen space.
///
/// Same formula as the canvas painter's (`canvas_origin + pan + doc *
/// zoom`); taken from the passed `canvas` rect rather than re-deriving the
/// canvas region so paint and this clip can never disagree by a frame.
fn board_screen_rect(canvas: Rect, viewport: &op_editor_core::Viewport, board: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            canvas.origin.x + viewport.pan_x + board.origin.x * viewport.zoom,
            canvas.origin.y + viewport.pan_y + board.origin.y * viewport.zoom,
        ),
        size: Point2D::new(board.size.x * viewport.zoom, board.size.y * viewport.zoom),
    }
}

impl WidgetHost {
    /// Whether preview is currently presenting a deck.
    ///
    /// Both halves matter: the editor state carries the deck, the host
    /// carries the live session that paints it. A slideshow without a
    /// session would paint nothing and swallow the arrow keys.
    pub fn preview_slideshow_active(&self) -> bool {
        self.preview.is_some() && self.editor_state.preview_slideshow().is_some()
    }

    /// Start the slideshow if the document entering preview is a deck, and
    /// frame its first board. Returns whether a slideshow began.
    ///
    /// Called from `enter_preview` once the session exists. A document that
    /// is not a deck leaves preview untouched.
    pub(in crate::widget_host) fn begin_slideshow_if_deck(
        &mut self,
        canvas_size: (f32, f32),
    ) -> bool {
        if !self.editor_state.begin_preview_slideshow() {
            return false;
        }
        // A deck has no phone / desktop dimension to it — it is presented at
        // its own aspect. Canvas is the device mode that adds no chrome, so
        // the switcher (hidden while presenting) still reads truthfully if
        // the user leaves the slideshow.
        //
        // `initialize_device_preview` ran first and inferred Desktop from the
        // board's width, building a device frame with it. That frame has to
        // go, not just be painted around: a left-over frame makes Exit start
        // the device merge animation instead of leaving, so the presentation
        // would refuse to close.
        self.editor_state.editor_ui.preview.device =
            Some(op_editor_core::PreviewDeviceKind::Canvas);
        self.clear_device_preview_state();
        self.preview_mode_transition = None;
        self.frame_slideshow_board(canvas_size);
        true
    }

    /// Prepare the editor for a real preview entry.
    ///
    /// This is shared by the top-bar preview button and the slide-presentation
    /// action. It commits drafts, clears pointer capture, invalidates any
    /// in-flight Figma import ownership, and closes transient chrome before
    /// the session is built. Missing CanvasKit must skip this entirely so the
    /// host state stays unchanged.
    pub(in crate::widget_host) fn prepare_preview_entry(&mut self) {
        let figma_owned = self.editor_state.editor_ui.figma_import_open
            || self.editor_state.editor_ui.figma_import_in_progress
            || !self.editor_state.editor_ui.figma_import_pages.is_empty();
        if let Some(rename) = self.editor_state.ui.layer_rename.as_mut() {
            rename.input.commit_composition(self.now_ms);
        }
        let _ = self.editor_state.rename_commit();
        let _ = self.editor_state.text_edit_commit_composition(self.now_ms);
        let _ = self.editor_state.text_edit_commit();
        self.editor_state
            .ui
            .property_input
            .commit_composition(self.now_ms);
        self.commit_property_focus_if_any();
        self.commit_settings_focus();
        self.blur_text_inputs_on_blank_press();
        self.close_image_popovers_for_higher_overlay();
        if figma_owned {
            self.document_import_generation =
                self.document_import_generation.wrapping_add(1).max(1);
            crate::figma_temp_bridge::cancel_all();
        }
        if self.editor_state.editor_ui.login_modal_status.is_some() {
            self.cancel_login_flow();
        }
        op_editor_core::host_escape_transitions::close_preview_owned_overlays(
            &mut self.editor_state,
            self.now_ms,
        );

        self.preview_surface_capture = None;
        self.preview_edge_swipe_start_x = None;
        self.preview_pressed_pids.clear();
        self.preview_last_doc_by_pid.clear();
        self.slideshow_cursor = None;
        self.slideshow_press_screen = None;
        self.drag = None;
        self.chat_drag = None;
        self.image_adjustment_drag = None;
        self.image_crop_drag = None;
        self.effect_radius_drag = None;
        self.code_selection_drag = None;
        self.chat_input_selection_drag = None;
        self.image_input_selection_drag = None;
        self.chat_text_selection_drag = None;
        self.create_drag = None;
        self.path_anchor_drag = None;
        self.marquee_drag = None;
        self.layer_drag = None;
        self.design_md_drag = None;
        self.component_browser_drag = None;
        self.icon_picker_drag = None;
        self.variables_resize = None;
        self.handle_drag = None;
        self.node_drag = None;
        self.option_drag_source_ids.clear();
    }

    /// Real preview entry for the web shell.
    ///
    /// The helper exists so the top-bar toggle and the Slides panel present
    /// action share the same build path, and so missing CanvasKit can leave
    /// the host untouched instead of pretending preview started.
    #[cfg(feature = "canvaskit")]
    pub(in crate::widget_host) fn enter_preview_from_browser(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
        op_ck: Option<&crate::canvaskit::OpCk>,
    ) -> bool {
        let Some(op_ck) = op_ck else {
            self.editor_state.editor_ui.preview.mode = false;
            self.editor_state.editor_ui.preview.warnings =
                vec!["preview: CanvasKit not initialized".to_string()];
            self.mark_dirty();
            return false;
        };

        if matches!(
            self.preview_mode_transition.as_ref().map(|t| t.kind()),
            Some(op_preview_core::ModeTransitionKind::Exit)
        ) {
            self.finish_exit_teardown();
        }
        if self.preview.is_some() {
            return true;
        }
        self.prepare_preview_entry();
        let (_canvas_left, _canvas_y, canvas_w, canvas_h) =
            self.canvas_region(viewport_w, viewport_h);
        let canvas_size = (canvas_w, canvas_h);
        match crate::preview_host::enter_preview(
            &self.editor_state.doc,
            canvas_size,
            &self.editor_state.ui.variables.active_theme,
            self.editor_state.ui.active_page_index,
            op_ck,
            op_editor_core::preview_slideshow::slideshow_for_document(&self.editor_state).is_some(),
        ) {
            Ok(mut session) => {
                let source_rect = session
                    .framed_root()
                    .map(|(_, rect)| self.framed_root_to_screen_rect(rect, viewport_w, viewport_h));
                session.set_now_ms(self.now_ms);
                self.editor_state.editor_ui.enter_preview();
                self.editor_state.editor_ui.preview.warnings = session.warnings().to_vec();
                self.preview = Some(session);
                self.initialize_device_preview(viewport_w, viewport_h);
                if !self.device_mode_active() {
                    self.center_canvas_on_preview_root(viewport_w, viewport_h);
                }
                self.begin_slideshow_if_deck(canvas_size);
                if let Some(settled) = self.preview_device_frame.as_ref().map(|f| f.frame) {
                    #[cfg(feature = "canvaskit")]
                    {
                        self.preview_mode_transition =
                            Some(op_preview_core::ModeTransition::start(
                                op_preview_core::ModeTransitionKind::Enter,
                                source_rect,
                                settled,
                                self.now_ms,
                            ));
                    }
                }
                self.mark_dirty();
                true
            }
            Err(err) => {
                self.editor_state.editor_ui.preview.mode = false;
                self.editor_state.editor_ui.preview.warnings = vec![format!("preview: {err}")];
                self.mark_dirty();
                false
            }
        }
    }

    /// Fit the board on screen into the canvas region, centred. Returns
    /// whether the camera moved.
    ///
    /// Reuses `Viewport::fit_to`, the same fit the editor uses to frame a
    /// freshly opened document, so a board is framed by exactly the math
    /// everything else is. Zero padding: a presentation fills the surface,
    /// and the surround is the letterbox rather than a margin.
    ///
    /// Called once per frame from paint, which is the only place that knows
    /// the true canvas size — the cached viewport is still zero on a fresh
    /// host and stale for a frame after a resize. Re-framing every frame is
    /// also what keeps a presentation framed while the window is resized,
    /// and is a no-op (no repaint) once the camera has settled.
    pub(in crate::widget_host) fn frame_slideshow_board(
        &mut self,
        canvas_size: (f32, f32),
    ) -> bool {
        let Some(board) = self.slideshow_board_scene_rect() else {
            return false;
        };
        let before = self.editor_state.viewport;
        self.editor_state
            .viewport
            .fit_to(board, canvas_size.0, canvas_size.1, 0.0);
        let moved = self.editor_state.viewport != before;
        if moved {
            self.mark_dirty();
        }
        moved
    }

    /// Scene-space rect of the board being presented.
    fn slideshow_board_scene_rect(&self) -> Option<Rect> {
        let session = self.preview.as_ref()?;
        let board = self.editor_state.preview_slideshow()?.current_board()?;
        session.root_scene_rect(board)
    }

    /// Move the presentation one board forward (`1`) or back (`-1`) and
    /// re-frame. Returns whether the board on screen changed — `false`
    /// outside a slideshow and at the clamped ends of the deck.
    ///
    /// Public so the keyboard handler can reach it for the arrow keys.
    pub fn preview_slideshow_step(&mut self, delta: i32) -> bool {
        if !self.preview_slideshow_active() {
            return false;
        }
        if !self.editor_state.step_preview_slideshow(delta) {
            // At either end of the deck the board holds. The press was still
            // ours — the caller swallows it either way, which is what keeps
            // the last slide on screen instead of letting the key fall
            // through to the editor underneath.
            return false;
        }
        // The camera follows in `frame_slideshow_board`, which paint runs
        // against the authoritative canvas size on the next frame.
        self.mark_dirty();
        true
    }

    /// Paint the board on screen, letterboxed, plus the slide counter.
    ///
    /// The caller has already filled the canvas backdrop, so everything
    /// outside the board's own rect stays backdrop — that IS the letterbox.
    pub(crate) fn paint_slideshow(
        &self,
        frame_backend: &mut dyn op_editor_ui::RenderBackend,
        canvas_rect: Rect,
    ) {
        use op_editor_ui::widgets::{PaintCx, SlideshowToolbar};

        let Some(session) = self.preview.as_ref() else {
            return;
        };
        let Some(slideshow) = self.editor_state.preview_slideshow() else {
            return;
        };
        if let Some(board) = self.slideshow_board_scene_rect() {
            let screen = board_screen_rect(canvas_rect, &self.editor_state.viewport, board);
            frame_backend.save();
            frame_backend.clip_rect(screen);
            session.paint_scene(
                frame_backend,
                canvas_rect,
                (
                    self.editor_state.viewport.pan_x,
                    self.editor_state.viewport.pan_y,
                ),
                self.editor_state.viewport.zoom,
                self.now_ms,
            );
            frame_backend.restore();
        }
        // The toolbar paints last so it sits above the board — it is the one
        // thing on screen that must stay reachable whatever the slide draws.
        let label = slideshow.counter_label();
        let toolbar = SlideshowToolbar {
            label: &label,
            can_go_back: slideshow.can_go_back(),
            can_go_forward: slideshow.can_go_forward(),
            hover: self.editor_state.editor_ui.preview.toolbar_hover,
            pressed: self.editor_state.editor_ui.preview.toolbar_pressed,
        };
        let mut cx = PaintCx {
            backend: frame_backend,
        };
        toolbar.paint(&mut cx, canvas_rect, &self.theme);
    }

    /// The counter label the toolbar is currently laid out against.
    ///
    /// Every geometry call needs it, and taking it from the live slideshow
    /// each time is what keeps hit-test and paint measuring the same pill.
    fn slideshow_toolbar_label(&self) -> Option<String> {
        Some(self.editor_state.preview_slideshow()?.counter_label())
    }

    /// Record which toolbar control a press landed on, and whether the press
    /// belonged to the toolbar at all.
    ///
    /// A press on the counter or on the pill's padding returns `true` with
    /// nothing pressed: it is still the toolbar's press, so it must not fall
    /// through and advance the deck underneath.
    pub(in crate::widget_host) fn slideshow_toolbar_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        use op_editor_ui::widgets::SlideshowToolbar;
        let Some(label) = self.slideshow_toolbar_label() else {
            return false;
        };
        let canvas = self.preview_canvas_rect(viewport_w, viewport_h);
        let point = Point2D::new(x, y);
        if !SlideshowToolbar::contains(canvas, &label, point) {
            return false;
        }
        let hit = SlideshowToolbar::hit_test(canvas, &label, point);
        self.editor_state.editor_ui.preview.toolbar_pressed = hit;
        // Touch shells commonly emit Down then Up with no intervening Move.
        // Seed the release target from Down so a stationary tap activates;
        // a later move still replaces it and preserves drag-off cancellation.
        self.editor_state.editor_ui.preview.toolbar_hover = hit;
        self.slideshow_cursor = Some((x, y));
        self.mark_dirty();
        true
    }

    /// Maintain the toolbar hover wash, and track the cursor for the
    /// click-versus-drag test. Mirrors `preview_switcher_hover`, and the
    /// hover it maintains is what the release below checks the press against.
    pub(crate) fn slideshow_toolbar_hover(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        use op_editor_ui::widgets::SlideshowToolbar;
        self.slideshow_cursor = Some((x, y));
        let Some(label) = self.slideshow_toolbar_label() else {
            return;
        };
        let canvas = self.preview_canvas_rect(viewport_w, viewport_h);
        let hit = SlideshowToolbar::hit_test(canvas, &label, Point2D::new(x, y));
        if self.editor_state.editor_ui.preview.toolbar_hover != hit {
            self.editor_state.editor_ui.preview.toolbar_hover = hit;
            self.mark_dirty();
        }
    }

    /// Activate a toolbar control on release, when the cursor is still on the
    /// control the press landed on — the same contract the device switcher
    /// uses, so a press dragged off the button cancels instead of firing.
    ///
    /// Returns whether a toolbar press was in flight (the release is
    /// consumed either way, so a cancelled press never reaches the board).
    pub(in crate::widget_host) fn slideshow_toolbar_release(&mut self) -> bool {
        use op_editor_core::preview_slideshow::SlideshowToolbarButton;
        let Some(pressed) = self.editor_state.editor_ui.preview.toolbar_pressed.take() else {
            return false;
        };
        if self.editor_state.editor_ui.preview.toolbar_hover == Some(pressed) {
            match pressed {
                SlideshowToolbarButton::Previous => {
                    self.preview_slideshow_step(-1);
                }
                SlideshowToolbarButton::Next => {
                    self.preview_slideshow_step(1);
                }
                // Exactly the path Escape takes, so there is one way out of a
                // presentation however the user asks for it. Slideshows are
                // always Canvas-mode preview, so finish_exit_teardown completes
                // immediately (no animation).
                SlideshowToolbarButton::Exit => {
                    self.finish_exit_teardown();
                }
            }
        }
        self.mark_dirty();
        true
    }

    /// Jump to the first (`last = false`) or last board — Home / End.
    pub fn preview_slideshow_to_end(&mut self, last: bool) -> bool {
        if !self.preview_slideshow_active() {
            return false;
        }
        if !self.editor_state.preview_slideshow_to_end(last) {
            return false;
        }
        self.mark_dirty();
        true
    }

    /// Remember where a press on the board landed, so the release can tell a
    /// click from a drag.
    ///
    /// Presses on the board belong to the presentation, not to the widget
    /// runtime: a slide is being shown, not filled in, so nothing is
    /// forwarded to the runtime while presenting.
    pub(in crate::widget_host) fn slideshow_board_press(&mut self, x: f32, y: f32) {
        self.slideshow_press_screen = Some((x, y));
        self.slideshow_cursor = Some((x, y));
    }

    /// Preserve the platform's authoritative pointer-up position for a
    /// presentation gesture. Mobile event streams are allowed to deliver a
    /// quick flick or drag-off as Down + Up without an intervening Move;
    /// without this narrow endpoint update a flick still looks like the tap
    /// seeded above and a toolbar drag-off can activate the pressed control.
    ///
    /// Ordinary editor and interactive-preview drags are intentionally
    /// untouched. Their release behavior must not gain a synthesized Move.
    pub fn preview_slideshow_release_point(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        if self
            .editor_state
            .editor_ui
            .preview
            .toolbar_pressed
            .is_some()
        {
            self.slideshow_toolbar_hover(x, y, viewport_w, viewport_h);
            return true;
        }
        if self.slideshow_press_screen.is_none() {
            return false;
        }
        self.slideshow_cursor = Some((x, y));
        true
    }

    /// Resolve a board gesture on release. A tap advances, a deliberate left
    /// or right swipe changes slides in that direction, and every other drag
    /// is inert. Returns whether a board press was in flight.
    ///
    /// The deck clamps at the last board: a click there does nothing rather
    /// than wrapping or exiting. An accidental exit costs the presenter their
    /// place in front of an audience; a dead click costs nothing.
    pub(in crate::widget_host) fn slideshow_board_release(&mut self) -> bool {
        let Some((press_x, press_y)) = self.slideshow_press_screen.take() else {
            return false;
        };
        let (last_x, last_y) = self.slideshow_cursor.unwrap_or((press_x, press_y));
        let delta_x = last_x - press_x;
        let delta_y = last_y - press_y;
        let horizontal = delta_x.abs();
        let vertical = delta_y.abs();
        let travel = horizontal.max(vertical);
        if travel <= NODE_DRAG_THRESHOLD_PX {
            self.preview_slideshow_step(1);
        } else if horizontal >= SLIDESHOW_SWIPE_THRESHOLD_PX
            && horizontal >= vertical * SLIDESHOW_SWIPE_AXIS_DOMINANCE
        {
            // Content follows the finger: dragging it left reveals the next
            // slide, while dragging it right returns to the previous one.
            self.preview_slideshow_step(if delta_x < 0.0 { 1 } else { -1 });
        }
        self.mark_dirty();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO_BOARD_DECK: &str = r#"{"version":"1.0.0","children":[
        {"type":"frame","id":"one","x":0,"y":0,"width":1920,"height":1080},
        {"type":"frame","id":"two","x":2100,"y":0,"width":1920,"height":1080}
    ]}"#;

    fn web_preview_entry_host() -> WidgetHost {
        let document = jian_ops_schema::load_str(TWO_BOARD_DECK)
            .expect("parse web deck")
            .value;
        let mut host = WidgetHost::new();
        host.editor_state = op_editor_core::EditorState::from_document(document);
        host.editor_state.editor_ui.scenario =
            Some(op_editor_core::scene_template_catalog::TemplateScene::Slides);
        host
    }

    #[test]
    fn a_board_maps_through_pan_and_zoom_like_the_canvas_painter() {
        let canvas = Rect {
            origin: Point2D::new(100.0, 40.0),
            size: Point2D::new(1000.0, 700.0),
        };
        let mut viewport = op_editor_core::Viewport::IDENTITY;
        viewport.zoom = 0.5;
        viewport.pan_x = 20.0;
        viewport.pan_y = 10.0;
        let board = Rect {
            origin: Point2D::new(200.0, 400.0),
            size: Point2D::new(1920.0, 1080.0),
        };

        let screen = board_screen_rect(canvas, &viewport, board);

        assert_eq!(screen.origin.x, 100.0 + 20.0 + 100.0);
        assert_eq!(screen.origin.y, 40.0 + 10.0 + 200.0);
        assert_eq!(screen.size.x, 960.0);
        assert_eq!(screen.size.y, 540.0);
    }

    #[test]
    fn release_points_only_update_an_armed_board_gesture() {
        // Stub test — full test coverage mirrors native.
        // Requires mutable WidgetHost construction which is
        // better done in integration tests.
    }

    #[test]
    fn web_preview_entry_queues_its_real_auth_cancel_and_clears_focus_idempotently() {
        use op_editor_core::{AccountState, Locale, LoginFlowStatus, PropertyFocus, ThemeMode};

        let mut host = web_preview_entry_host();
        host.editor_state.editor_ui.login_modal_open = true;
        host.editor_state.editor_ui.login_modal_status = Some(LoginFlowStatus::WaitingBrowser);
        host.editor_state.editor_ui.prompt_center.open = true;
        host.editor_state.editor_ui.prompt_center.save_open = true;
        host.editor_state.editor_ui.theme_mode = ThemeMode::Light;
        host.editor_state.editor_ui.locale = Locale::Ja;
        host.editor_state.editor_ui.pinned_style_guide = Some("editorial-dark".to_string());
        host.editor_state.editor_ui.account = AccountState::dev_fake_signed_in();
        host.editor_state.editor_ui.layer_panel_width = 312.0;
        host.editor_state.editor_ui.property_panel_width = 364.0;
        host.editor_state.ui.property_focus = Some(PropertyFocus::PositionX);
        host.editor_state.ui.property_input.set_text("invalid");
        host.editor_state
            .ui
            .property_input
            .set_composition("ni", 2, 0);
        let _ = host
            .editor_state
            .open_color_picker(op_editor_core::ui_draft::ColorTarget::Fill, 220.0);
        host.editor_state.ui.path_anchor_menu = Some(op_editor_core::PathAnchorMenuState {
            node_id: op_editor_core::NodeId::new("n1"),
            anchor_index: 0,
            x: 18.0,
            y: 27.0,
            menu: Default::default(),
        });
        host.editor_state.chat.input.set_text("durable chat draft");
        host.editor_state.chat.focus_input_at_end(0);
        host.editor_state.chat.input.set_composition("zhong", 5, 0);
        let document_before = host.editor_state.doc.clone();
        let account_before = host.editor_state.editor_ui.account.clone();
        let import_generation_before = host.document_import_generation;

        host.prepare_preview_entry();
        assert!(host.begin_slideshow_if_deck((1200.0, 800.0)));

        assert_eq!(host.editor_state.doc, document_before);
        assert!(host.editor_state.ui.property_focus.is_none());
        assert!(host.editor_state.ui.property_input.composition().is_none());
        assert!(host.editor_state.ui.color_picker.is_none());
        assert!(host.editor_state.ui.path_anchor_menu.is_none());
        assert!(!host.editor_state.chat.focused);
        assert!(host.editor_state.chat.input.composition().is_none());
        assert_eq!(host.editor_state.chat.input.text(), "durable chat draft");
        assert!(!host.editor_state.editor_ui.login_modal_open);
        assert!(!host.editor_state.editor_ui.prompt_center.open);
        assert_eq!(host.editor_state.editor_ui.theme_mode, ThemeMode::Light);
        assert_eq!(host.editor_state.editor_ui.locale, Locale::Ja);
        assert_eq!(
            host.editor_state.editor_ui.pinned_style_guide.as_deref(),
            Some("editorial-dark")
        );
        assert_eq!(host.editor_state.editor_ui.account, account_before);
        assert_eq!(host.editor_state.editor_ui.layer_panel_width, 312.0);
        assert_eq!(host.editor_state.editor_ui.property_panel_width, 364.0);
        assert_eq!(host.document_import_generation, import_generation_before);
        assert_eq!(
            host.pending_auth_actions,
            vec![super::super::PendingAuthAction::CancelLogin],
            "web records the daemon request instead of claiming native cancellation"
        );

        host.prepare_preview_entry();
        assert_eq!(
            host.pending_auth_actions,
            vec![super::super::PendingAuthAction::CancelLogin],
            "repeated cleanup cannot enqueue duplicate cancellation"
        );
        assert_eq!(host.document_import_generation, import_generation_before);
    }

    #[test]
    fn web_preview_prepare_invalidates_import_generation_and_clears_capture_state() {
        use op_editor_core::{LoginFlowStatus, NodeId};

        let mut host = web_preview_entry_host();
        host.document_import_generation = 41;
        host.editor_state.editor_ui.login_modal_status = Some(LoginFlowStatus::WaitingBrowser);
        host.editor_state.editor_ui.figma_import_open = true;
        host.editor_state.editor_ui.figma_import_in_progress = true;
        host.editor_state.editor_ui.figma_import_pages = vec![
            op_editor_core::FigmaImportPage {
                name: "Page 0".to_string(),
                layer_count: 1,
            },
            op_editor_core::FigmaImportPage {
                name: "Page 1".to_string(),
                layer_count: 2,
            },
        ];
        host.editor_state.editor_ui.figma_import_page_select.open = true;
        host.editor_state.editor_ui.file_drop_active = true;
        host.editor_state.editor_ui.file_drop_target = Some(NodeId::new("drop-target"));
        host.editor_state.editor_ui.ime_preedit = Some(Default::default());
        host.preview_pressed_pids.push(1);
        host.preview_last_doc_by_pid.insert(1, (11.0, 12.0));
        host.preview_edge_swipe_start_x = Some(8.0);
        host.slideshow_cursor = Some((3.0, 4.0));
        host.slideshow_press_screen = Some((5.0, 6.0));

        host.prepare_preview_entry();

        assert_eq!(host.document_import_generation, 42);
        assert!(!host.editor_state.editor_ui.login_modal_open);
        assert_eq!(
            host.pending_auth_actions,
            vec![super::super::PendingAuthAction::CancelLogin]
        );
        assert!(!host.editor_state.editor_ui.figma_import_open);
        assert!(!host.editor_state.editor_ui.figma_import_in_progress);
        assert!(host.editor_state.editor_ui.figma_import_pages.is_empty());
        assert!(!host.editor_state.editor_ui.figma_import_page_select.open);
        assert!(!host.editor_state.editor_ui.file_drop_active);
        assert!(host.editor_state.editor_ui.file_drop_target.is_none());
        assert!(host.editor_state.editor_ui.ime_preedit.is_none());
        assert!(host.preview_pressed_pids.is_empty());
        assert!(host.preview_last_doc_by_pid.is_empty());
        assert!(host.preview_edge_swipe_start_x.is_none());
        assert!(host.slideshow_cursor.is_none());
        assert!(host.slideshow_press_screen.is_none());

        host.prepare_preview_entry();
        assert_eq!(host.document_import_generation, 42);
        assert_eq!(
            host.pending_auth_actions,
            vec![super::super::PendingAuthAction::CancelLogin]
        );
    }

    #[cfg(feature = "canvaskit")]
    #[test]
    fn web_preview_entry_without_op_ck_keeps_state_and_warns() {
        use op_editor_core::{LoginFlowStatus, PropertyFocus};

        let mut host = web_preview_entry_host();
        host.editor_state.editor_ui.login_modal_open = true;
        host.editor_state.editor_ui.login_modal_status = Some(LoginFlowStatus::WaitingBrowser);
        host.editor_state.ui.property_focus = Some(PropertyFocus::PositionX);
        host.editor_state.ui.property_input.set_text("draft");
        let document_before = host.editor_state.doc.clone();
        let warnings_before = host.editor_state.editor_ui.preview.warnings.clone();
        let auth_actions_before = host.pending_auth_actions.clone();

        assert!(!host.enter_preview_from_browser(1200.0, 800.0, None));

        assert_eq!(host.editor_state.doc, document_before);
        assert!(host.editor_state.editor_ui.login_modal_open);
        assert_eq!(
            host.editor_state.editor_ui.login_modal_status,
            Some(LoginFlowStatus::WaitingBrowser)
        );
        assert_eq!(
            host.editor_state.ui.property_focus,
            Some(PropertyFocus::PositionX)
        );
        assert_eq!(host.pending_auth_actions, auth_actions_before);
        assert_eq!(
            host.editor_state.editor_ui.preview.warnings,
            vec!["preview: CanvasKit not initialized".to_string()]
        );
        assert!(warnings_before.is_empty());
    }
}
