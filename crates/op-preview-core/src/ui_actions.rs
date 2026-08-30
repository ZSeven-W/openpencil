//! Preview-owned state for typed Jian UI mutations.
//!
//! The sink never edits the source document. Visibility is retained as a
//! reversible override over the authored value, scroll requests stay ordered,
//! and each accepted mutation accumulates the redraw/hit-test work R6 consumes.

use jian_core::action::services::{
    ScrollAlignment, UiMutationOutcome, UiMutationRequest, UiMutationSink, UiMutationWork,
};
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisibilityOverride {
    Set(bool),
    Invert,
}

#[derive(Default)]
struct PreviewUiActionInner {
    visibility: BTreeMap<String, VisibilityOverride>,
    scroll_requests: VecDeque<(String, ScrollAlignment)>,
    pending_work: UiMutationWork,
}

/// Retained Preview-only state and the Jian-facing mutation sink.
#[derive(Clone, Default)]
pub(crate) struct PreviewUiActions {
    inner: Rc<RefCell<PreviewUiActionInner>>,
}

impl PreviewUiActions {
    /// Resolve the retained override against the authored visibility.
    pub(crate) fn visibility_for(&self, node_id: &str, authored: bool) -> bool {
        match self.inner.borrow().visibility.get(node_id) {
            Some(VisibilityOverride::Set(visible)) => *visible,
            Some(VisibilityOverride::Invert) => !authored,
            None => authored,
        }
    }

    /// Drain ordered scroll requests for the host/R6 overlay.
    pub(crate) fn drain_scroll_requests(&self) -> Vec<(String, ScrollAlignment)> {
        self.inner.borrow_mut().scroll_requests.drain(..).collect()
    }

    /// Take and clear the accumulated invalidation work.
    pub(crate) fn take_work(&self) -> UiMutationWork {
        std::mem::take(&mut self.inner.borrow_mut().pending_work)
    }
}

impl UiMutationSink for PreviewUiActions {
    fn apply(&self, request: &UiMutationRequest) -> UiMutationOutcome {
        let mut inner = self.inner.borrow_mut();
        match request {
            UiMutationRequest::SetVisibility { node_id, visible } => {
                inner
                    .visibility
                    .insert(node_id.clone(), VisibilityOverride::Set(*visible));
            }
            UiMutationRequest::ToggleVisibility { node_id } => {
                let next = match inner.visibility.get(node_id) {
                    Some(VisibilityOverride::Set(visible)) => {
                        Some(VisibilityOverride::Set(!visible))
                    }
                    Some(VisibilityOverride::Invert) => None,
                    None => Some(VisibilityOverride::Invert),
                };
                if let Some(next) = next {
                    inner.visibility.insert(node_id.clone(), next);
                } else {
                    inner.visibility.remove(node_id);
                }
            }
            UiMutationRequest::ScrollTo {
                target_id,
                alignment,
            } => {
                inner
                    .scroll_requests
                    .push_back((target_id.clone(), *alignment));
            }
        }
        inner
            .pending_work
            .merge(UiMutationWork::REDRAW_AND_HIT_TEST);
        UiMutationOutcome::Applied(UiMutationWork::REDRAW_AND_HIT_TEST)
    }
}
