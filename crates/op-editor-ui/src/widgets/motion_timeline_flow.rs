//! Input flow of the motion timeline panel, shared by both hosts: a press
//! resolves through the layout walk paint uses, drags follow the pointer
//! through `op_editor_core`'s anchored, snapping transitions, and a
//! release writes the draft as one undo step. Hosts only route events
//! here and repaint.

use crate::widgets::host_canvas_geometry::{
    canvas_region, motion_timeline_rect, motion_timeline_shown, property_panel_rect,
};
use crate::widgets::motion_timeline::{MotionTimelineView, TimelineHit, TimelineLayout};
use crate::Point2D;
use jian_core::CursorHint;
use op_editor_core::editor_ui_state::{
    MotionTimelineDrag, MotionTimelineTab, TimelineDragAnchor, TimelineKeyframeRef,
};
use op_editor_core::motion_timeline_projection::TimelineStrip;
use op_editor_core::EditorState;

/// How close, in pixels, a dragged time must come to a keyframe or the
/// playhead to snap to it.
pub const SNAP_PX: f32 = 6.0;
/// Factor one press of the zoom buttons scales the time axis by.
pub const ZOOM_STEP: f32 = 1.25;

/// The panel's rect and layout for this frame, when it is shown.
fn layout(
    state: &EditorState,
    viewport_w: f32,
    viewport_h: f32,
) -> Option<(MotionTimelineView, TimelineLayout)> {
    let rect = motion_timeline_rect(state, viewport_w, viewport_h)?;
    let view = MotionTimelineView::for_editor(state);
    let layout = TimelineLayout::new(rect, &view);
    Some((view, layout))
}

/// Route a press. Returns whether it landed on the panel (and so is
/// consumed); `additive` (Shift) toggles keyframes in the selection. A
/// press elsewhere drops the keyframe selection, so Delete goes back to
/// the canvas, except in the Inspector, which edits the selected
/// keyframe's values. An open screen dropdown closes on any press off its
/// rows.
pub fn press(
    state: &mut EditorState,
    point: Point2D,
    viewport_w: f32,
    viewport_h: f32,
    now_ms: u64,
    additive: bool,
) -> bool {
    let hit = layout(state, viewport_w, viewport_h)
        .and_then(|(view, layout)| Some((layout.hit(point, &view)?, view, layout)));
    let menu_was_open = std::mem::take(&mut state.editor_ui.motion_timeline.scope_menu_open);
    let Some((hit, view, layout)) = hit else {
        let in_inspector = state.property_panel_visible()
            && property_panel_rect(state, viewport_w, viewport_h).contains(point);
        if !in_inspector {
            state.clear_motion_keyframe_selection();
        }
        return false;
    };
    let pointer_ms = layout.ms_for_x(point.x);
    match hit {
        TimelineHit::ResizeEdge => {
            state.begin_motion_timeline_drag(MotionTimelineDrag::Height {
                pointer_y: point.y,
                height: layout.rect.size.y,
            });
        }
        TimelineHit::Scope => {
            state.editor_ui.motion_timeline.scope_menu_open = !menu_was_open;
        }
        TimelineHit::ScopeItem(id) => state.pick_motion_timeline_scope(&id),
        TimelineHit::Expand { node_id, animation } => {
            state
                .editor_ui
                .motion_timeline
                .toggle_expanded(&node_id, animation);
        }
        TimelineHit::Label(node_id) => state.select_motion_lane(&node_id),
        TimelineHit::Tab(MotionTimelineTab::Manual) => {
            let next = next_manual_id(&view);
            state.set_motion_timeline_tab(MotionTimelineTab::Manual, next);
        }
        TimelineHit::Tab(tab) => {
            if tab != view.tab {
                state.set_motion_timeline_tab(tab, None);
            }
        }
        TimelineHit::PlayPause => state.toggle_motion_playback(now_ms),
        TimelineHit::Loop => state.toggle_motion_loop(),
        TimelineHit::ZoomOut => zoom_by(state, 1.0 / ZOOM_STEP),
        TimelineHit::ZoomIn => zoom_by(state, ZOOM_STEP),
        TimelineHit::Ruler => {
            state.begin_motion_timeline_drag(MotionTimelineDrag::Playhead);
            state.update_motion_timeline_drag(pointer_ms, now_ms);
        }
        TimelineHit::Keyframe(key) => press_keyframe(state, &view, key, pointer_ms, additive),
        TimelineHit::StripEnd { node_id, animation } => {
            state.select_motion_lane(&node_id);
            if let Some(strip) = strip(&view, &node_id, animation) {
                let end_ms = strip.start_ms + strip.iteration_ms;
                state.begin_motion_timeline_drag_at(
                    MotionTimelineDrag::StripEnd {
                        node_id,
                        animation,
                        end_ms,
                    },
                    TimelineDragAnchor {
                        pointer_ms,
                        held_ms: end_ms,
                    },
                );
            }
        }
        TimelineHit::Strip { node_id, animation } => {
            if state.press_motion_strip(&node_id, animation, now_ms) {
                focus_inspector(state, &node_id, animation, viewport_w, viewport_h);
                return true;
            }
            state.select_motion_lane(&node_id);
            if let Some(strip) = strip(&view, &node_id, animation) {
                let held_ms = strip.start_ms;
                state.begin_motion_timeline_drag_at(
                    MotionTimelineDrag::Strip {
                        node_id,
                        animation,
                        delta_ms: 0.0,
                    },
                    TimelineDragAnchor {
                        pointer_ms,
                        held_ms,
                    },
                );
            }
        }
        TimelineHit::Background => {
            if !additive {
                state.clear_motion_keyframe_selection();
            }
        }
    }
    true
}

