//! Offline clips rendered with the same scene overrides and painter as Preview.

use std::io::{Seek, Write};

use jian_core::motion::Clip;
pub use jian_core::motion::ClipTrigger;
use jian_ops_schema::node::PenNode;
use op_editor_core::motion_export::MotionExportBackground;
use op_editor_ui::layout_scene::LayoutScene;

use crate::ExportError;
use op_motion_export::FrameClock;

mod contact_sheet;
mod lottie;
mod mp4;
mod render;
pub use contact_sheet::{
    render_motion_contact_sheet, MotionContactSheet, MotionFrameCell, MAX_CONTACT_SHEET_FRAMES,
};

pub use op_editor_core::motion_export::MAX_MOTION_EXPORT_FRAMES as MAX_MOTION_FRAMES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionFormat {
    Gif,
    Apng,
    PngSequence,
    Mp4,
    Lottie,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MotionExportOptions {
    pub trigger: ClipTrigger,
    pub fps: u32,
    pub scale: f32,
    pub duration_ms: Option<u64>,
    pub repeat: bool,
    pub background: MotionExportBackground,
    pub reduced_motion: bool,
}

impl Default for MotionExportOptions {
    fn default() -> Self {
        Self {
            trigger: ClipTrigger::Mount,
            fps: 30,
            scale: 1.0,
            duration_ms: None,
            repeat: true,
            background: MotionExportBackground::Transparent,
            reduced_motion: false,
        }
    }
}

/// Whether this renderer can produce H.264 MP4 on the current host.
pub fn mp4_available() -> bool {
    mp4::available()
}

/// Render one deliverable, including embedded soundtracks for MP4.
pub fn export_motion_bytes(
    scene: &LayoutScene,
    root: &PenNode,
    format: MotionFormat,
    options: &MotionExportOptions,
) -> Result<Vec<u8>, ExportError> {
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    if format == MotionFormat::Mp4
        && op_editor_core::audio_tracks::audio_duration_ms(root, &options.trigger) > 0
    {
        return Err(ExportError::UnsupportedFormat {
            format: "MP4 audio on this platform",
        });
    }
    let mut output = std::io::Cursor::new(Vec::new());
    export_motion(&mut output, scene, root, format, options)?;
    let bytes = output.into_inner();
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    if format == MotionFormat::Mp4 {
        let duration = motion_duration(root, &options.trigger, options.duration_ms)
            .ok_or(ExportError::MotionDurationRequired)?;
        return op_audio::mix_mp4(bytes, root, &options.trigger, duration).map_err(|error| {
            ExportError::MotionEncoding {
                format: "MP4 audio",
                reason: error.to_string(),
            }
        });
    }
    Ok(bytes)
}

/// Write a clip with one fixed crop across all frames. The writer owns
/// the output so callers can stream to a file without buffering its bytes.
pub fn export_motion<W: Write + Seek>(
    mut writer: W,
    scene: &LayoutScene,
    root: &PenNode,
    format: MotionFormat,
    options: &MotionExportOptions,
) -> Result<(), ExportError> {
    FrameClock::validate_fps(options.fps)?;
    let clip = build_clip(root, &options.trigger)?;
    let clock = FrameClock::new(
        options.fps,
        motion_duration(root, &options.trigger, options.duration_ms),
    )?;
    let fps = u64::from(options.fps);
    let frame_count = clock.frame_count();
    if format == MotionFormat::Mp4 {
        clock.validate_mp4()?;
    } else if frame_count > u64::from(MAX_MOTION_FRAMES) {
        return Err(ExportError::TooManyMotionFrames {
            frames: frame_count,
            limit: MAX_MOTION_FRAMES,
        });
    }
    let root_id = jian_core::document::tree::node_schema_id(root);
    if format == MotionFormat::Lottie {
        return lottie::encode(&mut writer, scene, root, clip, clock, options);
    }
    let transparent =
        format != MotionFormat::Mp4 && options.background == MotionExportBackground::Transparent;
    let renderer = render::FrameRenderer::new(scene, root_id, options.scale, clip)?
        .with_options(transparent, options.reduced_motion);
    let time_at = |index| clock.time_at(index);
    match format {
        MotionFormat::Mp4 => mp4::encode(writer, &renderer, clock),
        MotionFormat::Lottie => unreachable!("Lottie uses vector export above"),
        MotionFormat::Gif => {
            let (width, height) = renderer.size();
            let map_error = |error: gif::EncodingError| ExportError::MotionEncoding {
                format: "GIF",
                reason: error.to_string(),
            };
            let mut encoder =
                gif::Encoder::new(writer, width as u16, height as u16, &[]).map_err(map_error)?;
            if options.repeat {
                encoder
                    .set_repeat(gif::Repeat::Infinite)
                    .map_err(map_error)?;
            }
            let mut pixels = vec![0; width as usize * height as usize * 4];
            for index in 0..frame_count {
                let time = time_at(index);
                // Round timestamps to centiseconds, rather than rounding
                // every delay independently and drifting over the clip.
                let start = (time + 5) / 10;
                let end = (if index + 1 == frame_count {
                    time + 1_000 / fps
                } else {
                    time_at(index + 1)
                } + 5)
                    / 10;
                renderer.read_rgba(time, &mut pixels)?;
                let mut frame =
                    gif::Frame::from_rgba_speed(width as u16, height as u16, &mut pixels, 10);
                frame.delay = (end - start).max(1) as u16;
                frame.dispose = gif::DisposalMethod::Background;
                encoder.write_frame(&frame).map_err(map_error)?;
            }
            encoder.into_inner().map(|_| ()).map_err(map_error)
        }
        MotionFormat::Apng => {
            let (width, height) = renderer.size();
            let map_error = |error: png::EncodingError| ExportError::MotionEncoding {
                format: "APNG",
                reason: error.to_string(),
            };
            let mut encoder = png::Encoder::new(writer, width as u32, height as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .set_animated(frame_count as u32, if options.repeat { 0 } else { 1 })
                .map_err(map_error)?;
            let mut encoder = encoder.write_header().map_err(map_error)?;
            let mut pixels = vec![0; width as usize * height as usize * 4];
            for index in 0..frame_count {
                let time = time_at(index);
                let delay = if index + 1 == frame_count {
                    1_000 / fps
                } else {
                    time_at(index + 1) - time
                };
                renderer.read_rgba(time, &mut pixels)?;
                encoder
                    .set_frame_delay(delay.max(1) as u16, 1_000)
                    .map_err(map_error)?;
                encoder.write_image_data(&pixels).map_err(map_error)?;
            }
            encoder.finish().map_err(map_error)
        }
        MotionFormat::PngSequence => {
            let map_error = |error: zip::result::ZipError| ExportError::MotionEncoding {
                format: "PNG sequence",
                reason: error.to_string(),
            };
            let mut archive = zip::ZipWriter::new(writer);
            for index in 0..frame_count {
                let bytes = renderer.render_png(time_at(index))?;
                let name = format!("frame-{index:04}.png");
                archive
                    .start_file(name, zip::write::SimpleFileOptions::default())
                    .map_err(map_error)?;
                archive
                    .write_all(&bytes)
                    .map_err(|error| ExportError::Write(error.to_string()))?;
            }
            archive.finish().map_err(map_error)?;
            Ok(())
        }
    }
}

fn build_clip(root: &PenNode, trigger: &ClipTrigger) -> Result<Clip, ExportError> {
    let clip = Clip::build(root, trigger);
    if clip.is_empty() && op_editor_core::audio_tracks::audio_duration_ms(root, trigger) == 0 {
        return Err(ExportError::EmptyMotionClip);
    }
    Ok(clip)
}

fn motion_duration(root: &PenNode, trigger: &ClipTrigger, selected: Option<u64>) -> Option<u64> {
    selected.or_else(|| {
        let audio = op_editor_core::audio_tracks::audio_duration_ms(root, trigger);
        Clip::build(root, trigger)
            .duration_ms()
            .map(|ms| ms.max(audio))
            .or((audio > 0).then_some(audio))
    })
}

#[cfg(test)]
mod tests;
