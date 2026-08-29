//! Track C-3: screen-transition animations for APP MODE screen switches.
//!
//! `push` = the entering screen slides in from the right (240ms
//! ease-out); `pop` = the screen being left slides out to the right,
//! revealing the destination underneath (same 240ms ease-out, mirrored —
//! the plan doc names only push's easing explicitly, so pop reuses it for
//! visual symmetry); `replace` (tab switch / pill click) = a 160ms linear
//! cross-fade, both screens held in place. See the design doc's Track C-3.
//!
//! [`super::app_mode`]'s `reconcile` constructs a [`ScreenTransition`] on
//! every screen switch, snapshotting the OUTGOING scene page before it
//! overwrites `PreviewSession::scene` with the entering one.
//! [`PreviewSession::paint_framed_animated`] is the device frame's paint
//! entry point (`op-host-native::widget_host::preview_frame`) — it routes
//! straight to the steady-state [`PreviewSession::paint_framed`] once the
//! animation finishes, or composites both layers while it plays.
//!
//! "Skippable" (a nav firing again mid-animation): `reconcile` always
//! REPLACES `self.transition` outright rather than queuing — the very
//! next reconcile pass sees whatever `self.scene` had just settled to as
//! its fresh outgoing snapshot, so no half-finished slide lingers and no
//! second transition ever waits in line.

use super::PreviewSession;
use op_editor_ui::layout_scene::{LayoutScene, SceneNode, ScenePage};
use op_editor_ui::widgets::{paint_scene_page_with, PaintCx, PaintSceneOptions};
use op_editor_ui::{Point2D, Rect, RenderBackend};

pub(crate) const PUSH_POP_DURATION_MS: u64 = 240;
pub(crate) const REPLACE_DURATION_MS: u64 = 160;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransitionKind {
    Push,
    Pop,
    Replace,
}

/// Classify a screen switch from the route-stack depth recorded at the
/// LAST reconcile (`AppMode::mounted_stack`) versus the depth just after
/// this switch. The router's live stack has already advanced by the time
/// `reconcile` runs (the tap mutated it synchronously), so the verb
/// itself (`push`/`replace`/`pop`) isn't available here — but depth is
/// the exact, sufficient signature of all three: deeper is only ever a
/// push, shallower only ever a pop, same depth only ever a replace/reset.
pub(crate) fn classify_transition(prev_len: usize, new_len: usize) -> TransitionKind {
    match new_len.cmp(&prev_len) {
        std::cmp::Ordering::Greater => TransitionKind::Push,
        std::cmp::Ordering::Less => TransitionKind::Pop,
        std::cmp::Ordering::Equal => TransitionKind::Replace,
    }
}

/// An in-flight screen-transition animation.
pub(crate) struct ScreenTransition {
    kind: TransitionKind,
    started_at_ms: u64,
    duration_ms: u64,
    /// The scene page as it looked immediately before this switch — the
    /// layer a push/pop slides or a replace fades out.
    outgoing: ScenePage,
}

/// R8: the single deferred discrete input held across a transition.
///
/// Only these three shapes are safe to replay onto a screen that has just
/// finished arriving: they are discrete (one decision, no in-flight
/// state), they carry their own target, and none of them is meaningful
/// half-applied. Continuous gestures, text and IME are discarded instead
/// — replaying them would resume a drag or an edit against a widget that
/// no longer exists.
///
/// `route_generation` is the session's route counter at the moment the
/// input was captured. Replay compares it against the counter at
/// transition end: a second navigation in between means the screen this
/// input was aimed at is gone, so the input dies with it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DeferredDiscreteInput {
    Tap {
        scene_x: f32,
        scene_y: f32,
        pointer_id: u32,
        kind: jian_core::gesture::pointer::PointerKind,
        activation: Option<op_preview_contracts::UserActivationId>,
        route_generation: u64,
    },
    Submit {
        key: String,
        modifiers: jian_core::gesture::pointer::Modifiers,
        activation: Option<op_preview_contracts::UserActivationId>,
        route_generation: u64,
    },
    Back {
        activation: Option<op_preview_contracts::UserActivationId>,
        route_generation: u64,
    },
}

impl DeferredDiscreteInput {
    pub(crate) fn route_generation(&self) -> u64 {
        match self {
            Self::Tap {
                route_generation, ..
            }
            | Self::Submit {
                route_generation, ..
            }
            | Self::Back {
                route_generation, ..
            } => *route_generation,
        }
    }

