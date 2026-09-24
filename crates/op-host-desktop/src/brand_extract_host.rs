//! Desktop arm of Studio Home's 加链接 → brand kit.
//!
//! Home records the request on the shared state; this drains it onto a
//! worker thread (website fetch through the SSRF-screened importer fetcher,
//! or a staged screenshot's bytes), and hands the finished kit back. The
//! UI thread never blocks on the network.

use std::sync::mpsc::{self, Receiver, Sender};

use op_editor_core::{BrandKitPayload, BrandSourceRequest};
use op_host_native::widget_host::WidgetHostNative;
use op_host_services::brand_extract::{
    brand_kit_payload, extract_brand_from_image_bytes, extract_brand_from_url,
};

type BrandResult = (u64, Result<BrandKitPayload, String>);

/// In-flight extractions. One channel serves every job; a result for a
/// request the user already replaced is dropped by the Home state itself.
pub(crate) struct BrandJobs {
    tx: Sender<BrandResult>,
    rx: Receiver<BrandResult>,
    in_flight: usize,
}

impl BrandJobs {
    pub(crate) fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            in_flight: 0,
        }
    }

    pub(crate) fn is_pending(&self) -> bool {
        self.in_flight > 0
    }

    /// Start a queued request and land finished ones. Returns whether the
    /// Home chip changed.
    pub(crate) fn drain(&mut self, host: &mut WidgetHostNative) -> bool {
        if let Some((generation, request)) = host.take_home_brand_request() {
            self.spawn(generation, request);
        }
        let mut changed = false;
        while let Ok((generation, result)) = self.rx.try_recv() {
            self.in_flight = self.in_flight.saturating_sub(1);
            if let Err(detail) = &result {
                eprintln!("[brand] extraction failed: {detail}");
            }
            changed |= host.finish_home_brand(generation, result);
        }
        changed
    }

    fn spawn(&mut self, generation: u64, request: BrandSourceRequest) {
        let tx = self.tx.clone();
        self.in_flight += 1;
        let spawned = std::thread::Builder::new()
            .name("op-brand-extract".into())
            .spawn(move || {
                let _ = tx.send((generation, extract(request)));
            });
        if let Err(error) = spawned {
            // Report through the normal path instead of leaving the chip
            // spinning forever.
            let _ = self
                .tx
                .send((generation, Err(format!("could not start worker: {error}"))));
        }
    }
}

fn extract(request: BrandSourceRequest) -> Result<BrandKitPayload, String> {
    let kit = match request {
        BrandSourceRequest::Url(url) => extract_brand_from_url(&url),
        BrandSourceRequest::Image { name, data, .. } => {
            extract_brand_from_image_bytes(&data, &name)
        }
    };
    kit.map(|kit| brand_kit_payload(&kit))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_staged_screenshot_round_trips_through_the_worker() {
        // A tiny two-colour PNG built in memory: no file, no network.
        let img = image_png();
        let mut host = WidgetHostNative::new();
        let home = &mut host.editor_state_mut().editor_ui.home;
        home.brand.available = true;
        assert!(home
            .brand
            .press("", Some(("shot.png", "image/png", &img)), 1));
        let mut jobs = BrandJobs::new();
        assert!(!jobs.drain(&mut host), "started, nothing landed yet");
        assert!(jobs.is_pending());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while jobs.is_pending() && std::time::Instant::now() < deadline {
            if jobs.drain(&mut host) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let staged = host
            .editor_state()
            .editor_ui
            .home
            .brand
            .staged()
            .expect("kit staged");
        assert_eq!(staged.label, "shot");
        assert!(staged.variables.contains_key("--primary"));
    }

    fn image_png() -> Vec<u8> {
        // 64×40: white page with an orange block.
        let (w, h) = (64u32, 40u32);
        let mut raw = Vec::new();
        for y in 0..h {
            raw.push(0u8); // filter: none
            for x in 0..w {
                let block = (8..32).contains(&x) && (10..24).contains(&y);
                raw.extend_from_slice(if block {
                    &[230, 90, 20]
                } else {
                    &[255, 255, 255]
                });
            }
        }
        encode_png_rgb(w, h, &raw)
    }

    /// Minimal PNG writer (stored deflate blocks) so the test needs no
    /// encoder dependency.
    fn encode_png_rgb(w: u32, h: u32, raw: &[u8]) -> Vec<u8> {
        fn crc32(bytes: &[u8]) -> u32 {
            let mut crc = 0xFFFF_FFFFu32;
            for b in bytes {
                crc ^= u32::from(*b);
                for _ in 0..8 {
                    crc = if crc & 1 != 0 {
                        (crc >> 1) ^ 0xEDB8_8320
                    } else {
                        crc >> 1
                    };
                }
            }
            !crc
        }
        fn adler32(bytes: &[u8]) -> u32 {
            let (mut a, mut b) = (1u32, 0u32);
            for x in bytes {
                a = (a + u32::from(*x)) % 65521;
                b = (b + a) % 65521;
            }
            (b << 16) | a
        }
        fn chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            let mut body = kind.to_vec();
            body.extend_from_slice(data);
            out.extend_from_slice(&body);
            out.extend_from_slice(&crc32(&body).to_be_bytes());
        }
        let mut zlib = vec![0x78, 0x01];
        let blocks: Vec<&[u8]> = raw.chunks(65_535).collect();
        for (i, block) in blocks.iter().enumerate() {
            zlib.push(u8::from(i + 1 == blocks.len()));
            let len = block.len() as u16;
            zlib.extend_from_slice(&len.to_le_bytes());
            zlib.extend_from_slice(&(!len).to_le_bytes());
            zlib.extend_from_slice(block);
        }
        zlib.extend_from_slice(&adler32(raw).to_be_bytes());
        let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&w.to_be_bytes());
        ihdr.extend_from_slice(&h.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
        chunk(&mut out, b"IHDR", &ihdr);
        chunk(&mut out, b"IDAT", &zlib);
        chunk(&mut out, b"IEND", &[]);
        out
    }
}
