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
