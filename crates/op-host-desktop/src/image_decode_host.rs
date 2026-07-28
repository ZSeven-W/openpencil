//! Worker pool for paint-recorded local image decode requests.

#[path = "image_decode_stats.rs"]
mod image_decode_stats;

use crate::DesktopEvent;
use image_decode_stats::ImageDecodeStats;
use op_editor_ui::widgets::canvas_viewport_image::{
    cached_bytes_for, mark_decode_done, mark_decode_failed, pending_decode_count,
    take_pending_decodes,
};
use op_host_native::{decode_raster_capped, NativeBackend};
use skia_safe::{ConditionallySend, Sendable};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use winit::event_loop::EventLoopProxy;

const WORKER_COUNT: usize = 2;
const MAX_QUEUED_DECODES: usize = 4;

struct DecodeJob {
    id: u64,
    bytes: Arc<[u8]>,
    /// Longest raster edge paint asked for (device px).
    max_edge_px: u32,
}

struct DecodeResult {
    id: u64,
    image: Option<Sendable<skia_safe::Image>>,
    /// Edge the produced raster serves sharply.
    covers_edge_px: u32,
}

pub struct ImageDecodeHost {
    job_tx: Option<Sender<DecodeJob>>,
    result_rx: Receiver<DecodeResult>,
    workers: Vec<JoinHandle<()>>,
    in_flight: usize,
    wake_proxy: Arc<Mutex<Option<EventLoopProxy<DesktopEvent>>>>,
    stats: Option<ImageDecodeStats>,
}

impl ImageDecodeHost {
    pub fn new() -> Self {
        let (job_tx, job_rx) = mpsc::channel::<DecodeJob>();
        let (result_tx, result_rx) = mpsc::channel::<DecodeResult>();
        let job_rx = Arc::new(Mutex::new(job_rx));
        let wake_proxy = Arc::new(Mutex::new(None::<EventLoopProxy<DesktopEvent>>));
        let workers: Vec<JoinHandle<()>> = (0..WORKER_COUNT)
            .filter_map(|index| {
                let job_rx = Arc::clone(&job_rx);
                let result_tx = result_tx.clone();
                let wake_proxy = Arc::clone(&wake_proxy);
                std::thread::Builder::new()
                    .name(format!("op-image-decode-{index}"))
                    .spawn(move || decode_worker(job_rx, result_tx, wake_proxy))
                    .map_err(|err| {
                        eprintln!("openpencil-desktop: spawn image decode worker failed: {err}");
                    })
                    .ok()
            })
            .collect();
        // With no worker to drain the queue, disable submission entirely so
        // `pump` retires pending decodes instead of parking them forever
        // (images stay at thumbnail quality — degraded, not stuck).
        let job_tx = (!workers.is_empty()).then_some(job_tx);
        Self {
            job_tx,
            result_rx,
            workers,
            in_flight: 0,
            wake_proxy,
            stats: ImageDecodeStats::from_env(),
        }
    }

    pub fn set_wake_proxy(&mut self, proxy: EventLoopProxy<DesktopEvent>) {
        if let Ok(mut wake) = self.wake_proxy.lock() {
            *wake = Some(proxy);
        }
    }

    pub fn is_pending(&self) -> bool {
        self.in_flight > 0
    }

    #[cfg(test)]
    pub(crate) fn stats_snapshot(&self) -> Option<(u64, u64, usize, usize, &'static str)> {
        self.stats.as_ref().map(|stats| {
            let snapshot = stats.snapshot();
            (
                snapshot.installs,
                snapshot.reinstalls,
                snapshot.in_flight,
                snapshot.pending,
                snapshot.state,
            )
        })
    }

    /// Install completed rasters, then submit at most four queued ids.
    pub fn pump(&mut self, backend: &mut NativeBackend) -> bool {
        let mut changed = self.poll_results(backend);
        let free = MAX_QUEUED_DECODES.saturating_sub(self.in_flight);
        for pending in take_pending_decodes(free) {
            let id = pending.id;
            let Some(bytes) = cached_bytes_for(id)
                .or_else(|| op_editor_ui::collab_avatar_runtime::cached_collab_avatar_bytes(id))
            else {
                mark_decode_done(id);
                continue;
            };
            let Some(tx) = self.job_tx.as_ref() else {
                mark_decode_done(id);
                continue;
            };
            match tx.send(DecodeJob {
                id,
                bytes,
                max_edge_px: pending.max_edge_px,
            }) {
                Ok(()) => {
                    self.in_flight += 1;
                    changed = true;
                }
                Err(_) => mark_decode_done(id),
            }
        }
        if let Some(stats) = self.stats.as_ref() {
            stats.update_queue(self.in_flight, pending_decode_count());
        }
        changed
    }

