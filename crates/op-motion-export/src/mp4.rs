//! Incremental H.264 encoder and MP4 muxer for opaque RGBA frames.

use std::io::{Seek, Write};

use mp4::{AvcConfig, FourCC, Mp4Config, Mp4Sample, Mp4Writer, TrackConfig};
use rusty_h264_common::nal::{split_annex_b, NalUnitType};
use rusty_h264_common::{Profile, YuvFrame};
use rusty_h264_encoder::{Encoder, EncoderConfig};

use crate::{check_raster_size, FrameClock, MotionError};

enum Mux<W> {
    Unopened(W),
    Writing(Mp4Writer<W>),
}

pub struct Mp4Encoder<W> {
    mux: Option<Mux<W>>,
    encoder: Encoder,
    yuv: YuvFrame,
    width: usize,
    height: usize,
    clock: FrameClock,
    index: u64,
}

impl<W: Write + Seek> Mp4Encoder<W> {
    pub fn new(writer: W, size: (i32, i32), clock: FrameClock) -> Result<Self, MotionError> {
        clock.validate_mp4()?;
        if size.0 <= 0 || size.1 <= 0 {
            return Err(encoding_error("invalid frame dimensions"));
        }
        let (width, height) = (size.0 as usize, size.1 as usize);
        let (padded_width, padded_height) = (width.next_multiple_of(2), height.next_multiple_of(2));
        check_raster_size(padded_width as i64, padded_height as i64)?;

        let fps = clock.fps;
        let mut config = EncoderConfig::baseline(padded_width, padded_height);
        config.profile = Profile::High;
        config.cabac = true;
        config.transform_8x8 = true;
        config.level_idc = video_level(padded_width, padded_height, fps)?;
        config.qp = 22;
        config.gop_size = fps * 2;
        config.framerate = fps as f32;
        // Keep one access unit per call in presentation order: no CTS offsets.
        let encoder = Encoder::new(config).map_err(encoding_error)?;
        Ok(Self {
            mux: Some(Mux::Unopened(writer)),
            encoder,
            yuv: YuvFrame::black(padded_width, padded_height),
            width,
            height,
            clock,
            index: 0,
        })
    }

    pub fn push_rgba(&mut self, rgba: &[u8]) -> Result<(), MotionError> {
        if rgba.len() != self.width * self.height * 4 || self.index >= self.clock.frame_count() {
            return Err(encoding_error("unexpected frame size or frame count"));
        }
        convert_to_i420(rgba, self.width, self.height, &mut self.yuv);
        let encoded = self.encoder.try_encode(&self.yuv).map_err(encoding_error)?;
        let nals = split_annex_b(&encoded);
        let mut mux = match self.mux.take() {
            Some(Mux::Writing(mux)) => mux,
            Some(Mux::Unopened(writer)) => self.start_mux(writer, &nals)?,
            None => return Err(encoding_error("the MP4 writer has failed")),
        };
        write_access_unit(&mut mux, &nals, self.index)?;
        self.mux = Some(Mux::Writing(mux));
        self.index += 1;
        Ok(())
    }

    pub fn finish(self) -> Result<(), MotionError> {
        if self.index != self.clock.frame_count() {
            return Err(encoding_error("incomplete MP4 frame sequence"));
        }
        match self.mux {
            Some(Mux::Writing(mut mux)) => mux.write_end().map_err(encoding_error),
            _ => Err(encoding_error("the MP4 writer has failed")),
        }
    }

    fn start_mux(&self, writer: W, nals: &[&[u8]]) -> Result<Mp4Writer<W>, MotionError> {
        let params = |kind| {
            nals.iter()
                .find(|nal| nal_type(nal) == kind)
                .map(|nal| nal.to_vec())
                .ok_or_else(|| encoding_error("missing AVC parameter set"))
        };
        let mut track = TrackConfig::from(AvcConfig {
            width: self.yuv.width as u16,
            height: self.yuv.height as u16,
            seq_param_set: params(NalUnitType::Sps)?,
            pic_param_set: params(NalUnitType::Pps)?,
        });
        track.timescale = self.clock.fps;
        let config = Mp4Config {
            major_brand: FourCC::from(*b"isom"),
            minor_version: 512,
            compatible_brands: [*b"isom", *b"iso2", *b"avc1", *b"mp41"]
                .map(FourCC::from)
                .to_vec(),
            timescale: self.clock.fps,
        };
        let mut mux = Mp4Writer::write_start(writer, &config).map_err(encoding_error)?;
        mux.add_track(&track).map_err(encoding_error)?;
        Ok(mux)
    }
}

fn nal_type(nal: &[u8]) -> NalUnitType {
    NalUnitType::from_id(nal[0])
}

fn write_access_unit<W: Write + Seek>(
    writer: &mut Mp4Writer<W>,
    nals: &[&[u8]],
    index: u64,
) -> Result<(), MotionError> {
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

fn video_level(width: usize, height: usize, fps: u32) -> Result<u8, MotionError> {
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

fn encoding_error(error: impl std::fmt::Display) -> MotionError {
    MotionError::Encoding(error.to_string())
}
