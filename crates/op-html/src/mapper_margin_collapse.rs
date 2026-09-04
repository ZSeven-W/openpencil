//! Small union-find used to collapse connected vertical margin sets.

use super::{CollapseSet, Hint};

pub(super) struct DisjointSets {
    parent: Vec<usize>,
}

impl DisjointSets {
    pub(super) fn new(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
        }
    }

    pub(super) fn root(&mut self, index: usize) -> usize {
        let parent = self.parent[index];
        if parent == index {
            index
        } else {
            let root = self.root(parent);
            self.parent[index] = root;
            root
        }
    }

    pub(super) fn join(&mut self, left: usize, right: usize) {
        let left = self.root(left);
        let right = self.root(right);
        if left != right {
            self.parent[right] = left;
        }
    }
}

pub(super) fn edge_index(child: usize, top: bool) -> usize {
    child.saturating_mul(2) + usize::from(!top)
}

pub(super) fn choose_component_edge(
    sets: &mut DisjointSets,
    active: &[bool],
    hints: &[Option<Hint>],
    root: usize,
    value: CollapseSet,
) -> Option<(usize, bool)> {
    let prefer_positive = value.value() >= 0.0 && value.positive > f64::EPSILON;
    let mut fallback = None;
    for (index, hint) in hints.iter().enumerate() {
        let Some(hint) = hint else {
            continue;
        };
        for (top, candidate) in [(true, hint.top), (false, hint.bottom)] {
            let edge = edge_index(index, top);
            if !active[edge] || sets.root(edge) != root {
                continue;
            }
            fallback.get_or_insert((index, top));
            let owns_extreme = if prefer_positive {
                (candidate.positive - value.positive).abs() <= f64::EPSILON
            } else {
                (candidate.negative - value.negative).abs() <= f64::EPSILON
            };
            if owns_extreme {
                return Some((index, top));
            }
        }
    }
    fallback
}
