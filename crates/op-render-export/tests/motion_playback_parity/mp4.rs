use std::io::Cursor;

use ::mp4::Mp4Reader;
use rusty_h264_decoder::Decoder;

use super::{export, Case, EditorState, MotionExportFormat, FPS, FRAME_STEP_MS, HEIGHT, WIDTH};

pub(super) fn compare(case: &Case, editor: &EditorState, expected: &[Vec<u8>]) {
    let bytes = export(case, editor, MotionExportFormat::Mp4);
    let size = bytes.len() as u64;
    let mut reader = Mp4Reader::read_header(Cursor::new(bytes), size).unwrap();
    let track = &reader.tracks()[&1];
    assert_eq!(track.timescale(), FPS);
    assert_eq!(reader.sample_count(1).unwrap() as usize, expected.len());
    let mut annex_b = Vec::new();
    append_nal(&mut annex_b, track.sequence_parameter_set().unwrap());
    append_nal(&mut annex_b, track.picture_parameter_set().unwrap());
    for index in 0..expected.len() {
        let sample = reader.read_sample(1, (index + 1) as u32).unwrap().unwrap();
        assert_eq!(
            (sample.start_time, sample.duration, sample.rendering_offset),
            (index as u64, 1, 0)
        );
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
    assert_eq!(frames.len(), expected.len(), "decoded frame count");
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
