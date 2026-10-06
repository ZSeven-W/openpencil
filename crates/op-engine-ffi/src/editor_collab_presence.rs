//! Mobile pointer projection for collaboration presence.

use crate::desc::OpPointerPhase;
use crate::lifecycle::Session;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EditorPresencePointer {
    pointer_id: Option<u32>,
    editor_x: f32,
    editor_y: f32,
}

impl EditorPresencePointer {
    fn legacy(editor_x: f32, editor_y: f32) -> Self {
        Self {
            pointer_id: None,
            editor_x,
            editor_y,
        }
    }

    fn tracked(pointer_id: u32, editor_x: f32, editor_y: f32) -> Self {
        Self {
            pointer_id: Some(pointer_id),
            editor_x,
            editor_y,
        }
    }
}

impl Session {
    /// Track a direct-editor press/move/hover point. The point stays in editor
    /// coordinates so viewport changes are reflected when the frame pump maps
    /// it to document space.
    pub(crate) fn set_editor_presence_pointer(&mut self, x: f32, y: f32) -> bool {
        let Some(collab) = self.editor_collab.as_mut() else {
            return false;
        };
        let next = EditorPresencePointer::legacy(x, y);
        let changed = collab.presence_pointer != Some(next);
        collab.presence_pointer = Some(next);
        changed
    }

    /// Track only the first active `op_pointer` id. Secondary fingers must not
    /// steal or clear the cursor advertised for the primary touch.
    pub(crate) fn update_editor_presence_pointer(
        &mut self,
        pointer_id: u32,
        phase: OpPointerPhase,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(collab) = self.editor_collab.as_mut() else {
            return false;
        };
        match phase {
            OpPointerPhase::Down => {
                if collab
                    .presence_pointer
                    .is_some_and(|pointer| pointer.pointer_id.is_some_and(|id| id != pointer_id))
                {
                    return false;
                }
                let next = EditorPresencePointer::tracked(pointer_id, x, y);
                let changed = collab.presence_pointer != Some(next);
                collab.presence_pointer = Some(next);
                changed
            }
            OpPointerPhase::Move => {
                if collab
                    .presence_pointer
                    .is_none_or(|pointer| pointer.pointer_id != Some(pointer_id))
                {
                    return false;
                }
                let next = EditorPresencePointer::tracked(pointer_id, x, y);
                let changed = collab.presence_pointer != Some(next);
                collab.presence_pointer = Some(next);
                changed
            }
            OpPointerPhase::Up => {
                if collab
                    .presence_pointer
                    .is_none_or(|pointer| pointer.pointer_id != Some(pointer_id))
                {
                    return false;
                }
                collab.presence_pointer = None;
                true
            }
            OpPointerPhase::Cancel => collab.presence_pointer.take().is_some(),
        }
    }

    pub(crate) fn clear_editor_presence_pointer(&mut self) -> bool {
        self.editor_collab
            .as_mut()
            .and_then(|collab| collab.presence_pointer.take())
            .is_some()
    }

    /// Mirror desktop presence: only points inside the current canvas region
    /// become a document cursor; chrome points intentionally publish `None`.
    /// Floating overlays follow the desktop path and may cover that region.
    pub(crate) fn editor_presence_cursor(&self) -> Option<(f64, f64)> {
        let pointer = self.editor_collab.as_ref()?.presence_pointer?;
        let (width, height) = self.editor_viewport();
        self.editor
            .as_ref()?
            .canvas_doc_point(pointer.editor_x, pointer.editor_y, width, height)
    }
}
