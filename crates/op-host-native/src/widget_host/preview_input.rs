//! Live-preview lifecycle + input routing on `WidgetHostNative`.
//!
//! Toggling preview mode, resizing the preview surface, and forwarding
//! screen-space pointer / wheel / key events into the preview runtime.
//! Split out of the `widget_host.rs` spine to keep it under the repo's
//! 800-line cap; the runtime itself lives in `preview/`.

use super::*;

/// Pointer id the legacy mouse wrappers dispatch under (desktop preview
/// panels speak mouse; real ids ride the `_id` variants).
pub(in crate::widget_host) const LEGACY_PREVIEW_POINTER_ID: u32 = 1;

impl WidgetHostNative {
    /// True while ANY preview pointer is between Down and Up.
    #[cfg(any(test, feature = "testing"))]
    pub fn preview_any_pointer_held_for_test(&self) -> bool {
        !self.preview_pressed_pids.is_empty()
    }

    /// The factual timestamp for the live-preview pointer event being
    /// dispatched: the scoped event time when a timestamped entry
    /// variant (`apply_press_at` / `apply_cursor_move_at` /
    /// `apply_release_with_viewport_at` / `cancel_native_touch_gestures_at`)
    /// supplied one, else the host's global clock (`now_ms`). The
    /// global clock and the event time are deliberately independent:
    /// an out-of-order event (frame at 2000, Down at 950) keeps the
    /// clock at 2000 while the Swipe recognizer measures the factual
    /// 100 ms pair delta.
    pub(in crate::widget_host) fn preview_pointer_time_ms(&self) -> u64 {
        self.preview_event_time_ms.unwrap_or(self.now_ms)
    }

    /// Narrow test-only snapshot of one `$app` value from the live
    /// preview runtime. CLONED out of `PreviewSession` — the runtime's
    /// state is interior-mutable, so production callers never receive a
    /// reference into it. Compiled only when the `testing` feature is
    /// enabled (op-engine-ffi / op-host-native test builds).
    #[cfg(feature = "testing")]
    pub fn preview_app_state_value_for_test(
        &self,
        key: &str,
    ) -> Option<jian_core::value::RuntimeValue> {
        self.preview.as_ref()?.app_state_value_for_test(key)
    }

