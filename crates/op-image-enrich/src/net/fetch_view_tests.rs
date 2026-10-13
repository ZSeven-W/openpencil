//! Exercise the real candidate download/preparation/selection seam locally.
use super::*;
use crate::net::{JudgedCandidate, RelevanceVerdict};
use crate::{ImageJudgeFit, ImageJudgeView};
use base64::Engine as _;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};

struct CropJudge {
    deadline: Instant,
    calls: AtomicUsize,
}

impl ImageRelevanceJudge for CropJudge {
    fn supports_deadline_judging(&self) -> bool {
        true
    }
    fn judge(&self, _: &str, _: &str, _: &[Vec<u8>]) -> Vec<JudgedCandidate> {
        panic!("the caller deadline must reach the judge")
    }
    fn judge_before(
        &self,
        query: &str,
        intent: &str,
        images: &[Vec<u8>],
        deadline: Instant,
    ) -> Vec<JudgedCandidate> {
        assert_eq!(deadline, self.deadline);
        assert_eq!(query, "outdoor jogging");
        assert_eq!(intent, "A runner jogging on a tree-lined park path");
        assert_eq!(images.len(), 1);
        let preview = image::load_from_memory(&images[0]).unwrap().to_rgb8();
        assert_eq!(preview.dimensions(), (512, 287));
        assert!(preview.get_pixel(256, 0)[1] > 220);
        assert!(preview.get_pixel(256, 286)[1] > 220);
        self.calls.fetch_add(1, Ordering::Relaxed);
        vec![JudgedCandidate {
            index: 0,
            verdict: RelevanceVerdict::On,
            reason: "test crop inspected".into(),
        }]
    }
}

#[test]
fn cropped_judge_keeps_one_call_original_brief_deadline_and_complete_asset() {
    let source = image::RgbImage::from_fn(960, 1440, |_, y| {
        if y < 400 {
            image::Rgb([240, 10, 10])
        } else if y > 1040 {
            image::Rgb([10, 10, 240])
        } else {
            image::Rgb([10, 240, 10])
        }
    });
    let mut encoded = std::io::Cursor::new(Vec::new());
    source
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let bytes = encoded.into_inner();
    let serving_bytes = bytes.clone();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/thumbnail.png", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = [0_u8; 2048];
        assert!(stream.read(&mut request).unwrap() > 0);
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",serving_bytes.len()).unwrap();
        stream.write_all(&serving_bytes).unwrap();
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    let judge = CropJudge {
        deadline,
        calls: AtomicUsize::new(0),
    };
    let view = ImageJudgeView::new(132.0, 74.0, ImageJudgeFit::Cover).unwrap();
    let slot = JudgeSlot {
        query: "outdoor jogging",
        intent: "A runner jogging on a tree-lined park path",
        judge: &judge,
        deadline: Some(deadline),
        view: Some(view),
    };
    let used = Mutex::new(HashSet::new());
    let seen = Mutex::new(HashSet::new());
    let hit = RawHit {
        id: "portrait".into(),
        thumb_url: url,
        title: "Runner".into(),
        attribution: "test".into(),
        relevance_metadata: String::new(),
        original: None,
    };
    let result = crate::net::block_on_image_runtime(async {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .no_proxy()
            .build()
            .unwrap();
        fetch_judged_openverse(&client, vec![hit], &slot, &used, &seen, true, "test").await
    })
    .unwrap();
    server.join().unwrap();
    assert_eq!(judge.calls.load(Ordering::Relaxed), 1);
    let body = result.src.split_once(',').unwrap().1;
    let saved = base64::engine::general_purpose::STANDARD
        .decode(body)
        .unwrap();
    assert_eq!(saved, bytes, "only the judge derivative may be cropped");
    let original = image::load_from_memory(&saved).unwrap().to_rgb8();
    assert_eq!(original.dimensions(), (960, 1440));
    assert!(original.get_pixel(480, 0)[0] > 220);
    assert!(original.get_pixel(480, 1439)[2] > 220);
}

#[test]
fn expired_caller_deadline_starts_no_search_or_crop_judge() {
    let deadline = Instant::now() - Duration::from_secs(1);
    let judge = CropJudge {
        deadline,
        calls: AtomicUsize::new(0),
    };
    let result = fetch_first_image_url_blocking_with_judge_for_view(
        "outdoor jogging",
        Some(ImageAspectRatio::Wide),
        None,
        &Mutex::new(HashSet::new()),
        &judge,
        "A runner jogging on a tree-lined park path",
        Some(deadline),
        ImageJudgeView::new(132.0, 74.0, ImageJudgeFit::Cover),
    );
    assert!(result.is_none());
    assert_eq!(judge.calls.load(Ordering::Relaxed), 0);
}
