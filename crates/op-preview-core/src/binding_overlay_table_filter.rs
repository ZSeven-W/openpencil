//! Reflow only rows explicitly owned by the generated table-filter contract.

use super::*;

impl BindingOverlay {
    pub(super) fn filter_invalidation(
        &self,
        sites: &[BindingSite],
        before: &[serde_json::Value],
        after: &[serde_json::Value],
    ) -> InvalidationKind {
        let kind = Self::changed_invalidation(sites, before, after);
        let inner = self.inner.borrow();
        if sites
            .iter()
            .zip(before.iter().zip(after))
            .any(|(site, (a, b))| {
                a != b
                    && site.target == BindingTarget::Visible
                    && inner.filter_row_ids.contains(&site.node_id)
            })
        {
            kind.merge(InvalidationKind::Relayout)
        } else {
            kind
        }
    }

    fn filtered_layout_document(
        &self,
        authored: &jian_ops_schema::PenDocument,
        sites: &[BindingSite],
        state: &jian_core::state::StateGraph,
        pointer: &RuntimeValue,
        extra: &BTreeMap<(String, BindingTarget), serde_json::Value>,
    ) -> Option<jian_ops_schema::PenDocument> {
        if self.inner.borrow().filter_row_ids.is_empty() {
            return None;
        }
        let mut value = serde_json::to_value(authored).ok()?;
        materialize_json_nodes(&mut value, sites, self, state, pointer, extra);
        op_editor_core::table_filter_contract::materialize(&mut value);
        serde_json::from_value(value).ok()
    }

    fn clamp_scroll_after_filter(&self, scene: &LayoutScene) {
        let ids: Vec<_> = self.inner.borrow().scroll_values.keys().cloned().collect();
        let limits: Vec<_> = ids
            .iter()
            .map(|id| (id, self.max_offset(scene, id)))
            .collect();
        let mut inner = self.inner.borrow_mut();
        for (id, limit) in limits {
            if let Some(scroll) = inner.scroll_values.get_mut(id) {
                scroll.max_offset = limit;
                scroll.offset = scroll.offset.clamp(0.0, limit);
            }
        }
        inner.scroll_revision = inner.scroll_revision.wrapping_add(1);
    }
}

impl crate::session::PreviewSession {
    pub(crate) fn materialize_initial_table_filter(&mut self) {
        let active = {
            let inner = self.binding_overlay.inner.borrow();
            self.binding_sites.iter().any(|site| {
                site.target == BindingTarget::Visible
                    && inner.filter_row_ids.contains(&site.node_id)
            })
        };
        if active {
            self.apply_invalidation(InvalidationKind::Relayout);
        }
    }

    pub(super) fn bound_focus_id(&self) -> Option<String> {
        let key = self.runtime.focus.current()?;
        let node = self.runtime.document.as_ref()?.tree.nodes.get(key)?;
        Some(op_editor_core::PenNodeExt::id_str(&node.schema).to_owned())
    }

    pub(super) fn restore_bound_focus(&mut self, id: Option<String>) {
        let key = id.and_then(|id| self.runtime.document.as_ref()?.tree.by_id.get(&id).copied());
        if let Some(key) = key {
            let _ = self.runtime.focus_request(key);
            self.seed_focused_widget_state();
        }
    }

    pub(super) fn refresh_filtered_scene(
        &mut self,
        pointer: &RuntimeValue,
        extra: &BTreeMap<(String, BindingTarget), serde_json::Value>,
    ) {
        let Some(layout) = self.binding_overlay.filtered_layout_document(
            &self.layout_doc,
            &self.binding_sites,
            &self.runtime.state,
            pointer,
            extra,
        ) else {
            return;
        };
        let Some(paint) = self.binding_overlay.materialized_runtime_document(
            &self.binding_sites,
            &self.runtime.state,
            pointer,
            extra,
        ) else {
            return;
        };
        let theme = self
            .app
            .as_ref()
            .map(|app| app.theme.clone())
            .unwrap_or_default();
        let page = self.app.as_ref().map_or(0, |app| app.page_idx);
        self.scene = op_pen_loader::pen_document_to_layout_scene_for_preview(
            &paint,
            &layout,
            self.preserve_authored_geometry,
            &theme,
            page,
        );
        self.binding_overlay.clamp_scroll_after_filter(&self.scene);
    }
}
