//! Preview invalidation consumes Jian's canonical binding classification.

pub use jian_core::binding::InvalidationKind;

/// UI mutations alter paint and hit geometry but never authored layout.
pub(crate) fn from_ui_work(work: jian_core::action::services::UiMutationWork) -> InvalidationKind {
    if work.rebuild_hit_test {
        InvalidationKind::HitTest
    } else if work.redraw {
        InvalidationKind::PaintOnly
    } else {
        InvalidationKind::None
    }
}
