//! Safe-area ownership for generic and direct-editor pointer streams.

use crate::desc::OpPointerPhase;
use crate::lifecycle::Session;

#[derive(Default)]
pub(crate) struct SafeAreaPointerGate {
    ignored: Vec<u32>,
    #[cfg(feature = "editor")]
    editor_pointer_captured: bool,
    #[cfg(feature = "editor")]
    editor_transform_captured: Option<bool>,
}

impl SafeAreaPointerGate {
    pub(crate) fn reset(&mut self) {
        self.ignored.clear();
        #[cfg(feature = "editor")]
        {
            self.editor_pointer_captured = false;
            self.editor_transform_captured = None;
        }
    }

    pub(crate) fn route(
        &mut self,
        id: u32,
        phase: OpPointerPhase,
        point: (f32, f32),
        viewport: (f32, f32),
    ) -> bool {
        let ignored = self.ignored.iter().position(|pointer_id| *pointer_id == id);
        match phase {
            OpPointerPhase::Down => {
                if let Some(index) = ignored {
                    self.ignored.swap_remove(index);
                }
                if contains(point, viewport) {
                    true
                } else {
                    self.ignored.push(id);
                    false
                }
            }
            OpPointerPhase::Move => ignored.is_none(),
            OpPointerPhase::Up => {
                if let Some(index) = ignored {
                    self.ignored.swap_remove(index);
                    false
                } else {
                    true
                }
            }
            OpPointerPhase::Cancel => {
                // Cancel is global: route it even if the reported action id
                // began in a band, so accepted touches are reset as well.
                self.ignored.clear();
                true
            }
        }
    }

    #[cfg(feature = "editor")]
    fn begin_editor_pointer(&mut self, captured: bool) -> bool {
        self.editor_transform_captured = None;
        self.editor_pointer_captured = captured;
        captured
    }

    #[cfg(feature = "editor")]
    fn end_editor_pointer(&mut self) -> bool {
        std::mem::take(&mut self.editor_pointer_captured)
    }

    #[cfg(feature = "editor")]
    fn reset_editor(&mut self) {
        self.editor_pointer_captured = false;
        self.editor_transform_captured = None;
    }

    #[cfg(feature = "editor")]
    fn begin_editor_transform(&mut self, captured: bool) -> bool {
        self.editor_pointer_captured = false;
        self.editor_transform_captured = Some(captured);
        captured
    }

    #[cfg(feature = "editor")]
    fn editor_transform_captured(&self) -> bool {
        self.editor_transform_captured == Some(true)
    }
}

fn contains((x, y): (f32, f32), (w, h): (f32, f32)) -> bool {
    x >= 0.0 && x < w && y >= 0.0 && y < h
}

impl Session {
    pub(crate) fn route_safe_area_pointer(
        &mut self,
        id: u32,
        phase: OpPointerPhase,
        x: f32,
        y: f32,
    ) -> bool {
        let viewport = self.safe_area_viewport();
        self.pointer_gate.route(id, phase, (x, y), viewport)
    }

    #[cfg(feature = "editor")]
    pub(crate) fn safe_area_contains_surface_point(&self, x: f32, y: f32) -> bool {
        contains(self.safe_area_point(x, y), self.safe_area_viewport())
    }

    #[cfg(feature = "editor")]
    pub(crate) fn begin_editor_pointer_capture(&mut self, x: f32, y: f32) -> bool {
        let captured = self.safe_area_contains_surface_point(x, y);
        self.pointer_gate.begin_editor_pointer(captured)
    }

    #[cfg(feature = "editor")]
    pub(crate) fn editor_pointer_captured(&self) -> bool {
        self.pointer_gate.editor_pointer_captured
    }

    #[cfg(feature = "editor")]
    pub(crate) fn end_editor_pointer_capture(&mut self) -> bool {
        self.pointer_gate.end_editor_pointer()
    }

    #[cfg(feature = "editor")]
    pub(crate) fn reset_editor_pointer_capture(&mut self) {
        self.pointer_gate.reset_editor();
    }

    #[cfg(feature = "editor")]
    pub(crate) fn begin_editor_transform(&mut self, x: f32, y: f32) -> bool {
        let captured = self.safe_area_contains_surface_point(x, y);
        self.pointer_gate.begin_editor_transform(captured)
    }

    #[cfg(feature = "editor")]
    pub(crate) fn editor_transform_captured(&self) -> bool {
        self.pointer_gate.editor_transform_captured()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEWPORT: (f32, f32) = (300.0, 500.0);

    #[test]
    fn rejected_down_suppresses_the_stream_but_content_capture_may_leave() {
        let mut gate = SafeAreaPointerGate::default();
        assert!(!gate.route(1, OpPointerPhase::Down, (-1.0, 20.0), VIEWPORT));
        assert!(!gate.route(1, OpPointerPhase::Move, (20.0, 20.0), VIEWPORT));
        assert!(!gate.route(1, OpPointerPhase::Up, (20.0, 20.0), VIEWPORT));

        assert!(gate.route(2, OpPointerPhase::Down, (20.0, 20.0), VIEWPORT));
        assert!(gate.route(2, OpPointerPhase::Move, (-5.0, 20.0), VIEWPORT));
        assert!(gate.route(2, OpPointerPhase::Up, (-5.0, 20.0), VIEWPORT));
    }

    #[test]
    fn cancel_on_an_ignored_id_is_still_routed_globally() {
        let mut gate = SafeAreaPointerGate::default();
        assert!(gate.route(1, OpPointerPhase::Down, (20.0, 20.0), VIEWPORT));
        assert!(!gate.route(2, OpPointerPhase::Down, (-1.0, 20.0), VIEWPORT));
        assert!(gate.route(2, OpPointerPhase::Cancel, (-1.0, 20.0), VIEWPORT));
        assert!(gate.route(2, OpPointerPhase::Up, (20.0, 20.0), VIEWPORT));
    }

    #[cfg(feature = "editor")]
    #[test]
    fn direct_transform_ownership_is_fixed_at_the_first_event() {
        let mut gate = SafeAreaPointerGate::default();
        assert!(gate.begin_editor_transform(true));
        assert!(gate.editor_transform_captured());

        gate.reset_editor();
        assert!(!gate.begin_editor_transform(false));
        assert!(!gate.editor_transform_captured());
    }
}
