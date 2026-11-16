use std::io::Cursor;

use ::mp4::Mp4Reader;
use rusty_h264_common::YuvFrame;
use rusty_h264_decoder::Decoder;

use super::{export, Case, EditorState, MotionExportFormat, FPS, FRAME_STEP_MS, HEIGHT, WIDTH};

#[derive(Debug, PartialEq, Eq)]
struct Timing {
    timescale: u32,
    duration: std::time::Duration,
    samples: Vec<(u64, u32, i32, bool)>,
}

fn decode(bytes: Vec<u8>) -> (Timing, Vec<YuvFrame>) {
    let size = bytes.len() as u64;
    let mut reader = Mp4Reader::read_header(Cursor::new(bytes), size).unwrap();
    let track = &reader.tracks()[&1];
    let sample_count = reader.sample_count(1).unwrap();
    let mut timing = Timing {
        timescale: track.timescale(),
        duration: track.duration(),
        samples: Vec::with_capacity(sample_count as usize),
    };
    let mut annex_b = Vec::new();
    append_nal(&mut annex_b, track.sequence_parameter_set().unwrap());
    append_nal(&mut annex_b, track.picture_parameter_set().unwrap());
    for index in 1..=sample_count {
        let sample = reader.read_sample(1, index).unwrap().unwrap();
        timing.samples.push((
            sample.start_time,
            sample.duration,
            sample.rendering_offset,
            sample.is_sync,
        ));
        let mut avcc = sample.bytes.as_ref();
        while !avcc.is_empty() {
            let (length, rest) = avcc.split_at(4);
            let length = u32::from_be_bytes(length.try_into().unwrap()) as usize;
            let (nal, rest) = rest.split_at(length);
            append_nal(&mut annex_b, nal);
            avcc = rest;
        }
    }
    let frames = Decoder::new().decode_stream(&annex_b).unwrap();
    assert_eq!(frames.len(), timing.samples.len(), "decoded frame count");
    (timing, frames)
}

#[test]
#[ignore = "supply the browser download and daemon export via OP_BROWSER_MP4 and OP_DAEMON_MP4"]
fn browser_and_daemon_mp4_are_frame_and_timestamp_identical() {
    let read = |name| std::fs::read(std::env::var_os(name).expect(name)).unwrap();
    let (browser_timing, browser) = decode(read("OP_BROWSER_MP4"));
    let (daemon_timing, daemon) = decode(read("OP_DAEMON_MP4"));
    assert_eq!(browser_timing, daemon_timing, "sample clocks and keyframes");
    assert_eq!(browser.len(), daemon.len());
    let mut max_diff = 0;
    let mut worst_mae = 0.0_f64;
    let mut first_difference = None;
    for (index, (browser, daemon)) in browser.iter().zip(&daemon).enumerate() {
        assert_eq!(
            (browser.width, browser.height),
            (daemon.width, daemon.height)
        );
        for (plane, left, right) in [
            ("Y", &browser.y, &daemon.y),
            ("Cb", &browser.u, &daemon.u),
            ("Cr", &browser.v, &daemon.v),
        ] {
            assert_eq!(left.len(), right.len());
            let mut total = 0_u64;
            for (offset, (&a, &b)) in left.iter().zip(right).enumerate() {
                let difference = a.abs_diff(b);
                max_diff = max_diff.max(difference);
                total += u64::from(difference);
                if difference > 0 && first_difference.is_none() {
                    first_difference = Some((index, plane, offset, a, b));
                }
            }
            worst_mae = worst_mae.max(total as f64 / left.len() as f64);
        }
    }
    println!("decoded max difference {max_diff}/255, worst frame/plane MAE {worst_mae:.6}/255");
    assert_eq!(first_difference, None, "first decoded sample mismatch");
    let frame = &browser[0];
    println!(
        "{} frames, {}×{}, timescale {}, duration {:?}, exact Y/Cb/Cr and sample timing",
        browser.len(),
        frame.width,
        frame.height,
        browser_timing.timescale,
        browser_timing.duration
    );
}

pub(super) fn compare(case: &Case, editor: &EditorState, expected: &[Vec<u8>]) {
    let (timing, frames) = decode(export(case, editor, MotionExportFormat::Mp4));
    assert_eq!(timing.timescale, FPS);
    assert_eq!(frames.len(), expected.len());
    for (index, sample) in timing.samples.iter().enumerate() {
        assert_eq!((sample.0, sample.1, sample.2), (index as u64, 1, 0));
    }
    let mut worst_mean = 0.0_f64;
    for (index, (decoded, play)) in frames.iter().zip(expected).enumerate() {
        assert_eq!((decoded.width, decoded.height), (WIDTH, HEIGHT));
        let reference = reference_yuv(play);
        for (plane, actual, expected) in [
            ("Y", &decoded.y, &reference[0]),
            ("Cb", &decoded.u, &reference[1]),
            ("Cr", &decoded.v, &reference[2]),
        ] {
            assert_eq!(actual.len(), expected.len());
            let errors = actual.iter().zip(expected).map(|(a, b)| a.abs_diff(*b));
            let mean = errors.clone().map(f64::from).sum::<f64>() / actual.len() as f64;
            let max = errors.max().unwrap();
            worst_mean = worst_mean.max(mean);
            assert!(
                mean <= 1.0 && max <= 32,
                "{} MP4 {plane} at {}ms: mean {mean:.3}, max {max}",
                case.root,
                index as u64 * FRAME_STEP_MS
            );
        }
    }
    println!(
        "{} MP4: {} decoded frames, worst plane mean absolute error {worst_mean:.3}/255",
        case.root,
        frames.len()
    );
}

fn append_nal(stream: &mut Vec<u8>, nal: &[u8]) {
    stream.extend([0, 0, 0, 1]);
    stream.extend_from_slice(nal);
}

/// Project Play's opaque RGB into limited-range BT.601 and average chroma
/// over each 2x2 block for comparison with the decoded I420 sample.
fn reference_yuv(rgba: &[u8]) -> [Vec<u8>; 3] {
    let luma = |r: f64, g: f64, b: f64| 0.299 * r + 0.587 * g + 0.114 * b;
    let y = rgba
        .chunks_exact(4)
        .map(|pixel| {
            (16.0 + (219.0 / 255.0) * luma(pixel[0].into(), pixel[1].into(), pixel[2].into()))
                .round() as u8
        })
        .collect();
    let mut cb = Vec::with_capacity(WIDTH * HEIGHT / 4);
    let mut cr = Vec::with_capacity(WIDTH * HEIGHT / 4);
    for row in rgba.chunks_exact(WIDTH * 4 * 2) {
        let (top, bottom) = row.split_at(WIDTH * 4);
        for (top, bottom) in top.chunks_exact(8).zip(bottom.chunks_exact(8)) {
            let average = |channel| {
                (f64::from(top[channel])
                    + f64::from(top[channel + 4])
                    + f64::from(bottom[channel])
                    + f64::from(bottom[channel + 4]))
                    / 4.0
            };
            let (r, g, b) = (average(0), average(1), average(2));
            let y = luma(r, g, b);
            cb.push((128.0 + (224.0 / 255.0) * (b - y) / 1.772).round() as u8);
            cr.push((128.0 + (224.0 / 255.0) * (r - y) / 1.402).round() as u8);
        }
    }
    [y, cb, cr]
}