/// A pressed keyframe joins (or, with `additive`, toggles in) the
/// selection; when it ends up selected, the whole selection starts moving.
fn press_keyframe(
    state: &mut EditorState,
    view: &MotionTimelineView,
    key: TimelineKeyframeRef,
    pointer_ms: f64,
    additive: bool,
) {
    // Pressing a keyframe that is already selected keeps the selection,
    // so several selected keyframes can be dragged together.
    if additive || !view.is_selected(&key) {
        state.select_motion_keyframe(key.clone(), additive);
    }
    // The Inspector shows the keyframe's node (and so its values).
    state.select_motion_lane(&key.node_id);
    let keys = state.editor_ui.motion_timeline.selection.clone();
    if !keys.contains(&key) {
        return;
    }
    let Some(held_ms) = strip(view, &key.node_id, key.animation)
        .and_then(|strip| strip.keyframe_ms.get(key.keyframe).copied())
    else {
        return;
    };
    state.begin_motion_timeline_drag_at(
        MotionTimelineDrag::Keyframes {
            keys,
            delta_ms: 0.0,
        },
        TimelineDragAnchor {
            pointer_ms,
            held_ms,
        },
    );
}

/// Double-clicking a strip shows its animation in the Inspector: the node
/// selected, its Motion row open and scrolled into view.
fn focus_inspector(
    state: &mut EditorState,
    node_id: &str,
    animation: usize,
    viewport_w: f32,
    viewport_h: f32,
) {
    state.cancel_motion_timeline_drag();
    state.focus_motion_animation(node_id, animation);
    let rect = property_panel_rect(state, viewport_w, viewport_h);
    if let Some(offset) = crate::widgets::PropertyPanel::for_selection(state)
        .and_then(|panel| panel.motion_row_scroll(rect, animation))
    {
        state.editor_ui.property_panel_scroll.offset = offset;
    }
}

/// Follow a pointer move while a timeline drag is in flight. Returns
/// whether a drag consumed it (pointer capture: it holds even when the
/// pointer leaves the panel). `snap` is off while the modifier (Alt) is
/// held: the dragged time then follows the pointer exactly.
pub fn drag(
    state: &mut EditorState,
    point: Point2D,
    viewport_w: f32,
    viewport_h: f32,
    now_ms: u64,
    snap: bool,
) -> bool {
    let Some(drag) = &state.editor_ui.motion_timeline.drag else {
        return false;
    };
    let resizing = matches!(drag, MotionTimelineDrag::Height { .. });
    let Some((_, layout)) = layout(state, viewport_w, viewport_h) else {
        // The panel went away under the drag.
        state.cancel_motion_timeline_drag();
        return true;
    };
    if resizing {
        // The panel may take 60% of the canvas area it docks in.
        let (_, _, _, canvas_h) = canvas_region(state, viewport_w, viewport_h);
        let max_height = (canvas_h + layout.rect.size.y) * 0.6;
        state.resize_motion_timeline_to(point.y, max_height);
        return true;
    }
    let snap_ms = snap.then(|| f64::from(SNAP_PX) * layout.ms_per_px());
    state.drag_motion_timeline_to(layout.ms_for_x(point.x), snap_ms, now_ms);
    true
}

