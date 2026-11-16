//! Encode opaque motion frames as H.264 and mux them into a seekable MP4.

use std::io::{Seek, Write};

use mp4::{AvcConfig, FourCC, Mp4Config, Mp4Sample, Mp4Writer, TrackConfig};
use rusty_h264_common::nal::{split_annex_b, NalUnitType};
use rusty_h264_common::{Profile, YuvFrame};
use rusty_h264_encoder::{Encoder, EncoderConfig};

use super::super::render::FrameRenderer;
use crate::{ExportError, MAX_RASTER_SIDE_PX, MAX_RASTER_TOTAL_PX};

pub(in crate::motion) fn encode<W: Write + Seek>(
    writer: W,
    renderer: &FrameRenderer,
    fps: u32,
    frames: u64,
    time_at: impl Fn(u64) -> u64,
) -> Result<(), ExportError> {
    let (width, height) = renderer.size();
    let (width, height) = (width as usize, height as usize);
    let (padded_width, padded_height) = (width.next_multiple_of(2), height.next_multiple_of(2));
    let (width_px, height_px) = (padded_width as i64, padded_height as i64);
    if width_px > MAX_RASTER_SIDE_PX
        || height_px > MAX_RASTER_SIDE_PX
        || width_px * height_px > MAX_RASTER_TOTAL_PX
    {
        return Err(ExportError::OutputTooLarge {
            width_px,
            height_px,
        });
    }
    let mut config = EncoderConfig::baseline(padded_width, padded_height);
    config.profile = Profile::High;
    config.cabac = true;
    config.transform_8x8 = true;
    config.level_idc = video_level(padded_width, padded_height, fps)?;
    config.qp = 22;
    config.gop_size = fps * 2;
    config.framerate = fps as f32;
    // Keep one access unit per call in presentation order, so no frame
    // queues or CTS offsets are needed when writing the MP4 track.
    let mut encoder = Encoder::new(config).map_err(encoding_error)?;
    let mut rgba = vec![0; width * height * 4];
    let mut yuv = YuvFrame::black(padded_width, padded_height);
    let mut encode_frame = |index| {
        renderer.read_rgba(time_at(index), &mut rgba)?;
        convert_to_i420(&rgba, width, height, &mut yuv);
        encoder.try_encode(&yuv).map_err(encoding_error)
    };

    let first = encode_frame(0)?;
    let first_nals = split_annex_b(&first);
    let params = |kind| {
        first_nals
            .iter()
            .find(|nal| nal_type(nal) == kind)
            .map(|nal| nal.to_vec())
            .ok_or_else(|| encoding_error("missing AVC parameter set"))
    };
    let mut track = TrackConfig::from(AvcConfig {
        width: padded_width as u16,
        height: padded_height as u16,
        seq_param_set: params(NalUnitType::Sps)?,
        pic_param_set: params(NalUnitType::Pps)?,
    });
    track.timescale = fps;
    let mux = Mp4Config {
        major_brand: FourCC::from(*b"isom"),
        minor_version: 512,
        compatible_brands: [*b"isom", *b"iso2", *b"avc1", *b"mp41"]
            .map(FourCC::from)
            .to_vec(),
        timescale: fps,
    };
    let mut mp4 = Mp4Writer::write_start(writer, &mux).map_err(encoding_error)?;
    mp4.add_track(&track).map_err(encoding_error)?;
    write_access_unit(&mut mp4, &first_nals, 0)?;
    for index in 1..frames {
        let encoded = encode_frame(index)?;
        write_access_unit(&mut mp4, &split_annex_b(&encoded), index)?;
    }
    mp4.write_end().map_err(encoding_error)
}

fn nal_type(nal: &[u8]) -> NalUnitType {
    NalUnitType::from_id(nal[0])
}

fn write_access_unit<W: Write + Seek>(
    writer: &mut Mp4Writer<W>,
    nals: &[&[u8]],
    index: u64,
) -> Result<(), ExportError> {
    let mut bytes = Vec::new();
    let mut sync = false;
    let mut has_picture = false;
    for nal in nals {
        match nal_type(nal) {
            // The avc1 sample entry carries the SPS and PPS.
            NalUnitType::Sps | NalUnitType::Pps | NalUnitType::AccessUnitDelimiter => continue,
            NalUnitType::IdrSlice => {
                sync = true;
                has_picture = true;
            }
            NalUnitType::NonIdrSlice => has_picture = true,
            _ => {}
        }
        let len = u32::try_from(nal.len()).map_err(encoding_error)?;
        bytes.extend_from_slice(&len.to_be_bytes());
        bytes.extend_from_slice(nal);
    }
    if !has_picture {
        return Err(encoding_error(
            "encoder did not return a picture for the frame",
        ));
    }
    writer
        .write_sample(
            1,
            &Mp4Sample {
                start_time: index,
                duration: 1,
                rendering_offset: 0,
                is_sync: sync,
                bytes: bytes.into(),
            },
        )
        .map_err(encoding_error)
}

fn convert_to_i420(rgba: &[u8], width: usize, height: usize, frame: &mut YuvFrame) {
    // Limited-range BT.601, with a box filter over each 2×2 chroma block.
    let stride = frame.width;
    for y in (0..frame.height).step_by(2) {
        for x in (0..frame.width).step_by(2) {
            let (mut red, mut green, mut blue) = (0, 0, 0);
            for dy in 0..2 {
                for dx in 0..2 {
                    let (px, py) = (x + dx, y + dy);
                    let (r, g, b) = if px < width && py < height {
                        let offset = (py * width + px) * 4;
                        (
                            i32::from(rgba[offset]),
                            i32::from(rgba[offset + 1]),
                            i32::from(rgba[offset + 2]),
                        )
                    } else {
                        // The opaque export canvas pads odd edges with white.
                        (255, 255, 255)
                    };
                    frame.y[py * stride + px] =
                        (((66 * r + 129 * g + 25 * b + 128) >> 8) + 16) as u8;
                    red += r;
                    green += g;
                    blue += b;
                }
            }
            let index = (y / 2) * (stride / 2) + x / 2;
            frame.u[index] = (((-38 * red - 74 * green + 112 * blue + 512) >> 10) + 128) as u8;
            frame.v[index] = (((112 * red - 94 * green - 18 * blue + 512) >> 10) + 128) as u8;
        }
    }
}

fn video_level(width: usize, height: usize, fps: u32) -> Result<u8, ExportError> {
    // H.264 Annex A picture-size and macroblock-rate limits.
    const LEVELS: &[(u8, usize, usize)] = &[
        (30, 1_620, 40_500),
        (31, 3_600, 108_000),
        (32, 5_120, 216_000),
        (40, 8_192, 245_760),
        (42, 8_704, 522_240),
        (50, 22_080, 589_824),
        (51, 36_864, 983_040),
        (52, 36_864, 2_073_600),
        (60, 139_264, 4_177_920),
        (61, 139_264, 8_355_840),
        (62, 139_264, 16_711_680),
    ];
    let (columns, rows) = (width.div_ceil(16), height.div_ceil(16));
    let macroblocks = columns * rows;
    let largest_side = columns.max(rows);
    LEVELS
        .iter()
        .find_map(|&(level, size, rate)| {
            (macroblocks <= size
                && macroblocks * fps as usize <= rate
                && largest_side * largest_side <= 8 * size)
                .then_some(level)
        })
        .ok_or_else(|| encoding_error("resolution and frame rate exceed H.264 level limits"))
}

fn encoding_error(error: impl std::fmt::Display) -> ExportError {
    ExportError::MotionEncoding {
        format: "MP4",
        reason: error.to_string(),
    }
}
