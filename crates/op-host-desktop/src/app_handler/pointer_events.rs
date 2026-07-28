//! Pointer input handlers (cursor move, mouse buttons, wheel, pinch)
//! for `DesktopApp`'s `window_event` dispatcher. Carved out of the
//! `app_handler.rs` spine to keep it under the 800-line cap; pure code
//! motion.
//!
//! Handlers that used to `return` early out of `window_event` — thereby
//! skipping its post-event epilogue — return `false` here; the
//! dispatcher turns that back into the same early return.

use crate::{chat_attachment, chat_session, figma_import_session, frame, persistence, DesktopApp};
use winit::dpi::PhysicalPosition;
use winit::event::MouseScrollDelta;
use winit::event_loop::ActiveEventLoop;

fn dispatch_background_save_as_if_required(
    outcome: &op_host_services::doc_io::ActionOutcome,
    schedule: impl FnOnce() -> bool,
) -> bool {
    if !matches!(
        outcome,
        op_host_services::doc_io::ActionOutcome::SaveAsForkRequired
    ) {
        return false;
    }
    let _ = schedule();
    true
}

impl DesktopApp {
    pub(super) fn on_cursor_moved(&mut self, position: PhysicalPosition<f64>) -> bool {
        self.cursor_x = position.x as f32 / self.dpi;
        self.cursor_y = position.y as f32 / self.dpi;
        let model_picker_open = self.host.editor_state().editor_ui.chat_model_picker.open;
        let over_layer_panel = !model_picker_open
            && self.host.cursor_over_layer_panel(
                self.cursor_x,
                self.cursor_y,
                self.viewport_width,
                self.viewport_height,
            );
        if let Some(window) = self.window.as_ref() {
            // Borderless (Windows / Linux) windows have no OS-provided
            // edge-resize band — synthesize a resize cursor over the
            // outer ring, ahead of every panel / canvas hint. macOS keeps
            // its native decorations, so it's left untouched.
            #[cfg(not(target_os = "macos"))]
            if !self.host.is_dragging_node() && !window.is_maximized() {
                let vw = window.inner_size().width as f32 / self.dpi;
                let vh = window.inner_size().height as f32 / self.dpi;
                if let Some(dir) = crate::window_resize::window_resize_direction(
                    self.cursor_x,
                    self.cursor_y,
                    vw,
                    vh,
                ) {
                    window.set_cursor(winit::window::CursorIcon::from(dir));
                    self.pending_cursor_move = Some((self.cursor_x, self.cursor_y));
                    self.request_redraw(false);
                    return false;
                }
            }
            if self.host.is_dragging_node() || over_layer_panel {
                window.set_cursor(winit::window::CursorIcon::Default);
            } else {
                // `cursor_hint` hit-tests the layout-resolved render
                // scene. A mutation since the last paint may have left
                // the scene stale (`editor_state_dirty`), so refresh it
                // only when the pointer is outside the LayerPanel and a
                // canvas/overlay hint could need scene geometry.
                let _ = self.host.layout_scene();
                let viewport_w = window.inner_size().width as f32 / self.dpi;
                let viewport_h = window.inner_size().height as f32 / self.dpi;
                let hint =
                    self.host
                        .cursor_hint(self.cursor_x, self.cursor_y, viewport_w, viewport_h);
                use op_host_native::CursorHint;
                if matches!(hint, CursorHint::Rotate) {
                    if let Some(c) = self.rotate_cursor.as_ref() {
                        window.set_cursor(winit::window::Cursor::Custom(c.clone()));
                    } else {
                        window.set_cursor(winit::window::CursorIcon::Grabbing);
                    }
                } else {
                    let icon = match hint {
                        CursorHint::Default => winit::window::CursorIcon::Default,
                        CursorHint::Pointer => winit::window::CursorIcon::Pointer,
                        CursorHint::NotAllowed => winit::window::CursorIcon::NotAllowed,
                        CursorHint::Move => winit::window::CursorIcon::Move,
                        CursorHint::Grab => winit::window::CursorIcon::Grab,
                        CursorHint::Grabbing => winit::window::CursorIcon::Grabbing,
                        CursorHint::Crosshair => winit::window::CursorIcon::Crosshair,
                        CursorHint::Text => winit::window::CursorIcon::Text,
                        CursorHint::ResizeEw => winit::window::CursorIcon::EwResize,
                        CursorHint::ResizeNs => winit::window::CursorIcon::NsResize,
                        CursorHint::ResizeNwse => winit::window::CursorIcon::NwseResize,
                        CursorHint::ResizeNesw => winit::window::CursorIcon::NeswResize,
                        CursorHint::Rotate => unreachable!(),
                    };
                    window.set_cursor(icon);
                }
            }
        }
        // Coalesce cursor moves — apply once per redraw, not per 1000 Hz input event.
        self.pending_cursor_move = Some((self.cursor_x, self.cursor_y));
        self.request_redraw(false);
        true
    }

