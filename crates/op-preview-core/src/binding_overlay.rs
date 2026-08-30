//! Typed Preview binding overlay with read-only scroll and pointer locals.

use crate::binding_sites::BindingSite;
use crate::input_event::ScrollPhase;
use crate::invalidation::InvalidationKind;
use crate::scene_helpers::display_string;
use crate::ui_actions::PreviewUiActions;
use jian_core::action::services::ScrollAlignment;
use jian_core::binding::BindingTarget;
use jian_core::value::RuntimeValue;
use jian_ops_schema::node::PenNode;
use op_editor_ui::layout_scene::{
    LayoutScene, SceneFillLayer, SceneNode, SceneStroke, SceneStrokeAlign,
};
use op_editor_ui::{Color, Rect};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
struct ScrollValues {
    offset: f32,
    max_offset: f32,
    direction: &'static str,
}

impl Default for ScrollValues {
    fn default() -> Self {
        Self {
            offset: 0.0,
            max_offset: 0.0,
            direction: "none",
        }
    }
}

impl ScrollValues {
    fn progress(&self) -> f32 {
        if self.max_offset <= f32::EPSILON {
            0.0
        } else {
            (self.offset / self.max_offset).clamp(0.0, 1.0)
        }
    }

    fn runtime_value(&self) -> RuntimeValue {
        RuntimeValue(serde_json::json!({
            "offset": self.offset,
            "maxOffset": self.max_offset,
            "progress": self.progress(),
            "direction": self.direction,
        }))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct PointerValues {
    x: f32,
    y: f32,
    nx: f32,
    ny: f32,
    inside: bool,
}

impl PointerValues {
    fn from_sample(sample: crate::interaction_state::PointerSample, screen: Option<Rect>) -> Self {
        let (nx, ny, inside) = screen
            .filter(|rect| {
                sample.x.is_finite()
                    && sample.y.is_finite()
                    && rect.origin.x.is_finite()
                    && rect.origin.y.is_finite()
                    && rect.size.x.is_finite()
                    && rect.size.y.is_finite()
                    && rect.size.x > f32::EPSILON
                    && rect.size.y > f32::EPSILON
            })
            .map(|rect| {
                let nx = ((sample.x - rect.origin.x) / rect.size.x).clamp(0.0, 1.0);
                let ny = ((sample.y - rect.origin.y) / rect.size.y).clamp(0.0, 1.0);
                let inside = sample.present
                    && sample.x >= rect.origin.x
                    && sample.x <= rect.origin.x + rect.size.x
                    && sample.y >= rect.origin.y
                    && sample.y <= rect.origin.y + rect.size.y;
                (nx, ny, inside)
            })
            .unwrap_or((0.0, 0.0, false));
        Self {
            x: sample.x,
            y: sample.y,
            nx,
            ny,
            inside,
        }
    }

    fn runtime_value(&self) -> RuntimeValue {
        RuntimeValue(serde_json::json!({
            "x": self.x,
            "y": self.y,
            "nx": self.nx,
            "ny": self.ny,
            "inside": self.inside,
        }))
    }
}

#[derive(Default)]
struct OverlayInner {
    scroll_values: BTreeMap<String, ScrollValues>,
    active_scroll: Option<String>,
    pinned_nodes: BTreeSet<String>,
    sticky_children: BTreeMap<String, BTreeSet<String>>,
    node_scroll_ancestor: BTreeMap<String, String>,
    authored_runtime_document: Option<jian_ops_schema::PenDocument>,
}

#[derive(Default)]
pub(crate) struct BindingOverlay {
    inner: RefCell<OverlayInner>,
}

impl BindingOverlay {
    pub(crate) fn from_document(document: &jian_ops_schema::PenDocument) -> Self {
        let overlay = Self::default();
        overlay.replace_document(document);
        overlay
    }

    pub(crate) fn replace_document(&self, document: &jian_ops_schema::PenDocument) {
        let mut inner = self.inner.borrow_mut();
        let retained_scroll = std::mem::take(&mut inner.scroll_values);
        *inner = OverlayInner::default();
        collect_scroll_metadata(&document.children, None, &mut inner);
        if let Some(pages) = &document.pages {
            for page in pages {
                collect_scroll_metadata(&page.children, None, &mut inner);
            }
        }
        for (node_id, values) in retained_scroll {
            if inner.sticky_children.contains_key(&node_id) {
                inner.scroll_values.insert(node_id, values);
            }
        }
    }

    pub(crate) fn set_runtime_document(&self, document: Option<jian_ops_schema::PenDocument>) {
        self.inner.borrow_mut().authored_runtime_document = document;
    }

    fn materialized_runtime_document(
        &self,
        sites: &[BindingSite],
        state: &jian_core::state::StateGraph,
        pointer: &RuntimeValue,
        extra_values: &BTreeMap<(String, BindingTarget), serde_json::Value>,
    ) -> Option<jian_ops_schema::PenDocument> {
        let authored = self.inner.borrow().authored_runtime_document.clone()?;
        let mut document = serde_json::to_value(authored).ok()?;
        materialize_json_nodes(&mut document, sites, self, state, pointer, extra_values);
        serde_json::from_value(document).ok()
    }

    pub(crate) fn values(
        &self,
        sites: &[BindingSite],
        state: &jian_core::state::StateGraph,
        pointer: &RuntimeValue,
    ) -> Vec<serde_json::Value> {
        sites
            .iter()
            .map(|site| self.evaluate(site, state, pointer).0)
            .collect()
    }

    pub(crate) fn has_visual_state(&self) -> bool {
        self.inner
            .borrow()
            .scroll_values
            .values()
            .any(|values| values.offset > f32::EPSILON)
    }

    pub(crate) fn changed_invalidation(
        sites: &[BindingSite],
        before: &[serde_json::Value],
        after: &[serde_json::Value],
    ) -> InvalidationKind {
        sites
            .iter()
            .zip(before.iter().zip(after))
            .filter(|(_, (before, after))| before != after)
            .fold(InvalidationKind::None, |kind, (site, _)| {
                kind.merge(site.target.invalidation())
            })
    }

    pub(crate) fn apply_to_scene(
        &self,
        scene: &mut LayoutScene,
        sites: &[BindingSite],
        state: &jian_core::state::StateGraph,
        pointer: &RuntimeValue,
        ui_actions: &PreviewUiActions,
    ) {
        self.consume_scroll_requests(scene, ui_actions);
        let Some(page) = scene.pages.get_mut(scene.active_page_index) else {
            return;
        };
        for node in &mut page.children {
            self.apply_node(node, sites, state, pointer, ui_actions);
        }
    }

    fn consume_scroll_requests(&self, scene: &LayoutScene, ui_actions: &PreviewUiActions) {
        for (target_id, alignment) in ui_actions.drain_scroll_requests() {
            let container_id = self
                .inner
                .borrow()
                .node_scroll_ancestor
                .get(&target_id)
                .cloned();
            let Some(container_id) = container_id else {
                continue;
            };
            let Some(page) = scene.active_page() else {
                continue;
            };
            let (Some(container), Some(target)) = (page.find(&container_id), page.find(&target_id))
            else {
                continue;
            };
            let max_offset = self.max_offset(scene, &container_id);
            let current = self
                .inner
                .borrow()
                .scroll_values
                .get(&container_id)
                .map_or(0.0, |values| values.offset);
            let start = target.bounds.origin.y - container.bounds.origin.y;
            let end = target.bounds.origin.y + target.bounds.size.y
                - container.bounds.origin.y
                - container.bounds.size.y;
            let desired = match alignment {
                ScrollAlignment::Start => start,
                ScrollAlignment::Center => {
                    start - (container.bounds.size.y - target.bounds.size.y) / 2.0
                }
                ScrollAlignment::End => end,
                ScrollAlignment::Nearest => {
                    let visible_top = target.bounds.origin.y - current;
                    let visible_bottom = visible_top + target.bounds.size.y;
                    let viewport_top = container.bounds.origin.y;
                    let viewport_bottom = viewport_top + container.bounds.size.y;
                    if visible_top < viewport_top {
                        start
                    } else if visible_bottom > viewport_bottom {
                        end
                    } else {
                        current
                    }
                }
            };
            let mut inner = self.inner.borrow_mut();
            let values = inner.scroll_values.entry(container_id).or_default();
            values.max_offset = max_offset;
            values.offset = desired.clamp(0.0, max_offset);
        }
    }

    fn apply_node(
        &self,
        node: &mut SceneNode,
        sites: &[BindingSite],
        state: &jian_core::state::StateGraph,
        pointer: &RuntimeValue,
        ui_actions: &PreviewUiActions,
    ) {
        let node_id = node.id.clone();
        let mut node_sites: Vec<_> = sites
            .iter()
            .filter(|site| site.node_id == node_id)
            .collect();
        node_sites.sort_by_key(|site| site.target.application_order());
        for site in node_sites {
            let value = RuntimeValue(self.evaluate(site, state, pointer).0);
            apply_value(node, site.target, &value);
        }
        node.hidden = !ui_actions.visibility_for(&node.id, !node.hidden);
        for child in &mut node.children {
            self.apply_node(child, sites, state, pointer, ui_actions);
        }
        self.apply_scroll(node);
    }

    fn evaluate(
        &self,
        site: &BindingSite,
        state: &jian_core::state::StateGraph,
        pointer: &RuntimeValue,
    ) -> RuntimeValue {
        let mut locals = BTreeMap::new();
        if site.uses_scroll {
            let scroll = site
                .scroll_ancestor
                .as_deref()
                .and_then(|node_id| self.inner.borrow().scroll_values.get(node_id).cloned())
                .unwrap_or_default();
            locals.insert("scroll".to_owned(), scroll.runtime_value());
        }
        if site.uses_pointer {
            locals.insert("pointer".to_owned(), pointer.clone());
        }
        site.expr
            .eval_with_locals(state, None, Some(&site.node_id), &locals)
            .0
    }

    pub(crate) fn update_scroll(
        &self,
        node_id: Option<&str>,
        delta_y: f32,
        max_offset: Option<f32>,
        phase: ScrollPhase,
    ) -> bool {
        let mut inner = self.inner.borrow_mut();
        if let Some(node_id) = node_id {
            inner.active_scroll = Some(node_id.to_owned());
        }
        let Some(target) = inner.active_scroll.clone() else {
            return false;
        };
        let changed = {
            let values = inner.scroll_values.entry(target).or_default();
            let before = values.clone();
            if let Some(max_offset) = max_offset {
                values.max_offset = max_offset.max(0.0);
            }
            match phase {
                ScrollPhase::Cancelled => {
                    values.direction = "none";
                }
                ScrollPhase::Ended => {
                    values.offset = (values.offset - delta_y).clamp(0.0, values.max_offset);
                    values.direction = "none";
                }
                ScrollPhase::Began | ScrollPhase::Changed | ScrollPhase::Momentum => {
                    values.offset = (values.offset - delta_y).clamp(0.0, values.max_offset);
                    values.direction = if delta_y < 0.0 {
                        "down"
                    } else if delta_y > 0.0 {
                        "up"
                    } else {
                        values.direction
                    };
                }
            }
            *values != before
        };
        if matches!(phase, ScrollPhase::Ended | ScrollPhase::Cancelled) {
            inner.active_scroll = None;
        }
        changed
    }

    pub(crate) fn max_offset(&self, scene: &LayoutScene, node_id: &str) -> f32 {
        let Some(node) = scene.active_page().and_then(|page| page.find(node_id)) else {
            return 0.0;
        };
        let bottom = node
            .children
            .iter()
            .map(subtree_bottom)
            .fold(node.bounds.origin.y, f32::max);
        (bottom - node.bounds.origin.y - node.bounds.size.y).max(0.0)
    }

    fn apply_scroll(&self, node: &mut SceneNode) {
        let inner = self.inner.borrow();
        let Some(values) = inner.scroll_values.get(&node.id) else {
            return;
        };
        if values.offset <= f32::EPSILON {
            return;
        }
        let sticky = inner.sticky_children.get(&node.id);
        for child in &mut node.children {
            let pinned = inner.pinned_nodes.contains(&child.id)
                || sticky.is_some_and(|ids| ids.contains(&child.id));
            if !pinned {
                translate_for_scroll(child, -values.offset, &inner.pinned_nodes);
            }
        }
        node.children.sort_by_key(|child| {
            inner.pinned_nodes.contains(&child.id)
                || sticky.is_some_and(|ids| ids.contains(&child.id))
        });
    }
}

fn collect_scroll_metadata(
    nodes: &[PenNode],
    inherited_scroll: Option<&str>,
    inner: &mut OverlayInner,
) {
    use op_editor_core::PenNodeExt;
    for node in nodes {
        let node_id = node.id_str();
        let json = serde_json::to_value(node).unwrap_or(serde_json::Value::Null);
        if json.get("pin").and_then(serde_json::Value::as_bool) == Some(true) {
            inner.pinned_nodes.insert(node_id.to_owned());
        }
        let own_scroll = json
            .get("events")
            .and_then(|events| events.get("onScroll"))
            .and_then(serde_json::Value::as_array)
            .is_some_and(|actions| !actions.is_empty())
            .then_some(node_id);
        let nearest_scroll = own_scroll.or(inherited_scroll);
        if let Some(scroll) = nearest_scroll {
            inner
                .node_scroll_ancestor
                .insert(node_id.to_owned(), scroll.to_owned());
        }
        if let Some(scroll) = own_scroll {
            let sticky = json
                .get("stickyChildren")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect();
            inner.sticky_children.insert(scroll.to_owned(), sticky);
        }
        if let Some(children) = node.children() {
            collect_scroll_metadata(children, nearest_scroll, inner);
        }
    }
}

fn materialize_json_nodes(
    value: &mut serde_json::Value,
    sites: &[BindingSite],
    overlay: &BindingOverlay,
    state: &jian_core::state::StateGraph,
    pointer: &RuntimeValue,
    extra_values: &BTreeMap<(String, BindingTarget), serde_json::Value>,
) {
    match value {
        serde_json::Value::Object(object) => {
            let node_id = object
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            if let Some(node_id) = node_id {
                let mut node_sites: Vec<_> = sites
                    .iter()
                    .filter(|site| site.node_id == node_id)
                    .collect();
                node_sites.sort_by_key(|site| site.target.application_order());
                for site in node_sites {
                    let evaluated = overlay.evaluate(site, state, pointer);
                    let _ = jian_core::binding::apply_binding_value(
                        object,
                        site.target,
                        &evaluated,
                        true,
                    );
                }
                let mut extras: Vec<_> = extra_values
                    .iter()
                    .filter(|((target_id, _), _)| target_id == &node_id)
                    .collect();
                extras.sort_by_key(|((_, target), _)| target.application_order());
                for ((_, target), value) in extras {
                    let _ = jian_core::binding::apply_binding_value(
                        object,
                        *target,
                        &RuntimeValue(value.clone()),
                        true,
                    );
                }
            }
            for child in object.values_mut() {
                materialize_json_nodes(child, sites, overlay, state, pointer, extra_values);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                materialize_json_nodes(item, sites, overlay, state, pointer, extra_values);
            }
        }
        _ => {}
    }
}

pub(crate) fn apply_value(node: &mut SceneNode, target: BindingTarget, value: &RuntimeValue) {
    match target {
        BindingTarget::Content => {
            node.text = Some(display_string(value));
            node.text_runs.clear();
        }
        BindingTarget::Value => apply_widget_value(node, value),
        BindingTarget::Checked => {
            if let (Some(widget), Some(checked)) = (node.widget.as_mut(), value.as_bool()) {
                widget.checked = Some(checked);
            }
        }
        BindingTarget::SelectedValue => {
            if let (Some(widget), Some(selected)) = (node.widget.as_mut(), value.as_str()) {
                widget.value_str = Some(selected.to_owned());
            }
        }
        BindingTarget::Visible => {
            if let Some(visible) = value.as_bool() {
                node.hidden = !visible;
            }
        }
        BindingTarget::Opacity => {
            if let Some(opacity) = number(value) {
                node.opacity = opacity.clamp(0.0, 1.0);
            }
        }
        BindingTarget::Fill | BindingTarget::TextColor => {
            if let Some(color) = color(value) {
                node.fill = Some(color);
                if let Some(SceneFillLayer::Solid {
                    color: layer_color, ..
                }) = node.fill_layers.first_mut()
                {
                    *layer_color = color;
                }
            }
        }
        BindingTarget::Stroke => {
            if let Some(color) = color(value) {
                if let Some(stroke) = node.stroke.as_mut() {
                    stroke.color = color;
                } else {
                    node.stroke = Some(SceneStroke {
                        color,
                        width: 1.0,
                        sides: None,
                        align: SceneStrokeAlign::Center,
                    });
                }
            }
        }
        BindingTarget::CornerRadius => {
            if let Some(radius) = number(value) {
                node.corner_radius = radius.max(0.0);
                node.corner_radii = None;
            }
        }
        BindingTarget::X => {
            if let Some(x) = number(value) {
                node.bounds.origin.x = x;
            }
        }
        BindingTarget::Y => {
            if let Some(y) = number(value) {
                node.bounds.origin.y = y;
            }
        }
        BindingTarget::Width => {
            if let Some(width) = number(value) {
                node.bounds.size.x = width.max(0.0);
            }
        }
        BindingTarget::Height => {
            if let Some(height) = number(value) {
                node.bounds.size.y = height.max(0.0);
            }
        }
        BindingTarget::Rotation => {
            if let Some(degrees) = number(value) {
                node.rotation = degrees.to_radians();
            }
        }
        BindingTarget::ScaleX => {
            if let Some(scale) = number(value) {
                scale_axis(node, scale, true);
            }
        }
        BindingTarget::ScaleY => {
            if let Some(scale) = number(value) {
                scale_axis(node, scale, false);
            }
        }
        BindingTarget::Variant | BindingTarget::ActiveState => {
            apply_structural_state(node, value);
        }
    }
}

fn apply_widget_value(node: &mut SceneNode, value: &RuntimeValue) {
    let Some(widget) = node.widget.as_mut() else {
        return;
    };
    match widget.kind.as_str() {
        "switch" | "checkbox" => widget.checked = value.as_bool(),
        "slider" | "progress" | "number_input" => widget.value_num = number(value),
        _ => widget.value_str = value.as_str().map(str::to_owned),
    }
}

fn apply_structural_state(node: &mut SceneNode, value: &RuntimeValue) {
    if let Some(widget) = node.widget.as_mut() {
        match widget.kind.as_str() {
            "switch" | "checkbox" => widget.checked = value.as_bool(),
            _ => widget.value_str = value.as_str().map(str::to_owned),
        }
        return;
    }
    let Some(active) = value.as_str() else {
        return;
    };
    for child in &mut node.children {
        child.hidden = child.id != active;
    }
}

fn number(value: &RuntimeValue) -> Option<f32> {
    value
        .as_f64()
        .map(|number| number as f32)
        .or_else(|| value.as_i64().map(|number| number as f32))
}

fn color(value: &RuntimeValue) -> Option<Color> {
    let color = jian_core::scene::Color::from_hex(value.as_str()?)?;
    Some(Color::rgba_u8(
        color.r(),
        color.g(),
        color.b(),
        f32::from(color.a()) / 255.0,
    ))
}

fn scale_axis(node: &mut SceneNode, scale: f32, horizontal: bool) {
    let scale = scale.max(0.0);
    if horizontal {
        let center = node.bounds.origin.x + node.bounds.size.x / 2.0;
        node.bounds.size.x *= scale;
        node.bounds.origin.x = center - node.bounds.size.x / 2.0;
    } else {
        let center = node.bounds.origin.y + node.bounds.size.y / 2.0;
        node.bounds.size.y *= scale;
        node.bounds.origin.y = center - node.bounds.size.y / 2.0;
    }
}

fn subtree_bottom(node: &SceneNode) -> f32 {
    node.children
        .iter()
        .map(subtree_bottom)
        .fold(node.bounds.origin.y + node.bounds.size.y, f32::max)
}

fn translate_for_scroll(node: &mut SceneNode, dy: f32, pinned_nodes: &BTreeSet<String>) {
    if pinned_nodes.contains(&node.id) {
        return;
    }
    node.bounds.origin.y += dy;
    if node.aggregate_bounds_cache.size.x > 0.0 || node.aggregate_bounds_cache.size.y > 0.0 {
        node.aggregate_bounds_cache.origin.y += dy;
    }
    for point in &mut node.points {
        point.y += dy;
    }
    for anchor in &mut node.path_anchors {
        anchor.pos.y += dy;
        if let Some(handle) = &mut anchor.handle_in {
            handle.y += dy;
        }
        if let Some(handle) = &mut anchor.handle_out {
            handle.y += dy;
        }
    }
    for child in &mut node.children {
        translate_for_scroll(child, dy, pinned_nodes);
    }
}

impl crate::session::PreviewSession {
    pub(crate) fn pointer_binding_value(&self) -> RuntimeValue {
        let sample = self.interaction.pointer_sample();
        let screen = self.root_frames.first().map(|frame| frame.scene_rect);
        PointerValues::from_sample(sample, screen).runtime_value()
    }

    pub(crate) fn binding_values(&self) -> Vec<serde_json::Value> {
        let pointer = self.pointer_binding_value();
        self.binding_overlay
            .values(&self.binding_sites, &self.runtime.state, &pointer)
    }

    pub(crate) fn finish_binding_update(
        &mut self,
        before: &[serde_json::Value],
    ) -> InvalidationKind {
        let after = self.binding_values();
        let invalidation =
            BindingOverlay::changed_invalidation(&self.binding_sites, before, &after);
        self.apply_invalidation(invalidation);
        invalidation
    }

    /// Set one app-state value and return the strongest binding work whose
    /// evaluated value changed.
    pub fn set_state(&mut self, key: &str, value: serde_json::Value) -> InvalidationKind {
        let before = self.binding_values();
        self.runtime.state.app_set(key, value);
        self.runtime.scheduler.flush();
        self.finish_binding_update(&before)
    }

    pub(crate) fn apply_invalidation(&mut self, invalidation: InvalidationKind) {
        self.apply_invalidation_with_values(invalidation, &BTreeMap::new());
    }

    pub(crate) fn apply_invalidation_with_values(
        &mut self,
        invalidation: InvalidationKind,
        extra_values: &BTreeMap<(String, BindingTarget), serde_json::Value>,
    ) {
        match invalidation {
            InvalidationKind::None => {}
            InvalidationKind::PaintOnly => self.runtime.mark_dirty(),
            InvalidationKind::HitTest => {
                self.runtime.rebuild_spatial();
                self.runtime.mark_dirty();
            }
            InvalidationKind::Relayout => {
                let pointer = self.pointer_binding_value();
                if let Some(document) = self.binding_overlay.materialized_runtime_document(
                    &self.binding_sites,
                    &self.runtime.state,
                    &pointer,
                    extra_values,
                ) {
                    if let Err(error) = self.runtime.replace_document(document) {
                        self.warnings
                            .push(format!("preview: binding document swap failed: {error}"));
                    }
                }
                match crate::app_mode::solve_roots(&mut self.runtime, &self.measure) {
                    Ok((frames, available)) => {
                        self.root_frames = frames;
                        self.available = available;
                    }
                    Err(error) => self
                        .warnings
                        .push(format!("preview: binding relayout failed: {error}")),
                }
                self.runtime.mark_dirty();
            }
            InvalidationKind::Navigation => {}
        }
    }

    /// Convert and consume the R5 UI sink's accumulated redraw/hit-test work.
    pub fn take_ui_action_invalidation(&self) -> InvalidationKind {
        self.ui_actions.take_invalidation()
    }
}
