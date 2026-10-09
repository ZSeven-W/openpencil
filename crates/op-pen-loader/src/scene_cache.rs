//! Skip the layout-scene rebuild when its inputs are unchanged.
//!
//! [`editor_state_to_active_page_layout_scene`](crate::editor_state_to_active_page_layout_scene)
//! is a pure function of just the document, the authored-geometry latch, the
//! active page index, normal table reading posture, and the resolved variable table (which folds the active
//! theme and the transient fill / stroke ref caches). It drops every other piece
//! of editor state — selection, hover, chat, viewport, history. Yet the host
//! marks the scene dirty on nearly every interaction: hover (each mouse-move),
//! scroll / zoom, selection / marquee, caret + uncommitted-text drafts (each
//! keystroke), UI toggles, and streamed chat / codegen deltas.
//!
//! This cache holds compact identities for those inputs and skips the rebuild
//! when they still match. Document identity is the editor's
//! `(document_generation, document_revision)` pair, so a UI-only refresh is
//! `O(variables)` rather than cloning and comparing the entire node tree.

use jian_scene::layout_scene::LayoutScene;
use op_editor_core::scene_vars::VariableTable;

/// The inputs of the last layout-scene build, retained so an unchanged refresh
/// can skip the (taffy + reshape) rebuild.
#[derive(Default)]
pub struct SceneBuildCache {
    last: Option<BuiltInputs>,
}

/// Every value `editor_state_to_active_page_layout_scene` reads off the
/// `EditorState`:
/// the document, the authored-geometry latch (preview toggles it, changing the
/// layout mode), the active page index, normal table posture, and the resolved variable table — which
/// folds the active theme plus the transient fill / stroke ref caches, i.e. all
/// the non-doc resolution inputs. Everything else on the state (selection /
/// chat / hover / viewport) is dropped by the builder.
struct BuiltInputs {
    document_generation: u64,
    document_revision: u64,
    preserve_authored_geometry: bool,
    normal_table_view: bool,
    active_page_index: usize,
    var_table: VariableTable,
    /// Font-registry generation the scene was laid out against. A runtime
    /// font import/removal advances it without touching the document, so it
    /// must gate the rebuild — otherwise an already-open document keeps its
    /// stale (fallback-font) layout after an import.
    font_generation: u64,
}

impl SceneBuildCache {
    pub fn new() -> Self {
        Self { last: None }
    }

    /// Build a fresh [`LayoutScene`] only if the scene-relevant inputs (doc,
    /// active theme, active page) changed since the last build.
    ///
    /// Returns `Some(scene)` when a rebuild happened (the caller installs it),
    /// or `None` when the inputs are identical (the caller keeps its current
    /// scene).
    pub fn maybe_rebuild(&mut self, state: &op_editor_core::EditorState) -> Option<LayoutScene> {
        let document_generation = state.document_generation();
        let document_revision = state.document_revision();
        let preserve_authored_geometry = state.editor_ui.preserve_authored_geometry;
        let active_page_index = state.ui.active_page_index;
        let normal_table_view = state.editor_ui.workspace.visible;
        // Resolves the active theme + transient fill/stroke ref caches + the
        // doc-defined variables — every non-doc input the builder consumes.
        // Proportional to variable count, not node count, so cheap to rebuild
        // per refresh purely for the comparison.
        let var_table = crate::editor_state_var_table(state);
        let font_generation = crate::current_font_generation();
        if let Some(last) = &self.last {
            if last.document_generation == document_generation
                && last.document_revision == document_revision
                && last.active_page_index == active_page_index
                && last.preserve_authored_geometry == preserve_authored_geometry
                && last.normal_table_view == normal_table_view
                && last.font_generation == font_generation
                && last.var_table == var_table
            {
                return None;
            }
        }
        let scene = crate::editor_state_to_active_page_layout_scene(state);
        self.last = Some(BuiltInputs {
            document_generation,
            document_revision,
            preserve_authored_geometry,
            normal_table_view,
            active_page_index,
            var_table,
            font_generation,
        });
        Some(scene)
    }

