#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
use std::fs::File;
use std::io::Write;

use super::render::FrameRenderer;
use crate::ExportError;
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
use crate::{MAX_RASTER_SIDE_PX, MAX_RASTER_TOTAL_PX};

#[cfg(target_os = "macos")]
#[path = "mp4_avfoundation.rs"]
mod native;
#[cfg(any(target_os = "linux", target_os = "windows"))]
#[path = "mp4_ffmpeg.rs"]
mod native;

pub(super) fn available() -> bool {
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    {
        native::available()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        false
    }
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub(super) fn encode<W: Write>(
    mut writer: W,
    renderer: &FrameRenderer,
    fps: u32,
    frames: u64,
    time_at: impl Fn(u64) -> u64,
) -> Result<(), ExportError> {
    if !available() {
        return Err(ExportError::Mp4EncoderUnavailable);
    }
    let (width, height) = renderer.size();
    let padded = (width + width % 2, height + height % 2);
    let (width_px, height_px) = (i64::from(padded.0), i64::from(padded.1));
    if width_px > MAX_RASTER_SIDE_PX
        || height_px > MAX_RASTER_SIDE_PX
        || width_px * height_px > MAX_RASTER_TOTAL_PX
    {
        return Err(ExportError::OutputTooLarge {
            width_px,
            height_px,
        });
    }
    let directory = tempfile::tempdir().map_err(encoding_error)?;
    let path = directory.path().join("motion.mp4");
    let mut encoder = native::Encoder::open(&path, padded, fps)?;
    let mut source = vec![0; width as usize * height as usize * 4];
    let mut opaque = vec![255; padded.0 as usize * padded.1 as usize * 4];
    for index in 0..frames {
        renderer.read_rgba(time_at(index), &mut source)?;
        pack_bgra(&source, &mut opaque, width as usize, padded.0 as usize);
        encoder.push(&opaque, index)?;
    }
    encoder.finish(frames)?;
    let mut file = File::open(path).map_err(encoding_error)?;
    std::io::copy(&mut file, &mut writer).map_err(|error| ExportError::Write(error.to_string()))?;
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub(super) fn encode<W: Write>(
    _: W,
    _: &FrameRenderer,
    _: u32,
    _: u64,
    _: impl Fn(u64) -> u64,
) -> Result<(), ExportError> {
    Err(ExportError::Mp4EncoderUnavailable)
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
fn pack_bgra(source: &[u8], output: &mut [u8], width: usize, padded_width: usize) {
    for (row, out) in source
        .chunks_exact(width * 4)
        .zip(output.chunks_exact_mut(padded_width * 4))
    {
        for (rgba, bgra) in row.chunks_exact(4).zip(out.chunks_exact_mut(4)) {
            // The frame renderer already composited the source over white.
            bgra.copy_from_slice(&[rgba[2], rgba[1], rgba[0], 255]);
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
fn encoding_error(error: impl std::fmt::Display) -> ExportError {
    ExportError::MotionEncoding {
        format: "MP4",
        reason: error.to_string(),
    }
}

#[cfg(all(test, target_os = "macos"))]
#[path = "mp4_ffmpeg.rs"]
mod ffmpeg;

#[cfg(all(test, target_os = "macos"))]
#[test]
fn ffmpeg_fallback_writes_a_playable_h264_container_when_installed() {
    if !ffmpeg::available() {
        return;
    }
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("sample.mp4");
    let mut encoder = ffmpeg::Encoder::open(&path, (64, 64), 24).unwrap();
    let pixels = vec![255; 64 * 64 * 4];
    for index in 0..3 {
        encoder.push(&pixels, index).unwrap();
    }
    encoder.finish(3).unwrap();
    let result = std::fs::read(path).unwrap();
    assert_eq!(&result[4..8], b"ftyp");
    assert!(result.windows(4).any(|value| value == b"avc1"));
}
