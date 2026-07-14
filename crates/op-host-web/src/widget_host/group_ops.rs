use super::WidgetHost;
use op_editor_core::{BooleanOp, NodeId};
use op_editor_ui::Rect;

impl WidgetHost {
    pub fn apply_group(&mut self) -> bool {
        if self.editor_state.selection.is_empty() {
            return false;
        }
        let snap = self.editor_state.snapshot_for_history();
        if self
            .editor_state
            .group_selected(&mut self.next_node_id)
            .is_some()
        {
            self.editor_state.history_push_past(snap);
            self.mark_dirty();
            return true;
        }
        false
    }

    pub fn apply_boolean_op(&mut self, op: BooleanOp) -> bool {
        self.commit_variable_row_focus_if_any();
        self.refresh_layout_scene();
        let page = match self.layout_scene.active_page() {
            Some(page) => page,
            None => return false,
        };
        let selected: Vec<NodeId> = self.editor_state.selection.set.clone();
        if selected.len() < 2 {
            return false;
        }
        let mut rects = Vec::with_capacity(selected.len());
        for id in &selected {
            let Some(node) = page.find(id.as_str()) else {
                return false;
            };
            if !rect_has_extent(node.bounds) {
                return false;
            }
            rects.push(node.bounds);
        }
        let Some(contours) = axis_aligned_boolean_contours(&rects, op) else {
            return false;
        };
        let snap = self.editor_state.snapshot_for_history();
        let new_id = self.editor_state.replace_paths_with_polyline(
            &selected,
            &contours,
            &mut self.next_node_id,
        );
        match new_id {
            Some(id) => {
                self.editor_state.history_push_past(snap);
                self.editor_state.set_single_selection(id);
                self.mark_dirty();
                true
            }
            None => false,
        }
    }
}

fn rect_has_extent(rect: Rect) -> bool {
    rect.size.x > 0.0 && rect.size.y > 0.0
}

fn axis_aligned_boolean_contours(rects: &[Rect], op: BooleanOp) -> Option<Vec<Vec<(f64, f64)>>> {
    if rects.len() < 2 {
        return None;
    }
    match op {
        BooleanOp::Intersect => intersect_contour(rects),
        BooleanOp::Union | BooleanOp::Subtract | BooleanOp::Exclude => {
            grid_boolean_contours(rects, op)
        }
    }
}

fn intersect_contour(rects: &[Rect]) -> Option<Vec<Vec<(f64, f64)>>> {
    let mut left = rects[0].origin.x;
    let mut top = rects[0].origin.y;
    let mut right = rects[0].origin.x + rects[0].size.x;
    let mut bottom = rects[0].origin.y + rects[0].size.y;
    for rect in &rects[1..] {
        left = left.max(rect.origin.x);
        top = top.max(rect.origin.y);
        right = right.min(rect.origin.x + rect.size.x);
        bottom = bottom.min(rect.origin.y + rect.size.y);
    }
    if right <= left || bottom <= top {
        return None;
    }
    Some(vec![rect_contour(left, top, right, bottom)])
}

fn grid_boolean_contours(rects: &[Rect], op: BooleanOp) -> Option<Vec<Vec<(f64, f64)>>> {
    let mut xs = Vec::with_capacity(rects.len() * 2);
    let mut ys = Vec::with_capacity(rects.len() * 2);
    for rect in rects {
        xs.push(rect.origin.x);
        xs.push(rect.origin.x + rect.size.x);
        ys.push(rect.origin.y);
        ys.push(rect.origin.y + rect.size.y);
    }
    sort_unique(&mut xs);
    sort_unique(&mut ys);
    if xs.len() < 2 || ys.len() < 2 {
        return None;
    }
    let cols = xs.len() - 1;
    let rows = ys.len() - 1;
    let mut included = vec![vec![false; cols]; rows];
    for y in 0..rows {
        for x in 0..cols {
            let cx = (xs[x] + xs[x + 1]) * 0.5;
            let cy = (ys[y] + ys[y + 1]) * 0.5;
            let coverage = rects
                .iter()
                .filter(|rect| point_in_rect(cx, cy, **rect))
                .count();
            included[y][x] = match op {
                BooleanOp::Union => coverage > 0,
                BooleanOp::Subtract => point_in_rect(cx, cy, rects[0]) && coverage == 1,
                BooleanOp::Exclude => coverage % 2 == 1,
                BooleanOp::Intersect => false,
            };
        }
    }
    trace_grid_contours(&xs, &ys, &included)
}

fn sort_unique(values: &mut Vec<f32>) {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    values.dedup_by(|a, b| (*a - *b).abs() < f32::EPSILON);
}

fn point_in_rect(x: f32, y: f32, rect: Rect) -> bool {
    x >= rect.origin.x
        && x < rect.origin.x + rect.size.x
        && y >= rect.origin.y
        && y < rect.origin.y + rect.size.y
}

fn rect_contour(left: f32, top: f32, right: f32, bottom: f32) -> Vec<(f64, f64)> {
    vec![
        (left as f64, top as f64),
        (right as f64, top as f64),
        (right as f64, bottom as f64),
        (left as f64, bottom as f64),
    ]
}

fn trace_grid_contours(
    xs: &[f32],
    ys: &[f32],
    included: &[Vec<bool>],
) -> Option<Vec<Vec<(f64, f64)>>> {
    let rows = included.len();
    let cols = included.first().map_or(0, Vec::len);
    let mut edges = Vec::<((usize, usize), (usize, usize))>::new();
    for y in 0..rows {
        for x in 0..cols {
            if !included[y][x] {
                continue;
            }
            if y == 0 || !included[y - 1][x] {
                edges.push(((x, y), (x + 1, y)));
            }
            if x + 1 == cols || !included[y][x + 1] {
                edges.push(((x + 1, y), (x + 1, y + 1)));
            }
            if y + 1 == rows || !included[y + 1][x] {
                edges.push(((x + 1, y + 1), (x, y + 1)));
            }
            if x == 0 || !included[y][x - 1] {
                edges.push(((x, y + 1), (x, y)));
            }
        }
    }
    let mut contours = Vec::new();
    while let Some((start, mut current)) = edges.pop() {
        let mut points = vec![start];
        while current != start {
            points.push(current);
            let next_index = edges.iter().position(|(from, _)| *from == current)?;
            let (_, next) = edges.swap_remove(next_index);
            current = next;
        }
        if points.len() >= 3 {
            contours.push(
                points
                    .into_iter()
                    .map(|(x, y)| (xs[x] as f64, ys[y] as f64))
                    .collect(),
            );
        }
    }
    if contours.is_empty() {
        None
    } else {
        Some(contours)
    }
}