    /// Forget the cached inputs so the next [`maybe_rebuild`](Self::maybe_rebuild)
    /// always rebuilds. Call when the host installs a scene through a path that
    /// bypasses this cache. Whole-state replacement paths must also call this:
    /// a fresh state may restart at the same generation/revision pair as the
    /// document it replaced even though its contents differ.
    pub fn invalidate(&mut self) {
        self.last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::EditorState;

    fn state_with_rect(x: f64) -> EditorState {
        let doc = jian_ops_schema::load_str(&format!(
            r#"{{"version":"1.0.0","children":[
              {{"type":"rectangle","id":"r","name":"r","x":{x},"y":0,"width":40,"height":20}}
            ]}}"#
        ))
        .expect("fixture parses")
        .value;
        EditorState::from_document(doc)
    }

    #[test]
    fn first_build_returns_a_scene_then_unchanged_refreshes_are_skipped() {
        let mut cache = SceneBuildCache::new();
        let state = state_with_rect(0.0);

        assert!(
            cache.maybe_rebuild(&state).is_some(),
            "first build must produce a scene"
        );
        assert!(
            cache.maybe_rebuild(&state).is_none(),
            "an identical refresh must skip the rebuild"
        );
        assert!(
            cache.maybe_rebuild(&state).is_none(),
            "still skipped on repeat"
        );
    }

    #[test]
    fn a_document_change_forces_a_rebuild() {
        let mut cache = SceneBuildCache::new();
        let a = state_with_rect(0.0);
        let mut b = state_with_rect(100.0); // node moved → different doc
        b.mark_document_changed();

        assert!(cache.maybe_rebuild(&a).is_some());
        assert!(cache.maybe_rebuild(&a).is_none());
        assert!(
            cache.maybe_rebuild(&b).is_some(),
            "a moved node must rebuild the scene"
        );
        assert!(cache.maybe_rebuild(&b).is_none());
    }

    #[test]
    fn active_page_and_theme_changes_force_a_rebuild() {
        let mut cache = SceneBuildCache::new();
        let mut state = state_with_rect(0.0);
        assert!(cache.maybe_rebuild(&state).is_some());
        assert!(cache.maybe_rebuild(&state).is_none());

        state.ui.active_page_index = state.ui.active_page_index.wrapping_add(1);
        assert!(
            cache.maybe_rebuild(&state).is_some(),
            "an active-page switch must rebuild"
        );

        state.ui.active_page_index = state.ui.active_page_index.wrapping_sub(1);
        let _ = cache.maybe_rebuild(&state);
        state
            .ui
            .variables
            .active_theme
            .insert("mode".to_string(), "dark".to_string());
        assert!(
            cache.maybe_rebuild(&state).is_some(),
            "an active-theme change must rebuild (it re-resolves token fills)"
        );
    }

    #[test]
    fn toggling_preserve_authored_geometry_forces_a_rebuild() {
        // Preview mode flips `preserve_authored_geometry`, which switches the
        // layout-resolution path — so it must rebuild even though the document,
        // theme, and page are unchanged.
        let mut cache = SceneBuildCache::new();
        let mut state = state_with_rect(0.0);
        assert!(cache.maybe_rebuild(&state).is_some());
        assert!(cache.maybe_rebuild(&state).is_none());

        state.editor_ui.preserve_authored_geometry = !state.editor_ui.preserve_authored_geometry;
        assert!(
            cache.maybe_rebuild(&state).is_some(),
            "toggling preserve_authored_geometry must rebuild the scene"
        );
    }

    #[test]
    fn invalidate_forces_the_next_build() {
        let mut cache = SceneBuildCache::new();
        let state = state_with_rect(0.0);
        assert!(cache.maybe_rebuild(&state).is_some());
        assert!(cache.maybe_rebuild(&state).is_none());
        cache.invalidate();
        assert!(
            cache.maybe_rebuild(&state).is_some(),
            "invalidate must force the next rebuild"
        );
    }
}
