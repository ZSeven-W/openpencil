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
        self.materialize_pagination(&mut value, state);
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

impl BindingOverlay {
    fn pagination_values(
        &self,
        state: &jian_core::state::StateGraph,
    ) -> BTreeMap<String, serde_json::Value> {
        self.inner
            .borrow()
            .paging
            .iter()
            .flat_map(|spec| spec.keys())
            .map(|key| {
                (
                    key.to_owned(),
                    state.app_get(key).map_or(serde_json::Value::Null, |v| v.0),
                )
            })
            .collect()
    }
    pub(super) fn pagination_changed(&self, state: &jian_core::state::StateGraph) -> bool {
        self.pagination_values(state) != self.inner.borrow().paging_signature
    }
    pub(super) fn materialize_pagination(
        &self,
        document: &mut serde_json::Value,
        state: &jian_core::state::StateGraph,
    ) {
        let updates = op_editor_core::table_pagination::materialize(document, &|key| {
            state.app_get(key).map(|v| v.0)
        });
        for (key, value) in updates {
            if state.app_get(&key).map(|v| v.0).as_ref() != Some(&value) {
                state.app_set(&key, value);
            }
        }
        let signature = self.pagination_values(state);
        self.inner.borrow_mut().paging_signature = signature;
    }
}
