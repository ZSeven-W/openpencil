//! Inherent state and resource methods for [`CanvasKitBackend`].

use super::backend::CanvasKitBackend;
use super::bindings::OpCk;

impl CanvasKitBackend {
    pub fn new(ck: OpCk, dpr: f32, logical_w: u32, logical_h: u32) -> Self {
        let dpr = dpr.max(1.0);
        ck.set_dpr(dpr);
        Self {
            ck,
            dpr,
            logical_w,
            logical_h,
        }
    }

    pub fn logical_size(&self) -> (f32, f32) {
        (self.logical_w as f32, self.logical_h as f32)
    }

    /// Get a reference to the bridge for preview measurement construction.
    pub fn op_ck(&self) -> &OpCk {
        &self.ck
    }

    pub fn resize_for_display(&mut self, logical_w: u32, logical_h: u32, dpr: f32) {
        self.logical_w = logical_w.max(1);
        self.logical_h = logical_h.max(1);
        self.dpr = dpr.max(1.0);
        self.ck.set_dpr(self.dpr);
        let pw = ((self.logical_w as f32) * self.dpr).round() as u32;
        let ph = ((self.logical_h as f32) * self.dpr).round() as u32;
        self.ck.resize(pw.max(1), ph.max(1));
    }

    /// Register a user-imported font under its selectable family name.
    pub fn register_imported_font(&mut self, family: &str, bytes: &[u8]) -> bool {
        match crate::vf_normalize::with_default_wght_400(bytes) {
            Some(patched) => self.ck.register_imported_font(family, &patched),
            None => self.ck.register_imported_font(family, bytes),
        }
    }

    /// Register a fresh browser import and return its parsed family name.
    pub fn register_imported_font_from_bytes(&mut self, bytes: &[u8]) -> Option<String> {
        let family = crate::font_meta::parse_family(bytes)?;
        self.register_imported_font(&family, bytes)
            .then_some(family)
    }

    /// Register an app-bundled design face below imported and system faces.
    pub fn register_bundled_font(&mut self, family: &str, bytes: &[u8]) -> bool {
        self.ck.register_bundled_font(family, bytes)
    }

    /// Display names of every registered imported family.
    pub fn imported_family_list(&self) -> Vec<String> {
        self.ck.imported_family_list()
    }

    /// Drop a previously imported font face by family name.
    pub fn remove_imported_font(&mut self, family: &str) {
        self.ck.remove_imported_font(family);
    }

    /// Decode a bounded batch requested by the last paint.
    pub fn drain_pending_decodes(&mut self, max: usize) -> usize {
        use crate::image_decode_queue::{finish_web_decode, take_web_decode_batch};
        let batch = take_web_decode_batch(max);
        for job in &batch {
            let decoded = self.ck.decode_image(
                job.id as u32,
                (job.id >> 32) as u32,
                job.bytes.as_ref(),
                job.max_edge_px,
            );
            finish_web_decode(job.id, decoded);
        }
        batch.len()
    }
}