/// End a timeline drag, writing its draft. Returns whether a drag was in
/// flight (the release is then consumed).
pub fn release(state: &mut EditorState) -> bool {
    if state.editor_ui.motion_timeline.drag.is_none() {
        return false;
    }
    state.finish_motion_timeline_drag();
    true
}

/// Whether releasing the in-flight drag writes to the document (a scrub
/// or a resize does not), so hosts can gate it.
pub fn drag_writes(state: &EditorState) -> bool {
    state
        .editor_ui
        .motion_timeline
        .drag
        .as_ref()
        .is_some_and(|drag| {
            !matches!(
                drag,
                MotionTimelineDrag::Playhead | MotionTimelineDrag::Height { .. }
            )
        })
}

/// Whether Delete / Backspace belongs to the timeline: it shows and has
/// keyframes selected.
pub fn has_keyframe_selection(state: &EditorState) -> bool {
    motion_timeline_shown(state) && !state.editor_ui.motion_timeline.selection.is_empty()
}

/// Delete / Backspace while timeline keyframes are selected: delete them
/// instead of the canvas selection. Returns whether it did.
pub fn delete_keyframes(state: &mut EditorState) -> bool {
    if !has_keyframe_selection(state) {
        return false;
    }
    state.delete_selected_motion_keyframes();
    true
}

/// The pointer shape an in-flight timeline drag holds wherever the
/// pointer goes: north-south while resizing the panel, east-west while
/// stretching a strip.
pub fn drag_cursor(state: &EditorState) -> Option<CursorHint> {
    match state.editor_ui.motion_timeline.drag.as_ref()? {
        MotionTimelineDrag::Height { .. } => Some(CursorHint::ResizeNs),
        MotionTimelineDrag::StripEnd { .. } => Some(CursorHint::ResizeEw),
        _ => None,
    }
}

/// The pointer shape over the panel: the resize cursors over its top edge
/// and over a strip's end grip, from the layout walk the press uses.
pub fn hover_cursor(
    state: &EditorState,
    point: Point2D,
    viewport_w: f32,
    viewport_h: f32,
) -> Option<CursorHint> {
    let (view, layout) = layout(state, viewport_w, viewport_h)?;
    match layout.hit(point, &view)? {
        TimelineHit::ResizeEdge => Some(CursorHint::ResizeNs),
        TimelineHit::StripEnd { .. } => Some(CursorHint::ResizeEw),
        _ => None,
    }
}

/// [`drag_cursor`], else [`hover_cursor`].
pub fn cursor(
    state: &EditorState,
    point: Point2D,
    viewport_w: f32,
    viewport_h: f32,
) -> Option<CursorHint> {
    drag_cursor(state).or_else(|| hover_cursor(state, point, viewport_w, viewport_h))
}

/// The manual tab cycles through the scope's manual ids, one per press.
fn next_manual_id(view: &MotionTimelineView) -> Option<String> {
    let ids = &view.manual_ids;
    let current = match (view.tab, view.manual_id.as_deref()) {
        (MotionTimelineTab::Manual, Some(id)) => ids.iter().position(|other| other == id),
        _ => None,
    };
    match current {
        Some(at) => ids.get((at + 1) % ids.len()).cloned(),
        None => ids.first().cloned(),
    }
}

fn zoom_by(state: &mut EditorState, factor: f32) {
    let timeline = &mut state.editor_ui.motion_timeline;
    let zoom = timeline.zoom * factor;
    timeline.set_zoom(zoom);
}

fn strip<'a>(
    view: &'a MotionTimelineView,
    node_id: &str,
    animation: usize,
) -> Option<&'a TimelineStrip> {
    view.strips
        .iter()
        .find(|strip| strip.node_id == node_id && strip.animation == animation)
}

#[cfg(test)]
#[path = "motion_timeline_flow_tests.rs"]
mod tests;
