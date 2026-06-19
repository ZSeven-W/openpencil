//! Skip the layout-scene rebuild when its inputs are unchanged.
//!
//! [`editor_state_to_layout_scene`](crate::editor_state_to_layout_scene) is a
//! pure function of just the document, the authored-geometry latch, the active
//! page index, and the resolved variable table (which folds the active theme and
//! the transient fill / stroke ref caches). It drops every other piece of editor
//! state — selection, hover, chat, viewport, history. Yet the host marks the
//! scene dirty on nearly every interaction: hover (each mouse-move), scroll /
//! zoom, selection / marquee, caret + uncommitted-text drafts (each keystroke),
//! UI toggles, and — worst — streamed chat / codegen deltas, which today re-run
//! a full taffy solve plus a whole-tree `SceneNode` re-allocation *once per
//! animation frame* for content that never touches the canvas.
//!
//! This cache holds those inputs from the last build and skips the rebuild when
//! they still match, collapsing those reconversions to an `O(nodes)` comparison.
//! The check is content-based (not a hand-maintained revision counter), so no
//! mutation can silently leave a stale scene — correctness does not depend on
//! every mutating call site remembering to bump a flag.

use jian_ops_schema::PenDocument;
use op_editor_ui::layout_scene::LayoutScene;
use op_editor_ui::scene_vars::VariableTable;

/// The inputs of the last layout-scene build, retained so an unchanged refresh
/// can skip the (taffy + reshape) rebuild.
#[derive(Default)]
pub struct SceneBuildCache {
    last: Option<BuiltInputs>,
}

/// Every value `editor_state_to_layout_scene` reads off the `EditorState`:
/// the document, the authored-geometry latch (preview toggles it, changing the
/// layout mode), the active page index, and the resolved variable table — which
/// folds the active theme plus the transient fill / stroke ref caches, i.e. all
/// the non-doc resolution inputs. Everything else on the state (selection /
/// chat / hover / viewport) is dropped by the builder.
struct BuiltInputs {
    doc: PenDocument,
    preserve_authored_geometry: bool,
    active_page_index: usize,
    var_table: VariableTable,
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
    /// scene). Cheap fields are compared before the `O(nodes)` document compare
    /// so a page / theme switch short-circuits.
    pub fn maybe_rebuild(&mut self, state: &op_editor_core::EditorState) -> Option<LayoutScene> {
        let preserve_authored_geometry = state.editor_ui.preserve_authored_geometry;
        let active_page_index = state.ui.active_page_index;
        // Resolves the active theme + transient fill/stroke ref caches + the
        // doc-defined variables — every non-doc input the builder consumes.
        // Proportional to variable count, not node count, so cheap to rebuild
        // per refresh purely for the comparison.
        let var_table = crate::editor_state_var_table(state);
        if let Some(last) = &self.last {
            // Cheapest comparisons first; the O(nodes) document compare last.
            if last.active_page_index == active_page_index
                && last.preserve_authored_geometry == preserve_authored_geometry
                && last.var_table == var_table
                && last.doc == state.doc
            {
                return None;
            }
        }
        let scene = crate::editor_state_to_layout_scene(state);
        self.last = Some(BuiltInputs {
            doc: state.doc.clone(),
            preserve_authored_geometry,
            active_page_index,
            var_table,
        });
        Some(scene)
    }

    /// Forget the cached inputs so the next [`maybe_rebuild`](Self::maybe_rebuild)
    /// always rebuilds. Call when the host installs a scene through a path that
    /// bypasses this cache (otherwise the cache's "last built" would be stale).
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
            r#"{{"version":"0.8.0","children":[
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
        let b = state_with_rect(100.0); // node moved → different doc

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
