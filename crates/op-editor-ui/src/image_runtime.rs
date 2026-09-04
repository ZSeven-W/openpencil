//! Host-facing image cache and decode-queue facade.
//!
//! Canvas painting records image work inside the widget layer, while native
//! and web renderers perform the platform-specific decode. Hosts must use this
//! module for that handoff instead of reaching into `widgets` directly.

pub use crate::widgets::canvas_viewport_image::{
    cached_bytes_for, has_pending_decodes, mark_decode_done, mark_decode_failed,
    note_pending_decode, store_remote_image_bytes, take_pending_decodes, PendingDecode,
};

/// Read intrinsic dimensions without decoding pixels. Upload hosts use this
/// before embedding a picked image so crop initialization uses the real source
/// ratio on native and web.
pub use op_util::encoded_image_dimensions;

#[cfg(test)]
mod tests {
    use super::encoded_image_dimensions;

    #[test]
    fn encoded_headers_report_nonzero_intrinsic_dimensions() {
        let mut png = vec![0; 24];
        png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[8..12].copy_from_slice(&13u32.to_be_bytes());
        png[12..16].copy_from_slice(b"IHDR");
        png[16..20].copy_from_slice(&320u32.to_be_bytes());
        png[20..24].copy_from_slice(&180u32.to_be_bytes());
        assert_eq!(encoded_image_dimensions(&png), Some((320, 180)));

        let mut gif = b"GIF89a".to_vec();
        gif.extend_from_slice(&64u16.to_le_bytes());
        gif.extend_from_slice(&32u16.to_le_bytes());
        assert_eq!(encoded_image_dimensions(&gif), Some((64, 32)));
        assert_eq!(encoded_image_dimensions(b"not an image"), None);
    }

    #[test]
    fn facade_matches_the_shared_leaf_helper() {
        let svg = br#"<svg width="24" viewBox="0 0 24 16"/>"#;
        assert_eq!(encoded_image_dimensions(svg), Some((24, 16)));
        assert_eq!(
            encoded_image_dimensions(svg),
            op_util::encoded_image_dimensions(svg)
        );
    }
}
