use crate::layout_scene::{LayoutScene, SceneNode};
use crate::Rect;
use std::collections::HashMap;

pub const CANVAS_LAYOUT_TRANSITION_MS: u64 = 160;
const EPSILON: f32 = 0.5;

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasLayoutTransition {
    started_at_ms: u64,
    duration_ms: u64,
    start_bounds: HashMap<String, Rect>,
}

impl CanvasLayoutTransition {
    pub fn between(before: &LayoutScene, after: &LayoutScene, now_ms: u64) -> Option<Self> {
        Self::between_excluding(before, after, now_ms, None)
    }

    pub fn between_excluding(
        before: &LayoutScene,
        after: &LayoutScene,
        now_ms: u64,
        excluded_id: Option<&str>,
    ) -> Option<Self> {
        let mut before_bounds = HashMap::new();
        if let Some(page) = before.active_page() {
            for child in &page.children {
                collect_bounds(child, &mut before_bounds);
            }
        }

        let mut start_bounds = HashMap::new();
        if let Some(page) = after.active_page() {
            for child in &page.children {
                collect_changed_starts(child, &before_bounds, &mut start_bounds, excluded_id);
            }
        }
        Self::from_start_bounds(start_bounds, now_ms)
    }

    pub fn from_start_bounds(start_bounds: HashMap<String, Rect>, now_ms: u64) -> Option<Self> {
        (!start_bounds.is_empty()).then_some(Self {
            started_at_ms: now_ms,
            duration_ms: CANVAS_LAYOUT_TRANSITION_MS,
            start_bounds,
        })
    }

    pub fn is_active(&self, now_ms: u64) -> bool {
        now_ms < self.started_at_ms.saturating_add(self.duration_ms)
    }

    pub fn next_deadline_ms(&self, now_ms: u64) -> Option<u64> {
        if !self.is_active(now_ms) {
            return None;
        }
        Some((now_ms.saturating_add(16)).min(self.started_at_ms + self.duration_ms))
    }

    pub fn apply_to_scene(&self, scene: &mut LayoutScene, now_ms: u64) {
        if self.start_bounds.is_empty() {
            return;
        }
        let elapsed = now_ms.saturating_sub(self.started_at_ms);
        let t = (elapsed as f32 / self.duration_ms.max(1) as f32).clamp(0.0, 1.0);
        let progress = ease_out_cubic(t);
        let Some(page) = scene.pages.get_mut(scene.active_page_index) else {
            return;
        };
        for child in &mut page.children {
            apply_node_transition(child, &self.start_bounds, progress);
        }
    }

    pub fn len(&self) -> usize {
        self.start_bounds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.start_bounds.is_empty()
    }
}

fn collect_bounds(node: &SceneNode, out: &mut HashMap<String, Rect>) {
    let rect = node.aggregate_bounds();
    if rect.size.x > 0.0 || rect.size.y > 0.0 {
        out.insert(node.id.clone(), rect);
    }
    for child in &node.children {
        collect_bounds(child, out);
    }
}

fn collect_changed_starts(
    node: &SceneNode,
    before_bounds: &HashMap<String, Rect>,
    out: &mut HashMap<String, Rect>,
    excluded_id: Option<&str>,
) {
    if excluded_id == Some(node.id.as_str()) {
        return;
    }
    let rect = node.aggregate_bounds();
    if let Some(start) = before_bounds.get(&node.id) {
        let moved = (start.origin.x - rect.origin.x).abs() > EPSILON
            || (start.origin.y - rect.origin.y).abs() > EPSILON;
        if moved {
            out.insert(node.id.clone(), *start);
        }
    }
    for child in &node.children {
        collect_changed_starts(child, before_bounds, out, excluded_id);
    }
}

fn apply_node_transition(
    node: &mut SceneNode,
    start_bounds: &HashMap<String, Rect>,
    progress: f32,
) {
    if let Some(start) = start_bounds.get(&node.id) {
        let current = node.aggregate_bounds();
        let remaining = 1.0 - progress;
        translate_scene_subtree(
            node,
            (start.origin.x - current.origin.x) * remaining,
            (start.origin.y - current.origin.y) * remaining,
        );
        return;
    }
    for child in &mut node.children {
        apply_node_transition(child, start_bounds, progress);
    }
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

pub(crate) fn translate_scene_subtree(node: &mut SceneNode, dx: f32, dy: f32) {
    node.bounds.origin.x += dx;
    node.bounds.origin.y += dy;
    if node.aggregate_bounds_cache.size.x > 0.0 || node.aggregate_bounds_cache.size.y > 0.0 {
        node.aggregate_bounds_cache.origin.x += dx;
        node.aggregate_bounds_cache.origin.y += dy;
    }
    for point in &mut node.points {
        point.x += dx;
        point.y += dy;
    }
    for anchor in &mut node.path_anchors {
        anchor.pos.x += dx;
        anchor.pos.y += dy;
        if let Some(handle) = anchor.handle_in.as_mut() {
            handle.x += dx;
            handle.y += dy;
        }
        if let Some(handle) = anchor.handle_out.as_mut() {
            handle.x += dx;
            handle.y += dy;
        }
    }
    for child in &mut node.children {
        translate_scene_subtree(child, dx, dy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_scene::{NodeKind, SceneNode, ScenePage};

    fn scene_with_rect(id: &str, y: f32) -> LayoutScene {
        let mut node = SceneNode::leaf(id, NodeKind::Rect);
        node.bounds = Rect::xywh(10.0, y, 80.0, 40.0);
        LayoutScene {
            pages: vec![ScenePage {
                id: "p".into(),
                name: "Page".into(),
                children: vec![node],
            }],
            active_page_index: 0,
        }
    }

    fn scene_with_group(child_y: f32) -> LayoutScene {
        let mut child = SceneNode::leaf("child", NodeKind::Rect);
        child.bounds = Rect::xywh(10.0, child_y, 80.0, 40.0);
        let mut group = SceneNode::leaf("group", NodeKind::Frame);
        group.bounds = Rect::xywh(0.0, child_y, 100.0, 60.0);
        group.children = vec![child];
        LayoutScene {
            pages: vec![ScenePage {
                id: "p".into(),
                name: "Page".into(),
                children: vec![group],
            }],
            active_page_index: 0,
        }
    }

    #[test]
    fn transition_between_scenes_offsets_changed_nodes_until_finished() {
        let before = scene_with_rect("a", 20.0);
        let mut after = scene_with_rect("a", 120.0);
        let transition = CanvasLayoutTransition::between(&before, &after, 1_000)
            .expect("changed node should create a transition");
        assert_eq!(transition.len(), 1);

        transition.apply_to_scene(&mut after, 1_000);
        let node = after.active_page().unwrap().find("a").unwrap();
        assert!(
            (node.bounds.origin.y - 20.0).abs() < 0.1,
            "transition starts at previous visual y, got {:?}",
            node.bounds
        );

        let mut finished = scene_with_rect("a", 120.0);
        transition.apply_to_scene(&mut finished, 1_200);
        let node = finished.active_page().unwrap().find("a").unwrap();
        assert!(
            (node.bounds.origin.y - 120.0).abs() < 0.1,
            "transition ends at final layout y, got {:?}",
            node.bounds
        );
    }

    #[test]
    fn excluding_node_excludes_its_subtree_from_transition() {
        let before = scene_with_group(20.0);
        let after = scene_with_group(120.0);
        let transition =
            CanvasLayoutTransition::between_excluding(&before, &after, 1_000, Some("group"));

        assert!(
            transition.is_none(),
            "drag overlay should not animate descendants of the excluded node"
        );
    }
}