    fn poll_results(&mut self, backend: &mut NativeBackend) -> bool {
        let mut changed = false;
        while let Ok(result) = self.result_rx.try_recv() {
            self.in_flight = self.in_flight.saturating_sub(1);
            if let Some(image) = result.image {
                backend.install_raster_image(result.id, image.into_inner(), result.covers_edge_px);
                if let Some(stats) = self.stats.as_mut() {
                    stats.record_install(result.id);
                }
                mark_decode_done(result.id);
            } else {
                mark_decode_failed(result.id);
            }
            changed = true;
        }
        changed
    }
}

impl Drop for ImageDecodeHost {
    fn drop(&mut self) {
        self.job_tx.take();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn decode_worker(
    jobs: Arc<Mutex<Receiver<DecodeJob>>>,
    results: Sender<DecodeResult>,
    wake_proxy: Arc<Mutex<Option<EventLoopProxy<DesktopEvent>>>>,
) {
    loop {
        let job = match jobs.lock() {
            Ok(rx) => match rx.recv() {
                Ok(job) => job,
                Err(_) => break,
            },
            Err(_) => break,
        };
        let decoded = decode_raster_capped(&job.bytes, job.max_edge_px);
        let covers_edge_px = decoded.as_ref().map(|(_, covers)| *covers).unwrap_or(0);
        let decoded = decoded.map(|(image, _)| image);
        let thumbnail = decoded.as_ref().and_then(|image| {
            if jian_ops_schema::image_thumbs::thumb_for(job.id).is_none() {
                crate::image_downscale::make_blur_thumbnail_from_image(image)
            } else {
                None
            }
        });
        let image = decoded.and_then(|image| image.wrap_send().ok());
        if let (Some(_), Some(thumbnail)) = (&image, thumbnail) {
            if jian_ops_schema::image_thumbs::thumb_for(job.id).is_none() {
                jian_ops_schema::image_thumbs::store_thumb(job.id, thumbnail);
            }
        }
        if results
            .send(DecodeResult {
                id: job.id,
                image,
                covers_edge_px,
            })
            .is_err()
        {
            break;
        }
        if let Some(proxy) = wake_proxy.lock().ok().and_then(|wake| wake.clone()) {
            let _ = proxy.send_event(DesktopEvent::ImageDecodeReady);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_ui::widgets::canvas_viewport_image::{
        cached_bytes_for, mark_decode_done, note_pending_decode, store_remote_image_bytes,
        take_pending_decodes,
    };
    use op_host_native::NativeBackend;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    static DECODE_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn encode_test_png() -> Vec<u8> {
        let mut surface = skia_safe::surfaces::raster_n32_premul((3, 2)).unwrap();
        surface.canvas().clear(skia_safe::Color::BLUE);
        surface
            .image_snapshot()
            .encode(None, skia_safe::EncodedImageFormat::PNG, 100)
            .unwrap()
            .as_bytes()
            .to_vec()
    }

    fn encode_test_jpeg(width: i32, height: i32) -> Vec<u8> {
        let mut surface = skia_safe::surfaces::raster_n32_premul((width, height)).unwrap();
        surface.canvas().clear(skia_safe::Color::BLUE);
        surface
            .image_snapshot()
            .encode(None, skia_safe::EncodedImageFormat::JPEG, 60)
            .unwrap()
            .as_bytes()
            .to_vec()
    }

    #[test]
    fn worker_round_trip_decodes_and_installs_off_thread() {
        let _guard = DECODE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let id = 0xDEC0_DE01;
        for stale in take_pending_decodes(usize::MAX) {
            mark_decode_done(stale.id);
        }
        let png = encode_test_png();
        store_remote_image_bytes(id, png);
        assert!(cached_bytes_for(id).is_some());
        note_pending_decode(id, 256);

        let mut host = ImageDecodeHost::new();
        let mut backend = NativeBackend::with_dpi(1.0);
        assert!(!backend.image_decoded(id, &[], 64));
        assert!(host.pump(&mut backend), "first pump submits queued decode");

        let deadline = Instant::now() + Duration::from_secs(5);
        while !backend.image_decoded(id, &[], 64) {
            host.pump(&mut backend);
            assert!(Instant::now() < deadline, "decode worker never landed");
            std::thread::yield_now();
        }
        let image = backend.raster_image(id).expect("installed raster");
        assert_eq!(image.dimensions(), (3, 2).into());
    }

    #[test]
    fn worker_uses_the_bounded_avatar_cache_without_gui_decode() {
        let _decode_guard = DECODE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let _avatar_guard = crate::collab_avatar_host::lock_avatar_test_registry();
        let key = "avatar-decode-participant";
        let url = "https://cdn.example/avatar-decode.png";
        op_editor_ui::collab_avatar_runtime::begin_collab_avatar_generation(0xDEC0);
        assert!(op_editor_ui::collab_avatar_runtime::register_collab_avatar_url(key, Some(url)));
        assert!(op_editor_ui::collab_avatar_runtime::collab_avatar_image(key).is_none());
        let request = op_editor_ui::collab_avatar_runtime::take_collab_avatar_requests(1)
            .pop()
            .expect("avatar request queued");
        assert!(
            op_editor_ui::collab_avatar_runtime::complete_collab_avatar_request(
                &request,
                Some(encode_test_png())
            )
        );
        let image = op_editor_ui::collab_avatar_runtime::collab_avatar_image(key)
            .expect("avatar bytes are ready");
        note_pending_decode(image.image_id, 64);

        let mut host = ImageDecodeHost::new();
        let mut backend = NativeBackend::with_dpi(1.0);
        assert!(host.pump(&mut backend), "first pump submits avatar decode");
        let deadline = Instant::now() + Duration::from_secs(5);
        while !backend.image_decoded(image.image_id, &[], 64) {
            host.pump(&mut backend);
            assert!(Instant::now() < deadline, "avatar decode never landed");
            std::thread::yield_now();
        }
        assert_eq!(
            backend
                .raster_image(image.image_id)
                .expect("installed avatar raster")
                .dimensions(),
            (3, 2).into()
        );
    }

    #[test]
    fn successful_worker_decode_installs_a_fallback_thumbnail() {
        let _guard = DECODE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let id = 0xDEC0_7A11;
        for stale in take_pending_decodes(usize::MAX) {
            mark_decode_done(stale.id);
        }
        store_remote_image_bytes(id, encode_test_png());
        note_pending_decode(id, 256);

        let mut host = ImageDecodeHost::new();
        let mut backend = NativeBackend::with_dpi(1.0);
        assert!(host.pump(&mut backend), "first pump submits queued decode");

        let deadline = Instant::now() + Duration::from_secs(5);
        while host.is_pending() {
            host.pump(&mut backend);
            assert!(Instant::now() < deadline, "decode worker never landed");
            std::thread::yield_now();
        }

        let thumb = jian_ops_schema::image_thumbs::thumb_for(id)
            .expect("successful full decode generates a fallback thumbnail");
        assert!(thumb.starts_with(&[0xff, 0xd8]));
        assert!(thumb.len() <= 4 * 1024);
        if let Some(stats) = host.stats_snapshot() {
            assert_eq!(stats.0, 1, "successful install reaches telemetry");
            assert_eq!(stats.1, 0, "the first paint id is not a reinstall");
        }
    }

    #[test]
    fn worker_decode_failure_is_not_queued_again() {
        let _guard = DECODE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let id = 0xDEC0_BAD1;
        for stale in take_pending_decodes(usize::MAX) {
            mark_decode_done(stale.id);
        }
        store_remote_image_bytes(id, b"not an encoded image".to_vec());
        note_pending_decode(id, 256);

        let mut host = ImageDecodeHost::new();
        let mut backend = NativeBackend::with_dpi(1.0);
        assert!(host.pump(&mut backend), "first pump submits queued decode");

        let deadline = Instant::now() + Duration::from_secs(5);
        while host.is_pending() {
            host.pump(&mut backend);
            assert!(Instant::now() < deadline, "failed decode never completed");
            std::thread::yield_now();
        }

        assert!(!backend.image_decoded(id, &[], 64));
        note_pending_decode(id, 256);
        assert!(
            take_pending_decodes(1).is_empty(),
            "a failed payload must remain negatively cached"
        );
    }

    #[test]
    fn paint_thread_rejects_oversized_thumbnail_dimensions_before_rasterizing() {
        let mut backend = NativeBackend::with_dpi(1.0);
        let mut surface = skia_safe::surfaces::raster_n32_premul((20, 20)).unwrap();
        let oversized = encode_test_jpeg(64, 64);
        assert!(oversized.len() <= 4 * 1024);

        op_host_native::begin_image_paint_diagnostics();
        backend.draw_image_thumb(
            surface.canvas(),
            op_editor_ui::Rect::xywh(0.0, 0.0, 20.0, 20.0),
            0xDEC0_0032,
            &oversized,
        );
        backend.draw_image_thumb(
            surface.canvas(),
            op_editor_ui::Rect::xywh(0.0, 0.0, 20.0, 20.0),
            0xDEC0_0032,
            &encode_test_jpeg(16, 16),
        );
        let diagnostics = op_host_native::end_image_paint_diagnostics();

        assert_eq!(
            diagnostics.successful_thumbnail_draws, 0,
            "oversized dimensions are rejected and the id stays negatively cached"
        );
        assert_eq!(diagnostics.paint_thread_full_decodes, 0);
    }
}