    pub(crate) fn activation(&self) -> Option<op_preview_contracts::UserActivationId> {
        match self {
            Self::Tap { activation, .. }
            | Self::Submit { activation, .. }
            | Self::Back { activation, .. } => *activation,
        }
    }
}

/// R8: the transition-local pointer tracker.
///
/// A press that starts during a transition is NOT queued as raw phases —
/// the runtime never sees them. The tracker only remembers enough to
/// decide, at `Up`, whether the press was a Tap; anything that drifts too
/// far or lingers too long is dropped rather than guessed at.
///
/// Thresholds are jian's own gesture defaults, not local inventions:
/// [`jian_core::gesture::config::DEFAULT_DOUBLE_TAP_SLOP_PX`] bounds the
/// drift and [`jian_core::gesture::config::DEFAULT_LONG_PRESS_MS`] bounds
/// the duration — past it the press was a long-press, which is not a
/// discrete input and must not be synthesized into a Tap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TransitionTapTracker {
    pub(crate) pointer_id: u32,
    pub(crate) kind: jian_core::gesture::pointer::PointerKind,
    pub(crate) down_x: f32,
    pub(crate) down_y: f32,
    pub(crate) down_ms: u64,
}

impl TransitionTapTracker {
    /// Whether an `Up` for `pointer_id` at this point and time completes
    /// a Tap. A different pointer, too much drift, or too long a hold all
    /// mean "not a tap".
    pub(crate) fn completes_tap(&self, pointer_id: u32, x: f32, y: f32, now_ms: u64) -> bool {
        if pointer_id != self.pointer_id {
            return false;
        }
        let dx = x - self.down_x;
        let dy = y - self.down_y;
        let drift = (dx * dx + dy * dy).sqrt();
        let held = now_ms.saturating_sub(self.down_ms);
        drift <= jian_core::gesture::config::DEFAULT_DOUBLE_TAP_SLOP_PX
            && held < jian_core::gesture::config::DEFAULT_LONG_PRESS_MS
    }
}

impl ScreenTransition {
    pub(crate) fn start(kind: TransitionKind, outgoing: ScenePage, now_ms: u64) -> Self {
        let duration_ms = match kind {
            TransitionKind::Replace => REPLACE_DURATION_MS,
            TransitionKind::Push | TransitionKind::Pop => PUSH_POP_DURATION_MS,
        };
        Self {
            kind,
            started_at_ms: now_ms,
            duration_ms,
            outgoing,
        }
    }

    pub(crate) fn is_active(&self, now_ms: u64) -> bool {
        now_ms < self.started_at_ms.saturating_add(self.duration_ms)
    }

    /// Next wake time for the host's animation loop — same shape as
    /// `op_editor_ui::widgets::CanvasLayoutTransition::next_deadline_ms`,
    /// though the host doesn't actually need this: `next_animation_deadline_ms`
    /// already ticks ~30fps for the whole `self.preview.is_some()` window.
    /// Kept for parity / a future host that wants a tighter wake schedule.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn next_deadline_ms(&self, now_ms: u64) -> Option<u64> {
        if !self.is_active(now_ms) {
            return None;
        }
        Some((now_ms.saturating_add(16)).min(self.started_at_ms + self.duration_ms))
    }

    fn linear_t(&self, now_ms: u64) -> f32 {
        let elapsed = now_ms.saturating_sub(self.started_at_ms);
        (elapsed as f32 / self.duration_ms.max(1) as f32).clamp(0.0, 1.0)
    }

    /// Ease-out-cubic progress for the push/pop slide — the same formula
    /// `op_editor_ui::widgets::CanvasLayoutTransition` uses, copied rather
    /// than imported: that helper's `translate_scene_subtree` diffs ONE
    /// scene's nodes by id across a layout change (a per-node problem);
    /// sliding two independent whole pages by a viewport-origin offset
    /// (see `paint_framed_animated`) only needs this one-line formula, not
    /// the subtree walker, so there's nothing else worth reusing across
    /// the crate boundary.
    fn eased_t(&self, now_ms: u64) -> f32 {
        let t = self.linear_t(now_ms);
        1.0 - (1.0 - t).powi(3)
    }
}

