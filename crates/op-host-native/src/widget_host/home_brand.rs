//! Studio Home's 加链接 → brand kit, host arm.
//!
//! The press only records a request (link in the brief, else the newest
//! staged screenshot) on the shared Home state; extraction needs network /
//! image decoding and runs in the shell that owns them (desktop drains
//! [`WidgetHostNative::take_home_brand_request`] and reports back through
//! [`WidgetHostNative::finish_home_brand`]). The kit it produces is applied
//! to the document a send is about to run on — after Home's fresh-document
//! swap, so the brand lands on the new deliverable rather than on the work
//! that was just parked.

use super::WidgetHostNative;
use op_editor_core::{BrandKitPayload, BrandSourceRequest};

impl WidgetHostNative {
    /// The 加链接 press. A no-op on hosts that cannot extract.
    pub(in crate::widget_host) fn home_brand_press(&mut self) {
        let home = &self.editor_state.editor_ui.home;
        if !home.brand.available {
            return;
        }
        let draft = home.draft.clone();
        let image = self
            .editor_state
            .chat
            .pending_attachments
            .iter()
            .rev()
            .find(|attachment| attachment.is_image())
            .map(|a| (a.name.clone(), a.media_type.clone(), a.data.clone()));
        let now_ms = self.now_ms;
        self.editor_state.editor_ui.home.brand.press(
            &draft,
            image
                .as_ref()
                .map(|(name, media, data)| (name.as_str(), media.as_str(), data.as_slice())),
            now_ms,
        );
    }

    /// Shell: take the queued extraction request `(generation, source)`.
    pub fn take_home_brand_request(&mut self) -> Option<(u64, BrandSourceRequest)> {
        self.editor_state.editor_ui.home.brand.take_request()
    }

    /// Shell: report a finished extraction. Returns whether the chip
    /// changed (a stale or cancelled request is dropped).
    pub fn finish_home_brand(
        &mut self,
        generation: u64,
        result: Result<BrandKitPayload, String>,
    ) -> bool {
        let now_ms = self.now_ms;
        let changed = self
            .editor_state
            .editor_ui
            .home
            .brand
            .finish(generation, result, now_ms);
        if changed {
            self.mark_dirty();
        }
        changed
    }

    /// Apply the staged kit (if any) to the current document as one undo
    /// step. Called on the document a Home send runs on, before the turn
    /// is queued, so the run's palette seed finds the brand already there
    /// (existing variables win) and binds generated colours to it.
    pub(in crate::widget_host) fn apply_staged_home_brand(&mut self) -> bool {
        let Some(kit) = self.editor_state.editor_ui.home.brand.staged().cloned() else {
            return false;
        };
        if !self.collab_allows_variables_mutation() {
            return false;
        }
        let applied = self.editor_state.apply_brand_kit(&kit);
        if applied {
            self.mark_dirty();
        }
        applied
    }
}

#[cfg(test)]
#[path = "home_brand_tests.rs"]
mod tests;