    /// Center the canvas viewport on a scene-space `rect`, keeping zoom
    /// unchanged. Used by APP MODE preview to keep the mounted screen
    /// framed on entry and after a screen-switch reconcile. `viewport_w`
    /// / `viewport_h` are the values the paint path already threads
    /// (the host has no cached viewport-size field of its own —
    /// `last_viewport_w/h` is refreshed only on press, so it can be
    /// stale for a frame at enter time).
    pub(in crate::widget_host) fn center_canvas_on(
        &mut self,
        rect: op_editor_ui::Rect,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        let (_cx0, _cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let vp = &mut self.editor_state.viewport;
        vp.pan_x = cw / 2.0 - (rect.origin.x + rect.size.x / 2.0) * vp.zoom;
        vp.pan_y = ch / 2.0 - (rect.origin.y + rect.size.y / 2.0) * vp.zoom;
        self.mark_dirty();
    }

    /// Toggle Preview mode. `canvas_size` is the logical canvas region
    /// (used only on enter). Returns the new state (`true` = in preview).
    pub fn toggle_preview(&mut self, canvas_size: (f32, f32)) -> bool {
        if self.preview_active() {
            self.exit_preview();
            false
        } else {
            self.enter_preview(canvas_size)
        }
    }

    /// Track C-6 (`Cmd+P`): [`Self::toggle_preview`] using the host's own
    /// cached viewport (`last_viewport_w/h`) instead of a caller-supplied
    /// size — the keyboard shortcut's entry point (`op-host-desktop`,
    /// which has no access to this crate's private canvas-region math)
    /// is otherwise identical to the TopBar Play button's `press.rs` call
    /// site. `last_viewport_w/h` can be a frame stale right after a
    /// resize (same caveat `center_canvas_on`'s doc already accepts for
    /// this cache), which only affects entry centering, never whether
    /// preview toggles.
    pub fn toggle_preview_with_cached_viewport(&mut self) -> bool {
        let (_cx0, _cy0, cw, ch) = self.canvas_region(self.last_viewport_w, self.last_viewport_h);
        self.toggle_preview((cw, ch))
    }

    /// Resize hook called from the desktop runner's `Resized` handler.
    /// Preview layout is now derived per-root from the document (not the
    /// canvas region), so resizing only changes the paint transform, not
    /// the flex solve — `PreviewSession::resize` is itself a no-op.
    /// Returns early when not in preview.
    pub fn preview_resize(&mut self, viewport_w: f32, viewport_h: f32) {
        self.cache_preview_viewport(viewport_w, viewport_h);
        if self.preview.is_none() {
            return;
        }
        let (_cx0, _cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        if let Some(preview) = self.preview.as_mut() {
            preview.resize((cw, ch));
        }
        self.recompute_device_frame(viewport_w, viewport_h);
    }

    /// Route a printable character into the live preview runtime.
    /// Returns `true` when consumed by a focused widget. No-op (false)
    /// when not in preview.
    pub fn preview_dispatch_text(&mut self, text: &str) -> bool {
        let consumed = self.preview.as_mut().is_some_and(|p| p.dispatch_text(text));
        if consumed {
            self.mark_dirty();
        }
        consumed
    }

    /// Screen → document-space point when preview is active and the
    /// point is inside the canvas region; `None` otherwise.
    fn preview_doc_point(
        &self,
        screen_x: f32,
        screen_y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<op_editor_ui::Point2D> {
        self.preview.as_ref()?;
        if self.device_mode_active() {
            // Device mode fails closed: a missing frame must never fall
            // through to the editor viewport inverse.
            return self.device_preview_doc_point(screen_x, screen_y);
        }
        canvas_geometry::canvas_doc_point(
            &self.editor_state,
            screen_x,
            screen_y,
            viewport_w,
            viewport_h,
        )
    }

    /// Route a screen-space press into the live preview runtime as a
    /// pointer Down; the matching Up arrives via
    /// [`Self::preview_dispatch_release`], with Moves in between so
    /// drags (slider knobs) work. No-op (false) when not in preview or
    /// the press is outside the canvas region.
    pub fn preview_dispatch_press(
        &mut self,
        screen_x: f32,
        screen_y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        self.preview_dispatch_press_id(
            LEGACY_PREVIEW_POINTER_ID,
            screen_x,
            screen_y,
            viewport_w,
            viewport_h,
        )
    }

    /// [`Self::preview_dispatch_press`] carrying an explicit POINTER ID
    /// (R4): concurrent pointers hold independent drags and reach the
    /// runtime with distinct identities.
    pub fn preview_dispatch_press_id(
        &mut self,
        pointer_id: u32,
        screen_x: f32,
        screen_y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        use jian_core::gesture::pointer::{PointerKind, PointerPhase};
        // Track M-1: the canvas/device-frame rect is physically moving
        // mid-merge — same "discard, don't queue" call
        // `PreviewSession::transition_active` makes for a screen-switch
        // slide, for the same reason (no stable target to land on).
        if self.mode_transition_active() {
            return false;
        }
        let Some(doc) = self.preview_doc_point(screen_x, screen_y, viewport_w, viewport_h) else {
            return false;
        };
        self.capture_device_preview_surface(screen_x, screen_y);
        if !self.preview_pressed_pids.contains(&pointer_id) {
            self.preview_pressed_pids.push(pointer_id);
        }
        self.preview_last_doc_by_pid
            .insert(pointer_id, (doc.x, doc.y));
        // Track C-4: arm an edge-swipe candidate when THIS press started
        // in the device frame's left-edge dead zone AND it is the first
        // finger down — a second finger may never arm or fire the pop.
        // Forwarding Down to the runtime as usual is deliberate: a
        // genuine edge-swipe moves well past the tap-gesture's own
        // same-spot Down/Up tolerance before the pop fires, so it never
        // completes as a stray tap.
        if self.preview_pressed_pids.len() == 1 {
            self.arm_edge_swipe_candidate(screen_x);
            self.preview_edge_swipe_pid = Some(pointer_id);
        }
        let t_ms = self.preview_pointer_time_ms();
        let handled = self.preview.as_mut().is_some_and(|p| {
            p.dispatch_pointer_for_id_at(
                pointer_id,
                PointerKind::Mouse,
                doc.x,
                doc.y,
                PointerPhase::Down,
                t_ms,
            )
        });
        self.mark_dirty();
        handled
    }

    /// Route a cursor move into the live preview runtime — a drag
    /// (`Move`) while the preview press is held, a `Hover` otherwise.
    /// Returns `true` (move consumed) only when the point is on-canvas
    /// AND not over a floating overlay panel, so top-bar hover and
    /// floating-panel hover still work while previewing. (Other
    /// floating chrome — e.g. the chat panel — is already
    /// non-interactive in preview: `press.rs` swallows all non-topbar
    /// presses, so preview owning its hover is consistent.)
    pub fn preview_dispatch_move(&mut self, screen_x: f32, screen_y: f32) -> bool {
        self.preview_dispatch_move_id(LEGACY_PREVIEW_POINTER_ID, screen_x, screen_y)
    }

    /// [`Self::preview_dispatch_move`] for an explicit pointer id: only
    /// THAT pointer's held state decides drag-vs-hover, so one finger's
    /// lift cannot turn another finger's drag into hovers mid-stroke.
    pub fn preview_dispatch_move_id(
        &mut self,
        pointer_id: u32,
        screen_x: f32,
        screen_y: f32,
    ) -> bool {
        use jian_core::gesture::pointer::{PointerKind, PointerPhase};
        let (vw, vh) = (self.last_viewport_w, self.last_viewport_h);
        if self.over_topmost_panel(screen_x, screen_y, vw, vh) {
            return false;
        }
        let Some(doc) = self.preview_doc_point(screen_x, screen_y, vw, vh) else {
            return false;
        };
        // Track C-4: a held drag that crosses the edge-swipe threshold
        // fires `pop` and cancels the underlying gesture instead of
        // completing as a normal Move — checked BEFORE updating the last
        // documented point so the cancel dispatches at the gesture's last
        // real position, not this frame's.
        if self.preview_pressed_pids.contains(&pointer_id)
            && self.preview_edge_swipe_pid == Some(pointer_id)
            && self.maybe_fire_edge_swipe(screen_x)
        {
            self.cancel_preview_gesture_for_edge_swipe();
            self.mark_dirty();
            return true;
        }
        let phase = if self.preview_pressed_pids.contains(&pointer_id) {
            PointerPhase::Move
        } else {
            PointerPhase::Hover
        };
        self.preview_last_doc_by_pid
            .insert(pointer_id, (doc.x, doc.y));
        let t_ms = self.preview_pointer_time_ms();
        let emitted = self.preview.as_mut().is_some_and(|p| {
            p.dispatch_pointer_for_id_at(pointer_id, PointerKind::Mouse, doc.x, doc.y, phase, t_ms)
        });
        if emitted || self.preview_pressed_pids.contains(&pointer_id) {
            self.mark_dirty();
        }
        true
    }

    /// Complete a preview drag: pointer Up at the last known document
    /// point. Returns `true` when a preview press was in flight (the
    /// release is consumed).
    pub fn preview_dispatch_release(&mut self) -> bool {
        self.preview_dispatch_release_pid(LEGACY_PREVIEW_POINTER_ID)
    }

    /// [`Self::preview_dispatch_release`] for one explicit pointer id:
    /// completes THAT pointer's drag at its own last documented point
    /// without touching other concurrent pointers' streams.
    pub fn preview_dispatch_release_pid(&mut self, pointer_id: u32) -> bool {
        use jian_core::gesture::pointer::{PointerKind, PointerPhase};
        self.preview_surface_capture = None;
        if !self.preview_pressed_pids.contains(&pointer_id)
            && LEGACY_PREVIEW_POINTER_ID == pointer_id
        {
            // Preserve the legacy disarm-on-any-release behavior for the
            // mouse path even when nothing is pressed.
            self.disarm_edge_swipe();
        }
        let removed = self
            .preview_pressed_pids
            .iter()
            .position(|p| *p == pointer_id)
            .map(|idx| self.preview_pressed_pids.remove(idx))
            .is_some();
        if !removed {
            return false;
        }
        self.disarm_edge_swipe_when_all_released();
        if let Some((x, y)) = self.preview_last_doc_by_pid.remove(&pointer_id) {
            let t_ms = self.preview_pointer_time_ms();
            if let Some(p) = self.preview.as_mut() {
                p.dispatch_pointer_for_id_at(
                    pointer_id,
                    PointerKind::Mouse,
                    x,
                    y,
                    PointerPhase::Up,
                    t_ms,
                );
            }
        }
        self.mark_dirty();
        true
    }

    /// Track C-4 companion: once no pointer remains held, a gesture that
    /// never crossed the threshold must not leak into the NEXT press.
    fn disarm_edge_swipe_when_all_released(&mut self) {
        if self.preview_pressed_pids.is_empty() {
            self.disarm_edge_swipe();
        }
    }

    /// Route a wheel / trackpad-pan scroll into the preview runtime;
    /// `false` (not consumed — no `onScroll` node under the cursor)
    /// lets the caller fall back to canvas pan/zoom so the user can
    /// still navigate while previewing. Mouse wheels carry only
    /// `delta_y`; two-finger trackpad pans carry both axes.
    pub fn preview_dispatch_wheel(
        &mut self,
        screen_x: f32,
        screen_y: f32,
        delta_x: f32,
        delta_y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let Some(doc) = self.preview_doc_point(screen_x, screen_y, viewport_w, viewport_h) else {
            return false;
        };
        let consumed = self
            .preview
            .as_mut()
            .is_some_and(|p| p.dispatch_wheel(doc.x, doc.y, delta_x, delta_y));
        if consumed {
            self.mark_dirty();
        }
        consumed
    }

    /// Advance focus to the next (`shift=false`) / previous
    /// (`shift=true`) focusable widget in the live preview runtime —
    /// Tab / Shift+Tab. Lets the user reach a text input without
    /// clicking (the desktop runner otherwise drops Tab as a control
    /// char). Returns `true` while in preview so the caret repaints.
    pub fn preview_focus(&mut self, shift: bool) -> bool {
        let Some(preview) = self.preview.as_mut() else {
            return false;
        };
        if shift {
            preview.focus_previous();
        } else {
            preview.focus_next();
        }
        self.mark_dirty();
        true
    }

    /// Route a named key into the live preview runtime. `shift` drives
    /// Shift+Tab focus traversal + selection. Returns `true` when the
    /// runtime emitted any semantic event.
    pub fn preview_dispatch_key(&mut self, key: &str, shift: bool) -> bool {
        use jian_core::gesture::pointer::Modifiers;
        // Presenting a deck: the arrow keys move through the slides. They
        // are checked before the runtime sees them because during a
        // presentation that IS what they mean — a slide's widgets are being
        // shown, not filled in.
        if self.preview_slideshow_active() {
            match key {
                // Keynote's conventions: either axis steps the deck, so a
                // presenter's muscle memory works whichever key they reach
                // for.
                "ArrowRight" | "ArrowDown" => {
                    self.preview_slideshow_step(1);
                    return true;
                }
                "ArrowLeft" | "ArrowUp" => {
                    self.preview_slideshow_step(-1);
                    return true;
                }
                _ => {}
            }
        }
        let mods = if shift {
            Modifiers::SHIFT
        } else {
            Modifiers::empty()
        };
        let handled = self
            .preview
            .as_mut()
            .is_some_and(|p| p.dispatch_key(key, mods));
        // Tab traversal / text edits change focus or content even when
        // no semantic event fires, so always repaint while in preview.
        if self.preview.is_some() {
            self.mark_dirty();
        }
        handled
    }
}
