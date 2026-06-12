//! Ellipse arc-handle drag → `SetEllipseArc` command builder.
//! Relocated verbatim from `input.rs` (over the 800-line cap) when
//! the pen dispatch hooks landed there.

use super::WidgetHostNative;
use op_editor_ui::Point2D;

impl WidgetHostNative {
    /// Build the `SetEllipseArc` command for an in-progress arc-handle
    /// drag — converts the cursor doc point into start / sweep / inner
    /// geometry for the dragged handle. `None` for a missing or
    /// zero-size ellipse.
    pub(in crate::widget_host) fn arc_drag_command(
        &self,
        id: &op_editor_core::NodeId,
        handle: op_editor_ui::widgets::ArcHandle,
        doc: Point2D,
    ) -> Option<op_editor_core::EditorCommand> {
        use op_editor_core::EditorCommand;
        use op_editor_ui::widgets::ArcHandle;
        let node = self.layout_scene.active_page()?.find(id.as_str())?;
        let b = node.bounds;
        if b.size.x <= 0.0 || b.size.y <= 0.0 {
            return None;
        }
        let centre = Point2D::new(b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
        // Un-rotate the cursor into the ellipse's local frame.
        let doc = if node.rotation.abs() > f32::EPSILON {
            op_editor_ui::widgets::rotate_point(doc, centre, -node.rotation)
        } else {
            doc
        };
        // Cursor offset from the ellipse centre, normalised by the
        // radii so the angle is the same convention the painter uses.
        let nx = (doc.x - centre.x) / (b.size.x / 2.0);
        let ny = (doc.y - centre.y) / (b.size.y / 2.0);
        let old_start = node.arc_start_angle.unwrap_or(0.0);
        let old_sweep = node.arc_sweep_angle.unwrap_or(360.0);
        Some(match handle {
            ArcHandle::Start => {
                // Dragging the start handle keeps the end fixed; the
                // sweep keeps the sign of the existing arc.
                let new_start = norm360(ny.atan2(nx).to_degrees());
                let new_sweep = signed_sweep(old_start + old_sweep - new_start, old_sweep);
                EditorCommand::SetEllipseArc {
                    node_id: id.clone(),
                    start_angle: Some(new_start as f64),
                    sweep_angle: Some(new_sweep as f64),
                    inner_radius: None,
                }
            }
            ArcHandle::Sweep => {
                let new_sweep = signed_sweep(ny.atan2(nx).to_degrees() - old_start, old_sweep);
                EditorCommand::SetEllipseArc {
                    node_id: id.clone(),
                    start_angle: None,
                    sweep_angle: Some(new_sweep as f64),
                    inner_radius: None,
                }
            }
            ArcHandle::Inner => {
                let frac = (nx * nx + ny * ny).sqrt().clamp(0.0, 1.0);
                EditorCommand::SetEllipseArc {
                    node_id: id.clone(),
                    start_angle: None,
                    sweep_angle: None,
                    inner_radius: Some(frac as f64),
                }
            }
        })
    }
}

/// Normalise an angle into `[0, 360)` degrees.
fn norm360(deg: f32) -> f32 {
    let s = deg % 360.0;
    if s < 0.0 {
        s + 360.0
    } else {
        s
    }
}

/// Normalise a sweep into `(0, 360]` — a sweep that collapses to 0
/// snaps to a full 360° circle.
fn norm_sweep(deg: f32) -> f32 {
    let s = norm360(deg);
    if s <= 0.0001 {
        360.0
    } else {
        s
    }
}

/// A sweep that keeps the sign of the arc being edited — an
/// MCP-authored negative (counter-clockwise) sweep stays negative
/// under a canvas drag instead of flipping to the major arc. A
/// negative sweep that collapses to 0 snaps to a full -360° circle
/// (mirroring `norm_sweep`'s positive 0 → 360 rule).
fn signed_sweep(raw: f32, old_sweep: f32) -> f32 {
    let pos = norm_sweep(raw);
    if old_sweep < 0.0 {
        let neg = pos - 360.0;
        if neg == 0.0 {
            -360.0
        } else {
            neg
        }
    } else {
        pos
    }
}
