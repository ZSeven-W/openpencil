use super::*;
use std::io::{Cursor, Read};

fn fixture() -> (PenNode, LayoutScene) {
    let mut document = jian_ops_schema::load_str(
        r##"{"version":"1.1","children":[{"type":"rectangle","id":"box","width":16,"height":12,
          "fill":[{"type":"solid","color":"#ff0000"}],
          "animations":[{"trigger":"mount","durationMs":90,"easing":"linear",
          "keyframes":[{"offset":0,"values":{"opacity":0}},{"offset":1,"values":{"opacity":1}}]}]}]}"##,
    ).unwrap().value;
    let scene = op_pen_loader::pen_document_to_layout_scene(&document, &Default::default(), 0);
    (document.children.remove(0), scene)
}

fn export(format: MotionFormat) -> Vec<u8> {
    let (root, scene) = fixture();
    let mut output = Cursor::new(Vec::new());
    export_motion(
        &mut output,
        &scene,
        &root,
        format,
        &MotionExportOptions::default(),
    )
    .unwrap();
    output.into_inner()
}

#[test]
fn png_sequence_captures_start_and_end_on_the_same_canvas() {
    let mut zip = zip::ZipArchive::new(Cursor::new(export(MotionFormat::PngSequence))).unwrap();
    assert_eq!(zip.len(), 4);
    for (name, alpha) in [("frame-0000.png", 0), ("frame-0003.png", 255)] {
        let mut bytes = Vec::new();
        zip.by_name(name).unwrap().read_to_end(&mut bytes).unwrap();
        let mut decoder = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
        let mut pixels = vec![0; decoder.output_buffer_size().unwrap()];
        let info = decoder.next_frame(&mut pixels).unwrap();
        assert_eq!((info.width, info.height), (16, 12));
        assert_eq!(pixels[3], alpha);
    }
}

#[test]
fn apng_keeps_the_frame_count_and_final_short_step() {
    let mut decoder = png::Decoder::new(Cursor::new(export(MotionFormat::Apng)))
        .read_info()
        .unwrap();
    let control = decoder.info().animation_control.unwrap();
    assert_eq!((control.num_frames, control.num_plays), (4, 0));
    let mut pixels = vec![0; decoder.output_buffer_size().unwrap()];
    let delays: Vec<_> = (0..4)
        .map(|_| {
            decoder.next_frame(&mut pixels).unwrap();
            let control = decoder.info().frame_control.unwrap();
            (control.delay_num, control.delay_den)
        })
        .collect();
    assert_eq!(delays, vec![(33, 1000), (33, 1000), (24, 1000), (33, 1000)]);
}

#[test]
fn gif_rounds_at_frame_boundaries_and_repeats() {
    let mut decoder = gif::DecodeOptions::new()
        .read_info(Cursor::new(export(MotionFormat::Gif)))
        .unwrap();
    let mut delays = Vec::new();
    while let Some(frame) = decoder.read_next_frame().unwrap() {
        delays.push(frame.delay);
    }
    assert_eq!(decoder.repeat(), gif::Repeat::Infinite);
    assert_eq!(delays, [3, 4, 2, 3]);
}

#[test]
fn motion_limits_are_checked_before_the_writer_is_used() {
    let (root, scene) = fixture();
    let mut output = Cursor::new(Vec::new());
    let options = MotionExportOptions {
        duration_ms: Some(u64::MAX),
        ..Default::default()
    };
    assert!(matches!(
        export_motion(&mut output, &scene, &root, MotionFormat::Apng, &options),
        Err(ExportError::TooManyMotionFrames { .. })
    ));
    assert!(output.into_inner().is_empty());
}

#[test]
fn contact_sheet_preserves_order_crop_and_exact_samples() {
    let (root, scene) = fixture();
    let sheet =
        render_motion_contact_sheet(&scene, &root, &ClipTrigger::Mount, &[90, 0, 45], 1.0).unwrap();
    let mut decoder = png::Decoder::new(Cursor::new(sheet.png))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; decoder.output_buffer_size().unwrap()];
    let info = decoder.next_frame(&mut pixels).unwrap();
    assert_eq!((info.width, info.height), (40, 32));
    assert_eq!(
        sheet
            .frames
            .iter()
            .map(|frame| frame.time_ms)
            .collect::<Vec<_>>(),
        [90, 0, 45]
    );
    assert_eq!((sheet.frames[1].x, sheet.frames[1].y), (24, 0));
    assert_eq!((sheet.frames[2].x, sheet.frames[2].y), (0, 20));
    for (frame, alpha) in sheet.frames.iter().zip([255, 0, 128]) {
        let pos = (frame.y as usize * info.line_size) + (frame.x as usize * 4);
        assert_eq!(pixels[pos + 3], alpha);
    }
}

#[test]
fn reduced_export_keeps_terminal_fills_and_white_background_at_the_first_frame() {
    let (root, scene) = fixture();
    for (reduced_motion, rgb) in [(false, [255, 255, 255]), (true, [255, 0, 0])] {
        let mut output = Cursor::new(Vec::new());
        let options = MotionExportOptions {
            background: MotionExportBackground::White,
            reduced_motion,
            ..MotionExportOptions::default()
        };
        export_motion(
            &mut output,
            &scene,
            &root,
            MotionFormat::PngSequence,
            &options,
        )
        .unwrap();
        let mut zip = zip::ZipArchive::new(Cursor::new(output.into_inner())).unwrap();
        let mut png = Vec::new();
        zip.by_name("frame-0000.png")
            .unwrap()
            .read_to_end(&mut png)
            .unwrap();
        let mut reader = png::Decoder::new(Cursor::new(png)).read_info().unwrap();
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        reader.next_frame(&mut pixels).unwrap();
        assert_eq!(&pixels[..3], rgb);
    }
}

#[test]
fn mp4_rejects_clips_over_ten_minutes() {
    let (root, scene) = fixture();
    let mut output = Cursor::new(Vec::new());
    let too_long = MotionExportOptions {
        duration_ms: Some(op_editor_core::motion_export::MAX_MP4_DURATION_MS + 1),
        ..Default::default()
    };
    assert!(matches!(
        export_motion(&mut output, &scene, &root, MotionFormat::Mp4, &too_long),
        Err(ExportError::MotionDurationTooLong { .. })
    ));
    assert!(output.into_inner().is_empty());
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
#[test]
fn rust_mp4_writes_even_dimensions_and_an_exact_sample_clock() {
    let (root, mut scene) = fixture();
    scene.pages[0].children[0].bounds.size.x = 65.0;
    scene.pages[0].children[0].bounds.size.y = 65.0;
    let mut output = Cursor::new(Vec::new());
    export_motion(
        &mut output,
        &scene,
        &root,
        MotionFormat::Mp4,
        &Default::default(),
    )
    .unwrap();
    let bytes = output.into_inner();
    let mut reader =
        ::mp4::Mp4Reader::read_header(Cursor::new(&bytes), bytes.len() as u64).unwrap();
    let track = &reader.tracks()[&1];
    assert_eq!((track.width(), track.height()), (66, 66));
    assert_eq!(track.timescale(), 30);
    assert_eq!(reader.sample_count(1).unwrap(), 4);
    for sample_id in 1..=4 {
        let sample = reader.read_sample(1, sample_id).unwrap().unwrap();
        assert_eq!(sample.start_time, u64::from(sample_id - 1));
        assert_eq!(sample.duration, 1);
        assert_eq!(sample.rendering_offset, 0);
        assert_eq!(sample.is_sync, sample_id == 1);
    }
}
