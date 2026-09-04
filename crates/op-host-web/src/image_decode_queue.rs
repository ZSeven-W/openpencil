//! Native-testable bookkeeping for CanvasKit's frame-budgeted image decode drain.

use op_editor_ui::image_runtime::{
    cached_bytes_for, mark_decode_done, mark_decode_failed, take_pending_decodes,
};
use std::sync::Arc;

/// One drained decode: image id, its encoded bytes, and the longest
/// raster edge the current view needs (device px).
pub(crate) struct WebDecodeJob {
    pub id: u64,
    pub bytes: Arc<[u8]>,
    pub max_edge_px: u32,
}

pub(crate) fn take_web_decode_batch(max: usize) -> Vec<WebDecodeJob> {
    take_pending_decodes(max)
        .into_iter()
        .filter_map(|pending| {
            match cached_bytes_for(pending.id).or_else(|| {
                op_editor_ui::collab_avatar_runtime::cached_collab_avatar_bytes(pending.id)
            }) {
                Some(bytes) => Some(WebDecodeJob {
                    id: pending.id,
                    bytes,
                    max_edge_px: pending.max_edge_px,
                }),
                None => {
                    mark_decode_done(pending.id);
                    None
                }
            }
        })
        .collect()
}

pub(crate) fn finish_web_decode(id: u64, decoded: bool) {
    if decoded {
        mark_decode_done(id);
    } else {
        mark_decode_failed(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_ui::image_runtime::{
        has_pending_decodes, note_pending_decode, store_remote_image_bytes,
    };
    use std::sync::Mutex;

    static DECODE_REGISTRY_LOCK: Mutex<()> = Mutex::new(());

    fn clear_pending_work() {
        for entry in take_pending_decodes(usize::MAX) {
            mark_decode_done(entry.id);
        }
    }

    #[test]
    fn web_decode_batch_takes_at_most_two_and_keeps_remaining_work_queued() {
        let _guard = DECODE_REGISTRY_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        clear_pending_work();
        for id in 0xC100..0xC103 {
            store_remote_image_bytes(id, vec![id as u8]);
            note_pending_decode(id, 256);
        }

        let batch = take_web_decode_batch(2);

        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0].id, 0xC100);
        assert_eq!(batch[0].bytes.as_ref(), &[0x00]);
        assert_eq!(batch[0].max_edge_px, 256);
        assert_eq!(batch[1].id, 0xC101);
        assert_eq!(batch[1].bytes.as_ref(), &[0x01]);
        assert_eq!(batch[1].max_edge_px, 256);
        assert!(has_pending_decodes());
        for job in batch {
            finish_web_decode(job.id, true);
        }
        clear_pending_work();
    }

    #[test]
    fn web_decode_failure_is_not_queued_again() {
        let _guard = DECODE_REGISTRY_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        clear_pending_work();
        let id = 0xC1_BAD;
        store_remote_image_bytes(id, b"not an encoded image".to_vec());
        note_pending_decode(id, 256);
        let batch = take_web_decode_batch(2);
        assert_eq!(batch.len(), 1);

        finish_web_decode(id, false);
        note_pending_decode(id, 256);

        assert!(
            take_web_decode_batch(2).is_empty(),
            "a failed bridge decode must remain negatively cached"
        );
    }

    #[test]
    fn web_decode_batch_accepts_bounded_profile_avatar_bytes() {
        let _decode_guard = DECODE_REGISTRY_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let _ = op_editor_ui::collab_avatar_runtime::begin_collab_avatar_generation(0xACCA_0001);
        let _ = op_editor_ui::collab_avatar_runtime::register_account_avatar_url(None);
        clear_pending_work();
        let mut png = vec![0; 32];
        png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[8..12].copy_from_slice(&13_u32.to_be_bytes());
        png[12..16].copy_from_slice(b"IHDR");
        png[16..20].copy_from_slice(&16_u32.to_be_bytes());
        png[20..24].copy_from_slice(&16_u32.to_be_bytes());
        assert!(
            op_editor_ui::collab_avatar_runtime::install_account_avatar_bytes(
                "web-avatar-revision",
                png
            )
        );
        let image = op_editor_ui::collab_avatar_runtime::account_avatar_image()
            .expect("ready account avatar");
        note_pending_decode(image.image_id, 64);

        let batch = take_web_decode_batch(1);
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].id, image.image_id);
        finish_web_decode(batch[0].id, true);
        clear_pending_work();
    }
}