    pub(super) fn on_left_press(&mut self, event_loop: &ActiveEventLoop) -> bool {
        // Drain queued cursor move so hover state is current before press lands.
        if self.drain_pending_cursor_move() {
            self.redraw_dirty = true;
        }
        // Borderless-window edge resize (Windows / Linux). A press on the
        // outer ring hands the drag to the OS, ahead of the TopBar
        // window-drag and every app hit-test. macOS keeps native edges.
        #[cfg(not(target_os = "macos"))]
        if let Some(w) = self.window.as_ref() {
            if !w.is_maximized() {
                let vw = w.inner_size().width as f32 / self.dpi;
                let vh = w.inner_size().height as f32 / self.dpi;
                if let Some(dir) = crate::window_resize::window_resize_direction(
                    self.cursor_x,
                    self.cursor_y,
                    vw,
                    vh,
                ) {
                    let _ = w.drag_resize_window(dir);
                    return false;
                }
            }
        }
        // Custom window chrome — the native title bar is
        // hidden, so a press on the TopBar's window-control
        // dots drives the window, and a press on the bar's
        // blank area starts a window drag.
        if self.cursor_y < op_editor_ui::widgets::TOP_BAR_HEIGHT {
            use op_editor_ui::widgets::{TopBar, WindowControl};
            use op_editor_ui::{Point2D, Rect};
            let tb_rect = Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(self.viewport_width, op_editor_ui::widgets::TOP_BAR_HEIGHT),
            };
            let tb = TopBar::for_editor_ui(&self.host.editor_state().editor_ui);
            let p = Point2D::new(self.cursor_x, self.cursor_y);
            if let Some(ctl) = tb.window_control_at(tb_rect, p) {
                match ctl {
                    WindowControl::Close => {
                        // Mirror `WindowEvent::CloseRequested`:
                        // finalize-lifecycle invariant BEFORE the
                        // unsaved-changes prompt (same ordering
                        // rationale as that handler), the prompt can
                        // abort, and settings flush + save before
                        // exit — a bare `exit()` would drop work.
                        chat_session::finalize_design_session_if_needed(
                            &mut self.host,
                            &self.current_chat,
                            "teardown-backstop",
                        );
                        if self.confirm_close() {
                            self.host.flush_settings_input();
                            op_host_services::settings_io::save(self.host.editor_state());
                            event_loop.exit();
                        }
                    }
                    WindowControl::Minimize => {
                        if let Some(w) = self.window.as_ref() {
                            w.set_minimized(true);
                        }
                    }
                    WindowControl::Maximize => {
                        if let Some(w) = self.window.as_ref() {
                            w.set_maximized(!w.is_maximized());
                        }
                    }
                }
                return false;
            }
            // A press on the bar that hits none of the app's
            // own buttons is a window-drag grab.
            if tb.hit_test(tb_rect, p).is_none() {
                if let Some(w) = self.window.as_ref() {
                    let _ = w.drag_window();
                }
                return false;
            }
        }
        // One pointer gesture is one collaboration transaction. The runtime
        // snapshots immediately before the host can mutate the document;
        // ordered remote frames queue until the matching release below.
        self.collab_runtime.begin_local_edit(&mut self.host);
        let consumed = self.host.apply_press(
            self.cursor_x,
            self.cursor_y,
            self.viewport_width,
            self.viewport_height,
        );
        // Press may focus/blur an input or reposition its caret.
        // Publish the anchor before enabling composition.
        self.sync_native_ime();
        // TopBar fullscreen button raised an intent — toggle the
        // real window through the shared menu-action path.
        if self.host.editor_state().editor_ui.pending_fullscreen_toggle {
            self.host
                .editor_state_mut()
                .editor_ui
                .pending_fullscreen_toggle = false;
            self.handle_menu_action(crate::menu::MenuAction::ToggleFullscreen, event_loop);
        }
        if let Some(text) = self.host.editor_state_mut().chat.pending_copy_text.take() {
            crate::clipboard::set_text(&text);
        }
        // A click on the chat Send button raises
        // `pending_send` — launch the provider turn.
        if self.launch_chat_if_pending() {
            self.request_redraw(true);
        }
        if self.drain_new_chat() {
            self.request_redraw(true);
        }
        if self.drain_stop_chat() {
            self.request_redraw(true);
        }
        if self.drain_close_chat_tab() {
            self.request_redraw(true);
        }
        // A click on the attach button raises
        // `pending_attachment_pick` — open the file picker.
        if chat_attachment::drain_attachment_pick(&mut self.host) {
            self.request_redraw(true);
        }
        // #20 theme presets — drain the preset dropdown's
        // pending `.optheme` import / export (blocking rfd
        // dialog, like `persistence::run_action` below).
        if crate::theme_preset_host::drain_preset_io(&mut self.host) {
            self.request_redraw(true);
        }
        // Font import / removal raised by the property-panel
        // font picker — open the rfd dialog / run FontStore IO,
        // then refresh the picker's imported-family snapshot.
        let font_request_ran = crate::font_import_host::drain_font_requests(&mut self.host);
        if font_request_ran {
            self.host.refresh_missing_fonts_prompt();
            self.request_redraw(true);
        }
        let missing_fonts_detection_ready = {
            let ui = &self.host.editor_state().editor_ui;
            ui.missing_fonts_pending_detect && ui.system_fonts_loaded
        };
        if missing_fonts_detection_ready {
            self.host.complete_pending_missing_fonts_detection();
            self.request_redraw(true);
        }
        if let Some(action) = self
            .host
            .editor_state_mut()
            .editor_ui
            .pending_file_action
            .take()
        {
            // ExportImage → close source overlay + open picker dialog.
            if matches!(
                action,
                op_editor_core::editor_ui_state::FileAction::ExportImage
            ) {
                let eui = &mut self.host.editor_state_mut().editor_ui;
                eui.file_menu_open = false;
                eui.file_menu.hover = None;
                eui.image_panel.close_popovers();
                eui.export_dialog_open = true;
                self.host.mark_editor_state_dirty();
                self.request_redraw(true);
            } else if matches!(action, op_editor_core::editor_ui_state::FileAction::Save) {
                self.host.commit_variable_row_focus_if_any_pub();
                self.request_background_save();
            } else {
                // The file menu just closed (file_menu_open=false set
                // in dispatch_file_menu_press), but `run_action` opens
                // a BLOCKING native rfd dialog. `request_redraw` only
                // defers a repaint to the next frame, which never
                // arrives before the modal — so paint one synchronous
                // frame here to actually dismiss the menu before the
                // dialog covers it.
                if let (Some(ctx), Some(backend)) = (self.ctx.as_mut(), self.backend.as_mut()) {
                    frame::paint(
                        ctx,
                        backend,
                        &mut self.host,
                        self.viewport_width,
                        self.viewport_height,
                        self.dpi,
                    );
                }
                if matches!(action, op_editor_core::editor_ui_state::FileAction::SaveAs) {
                    self.host.commit_variable_row_focus_if_any_pub();
                    self.request_background_save_as();
                } else {
                    let outcome = persistence::run_action(
                        action,
                        &mut self.host,
                        &mut self.current_path,
                        self.window.as_ref(),
                    );
                    if !dispatch_background_save_as_if_required(&outcome, || {
                        self.request_background_save_as()
                    }) {
                        match outcome {
                            // `mark_document_saved` cancels any
                            // in-flight Figma import internally, so a
                            // stale worker can't overwrite the fresh
                            // document when its result lands.
                            op_host_services::doc_io::ActionOutcome::Saved => {
                                self.mark_document_saved()
                            }
                            // User picked a `.fig`; confirm its output,
                            // then replace any prior import session and let
                            // `pump` apply the document once parsing finishes.
                            op_host_services::doc_io::ActionOutcome::FigmaImportStarted(path) => {
                                let _ = self.begin_figma_import(path);
                            }
                            op_host_services::doc_io::ActionOutcome::FigmaImportSelection(
                                selection,
                            ) => {
                                if figma_import_session::finish_selection(
                                    &mut self.host,
                                    &mut self.current_figma_import,
                                    selection,
                                ) {
                                    self.request_redraw(true);
                                }
                            }
                            // User picked a saved page or ZIP project; same
                            // background session discipline as the Figma branch.
                            op_host_services::doc_io::ActionOutcome::HtmlImportStarted(path) => {
                                let _ = self.begin_html_import(path);
                            }
                            op_host_services::doc_io::ActionOutcome::SaveAsForkRequired => {
                                unreachable!("required Save As fork was dispatched above")
                            }
                            op_host_services::doc_io::ActionOutcome::Noop => {}
                        }
                    }
                }
            }
        }
        // Deferred press actions above can also close an input.
        self.sync_native_ime();
        if consumed {
            self.request_redraw(true);
        }
        true
    }

    pub(super) fn on_right_press(&mut self) {
        if self.drain_pending_cursor_move() {
            self.redraw_dirty = true;
        }
        let consumed = self.host.apply_right_press(
            self.cursor_x,
            self.cursor_y,
            self.viewport_width,
            self.viewport_height,
        );
        self.sync_native_ime();
        if consumed {
            self.request_redraw(true);
        }
    }

    pub(super) fn on_middle_press(&mut self) {
        if self.drain_pending_cursor_move() {
            self.redraw_dirty = true;
        }
        // Middle-button drag pans regardless of the active
        // tool (TS parity: e.button === 1 starts a pan).
        if self.host.apply_pan_press(self.cursor_x, self.cursor_y) {
            self.request_redraw(true);
        }
    }

    pub(super) fn on_middle_release(&mut self) {
        if self.drain_pending_cursor_move() {
            self.redraw_dirty = true;
        }
        let consumed = self
            .host
            .apply_release_with_viewport(self.viewport_width, self.viewport_height);
        if consumed {
            self.request_redraw(true);
        }
    }

    pub(super) fn on_left_release(&mut self) {
        // Drain pending cursor moves before release so drag-end commits final position.
        if self.drain_pending_cursor_move() {
            self.redraw_dirty = true;
        }
        let consumed = self
            .host
            .apply_release_with_viewport(self.viewport_width, self.viewport_height);
        self.collab_runtime.finish_local_edit(&mut self.host);
        self.sync_native_ime();
        if consumed {
            self.request_redraw(true);
        }
    }

    pub(super) fn on_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        // Figma-style routing: pixel-delta pans, line-delta zooms; Cmd promotes pixel→zoom for trackpad-only laptops
        // a pinch sensor.
        let consumed = match delta {
            MouseScrollDelta::LineDelta(_, y) => self.host.apply_wheel(
                self.cursor_x,
                self.cursor_y,
                y * 16.0,
                self.viewport_width,
                self.viewport_height,
            ),
            MouseScrollDelta::PixelDelta(p) => {
                let dx = p.x as f32 / self.dpi;
                let dy = p.y as f32 / self.dpi;
                if self.zoom_modifier {
                    self.host.apply_pinch_gesture(
                        self.cursor_x,
                        self.cursor_y,
                        dy,
                        self.viewport_width,
                        self.viewport_height,
                    )
                } else {
                    self.host.apply_pan_gesture(
                        self.cursor_x,
                        self.cursor_y,
                        dx,
                        dy,
                        self.viewport_width,
                        self.viewport_height,
                    )
                }
            }
        };
        if consumed {
            self.request_redraw(true);
        }
    }

    pub(super) fn on_pinch_gesture(&mut self, delta: f64) {
        let consumed = self.host.apply_pinch_gesture(
            self.cursor_x,
            self.cursor_y,
            delta as f32 * 400.0,
            self.viewport_width,
            self.viewport_height,
        );
        if consumed {
            self.request_redraw(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::dispatch_background_save_as_if_required;
    use op_host_services::doc_io::ActionOutcome;

    #[test]
    fn typed_save_as_fork_outcome_invokes_the_background_dispatcher() {
        let mut scheduled = 0;
        assert!(dispatch_background_save_as_if_required(
            &ActionOutcome::SaveAsForkRequired,
            || {
                scheduled += 1;
                true
            }
        ));
        assert_eq!(scheduled, 1);
        assert!(dispatch_background_save_as_if_required(
            &ActionOutcome::SaveAsForkRequired,
            || {
                scheduled += 1;
                false
            }
        ));
        assert_eq!(
            scheduled, 2,
            "picker cancellation is still a handled dispatch, not a sync fallback"
        );
        assert!(!dispatch_background_save_as_if_required(
            &ActionOutcome::Noop,
            || {
                scheduled += 1;
                true
            }
        ));
        assert_eq!(scheduled, 2);
    }
}