/// Apply the Replace cross-fade once to this transition root's composed
/// output. The painter creates a temporary normal-blend layer even for a
/// leaf root, so overlapping descendants, effects, shaders, and text runs
/// all fade together without rewriting their authored paint alpha.
fn fade_scene_node(node: &mut SceneNode, factor: f32) {
    let factor = if factor.is_finite() {
        factor.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let authored_opacity = if node.composite_opacity.is_finite() {
        node.composite_opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };
    node.composite_opacity = authored_opacity * factor;
}

/// Paint one root, optionally faded, inside `content_clip`. Mirrors the
/// clip/cull/paint sequence `PreviewSession::paint_framed` uses for its
/// single (steady-state) layer.
#[allow(clippy::too_many_arguments)]
fn paint_content_layer(
    backend: &mut dyn RenderBackend,
    page: &ScenePage,
    root_id: &str,
    origin: Point2D,
    fit: f32,
    content_clip: Rect,
    cull: Rect,
    fade: Option<f32>,
) {
    let faded_page;
    let page_ref: &ScenePage = match fade {
        Some(factor) => {
            let mut clone = page.clone();
            if let Some(root) = clone.children.iter_mut().find(|c| c.id == root_id) {
                fade_scene_node(root, factor);
            }
            faded_page = clone;
            &faded_page
        }
        None => page,
    };
    backend.save();
    backend.clip_rect(content_clip);
    {
        let mut cx = PaintCx { backend };
        paint_scene_page_with(
            &mut cx,
            page_ref,
            origin,
            fit,
            cull,
            PaintSceneOptions {
                only_root: Some(root_id),
                skip_node: None,
            },
        );
    }
    backend.restore();
}

impl PreviewSession {
    /// Device-frame paint entry point (`widget_host::preview_frame`
    /// calls this in place of [`PreviewSession::paint_framed`]): routes
    /// straight through when no transition is playing, or composites the
    /// outgoing + entering screens for the in-flight animation.
    ///
    /// Deliberately simplified versus the steady-state `paint_framed` for
    /// the animation's short window: no pinned nav / status-bar strip and
    /// no focus caret paint while `is_active` — all three resume the very
    /// next frame the animation ends. Both layers otherwise go through the
    /// exact same painter (`paint_scene_page_with`) `paint_framed` itself
    /// calls.
    #[allow(clippy::too_many_arguments)]
    pub fn paint_framed_animated(
        &self,
        backend: &mut dyn RenderBackend,
        only_root: &str,
        content_clip: Rect,
        content_origin: Point2D,
        fit: f32,
        pinned: Option<&super::present::PinnedPaint>,
        pinned_top: Option<&super::present::PinnedPaint>,
        now_ms: u64,
    ) {
        let Some(transition) = self.transition.as_ref().filter(|t| t.is_active(now_ms)) else {
            self.paint_framed(
                backend,
                only_root,
                content_clip,
                content_origin,
                fit,
                pinned,
                pinned_top,
                now_ms,
            );
            return;
        };
        let Some(outgoing_root) = transition.outgoing.children.first() else {
            self.paint_framed(
                backend,
                only_root,
                content_clip,
                content_origin,
                fit,
                pinned,
                pinned_top,
                now_ms,
            );
            return;
        };
        let outgoing_id = outgoing_root.id.clone();

        let overlaid_entering;
        let entering_scene: &LayoutScene = if self.runtime.widget_states.iter().next().is_none()
            && self.binding_sites.is_empty()
        {
            &self.scene
        } else {
            overlaid_entering = self.overlay_runtime_state(&self.scene);
            &overlaid_entering
        };
        let Some(entering_page) = entering_scene.active_page() else {
            return;
        };

        const CULL_MARGIN: f32 = 64.0;
        let cull = Rect {
            origin: Point2D::new(
                content_clip.origin.x - CULL_MARGIN,
                content_clip.origin.y - CULL_MARGIN,
            ),
            size: Point2D::new(
                content_clip.size.x + CULL_MARGIN * 2.0,
                content_clip.size.y + CULL_MARGIN * 2.0,
            ),
        };

        match transition.kind {
            TransitionKind::Push => {
                // Bottom: outgoing, static (already correctly placed).
                // Top: entering, sliding in from the right toward 0.
                paint_content_layer(
                    backend,
                    &transition.outgoing,
                    &outgoing_id,
                    content_origin,
                    fit,
                    content_clip,
                    cull,
                    None,
                );
                let dx = content_clip.size.x * (1.0 - transition.eased_t(now_ms));
                let entering_origin = Point2D::new(content_origin.x + dx, content_origin.y);
                paint_content_layer(
                    backend,
                    entering_page,
                    only_root,
                    entering_origin,
                    fit,
                    content_clip,
                    cull,
                    None,
                );
            }
            TransitionKind::Pop => {
                // Bottom: entering (the destination, already revealed).
                // Top: outgoing, sliding right off-screen to uncover it.
                paint_content_layer(
                    backend,
                    entering_page,
                    only_root,
                    content_origin,
                    fit,
                    content_clip,
                    cull,
                    None,
                );
                let dx = content_clip.size.x * transition.eased_t(now_ms);
                let outgoing_origin = Point2D::new(content_origin.x + dx, content_origin.y);
                paint_content_layer(
                    backend,
                    &transition.outgoing,
                    &outgoing_id,
                    outgoing_origin,
                    fit,
                    content_clip,
                    cull,
                    None,
                );
            }
            TransitionKind::Replace => {
                let t = transition.linear_t(now_ms);
                paint_content_layer(
                    backend,
                    &transition.outgoing,
                    &outgoing_id,
                    content_origin,
                    fit,
                    content_clip,
                    cull,
                    Some(1.0 - t),
                );
                paint_content_layer(
                    backend,
                    entering_page,
                    only_root,
                    content_origin,
                    fit,
                    content_clip,
                    cull,
                    Some(t),
                );
            }
        }

        // The pinned strips stay STEADY through the whole slide/fade,
        // painted opaque from the entering screen — exactly as
        // `paint_framed` paints them outside a transition. Without this
        // the strip band renders nothing for the transition's 160-240ms
        // (the content layers are clipped to `content_clip`, which the
        // host inset past the strips), so a tab switch flashes the nav
        // to bare bezel and back — the opposite of what a persistent
        // bottom bar is for.
        for paint in [pinned, pinned_top].into_iter().flatten() {
            if entering_page.find(&paint.node_id).is_none() {
                continue;
            }
            backend.save();
            backend.clip_rect(paint.strip_clip);
            {
                let mut cx = PaintCx {
                    backend: &mut *backend,
                };
                op_editor_ui::widgets::paint_scene_subtree(
                    &mut cx,
                    entering_page,
                    &paint.node_id,
                    paint.paint_origin,
                    fit,
                );
            }
            backend.restore();
        }
    }

    /// Whether a Track C-3 transition is currently playing, using the
    /// session's current clock (`last_now_ms` — the greatest value
    /// pushed via `set_now_ms` or synchronized from an explicit
    /// `dispatch_pointer_phase_at` timestamp, see `input.rs`).
    /// `input.rs`'s pointer dispatch gates on this — DISCARDING
    /// taps/drags/wheel while a screen-transition animation plays,
    /// rather than queuing them, is the "simple and doesn't break
    /// anything" choice: a tap mid-slide has no stable target to land on
    /// anyway (the content is physically moving), and the window is
    /// short (160-240ms) — a queued tap would risk firing against
    /// whichever screen happens to be mounted once the animation ends,
    /// which is not necessarily what the user was aiming at when they
    /// tapped.
    /// Public so a host can keep driving frames while the screen slide
    /// plays: an event-driven shell (the web host) repaints only when
    /// something asks it to, and without this the animation renders as
    /// one or two discrete jumps whenever an unrelated event happens to
    /// trigger a paint.
    pub fn transition_active(&self) -> bool {
        self.transition
            .as_ref()
            .is_some_and(|t| t.is_active(self.last_now_ms))
    }

    /// Test-only: whether a transition is currently playing at `now_ms`.
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn transition_active_for_test(&self, now_ms: u64) -> bool {
        self.transition
            .as_ref()
            .is_some_and(|t| t.is_active(now_ms))
    }

    /// Test-only: the kind of the current (possibly finished) transition.
    #[cfg(all(test, not(target_os = "windows")))]
    pub(crate) fn transition_kind_for_test(&self) -> Option<TransitionKind> {
        self.transition.as_ref().map(|t| t.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(root_id: &str) -> ScenePage {
        use op_editor_ui::layout_scene::NodeKind;
        let mut root = SceneNode::leaf(root_id, NodeKind::Frame);
        root.bounds = Rect::xywh(0.0, 0.0, 390.0, 844.0);
        ScenePage {
            id: "p".into(),
            name: "P".into(),
            children: vec![root],
        }
    }

    #[test]
    fn classify_by_stack_depth() {
        assert_eq!(classify_transition(1, 2), TransitionKind::Push);
        assert_eq!(classify_transition(2, 1), TransitionKind::Pop);
        assert_eq!(classify_transition(1, 1), TransitionKind::Replace);
        assert_eq!(classify_transition(3, 5), TransitionKind::Push);
    }

    #[test]
    fn push_pop_use_240ms_replace_uses_160ms() {
        let push = ScreenTransition::start(TransitionKind::Push, page("a"), 0);
        assert!(push.is_active(239));
        assert!(!push.is_active(240));

        let pop = ScreenTransition::start(TransitionKind::Pop, page("a"), 0);
        assert!(pop.is_active(239));
        assert!(!pop.is_active(240));

        let replace = ScreenTransition::start(TransitionKind::Replace, page("a"), 0);
        assert!(replace.is_active(159));
        assert!(!replace.is_active(160));
    }

    #[test]
    fn eased_t_starts_at_zero_ends_at_one() {
        let t = ScreenTransition::start(TransitionKind::Push, page("a"), 1_000);
        assert_eq!(t.eased_t(1_000), 0.0);
        assert!((t.eased_t(1_240) - 1.0).abs() < 1e-6);
        // Monotonic ease-out: past the midpoint, more than half done.
        assert!(t.eased_t(1_120) > 0.5);
    }

    #[test]
    fn next_deadline_ms_ticks_until_done() {
        let t = ScreenTransition::start(TransitionKind::Replace, page("a"), 1_000);
        assert_eq!(t.next_deadline_ms(1_000), Some(1_016));
        // Never overshoots the animation's own end.
        assert_eq!(t.next_deadline_ms(1_150), Some(1_160));
        assert_eq!(
            t.next_deadline_ms(1_160),
            None,
            "finished — no more wakeups"
        );
    }

    #[test]
    fn linear_t_is_unclamped_progress_for_replace() {
        let t = ScreenTransition::start(TransitionKind::Replace, page("a"), 1_000);
        assert_eq!(t.linear_t(1_000), 0.0);
        assert!((t.linear_t(1_080) - 0.5).abs() < 1e-6);
        assert_eq!(t.linear_t(1_160), 1.0);
        assert_eq!(t.linear_t(5_000), 1.0, "clamped past the end");
    }

    #[test]
    fn fade_multiplies_only_the_transition_root_composite_opacity() {
        use op_editor_ui::Color;

        let mut child = SceneNode::leaf("c", op_editor_ui::layout_scene::NodeKind::Rect);
        child.composite_opacity = 0.4;
        child.opacity = 0.3;
        child.fill = Some(Color {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 0.7,
        });

        let mut root = SceneNode::leaf("r", op_editor_ui::layout_scene::NodeKind::Frame);
        root.composite_opacity = 0.6;
        root.opacity = 0.8;
        root.fill = Some(Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 0.9,
        });
        root.children.push(child);
        let steady_page = ScenePage {
            id: "p".into(),
            name: "P".into(),
            children: vec![root],
        };
        let mut faded_page = steady_page.clone();

        fade_scene_node(&mut faded_page.children[0], 0.5);

        let faded_root = &faded_page.children[0];
        assert!((faded_root.composite_opacity - 0.3).abs() < 1e-6);
        assert!((faded_root.opacity - 0.8).abs() < 1e-6);
        assert!((faded_root.fill.unwrap().a - 0.9).abs() < 1e-6);
        assert!((faded_root.children[0].composite_opacity - 0.4).abs() < 1e-6);
        assert!((faded_root.children[0].opacity - 0.3).abs() < 1e-6);
        assert!((faded_root.children[0].fill.unwrap().a - 0.7).abs() < 1e-6);
        assert!(
            (steady_page.children[0].composite_opacity - 0.6).abs() < 1e-6,
            "the steady-state scene must remain authored and reusable"
        );
    }

    #[test]
    fn fade_adds_a_composite_layer_to_a_leaf_root() {
        let mut leaf = SceneNode::leaf("leaf", op_editor_ui::layout_scene::NodeKind::Rect);
        assert_eq!(leaf.composite_opacity, 1.0);

        fade_scene_node(&mut leaf, 0.25);

        assert!((leaf.composite_opacity - 0.25).abs() < 1e-6);
    }
}
